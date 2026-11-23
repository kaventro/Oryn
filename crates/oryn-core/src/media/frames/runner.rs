use std::ffi::OsString;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub trait Runner: Send + Sync {
    fn run(&self, program: &str, args: &[OsString]) -> bool;
}

pub struct ProcessRunner {
    timeout: Duration,
}

impl ProcessRunner {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Default for ProcessRunner {
    fn default() -> Self {
        Self::new(Duration::from_secs(15))
    }
}

impl Runner for ProcessRunner {
    fn run(&self, program: &str, args: &[OsString]) -> bool {
        let Ok(mut child) = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            return false;
        };
        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status.success(),
                Ok(None) if started.elapsed() < self.timeout => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn reports_exit_status_missing_programs_and_timeouts() {
        let r = ProcessRunner::new(Duration::from_millis(300));
        assert!(r.run("true", &[]));
        assert!(!r.run("false", &[]));
        assert!(!r.run("oryn-no-such-program", &[]));
        let started = Instant::now();
        assert!(!r.run("sleep", &["5".into()]));
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
