# Capability survey cases

Throwaway inputs for a broad survey of a 1.7B text model and a 2B vision model inside Kawaro. Nothing here is part of the workspace. Everything is synthetic and written by hand or rendered by script; no real user data.

## Files

- `text_cases.jsonl`: 48 text cases, 8 categories x (3 ar + 3 en). Built by `../make_text_cases.py`. Line: `id, category, lang, system, user, gold, check, max_tokens`.
- `vl_cases.jsonl` and `img/`: 24 image cases (12 ar, 12 en), 2 per kind and language pair: receipt (2 per language), table to Markdown, bar chart (2), shape scene, UI screenshot, form, stamped document, handwriting-like, low contrast, rotated 90 degrees. Built by `../make_vl_cases.py` (Pillow with raqm, fixed seed). Line: `id, lang, kind, image, prompt, gold, check`.
- `audio/`: 6 clips (ar1 to ar3, en1 to en3), 16 kHz mono wav, 9 to 14 seconds, each with a `.txt` reference transcript. Built by `../make_audio_cases.sh`.
- Checkers: `../../check_text.py` and `../../check_vl.py`, both `score(case, output_text) -> (ok, reason)`, stdlib only, `python3 -I` compatible. `check_text.py` also runs as a script over a `{"id","output"}` jsonl.

## How gold is defined

Text checks (`check` field):

- `json_fields`: parse the first JSON object in the output. `gold.fields` requires exactly those keys; strings match after normalisation (Arabic diacritics, alef forms and case folded) by equality or containment, numbers within 0.005. `gold.exact` (tool_call) requires the whole object to equal the gold, including `{"tool":"none","args":{}}` for out of scope requests.
- `keywords`: every group in `any_groups` needs at least one alternative present; `forbid` words must be absent; optional `max_words` and `max_sentences`. For classify, `label` must be the only label from `labels` found in the output.
- `regex`: first line of the output must match every pattern in `patterns` (lowercase, no spaces, right extension), contain one of `tokens_any`, and stay within `max_len`.
- `flag`: passes when the output contains a "cannot know" marker (English or Arabic list in `check_text.py`). It does not detect a confident wrong answer that also hedges; read failures by eye.

Vision checks: `json_fields` as above; `table` compares a Markdown table cell by cell (first Arabic column is the rightmost one, the prompt says so); `keywords` uses `gold.all` groups, where an entry starting with `#` is a number matched on digit boundaries (`#58` does not match 580).

Known limits: keyword checks reward presence, not exact transcription, so they cannot give a character error rate. Gold wording was fixed before any model ran.

## Audio caveat

Clips come from local synthesizers: `espeak-ng -v ar` for Arabic (robotic, no diacritics, so the pronunciation is guessed) and `flite` voice `slt` for English. No piper, festival or Arabic mbrola voice is installed and nothing was downloaded. Synthetic speech is weak evidence: it is cleaner than real speech, has no accents, noise or overlap, and the Arabic voice is much worse than a human. A pass here means "worth a real recording test", never "ship".

## Rendering notes

Noto Naskh Arabic has no hyphen glyph, so ISO dates and phone numbers inside Arabic images are drawn with Liberation Sans. Arabic stamps carry no date for the same reason. The shape scene differs per language (3 trees and a red roof in English, 2 trees and a blue roof in Arabic).
