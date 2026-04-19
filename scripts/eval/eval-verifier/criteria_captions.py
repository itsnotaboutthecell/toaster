"""
Criteria for the caption-grouping verifier.

A candidate is a grouping of oracle words into display lines:
``lines: [[word_idx, ...], ...]``. We score each candidate on readability,
punctuation respect, line-length balance, and timing coverage.

Criterion IDs are load-bearing — ``backends._mock_score`` dispatches on
them. Keep in sync.
"""

from __future__ import annotations

from typing import Dict, List, Sequence

from verifier_core import Criterion, Trial


GROUND_TRUTH_NOTE = (
    "**IMPORTANT:** Prefer line groupings that (a) break on sentence or "
    "clause boundaries, (b) never split inside a quoted span, (c) keep "
    "average line length in a readable band (about 20-42 chars, <= 7 "
    "words), and (d) cover every oracle word exactly once with no drops "
    "or duplicates. A single huge line or one-word-per-line grouping "
    "must score worse than a balanced split even if it has zero quote "
    "splits."
)


CRITERIA: List[Criterion] = [
    Criterion(
        id="readability",
        name="Readability",
        description=(
            "Lines should have a comfortable reading length: roughly "
            "20-42 characters and <= 7 words each. Penalize both lines "
            "that are too short (choppy) and lines that are too long "
            "(eye-swamp)."
        ),
    ),
    Criterion(
        id="punctuation_respect",
        name="Punctuation Respect",
        description=(
            "Line breaks should fall on sentence or clause boundaries "
            "whenever possible, and must never split inside a quoted "
            "span. A break that splits a quoted phrase is a hard "
            "failure."
        ),
    ),
    Criterion(
        id="line_length_balance",
        name="Line Length Balance",
        description=(
            "Across lines, the character-count distribution should be "
            "balanced. Alternating tiny and huge lines is visually "
            "jarring even if the mean is on target."
        ),
    ),
    Criterion(
        id="timing_coverage",
        name="Timing Coverage",
        description=(
            "Each oracle word must appear in exactly one line. Dropped "
            "or duplicated word indices are an export defect, not a "
            "styling choice."
        ),
    ),
]


def _word_chars(w: dict) -> int:
    return len(str(w.get("text", "")))


def _analyze(fixture: dict, candidate: dict) -> Dict[str, float]:
    oracle = fixture["oracle_words"]
    lines: Sequence[Sequence[int]] = candidate.get("lines", [])

    total_words = len(oracle)
    seen: Dict[int, int] = {}
    for line in lines:
        for idx in line:
            seen[idx] = seen.get(idx, 0) + 1
    missing = sum(1 for i in range(total_words) if i not in seen)
    duplicated = sum(v - 1 for v in seen.values() if v > 1)

    # Char counts per line (sum of word chars + space separators).
    line_char_counts: List[int] = []
    words_per_line: List[int] = []
    for line in lines:
        if not line:
            continue
        chars = sum(_word_chars(oracle[i]) for i in line if 0 <= i < total_words)
        chars += max(0, len(line) - 1)  # inter-word spaces
        line_char_counts.append(chars)
        words_per_line.append(len(line))
    if not line_char_counts:
        line_char_counts = [0]
        words_per_line = [0]

    mean_line_chars = sum(line_char_counts) / len(line_char_counts)
    max_words_per_line = max(words_per_line)

    # Coefficient of variation in line length.
    if len(line_char_counts) > 1 and mean_line_chars > 0:
        var = sum(
            (c - mean_line_chars) ** 2 for c in line_char_counts
        ) / len(line_char_counts)
        cv = (var**0.5) / mean_line_chars
    else:
        cv = 0.0

    # Boundary-end ratio: fraction of lines whose last word is a
    # sentence end OR is the last word overall.
    boundary_ends = 0
    for line in lines:
        if not line:
            continue
        last = line[-1]
        if last == total_words - 1:
            boundary_ends += 1
        elif 0 <= last < total_words and oracle[last].get("is_sentence_end"):
            boundary_ends += 1
    boundary_end_ratio = boundary_ends / len(lines) if lines else 0.0

    # Quote splits: a quoted run is a maximal run of oracle indices with
    # in_quote=true. A split is any line boundary that falls strictly
    # inside such a run.
    quote_splits = 0
    line_end_positions = set()
    cursor = 0
    for line in lines:
        if not line:
            continue
        cursor += len(line)
        line_end_positions.add(cursor - 1)
    # Walk oracle, tracking quoted runs.
    i = 0
    while i < total_words:
        if oracle[i].get("in_quote"):
            j = i
            while j + 1 < total_words and oracle[j + 1].get("in_quote"):
                j += 1
            # Positional indices of the quoted span inside the flattened
            # line sequence, computed from the candidate's lines.
            flat: List[int] = [idx for line in lines for idx in line]
            span_positions = [
                pos for pos, idx in enumerate(flat) if i <= idx <= j
            ]
            for pos in span_positions[:-1]:
                if pos in line_end_positions:
                    quote_splits += 1
            i = j + 1
        else:
            i += 1

    return {
        "total_words": total_words,
        "missing_words": missing,
        "duplicated_words": duplicated,
        "mean_line_chars": mean_line_chars,
        "max_words_per_line": max_words_per_line,
        "line_length_cv": cv,
        "boundary_end_ratio": boundary_end_ratio,
        "quote_splits": quote_splits,
        "line_count": len(line_char_counts),
    }


def build_trial(fixture: dict, candidate: dict) -> Trial:
    stats = _analyze(fixture, candidate)

    trace_lines = [
        f"candidate={candidate['name']}",
        f"fixture={fixture.get('fixture', 'unknown')}",
        f"line_count={int(stats['line_count'])}",
        f"total_words={int(stats['total_words'])}",
        f"missing_words={int(stats['missing_words'])}",
        f"duplicated_words={int(stats['duplicated_words'])}",
        f"mean_line_chars={stats['mean_line_chars']:.1f}",
        f"max_words_per_line={int(stats['max_words_per_line'])}",
        f"line_length_cv={stats['line_length_cv']:.3f}",
        f"boundary_end_ratio={stats['boundary_end_ratio']:.3f}",
        f"quote_splits={int(stats['quote_splits'])}",
        "",
        "Grouped lines:",
    ]
    for k, line in enumerate(candidate.get("lines", [])):
        text = " ".join(
            fixture["oracle_words"][i]["text"]
            for i in line
            if 0 <= i < len(fixture["oracle_words"])
        )
        trace_lines.append(f"  line[{k}] ({len(line)} words): {text!r}")

    problem_lines = [
        f"task=captions/{fixture.get('fixture', 'unknown')}",
        f"oracle_word_count={len(fixture['oracle_words'])}",
        "Oracle words:",
    ]
    for i, w in enumerate(fixture["oracle_words"]):
        tags = []
        if w.get("is_sentence_end"):
            tags.append("sentence_end")
        if w.get("in_quote"):
            tags.append("quoted")
        tag = f" ({','.join(tags)})" if tags else ""
        problem_lines.append(f"  [{i}] {w['text']!r}{tag}")

    return Trial(
        trial_name=candidate["name"],
        reward=0.0,
        problem="\n".join(problem_lines),
        trace="\n".join(trace_lines),
    )
