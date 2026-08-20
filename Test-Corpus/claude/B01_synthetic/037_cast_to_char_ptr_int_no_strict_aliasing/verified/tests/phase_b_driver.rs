//! Phase B — valid-path differential tests for the lowest-level exported
//! symbol, `void driver(int)`.  Rows 1-9 and 30 of CONFIGS.md.
//!
//! Both implementations are dlopen'd; `driver` is called through the `.so`
//! export in each of them and the bytes written to stdout are compared.

mod common;

use common::*;

/// CONFIGS row 1 — `x == 0`, every byte takes the `%02x` zero-padding path.
#[test]
fn cfg_01_driver_zero() {
    assert_driver_eq(0);
    let c = call_driver(c_impl(), 0);
    assert_eq!(c, b"00000000\n", "sanity: C output for driver(0)");
}

/// CONFIGS row 2 — `x == -1`, every byte `0xff`.
#[test]
fn cfg_02_driver_all_ones() {
    assert_driver_eq(-1);
    assert_eq!(call_driver(c_impl(), -1), b"ffffffff\n");
}

/// CONFIGS row 3 — sweep the low byte over `0..=255`: crosses the `0x0f`/`0x10`
/// zero-padding boundary of `%02x`.
#[test]
fn cfg_03_driver_low_byte_sweep() {
    let xs: Vec<i32> = (0..=255).collect();
    assert_driver_batch_eq("low byte sweep", &xs);
    // ... and the same byte values in each of the other three byte positions
    for shift in [8u32, 16, 24] {
        let xs: Vec<i32> = (0..=255u32).map(|b| (b << shift) as i32).collect();
        assert_driver_batch_eq(&format!("byte sweep <<{shift}"), &xs);
    }
}

/// CONFIGS row 4 — one bit set, all 32 positions (includes the sign bit).
#[test]
fn cfg_04_driver_single_bit() {
    let xs: Vec<i32> = (0..32).map(|i| (1u32 << i) as i32).collect();
    assert_driver_batch_eq("single bit", &xs);
    let xs: Vec<i32> = (0..32).map(|i| !(1u32 << i) as i32).collect();
    assert_driver_batch_eq("single bit clear", &xs);
}

/// CONFIGS row 5 — the interesting boundary values of `int`.
#[test]
fn cfg_05_driver_boundaries() {
    let xs: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
        0x7f7f_7f7fu32 as i32,
        0x8080_8080u32 as i32,
        0x0000_00ffu32 as i32,
        0xff00_0000u32 as i32,
        0x0000_ff00u32 as i32,
        0x00ff_0000u32 as i32,
        0x1000_0000u32 as i32,
        0x0f0f_0f0fu32 as i32,
        0xf0f0_f0f0u32 as i32,
    ];
    assert_driver_batch_eq("boundaries", &xs);
}

/// CONFIGS row 6 — four distinct bytes, mixing padded and unpadded classes,
/// including bytes that look like ASCII/newline in the raw buffer.
#[test]
fn cfg_06_driver_mixed_bytes() {
    let xs: Vec<i32> = vec![
        0x0a0b_0c0du32 as i32,
        0xf00f_10efu32 as i32,
        0x0a0a_0a0au32 as i32, // '\n' bytes
        0x2030_4050u32 as i32,
        0x0102_0304u32 as i32,
        0x0403_0201u32 as i32,
        0x00ff_00ffu32 as i32,
        0xff00_ff00u32 as i32,
        0x7f80_0100u32 as i32,
        0x0080_7fffu32 as i32,
    ];
    assert_driver_batch_eq("mixed bytes", &xs);
}

/// CONFIGS row 7 — 20 000 uniformly random `i32` (fixed seed).
#[test]
fn cfg_07_driver_random_i32() {
    let mut rng = Rng::new();
    let xs: Vec<i32> = (0..20_000).map(|_| rng.next_i32()).collect();
    for chunk in xs.chunks(2_000) {
        assert_driver_batch_eq("random i32", chunk);
    }
}

/// CONFIGS row 8 — random values built only from nibble-sized bytes, so every
/// byte hits the zero-padding path.
#[test]
fn cfg_08_driver_random_nibbles() {
    let mut rng = Rng::new();
    let xs: Vec<i32> = (0..4_000)
        .map(|_| (rng.next_u32() & 0x0f0f_0f0f) as i32)
        .collect();
    for chunk in xs.chunks(2_000) {
        assert_driver_batch_eq("random nibbles", chunk);
    }
    // and only high-nibble bytes (never padded)
    let xs: Vec<i32> = (0..4_000)
        .map(|_| (rng.next_u32() | 0xf0f0_f0f0) as i32)
        .collect();
    for chunk in xs.chunks(2_000) {
        assert_driver_batch_eq("random high nibbles", chunk);
    }
}

/// CONFIGS row 9 / ERRORS row "re-entrancy" — 4096 consecutive calls in one
/// process, alternating values: no state may leak between calls.
#[test]
fn cfg_29_repeated_calls() {
    let mut xs: Vec<i32> = Vec::with_capacity(4096);
    for i in 0..4096i32 {
        xs.push(if i % 2 == 0 { i } else { -i });
    }
    assert_driver_batch_eq("repeated calls", &xs);
}

/// CONFIGS row 30 — `driver` and `main` resolved from the same handle and
/// interleaved.
#[test]
fn cfg_30_interleaved_symbols() {
    let mut rng = Rng::with_seed(0xDEAD_BEEF_1234_5678);
    for _ in 0..25 {
        let x = rng.next_i32();
        assert_driver_eq(x);
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        assert_main_eq(format!("{v}\n").as_bytes(), Stdin::File);
        assert_driver_eq(v as i32);
    }
}
