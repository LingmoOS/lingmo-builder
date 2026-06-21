pub mod model;
pub mod validate;

use lingmo_core_engine::error::{BuildError, BuildResult};
use crate::model::BuildConfig;

/// Parse a TOML build configuration from a file path.
pub fn parse_toml_file(path: impl AsRef<std::path::Path>) -> BuildResult<BuildConfig> {
    let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
        BuildError::Config(format!(
            "Failed to read config file '{}': {}",
            path.as_ref().display(),
            e
        ))
    })?;
    parse_toml(&content)
}

/// Parse a TOML build configuration from a string.
pub fn parse_toml(content: &str) -> BuildResult<BuildConfig> {
    let config: BuildConfig = toml::from_str(content).map_err(|e| {
        BuildError::Config(format!("Invalid TOML configuration: {}", e))
    })?;
    Ok(config)
}
