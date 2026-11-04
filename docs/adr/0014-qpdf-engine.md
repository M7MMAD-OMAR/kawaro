# ADR 0014: PDF structure through a qpdf child process

Status: accepted (owner approval 2026-11-04; implemented in S-128)
Date: 2026-11-04

## Context

Kawaro handles PDFs with lopdf (merge, rotate, split, page operations,
metadata) and Ghostscript (compress, pages to images), and S-126 added poppler
for text. The PDF benchmark (`docs/research/pdf-feature-benchmark.md`, P0 item
4) asks for three structural jobs that lopdf does not do safely: linearize a
file for fast web view, clean the object structure without re-encoding images,
and report whether a file is encrypted and structurally sound before the user
touches it.

qpdf is the tool the benchmark names: Apache-2.0, small, no daemon and no
network, with `--linearize`, a lossless structural rewrite and `--check` plus
`--show-encryption`. It is C++. Linking it would put a C++ build and ABI on
every target and grow the binary and the install, while the project already
treats heavy engines as separate programs: FFmpeg, LibreOffice, Ghostscript
and, since S-126, poppler. qpdf as a system program fits that model: it is
looked up at run time, its absence is a clear state, and the base install
stays light. It is never downloaded (ADR 0010) and never linked.

The owner approved the new child program and this record on 2026-11-04. S-128
implements it and also adds the Ghostscript compress DPI field described at the
end of the consequences.

## Options

| Option | Cost | What it gives | Verdict |
| --- | --- | --- | --- |
| Link the qpdf C++ libraries through a Rust binding | A C++ toolchain and ABI on every target, larger binary and install | Direct API access and no child process | Rejected: the base install stays light and the other heavy engines are child processes too |
| Reimplement linearize, cleanup and check on lopdf | Signature and encryption edge cases, object stream and cross-reference work, months of fixtures | No new program | Rejected: more code and more risk for a worse result than the established tool |
| Run the system `qpdf` as a child process | One more optional system program, hidden when absent | The three jobs with the tool the benchmark recommends, Apache-2.0, no link | Chosen |
| Do none of it | Nothing | No new program | Rejected: fast web view and safe structure checks are table stakes in the benchmark P0 list |

## Decision

1. **A new engine id `qpdf`.** `QpdfEngine` lives in
   `crates/engines/src/qpdf.rs`, is registered in `kawaro_engines::all()` and
   is `available()` exactly when `qpdf` is found by `locate::find`
   (`Program::Qpdf`, PATH first, then the known folders on Windows). Nothing
   is linked and nothing is downloaded.

2. **It is a separate process on purpose.** qpdf is C++, and linking it would
   add a C++ build and ABI to every target and to the packaging, while the
   base install stays light only if heavy tools are system programs. The child
   runs in a killable `ProcessTree` (S-030) with cancellation and a timeout,
   exactly like Ghostscript and poppler. The version comes from the system and
   is never vendored.

3. **Three ops with fixed commands.** Each task writes exactly one file:
   - `linearize`: `qpdf --linearize <absolute input> <output>`, a valid PDF
     with the `/Linearized` marker and unchanged pages.
   - `cleanup`: `qpdf --object-streams=generate --compress-streams=y
     --recompress-flate --compression-level=9 <absolute input> <output>`, a
     lossless structural rewrite with unchanged page content.
   - `check`: `qpdf --check` and `qpdf --show-encryption` both read, with
     their output captured into one UTF-8 text file at `task.output` (no byte
     order mark, LF line endings). It reports encrypted yes or no, whether a
     password is required, the permission summary and structural warnings, and
     it succeeds when qpdf returns warnings rather than a hard failure. It
     never decrypts and never modifies the input.

4. **Encrypted input is refused, not handled.** `linearize` and `cleanup`
   return `EngineError::Input` for an encrypted PDF, with a reason that says a
   password is required. There is no password prompt and no decryption in this
   story. `check` reports the encryption state without a password when qpdf
   can read the encryption dictionary, and says so when a password is needed
   to go further. A later story can add password handling.

5. **A missing program is a first class state.** `available()` is false, so
   the three actions are hidden from the hive and from `kawaro_plan`;
   `kawaro doctor` prints one `qpdf` line saying it is missing; a direct run
   returns `EngineError::Unavailable` naming `qpdf`, the package (`qpdf` on
   Debian, Ubuntu and Fedora) and `https://qpdf.readthedocs.io/`. There is no
   silent fallback to Ghostscript: its `-dFastWebView` re-distils the file and
   is not the same lossless operation.

6. **Signatures are reported, never re-signed.** Every structural rewrite
   invalidates signatures and object IDs. `check` reports what qpdf reports
   about signatures, `linearize` and `cleanup` do not promise to keep a
   signature, and Kawaro never signs. The limitation is recorded in
   `docs/engines.md` and in the action descriptions.

7. **Three actions, no options.** `pdf.linearize` (ring 2, per-file, suffix
   `-linearized`), `pdf.cleanup` (ring 2, per-file, suffix `-cleaned`) and
   `pdf.check` (ring 2, per-file, extension `txt`, suffix `-check`), all
   bilingual. None carries an option, because every parameter is fixed. The
   built-in registry grows to 56 actions and the option action count stays 17.

8. **Failures map to the existing variants.** A non-zero exit is
   `EngineError::Failed` with the stderr tail, a timeout is `Failed`, a
   killed child is `Cancelled`, an encrypted or unreadable input is `Input`,
   and a bad op is `UnsupportedOp`. No partial file is left at `task.output`;
   the core queue keeps the naming and the never overwrite rule.

## Consequences

- qpdf becomes the fifth child program beside FFmpeg, LibreOffice, Ghostscript
  and poppler, with the same lookup, `available()` and `doctor` treatment.
  The engines crate gains no dependency and the workspace MSRV stays 1.85.
- **Risk when qpdf is missing:** fast web view, lossless cleanup and the
  encryption and structure report are unavailable. The actions are hidden
  rather than failing late, `kawaro doctor` and the direct run say what to
  install, and every other PDF operation is unaffected. Tests that need the
  real tool skip with a printed notice; a fake program covers the flags.
- Recorded limitation: structural rewrites break signatures and object IDs, and
  Kawaro does not sign. Users who need signatures keep the original and do not
  run these actions on it.
- Recorded limitation: encrypted PDFs cannot use `linearize` or `cleanup`
  until a password story exists. `check` still reports the encryption state.
- `docs/engines.md` gains the qpdf row: Apache-2.0, system program only,
  version from the system, package hints for Debian, Ubuntu and Fedora.
- S-128 also exposes a Ghostscript compress DPI field (0 to 600, 0 means the
  preset decides) using the documented lowercase flags
  `-dDownsampleColorImages`, `-dDownsampleGrayImages`,
  `-dDownsampleMonoImages`, with `-dColorImageResolution`,
  `-dGrayImageResolution` and `-dMonoImageResolution`. That change stays in
  the Ghostscript adapter, not in qpdf.
