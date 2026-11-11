# Windows application support (amd64 Live)

`common/package-lists/wine.list.chroot` includes `lingmo-wine`, Wine's wrappers,
64-bit loader and fonts. The native manager package must first be published to
the Lingmo archive, or staged in `common/packages.chroot/` for a local build.
Build it from the sibling `lingmo-wine` directory with `dpkg-buildpackage -uc -us -b`.

`common/hooks/live/05-wine-multiarch.chroot` enables i386 in the target rootfs,
updates APT metadata and installs wine32:i386 with the 32-bit Mesa/OpenGL/Vulkan
runtime. This is explicit because the Live build disables apt recommendations.
The hook exits untouched on arm64/i386 and fails on apt errors, so an image
cannot silently omit the promised 32-bit runtime. Do not run it on the host.
The Calamares Live install inherits these packages; simple-cdd is unchanged.

Validate configuration without a network, root privileges or package installs:

```sh
python3 tests/wine-config-test.py
```

The release ISO still needs native testing with representative 32-bit and
64-bit EXE/MSI files. Prefixes isolate application settings, not host access.
The manager uses no privileged or autostart service, and no automatic
Winetricks/DXVK installation. Build-time multiarch installation does not run
any Windows binaries.
