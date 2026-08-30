//! Phase D — feature-combination guard.
//!
//! The crate declares no Cargo features, so the feature power-set has exactly one
//! member (the default = empty configuration) and `--no-default-features`,
//! `--all-features` and the default build are the same library. That claim is
//! asserted here so that adding a feature later forces CONFIGS.md/SYMBOLS.md to be
//! revisited instead of silently leaving code paths unverified.

/// Parses `[features]` out of Cargo.toml and fails if any feature exists.
#[test]
fn no_cargo_features_declared() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("Cargo.toml unreadable");

    let mut in_features = false;
    let mut features = Vec::new();
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_features = line == "[features]";
            continue;
        }
        if in_features {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((name, _)) = line.split_once('=') {
                features.push(name.trim().to_string());
            }
        }
    }

    assert!(
        features.is_empty(),
        "Cargo features were added ({features:?}) but the verification matrix in \
         CONFIGS.md / SYMBOLS.md still assumes a single default configuration. \
         Re-run Phases B and C for every combination and update those documents."
    );
}

/// `default-features` must not be implied by an (absent) feature table.
#[test]
fn default_feature_set_is_empty() {
    // If a `default` feature existed, cfg!(feature = "default") could never be
    // true here anyway; instead assert no feature-gated code exists in the crate.
    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("src/lib.rs unreadable");
    let gated: Vec<&str> = lib
        .lines()
        .filter(|l| l.contains("cfg(feature") || l.contains("cfg_attr(feature"))
        .collect();
    assert!(
        gated.is_empty(),
        "src/lib.rs contains feature-gated code, so more than one configuration \
         must be verified: {gated:?}"
    );
}
