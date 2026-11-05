//! The picker window: top bar (gear / search / close), emoji grid, category bar.

use libadwaita::prelude::*;
use gtk::{gdk, gio, glib};
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::emoji_data::{EMOJIS, TONE_VARIANTS};
use crate::extension::ExtensionState;
use crate::prefs;
use crate::store::Recents;
use crate::APP_ID;

/// (glyph, tooltip) for the ten category tabs, in display order.
const CATEGORIES: [(&str, &str); 10] = [
    ("🕘", "Recents"),
    ("😀", "Smileys"),
    ("✋", "People"),
    ("🐶", "Animals"),
    ("🍎", "Food"),
    ("✈️", "Travel"),
    ("⚽", "Activities"),
    ("💡", "Objects"),
    ("❤️", "Symbols"),
    ("🏁", "Flags"),
];

const CAT_RECENTS: u8 = 0;

/// Resolve the skin-tone setting robustly: dconf may hold the enum either as
/// its integer value or as the nick string, depending on the writer.
pub fn skin_tone_value(settings: &gio::Settings) -> usize {
    let value = settings.value("skin-tone");
    if let Some(nick) = value.get::<String>() {
        return match nick.as_str() {
            "light" => 1,
            "medium-light" => 2,
            "medium" => 3,
            "medium-dark" => 4,
            "dark" => 5,
            "all" => 6,
            _ => 0,
        };
    }
    value.get::<i32>().unwrap_or(0).clamp(0, 6) as usize
}

/// Width ≈ 60·columns + 33 (e.g. 573 px at 9 columns); height matches the
/// original picker (400 px).
pub const WINDOW_HEIGHT: i32 = 400;

/// skin-tone enum value meaning "show every tone variant at once".
pub const TONE_ALL: usize = 6;

fn window_width_for_columns(columns: i32) -> i32 {
    60 * columns + 33
}

const CSS: &str = r#"
flowboxchild {
    padding: 0;
}
.emoji-cell {
    background: @card_bg_color;
    color: @window_fg_color;
    border: none;
    box-shadow: none;
    border-radius: 12px;
    font-size: 28px;
    min-width: 52px;
    min-height: 52px;
    padding: 0;
}
.emoji-cell:hover {
    filter: brightness(1.15);
}
.emoji-cell:focus {
    outline: 2px solid @accent_color;
    outline-offset: 1px;
}
.category-tab {
    font-size: 14px;
    min-height: 30px;
    padding: 0 4px;
    border-radius: 12px;
    background: none;
    border: none;
    box-shadow: none;
    outline: none;
}
.category-tab.active {
    background: alpha(@accent_bg_color, 0.30);
}
.not-installed {
    color: @error_color;
}
"#;

thread_local! {
    static PICKER: RefCell<Option<Rc<Picker>>> = const { RefCell::new(None) };
}

pub struct Picker {
    app: adw::Application,
    window: adw::ApplicationWindow,
    gear: gtk::Button,
    close: gtk::Button,
    entry: gtk::SearchEntry,
    flow: gtk::FlowBox,
    tabs: Vec<gtk::Button>,
    settings: gio::Settings,
    recents: Rc<Recents>,
    category: Cell<u8>,
    /// Category shown again when the search is cleared.
    restore_category: Cell<u8>,
    /// Guard: a programmatic entry clear (category switch) is pending.
    switching: Cell<bool>,
    searching: Cell<bool>,
    multi_buffer: RefCell<String>,
    timer: RefCell<Option<glib::SourceId>>,
    cells: RefCell<Vec<(gtk::Button, String)>>,
}

impl Picker {
    pub fn present(_app: &adw::Application) {
        PICKER.with(|slot| {
            if let Some(picker) = slot.borrow().as_ref() {
                picker.window.present();
                picker.focus_first();
            }
        });
    }

    pub fn get_or_create(app: &adw::Application) -> Rc<Picker> {
        PICKER.with(|slot| {
            if let Some(picker) = slot.borrow().as_ref() {
                return picker.clone();
            }
            let picker = Picker::new(app);
            *slot.borrow_mut() = Some(picker.clone());
            picker
        })
    }

    fn new(app: &adw::Application) -> Rc<Picker> {
        let settings = gio::Settings::new(APP_ID);
        let recents = Rc::new(Recents::load());

        load_css();

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Emoxidize")
            .default_width(window_width_for_columns(settings.int("columns").clamp(5, 15)))
            .default_height(WINDOW_HEIGHT)
            .build();

        // --- top bar: [gear] [search entry] [close] ------------------------
        let header = gtk::HeaderBar::new();
        header.set_show_title_buttons(false);

        let gear = gtk::Button::new();
        gear.set_icon_name("preferences-system-symbolic");
        gear.add_css_class("flat");
        gear.add_css_class("circular");
        gear.set_tooltip_text(Some("Preferences"));

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some("Search emojis..."));
        // Keep the entry's natural width small so the window width is driven
        // by the grid (width ≈ 60·columns + 33), not by the top bar.
        entry.set_width_chars(18);
        entry.set_hexpand(true);
        entry.set_valign(gtk::Align::Center);

        let close = gtk::Button::new();
        close.set_icon_name("window-close-symbolic");
        close.add_css_class("flat");
        close.add_css_class("circular");
        close.set_tooltip_text(Some("Close"));

        header.set_title_widget(Some(&entry));
        header.pack_start(&gear);
        header.pack_end(&close);

        // --- emoji grid -----------------------------------------------------
        let flow = gtk::FlowBox::new();
        flow.set_selection_mode(gtk::SelectionMode::None);
        flow.set_activate_on_single_click(true);
        flow.set_column_spacing(6);
        flow.set_row_spacing(6);
        flow.set_margin_top(8);
        flow.set_margin_bottom(8);
        flow.set_margin_start(8);
        flow.set_margin_end(8);
        flow.set_halign(gtk::Align::Center);
        flow.set_valign(gtk::Align::Start);

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scrolled.set_child(Some(&flow));
        scrolled.set_vexpand(true);

        // --- category bar ---------------------------------------------------
        let cat_bar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        cat_bar.set_homogeneous(true);
        cat_bar.set_hexpand(true);
        cat_bar.set_margin_top(4);
        cat_bar.set_margin_bottom(5);
        cat_bar.set_margin_start(4);
        cat_bar.set_margin_end(4);
        let mut tabs = Vec::with_capacity(CATEGORIES.len());
        for (glyph, name) in CATEGORIES {
            let tab = gtk::Button::with_label(glyph);
            tab.add_css_class("category-tab");
            tab.set_tooltip_text(Some(name));
            // VS16-carrying glyphs (✈️, ❤️) must not inflate the bar.
            if let Some(label) = tab.child().and_downcast::<gtk::Label>() {
                label.set_max_width_chars(1);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            }
            cat_bar.append(&tab);
            tabs.push(tab);
        }

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&header);
        root.append(&scrolled);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&cat_bar);
        window.set_content(Some(&root));

        let picker = Rc::new(Picker {
            app: app.clone(),
            window,
            gear,
            close,
            entry,
            flow,
            tabs,
            settings,
            recents,
            category: Cell::new(CAT_RECENTS),
            restore_category: Cell::new(CAT_RECENTS),
            switching: Cell::new(false),
            searching: Cell::new(false),
            multi_buffer: RefCell::new(String::new()),
            timer: RefCell::new(None),
            cells: RefCell::new(Vec::new()),
        });

        picker.wire_signals();
        // Start watching the paste bridge as early as possible so its proxy
        // is ready by the time the first emoji is selected.
        ExtensionState::get();
        // Startup category: Recents when history is non-empty, else Smileys.
        let start = if picker.recents.is_empty() { 1 } else { CAT_RECENTS };
        picker.show_category(start);
        picker.apply_columns(true);

        // Mirror the freshly loaded file into GSettings (no-op when equal).
        let list = picker.recents.list();
        if picker.recents_value() != list {
            picker.mirror_recents(&list);
        }

        picker
    }

    fn wire_signals(self: &Rc<Self>) {
        let selfg = self.clone();
        self.gear.connect_clicked(move |_| selfg.open_prefs());

        let selfclose = self.clone();
        self.close.connect_clicked(move |_| selfclose.dismiss());

        let selfsearch = self.clone();
        self.entry.connect_search_changed(move |_| {
            selfsearch.on_search_changed()
        });

        let selfact = self.clone();
        self.entry.connect_activate(move |_| {
            // Return with focus in the entry activates the first result.
            selfact.activate_first();
        });

        // Up/Down from the search entry jump into the grid.
        let selfentrykey = self.clone();
        let entry_keys = gtk::EventControllerKey::new();
        entry_keys.connect_key_pressed(move |_, key, _, _| {
            if matches!(key, gdk::Key::Up | gdk::Key::Down) {
                selfentrykey.focus_first();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.entry.add_controller(entry_keys);

        // Window-level keys (capture phase): Escape dismisses, typing goes to
        // the search entry without clicking it first.
        let selfkey = self.clone();
        let window_keys = gtk::EventControllerKey::new();
        window_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        window_keys.connect_key_pressed(move |_, key, _, state| {
            use gdk::Key;
            if key == Key::Escape {
                selfkey.dismiss();
                return glib::Propagation::Stop;
            }
            let dangerous = state.intersects(
                gdk::ModifierType::CONTROL_MASK
                    | gdk::ModifierType::ALT_MASK
                    | gdk::ModifierType::META_MASK
                    | gdk::ModifierType::SUPER_MASK
                    | gdk::ModifierType::HYPER_MASK,
            );
            if !dangerous {
                // Arrow navigation across the grid (FlowBox does not move
                // focus between focusable button children on its own).
                let columns = selfkey.settings.int("columns").clamp(5, 15);
                let delta: i32 = match key {
                    Key::Right => 1,
                    Key::Left => -1,
                    Key::Down => columns,
                    Key::Up => -columns,
                    _ => 0,
                };
                if delta != 0 && selfkey.move_cell_focus(delta) {
                    return glib::Propagation::Stop;
                }
                if let Some(ch) = key.to_unicode() {
                    if !ch.is_control() && ch != ' ' {
                        if !selfkey.entry.has_focus() {
                            selfkey.entry.grab_focus();
                        }
                        entry_insert_char(&selfkey.entry, ch);
                        return glib::Propagation::Stop;
                    }
                }
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(window_keys);

        let selfclose_req = self.clone();
        self.window.connect_close_request(move |_| {
            selfclose_req.dismiss();
            glib::Propagation::Stop
        });

        // Category tabs.
        for (idx, tab) in self.tabs.iter().enumerate() {
            let selftab = self.clone();
            let idx = idx as u8;
            tab.connect_clicked(move |_| {
                selftab.switch_to_category(idx);
            });
        }

        // Live-apply settings changes (external writes must not crash us).
        for key in [
            "columns",
            "skin-tone",
            "multi-emoji",
            "timeout-ms",
            "run-in-background",
            "paste-on-select",
            "recents",
        ] {
            let selfset = self.clone();
            let key = key.to_string();
            self.settings
                .connect_changed(Some(key.as_str()), move |_, changed| {
                    selfset.apply_setting(changed);
                });
        }
    }

    fn open_prefs(self: &Rc<Self>) {
        prefs::open(&self.window, &self.settings);
    }

    // ---------------------------------------------------------------- state --

    fn apply_setting(self: &Rc<Self>, key: &str) {
        match key {
            "columns" => self.apply_columns(true),
            "skin-tone" => self.rerender(),
            "multi-emoji" => {
                self.multi_buffer.borrow_mut().clear();
                if let Some(id) = self.timer.borrow_mut().take() {
                    id.remove();
                }
            }
            "recents" => {
                let incoming = self.recents_value();
                if incoming != self.recents.list() {
                    self.recents.replace(incoming);
                    if self.category.get() == CAT_RECENTS && !self.searching.get() {
                        self.populate(self.recents.list(), false);
                    }
                }
            }
            // timeout-ms / run-in-background / paste-on-select need no
            // immediate action in the picker; the Preferences window syncs
            // itself.
            _ => {}
        }
    }

    fn apply_columns(self: &Rc<Self>, resize: bool) {
        let columns = self.settings.int("columns").clamp(5, 15);
        self.flow.set_min_children_per_line(columns as u32);
        self.flow.set_max_children_per_line(columns as u32);
        if resize {
            self.window
                .set_default_size(window_width_for_columns(columns), WINDOW_HEIGHT);
        }
    }

    fn rerender(self: &Rc<Self>) {
        if self.searching.get() {
            let q = self.entry.text().to_string();
            self.run_search(&q);
        } else {
            let cat = self.category.get();
            let items = self.category_items(cat);
            self.populate(items, false);
        }
    }

    fn category_items(self: &Rc<Self>, cat: u8) -> Vec<String> {
        if cat == CAT_RECENTS {
            self.recents.list()
        } else {
            let mut items = Vec::new();
            for e in EMOJIS.iter().filter(|e| e.cat == cat) {
                self.push_toned(e.ch, &mut items);
            }
            items
        }
    }

    /// Append `ch`; when the skin-tone setting is "all", append the base plus
    /// every tone variant of tone-capable emojis.
    fn push_toned(self: &Rc<Self>, ch: &str, out: &mut Vec<String>) {
        if self.enum_tone() == TONE_ALL {
            out.push(ch.to_string());
            if let Some((_, variants)) = TONE_VARIANTS.iter().find(|(base, _)| *base == ch) {
                for v in variants {
                    out.push(v.to_string());
                }
            }
        } else {
            out.push(self.apply_tone(ch));
        }
    }

    fn show_category(self: &Rc<Self>, cat: u8) {
        self.category.set(cat);
        self.searching.set(false);
        let items = self.category_items(cat);
        self.populate(items, true);
        self.update_pill();
    }

    fn switch_to_category(self: &Rc<Self>, cat: u8) {
        // Clear the search (if any) without treating it as a user clear; the
        // delayed search-changed signal is swallowed by the guard flag.
        if !self.entry.text().is_empty() {
            self.switching.set(true);
            self.entry.set_text("");
        }
        self.show_category(cat);
    }

    fn on_search_changed(self: &Rc<Self>) {
        if self.switching.replace(false) {
            return;
        }
        let query = self.entry.text().to_string();
        if query.is_empty() {
            if self.searching.get() {
                self.show_category(self.restore_category.get());
            }
        } else {
            if !self.searching.get() {
                self.restore_category.set(self.category.get());
            }
            self.searching.set(true);
            self.run_search(&query);
        }
    }

    fn run_search(self: &Rc<Self>, query: &str) {
        let ql = query.to_lowercase();
        let mut scored: Vec<(u8, usize)> = Vec::new();
        for (i, e) in EMOJIS.iter().enumerate() {
            let score = if e.name == ql {
                0
            } else if e.name.starts_with(&ql) {
                1
            } else if e.name.contains(&ql) {
                2
            } else if e.keywords.split_whitespace().any(|k| k.starts_with(&ql)) {
                3
            } else if e.keywords.contains(&ql) {
                4
            } else {
                continue;
            };
            scored.push((score, i));
        }
        scored.sort_unstable();
        let mut items = Vec::new();
        for (_, i) in scored {
            self.push_toned(EMOJIS[i].ch, &mut items);
        }
        // Keep focus in the entry while results refresh; grabbing the first
        // cell here would strand Backspace/Ctrl+A on a grid button.
        self.populate(items, false);
        // The category pill deliberately stays untouched while searching.
    }

    fn populate(self: &Rc<Self>, items: Vec<String>, focus_first: bool) {
        while let Some(child) = self.flow.first_child() {
            self.flow.remove(&child);
        }
        let mut cells = Vec::with_capacity(items.len());
        for ch in items {
            let button = gtk::Button::with_label(&ch);
            button.add_css_class("emoji-cell");
            // Fixed square cells: wide glyphs (ZWJ sequences, flags) must not
            // inflate the whole grid; they overflow their tile slightly.
            button.set_size_request(54, 54);
            button.set_focusable(true);
            if let Some(label) = button.child().and_downcast::<gtk::Label>() {
                // Cap the label's *requested* width (max-width-chars only
                // applies with ellipsize set); the allocation stays above the
                // request, so glyphs still paint unclipped and overflow wide
                // family/flag sequences slightly.
                label.set_max_width_chars(1);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            }
            let picker = self.clone();
            let ch_clone = ch.clone();
            button.connect_clicked(move |_| {
                picker.select(&ch_clone);
            });
            self.flow.append(&button);
            cells.push((button, ch));
        }
        *self.cells.borrow_mut() = cells;
        if focus_first {
            self.focus_first();
        }
    }

    fn focus_first(self: &Rc<Self>) {
        if let Some((button, _)) = self.cells.borrow().first() {
            button.grab_focus();
        }
    }

    /// Move the cell focus by `delta` positions (arrows). Returns false when
    /// no grid cell currently has the focus, so the key keeps its normal
    /// meaning elsewhere (e.g. cursor movement in the search entry).
    fn move_cell_focus(self: &Rc<Self>, delta: i32) -> bool {
        let cells = self.cells.borrow();
        let current = cells.iter().position(|(b, _)| b.has_focus());
        let Some(current) = current else {
            return false;
        };
        let target = (current as i32 + delta).clamp(0, cells.len() as i32 - 1) as usize;
        cells[target].0.grab_focus();
        true
    }

    fn activate_first(self: &Rc<Self>) {
        let first = self.cells.borrow().first().map(|(_, ch)| ch.clone());
        if let Some(ch) = first {
            self.select(&ch);
        }
    }

    fn update_pill(self: &Rc<Self>) {
        let active = self.category.get();
        for (idx, tab) in self.tabs.iter().enumerate() {
            if idx as u8 == active {
                tab.set_css_classes(&["category-tab", "active"]);
            } else {
                tab.set_css_classes(&["category-tab"]);
            }
        }
    }

    // ----------------------------------------------------------- selection --

    fn apply_tone(self: &Rc<Self>, ch: &str) -> String {
        let tone = self.enum_tone();
        if tone == 0 {
            return ch.to_string();
        }
        TONE_VARIANTS
            .iter()
            .find(|(base, _)| *base == ch)
            .map(|(_, variants)| variants[tone - 1].to_string())
            .unwrap_or_else(|| ch.to_string())
    }

    fn enum_tone(self: &Rc<Self>) -> usize {
        skin_tone_value(&self.settings)
    }

    fn recents_value(self: &Rc<Self>) -> Vec<String> {
        self.settings.value("recents").get::<Vec<String>>().unwrap_or_default()
    }

    fn mirror_recents(self: &Rc<Self>, list: &[String]) {
        let _ = self.settings.set_value("recents", &list.to_variant());
    }

    fn copy_to_clipboard(self: &Rc<Self>, text: &str) {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(text);
        }
    }

    fn select(self: &Rc<Self>, ch: &str) {
        self.recents.touch(ch);
        // Mirror into GSettings; our own changed handler sees identical
        // content and does nothing.
        let list = self.recents.list();
        self.mirror_recents(&list);

        if self.category.get() == CAT_RECENTS && !self.searching.get() {
            self.populate(self.recents.list(), false);
        }

        let extension = ExtensionState::get();
        let bridge_paste = self.settings.boolean("paste-on-select") && extension.is_present();

        if self.settings.boolean("multi-emoji") {
            self.multi_buffer.borrow_mut().push_str(ch);
            let buffer = self.multi_buffer.borrow().clone();
            if !bridge_paste {
                self.copy_to_clipboard(&buffer);
            }
            let ms = self.settings.int("timeout-ms").max(0);
            if ms > 0 {
                self.restart_timer(ms as u64);
            }
        } else if bridge_paste && extension.paste(ch) {
            self.dismiss();
        } else {
            self.copy_to_clipboard(ch);
            self.dismiss();
        }
    }

    fn restart_timer(self: &Rc<Self>, ms: u64) {
        if let Some(old) = self.timer.borrow_mut().take() {
            old.remove();
        }
        let picker = self.clone();
        let id = glib::timeout_add_local(Duration::from_millis(ms), move || {
            let buffer = picker.multi_buffer.borrow().clone();
            if picker.settings.boolean("paste-on-select")
                && !buffer.is_empty()
                && ExtensionState::get().paste(&buffer)
            {
                // handed to the bridge
            } else if !buffer.is_empty() {
                picker.copy_to_clipboard(&buffer);
            }
            picker.dismiss();
            glib::ControlFlow::Break
        });
        *self.timer.borrow_mut() = Some(id);
    }

    fn dismiss(self: &Rc<Self>) {
        if self.settings.boolean("run-in-background") {
            self.window.set_visible(false);
        } else {
            self.app.quit();
        }
    }
}

/// Insert a character at the editable's cursor and leave the caret after it.
/// A pending selection (Ctrl+A, shift-extended) is replaced, as in a plain
/// text editor.
fn entry_insert_char(entry: &gtk::SearchEntry, ch: char) {
    let text = entry.text().to_string();
    let cursor = entry.property::<i32>("cursor-position").max(0) as usize;
    let bound = entry.property::<i32>("selection-bound-pos").max(0) as usize;
    let (start, end) = if cursor == bound {
        (cursor, cursor)
    } else {
        (cursor.min(bound), cursor.max(bound))
    };
    let byte = |idx: usize| {
        text.char_indices()
            .nth(idx)
            .map(|(b, _)| b)
            .unwrap_or(text.len())
    };
    let mut updated = String::with_capacity(text.len() + ch.len_utf8());
    updated.push_str(&text[..byte(start)]);
    updated.push(ch);
    updated.push_str(&text[byte(end)..]);
    entry.set_text(&updated);
    entry.set_position((start + 1) as i32);
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
