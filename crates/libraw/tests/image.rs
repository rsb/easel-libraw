use easel_libraw::error::Kind;
use easel_libraw::image::{ImageBuffer, Pixel};

#[test]
fn pixel_from_srgb_roundtrips() {
  let p = Pixel::from_srgb(0xAA, 0xBB, 0xCC);
  assert_eq!(p.r(), 0xAA);
  assert_eq!(p.g(), 0xBB);
  assert_eq!(p.b(), 0xCC);
  assert_eq!(p.packed(), 0x00AABBCC);
}

#[test]
fn pixel_from_packed_roundtrips() {
  let p = Pixel::from_packed(0x00112233);
  assert_eq!(p.r(), 0x11);
  assert_eq!(p.g(), 0x22);
  assert_eq!(p.b(), 0x33);
  assert_eq!(p.packed(), 0x00112233);
}

#[test]
fn pixel_black() {
  let p = Pixel::from_srgb(0, 0, 0);
  assert_eq!(p.packed(), 0x00000000);
}

#[test]
fn pixel_white() {
  let p = Pixel::from_srgb(255, 255, 255);
  assert_eq!(p.packed(), 0x00FFFFFF);
}

#[test]
fn image_buffer_construction() {
  let pixels = vec![Pixel::from_packed(0x00FF0000); 6];
  let buf = ImageBuffer::new(3, 2, pixels).unwrap();
  assert_eq!(buf.width(), 3);
  assert_eq!(buf.height(), 2);
  assert_eq!(buf.pixels().len(), 6);
}

#[test]
fn image_buffer_rejects_mismatched_pixel_count() {
  let result = ImageBuffer::new(4, 4, vec![Pixel::from_packed(0); 5]);
  assert!(result.is_err());
  assert_eq!(result.unwrap_err().kind(), Kind::Corrupt);
}

#[test]
fn image_buffer_single_pixel() {
  let pixels = vec![Pixel::from_srgb(128, 64, 32)];
  let buf = ImageBuffer::new(1, 1, pixels).unwrap();
  assert_eq!(buf.width(), 1);
  assert_eq!(buf.height(), 1);
  assert_eq!(buf.pixels()[0].r(), 128);
  assert_eq!(buf.pixels()[0].g(), 64);
  assert_eq!(buf.pixels()[0].b(), 32);
}

#[test]
fn image_buffer_from_rgb8() {
  let data: Vec<u8> = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 128, 128, 128];
  let buf = ImageBuffer::from_rgb8(&data, 2, 2).unwrap();
  assert_eq!(buf.width(), 2);
  assert_eq!(buf.height(), 2);
  assert_eq!(buf.pixels()[0], Pixel::from_srgb(255, 0, 0));
  assert_eq!(buf.pixels()[1], Pixel::from_srgb(0, 255, 0));
  assert_eq!(buf.pixels()[2], Pixel::from_srgb(0, 0, 255));
  assert_eq!(buf.pixels()[3], Pixel::from_srgb(128, 128, 128));
}

#[test]
fn image_buffer_from_rgb8_wrong_length_returns_corrupt() {
  let data: Vec<u8> = vec![255, 0, 0, 0, 255]; // 5 bytes, not 12
  let result = ImageBuffer::from_rgb8(&data, 2, 2);
  assert!(result.is_err());
  assert_eq!(result.unwrap_err().kind(), Kind::Corrupt);
}

#[test]
fn image_buffer_from_rgb16() {
  // 1x1 image, 16-bit RGB = 6 bytes
  // Native-endian u16 values: R=65535, G=32768, B=0
  let r = 65535u16.to_ne_bytes();
  let g = 32768u16.to_ne_bytes();
  let b = 0u16.to_ne_bytes();
  let data: Vec<u8> = [r, g, b].concat();

  let buf = ImageBuffer::from_rgb16(&data, 1, 1).unwrap();
  // 16-bit values are downshifted to 8-bit: val >> 8
  assert_eq!(buf.pixels()[0].r(), 255);
  assert_eq!(buf.pixels()[0].g(), 128);
  assert_eq!(buf.pixels()[0].b(), 0);
}

#[test]
fn image_buffer_from_rgb16_wrong_length_returns_corrupt() {
  let data: Vec<u8> = vec![0; 10]; // 10 bytes, not 6
  let result = ImageBuffer::from_rgb16(&data, 1, 1);
  assert!(result.is_err());
  assert_eq!(result.unwrap_err().kind(), Kind::Corrupt);
}

#[test]
fn image_buffer_zero_dimensions() {
  let buf = ImageBuffer::new(0, 0, vec![]).unwrap();
  assert_eq!(buf.width(), 0);
  assert_eq!(buf.height(), 0);
  assert_eq!(buf.pixels().len(), 0);
}

#[test]
fn image_buffer_pixels_accessible_by_index() {
  let pixels = vec![
    Pixel::from_srgb(10, 20, 30),
    Pixel::from_srgb(40, 50, 60),
    Pixel::from_srgb(70, 80, 90),
    Pixel::from_srgb(100, 110, 120),
  ];
  let buf = ImageBuffer::new(2, 2, pixels).unwrap();

  // Row-major: pixel at (x=1, y=0) is index 1
  let p = buf.pixels()[1];
  assert_eq!(p.r(), 40);
  assert_eq!(p.g(), 50);
  assert_eq!(p.b(), 60);

  // Pixel at (x=0, y=1) is index 2
  let p = buf.pixels()[2];
  assert_eq!(p.r(), 70);
}

#[test]
fn new_overflow_dimensions() {
  let pixels = vec![Pixel::from_packed(0); 4];
  let result = ImageBuffer::new(u32::MAX, u32::MAX, pixels);
  assert!(result.is_err());
  assert_eq!(result.unwrap_err().kind(), Kind::Corrupt);
}

#[test]
fn from_rgb8_overflow_dimensions() {
  let data = vec![0u8; 12];
  let result = ImageBuffer::from_rgb8(&data, u32::MAX, u32::MAX);
  assert!(result.is_err());
  assert_eq!(result.unwrap_err().kind(), Kind::Corrupt);
}

#[test]
fn from_rgb16_overflow_dimensions() {
  let data = vec![0u8; 12];
  let result = ImageBuffer::from_rgb16(&data, u32::MAX, u32::MAX);
  assert!(result.is_err());
  assert_eq!(result.unwrap_err().kind(), Kind::Corrupt);
}
