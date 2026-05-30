#!/bin/python3
"""Adaptive off-line zero hunt with local refinement.

Runs a coarse search near known zero heights, ranks low-|zeta| points, and
performs local coordinate descent refinement around top seeds.
"""

from __future__ import annotations

import json
import math
import urllib.request
from dataclasses import dataclass
from pathlib import Path

API_URL = "http://127.0.0.1:8080/v1/csif/math"
ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / "docs" / "findings" / "artifacts"
OUT = ART / "rh_counterexample_refine_candidates.json"

ZERO_HEIGHTS = [
    14.134725141,
    21.022039639,
    25.010857580,
    30.424876126,
    32.935061588,
]
SIGMA_GRID = [0.35, 0.40, 0.45, 0.55, 0.60, 0.65]
T_OFFSETS = [-0.20, -0.10, -0.05, -0.02, -0.01, 0.01, 0.02, 0.05, 0.10, 0.20]

TOP_SEEDS = 8
REFINE_ITERS = 8
INITIAL_SIGMA_STEP = 0.02
INITIAL_T_STEP = 0.04

CANDIDATE_EPS = 1e-3
VALIDATION_EPS = 1e-6


@dataclass(frozen=True)
class Point:
    sigma: float
    t: float


def eval_expr(expr: str) -> dict:
    payload = {
        "expression": expr,
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


def parse_complex(value: object) -> tuple[float, float]:
    if isinstance(value, dict) and "re" in value and "im" in value:
        return float(value["re"]), float(value["im"])
    if isinstance(value, (int, float)):
        return float(value), 0.0
    raise ValueError(f"unexpected value payload: {value!r}")


def zeta_abs(point: Point) -> tuple[float, float, float]:
    payload = eval_expr(f"zeta({point.sigma}+{point.t}i)")
    if payload.get("error"):
        raise RuntimeError(str(payload["error"]))
    re_part, im_part = parse_complex(payload.get("result"))
    return math.hypot(re_part, im_part), re_part, im_part


def symmetry_proxy(point: Point) -> float:
    left, _, _ = zeta_abs(point)
    right, _, _ = zeta_abs(Point(1.0 - point.sigma, point.t))
    return abs(left - right)


def coarse_scan() -> list[dict]:
    rows = []
    for base_t in ZERO_HEIGHTS:
        for sigma in SIGMA_GRID:
            for dt in T_OFFSETS:
                point = Point(sigma=sigma, t=base_t + dt)
                z_abs, re_part, im_part = zeta_abs(point)
                rows.append(
                    {
                        "sigma": point.sigma,
                        "t": point.t,
                        "zeta_abs": z_abs,
                        "zeta_re": re_part,
                        "zeta_im": im_part,
                        "off_critical_line": abs(point.sigma - 0.5) > 1e-12,
                    }
                )
    rows.sort(key=lambda item: item["zeta_abs"])
    return rows


def refine_seed(seed: Point) -> dict:
    current = seed
    best_abs, best_re, best_im = zeta_abs(current)
    sigma_step = INITIAL_SIGMA_STEP
    t_step = INITIAL_T_STEP

    for _ in range(REFINE_ITERS):
        improved = False
        neighbors = [
            Point(current.sigma + sigma_step, current.t),
            Point(current.sigma - sigma_step, current.t),
            Point(current.sigma, current.t + t_step),
            Point(current.sigma, current.t - t_step),
            Point(current.sigma + sigma_step, current.t + t_step),
            Point(current.sigma + sigma_step, current.t - t_step),
            Point(current.sigma - sigma_step, current.t + t_step),
            Point(current.sigma - sigma_step, current.t - t_step),
        ]

        for n in neighbors:
            # Keep this adversarial search off critical line for negative-side pressure.
            if abs(n.sigma - 0.5) < 1e-9:
                continue
            if n.sigma <= 0.0 or n.sigma >= 1.0:
                continue
            z_abs, re_part, im_part = zeta_abs(n)
            if z_abs < best_abs:
                current = n
                best_abs = z_abs
                best_re = re_part
                best_im = im_part
                improved = True

        if not improved:
            sigma_step *= 0.5
            t_step *= 0.5

    sym = symmetry_proxy(current)
    validated = best_abs <= VALIDATION_EPS and sym <= VALIDATION_EPS

    return {
        "seed_sigma": seed.sigma,
        "seed_t": seed.t,
        "sigma": current.sigma,
        "t": current.t,
        "zeta_abs": best_abs,
        "zeta_re": best_re,
        "zeta_im": best_im,
        "symmetry_residual": sym,
        "off_critical_line": abs(current.sigma - 0.5) > 1e-12,
        "candidate": best_abs <= CANDIDATE_EPS,
        "validated": validated,
    }


def main() -> None:
    ART.mkdir(parents=True, exist_ok=True)

    coarse_rows = coarse_scan()
    seeds = [Point(item["sigma"], item["t"]) for item in coarse_rows[:TOP_SEEDS]]
    refined = [refine_seed(seed) for seed in seeds]
    refined.sort(key=lambda item: item["zeta_abs"])

    result = {
        "program": "RH Counterexample Adaptive Refine",
        "version": "v0.1",
        "api_url": API_URL,
        "search": {
            "top_seed_count": TOP_SEEDS,
            "refine_iters": REFINE_ITERS,
            "candidate_eps": CANDIDATE_EPS,
            "validation_eps": VALIDATION_EPS,
            "coarse_probe_count": len(coarse_rows),
        },
        "coarse_best": coarse_rows[:TOP_SEEDS],
        "refined": refined,
        "summary": {
            "candidate_count": sum(1 for item in refined if item["candidate"]),
            "validated_count": sum(1 for item in refined if item["validated"]),
            "best_refined_abs": refined[0]["zeta_abs"] if refined else None,
        },
        "scope_note": "Exploratory adaptive minimization only; candidates require independent high-precision validation.",
    }

    OUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
