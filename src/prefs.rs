//! Preferences window: Extension / Behavior / Appearance / System sections
//! and a version footer. Created once and re-presented; hidden on close so
//! its GSettings handlers stay attached to live widgets.

use libadwaita::prelude::*;
use gtk::{gdk, gio, glib};
use libadwaita as adw;

use crate::extension::ExtensionState;
use crate::window::skin_tone_value;
use std::cell::RefCell;

thread_local! {
    static PREFS: RefCell<Option<adw::Window>> = const { RefCell::new(None) };
}

pub fn open(parent: &adw::ApplicationWindow, settings: &gio::Settings) {
    PREFS.with(|slot| {
        if let Some(win) = slot.borrow().as_ref() {
            win.present();
            return;
        }

        let win = adw::Window::builder()
            .title("Preferences")
            .transient_for(parent)
            .modal(true)
            .default_width(600)
            .default_height(770)
            .hide_on_close(true)
            .build();

        // Escape / Ctrl+W close the window.
        let keys = gtk::EventControllerKey::new();
        let win_for_keys = win.clone();
        keys.connect_key_pressed(move |_, key, _, state| {
            use gdk::Key;
            let escape = key == Key::Escape;
            let ctrl_w = state.contains(gdk::ModifierType::CONTROL_MASK) && key == Key::w;
            if escape || ctrl_w {
                win_for_keys.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        win.add_controller(keys);

        let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
        page.set_margin_top(12);
        page.set_margin_bottom(12);
        page.set_margin_start(12);
        page.set_margin_end(12);

        // ------------------------------------------------------ Extension --
        section(&page, "Extension");

        let group = adw::PreferencesGroup::new();
        let paste_row = adw::SwitchRow::builder()
            .title("Paste automatically")
            .subtitle("Paste the selected emoji directly into the focused window (requires the GNOME Shell extension)")
            .sensitive(false)
            .build();
        paste_row.set_active(settings.boolean("paste-on-select"));
        paste_row.connect_active_notify({
            let settings = settings.clone();
            move |row| {
                let _ = settings.set_boolean("paste-on-select", row.is_active());
            }
        });
        settings.connect_changed(Some("paste-on-select"), {
            let row = paste_row.clone();
            move |settings, _| {
                let v = settings.boolean("paste-on-select");
                if row.is_active() != v {
                    row.set_active(v);
                }
            }
        });
        let badge = gtk::Label::new(Some("● Not installed"));
        badge.add_css_class("not-installed");
        badge.set_valign(gtk::Align::Center);
        ExtensionState::get().connect_changed({
            let badge = badge.clone();
            let row = paste_row.clone();
            move |present| {
                row.set_sensitive(present);
                if present {
                    badge.set_text("● Installed");
                    badge.remove_css_class("not-installed");
                    badge.add_css_class("installed");
                } else {
                    badge.set_text("● Not installed");
                    badge.remove_css_class("installed");
                    badge.add_css_class("not-installed");
                }
            }
        });
        paste_row.add_suffix(&badge);
        group.add(&paste_row);

        let ext_row = adw::ActionRow::builder()
            .title("GNOME Shell Extension")
            .subtitle("Required for automatic pasting — opens extensions.gnome.org")
            .build();
        let get_ext = gtk::LinkButton::builder()
            .label("Get Extension")
            .uri("https://extensions.gnome.org")
            .build();
        get_ext.set_valign(gtk::Align::Center);
        ext_row.add_suffix(&get_ext);
        group.add(&ext_row);
        page.append(&group);

        // -------------------------------------------------------- Behavior --
        section(&page, "Behavior");

        let group = adw::PreferencesGroup::new();
        let multi_row = adw::SwitchRow::builder()
            .title("Multi-emoji mode")
            .subtitle("Select multiple emojis consecutively before pasting")
            .build();
        multi_row.set_active(settings.boolean("multi-emoji"));
        multi_row.connect_active_notify({
            let settings = settings.clone();
            move |row| {
                let _ = settings.set_boolean("multi-emoji", row.is_active());
            }
        });
        settings.connect_changed(Some("multi-emoji"), {
            let row = multi_row.clone();
            move |settings, _| {
                let v = settings.boolean("multi-emoji");
                if row.is_active() != v {
                    row.set_active(v);
                }
            }
        });
        group.add(&multi_row);

        let delay_row = adw::ActionRow::builder()
            .title("Paste delay (ms)")
            .subtitle("Time to wait after the last emoji selection before pasting")
            .build();
        let delay_spin = gtk::SpinButton::with_range(0.0, 1000.0, 50.0);
        delay_spin.set_valign(gtk::Align::Center);
        delay_spin.set_value(f64::from(settings.int("timeout-ms")));
        delay_spin.connect_value_changed({
            let settings = settings.clone();
            move |spin| {
                let v = spin.value() as i32;
                if settings.int("timeout-ms") != v {
                    let _ = settings.set_int("timeout-ms", v);
                }
            }
        });
        settings.connect_changed(Some("timeout-ms"), {
            let spin = delay_spin.clone();
            move |settings, _| {
                let v = settings.int("timeout-ms");
                if spin.value() as i32 != v {
                    spin.set_value(f64::from(v));
                }
            }
        });
        // Dimmed (but operable) while multi-emoji is off or the value is 0.
        {
            let row = delay_row.clone();
            let owned = settings.clone();
            let update_dim = move || {
                let dim = !owned.boolean("multi-emoji") || owned.int("timeout-ms") == 0;
                if dim {
                    row.add_css_class("dimmed");
                } else {
                    row.remove_css_class("dimmed");
                }
            };
            update_dim();
            settings.connect_changed(Some("multi-emoji"), {
                let update_dim = update_dim.clone();
                move |_, _| update_dim()
            });
            settings.connect_changed(Some("timeout-ms"), {
                move |_, _| update_dim()
            });
        }
        delay_row.add_suffix(&delay_spin);
        group.add(&delay_row);
        page.append(&group);

        // ------------------------------------------------------ Appearance --
        section(&page, "Appearance");

        let group = adw::PreferencesGroup::new();
        let tone_row = adw::ActionRow::builder()
            .title("Skin tone")
            .subtitle("Default skin tone used for emojis that support it")
            .build();
        let tones = ["👍", "👍🏻", "👍🏼", "👍🏽", "👍🏾", "👍🏿", "👍🏻👍🏿"];
        let model = gtk::StringList::new(&tones);
        let tone_dd = gtk::DropDown::new(
            Some(model),
            Some(gtk::StringObject::this_expression("string")),
        );
        tone_dd.set_valign(gtk::Align::Center);
        tone_dd.set_selected(skin_tone_value(settings) as u32);
        tone_dd.connect_selected_notify({
            let settings = settings.clone();
            move |dd| {
                let v = dd.selected() as i32;
                if settings.enum_("skin-tone") != v {
                    let _ = settings.set_enum("skin-tone", v);
                }
            }
        });
        settings.connect_changed(Some("skin-tone"), {
            let dd = tone_dd.clone();
            move |settings, _| {
                let v = skin_tone_value(settings) as u32;
                if dd.selected() != v {
                    dd.set_selected(v);
                }
            }
        });
        tone_row.add_suffix(&tone_dd);
        group.add(&tone_row);

        let cols_row = adw::ActionRow::builder()
            .title("Columns")
            .subtitle("Number of columns in the emoji grid")
            .build();
        let cols_spin = gtk::SpinButton::with_range(5.0, 15.0, 1.0);
        cols_spin.set_valign(gtk::Align::Center);
        cols_spin.set_value(f64::from(settings.int("columns")));
        cols_spin.connect_value_changed({
            let settings = settings.clone();
            move |spin| {
                let v = spin.value() as i32;
                if settings.int("columns") != v {
                    let _ = settings.set_int("columns", v);
                }
            }
        });
        settings.connect_changed(Some("columns"), {
            let spin = cols_spin.clone();
            move |settings, _| {
                let v = settings.int("columns");
                if spin.value() as i32 != v {
                    spin.set_value(f64::from(v));
                }
            }
        });
        cols_row.add_suffix(&cols_spin);
        group.add(&cols_row);
        page.append(&group);

        // ----------------------------------------------------------- System --
        section(&page, "System");

        let group = adw::PreferencesGroup::new();
        let bg_row = adw::SwitchRow::builder()
            .title("Run in background")
            .subtitle("Keep the app running after closing for faster startup")
            .build();
        bg_row.set_active(settings.boolean("run-in-background"));
        bg_row.connect_active_notify({
            let settings = settings.clone();
            move |row| {
                let _ = settings.set_boolean("run-in-background", row.is_active());
            }
        });
        settings.connect_changed(Some("run-in-background"), {
            let row = bg_row.clone();
            move |settings, _| {
                let v = settings.boolean("run-in-background");
                if row.is_active() != v {
                    row.set_active(v);
                }
            }
        });
        group.add(&bg_row);

        let shortcut_row = adw::ActionRow::builder()
            .title("Keyboard shortcut")
            .subtitle("Add this command to your system's keyboard settings to open Emoxidize with a shortcut")
            .build();
        let command = shortcut_command();
        let cmd_entry = gtk::Entry::new();
        cmd_entry.set_text(&command);
        cmd_entry.set_editable(false);
        cmd_entry.set_width_chars(20);
        cmd_entry.set_valign(gtk::Align::Center);
        let copy_btn = gtk::Button::new();
        copy_btn.set_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("circular");
        copy_btn.set_tooltip_text(Some("Copy command"));
        copy_btn.set_valign(gtk::Align::Center);
        {
            let command = command.clone();
            copy_btn.connect_clicked(move |_| {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&command);
                }
            });
        }
        let cmd_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        cmd_box.append(&cmd_entry);
        cmd_box.append(&copy_btn);
        shortcut_row.add_suffix(&cmd_box);
        group.add(&shortcut_row);
        page.append(&group);

        // ----------------------------------------------------------- footer --
        let footer =
        gtk::Label::new(Some(concat!("Emoxidize v", env!("CARGO_PKG_VERSION"), " • Rust")));
        footer.add_css_class("dim-label");
        footer.set_halign(gtk::Align::Center);
        footer.set_margin_top(6);
        page.append(&footer);

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scrolled.set_child(Some(&page));
        scrolled.set_vexpand(true);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let header = gtk::HeaderBar::new();
        root.append(&header);
        root.append(&scrolled);
        win.set_content(Some(&root));

        win.present();
        *slot.borrow_mut() = Some(win);
    });
}

fn section(page: &gtk::Box, title: &str) {
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.add_css_class("title-4");
    page.append(&label);
}

/// Resolved launcher command: APPIMAGE/ARGV0 environment override, else the
/// conventional installed path.
fn shortcut_command() -> String {
    std::env::var("APPIMAGE")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("ARGV0").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "/usr/bin/emoxidize".to_string())
}
