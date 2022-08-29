use std::collections::BTreeSet;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::path::PathBuf;

use dirs::config_dir;
use serde_derive::{Deserialize, Serialize};

use crate::errors::Error;
use crate::errors::Result;
use crate::paths;

/// Handles and saves configuration variables between application calls.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Config {
    /// Dotfiles configuration. Each `Dotfiles` corresponds to a potentially different top-level
    /// store of dotfiles.
    pub dotfiles: Vec<Dotfiles>,
}

/// Represents a top-level container of dotfiles each containing a subset of dotfiles to be synced
/// into `symlink_directory`. Each dotfile represented in the set of `paths` is considered to be a
/// relative to either the `dotfile_directory` or the `symlink_directory` and may consist of an
/// arbitrary number of path segments.
///
/// When stowing files, each relative path must actually exist in the `symlink_directory` and when
/// deploying or restoring files, each relative path must actually exist in the
/// `dotfile_directory`.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Dotfiles {
    /// Path relative to which dotfiles paths are truncated when determining the appropriate
    /// symlink path in `symlink_directory`.
    pub dotfile_directory: DotfilesDir,

    /// Path where dotfiles should "land" when deloyed or from which they should be moved and
    /// symlinked when stowed. If not set in config file, the default is $HOME.
    pub symlink_directory: SymlinkDir,

    /// Relative path of actual dotfiles. A dotfile is a regular file or directory stored outside
    /// of `symlink_directory` that user wants symlinked to `symlink_directory`.
    pub paths: BTreeSet<DotfilePath>,
}

/// DotfilesDir is directory path that must always exist where dotfiles are stored. The type doesn't
/// do much more than impose the aforementioned existence requirement and distinguish itself from
/// `SymlinkDir`s as well as run-of-the-mill `PathBuf`s.
///
/// Using the type system in this way helps other programmers make confident use of the code -- if
/// we just passed around `PathBuf`s there would always be some question as to whether the path has
/// been validated appropriately.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, Debug, PartialEq, Clone)]
pub struct DotfilesDir(PathBuf);

impl TryFrom<PathBuf> for DotfilesDir {
    type Error = Error;

    fn try_from(pb: PathBuf) -> Result<Self> {
        // verify path exists
        let _ = pb.symlink_metadata()?;

        Ok(Self(pb))
    }
}

impl Deref for DotfilesDir {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// SymlinkDir is directory path that must always exist where dotfiles are stored. The type doesn't
/// do much more than impose the aforementioned existence requirement and distinguish itself from
/// `DotfilesDir`s as well as run-of-the-mill `PathBuf`s.
///
/// Using the type system in this way helps other programmers make confident use of the code -- if
/// we just passed around `PathBuf`s there would always be some question as to whether the path has
/// been validated appropriately.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, Debug, PartialEq, Clone)]
pub struct SymlinkDir(PathBuf);

impl TryFrom<PathBuf> for SymlinkDir {
    type Error = Error;

    fn try_from(pb: PathBuf) -> Result<Self> {
        // verify path exists
        let _ = pb.symlink_metadata()?;

        Ok(Self(pb))
    }
}

impl Deref for SymlinkDir {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// DotfilePath is a type that is always relative. Meant to ensure that where a path is meant to be
/// a dotfile path it has already been validated as or transformed into a relative path.
#[derive(Serialize, Deserialize, Ord, PartialOrd, Eq, Debug, PartialEq, Clone)]
pub struct DotfilePath(PathBuf);

impl TryFrom<PathBuf> for DotfilePath {
    type Error = Error;

    fn try_from(pb: PathBuf) -> Result<Self> {
        if pb.is_relative() {
            return Ok(Self(pb));
        }
        Err(Error::DotfilePathMustBeRelative(pb.clone()))
    }
}

impl TryFrom<(DotfilesDir, PathBuf)> for DotfilePath {
    type Error = Error;

    fn try_from(pb: (DotfilesDir, PathBuf)) -> Result<Self> {
        let (dir, dotfile_path) = pb;
        if !dotfile_path.is_absolute() {
            return Ok(Self(dotfile_path.to_path_buf()));
        }
        if dotfile_path.starts_with(&*dir) {
            return Ok(Self(dotfile_path.strip_prefix(&*dir)?.to_path_buf()));
        }
        Err(Error::PathDoesNotStartWithPrefix(
            dotfile_path.clone(),
            dir.to_path_buf(),
        ))
    }
}

impl TryFrom<(&SymlinkDir, &DotfilesDir, &PathBuf)> for DotfilePath {
    type Error = Error;

    fn try_from(pb: (&SymlinkDir, &DotfilesDir, &PathBuf)) -> Result<Self> {
        let (symlink_dir, dotfile_dir, path) = pb;
        let mut symlink_path = path.clone();
        let dotfile_path: PathBuf;
        let mut result =  path.clone();

        // check if path is absolute (starts with root)
        if symlink_path.has_root() {
            //  if so, verify that it starts with the symlink dir
            if !symlink_path.starts_with(&**symlink_dir) {
                return Err(Error::PathDoesNotStartWithPrefix(
                    symlink_path,
                    symlink_dir.to_path_buf(),
                ));
            }

            // then strip the symlink dir and join onto the dotfile dir to create the correct dotfile
            // path
            dotfile_path = dotfile_dir.join(symlink_path.strip_prefix(&**symlink_dir)?);
            result = symlink_path.strip_prefix(&**symlink_dir)?.to_path_buf();
        } else {
            //  if not, try joining with symlink_dir and dotfile_dir
            symlink_path = symlink_dir.join(&path);
            dotfile_path = dotfile_dir.join(&path);
        }

        // check if dotfile path already exists
        let _ = symlink_path.symlink_metadata()?;

        // check if symlink already exists at symlink_dir/path
        let metadata = symlink_path.symlink_metadata()?;

        // check if symlink path is already a symlink
        if metadata.is_symlink() {
            let canonical = symlink_path.canonicalize()?;

            // check if symlink already points to the desired dotfile path
            if canonical == dotfile_path.canonicalize()? {
                // if they already point at the same path then it's a valid file
                return Ok(Self(result));
            }

            // any other symlink destination is invalid
            return Err(Error::SymlinkPathAlreadyExists(symlink_path));
        }
        Ok(Self(result))
    }
}

impl Deref for DotfilePath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Dotfiles {
    pub(crate) fn is_dotfile(&self, path: &PathBuf) -> bool {
        match DotfilePath::try_from((self.dotfile_directory.clone(), path.clone())) {
            Ok(p) => self.paths.contains(&p),
            Err(_) => false,
        }
    }

    pub(crate) fn restore_dotfile(&mut self, path: &DotfilePath) -> Result<Option<()>> {
        //let df_path = (self.dotfile_directory,

        // get dotfile and symlink paths. need to check in each branch if the given path belongs to
        // this set of Dotfiles so we can gracefully return Ok(None) if not
        let path_metadata = path.symlink_metadata()?;
        let (dotfile_path, symlink_path): (PathBuf, PathBuf) = if path_metadata.is_symlink() {
            let dotfile_path = fs::read_link(&**path)?;
            let symlink_path = path.to_path_buf();
            if !self.is_dotfile(&dotfile_path) {
                return Ok(None);
            }

            (dotfile_path, symlink_path)
        } else {
            if !self.is_dotfile(&path.clone()) {
                return Ok(None);
            }
            // strip dotfile directory
            let symlink_path = path.strip_prefix(&*self.dotfile_directory)?;
            // replace with home directory
            let symlink_path = self.symlink_directory.join(symlink_path);

            (path.to_path_buf(), symlink_path)
        };

        if symlink_path.exists() {
            fs::remove_file(&symlink_path)?;
        };

        paths::move_file(&dotfile_path, &symlink_path)?;

        Ok(Some(()))
    }

    // Deploy a dotfile from the dotfile store to the user's home directory.
    pub(crate) fn deploy(&self, path: &DotfilePath) -> Result<()> {
        let dotfile_path = self
            .dotfile_directory
            .exists()
            .then(|| self.dotfile_directory.clone())
            .ok_or(Error::PathDoesNotExist(
                self.dotfile_directory.to_path_buf(),
            ))?
            .join(&**path);

        // if the given dotfile_path doesn't exist at this point then it's definitely not legit
        if !dotfile_path.exists() {
            return Err(Error::DotfileNotFound(dotfile_path));
        }

        // don't try deploying a dotfile that we don't "own"
        if !self.is_dotfile(&dotfile_path) {
            return Err(Error::NoMatchingDotfileConfigured(dotfile_path));
        }

        let symlink_path = self.symlink_directory.join(&**path);
        if symlink_path.exists() {
            // read_link will return an error if:
            // * it is not a symbolic link
            // * it doesn't exist
            if fs::read_link(&symlink_path)? == dotfile_path {
                return Ok(());
            }
            // we reach this point if the path is a symlink but it doesn't point to the expected
            // dotfile. in that case, return an error
            return Err(Error::SymlinkPathAlreadyExists(symlink_path.clone()));
        }

        let symlink_path_dir =
            symlink_path
                .parent()
                .ok_or(Error::InvalidSymlinkDestinationDirectory(
                    symlink_path.clone(),
                ))?;

        if !symlink_path_dir.exists() {
            fs::create_dir_all(symlink_path_dir)?;
        }

        paths::create_symlink(&dotfile_path, &symlink_path)?;

        return Ok(());
    }

    pub(crate) fn deploy_all(&self) -> Result<()> {
        for path in self.paths.iter() {
            self.deploy(path)?;
        }
        Ok(())
    }

    fn stow_path(&mut self, stow_path: &DotfilePath) -> Result<()> {
        log::debug!("");
        log::debug!("stow_path: {:?}", stow_path);
        let symlink_path = self.symlink_directory.join(&**stow_path);
        log::debug!("");
        log::debug!("symlink_directory: {:?}", self.symlink_directory);
        log::debug!("symlink_path: {:?}", symlink_path);
        log::debug!("canonicalized symlink_path: {:?}", symlink_path.canonicalize()?);

        let dotfile_path = self.dotfile_directory.join(&**stow_path);
        log::debug!("");
        log::debug!("dotfile_directory: {:?}", self.dotfile_directory);
        log::debug!("dotfile_path: {:?}", dotfile_path);

        if dotfile_path.try_exists()? {
            if symlink_path.canonicalize()? == dotfile_path
            {
                log::debug!("");
                log::debug!("path already stowed: {:?}", stow_path);
                return Ok(());
            }

            return Err(Error::DotfilePathAlreadyExists(stow_path.to_path_buf()))
        }

        let _ = symlink_path.try_exists()?;
        log::debug!("creating symlink0");
        paths::move_file(&symlink_path, &dotfile_path)?;
        log::debug!("creating symlink5");
        paths::create_symlink(&dotfile_path, &symlink_path)?;

        log::debug!("stowed path: {:?}", stow_path);

        let _ = self.paths.insert(stow_path.clone());
        Ok(())
    }
}

impl Config {
    /// Load a config from disk and return it to caller.
    pub fn load() -> Result<Self> {
        if let Some(config_path) = Self::get_config_file() {
            let toml = crate::paths::read_path(&config_path)?;
            Ok(toml::from_str(&toml)?)
        } else {
            Ok(Self {
                dotfiles: Vec::new(),
            })
        }
    }

    /// Deploy specified dotfiles.
    pub fn deploy_paths(&self, paths: Vec<PathBuf>) -> Result<()> {
        'paths: for path in paths.iter() {
            for dotfiles in &self.dotfiles {
                let dotfile_path = match DotfilePath::try_from((
                    dotfiles.dotfile_directory.clone(),
                    path.to_path_buf(),
                )) {
                    Err(_) => continue,
                    Ok(p) => p,
                };
                match dotfiles.deploy(&dotfile_path) {
                    Err(Error::DotfileNotFound(_)) => continue,
                    Err(e) => return Err(e),
                    Ok(_) => continue 'paths,
                };
            }
            // if we reach this point then the current `path` hasn't been found. that's a whoopin'
            //
            // TODO: should we fail on the first dotfile that doesn't match? or should this be
            // all-or-nothing where if any specified dotfile doesn't match an existing dotfile then
            // we cowardly refuse to do anything
            //
            // i'm leaning toward all-or nothing, but that would ideally be a separate check from
            // the deployment step
            return Err(Error::NoMatchingDotfileConfigured(path.clone()));
        }
        Ok(())
    }

    /// Deploy all dotfiles.
    pub fn deploy_all(&self) -> Result<()> {
        for dotfiles in &self.dotfiles {
            dotfiles.deploy_all()?;
        }
        Ok(())
    }

    fn stow_path(
        &mut self,
        symlink_dir: &SymlinkDir,
        dotfile_dir: &DotfilesDir,
        stow_path: &DotfilePath,
    ) -> Result<()> {
        for dotfiles in &mut self.dotfiles {
            if dotfiles.dotfile_directory == *dotfile_dir
                && dotfiles.symlink_directory == *symlink_dir
            {
                return dotfiles.stow_path(stow_path);
            }
        }
        // if we reach this point then we need to create a new dotfiles entry in this config and
        // stow using that

        self.add_dotfiles(&symlink_dir, &dotfile_dir)?;
        self.dotfiles
            .last_mut()
            .ok_or(Error::UnexpectedError(
                "could not retrieve new dotfiles dir",
            ))?
            .stow_path(stow_path)?;
        Ok(())
    }

    /// Stow paths in given dotfile dir.
    pub fn stow_paths(
        &mut self,
        symlink_dir: SymlinkDir,
        dotfile_dir: DotfilesDir,
        stow_paths: Vec<DotfilePath>,
    ) -> Result<()> {
        for path in stow_paths.iter() {
            log::info!("stowing path: {:?}", path);
            self.stow_path(&symlink_dir, &dotfile_dir, path)?;
        }
        Ok(())
    }

    /// Restores the named dotfile if it can be found in one of the configured dotfile directories.
    pub fn restore_dotfile(&mut self, path: DotfilePath) -> Result<()> {
        for dotfiles in &mut self.dotfiles {
            log::debug!("meow");
            if let Some(_) = dotfiles.restore_dotfile(&path)? {
                return Ok(());
            }
            log::debug!("meow");
        }
        Err(Error::DotfileNotFound(path.to_path_buf()))
    }

    /// Adds new dotfiles to dotfile_dir
    pub fn add_dotfiles(
        &mut self,
        symlink_dir: &SymlinkDir,
        dotfile_dir: &DotfilesDir,
    ) -> Result<()> {
        if !dotfile_dir.exists() {
            return Err(Error::BadInput("path does not exist"));
        } else if !dotfile_dir.is_dir() {
            return Err(Error::BadInput("path must be a directory"));
        };

        if !symlink_dir.exists() {
            return Err(Error::BadInput("path does not exist"));
        } else if !symlink_dir.is_dir() {
            return Err(Error::BadInput("path must be a directory"));
        };

        self.dotfiles.push(Dotfiles {
            dotfile_directory: dotfile_dir.clone(),
            symlink_directory: symlink_dir.clone(),
            paths: BTreeSet::new(),
        });

        self.write_toml_config()?;
        Ok(())
    }

    /// Search `$HOME` and `$XDG_CONFIG_HOME` for config file path.
    fn get_config_file() -> Option<PathBuf> {
        let config_path = Self::config_file_path().ok()?;

        if config_path.exists() {
            return Some(config_path);
        };
        None
    }

    fn config_file_path() -> Result<PathBuf> {
        Ok(config_dir()
            .ok_or(Error::CannotDetermineConfigDir)?
            .join("badm")
            .join("config.toml"))
    }

    /// Save configuration variables to config file `.badm.toml`. If file cannot be found
    /// it will be written to $HOME.
    ///
    /// Valid locations for file location include: `$HOME` and `$XDG_CONFIG_HOME`.
    pub fn write_toml_config(&self) -> Result<()> {
        let config_file_path = Self::config_file_path()?;
        fs::create_dir_all(
            config_file_path
                .parent()
                .ok_or(Error::CannotDetermineConfigDir)?,
        )?;
        let toml = toml::to_string(&self).unwrap();
        let mut file = File::create(config_file_path)?;

        file.write_all(&toml.into_bytes())?;
        file.sync_data()?;

        Ok(())
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        self.write_toml_config().unwrap();
    }
}
