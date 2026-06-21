use serde::{Deserialize, Serialize};
use std::fmt;

/// The strategy used to split the root filesystem into multiple SquashFS volumes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VolumeStrategy {
    /// Split by size threshold (e.g. 1 GiB per volume).
    /// Files are distributed in deterministic order until the size cap is reached.
    Size,

    /// Each plugin gets its own volume based on [volume_group] declarations.
    /// Core system files that don't belong to any plugin go to volume 1.
    Plugin,

    /// Split by profile (desktop / server / core). Each profile's files
    /// go to a separate volume. This requires that the image only contains
    /// one profile at a time.
    Profile,

    /// A single volume (no splitting). Behaves like the original single-squashfs mode.
    Single,
}

impl fmt::Display for VolumeStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VolumeStrategy::Size => write!(f, "size"),
            VolumeStrategy::Plugin => write!(f, "plugin"),
            VolumeStrategy::Profile => write!(f, "profile"),
            VolumeStrategy::Single => write!(f, "single"),
        }
    }
}

impl VolumeStrategy {
    /// Parse a strategy string into a VolumeStrategy variant.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "size" => Some(VolumeStrategy::Size),
            "plugin" => Some(VolumeStrategy::Plugin),
            "profile" => Some(VolumeStrategy::Profile),
            "single" => Some(VolumeStrategy::Single),
            _ => None,
        }
    }
}

/// Configuration for multi-volume SquashFS generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    /// Which strategy to use.
    pub strategy: VolumeStrategy,

    /// Maximum size per volume (in bytes). Only used when strategy is Size.
    /// Default: 1 GiB (1_073_741_824)
    pub max_volume_size: u64,

    /// SquashFS compression algorithm.
    /// Default: "zstd"
    pub compression: String,

    /// SquashFS block size.
    /// Default: 131_072 (128 KiB)
    pub block_size: u64,

    /// Output filename pattern. `{}` is replaced with the volume index (1-based).
    /// Default: "filesystem.part{}.squashfs"
    pub output_pattern: String,

    /// Whether to generate a manifest file.
    /// Default: true
    pub generate_manifest: bool,

    /// Whether to verify checksums after building.
    /// Default: true
    pub verify_checksums: bool,
}

impl Default for VolumeConfig {
    fn default() -> Self {
        VolumeConfig {
            strategy: VolumeStrategy::Single,
            max_volume_size: 1_073_741_824,      // 1 GiB
            compression: "zstd".into(),
            block_size: 131_072,
            output_pattern: "filesystem.part{}.squashfs".into(),
            generate_manifest: true,
            verify_checksums: true,
        }
    }
}

impl VolumeConfig {
    /// Parse a human-readable size string to bytes.
    /// Supports: B, KiB, MiB, GiB (or KB, MB, GB using power-of-10).
    pub fn parse_size(input: &str) -> Option<u64> {
        let input = input.trim().to_lowercase();

        let (num_str, suffix) = if input.ends_with("kib") {
            (&input[..input.len() - 3], 1024u64)
        } else if input.ends_with("mib") {
            (&input[..input.len() - 3], 1024u64 * 1024)
        } else if input.ends_with("gib") {
            (&input[..input.len() - 3], 1024u64 * 1024 * 1024)
        } else if input.ends_with("kb") {
            (&input[..input.len() - 2], 1000u64)
        } else if input.ends_with("mb") {
            (&input[..input.len() - 2], 1000u64 * 1000)
        } else if input.ends_with("gb") {
            (&input[..input.len() - 2], 1000u64 * 1000 * 1000)
        } else if input.ends_with("b") {
            (&input[..input.len() - 1], 1u64)
        } else {
            (input.as_str(), 1u64)
        };

        let num: u64 = num_str.trim().parse().ok()?;
        Some(num * suffix)
    }

    /// Return the filename for a given volume index (1-based).
    pub fn volume_filename(&self, index: usize) -> String {
        self.output_pattern.replace("{}", &index.to_string())
    }

    /// Return the manifest filename.
    pub fn manifest_filename(&self) -> &'static str {
        "filesystem.manifest.json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(VolumeConfig::parse_size("500MB"), Some(500_000_000));
        assert_eq!(VolumeConfig::parse_size("1GiB"), Some(1_073_741_824));
        assert_eq!(VolumeConfig::parse_size("128KiB"), Some(131_072));
        assert_eq!(VolumeConfig::parse_size("2GB"), Some(2_000_000_000));
        assert_eq!(
            VolumeConfig::parse_size("1024"),
            Some(1024)
        );
        assert_eq!(VolumeConfig::parse_size(""), None);
        assert_eq!(VolumeConfig::parse_size("abc"), None);
    }

    #[test]
    fn test_volume_filename() {
        let cfg = VolumeConfig::default();
        assert_eq!(cfg.volume_filename(1), "filesystem.part1.squashfs");
        assert_eq!(cfg.volume_filename(3), "filesystem.part3.squashfs");
    }
}
