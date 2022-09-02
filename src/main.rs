use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{crate_authors, crate_description, crate_name};
use clap::{App, AppSettings, Arg, ArgMatches};
use glob::glob;
use log;
use pretty_env_logger;

use ghmd::Config;
use ghmd::{DotfilePath, DotfilesDir, SymlinkDir};

fn main() -> Result<()> {
    let stow_subcommand = App::new("stow")
        .about(
            "store input files in the specified dotfiles directory, and replace the file's \
            original path with a symlink",
        )
        .display_order(2)
        .arg(
            Arg::with_name("symlink_dir")
                .help("path relative to which symlink directory")
                .required(true)
                .multiple(false),
        )
        .arg(
            Arg::with_name("dotfiles_dir")
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
            Arg::with_name("dotfiles_dir")
                .help("path of the dotfiles directory")
                .required(true)
                .multiple(false),
        )
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
        .after_help("https://github.com/waynr/ghmd")
        .arg(
            Arg::with_name("verbose")
                .short('v')
                .help("path of the dotfiles directory")
                .action(clap::ArgAction::Count),
        )
        .subcommands(vec![stow_subcommand, deploy_subcommand, restore_subcommand])
        .get_matches();

    let verbosity = matches.get_one::<u8>("verbose").copied();

    let mut logger_builder = &mut pretty_env_logger::formatted_builder();

    let level = match verbosity {
        Some(0) => log::LevelFilter::Info,
        Some(1) => log::LevelFilter::Debug,
        Some(_) => log::LevelFilter::Trace,
        None => log::LevelFilter::Info,
    };

    logger_builder = logger_builder.filter_level(level);
    if level == log::LevelFilter::Info {
        logger_builder = logger_builder.default_format();
        logger_builder = logger_builder.format_module_path(false);
        logger_builder = logger_builder.format_level(false);
        logger_builder = logger_builder.format_timestamp(None);
    }

    logger_builder.try_init()?;
    log::debug!("verbosity set to {0}", level);

    let mut config = Config::load()?;

    match matches.subcommand() {
        Some(("stow", stow_matches)) => stow(&mut config, stow_matches)?,
        Some(("deploy", deploy_matches)) => deploy(&config, deploy_matches)?,
        Some(("restore", restore_matches)) => restore(&mut config, restore_matches)?,
        Some((s, _)) => return Err(anyhow!("invalid subcommand: {0}", s)),
        None => return Err(anyhow!("missing subcommand")),
    }
    Ok(())
}

fn stow(config: &mut Config, matches: &ArgMatches) -> Result<()> {
    let dotfiles_dir: DotfilesDir = matches
        .get_one::<String>("dotfiles_dir")
        .and_then(|s| Some(PathBuf::from(s)))
        .ok_or(anyhow!("must include dotfiles_dir argument"))?
        .try_into()?;

    let symlink_dir: SymlinkDir = matches
        .get_one::<String>("symlink_dir")
        .and_then(|s| Some(PathBuf::from(s)))
        .ok_or(anyhow!("must include symlink_dir argument"))?
        .try_into()?;

    log::debug!("dotfiles_dir: {:?}", dotfiles_dir);
    log::debug!("symlink_dir: {:?}", symlink_dir);

    let mut dotfile_paths: Vec<DotfilePath> = Vec::new();
    for glob_path in matches.values_of("files").unwrap() {
        for path in &glob(glob_path)?
            .filter_map(Result::ok)
            .collect::<Vec<PathBuf>>()
        {
            dotfile_paths.push((&symlink_dir, &dotfiles_dir, path).try_into()?);
        }
    }

    config.stow_paths(symlink_dir, dotfiles_dir, dotfile_paths)?;
    Ok(())
}

fn deploy(config: &Config, values: &ArgMatches) -> Result<()> {
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
    let dotfiles_dir: DotfilesDir = matches
        .get_one::<String>("dotfiles_dir")
        .and_then(|s| Some(PathBuf::from(s)))
        .ok_or(anyhow!("must include dotfiles_dir argument"))?
        .try_into()?;

    let dotfiles: Vec<PathBuf> = matches
        .values_of("dotfiles")
        .unwrap()
        .map(PathBuf::from)
        .collect();

    for dotfile in dotfiles.into_iter() {
        let dotfile: DotfilePath = (dotfiles_dir.clone(), dotfile).try_into()?;
        config.restore_dotfile(dotfile)?;
    }

    Ok(())
}
