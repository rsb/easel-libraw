use std::fmt;
use std::panic::Location;

// ---------------------------------------------------------------------------
// Kind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
  Io,
  Unsupported,
  Corrupt,
  Resource,
}

impl Kind {
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
// Error
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Error {
  kind: Kind,
  message: String,
  source: Option<Box<dyn std::error::Error>>,
  location: &'static Location<'static>,
}

impl Error {
  #[track_caller]
  pub fn io(msg: impl Into<String>) -> Self {
    Self {
      kind: Kind::Io,
      message: msg.into(),
      source: None,
      location: Location::caller(),
    }
  }

  #[track_caller]
  pub fn unsupported(msg: impl Into<String>) -> Self {
    Self {
      kind: Kind::Unsupported,
      message: msg.into(),
      source: None,
      location: Location::caller(),
    }
  }

  #[track_caller]
  pub fn corrupt(msg: impl Into<String>) -> Self {
    Self {
      kind: Kind::Corrupt,
      message: msg.into(),
      source: None,
      location: Location::caller(),
    }
  }

  #[track_caller]
  pub fn resource(msg: impl Into<String>) -> Self {
    Self {
      kind: Kind::Resource,
      message: msg.into(),
      source: None,
      location: Location::caller(),
    }
  }

  pub fn with_source(mut self, source: impl std::error::Error + 'static) -> Self {
    self.source = Some(Box::new(source));
    self
  }

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

  pub fn kind(&self) -> Kind {
    self.kind
  }

  pub fn is_kind(&self, kind: Kind) -> bool {
    self.kind == kind
  }

  pub fn message(&self) -> &str {
    &self.message
  }

  pub fn location(&self) -> &'static Location<'static> {
    self.location
  }
}

impl fmt::Display for Error {
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
// ResultExt
// ---------------------------------------------------------------------------

pub trait ResultExt<T> {
  fn context(self, ctx: impl Into<String>) -> Result<T, Error>;
}

impl<T> ResultExt<T> for Result<T, Error> {
  fn context(self, ctx: impl Into<String>) -> Result<T, Error> {
    self.map_err(|e| e.context(ctx))
  }
}
