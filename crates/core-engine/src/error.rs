use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Stage {stage} failed: {detail}")]
    StageFailed { stage: String, detail: String },

    #[error("Command failed: {command}\n  exit code: {exit_code}\n  stderr: {stderr}")]
    CommandFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },

    #[error("Debootstrap failed: {0}")]
    Debootstrap(String),

    #[error("Plugin error: {plugin}: {detail}")]
    Plugin { plugin: String, detail: String },

    #[error("Chroot operation failed: {0}")]
    Chroot(String),

    #[error("ISO generation failed: {0}")]
    IsoGeneration(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not running as root: required for chroot and mount operations")]
    NotRoot,

    #[error("{0}")]
    Other(String),
}

pub type BuildResult<T> = Result<T, BuildError>;
