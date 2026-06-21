#!/bin/bash
#=============================================================================
# chroot-mount.sh - Safely mount/unmount chroot filesystems
#=============================================================================
# Usage: chroot-mount.sh <rootfs-dir> {mount|umount}
#
# Mounts the required virtual filesystems for chroot operations:
#   /proc, /sys, /dev, /dev/pts, /run
#
# Unmounts them in reverse order with lazy unmount as fallback.
# Called by the Lingmo Builder Rust pipeline stage executor.
#=============================================================================
set -euo pipefail

ROOTFS="${1:?Usage: $0 <rootfs-dir> {mount|umount}}"
ACTION="${2:?Usage: $0 <rootfs-dir> {mount|umount}}"

if [ ! -d "$ROOTFS" ]; then
    echo "ERROR: Rootfs directory does not exist: $ROOTFS"
    exit 1
fi

do_mount() {
    # Mount proc
    mount -t proc proc "$ROOTFS/proc" 2>/dev/null || true

    # Mount sysfs
    mount -t sysfs sys "$ROOTFS/sys" 2>/dev/null || true

    # Mount udev
    mount -t devtmpfs udev "$ROOTFS/dev" 2>/dev/null || true

    # Mount devpts
    mount -t devpts devpts "$ROOTFS/dev/pts" -o mode=0620,ptmxmode=0666,gid=5 2>/dev/null || true

    # Mount tmpfs for /run
    mount -t tmpfs run "$ROOTFS/run" 2>/dev/null || true

    # Mount shm
    mount -t tmpfs shm "$ROOTFS/dev/shm" -o mode=1777 2>/dev/null || true
}

do_umount() {
    # Unmount in reverse order
    local targets=(
        "$ROOTFS/dev/pts"
        "$ROOTFS/dev/shm"
        "$ROOTFS/run"
        "$ROOTFS/dev"
        "$ROOTFS/sys"
        "$ROOTFS/proc"
    )

    for target in "${targets[@]}"; do
        if mountpoint -q "$target" 2>/dev/null; then
            umount "$target" 2>/dev/null || umount -l "$target" 2>/dev/null || true
        fi
    done
}

case "$ACTION" in
    mount)
        do_mount
        ;;
    umount)
        do_umount
        ;;
    *)
        echo "ERROR: Invalid action '$ACTION'. Use 'mount' or 'umount'."
        exit 1
        ;;
esac
