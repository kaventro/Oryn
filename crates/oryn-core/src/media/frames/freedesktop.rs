use std::path::{Path, PathBuf};

use md5::{Digest, Md5};

use super::{non_empty, FrameRequest, FrameSource};

pub struct FreedesktopCache {
    root: PathBuf,
}

impl FreedesktopCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn for_user() -> Self {
        let root = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
            .unwrap_or_default()
            .join("thumbnails");
        Self::new(root)
    }
}

impl FrameSource for FreedesktopCache {
    fn render(&self, request: &FrameRequest) -> Option<PathBuf> {
        let name = format!("{}.png", hex::encode(Md5::digest(file_uri(&request.video))));
        let video_time = std::fs::metadata(&request.video).ok()?.modified().ok()?;
        ["xx-large", "x-large", "large", "normal"]
            .iter()
            .map(|size| self.root.join(size).join(&name))
            .find(|p| {
                non_empty(p)
                    && std::fs::metadata(p)
                        .and_then(|m| m.modified())
                        .is_ok_and(|t| t >= video_time)
            })
    }
}

pub fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for &b in path.as_os_str().as_encoded_bytes() {
        if b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@/".contains(&b) {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_uri_the_thumbnail_spec_hashes() {
        assert_eq!(
            file_uri(Path::new("/home/me/My Clip #1.mp4")),
            "file:///home/me/My%20Clip%20%231.mp4"
        );
        assert_eq!(
            file_uri(Path::new("/tmp/\u{e9}.mkv")),
            "file:///tmp/%C3%A9.mkv"
        );
    }

    #[test]
    fn finds_a_fresh_cached_thumbnail() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        std::fs::write(&video, b"v").unwrap();
        let cache = FreedesktopCache::new(dir.path().join("thumbnails"));
        let req = FrameRequest {
            video: video.clone(),
            out: dir.path().join("out.png"),
            size: 512,
        };
        assert_eq!(cache.render(&req), None);
        let name = format!("{}.png", hex::encode(Md5::digest(file_uri(&video))));
        let large = dir.path().join("thumbnails/large");
        std::fs::create_dir_all(&large).unwrap();
        std::fs::write(large.join(&name), b"png").unwrap();
        assert_eq!(cache.render(&req), Some(large.join(name)));
    }
}
