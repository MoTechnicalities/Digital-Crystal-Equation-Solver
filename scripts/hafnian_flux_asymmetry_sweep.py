#!/bin/python3
"""Asymmetry-only hafnian flux sweep.

Protocol intent: isolate symmetry-gap effects while keeping high phase coherence
in the upper-triangle seed structure.
"""

from __future__ import annotations

import csv
import json
import math
import random
import statistics
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path

API_URL = "http://127.0.0.1:8080/v1/csif/math"
SEED = 20260529
DIMENSION = 6
CASES_PER_LEVEL = 16
ASYM_LEVELS = [0.0, 0.1, 0.2, 0.35, 0.5, 0.7]

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT_DIR = ROOT / "docs" / "findings" / "artifacts"
CSV_PATH = ARTIFACT_DIR / "hafnian_flux_asymmetry_sweep.csv"
SUMMARY_PATH = ARTIFACT_DIR / "hafnian_flux_asymmetry_sweep_summary.json"
PLOT_PATH = ARTIFACT_DIR / "hafnian_flux_asymmetry_residual_vs_gap.svg"


@dataclass
class Row:
    asymmetry_level: float
    case_id: str
    coherence_magnitude: float
    symmetry_gap_mean_abs: float
    observed_hafnian_theta: float
    predicted_uniform_theta: float
    uniform_phase_residual: float
    mean_edge_phase: float


def wrap_pi(theta: float) -> float:
    return (theta + math.pi) % (2 * math.pi) - math.pi


def format_num(value: float) -> str:
    if abs(value) < 1e-12:
        value = 0.0
    return f"{value:.6f}"


def complex_literal(re: float, im: float) -> str:
    re_s = format_num(re)
    im_s = format_num(im)
    if im_s.startswith("-"):
        return f"({re_s}{im_s}i)"
    return f"({re_s}+{im_s}i)"


def matrix_to_expression(matrix: list[list[complex]]) -> str:
    rows = []
    for row in matrix:
        rows.append("[" + ",".join(complex_literal(v.real, v.imag) for v in row) + "]")
    return "hafnian([" + ",".join(rows) + "])"


def post_expression(expression: str) -> dict:
    payload = {
        "expression": expression,
        "mode": "algebraic",
        "angle_unit": "radians",
    }
    body = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        API_URL,
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        raise RuntimeError(f"HTTP {exc.code}: {exc.read().decode('utf-8', errors='replace')}") from exc


def flux_probe_from_response(response: dict) -> dict:
    trace = response.get("derivation_trace") or []
    for step in trace:
        trust = (step or {}).get("numeric_trust")
        if not trust:
            continue
        probe = trust.get("hafnian_flux_probe")
        if probe:
            return probe
    raise RuntimeError("No hafnian_flux_probe in derivation_trace")


def build_high_coherence_symmetric_matrix(n: int, rng: random.Random) -> list[list[complex]]:
    matrix = [[0j for _ in range(n)] for _ in range(n)]
    for i in range(n):
        for j in range(i + 1, n):
            phase = rng.gauss(0.0, 0.08)
            mag = max(0.1, 1.0 + rng.gauss(0.0, 0.04))
            value = mag * complex(math.cos(phase), math.sin(phase))
            matrix[i][j] = value
            matrix[j][i] = value
    return matrix


def apply_asymmetry(matrix: list[list[complex]], rng: random.Random, level: float) -> None:
    if level <= 0.0:
        return
    n = len(matrix)
    for i in range(n):
        for j in range(i + 1, n):
            base = matrix[i][j]
            base_phase = math.atan2(base.imag, base.real)
            base_mag = abs(base)
            phase_delta = rng.uniform(-level, level)
            mag_delta = rng.uniform(-0.45 * level, 0.45 * level)
            phase = base_phase + phase_delta
            mag = max(0.1, base_mag * (1.0 + mag_delta))
            matrix[j][i] = mag * complex(math.cos(phase), math.sin(phase))


def pearson(xs: list[float], ys: list[float]) -> float:
    if len(xs) < 2 or len(xs) != len(ys):
        return 0.0
    mx = statistics.fmean(xs)
    my = statistics.fmean(ys)
    dx = [x - mx for x in xs]
    dy = [y - my for y in ys]
    numerator = sum(a * b for a, b in zip(dx, dy))
    denom = math.sqrt(sum(a * a for a in dx) * sum(b * b for b in dy))
    if denom <= 1e-12:
        return 0.0
    return numerator / denom


def emit_csv(rows: list[Row]) -> None:
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    with CSV_PATH.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "asymmetry_level",
                "case_id",
                "coherence_magnitude",
                "symmetry_gap_mean_abs",
                "observed_hafnian_theta",
                "predicted_uniform_theta",
                "uniform_phase_residual",
                "mean_edge_phase",
            ]
        )
        for row in rows:
            writer.writerow(
                [
                    f"{row.asymmetry_level:.6f}",
                    row.case_id,
                    f"{row.coherence_magnitude:.10f}",
                    f"{row.symmetry_gap_mean_abs:.10f}",
                    f"{row.observed_hafnian_theta:.10f}",
                    f"{row.predicted_uniform_theta:.10f}",
                    f"{row.uniform_phase_residual:.10f}",
                    f"{row.mean_edge_phase:.10f}",
                ]
            )


def write_plot(rows: list[Row]) -> None:
    width, height = 920, 620
    margin_left, margin_right = 82, 30
    margin_top, margin_bottom = 70, 70
    plot_w = width - margin_left - margin_right
    plot_h = height - margin_top - margin_bottom

    xs = [r.symmetry_gap_mean_abs for r in rows]
    ys = [abs(r.uniform_phase_residual) for r in rows]
    x_min, x_max = min(xs), max(xs)
    y_min, y_max = min(ys), max(ys)
    if abs(x_max - x_min) < 1e-12:
        x_max += 1.0
    if abs(y_max - y_min) < 1e-12:
        y_max += 1.0

    x_pad = (x_max - x_min) * 0.08
    y_pad = (y_max - y_min) * 0.12
    x_min -= x_pad
    x_max += x_pad
    y_min = max(0.0, y_min - y_pad)
    y_max += y_pad

    def sx(x: float) -> float:
        return margin_left + (x - x_min) / (x_max - x_min) * plot_w

    def sy(y: float) -> float:
        return margin_top + plot_h - (y - y_min) / (y_max - y_min) * plot_h

    lines = []
    lines.append(f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">')
    lines.append('<rect x="0" y="0" width="100%" height="100%" fill="#f9f4ea"/>')
    lines.append('<rect x="18" y="18" width="884" height="584" rx="18" fill="#fffaf2" stroke="#d8cfbf"/>')
    lines.append('<text x="460" y="46" text-anchor="middle" font-family="Georgia, serif" font-size="24" fill="#1b2b34">Asymmetry-Only Sweep: |Residual| vs Symmetry Gap</text>')

    for t in range(6):
        xv = x_min + (x_max - x_min) * t / 5.0
        yv = y_min + (y_max - y_min) * t / 5.0
        px = sx(xv)
        py = sy(yv)
        lines.append(f'<line x1="{px:.2f}" y1="{margin_top}" x2="{px:.2f}" y2="{margin_top + plot_h}" stroke="#efe7d8"/>')
        lines.append(f'<line x1="{margin_left}" y1="{py:.2f}" x2="{margin_left + plot_w}" y2="{py:.2f}" stroke="#efe7d8"/>')
        lines.append(f'<text x="{px:.2f}" y="{margin_top + plot_h + 22}" text-anchor="middle" font-family="Courier New, monospace" font-size="12" fill="#4a5b66">{xv:.3f}</text>')
        lines.append(f'<text x="{margin_left - 10}" y="{py + 4:.2f}" text-anchor="end" font-family="Courier New, monospace" font-size="12" fill="#4a5b66">{yv:.3f}</text>')

    lines.append(f'<line x1="{margin_left}" y1="{margin_top + plot_h}" x2="{margin_left + plot_w}" y2="{margin_top + plot_h}" stroke="#2e3f4a" stroke-width="1.5"/>')
    lines.append(f'<line x1="{margin_left}" y1="{margin_top}" x2="{margin_left}" y2="{margin_top + plot_h}" stroke="#2e3f4a" stroke-width="1.5"/>')

    for row in rows:
        # Warm palette by asymmetry level.
        level = row.asymmetry_level / max(ASYM_LEVELS)
        red = int(80 + 150 * level)
        green = int(120 - 70 * level)
        blue = int(110 - 80 * level)
        color = f"#{red:02x}{green:02x}{blue:02x}"
        lines.append(
            f'<circle cx="{sx(row.symmetry_gap_mean_abs):.2f}" cy="{sy(abs(row.uniform_phase_residual)):.2f}" r="4.6" fill="{color}" fill-opacity="0.8" stroke="#ffffff" stroke-width="0.8"/>'
        )

    lines.append(f'<text x="{margin_left + plot_w / 2:.2f}" y="{height - 18}" text-anchor="middle" font-family="Georgia, serif" font-size="16" fill="#1b2b34">symmetry_gap_mean_abs</text>')
    lines.append(f'<text x="24" y="{margin_top + plot_h / 2:.2f}" text-anchor="middle" transform="rotate(-90 24 {margin_top + plot_h / 2:.2f})" font-family="Georgia, serif" font-size="16" fill="#1b2b34">|uniform_phase_residual|</text>')

    lines.append('</svg>')
    PLOT_PATH.write_text("\n".join(lines), encoding="utf-8")


def build_summary(rows: list[Row]) -> dict:
    by_level: dict[float, list[Row]] = {}
    for row in rows:
        by_level.setdefault(row.asymmetry_level, []).append(row)

    residual_abs = [abs(r.uniform_phase_residual) for r in rows]
    symmetry_gap = [r.symmetry_gap_mean_abs for r in rows]
    coherence = [r.coherence_magnitude for r in rows]

    summary = {
        "seed": SEED,
        "dimension": DIMENSION,
        "cases_per_level": CASES_PER_LEVEL,
        "levels": ASYM_LEVELS,
        "total_cases": len(rows),
        "correlations": {
            "pearson_residual_abs_vs_symmetry_gap": pearson(residual_abs, symmetry_gap),
            "pearson_residual_abs_vs_coherence": pearson(residual_abs, coherence),
        },
        "levels_summary": {},
    }

    for level in ASYM_LEVELS:
        items = by_level.get(level, [])
        if not items:
            continue
        level_residual = [abs(r.uniform_phase_residual) for r in items]
        level_gap = [r.symmetry_gap_mean_abs for r in items]
        level_coh = [r.coherence_magnitude for r in items]
        summary["levels_summary"][f"{level:.2f}"] = {
            "count": len(items),
            "residual_abs_mean": statistics.fmean(level_residual),
            "residual_abs_max": max(level_residual),
            "symmetry_gap_mean": statistics.fmean(level_gap),
            "coherence_mean": statistics.fmean(level_coh),
        }

    SUMMARY_PATH.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    return summary


def run() -> None:
    rng = random.Random(SEED)
    rows: list[Row] = []
    failures: list[str] = []

    for level in ASYM_LEVELS:
        for idx in range(CASES_PER_LEVEL):
            matrix = build_high_coherence_symmetric_matrix(DIMENSION, rng)
            apply_asymmetry(matrix, rng, level)
            expression = matrix_to_expression(matrix)
            case_id = f"asym_{level:.2f}_{idx + 1:02d}"
            try:
                response = post_expression(expression)
                probe = flux_probe_from_response(response)
            except Exception as exc:  # noqa: BLE001
                failures.append(f"{case_id}: {exc}")
                continue

            rows.append(
                Row(
                    asymmetry_level=level,
                    case_id=case_id,
                    coherence_magnitude=float(probe["coherence_magnitude"]),
                    symmetry_gap_mean_abs=float(probe["symmetry_gap_mean_abs"]),
                    observed_hafnian_theta=wrap_pi(float(probe["observed_hafnian_theta"])),
                    predicted_uniform_theta=wrap_pi(float(probe["predicted_uniform_theta"])),
                    uniform_phase_residual=wrap_pi(float(probe["uniform_phase_residual"])),
                    mean_edge_phase=wrap_pi(float(probe["mean_edge_phase"])),
                )
            )

    emit_csv(rows)
    write_plot(rows)
    summary = build_summary(rows)

    print("Asymmetry-only sweep complete")
    print(f"Rows: {len(rows)}")
    print(f"Failures: {len(failures)}")
    print(f"CSV: {CSV_PATH}")
    print(f"Plot: {PLOT_PATH}")
    print(f"Summary: {SUMMARY_PATH}")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    run()
