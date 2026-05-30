# Logic as Geometry of Constraints: Invariants Framework (v0.1)

Status: Formal definition section for testable theorem development.

## 1. Motivation

Arithmetic expressions are usually treated as value-producing syntax. In this lab framework, they are constrained composition programs that also induce geometric trajectories.

The central claim is:

- logic is the geometry of constrained operator paths

This document formalizes that claim with two invariants.

## 2. Core Objects

Let an expression evaluation produce:

$$
E \mapsto (V(E), \Pi(E), \Theta(E))
$$

where:
- $V(E)$ is the endpoint value,
- $\Pi(E)$ is the ordered operator path induced by parse tree and precedence,
- $\Theta(E)$ is the terminal geometric phase state (for example, cumulative phase signature).

Let $T(E)$ be the parse tree chosen by parentheses and precedence rules.

## 3. Invariant A: Path Signature Invariant (Tree/Order-Sensitive)

Definition:

$$
\mathrm{PSI}(E) := \mathcal{S}(T(E), \Pi(E))
$$

where $\mathcal{S}$ is a deterministic encoding of:
- tree structure,
- operator sequence,
- stepwise geometric annotations used by the engine.

Required property:

- If two expressions differ in grouping or operation order, then PSI must differ.

Formally:

$$
T(E_1) \neq T(E_2) \;\Rightarrow\; \mathrm{PSI}(E_1) \neq \mathrm{PSI}(E_2)
$$

Interpretation:
- PSI captures logic-level constraint differences even when values are numerically close.

## 4. Invariant B: Endpoint Invariant (Value-Sensitive)

Definition:

$$
\mathrm{EI}(E) := \mathcal{E}(V(E), \Theta(E))
$$

where $\mathcal{E}$ is a deterministic encoding of:
- evaluated endpoint value,
- terminal phase state.

Required property:

- If endpoint value or terminal phase differs beyond tolerance policy, EI must differ.

Formally:

$$
(V(E_1), \Theta(E_1)) \neq (V(E_2), \Theta(E_2)) \;\Rightarrow\; \mathrm{EI}(E_1) \neq \mathrm{EI}(E_2)
$$

Interpretation:
- EI captures semantic endpoint distinctions regardless of internal path details.

## 5. Joint Semantics

A robust expression identity requires both invariants:

$$
E_1 \equiv E_2 \iff \mathrm{PSI}(E_1)=\mathrm{PSI}(E_2) \;\wedge\; \mathrm{EI}(E_1)=\mathrm{EI}(E_2)
$$

This yields a two-axis semantics:
- logic axis: path and constraints (PSI)
- endpoint axis: value and terminal geometry (EI)

## 6. Minimal Example

Compare:

1. $E_1 = 2 + 2 \times 3$
2. $E_2 = (2 + 2) \times 3$

Expected under this framework:

1. $\mathrm{PSI}(E_1) \neq \mathrm{PSI}(E_2)$ because tree and order differ.
2. $\mathrm{EI}(E_1) \neq \mathrm{EI}(E_2)$ because endpoint value differs.

This is the concrete version of:
- parentheses are geometric constraints

## 7. Testable Theorem Candidates

### Theorem Candidate T1 (Constraint Distinguishability)

Statement:
- Distinct valid parenthesizations of the same token multiset induce distinct PSI values.

Test shape:
1. Generate parenthesized variants with fixed tokens.
2. Evaluate each expression deterministically.
3. Assert pairwise PSI inequality.

### Theorem Candidate T2 (Endpoint Stability)

Statement:
- Under fixed runtime configuration, repeated evaluations of the same expression produce stable EI.

Test shape:
1. Repeat evaluation $N$ times per expression.
2. Assert EI is byte-stable or tolerance-stable by policy.

### Theorem Candidate T3 (Path-Endpoint Decoupling Cases)

Statement:
- There exist expression pairs where PSI differs while EI value component matches.

Test shape:
1. Search expression families with algebraic endpoint agreement.
2. Check PSI inequality with endpoint-value equality.
3. Record phase component behavior in EI.

## 8. Operationalization in This Repo

Current implementation surfaces needed ingredients in deterministic math responses:
- derivation trace and rule sequence for PSI construction,
- phase signature and result object for EI construction.

Suggested artifact extension:
- add explicit `path_signature` and `endpoint_signature` fields to the response envelope for direct theorem testing.

## 9. Acceptance Criteria for Framework Promotion

The framework is considered operational when:

1. PSI encoder is deterministic and documented.
2. EI encoder is deterministic and tolerance-policy documented.
3. T1 and T2 pass on a stable conformance subset.
4. At least one T3 witness pair is documented in findings.

## 10. Scope and Caution

This section defines a formal testing framework for the claim "logic = geometry of constraints." It does not by itself prove broad mathematical universality. Proof strength depends on successful theorem tests and future generalization work.
