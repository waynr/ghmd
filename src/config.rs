use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::errors::InputError;
use dirs::{config_dir, home_dir};
use serde_derive::{Deserialize, Serialize};

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
    pub paths: Vec<PathBuf>,
}

impl Config {
    pub(crate) fn new<P: AsRef<Path>>(
        directory: P,
        dotfiles: Vec<P>,
    ) -> Result<Self, InputError> {
        let directory = directory.as_ref().to_path_buf();

        if directory.is_dir() {
            Ok(Self {
                dotfiles: Vec::from(vec![Dotfiles {
                    directory,
                    paths: dotfiles.iter().map(|p| p.as_ref().to_path_buf()).collect(),
                }]),
            })
        } else {
            Err(InputError::BadInput {
                err: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Input to set dots directory is invalid",
                ),
            })
        }
    }

    /// Adds new top-level dotfiles directory, and writes TOML config file.
    ///
    /// Only accepts existing dotfiles directory.
    pub fn add_dir<P: AsRef<Path>>(path: P) -> Result<(), InputError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(InputError::BadInput {
                err: io::Error::new(io::ErrorKind::InvalidInput, "path does not exist"),
            });
        } else if !path.is_dir() {
            return Err(InputError::BadInput {
                err: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "path must be a directory",
                ),
            });
        };

        let config = Self::new(&path, Vec::new())?;

        config.write_toml_config()?;
        Ok(())
    }

    /// If config file `.badm.toml` exists, get dotfiles directory path.
    pub fn get_dots_dir() -> Option<PathBuf> {
        if let Some(config_path) = Self::get_config_file() {
            let toml = crate::paths::read_path(&config_path).unwrap();

            let config: Self = toml::from_str(&toml).expect("Not able to read config!");
            // for now, assume there is only one dotfiles directory, later we can add more if
            // necessary.
            Some(config.dotfiles[0].directory.clone())
        } else {
            None
        }
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
    pub fn write_toml_config(self) -> io::Result<()> {
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
    type Error = InputError;
    fn try_from(file: File) -> Result<Self, Self::Error> {
        let mut file = file;

        let contents = crate::paths::read_file(&mut file)?;

        Ok(toml::from_str(&contents)?)
    }
}

impl TryFrom<PathBuf> for Config {
    type Error = InputError;
    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(path)?;
        Self::try_from(file)
    }
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config: Self = toml::from_str(s)?;
        Ok(config)
    }
}
