use std::{fs, path::PathBuf};

use digitalcrystal_engine::{
    BankRecord, CrystalRecord, RWIF_EDGE_SCHEMA_VERSION, RWIF_EVENT_SCHEMA_VERSION,
    RWIF_SCHEMA_VERSION, migrate_bank_to_v2, replay_signature_for_crystal, validate_bank,
    validate_crystal,
};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/rwif_v2/fixtures")
        .join(name)
}

fn load_bank_fixture(name: &str) -> BankRecord {
    let contents = fs::read_to_string(fixture_path(name)).expect("fixture should read");
    serde_json::from_str(&contents).expect("fixture should parse")
}

fn load_crystal_fixture(name: &str) -> CrystalRecord {
    let contents = fs::read_to_string(fixture_path(name)).expect("fixture should read");
    serde_json::from_str(&contents).expect("fixture should parse")
}

#[test]
fn rwif_v1_fixture_migrates_additively_to_v2() {
    let migrated = migrate_bank_to_v2(load_bank_fixture("RWIF-C-001-v1-bank.json"));
    let report = validate_bank(&migrated);

    assert!(report.valid, "expected migrated bank to validate: {:?}", report.issues);
    assert_eq!(migrated.rwif_schema_version.as_deref(), Some(RWIF_SCHEMA_VERSION));
    assert_eq!(
        migrated.crystals[0].rwif_schema_version.as_deref(),
        Some(RWIF_SCHEMA_VERSION)
    );
    assert_eq!(
        migrated.crystals[0].edges[0].schema_version.as_deref(),
        Some(RWIF_EDGE_SCHEMA_VERSION)
    );
    assert_eq!(
        migrated.crystals[0].edges[0].phase_trajectory[0].schema_version.as_deref(),
        Some(RWIF_EVENT_SCHEMA_VERSION)
    );
    assert_eq!(
        migrated.crystals[0].edges[0].phase_trajectory[0].phase_theta,
        Some(0.5236)
    );
}

#[test]
fn invalid_rwif_v2_fixture_reports_missing_integer_wrap_mode() {
    let bank = load_bank_fixture("RWIF-C-002-v2-invalid-missing-integer-wrap.json");
    let report = validate_bank(&bank);

    assert!(!report.valid, "expected invalid bank to fail validation");
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == "RWIF_EDGE_INTEGER_WRAP_MODE_MISSING"));
}

#[test]
fn crystal_fixture_validates_and_round_trips() {
    let crystal = load_crystal_fixture("RWIF-C-003-v2-crystal.json");
    let report = validate_crystal(&crystal);
    assert!(report.valid, "expected valid crystal fixture: {:?}", report.issues);

    let as_value = serde_json::to_value(&crystal).expect("crystal should serialize");
    let round_trip: CrystalRecord = serde_json::from_value(as_value).expect("crystal should deserialize");
    assert_eq!(crystal, round_trip);
}

#[test]
fn crystal_replay_signature_is_stable_across_round_trip() {
    let crystal = load_crystal_fixture("RWIF-C-003-v2-crystal.json");
    let baseline = replay_signature_for_crystal(&crystal).expect("signature should serialize");

    let round_trip: CrystalRecord = serde_json::from_str(
        &serde_json::to_string(&crystal).expect("crystal should serialize"),
    )
    .expect("crystal should deserialize");
    let repeated = replay_signature_for_crystal(&round_trip).expect("signature should serialize");

    assert_eq!(baseline, repeated);
}