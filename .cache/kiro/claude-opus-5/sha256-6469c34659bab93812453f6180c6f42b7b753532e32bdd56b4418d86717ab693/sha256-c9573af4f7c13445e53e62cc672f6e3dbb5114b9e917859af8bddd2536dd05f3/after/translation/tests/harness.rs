//! Harness self-checks.
//!
//! These guard against the failure mode where the "differential" suite silently
//! compares a library against itself, which would make every other test in the
//! suite vacuous.

mod common;

use common::*;

#[test]
fn harness_loads_two_distinct_shared_objects() {
    let c = c_so_path().canonicalize().expect("C .so path");
    let r = rust_so_path().canonicalize().expect("Rust .so path");
    assert_ne!(c, r, "the C and Rust .so paths must differ");
    println!("C    : {}", c.display());
    println!("Rust : {}", r.display());

    // Different files, and the Rust one really is the Rust cdylib.
    assert!(
        c.to_string_lossy().contains("c_src"),
        "C .so should live under c_src/build, got {}",
        c.display()
    );
    assert!(
        r.file_name().unwrap().to_string_lossy() == "libunderhanded_c_nuke_lib.so",
        "unexpected Rust .so name: {}",
        r.display()
    );

    let cb = std::fs::read(&c).unwrap();
    let rb = std::fs::read(&r).unwrap();
    assert_ne!(cb, rb, "the two .so files must not be byte-identical");
}

#[test]
fn harness_both_libraries_export_both_symbols() {
    // `Lib::open` panics if either symbol is absent, so simply opening both is
    // the assertion. Then check the resolved addresses come from different
    // objects, i.e. RTLD_LOCAL kept the two `spectral_contrast` definitions apart.
    let p = pair();
    let c_addr = p.c.spectral_contrast as usize;
    let r_addr = p.rust.spectral_contrast as usize;
    assert_ne!(
        c_addr, r_addr,
        "both libraries resolved `spectral_contrast` to the same address — the \
         differential test would be comparing one implementation with itself"
    );
    let c_match = p.c.r#match as usize;
    let r_match = p.rust.r#match as usize;
    assert_ne!(c_match, r_match, "`match` resolved to the same address in both libraries");
}

#[test]
fn harness_detects_a_planted_divergence() {
    // Prove the comparison logic can fail: feed the two sides *different* inputs
    // and confirm the bit comparison rejects it.
    let p = pair();
    let mut a = vec![1.0f32, 2.0, 3.0, 4.0];
    let mut b = vec![1.0f32, 2.0, 3.0, 4.0];
    let mut a2 = vec![1.0f32, 2.0, 3.0, 4.25];
    let mut b2 = vec![1.0f32, 2.0, 3.0, 4.0];
    let x = unsafe { (p.c.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), 4) };
    let y = unsafe { (p.rust.spectral_contrast)(a2.as_mut_ptr(), b2.as_mut_ptr(), 4) };
    assert_ne!(
        x.to_bits(),
        y.to_bits(),
        "the harness must be able to observe a difference at all"
    );
}
