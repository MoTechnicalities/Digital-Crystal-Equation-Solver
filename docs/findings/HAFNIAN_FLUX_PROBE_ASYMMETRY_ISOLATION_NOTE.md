# Hafnian Flux Probe Asymmetry Isolation Note (v0.1)

Status: Experimental causality-isolation protocol.

## Goal

Test whether symmetry-gap increase alone drives residual growth when phase coherence is intentionally held near-constant and high.

## Separate Protocol

This protocol is intentionally separate from the family sweep.

- Script: `scripts/hafnian_flux_asymmetry_sweep.py`
- Endpoint: `POST /v1/csif/math`
- Mode: `algebraic`
- Angle unit: `radians`
- Seed: `20260529`
- Matrix dimension: `n = 6`
- Cases per asymmetry level: `16`
- Levels: `0.00, 0.10, 0.20, 0.35, 0.50, 0.70`
- Total cases: `96`

Construction strategy:
- Start from high-coherence symmetric matrices.
- Apply asymmetry only by perturbing lower-triangle phase and magnitude according to level.
- Keep upper-triangle generation fixed to preserve baseline coherence behavior.

## Artifacts

- Dataset: [docs/findings/artifacts/hafnian_flux_asymmetry_sweep.csv](artifacts/hafnian_flux_asymmetry_sweep.csv)
- Summary: [docs/findings/artifacts/hafnian_flux_asymmetry_sweep_summary.json](artifacts/hafnian_flux_asymmetry_sweep_summary.json)
- Plot: [docs/findings/artifacts/hafnian_flux_asymmetry_residual_vs_gap.svg](artifacts/hafnian_flux_asymmetry_residual_vs_gap.svg)

## Results

Correlation outputs:
- `pearson_residual_abs_vs_symmetry_gap = -0.0656`
- `pearson_residual_abs_vs_coherence = -0.3089`

Per-level means:

| level | symmetry_gap_mean | coherence_mean | mean |residual| |
|---|---:|---:|---:|
| 0.00 | 0.0000 | 0.9970 | 0.00186 |
| 0.10 | 0.0556 | 0.9972 | 0.00240 |
| 0.20 | 0.1143 | 0.9971 | 0.00236 |
| 0.35 | 0.1994 | 0.9971 | 0.00232 |
| 0.50 | 0.2925 | 0.9975 | 0.00164 |
| 0.70 | 0.4161 | 0.9971 | 0.00186 |

## Causality-Isolation Readout

In this specific isolation design:
- Symmetry gap increases strongly across levels.
- Coherence remains tightly near 0.997.
- Residual stays small and fairly flat.

Interpretation:
- This dataset does not show a strong monotonic residual increase from symmetry-gap alone under preserved high coherence.
- The larger residual regime observed previously appears more coherence-sensitive than symmetry-gap-sensitive in this operating region.

## Reproduction

```bash
/bin/python3 scripts/hafnian_flux_asymmetry_sweep.py
```

## Scope and caution

This is still experimental evidence, not proof. It isolates one perturbation style and one dimension regime; broader causality claims require additional protocols and seeds.
