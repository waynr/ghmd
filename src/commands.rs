//! Includes the commands used by the badm crate/application.

use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};

use crate::errors::Error;
use crate::errors::Result;
use crate::paths::join_full_paths;
use crate::Config;
use crate::FileHandler;

/// Take input from file at path and store in set dotfiles directory.
pub fn store_dotfile(config: &Config, path: &Path) -> Result<PathBuf> {
    let dots_dir = config.get_dots_dir();

    // create destination path
    let dst_path = join_full_paths(&dots_dir, &path)?;

    // if symlink already exists and points to src file, early return
    if dst_path.exists() && fs::read_link(&dst_path)? == path {
        return Ok(dst_path);
    };

    // create directory if not available
    let dst_dir = dst_path
        .parent()
        .ok_or(Error::InvalidDotfileDestinationDirectory(dst_path.clone()))?;

    if !dst_dir.exists() {
        fs::create_dir_all(dst_dir)?;
    };

    // move dotfile to dotfiles directory
    FileHandler::move_file(&path, &dst_path)?;

    Ok(dst_path)
}

/// Create symlinks in directories relative to the dotfiles' directory hierarchy
/// for deploying new configurations.
/// Example: if Ferris downloaded a git dotfiles repo onto a new machine into the
/// .dotfiles directory:
///
/// <pre>
/// /home
/// └── ferris
///     └── .dotfiles
///         └── home
///             └── ferris
///                 └── .config
///                     └── .gitconfig
/// </pre>
///
/// They could easily setup their configuration files on this machine by setting
/// up the relative symlinks by storing their configuration files in one directory, and
/// have that directory mimic the directory hiearchy of the target machine. This is what
/// BADM hopes to achieve.
///
/// <pre>
/// /home
/// └── ferris
///     ├── .config
///     │   └── .gitconfig -> /home/ferris/.dotfiles/home/ferris/.config/.gitconfig
///     └── .dotfiles
///         └── home
///             └── ferris
///                 └── .config
///                     └── .gitconfig
/// </pre>
///
/// Directories to replicate the stored dotfile's directory structure will be created if
/// not found.
// REVIEW: not enough checks - need to ensure valid entry.
pub fn deploy_dotfile(src: &Path, dst: &Path) -> io::Result<()> {
    // if symlink already exists and points to src file, early return
    if dst.exists() && fs::read_link(&dst)? == src {
        return Ok(());
    };

    let dst_dir = dst.parent().unwrap();
    if !dst_dir.exists() {
        fs::create_dir_all(dst_dir)?;
    };

    FileHandler::create_symlink(&src, &dst)
}
