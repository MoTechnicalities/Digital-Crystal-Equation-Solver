#!/bin/python3
"""Run a reproducible hafnian flux-probe sweep and emit CSV + SVG artifacts.

This script queries the local DigitalCrystal API endpoint at /v1/csif/math using
controlled matrix families and records flux probe metrics from derivation trace
numeric trust metadata.
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
from typing import Iterable

API_URL = "http://127.0.0.1:8080/v1/csif/math"
SEED = 20260529
DIMENSION = 6
CASES_PER_FAMILY = 18

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT_DIR = ROOT / "docs" / "findings" / "artifacts"
CSV_PATH = ARTIFACT_DIR / "hafnian_flux_sweep.csv"
PLOT1_PATH = ARTIFACT_DIR / "hafnian_flux_residual_vs_coherence.svg"
PLOT2_PATH = ARTIFACT_DIR / "hafnian_flux_residual_vs_symmetry_gap.svg"
SUMMARY_PATH = ARTIFACT_DIR / "hafnian_flux_sweep_summary.json"


@dataclass
class Row:
    family: str
    case_id: str
    dimension: int
    off_diagonal_pairs: int
    coherence_magnitude: float
    symmetry_gap_mean_abs: float
    observed_hafnian_theta: float
    predicted_uniform_theta: float
    uniform_phase_residual: float
    mean_edge_phase: float
    mean_edge_magnitude: float
    magnitude_coefficient_of_variation: float


def wrap_pi(theta: float) -> float:
    return (theta + math.pi) % (2 * math.pi) - math.pi


def format_number(value: float) -> str:
    if abs(value) < 1e-12:
        value = 0.0
    return f"{value:.6f}"


def complex_literal(re: float, im: float) -> str:
    re_s = format_number(re)
    im_s = format_number(im)
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
    if not trace:
        raise RuntimeError("No derivation_trace returned")
    for step in trace:
        trust = (step or {}).get("numeric_trust")
        if not trust:
            continue
        probe = trust.get("hafnian_flux_probe")
        if probe:
            return probe
    raise RuntimeError("No hafnian_flux_probe returned in derivation_trace")


def build_symmetric_matrix(n: int, rng: random.Random, phase_sigma: float, mag_jitter: float) -> list[list[complex]]:
    matrix = [[0j for _ in range(n)] for _ in range(n)]
    for i in range(n):
        for j in range(i + 1, n):
            phase = rng.gauss(0.0, phase_sigma)
            mag = max(0.1, 1.0 + rng.gauss(0.0, mag_jitter))
            value = mag * complex(math.cos(phase), math.sin(phase))
            matrix[i][j] = value
            matrix[j][i] = value
    return matrix


def build_low_coherence_matrix(n: int, rng: random.Random) -> list[list[complex]]:
    matrix = [[0j for _ in range(n)] for _ in range(n)]
    for i in range(n):
        for j in range(i + 1, n):
            phase = rng.uniform(-math.pi, math.pi)
            mag = rng.uniform(0.4, 1.6)
            value = mag * complex(math.cos(phase), math.sin(phase))
            matrix[i][j] = value
            matrix[j][i] = value
    return matrix


def perturb_asymmetry(matrix: list[list[complex]], rng: random.Random, phase_push: float, mag_push: float) -> None:
    n = len(matrix)
    for i in range(n):
        for j in range(i + 1, n):
            base = matrix[i][j]
            phase = math.atan2(base.imag, base.real)
            mag = abs(base)
            phase += rng.uniform(-phase_push, phase_push)
            mag = max(0.1, mag + rng.uniform(-mag_push, mag_push))
            matrix[j][i] = mag * complex(math.cos(phase), math.sin(phase))


def emit_csv(rows: Iterable[Row]) -> None:
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    with CSV_PATH.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "family",
                "case_id",
                "dimension",
                "off_diagonal_pairs",
                "coherence_magnitude",
                "symmetry_gap_mean_abs",
                "observed_hafnian_theta",
                "predicted_uniform_theta",
                "uniform_phase_residual",
                "mean_edge_phase",
                "mean_edge_magnitude",
                "magnitude_coefficient_of_variation",
            ]
        )
        for row in rows:
            writer.writerow(
                [
                    row.family,
                    row.case_id,
                    row.dimension,
                    row.off_diagonal_pairs,
                    f"{row.coherence_magnitude:.10f}",
                    f"{row.symmetry_gap_mean_abs:.10f}",
                    f"{row.observed_hafnian_theta:.10f}",
                    f"{row.predicted_uniform_theta:.10f}",
                    f"{row.uniform_phase_residual:.10f}",
                    f"{row.mean_edge_phase:.10f}",
                    f"{row.mean_edge_magnitude:.10f}",
                    f"{row.magnitude_coefficient_of_variation:.10f}",
                ]
            )


def build_summary(rows: list[Row]) -> dict:
    by_family: dict[str, list[Row]] = {}
    for row in rows:
        by_family.setdefault(row.family, []).append(row)

    summary = {
        "seed": SEED,
        "dimension": DIMENSION,
        "cases_per_family": CASES_PER_FAMILY,
        "total_cases": len(rows),
        "families": {},
    }

    for family, family_rows in by_family.items():
        residual_abs = [abs(r.uniform_phase_residual) for r in family_rows]
        coherence = [r.coherence_magnitude for r in family_rows]
        symmetry_gap = [r.symmetry_gap_mean_abs for r in family_rows]
        summary["families"][family] = {
            "count": len(family_rows),
            "coherence_mean": statistics.fmean(coherence),
            "coherence_min": min(coherence),
            "coherence_max": max(coherence),
            "symmetry_gap_mean": statistics.fmean(symmetry_gap),
            "residual_abs_mean": statistics.fmean(residual_abs),
            "residual_abs_max": max(residual_abs),
        }

    with SUMMARY_PATH.open("w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2)

    return summary


def svg_scatter(path: Path, rows: list[Row], x_key: str, y_key: str, title: str, x_label: str, y_label: str) -> None:
    palette = {
        "coherent": "#0b7d6c",
        "partial_coherence": "#1f78b4",
        "low_coherence": "#cc6a00",
        "symmetry_perturbed": "#b22222",
    }

    width, height = 920, 620
    margin_left, margin_right = 82, 30
    margin_top, margin_bottom = 70, 70
    plot_w = width - margin_left - margin_right
    plot_h = height - margin_top - margin_bottom

    xs = [getattr(r, x_key) for r in rows]
    ys = [abs(getattr(r, y_key)) for r in rows]
    x_min, x_max = min(xs), max(xs)
    y_min, y_max = min(ys), max(ys)

    if abs(x_max - x_min) < 1e-12:
        x_min -= 1.0
        x_max += 1.0
    if abs(y_max - y_min) < 1e-12:
        y_min -= 1.0
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
    lines.append('<rect x="0" y="0" width="100%" height="100%" fill="#f7f3ea"/>')
    lines.append('<rect x="18" y="18" width="884" height="584" rx="18" fill="#fffaf1" stroke="#d8cfbf"/>')
    lines.append(f'<text x="{width/2:.1f}" y="46" text-anchor="middle" font-family="Georgia, serif" font-size="24" fill="#1b2b34">{title}</text>')

    # Grid + ticks
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
        x = getattr(row, x_key)
        y = abs(getattr(row, y_key))
        lines.append(
            f'<circle cx="{sx(x):.2f}" cy="{sy(y):.2f}" r="4.5" fill="{palette[row.family]}" fill-opacity="0.78" stroke="#ffffff" stroke-width="0.8"/>'
        )

    lines.append(f'<text x="{margin_left + plot_w / 2:.2f}" y="{height - 18}" text-anchor="middle" font-family="Georgia, serif" font-size="16" fill="#1b2b34">{x_label}</text>')
    lines.append(
        f'<text x="24" y="{margin_top + plot_h / 2:.2f}" text-anchor="middle" transform="rotate(-90 24 {margin_top + plot_h / 2:.2f})" font-family="Georgia, serif" font-size="16" fill="#1b2b34">{y_label}</text>'
    )

    legend_x, legend_y = width - 250, 92
    lines.append(f'<rect x="{legend_x}" y="{legend_y}" width="210" height="128" rx="10" fill="#fff" stroke="#d8cfbf"/>')
    y_offset = legend_y + 24
    for name in ["coherent", "partial_coherence", "low_coherence", "symmetry_perturbed"]:
        lines.append(f'<circle cx="{legend_x + 18}" cy="{y_offset}" r="5" fill="{palette[name]}"/>')
        lines.append(f'<text x="{legend_x + 32}" y="{y_offset + 4}" font-family="Courier New, monospace" font-size="12" fill="#24343f">{name}</text>')
        y_offset += 26

    lines.append("</svg>")
    path.write_text("\n".join(lines), encoding="utf-8")


def run() -> None:
    rng = random.Random(SEED)
    rows: list[Row] = []
    failures: list[str] = []

    families = [
        "coherent",
        "partial_coherence",
        "low_coherence",
        "symmetry_perturbed",
    ]

    for family in families:
        for idx in range(CASES_PER_FAMILY):
            case_id = f"{family}-{idx + 1:02d}"
            if family == "coherent":
                matrix = build_symmetric_matrix(DIMENSION, rng, phase_sigma=0.06, mag_jitter=0.04)
            elif family == "partial_coherence":
                matrix = build_symmetric_matrix(DIMENSION, rng, phase_sigma=0.35, mag_jitter=0.15)
            elif family == "low_coherence":
                matrix = build_low_coherence_matrix(DIMENSION, rng)
            else:
                matrix = build_symmetric_matrix(DIMENSION, rng, phase_sigma=0.18, mag_jitter=0.08)
                perturb_asymmetry(matrix, rng, phase_push=0.55, mag_push=0.25)

            expression = matrix_to_expression(matrix)
            try:
                response = post_expression(expression)
                probe = flux_probe_from_response(response)
            except Exception as exc:  # noqa: BLE001
                failures.append(f"{case_id}: {exc}")
                continue

            rows.append(
                Row(
                    family=family,
                    case_id=case_id,
                    dimension=int(probe["dimension"]),
                    off_diagonal_pairs=int(probe["off_diagonal_pairs"]),
                    coherence_magnitude=float(probe["coherence_magnitude"]),
                    symmetry_gap_mean_abs=float(probe["symmetry_gap_mean_abs"]),
                    observed_hafnian_theta=wrap_pi(float(probe["observed_hafnian_theta"])),
                    predicted_uniform_theta=wrap_pi(float(probe["predicted_uniform_theta"])),
                    uniform_phase_residual=wrap_pi(float(probe["uniform_phase_residual"])),
                    mean_edge_phase=wrap_pi(float(probe["mean_edge_phase"])),
                    mean_edge_magnitude=float(probe["mean_edge_magnitude"]),
                    magnitude_coefficient_of_variation=float(probe["magnitude_coefficient_of_variation"]),
                )
            )

    emit_csv(rows)
    summary = build_summary(rows)
    svg_scatter(
        PLOT1_PATH,
        rows,
        x_key="coherence_magnitude",
        y_key="uniform_phase_residual",
        title="Hafnian Flux Probe Sweep: |Residual| vs Coherence",
        x_label="coherence_magnitude",
        y_label="|uniform_phase_residual|",
    )
    svg_scatter(
        PLOT2_PATH,
        rows,
        x_key="symmetry_gap_mean_abs",
        y_key="uniform_phase_residual",
        title="Hafnian Flux Probe Sweep: |Residual| vs Symmetry Gap",
        x_label="symmetry_gap_mean_abs",
        y_label="|uniform_phase_residual|",
    )

    print("Sweep complete")
    print(f"Rows: {len(rows)}")
    print(f"Failures: {len(failures)}")
    print(f"CSV: {CSV_PATH}")
    print(f"Plot 1: {PLOT1_PATH}")
    print(f"Plot 2: {PLOT2_PATH}")
    print(f"Summary: {SUMMARY_PATH}")
    print(json.dumps(summary, indent=2))
    if failures:
        print("Failure samples:")
        for item in failures[:10]:
            print(f"- {item}")


if __name__ == "__main__":
    run()
