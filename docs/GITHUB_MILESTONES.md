# GitHub Milestones

This file turns the current roadmap into milestone-ready GitHub planning. Each issue below is written to be directly transferable into a GitHub issue body.

## Milestone 1: Contract Lock-In

Target outcome:
- the repo enforces the imported CSIF/RWIF specification boundary before broad implementation work begins

### Issue 1.1: Map implementation surfaces to governing specs

Summary:
- create a spec traceability map linking runtime, storage, API, and validation surfaces to the imported spec pack

Acceptance criteria:
- each major module references one or more governing spec documents
- divergence policy is documented in `docs/adr/`
- traceability doc exists in the repo

Spec refs:
- `CSIF_V2_ENGINE_SPEC.md`
- `RWIF_V2_FIELD_SPEC.md`
- `CSIF_RWIF_V2_PROJECT_BLUEPRINT.md`

### Issue 1.2: Define immutable baseline solver profile

Summary:
- finalize the baseline deterministic runtime profile for the first production implementation

Acceptance criteria:
- baseline config is committed
- engine mode, wrap mode, integer wrap mode, integration rule, and quantization are explicit
- documentation explains why the profile is the default

Spec refs:
- `CSIF_RWIF_V2_PROJECT_BLUEPRINT.md`
- `RWIF_V2_FIELD_SPEC.md`

## Milestone 2: RWIF Storage and Replay Correctness

Target outcome:
- the project starts from storage and replay correctness rather than feature breadth

### Issue 2.1: Complete RWIF v2 typed model coverage

Summary:
- extend the initial RWIF models to cover the full event, edge, crystal, and bank contract with stable serde behavior

Acceptance criteria:
- all RWIF v2 fields in the imported spec are represented
- unknown fields are preserved non-destructively
- serialization and deserialization round trips are tested

Spec refs:
- `RWIF_V2_FIELD_SPEC.md`

### Issue 2.2: Add additive RWIF v1 to v2 migration module

Summary:
- promote the current migration helpers into a dedicated, documented migration surface

Acceptance criteria:
- migration preserves existing values
- migration adds missing v2 metadata only
- rollback-safe fixture coverage exists

Spec refs:
- `RWIF_V2_FIELD_SPEC.md`
- `CSIF_RWIF_V2_PROJECT_BLUEPRINT.md`

### Issue 2.3: Build RWIF conformance fixture corpus

Summary:
- add the first formal fixture pack and test harness for valid v1, migrated v2, and invalid v2 documents

Acceptance criteria:
- fixture folder contains valid and invalid bank/crystal cases
- test output identifies failing conformance rules clearly
- fixture naming follows stable conformance IDs

Spec refs:
- `RWIF_V2_FIELD_SPEC.md`
- `CSIF_V2_CONFORMANCE_TEST_SPEC.md`

## Milestone 3: CSIF Runtime Core

Target outcome:
- stand up the first deterministic geometric runtime rather than only config and transport

### Issue 3.1: Implement route audit core types and stop-reason semantics

Summary:
- flesh out route audit structures and ensure all core stop reasons are emitted explicitly

Acceptance criteria:
- route audit supports selected path and contradiction metrics
- stop reasons cover success, no path, contradiction, and budget exhaustion
- tests map directly to conformance scenarios

Spec refs:
- `CSIF_V2_ENGINE_SPEC.md`
- `CSIF_V2_CONFORMANCE_TEST_SPEC.md`

### Issue 3.2: Implement deterministic phase helpers and reverse traversal identity

Summary:
- promote the current phase helper into the first real numeric core with reverse traversal checks

Acceptance criteria:
- principal interval behavior is tested against canonical boundary angles
- reverse traversal identity tests exist
- deterministic numeric policy is documented

Spec refs:
- `CSIF_V2_ENGINE_SPEC.md`
- `CSIF_V2_RUST_ENGINE_TRAITS.md`

## Milestone 4: Containerized Service Boundary

Target outcome:
- expose a stable, typed API boundary suitable for Dockerized scientific deployment

### Issue 4.1: Expand the Axum service from validation endpoints to solver endpoints

Summary:
- grow the current Axum boundary into a real solver-facing API surface

Acceptance criteria:
- API exposes health, config, RWIF validation, and first solver operation routes
- config loading is typed and startup-failing on invalid config
- route tests cover the public contract

Spec refs:
- `CSIF_V2_ENGINE_SPEC.md`
- `CSIF_V2_RUST_ENGINE_TRAITS.md`

### Issue 4.2: Harden Docker runtime contract

Summary:
- document and test the container boundary as the standard deployment path

Acceptance criteria:
- image builds reproducibly
- runtime config path is explicit
- container start instructions are documented

Spec refs:
- `CSIF_RWIF_V2_PROJECT_BLUEPRINT.md`

## Milestone 5: Qualification Gates

Target outcome:
- no major expansion occurs until determinism and replay gates exist

### Issue 5.1: Add deterministic replay gate

Summary:
- implement byte-stable replay verification across repeated runs

Acceptance criteria:
- repeated identical inputs produce identical encoded outputs
- gate is runnable locally and in CI
- failure output is diagnosable

Spec refs:
- `CSIF_V2_CONFORMANCE_TEST_SPEC.md`
- `CSIF_V2_ENGINE_SPEC.md`

### Issue 5.2: Add contradiction threshold conformance coverage

Summary:
- implement fixture-driven contradiction threshold tests for above and below threshold cases

Acceptance criteria:
- below-threshold cases do not gate
- above-threshold cases emit contradiction stop reasons
- results are summarized in a machine-readable form

Spec refs:
- `CSIF_V2_CONFORMANCE_TEST_SPEC.md`

## Milestone 6: First Science Use Cases

Target outcome:
- apply the deterministic engine to real scientific workloads only after contract correctness is established

### Issue 6.1: Define first science calculation domain pack

Summary:
- select and document the first scoped scientific calculation family for the solver

Acceptance criteria:
- domain scope is narrow and benchmarkable
- fixtures and expected outputs are committed
- performance and correctness metrics are defined up front

Spec refs:
- `CSIF_RWIF_V2_PROJECT_BLUEPRINT.md`

### Issue 6.2: Publish benchmark harness for lab-oriented workloads

Summary:
- establish reproducible benchmark inputs and reporting for scientific workloads

Acceptance criteria:
- benchmark corpus is versioned
- p50 and p95 latency reporting exists
- benchmark config is immutable during runs

Spec refs:
- `CSIF_RWIF_V2_PROJECT_BLUEPRINT.md`