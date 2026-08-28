//! Phase D — feature-combination bookkeeping.
//!
//! Asserts that the set of cargo features declared in `Cargo.toml` is exactly
//! the set the verification matrix covers, so that adding a feature without
//! extending `run_verification.sh` fails the build.

mod common;

use common::assert_same;

/// The complete set of `[features]` keys the verification matrix knows about.
const KNOWN_FEATURES: &[&str] = &["default", "test_internals"];

#[test]
fn cargo_toml_declares_only_known_features() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    let mut in_features = false;
    let mut declared: Vec<String> = Vec::new();
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_features = line == "[features]";
            continue;
        }
        if !in_features || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, _)) = line.split_once('=') {
            declared.push(k.trim().to_string());
        }
    }
    declared.sort();
    let mut known: Vec<String> = KNOWN_FEATURES.iter().map(|s| s.to_string()).collect();
    known.sort();
    assert_eq!(
        declared, known,
        "Cargo.toml features changed — update run_verification.sh and this list"
    );
}

/// Runs under every feature combination: the public behaviour must be identical
/// no matter which features are enabled, because features only add test-only
/// exports and must never alter `memchra2`.
#[test]
fn memchra2_behaviour_is_feature_independent() {
    // A fixed vector of representative inputs (mirrors the CONFIGS.md classes).
    let cases: &[(i32, i32, i32, i32)] = &[
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -2, -3, -4),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (0x3F80_0000u32 as i32, 0, 0, 0),
        (0x447A_0000u32 as i32, 0, 0, 0),
        (0x7F80_0000u32 as i32, 0, 0, 0),
        (0x7FC0_0000u32 as i32, 0, 0, 0),
        (0xFF80_0000u32 as i32, 0, 0, 0),
        (0x4479_FFFFu32 as i32, -1, 255, 256),
        (12345, -67890, 0, i32::MIN),
    ];
    // Values pinned from the C ground truth; identical for every feature combo.
    let mut observed = Vec::new();
    for &(a, b, c, d) in cases {
        observed.push(assert_same(a, b, c, d));
    }
    let feature_tag = if cfg!(feature = "test_internals") {
        "test_internals"
    } else {
        "default"
    };
    println!("feature combo `{feature_tag}` results: {observed:?}");
    assert_eq!(observed.len(), cases.len());
}
