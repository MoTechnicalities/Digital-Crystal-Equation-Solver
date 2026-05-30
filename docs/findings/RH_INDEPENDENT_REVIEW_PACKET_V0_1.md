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

- O1 through O5: `satisfied`.
- O6: `open`.
- C6 checklist states:
  - `C6-SUB-01`: true
  - `C6-SUB-02`: true
  - `C6-SUB-03`: true
  - `C6-SUB-04`: false
  - `C6-SUB-05`: expected true after this packet linkage is merged
  - `C6-SUB-06`: false

## 5. Review Assertions

Independent reviewer should verify:
- artifacts are reproducible from clean checkout;
- no proof-completion claim is made while C6-SUB-04/C6-SUB-06 remain open;
- manuscript checklist state matches machine-reported state.
