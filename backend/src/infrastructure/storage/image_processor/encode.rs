use super::ImageProcessor;
use anyhow::Result;
use image::{
    DynamicImage, ImageFormat,
    codecs::{jpeg, png},
    imageops,
};

impl ImageProcessor {
    pub(super) fn compress(&self, img: &DynamicImage) -> Result<Vec<u8>> {
        let resized = Self::resize_if_needed(img, self.max_width, self.max_height);
        Self::encode_jpeg(&resized, self.jpeg_quality)
    }

    pub(super) fn compress_with_extension(&self, img: &DynamicImage, ext: &str) -> Result<Vec<u8>> {
        let resized = Self::resize_if_needed(img, self.max_width, self.max_height);
        let ext = ext.trim().trim_start_matches('.').to_ascii_lowercase();

        match ext.as_str() {
            "jpg" | "jpeg" => Self::encode_jpeg(&resized, self.jpeg_quality),
            "png" => Self::convert_to_png(&resized),
            "webp" => {
                let mut cursor = std::io::Cursor::new(Vec::new());
                resized.write_to(&mut cursor, ImageFormat::WebP)?;
                Ok(cursor.into_inner())
            }
            "gif" => {
                let mut cursor = std::io::Cursor::new(Vec::new());
                resized.write_to(&mut cursor, ImageFormat::Gif)?;
                Ok(cursor.into_inner())
            }
            _ => self.compress(&resized),
        }
    }

    pub(super) fn convert_to_png(img: &DynamicImage) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let encoder = png::PngEncoder::new(&mut buf);
        img.write_with_encoder(encoder)?;
        Ok(buf)
    }

    pub(super) fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let encoder = jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
        img.write_with_encoder(encoder)?;
        Ok(buf)
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
}
