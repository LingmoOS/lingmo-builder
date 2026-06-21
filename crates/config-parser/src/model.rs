use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};



/// Top-level build configuration parsed from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default)]
    pub distro: DistroSection,

    #[serde(default = "default_profile")]
    pub profile: String,

    pub desktop: Option<String>,

    #[serde(default = "default_installer")]
    pub installer: String,

    pub installer_mode: Option<String>,

    #[serde(default)]
    pub packages: PackagesSection,

    #[serde(default)]
    pub system: SystemSection,

    #[serde(default)]
    pub output: OutputSection,

    #[serde(default)]
    pub repositories: RepositoriesSection,

    #[serde(default)]
    pub volume: VolumeSection,

    #[serde(default)]
    pub plugins: Vec<String>,

    #[serde(default)]
    pub hooks: Vec<HookSection>,

    #[serde(default)]
    pub cache: CacheSection,
}

fn default_profile() -> String {
    "core".into()
}

fn default_installer() -> String {
    "calamares".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroSection {
    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_version")]
    pub version: String,

    #[serde(default = "default_codename")]
    pub codename: String,

    #[serde(default = "default_mirror")]
    pub mirror: String,

    #[serde(default = "default_architecture")]
    pub architecture: String,

    #[serde(default = "default_components")]
    pub components: Vec<String>,
}

impl Default for DistroSection {
    fn default() -> Self {
        DistroSection {
            name: default_name(),
            version: default_version(),
            codename: default_codename(),
            mirror: default_mirror(),
            architecture: default_architecture(),
            components: default_components(),
        }
    }
}

// ---------------------------------------------------------------------------
// Repository section
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoriesSection {
    #[serde(default = "default_repo_format")]
    pub default_format: String,

    #[serde(default)]
    pub debian: DebianRepoSection,

    #[serde(default)]
    pub extra: Vec<ExtraRepoSection>,
}

impl Default for RepositoriesSection {
    fn default() -> Self {
        RepositoriesSection {
            default_format: default_repo_format(),
            debian: DebianRepoSection::default(),
            extra: vec![ExtraRepoSection {
                name: "lingmo".into(),
                repo_type: Some("obs".into()),
                url: "http://download.opensuse.org/repositories/home:/LingmoOS/Debian_13/"
                    .into(),
                suite: Some("/".into()),
                components: None,
                key_url: Some(
                    "https://download.opensuse.org/repositories/home:LingmoOS/Debian_13/Release.key"
                        .into(),
                ),
                key_path: Some("/etc/apt/keyrings/lingmo.gpg".into()),
                enabled: Some(true),
            }],
        }
    }
}

fn default_repo_format() -> String {
    "deb822".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebianRepoSection {
    #[serde(default = "default_debian_mirror")]
    pub mirror: String,

    #[serde(default = "default_debian_security_mirror")]
    pub security_mirror: String,

    #[serde(default = "default_debian_suite")]
    pub suite: String,

    #[serde(default = "default_debian_components")]
    pub components: Vec<String>,

    #[serde(default)]
    pub source_enabled: bool,
}

impl Default for DebianRepoSection {
    fn default() -> Self {
        DebianRepoSection {
            mirror: default_debian_mirror(),
            security_mirror: default_debian_security_mirror(),
            suite: default_debian_suite(),
            components: default_debian_components(),
            source_enabled: false,
        }
    }
}

fn default_debian_mirror() -> String {
    "https://mirrors.tuna.tsinghua.edu.cn/debian".into()
}
fn default_debian_security_mirror() -> String {
    "https://mirrors.tuna.tsinghua.edu.cn/debian-security".into()
}
fn default_debian_suite() -> String {
    "trixie".into()
}
fn default_debian_components() -> Vec<String> {
    vec![
        "main".into(),
        "contrib".into(),
        "non-free".into(),
        "non-free-firmware".into(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraRepoSection {
    pub name: String,

    #[serde(default)]
    pub repo_type: Option<String>,

    pub url: String,

    #[serde(default)]
    pub suite: Option<String>,

    #[serde(default)]
    pub components: Option<Vec<String>>,

    pub key_url: Option<String>,

    pub key_path: Option<String>,

    #[serde(default)]
    pub enabled: Option<bool>,
}

// ---------------------------------------------------------------------------

fn default_name() -> String {
    "LingmoOS".into()
}
fn default_version() -> String {
    "1.0".into()
}
fn default_codename() -> String {
    "trixie".into()
}
fn default_mirror() -> String {
    "http://deb.debian.org/debian".into()
}
fn default_architecture() -> String {
    "amd64".into()
}
fn default_components() -> Vec<String> {
    vec!["main".into(), "contrib".into(), "non-free".into(), "non-free-firmware".into()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagesSection {
    #[serde(default)]
    pub base: Vec<String>,

    #[serde(default)]
    pub additional: Vec<String>,

    #[serde(default)]
    pub remove: Vec<String>,

    #[serde(default)]
    pub pin_priorities: HashMap<String, i32>,
}

impl Default for PackagesSection {
    fn default() -> Self {
        PackagesSection {
            base: vec![],
            additional: vec![],
            remove: vec![],
            pin_priorities: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSection {
    #[serde(default = "default_hostname")]
    pub hostname: String,

    #[serde(default = "default_locale")]
    pub locale: String,

    #[serde(default = "default_timezone")]
    pub timezone: String,

    #[serde(default = "default_keyboard")]
    pub keyboard_layout: String,

    #[serde(default)]
    pub users: Vec<UserSection>,

    #[serde(default)]
    pub fstab: Vec<String>,

    #[serde(default = "default_kernel_cmdline")]
    pub kernel_cmdline: String,
}

impl Default for SystemSection {
    fn default() -> Self {
        SystemSection {
            hostname: default_hostname(),
            locale: default_locale(),
            timezone: default_timezone(),
            keyboard_layout: default_keyboard(),
            users: vec![],
            fstab: vec![],
            kernel_cmdline: default_kernel_cmdline(),
        }
    }
}

fn default_hostname() -> String {
    "lingmo".into()
}
fn default_locale() -> String {
    "en_US.UTF-8".into()
}
fn default_timezone() -> String {
    "UTC".into()
}
fn default_keyboard() -> String {
    "us".into()
}
fn default_kernel_cmdline() -> String {
    "quiet splash".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSection {
    pub username: String,

    #[serde(default = "default_shell")]
    pub shell: String,

    #[serde(default)]
    pub password_hash: String,

    #[serde(default)]
    pub groups: Vec<String>,

    #[serde(default)]
    pub sudo: bool,
}

fn default_shell() -> String {
    "/bin/bash".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSection {
    #[serde(default = "default_iso_name")]
    pub iso_name: String,

    #[serde(default = "default_iso_label")]
    pub iso_label: String,

    #[serde(default = "default_iso_volume")]
    pub iso_volume: String,

    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,

    #[serde(default = "default_work_dir")]
    pub work_dir: PathBuf,

    #[serde(default = "default_squashfs_compression")]
    pub squashfs_compression: String,

    #[serde(default = "default_squashfs_block_size")]
    pub squashfs_block_size: u64,
}

impl Default for OutputSection {
    fn default() -> Self {
        OutputSection {
            iso_name: default_iso_name(),
            iso_label: default_iso_label(),
            iso_volume: default_iso_volume(),
            output_dir: default_output_dir(),
            work_dir: default_work_dir(),
            squashfs_compression: default_squashfs_compression(),
            squashfs_block_size: default_squashfs_block_size(),
        }
    }
}

fn default_iso_name() -> String { "lingmo.iso".into() }
fn default_iso_label() -> String { "LINGMO_LIVE".into() }
fn default_iso_volume() -> String { "LINGMO_LIVE".into() }
fn default_output_dir() -> PathBuf { PathBuf::from("./output") }
fn default_work_dir() -> PathBuf { PathBuf::from("./work") }
fn default_squashfs_compression() -> String { "zstd".into() }
fn default_squashfs_block_size() -> u64 { 131_072 }

// ---------------------------------------------------------------------------
// Volume section
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSection {
    /// Splitting strategy: "single" (default), "size", "plugin", "profile"
    #[serde(default = "default_volume_strategy")]
    pub strategy: String,

    /// Max bytes per volume when strategy = "size" (default: 1 GiB)
    #[serde(default = "default_volume_max_size")]
    pub max_volume_size: u64,

    /// SquashFS compression (default: "zstd")
    #[serde(default = "default_volume_compression")]
    pub compression: String,

    /// SquashFS block size in bytes (default: 131072 = 128 KiB)
    #[serde(default = "default_volume_block_size")]
    pub block_size: u64,

    /// Output filename pattern (default: "filesystem.part{}.squashfs")
    #[serde(default = "default_volume_pattern")]
    pub output_pattern: String,

    /// Generate manifest JSON (default: true)
    #[serde(default = "default_volume_manifest")]
    pub generate_manifest: bool,

    /// Verify SHA-256 checksums after build (default: true)
    #[serde(default = "default_volume_verify")]
    pub verify_checksums: bool,
}

fn default_volume_strategy() -> String { "single".into() }
fn default_volume_max_size() -> u64 { 1_073_741_824 }
fn default_volume_compression() -> String { "zstd".into() }
fn default_volume_block_size() -> u64 { 131_072 }
fn default_volume_pattern() -> String { "filesystem.part{}.squashfs".into() }
fn default_volume_manifest() -> bool { true }
fn default_volume_verify() -> bool { true }

impl Default for VolumeSection {
    fn default() -> Self {
        VolumeSection {
            strategy: default_volume_strategy(),
            max_volume_size: default_volume_max_size(),
            compression: default_volume_compression(),
            block_size: default_volume_block_size(),
            output_pattern: default_volume_pattern(),
            generate_manifest: default_volume_manifest(),
            verify_checksums: default_volume_verify(),
        }
    }
}

// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSection {
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,

    #[serde(default = "default_cache_dir")]
    pub directory: PathBuf,

    #[serde(default)]
    pub stages: Vec<String>,
}

fn default_cache_enabled() -> bool { false }
fn default_cache_dir() -> PathBuf { PathBuf::from("./cache") }

impl Default for CacheSection {
    fn default() -> Self {
        CacheSection {
            enabled: default_cache_enabled(),
            directory: default_cache_dir(),
            stages: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookSection {
    pub name: String,
    pub script: String,
    #[serde(default)]
    pub stage: String,
    #[serde(default = "default_hook_chroot")]
    pub chroot: bool,
    #[serde(default = "default_hook_order")]
    pub order: u32,
}

fn default_hook_chroot() -> bool { true }
fn default_hook_order() -> u32 { 50 }
