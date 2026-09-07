//! Extra hardening for Phase B: exhaustive sweeps over the individual byte
//! positions that the C code treats with distinct shift expressions.
//!
//! `c_src/src/lib.c` uses a *different* shift expression per tail offset:
//!   d[6] -> `((size_t)d[6] << 24) << 24`   (== << 48)
//!   d[5] -> `((size_t)d[5] << 20) << 20`   (== << 40)
//!   d[4] -> `((size_t)d[4] << 16) << 16`   (== << 32)
//!   d[3] -> `(d[3] << 24)`                 (int arithmetic -> sign-extends!)
//!   d[2] -> `(d[2] << 16)`
//!   d[1] -> `(d[1] << 8)`
//!   d[0] -> `d[0]`
//! and in the body block:
//!   d[0..3] -> int OR, sign-extended to size_t
//!   d[4..7] -> int OR, cast to size_t, then `<< 16 << 16`
//! Each of these is swept over its whole 0..=255 value range here.

mod common;

use common::*;

/// Exhaustive: every byte position x every byte value x every tail length.
#[test]
fn exh_tail_every_byte_position_and_value() {
    for len in 1..=7usize {
        for pos in 0..len {
            for v in 0u16..=255 {
                let mut buf = [0u8; 8];
                buf[pos] = v as u8;
                assert_hash_eq(
                    &mut buf,
                    len,
                    0,
                    &format!("EXH tail len={} pos={} v={:#04x}", len, pos, v),
                );
            }
        }
    }
}

/// Exhaustive: every byte position of a single body block x every byte value.
#[test]
fn exh_body_every_byte_position_and_value() {
    for pos in 0..8usize {
        for v in 0u16..=255 {
            let mut buf = [0u8; 8];
            buf[pos] = v as u8;
            assert_hash_eq(
                &mut buf,
                8,
                0,
                &format!("EXH body pos={} v={:#04x}", pos, v),
            );
        }
    }
}

/// Exhaustive: body block byte position x value, with a nonzero tail present so
/// the body path and the tail path interact.
#[test]
fn exh_body_plus_tail_interaction() {
    let mut rng = Rng::new(SEED ^ 0xBEEF);
    for pos in 0..8usize {
        for v in [0u8, 1, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
            for tail in 1..=7usize {
                let mut buf = vec![0u8; 16];
                rng.fill(&mut buf);
                buf[pos] = v;
                let seed = rng.next_u64() as usize;
                let ctx = format!("EXH body+tail pos={} v={:#04x} tail={}", pos, v, tail);
                assert_hash_eq(&mut buf, 8 + tail, 0, &ctx);
                assert_hash_eq(&mut buf, 8 + tail, seed, &ctx);
            }
        }
    }
}

/// Exhaustive over every `len` from 0 to 200 with several random buffers and
/// several random seeds each — catches any off-by-one in the block/tail split.
#[test]
fn exh_every_len_0_to_200() {
    let mut rng = Rng::new(SEED ^ 0xC0DE);
    let mut buf = vec![0u8; 208];
    for len in 0..=200usize {
        for _ in 0..8 {
            rng.fill(&mut buf);
            for _ in 0..4 {
                let seed = rng.next_u64() as usize;
                assert_hash_eq(&mut buf, len, seed, &format!("EXH len={}", len));
            }
            assert_hash_eq(&mut buf, len, 0, &format!("EXH len={} seed0", len));
        }
    }
}

/// Seed bit sweep: one bit set at a time, plus one bit clear at a time.
#[test]
fn exh_seed_single_bit_sweep() {
    let mut rng = Rng::new(SEED ^ 0x51D);
    let mut buf = vec![0u8; 48];
    rng.fill(&mut buf);
    for bit in 0..64u32 {
        let s1 = 1usize << bit;
        let s2 = !(1usize << bit);
        for len in [0usize, 1, 7, 8, 9, 15, 16, 33] {
            assert_hash_eq(&mut buf, len, s1, &format!("EXH seed bit{} set len={}", bit, len));
            assert_hash_eq(&mut buf, len, s2, &format!("EXH seed bit{} clr len={}", bit, len));
        }
    }
}
