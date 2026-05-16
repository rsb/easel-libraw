use crate::error::{self as fail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Pixel(u32);

const _: () = assert!(size_of::<Pixel>() == size_of::<u32>());

impl Pixel {
  pub fn from_srgb(r: u8, g: u8, b: u8) -> Self {
    Self(((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
  }

  pub fn r(self) -> u8 {
    ((self.0 >> 16) & 0xFF) as u8
  }

  pub fn g(self) -> u8 {
    ((self.0 >> 8) & 0xFF) as u8
  }

  pub fn b(self) -> u8 {
    (self.0 & 0xFF) as u8
  }

  pub fn packed(self) -> u32 {
    self.0
  }

  pub fn from_packed(v: u32) -> Self {
    Self(v)
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageBuffer {
  width: u32,
  height: u32,
  pixels: Vec<Pixel>,
}

impl ImageBuffer {
  pub fn new(width: u32, height: u32, pixels: Vec<Pixel>) -> Result<Self, fail::Error> {
    let expected = (width as usize) * (height as usize);
    if pixels.len() != expected {
      return Err(
        fail::Error::corrupt(format!(
          "pixel count {} does not match width * height ({}x{} = {})",
          pixels.len(),
          width,
          height,
          expected,
        ))
        .context("ImageBuffer::new failed"),
      );
    }
    Ok(Self {
      width,
      height,
      pixels,
    })
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  pub fn pixels(&self) -> &[Pixel] {
    &self.pixels
  }

  pub fn from_rgb8(data: &[u8], width: u32, height: u32) -> Result<Self, fail::Error> {
    let expected = (width as usize) * (height as usize) * 3;
    if data.len() != expected {
      return Err(
        fail::Error::corrupt(format!(
          "RGB8 data length {} does not match {}x{}x3 = {}",
          data.len(),
          width,
          height,
          expected,
        ))
        .context("ImageBuffer::from_rgb8 failed"),
      );
    }

    let pixels = data
      .chunks_exact(3)
      .map(|rgb| Pixel::from_srgb(rgb[0], rgb[1], rgb[2]))
      .collect();

    Ok(Self {
      width,
      height,
      pixels,
    })
  }

  pub fn from_rgb16(data: &[u8], width: u32, height: u32) -> Result<Self, fail::Error> {
    let expected = (width as usize) * (height as usize) * 6;
    if data.len() != expected {
      return Err(
        fail::Error::corrupt(format!(
          "RGB16 data length {} does not match {}x{}x6 = {}",
          data.len(),
          width,
          height,
          expected,
        ))
        .context("ImageBuffer::from_rgb16 failed"),
      );
    }

    let pixels = data
      .chunks_exact(6)
      .map(|rgb| {
        let r = u16::from_ne_bytes([rgb[0], rgb[1]]);
        let g = u16::from_ne_bytes([rgb[2], rgb[3]]);
        let b = u16::from_ne_bytes([rgb[4], rgb[5]]);
        Pixel::from_srgb((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8)
      })
      .collect();

    Ok(Self {
      width,
      height,
      pixels,
    })
  }
}
