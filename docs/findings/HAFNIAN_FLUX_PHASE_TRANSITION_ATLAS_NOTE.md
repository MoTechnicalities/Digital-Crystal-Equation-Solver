# Hafnian Flux Phase-Transition Atlas Note (v0.1)

Status: Experimental transition-mapping report.

## Objective

Map stable and unstable prediction regimes for the uniform-phase residual by jointly varying:
- coherence-driving phase spread
- symmetry perturbation level
- matrix dimension

and derive coherence-cliff threshold estimates with confidence bands.

## New Protocol Scripts

- `scripts/hafnian_flux_transition_sweep.py`
  - Generates transition sweep dataset and heatmap-style SVG atlas.
- `scripts/hafnian_flux_transition_inference.py`
  - Computes per-dimension coherence threshold estimates and bootstrap 95% CIs.

## Run Configuration

Sweep parameters:
- dimensions: `4, 6, 8, 10`
- phase sigma levels: `0.05, 0.15, 0.30, 0.60, 1.10`
- asymmetry levels: `0.00, 0.15, 0.35, 0.55`
- samples per cell: `3`
- total cases: `240`
- failures: `0`

Threshold inference parameters:
- residual threshold for cliff: `|residual| = 0.10`
- bootstrap rounds: `2000`
- bootstrap seed: `20260530`

## Artifacts

- Sweep dataset: [docs/findings/artifacts/hafnian_flux_transition_sweep.csv](artifacts/hafnian_flux_transition_sweep.csv)
- Heatmap atlas: [docs/findings/artifacts/hafnian_flux_transition_heatmap.svg](artifacts/hafnian_flux_transition_heatmap.svg)
- Sweep summary: [docs/findings/artifacts/hafnian_flux_transition_summary.json](artifacts/hafnian_flux_transition_summary.json)
- Thresholds + CIs: [docs/findings/artifacts/hafnian_flux_transition_thresholds.json](artifacts/hafnian_flux_transition_thresholds.json)

## Threshold Estimates With Confidence Bands

Estimated coherence threshold where mean residual crosses 0.10 as coherence decreases:

| dimension | threshold estimate | 95% CI |
|---|---:|---:|
| 4  | 0.8097 | [0.7689, 0.8672] |
| 6  | 0.8252 | [0.7832, 0.8437] |
| 8  | 0.7898 | [0.7017, 0.8839] |
| 10 | 0.7383 | [0.6294, 0.7813] |

## Readout

- The atlas shows a clear transition band from low residual to high residual as coherence decreases.
- Across tested dimensions, the cliff region appears in the neighborhood of coherence ~0.74-0.83, with uncertainty reflected by the bootstrap CIs.
- Symmetry perturbation contributes structure in the heatmap, but coherence remains the dominant separator for this threshold criterion.

## Reproduction

```bash
/bin/python3 scripts/hafnian_flux_transition_sweep.py
/bin/python3 scripts/hafnian_flux_transition_inference.py
```

## Scope and caution

These thresholds are empirical and protocol-specific. They are not theorem-level bounds and should be re-estimated for new dimension ranges, perturbation distributions, and seeds.
