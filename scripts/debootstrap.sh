#!/bin/bash
#=============================================================================
# debootstrap.sh - Bootstrap a Debian base system
#=============================================================================
# Usage: debootstrap.sh <target-dir> <codename> <mirror> <arch> <variant>
#
# This script wraps debootstrap(8) with safety checks and retry logic.
# It is invoked by the Lingmo Builder Rust pipeline engine.
#=============================================================================
set -euo pipefail

RETRIES=3

TARGET="${1:?Usage: $0 <target-dir> <codename> <mirror> <arch> <variant>}"
CODENAME="${2}"
MIRROR="${3}"
ARCH="${4}"
VARIANT="${5:-buildd}"

if [ -z "$TARGET" ] || [ -z "$CODENAME" ] || [ -z "$MIRROR" ] || [ -z "$ARCH" ]; then
    echo "ERROR: Missing required arguments"
    echo "Usage: $0 <target-dir> <codename> <mirror> <arch> [variant]"
    exit 1
fi

# Validate target is a directory
if [ -f "$TARGET" ]; then
    echo "ERROR: Target path exists but is not a directory: $TARGET"
    exit 1
fi

# Create target if it doesn't exist
mkdir -p "$TARGET"

echo "==> Bootstrapping Debian $CODENAME ($ARCH) into $TARGET"
echo "    Mirror: $MIRROR"
echo "    Variant: $VARIANT"

for i in $(seq 1 $RETRIES); do
    echo "==> Attempt $i of $RETRIES..."
    if debootstrap \
        --arch="$ARCH" \
        --variant="$VARIANT" \
        --include=apt-transport-https,ca-certificates \
        "$CODENAME" \
        "$TARGET" \
        "$MIRROR"; then
        echo "==> Bootstrap successful"
        exit 0
    fi

    if [ $i -lt $RETRIES ]; then
        WAIT=$((i * 5))
        echo "==> Bootstrap failed. Retrying in ${WAIT}s..."
        sleep "$WAIT"
    fi
done

echo "ERROR: Bootstrap failed after $RETRIES attempts"
exit 1
