# PRD: Release Code Signing (cross-platform desktop)

## Problem & Goals

The release pipeline ships unsigned installers
(`tauri.conf.json:60`, `signCommand: ""`; macOS `signingIdentity:
"-"` ad-hoc; no Linux signing). End users hit SmartScreen
(Windows), Gatekeeper (macOS), and have no way to verify Linux
artifacts. This bundle adds Authenticode, Developer ID +
notarization, and detached GPG signing, gated on tagged release
workflows, with secrets injected via GitHub Actions org secrets.

Desktop-only. iOS, Mac App Store, Microsoft Store, Snap, and
Flatpak distribution are explicitly out of scope.

## Scope

### In scope

- Three platform-specific wrapper scripts.
- `tauri.conf.json` and `wix.signCommand` wiring for Windows.
- macOS `signingIdentity` set; `notarytool` + stapler invocation.
- Linux GPG detached-signature production + verification.
- Release workflow updates: signing runs only on tagged refs.
- `docs/release-signing.md` documenting cert options and key
  custody.
- CI verification: `signtool verify`, `codesign --verify --deep
  --strict`, `gpg --verify` against the published artifacts.

### Out of scope (explicit)

- iOS / TestFlight.
- Mac App Store / Microsoft Store / WinGet submission.
- Snap / Flatpak signing.
- Cert procurement workflow (documented as a manual orchestrator
  step).
- Auto-rotation / renewal of certs.

## Requirements

### R-001 — Windows Authenticode signing

- Description: `scripts/sign-windows.ps1` consumes a CI-injected
  cert (path or SignPath/Azure Trusted Signing token) and signs the
  Tauri-built `.exe`, MSI, and NSIS installer artifacts.
  `tauri.conf.json` `signCommand` invokes the wrapper. `wix.
  signCommand` is set so the MSI is signed during Tauri's build.
- Acceptance Criteria
  - AC-001-a — Source review of `scripts/sign-windows.ps1` confirms
    it (i) errors out if required env vars are absent, (ii) calls
    `signtool sign` with `/td sha256 /fd sha256 /tr` (timestamping)
    and (iii) signs both the `.exe` and the MSI.
  - AC-001-b — Manual: run the signed installer on a clean
    Windows 11 VM; SmartScreen accepts (or shows the standard
    "Verified Publisher" dialog, not the "Unknown Publisher"
    warning).
  - AC-001-c — Script verification: `signtool verify /pa /v
    <artifact>` exits 0 against the published `.exe` and `.msi`.
    Wired as a CI step `signtool verify` on the release workflow.

### R-002 — macOS Developer ID + notarization + stapling

- Description: `scripts/sign-macos.sh` runs `codesign --deep
  --options runtime --timestamp` against the `.app`, then bundles
  to `.dmg`, signs the `.dmg`, submits to `notarytool`, polls
  until the notarization is `Accepted`, and runs `xcrun stapler
  staple` against both `.app` and `.dmg`. `tauri.conf.json`
  `signingIdentity` is set to a Developer ID identity (provided via
  env var on CI; documented as `"Developer ID Application: ..."`).
- Acceptance Criteria
  - AC-002-a — Source review of `scripts/sign-macos.sh` confirms
    the codesign + notarytool + stapler sequence and a 15-minute
    polling timeout.
  - AC-002-b — Manual: open the signed/stapled `.dmg` on a clean
    macOS VM; Gatekeeper accepts and the app launches without an
    "unidentified developer" prompt.
  - AC-002-c — Script verification: `codesign --verify --deep
    --strict <Toaster.app>` and `spctl --assess --type execute -v
    <Toaster.app>` both exit 0. Wired as a CI verification step.

### R-003 — Linux detached GPG signatures

- Description: `scripts/sign-linux.sh` runs `gpg --detach-sign
  --armor` against the AppImage and the `.deb`, producing
  `Toaster-<ver>.AppImage.sig` and `toaster_<ver>_amd64.deb.sig`.
  The maintainer pubkey is published at `docs/maintainer.gpg` and
  referenced in the GitHub Release body.
- Acceptance Criteria
  - AC-003-a — Source review of `scripts/sign-linux.sh` confirms
    detached signature output for both AppImage and .deb and an
    exit-on-missing-key guard.
  - AC-003-b — Script verification: `gpg --verify
    Toaster.AppImage.sig Toaster.AppImage` and the equivalent for
    the `.deb` both exit 0 against the published artifacts. Wired
    as a CI step.
  - AC-003-c — `docs/release-signing.md` documents the maintainer
    pubkey location and the verification command users should run.

### R-004 — CI gating: signing only on tagged releases, no repo
secrets

- Description: signing steps in `.github/workflows/release.yml` are
  gated on `if: startsWith(github.ref, 'refs/tags/v')`. All
  required secrets are injected via `secrets.*` env vars; no cert,
  password, or key material is committed to the repo.
- Acceptance Criteria
  - AC-004-a — `.github/workflows/release.yml` review confirms the
    signing job has the tag guard and references env-injected
    secrets only.
  - AC-004-b — `rg -i "MII[A-Za-z0-9+/]{20,}|-----BEGIN
    (CERTIFICATE|PRIVATE KEY|PGP PRIVATE)" .` returns zero
    matches outside `node_modules/`, `target/`, and `dist/`.
    Documented as a CI grep step.
  - AC-004-c — Manual: trigger a non-tagged push and confirm the
    signing steps are skipped (visible in workflow logs).

### R-005 — Release-signing docs

- Description: `docs/release-signing.md` documents (i) Windows cert
  custody options (SignPath, Azure Trusted Signing, SafeNet
  eToken; recommend EV), (ii) Apple Developer account requirement
  + how to generate the App-Specific Password for notarytool, (iii)
  Linux GPG key custody + pubkey publication.
- Acceptance Criteria
  - AC-005-a — `docs/release-signing.md` exists and contains the
    three platform sections named "Windows", "macOS", and "Linux"
    with the required subsections.
  - AC-005-b — `BLUEPRINT.md` "Sequencing & conflict-avoidance"
    section confirms this bundle runs after Bundles 1-3 (so signed
    installers contain the full export feature set) and only edits
    `tauri.conf.json` to add `signCommand` (does not touch the
    bundle metadata Bundle 1 might also touch; coordinated via the
    sequencing rule).

## Edge cases & constraints

- Cert expiry: out of scope; documented as a maintainer task in
  `docs/release-signing.md`.
- Notarization timeout: hard 15-minute polling cap; on timeout the
  release job fails loudly so a maintainer can investigate.
- A user without `gpg` installed can still install the .deb / run
  the AppImage; signature verification is opt-in.
- ASCII-only changes; 800-line cap holds.

## Data model (if applicable)

n/a.

## Non-functional requirements

- AGENTS.md "Local-only inference" — signing infrastructure has no
  inference path.
- AGENTS.md "Verified means the live app, not `cargo check`" —
  R-001/R-002 include live-app installer ACs on clean VMs.
- No secrets in the repo (R-004 grep AC).
