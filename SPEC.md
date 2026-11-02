# Emojify — Clean-Room Reverse-Engineering Specification

**Artifact:** GTK4/libadwaita emoji picker, binary `build/src/emojify` (ELF x86-64, PIE, dynamically linked, with debug info).
**Documented version:** v1.3.3 (self-reported in the Preferences footer: “Emojify v1.3.3 • Built with Meson 1.12.1”).
**Method:** Strict black-box observation. The application was executed inside an isolated virtual session (Xvfb display `:77`, private D-Bus session bus, sandboxed `XDG_*` directories) and probed via screenshots, synthetic pointer/keyboard input (`xdotool`), a custom XFixes-based selection monitor, and its own runtime diagnostics. No source, build, or resource files were read.

---

## 1. Identity

| Property | Value |
|---|---|
| Window title (main) | `Emojify` |
| Window title (settings) | `Preferences` |
| GApplication application ID | `xyz.riothedev.emojify` |
| GSettings schema ID | `xyz.riothedev.emojify` |
| Toolkit | GTK 4 + libadwaita via gtkmm/C++ (`libgtkmm-4.0`, `libadwaita-1`, `libglibmm-2.68`, `libgiomm-2.68`) |
| Notable linked libraries | `libgtk-4`, `libadwaita-1`, `libgtkmm-4.0`, `libglibmm`, `libsqlite3`, `libtinysparql-3` (also pulled by GTK), standard GTK stack |

The application is a **single-instance GApplication** with optional D-Bus service mode.

## 2. Runtime requirements

- A display (X11 or Wayland). Without one, every invocation prints
  `Gtk-WARNING **: Failed to open display` and exits with status **1** — including `--help`
  (GTK is initialised before argument handling).
- A D-Bus session bus (GSettings/dconf, GApplication single-instance wiring).
- A GSettings schema `xyz.riothedev.emojify` reachable through `GSETTINGS_SCHEMA_DIR` or the
  default schema sources. If it is missing, the app aborts immediately:
  `GLib-GIO-ERROR **: Settings schema 'xyz.riothedev.emojify' is not installed` (SIGABRT, exit 134).
  Missing *keys* produce the same style of abort, naming the first missing key
  (this is how the schema was reconstructed; see §5).

## 3. Command-line interface

```
Usage: emojify [OPTION…]
Help:     -h, --help | --help-all | --help-gapplication
Service:  --gapplication-service   (enter GApplication service mode)
```

Observed behaviour:

| Invocation | Behaviour |
|---|---|
| `emojify` (no instance running) | Shows the picker window |
| `emojify` (instance already running) | Activates the existing instance (shows its window); no second process |
| `emojify --gapplication-service` | Registers on the bus and idles **without a window**; later plain launches activate it and it presents its window |
| `emojify --help` / `-h` | Prints usage (requires a display) |
| `emojify --frobnicate` | Prints `Unknown option --frobnicate`, **continues** (window shows; the probe was killed by its timeout, exit not observed directly) |
| `emojify xyzzy` (positional args) | Ignored; GUI starts |
| no display | `Gtk-WARNING: Failed to open display`, exit 1, for *all* argument forms |

## 4. Main window

### 4.1 Layout

```
┌──────────────────────────────────────────────────────────┐
│ [⚙]  [ 🔍 Search emojis…                        (⌫)  (×)]│  top bar
├──────────────────────────────────────────────────────────┤
│  😀  😃  😄  😁  😆  …                    (scrollable)  │  emoji grid
│  …                                              │
├──────────────────────────────────────────────────────────┤
│   🕘   😀   ✋   🐶   🍎   ✈️   ⚽   💡   ❤️   🏁          │  category bar
└──────────────────────────────────────────────────────────┘
```

- **Top bar:** settings gear button (left), search entry with placeholder
  “Search emojis...”, an in-entry **clear button** (appears when text is present), and a
  window close button `×` (right).
- **Emoji grid:** square cells, one emoji glyph per cell; the focused cell gets a blue
  focus ring. The grid scrolls (mouse wheel; overlay scrollbar — not visible at rest).
- **Category bar:** ten tabs, each rendered as an emoji glyph; the active tab gets a
  blue rounded-pill background:

  | Tab | Category | First entries observed |
  |---|---|---|
  | 🕘 | Recents | most-recent-first user history (§6) |
  | 😀 | Smileys & emotion | 😀 😃 😄 😁 😆 😅 🤣 😂 🙂 🙃 |
  | ✋ | People & body | ✋ 🤚 🖐 ✌ 🤞 🤟 🤘 👌 🤏 👈 … 👂 👃 🧠 🫀 👀 👄 |
  | 🐶 | Animals & nature | 🐒 🐶 🐺 🐱 🦁 🐯 … 🐻 🐼 🐨 |
  | 🍎 | Food & drink | 🍇 🍈 🍉 🍊 🍋 🍋‍🟩 🍌 🍍 … 🥦 |
  | ✈️ | Travel & places | *(verified present; not catalogued)* |
  | ⚽ | Activities | *(verified present)* |
  | 💡 | Objects | 👓 🕶 🥼 👔 … 🎤 |
  | ❤️ | Symbols | 🏧 ♿ 🚰 ♿ 🚻 … ⚛ ☸ ☪ ☮ ✡ ☯ ✝ ☦ ☪ 🕎 🔱 |
  | 🏁 | Flags | 🏁 🚩 🎌 🏴 🏳️ 🏳️‍🌈 🏳️‍⚧️ 🏴‍☠️ then country flags in code order (🇦🇨 🇦🇩 🇦🇪 🇦🇫 …) |

- **Default geometry:** 573×400 px at 9 columns. The window width is content-driven:
  measured 573 → **633 px when columns was changed 9 → 10** (≈ 60 px per column ⇒
  `width ≈ 60·columns + 33`). Height stayed 400 px.
- **Skin tone:** every tone-supporting glyph (hands, 🤝, 👏, firefighters, …) is rendered
  with the configured skin tone (§5 `skin-tone`); verified for `medium`, `light`, `dark`.

### 4.2 Startup state

- **Category:** opens on **Recents** when history is non-empty; on the very first run
  (empty history) it opens on **Smileys 😀**.
- **Focus:** the first grid cell is focused on open and after each category switch;
  the search entry accepts typing immediately (type-to-search works without clicking).

### 4.3 Search

- Typing filters live across all categories. Query `fire` returned:
  🔥, firefighter (multiple ZWJ variants, tone-applied), 🚒, 🎆, 🧨, 🧯, ❤️‍🔥, 🦞, 🎇 —
  i.e. matching is done on emoji names/keywords (CLDR-annotation-like), not substrings of names only.
- Results are laid out in the same grid; the first result is focused; `Return` activates it.
- No-results state: an **empty grid** (no “no results” placeholder was shown).
- The clear button empties the entry and restores the previously active category.
- The category-bar pill does not change while searching.

### 4.4 Keyboard

| Key | Effect |
|---|---|
| letters/digits | go to search (type-to-search from launch) |
| `←` `→` `↑` `↓` | move the cell focus ring through the grid |
| `Return` | activate focused cell (same effect as clicking it) |
| `Escape` | dismiss the picker (§8) |
| mouse wheel | scrolls the grid |

## 5. Configuration (GSettings schema)

Reconstructed entirely from runtime abort messages plus observed UI behaviour.
This exact schema was compiled into a scratch dir and used to run the app for all sessions
(see §12). Key semantics were verified by toggling values and observing UI/process changes.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<schemalist>
  <!-- enum reconstructed: 6 values shown by the UI dropdown -->
  <enum id="xyz.riothedev.emojify.SkinTone">
    <value nick="default" value="0"/>
    <value nick="light" value="1"/>
    <value nick="medium-light" value="2"/>
    <value nick="medium" value="3"/>
    <value nick="medium-dark" value="4"/>
    <value nick="dark" value="5"/>
  </enum>
  <schema id="xyz.riothedev.emojify" path="/xyz/riothedev/emojify/">
    <key name="run-in-background" type="b"><default>false</default></key>
    <key name="columns" type="i"><default>9</default></key>
    <key name="skin-tone" enum="xyz.riothedev.emojify.SkinTone"><default>"default"</default></key>
    <key name="paste-on-select" type="b"><default>false</default></key>
    <key name="multi-emoji" type="b"><default>false</default></key>
    <key name="timeout-ms" type="i"><default>0</default></key>
  </schema>
</schemalist>
```

*True shipped defaults for `columns` (UI showed 9) and `timeout-ms` (UI displayed 50 while
the key held 0) are our best reading; the app clamps/displays a minimum of 50 in the spinner.*

| Key | Type | Observed semantics |
|---|---|---|
| `run-in-background` | bool | `false`: closing (× or Escape) **terminates the process** (verified exit). `true`: closing **hides** the window; the process keeps running; a later launch re-presents the window (“faster startup” per its UI description). |
| `columns` | int | Grid column count. Applied **live** (grid re-renders, window width follows). Changed by the Preferences spinner (step 1). |
| `skin-tone` | enum | Default tone for tone-supporting emoji. Dropdown of 6 ✌ variants; checkmark on the active one. Applied to grid and search results. |
| `paste-on-select` | bool | “Paste automatically — paste the selected emoji directly into the focused window (requires the GNOME Shell extension)”. The toggle is **insensitive** while the extension is absent; the row shows a red “● Not installed” badge. |
| `multi-emoji` | bool | “Select multiple emojis consecutively before pasting”. `true`: clicking emojis keeps the window open and records each into recents; the paste step happens after `timeout-ms` following the last selection (or on `Return`). `false`: a selection immediately dismisses the window. |
| `timeout-ms` | int | “Paste delay (ms) — time to wait after the last emoji selection before pasting”. Spinner step **50** via ± buttons. With `multi-emoji` and a 2000 ms value the window dismissed ≈2 s after the last selection (no extension present, so no paste was observable). |

## 6. Persistent state (files)

- `~/.local/share/emojify/recents.txt` (under `XDG_DATA_HOME`)
  - One emoji per line, **most-recent-first**, de-duplicated (re-selecting moves it to the top).
  - Updated on every selection (file mtime advanced per click).
  - **Capacity: 41 entries** — the list grew to 41 and then stayed at exactly 41 while new
    emojis were added (oldest evicted).
  - Not stored in GSettings (no corresponding key).
- GSettings/dconf database holds the six keys of §5.
- No other state files were observed in `XDG_{DATA,STATE,CONFIG}_HOME` besides GTK/GLib caches.

## 7. Preferences window

Opened by the gear button. A **separate 600×770 window** titled `Preferences`
(in our WM-less X11 session it stacked over the main window and had **no visible headerbar
and no close button**; `Escape` and `Ctrl+W` did not close it). Content (a scrolled page with
grouped cards and a footer):

```
Extension
  Paste automatically     [toggle, insensitive]  ● Not installed
    "Paste the selected emoji directly into the focused window
     (requires the GNOME Shell extension)"
  GNOME Shell Extension                     [🔗 Get Extension]
    "Required for automatic pasting — opens extensions.gnome.org"
Behavior
  Multi-emoji mode        [toggle]
    "Select multiple emojis consecutively before pasting"
  Paste delay (ms)        [ 50 | − | + ]   (dimmed while value is 0, still operable)
    "Time to wait after the last emoji selection before pasting"
Appearance
  Skin tone               [✌ ▾]  (6 ✌ variants; ✓ on active)
    "Default skin tone used for emojis that support it"
  Columns                 [ 9 | − | + ]
    "Number of columns in the emoji grid"
System
  Run in background       [toggle]
    "Keep the app running after closing for faster startup"
  Keyboard shortcut
    "Add this command to your system's keyboard settings
     to open Emojify with a shortcut"
    [ /usr/bin/emojify            ] [⧉]
Footer: Emojify v1.3.3 • Built with Meson 1.12.1
```

- **Keyboard-shortcut row:** a read-only entry showing the resolved launcher command plus a
  copy button. Resolution observed:
  - if the environment defines `APPIMAGE`/`ARGV0` (AppImage conventions), their value is used —
    in our harness the host leaked `APPIMAGE=/home/…/ZCode-….AppImage` and the app copied that;
  - otherwise the resolved command was **`/usr/bin/emojify`** even though the binary under test
    lived at `./build/src/emojify` (i.e. it is not derived from the actual executable path).
- The copy button **does write to the X11 `CLIPBOARD` selection** (verified: captured
  `/usr/bin/emojify` via an XFixes selection monitor).
- “Get Extension” was deliberately **not clicked** (it opens extensions.gnome.org — an
  external action); documented from its label only.

## 8. Selection, clipboard & paste pipeline

Verified with an XFixes-based selection monitor plus an X11 clipboard persistor running
alongside the app:

- **Clicking an emoji never touches the X11 `CLIPBOARD` or `PRIMARY` selection** —
  zero ownership events were observed for both selections across single- and multi-select
  tests, with `paste-on-select` both false and true.
- What a selection *does* do: append/move the emoji in `recents.txt` (§6) and dismiss the
  window (single-emoji mode) or keep it open (multi-emoji mode).
- The output path is therefore the **GNOME Shell extension** (“required for automatic
  pasting”): the app delegates pasting into the focused window to the extension. Under
  Xvfb without the extension, no paste/copy side effect exists; the picker simply dismisses.
- Consequence for integrators: on X11 without the extension the app is **not** a
  clipboard-writing picker; treat it as a Wayland/Shell-extension-first tool.

## 9. Lifecycle semantics

| Scenario | Observed |
|---|---|
| second launch while an instance runs | single-instance: activates and presents the existing window; no second process. Re-launch while visible does **not** hide it (present, not toggle). |
| `--gapplication-service` | idles without a window; plain launches activate it and it presents its window |
| × / Escape, `run-in-background=false` | process terminates (verified exit) |
| × / Escape, `run-in-background=true` | window hidden; process stays; next launch re-presents |
| external GSettings write (any key, via `gsettings set`) | a *normally launched* instance **exits cleanly (status 0, no log output)**; a `--gapplication-service` instance applies the change live and survives (verified for `skin-tone`, `columns`) |
| `timeout-ms` write while the Preferences window was open (early session) | one unreplicated observation of the value being applied live without exit |

## 10. Theming

- Follows the libadwaita/system color scheme: setting
  `org.gnome.desktop.interface color-scheme` to `prefer-dark` switched the UI to dark
  (verified). Light by default in our sandbox.
- `GTK_THEME=Adwaita-dark` had **no effect** (ignored by libadwaita).

## 11. Observed quirks / robustness findings

1. **Hard dependency on the schema:** missing schema or any missing key ⇒ SIGABRT at startup
   (each missing key is revealed one at a time — see §12 recipe).
2. **Self-exit on external settings writes** (§9): a plain instance exits with status 0, silently,
   when any of its settings keys is modified externally (e.g. `gsettings set`). Looks like an
   “apply by restart” strategy that relies on a D-Bus/desktop-file activator re-launching it;
   without an activator it simply dies. `--gapplication-service` instances are immune.
3. The keyboard-shortcut command resolves via `APPIMAGE`/`ARGV0` and otherwise assumes
   `/usr/bin/emojify` — misleading when running from a build tree.
4. Preferences window had no close affordance in our WM-less environment; Escape/Ctrl+W did
   nothing (may differ under a real WM).
5. No-results search shows an empty grid with no message.
6. `GTK_THEME` ignored (libadwaita colour-scheme is authoritative).

## 12. Replication recipe (verification harness)

Everything below was used to run the app isolated from the desktop; it is sufficient to
reproduce every observation in this document.

```bash
# sandbox + private bus + virtual display (no WM needed; GTK4 uses CSD)
SB=/tmp/ai-agent/emojify-sandbox
mkdir -p $SB/{config,cache,data,state,run,shots,schema}; chmod 700 $SB/run
Xvfb :77 -screen 0 1280x900x24 -nolisten tcp -noreset &
dbus-daemon --session --fork --address=unix:path=$SB/run/bus

# compile the reconstructed schema (§5) — first run aborts naming the first
# missing key; add that key, recompile, re-run until the UI opens
glib-compile-schemas $SB/schema

# run
env DISPLAY=:77 GDK_BACKEND=x11 DBUS_SESSION_BUS_ADDRESS=unix:path=$SB/run/bus \
    XDG_CONFIG_HOME=$SB/config XDG_CACHE_HOME=$SB/cache XDG_DATA_HOME=$SB/data \
    XDG_STATE_HOME=$SB/state XDG_RUNTIME_DIR=$SB/run \
    NO_AT_BRIDGE=1 GTK_A11Y=none unset-WAYLAND_DISPLAY \
    GSETTINGS_SCHEMA_DIR=$SB/schema ./build/src/emojify
```

Screen observation: `magick import -display :77 -window root shot.png`;
input: `xdotool mousemove/click/key/type`; window discovery: `xdotool search --onlyvisible`.
Clipboard verification requires a selection persistor (the app disowns selections when its
window unmaps) — an XFixes-based manager was written for this harness.

## 13. Not verified / open questions

- Actual paste behaviour with the GNOME Shell extension installed (Wayland side).
- Whether the 41-entry recents cap is exact by design or configuration-dependent.
- Bounds of the `columns` spinner and `timeout-ms` range (spinner limits never reached).
- Real-WM behaviour of the Preferences window (headerbar/close button).
- Escape/Ctrl+W handling of the Preferences window under a real WM.
- Whether the “self-exit on external write” is intentional restart-on-apply or a defect.
