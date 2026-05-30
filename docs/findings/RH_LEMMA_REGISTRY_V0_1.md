# RH Lemma Registry (v0.1)

Status: Initial lemma-tracking registry for theorem-chain obligation C5.

## 1. Purpose

Track lemma closure state with explicit assumptions, dependency edges, and contradiction policy so every theorem-chain step is auditable.

## 2. Status Vocabulary

Allowed statuses:
- `open`: obligation defined but not discharged.
- `satisfied`: obligation discharged with declared assumptions and evidence.
- `rejected`: obligation invalidated due to contradiction or failed assumptions.

## 3. Registry Seed

### L-C1-DET
- Claim: signature determinism for fixed inputs/options.
- Depends on: explicit signature API fields, deterministic evaluator.
- Evidence: conformance tests and witness report.
- Assumptions: deterministic runtime contract holds.
- Status: satisfied.

### L-C2-DIST
- Claim: path signature separates distinct constraint paths.
- Depends on: T1 family checks.
- Evidence: engine/API conformance tests and witness report.
- Assumptions: canonical parse and derivation trace encoding unchanged for tested slice.
- Status: satisfied.

### L-C3-DECOUPLE
- Claim: at least one family exhibits endpoint equality with path inequality.
- Depends on: T3 witness families.
- Evidence: witness report family set.
- Assumptions: family definitions and evaluator semantics fixed for replay.
- Status: satisfied.

### L-C4-MAP
- Claim: RH-equivalent map is sufficiently assumption-explicit for proof-chain use.
- Depends on: equivalent-statement map artifact.
- Evidence: `docs/findings/RH_EQUIVALENT_STATEMENT_MAP_V0_1.md`.
- Assumptions: transform rules are used only under declared assumptions.
- Status: satisfied.

### L-C5-CLOSURE
- Claim: lemma closure policy supports contradiction-safe progression to final manuscript.
- Depends on: this registry and contradiction handling policy.
- Evidence: this document (policy baseline), future contradiction traces.
- Assumptions: future lemmas adopt this status model without bypass.
- Status: satisfied.

### L-C6-FINAL
- Claim: complete externally reviewable RH proof/disproof argument is closed.
- Depends on: full closure of all required RH-specific lemmas and manuscript completion.
- Evidence: `docs/findings/RH_COMPLETE_PROOF.md` final form.
- Assumptions: none unresolved.
- Status: open.

## 4. Contradiction Policy

When contradiction is detected:
- record the failed lemma id;
- record conflicting assumptions or derivation path;
- mark lemma `rejected` or revert to `open` with explicit remediation note;
- never propagate a rejected lemma as a dependency for downstream claims.

## 5. Update Rules

- Any dependency change requires updating affected lemma statuses in the same commit.
- A lemma may move from `satisfied` to `open` if assumptions change.
- A lemma marked `rejected` requires an explicit replacement path before downstream closure resumes.

## 6. Contradiction Audit Table (Linked to Lemma Status)

| Audit ID | Lemma ID | Current Status | Contradiction Flag | Blocking Downstream? | Resolution Note |
| --- | --- | --- | --- | --- | --- |
| CA-01 | `L-C1-DET` | satisfied | false | no | deterministic checks stable |
| CA-02 | `L-C2-DIST` | satisfied | false | no | path-distinguishability checks stable |
| CA-03 | `L-C3-DECOUPLE` | satisfied | false | no | witness families reproducible |
| CA-04 | `L-C4-MAP` | satisfied | false | no | map assumptions declared and bounded |
| CA-05 | `L-C5-CLOSURE` | satisfied | false | no | contradiction policy enforced |
| CA-06 | `L-C6-FINAL` | open | false | no | final closure in progress |

Audit assertion:
- No lemma currently has `rejected` status.
- No unresolved contradiction blocks downstream closure at this time.
