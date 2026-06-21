use lingmo_core_engine::types::{DistroConfig, OutputConfig};
use lingmo_core_engine::{run_command, BuildResult};
use std::path::Path;

/// Generate a squashfs filesystem from a rootfs directory.
pub fn generate_squashfs(
    rootfs: &Path,
    output: &Path,
    compression: &str,
    block_size: u64,
) -> BuildResult<()> {
    tracing::info!(
        "Generating squashfs: {} -> {} (compression: {}, block: {})",
        rootfs.display(),
        output.display(),
        compression,
        block_size
    );

    let block_size_str = block_size.to_string();
    run_command(
        "mksquashfs",
        &[
            rootfs.to_str().unwrap(),
            output.to_str().unwrap(),
            "-comp",
            compression,
            "-b",
            &block_size_str,
            "-noappend",
            "-no-xattrs",
            "-no-exports",
            "-all-root",
        ],
        "Creating squashfs filesystem",
    )?;

    tracing::info!("Squashfs created: {}", output.display());
    Ok(())
}

/// Generate GRUB configuration for the ISO.
pub fn generate_grub_config(
    distro: &DistroConfig,
    kernel_cmdline: &str,
    output_dir: &Path,
) -> BuildResult<()> {
    let grub_dir = output_dir.join("boot/grub");
    std::fs::create_dir_all(&grub_dir).map_err(|e| lingmo_core_engine::BuildError::Io {
        path: grub_dir.clone(),
        source: e,
    })?;

    let grub_cfg = format!(
        r#"set default="0"
set timeout=5

insmod all_video
insmod gfxterm
insmod png
insmod font

if loadfont $prefix/fonts/unicode.pf2; then
    set gfxmode=auto
    set gfxpayload=keep
    terminal_output gfxterm
fi

menuentry "{} {} Live" {{
    linux /live/vmlinuz {} boot=live components quiet
    initrd /live/initrd.img
}}

menuentry "{} {} Live (Safe Graphics)" {{
    linux /live/vmlinuz {} boot=live components nomodeset radeon.modeset=0 amdgpu.modeset=0 nouveau.modeset=0
    initrd /live/initrd.img
}}

menuentry "{} {} Install" {{
    linux /live/vmlinuz {} boot=live components quiet install
    initrd /live/initrd.img
}}
"#,
        distro.name, distro.version, kernel_cmdline,
        distro.name, distro.version, kernel_cmdline,
        distro.name, distro.version, kernel_cmdline,
    );

    let grub_cfg_path = grub_dir.join("grub.cfg");
    std::fs::write(&grub_cfg_path, &grub_cfg).map_err(|e| {
        lingmo_core_engine::BuildError::Io {
            path: grub_cfg_path,
            source: e,
        }
    })?;

    Ok(())
}

/// Build the final bootable ISO image.
pub fn build_iso(
    iso_dir: &Path,
    squashfs_path: &Path,
    output_cfg: &OutputConfig,
    distro: &DistroConfig,
    kernel_cmdline: &str,
) -> BuildResult<()> {
    tracing::info!("Building ISO image");

    let live_dir = iso_dir.join("live");
    std::fs::create_dir_all(&live_dir).map_err(|e| lingmo_core_engine::BuildError::Io {
        path: live_dir.clone(),
        source: e,
    })?;

    // Copy squashfs to live directory
    let dest_squashfs = live_dir.join("filesystem.squashfs");
    std::fs::copy(squashfs_path, &dest_squashfs).map_err(|e| {
        lingmo_core_engine::BuildError::Io {
            path: dest_squashfs,
            source: e,
        }
    })?;

    // Generate GRUB configuration
    generate_grub_config(distro, kernel_cmdline, iso_dir)?;

    // Create EFI boot image
    let efi_dir = iso_dir.join("EFI/BOOT");
    std::fs::create_dir_all(&efi_dir).map_err(|e| lingmo_core_engine::BuildError::Io {
        path: efi_dir.clone(),
        source: e,
    })?;

    // Build ISO with xorriso
    let iso_path = output_cfg.output_dir.join(&output_cfg.iso_name);
    std::fs::create_dir_all(&output_cfg.output_dir).map_err(|e| {
        lingmo_core_engine::BuildError::Io {
            path: output_cfg.output_dir.clone(),
            source: e,
        }
    })?;

    run_command(
        "xorriso",
        &[
            "-as",
            "mkisofs",
            "-iso-level",
            "3",
            "-full-iso9660-filenames",
            "-volid",
            &output_cfg.iso_volume,
            "-appid",
            &format!("{} Builder", distro.name),
            "-publisher",
            &distro.name,
            "-eltorito-boot",
            "boot/grub/bios.img",
            "-no-emul-boot",
            "-boot-load-size",
            "4",
            "-boot-info-table",
            "--eltorito-catalog",
            "boot/grub/boot.cat",
            "-eltorito-alt-boot",
            "-e",
            "EFI/BOOT/BOOTx64.EFI",
            "-no-emul-boot",
            "-isohybrid-gpt-basdat",
            "-o",
            iso_path.to_str().unwrap(),
            iso_dir.to_str().unwrap(),
        ],
        "Generating ISO image",
    )?;

    tracing::info!("ISO created: {}", iso_path.display());
    Ok(())
}
