//! Bounded pipe reads: a helper's exit does not imply EOF on inherited stdout.

use super::{StopToken, UnavailableReason};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

/// The caller owns termination/reaping on error. Never block on pipe EOF, and
/// drain while the helper runs so a full pipe cannot prevent it from exiting.
pub(super) fn read_output(
    child: &mut Child,
    timeout: Duration,
    limit: usize,
    stop: Option<&StopToken>,
) -> Result<Vec<u8>, UnavailableReason> {
    let stdout = child
        .stdout
        .as_mut()
        .ok_or(UnavailableReason::SourceError)?;
    let fd = stdout.as_raw_fd();
    // SAFETY: stdout owns fd throughout this call. fcntl retains no descriptor.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(UnavailableReason::SourceError);
    }
    let started = Instant::now();
    let mut output = Vec::new();
    let mut eof = false;
    let mut status = None;
    loop {
        if stop.is_some_and(StopToken::is_stopped) || started.elapsed() >= timeout {
            return Err(UnavailableReason::TimedOut);
        }
        if !eof {
            let mut buffer = [0_u8; 4096];
            match child.stdout.as_mut().unwrap().read(&mut buffer) {
                Ok(0) => eof = true,
                Ok(count) => {
                    if count > limit.saturating_sub(output.len()) {
                        return Err(UnavailableReason::InvalidData);
                    }
                    output.extend_from_slice(&buffer[..count]);
                    // Drain the next chunk immediately, checking the deadline
                    // on every read even if the writer never pauses.
                    continue;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(UnavailableReason::SourceError),
            }
        }
        if status.is_none() {
            status = child
                .try_wait()
                .map_err(|_| UnavailableReason::SourceError)?;
        }
        if eof && let Some(status) = status {
            return if status.success() {
                Ok(output)
            } else {
                Err(UnavailableReason::SourceError)
            };
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    struct FixtureChild(Child);

    impl FixtureChild {
        fn shell(script: &str) -> Self {
            Self(
                Command::new("sh")
                    .args(["-c", script])
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap(),
            )
        }
    }

    impl Drop for FixtureChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn drains_a_finite_payload_larger_than_the_pipe_before_waiting_for_exit() {
        let mut child = FixtureChild::shell("printf '%060000d' 0");
        let output = read_output(&mut child.0, Duration::from_secs(2), 65_536, None).unwrap();
        assert_eq!(output.len(), 60_000);
        assert!(output.iter().all(|byte| *byte == b'0'));
        assert!(child.0.try_wait().unwrap().unwrap().success());
    }

    #[test]
    fn rejects_output_past_the_limit_instead_of_retaining_it() {
        let mut child = FixtureChild::shell("printf '%05000d' 0");
        assert_eq!(
            read_output(&mut child.0, Duration::from_secs(2), 4096, None),
            Err(UnavailableReason::InvalidData)
        );
    }

    #[test]
    fn exited_helper_with_a_live_stdout_writer_still_obeys_the_deadline() {
        let mut holder = FixtureChild::shell("exec sleep 2");
        let mut child = FixtureChild::shell("exit 0");
        // The helper has exited, but another owned process still holds the
        // writer. This is the same pipe state as an inherited helper stdout.
        child.0.wait().unwrap();
        child.0.stdout = holder.0.stdout.take();
        let started = Instant::now();
        assert_eq!(
            read_output(&mut child.0, Duration::from_millis(30), 4096, None),
            Err(UnavailableReason::TimedOut)
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(holder.0.try_wait().unwrap().is_none());
    }

    #[test]
    fn stop_interrupts_a_pipe_without_waiting_for_its_writer() {
        let mut child = FixtureChild::shell("exec sleep 2");
        let stop = StopToken::new();
        let stopping = stop.clone();
        let stopper = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            stopping.stop();
        });
        let started = Instant::now();
        let result = read_output(&mut child.0, Duration::from_secs(2), 4096, Some(&stop));
        stopper.join().unwrap();
        assert_eq!(result, Err(UnavailableReason::TimedOut));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
