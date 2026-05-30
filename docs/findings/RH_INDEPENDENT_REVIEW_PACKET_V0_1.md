# RH Independent Review Packet (v0.1)

Status: Initial external-review packet for C6-SUB-05.

## 1. Scope

This packet provides a deterministic replication path for current RH proof-program artifacts. It does not claim completion of RH proof/disproof.

## 2. Required Environment

- Linux workstation with Rust toolchain and Python 3.
- Repository root: `DigitalCrystal/`.

## 3. Replication Steps

1. Verify deterministic conformance slices:

```bash
cargo test -p digitalcrystal-engine
cargo test -p digitalcrystal-api
```

2. Regenerate witness and proof-status artifacts:

```bash
/bin/python3 scripts/logic_geometry_witness_catalog.py
/bin/python3 scripts/rh_proof_pipeline_status.py
```

3. Inspect generated status artifact:

- `docs/findings/artifacts/rh_proof_pipeline_status.json`

## 4. Expected Outcomes (Current Baseline)

- O1 through O6: `satisfied`.
- O7: `open` until external verification artifacts are completed and validated.
- C6 checklist states:
  - `C6-SUB-01`: true
  - `C6-SUB-02`: true
  - `C6-SUB-03`: true
  - `C6-SUB-04`: true
  - `C6-SUB-05`: true
  - `C6-SUB-06`: true

## 5. Review Assertions

Independent reviewer should verify:
- artifacts are reproducible from clean checkout;
- no external-verification completion claim is made unless O7 artifacts are valid;
- manuscript checklist state matches machine-reported state.

## 6. Reviewer Handoff Checklist

Provide reviewer with the following files at one fixed commit:

- `docs/findings/RH_COMPLETE_PROOF.md`
- `docs/findings/RH_THEOREM_CHAIN_V0_1.md`
- `docs/findings/RH_EQUIVALENT_STATEMENT_MAP_V0_1.md`
- `docs/findings/RH_LEMMA_REGISTRY_V0_1.md`
- `docs/findings/artifacts/rh_proof_pipeline_status.json`
- `docs/findings/artifacts/rh_independent_review_attestations.json`
- `docs/findings/artifacts/rh_reproducibility_manifest.json`
- `docs/findings/artifacts/rh_proof_version_lock.json`

Handoff metadata to include in reviewer ticket/email:

- repository commit SHA under review
- reviewer environment identifier convention
- required command sequence from Section 3
- expected contradiction-audit hash from current status artifact

## 7. Reviewer Return Protocol

Reviewer must return all three artifacts with concrete values:

1. Attestations artifact
- file: `docs/findings/artifacts/rh_independent_review_attestations.json`
- required: populated `reviewer_id`, `environment_id`, `outcome`, `signed_reference`
- acceptance: at least one `outcome: supports`

2. Reproducibility manifest
- file: `docs/findings/artifacts/rh_reproducibility_manifest.json`
- required: at least one `environment_origin: independent` run
- acceptance: at least one such run marked `status: passed`

3. Proof version lock
- file: `docs/findings/artifacts/rh_proof_version_lock.json`
- required: `proof_document`, `proof_commit`, `contradiction_audit_hash`, `locked_at`
- acceptance: `contradiction_audit_hash` matches script-computed expected hash

## 8. Acceptance Gate (O7)

O7 is satisfied only when all conditions hold:

- attestations artifact is valid and includes supporting independent outcome
- reproducibility manifest includes independent passed run
- proof version lock hash matches current contradiction-audit sections

Otherwise `prize_ready` must remain `false`.
