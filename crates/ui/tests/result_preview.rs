//! S-142: the production preview scheduler.
//!
//! The cases use a fake provider, so they never touch a file, an engine or a
//! codec. They cover the rules of the scheduler: the newest request wins, a
//! result of a closed panel never reaches a newer panel, one request runs and
//! one waits, a cancelled request stops, and a malformed buffer is refused.
//!
//! These cases were written and never executed. No runtime acceptance is
//! claimed by them.

use kawaro_core::CancelToken;
use kawaro_core::action::Registry;
use kawaro_core::i18n::Lang;
use kawaro_core::image_preview::{
    DisplayPixels, ImagePreviewArtifact, ImagePreviewError, ImagePreviewMode, ImagePreviewProvider,
    ImagePreviewRequest, PreviewStamp,
};
use kawaro_ui::preview_schedule::{Failure, Outcome, Scheduler};
use kawaro_ui::stage_two::{PreviewUpdate, StageTwoState};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The release that lets every request finish.
const ALL: u64 = u64::MAX;

/// The action of every request: the fake never reads it.
fn action() -> kawaro_core::ActionDef {
    Registry::builtin()
        .get("image.to-webp")
        .expect("the built-in image.to-webp action")
        .clone()
}

/// One request of the fake.
fn request(stamp: PreviewStamp, mode: ImagePreviewMode) -> ImagePreviewRequest {
    ImagePreviewRequest {
        stamp,
        input: PathBuf::from("/tmp/kawaro-s142-photo.png"),
        resolved_action: action(),
        mode,
    }
}

/// What the fake answers.
#[derive(Clone, Copy, Debug)]
enum Answer {
    /// A picture of `width` x `height` with `bytes` RGBA bytes, so a
    /// mismatched length is a malformed buffer.
    Picture {
        width: u32,
        height: u32,
        bytes: usize,
    },
    /// A policy refusal.
    PolicyLimit,
    /// A plain failure.
    Failed,
}

/// A provider that holds every request until the test releases it.
struct Fake {
    /// The request numbers that may finish now.
    released: Mutex<Vec<u64>>,
    /// The stamps the provider saw, in order.
    seen: Mutex<Vec<PreviewStamp>>,
    /// The requests running right now.
    running: AtomicUsize,
    /// The most requests that ran at the same time.
    peak: AtomicUsize,
    /// True when the fake ignores the cancel token, like a codec that is
    /// already inside its own stage.
    deaf: bool,
    /// The answer of the fake.
    answer: Answer,
}

impl Fake {
    fn new(answer: Answer) -> Arc<Fake> {
        Arc::new(Fake {
            released: Mutex::new(Vec::new()),
            seen: Mutex::new(Vec::new()),
            running: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            deaf: false,
            answer,
        })
    }

    /// A fake that finishes even when its token is cancelled.
    fn deaf(answer: Answer) -> Arc<Fake> {
        Arc::new(Fake {
            released: Mutex::new(Vec::new()),
            seen: Mutex::new(Vec::new()),
            running: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            deaf: true,
            answer,
        })
    }

    /// Lets the request `number` finish.
    fn release(&self, number: u64) {
        self.released.lock().unwrap().push(number);
    }

    /// Lets every request finish.
    fn release_all(&self) {
        self.released.lock().unwrap().push(ALL);
    }

    /// The stamps the provider saw.
    fn seen(&self) -> Vec<PreviewStamp> {
        self.seen.lock().unwrap().clone()
    }

    /// The most requests that ran at the same time.
    fn peak(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    fn enter(&self) {
        let running = self.running.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(running, Ordering::SeqCst);
    }

    fn leave(&self) {
        self.running.fetch_sub(1, Ordering::SeqCst);
    }
}

impl ImagePreviewProvider for Fake {
    fn preview(
        &self,
        request: &ImagePreviewRequest,
        cancel: &CancelToken,
    ) -> Result<ImagePreviewArtifact, ImagePreviewError> {
        self.seen.lock().unwrap().push(request.stamp);
        self.enter();
        loop {
            let ready = {
                let released = self.released.lock().unwrap();
                released.contains(&request.stamp.request) || released.contains(&ALL)
            };
            if ready {
                break;
            }
            if !self.deaf && cancel.is_cancelled() {
                self.leave();
                return Err(ImagePreviewError::Cancelled);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        self.leave();
        match self.answer {
            Answer::Picture {
                width,
                height,
                bytes,
            } => Ok(ImagePreviewArtifact {
                stamp: request.stamp,
                pixels: DisplayPixels {
                    width,
                    height,
                    rgba8: vec![0; bytes],
                },
                // The intended output, never the size of the display proxy.
                output_width: width * 4,
                output_height: height * 4,
                output_format: String::from("webp"),
                final_encoded_bytes: match request.mode {
                    ImagePreviewMode::EncodedResult => Some(4096),
                    ImagePreviewMode::TransformedDraft => None,
                },
                mode: request.mode,
            }),
            Answer::PolicyLimit => Err(ImagePreviewError::PolicyLimit),
            Answer::Failed => Err(ImagePreviewError::Failed(String::from("no picture"))),
        }
    }
}

/// The next published outcome, or None when none arrives in time.
fn wait_for(scheduler: &Scheduler) -> Option<Outcome> {
    for _ in 0..4000 {
        if let Some(outcome) = scheduler.poll() {
            return Some(outcome);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    None
}

/// Lets the debounce of an automatic draft pass.
fn pass_debounce(scheduler: &Scheduler) {
    scheduler.tick(Instant::now() + Duration::from_secs(1));
}

/// A panel of an image action with options.
fn panel() -> StageTwoState {
    StageTwoState::build(
        &action(),
        &[PathBuf::from("/tmp/kawaro-s142-photo.png")],
        Lang::En,
        false,
    )
    .expect("image.to-webp has options")
}

/// The first published update of the panel.
fn wait_for_update(state: &StageTwoState) -> Option<PreviewUpdate> {
    for _ in 0..4000 {
        if let Some(update) = state.poll_preview() {
            return Some(update);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    None
}

#[test]
fn only_the_newest_of_two_rapid_changes_publishes() {
    let fake = Fake::new(Answer::Picture {
        width: 8,
        height: 8,
        bytes: 8 * 8 * 4,
    });
    let scheduler = Scheduler::new();
    scheduler.set_provider(fake.clone());
    let token = CancelToken::new();

    let first = scheduler.stamp(1, 4);
    let second = scheduler.stamp(1, 4);
    scheduler.request_draft(
        request(first, ImagePreviewMode::TransformedDraft),
        token.clone(),
    );
    // The second change lands while the first one is still queued.
    scheduler.request_draft(
        request(second, ImagePreviewMode::TransformedDraft),
        token.clone(),
    );
    pass_debounce(&scheduler);

    fake.release(second.request);
    fake.release(first.request);
    let outcome = wait_for(&scheduler).expect("a published outcome");
    assert_eq!(outcome.stamp(), second, "only the newest request publishes");
    let seen = fake.seen();
    assert_eq!(seen.len(), 1, "the older request never ran: {seen:?}");
}

#[test]
fn a_late_result_of_a_closed_panel_never_reaches_the_new_panel() {
    let fake = Fake::deaf(Answer::Picture {
        width: 8,
        height: 8,
        bytes: 8 * 8 * 4,
    });
    let scheduler = Scheduler::new();
    scheduler.set_provider(fake.clone());
    let token = CancelToken::new();

    // The first panel asks and the provider holds the answer.
    let old = scheduler.stamp(1, 1);
    scheduler.request_encoded(request(old, ImagePreviewMode::EncodedResult), token.clone());
    // The panel closes: its work is cancelled and its stamp is dropped.
    scheduler.cancel_panel(1, 1);

    // The second panel asks and its answer comes first.
    let new = scheduler.stamp(1, 2);
    scheduler.request_encoded(request(new, ImagePreviewMode::EncodedResult), token.clone());
    fake.release(new.request);
    fake.release(old.request);

    let outcome = wait_for(&scheduler).expect("a published outcome");
    assert_eq!(
        outcome.stamp(),
        new,
        "the old panel's result is not published"
    );
    assert_ne!(outcome.stamp().panel, old.panel);
}

#[test]
fn one_request_runs_and_one_waits() {
    let fake = Fake::new(Answer::Picture {
        width: 8,
        height: 8,
        bytes: 8 * 8 * 4,
    });
    let scheduler = Scheduler::new();
    scheduler.set_provider(fake.clone());
    let token = CancelToken::new();

    let first = scheduler.stamp(1, 5);
    let second = scheduler.stamp(1, 5);
    let third = scheduler.stamp(1, 5);
    scheduler.request_encoded(
        request(first, ImagePreviewMode::EncodedResult),
        token.clone(),
    );
    // Two more changes land while the first one runs: only the newest waits.
    scheduler.request_encoded(
        request(second, ImagePreviewMode::EncodedResult),
        token.clone(),
    );
    scheduler.request_encoded(
        request(third, ImagePreviewMode::EncodedResult),
        token.clone(),
    );

    fake.release(first.request);
    fake.release(second.request);
    fake.release(third.request);
    let outcome = wait_for(&scheduler).expect("a published outcome");
    assert_eq!(outcome.stamp(), third, "the newest request publishes");
    assert_eq!(fake.peak(), 1, "one codec at a time");
    let seen = fake.seen();
    assert_eq!(seen.len(), 2, "the middle request never ran: {seen:?}");
}

#[test]
fn a_cancelled_request_stops_and_reports_the_cancellation() {
    let fake = Fake::new(Answer::Picture {
        width: 8,
        height: 8,
        bytes: 8 * 8 * 4,
    });
    let scheduler = Scheduler::new();
    scheduler.set_provider(fake.clone());
    let token = CancelToken::new();

    let stamp = scheduler.stamp(1, 6);
    scheduler.request_encoded(
        request(stamp, ImagePreviewMode::EncodedResult),
        token.clone(),
    );
    // The panel closes while the request runs.
    token.cancel();
    scheduler.cancel_panel(1, 6);

    let outcome = wait_for(&scheduler).expect("a published outcome");
    match outcome {
        Outcome::Failed { kind, .. } => assert_eq!(kind, Failure::Cancelled),
        other => panic!("a cancelled request fails: {other:?}"),
    }
}

#[test]
fn a_malformed_buffer_is_refused() {
    let fake = Fake::new(Answer::Picture {
        width: 8,
        height: 8,
        // Four bytes per pixel would be 256; this is not.
        bytes: 100,
    });
    let scheduler = Scheduler::new();
    scheduler.set_provider(fake.clone());
    let token = CancelToken::new();

    let stamp = scheduler.stamp(1, 7);
    scheduler.request_encoded(
        request(stamp, ImagePreviewMode::EncodedResult),
        token.clone(),
    );
    fake.release(stamp.request);

    let outcome = wait_for(&scheduler).expect("a published outcome");
    match outcome {
        Outcome::Failed { kind, .. } => assert_eq!(kind, Failure::Failed),
        other => panic!("a malformed buffer is refused: {other:?}"),
    }
}

#[test]
fn a_draft_waits_for_its_debounce() {
    let fake = Fake::new(Answer::Picture {
        width: 8,
        height: 8,
        bytes: 8 * 8 * 4,
    });
    let scheduler = Scheduler::new();
    scheduler.set_provider(fake.clone());
    let token = CancelToken::new();

    let stamp = scheduler.stamp(1, 8);
    scheduler.request_draft(
        request(stamp, ImagePreviewMode::TransformedDraft),
        token.clone(),
    );
    // The debounce has not passed, so nothing runs yet.
    std::thread::sleep(Duration::from_millis(50));
    assert!(fake.seen().is_empty(), "the draft waits for its debounce");

    pass_debounce(&scheduler);
    fake.release(stamp.request);
    let outcome = wait_for(&scheduler).expect("a published outcome");
    assert_eq!(outcome.stamp(), stamp);
    assert_eq!(fake.seen().len(), 1);
}

#[test]
fn the_panel_shows_the_draft_without_a_byte_count() {
    let fake = Fake::new(Answer::Picture {
        width: 8,
        height: 8,
        bytes: 8 * 8 * 4,
    });
    let scheduler = Scheduler::new();
    scheduler.set_provider(fake.clone());
    let state = panel();
    state.attach_preview(scheduler.clone(), 1, 2);
    state.request_preview();
    fake.release_all();

    let update = wait_for_update(&state).expect("a published preview");
    assert!(
        update.image.is_some(),
        "the panel shows the transformed draft"
    );
    assert_eq!(update.label, "Transformed draft");
    assert_eq!(update.bytes, None, "a draft has no byte count");
    assert_eq!(update.failure, None);
    assert_eq!(
        update
            .output
            .as_ref()
            .map(|output| (output.width, output.height)),
        Some((32, 32)),
        "the readout takes the intended output, never the proxy size"
    );
}

#[test]
fn the_panel_shows_the_encoded_result_with_its_byte_count() {
    let fake = Fake::new(Answer::Picture {
        width: 8,
        height: 8,
        bytes: 8 * 8 * 4,
    });
    let scheduler = Scheduler::new();
    scheduler.set_provider(fake.clone());
    let state = panel();
    state.attach_preview(scheduler.clone(), 1, 2);
    state.request_encoded();
    fake.release_all();

    let update = wait_for_update(&state).expect("a published preview");
    assert!(update.image.is_some(), "the panel shows the encoded result");
    assert_eq!(update.label, "Encoded result");
    assert_eq!(
        update.bytes,
        Some(4096),
        "the byte count belongs to the encoding"
    );
}

#[test]
fn a_refused_or_failed_request_clears_the_picture_and_says_why() {
    for (answer, kind) in [
        (Answer::PolicyLimit, Failure::PolicyLimit),
        (Answer::Failed, Failure::Failed),
    ] {
        let fake = Fake::new(answer);
        let scheduler = Scheduler::new();
        scheduler.set_provider(fake.clone());
        let state = panel();
        state.attach_preview(scheduler.clone(), 1, 2);
        state.request_preview();
        fake.release_all();

        let update = wait_for_update(&state).expect("a published preview");
        assert!(update.image.is_none(), "the source is never a result");
        assert_eq!(update.failure, Some(kind));
        assert_eq!(update.bytes, None);
    }
}
