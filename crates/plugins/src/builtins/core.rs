use crate::trait_def::Plugin;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn name(&self) -> &'static str {
        "core"
    }

    fn description(&self) -> &'static str {
        "Essential base system configuration and packages"
    }

    fn packages(&self) -> Vec<&'static str> {
        vec![
            "systemd",
            "systemd-sysv",
            "udev",
            "dbus",
            "apt",
            "apt-utils",
            "bash-completion",
            "command-not-found",
            "less",
            "man-db",
            "manpages",
            "info",
            "nano",
            "vim-tiny",
            "iproute2",
            "iputils-ping",
            "netcat-openbsd",
            "openssh-client",
            "ca-certificates",
            "curl",
            "wget",
            "gnupg",
            "lsb-release",
            "sudo",
            "adduser",
            "passwd",
            "locales",
            "tzdata",
            "keyboard-configuration",
            "console-setup",
            "whiptail",
            "dialog",
            "python3",
            "python3-apt",
            "perl-base",
        ]
    }

    fn chroot_hooks(&self) -> Vec<String> {
        vec![
            // Enable essential system services
            r#"#!/bin/bash
systemctl enable systemd-networkd
systemctl enable systemd-resolved
systemctl enable systemd-timesyncd
systemctl enable systemd-journald
ln -sf /run/systemd/resolve/stub-resolv.conf /etc/resolv.conf
"#
            .into(),
        ]
    }
}
