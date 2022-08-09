//! `badm` is a tool that stores your configuration files, or
//! [dotfiles](https://en.wikipedia.org/wiki/Hidden_file_and_hidden_directory), in a directory that replicates the directory hierarchy of the
//! dotfiles' original path, and creates symlinks to their original paths. This creates a
//! standardized and systematic approach for managing, deploying, and sharing dotfiles
//! among different systems and users.
//!
//! badm is ultimately "But Another Dotfiles Manager".
//!
//! # Examples
//!
//! - ferris has created a directory to store their dotfiles at `~/.dots`
//! - `badm set-dir ~/.dots` sets the BADM dotfiles dir at `~/.dots`
//! - badm will search for a badm config file at one of the two valid locations: `$HOME`
//!   and `$XDG_CONFIG_HOME`. If the config file not found, badm will create it under
//!   `$HOME`
//!
//! <pre>
//! /home
//! └── ferris
//!     └── .dots
//!         ├── .badm.toml
//!         └── .gitconfig
//! </pre>
//!
//!
//! - to store `~/.gitconfig` as a dotfile, ferris runs `badm stow ~/.gitconfig`
//!   _(relative paths work as well)_
//! - badm replicates the path of the dotfile under the `~/.dots` directory
//! - the dotfile is moved to this new path in the set dotfiles directory and symlinked at
//!   its original path which points to its new path
//!
//! <pre>
//! /home
//! └── ferris
//!     ├── .badm.toml
//!     ├── .dots
//!     │   └── home
//!     │       └── ferris
//!     │           └── .gitconfig
//!     └── .gitconfig -> /home/ferris/.dots/home/ferris/.gitconfig
//! </pre>
//!
//! # Commands
//!
//! - `badm set-dir <DIRECTORY>` - set dotfiles directory location, if the location is not
//!   created BADM has the ability to create one for you
//! - `badm stow <FILE>` - store a file in the dotfiles directory, create a symlink at the
//!   original source of the stowed file.
//! - `badm deploy <FILE>` - for new configurations, create symlinks in directories
//!   relative to the dotfile's directory hierarchy. Directories to replicate the stored
//!   dotfile's directory structure will be created if not found.
//! - `badm restore <FILE>` - restore the stored file from the dotfiles directory and
//!   replace the symlink with the original file

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
pub use crate::errors::Result;
