// Phase B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH the C `.so` and the
// Rust `.so` through their exported symbols and compares captured stdout
// byte-for-byte. Rows marked "(random)" in CONFIGS.md use many property-style
// inputs generated from the fixed seed `common::SEED`.
//
// Rows 17 and 18 (the `bad()` / `driver(0)` UB rows) live in `ub_bad.rs`.

mod common;

use common::*;
use std::ffi::c_char;

// ---------------------------------------------------------------------------
// Row 1 -- printLine, empty string (length-0 boundary)
// ---------------------------------------------------------------------------

#[test]
fn row1_printline_empty_string() {
    let buf = cstr(b"");
    diff("row1 printLine(\"\")", |imp| imp.print_line(buf.as_ptr() as *const c_char));

    // Sanity-anchor the shared behaviour against the C source's intent:
    // `printf("%s\n", "")` emits exactly one newline.
    let out = capture_stdout(|| c_default().print_line(buf.as_ptr() as *const c_char));
    assert_eq!(out, b"\n", "C should emit exactly one newline for \"\"");
}

// ---------------------------------------------------------------------------
// Row 2 -- printLine, every possible single non-NUL byte (exhaustive)
// ---------------------------------------------------------------------------

#[test]
fn row2_printline_all_single_bytes_exhaustive() {
    for b in 1u8..=255 {
        let buf = cstr(&[b]);
        diff(&format!("row2 printLine(single byte 0x{b:02x})"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 3 -- printLine, random printable ASCII, lengths 2..=64
// ---------------------------------------------------------------------------

#[test]
fn row3_printline_random_ascii() {
    let mut rng = Rng::new(SEED ^ 3);
    for case in 0..400 {
        let len = 2 + rng.below(63);
        let payload: Vec<u8> = (0..len).map(|_| rng.byte_in(0x20, 0x7e)).collect();
        let buf = cstr(&payload);
        diff(&format!("row3 case={case} len={len}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 4 -- printLine, printf conversion specifiers must stay literal
// ---------------------------------------------------------------------------

#[test]
fn row4_printline_format_specifiers_are_literal() {
    // The C side reaches `puts(line)`; the Rust side reaches
    // `printf("%s\n", line)`. If the Rust translation ever passed `line` as the
    // *format* string, these payloads would diverge (or crash on `%n`).
    const NASTY: &[&[u8]] = &[
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%p",
        b"%1000d",
        b"%999999999s",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"100%% done",
        b"printf(\"%s\\n\", line)",
        b"%.*s",
        b"%-+ #0hlLqjzt%s",
        b"%",
    ];
    for (i, p) in NASTY.iter().enumerate() {
        let buf = cstr(p);
        diff(&format!("row4 fixed[{i}]={:?}", String::from_utf8_lossy(p)), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }

    // Randomized: specifiers interleaved with ordinary text.
    let mut rng = Rng::new(SEED ^ 4);
    const TOK: &[&[u8]] = &[b"%s", b"%d", b"%n", b"%%", b"%p", b"%x", b"abc", b" ", b"%5.2f"];
    for case in 0..200 {
        let n = 1 + rng.below(12);
        let mut payload = Vec::new();
        for _ in 0..n {
            payload.extend_from_slice(TOK[rng.below(TOK.len())]);
        }
        let buf = cstr(&payload);
        diff(&format!("row4 random case={case}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 5 -- printLine, embedded control bytes
// ---------------------------------------------------------------------------

#[test]
fn row5_printline_control_bytes() {
    const CTRL: &[u8] = &[b'\n', b'\r', b'\t', 0x01, 0x1b, 0x7f, 0x0b, 0x0c, 0x08];

    for &c in CTRL {
        let buf = cstr(&[b'a', c, b'b']);
        diff(&format!("row5 fixed ctrl=0x{c:02x}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }

    let mut rng = Rng::new(SEED ^ 5);
    for case in 0..200 {
        let len = 1 + rng.below(48);
        let payload: Vec<u8> = (0..len)
            .map(|_| {
                if rng.next_u32() % 2 == 0 {
                    CTRL[rng.below(CTRL.len())]
                } else {
                    rng.byte_in(0x20, 0x7e)
                }
            })
            .collect();
        let buf = cstr(&payload);
        diff(&format!("row5 random case={case} len={len}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 6 -- printLine, high / non-UTF-8 bytes
// ---------------------------------------------------------------------------

#[test]
fn row6_printline_high_bytes() {
    // Deliberately invalid UTF-8 sequences: a Rust translation that routed the
    // buffer through `str`/`String` would reject or mangle these.
    const INVALID_UTF8: &[&[u8]] = &[
        &[0xff],
        &[0xfe, 0xff],
        &[0x80],
        &[0xc3],             // truncated 2-byte sequence
        &[0xe2, 0x82],       // truncated 3-byte sequence
        &[0xf0, 0x9f, 0x92], // truncated 4-byte sequence
        &[0xc0, 0x80],       // overlong encoding of NUL
        &[0xed, 0xa0, 0x80], // UTF-16 surrogate half
        &[0xf5, 0x80, 0x80, 0x80],
    ];
    for (i, p) in INVALID_UTF8.iter().enumerate() {
        let buf = cstr(p);
        diff(&format!("row6 invalid-utf8[{i}]"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }

    let mut rng = Rng::new(SEED ^ 6);
    for case in 0..200 {
        let len = 1 + rng.below(48);
        let payload: Vec<u8> = (0..len).map(|_| rng.byte_in(0x80, 0xff)).collect();
        let buf = cstr(&payload);
        diff(&format!("row6 random case={case} len={len}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 7 -- printLine, full byte range 0x01..=0xff, lengths 1..=256
// ---------------------------------------------------------------------------

#[test]
fn row7_printline_full_byte_range() {
    let mut rng = Rng::new(SEED ^ 7);
    for case in 0..600 {
        let len = 1 + rng.below(256);
        let payload: Vec<u8> = (0..len).map(|_| rng.byte_in(0x01, 0xff)).collect();
        let buf = cstr(&payload);
        diff(&format!("row7 case={case} len={len}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }

    // Every non-NUL byte exactly once, in one long string.
    let all: Vec<u8> = (1u8..=255).collect();
    let buf = cstr(&all);
    diff("row7 all-255-bytes-in-one-string", |imp| {
        imp.print_line(buf.as_ptr() as *const c_char)
    });
}

// ---------------------------------------------------------------------------
// Row 8 -- printLine, long strings crossing the stdio buffer
// ---------------------------------------------------------------------------

#[test]
fn row8_printline_oversized_strings() {
    let mut rng = Rng::new(SEED ^ 8);
    for &len in &[1024usize, 4096, 8192, 65536, 262144] {
        let payload: Vec<u8> = (0..len).map(|_| rng.byte_in(0x01, 0xff)).collect();
        let buf = cstr(&payload);
        diff(&format!("row8 len={len}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });

        // Confirm nothing is truncated by either side.
        let out = capture_stdout(|| rust().print_line(buf.as_ptr() as *const c_char));
        assert_eq!(out.len(), len + 1, "row8 len={len}: unexpected output length");
    }
}

// ---------------------------------------------------------------------------
// Row 9 -- printLine, lengths exactly at buffer boundaries
// ---------------------------------------------------------------------------

#[test]
fn row9_printline_buffer_boundary_lengths() {
    let mut rng = Rng::new(SEED ^ 9);
    let mut lens: Vec<usize> = Vec::new();
    for base in [512usize, 1024, 4096, 8192] {
        lens.extend_from_slice(&[base - 1, base, base + 1]);
    }
    // BUFSIZ on glibc is 8192, but check the classic 1024/4096 pipe sizes too.
    for len in lens {
        let payload: Vec<u8> = (0..len).map(|_| rng.byte_in(0x20, 0x7e)).collect();
        let buf = cstr(&payload);
        diff(&format!("row9 boundary len={len}"), |imp| {
            imp.print_line(buf.as_ptr() as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 10 -- printLine(NULL) (valid-path side of the one rejection branch)
// ---------------------------------------------------------------------------

#[test]
fn row10_printline_null() {
    diff("row10 printLine(NULL)", |imp| imp.print_line(std::ptr::null()));
}

// ---------------------------------------------------------------------------
// Rows 11 & 12 -- good()
// ---------------------------------------------------------------------------

#[test]
fn row11_good_single_call() {
    diff("row11 good()", |imp| imp.good());

    let out = capture_stdout(|| c_default().good());
    assert_eq!(out, b"string\n", "C good() must emit the literal \"string\\n\"");
}

#[test]
fn row12_good_repeated() {
    diff("row12 good() x50", |imp| {
        for _ in 0..50 {
            imp.good();
        }
    });
}

// ---------------------------------------------------------------------------
// Rows 13-16 -- driver() with non-zero `useGood`
// ---------------------------------------------------------------------------

#[test]
fn row13_driver_one() {
    diff("row13 driver(1)", |imp| imp.driver(1));
}

#[test]
fn row14_driver_random_positive() {
    let mut rng = Rng::new(SEED ^ 14);
    for case in 0..300 {
        // Non-zero positive i32.
        let v = 1 + (rng.next_u32() % (i32::MAX as u32)) as i32;
        diff(&format!("row14 case={case} driver({v})"), |imp| imp.driver(v));
    }
}

#[test]
fn row15_driver_random_negative() {
    let mut rng = Rng::new(SEED ^ 15);
    for case in 0..300 {
        // Non-zero negative i32, including the possibility of i32::MIN.
        let v = -1 - (rng.next_u32() % (i32::MAX as u32 + 1)) as i64;
        let v = v.max(i32::MIN as i64) as i32;
        assert_ne!(v, 0);
        diff(&format!("row15 case={case} driver({v})"), |imp| imp.driver(v));
    }
}

#[test]
fn row16_driver_boundary_values() {
    // `if (useGood)` is true for EVERY non-zero int. A translation using
    // `useGood > 0` would fail on the negative values here.
    let vals: &[i32] = &[
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        0x8000_0000u32 as i32,
        0x7fff_ffff,
        0x0000_0100,
        0x0001_0000,
        -0x8000_0000i64 as i32,
    ];
    for &v in vals {
        assert_ne!(v, 0, "row16 values must all be non-zero");
        diff(&format!("row16 driver({v})"), |imp| imp.driver(v));
    }
}

// ---------------------------------------------------------------------------
// Rows 19 & 20 -- composed pipeline / ordering
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Op {
    PrintLine(Vec<u8>),
    PrintLineNull,
    Good,
    DriverGood(i32),
}

fn replay(imp: &Impl, ops: &[Op]) {
    for op in ops {
        match op {
            Op::PrintLine(b) => imp.print_line(b.as_ptr() as *const c_char),
            Op::PrintLineNull => imp.print_line(std::ptr::null()),
            Op::Good => imp.good(),
            Op::DriverGood(v) => imp.driver(*v),
        }
    }
}

#[test]
fn row19_composed_random_sequences() {
    // Drives the library the way a real consumer does: many entry points
    // interleaved into ONE captured stream, so output ordering and the shared
    // stdout buffering of the composed pipeline are verified -- something the
    // per-call rows above cannot observe.
    //
    // `bad()` / `driver(0)` are excluded here by design: they are undefined
    // behaviour at -O0 (see ERRORS.md §UB) and are covered in `ub_bad.rs`.
    let mut rng = Rng::new(SEED ^ 19);
    for seq in 0..100 {
        let n = 20;
        let mut ops = Vec::with_capacity(n);
        for _ in 0..n {
            ops.push(match rng.below(4) {
                0 => {
                    let len = rng.below(40);
                    let payload: Vec<u8> = (0..len).map(|_| rng.byte_in(0x01, 0xff)).collect();
                    Op::PrintLine(cstr(&payload))
                }
                1 => Op::PrintLineNull,
                2 => Op::Good,
                _ => {
                    let v = 1 + (rng.next_u32() % 1000) as i32;
                    Op::DriverGood(v)
                }
            });
        }
        assert_same(c_default(), rust(), &format!("row19 seq={seq}"), |imp| {
            replay(imp, &ops)
        });
    }
}

#[test]
fn row20_no_residual_state_between_entry_points() {
    // printLine immediately after good()/driver(1): checks there is no residual
    // state or interleaving artifact between entry points.
    let payload = cstr(b"after-good-payload");
    let combos: Vec<Vec<Op>> = vec![
        vec![Op::Good, Op::PrintLine(payload.clone())],
        vec![Op::DriverGood(1), Op::PrintLine(payload.clone())],
        vec![Op::PrintLine(payload.clone()), Op::Good],
        vec![Op::PrintLineNull, Op::Good, Op::PrintLineNull],
        vec![Op::Good, Op::PrintLineNull, Op::DriverGood(-1), Op::PrintLine(payload.clone())],
        vec![Op::DriverGood(i32::MIN), Op::PrintLine(payload.clone()), Op::Good],
    ];
    for (i, ops) in combos.iter().enumerate() {
        assert_same(c_default(), rust(), &format!("row20 combo={i}"), |imp| {
            replay(imp, ops)
        });
    }
}
