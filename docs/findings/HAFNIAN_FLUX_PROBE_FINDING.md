# Hafnian Flux Probe: Experimental Finding (v0.1)

Status: Experimental, deterministic instrumentation result.

## Why this exists

The engine now computes exact hafnians (dimension-capped) and attaches an experimental flux-oriented trust probe to hafnian derivation steps. This document records the first clean result and how to reproduce it.

## Finding Summary

For the symmetric 4x4 all-ones-off-diagonal matrix

$$
A =
\begin{bmatrix}
0 & 1 & 1 & 1 \\
1 & 0 & 1 & 1 \\
1 & 1 & 0 & 1 \\
1 & 1 & 1 & 0
\end{bmatrix}
$$

the exact hafnian is

$$
\operatorname{Haf}(A) = 3
$$

and the experimental flux probe reports perfect phase-coherence and zero residual between observed and predicted phase:

- coherence_magnitude: 1
- symmetry_gap_mean_abs: 0
- observed_hafnian_theta: 0
- predicted_uniform_theta: 0
- uniform_phase_residual: 0

Interpretation in this first controlled case: when off-diagonal edge phases are uniformly aligned and symmetric, the observed hafnian phase follows the uniform-phase prediction.

## Probe Hypothesis

The probe currently publishes this hypothesis string:

"If edge phases are coherent and symmetric, hafnian phase tends to follow n/2 * mean edge phase."

In compact form:

$$
\theta_{\text{pred}} = \operatorname{wrap}_{(-\pi,\pi]}\!\left(\frac{n}{2}\,\bar{\phi}\right)
$$

with residual

$$
\Delta\theta = \operatorname{wrap}_{(-\pi,\pi]}\!(\theta_{\text{obs}} - \theta_{\text{pred}})
$$

where:
- $n$ is matrix dimension,
- $\bar{\phi}$ is mean off-diagonal edge phase,
- $\theta_{\text{obs}}$ is phase of the exact hafnian.

## Exact Payload (Observed)

This payload was observed in the lab Trust dialog Flux Probe panel and via `POST /v1/csif/math` on the same run:

```json
{
  "coherence_magnitude": 1,
  "diagonal_max_abs": 0,
  "dimension": 4,
  "experimental": true,
  "hypothesis": "If edge phases are coherent and symmetric, hafnian phase tends to follow n/2 * mean edge phase.",
  "magnitude_coefficient_of_variation": 0,
  "mean_edge_magnitude": 1,
  "mean_edge_phase": 0,
  "observed_hafnian_theta": 0,
  "off_diagonal_pairs": 6,
  "predicted_uniform_theta": 0,
  "symmetry_gap_mean_abs": 0,
  "uniform_phase_residual": 0
}
```

## Reproduction

### API reproduction

Run:

```bash
curl -sS -X POST http://127.0.0.1:8080/v1/csif/math \
  -H 'content-type: application/json' \
  -d '{"expression":"hafnian([[0,1,1,1],[1,0,1,1],[1,1,0,1],[1,1,1,0]])","mode":"algebraic","angle_unit":"radians"}' \
  | jq '.derivation_trace[0].numeric_trust.hafnian_flux_probe'
```

Expected high-level outcome:
- exact result is `3`
- flux probe exists under `derivation_trace[0].numeric_trust.hafnian_flux_probe`
- coherence is `1` and residual is `0` for this case

### UI reproduction

1. Open `/labs/special-functions`.
2. Click `Hafnian sample`.
3. Click `Evaluate`.
4. In derivation trace, click `Trust` on `hafnian_matrix`.
5. Inspect the `Flux Probe` panel.

## Scope and Caution

This is not a theorem claim. It is an instrumentation-backed deterministic observation for controlled examples. The probe is intended for hypothesis tracking and comparative experiments, not as a substitute for formal proof.

## Implementation Anchors

- Engine probe computation and trust attachment: `crates/digitalcrystal-engine/src/lib.rs`
- Trust dialog Flux Probe panel rendering: `apps/api/src/main.rs`

## Next Experiment Set

- Structured random complex symmetric matrices with controlled phase dispersion.
- Residual-vs-coherence sweep to test monotonicity trends.
- Dimension sweep near the exact hafnian cap.
- Cases with deliberate symmetry-gap perturbations to quantify residual growth.
