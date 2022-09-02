//! `ghmd` is a tool that manages symlinmks. Usually these symlinks point to  
//! [dotfiles](https://en.wikipedia.org/wiki/Hidden_file_and_hidden_directory) stored somewhere
//! outside your home directory.
//!
//! ghmd stands for "Gotta Have My Dots" because the original intent is for managing symlinks
//! pointing at dotfiles.
//!

#![cfg_attr(test, deny(warnings))]
#![deny(clippy::all)]
#![allow(clippy::must_use_candidate)]
#![deny(
    future_incompatible,
    missing_debug_implementations,
    missing_docs,
    missing_copy_implementations,
    missing_docs,
    nonstandard_style,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    unused_qualifications
)]

pub(crate) mod config;
mod errors;
pub mod paths;

pub use crate::config::Config;
pub use crate::config::{DotfilesDir, DotfilePath, SymlinkDir};
pub use crate::errors::Result;
