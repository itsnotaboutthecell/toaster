-- Task graph for release-code-signing.
INSERT INTO todos (id, title, description, status) VALUES
  ('rcs-windows-script',
   'Add scripts/sign-windows.ps1 (Authenticode wrapper)',
   'Create scripts/sign-windows.ps1: assert required env vars, call signtool sign with /td sha256 /fd sha256 /tr <timestamp> against .exe and .msi. Add --verify-only mode that runs signtool verify /pa /v. Verifier: AC-001-a, AC-001-c.',
   'pending'),

  ('rcs-macos-script',
   'Add scripts/sign-macos.sh (codesign + notarytool + stapler)',
   'Create scripts/sign-macos.sh: codesign --deep --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" against .app and .dmg; xcrun notarytool submit --wait with 15-min timeout; xcrun stapler staple. Add --verify-only mode (codesign --verify --deep --strict + spctl --assess). Verifier: AC-002-a, AC-002-c.',
   'pending'),

  ('rcs-linux-script',
   'Add scripts/sign-linux.sh (GPG detached signatures)',
   'Create scripts/sign-linux.sh: gpg --detach-sign --armor --local-user "$GPG_KEY_ID" against AppImage and .deb. Add --verify-only mode (gpg --verify). Generate docs/maintainer.gpg via gpg --export --armor. Verifier: AC-003-a, AC-003-b.',
   'pending'),

  ('rcs-tauri-conf',
   'Wire signCommand + Apple signingIdentity in tauri.conf.json',
   'Edit src-tauri/tauri.conf.json:60 to set bundle.windows.signCommand to "pwsh scripts/sign-windows.ps1 --file %1"; mirror in bundle.windows.wix.signCommand. Replace bundle.macOS.signingIdentity "-" with "$APPLE_SIGNING_IDENTITY" (CI-resolved). No other tauri.conf.json edits.',
   'pending'),

  ('rcs-release-yml',
   'Update .github/workflows/release.yml: tag-gated signing + verification + secret grep',
   'Add signing job with if: startsWith(github.ref, ''refs/tags/v''). Inject all secrets via ${{ secrets.* }}. Add verification step running the three platform scripts in --verify-only mode. Add a grep step that asserts no PEM/cert markers exist in the repo. Verifier: AC-004-a, AC-004-b, AC-004-c.',
   'pending'),

  ('rcs-docs',
   'Author docs/release-signing.md',
   'Document Windows cert custody (SignPath, Azure Trusted Signing, SafeNet eToken; recommend EV), Apple Developer + App-Specific Password setup, Linux GPG key custody and pubkey publication. Three top-level sections: Windows, macOS, Linux. Verifier: AC-005-a (BLUEPRINT lists docs as deliverable).',
   'pending'),

  ('rcs-qc-windows',
   'QC: Windows signing (R-001)',
   'Verifies AC-001-a (BLUEPRINT doc-section), AC-001-b (live VM install), AC-001-c (signtool verify script).',
   'pending'),

  ('rcs-qc-macos',
   'QC: macOS signing + notarization (R-002)',
   'Verifies AC-002-a, AC-002-b (live macOS VM), AC-002-c (codesign --verify script).',
   'pending'),

  ('rcs-qc-linux',
   'QC: Linux GPG signing (R-003)',
   'Verifies AC-003-a, AC-003-b (gpg --verify script), AC-003-c (PRD documents publication).',
   'pending'),

  ('rcs-qc-ci',
   'QC: CI gating + secret hygiene (R-004)',
   'Verifies AC-004-a (workflow review), AC-004-b (repo-wide grep), AC-004-c (non-tag push skips signing).',
   'pending'),

  ('rcs-qc-docs',
   'QC: docs (R-005)',
   'Verifies AC-005-a (touch-list lists docs/release-signing.md and docs/maintainer.gpg), AC-005-b (sequencing section present).',
   'pending'),

  ('feature-qc',
   'QC: coverage gate green',
   'Run pwsh scripts/check-feature-coverage.ps1 -Feature release-code-signing and confirm exit 0.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('rcs-tauri-conf', 'rcs-windows-script'),
  ('rcs-tauri-conf', 'rcs-macos-script'),
  ('rcs-release-yml', 'rcs-windows-script'),
  ('rcs-release-yml', 'rcs-macos-script'),
  ('rcs-release-yml', 'rcs-linux-script'),
  ('rcs-release-yml', 'rcs-tauri-conf'),
  ('rcs-qc-windows', 'rcs-tauri-conf'),
  ('rcs-qc-windows', 'rcs-release-yml'),
  ('rcs-qc-macos', 'rcs-tauri-conf'),
  ('rcs-qc-macos', 'rcs-release-yml'),
  ('rcs-qc-linux', 'rcs-linux-script'),
  ('rcs-qc-linux', 'rcs-release-yml'),
  ('rcs-qc-ci', 'rcs-release-yml'),
  ('rcs-qc-docs', 'rcs-docs'),
  ('feature-qc', 'rcs-qc-windows'),
  ('feature-qc', 'rcs-qc-macos'),
  ('feature-qc', 'rcs-qc-linux'),
  ('feature-qc', 'rcs-qc-ci'),
  ('feature-qc', 'rcs-qc-docs');
