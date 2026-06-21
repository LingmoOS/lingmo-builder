use crate::trait_def::Plugin;
use lingmo_core_engine::types::DesktopEnvironment;

pub struct XfcePlugin;

impl Plugin for XfcePlugin {
    fn name(&self) -> &'static str {
        "xfce"
    }

    fn description(&self) -> &'static str {
        "Xfce lightweight desktop environment"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["core", "networkmanager"]
    }

    fn volume_group(&self) -> Option<&'static str> {
        Some("desktop")
    }

    fn volume_prefixes(&self) -> Vec<&'static str> {
        vec![
            "usr/share/xfce",
            "usr/share/xfce4",
            "usr/share/thunar",
            "usr/share/lightdm",
            "usr/share/gtksourceview",
            "usr/lib/xfce4",
            "etc/xfce",
            "etc/lightdm",
            "etc/xdg/xfce4",
        ]
    }

    fn supported_desktops(&self) -> Option<Vec<DesktopEnvironment>> {
        Some(vec![
            DesktopEnvironment::XFCE,
            DesktopEnvironment::LXQT,
        ])
    }

    fn packages(&self) -> Vec<&'static str> {
        vec![
            "xfce4",
            "xfce4-goodies",
            "xfce4-power-manager",
            "xfce4-screenshooter",
            "xfce4-whiskermenu-plugin",
            "xfce4-clipman-plugin",
            "xfce4-notifyd",
            "xfce4-pulseaudio-plugin",
            "xfce4-taskmanager",
            "xfce4-terminal",
            "xfce4-settings",
            "thunar",
            "thunar-archive-plugin",
            "thunar-volman",
            "thunar-media-tags-plugin",
            "tumbler",
            "mousepad",
            "ristretto",
            "parole",
            "orage",
            "xfburn",
            "gigolo",
            "lightdm",
            "lightdm-gtk-greeter",
            "lightdm-gtk-greeter-settings",
            "gtk3-engines-xfce",
            "greybird-gtk-theme",
            "numix-gtk-theme",
            "pulseaudio",
            "pavucontrol",
            "pipewire-pulse",
            "wireplumber",
        ]
    }

    fn chroot_hooks(&self) -> Vec<String> {
        vec![
            r#"#!/bin/bash
systemctl enable lightdm
systemctl set-default graphical.target
"#
            .into(),
        ]
    }
}
