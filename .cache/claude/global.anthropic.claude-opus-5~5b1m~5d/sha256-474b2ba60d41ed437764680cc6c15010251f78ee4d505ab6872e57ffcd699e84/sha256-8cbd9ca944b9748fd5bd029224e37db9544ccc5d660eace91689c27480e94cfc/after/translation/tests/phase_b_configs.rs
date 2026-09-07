//! Phase B — valid-path differential tests.
//!
//! One test (or one clearly-labelled section) per row of `CONFIGS.md`.
//! Everything goes through the exported `.so` symbols of BOTH libraries.

mod harness;

use harness::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Row 1..5 — printHexCharLine over the whole char domain (axis F)
// ---------------------------------------------------------------------------

/// CONFIGS row 1: exhaustive over all 256 `char` bit patterns.
#[test]
fn row01_hex_all_256_char_values() {
    for v in i16::from(i8::MIN)..=i16::from(i8::MAX) {
        let v = v as i8;
        diff(&format!("printHexCharLine({v})"), |lib| unsafe {
            sym_hex(lib)(v as c_char)
        });
    }
}

/// CONFIGS row 2: zero boundary.
#[test]
fn row02_hex_zero() {
    diff_eq("printHexCharLine(0)", b"00\n", |lib| unsafe {
        sym_hex(lib)(0)
    });
}

/// CONFIGS row 3: 1..=15, the zero-padding band.
#[test]
fn row03_hex_zero_pad_band() {
    for v in 1i8..=15 {
        let expect = format!("0{v:x}\n");
        diff_eq(
            &format!("printHexCharLine({v}) pad band"),
            expect.as_bytes(),
            |lib| unsafe { sym_hex(lib)(v as c_char) },
        );
    }
}

/// CONFIGS row 4: 16..=127, plain two-digit band.
#[test]
fn row04_hex_two_digit_band() {
    for v in 16i8..=127 {
        let expect = format!("{v:02x}\n");
        diff_eq(
            &format!("printHexCharLine({v}) 2-digit band"),
            expect.as_bytes(),
            |lib| unsafe { sym_hex(lib)(v as c_char) },
        );
    }
}

/// CONFIGS row 5: -128..=-1 — signed `char` promotes to a negative `int`, which
/// `%02x` reinterprets as `unsigned int`, so EIGHT hex digits are printed.
#[test]
fn row05_hex_sign_extension_band() {
    for v in -128i16..=-1 {
        let v = v as i8;
        let expect = format!("{:08x}\n", (v as i32) as u32);
        diff_eq(
            &format!("printHexCharLine({v}) sign-extend band"),
            expect.as_bytes(),
            |lib| unsafe { sym_hex(lib)(v as c_char) },
        );
    }
}

/// CONFIGS row 6: randomized long sequences in a single capture (buffering /
/// ordering, not just single values).
#[test]
fn row06_hex_randomized_sequences() {
    let mut rng = Rng::new();
    for batch in 0..64 {
        let vals: Vec<i8> = (0..256).map(|_| rng.next_u8() as i8).collect();
        let out = diff(&format!("hex random batch {batch}"), |lib| unsafe {
            let f = sym_hex(lib);
            for &v in &vals {
                f(v as c_char);
            }
        });
        assert!(!out.is_empty(), "capture unexpectedly empty");
    }
}

// ---------------------------------------------------------------------------
// Row 7..14 — printLine payload shapes (axis E/G)
// ---------------------------------------------------------------------------

/// CONFIGS row 7: empty string.
#[test]
fn row07_line_empty_string() {
    let buf = cbuf(b"");
    diff_eq("printLine(\"\")", b"\n", |lib| unsafe {
        sym_line(lib)(buf.as_ptr() as *const c_char)
    });
}

/// CONFIGS row 8: every single non-NUL byte value.
#[test]
fn row08_line_single_byte_all_values() {
    for b in 1u8..=255 {
        let buf = cbuf(&[b]);
        diff_eq(
            &format!("printLine(single byte {b:#04x})"),
            &[b, b'\n'],
            |lib| unsafe { sym_line(lib)(buf.as_ptr() as *const c_char) },
        );
    }
}

/// CONFIGS row 9: randomized printable-ASCII strings, lengths 0..=64.
#[test]
fn row09_line_randomized_ascii() {
    let mut rng = Rng::new();
    for i in 0..2000 {
        let len = rng.below(65) as usize;
        let s = rand_ascii(&mut rng, len);
        let buf = cbuf(&s);
        let mut expect = s.clone();
        expect.push(b'\n');
        diff_eq(
            &format!("printLine(random ascii #{i}, len {len})"),
            &expect,
            |lib| unsafe { sym_line(lib)(buf.as_ptr() as *const c_char) },
        );
    }
}

/// CONFIGS row 10: randomized arbitrary non-NUL bytes (incl. non-UTF-8 high
/// bytes) — the C code is byte-transparent and Rust must not validate UTF-8.
#[test]
fn row10_line_randomized_raw_bytes() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 0xAAAA);
    for i in 0..2000 {
        let len = 1 + rng.below(128) as usize;
        let s = rand_bytes_no_nul(&mut rng, len);
        let buf = cbuf(&s);
        let mut expect = s.clone();
        expect.push(b'\n');
        diff_eq(
            &format!("printLine(random raw #{i}, len {len})"),
            &expect,
            |lib| unsafe { sym_line(lib)(buf.as_ptr() as *const c_char) },
        );
    }
}

/// CONFIGS row 11: payloads larger than the stdio buffer.
#[test]
fn row11_line_long_payloads() {
    for &len in &[1024usize, 4096, 4095, 4097, 65536, 100_000] {
        let s: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let buf = cbuf(&s);
        let mut expect = s.clone();
        expect.push(b'\n');
        diff_eq(
            &format!("printLine(long len {len})"),
            &expect,
            |lib| unsafe { sym_line(lib)(buf.as_ptr() as *const c_char) },
        );
    }
}

/// CONFIGS row 12: `printf` conversion specifiers must be treated as DATA.
#[test]
fn row12_line_format_specifiers_are_data() {
    let payloads: &[&[u8]] = &[
        b"%s",
        b"%d",
        b"%n",
        b"%%",
        b"%p",
        b"%1000000d",
        b"%s%s%s%s%s%s%s%s",
        b"100%% sure",
        b"a%nb%sc",
    ];
    for p in payloads {
        let buf = cbuf(p);
        let mut expect = p.to_vec();
        expect.push(b'\n');
        diff_eq(
            &format!("printLine(fmt-as-data {:?})", String::from_utf8_lossy(p)),
            &expect,
            |lib| unsafe { sym_line(lib)(buf.as_ptr() as *const c_char) },
        );
    }
}

/// CONFIGS row 13: interior NUL — output must stop at the first NUL.
#[test]
fn row13_line_interior_nul_truncates() {
    let raw = b"abc\0def";
    let buf = cbuf(raw); // "abc\0def\0"
    diff_eq("printLine(\"abc\\0def\")", b"abc\n", |lib| unsafe {
        sym_line(lib)(buf.as_ptr() as *const c_char)
    });

    // NUL as the very first byte behaves like the empty string.
    let buf2 = cbuf(b"\0tail");
    diff_eq("printLine(\"\\0tail\")", b"\n", |lib| unsafe {
        sym_line(lib)(buf2.as_ptr() as *const c_char)
    });
}

/// CONFIGS row 14: randomized sequence of 100 mixed-shape strings per capture.
#[test]
fn row14_line_randomized_mixed_sequences() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 0x1414);
    for batch in 0..40 {
        // Pre-build the buffers so both libraries see identical input.
        let bufs: Vec<Vec<u8>> = (0..100)
            .map(|_| match rng.below(4) {
                0 => cbuf(b""),
                1 => {
                    let n = rng.below(40) as usize;
                    cbuf(&rand_ascii(&mut rng, n))
                }
                2 => {
                    let n = 1 + rng.below(80) as usize;
                    cbuf(&rand_bytes_no_nul(&mut rng, n))
                }
                _ => cbuf(b"%s %d %n literal"),
            })
            .collect();
        diff(&format!("printLine mixed batch {batch}"), |lib| unsafe {
            let f = sym_line(lib);
            for b in &bufs {
                f(b.as_ptr() as *const c_char);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Row 15..18 — bad() / good() (axes C, D, H)
// ---------------------------------------------------------------------------

/// CONFIGS row 15: `bad()` — CHAR_MAX*2 truncated to `char` == -2, which then
/// sign-extends through `%02x` to eight hex digits.
#[test]
fn row15_bad_single_call() {
    diff_eq("bad()", b"fffffffe\n", |lib| unsafe { sym_bad(lib)() });
}

/// CONFIGS row 16: `bad()` repeated in one capture.
#[test]
fn row16_bad_repeated() {
    let expect: Vec<u8> = b"fffffffe\n".repeat(50);
    diff_eq("bad() x50", &expect, |lib| unsafe {
        let f = sym_bad(lib);
        for _ in 0..50 {
            f();
        }
    });
}

const GOOD_OUT: &[u8] = b"04\ndata value is too large to perform arithmetic safely.\n";

/// CONFIGS row 17: `good()` == goodG2B() then goodB2G(); ordering matters.
#[test]
fn row17_good_single_call() {
    diff_eq("good()", GOOD_OUT, |lib| unsafe { sym_good(lib)() });
}

/// CONFIGS row 18: `good()` repeated in one capture.
#[test]
fn row18_good_repeated() {
    let expect: Vec<u8> = GOOD_OUT.repeat(50);
    diff_eq("good() x50", &expect, |lib| unsafe {
        let f = sym_good(lib);
        for _ in 0..50 {
            f();
        }
    });
}

// ---------------------------------------------------------------------------
// Row 19..23 — driver() (axis B)
// ---------------------------------------------------------------------------

/// CONFIGS row 19: `driver(0)` -> bad().
#[test]
fn row19_driver_zero() {
    diff_eq("driver(0)", b"fffffffe\n", |lib| unsafe {
        sym_driver(lib)(0)
    });
}

/// CONFIGS row 20: `driver(1)` -> good().
#[test]
fn row20_driver_one() {
    diff_eq("driver(1)", GOOD_OUT, |lib| unsafe { sym_driver(lib)(1) });
}

/// CONFIGS row 21: non-zero edge values, including ones whose low byte is zero
/// (probing for a narrowing bug that would misread them as false).
#[test]
fn row21_driver_nonzero_edges() {
    let vals: &[i32] = &[
        -1,
        2,
        -2,
        3,
        i32::MAX,
        i32::MIN,
        0x100,
        0x1_0000,
        0x0100_0000,
        0x7FFF_FF00,
        -0x100,
        -0x1_0000,
        0xFF,
        -0x8000_0000i64 as i32,
        1 << 30,
    ];
    for &v in vals {
        diff_eq(&format!("driver({v})"), GOOD_OUT, |lib| unsafe {
            sym_driver(lib)(v as c_int)
        });
    }
}

/// CONFIGS row 22: 4096 randomized `i32` values (mixed zero / non-zero).
#[test]
fn row22_driver_randomized_i32() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 0x2222);
    for i in 0..4096 {
        // Bias every 8th value to 0 so the false branch is well covered, and
        // sprinkle in low-byte-zero values.
        let v = match i % 8 {
            0 => 0,
            1 => (rng.next_i32() & !0xFF) as i32,
            2 => rng.next_i32() >> (rng.below(31) as u32),
            _ => rng.next_i32(),
        };
        let expect: &[u8] = if v == 0 { b"fffffffe\n" } else { GOOD_OUT };
        diff_eq(&format!("driver(random {v})"), expect, |lib| unsafe {
            sym_driver(lib)(v as c_int)
        });
    }
}

/// CONFIGS row 23: randomized interleaved `driver` calls in a single capture.
#[test]
fn row23_driver_randomized_sequences() {
    let mut rng = Rng::with_seed(Rng::SEED ^ 0x2323);
    for batch in 0..40 {
        let vals: Vec<i32> = (0..64)
            .map(|_| if rng.below(3) == 0 { 0 } else { rng.next_i32() })
            .collect();
        let mut expect = Vec::new();
        for &v in &vals {
            expect.extend_from_slice(if v == 0 { b"fffffffe\n" } else { GOOD_OUT });
        }
        diff_eq(
            &format!("driver sequence batch {batch}"),
            &expect,
            |lib| unsafe {
                let f = sym_driver(lib);
                for &v in &vals {
                    f(v as c_int);
                }
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 24 — all five exports interleaved (composed pipeline)
// ---------------------------------------------------------------------------

/// CONFIGS row 24: randomized interleaving of every public entry point in a
/// single capture. Per-function tests cannot see ordering / buffering bugs in
/// the composed pipeline.
#[test]
fn row24_all_exports_interleaved() {
    #[derive(Clone)]
    enum Op {
        Driver(i32),
        Good,
        Bad,
        Hex(i8),
        Line(Vec<u8>),
        LineNull,
    }

    let mut rng = Rng::with_seed(Rng::SEED ^ 0x2424);
    for batch in 0..50 {
        let ops: Vec<Op> = (0..80)
            .map(|_| match rng.below(6) {
                0 => Op::Driver(if rng.below(2) == 0 { 0 } else { rng.next_i32() }),
                1 => Op::Good,
                2 => Op::Bad,
                3 => Op::Hex(rng.next_u8() as i8),
                4 => {
                    let n = rng.below(48) as usize;
                    Op::Line(cbuf(&rand_bytes_no_nul(&mut rng, n)))
                }
                _ => Op::LineNull,
            })
            .collect();

        diff(&format!("interleaved batch {batch}"), |lib| unsafe {
            let fd = sym_driver(lib);
            let fg = sym_good(lib);
            let fb = sym_bad(lib);
            let fh = sym_hex(lib);
            let fl = sym_line(lib);
            for op in &ops {
                match op {
                    Op::Driver(v) => fd(*v as c_int),
                    Op::Good => fg(),
                    Op::Bad => fb(),
                    Op::Hex(v) => fh(*v as c_char),
                    Op::Line(b) => fl(b.as_ptr() as *const c_char),
                    Op::LineNull => fl(std::ptr::null()),
                }
            }
        });
    }
}
