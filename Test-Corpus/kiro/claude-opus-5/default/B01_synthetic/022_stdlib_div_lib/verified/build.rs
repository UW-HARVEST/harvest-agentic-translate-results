//! Records the resolved feature set so the integration tests can rebuild the
//! `cdylib` with exactly the same configuration they were themselves built with.
//!
//! `cargo test` does not refresh a `cdylib` artifact, so the differential tests
//! build it themselves; without this they could not know which features to pass.

fn main() {
    // Cargo sets one CARGO_FEATURE_<NAME> variable per enabled feature, with the
    // name upper-cased and non-alphanumerics replaced by `_`. Feature names are
    // lower-cased again here; a feature whose real name contains `_` and one
    // that contains `-` are indistinguishable at this point, so prefer `-`,
    // which is what Cargo itself accepts on the command line either way.
    let mut features: Vec<String> = std::env::vars()
        .filter_map(|(k, _)| k.strip_prefix("CARGO_FEATURE_").map(str::to_owned))
        .map(|f| f.to_lowercase())
        .collect();
    features.sort();
    println!("cargo:rustc-env=DRIVER_ACTIVE_FEATURES={}", features.join(","));
    println!("cargo:rerun-if-changed=build.rs");
}
