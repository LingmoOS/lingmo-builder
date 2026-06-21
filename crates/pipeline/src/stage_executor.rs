use lingmo_core_engine::error::BuildResult;
use lingmo_core_engine::types::{BuildContext, Profile};
use lingmo_core_engine::{
    copy_recursive, ensure_root, run_command, run_command_with_input, write_file, BuildError,
};
use lingmo_iso_builder::build_iso;
use lingmo_plugins::registry::PluginRegistry;
use lingmo_repository_manager::deb822::{Deb822Source, DebianSourceEntry};
use lingmo_repository_manager::keyring::{download_and_dearmor_key, RepositoryKey};
use lingmo_volume_manager::strategy::{VolumeConfig, VolumeStrategy};
use lingmo_volume_manager::{build_volumes, split_volumes, FileCatalog, ManifestConfigRef, VolumeGroup};

use crate::stage::Stage;

pub struct StageExecutor {
    ctx: BuildContext,
    registry: PluginRegistry,
}

impl StageExecutor {
    pub fn new(ctx: BuildContext, registry: PluginRegistry) -> Self {
        StageExecutor { ctx, registry }
    }

    pub fn execute(&self, stage: Stage) -> BuildResult<()> {
        tracing::info!("Starting stage: {} ({})", stage, stage.description());

        if stage.requires_root() {
            ensure_root()?;
        }

        match stage {
            Stage::Init => self.stage_init(),
            Stage::Bootstrap => self.stage_bootstrap(),
            Stage::ConfigureDebianRepos => self.stage_configure_debian_repos(),
            Stage::ConfigureExtraRepos => self.stage_configure_extra_repos(),
            Stage::InstallBase => self.stage_install_base(),
            Stage::InstallKernel => self.stage_install_kernel(),
            Stage::InstallFirmware => self.stage_install_firmware(),
            Stage::ApplyProfile => self.stage_apply_profile(),
            Stage::InstallDesktop => self.stage_install_desktop(),
            Stage::AdditionalPackages => self.stage_additional_packages(),
            Stage::FilesystemOverlays => self.stage_filesystem_overlays(),
            Stage::ChrootHooks => self.stage_chroot_hooks(),
            Stage::SystemConfig => self.stage_system_config(),
            Stage::InstallBootloader => self.stage_install_bootloader(),
            Stage::GenerateSquashfsVolumes => self.stage_generate_squashfs_volumes(),
            Stage::GenerateIso => self.stage_generate_iso(),
            Stage::Cleanup => self.stage_cleanup(),
        }?;

        tracing::info!("Completed stage: {}", stage);
        Ok(())
    }

    fn stage_init(&self) -> BuildResult<()> {
        let work_dir = &self.ctx.output.work_dir;
        let output_dir = &self.ctx.output.output_dir;

        for dir in &[
            work_dir,
            output_dir,
            &self.ctx.rootfs_dir(),
            &self.ctx.overlay_dir(),
            &self.ctx.iso_dir(),
            &work_dir.join("apt-cache"),
            &work_dir.join("hooks"),
            &work_dir.join("squashfs-volumes"),
        ] {
            std::fs::create_dir_all(dir).map_err(|e| BuildError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?;
        }

        let checksums = work_dir.join("checksums.json");
        if !checksums.exists() {
            write_file(&checksums, "{}")?;
        }

        tracing::info!("Workspace initialized at {}", work_dir.display());
        tracing::info!("Output directory: {}", output_dir.display());
        Ok(())
    }

    fn stage_bootstrap(&self) -> BuildResult<()> {
        let rootfs = self.ctx.rootfs_dir();
        let distro = &self.ctx.distro;

        if rootfs.join("debootstrap/debootstrap.log").exists() {
            tracing::info!("Rootfs already bootstrapped, skipping");
            return Ok(());
        }

        let variant = match self.ctx.profile {
            Profile::Core => "minbase",
            _ => "buildd",
        };

        let script_path = Self::find_script("debootstrap.sh")?;

        let args = vec![
            rootfs.to_str().unwrap(),
            &distro.codename,
            &distro.mirror,
            &distro.architecture,
            variant,
        ];

        run_command(
            &script_path,
            &args.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
            "Bootstrapping Debian rootfs",
        )?;

        Ok(())
    }

    fn stage_configure_debian_repos(&self) -> BuildResult<()> {
        let rootfs = self.ctx.rootfs_dir();
        let repos = &self.ctx.repositories;
        let debian = &repos.debian;

        // Remove legacy sources.list
        let legacy = rootfs.join("etc/apt/sources.list");
        if legacy.exists() {
            std::fs::remove_file(&legacy).map_err(|e| BuildError::Io {
                path: legacy,
                source: e,
            })?;
        }

        // Generate debian.sources in DEB822 format
        let mut sources = DebianSourceEntry::new();

        // Main repository
        let mut suites = vec![debian.suite.clone()];
        suites.push(format!("{}-updates", debian.suite));
        suites.push(format!("{}-backports", debian.suite));

        sources.add(Deb822Source {
            types: vec!["deb".into()],
            uris: vec![debian.mirror.clone()],
            suites,
            components: debian.components.clone(),
            signed_by: Some("/usr/share/keyrings/debian-archive-keyring.gpg".into()),
            architectures: None,
        });

        // Security repository
        sources.add(Deb822Source {
            types: vec!["deb".into()],
            uris: vec![debian.security_mirror.clone()],
            suites: vec![format!("{}-security", debian.suite)],
            components: debian.components.clone(),
            signed_by: Some("/usr/share/keyrings/debian-archive-keyring.gpg".into()),
            architectures: None,
        });

        // Source repositories if enabled
        if debian.source_enabled {
            let mut src_suites = vec![debian.suite.clone()];
            src_suites.push(format!("{}-updates", debian.suite));

            sources.add(Deb822Source {
                types: vec!["deb-src".into()],
                uris: vec![debian.mirror.clone()],
                suites: src_suites,
                components: debian.components.clone(),
                signed_by: Some("/usr/share/keyrings/debian-archive-keyring.gpg".into()),
                architectures: None,
            });
        }

        let sources_dir = rootfs.join("etc/apt/sources.list.d");
        std::fs::create_dir_all(&sources_dir).map_err(|e| BuildError::Io {
            path: sources_dir.clone(),
            source: e,
        })?;

        write_file(&sources_dir.join("debian.sources"), &sources.to_string())?;

        // Write APT pin priorities
        if !self.ctx.packages.pin_priorities.is_empty() {
            let mut pref = String::new();
            for (pkg, priority) in &self.ctx.packages.pin_priorities {
                pref.push_str(&format!(
                    "Package: {}\nPin: release a={}\nPin-Priority: {}\n\n",
                    pkg, debian.suite, priority
                ));
            }
            let prefs_dir = rootfs.join("etc/apt/preferences.d");
            std::fs::create_dir_all(&prefs_dir).map_err(|e| BuildError::Io {
                path: prefs_dir.clone(),
                source: e,
            })?;
            write_file(&prefs_dir.join("lingmo"), &pref)?;
        }

        tracing::info!("Debian repositories configured (DEB822 format)");
        Ok(())
    }

    fn stage_configure_extra_repos(&self) -> BuildResult<()> {
        let rootfs = self.ctx.rootfs_dir();

        for extra in &self.ctx.repositories.extra {
            if !extra.enabled {
                tracing::info!("Skipping disabled repository: {}", extra.name);
                continue;
            }

            tracing::info!("Configuring extra repository: {}", extra.name);

            // Download and install GPG key if key_url is provided
            if let Some(ref key_url) = extra.key_url {
                let default_key_path = format!("/etc/apt/keyrings/{}.gpg", extra.name);
                let key_path = extra.key_path.as_deref().unwrap_or(&default_key_path);

                let key = RepositoryKey {
                    url: key_url.clone(),
                    dest_path: key_path.to_string(),
                };
                download_and_dearmor_key(&key, &rootfs)?;
            }

            // Generate .sources file in DEB822 format
            let signed_by = extra.key_path.as_deref().map(|k| {
                if k.starts_with('/') {
                    k.to_string()
                } else {
                    format!("/etc/apt/keyrings/{}", k)
                }
            });

            let source_entry = Deb822Source {
                types: vec!["deb".into()],
                uris: vec![extra.url.clone()],
                suites: vec![extra.suite.clone()],
                components: extra.components.clone(),
                signed_by,
                architectures: None,
            };

            let sources_dir = rootfs.join("etc/apt/sources.list.d");
            std::fs::create_dir_all(&sources_dir).map_err(|e| BuildError::Io {
                path: sources_dir.clone(),
                source: e,
            })?;

            write_file(
                &sources_dir.join(format!("{}.sources", extra.name)),
                &source_entry.to_string(),
            )?;

            tracing::info!("Repository {} configured", extra.name);
        }

        // Run apt update after all repositories are configured
        self.chroot_exec(&["apt-get", "update", "--quiet=2"], "Updating APT cache")?;

        tracing::info!("Extra repositories configured and APT cache updated");
        Ok(())
    }

    fn stage_install_base(&self) -> BuildResult<()> {
        let mut packages = vec![
            "systemd",
            "systemd-sysv",
            "udev",
            "dbus",
            "apt",
            "apt-utils",
            "dpkg",
            "bash",
            "coreutils",
            "util-linux",
            "mount",
            "e2fsprogs",
            "ca-certificates",
            "wget",
            "curl",
            "locales",
            "tzdata",
            "keyboard-configuration",
            "console-setup",
            "sudo",
            "adduser",
            "passwd",
            "whiptail",
            "dialog",
            "policykit-1",
            "polkitd",
        ];

        packages.extend(self.ctx.packages.base.iter().map(|s| s.as_str()));

        let packages_str = packages.join(" ");
        self.apt_install(&packages_str, "Installing base system packages")
    }

    fn stage_install_kernel(&self) -> BuildResult<()> {
        let arch = &self.ctx.distro.architecture;
        let kernel_pkg = format!("linux-image-{}", arch);
        let headers_pkg = format!("linux-headers-{}", arch);

        self.apt_install(
            &format!("{} {} firmware-linux", kernel_pkg, headers_pkg),
            "Installing Linux kernel and firmware",
        )
    }

    fn stage_install_firmware(&self) -> BuildResult<()> {
        let firmware_packages = vec![
            "firmware-linux",
            "firmware-linux-nonfree",
            "firmware-misc-nonfree",
            "firmware-amd-graphics",
            "firmware-intel-sound",
            "firmware-iwlwifi",
            "firmware-realtek",
            "firmware-bnx2",
            "firmware-bnx2x",
            "firmware-cxgb3",
            "firmware-cxgb4",
            "firmware-myricom",
            "firmware-netxen",
            "firmware-qlogic",
            "firmware-qlgen",
            "firmware-samsung",
            "firmware-siano",
            "firmware-ti-connectivity",
            "firmware-zd1211",
        ];

        self.apt_install(&firmware_packages.join(" "), "Installing firmware packages")
    }

    fn stage_apply_profile(&self) -> BuildResult<()> {
        let profile_packages = match self.ctx.profile {
            Profile::Desktop => vec![
                "task-desktop",
                "task-english",
                "xorg",
                "xserver-xorg",
                "xserver-xorg-video-all",
                "xserver-xorg-input-all",
                "x11-utils",
                "x11-xserver-utils",
                "desktop-base",
                "desktop-file-utils",
                "shared-mime-info",
                "xdg-utils",
                "xdg-user-dirs",
                "fonts-dejavu-core",
                "fonts-noto",
                "fonts-liberation",
                "pulseaudio",
                "pipewire-pulse",
                "wireplumber",
                "bluez",
                "bluetooth",
                "cups",
                "cups-bsd",
                "cups-client",
                "avahi-daemon",
                "network-manager",
                "network-manager-gnome",
                "software-properties-common",
                "gnome-software",
                "flatpak",
                "firefox-esr",
                "libreoffice",
                "gimp",
                "vlc",
                "celluloid",
                "thunderbird",
                "file-roller",
                "evince",
                "eog",
                "totem",
                "rhythmbox",
            ],
            Profile::Server => vec![
                "task-server",
                "openssh-server",
                "openssh-client",
                "fail2ban",
                "ufw",
                "rsync",
                "vim",
                "tmux",
                "htop",
                "iotop",
                "net-tools",
                "dnsutils",
                "tcpdump",
                "iptables",
                "nftables",
                "postfix",
                "logrotate",
                "unattended-upgrades",
                "apticron",
                "needrestart",
                "etckeeper",
                "git",
                "ethtool",
                "lvm2",
                "mdadm",
                "smartmontools",
            ],
            Profile::Core => vec!["systemd", "systemd-sysv", "apt", "bash", "coreutils"],
        };

        self.apt_install(
            &profile_packages.join(" "),
            &format!("Applying {} profile packages", self.ctx.profile),
        )
    }

    fn stage_install_desktop(&self) -> BuildResult<()> {
        let de = match &self.ctx.desktop {
            Some(de) => de,
            None => {
                tracing::info!("No desktop environment selected, skipping");
                return Ok(());
            }
        };

        let de_packages = match de {
            lingmo_core_engine::DesktopEnvironment::KDE => vec![
                "task-kde-desktop",
                "sddm",
                "plasma-desktop",
                "plasma-nm",
                "plasma-pa",
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
                "ktorrent",
            ],
            lingmo_core_engine::DesktopEnvironment::GNOME => vec![
                "task-gnome-desktop",
                "gdm3",
                "gnome-shell",
                "gnome-session",
                "gnome-terminal",
                "nautilus",
                "gnome-software",
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
                "eog",
                "evince",
                "totem",
                "gedit",
                "file-roller",
                "baobab",
                "cheese",
                "simple-scan",
            ],
            lingmo_core_engine::DesktopEnvironment::XFCE => vec![
                "task-xfce-desktop",
                "lightdm",
                "xfce4",
                "xfce4-goodies",
                "xfce4-power-manager",
                "xfce4-screenshooter",
                "xfce4-whiskermenu-plugin",
                "xfce4-clipman-plugin",
                "xfce4-notifyd",
                "thunar",
                "thunar-archive-plugin",
                "thunar-volman",
                "mousepad",
                "ristretto",
                "parole",
                "orage",
                "xfburn",
                "gigolo",
            ],
            lingmo_core_engine::DesktopEnvironment::LXQT => vec![
                "lxqt",
                "lxqt-core",
                "lxqt-panel",
                "lxqt-session",
                "lxqt-runner",
                "lxqt-powermanagement",
                "lxqt-notificationd",
                "lxqt-policykit",
                "lxqt-qtplugin",
                "lxqt-config",
                "lxqt-about",
                "lxqt-admin",
                "lxqt-openssh-askpass",
                "lxqt-sudo",
                "sddm",
                "pavucontrol-qt",
                "qterminal",
                "qalculate-gtk",
                "featherpad",
                "pcmanfm-qt",
            ],
            lingmo_core_engine::DesktopEnvironment::MATE => vec![
                "task-mate-desktop",
                "lightdm",
                "mate-desktop-environment",
                "mate-desktop-environment-extras",
                "mate-terminal",
                "caja",
                "pluma",
                "eom",
                "atril",
                "engrampa",
                "mate-calc",
                "mate-system-monitor",
                "mate-power-manager",
                "mate-screensaver",
                "mate-tweak",
            ],
            lingmo_core_engine::DesktopEnvironment::Cinnamon => vec![
                "task-cinnamon-desktop",
                "lightdm",
                "cinnamon",
                "cinnamon-core",
                "cinnamon-desktop-data",
                "cinnamon-screensaver",
                "cinnamon-control-center",
                "cinnamon-settings-daemon",
                "nemo",
                "nemo-fileroller",
                "xed",
                "xviewer",
                "xreader",
                "pix",
                "rhythmbox",
            ],
            lingmo_core_engine::DesktopEnvironment::Budgie => vec![
                "budgie-desktop",
                "budgie-indicator-applet",
                "budgie-window-buttons",
                "budgie-workspace-stopwatch",
                "budgie-countdown-applet",
                "budgie-quicknotes",
                "budgie-hotcorners",
                "budgie-showtime",
                "budgie-takeabreak",
                "budgie-weathershow",
                "budgie-virtualkeyboard",
                "gdm3",
                "gnome-terminal",
                "nautilus",
                "gnome-control-center",
                "gnome-software",
            ],
            lingmo_core_engine::DesktopEnvironment::Sway => vec![
                "sway",
                "swaybg",
                "swayidle",
                "swaylock",
                "waybar",
                "wofi",
                "mako-notifier",
                "foot",
                "grim",
                "slurp",
                "wl-clipboard",
                "brightnessctl",
                "pavucontrol",
                "network-manager-gnome",
                "thunar",
                "pcmanfm-qt",
                "qt5-wayland",
                "qt6-wayland",
                "xdg-desktop-portal-wlr",
            ],
            lingmo_core_engine::DesktopEnvironment::Hyprland => vec![
                "hyprland",
                "hyprpaper",
                "hypridle",
                "hyprlock",
                "waybar",
                "wofi",
                "dunst",
                "kitty",
                "foot",
                "grim",
                "slurp",
                "wl-clipboard",
                "brightnessctl",
                "pavucontrol",
                "network-manager-gnome",
                "thunar",
                "qt5-wayland",
                "qt6-wayland",
                "xdg-desktop-portal-hyprland",
                "polkit-kde-agent",
            ],
            lingmo_core_engine::DesktopEnvironment::None => {
                tracing::info!("No desktop environment selected, skipping");
                return Ok(());
            }
        };

        self.apt_install(
            &de_packages.join(" "),
            &format!("Installing {} desktop environment", de),
        )
    }

    fn stage_additional_packages(&self) -> BuildResult<()> {
        if self.ctx.packages.additional.is_empty() {
            tracing::info!("No additional packages specified, skipping");
            return Ok(());
        }

        let packages = self.ctx.packages.additional.join(" ");
        self.apt_install(&packages, "Installing additional packages")
    }

    fn stage_filesystem_overlays(&self) -> BuildResult<()> {
        let overlay_dir = self.ctx.overlay_dir();
        let rootfs = self.ctx.rootfs_dir();

        if !overlay_dir.exists() {
            tracing::info!("No overlay directory found, skipping");
            return Ok(());
        }

        for entry in std::fs::read_dir(&overlay_dir).map_err(|e| BuildError::Io {
            path: overlay_dir.clone(),
            source: e,
        })? {
            let entry = entry.map_err(|e| BuildError::Io {
                path: overlay_dir.clone(),
                source: e,
            })?;
            let path = entry.path();
            let dest = rootfs.join(entry.file_name());
            if path.is_dir() {
                copy_recursive(&path, &dest)?;
            } else {
                std::fs::copy(&path, &dest).map_err(|e| BuildError::Io {
                    path: dest,
                    source: e,
                })?;
            }
        }

        // Apply plugin overlays
        for plugin in self.registry.plugins() {
            for overlay in plugin.overlays() {
                if overlay.exists() {
                    let dest = rootfs.join(overlay.strip_prefix("/").unwrap_or(&overlay));
                    copy_recursive(&overlay, &dest)?;
                    tracing::debug!(
                        "Plugin '{}' applied overlay: {} -> {}",
                        plugin.name(),
                        overlay.display(),
                        dest.display()
                    );
                }
            }
        }

        Ok(())
    }

    fn stage_chroot_hooks(&self) -> BuildResult<()> {
        // Run plugin chroot hooks
        for plugin in self.registry.plugins() {
            for (i, hook_script) in plugin.chroot_hooks().iter().enumerate() {
                let hook_name = format!("{}-hook-{}.sh", plugin.name(), i);
                let hook_path = self.ctx.rootfs_dir().join("tmp").join(&hook_name);
                write_file(&hook_path, hook_script)?;

                self.chroot_exec_as_cmd(
                    &format!("bash /tmp/{}", hook_name),
                    &format!("Running plugin hook '{}'", hook_name),
                )?;

                std::fs::remove_file(&hook_path).map_err(|e| BuildError::Io {
                    path: hook_path,
                    source: e,
                })?;
            }
        }

        // Run user-defined hooks that are marked for chroot execution
        for hook in &self.ctx.hooks {
            if !hook.chroot {
                continue;
            }
            let hook_path = self.ctx.rootfs_dir().join("tmp").join(&hook.name);
            write_file(&hook_path, &hook.script)?;

            self.chroot_exec_as_cmd(
                &format!("bash /tmp/{}", hook.name),
                &format!("Running user hook '{}'", hook.name),
            )?;

            std::fs::remove_file(&hook_path).map_err(|e| BuildError::Io {
                path: hook_path,
                source: e,
            })?;
        }

        Ok(())
    }

    fn stage_system_config(&self) -> BuildResult<()> {
        let rootfs = self.ctx.rootfs_dir();
        let sys = &self.ctx.system;

        write_file(
            &rootfs.join("etc/hostname"),
            &format!("{}\n", sys.hostname),
        )?;

        write_file(
            &rootfs.join("etc/hosts"),
            &format!(
                "127.0.0.1\tlocalhost\n127.0.1.1\t{}\n\n::1\t\tlocalhost ip6-localhost ip6-loopback\nff02::1\t\tip6-allnodes\nff02::2\t\tip6-allrouters\n",
                sys.hostname
            ),
        )?;

        write_file(
            &rootfs.join("etc/timezone"),
            &format!("{}\n", sys.timezone),
        )?;

        self.chroot_exec(
            &["dpkg-reconfigure", "-f", "noninteractive", "tzdata"],
            "Configuring timezone",
        )?;

        // Locale
        let locale_gen = format!("{}\n", sys.locale);
        write_file(&rootfs.join("etc/locale.gen"), &locale_gen)?;
        self.chroot_exec(
            &["locale-gen"],
            "Generating system locales",
        )?;

        let default_locale = format!("LANG={}\n", sys.locale);
        write_file(&rootfs.join("etc/default/locale"), &default_locale)?;

        // Keyboard layout
        let keyboard_content = format!(
            "XKBMODEL=\"pc105\"\nXKBLAYOUT=\"{}\"\nXKBVARIANT=\"\"\nXKBOPTIONS=\"\"\nBACKSPACE=\"guess\"\n",
            sys.keyboard_layout
        );
        write_file(
            &rootfs.join("etc/default/keyboard"),
            &keyboard_content,
        )?;

        // Create users
        for user in &sys.users {
            self.chroot_exec(
                &[
                    "useradd",
                    "-m",
                    "-s",
                    &user.shell,
                    "-G",
                    &user.groups.join(","),
                    &user.username,
                ],
                &format!("Creating user '{}'", user.username),
            )?;

            if user.sudo {
                let sudoers_path = rootfs.join(format!("etc/sudoers.d/{}", user.username));
                write_file(
                    &sudoers_path,
                    &format!("{} ALL=(ALL:ALL) ALL\n", user.username),
                )?;
                std::fs::set_permissions(
                    &sudoers_path,
                    std::os::unix::fs::PermissionsExt::from_mode(0o440),
                )
                .map_err(|e| BuildError::Io {
                    path: sudoers_path,
                    source: e,
                })?;
            }
        }

        // Set root password hash if provided
        if let Some(root) = sys.users.iter().find(|u| u.username == "root") {
            self.chroot_exec_with_input(
                &["chpasswd", "-e"],
                &format!("root:{}", root.password_hash),
                "Setting root password",
            )?;
        }

        // Write fstab
        if !sys.fstab_entries.is_empty() {
            let fstab_content = sys.fstab_entries.join("\n") + "\n";
            write_file(&rootfs.join("etc/fstab"), &fstab_content)?;
        }

        // Remove machine-id (will be generated on first boot)
        let machine_id = rootfs.join("etc/machine-id");
        if machine_id.exists() {
            std::fs::remove_file(&machine_id).map_err(|e| BuildError::Io {
                path: machine_id.clone(),
                source: e,
            })?;
        }
        write_file(&machine_id, "")?;

        // Set kernel cmdline
        if !sys.kernel_cmdline.is_empty() {
            let cmdline_path = rootfs.join("etc/kernel/cmdline");
            write_file(&cmdline_path, &format!("{}\n", sys.kernel_cmdline))?;
        }

        Ok(())
    }

    fn stage_install_bootloader(&self) -> BuildResult<()> {
        let rootfs = self.ctx.rootfs_dir();
        let script_path = Self::find_script("grub-install.sh")?;

        let iso_dir = self.ctx.iso_dir();
        let boot_dir = iso_dir.join("boot");
        let efi_dir = iso_dir.join("EFI");

        std::fs::create_dir_all(&boot_dir).map_err(|e| BuildError::Io {
            path: boot_dir.clone(),
            source: e,
        })?;
        std::fs::create_dir_all(&efi_dir).map_err(|e| BuildError::Io {
            path: efi_dir.clone(),
            source: e,
        })?;

        let grub_packages = vec![
            "grub-pc",
            "grub-pc-bin",
            "grub-efi-amd64",
            "grub-efi-amd64-bin",
            "grub-efi-amd64-signed",
            "shim-signed",
            "shim-helpers-amd64-signed",
            "mokutil",
            "efibootmgr",
            "mtools",
            "xorriso",
        ];

        self.apt_install(&grub_packages.join(" "), "Installing GRUB packages")?;

        // Mount required filesystems for grub-install in chroot
        self.mount_chroot()?;

        let result = run_command(
            &script_path,
            &[rootfs.to_str().unwrap(), iso_dir.to_str().unwrap()],
            "Installing GRUB bootloader",
        );

        self.umount_chroot()?;

        result.map(|_| ())
    }

    fn stage_generate_squashfs_volumes(&self) -> BuildResult<()> {
        let rootfs = self.ctx.rootfs_dir();
        let work_dir = self.ctx.output.work_dir.join("squashfs-volumes");

        // Build volume config from context
        let strategy = VolumeStrategy::parse_str(&self.ctx.volume.strategy)
            .ok_or_else(|| BuildError::Config(format!(
                "Invalid volume strategy '{}'",
                self.ctx.volume.strategy
            )))?;
        let volume_config = VolumeConfig {
            strategy,
            max_volume_size: self.ctx.volume.max_volume_size,
            compression: self.ctx.volume.compression.clone(),
            block_size: self.ctx.volume.block_size,
            output_pattern: self.ctx.volume.output_pattern.clone(),
            generate_manifest: self.ctx.volume.generate_manifest,
            verify_checksums: self.ctx.volume.verify_checksums,
        };

        // Walk and catalog the rootfs
        let catalog = FileCatalog::from_rootfs(&rootfs)?;

        // Build plugin volume groups for plugin-based splitting
        let volume_groups = self.build_volume_groups();

        // Split files into volumes
        let split = split_volumes(&catalog, &volume_config, &volume_groups)?;

        // Build manifest config reference
        let manifest_config = ManifestConfigRef {
            profile: self.ctx.profile.to_string(),
            desktop: self.ctx.desktop.as_ref().map(|d| d.to_string()),
            architecture: self.ctx.distro.architecture.clone(),
            compression: volume_config.compression.clone(),
            block_size: volume_config.block_size,
        };

        // Build the volumes
        let result = build_volumes(
            &split,
            &volume_config,
            &work_dir,
            &rootfs,
            manifest_config,
        )?;

        // Store volume metadata for ISO stage
        tracing::info!(
            "Generated {} SquashFS volume(s) in {}",
            result.results.len(),
            work_dir.display()
        );
        for vol in &result.results {
            let uncompressed = split.volumes
                .iter()
                .find(|v| v.index == vol.index)
                .map(|v| v.total_size)
                .unwrap_or(0);
            tracing::info!(
                "  Volume {} ({}): {} bytes -> {} bytes compressed",
                vol.index,
                vol.filename,
                uncompressed,
                vol.compressed_size,
            );
        }

        // Verify volume ordering (live-boot handles mounting automatically)
        lingmo_volume_manager::mounter::verify_volume_ordering(&result.manifest)?;

        Ok(())
    }

    fn build_volume_groups(&self) -> Vec<VolumeGroup> {
        let mut groups: Vec<VolumeGroup> = Vec::new();

        for plugin in self.registry.plugins() {
            if let Some(group_name) = plugin.volume_group() {
                let prefixes: Vec<std::path::PathBuf> = plugin
                    .volume_prefixes()
                    .iter()
                    .map(|p| std::path::PathBuf::from(p))
                    .collect();

                if !prefixes.is_empty() {
                    groups.push(VolumeGroup {
                        label: group_name.to_string(),
                        priority: plugin.priority() as usize,
                        prefixes,
                    });
                }
            }
        }

        groups
    }

    fn stage_generate_iso(&self) -> BuildResult<()> {
        let volume_dir = self.ctx.output.work_dir.join("squashfs-volumes");
        let manifest_path = volume_dir.join("filesystem.manifest.json");

        // Check if multi-volume manifest exists
        if manifest_path.exists() {
            tracing::info!("Multi-volume manifest detected, building ISO with all volumes");
            // Copy all volume files to the ISO directory
            let iso_volume_dir = self.ctx.iso_dir().join("live");
            std::fs::create_dir_all(&iso_volume_dir).map_err(|e| BuildError::Io {
                path: iso_volume_dir.clone(),
                source: e,
            })?;

            // Copy all .squashfs files and the manifest
            for entry in std::fs::read_dir(&volume_dir).map_err(|e| BuildError::Io {
                path: volume_dir.clone(),
                source: e,
            })? {
                let entry = entry.map_err(|e| BuildError::Io {
                    path: volume_dir.clone(),
                    source: e,
                })?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "squashfs" || ext == "json") {
                    let dest = iso_volume_dir.join(entry.file_name());
                    std::fs::copy(&path, &dest).map_err(|e| BuildError::Io {
                        path: dest,
                        source: e,
                    })?;
                }
            }

            // Use the first volume as the "primary" one for GRUB detection
            let primary_squashfs = iso_volume_dir.join("filesystem.part1.squashfs");

            build_iso(
                &self.ctx.iso_dir(),
                &primary_squashfs,
                &self.ctx.output,
                &self.ctx.distro,
                &self.ctx.system.kernel_cmdline,
            )
        } else {
            // Fallback to single squashfs
            let squashfs_path = self.ctx.squashfs_path();
            if squashfs_path.exists() {
                build_iso(
                    &self.ctx.iso_dir(),
                    &squashfs_path,
                    &self.ctx.output,
                    &self.ctx.distro,
                    &self.ctx.system.kernel_cmdline,
                )
            } else {
                Err(BuildError::Other(
                    "No squashfs files found. Run generate-squashfs-volumes stage first.".into(),
                ))
            }
        }
    }

    fn stage_cleanup(&self) -> BuildResult<()> {
        let work_dir = &self.ctx.output.work_dir;

        let rootfs = self.ctx.rootfs_dir();
        if rootfs.exists() {
            self.umount_chroot()?;
            std::fs::remove_dir_all(&rootfs).map_err(|e| BuildError::Io {
                path: rootfs,
                source: e,
            })?;
        }

        let iso_dir = self.ctx.iso_dir();
        if iso_dir.exists() {
            std::fs::remove_dir_all(&iso_dir).map_err(|e| BuildError::Io {
                path: iso_dir,
                source: e,
            })?;
        }

        let squashfs = self.ctx.squashfs_path();
        if squashfs.exists() {
            std::fs::remove_file(&squashfs).map_err(|e| BuildError::Io {
                path: squashfs,
                source: e,
            })?;
        }

        tracing::info!("Workspace cleaned: {}", work_dir.display());
        Ok(())
    }

    // -- Helpers --

    fn apt_install(&self, packages: &str, description: &str) -> BuildResult<()> {
        if packages.trim().is_empty() {
            tracing::warn!("No packages specified for: {}", description);
            return Ok(());
        }

        let mut args: Vec<String> = vec![
            "apt-get".to_string(),
            "install".to_string(),
            "--yes".to_string(),
            "--no-install-recommends".to_string(),
            "--quiet=2".to_string(),
            "--option=Dpkg::Options::=--force-confdef".to_string(),
            "--option=Dpkg::Options::=--force-confold".to_string(),
        ];

        for pkg in packages.split_whitespace() {
            let pkg = pkg.trim();
            if !pkg.is_empty() {
                args.push(pkg.to_string());
            }
        }

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        // Wrap in single chroot call via bash -c to handle the full apt-get command
        let cmd = format!(
            "DEBIAN_FRONTEND=noninteractive {}",
            args_refs.join(" ")
        );
        self.chroot_exec_as_cmd(&cmd, description).map(|_| ())
    }

    fn chroot_exec(&self, args: &[&str], description: &str) -> BuildResult<String> {
        let script_path = Self::find_script("chroot-mount.sh")?;
        let rootfs = self.ctx.rootfs_dir().to_str().unwrap().to_string();

        // Mount chroot filesystems first
        run_command(
            &script_path,
            &[&rootfs, "mount"],
            &format!("Mounting chroot filesystems for: {}", description),
        )?;

        let cmd = format!("chroot {} /bin/bash -c '{}'", rootfs, args.join(" "));
        let result = run_command("/bin/bash", &["-c", &cmd], description);

        // Unmount
        run_command(
            &script_path,
            &[&rootfs, "umount"],
            "Unmounting chroot filesystems",
        )?;

        result
    }

    fn chroot_exec_as_cmd(&self, cmd: &str, description: &str) -> BuildResult<String> {
        let script_path = Self::find_script("chroot-mount.sh")?;
        let rootfs = self.ctx.rootfs_dir().to_str().unwrap().to_string();

        run_command(
            &script_path,
            &[&rootfs, "mount"],
            &format!("Mounting chroot filesystems for: {}", description),
        )?;

        let full_cmd = format!("chroot {} /bin/bash -c '{}'", rootfs, cmd);
        let result = run_command("/bin/bash", &["-c", &full_cmd], description);

        run_command(
            &script_path,
            &[&rootfs, "umount"],
            "Unmounting chroot filesystems",
        )?;

        result
    }

    fn chroot_exec_with_input(
        &self,
        args: &[&str],
        input: &str,
        description: &str,
    ) -> BuildResult<String> {
        let script_path = Self::find_script("chroot-mount.sh")?;
        let rootfs = self.ctx.rootfs_dir().to_str().unwrap().to_string();

        run_command(
            &script_path,
            &[&rootfs, "mount"],
            &format!("Mounting chroot filesystems for: {}", description),
        )?;

        let cmd = format!("chroot {} /bin/bash -c '{}'", rootfs, args.join(" "));
        let result = run_command_with_input("/bin/bash", &["-c", &cmd], input, description);

        run_command(
            &script_path,
            &[&rootfs, "umount"],
            "Unmounting chroot filesystems",
        )?;

        result
    }

    fn mount_chroot(&self) -> BuildResult<()> {
        let script_path = Self::find_script("chroot-mount.sh")?;
        let rootfs = self.ctx.rootfs_dir().to_str().unwrap().to_string();
        run_command(
            &script_path,
            &[&rootfs, "mount"],
            "Mounting chroot filesystems",
        )?;
        Ok(())
    }

    fn umount_chroot(&self) -> BuildResult<()> {
        let script_path = Self::find_script("chroot-mount.sh")?;
        let rootfs = self.ctx.rootfs_dir().to_str().unwrap().to_string();
        run_command(
            &script_path,
            &[&rootfs, "umount"],
            "Unmounting chroot filesystems",
        )?;
        Ok(())
    }

    fn find_script(name: &str) -> BuildResult<String> {
        // Search in several locations
        let candidates = vec![
            format!("./scripts/{}", name),
            format!("/usr/share/lingmo-builder/scripts/{}", name),
            format!("/usr/local/share/lingmo-builder/scripts/{}", name),
        ];

        for path in &candidates {
            if std::path::Path::new(path).exists() {
                return Ok(path.clone());
            }
        }

        // Fallback: try to find relative to the binary
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let path = dir.join("scripts").join(name);
                if path.exists() {
                    return Ok(path.to_str().unwrap().to_string());
                }
                // Also check ../share/lingmo-builder/scripts/
                let path = dir
                    .parent()
                    .map(|p| p.join("share").join("lingmo-builder").join("scripts").join(name));
                if let Some(p) = path {
                    if p.exists() {
                        return Ok(p.to_str().unwrap().to_string());
                    }
                }
            }
        }

        Err(BuildError::Other(format!(
            "Script not found: {}. Checked: {:?}",
            name, candidates
        )))
    }
}
