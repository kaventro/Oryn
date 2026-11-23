use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;

pub fn encode_rgba(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let stride = (width as usize).checked_mul(4)?;
    if width == 0 || height == 0 || rgba.len() != stride.checked_mul(height as usize)? {
        return None;
    }
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in rgba.chunks_exact(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = ZlibEncoder::new(Vec::new(), Compression::fast());
    z.write_all(&raw).ok()?;
    let data = z.finish().ok()?;
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &data);
    chunk(&mut out, b"IEND", &[]);
    Some(out)
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut crc = flate2::Crc::new();
    crc.update(kind);
    crc.update(data);
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc.sum().to_be_bytes());
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn writes_a_valid_png() {
        let pixels = [255, 0, 0, 255, 0, 255, 0, 255];
        let png = encode_rgba(2, 1, &pixels).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 2);
        let idat_len = u32::from_be_bytes(png[33..37].try_into().unwrap()) as usize;
        assert_eq!(&png[37..41], b"IDAT");
        let mut raw = Vec::new();
        flate2::read::ZlibDecoder::new(&png[41..41 + idat_len])
            .read_to_end(&mut raw)
            .unwrap();
        assert_eq!(raw, [0, 255, 0, 0, 255, 0, 255, 0, 255]);
        assert!(png.ends_with(&[0xae, 0x42, 0x60, 0x82]));
    }

    #[test]
    fn rejects_mismatched_buffers() {
        assert!(encode_rgba(2, 2, &[0; 4]).is_none());
        assert!(encode_rgba(0, 0, &[]).is_none());
    }
}
