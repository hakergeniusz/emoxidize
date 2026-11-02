#!/bin/sh
# Install the Emoxidize paste bridge for the current user (GNOME Shell 45+).
set -eu
SRC=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/emoxidize@hakergeniusz
DEST=${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions
mkdir -p "$DEST"
rm -rf "$DEST/emoxidize@hakergeniusz"
cp -r "$SRC" "$DEST/"
echo "Installed to $DEST/emoxidize@hakergeniusz"
echo "Enable it with: gnome-extensions enable emoxidize@hakergeniusz"
echo "Log out and back in (or restart the shell) if it was a fresh install."
