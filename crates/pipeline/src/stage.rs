use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stage {
    Init,
    Bootstrap,
    /// Configure Debian repositories (DEB822 format, debian.sources)
    ConfigureDebianRepos,
    /// Configure extra repositories (Lingmo OBS, etc.), download keys, apt update
    ConfigureExtraRepos,
    InstallBase,
    InstallKernel,
    InstallFirmware,
    ApplyProfile,
    InstallDesktop,
    AdditionalPackages,
    FilesystemOverlays,
    ChrootHooks,
    SystemConfig,
    InstallBootloader,
    /// Replaces GenerateSquashfs. Produces one or more SquashFS volumes
    /// depending on the configured strategy (single, size, plugin, profile).
    /// Backward-compatible alias "generate-squashfs" is also recognized.
    GenerateSquashfsVolumes,
    GenerateIso,
    Cleanup,
}

impl Stage {
    pub const ALL: &'static [Stage] = &[
        Stage::Init,
        Stage::Bootstrap,
        Stage::ConfigureDebianRepos,
        Stage::ConfigureExtraRepos,
        Stage::InstallBase,
        Stage::InstallKernel,
        Stage::InstallFirmware,
        Stage::ApplyProfile,
        Stage::InstallDesktop,
        Stage::AdditionalPackages,
        Stage::FilesystemOverlays,
        Stage::ChrootHooks,
        Stage::SystemConfig,
        Stage::InstallBootloader,
        Stage::GenerateSquashfsVolumes,
        Stage::GenerateIso,
        Stage::Cleanup,
    ];

    pub fn description(&self) -> &'static str {
        match self {
            Stage::Init => "Initialize build workspace",
            Stage::Bootstrap => "Bootstrap rootfs via debootstrap",
            Stage::ConfigureDebianRepos => {
                "Configure Debian repositories (DEB822 format)"
            }
            Stage::ConfigureExtraRepos => {
                "Configure extra repositories and install GPG keys"
            }
            Stage::InstallBase => "Install base system packages",
            Stage::InstallKernel => "Install Linux kernel",
            Stage::InstallFirmware => "Install firmware and drivers",
            Stage::ApplyProfile => "Apply system profile (desktop/server/core)",
            Stage::InstallDesktop => "Install desktop environment",
            Stage::AdditionalPackages => "Install additional user-specified packages",
            Stage::FilesystemOverlays => "Apply filesystem overlays",
            Stage::ChrootHooks => "Run chroot configuration hooks",
            Stage::SystemConfig => "Configure system (users, locale, hostname, fstab)",
            Stage::InstallBootloader => "Install GRUB bootloader (BIOS/UEFI)",
            Stage::GenerateSquashfsVolumes => {
                "Generate SquashFS volume(s) with manifest"
            }
            Stage::GenerateIso => "Generate ISO image",
            Stage::Cleanup => "Clean up workspace",
        }
    }

    pub fn is_cacheable(&self) -> bool {
        matches!(
            self,
            Stage::Bootstrap
                | Stage::InstallBase
                | Stage::InstallKernel
                | Stage::InstallFirmware
                | Stage::ApplyProfile
                | Stage::InstallDesktop
                | Stage::AdditionalPackages
        )
    }

    pub fn requires_root(&self) -> bool {
        !matches!(
            self,
            Stage::Init
                | Stage::ConfigureDebianRepos
                | Stage::ConfigureExtraRepos
                | Stage::GenerateSquashfsVolumes
                | Stage::GenerateIso
                | Stage::Cleanup
        )
    }

    pub fn needs_network(&self) -> bool {
        matches!(
            self,
            Stage::Bootstrap
                | Stage::ConfigureExtraRepos
                | Stage::InstallBase
                | Stage::InstallKernel
                | Stage::InstallFirmware
                | Stage::ApplyProfile
                | Stage::InstallDesktop
                | Stage::AdditionalPackages
        )
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
