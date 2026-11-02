#!/bin/sh
# Generate the cargo dependency sources for the Flatpak build.
#
# Requires: cargo, python3, and flatpak-cargo-generator.py
# (https://github.com/flatpak/flatpak-builder-tools/blob/master/cargo/flatpak-cargo-generator.py)
#
# Usage:  ./generate-sources.sh   (run from the repository root or this dir)
set -eu

REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$REPO_ROOT"

if ! command -v flatpak-cargo-generator.py >/dev/null 2>&1; then
    echo "flatpak-cargo-generator.py not found on PATH." >&2
    echo "Fetch it: curl -fsSLo /tmp/flatpak-cargo-generator.py \\" >&2
    echo "  https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py" >&2
    exit 1
fi

python3 flatpak-cargo-generator.py Cargo.lock -o packaging/flatpak/generated-cargo-sources.json
echo "wrote packaging/flatpak/generated-cargo-sources.json"
