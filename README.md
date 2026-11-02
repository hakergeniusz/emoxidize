# Emoxidize

**Emoxidize — MIT, clean-room implementation based on black-box observation of an emoji picker, spec in SPEC.md. App ID `io.github.hakergeniusz.Emoxidize`. Not derived from original source.**

Emoxidize is a single-instance GTK4/libadwaita emoji picker written in Rust. It
implements the behaviour documented in [SPEC.md](SPEC.md) (§3–§11) under the new
application identity `io.github.hakergeniusz.Emoxidize`.

![icon](data/icons/hicolor/128x128/apps/io.github.hakergeniusz.Emoxidize.png)

## Features

- **Picker window** (default 333×400 at 5 columns; width ≈ 60·columns + 33):
  top bar with settings gear, live search entry (“Search emojis...”, built-in
  clear button) and close button; scrollable emoji grid with an accent-coloured
  focus ring; bottom category bar with ten tabs — 🕘 Recents, 😀 Smileys,
  ✋ People, 🐶 Animals, 🍎 Food, ✈️ Travel, ⚽ Activities, 💡 Objects,
  ❤️ Symbols, 🏁 Flags — the active tab highlighted with an accent pill.
- **Startup state**: opens on Recents when history is non-empty, otherwise on
  Smileys; first cell focused; type-to-search works immediately without
  clicking.
- **Search**: live filter across all categories by CLDR name and keywords
  (e.g. “fire” → 🔥, firefighters, 🚒, 🎆, 🧨, 🧯, ❤️‍🔥 …); first result
  focused, `Return` activates it; no results = empty grid; the clear button
  restores the previously shown category while the category pill stays
  untouched during a search.
- **Keyboard**: letters/digits go to search, arrows move the cell focus,
  `Return` activates, `Escape` dismisses, mouse wheel scrolls.
- **Skin tones**: the setting applies the chosen tone to tone-capable emojis —
  and the **“all”** option displays every tone variant of each emoji at once,
  in both the category grid and search results.
- **GSettings** schema `io.github.hakergeniusz.Emoxidize`:
  `run-in-background`, `columns` (5–15, default 5), `skin-tone` enum
  (default/light/medium-light/medium/medium-dark/dark/**all**),
  `paste-on-select`, `multi-emoji`, `timeout-ms` (0–1000) and an internal
  `recents` mirror. All keys apply live (external writes via `gsettings set`
  are picked up while running).
- **Recents**: most-recent-first, de-duplicated, capped at 40 entries,
  persisted in `$XDG_DATA_HOME/emoxidize/recents.txt`.
- **Preferences window** (600×770): Extension section (paste-automatically
  toggle — sensitive once the paste bridge is installed — with an
  Installed/Not-installed badge and a “Get Extension” link), Behavior
  (multi-emoji mode, paste delay 0–1000 ms), Appearance (skin-tone dropdown
  with seven options, columns spinner), System (run in background,
  keyboard-shortcut command with copy-to-clipboard button), footer
  “Emoxidize v<version> • Rust”.
- **Selection**: without the bridge the emoji is copied to the CLIPBOARD and
  the picker dismisses; in multi-emoji mode selections accumulate in the
  clipboard and the picker dismisses `timeout-ms` after the last selection.
  With the **GNOME Shell paste bridge** and “Paste automatically” enabled, the
  emoji is pasted straight into the focused window instead.
- **Lifecycle**: second launches present the existing window (present, not
  toggle); `--gapplication-service` idles without a window and is activated by
  later launches; closing honours `run-in-background`.

## GNOME Shell paste bridge

`extension/emoxidize@hakergeniusz` is a GNOME Shell (45+) extension that owns
the bus name `io.github.hakergeniusz.Emoxidize.Extension` and exposes
`Paste(s text)`. It stores the emoji in Mutter's global clipboard (so it
survives the picker exiting) and injects <kbd>Ctrl</kbd>+<kbd>V</kbd> into the
focused window via Clutter's virtual keyboard, waiting for focus to return to
a non-picker window first.

Install it for the current user:

```sh
mkdir -p ~/.local/share/gnome-shell/extensions
cp -r extension/emoxidize@hakergeniusz ~/.local/share/gnome-shell/extensions/
gnome-extensions enable emoxidize@hakergeniusz
# then log out/in or restart the shell as needed
```

The distribution packages install the extension to
`/usr/share/gnome-shell/extensions/`.

## CLI

```
Usage: emoxidize [OPTION…]

  -h, --help                 show help (requires a display, like every
                             invocation: GTK is initialised first)
  --gapplication-service     enter service mode
```

Unknown options are reported (`Unknown option --frobnicate`) and ignored;
positional arguments are ignored. Without a display every invocation prints
`Gtk-WARNING **: Failed to open display` and exits with status 1.

## Building and running

Requirements: Rust (cargo), GTK4 ≥ 4.12 and libadwaita ≥ 1.4 development
packages, `glib-compile-schemas`.

```sh
glib-compile-schemas data/          # produces data/gschemas.compiled
cargo build
GSETTINGS_SCHEMA_DIR=$PWD/data ./target/debug/emoxidize
# or simply: GSETTINGS_SCHEMA_DIR=$PWD/data cargo run
```

## Packages

| Target | Location | Build |
|---|---|---|
| Arch Linux | `packaging/arch/PKGBUILD` | `cd packaging/arch && makepkg -f` |
| Debian/Ubuntu | `packaging/debian/` | copy to repo root → `dpkg-buildpackage -us -uc -b` |
| Fedora/RPM | `packaging/fedora/emoxidize.spec` | `rpmbuild -ba packaging/fedora/emoxidize.spec` |
| Flatpak | `packaging/flatpak/io.github.hakergeniusz.Emoxidize.json` | `packaging/flatpak/generate-sources.sh` then `flatpak-builder --force-clean build packaging/flatpak/io.github.hakergeniusz.Emoxidize.json` |

Debian builds two packages: `emoxidize` and
`gnome-shell-extension-emoxidize`. The Fedora spec likewise has an optional
`bridge` subpackage. The Flatpak manifest vendors cargo dependencies via
`generate-sources.sh` (needs `flatpak-cargo-generator.py`).

To install system-wide from source: copy `target/release/emoxidize` to
`/usr/bin`, the desktop/metainfo/icon files from `data/` to their usual
`/usr/share/{applications,metainfo,icons/hicolor/…}` locations, the schema
into `/usr/share/glib-2.0/schemas/` followed by
`glib-compile-schemas /usr/share/glib-2.0/schemas/`.

## Emoji data

The catalog (`src/emoji_data.rs`) is **generated** exclusively from public
Unicode data — [emoji-test.txt](https://www.unicode.org/Public/emoji/latest/emoji-test.txt)
(canonical sequences, grouping and ordering) and the CLDR English
[annotations](https://github.com/unicode-org/cldr-json) (short names and search
keywords) — via `tools/gen_emoji_data.py`. It contains 1920 canonical emojis
across the nine categories plus skin-tone variant tables for 332 bases. To
regenerate:

```sh
python3 tools/gen_emoji_data.py
```

## Repository layout

```
src/main.rs          CLI handling, GTK/adw init, single-instance GApplication
src/window.rs        picker window: grid, search, categories, keyboard,
                     selection pipeline, live settings
src/prefs.rs         preferences window
src/extension.rs     GNOME Shell paste-bridge client (D-Bus)
src/store.rs         recents persistence (XDG_DATA_HOME)
src/emoji_data.rs    generated Unicode/CLDR emoji catalog
data/                GSettings schema, desktop file, metainfo, icon set
extension/           GNOME Shell paste bridge (emoxidize@hakergeniusz)
packaging/           Arch PKGBUILD, Debian dir, Fedora spec, Flatpak manifest
tools/               emoji-data generator (public Unicode sources)
SPEC.md              black-box observation spec (evidence of the clean room)
```

## License

MIT — see [LICENSE](LICENSE).
