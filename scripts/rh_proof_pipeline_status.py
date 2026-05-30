#!/bin/python3
"""Generate RH proof-program status from concrete repository artifacts.

This is not a proof engine. It is a deterministic progress tracker that maps
required prize-level obligations to current evidence artifacts.
"""

from __future__ import annotations

import json
import re
import hashlib
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


def section_by_heading(markdown: str, heading: str) -> str:
    marker = f"\n{heading}\n"
    start = markdown.find(marker)
    if start == -1:
        if markdown.startswith(f"{heading}\n"):
            start = 0
        else:
            return ""
    else:
        start += 1

    content_start = start + len(heading) + 1
    remainder = markdown[content_start:]
    next_header_offset = remainder.find("\n## ")
    if next_header_offset == -1:
        return remainder.strip()
    return remainder[:next_header_offset].strip()


def contradiction_audit_expected_hash() -> tuple[str, list[str]]:
    missing = []
    proof_path = "docs/findings/RH_COMPLETE_PROOF.md"
    registry_path = "docs/findings/RH_LEMMA_REGISTRY_V0_1.md"

    if not exists(proof_path):
        missing.append(proof_path)
        proof_section = ""
    else:
        proof_text = read_text(proof_path)
        proof_section = section_by_heading(proof_text, "## 11. Contradiction Audit Mirror (C6-SUB-06)")
        if not proof_section:
            missing.append(f"{proof_path}::missing_section::## 11. Contradiction Audit Mirror (C6-SUB-06)")

    if not exists(registry_path):
        missing.append(registry_path)
        registry_section = ""
    else:
        registry_text = read_text(registry_path)
        registry_section = section_by_heading(registry_text, "## 6. Contradiction Audit Table (Linked to Lemma Status)")
        if not registry_section:
            missing.append(f"{registry_path}::missing_section::## 6. Contradiction Audit Table (Linked to Lemma Status)")

    payload = f"{proof_section}\n---\n{registry_section}".encode("utf-8")
    return hashlib.sha256(payload).hexdigest(), missing


def external_verification_contract_check() -> tuple[bool, list[str], dict]:
    """Require external evidence artifacts so prize readiness is never doc-only."""
    missing: list[str] = []
    details: dict[str, object] = {
        "required_artifacts": {
            "attestations": "docs/findings/artifacts/rh_independent_review_attestations.json",
            "repro_manifest": "docs/findings/artifacts/rh_reproducibility_manifest.json",
            "proof_version_lock": "docs/findings/artifacts/rh_proof_version_lock.json",
        }
    }

    att_path = details["required_artifacts"]["attestations"]
    repro_path = details["required_artifacts"]["repro_manifest"]
    lock_path = details["required_artifacts"]["proof_version_lock"]

    att = load_json(att_path)
    if att is None:
        missing.append(att_path)
        details["attestations_valid"] = False
    else:
        attestations = att.get("attestations")
        if not isinstance(attestations, list) or not attestations:
            missing.append(f"{att_path}::attestations_missing_or_empty")
            details["attestations_valid"] = False
        else:
            valid_entries = True
            for index, entry in enumerate(attestations):
                if not isinstance(entry, dict):
                    valid_entries = False
                    missing.append(f"{att_path}::attestation_not_object::{index}")
                    continue
                required_fields = ["reviewer_id", "environment_id", "outcome", "signed_reference"]
                for field in required_fields:
                    if not entry.get(field):
                        valid_entries = False
                        missing.append(f"{att_path}::missing_field::{index}::{field}")
                if entry.get("outcome") not in {"supports", "rejects", "inconclusive"}:
                    valid_entries = False
                    missing.append(f"{att_path}::invalid_outcome::{index}")
            has_support = any(
                isinstance(entry, dict) and entry.get("outcome") == "supports"
                for entry in attestations
            )
            if not has_support:
                valid_entries = False
                missing.append(f"{att_path}::no_supporting_attestation")
            details["attestations_valid"] = valid_entries

    repro = load_json(repro_path)
    if repro is None:
        missing.append(repro_path)
        details["repro_manifest_valid"] = False
    else:
        runs = repro.get("runs")
        if not isinstance(runs, list) or not runs:
            missing.append(f"{repro_path}::runs_missing_or_empty")
            details["repro_manifest_valid"] = False
        else:
            independent_pass = any(
                isinstance(run, dict)
                and run.get("environment_origin") == "independent"
                and run.get("status") == "passed"
                and run.get("run_id")
                for run in runs
            )
            details["repro_manifest_valid"] = independent_pass
            if not independent_pass:
                missing.append(f"{repro_path}::no_independent_passed_run")

    expected_hash, hash_missing = contradiction_audit_expected_hash()
    details["expected_contradiction_audit_hash"] = expected_hash
    if hash_missing:
        missing.extend([f"contradiction_hash_source::{item}" for item in hash_missing])

    lock = load_json(lock_path)
    if lock is None:
        missing.append(lock_path)
        details["proof_version_lock_valid"] = False
    else:
        required_lock_fields = ["proof_document", "proof_commit", "contradiction_audit_hash", "locked_at"]
        lock_ok = True
        for field in required_lock_fields:
            if not lock.get(field):
                lock_ok = False
                missing.append(f"{lock_path}::missing_field::{field}")

        if lock.get("proof_document") != "docs/findings/RH_COMPLETE_PROOF.md":
            lock_ok = False
            missing.append(f"{lock_path}::proof_document_mismatch")

        if lock.get("contradiction_audit_hash") != expected_hash:
            lock_ok = False
            missing.append(f"{lock_path}::contradiction_audit_hash_mismatch")

        details["proof_version_lock_valid"] = lock_ok

    return len(missing) == 0, missing, details


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
        Obligation(
            id="O7-external-verification",
            description="Independent external verification artifacts exist and are consistent with contradiction-audit version lock.",
            required_paths=(
                "docs/findings/artifacts/rh_independent_review_attestations.json",
                "docs/findings/artifacts/rh_reproducibility_manifest.json",
                "docs/findings/artifacts/rh_proof_version_lock.json",
            ),
        ),
    ]

    checks = []
    theorem_proof_check_details = {}
    external_verification_details = {}
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
        if item.id == "O7-external-verification":
            external_ok, external_missing, external_details = external_verification_contract_check()
            external_verification_details = external_details
            if not external_ok:
                missing.extend(external_missing)
        # Keep status output readable by collapsing duplicate diagnostics.
        missing = list(dict.fromkeys(missing))
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
            "O7-external-verification": external_verification_details,
        },
        "next_actions": [
            "Collect independent signed review attestations artifact.",
            "Record at least one passed reproducibility run from an independent environment.",
            "Lock theorem-proof artifact version with matching contradiction-audit hash.",
        ],
    }

    ART.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(status, indent=2), encoding="utf-8")
    print(json.dumps(status, indent=2))


if __name__ == "__main__":
    main()
