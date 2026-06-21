use std::path::PathBuf;

use clap::Args;

use lingmo_core_engine::logging;
use lingmo_core_engine::{ensure_root, BuildError, BuildResult};
use lingmo_pipeline::pipeline::Pipeline;
use lingmo_pipeline::stage::Stage;
use lingmo_plugins::registry::PluginRegistry;

#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Path to build configuration file
    #[arg(short, long, default_value = "lingmo.toml")]
    pub config: PathBuf,

    /// Profile override (desktop, server, core)
    #[arg(short, long)]
    pub profile: Option<String>,

    /// Skip specific stages
    #[arg(long, value_delimiter = ',')]
    pub skip_stages: Vec<String>,

    /// Run only specific stages
    #[arg(long, value_delimiter = ',')]
    pub only_stages: Vec<String>,

    /// Override output directory
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Override work directory
    #[arg(short = 'w', long)]
    pub work_dir: Option<PathBuf>,

    /// Verbose output (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// JSON log output
    #[arg(long)]
    pub json: bool,

    /// Dry run (print what would be done)
    #[arg(long)]
    pub dry_run: bool,

    /// Architecture override
    #[arg(long)]
    pub arch: Option<String>,

    /// Do not require root
    #[arg(long)]
    pub no_root_check: bool,
}

pub fn execute(args: &BuildArgs) -> BuildResult<()> {
    logging::init_logging(args.verbose, args.json);

    tracing::info!("Lingmo Builder v{}", env!("CARGO_PKG_VERSION"));

    // Parse config
    if !args.config.exists() {
        return Err(BuildError::Config(format!(
            "Config file not found: {}",
            args.config.display()
        )));
    }

    let config = lingmo_config_parser::parse_toml_file(&args.config)?;

    // Validate config
    let warnings = config.validate()?;
    for warning in &warnings {
        tracing::warn!("Configuration warning: {}", warning);
    }

    // Override from CLI
    let mut config = config;
    if let Some(ref profile) = args.profile {
        config.profile = profile.clone();
    }
    if let Some(ref output_dir) = args.output_dir {
        config.output.output_dir = output_dir.clone();
    }
    if let Some(ref work_dir) = args.work_dir {
        config.output.work_dir = work_dir.clone();
    }
    if let Some(ref arch) = args.arch {
        config.distro.architecture = arch.clone();
    }

    let mut ctx = config.to_context()?;
    ctx.verbosity = args.verbose;
    ctx.dry_run = args.dry_run;

    tracing::info!(
        "Building: {} {} ({}) [{}]",
        ctx.distro.name,
        ctx.distro.version,
        ctx.distro.architecture,
        ctx.profile
    );

    if !args.no_root_check {
        ensure_root()?;
    }

    // Resolve plugins
    let registry = PluginRegistry::new();
    if !ctx.plugins.is_empty() {
        let resolved = registry.resolve(&ctx.plugins)?;
        tracing::info!(
            "Resolved {} plugins: {}",
            resolved.len(),
            resolved.iter().map(|p| p.name()).collect::<Vec<_>>().join(", ")
        );
    }

    // Build pipeline
    let mut pipeline = Pipeline::new(ctx, registry);

    // Parse stage filters
    if !args.skip_stages.is_empty() {
        let stages: Vec<Stage> = args
            .skip_stages
            .iter()
            .map(|s| parse_stage(s))
            .collect::<Result<_, _>>()?;
        pipeline = pipeline.with_skip_stages(stages);
    }

    if !args.only_stages.is_empty() {
        let stages: Vec<Stage> = args
            .only_stages
            .iter()
            .map(|s| parse_stage(s))
            .collect::<Result<_, _>>()?;
        pipeline = pipeline.with_only_stages(stages);
    }

    if args.dry_run {
        tracing::info!("Dry run mode - no changes will be made");
        return Ok(());
    }

    pipeline.execute()?;

    tracing::info!("Build complete!");
    Ok(())
}

fn parse_stage(s: &str) -> Result<Stage, BuildError> {
    match s.to_lowercase().as_str() {
        "init" => Ok(Stage::Init),
        "bootstrap" => Ok(Stage::Bootstrap),
        "configure-apt" | "configure-debian-repos" => Ok(Stage::ConfigureDebianRepos),
        "configure-extra-repos" => Ok(Stage::ConfigureExtraRepos),
        "install-base" => Ok(Stage::InstallBase),
        "install-kernel" => Ok(Stage::InstallKernel),
        "install-firmware" => Ok(Stage::InstallFirmware),
        "apply-profile" => Ok(Stage::ApplyProfile),
        "install-desktop" => Ok(Stage::InstallDesktop),
        "additional-packages" => Ok(Stage::AdditionalPackages),
        "filesystem-overlays" => Ok(Stage::FilesystemOverlays),
        "chroot-hooks" => Ok(Stage::ChrootHooks),
        "system-config" => Ok(Stage::SystemConfig),
        "install-bootloader" => Ok(Stage::InstallBootloader),
        "generate-squashfs" | "generate-squashfs-volumes" => {
            Ok(Stage::GenerateSquashfsVolumes)
        }
        "generate-iso" => Ok(Stage::GenerateIso),
        "cleanup" => Ok(Stage::Cleanup),
        _ => Err(BuildError::Config(format!(
            "Unknown stage '{}'. Valid stages: init, bootstrap, configure-debian-repos, \
             configure-extra-repos, install-base, install-kernel, install-firmware, \
             apply-profile, install-desktop, additional-packages, filesystem-overlays, \
             chroot-hooks, system-config, install-bootloader, generate-squashfs-volumes, \
             generate-iso, cleanup",
            s
        ))),
    }
}
