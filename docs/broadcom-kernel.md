# Broadcom driver for the amd64 Live kernel

CI runs `bash tools/ci/download-broadcom.sh` before preparing the build tree.
The script downloads Debian's `broadcom-sta-dkms_6.30.223.271-32_amd64.deb`
into `scripts/lingmo-config/common/packages.chroot/`. The existing copy step
then stages it in `build/lingmo-config/common/packages.chroot/`.

The package URL, version, architecture and SHA-256 are fixed. Download goes to
a temporary file; checksum and dpkg-deb metadata must match before it replaces
the destination. Any error aborts CI. Existing polkit packages are untouched.
No Debian sid/testing sources are added to the target system. The package
list still requests broadcom-sta-dkms, supplied by this newer local package.

The -26 package from Trixie failed during DKMS compilation with 6.18.62-lingmo.
The -32 source has been compiled locally against those headers to produce
wl.ko. This does not certify hardware operation or Secure Boot enrollment.
Full ISO construction and boot validation remain necessary.

For local builds, run the same script before copying scripts/ into the build
workspace. An optional argument changes the staging directory for tests.
The download is intentionally not checked into Git. Update the version, URL
and hash together when changing drivers. Only amd64 CI is covered.

Validation: `python3 tests/test_broadcom_download.py` covers step ordering,
repeat staging, failed downloads, hash/metadata mismatch, temporary cleanup
and preservation of an existing package. No root or network is used by tests.
