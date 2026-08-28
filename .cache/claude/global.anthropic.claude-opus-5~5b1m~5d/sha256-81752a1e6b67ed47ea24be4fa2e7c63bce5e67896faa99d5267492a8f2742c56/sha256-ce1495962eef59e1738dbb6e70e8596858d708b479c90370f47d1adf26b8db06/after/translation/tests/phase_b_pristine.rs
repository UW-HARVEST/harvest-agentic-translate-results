//! Pristine-global-state differential tests.
//!
//! These live in their OWN test binary (= own process) and NEVER write to the
//! exported `matrix` object, so the *as-linked initializer* of that global is
//! observable and comparable between the two `.so`s.
//!
//! Why this file exists: the other test files reset `matrix` to a constant the
//! test itself owns, which would mask a wrong initializer compiled into the Rust
//! `.so`. `mutation_check.sh` proved that blind spot was real (the mutant
//! `matrix[2][3] = 0xD5` survived until this file was added).

mod common;

use common::{load_pristine, DEFAULT_MATRIX};

#[test]
fn p1_matrix_initializers_are_byte_identical() {
    let p = load_pristine();
    let (c_bytes, rs_bytes) = common::pristine_matrix_bytes();
    assert_eq!(
        c_bytes, rs_bytes,
        "the as-linked initializer of the exported `matrix` object differs:\n\
         C   = {c_bytes:02x?}\n\
         Rust= {rs_bytes:02x?}"
    );
    // and the live objects still agree
    assert_eq!(p.c.matrix_bytes(), p.rs.matrix_bytes());
}

#[test]
fn p2_matrix_initializer_matches_the_c_source_literal() {
    let _p = load_pristine();
    let (c_bytes, rs_bytes) = common::pristine_matrix_bytes();
    let mut want = [0u8; 48];
    for (i, v) in DEFAULT_MATRIX.iter().enumerate() {
        want[i * 4..i * 4 + 4].copy_from_slice(&v.to_ne_bytes());
    }
    assert_eq!(
        c_bytes, want,
        "the C `matrix` initializer is not the literal from c_src/src/lib.c:28"
    );
    assert_eq!(
        rs_bytes, want,
        "the Rust `matrix` initializer is not the literal from c_src/src/lib.c:28"
    );
}

#[test]
fn p3_checksum_and_matrixsum_on_the_untouched_globals() {
    let p = load_pristine();
    // Computed with nothing ever having written to `matrix`.
    let c = p.c.calculate_matrix_checksum();
    let r = p.rs.calculate_matrix_checksum();
    assert_eq!(c, r, "checksum of the untouched global diverged");
    assert_eq!(c, 916, "0x01+..+0xD4 == 916");

    for (a, b, cc, d) in [
        (0, 0, 0, 0),
        (1, 0, 0, 0),
        (0, 1, 0, 0),
        (0, 0, 1, 0),
        (0, 0, 0, 1),
        (1, 2, 3, 4),
        (-1, -1, -1, -1),
        (i32::MIN, i32::MAX, 1, -1),
    ] {
        assert_eq!(
            p.c.matrixsum(a, b, cc, d),
            p.rs.matrixsum(a, b, cc, d),
            "matrixsum({a},{b},{cc},{d}) on untouched globals diverged"
        );
    }
    // still untouched
    assert_eq!(p.c.matrix_read(), DEFAULT_MATRIX);
    assert_eq!(p.rs.matrix_read(), DEFAULT_MATRIX);
}
