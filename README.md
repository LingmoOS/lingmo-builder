# Lingmo Builder

A modular, multi-stage Debian-based Linux distribution builder written in Rust. Constructs bootable live ISO images from declarative TOML configuration.

## Features

- **Pipeline architecture** — 17 ordered stages from debootstrap to ISO generation
- **Plugin system** — 7 built-in plugins (KDE, GNOME, XFCE, NetworkManager, NVIDIA, minimal) with dependency resolution
- **Multi-volume SquashFS** — Split rootfs across filesystem volumes (size-based, plugin-based, or profile-based strategies)
- **DEB822 repository config** — Modern `.sources` format with GPG keyring management
- **BIOS+UEFI boot** — Hybrid ISO with GRUB bootloader, Calamares or Debian Installer support

## Quick Start

```bash
# Generate a config template
lingmo-builder init -p desktop -d kde

# Build the ISO
sudo lingmo-builder build
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `init` | Generate a TOML configuration template |
| `build` | Build a distribution ISO from config |
| `clean` | Remove build artifacts |
| `list-plugins` | List available built-in plugins |
| `validate` | Validate a configuration file |

## Pipeline Stages (in order)

| # | Stage | Description | Root | Network |
|---|-------|-------------|------|---------|
| 1 | `init` | Create workspace directories | No | No |
| 2 | `bootstrap` | Debootstrap base rootfs | Yes | Yes |
| 3 | `configure-debian-repos` | Write DEB822 `.sources` files | No | No |
| 4 | `configure-extra-repos` | Download GPG keys, write extra `.sources`, apt update | No | Yes |
| 5 | `install-base` | Base system packages (systemd, apt, sudo, etc.) | Yes | Yes |
| 6 | `install-kernel` | Linux kernel + firmware-linux | Yes | Yes |
| 7 | `install-firmware` | 20+ firmware packages | Yes | Yes |
| 8 | `apply-profile` | Profile-specific packages (desktop/server/core) | Yes | Yes |
| 9 | `install-desktop` | DE-specific packages (KDE/GNOME/XFCE/etc.) | Yes | Yes |
| 10 | `additional-packages` | User-specified packages | Yes | Yes |
| 11 | `filesystem-overlays` | Apply overlay directories | Yes | No |
| 12 | `chroot-hooks` | Run shell hooks inside chroot | Yes | No |
| 13 | `system-config` | Hostname, locale, users, fstab, kernel cmdline | Yes | No |
| 14 | `install-bootloader` | GRUB BIOS+UEFI installation | Yes | No |
| 15 | `generate-squashfs-volumes` | Catalog rootfs, split into volumes, build SquashFS | No | No |
| 16 | `generate-iso` | Assemble ISO with xorriso | No | No |
| 17 | `cleanup` | Unmount chroot, remove work files | No | No |

## Configuration

Full example at [`config/example.toml`](config/example.toml).

```toml
[distro]
name = "LingmoOS"
version = "2026.1"
codename = "trixie"
mirror = "https://mirrors.tuna.tsinghua.edu.cn/debian"
architecture = "amd64"
components = ["main", "contrib", "non-free", "non-free-firmware"]

profile = "desktop"
desktop = "kde"

[repositories]
default_format = "deb822"

[repositories.debian]
mirror = "https://mirrors.tuna.tsinghua.edu.cn/debian"
security_mirror = "https://mirrors.tuna.tsinghua.edu.cn/debian-security"
suite = "trixie"
components = ["main", "contrib", "non-free", "non-free-firmware"]

[[repositories.extra]]
name = "lingmo"
repo_type = "obs"
url = "http://download.opensuse.org/repositories/home:/LingmoOS/Debian_13/"
key_url = "https://download.opensuse.org/repositories/home:LingmoOS/Debian_13/Release.key"
key_path = "/etc/apt/keyrings/lingmo.gpg"

[volume]
strategy = "size"
max_volume_size = 1073741824
compression = "zstd"

[output]
iso_name = "lingmo-2026.1-amd64.iso"

[cache]
enabled = true
directory = "./cache"
stages = ["bootstrap", "install-base"]

plugins = ["core", "kde", "networkmanager", "nvidia"]
```

## Volume Splitting Strategies

| Strategy | Behavior |
|----------|----------|
| `single` | One squashfs volume (traditional) |
| `size` | Split by size threshold (default 1 GiB) |
| `plugin` | Per-plugin volume groups (core, desktop, drivers) |
| `profile` | Separate desktop files from core system |

Volumes are named `filesystem.part1.squashfs`, `filesystem.part2.squashfs`, etc. Mounting is handled automatically by Debian live-boot at boot time via overlayfs.

## Plugin System

Plugins inject packages, filesystem overlays, and chroot hooks into the pipeline. Built-in plugins:

| Plugin | Dependencies | Description |
|--------|-------------|-------------|
| `core` | — | Base system (30+ packages) |
| `kde` | core, networkmanager | KDE Plasma desktop |
| `gnome` | core, networkmanager | GNOME desktop |
| `xfce` | core, networkmanager | XFCE desktop |
| `networkmanager` | core | Network management |
| `nvidia` | core | Proprietary NVIDIA drivers |
| `minimal` | core | Minimal packages for embedded/server |

## Architecture

```
lingmo-builder/
├── crates/
│   ├── cli/                  # CLI binary (clap)
│   ├── config-parser/        # TOML parsing & validation
│   ├── core-engine/          # Shared types, errors, utilities
│   ├── iso-builder/          # ISO image assembly (xorriso)
│   ├── pipeline/             # Stage orchestration (17 stages)
│   ├── plugins/              # Plugin trait + 7 builtins
│   ├── repository-manager/   # DEB822 format + GPG keyring
│   └── volume-manager/       # Multi-volume SquashFS
├── config/example.toml       # Full configuration example
└── scripts/                  # Shell helpers (debootstrap, chroot, grub)
```

## Requirements

- Rust 2021 edition
- `debootstrap`, `xorriso`, `gpg`, `curl`, `mksquashfs` (from squashfs-tools)
- Root access for chroot and mount operations

## License

GPL-3.0-only
