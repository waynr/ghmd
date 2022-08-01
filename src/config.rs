use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use dirs::{config_dir, home_dir};
use serde_derive::{Deserialize, Serialize};

use crate::errors::Error;
use crate::errors::Result;
use crate::paths::is_symlink;
use crate::FileHandler;

/// Handles and saves configuration variables between application calls.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Config {
    /// Dotfiles configuration. Each `Dotfiles` corresponds to a potentially different top-level
    /// store of dotfiles.
    pub dotfiles: Vec<Dotfiles>,
}

/// Represents a top-level container of dotfiles each containing a subset of dotfiles to be synced
/// into $HOME.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Dotfiles {
    /// Path relative to which dotfiles paths are truncated when determining the appropriate
    /// symlink path in $HOME.
    pub directory: PathBuf,

    /// Path of actual dotfiles. A dotfile is a regular file or directory stored outside of $HOME that
    /// user wants symlinked to $HOME.
    pub paths: HashMap<PathBuf, bool>,
}

impl Dotfiles {
    pub(crate) fn is_dotfile(&self, path: &PathBuf) -> bool {
        self.paths.contains_key(path)
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
            let symlink_path = path.strip_prefix(&self.directory)?;
            // replace with home directory
            let symlink_path =
                PathBuf::from(home_dir().ok_or(Error::MissingHomeDirectory)?)
                    .join(symlink_path);

            (path.to_path_buf(), symlink_path)
        };

        if symlink_path.exists() {
            fs::remove_file(&symlink_path)?;
        };

        FileHandler::move_file(&dotfile_path, &symlink_path)?;

        Ok(Some(()))
    }
}

impl Config {
    /// Load a config from disk and return it to caller.
    pub fn load() -> Result<Self> {
        if let Some(config_path) = Self::get_config_file() {
            let toml = crate::paths::read_path(&config_path)?;

            let config: Self = toml::from_str(&toml).expect("Not able to read config!");
            Ok(config)
        } else {
            Ok(Self {
                dotfiles: Vec::new(),
            })
        }
    }

    /// Restores the named dotfile if id can be found in one of the configured dotfile directories.
    pub fn restore_dotfile(&mut self, path: PathBuf) -> Result<()> {
        for dotfiles in self.dotfiles.iter_mut() {
            if let Some(_) = dotfiles.restore_dotfile(&path)? {
                return Ok(());
            }
        }
        Ok(())
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
            directory: path.to_path_buf(),
            paths: HashMap::new(),
        });

        self.write_toml_config()?;
        Ok(())
    }

    /// If config file `.badm.toml` exists, get dotfiles directory path.
    ///
    /// deprecated -- only returns the first directory path
    pub fn get_dots_dir(&self) -> PathBuf {
        self.dotfiles[0].directory.clone()
    }

    /// Search `$HOME` and `$XDG_CONFIG_HOME` for config file path.
    fn get_config_file() -> Option<PathBuf> {
        let search_paths = |file_name: &str, dirs_vec: Vec<PathBuf>| -> Option<PathBuf> {
            for dir in dirs_vec {
                let possible_file_path = dir.join(file_name);

                if possible_file_path.exists() {
                    return Some(possible_file_path);
                };
            }
            None
        };

        let config_file_name = ".badm.toml";
        search_paths(
            config_file_name,
            vec![config_dir().unwrap(), home_dir().unwrap()],
        )
    }

    /// Save configuration variables to config file `.badm.toml`. If file cannot be found
    /// it will be written to $HOME.
    ///
    /// Valid locations for file location include: `$HOME` and `$XDG_CONFIG_HOME`.
    pub fn write_toml_config(&self) -> Result<()> {
        // check to see if config file already exists, if not default to HOME
        let config_file_path = match Self::get_config_file() {
            Some(path) => path,
            None => config_dir().unwrap().join(".badm.toml"),
        };

        let toml = toml::to_string(&self).unwrap();
        let mut file = File::create(config_file_path)?;

        file.write_all(&toml.into_bytes())?;
        file.sync_data()?;

        Ok(())
    }
}

impl TryFrom<File> for Config {
    type Error = Error;
    fn try_from(file: File) -> std::result::Result<Self, Self::Error> {
        let mut file = file;

        let contents = crate::paths::read_file(&mut file)?;

        Ok(toml::from_str(&contents)?)
    }
}

impl TryFrom<PathBuf> for Config {
    type Error = Error;
    fn try_from(path: PathBuf) -> std::result::Result<Self, Self::Error> {
        let file = File::open(path)?;
        Self::try_from(file)
    }
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let config: Self = toml::from_str(s)?;
        Ok(config)
    }
}
