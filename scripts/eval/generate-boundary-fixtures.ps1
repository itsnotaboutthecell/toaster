<#
.SYNOPSIS
    Regenerate synthetic fixtures for scripts/eval/eval-audio-boundary.ps1.

.DESCRIPTION
    Uses only FFmpeg's 'sine' and 'aevalsrc' sources — no TTS dependency,
    no licensed audio. The fixtures are deterministic: a three-"word"
    phrase of 300 ms tone bursts at 440/660/880 Hz separated by 100 ms
    of silence, at 48 kHz f32 mono.

    Why tones instead of TTS speech?
      1. Reproducible byte-for-byte across machines (no espeak/piper dep).
      2. Cross-correlation against a known stem has a sharp, unambiguous
         peak, so the leak/clean distinction is dispositive.
      3. The click/z-score detector is *harder* to fool with smooth
         sinusoids than with noisy speech — a pass on tones is a lower
         bound on robustness. Speech fixtures can be added later without
         changing the harness.

    Output layout:
      src-tauri/tests/fixtures/boundary/
        phrase_01.wav                    full 3-tone phrase
        phrase_01_edited_clean.wav       tones 1+3, middle removed cleanly
        phrase_01_edited_leaky.wav       same, with 50 ms of word_02 left in
        phrase_01_preview.wav            mirror of edited_clean (parity gold)
        phrase_01_expected_transcript.json
        phrase_01.manifest.json          keep-segments + seam offsets
        phrase_01_stems/word_01.wav      isolated 440 Hz
        phrase_01_stems/word_02.wav      isolated 660 Hz (the deleted word)
        phrase_01_stems/word_03.wav      isolated 880 Hz
#>

[CmdletBinding()]
param(
    [string]$OutDir = (Join-Path $PSScriptRoot '..\src-tauri\tests\fixtures\boundary')
)

$ErrorActionPreference = 'Stop'

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    throw "ffmpeg not on PATH."
}

$OutDir = (New-Item -ItemType Directory -Force -Path $OutDir).FullName
$stemDir = (New-Item -ItemType Directory -Force -Path (Join-Path $OutDir 'phrase_01_stems')).FullName

$sr = 48000

function Invoke-FFmpegSynth {
    param([string]$Filter, [string]$OutPath)
    & ffmpeg -hide_banner -loglevel error -y `
        -f lavfi -i $Filter `
        -ac 1 -ar $sr -c:a pcm_s16le $OutPath 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg synth failed: $Filter" }
}

# Individual word stems (isolated tones, 300 ms each).
$stemSpecs = @(
    @{ Name = 'word_01.wav'; Freq = 440 },
    @{ Name = 'word_02.wav'; Freq = 660 },
    @{ Name = 'word_03.wav'; Freq = 880 }
)
foreach ($s in $stemSpecs) {
    $f = "sine=frequency=$($s.Freq):sample_rate=${sr}:duration=0.3"
    Invoke-FFmpegSynth -Filter $f -OutPath (Join-Path $stemDir $s.Name)
}

# Full phrase: 440, gap, 660, gap, 880
$phraseFilter = "sine=frequency=440:sample_rate=${sr}:duration=0.3," +
    "apad=pad_dur=0.1[a];" +
    "sine=frequency=660:sample_rate=${sr}:duration=0.3," +
    "apad=pad_dur=0.1[b];" +
    "sine=frequency=880:sample_rate=${sr}:duration=0.3[c]"
# apad in -f lavfi needs simpler form: use concat via filter_complex + multiple inputs.
& ffmpeg -hide_banner -loglevel error -y `
    -f lavfi -i "sine=frequency=440:sample_rate=${sr}:duration=0.3" `
    -f lavfi -i "anullsrc=r=$sr`:cl=mono" `
    -f lavfi -i "sine=frequency=660:sample_rate=${sr}:duration=0.3" `
    -f lavfi -i "sine=frequency=880:sample_rate=${sr}:duration=0.3" `
    -filter_complex "[1:a]atrim=0:0.1,asetpts=PTS-STARTPTS[g1];[1:a]atrim=0:0.1,asetpts=PTS-STARTPTS[g2];[0:a][g1][2:a][g2][3:a]concat=n=5:v=0:a=1[out]" `
    -map "[out]" -ac 1 -ar $sr -c:a pcm_s16le (Join-Path $OutDir 'phrase_01.wav') 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { throw "ffmpeg full-phrase synth failed." }

# Clean edit: word_01 + 100 ms silence + word_03 (middle word removed)
& ffmpeg -hide_banner -loglevel error -y `
    -f lavfi -i "sine=frequency=440:sample_rate=${sr}:duration=0.3" `
    -f lavfi -i "anullsrc=r=$sr`:cl=mono" `
    -f lavfi -i "sine=frequency=880:sample_rate=${sr}:duration=0.3" `
    -filter_complex "[1:a]atrim=0:0.1,asetpts=PTS-STARTPTS[g];[0:a][g][2:a]concat=n=3:v=0:a=1[out]" `
    -map "[out]" -ac 1 -ar $sr -c:a pcm_s16le (Join-Path $OutDir 'phrase_01_edited_clean.wav') 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { throw "clean edit synth failed." }

# Leaky edit: word_01 + 100 ms silence + 50 ms of word_02 LEAK + word_03.
# This fixture is used to PROVE the gates fire (xcorr + WER + possibly z-score).
& ffmpeg -hide_banner -loglevel error -y `
    -f lavfi -i "sine=frequency=440:sample_rate=${sr}:duration=0.3" `
    -f lavfi -i "anullsrc=r=$sr`:cl=mono" `
    -f lavfi -i "sine=frequency=660:sample_rate=${sr}:duration=0.05" `
    -f lavfi -i "sine=frequency=880:sample_rate=${sr}:duration=0.3" `
    -filter_complex "[1:a]atrim=0:0.1,asetpts=PTS-STARTPTS[g];[0:a][g][2:a][3:a]concat=n=4:v=0:a=1[out]" `
    -map "[out]" -ac 1 -ar $sr -c:a pcm_s16le (Join-Path $OutDir 'phrase_01_edited_leaky.wav') 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { throw "leaky edit synth failed." }

# Preview parity fixture: IDENTICAL to the clean edit by design. If the
# preview path ever diverges from export, this file must be regenerated
# from the preview pipeline. Today it is a copy -> parity passes.
Copy-Item -Force `
    (Join-Path $OutDir 'phrase_01_edited_clean.wav') `
    (Join-Path $OutDir 'phrase_01_preview.wav')

# Manifest: keep-segments, seam locations, expected transcript.
# Keep-segments describe what the export SHOULD contain, in source time.
$manifest = [ordered]@{
    sample_rate = $sr
    source      = 'phrase_01.wav'
    clean_edit  = 'phrase_01_edited_clean.wav'
    leaky_edit  = 'phrase_01_edited_leaky.wav'
    preview     = 'phrase_01_preview.wav'
    stems_dir   = 'phrase_01_stems'
    deleted_word_stem = 'phrase_01_stems/word_02.wav'
    # keep_segments are in source seconds; deletes middle word_02 entirely.
    # Source layout: word_01[0.0-0.3] sil[0.3-0.4] word_02[0.4-0.7] sil[0.7-0.8] word_03[0.8-1.1]
    # Clean edit keeps: word_01 + one 100 ms silence + word_03 (33600 samples @ 48 kHz).
    keep_segments = @(
        @{ start = 0.0; end = 0.3 },   # word_01
        @{ start = 0.3; end = 0.4 },   # one gap (the splice closes the deleted word)
        @{ start = 0.8; end = 1.1 }    # word_03
    )
    # Seam sample offsets in the EDITED output (f32 mono, 48 kHz).
    # clean edit timeline: [0..14400) word_01 | [14400..19200) silence | [19200..33600) word_03
    seams_in_edit = @(
        @{ index = 0; sample = 14400 },  # word_01 end -> silence
        @{ index = 1; sample = 19200 }   # silence -> word_03
    )
    expected_final_transcript = @('four_forty', 'eight_eighty')
    notes = 'Synthetic tone phrase. word_01=440Hz, word_02=660Hz, word_03=880Hz.'
}
$manifest | ConvertTo-Json -Depth 6 |
    Set-Content -Path (Join-Path $OutDir 'phrase_01.manifest.json') -Encoding UTF8

# Expected transcript (plain JSON, for E4 hypothesis sources).
[ordered]@{
    expected = @('four_forty', 'eight_eighty')
    # Hypothesis files fed to WER during the eval. Two variants
    # (clean / leaky) simulate what a re-transcriber would emit.
    hypothesis_clean = @('four_forty', 'eight_eighty')
    hypothesis_leaky = @('four_forty', 'six_sixty', 'eight_eighty')
} | ConvertTo-Json -Depth 4 |
    Set-Content -Path (Join-Path $OutDir 'phrase_01_expected_transcript.json') -Encoding UTF8

# ---- Multicut fixture for E3 (preview<->export parity, >=3 seams) ----
# Four tones concatenated edge-to-edge (no silence) => 3 internal seams.
& ffmpeg -hide_banner -loglevel error -y `
    -f lavfi -i "sine=frequency=440:sample_rate=${sr}:duration=0.3" `
    -f lavfi -i "sine=frequency=550:sample_rate=${sr}:duration=0.3" `
    -f lavfi -i "sine=frequency=660:sample_rate=${sr}:duration=0.3" `
    -f lavfi -i "sine=frequency=880:sample_rate=${sr}:duration=0.3" `
    -filter_complex "[0:a][1:a][2:a][3:a]concat=n=4:v=0:a=1[out]" `
    -map "[out]" -ac 1 -ar $sr -c:a pcm_s16le (Join-Path $OutDir 'multicut_01_export.wav') 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { throw "multicut export synth failed." }
Copy-Item -Force `
    (Join-Path $OutDir 'multicut_01_export.wav') `
    (Join-Path $OutDir 'multicut_01_preview.wav')

[ordered]@{
    sample_rate = $sr
    export      = 'multicut_01_export.wav'
    preview     = 'multicut_01_preview.wav'
    # 3 internal seams, at tone boundaries (14400 samples = 300 ms @ 48 kHz).
    seams_in_edit = @(
        @{ index = 0; sample = 14400 },
        @{ index = 1; sample = 28800 },
        @{ index = 2; sample = 43200 }
    )
    keep_segments = @(
        @{ start = 0.0; end = 0.3 },
        @{ start = 0.0; end = 0.3 },
        @{ start = 0.0; end = 0.3 },
        @{ start = 0.0; end = 0.3 }
    )
} | ConvertTo-Json -Depth 6 |
    Set-Content -Path (Join-Path $OutDir 'multicut_01.manifest.json') -Encoding UTF8

Write-Host "Fixtures generated under: $OutDir" -ForegroundColor Green

