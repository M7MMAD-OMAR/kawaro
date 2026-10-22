// S-025a (backlog/S-025-windows-engines.md): the Windows process tree
// decisions, kept in a shared file so a Linux run mutates them. The test
// module below is the tester's contract and must not change; the
// implementation goes above it.
//
// Contract pinned by the tests (all items visible to `super::*`):
// - `CREATE_SUSPENDED` and `CREATE_NO_WINDOW`: `u32` constants, 0x4 and
//   0x08000000.
// - `trait JobOps` with two associated types and these methods, in this
//   exact shape (the tests implement it for a fake):
//     type Job;      // an owned job handle, closed when dropped
//     type Child;    // the spawned process
//     fn create_job(&mut self) -> io::Result<Self::Job>;
//     fn kill_on_close(&mut self, job: &Self::Job) -> io::Result<()>;
//     fn spawn_suspended(&mut self) -> io::Result<(Self::Child, u32)>; // (child, pid)
//     fn assign(&mut self, job: &Self::Job, child: &Self::Child) -> io::Result<()>;
//     fn threads_of(&mut self, pid: u32) -> io::Result<Vec<u32>>;
//     fn resume(&mut self, tid: u32) -> io::Result<()>;
//     fn terminate_process(&mut self, child: &Self::Child);
//   `io` is `std::io`.
// - `start_in_job<O: JobOps>(ops: &mut O) -> io::Result<(O::Job, O::Child)>`:
//   on success the job is returned still open; on failure after the spawn
//   the child is terminated once and the job is dropped (closed) before the
//   error returns.
// - `threads_of(pid: u32, entries: &[(u32, u32)]) -> Vec<u32>`, entries being
//   (owner pid, thread id) pairs, result in entry order.
// - `has_exe_extension(name: &str) -> bool`.

use std::io;

/// Starts the process with its main thread suspended, so it cannot start
/// children before it joins the job.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) const CREATE_SUSPENDED: u32 = 0x4;
/// No console window for a console tool started by a service with none.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The system calls `start_in_job` makes, injected so a fake can record them.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) trait JobOps {
    /// An owned job handle, closed when dropped.
    type Job;
    /// The spawned process.
    type Child;
    fn create_job(&mut self) -> io::Result<Self::Job>;
    fn kill_on_close(&mut self, job: &Self::Job) -> io::Result<()>;
    /// The suspended child and its process id.
    fn spawn_suspended(&mut self) -> io::Result<(Self::Child, u32)>;
    fn assign(&mut self, job: &Self::Job, child: &Self::Child) -> io::Result<()>;
    fn threads_of(&mut self, pid: u32) -> io::Result<Vec<u32>>;
    fn resume(&mut self, tid: u32) -> io::Result<()>;
    fn terminate_process(&mut self, child: &Self::Child);
}

/// Creates a kill-on-close job, spawns the child suspended, puts it in the
/// job and resumes its threads. The job comes back open: closing it kills
/// the whole tree. On a failure after the spawn the child is terminated and
/// the job closed before the error returns.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn start_in_job<O: JobOps>(ops: &mut O) -> io::Result<(O::Job, O::Child)> {
    let job = ops.create_job()?;
    ops.kill_on_close(&job)?;
    let (child, pid) = ops.spawn_suspended()?;
    match assign_and_resume(ops, &job, &child, pid) {
        Ok(()) => Ok((job, child)),
        Err(e) => {
            ops.terminate_process(&child);
            drop(job);
            Err(e)
        }
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn assign_and_resume<O: JobOps>(
    ops: &mut O,
    job: &O::Job,
    child: &O::Child,
    pid: u32,
) -> io::Result<()> {
    ops.assign(job, child)?;
    let threads = ops.threads_of(pid)?;
    if threads.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "the suspended process has no thread",
        ));
    }
    for tid in threads {
        ops.resume(tid)?;
    }
    Ok(())
}

/// The thread ids owned by `pid`, from (owner pid, thread id) pairs, in
/// entry order.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn threads_of(pid: u32, entries: &[(u32, u32)]) -> Vec<u32> {
    entries
        .iter()
        .filter(|(owner, _)| *owner == pid)
        .map(|(_, tid)| *tid)
        .collect()
}

/// Whether `name` ends in `.exe` or `.com`, any case, after a non-empty stem.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn has_exe_extension(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty() && (ext.eq_ignore_ascii_case("exe") || ext.eq_ignore_ascii_case("com"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::io;
    use std::rc::Rc;

    const PID: u32 = 10;
    const ASSIGN_DENIED: i32 = 5;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Op {
        CreateJob,
        KillOnClose,
        SpawnSuspended,
        Assign,
        ThreadsOf(u32),
        Resume(u32),
        TerminateProcess,
    }

    /// A job handle that counts its closes.
    #[derive(Debug)]
    struct FakeJob {
        closes: Rc<Cell<u32>>,
    }

    impl Drop for FakeJob {
        fn drop(&mut self) {
            self.closes.set(self.closes.get() + 1);
        }
    }

    struct Fake {
        log: Rc<RefCell<Vec<Op>>>,
        closes: Rc<Cell<u32>>,
        assign_fails: bool,
        threads: Vec<u32>,
    }

    impl Fake {
        fn new(assign_fails: bool, threads: Vec<u32>) -> Self {
            Fake {
                log: Rc::new(RefCell::new(Vec::new())),
                closes: Rc::new(Cell::new(0)),
                assign_fails,
                threads,
            }
        }

        fn ops(&self) -> Vec<Op> {
            self.log.borrow().clone()
        }

        fn count(&self, want: &Op) -> usize {
            self.log.borrow().iter().filter(|op| *op == want).count()
        }

        fn resumes(&self) -> usize {
            self.log
                .borrow()
                .iter()
                .filter(|op| matches!(op, Op::Resume(_)))
                .count()
        }
    }

    impl JobOps for Fake {
        type Job = FakeJob;
        type Child = u32;

        fn create_job(&mut self) -> io::Result<FakeJob> {
            self.log.borrow_mut().push(Op::CreateJob);
            Ok(FakeJob {
                closes: Rc::clone(&self.closes),
            })
        }

        fn kill_on_close(&mut self, _job: &FakeJob) -> io::Result<()> {
            self.log.borrow_mut().push(Op::KillOnClose);
            Ok(())
        }

        fn spawn_suspended(&mut self) -> io::Result<(u32, u32)> {
            self.log.borrow_mut().push(Op::SpawnSuspended);
            Ok((PID, PID))
        }

        fn assign(&mut self, _job: &FakeJob, child: &u32) -> io::Result<()> {
            assert_eq!(*child, PID, "assign gets the spawned child");
            self.log.borrow_mut().push(Op::Assign);
            if self.assign_fails {
                Err(io::Error::from_raw_os_error(ASSIGN_DENIED))
            } else {
                Ok(())
            }
        }

        fn threads_of(&mut self, pid: u32) -> io::Result<Vec<u32>> {
            self.log.borrow_mut().push(Op::ThreadsOf(pid));
            Ok(self.threads.clone())
        }

        fn resume(&mut self, tid: u32) -> io::Result<()> {
            self.log.borrow_mut().push(Op::Resume(tid));
            Ok(())
        }

        fn terminate_process(&mut self, child: &u32) {
            assert_eq!(*child, PID, "terminate gets the spawned child");
            self.log.borrow_mut().push(Op::TerminateProcess);
        }
    }

    #[test]
    fn ac3_flag_constants_have_the_win32_values() {
        assert_eq!(CREATE_SUSPENDED, 0x4_u32);
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000_u32);
    }

    #[test]
    fn ac1_success_runs_the_sequence_and_keeps_the_job_open() {
        let mut fake = Fake::new(false, vec![1, 3]);
        let started = start_in_job(&mut fake);
        assert_eq!(
            fake.ops(),
            vec![
                Op::CreateJob,
                Op::KillOnClose,
                Op::SpawnSuspended,
                Op::Assign,
                Op::ThreadsOf(PID),
                Op::Resume(1),
                Op::Resume(3),
            ]
        );
        assert_eq!(fake.count(&Op::TerminateProcess), 0);
        let (job, child) = started.expect("start_in_job succeeds");
        assert_eq!(child, PID);
        // Closing the job now would kill the child through kill-on-close.
        assert_eq!(fake.closes.get(), 0, "the job stays open while held");
        drop(job);
        assert_eq!(fake.closes.get(), 1, "the returned job closes once");
    }

    #[test]
    fn ac1_assign_failure_terminates_returns_the_error_and_closes_the_job() {
        let mut fake = Fake::new(true, vec![1, 3]);
        let r = start_in_job(&mut fake);
        let e = r.expect_err("assign failure is an error");
        assert_eq!(e.raw_os_error(), Some(ASSIGN_DENIED), "the assign error");
        assert_eq!(
            fake.ops(),
            vec![
                Op::CreateJob,
                Op::KillOnClose,
                Op::SpawnSuspended,
                Op::Assign,
                Op::TerminateProcess,
            ]
        );
        assert_eq!(fake.resumes(), 0, "no resume after a failed assign");
        assert_eq!(fake.count(&Op::TerminateProcess), 1);
        assert_eq!(fake.closes.get(), 1, "no job handle left open");
    }

    #[test]
    fn ac2_threads_of_keeps_the_threads_of_one_pid_in_order() {
        let entries = [(10, 1), (11, 2), (10, 3)];
        assert_eq!(threads_of(10, &entries), vec![1, 3]);
        assert_eq!(threads_of(12, &entries), Vec::<u32>::new());
    }

    #[test]
    fn ac2_no_threads_is_an_error_and_terminates_the_suspended_process() {
        let mut fake = Fake::new(false, Vec::new());
        let r = start_in_job(&mut fake);
        assert!(r.is_err(), "an empty thread list is an error");
        drop(r);
        assert_eq!(fake.count(&Op::TerminateProcess), 1, "terminated once");
        assert_eq!(fake.resumes(), 0);
        assert_eq!(fake.closes.get(), 1, "no job handle left open");
        assert_eq!(fake.ops().last(), Some(&Op::TerminateProcess));
    }

    #[test]
    fn ac5_has_exe_extension_for_the_nine_names() {
        for (name, want) in [
            ("a.exe", true),
            ("A.EXE", true),
            ("soffice.Com", true),
            ("a.bat", false),
            ("a.cmd", false),
            ("a", false),
            ("a.exe.txt", false),
            (".exe", false),
        ] {
            assert_eq!(has_exe_extension(name), want, "{name}");
        }
    }
}
