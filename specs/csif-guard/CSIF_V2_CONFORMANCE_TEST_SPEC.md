# CSIF v2 Conformance Test Specification

This document defines executable conformance tests for:
- [CSIF_V2_ENGINE_SPEC.md](CSIF_V2_ENGINE_SPEC.md)
- [RWIF_V2_FIELD_SPEC.md](RWIF_V2_FIELD_SPEC.md)

It includes required core tests and optional experimental profile tests.

## 1. Test Objectives

A CSIF implementation passes conformance when it demonstrates:
- deterministic, byte-stable query outputs
- correct phase wrapping and reverse traversal identity
- correct integer overflow handling under configured policy
- explicit stop reasons and contradiction gating
- append-only RWIF-compatible event emission

## 2. Conformance Levels

- L1 Core Required: Sections 4 through 15 of CSIF v2 spec.
- L2 Experimental Operator-Phase: Section 16.
- L3 Experimental Language/Lobe Geometry: Section 17.

Release gate recommendation:
- Production: L1 REQUIRED.
- Research profile: L1 + selected L2/L3 tests.

## 3. Canonical Test Output Contract

Each test case MUST emit a machine-readable result object with:
- test_id
- spec_section
- pass (bool)
- observed
- expected
- notes

A full run MUST emit:
- total
- passed
- failed
- conformance_level

## 4. L1 Core Required Tests

### T-001 Determinism Replay

Spec: 4.1, 4.3, 14

Procedure:
1. Load fixed graph and config.
2. Execute same query N times (N >= 100).
3. Canonically encode query result bytes each run.
4. Compare hashes.

Pass criteria:
- All hashes identical.

### T-002 wrap_pi Principal Interval

Spec: 5.1

Procedure:
1. Evaluate wrap_pi for boundary and overflow angles:
- -4pi, -3pi, -2pi, -pi, 0, pi, 2pi, 3pi, 4pi
2. Verify output bounded to [-pi, pi).

Pass criteria:
- All values inside principal domain.
- Known boundary expectations satisfied.

### T-003 Reverse Traversal Identity

Spec: 6.2, 8

Procedure:
1. Sample edge phases theta_i.
2. Compute reverse phase r_i = wrap_pi(-theta_i).
3. Verify wrap_pi(theta_i + r_i) == 0 under final rounding policy.

Pass criteria:
- Residual equals configured zero tolerance.

### T-004 Integer Wrap Mode Clamp

Spec: 5.3, 9

Procedure:
1. Run signed-state composition with integer_wrap_mode=clamp.
2. Inject over/underflow updates beyond range.

Pass criteria:
- Values saturate to configured min/max.
- No panic/overflow abort.

### T-005 Integer Wrap Mode Overflow Modulo

Spec: 5.3, 9

Procedure:
1. Run signed-state composition with integer_wrap_mode=overflow_modulo.
2. Inject over/underflow updates beyond range.

Pass criteria:
- Values wrap modulo configured signed domain.
- No panic/overflow abort.

### T-006 Stop Reason Completeness

Spec: 8, 12, 14

Procedure:
1. Execute scenario set covering:
- path found
- no path
- anti-lobe match
- contradiction
- timeout/budget
2. Verify route audit stop_reason.

Pass criteria:
- Correct stop reason per scenario.
- No missing stop_reason.

### T-007 Contradiction Threshold Gate

Spec: 7.1, 7.2, 7.3

Procedure:
1. Construct multi-path pair with known residual below threshold.
2. Construct multi-path pair above threshold.

Pass criteria:
- Below threshold: no contradiction gate.
- Above threshold: contradiction_detected.

### T-008 RWIF Event Mapping Integrity

Spec: 10, 13

Procedure:
1. Execute query/update cycle.
2. Persist trajectory events.
3. Validate required field mapping:
- phase, confidence_band, drift_delta, event_type, source
4. Validate optional v2 state fields are present when enabled.

Pass criteria:
- Required mapping complete.
- No destructive mutation of prior events.

## 5. L2 Experimental Operator-Phase Tests

### T-101 Operator Map Publication

Spec: 16.1

Procedure:
1. Enable operator-phase profile.
2. Read engine config surface.

Pass criteria:
- Active operator phase map present and explicit.

### T-102 Precedence Trajectory Separation

Spec: 16.2

Procedure:
1. Parse/execute `2 + 2 * 3`.
2. Parse/execute `(2 + 2) * 3`.
3. Compare operator-phase trajectories.

Pass criteria:
- Distinct trajectories OR explicit canonical rewrite justification.

### T-103 Ambiguity Torsion Score

Spec: 16.3

Procedure:
1. Select expression/sentence with >=2 valid parses.
2. Compute r_ambiguity between parse trajectories.

Pass criteria:
- Non-zero graded ambiguity signal emitted.
- Warning includes residual magnitude.

## 6. L3 Experimental Language/Lobe Tests

### T-201 Cross-Lobe Structural Equivalence

Spec: 17.1

Procedure:
1. Encode semantically equivalent statements in >=2 lobes/languages.
2. Compute intra-lobe and cross-lobe residuals.

Pass criteria:
- Cross-lobe residual below configured equivalence threshold.
- Calibration confidence reported.

### T-202 Soft Negation Ordering

Spec: 17.2

Procedure:
1. Evaluate phrase set from positive to negative continuum.
2. Compare phase positions/residual ranking.

Pass criteria:
- Ordering follows configured soft-negation monotonic direction.

### T-203 Experimental Metric Emission

Spec: 17.3

Procedure:
1. Run synonym, ambiguity, metaphor, humor benchmark set.
2. Verify metric object outputs.

Pass criteria:
- Metric fields emitted with deterministic schema.
- Advisory metrics do not override core safety gates.

## 7. Recommended Test Artifacts

- `fixtures/core_graph.json`
- `fixtures/contradiction_cases.json`
- `fixtures/operator_phase_cases.json`
- `fixtures/language_cases.json`
- `results/conformance_summary.json`

## 8. CI Gate Policy

Recommended CI blocking rules:
- FAIL build if any L1 test fails.
- Warn (non-blocking) for L2/L3 unless explicitly promoted.

## 9. Minimal Runner Interface (Pseudo)

```text
run_conformance --level L1 --fixtures fixtures/ --out results/conformance_summary.json
run_conformance --level L2 --fixtures fixtures/ --out results/conformance_summary_l2.json
run_conformance --level L3 --fixtures fixtures/ --out results/conformance_summary_l3.json
```

## 10. Conformance Verdict Format

```json
{
  "suite": "csif_v2_conformance",
  "level": "L1",
  "total": 8,
  "passed": 8,
  "failed": 0,
  "verdict": "PASS",
  "tests": []
}
```
