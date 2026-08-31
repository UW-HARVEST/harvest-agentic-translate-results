//! Differential test of the public API (`void driver(int)`).
//!
//! Every call goes through the exported symbol of a `dlopen`ed shared object —
//! never through the Rust crate directly — so the C library and the Rust
//! library are exercised through an identical interface.

mod common;

use common::Drivers;

/// A deterministic spread of inputs: boundaries, small magnitudes, powers of
/// two, the values around which `2*x + 300` wraps, and a pseudo-random sweep.
fn test_inputs() -> Vec<i32> {
    let mut inputs: Vec<i32> = Vec::new();

    // Small magnitudes around zero.
    for x in -300..=300 {
        inputs.push(x);
    }

    // Extremes and near-extremes.
    inputs.extend_from_slice(&[
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 149,
        i32::MIN + 150,
        i32::MIN + 151,
        i32::MIN / 2,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 149,
        i32::MAX - 150,
        i32::MAX - 151,
        i32::MAX / 2,
    ]);

    // Exactly where `2*x` and `2*x + 300` cross the signed 32-bit boundaries.
    inputs.extend_from_slice(&[
        1073741823,  // 2*x == i32::MAX - 1
        1073741824,  // 2*x overflows
        1073741673,  // 2*x + 300 == i32::MAX - 1
        1073741674,  // 2*x + 300 overflows
        -1073741824, // 2*x == i32::MIN
        -1073741825,
        -1073741974, // 2*x + 300 == i32::MIN
        -1073741975,
    ]);

    // Powers of two and their negations / neighbours.
    for bit in 0..31 {
        let p = 1i32 << bit;
        inputs.push(p);
        inputs.push(-p);
        inputs.push(p - 1);
        inputs.push(p.wrapping_neg().wrapping_sub(1));
    }

    // Deterministic pseudo-random sweep (xorshift32) over the whole range.
    let mut state: u32 = 0x1234_5678;
    for _ in 0..2000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        inputs.push(state as i32);
    }

    inputs
}

#[test]
fn driver_output_matches_c_byte_for_byte() {
    let drivers = Drivers::load();

    let mut mismatches: Vec<String> = Vec::new();
    for x in test_inputs() {
        let c = drivers.c_output(x);
        let rust = drivers.rust_output(x);
        if c != rust {
            mismatches.push(format!(
                "driver({x}): C = {:?} ({c:?}), Rust = {:?} ({rust:?})",
                String::from_utf8_lossy(&c),
                String::from_utf8_lossy(&rust),
            ));
            if mismatches.len() >= 20 {
                break;
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} mismatch(es) between the C and Rust `driver`:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Repeated calls must keep producing identical output, and the captured stream
/// must actually contain something (guards against the capture harness silently
/// swallowing all output and making the comparison vacuous).
#[test]
fn capture_harness_is_not_vacuous() {
    let drivers = Drivers::load();

    let c = drivers.c_output(21);
    let rust = drivers.rust_output(21);
    assert_eq!(c, b"342\n", "unexpected C output for driver(21)");
    assert_eq!(rust, c);

    // Sequential calls: state must not leak between invocations.
    for _ in 0..3 {
        assert_eq!(drivers.c_output(-150), b"0\n");
        assert_eq!(drivers.rust_output(-150), b"0\n");
    }
}
