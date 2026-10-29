#!/usr/bin/env bash
# Third stage: Omni (audio) first, then the text Q8_0 reference.
set -euo pipefail
DEST="${LFM3_SPIKE_DIR:-$HOME/.local/share/kawaro-spike/lfm3}"
get() { local url=$1 out=$2 sha=$3
  if [ -f "$out" ] && echo "$sha  $out" | sha256sum -c --status; then echo "ok (cached) $out"; return; fi
  for i in 1 2 3 4 5 6 7 8; do curl --http1.1 -fL --proto '=https' --proto-redir '=https' --max-redirs 5 -C - --retry 5 --retry-delay 3 -o "$out.part" "$url" && break; sleep 5; done
  echo "$sha  $out.part" | sha256sum -c --status; mv "$out.part" "$out"; echo "ok $out"; }
H=https://huggingface.co/LiquidAI


get "$H/LFM3-Omni-2B-GGUF/resolve/main/LFM3-Omni-2B-Q4_0.gguf" "$DEST/models/LFM3-Omni-2B-Q4_0.gguf" 6d2273a383b92dd7c5db0eb4c8dfeae13e805b485a6621c4b391ae179c2b5f28
get "$H/LFM3-Omni-2B-GGUF/resolve/main/mmproj-LFM3-Omni-2B-Q8_0.gguf" "$DEST/models/mmproj-LFM3-Omni-2B-Q8_0.gguf" 720ff4fc09df0e026c4ec641d0c698bf27b5bcd50561944c88eda5e3dffe1839
get "$H/LFM3-1.7B-GGUF/resolve/main/LFM3-1.7B-Q8_0.gguf" "$DEST/models/LFM3-1.7B-Q8_0.gguf" 8f6b24182258205e9e01e21f1f24a1f208c1e8115578825d202ce39976af20b6
echo "done stage 3"
