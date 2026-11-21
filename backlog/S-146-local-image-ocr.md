# S-146: Optional local image OCR

Status: source implemented and parent source-reviewed on 2026-11-21 (adapter, language preflight, probe cache, APNG refusal). Runtime and OCR quality unverified; executable checks deferred by owner order.
Date: 2026-11-15.
Spec: ../docs/specs/2026-11-15-next-features.md
Implementation packet: ../docs/superpowers/plans/2026-11-15-next-features.md

## Story
I want printed text from an image saved locally as a new UTF-8 file, with explicit language selection and no document upload.

## Current facts
- Existing PDF text extraction is not image OCR.
- Historical real Arabic evaluation chose Tesseract over LFM3-VL and did not justify universal preprocessing.
- Current process and queue contracts already provide child lifetime and safe temp-output finalization.

## Acceptance criteria
- AC1: Add a system Tesseract adapter and typed image.to-text action for still JPEG/PNG/BMP inputs, with ara/eng/ara+eng selection only after each requested language is detected. Share deterministic orientation/alpha preparation into private PNG and explicitly refuse unsupported/multiframe containers.
- AC2: Keep one task/one txt output and the existing engine trait. No base-install expansion, model fetch, LLM or Python runtime.
- AC3: Pass input/output/language as separate arguments, use private work space, check exit and expected valid UTF-8 output, and let the queue alone finalize.
- AC4: Reuse process-tree cancellation/timeouts and cleanup; empty recognition returns a specific No printed text recognized failure with no committed output. The serialized skip-reason enum remains unchanged for old IPC clients.
- AC5: Preserve recognized Arabic spelling/digits/diacritics/order as returned; never silently post-edit or claim handwriting/translation support.
- AC6: Publish quality claims only after later real-fixture evaluation with ground truth and limitations. No OCR, inference or benchmark runs in the planning pass.

## Future focused cases, not run
- Missing executable/language preflight blocks execution with the capability explanation.
- Spaces/metacharacters remain one argv path; selected language is explicit.
- Empty/non-UTF8/missing output and nonzero exit have distinct non-success outcomes.
- Cancel/timeout leaves no final text file and source bytes unchanged; later real fixtures inspect mixed RTL and columns.

## Boundaries
No PDF OCR layer, automatic language guessing, handwritten recognition, preprocessing default, model installation or accuracy guarantee.

## Delivery and review
Parent owns architecture, packet approval and final review. Selected Pi/MiniMax-M3.1-Flash executes only a later authorized bounded packet. Native platform qualification follows stable feature behavior. Source implementation, executed acceptance and public release are recorded separately.

## Planning log
- 2026-11-15: parent authored this story from current source research and the owner request. No product code or executable check changed or ran.
