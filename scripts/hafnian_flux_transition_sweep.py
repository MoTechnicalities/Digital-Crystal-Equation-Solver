#!/bin/python3
"""Phase-transition sweep for hafnian flux probe.

Generates a grid across coherence-driving phase spread and asymmetry perturbation,
across multiple dimensions, then emits:
- CSV dataset
- heatmap-style SVG (mean |residual| by coherence/symmetry bins)
- summary JSON
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
SEED = 20260530
DIMENSIONS = [4, 6, 8, 10]
PHASE_SIGMAS = [0.05, 0.15, 0.3, 0.6, 1.1]
ASYMMETRY_LEVELS = [0.0, 0.15, 0.35, 0.55]
SAMPLES_PER_CELL = 3

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT_DIR = ROOT / "docs" / "findings" / "artifacts"
CSV_PATH = ARTIFACT_DIR / "hafnian_flux_transition_sweep.csv"
HEATMAP_PATH = ARTIFACT_DIR / "hafnian_flux_transition_heatmap.svg"
SUMMARY_PATH = ARTIFACT_DIR / "hafnian_flux_transition_summary.json"

COHERENCE_BIN_COUNT = 8
SYMMETRY_BIN_COUNT = 8


@dataclass
class Row:
    dimension: int
    phase_sigma: float
    asymmetry_level: float
    case_id: str
    coherence_magnitude: float
    symmetry_gap_mean_abs: float
    residual_abs: float


def format_num(value: float) -> str:
    if abs(value) < 1e-12:
        value = 0.0
    return f"{value:.6f}"


def complex_literal(z: complex) -> str:
    re_s = format_num(z.real)
    im_s = format_num(z.imag)
    if im_s.startswith("-"):
        return f"({re_s}{im_s}i)"
    return f"({re_s}+{im_s}i)"


def matrix_to_expression(matrix: list[list[complex]]) -> str:
    rows = []
    for row in matrix:
        rows.append("[" + ",".join(complex_literal(v) for v in row) + "]")
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
        with urllib.request.urlopen(request, timeout=60) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {exc.code}: {detail}") from exc


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


def build_symmetric_matrix(n: int, rng: random.Random, phase_sigma: float, mag_jitter: float = 0.06) -> list[list[complex]]:
    matrix = [[0j for _ in range(n)] for _ in range(n)]
    for i in range(n):
        for j in range(i + 1, n):
            phase = rng.gauss(0.0, phase_sigma)
            mag = max(0.1, 1.0 + rng.gauss(0.0, mag_jitter))
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
            phase = base_phase + rng.uniform(-level, level)
            mag = max(0.1, base_mag * (1.0 + rng.uniform(-0.45 * level, 0.45 * level)))
            matrix[j][i] = mag * complex(math.cos(phase), math.sin(phase))


def emit_csv(rows: list[Row]) -> None:
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    with CSV_PATH.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "dimension",
                "phase_sigma",
                "asymmetry_level",
                "case_id",
                "coherence_magnitude",
                "symmetry_gap_mean_abs",
                "residual_abs",
            ]
        )
        for row in rows:
            writer.writerow(
                [
                    row.dimension,
                    f"{row.phase_sigma:.6f}",
                    f"{row.asymmetry_level:.6f}",
                    row.case_id,
                    f"{row.coherence_magnitude:.10f}",
                    f"{row.symmetry_gap_mean_abs:.10f}",
                    f"{row.residual_abs:.10f}",
                ]
            )


def build_heatmap(rows: list[Row]) -> dict:
    coh_values = [r.coherence_magnitude for r in rows]
    gap_values = [r.symmetry_gap_mean_abs for r in rows]
    if not rows:
        raise RuntimeError("No rows to plot")

    coh_min, coh_max = min(coh_values), max(coh_values)
    gap_min, gap_max = min(gap_values), max(gap_values)
    if abs(coh_max - coh_min) < 1e-12:
        coh_max += 1.0
    if abs(gap_max - gap_min) < 1e-12:
        gap_max += 1.0

    grid = [[[] for _ in range(SYMMETRY_BIN_COUNT)] for _ in range(COHERENCE_BIN_COUNT)]

    def coh_bin(c: float) -> int:
        idx = int((c - coh_min) / (coh_max - coh_min) * COHERENCE_BIN_COUNT)
        return max(0, min(COHERENCE_BIN_COUNT - 1, idx if idx < COHERENCE_BIN_COUNT else COHERENCE_BIN_COUNT - 1))

    def gap_bin(g: float) -> int:
        idx = int((g - gap_min) / (gap_max - gap_min) * SYMMETRY_BIN_COUNT)
        return max(0, min(SYMMETRY_BIN_COUNT - 1, idx if idx < SYMMETRY_BIN_COUNT else SYMMETRY_BIN_COUNT - 1))

    for row in rows:
        i = coh_bin(row.coherence_magnitude)
        j = gap_bin(row.symmetry_gap_mean_abs)
        grid[i][j].append(row.residual_abs)

    cell_mean = [[None for _ in range(SYMMETRY_BIN_COUNT)] for _ in range(COHERENCE_BIN_COUNT)]
    values = []
    for i in range(COHERENCE_BIN_COUNT):
        for j in range(SYMMETRY_BIN_COUNT):
            if grid[i][j]:
                mean_val = statistics.fmean(grid[i][j])
                cell_mean[i][j] = mean_val
                values.append(mean_val)

    vmin = min(values) if values else 0.0
    vmax = max(values) if values else 1.0
    if abs(vmax - vmin) < 1e-12:
        vmax = vmin + 1.0

    width, height = 980, 700
    margin_left, margin_right, margin_top, margin_bottom = 120, 90, 90, 110
    plot_w = width - margin_left - margin_right
    plot_h = height - margin_top - margin_bottom
    cell_w = plot_w / SYMMETRY_BIN_COUNT
    cell_h = plot_h / COHERENCE_BIN_COUNT

    def color_for(value: float | None) -> str:
        if value is None:
            return "#eee8dd"
        t = (value - vmin) / (vmax - vmin)
        # Warm scientific palette: low residual teal -> high residual deep rust.
        r = int(28 + t * 190)
        g = int(129 - t * 75)
        b = int(132 - t * 96)
        return f"#{r:02x}{g:02x}{b:02x}"

    lines = []
    lines.append(f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">')
    lines.append('<rect x="0" y="0" width="100%" height="100%" fill="#f7f2e7"/>')
    lines.append(f'<text x="{width/2:.1f}" y="46" text-anchor="middle" font-family="Georgia, serif" font-size="28" fill="#1f2e36">Hafnian Flux Phase-Transition Atlas</text>')
    lines.append(f'<text x="{width/2:.1f}" y="74" text-anchor="middle" font-family="Courier New, monospace" font-size="13" fill="#4f5f69">Cell color = mean |uniform_phase_residual|</text>')

    # Draw cells with y-axis high coherence at top.
    for i in range(COHERENCE_BIN_COUNT):
        for j in range(SYMMETRY_BIN_COUNT):
            x = margin_left + j * cell_w
            y = margin_top + i * cell_h
            lines.append(
                f'<rect x="{x:.2f}" y="{y:.2f}" width="{cell_w:.2f}" height="{cell_h:.2f}" fill="{color_for(cell_mean[COHERENCE_BIN_COUNT - 1 - i][j])}" stroke="#ffffff" stroke-width="1"/>'
            )

    lines.append(f'<rect x="{margin_left}" y="{margin_top}" width="{plot_w}" height="{plot_h}" fill="none" stroke="#24343f" stroke-width="1.5"/>')

    for k in range(SYMMETRY_BIN_COUNT + 1):
        x = margin_left + k * cell_w
        lines.append(f'<line x1="{x:.2f}" y1="{margin_top + plot_h}" x2="{x:.2f}" y2="{margin_top + plot_h + 8}" stroke="#24343f"/>')
    for k in range(COHERENCE_BIN_COUNT + 1):
        y = margin_top + k * cell_h
        lines.append(f'<line x1="{margin_left - 8}" y1="{y:.2f}" x2="{margin_left}" y2="{y:.2f}" stroke="#24343f"/>')

    # Axis labels
    lines.append(f'<text x="{margin_left + plot_w/2:.2f}" y="{height - 40}" text-anchor="middle" font-family="Georgia, serif" font-size="18" fill="#1f2e36">symmetry_gap_mean_abs</text>')
    lines.append(f'<text x="34" y="{margin_top + plot_h/2:.2f}" text-anchor="middle" transform="rotate(-90 34 {margin_top + plot_h/2:.2f})" font-family="Georgia, serif" font-size="18" fill="#1f2e36">coherence_magnitude</text>')

    # Tick labels (min-mid-max)
    lines.append(f'<text x="{margin_left:.2f}" y="{height - 68}" text-anchor="middle" font-family="Courier New, monospace" font-size="12" fill="#4f5f69">{gap_min:.3f}</text>')
    lines.append(f'<text x="{margin_left + plot_w/2:.2f}" y="{height - 68}" text-anchor="middle" font-family="Courier New, monospace" font-size="12" fill="#4f5f69">{(gap_min + gap_max)/2:.3f}</text>')
    lines.append(f'<text x="{margin_left + plot_w:.2f}" y="{height - 68}" text-anchor="middle" font-family="Courier New, monospace" font-size="12" fill="#4f5f69">{gap_max:.3f}</text>')

    lines.append(f'<text x="{margin_left - 20:.2f}" y="{margin_top + plot_h + 4:.2f}" text-anchor="end" font-family="Courier New, monospace" font-size="12" fill="#4f5f69">{coh_min:.3f}</text>')
    lines.append(f'<text x="{margin_left - 20:.2f}" y="{margin_top + plot_h/2 + 4:.2f}" text-anchor="end" font-family="Courier New, monospace" font-size="12" fill="#4f5f69">{(coh_min + coh_max)/2:.3f}</text>')
    lines.append(f'<text x="{margin_left - 20:.2f}" y="{margin_top + 4:.2f}" text-anchor="end" font-family="Courier New, monospace" font-size="12" fill="#4f5f69">{coh_max:.3f}</text>')

    # Color legend
    legend_x = margin_left + plot_w + 24
    legend_y = margin_top
    legend_h = plot_h
    legend_steps = 80
    for s in range(legend_steps):
        t = s / (legend_steps - 1)
        value = vmin + (1 - t) * (vmax - vmin)
        y = legend_y + t * legend_h
        lines.append(f'<rect x="{legend_x}" y="{y:.2f}" width="22" height="{legend_h/legend_steps + 1:.2f}" fill="{color_for(value)}" stroke="none"/>')
    lines.append(f'<rect x="{legend_x}" y="{legend_y}" width="22" height="{legend_h}" fill="none" stroke="#24343f"/>')
    lines.append(f'<text x="{legend_x + 28}" y="{legend_y + 12}" font-family="Courier New, monospace" font-size="12" fill="#4f5f69">{vmax:.3f}</text>')
    lines.append(f'<text x="{legend_x + 28}" y="{legend_y + legend_h/2 + 4}" font-family="Courier New, monospace" font-size="12" fill="#4f5f69">{(vmin + vmax)/2:.3f}</text>')
    lines.append(f'<text x="{legend_x + 28}" y="{legend_y + legend_h - 2}" font-family="Courier New, monospace" font-size="12" fill="#4f5f69">{vmin:.3f}</text>')

    lines.append("</svg>")
    HEATMAP_PATH.write_text("\n".join(lines), encoding="utf-8")

    return {
        "coherence_range": [coh_min, coh_max],
        "symmetry_gap_range": [gap_min, gap_max],
        "residual_mean_range": [vmin, vmax],
    }


def summarize(rows: list[Row], failures: list[str], heatmap_meta: dict) -> dict:
    by_dim: dict[int, list[Row]] = {}
    for row in rows:
        by_dim.setdefault(row.dimension, []).append(row)

    summary = {
        "seed": SEED,
        "dimensions": DIMENSIONS,
        "phase_sigmas": PHASE_SIGMAS,
        "asymmetry_levels": ASYMMETRY_LEVELS,
        "samples_per_cell": SAMPLES_PER_CELL,
        "total_cases": len(rows),
        "failure_count": len(failures),
        "heatmap": heatmap_meta,
        "dimension_summary": {},
    }

    for dim, items in by_dim.items():
        residual = [r.residual_abs for r in items]
        coherence = [r.coherence_magnitude for r in items]
        gaps = [r.symmetry_gap_mean_abs for r in items]
        summary["dimension_summary"][str(dim)] = {
            "count": len(items),
            "residual_abs_mean": statistics.fmean(residual),
            "residual_abs_max": max(residual),
            "coherence_mean": statistics.fmean(coherence),
            "coherence_min": min(coherence),
            "coherence_max": max(coherence),
            "symmetry_gap_mean": statistics.fmean(gaps),
            "symmetry_gap_max": max(gaps),
        }

    SUMMARY_PATH.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    return summary


def run() -> None:
    rng = random.Random(SEED)
    rows: list[Row] = []
    failures: list[str] = []

    for dim in DIMENSIONS:
        for sigma in PHASE_SIGMAS:
            for asym in ASYMMETRY_LEVELS:
                for sample_idx in range(SAMPLES_PER_CELL):
                    case_id = f"d{dim}_s{sigma:.2f}_a{asym:.2f}_k{sample_idx + 1}"
                    matrix = build_symmetric_matrix(dim, rng, sigma)
                    apply_asymmetry(matrix, rng, asym)
                    expression = matrix_to_expression(matrix)
                    try:
                        response = post_expression(expression)
                        probe = flux_probe_from_response(response)
                        residual_abs = abs(float(probe["uniform_phase_residual"]))
                        rows.append(
                            Row(
                                dimension=dim,
                                phase_sigma=sigma,
                                asymmetry_level=asym,
                                case_id=case_id,
                                coherence_magnitude=float(probe["coherence_magnitude"]),
                                symmetry_gap_mean_abs=float(probe["symmetry_gap_mean_abs"]),
                                residual_abs=residual_abs,
                            )
                        )
                    except Exception as exc:  # noqa: BLE001
                        failures.append(f"{case_id}: {exc}")

    emit_csv(rows)
    heatmap_meta = build_heatmap(rows)
    summary = summarize(rows, failures, heatmap_meta)

    print("Transition sweep complete")
    print(f"Rows: {len(rows)}")
    print(f"Failures: {len(failures)}")
    print(f"CSV: {CSV_PATH}")
    print(f"Heatmap: {HEATMAP_PATH}")
    print(f"Summary: {SUMMARY_PATH}")
    print(json.dumps(summary, indent=2))
    if failures:
        print("Failure samples:")
        for item in failures[:10]:
            print(f"- {item}")


if __name__ == "__main__":
    run()
