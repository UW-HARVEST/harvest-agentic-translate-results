//! Phase D — feature-combination coverage.
//!
//! `translation/Cargo.toml` declares no `[features]` table and no
//! `cfg(feature = ...)` exists in `src/`, so the crate has exactly ONE build
//! configuration. This test pins that fact down so that adding a feature in
//! future forces the differential matrix to be extended, and re-runs a
//! representative differential sweep under whatever configuration is active
//! (`--no-default-features`, `--all-features`, debug and release all execute
//! this file).

mod common;
use common::{Libs, Rng};

#[test]
fn crate_declares_no_features() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    // Ignore the [dev-dependencies] we added for the harness.
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml gained a [features] table — CONFIGS.md/ERRORS.md must now be \
         re-verified under every feature combination"
    );
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
    assert!(
        !src.contains("feature ="),
        "src/lib.rs gained a cfg(feature = ...) branch — extend the matrix"
    );
    println!("no [features]; exactly one build configuration");
}

/// Runs under every profile/feature invocation, so the differential guarantee
/// is re-established for each one rather than only for `--release` default.
#[test]
fn differential_sweep_under_active_configuration() {
    let l = Libs::load();
    println!(
        "configuration: debug_assertions={}, overflow_checks(implied)={}",
        cfg!(debug_assertions),
        cfg!(debug_assertions)
    );

    // Full exhaustive sweep of the well-defined domain.
    for x in -16..=8223 {
        l.assert_same(x, "feature_combos/exhaustive");
    }
    // Plus fixed-seed fuzz.
    let mut rng = Rng::new(0xFEA7_0BEE);
    for _ in 0..50_000 {
        let x = rng.range_i32(-16, 8223);
        l.assert_same(x, "feature_combos/fuzz");
    }
    println!("exhaustive [-16, 8223] + 50000 fuzz draws matched under this configuration");
}
