<#
.SYNOPSIS
    Regenerate multi-backend parity fixtures under
    src-tauri/tests/fixtures/parity/.

.DESCRIPTION
    Each fixture is a deterministic audio clip (FFmpeg sine-burst "words"
    separated by silence) with a companion `oracle.json` containing the
    ground-truth per-word start/end microsecond timings.

    Oracle policy
    -------------
    The canonical parity oracle is a **forced aligner** (Gentle MIT,
    WhisperX BSD-2, or MFA MIT; or whisper.cpp with --max-len 1 once
    p1-authoritative-flag-actionable lands and we pin a native-timing
    model). For these synthetic fixtures the oracle is **analytically
    exact** — we know the source-sample offset of every sine burst,
    which is strictly tighter than any forced aligner on real speech.

    When real speech fixtures are added, regenerate the oracle with:

        # Gentle (MIT, Docker)
        docker run --rm -v "$PWD:/work" lowerquality/gentle \
            /work/audio.wav /work/transcript.txt --nthreads=1 --conservative

        # whisper.cpp (MIT, native) once authoritative-timings lands
        whisper-cli -f audio.wav -m ggml-base.en.bin --max-len 1 --output-json

    …and document the exact invocation + version in `oracle.meta.json`.

    Output layout
    -------------
    src-tauri/tests/fixtures/parity/
      <stem>.wav
      <stem>.oracle.json              [{word,start_us,end_us}, ...]
      <stem>.oracle.meta.json         oracle source + invocation
      backend_outputs/<backend>/<stem>.result.json
          Cached NormalizedTranscriptionResult from a real run of
          <backend> against <stem>.wav. Committed when available; the
          runner skips cleanly if absent.

    The generator only produces .wav + oracle.* — backend outputs are
    cached by whoever runs the live adapter (eval-harness-runner or a
    developer with the backend installed locally).
#>

[CmdletBinding()]
param(
    [string]$OutDir = (Join-Path $PSScriptRoot '..\src-tauri\tests\fixtures\parity')
)

$ErrorActionPreference = 'Stop'

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    throw "ffmpeg not on PATH."
}

$OutDir = (New-Item -ItemType Directory -Force -Path $OutDir).FullName
[void](New-Item -ItemType Directory -Force -Path (Join-Path $OutDir 'backend_outputs\whisper'))
[void](New-Item -ItemType Directory -Force -Path (Join-Path $OutDir 'backend_outputs\parakeet'))

$sr = 48000

# A "word" is a sine burst. Multiple bursts separated by silence
# produce deterministic, analytically-known word boundaries. Each
# fixture = array of { word, freq, duration_s, gap_after_s }.
$fixtures = @(
    @{
        Stem   = 'phrase_alpha'
        Notes  = 'Five connected words with short gaps; mixed durations.'
        Words  = @(
            @{ Word = 'hello';    Freq = 440; Dur = 0.420; Gap = 0.080 },
            @{ Word = 'world';    Freq = 523; Dur = 0.360; Gap = 0.120 },
            @{ Word = 'this';     Freq = 587; Dur = 0.280; Gap = 0.100 },
            @{ Word = 'is';       Freq = 659; Dur = 0.200; Gap = 0.090 },
            @{ Word = 'toaster';  Freq = 784; Dur = 0.540; Gap = 0.000 }
        )
    },
    @{
        Stem   = 'phrase_bravo'
        Notes  = 'Seven words with one long mid-silence (>=400ms) to probe pre-speech padding behavior.'
        Words  = @(
            @{ Word = 'the';      Freq = 392; Dur = 0.180; Gap = 0.080 },
            @{ Word = 'quick';    Freq = 440; Dur = 0.260; Gap = 0.070 },
            @{ Word = 'brown';    Freq = 494; Dur = 0.310; Gap = 0.450 },
            @{ Word = 'fox';      Freq = 523; Dur = 0.300; Gap = 0.100 },
            @{ Word = 'jumps';    Freq = 587; Dur = 0.340; Gap = 0.100 },
            @{ Word = 'over';     Freq = 659; Dur = 0.280; Gap = 0.080 },
            @{ Word = 'lazily';   Freq = 698; Dur = 0.560; Gap = 0.000 }
        )
    }
)

function New-ToneWav {
    param([string]$OutPath, [int]$Freq, [double]$DurS, [int]$SampleRate)
    & ffmpeg -hide_banner -loglevel error -y `
        -f lavfi -i "sine=frequency=$Freq`:sample_rate=$SampleRate`:duration=$DurS" `
        -ac 1 -ar $SampleRate -c:a pcm_s16le $OutPath 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "sine synth failed: f=$Freq dur=$DurS" }
}

function New-SilenceWav {
    param([string]$OutPath, [double]$DurS, [int]$SampleRate)
    & ffmpeg -hide_banner -loglevel error -y `
        -f lavfi -i "anullsrc=r=$SampleRate`:cl=mono" -t $DurS `
        -ac 1 -ar $SampleRate -c:a pcm_s16le $OutPath 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "silence synth failed: dur=$DurS" }
}

function New-Fixture {
    param([hashtable]$Fx, [string]$OutDir, [int]$SampleRate)
    $stem = $Fx.Stem
    $tmpDir = Join-Path $OutDir "$stem.__tmp"
    [void](New-Item -ItemType Directory -Force -Path $tmpDir)
    try {
        $segFiles = [System.Collections.Generic.List[string]]::new()
        $cursorUs = [int64]0
        $oracle   = [System.Collections.Generic.List[object]]::new()
        for ($i = 0; $i -lt $Fx.Words.Count; $i++) {
            $w = $Fx.Words[$i]
            $wordPath = Join-Path $tmpDir ("w{0:D2}.wav" -f $i)
            New-ToneWav -OutPath $wordPath -Freq ([int]$w.Freq) -DurS ([double]$w.Dur) -SampleRate $SampleRate
            [void]$segFiles.Add([string]$wordPath)
            $startUs = [int64]$cursorUs
            $endUs   = [int64]($cursorUs + [int64]([Math]::Round([double]$w.Dur * 1000000.0)))
            [void]$oracle.Add([ordered]@{
                word     = [string]$w.Word
                start_us = [int64]$startUs
                end_us   = [int64]$endUs
            })
            $cursorUs = [int64]$endUs
            if ([double]$w.Gap -gt 0) {
                $gapPath = Join-Path $tmpDir ("g{0:D2}.wav" -f $i)
                New-SilenceWav -OutPath $gapPath -DurS ([double]$w.Gap) -SampleRate $SampleRate
                [void]$segFiles.Add([string]$gapPath)
                $cursorUs = [int64]($cursorUs + [int64]([Math]::Round([double]$w.Gap * 1000000.0)))
            }
        }

        # Concat list
        $listPath = Join-Path $tmpDir 'concat.txt'
        $listLines = foreach ($p in $segFiles) { "file '" + ($p -replace '\\', '/') + "'" }
        $listLines | Set-Content -Path $listPath -Encoding ASCII
        $finalWav = Join-Path $OutDir ("$stem.wav")
        & ffmpeg -hide_banner -loglevel error -y -f concat -safe 0 -i $listPath `
            -ac 1 -ar $SampleRate -c:a pcm_s16le $finalWav 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "concat failed for $stem" }

        $oracleArr = @($oracle.ToArray())
        ($oracleArr | ConvertTo-Json -Depth 4) |
            Set-Content -Path (Join-Path $OutDir "$stem.oracle.json") -Encoding UTF8

        [ordered]@{
            fixture                 = $stem
            oracle_source           = 'analytical_ground_truth'
            oracle_source_rationale = @(
                'Fixture audio is synthesized from FFmpeg sine bursts at known',
                'sample offsets. Per-word start_us/end_us are exact by',
                'construction (cumulative word/gap durations, microsecond-',
                'precise). This is strictly tighter than any forced aligner',
                'would produce on real speech, so the oracle is safe to use',
                'as the parity gate reference for these synthetic fixtures.'
            ) -join ' '
            oracle_invocation       = 'scripts/eval/generate-parity-fixtures.ps1'
            preferred_real_speech_oracle = [ordered]@{
                primary   = 'Gentle (MIT) via Docker: lowerquality/gentle --nthreads=1 --conservative'
                secondary = 'whisper.cpp with --max-len 1 once authoritative-timings lands'
                tertiary  = 'Montreal Forced Aligner (MIT)'
                notes     = 'When real speech fixtures replace synthetic ones, regenerate oracle.json via one of these and update this file.'
            }
            sample_rate_hz         = $SampleRate
            duration_us            = [int64]$cursorUs
            word_count             = $Fx.Words.Count
            notes                  = $Fx.Notes
        } | ConvertTo-Json -Depth 6 |
            Set-Content -Path (Join-Path $OutDir "$stem.oracle.meta.json") -Encoding UTF8

        Write-Host ("  {0}: {1} words, {2:N0} us, {3}" -f $stem, $Fx.Words.Count, $cursorUs, $finalWav) -ForegroundColor Green
    } finally {
        if (Test-Path $tmpDir) { Remove-Item $tmpDir -Recurse -Force -ErrorAction SilentlyContinue }
    }
}

foreach ($fx in $fixtures) { New-Fixture -Fx $fx -OutDir $OutDir -SampleRate $sr }

# README in backend_outputs explaining the cache contract.
$readmePath = Join-Path $OutDir 'backend_outputs\README.md'
@"
# Cached backend outputs for multi-backend parity eval

Layout: ``backend_outputs/<backend>/<fixture_stem>.result.json``

Each file is a ``NormalizedTranscriptionResult`` (see
``src-tauri/src/managers/transcription/adapter.rs``) captured from a real
run of ``<backend>`` against ``<fixture_stem>.wav``. The parity runner
(``scripts/eval/eval-multi-backend-parity.ps1``) consumes these to compute
per-backend boundary error vs the oracle and cross-backend parity.

If no result file exists for a (backend, fixture) pair, the runner logs
a ``skip`` with the reason. In ``-StrictMode`` skip promotes to fail.

Regeneration: the eval-harness-runner agent or a developer with the
backend installed runs the app, transcribes the fixture, serializes the
adapter output here. Do NOT hand-author these files — they must come
from the real adapter path, otherwise the gate is not measuring the
adapter.

Schema (subset):

``````json
{
  "words": [
    { "text": "hello", "start_us": 0, "end_us": 420000, "confidence": null }
  ],
  "language": "en-US",
  "word_timestamps_authoritative": true,
  "input_sample_rate_hz": 16000
}
``````
"@ | Set-Content -Path $readmePath -Encoding UTF8

Write-Host ""
Write-Host "Parity fixtures generated under: $OutDir" -ForegroundColor Cyan
Write-Host "  Commit: *.wav, *.oracle.json, *.oracle.meta.json" -ForegroundColor DarkGray
Write-Host "  Do not commit: oracle intermediates (see .gitignore)" -ForegroundColor DarkGray
