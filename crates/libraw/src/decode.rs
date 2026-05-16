use std::path::Path;

use crate::error::Error;
use crate::image::ImageBuffer;

pub trait RawDecode: Send + Sync {
  fn decode(&self, path: &Path) -> Result<ImageBuffer, Error>;

  fn decode_thumbnail(&self, path: &Path) -> Result<ImageBuffer, Error> {
    let _ = path;
    Err(Error::unsupported("thumbnail extraction not supported"))
  }

  fn decode_preview(&self, path: &Path) -> Result<ImageBuffer, Error> {
    let _ = path;
    Err(Error::unsupported("preview decode not supported"))
  }
}
