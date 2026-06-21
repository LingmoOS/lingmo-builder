use std::path::PathBuf;

use clap::Args;

use lingmo_core_engine::error::BuildResult;
use lingmo_core_engine::logging;

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Path to build configuration file
    #[arg(short, long, default_value = "lingmo.toml")]
    pub config: PathBuf,

    /// Verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

pub fn execute(args: &ValidateArgs) -> BuildResult<()> {
    logging::init_logging(args.verbose, false);

    if !args.config.exists() {
        tracing::error!("Config file not found: {}", args.config.display());
        return Err(lingmo_core_engine::BuildError::Config(format!(
            "Config file not found: {}",
            args.config.display()
        )));
    }

    let config = match lingmo_config_parser::parse_toml_file(&args.config) {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!("Config parse error: {}", e);
            return Err(e);
        }
    };

    match config.validate() {
        Ok(warnings) => {
            if warnings.is_empty() {
                tracing::info!("Configuration is valid");
            } else {
                tracing::info!("Configuration is valid with {} warning(s):", warnings.len());
                for w in &warnings {
                    tracing::warn!("  {}", w);
                }
            }
        }
        Err(e) => {
            tracing::error!("Configuration validation failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
