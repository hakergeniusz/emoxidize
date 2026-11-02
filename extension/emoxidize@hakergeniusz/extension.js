// Emoxidize Paste Bridge — GNOME Shell extension (ESM, GNOME 45+).
//
// Owns the well-known bus name io.github.hakergeniusz.Emoxidize.Extension and
// exposes a single method Paste(s text). The picker calls it when
// "paste-on-select" is enabled; the extension stores `text` in Mutter's global
// clipboard (so it survives the picker exiting) and injects Ctrl+V into the
// focused window via Clutter's virtual keyboard. If the picker still has
// keyboard focus (it hides itself right before calling Paste), the injection
// is deferred until focus returns to a real target window.

import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import St from 'gi://St';
import Clutter from 'gi://Clutter';

import {Extension} from 'resource:///org/gnome/Shell/Extensions/js/extensions.js';

const BUS_NAME = 'io.github.hakergeniusz.Emoxidize.Extension';
const OBJECT_PATH = '/io/github/hakergeniusz/Emoxidize';

const IFACE_XML = `
<node>
  <interface name="io.github.hakergeniusz.Emoxidize.Extension">
    <method name="Paste">
      <arg type="s" direction="in" name="text"/>
    </method>
  </interface>
</node>`;

export default class EmoxidizePasteBridge extends Extension {
    enable() {
        this._nodeInfo = Gio.DBusNodeInfo.new_for_xml(IFACE_XML);
        this._interface = this._nodeInfo.lookup_interface(BUS_NAME);
        this._retries = 0;
        this._timeoutIds = [];

        this._ownerId = Gio.DBus.session.own_name(
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            null,
            (conn, name) => {
                log(`[${this.metadata.uuid}] lost bus name ${name}`);
            }
        );

        this._regId = Gio.DBus.session.register_object(
            OBJECT_PATH,
            this._interface,
            (conn, sender, path, iface, method, args) => {
                if (method === 'Paste') {
                    this._paste(args[0].unpack());
                }
            },
            null
        );

        this._keyboard = Clutter.get_default_backend()
            .get_default_seat()
            .create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
    }

    disable() {
        for (const id of this._timeoutIds.splice(0)) {
            GLib.source_remove(id);
        }
        if (this._regId !== undefined) {
            Gio.DBus.session.unregister_object(this._regId);
            this._regId = undefined;
        }
        if (this._ownerId !== undefined) {
            Gio.DBus.session.unown_name(this._ownerId);
            this._ownerId = undefined;
        }
        this._keyboard = null;
        this._interface = null;
        this._nodeInfo = null;
    }

    _paste(text) {
        try {
            St.Clipboard.get_default().set(St.ClipboardType.CLIPBOARD, text);
            this._retries = 0;
            this._pasteWhenPickerGone();
        } catch (e) {
            logError(e, `[${this.metadata.uuid}] Paste failed`);
        }
    }

    _pasteWhenPickerGone() {
        if (!this._pickerHasFocus()) {
            this._injectCtrlV();
            return;
        }
        if (this._retries >= 5) {
            // Focus never came back; paste anyway.
            this._injectCtrlV();
            return;
        }
        this._retries += 1;
        const id = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 120, () => {
            this._pasteWhenPickerGone();
            return GLib.SOURCE_REMOVE;
        });
        this._timeoutIds.push(id);
    }

    _pickerHasFocus() {
        const win = global.display.get_focus_window();
        if (!win) {
            return false;
        }
        const wmClass = win.get_wm_class();
        return typeof wmClass === 'string' && /emoxidize/i.test(wmClass);
    }

    _injectCtrlV() {
        if (!this._keyboard) {
            return;
        }
        const time = Clutter.get_current_event_time();
        this._keyboard.notify_keyval(time, Clutter.KEY_Control_L, Clutter.KeyState.PRESSED);
        this._keyboard.notify_keyval(time, Clutter.KEY_v, Clutter.KeyState.PRESSED);
        this._keyboard.notify_keyval(time, Clutter.KEY_v, Clutter.KeyState.RELEASED);
        this._keyboard.notify_keyval(time, Clutter.KEY_Control_L, Clutter.KeyState.RELEASED);
    }
}
