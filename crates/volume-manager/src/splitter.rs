use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use lingmo_core_engine::error::BuildResult;

use crate::catalog::{FileCatalog, FileEntry, VolumeAssignment, VolumeSplit};
use crate::strategy::{VolumeConfig, VolumeStrategy};

/// Split the rootfs into multiple volumes according to the given strategy and config.
pub fn split_volumes(
    catalog: &FileCatalog,
    config: &VolumeConfig,
    volume_groups: &[VolumeGroup],
) -> BuildResult<VolumeSplit> {
    match config.strategy {
        VolumeStrategy::Size => split_by_size(catalog, config),
        VolumeStrategy::Plugin => split_by_plugin(catalog, volume_groups, config),
        VolumeStrategy::Profile => split_by_profile(catalog, config),
        VolumeStrategy::Single => split_single(catalog),
    }
}

/// A plugin-declared volume group: which directory prefixes belong to which volume.
#[derive(Debug, Clone)]
pub struct VolumeGroup {
    pub label: String,
    pub priority: usize,
    pub prefixes: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// Size-based splitting
// ---------------------------------------------------------------------------

fn split_by_size(catalog: &FileCatalog, config: &VolumeConfig) -> BuildResult<VolumeSplit> {
    let max_bytes = config.max_volume_size;
    let mut volumes: Vec<VolumeAssignment> = Vec::new();
    let mut current_entries: Vec<FileEntry> = Vec::new();
    let mut current_size: u64 = 0;
    let mut volume_index: usize = 0;
    let mut is_first = true;

    // First, add all directories to volume 1 (they carry little size cost)
    // so that the directory structure is always present in the first volume.
    let all_dirs: Vec<FileEntry> = catalog
        .iter()
        .filter(|e| e.is_dir)
        .cloned()
        .collect();

    // Process files in deterministic order
    for entry in catalog.files() {
        // Check if this file would exceed the volume size limit
        // (only if we already have files in the current volume)
        if !is_first && current_size > 0 && (current_size + entry.size) > max_bytes {
            // Finalize current volume
            volume_index += 1;
            let label = if volume_index == 1 {
                "core".to_string()
            } else {
                format!("volume{}", volume_index)
            };

            volumes.push(VolumeAssignment {
                index: volume_index,
                label,
                entries: std::mem::take(&mut current_entries),
                total_size: current_size,
            });
            current_size = 0;
        }

        current_entries.push(entry.clone());
        current_size = current_size.saturating_add(entry.size);
        is_first = false;
    }

    // Finalize the last volume
    volume_index += 1;
    let label = if volume_index == 1 {
        "core".to_string()
    } else {
        format!("volume{}", volume_index)
    };

    volumes.push(VolumeAssignment {
        index: volume_index,
        label,
        entries: std::mem::take(&mut current_entries),
        total_size: current_size,
    });

    // Add directories to the first volume only (to avoid duplication)
    if let Some(first) = volumes.first_mut() {
        first.entries.extend(all_dirs);
        first.entries.sort_by(|a, b| a.relative.cmp(&b.relative));
    }

    tracing::info!(
        "Size-based split: {} volumes (max {} per volume)",
        volumes.len(),
        crate::catalog::humansize(config.max_volume_size)
    );

    Ok(VolumeSplit {
        volumes,
        catalog: catalog.clone(),
    })
}

// ---------------------------------------------------------------------------
// Plugin-based splitting
// ---------------------------------------------------------------------------

fn split_by_plugin(
    catalog: &FileCatalog,
    volume_groups: &[VolumeGroup],
    _config: &VolumeConfig,
) -> BuildResult<VolumeSplit> {
    // Build a prefix → group mapping
    let mut prefix_map: BTreeMap<&Path, &VolumeGroup> = BTreeMap::new();
    for group in volume_groups {
        for prefix in &group.prefixes {
            prefix_map.insert(prefix.as_path(), group);
        }
    }

    // Assign each file to a group based on its relative path
    let mut group_entries: BTreeMap<String, Vec<FileEntry>> = BTreeMap::new();
    let mut unassigned: Vec<FileEntry> = Vec::new();

    for entry in catalog.iter() {
        let mut matched = false;
        for (prefix, group) in &prefix_map {
            if entry.relative.starts_with(prefix) {
                group_entries
                    .entry(group.label.clone())
                    .or_default()
                    .push(entry.clone());
                matched = true;
                break;
            }
        }
        if !matched {
            unassigned.push(entry.clone());
        }
    }

    // Build volumes: core goes first, then plugin groups
    let mut volumes: Vec<VolumeAssignment> = Vec::new();
    let mut index: usize = 0;

    // Volume 1: unassigned (core) files
    if !unassigned.is_empty() {
        index += 1;
        let total_size: u64 = unassigned.iter().map(|e| e.size).sum();
        volumes.push(VolumeAssignment {
            index,
            label: "core".into(),
            entries: unassigned,
            total_size,
        });
    }

    // Subsequent volumes: one per plugin group
    let group_count = group_entries.len();
    for (label, mut entries) in group_entries {
        entries.sort_by(|a, b| a.relative.cmp(&b.relative));
        index += 1;
        let total_size: u64 = entries.iter().map(|e| e.size).sum();
        volumes.push(VolumeAssignment {
            index,
            label,
            entries,
            total_size,
        });
    }

    tracing::info!(
        "Plugin-based split: {} volumes ({} plugin groups)",
        volumes.len(),
        group_count
    );

    Ok(VolumeSplit {
        volumes,
        catalog: catalog.clone(),
    })
}

// ---------------------------------------------------------------------------
// Profile-based splitting
// ---------------------------------------------------------------------------

fn split_by_profile(
    catalog: &FileCatalog,
    _config: &VolumeConfig,
) -> BuildResult<VolumeSplit> {
    // Profile-based split: separate files by common directory prefixes.
    // Desktop files → volume 2, core system → volume 1.
    let desktop_prefixes: Vec<PathBuf> = vec![
        "usr/share/applications".into(),
        "usr/share/icons".into(),
        "usr/share/themes".into(),
        "usr/share/fonts".into(),
        "usr/share/wallpapers".into(),
        "usr/share/sounds".into(),
        "usr/share/backgrounds".into(),
        "usr/share/desktop-base".into(),
        "usr/share/doc".into(),
        "usr/share/help".into(),
        "usr/share/man".into(),
        "usr/share/gnome".into(),
        "usr/share/kde".into(),
        "usr/share/xfce".into(),
        "usr/lib/xorg".into(),
        "usr/lib/firmware".into(),
    ];

    let mut core_entries: Vec<FileEntry> = Vec::new();
    let mut desktop_entries: Vec<FileEntry> = Vec::new();

    for entry in catalog.iter() {
        let is_desktop = desktop_prefixes.iter().any(|p| entry.relative.starts_with(p));
        if is_desktop {
            desktop_entries.push(entry.clone());
        } else {
            core_entries.push(entry.clone());
        }
    }

    let mut volumes: Vec<VolumeAssignment> = Vec::new();

    // Volume 1: core
    let core_size: u64 = core_entries.iter().map(|e| e.size).sum();
    volumes.push(VolumeAssignment {
        index: 1,
        label: "core".into(),
        entries: core_entries,
        total_size: core_size,
    });

    let has_extensions = !desktop_entries.is_empty();

    // Volume 2: desktop extensions (if any)
    if has_extensions {
        let desktop_size: u64 = desktop_entries.iter().map(|e| e.size).sum();
        volumes.push(VolumeAssignment {
            index: 2,
            label: "extensions".into(),
            entries: desktop_entries,
            total_size: desktop_size,
        });
    }

    tracing::info!(
        "Profile-based split: {} volumes (core + {} extensions)",
        volumes.len(),
        if has_extensions { 1 } else { 0 }
    );

    Ok(VolumeSplit {
        volumes,
        catalog: catalog.clone(),
    })
}

// ---------------------------------------------------------------------------
// Single volume (passthrough)
// ---------------------------------------------------------------------------

fn split_single(catalog: &FileCatalog) -> BuildResult<VolumeSplit> {
    let total_size: u64 = catalog.files().map(|e| e.size).sum();
    let volumes = vec![VolumeAssignment {
        index: 1,
        label: "full".into(),
        entries: catalog.entries.clone(),
        total_size,
    }];

    tracing::info!("Single volume (no splitting): 1 volume");

    Ok(VolumeSplit {
        volumes,
        catalog: catalog.clone(),
    })
}
