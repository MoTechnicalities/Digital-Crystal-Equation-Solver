#!/bin/python3
"""Dipole-style symmetry analysis for RH exploratory search.

For each height t and offset delta, sample mirrored points around the critical
line:
  s+ = 0.5 + delta + i t
  s- = 0.5 - delta + i t
Then compute a dipole vector zeta(s+) - zeta(s-) and derived metrics.

This is an exploratory targeting signal, not a formal proof certificate.
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
JSON_OUT = ART / "rh_dipole_analysis.json"
CSV_OUT = ART / "rh_dipole_analysis.csv"

T_SAMPLES = [
    14.134725141,
    21.022039639,
    25.010857580,
    30.424876126,
    32.935061588,
]
DELTA_SAMPLES = [0.01, 0.02, 0.05, 0.10, 0.15]


def eval_expr(expr: str) -> dict:
    payload = {
        "expression": expr,
        "mode": "algebraic",
        "angle_unit": "radians",
    }
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


def zeta_complex(sigma: float, t: float) -> tuple[float, float]:
    payload = eval_expr(f"zeta({sigma}+{t}i)")
    if payload.get("error"):
        raise RuntimeError(str(payload["error"]))
    return parse_complex(payload.get("result"))


def c_abs(z: tuple[float, float]) -> float:
    return math.hypot(z[0], z[1])


def main() -> None:
    ART.mkdir(parents=True, exist_ok=True)

    rows: list[dict] = []
    for t in T_SAMPLES:
        center = zeta_complex(0.5, t)
        center_abs = c_abs(center)
        for delta in DELTA_SAMPLES:
            z_plus = zeta_complex(0.5 + delta, t)
            z_minus = zeta_complex(0.5 - delta, t)

            dipole_vec = (z_plus[0] - z_minus[0], z_plus[1] - z_minus[1])
            dipole_strength = c_abs(dipole_vec)
            plus_abs = c_abs(z_plus)
            minus_abs = c_abs(z_minus)
            mirror_abs_gap = abs(plus_abs - minus_abs)
            normalized_gap = mirror_abs_gap / max(plus_abs + minus_abs, 1e-12)

            # Off-line asymmetry pressure: high when one side differs strongly.
            asymmetry_score = normalized_gap * (1.0 + dipole_strength)

            rows.append(
                {
                    "t": t,
                    "delta": delta,
                    "zeta_plus_re": z_plus[0],
                    "zeta_plus_im": z_plus[1],
                    "zeta_minus_re": z_minus[0],
                    "zeta_minus_im": z_minus[1],
                    "zeta_center_re": center[0],
                    "zeta_center_im": center[1],
                    "zeta_plus_abs": plus_abs,
                    "zeta_minus_abs": minus_abs,
                    "zeta_center_abs": center_abs,
                    "dipole_re": dipole_vec[0],
                    "dipole_im": dipole_vec[1],
                    "dipole_strength": dipole_strength,
                    "mirror_abs_gap": mirror_abs_gap,
                    "normalized_gap": normalized_gap,
                    "asymmetry_score": asymmetry_score,
                }
            )

    rows_by_asym = sorted(rows, key=lambda r: r["asymmetry_score"], reverse=True)
    rows_by_center = sorted(rows, key=lambda r: r["zeta_center_abs"])

    with CSV_OUT.open("w", newline="", encoding="utf-8") as fp:
        writer = csv.DictWriter(fp, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)

    result = {
        "program": "RH Dipole Symmetry Analysis",
        "version": "v0.1",
        "api_url": API_URL,
        "sample_config": {
            "t_samples": T_SAMPLES,
            "delta_samples": DELTA_SAMPLES,
            "probe_count": len(rows),
        },
        "top_asymmetry_windows": rows_by_asym[:10],
        "top_low_center_windows": rows_by_center[:10],
        "summary": {
            "max_asymmetry_score": rows_by_asym[0]["asymmetry_score"] if rows_by_asym else None,
            "min_center_abs": rows_by_center[0]["zeta_center_abs"] if rows_by_center else None,
            "note": "High asymmetry windows are follow-up targets for anti-side stress probing.",
        },
        "scope_note": "Exploratory dipole metric only; not a theorem-proof or zero-certificate.",
    }

    JSON_OUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
