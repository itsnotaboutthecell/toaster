# Assessment: Scriptable video editing

> Exploratory document. Not a PRD. Not on the coverage gate. Captures the architectural surface so the user can pick a direction.

## 1. Data model sketch - "video edit profile"

Pseudocode of what a profile struct would carry. Names illustrative.

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportProfile {
    pub schema_version: u32,                    // forward-compat field
    pub name: String,                           // "desktop-tutorial", "mobile-shorts", ...
    pub orientation: Orientation,               // Horizontal | Vertical | Source
    pub aspect_ratio: Option<AspectRatio>,      // override source aspect, e.g. 9:16
    pub captions: CaptionProfile,               // position %, font, bg, brand color, max width
    pub brand: BrandPalette,                    // primary / secondary / accent hex
    pub cleanup: CleanupProfile,                // protected words, filler words, prompt template id
    pub audio: AudioProfile,                    // loudness target LUFS, fade in/out ms, normalize on/off
    pub video: VideoProfile,                    // codec, bitrate target, encoder pref (cpu/gpu)
    pub output: OutputTarget,                   // file path template, container format
    pub edits: Option<EditScript>,              // optional declarative cuts; absent = "use the project's editor state"
}

pub enum Orientation { Horizontal, Vertical, Source }

pub struct CaptionProfile {
    pub position_percent: f32,
    pub font_family: CaptionFontFamily,
    pub font_size_px: u32,
    pub text_color: Rgba,
    pub bg_color: Rgba,
    pub max_width_percent: f32,
    pub padding_x_px: u32,
    pub padding_y_px: u32,
    pub corner_radius_px: u32,
}

pub struct BrandPalette { pub primary: Rgba, pub secondary: Rgba, pub accent: Rgba }

pub struct CleanupProfile {
    pub allow_words: Vec<String>,         // == AppSettings.custom_words
    pub filler_words: Vec<String>,        // == AppSettings.custom_filler_words
    pub prompt_template_id: Option<String>,
}
```

Critical: every field above already has a corresponding scalar setting in `AppSettings`. A profile is a *named bundle of overrides*, not a new data domain. This is what makes the wedge step (Section 6) valuable in isolation.

## 2. Pipeline touchpoints (where profiles plug in)

- **Export command boundary**: `src-tauri/src/commands/export.rs` - the natural place to accept a `profile: Option<ExportProfile>` parameter. When present, fields override `AppSettings` for the duration of that export. When absent, behavior is identical to today (no regression risk for existing GUI users).
- **Splice / boundaries**: `src-tauri/src/managers/splice/{boundaries.rs, loudness.rs, clarity.rs}` already encapsulate per-edit decisions and consume scalar parameters. They do not need to know about profiles - the export command flattens profile -> per-call params.
- **Caption authority**: `src-tauri/src/managers/captions/` - single source of truth for layout. Profile values flow through here, never duplicated in the React preview.
- **Editor / time mapping**: `src-tauri/src/managers/editor/` - keep-segments are project-scoped, not profile-scoped. A profile's optional `EditScript` field would be a *declarative override* of the project's editor state (e.g. "cut the first 5 seconds, regardless of GUI state").
- **Cleanup**: `src-tauri/src/managers/cleanup/` - Item 3 (`postprocessor-word-list-source-of-truth`) already aligns the cleanup module with `custom_words` / `custom_filler_words`. Once Item 3 lands, the profile's `CleanupProfile` is a one-line plug-in.

## 3. Single-source-of-truth implications

Profiles encode *user intent*; backend managers remain the single source of truth for *behavior*. Concrete rules:

- A profile may set `caption_position_percent = 12.0`. The backend `managers/captions/` still computes the actual ASS line; the React preview consumes the same backend output. No layout math in profile or in the React side.
- A profile may set `cleanup.filler_words = ["um", "uh"]`. The backend cleanup prompt builder reads this list; the React `DiscardWords` UI also reads from the same setting key. Two surfaces, one source.
- A profile may declare an `EditScript`. The backend editor manager applies it as the keep-segment authority for that export run; the React editor view reflects the same segments via the existing project read path.

If at any future point a profile needs a value the backend does not already authority-own, that is a sign the architecture is leaking - either lift the authority into the right manager, or stop and re-discuss.

## 4. Risk register

| Risk | Description | Mitigation guidance |
|------|-------------|---------------------|
| Hosted-inference creep | A "scripted edit" tempts contributors to add an LLM call ("auto-tag scenes", "auto-color-grade"). | Hard No per AGENTS.md. Any profile field that takes a model name must be a local-only provider id (same registry as cleanup, today). Code review checks this at PR time. |
| Test/fixture explosion | One profile per device class (desktop / mobile / square / 4k) plus per-brand permutations explodes the eval matrix. | Profiles compose: a `desktop-base` + a `brand-acme` overlay. Eval gates run only the profiles that touched files in the diff. Reuse existing `eval/fixtures/` per AGENTS.md "transcript-precision-eval". |
| Profile schema drift | Once profiles ship, `schema_version` becomes a long-tail compatibility burden. | Treat profiles as code-adjacent: keep the schema in `src-tauri/src/managers/export/profile.rs` (or wherever the export module lands) with serde derive + a migration stub from day 1. |
| FFmpeg version coupling | Encoder flags vary across FFmpeg versions; a profile authored against ffmpeg 6 may break on ffmpeg 7. | Keep encoder preferences declarative (codec name + bitrate + preset name) and let the export pipeline translate to flags per detected ffmpeg version, not hardcoded in the profile JSON. |
| Profile-vs-project source-of-authority confusion | A user opens project X with profile Y - whose caption position wins? | Decision-of-record: profile overrides project, project overrides defaults. Document this in `docs/post-processing.md` (or wherever profiles get docs) before shipping any architecture. |
| Accidental external file system writes | Profiles often include "output path templates"; bugs in template parsing can write outside the chosen directory. | Path canonicalization + reject-if-escapes-base in the export command. Same gate as today's manual export already enforces. |

## 5. Three candidate architectures

### 5.A - JSON profile + CLI flag on existing exporter

- **Shape:** add `--profile <path-to-json>` to the export Tauri command. Profiles are plain JSON files matching the `ExportProfile` schema. A small `toaster export --profile X --input proj.toaster --output out.mp4` shim wraps the Tauri command for headless use.
- **Pros:** smallest possible change; reuses 100% of the existing export pipeline; trivially scriptable from any language; no new dependencies; profiles are diff-friendly text.
- **Cons:** no expressiveness beyond fields the schema models. Conditional logic ("if source is portrait, use mobile profile, else desktop") lives in the user's wrapper script, not in Toaster.
- **Effort:** smallest. Could be a single feature.
- **Risk:** lowest.

### 5.B - Declarative YAML pipeline (multi-step)

- **Shape:** a YAML pipeline file describing a sequence of named steps (`load-project`, `apply-cleanup`, `apply-captions`, `export`). Each step takes structured inputs. The pipeline runner knows nothing about FFmpeg - it dispatches to the same backend managers.
- **Pros:** richer expressiveness (multi-output runs, `for-each` over profiles, conditional branches via simple expressions). Familiar to CI users.
- **Cons:** new runtime dependency (YAML parser + a small expression engine). Profile and pipeline are now two separate concepts. Higher learning curve for users.
- **Effort:** medium. Two or three features.
- **Risk:** medium - the expression engine is a small surface that can grow uncontrollably.

### 5.C - Embedded scripting (e.g. Rhai)

- **Shape:** users write small `.rhai` scripts that call Toaster APIs. Profiles become structured data passed into scripts; pipelines become arbitrary code.
- **Pros:** maximum flexibility; users can express any conditional / iterative logic.
- **Cons:** sandboxing burden (a malicious script must not exfiltrate the project file or call out to the network); scripts are no longer diff-friendly text; "what does this profile do" requires reading code; tempts hosted-inference creep ("just import a hosted SDK from your script").
- **Effort:** large. Multiple features.
- **Risk:** highest, especially around sandbox + hosted-inference creep.

## 6. Wedge first step (no commitment to an architecture)

**Extract the current export pipeline's per-call configuration into a typed `ExportProfile` struct (Rust), with serde derive and a unit test asserting round-trip JSON.** No CLI flag, no script runner, no scripting language - just refactor today's "the export command takes 14 separate parameters" into "the export command takes one `ExportProfile` (built from `AppSettings` if absent)."

Why this is a no-regrets step regardless of which architecture wins:

- Architecture A (JSON + CLI): the struct *is* the JSON schema; the CLI flag deserializes into it directly.
- Architecture B (YAML pipeline): the struct is the type one of the pipeline steps takes; the YAML loader builds it.
- Architecture C (Rhai): the struct is the type the script API hands back; the script's interface is `export(profile)`.

It also delivers immediate value today: it forces the export pipeline to define its own contract instead of being implicitly coupled to `AppSettings`, which makes Items 1-3 in the user's current feedback batch easier to test and easier to reason about.

This wedge would be the candidate first PM feature once the user picks a direction. **No work happens on it until then.**

## 7. Open questions for the user

1. Architecture choice: A, B, C, or "stay with the wedge step for now and decide later"?
2. Headless invocation: does Toaster need a true CLI binary that runs without launching the Tauri window, or is "scriptable from inside the app" (e.g. a profile dropdown next to Export) sufficient? The two have different effort profiles.
3. Profile distribution: are profiles per-user (live in `AppData`) or per-project (live next to the `.toaster` project file)? Both have valid use cases; pick one default.
4. Brand palette scope: a single global palette per user, or per-profile? Today there is no brand-color setting at all; this would be net-new.
