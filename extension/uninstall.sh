#!/bin/sh
set -eu
gnome-extensions disable emoxidize@hakergeniusz 2>/dev/null || true
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/emoxidize@hakergeniusz"
echo "Removed the Emoxidize paste bridge (user install)."
