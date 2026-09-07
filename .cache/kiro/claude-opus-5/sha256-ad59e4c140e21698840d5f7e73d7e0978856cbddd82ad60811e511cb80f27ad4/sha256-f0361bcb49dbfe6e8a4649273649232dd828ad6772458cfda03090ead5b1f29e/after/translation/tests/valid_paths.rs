//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through their exported symbols and
//! compares the raw stdout bytes. Randomized rows use the fixed seed
//! `common::SEED` so failures reproduce exactly.

mod common;

use common::{diff, named_floats, Api, Rng, SEED};
use std::ffi::{c_char, CString};

fn cs(s: &str) -> CString {
    CString::new(s).expect("test string contains an interior NUL")
}

fn call_print_line(api: &Api, bytes: &[u8]) {
    // `bytes` must already be NUL-terminated.
    unsafe { (api.print_line)(bytes.as_ptr() as *const c_char) }
}

// ---------------------------------------------------------------------------
// Row 1 — printLine, randomized short ASCII
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_print_line_random_ascii() {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _-.,;:!?";
    let mut rng = Rng::new(SEED);
    for i in 0..512 {
        let len = 1 + (rng.next_u32() % 64) as usize;
        let mut buf: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(rng.next_u32() as usize) % ALPHABET.len()])
            .collect();
        buf.push(0);
        diff(&format!("row1 printLine #{i} len={len}"), |api| {
            call_print_line(api, &buf)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 2 — printLine, empty string
// ---------------------------------------------------------------------------

#[test]
fn cfg_02_print_line_empty() {
    let s = cs("");
    diff("row2 printLine empty", |api| unsafe {
        (api.print_line)(s.as_ptr())
    });
}

// ---------------------------------------------------------------------------
// Row 3 — printLine, long strings crossing the stdio buffer
// ---------------------------------------------------------------------------

#[test]
fn cfg_03_print_line_long() {
    for &len in &[1usize, 2, 127, 128, 4095, 4096, 4097, 65536] {
        let mut buf: Vec<u8> = std::iter::repeat(b'X').take(len).collect();
        buf.push(0);
        diff(&format!("row3 printLine len={len}"), |api| {
            call_print_line(api, &buf)
        });
    }
    // A long string with varying content, so a chunking bug cannot hide behind
    // a uniform fill.
    let mut rng = Rng::new(SEED ^ 3);
    let mut buf: Vec<u8> = (0..70_000)
        .map(|_| b'!' + (rng.next_u32() % 90) as u8)
        .collect();
    buf.push(0);
    diff("row3 printLine long varied", |api| call_print_line(api, &buf));
}

// ---------------------------------------------------------------------------
// Row 4 — printLine with printf directives in the *argument*
// ---------------------------------------------------------------------------

#[test]
fn cfg_04_print_line_format_directives() {
    for s in [
        "%d", "%s", "%n", "%%", "%p", "%99999999d", "%.*s", "%hhn",
        "100% done", "a%sb%dc%nd", "%s%s%s%s%s%s%s%s%s%s",
    ] {
        let c = cs(s);
        diff(&format!("row4 printLine {s:?}"), |api| unsafe {
            (api.print_line)(c.as_ptr())
        });
    }
}

// ---------------------------------------------------------------------------
// Row 5 — printLine with embedded control bytes and high bytes
// ---------------------------------------------------------------------------

#[test]
fn cfg_05_print_line_control_and_high_bytes() {
    let mut cases: Vec<Vec<u8>> = vec![
        b"line1\nline2\0".to_vec(),
        b"tab\there\0".to_vec(),
        b"cr\rhere\0".to_vec(),
        b"bell\x07\0".to_vec(),
        b"esc\x1b[31mred\x1b[0m\0".to_vec(),
        "utf8: \u{00e9}\u{4e2d}\u{6587}\u{1f600}\0".as_bytes().to_vec(),
    ];
    // Every single non-NUL byte value, one per call.
    for b in 1u8..=255 {
        cases.push(vec![b, 0]);
    }
    // A run of all high bytes (invalid UTF-8) — printf is byte-oriented here.
    let mut high: Vec<u8> = (0x80u8..=0xFF).collect();
    high.push(0);
    cases.push(high);

    for (i, buf) in cases.iter().enumerate() {
        diff(&format!("row5 printLine case#{i}"), |api| {
            call_print_line(api, buf)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 6 — printIntLine, named extremes
// ---------------------------------------------------------------------------

#[test]
fn cfg_06_print_int_line_extremes() {
    for v in [0i32, 1, -1, 2, -2, 9, 10, 99, 100, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        diff(&format!("row6 printIntLine {v}"), |api| unsafe {
            (api.print_int_line)(v)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 7 — printIntLine, randomized over the full i32 range
// ---------------------------------------------------------------------------

#[test]
fn cfg_07_print_int_line_random() {
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..512 {
        let v = rng.next_i32();
        diff(&format!("row7 printIntLine #{i} {v}"), |api| unsafe {
            (api.print_int_line)(v)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 8 — bad(), ordinary magnitudes, randomized
// ---------------------------------------------------------------------------

#[test]
fn cfg_08_bad_ordinary_random() {
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..1024 {
        let v = rng.signed_log_f32(1e-3, 1e3);
        diff(&format!("row8 bad({v:e}) #{i}"), |api| unsafe { (api.bad)(v) });
    }
}

// ---------------------------------------------------------------------------
// Row 9 — bad(), exact quotients
// ---------------------------------------------------------------------------

#[test]
fn cfg_09_bad_exact_quotients() {
    for m in [1.0f32, 2.0, 4.0, 5.0, 8.0, 10.0, 20.0, 25.0, 50.0, 100.0, 0.5, 0.25, 0.125] {
        for v in [m, -m] {
            diff(&format!("row9 bad({v})"), |api| unsafe { (api.bad)(v) });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — bad(), truncating quotients, both signs
// ---------------------------------------------------------------------------

#[test]
fn cfg_10_bad_truncating_quotients() {
    for m in [3.0f32, 6.0, 7.0, 9.0, 11.0, 13.0, 30.0, 33.0, 99.0, 101.0, 1000.0, 0.3, 0.7] {
        for v in [m, -m] {
            diff(&format!("row10 bad({v})"), |api| unsafe { (api.bad)(v) });
        }
    }
    // Randomized: quotients with fractional parts of every sign/size.
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..512 {
        let v = rng.signed_log_f32(1.0, 1e4);
        diff(&format!("row10 bad rand #{i} {v:e}"), |api| unsafe { (api.bad)(v) });
    }
}

// ---------------------------------------------------------------------------
// Row 11 — bad(), tiny magnitudes -> the (int) cast overflows
// ---------------------------------------------------------------------------

#[test]
fn cfg_11_bad_tiny_magnitudes() {
    let named = [
        f32::from_bits(1),
        f32::from_bits(2),
        f32::from_bits(0x0000_FFFF),
        f32::MIN_POSITIVE,
        1e-40f32,
        1e-30f32,
        1e-20f32,
        1e-10f32,
        5e-7f32,
        1e-6f32,
    ];
    for m in named {
        for v in [m, -m] {
            diff(&format!("row11 bad({v:e})"), |api| unsafe { (api.bad)(v) });
        }
    }
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..512 {
        let v = rng.signed_log_f32(1e-45, 1e-6);
        diff(&format!("row11 bad rand #{i} {v:e}"), |api| unsafe { (api.bad)(v) });
    }
}

// ---------------------------------------------------------------------------
// Row 12 — bad(), divisors straddling the INT_MAX cast boundary
// ---------------------------------------------------------------------------

#[test]
fn cfg_12_bad_cast_boundary() {
    let base = (common::CAST_LIMIT as f32).to_bits();
    for delta in -8i32..=8 {
        let bits = (base as i64 + delta as i64) as u32;
        let m = f32::from_bits(bits);
        for v in [m, -m] {
            diff(&format!("row12 bad({v:e}) delta={delta}"), |api| unsafe {
                (api.bad)(v)
            });
        }
    }
    // Also probe around 100/(2^31) computed in f64 then rounded both ways, and
    // the negative-side limit 100/(2^31 + 1).
    for m in [
        (100.0f64 / 2147483648.0) as f32,
        (100.0f64 / 2147483647.0) as f32,
        (100.0f64 / 2147483649.0) as f32,
        (100.0f64 / 2147483650.0) as f32,
    ] {
        for v in [m, -m] {
            diff(&format!("row12 bad limit({v:e})"), |api| unsafe { (api.bad)(v) });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — bad(), large magnitudes -> quotient truncates to 0
// ---------------------------------------------------------------------------

#[test]
fn cfg_13_bad_large_magnitudes() {
    for m in [101.0f32, 200.0, 1e3, 1e6, 1e10, 1e20, 1e30, 1e38, f32::MAX] {
        for v in [m, -m] {
            diff(&format!("row13 bad({v:e})"), |api| unsafe { (api.bad)(v) });
        }
    }
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..512 {
        let v = rng.signed_log_f32(1e2, 3e38);
        diff(&format!("row13 bad rand #{i} {v:e}"), |api| unsafe { (api.bad)(v) });
    }
}

// ---------------------------------------------------------------------------
// Row 14 — bad(), zeroes / infinities / NaNs
// ---------------------------------------------------------------------------

#[test]
fn cfg_14_bad_special_values() {
    let specials: Vec<(&str, f32)> = vec![
        ("+0.0", 0.0),
        ("-0.0", -0.0),
        ("+inf", f32::INFINITY),
        ("-inf", f32::NEG_INFINITY),
        ("qnan", f32::NAN),
        ("-qnan", -f32::NAN),
        ("snan", f32::from_bits(0x7F80_0001)),
        ("-snan", f32::from_bits(0xFF80_0001)),
        ("nan_payload", f32::from_bits(0x7FC1_2345)),
    ];
    for (name, v) in specials {
        diff(&format!("row14 bad({name})"), |api| unsafe { (api.bad)(v) });
    }
}

// ---------------------------------------------------------------------------
// Row 15 — bad(), 1024 fully random bit patterns
// ---------------------------------------------------------------------------

#[test]
fn cfg_15_bad_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..1024 {
        let v = rng.any_f32();
        diff(&format!("row15 bad bits=0x{:08x} #{i}", v.to_bits()), |api| unsafe {
            (api.bad)(v)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 16 — good(), accepted branch (randomized |x| > 1e-6)
// ---------------------------------------------------------------------------

#[test]
fn cfg_16_good_accepted_random() {
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..1024 {
        let v = rng.signed_log_f32(1.1e-6, 1e6);
        diff(&format!("row16 good({v:e}) #{i}"), |api| unsafe { (api.good)(v) });
    }
}

// ---------------------------------------------------------------------------
// Row 17 — good(), rejected branch
// ---------------------------------------------------------------------------

#[test]
fn cfg_17_good_rejected() {
    let named = [
        0.0f32,
        -0.0f32,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-30f32,
        -1e-30f32,
        5e-7f32,
        -5e-7f32,
        1e-6f32,
        -1e-6f32,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001),
    ];
    for v in named {
        diff(&format!("row17 good({v:e})"), |api| unsafe { (api.good)(v) });
    }
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..512 {
        let v = rng.signed_log_f32(1e-45, 1e-6);
        diff(&format!("row17 good rand #{i} {v:e}"), |api| unsafe { (api.good)(v) });
    }
}

// ---------------------------------------------------------------------------
// Row 18 — good(), the guard boundary pair
// ---------------------------------------------------------------------------

#[test]
fn cfg_18_good_guard_boundary() {
    let g = common::GUARD_F32;
    let up = common::guard_next_up();
    let down = f32::from_bits(g.to_bits() - 1);
    for (name, v) in [
        ("guard-1ulp", down),
        ("-guard-1ulp", -down),
        ("guard", g),
        ("-guard", -g),
        ("guard+1ulp", up),
        ("-guard+1ulp", -up),
        ("guard+2ulp", f32::from_bits(g.to_bits() + 2)),
        ("-guard+2ulp", -f32::from_bits(g.to_bits() + 2)),
    ] {
        diff(&format!("row18 good({name}={v:e})"), |api| unsafe { (api.good)(v) });
        // And through bad(), where the same magnitudes are unguarded.
        diff(&format!("row18 bad({name}={v:e})"), |api| unsafe { (api.bad)(v) });
    }
}

// ---------------------------------------------------------------------------
// Row 19 — good(), guard passes but the (int) cast still overflows
// ---------------------------------------------------------------------------

#[test]
fn cfg_19_good_accepted_but_cast_overflows() {
    // |x| in (1e-6, 100/2^31]: fabs(x) > 1e-6 so the guard lets it through, yet
    // 100.0/x >= 2^31 so the (int) cast is out of range.
    let mut rng = Rng::new(SEED ^ 19);
    let lo = 1.0000002e-6f64; // just above the guard
    let hi = common::CAST_LIMIT; // ~4.6566e-8 ... note: BELOW the guard
    // The interval above is empty (the cast limit is smaller than the guard),
    // which is itself a fact about the C: every guard-accepted value is also
    // cast-safe *unless* the quotient is large for other reasons. Assert that
    // and instead sweep the region just above the guard, where the quotient is
    // near 1e8 — large but in range.
    assert!(hi < lo, "cast limit {hi:e} should be below the guard threshold");
    for i in 0..512 {
        let v = rng.signed_log_f32(1.0000002e-6, 1e-5);
        diff(&format!("row19 good({v:e}) #{i}"), |api| unsafe { (api.good)(v) });
        diff(&format!("row19 bad({v:e}) #{i}"), |api| unsafe { (api.bad)(v) });
    }
    // Explicit walk over the first 64 floats above the guard.
    let base = common::GUARD_F32.to_bits();
    for k in 1..=64u32 {
        let v = f32::from_bits(base + k);
        diff(&format!("row19 good(+{k}ulp)"), |api| unsafe { (api.good)(v) });
    }
}

// ---------------------------------------------------------------------------
// Row 20 — good(), infinities pass the guard
// ---------------------------------------------------------------------------

#[test]
fn cfg_20_good_infinities() {
    for (name, v) in [("+inf", f32::INFINITY), ("-inf", f32::NEG_INFINITY)] {
        diff(&format!("row20 good({name})"), |api| unsafe { (api.good)(v) });
    }
    for m in [1e38f32, f32::MAX] {
        for v in [m, -m] {
            diff(&format!("row20 good({v:e})"), |api| unsafe { (api.good)(v) });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21 — good(), 1024 fully random bit patterns
// ---------------------------------------------------------------------------

#[test]
fn cfg_21_good_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..1024 {
        let v = rng.any_f32();
        diff(&format!("row21 good bits=0x{:08x} #{i}", v.to_bits()), |api| unsafe {
            (api.good)(v)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 22 — driver(), both arguments ordinary
// ---------------------------------------------------------------------------

#[test]
fn cfg_22_driver_ordinary_random() {
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..1024 {
        let g = rng.signed_log_f32(1e-3, 1e3);
        let b = rng.signed_log_f32(1e-3, 1e3);
        diff(&format!("row22 driver({g:e}, {b:e}) #{i}"), |api| unsafe {
            (api.driver)(g, b)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 23 — driver(), full cross product of the named float classes
// ---------------------------------------------------------------------------

#[test]
fn cfg_23_driver_named_cross_product() {
    let names = named_floats();
    for (gn, g) in &names {
        for (bn, b) in &names {
            diff(&format!("row23 driver(good={gn}, bad={bn})"), |api| unsafe {
                (api.driver)(*g, *b)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — driver(), randomized full-domain bit-pattern pairs
// ---------------------------------------------------------------------------

#[test]
fn cfg_24_driver_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..2048 {
        let g = rng.any_f32();
        let b = rng.any_f32();
        diff(
            &format!(
                "row24 driver(0x{:08x}, 0x{:08x}) #{i}",
                g.to_bits(),
                b.to_bits()
            ),
            |api| unsafe { (api.driver)(g, b) },
        );
    }
}

// ---------------------------------------------------------------------------
// Row 25 — interleaved sequence of calls to all five exports
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Op {
    Line(Vec<u8>),
    LineNull,
    IntLine(i32),
    Bad(f32),
    Good(f32),
    Driver(f32, f32),
}

fn run_ops(api: &Api, ops: &[Op]) {
    for op in ops {
        unsafe {
            match op {
                Op::Line(b) => (api.print_line)(b.as_ptr() as *const c_char),
                Op::LineNull => (api.print_line)(std::ptr::null()),
                Op::IntLine(v) => (api.print_int_line)(*v),
                Op::Bad(v) => (api.bad)(*v),
                Op::Good(v) => (api.good)(*v),
                Op::Driver(g, b) => (api.driver)(*g, *b),
            }
        }
    }
}

#[test]
fn cfg_25_interleaved_call_sequences() {
    let mut rng = Rng::new(SEED ^ 25);
    for seq in 0..128 {
        let n = 1 + (rng.next_u32() % 24) as usize;
        let mut ops = Vec::with_capacity(n);
        for _ in 0..n {
            let pick = rng.next_u32() % 6;
            ops.push(match pick {
                0 => {
                    let len = (rng.next_u32() % 20) as usize;
                    let mut b: Vec<u8> = (0..len)
                        .map(|_| b'a' + (rng.next_u32() % 26) as u8)
                        .collect();
                    b.push(0);
                    Op::Line(b)
                }
                1 => Op::LineNull,
                2 => Op::IntLine(rng.next_i32()),
                3 => Op::Bad(rng.any_f32()),
                4 => Op::Good(rng.any_f32()),
                _ => Op::Driver(rng.any_f32(), rng.any_f32()),
            });
        }
        diff(&format!("row25 sequence #{seq} ({n} ops)"), |api| {
            run_ops(api, &ops)
        });
    }
}
