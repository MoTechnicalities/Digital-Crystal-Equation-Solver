# RH Theorem Chain (v0.1)

Status: Draft theorem-obligation scaffold, not a completed proof.

## Objective

Define a reviewable theorem chain from the current geometric-logic invariant layer to a formally checkable RH proof/disproof program.

## Canonical Target

Prove or disprove:
- every non-trivial zero of zeta has real part `1/2`.

## Chain Skeleton

1. C1: Path/Endpoint signature determinism
- Inputs: deterministic evaluator contract, explicit `path_signature` and `endpoint_signature` fields.
- Claim: under fixed input/options, signatures are stable across reruns.
- Evidence path: Rust conformance tests (T2), witness artifacts.

2. C2: Constraint-path distinguishability
- Inputs: expression families with distinct parse/derivation paths.
- Claim: path signatures separate non-equivalent constraint paths.
- Evidence path: Rust/API conformance tests (T1), witness report family checks.

3. C3: Path-endpoint decoupling witnesses
- Inputs: algebraically equivalent endpoints with distinct derivation paths.
- Claim: there exist families where endpoint value equality coexists with path-signature inequality.
- Evidence path: Rust/API conformance tests (T3), witness report witness families.

4. C4: RH-equivalent statement mapping layer
- Inputs: selected RH-equivalent statements and admissible transform rules.
- Claim: transformations preserve proof obligations and do not erase required assumptions.
- Evidence path: `docs/findings/RH_EQUIVALENT_STATEMENT_MAP_V0_1.md`.

5. C5: Lemma closure and contradiction elimination
- Inputs: explicit lemma registry and dependency graph.
- Claim: each lemma is either proven under declared assumptions or rejected with tracked contradiction.
- Evidence path: `docs/findings/RH_LEMMA_REGISTRY_V0_1.md`.

6. C6: Final theorem manuscript
- Inputs: complete chain C1..C5 with independent checks.
- Claim: external reviewers can validate proof/disproof end-to-end.
- Evidence path: `docs/findings/RH_COMPLETE_PROOF.md`.

## Obligation Registry (Initial)

- O-C1: signature stability report remains passing.
- O-C2: path distinguishability report remains passing.
- O-C3: at least one path-endpoint decoupling witness family remains passing.
- O-C4: RH equivalent-statement map document exists with explicit assumptions.
- O-C5: lemma registry exists with status fields (`open|satisfied|rejected`).
- O-C6: externally reviewable complete manuscript exists.

## Immediate Implementation Notes

- O-C1..O-C5 now have committed baseline artifacts.
- O-C6 remains open pending full proof/disproof closure.
