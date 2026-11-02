//! Emoxidize — single-instance GTK4/libadwaita emoji picker.
//!
//! Clean-room implementation based on black-box observation; see SPEC.md.

mod emoji_data;
mod extension;
mod prefs;
mod store;
mod window;

use gtk::{glib, prelude::*};
use libadwaita as adw;

pub const APP_ID: &str = "io.github.hakergeniusz.Emoxidize";

fn print_usage() {
    print!(
        r#"Usage:
  emoxidize [OPTION…]

Help Options:
  -h, --help                 Show help options
  --help-all                 Show all help options
  --help-gapplication        Show GApplication options

Application Options:
  --gapplication-service     Enter service mode (idle on the session bus; later
                             launches activate the picker instead of starting a
                             new process)
"#
    );
}

fn main() -> glib::ExitCode {
    // GTK is initialised before argument handling: without a display every
    // invocation (including --help) fails here with status 1.
    if gtk::init().is_err() {
        eprintln!("Gtk-WARNING **: Failed to open display");
        std::process::exit(1);
    }
    if adw::init().is_err() {
        eprintln!("Failed to initialize libadwaita");
        std::process::exit(1);
    }

    // Pre-parse the command line: help exits, unknown options are reported and
    // ignored (the picker still shows), positional arguments are ignored.
    let argv: Vec<String> = std::env::args().collect();
    let mut pass = vec![argv
        .first()
        .cloned()
        .unwrap_or_else(|| "emoxidize".to_string())];
    for arg in argv.iter().skip(1) {
        match arg.as_str() {
            "-h" | "--help" | "-?" | "--help-all" | "--help-gapplication" => {
                print_usage();
                return glib::ExitCode::SUCCESS;
            }
            "--gapplication-service" => pass.push(arg.clone()),
            other if other.starts_with('-') && other != "-" => {
                println!("Unknown option {other}");
            }
            _ => {}
        }
    }

    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_startup(|app| {
        window::Picker::get_or_create(app);
        // Hold the application so it survives with a hidden window
        // (run-in-background) and stays idle in --gapplication-service mode.
        // The guard is intentionally leaked: the residency is permanent and
        // ends only via app.quit() (glib::ExitCode from run_with_args).
        std::mem::forget(app.hold());
    });
    app.connect_activate(|app| window::Picker::present(app));

    // We pass our own filtered argv so GApplication never sees the unknown
    // options we already reported.
    app.upcast_ref::<gtk::gio::Application>().run_with_args(&pass)
}
