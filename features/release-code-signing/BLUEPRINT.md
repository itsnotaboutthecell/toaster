# Blueprint: Release Code Signing (cross-platform desktop)

## Architecture decisions

- **R-001 (Windows Authenticode)**: thin PowerShell wrapper at
  `scripts/sign-windows.ps1` that asserts required env vars
  (`SIGN_CERT_PATH` or `SIGN_PROVIDER`, `SIGN_PASSWORD` or
  `SIGN_TOKEN`, `SIGN_TIMESTAMP_URL`), then invokes `signtool sign
  /td sha256 /fd sha256 /tr <url> /tt sha256 <artifact>` for both
  the `.exe` and the MSI. `tauri.conf.json` gets
  `bundle.windows.signCommand: "pwsh scripts/sign-windows.ps1
  --file %1"`. `bundle.windows.wix.signCommand` mirrors. Custody:
  documented in `docs/release-signing.md` (SignPath / Azure Trusted
  Signing / SafeNet eToken).
- **R-002 (macOS)**: bash script at `scripts/sign-macos.sh`. Runs
  `codesign --deep --options runtime --timestamp --sign
  "$APPLE_SIGNING_IDENTITY"` against the `.app`, then signs the
  `.dmg`, submits via `xcrun notarytool submit ... --wait`, and
  runs `xcrun stapler staple`. 15-min polling timeout enforced via
  `notarytool --timeout 15m`. `tauri.conf.json`:
  `bundle.macOS.signingIdentity` becomes a placeholder
  (`"$APPLE_SIGNING_IDENTITY"` substituted at CI time, never a
  literal in the repo).
- **R-003 (Linux GPG)**: bash script at `scripts/sign-linux.sh`.
  `gpg --detach-sign --armor --local-user "$GPG_KEY_ID" <artifact>`
  for both AppImage and .deb. Pubkey at `docs/maintainer.gpg` (ASCII
  armored) committed to the repo as a public artifact, referenced
  from the release notes.
- **R-004 (CI)**: edits to `.github/workflows/release.yml`:
  - `if: startsWith(github.ref, 'refs/tags/v')` on the signing
    job.
  - Env block sources only `${{ secrets.* }}`.
  - Verification step calls `signtool verify`, `codesign --verify
    --deep --strict`, `spctl --assess`, and `gpg --verify`.
  - A repo-wide grep step asserts no PEM/cert material was
    accidentally committed.
- **R-005 (docs)**: `docs/release-signing.md` with per-platform
  sections.

## Component & module touch-list

| File | Change |
|------|--------|
| `scripts/sign-windows.ps1` (new) | Authenticode wrapper, env-var guarded. |
| `scripts/sign-macos.sh` (new) | codesign + notarytool + stapler. |
| `scripts/sign-linux.sh` (new) | GPG detached signatures for AppImage + .deb. |
| `src-tauri/tauri.conf.json:60` | Set `bundle.windows.signCommand` to invoke the wrapper. Set `bundle.windows.wix.signCommand`. |
| `src-tauri/tauri.conf.json:42` | Replace `signingIdentity: "-"` with `"$APPLE_SIGNING_IDENTITY"` (resolved at CI time). |
| `.github/workflows/release.yml` | Add signing job (tag-gated), verification job, secret-grep step. |
| `docs/release-signing.md` (new) | Per-platform cert-custody + setup docs. |
| `docs/maintainer.gpg` (new, generated) | ASCII-armored maintainer pubkey. |
| Source code (`src/`, `src-tauri/src/`) | **No changes.** This bundle is build/release infra only. |

## Single-source-of-truth placement

- Signing parameters live in the wrapper scripts. `tauri.conf.json`
  references the wrappers; CI references the wrappers. There is no
  parallel inline `signtool` call elsewhere.
- Cert/key material lives in CI org secrets, never in the repo.

## Data flow

```
git tag v0.1.0 -> push
  -> .github/workflows/release.yml triggered
       -> build job (Tauri) per platform
       -> signing job (tag-gated):
            Windows  -> scripts/sign-windows.ps1 -> signtool
            macOS    -> scripts/sign-macos.sh    -> codesign + notarytool + staple
            Linux    -> scripts/sign-linux.sh    -> gpg --detach-sign
       -> verification job:
            signtool verify / codesign --verify / spctl / gpg --verify
       -> publish job: upload signed artifacts + .sig files to GitHub Release
```

## Migration / compatibility

- First signed release breaks installer file hashes for users on
  in-place updaters; the existing Tauri updater (`tauri.conf.json:
  67-72`) handles signature verification independently via
  `pubkey`, unaffected.

## Sequencing & conflict-avoidance

- **Position**: bundle 5 of 5 in execution order. Runs after
  Bundles 1-3 so the signed installer ships the full export
  feature set, and after Bundle 4 (`ui-experimental-and-cleanup`)
  so cleanup-removed strings are not in shipped binaries.
- **Files this bundle owns**: the three signing scripts;
  `tauri.conf.json` `signCommand` / `signingIdentity` fields only;
  the signing/verification/grep steps in
  `.github/workflows/release.yml`; `docs/release-signing.md`;
  `docs/maintainer.gpg`.
- **Files this bundle agrees not to touch**: anything under
  `src/`, `src-tauri/src/`, `eval/`, `features/`, or
  `tauri.conf.json` outside the explicit signing fields. No source
  code changes — if a code edit feels necessary, it belongs in a
  different bundle.
- **Coordination with Bundle 1**: Bundle 1 modifies bundle metadata
  (e.g. file associations) only inside the Loudness section per its
  own sequencing rule. Bundle 4 only adds `signCommand` /
  `signingIdentity` fields. The two bundles' edits are non-
  overlapping at field level and merge cleanly.

## Risk register

| Risk | Mitigation | AC catching regression |
|------|------------|------------------------|
| Cert material accidentally committed | `rg` grep step in CI for PEM/cert markers | AC-004-b |
| Signing runs on PR / draft branches and burns secrets | Tag guard `if: startsWith(github.ref, 'refs/tags/v')` | AC-004-a, AC-004-c |
| Notarization hangs forever | 15-min polling timeout in `sign-macos.sh` | AC-002-a |
| Wrapper silently skips signing on missing env vars | Scripts `exit 1` on missing required env vars | AC-001-a, AC-002-a, AC-003-a |
| Stapling skipped (notarized but not stapled means offline launch fails) | Script always invokes `xcrun stapler staple` after notarization succeeds | AC-002-c |
| Linux pubkey not discoverable | `docs/maintainer.gpg` committed; release notes link to it; `docs/release-signing.md` documents it | AC-003-c, AC-005-a |
| EV cert vendor lock-in | `docs/release-signing.md` lists 3 vendor options; choice is reversible | AC-005-a |
| iOS / mobile scope creep | PRD out-of-scope is explicit; reviewer enforces | (architectural) |
| Source-code edits sneak in (Handy-era stub revival) | Bundle owns no source files; PR diff review | (sequencing) |
