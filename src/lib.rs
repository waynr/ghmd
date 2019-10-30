#![allow(dead_code)]

pub use crate::config::Config;

mod config;

use std::fs::{self, File};
use std::io::{self, prelude::*, BufWriter, Error, ErrorKind};
use std::path::{self, Path, PathBuf};

// TODO: create dotfile struct

/// Dotfile is removed from the set dotfiles directory and moved to its symlink location.
/// The input can either be a dotfile's symlink path or the dotfile path itself.
pub fn unstow_dotfile<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_path_buf();

    // get src and dst paths
    let (src_path, dst_path): (PathBuf, PathBuf) = if is_symlink(&path)? {
        let src_path = fs::read_link(&path)?;
        fs::remove_file(&path)?;

        (src_path, path)
    } else {
        // check to see if file is in dotfiles dir
        let dots_dir = Config::get_dots_dir().unwrap();

        if !path.starts_with(&dots_dir) {
            // throw error
            let err = Error::new(
                ErrorKind::InvalidInput,
                "input path not located in dotfiles dir!",
            );
            return Err(err);
        };
        let dst_path =
            PathBuf::from("/").join(path.strip_prefix(dots_dir).unwrap().to_path_buf());

        if dst_path.exists() {
            fs::remove_file(&dst_path)?;
        };
        println!("{:?}", &dst_path);

        (path, dst_path)
    };

    FileHandler::move_file(src_path, dst_path)?;

    Ok(())
}

/// Create symlinks in directories relative to the dotfiles' directory hierarchy
/// for "rolling out" new configurations.
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
pub fn create_dotfile_symlink<P: AsRef<Path>>(src: P) -> io::Result<()> {
    let dots_dir = Config::get_dots_dir().unwrap();
    println!("{:?}", dots_dir);

    let dst_symlink = PathBuf::from("/").join(
        src.as_ref()
            .strip_prefix(dots_dir)
            .expect("Not able to create destination path"),
    );

    // if symlink already exists and points to src file, early return
    if dst_symlink.exists() && fs::read_link(&dst_symlink)? == src.as_ref() {
        return Ok(());
    };

    let dst_dir = dst_symlink.parent().unwrap();
    if !dst_dir.exists() {
        fs::create_dir_all(dst_dir)?;
    };

    FileHandler::create_symlink(&src, &dst_symlink)
}

// REVIEW:
//     - recursive flag?
//     - glob patterns?
pub fn stow_dotfile<P: AsRef<Path>>(src: P) -> io::Result<PathBuf> {
    // create destination path
    let dots_dir = match Config::get_dots_dir() {
        Some(dir) => dir,
        None => {
            let err = io::Error::new(
                io::ErrorKind::NotFound,
                "Not able to complete operation because BADM_DIR was not set. Please \
                 run `badm set-home=<DIR> first.`",
            );
            return Err(err);
        }
    };

    let dst_path = join_full_paths(dots_dir, &src).unwrap();

    // if symlink already exists and points to src file, early return
    if dst_path.exists() && fs::read_link(&dst_path)? == src.as_ref() {
        return Ok(dst_path);
    };

    // create directory if not available
    let dst_dir = dst_path.parent().unwrap();

    if !dst_dir.exists() {
        fs::create_dir_all(dst_dir)?;
    };

    // move dotfile to dotfiles directory
    FileHandler::store_file(src, &dst_path)?;

    Ok(dst_path)
}

pub struct FileHandler;

impl FileHandler {
    /// Store a file in the dotfiles directory, create a symlink at the original
    /// source of the stowed file.
    pub fn store_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<()> {
        let src = src.as_ref();
        let dst = dst.as_ref();

        FileHandler::move_file(&src, &dst)?;

        FileHandler::create_symlink(dst, src)?;

        Ok(())
    }

    /// Create a symlink at "dst" pointing to "src."
    ///
    /// For Unix platforms, std::os::unix::fs::symlink is used to create
    /// symlinks. For Windows, std::os::windows::fs::symlink_file is used.
    // TODO|BUGFIX: ensure or throw error when dst parent does not exist
    pub fn create_symlink<P: AsRef<Path>, Q: AsRef<Path>>(
        src: P,
        dst: Q,
    ) -> io::Result<()> {
        let src = src.as_ref();
        let dst = dst.as_ref();

        #[cfg(not(target_os = "windows"))]
        use std::os::unix::fs::symlink;

        #[cfg(target_os = "windows")]
        use std::os::windows::fs::symlink_file as symlink;
        symlink(src, dst)?;

        Ok(())
    }

    pub fn move_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<()> {
        let src = src.as_ref();
        let dst = dst.as_ref();

        // read file to String
        let mut contents = String::new();
        let mut f = File::open(src)?;
        f.read_to_string(&mut contents)?;

        // write String contents to dst file
        let dst_file = File::create(dst)?;
        let mut writer = BufWriter::new(dst_file);
        writer.write_all(contents.as_bytes())?;

        // writer.into_inner().unwrap().sync_all()?;

        // remove file at src location
        fs::remove_file(&src)?;

        Ok(())
    }
}

/// Joins two full paths together.
/// If path is unix and second path argument contains root directory, it is stripped.
///
/// This behavior is an anti-use case of [`PathBuf::join`], but is valid for the need to
/// replicate directory paths containing root within others.
///
/// [`PathBuf`::join]: struct.PathBuf.html#method.join
///
/// # Examples
///
/// ```
/// use badm_core::join_full_paths;
/// use std::path::PathBuf;
/// # use std::path;
///
/// let path_1 = PathBuf::from("/home/ferris/.dotfiles");
/// let path_2 = PathBuf::from("/home/ferris");
///
/// assert_eq!(
///     join_full_paths(&path_1, &path_2),
///     Ok(PathBuf::from("/home/ferris/.dotfiles/home/ferris"))
/// );
/// ```
// TODO: test windows root paths
pub fn join_full_paths<P: AsRef<Path>, Q: AsRef<Path>>(
    path_1: P,
    path_2: Q,
) -> Result<PathBuf, path::StripPrefixError> {
    let path_1 = path_1.as_ref();
    let path_2 = path_2.as_ref();

    if path_2.has_root() && cfg!(target_family = "unix") {
        let path_2 = path_2.strip_prefix("/")?;
        return Ok(path_1.join(path_2));
    };
    Ok(path_1.join(path_2))
}

pub fn is_symlink<P: AsRef<Path>>(location: P) -> io::Result<bool> {
    let filetype = fs::symlink_metadata(location)?.file_type();

    Ok(filetype.is_symlink())
}