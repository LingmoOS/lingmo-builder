use std::path::{Path, PathBuf};

use lingmo_core_engine::error::{BuildError, BuildResult};
use walkdir::WalkDir;

/// A single file entry discovered during rootfs traversal.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Absolute path on the host filesystem inside the rootfs.
    pub source: PathBuf,

    /// Path relative to the rootfs root.
    pub relative: PathBuf,

    /// File size in bytes.
    pub size: u64,

    /// Whether this is a regular file (not a directory, symlink, or special).
    pub is_file: bool,

    /// Whether this is a directory.
    pub is_dir: bool,

    /// Whether this is a symlink.
    pub is_symlink: bool,
}

/// A complete catalog of all files in a rootfs, sorted deterministically.
#[derive(Debug, Clone)]
pub struct FileCatalog {
    /// All entries, sorted by relative path.
    pub entries: Vec<FileEntry>,

    /// Total size of all regular files in bytes.
    pub total_size: u64,

    /// Total number of entries (files + dirs + symlinks).
    pub total_count: usize,

    /// Number of regular files.
    pub file_count: usize,
}

impl FileCatalog {
    /// Walk the rootfs directory and build a sorted, deterministic file catalog.
    ///
    /// The walk uses a consistent order (sorted by path) to ensure
    /// reproducible volume splits.
    pub fn from_rootfs(rootfs: &Path) -> BuildResult<Self> {
        if !rootfs.is_dir() {
            return Err(BuildError::Config(format!(
                "Rootfs path is not a directory: {}",
                rootfs.display()
            )));
        }

        let mut entries: Vec<FileEntry> = Vec::new();
        let mut total_size: u64 = 0;
        let mut file_count: usize = 0;

        // Collect all entries first
        for entry in WalkDir::new(rootfs)
            .sort_by(|a, b| a.path().cmp(b.path()))
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                // Skip common mount points and special dirs
                let skip = [
                    "proc",
                    "sys",
                    "dev",
                    "run",
                    "mnt",
                    "tmp",
                ];
                if e.depth() == 1 {
                    if let Some(name) = e.file_name().to_str() {
                        if skip.contains(&name) {
                            return false;
                        }
                    }
                }
                true
            })
        {
            let entry = entry.map_err(|e| {
                let msg = e.to_string();
                let io_err = e
                    .into_io_error()
                    .unwrap_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, msg));
                BuildError::Io {
                    path: rootfs.to_path_buf(),
                    source: io_err,
                }
            })?;

            let path = entry.path().to_path_buf();
            let relative = path
                .strip_prefix(rootfs)
                .map_err(|_| BuildError::Other(format!(
                    "Path {} is not under rootfs {}",
                    path.display(),
                    rootfs.display()
                )))?
                .to_path_buf();

            // Skip the rootfs root itself
            if relative.as_os_str().is_empty() {
                continue;
            }

            let metadata = entry.metadata().map_err(|e| BuildError::Io {
                path: path.clone(),
                source: e.into(),
            })?;

            let fe = FileEntry {
                source: path,
                relative,
                size: metadata.len(),
                is_file: metadata.is_file(),
                is_dir: metadata.is_dir(),
                is_symlink: metadata.file_type().is_symlink(),
            };

            if fe.is_file {
                total_size = total_size.saturating_add(fe.size);
                file_count += 1;
            }

            entries.push(fe);
        }

        // Ensure deterministic order
        entries.sort_by(|a, b| a.relative.cmp(&b.relative));

        let total_count = entries.len();

        tracing::info!(
            "Cataloged rootfs: {} entries ({} files), {} total",
            total_count,
            file_count,
            humansize(total_size)
        );

        Ok(FileCatalog {
            entries,
            total_size,
            total_count,
            file_count,
        })
    }

    /// Iterate over regular files in order.
    pub fn files(&self) -> impl Iterator<Item = &FileEntry> {
        self.entries.iter().filter(|e| e.is_file)
    }

    /// Iterate over all entries in order.
    pub fn iter(&self) -> impl Iterator<Item = &FileEntry> {
        self.entries.iter()
    }
}

pub(crate) fn humansize(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_idx])
}

/// Represents an assignment of files to a specific volume.
#[derive(Debug, Clone)]
pub struct VolumeAssignment {
    /// 1-based volume index.
    pub index: usize,

    /// Name/label for this volume (e.g. "core", "kde", "drivers").
    pub label: String,

    /// Files assigned to this volume.
    pub entries: Vec<FileEntry>,

    /// Total size of files in this volume.
    pub total_size: u64,
}

/// The result of splitting a rootfs into multiple volumes.
#[derive(Debug, Clone)]
pub struct VolumeSplit {
    /// All volume assignments, ordered by mount priority (index 1 = lowest).
    pub volumes: Vec<VolumeAssignment>,

    /// The original catalog.
    pub catalog: FileCatalog,
}

impl VolumeSplit {
    /// Number of volumes.
    pub fn count(&self) -> usize {
        self.volumes.len()
    }

    /// True if there is only one volume (no splitting).
    pub fn is_single(&self) -> bool {
        self.volumes.len() <= 1
    }
}
