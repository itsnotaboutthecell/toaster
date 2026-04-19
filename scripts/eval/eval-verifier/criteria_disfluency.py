"""
Criteria for the disfluency-cleanup verifier.

Toaster's cleanup logic has to make two entangled decisions when it sees
a repetition like "the the best best part":

  1. Group collapse: each repetition group should collapse to exactly
     one survivor (not zero, not two).
  2. Survivor selection: the survivor should be the audibly clearer
     take, so the final edit sounds like a confident read rather than a
     mumble.

These are distinct concerns: a candidate can score perfectly on (1) and
still ship a bad edit if (2) is wrong. This module adds four criteria
that separate those axes, plus the audio-aware splice-cleanliness
criterion that every cleanup path needs.

Criterion IDs are load-bearing — ``backends._mock_score`` dispatches on
them. Keep in sync.
"""

from __future__ import annotations

from typing import Dict, List, Sequence

from verifier_core import Criterion, Trial

try:
    from audio_features import articulation_score, seam_silence_ratio
except Exception:  # pragma: no cover
    articulation_score = None  # type: ignore
    seam_silence_ratio = None  # type: ignore


GROUND_TRUTH_NOTE = (
    "**IMPORTANT:** For each disfluency group (same stem repeated back-to-"
    "back), the candidate must keep EXACTLY ONE member — the clearest one "
    "by the audio. A candidate that collapses a group but picks the "
    "mumbled take is WORSE than a candidate that keeps both members "
    "unchanged, because a mumbled survivor is a product-visible defect. "
    "Splice points must fall in silence; cuts that slice into voiced "
    "speech are click-prone."
)


CRITERIA: List[Criterion] = [
    Criterion(
        id="group_collapse_completeness",
        name="Group Collapse Completeness",
        description=(
            "Every repetition group should end up with exactly one "
            "surviving member. Zero survivors (dropped content) and "
            "more than one survivor (incomplete collapse) are both "
            "failures. Content words outside any group must be kept."
        ),
    ),
    Criterion(
        id="survivor_clarity",
        name="Survivor Audio Clarity",
        description=(
            "For each collapsed group, the surviving member should be "
            "the one with the highest articulation score — louder, "
            "fuller, not mumbled. This is scored from the real audio "
            "(peak dBFS, RMS dBFS, silence ratio). Picking the mumbled "
            "take when a clear take is available must score worst."
        ),
    ),
    Criterion(
        id="cut_placement_cleanliness",
        name="Cut Placement Cleanliness",
        description=(
            "Every deletion creates two splice seams (at the start and "
            "end of the deleted word). Seams should fall inside silent "
            "regions, not voiced speech — otherwise the edit clicks. "
            "Measured by the silence ratio of a 40 ms window centered "
            "on each seam. Neutral (0.5) when no audio is attached."
        ),
    ),
    Criterion(
        id="pacing_agreement",
        name="Pacing Agreement With Human Oracle",
        description=(
            "Beyond fillers and repetition collapse, a good cleanup is "
            "willing to drop non-essential content words (sentence-"
            "initial 'And', hedge phrases like 'kind of', 'like', 'you "
            "know') to tighten pacing. This criterion compares the "
            "candidate's kept/deleted decisions on every NON-filler, "
            "NON-group word against a human-provided oracle labeling. "
            "Score = (correct matches) / (scoreable words). Fixtures "
            "without an oracle score neutral 1.0 so this criterion is "
            "silent when no ground truth is available."
        ),
    ),
    Criterion(
        id="timing_monotonicity",
        name="Timing Monotonicity",
        description=(
            "Kept indices must be strictly increasing, and the "
            "underlying oracle timings must remain non-overlapping."
        ),
    ),
]


# ---------------------------------------------------------------------------
# Trial construction
# ---------------------------------------------------------------------------


def _groups(fixture: dict) -> List[dict]:
    if "groups" in fixture:
        return list(fixture["groups"])
    # Derive groups from ``group_id`` on oracle_words if absent.
    buckets: Dict[str, List[int]] = {}
    for i, w in enumerate(fixture["oracle_words"]):
        gid = w.get("group_id")
        if gid:
            buckets.setdefault(gid, []).append(i)
    return [{"id": gid, "member_indices": idxs} for gid, idxs in buckets.items()]


def _analyze(fixture: dict, candidate: dict) -> Dict[str, float]:
    oracle = fixture["oracle_words"]
    kept = list(candidate.get("kept_indices", []))
    kept_set = set(kept)
    total = len(oracle)
    groups = _groups(fixture)

    # --- Group collapse ----------------------------------------------------
    groups_total = len(groups)
    groups_ok = 0
    groups_with_zero = 0
    groups_with_many = 0
    for g in groups:
        survivors = sum(1 for i in g["member_indices"] if i in kept_set)
        if survivors == 1:
            groups_ok += 1
        elif survivors == 0:
            groups_with_zero += 1
        else:
            groups_with_many += 1

    # Non-grouped content words must all survive.
    grouped_indices = {i for g in groups for i in g["member_indices"]}
    non_group_total = sum(1 for i in range(total) if i not in grouped_indices)
    non_group_kept = sum(
        1 for i in range(total) if i not in grouped_indices and i in kept_set
    )

    # --- Monotonicity ------------------------------------------------------
    kept_monotonic = kept == sorted(kept)
    last_end = -1
    for idx in kept:
        if not 0 <= idx < total:
            kept_monotonic = False
            break
        s = int(oracle[idx]["start_us"])
        e = int(oracle[idx]["end_us"])
        if s < last_end or e <= s:
            kept_monotonic = False
        last_end = e

    # --- Survivor clarity (audio-aware) ------------------------------------
    audio_path = fixture.get("audio_path")
    survivor_clarity_num = 0.0
    survivor_clarity_den = 0.0
    if audio_path and articulation_score is not None and groups:
        for g in groups:
            members = g["member_indices"]
            survivors = [i for i in members if i in kept_set]
            if len(survivors) != 1:
                # Not a valid collapse; clarity is undefined for this group.
                continue
            survivor = survivors[0]
            scores = [
                articulation_score(
                    audio_path,
                    int(oracle[i]["start_us"]),
                    int(oracle[i]["end_us"]),
                )
                for i in members
            ]
            max_score = max(scores)
            if max_score <= 0:
                continue
            # Proportional credit: 1.0 if we kept the clearest, lower if
            # we kept a mumbled one.
            survivor_clarity_num += scores[members.index(survivor)] / max_score
            survivor_clarity_den += 1.0

    clarity_ratio = (
        survivor_clarity_num / survivor_clarity_den
        if survivor_clarity_den > 0
        else 0.0
    )

    # --- Splice cleanliness (audio-aware) ----------------------------------
    seam_silences: List[float] = []
    if audio_path and seam_silence_ratio is not None:
        deleted = [i for i in range(total) if i not in kept_set]
        for i in deleted:
            seam_silences.append(
                seam_silence_ratio(audio_path, int(oracle[i]["start_us"]))
            )
            seam_silences.append(
                seam_silence_ratio(audio_path, int(oracle[i]["end_us"]))
            )
    mean_seam_silence = (
        sum(seam_silences) / len(seam_silences) if seam_silences else 0.0
    )

    # --- Pacing agreement vs. human oracle ---------------------------------
    # Only scored on NON-filler, NON-grouped words. The human oracle
    # records `label: keep|delete` per word; we measure agreement.
    pacing_scoreable = 0
    pacing_correct = 0
    pacing_correct_cuts = 0
    pacing_missed_cuts = 0
    pacing_overcuts = 0
    human_cut_labels = 0
    grouped = grouped_indices
    for i, w in enumerate(oracle):
        if i in grouped:
            continue
        if bool(w.get("is_filler")):
            continue
        human_label = w.get("human_label")
        if human_label not in ("keep", "delete"):
            continue
        pacing_scoreable += 1
        candidate_keeps = i in kept_set
        human_keeps = human_label == "keep"
        if human_label == "delete":
            human_cut_labels += 1
        if candidate_keeps == human_keeps:
            pacing_correct += 1
            if not human_keeps:
                pacing_correct_cuts += 1
        else:
            if human_keeps and not candidate_keeps:
                pacing_overcuts += 1  # candidate cut something human kept
            else:
                pacing_missed_cuts += 1  # candidate kept something human cut
    pacing_agreement = (
        pacing_correct / pacing_scoreable if pacing_scoreable > 0 else 1.0
    )
    pacing_has_oracle = pacing_scoreable > 0

    return {
        "groups_total": groups_total,
        "groups_ok": groups_ok,
        "groups_with_zero": groups_with_zero,
        "groups_with_many": groups_with_many,
        "non_group_total": non_group_total,
        "non_group_kept": non_group_kept,
        "kept_monotonic": kept_monotonic,
        "clarity_ratio": clarity_ratio,
        "clarity_groups_scored": int(survivor_clarity_den),
        "mean_seam_silence": mean_seam_silence,
        "seam_samples": len(seam_silences),
        "pacing_agreement": pacing_agreement,
        "pacing_scoreable": pacing_scoreable,
        "pacing_correct": pacing_correct,
        "pacing_correct_cuts": pacing_correct_cuts,
        "pacing_missed_cuts": pacing_missed_cuts,
        "pacing_overcuts": pacing_overcuts,
        "pacing_has_oracle": pacing_has_oracle,
        "human_cut_labels": human_cut_labels,
        "audio_present": bool(
            audio_path and articulation_score is not None
        ),
    }


def build_trial(fixture: dict, candidate: dict) -> Trial:
    stats = _analyze(fixture, candidate)

    trace_lines = [
        f"candidate={candidate['name']}",
        f"fixture={fixture.get('fixture', 'unknown')}",
        f"kept_count={len(candidate.get('kept_indices', []))}",
        f"groups_total={int(stats['groups_total'])}",
        f"groups_ok={int(stats['groups_ok'])}",
        f"groups_with_zero={int(stats['groups_with_zero'])}",
        f"groups_with_many={int(stats['groups_with_many'])}",
        f"non_group_total={int(stats['non_group_total'])}",
        f"non_group_kept={int(stats['non_group_kept'])}",
        f"kept_monotonic={'true' if stats['kept_monotonic'] else 'false'}",
        f"audio_present={'true' if stats['audio_present'] else 'false'}",
        f"clarity_ratio={stats['clarity_ratio']:.3f}",
        f"clarity_groups_scored={int(stats['clarity_groups_scored'])}",
        f"mean_seam_silence={stats['mean_seam_silence']:.3f}",
        f"seam_samples={int(stats['seam_samples'])}",
        f"pacing_has_oracle={'true' if stats['pacing_has_oracle'] else 'false'}",
        f"pacing_agreement={stats['pacing_agreement']:.3f}",
        f"pacing_scoreable={int(stats['pacing_scoreable'])}",
        f"pacing_correct={int(stats['pacing_correct'])}",
        f"pacing_correct_cuts={int(stats['pacing_correct_cuts'])}",
        f"pacing_missed_cuts={int(stats['pacing_missed_cuts'])}",
        f"pacing_overcuts={int(stats['pacing_overcuts'])}",
        f"human_cut_labels={int(stats['human_cut_labels'])}",
        "",
        "Kept words (in order):",
    ]
    for idx in candidate.get("kept_indices", []):
        if 0 <= idx < len(fixture["oracle_words"]):
            w = fixture["oracle_words"][idx]
            gid = w.get("group_id") or "-"
            trace_lines.append(
                f"  [{idx}] {w['text']!r} group={gid}  "
                f"[{w['start_us']}us -> {w['end_us']}us]"
            )

    problem_lines = [
        f"task=disfluency/{fixture.get('fixture', 'unknown')}",
        f"oracle_word_count={len(fixture['oracle_words'])}",
        f"desired_kept_indices={fixture.get('desired_kept_indices', [])}",
        "Repetition groups:",
    ]
    for g in _groups(fixture):
        members = ", ".join(
            f"[{i}]{fixture['oracle_words'][i]['text']!r}"
            for i in g["member_indices"]
        )
        problem_lines.append(f"  {g['id']}: {members}")
    problem_lines.append("Oracle words:")
    for i, w in enumerate(fixture["oracle_words"]):
        gid = w.get("group_id") or "-"
        hint = w.get("clarity_hint")
        hint_txt = f" clarity_hint={hint}" if hint is not None else ""
        problem_lines.append(
            f"  [{i}] {w['text']!r} group={gid}{hint_txt}  "
            f"[{w['start_us']}us -> {w['end_us']}us]"
        )

    return Trial(
        trial_name=candidate["name"],
        reward=0.0,
        problem="\n".join(problem_lines),
        trace="\n".join(trace_lines),
    )
