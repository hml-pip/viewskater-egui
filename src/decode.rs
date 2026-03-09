use eframe::egui;

/// Convert a DynamicImage directly to egui's ColorImage, bypassing both
/// image crate v0.25's slow CICP color space conversion and egui's
/// per-pixel `from_rgba_unmultiplied` conversion. Goes straight from
/// decoded pixel data to `Vec<Color32>`.
pub fn image_to_color_image(img: image::DynamicImage) -> egui::ColorImage {
    use image::DynamicImage;
    match img {
        DynamicImage::ImageRgb8(buf) => {
            let w = buf.width() as usize;
            let h = buf.height() as usize;
            let rgb = buf.into_raw();
            let pixels: Vec<egui::Color32> = rgb
                .chunks_exact(3)
                .map(|c| egui::Color32::from_rgb(c[0], c[1], c[2]))
                .collect();
            egui::ColorImage {
                size: [w, h],
                pixels,
            }
        }
        DynamicImage::ImageRgba8(buf) => {
            let w = buf.width() as usize;
            let h = buf.height() as usize;
            let rgba = buf.into_raw();
            let pixels: Vec<egui::Color32> = rgba
                .chunks_exact(4)
                .map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
                .collect();
            egui::ColorImage {
                size: [w, h],
                pixels,
            }
        }
        other => {
            let rgba = other.into_rgba8();
            let w = rgba.width() as usize;
            let h = rgba.height() as usize;
            let pixels = rgba.into_raw();
            egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels)
        }
    }
}
