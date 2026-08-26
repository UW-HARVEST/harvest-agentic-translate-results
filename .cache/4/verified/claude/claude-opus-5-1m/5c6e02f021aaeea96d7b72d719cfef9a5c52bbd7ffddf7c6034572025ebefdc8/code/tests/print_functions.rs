//! Phase B + Phase C for the two lowest-level entry points, `printLine` and
//! `printIntLine`.
//!
//! CONFIGS.md rows 1-15, ERRORS.md rows 1-8 and G1-G5.
//! Both implementations are invoked through `dlsym` on their `.so`.

mod common;

use common::*;
use std::ffi::{c_char, CString};

/// Emit one string through `printLine` in both objects and compare the bytes.
fn diff_print_line(pair: &Pair, row: &str, case: &str, bytes: &[u8]) {
    let cs = CString::new(bytes).expect("interior NUL");
    let ptr = cs.as_ptr();
    let c_out = {
        let f = pair.print_line(Which::C);
        capture(|| unsafe { f(ptr) })
    };
    let r_out = {
        let f = pair.print_line(Which::Rust);
        capture(|| unsafe { f(ptr) })
    };
    assert_same(row, case, &c_out, &r_out);

    // Cross-check against the C contract: the bytes followed by one newline.
    let mut expected = bytes.to_vec();
    expected.push(b'\n');
    assert_same(row, &format!("{case} (vs printf(\"%s\\n\"))"), &expected, &c_out);
}

fn diff_print_int_line(pair: &Pair, row: &str, v: i32) {
    let c_out = {
        let f = pair.print_int_line(Which::C);
        capture(|| unsafe { f(v) })
    };
    let r_out = {
        let f = pair.print_int_line(Which::Rust);
        capture(|| unsafe { f(v) })
    };
    assert_same(row, &format!("printIntLine({v})"), &c_out, &r_out);
    assert_same(
        row,
        &format!("printIntLine({v}) (vs printf(\"%d\\n\"))"),
        format!("{v}\n").as_bytes(),
        &c_out,
    );
}

// ---------------------------------------------------------------------------
// CONFIGS row 1 / ERRORS row 1 / G1 — the NULL guard
// ---------------------------------------------------------------------------

#[test]
fn row01_print_line_null_prints_nothing() {
    let pair = Pair::load();
    let c_out = {
        let f = pair.print_line(Which::C);
        capture(|| unsafe { f(std::ptr::null()) })
    };
    let r_out = {
        let f = pair.print_line(Which::Rust);
        capture(|| unsafe { f(std::ptr::null()) })
    };
    assert_same("row01/ERR1", "printLine(NULL)", &c_out, &r_out);
    assert!(
        c_out.is_empty(),
        "C printLine(NULL) unexpectedly printed {:?}",
        show(&c_out)
    );
}

// ---------------------------------------------------------------------------
// CONFIGS row 2 / ERRORS row 2 / G2 — empty string
// ---------------------------------------------------------------------------

#[test]
fn row02_print_line_empty_string() {
    let pair = Pair::load();
    diff_print_line(&pair, "row02/ERR2", "\"\"", b"");
}

// ---------------------------------------------------------------------------
// CONFIGS row 3 — every single byte value
// ---------------------------------------------------------------------------

#[test]
fn row03_print_line_every_single_byte_value() {
    let pair = Pair::load();
    for b in 1u8..=255 {
        diff_print_line(&pair, "row03", &format!("single byte 0x{b:02x}"), &[b]);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 4 — random printable ASCII
// ---------------------------------------------------------------------------

#[test]
fn row04_print_line_random_printable_ascii() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x04);
    for i in 0..300 {
        let len = rng.range_usize(1, 64);
        let s = rng.ascii_printable(len);
        diff_print_line(&pair, "row04", &format!("ascii #{i} len={len}"), &s);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 5 / ERRORS row 4 — arbitrary (non-UTF-8) bytes
// ---------------------------------------------------------------------------

#[test]
fn row05_print_line_random_arbitrary_bytes() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x05);
    for i in 0..300 {
        let len = rng.range_usize(1, 64);
        let s = rng.c_string_bytes(len);
        diff_print_line(&pair, "row05/ERR4", &format!("bytes #{i} len={len}"), &s);
    }
    // Hand-picked invalid UTF-8 sequences.
    for (i, s) in [
        vec![0xFFu8],
        vec![0xFE, 0xFF],
        vec![0x80],
        vec![0xC0, 0x80],
        vec![0xED, 0xA0, 0x80],       // UTF-16 surrogate encoding
        vec![0xF5, 0x80, 0x80, 0x80], // > U+10FFFF
        vec![0xE0, 0x80],             // truncated
        b"abc\xffdef".to_vec(),
    ]
    .iter()
    .enumerate()
    {
        diff_print_line(&pair, "row05/ERR4", &format!("invalid utf8 #{i}"), s);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 6 — embedded newlines and other control bytes
// ---------------------------------------------------------------------------

#[test]
fn row06_print_line_embedded_control_bytes() {
    let pair = Pair::load();
    for (i, s) in [
        b"\n".to_vec(),
        b"a\nb".to_vec(),
        b"\n\n\n".to_vec(),
        b"a\r\nb".to_vec(),
        b"\t\ttabbed".to_vec(),
        b"vert\x0btab".to_vec(),
        b"form\x0cfeed".to_vec(),
        b"trailing\n".to_vec(),
        b"\nleading".to_vec(),
        b"\x01\x02\x03\x04\x05\x06\x07\x08".to_vec(),
        b"\x1b[31mred\x1b[0m".to_vec(),
        b"del\x7f".to_vec(),
    ]
    .iter()
    .enumerate()
    {
        diff_print_line(&pair, "row06", &format!("control #{i}"), s);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 7 / ERRORS row 3 — printf conversion specifiers in the payload
// ---------------------------------------------------------------------------

#[test]
fn row07_print_line_format_specifiers_are_literal() {
    let pair = Pair::load();
    for (i, s) in [
        b"%s".to_vec(),
        b"%d".to_vec(),
        b"%n".to_vec(),
        b"%%".to_vec(),
        b"%p %x %o".to_vec(),
        b"%s%s%s%s%s%s%s%s".to_vec(),
        b"100%".to_vec(),
        b"%.*f".to_vec(),
        b"%1$s %2$s".to_vec(),
        b"%".to_vec(),
    ]
    .iter()
    .enumerate()
    {
        diff_print_line(&pair, "row07/ERR3", &format!("fmt #{i}"), s);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 8 / ERRORS row 5 / G3 — long strings crossing stdio buffers
// ---------------------------------------------------------------------------

#[test]
fn row08_print_line_long_strings() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x08);
    for len in [1usize, 2, 127, 128, 1023, 1024, 4095, 4096, 4097, 8192, 65536] {
        let filler = vec![b'A'; len];
        diff_print_line(&pair, "row08/ERR5", &format!("'A' x {len}"), &filler);
        let s = rng.c_string_bytes(len);
        diff_print_line(&pair, "row08/ERR5", &format!("random x {len}"), &s);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 9 — repeated calls in one process
// ---------------------------------------------------------------------------

#[test]
fn row09_print_line_repeated_calls_in_one_capture() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x09);
    let strings: Vec<CString> = (0..64)
        .map(|_| {
            let len = rng.range_usize(0, 40);
            CString::new(rng.ascii_printable(len)).unwrap()
        })
        .collect();
    let ptrs: Vec<*const c_char> = strings.iter().map(|s| s.as_ptr()).collect();

    let c_out = {
        let f = pair.print_line(Which::C);
        capture(|| {
            for p in &ptrs {
                unsafe { f(*p) };
            }
            // interleave the NULL case
            unsafe { f(std::ptr::null()) };
            for p in ptrs.iter().rev() {
                unsafe { f(*p) };
            }
        })
    };
    let r_out = {
        let f = pair.print_line(Which::Rust);
        capture(|| {
            for p in &ptrs {
                unsafe { f(*p) };
            }
            unsafe { f(std::ptr::null()) };
            for p in ptrs.iter().rev() {
                unsafe { f(*p) };
            }
        })
    };
    assert_same("row09", "129 chained printLine calls", &c_out, &r_out);
}

// ---------------------------------------------------------------------------
// CONFIGS rows 10-12 / ERRORS rows 6-7 / G4 — printIntLine fixed values
// ---------------------------------------------------------------------------

#[test]
fn row10_to_12_print_int_line_fixed_and_endpoint_values() {
    let pair = Pair::load();
    for v in [
        0i32,
        1,
        -1,
        9,
        -9,
        10,
        -10,
        99,
        -99,
        100,
        -100,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        2147483646,
        -2147483647,
    ] {
        diff_print_int_line(&pair, "row10-12/ERR6-7/G4", v);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 13 / G5 — power-of-two boundaries (digit-count transitions)
// ---------------------------------------------------------------------------

#[test]
fn row13_print_int_line_power_of_two_boundaries() {
    let pair = Pair::load();
    for k in 0..32u32 {
        let p = 1i64 << k;
        for delta in [-1i64, 0, 1] {
            for sign in [1i64, -1] {
                let v = sign * (p + delta);
                if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                    diff_print_int_line(&pair, "row13/G5", v as i32);
                }
            }
        }
    }
    // Decimal digit-count boundaries.
    let mut p = 1i64;
    while p <= i32::MAX as i64 {
        for delta in [-1i64, 0, 1] {
            for sign in [1i64, -1] {
                let v = sign * (p + delta);
                if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                    diff_print_int_line(&pair, "row13/G5", v as i32);
                }
            }
        }
        p *= 10;
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 14 / ERRORS row 8 / G5 — random full-range i32
// ---------------------------------------------------------------------------

#[test]
fn row14_print_int_line_random_full_range() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x14);
    for _ in 0..600 {
        diff_print_int_line(&pair, "row14/ERR8/G5", rng.next_i32());
    }
    // Bit patterns that only make sense as raw `int`s coming across the FFI
    // boundary (a C `int` parameter accepts any 32-bit pattern).
    for raw in [
        0x8000_0000u32,
        0xFFFF_FFFF,
        0x7FFF_FFFF,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x0000_0001,
        0x8000_0001,
    ] {
        diff_print_int_line(&pair, "row14/ERR8/G5", raw as i32);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row 15 — printIntLine repeated / interleaved with printLine
// ---------------------------------------------------------------------------

#[test]
fn row15_print_int_line_interleaved_with_print_line() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0x15);

    #[derive(Clone)]
    enum Op {
        Int(i32),
        Str(Vec<u8>),
        Null,
    }
    let ops: Vec<Op> = (0..200)
        .map(|_| match rng.below(3) {
            0 => Op::Int(rng.next_i32()),
            1 => {
                let len = rng.range_usize(0, 30);
                Op::Str(rng.ascii_printable(len))
            }
            _ => Op::Null,
        })
        .collect();

    let run = |which: Which| {
        let pl = pair.print_line(which);
        let pil = pair.print_int_line(which);
        let ops = ops.clone();
        capture(move || {
            for op in &ops {
                match op {
                    Op::Int(v) => unsafe { pil(*v) },
                    Op::Str(s) => {
                        let cs = CString::new(s.clone()).unwrap();
                        unsafe { pl(cs.as_ptr()) }
                    }
                    Op::Null => unsafe { pl(std::ptr::null()) },
                }
            }
        })
    };

    let c_out = run(Which::C);
    let r_out = run(Which::Rust);
    assert_same("row15", "200 interleaved print ops", &c_out, &r_out);
}
