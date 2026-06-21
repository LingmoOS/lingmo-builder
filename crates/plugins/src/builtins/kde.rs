use crate::trait_def::Plugin;
use lingmo_core_engine::types::DesktopEnvironment;

pub struct KdePlugin;

impl Plugin for KdePlugin {
    fn name(&self) -> &'static str {
        "kde"
    }

    fn description(&self) -> &'static str {
        "KDE Plasma desktop environment"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["core", "networkmanager"]
    }

    fn volume_group(&self) -> Option<&'static str> {
        Some("desktop")
    }

    fn volume_prefixes(&self) -> Vec<&'static str> {
        vec![
            "usr/share/kde",
            "usr/share/plasma",
            "usr/share/sddm",
            "usr/share/kate",
            "usr/share/dolphin",
            "usr/share/konsole",
            "usr/share/kcalc",
            "usr/share/okular",
            "usr/share/gwenview",
            "usr/share/spectacle",
            "usr/share/elisa",
            "usr/share/korganizer",
            "usr/share/kdeconnect",
            "usr/share/kwin",
            "usr/lib/kde",
            "usr/lib/x86_64-linux-gnu/qt5/plugins/kde",
            "usr/lib/x86_64-linux-gnu/qt6/plugins/kde",
            "etc/sddm",
            "etc/kde",
            "etc/xdg/plasma",
            "etc/xdg/kde",
            "var/lib/sddm",
        ]
    }

    fn supported_desktops(&self) -> Option<Vec<DesktopEnvironment>> {
        Some(vec![DesktopEnvironment::KDE])
    }

    fn packages(&self) -> Vec<&'static str> {
        vec![
            "plasma-desktop",
            "plasma-nm",
            "plasma-pa",
            "plasma-discover",
            "plasma-systemmonitor",
            "plasma-workspace",
            "plasma-workspace-wayland",
            "kde-config-gtk-style",
            "kde-config-gtk-style-preview",
            "kdeconnect",
            "dolphin",
            "konsole",
            "kate",
            "gwenview",
            "okular",
            "spectacle",
            "kcalc",
            "kwrite",
            "ark",
            "elisa",
            "korganizer",
            "kaddressbook",
            "kmail",
            "ktorrent",
            "kdenlive",
            "krita",
            "sddm",
            "sddm-theme-breeze",
            "plasma-desktopthemes-glob",
            "kwin-x11",
            "kwin-wayland",
            "plasma-integration",
            "qt5-integration",
            "qt6-integration",
            "breeze",
            "breeze-gtk-theme",
            "breeze-icon-theme",
            "phonon-backend-vlc",
            "pulseaudio",
            "pulseaudio-module-bluetooth",
            "pavucontrol",
            "pipewire-pulse",
            "wireplumber",
            "bluedevil",
            "print-manager",
            "user-manager",
        ]
    }

    fn chroot_hooks(&self) -> Vec<String> {
        vec![
            r#"#!/bin/bash
systemctl enable sddm
systemctl set-default graphical.target
"#
            .into(),
        ]
    }
}
