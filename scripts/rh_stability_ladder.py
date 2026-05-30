#!/bin/python3
"""Stability ladder for candidate points.

Uses multi-scale neighborhood perturbations to assess whether low-|zeta| points
remain stable under local coordinate changes.
"""

from __future__ import annotations

import json
import math
import urllib.request
from pathlib import Path

API_URL = "http://127.0.0.1:8080/v1/csif/math"
ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "docs" / "findings" / "artifacts"
REFINE_PATH = ART / "rh_counterexample_refine_candidates.json"
OUT = ART / "rh_stability_ladder.json"

EPS_LEVELS = [1e-2, 5e-3, 1e-3]
TOP_POINTS = 6


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
    raise ValueError(f"unexpected complex value: {value!r}")


def zeta_abs(sigma: float, t: float) -> float:
    payload = eval_expr(f"zeta({sigma}+{t}i)")
    if payload.get("error"):
        raise RuntimeError(str(payload["error"]))
    re_part, im_part = parse_complex(payload.get("result"))
    return math.hypot(re_part, im_part)


def neighborhood_abs(sigma: float, t: float, eps: float) -> list[float]:
    points = [
        (sigma, t),
        (sigma + eps, t),
        (sigma - eps, t),
        (sigma, t + eps),
        (sigma, t - eps),
        (sigma + eps, t + eps),
        (sigma + eps, t - eps),
        (sigma - eps, t + eps),
        (sigma - eps, t - eps),
    ]
    values = []
    for s, tt in points:
        if s <= 0.0 or s >= 1.0:
            continue
        values.append(zeta_abs(s, tt))
    return values


def main() -> None:
    ART.mkdir(parents=True, exist_ok=True)

    if not REFINE_PATH.exists():
        raise SystemExit("missing refine artifact: docs/findings/artifacts/rh_counterexample_refine_candidates.json")

    refine = json.loads(REFINE_PATH.read_text(encoding="utf-8"))
    points = (refine.get("refined") or [])[:TOP_POINTS]

    ladder = []
    for item in points:
        sigma = float(item["sigma"])
        t = float(item["t"])
        base = float(item["zeta_abs"])

        levels = []
        stable_all = True
        for eps in EPS_LEVELS:
            vals = neighborhood_abs(sigma, t, eps)
            if not vals:
                continue
            span = max(vals) - min(vals)
            ratio = span / max(base, 1e-12)
            stable = ratio < 1.0
            levels.append(
                {
                    "eps": eps,
                    "min_abs": min(vals),
                    "max_abs": max(vals),
                    "span": span,
                    "span_ratio_to_base": ratio,
                    "stable": stable,
                }
            )
            stable_all = stable_all and stable

        ladder.append(
            {
                "sigma": sigma,
                "t": t,
                "base_abs": base,
                "levels": levels,
                "stable_all_levels": stable_all,
            }
        )

    result = {
        "program": "RH Stability Ladder",
        "version": "v0.1",
        "api_url": API_URL,
        "source_refine_artifact": str(REFINE_PATH.relative_to(ROOT)),
        "points_analyzed": len(ladder),
        "eps_levels": EPS_LEVELS,
        "ladder": ladder,
        "summary": {
            "stable_points": sum(1 for item in ladder if item["stable_all_levels"]),
            "unstable_points": sum(1 for item in ladder if not item["stable_all_levels"]),
        },
        "scope_note": "Neighborhood stability proxy only; not a substitute for arbitrary-precision certification.",
    }

    OUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
