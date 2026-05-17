use std::io::Write;
use std::path::Path;

use easel_libraw::error::Kind;
use easel_libraw::{LibRawAdapter, RawDecode};

#[test]
fn decode_nonexistent_file_returns_io() {
  let adapter = LibRawAdapter;
  let result = adapter.decode(Path::new("/tmp/does_not_exist_9f8a7b.cr2"));
  assert!(result.is_err());
  assert_eq!(result.unwrap_err().kind(), Kind::Io);
}

#[test]
fn decode_thumbnail_nonexistent_file_returns_io() {
  let adapter = LibRawAdapter;
  let result = adapter.decode_thumbnail(Path::new("/tmp/does_not_exist_9f8a7b.cr2"));
  assert!(result.is_err());
  assert_eq!(result.unwrap_err().kind(), Kind::Io);
}

#[test]
fn decode_preview_nonexistent_file_returns_io() {
  let adapter = LibRawAdapter;
  let result = adapter.decode_preview(Path::new("/tmp/does_not_exist_9f8a7b.cr2"));
  assert!(result.is_err());
  assert_eq!(result.unwrap_err().kind(), Kind::Io);
}

#[test]
fn decode_non_raw_file_returns_corrupt() {
  let dir = std::env::temp_dir().join("easel_libraw_test_not_raw");
  std::fs::create_dir_all(&dir).unwrap();
  let path = dir.join("garbage.cr2");
  let mut f = std::fs::File::create(&path).unwrap();
  f.write_all(b"this is not a raw file at all").unwrap();
  drop(f);

  let adapter = LibRawAdapter;
  let result = adapter.decode(&path);

  assert!(result.is_err());
  let err = result.unwrap_err();
  assert_eq!(err.kind(), Kind::Corrupt);

  std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn decode_thumbnail_non_raw_file_returns_corrupt() {
  let dir = std::env::temp_dir().join("easel_libraw_test_thumb_not_raw");
  std::fs::create_dir_all(&dir).unwrap();
  let path = dir.join("garbage.nef");
  let mut f = std::fs::File::create(&path).unwrap();
  f.write_all(b"not a nef file").unwrap();
  drop(f);

  let adapter = LibRawAdapter;
  let result = adapter.decode_thumbnail(&path);

  assert!(result.is_err());
  let err = result.unwrap_err();
  assert_eq!(err.kind(), Kind::Corrupt);

  std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn decode_preview_non_raw_file_returns_corrupt() {
  let dir = std::env::temp_dir().join("easel_libraw_test_preview_not_raw");
  std::fs::create_dir_all(&dir).unwrap();
  let path = dir.join("garbage.arw");
  let mut f = std::fs::File::create(&path).unwrap();
  f.write_all(b"not an arw file").unwrap();
  drop(f);

  let adapter = LibRawAdapter;
  let result = adapter.decode_preview(&path);

  assert!(result.is_err());
  let err = result.unwrap_err();
  assert_eq!(err.kind(), Kind::Corrupt);

  std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn decode_path_with_interior_null_returns_io() {
  let adapter = LibRawAdapter;
  let result = adapter.decode(Path::new("/tmp/bad\0path.cr2"));

  assert!(result.is_err());
  let err = result.unwrap_err();
  assert_eq!(err.kind(), Kind::Io);
}

#[test]
fn decode_thumbnail_path_with_interior_null_returns_io() {
  let adapter = LibRawAdapter;
  let result = adapter.decode_thumbnail(Path::new("/tmp/bad\0path.cr2"));

  assert!(result.is_err());
  let err = result.unwrap_err();
  assert_eq!(err.kind(), Kind::Io);
}

#[test]
fn decode_preview_path_with_interior_null_returns_io() {
  let adapter = LibRawAdapter;
  let result = adapter.decode_preview(Path::new("/tmp/bad\0path.cr2"));

  assert!(result.is_err());
  let err = result.unwrap_err();
  assert_eq!(err.kind(), Kind::Io);
}

#[test]
fn error_display_includes_message() {
  let adapter = LibRawAdapter;
  let err = adapter
    .decode(Path::new("/tmp/does_not_exist_9f8a7b.cr2"))
    .unwrap_err();
  let display = format!("{err}");
  assert!(!display.is_empty());
}

#[test]
fn adapter_implements_send_and_sync() {
  fn assert_send_sync<T: Send + Sync>() {}
  assert_send_sync::<LibRawAdapter>();
}

struct DecodeOnly;

impl RawDecode for DecodeOnly {
  fn decode(&self, _path: &Path) -> Result<easel_libraw::ImageBuffer, easel_libraw::Error> {
    Err(easel_libraw::Error::unsupported("stub"))
  }
}

#[test]
fn default_decode_thumbnail_returns_unsupported() {
  let d = DecodeOnly;
  let result = d.decode_thumbnail(Path::new("/tmp/anything"));
  assert!(result.is_err());
  let err = result.unwrap_err();
  assert_eq!(err.kind(), Kind::Unsupported);
  assert!(err.message().contains("thumbnail"));
}

#[test]
fn default_decode_preview_returns_unsupported() {
  let d = DecodeOnly;
  let result = d.decode_preview(Path::new("/tmp/anything"));
  assert!(result.is_err());
  let err = result.unwrap_err();
  assert_eq!(err.kind(), Kind::Unsupported);
  assert!(err.message().contains("preview"));
}
