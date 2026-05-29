# CSIF-Guard Spec Transfer Manifest

Source repo:
- /home/mogir/Desktop/Mogir_Jason_Rofick/AI-GitHub_projects/CSIF-Guard

Destination folder:
- /home/mogir/Desktop/Mogir_Jason_Rofick/Science/DigitalCrystal/specs/csif-guard

Purpose:
- Carry forward the authoritative specification documents needed to maintain the CSIF/RWIF runtime, storage, conformance, and semantic technology contracts in the new DigitalCrystal project.

Selected authoritative documents:
- CSIF_V2_ENGINE_SPEC.md
  - Core CSIF runtime and boundary contract.
- RWIF_V2_FIELD_SPEC.md
  - Canonical RWIF v2 storage and replay schema contract.
- CSIF_V2_CONFORMANCE_TEST_SPEC.md
  - Normative conformance requirements tied to the CSIF and RWIF specs.
- CSIF_V2_RUST_ENGINE_TRAITS.md
  - Rust-facing trait and interface contract for the solver implementation.
- CSIF_RWIF_V2_PROJECT_BLUEPRINT.md
  - Combined architecture and project-maintenance blueprint.
- CSIF_RWIF_V2_IMPLEMENTATION_QUICKSTART.md
  - Operational guide for implementing and validating the spec set.
- SEMANTIC_LAYER0_SPEC_V0_2.md
  - Technology-layer baseline semantic contract.
- SEMANTIC_LAYER1_SPEC_V0_4.md
  - Technology-layer semantic expansion contract.
- SEMANTIC_LAYER2_SPEC_V0_4.md
  - Technology-layer semantic and stability contract.
- SEMANTIC_LAYER3_SPEC_V0_1.md
  - Highest current semantic-layer specification in this repo.

Intentionally excluded:
- README.md
  - Useful project overview, but not the canonical source for the current v2 specification set and includes older framing.
- csif_agent_v2/README.md and csif_agent_v2_rust/README.md
  - Useful implementation guidance, but not authoritative spec documents.
- tests/conformance/*.json and benchmark/baseline JSON outputs
  - Validation artifacts and outputs, not governing specifications.
- storage/rwif.py and migration scripts
  - Reference implementation/code, not documents.

Note:
- No document explicitly named CRIS was found in this repo. The authoritative crystal/runtime specification family present here is the CSIF/RWIF v2 set plus the semantic-layer and Rust trait specifications.
