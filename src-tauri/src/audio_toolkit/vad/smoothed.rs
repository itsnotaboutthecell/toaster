//! Pre-roll / onset / hangover hysteresis around a raw VAD.
//!
//! Lifted verbatim (plus this header and the three default
//! constants) from Handy's `src-tauri/src/audio_toolkit/vad/smoothed.rs`
//! at commit `af6ec6c903e4c315dbbc395263069b24488596d9` — MIT-licensed;
//! copyright retained. See `features/reintroduce-silero-vad/BLUEPRINT.md`
//! AD-2 for rationale (the hysteresis contract is already correct and
//! the invariants we need; no reason to rewrite it).
//!
//! The three constants [`DEFAULT_PREFILL_FRAMES`], [`DEFAULT_ONSET_FRAMES`],
//! [`DEFAULT_HANGOVER_FRAMES`] encode the BLUEPRINT §"Use case 1"
//! parameter table (120 ms pre-roll / 60 ms onset / 200 ms hangover at
//! 30 ms frame size) as the single source of truth for all three
//! file-based consumers.

use super::{VadFrame, VoiceActivityDetector};
use anyhow::Result;
use std::collections::VecDeque;

/// Pre-roll buffer size in 30 ms frames (= 120 ms). Matches the value
/// Handy used and the number recorded in the PRD.
#[allow(dead_code)] // consumed by Phase 2 callers (R-002 prefilter).
pub const DEFAULT_PREFILL_FRAMES: usize = 4;

/// Consecutive voice-frame count required to open a speech span
/// (= 60 ms at 30 ms framing). Rejects one-off noise / keystrokes.
#[allow(dead_code)]
pub const DEFAULT_ONSET_FRAMES: usize = 2;

/// Silent-frame grace window before closing a speech span
/// (~= 200 ms at 30 ms framing ≈ 7 frames). Prevents chopping
/// mid-word on brief pauses.
#[allow(dead_code)]
pub const DEFAULT_HANGOVER_FRAMES: usize = 7;

#[allow(dead_code)] // wired by R-002 / R-003 / R-004 consumers in Phase 2.
pub struct SmoothedVad {
    inner_vad: Box<dyn VoiceActivityDetector>,
    prefill_frames: usize,
    hangover_frames: usize,
    onset_frames: usize,

    frame_buffer: VecDeque<Vec<f32>>,
    hangover_counter: usize,
    onset_counter: usize,
    in_speech: bool,

    temp_out: Vec<f32>,
}

impl SmoothedVad {
    #[allow(dead_code)] // constructor called by Phase 2 consumers.
    pub fn new(
        inner_vad: Box<dyn VoiceActivityDetector>,
        prefill_frames: usize,
        hangover_frames: usize,
        onset_frames: usize,
    ) -> Self {
        Self {
            inner_vad,
            prefill_frames,
            hangover_frames,
            onset_frames,
            frame_buffer: VecDeque::new(),
            hangover_counter: 0,
            onset_counter: 0,
            in_speech: false,
            temp_out: Vec::new(),
        }
    }
}

impl VoiceActivityDetector for SmoothedVad {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
        // 1. Buffer every incoming frame for possible pre-roll.
        self.frame_buffer.push_back(frame.to_vec());
        while self.frame_buffer.len() > self.prefill_frames + 1 {
            self.frame_buffer.pop_front();
        }

        // 2. Delegate to the wrapped boolean VAD.
        let is_voice = self.inner_vad.is_voice(frame)?;

        match (self.in_speech, is_voice) {
            // Potential start of speech — need to accumulate onset frames.
            (false, true) => {
                self.onset_counter += 1;
                if self.onset_counter >= self.onset_frames {
                    // Enough consecutive voice frames — open a span.
                    self.in_speech = true;
                    self.hangover_counter = self.hangover_frames;
                    self.onset_counter = 0;

                    // Collect prefill + current frame.
                    self.temp_out.clear();
                    for buf in &self.frame_buffer {
                        self.temp_out.extend(buf);
                    }
                    Ok(VadFrame::Speech(&self.temp_out))
                } else {
                    Ok(VadFrame::Noise)
                }
            }

            // Ongoing speech.
            (true, true) => {
                self.hangover_counter = self.hangover_frames;
                Ok(VadFrame::Speech(frame))
            }

            // End of speech or interruption during onset phase.
            (true, false) => {
                if self.hangover_counter > 0 {
                    self.hangover_counter -= 1;
                    Ok(VadFrame::Speech(frame))
                } else {
                    self.in_speech = false;
                    Ok(VadFrame::Noise)
                }
            }

            // Silence or broken onset sequence.
            (false, false) => {
                self.onset_counter = 0;
                Ok(VadFrame::Noise)
            }
        }
    }

    fn reset(&mut self) {
        self.frame_buffer.clear();
        self.hangover_counter = 0;
        self.onset_counter = 0;
        self.in_speech = false;
        self.temp_out.clear();
        self.inner_vad.reset();
    }
}
