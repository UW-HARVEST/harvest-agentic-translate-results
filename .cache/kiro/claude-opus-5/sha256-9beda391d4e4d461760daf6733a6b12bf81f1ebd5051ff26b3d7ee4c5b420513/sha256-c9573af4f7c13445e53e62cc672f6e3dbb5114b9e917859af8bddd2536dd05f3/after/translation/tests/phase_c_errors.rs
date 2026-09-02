//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E14). Both `.so`s are called through
//! their exports and must agree on the exact returned value (this library has no
//! error codes, so "same rejection" means the identical `size_t` result and the
//! identical printed output — see ERRORS.md for why).

mod common;

use common::*;
use std::ffi::c_void;

/// E1 — `len == 0` with `p == NULL`: never dereferenced, must not fault.
#[test]
fn e1_null_pointer_zero_len() {
    for p in pairs() {
        let v = unsafe { assert_hash_eq_ptr(&p, std::ptr::null_mut(), 0, 0, "E1") };
        // sanity: the empty-message hash is a fixed non-trivial constant
        assert_ne!(v, 0, "E1 empty hash unexpectedly zero");
    }
}

/// E2 — `len == 0` with a valid pointer: result must equal the NULL case.
#[test]
fn e2_zero_len_pointer_independent() {
    for p in pairs() {
        let null_v = unsafe { assert_hash_eq_ptr(&p, std::ptr::null_mut(), 0, 0, "E2 null") };
        let mut buf = [0xABu8; 32];
        let v = unsafe {
            assert_hash_eq_ptr(&p, buf.as_mut_ptr() as *mut c_void, 0, 0, "E2 valid ptr")
        };
        assert_eq!(v, null_v, "E2 len=0 must not depend on the pointer");
        // one-past-the-end pointer with len 0 is also fine
        let end = unsafe { buf.as_mut_ptr().add(buf.len()) } as *mut c_void;
        let v2 = unsafe { assert_hash_eq_ptr(&p, end, 0, 0, "E2 end ptr") };
        assert_eq!(v2, null_v);
    }
}

/// E3 — `len == 0`, `p == NULL`, across the whole seed space.
#[test]
fn e3_null_zero_len_seed_sweep() {
    let mut rng = Rng::new(0xE3_0000_0001);
    for p in pairs() {
        for &seed in [0usize, 1, usize::MAX, usize::MAX - 1, usize::MAX / 2].iter() {
            unsafe {
                assert_hash_eq_ptr(&p, std::ptr::null_mut(), 0, seed, "E3 fixed seed");
            }
        }
        for _ in 0..5000 {
            let seed = rng.next_u64() as usize;
            unsafe {
                assert_hash_eq_ptr(&p, std::ptr::null_mut(), 0, seed, "E3 random seed");
            }
        }
    }
}

/// E4 — `switch` `case 0` arm: `len` an exact multiple of 8.
#[test]
fn e4_switch_case_zero() {
    let mut rng = Rng::new(0xE4_0000_0001);
    for p in pairs() {
        let mut buf = vec![0u8; 512];
        for nb in 0..64usize {
            for _ in 0..40 {
                rng.fill(&mut buf);
                let seed = rng.next_u64() as usize;
                assert_hash_eq(&p, &mut buf, nb * 8, seed, &format!("E4 nb={nb}"));
            }
        }
    }
}

/// E5 — `switch` arms `case 1`..`case 7`, all seven fall-through entry points,
/// at several block counts.
#[test]
fn e5_switch_cases_one_to_seven() {
    let mut rng = Rng::new(0xE5_0000_0001);
    for p in pairs() {
        let mut buf = vec![0u8; 512];
        for rem in 1..8usize {
            for nb in 0..20usize {
                for _ in 0..60 {
                    rng.fill(&mut buf);
                    let seed = rng.next_u64() as usize;
                    assert_hash_eq(
                        &p,
                        &mut buf,
                        nb * 8 + rem,
                        seed,
                        &format!("E5 rem={rem} nb={nb}"),
                    );
                }
            }
        }
    }
}

/// E6 — the missing `default:` arm is unreachable (`len - i` is provably 0..=7).
/// Documented; asserted structurally by exhaustively covering 0..=7.
#[test]
fn e6_switch_default_unreachable() {
    let mut rng = Rng::new(0xE6_0000_0001);
    for p in pairs() {
        let mut buf = vec![0u8; 128];
        let mut seen = [false; 8];
        for len in 0..128usize {
            rng.fill(&mut buf);
            seen[len % 8] = true;
            assert_hash_eq(&p, &mut buf, len, 0, &format!("E6 len={len}"));
        }
        assert!(seen.iter().all(|&s| s), "all 8 switch arms must be covered");
    }
}

/// E7 — `len == 1` on an exact 1-byte allocation: must read exactly one byte.
#[test]
fn e7_exact_one_byte() {
    let mut rng = Rng::new(0xE7_0000_0001);
    for p in pairs() {
        for _ in 0..2000 {
            let mut v: Vec<u8> = vec![rng.next_u8()];
            unsafe {
                assert_hash_eq_ptr(&p, v.as_mut_ptr() as *mut c_void, 1, 0, "E7 exact len=1");
            }
        }
    }
}

/// E8 — `len == 7` on an exact 7-byte allocation (widest tail arm).
#[test]
fn e8_exact_seven_bytes() {
    let mut rng = Rng::new(0xE8_0000_0001);
    for p in pairs() {
        for _ in 0..2000 {
            let mut v: Vec<u8> = (0..7).map(|_| rng.next_u8()).collect();
            assert_eq!(v.len(), 7);
            unsafe {
                assert_hash_eq_ptr(&p, v.as_mut_ptr() as *mut c_void, 7, 0, "E8 exact len=7");
            }
        }
    }
}

/// E9 — `len == 8` on an exact 8-byte allocation (first full block).
#[test]
fn e9_exact_eight_bytes() {
    let mut rng = Rng::new(0xE9_0000_0001);
    for p in pairs() {
        for _ in 0..2000 {
            let mut v: Vec<u8> = (0..8).map(|_| rng.next_u8()).collect();
            unsafe {
                assert_hash_eq_ptr(&p, v.as_mut_ptr() as *mut c_void, 8, 0, "E9 exact len=8");
            }
        }
    }
}

/// E10 — tail `case 4` signed-overflow: `d[3] << 24` with `d[3] >= 0x80`.
/// Exhaustive over all 256 values of `d[3]` for every tail length that reads it.
#[test]
fn e10_tail_signed_overflow_exhaustive_d3() {
    for p in pairs() {
        for len in 4..8usize {
            for d3 in 0..=255u8 {
                let mut buf = [0u8; 8];
                buf[0] = 0x11;
                buf[1] = 0x22;
                buf[2] = 0x33;
                buf[3] = d3;
                buf[4] = 0x99;
                buf[5] = 0xAA;
                buf[6] = 0xBB;
                buf[7] = 0xCC;
                assert_hash_eq(&p, &mut buf, len, 0, &format!("E10 len={len} d3={d3:#04x}"));
            }
        }
        // also exhaustive over d[2] and d[1] (positive-only shifts, no overflow)
        for len in 2..8usize {
            for v in 0..=255u8 {
                let mut buf = [0x5Au8; 8];
                buf[1] = v;
                assert_hash_eq(&p, &mut buf, len, 0, &format!("E10b len={len} d1={v:#04x}"));
                let mut buf2 = [0x5Au8; 8];
                buf2[2] = v;
                assert_hash_eq(&p, &mut buf2, len, 0, &format!("E10c len={len} d2={v:#04x}"));
            }
        }
        // exhaustive over the size_t-cast tail bytes d[4], d[5], d[6]
        for (idx, minlen) in [(4usize, 5usize), (5, 6), (6, 7)] {
            for v in 0..=255u8 {
                for len in minlen..8usize {
                    let mut buf = [0x3Cu8; 8];
                    buf[idx] = v;
                    assert_hash_eq(
                        &p,
                        &mut buf,
                        len,
                        0,
                        &format!("E10d len={len} d{idx}={v:#04x}"),
                    );
                }
            }
        }
        // exhaustive over d[0]
        for v in 0..=255u8 {
            let mut buf = [0x77u8; 8];
            buf[0] = v;
            for len in 1..8usize {
                assert_hash_eq(&p, &mut buf, len, 0, &format!("E10e len={len} d0={v:#04x}"));
            }
        }
    }
}

/// E11 — main-loop signed overflow: exhaustive over `d[3]` and `d[7]` of a
/// single full block, and over both simultaneously for multi-block inputs.
#[test]
fn e11_block_signed_overflow_exhaustive() {
    for p in pairs() {
        // exhaustive d[3] x d[7] for a single 8-byte block: 65536 combinations
        for d3 in 0..=255u8 {
            for d7 in 0..=255u8 {
                let mut buf = [0u8; 8];
                buf[0] = 0x01;
                buf[1] = 0x02;
                buf[2] = 0x03;
                buf[3] = d3;
                buf[4] = 0x05;
                buf[5] = 0x06;
                buf[6] = 0x07;
                buf[7] = d7;
                assert_hash_eq(&p, &mut buf, 8, 0, "E11 single block d3xd7");
            }
        }
        // exhaustive over every byte position of one block
        for idx in 0..8usize {
            for v in 0..=255u8 {
                let mut buf = [0x40u8; 8];
                buf[idx] = v;
                assert_hash_eq(&p, &mut buf, 8, 0, &format!("E11b idx={idx} v={v:#04x}"));
            }
        }
        // two blocks, both top bytes swept at the 0x80 boundary
        for d3 in [0x00u8, 0x7F, 0x80, 0xFF] {
            for d7 in [0x00u8, 0x7F, 0x80, 0xFF] {
                for e3 in [0x00u8, 0x7F, 0x80, 0xFF] {
                    for e7 in [0x00u8, 0x7F, 0x80, 0xFF] {
                        let mut buf = [0x5Fu8; 16];
                        buf[3] = d3;
                        buf[7] = d7;
                        buf[11] = e3;
                        buf[15] = e7;
                        for len in 16..24usize {
                            let mut b2 = vec![0x5Fu8; 24];
                            b2[..16].copy_from_slice(&buf);
                            assert_hash_eq(&p, &mut b2, len, 0, "E11c two blocks");
                        }
                    }
                }
            }
        }
    }
}

// E12 and E13 (the `siphash` stdout-comparison rows) live in
// `tests/siphash_stdout.rs` — see the note in phase_b_valid.rs.

/// E14 — oversized `len` is genuine UB in C (unchecked out-of-bounds read) and
/// cannot be compared. Instead assert the largest SAFE boundary we can:
/// `len` equal to the exact allocation size for a range of large buffers, plus
/// `len` values one step below/at the buffer end.
#[test]
fn e14_length_boundaries_within_allocation() {
    let mut rng = Rng::new(0xE14_0000_0001);
    for p in pairs() {
        for size in [1usize, 7, 8, 9, 63, 64, 65, 255, 256, 257, 1023, 1024, 4096] {
            let mut v: Vec<u8> = (0..size).map(|_| rng.next_u8()).collect();
            let ptr = v.as_mut_ptr() as *mut c_void;
            for len in [0usize, 1, size.saturating_sub(1), size] {
                if len > size {
                    continue;
                }
                let q = if len == 0 { std::ptr::null_mut() } else { ptr };
                unsafe {
                    assert_hash_eq_ptr(
                        &p,
                        q,
                        len,
                        rng.next_u64() as usize,
                        &format!("E14 size={size} len={len}"),
                    );
                }
            }
        }
    }
}
