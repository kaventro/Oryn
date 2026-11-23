use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, bail};
use oryn_core::media::frames::VideoFrames;
use oryn_core::media::{MediaInfo, MediaProbe, VideoStream};

use super::{AudioOutput, AudioPlayer, Launcher, MediaServices};

#[derive(Default)]
pub struct Log {
    pub events: RefCell<Vec<String>>,
}

impl Log {
    pub fn push(&self, e: impl Into<String>) {
        self.events.borrow_mut().push(e.into());
    }

    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.events.borrow_mut())
    }
}

pub struct FakeProbe;

impl MediaProbe for FakeProbe {
    fn accepts(&self, _: &str) -> bool {
        true
    }

    fn probe(&self, path: &Path) -> Result<MediaInfo> {
        if path.to_string_lossy().contains("broken") {
            bail!("this file has no audio Oryn can read");
        }
        Ok(MediaInfo {
            duration: Some(Duration::from_secs(100)),
            video: (oryn_core::preview::extension(path) == "mp4").then(|| VideoStream {
                codec: "H.264".into(),
                width: 1280,
                height: 720,
            }),
            ..Default::default()
        })
    }
}

pub struct FakeFrames;

impl VideoFrames for FakeFrames {
    fn frame(&self, video: &Path, _: u32) -> Option<PathBuf> {
        Some(video.with_extension("png"))
    }
}

pub struct FakeOutput(pub Rc<Log>);

struct FakePlayer {
    log: Rc<Log>,
    playing: bool,
    at: Duration,
}

impl AudioOutput for FakeOutput {
    fn open(&self, path: &Path) -> Result<Box<dyn AudioPlayer>> {
        if path.to_string_lossy().contains("broken") {
            bail!("this audio format can't be played");
        }
        self.0.push("open");
        Ok(Box::new(FakePlayer {
            log: self.0.clone(),
            playing: false,
            at: Duration::ZERO,
        }))
    }
}

impl AudioPlayer for FakePlayer {
    fn play(&mut self) {
        self.playing = true;
        self.log.push("play");
    }

    fn pause(&mut self) {
        self.playing = false;
        self.log.push("pause");
    }

    fn is_playing(&self) -> bool {
        self.playing
    }

    fn is_finished(&self) -> bool {
        false
    }

    fn position(&self) -> Duration {
        self.at
    }

    fn seek(&mut self, to: Duration) -> Result<()> {
        self.at = to;
        self.log.push(format!("seek {}", to.as_secs()));
        Ok(())
    }
}

pub struct FakeLauncher(pub Rc<Log>);

impl Launcher for FakeLauncher {
    fn launch(&self, path: &Path) {
        self.0.push(format!(
            "launch {}",
            path.file_name().unwrap().to_string_lossy()
        ));
    }
}

pub fn services(log: &Rc<Log>) -> MediaServices {
    MediaServices {
        probe: Arc::new(FakeProbe),
        frames: Arc::new(FakeFrames),
        audio: Rc::new(FakeOutput(log.clone())),
        launcher: Rc::new(FakeLauncher(log.clone())),
    }
}
