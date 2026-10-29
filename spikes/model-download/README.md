# Spike: model download mechanism (ADR 0010 Part A)

Throwaway. Proves that the updater's primitive (system `curl` with a fixed argument list, as in `crates/core/src/update/fetch.rs`) is enough for resumable, pinned, verified downloads of 1 GB class files. Nothing here touches the internet: every test uses local servers on 127.0.0.1 (a Range capable `http.server` variant written in `test_download.py`).

Difference from production: production is https only (`--proto =https --proto-redir =https`). Tests use plain http on loopback, so `downloader.build_args(test=True)` widens both flags to `=http,https`, and `download(test=True)` asserts the host is 127.0.0.1. TLS itself is not exercised.

Run: `python3 -I test_download.py` (about 35 s, stdlib only, curl 8.18.0 on Fedora). Files: `downloader.py` (reference behaviour, 113 lines), `test_download.py` (19 tests).

## Result: 19 of 19 passed

| # | Test | Flags that matter | Result | Notes |
|---|------|-------------------|--------|-------|
| 1 | Resume after drop at 40% | `-C -` | PASS | First run exit 18, `.part` = 160000 of 400000 bytes. Second run sent `Range: bytes=160000-`; final sha256 equals source. |
| 2a | Wrong hash, right size | none (sha256 in code) | PASS | `.part` deleted, one report, final name never created. |
| 2b | Truncated (honest Content-Length, 70% of file) | none | PASS | curl exits 0; the size check catches it. Deleted, reported once. |
| 2c | Larger than pinned | `--max-filesize N` | PASS | curl exit 63. Also passes with the curl limit off: size check deletes it. |
| 3 | Atomic rename | `os.replace` | PASS | Sampled every 50 ms during a 4 s download: `.part` visible, final never visible while `.part` exists; a later corrupt download leaves no final file. |
| 4a | Redirect to second port (CDN stand-in) | `--location` | PASS | Followed, served from port B. |
| 4b | Redirect count bounded | `--max-redirs 2` on 3 hops | PASS | curl exit 47. |
| 4c | Production flags refuse an http URL | `--proto =https` | PASS | curl exit 1. `download()` also refuses non-https before spawning curl (`not-https`, server saw no request). |
| 4d | Redirect to http refused | `--proto =http,https --proto-redir =https` | PASS | `curl: (1) Protocol "http" disabled (in redirect)`. Test set (`--proto-redir =http,https`) follows it on loopback. |
| 5a | Stall at 1 byte/s aborted | `--speed-limit 100 --speed-time 2` | PASS | Exit 28 after 2.0 s. `.part` kept for resume. |
| 5b | Slow steady download not aborted | `--speed-limit 1000 --speed-time 2`, no `--max-time` | PASS | 3.9 s download; `--max-time 2` on the same URL exits 28. |
| 6a | Announced Content-Length too big | `--max-filesize` | PASS | Exit 63 before the body. |
| 6b | Server lies (no Content-Length, endless) | `--max-filesize` | PASS | curl itself stops it: `(63) Exceeded the maximum allowed file size (400000) with 400000 bytes`. |
| 6c | Exact size, no Content-Length | `--max-filesize` | PASS | Accepted, so the limit is not off by one. |
| 6d | Same lying server, curl limit off | `.part` size polling | PASS | Watcher kills curl when `.part` > pinned size, deletes it. Belt and braces; granularity is the poll interval. |
| 7 | Disk space | `shutil.disk_usage` | PASS | Fake free space below size + margin: `no-space`, curl never started (server log empty). Real `disk_usage` with `1 << 60` refused. |
| 8a | Progress at 1 Hz from `.part` size | none | PASS | Samples `[24, 49, 74]` percent for a 4 s download, monotonic. |
| 8b | `curl --progress-bar` | | observed | 39 stderr fragments separated by `\r`, ends `100.0%`. Works but see recommendation. |

## Findings and recommendation

**Recommended flag set** (the existing updater set, minus `--max-time`, plus resume and progress timeout):

```
--disable --silent --show-error --fail --location
--proto =https --proto-redir =https --max-redirs 5
--connect-timeout 30 --speed-limit 1024 --speed-time 30
-C - --max-filesize <pinned size>
--user-agent kawaro-models --output <dest>.part <url>
```

- `-C -` resumes from the size of the existing output file. Because `--fail` is on, a stale `.part` that equals the full size makes the server answer 416 (exit 22); `downloader.py` therefore skips curl when `.part` equals the pinned size and goes straight to verification. A `.part` larger than pinned is deleted first.
- Keep the `.part` only after retryable exits (18, 28, 52, 55, 56). Delete it on 22, 33 (server ignored Range), 63 and on any verify failure.
- If a server ignores Range and answers 200, curl exits 33 (not exercised here; handled by the delete rule, untested).
- `--speed-limit/--speed-time` is the progress based timeout. Never `--max-time` for models.
- **Size ceiling:** curl 8.18 enforces `--max-filesize` for both announced and unannounced sizes, so the flag is enough. Whether older curl (Windows 7.55, see below) enforces it on bodies without Content-Length is NOT verified; the `.part` size check after exit and the polling watcher cover it.
- **Progress:** poll the `.part` size against the pinned size once per second in the daemon (a `stat`, no parsing, no stderr coupling, correct across resumes, works the same on every curl version). `--progress-bar` output is a `\r` separated, locale and version dependent stream and would force reading a pipe in real time; `--write-out` only prints at the end. Not recommended.
- The `.part` watcher also kills curl when `.part` exceeds the pinned size, independent of curl version.
- Honest limit: all tests are plain http on loopback. HTTPS, a real CDN redirect with a signed URL, and servers that send 200 to a Range request were not tested.

## Windows note (from curl documentation knowledge, not tested)

`curl.exe` ships in System32 on Windows 10 1803 and later (7.55.1 at first; updated by Windows Update to 8.x on current Windows 10 and 11, exact version varies by build). Release versions where each flag appeared, from the curl manual and changelog as remembered:

| Flag | Since | Within 7.55? |
|------|-------|--------------|
| `--silent`, `--show-error`, `--fail`, `--location`, `--output`, `--user-agent`, `--max-redirs`, `--connect-timeout` | very old | yes |
| `--max-filesize` | 7.10.8 | yes |
| `-C -`, `--speed-limit`, `--speed-time` | very old | yes |
| `--proto`, `--proto-redir` | 7.20.2 | yes |
| `--disable` (`-q`) | old (long form 7.x) | yes, as first argument |

Avoid because they need newer curl: `--fail-with-body` (7.76), `--no-clobber` (7.83), `--ssl-revoke-best-effort` (7.70, may be needed on Schannel behind inspecting proxies), `--write-out '%{json}'` (7.70). None are in the recommended set. Uncertain: exact Windows 10 versions that still carry 7.55.1, and whether 7.55's `--max-filesize` stops a body without Content-Length. Test on a Windows 10 virtual machine before relying on it.
