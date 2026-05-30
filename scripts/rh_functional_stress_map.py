#!/bin/python3
"""Functional-equation stress proxy map for off-line exploration.

Computes a symmetry residual proxy |zeta(s)| - |zeta(1-s)| over a mesh and
ranks high-stress points for follow-up.
"""

from __future__ import annotations

import csv
import json
import math
import urllib.request
from pathlib import Path

API_URL = "http://127.0.0.1:8080/v1/csif/math"
ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "docs" / "findings" / "artifacts"
CSV_OUT = ART / "rh_functional_stress_map.csv"
JSON_OUT = ART / "rh_functional_stress_summary.json"

SIGMA_GRID = [0.30, 0.35, 0.40, 0.45, 0.55, 0.60, 0.65, 0.70]
T_GRID = [
    14.00,
    14.10,
    14.20,
    21.00,
    21.05,
    21.10,
    25.00,
    25.05,
    25.10,
    30.35,
    30.40,
    30.45,
]


def eval_expr(expr: str) -> dict:
    payload = {"expression": expr, "mode": "algebraic", "angle_unit": "radians"}
    req = urllib.request.Request(
        API_URL,
        data=json.dumps(payload).encode("utf-8"),
        method="POST",
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def parse_complex(value: object) -> tuple[float, float]:
    if isinstance(value, dict) and "re" in value and "im" in value:
        return float(value["re"]), float(value["im"])
    if isinstance(value, (int, float)):
        return float(value), 0.0
    raise ValueError(f"unexpected complex payload: {value!r}")


def zeta_abs(sigma: float, t: float) -> float:
    payload = eval_expr(f"zeta({sigma}+{t}i)")
    if payload.get("error"):
        raise RuntimeError(str(payload["error"]))
    re_part, im_part = parse_complex(payload.get("result"))
    return math.hypot(re_part, im_part)


def main() -> None:
    ART.mkdir(parents=True, exist_ok=True)

    rows = []
    for sigma in SIGMA_GRID:
        for t in T_GRID:
            left = zeta_abs(sigma, t)
            right = zeta_abs(1.0 - sigma, t)
            residual = abs(left - right)
            rows.append(
                {
                    "sigma": sigma,
                    "one_minus_sigma": 1.0 - sigma,
                    "t": t,
                    "zeta_abs_sigma": left,
                    "zeta_abs_reflect": right,
                    "symmetry_residual": residual,
                }
            )

    rows.sort(key=lambda item: item["symmetry_residual"], reverse=True)

    with CSV_OUT.open("w", newline="", encoding="utf-8") as fp:
        writer = csv.DictWriter(
            fp,
            fieldnames=[
                "sigma",
                "one_minus_sigma",
                "t",
                "zeta_abs_sigma",
                "zeta_abs_reflect",
                "symmetry_residual",
            ],
        )
        writer.writeheader()
        writer.writerows(rows)

    summary = {
        "program": "RH Functional Stress Map Proxy",
        "version": "v0.1",
        "api_url": API_URL,
        "probe_count": len(rows),
        "top_stress_points": rows[:12],
        "max_residual": rows[0]["symmetry_residual"] if rows else None,
        "median_residual": rows[len(rows) // 2]["symmetry_residual"] if rows else None,
        "scope_note": "Symmetry residual proxy for exploratory targeting; not a formal functional-equation proof metric.",
    }

    JSON_OUT.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
