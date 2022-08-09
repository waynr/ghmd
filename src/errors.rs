use std::io;
use std::path;

use thiserror::Error;
use toml;
use block_utils;
use fs_extra;

/// The Result type for badm.
pub type Result<T> = std::result::Result<T, Error>;

/// The Error type for badm.
#[derive(Error, Debug)]
pub enum Error {
    /// Config toml is malformed.
    #[error("could not parse toml")]
    InvalidToml(#[from] toml::de::Error),

    /// Wrapper around `io::Error`.
    #[error("meow")]
    StdIOError(#[from] io::Error),

    /// Wrapper around `std::path::StripPrefixError`.
    #[error("could not strip prefix")]
    StripPrefixError(#[from] path::StripPrefixError),

    /// Indicates bad input detected.
    #[error("bad input detected: {0}")]
    BadInput(&'static str),

    #[error("unable to retrieve path device info")]
    BlockUtilsError(#[from] block_utils::BlockUtilsError),

    #[error("fs_extra error")]
    FSExtraError(#[from] fs_extra::error::Error),

    #[error("unable to retrieve path device info for {0}")]
    UnableToRetrievePathDeviceInfo(path::PathBuf),

    #[error("config not found")]
    ConfigNotFound,

    #[error("missing HOME directory!")]
    MissingHomeDirectory,

    #[error("expected path '{0}' to exist, but it doesn't")]
    PathDoesNotExist(path::PathBuf),

    #[error("'{0}' already exists and doesn't point to the expected dotfile")]
    SymlinkPathAlreadyExists(path::PathBuf),

    #[error("could not find specified dotfile: {0}")]
    DotfileNotFound(path::PathBuf),

    #[error("invalid dotfile destination directory: {0}")]
    InvalidDotfileDestinationDirectory(path::PathBuf),

    #[error("invalid symlink destination directory: {0}")]
    InvalidSymlinkDestinationDirectory(path::PathBuf),

    #[error("no configured dotfile found that matches {0}")]
    NoMatchingDotfileConfigured(path::PathBuf),
}
