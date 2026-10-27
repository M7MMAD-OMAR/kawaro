# Winget new-package PR: what Microsoft requires before merge, and what happens after

Research date: 2026-10-27. All claims below are tied to a source URL or a file path in
`microsoft/winget-pkgs`, `microsoft/winget-cli`, or Microsoft Learn. Where no rule is
documented, that is stated explicitly.

Subject PR: <https://github.com/microsoft/winget-pkgs/pull/462805>
("New package: Sbarah.Kawaro version 0.4.0"), labels `New-Package`, `Azure-Pipeline-Passed`,
`Validation-Completed`, `mergeStateStatus=BLOCKED`, `reviewDecision=REVIEW_REQUIRED`.

As of 2026-10-27 the pull request is no longer a draft, it carries an Arabic locale manifest next to
the English one, its body records the local `winget validate` and `winget install` results, and a
comment on it carries the publisher-controlled evidence for the domain check. All twelve automated
checks pass. The only gate left is one approving review.

---

## 0. The one-line answer

The 12 automated checks are **not** the gate. The gate is **one approving human review** from
someone with Write/Admin permission on `microsoft/winget-pkgs`. The repo's own ruleset
(`Default branch security`) sets `required_approving_review_count: 1`, and the auto-merge policy
only *enables* auto-merge when `Validation-Completed` is present: auto-merge then waits for the
required approval.

### Where the approval is actually given

Not on the pull request page of the contributor, and not by the author. The author cannot approve
their own pull request, and cannot request a review either: the API refuses with `404 Not Found`,
because requesting a reviewer needs write access to the target repository. The approval comes from
somebody else entirely:

1. A moderator opens `microsoft/winget-pkgs`, finds the pull request, and writes an **Approve**
   review. That review is what satisfies `required_approving_review_count: 1`.
2. The `moderatorTriggers.yml` policy turns that review into the `Moderator-Approved` label.
3. The auto-merge policy, already armed by `Validation-Completed`, squashes the pull request into
   `master` and the publish pipeline runs. The package then reaches the community source within
   about an hour.

The two red markers in the pull request sidebar, "Review required" and "Merging is blocked", are
this single gate shown twice. They are not a failure and not something the author can clear.

Who reviews: the moderators are the administrators and community moderators listed in
`doc/Moderation.md`. In the 30 days measured on 2026-10-27, every one of the last 40 merged pull
requests was approved by the same person, `stephengillie`. For an infrequent contributor the
documented route is a GitHub @mention: the guide says to check the recently closed pull requests
labelled `Moderator-Approved` to see who is active, to mention **at most two** moderators at a
time, and to say plainly what is being asked for. A `Policy-*`, `Validation-Domain` or
`Validation-Executable-Error` label needs an administrator, and a community moderator cannot
advance that.

The wait is real: the median of the last 100 merged `New-Package` pull requests was 563 hours
(about 23 days) from creation to merge, and only 8 of the 100 merged within a day. The queue of
`New-Package` pull requests with `Validation-Completed` and no moderator decision held 2638 entries
on 2026-10-27, and this one was the 977th from the oldest end.

---

## 1. The `New-Package` label flow

### What the label means
- Label description (live, via `gh api repos/microsoft/winget-pkgs/labels`):
  `New-Package :: The pull request adds the first version of a package to the repository.`
  (sibling label `New-Manifest :: The pull request adds new manifest content to the repository.`)
- Source: <https://github.com/microsoft/winget-pkgs/labels>
- The label itself is a **routing tag, not a blocker**. It is what puts a PR in the
  "new packages ready for moderator review" queue, which is reviewed separately from version bumps:
  <https://github.com/microsoft/winget-pkgs/blob/master/doc/Moderation.md> (section
  "Before You Request Assistance").

### What Microsoft does with a New-Package PR (the documented pipeline)
Source for all 10 steps: <https://github.com/microsoft/winget-pkgs/blob/master/doc/Validation.md>

| Step | Name | What it does |
|---|---|---|
| 01 | Pull Request Validation | exactly one package version; **only manifest files**; no merge conflicts |
| 02 | Manifest Validation | YAML/schema conformance, required fields, multi-file only, path/filename == `PackageIdentifier`/`PackageVersion` (case-sensitive) |
| 03 | URLs Validation | URLs resolve, HTTPS, no SmartScreen-malicious/reputation flag |
| 04 | URL Domain Validation | installer domain is the publisher's official domain; no shortener/mirror/redirect |
| 05 | Manifest Policy Validation | metadata vs. the 12 content policies; a hit does **not** block, it triggers manual review (`Policy-Test-2.x`) |
| 06 | Catalog Content Verification | simulates the merge: `DisplayVersion` range overlap, dependency existence, removal safety |
| 07 | Installers Scan | SHA256 match, multi-AV static scan, **PUA detection → cannot be accepted** |
| 08 | Installation Validation | silent install as a **standard (non-elevated) user**, correct `InstallerType`, elevation, executable discovery, Defender post-install scan |
| 09 | Installer Metadata Validation | MSIX metadata, icon extraction, `InstallationMetadata` consistency (skipped if 08 failed) |
| 10 | Validation Completed | summary; on success applies `Validation-Completed` |

### The merge gate (the actual requirement)
- `.github/policies/automergeTriggers.yml`: enables auto-merge (squash) **when
  `Validation-Completed` is present**:
  <https://github.com/microsoft/winget-pkgs/blob/master/.github/policies/automergeTriggers.yml>
- `.github/policies/labelAdded.validationCompleted.yml`: when `Validation-Completed` is added and
  `Moderator-Approved` / `Validation-Metadata` / `Publisher-Verified` are **not** present, the bot
  posts: *"The check-in policies require a moderator to approve PRs from the community."*
  <https://github.com/microsoft/winget-pkgs/blob/master/.github/policies/labelAdded.validationCompleted.yml>
- `.github/policies/moderatorTriggers.yml`: an approving `Pull_Request_Review` from a moderator /
  Write-permission user adds the `Moderator-Approved` label:
  <https://github.com/microsoft/winget-pkgs/blob/master/.github/policies/moderatorTriggers.yml>
- Branch ruleset `Default branch security` (id 4385804), live via API:
  `required_approving_review_count: 1`, `require_last_push_approval: true`,
  `dismiss_stale_reviews_on_push: true`, `require_extra_approval_for_unattributed_changes: true`.
  The org ruleset `microsoft-production-ruleset` (id 5351760) additionally sets
  `require_code_owner_review: true`.
  API: <https://api.github.com/repos/microsoft/winget-pkgs/rulesets/4385804>

**Consequence for Kawaro:** `Validation-Completed` is already applied, so auto-merge is armed, but
the PR is `REVIEW_REQUIRED` / `BLOCKED` because no approving review exists. It is also a **draft**,
so it must be marked *Ready for review* before auto-merge can fire.

### How long does human review take?
There is **no documented SLA**. Sources, in order of authority:

- `doc/Moderation.md`: *"There is no definitive time that a PR spends waiting for review - it really
  depends on when it was submitted, who is available for reviews, and how complex the reviews are.
  ... The longest I've seen a PR to review is 3 days (due to a weekend + a holiday), but I would say
  that 90% of PRs are reviewed within 24 hours."* (Thierry Lelégard, moderator)
  <https://github.com/microsoft/winget-pkgs/discussions/192170>
- `doc/FAQ.md` ("How long do packages take to be published?"): the three stages are PR approval →
  moderator review → publishing pipeline. *"Typically, changes are published within one hour after
  the pull request is merged."* <https://github.com/microsoft/winget-pkgs/blob/master/doc/FAQ.md>
- `doc/Moderation.md`: moderators *"generally prioritize the oldest open pull requests that have
  completed validation"*, and *"New packages generally require more scrutiny because moderators must
  establish package identity, installer provenance, metadata accuracy, and policy compliance without
  prior manifest history."*

**Measured, not documented:** sampling the last 100 merged `New-Package` PRs in the repo
(`gh search prs --label New-Package --merged-at ">=2026-06-01"`, created→closed):
median **563 h (≈23 days)**, p25 73 h, p75 658 h, 8/100 ≤ 24 h, 33/100 ≤ 7 days.
Only ~8 % of the fast tail are packages from the maintainer's own org; community New-Package PRs
cluster at the multi-week end. A moderator on
<https://github.com/microsoft/winget-pkgs/discussions/333858> confirms: *"PRs for new packages
average well over a week before they get merged… and that's when counting from after the PRs first
get their `Validation-Completed` tags."*

### What the human reviewer checks that the pipeline cannot
From `doc/Moderation.md` ("Reviewing Pull Requests"): *"Moderators should review PRs to ensure the
metadata is accurate and that the Windows Package Manager will behave predictably with the given
manifest. This includes checking the metadata and testing the installation of packages."*

From `doc/FAQ.md` ("How do I know the packages in the Community Repository are safe?"): manual
review is where moderators *"check for the installation of any potentially unwanted applications,
applications which change settings unexpectedly, and to ensure the installation truly works as
expected."*

From `CONTRIBUTING.md` ("Testing"): manifests must be checked so that
- the app installs **unattended**,
- the app **version** matches `PackageVersion` (or `AppsAndFeaturesEntries` is present),
- the app **publisher** matches the defaultLocale `Publisher` (or `AppsAndFeaturesEntries` is present),
- the app **name** matches the defaultLocale `PackageName` (or `AppsAndFeaturesEntries` is present).

Observed moderator behaviour on real New-Package PRs (evidence, not policy): moderators re-run the
install by hand and post the raw output, e.g. `stephengillie` on PR 453052 posted "Automatic
Validation ended with: … Executable … returned exit code: -1073741515" and then "Manual Validation
ended with: htop-win 0.2.14"; on PR 449622 a moderator quoted the policy text that the automated
policy test tripped, verbatim, and asked the author to reword. Both PRs were then approved.
<https://github.com/microsoft/winget-pkgs/pull/453052>, <https://github.com/microsoft/winget-pkgs/pull/449622>

### Common rejection reasons
| Reason | Where it shows up |
|---|---|
| Wrong manifest folder path / filename casing vs `PackageIdentifier` | `Manifest-Path-Error` (step 02) |
| Two package versions or non-manifest files in one PR | `PullRequest-Error` (step 01) |
| Installer URL from a mirror/aggregator/shortener/redirect | `Validation-Domain`, `Validation-Unapproved-URL`, `Validation-Indirect-URL` (step 04): administrator review |
| Silent install not silent / wrong `InstallerType` / missing elevation | `Validation-Unattended-Failed` (step 08) |
| Missing/incorrect `InstallerSha256` | `Error-Hash-Mismatch` (step 07) |
| PUA / malware detection | `Binary-Validation-Error`: cannot be waived per `doc/ValidationFailureGuide.md` |
| `DisplayVersion` range overlap with an adjacent version | `Manifest-AppsAndFeaturesVersion-Error` (step 06) |
| Metadata violating a content policy | `Policy-Test-2.1`…`2.12`: administrator review |
| App name / publisher / version not matching the installed app | moderator rejection; use `AppsAndFeaturesEntries` |
| Stale PR | `Needs-Author-Feedback` → 5 days no activity → `No-Recent-Activity` → 3 more days → **auto-closed** (`.github/policies/scheduledSearch.noRecentActivity.yml`) |

`doc/ValidationFailureGuide.md` has a table mapping each label to category and
"Author Action / Review" (✅ author fixes / 🔒 administrator review / ⚠️ maybe):
<https://github.com/microsoft/winget-pkgs/blob/master/doc/ValidationFailureGuide.md>

---

## 2. PackageIdentifier rules: is the publisher prefix required to equal `Publisher`?

### The two rules that actually exist
1. **Folder path ↔ `PackageIdentifier`**, not ↔ `Publisher`.
   `doc/manifest/schema/1.12.0/README.md`: *"The publisher and application folders MUST match the
   values used to define the Id."* and *"Publisher folder is the name of the company that publishes
   the tool."* <https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/README.md>
   `doc/Authoring.md`: *"The first directory is the lowercase first letter of the 'PackageIdentifier.'
   Subsequent directories match each segment of the 'PackageIdentifier' (case-sensitive)."*
   <https://github.com/microsoft/winget-pkgs/blob/master/doc/Authoring.md>
   → For Kawaro: `manifests/s/Sbarah/Kawaro/0.4.0/` with `PackageIdentifier: Sbarah.Kawaro` is correct.
   The word "publisher" in the folder rule means **the publisher segment of the identifier**, i.e.
   `Sbarah`, never the `Publisher:` field.

2. **`Publisher` field ↔ Add/Remove Programs**, not ↔ identifier prefix.
   `doc/manifest/schema/1.12.0/defaultLocale.md`: *"This key represents the name of the publisher for
   a given package. **This field is intended to allow the full publisher's or ISV's name to be
   displayed as they wish.** … The best practice is to ensure this matches the entry for the package
   when it has been installed. This should be the value of the `Publisher` subkey for the package in
   the Windows registry. The impact is associated with `winget upgrade` and `winget list`."*
   <https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/defaultLocale.md>
   Microsoft Learn: *"The Publisher should match the entry made in Add / Remove Programs to help the
   correlation with manifests to support export, and upgrade."*
   <https://learn.microsoft.com/en-us/windows/package-manager/package/manifest>

### Is there an official rule that the identifier prefix == `Publisher`? **No.**
- The JSON schema constrains only the *shape* of `PackageIdentifier`, never its content:
  `"pattern": "^[^\\.\\s\\\\/:\\*\\?\"<>\\|\\x01-\\x1f]{1,32}(\\.[^\\.\\s\\\\/:\\*\\?\"<>\\|\\x01-\\x1f]{1,32}){1,7}$"`,
  `maxLength: 128`.
  <https://github.com/microsoft/winget-cli/blob/master/schemas/JSON/manifests/v1.12.0/manifest.version.1.12.0.json>
- The winget-cli runtime validation is **path-safety only**: it never compares the identifier to the
  `Publisher` field:
  `src/AppInstallerCommonCore/Manifest/ManifestValidation.cpp` →
  `ValidatePackageIdentifier()` → `ValidateFieldValueUsedInPathConstruction("PackageIdentifier", value, /* disallowWhitespace */ true)`,
  which only rejects whitespace/control chars, `\/:*?"<>|`, path traversal, over-length, and reserved
  device names. <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/Manifest/ManifestValidation.cpp>
- **No code anywhere in winget-cli splits `PackageIdentifier` into publisher/package segments.**
  Searched `src/` for any `Id`-splitting or `Id`-vs-`Publisher` comparison: none. The identifier is
  treated as an opaque string for path construction and as a repository key.
  The only consumer of `Publisher` is the ARP-correlation path
  (`Manifest::GetNameAndPublisherPairs()` in `src/AppInstallerCommonCore/Manifest/Manifest.cpp`,
  used by `src/AppInstallerRepositoryCore/ARPCorrelation.cpp` and `ARPCorrelationAlgorithms.cpp`),
  which compares `Publisher` against the **installed ARP entry**, never against the identifier.

### What real packages do (they differ routinely)
| PackageIdentifier | `Publisher` field | same? |
|---|---|---|
| `Microsoft.WindowsTerminal` | `Microsoft Corporation` | **no** |
| `Microsoft.PowerToys` | `Microsoft Corporation` | **no** |
| `7zip.7zip` | `Igor Pavlov` | **no** |
| `Git.Git` | `The Git Development Community` | **no** |
| `yt-dlp.yt-dlp` | `yt-dlp` | yes |
| `GnuWin32.Make` | `GnuWin32` | yes |

(All fetched live from `manifests/.../*.locale.en-US.yaml` on `microsoft/winget-pkgs@master`.)

### The documented identifier rule that *does* bite
`doc/manifest/schema/1.12.0/README.md`, Best Practices: *"Avoid creating multiple publisher folders.
For example, do not create 'Contoso Ltd.' if there is already a 'Contoso' folder."*
So the identifier prefix must be **consistent with itself across the package**, and the identifier
must be globally unique: *"The package identifier must be unique. You cannot have multiple
submissions with the same package identifier."*

**Verdict for Kawaro:** `Sbarah.Kawaro` + `Publisher: Kawaro` is legal and matches the dominant real-
world pattern (the identifier namespace segment is the *maintainer's* namespace, the `Publisher`
field is the *product's* ARP publisher). The risk is not the schema, it is that the ARP entry
(`Manufacturer`/`ProductName` = `Kawaro`) must match `Publisher: Kawaro`, which it does.
The one thing to double check by hand is that the moderator sees a *coherent* story: an identifier
prefix `Sbarah` with publisher `Kawaro` is normal, but the **domain** check (step 04) compares the
installer URL's domain against the expected domain *for the publisher*, and `PublisherUrl` is
`https://kawaro.sbarah.com/` while the installer is on `github.com/M7MMAD-OMAR/kawaro`. That is a
domain-domain alignment judgment a human makes, see section 7.

---

## 3. Self-signed / test-signed installers

### Is there a documented rule that the installer must carry a trusted code-signing certificate? **No.**
- The winget-pkgs policy set (`doc/Policies.md`) requires: supported installer type, unattended
  install, no PUA/malware, official-source URLs, multi-file manifests, URL policy.
  It says **nothing** about code signing for MSI/EXE.
  <https://github.com/microsoft/winget-pkgs/blob/master/doc/Policies.md>
- The Learn policies (2.1 to 2.12) are all about **content/metadata**, not signatures. The only
  signature-related sentence is in §2.3.1, which puts "certificate acquisition" on the **publisher**,
  not on the manifest: *"You are solely responsible for all Product safety testing, certificate
  acquisition, and implementation of any appropriate feature safeguards."*
  <https://learn.microsoft.com/en-us/windows/package-manager/package/windows-package-manager-policies>
- The **only** documented "must be signed" rule in the schema docs is **MSIX-specific**:
  `doc/manifest/schema/1.12.0/installer.md`: *"IMPORTANT: MSIX installers must be signed to be
  included in the Microsoft community package repository."* (for `SignatureSha256`).
  MSI/EXE have no equivalent requirement.
  <https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/installer.md>

### What the pipeline actually does with the binary (steps 07 to 09)
- SHA256 match → multi-AV static scan → **PUA detection** → dynamic install as standard user →
  Defender post-install scan. **None of these is a code-signature check.**
  `doc/Validation.md` steps 07 to 09. A self-signed signature is invisible to all of them; a
  *malicious* binary is not.
- `URL-Validation-Error` is documented as *"Microsoft Defender SmartScreen triggered during automatic
  validation"*: that is **URL reputation**, not the installer's Authenticode signature.

### Is there a warning/trust mechanism?
- **`InstallerSha256`**: integrity only. It pins the exact bytes; it says nothing about who signed
  them. The pipeline re-downloads and re-hashes the file (step 07), so a self-signed MSI that is
  byte-identical passes.
- **`InstallerUrl` domain**: this *is* the de-facto trust anchor for the community repo. `doc/Policies.md`
  §Manifest URLs: *"any URLs for installers need to be discoverable on the publishers website"*, and
  step 04 checks the domain is the publisher's official domain. An MSI hosted on the publisher's own
  GitHub release page is on-policy.
- **`Publisher` trust**: there are labels `Publisher-Verified` (verified developer),
  `MSFT-Verified`, `Partner-Verified`, and `Author-Not-Verified` (*"The author's relationship to the
  package publisher has not been verified"*). These are about **the PR author's relationship to the
  publisher**, not the binary's signature. `Publisher-Verified` is what lets
  `.github/policies/verifiedDeveloper.yml` auto-approve without moderator review:
  *"The author is a trusted publisher registered with the Windows Package Manager service. No
  additional moderator review is required."* Kawaro's author is not verified, so normal moderator
  review applies.
- **No "Catalog" field** exists in the manifest schema for this purpose. `Catalog` in winget means
  the MSIX catalog API (`APPINSTALLER_CLI_ERROR_DISPLAYCATALOG_API_FAILED`), unrelated to signing.

### Does winget itself verify the installer's signature at install time? **No.**
- `GetAuthenticodeSubject()` exists in `src/AppInstallerSharedLib/Certificates.cpp` but is only
  used by `DebugCommand` and the DSC `ConfigurationFlow`, and it explicitly *"Only returns a value
  when WinVerifyTrust confirms the signature is valid and trusted"*: i.e. it reports "(unsigned)"
  rather than blocking.
  <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerSharedLib/Certificates.cpp>
- `WinVerifyTrust` in the install path exists only for **MSIX** (`src/AppInstallerCommonCore/MsixInfo.cpp`).
- What *can* bite is **Mark of the Web**: `src/AppInstallerCommonCore/Downloader.cpp`
  (`ApplyMotwIfApplicable` / `ApplyMotwUsingIAttachmentExecuteIfApplicable`) writes a
  `Zone.Identifier` ADS onto the downloaded installer. An unsigned or self-signed binary then hits
  SmartScreen on the user's machine (the "Windows protected your PC" prompt), even though winget
  itself does not block it. This is a **user-experience** consequence of not having a trusted
  signature, not a merge blocker.

### Real-world precedent
`RustDesk` was rejected from the community repo for **ESRP/PUA** detections, and the resolution advice
was to sign with a valid code-signing certificate: i.e. signing was the fix for a *PUA reputation*
problem, not for a missing-signature rule. See
<https://github.com/rustdesk/rustdesk/discussions/13025> and PR 293118.

**Verdict for Kawaro:** a self-signed MSI is **not** a documented merge blocker. It is however a
**reputation risk** in step 07 (PUA/SmartScreen heuristics weight unsigned, rarely-downloaded
binaries negatively) and a **user-facing** SmartScreen prompt. If the PR is rejected with a PUA or
SmartScreen label, `doc/ValidationFailureGuide.md` says that label **cannot be waived** and the
documented remedy is to submit the binary to Microsoft Defender for analysis:
<https://www.microsoft.com/wdsi/filesubmission>. Getting a real (free, e.g. SignPath) certificate
before merge removes that risk entirely.

---

## 4. The PR-body checklist boxes

### Template text (live)
`https://github.com/microsoft/winget-pkgs/blob/master/.github/PULL_REQUEST_TEMPLATE.md`

```
## ✅ Checklist
<!-- Place an "x" between the brackets to check an item. e.g: [x] -->

- [ ] Signed the [Contributor License Agreement](https://cla.opensource.microsoft.com)
- [ ] Linked to an issue (if applicable)

## 📦 Manifest Checklist

- [ ] Checked that there aren't other open pull requests for the same manifest update/change
- [ ] This PR only modifies one (1) manifest
- [ ] Validated manifest locally with `winget validate --manifest <path>`
- [ ] Tested manifest locally with `winget install --manifest <path>`
- [ ] Manifest conforms to the [1.12 schema](https://github.com/microsoft/winget-pkgs/tree/master/doc/manifest/schema/1.12.0)
```

### Who ticks them
**The submitter ticks them.** The template comment says so explicitly: *"Place an 'x' between the
brackets to check an item."* They are an attestation by the author, not a form Microsoft fills in.

### Does Microsoft / the bot verify them?
- **The CLA box is not what enforces the CLA.** The CLA is enforced by a *separate automated
  process*: `doc/Validation.md` step 01 note: *"The Contributor License Agreement (CLA) is checked by
  a separate automated process, not by this validation step. If the CLA has not been signed, a
  `Needs-CLA` label is applied and the PR cannot be merged until it is resolved."*
  The bot's reply template is in `.github/policies/labelAdded.needsCLA.yml`.
  On the Kawaro PR the bot already posted that reply, and the author replied
  `@microsoft-github-policy-service agree`: that is how the CLA is signed.
  Note: the ruleset's `require_last_push_approval: true` means any later push re-arms the review.
- **The three manifest-checklist boxes are not machine-checked.** No workflow in
  `.github/workflows/` parses the PR body for `- [x]`. Grep of all workflow files for
  `checklist` / `- [x]` / PR-body parsing returns nothing. What *is* machine-checked is the
  *substance* of each box, by the pipeline:
  - "one manifest / one version" → step 01 (`PullRequest-Error`).
  - "no other open PR" → moderator judgement; there is a `Possible-Duplicate` label and the
    `wingetbot-pr-triage` workflow has a "Possible duplicate" class.
  - "conforms to the 1.12 schema" → step 02 (`Manifest-Validation-Error`, `Manifest-Path-Error`,
    `Manifest-Version-Deprecated`).
  - "validated / installed locally" → **not** machine-checkable; it is the author's attestation.
- **A reviewer may read them.** They are part of the PR body a moderator reads; the body is
  untrusted input to the bots, so ticking them does not move any gate, but leaving them unticked
  while the PR sits in the moderator queue reads as an unfinished submission.

**Verdict for Kawaro:** the three unticked boxes (`winget validate`, `winget install`, schema
conformance) are the author's own attestation and are **not** formally required for merge, but
they are the three things the template asks for, and the PR body itself says *"This is a draft
pending local validation and unattended installation testing."* Tick them only after actually
running them. The CLA box should be ticked once the CLA is signed (the bot's `agree` comment
already happened on this PR, but the `Needs-CLA` label was applied at 11:22 and the author
responded at 11:25).

---

## 5. After merge

### Publish pipeline
- `doc/FAQ.md`: *"Typically, changes are published within one hour after the pull request is merged.
  If you don't see the changes published, check the WinGetSvc-Publish Pipeline … for errors."*
  <https://github.com/microsoft/winget-pkgs/blob/master/doc/FAQ.md>
- `doc/Validation.md` §"After Validation: What Happens Next?": *"After merge, the package passes
  through the publishing pipeline. Once successfully published, you will see a comment and label on
  the PR. **Changes typically appear in the WinGet source within one hour.**"*
- The bot posts a comment on the PR; `.github/policies/wingetbotTriggers.yml` adds the
  `Publish-Pipeline-Succeeded` label when wingetbot reports *"Publish pipeline succeeded"*.
- A moderator can re-run validation with `@wingetbot run` (`doc/Moderation.md`).

### Exact commands to verify a published version
```powershell
winget source update --name winget     # refresh the default community source
winget search Sbarah.Kawaro            # is it in the catalog at all?
winget show Sbarah.Kawaro --versions   # which versions are published?
winget show --id Sbarah.Kawaro --exact --version 0.4.0
winget install --id Sbarah.Kawaro --version 0.4.0 --exact
```
- `winget source update --name winget`: <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/source.md>
  (*"The **update** sub-command combined with the **--name** option can direct and update to an
  individual source. For example: `winget source update --name winget`"*)
- `winget show ... --versions`: <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/show.md>
- `winget install --id ... --version ...`: <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/install.md>
  (`winget install --id Microsoft.PowerToys --version 0.15.2`)
- Note `winget search`/`show`/`install` read the **REST source** (the published index), not the GitHub
  repo. GitHub `master` is not the catalog.

### Does merge require a version bump / a second version before `upgrade` works? **No, but `upgrade` needs a newer version to exist.**
- `winget upgrade` only offers a package whose **available** version is greater than the installed
  one. With only 0.4.0 in the catalog, `winget upgrade` will list nothing for Kawaro. That is
  expected, not a defect. A second manifest (0.4.1) is what makes `upgrade` meaningful.
- The real trap is **version mapping**, not the version count. `doc/FAQ.md`: *"The Package Version,
  sometimes referred to as Build Version, Actual Version, or DisplayVersion is the version which is
  written to the ARP entry. This value is what the Windows Package Manager uses to determine which
  version of an application is currently installed."* and *"Some applications, packages do not report
  a version resulting in an upgrade loop or other unwanted behavior."*
- `doc/Authoring.md` §"When is AppsAndFeaturesEntries needed?": `DisplayVersion` in
  `AppsAndFeaturesEntries` is **required** when (a) the installer does not write a `DisplayVersion`,
  (b) `PackageVersion` ever differed from the installer's `DisplayVersion`, or (c) the `DisplayVersion`
  is not sortable. Omitting it in those cases causes *"an upgrade loop where the `Available` version
  shown when running `winget upgrade` is lower than the `PackageVersion` of the latest manifest."*
  <https://github.com/microsoft/winget-pkgs/blob/master/doc/Authoring.md>
- Step 06 (Catalog Content Verification) checks exactly this on merge: *"`DisplayVersion` range
  overlaps, the most common failure."*
- Practical rule for Kawaro: if the MSI writes `DisplayVersion` = `0.4.0` (matching `PackageVersion`),
  no `AppsAndFeaturesEntries` is needed now. **If a later release writes a different `DisplayVersion`,
  every manifest in the package must carry `AppsAndFeaturesEntries.DisplayVersion`**, including 0.4.0,
  or step 06 will reject the next version with `Manifest-AppsAndFeaturesVersion-Error`.
- The mixed set is the silent failure: some manifests carry the field and others do not, the catalog
  cannot map the installed version to an available one, and the user sees an upgrade loop where the
  available version is lower than the latest `PackageVersion`. `scripts/release/check-winget-versions.sh`
  refuses a mixed set, and `tests/release/s078_winget_versions.sh` covers it against a real MSI built
  with wixl.
- `winget list` / `winget upgrade` correlate the installed app to the manifest by
  **PackageName + Publisher** (fuzzy, `WordsEditDistanceMatchConfidenceAlgorithm` in
  `src/AppInstallerRepositoryCore/ARPCorrelationAlgorithms.cpp`) and by **ProductCode /
  UpgradeCode** (exact). Declaring `ProductCode: {39472B1E-48C0-47A9-9AE8-20ED5435C0CA}` gives the
  exact path, which is the reliable one.
  <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerRepositoryCore/ARPCorrelation.cpp>

---

## 6. Per-user MSI (`Scope: user`), custom ProductCode, and `InstallerSwitches`

### Does the validation require `InstallerSwitches.Silent: /qn /norestart` for `InstallerType: msi`? **No, the default is enough.**
- winget-cli ships **built-in default switches** for MSI, applied when the manifest does not specify
  them. `src/AppInstallerCommonCore/Manifest/ManifestCommon.cpp` → `GetDefaultKnownSwitches()`:
  ```cpp
  case InstallerTypeEnum::Burn:
  case InstallerTypeEnum::Wix:
  case InstallerTypeEnum::Msi:
      return {
          {InstallerSwitchType::Silent,           "/quiet /norestart"},
          {InstallerSwitchType::SilentWithProgress, "/passive /norestart"},
          {InstallerSwitchType::Log,              "/log \"<logpath>\""},
          {InstallerSwitchType::InstallLocation,  "TARGETDIR=\"<installpath>\""}
      };
  ```
  <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/Manifest/ManifestCommon.cpp>
- These defaults are **injected into the manifest at parse time** if absent:
  `src/AppInstallerCommonCore/Manifest/ManifestYamlPopulator.cpp` → *"Populate installer default
  switches if not exists"*.
- `/quiet` is normalised to `/qn` at MSI-argument parse time:
  `src/AppInstallerCommonCore/MsiExecArguments.cpp` → `{ "quiet"sv, "/qn"sv }`.
- Runtime precedence (`GetInstallerArgsTemplate` in
  `src/AppInstallerCLICore/Workflows/ShellExecuteInstallerHandler.cpp`):
  `SilentWithProgress` is the default experience; `Silent` is used when `--silent` is passed;
  `Interactive` when `--interactive` is passed; `--override` replaces everything.
- **So `InstallerSwitches` is optional for MSI.** Adding `Silent: /qn /norestart` is harmless but
  redundant. (Note the built-in default is `/quiet /norestart`, i.e. `msiexec` long-form equivalent.)
- The *policy* requirement is not about writing the switch, it is that the installer **must support**
  silent **and** silent-with-progress: `doc/Policies.md` §Installer Behaviors (*"Packages submitted to
  the community repository must install without requiring user interaction. The default installation
  mechanism in WinGet is **'silent with progress''"*) and `doc/manifest/schema/1.12.0/installer.md`
  §`InstallerSwitches` (*"The Microsoft community repository currently requires support for silent
  and silent with progress installation. **Many custom .exe installers will require the proper
  switches** to meet this requirement."*). MSI gets this for free from msiexec.

### Extra requirements for a per-user MSI (`Scope: user`)
1. **`Scope: user` must match reality.** The pipeline installs as a **standard (non-elevated) user**
   (`doc/Validation.md` step 08) and the ARP entry is then expected under `HKCU`. winget reads
   `HKEY_CURRENT_USER` for `ScopeEnum::User` (`src/AppInstallerRepositoryCore/Microsoft/ARPHelper.cpp`).
   If the MSI actually installs per-machine, `Scope: user` is wrong.
2. **Elevation.** For `Scope: user` no elevation is needed and none should be requested. If the MSI
   writes to `Program Files` / `HKLM`, `ElevationRequirement: elevationRequired` becomes mandatory
   (step 08: *"if the installer requires administrator privileges … the manifest must declare
   `ElevationRequirement: elevationRequired`"*). Conversely, `elevationProhibited` blocks an install
   launched from an elevated shell (`InstallFlow.cpp`, `APPINSTALLER_CLI_ERROR_INSTALLER_PROHIBITS_ELEVATION`).
   For a genuine per-user MSI, leave `ElevationRequirement` unset.
   `doc/manifest/schema/1.12.0/installer.md` §ElevationRequirement:
   <https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/installer.md>
3. **ProductCode.** Optional in the schema (`required` for the installer file is only
   `PackageIdentifier, PackageVersion, Installers, ManifestType, ManifestVersion`) but **strongly
   recommended for MSI**, because it is the exact-match key for ARP correlation and for
   uninstall/upgrade. `DoesInstallerTypeUseProductCode()` returns true for Msi/Inno/Exe/Nullsoft/
   Wix/Burn/Portable. Format: a GUID in braces, e.g. `{39472B1E-48C0-47A9-9AE8-20ED5435C0CA}`:
   matches `ProductCodeValidator` (1..255 chars, nullable) in the REST source utils.
   If you also specify `AppsAndFeaturesEntries`, the `ProductCode` must be **duplicated inside it**:
   *"When AppsAndFeaturesEntries are specified, the ProductCode should be placed both within the
   installer and the AppsAndFeaturesEntries."*
4. **Uninstall must work.** Step 08 also runs an uninstall test (`Validation-Uninstall-Error`).
   For MSI, winget uninstalls via `msiexec /x <ProductCode>` with `/quiet /norestart`
   (`ShellExecuteInstallerHandler.cpp`, `GetMsiExecUninstallArgs`), so the ProductCode must be the
   real one or uninstall silently fails.
5. **`UpgradeCode`**: optional, but it is the key that lets msiexec perform an in-place upgrade
   across versions. If the MSI keeps a stable `UpgradeCode` across releases, declare it in
   `AppsAndFeaturesEntries.UpgradeCode` when you add `AppsAndFeaturesEntries`.
6. **Per-user MSI properties.** winget does **not** inject `ALLUSERS=2` / `MSIINSTALLPERUSER=1`;
   there is no such code in `src/`. If the MSI needs those, they must be baked into the MSI itself
   (or passed via `InstallerSwitches.Custom`, e.g. `ALLUSERS=2`): this is a property of the MSI
   package, not of the manifest.

#### Measured: does `wixl` bake `ALLUSERS` into a per-user MSI?

**No.** `wixl` (msitools 0.106) accepts `InstallScope="perUser"` and silently drops it: the built
MSI has no `ALLUSERS` and no `MSIINSTALLPERUSER` row. Measured on this machine with two builds of
the same `.wxs`:

| build | `.wxs` | `msiinfo export <msi> Property` |
|---|---|---|
| A | `<Package ... InstallScope="perUser" />` | no `ALLUSERS`, no `MSIINSTALLPERUSER` |
| B | A plus explicit `<Property Id="ALLUSERS" Value="2" />` and `<Property Id="MSIINSTALLPERUSER" Value="1" />` | `ALLUSERS=2`, `MSIINSTALLPERUSER=1` |

So the incantation that works with `wixl` is the explicit `<Property>` pair, **not**
`InstallScope="perUser"`. `strings $(which wixl)` finds the literal `ALLUSERS` in the binary, so
the compiler does know the name; it just does not map `InstallScope` onto it.

**What that means for the 0.4.0 MSI.** Without `ALLUSERS`, the install scope is decided by
msiexec's own default, and the registration lands in `HKLM` when the installer runs elevated
(SYSTEM or an administrator), which is what the local VM test showed: an install as SYSTEM put
the ARP entry in `HKLM\...\Uninstall\{39472B1E-...}`, and the files under the *installing* profile's
`LocalAppData`, not under `HKCU` of the target user. The manifest declares `Scope: user`, and
winget reads `HKEY_CURRENT_USER` for `ScopeEnum::User`
(`src/AppInstallerRepositoryCore/Microsoft/ARPHelper.cpp`), so an elevated install of this MSI is
invisible to `winget list` and to the uninstall path.

**Why the PR still passed.** Microsoft's step 08 *does* run the install as a standard
(non-elevated) user (`doc/Validation.md` step 08: *"runs it in an isolated environment as a
**standard (non-elevated) user**"*), and it passed on this PR. A standard user's install is
per-user by construction, so the `HKCU` path is what the pipeline exercised and what it approved.
The gap only shows up for an **elevated** install, which the pipeline does not test and which a
moderator may not test either.

**Recommendation.** Add the explicit pair to `installer/windows/kawaro.wxs` so the MSI is
per-user by construction and the `HKCU` registration is guaranteed for every user, elevated or
not. This is a change to the next MSI build (0.4.1 or later), not to the submitted 0.4.0
manifest, so it does not disturb the open pull request. The `Scope: user` declaration stays
correct either way; the change makes the installer match what the manifest already promises.

### Nothing special for a "custom" product code
A per-user MSI with a self-chosen ProductCode is normal; there is no documented requirement that
the ProductCode match a "known" vendor. It must, however, match the MSI's actual ProductCode, or
correlation/uninstall break.

---

## 7. Things specific to the Kawaro PR that the pipeline passed but a human may still flag

These are judgement calls the automation cannot make; each has a source.

1. **Domain ↔ publisher alignment (step 04).** `doc/Policies.md` §Manifest URLs: *"any URLs for
   installers need to be discoverable on the publishers website"*. The installer URL is
   `https://github.com/M7MMAD-OMAR/kawaro/releases/download/v0.4.0/kawaro-0.4.0-windows-x64.msi`,
   the GitHub org is `M7MMAD-OMAR`, and `PublisherUrl` is `https://kawaro.sbarah.com/`. The
   validation guide's fix hint is to show that the download is reachable from the publisher's site
   ("Including `PackageUrl` in your manifest and ensuring the `InstallerUrl` can be found by
   navigating the publisher's website from that URL helps speed up moderator reviews"), and
   `Validation-Domain` / `Validation-Unapproved-URL` require **administrator** review: a community
   moderator cannot waive them. `PackageUrl` is the field that makes this linkable.
2. **Publisher name vs. the `Publisher` field.** `doc/manifest/schema/1.12.0/defaultLocale.md`:
   the `Publisher` field should match the **installed** ARP entry, and Kawaro's MSI
   Manufacturer/ProductName are both `Kawaro`, so `Publisher: Kawaro` + `PackageName: Kawaro` is
   right. The identifier prefix `Sbarah` is a namespace, not a mismatch.
3. **Self-signed MSI.** Not a documented blocker (§3), but it is the kind of thing that draws a
   `Validation-SmartScreen` / PUA objection. A free trusted certificate before merge is the cheapest
   way to remove that risk.
4. **Draft status.** A draft PR cannot be auto-merged; it must be marked *Ready for review*.
5. **The `require_last_push_approval` rule.** Any push after a moderator approves invalidates that
   approval (`dismiss_stale_reviews_on_push: true` in the repo ruleset, `require_last_push_approval: true`
   in both). Push the final state, including the ticked checklist and any signature change, *before*
   asking for review.

---

## 8. Compact checklist

### BEFORE MERGE: REQUIRED
- [x] One package version, manifest files only, correct path `manifests/<l>/<Id-segments>/<version>/`
     , <https://github.com/microsoft/winget-pkgs/blob/master/doc/Validation.md> (step 01)
- [x] Multi-file manifest, `ManifestVersion: 1.12.0`, schema header on every file
     , <https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/README.md>
- [x] `InstallerSha256` matches the released file: step 07
- [x] HTTPS, official publisher domain, no redirect/shortener/mirror: steps 03 to 04
- [x] Silent **and** silent-with-progress install as standard user, step 08
- [x] No PUA/malware detection, step 07 (not waivable)
- [x] No `DisplayVersion` range overlap, step 06
- [x] CLA signed (bot comment, not the PR-body box), `doc/Validation.md` step 01 note
- [x] **One approving review from a Write-permission user / moderator**, branch ruleset 4385804
- [x] PR marked **Ready for review** (not draft), auto-merge only fires on non-drafts

### BEFORE MERGE, OPTIONAL / GOOD PRACTICE
- [ ] Tick the three manifest-checklist boxes in the PR body after actually running
      `winget validate --manifest <path>` and `winget install --manifest <path>`
     , <https://github.com/microsoft/winget-pkgs/blob/master/.github/PULL_REQUEST_TEMPLATE.md>
- [ ] Add `PackageUrl` pointing at the release page so the domain check is provable
- [ ] Use a trusted (e.g. SignPath) code-signing certificate instead of the self-signed test cert
- [ ] `ProductCode` correct (already present), needed for ARP correlation, uninstall and upgrade
- [ ] `UpgradeCode` in `AppsAndFeaturesEntries` when you add `AppsAndFeaturesEntries`
- [ ] Plan the `DisplayVersion` story now so the *next* version does not trip step 06

### AFTER MERGE
- [ ] `winget source update --name winget`: <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/source.md>
- [ ] `winget show --id Sbarah.Kawaro --versions`: <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/show.md>
- [ ] Expect publication within ~1 hour; the PR gets a bot comment + `Publish-Pipeline-Succeeded`
     , <https://github.com/microsoft/winget-pkgs/blob/master/doc/FAQ.md>
- [ ] `winget upgrade` stays empty until a **higher** version is published; not a defect
- [ ] If a later release changes `DisplayVersion`, add `AppsAndFeaturesEntries.DisplayVersion` to
      **every** version including 0.4.0, <https://github.com/microsoft/winget-pkgs/blob/master/doc/Authoring.md>

### NOT DOCUMENTED (explicitly)
- No published SLA for moderator review.
- No rule that the identifier's publisher prefix must equal the `Publisher` field.
- No rule that an MSI/EXE installer must carry a trusted code-signing certificate.
- No rule that merging 0.4.0 requires a second version before `winget upgrade` works.
- No rule that `InstallerSwitches` must be specified for `InstallerType: msi`.
- No machine check of the PR-body checklist boxes.

---

### Source index
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/Validation.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/ValidationFailureGuide.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/Moderation.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/Policies.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/Authoring.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/FAQ.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/FirstContribution.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/README.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/README.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/version.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/defaultLocale.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.12.0/installer.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/.github/PULL_REQUEST_TEMPLATE.md>
- <https://github.com/microsoft/winget-pkgs/blob/master/.github/policies/automergeTriggers.yml>
- <https://github.com/microsoft/winget-pkgs/blob/master/.github/policies/labelAdded.validationCompleted.yml>
- <https://github.com/microsoft/winget-pkgs/blob/master/.github/policies/moderatorTriggers.yml>
- <https://github.com/microsoft/winget-pkgs/blob/master/.github/policies/verifiedDeveloper.yml>
- <https://github.com/microsoft/winget-pkgs/blob/master/.github/policies/scheduledSearch.noRecentActivity.yml>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/Manifest/ManifestValidation.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/Manifest/ManifestCommon.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/Manifest/ManifestYamlPopulator.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/Manifest/Manifest.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/MsiExecArguments.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/Downloader.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCLICore/Workflows/ShellExecuteInstallerHandler.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCLICore/Workflows/MsiInstallFlow.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCLICore/Workflows/InstallFlow.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerRepositoryCore/ARPCorrelation.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerRepositoryCore/ARPCorrelationAlgorithms.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerRepositoryCore/Microsoft/ARPHelper.cpp>
- <https://github.com/microsoft/winget-cli/blob/master/schemas/JSON/manifests/v1.12.0/manifest.version.1.12.0.json>
- <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/package/manifest.md>
- <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/package/repository.md>
- <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/package/winget-validation.md>
- <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/source.md>
- <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/show.md>
- <https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/install.md>
- <https://learn.microsoft.com/en-us/windows/package-manager/package/manifest>
- <https://learn.microsoft.com/en-us/windows/package-manager/package/repository>
- <https://learn.microsoft.com/en-us/windows/package-manager/package/windows-package-manager-policies>
- <https://github.com/microsoft/winget-pkgs/discussions/192170> (review-time quotes)
- <https://github.com/microsoft/winget-pkgs/discussions/333858> (queue backlog)
