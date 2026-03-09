use super::*;
use image::{ImageBuffer, Rgb, RgbImage, codecs::jpeg::JpegEncoder};

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
    let img: RgbImage = ImageBuffer::from_pixel(1, 1, Rgb([255, 0, 0]));
    let mut buf = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut buf, 85);
    img.write_with_encoder(encoder).unwrap();
    assert!(ImageProcessor::validate_image_bytes(&buf).is_ok());
    assert!(ImageProcessor::validate_image_bytes(&[0, 1, 2, 3]).is_err());
}
