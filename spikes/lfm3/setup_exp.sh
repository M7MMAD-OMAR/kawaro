#!/usr/bin/env bash
# Experiment setup: Whisper large-v3-turbo (CT2) and an EasyOCR venv (CPU torch). Resumable.
set -u
D="$HOME/.local/share/kawaro-spike/lfm3"
$D/venv/bin/python - <<'PY'
from huggingface_hub import snapshot_download
print(snapshot_download("deepdml/faster-whisper-large-v3-turbo-ct2"))
PY
echo "turbo done"
python3 -m venv "$D/venv-ocr"
"$D/venv-ocr/bin/pip" -q install torch torchvision --index-url https://download.pytorch.org/whl/cpu
"$D/venv-ocr/bin/pip" -q install easyocr
"$D/venv-ocr/bin/python" -c "import easyocr;print('easyocr ok')"
echo "setup done"
