use clap::Args;

use lingmo_core_engine::error::BuildResult;
use lingmo_core_engine::logging;
use lingmo_plugins::registry::PluginRegistry;

#[derive(Args, Debug)]
pub struct ListPluginsArgs {
    /// Verbose output with package details
    #[arg(short, long)]
    pub verbose: bool,
}

pub fn execute(args: &ListPluginsArgs) -> BuildResult<()> {
    logging::init_logging(if args.verbose { 2 } else { 1 }, false);

    let registry = PluginRegistry::new();
    let plugins = registry.list_plugins();

    println!("Available plugins ({}):", plugins.len());
    println!();
    for plugin in &plugins {
        println!("  {}:", plugin.name);
        if !plugin.description.is_empty() {
            println!("    Description: {}", plugin.description);
        }
        if !plugin.dependencies.is_empty() {
            println!("    Dependencies: {}", plugin.dependencies.join(", "));
        }
        println!("    Packages: {}", plugin.package_count);

        if args.verbose {
            if let Some(p) = registry.get(&plugin.name) {
                println!("    Supported desktops: {:?}", p.supported_desktops());
                if let Some(vg) = p.volume_group() {
                    println!("    Volume group: {}", vg);
                }
                let prefixes = p.volume_prefixes();
                if !prefixes.is_empty() {
                    println!("    Volume prefixes: {}", prefixes.join(", "));
                }
                let required = p.required_volumes();
                if !required.is_empty() {
                    println!("    Required volumes: {}", required.join(", "));
                }
            }
        }
        println!();
    }

    Ok(())
}
