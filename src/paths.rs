//! Includes paths/fs-specific helper functions.
use std::env;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use block_utils::get_device_from_path;

use crate::errors::{Error, Result};

/// Wrapper for `is_symlink` for paths
pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|md| md.file_type().is_symlink())
        .unwrap_or(false)
}

/// Verifies path exists and returns absolute path.
pub fn get_absolute(path: PathBuf) -> Result<PathBuf> {
    let mut absolute = env::current_dir()?;
    absolute.push(path);

    // symlink_metadata should return an error if the path doesn't exist
    let _ = absolute.symlink_metadata()?;

    Ok(absolute)
}

pub(crate) fn read_path(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    read_file(&mut file)
}

pub(crate) fn read_file(file: &mut File) -> Result<String> {
    let mut contents = String::new();
    let _ = file.read_to_string(&mut contents)?;

    Ok(contents)
}

/// Joins two full paths together.
/// If path is unix and second path argument contains root directory, it is stripped.
///
/// This behavior is an anti-use case of [`PathBuf::join`], but is valid for the need to
/// replicate directory paths containing root within others.
///
/// [`PathBuf::join`]: std/path/struct.PathBuf.html#method.join
///
/// # Examples
///
/// ```
/// use badm::paths::join_full_paths;
/// use std::path::PathBuf;
///
/// assert_eq!(
///     join_full_paths(
///         &PathBuf::from("/home/ferris/.dotfiles"),
///         &PathBuf::from("/home/ferris")
///     ),
///     Ok(PathBuf::from("/home/ferris/.dotfiles/home/ferris"))
/// );
/// ```
#[allow(clippy::module_name_repetitions)]
pub fn join_full_paths(path_1: &Path, path_2: &Path) -> Result<PathBuf> {
    if path_2.has_root() && cfg!(target_family = "unix") {
        let path_2 = path_2.strip_prefix("/")?;
        return Ok(path_1.join(path_2));
    };
    Ok(path_1.join(path_2))
}

/// Store a file in the dotfiles directory, create a symlink at the original
/// source of the stowed file.
pub fn store_file(src: PathBuf, dst: PathBuf) -> Result<()> {
    move_file(src.clone(), dst.clone())?;
    create_symlink(dst, src)?;
    Ok(())
}

/// Read file at path src and write to created/truncated file at path dst.
pub fn move_file(src: PathBuf, dst: PathBuf) -> Result<()> {
    let (sid, ssn) = match get_device_from_path(src.clone())? {
        (_, None) => return Err(Error::UnableToRetrievePathDeviceInfo(src.clone())),
        (_, Some(d)) => (d.id, d.serial_number),
    };
    let (did, dsn) = match get_device_from_path(dst.clone())? {
        (_, None) => return Err(Error::UnableToRetrievePathDeviceInfo(dst.clone())),
        (_, Some(d)) => (d.id, d.serial_number),
    };

    let same = match (sid, ssn, did, dsn) {
        (Some(sid), Some(ssn), Some(did), Some(dsn)) => sid == did && ssn == dsn,
        (Some(sid), _, Some(did), _) => sid == did,
        (_, Some(ssn), _, Some(dsn)) => ssn == dsn,
        (_, _, _, _) => false,
    };

    if same {
        // if src and dst are on the same filesystem, there is no need to copy bytes around at all,
        // just rename the file
        fs::rename(src, dst)?;
    } else {
        let meta = src.symlink_metadata()?;
        if meta.is_symlink() || meta.is_file() {
            let mut opts = fs_extra::file::CopyOptions::new();
            opts.overwrite = false;
            opts.skip_exist = true;
            let _ = fs_extra::file::move_file(src, dst, &opts)?;
        } else { // it's a dir
            let mut opts = fs_extra::dir::CopyOptions::new();
            opts.overwrite = false;
            opts.skip_exist = true;
            opts.copy_inside = true;
            let _ = fs_extra::dir::move_dir(src, dst, &opts)?;
        }
    }

    Ok(())
}

/// Create a symlink at "dst" pointing to "src."
///
/// For Unix platforms, [`std::os::unix::fs::symlink`] is used to create
/// symlinks. For Windows, [`std::os::windows::fs::symlink_file`] is used.
///
/// [`std::os::unix::fs::symlink`]: std/os/unix/fs/fn.symlink.html
/// [`std::os::windows::fs::symlink_file`]: std/os/windows/fs/fn.symlink_file.html
pub fn create_symlink(src: PathBuf, dst: PathBuf) -> io::Result<()> {
    #[cfg(not(target_os = "windows"))]
    use std::os::unix::fs::symlink;

    #[cfg(target_os = "windows")]
    use std::os::windows::fs::symlink_file as symlink;
    symlink(src, dst)
}
