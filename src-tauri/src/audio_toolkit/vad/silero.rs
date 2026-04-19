//! ORT-direct Silero VAD wrapper.
//!
//! Implements the [`VoiceActivityDetector`] trait against the
//! legacy Silero v4 ONNX I/O contract (inputs: `input:[1,N]f32`,
//! `sr:[1]i64`, `h:[2,1,64]f32`, `c:[2,1,64]f32`; outputs: `output`,
//! `hn`, `cn`). The contract is stable across v4 model revisions and
//! is the shape consumed by [`thewh1teagle/vad-rs`](https://github.com/thewh1teagle/vad-rs)
//! — which is the reference implementation Handy used. We keep that
//! contract so we could swap the model file without code changes.
//!
//! BLUEPRINT.md AD-1 mandates using the `ort` crate directly rather
//! than bringing `vad-rs` back as a dependency: `transcribe-rs`
//! already pulls `ort = 2.0.0-rc.12` into the graph, so reusing it
//! here keeps **one** ONNX runtime, one set of DLL/SO loading quirks,
//! and one upgrade path. The tensor-plumbing cost is the ~100 LOC
//! below.

use anyhow::{anyhow, Result};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::{Shape, Tensor};
use std::path::Path;

use super::{VadFrame, VoiceActivityDetector};

/// Frame duration (ms) the wrapper expects. Matches Handy's choice
/// and keeps the math clean: 30 ms * 16 kHz = 480 samples.
pub const SILERO_FRAME_MS: u32 = 30;

/// Frame size in samples at 16 kHz. Silero v4 accepts variable-length
/// inputs but downstream hysteresis (`SmoothedVad`) depends on a fixed
/// framing cadence — we standardize on 30 ms.
pub const SILERO_FRAME_SAMPLES_16K: usize = 480;

/// Default per-frame speech probability threshold used by R-002 /
/// R-003 / R-004 consumers. Matches the BLUEPRINT `SPEECH_PROB_THRESHOLD`
/// constant and the Silero community default of `0.5`.
#[allow(dead_code)] // wired by Phase 2 consumers (prefilter, boundary, filler).
pub const DEFAULT_SILERO_THRESHOLD: f32 = 0.5;

/// Silero hidden-state dimensionality (v4). 2 layers × 1 batch × 64 hidden.
const STATE_HIDDEN: usize = 64;
const STATE_LEN: usize = 2 * STATE_HIDDEN;

/// Supported sample rates per Silero VAD spec.
const SUPPORTED_SAMPLE_RATES: &[usize] = &[8_000, 16_000];

#[allow(dead_code)] // wired by R-002 / R-003 / R-004 consumers in Phase 2.
pub struct SileroVad {
    session: Session,
    /// LSTM hidden state, laid out as `[2, 1, 64]` row-major. Kept as
    /// an owned `Vec<f32>` rather than an ndarray so we pay the
    /// tensor-construction cost once per `compute` call with zero
    /// reshape copies.
    h_state: Vec<f32>,
    /// LSTM cell state, same shape/layout as `h_state`.
    c_state: Vec<f32>,
    sample_rate_hz: i64,
    threshold: f32,
}

impl SileroVad {
    /// Load the Silero ONNX model from `model_path` and prepare a
    /// stateful session. Returns an error if the path does not exist,
    /// the ONNX runtime fails to initialize, or the sample rate is
    /// outside the supported set — callers must treat the error as a
    /// graceful-absence signal and fall back to the non-VAD path per
    /// BLUEPRINT.md AD-8.
    #[allow(dead_code)] // constructor called by Phase 2 consumers.
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        sample_rate_hz: usize,
        threshold: f32,
    ) -> Result<Self> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(anyhow!(
                "threshold must be in [0.0, 1.0], got {threshold}"
            ));
        }
        if !SUPPORTED_SAMPLE_RATES.contains(&sample_rate_hz) {
            return Err(anyhow!(
                "Silero VAD supports only 8 kHz or 16 kHz, got {sample_rate_hz}"
            ));
        }
        let path = model_path.as_ref();
        if !path.exists() {
            return Err(anyhow!(
                "Silero VAD model not found at {}",
                path.display()
            ));
        }

        let session = Session::builder()
            .map_err(|e| anyhow!("ort Session::builder failed: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow!("ort optimization level set failed: {e}"))?
            .with_intra_threads(1)
            .map_err(|e| anyhow!("ort intra_threads failed: {e}"))?
            .with_inter_threads(1)
            .map_err(|e| anyhow!("ort inter_threads failed: {e}"))?
            .commit_from_file(path)
            .map_err(|e| anyhow!("ort commit_from_file failed: {e}"))?;

        Ok(Self {
            session,
            h_state: vec![0.0; STATE_LEN],
            c_state: vec![0.0; STATE_LEN],
            sample_rate_hz: sample_rate_hz as i64,
            threshold,
        })
    }

    /// Run one inference pass, returning the raw speech probability
    /// in [0, 1]. Updates the stateful LSTM hidden tensors. Intended
    /// for callers that want access to the probability curve (R-003
    /// boundary refinement, R-004 filler classifier); R-002 uses the
    /// `VadFrame` API via the trait impl below.
    #[allow(dead_code)] // consumed by Phase 2 callers.
    pub fn compute(&mut self, samples: &[f32]) -> Result<f32> {
        if samples.is_empty() {
            return Err(anyhow!("empty frame"));
        }

        let input_tensor = Tensor::<f32>::from_array((
            Shape::new([1i64, samples.len() as i64]),
            samples.to_vec(),
        ))
        .map_err(|e| anyhow!("failed to build input tensor: {e}"))?;
        let sr_tensor = Tensor::<i64>::from_array((Shape::new([1i64]), vec![self.sample_rate_hz]))
            .map_err(|e| anyhow!("failed to build sr tensor: {e}"))?;
        let h_tensor = Tensor::<f32>::from_array((
            Shape::new([2i64, 1, STATE_HIDDEN as i64]),
            self.h_state.clone(),
        ))
        .map_err(|e| anyhow!("failed to build h tensor: {e}"))?;
        let c_tensor = Tensor::<f32>::from_array((
            Shape::new([2i64, 1, STATE_HIDDEN as i64]),
            self.c_state.clone(),
        ))
        .map_err(|e| anyhow!("failed to build c tensor: {e}"))?;

        let outputs = self
            .session
            .run(ort::inputs![
                "input" => input_tensor,
                "sr" => sr_tensor,
                "h" => h_tensor,
                "c" => c_tensor,
            ])
            .map_err(|e| anyhow!("Silero VAD inference failed: {e}"))?;

        // Update LSTM hidden state for the next call. `try_extract_tensor`
        // returns `(&Shape, &[T])` in ort rc.12; we only need the data
        // slice and assume the ONNX honors the documented [2,1,64] shape.
        let (_, hn) = outputs
            .get("hn")
            .ok_or_else(|| anyhow!("Silero ONNX did not return `hn`"))?
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("failed to extract hn: {e}"))?;
        if hn.len() != STATE_LEN {
            return Err(anyhow!("hn length {} != expected {STATE_LEN}", hn.len()));
        }
        self.h_state.copy_from_slice(hn);

        let (_, cn) = outputs
            .get("cn")
            .ok_or_else(|| anyhow!("Silero ONNX did not return `cn`"))?
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("failed to extract cn: {e}"))?;
        if cn.len() != STATE_LEN {
            return Err(anyhow!("cn length {} != expected {STATE_LEN}", cn.len()));
        }
        self.c_state.copy_from_slice(cn);

        let (_, out) = outputs
            .get("output")
            .ok_or_else(|| anyhow!("Silero ONNX did not return `output`"))?
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("failed to extract output: {e}"))?;
        let prob = out
            .first()
            .copied()
            .ok_or_else(|| anyhow!("Silero output tensor was empty"))?;

        Ok(prob)
    }
}

impl VoiceActivityDetector for SileroVad {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
        let prob = self.compute(frame)?;
        if prob > self.threshold {
            Ok(VadFrame::Speech(frame))
        } else {
            Ok(VadFrame::Noise)
        }
    }

    fn reset(&mut self) {
        self.h_state.fill(0.0);
        self.c_state.fill(0.0);
    }
}
