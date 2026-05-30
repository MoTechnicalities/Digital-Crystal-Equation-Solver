#!/bin/python3
"""Generate RH proof-program status from concrete repository artifacts.

This is not a proof engine. It is a deterministic progress tracker that maps
required prize-level obligations to current evidence artifacts.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "docs" / "findings" / "artifacts"
OUT = ART / "rh_proof_pipeline_status.json"


@dataclass(frozen=True)
class Obligation:
    id: str
    description: str
    required_paths: tuple[str, ...]


def exists(rel_path: str) -> bool:
    return (ROOT / rel_path).exists()


def read_text(rel_path: str) -> str:
    return (ROOT / rel_path).read_text(encoding="utf-8")


def load_json(rel_path: str) -> dict | None:
    path = ROOT / rel_path
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def explicit_signature_contract_ok() -> tuple[bool, list[str]]:
    missing = []

    engine_path = "crates/digitalcrystal-engine/src/lib.rs"
    if not exists(engine_path):
        missing.append(engine_path)
    else:
        source = read_text(engine_path)
        if "pub path_signature: String" not in source:
            missing.append("crates/digitalcrystal-engine/src/lib.rs::path_signature_field")
        if "pub endpoint_signature: String" not in source:
            missing.append("crates/digitalcrystal-engine/src/lib.rs::endpoint_signature_field")

    report_path = "docs/findings/artifacts/logic_geometry_witness_report.json"
    report = load_json(report_path)
    if report is None:
        missing.append(report_path)
    else:
        if report.get("signature_source") != "explicit_api_fields":
            missing.append("logic_geometry_witness_report.json::signature_source")
        if report.get("theorem_checks", {}).get("signature_fields_present_all") is not True:
            missing.append("logic_geometry_witness_report.json::signature_fields_present_all")

    return len(missing) == 0, missing


def theorem_proof_contract_check() -> tuple[bool, list[str], dict]:
    """Require manuscript quality markers, not just file existence, for O6."""
    rel_path = "docs/findings/RH_COMPLETE_PROOF.md"
    missing = []
    details: dict[str, object] = {
        "required_section_headers": [],
        "required_status_markers": [],
        "forbidden_markers": [],
        "c6_checklist_required_ids": [],
        "c6_checklist_states": {},
    }

    if not exists(rel_path):
        missing.append(rel_path)
        return False, missing, details

    text = read_text(rel_path)

    required_headers = [
        "## 1. Claim and Verification Standard",
        "## 2. Scope and Boundaries",
        "## 3. Section Anchors from RH Theorem Chain Obligations",
        "### 3.1 O-C1 / C1: Path and Endpoint Signature Determinism",
        "### 3.2 O-C2 / C2: Constraint-Path Distinguishability",
        "### 3.3 O-C3 / C3: Path-Endpoint Decoupling Witnesses",
        "### 3.4 O-C4 / C4: RH Equivalent-Statement Mapping Layer",
        "### 3.5 O-C5 / C5: Lemma Closure and Contradiction Elimination",
        "### 3.6 O-C6 / C6: Final End-to-End Theorem Manuscript",
        "## 4. Prize Readiness Assessment (Current)",
        "## 5. Reproducibility and Audit Commands",
        "## 6. External Review Checklist",
    ]
    details["required_section_headers"] = required_headers

    for header in required_headers:
        if header not in text:
            missing.append(f"{rel_path}::missing_header::{header}")

    required_markers = [
        "Status:\n- satisfied",
        "Current assessment:\n- Not prize-ready.",
    ]
    details["required_status_markers"] = required_markers
    for marker in required_markers:
        if marker not in text:
            missing.append(f"{rel_path}::missing_marker::{marker}")

    forbidden_markers = [
        "Current assessment:\n- Prize-ready.",
        "Current assessment:\n- Completed proof.",
    ]
    details["forbidden_markers"] = forbidden_markers
    for marker in forbidden_markers:
        if marker in text:
            missing.append(f"{rel_path}::forbidden_marker_present::{marker}")

    has_repro_cmd = (
        "/bin/python3 scripts/rh_proof_pipeline_status.py" in text
        and "cargo test -p digitalcrystal-engine" in text
        and "cargo test -p digitalcrystal-api" in text
    )
    details["repro_commands_present"] = has_repro_cmd
    if not has_repro_cmd:
        missing.append(f"{rel_path}::missing_repro_commands")

    has_review_checklist = all(
        phrase in text
        for phrase in [
            "Reviewer should confirm:",
            "deterministic reproducibility",
            "explicit separation between evidence infrastructure and formal proof steps",
        ]
    )
    details["review_checklist_present"] = has_review_checklist
    if not has_review_checklist:
        missing.append(f"{rel_path}::missing_review_checklist")

    required_c6_ids = [
        "C6-SUB-01",
        "C6-SUB-02",
        "C6-SUB-03",
        "C6-SUB-04",
        "C6-SUB-05",
        "C6-SUB-06",
    ]
    details["c6_checklist_required_ids"] = required_c6_ids

    checklist_pattern = re.compile(r"^- \[(?P<state>[ xX])\] (?P<id>C6-SUB-[0-9]{2}):", re.MULTILINE)
    checklist_states: dict[str, bool] = {}
    for match in checklist_pattern.finditer(text):
        checklist_states[match.group("id")] = match.group("state").lower() == "x"
    details["c6_checklist_states"] = checklist_states

    for item_id in required_c6_ids:
        if item_id not in checklist_states:
            missing.append(f"{rel_path}::missing_c6_checklist_item::{item_id}")
        elif not checklist_states[item_id]:
            missing.append(f"{rel_path}::c6_checklist_item_open::{item_id}")

    return len(missing) == 0, missing, details


def main() -> None:
    obligations = [
        Obligation(
            id="O1-framework",
            description="Formal invariant framework (PSI/EI) is documented.",
            required_paths=("docs/findings/LOGIC_GEOMETRY_INVARIANTS_FRAMEWORK.md",),
        ),
        Obligation(
            id="O2-witness-catalog",
            description="Deterministic witness catalog exists with T1/T2/T3 checks.",
            required_paths=(
                "docs/findings/LOGIC_GEOMETRY_WITNESS_CATALOG_NOTE.md",
                "docs/findings/artifacts/logic_geometry_witness_report.json",
            ),
        ),
        Obligation(
            id="O3-transition-atlas",
            description="Transition atlas and threshold CI artifacts exist.",
            required_paths=(
                "docs/findings/HAFNIAN_FLUX_PHASE_TRANSITION_ATLAS_NOTE.md",
                "docs/findings/artifacts/hafnian_flux_transition_thresholds.json",
            ),
        ),
        Obligation(
            id="O4-rh-problem-clarity",
            description="RH problem statement and prize criteria are documented in product UI.",
            required_paths=("apps/api/src/main.rs",),
        ),
        Obligation(
            id="O5-explicit-signatures",
            description="First-class API path/endpoint signatures are mandatory and consumed by witness conformance artifacts.",
            required_paths=(
                "crates/digitalcrystal-engine/src/lib.rs",
                "docs/findings/artifacts/logic_geometry_witness_report.json",
            ),
        ),
        Obligation(
            id="O6-theorem-proof",
            description="A complete formal proof (or disproof) document exists and is externally verifiable.",
            required_paths=("docs/findings/RH_COMPLETE_PROOF.md",),
        ),
    ]

    checks = []
    theorem_proof_check_details = {}
    for item in obligations:
        missing = [path for path in item.required_paths if not exists(path)]
        if item.id == "O5-explicit-signatures":
            explicit_ok, explicit_missing = explicit_signature_contract_ok()
            if not explicit_ok:
                missing.extend(explicit_missing)
        if item.id == "O6-theorem-proof":
            theorem_ok, theorem_missing, theorem_details = theorem_proof_contract_check()
            theorem_proof_check_details = theorem_details
            if not theorem_ok:
                missing.extend(theorem_missing)
        checks.append(
            {
                "id": item.id,
                "description": item.description,
                "required_paths": list(item.required_paths),
                "status": "satisfied" if not missing else "open",
                "missing_paths": missing,
            }
        )

    satisfied = [c for c in checks if c["status"] == "satisfied"]
    open_items = [c for c in checks if c["status"] != "satisfied"]

    status = {
        "program": "Riemann Hypothesis Prize Proof Pipeline",
        "summary": {
            "total_obligations": len(checks),
            "satisfied": len(satisfied),
            "open": len(open_items),
            "prize_ready": len(open_items) == 0,
        },
        "obligations": checks,
        "gate_details": {
            "O6-theorem-proof": theorem_proof_check_details,
        },
        "next_actions": [
            "Add conformance tests for theorem candidates T1/T2/T3.",
            "Produce RH-specific theorem chain with externally reviewable proof obligations.",
            "Draft and validate complete proof manuscript candidate.",
        ],
    }

    ART.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(status, indent=2), encoding="utf-8")
    print(json.dumps(status, indent=2))


if __name__ == "__main__":
    main()
