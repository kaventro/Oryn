mod chain;
mod command;
mod png;
mod runner;
mod store;

#[cfg(all(unix, not(target_os = "macos")))]
mod freedesktop;
#[cfg(target_os = "macos")]
mod quicklook;
#[cfg(windows)]
mod shell;

use std::path::{Path, PathBuf};

pub use chain::FrameChain;
pub use command::ThumbnailerCommand;
pub use png::encode_rgba;
pub use runner::{ProcessRunner, Runner};
pub use store::FrameStore;

#[derive(Clone, Debug, PartialEq)]
pub struct FrameRequest {
    pub video: PathBuf,
    pub out: PathBuf,
    pub size: u32,
}

pub trait FrameSource: Send + Sync {
    fn render(&self, request: &FrameRequest) -> Option<PathBuf>;
}

pub trait VideoFrames: Send + Sync {
    fn frame(&self, video: &Path, size: u32) -> Option<PathBuf>;
}

pub fn system_source() -> Box<dyn FrameSource> {
    let runner: std::sync::Arc<dyn Runner> = std::sync::Arc::new(ProcessRunner::default());
    #[cfg(target_os = "macos")]
    let sources: Vec<Box<dyn FrameSource>> =
        vec![Box::new(quicklook::QuickLookFrames::new(runner))];
    #[cfg(windows)]
    let sources: Vec<Box<dyn FrameSource>> = {
        let _ = runner;
        vec![Box::new(shell::ShellFrames)]
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let sources: Vec<Box<dyn FrameSource>> = vec![
        Box::new(freedesktop::FreedesktopCache::for_user()),
        Box::new(ThumbnailerCommand::ffmpegthumbnailer(runner.clone())),
        Box::new(ThumbnailerCommand::totem(runner)),
    ];
    #[cfg(not(any(unix, windows)))]
    let sources: Vec<Box<dyn FrameSource>> = {
        let _ = runner;
        Vec::new()
    };
    Box::new(FrameChain::new(sources))
}

pub(crate) fn non_empty(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0)
}
