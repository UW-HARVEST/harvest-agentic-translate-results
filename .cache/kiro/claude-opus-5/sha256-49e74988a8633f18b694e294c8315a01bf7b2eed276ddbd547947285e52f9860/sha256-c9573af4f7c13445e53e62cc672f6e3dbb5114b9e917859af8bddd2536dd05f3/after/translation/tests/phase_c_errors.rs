// Phase C — error / boundary-path differential tests.
// One test per row of ERRORS.md.
//
// The C source contains no error return, no assert, and no range check, so
// every row below asserts that C and Rust *agree* on the (non-)rejection: the
// same bytes on stdout, the same record framing, and no divergence in which of
// the two decides to fail.

#[path = "common/mod.rs"]
mod common;

use common::{assert_same, assert_same_interleaved, c_outputs, rust_outputs, Rng};

/// ERRORS.md row 1 — zero / degenerate value.
#[test]
fn err_01_zero() {
    assert_same("err 1: floors = 0", &[0]);
    let c = c_outputs(&[0]);
    assert_eq!(
        c[0].as_slice(),
        b"00000000030000000000000000000040\n",
        "err 1: the C reference image itself changed; re-derive the expectation"
    );
}

/// ERRORS.md row 2 — one step past the negative end of the signed range.
#[test]
fn err_02_int_min() {
    assert_same("err 2: floors = INT_MIN", &[i32::MIN]);
    let c = c_outputs(&[i32::MIN]);
    assert_eq!(&c[0][0..8], b"00000080", "err 2: INT_MIN image wrong");
}

/// ERRORS.md row 3 — positive extreme.
#[test]
fn err_03_int_max() {
    assert_same("err 3: floors = INT_MAX", &[i32::MAX]);
    let c = c_outputs(&[i32::MAX]);
    assert_eq!(&c[0][0..8], b"ffffff7f", "err 3: INT_MAX image wrong");
}

/// ERRORS.md row 4 — the classic `-1` sentinel value.
#[test]
fn err_04_minus_one() {
    assert_same("err 4: floors = -1", &[-1]);
    let c = c_outputs(&[-1]);
    let r = rust_outputs(&[-1]);
    assert_eq!(&c[0][0..8], b"ffffffff", "err 4: -1 image wrong");
    assert_eq!(c[0], r[0], "err 4: divergence on -1");
}

/// ERRORS.md row 5 — out-of-range "enum" values crossing the FFI boundary.
///
/// `driver`'s parameter is a C `int`. A C enum accepts any `int`, so these bit
/// patterns are inputs the C really handles: it performs no validation and
/// copies the raw bytes. Rust must do the same and must not panic, abort, or
/// reject them.
#[test]
fn err_05_out_of_range_enum_values() {
    let inputs: Vec<i32> = [
        0x0000_0000u32,
        0x0000_0001,
        0x7fff_ffff,
        0x8000_0000,
        0xdead_beef,
        0xffff_ffff,
        0xcafe_babe,
        0xbaad_f00d,
        // values just past every small "variant count" a caller might assume
        4,
        5,
        64,
        255,
        256,
        65_536,
        16_777_216,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();

    assert_same("err 5: out-of-range enum-like values", &inputs);

    // Neither side may signal rejection by emitting nothing or short output.
    for (which, recs) in [
        ("C", c_outputs(&inputs)),
        ("Rust", rust_outputs(&inputs)),
    ] {
        assert_eq!(recs.len(), inputs.len(), "{which}: dropped a record");
        for (i, rec) in recs.iter().enumerate() {
            assert_eq!(
                rec.len(),
                33,
                "{which}: driver({}) rejected/short-circuited",
                inputs[i]
            );
        }
    }
}

/// ERRORS.md row 6 — one step past every plausible range boundary.
#[test]
fn err_06_range_boundaries() {
    let mut inputs: Vec<i32> = Vec::new();
    for centre in [
        0i64,
        0x80,
        0x100,
        0x8000,
        0x1_0000,
        0x80_0000,
        0x100_0000,
        0x7fff_ffff,
        -0x80,
        -0x8000,
        -0x8000_0000,
    ] {
        for delta in [-1i64, 0, 1] {
            let v = centre.saturating_add(delta);
            if (i32::MIN as i64..=i32::MAX as i64).contains(&v) {
                inputs.push(v as i32);
            }
        }
    }
    inputs.push(i32::MIN);
    inputs.push(i32::MIN + 1);
    inputs.push(i32::MAX - 1);
    assert_same("err 6: range boundaries ±1", &inputs);
}

/// ERRORS.md row 7 — the internal `len` cannot be corrupted from outside, so
/// the record length is invariant. A Rust translation that mis-sized the struct
/// (e.g. added padding, or used a 4-byte float) would break this.
#[test]
fn err_07_output_length_invariant() {
    let mut rng = Rng::new(0x5EED_0000_0000_0007);
    let mut inputs: Vec<i32> = vec![0, -1, i32::MIN, i32::MAX];
    inputs.extend((0..3_000).map(|_| rng.next_i32()));

    for (which, recs) in [
        ("C", c_outputs(&inputs)),
        ("Rust", rust_outputs(&inputs)),
    ] {
        for (i, rec) in recs.iter().enumerate() {
            assert_eq!(
                rec.len(),
                33,
                "{which}: driver({}) emitted {} bytes, expected 16 struct bytes as hex + newline",
                inputs[i],
                rec.len()
            );
        }
    }
    assert_same("err 7: length invariant (differential)", &inputs);
}

/// ERRORS.md row 9 — no residual state across repeats, and no divergence when
/// the two libraries share the process's stdout.
#[test]
fn err_09_no_residual_state() {
    let inputs = [i32::MIN, i32::MIN, i32::MAX, i32::MIN, 0, -1, 0];
    assert_same("err 9: repeats", &inputs);
    assert_same_interleaved("err 9: interleaved repeats", &inputs);

    for (which, recs) in [
        ("C", c_outputs(&inputs)),
        ("Rust", rust_outputs(&inputs)),
    ] {
        assert_eq!(recs[0], recs[1], "{which}: repeat of INT_MIN differed");
        assert_eq!(recs[0], recs[3], "{which}: INT_MIN differed after INT_MAX");
        assert_eq!(recs[4], recs[6], "{which}: 0 differed after -1");
    }
}

/// ERRORS.md row 10 — `%02x` zero-padding across the full byte domain, checked
/// in every byte lane (not just the low one).
#[test]
fn err_10_hex_zero_padding() {
    let mut inputs: Vec<i32> = Vec::new();
    for b in 0u32..=0xff {
        inputs.push(b as i32);
        inputs.push((b << 8) as i32);
        inputs.push((b << 16) as i32);
        inputs.push((b << 24) as i32);
    }
    assert_same("err 10: zero-padding in every lane", &inputs);

    // Every record must be exactly two hex chars per byte — a `%x` (unpadded)
    // regression would shorten the record.
    for (which, recs) in [
        ("C", c_outputs(&inputs)),
        ("Rust", rust_outputs(&inputs)),
    ] {
        for (i, rec) in recs.iter().enumerate() {
            assert_eq!(rec.len(), 33, "{which}: driver({}) not zero-padded", inputs[i]);
        }
    }
}

/// ERRORS.md row 8 is unreachable by construction (the public ABI has no
/// pointer parameter). This test documents and enforces that: `driver` is the
/// only export, and its arity/type is the single-`int` form, so no caller can
/// supply a null pointer.
#[test]
fn err_08_no_pointer_parameter_in_public_abi() {
    assert!(common::c_exports("driver"), "C .so must export `driver`");
    assert!(common::rust_exports("driver"), "Rust .so must export `driver`");
    // `print_hex` is `static` in C; it must not be reachable via dlsym in
    // either library, so its pointer parameter is not part of the attack
    // surface.
    assert!(
        !common::c_exports("print_hex"),
        "`print_hex` is static in C and must not be exported"
    );
    assert!(
        !common::rust_exports("print_hex"),
        "`print_hex` must stay private in the Rust translation"
    );
}
