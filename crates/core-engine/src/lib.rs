pub mod error;
pub mod logging;
pub mod types;

pub use error::{BuildError, BuildResult};
pub use types::*;

use std::path::Path;
use std::process::Command;

pub fn ensure_root() -> BuildResult<()> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|_| BuildError::Other("Failed to check user ID".into()))?;
    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uid != "0" {
        return Err(BuildError::NotRoot);
    }
    Ok(())
}

pub fn run_command(cmd: &str, args: &[&str], description: &str) -> BuildResult<String> {
    tracing::info!("{}", description);
    tracing::debug!("  running: {} {}", cmd, args.join(" "));

    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| BuildError::CommandFailed {
            command: format!("{} {}", cmd, args.join(" ")),
            exit_code: -1,
            stderr: e.to_string(),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(BuildError::CommandFailed {
            command: format!("{} {}", cmd, args.join(" ")),
            exit_code: output.status.code().unwrap_or(-1),
            stderr,
        });
    }

    if !stderr.is_empty() {
        tracing::warn!("  stderr: {}", stderr.trim());
    }

    Ok(stdout)
}

pub fn run_command_with_input(
    cmd: &str,
    args: &[&str],
    stdin_data: &str,
    description: &str,
) -> BuildResult<String> {
    tracing::info!("{}", description);
    tracing::debug!("  running: {} {} (stdin)", cmd, args.join(" "));

    let mut child = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| BuildError::CommandFailed {
            command: format!("{} {}", cmd, args.join(" ")),
            exit_code: -1,
            stderr: e.to_string(),
        })?;

    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_data.as_bytes()).map_err(|e| {
            BuildError::CommandFailed {
                command: format!("{} {}", cmd, args.join(" ")),
                exit_code: -1,
                stderr: e.to_string(),
            }
        })?;
    }

    let output = child.wait_with_output().map_err(|e| BuildError::CommandFailed {
        command: format!("{} {}", cmd, args.join(" ")),
        exit_code: -1,
        stderr: e.to_string(),
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(BuildError::CommandFailed {
            command: format!("{} {}", cmd, args.join(" ")),
            exit_code: output.status.code().unwrap_or(-1),
            stderr,
        });
    }

    if !stderr.is_empty() {
        tracing::warn!("  stderr: {}", stderr.trim());
    }

    Ok(stdout)
}

pub fn copy_recursive(src: &Path, dst: &Path) -> BuildResult<()> {
    tracing::debug!("copying {} -> {}", src.display(), dst.display());
    if src.is_dir() {
        std::fs::create_dir_all(dst)
            .map_err(|e| BuildError::Io {
                path: dst.to_path_buf(),
                source: e,
            })?;
        for entry in std::fs::read_dir(src).map_err(|e| BuildError::Io {
            path: src.to_path_buf(),
            source: e,
        })? {
            let entry = entry.map_err(|e| BuildError::Io {
                path: src.to_path_buf(),
                source: e,
            })?;
            let file_type = entry.file_type().map_err(|e| BuildError::Io {
                path: entry.path(),
                source: e,
            })?;
            if file_type.is_dir() {
                copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
            } else {
                std::fs::copy(entry.path(), dst.join(entry.file_name())).map_err(|e| {
                    BuildError::Io {
                        path: entry.path(),
                        source: e,
                    }
                })?;
            }
        }
    } else {
        std::fs::copy(src, dst).map_err(|e| BuildError::Io {
            path: src.to_path_buf(),
            source: e,
        })?;
    }
    Ok(())
}

pub fn remove_dir_contents(path: &Path) -> BuildResult<()> {
    if !path.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(path).map_err(|e| BuildError::Io {
        path: path.to_path_buf(),
        source: e,
    })? {
        let entry = entry.map_err(|e| BuildError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        let path = entry.path();
        if entry.file_type().map_err(|e| BuildError::Io {
            path: path.clone(),
            source: e,
        })?.
        is_dir()
        {
            std::fs::remove_dir_all(&path).map_err(|e| BuildError::Io {
                path,
                source: e,
            })?;
        } else {
            std::fs::remove_file(&path).map_err(|e| BuildError::Io {
                path,
                source: e,
            })?;
        }
    }
    Ok(())
}

pub fn write_file(path: &Path, content: &str) -> BuildResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| BuildError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    std::fs::write(path, content).map_err(|e| BuildError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

pub fn create_symlink(target: &Path, link: &Path) -> BuildResult<()> {
    if link.exists() {
        std::fs::remove_file(link).map_err(|e| BuildError::Io {
            path: link.to_path_buf(),
            source: e,
        })?;
    }
    std::os::unix::fs::symlink(target, link).map_err(|e| BuildError::Io {
        path: link.to_path_buf(),
        source: e,
    })
}

pub fn calculate_sha256(path: &Path) -> BuildResult<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path).map_err(|e| BuildError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer).map_err(|e| BuildError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
