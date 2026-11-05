# S-139 native keyboard fixtures

Generated locally on 2026-11-05 by [generate.py](generate.py), using Python ctypes and the actual system libxkbcommon API, not a handwritten mapping. Rules evdev, model pc105, empty variants/options. us.xkb uses us; us-fr.xkb uses us,fr with French in group 1. Files include the terminating NUL required by the Wayland XkbV1 wire format. No compositor or production dependency change was used.

Local packages: libxkbcommon 1.13.1-2.fc44, xkeyboard-config 2.47-1.fc44. Generated keymap data derives from xkeyboard-config; its upstream copyright and permissive notices are retained in [COPYING.xkeyboard-config](COPYING.xkeyboard-config). Fedora metadata identifies HPND, HPND-sell-variant, X11, X11-distribute-modifications-variant, MIT, MIT-open-group and xkeyboard-config-Zinoviev. Generator library metadata: MIT, X11, MIT-CMU. This is data generation, not linking a new production dependency.

SHA256:

- us.xkb: 88ab88dd3fac626117767aa1f93b9745bb2c6c5edee35eb96a4036233eb82a51
- us-fr.xkb: 3f469fcfbea5781f51249407b4f974eab1459e4c51ea481520a8f4f9acbb0c66

The tests use evdev physical codes, not XKB codes (the production translator adds 8). Shift=1, Lock=2, Control=4, Mod5=128 are indices from these generated maps, not assumptions for arbitrary compositor maps. Native end-to-end typing remains a separate Lead gate.
