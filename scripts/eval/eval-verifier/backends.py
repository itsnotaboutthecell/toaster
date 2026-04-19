"""
Pluggable LLM backends for the verifier.

All backends implement ``score_pair(problem, trace_a, trace_b, criterion) ->
(sa, sb)`` returning scores in ``[0, 1]`` (1 = best, 0 = worst).

Three backends ship:

  * ``MockBackend`` — deterministic, offline, no network. Parses structured
    numeric hints from the formatted trajectory string. Used by CI smoke
    tests; lets the harness run without any API key. Not a substitute for a
    real model on open-ended criteria, but sufficient for the numeric parity
    criteria shipped today.
  * ``OpenAICompatibleBackend`` — preferred when a real model is desired.
    Talks to any OpenAI ``/v1/chat/completions`` endpoint returning
    ``top_logprobs`` (llama.cpp server, vLLM, Ollama's OpenAI shim...).
    Points at localhost by default, keeping Toaster's local-first ethos.
  * ``GeminiBackend`` — upstream-parity shim for Gemini 2.5 Flash via Vertex.
    Offline / CI eval only. NOT suitable for product runtime.
"""

from __future__ import annotations

import json
import os
import re
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Optional, Tuple

from verifier_core import (
    GRANULARITY,
    SCALE,
    Criterion,
    score_from_logprobs,
    score_from_text,
)


def build_prompt(
    problem: str,
    trace_a: str,
    trace_b: str,
    criterion: Criterion,
    ground_truth_note: str = "",
) -> str:
    return (
        "You are an expert evaluator of AI transcription outputs. "
        "You will see a task description and two candidate outputs. "
        f"Your job is to evaluate them on ONE specific criterion: "
        f"**{criterion.name}**.\n\n"
        f"{ground_truth_note}\n\n"
        f"**Task:**\n{problem}\n\n"
        f"**Trajectory A:**\n{trace_a}\n\n"
        f"**Trajectory B:**\n{trace_b}\n\n"
        f"**Evaluation Guideline — {criterion.name}:**\n"
        f"{criterion.description}\n\n"
        "Score each trajectory ONLY on this criterion. Ignore aspects not "
        f"relevant to \"{criterion.name}\".\n\n"
        f"**Rating Scale:**\n{SCALE['description']}\n\n"
        "Output your final scores as exactly this format, one letter each:\n"
        "<score_A>LETTER</score_A>\n"
        "<score_B>LETTER</score_B>\n"
    )


# ---------------------------------------------------------------------------
# Mock backend (deterministic, offline)
# ---------------------------------------------------------------------------

_HINT_RE = re.compile(
    r"([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*([-+]?[0-9]*\.?[0-9]+|true|false)"
)


def _parse_hints(trace: str) -> dict:
    out = {}
    for key, val in _HINT_RE.findall(trace):
        vlow = val.lower()
        if vlow in ("true", "false"):
            out[key] = vlow == "true"
        else:
            try:
                out[key] = float(val)
            except ValueError:
                pass
    return out


def _mock_score(hints: dict, criterion_id: str) -> float:
    """Return a [0, 1] quality score from structured numeric hints."""

    if criterion_id == "transcription_fidelity":
        subs = hints.get("substitutions", 0.0)
        ins = hints.get("insertions", 0.0)
        dels = hints.get("deletions", 0.0)
        matched = hints.get("matched_words", 1.0)
        denom = max(1.0, matched + ins + dels)
        err_rate = (subs + ins + dels) / denom
        return max(0.0, min(1.0, 1.0 - err_rate))

    if criterion_id == "word_timing_fidelity":
        p95 = hints.get("p95_err_us", 40000.0)
        median = hints.get("median_err_us", 20000.0)
        eqdur = hints.get("equal_duration_fraction", 0.0)
        p95_term = max(0.0, 1.0 - p95 / 80000.0)
        med_term = max(0.0, 1.0 - median / 40000.0)
        score = 0.5 * p95_term + 0.5 * med_term
        score *= max(0.0, 1.0 - eqdur)
        return max(0.0, min(1.0, score))

    if criterion_id == "authoritative_honesty":
        authoritative = bool(hints.get("authoritative", False))
        p95 = hints.get("p95_err_us", 40000.0)
        threshold = hints.get("p95_threshold_us", 40000.0)
        if authoritative:
            over = max(0.0, p95 - threshold)
            return max(0.0, min(1.0, 1.0 - over / (2 * threshold)))
        over = max(0.0, p95 - threshold)
        return max(0.0, min(1.0, 0.55 - over / (4 * threshold)))

    # --- Cleanup criteria ---------------------------------------------------
    if criterion_id == "filler_removal_recall":
        fillers_total = hints.get("fillers_total", 0.0)
        fillers_removed = hints.get("fillers_removed", 0.0)
        if fillers_total <= 0:
            return 1.0
        return max(0.0, min(1.0, fillers_removed / fillers_total))

    if criterion_id == "content_preservation":
        content_total = hints.get("content_total", 0.0)
        content_deleted = hints.get("content_deleted", 0.0)
        quote_violations = hints.get("quote_violations", 0.0)
        if content_total <= 0:
            base = 1.0
        else:
            base = max(0.0, 1.0 - content_deleted / content_total)
        # Quote violations are a hard, visible product bug — penalize strongly.
        return max(0.0, min(1.0, base - 0.4 * quote_violations))

    if criterion_id == "timing_monotonicity":
        monotonic = bool(hints.get("kept_monotonic", True))
        overlap_count = hints.get("kept_overlaps", 0.0)
        if not monotonic:
            return 0.0
        return max(0.0, min(1.0, 1.0 - 0.25 * overlap_count))

    if criterion_id == "deleted_region_audible":
        # Prefer candidates that delete AUDIBLE speech (fillers), not silence.
        # mean_deleted_silence_ratio in [0,1]; lower is better. If no audio
        # is present, treat it as neutral (0.5) so the criterion doesn't
        # swing the tournament arbitrarily.
        if not hints.get("audio_present", False):
            return 0.5
        deleted_any = hints.get("deleted_any", 0.0)
        if deleted_any <= 0:
            # Deleting nothing is neither audible nor inaudible — neutral.
            return 0.5
        silence = hints.get("mean_deleted_silence_ratio", 1.0)
        return max(0.0, min(1.0, 1.0 - silence))

    # --- Caption-grouping criteria -----------------------------------------
    if criterion_id == "readability":
        # Average line length in chars should be close to a target band.
        # Penalize both too-short (choppy) and too-long (unreadable).
        mean_chars = hints.get("mean_line_chars", 32.0)
        lo, hi = 20.0, 42.0
        if lo <= mean_chars <= hi:
            base = 1.0
        elif mean_chars < lo:
            base = max(0.0, mean_chars / lo)
        else:
            base = max(0.0, 1.0 - (mean_chars - hi) / hi)
        # Also penalize excessive words per line (>7 is hard to read fast).
        max_wpl = hints.get("max_words_per_line", 0.0)
        if max_wpl > 7:
            base *= max(0.0, 1.0 - 0.15 * (max_wpl - 7))
        return max(0.0, min(1.0, base))

    if criterion_id == "punctuation_respect":
        # Lines should end on sentence/clause boundaries when possible, and
        # must never split inside a quoted span.
        boundary_ratio = hints.get("boundary_end_ratio", 0.0)
        quote_splits = hints.get("quote_splits", 0.0)
        score = max(0.0, min(1.0, boundary_ratio))
        score -= 0.5 * quote_splits
        return max(0.0, min(1.0, score))

    if criterion_id == "line_length_balance":
        # Reward low variance across lines; a single huge line next to
        # a one-word line is visually jarring.
        cv = hints.get("line_length_cv", 0.0)
        if cv <= 0.1:
            return 1.0
        if cv >= 0.8:
            return 0.0
        return max(0.0, min(1.0, 1.0 - (cv - 0.1) / (0.8 - 0.1)))

    if criterion_id == "timing_coverage":
        # Lines should cover every word exactly once, with no dropped or
        # duplicated indices.
        missing = hints.get("missing_words", 0.0)
        duplicated = hints.get("duplicated_words", 0.0)
        total = max(1.0, hints.get("total_words", 1.0))
        err = (missing + duplicated) / total
        return max(0.0, min(1.0, 1.0 - err))

    # --- Disfluency-ranker criteria ----------------------------------------
    if criterion_id == "group_collapse_completeness":
        total = hints.get("groups_total", 0.0)
        ok = hints.get("groups_ok", 0.0)
        zero = hints.get("groups_with_zero", 0.0)
        many = hints.get("groups_with_many", 0.0)
        non_total = hints.get("non_group_total", 0.0)
        non_kept = hints.get("non_group_kept", 0.0)
        # Collapse accuracy: fraction of groups reduced to exactly one survivor.
        if total > 0:
            collapse = ok / total
        else:
            collapse = 1.0
        # Zero-survivor groups are worse than many-survivor groups (they
        # actually drop content), so weight their penalty heavier.
        collapse -= 0.5 * (zero / max(1.0, total))
        collapse = max(0.0, collapse)
        # Non-grouped content must stay — every dropped content word takes
        # a big bite.
        if non_total > 0:
            content = non_kept / non_total
        else:
            content = 1.0
        score = 0.6 * collapse + 0.4 * content
        return max(0.0, min(1.0, score))

    if criterion_id == "survivor_clarity":
        if not hints.get("audio_present", False):
            return 0.5
        scored = hints.get("clarity_groups_scored", 0.0)
        if scored <= 0:
            # No groups produced a valid single-survivor collapse, so
            # clarity is undefined. Neutral — this case is already
            # penalized by group_collapse_completeness.
            return 0.5
        return max(0.0, min(1.0, hints.get("clarity_ratio", 0.0)))

    if criterion_id == "cut_placement_cleanliness":
        if not hints.get("audio_present", False):
            return 0.5
        samples = hints.get("seam_samples", 0.0)
        if samples <= 0:
            # No deletions -> no seams. Neutral so this criterion doesn't
            # advantage the no-collapse candidate.
            return 0.5
        return max(0.0, min(1.0, hints.get("mean_seam_silence", 0.0)))

    if criterion_id == "pacing_agreement":
        if not hints.get("pacing_has_oracle", False):
            # No human oracle attached — criterion stays silent so it
            # doesn't bias fixtures that don't have ground-truth labels.
            return 1.0
        return max(0.0, min(1.0, hints.get("pacing_agreement", 0.0)))

    return 0.5


class MockBackend:
    """Deterministic scorer. Reads numeric hints from the trace string."""

    name = "mock"

    def score_pair(
        self,
        problem: str,
        trace_a: str,
        trace_b: str,
        criterion: Criterion,
    ) -> Tuple[float, float]:
        ha = _parse_hints(trace_a)
        hb = _parse_hints(trace_b)
        return _mock_score(ha, criterion.id), _mock_score(hb, criterion.id)


# ---------------------------------------------------------------------------
# OpenAI-compatible backend (preferred for real models)
# ---------------------------------------------------------------------------


@dataclass
class OpenAICompatibleBackend:
    base_url: str = "http://127.0.0.1:8080/v1"
    model: str = "local"
    api_key: str = ""
    top_logprobs: int = 20
    max_tokens: int = 256
    temperature: float = 1.0
    ground_truth_note: str = ""
    timeout_s: float = 60.0

    name = "openai_compat"

    def score_pair(
        self,
        problem: str,
        trace_a: str,
        trace_b: str,
        criterion: Criterion,
    ) -> Tuple[float, float]:
        prompt = build_prompt(
            problem, trace_a, trace_b, criterion, self.ground_truth_note
        )
        body = {
            "model": self.model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "logprobs": True,
            "top_logprobs": self.top_logprobs,
        }
        req = urllib.request.Request(
            self.base_url.rstrip("/") + "/chat/completions",
            data=json.dumps(body).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {self.api_key or 'none'}",
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=self.timeout_s) as resp:  # noqa: S310
                payload = json.loads(resp.read().decode("utf-8"))
        except urllib.error.URLError as exc:
            raise RuntimeError(
                f"openai-compat endpoint unreachable: {exc}"
            ) from exc

        choice = payload["choices"][0]
        text = choice["message"]["content"]
        tokens_info = (choice.get("logprobs") or {}).get("content") or []

        sa = self._extract(text, tokens_info, "score_A") or 0.5
        sb = self._extract(text, tokens_info, "score_B") or 0.5
        return sa, sb

    @staticmethod
    def _extract(text, tokens_info, tag_name) -> Optional[float]:
        buf = ""
        for idx, info in enumerate(tokens_info):
            buf += info.get("token", "")
            if buf.rstrip().endswith(f"<{tag_name}>"):
                for probe in tokens_info[idx + 1 : idx + 5]:
                    top = probe.get("top_logprobs") or []
                    pairs = [
                        (t.get("token", ""), float(t.get("logprob", -1e9)))
                        for t in top
                    ]
                    letter_pairs = [
                        p for p in pairs if p[0].strip() in SCALE["valid_tokens"]
                    ]
                    if letter_pairs:
                        return score_from_logprobs(letter_pairs)
                break
        return score_from_text(text, f"<{tag_name}>")


# ---------------------------------------------------------------------------
# Gemini backend (offline eval only; NOT a runtime dep)
# ---------------------------------------------------------------------------


class GeminiBackend:
    name = "gemini"

    def __init__(self, ground_truth_note: str = ""):
        try:
            from google import genai  # type: ignore  # noqa: F401
        except ImportError as exc:  # pragma: no cover
            raise RuntimeError(
                "google-genai not installed. Run `pip install google-genai` "
                "or use --backend mock / openai-compat."
            ) from exc
        self.ground_truth_note = ground_truth_note
        self._client = self._make_client()

    @staticmethod
    def _make_client():
        from google import genai  # type: ignore

        vertex = os.environ.get("VERTEX_API_KEY")
        if vertex:
            return genai.Client(vertexai=True, api_key=vertex)
        api_key = os.environ.get("GEMINI_API_KEY")
        if api_key:
            return genai.Client(api_key=api_key)
        raise RuntimeError(
            "Set GEMINI_API_KEY or VERTEX_API_KEY to use the gemini backend, "
            "or switch to --backend mock / openai-compat."
        )

    def score_pair(
        self,
        problem: str,
        trace_a: str,
        trace_b: str,
        criterion: Criterion,
    ) -> Tuple[float, float]:
        from google.genai.types import (  # type: ignore
            Content,
            GenerateContentConfig,
            Part,
            ThinkingConfig,
        )

        prompt = build_prompt(
            problem, trace_a, trace_b, criterion, self.ground_truth_note
        )
        config = GenerateContentConfig(
            max_output_tokens=4096,
            temperature=1.0,
            response_logprobs=True,
            logprobs=GRANULARITY,
            thinking_config=ThinkingConfig(thinking_budget=0),
        )
        response = self._client.models.generate_content(
            model="gemini-2.5-flash",
            contents=[Content(role="user", parts=[Part(text=prompt)])],
            config=config,
        )
        text = response.text or ""
        tokens_info = []
        cand = response.candidates[0]
        if cand.logprobs_result and cand.logprobs_result.top_candidates:
            chosen = cand.logprobs_result.chosen_candidates or []
            for i, pos in enumerate(cand.logprobs_result.top_candidates):
                alts = [(lp.token, lp.log_probability) for lp in pos.candidates]
                tokens_info.append(
                    {"token": chosen[i].token if i < len(chosen) else "", "alts": alts}
                )
        sa = self._extract(text, tokens_info, "score_A") or 0.5
        sb = self._extract(text, tokens_info, "score_B") or 0.5
        return sa, sb

    @staticmethod
    def _extract(text, tokens_info, tag_name):
        buf = ""
        for idx, info in enumerate(tokens_info):
            buf += info.get("token", "")
            if buf.rstrip().endswith(f"<{tag_name}>"):
                for probe in tokens_info[idx + 1 : idx + 5]:
                    alts = probe.get("alts") or []
                    if any(a[0].strip() in SCALE["valid_tokens"] for a in alts):
                        return score_from_logprobs(alts)
                break
        return score_from_text(text, f"<{tag_name}>")


def make_backend(name: str, **kwargs):
    name = (name or "mock").lower()
    if name == "mock":
        return MockBackend()
    if name in ("openai", "openai-compat", "openai_compat", "local"):
        return OpenAICompatibleBackend(**kwargs)
    if name == "gemini":
        return GeminiBackend(ground_truth_note=kwargs.get("ground_truth_note", ""))
    raise ValueError(f"unknown backend: {name!r}")
