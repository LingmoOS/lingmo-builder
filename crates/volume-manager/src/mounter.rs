use lingmo_core_engine::error::{BuildError, BuildResult};

use crate::manifest::VolumeManifest;

/// Verify that volumes are in a valid ordering for Debian live-boot.
///
/// Debian live-boot discovers squashfs files under `/live/` and assembles
/// them into a union filesystem automatically via overlayfs integration
/// in the initramfs. This function validates that:
///
/// - Volume indices are sequential starting from 1 (mount order)
/// - Dependencies reference only volumes that exist
/// - The ordering is consistent
///
/// No mount scripts or systemd units are generated — the boot-time
/// assembly is handled entirely by live-boot + initramfs-tools.
pub fn verify_volume_ordering(manifest: &VolumeManifest) -> BuildResult<()> {
    if manifest.volumes.is_empty() {
        return Err(BuildError::Other(
            "Cannot verify ordering: no volumes in manifest".into(),
        ));
    }

    // Check indices are sequential starting from 1
    let mut expected = 1;
    for vol in &manifest.volumes {
        if vol.index != expected {
            return Err(BuildError::Other(format!(
                "Volume index gap: expected {}, got {} for '{}'",
                expected, vol.index, vol.name
            )));
        }
        expected += 1;
    }

    // Check dependencies are consistent
    for vol in &manifest.volumes {
        if let Some(deps) = manifest.dependencies.get(&vol.label) {
            for dep in deps {
                if !manifest.volumes.iter().any(|v| v.label == *dep) {
                    return Err(BuildError::Other(format!(
                        "Volume '{}' depends on '{}' which is not in the manifest",
                        vol.label, dep
                    )));
                }
            }
        }
    }

    tracing::info!(
        "Volume ordering verified: {} volumes (index 1..{})",
        manifest.volume_count,
        manifest.volume_count
    );

    Ok(())
}
