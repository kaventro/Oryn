use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{App, Global};
use oryn_core::media::frames::{FrameStore, VideoFrames};
use oryn_core::media::{MediaProbe, ProbeRouter};

use super::{AudioOutput, Launcher, RodioOutput, SystemLauncher};

#[derive(Clone)]
pub struct MediaServices {
    pub probe: Arc<dyn MediaProbe>,
    pub frames: Arc<dyn VideoFrames>,
    pub audio: Rc<dyn AudioOutput>,
    pub launcher: Rc<dyn Launcher>,
}

impl Global for MediaServices {}

impl Default for MediaServices {
    fn default() -> Self {
        Self {
            probe: Arc::new(ProbeRouter::default()),
            frames: Arc::new(FrameStore::system()),
            audio: Rc::new(RodioOutput::default()),
            launcher: Rc::new(SystemLauncher),
        }
    }
}

impl MediaServices {
    pub fn from_app(cx: &mut App) -> Self {
        cx.default_global::<Self>().clone()
    }
}
