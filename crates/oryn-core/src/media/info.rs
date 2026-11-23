use std::time::Duration;

use super::clock;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaInfo {
    pub duration: Option<Duration>,
    pub audio: Option<AudioStream>,
    pub video: Option<VideoStream>,
    pub tags: Tags,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioStream {
    pub codec: String,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VideoStream {
    pub codec: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

impl MediaInfo {
    pub fn facts(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if let Some(d) = self.duration {
            out.push(("Duration", clock(d)));
        }
        if let Some(v) = &self.video {
            if v.width > 0 && v.height > 0 {
                out.push(("Resolution", format!("{} \u{00d7} {}", v.width, v.height)));
            }
            if !v.codec.is_empty() {
                out.push(("Video", v.codec.clone()));
            }
        }
        if let Some(a) = &self.audio {
            if !a.codec.is_empty() {
                out.push(("Audio", a.codec.clone()));
            }
            if let Some(rate) = a.sample_rate {
                out.push((
                    "Sample rate",
                    format!("{:.1} kHz", f64::from(rate) / 1000.0),
                ));
            }
            if let Some(ch) = a.channels {
                out.push(("Channels", channel_label(ch)));
            }
        }
        out
    }

    pub fn merge(mut self, other: MediaInfo) -> MediaInfo {
        self.duration = self.duration.or(other.duration);
        self.audio = self.audio.or(other.audio);
        self.video = self.video.or(other.video);
        self.tags.title = self.tags.title.or(other.tags.title);
        self.tags.artist = self.tags.artist.or(other.tags.artist);
        self.tags.album = self.tags.album.or(other.tags.album);
        self
    }
}

fn channel_label(n: u16) -> String {
    match n {
        1 => "Mono".into(),
        2 => "Stereo".into(),
        n => format!("{n} channels"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_video_facts_in_order() {
        let info = MediaInfo {
            duration: Some(Duration::from_secs(95)),
            video: Some(VideoStream {
                codec: "H.264".into(),
                width: 1920,
                height: 1080,
            }),
            audio: Some(AudioStream {
                codec: "AAC".into(),
                sample_rate: Some(48_000),
                channels: Some(2),
            }),
            ..Default::default()
        };
        assert_eq!(
            info.facts(),
            vec![
                ("Duration", "1:35".to_string()),
                ("Resolution", "1920 \u{00d7} 1080".to_string()),
                ("Video", "H.264".to_string()),
                ("Audio", "AAC".to_string()),
                ("Sample rate", "48.0 kHz".to_string()),
                ("Channels", "Stereo".to_string()),
            ]
        );
    }

    #[test]
    fn skips_unknown_values() {
        let info = MediaInfo {
            video: Some(VideoStream::default()),
            audio: Some(AudioStream {
                channels: Some(6),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(info.facts(), vec![("Channels", "6 channels".to_string())]);
    }

    #[test]
    fn merge_keeps_known_values_first() {
        let a = MediaInfo {
            duration: Some(Duration::from_secs(1)),
            ..Default::default()
        };
        let b = MediaInfo {
            duration: Some(Duration::from_secs(9)),
            tags: Tags {
                title: Some("Song".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let m = a.merge(b);
        assert_eq!(m.duration, Some(Duration::from_secs(1)));
        assert_eq!(m.tags.title.as_deref(), Some("Song"));
    }
}
