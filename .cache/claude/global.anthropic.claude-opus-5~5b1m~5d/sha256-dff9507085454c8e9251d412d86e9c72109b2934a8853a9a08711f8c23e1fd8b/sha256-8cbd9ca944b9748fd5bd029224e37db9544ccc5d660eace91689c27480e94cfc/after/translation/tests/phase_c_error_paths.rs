//! Phase C — error-path / boundary differential tests.
//!
//! One test per row of `ERRORS.md`.  `driver` is `void` and has no error
//! returns, so "same rejection" means "same bytes on stdout and the same normal
//! (void) return with no crash" for each adversarial input.

mod common;

use common::*;
use std::ffi::c_char;

/// The twelve classification field names, in the order `driver` prints them.
const CLASS_FIELDS: [&str; 12] = [
    "alphanumeric",
    "alphabetic",
    "lowercase",
    "uppercase",
    "digit",
    "hexadecimal",
    "control",
    "graphical",
    "space",
    "blank",
    "printing",
    "punctuation",
];

/// Asserts every classification field of `out` reads `: 0`.
fn assert_all_classes_zero(out: &[u8], what: &str) {
    let text = show(out);
    for field in CLASS_FIELDS {
        let needle = format!("{field}: 0\\n");
        assert!(
            text.contains(&needle),
            "{what}: expected `{field}: 0` in {text}"
        );
    }
}

// --- Row 1: c == 0 (NUL) ---------------------------------------------------

fn err_nul_byte() {
    let c: c_char = 0;
    let cout = c_output(c);
    let rout = rust_output(c);
    assert!(
        cout == rout,
        "NUL diverged:\n  C   : {}\n  RUST: {}",
        show(&cout),
        show(&rout)
    );
    // Concrete expected result from ERRORS.md row 1.
    let text = show(&cout);
    assert!(text.contains("control: 2"), "expected iscntrl bit 2: {text}");
    assert!(text.contains("printing: 0"));
    assert!(text.contains("alphanumeric: 0"));
    // `printf("%c", 0)` writes a real NUL byte, twice (to lower / to upper).
    assert_eq!(
        cout.iter().filter(|&&b| b == 0).count(),
        2,
        "expected two NUL bytes in output: {text}"
    );
    assert_eq!(cout.iter().filter(|&&b| b == b'\n').count(), 14);
}

// --- Row 2: c == CHAR_MIN (-128) -------------------------------------------

fn err_char_min() {
    let c: c_char = -128;
    let cout = c_output(c);
    let rout = rust_output(c);
    assert!(
        cout == rout,
        "CHAR_MIN diverged:\n  C   : {}\n  RUST: {}",
        show(&cout),
        show(&rout)
    );
    let text = show(&cout);
    assert_all_classes_zero(&cout, "CHAR_MIN");
    // `%c` must emit byte 0x80 for both conversions.
    assert_eq!(
        cout.iter().filter(|&&b| b == 0x80).count(),
        2,
        "expected two 0x80 bytes: {text}"
    );
}

// --- Row 3: c == CHAR_MAX (127, DEL) ---------------------------------------

fn err_char_max() {
    let c: c_char = 127;
    let cout = c_output(c);
    let rout = rust_output(c);
    assert!(
        cout == rout,
        "CHAR_MAX diverged:\n  C   : {}\n  RUST: {}",
        show(&cout),
        show(&rout)
    );
    let text = show(&cout);
    assert!(text.contains("control: 2"), "DEL must be iscntrl: {text}");
    assert!(text.contains("printing: 0"), "DEL must not be isprint: {text}");
    assert!(text.contains("graphical: 0"), "DEL must not be isgraph: {text}");
    assert!(text.contains("punctuation: 0"));
    assert_eq!(cout.iter().filter(|&&b| b == 0x7f).count(), 2);
}

// --- Row 4: c == -1 (0xFF, the EOF alias) ----------------------------------

fn err_minus_one() {
    let c: c_char = -1;
    let cout = c_output(c);
    let rout = rust_output(c);
    assert!(
        cout == rout,
        "-1 diverged:\n  C   : {}\n  RUST: {}",
        show(&cout),
        show(&rout)
    );
    assert_all_classes_zero(&cout, "driver(-1)");
    assert_eq!(cout.iter().filter(|&&b| b == 0xff).count(), 2);
}

// --- Row 5: every negative char -------------------------------------------

fn err_all_negative_chars() {
    for i in -128i16..0 {
        let c = i as i8 as c_char;
        let cout = c_output(c);
        let rout = rust_output(c);
        assert!(
            cout == rout,
            "negative char {c} diverged:\n  C   : {}\n  RUST: {}",
            show(&cout),
            show(&rout)
        );
        // All twelve class predicates must be 0 in the "C" locale.
        let text = String::from_utf8_lossy(&cout).to_string();
        for line in text.lines() {
            if let Some((name, val)) = line.split_once(": ") {
                if name.starts_with("to ") {
                    continue;
                }
                assert_eq!(
                    val.trim(),
                    "0",
                    "negative char {c}: `{name}` should be 0 but C said `{val}`"
                );
            }
        }
        // %c must reproduce the original byte.
        assert_eq!(
            cout.iter().filter(|&&b| b == c as u8).count(),
            2,
            "negative char {c}: %c did not reproduce byte 0x{:02x}",
            c as u8
        );
    }
}

// --- Row 6: one step past the ASCII range ---------------------------------

fn err_one_past_ascii() {
    // 0x7F is the last positive char; 0x80 is one past it and wraps negative.
    for b in [0x7Eu8, 0x7F, 0x80, 0x81] {
        let c = b as c_char;
        let cout = c_output(c);
        let rout = rust_output(c);
        assert!(
            cout == rout,
            "byte 0x{b:02x} diverged:\n  C   : {}\n  RUST: {}",
            show(&cout),
            show(&rout)
        );
    }
    // 0x80 as an unsigned byte and -128 as a signed char are the same argument.
    assert_eq!(c_output(0x80u8 as c_char), c_output(-128i8 as c_char));
    assert_eq!(rust_output(0x80u8 as c_char), rust_output(-128i8 as c_char));
}

// --- Row 7: out-of-range int truncated at the FFI boundary -----------------

fn err_out_of_range_int_truncation() {
    // C `enum`s / small integer parameters accept any `int`; a value with no
    // valid `char` representation is truncated to the low 8 bits.  Both
    // implementations must agree on the truncated behaviour.
    let mut raw: Vec<i32> = vec![
        256,
        257,
        -129,
        -256,
        1000,
        -1000,
        65_536,
        65_663,
        i32::MIN,
        i32::MAX,
        0x0100_0041, // low byte 'A'
        0x0000_0141, // low byte 'A'
        -0x7FFF_FFBF,
        0x7FFF_FF7A, // low byte 'z'
        -0x8000_0000 + 0x30, // low byte '0'
    ];
    // Plus: every byte value lifted into all four "high garbage" patterns, so a
    // wrong narrowing anywhere in the 256-value domain is caught.
    let mut rng = Rng::new(SEED ^ 7);
    for high in [0x0000_0100u32, 0xFFFF_FF00, 0x1234_5600, 0xDEAD_BE00] {
        for _ in 0..24 {
            raw.push((high | rng.next_u8() as u32) as i32);
        }
    }
    let cf = c_driver();
    let rf = rust_driver();

    // Re-type both exported symbols as taking a full `int`, so the raw
    // out-of-range value really crosses the FFI boundary and each callee has to
    // narrow it itself (this is how a C caller with a mismatched prototype, or
    // an `enum` value with no valid variant, actually reaches the function).
    let cf_int: unsafe extern "C" fn(std::ffi::c_int) =
        unsafe { std::mem::transmute::<DriverFn, unsafe extern "C" fn(std::ffi::c_int)>(cf) };
    let rf_int: unsafe extern "C" fn(std::ffi::c_int) =
        unsafe { std::mem::transmute::<DriverFn, unsafe extern "C" fn(std::ffi::c_int)>(rf) };

    for v in raw {
        // (a) full `int` through the boundary — C and Rust must narrow alike.
        let a_int = capture(|| unsafe { cf_int(v) });
        let b_int = capture(|| unsafe { rf_int(v) });
        assert!(
            a_int == b_int,
            "raw out-of-range int {v} diverged across the FFI boundary:\n  C   : {}\n  RUST: {}",
            show(&a_int),
            show(&b_int)
        );

        // (b) and the documented expectation: identical to `(char) v`.
        let c = (v as u32 & 0xFF) as u8 as c_char;
        let a = capture(|| unsafe { cf(c) });
        let b = capture(|| unsafe { rf(c) });
        assert!(
            a == b,
            "out-of-range int {v} (truncated to 0x{:02x}) diverged:\n  C   : {}\n  RUST: {}",
            c as u8,
            show(&a),
            show(&b)
        );
        assert_eq!(
            a, a_int,
            "C did not narrow {v} to (char)0x{:02x}",
            c as u8
        );
        assert_eq!(
            b, b_int,
            "Rust did not narrow {v} to (char)0x{:02x}",
            c as u8
        );
    }
}

// --- Row 8: repeated invocation over the whole domain ----------------------

fn err_repeated_calls_no_state_leak() {
    let cf = c_driver();
    let rf = rust_driver();
    let all = all_chars();

    let a = {
        let all = all.clone();
        capture(|| unsafe {
            for &c in &all {
                cf(c);
            }
        })
    };
    let b = {
        let all = all.clone();
        capture(|| unsafe {
            for &c in &all {
                rf(c);
            }
        })
    };
    assert!(
        a == b,
        "256-call stream diverged (C {} bytes, Rust {} bytes)",
        a.len(),
        b.len()
    );

    // The stream must be the exact concatenation of the individual outputs
    // (proves `setlocale(LC_ALL,"C")` is idempotent in both builds).
    let mut concat_c = Vec::new();
    let mut concat_r = Vec::new();
    for &c in &all {
        concat_c.extend(c_output(c));
        concat_r.extend(rust_output(c));
    }
    assert!(concat_c == a, "C: repeated calls are not stateless");
    assert!(concat_r == b, "Rust: repeated calls are not stateless");

    // Second full pass must reproduce the first byte-for-byte.
    let a2 = {
        let all = all.clone();
        capture(|| unsafe {
            for &c in &all {
                cf(c);
            }
        })
    };
    assert!(a2 == a, "C second pass differs from first");
}

// --- Row 9: interleaving C and Rust in one process -------------------------

fn err_interleaved_c_and_rust() {
    let cf = c_driver();
    let rf = rust_driver();
    let mut rng = Rng::new(SEED ^ 0xC0FFEE);
    // Baselines taken in isolation.
    let mut baseline = std::collections::HashMap::new();
    for c in all_chars() {
        baseline.insert(c, c_output(c));
    }
    for _ in 0..400 {
        let c = rng.next_i8() as c_char;
        // Interleave in a random order and confirm each call still matches its
        // isolated baseline.
        if rng.next_u64() & 1 == 0 {
            let a = capture(|| unsafe { cf(c) });
            let b = capture(|| unsafe { rf(c) });
            assert!(a == b && a == baseline[&c], "interleaved C/Rust drifted for {c}");
        } else {
            let b = capture(|| unsafe { rf(c) });
            let a = capture(|| unsafe { cf(c) });
            assert!(a == b && a == baseline[&c], "interleaved Rust/C drifted for {c}");
        }
    }
}

// --- Generic boundary sweep (belt and braces) ------------------------------

fn err_generic_boundary_sweep_exhaustive() {
    // Every representable argument, asserting exact byte equality.  There is no
    // pointer, length, or enum parameter in this API, so this is the complete
    // input domain.
    for c in all_chars() {
        assert_same(c, "generic exhaustive boundary sweep");
    }
}

// ---------------------------------------------------------------------------
// Single entry point.
//
// All rows run inside ONE `#[test]` deliberately: the harness captures fd 1,
// which is process-global, and libtest writes its own per-test progress lines
// ("test foo ... ok") to fd 1 from the test-runner thread.  With more than one
// `#[test]` in this binary those progress lines race into the captured bytes
// and produce bogus divergences.  One test per capturing binary removes the
// race entirely, regardless of `--test-threads`.  Per-row progress is reported
// on stderr, which is never redirected.
// ---------------------------------------------------------------------------

#[test]
fn phase_c_all_error_rows() {
    let rows: &[(&str, fn())] = &[
        ("err_nul_byte", err_nul_byte as fn()),
        ("err_char_min", err_char_min as fn()),
        ("err_char_max", err_char_max as fn()),
        ("err_minus_one", err_minus_one as fn()),
        ("err_all_negative_chars", err_all_negative_chars as fn()),
        ("err_one_past_ascii", err_one_past_ascii as fn()),
        ("err_out_of_range_int_truncation", err_out_of_range_int_truncation as fn()),
        ("err_repeated_calls_no_state_leak", err_repeated_calls_no_state_leak as fn()),
        ("err_interleaved_c_and_rust", err_interleaved_c_and_rust as fn()),
        ("err_generic_boundary_sweep_exhaustive", err_generic_boundary_sweep_exhaustive as fn()),
    ];
    eprintln!("\n=== ERRORS.md: {} rows ===", rows.len());
    for (i, (name, f)) in rows.iter().enumerate() {
        eprint!("  [{:>2}/{}] {name} ... ", i + 1, rows.len());
        f();
        eprintln!("ok");
    }
    eprintln!("=== ERRORS.md: all {} rows passed ===", rows.len());
}
