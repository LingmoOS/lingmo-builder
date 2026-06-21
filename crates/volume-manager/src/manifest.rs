use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use lingmo_core_engine::error::{BuildError, BuildResult};
use lingmo_core_engine::calculate_sha256;

use crate::catalog::VolumeSplit;

/// The top-level manifest structure stored as `filesystem.manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeManifest {
    /// Format version identifier for forward compatibility.
    pub format: String,

    /// Total size of the full rootfs (uncompressed) across all volumes.
    pub total_uncompressed_size: u64,

    /// Number of volumes in the set.
    pub volume_count: usize,

    /// Ordered list of volumes (mount order: index 1 = lowest/earliest).
    pub volumes: Vec<VolumeInfo>,

    /// Dependency map: volume label → list of volume labels it depends on.
    /// This is used to determine correct overlay mount ordering.
    pub dependencies: BTreeMap<String, Vec<String>>,

    /// Metadata for reconstruction.
    pub metadata: ManifestMetadata,
}

/// Information about a single SquashFS volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    /// Volume name / filename (e.g. "filesystem.part1.squashfs").
    pub name: String,

    /// Human-readable label (e.g. "core", "kde", "drivers").
    pub label: String,

    /// 1-based index in mount order (lower = mounted first / lower in overlay stack).
    pub index: usize,

    /// Size of the compressed SquashFS file in bytes.
    pub compressed_size: u64,

    /// Size of the uncompressed data in bytes.
    pub uncompressed_size: u64,

    /// Number of files (excluding directories) in this volume.
    pub file_count: usize,

    /// SHA-256 hash of the SquashFS file.
    pub sha256: String,

    /// Optional list of file paths included in this volume (for debugging).
    /// This is omitted in production manifests by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_list: Option<Vec<String>>,
}

/// Metadata about the build that produced this manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMetadata {
    /// Builder version.
    pub builder: String,

    /// Timestamp of build (ISO 8601).
    pub build_timestamp: String,

    /// Build configuration summary.
    pub config: ManifestConfigRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestConfigRef {
    pub profile: String,
    pub desktop: Option<String>,
    pub architecture: String,
    pub compression: String,
    pub block_size: u64,
}

impl VolumeManifest {
    /// Create a new manifest from a completed volume split and build results.
    pub fn new(
        split: &VolumeSplit,
        volume_files: &[(String, u64, String)], // (filename, compressed_size, sha256)
        config_ref: ManifestConfigRef,
    ) -> Self {
        let total_uncompressed: u64 = split.volumes.iter().map(|v| v.total_size).sum();
        let volume_count = split.volumes.len();

        let mut volumes: Vec<VolumeInfo> = Vec::new();
        let mut dependencies: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for (i, vol) in split.volumes.iter().enumerate() {
            // Find matching build result
            let (compressed_size, sha256) = volume_files
                .get(i)
                .map(|(_, s, h)| (*s, h.clone()))
                .unwrap_or((0, "unverified".into()));

            let vol_info = VolumeInfo {
                name: volume_files
                    .get(i)
                    .map(|(name, _, _)| name.clone())
                    .unwrap_or_else(|| format!("filesystem.part{}.squashfs", vol.index)),
                label: vol.label.clone(),
                index: vol.index,
                compressed_size,
                uncompressed_size: vol.total_size,
                file_count: vol.entries.iter().filter(|e| e.is_file).count(),
                sha256,
                file_list: None,
            };
            volumes.push(vol_info);

            // Build dependency graph: volume N depends on all earlier volumes
            if i > 0 {
                let deps: Vec<String> = split.volumes[..i]
                    .iter()
                    .map(|v| v.label.clone())
                    .collect();
                dependencies.insert(vol.label.clone(), deps);
            } else {
                dependencies.insert(vol.label.clone(), vec![]);
            }
        }

        VolumeManifest {
            format: "multi-squashfs-v1".into(),
            total_uncompressed_size: total_uncompressed,
            volume_count,
            volumes,
            dependencies,
            metadata: ManifestMetadata {
                builder: env!("CARGO_PKG_NAME").to_string(),
                build_timestamp: chrono::Utc::now().to_rfc3339(),
                config: config_ref,
            },
        }
    }

    /// Write the manifest as pretty-printed JSON to the specified path.
    pub fn write_to(&self, path: &Path) -> BuildResult<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            BuildError::Other(format!("Failed to serialize manifest: {}", e))
        })?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| BuildError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        std::fs::write(path, &json).map_err(|e| BuildError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        tracing::info!("Manifest written to {}", path.display());
        Ok(())
    }

    /// Load a manifest from a JSON file.
    pub fn from_file(path: &Path) -> BuildResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| BuildError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let manifest: VolumeManifest = serde_json::from_str(&content).map_err(|e| {
            BuildError::Other(format!(
                "Failed to parse manifest '{}': {}",
                path.display(),
                e
            ))
        })?;

        Ok(manifest)
    }

    /// Verify checksums of all volumes against the manifest.
    pub fn verify(&self, directory: &Path) -> BuildResult<()> {
        tracing::info!("Verifying {} volume checksums...", self.volume_count);

        for vol in &self.volumes {
            let vol_path = directory.join(&vol.name);
            if !vol_path.exists() {
                return Err(BuildError::Other(format!(
                    "Volume file not found: {}",
                    vol_path.display()
                )));
            }

            let actual_hash = calculate_sha256(&vol_path)?;
            if actual_hash != vol.sha256 {
                return Err(BuildError::Other(format!(
                    "Checksum mismatch for '{}': expected {}, got {}",
                    vol.name, vol.sha256, actual_hash
                )));
            }

            tracing::debug!("  ✓ {} SHA256 verified", vol.name);
        }

        tracing::info!("All {} volume checksums verified", self.volume_count);
        Ok(())
    }
}
