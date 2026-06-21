use std::path::PathBuf;

use clap::Args;

use lingmo_core_engine::error::BuildResult;
use lingmo_core_engine::{logging, BuildError};

#[derive(Args, Debug)]
pub struct CleanArgs {
    /// Path to build configuration file
    #[arg(short, long, default_value = "lingmo.toml")]
    pub config: PathBuf,

    /// Work directory to clean (overrides config)
    #[arg(short, long)]
    pub work_dir: Option<PathBuf>,

    /// Output directory to clean
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Remove everything including output ISO
    #[arg(long)]
    pub all: bool,
}

pub fn execute(args: &CleanArgs) -> BuildResult<()> {
    logging::init_logging(args.verbose, false);

    if let Some(ref work_dir) = args.work_dir {
        clean_dir(work_dir, "work")?;
    } else if args.config.exists() {
        let config = lingmo_config_parser::parse_toml_file(&args.config)?;
        clean_dir(&config.output.work_dir, "work")?;
        if args.all {
            clean_dir(&config.output.output_dir, "output")?;
        }
    } else {
        // Default clean
        clean_dir(&PathBuf::from("./work"), "work")?;
        if args.all {
            clean_dir(&PathBuf::from("./output"), "output")?;
        }
    }

    Ok(())
}

fn clean_dir(dir: &PathBuf, label: &str) -> BuildResult<()> {
    if !dir.exists() {
        tracing::info!("{} directory does not exist: {}", label, dir.display());
        return Ok(());
    }

    tracing::info!("Cleaning {} directory: {}", label, dir.display());
    std::fs::remove_dir_all(dir).map_err(|e| BuildError::Io {
        path: dir.clone(),
        source: e,
    })?;

    tracing::info!("{} directory cleaned", label);
    Ok(())
}
