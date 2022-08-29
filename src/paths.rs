//! Includes paths/fs-specific helper functions.
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::os::linux::fs::MetadataExt;

use crate::errors::Result;

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
    move_file(&src, &dst)?;
    create_symlink(&dst, &src)?;
    Ok(())
}

/// Read file at path src and write to created/truncated file at path dst.
pub fn move_file(src: &PathBuf, dst: &PathBuf) -> Result<()> {
        println!("creating symlink2");
    let src_meta = src.symlink_metadata()?;
        println!("creating symlink3");

        println!("creating symlink4");
    if dst.exists() && src_meta.st_dev() == dst.symlink_metadata()?.st_dev() {
        // if src and dst are on the same filesystem, there is no need to copy bytes around at all,
        // just rename the file
        fs::rename(src, dst)?;
    } else {
        if src_meta.is_symlink() || src_meta.is_file() {
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
pub fn create_symlink(src: &PathBuf, dst: &PathBuf) -> io::Result<()> {
    #[cfg(not(target_os = "windows"))]
    use std::os::unix::fs::symlink;

    #[cfg(target_os = "windows")]
    use std::os::windows::fs::symlink_file as symlink;
    symlink(src, dst)
}
