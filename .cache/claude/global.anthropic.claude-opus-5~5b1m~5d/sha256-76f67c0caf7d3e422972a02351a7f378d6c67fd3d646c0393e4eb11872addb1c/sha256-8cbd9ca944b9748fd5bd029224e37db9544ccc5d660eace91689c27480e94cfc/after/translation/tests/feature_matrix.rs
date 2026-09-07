//! Phase D — guard the claim that there is exactly ONE feature combination.
//!
//! If anybody ever adds a `[features]` table to `Cargo.toml`, this test fails
//! and forces Phases B–C to be re-run for the new combinations (see
//! `run_all_configs.sh`).

#[test]
fn cargo_toml_declares_no_features() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    let has_features = manifest
        .lines()
        .map(str::trim)
        .any(|l| l == "[features]" || l.starts_with("[features."));
    assert!(
        !has_features,
        "Cargo.toml now declares features — re-run every phase for each feature \
         combination (see run_all_configs.sh) and update SYMBOLS.md"
    );
}

#[test]
fn no_default_features_is_the_same_configuration() {
    // With no features declared, `--no-default-features` cannot change any code
    // path. Assert there is no `cfg(feature = ...)` in the crate source either,
    // so the claim holds at the source level too.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("read src/lib.rs");
    assert!(
        !src.contains("feature ="),
        "src/lib.rs contains a cfg(feature = ...) gate; the feature matrix is not trivial"
    );
}
