# S-088 Download page: the demo film, a sample to try, and the honest warning
Status: approved under manager delegation (2026-10-26)

## Story
As someone downloading Kawaro for the first time, I want the download page to show the drag demo while my file downloads, hand me a sample image to try it on right away, and explain the Windows warning honestly, so that the first try succeeds instead of stalling at a scary dialog.

## Design intent (owner, 2026-10-26)
The owner asked that the download experience itself teaches: a video of the drag and the shortcut so the user understands, a prompt to try it while the download runs, a "Try it on an image" button, and a simplified settings pointer afterwards. Beautiful and systematic: the site design system, Arabic and English, RTL.

## Files
`site/download/index.html`, `site/en/download/index.html`, `site/build.py`, `site/media/` (the demo clip reuse and poster), `site/assets/` (the sample image), `tests/site/`, `scripts/site/check-links.sh` (only if new asset links appear).

## Acceptance criteria
- AC1: the Arabic and English download pages show the demo film (the existing hero demo clip, silent, with a poster and native controls, no autoplay with sound) next to the download buttons, with a caption naming the flow: drag a file, hold the drag key, drop on the cell. The caption names no specific key until the measured contracts exist: it reads the key from the S-084 manager decision line for Windows and the S-075 measured contract for Linux, and a page build that has neither may ship only the key-free wording (drag, drop on the cell). Under reduced motion the page shows the poster and the numbered caption steps instead of the moving film.
- AC2: beside the film, a numbered "after the download" walkthrough in Arabic and English: install, open Kawaro from the right click menu or the icon, drag a file while holding the drag key (wording follows the same measured-key rule as AC1: if the S-084 winner is the right-button drag, the step says drag with the right button and holds no key), drop on the cell of the action, the result appears next to the original. Each step names the control the user will actually see (S-087 welcome buttons included).
- AC3: a "Try it on an image" / «جرّب على صورة» button downloads a small sample image from the site (a few hundred kilobytes at most, licence clean, generated from the same fixture S-087 embeds, so the app and the page always show the same sample) into the browser's Downloads folder, and the accompanying line tells the user to drag that file onto the hive to get a result beside it. The button works in both languages and has a descriptive accessible label.
- AC4: the Windows download block links the SmartScreen guide of S-083 and carries its two-line honest note; the page never states that the build is signed or that the warning disappears, and never asks the user to disable SmartScreen.
- AC5: the site keeps every existing security header and language switch line; no long dashes anywhere in the touched pages; `PATH=/usr/bin:$PATH bash tests/site/run_heavy.sh` passes, `scripts/design-lint.sh` passes, and the pixel tests hide the film before comparison (the known convention for film elements).
- AC6: one CHANGELOG line in both languages.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | page tests assert the film element, poster, controls, both caption languages and the numbered walkthrough; a reduced-motion block renders the static variant | static, snapshot |
| AC3 | the sample asset is linked, under the size limit, and the button label exists in both languages; `check-links.sh` resolves the asset | static |
| AC4 | the Windows block links the guide and the note text passes a claim scan (no "signed", no "warning gone", no "disable") | static |
| AC5 | site heavy tests, design lint, no-long-dash check | static |

## Out of scope
- A new filmed production: this reuses the already published hero demo clip of S-068 and S-070; a fresh promo film is a separate video task with its own brief.
- The in-app welcome window and its Try button (S-087), the signing pipeline (S-083), the free stores (S-078).
- Analytics of any kind (the site claims no analytics and Cloudflare Web Analytics is switched off).
