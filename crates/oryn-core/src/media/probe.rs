use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Result};

use super::{IsoBmffProbe, MatroskaProbe, MediaInfo, SymphoniaProbe};
use crate::preview::extension;

pub trait MediaProbe: Send + Sync {
    fn accepts(&self, ext: &str) -> bool;
    fn probe(&self, path: &Path) -> Result<MediaInfo>;
}

pub struct ProbeRouter {
    probes: Vec<Arc<dyn MediaProbe>>,
}

impl ProbeRouter {
    pub fn new(probes: Vec<Arc<dyn MediaProbe>>) -> Self {
        Self { probes }
    }
}

impl Default for ProbeRouter {
    fn default() -> Self {
        Self::new(vec![
            Arc::new(IsoBmffProbe),
            Arc::new(MatroskaProbe),
            Arc::new(SymphoniaProbe),
        ])
    }
}

impl MediaProbe for ProbeRouter {
    fn accepts(&self, ext: &str) -> bool {
        self.probes.iter().any(|p| p.accepts(ext))
    }

    fn probe(&self, path: &Path) -> Result<MediaInfo> {
        let ext = extension(path);
        let mut found: Option<MediaInfo> = None;
        let mut last_error = None;
        for probe in self.probes.iter().filter(|p| p.accepts(&ext)) {
            match probe.probe(path) {
                Ok(info) => {
                    found = Some(match found {
                        Some(prev) => prev.merge(info),
                        None => info,
                    })
                }
                Err(e) => last_error = Some(e),
            }
        }
        match (found, last_error) {
            (Some(info), _) => Ok(info),
            (None, Some(e)) => Err(e),
            (None, None) => bail!("Oryn can't read .{ext} files"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::media::{Tags, VideoStream};

    struct Fixed(&'static str, Option<MediaInfo>);

    impl MediaProbe for Fixed {
        fn accepts(&self, ext: &str) -> bool {
            ext == self.0
        }

        fn probe(&self, _: &Path) -> Result<MediaInfo> {
            match &self.1 {
                Some(i) => Ok(i.clone()),
                None => bail!("broken"),
            }
        }
    }

    #[test]
    fn merges_every_probe_that_accepts_the_extension() {
        let video = MediaInfo {
            video: Some(VideoStream {
                codec: "H.264".into(),
                width: 640,
                height: 360,
            }),
            ..Default::default()
        };
        let audio = MediaInfo {
            duration: Some(Duration::from_secs(4)),
            tags: Tags {
                title: Some("Clip".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let router = ProbeRouter::new(vec![
            Arc::new(Fixed("mp4", Some(video))),
            Arc::new(Fixed("wav", None)),
            Arc::new(Fixed("mp4", Some(audio))),
        ]);
        let info = router.probe(Path::new("a.mp4")).unwrap();
        assert_eq!(info.video.unwrap().width, 640);
        assert_eq!(info.duration, Some(Duration::from_secs(4)));
        assert_eq!(info.tags.title.as_deref(), Some("Clip"));
    }

    #[test]
    fn reports_the_failure_when_nothing_reads_the_file() {
        let router = ProbeRouter::new(vec![Arc::new(Fixed("wav", None))]);
        assert_eq!(
            router.probe(Path::new("a.wav")).unwrap_err().to_string(),
            "broken"
        );
        assert!(router.probe(Path::new("a.xyz")).is_err());
        assert!(!router.accepts("xyz"));
    }

    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/media")
            .join(name)
    }

    fn seconds(info: &MediaInfo) -> f64 {
        info.duration.unwrap().as_secs_f64()
    }

    #[test]
    fn reads_every_audio_format_oryn_plays() {
        let router = ProbeRouter::default();
        for (name, codec) in [
            ("tone.mp3", "MP3"),
            ("tone.flac", "FLAC"),
            ("tone.ogg", "Vorbis"),
            ("tone.m4a", "AAC"),
        ] {
            let info = router.probe(&fixture(name)).unwrap();
            assert!(
                (seconds(&info) - 2.0).abs() < 0.2,
                "{name}: {:?}",
                info.duration
            );
            let audio = info.audio.unwrap();
            assert_eq!(audio.codec, codec, "{name}");
            assert_eq!(audio.channels, Some(1), "{name}");
            assert_eq!(audio.sample_rate, Some(8_000), "{name}");
        }
    }

    #[test]
    fn reads_real_video_headers() {
        let router = ProbeRouter::default();
        for (name, codec) in [("clip.mp4", "H.264"), ("clip.webm", "VP8")] {
            let info = router.probe(&fixture(name)).unwrap();
            let video = info.video.clone().unwrap();
            assert_eq!((video.width, video.height), (64, 36), "{name}");
            assert_eq!(video.codec, codec, "{name}");
            assert!(
                (seconds(&info) - 1.0).abs() < 0.2,
                "{name}: {:?}",
                info.duration
            );
        }
    }
}
