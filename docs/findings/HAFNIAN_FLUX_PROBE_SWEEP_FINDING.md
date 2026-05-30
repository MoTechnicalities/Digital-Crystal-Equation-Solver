# Hafnian Flux Probe Sweep: Coherence and Symmetry Trends (v0.1)

Status: Experimental, reproducible sweep report.

## Purpose

This report extends the first single-case finding with a controlled sweep over matrix families to test whether phase residual tracks coherence loss and symmetry perturbation.

## Protocol

- Engine endpoint: `POST /v1/csif/math`
- Mode: `algebraic`
- Angle unit: `radians`
- Probe source: `derivation_trace[*].numeric_trust.hafnian_flux_probe`
- Seed: `20260529`
- Dimension: `n = 6`
- Cases per family: `18`
- Total cases: `72`

Families:
- `coherent`: symmetric, tight phase spread, low magnitude jitter
- `partial_coherence`: symmetric, moderate phase spread and jitter
- `low_coherence`: symmetric, broad random phase and magnitude spread
- `symmetry_perturbed`: starts symmetric then lower-triangle is phase/magnitude perturbed

## Key Metrics

For each case, we capture:
- `coherence_magnitude`
- `symmetry_gap_mean_abs`
- `observed_hafnian_theta`
- `predicted_uniform_theta`
- `uniform_phase_residual`

Residual analysis uses $|\Delta\theta| = |\text{uniform_phase_residual}|$.

## Summary Results

| family | coherence mean | symmetry-gap mean | mean |residual| | max |residual| |
|---|---:|---:|---:|---:|
| coherent | 0.9983 | 0.0000 | 0.0013 | 0.0051 |
| partial_coherence | 0.9474 | 0.0000 | 0.0401 | 0.0964 |
| low_coherence | 0.2354 | 0.0000 | 1.6229 | 3.0964 |
| symmetry_perturbed | 0.9855 | 0.3168 | 0.0111 | 0.0262 |

## Interpretation

Observed trend in this sweep:
- As coherence drops from near-1 toward low values, residual grows sharply.
- With high coherence but induced asymmetry, residual rises above coherent baseline, though still far below low-coherence cases in this dataset.

This supports the experimental hypothesis that both phase alignment and symmetry quality influence hafnian phase predictability under the probe model.

## Artifacts

- CSV dataset: [docs/findings/artifacts/hafnian_flux_sweep.csv](artifacts/hafnian_flux_sweep.csv)
- Summary JSON: [docs/findings/artifacts/hafnian_flux_sweep_summary.json](artifacts/hafnian_flux_sweep_summary.json)
- Plot 1: [docs/findings/artifacts/hafnian_flux_residual_vs_coherence.svg](artifacts/hafnian_flux_residual_vs_coherence.svg)
- Plot 2: [docs/findings/artifacts/hafnian_flux_residual_vs_symmetry_gap.svg](artifacts/hafnian_flux_residual_vs_symmetry_gap.svg)

## Reproduction

```bash
/bin/python3 scripts/hafnian_flux_sweep.py
```

## Scope and Caution

These are deterministic measurements from this implementation and seed, not a formal proof. They are intended to guide further experiments and possible theorem-building work.

## Next Expansions

- Dimension sweep near current exact hafnian cap.
- Alternative phase-mixture distributions (clustered multimodal, heavy-tail).
- Confidence intervals across multiple seeds.
- Residual decomposition against magnitude CV versus symmetry gap.
