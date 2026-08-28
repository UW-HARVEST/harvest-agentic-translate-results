//! Phase D — feature-matrix completeness.
//!
//! The completion gate requires every phase to hold under EVERY feature
//! combination. This test mechanically proves what that matrix is, so the claim
//! "there is only one configuration" cannot silently rot: if anyone adds a
//! `[features]` table or an optional dependency, this test fails and tells the
//! next person to extend the matrix.

use std::path::PathBuf;

fn manifest_text() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Strip `#` comments and collect the top-level table headers plus their bodies.
fn sections(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut current = String::new();
    for raw in text.lines() {
        let line = match raw.find('#') {
            Some(i) => &raw[..i],
            None => raw,
        }
        .trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line.trim_matches(|c| c == '[' || c == ']').to_string();
            out.push((current.clone(), Vec::new()));
        } else if let Some(last) = out.last_mut() {
            last.1.push(line.to_string());
        } else {
            let _ = &current;
        }
    }
    out
}

#[test]
fn feature_matrix_is_exhaustive() {
    let text = manifest_text();
    let secs = sections(&text);

    // 1. No [features] table.
    let feature_secs: Vec<&(String, Vec<String>)> =
        secs.iter().filter(|(n, _)| n == "features").collect();
    assert!(
        feature_secs.is_empty(),
        "Cargo.toml now declares a [features] table: {:?}\n\
         Phases B and C must be re-run for every combination; update CONFIGS.md \
         and the verification script (verify.sh).",
        feature_secs.iter().map(|(_, b)| b).collect::<Vec<_>>()
    );

    // 2. No optional dependencies (they implicitly create features).
    for (name, body) in &secs {
        if name.contains("dependencies") {
            for line in body {
                assert!(
                    !line.replace(' ', "").contains("optional=true"),
                    "optional dependency in [{name}] creates an implicit feature: {line}"
                );
            }
        }
    }

    // 3. The library target must stay a cdylib, or these differential tests
    //    would no longer be exercising the exported C ABI.
    let lib = secs
        .iter()
        .find(|(n, _)| n == "lib")
        .expect("Cargo.toml must declare a [lib] section");
    assert!(
        lib.1.iter().any(|l| l.contains("crate-type") && l.contains("cdylib")),
        "[lib] must keep crate-type = [\"cdylib\"]; got {:?}",
        lib.1
    );
    assert!(
        lib.1.iter().any(|l| l.contains("premultiply_lib")),
        "[lib] name changed; update tests/common/mod.rs::rust_so_path"
    );
}

/// Guard against the crate acquiring `#[cfg(feature = ...)]` code paths without
/// a corresponding `[features]` table entry (which would be a silent
/// mis-configuration rather than a compile error).
#[test]
fn source_has_no_feature_gated_code() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let src = std::fs::read_to_string(&p).expect("read src/lib.rs");
    let mut offenders = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let l = line.replace(' ', "");
        if l.contains("feature=\"") && !line.trim_start().starts_with("//") {
            offenders.push(format!("{}:{}: {}", p.display(), i + 1, line.trim()));
        }
    }
    assert!(
        offenders.is_empty(),
        "feature-gated code found but Cargo.toml declares no [features]:\n{}",
        offenders.join("\n")
    );
}
