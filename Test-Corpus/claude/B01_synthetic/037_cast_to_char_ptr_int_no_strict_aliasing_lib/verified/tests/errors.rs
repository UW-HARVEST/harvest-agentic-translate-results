//! Phase C -- error-path / boundary differential tests.
//!
//! One test per row of `ERRORS.md`. `driver` has no error return value (it is
//! `void` and the C contains zero rejection constructs), so "same error" means
//! "the same observable reaction to the invalid / extreme input": both `.so`
//! files must emit the identical byte stream and neither may crash, sign-extend,
//! truncate, or skip the trailing newline.

mod harness;

use harness::*;
use std::ffi::c_int;

const SEED: u64 = 0xdead_beef_1234_5678;

// ---------------------------------------------------------------- E1 + E2
/// The only guard in the library is `i < len` in `print_hex`. `print_hex` is
/// `static`, so `len == 0` / `len < 0` cannot be reached from outside: `driver`
/// always passes `sizeof(raw) == sizeof(int)`. The differential assertion is
/// that the guard never degenerates on either side -- every call performs
/// exactly 4 iterations and still emits the trailing `"\n"`.
#[test]
fn e1_loop_bound_never_degenerates() {
    let mut rng = Rng::new(SEED ^ 1);
    let mut xs: Vec<c_int> = vec![0, 1, -1, c_int::MAX, c_int::MIN, c_int::MIN + 1, c_int::MAX - 1];
    xs.extend((0..cases()).map(|_| rng.next_i32()));

    for &x in &xs {
        let c = c_out(x);
        let r = rust_out(x);
        assert_eq!(c, r, "E1/E2: divergence for {x:#010x}");
        // 4 iterations of `%02x` => 8 digits, plus the unconditional newline.
        assert_eq!(
            c.len(),
            9,
            "E1/E2: C emitted {} bytes for {x:#010x}; the loop bound degenerated",
            c.len()
        );
        assert_eq!(
            r.len(),
            9,
            "E1/E2: Rust emitted {} bytes for {x:#010x}; the loop bound degenerated",
            r.len()
        );
        assert_eq!(c[8], b'\n', "E1/E2: C dropped the trailing newline");
        assert_eq!(r[8], b'\n', "E1/E2: Rust dropped the trailing newline");
        assert_ne!(c, b"\n".to_vec(), "E1/E2: zero-iteration output leaked out");
    }
}

// ---------------------------------------------------------------- E3
/// `INT_MIN` -- the most negative value. The high byte is `0x80`; a signed
/// `char` deref plus sign extension would print `ffffff80` instead of `80`.
#[test]
fn e3_int_min_no_sign_extension() {
    let x = c_int::MIN;
    let c = c_out(x);
    let r = rust_out(x);
    assert_eq!(c, r, "E3: divergence for INT_MIN");
    assert_eq!(
        c,
        b"00000080\n".to_vec(),
        "E3: unexpected C ground truth for INT_MIN"
    );
    assert!(
        !String::from_utf8_lossy(&r).contains("ffffff"),
        "E3: Rust sign-extended the 0x80 byte: {:?}",
        String::from_utf8_lossy(&r)
    );
    assert_same(x, "E3");
}

// ---------------------------------------------------------------- E4
#[test]
fn e4_minus_one_all_ff() {
    let c = c_out(-1);
    let r = rust_out(-1);
    assert_eq!(c, r, "E4: divergence for -1");
    assert_eq!(c, b"ffffffff\n".to_vec(), "E4: unexpected C ground truth for -1");
    assert_same(-1, "E4");
}

// ---------------------------------------------------------------- E5
#[test]
fn e5_int_max() {
    let x = c_int::MAX;
    let c = c_out(x);
    let r = rust_out(x);
    assert_eq!(c, r, "E5: divergence for INT_MAX");
    assert_eq!(
        c,
        b"ffffff7f\n".to_vec(),
        "E5: unexpected C ground truth for INT_MAX"
    );
    // One step past INT_MAX wraps to INT_MIN in the caller; both must agree.
    assert_same(c_int::MAX.wrapping_add(1), "E5");
}

// ---------------------------------------------------------------- E6
/// Every high-bit byte value (`0x80..=0xff`) in every one of the four byte
/// positions: the `unsigned char` -> `int` promotion at `printf("%02x", p[i])`.
#[test]
fn e6_every_high_bit_byte_in_every_position() {
    for position in 0..4usize {
        let xs: Vec<c_int> = (0x80u32..=0xff)
            .map(|v| {
                let mut b = [0u8; 4];
                b[position] = v as u8;
                c_int::from_ne_bytes(b)
            })
            .collect();
        for &x in &xs {
            let c = c_out(x);
            let r = rust_out(x);
            assert_eq!(
                c, r,
                "E6: divergence for high-bit byte at position {position}, value {x:#010x}"
            );
            assert_eq!(c.len(), 9, "E6: promotion produced more than two digits: {c:?}");
            assert_eq!(c, expected(x), "E6: unexpected C ground truth for {x:#010x}");
        }
        assert_same_batch(&xs, "E6");
    }

    // Also the saturated case: all four bytes in 0x80..=0xff simultaneously.
    let mut rng = Rng::new(SEED ^ 6);
    let xs: Vec<c_int> = (0..cases())
        .map(|_| {
            let mut b = [0u8; 4];
            for slot in b.iter_mut() {
                *slot = 0x80 | (rng.below(0x80) as u8);
            }
            c_int::from_ne_bytes(b)
        })
        .collect();
    for &x in &xs {
        assert_same(x, "E6");
    }
    assert_same_batch(&xs, "E6");
}

// ---------------------------------------------------------------- E7
/// `%02x` zero padding: bytes below `0x10` must produce two digits.
#[test]
fn e7_zero_padding() {
    let c = c_out(0);
    let r = rust_out(0);
    assert_eq!(c, r, "E7: divergence for 0");
    assert_eq!(c, b"00000000\n".to_vec(), "E7: unexpected C ground truth for 0");

    // Every byte value that needs padding, in every position.
    for position in 0..4usize {
        let xs: Vec<c_int> = (0x00u32..=0x0f)
            .map(|v| {
                let mut b = [0u8; 4];
                b[position] = v as u8;
                c_int::from_ne_bytes(b)
            })
            .collect();
        for &x in &xs {
            assert_same(x, "E7");
        }
        assert_same_batch(&xs, "E7");
    }

    // All four bytes below 0x10 at once: output must still be 8 digits, not 4.
    let mut rng = Rng::new(SEED ^ 7);
    let xs: Vec<c_int> = (0..cases())
        .map(|_| {
            let mut b = [0u8; 4];
            for slot in b.iter_mut() {
                *slot = rng.below(0x10) as u8;
            }
            c_int::from_ne_bytes(b)
        })
        .collect();
    for &x in &xs {
        let c = c_out(x);
        assert_eq!(c.len(), 9, "E7: padding lost for {x:#010x}: {c:?}");
        assert_same(x, "E7");
    }
    assert_same_batch(&xs, "E7");
}

// ---------------------------------------------------------------- E8
/// Generic FFI boundary: a value that is out of range for the declared `int`
/// parameter. The SysV ABI passes `int` in the low 32 bits of the argument
/// register, so a caller can deliver arbitrary garbage in the upper 32 bits --
/// the C reads only the low half, and the Rust must do exactly the same.
/// (This is the analogue of an out-of-range enum value for an API with no enum.)
#[test]
fn e8_upper_argument_bits_ignored() {
    let c64 = c_driver_u64();
    let r64 = rust_driver_u64();
    let cf = c_driver();

    let mut rng = Rng::new(SEED ^ 8);
    let mut values: Vec<u64> = vec![
        0x0000_0000_0000_0000,
        0xffff_ffff_0000_0000,
        0xffff_ffff_ffff_ffff,
        0xdead_beef_0000_0001,
        0x0000_0001_8000_0000,
        0x7fff_ffff_7fff_ffff,
        0x8000_0000_0000_0000,
        u64::MAX - 1,
    ];
    values.extend((0..cases()).map(|_| rng.next_u64()));

    for &v in &values {
        let c = capture_file(|| unsafe { c64(v) });
        let r = capture_file(|| unsafe { r64(v) });
        assert_eq!(
            c, r,
            "E8: divergence when the argument register holds {v:#018x}"
        );
        // The C must have looked at the low 32 bits only.
        let low = v as u32 as c_int;
        let reference = capture_file(|| unsafe { cf(low) });
        assert_eq!(
            c, reference,
            "E8: C reacted to the upper 32 bits of {v:#018x} (test-harness assumption)"
        );
        assert_eq!(
            r, reference,
            "E8: Rust did not ignore the upper 32 bits of {v:#018x}"
        );
        assert_eq!(c.len(), 9, "E8: unexpected output length for {v:#018x}");
    }
}

// ---------------------------------------------------------------- E9 + E10
/// Documented non-applicable rows, asserted mechanically so the claim cannot
/// rot: the public header declares exactly one function, taking one `int` and
/// returning `void` -- there is no pointer, no length and no enum to abuse, and
/// the library exports no other entry point that could take one.
#[test]
fn e9_e10_no_pointer_length_or_enum_in_public_api() {
    let header = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/include/driver.h"
    ))
    .expect("read driver.h");
    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//") && !l.starts_with('#') && !l.is_empty())
        .collect();
    assert_eq!(
        decls,
        vec!["void driver(int x);"],
        "the public API changed: ERRORS.md rows E9/E10 must be revisited"
    );
    assert!(!header.contains('*'), "E9: header now declares a pointer parameter");
    assert!(!header.contains("enum"), "E10: header now declares an enum");

    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/src/driver.c"
    ))
    .expect("read driver.c");
    assert!(
        !source.contains("assert")
            && !source.contains("return -1")
            && !source.contains("NULL")
            && !source.contains("errno"),
        "the C gained an error surface: ERRORS.md must be regenerated"
    );

    // And no second exported entry point exists on either side.
    assert!(
        unsafe { impls().c.get::<unsafe extern "C" fn()>(b"print_hex\0") }.is_err(),
        "print_hex is `static` in C and must not be exported"
    );
    assert!(
        unsafe { impls().rust.get::<unsafe extern "C" fn()>(b"print_hex\0") }.is_err(),
        "print_hex must not be exported from the Rust .so either"
    );
}
