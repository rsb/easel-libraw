use crate::error::{self as fail};

// ---------------------------------------------------------------------------
// Pixel — packed 0x00RRGGBB in a single u32
// ---------------------------------------------------------------------------

/// A single RGB pixel packed as `0x00RRGGBB`. repr(transparent) guarantees
/// layout-compatibility with u32 for zero-copy handoff to framebuffers and
/// GPU texture APIs.
///
/// Channel values are in the sRGB color space — the standard nonlinear gamma
/// encoding used by monitors, cameras, and the web. The `srgb` naming is a
/// contract: consumers must linearize (gamma-decode) before blending,
/// filtering, or any operation that assumes proportional light values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Pixel(u32);

// Compile-time proof that repr(transparent) produced the expected layout.
// Guards against future field additions or wrapper changes silently breaking
// the assumption that &[Pixel] can be reinterpreted as &[u32].
const _: () = assert!(size_of::<Pixel>() == size_of::<u32>());

// Methods take `self` by value (not &self) because Pixel is Copy and fits in
// a register — passing by value avoids an indirection and lets the compiler
// keep everything in registers without a pointer dereference.
impl Pixel {
  /// Packs three 8-bit gamma-encoded sRGB channels into `0x00RRGGBB`: red in
  /// bits 16–23, green in bits 8–15, blue in bits 0–7. Callers must supply
  /// values already in the sRGB transfer function — not linear light.
  pub fn from_srgb(r: u8, g: u8, b: u8) -> Self {
    Self(((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
  }

  /// Extracts the red channel (bits 16–23).
  pub fn r(self) -> u8 {
    ((self.0 >> 16) & 0xFF) as u8
  }

  /// Extracts the green channel (bits 8–15).
  pub fn g(self) -> u8 {
    ((self.0 >> 8) & 0xFF) as u8
  }

  /// Extracts the blue channel (bits 0–7).
  pub fn b(self) -> u8 {
    (self.0 & 0xFF) as u8
  }

  /// Returns the raw u32 for direct framebuffer writes or bitwise comparison.
  pub fn packed(self) -> u32 {
    self.0
  }

  /// Wraps a raw u32 as a Pixel without validation. Caller is responsible
  /// for ensuring the value follows the `0x00RRGGBB` convention.
  pub fn from_packed(v: u32) -> Self {
    Self(v)
  }
}

// ---------------------------------------------------------------------------
// ImageBuffer — width × height grid of packed Pixel values
// ---------------------------------------------------------------------------

/// The crate's output type. Every `RawDecode` method returns an ImageBuffer.
///
/// Pixels are stored in row-major order: the first `width` entries are row 0
/// left-to-right, then row 1, and so on. To address column x of row y:
/// `pixels[y * width + x]`. Once constructed, the invariant
/// `pixels.len() == width * height` always holds, making that indexing safe
/// without per-access bounds checks.
///
/// No alpha channel — RAW photos have no transparency. The top byte of each
/// u32 is zero for buffers produced by `from_rgb8` and `from_rgb16` (the
/// decode paths). `from_packed` does not enforce this — callers using it
/// directly are responsible for the `0x00RRGGBB` convention. The layout is
/// compatible with `0xAARRGGBB` if alpha is needed later.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageBuffer {
  width: u32,
  height: u32,
  pixels: Vec<Pixel>,
}

impl ImageBuffer {
  /// Validates that `pixels.len() == width * height` (with overflow-checked
  /// multiplication for 32-bit targets) and constructs the buffer. This is
  /// the sole enforcement point for the length invariant.
  pub fn new(width: u32, height: u32, pixels: Vec<Pixel>) -> Result<Self, fail::Error> {
    let expected = (width as usize)
      .checked_mul(height as usize)
      .ok_or_else(|| fail::Error::corrupt("width * height overflows usize"))?;
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

  /// Returns the pixel at the given coordinate, or `None` if either
  /// coordinate is out of bounds. Coordinates are zero-indexed; valid
  /// ranges are `0..width` for `x` and `0..height` for `y`.
  pub fn get(&self, x: u32, y: u32) -> Option<Pixel> {
    if x >= self.width || y >= self.height {
      return None;
    }
    // Cast to usize before arithmetic to avoid u32 overflow on very large
    // images, even though the constructor guarantees width * height fits.
    Some(self.pixels[(y as usize) * (self.width as usize) + (x as usize)])
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

  /// Converts a flat `[R, G, B, R, G, B, …]` byte buffer (8-bit per channel,
  /// as produced by JPEG thumbnail decoding) into packed Pixel values.
  /// Expects exactly `width * height * 3` bytes — one byte per channel, three
  /// channels per pixel, for every pixel in the image.
  pub fn from_rgb8(data: &[u8], width: u32, height: u32) -> Result<Self, fail::Error> {
    let expected = (width as usize)
      .checked_mul(height as usize)
      .and_then(|n| n.checked_mul(3))
      .ok_or_else(|| fail::Error::corrupt("RGB8 dimensions overflow usize"))?;
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

    // chunks_exact(3) yields non-overlapping 3-byte windows. The "exact"
    // variant would silently drop a remainder shorter than 3 bytes, but the
    // length check above guarantees the slice divides evenly — no leftovers.
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

  /// Converts a flat 16-bit-per-channel RGB buffer (6 bytes per pixel) into
  /// 8-bit packed pixels. Common source: RAW files from high-end cameras and
  /// HDR workflows where each channel has 16 bits of dynamic range.
  pub fn from_rgb16(data: &[u8], width: u32, height: u32) -> Result<Self, fail::Error> {
    let expected = (width as usize)
      .checked_mul(height as usize)
      .and_then(|n| n.checked_mul(6))
      .ok_or_else(|| fail::Error::corrupt("RGB16 dimensions overflow usize"))?;
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

    // Each 6-byte chunk is three u16 channels in native byte order (R, G, B).
    // The byte arrangement within each u16 depends on the host: [lo, hi] on
    // little-endian, [hi, lo] on big-endian.
    //
    // from_ne_bytes (native endianness) is correct here because the source is
    // LibRaw's in-memory malloc'd buffer on the same host — not a serialized
    // file format with a fixed byte order. If this function were consuming
    // data from a file (e.g., 16-bit PNG is big-endian), from_be_bytes would
    // be required instead.
    //
    // >> 8 is bit-depth reduction: keeps only the top byte of each 16-bit
    // value (equivalent to dividing by 256 and flooring). A 16-bit 0xABCD
    // becomes 8-bit 0xAB. Slightly biased vs. proper rounding, but this is a
    // display path — the sub-LSB error is invisible on 8/10-bit monitors.
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
