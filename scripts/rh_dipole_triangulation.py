#!/bin/python3
"""Adaptive triangulation follow-up for RH dipole hot windows.

Consumes dipole-analysis hot windows and performs local triangular probes around
both mirrored sides to produce higher-resolution follow-up targets.

This is an exploratory targeting layer, not a proof certificate.
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
DIPOLE_IN = ART / "rh_dipole_analysis.json"
JSON_OUT = ART / "rh_dipole_triangulation.json"
CSV_OUT = ART / "rh_dipole_triangulation.csv"

TOP_WINDOWS = 6
SIGMA_STEP = 0.01
T_STEP = 0.02


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


def zeta_complex(sigma: float, t: float) -> tuple[float, float]:
    payload = eval_expr(f"zeta({sigma}+{t}i)")
    if payload.get("error"):
        raise RuntimeError(str(payload["error"]))
    return parse_complex(payload.get("result"))


def c_abs(z: tuple[float, float]) -> float:
    return math.hypot(z[0], z[1])


def triangle_points(base_sigma: float, t: float) -> list[tuple[float, float]]:
    return [
        (base_sigma, t),
        (base_sigma + SIGMA_STEP, t + T_STEP),
        (base_sigma - SIGMA_STEP, t + T_STEP),
    ]


def window_triangulation(t: float, delta: float) -> dict:
    right_base = 0.5 + delta
    left_base = 0.5 - delta

    right_tri = triangle_points(right_base, t)
    left_tri = triangle_points(left_base, t)

    rows = []
    for side, points in (("right", right_tri), ("left", left_tri)):
        for idx, (sigma, tt) in enumerate(points):
            if sigma <= 0.0 or sigma >= 1.0:
                continue
            z = zeta_complex(sigma, tt)
            rows.append(
                {
                    "side": side,
                    "vertex": idx,
                    "sigma": sigma,
                    "t": tt,
                    "zeta_re": z[0],
                    "zeta_im": z[1],
                    "zeta_abs": c_abs(z),
                }
            )

    right = [r for r in rows if r["side"] == "right"]
    left = [r for r in rows if r["side"] == "left"]

    right_best = min(right, key=lambda r: r["zeta_abs"]) if right else None
    left_best = min(left, key=lambda r: r["zeta_abs"]) if left else None

    right_mean = sum(r["zeta_abs"] for r in right) / max(len(right), 1)
    left_mean = sum(r["zeta_abs"] for r in left) / max(len(left), 1)
    side_gap = abs(right_mean - left_mean)

    return {
        "t": t,
        "delta": delta,
        "right_mean_abs": right_mean,
        "left_mean_abs": left_mean,
        "triangle_side_gap": side_gap,
        "right_best": right_best,
        "left_best": left_best,
        "tri_points": rows,
    }


def main() -> None:
    ART.mkdir(parents=True, exist_ok=True)

    if not DIPOLE_IN.exists():
        raise SystemExit("missing input artifact: docs/findings/artifacts/rh_dipole_analysis.json")

    dipole = json.loads(DIPOLE_IN.read_text(encoding="utf-8"))
    hot = (dipole.get("top_asymmetry_windows") or [])[:TOP_WINDOWS]

    windows = []
    flat_rows = []

    for item in hot:
        t = float(item["t"])
        delta = float(item["delta"])
        tri = window_triangulation(t, delta)
        windows.append(tri)

        for row in tri["tri_points"]:
            flat_rows.append(
                {
                    "t": tri["t"],
                    "delta": tri["delta"],
                    "side": row["side"],
                    "vertex": row["vertex"],
                    "sigma": row["sigma"],
                    "probe_t": row["t"],
                    "zeta_re": row["zeta_re"],
                    "zeta_im": row["zeta_im"],
                    "zeta_abs": row["zeta_abs"],
                    "triangle_side_gap": tri["triangle_side_gap"],
                }
            )

    windows_by_gap = sorted(windows, key=lambda w: w["triangle_side_gap"], reverse=True)
    best_points = sorted(
        [
            w["right_best"]
            for w in windows
            if isinstance(w.get("right_best"), dict)
        ]
        + [
            w["left_best"]
            for w in windows
            if isinstance(w.get("left_best"), dict)
        ],
        key=lambda r: r["zeta_abs"],
    )

    if flat_rows:
        with CSV_OUT.open("w", newline="", encoding="utf-8") as fp:
            writer = csv.DictWriter(fp, fieldnames=list(flat_rows[0].keys()))
            writer.writeheader()
            writer.writerows(flat_rows)

    result = {
        "program": "RH Dipole Triangulation",
        "version": "v0.1",
        "api_url": API_URL,
        "source_dipole_artifact": str(DIPOLE_IN.relative_to(ROOT)),
        "config": {
            "top_windows": TOP_WINDOWS,
            "sigma_step": SIGMA_STEP,
            "t_step": T_STEP,
            "window_count": len(windows),
            "probe_count": len(flat_rows),
        },
        "top_triangle_gap_windows": windows_by_gap[:6],
        "top_low_abs_points": best_points[:12],
        "summary": {
            "max_triangle_side_gap": windows_by_gap[0]["triangle_side_gap"] if windows_by_gap else None,
            "min_triangulated_abs": best_points[0]["zeta_abs"] if best_points else None,
        },
        "scope_note": "Triangulation is a local refinement heuristic; any counterexample claim still requires rigorous certification.",
    }

    JSON_OUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
