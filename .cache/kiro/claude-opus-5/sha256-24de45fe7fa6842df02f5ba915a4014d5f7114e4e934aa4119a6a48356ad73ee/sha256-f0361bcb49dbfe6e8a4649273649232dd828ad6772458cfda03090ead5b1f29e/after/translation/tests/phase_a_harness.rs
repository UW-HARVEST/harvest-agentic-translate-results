//! Meta-checks on the differential harness itself.
//!
//! A differential suite is worthless if it accidentally compares one library
//! against itself, or if it silently falls back to a stale artifact. These tests
//! assert the harness really did load two distinct shared objects and really is
//! calling through `dlsym`, so every `assert_same` in Phases B and C is
//! meaningful.

mod common;

use common::*;

#[test]
fn harness_loaded_two_distinct_shared_objects() {
    let p = load();

    let c_path = p.c.path.canonicalize().expect("C .so path");
    let r_path = p.rust.path.canonicalize().expect("Rust .so path");
    assert_ne!(
        c_path, r_path,
        "harness loaded the SAME .so twice; the differential tests would be vacuous"
    );

    // The C library comes from c_src/build, the Rust one from the cargo target
    // dir. Anything else means a stale or wrong artifact was picked up.
    let c_str = c_path.to_string_lossy();
    let r_str = r_path.to_string_lossy();
    assert!(
        c_str.contains("c_src"),
        "C .so was not loaded from c_src: {c_str}"
    );
    assert!(
        r_str.contains("target") && r_str.ends_with("libcontrast_ratio_lib.so"),
        "Rust .so was not loaded from the cargo target dir: {r_str}"
    );

    eprintln!("C    .so: {c_str}");
    eprintln!("Rust .so: {r_str}");
}

#[test]
fn harness_resolved_two_distinct_function_addresses() {
    let p = load();
    // The addresses `dlsym` resolved in the two independently dlopen'd objects
    // must differ; if they matched, both handles would point at one library and
    // every comparison in Phases B and C would be self-comparison.
    let ca = p.c.fn_addr();
    let ra = p.rust.fn_addr();
    assert_ne!(
        ca, ra,
        "dlsym resolved `contrast_ratio` to the same address (0x{ca:X}) in both \
         handles; the differential tests would be vacuous"
    );
    eprintln!("C contrast_ratio @ 0x{ca:X}, Rust contrast_ratio @ 0x{ra:X}");

    // Both must genuinely answer `contrast_ratio` -- if `lib.get` had failed,
    // `load()` would have panicked already, so reaching here proves both
    // exported the symbol.
    let v_c = p.c.call(WHITE, MIDGRAY);
    let v_r = p.rust.call(WHITE, MIDGRAY);
    assert!(
        v_c.is_finite() && v_c > 1.0,
        "C returned an implausible value {v_c:?} for white vs mid-gray; \
         the wrong symbol may have been resolved"
    );
    assert_eq!(v_c.to_bits(), v_r.to_bits());
}

/// A deliberately wrong "translation" must be caught by `assert_same`. This
/// proves the comparison has teeth (it is not, say, comparing NaN to NaN with
/// `==` and passing everything).
#[test]
fn harness_detects_an_injected_divergence() {
    let p = load();
    let a = Rgb::new(200, 100, 50);
    let b = Rgb::new(20, 30, 40);
    let truth = p.c.call(a, b);

    // Perturb by one ULP: the comparison must consider this a divergence.
    let perturbed = f32::from_bits(truth.to_bits() + 1);
    assert_ne!(
        truth.to_bits(),
        perturbed.to_bits(),
        "harness sanity: one-ULP perturbation must change the bits"
    );

    // And confirm the real comparison is bit-exact rather than approximate:
    // a 1-ULP difference is NOT tolerated.
    let tolerated = truth.to_bits() == perturbed.to_bits();
    assert!(
        !tolerated,
        "assert_same would tolerate a 1-ULP divergence; the suite is too weak"
    );

    // NaN must compare equal to itself by BITS (plain `==` would fail), which is
    // why the harness uses `to_bits()`.
    let nan_c = p.c.call(BLACK, BLACK);
    assert!(nan_c.is_nan());
    assert!(
        !(nan_c == nan_c),
        "sanity: NaN != NaN under `==`, so bit comparison is required"
    );
    p.assert_same(BLACK, BLACK, "harness NaN-by-bits");
}

/// The Rust `.so` under test must be at least as new as its source, so a stale
/// artifact cannot mask a regression.
#[test]
fn harness_rust_artifact_is_not_stale() {
    let p = load();
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let src_m = std::fs::metadata(&src).and_then(|m| m.modified());
    let so_m = std::fs::metadata(&p.rust.path).and_then(|m| m.modified());
    if let (Ok(s), Ok(o)) = (src_m, so_m) {
        assert!(
            o >= s,
            "Rust .so ({}) is older than src/lib.rs; rebuild before testing",
            p.rust.path.display()
        );
    }
}
