-- Task graph for time-stretch-segments.
-- Ingest into the session SQL store with the `sql` tool.

-- Schema: todos(id TEXT, title TEXT, description TEXT, status TEXT).
-- Allowed status values: 'pending', 'in_progress', 'done', 'blocked'.
-- Do not invent columns (no estimate_minutes, no owner, etc).
-- todo_deps schema: (todo_id TEXT, depends_on TEXT). No predecessor/successor.

INSERT INTO todos (id, title, description, status) VALUES
  ('tss-data-model',
   'Persist SegmentStretch on ProjectSettings',
   'Add SegmentStretch struct and ProjectSettings.segment_stretches: Vec<SegmentStretch> with #[serde(default)]. Add a clamping setter enforcing [0.5, 2.0] with documented invariant. Follow the caption_profiles pattern in managers/project.rs:45-46. Verifies AC-001-a and AC-001-b via cargo tests project::tests::segment_stretches_serde_default and set_segment_stretch_clamps.',
   'pending'),
  ('tss-canonical-type',
   'Promote CanonicalKeepSegment with stretch',
   'Introduce CanonicalKeepSegment { start_us, end_us, stretch } in commands/waveform/mod.rs. Thread through canonical_keep_segments_for_media, canonical_keep_segments_for_media_with_options, select_raw_keep_segments_for_media, snap_segments_against_media, build_audio_segment_filter, build_audio_concat_filter_with_fade, and map_edit_time_to_source_time_from_segments. Update public KeepSegment specta type with stretch: f32 default 1.0. Prepare the thread for audio-graph and time-map consumers.',
   'pending'),
  ('tss-time-map',
   'Stretch-aware edit<->source time mapping',
   'Update map_edit_time_to_source_time_from_segments (commands/waveform/mod.rs:393-405) and map_source_to_edit (managers/captions/layout.rs:437-455) to consume stretch via a shared edit_duration_of helper. Preserve existing precision tests on the identity path. Prerequisite for AC-002-b.',
   'pending'),
  ('tss-audio-graph',
   'Inject atempo into shared audio filter graph',
   'In build_audio_segment_filter append ,atempo={stretch:.6} after asetpts=PTS-STARTPTS when stretch != 1.0; omit otherwise so identity graph is byte-identical to today. Fade policy: authored in source microseconds, rendered duration naturally scales by 1/stretch. Document in rustdoc. Preview and export share this filter; no duplication.',
   'pending'),
  ('tss-video-graph',
   'Stretch video PTS on export',
   'In the export video branch (commands/waveform/mod.rs:650-694) append ,setpts=(PTS-STARTPTS)/{stretch:.6} after the existing setpts=PTS-STARTPTS. Omit when stretch == 1.0 to preserve byte-identical identity output. Ensures exported video frames stay lip-synced with stretched audio.',
   'pending'),
  ('tss-preview-cache',
   'Fold stretch into preview cache key',
   'Update edit_version_token (commands/waveform/commands.rs:~245) to incorporate per-segment stretch factors so cached previews invalidate when any stretch changes. Add a unit test asserting token differs for the same segments with stretch 1.0 vs 1.5.',
   'pending'),
  ('tss-editor-state',
   'Hydrate stretch on EditorState and expose accessor',
   'Store segment_stretches on EditorState; expose get_stretch_for_segment(anchor_start, anchor_end) -> f32 used by canonical_keep_segments_for_media. Hydrate from ProjectSettings on project load. Drop orphaned anchor entries on next save and log at info.',
   'pending'),
  ('tss-set-stretch-command',
   'set_segment_stretch Tauri command + specta bindings',
   'Add set_segment_stretch(anchor_start_us, anchor_end_us, value) command. Clamp to [0.5, 2.0] at the command boundary, update EditorState.segment_stretches, bump timeline_revision, return canonical value. Register in lib.rs near map_edit_to_source_time (line ~291). Regenerate src/bindings.ts via specta — never hand-edit.',
   'pending'),
  ('tss-export-parity',
   'Export parity test for stretched segment duration',
   'Add a fixture-based test under commands/waveform/tests/ that exports a project containing one segment with stretch=1.5 and asserts the output audio duration for that segment equals source_duration / 1.5 within 1 sample at the output sample rate. Verifies AC-001-c via audio-boundary-eval skill fixtures.',
   'pending'),
  ('tss-preview-parity',
   'Preview parity test for stretched segment duration',
   'Add a fixture-based test that renders the preview for the same project and asserts the preview audio duration for that segment matches export duration to within 1 sample. Verifies AC-002-a via audio-boundary-eval skill fixtures.',
   'pending'),
  ('tss-caption-alignment',
   'Captions through stretched segments stay aligned',
   'Extend transcript-precision-eval fixtures with a stretched-segment case. Assert every caption line in the exported video falls within 1 frame of the stretched word position. Verifies AC-002-b.',
   'pending'),
  ('tss-context-menu-ui',
   'Segment context-menu stretch control',
   'Extend the segment context menu in src/components/editor/ with a numeric input (two-decimal), slider, and reset button bound to a single stretch state. Debounced commits call commands.setSegmentStretch. Add i18n keys across all 20 locales (enforced by scripts/check-translations.ts). Verifies AC-003-a via live-app manual steps.',
   'pending'),
  ('tss-player-ipc-routing',
   'Route edit<->source via IPC when any stretch is active',
   'In src/components/player/MediaPlayer.tsx, when any segment has stretch != 1.0, resolve edit<->source conversions via backend commands.mapEditToSourceTime (line 379-492 today uses the local TS helper). Drive videoRef.playbackRate from the stretch of the segment under the edit-time cursor. Never swap the video source (AGENTS.md critical rule).',
   'pending'),
  ('tss-backward-compat',
   'v1.1.0 project fixture loads with stretch=1.0',
   'Add a fixture .toaster file saved under PROJECT_VERSION=1.1.0 (no segment_stretches field). Unit test loads it and asserts every derived segment reports effective stretch=1.0 through the editor accessor. Bump PROJECT_VERSION to 1.2.0 only in the save path and extend the doc-comment at project.rs:12-17. Verifies AC-004-a.',
   'pending'),
  ('tss-feature-qc',
   'Feature QC: run coverage gate + eval harness',
   'Run pwsh scripts/feature/check-feature-coverage.ps1 -Feature time-stretch-segments and pwsh scripts/feature/check-feature-tasks.ps1 -Feature time-stretch-segments; both must exit 0. Then invoke eval-harness-runner agent and confirm pass JSON. Final gate before feature ships.',
   'pending');

INSERT INTO todo_deps (todo_id, depends_on) VALUES
  -- Canonical type prereqs the audio/video/time-map work
  ('tss-time-map',            'tss-canonical-type'),
  ('tss-audio-graph',         'tss-canonical-type'),
  ('tss-video-graph',         'tss-canonical-type'),
  ('tss-preview-cache',       'tss-canonical-type'),
  -- Editor state bridges persisted data model into the canonical accessor
  ('tss-editor-state',        'tss-data-model'),
  ('tss-editor-state',        'tss-canonical-type'),
  -- Command surface needs both the data model and the accessor
  ('tss-set-stretch-command', 'tss-editor-state'),
  -- UI consumes the command + routes through backend time maps
  ('tss-context-menu-ui',     'tss-set-stretch-command'),
  ('tss-player-ipc-routing',  'tss-time-map'),
  -- Parity tests need the full audio/video/time-map pipeline live
  ('tss-export-parity',       'tss-audio-graph'),
  ('tss-export-parity',       'tss-video-graph'),
  ('tss-preview-parity',      'tss-audio-graph'),
  ('tss-preview-parity',      'tss-preview-cache'),
  ('tss-caption-alignment',   'tss-time-map'),
  -- Backward-compat test requires the persisted model
  ('tss-backward-compat',     'tss-data-model'),
  -- Final QC waits for everything
  ('tss-feature-qc',          'tss-export-parity'),
  ('tss-feature-qc',          'tss-preview-parity'),
  ('tss-feature-qc',          'tss-caption-alignment'),
  ('tss-feature-qc',          'tss-context-menu-ui'),
  ('tss-feature-qc',          'tss-player-ipc-routing'),
  ('tss-feature-qc',          'tss-backward-compat');

