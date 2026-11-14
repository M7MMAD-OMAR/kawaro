# Zoom-out review: S-083 to S-088 against the Kawaro product

> **Historical record, restored 2026-11-15.** This review was written on 2026-10-26
> against the S-083 to S-088 drafts as they stood at that time. It was restored from
> the branch `hermes-subagent/subagent-sa-0-e62d68ae`, commit `7b63bf8`, because
> `backlog/S-083` to `backlog/S-088` and `docs/PLAN.md` cite it and the file was
> missing from `main`. Its findings, severities and verdicts are historical. They
> are not current acceptance for those stories, they do not certify any platform
> as verified today, and they do not close any open item in `docs/PLAN.md`.
> No text was changed when the file was restored: the original already contains no
> em dash, no en dash and no guillemet, so no normalization was needed.

Persona: Skeptical Architect (egm-zoom-out), Mode REVIEW, 2026-10-26.
Scope: the six drafts S-083 to S-088 (present only as drafts in the main checkout at 8e91b5e) against CLAUDE.md, docs/PLAN.md, and the approved stories S-075, S-076, S-078, S-079. Tree claims in the drafts were spot checked: `installer/windows/kawaro.wxs` really has Product Name "Kawaro", a StartShortcut with no Icon attribute and no icon element, and AUMID DisplayName "Kawaro"; no `build.rs` in `crates/cli` or `crates/daemon` and no winres-style dependency anywhere; `crates/daemon/src/portal_bus.rs:246-252` writes `config.toml` automatically on a portal denial; `scripts/windows/` has `sign-local.sh` and `sign-update-manifest.sh` references but no `sign-release.sh`; `site/media/hero*` exists, so the S-088 reuse claim is sound.

## Verdict on the four altitude questions

1. **Do the six stories solve the five problems end to end?** Coverage exists for all five, but three end-to-end promises do not hold as written. Problem 3 (Alt conflicts) is fixed only on the default and picker paths: the S-048 chord (Ctrl+Alt+H) stays callable during a drag and contains the exact drag modifiers the S-084 research condemns (B2). Problem 2's captions describe a held drag key that may not exist if the right-button drag wins the S-084 measurement (M8), and the configured key they name is not necessarily the key the daemon listens for (M1). Problem 1 is solved only when a maintainer buys an OV certificate and reputation accumulates; the set ships guidance, not a warning removal, and says so honestly.
2. **What is missing for a user who installs today?** (a) Any measured Windows drag trigger: S-084's default is unnamed until a spike on a real machine or the S-044 VM, so after all six land minus that spike the Windows drag path is still undefined and the user falls back to the hazardous chord. (b) The simplified settings content and the drag-key picker are S-076b, outside this set and pending, yet S-087 and S-084 both depend on it. (c) A first-run state that survives the app's own config writers (M2). (d) Any live Windows evidence: all six close on static tests and owner-checklist steps, while the report they answer was observed live.
3. **Product law conflicts?** No telemetry conflict: S-088 keeps analytics off and S-083's Microsoft malware-portal submission sends release artifacts, not user files. No focus-stealing conflict: S-086 and S-087 open windows only from user clicks and explicitly refuse auto-open. One light-base-install gap: S-087 embeds demo frames and a sample image with no size budget (M9).
4. **Can each story land independently?** No. S-084 is a keystone whose manager decision feeds S-087 and S-088 and whose D1/D2 amend the pending S-076b, with no stated ordering (M3). S-087 cannot land before S-086 and disagrees with it about the mechanism (M4). S-088 needs S-083's guide and S-084's winner. S-085 and S-086 share `kawaro.wxs` and the s051 tests. Only S-083 is close to independent, and its signing half waits on a maintainer secret outside the agent chain.

## Findings

### Blockers

**B1. Severity: Blocker. Story: S-088 (AC1).**
Gap: AC1 tells the download page to write "Alt on Linux where S-075 measured it". S-075 measured no working Alt anywhere: Alt-only via the portal is disproven on GNOME 50 and KWin 6.7.5, the Hyprland Alt bind is "To be measured (S-075a)", and the KDE and GNOME native spikes are pending. The S-075 contract says the only valid Alt claims are the measured ones, and audit AUDIT-V2-001 already re-baselined one story for exactly this sin. As written, the AC makes a public, testable unmeasured claim and the site tests would encode it.
Strongest counter-argument: the caption is per platform and can be updated after S-075a measures Hyprland, so the AC describes a future state. Rebuttal: acceptance criteria are executed at land time; a test asserting the caption today ships the unmeasured claim on the live download page.

**B2. Severity: Blocker. Story: story set (S-084 with S-086, problem 3).**
Gap: the S-084 story statement promises the trigger "must never change what Explorer does with my drag", but the S-048 chord (default Ctrl+Alt+H, pressed during a drag) remains a shipped, documented Windows trigger and holds Ctrl and Alt during the drag. The story's own research says any held chord containing Ctrl, Shift or Alt flips the Explorer drag badge and changes what a drop outside the hive does. AC2 constrains only "the chosen trigger"; AC5 only preserves the chord's taken-conflict logging. The owner's reported failure, silently creating shortcuts, survives on the fallback path and in the current README instructions.
Strongest counter-argument: the chord only fires when the measured trigger conflicts, and the harm needs a drop outside the hive, so the blast radius is small. Rebuttal: the harm is silent modification of what a drop means, the exact bug reported; a story whose stated invariant is "never" cannot leave one callable path that violates it.

### Major

**M1. Severity: Major. Story: S-087 (AC3) with S-076 and S-075.**
Gap: AC3 has the demo captions name "the user's effective drag key from `Config` (`drop_key`, with the S-084 Windows default applied)". The effective trigger is defined by S-075's per-desktop contract, not by `drop_key`: wherever native Alt fails, the registered trigger is the named Super+Z fallback (S-075 AC4, AC7). On such a desktop the captions would instruct the user to hold a key the daemon does not listen for.
Strongest counter-argument: `drop_key` is the user's intent and S-075 registers it wherever possible. Rebuttal: S-075 AC7 itself enumerates states where the fallback key is the real trigger; "effective" must read the doctor state line, or the caption must say the configured key and the docs must name the fallback.

**M2. Severity: Major. Story: S-087 (AC1, D1) with S-084 and S-076.**
Gap: the first-run gate is "no config file yet", but `config.toml` is written by subsystems that have nothing to do with the welcome: `crates/daemon/src/portal_bus.rs:246-252` saves it automatically when the user denies the GlobalShortcuts portal dialog, any Settings save writes it, and S-084 AC4's "a fresh Windows config uses the chosen trigger" implies another writer. A user who denies a portal prompt or saves any setting before clicking the icon loses the welcome forever, and no story owns a one-shot welcome-seen state or a test that the daemon never writes config unprompted.
Strongest counter-argument: on Windows the portal path never runs and the daemon is read-mostly, so in practice the gate holds. Rebuttal: "in practice" is not an AC; S-087 ships a headline onboarding feature with a trigger condition three other stories can invalidate silently.

**M3. Severity: Major. Story: S-084 with S-076b, S-087, S-088.**
Gap: S-084 is a keystone with no stated landing order. Its D1 and D2 amend the AC5 and AC6 of S-076b, which is approved and pending: if S-076b lands first, its tests encode "a missing `drop_key` means Alt" and "Alt alone is accepted", precisely what S-084 supersedes on Windows, and must then be rewritten. S-087 AC3 and S-088 AC1 both consume the S-084 manager decision that does not exist until a spike runs. Three stories reference a decision line that gates one of them, and none names the prerequisite.
Strongest counter-argument: the manager decision line in S-084 AC1 is an explicit gate, so the tester will not fabricate the default. Rebuttal: the gate protects S-084 only; nothing stops S-076b from landing the superseded Alt semantics first, which is rework the sequence should prevent.

**M4. Severity: Major. Story: S-086 (out of scope) with S-087 (D1).**
Gap: the two stories disagree about the mechanism and the end state of the same click. S-086 says S-087 will replace Settings "by retargeting the shortcut"; S-087 D1 does it by making `settings` open the welcome window when no config exists. S-086 AC3's "ten clicks leave exactly one Settings window" contradicts the post-S-087 world where the same clicks leave one welcome window. A tester reading S-086 alone writes tests for the superseded end state.
Strongest counter-argument: D1 explicitly amends S-086 AC1 and keeps every other criterion, so the intent is clear enough. Rebuttal: D1 amends AC1 but not AC2's "Settings window" wording or the out-of-scope mechanism claim; two stories with different stories about one click is how tests get rewritten twice.

**M5. Severity: Major. Story: S-085 (AC2, AC3).**
Gap: renaming the MSI Product name and the Start shortcut touches the installer's upgrade identity, and `installer/windows/kawaro.wxs` states in a comment "Never change UpgradeCode: it is how a newer package replaces an older one." No AC in S-085 pins the UpgradeCode, and no test proves that an MSI built with the new bilingual Product name still upgrades and replaces an install of the old-named MSI. S-079 AC9 checks upgrades only in the owner-run live checklist.
Strongest counter-argument: the UpgradeCode attribute is already fixed in the wxs and the story does not touch it, so nothing changes. Rebuttal: the story changes the row adjacent to the invariant and its tests export Shortcut and Registry tables but never the upgrade relation; an AC asserting the unchanged UpgradeCode is one line and closes the hole.

**M6. Severity: Major. Story: S-083 (AC2) with S-078 (AC5).**
Gap: S-083 AC2 generates `SHA256SUMS` after signing and requires that the pipeline sign the MSI, which changes its bytes; S-078 AC5 pins the MSI's SHA-256 in the winget manifests. Neither story states that manifest generation must consume the post-signing artifact. If S-078's winget substory lands on the unsigned MSI, the published manifest hash dies at the first signed release and winget installs fail the hash check until new manifests are merged.
Strongest counter-argument: both pipelines regenerate per stable tag, so the drift heals at the next release. Rebuttal: a release with a broken catalog entry is exactly the "download stalls" experience this block exists to fix, and the ordering constraint is one sentence to state.

**M7. Severity: Major. Story: story set (S-083 to S-088).**
Gap: the six drafts are absent from `docs/PLAN.md`, which the plan itself positions as the authority that settles conflicts. The plan currently says Windows work starts at 0.6.0 and the next block is S-072 to S-082, so a fresh reader cannot tell which release ships the Windows trigger, the welcome window, or the signed installer. The set also answers a beta report, which suggests the 0.4.0 line, but no story names a release.
Strongest counter-argument: the plan is updated when the block lands, as happened for earlier blocks. Rebuttal: S-078, S-079 and S-075 all sit in the plan while approved; leaving six approved stories out of the conflict-settlement document is what breeds exactly the cross-story drift this set already contains (M3, M6).

**M8. Severity: Major. Story: S-087 (AC2, AC3) and S-088 (AC1, AC2).**
Gap: the demo flow, captions and walkthrough all bake in "holding the drag key", while S-084's leading candidate is the right-button drag, for which there is no key to hold. If the measurement picks the right-button drag or the tray hover drop, the shipped captions and the numbered walkthrough misdescribe the interaction the user must perform, and the ACs are written so that they must be satisfied regardless of the outcome.
Strongest counter-argument: M4 may still pick a Win chord and the wording is a placeholder for "the measured trigger". Rebuttal: the ACs say the captions name the effective key and the steps say "hold the drag key"; those are testable strings, not placeholders. The criteria must be written so any measured outcome satisfies them.

**M9. Severity: Major. Story: S-087 (AC2, Files).**
Gap: the welcome embeds a demo frame set and a sample image through `crates/ui/build.rs` with no size budget, while the light base install is a non-negotiable. Only the site's sample (S-088 AC3) has a cap. Pre-rendered frames of a whole drag flow are an easy path to a multi-megabyte binary, and no AC measures the installed size delta.
Strongest counter-argument: frames are still far lighter than a video decoder and the set is small by construction. Rebuttal: "small by construction" is an assumption, not a gate; a one-line cap (for example, the whole welcome resources under 1 MB) turns the product law into a test.

### Nits

**N1. Severity: Nit. Story: S-083 (Files, AC1).**
Gap: `scripts/windows/sign-release.sh` would be the third signing script beside `sign-local.sh` and `sign-update-manifest.sh`. The local script already signs a release with a certificate from `~/.config/kawaro-signing/windows/`; parameterizing it with a certificate source covers both cases and keeps one signing code path to audit.
Strongest counter-argument: CI signing has different inputs and failure modes and deserves its own script. Rebuttal: one script with a certificate argument and one verify step is simpler to keep honest than a family of scripts that can drift in timestamp and verification behavior.

**N2. Severity: Nit. Story: S-087 (Files) with S-088 (AC3).**
Gap: two sample images with separate provenance (the in-app fixture in `tests/fixtures/`, the site asset in `site/assets/`) answer the same "try it on an image" prompt. One source asset with two deployment copies avoids the day the in-app sample and the downloaded sample teach different things.
Strongest counter-argument: the two contexts need different sizes and formats. Rebuttal: generate one from the other in the build, which is what S-088 already claims for its asset ("generated from brand assets").

**N3. Severity: Nit. Story: S-087 (AC2, out of scope).**
Gap: the owner's report asked for a demo video; the app ships pre-rendered frames and gives the film only to the download page (S-088). The tradeoff is defensible under the light-install law and is documented in the out-of-scope list, but the divergence from the literal ask carries no recorded owner sign-off, unlike other owner decisions in these stories.
Strongest counter-argument: Slint's software renderer has no video decoder, so frames are the only compliant option and the owner will not care about the form. Rebuttal: likely true; recording one manager decision line saying the frames replace the video costs nothing and prevents a "where is the video" rework.
