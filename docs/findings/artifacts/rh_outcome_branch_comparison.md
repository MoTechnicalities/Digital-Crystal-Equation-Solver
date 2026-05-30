# RH Outcome Branch Comparison

Question:
- Which branch is currently justified by the committed evidence, and which branch is currently excluded?

Reviewer note:
- Reviewer-facing comparison of the selected internal branch versus the currently excluded opposing branch.

## Selected Branch

- Branch: `rh_likely_true_internal`
- Status: `selected`
- Confidence band: `medium`
- Geometric signal score: `100`

Why this cup:
- Geometric theorem checks T1/T2 pass and T3 witnesses exist.
- No rejected lemmas are currently active in the chain.
- No validated off-critical-line candidate has been recorded.
- Dipole stress windows are present but none certifies an off-critical zero witness.

Positive contract conclusion:
- Current internal evidence is sufficient to pin RH to the likely-true branch.

Positive contract premises:
- P-01: T1 constraint-distinguishability passes on the committed witness slice. Holds: True.
- P-02: T2 endpoint-stability passes on repeated runs for the committed witness slice. Holds: True.
- P-03: At least one T3 path-endpoint decoupling witness exists. Holds: True.
- P-04: No active lemma is currently marked rejected in the RH chain. Holds: True.
- P-05: No validated off-critical-line counterexample candidate is currently recorded. Holds: True.
- P-06: Transition-threshold support artifact is present. Holds: True.
- P-07: Dipole analysis artifact is present with a non-empty probe set and ranked asymmetry windows. Holds: True.

## Excluded Branch

- Branch: `counterexample_candidate_internal`
- Status: `excluded_for_now`

Why not the other cup:
- The counterexample branch requires at least one validated off-critical-line zero candidate.
- The coarse counterexample search reports zero candidates.
- The adaptive refine search reports zero candidates and zero validated points.
- Refined near-miss points do not remain stable across all tested neighborhood scales.
- Dipole stress windows are present but do not produce a certified off-critical zero witness.

Exclusion contract conclusion:
- The current artifact set does not justify pinning the counterexample branch.

Exclusion requirements currently holding:
- N-01: The counterexample branch requires at least one validated off-critical-line zero candidate. Holds: True.
- N-02: The coarse counterexample search reports zero candidates. Holds: True.
- N-03: The adaptive refine search reports zero candidates and zero validated points. Holds: True.
- N-04: Refined near-miss points do not remain stable across all tested neighborhood scales. Holds: True.
- N-05: Dipole stress windows are present but do not produce a certified off-critical zero witness. Holds: True.

## Contrast Summary

- Selected branch conclusion: Current internal evidence is sufficient to pin RH to the likely-true branch.
- Excluded branch conclusion: The current artifact set does not justify pinning the counterexample branch.
- Decision rule: Select the RH-likely branch only when all positive premises hold and the counterexample branch lacks a validated off-critical-line witness.

Scope note:
- Comparison artifact only; this is an internal evidence contrast, not an external proof claim.
