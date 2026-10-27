#!/usr/bin/env bash
# S-087: generates the bundled sample image of the first-run welcome,
# tests/fixtures/images/welcome-sample.png. It is the source the welcome
# window embeds (crates/ui/build.rs) and the S-088 download button builds
# its site asset from (S-087 D2 and D6). A honeycomb of hexagons in the
# light scheme of design/tokens.json, so even the sample art comes from the
# design tokens.
#
# Usage: scripts/gen-welcome-sample.sh
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/tests/fixtures/images/welcome-sample.png"
TOKENS="$ROOT/design/tokens.json"

colour() {
  jq -r ".color.light.\"$1\"" "$TOKENS"
}

# One pointy top hexagon as a magick polygon draw command.
hex() {
  awk -v cx="$1" -v cy="$2" -v r="$3" 'BEGIN {
    for (i = 0; i < 6; i++) {
      a = (i * 60 + 30) * 3.14159265358979 / 180;
      printf "%s%.2f,%.2f", (i ? " " : ""), cx + r * cos(a), cy + r * sin(a);
    }
  }'
}

BG="$(colour surface-0)"
EDGE="$(colour border)"
FILL1="$(colour surface-2)"
FILL2="$(colour accent)"
FILL3="$(colour surface-3)"
FILL4="$(colour surface-1)"
FILL5="$(colour success)"

R=54
SQ3=1.7320508
DX="$(awk -v r="$R" -v s="$SQ3" 'BEGIN { print r * s }')"
DY="$(awk -v r="$R" 'BEGIN { print r * 1.5 }')"

DRAW=""
row=0
y=0
while awk -v y="$y" 'BEGIN { exit !(y < 480 + 120) }'; do
  xoff=0
  [ $((row % 2)) -eq 1 ] && xoff="$(awk -v d="$DX" 'BEGIN { print d / 2 }')"
  col=0
  x=-80
  while awk -v x="$x" -v xo="$xoff" 'BEGIN { exit !(x < 640 + 120) }'; do
    cx="$(awk -v x="$x" -v xo="$xoff" 'BEGIN { print x + xo }')"
    pick=$(( (row * 3 + col * 2) % 5 ))
    case $pick in
      0) fill="$FILL1" ;;
      1) fill="$FILL4" ;;
      2) fill="$FILL3" ;;
      3) fill="$FILL2" ;;
      4) fill="$FILL5" ;;
    esac
    DRAW="$DRAW -fill '$fill' -stroke '$EDGE' -strokewidth 2 -draw 'polygon $(hex "$cx" "$y" "$R")'"
    col=$((col + 1))
    x="$(awk -v x="$x" -v d="$DX" 'BEGIN { print x + d }')"
  done
  row=$((row + 1))
  y="$(awk -v y="$y" -v d="$DY" 'BEGIN { print y + d }')"
done

eval "magick -size 640x480 xc:'$BG' $DRAW -depth 8 -modulate 102,104,100 '$OUT'"
echo "wrote $OUT"
