//! 图片处理器
//!
//! 提供图片压缩、缩略图生成、滤镜等功能

use anyhow::Result;
use image::{
    codecs::{jpeg, png},
    imageops, DynamicImage, RgbaImage,
};
use sha2::{Digest, Sha256};

// 使用领域层的参数类型
pub use crate::models::{FilterParams, WatermarkParams};

#[derive(Clone)]
pub struct ImageProcessor {
    pub max_width: u32,
    pub max_height: u32,
    pub thumbnail_size: u32,
    pub jpeg_quality: u8,
}

impl ImageProcessor {
    pub fn new(max_width: u32, max_height: u32, thumbnail_size: u32, jpeg_quality: u8) -> Self {
        Self {
            max_width,
            max_height,
            thumbnail_size,
            jpeg_quality,
        }
    }

    #[tracing::instrument(skip(self, path))]
    pub fn process_from_file(&self, path: &std::path::Path) -> Result<(Vec<u8>, Vec<u8>)> {
        let img = image::open(path)?;
        let compressed = self.compress(&img)?;
        let thumbnail = self.generate_thumbnail(&img)?;
        Ok((compressed, thumbnail))
    }

    #[tracing::instrument(skip(self, data))]
    pub fn process(&self, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        if data.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
            return Ok((data.to_vec(), data.to_vec()));
        }
        let img = image::load_from_memory(data)?;
        let compressed = self.compress(&img)?;
        let thumbnail = self.generate_thumbnail(&img)?;
        Ok((compressed, thumbnail))
    }

    #[tracing::instrument(skip(self, data))]
    pub fn edit_image(
        &self,
        data: &[u8],
        crop: Option<(u32, u32, u32, u32)>,
        rotate: Option<i32>,
        filters: &Option<FilterParams>,
        watermark: &Option<WatermarkParams>,
        format: Option<&str>,
    ) -> Result<Vec<u8>> {
        let mut img = image::load_from_memory(data)?;

        if let Some((x, y, width, height)) = crop {
            let cropped = image::imageops::crop(&mut img, x, y, width, height);
            let cropped_rgba: RgbaImage = cropped.to_image();
            img = DynamicImage::ImageRgba8(cropped_rgba);
        }

        if let Some(degrees) = rotate {
            img = match degrees {
                90 => img.rotate90(),
                180 => img.rotate180(),
                270 => img.rotate270(),
                _ => img,
            };
        }

        if let Some(f) = filters {
            img = Self::apply_filters(&mut img, f)?;
        }

        if let Some(wm) = watermark {
            img = Self::add_watermark(&mut img, wm)?;
        }

        match format {
            Some("png") => Self::convert_to_png(&img),
            _ => self.compress(&img),
        }
    }

    fn compress(&self, img: &DynamicImage) -> Result<Vec<u8>> {
        let resized = Self::resize_if_needed(img, self.max_width, self.max_height);
        let mut buf = Vec::new();
        let encoder = jpeg::JpegEncoder::new_with_quality(&mut buf, self.jpeg_quality);
        resized.write_with_encoder(encoder)?;
        Ok(buf)
    }

    pub fn generate_thumbnail(&self, img: &DynamicImage) -> Result<Vec<u8>> {
        let thumb = img.resize(
            self.thumbnail_size,
            self.thumbnail_size,
            imageops::FilterType::Lanczos3,
        );
        let mut buf = Vec::new();
        let encoder = jpeg::JpegEncoder::new_with_quality(&mut buf, 75);
        thumb.write_with_encoder(encoder)?;
        Ok(buf)
    }

    fn convert_to_png(img: &DynamicImage) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let encoder = png::PngEncoder::new(&mut buf);
        img.write_with_encoder(encoder)?;
        Ok(buf)
    }

    fn add_watermark(img: &mut DynamicImage, wm: &WatermarkParams) -> Result<DynamicImage> {
        let width = img.width();
        let height = img.height();

        if let Some(text) = &wm.text {
            if text.is_empty() {
                return Ok(DynamicImage::clone(img));
            }

            let position = wm.position.as_deref().unwrap_or("bottom-right");
            let opacity = wm.opacity.unwrap_or(128).clamp(0, 255) as u8;

            let char_width = 12;
            let char_height = 20;
            let padding = 10;

            let text_width = text.len() as u32 * char_width;
            let text_height = char_height;

            let x = match position {
                "top-left" | "bottom-left" => padding,
                "top-right" | "bottom-right" => width.saturating_sub(text_width + padding),
                _ => padding,
            };
            let y = match position {
                "top-left" | "top-right" => padding,
                "bottom-left" | "bottom-right" => height.saturating_sub(text_height + padding),
                _ => padding,
            };

            if x >= width || y >= height {
                return Ok(DynamicImage::clone(img));
            }

            let x_end = std::cmp::min(x + text_width, width);
            let y_end = std::cmp::min(y + text_height, height);

            let mut rgba = img.to_rgba8();
            let rgba_bytes = rgba.as_mut();

            for py in y..y_end {
                for px in x..x_end {
                    let pixel_idx = (py * width + px) as usize * 4;
                    let rel_x = px - x;
                    let rel_y = py - y;

                    if rel_y >= char_height {
                        continue;
                    }

                    let char_index = (rel_x / char_width) as usize;
                    if char_index >= text.len() {
                        continue;
                    }

                    let is_core = rel_x >= 2 && rel_x < char_width - 2 && rel_y >= 2 && rel_y < char_height - 2;
                    let alpha = if is_core {
                        opacity
                    } else if rel_x < 2 || rel_x >= char_width - 2 || rel_y < 2 || rel_y >= char_height - 2 {
                        (opacity as u16 * 3 / 4) as u8
                    } else {
                        opacity / 2
                    };

                    let src_a = alpha as f32 / 255.0;
                    let dst_a = rgba_bytes[pixel_idx + 3] as f32 / 255.0;
                    let out_a = src_a + dst_a * (1.0 - src_a);

                    if out_a > 0.0 {
                        rgba_bytes[pixel_idx] = (255.0 * src_a + rgba_bytes[pixel_idx] as f32 * dst_a * (1.0 - src_a)) as u8;
                        rgba_bytes[pixel_idx + 1] = (255.0 * src_a + rgba_bytes[pixel_idx + 1] as f32 * dst_a * (1.0 - src_a)) as u8;
                        rgba_bytes[pixel_idx + 2] = (255.0 * src_a + rgba_bytes[pixel_idx + 2] as f32 * dst_a * (1.0 - src_a)) as u8;
                        rgba_bytes[pixel_idx + 3] = (out_a * 255.0) as u8;
                    }
                }
            }

            *img = DynamicImage::ImageRgba8(rgba);
        }

        Ok(DynamicImage::clone(img))
    }

    fn apply_filters(img: &mut DynamicImage, f: &FilterParams) -> Result<DynamicImage> {
        use image::GenericImageView;

        let rgba = img.to_rgba8();
        let (width, height) = img.dimensions();
        let mut rgba_bytes: Vec<u8> = rgba.as_raw().to_vec();

        for y in 0..height {
            for x in 0..width {
                let pixel_idx = (y * width + x) as usize * 4;
                let mut pixel = [
                    rgba_bytes[pixel_idx],
                    rgba_bytes[pixel_idx + 1],
                    rgba_bytes[pixel_idx + 2],
                    rgba_bytes[pixel_idx + 3],
                ];

                if let Some(b) = f.brightness {
                    let adjustment = b - 128;
                    pixel[0] = (pixel[0] as i32 + adjustment).clamp(0, 255) as u8;
                    pixel[1] = (pixel[1] as i32 + adjustment).clamp(0, 255) as u8;
                    pixel[2] = (pixel[2] as i32 + adjustment).clamp(0, 255) as u8;
                }

                if let Some(c) = f.contrast {
                    let factor = (c as f32 / 128.0).clamp(0.5, 3.0);
                    pixel[0] = (((pixel[0] as f32 - 128.0) * factor) + 128.0).clamp(0.0, 255.0) as u8;
                    pixel[1] = (((pixel[1] as f32 - 128.0) * factor) + 128.0).clamp(0.0, 255.0) as u8;
                    pixel[2] = (((pixel[2] as f32 - 128.0) * factor) + 128.0).clamp(0.0, 255.0) as u8;
                }

                if let Some(s) = f.saturation {
                    let factor = (s as f32 / 128.0).clamp(0.0, 2.0);
                    let gray = (pixel[0] as f32 * 0.299 + pixel[1] as f32 * 0.587 + pixel[2] as f32 * 0.114) as u8;
                    pixel[0] = ((pixel[0] as f32 - gray as f32) * factor + gray as f32).clamp(0.0, 255.0) as u8;
                    pixel[1] = ((pixel[1] as f32 - gray as f32) * factor + gray as f32).clamp(0.0, 255.0) as u8;
                    pixel[2] = ((pixel[2] as f32 - gray as f32) * factor + gray as f32).clamp(0.0, 255.0) as u8;
                }

                if let Some(true) = f.grayscale {
                    let gray = (pixel[0] as f32 * 0.299 + pixel[1] as f32 * 0.587 + pixel[2] as f32 * 0.114) as u8;
                    pixel[0] = gray;
                    pixel[1] = gray;
                    pixel[2] = gray;
                }

                if let Some(true) = f.sepia {
                    let gray = (pixel[0] as f32 * 0.299 + pixel[1] as f32 * 0.587 + pixel[2] as f32 * 0.114) as u8;
                    pixel[0] = (gray as f32 * 1.07).min(255.0) as u8;
                    pixel[1] = (gray as f32 * 0.74).min(255.0) as u8;
                    pixel[2] = (gray as f32 * 0.43).min(255.0) as u8;
                }

                rgba_bytes[pixel_idx] = pixel[0];
                rgba_bytes[pixel_idx + 1] = pixel[1];
                rgba_bytes[pixel_idx + 2] = pixel[2];
                rgba_bytes[pixel_idx + 3] = pixel[3];
            }
        }

        let rgba_image = RgbaImage::from_raw(width, height, rgba_bytes)
            .ok_or_else(|| anyhow::anyhow!("Failed to create RGBA image: data size mismatch"))?;

        Ok(DynamicImage::ImageRgba8(rgba_image))
    }

    fn resize_if_needed(img: &DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
        use image::GenericImageView;

        let (width, height) = img.dimensions();

        if width <= max_width && height <= max_height {
            return img.clone();
        }

        let ratio = (max_width as f32 / width as f32).min(max_height as f32 / height as f32);
        let new_width = (width as f32 * ratio) as u32;
        let new_height = (height as f32 * ratio) as u32;

        img.resize(new_width, new_height, imageops::FilterType::Lanczos3)
    }

    pub fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    pub fn get_extension(filename: &str) -> String {
        if let Some(dot_pos) = filename.rfind('.')
            && dot_pos < filename.len() - 1 {
                filename[dot_pos + 1..].to_lowercase()
            } else {
                "jpg".to_string()
            }
    }

    pub fn is_image(content_type: Option<&str>) -> bool {
        content_type.is_some_and(|ct| ct.starts_with("image/"))
    }

    pub fn validate_image_bytes(data: &[u8]) -> Result<()> {
        const JPEG_SIGNATURE: &[u8] = &[0xFF, 0xD8, 0xFF];
        const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47];
        const GIF_SIGNATURE: &[u8] = &[0x47, 0x49, 0x46, 0x38];
        const WEBP_SIGNATURE: &[u8] = &[0x52, 0x49, 0x46, 0x46];
        const ICO_SIGNATURE: &[u8] = &[0x00, 0x00, 0x01, 0x00];

        let valid = data.len() >= 4 && (
            data.starts_with(JPEG_SIGNATURE) ||
            data.starts_with(PNG_SIGNATURE) ||
            data.starts_with(GIF_SIGNATURE) ||
            data.starts_with(WEBP_SIGNATURE) ||
            data.starts_with(ICO_SIGNATURE)
        );

        if !valid {
            return Err(anyhow::anyhow!("Invalid image file signature"));
        }

        if !data.starts_with(ICO_SIGNATURE) {
            let _ = image::load_from_memory(data)
                .map_err(|e| anyhow::anyhow!("Image file is corrupted or invalid: {}", e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, RgbImage, Rgb, codecs::jpeg::JpegEncoder};

    #[test]
    fn test_calculate_hash() {
        let data = b"test data";
        let hash = ImageProcessor::calculate_hash(data);
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(ImageProcessor::get_extension("photo.jpg"), "jpg");
        assert_eq!(ImageProcessor::get_extension("image.JPEG"), "jpeg");
        assert_eq!(ImageProcessor::get_extension("noext"), "jpg");
    }

    #[test]
    fn test_is_image() {
        assert!(ImageProcessor::is_image(Some("image/jpeg")));
        assert!(ImageProcessor::is_image(Some("image/png")));
        assert!(!ImageProcessor::is_image(Some("text/plain")));
        assert!(!ImageProcessor::is_image(None));
    }

    #[test]
    fn test_validate_image_bytes() {
        // 使用 image crate 生成一个有效的 1x1 JPEG 图像
        let img: RgbImage = ImageBuffer::from_pixel(1, 1, Rgb([255, 0, 0]));
        let mut buf = Vec::new();
        let encoder = JpegEncoder::new_with_quality(&mut buf, 85);
        img.write_with_encoder(encoder).unwrap();
        assert!(ImageProcessor::validate_image_bytes(&buf).is_ok());

        // 无效数据
        assert!(ImageProcessor::validate_image_bytes(&[0, 1, 2, 3]).is_err());
    }
}
