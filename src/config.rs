use std::collections::BTreeSet;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use dirs::{config_dir, home_dir};
use serde_derive::{Deserialize, Serialize};

use crate::errors::Error;
use crate::errors::Result;
use crate::paths;
use crate::paths::is_symlink;

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
    pub dotfile_directory: PathBuf,

    /// Path where dotfiles should "land" when deloyed or from which they should be moved and
    /// symlinked when stowed. If not set in config file, the default is $HOME.
    pub symlink_directory: PathBuf,

    /// Relative path of actual dotfiles. A dotfile is a regular file or directory stored outside
    /// of `symlink_directory` that user wants symlinked to `symlink_directory`.
    pub paths: BTreeSet<PathBuf>,
}

impl Dotfiles {
    pub(crate) fn is_dotfile(&self, path: &PathBuf) -> bool {
        self.paths.contains(path)
    }

    pub(crate) fn restore_dotfile(&mut self, path: &PathBuf) -> Result<Option<()>> {
        // get dotfile and symlink paths. need to check in each branch if the given path belongs to
        // this set of Dotfiles so we can gracefully return Ok(None) if not
        let (dotfile_path, symlink_path): (PathBuf, PathBuf) = if is_symlink(path) {
            let dotfile_path = fs::read_link(path)?;
            let symlink_path = path.to_path_buf();
            if !self.is_dotfile(&dotfile_path) {
                return Ok(None);
            }

            (dotfile_path, symlink_path)
        } else {
            if !self.is_dotfile(path) {
                return Ok(None);
            }
            // strip dotfile directory
            let symlink_path = path.strip_prefix(&self.dotfile_directory)?;
            // replace with home directory
            let symlink_path =
                PathBuf::from(home_dir().ok_or(Error::MissingHomeDirectory)?)
                    .join(symlink_path);

            (path.to_path_buf(), symlink_path)
        };

        if symlink_path.exists() {
            fs::remove_file(&symlink_path)?;
        };

        paths::move_file(dotfile_path, symlink_path)?;

        Ok(Some(()))
    }

    // Deploy a dotfile from the dotfile store to the user's home directory.
    pub(crate) fn deploy(&self, path: &PathBuf) -> Result<()> {
        let mut dotfile_path = path.clone();
        if !dotfile_path.exists() {
            // if the specified path doesn't exist, treat it as a relative path and try again
            dotfile_path = self
                .dotfile_directory
                .exists()
                .then(|| self.dotfile_directory.clone())
                .ok_or(Error::PathDoesNotExist(self.dotfile_directory.clone()))?
                .join(dotfile_path);
        }

        // if the given dotfile_path doesn't exist at this point then it's definitely not legit
        if !dotfile_path.exists() {
            return Err(Error::DotfileNotFound(dotfile_path));
        }

        // don't try deploying a dotfile that we don't "own"
        if !self.is_dotfile(&dotfile_path) {
            return Err(Error::NoMatchingDotfileConfigured(dotfile_path));
        }

        // this is kind of silly considering we intentionally made the path absolute above, but
        // since it's a better user experience not to assume either absolute or relative paths we
        // need to strip the dotfile prefix before reconstructing the symlink path here.
        //
        // also, TODO: sort out all the references and cloning going on here...
        let relative_path = dotfile_path.strip_prefix(self.dotfile_directory.clone())?;

        let symlink_path = self.symlink_directory.join(relative_path);
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

        paths::create_symlink(dotfile_path, symlink_path)?;

        return Ok(());
    }

    pub(crate) fn deploy_all(&self) -> Result<()> {
        for path in self.paths.iter() {
            self.deploy(path)?;
        }
        Ok(())
    }
}

impl Config {
    /// Load a config from disk and return it to caller.
    pub fn load() -> Result<Self> {
        if let Some(config_path) = Self::get_config_file() {
            let toml = crate::paths::read_path(&config_path)?;

            let mut config: Self = toml::from_str(&toml)?;
            for dotfiles in config.dotfiles.iter_mut() {
                if dotfiles.symlink_directory.as_os_str().len() == 0 {
                    dotfiles.symlink_directory =
                        home_dir().ok_or(Error::MissingHomeDirectory)?;
                }
            }
            Ok(config)
        } else {
            Ok(Self {
                dotfiles: Vec::new(),
            })
        }
    }

    /// Deploy specified dotfiles.
    pub fn deploy_paths(&self, paths: Vec<PathBuf>) -> Result<()> {
        'paths: for path in paths.iter() {
            for dotfiles in self.dotfiles.iter() {
                match dotfiles.deploy_all() {
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
        for dotfiles in self.dotfiles.iter() {
            dotfiles.deploy_all()?;
        }
        Ok(())
    }

    /// Stow paths in given dotfile dir.
    pub fn stow<P: AsRef<Path>>(&self, _dotfile_dir: P, _stow_paths: Vec<P>) -> Result<()> {
        Ok(())
    }

    /// Restores the named dotfile if id can be found in one of the configured dotfile directories.
    pub fn restore_dotfile(&mut self, path: PathBuf) -> Result<()> {
        for dotfiles in self.dotfiles.iter_mut() {
            if let Some(_) = dotfiles.restore_dotfile(&path)? {
                return Ok(());
            }
        }
        Err(Error::DotfileNotFound(path.clone()))
    }

    /// Adds new top-level dotfiles directory, and writes TOML config file.
    ///
    /// Only accepts existing dotfiles directory.
    pub fn add_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(Error::BadInput("path does not exist"));
        } else if !path.is_dir() {
            return Err(Error::BadInput("path must be a directory"));
        };

        self.dotfiles.push(Dotfiles {
            dotfile_directory: path.to_path_buf(),
            symlink_directory: home_dir().ok_or(Error::MissingHomeDirectory)?,
            paths: BTreeSet::new(),
        });

        self.write_toml_config()?;
        Ok(())
    }

    /// If config file `.badm.toml` exists, get dotfiles directory path.
    ///
    /// deprecated -- only returns the first directory path
    pub fn get_dots_dir(&self) -> PathBuf {
        self.dotfiles[0].dotfile_directory.clone()
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
