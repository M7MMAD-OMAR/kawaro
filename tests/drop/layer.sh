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
#     its first commit is already that rectangle.
#     With AT=still (manager decision 12: no position, the drag or pointer
#     stands still, no --at), the full region lasts at most 300 ms from the
#     catcher's ready commit: no enter on the catcher before its first
#     set_input_region(wl_region), that region is exactly one add equal to
#     the show's last "hive surface" rectangle (the centred hive, which
#     never moves afterwards), it is also the catcher's last region, and it
#     comes after the catcher's ready commit and at most 400 ms after it,
#     by the trace's timestamps (still_region below). S-021 AC4: the
#     reference was the first frame (the hive subsurface's first
#     attach(wl_buffer)); a drop show on the layer path now draws nothing
#     until the drag enters, so it is the catcher's first commit after its
#     layer surface's first configure (catcher_ready below). With AT=0 the
#     same 400 ms bound holds for the switch: a first position later than
#     that is past the phase;
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

# region_of SEGLINE: for the set_input_region at line SEGLINE of $seg (the
# caller's local), prints "ADDS ARGS": the number of wl_region#R.add
# between R's create_region and it, and the arguments of the last such add
# ("-" if none). Prints "none" if R has no create_region before it.
region_of() {
  local l=$1 r c
  r=$(sed -n "${l}p" <<<"$seg" | grep -oE 'wl_region[#@][0-9]+' | head -1)
  c=$(head -n "$l" <<<"$seg" | grep -nE -- "-> wl_compositor[#@][0-9]+\.create_region\(new id ${r}\)" | tail -1 | cut -d: -f1)
  [ -n "$r" ] && [ -n "$c" ] || { echo none; return; }
  local a; a=$(sed -n "${c},${l}p" <<<"$seg" | grep -E -- "-> ${r}\.add\(")
  echo "$(grep -c . <<<"$a") $(tail -1 <<<"$a" | sed -nE 's/.*\.add\(([^)]*)\).*/\1/p' | tr -d ',' | grep . || echo -)"
}

# trace_ms LINE: the "[HH:MM:SS.ffffff]" timestamp of a trace line, in ms.
trace_ms() {
  sed -nE 's/^\[([0-9]+):([0-9]+):([0-9]+\.[0-9]+)\].*/\1 \2 \3/p' <<<"$1" |
    awk '{ printf "%.3f\n", ($1 * 3600 + $2 * 60 + $3) * 1000 }'
}

# catcher_ready SEG (S-021 AC4): the line of SEG, the catcher's ready
# commit: the first commit on the catcher (the wl_surface given to
# get_layer_surface) after the first configure event of its
# zwlr_layer_surface_v1. The commit before that configure carries no buffer;
# the ready commit carries the catcher's one transparent pixel. Prints
# nothing when there is no such commit.
catcher_ready() {
  local seg=$1 gl lsf cat cf
  gl=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1)
  lsf=$(grep -oE 'zwlr_layer_surface_v1[#@][0-9]+' <<<"$gl")
  cat=$(grep -oE 'wl_surface[#@][0-9]+$' <<<"$gl")
  [ -n "$lsf" ] && [ -n "$cat" ] || return 0
  cf=$(grep -nE -- "^\[[0-9:.]+\] ${lsf}\.configure\(" <<<"$seg" | head -1 | cut -d: -f1)
  [ -n "$cf" ] || return 0
  tail -n +"$cf" <<<"$seg" | grep -nE -- "-> ${cat}\.commit\(" | head -1 | cut -d: -f1 | awk -v c="$cf" 'NF { print $1 + c - 1 }'
}

# still_region SEG (manager decision 12), for one drop show's trace. Prints
#   DT|ENTERS|FIRST|LAST|RECT|SW
# DT: ms from the catcher's ready commit (catcher_ready, S-021 AC4) to the
# catcher's first set_input_region(wl_region),
# negative if the region comes first, "none" if either is missing;
# ENTERS: wl_pointer or wl_data_device enters naming the catcher before
# that region; FIRST and LAST: region_of for that first region and for the
# catcher's last set_input_region in the show ("nil" for a nil region);
# RECT: X Y W H of the show's last "hive surface: layer" line; SW: the
# segment line of that first region.
still_region() {
  local seg=$1 cat child fr="" sw="" dt=none ent=0 first=none last=none rect l
  cat=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
  child=$(grep -oE -- '-> wl_subcompositor[#@][0-9]+\.get_subsurface\(new id wl_subsurface[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
  rect=$(grep -E '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$' <<<"$seg" | tail -1 | cut -d' ' -f4-)
  if [ -n "$cat" ] && [ -n "$child" ]; then
    fr=$(catcher_ready "$seg")
    [ -n "$fr" ] && fr="$fr:$(sed -n "${fr}p" <<<"$seg")"
    sw=$(grep -nE -- "-> ${cat}\.set_input_region\(wl_region" <<<"$seg" | head -1)
    if [ -n "$fr" ] && [ -n "$sw" ]; then
      dt=$(awk -v a="$(trace_ms "${fr#*:}")" -v b="$(trace_ms "${sw#*:}")" 'BEGIN { if (a == "" || b == "") print "none"; else printf "%.1f\n", b - a }')
    fi
    if [ -n "$sw" ]; then
      ent=$(head -n "${sw%%:*}" <<<"$seg" | grep -cE -- "^\[[0-9:.]+\] (wl_pointer|wl_data_device)[#@][0-9]+\.enter\([0-9]+, ${cat}, ")
      first=$(region_of "${sw%%:*}")
    fi
    l=$(grep -nE -- "-> ${cat}\.set_input_region\(" <<<"$seg" | tail -1 | cut -d: -f1)
    if [ -n "$l" ]; then
      if sed -n "${l}p" <<<"$seg" | grep -qE 'set_input_region\(wl_region'; then last=$(region_of "$l"); else last=nil; fi
    fi
  fi
  echo "$dt|$ent|$first|$last|${rect:-none}|${sw%%:*}"
}

# within_ms DT MAX: DT is a number and 0 <= DT <= MAX.
within_ms() { awk -v d="$1" -v m="$2" 'BEGIN { exit !(d != "none" && d != "" && d + 0 >= 0 && d + 0 <= m) }'; }

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
  # Manager decision 12: the full region phase ends at most 300 ms after
  # the catcher's ready commit (S-021 AC4; it was the first frame), and the
  # 400 ms bound leaves 100 ms for a loaded machine.
  local sdt=none sent=0 sfirst=none slast=none srect=none pdt=none fr
  if [ "$at" = still ]; then
    IFS='|' read -r sdt sent sfirst slast srect sw <<<"$(still_region "$seg")"
  elif [ "$at" = 0 ] && [ -n "$sw" ] && [ -n "$child" ]; then
    fr=$(catcher_ready "$seg")
    [ -n "$fr" ] && fr=$(sed -n "${fr}p" <<<"$seg")
    [ -n "$fr" ] && pdt=$(awk -v a="$(trace_ms "$fr")" -v b="$(trace_ms "$(sed -n "${sw}p" <<<"$seg")")" 'BEGIN { if (a == "" || b == "") print "none"; else printf "%.1f\n", b - a }')
  fi
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
  elif [ "$at" = still ]; then
    echo "INFO S-019 show $n (still, decision 12): the catcher's first wl_region ${sdt} ms after the catcher's ready commit, enters on the catcher before it $sent, that region (adds, last add) $sfirst, last catcher region $slast, hive rect $srect"
    check "S-019 decision 12, S-021 AC4 show $n (still): no position, so the catcher's first input region is the hive rectangle (one add, the last \"hive surface\" line) within 400 ms of the catcher's ready commit (first commit after the first configure)" \
      '[ "$srect" != none ] && [ "$sfirst" = "1 $srect" ] && within_ms "$sdt" 400'
    check "S-019 decision 12 show $n (still): no enter on the catcher before that switch, and it stays the catcher's last region (no move afterwards)" \
      '[ "$sent" -eq 0 ] && [ "$slast" = "1 $srect" ]'
  else
    check "S-019 AC8 show $n: the catcher keeps its default input region until the first position (an enter on it, and no set_input_region(wl_region) before it)" \
      '[ -n "$fp" ] && [ "$early" -eq 0 ]'
    check "S-019 AC8 show $n: after the first position the catcher's last input region is one add equal to the hive rectangle (S-020 AC1)" \
      '[ -n "$hh" ] && [ -n "$sw" ] && [ -n "$last" ] && [ "$last" -ge "$sw" ] && [ "$last_reg" = "1 $hx $hy $hw $hh" ]'
    echo "INFO S-019 show $n: switch to the hive rectangle ${pdt} ms after the catcher's ready commit (negative: before it)"
    check "S-019 decision 12, S-021 AC4 show $n: the switch at the first position comes at most 400 ms after the catcher's ready commit (first commit after the first configure)" \
      'awk -v d="$pdt" "BEGIN { exit !(d != \"none\" && d + 0 <= 400) }"'
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

# S-021 (AC3, AC4, AC7): a drop show on the layer path binds a hidden
# catcher and draws the hive only once a drag of files with an action has
# entered it. With SBARHIVE_DEBUG_FOCUS the daemon prints on stderr, in the
# same stream as the WAYLAND_DEBUG trace, exactly one of these lines per drop
# show on the layer path (none on the window path, none for a tap that binds
# nothing):
#   hive reveal: N       the hive is drawn now; N is the number of ids in
#                        the "drop cells:" line before it
#   hive probe: hidden (R)
#                        the catcher is gone without a drawn hive; R is
#                        pointer (wl_pointer.enter on the catcher: no drag),
#                        no-uri-list (the offer has no text/uri-list),
#                        no-actions (the files have no action), release (the
#                        key of `drop --hold` let go) or cap (10 s with no
#                        event, ENTER_TIMEOUT)
# The lines carry no timestamp, so "before" is the line order of that
# stream. "Visible" is the first attach(wl_buffer) on the hive subsurface
# (the wl_surface given to get_subsurface); the catcher still takes its one
# transparent pixel at bind, and the subsurface, its position and both input
# regions are made at bind as before S-021: only the hive's buffers wait.
#
# probe_show SEG prints
#   ALL|REVEALS|HIDES|N|R|CELLS|PRE|ATT|CATT|E1|A1|AFTER|CHILD
# ALL: lines starting "hive reveal:" or "hive probe:" (any form); REVEALS:
# "hive reveal: N" lines; HIDES: "hive probe: hidden (R)" lines with a
# reason of the list; N and R: of the first such line; CELLS: ids in the
# last "drop cells:" line before the reveal line ("none" if there is none);
# PRE: attach(wl_buffer) on the hive subsurface before the reveal line (in
# the whole show when there is none); ATT: all of them in the show; CATT:
# attach(wl_buffer) on the catcher; E1 and A1: segment lines of the first
# wl_data_device.enter naming the catcher and of the hive's first attach
# ("none"); AFTER: "hive anchor:" or "hive surface:" lines after the hide
# line; CHILD: the hive subsurface ("none").
probe_show() {
  local seg=$1 cat child all rev hid n="" r="" rl hl cells=none pre att catt e1 a1 after=0
  cat=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
  child=$(grep -oE -- '-> wl_subcompositor[#@][0-9]+\.get_subsurface\(new id wl_subsurface[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
  all=$(grep -cE '^hive (reveal|probe):' <<<"$seg")
  rev=$(grep -cE '^hive reveal: [0-9]+$' <<<"$seg")
  hid=$(grep -cE '^hive probe: hidden \((pointer|no-uri-list|no-actions|release|cap)\)$' <<<"$seg")
  rl=$(grep -nE '^hive reveal: [0-9]+$' <<<"$seg" | head -1)
  hl=$(grep -nE '^hive probe: hidden \((pointer|no-uri-list|no-actions|release|cap)\)$' <<<"$seg" | head -1)
  [ -n "$rl" ] && n=${rl##* }
  [ -n "$hl" ] && r=$(sed -nE 's/.*hidden \(([a-z-]+)\)$/\1/p' <<<"$hl")
  if [ -n "$rl" ]; then
    local cl; cl=$(head -n "${rl%%:*}" <<<"$seg" | grep -E '^drop cells:' | tail -1)
    [ -n "$cl" ] && cells=$(sed -E 's/^drop cells: ?//' <<<"$cl" | wc -w)
  fi
  local apat="-> no-hive-subsurface\\.attach"; [ -n "$child" ] && apat="-> ${child}\\.attach\\(wl_buffer"
  att=$(grep -cE -- "$apat" <<<"$seg")
  if [ -n "$rl" ]; then pre=$(head -n "${rl%%:*}" <<<"$seg" | grep -cE -- "$apat"); else pre=$att; fi
  catt=0; [ -n "$cat" ] && catt=$(grep -cE -- "-> ${cat}\\.attach\\(wl_buffer" <<<"$seg")
  e1=none; [ -n "$cat" ] && e1=$(grep -nE -- "^\[[0-9:.]+\] wl_data_device[#@][0-9]+\.enter\([0-9]+, ${cat}, " <<<"$seg" | head -1 | cut -d: -f1)
  a1=$(grep -nE -- "$apat" <<<"$seg" | head -1 | cut -d: -f1)
  [ -n "$hl" ] && after=$(tail -n +"${hl%%:*}" <<<"$seg" | grep -cE '^hive (anchor|surface):')
  echo "$all|$rev|$hid|${n:-none}|${r:-none}|$cells|$pre|$att|$catt|${e1:-none}|${a1:-none}|$after|${child:-none}"
}

# probe_checks NAME SEG WANT: the S-021 checks of one drop show on the layer
# path. WANT is "reveal" (a drag of files with an action enters) or the
# reason of the hide (pointer, no-uri-list, no-actions, release, cap).
# Sets PROBE (probe_show's fields) for the caller's own checks.
probe_checks() {
  local name=$1 seg=$2 want=$3
  PROBE=$(probe_show "$seg")
  local all rev hid n r cells pre att catt e1 a1 after child
  IFS='|' read -r all rev hid n r cells pre att catt e1 a1 after child <<<"$PROBE"
  echo "INFO S-021 $name: probe lines $all ($(grep -E '^hive (reveal|probe):' <<<"$seg" | tr '\n' '|')), reveal N $n, hide reason $r, ids in the drop cells line before the reveal $cells, hive subsurface $child: attach before the reveal line $pre, in the show $att, first at line $a1; catcher attach $catt; first drag enter on the catcher at line $e1; anchor or surface lines after the hide $after"
  if [ "$want" = reveal ]; then
    check "S-021 AC3 $name: exactly one probe line, \"hive reveal: N\" (no \"hive probe:\" line), and N equals the ids of the \"drop cells:\" line before it" \
      '[ "$all" -eq 1 ] && [ "$rev" -eq 1 ] && [ "$n" != none ] && [ "$cells" != none ] && [ "$n" -eq "$cells" ]'
    check "S-021 AC4 $name: no attach on the hive subsurface before the \"hive reveal:\" line, at least one after it (visible), and the catcher took its pixel" \
      '[ "$child" != none ] && [ "$rev" -eq 1 ] && [ "$pre" -eq 0 ] && [ "$att" -ge 1 ] && [ "$catt" -ge 1 ]'
    check "S-021 AC7 $name: the hive's first attach comes after the first wl_data_device.enter on the catcher" \
      '[ "$e1" != none ] && [ "$a1" != none ] && [ "$a1" -gt "$e1" ]'
  else
    check "S-021 AC3 $name: exactly one probe line, \"hive probe: hidden ($want)\" (no \"hive reveal:\" line), and no \"hive anchor\" or \"hive surface\" line after it" \
      '[ "$all" -eq 1 ] && [ "$hid" -eq 1 ] && [ "$r" = "$want" ] && [ "$after" -eq 0 ]'
    check "S-021 AC4 $name: no attach on the hive subsurface in the whole show, and the catcher took its pixel" \
      '[ "$att" -eq 0 ] && [ "$catt" -ge 1 ]'
  fi
}
