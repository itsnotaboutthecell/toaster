<#
.SYNOPSIS
    Stub for the poster-frame-export eval. Not yet implemented.

.DESCRIPTION
    Planning-phase stub committed so features/poster-frame-export
    can reach STATE=planned with a valid coverage.json script
    verifier. Real implementation lands as part of the
    poster-frame-export feature execution (see
    features/poster-frame-export/tasks/poster-frame-export-eval).

    When implemented, accepts:
      -Mode <attachment|no-attachment|clamp|audio-only|cleanup|argv-identity>
      -Format <mp4|mov|mp3|wav|m4a|opus>
      -Fixture <path to .toaster project>

    Behavior by mode:
      attachment      : export fixture; assert ffprobe shows an
                        image/png attachment stream.
      no-attachment   : export fixture with poster_frame_ms = null;
                        assert no attachment stream.
      clamp           : export fixture with poster_frame_ms past
                        the edited duration; assert a valid
                        attachment stream still exists.
      audio-only      : export fixture to -Format audio-only;
                        assert no attachment and no -attach argv.
      cleanup         : run repeated exports (success + induced
                        failure); assert zero toaster_poster_*.png
                        remain in std::env::temp_dir().
      argv-identity   : with poster_frame_ms = null, assert the
                        logged FFmpeg argv is byte-identical to the
                        committed golden in eval/fixtures/.

    Exits 0 on pass, 1 on fail, 2 on not-implemented.
#>

[CmdletBinding()]
param(
    [string]$Mode = 'attachment',
    [string]$Format = 'mp4',
    [string]$Fixture
)

Write-Host "[STUB] eval-poster-frame.ps1 not implemented yet (Mode=$Mode Format=$Format)." -ForegroundColor Yellow
Write-Host "       Implementation is tracked by feature 'poster-frame-export'."
exit 2
