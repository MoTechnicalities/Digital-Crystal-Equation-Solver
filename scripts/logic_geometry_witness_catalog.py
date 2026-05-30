#!/bin/python3
"""Generate first geometric-logic witness catalog using live CSIF math endpoint.

This script operationalizes first-class API signature fields:
- PSI: payload.path_signature
- EI: payload.endpoint_signature
"""

from __future__ import annotations

import csv
import json
import urllib.request
from dataclasses import dataclass
from pathlib import Path

API_URL = "http://127.0.0.1:8080/v1/csif/math"
RUN_REPEATS = 5

ROOT = Path(__file__).resolve().parents[1]
ART_DIR = ROOT / "docs" / "findings" / "artifacts"
CSV_PATH = ART_DIR / "logic_geometry_witness_catalog.csv"
JSON_PATH = ART_DIR / "logic_geometry_witness_report.json"


@dataclass(frozen=True)
class ExprCase:
    case_id: str
    expression: str
    family: str


def post_eval(expression: str) -> dict:
    payload = {
        "expression": expression,
        "mode": "algebraic",
        "angle_unit": "radians",
    }
    body = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        API_URL,
        data=body,
        method="POST",
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def require_signature(payload: dict, field_name: str) -> str:
    value = payload.get(field_name)
    if not isinstance(value, str) or not value:
        raise ValueError(f"required signature field missing or invalid: {field_name}")
    return value


def main() -> None:
    ART_DIR.mkdir(parents=True, exist_ok=True)

    cases = [
        ExprCase("A1", "2 + 2 * 3", "precedence_pair_a"),
        ExprCase("A2", "(2 + 2) * 3", "precedence_pair_a"),
        ExprCase("B1", "(2 + 3) + 4", "assoc_add_pair_b"),
        ExprCase("B2", "2 + (3 + 4)", "assoc_add_pair_b"),
        ExprCase("C1", "(2 * 3) * 4", "assoc_mul_pair_c"),
        ExprCase("C2", "2 * (3 * 4)", "assoc_mul_pair_c"),
        ExprCase("D1", "(1 + 2) * 3", "distrib_pair_d"),
        ExprCase("D2", "(1 * 3) + (2 * 3)", "distrib_pair_d"),
    ]

    rows = []
    by_case = {}

    for case in cases:
        runs = []
        for run_idx in range(1, RUN_REPEATS + 1):
            payload = post_eval(case.expression)
            psi_sig = require_signature(payload, "path_signature")
            ei_sig = require_signature(payload, "endpoint_signature")
            row = {
                "case_id": case.case_id,
                "family": case.family,
                "expression": case.expression,
                "run": run_idx,
                "result": payload.get("result"),
                "psi_sig": psi_sig,
                "ei_sig": ei_sig,
                "final_theta": (payload.get("phase_signature") or {}).get("final_theta"),
                "cumulative_theta": (payload.get("phase_signature") or {}).get("cumulative_theta"),
                "trace_len": len(payload.get("derivation_trace") or []),
                "has_path_signature": isinstance(payload.get("path_signature"), str),
                "has_endpoint_signature": isinstance(payload.get("endpoint_signature"), str),
            }
            rows.append(row)
            runs.append(row)
        by_case[case.case_id] = runs

    # T2: stability for each expression across repeats.
    t2 = {}
    for case in cases:
        runs = by_case[case.case_id]
        psi_set = {r["psi_sig"] for r in runs}
        ei_set = {r["ei_sig"] for r in runs}
        t2[case.case_id] = {
            "expression": case.expression,
            "psi_stable": len(psi_set) == 1,
            "ei_stable": len(ei_set) == 1,
            "psi_variants": len(psi_set),
            "ei_variants": len(ei_set),
        }

    # T1/T3 checks on family pairs using first run representatives.
    first = {k: v[0] for k, v in by_case.items()}
    pair_map = {
        "precedence_pair_a": ("A1", "A2"),
        "assoc_add_pair_b": ("B1", "B2"),
        "assoc_mul_pair_c": ("C1", "C2"),
        "distrib_pair_d": ("D1", "D2"),
    }

    pair_results = {}
    t1_pass = True
    t3_witnesses = []
    for family, (left, right) in pair_map.items():
        l = first[left]
        r = first[right]
        psi_diff = l["psi_sig"] != r["psi_sig"]
        ei_diff = l["ei_sig"] != r["ei_sig"]
        value_equal = l["result"] == r["result"]
        pair_results[family] = {
            "left": {"case_id": left, "expression": l["expression"], "result": l["result"], "psi_sig": l["psi_sig"], "ei_sig": l["ei_sig"]},
            "right": {"case_id": right, "expression": r["expression"], "result": r["result"], "psi_sig": r["psi_sig"], "ei_sig": r["ei_sig"]},
            "psi_diff": psi_diff,
            "ei_diff": ei_diff,
            "value_equal": value_equal,
        }
        # For parenthesization/ordering pairs, PSI should differ.
        if family in {"precedence_pair_a", "assoc_add_pair_b", "assoc_mul_pair_c", "distrib_pair_d"} and not psi_diff:
            t1_pass = False
        if psi_diff and value_equal:
            t3_witnesses.append(family)

    report = {
        "api_url": API_URL,
        "signature_source": "explicit_api_fields",
        "required_signature_fields": ["path_signature", "endpoint_signature"],
        "run_repeats": RUN_REPEATS,
        "case_count": len(cases),
        "row_count": len(rows),
        "theorem_checks": {
            "T1_constraint_distinguishability_pass": t1_pass,
            "T2_endpoint_stability_all_pass": all(v["psi_stable"] and v["ei_stable"] for v in t2.values()),
            "T3_path_endpoint_decoupling_witnesses": t3_witnesses,
            "signature_fields_present_all": all(
                row["has_path_signature"] and row["has_endpoint_signature"] for row in rows
            ),
        },
        "stability": t2,
        "pair_results": pair_results,
        "psi_encoder_note": "Uses payload.path_signature directly from API response.",
        "ei_encoder_note": "Uses payload.endpoint_signature directly from API response.",
    }

    with CSV_PATH.open("w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow([
            "case_id",
            "family",
            "expression",
            "run",
            "result",
            "psi_sig",
            "ei_sig",
            "final_theta",
            "cumulative_theta",
            "trace_len",
        ])
        for r in rows:
            writer.writerow([
                r["case_id"],
                r["family"],
                r["expression"],
                r["run"],
                r["result"],
                r["psi_sig"],
                r["ei_sig"],
                r["final_theta"],
                r["cumulative_theta"],
                r["trace_len"],
            ])

    JSON_PATH.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
