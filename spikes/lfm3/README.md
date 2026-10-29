# LFM3 spike

Throwaway experiment for ADR 0010 (`docs/research/lfm3-local-models.md`, section 8). Nothing here is merged into `crates/`. Weights and the runtime live outside the repository in `~/.local/share/kawaro-spike/lfm3/`.

## Pass bars (written before any result exists)

The first full run of each test is the headline number. If the prompt is tuned afterwards on the same items, the tuned figure is reported as tuned, never as accuracy.

Natural language to action (`run_actions.py`, 30 requests, QAD-Q4_0 is the candidate, Q8_0 the reference):

- Action accuracy at least 90 percent overall, at least 85 percent for Arabic, at least 90 percent for English.
- Every request whose right answer is `none` answered `none`.
- Registry rejection at most 5 percent (unknown id, wrong file kind, unknown or out of range param). This measures the claim "the model proposes, the registry disposes".
- Cold first request (prompt not cached, the daemon just started) at most 10 seconds on 4 cores; the median warm request at most 3 seconds.
- Peak RSS at most 2.5 GB.

OCR (`run_ocr.py`, 10 synthetic renders, VL-2B against Tesseract `ara+eng`):

- Character error rate, Arabic normalised (diacritics stripped, alef forms unified, whitespace collapsed): VL at most 1.25 times the Tesseract rate for Arabic, and not worse than Tesseract for English.
- At most 60 seconds per page on 4 cores, peak RSS at most 4 GB.
- Renders are synthetic and cleaner than real scans, so both engines look better than they will on real files. A pass here means "worth a real-scan test", not "ship".

Decision rule: if the text bars pass, the action selector goes to a story. If the OCR bars fail, `image.to-text` stays on Tesseract and VL is dropped from the starter catalog. The download mechanism (ADR 0010 part a) does not depend on either result.

## Method notes

- Runtime: the official llama.cpp release b11972 for Ubuntu x64 (CPU), sha256 pinned in `fetch.sh`. It reports `version: 0.8.0-dev (build 11972)`, so a runtime probe must parse the build number, not a semantic version.
- `fetch.sh` and `fetch2.sh` resume from `.part` files and verify sha256 before the atomic rename, the same rule the real downloader will use.
- Always run through `scope.sh` (6 GB memory scope). CLAUDE.md asks for `CPUQuota=200%`; the spike uses 400% with `-t 4` so the speed numbers resemble a 4 core laptop, and pins the CPUs with `taskset`. Memory ceiling is unchanged.
- The server binds 127.0.0.1 with a random `--api-key`, one slot (`-np 1`), web UI off. Local model paths only, never `-hf`.
- Results land in `out/*.json`. Logs are ignored by git.
