#!/bin/bash
#=============================================================================
# grub-install.sh - Install GRUB bootloader for ISO generation
#=============================================================================
# Usage: grub-install.sh <rootfs-dir> <iso-dir>
#
# Sets up GRUB for both BIOS (legacy) and UEFI boot on the target ISO.
# Creates:
#   - BIOS boot image (boot/grub/bios.img)
#   - UEFI boot image (EFI/BOOT/BOOTx64.EFI)
#   - GRUB configuration
#
# Called by the Lingmo Builder Rust pipeline stage executor.
#=============================================================================
set -euo pipefail

ROOTFS="${1:?Usage: $0 <rootfs-dir> <iso-dir>}"
ISODIR="${2:?Usage: $0 <rootfs-dir> <iso-dir>}"

if [ ! -d "$ROOTFS" ]; then
    echo "ERROR: Rootfs directory does not exist: $ROOTFS"
    exit 1
fi

mkdir -p "$ISODIR/boot/grub"
mkdir -p "$ISODIR/EFI/BOOT"

#=============================================================================
# BIOS boot image
#=============================================================================
echo "==> Creating BIOS boot image..."
# Use grub-mkimage to create a BIOS bootable image
grub-mkimage \
    -O i386-pc \
    -o "$ISODIR/boot/grub/bios.img" \
    -p "/boot/grub" \
    biosdisk iso9660 part_msdos part_gpt fat ntfs ext2 \
    normal configfile search search_fs_file search_fs_uuid \
    search_label linux chain boot minicmd cat echo ls \
    test true false loadenv help reboot halt sleep \
    gfxterm gfxmenu gfxterm_background gfxterm_menu \
    video all_video font png jpeg

# Create boot catalog
touch "$ISODIR/boot/grub/boot.cat"

#=============================================================================
# UEFI boot image
#=============================================================================
echo "==> Creating UEFI boot image..."

# Check if we need signed or unsigned EFI
if [ -f "$ROOTFS/usr/lib/grub/x86_64-efi-signed/grubnetx64.efi.signed" ]; then
    echo "    Using signed GRUB EFI binary"
    cp "$ROOTFS/usr/lib/grub/x86_64-efi-signed/grubnetx64.efi.signed" \
       "$ISODIR/EFI/BOOT/BOOTx64.EFI"
else
    echo "    Building unsigned GRUB EFI binary"
    grub-mkimage \
        -O x86_64-efi \
        -o "$ISODIR/EFI/BOOT/BOOTx64.EFI" \
        -p "/boot/grub" \
        boot linux search normal configfile part_gpt part_msdos \
        fat iso9660 ext2 udf ntfs chain efifwsetup efi_uga \
        efi_gop file echo all_video video gfxterm font \
        gfxmenu gfxterm_background gfxterm_menu test true false \
        loadenv reboot halt sleep help ls cat \
        loopback regexp tr truncate
fi

# Copy GRUB fonts and localization
if [ -d "$ROOTFS/usr/share/grub" ]; then
    cp -r "$ROOTFS/usr/share/grub"/*.pf2 "$ISODIR/boot/grub/" 2>/dev/null || true
fi

# Copy GRUB locale files
if [ -d "$ROOTFS/usr/share/locale" ]; then
    mkdir -p "$ISODIR/boot/grub/locale"
    cp "$ROOTFS/usr/share/locale/en@quot/LC_MESSAGES/grub.mo" \
       "$ISODIR/boot/grub/locale/en.mo" 2>/dev/null || true
fi

# Copy any GRUB themes
if [ -d "$ROOTFS/usr/share/grub/themes" ]; then
    cp -r "$ROOTFS/usr/share/grub/themes" "$ISODIR/boot/grub/" 2>/dev/null || true
fi

echo "==> GRUB installation complete"
echo "    BIOS:  $ISODIR/boot/grub/bios.img"
echo "    UEFI:  $ISODIR/EFI/BOOT/BOOTx64.EFI"
