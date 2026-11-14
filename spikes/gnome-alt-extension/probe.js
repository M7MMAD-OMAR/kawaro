// SPDX-License-Identifier: GPL-3.0-or-later
// Phase A of the spike, installed with Shell.Eval only (--unsafe-mode on the
// private bus): the same three capture mechanisms as the extension, without
// any extension, to establish what a real extension could see. No command
// is run here; events are appended to $XDG_RUNTIME_DIR/probe-keys.log with
// UTC and monotonic stamps. `global._sbh.reset()` empties the log,
// `global._sbh.remove()` disconnects everything again. Legacy GJS (imports),
// because Shell.Eval evaluates in the shell's global context.
(() => {
  const GLib = imports.gi.GLib;
  const Clutter = imports.gi.Clutter;
  const Meta = imports.gi.Meta;
  const Shell = imports.gi.Shell;
  const LOG = GLib.build_filenamev([GLib.get_user_runtime_dir(), 'probe-keys.log']);
  const WATCH = new Map([[Clutter.KEY_Alt_L, 'Alt_L'], [Clutter.KEY_Alt_R, 'Alt_R'], [Clutter.KEY_F9, 'F9']]);
  const s = {
    buf: '',
    conns: [],
    grabs: {},
    log(x) {
      s.buf += GLib.DateTime.new_now_utc().format('%H:%M:%S.%f') + ' ' +
        Math.round(GLib.get_monotonic_time() / 1000) + ' ' + x + '\n';
      GLib.file_set_contents(LOG, s.buf);
    },
    reset() {
      s.buf = '';
      GLib.file_set_contents(LOG, '');
      s.log('probe reset');
    },
    remove() {
      for (const [obj, id] of s.conns) obj.disconnect(id);
      s.conns = [];
      for (const tag in s.grabs) {
        if (s.grabs[tag] > 0) {
          try { global.display.ungrab_accelerator(s.grabs[tag]); } catch (e) { s.log(tag + ' ungrab error ' + e); }
        }
      }
      s.log('probe removed');
    },
  };
  global._sbh = s;

  const onKey = (tag, event, forced) => {
    const t = forced ?? event.type();
    const sym = event.get_key_symbol ? event.get_key_symbol() : -1;
    const name = WATCH.get(sym);
    s.log(tag + '-event type=' + t + ' sym=' + sym + ' ' + (name || ''));
    if (t !== Clutter.EventType.KEY_PRESS && t !== Clutter.EventType.KEY_RELEASE)
      return Clutter.EVENT_PROPAGATE;
    s.log(tag + '-' + (t === Clutter.EventType.KEY_PRESS ? 'press' : 'release') + ' ' + (name || ('sym-' + sym)));
    return Clutter.EVENT_PROPAGATE;
  };
  s.conns.push([global.stage, global.stage.connect('captured-event', (a, e) => onKey('stage-cap', e))]);
  s.conns.push([global.stage, global.stage.connect('key-press-event', (a, e) => onKey('stage-key-press', e, Clutter.EventType.KEY_PRESS))]);
  s.conns.push([global.stage, global.stage.connect('key-release-event', (a, e) => onKey('stage-key-release', e, Clutter.EventType.KEY_RELEASE))]);

  for (const [accel, tag] of [['<Alt>', 'grab-alt'], ['<F9>', 'grab-f9'], ['<Super>z', 'grab-superz']]) {
    try {
      const a = global.display.grab_accelerator(accel, Meta.KeyBindingFlags.NONE);
      s.grabs[tag] = a;
      if (a) {
        Main.wm.allowKeybinding(Meta.external_binding_name_for_action(a), Shell.ActionMode.ALL);
        s.log(tag + ' grabbed action ' + a);
      } else {
        s.log(tag + ' returned 0');
      }
    } catch (e) {
      s.grabs[tag] = -1;
      s.log(tag + ' error ' + e);
    }
  }
  s.conns.push([global.display, global.display.connect('accelerator-activated', (d, action, dev) => s.log('accel-activated action ' + action + ' device ' + dev))]);
  try {
    s.conns.push([global.display, global.display.connect('accelerator-deactivated', (d, action) => s.log('accel-deactivated action ' + action))]);
  } catch (e) {
    s.log('accel-deactivated signal missing: ' + e);
  }
  s.log('probe installed');
  return true;
})()
