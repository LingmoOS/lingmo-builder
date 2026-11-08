#!/usr/bin/env bash
# Upload complete ISO files and checksums; never split the SourceForge copy.
set -euo pipefail
: "${SF_USERNAME:?Set SF_USERNAME repository variable}"
: "${SF_PROJECT:?Set SF_PROJECT repository variable}"
: "${SF_REMOTE_DIR:?Set SF_REMOTE_DIR to the project files base directory}"
: "${SF_SSH_PRIVATE_KEY:?Set SF_SSH_PRIVATE_KEY repository secret}"
: "${SF_KNOWN_HOSTS:?Set verified SF_KNOWN_HOSTS repository secret}"
[[ "$SF_USERNAME" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'Invalid SourceForge username' >&2; exit 2; }
[[ "$SF_PROJECT" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'Invalid SourceForge project' >&2; exit 2; }
[[ "$SF_REMOTE_DIR" =~ ^[a-zA-Z0-9_./-]+$ && "$SF_REMOTE_DIR" != /* && "/$SF_REMOTE_DIR/" != *"/../"* ]] || { echo 'Invalid project-relative directory' >&2; exit 2; }
# CI supplies the canonical version from the same check that guarded the ISO.
# Do not derive a timestamp or clone version metadata again after the build.
if [ -n "${SF_BUILD_VERSION:-}" ]; then
    [[ "$SF_BUILD_VERSION" =~ ^[a-zA-Z0-9][a-zA-Z0-9_.-]*$ && "$SF_BUILD_VERSION" != *..* ]] || { echo 'Invalid build version' >&2; exit 2; }
    SF_REMOTE_DIR="${SF_REMOTE_DIR%/}/$SF_BUILD_VERSION"
fi
source_dir="${1:-artifact}"
shopt -s nullglob
images=("$source_dir"/*.iso)
(( ${#images[@]} > 0 )) || { echo 'No ISO files to publish' >&2; exit 1; }
for image in "${images[@]}"; do
    [[ -s "$image" ]] || { echo "Empty ISO: $image" >&2; exit 1; }
done
work=$(mktemp -d)
cleanup() { rm -rf -- "$work"; }
trap cleanup EXIT
chmod 700 "$work"
printf '%s\n' "$SF_SSH_PRIVATE_KEY" > "$work/key"
printf '%s\n' "$SF_KNOWN_HOSTS" > "$work/known_hosts"
chmod 600 "$work/key" "$work/known_hosts"
# rsync invokes SSH using an argument-preserving wrapper; key material is never logged.
cat > "$work/ssh" <<'SSH'
#!/bin/sh
exec ssh -i "$SF_UPLOAD_KEY" -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile="$SF_UPLOAD_HOSTS" "$@"
SSH
chmod 700 "$work/ssh"
export SF_UPLOAD_KEY="$work/key" SF_UPLOAD_HOSTS="$work/known_hosts"
bash "$(dirname "$0")/checksums.sh" "$source_dir"
destination="$SF_USERNAME,$SF_PROJECT@frs.sourceforge.net:/home/frs/project/$SF_PROJECT/$SF_REMOTE_DIR/"
# Partial files stay hidden; publish each completed ISO by rename at the end.
rsync -av --mkpath --checksum --partial --partial-dir=.rsync-partial --delay-updates \
    -e "$work/ssh" -- "${images[@]}" "$destination"
checksums=();for tool in sha256sum sha512sum md5sum; do
    checksums+=("$source_dir/$tool.txt" "$source_dir/$tool.README")
done
rsync -av --mkpath --checksum --delay-updates -e "$work/ssh" -- "${checksums[@]}" "$destination"
echo "Published ${#images[@]} ISO file(s), checksum manifests and instructions to SourceForge."
