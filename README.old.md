# Lingmo OS Builder

Lingmo OS 的 ISO 构建仓库。

- live 镜像走 live-build，配置在 `scripts/lingmo-config/`，由 `scripts/build.sh` 驱动。
- installer 镜像走 simple-cdd + debian-cd，配置在 `scripts/simple-cdd/`。

产物是可以直接写 U 盘的 hybrid ISO。

## 产出

| 类型 | 命令 | 中间产物 | 最终路径 |
| --- | --- | --- | --- |
| live | `sudo ./build.sh` | `live-image-<arch>.hybrid.iso` | `scripts/images/lingmo-os-5.0-canary-26a01-desktop-amd64.iso` |
| installer | `sudo ./build.sh --installer` | `simple-cdd/images/lingmo-<ver>-<arch>-BD-1.iso` | `scripts/images/lingmo-os-<ver>-installer-amd64.iso` |

镜像名由 `build.sh` 的 `target_image_name()` 拼：

- live：`lingmo-os-<LINGMO_SYS_VERSION>-<LINGMO_VERTYPE>-<LINGMO_BUILD_VER>-desktop-<arch>.<ext>`，默认值是 `5.0` / `canary` / `26a01`。
- installer：`lingmo-os-<LINGMO_VERSION>-installer[-<variant>]-<arch>.<ext>`，`LINGMO_VERSION` 默认取发行版名（`trixie`），`--version` 可覆盖。
- 同一个 ISO 的构建日志落在旁边，扩展名换成 `.log`（`scripts/images/<同名>.log`）。
- armel / armhf 走 `--binary-images hdd --binary-filesystem ext4 --chroot-filesystem none`，产出 `.img` 而不是 `.iso`。

CI 发布时，超过 2GiB 的 ISO 会被切成 `<iso>.part-aa`、`<iso>.part-ab`… 上传，见下面的 CI 一节。

## 依赖与环境

- Debian 系构建机。`build.sh` 会读 `/usr/lib/os-release` 的 `ID=` / `ID_LIKE=debian`，否则退到 `/etc/debian_version`。
- live 镜像：`live-build >= 1:20230502`、`debootstrap >= 1.0.97`。版本在 `scripts/build.sh` 里用 `dpkg --compare-versions` 卡。
- live-build 必须是 Lingmo 定制版（版本号带 `+lingmo`）。上游版不认 `LB_LIVE_DIR`，介质目录会退回 `live/`，而引导参数里的 `live-media-path=core.lm` 不会变，ISO 起来找不到介质。
- installer 镜像：`simple-cdd >= 0.6.9`、`debian-cd >= 3.2.1`。
- 需要 root。非 root 时脚本用 `sudo`，找不到 `sudo` 直接退出。
- `auto/config` 会在 `/usr/share/live/build/data/debian-cd/` 下建 `<codename> -> sid` 的符号链接（live-build 的 debian-cd 数据目录不认未知发行版）。目录不可写时会打印要求手动执行的 `ln -sf` 命令并退出。CI 里这一步是单独一条 `ln -sf sid .../debian-cd/trixie`。
- 分层构建时钩子会在 chroot 同级建 `scripts/layer-work/`，层内文件用硬链接，所以它和 chroot 必须落在同一个文件系统上。
- Python 只在 CI 的版本门禁里用到（`lingmo-version` 脚本）。构建本身不需要 Python / Node。

## 目录结构

```
.
├── .getopt.sh                  build.sh 的命令行选项定义（需与 build.sh 同目录）
├── scripts/
│   ├── build.sh                构建入口：解析参数、调 lb config/lb build 或 simple-cdd
│   ├── build_all.sh            单行脚本，内容只有 `build.sh`
│   ├── make-sums               生成 md5/sha1/sha256/sha384/sha512 校验文件（build.sh 里调用已注释）
│   ├── auto/
│   │   ├── config              live-build 的 auto/config：选镜像源、调 lb config
│   │   └── clean               live-build 的 auto/clean：lb clean noauto + 清 config/*
│   ├── simple-cdd/             installer 镜像的 simple-cdd / debian-cd 配置
│   │   ├── simple-cdd.conf
│   │   ├── disc-end-hook       改 ISO 品牌、换 grub 主题、重算 md5sum.txt
│   │   ├── local_packages/     放进 ISO 的本地 deb（当前只有 .empty）
│   │   └── profiles/           lingmo / offline / default / kali 的 preseed、postinst、downloads
│   └── lingmo-config/
│       ├── common/             live 镜像公共配置，见下表
│       ├── variant-default     普通文件，内容是 `variant-everything`（指针，不是目录）
│       ├── variant-everything/package-lists/
│       ├── variant-minimal/    只有 .empty，不追加任何包清单
│       ├── installer-default/packages
│       ├── installer-netinst/packages
│       └── installer-everything/packages
├── tools/
│   ├── build_version.sh        构建号计数器（build.sh 有 source，但调用被注释）
│   ├── build_version_2.sh      CI 生成 Release tag 用
│   ├── pkg_build/              独立的 Qt 打包工具（lingmo-pkgbuild / lingmo-repotool），与 ISO 构建无关
│   ├── config.sh               其他仓库遗留的打包脚本，ISO 构建不调用
│   ├── default.xml             repo manifest，ISO 构建不调用
│   ├── update_status.sh        写 get_status.build.sa 状态文件，ISO 构建不调用
│   └── utils/.getopt.sh        与根目录 .getopt.sh 内容相同
└── .github/workflows/build.yml
```

`scripts/lingmo-config/common/` 的内容：

| 路径 | 作用 |
| --- | --- |
| `archives/` | 构建期额外 APT 源：`lingmo.list.{chroot,binary}` 指向 OBS `home:/LingmoOS/Debian_13`，`firmware.list.*` 指向 `deb.debian.org` 的 testing non-free |
| `package-lists/` | chroot 包清单：`lingmo_default_chroot`、`hardware-support`、`virtualization`、`linux-headers`、`standard`、`lingmo_extra_list_bookworm`；binary 侧是 `lingmo_default_binary` |
| `preseed/` | chroot 阶段的 debconf preseed |
| `hooks/live/lingmo-hacks.chroot` | chroot 阶段定制：写 `/usr/lib/os-release` 与 `/etc/os-version`、hold 内核包、删多余桌面包、`update-initramfs -u`、`ldconfig`、`glib-compile-schemas` |
| `hooks/live/20-make-layer-images.binary` | binary 阶段把根文件系统拆成 9 个 `.img`，写 `core.module` |
| `includes.chroot_after_packages/` | 直接覆盖进 rootfs 的文件：`/etc/sddm.conf`、`/etc/skel`、`/etc/live/config.conf.d/`、plasma 桌面主题、live-config 钩子 `1165-*` / `1166-*` |
| `includes.binary/` | 写进 ISO 根目录的文件：`Version`、`README`、`licenses/` |
| `includes.installer/` | installer 的 `preseed.cfg`、`lingmo-finish-install`、`finish-install.d/14lingmo-finish-install`、`live-installer.d/unhold` |
| `bootloaders/grub-pc/` | grub 菜单与主题：`grub.cfg`、`config.cfg`、`theme.cfg`、`memtest.cfg`、`loopback.cfg`、`theme/`、`next/`、`lingmo/`、`live-theme/` |
| `layer-images.conf` | 分层映射表（见下） |

## 本地构建

先做三件容易漏的准备，否则 `build.sh` 起不来或产出缺包：

1. 在 `scripts/` 目录下执行。`build.sh` 顶部是 `source ../tools/build_version.sh`，相对路径，换个目录就找不到。
2. `.getopt.sh` 要放在 `build.sh` 旁边。它留在仓库根目录，`scripts/` 下没有：

   ```sh
   cp .getopt.sh scripts/
   ```

3. 把 `variant-default` 换成指向 `variant-everything` 的符号链接。它现在是个普通文件，`build.sh` 的 `[ -d ... ]` 检查通不过，auto/config 的 `cp -rTL` 也只会把这个文件本身拷进 `config/`：

   ```sh
   cd scripts/lingmo-config
   ln -sfn "$(cat variant-default)" variant-default.dir && mv -T variant-default.dir variant-default
   ```

   CI 里是等价的两条命令：`TARGET=$(cat variant-default); ln -sfn "$TARGET" variant-default-dir; mv variant-default-dir variant-default`。

live 镜像：

```sh
cd scripts
sudo ./build.sh                          # 等价于 --live --variant default
sudo ./build.sh --variant everything
sudo ./build.sh --variant minimal
sudo ./build.sh -a amd64 -D              # -D 打印调试信息，命令同时 tee 到日志
sudo ./build.sh --no-clean               # 不清上一轮的 build 目录
sudo ./build.sh --clean                  # 只清理，不构建
sudo ./build.sh --get-image-path         # 只打印目标镜像名
```

流程是 `lb config -a <arch> --distribution <dist> -- --variant <variant>` 加 `sudo lb build`。`scripts/auto/config` 负责把 `lingmo-config/common` 和 `lingmo-config/variant-<variant>` 叠到 `config/` 里，再按参数生成 `lb config` 调用。

installer 镜像：

```sh
cd scripts
sudo ./build.sh --installer --variant default
sudo ./build.sh --installer --variant netinst
sudo ./build.sh --installer --variant everything
sudo ./build.sh --installer --version 26a01    # 覆盖镜像名里的版本段
```

installer 不使用 live-build。`build.sh` 会把 `/usr/share/debian-cd` 拷到 `simple-cdd/debian-cd`、用 `lingmo-config/installer-<variant>/packages` 生成 `simple-cdd/profiles/lingmo.downloads`，再跑：

```sh
build-simple-cdd --verbose --debug --force-root --conf simple-cdd.conf \
  --dist <codename> --debian-mirror <mirror> \
  --profiles "lingmo offline" --auto-profiles "lingmo offline"
```

netinst 用 `DISKTYPE=NETINST` 和单 profile `lingmo`；其余用 `DISKTYPE=BD`。`build.sh` 里还有 `purple` 分支（`DISKTYPE=BD`，多带 `lingmo-purple` profile 和 `debian-installer/theme=Clearlooks-Purple`），但仓库没有 `installer-purple/packages`，这个变体跑不起来。

镜像源：`scripts/.mirror` 存在时用它的内容，否则用 `http://ftp.debian.org/debian`。debootstrap 用的具体发行版由 `wget .../dists/<dist>/Release` 里的 `Codename:` 决定。

可用变体：

| 类型 | 变体 | 说明 |
| --- | --- | --- |
| live | `default` | 指向 `variant-everything`（见上面的符号链接） |
| live | `everything` | `variant-everything/package-lists/`：`lingmo.list.chroot` + `lingmo_everything.list.chroot` |
| live | `minimal` | `variant-minimal/.empty`，只用 `common/` 的清单 |
| installer | `default` | `installer-default/packages` |
| installer | `netinst` | `installer-netinst/packages`，几乎只有固件 |
| installer | `everything` | `installer-everything/packages` |

## 命令行参数

`build.sh` 用 `getopt`，选项表在 `.getopt.sh`：

| 参数 | 作用 |
| --- | --- |
| `-d` / `--distribution <dist>` | 发行版，默认 `trixie` |
| `-a` / `--arch <arch>` | 目标架构，默认 `dpkg --print-architecture`；`x64`→`amd64`、`x86`→`i386` |
| `--variant <name>` | live 或 installer 的变体，见上表 |
| `--version <ver>` | 覆盖 `LINGMO_VERSION`（installer 镜像名） |
| `--live` / `--installer` | 选构建类型，默认 `live` |
| `--subdir <dir>` | 产物放进 `images/<dir>/` |
| `--clean` | 只清理 |
| `--no-clean` | 跳过清理 |
| `--get-image-path` | 打印目标镜像名后退出 |
| `-p` / `--proposed-updates` | 目前是空操作：`auto/config` 会打印 Debian 没有 proposed-updates 的提示，写源的代码被注释 |
| `-v` / `--verbose` | 命令输出同时打到终端 |
| `-D` / `--debug` | 打印脚本内部变量 |
| `-s` / `--salt` | 无参开关，脚本里只 `shift`，没有效果 |
| `-h` / `--help` | 打印用法 |

跨架构限制：只有 `amd64` / `i386` 之间可以互相构建，其它组合直接报错退出（`--installer` 不受此限）。

## 环境变量

| 变量 | 默认 | 作用 |
| --- | --- | --- |
| `LB_LIVE_DIR` | `core.lm` | ISO 里的介质目录名。`auto/config` 把它写进 `--bootappend-live` 的 `live-media-path=`，定制版 live-build 用它决定 `binary/<dir>`。改成别的值时引导参数和介质目录会一起变 |
| `LINGMO_LAYERED_ROOTFS` | `yes` | 是否把根文件系统拆成多个 `.img`。不是 `yes` 时 `20-make-layer-images.binary` 直接跳过，ISO 里保留单个 `filesystem.squashfs` |
| `LINGMO_LAYER_MODULE` | `core` | 层清单名（`core.module`），同时写进引导参数的 `module=` |

其余构建参数是 `build.sh` 里的变量而非环境变量：`LINGMO_SYS_VERSION=5.0`、`LINGMO_VERTYPE=canary`、`LINGMO_BUILD_VER=26a01`，没有对应的命令行开关，要改就改脚本（注意 `lingmo-hacks.chroot` 里也写了一份版本号，见"版本号与品牌"一节）。

## auto/config 里的 lb config

`scripts/auto/config` 最后是一条 `lb config noauto` 加三十多个参数。要改的几处：

- `--mirror-bootstrap` / `--mirror-debian-installer` 用 `lingmo_mirror`，`--mirror-binary` 用 `public_lingmo_mirror`。
- security 源固定走 CDN：`--mirror-chroot-security` / `--mirror-binary-security` 都是 `http://deb.debian.org/debian-security`。
- `--apt-options` 合并了 `DPkg::Options::="--force-confnew"`、`-y`、`Acquire::Retries=5`、`Acquire::http::Timeout=30`。
- `--bootappend-live` 带 `boot=live components quiet splash noeject live-media-path=${LB_LIVE_DIR:-live} module=${LINGMO_LAYER_MODULE:-core}`。
- `--bootappend-live-failsafe` 带 `memtest noapic noapm nodma ...`。
- `--memtest memtest86+`、`--bootloaders "grub-efi grub-pc"`、`--uefi-secure-boot enable`、`--firmware-binary true`、`--firmware-chroot true`、`--linux-packages "linux-image linux-headers"`、`--iso-volume "Lingmo OS 5.x"`。
- 按架构追加：i386 加 `--linux-flavours 686-pae`，arm64 加 `--bootloaders grub-efi --uefi-secure-boot disable`，armel/armhf 加 `--binary-images hdd --binary-filesystem ext4 --chroot-filesystem none`。

这条命令是一条带续行的长命令，命令内部不能插注释行，每行末尾的 `\` 不能丢。

## ISO 内部结构

live ISO 的根目录：

```
core.lm/                        ← LB_LIVE_DIR，介质目录
├── vmlinuz-*                   ← 从 chroot/boot 拷来的内核
├── initrd.img-*                ← initramfs
├── memtest.bin / memtest.efi   ← --memtest memtest86+ 的产物
├── core.module                 ← 层清单（LINGMO_LAYER_MODULE）
├── system.img
├── firmware.img
├── kernel.img
├── desktop.img
├── lingmo.img
├── apps.img
├── vendor.img
├── device.img
├── misc.img
└── filesystem.squashfs         ← 分层成功后被钩子删除，失败时保留
/boot/grub/                     ← grub 菜单与主题，来自 lingmo-config/common/bootloaders/grub-pc
```

`vmlinuz-*` / `initrd.img-*` / `memtest.bin` / `memtest.efi` 是 live-build 的 binary 阶段产物，本仓库不生成也不改名，只决定它们落在哪个目录。

`core.module` 每行一个文件名，按顺序叠加，**越靠后优先级越高**：

```
system.img
firmware.img
kernel.img
desktop.img
lingmo.img
apps.img
vendor.img
device.img
misc.img
```

层集合由钩子里的 `ORDER` 写死（`system firmware kernel desktop lingmo apps vendor device misc`）。**空层也会生成 `.img`**，所以 `core.module` 的层数恒定，Calamares 那边的解包列表不用跟着映射表变。增减层要同时改三处：

1. `scripts/lingmo-config/common/layer-images.conf`
2. `scripts/lingmo-config/common/hooks/live/20-make-layer-images.binary` 里的 `ORDER`
3. `lingmo-installer-config` 仓库 `calamares/modules/unpackfs.conf` 的层列表

引导参数 `module=core` 对应 `core.module`，`live-media-path=core.lm` 对应介质目录，两者要和 `LB_LIVE_DIR` / `LINGMO_LAYER_MODULE` 保持一致。

## 分层规则

钩子读 `config/layer-images.conf`（即 `lingmo-config/common/layer-images.conf` 被拷过去的那份），每行三列，`#` 开头是注释：

```
path  <路径前缀>   <层名>
pkg   <包名glob>   <层名>
```

判定顺序：

1. 符号链接 → `misc`（不管它属于哪个包）。
2. `path` 规则，按文件里的先后顺序，第一条命中的生效。
3. `pkg` 规则，按文件里的先后顺序，第一条命中的生效。
4. 兜底 → `system`。

当前 `path` 规则把 `/vendor`、`/device`、`/boot`、`/lib/modules`、`/usr/lib/modules`、`/usr/src` 分走；`pkg` 规则按内核与固件、桌面栈、Lingmo 自有组件（`lingmo-*`、`velora-*`、`lingmoos-*`）、应用分成 `kernel` / `firmware` / `desktop` / `lingmo` / `apps`，最后一行 `pkg * system` 兜底。兜底行必须在最后。

钩子的处理流程（`20-make-layer-images.binary`）：

1. 用 `dpkg-query --admindir=chroot/var/lib/dpkg` 逐包列出文件，包名先过 `pkg` 规则定层。
2. 一次 `find` 收集符号链接（全部进 `misc`）和目录；目录单独用 `mkdir -p` 建，不交给 `cp`。
3. 收集 chroot 里所有真实文件，和包清单做差集，得到"非包所有"的文件（hook 产物、initramfs 等），兜底进 `system`。
4. 按规则把路径分到各层的清单文件，再每层 `cp -dl --parents` 批量硬链接，`mksquashfs -comp xz` 出 `<层>.img`。
5. 补 FHS 骨架目录（`dev proc sys run tmp root home srv opt mnt media etc var usr boot`）进 `system` 层。
6. 写 `core.module`，`misc.img` 放最后。
7. 全部成功后才删 `filesystem.squashfs`。

## CI

`.github/workflows/build.yml`，触发条件：push 到 `main`、PR 到 `main`、以及手动 `workflow_dispatch`（布尔输入 `release`，默认 `true`，决定构建完是否发 Release）。

job 跑在 `debian:testing` 容器里，`options: --privileged`。步骤：

1. 装 `git`、`ca-certificates`。
2. checkout（`fetch-depth: 1`）。
3. 版本一致性门禁：clone `LingmoOS/lingmo-version` 和 `LingmoOS/base-files` 到 `/tmp`，跑 `python3 lingmo-version check --base-files /tmp/base-files --builder "$PWD"`，不一致就终止。容器里没有 python3 时这一步会先装。
4. 装构建依赖：`live-build debootstrap xorriso squashfs-tools mtools dosfstools gpg cpio zstd gettext sudo git jq`，外加从 `cli.github.com` 装 `gh`。
5. 换成 Lingmo 的 live-build：写 `/etc/apt/sources.list.d/lingmo-obs.list`（`deb https://download.opensuse.org/repositories/home:/LingmoOS/Debian_13/ /`）、导入 `Release.key`，再写 `/etc/apt/preferences.d/lingmo-live-build`：

   ```
   Package: live-build
   Pin: origin download.opensuse.org
   Pin-Priority: 1001
   ```

   `apt-get install --allow-downgrades live-build`，最后断言 `dpkg-query -W -f='${Version}' live-build` 含 `+lingmo`，否则退出。必须用 apt pin，是因为容器是 testing，自带的 live-build 版本比 trixie 分支的 Lingmo 定制版高，只靠版本号选不中。
6. 清 `apt` 缓存和 `/tmp` 腾空间。
7. `ln -sf sid /usr/share/live/build/data/debian-cd/trixie`。
8. 准备构建目录：`mkdir build && cp -r scripts/* build/ && cp .getopt.sh build/`，`chmod +x build/build.sh build/auto/config`，把 `build/lingmo-config/variant-default` 改成指向 `variant-everything` 的符号链接。
9. `cd build && sudo ./build.sh`（无参数：live + variant default）。
10. 收集 `build/images/*.iso` 到 `artifact/`，用 `actions/upload-artifact@v4` 上传为 `upload-iso`（`compression-level: 0`）。
11. push 到 `main`，或手动触发且勾了 `release` 时生成 tag：`lingmo-os-$(date +%y%m%d%H%M).$(git rev-parse --short HEAD)`，由 `tools/build_version_2.sh` 的 `generate_build_ver` 产出。
12. 发布：tag 已存在就覆盖上传（`gh release upload --clobber`），不存在就 `gh release create --latest=false`。ISO 大于 2147483648 字节时 `split -b 1900M <iso> <iso>.part-` 切成 `.part-aa`、`.part-ab`…（GitHub Release 单文件上限 2GiB），逐个上传后删掉本地分片；没超限就整包上传。

PR 只构建不发布。

## 与 OBS 的关系

仓库不向 OBS 推包，只是消费 OBS 上的 `home:LingmoOS/Debian_13`：

- CI 从这个源装 Lingmo 定制版 `live-build`，并用 apt pin 压过 testing 自带的版本。
- `lingmo-config/common/archives/lingmo.list.chroot` 和 `lingmo.list.binary` 把这个源加进构建期 APT 源，Lingmo 的桌面组件（`dde-*`、`lingmo-*`、`velora-*` 等）从那里装。

installer 一侧的 `disc-end-hook` 只做品牌替换，和 OBS 无关。

## 版本号与品牌

`hooks/live/lingmo-hacks.chroot` 里写死了一组版本信息，生成的 `/usr/lib/os-release` 和 `/etc/os-version` 都来自它：

```
VERSION=5.0  CODENAME=Lithium  BUILD_VER=26a01  DEBIAN_VER=13.7  VERSION_TYPE=Alpha
BUILD_ID=$(date +%Y%m%d)  BUILD_TIME=261014204
```

改版本时要同时改 `build.sh` 的 `LINGMO_SYS_VERSION` / `LINGMO_BUILD_VER`、`lingmo-hacks.chroot` 里的这几个值，以及 `includes.binary/Version`、`includes.chroot_after_packages/etc/system/currentVersion`，否则镜像名和系统内版本对不上。

`/etc/os-version` 由同一段脚本按 `os-release` 重写，`dtkcore`、系统监视器、设备管理器读的是它。

## 已知问题

1. `auto/config` 里的 `lb config` 是一条命令加续行。命令内部不能插注释行，行尾的 `\` 也不能丢；断一处，后面的参数会被 shell 当成新命令。
2. `.getopt.sh` 在仓库根目录，`build.sh` 却按 `$(dirname $0)/.getopt.sh` 找。直接在 `scripts/` 里跑会 source 失败，先按上面"本地构建"第 2 条拷一份。
3. `variant-default` 是普通文件不是目录。`build.sh` 的 `[ ! -d ]` 只打印 `ERROR: Unknown variant...` 而不退出，`auto/config` 的 `cp -rTL` 会把这个文件本身拷进 `config/`，最终 ISO 缺 `variant-everything` 的包清单。按 CI 的做法改成符号链接。
4. 上游 live-build 不认 `LB_LIVE_DIR`。介质目录会回落到 `live/`，引导参数里却还是 `live-media-path=core.lm`，ISO 起不来。CI 用 `+lingmo` 版本断言挡住这种情况。
5. 分层钩子的硬链接坑：chroot 里 bind-mount 进来的少数文件在别的设备上，`cp -dl` 会整批失败。钩子按 `cp:` 报错行解析出这些文件，只对它们回退成复制。不要改成"整层重新复制"，27G 量级的 rootfs 会又慢又爆盘。
6. 目录不能用 `cp --parents` 建（会报 `-r not specified` 并跳过），钩子是单独收集目录再用 `mkdir -p` 建。
7. 被 `find -prune` 掉的 `/dev`、`/proc`、`/sys`、`/run`、`/mnt`、`/media` 本身必须补进 `system` 层。缺了这些目录，initramfs 里 `mount --move /dev /root/dev` 会失败，内核 panic。
8. `sort` / `comm` 必须统一 `LC_ALL=C`，`allfiles` 和 dpkg 清单的前导斜杠也要一致。不一致时 `comm` 会报 `file 1 is not in sorted order` 并给出错误结果（历史上所有文件曾被判成"非包所有"）。
9. 钩子有两个安全阀：某层列了 N 条路径却一个都没建出来，或层内文件总数不到 chroot 的一半，都会中止构建并保留 `filesystem.squashfs`。看到这两条错误先查映射表和文件权限，不要绕过。
10. 非 `default` 变体拼出来的镜像名少一个连字符：`lingmo-os-5.0-canary26a01-desktop-amd64.iso`（`default` 是 `canary-26a01`）。代码如此，改名前先确认下游有没有依赖这个格式。
11. `LINGMO_BUILD_VER` 硬编码为 `26a01`，`build.sh` 里 `LINGMO_BUILD_VER=$(generate_build_ver)` 那行是注释掉的；`tools/build_version.sh` 的 `.version.build` 计数器目前只有 `tools/test.sh` 会用，CI 的 tag 走的是 `build_version_2.sh`。
12. `build.sh` 的 `purple` installer 分支没有对应的 `installer-purple/packages`，跑起来会在生成 `lingmo.downloads` 时失败。
13. `build.sh` 里 `LINGMO_LAYERED_ROOTFS` 上方的注释还写着"分层暂不开启"，但代码默认值是 `yes`。以代码为准。
