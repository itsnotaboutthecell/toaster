-- Task graph for export-format-mov.
-- Ingest into the session SQL store with the `sql` tool.

-- Schema: todos(id TEXT, title TEXT, description TEXT, status TEXT).
-- Allowed status values: 'pending', 'in_progress', 'done', 'blocked'.
-- todo_deps schema: (todo_id TEXT, depends_on TEXT).
INSERT INTO todos (id, title, description, status) VALUES
  ('mov-backend-enum',
   'Add Mov variant to AudioExportFormat enum',
   'Edit src-tauri/src/commands/waveform/export_format.rs:29 to add Mov between Mp4 and Mp3 (serde rename_all=lowercase handles the wire form). Update extension() to return ".mov", is_audio_only() returns false, export_format_codec_map returns None (same as Mp4). Extend the existing module-level tests and add new tests export_format_mov_variant (AC-001-a) and export_format_mov_settings_roundtrip (AC-002-b) that assert the serde round-trip produces "mov". No changes to commands/waveform/mod.rs in this task. Verifier: see coverage.json AC-001-a and AC-002-b.',
   'pending'),
  ('mov-ffmpeg-args',
   'Emit -f mov + -pix_fmt yuv420p in build_export_args',
   'Edit src-tauri/src/commands/waveform/mod.rs:571-729 (build_export_args). When format == AudioExportFormat::Mov, append ["-f", "mov", "-pix_fmt", "yuv420p"] to args just before the trailing args.push(output_path.to_string()) at line 727. Mp4 path is unchanged. Add an integration test under src-tauri/tests/ named export_format_mov_codec_parity that asserts the argv for Mp4 and Mov differ only in (a) the -f mux tokens (absent on Mp4, present on Mov) and (b) the trailing output path. All -c:*, -b:*, -vf, -af, -filter_complex, and -map tokens must be byte-identical. Verifier: coverage.json AC-003-a.',
   'pending'),
  ('mov-frontend-option',
   'Surface mov option in Advanced Settings export dropdown',
   'Edit src/components/settings/advanced/ExportGroup.tsx:18 to change EXPORT_FORMATS from ["mp4", "mp3", "wav", "m4a", "opus"] to ["mp4", "mov", "mp3", "wav", "m4a", "opus"]. No other changes; the dropdown label flows through t("settings.export.format.options.${value}.label") so this alone wires the UI. After cargo build regenerates src/bindings.ts, confirm the AudioExportFormat union there contains "mov"; do NOT hand-edit bindings.ts beyond the temporary one-line union patch permitted by AGENTS.md. Verifier: coverage.json AC-001-c (live-app manual check).',
   'pending'),
  ('mov-i18n-20-locales',
   'Add settings.export.format.options.mov.{label,description} to 20 locales',
   'Add the new key to src/i18n/locales/en/translation.json:458 (alongside mp4) with label "Video (mov)" and description "Edited video with H.264 video and AAC audio in a mov container. Use for Final Cut, Premiere, or DaVinci Resolve import on macOS." For the other 19 locales (ar, bg, cs, de, es, fr, he, it, ja, ko, pl, pt, ru, sv, tr, uk, vi, zh, zh-TW), add the same nested key path with the English copy as the fallback per the existing convention; translators will complete the localization in a subsequent pass. Run bun scripts/check-translations.ts and confirm exit 0. Verifier: coverage.json AC-002-a.',
   'pending'),
  ('mov-qc',
   'QC: ffprobe fixture export at mp4 and mov',
   'Run the manual ffprobe live-app verification from coverage.json AC-001-b. Requires completion of all four implementation tasks. Record the ffprobe output for both containers in the session journal and in the PR body.',
   'pending'),
  ('feature-qc',
   'Run coverage + translations + file-size gates',
   'Run: (1) pwsh scripts/feature/check-feature-coverage.ps1 -Feature export-format-mov; (2) pwsh scripts/feature/check-feature-tasks.ps1 -Feature export-format-mov; (3) bun scripts/check-translations.ts; (4) bun run check:file-sizes. All must exit 0. Then run the eval-harness-runner agent to re-validate nothing downstream regressed.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('mov-ffmpeg-args',     'mov-backend-enum'),
  ('mov-frontend-option', 'mov-backend-enum'),
  ('mov-i18n-20-locales', 'mov-frontend-option'),
  ('mov-qc',              'mov-ffmpeg-args'),
  ('mov-qc',              'mov-frontend-option'),
  ('mov-qc',              'mov-i18n-20-locales'),
  ('feature-qc',          'mov-qc');

