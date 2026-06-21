use crate::trait_def::Plugin;
use lingmo_core_engine::types::DesktopEnvironment;

pub struct NvidiaPlugin;

impl Plugin for NvidiaPlugin {
    fn name(&self) -> &'static str {
        "nvidia"
    }

    fn description(&self) -> &'static str {
        "NVIDIA proprietary driver support"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["core"]
    }

    fn volume_group(&self) -> Option<&'static str> {
        Some("drivers")
    }

    fn volume_prefixes(&self) -> Vec<&'static str> {
        vec![
            "usr/lib/nvidia",
            "usr/lib/x86_64-linux-gnu/nvidia",
            "usr/share/nvidia",
            "usr/src/nvidia",
            "etc/modprobe.d/nvidia",
            "etc/OpenCL",
            "etc/glvnd",
            "etc/ld.so.conf.d/nvidia",
            "var/lib/nvidia",
        ]
    }

    fn supported_desktops(&self) -> Option<Vec<DesktopEnvironment>> {
        Some(vec![
            DesktopEnvironment::KDE,
            DesktopEnvironment::GNOME,
            DesktopEnvironment::XFCE,
            DesktopEnvironment::Cinnamon,
            DesktopEnvironment::Budgie,
            DesktopEnvironment::Sway,
            DesktopEnvironment::Hyprland,
        ])
    }

    fn packages(&self) -> Vec<&'static str> {
        vec![
            "nvidia-detect",
            "nvidia-driver",
            "nvidia-settings",
            "nvidia-xconfig",
            "nvidia-persistenced",
            "firmware-misc-nonfree",
            "glx-alternative-nvidia",
            "libgl1-nvidia-glvnd-glx",
            "libnvidia-egl-wayland1",
            "libnvidia-egl-gbm1",
            "nvidia-vulkan-common",
            "nvidia-vulkan-icd",
            "nvidia-suspend-common",
            "nvidia-powerd",
        ]
    }

    fn chroot_hooks(&self) -> Vec<String> {
        vec![
            r#"#!/bin/bash
# Configure NVIDIA DRM kernel mode setting
if [ -d /etc/modprobe.d ]; then
    echo "options nvidia-drm modeset=1" > /etc/modprobe.d/nvidia-drm.conf
    echo "options nvidia NVreg_PreserveVideoMemoryAllocations=1" > /etc/modprobe.d/nvidia-power.conf
fi

# Enable persistence daemon
systemctl enable nvidia-persistenced
"#
            .into(),
        ]
    }
}
