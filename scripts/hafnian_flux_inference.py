#!/bin/python3
"""Statistical inference layer for hafnian flux sweep artifacts.

Reads docs/findings/artifacts/hafnian_flux_sweep.csv and writes
hafnian_flux_sweep_inference.json with correlation and bootstrap summaries.
"""

from __future__ import annotations

import csv
import json
import math
import random
import statistics
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT_DIR = ROOT / "docs" / "findings" / "artifacts"
CSV_PATH = ARTIFACT_DIR / "hafnian_flux_sweep.csv"
OUT_PATH = ARTIFACT_DIR / "hafnian_flux_sweep_inference.json"

BOOTSTRAP_SEED = 20260529
BOOTSTRAP_ROUNDS = 4000


@dataclass
class Record:
    family: str
    coherence: float
    symmetry_gap: float
    residual_abs: float


def percentile(values: list[float], p: float) -> float:
    if not values:
        raise ValueError("percentile called with empty values")
    sorted_vals = sorted(values)
    pos = (len(sorted_vals) - 1) * p
    lo = int(math.floor(pos))
    hi = int(math.ceil(pos))
    if lo == hi:
        return sorted_vals[lo]
    frac = pos - lo
    return sorted_vals[lo] * (1 - frac) + sorted_vals[hi] * frac


def pearson(xs: list[float], ys: list[float]) -> float:
    if len(xs) != len(ys) or len(xs) < 2:
        raise ValueError("pearson requires equal-length arrays with n >= 2")
    mx = statistics.fmean(xs)
    my = statistics.fmean(ys)
    dx = [x - mx for x in xs]
    dy = [y - my for y in ys]
    cov = sum(a * b for a, b in zip(dx, dy))
    vx = sum(a * a for a in dx)
    vy = sum(b * b for b in dy)
    denom = math.sqrt(vx * vy)
    if denom <= 1e-12:
        return 0.0
    return cov / denom


def load_records() -> list[Record]:
    records: list[Record] = []
    with CSV_PATH.open("r", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            records.append(
                Record(
                    family=row["family"],
                    coherence=float(row["coherence_magnitude"]),
                    symmetry_gap=float(row["symmetry_gap_mean_abs"]),
                    residual_abs=abs(float(row["uniform_phase_residual"])),
                )
            )
    return records


def bootstrap_mean_ci(values: list[float], rounds: int, rng: random.Random) -> dict:
    if not values:
        return {"mean": None, "ci95_low": None, "ci95_high": None}
    samples = []
    n = len(values)
    for _ in range(rounds):
        draw = [values[rng.randrange(n)] for _ in range(n)]
        samples.append(statistics.fmean(draw))
    return {
        "mean": statistics.fmean(values),
        "ci95_low": percentile(samples, 0.025),
        "ci95_high": percentile(samples, 0.975),
    }


def main() -> None:
    records = load_records()
    if not records:
        raise RuntimeError(f"No records found in {CSV_PATH}")

    coherence = [r.coherence for r in records]
    symmetry_gap = [r.symmetry_gap for r in records]
    residual_abs = [r.residual_abs for r in records]

    rng = random.Random(BOOTSTRAP_SEED)

    by_family: dict[str, list[Record]] = {}
    for rec in records:
        by_family.setdefault(rec.family, []).append(rec)

    family_stats = {}
    for family, items in by_family.items():
        vals = [r.residual_abs for r in items]
        family_stats[family] = {
            "count": len(vals),
            "residual_abs": bootstrap_mean_ci(vals, BOOTSTRAP_ROUNDS, rng),
            "coherence_mean": statistics.fmean(r.coherence for r in items),
            "symmetry_gap_mean": statistics.fmean(r.symmetry_gap for r in items),
        }

    result = {
        "source_csv": str(CSV_PATH.relative_to(ROOT)),
        "total_records": len(records),
        "bootstrap": {
            "rounds": BOOTSTRAP_ROUNDS,
            "seed": BOOTSTRAP_SEED,
        },
        "correlations": {
            "pearson_residual_abs_vs_coherence": pearson(residual_abs, coherence),
            "pearson_residual_abs_vs_symmetry_gap": pearson(residual_abs, symmetry_gap),
        },
        "overall": {
            "residual_abs": bootstrap_mean_ci(residual_abs, BOOTSTRAP_ROUNDS, rng),
        },
        "families": family_stats,
        "notes": [
            "Correlation signs indicate direction only; no causal proof is claimed.",
            "Bootstrap CIs are empirical intervals over this fixed-sweep dataset.",
        ],
    }

    OUT_PATH.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
