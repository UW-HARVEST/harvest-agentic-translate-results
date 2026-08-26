//! Phase B — valid-path differential tests for the low-level entry point
//! `void driver(int)` (rows C1–C9 and C24 of CONFIGS.md).
//!
//! Both implementations are called through their `.so` exports; the captured
//! bytes on fd 1 must be identical.

mod common;

use common::*;

/// C1 — `x == 0`: every byte zero, so every `%02x` is a zero-padded pair.
#[test]
fn c1_driver_zero() {
    diff_driver_batch(&[0], "C1 x=0");
}

/// C2 — small magnitudes, positive and negative, including the carry into the
/// second byte and the sign extension of negatives.
#[test]
fn c2_driver_small_magnitudes() {
    let values: Vec<i32> = (-1024..=1024).collect();
    diff_driver_batch(&values, "C2 -1024..=1024");
}

/// C3 — exhaustive 8⁴ matrix over the interesting byte values (nibble padding,
/// 0x00/0xff extremes, sign bit).
#[test]
fn c3_driver_nibble_padding_matrix() {
    const B: [u32; 8] = [0x00, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0xf0, 0xff];
    let mut values = Vec::with_capacity(4096);
    for &b0 in &B {
        for &b1 in &B {
            for &b2 in &B {
                for &b3 in &B {
                    values.push(((b3 << 24) | (b2 << 16) | (b1 << 8) | b0) as i32);
                }
            }
        }
    }
    assert_eq!(values.len(), 4096);
    diff_driver_batch(&values, "C3 nibble matrix");
}

/// C4 — signed/unsigned boundaries and all powers of two ±1.
#[test]
fn c4_driver_boundary_values() {
    let mut values: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
        u32::MAX as i32,
        0x8000_0000u32 as i32,
        0x7fff_ffff,
        0x0000_00ff,
        0x0000_ff00,
        0x00ff_0000,
        0xff00_0000u32 as i32,
        0x0f0f_0f0f,
        0xf0f0_f0f0u32 as i32,
        0x1234_5678,
        0x7856_3412,
    ];
    for bit in 0..32u32 {
        let p = 1u32 << bit;
        values.push(p as i32);
        values.push(p.wrapping_sub(1) as i32);
        values.push(p.wrapping_add(1) as i32);
        values.push((p as i32).wrapping_neg());
    }
    diff_driver_batch(&values, "C4 boundaries");
}

/// C5 — 20 000 uniformly random `i32` (fixed seed).
#[test]
fn c5_driver_random_full_range() {
    let mut rng = Rng::new(0x5EED_0001);
    let values: Vec<i32> = (0..20_000).map(|_| rng.next_i32()).collect();
    diff_driver_batch(&values, "C5 random full range");
}

/// C6 — byte-position sweep: each byte takes every value while the rest hold a
/// fixed non-zero pattern.  Catches byte-order and per-byte formatting bugs.
#[test]
fn c6_driver_byte_position_sweep() {
    let base: u32 = 0xa5_5a_c3_3c;
    let mut values = Vec::with_capacity(1024);
    for pos in 0..4u32 {
        let mask = !(0xffu32 << (pos * 8));
        for b in 0..=255u32 {
            values.push(((base & mask) | (b << (pos * 8))) as i32);
        }
    }
    assert_eq!(values.len(), 1024);
    diff_driver_batch(&values, "C6 byte sweep");
}

/// C7 — exhaustive over the low 16 bits with a fixed high half.
#[test]
fn c7_driver_exhaustive_low_16_bits() {
    for high in [0x0000u32, 0xdead, 0xffff] {
        let values: Vec<i32> = (0..=0xffffu32)
            .map(|low| ((high << 16) | low) as i32)
            .collect();
        assert_eq!(values.len(), 65_536);
        diff_driver_batch(&values, &format!("C7 high={high:#06x}"));
    }
}

/// C8 — many consecutive calls in one process: the stdio buffer accumulates and
/// the records must be concatenated without separators.
#[test]
fn c8_driver_repeated_calls_one_process() {
    let mut rng = Rng::new(0x5EED_0008);
    let values: Vec<i32> = (0..10_000).map(|_| rng.next_i32()).collect();
    diff_driver_batch(&values, "C8 10k calls");

    // And explicitly check the shape of the accumulated stream.
    let p = pair();
    let out = capture_child(|| {
        for &v in values.iter().take(3) {
            unsafe { (p.c.driver)(v) }
        }
    })
    .out;
    assert_eq!(out.len(), 27);
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 3);
}

/// C9 — stdout is a pipe (different stdio buffering mode) instead of a regular
/// file.  Kept well under the pipe capacity so the single-process capture
/// cannot block.
#[test]
fn c9_driver_stdout_is_a_pipe() {
    let mut rng = Rng::new(0x5EED_0009);
    let mut values: Vec<i32> = vec![0, -1, i32::MIN, i32::MAX];
    values.extend((0..5_000).map(|_| rng.next_i32()));
    diff_driver_batch_piped(&values, "C9 pipe stdout");
}

/// C24 (driver half) — the two entry points used in the same process.  The
/// `main` half lives in main_valid.rs; here `driver` is called before and after
/// a captured batch to make sure no per-call state leaks.
#[test]
fn c24_driver_state_is_not_sticky() {
    let p = pair();
    for _ in 0..3 {
        let c = capture_child(|| unsafe {
            (p.c.driver)(1);
            (p.c.driver)(-1);
            (p.c.driver)(0);
        });
        let r = capture_child(|| unsafe {
            (p.rs.driver)(1);
            (p.rs.driver)(-1);
            (p.rs.driver)(0);
        });
        assert_eq!(
            (as_text(&c.out), c.status),
            (as_text(&r.out), r.status),
            "C24 interleaved driver calls"
        );
        assert_eq!(as_text(&c.out), "01000000\nffffffff\n00000000\n");
    }
}
