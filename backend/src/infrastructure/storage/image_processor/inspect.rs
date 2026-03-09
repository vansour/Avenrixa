use super::ImageProcessor;
use anyhow::Result;

impl ImageProcessor {
    pub fn calculate_hash(data: &[u8]) -> String {
        blake3::hash(data).to_hex().to_string()
    }

    pub fn get_extension(filename: &str) -> String {
        if let Some(dot_pos) = filename.rfind('.')
            && dot_pos < filename.len() - 1
        {
            filename[dot_pos + 1..].to_lowercase()
        } else {
            "jpg".to_string()
        }
    }

    pub fn is_image(content_type: Option<&str>) -> bool {
        content_type.is_some_and(|content_type| content_type.starts_with("image/"))
    }

    pub fn validate_image_bytes(data: &[u8]) -> Result<()> {
        const JPEG_SIGNATURE: &[u8] = &[0xFF, 0xD8, 0xFF];
        const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47];
        const GIF_SIGNATURE: &[u8] = &[0x47, 0x49, 0x46, 0x38];
        const WEBP_SIGNATURE: &[u8] = &[0x52, 0x49, 0x46, 0x46];
        const ICO_SIGNATURE: &[u8] = &[0x00, 0x00, 0x01, 0x00];

        let valid = data.len() >= 4
            && (data.starts_with(JPEG_SIGNATURE)
                || data.starts_with(PNG_SIGNATURE)
                || data.starts_with(GIF_SIGNATURE)
                || data.starts_with(WEBP_SIGNATURE)
                || data.starts_with(ICO_SIGNATURE));

        if !valid {
            return Err(anyhow::anyhow!("Invalid image file signature"));
        }

        if !data.starts_with(ICO_SIGNATURE) {
            let _ = image::load_from_memory(data).map_err(|error| {
                anyhow::anyhow!("Image file is corrupted or invalid: {}", error)
            })?;
        }

        Ok(())
    }
}
