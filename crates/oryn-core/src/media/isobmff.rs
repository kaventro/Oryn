use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use super::{AudioStream, MediaInfo, MediaProbe, VideoStream};

const EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "m4a", "3gp"];
const MAX_MOOV: u64 = 64 * 1024 * 1024;

pub struct IsoBmffProbe;

impl MediaProbe for IsoBmffProbe {
    fn accepts(&self, ext: &str) -> bool {
        EXTENSIONS.contains(&ext)
    }

    fn probe(&self, path: &Path) -> Result<MediaInfo> {
        let mut file =
            File::open(path).with_context(|| format!("can't open {}", path.display()))?;
        let moov = find_moov(&mut file)?;
        Ok(parse_moov(&moov))
    }
}

fn find_moov(file: &mut File) -> Result<Vec<u8>> {
    let len = file.metadata()?.len();
    let mut pos = 0u64;
    while pos + 8 <= len {
        file.seek(SeekFrom::Start(pos))?;
        let mut head = [0u8; 16];
        file.read_exact(&mut head[..8])?;
        let mut size = u64::from(u32::from_be_bytes([head[0], head[1], head[2], head[3]]));
        let mut header = 8u64;
        if size == 1 {
            file.read_exact(&mut head[8..16])?;
            size = u64::from_be_bytes(head[8..16].try_into()?);
            header = 16;
        } else if size == 0 {
            size = len - pos;
        }
        if size < header || pos + size > len {
            bail!("this video file is damaged");
        }
        if &head[4..8] == b"moov" {
            let body = size - header;
            if body > MAX_MOOV {
                bail!("this video's header is too large to read");
            }
            let mut buf = vec![0u8; body as usize];
            file.read_exact(&mut buf)?;
            return Ok(buf);
        }
        pos += size;
    }
    bail!("this file has no movie header")
}

struct Boxes<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for Boxes<'a> {
    type Item = (&'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        let d = self.data;
        let size = u32_at(d, self.pos)? as usize;
        let kind = d.get(self.pos + 4..self.pos + 8)?;
        let (start, end) = match size {
            0 => (self.pos + 8, d.len()),
            1 => {
                let large = usize::try_from(u64_at(d, self.pos + 8)?).ok()?;
                (self.pos + 16, self.pos.checked_add(large)?)
            }
            n if n >= 8 => (self.pos + 8, self.pos.checked_add(n)?),
            _ => return None,
        };
        let body = d.get(start..end)?;
        self.pos = end;
        Some((kind, body))
    }
}

fn boxes(data: &[u8]) -> Boxes<'_> {
    Boxes { data, pos: 0 }
}

fn child<'a>(data: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
    boxes(data).find(|(k, _)| *k == kind).map(|(_, b)| b)
}

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

fn parse_moov(moov: &[u8]) -> MediaInfo {
    let mut info = MediaInfo {
        duration: child(moov, b"mvhd").and_then(movie_length),
        ..Default::default()
    };
    for (kind, trak) in boxes(moov) {
        if kind != b"trak" {
            continue;
        }
        let Some(track) = Track::parse(trak) else {
            continue;
        };
        match &track.handler {
            b"vide" if info.video.is_none() => info.video = Some(track.video()),
            b"soun" if info.audio.is_none() => info.audio = Some(track.audio()),
            _ => {}
        }
    }
    info
}

fn movie_length(mvhd: &[u8]) -> Option<Duration> {
    let (scale, length) = match mvhd.first()? {
        1 => (u32_at(mvhd, 20)?, u64_at(mvhd, 24)?),
        _ => (u32_at(mvhd, 12)?, u64::from(u32_at(mvhd, 16)?)),
    };
    (scale > 0 && length != u64::MAX && length != u64::from(u32::MAX))
        .then(|| Duration::from_secs_f64(length as f64 / f64::from(scale)))
}

struct Track<'a> {
    handler: [u8; 4],
    display: (u32, u32),
    entry: &'a [u8],
    codec: [u8; 4],
}

impl<'a> Track<'a> {
    fn parse(trak: &'a [u8]) -> Option<Self> {
        let mdia = child(trak, b"mdia")?;
        let handler: [u8; 4] = child(mdia, b"hdlr")?.get(8..12)?.try_into().ok()?;
        let stsd = child(child(child(mdia, b"minf")?, b"stbl")?, b"stsd")?;
        let (codec, entry) = boxes(stsd.get(8..)?).next()?;
        let display = child(trak, b"tkhd")
            .and_then(display_size)
            .unwrap_or((0, 0));
        Some(Self {
            handler,
            display,
            entry,
            codec: codec.try_into().ok()?,
        })
    }

    fn video(&self) -> VideoStream {
        let coded = (
            u16_at(self.entry, 24).map(u32::from).unwrap_or(0),
            u16_at(self.entry, 26).map(u32::from).unwrap_or(0),
        );
        let (width, height) = if self.display.0 > 0 && self.display.1 > 0 {
            self.display
        } else {
            coded
        };
        VideoStream {
            codec: codec_label(&self.codec),
            width,
            height,
        }
    }

    fn audio(&self) -> AudioStream {
        let channels = u16_at(self.entry, 16).filter(|c| *c > 0);
        let rate = u32_at(self.entry, 24).map(|r| r >> 16).filter(|r| *r > 0);
        AudioStream {
            codec: codec_label(&self.codec),
            sample_rate: rate,
            channels,
        }
    }
}

fn display_size(tkhd: &[u8]) -> Option<(u32, u32)> {
    let at = if *tkhd.first()? == 1 { 88 } else { 76 };
    Some((u32_at(tkhd, at)? >> 16, u32_at(tkhd, at + 4)? >> 16))
}

fn codec_label(fourcc: &[u8; 4]) -> String {
    let name = match fourcc {
        b"avc1" | b"avc3" => "H.264",
        b"hvc1" | b"hev1" => "HEVC",
        b"av01" => "AV1",
        b"vp09" => "VP9",
        b"vp08" => "VP8",
        b"mp4v" => "MPEG-4",
        b"apch" | b"apcn" | b"apcs" | b"apco" | b"ap4h" | b"ap4x" => "Apple ProRes",
        b"jpeg" | b"mjpa" => "Motion JPEG",
        b"mp4a" => "AAC",
        b"ac-3" => "AC-3",
        b"ec-3" => "E-AC-3",
        b"alac" => "ALAC",
        b"Opus" => "Opus",
        b"fLaC" => "FLAC",
        b"lpcm" | b"sowt" | b"twos" => "PCM",
        _ => return String::from_utf8_lossy(fourcc).trim().to_string(),
    };
    name.to_string()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn atom(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        out
    }

    fn full(version: u8, body: &[u8]) -> Vec<u8> {
        let mut out = vec![version, 0, 0, 0];
        out.extend_from_slice(body);
        out
    }

    fn trak(handler: &[u8; 4], codec: &[u8; 4], entry: &[u8], size: (u32, u32)) -> Vec<u8> {
        let mut tkhd = vec![0u8; 72];
        tkhd.extend_from_slice(&(size.0 << 16).to_be_bytes());
        tkhd.extend_from_slice(&(size.1 << 16).to_be_bytes());
        let mut hdlr = vec![0u8; 4];
        hdlr.extend_from_slice(handler);
        hdlr.extend_from_slice(&[0u8; 12]);
        let mut stsd = full(0, &1u32.to_be_bytes());
        stsd.extend(atom(codec, entry));
        let stbl = atom(b"stbl", &atom(b"stsd", &stsd));
        let minf = atom(b"minf", &stbl);
        let mut mdia = atom(b"hdlr", &full(0, &hdlr));
        mdia.extend(minf);
        let mut t = atom(b"tkhd", &full(0, &tkhd));
        t.extend(atom(b"mdia", &mdia));
        atom(b"trak", &t)
    }

    pub fn movie(path: &Path, codec: &[u8; 4], width: u32, height: u32, seconds: u32) {
        let mut mvhd = vec![0u8; 8];
        mvhd.extend_from_slice(&600u32.to_be_bytes());
        mvhd.extend_from_slice(&(seconds * 600).to_be_bytes());
        mvhd.extend_from_slice(&[0u8; 80]);
        let mut video_entry = vec![0u8; 24];
        video_entry.extend_from_slice(&(width as u16).to_be_bytes());
        video_entry.extend_from_slice(&(height as u16).to_be_bytes());
        video_entry.extend_from_slice(&[0u8; 50]);
        let mut audio_entry = vec![0u8; 16];
        audio_entry.extend_from_slice(&2u16.to_be_bytes());
        audio_entry.extend_from_slice(&16u16.to_be_bytes());
        audio_entry.extend_from_slice(&[0u8; 4]);
        audio_entry.extend_from_slice(&(44_100u32 << 16).to_be_bytes());
        let mut moov = atom(b"mvhd", &full(0, &mvhd));
        moov.extend(trak(b"vide", codec, &video_entry, (width, height)));
        moov.extend(trak(b"soun", b"mp4a", &audio_entry, (0, 0)));
        let mut file = atom(b"ftyp", b"isom\0\0\0\0isom");
        file.extend(atom(b"mdat", &[0u8; 32]));
        file.extend(atom(b"moov", &moov));
        std::fs::write(path, file).unwrap();
    }

    #[test]
    fn reads_resolution_codec_and_length() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clip.mov");
        movie(&path, b"hvc1", 3840, 2160, 42);
        let info = IsoBmffProbe.probe(&path).unwrap();
        assert_eq!(info.duration, Some(Duration::from_secs(42)));
        assert_eq!(
            info.video,
            Some(VideoStream {
                codec: "HEVC".into(),
                width: 3840,
                height: 2160
            })
        );
        let audio = info.audio.unwrap();
        assert_eq!(audio.codec, "AAC");
        assert_eq!(audio.channels, Some(2));
        assert_eq!(audio.sample_rate, Some(44_100));
    }

    #[test]
    fn rejects_damaged_and_unknown_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.mp4");
        std::fs::write(&path, [0, 0, 0, 200, b'f', b't', b'y', b'p']).unwrap();
        assert!(IsoBmffProbe.probe(&path).is_err());
        std::fs::write(&path, atom(b"ftyp", b"isom")).unwrap();
        assert!(IsoBmffProbe.probe(&path).is_err());
    }

    #[test]
    fn survives_truncated_boxes() {
        let moov = atom(b"trak", &[0, 0, 0, 99, b'm', b'd']);
        assert_eq!(parse_moov(&moov), MediaInfo::default());
        assert_eq!(codec_label(b"xyz "), "xyz");
    }
}
