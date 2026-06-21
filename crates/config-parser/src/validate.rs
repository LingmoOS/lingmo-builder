use std::collections::HashSet;

use lingmo_core_engine::error::{BuildError, BuildResult};
use lingmo_core_engine::types::{
    Architecture, CacheConfig, DesktopEnvironment, DiMode, DistroConfig, Hook, InstallerType,
    OutputConfig, PackageConfig, Profile, SystemConfig, UserConfig,
};

use crate::model::BuildConfig;

impl BuildConfig {
    /// Validate the configuration and return a list of warnings.
    pub fn validate(&self) -> BuildResult<Vec<String>> {
        let mut warnings = Vec::new();

        // Validate profile
        let valid_profiles = ["desktop", "server", "core"];
        if !valid_profiles.contains(&self.profile.to_lowercase().as_str()) {
            return Err(BuildError::Validation(format!(
                "Invalid profile '{}'. Must be one of: {:?}",
                self.profile, valid_profiles
            )));
        }

        // Validate desktop environment
        if let Some(ref de) = self.desktop {
            let valid_desktops = [
                "kde", "gnome", "xfce", "lxqt", "mate", "cinnamon", "budgie", "sway", "hyprland",
            ];
            if !valid_desktops.contains(&de.to_lowercase().as_str()) {
                return Err(BuildError::Validation(format!(
                    "Invalid desktop environment '{}'. Must be one of: {:?}",
                    de, valid_desktops
                )));
            }
            if self.profile.to_lowercase() != "desktop" {
                warnings.push(format!(
                    "Desktop environment '{}' specified but profile is '{}' (not 'desktop')",
                    de, self.profile
                ));
            }
        }

        // Validate installer
        let valid_installers = ["calamares", "debian-installer"];
        if !valid_installers.contains(&self.installer.to_lowercase().as_str()) {
            return Err(BuildError::Validation(format!(
                "Invalid installer '{}'. Must be one of: {:?}",
                self.installer, valid_installers
            )));
        }

        // Validate installer mode
        if let Some(ref mode) = self.installer_mode {
            let valid_modes = ["graphical", "text", "ncurses"];
            if !valid_modes.contains(&mode.to_lowercase().as_str()) {
                return Err(BuildError::Validation(format!(
                    "Invalid installer mode '{}'. Must be one of: {:?}",
                    mode, valid_modes
                )));
            }
        }

        // Validate architecture
        let valid_archs = ["amd64", "i386", "arm64"];
        if !valid_archs.contains(&self.distro.architecture.to_lowercase().as_str()) {
            return Err(BuildError::Validation(format!(
                "Invalid architecture '{}'. Must be one of: {:?}",
                self.distro.architecture, valid_archs
            )));
        }

        // Validate repository format
        let valid_formats = ["deb822", "legacy"];
        if !valid_formats.contains(&self.repositories.default_format.to_lowercase().as_str()) {
            return Err(BuildError::Validation(format!(
                "Invalid repository format '{}'. Must be one of: {:?}",
                self.repositories.default_format, valid_formats
            )));
        }

        // Validate extra repository names are unique
        let mut seen_repos = std::collections::HashSet::new();
        for repo in &self.repositories.extra {
            if repo.name.is_empty() {
                return Err(BuildError::Validation(
                    "Extra repository name cannot be empty".into(),
                ));
            }
            if !seen_repos.insert(&repo.name) {
                return Err(BuildError::Validation(format!(
                    "Duplicate repository name '{}'",
                    repo.name
                )));
            }
            if !repo.url.starts_with("http://") && !repo.url.starts_with("https://") {
                return Err(BuildError::Validation(format!(
                    "Invalid repository URL '{}' for '{}'. Must start with http:// or https://",
                    repo.url, repo.name
                )));
            }
        }

        // Validate volume strategy
        let valid_strategies = ["single", "size", "plugin", "profile"];
        if !valid_strategies.contains(&self.volume.strategy.to_lowercase().as_str()) {
            return Err(BuildError::Validation(format!(
                "Invalid volume strategy '{}'. Must be one of: {:?}",
                self.volume.strategy, valid_strategies
            )));
        }

        // Validate volume max size
        if self.volume.max_volume_size == 0 {
            return Err(BuildError::Validation(
                "Volume max_volume_size must be greater than 0".into(),
            ));
        }

        // Validate volume block size
        let valid_block_sizes = [4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288, 1048576];
        if !valid_block_sizes.contains(&self.volume.block_size) {
            warnings.push(format!(
                "Unusual squashfs block size {} (typical: 4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288, 1048576)",
                self.volume.block_size
            ));
        }

        // Validate mirror URL
        if !self.distro.mirror.starts_with("http://")
            && !self.distro.mirror.starts_with("https://")
        {
            return Err(BuildError::Validation(format!(
                "Invalid mirror URL '{}'. Must start with http:// or https://",
                self.distro.mirror
            )));
        }

        // Validate hostname
        if self.system.hostname.is_empty() {
            return Err(BuildError::Validation("Hostname cannot be empty".into()));
        }

        // Validate users
        let mut seen_users = HashSet::new();
        for user in &self.system.users {
            if user.username.is_empty() {
                return Err(BuildError::Validation("Username cannot be empty".into()));
            }
            if !seen_users.insert(&user.username) {
                return Err(BuildError::Validation(format!(
                    "Duplicate user '{}'",
                    user.username
                )));
            }
        }

        // Validate output directory
        if self.output.output_dir == self.output.work_dir {
            warnings.push("Output and work directories are the same".into());
        }

        // Validate plugins exist (will be resolved by registry at runtime)
        // — we defer full plugin validation to the registry

        // Validate stage references in hooks
        let valid_stages = [
            "init", "bootstrap",
            "configure-debian-repos", "configure-extra-repos",
            "install-base", "install-kernel",
            "install-firmware", "apply-profile", "install-desktop", "additional-packages",
            "filesystem-overlays", "chroot-hooks", "system-config", "install-bootloader",
            "generate-squashfs", "generate-squashfs-volumes", "generate-iso", "cleanup",
        ];
        for hook in &self.hooks {
            if !hook.stage.is_empty()
                && !valid_stages.contains(&hook.stage.to_lowercase().as_str())
            {
                warnings.push(format!(
                    "Hook '{}' references unknown stage '{}'",
                    hook.name, hook.stage
                ));
            }
        }

        Ok(warnings)
    }

    /// Convert the parsed config into a [BuildContext] for pipeline execution.
    pub fn to_context(&self) -> BuildResult<lingmo_core_engine::types::BuildContext> {
        let profile = match self.profile.to_lowercase().as_str() {
            "desktop" => Profile::Desktop,
            "server" => Profile::Server,
            "core" => Profile::Core,
            _ => unreachable!(),
        };

        let desktop = self.desktop.as_ref().map(|d| match d.to_lowercase().as_str() {
            "kde" => DesktopEnvironment::KDE,
            "gnome" => DesktopEnvironment::GNOME,
            "xfce" => DesktopEnvironment::XFCE,
            "lxqt" => DesktopEnvironment::LXQT,
            "mate" => DesktopEnvironment::MATE,
            "cinnamon" => DesktopEnvironment::Cinnamon,
            "budgie" => DesktopEnvironment::Budgie,
            "sway" => DesktopEnvironment::Sway,
            "hyprland" => DesktopEnvironment::Hyprland,
            _ => DesktopEnvironment::None,
        });

        let installer = match self.installer.to_lowercase().as_str() {
            "calamares" => InstallerType::Calamares,
            "debian-installer" => {
                let mode = match self.installer_mode.as_deref() {
                    Some("text") => DiMode::Text,
                    Some("ncurses") => DiMode::Ncurses,
                    _ => DiMode::Graphical,
                };
                InstallerType::DebianInstaller { mode }
            }
            _ => InstallerType::Calamares,
        };

        let architecture = match self.distro.architecture.to_lowercase().as_str() {
            "amd64" => Architecture::amd64(),
            "i386" => Architecture::i386(),
            "arm64" => Architecture::arm64(),
            _ => Architecture::amd64(),
        };

        let hooks: Vec<Hook> = self
            .hooks
            .iter()
            .map(|h| Hook {
                name: h.name.clone(),
                script: h.script.clone(),
                stage: h.stage.clone(),
                chroot: h.chroot,
                order: h.order,
            })
            .collect();

        Ok(lingmo_core_engine::types::BuildContext {
            distro: DistroConfig {
                name: self.distro.name.clone(),
                version: self.distro.version.clone(),
                codename: self.distro.codename.clone(),
                mirror: self.distro.mirror.clone(),
                architecture: self.distro.architecture.clone(),
                components: self.distro.components.clone(),
            },
            profile,
            desktop,
            installer,
            packages: PackageConfig {
                base: self.packages.base.clone(),
                additional: self.packages.additional.clone(),
                remove: self.packages.remove.clone(),
                pin_priorities: self.packages.pin_priorities.clone(),
            },
            system: SystemConfig {
                hostname: self.system.hostname.clone(),
                locale: self.system.locale.clone(),
                timezone: self.system.timezone.clone(),
                keyboard_layout: self.system.keyboard_layout.clone(),
                users: self
                    .system
                    .users
                    .iter()
                    .map(|u| UserConfig {
                        username: u.username.clone(),
                        password_hash: u.password_hash.clone(),
                        shell: u.shell.clone(),
                        groups: u.groups.clone(),
                        sudo: u.sudo,
                    })
                    .collect(),
                fstab_entries: self.system.fstab.clone(),
                kernel_cmdline: self.system.kernel_cmdline.clone(),
            },
            output: OutputConfig {
                iso_name: self.output.iso_name.clone(),
                iso_label: self.output.iso_label.clone(),
                iso_volume: self.output.iso_volume.clone(),
                output_dir: self.output.output_dir.clone(),
                work_dir: self.output.work_dir.clone(),
                squashfs_compression: self.output.squashfs_compression.clone(),
                squashfs_block_size: self.output.squashfs_block_size,
            },
            volume: lingmo_core_engine::types::VolumeBuildConfig {
                strategy: self.volume.strategy.clone(),
                max_volume_size: self.volume.max_volume_size,
                compression: self.volume.compression.clone(),
                block_size: self.volume.block_size,
                output_pattern: self.volume.output_pattern.clone(),
                generate_manifest: self.volume.generate_manifest,
                verify_checksums: self.volume.verify_checksums,
            },
            repositories: lingmo_core_engine::types::RepositoryConfig {
                default_format: self.repositories.default_format.clone(),
                debian: lingmo_core_engine::types::DebianRepositoryConfig {
                    mirror: self.repositories.debian.mirror.clone(),
                    security_mirror: self.repositories.debian.security_mirror.clone(),
                    suite: self.repositories.debian.suite.clone(),
                    components: self.repositories.debian.components.clone(),
                    source_enabled: self.repositories.debian.source_enabled,
                },
                extra: self
                    .repositories
                    .extra
                    .iter()
                    .map(|r| lingmo_core_engine::types::ExtraRepository {
                        name: r.name.clone(),
                        repo_type: r.repo_type.clone().unwrap_or_else(|| "obs".into()),
                        url: r.url.clone(),
                        suite: r.suite.clone().unwrap_or_else(|| "/".into()),
                        components: r
                            .components
                            .clone()
                            .unwrap_or_default(),
                        key_url: r.key_url.clone(),
                        key_path: r.key_path.clone(),
                        enabled: r.enabled.unwrap_or(true),
                    })
                    .collect(),
            },
            plugins: self.plugins.clone(),
            hooks,
            cache: CacheConfig {
                enabled: self.cache.enabled,
                directory: self.cache.directory.clone(),
                stages: self.cache.stages.clone(),
            },
            architecture,
            verbosity: 1,
            dry_run: false,
        })
    }
}
