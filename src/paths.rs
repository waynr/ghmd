//! Includes paths/fs-specific helper functions.
use std::fs;
use std::io;
use std::path::PathBuf;
use std::os::linux::fs::MetadataExt;

use crate::errors::Result;

/// Read file at path src and write to created/truncated file at path dst.
pub fn move_file(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    let src_meta = src.symlink_metadata()?;

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
