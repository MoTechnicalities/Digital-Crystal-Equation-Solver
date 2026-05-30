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


def looks_like_git_sha(value: object) -> bool:
    if not isinstance(value, str):
        return False
    return re.fullmatch(r"[0-9a-f]{7,40}", value) is not None


def is_placeholder_value(value: object) -> bool:
    if not isinstance(value, str):
        return False
    normalized = value.strip().upper()
    return (
        "TEMPLATE" in normalized
        or "YYYY-" in normalized
        or "TODO" in normalized
        or normalized == ""
    )


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
    has_supporting_attestation = False
    has_independent_passed_run = False
    lock_matches_expected_hash = False
    lock_has_non_template_commit = False

    details: dict[str, object] = {
        "required_artifacts": {
            "attestations": "docs/findings/artifacts/rh_independent_review_attestations.json",
            "repro_manifest": "docs/findings/artifacts/rh_reproducibility_manifest.json",
            "proof_version_lock": "docs/findings/artifacts/rh_proof_version_lock.json",
        },
        "required_schema_version": "RH_EXT_VERIFY_V2",
    }

    att_path = details["required_artifacts"]["attestations"]
    repro_path = details["required_artifacts"]["repro_manifest"]
    lock_path = details["required_artifacts"]["proof_version_lock"]

    att = load_json(att_path)
    if att is None:
        missing.append(att_path)
        details["attestations_valid"] = False
    else:
        details["attestations_schema_version"] = att.get("schema_version")
        if att.get("schema_version") != "RH_EXT_VERIFY_V2":
            missing.append(f"{att_path}::schema_version_mismatch")

        contract = att.get("closure_contract")
        if not isinstance(contract, dict):
            missing.append(f"{att_path}::missing_closure_contract")

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
                for strict_field in ["reviewer_id", "environment_id", "signed_reference"]:
                    if is_placeholder_value(entry.get(strict_field)):
                        valid_entries = False
                        missing.append(f"{att_path}::placeholder_value::{index}::{strict_field}")

            has_support = any(
                isinstance(entry, dict) and entry.get("outcome") == "supports"
                for entry in attestations
            )
            has_supporting_attestation = has_support
            if not has_support:
                valid_entries = False
                missing.append(f"{att_path}::no_supporting_attestation")

            supporting_reviewers = {
                entry.get("reviewer_id")
                for entry in attestations
                if isinstance(entry, dict) and entry.get("outcome") == "supports"
            }
            if len(supporting_reviewers) < 1:
                valid_entries = False
                missing.append(f"{att_path}::supporting_reviewer_missing")

            details["attestations_valid"] = valid_entries
            details["supporting_attestation_present"] = has_support

    repro = load_json(repro_path)
    if repro is None:
        missing.append(repro_path)
        details["repro_manifest_valid"] = False
    else:
        details["repro_schema_version"] = repro.get("schema_version")
        if repro.get("schema_version") != "RH_EXT_VERIFY_V2":
            missing.append(f"{repro_path}::schema_version_mismatch")

        contract = repro.get("closure_contract")
        if not isinstance(contract, dict):
            missing.append(f"{repro_path}::missing_closure_contract")

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
                and not is_placeholder_value(run.get("run_id"))
                for run in runs
            )
            has_independent_passed_run = independent_pass
            details["repro_manifest_valid"] = independent_pass
            if not independent_pass:
                missing.append(f"{repro_path}::no_independent_passed_run")

            passed_runs_with_status_artifact = any(
                isinstance(run, dict)
                and run.get("environment_origin") == "independent"
                and run.get("status") == "passed"
                and isinstance(run.get("artifact_checks"), list)
                and any(
                    isinstance(check, dict)
                    and check.get("path") == "docs/findings/artifacts/rh_proof_pipeline_status.json"
                    and check.get("status") == "passed"
                    for check in run.get("artifact_checks")
                )
                for run in runs
            )
            details["independent_pass_with_status_artifact_check"] = passed_runs_with_status_artifact
            if not passed_runs_with_status_artifact:
                missing.append(f"{repro_path}::missing_passed_status_artifact_check")

    expected_hash, hash_missing = contradiction_audit_expected_hash()
    details["expected_contradiction_audit_hash"] = expected_hash
    if hash_missing:
        missing.extend([f"contradiction_hash_source::{item}" for item in hash_missing])

    lock = load_json(lock_path)
    if lock is None:
        missing.append(lock_path)
        details["proof_version_lock_valid"] = False
    else:
        details["lock_schema_version"] = lock.get("schema_version")
        if lock.get("schema_version") != "RH_EXT_VERIFY_V2":
            missing.append(f"{lock_path}::schema_version_mismatch")

        contract = lock.get("closure_contract")
        if not isinstance(contract, dict):
            missing.append(f"{lock_path}::missing_closure_contract")

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
        else:
            lock_matches_expected_hash = True

        if not looks_like_git_sha(lock.get("proof_commit")):
            lock_ok = False
            missing.append(f"{lock_path}::invalid_or_placeholder_proof_commit")
        else:
            lock_has_non_template_commit = True

        details["proof_version_lock_valid"] = lock_ok

    details["o7_closure_checklist"] = [
        {
            "id": "O7-CHECK-01",
            "criterion": "At least one non-placeholder independent supporting attestation is present.",
            "satisfied": has_supporting_attestation,
            "action_if_open": "Replace template attestation with a real independent reviewer record whose outcome is supports.",
        },
        {
            "id": "O7-CHECK-02",
            "criterion": "At least one independent reproducibility run has status passed with a passed proof-status artifact check.",
            "satisfied": bool(has_independent_passed_run and details.get("independent_pass_with_status_artifact_check")),
            "action_if_open": "Add a real independent run with status passed and artifact_checks entry for docs/findings/artifacts/rh_proof_pipeline_status.json marked passed.",
        },
        {
            "id": "O7-CHECK-03",
            "criterion": "Proof version lock has non-template commit SHA and matches the expected contradiction-audit hash.",
            "satisfied": bool(lock_matches_expected_hash and lock_has_non_template_commit),
            "action_if_open": "Set proof_commit to a real git SHA and set contradiction_audit_hash to the expected hash emitted by this script.",
        },
    ]

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
        "## 12. Internal Outcome Pin Contract",
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


def internal_outcome_pin_contract_check() -> tuple[bool, list[str], dict]:
    rel_path = "docs/findings/artifacts/rh_outcome_pinner_status.json"
    comparison_path = "docs/findings/artifacts/rh_outcome_branch_comparison.json"
    dipole_path = "docs/findings/artifacts/rh_dipole_analysis.json"
    triangulation_path = "docs/findings/artifacts/rh_dipole_triangulation.json"
    manuscript_path = "docs/findings/RH_COMPLETE_PROOF.md"
    missing: list[str] = []
    details: dict[str, object] = {
        "required_pinned_outcome": "rh_likely_true_internal",
        "required_comparison_artifact": comparison_path,
        "required_dipole_artifact": dipole_path,
        "required_triangulation_artifact": triangulation_path,
        "required_manuscript_section": "## 12. Internal Outcome Pin Contract",
        "positive_pin_premises": {},
        "negative_outcome_requirements": {},
    }

    payload = load_json(rel_path)
    if payload is None:
        missing.append(rel_path)
        return False, missing, details

    if payload.get("pinned_outcome") != "rh_likely_true_internal":
        missing.append(f"{rel_path}::pinned_outcome_mismatch")

    positive = (payload.get("proof_contracts") or {}).get("positive_pin_proof") or {}
    negative = (payload.get("proof_contracts") or {}).get("negative_outcome_exclusion") or {}

    positive_premises = positive.get("premises") or []
    negative_requirements = negative.get("requirements_not_met") or []
    details["positive_pin_premises"] = {
        item.get("id", f"premise_{index}"): bool(item.get("holds"))
        for index, item in enumerate(positive_premises)
        if isinstance(item, dict)
    }
    details["negative_outcome_requirements"] = {
        item.get("id", f"requirement_{index}"): bool(item.get("holds"))
        for index, item in enumerate(negative_requirements)
        if isinstance(item, dict)
    }

    if positive.get("conclusion_enabled") is not True:
        missing.append(f"{rel_path}::positive_pin_proof_not_enabled")
    if negative.get("exclusion_enabled") is not True:
        missing.append(f"{rel_path}::negative_outcome_exclusion_not_enabled")

    required_positive_ids = ["P-01", "P-02", "P-03", "P-04", "P-05", "P-06", "P-07", "P-08"]
    for item_id in required_positive_ids:
        if details["positive_pin_premises"].get(item_id) is not True:
            missing.append(f"{rel_path}::positive_premise_not_satisfied::{item_id}")

    required_negative_ids = ["N-01", "N-02", "N-03", "N-04", "N-05", "N-06"]
    for item_id in required_negative_ids:
        if details["negative_outcome_requirements"].get(item_id) is not True:
            missing.append(f"{rel_path}::negative_requirement_not_satisfied::{item_id}")

    dipole_summary = payload.get("dipole_summary") or {}
    details["pinner_dipole_summary_present"] = bool(dipole_summary)
    details["pinner_dipole_probe_count"] = int(dipole_summary.get("probe_count") or 0)
    details["pinner_dipole_top_asymmetry_score"] = dipole_summary.get("top_asymmetry_score")
    if not dipole_summary:
        missing.append(f"{rel_path}::missing_dipole_summary")
    elif not bool(dipole_summary.get("artifact_present")):
        missing.append(f"{rel_path}::dipole_artifact_not_present")
    elif int(dipole_summary.get("probe_count") or 0) <= 0:
        missing.append(f"{rel_path}::dipole_probe_count_invalid")

    triangulation_summary = payload.get("triangulation_summary") or {}
    details["pinner_triangulation_summary_present"] = bool(triangulation_summary)
    details["pinner_triangulation_probe_count"] = int(triangulation_summary.get("probe_count") or 0)
    details["pinner_triangulation_max_gap"] = triangulation_summary.get("max_triangle_side_gap")
    if not triangulation_summary:
        missing.append(f"{rel_path}::missing_triangulation_summary")
    elif not bool(triangulation_summary.get("artifact_present")):
        missing.append(f"{rel_path}::triangulation_artifact_not_present")
    elif int(triangulation_summary.get("probe_count") or 0) <= 0:
        missing.append(f"{rel_path}::triangulation_probe_count_invalid")

    dipole = load_json(dipole_path)
    if dipole is None:
        missing.append(dipole_path)
    else:
        details["dipole_program"] = dipole.get("program")
        details["dipole_probe_count"] = int((dipole.get("sample_config") or {}).get("probe_count") or 0)
        details["dipole_top_asymmetry_windows"] = len(dipole.get("top_asymmetry_windows") or [])
        if dipole.get("program") != "RH Dipole Symmetry Analysis":
            missing.append(f"{dipole_path}::program_mismatch")
        if int((dipole.get("sample_config") or {}).get("probe_count") or 0) <= 0:
            missing.append(f"{dipole_path}::probe_count_invalid")
        if len(dipole.get("top_asymmetry_windows") or []) == 0:
            missing.append(f"{dipole_path}::missing_top_asymmetry_windows")

    triangulation = load_json(triangulation_path)
    if triangulation is None:
        missing.append(triangulation_path)
    else:
        details["triangulation_program"] = triangulation.get("program")
        details["triangulation_probe_count"] = int((triangulation.get("config") or {}).get("probe_count") or 0)
        details["triangulation_gap_windows"] = len(triangulation.get("top_triangle_gap_windows") or [])
        if triangulation.get("program") != "RH Dipole Triangulation":
            missing.append(f"{triangulation_path}::program_mismatch")
        if int((triangulation.get("config") or {}).get("probe_count") or 0) <= 0:
            missing.append(f"{triangulation_path}::probe_count_invalid")
        if len(triangulation.get("top_triangle_gap_windows") or []) == 0:
            missing.append(f"{triangulation_path}::missing_top_triangle_gap_windows")

    comparison = load_json(comparison_path)
    if comparison is None:
        missing.append(comparison_path)
    else:
        selected_branch = comparison.get("selected_branch") or {}
        excluded_branch = comparison.get("excluded_branch") or {}
        contrast_summary = comparison.get("contrast_summary") or {}

        details["comparison_selected_branch"] = selected_branch.get("branch")
        details["comparison_excluded_branch"] = excluded_branch.get("branch")
        details["comparison_has_decision_rule"] = bool(contrast_summary.get("decision_rule"))

        if comparison.get("program") != "RH Outcome Branch Comparison":
            missing.append(f"{comparison_path}::program_mismatch")
        if selected_branch.get("branch") != "rh_likely_true_internal":
            missing.append(f"{comparison_path}::selected_branch_mismatch")
        if selected_branch.get("status") != "selected":
            missing.append(f"{comparison_path}::selected_branch_status_mismatch")
        if excluded_branch.get("branch") != "counterexample_candidate_internal":
            missing.append(f"{comparison_path}::excluded_branch_mismatch")
        if excluded_branch.get("status") != "excluded_for_now":
            missing.append(f"{comparison_path}::excluded_branch_status_mismatch")
        if not (selected_branch.get("why_this_cup") or []):
            missing.append(f"{comparison_path}::missing_selected_branch_rationale")
        if not (excluded_branch.get("why_not_the_other_cup") or []):
            missing.append(f"{comparison_path}::missing_excluded_branch_rationale")
        if not contrast_summary.get("selected_branch_conclusion"):
            missing.append(f"{comparison_path}::missing_selected_branch_conclusion")
        if not contrast_summary.get("excluded_branch_conclusion"):
            missing.append(f"{comparison_path}::missing_excluded_branch_conclusion")
        if not contrast_summary.get("decision_rule"):
            missing.append(f"{comparison_path}::missing_decision_rule")

    if not exists(manuscript_path):
        missing.append(manuscript_path)
    else:
        manuscript = read_text(manuscript_path)
        section = "## 12. Internal Outcome Pin Contract"
        details["manuscript_section_present"] = section in manuscript
        if section not in manuscript:
            missing.append(f"{manuscript_path}::missing_section::{section}")

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
            id="O6b-outcome-pin-contract",
            description="The internal outcome pinner states and satisfies both the positive pin contract and the counterexample-branch exclusion contract, with a reviewer-facing branch-comparison artifact.",
            required_paths=(
                "docs/findings/artifacts/rh_outcome_pinner_status.json",
                "docs/findings/artifacts/rh_outcome_branch_comparison.json",
                "docs/findings/artifacts/rh_dipole_analysis.json",
                "docs/findings/artifacts/rh_dipole_triangulation.json",
                "docs/findings/RH_COMPLETE_PROOF.md",
            ),
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
    outcome_pin_contract_details = {}
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
        if item.id == "O6b-outcome-pin-contract":
            outcome_ok, outcome_missing, outcome_details = internal_outcome_pin_contract_check()
            outcome_pin_contract_details = outcome_details
            if not outcome_ok:
                missing.extend(outcome_missing)
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

    next_actions = [
        "Preserve the positive-pin sufficiency premises and the counterexample-branch exclusion contract.",
    ]
    o7_checklist = external_verification_details.get("o7_closure_checklist")
    if isinstance(o7_checklist, list):
        for item in o7_checklist:
            if isinstance(item, dict) and item.get("satisfied") is not True and item.get("action_if_open"):
                next_actions.append(str(item.get("action_if_open")))

    if len(next_actions) == 1:
        next_actions.extend(
            [
                "Collect independent signed review attestations artifact.",
                "Record at least one passed reproducibility run from an independent environment.",
                "Lock theorem-proof artifact version with matching contradiction-audit hash.",
            ]
        )

    next_actions = list(dict.fromkeys(next_actions))

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
            "O6b-outcome-pin-contract": outcome_pin_contract_details,
            "O7-external-verification": external_verification_details,
        },
        "next_actions": next_actions,
    }

    ART.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(status, indent=2), encoding="utf-8")
    print(json.dumps(status, indent=2))


if __name__ == "__main__":
    main()
