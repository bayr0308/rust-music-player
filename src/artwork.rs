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
