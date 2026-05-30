# DigitalCrystal Roadmap

This roadmap is the minimum execution plan extracted from the carried CSIF/RWIF specification set and adapted to a Rust plus Docker scientific solver project.

For GitHub execution planning derived from this roadmap, see [docs/GITHUB_MILESTONES.md](docs/GITHUB_MILESTONES.md).

## Phase 0: Spec Lock-In

Goal:
- establish the copied spec pack as the governing source of truth

Deliverables:
- keep `specs/csif-guard/` unchanged as imported source material
- map implementation work back to spec sections
- record any future divergence through ADRs in `docs/adr/`

Exit criteria:
- every major implementation area points back to a governing spec document

## Phase 1: Rust Solver Skeleton

Goal:
- stand up a compile-clean Rust workspace with CSIF/RWIF-aligned interfaces

Deliverables:
- core engine crate for phase wrapping, state types, and route audit structures
- API crate for a service boundary
- immutable default config in `configs/solver.default.toml`

Exit criteria:
- workspace builds cleanly
- core types match the intent of `CSIF_V2_ENGINE_SPEC.md` and `CSIF_V2_RUST_ENGINE_TRAITS.md`

## Phase 2: RWIF Boundary

Goal:
- define the storage and replay boundary before expanding features

Deliverables:
- RWIF event, edge, crystal, and bank serialization models
- additive migration path for RWIF v1 to v2 inputs
- fixture corpus in `data/rwif/` and `tests/conformance/`

Exit criteria:
- persisted events map directly to the RWIF v2 field spec
- migration behavior is additive and testable

## Phase 3: Deterministic Equation Runtime

Goal:
- implement the first real geometric equation-solving flow

Deliverables:
- deterministic numeric path selection and stop reasons
- route audit output
- replay-safe result encoding

Exit criteria:
- identical input and config produce byte-stable outputs
- replay reproduces the same route decisions

Milestone 3.3 (Life Loop v0):
- add deterministic life-loop state models (goals, episodes, adaptation counters)
- implement tick scheduler with repeat-failure guard and auditable outcomes
- persist life-loop state to RWIF-adjacent JSON for restart-safe continuity
- expose life-loop health scoring to nurture reliable, knowledgeable behavior

## Phase 4: Containerized Service

Goal:
- package the runtime as a reproducible lab-friendly service

Deliverables:
- Docker image for the API process
- environment-driven configuration binding
- documented startup flow

Exit criteria:
- `docker build` succeeds
- container start command is documented and repeatable

## Phase 5: Qualification Gates

Goal:
- prevent expansion before the system is trustworthy

Deliverables:
- determinism gate
- contradiction/rejection gate
- replay gate
- performance baseline gate

Exit criteria:
- qualification report exists and all mandatory gates pass

## Phase 6: Science Domain Expansion

Goal:
- apply the engine to concrete scientific calculation workflows

Candidate targets:
- symbolic and numeric equation solving
- constrained system solving
- lab-oriented deterministic calculation APIs
- domain-specific crystalized problem banks

Exit criteria:
- each new domain ships with fixtures, replay cases, and benchmark evidence

## Immediate Next Actions

1. Add RWIF-backed conformance fixtures for Life Loop state snapshots and replay regression.
2. Add controlled goal-nurture policies (confidence decay/recovery schedules) behind immutable config flags.
3. Add milestone-level perf/health baselines for life-loop tick throughput and adaptation rate.