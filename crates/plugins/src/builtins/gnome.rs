use crate::trait_def::Plugin;
use lingmo_core_engine::types::DesktopEnvironment;

pub struct GnomePlugin;

impl Plugin for GnomePlugin {
    fn name(&self) -> &'static str {
        "gnome"
    }

    fn description(&self) -> &'static str {
        "GNOME desktop environment"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["core", "networkmanager"]
    }

    fn volume_group(&self) -> Option<&'static str> {
        Some("desktop")
    }

    fn volume_prefixes(&self) -> Vec<&'static str> {
        vec![
            "usr/share/gnome",
            "usr/share/gdm",
            "usr/share/nautilus",
            "usr/share/gnome-shell",
            "usr/share/gnome-control-center",
            "usr/share/gnome-software",
            "usr/lib/gnome",
            "usr/lib/gdm",
            "etc/gdm3",
            "etc/gnome",
            "etc/dconf",
        ]
    }

    fn supported_desktops(&self) -> Option<Vec<DesktopEnvironment>> {
        Some(vec![DesktopEnvironment::GNOME])
    }

    fn packages(&self) -> Vec<&'static str> {
        vec![
            "gnome-shell",
            "gnome-session",
            "gnome-terminal",
            "nautilus",
            "gnome-software",
            "gnome-software-plugin-flatpak",
            "gnome-control-center",
            "gnome-tweaks",
            "gnome-shell-extensions",
            "gnome-calculator",
            "gnome-calendar",
            "gnome-contacts",
            "gnome-logs",
            "gnome-maps",
            "gnome-music",
            "gnome-photos",
            "gnome-weather",
            "gnome-clocks",
            "gnome-characters",
            "gnome-font-viewer",
            "gnome-screenshot",
            "gnome-system-monitor",
            "gnome-disk-utility",
            "gnome-keyring",
            "eog",
            "evince",
            "totem",
            "gedit",
            "file-roller",
            "baobab",
            "cheese",
            "simple-scan",
            "gdm3",
            "adwaita-icon-theme",
            "adwaita-qt",
            "yelp",
            "pulseaudio",
            "pulseaudio-module-bluetooth",
            "pipewire-pulse",
            "wireplumber",
            "pavucontrol",
            "orca",
            "firefox-esr",
            "libreoffice-gnome",
        ]
    }

    fn chroot_hooks(&self) -> Vec<String> {
        vec![
            r#"#!/bin/bash
systemctl enable gdm3
systemctl set-default graphical.target
"#
            .into(),
        ]
    }
}
