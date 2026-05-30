# Findings Index

This directory stores reproducible experimental findings generated from the DigitalCrystal engine and lab surfaces.

## Reports

- [HAFNIAN_FLUX_PROBE_FINDING.md](HAFNIAN_FLUX_PROBE_FINDING.md)
  - First instrumentation-backed single-case finding and exact payload record.
- [HAFNIAN_FLUX_PROBE_SWEEP_FINDING.md](HAFNIAN_FLUX_PROBE_SWEEP_FINDING.md)
  - Controlled family sweep with CSV + SVG artifacts.
- [HAFNIAN_FLUX_PROBE_INFERENCE_NOTE.md](HAFNIAN_FLUX_PROBE_INFERENCE_NOTE.md)
  - Correlation and bootstrap confidence-interval follow-up on sweep results.
- [HAFNIAN_FLUX_PROBE_ASYMMETRY_ISOLATION_NOTE.md](HAFNIAN_FLUX_PROBE_ASYMMETRY_ISOLATION_NOTE.md)
  - Separate asymmetry-only protocol focused on symmetry-gap causality isolation.
- [HAFNIAN_FLUX_PHASE_TRANSITION_ATLAS_NOTE.md](HAFNIAN_FLUX_PHASE_TRANSITION_ATLAS_NOTE.md)
  - Multi-dimension transition atlas with coherence-cliff thresholds and CI bands.
- [LOGIC_GEOMETRY_INVARIANTS_FRAMEWORK.md](LOGIC_GEOMETRY_INVARIANTS_FRAMEWORK.md)
  - Formal definitions for Path Signature Invariant and Endpoint Invariant with theorem candidates.
- [LOGIC_GEOMETRY_WITNESS_CATALOG_NOTE.md](LOGIC_GEOMETRY_WITNESS_CATALOG_NOTE.md)
  - First experimental witness catalog validating provisional PSI/EI checks (T1/T2/T3).
- [RH_PROOF_PROGRAM_V0_1.md](RH_PROOF_PROGRAM_V0_1.md)
  - Prize-proof development program with obligation stages and artifact-driven status.
- [RH_THEOREM_CHAIN_V0_1.md](RH_THEOREM_CHAIN_V0_1.md)
  - RH-specific theorem-obligation chain scaffold from signature invariants to final manuscript.

## Artifacts

Generated artifacts are stored in [artifacts/](artifacts/) and are intended to be committed so results remain inspectable in GitHub.

Notable inference artifact:
- [artifacts/hafnian_flux_sweep_inference.json](artifacts/hafnian_flux_sweep_inference.json)

Notable asymmetry isolation artifacts:
- [artifacts/hafnian_flux_asymmetry_sweep.csv](artifacts/hafnian_flux_asymmetry_sweep.csv)
- [artifacts/hafnian_flux_asymmetry_sweep_summary.json](artifacts/hafnian_flux_asymmetry_sweep_summary.json)
- [artifacts/hafnian_flux_asymmetry_residual_vs_gap.svg](artifacts/hafnian_flux_asymmetry_residual_vs_gap.svg)

Notable phase-transition artifacts:
- [artifacts/hafnian_flux_transition_sweep.csv](artifacts/hafnian_flux_transition_sweep.csv)
- [artifacts/hafnian_flux_transition_heatmap.svg](artifacts/hafnian_flux_transition_heatmap.svg)
- [artifacts/hafnian_flux_transition_summary.json](artifacts/hafnian_flux_transition_summary.json)
- [artifacts/hafnian_flux_transition_thresholds.json](artifacts/hafnian_flux_transition_thresholds.json)

Notable logic-geometry artifacts:
- [artifacts/logic_geometry_witness_catalog.csv](artifacts/logic_geometry_witness_catalog.csv)
- [artifacts/logic_geometry_witness_report.json](artifacts/logic_geometry_witness_report.json)

Notable RH proof-program artifact:
- [artifacts/rh_proof_pipeline_status.json](artifacts/rh_proof_pipeline_status.json)
