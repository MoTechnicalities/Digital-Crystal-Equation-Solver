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


def load_json(path: Path) -> dict | None:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


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
    lemma_registry_path = ROOT / "docs" / "findings" / "RH_LEMMA_REGISTRY_V0_1.md"

    witness = load_json(witness_path)
    transition = load_json(transition_path)
    counterexample = load_json(counterexample_path)

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

    has_validated_counterexample = False
    counterexample_summary = {
        "artifact_present": counterexample is not None,
        "validated_off_critical_line": False,
        "candidate_count": 0,
    }
    if counterexample is not None:
        candidates = counterexample.get("candidates") or []
        counterexample_summary["candidate_count"] = len(candidates)
        has_validated_counterexample = any(
            isinstance(item, dict)
            and bool(item.get("validated"))
            and bool(item.get("off_critical_line"))
            for item in candidates
        )
        counterexample_summary["validated_off_critical_line"] = has_validated_counterexample

    geometric_signal_score = 0
    geometric_signal_score += 40 if t1 else 0
    geometric_signal_score += 30 if t2 else 0
    geometric_signal_score += 30 if t3_count > 0 else 0

    transition_present = transition is not None

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
        "rationale": rationale,
        "falsification_hooks": [
            "Any validated off-critical-line zero candidate flips outcome to counterexample_candidate_internal.",
            "Any regression in T1/T2 stability or removal of all T3 witnesses downgrades to undecided_internal.",
            "Any lemma moved to rejected in active chain downgrades confidence and may force undecided_internal.",
        ],
        "scope_note": "Internal directional pinner only; this is not a formal theorem proof/disproof claim.",
    }

    ART.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
