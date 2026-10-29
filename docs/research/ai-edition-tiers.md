# Optional engines by hardware: product view

Status: proposal built from measurements (2026-10-29). Nothing here is scheduled for a release. Numbers come from `spikes/lfm3/out/` on one machine (i7-14650HX, 4 threads, busy); slower laptops will be slower, so every time shown to a user must come from a calibration on their own machine, never from this table.

## 1. What actually changes the user's day

Ranked by measured value, not by novelty.

| Rank | Feature | Why it matters | Evidence |
|---|---|---|---|
| 1 | Speech to text (`audio.to-text`, `video.to-subtitles`) | Hours of work saved, nothing comparable in the base install, fully offline | Whisper turbo: word error 0.11 on Arabic read speech, 0.23 on Syrian dialect, 0.06 on English (research 8c) |
| 2 | Better OCR for scans | Only where Tesseract struggles | EasyOCR beat Tesseract on book pages (0.14 against 0.18) but lost on lines and noisy scans and is 20 to 40 times slower; no clear winner (below) |
| 3 | Text model features (file names, extraction, action proposals) | Convenience, easy to get wrong | 6/6 file names, 3/6 extraction, 80% action choice: suggestions only |

Not offered: LFM3-VL for OCR (lost on real pages), Omni for speech (Whisper is twice as accurate), translation, long-text rewriting.

## 2. Speech tiers (measured)

All five sizes, same 32 real clips (research 8c), int8 on CPU with 4 threads. RAM is the peak resident size of the whole run, measured with `/usr/bin/time -v`. Word error: Arabic read speech / Syrian dialect / English read speech.

| Tier | Engine | Disk | Peak RAM | Seconds per 10 s clip | Word error ar / syrian / en |
|---|---|---|---|---|---|
| Tiny | Whisper tiny | 0.08 GB | 0.28 GB | 0.5 | 0.64 / 0.71 / 0.11 |
| Fast | Whisper base | 0.15 GB | 0.37 GB | 0.7 to 1.0 | 0.47 / 0.93 / 0.11 |
| Balanced | Whisper small | 0.48 GB | 0.83 GB | 2.1 | 0.24 / 0.39 / 0.10 |
| Best | Whisper large-v3-turbo | 1.6 GB | 2.07 GB | 5 | **0.11 / 0.23 / 0.06** |
| (dropped) | Whisper medium | 1.5 GB | 2.12 GB | 4.6 | 0.12 / 0.29 / 0.07 |

Medium is dominated by turbo (same disk and RAM, slightly worse, slightly faster per clip only by noise), so it is not offered. Tiny and base are usable for English only; for Arabic they are poor, so the card must say so instead of offering them as a "light" option.

Reading it as a product: Tiny and Fast are English only. Balanced is the default for most machines. Best is the "I need it right" option and costs half the clip length in waiting time here: one hour of audio takes about 30 minutes, which is acceptable only as a background job with progress in the tray.

## 3. How the recommendation is made

At first use of the Models section (and again when the user asks) the daemon reads, locally and without sending anything: available RAM, physical cores, whether the CPU has AVX2, free disk on the models drive, and whether the machine is on battery. Then it runs a 5 second built-in clip through the installed or smallest engine to measure real speed.

Proposed rule, to be tuned with beta data (the thresholds are guesses, not measurements):

- Recommend Best when available RAM is at least 6 GB, at least 6 cores, and the calibration says 0.8 x clip length or faster.
- Recommend Balanced when available RAM is at least 3 GB and at least 4 cores.
- Otherwise show Fast for English and say plainly that Arabic quality needs a stronger machine, with a "run it anyway" option.
- Always show all tiers, mark one "Recommended for your computer", and let the user pick. The recommendation never downloads anything by itself.

Each row states in the user's own terms: size on disk, RAM used, "about N minutes for a one hour recording on your computer" (from calibration), and a plain quality line ("best for Arabic and dialects", "good for clear speech"). Removing an engine frees the space with one click.

## 4. Product rules that follow

1. Opt-in per engine, one click, licence line on the card, nothing leaves the machine (ADR 0010 part A; the download mechanism passed `spikes/model-download/`).
2. Honest speed: estimates come from the user's own calibration, with a range, never a promise.
3. Long jobs run in the daemon with tray progress and one notification, and can be cancelled; the original file is never touched.
4. Quality labels come from our measured table, updated when we re-measure. No marketing numbers on the card.
5. Failure is visible: if the recorded speech is noisy or the engine returns a repeating loop, the result is flagged as low confidence instead of saved silently.
6. Power use: on battery the Best tier asks before starting.

## 5. Open questions that need measurement before any story

- Distilled variants and a long-file memory test (RAM above is for 10 second clips).
- Noisy recordings, long files (chunking, memory), music with speech, phone calls.
- Dialects beyond Syrian and Egyptian (Gulf, Maghreb) and code-switching.
- Preprocessing was tried and did not help Tesseract (below); other noisy-scan ideas are open.
- Real laptops: a 4 core 8 GB machine and an older one, to validate the thresholds above.
- Licence check for each engine (Whisper is MIT; EasyOCR is Apache-2.0; both must be confirmed at the pinned versions) and the counsel question for any vendor-hosted weights.
