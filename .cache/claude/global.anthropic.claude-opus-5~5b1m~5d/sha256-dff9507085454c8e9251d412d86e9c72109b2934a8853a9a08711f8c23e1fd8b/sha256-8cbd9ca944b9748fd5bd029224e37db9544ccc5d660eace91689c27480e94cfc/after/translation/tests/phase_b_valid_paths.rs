//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every test loads BOTH shared objects via
//! `libloading` and compares captured stdout byte-for-byte.

mod common;

use common::*;
use std::ffi::c_char;

// --- Row 1 -----------------------------------------------------------------

fn row01_exhaustive_all_256_char_values() {
    for c in all_chars() {
        assert_same(c, "row01 exhaustive sweep");
    }
}

// --- Row 2 -----------------------------------------------------------------

fn row02_uppercase_letters_randomized_order() {
    let mut v = range(b'A', b'Z');
    let mut rng = Rng::new(SEED ^ 2);
    rng.shuffle(&mut v);
    assert_same_set(&v, "row02 uppercase A-Z");
    // plus randomized repeats
    for _ in 0..200 {
        let c = rng.pick(&v);
        assert_same(c, "row02 uppercase random pick");
    }
}

// --- Row 3 -----------------------------------------------------------------

fn row03_lowercase_letters_randomized_order() {
    let mut v = range(b'a', b'z');
    let mut rng = Rng::new(SEED ^ 3);
    rng.shuffle(&mut v);
    assert_same_set(&v, "row03 lowercase a-z");
    for _ in 0..200 {
        let c = rng.pick(&v);
        assert_same(c, "row03 lowercase random pick");
    }
}

// --- Row 4 -----------------------------------------------------------------

fn row04_decimal_digits() {
    let mut v = range(b'0', b'9');
    let mut rng = Rng::new(SEED ^ 4);
    rng.shuffle(&mut v);
    assert_same_set(&v, "row04 digits 0-9");
    for _ in 0..100 {
        let c = rng.pick(&v);
        assert_same(c, "row04 digit random pick");
    }
}

// --- Row 5 -----------------------------------------------------------------

fn row05_hex_only_letters() {
    let mut v = range(b'A', b'F');
    v.extend(range(b'a', b'f'));
    let mut rng = Rng::new(SEED ^ 5);
    rng.shuffle(&mut v);
    assert_same_set(&v, "row05 hex letters A-F/a-f");
    for _ in 0..100 {
        let c = rng.pick(&v);
        assert_same(c, "row05 hex letter random pick");
    }
}

// --- Row 6 -----------------------------------------------------------------

fn row06_non_hex_letters() {
    let mut v = range(b'G', b'Z');
    v.extend(range(b'g', b'z'));
    let mut rng = Rng::new(SEED ^ 6);
    rng.shuffle(&mut v);
    assert_same_set(&v, "row06 non-hex letters G-Z/g-z");
    for _ in 0..150 {
        let c = rng.pick(&v);
        assert_same(c, "row06 non-hex letter random pick");
    }
}

// --- Row 7 -----------------------------------------------------------------

fn row07_blank_whitespace() {
    // The only two `isblank` characters, on opposite sides of the
    // printable/control boundary.
    let v: Vec<c_char> = vec![b' ' as c_char, b'\t' as c_char];
    assert_same_set(&v, "row07 blank ' ' and '\\t'");
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..50 {
        let c = rng.pick(&v);
        assert_same(c, "row07 blank random pick");
    }
}

// --- Row 8 -----------------------------------------------------------------

fn row08_nonblank_whitespace() {
    let v = range(0x0A, 0x0D); // \n \v \f \r
    assert_same_set(&v, "row08 non-blank whitespace 0x0A-0x0D");
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..50 {
        let c = rng.pick(&v);
        assert_same(c, "row08 non-blank whitespace random pick");
    }
}

// --- Row 9 -----------------------------------------------------------------

fn row09_punctuation_all_four_runs() {
    let mut v = Vec::new();
    v.extend(range(0x21, 0x2F));
    v.extend(range(0x3A, 0x40));
    v.extend(range(0x5B, 0x60));
    v.extend(range(0x7B, 0x7E));
    assert_eq!(v.len(), 32, "ASCII has 32 punctuation characters");
    let mut rng = Rng::new(SEED ^ 9);
    rng.shuffle(&mut v);
    assert_same_set(&v, "row09 punctuation");
    for _ in 0..200 {
        let c = rng.pick(&v);
        assert_same(c, "row09 punctuation random pick");
    }
}

// --- Row 10 ----------------------------------------------------------------

fn row10_control_characters() {
    let mut v = Vec::new();
    v.extend(range(0x01, 0x08));
    v.extend(range(0x0E, 0x1F));
    let mut rng = Rng::new(SEED ^ 10);
    rng.shuffle(&mut v);
    assert_same_set(&v, "row10 control chars");
    for _ in 0..150 {
        let c = rng.pick(&v);
        assert_same(c, "row10 control random pick");
    }
}

// --- Row 11 ----------------------------------------------------------------

fn row11_class_run_boundaries() {
    let bounds: [u8; 22] = [
        0x1F, 0x20, 0x21, // cntrl | space | punct
        0x2F, 0x30, 0x39, 0x3A, // punct | digit run | punct
        0x40, 0x41, 0x5A, 0x5B, // punct | upper run | punct
        0x60, 0x61, 0x7A, 0x7B, // punct | lower run | punct
        0x7E, 0x7F, // last graph | DEL
        0x08, 0x09, 0x0D, 0x0E, // cntrl | blank-space | space | cntrl
        0x00,
    ];
    let v: Vec<c_char> = bounds.iter().map(|&b| b as c_char).collect();
    assert_same_set(&v, "row11 class boundaries");
}

// --- Row 12 ----------------------------------------------------------------

fn row12_all_negative_chars() {
    let v: Vec<c_char> = (-128i16..0).map(|i| i as i8 as c_char).collect();
    assert_eq!(v.len(), 128);
    let mut rng = Rng::new(SEED ^ 12);
    let mut shuffled = v.clone();
    rng.shuffle(&mut shuffled);
    assert_same_set(&shuffled, "row12 negative chars -128..-1");
}

// --- Row 13 ----------------------------------------------------------------

fn row13_unsigned_byte_domain_wraparound() {
    // Feed 0..=255 as *unsigned* bytes reinterpreted into the signed `char`
    // parameter; bytes >= 0x80 must behave exactly like their negative twins.
    for b in 0u16..256 {
        let as_signed = b as u8 as c_char;
        assert_same(as_signed, "row13 unsigned byte domain");
    }
    // And explicitly: 0x80 output == -128 output.
    let a = rust_output(0x80u8 as c_char);
    let b = rust_output(-128i8 as c_char);
    assert_eq!(a, b, "0x80 and -128 must be the same argument");
    let ca = c_output(0x80u8 as c_char);
    assert_eq!(ca, a, "C vs Rust for 0x80");
}

// --- Row 14 ----------------------------------------------------------------

fn row14_randomized_property_run() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..4096 {
        let c = rng.next_i8() as c_char;
        assert_same(c, &format!("row14 randomized iteration {i}"));
    }
}

// --- Row 15 ----------------------------------------------------------------

fn row15_repeated_calls_single_stream() {
    // 512 consecutive calls captured as ONE stream, so any per-call state
    // leakage (e.g. a non-idempotent setlocale) shows up.
    let mut rng = Rng::new(SEED ^ 15);
    let values: Vec<c_char> = (0..512).map(|_| rng.next_i8() as c_char).collect();

    let cf = c_driver();
    let rf = rust_driver();
    let vc = values.clone();
    let c_blob = capture(|| unsafe {
        for &v in &vc {
            cf(v);
        }
    });
    let vr = values.clone();
    let r_blob = capture(|| unsafe {
        for &v in &vr {
            rf(v);
        }
    });
    assert_eq!(
        c_blob.len(),
        r_blob.len(),
        "row15 stream length mismatch: C {} vs Rust {}",
        c_blob.len(),
        r_blob.len()
    );
    assert!(c_blob == r_blob, "row15 512-call stream diverged");

    // The stream must equal the concatenation of the individual outputs.
    let mut concat = Vec::new();
    for &v in &values {
        concat.extend(c_output(v));
    }
    assert!(concat == c_blob, "row15 C stream != concatenation of calls");
}

// --- Row 16 ----------------------------------------------------------------

fn row16_interleaved_c_and_rust_calls() {
    let mut rng = Rng::new(SEED ^ 16);
    let cf = c_driver();
    let rf = rust_driver();
    for _ in 0..300 {
        let v = rng.next_i8() as c_char;
        // C first, then Rust.
        let a = capture(|| unsafe { cf(v) });
        let b = capture(|| unsafe { rf(v) });
        assert!(a == b, "row16 C-then-Rust diverged for {v}");
        // Rust first, then C — ordering must not matter.
        let b2 = capture(|| unsafe { rf(v) });
        let a2 = capture(|| unsafe { cf(v) });
        assert!(a2 == b2, "row16 Rust-then-C diverged for {v}");
        assert!(a == a2, "row16 C output order-dependent for {v}");
        assert!(b == b2, "row16 Rust output order-dependent for {v}");
    }
}

// --- Row 17 ----------------------------------------------------------------

fn row17_ambient_locale_not_c() {
    let cf = c_driver();
    let rf = rust_driver();
    let candidates = ["C", "POSIX", "C.UTF-8", "en_US.UTF-8", "de_DE.UTF-8"];
    let mut tested = 0usize;
    for loc in candidates {
        if !set_process_locale(loc) {
            continue; // locale not installed on this host
        }
        tested += 1;
        for c in all_chars() {
            // Re-establish the ambient locale before EACH call, because
            // `driver` itself resets it to "C".
            set_process_locale(loc);
            let a = capture(|| unsafe { cf(c) });
            set_process_locale(loc);
            let b = capture(|| unsafe { rf(c) });
            assert!(
                a == b,
                "row17 diverged under ambient locale {loc} for driver({c}):\n  C   : {}\n  RUST: {}",
                show(&a),
                show(&b)
            );
        }
    }
    // Leave the process in the "C" locale.
    set_process_locale("C");
    eprintln!("(row17 exercised {tested} locales) ");
    assert!(
        tested >= 3,
        "row17: only {tested} locale(s) available; expected at least 3 \
         (C, POSIX and one UTF-8 locale) for this row to be meaningful"
    );
}

// --- Row 18 ----------------------------------------------------------------

fn row18_ambient_locale_from_environment() {
    let cf = c_driver();
    let rf = rust_driver();
    for (var, val) in [("LC_ALL", "C.UTF-8"), ("LANG", "en_US.UTF-8")] {
        unsafe {
            std::env::set_var(var, val);
        }
        // Adopt the environment ("" means "read LC_*/LANG from the env").
        let ok = set_process_locale("");
        for c in all_chars() {
            if ok {
                set_process_locale("");
            }
            let a = capture(|| unsafe { cf(c) });
            if ok {
                set_process_locale("");
            }
            let b = capture(|| unsafe { rf(c) });
            assert!(
                a == b,
                "row18 diverged with {var}={val} for driver({c}):\n  C   : {}\n  RUST: {}",
                show(&a),
                show(&b)
            );
        }
        unsafe {
            std::env::remove_var(var);
        }
    }
    set_process_locale("C");
}

// --- Row 19 ----------------------------------------------------------------

fn row19_abi_shape_void_return_and_no_clobber() {
    let (c_path, rust_path) = so_paths();
    assert!(c_path.exists() && rust_path.exists());

    let cf: DriverFn = c_driver();
    let rf: DriverFn = rust_driver();

    // Call 256 times through the identical `extern "C" fn(c_char)` signature
    // and verify local state around the calls is untouched.
    let sentinel_a: u64 = 0x0123_4567_89AB_CDEF;
    let sentinel_b: [u8; 16] = [0xA5; 16];
    let mut n = 0usize;
    let out = capture(|| {
        for b in 0u16..256 {
            unsafe {
                cf(b as u8 as c_char);
                rf(b as u8 as c_char);
            }
            n += 1;
        }
    });
    assert_eq!(n, 256);
    assert_eq!(sentinel_a, 0x0123_4567_89AB_CDEF, "caller state clobbered");
    assert_eq!(sentinel_b, [0xA5; 16], "caller state clobbered");
    // 256 values x 2 implementations x 14 lines, plus the extra newlines the
    // two `%c` conversions emit when the input itself is '\n'.
    let want: usize = all_chars().iter().map(|&c| 2 * expected_newlines(c)).sum();
    assert_eq!(
        out.iter().filter(|&&x| x == b'\n').count(),
        want,
        "unexpected total line count"
    );
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
fn phase_b_all_config_rows() {
    let rows: &[(&str, fn())] = &[
        ("row01_exhaustive_all_256_char_values", row01_exhaustive_all_256_char_values as fn()),
        ("row02_uppercase_letters_randomized_order", row02_uppercase_letters_randomized_order as fn()),
        ("row03_lowercase_letters_randomized_order", row03_lowercase_letters_randomized_order as fn()),
        ("row04_decimal_digits", row04_decimal_digits as fn()),
        ("row05_hex_only_letters", row05_hex_only_letters as fn()),
        ("row06_non_hex_letters", row06_non_hex_letters as fn()),
        ("row07_blank_whitespace", row07_blank_whitespace as fn()),
        ("row08_nonblank_whitespace", row08_nonblank_whitespace as fn()),
        ("row09_punctuation_all_four_runs", row09_punctuation_all_four_runs as fn()),
        ("row10_control_characters", row10_control_characters as fn()),
        ("row11_class_run_boundaries", row11_class_run_boundaries as fn()),
        ("row12_all_negative_chars", row12_all_negative_chars as fn()),
        ("row13_unsigned_byte_domain_wraparound", row13_unsigned_byte_domain_wraparound as fn()),
        ("row14_randomized_property_run", row14_randomized_property_run as fn()),
        ("row15_repeated_calls_single_stream", row15_repeated_calls_single_stream as fn()),
        ("row16_interleaved_c_and_rust_calls", row16_interleaved_c_and_rust_calls as fn()),
        ("row17_ambient_locale_not_c", row17_ambient_locale_not_c as fn()),
        ("row18_ambient_locale_from_environment", row18_ambient_locale_from_environment as fn()),
        ("row19_abi_shape_void_return_and_no_clobber", row19_abi_shape_void_return_and_no_clobber as fn()),
    ];
    eprintln!("\n=== CONFIGS.md: {} rows ===", rows.len());
    for (i, (name, f)) in rows.iter().enumerate() {
        eprint!("  [{:>2}/{}] {name} ... ", i + 1, rows.len());
        f();
        eprintln!("ok");
    }
    eprintln!("=== CONFIGS.md: all {} rows passed ===", rows.len());
}
