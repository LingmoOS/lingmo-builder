use clap::{Parser, Subcommand};

use crate::commands;

#[derive(Parser, Debug)]
#[command(
    name = "lingmo-builder",
    version,
    about = "Production-grade Debian-based distribution builder",
    long_about = "Lingmo Builder - A modular, multi-stage Linux distribution builder\n\
                   Built with Rust | Pipeline-based | Plugin-extensible\n\n\
                   Build bootable Debian-based ISO images from declarative TOML configuration."
)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Build a distribution ISO from a configuration file
    Build(commands::build::BuildArgs),

    /// Clean build artifacts (work directory)
    Clean(commands::clean::CleanArgs),

    /// Generate a configuration template
    Init(commands::init::InitArgs),

    /// List all available build plugins
    ListPlugins(commands::list_plugins::ListPluginsArgs),

    /// Validate a configuration file
    Validate(commands::validate::ValidateArgs),
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build(args) => commands::build::execute(&args)?,
        Commands::Clean(args) => commands::clean::execute(&args)?,
        Commands::Init(args) => commands::init::execute(&args)?,
        Commands::ListPlugins(args) => commands::list_plugins::execute(&args)?,
        Commands::Validate(args) => commands::validate::execute(&args)?,
    }

    Ok(())
}
