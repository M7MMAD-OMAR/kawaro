#!/usr/bin/env bash
# Throwaway spike: fetch the pinned llama.cpp CPU build and the LFM3 weights
# into a directory OUTSIDE the repository, resumable, sha256 verified.
# Hashes come from the GitHub release digest and the Hugging Face LFS oid.
set -euo pipefail

DEST="${LFM3_SPIKE_DIR:-$HOME/.local/share/kawaro-spike/lfm3}"
mkdir -p "$DEST/runtime" "$DEST/models"

get() { # url dest sha256
  local url=$1 out=$2 sha=$3
  if [ -f "$out" ] && echo "$sha  $out" | sha256sum -c --status; then
    echo "ok (cached) $out"; return
  fi
  curl -fL --proto '=https' --proto-redir '=https' --max-redirs 5 \
    -C - --retry 5 --retry-delay 3 -o "$out.part" "$url"
  echo "$sha  $out.part" | sha256sum -c --status
  mv "$out.part" "$out"
  echo "ok $out"
}

B=b11972
get "https://github.com/ggml-org/llama.cpp/releases/download/$B/llama-$B-bin-ubuntu-x64.tar.gz" \
  "$DEST/runtime/llama-$B-bin-ubuntu-x64.tar.gz" \
  24519b79152045d6b8b8844ee44ffd96e59a0e1d52a023aa28a1ddc55a3c30e6
if [ ! -d "$DEST/runtime/$B" ]; then
  mkdir -p "$DEST/runtime/$B"
  tar -xzf "$DEST/runtime/llama-$B-bin-ubuntu-x64.tar.gz" -C "$DEST/runtime/$B"
fi

H=https://huggingface.co/LiquidAI
get "$H/LFM3-1.7B-GGUF/resolve/main/LFM3-1.7B-QAD-Q4_0.gguf" "$DEST/models/LFM3-1.7B-QAD-Q4_0.gguf" \
  43a7d09a0e9cba91b63eb7052488b9564569bddfd65754d2e426465e572a7452
echo "done: $DEST"
