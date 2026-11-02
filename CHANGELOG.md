# Changelog

All notable changes to Emoxidize are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
follows [Semantic Versioning](https://semver.org/).

## [1.0.0-beta.1] — 2026-11-02

First public beta.

### Added
- Picker window (GTK4/libadwaita, single-instance) with ten category tabs,
  live name/keyword search, keyboard navigation and type-to-search.
- Recents history: most-recent-first, de-duplicated, capped at 40 entries,
  persisted under `$XDG_DATA_HOME/emoxidize/recents.txt`.
- Skin tones: default/light/medium-light/medium/medium-dark/dark applied live
  to tone-capable emojis — plus an **“all”** mode that shows every tone
  variant of each emoji at once.
- “Paste automatically” via the new GNOME Shell paste bridge
  (`emoxidize@hakergeniusz`, GNOME Shell 45–51): the selected emoji is pasted
  into the focused window through the Mutter clipboard and a virtual keyboard
  Ctrl+V; the Preferences badge reflects bridge presence. The extension is
  self-hosted (not on extensions.gnome.org): ships in the native packages and
  as an install script / release zip.
- Multi-emoji mode with a configurable paste delay (0–1000 ms).
- GSettings schema `io.github.hakergeniusz.Emoxidize`; all keys apply live,
  including external writes.
- CLI: `--gapplication-service` service mode, help output, graceful handling
  of unknown options, exit 1 without a display.
- Packages: Arch PKGBUILD, Debian packaging (picker + shell extension),
  Fedora RPM spec and a Flatpak manifest.

[1.0.0-beta.1]: https://github.com/hakergeniusz/emoxidize/releases/tag/v1.0.0-beta.1
