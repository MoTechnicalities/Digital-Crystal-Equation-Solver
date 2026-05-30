#!/bin/python3
"""Adversarial negative-side RH attack harness.

Searches for off-critical-line points where |zeta(s)| is unusually small.
This is not a proof/disproof tool; it produces structured candidates for
follow-up validation.
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
OUT = ART / "rh_counterexample_candidates.json"

# First known critical-line zero heights. We intentionally probe around these
# heights but off the critical line.
ZERO_HEIGHTS = [
    14.134725141,
    21.022039639,
    25.010857580,
    30.424876126,
    32.935061588,
]

SIGMA_GRID = [0.35, 0.40, 0.45, 0.55, 0.60, 0.65]
T_OFFSETS = [-0.10, -0.05, -0.02, -0.01, 0.01, 0.02, 0.05, 0.10]

# Candidate threshold is intentionally strict to avoid noisy false positives.
CANDIDATE_EPS = 1e-3
VALIDATION_EPS = 1e-6


@dataclass(frozen=True)
class ProbePoint:
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
    raise ValueError(f"unexpected zeta payload result: {value!r}")


def zeta_abs_at(point: ProbePoint) -> tuple[float, tuple[float, float]]:
    expr = f"zeta({point.sigma}+{point.t}i)"
    payload = eval_expr(expr)
    if payload.get("error"):
        raise RuntimeError(f"zeta evaluation failed at {expr}: {payload['error']}")
    re_part, im_part = parse_complex(payload.get("result"))
    return math.hypot(re_part, im_part), (re_part, im_part)


def symmetry_residual(sigma: float, t: float) -> float:
    # Basic geometric anti-logic consistency check:
    # compare |zeta(sigma+it)| vs |zeta(1-sigma+it)|.
    left, _ = zeta_abs_at(ProbePoint(sigma, t))
    right, _ = zeta_abs_at(ProbePoint(1.0 - sigma, t))
    return abs(left - right)


def main() -> None:
    ART.mkdir(parents=True, exist_ok=True)

    probes = []
    for base_t in ZERO_HEIGHTS:
        for sigma in SIGMA_GRID:
            for dt in T_OFFSETS:
                probes.append(ProbePoint(sigma=sigma, t=base_t + dt))

    candidates = []
    failures = []
    min_seen = {
        "sigma": None,
        "t": None,
        "zeta_abs": float("inf"),
    }

    for point in probes:
        try:
            z_abs, (re_part, im_part) = zeta_abs_at(point)
            if z_abs < min_seen["zeta_abs"]:
                min_seen = {
                    "sigma": point.sigma,
                    "t": point.t,
                    "zeta_abs": z_abs,
                }

            if z_abs <= CANDIDATE_EPS:
                sym = symmetry_residual(point.sigma, point.t)
                validated = z_abs <= VALIDATION_EPS and sym <= VALIDATION_EPS
                candidates.append(
                    {
                        "sigma": point.sigma,
                        "t": point.t,
                        "zeta_re": re_part,
                        "zeta_im": im_part,
                        "zeta_abs": z_abs,
                        "off_critical_line": abs(point.sigma - 0.5) > 1e-12,
                        "symmetry_residual": sym,
                        "validated": validated,
                        "validation_note": (
                            "validated by strict epsilon and symmetry residual"
                            if validated
                            else "candidate only; requires stronger independent validation"
                        ),
                    }
                )
        except Exception as exc:  # noqa: BLE001
            failures.append(
                {
                    "sigma": point.sigma,
                    "t": point.t,
                    "error": str(exc),
                }
            )

    candidates.sort(key=lambda item: item["zeta_abs"])

    result = {
        "program": "RH Counterexample Attack Harness",
        "version": "v0.1",
        "api_url": API_URL,
        "search_space": {
            "zero_heights": ZERO_HEIGHTS,
            "sigma_grid": SIGMA_GRID,
            "t_offsets": T_OFFSETS,
            "candidate_eps": CANDIDATE_EPS,
            "validation_eps": VALIDATION_EPS,
            "probe_count": len(probes),
        },
        "summary": {
            "candidate_count": len(candidates),
            "validated_count": sum(1 for item in candidates if item["validated"]),
            "failure_count": len(failures),
            "minimum_probe_abs": min_seen,
        },
        "candidates": candidates,
        "failures": failures,
        "scope_note": "Adversarial internal search only. Any candidate requires independent external verification.",
    }

    OUT.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
