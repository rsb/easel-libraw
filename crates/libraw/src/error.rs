use std::fmt;
use std::panic::Location;

// ---------------------------------------------------------------------------
// Kind — the four failure categories that downstream code can match on
// ---------------------------------------------------------------------------

/// Classifies every error into one of four categories so callers can decide
/// how to respond without parsing message strings. The variants map directly
/// to the failure modes a RAW-processing pipeline encounters:
///
/// - `Io` — the file could not be read at all (missing, permissions, device).
/// - `Unsupported` — the file was read but its format is not handled.
/// - `Corrupt` — the file claims to be a supported format but its data is
///   malformed, truncated, or internally inconsistent.
/// - `Resource` — the operation failed due to memory or system limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
  Io,
  Unsupported,
  Corrupt,
  Resource,
}

impl Kind {
  /// Returns a short, human-readable label for this category. Used as the
  /// trailing phrase in `Display` output (e.g., "open failed: i/o failure").
  pub fn default_message(&self) -> &'static str {
    match self {
      Kind::Io => "i/o failure",
      Kind::Unsupported => "unsupported format",
      Kind::Corrupt => "corrupt data",
      Kind::Resource => "resource exhaustion",
    }
  }
}

impl fmt::Display for Kind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.default_message())
  }
}

// ---------------------------------------------------------------------------
// Error — a categorized, located, chainable error value
// ---------------------------------------------------------------------------

/// The crate's single error type. Every fallible operation returns this.
///
/// Design choices:
/// - `kind` enables programmatic matching without downcasting.
/// - `message` carries human-readable detail (may be empty).
/// - `source` optionally wraps an underlying std::error::Error for chaining.
/// - `location` captures the call site via `#[track_caller]`, so logs show
///   where the error was constructed without needing a full backtrace.
///
/// The type is `Send + Sync` so it can cross thread boundaries (e.g., when
/// decoding is dispatched to a thread pool).
#[derive(Debug)]
pub struct Error {
  kind: Kind,
  message: String,
  source: Option<Box<dyn std::error::Error + Send + Sync>>,
  location: &'static Location<'static>,
}

impl Error {
  // -------------------------------------------------------------------------
  // Constructors — one per Kind, all #[track_caller] so Location is correct
  // -------------------------------------------------------------------------

  /// Creates an I/O error. Use when the file system or device layer fails
  /// (file not found, permission denied, read interrupted).
  #[track_caller]
  pub fn io(msg: impl Into<String>) -> Self {
    Self {
      kind: Kind::Io,
      message: msg.into(),
      source: None,
      location: Location::caller(),
    }
  }

  /// Creates an unsupported-format error. Use when the file is readable but
  /// its codec, pixel layout, or container is not handled by this crate.
  #[track_caller]
  pub fn unsupported(msg: impl Into<String>) -> Self {
    Self {
      kind: Kind::Unsupported,
      message: msg.into(),
      source: None,
      location: Location::caller(),
    }
  }

  /// Creates a corrupt-data error. Use when the file header or payload
  /// violates format invariants (bad magic, truncated segments, impossible
  /// dimensions).
  #[track_caller]
  pub fn corrupt(msg: impl Into<String>) -> Self {
    Self {
      kind: Kind::Corrupt,
      message: msg.into(),
      source: None,
      location: Location::caller(),
    }
  }

  /// Creates a resource-exhaustion error. Use when memory allocation or
  /// internal pool limits are exceeded.
  #[track_caller]
  pub fn resource(msg: impl Into<String>) -> Self {
    Self {
      kind: Kind::Resource,
      message: msg.into(),
      source: None,
      location: Location::caller(),
    }
  }

  // -------------------------------------------------------------------------
  // Builder methods
  // -------------------------------------------------------------------------

  /// Attaches a lower-level error as the cause. Appears in Display output
  /// as "caused by: …" and is returned by `std::error::Error::source()`.
  pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
    self.source = Some(Box::new(source));
    self
  }

  /// Prepends a contextual callsite description to the message, producing
  /// "context: original message". If the original message is empty, the
  /// context becomes the entire message. Kind and location are preserved
  /// from the original error — context describes *where* the error surfaced,
  /// not *what* went wrong.
  pub fn context(self, ctx: impl Into<String>) -> Self {
    let message = if self.message.is_empty() {
      ctx.into()
    } else {
      format!("{}: {}", ctx.into(), self.message)
    };

    Self {
      kind: self.kind,
      message,
      source: self.source,
      location: self.location,
    }
  }

  // -------------------------------------------------------------------------
  // Accessors
  // -------------------------------------------------------------------------

  /// Returns the error category.
  pub fn kind(&self) -> Kind {
    self.kind
  }

  /// Convenience predicate for matching a specific category.
  pub fn is_kind(&self, kind: Kind) -> bool {
    self.kind == kind
  }

  /// Returns the human-readable detail string (may be empty).
  pub fn message(&self) -> &str {
    &self.message
  }

  /// Returns the source-code location where this error was constructed.
  pub fn location(&self) -> &'static Location<'static> {
    self.location
  }
}

// ---------------------------------------------------------------------------
// Display — "message: kind\n  caused by: source"
// ---------------------------------------------------------------------------

impl fmt::Display for Error {
  /// Format: `"{message}: {kind}"` when message is non-empty, otherwise just
  /// `"{kind}"`. If a source is attached, appends `"\n  caused by: {source}"`.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.message.is_empty() {
      write!(f, "{}", self.kind.default_message())?;
    } else {
      write!(f, "{}: {}", self.message, self.kind.default_message())?;
    }
    if let Some(source) = &self.source {
      write!(f, "\n  caused by: {source}")?;
    }
    Ok(())
  }
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    self
      .source
      .as_ref()
      .map(|s| s.as_ref() as &(dyn std::error::Error + 'static))
  }
}

// ---------------------------------------------------------------------------
// ResultExt — ergonomic .context() on Result<T, Error>
// ---------------------------------------------------------------------------

/// Extension trait that adds `.context("msg")` to `Result<T, Error>`, mirroring
/// the pattern from anyhow/eyre but staying within this crate's error type.
pub trait ResultExt<T> {
  fn context(self, ctx: impl Into<String>) -> Result<T, Error>;
}

impl<T> ResultExt<T> for Result<T, Error> {
  fn context(self, ctx: impl Into<String>) -> Result<T, Error> {
    self.map_err(|e| e.context(ctx))
  }
}
