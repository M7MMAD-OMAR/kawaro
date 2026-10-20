# shellcheck shell=bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# S-018d (AC13), revised by S-020 (AC1, AC2): drop mode on the layer path, from the daemon's
# WAYLAND_DEBUG=1 trace. Sourced by tests/drop/kwin.sh and
# tests/overlay/kwin.sh, which define check() and DLOG.
#
# layer_drop_show N FROM TO ENTER
#   checks one drop-mode show, trace lines FROM+1 .. TO (absolute):
#   - exactly one get_layer_surface; every set_keyboard_interactivity is 0;
#   - S-020 AC1: the catcher (the wl_surface given to get_layer_surface)
#     gets an empty input region before its first commit (the last
#     set_input_region(wl_region#R) before that commit, R made by
#     create_region with no wl_region#R.add between); after the first
#     configure of this show's layer surface and before the catcher's first
#     commit after it (the commit that presents the synced hive subsurface,
#     its first frame), the last set_input_region on the catcher is a region
#     with exactly one add, equal to X Y W H of this show's
#     "hive surface: layer X Y W H" line; and the hive subsurface gets an
#     empty region (set_input_region(wl_region#R), never nil, no add);
#   - exactly one get_data_device, after get_layer_surface;
#   - every wl_data_device created earlier in the trace is released before
#     it (the previous show's device first of all);
#   - S-020 AC2, with ENTER=1 (a real drag): the first enter on this show's
#     data device names the catcher, at a point inside [X, X+W] x [Y, Y+H].
#     (That (X, Y) is taken off before dnd::pick is not in the trace; the
#     real drop on the WEBP cell in tests/drop/kwin.sh guards it.)
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

  # The hive rectangle of this show, in catcher coordinates.
  local hx="" hy="" hw="" hh=""
  read -r _ _ _ hx hy hw hh < <(grep -E '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$' <<<"$seg" | head -1) || true

  # region_of SEGLINE: for the set_input_region at line SEGLINE of $seg,
  # prints "ADDS ARGS": the number of wl_region#R.add between R's
  # create_region and it, and the arguments of the last such add ("-" if
  # none). Prints "none" if R has no create_region before it.
  region_of() {
    local l=$1 r c
    r=$(sed -n "${l}p" <<<"$seg" | grep -oE 'wl_region[#@][0-9]+' | head -1)
    c=$(head -n "$l" <<<"$seg" | grep -nE -- "-> wl_compositor[#@][0-9]+\.create_region\(new id ${r}\)" | tail -1 | cut -d: -f1)
    [ -n "$r" ] && [ -n "$c" ] || { echo none; return; }
    local a; a=$(sed -n "${c},${l}p" <<<"$seg" | grep -E -- "-> ${r}\.add\(")
    echo "$(grep -c . <<<"$a") $(tail -1 <<<"$a" | sed -nE 's/.*\.add\(([^)]*)\).*/\1/p' | tr -d ',' | grep . || echo -)"
  }

  # a) before the catcher's first commit: an empty region.
  local cc1="" pre="" pre_reg=none
  if [ -n "$cat" ]; then
    cc1=$(grep -nE -- "-> ${cat}\.commit\(" <<<"$seg" | head -1 | cut -d: -f1)
    [ -n "$cc1" ] && pre=$(head -n "$cc1" <<<"$seg" | grep -nE -- "-> ${cat}\.set_input_region\(wl_region[#@][0-9]+\)" | tail -1 | cut -d: -f1)
    [ -n "$pre" ] && pre_reg=$(region_of "$pre")
  fi
  # b) after the first configure, before the catcher's first commit after
  # it: the last set_input_region on the catcher is the hive rectangle.
  local ls_id cfg="" cc2="" post="" post_reg=none
  ls_id=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'zwlr_layer_surface_v1[#@][0-9]+$')
  if [ -n "$ls_id" ] && [ -n "$cat" ]; then
    cfg=$(grep -nE -- "^\[[0-9:.]+\] ${ls_id}\.configure\(" <<<"$seg" | head -1 | cut -d: -f1)
  fi
  if [ -n "$cfg" ]; then
    cc2=$(tail -n +"$cfg" <<<"$seg" | grep -nE -- "-> ${cat}\.commit\(" | head -1 | cut -d: -f1)
    [ -n "$cc2" ] && cc2=$((cfg + cc2 - 1))
  fi
  if [ -n "$cc2" ]; then
    post=$(sed -n "${cfg},${cc2}p" <<<"$seg" | grep -nE -- "-> ${cat}\.set_input_region\(" | tail -1 | cut -d: -f1)
    if [ -n "$post" ]; then
      post=$((cfg + post - 1))
      if sed -n "${post}p" <<<"$seg" | grep -qE 'set_input_region\(wl_region'; then post_reg=$(region_of "$post"); else post_reg=nil; fi
    fi
  fi
  # c) the hive subsurface: every set_input_region an empty region.
  local csets="" cbad=0 cn=0 cl
  if [ -n "$child" ]; then
    csets=$(grep -nE -- "-> ${child}\.set_input_region\(" <<<"$seg" | cut -d: -f1)
    for cl in $csets; do
      cn=$((cn + 1))
      if sed -n "${cl}p" <<<"$seg" | grep -qE 'set_input_region\(wl_region'; then
        [ "$(region_of "$cl")" = "0 -" ] || cbad=$((cbad + 1))
      else
        cbad=$((cbad + 1))
      fi
    done
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

  echo "INFO S-018d show $n: catcher ${cat:-none}, hive subsurface ${child:-none}, keyboard_interactivity 0 x$kb0 other x$kbx, hive rect ${hx:-?} ${hy:-?} ${hw:-?} ${hh:-?}, catcher region before first commit (adds, last add) ${pre_reg}, after configure ${post_reg}, subsurface set_input_region x$cn (not empty x$cbad), get_data_device x$ngd ${dev:-none} at line ${abs:-none}, unreleased before it: ${unreleased:-none}"

  check "AC13 show $n: exactly one get_layer_surface (drop mode is a layer)" '[ "$(grep -c . <<<"$gls")" -eq 1 ] && [ -n "$gl" ]'
  check "AC13 show $n: the catcher takes no keyboard (every set_keyboard_interactivity is 0)" '[ "$kb0" -ge 1 ] && [ "$kbx" -eq 0 ]'
  check "S-020 AC1 show $n: before its first commit the catcher has an empty input region (create_region, no add, set_input_region)" \
    '[ "$pre_reg" = "0 -" ]'
  check "S-020 AC1 show $n: after the first configure and before the first hive frame the catcher's last input region is one add equal to the hive rectangle" \
    '[ -n "$hh" ] && [ "$post_reg" = "1 $hx $hy $hw $hh" ]'
  check "S-020 AC1 show $n: the hive subsurface has an empty input region (wl_region, no add)" \
    '[ "$cn" -ge 1 ] && [ "$cbad" -eq 0 ]'
  check "AC13 show $n: exactly one get_data_device, after get_layer_surface" \
    '[ "$ngd" -eq 1 ] && [ -n "$gd" ] && [ -n "$gl" ] && [ "$gd" -gt "$gl" ]'
  check "AC13 show $n: every earlier wl_data_device is released before it" '[ -n "$abs" ] && [ -z "$unreleased" ]'
  if [ "$enter" = 1 ]; then
    local ent="" ex="" ey="" esurf=""
    [ -n "$dev" ] && ent=$(grep -E -- "^\[[0-9:.]+\] ${dev}\.enter\(" <<<"$seg" | head -1)
    if [ -n "$ent" ]; then
      esurf=$(grep -oE 'enter\([0-9]+, wl_surface[#@][0-9]+' <<<"$ent" | grep -oE 'wl_surface[#@][0-9]+$')
      read -r ex ey < <(sed -nE 's/.*enter\([0-9]+, wl_surface[#@][0-9]+, (-?[0-9.]+), (-?[0-9.]+),.*/\1 \2/p' <<<"$ent") || true
    fi
    echo "INFO S-020 show $n: first enter on ${dev:-none}: surface ${esurf:-none} at ${ex:-?} ${ey:-?} (catcher ${cat:-none}, hive ${hx:-?} ${hy:-?} ${hw:-?} ${hh:-?})"
    check "S-020 AC2 show $n: the first enter on this show's data device names the catcher" \
      '[ -n "$cat" ] && [ "$esurf" = "$cat" ]'
    check "S-020 AC2 show $n: the first enter is inside the hive rectangle [X, X+W] x [Y, Y+H]" \
      '[ -n "$ex" ] && [ -n "$hh" ] && awk -v x="$ex" -v y="$ey" -v X="$hx" -v Y="$hy" -v W="$hw" -v H="$hh" "BEGIN { exit !(x >= X && x <= X + W && y >= Y && y <= Y + H) }"'
  fi
}
