//! The production preview scheduler (S-142-C).
//!
//! One worker thread lives as long as the hive. It runs one preview at a
//! time and keeps at most one newer request waiting, so a busy codec never
//! overlaps a new one and the queue never grows. The newest-wins decision
//! comes from the [`Generation`] machine, the one of S-118 extended with the
//! stamp of the running and of the wanted request and with the hold that
//! keeps an automatic draft waiting for its debounce. The machine lives here
//! because the scheduler needs it on every platform; `preview_worker`
//! re-exports it under its old name.
//!
//! The UI never calls an engine. The provider is a
//! [`SharedPreviewProvider`], injected by the daemon, and every request
//! carries the resolved action, so the preview and the run describe the same
//! work. Nothing is written to disk and no job event is emitted.
//!
//! The scheduler keeps no cache: the panel holds the current picture, and no
//! encoded artifact and no full pixel buffer is retained anywhere.

use kawaro_core::CancelToken;
use kawaro_core::image_preview::{
    ImagePreviewArtifact, ImagePreviewError, ImagePreviewMode, ImagePreviewRequest, PreviewStamp,
    SharedPreviewProvider,
};
use slint::{Rgba8Pixel, SharedPixelBuffer};
#[cfg(unix)]
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The debounce of an automatic draft, in milliseconds. It is a UI policy,
/// not a measured performance claim.
pub const DRAFT_DEBOUNCE: Duration = Duration::from_millis(180);

/// The longest side of a display buffer, in pixels. The buffer of a preview
/// never exceeds it on either side.
pub const MAX_DISPLAY_SIDE: u32 = 512;

/// What the worker should do next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Render this generation now.
    Render(u64),
    /// Nothing to render until the next submit.
    Idle,
}

/// The newest-wins state machine of one preview worker (S-118 AC5): one
/// render in flight, at most one generation waiting, and a finished render is
/// published only while it is still the newest.
///
/// S-142 extends it for the production scheduler: the id is the only
/// counter, the stamp of the running and of the wanted request travels with
/// it, and a held request waits for its debounce before it starts.
#[derive(Clone, Debug, Default)]
pub struct Generation {
    next: u64,
    wanted: Option<u64>,
    running: Option<u64>,
    published: Option<u64>,
    /// The stamp of the running request.
    running_stamp: Option<PreviewStamp>,
    /// The stamp of the wanted request.
    wanted_stamp: Option<PreviewStamp>,
    /// The wanted request waits for its debounce and does not start yet.
    held: bool,
}

impl Generation {
    pub fn new() -> Self {
        Self::default()
    }

    /// The id of the next request. It is the only counter of the production
    /// path, so a request is identified by one number everywhere.
    pub fn allocate(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }

    /// The generation being rendered now, if any.
    pub fn running(&self) -> Option<u64> {
        self.running
    }

    /// The generation waiting for the render in flight, if any.
    pub fn wanted(&self) -> Option<u64> {
        self.wanted
    }

    /// The last published generation, if any.
    pub fn published(&self) -> Option<u64> {
        self.published
    }

    /// The stamp of the running request, if any.
    pub fn running_stamp(&self) -> Option<PreviewStamp> {
        self.running_stamp
    }

    /// The stamp of the wanted request, if any.
    pub fn wanted_stamp(&self) -> Option<PreviewStamp> {
        self.wanted_stamp
    }

    /// A value changed: it becomes the newest generation, and it starts only
    /// when nothing is in flight.
    ///
    /// `held` keeps the request from starting until [`Generation::release`].
    /// That is how the debounce of an automatic draft works: the request is
    /// already the newest one, so a running one is stale, but it does not
    /// start yet.
    pub fn submit(&mut self, id: u64, stamp: PreviewStamp, held: bool) -> Next {
        self.wanted = Some(id);
        self.wanted_stamp = Some(stamp);
        self.held = held;
        self.start()
    }

    /// The held request may start now.
    pub fn release(&mut self) -> Next {
        self.held = false;
        self.start()
    }

    /// The wanted request is dropped: its panel closed before it started.
    pub fn cancel_wanted(&mut self) {
        self.wanted = None;
        self.wanted_stamp = None;
        self.held = false;
    }

    /// The render in flight finished. Returns the generation that may be
    /// published (only when it is still the newest) and what to render next.
    pub fn finish(&mut self) -> (Option<u64>, Next) {
        let Some(done) = self.running.take() else {
            return (None, self.start());
        };
        self.running_stamp = None;
        let published = self.wanted.is_none().then_some(done);
        if published.is_some() {
            self.published = Some(done);
        }
        (published, self.start())
    }

    fn start(&mut self) -> Next {
        if self.running.is_some() || self.held {
            return Next::Idle;
        }
        match self.wanted.take() {
            Some(id) => {
                self.running = Some(id);
                self.running_stamp = self.wanted_stamp.take();
                Next::Render(id)
            }
            None => Next::Idle,
        }
    }
}

/// Why a preview did not produce a picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The request was cancelled before it finished.
    Cancelled,
    /// The source file changed while it was read.
    SourceChanged,
    /// The source or the requested result is over the preview policy limits.
    PolicyLimit,
    /// The operation has no preview, or the request is not a valid one.
    Unsupported,
    /// The preview failed.
    Failed,
}

/// The facts of one output, read from the engine artifact.
///
/// The size and the format are the intended output, never the size of the
/// display proxy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    /// The intended output width, in pixels.
    pub width: u32,
    /// The intended output height, in pixels.
    pub height: u32,
    /// The output format, as an extension without a dot.
    pub format: String,
    /// The exact length of the encoded artifact. `None` for a draft, which
    /// never encodes.
    pub final_encoded_bytes: Option<u64>,
    /// The mode this output belongs to.
    pub mode: ImagePreviewMode,
}

/// One published preview.
#[derive(Clone, Debug)]
pub struct Publication {
    /// The stamp of the request this belongs to.
    pub stamp: PreviewStamp,
    /// The display buffer, at most 512 pixels on its longest side.
    pub image: SharedPixelBuffer<Rgba8Pixel>,
    /// The facts of the output, when the request produced them. `None` for
    /// the legacy source draft, which is not a result.
    pub output: Option<Output>,
    /// True for the legacy source draft: a picture of the source, never a
    /// picture of the result.
    pub source: bool,
}

/// What the scheduler publishes for the panel.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// A picture, with the facts of its output.
    Ready(Publication),
    /// No picture: the request failed or was refused.
    Failed {
        /// The stamp of the request that failed.
        stamp: PreviewStamp,
        /// The typed reason.
        kind: Failure,
        /// The detail of the reason, for the log.
        detail: String,
    },
}

impl Outcome {
    /// The stamp of the request this belongs to.
    pub fn stamp(&self) -> PreviewStamp {
        match self {
            Outcome::Ready(publication) => publication.stamp,
            Outcome::Failed { stamp, .. } => *stamp,
        }
    }
}

/// What the worker runs. One at a time, so the variant size does not matter.
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
enum Work {
    /// An engine preview of one resolved action.
    Engine(ImagePreviewRequest),
    /// The legacy source draft: a bounded local decode, with no engine.
    #[cfg(unix)]
    Source(PathBuf),
}

/// One queued request.
#[derive(Clone)]
struct Job {
    /// The stamp of the request.
    stamp: PreviewStamp,
    /// What the worker runs.
    work: Work,
    /// The token that stops this request. It belongs to the panel, so
    /// closing the panel stops the work of that panel only.
    token: CancelToken,
    /// When an automatic draft may start. `None` starts it at once.
    due: Option<Instant>,
}

/// The state the UI thread and the worker share.
struct State {
    /// The newest-wins machine and the only production counter.
    generation: Generation,
    /// The request in flight.
    running: Option<Job>,
    /// The newest queued request. It replaces an older one.
    pending: Option<Job>,
}

/// The shared part of the scheduler.
struct Shared {
    /// The machine and the queued requests.
    state: Mutex<State>,
    /// The provider that answers the previews, when the daemon injected one.
    provider: Mutex<Option<SharedPreviewProvider>>,
    /// The finished outcomes. The receiver is behind a lock, so the shared
    /// part stays shareable between the interface and the worker.
    done: Mutex<Receiver<Outcome>>,
    /// The sender of the finished outcomes.
    done_tx: Sender<Outcome>,
}

/// The preview scheduler of one hive.
///
/// The hive owns it, so the one worker thread lives as long as the hive and
/// every panel replacement shares it: there is never more than one running
/// request and one pending request, not one pair per panel.
pub struct Scheduler {
    /// The shared state, the provider and the outcome channel.
    shared: Arc<Shared>,
    /// The channel that hands the jobs to the worker. It is not in the shared
    /// part, so dropping the scheduler closes the channel and the worker
    /// ends with it.
    jobs: Sender<Job>,
}

impl Scheduler {
    /// Creates the scheduler and starts its one worker thread.
    pub fn new() -> Rc<Self> {
        let (jobs, inbox) = mpsc::channel();
        let (done_tx, done) = mpsc::channel();
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                generation: Generation::new(),
                running: None,
                pending: None,
            }),
            provider: Mutex::new(None),
            done: Mutex::new(done),
            done_tx,
        });
        let worker = shared.clone();
        // A worker that cannot start leaves the scheduler without one: the
        // panel shows its dimensions and no picture, and nothing panics.
        let _ = std::thread::Builder::new()
            .name(String::from("kawaro-preview"))
            .spawn(move || worker_loop(worker, inbox));
        Rc::new(Scheduler { shared, jobs })
    }

    /// The provider that answers the preview requests.
    pub fn set_provider(&self, provider: SharedPreviewProvider) {
        *lock(&self.shared.provider) = Some(provider);
    }

    /// True when a provider answers the preview requests.
    pub fn has_provider(&self) -> bool {
        lock(&self.shared.provider).is_some()
    }

    /// The stamp of the next request of `panel` in `session`.
    ///
    /// The request number comes from the generation machine, the only
    /// counter of the production path.
    pub fn stamp(&self, session: u64, panel: u64) -> PreviewStamp {
        let mut state = lock(&self.shared.state);
        PreviewStamp {
            session,
            panel,
            request: state.generation.allocate(),
        }
    }

    /// Queues an automatic draft: it starts after the debounce.
    pub fn request_draft(&self, request: ImagePreviewRequest, token: CancelToken) {
        let stamp = request.stamp;
        self.submit(
            stamp,
            Work::Engine(request),
            token,
            Some(Instant::now() + DRAFT_DEBOUNCE),
        );
    }

    /// Queues an explicit encoded request: it starts at once.
    pub fn request_encoded(&self, request: ImagePreviewRequest, token: CancelToken) {
        let stamp = request.stamp;
        self.submit(stamp, Work::Engine(request), token, None);
    }

    /// Queues the legacy source draft: a bounded local decode, with no
    /// engine and no result.
    #[cfg(unix)]
    pub fn request_source(&self, path: PathBuf, stamp: PreviewStamp, token: CancelToken) {
        self.submit(stamp, Work::Source(path), token, None);
    }

    /// Queues one request. The newest one replaces the queued older one, and
    /// a held request waits for its debounce.
    fn submit(&self, stamp: PreviewStamp, work: Work, token: CancelToken, due: Option<Instant>) {
        let mut state = lock(&self.shared.state);
        state.pending = Some(Job {
            stamp,
            work,
            token,
            due,
        });
        let next = state.generation.submit(stamp.request, stamp, due.is_some());
        dispatch(&mut state, next, &self.jobs);
    }

    /// Starts the queued request whose debounce elapsed.
    ///
    /// The call comes from the panel's tick, so a draft starts at most one
    /// frame after its debounce.
    pub fn tick(&self, now: Instant) {
        let mut state = lock(&self.shared.state);
        let due = state.pending.as_ref().and_then(|job| job.due);
        if due.is_none_or(|due| now < due) {
            return;
        }
        if let Some(job) = state.pending.as_mut() {
            job.due = None;
        }
        let next = state.generation.release();
        dispatch(&mut state, next, &self.jobs);
    }

    /// The next finished outcome, if any. The panel keeps the newest one
    /// that belongs to it.
    pub fn poll(&self) -> Option<Outcome> {
        let receiver = lock(&self.shared.done);
        receiver.try_recv().ok()
    }

    /// The panel closed: its queued request is dropped and its running one is
    /// cancelled. The work of another panel is untouched.
    pub fn cancel_panel(&self, session: u64, panel: u64) {
        let mut state = lock(&self.shared.state);
        let queued_is_ours = state
            .pending
            .as_ref()
            .is_some_and(|job| job.stamp.session == session && job.stamp.panel == panel);
        if queued_is_ours {
            state.pending = None;
            state.generation.cancel_wanted();
        }
        let running_is_ours = state
            .running
            .as_ref()
            .is_some_and(|job| job.stamp.session == session && job.stamp.panel == panel);
        if running_is_ours && let Some(job) = &state.running {
            job.token.cancel();
        }
    }
}

/// Hands the queued request to the worker when the machine says it may
/// start.
fn dispatch(state: &mut State, next: Next, jobs: &Sender<Job>) {
    if !matches!(next, Next::Render(_)) {
        return;
    }
    let Some(job) = state.pending.take() else {
        return;
    };
    state.running = Some(job.clone());
    // A closed channel means the scheduler is gone: the job is dropped and
    // the panel falls back to its dimensions.
    let _ = jobs.send(job);
}

/// The one worker of the hive.
///
/// It takes a job, runs it, publishes the outcome and starts the queued one.
/// A job that is queued while a codec runs waits for it: two codecs never
/// run at the same time.
fn worker_loop(shared: Arc<Shared>, inbox: Receiver<Job>) {
    let mut next = inbox.recv().ok();
    while let Some(job) = next {
        let outcome = run(&shared, &job);
        // The result of a request that is no longer the newest one is not
        // published: the machine says so.
        let mut state = lock(&shared.state);
        let (published, step) = state.generation.finish();
        state.running = None;
        if published.is_some() {
            let _ = shared.done_tx.send(outcome);
        }
        next = match step {
            Next::Render(_) => {
                let job = state.pending.take();
                if let Some(job) = &job {
                    state.running = Some(job.clone());
                }
                job
            }
            Next::Idle => None,
        };
        drop(state);
        if next.is_none() {
            next = inbox.recv().ok();
        }
    }
}

/// Runs one job and turns it into an outcome.
fn run(shared: &Shared, job: &Job) -> Outcome {
    match &job.work {
        Work::Engine(request) => {
            let provider = { lock(&shared.provider).clone() };
            let Some(provider) = provider else {
                return Outcome::Failed {
                    stamp: job.stamp,
                    kind: Failure::Unsupported,
                    detail: String::from("no preview provider is installed"),
                };
            };
            match provider.preview(request, &job.token) {
                Ok(artifact) => publication(job.stamp, artifact),
                Err(error) => Outcome::Failed {
                    stamp: job.stamp,
                    kind: kind_of(&error),
                    detail: error.to_string(),
                },
            }
        }
        #[cfg(unix)]
        Work::Source(path) => match decode_source(path) {
            Some(image) => Outcome::Ready(Publication {
                stamp: job.stamp,
                image,
                output: None,
                source: true,
            }),
            None => Outcome::Failed {
                stamp: job.stamp,
                kind: Failure::Failed,
                detail: String::from("the source draft cannot be decoded"),
            },
        },
    }
}

/// The publication of one engine artifact, when its buffer is within the
/// display policy.
fn publication(stamp: PreviewStamp, artifact: ImagePreviewArtifact) -> Outcome {
    let Some(image) = buffer_of(&artifact) else {
        return Outcome::Failed {
            stamp,
            kind: Failure::Failed,
            detail: String::from("the display buffer does not match its size"),
        };
    };
    Outcome::Ready(Publication {
        stamp,
        image,
        output: Some(Output {
            width: artifact.output_width,
            height: artifact.output_height,
            format: artifact.output_format,
            final_encoded_bytes: artifact.final_encoded_bytes,
            mode: artifact.mode,
        }),
        source: false,
    })
}

/// The display buffer of an artifact.
///
/// The size must be real and bounded, and the RGBA length must match the
/// checked size, before a Slint buffer is made.
fn buffer_of(artifact: &ImagePreviewArtifact) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let (width, height) = (artifact.pixels.width, artifact.pixels.height);
    if width == 0 || height == 0 || width > MAX_DISPLAY_SIDE || height > MAX_DISPLAY_SIDE {
        return None;
    }
    let expected = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))?;
    if expected != artifact.pixels.rgba8.len() as u64 {
        return None;
    }
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let pixels = buffer.make_mut_slice();
    for (dst, src) in pixels
        .iter_mut()
        .zip(artifact.pixels.rgba8.as_chunks::<4>().0)
    {
        *dst = Rgba8Pixel::new(src[0], src[1], src[2], src[3]);
    }
    Some(buffer)
}

/// The legacy source draft: a bounded local decode, masked to the hexagon.
/// It is a picture of the source, never a picture of the result.
#[cfg(unix)]
fn decode_source(path: &std::path::Path) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let mut buffer = crate::preview_worker::decode_draft(path, crate::stage_two::MAX_DRAFT)?;
    let circumradius = (buffer.width() as f32 / 1.732_050_8).min(buffer.height() as f32 / 2.0);
    crate::preview_worker::mask_hexagon(&mut buffer, circumradius)?;
    Some(buffer)
}

/// The typed reason of an engine failure.
fn kind_of(error: &ImagePreviewError) -> Failure {
    match error {
        ImagePreviewError::Cancelled => Failure::Cancelled,
        ImagePreviewError::SourceChanged => Failure::SourceChanged,
        ImagePreviewError::PolicyLimit => Failure::PolicyLimit,
        ImagePreviewError::Unsupported(_) => Failure::Unsupported,
        ImagePreviewError::Failed(_) => Failure::Failed,
    }
}

/// A lock that survives a poisoned mutex: a panic in a worker must not take
/// the interface with it.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}
