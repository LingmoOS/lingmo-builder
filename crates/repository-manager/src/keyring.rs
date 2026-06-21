/// Keyring management for APT repositories.
///
/// Downloads, dearmors, and installs GPG keys for third-party
/// repositories under `/etc/apt/keyrings/`.
use std::path::Path;

use lingmo_core_engine::error::{BuildError, BuildResult};
use lingmo_core_engine::run_command_with_input;

/// Describes a repository key that needs to be installed.
#[derive(Debug, Clone)]
pub struct RepositoryKey {
    /// URL to download the key from
    pub url: String,
    /// Destination path inside the target rootfs
    pub dest_path: String,
}

/// Download an ASCII-armored GPG key and convert it to binary keyring
/// format using `gpg --dearmor`.
///
/// This is equivalent to:
/// ```sh
/// curl -fsSL <url> | gpg --dearmor > <dest>
/// ```
pub fn download_and_dearmor_key(
    key: &RepositoryKey,
    rootfs: &Path,
) -> BuildResult<()> {
    let dest = if key.dest_path.starts_with('/') {
        rootfs.join(&key.dest_path[1..])
    } else {
        rootfs.join(&key.dest_path)
    };

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| BuildError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    tracing::info!(
        "Downloading GPG key from {} to {}",
        key.url,
        dest.display()
    );

    // Download and dearmor in one pipeline
    let armored = download_key(&key.url)?;

    run_command_with_input(
        "gpg",
        &["--dearmor", "--output", dest.to_str().unwrap()],
        &armored,
        &format!("Dearchiving GPG key to {}", dest.display()),
    )?;

    if !dest.exists() {
        return Err(BuildError::Other(format!(
            "Key file was not created at {}",
            dest.display()
        )));
    }

    tracing::info!("Key installed at {}", dest.display());
    Ok(())
}

/// Download a GPG key from a URL and return its content as a string.
fn download_key(url: &str) -> BuildResult<String> {
    let output = std::process::Command::new("curl")
        .args(["-fsSL", url])
        .output()
        .map_err(|e| BuildError::CommandFailed {
            command: format!("curl -fsSL {}", url),
            exit_code: -1,
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BuildError::CommandFailed {
            command: format!("curl -fsSL {}", url),
            exit_code: output.status.code().unwrap_or(-1),
            stderr: stderr.to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Verify that a key file exists and is a valid GPG keyring.
pub fn verify_keyring(path: &Path) -> BuildResult<()> {
    if !path.exists() {
        return Err(BuildError::Other(format!(
            "Key file not found: {}",
            path.display()
        )));
    }

    let output = std::process::Command::new("gpg")
        .args(["--quiet", "--no-default-keyring", "--keyring", path.to_str().unwrap(), "--list-keys"])
        .output()
        .map_err(|e| BuildError::CommandFailed {
            command: format!("gpg --list-keys {}", path.display()),
            exit_code: -1,
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BuildError::Other(format!(
            "Invalid GPG keyring at {}: {}",
            path.display(),
            stderr.trim()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_key_struct() {
        let key = RepositoryKey {
            url: "https://example.com/key.gpg".into(),
            dest_path: "/etc/apt/keyrings/example.gpg".into(),
        };
        assert_eq!(key.url, "https://example.com/key.gpg");
        assert_eq!(key.dest_path, "/etc/apt/keyrings/example.gpg");
    }
}
