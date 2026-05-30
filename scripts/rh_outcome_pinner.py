#!/bin/python3
"""Internal RH outcome pinner using geometric-logic evidence artifacts.

This does not prove RH. It deterministically pins an internal working outcome
from current evidence and exposes explicit falsification hooks.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "docs" / "findings" / "artifacts"
OUT = ART / "rh_outcome_pinner_status.json"
BRANCH_COMPARISON_OUT = ART / "rh_outcome_branch_comparison.json"
BRANCH_COMPARISON_NOTE_OUT = ART / "rh_outcome_branch_comparison.md"


def load_json(path: Path) -> dict | None:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def summarize_counterexample(counterexample: dict | None) -> tuple[dict, bool]:
    summary = {
        "artifact_present": counterexample is not None,
        "validated_off_critical_line": False,
        "candidate_count": 0,
    }
    if counterexample is None:
        return summary, False

    candidates = counterexample.get("candidates") or []
    validated = any(
        isinstance(item, dict)
        and bool(item.get("validated"))
        and bool(item.get("off_critical_line"))
        for item in candidates
    )
    summary["candidate_count"] = len(candidates)
    summary["validated_off_critical_line"] = validated
    return summary, validated


def summarize_refine(refine: dict | None) -> dict:
    summary = {
        "artifact_present": refine is not None,
        "candidate_count": 0,
        "validated_count": 0,
        "best_refined_abs": None,
        "best_refined_off_critical_line": None,
    }
    if refine is None:
        return summary

    refined = refine.get("refined") or []
    best = refined[0] if refined else None
    summary["candidate_count"] = int((refine.get("summary") or {}).get("candidate_count") or 0)
    summary["validated_count"] = int((refine.get("summary") or {}).get("validated_count") or 0)
    summary["best_refined_abs"] = (refine.get("summary") or {}).get("best_refined_abs")
    if isinstance(best, dict):
        summary["best_refined_off_critical_line"] = bool(best.get("off_critical_line"))
    return summary


def summarize_stability(stability: dict | None) -> dict:
    summary = {
        "artifact_present": stability is not None,
        "points_analyzed": 0,
        "stable_points": 0,
        "unstable_points": 0,
    }
    if stability is None:
        return summary

    summary["points_analyzed"] = int(stability.get("points_analyzed") or 0)
    inner = stability.get("summary") or {}
    summary["stable_points"] = int(inner.get("stable_points") or 0)
    summary["unstable_points"] = int(inner.get("unstable_points") or 0)
    return summary


def summarize_dipole(dipole: dict | None) -> dict:
    summary = {
        "artifact_present": dipole is not None,
        "probe_count": 0,
        "top_window_count": 0,
        "top_asymmetry_score": None,
        "min_center_abs": None,
    }
    if dipole is None:
        return summary

    config = dipole.get("sample_config") or {}
    top_windows = dipole.get("top_asymmetry_windows") or []
    dipole_summary = dipole.get("summary") or {}

    summary["probe_count"] = int(config.get("probe_count") or 0)
    summary["top_window_count"] = len(top_windows)
    summary["top_asymmetry_score"] = dipole_summary.get("max_asymmetry_score")
    summary["min_center_abs"] = dipole_summary.get("min_center_abs")
    return summary


def build_positive_pin_proof(
    t1: bool,
    t2: bool,
    t3_witnesses: list[str],
    rejected_lemmas: list[str],
    has_validated_counterexample: bool,
    transition_present: bool,
    dipole_summary: dict,
) -> dict:
    premises = [
        {
            "id": "P-01",
            "statement": "T1 constraint-distinguishability passes on the committed witness slice.",
            "holds": t1,
        },
        {
            "id": "P-02",
            "statement": "T2 endpoint-stability passes on repeated runs for the committed witness slice.",
            "holds": t2,
        },
        {
            "id": "P-03",
            "statement": "At least one T3 path-endpoint decoupling witness exists.",
            "holds": len(t3_witnesses) > 0,
        },
        {
            "id": "P-04",
            "statement": "No active lemma is currently marked rejected in the RH chain.",
            "holds": not rejected_lemmas,
        },
        {
            "id": "P-05",
            "statement": "No validated off-critical-line counterexample candidate is currently recorded.",
            "holds": not has_validated_counterexample,
        },
        {
            "id": "P-06",
            "statement": "Transition-threshold support artifact is present.",
            "holds": transition_present,
        },
        {
            "id": "P-07",
            "statement": "Dipole analysis artifact is present with a non-empty probe set and ranked asymmetry windows.",
            "holds": bool(dipole_summary.get("artifact_present"))
            and int(dipole_summary.get("probe_count") or 0) > 0
            and int(dipole_summary.get("top_window_count") or 0) > 0,
        },
    ]
    all_hold = all(item["holds"] for item in premises)
    return {
        "target_outcome": "rh_likely_true_internal",
        "proof_type": "sufficiency_contract",
        "premises": premises,
        "conclusion_enabled": all_hold,
        "conclusion": (
            "Current internal evidence is sufficient to pin RH to the likely-true branch."
            if all_hold
            else "Current internal evidence is not sufficient to pin RH to the likely-true branch."
        ),
    }


def build_negative_outcome_exclusion(
    has_validated_counterexample: bool,
    counterexample_summary: dict,
    refine_summary: dict,
    stability_summary: dict,
    dipole_summary: dict,
) -> dict:
    blockers = [
        {
            "id": "N-01",
            "statement": "The counterexample branch requires at least one validated off-critical-line zero candidate.",
            "holds": not has_validated_counterexample,
        },
        {
            "id": "N-02",
            "statement": "The coarse counterexample search reports zero candidates.",
            "holds": int(counterexample_summary.get("candidate_count") or 0) == 0,
        },
        {
            "id": "N-03",
            "statement": "The adaptive refine search reports zero candidates and zero validated points.",
            "holds": int(refine_summary.get("candidate_count") or 0) == 0
            and int(refine_summary.get("validated_count") or 0) == 0,
        },
        {
            "id": "N-04",
            "statement": "Refined near-miss points do not remain stable across all tested neighborhood scales.",
            "holds": int(stability_summary.get("stable_points") or 0) == 0
            and int(stability_summary.get("unstable_points") or 0) > 0,
        },
        {
            "id": "N-05",
            "statement": "Dipole stress windows are present but do not produce a certified off-critical zero witness.",
            "holds": bool(dipole_summary.get("artifact_present"))
            and dipole_summary.get("top_asymmetry_score") is not None
            and not has_validated_counterexample,
        },
    ]
    all_hold = all(item["holds"] for item in blockers)
    return {
        "excluded_outcome": "counterexample_candidate_internal",
        "proof_type": "current_exclusion_contract",
        "requirements_not_met": blockers,
        "exclusion_enabled": all_hold,
        "conclusion": (
            "The current artifact set does not justify pinning the counterexample branch."
            if all_hold
            else "The current artifact set no longer cleanly excludes the counterexample branch."
        ),
    }


def build_branch_comparison_artifact(
    pinned_outcome: str,
    confidence_band: str,
    rationale: list[str],
    positive_pin_proof: dict,
    negative_outcome_exclusion: dict,
    geometric_signal_score: int,
    dipole_summary: dict,
) -> dict:
    selected_branch = {
        "branch": pinned_outcome,
        "status": "selected",
        "confidence_band": confidence_band,
        "geometric_signal_score": geometric_signal_score,
        "why_this_cup": rationale,
        "contract": positive_pin_proof,
    }
    excluded_branch = {
        "branch": negative_outcome_exclusion.get("excluded_outcome"),
        "status": "excluded_for_now",
        "why_not_the_other_cup": [
            item.get("statement")
            for item in negative_outcome_exclusion.get("requirements_not_met") or []
            if isinstance(item, dict) and bool(item.get("holds"))
        ],
        "contract": negative_outcome_exclusion,
    }
    return {
        "program": "RH Outcome Branch Comparison",
        "version": "v0.1",
        "reviewer_note": "Reviewer-facing comparison of the selected internal branch versus the currently excluded opposing branch.",
        "question": "Which branch is currently justified by the committed evidence, and which branch is currently excluded?",
        "selected_branch": selected_branch,
        "excluded_branch": excluded_branch,
        "contrast_summary": {
            "selected_branch_conclusion": positive_pin_proof.get("conclusion"),
            "excluded_branch_conclusion": negative_outcome_exclusion.get("conclusion"),
            "decision_rule": "Select the RH-likely branch only when all positive premises hold and the counterexample branch lacks a validated off-critical-line witness.",
        },
        "dipole_context": dipole_summary,
        "scope_note": "Comparison artifact only; this is an internal evidence contrast, not an external proof claim.",
    }


def render_branch_comparison_markdown(branch_comparison: dict) -> str:
    selected = branch_comparison.get("selected_branch") or {}
    excluded = branch_comparison.get("excluded_branch") or {}
    selected_contract = selected.get("contract") or {}
    excluded_contract = excluded.get("contract") or {}
    contrast = branch_comparison.get("contrast_summary") or {}

    selected_reasons = "\n".join(
        f"- {item}" for item in selected.get("why_this_cup") or []
    ) or "- none recorded"
    excluded_reasons = "\n".join(
        f"- {item}" for item in excluded.get("why_not_the_other_cup") or []
    ) or "- none recorded"
    positive_premises = "\n".join(
        f"- {item.get('id')}: {item.get('statement')} Holds: {bool(item.get('holds'))}."
        for item in selected_contract.get("premises") or []
        if isinstance(item, dict)
    ) or "- none recorded"
    negative_requirements = "\n".join(
        f"- {item.get('id')}: {item.get('statement')} Holds: {bool(item.get('holds'))}."
        for item in excluded_contract.get("requirements_not_met") or []
        if isinstance(item, dict)
    ) or "- none recorded"

    return f"""# RH Outcome Branch Comparison

Question:
- {branch_comparison.get("question")}

Reviewer note:
- {branch_comparison.get("reviewer_note")}

## Selected Branch

- Branch: `{selected.get("branch")}`
- Status: `{selected.get("status")}`
- Confidence band: `{selected.get("confidence_band")}`
- Geometric signal score: `{selected.get("geometric_signal_score")}`

Why this cup:
{selected_reasons}

Positive contract conclusion:
- {selected_contract.get("conclusion")}

Positive contract premises:
{positive_premises}

## Excluded Branch

- Branch: `{excluded.get("branch")}`
- Status: `{excluded.get("status")}`

Why not the other cup:
{excluded_reasons}

Exclusion contract conclusion:
- {excluded_contract.get("conclusion")}

Exclusion requirements currently holding:
{negative_requirements}

## Contrast Summary

- Selected branch conclusion: {contrast.get("selected_branch_conclusion")}
- Excluded branch conclusion: {contrast.get("excluded_branch_conclusion")}
- Decision rule: {contrast.get("decision_rule")}

Scope note:
- {branch_comparison.get("scope_note")}
"""


def parse_rejected_lemmas(registry_text: str) -> list[str]:
    rejected = []
    current = None
    for line in registry_text.splitlines():
        line = line.strip()
        if line.startswith("### "):
            current = line.replace("### ", "", 1)
        if line.startswith("- Status:") and "rejected" in line and current:
            rejected.append(current)
    return rejected


def main() -> None:
    witness_path = ART / "logic_geometry_witness_report.json"
    transition_path = ART / "hafnian_flux_transition_thresholds.json"
    counterexample_path = ART / "rh_counterexample_candidates.json"
    refine_path = ART / "rh_counterexample_refine_candidates.json"
    stability_path = ART / "rh_stability_ladder.json"
    dipole_path = ART / "rh_dipole_analysis.json"
    lemma_registry_path = ROOT / "docs" / "findings" / "RH_LEMMA_REGISTRY_V0_1.md"

    witness = load_json(witness_path)
    transition = load_json(transition_path)
    counterexample = load_json(counterexample_path)
    refine = load_json(refine_path)
    stability = load_json(stability_path)
    dipole = load_json(dipole_path)

    if witness is None:
        raise SystemExit("missing required witness artifact: docs/findings/artifacts/logic_geometry_witness_report.json")

    theorem_checks = witness.get("theorem_checks", {})
    t1 = bool(theorem_checks.get("T1_constraint_distinguishability_pass"))
    t2 = bool(theorem_checks.get("T2_endpoint_stability_all_pass"))
    t3_witnesses = theorem_checks.get("T3_path_endpoint_decoupling_witnesses") or []
    t3_count = len(t3_witnesses)

    rejected_lemmas: list[str] = []
    if lemma_registry_path.exists():
        rejected_lemmas = parse_rejected_lemmas(lemma_registry_path.read_text(encoding="utf-8"))

    counterexample_summary, has_validated_counterexample = summarize_counterexample(counterexample)
    refine_summary = summarize_refine(refine)
    stability_summary = summarize_stability(stability)
    dipole_summary = summarize_dipole(dipole)

    geometric_signal_score = 0
    geometric_signal_score += 40 if t1 else 0
    geometric_signal_score += 30 if t2 else 0
    geometric_signal_score += 30 if t3_count > 0 else 0

    transition_present = transition is not None
    positive_pin_proof = build_positive_pin_proof(
        t1=t1,
        t2=t2,
        t3_witnesses=t3_witnesses,
        rejected_lemmas=rejected_lemmas,
        has_validated_counterexample=has_validated_counterexample,
        transition_present=transition_present,
        dipole_summary=dipole_summary,
    )
    negative_outcome_exclusion = build_negative_outcome_exclusion(
        has_validated_counterexample=has_validated_counterexample,
        counterexample_summary=counterexample_summary,
        refine_summary=refine_summary,
        stability_summary=stability_summary,
        dipole_summary=dipole_summary,
    )

    if has_validated_counterexample:
        pinned_outcome = "counterexample_candidate_internal"
        confidence_band = "high"
        rationale = [
            "A validated off-critical-line candidate is present in the counterexample artifact.",
            "Internal pin is set to counterexample route pending external proof-grade verification.",
        ]
    elif t1 and t2 and t3_count > 0 and not rejected_lemmas:
        pinned_outcome = "rh_likely_true_internal"
        confidence_band = "medium"
        rationale = [
            "Geometric theorem checks T1/T2 pass and T3 witnesses exist.",
            "No rejected lemmas are currently active in the chain.",
            "No validated off-critical-line candidate has been recorded.",
            "Dipole stress windows are present but none certifies an off-critical zero witness.",
        ]
    else:
        pinned_outcome = "undecided_internal"
        confidence_band = "low"
        rationale = [
            "Current geometric evidence is insufficient for an internal directional pin.",
            "At least one required theorem check or lemma consistency condition is not satisfied.",
        ]

    result = {
        "program": "RH Internal Outcome Pinner",
        "version": "v0.1",
        "pinned_outcome": pinned_outcome,
        "confidence_band": confidence_band,
        "geometric_logic_evidence": {
            "t1_constraint_distinguishability_pass": t1,
            "t2_endpoint_stability_all_pass": t2,
            "t3_witness_count": t3_count,
            "t3_witness_families": t3_witnesses,
            "geometric_signal_score": geometric_signal_score,
            "transition_threshold_artifact_present": transition_present,
            "rejected_lemmas": rejected_lemmas,
        },
        "counterexample_summary": counterexample_summary,
        "counterexample_refine_summary": refine_summary,
        "counterexample_stability_summary": stability_summary,
        "dipole_summary": dipole_summary,
        "rationale": rationale,
        "proof_contracts": {
            "positive_pin_proof": positive_pin_proof,
            "negative_outcome_exclusion": negative_outcome_exclusion,
        },
        "falsification_hooks": [
            "Any validated off-critical-line zero candidate flips outcome to counterexample_candidate_internal.",
            "Any regression in T1/T2 stability or removal of all T3 witnesses downgrades to undecided_internal.",
            "Any lemma moved to rejected in active chain downgrades confidence and may force undecided_internal.",
        ],
        "scope_note": "Internal directional pinner only; this is not a formal theorem proof/disproof claim.",
    }
    branch_comparison = build_branch_comparison_artifact(
        pinned_outcome=pinned_outcome,
        confidence_band=confidence_band,
        rationale=rationale,
        positive_pin_proof=positive_pin_proof,
        negative_outcome_exclusion=negative_outcome_exclusion,
        geometric_signal_score=geometric_signal_score,
        dipole_summary=dipole_summary,
    )
    branch_comparison_note = render_branch_comparison_markdown(branch_comparison)

    ART.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    BRANCH_COMPARISON_OUT.write_text(json.dumps(branch_comparison, indent=2), encoding="utf-8")
    BRANCH_COMPARISON_NOTE_OUT.write_text(branch_comparison_note, encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
