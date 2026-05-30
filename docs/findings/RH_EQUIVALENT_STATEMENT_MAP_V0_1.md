# RH Equivalent Statement Map (v0.1)

Status: Initial assumption-aware mapping layer for theorem-chain obligation C4.

## 1. Purpose

Provide a constrained map of selected RH-equivalent statements with explicit assumptions and allowed transform links so theorem obligations can be tracked without hidden implication jumps.

## 2. Canonical Root Node

- Node `RH-ROOT`
- Statement: all non-trivial zeros of zeta satisfy `Re(s)=1/2`.
- Role: canonical target statement for the chain.

## 3. Selected Equivalent Nodes (Registry Seed)

- Node `EQ-HARDY-Z`
- Statement: all non-trivial zeros of Hardy's Z-function correspond to critical-line zeros under the standard zeta-line correspondence.
- Dependency intent: computational checks often project to Z-function formulations.

- Node `EQ-XI-PRODUCT`
- Statement: xi-function entire-product representation places all non-trivial zero ordinates on the transformed critical axis equivalent to RH.
- Dependency intent: bridge to entire-function framing and symmetry arguments.

- Node `EQ-PRIME-ERROR-BOUND`
- Statement: prime-counting error-term bound formulation equivalent to RH under standard analytic number theory assumptions.
- Dependency intent: downstream interpretation link, not currently used as a computational proof step.

## 4. Admissible Transform Rules

Transforms are permitted only when all listed assumptions are declared in the consuming lemma.

- Rule `T-ANALYTIC-CONTINUATION`
- From/To: zeta-domain formulations <-> equivalent entire-function formulations.
- Required assumptions: analytic continuation and functional equation context explicitly declared.

- Rule `T-ZETA-XI-NORMALIZATION`
- From/To: zeta/xi normalized identities where zero sets are preserved by declared factors.
- Required assumptions: factor non-vanishing in mapped region; domain exclusions documented.

- Rule `T-EQUIVALENT-BOUND-INTERPRETATION`
- From/To: statement-level translation to prime-error bounds.
- Required assumptions: exact theorem version and constants are specified.

## 5. Assumption Ledger

Every lemma that consumes a mapping edge must include:
- theorem source citation or internal lemma source id;
- domain restrictions;
- excluded singularities or exceptional sets;
- whether implication is used as equivalence or one-way implication.

## 6. Failure Criteria

Mapping usage is invalid if any of the following occur:
- undeclared assumption appears in a proof step;
- one-way implication is treated as equivalence;
- transform target is used outside declared domain;
- required exceptional-set treatment is omitted.

## 7. Machine-Checkable Seed Structure

Planned artifact shape for next iteration:
- graph node id
- statement text
- assumption set id list
- outgoing transform edge id list
- edge rule id and directionality

This v0.1 note is the authoritative textual source until graph serialization is committed.
