use image::{DynamicImage, ImageFormat, RgbImage, RgbaImage};
use lofty::{file::TaggedFileExt, picture::PictureType};
use std::{fs, io::Cursor, path::Path};
use zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};

const MAX_COVER_BYTES: usize = 20 * 1024 * 1024;

/// Decode the complete image before uploading it. The image crate's lenient
/// JPEG path can report success after filling an unreadable scan with gray.
pub fn decode_cover(bytes: &[u8]) -> Result<RgbaImage, String> {
    if bytes.len() > MAX_COVER_BYTES {
        return Err("Обложка слишком большая".into());
    }
    let format = image::guess_format(bytes).map_err(|e| e.to_string())?;
    let image = if format == ImageFormat::Jpeg {
        let options = DecoderOptions::default()
            .set_strict_mode(true)
            .set_max_width(8192)
            .set_max_height(8192)
            .jpeg_set_out_colorspace(ColorSpace::RGB);
        let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(bytes), options);
        let pixels = decoder.decode().map_err(|e| e.to_string())?;
        let (width, height) = decoder.dimensions().ok_or("Нет размеров обложки")?;
        DynamicImage::ImageRgb8(
            RgbImage::from_raw(width as u32, height as u32, pixels)
                .ok_or("Некорректный размер обложки")?,
        )
    } else {
        let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        reader.limits(limits);
        reader.decode().map_err(|e| e.to_string())?
    };
    Ok(image
        .resize_to_fill(600, 600, image::imageops::FilterType::Lanczos3)
        .to_rgba8())
}

/// Prefer a valid front cover, then other embedded pictures and sidecar files.
/// A corrupt first picture must not hide a usable alternative.
pub fn load_cover(path: &Path) -> Option<RgbaImage> {
    if let Ok(file) = lofty::read_from_path(path) {
        let mut pictures: Vec<_> = file.tags().iter().flat_map(|tag| tag.pictures()).collect();
        pictures.sort_by_key(|picture| picture.pic_type() != PictureType::CoverFront);
        for picture in pictures {
            if let Ok(image) = decode_cover(picture.data()) {
                return Some(image);
            }
        }
    }
    let parent = path.parent()?;
    for name in [
        "cover.jpg",
        "cover.png",
        "folder.jpg",
        "folder.png",
        "Cover.jpg",
        "Cover.png",
    ] {
        let candidate = parent.join(name);
        if fs::metadata(&candidate).is_ok_and(|m| m.len() <= MAX_COVER_BYTES as u64)
            && let Ok(bytes) = fs::read(candidate)
            && let Ok(image) = decode_cover(&bytes)
        {
            return Some(image);
        }
    }
    None
}

/// Select a midtone color from artwork, avoiding white/black borders. The
/// result is dark enough for light text and is composited over native Acrylic.
pub fn cover_tint(image: &image::RgbaImage) -> [u8; 3] {
    let thumbnail = image::imageops::resize(image, 48, 48, image::imageops::FilterType::Triangle);
    let mut bins = [[0_u64; 4]; 512];
    for pixel in thumbnail.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < 128 {
            continue;
        }
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        if max < 35 || min > 225 || max - min < 20 {
            continue;
        }
        let index = (r as usize / 32) * 64 + (g as usize / 32) * 8 + b as usize / 32;
        let weight = (max - min) as u64;
        bins[index][0] += weight;
        bins[index][1] += r as u64 * weight;
        bins[index][2] += g as u64 * weight;
        bins[index][3] += b as u64 * weight;
    }
    let bin = bins.iter().max_by_key(|b| b[0]).unwrap();
    if bin[0] == 0 {
        return [36, 39, 45];
    }
    std::array::from_fn(|i| (18.0 + (bin[i + 1] / bin[0]) as f32 * 0.28) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_fixture() -> Vec<u8> {
        let image = RgbImage::from_fn(128, 128, |x, y| {
            image::Rgb([(x * 13 + y * 7) as u8, (y * 11) as u8, (x * 3) as u8])
        });
        let mut bytes = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut bytes)
            .encode_image(&image)
            .unwrap();
        bytes
    }

    #[test]
    fn complete_jpeg_decodes_and_corrupt_scan_is_rejected() {
        let bytes = jpeg_fixture();
        assert_eq!(decode_cover(&bytes).unwrap().dimensions(), (600, 600));
        let mut damaged = bytes.clone();
        let sos = damaged.windows(2).position(|b| b == [0xff, 0xda]).unwrap();
        let start = sos + 2 + u16::from_be_bytes([damaged[sos + 2], damaged[sos + 3]]) as usize;
        // The same invalid marker found inside the Fubuki cover's entropy data.
        damaged[start + 100..start + 102].copy_from_slice(&[0xff, 0x93]);
        assert!(decode_cover(&damaged).is_err());
        assert!(decode_cover(&bytes[..bytes.len() / 2]).is_err());
    }

    #[test]
    fn damaged_sidecar_does_not_hide_valid_alternative() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("cover.jpg"), b"broken jpeg").unwrap();
        let valid = RgbaImage::from_pixel(24, 24, image::Rgba([210, 30, 60, 255]));
        valid.save(dir.path().join("folder.png")).unwrap();
        let cover = load_cover(&dir.path().join("track.mp3")).unwrap();
        assert_eq!(cover.get_pixel(300, 300), &image::Rgba([210, 30, 60, 255]));
    }

    #[test]
    fn tint_preserves_cover_hue_without_brightening_the_surface() {
        let red = image::RgbaImage::from_pixel(10, 10, image::Rgba([220, 20, 40, 255]));
        let tint = cover_tint(&red);
        assert!(tint[0] > tint[1] * 2);
        assert!(tint.iter().all(|v| *v < 100));
        let white = image::RgbaImage::from_pixel(10, 10, image::Rgba([255, 255, 255, 255]));
        assert_eq!(cover_tint(&white), [36, 39, 45]);
    }
}
