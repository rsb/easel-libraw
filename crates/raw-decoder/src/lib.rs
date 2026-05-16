use std::path::Path;

use lab_fail as fail;
use lab_image::ImageBuffer;

pub trait RawDecode: Send + Sync {
  fn decode(&self, path: &Path) -> Result<ImageBuffer, fail::Error>;

  fn decode_thumbnail(&self, path: &Path) -> Result<ImageBuffer, fail::Error> {
    let _ = path;
    Err(fail::Error::unsupported("thumbnail extraction not supported"))
  }

  fn decode_preview(&self, path: &Path) -> Result<ImageBuffer, fail::Error> {
    let _ = path;
    Err(fail::Error::unsupported("preview decode not supported"))
  }
}
