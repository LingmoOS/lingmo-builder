use crate::trait_def::Plugin;

pub struct NetworkManagerPlugin;

impl Plugin for NetworkManagerPlugin {
    fn name(&self) -> &'static str {
        "networkmanager"
    }

    fn description(&self) -> &'static str {
        "NetworkManager for network configuration"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["core"]
    }

    fn packages(&self) -> Vec<&'static str> {
        vec![
            "network-manager",
            "network-manager-gnome",
            "modemmanager",
            "mobile-broadband-provider-info",
            "usb-modeswitch",
            "usb-modeswitch-data",
            "wireless-tools",
            "rfkill",
            "iw",
            "crda",
        ]
    }

    fn chroot_hooks(&self) -> Vec<String> {
        vec![
            r#"#!/bin/bash
systemctl enable NetworkManager
systemctl enable ModemManager
systemctl disable systemd-networkd
"#
            .into(),
        ]
    }
}
