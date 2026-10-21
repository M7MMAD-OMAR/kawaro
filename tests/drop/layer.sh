# shellcheck shell=bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# S-018d (AC13), revised by S-020 (AC1, AC2): drop mode on the layer path, from the daemon's
# WAYLAND_DEBUG=1 trace. Sourced by tests/drop/kwin.sh and
# tests/overlay/kwin.sh, which define check() and DLOG.
#
# layer_drop_show N FROM TO ENTER [AT]
#   checks one drop-mode show, trace lines FROM+1 .. TO (absolute):
#   - exactly one get_layer_surface; every set_keyboard_interactivity is 0;
#   - S-019d (S-019 AC8, decision 2, superseding the S-020 AC1 order): the
#     catcher (the wl_surface given to get_layer_surface) takes input with
#     its default region until the first position, the first
#     wl_pointer.enter or wl_data_device.enter naming it: no
#     set_input_region(wl_region...) on it before that enter. After it, the
#     last set_input_region on the catcher in the show is a region with
#     exactly one add, equal to X Y W H of the show's last
#     "hive surface: layer X Y W H" line (S-020 AC1: the hive rectangle).
#     With AT=1 (`sbarhive drop --at X,Y`, decision 1) the position is
#     known from the start: the last set_input_region on the catcher before
#     its first commit is already that rectangle;
#   - S-019 AC9: from the start of the show to the switch to the hive
#     rectangle, no wl_data_offer.accept with a type, no
#     wl_data_offer.receive and no "drop cells:" line (the anchor phase
#     never reaches the drop state machine); with ENTER=1, exactly one
#     "drop cells:" line in the show (one per enter, S-014 AC8);
#   - S-020 AC1: the hive subsurface gets an empty region
#     (set_input_region(wl_region#R), never nil, no add);
#   - exactly one get_data_device, after get_layer_surface;
#   - every wl_data_device created earlier in the trace is released before
#     it (the previous show's device first of all);
#   - S-020 AC2, with ENTER=1 (a real drag): the first enter on this show's
#     data device names the catcher, at a point inside [X, X+W] x [Y, Y+H]
#     of the show's last "hive surface" line.
#     (That (X, Y) is taken off before dnd::pick is not in the trace; the
#     real drop on the WEBP cell in tests/drop/kwin.sh guards it.)
# Requests print as "-> interface#id.request(...)" (wayland-rs) or with "@"
# (libwayland); events print without the arrow.

layer_drop_show() {
  local n=$1 from=$2 to=$3 enter=$4 at=${5:-0}
  local seg; seg=$(sed -n "$((from + 1)),${to}p" "$DLOG")
  local gls gl cat child
  gls=$(grep -nE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' <<<"$seg")
  gl=$(head -1 <<<"$gls" | cut -d: -f1)
  cat=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
  child=$(grep -oE -- '-> wl_subcompositor[#@][0-9]+\.get_subsurface\(new id wl_subsurface[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')

  local kb0 kbx
  kb0=$(grep -cE -- '-> zwlr_layer_surface_v1[#@][0-9]+\.set_keyboard_interactivity\(0\)' <<<"$seg")
  kbx=$(grep -cE -- '-> zwlr_layer_surface_v1[#@][0-9]+\.set_keyboard_interactivity\([^0]' <<<"$seg")

  # The hive rectangle of this show, in catcher coordinates: the last line,
  # after a drop-mode move to the first position (S-019 AC8).
  local hx="" hy="" hw="" hh=""
  read -r _ _ _ hx hy hw hh < <(grep -E '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$' <<<"$seg" | tail -1) || true

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

  # a) S-019 AC8: the first position, the first pointer or drag enter
  # naming the catcher; before it no region on the catcher.
  local fp="" early=0 cc1="" last="" last_reg=none sw=""
  if [ -n "$cat" ]; then
    fp=$(grep -nE -- "^\[[0-9:.]+\] (wl_pointer|wl_data_device)[#@][0-9]+\.enter\([0-9]+, ${cat}, " <<<"$seg" | head -1 | cut -d: -f1)
    [ -n "$fp" ] && early=$(head -n "$fp" <<<"$seg" | grep -cE -- "-> ${cat}\.set_input_region\(wl_region")
    cc1=$(grep -nE -- "-> ${cat}\.commit\(" <<<"$seg" | head -1 | cut -d: -f1)
  fi
  # b) the last set_input_region on the catcher in the show (with AT=1, the
  # last one before its first commit) is the hive rectangle.
  if [ -n "$cat" ]; then
    if [ "$at" = 1 ]; then
      [ -n "$cc1" ] && last=$(head -n "$cc1" <<<"$seg" | grep -nE -- "-> ${cat}\.set_input_region\(" | tail -1 | cut -d: -f1)
    else
      last=$(grep -nE -- "-> ${cat}\.set_input_region\(" <<<"$seg" | tail -1 | cut -d: -f1)
      [ -n "$fp" ] && sw=$(tail -n +"$fp" <<<"$seg" | grep -nE -- "-> ${cat}\.set_input_region\(wl_region" | head -1 | cut -d: -f1)
      [ -n "$sw" ] && sw=$((fp + sw - 1))
    fi
    if [ -n "$last" ]; then
      if sed -n "${last}p" <<<"$seg" | grep -qE 'set_input_region\(wl_region'; then last_reg=$(region_of "$last"); else last_reg=nil; fi
    fi
  fi
  [ "$at" = 1 ] && sw=$last
  # S-019 AC9: nothing of the drop machinery before the switch.
  local pre_acc=0 pre_rcv=0 pre_cells=0 ncells
  if [ -n "$sw" ]; then
    pre_acc=$(head -n "$sw" <<<"$seg" | grep -cE -- '-> wl_data_offer[#@][0-9]+\.accept\([0-9]+, "')
    pre_rcv=$(head -n "$sw" <<<"$seg" | grep -cE -- '-> wl_data_offer[#@][0-9]+\.receive\(')
    pre_cells=$(head -n "$sw" <<<"$seg" | grep -cE '^drop cells:')
  fi
  ncells=$(grep -cE '^drop cells:' <<<"$seg")
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

  echo "INFO S-018d show $n: catcher ${cat:-none}, hive subsurface ${child:-none}, keyboard_interactivity 0 x$kb0 other x$kbx, hive rect ${hx:-?} ${hy:-?} ${hw:-?} ${hh:-?}, first position (enter on the catcher) at segment line ${fp:-none}, regions on the catcher before it $early, switch at ${sw:-none}, last catcher region (adds, last add) ${last_reg}, before the switch: accept with a type $pre_acc receive $pre_rcv drop cells $pre_cells, drop cells lines $ncells, subsurface set_input_region x$cn (not empty x$cbad), get_data_device x$ngd ${dev:-none} at line ${abs:-none}, unreleased before it: ${unreleased:-none}"

  check "AC13 show $n: exactly one get_layer_surface (drop mode is a layer)" '[ "$(grep -c . <<<"$gls")" -eq 1 ] && [ -n "$gl" ]'
  check "AC13 show $n: the catcher takes no keyboard (every set_keyboard_interactivity is 0)" '[ "$kb0" -ge 1 ] && [ "$kbx" -eq 0 ]'
  if [ "$at" = 1 ]; then
    check "S-019 AC8 show $n (--at): before its first commit the catcher's input region is already the hive rectangle (one add)" \
      '[ -n "$hh" ] && [ "$last_reg" = "1 $hx $hy $hw $hh" ]'
  else
    check "S-019 AC8 show $n: the catcher keeps its default input region until the first position (an enter on it, and no set_input_region(wl_region) before it)" \
      '[ -n "$fp" ] && [ "$early" -eq 0 ]'
    check "S-019 AC8 show $n: after the first position the catcher's last input region is one add equal to the hive rectangle (S-020 AC1)" \
      '[ -n "$hh" ] && [ -n "$sw" ] && [ -n "$last" ] && [ "$last" -ge "$sw" ] && [ "$last_reg" = "1 $hx $hy $hw $hh" ]'
  fi
  check "S-019 AC9 show $n: before the switch to the hive rectangle no wl_data_offer.accept with a type, no receive, no \"drop cells\" line" \
    '[ -n "$sw" ] && [ "$pre_acc" -eq 0 ] && [ "$pre_rcv" -eq 0 ] && [ "$pre_cells" -eq 0 ]'
  check "S-020 AC1 show $n: the hive subsurface has an empty input region (wl_region, no add)" \
    '[ "$cn" -ge 1 ] && [ "$cbad" -eq 0 ]'
  check "AC13 show $n: exactly one get_data_device, after get_layer_surface" \
    '[ "$ngd" -eq 1 ] && [ -n "$gd" ] && [ -n "$gl" ] && [ "$gd" -gt "$gl" ]'
  check "AC13 show $n: every earlier wl_data_device is released before it" '[ -n "$abs" ] && [ -z "$unreleased" ]'
  if [ "$enter" = 1 ]; then
    check "S-019 AC9 show $n: exactly one \"drop cells\" line (one per enter into the hive)" '[ "$ncells" -eq 1 ]'
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
