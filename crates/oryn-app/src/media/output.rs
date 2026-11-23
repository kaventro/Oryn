use std::path::Path;
use std::time::Duration;

use anyhow::Result;

pub trait AudioPlayer {
    fn play(&mut self);
    fn pause(&mut self);
    fn is_playing(&self) -> bool;
    fn is_finished(&self) -> bool;
    fn position(&self) -> Duration;
    fn seek(&mut self, to: Duration) -> Result<()>;
}

pub trait AudioOutput {
    fn open(&self, path: &Path) -> Result<Box<dyn AudioPlayer>>;
}

pub trait Launcher {
    fn launch(&self, path: &Path);
}

pub struct SystemLauncher;

impl Launcher for SystemLauncher {
    fn launch(&self, path: &Path) {
        crate::opener::open(path);
    }
}
