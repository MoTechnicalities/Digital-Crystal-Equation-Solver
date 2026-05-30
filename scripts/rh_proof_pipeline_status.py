#!/bin/python3
"""Generate RH proof-program status from concrete repository artifacts.

This is not a proof engine. It is a deterministic progress tracker that maps
required prize-level obligations to current evidence artifacts.
"""

from __future__ import annotations

import json
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
    for item in obligations:
        missing = [path for path in item.required_paths if not exists(path)]
        if item.id == "O5-explicit-signatures":
            explicit_ok, explicit_missing = explicit_signature_contract_ok()
            if not explicit_ok:
                missing.extend(explicit_missing)
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
