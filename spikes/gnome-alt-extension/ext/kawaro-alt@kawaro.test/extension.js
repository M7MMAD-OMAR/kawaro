// SPDX-License-Identifier: GPL-3.0-or-later
// Spike (S-075 investigation): can a GNOME Shell extension capture Alt_L
// press and release globally, including during a real GTK file drag, and
// feed the same IPC the Hyprland bind runs (`kawaro drop --hold` on the
// press, `kawaro drop --release` on the release)? Three capture mechanisms
// are logged side by side, and each forwarded press or release is stamped
// by a wrapper script, so hold_ms and release can be timed at the daemon.
//
// Capture mechanisms (every one logs with its own tag):
//   stage-cap-*    global.stage captured-event (capture phase, before
//                  delivery to the focused client)
//   stage-key-*    global.stage key-press-event / key-release-event
//   accel-*        global.display.grab_accelerator + accelerator-activated
//                  / accelerator-deactivated (the path the GlobalShortcuts
//                  portal backend takes)
// Watched keys: Alt_L and Alt_R (the test) and F9 (a conventional key,
// the positive control). <Alt> and <F9> and <Super>z are also grabbed as
// accelerators, to see which of the grabs this shell realizes.
//
// Forwarding: at most one press in flight. The first captured press of a
// watched key runs the configured hold command, the next captured release
// runs the release command. Repeat presses while held are logged and
// ignored. Everything is logged to the configured log file with UTC and
// monotonic stamps; the log is rewritten whole on every line (few hundred
// lines, keeps the code trivial).
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const WATCH = new Map([
    [Clutter.KEY_Alt_L, 'Alt_L'],
    [Clutter.KEY_Alt_R, 'Alt_R'],
    [Clutter.KEY_F9, 'F9'],
]);
const GRABS = [
    ['<Alt>', 'grab-alt'],
    ['<F9>', 'grab-f9'],
    ['<Super>z', 'grab-superz'],
];

export default class KawaroAltExtension extends Extension {
    enable() {
        this._buf = '';
        this._conf = {};
        this._pressed = null;
        this._conns = [];
        this._actions = [];
        this._readConf();
        this._log('enabled');

        this._conns.push([global.stage, global.stage.connect('captured-event',
            (_a, e) => this._onKey('stage-cap', e))]);
        this._conns.push([global.stage, global.stage.connect('key-press-event',
            (_a, e) => this._onKey('stage-key-press', e, Clutter.EventType.KEY_PRESS))]);
        this._conns.push([global.stage, global.stage.connect('key-release-event',
            (_a, e) => this._onKey('stage-key-release', e, Clutter.EventType.KEY_RELEASE))]);

        for (const [accel, tag] of GRABS) {
            try {
                const action = global.display.grab_accelerator(accel, Meta.KeyBindingFlags.NONE);
                if (action) {
                    Main.wm.allowKeybinding(Meta.external_binding_name_for_action(action),
                        Shell.ActionMode.ALL);
                    this._actions.push(action);
                    this._log(`${tag} grabbed action ${action}`);
                } else {
                    this._log(`${tag} grab returned 0`);
                }
            } catch (e) {
                this._log(`${tag} grab error ${e}`);
            }
        }

        this._conns.push([global.display, global.display.connect('accelerator-activated',
            (_d, action, deviceId) => {
                this._log(`accel-activated action ${action} device ${deviceId}`);
                this._press(`accel:${action}`);
            })]);
        try {
            this._conns.push([global.display, global.display.connect('accelerator-deactivated',
                (_d, action) => {
                    this._log(`accel-deactivated action ${action}`);
                    this._release(`accel:${action}`);
                })]);
        } catch (e) {
            this._log(`accel-deactivated signal missing: ${e}`);
        }
    }

    disable() {
        for (const [obj, id] of this._conns)
            obj.disconnect(id);
        this._conns = [];
        for (const action of this._actions) {
            try {
                global.display.ungrab_accelerator(action);
            } catch (e) {
                this._log(`ungrab error ${e}`);
            }
        }
        this._actions = [];
        this._log('disabled');
        this._conf = {};
    }

    _onKey(tag, event, forcedType = null) {
        const type = forcedType ?? event.type();
        const sym = event.get_key_symbol ? event.get_key_symbol() : -1;
        const name = WATCH.get(sym);
        this._log(`${tag}-event type=${type} sym=${sym} ${name || ''}`);
        if (type !== Clutter.EventType.KEY_PRESS && type !== Clutter.EventType.KEY_RELEASE)
            return Clutter.EVENT_PROPAGATE;
        if (!name)
            return Clutter.EVENT_PROPAGATE;
        const edge = type === Clutter.EventType.KEY_PRESS ? 'press' : 'release';
        this._log(`${tag}-${edge} ${name}`);
        // stage-key-* repeats what stage-cap-* already forwarded; the
        // pressed guard makes sure only one command runs per physical key.
        if (edge === 'press')
            this._press(`${tag}:${name}`);
        else
            this._release(`${tag}:${name}`);
        return Clutter.EVENT_PROPAGATE;
    }

    _press(tag) {
        if (this._pressed) {
            this._log(`press-ignored ${tag} (already holding ${this._pressed})`);
            return;
        }
        this._pressed = tag;
        this._log(`forward-hold via ${tag}`);
        this._spawn(this._conf.hold_cmd);
    }

    _release(tag) {
        if (!this._pressed) {
            this._log(`release-ignored ${tag} (nothing held)`);
            return;
        }
        this._log(`forward-release via ${tag} (held as ${this._pressed})`);
        this._pressed = null;
        this._spawn(this._conf.release_cmd);
    }

    _spawn(cmd) {
        if (!cmd) {
            this._log('no command configured');
            return;
        }
        try {
            GLib.spawn_command_line_async(cmd);
        } catch (e) {
            this._log(`spawn error ${e}`);
        }
    }

    _readConf() {
        const dir = GLib.get_user_runtime_dir();
        this._logPath = GLib.build_filenamev([dir, 'kawaro-ext.log']);
        try {
            const [, bytes] = GLib.file_get_contents(GLib.build_filenamev([dir, 'kawaro-ext.conf']));
            const text = typeof bytes === 'string' ? bytes : new TextDecoder().decode(bytes);
            for (const line of text.split('\n')) {
                const i = line.indexOf('=');
                if (i > 0)
                    this._conf[line.slice(0, i).trim()] = line.slice(i + 1).trim();
            }
            if (this._conf.log)
                this._logPath = this._conf.log;
        } catch (e) {
            this._log(`conf read error ${e}`);
        }
    }

    _log(s) {
        this._buf += `${GLib.DateTime.new_now_utc().format('%H:%M:%S.%f')} ` +
            `${Math.round(GLib.get_monotonic_time() / 1000)} ${s}\n`;
        try {
            GLib.file_set_contents(this._logPath, this._buf);
        } catch (e) {
            // Nothing to do: logging must never break the shell.
        }
    }
}
