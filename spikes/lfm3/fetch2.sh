#!/usr/bin/env bash
# Second stage, shipping-quant first: VL mmproj, VL QAD-Q4_0, text Q8_0 reference last.
set -euo pipefail
DEST="${LFM3_SPIKE_DIR:-$HOME/.local/share/kawaro-spike/lfm3}"
get() { local url=$1 out=$2 sha=$3
  if [ -f "$out" ] && echo "$sha  $out" | sha256sum -c --status; then echo "ok (cached) $out"; return; fi
  curl -fL --proto '=https' --proto-redir '=https' --max-redirs 5 -C - --retry 5 --retry-delay 3 -o "$out.part" "$url"
  echo "$sha  $out.part" | sha256sum -c --status; mv "$out.part" "$out"; echo "ok $out"; }
H=https://huggingface.co/LiquidAI
get "$H/LFM3-VL-2B-GGUF/resolve/main/mmproj-LFM3-VL-2B-Q8_0.gguf" "$DEST/models/mmproj-LFM3-VL-2B-Q8_0.gguf" c5c5b58d544a18da919172d2d0368090ea1a4a351b6feb7f8a326a264a5918ab
get "$H/LFM3-VL-2B-GGUF/resolve/main/LFM3-VL-2B-QAD-Q4_0.gguf" "$DEST/models/LFM3-VL-2B-QAD-Q4_0.gguf" 621c455376cb96a20891697deb9975c6fb47dd5ea98ed3ad1f3b7e8f24f0b1e6
get "$H/LFM3-1.7B-GGUF/resolve/main/LFM3-1.7B-Q8_0.gguf" "$DEST/models/LFM3-1.7B-Q8_0.gguf" 8f6b24182258205e9e01e21f1f24a1f208c1e8115578825d202ce39976af20b6
echo "done stage 2"
