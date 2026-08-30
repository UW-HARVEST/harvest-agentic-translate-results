// Phase C — error-path / rejection differential tests.
//
// One test per row of ERRORS.md. The C library contains ZERO rejection branches
// (no `return <err>`, no error enum, no assert, no range/null check — see
// ERRORS.md for the mechanical grep), and `driver` returns `void`. The correct
// differential assertion for such an API is therefore "C and Rust produce the
// SAME observable result for the same hostile input" — identical stdout bytes,
// identical output length, neither aborting — rather than "same error code",
// since no error code exists to compare.
//
// Every test below constructs the exact condition named in its ERRORS.md row and
// calls BOTH `.so` files through their exported `driver` symbol.

mod common;
use common::*;

/// Asserts that both libraries "reject or accept" `x` identically, and reports
/// which it was, so a divergence in *acceptance* is caught too.
fn assert_same_disposition(row: &str, x: i32) {
    let c = run_c(x);
    let r = run_rust(x);
    assert_eq!(
        c,
        r,
        "[{row}] C and Rust disagree for driver({x}) / 0x{:08x}:\n  C   : {:?}\n  Rust: {:?}",
        x as u32,
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    // Both must ACCEPT (the C never rejects): a full 33-byte record, not an
    // early return, not an empty line.
    assert_eq!(
        c.len(),
        33,
        "[{row}] the C accepts every int, so driver({x}) must emit a full record"
    );
    assert!(
        !r.is_empty(),
        "[{row}] Rust returned no output for driver({x}) while C emitted a record"
    );
    check_shape(x, &c);
    check_shape(x, &r);
}

// ---------------------------------------------------------------- E1
fn err_e1_zero() {
    assert_same_disposition("E1", 0);
    // Explicit sentinel-shape check: no rejection, low word is all zeroes.
    assert_eq!(run_c(0), run_rust(0));
    assert_eq!(&run_rust(0)[..8], b"00000000");
}

// ---------------------------------------------------------------- E2
fn err_e2_negative_one() {
    assert_same_disposition("E2", -1);
    assert_eq!(&run_rust(-1)[..8], b"ffffffff");
    assert_eq!(&run_c(-1)[..8], b"ffffffff");
}

// ---------------------------------------------------------------- E3
fn err_e3_int_min() {
    assert_same_disposition("E3", i32::MIN);
    assert_eq!(&run_c(i32::MIN)[..8], b"00000080");
    assert_eq!(&run_rust(i32::MIN)[..8], b"00000080");
}

// ---------------------------------------------------------------- E4
fn err_e4_int_max() {
    assert_same_disposition("E4", i32::MAX);
    assert_eq!(&run_c(i32::MAX)[..8], b"ffffff7f");
    assert_eq!(&run_rust(i32::MAX)[..8], b"ffffff7f");
}

// ---------------------------------------------------------------- E5
fn err_e5_one_step_inside_extremes() {
    for x in [i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MAX - 1, i32::MAX] {
        assert_same_disposition("E5", x);
    }
}

// ---------------------------------------------------------------- E6
fn err_e6_out_of_range_enum_like_values() {
    // `driver`'s parameter is a plain `int`, so a C enum passed across this FFI
    // boundary has no valid-variant set at all: every one of these is an
    // "out-of-range enum value" and the C must handle each without rejecting.
    let probes: [i32; 14] = [
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        255,
        256,
        65_536,
        0x7FFF_FFFF,
        0x8000_0000u32 as i32,
        0xDEAD_BEEFu32 as i32,
        i32::MIN,
    ];
    for x in probes {
        assert_same_disposition("E6", x);
    }
}

// ---------------------------------------------------------------- E7
fn err_e7_unsigned_overflow_values() {
    // Unsigned values above INT_MAX handed to a signed `int` parameter.
    for u in [
        0x8000_0000u32,
        0x8000_0001,
        0xFFFF_FFFF,
        0xFFFF_FFFE,
        0xC000_0000,
    ] {
        let x = u as i32;
        assert_same_disposition("E7", x);
        // The low four output bytes must be the unsigned pattern verbatim.
        let hex: String = u.to_le_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(&run_c(x)[..8], hex.as_bytes());
        assert_eq!(&run_rust(x)[..8], hex.as_bytes());
    }
}

// ---------------------------------------------------------------- E8
fn err_e8_len_is_always_16_no_empty_loop() {
    // print_hex's `len <= 0` degenerate branch (which would print only "\n")
    // is unreachable from the public API: driver always passes
    // sizeof(house) == 16. Verified as an invariant over many inputs, for both
    // libraries: never a bare newline, always exactly 16 hex-encoded bytes.
    let mut rng = Rng::new(SEED ^ 0xE8);
    for _ in 0..256 {
        let x = rng.next_i32();
        for (name, out) in [("C", run_c(x)), ("Rust", run_rust(x))] {
            assert_ne!(out, b"\n".to_vec(), "[E8] {name} took the empty-loop path");
            assert_eq!(
                out.len(),
                33,
                "[E8] {name} printed {} bytes for driver({x}); len must always be 16",
                out.len()
            );
            assert_eq!((out.len() - 1) / 2, 16, "[E8] hex byte count must be 16");
        }
    }
}

// ---------------------------------------------------------------- E9
fn err_e9_print_hex_not_reachable_externally() {
    // The only pointer parameter in the library belongs to `print_hex`, which is
    // `static`. A NULL cannot be supplied by any external caller because the
    // symbol is not exported — assert that for BOTH objects, which is what makes
    // the "null pointer" boundary vacuous rather than untested.
    for so in [c_so_path(), rust_so_path()] {
        let syms = dynamic_symbols(&so);
        assert!(
            !syms.iter().any(|s| s == "print_hex"),
            "print_hex must not be dynamically reachable in {:?}, got {syms:?}",
            so
        );
    }
    // And confirm dlsym genuinely fails for it in both objects.
    unsafe {
        for path in [c_so_path(), rust_so_path()] {
            let lib = libloading::Library::new(&path).unwrap();
            let sym: Result<libloading::Symbol<unsafe extern "C" fn(*const u8, i32)>, _> =
                lib.get(b"print_hex\0");
            assert!(
                sym.is_err(),
                "dlsym unexpectedly resolved print_hex in {:?}",
                path
            );
        }
    }
}

// ---------------------------------------------------------------- E10
fn err_e10_no_retained_state_between_calls() {
    // Probe for retained state that could make a later call behave differently:
    // feed a hostile sequence, then re-issue each value alone and require the
    // same bytes from both libraries.
    let hostile: Vec<i32> = vec![
        i32::MIN,
        i32::MAX,
        -1,
        0,
        0xFFFF_FFFFu32 as i32,
        0x8000_0000u32 as i32,
        1,
    ];

    let c_batch = run_c_seq(&hostile);
    let r_batch = run_rust_seq(&hostile);
    assert_eq!(c_batch, r_batch, "[E10] batch output diverged");

    // Batched output must equal the concatenation of the isolated calls.
    let mut c_indiv = Vec::new();
    let mut r_indiv = Vec::new();
    for &x in &hostile {
        c_indiv.extend(run_c(x));
        r_indiv.extend(run_rust(x));
    }
    assert_eq!(
        c_batch, c_indiv,
        "[E10] C retained state across calls (batch != individual)"
    );
    assert_eq!(
        r_batch, r_indiv,
        "[E10] Rust retained state across calls (batch != individual)"
    );
}

// ------------------------------------------------ generic FFI boundary sweep
fn err_generic_boundary_sweep_full_int_edges() {
    // Every value within 2 of each signed/unsigned boundary, plus each byte-lane
    // boundary, driven through both libraries.
    let mut xs: Vec<i32> = Vec::new();
    for anchor in [
        0i64,
        1,
        -1,
        i32::MIN as i64,
        i32::MAX as i64,
        i16::MIN as i64,
        i16::MAX as i64,
        i8::MIN as i64,
        i8::MAX as i64,
        u8::MAX as i64,
        u16::MAX as i64,
        0x0100_0000,
        0x00FF_FFFF,
    ] {
        for d in -2i64..=2 {
            let v = anchor + d;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                xs.push(v as i32);
            }
        }
    }
    xs.sort_unstable();
    xs.dedup();
    for x in xs {
        assert_same_disposition("generic-boundary", x);
    }
}

// ---------------------------------------------------------------- runner
fn main() {
    let cases: &[(&str, fn())] = &[
        ("err_e1_zero", err_e1_zero as fn()),
        ("err_e2_negative_one", err_e2_negative_one as fn()),
        ("err_e3_int_min", err_e3_int_min as fn()),
        ("err_e4_int_max", err_e4_int_max as fn()),
        ("err_e5_one_step_inside_extremes", err_e5_one_step_inside_extremes as fn()),
        ("err_e6_out_of_range_enum_like_values", err_e6_out_of_range_enum_like_values as fn()),
        ("err_e7_unsigned_overflow_values", err_e7_unsigned_overflow_values as fn()),
        ("err_e8_len_is_always_16_no_empty_loop", err_e8_len_is_always_16_no_empty_loop as fn()),
        ("err_e9_print_hex_not_reachable_externally", err_e9_print_hex_not_reachable_externally as fn()),
        ("err_e10_no_retained_state_between_calls", err_e10_no_retained_state_between_calls as fn()),
        ("err_generic_boundary_sweep_full_int_edges", err_generic_boundary_sweep_full_int_edges as fn()),
    ];
    run_suite("phase_c_errors", cases);
}
