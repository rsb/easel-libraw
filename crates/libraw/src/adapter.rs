use std::ffi::{CStr, CString};
use std::path::Path;

use easel_libraw_ffi as ffi;

use crate::decode::RawDecode;
use crate::error::{self as fail, ResultExt};
use crate::image::ImageBuffer;

// ---------------------------------------------------------------------------
// LibRawProcessor — RAII wrapper
// ---------------------------------------------------------------------------

struct LibRawProcessor {
  ptr: *mut ffi::libraw_data_t,
}

impl LibRawProcessor {
  fn new() -> Result<Self, fail::Error> {
    let ptr = unsafe { ffi::libraw_init(0) };
    if ptr.is_null() {
      return Err(fail::Error::resource("libraw_init returned null"));
    }
    Ok(Self { ptr })
  }

  fn open_file(&mut self, c_path: &CString) -> Result<(), fail::Error> {
    let rc = unsafe { ffi::libraw_open_file(self.ptr, c_path.as_ptr()) };
    if rc != 0 {
      return Err(fail::Error::corrupt(libraw_error(rc, "libraw_open_file")));
    }
    Ok(())
  }

  fn unpack(&mut self) -> Result<(), fail::Error> {
    let rc = unsafe { ffi::libraw_unpack(self.ptr) };
    if rc != 0 {
      return Err(fail::Error::corrupt(libraw_error(rc, "libraw_unpack")));
    }
    Ok(())
  }

  fn unpack_thumb(&mut self) -> Result<(), fail::Error> {
    let rc = unsafe { ffi::libraw_unpack_thumb(self.ptr) };
    if rc != 0 {
      return Err(fail::Error::corrupt(libraw_error(rc, "libraw_unpack_thumb")));
    }
    Ok(())
  }

  fn configure_output_srgb16(&mut self) {
    unsafe {
      (*self.ptr).params.use_camera_wb = 1;
      (*self.ptr).params.output_color = 1; // sRGB
      (*self.ptr).params.output_bps = 16;
    }
  }

  fn configure_half_size(&mut self) {
    unsafe {
      (*self.ptr).params.half_size = 1;
    }
  }

  fn dcraw_process(&mut self) -> Result<(), fail::Error> {
    let rc = unsafe { ffi::libraw_dcraw_process(self.ptr) };
    if rc != 0 {
      return Err(fail::Error::corrupt(libraw_error(rc, "libraw_dcraw_process")));
    }
    Ok(())
  }

  fn dcraw_make_mem_thumb(&mut self) -> Result<ImageBuffer, fail::Error> {
    let mut errcode: i32 = 0;

    let thumb_ptr = unsafe { ffi::libraw_dcraw_make_mem_thumb(self.ptr, &mut errcode) };

    if thumb_ptr.is_null() {
      return Err(fail::Error::corrupt(libraw_error(
        errcode,
        "libraw_dcraw_make_mem_thumb",
      )));
    }

    let result = unsafe {
      let thumb = &*thumb_ptr;
      let data_size = thumb.data_size as usize;
      let data_ptr = thumb.data.as_ptr();
      let owned = std::slice::from_raw_parts(data_ptr, data_size).to_vec();
      let format = thumb.type_;
      let width = thumb.width as u32;
      let height = thumb.height as u32;
      let bits = thumb.bits;
      let colors = thumb.colors;

      ffi::libraw_dcraw_clear_mem(thumb_ptr);
      (owned, format, width, height, bits, colors)
    };

    let (owned, format, width, height, bits, colors) = result;

    if format == ffi::LibRaw_image_formats_LIBRAW_IMAGE_JPEG {
      decode_jpeg_thumbnail(&owned)
    } else if format == ffi::LibRaw_image_formats_LIBRAW_IMAGE_BITMAP {
      if bits == 8 && colors == 3 {
        ImageBuffer::from_rgb8(&owned, width, height)
          .context("thumbnail bitmap RGB8 conversion failed")
      } else if bits == 16 && colors == 3 {
        ImageBuffer::from_rgb16(&owned, width, height)
          .context("thumbnail bitmap RGB16 conversion failed")
      } else {
        Err(fail::Error::unsupported(format!(
          "thumbnail bitmap format: {bits}-bit, {colors} channels"
        )))
      }
    } else {
      Err(fail::Error::unsupported(format!(
        "thumbnail format type {format}"
      )))
    }
  }

  fn dcraw_make_mem_image(&mut self) -> Result<ImageBuffer, fail::Error> {
    let mut errcode: i32 = 0;

    let image_ptr = unsafe { ffi::libraw_dcraw_make_mem_image(self.ptr, &mut errcode) };

    if image_ptr.is_null() {
      return Err(fail::Error::corrupt(libraw_error(
        errcode,
        "libraw_dcraw_make_mem_image",
      )));
    }

    let (owned_rgb, width, height) = unsafe {
      let image = &*image_ptr;

      if image.type_ != ffi::LibRaw_image_formats_LIBRAW_IMAGE_BITMAP {
        ffi::libraw_dcraw_clear_mem(image_ptr);
        return Err(fail::Error::unsupported("decoded image is not bitmap format"));
      }

      if image.bits != 16 {
        ffi::libraw_dcraw_clear_mem(image_ptr);
        return Err(fail::Error::unsupported(format!(
          "expected 16-bit output, got {}-bit",
          image.bits
        )));
      }

      if image.colors != 3 {
        ffi::libraw_dcraw_clear_mem(image_ptr);
        return Err(fail::Error::unsupported(format!(
          "expected 3-channel RGB, got {} channels",
          image.colors
        )));
      }

      let data_size = image.data_size as usize;
      let data_ptr = image.data.as_ptr();
      let raw_rgb = std::slice::from_raw_parts(data_ptr, data_size);
      let owned = raw_rgb.to_vec();

      let width = image.width as u32;
      let height = image.height as u32;

      ffi::libraw_dcraw_clear_mem(image_ptr);
      (owned, width, height)
    };

    ImageBuffer::from_rgb16(&owned_rgb, width, height)
      .context("LibRaw decode pixel conversion failed")
  }
}

impl Drop for LibRawProcessor {
  fn drop(&mut self) {
    unsafe { ffi::libraw_close(self.ptr) };
  }
}

// ---------------------------------------------------------------------------
// LibRawAdapter — RawDecode implementation
// ---------------------------------------------------------------------------

pub struct LibRawAdapter;

impl RawDecode for LibRawAdapter {
  fn decode(&self, path: &Path) -> Result<ImageBuffer, fail::Error> {
    let c_path = path_to_cstring(path)?;

    let mut processor = LibRawProcessor::new().context("LibRawAdapter::decode init failed")?;

    processor
      .open_file(&c_path)
      .context("LibRawAdapter::decode open failed")?;

    processor
      .unpack()
      .context("LibRawAdapter::decode unpack failed")?;

    processor.configure_output_srgb16();

    processor
      .dcraw_process()
      .context("LibRawAdapter::decode process failed")?;

    processor
      .dcraw_make_mem_image()
      .context("LibRawAdapter::decode make_mem_image failed")
  }

  fn decode_thumbnail(&self, path: &Path) -> Result<ImageBuffer, fail::Error> {
    let c_path = path_to_cstring(path)?;

    let mut processor =
      LibRawProcessor::new().context("LibRawAdapter::decode_thumbnail init failed")?;

    processor
      .open_file(&c_path)
      .context("LibRawAdapter::decode_thumbnail open failed")?;

    processor
      .unpack_thumb()
      .context("LibRawAdapter::decode_thumbnail unpack_thumb failed")?;

    processor
      .dcraw_make_mem_thumb()
      .context("LibRawAdapter::decode_thumbnail make_mem_thumb failed")
  }

  fn decode_preview(&self, path: &Path) -> Result<ImageBuffer, fail::Error> {
    let c_path = path_to_cstring(path)?;

    let mut processor =
      LibRawProcessor::new().context("LibRawAdapter::decode_preview init failed")?;

    processor
      .open_file(&c_path)
      .context("LibRawAdapter::decode_preview open failed")?;

    processor
      .unpack()
      .context("LibRawAdapter::decode_preview unpack failed")?;

    processor.configure_half_size();
    processor.configure_output_srgb16();

    processor
      .dcraw_process()
      .context("LibRawAdapter::decode_preview process failed")?;

    processor
      .dcraw_make_mem_image()
      .context("LibRawAdapter::decode_preview make_mem_image failed")
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn decode_jpeg_thumbnail(jpeg_data: &[u8]) -> Result<ImageBuffer, fail::Error> {
  use jpeg_decoder::{Decoder, PixelFormat};

  let mut decoder = Decoder::new(jpeg_data);
  let pixels = decoder
    .decode()
    .map_err(|e| fail::Error::corrupt(format!("JPEG thumbnail decode failed: {e}")))?;

  let info = decoder.info().ok_or_else(|| {
    fail::Error::corrupt("JPEG thumbnail decoded but produced no image info")
  })?;

  match info.pixel_format {
    PixelFormat::RGB24 => {
      ImageBuffer::from_rgb8(&pixels, info.width as u32, info.height as u32)
        .context("JPEG thumbnail pixel conversion failed")
    }
    PixelFormat::L8 => {
      let rgb: Vec<u8> = pixels.iter().flat_map(|&g| [g, g, g]).collect();
      ImageBuffer::from_rgb8(&rgb, info.width as u32, info.height as u32)
        .context("JPEG thumbnail grayscale conversion failed")
    }
    other => Err(fail::Error::unsupported(format!(
      "JPEG thumbnail pixel format: {other:?}"
    ))),
  }
}

fn libraw_error(rc: i32, func: &str) -> String {
  let ptr = unsafe { ffi::libraw_strerror(rc) };
  if ptr.is_null() {
    return format!("{func} failed: code {rc}");
  }
  let msg = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
  format!("{func} failed: {msg}")
}

#[cfg(unix)]
fn path_to_cstring(path: &Path) -> Result<CString, fail::Error> {
  use std::os::unix::ffi::OsStrExt;
  CString::new(path.as_os_str().as_bytes())
    .map_err(|_| fail::Error::io("path contains interior null byte"))
}

#[cfg(not(unix))]
fn path_to_cstring(path: &Path) -> Result<CString, fail::Error> {
  let s = path
    .to_str()
    .ok_or_else(|| fail::Error::io("path is not valid UTF-8"))?;
  CString::new(s).map_err(|_| fail::Error::io("path contains interior null byte"))
}
