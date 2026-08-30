//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! Phase B rows come from `CONFIGS.md`, Phase C rows from `ERRORS.md`.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Phase B — valid-path rows (CONFIGS.md C1..C23)
// ---------------------------------------------------------------------------

fn config_c1_nul() {
    assert_same(0, "C1 NUL 0x00");
}

fn config_c2_cntrl_low() {
    assert_same_range(0x01..=0x08, 64, "C2 pure cntrl 0x01..0x08");
}

fn config_c3_tab() {
    assert_same(b'\t' as c_char, "C3 tab: cntrl+space+blank");
}

fn config_c4_cntrl_space_not_blank() {
    assert_same_range(0x0A..=0x0D, 32, "C4 \\n \\v \\f \\r: cntrl+space, not blank");
}

fn config_c5_cntrl_high() {
    assert_same_range(0x0E..=0x1F, 64, "C5 pure cntrl 0x0E..0x1F");
}

fn config_c6_space() {
    assert_same(b' ' as c_char, "C6 space: print+space+blank, not graph");
}

fn config_c7_punct_21_2f() {
    assert_same_range(0x21..=0x2F, 64, "C7 punct 0x21..0x2F");
}

fn config_c8_digits() {
    assert_same_range(b'0'..=b'9', 64, "C8 digits: digit+xdigit+alnum, not alpha");
}

fn config_c9_punct_3a_40() {
    assert_same_range(0x3A..=0x40, 64, "C9 punct 0x3A..0x40");
}

fn config_c10_upper_hex() {
    assert_same_range(b'A'..=b'F', 64, "C10 'A'..'F': upper+alpha+alnum+xdigit");
}

fn config_c11_upper_nonhex() {
    assert_same_range(b'G'..=b'Z', 64, "C11 'G'..'Z': upper+alpha+alnum, no xdigit");
}

fn config_c12_punct_5b_60() {
    assert_same_range(0x5B..=0x60, 64, "C12 punct 0x5B..0x60 (incl. backtick)");
}

fn config_c13_lower_hex() {
    assert_same_range(b'a'..=b'f', 64, "C13 'a'..'f': lower+alpha+alnum+xdigit");
}

fn config_c14_lower_nonhex() {
    assert_same_range(b'g'..=b'z', 64, "C14 'g'..'z': lower+alpha+alnum, no xdigit");
}

fn config_c15_punct_7b_7e() {
    assert_same_range(0x7B..=0x7E, 32, "C15 punct 0x7B..0x7E");
}

fn config_c16_del() {
    assert_same(0x7F, "C16 DEL: cntrl, not print/graph");
}

fn config_c17_negative_chars() {
    // 0x80..=0xFF reach the callee as negative `char` values, i.e. negative
    // indices into glibc's ctype tables.
    assert_same_range(0x80..=0xFF, 256, "C17 negative char (0x80..0xFF)");
}

fn config_c18_exhaustive_all_256() {
    for b in 0u16..=255 {
        assert_same(b as u8 as c_char, "C18 exhaustive");
    }
}

fn config_c19_repeated_calls_idempotent() {
    let mut rng = Rng::new(0xC19_C19);
    for _ in 0..40 {
        let b = rng.next_u8() as c_char;
        let first_c = c_out(b);
        let first_r = rust_out(b);
        assert_eq!(first_c, first_r, "C19 first call mismatch for {b}");
        for _ in 0..3 {
            assert_eq!(c_out(b), first_c, "C19 C not idempotent for {b}");
            assert_eq!(rust_out(b), first_r, "C19 Rust not idempotent for {b}");
        }
    }
}

fn config_c20_hostile_caller_locale() {
    // The callee resets LC_ALL to "C" itself, so a caller-installed locale must
    // not change anything. Whatever locales exist on this box, both
    // implementations must react identically.
    for name in [
        &b"en_US.UTF-8"[..],
        &b"C.UTF-8"[..],
        &b"POSIX"[..],
        &b""[..],
    ] {
        for b in [0u8, b'A', b'z', b'5', 0x20, 0x7F, 0x80, 0xB5, 0xE9, 0xFF] {
            let available = try_set_locale(name);
            let c_bytes = c_out(b as c_char);
            let _ = try_set_locale(name);
            let r_bytes = rust_out(b as c_char);
            assert_eq!(
                c_bytes, r_bytes,
                "C20 mismatch under locale {:?} (available={available}) for byte 0x{b:02x}",
                String::from_utf8_lossy(name)
            );
        }
    }
    // Leave the process in a known state.
    try_set_locale(b"C");
}

fn config_c21_interleaved_calls() {
    let mut rng = Rng::new(0xC21_5EED);
    for _ in 0..200 {
        let b = rng.next_u8() as c_char;
        // Alternate which side goes first to catch any shared-stream state.
        if rng.next_u32() & 1 == 0 {
            let a = c_out(b);
            let d = rust_out(b);
            assert_eq!(a, d, "C21 mismatch (C first) for {b}");
        } else {
            let d = rust_out(b);
            let a = c_out(b);
            assert_eq!(a, d, "C21 mismatch (Rust first) for {b}");
        }
    }
}

fn config_c22_wide_int_argument() {
    let mut rng = Rng::new(0xC22_5EED);
    for _ in 0..500 {
        let v = rng.next_u32() as c_int;
        assert_same_wide(v, "C22 randomized wide int argument");
    }
    for low in 0u32..=255 {
        // Same low byte, garbage above it.
        let v = (0x1234_5600u32 | low) as c_int;
        assert_same_wide(v, "C22 garbage high bits, sweeping low byte");
    }
}

fn config_c23_randomized_sweep() {
    let mut rng = Rng::new(0xDEAD_BEEF_C23);
    for _ in 0..4000 {
        let b = rng.next_u8() as c_char;
        assert_same(b, "C23 randomized sweep");
    }
}

// ---------------------------------------------------------------------------
// Phase C — error/rejection rows (ERRORS.md E1..E9)
//
// `driver` returns void and has no rejection path at all, so for every row the
// asserted "same error/rejection" is: both implementations return normally and
// emit the identical 14-line byte stream. A Rust panic/abort or a differing
// stream is the failure mode these tests are hunting.
// ---------------------------------------------------------------------------

/// Every call must produce exactly these 14 lines, in this order.
const LABELS: [&str; 14] = [
    "alphanumeric: ",
    "alphabetic: ",
    "lowercase: ",
    "uppercase: ",
    "digit: ",
    "hexadecimal: ",
    "control: ",
    "graphical: ",
    "space: ",
    "blank: ",
    "printing: ",
    "punctuation: ",
    "to lower: ",
    "to upper: ",
];

fn assert_shape(bytes: &[u8], ctx: &str) {
    let lines: Vec<&[u8]> = bytes.split(|b| *b == b'\n').collect();
    // 14 lines each terminated by '\n' => 15 pieces, the last empty.
    assert_eq!(lines.len(), 15, "{ctx}: expected 14 newline-terminated lines");
    assert!(lines[14].is_empty(), "{ctx}: trailing data after last line");
    for (i, label) in LABELS.iter().enumerate() {
        assert!(
            lines[i].starts_with(label.as_bytes()),
            "{ctx}: line {i} = {:?} does not start with {label:?}",
            String::from_utf8_lossy(lines[i])
        );
    }
}

fn error_e1_nul_byte() {
    // %c is asked to print a NUL byte; the line therefore contains an embedded
    // NUL. Both sides must emit it identically (not stop early, not skip it).
    let c_bytes = c_out(0);
    let r_bytes = rust_out(0);
    assert_eq!(c_bytes, r_bytes, "E1 NUL byte output mismatch");
    assert_shape(&c_bytes, "E1 C");
    assert_shape(&r_bytes, "E1 Rust");
    assert!(
        c_bytes.windows(2).any(|w| w == b"\0\n"),
        "E1: expected an embedded NUL from %c, got {:?}",
        String::from_utf8_lossy(&c_bytes)
    );
}

fn error_e2_negative_chars() {
    for b in 0x80u16..=0xFF {
        let c = b as u8 as c_char;
        assert!(c < 0, "E2 precondition: 0x{b:02x} must be a negative char");
        let c_bytes = c_out(c);
        let r_bytes = rust_out(c);
        assert_eq!(c_bytes, r_bytes, "E2 mismatch for byte 0x{b:02x}");
        assert_shape(&c_bytes, "E2");
    }
}

fn error_e3_min_char() {
    let c_bytes = c_out(-128);
    let r_bytes = rust_out(-128);
    assert_eq!(c_bytes, r_bytes, "E3 mismatch at c = -128");
    assert_shape(&c_bytes, "E3");
}

fn error_e4_del_127() {
    let c_bytes = c_out(127);
    let r_bytes = rust_out(127);
    assert_eq!(c_bytes, r_bytes, "E4 mismatch at c = 127 (DEL)");
    assert_shape(&c_bytes, "E4");
}

fn error_e5_eof_like_ff() {
    // 0xFF == (char)-1, the value that aliases EOF after promotion.
    let c_bytes = c_out(-1);
    let r_bytes = rust_out(-1);
    assert_eq!(c_bytes, r_bytes, "E5 mismatch at c = -1 (0xFF / EOF-like)");
    assert_shape(&c_bytes, "E5");
}

fn error_e6_wide_int_arg() {
    // A `char` parameter accepts any `int` at the ABI level — the exact same
    // situation as an out-of-range enum value crossing the FFI boundary.
    for v in [
        0x1234_5641i32,
        0x0000_01FFu32 as i32,
        0xFFFF_FF80u32 as i32,
        0x7FFF_FF41,
        -0x7FFF_FFBF,
        0x0000_0100,
        0x00FF_FF00,
    ] {
        let c_bytes = c_out_wide(v);
        let r_bytes = rust_out_wide(v);
        assert_eq!(c_bytes, r_bytes, "E6 mismatch for wide arg 0x{v:08x}");
        assert_shape(&c_bytes, "E6");
    }
}

fn error_e7_one_past_range() {
    // One step past every documented boundary of the ctype tables, plus the
    // extremes of `int`.
    for v in [256i32, 257, -129, -130, 128, -128, 255, i32::MIN, i32::MAX, 0] {
        let c_bytes = c_out_wide(v);
        let r_bytes = rust_out_wide(v);
        assert_eq!(c_bytes, r_bytes, "E7 mismatch for out-of-range arg {v}");
        assert_shape(&c_bytes, "E7");
    }
}

fn error_e8_locale_hostile_caller() {
    // Install every locale we can, then check the callee's own reset makes the
    // result identical to the pristine "C"-locale result.
    try_set_locale(b"C");
    let baseline: Vec<(u8, Vec<u8>)> = [0u8, b'A', b'a', b'0', 0x20, 0x7F, 0x80, 0xE9, 0xFF]
        .iter()
        .map(|b| (*b, c_out(*b as c_char)))
        .collect();

    for name in [&b"en_US.UTF-8"[..], &b"C.UTF-8"[..], &b"POSIX"[..], &b""[..]] {
        for (b, base) in baseline.iter() {
            try_set_locale(name);
            let c_bytes = c_out(*b as c_char);
            try_set_locale(name);
            let r_bytes = rust_out(*b as c_char);
            assert_eq!(
                c_bytes, r_bytes,
                "E8 C/Rust mismatch under locale {:?} for 0x{b:02x}",
                String::from_utf8_lossy(name)
            );
            assert_eq!(
                &c_bytes, base,
                "E8 callee's setlocale reset failed under {:?} for 0x{b:02x}",
                String::from_utf8_lossy(name)
            );
        }
    }
    try_set_locale(b"C");
}

fn error_e9_setlocale_result_ignored() {
    // The C never checks setlocale's return value, so a bogus locale left
    // installed by the caller (setlocale returns NULL and changes nothing) must
    // not alter the 14 printfs.
    try_set_locale(b"C");
    let baseline = c_out(b'A' as c_char);
    let bogus = try_set_locale(b"no_SUCH.locale-42");
    assert!(!bogus, "E9 precondition: bogus locale should fail to install");
    let c_bytes = c_out(b'A' as c_char);
    let r_bytes = rust_out(b'A' as c_char);
    assert_eq!(c_bytes, r_bytes, "E9 mismatch after failed setlocale");
    assert_eq!(c_bytes, baseline, "E9 output changed after failed setlocale");
    assert_shape(&r_bytes, "E9");
}

// ---------------------------------------------------------------------------
// Harness self-checks — make sure a real comparison is happening.
// ---------------------------------------------------------------------------

fn harness_captures_real_output() {
    let out = c_out(b'A' as c_char);
    assert!(!out.is_empty(), "harness captured nothing from the C .so");
    assert_shape(&out, "harness C");
    let out = rust_out(b'A' as c_char);
    assert!(!out.is_empty(), "harness captured nothing from the Rust .so");
    assert_shape(&out, "harness Rust");
    // Different inputs must produce different output, otherwise the harness
    // could be trivially "passing".
    assert_ne!(
        c_out(b'A' as c_char),
        c_out(b'z' as c_char),
        "harness is not input-sensitive"
    );
}

fn harness_raw_bit_values_are_glibc_masks() {
    // Guards the assumption in ctype.rs: glibc's is*() macros yield the raw
    // table bit, not a normalised 1. Compare against the C, which is truth.
    let out = String::from_utf8_lossy(&c_out(b'A' as c_char)).to_string();
    assert!(
        out.contains("alphabetic: 1024"),
        "unexpected C bit value; got:\n{out}"
    );
    assert_eq!(out, String::from_utf8_lossy(&rust_out(b'A' as c_char)));
}

// ---------------------------------------------------------------------------
// Sequential runner (`harness = false`).
//
// The differential harness captures output by pointing file descriptor 1 at a
// scratch file, which is process-global.  libtest's default harness runs tests
// on several threads and writes its own progress to fd 1, so its writes land
// inside another thread's capture window and corrupt the comparison.  Running
// the cases strictly sequentially from a single thread removes that race
// entirely, and keeps per-case reporting.
// ---------------------------------------------------------------------------

fn main() {
    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with('-'));

    let tests: Vec<(&str, fn())> = vec![
        ("config_c1_nul", config_c1_nul as fn()),
        ("config_c2_cntrl_low", config_c2_cntrl_low as fn()),
        ("config_c3_tab", config_c3_tab as fn()),
        ("config_c4_cntrl_space_not_blank", config_c4_cntrl_space_not_blank as fn()),
        ("config_c5_cntrl_high", config_c5_cntrl_high as fn()),
        ("config_c6_space", config_c6_space as fn()),
        ("config_c7_punct_21_2f", config_c7_punct_21_2f as fn()),
        ("config_c8_digits", config_c8_digits as fn()),
        ("config_c9_punct_3a_40", config_c9_punct_3a_40 as fn()),
        ("config_c10_upper_hex", config_c10_upper_hex as fn()),
        ("config_c11_upper_nonhex", config_c11_upper_nonhex as fn()),
        ("config_c12_punct_5b_60", config_c12_punct_5b_60 as fn()),
        ("config_c13_lower_hex", config_c13_lower_hex as fn()),
        ("config_c14_lower_nonhex", config_c14_lower_nonhex as fn()),
        ("config_c15_punct_7b_7e", config_c15_punct_7b_7e as fn()),
        ("config_c16_del", config_c16_del as fn()),
        ("config_c17_negative_chars", config_c17_negative_chars as fn()),
        ("config_c18_exhaustive_all_256", config_c18_exhaustive_all_256 as fn()),
        ("config_c19_repeated_calls_idempotent", config_c19_repeated_calls_idempotent as fn()),
        ("config_c20_hostile_caller_locale", config_c20_hostile_caller_locale as fn()),
        ("config_c21_interleaved_calls", config_c21_interleaved_calls as fn()),
        ("config_c22_wide_int_argument", config_c22_wide_int_argument as fn()),
        ("config_c23_randomized_sweep", config_c23_randomized_sweep as fn()),
        ("error_e1_nul_byte", error_e1_nul_byte as fn()),
        ("error_e2_negative_chars", error_e2_negative_chars as fn()),
        ("error_e3_min_char", error_e3_min_char as fn()),
        ("error_e4_del_127", error_e4_del_127 as fn()),
        ("error_e5_eof_like_ff", error_e5_eof_like_ff as fn()),
        ("error_e6_wide_int_arg", error_e6_wide_int_arg as fn()),
        ("error_e7_one_past_range", error_e7_one_past_range as fn()),
        ("error_e8_locale_hostile_caller", error_e8_locale_hostile_caller as fn()),
        ("error_e9_setlocale_result_ignored", error_e9_setlocale_result_ignored as fn()),
        ("harness_captures_real_output", harness_captures_real_output as fn()),
        ("harness_raw_bit_values_are_glibc_masks", harness_raw_bit_values_are_glibc_masks as fn()),
    ];

    let mut passed = 0usize;
    let mut failed: Vec<&str> = Vec::new();
    let mut skipped = 0usize;

    println!("\nrunning {} tests (sequential harness)", tests.len());
    for (name, f) in tests {
        if let Some(flt) = &filter {
            if !name.contains(flt.as_str()) {
                skipped += 1;
                continue;
            }
        }
        print!("test {name} ... ");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        match std::panic::catch_unwind(f) {
            Ok(()) => {
                println!("ok");
                passed += 1;
            }
            Err(_) => {
                println!("FAILED");
                failed.push(name);
            }
        }
        let _ = std::io::stdout().flush();
    }

    println!();
    if failed.is_empty() {
        println!("test result: ok. {passed} passed; 0 failed; {skipped} filtered out");
    } else {
        println!("failures:");
        for n in &failed {
            println!("    {n}");
        }
        println!(
            "test result: FAILED. {passed} passed; {} failed; {skipped} filtered out",
            failed.len()
        );
        std::process::exit(101);
    }
}
