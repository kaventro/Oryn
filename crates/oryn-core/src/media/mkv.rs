use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use matroska::{Matroska, Settings};

use super::{AudioStream, MediaInfo, MediaProbe, Tags, VideoStream};

pub struct MatroskaProbe;

impl MediaProbe for MatroskaProbe {
    fn accepts(&self, ext: &str) -> bool {
        matches!(ext, "mkv" | "webm" | "mka")
    }

    fn probe(&self, path: &Path) -> Result<MediaInfo> {
        let file = File::open(path).with_context(|| format!("can't open {}", path.display()))?;
        let mkv = Matroska::open(file).context("this Matroska file can't be read")?;
        Ok(describe(&mkv))
    }
}

fn describe(mkv: &Matroska) -> MediaInfo {
    let mut info = MediaInfo {
        duration: mkv.info.duration,
        tags: Tags {
            title: mkv.info.title.clone().filter(|t| !t.trim().is_empty()),
            ..Default::default()
        },
        ..Default::default()
    };
    for track in &mkv.tracks {
        match &track.settings {
            Settings::Video(v) if info.video.is_none() => {
                info.video = Some(VideoStream {
                    codec: codec_label(&track.codec_id),
                    width: v.display_width.unwrap_or(v.pixel_width) as u32,
                    height: v.display_height.unwrap_or(v.pixel_height) as u32,
                })
            }
            Settings::Audio(a) if info.audio.is_none() => {
                info.audio = Some(AudioStream {
                    codec: codec_label(&track.codec_id),
                    sample_rate: (a.sample_rate > 0.0).then_some(a.sample_rate as u32),
                    channels: (a.channels > 0).then_some(a.channels as u16),
                })
            }
            _ => {}
        }
    }
    info
}

fn codec_label(id: &str) -> String {
    let name = match id {
        "V_MPEG4/ISO/AVC" => "H.264",
        "V_MPEGH/ISO/HEVC" => "HEVC",
        "V_AV1" => "AV1",
        "V_VP9" => "VP9",
        "V_VP8" => "VP8",
        "V_MPEG4/ISO/ASP" | "V_MPEG4/ISO/SP" => "MPEG-4",
        "V_MPEG2" => "MPEG-2",
        "V_THEORA" => "Theora",
        "A_AAC" => "AAC",
        "A_OPUS" => "Opus",
        "A_VORBIS" => "Vorbis",
        "A_FLAC" => "FLAC",
        "A_MPEG/L3" => "MP3",
        "A_AC3" => "AC-3",
        "A_EAC3" => "E-AC-3",
        "A_DTS" => "DTS",
        id if id.starts_with("A_AAC") => "AAC",
        id if id.starts_with("A_PCM") => "PCM",
        id => return id.trim_start_matches(['V', 'A', '_']).to_string(),
    };
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_common_codecs() {
        assert_eq!(codec_label("V_MPEG4/ISO/AVC"), "H.264");
        assert_eq!(codec_label("A_AAC/MPEG4/LC"), "AAC");
        assert_eq!(codec_label("A_PCM/INT/LIT"), "PCM");
        assert_eq!(codec_label("V_PRORES"), "PRORES");
    }

    #[test]
    fn rejects_files_that_are_not_matroska() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake.mkv");
        std::fs::write(&path, b"plain text").unwrap();
        assert!(MatroskaProbe.probe(&path).is_err());
        assert!(MatroskaProbe.accepts("webm"));
    }
}
