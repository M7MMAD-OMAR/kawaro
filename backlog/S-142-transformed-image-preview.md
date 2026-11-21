# S-142: Transformed image preview

Status: engine preparation, transformed draft, scheduler and panel integration source implemented; parent source-reviewed on 2026-11-21 (fixed missing Clone derives on the scheduler Job and Work types). Known limit: a superseded running preview is not cancelled; it finishes and is discarded (the scheduler tests pin one running plus one waiting request). Runtime, timing and visual checks deferred by owner order.
Date: 2026-11-15.
Spec: ../docs/specs/2026-11-15-next-features.md
Implementation packet: ../docs/superpowers/plans/2026-11-15-next-features.md

## Story
I want to see the intended image result before conversion, with an honest distinction between a quick display draft and an explicitly encoded artifact.

## Current facts
- stage_two.rs render shows the source draft; the export image path applies orientation, transforms, encoding and metadata.
- The image pipeline is private to engines. UI depends on core; daemon composes both.
- Session/panel generations exist, but current decode does not preempt cancellation.

## Acceptance criteria
- AC1: Share operation/geometry/orientation/alpha/metadata preparation with export, without changing the Engine trait or adding engines to UI dependencies.
- AC2: Provide automatic transformed draft and explicit encoded-result modes. Draft never exposes exact encoded size; encoded mode counts final artifact bytes and displays a bounded decode of those bytes.
- AC3: Use one running and one latest pending request, 180 ms draft debounce, a 512-pixel display-side cap and the specified display-cache limits. Reject stale session/panel/request/source results.
- AC4: Apply the spec admission limits before full encode, including 16,000,000 source pixels and 64 MiB input. Describe these as policies, not hard process-RSS or latency guarantees.
- AC5: Preserve Run/Back/Escape/Reset, RTL and small-panel footer reachability. No preview job/output write, no automatic compress-to encode, no changed defaults.
- AC6: Cover the exact source/export parity and stale-event cases in plan section 1 when checks resume; do not report tests as run now.

## Future focused cases, not run
- 400x200 at 50 percent gives intended 200x100; square crop gives 200x200 in both paths.
- EXIF6, JPEG metadata on/off and transparent-to-JPEG match the real engine rules.
- A/B edits and closing/replacing a panel publish only the newest permitted result.
- Oversize/corrupt/unsupported input and unreachable byte target write nothing and expose truthful states.

## Boundaries
No PDF/video/animation preview, new codecs, platform adapters, release or blanket timing guarantee.

## Delivery and review
Parent owns architecture, packet approval and final review. Selected Pi/MiniMax-M3.1-Flash executes only a later authorized bounded packet. Native platform qualification follows stable feature behavior. Source implementation, executed acceptance and public release are recorded separately.

## Planning log
- 2026-11-15: parent authored this story from current source research and the owner request. No product code or executable check changed or ran.
