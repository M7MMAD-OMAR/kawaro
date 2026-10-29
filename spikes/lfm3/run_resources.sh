#!/usr/bin/env bash
# Resource survey: speed by thread count, speed by context depth, RSS by context size.
# Run through scope.sh. Output: out/resources-<model>.txt (llama-bench markdown tables).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
HOME_D="$HOME/.local/share/kawaro-spike/lfm3"
BENCH="$(find "$HOME_D/runtime/b11972" -name llama-bench | head -1)"
M="$1"; TAG="$2"
OUT="$HERE/out/resources-$TAG.txt"
{
  echo "== threads 1,2,4,8 (pp512, tg128)"
  "$BENCH" -m "$M" -t 1,2,4,8 -p 512 -n 128 -r 2 -o md
  echo "== prompt processing by size (4 threads): pp 256, 1024, 4096"
  "$BENCH" -m "$M" -t 4 -p 256,1024,4096 -n 0 -r 1 -o md
  echo "== decode at depth (4 threads): tg128 after 0, 4096, 16384 tokens of context"
  "$BENCH" -m "$M" -t 4 -p 0 -n 128 -d 0,4096,16384 -r 1 -o md
} 2>&1 | tee "$OUT"
echo "== RSS by context size" | tee -a "$OUT"
"$HERE/rss_ctx.py" "$M" | tee -a "$OUT"
