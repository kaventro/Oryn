use std::fs::File;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use symphonia::core::codecs::{CodecParameters, CODEC_TYPE_NULL};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, MetadataRevision, StandardTagKey};
use symphonia::core::probe::Hint;
use symphonia::core::units::TimeBase;

use super::{AudioStream, MediaInfo, MediaProbe, Tags};
use crate::preview::extension;

const EXTENSIONS: &[&str] = &[
    "mp3", "wav", "flac", "ogg", "oga", "m4a", "aac", "mp4", "m4v", "mov",
];
const MAX_PACKETS: usize = 4_000_000;

pub struct SymphoniaProbe;

impl MediaProbe for SymphoniaProbe {
    fn accepts(&self, ext: &str) -> bool {
        EXTENSIONS.contains(&ext)
    }

    fn probe(&self, path: &Path) -> Result<MediaInfo> {
        let file = File::open(path).with_context(|| format!("can't open {}", path.display()))?;
        let stream = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        hint.with_extension(&extension(path));
        let mut probed = symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .context("this file has no audio Oryn can read")?;
        let mut tags = Tags::default();
        if let Some(rev) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
            read_tags(rev, &mut tags);
        }
        if let Some(rev) = probed.format.metadata().current() {
            read_tags(rev, &mut tags);
        }
        let track = probed
            .format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .context("this file has no audio track")?;
        let id = track.id;
        let params = track.codec_params.clone();
        let duration = declared_length(&params)
            .or_else(|| counted_length(probed.format.as_mut(), id, &params));
        Ok(MediaInfo {
            duration,
            audio: Some(AudioStream {
                codec: codec_name(&params),
                sample_rate: params.sample_rate,
                channels: params.channels.map(|c| c.count() as u16),
            }),
            video: None,
            tags,
        })
    }
}

fn time_base(params: &CodecParameters) -> Option<TimeBase> {
    params
        .time_base
        .or_else(|| params.sample_rate.map(|r| TimeBase::new(1, r)))
}

fn to_duration(base: TimeBase, ts: u64) -> Duration {
    let t = base.calc_time(ts);
    Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac.clamp(0.0, 1.0))
}

fn declared_length(params: &CodecParameters) -> Option<Duration> {
    Some(to_duration(time_base(params)?, params.n_frames?))
}

fn counted_length(
    format: &mut dyn FormatReader,
    track: u32,
    params: &CodecParameters,
) -> Option<Duration> {
    let base = time_base(params)?;
    let mut total = 0u64;
    for _ in 0..MAX_PACKETS {
        match format.next_packet() {
            Ok(p) if p.track_id() == track => total = total.saturating_add(p.dur),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    (total > 0).then(|| to_duration(base, total))
}

fn codec_name(params: &CodecParameters) -> String {
    let Some(desc) = symphonia::default::get_codecs().get_codec(params.codec) else {
        return String::new();
    };
    match desc.short_name {
        n if n.starts_with("pcm") => "PCM".into(),
        "vorbis" => "Vorbis".into(),
        n => n.to_uppercase(),
    }
}

fn read_tags(rev: &MetadataRevision, tags: &mut Tags) {
    for tag in rev.tags() {
        let slot = match tag.std_key {
            Some(StandardTagKey::TrackTitle) => &mut tags.title,
            Some(StandardTagKey::Artist) => &mut tags.artist,
            Some(StandardTagKey::Album) => &mut tags.album,
            _ => continue,
        };
        let value = tag.value.to_string();
        if slot.is_none() && !value.trim().is_empty() {
            *slot = Some(value.trim().to_string());
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn wav(path: &Path, seconds: u32, rate: u32, channels: u16) {
        let frames = seconds * rate;
        let data = frames * u32::from(channels) * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        b.extend_from_slice(&(channels * 2).to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data.to_le_bytes());
        b.resize(b.len() + data as usize, 0);
        std::fs::write(path, b).unwrap();
    }

    #[test]
    fn reads_wav_length_and_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        wav(&path, 3, 8_000, 2);
        let info = SymphoniaProbe.probe(&path).unwrap();
        assert_eq!(info.duration, Some(Duration::from_secs(3)));
        let audio = info.audio.unwrap();
        assert_eq!(audio.codec, "PCM");
        assert_eq!(audio.sample_rate, Some(8_000));
        assert_eq!(audio.channels, Some(2));
    }

    #[test]
    fn rejects_files_that_are_not_audio() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake.mp3");
        std::fs::write(&path, b"not really audio").unwrap();
        assert!(SymphoniaProbe.probe(&path).is_err());
        assert!(SymphoniaProbe.accepts("flac"));
        assert!(!SymphoniaProbe.accepts("mkv"));
    }
}
