# S-069 macOS base: CI build and test on a GitHub macOS runner
Status: review

## Story
As the maintainer, I want Kawaro to build and test on macOS through a GitHub-hosted runner, so that the macOS port (last in the platform order) starts from a measured list of failures instead of a guess, with no Apple hardware needed yet.

## Acceptance criteria
- AC1: `.github/workflows/macos.yml` has exactly one job, on `runs-on: macos-14` (Apple silicon).
- AC2: It triggers on push to `story/**`, on `workflow_dispatch` and on a weekly `schedule` (one five-field cron entry). It does not trigger on `pull_request` or on pushes to main.
- AC3: It runs `cargo check --workspace --all-targets --keep-going` (so one failing crate does not hide errors in the others) and `cargo test --workspace --no-fail-fast`, each with `continue-on-error: true`, so the job never blocks main.
- AC4: Each cargo step tees its output to a file under `logs/`, and the files are uploaded with `actions/upload-artifact@v7` even when a step failed.
- AC5: The job writes a table with the outcome of both steps to `$GITHUB_STEP_SUMMARY`.
- AC6: Permissions are `contents: read` only and the file contains no `secrets.` reference.
- AC7: `docs/adr/0009-macos-build-and-test.md` records the options and the decision.
- AC8: After the first run on `story/s069`, `docs/macos-gap.md` and `docs/macos-gap.ar.md` list every crate with its exact macOS failure class and a proposed next story.
- AC9: Nothing under `crates/` changes in this story.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | `tests/macos/test_workflow.py::test_runner_is_macos_14`: one job, runner string equals `macos-14` | unit |
| AC2 | `test_triggers`: push branches equal `['story/**']`, `workflow_dispatch` present, every cron has 5 fields, no `pull_request` | unit |
| AC3, AC4 | `test_cargo_steps_do_not_block`: both commands found, `continue-on-error` is true, `tee` in the run text | unit |
| AC4 | `test_artifact_upload`: `actions/upload-artifact@v7` present with `if: always()` | unit |
| AC5 | `test_step_summary_written`: `$GITHUB_STEP_SUMMARY` referenced | unit |
| AC6 | `test_read_only_permissions`, `test_no_secrets` | unit |
| AC1 to AC6 | `test_toolchain_and_checkout`, `test_no_long_dashes`: pinned actions and the dash rule | unit |
| AC8 | A real run on GitHub: `gh run list --branch story/s069` shows a completed run and its logs are read | integration |

Run: `python3 -m unittest -v tests/macos/test_workflow.py`.

## Out of scope
- Fixing any macOS compile or test failure (each gap gets its own story).
- Making the macOS job required, or running it on main and pull requests.
- Finder integration, code signing, notarization (needs a real Mac, ADR 0009).
- Cross compiling from Linux and macOS virtual machines (rejected in ADR 0009).

## Review log
- 2026-10-26: story written, test written first and shown failing (workflow file missing), then passing with the workflow.
