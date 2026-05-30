#!/bin/python3
"""Inference for hafnian phase-transition sweep.

Reads transition sweep CSV and computes per-dimension coherence-threshold estimates
for residual cliff crossing with bootstrap confidence bands.
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
CSV_PATH = ARTIFACT_DIR / "hafnian_flux_transition_sweep.csv"
OUT_PATH = ARTIFACT_DIR / "hafnian_flux_transition_thresholds.json"

BOOTSTRAP_SEED = 20260530
BOOTSTRAP_ROUNDS = 2000
RESIDUAL_THRESHOLD = 0.10
COHERENCE_BIN_COUNT = 10


@dataclass
class Record:
    dimension: int
    coherence: float
    residual_abs: float


def percentile(values: list[float], p: float) -> float:
    if not values:
        raise ValueError("Empty list")
    vals = sorted(values)
    pos = (len(vals) - 1) * p
    lo = int(math.floor(pos))
    hi = int(math.ceil(pos))
    if lo == hi:
        return vals[lo]
    frac = pos - lo
    return vals[lo] * (1 - frac) + vals[hi] * frac


def load_records() -> list[Record]:
    records: list[Record] = []
    with CSV_PATH.open("r", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            records.append(
                Record(
                    dimension=int(row["dimension"]),
                    coherence=float(row["coherence_magnitude"]),
                    residual_abs=float(row["residual_abs"]),
                )
            )
    return records


def estimate_threshold(records: list[Record], residual_threshold: float) -> float | None:
    if not records:
        return None
    cmin = min(r.coherence for r in records)
    cmax = max(r.coherence for r in records)
    if abs(cmax - cmin) < 1e-12:
        return None

    bins = [[] for _ in range(COHERENCE_BIN_COUNT)]
    for rec in records:
        idx = int((rec.coherence - cmin) / (cmax - cmin) * COHERENCE_BIN_COUNT)
        if idx == COHERENCE_BIN_COUNT:
            idx -= 1
        idx = max(0, min(COHERENCE_BIN_COUNT - 1, idx))
        bins[idx].append(rec.residual_abs)

    bin_means: list[tuple[float, float]] = []
    for i, bucket in enumerate(bins):
        if not bucket:
            continue
        center = cmin + (i + 0.5) / COHERENCE_BIN_COUNT * (cmax - cmin)
        mean_resid = statistics.fmean(bucket)
        bin_means.append((center, mean_resid))

    if len(bin_means) < 2:
        return None

    # Traverse from high coherence to low coherence, find first threshold crossing.
    bin_means.sort(key=lambda x: x[0], reverse=True)
    for i in range(len(bin_means) - 1):
        c1, r1 = bin_means[i]
        c2, r2 = bin_means[i + 1]
        if r1 <= residual_threshold < r2:
            if abs(r2 - r1) < 1e-12:
                return (c1 + c2) / 2.0
            t = (residual_threshold - r1) / (r2 - r1)
            return c1 + t * (c2 - c1)

    # If always above or always below threshold, return edge indicator.
    max_r = max(r for _, r in bin_means)
    min_r = min(r for _, r in bin_means)
    if min_r > residual_threshold:
        return min(c for c, _ in bin_means)
    if max_r <= residual_threshold:
        return max(c for c, _ in bin_means)
    return None


def bootstrap_threshold_ci(records: list[Record], rounds: int, rng: random.Random, residual_threshold: float) -> dict:
    est = estimate_threshold(records, residual_threshold)
    if est is None:
        return {
            "estimate": None,
            "ci95_low": None,
            "ci95_high": None,
            "valid_bootstrap_samples": 0,
        }

    n = len(records)
    samples = []
    for _ in range(rounds):
        draw = [records[rng.randrange(n)] for _ in range(n)]
        value = estimate_threshold(draw, residual_threshold)
        if value is not None:
            samples.append(value)

    if not samples:
        return {
            "estimate": est,
            "ci95_low": None,
            "ci95_high": None,
            "valid_bootstrap_samples": 0,
        }

    return {
        "estimate": est,
        "ci95_low": percentile(samples, 0.025),
        "ci95_high": percentile(samples, 0.975),
        "valid_bootstrap_samples": len(samples),
    }


def main() -> None:
    all_records = load_records()
    if not all_records:
        raise RuntimeError(f"No records found in {CSV_PATH}")

    by_dim: dict[int, list[Record]] = {}
    for rec in all_records:
        by_dim.setdefault(rec.dimension, []).append(rec)

    rng = random.Random(BOOTSTRAP_SEED)

    thresholds = {}
    for dim in sorted(by_dim):
        records = by_dim[dim]
        ci = bootstrap_threshold_ci(records, BOOTSTRAP_ROUNDS, rng, RESIDUAL_THRESHOLD)
        thresholds[str(dim)] = {
            "count": len(records),
            "coherence_min": min(r.coherence for r in records),
            "coherence_max": max(r.coherence for r in records),
            "residual_abs_mean": statistics.fmean(r.residual_abs for r in records),
            "residual_abs_max": max(r.residual_abs for r in records),
            "coherence_threshold_for_residual": ci,
        }

    result = {
        "source_csv": str(CSV_PATH.relative_to(ROOT)),
        "bootstrap": {
            "rounds": BOOTSTRAP_ROUNDS,
            "seed": BOOTSTRAP_SEED,
        },
        "residual_threshold": RESIDUAL_THRESHOLD,
        "dimensions": thresholds,
        "notes": [
            "Threshold is estimated where mean residual crosses the configured cutoff as coherence decreases.",
            "CI bands are bootstrap percentile intervals from resampled rows per dimension.",
        ],
    }

    OUT_PATH.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
