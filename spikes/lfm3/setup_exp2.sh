#!/usr/bin/env bash
# Whisper tiny and medium (CT2) for the tier survey. Resumable.
D="$HOME/.local/share/kawaro-spike/lfm3"
$D/venv/bin/python - <<'PY'
from huggingface_hub import snapshot_download
for r in ("Systran/faster-whisper-tiny", "Systran/faster-whisper-medium"):
    print(snapshot_download(r))
PY
echo "setup2 done"
