//! 图片处理器
//!
//! 提供图片压缩与缩略图生成功能

mod encode;
mod inspect;
#[cfg(test)]
mod tests;

use anyhow::Result;

#[derive(Clone)]
pub struct ImageProcessor {
    pub max_width: u32,
    pub max_height: u32,
    pub jpeg_quality: u8,
}

impl ImageProcessor {
    pub fn new(max_width: u32, max_height: u32, jpeg_quality: u8) -> Self {
        Self {
            max_width,
            max_height,
            jpeg_quality,
        }
    }

    #[tracing::instrument(skip(self, path))]
    pub fn process_from_file(&self, path: &std::path::Path, ext: &str) -> Result<Vec<u8>> {
        let data = std::fs::read(path)?;
        let img = image::load_from_memory(&data)?;
        self.compress_with_extension(&img, ext)
    }

    #[tracing::instrument(skip(self, data))]
    pub fn generate_thumbnail(&self, data: &[u8], thumbnail_size: u32) -> Result<Vec<u8>> {
        let img = image::load_from_memory(data)?;
        self.generate_thumbnail_from_image(&img, thumbnail_size)
    }

    #[tracing::instrument(skip(self, data))]
    #[cfg(test)]
    pub fn process(&self, data: &[u8], ext: &str) -> Result<Vec<u8>> {
        if data.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
            return Ok(data.to_vec());
        }
        let img = image::load_from_memory(data)?;
        self.compress_with_extension(&img, ext)
    }
}
