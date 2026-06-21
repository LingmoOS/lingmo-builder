use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Profile {
    Desktop,
    Server,
    Core,
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Profile::Desktop => write!(f, "desktop"),
            Profile::Server => write!(f, "server"),
            Profile::Core => write!(f, "core"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DesktopEnvironment {
    KDE,
    GNOME,
    XFCE,
    LXQT,
    MATE,
    Cinnamon,
    Budgie,
    Sway,
    Hyprland,
    None,
}

impl std::fmt::Display for DesktopEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DesktopEnvironment::KDE => write!(f, "kde"),
            DesktopEnvironment::GNOME => write!(f, "gnome"),
            DesktopEnvironment::XFCE => write!(f, "xfce"),
            DesktopEnvironment::LXQT => write!(f, "lxqt"),
            DesktopEnvironment::MATE => write!(f, "mate"),
            DesktopEnvironment::Cinnamon => write!(f, "cinnamon"),
            DesktopEnvironment::Budgie => write!(f, "budgie"),
            DesktopEnvironment::Sway => write!(f, "sway"),
            DesktopEnvironment::Hyprland => write!(f, "hyprland"),
            DesktopEnvironment::None => write!(f, "none"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiMode {
    Graphical,
    Text,
    Ncurses,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallerType {
    Calamares,
    DebianInstaller { mode: DiMode },
}

impl std::fmt::Display for InstallerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallerType::Calamares => write!(f, "calamares"),
            InstallerType::DebianInstaller { mode: _ } => write!(f, "debian-installer"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Architecture {
    pub dpkg: String,
    pub kernel: String,
    pub grub: String,
    pub qemu: String,
}

impl Architecture {
    pub fn amd64() -> Self {
        Architecture {
            dpkg: "amd64".into(),
            kernel: "amd64".into(),
            grub: "x86_64-efi".into(),
            qemu: "x86_64".into(),
        }
    }

    pub fn i386() -> Self {
        Architecture {
            dpkg: "i386".into(),
            kernel: "686".into(),
            grub: "i386-efi".into(),
            qemu: "i386".into(),
        }
    }

    pub fn arm64() -> Self {
        Architecture {
            dpkg: "arm64".into(),
            kernel: "arm64".into(),
            grub: "arm64-efi".into(),
            qemu: "aarch64".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroConfig {
    pub name: String,
    pub version: String,
    pub codename: String,
    pub mirror: String,
    pub architecture: String,
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub base: Vec<String>,
    pub additional: Vec<String>,
    pub remove: Vec<String>,
    pub pin_priorities: HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub hostname: String,
    pub locale: String,
    pub timezone: String,
    pub keyboard_layout: String,
    pub users: Vec<UserConfig>,
    pub fstab_entries: Vec<String>,
    pub kernel_cmdline: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub username: String,
    pub password_hash: String,
    pub shell: String,
    pub groups: Vec<String>,
    pub sudo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub iso_name: String,
    pub iso_label: String,
    pub iso_volume: String,
    pub output_dir: PathBuf,
    pub work_dir: PathBuf,
    pub squashfs_compression: String,
    pub squashfs_block_size: u64,
}

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig {
            iso_name: "lingmo.iso".into(),
            iso_label: "LINGMO_LIVE".into(),
            iso_volume: "LINGMO_LIVE".into(),
            output_dir: PathBuf::from("./output"),
            work_dir: PathBuf::from("./work"),
            squashfs_compression: "zstd".into(),
            squashfs_block_size: 131_072,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub name: String,
    pub script: String,
    pub stage: String,
    pub chroot: bool,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub directory: PathBuf,
    pub stages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VolumeBuildConfig {
    /// Splitting strategy: "single", "size", "plugin", "profile"
    pub strategy: String,
    /// Max bytes per volume (strategy = "size")
    pub max_volume_size: u64,
    /// SquashFS compression
    pub compression: String,
    /// SquashFS block size
    pub block_size: u64,
    /// Output pattern (e.g. "filesystem.part{}.squashfs")
    pub output_pattern: String,
    /// Generate manifest JSON
    pub generate_manifest: bool,
    /// Verify checksums after build
    pub verify_checksums: bool,
}

impl Default for VolumeBuildConfig {
    fn default() -> Self {
        VolumeBuildConfig {
            strategy: "single".into(),
            max_volume_size: 1_073_741_824,
            compression: "zstd".into(),
            block_size: 131_072,
            output_pattern: "filesystem.part{}.squashfs".into(),
            generate_manifest: true,
            verify_checksums: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraRepository {
    pub name: String,
    /// Repository type: "obs", "apt", or "plain"
    pub repo_type: String,
    pub url: String,
    pub suite: String,
    pub components: Vec<String>,
    pub key_url: Option<String>,
    pub key_path: Option<String>,
    pub enabled: bool,
}

impl Default for ExtraRepository {
    fn default() -> Self {
        ExtraRepository {
            name: String::new(),
            repo_type: "obs".into(),
            url: String::new(),
            suite: "/".into(),
            components: vec![],
            key_url: None,
            key_path: None,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebianRepositoryConfig {
    pub mirror: String,
    pub security_mirror: String,
    pub suite: String,
    pub components: Vec<String>,
    pub source_enabled: bool,
}

impl Default for DebianRepositoryConfig {
    fn default() -> Self {
        DebianRepositoryConfig {
            mirror: "https://mirrors.tuna.tsinghua.edu.cn/debian".into(),
            security_mirror: "https://mirrors.tuna.tsinghua.edu.cn/debian-security".into(),
            suite: "trixie".into(),
            components: vec![
                "main".into(),
                "contrib".into(),
                "non-free".into(),
                "non-free-firmware".into(),
            ],
            source_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub default_format: String,
    pub debian: DebianRepositoryConfig,
    pub extra: Vec<ExtraRepository>,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        RepositoryConfig {
            default_format: "deb822".into(),
            debian: DebianRepositoryConfig::default(),
            extra: vec![ExtraRepository {
                name: "lingmo".into(),
                repo_type: "obs".into(),
                url: "http://download.opensuse.org/repositories/home:/LingmoOS/Debian_13/".into(),
                suite: "/".into(),
                components: vec![],
                key_url: Some(
                    "https://download.opensuse.org/repositories/home:LingmoOS/Debian_13/Release.key"
                        .into(),
                ),
                key_path: Some("/etc/apt/keyrings/lingmo.gpg".into()),
                enabled: true,
            }],
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildContext {
    pub distro: DistroConfig,
    pub profile: Profile,
    pub desktop: Option<DesktopEnvironment>,
    pub installer: InstallerType,
    pub packages: PackageConfig,
    pub system: SystemConfig,
    pub output: OutputConfig,
    pub volume: VolumeBuildConfig,
    pub repositories: RepositoryConfig,
    pub plugins: Vec<String>,
    pub hooks: Vec<Hook>,
    pub cache: CacheConfig,
    pub architecture: Architecture,
    pub verbosity: u8,
    pub dry_run: bool,
}

impl BuildContext {
    pub fn rootfs_dir(&self) -> PathBuf {
        self.output.work_dir.join("chroot")
    }

    pub fn overlay_dir(&self) -> PathBuf {
        self.output.work_dir.join("overlay")
    }

    pub fn squashfs_path(&self) -> PathBuf {
        self.output.work_dir.join("filesystem.squashfs")
    }

    pub fn iso_dir(&self) -> PathBuf {
        self.output.work_dir.join("iso")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.cache.directory.clone()
    }
}
