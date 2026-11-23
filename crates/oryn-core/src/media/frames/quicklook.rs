use std::path::PathBuf;
use std::sync::Arc;

use super::{non_empty, FrameRequest, FrameSource, Runner};

pub struct QuickLookFrames {
    runner: Arc<dyn Runner>,
}

impl QuickLookFrames {
    pub fn new(runner: Arc<dyn Runner>) -> Self {
        Self { runner }
    }
}

impl FrameSource for QuickLookFrames {
    fn render(&self, request: &FrameRequest) -> Option<PathBuf> {
        let work = request.out.with_extension("ql");
        std::fs::create_dir_all(&work).ok()?;
        let args = [
            "-t".into(),
            "-s".into(),
            request.size.to_string().into(),
            "-o".into(),
            work.clone().into(),
            request.video.clone().into(),
        ];
        let made = work.join(format!(
            "{}.png",
            request.video.file_name()?.to_string_lossy()
        ));
        let ok = self.runner.run("/usr/bin/qlmanage", &args)
            && non_empty(&made)
            && std::fs::rename(&made, &request.out).is_ok();
        let _ = std::fs::remove_dir_all(&work);
        ok.then(|| request.out.clone())
    }
}
