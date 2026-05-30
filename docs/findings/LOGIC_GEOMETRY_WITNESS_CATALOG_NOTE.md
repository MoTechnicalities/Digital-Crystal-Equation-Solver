# Logic-Geometry Witness Catalog Note (v0.1)

Status: Experimental launch note for geometric-logic theorem checks.

## Intent

Kick off concrete experiments for the "logic = geometry of constraints" framework by generating a witness catalog with provisional signature encoders.

This run operationalizes:
- Path Signature Invariant (PSI) as a deterministic hash over derivation path structure.
- Endpoint Invariant (EI) as a deterministic hash over endpoint value and terminal phase metadata.

## Protocol

Script:
- `scripts/logic_geometry_witness_catalog.py`

Runtime setup:
- endpoint: `POST /v1/csif/math`
- mode: `algebraic`
- repeats per expression: `5`

Expression families tested:
- precedence pair: `2 + 2 * 3` vs `(2 + 2) * 3`
- additive association pair: `(2 + 3) + 4` vs `2 + (3 + 4)`
- multiplicative association pair: `(2 * 3) * 4` vs `2 * (3 * 4)`
- distributive pair: `(1 + 2) * 3` vs `(1 * 3) + (2 * 3)`

## Artifacts

- dataset CSV: [docs/findings/artifacts/logic_geometry_witness_catalog.csv](artifacts/logic_geometry_witness_catalog.csv)
- report JSON: [docs/findings/artifacts/logic_geometry_witness_report.json](artifacts/logic_geometry_witness_report.json)

## Results (First Pass)

From the generated report:

- T1 Constraint Distinguishability: `PASS`
- T2 Stability (PSI/EI across repeats): `PASS`
- T3 Path-Endpoint Decoupling Witnesses: found in
  - `assoc_add_pair_b`
  - `assoc_mul_pair_c`
  - `distrib_pair_d`

Interpretation:
- Distinct grouping/order produces distinct path signatures as expected.
- Deterministic replay behavior is stable under repeat runs for these samples.
- Multiple families show same endpoint value but different path signature, consistent with your geometric-logic claim.

## Geometric Logic Readout

Your principle remains central:

- parentheses are geometric constraints on admissible phase trajectories

This run turns that from narrative into inspectable artifacts with machine-checkable signature deltas.

## Reproduction

```bash
/bin/python3 scripts/logic_geometry_witness_catalog.py
```

## Next Step

Promote provisional signatures to first-class API fields (`path_signature`, `endpoint_signature`) and mirror these checks in conformance tests for T1/T2/T3 continuity.
