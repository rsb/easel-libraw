use rsb_libraw::error::{Error, Kind, ResultExt};

#[test]
fn error_io_has_correct_kind() {
  let err = Error::io("file not found");
  assert_eq!(err.kind(), Kind::Io);
  assert_eq!(err.message(), "file not found");
}

#[test]
fn error_unsupported_has_correct_kind() {
  let err = Error::unsupported("HEIF thumbnail");
  assert_eq!(err.kind(), Kind::Unsupported);
  assert_eq!(err.message(), "HEIF thumbnail");
}

#[test]
fn error_corrupt_has_correct_kind() {
  let err = Error::corrupt("truncated file");
  assert_eq!(err.kind(), Kind::Corrupt);
  assert_eq!(err.message(), "truncated file");
}

#[test]
fn error_resource_has_correct_kind() {
  let err = Error::resource("out of memory");
  assert_eq!(err.kind(), Kind::Resource);
  assert_eq!(err.message(), "out of memory");
}

#[test]
fn error_context_prepends_callsite() {
  let err = Error::corrupt("bad data").context("Decoder::process failed");
  assert_eq!(err.message(), "Decoder::process failed: bad data");
  assert_eq!(err.kind(), Kind::Corrupt);
}

#[test]
fn error_context_on_empty_message() {
  let err = Error::io("").context("open failed");
  assert_eq!(err.message(), "open failed");
}

#[test]
fn error_is_kind_returns_true_for_matching() {
  let err = Error::unsupported("test");
  assert!(err.is_kind(Kind::Unsupported));
  assert!(!err.is_kind(Kind::Io));
  assert!(!err.is_kind(Kind::Corrupt));
  assert!(!err.is_kind(Kind::Resource));
}

#[test]
fn error_display_includes_kind_message() {
  let err = Error::corrupt("bad header");
  let display = format!("{err}");
  assert!(display.contains("bad header"));
  assert!(display.contains("corrupt data"));
}

#[test]
fn error_display_with_source() {
  let source = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
  let err = Error::io("open failed").with_source(source);
  let display = format!("{err}");
  assert!(display.contains("caused by"));
}

#[test]
fn error_location_captured() {
  let err = Error::io("test");
  let loc = err.location();
  assert!(loc.file().contains("error.rs"));
}

#[test]
fn result_ext_context_on_err() {
  let result: Result<(), Error> = Err(Error::corrupt("inner"));
  let result = result.context("outer");
  let err = result.unwrap_err();
  assert_eq!(err.message(), "outer: inner");
}

#[test]
fn result_ext_context_on_ok() {
  let result: Result<u32, Error> = Ok(42);
  let result = result.context("should not appear");
  assert_eq!(result.unwrap(), 42);
}

#[test]
fn error_is_send_sync() {
  fn assert_send_sync<T: Send + Sync>() {}
  assert_send_sync::<Error>();
}

#[test]
fn kind_display() {
  assert_eq!(format!("{}", Kind::Io), "i/o failure");
  assert_eq!(format!("{}", Kind::Unsupported), "unsupported format");
  assert_eq!(format!("{}", Kind::Corrupt), "corrupt data");
  assert_eq!(format!("{}", Kind::Resource), "resource exhaustion");
}

#[test]
fn error_display_empty_message_shows_kind_only() {
  let err = Error::io("");
  let display = format!("{err}");
  assert_eq!(display, "i/o failure");
}

#[test]
fn error_source_returns_some_when_set() {
  use std::error::Error as StdError;
  let source = std::io::Error::other("inner");
  let err = Error::io("outer").with_source(source);
  assert!(err.source().is_some());
  assert!(err.source().unwrap().to_string().contains("inner"));
}

#[test]
fn error_source_returns_none_when_unset() {
  use std::error::Error as StdError;
  let err = Error::io("no source");
  assert!(err.source().is_none());
}
