use crate::trait_def::Plugin;

pub struct MinimalPlugin;

impl Plugin for MinimalPlugin {
    fn name(&self) -> &'static str {
        "minimal"
    }

    fn description(&self) -> &'static str {
        "Minimal system profile for container/embedded use"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["core"]
    }

    fn packages(&self) -> Vec<&'static str> {
        vec![
            "busybox",
            "dropbear",
            "e2fsprogs",
            "f2fs-tools",
            "dosfstools",
            "squashfs-tools",
            "procps",
            "psmisc",
            "grep",
            "findutils",
            "sed",
            "gawk",
            "file",
            "tar",
            "gzip",
            "bzip2",
            "xz-utils",
            "zstd",
            "kmod",
            "initramfs-tools",
            "squashfs-tools",
        ]
    }

    fn chroot_hooks(&self) -> Vec<String> {
        vec![
            r#"#!/bin/bash
# Minimal system tuning
systemctl mask systemd-udevd systemd-journald
echo "LINGMO_MINIMAL" > /etc/hostname
"#
            .into(),
        ]
    }
}
