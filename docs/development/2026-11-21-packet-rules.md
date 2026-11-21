# Rules for every implementation packet (2026-11-21)

Parent: the Opus manager session owns plans, decisions and review. Executors: `pi/minimax/MiniMax-M3.1-Flash` and `antigravity/gemini-4-neon`. Each executor gets one packet with an explicit file scope.

## Hard rules

1. Work only inside the files your packet lists. If you must touch another file, stop and report why instead of editing it. Other agents edit other files in the same checkout at the same time.
2. Never run `git commit`, `git push`, `git stash`, `git checkout`, `git reset`, `git clean` or any command that rewrites the tree. The parent commits.
3. Do not run builds, tests, clippy, fmt, benchmarks, installers or compositors. The owner deferred all executable checks (CLAUDE.md, "Active programme"). Read code carefully instead: check every type, import, match arm and Slint property you use against the real source.
4. Read before you write: before you call any function or type, find its real definition (`graft ask`, `graft grep`, `graft skeleton <file>`, or `rg`). Never invent an API.
5. Preserve partial work already present. Extend it; do not rewrite a file from scratch.
6. Every exhaustive `match` you change: find all sibling matches with `graft grep` and update them all.
7. Design rules from CLAUDE.md apply: no shadows, gradients, one-sided borders, hand written colours or sizes (tokens only), no em dash or en dash anywhere (use comma, colon or a new sentence). UI strings in Arabic and English. Every interactive Slint element gets an accessible label. RTL: reverse element order by UI language.
8. Never put action logic in the UI. No network, no telemetry, never modify the original file.
9. English in code, comments and docs.

## Your final report

Reply with:
- Files changed, each with one line on what changed.
- Every new public function/type with its signature.
- Anything you could not finish and why.
- Every place you were unsure the code compiles (name the line).
- Future test cases a later check stage should run (not executed).
