use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use lingmo_core_engine::error::{BuildError, BuildResult};
use lingmo_core_engine::{calculate_sha256, run_command};

use crate::catalog::VolumeSplit;
use crate::manifest::{ManifestConfigRef, VolumeManifest};
use crate::strategy::VolumeConfig;

/// Result of building a single SquashFS volume.
#[derive(Debug, Clone)]
pub struct VolumeBuildResult {
    /// Output filename (e.g. "filesystem.part1.squashfs").
    pub filename: String,

    /// Absolute or relative path to the output file.
    pub path: PathBuf,

    /// Compressed size in bytes.
    pub compressed_size: u64,

    /// SHA-256 hash of the compressed file.
    pub sha256: String,

    /// Volume index (1-based).
    pub index: usize,
}

/// Result of building all volumes.
#[derive(Debug, Clone)]
pub struct MultiVolumeResult {
    pub results: Vec<VolumeBuildResult>,
    pub manifest: VolumeManifest,
    pub manifest_path: PathBuf,
}

/// Build all SquashFS volumes from a volume split.
///
/// For each volume:
/// 1. Create a temporary staging directory
/// 2. Populate it with hard links to the source files (preserving directory structure)
/// 3. Run mksquashfs on the staging directory
/// 4. Delete the staging directory
/// 5. Compute SHA-256 checksum
pub fn build_volumes(
    split: &VolumeSplit,
    config: &VolumeConfig,
    output_dir: &Path,
    rootfs: &Path,
    manifest_config: ManifestConfigRef,
) -> BuildResult<MultiVolumeResult> {
    std::fs::create_dir_all(output_dir).map_err(|e| BuildError::Io {
        path: output_dir.to_path_buf(),
        source: e,
    })?;

    let mut results: Vec<VolumeBuildResult> = Vec::new();
    let mut volume_files: Vec<(String, u64, String)> = Vec::new();

    for vol in &split.volumes {
        let volume_filename = config.volume_filename(vol.index);
        let output_path = output_dir.join(&volume_filename);

        tracing::info!(
            "Building volume {} ('{}'): {} files, {} uncompressed",
            vol.index,
            vol.label,
            vol.entries.len(),
            crate::catalog::humansize(vol.total_size),
        );

        // Create a temporary staging directory with hard links
        let staging_dir = output_dir.join(format!(".staging-vol{}", vol.index));
        if staging_dir.exists() {
            std::fs::remove_dir_all(&staging_dir).map_err(|e| BuildError::Io {
                path: staging_dir.clone(),
                source: e,
            })?;
        }

        // Populate staging directory with hard links
        populate_staging(&staging_dir, rootfs, &vol.entries)?;

        // Build squashfs from staging
        let block_size_str = config.block_size.to_string();
        run_command(
            "mksquashfs",
            &[
                staging_dir.to_str().unwrap(),
                output_path.to_str().unwrap(),
                "-comp",
                &config.compression,
                "-b",
                &block_size_str,
                "-noappend",
                "-no-xattrs",
                "-no-exports",
                "-all-root",
            ],
            &format!(
                "Creating SquashFS volume {} ({})",
                vol.index,
                volume_filename
            ),
        )?;

        // Remove staging
        std::fs::remove_dir_all(&staging_dir).map_err(|e| BuildError::Io {
            path: staging_dir,
            source: e,
        })?;

        // Get compressed size and checksum
        let metadata = std::fs::metadata(&output_path).map_err(|e| BuildError::Io {
            path: output_path.clone(),
            source: e,
        })?;
        let compressed_size = metadata.len();

        let sha256 = if config.verify_checksums {
            let hash = calculate_sha256(&output_path)?;
            tracing::debug!("  SHA256 ({}) = {}", volume_filename, hash);
            hash
        } else {
            "unverified".into()
        };

        volume_files.push((volume_filename.clone(), compressed_size, sha256.clone()));

        results.push(VolumeBuildResult {
            filename: volume_filename.clone(),
            path: output_path,
            compressed_size,
            sha256: sha256.clone(),
            index: vol.index,
        });

        tracing::info!(
            "  → {} ({} compressed, ratio {:.1}%)",
            volume_filename,
            crate::catalog::humansize(compressed_size),
            if vol.total_size > 0 {
                (compressed_size as f64 / vol.total_size as f64) * 100.0
            } else {
                0.0
            },
        );
    }

    // Generate manifest
    let manifest = VolumeManifest::new(split, &volume_files, manifest_config);
    let manifest_path = output_dir.join(config.manifest_filename());

    if config.generate_manifest {
        manifest.write_to(&manifest_path)?;
    }

    tracing::info!(
        "All {} volumes built successfully",
        results.len()
    );

    Ok(MultiVolumeResult {
        results,
        manifest,
        manifest_path,
    })
}

/// Populate a staging directory with hard links to source files.
///
/// This creates the same directory structure as the rootfs but populated with
/// hard links instead of copies, making the operation very fast (metadata-only,
/// no data copy).
fn populate_staging(
    staging_dir: &Path,
    _rootfs: &Path,
    entries: &[crate::catalog::FileEntry],
) -> BuildResult<()> {
    for entry in entries {
        let dest = staging_dir.join(&entry.relative);

        if entry.is_dir {
            std::fs::create_dir_all(&dest).map_err(|e| BuildError::Io {
                path: dest,
                source: e,
            })?;
        } else if entry.is_symlink {
            // Read the symlink target and recreate it
            let target = std::fs::read_link(&entry.source).map_err(|e| BuildError::Io {
                path: entry.source.clone(),
                source: e,
            })?;
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| BuildError::Io {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
            symlink(&target, &dest).map_err(|e| BuildError::Io {
                path: dest,
                source: e,
            })?;
        } else if entry.is_file {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| BuildError::Io {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
            // Create a hard link (fast, no data copy)
            let hardlink_result = std::fs::hard_link(&entry.source, &dest);
            if let Err(e) = hardlink_result {
                // Fallback: copy if hard link fails (e.g. cross-device)
                if e.kind() == std::io::ErrorKind::CrossesDevices {
                    std::fs::copy(&entry.source, &dest).map_err(|e2| BuildError::Io {
                        path: dest,
                        source: e2,
                    })?;
                } else {
                    return Err(BuildError::Io {
                        path: dest,
                        source: e,
                    });
                }
            };
        }
    }

    Ok(())
}
