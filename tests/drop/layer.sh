# shellcheck shell=bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# S-018d (AC13, AC14): drop mode on the layer path, from the daemon's
# WAYLAND_DEBUG=1 trace. Sourced by tests/drop/kwin.sh and
# tests/overlay/kwin.sh, which define check() and DLOG.
#
# layer_drop_show N FROM TO ENTER
#   checks one drop-mode show, trace lines FROM+1 .. TO (absolute):
#   - exactly one get_layer_surface; every set_keyboard_interactivity is 0;
#   - the catcher (the wl_surface given to get_layer_surface) gets an empty
#     input region: set_input_region(wl_region#R) with R made by
#     create_region and no wl_region#R.add before it;
#   - exactly one get_data_device, after get_layer_surface;
#   - every wl_data_device created earlier in the trace is released before
#     it (the previous show's device first of all);
#   - with ENTER=1 (a real drag), the enter on this show's data device names
#     this show's hive subsurface (the first wl_surface of get_subsurface):
#     only a data device on the layer connection is told about that surface.
# Requests print as "-> interface#id.request(...)" (wayland-rs) or with "@"
# (libwayland); events print without the arrow.

layer_drop_show() {
  local n=$1 from=$2 to=$3 enter=$4
  local seg; seg=$(sed -n "$((from + 1)),${to}p" "$DLOG")
  local gls gl cat child
  gls=$(grep -nE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' <<<"$seg")
  gl=$(head -1 <<<"$gls" | cut -d: -f1)
  cat=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
  child=$(grep -oE -- '-> wl_subcompositor[#@][0-9]+\.get_subsurface\(new id wl_subsurface[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')

  local kb0 kbx
  kb0=$(grep -cE -- '-> zwlr_layer_surface_v1[#@][0-9]+\.set_keyboard_interactivity\(0\)' <<<"$seg")
  kbx=$(grep -cE -- '-> zwlr_layer_surface_v1[#@][0-9]+\.set_keyboard_interactivity\([^0]' <<<"$seg")

  local sir="" reg="" rc="" adds=none
  if [ -n "$cat" ]; then
    sir=$(grep -nE -- "-> ${cat}\.set_input_region\(wl_region[#@][0-9]+\)" <<<"$seg" | tail -1)
    reg=$(grep -oE 'wl_region[#@][0-9]+' <<<"${sir#*:}" | head -1)
  fi
  if [ -n "$reg" ]; then
    rc=$(head -n "${sir%%:*}" <<<"$seg" | grep -nE -- "-> wl_compositor[#@][0-9]+\.create_region\(new id ${reg}\)" | tail -1 | cut -d: -f1)
    [ -n "$rc" ] && adds=$(sed -n "${rc},${sir%%:*}p" <<<"$seg" | grep -cE -- "-> ${reg}\.add\(")
  fi

  local gds gd="" dev="" ngd
  gds=$(grep -nE -- '-> wl_data_device_manager[#@][0-9]+\.get_data_device\(new id wl_data_device[#@][0-9]+' <<<"$seg")
  ngd=$(grep -c . <<<"$gds")
  if [ -n "$gds" ]; then
    gd=$(head -1 <<<"$gds" | cut -d: -f1)
    dev=$(head -1 <<<"$gds" | grep -oE 'new id wl_data_device[#@][0-9]+' | grep -oE 'wl_data_device[#@][0-9]+')
  fi

  # Earlier devices (absolute lines) without a release before this one.
  local unreleased="" abs="" line l id
  if [ -n "$gd" ]; then
    abs=$((from + gd))
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      l=${line%%:*}
      id=$(grep -oE 'new id wl_data_device[#@][0-9]+' <<<"$line" | grep -oE 'wl_data_device[#@][0-9]+')
      sed -n "$((l + 1)),$((abs - 1))p" "$DLOG" | grep -qE -- "-> ${id}\.release\(" || unreleased+="$id(line $l) "
    done < <(head -n "$((abs - 1))" "$DLOG" | grep -nE -- '-> wl_data_device_manager[#@][0-9]+\.get_data_device\(new id wl_data_device[#@][0-9]+')
  fi

  echo "INFO S-018d show $n: catcher ${cat:-none}, hive subsurface ${child:-none}, keyboard_interactivity 0 x$kb0 other x$kbx, input region ${reg:-none} (adds $adds), get_data_device x$ngd ${dev:-none} at line ${abs:-none}, unreleased before it: ${unreleased:-none}"

  check "AC13 show $n: exactly one get_layer_surface (drop mode is a layer)" '[ "$(grep -c . <<<"$gls")" -eq 1 ] && [ -n "$gl" ]'
  check "AC13 show $n: the catcher takes no keyboard (every set_keyboard_interactivity is 0)" '[ "$kb0" -ge 1 ] && [ "$kbx" -eq 0 ]'
  check "AC13 show $n: the catcher has an empty input region (create_region, no add, set_input_region)" \
    '[ -n "$reg" ] && [ -n "$rc" ] && [ "$adds" = 0 ]'
  check "AC13 show $n: exactly one get_data_device, after get_layer_surface" \
    '[ "$ngd" -eq 1 ] && [ -n "$gd" ] && [ -n "$gl" ] && [ "$gd" -gt "$gl" ]'
  check "AC13 show $n: every earlier wl_data_device is released before it" '[ -n "$abs" ] && [ -z "$unreleased" ]'
  if [ "$enter" = 1 ]; then
    check "AC14 show $n: the drag enters this show's data device on the hive subsurface (layer connection)" \
      '[ -n "$dev" ] && [ -n "$child" ] && grep -qE "^\[[0-9:.]+\] ${dev}\.enter\([0-9]+, ${child}," <<<"$seg"'
  fi
}
