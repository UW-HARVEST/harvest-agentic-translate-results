// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH `.so`s through their
// exported symbols and asserts byte-identical `stdout`. Randomized rows use a
// fixed seed so failures are reproducible.
//
// Ordered lowest-level entry point first (printIntLine, printLine), then the
// mid-level fns (bad, good), then the top-level wrapper (driver), then the
// composed pipeline.

#![allow(non_snake_case)]

mod harness;
use harness::{Api, Rng, cstr, diff};

// ===========================================================================
// printIntLine — lowest-level leaf (CONFIGS rows 1-6)
// ===========================================================================

/// CONFIGS row 1 — `printIntLine(0)`.
#[test]
fn cfg_01_print_int_line_zero() {
    let out = diff("printIntLine(0)", |a: &Api| a.print_int_line(0));
    assert_eq!(out, b"0\n", "C must print the %d rendering of 0");
}

/// CONFIGS row 2 — small positive values.
#[test]
fn cfg_02_print_int_line_small_positive() {
    for v in 1..=9i32 {
        let out = diff(&format!("printIntLine({v})"), |a: &Api| a.print_int_line(v));
        assert_eq!(out, format!("{v}\n").into_bytes());
    }
}

/// CONFIGS row 3 — small negative values.
#[test]
fn cfg_03_print_int_line_small_negative() {
    for v in -9..=-1i32 {
        let out = diff(&format!("printIntLine({v})"), |a: &Api| a.print_int_line(v));
        assert_eq!(out, format!("{v}\n").into_bytes());
    }
}

/// CONFIGS row 4 — `int` boundary values.
#[test]
fn cfg_04_print_int_line_boundaries() {
    for v in [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, -1, 0, 1] {
        let out = diff(&format!("printIntLine({v})"), |a: &Api| a.print_int_line(v));
        assert_eq!(out, format!("{v}\n").into_bytes());
    }
}

/// CONFIGS row 5 — 512 randomized full-range `i32` (seeded).
#[test]
fn cfg_05_print_int_line_randomized() {
    let mut rng = Rng::new(0xC0FFEE_01);
    for _ in 0..512 {
        let v = rng.next_i32();
        let out = diff(&format!("printIntLine({v}) [rand]"), |a: &Api| a.print_int_line(v));
        assert_eq!(out, format!("{v}\n").into_bytes());
    }
}

/// CONFIGS row 6 — digit-width sweep: every `%d` field width, both signs.
#[test]
fn cfg_06_print_int_line_digit_width_sweep() {
    let mut vals: Vec<i32> = Vec::new();
    let mut p: i64 = 1;
    for _ in 0..10 {
        for cand in [p, p - 1, -p, -(p - 1)] {
            if cand >= i32::MIN as i64 && cand <= i32::MAX as i64 {
                vals.push(cand as i32);
            }
        }
        p *= 10;
    }
    for v in vals {
        diff(&format!("printIntLine({v}) [width]"), |a: &Api| a.print_int_line(v));
    }
}

// ===========================================================================
// printLine — lowest-level leaf (CONFIGS rows 7-14)
// ===========================================================================

/// CONFIGS row 7 — NULL pointer takes the false side of the `driver.c:32` guard.
#[test]
fn cfg_07_print_line_null() {
    let out = diff("printLine(NULL)", |a: &Api| a.print_line(std::ptr::null()));
    assert!(out.is_empty(), "NULL must produce no output at all, got {:?}", out);
}

/// CONFIGS row 8 — empty string.
#[test]
fn cfg_08_print_line_empty() {
    let s = cstr(b"");
    let out = diff("printLine(\"\")", |a: &Api| a.print_line(s.as_ptr()));
    assert_eq!(out, b"\n");
}

/// CONFIGS row 9 — every single-byte payload (all 255 non-NUL bytes).
#[test]
fn cfg_09_print_line_single_byte() {
    for b in 1u8..=255 {
        let s = cstr(&[b]);
        let out = diff(&format!("printLine(single byte {b:#04x})"), |a: &Api| {
            a.print_line(s.as_ptr())
        });
        assert_eq!(out, vec![b, b'\n']);
    }
}

/// CONFIGS row 10 — many randomized short ASCII strings (seeded).
#[test]
fn cfg_10_print_line_randomized_short() {
    let mut rng = Rng::new(0xC0FFEE_02);
    for _ in 0..256 {
        let len = rng.below(32) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| 0x20 + (rng.below(95) as u8)).collect();
        let s = cstr(&bytes);
        let out = diff("printLine(rand ascii)", |a: &Api| a.print_line(s.as_ptr()));
        let mut want = bytes.clone();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

/// CONFIGS row 11 — length sweep across the stdio buffer boundaries, up to 64 KiB.
#[test]
fn cfg_11_print_line_length_sweep() {
    let mut rng = Rng::new(0xC0FFEE_03);
    let mut lens: Vec<usize> = vec![1, 2, 3, 127, 128, 255, 256, 1023, 1024, 4095, 4096, 4097];
    lens.extend([8191, 8192, 8193, 16384, 32768, 65535, 65536]);
    for len in lens {
        let bytes = rng.cstring_bytes(len);
        let s = cstr(&bytes);
        let out = diff(&format!("printLine(len={len})"), |a: &Api| a.print_line(s.as_ptr()));
        assert_eq!(out.len(), len + 1, "len={len}");
        assert_eq!(&out[..len], &bytes[..]);
        assert_eq!(out[len], b'\n');
    }
}

/// CONFIGS row 12 — payload full of `printf` conversion specifiers. `line` is a
/// data argument to `"%s\n"`, so it must be echoed, never interpreted.
#[test]
fn cfg_12_print_line_format_specifiers() {
    for p in [
        &b"%s"[..],
        &b"%d"[..],
        &b"%n"[..],
        &b"%%"[..],
        &b"%s %d %n %% %p %x"[..],
        &b"%999999999d"[..],
        &b"100% done"[..],
    ] {
        let s = cstr(p);
        let out = diff("printLine(format specifiers)", |a: &Api| a.print_line(s.as_ptr()));
        let mut want = p.to_vec();
        want.push(b'\n');
        assert_eq!(out, want, "payload must be echoed verbatim, not interpreted");
    }
}

/// CONFIGS row 13 — high / non-ASCII bytes, including invalid UTF-8 sequences.
#[test]
fn cfg_13_print_line_high_bytes() {
    // Every non-NUL byte value in one payload (invalid UTF-8 on purpose).
    let all: Vec<u8> = (1u8..=255).collect();
    let s = cstr(&all);
    let out = diff("printLine(all 255 bytes)", |a: &Api| a.print_line(s.as_ptr()));
    let mut want = all.clone();
    want.push(b'\n');
    assert_eq!(out, want);

    for p in [
        &b"\xff\xfe\xfd"[..],
        &b"\x80\x80\x80"[..],
        &b"\xc3\x28"[..],   // invalid 2-byte sequence
        &b"\xe2\x82"[..],   // truncated 3-byte sequence
        &b"caf\xc3\xa9"[..] // valid UTF-8
    ] {
        let s = cstr(p);
        let out = diff("printLine(high bytes)", |a: &Api| a.print_line(s.as_ptr()));
        let mut want = p.to_vec();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

/// CONFIGS row 14 — embedded control characters.
#[test]
fn cfg_14_print_line_embedded_control_chars() {
    for p in [
        &b"a\nb"[..],
        &b"a\r\nb"[..],
        &b"a\tb"[..],
        &b"\n"[..],
        &b"\n\n\n"[..],
        &b"trailing\n"[..],
        &b"\x07\x08\x0b\x0c\x1b[31m"[..],
    ] {
        let s = cstr(p);
        let out = diff("printLine(control chars)", |a: &Api| a.print_line(s.as_ptr()));
        let mut want = p.to_vec();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

// ===========================================================================
// bad / good — mid-level, called DIRECTLY (CONFIGS rows 15-19)
// ===========================================================================

/// CONFIGS row 15 — `bad()` alone: the under-sized `alloca(10)` path.
#[test]
fn cfg_15_bad_direct_single() {
    let out = diff("bad()", |a: &Api| a.bad());
    assert_eq!(out, b"0\n", "data[0] is source[0] == 0");
}

/// CONFIGS row 16 — `good()` alone: the correctly-sized `alloca(40)` path.
#[test]
fn cfg_16_good_direct_single() {
    let out = diff("good()", |a: &Api| a.good());
    assert_eq!(out, b"0\n");
}

/// CONFIGS row 17 — `bad()` 256x. The 30-byte overrun must not accumulate into
/// observable state on either side.
#[test]
fn cfg_17_bad_repeated() {
    let out = diff("bad() x256", |a: &Api| {
        for _ in 0..256 {
            a.bad();
        }
    });
    assert_eq!(out, b"0\n".repeat(256));
}

/// CONFIGS row 18 — `good()` 256x.
#[test]
fn cfg_18_good_repeated() {
    let out = diff("good() x256", |a: &Api| {
        for _ in 0..256 {
            a.good();
        }
    });
    assert_eq!(out, b"0\n".repeat(256));
}

/// CONFIGS row 19 — seeded interleaving of `bad` and `good` in one stream.
#[test]
fn cfg_19_bad_good_interleaved() {
    let mut rng = Rng::new(0xC0FFEE_04);
    let plan: Vec<bool> = (0..512).map(|_| rng.next_u64() & 1 == 1).collect();
    let out = diff("bad/good interleaved x512", |a: &Api| {
        for &g in &plan {
            if g { a.good() } else { a.bad() }
        }
    });
    assert_eq!(out, b"0\n".repeat(512));
}

// ===========================================================================
// driver — top-level wrapper (CONFIGS rows 20-25)
// ===========================================================================

/// CONFIGS row 20 — `driver(0)` selects the `bad()` branch.
#[test]
fn cfg_20_driver_zero_selects_bad() {
    let out = diff("driver(0)", |a: &Api| a.driver(0));
    assert_eq!(out, b"0\n");
}

/// CONFIGS row 21 — `driver(1)` selects the `good()` branch.
#[test]
fn cfg_21_driver_one_selects_good() {
    let out = diff("driver(1)", |a: &Api| a.driver(1));
    assert_eq!(out, b"0\n");
}

/// CONFIGS row 22 — non-zero flags that are not 1: C truthiness, not `== 1`.
#[test]
fn cfg_22_driver_truthy_non_one() {
    for v in [2, -1, 7, 0x100, 0x7fff_ffff, i32::MIN, -2, 1000000] {
        let out = diff(&format!("driver({v})"), |a: &Api| a.driver(v));
        assert_eq!(out, b"0\n", "flag {v}");
    }
}

/// CONFIGS row 23 — 512 randomized full-range flags (seeded), mixing zero and
/// non-zero. Every 16th draw is forced to 0 so the zero branch is hit often.
#[test]
fn cfg_23_driver_randomized_flags() {
    let mut rng = Rng::new(0xC0FFEE_05);
    for i in 0..512 {
        let v = if i % 16 == 0 { 0 } else { rng.next_i32() };
        let out = diff(&format!("driver({v}) [rand]"), |a: &Api| a.driver(v));
        assert_eq!(out, b"0\n");
    }
}

/// CONFIGS row 24 — a randomized *sequence* of flags captured as one stream,
/// so any order-dependence or leaked state would show up.
#[test]
fn cfg_24_driver_randomized_sequence() {
    let mut rng = Rng::new(0xC0FFEE_06);
    let plan: Vec<i32> = (0..512)
        .map(|i| if i % 5 == 0 { 0 } else { rng.next_i32() })
        .collect();
    let out = diff("driver(seq) x512", |a: &Api| {
        for &v in &plan {
            a.driver(v);
        }
    });
    assert_eq!(out, b"0\n".repeat(512));
}

/// CONFIGS row 25 — wrapper/low-level equivalence: `driver(0)` must be
/// indistinguishable from `bad()`, and `driver(nonzero)` from `good()`, on both
/// implementations. Compares C-vs-Rust *and* wrapper-vs-leaf.
#[test]
fn cfg_25_driver_equivalent_to_direct_calls() {
    let d0 = diff("driver(0) == bad()", |a: &Api| a.driver(0));
    let b0 = diff("bad() [equiv]", |a: &Api| a.bad());
    assert_eq!(d0, b0, "driver(0) must compose bad() exactly");

    for v in [1, 2, -1, i32::MAX, i32::MIN] {
        let dv = diff(&format!("driver({v}) == good()"), |a: &Api| a.driver(v));
        let g = diff("good() [equiv]", |a: &Api| a.good());
        assert_eq!(dv, g, "driver({v}) must compose good() exactly");
    }
}

// ===========================================================================
// Composed pipeline across all five entry points (CONFIGS row 26)
// ===========================================================================

/// CONFIGS row 26 — 1024 seeded operations mixing all five exported symbols
/// into a single captured stream. Exercises the composed pipeline, where
/// per-function tests are blind.
#[test]
fn cfg_26_mixed_pipeline_randomized() {
    #[derive(Clone)]
    enum Op {
        Driver(i32),
        Bad,
        Good,
        Int(i32),
        Line(Option<Vec<u8>>),
    }

    let mut rng = Rng::new(0xC0FFEE_07);
    let mut plan: Vec<Op> = Vec::with_capacity(1024);
    for _ in 0..1024 {
        plan.push(match rng.below(5) {
            0 => Op::Driver(if rng.below(3) == 0 { 0 } else { rng.next_i32() }),
            1 => Op::Bad,
            2 => Op::Good,
            3 => Op::Int(rng.next_i32()),
            _ => {
                if rng.below(8) == 0 {
                    Op::Line(None)
                } else {
                    let len = rng.below(64) as usize;
                    Op::Line(Some(rng.cstring_bytes(len)))
                }
            }
        });
    }
    // Pre-build the CStrings so both runs pass identical pointers/content.
    let prepared: Vec<(Op, Option<std::ffi::CString>)> = plan
        .into_iter()
        .map(|op| {
            let cs = match &op {
                Op::Line(Some(b)) => Some(cstr(b)),
                _ => None,
            };
            (op, cs)
        })
        .collect();

    let out = diff("mixed pipeline x1024", |a: &Api| {
        for (op, cs) in &prepared {
            match op {
                Op::Driver(v) => a.driver(*v),
                Op::Bad => a.bad(),
                Op::Good => a.good(),
                Op::Int(v) => a.print_int_line(*v),
                Op::Line(None) => a.print_line(std::ptr::null()),
                Op::Line(Some(_)) => a.print_line(cs.as_ref().unwrap().as_ptr()),
            }
        }
    });
    assert!(!out.is_empty());
}
