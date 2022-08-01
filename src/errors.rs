use std::io;
use std::path;

use thiserror::Error;
use toml;

/// The Result type for badm.
pub type Result<T> = std::result::Result<T, Error>;

/// The Error type for badm.
#[derive(Error, Debug)]
pub enum Error {
    #[error("could not parse toml")]
    InvalidToml(#[from] toml::de::Error),

    #[error("meow")]
    StdIOError(#[from] io::Error),

    #[error("could not strip prefix")]
    StripPrefixError(#[from] path::StripPrefixError),

    #[error("bad input detected: {0}")]
    BadInput(&'static str),

    #[error("config not found")]
    ConfigNotFound,

    #[error("invalid dotfile destination directory: {0}")]
    InvalidDotfileDestinationDirectory(path::PathBuf),
}
