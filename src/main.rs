use glob::glob;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{crate_authors, crate_description, crate_name};
use clap::{App, AppSettings, Arg, ArgMatches};

use ghmd::paths;
use ghmd::Config;

fn main() -> Result<()> {
    let stow_subcommand = App::new("stow")
        .about(
            "store input files in the specified dotfiles directory, and replace the file's \
            original path with a symlink",
        )
        .display_order(2)
        .arg(
            Arg::with_name("dotfile_dir")
                .help("path of the dotfiles directory")
                .required(true)
                .multiple(false),
        )
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
        .about("restore specified dotfiles to their original locations")
        .display_order(4)
        .arg(
            Arg::with_name("dotfiles")
                .help("the dotfiles to restore to original locations")
                .multiple(true)
                .required(true),
        );

    let matches = App::new(crate_name!())
        .setting(AppSettings::ArgRequiredElseHelp)
        .about(crate_description!())
        .author(crate_authors!())
        .after_help("https://github.com/jakeschurch/badm")
        .subcommands(vec![stow_subcommand, deploy_subcommand, restore_subcommand])
        .get_matches();

    let mut config = Config::load()?;

    match matches.subcommand() {
        Some(("stow", stow_matches)) => stow(config, stow_matches)?,
        Some(("deploy", deploy_matches)) => deploy(config, deploy_matches)?,
        Some(("restore", restore_matches)) => restore(&mut config, restore_matches)?,
        Some((s, _)) => return Err(anyhow!("invalid subcommand: {0}", s)),
        None => return Err(anyhow!("missing subcommand")),
    }
    Ok(())
}

fn stow(config: Config, matches: &ArgMatches) -> Result<()> {
    let mut paths = vec![];

    let dotfiles_dir: PathBuf = matches
        .get_one::<PathBuf>("dotfiles_dir")
        .ok_or(anyhow!("must include dotfiles_dir argument"))?
        .into();

    for path in matches.values_of("files").unwrap() {
        let mut glob_paths: Vec<PathBuf> = glob(path)?.filter_map(Result::ok).collect();

        paths.append(&mut glob_paths);
    }

    config.stow(dotfiles_dir, paths)?;
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
