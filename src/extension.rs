//! Client for the Emoxidize GNOME Shell paste bridge.
//!
//! The extension owns the well-known bus name while it is enabled; we watch
//! it and expose presence to the UI plus a `Paste` call for the selection
//! pipeline. Without the bridge the picker falls back to writing the
//! clipboard directly.

use gtk::gio;
use gtk::glib;
use gtk::gio::prelude::*;
use gtk::glib::prelude::ToVariant;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

pub const BUS_NAME: &str = "io.github.hakergeniusz.Emoxidize.Extension";
pub const OBJECT_PATH: &str = "/io/github/hakergeniusz/Emoxidize";

thread_local! {
    static STATE: RefCell<Option<Rc<ExtensionState>>> = const { RefCell::new(None) };
}

pub struct ExtensionState {
    present: Cell<bool>,
    proxy: RefCell<Option<gio::DBusProxy>>,
    listeners: RefCell<Vec<Box<dyn Fn(bool)>>>,
}

impl ExtensionState {
    /// Shared state; starts watching the bridge name on first use.
    pub fn get() -> Rc<ExtensionState> {
        STATE.with(|slot| {
            if let Some(state) = slot.borrow().as_ref() {
                return state.clone();
            }
            let state = Rc::new(ExtensionState {
                present: Cell::new(false),
                proxy: RefCell::new(None),
                listeners: RefCell::new(Vec::new()),
            });

            let watcher = state.clone();
            let watcher2 = state.clone();
            gio::bus_watch_name(
                gio::BusType::Session,
                BUS_NAME,
                gio::BusNameWatcherFlags::NONE,
                move |_conn, _name, _owner| watcher.set_present(true),
                move |_conn, _name| watcher2.set_present(false),
            );
            // The watcher id lives for the whole process; no cancel needed.

            slot.borrow_mut().replace(state.clone());
            state
        })
    }

    pub fn is_present(&self) -> bool {
        self.present.get()
    }

    /// Register a listener notified whenever bridge presence changes. The
    /// listener must be valid for the program's lifetime (used for widgets
    /// that are never destroyed).
    pub fn connect_changed(&self, listener: impl Fn(bool) + 'static) {
        listener(self.present.get());
        self.listeners.borrow_mut().push(Box::new(listener));
    }

    fn set_present(self: &Rc<Self>, present: bool) {
        if self.present.replace(present) != present {
            self.proxy.borrow_mut().take();
            if present {
                self.create_proxy();
            }
            for listener in self.listeners.borrow().iter() {
                listener(present);
            }
        }
    }

    fn create_proxy(self: &Rc<Self>) {
        let state = self.clone();
        glib::spawn_future_local(async move {
            let result = async {
                let conn = gio::bus_get_future(gio::BusType::Session).await?;
                gio::DBusProxy::new_future(
                    &conn,
                    gio::DBusProxyFlags::DO_NOT_AUTO_START,
                    None,
                    Some(BUS_NAME),
                    OBJECT_PATH,
                    BUS_NAME,
                )
                .await
            }
            .await;
            match result {
                Ok(proxy) => {
                    *state.proxy.borrow_mut() = Some(proxy);
                }
                Err(err) => {
                    eprintln!("Emoxidize: bridge proxy error: {err}");
                    state.set_present(false);
                }
            }
        });
    }

    /// Paste into the focused window via the bridge. Blocking on purpose:
    /// every caller dismisses (and often quits) immediately afterwards, so a
    /// fire-and-forget async call would never leave the process. Returns
    /// false when the bridge is not ready (caller should fall back to the
    /// clipboard).
    pub fn paste(&self, text: &str) -> bool {
        let Some(proxy) = self.proxy.borrow().clone() else {
            eprintln!("Emoxidize: paste requested but the bridge proxy is not ready");
            return false;
        };
        // D-Bus method parameters travel as a tuple: (s).
        let params = (text,).to_variant();
        match proxy.call_sync(
            "Paste",
            Some(&params),
            gio::DBusCallFlags::NONE,
            3000,
            None::<&gio::Cancellable>,
        ) {
            Ok(_) => true,
            Err(err) => {
                eprintln!("Emoxidize: Paste call failed: {err}");
                false
            }
        }
    }
}
