use glob::glob;
use std::path::{Path, PathBuf};

#[macro_use]
extern crate clap;

use anyhow::{anyhow, Result};
use clap::{App, AppSettings, Arg, ArgMatches};

use badm::paths;
use badm::Config;

fn main() -> Result<()> {
    let set_dir_subcommand = App::new("add-dotfiles-dir")
        .about("add new dotfiles directory")
        .version("1.0")
        .display_order(1)
        .arg(
            Arg::with_name("directory")
                .help("directory to store dotfiles")
                .required(true),
        );

    let stow_subcommand = App::new("stow")
        .about(
            "store input files in the dotfiles directory, and replace the file's \
             original path with a symlink",
        )
        .version("0.1")
        .display_order(2)
        .arg(
            Arg::with_name("files")
                .help("path of the file/files to be stored in the dotfiles directory")
                .required(true)
                .multiple(true),
        );

    let deploy_subcommand = App::new("deploy")
        .about(
            "for new configurations, create symlinks in directories relative to the \
             dotfile's directory hierarchy. Directories to replicate the stored \
             dotfile's directory structure will be created if not found.",
        )
        .version("0.1")
        .display_order(3)
        .arg(
            Arg::with_name("dotfiles")
                .help("stored dotfile/s to be deployed to system")
                .required(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("all")
                .help("deploy all stored dotfiles")
                .long("all")
                .conflicts_with("dotfiles")
                .required(true),
        );

    let restore_subcommand = App::new("restore")
        .about("restore all dotfiles to their original locations")
        .version("0.1")
        .display_order(4)
        .arg(
            Arg::with_name("dotfiles")
                .help("the dotfiles to restore to original locations")
                .multiple(true)
                .required(true),
        );

    let matches = App::new("badm")
        .setting(AppSettings::ArgRequiredElseHelp)
        .about(crate_description!())
        .version(crate_version!())
        .author(crate_authors!())
        .after_help("https://github.com/jakeschurch/badm")
        .subcommands(vec![
            set_dir_subcommand,
            stow_subcommand,
            deploy_subcommand,
            restore_subcommand,
        ])
        .get_matches();

    let mut config = Config::load()?;

    match matches.subcommand() {
        ("add-dotfiles-dir", Some(set_dir_matches)) => {
            let dir_path = set_dir_matches.value_of("directory").unwrap();
            add_dir(&mut config, dir_path)?
        },
        ("stow", Some(stow_matches)) => stow(config, stow_matches)?,
        ("deploy", Some(deploy_matches)) => deploy(config, deploy_matches)?,
        ("restore", Some(restore_matches)) => restore(&mut config, restore_matches)?,
        (s, _) => return Err(anyhow!("invalid subcommand: {0}", s)),
    }
    Ok(())
}

fn add_dir<P: AsRef<Path>>(config: &mut Config, path: P) -> Result<()> {
    let path = path.as_ref().to_path_buf();

    config.add_dir(path)?;

    println! {"dotfiles path successfully added"};
    Ok(())
}

fn stow(config: Config, values: &ArgMatches) -> Result<()> {
    let mut paths = vec![];

    for path in values.values_of("files").unwrap() {
        let mut glob_paths: Vec<PathBuf> = glob(path)?.filter_map(Result::ok).collect();

        paths.append(&mut glob_paths);
    }

    config.deploy_paths(paths)?;
    Ok(())
}

fn deploy(config: Config, values: &ArgMatches) -> Result<()> {
    println!("deploying dotfiles");

    if values.is_present("all") {
        config.deploy_all()?;
        return Ok(());
    };

    let paths: Vec<PathBuf> = values
        .values_of("dotfiles")
        .unwrap()
        .map(PathBuf::from)
        .collect();

    config.deploy_paths(paths)?;
    Ok(())
}

fn restore(config: &mut Config, matches: &ArgMatches) -> Result<()> {
    let dotfiles: Vec<PathBuf> = matches
        .values_of("dotfiles")
        .unwrap()
        .map(PathBuf::from)
        .collect();

    for dotfile in dotfiles.into_iter() {
        let dotfile = paths::get_absolute(dotfile)?;
        config.restore_dotfile(dotfile)?;
    }

    Ok(())
}
