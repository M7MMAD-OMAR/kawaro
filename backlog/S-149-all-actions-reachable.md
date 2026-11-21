# S-149: keep every matching action reachable

Status: source implemented and parent source-reviewed on 2026-11-21: outer-ring paging with a More cell (id kawaro.more), page size equal to the accepted outer slots of the actual placement minus one, PageUp/PageDown, routing by action id, X11 pages planned at one fixed pointer. Known limit: a placement with fewer than 6 slots per page falls into the compact ring with More in ring 1 (no action lost). Runtime and visual checks deferred.

## Problem

ui/layout.rs::assign truncates the outer queue to RING2 (12). The image action registry now exceeds that capacity, so later actions such as target-byte compression and image OCR can disappear. Reordering the registry merely hides another action.

## Scope

Keep the current two-ring geometry and compact default surface. Add a visible overflow entry or paging control that exposes every remaining action in stable registry order inside the same hive session. The control must work by pointer and keyboard in both directions and RTL. Do not execute an action while browsing, steal focus, add native window controls or reset the drag/session ownership state.

Retain all matching actions independently from the currently placed page. Route selection and customization by stable action id. Back and Escape return through the existing session rules. Never silently drop a matching action or reuse the same slot id for a stale action after a page transition.

## Acceptance, execution deferred

- JPEG/PNG/BMP selection with all installed engines can reach image.to-text and image.compress-to.
- Every input action is reachable exactly once, including more than 18 custom actions.
- Empty/single-page cases preserve existing placement and navigation.
- LTR/RTL pointer and keyboard page transitions retain the current session and selection safely.
- A late preview or preset reply cannot mutate an unrelated action opened from another page.

Visual and runtime acceptance remain deferred with the other feature checks. Parent design must settle the exact overflow interaction before implementation; increasing the window without a bound or moving OCR earlier is not sufficient.
