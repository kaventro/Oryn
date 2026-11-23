use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use super::{non_empty, FrameRequest, FrameSource, Runner};

type Args = fn(&FrameRequest) -> Vec<OsString>;

pub struct ThumbnailerCommand {
    program: &'static str,
    args: Args,
    runner: Arc<dyn Runner>,
}

impl ThumbnailerCommand {
    pub fn new(program: &'static str, args: Args, runner: Arc<dyn Runner>) -> Self {
        Self {
            program,
            args,
            runner,
        }
    }

    pub fn ffmpegthumbnailer(runner: Arc<dyn Runner>) -> Self {
        Self::new(
            "ffmpegthumbnailer",
            |r| {
                vec![
                    "-i".into(),
                    r.video.clone().into(),
                    "-o".into(),
                    r.out.clone().into(),
                    "-s".into(),
                    r.size.to_string().into(),
                    "-c".into(),
                    "png".into(),
                ]
            },
            runner,
        )
    }

    pub fn totem(runner: Arc<dyn Runner>) -> Self {
        Self::new(
            "totem-video-thumbnailer",
            |r| {
                vec![
                    "-s".into(),
                    r.size.to_string().into(),
                    r.video.clone().into(),
                    r.out.clone().into(),
                ]
            },
            runner,
        )
    }
}

impl FrameSource for ThumbnailerCommand {
    fn render(&self, request: &FrameRequest) -> Option<PathBuf> {
        let args = (self.args)(request);
        (self.runner.run(self.program, &args) && non_empty(&request.out))
            .then(|| request.out.clone())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use parking_lot::Mutex;

    use super::*;

    #[derive(Default)]
    pub struct Recorder {
        pub calls: Mutex<Vec<(String, Vec<OsString>)>>,
        pub writes: Option<PathBuf>,
    }

    impl Runner for Recorder {
        fn run(&self, program: &str, args: &[OsString]) -> bool {
            self.calls.lock().push((program.to_string(), args.to_vec()));
            if let Some(path) = &self.writes {
                std::fs::write(path, b"png").unwrap();
            }
            self.writes.is_some()
        }
    }

    #[test]
    fn passes_the_video_as_a_plain_argument() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("f.png");
        let runner = Arc::new(Recorder {
            writes: Some(out.clone()),
            ..Default::default()
        });
        let source = ThumbnailerCommand::ffmpegthumbnailer(runner.clone());
        let req = FrameRequest {
            video: "/videos/-rf; echo hi.mp4".into(),
            out: out.clone(),
            size: 512,
        };
        assert_eq!(source.render(&req), Some(out.clone()));
        let calls = runner.calls.lock();
        assert_eq!(calls[0].0, "ffmpegthumbnailer");
        assert_eq!(calls[0].1[1], OsString::from("/videos/-rf; echo hi.mp4"));
        assert_eq!(calls[0].1[5], OsString::from("512"));
    }

    #[test]
    fn fails_when_the_tool_writes_nothing() {
        let runner = Arc::new(Recorder::default());
        let source = ThumbnailerCommand::totem(runner);
        let req = FrameRequest {
            video: "/v.mkv".into(),
            out: "/nowhere/f.png".into(),
            size: 256,
        };
        assert_eq!(source.render(&req), None);
    }
}
