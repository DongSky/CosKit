//! Small lossless float previews. They are display proxies, never restoration data.
use crate::{FormatError, Result};
use photocraft_doc::{ColorMode, SampleType};
use std::io::Read;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PreviewMetadata {
    pub width: u32,
    pub height: u32,
    pub mode: ColorMode,
    pub depth: SampleType,
    pub icc: Option<Vec<u8>>,
}
fn pixels(m: &PreviewMetadata) -> Result<usize> {
    if m.width == 0 || m.height == 0 || m.width > 768 || m.height > 768 || m.icc.as_ref().is_some_and(|v| v.len() > 4 << 20) {
        return Err(FormatError::LimitExceeded("version preview dimensions/profile".into()));
    }
    Ok(m.width as usize * m.height as usize)
}
pub fn encode(meta: &PreviewMetadata, px: &[[f32; 4]]) -> Result<Vec<u8>> {
    if px.len() != pixels(meta)? || px.iter().flatten().any(|v| !v.is_finite()) {
        return Err(FormatError::corrupt("invalid preview pixels"));
    }
    let header = serde_json::to_vec(meta)?;
    if header.len() > (8 << 20) - 12 {
        return Err(FormatError::LimitExceeded("preview header limit".into()));
    }
    let mut out = b"CKPREV01".to_vec();
    out.extend_from_slice(&(header.len() as u32).to_le_bytes());
    out.extend_from_slice(&header);
    let data: Vec<u8> = px.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
    out.extend(ruzstd::encoding::compress_to_vec(data.as_slice(), ruzstd::encoding::CompressionLevel::Fastest));
    Ok(out)
}
pub fn decode(bytes: &[u8]) -> Result<(PreviewMetadata, Vec<[f32; 4]>)> {
    if bytes.get(..8) != Some(b"CKPREV01") {
        return Err(FormatError::corrupt("invalid version preview"));
    }
    let len: [u8; 4] = bytes.get(8..12).and_then(|v| v.try_into().ok()).ok_or_else(|| FormatError::corrupt("truncated preview"))?;
    let end = 12usize.checked_add(u32::from_le_bytes(len) as usize).filter(|n| *n <= (8 << 20)).ok_or_else(|| FormatError::corrupt("preview header limit"))?;
    let meta: PreviewMetadata = serde_json::from_slice(bytes.get(12..end).ok_or_else(|| FormatError::corrupt("truncated preview header"))?)?;
    let count = pixels(&meta)?;
    let data = bytes.get(end..).ok_or_else(|| FormatError::corrupt("truncated preview pixels"))?;
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(data).map_err(|e| FormatError::corrupt(e.to_string()))?;
    let mut raw = Vec::new();
    (&mut decoder).take((count * 16 + 1) as u64).read_to_end(&mut raw)?;
    if raw.len() != count * 16 {
        return Err(FormatError::corrupt("preview pixel length mismatch"));
    }
    let mut px = Vec::with_capacity(count);
    for chunk in raw.chunks_exact(16) {
        let mut pixel = [0.0; 4];
        for (out, b) in pixel.iter_mut().zip(chunk.chunks_exact(4)) {
            let four = b.try_into().map_err(|_| FormatError::corrupt("preview sample"))?;
            *out = f32::from_le_bytes(four);
            if !out.is_finite() {
                return Err(FormatError::corrupt("non-finite preview sample"));
            }
        }
        px.push(pixel);
    }
    Ok((meta, px))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn float_preview_roundtrips_hdr_signed_zero_and_profile() {
        for depth in SampleType::ALL {
            let meta = PreviewMetadata { width: 2, height: 1, mode: ColorMode::Lab, depth, icc: Some(vec![0, 255, 42]) };
            let px = vec![[2.5, -0.0, -0.75, 1.0], [0.0, 0.5, 16.0, 0.25]];
            let bytes = encode(&meta, &px).unwrap();
            let (back, pixels) = decode(&bytes).unwrap();
            assert_eq!(back.icc, meta.icc);
            assert_eq!(back.mode, meta.mode);
            assert_eq!(back.depth, depth);
            assert_eq!(pixels.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>(), px.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>());
            for i in 0..bytes.len() {
                assert!(decode(&bytes[..i]).is_err(), "prefix {i}");
            }
        }
    }
    #[test]
    fn preview_rejects_overlarge_invalid_or_nonfinite_inputs() {
        let mut meta = PreviewMetadata { width: 1, height: 1, mode: ColorMode::Rgb, depth: SampleType::F32, icc: None };
        assert!(encode(&meta, &[[f32::NAN; 4]]).is_err());
        assert!(encode(&meta, &[]).is_err());
        meta.width = u32::MAX;
        assert!(encode(&meta, &[[0.0; 4]]).is_err());
        assert!(decode(b"CKPREV01\xff\xff\xff\xff").is_err());
    }
}
