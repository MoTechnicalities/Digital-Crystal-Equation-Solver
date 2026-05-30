# Hafnian Flux Probe Inference Note (v0.1)

Status: Experimental statistical follow-up.

## What was added

This note adds an inference layer on top of the 72-case sweep dataset by computing:
- Pearson correlations
- bootstrap 95% confidence intervals for residual means

Source artifact:
- [docs/findings/artifacts/hafnian_flux_sweep.csv](artifacts/hafnian_flux_sweep.csv)

Inference artifact:
- [docs/findings/artifacts/hafnian_flux_sweep_inference.json](artifacts/hafnian_flux_sweep_inference.json)

## Headline numbers

From the generated inference JSON:

- `pearson_residual_abs_vs_coherence = -0.8122`
- `pearson_residual_abs_vs_symmetry_gap = -0.2775`

Bootstrap means and 95% CI for $|\Delta\theta|$:

- coherent: mean $0.00131$, CI $[0.00076, 0.00197]$
- partial_coherence: mean $0.04013$, CI $[0.02845, 0.05305]$
- low_coherence: mean $1.62292$, CI $[1.18879, 2.04786]$
- symmetry_perturbed: mean $0.01106$, CI $[0.00824, 0.01398]$

## Interpretation

- The strong negative coherence correlation is consistent with the probe narrative: lower coherence is associated with larger residual.
- Symmetry-gap correlation appears weaker in this mixed dataset, likely because one family has non-zero gap while other families are exactly zero-gap; this suggests dedicated asymmetry sweeps should be isolated when estimating symmetry effects.

## Reproduction

```bash
/bin/python3 scripts/hafnian_flux_inference.py
```

## Scope and caution

This is descriptive inference on a deterministic generated dataset, not a formal proof and not a generalization guarantee across all matrix ensembles.
