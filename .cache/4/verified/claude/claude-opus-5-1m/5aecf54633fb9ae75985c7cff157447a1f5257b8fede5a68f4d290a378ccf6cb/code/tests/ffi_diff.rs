//! Phase B — valid-path differential tests through the **shared-object FFI
//! boundary**. Covers `CONFIGS.md` rows 1–11 and 18–22.
//!
//! Both libraries are loaded with `libloading` and driven through their exported
//! C symbols. Tests are ordered lowest-level entry point first (`printIntLine`,
//! `printLine`), then the composed operations (`bad`, `good`), then the top-level
//! `main`, so a failure points at the deepest broken layer.

mod common;
use common::ffi::{Call, MixedOp};
use common::{ffi, Rng, SEED};

// ---------------------------------------------------------------------------
// Row 1–2: printIntLine — the lowest-level entry point
// ---------------------------------------------------------------------------

#[test]
fn row01_print_int_line_randomized_full_i32() {
    // Boundary values first, then randomized coverage of the whole i32 range.
    for v in [0i32, 1, -1, 7, 9, 10, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        ffi::assert_same(&Call::PrintIntLine(v), b"", &format!("printIntLine({v})"));
    }
    let mut rng = Rng::new(SEED);
    for _ in 0..200 {
        let v = rng.i32_any();
        ffi::assert_same(&Call::PrintIntLine(v), b"", &format!("printIntLine({v})"));
    }
    // Small magnitudes are the values the program itself produces.
    for _ in 0..100 {
        let v = rng.in_range(-32, 32) as i32;
        ffi::assert_same(&Call::PrintIntLine(v), b"", &format!("printIntLine({v})"));
    }
}

#[test]
fn row02_print_int_line_repeated_ordering() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..30 {
        let n = rng.in_range(0, 12) as usize;
        let ops: Vec<MixedOp> = (0..n).map(|_| MixedOp::Int(rng.i32_any())).collect();
        ffi::assert_same(&Call::Mixed(&ops), b"", "repeated printIntLine");
    }
}

// ---------------------------------------------------------------------------
// Row 3–7: printLine
// ---------------------------------------------------------------------------

#[test]
fn row03_print_line_randomized_ascii() {
    let mut rng = Rng::new(SEED ^ 3);
    let alpha: Vec<u8> = (0x20u8..0x7f).collect();
    for _ in 0..250 {
        let n = rng.in_range(0, 64) as usize;
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(&alpha)).collect();
        ffi::assert_same(&Call::PrintLine(Some(&s)), b"", "printLine ascii");
    }
}

#[test]
fn row04_print_line_format_specifiers_pass_through() {
    // printf("%s\n", line) must not interpret these; they are data, not a format.
    for s in [
        &b"%d"[..],
        b"%s",
        b"%n",
        b"%p %x %%",
        b"100%",
        b"%s%s%s%s%s%s%s%s",
        b"%.999d",
        b"a%db%sc",
    ] {
        ffi::assert_same(&Call::PrintLine(Some(s)), b"", "printLine format bytes");
    }
}

#[test]
fn row05_print_line_non_utf8_bytes() {
    // A C string is bytes, not UTF-8; the Rust export must not validate or
    // replace anything.
    let fixed: Vec<Vec<u8>> = vec![
        vec![0xff],
        vec![0x80, 0x81, 0x82],
        vec![0xc3],             // truncated 2-byte sequence
        vec![0xe2, 0x82],       // truncated 3-byte sequence
        vec![0xf0, 0x9f, 0x92], // truncated 4-byte sequence
        vec![0xfe, 0xff, 0xfe, 0xff],
        vec![0xc0, 0x80], // overlong encoding of NUL
        vec![0xed, 0xa0, 0x80], // UTF-16 surrogate half
    ];
    for s in &fixed {
        ffi::assert_same(&Call::PrintLine(Some(s)), b"", "printLine non-utf8 fixed");
    }
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..150 {
        let n = rng.in_range(1, 32) as usize;
        // Any nonzero byte is legal inside a C string.
        let s: Vec<u8> = (0..n).map(|_| rng.in_range(1, 255) as u8).collect();
        ffi::assert_same(&Call::PrintLine(Some(&s)), b"", "printLine non-utf8 random");
    }
}

#[test]
fn row06_print_line_embedded_control_chars() {
    for s in [
        &b"a\nb"[..],
        b"a\tb",
        b"\n",
        b"\r\n",
        b"a\rb",
        b"\x0b\x0c",
        b"line1\nline2\nline3",
        b"trailing\n",
    ] {
        ffi::assert_same(&Call::PrintLine(Some(s)), b"", "printLine control chars");
    }
}

#[test]
fn row07_print_line_and_int_line_interleaved() {
    let mut rng = Rng::new(SEED ^ 7);
    let words: [&[u8]; 5] = [b"a", b"", b"zz", b"%d", b"\xff\xfe"];
    for _ in 0..40 {
        let n = rng.in_range(0, 14) as usize;
        let ops: Vec<MixedOp> = (0..n)
            .map(|_| {
                if rng.next_u64() % 2 == 0 {
                    MixedOp::Int(rng.i32_any())
                } else {
                    MixedOp::Line(words[(rng.next_u64() % words.len() as u64) as usize])
                }
            })
            .collect();
        ffi::assert_same(&Call::Mixed(&ops), b"", "interleaved printLine/printIntLine");
    }
}

// ---------------------------------------------------------------------------
// Row 8–11: bad() through the FFI boundary.
//
// Only the indices whose out-of-bounds store stays inside bad()'s own frame are
// exercised here (data <= 15). For data >= 16 the C store lands on a live saved
// frame pointer or return address, which corrupts *whichever* caller invoked the
// shared object; the outcome is a property of the caller's frame, not of the
// library, so those rows are covered through the executables instead
// (CONFIGS.md rows 12–17, tests/exe_diff.rs).
// ---------------------------------------------------------------------------

#[test]
fn row08_bad_index_in_range_randomized() {
    for k in 0..10 {
        let stdin = format!("{k}\n");
        ffi::assert_same(&Call::Bad, stdin.as_bytes(), &format!("bad idx {k}"));
    }
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..120 {
        let k = rng.in_range(0, 9);
        // Vary the textual spelling as well as the value.
        let stdin = match rng.next_u64() % 5 {
            0 => format!("{k}\n"),
            1 => format!("  {k}\n"),
            2 => format!("+{k}\n"),
            3 => format!("{k}"), // no trailing newline
            _ => format!("000000000{k}\n"),
        };
        ffi::assert_same(&Call::Bad, stdin.as_bytes(), &format!("bad idx {k} spelled"));
    }
}

#[test]
fn row09_bad_negative_index_randomized() {
    for k in [-1i64, -2, -9, -10, -100, i32::MIN as i64] {
        let stdin = format!("{k}\n");
        ffi::assert_same(&Call::Bad, stdin.as_bytes(), &format!("bad neg {k}"));
    }
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..120 {
        let k = rng.in_range(i32::MIN as i64, -1);
        let stdin = format!("{k}\n");
        ffi::assert_same(&Call::Bad, stdin.as_bytes(), &format!("bad neg {k}"));
    }
}

#[test]
fn row10_bad_oob_within_own_frame() {
    // 10 = alignment padding, 11..13 = dead inputBuffer, 14 = i, 15 = data.
    // All benign in C; all must print ten zeros.
    for k in 10..=15 {
        let stdin = format!("{k}\n");
        ffi::assert_same(&Call::Bad, stdin.as_bytes(), &format!("bad oob idx {k}"));
    }
}

#[test]
fn row11_bad_eof_stdin() {
    ffi::assert_same(&Call::Bad, b"", "bad with empty stdin");
}

// ---------------------------------------------------------------------------
// Row 18–20: good() — exercises the static goodG2B and goodB2G
// ---------------------------------------------------------------------------

#[test]
fn row18_good_index_in_range_randomized() {
    for k in 0..10 {
        let stdin = format!("{k}\n");
        ffi::assert_same(&Call::Good, stdin.as_bytes(), &format!("good idx {k}"));
    }
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..120 {
        let k = rng.in_range(0, 9);
        let stdin = format!("{k}\n");
        ffi::assert_same(&Call::Good, stdin.as_bytes(), &format!("good idx {k}"));
    }
}

#[test]
fn row19_good_index_rejected_by_goodb2g() {
    // goodB2G's guard is `data >= 0 && data < 10`, so both the negative and the
    // too-large side must produce the out-of-bounds message -- and goodG2B must
    // still have printed its own ten values first.
    for k in [-1i64, -2, i32::MIN as i64, 10, 11, 15, 16, 26, 100, 100000, i32::MAX as i64] {
        let stdin = format!("{k}\n");
        ffi::assert_same(&Call::Good, stdin.as_bytes(), &format!("good rejected {k}"));
    }
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..150 {
        // Deliberately includes the huge indices: goodB2G is bounds-checked, so
        // unlike bad() these are all safe and must match exactly.
        let k = if rng.next_u64() % 2 == 0 {
            rng.in_range(10, i32::MAX as i64)
        } else {
            rng.in_range(i32::MIN as i64, -1)
        };
        let stdin = format!("{k}\n");
        ffi::assert_same(&Call::Good, stdin.as_bytes(), &format!("good rejected {k}"));
    }
}

#[test]
fn row20_good_eof_stdin() {
    ffi::assert_same(&Call::Good, b"", "good with empty stdin");
    ffi::assert_same(&Call::Good, b"\n", "good with blank line");
}

// ---------------------------------------------------------------------------
// Row 21–22: main() through the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn row21_main_with_args_randomized() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..80 {
        let a = rng.in_range(0, 9);
        let b = rng.in_range(0, 9);
        let stdin = format!("{a}\n{b}\n");
        ffi::assert_same(
            &Call::Main { with_args: true },
            stdin.as_bytes(),
            &format!("main({a},{b})"),
        );
    }
}

#[test]
fn row22_main_with_null_argv() {
    // The C body never reads argc/argv, so argc=0/argv=NULL must behave the same.
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..40 {
        let a = rng.in_range(0, 9);
        let b = rng.in_range(0, 9);
        let stdin = format!("{a}\n{b}\n");
        let (c_with, r_with) = ffi::both(&Call::Main { with_args: true }, stdin.as_bytes());
        let (c_without, r_without) = ffi::both(&Call::Main { with_args: false }, stdin.as_bytes());
        assert_eq!(c_with, r_with, "main with args diverged for {a},{b}");
        assert_eq!(c_without, r_without, "main with NULL argv diverged for {a},{b}");
        // And argc/argv genuinely must not matter, in either implementation.
        assert_eq!(c_with, c_without, "C main depended on argv (it must not)");
        assert_eq!(r_with, r_without, "Rust main depended on argv (it must not)");
    }
    ffi::assert_same(&Call::Main { with_args: false }, b"", "main NULL argv, EOF");
}
