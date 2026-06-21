use std::path::PathBuf;

use lingmo_core_engine::types::DesktopEnvironment;

/// A plugin customizes the build by injecting packages, filesystem overlays,
/// and chroot configuration hooks into the build pipeline.
///
/// Plugins are discovered by name via configuration and loaded from the
/// built-in plugin registry. Each plugin declares its dependencies on other
/// plugins, allowing the pipeline to resolve a correct ordering.
pub trait Plugin: Send + Sync {
    /// Unique name identifying this plugin (e.g. "kde", "nvidia", "networkmanager").
    fn name(&self) -> &'static str;

    /// Human-readable description of what this plugin does.
    fn description(&self) -> &'static str {
        ""
    }

    /// Other plugins this plugin depends on. Dependencies are guaranteed
    /// to be applied before this plugin.
    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Packages to install when this plugin is active.
    /// These are merged into the package installation stage.
    fn packages(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Desktop environments this plugin is compatible with.
    /// Return `None` if compatible with all environments.
    fn supported_desktops(&self) -> Option<Vec<DesktopEnvironment>> {
        None
    }

    /// Filesystem overlays to copy into the rootfs.
    /// Paths are relative to the plugin's overlay directory, or absolute
    /// paths on the host system that should be copied into the rootfs
    /// preserving their relative path.
    ///
    /// Example: `"/etc/NetworkManager/conf.d/"` would be placed at
    /// `<rootfs>/etc/NetworkManager/conf.d/`.
    fn overlays(&self) -> Vec<PathBuf> {
        vec![]
    }

    /// Shell scripts to execute inside the chroot during the hooks stage.
    /// Each string is a complete shell script that will be written to a
    /// temporary file and executed via `bash /tmp/<hook>.sh` inside chroot.
    fn chroot_hooks(&self) -> Vec<String> {
        vec![]
    }

    /// Priority controls ordering among plugins at the same dependency level.
    /// Higher priority plugins run first. Default is 0.
    fn priority(&self) -> i32 {
        0
    }

    // -----------------------------------------------------------------------
    // Volume-aware splitting
    // -----------------------------------------------------------------------

    /// Volume group this plugin's files should be assigned to.
    ///
    /// When using `VolumeStrategy::Plugin`, files installed by this plugin
    /// (via `packages()`, `overlays()`, and `chroot_hooks()`) are placed
    /// into a separate SquashFS volume identified by this label.
    ///
    /// Return `None` (default) to place files in the core/base volume.
    fn volume_group(&self) -> Option<&'static str> {
        None
    }

    /// Directory prefixes that identify files belonging to this plugin's
    /// volume group. When a file's relative path starts with any of these
    /// prefixes, it is assigned to this plugin's volume.
    ///
    /// Example: `["usr/share/kde", "etc/sddm"]` for a KDE plugin.
    fn volume_prefixes(&self) -> Vec<&'static str> {
        vec![]
    }

    /// List of other volume groups that must be mounted before this
    /// plugin's volume. Defaults to `["core"]` which means "volume 1".
    fn required_volumes(&self) -> Vec<&'static str> {
        vec!["core"]
    }
}
