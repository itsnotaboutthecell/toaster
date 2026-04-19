# Feature request: Release Code Signing (cross-platform desktop)

## 1. Problem & Goals

Closes PRD `product-map-v1` Blocker B1, expanded to all three
desktop OSes. Today `tauri.conf.json:60` has `"signCommand": ""` and
no notarization or Linux signing. Users on Windows hit SmartScreen
warnings, macOS users hit Gatekeeper "unidentified developer", Linux
users have no integrity check on AppImage / .deb.

This bundle wires Authenticode (Windows), Developer ID + notarytool
(macOS), and detached GPG (Linux) signing into the release pipeline.
Mobile and store distributions are explicitly out of scope.

## 2. Desired Outcome & Acceptance Criteria

- Windows MSI + NSIS installers are Authenticode-signed; SmartScreen
  accepts on a clean Win11 VM.
- macOS .app + .dmg are Developer ID-signed, notarized, stapled;
  Gatekeeper accepts on a clean macOS VM.
- Linux AppImage and .deb ship with detached GPG signatures; the
  maintainer pubkey is published; `gpg --verify` passes against the
  released artifact.
- All signing runs only on tagged release workflows; no secrets in
  the repo.
- One signing wrapper script per platform; secrets injected via
  GitHub Actions org secrets.

## 3. Scope Boundaries

### In scope

- `scripts/sign-windows.ps1` (Authenticode wrapper).
- `scripts/sign-macos.sh` (codesign + notarytool + stapler).
- `scripts/sign-linux.sh` (gpg --detach-sign for AppImage and .deb).
- `tauri.conf.json` `signCommand` for Windows MSI/NSIS pointing at
  the wrapper.
- `wix.signCommand` for the MSI specifically.
- macOS `signingIdentity` set to "Developer ID Application: ..." (no
  longer "-").
- Updates to `.github/workflows/release.yml` to run the signing
  scripts on tagged runs only, with secrets passed via env.
- Documentation: `docs/release-signing.md` describing cert custody
  options (SignPath / Azure Trusted Signing / SafeNet eToken),
  Apple Developer account setup, and GPG key publication location.
- Verification scripts (`signtool verify`, `codesign --verify --deep
  --strict`, `gpg --verify`) wired as CI verification steps.

### Out of scope (explicit)

- iOS / Mac App Store / TestFlight.
- Microsoft Store / WinGet first-party submission.
- Snap / Flatpak signing channels (.deb + AppImage only).
- Auto-renewal of certs.
- Cert procurement (decision documented in
  `docs/release-signing.md`; orchestrator obtains certs).

## 4. References to Existing Code

- `src-tauri/tauri.conf.json:60` — empty `signCommand`.
- `src-tauri/tauri.conf.json:38-44` — macOS bundle config with
  `signingIdentity: "-"` (ad-hoc).
- `.github/workflows/release.yml` — current release workflow; lacks
  signing steps.
- `features/product-map-v1/PRD.md:314-319` — B1 statement.

## 5. Edge Cases & Constraints

- Tagged runs only: `if: startsWith(github.ref, 'refs/tags/v')`.
- Secret detection: scripts must `exit 1` if required env vars are
  missing rather than silently skipping signing.
- Notarization is async; the macOS script must poll until done or
  timeout (15 min default).
- Linux: AppImage signature is detached `.AppImage.sig` next to the
  `.AppImage`; .deb gets `.deb.sig`. Maintainer pubkey published in
  the GitHub Releases body and in the repo at `docs/maintainer.gpg`.
- ASCII-only.
- No hosted-inference dependency.

## 6. Data Model (optional)

n/a (this is build/release infra only).

## Q&A

Pre-answered:

- Q: macOS in scope?
  - A: Yes. Developer ID + notarization + stapling. No Mac App Store.
- Q: Linux signing channel?
  - A: Detached GPG for AppImage + .deb. No Snap, no Flatpak.
- Q: Cert custody for Windows EV?
  - A: Document SignPath and Azure Trusted Signing as the supported
    options. The orchestrator chooses; bundle does not pick.
- Q: Mobile?
  - A: Out of scope. Toaster is desktop-only.
