# DigitalCrystal

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache__2.0-blue.svg)](LICENSE)

DigitalCrystal is a new open-source Rust and Docker project for science-grade deterministic equation solving built on the CSIF/RWIF specification family carried forward from CSIF-Guard.

The minimum project charter for this repository is:
- provide a Rust geometric crystal equation solver that is deterministic, auditable, and replayable
- expose that solver inside a Docker-first deployment model suitable for laboratories and workstation science workloads
- preserve a strict CSIF runtime contract for reasoning and route stability
- preserve a strict RWIF storage contract for append-only persistence, replay, and migration
- keep the system practical for CPU-first deployment while remaining extensible for future higher-performance execution modes

## Mission

Build a free, reproducible alternative to expensive equation-solving infrastructure by packaging a deterministic Rust geometric solver into a portable container stack that can be applied across science-oriented calculation domains.

## Non-Negotiable Contracts

The repository is governed by the specification set in [specs/csif-guard](specs/csif-guard):
- `CSIF_V2_ENGINE_SPEC.md`
- `RWIF_V2_FIELD_SPEC.md`
- `CSIF_V2_CONFORMANCE_TEST_SPEC.md`
- `CSIF_V2_RUST_ENGINE_TRAITS.md`
- `CSIF_RWIF_V2_PROJECT_BLUEPRINT.md`
- `CSIF_RWIF_V2_IMPLEMENTATION_QUICKSTART.md`
- `SEMANTIC_LAYER0_SPEC_V0_2.md`
- `SEMANTIC_LAYER1_SPEC_V0_4.md`
- `SEMANTIC_LAYER2_SPEC_V0_4.md`
- `SEMANTIC_LAYER3_SPEC_V0_1.md`

Project requirements extracted from that spec pack:
- deterministic output under fixed input and fixed configuration
- append-only RWIF-compatible event persistence
- explicit stop reasons and route audit traces
- additive RWIF v1 to v2 migration behavior
- replayability from persisted events
- measurable qualification gates for determinism, contradiction handling, replay, and throughput

## Minimum Scope

This initial scaffold sets up the smallest practical repository shape for the new project:
- a Rust workspace
- an engine crate for solver logic and spec-aligned types
- an API application crate for service exposure
- deterministic configuration files
- Docker build support
- a home for conformance and RWIF fixture data
- the copied specification set that governs future implementation work

## Repository Layout

```text
DigitalCrystal/
├── .github/workflows/          # CI entrypoint
├── apps/api/                   # API/service binary
├── crates/digitalcrystal-engine/ # solver core crate
├── configs/                    # immutable runtime profiles
├── data/rwif/                  # RWIF fixtures and migrated banks
├── docker/                     # container support files if expanded later
├── docs/adr/                   # architectural decision records
├── docs/findings/              # experimental findings and reproducible results
├── scripts/                    # helper automation
├── specs/csif-guard/           # carried-forward governing specs
├── tests/conformance/          # conformance fixtures and harnesses
├── Cargo.toml                  # workspace root
├── Dockerfile                  # container build
└── ROADMAP.md                  # phased execution plan
```

## Rust/Docker Charter

### Rust

Rust is the implementation language for the first production-grade solver because it best matches the copied trait/spec package:
- explicit numeric policy control
- deterministic serialization and replay behavior
- low-overhead CPU execution
- strong type boundaries for CSIF and RWIF contracts

### Docker

Docker is the default deployment boundary because it provides:
- reproducible lab and workstation deployment
- consistent runtime configuration
- portable packaging for science tools and APIs
- a clean path to benchmark, validate, and distribute the solver

## Immediate Build Target

Phase 1 for this repository is not feature breadth. It is contract lock-in:
- implement the spec-aligned Rust types and runtime skeleton
- define the immutable solver configuration profile
- stand up a containerized API process
- add conformance and determinism gates before broad domain expansion

## Getting Started

Build the workspace:

```bash
cargo build
```

Run the Axum API:

```bash
cargo run -p digitalcrystal-api -- --config configs/solver.default.toml
```

Build the container:

```bash
docker build -t digitalcrystal:dev .
```

Available starter routes:
- `GET /`
- `GET /labs/special-functions`
- `GET /health`
- `GET /v1/config`
- `GET /v1/platform/modules`
- `POST /v1/csif/math`
- `POST /v1/rwif/validate`
- `POST /v1/solve/linear`

The current API now exposes the first platform shell for the multi-domain product direction:
- a landing page served directly by the Rust API at `/`
- a first interactive Special Functions Lab page at `/labs/special-functions`
- a typed module catalog at `/v1/platform/modules`
- a first deterministic math endpoint at `/v1/csif/math`
- foundation solver and RWIF endpoints that future domain packs will build on

Current `POST /v1/csif/math` slice:
- follows the predecessor request shape: `expression`, optional `mode`, optional `angle_unit`
- supports deterministic scalar, matrix-literal, and first-pass complex expressions with `+`, `-`, `*`, `/`, `^`, parentheses, `[[...], [...]]`, `i`, `pi`, `e`, `abs`, `arg`, `conj`, `exp`, `ln`, `log`, `gamma`, `lambertw`, `zeta`, `polylog`, `gammainc`, `besselj`, `bessely`, `besseli`, `besselk`, `j_sph`, `det`, `inverse`, `hafnian`, `tf`, `sqrt`, `sin`, and `cos`
- supports implicit complex literals such as `2+3i` and expressions such as `exp(i*pi) + 1`
- uses complex-domain continuation for `bessely` and `besselk` instead of the earlier restricted real-only branch
- returns matrix-valued results when appropriate, a richer derivation trace, step-level `bridge_audit` math-job diagnostics, a phase signature with both discrete slot and cumulative trajectory values, and an RWIF-shaped export artifact

Current `/labs/special-functions` slice:
- evaluates expressions directly against `/v1/csif/math`
- renders derivation trace, bridge audit, RWIF export, and raw JSON
- includes matrix and transfer-function samples, expanded Bessel samples, and a selectable phase visualization that can render either discrete operator slots or cumulative composition

## License

This project is licensed under the Apache License 2.0. See [LICENSE](LICENSE).

## Next Documents To Read

Start here, in order:
1. [specs/csif-guard/CSIF_V2_ENGINE_SPEC.md](specs/csif-guard/CSIF_V2_ENGINE_SPEC.md)
2. [specs/csif-guard/RWIF_V2_FIELD_SPEC.md](specs/csif-guard/RWIF_V2_FIELD_SPEC.md)
3. [specs/csif-guard/CSIF_V2_RUST_ENGINE_TRAITS.md](specs/csif-guard/CSIF_V2_RUST_ENGINE_TRAITS.md)
4. [ROADMAP.md](ROADMAP.md)
5. [docs/GITHUB_MILESTONES.md](docs/GITHUB_MILESTONES.md)
6. [docs/findings/HAFNIAN_FLUX_PROBE_FINDING.md](docs/findings/HAFNIAN_FLUX_PROBE_FINDING.md)
7. [docs/findings/HAFNIAN_FLUX_PROBE_SWEEP_FINDING.md](docs/findings/HAFNIAN_FLUX_PROBE_SWEEP_FINDING.md)
8. [docs/findings/README.md](docs/findings/README.md)
9. [docs/findings/HAFNIAN_FLUX_PROBE_INFERENCE_NOTE.md](docs/findings/HAFNIAN_FLUX_PROBE_INFERENCE_NOTE.md)
10. [docs/findings/HAFNIAN_FLUX_PROBE_ASYMMETRY_ISOLATION_NOTE.md](docs/findings/HAFNIAN_FLUX_PROBE_ASYMMETRY_ISOLATION_NOTE.md)
11. [docs/findings/HAFNIAN_FLUX_PHASE_TRANSITION_ATLAS_NOTE.md](docs/findings/HAFNIAN_FLUX_PHASE_TRANSITION_ATLAS_NOTE.md)
12. [docs/findings/LOGIC_GEOMETRY_INVARIANTS_FRAMEWORK.md](docs/findings/LOGIC_GEOMETRY_INVARIANTS_FRAMEWORK.md)
13. [docs/findings/LOGIC_GEOMETRY_WITNESS_CATALOG_NOTE.md](docs/findings/LOGIC_GEOMETRY_WITNESS_CATALOG_NOTE.md)