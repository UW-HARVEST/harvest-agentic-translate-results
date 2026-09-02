//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call is made through `dlsym` on both
//! the C `.so` and the Rust `.so`; the bytes each writes to `stdout` must match
//! exactly. Randomized rows use the fixed-seed `SplitMix64` in `common`.

mod common;

use common::*;
use std::ffi::c_char;

// ── Row 1 ────────────────────────────────────────────────────────────────────
#[test]
fn cfg_01_print_line_empty() {
    assert_same_print_line("empty string", b"");
}

// ── Row 2 ────────────────────────────────────────────────────────────────────
#[test]
fn cfg_02_print_line_single_byte() {
    for b in 1u8..=255 {
        assert_same_print_line(&format!("single byte {b:#04x}"), &[b]);
    }
}

// ── Row 3 ────────────────────────────────────────────────────────────────────
#[test]
fn cfg_03_print_line_random_ascii() {
    let mut rng = Rng::new();
    for i in 0..512 {
        let len = rng.range(1, 256) as usize;
        let s: Vec<u8> = (0..len).map(|_| rng.range(0x20, 0x7e) as u8).collect();
        assert_same_print_line(&format!("random ascii #{i} len={len}"), &s);
    }
}

// ── Row 4 ────────────────────────────────────────────────────────────────────
#[test]
fn cfg_04_print_line_format_specifiers() {
    // `line` is an argument to printf("%s\n", line), never the format string.
    const SPECS: [&[u8]; 6] = [
        b"%s",
        b"%d %d %d %d",
        b"%n",
        b"%%",
        b"%99999999d",
        b"%p %x %hhn %.*s",
    ];
    for (i, s) in SPECS.iter().enumerate() {
        assert_same_print_line(&format!("fixed format specifier #{i}"), s);
    }
    let mut rng = Rng::seeded(SEED ^ 4);
    for i in 0..256 {
        let mut s = Vec::new();
        for _ in 0..rng.range(1, 12) {
            s.extend_from_slice(rng.pick(&SPECS));
            s.push(rng.range(0x20, 0x7e) as u8);
        }
        assert_same_print_line(&format!("assembled format specifiers #{i}"), &s);
    }
}

// ── Row 5 ────────────────────────────────────────────────────────────────────
#[test]
fn cfg_05_print_line_embedded_control() {
    const CTRL: [u8; 6] = [b'\n', b'\r', b'\t', 0x0b, 0x0c, 0x7f];
    let mut rng = Rng::seeded(SEED ^ 5);
    for i in 0..256 {
        let len = rng.range(1, 64) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(2) == 0 {
                    *rng.pick(&CTRL)
                } else {
                    rng.range(0x20, 0x7e) as u8
                }
            })
            .collect();
        assert_same_print_line(&format!("embedded control #{i} len={len}"), &s);
    }
}

// ── Row 6 ────────────────────────────────────────────────────────────────────
#[test]
fn cfg_06_print_line_random_bytes() {
    let mut rng = Rng::seeded(SEED ^ 6);
    for i in 0..512 {
        let len = rng.range(1, 512) as usize;
        // 0x01..=0xFF: arbitrary, frequently invalid UTF-8.
        let s: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        assert_same_print_line(&format!("random bytes #{i} len={len}"), &s);
    }
}

// ── Row 7 ────────────────────────────────────────────────────────────────────
#[test]
fn cfg_07_print_line_buffer_boundaries() {
    const LENS: [usize; 11] = [
        1,
        2,
        4095,
        4096,
        4097,
        8191,
        8192,
        8193,
        65535,
        65536,
        1024 * 1024,
    ];
    let mut rng = Rng::seeded(SEED ^ 7);
    for len in LENS {
        let s: Vec<u8> = (0..len).map(|_| rng.range(0x21, 0x7e) as u8).collect();
        assert_same_print_line(&format!("buffer boundary len={len}"), &s);
    }
}

// ── Rows 8–10 ────────────────────────────────────────────────────────────────
#[test]
fn cfg_08_print_int_zero() {
    assert_same_print_int_line("zero", 0);
}

#[test]
fn cfg_09_print_int_plus_minus_one() {
    assert_same_print_int_line("one", 1);
    assert_same_print_int_line("minus one", -1);
}

#[test]
fn cfg_10_print_int_extremes() {
    assert_same_print_int_line("INT_MAX", i32::MAX);
    assert_same_print_int_line("INT_MIN", i32::MIN);
}

// ── Row 11 ───────────────────────────────────────────────────────────────────
#[test]
fn cfg_11_print_int_width_boundaries() {
    let mut p: i64 = 1;
    for _ in 1..=9 {
        p *= 10;
        for v in [p - 1, p, -(p - 1), -p] {
            assert_same_print_int_line(&format!("width boundary {v}"), v as i32);
        }
    }
}

// ── Row 12 ───────────────────────────────────────────────────────────────────
#[test]
fn cfg_12_int_random() {
    let mut rng = Rng::seeded(SEED ^ 12);
    for i in 0..2048 {
        let v = rng.next_i32();
        assert_same_print_int_line(&format!("random i32 #{i} = {v}"), v);
    }
}

// ── Row 13 ───────────────────────────────────────────────────────────────────
#[test]
fn cfg_13_print_int_random_small() {
    let mut rng = Rng::seeded(SEED ^ 13);
    for i in 0..1024 {
        let v = rng.range(-1000, 1000) as i32;
        assert_same_print_int_line(&format!("random small #{i} = {v}"), v);
    }
}

// ── Rows 14–16: level-1 and level-2 entry points ─────────────────────────────
#[test]
fn cfg_14_good_single() {
    assert_same("good()", |api| unsafe { (api.good)() });
}

#[test]
fn cfg_15_bad_single() {
    assert_same("bad()", |api| unsafe { (api.bad)() });
}

#[test]
fn cfg_16_driver_single() {
    assert_same("driver()", |api| unsafe { (api.driver)() });
}

/// The composed pipeline must also match the exact expected byte stream of the
/// C original, not merely agree with itself. `bad()` prints 0 twice because the
/// original discards `intOne + intTwo`; `good()` prints 0 then 2.
#[test]
fn cfg_16b_driver_expected_bytes() {
    let l = libs();
    const EXPECTED: &str = "Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n";
    for api in [&l.c, &l.rust] {
        let out = capture(|| unsafe { (api.driver)() });
        assert_eq!(
            String::from_utf8_lossy(&out),
            EXPECTED,
            "{} driver() byte stream",
            api.name
        );
    }
}

// ── Row 17 ───────────────────────────────────────────────────────────────────
#[test]
fn cfg_17_repeated_invocations() {
    assert_same("good() x32", |api| unsafe {
        for _ in 0..32 {
            (api.good)()
        }
    });
    assert_same("bad() x32", |api| unsafe {
        for _ in 0..32 {
            (api.bad)()
        }
    });
    assert_same("driver() x32", |api| unsafe {
        for _ in 0..32 {
            (api.driver)()
        }
    });
}

// ── Row 18 ───────────────────────────────────────────────────────────────────
enum Op {
    Line(Vec<u8>),
    LineNull,
    Int(i32),
    Good,
    Bad,
    Driver,
}

#[test]
fn cfg_18_random_mixed_sequences() {
    let mut rng = Rng::seeded(SEED ^ 18);
    for seq in 0..256 {
        let n = rng.range(1, 24) as usize;
        let ops: Vec<Op> = (0..n)
            .map(|_| match rng.below(6) {
                0 => {
                    let len = rng.range(0, 40) as usize;
                    Op::Line((0..len).map(|_| rng.range(1, 255) as u8).collect())
                }
                1 => Op::LineNull,
                2 => Op::Int(rng.next_i32()),
                3 => Op::Good,
                4 => Op::Bad,
                _ => Op::Driver,
            })
            .collect();
        // NUL-terminate up front so both replays see identical buffers.
        let bufs: Vec<Vec<u8>> = ops
            .iter()
            .map(|o| match o {
                Op::Line(v) => {
                    let mut b = v.clone();
                    b.push(0);
                    b
                }
                _ => Vec::new(),
            })
            .collect();

        assert_same(&format!("mixed sequence #{seq} len={n}"), |api| unsafe {
            for (op, buf) in ops.iter().zip(bufs.iter()) {
                match op {
                    Op::Line(_) => (api.print_line)(buf.as_ptr() as *const c_char),
                    Op::LineNull => (api.print_line)(std::ptr::null()),
                    Op::Int(v) => (api.print_int_line)(*v),
                    Op::Good => (api.good)(),
                    Op::Bad => (api.bad)(),
                    Op::Driver => (api.driver)(),
                }
            }
        });
    }
}
