-- Task graph for chapter-markers.
-- Ingest into the session SQL store with the `sql` tool.
--
-- Schema: todos(id TEXT, title TEXT, description TEXT, status TEXT).
-- Allowed status values: 'pending', 'in_progress', 'done', 'blocked'.
-- todo_deps schema: (todo_id TEXT, depends_on TEXT).

INSERT INTO todos (id, title, description, status) VALUES
  ('chapter-markers-paragraph-source',
   'Locate or select the upstream paragraph signal',
   'Decide where paragraph boundaries come from. Repo grep for paragraph|Paragraph in src-tauri/src/ returned zero hits at plan time. Options ranked by preference: (1) an existing editor-level grouping in src-tauri/src/managers/editor/ or transcript post-processing; (2) TranscriptionSegment boundaries from transcribe_rs as consumed at src-tauri/src/managers/transcription/adapter_normalize.rs:14. Forbidden: invent a new clustering heuristic (REQUEST.md §3 out-of-scope). Deliverable: append a Decision section to features/chapter-markers/journal.md citing the chosen signal and the function returning &[Paragraph] (or an equivalent borrowed slice) to the export command. If no acceptable signal exists, mark this task blocked and stop the bundle. No verifier AC — this is a design gate that unblocks chapter-markers-core.',
   'pending'),
  ('chapter-markers-core',
   'Implement managers::export::chapters',
   'Create src-tauri/src/managers/export/chapters.rs per BLUEPRINT.md §Component & module touch-list. Expose Chapter struct, build_chapters_for_export(paragraphs, keep_segments) -> Vec<Chapter>, derive_title, merge_short_chapters, chapters_to_ffmetadata, chapters_to_webvtt. Promote map_source_range_to_edit_time at src-tauri/src/managers/export.rs:239 to pub(crate) and reuse it — do not fork the edit-time mapping. Add #[cfg(test)] submodule with named tests build_happy_path, webvtt_grammar, title_derivation, short_paragraph_merge. Verifiers: AC-001-a, AC-001-c, AC-002-a, AC-002-b per coverage.json.',
   'pending'),
  ('chapter-markers-wire-export',
   'Wire chapter emission into export_edited_media + build_export_args',
   'In src-tauri/src/commands/waveform/commands.rs:386 export_edited_media, between snap_segments_against_media (line 482) and build_export_args (line 483): fetch paragraphs from the source selected by chapter-markers-paragraph-source, call build_chapters_for_export, and when the result is non-empty write a temp .ffmetadata file (cleanup mirroring the ASS temp at line 506) plus a sibling <basename>.chapters.vtt sidecar via Path::with_extension("chapters.vtt") against output_path_buf (line 464). Extend src-tauri/src/commands/waveform/mod.rs:571 build_export_args with an optional chapter_metadata_path parameter; when Some prepend -f ffmetadata -i <path> and append -map_metadata <idx>. Empty-paragraph path MUST pass None and skip the sidecar (R-004). Verifier: AC-001-b, AC-003-a, AC-004-a via the eval scripts once chapter-markers-eval implements them.',
   'pending'),
  ('chapter-markers-eval',
   'Replace eval stubs with real fixture checks',
   'Replace the planning stubs at scripts/eval/eval-chapter-markers.ps1 (exit 2) and scripts/eval/eval-chapter-markers-stretch.ps1 (exit 2). The container-mode run must render a fixture edit, run ffprobe -show_chapters, and assert atoms match the backend Vec<Chapter> within 1 ms per boundary (AC-001-b). The empty-mode run must render a fixture whose paragraph list is empty and assert zero chapter atoms AND no <basename>.chapters.vtt (AC-004-a). The stretch script must render a 2x-stretch segment spanning a paragraph boundary and assert chapter starts track edit time (AC-003-a). Both scripts must exit 0 on success and a non-zero code on failure so scripts/eval/run-eval-harness.ps1 can adopt them later. Verifiers: AC-001-b, AC-003-a, AC-004-a.',
   'pending'),
  ('chapter-markers-qc',
   'QC: coverage + tasks gates',
   'Run pwsh scripts/feature/check-feature-coverage.ps1 -Feature chapter-markers AND pwsh scripts/feature/check-feature-tasks.ps1 -Feature chapter-markers; both must exit 0.',
   'pending'),
  ('feature-qc',
   'QC: run eval harness',
   'Run the eval harness via the eval-harness-runner agent to confirm chapter-markers evals are green alongside the existing edit-quality + audio-boundary evals.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  ('chapter-markers-core', 'chapter-markers-paragraph-source'),
  ('chapter-markers-wire-export', 'chapter-markers-core'),
  ('chapter-markers-eval', 'chapter-markers-wire-export'),
  ('chapter-markers-qc', 'chapter-markers-eval'),
  ('feature-qc', 'chapter-markers-qc');
