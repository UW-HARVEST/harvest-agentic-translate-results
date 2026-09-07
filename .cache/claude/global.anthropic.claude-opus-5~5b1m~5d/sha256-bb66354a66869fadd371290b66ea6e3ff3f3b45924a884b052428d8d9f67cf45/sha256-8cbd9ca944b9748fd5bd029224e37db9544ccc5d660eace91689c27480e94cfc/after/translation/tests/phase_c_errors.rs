//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.  The library has no error-return surface, so each row asserts
//! that C and Rust agree *exactly* on the degenerate/hostile input rather than
//! merely "both failed".

mod common;

use common::*;
use std::ffi::{c_int, c_void};

// E1 — null pointer with zero length
#[test]
fn err_e1_null_ptr_zero_len() {
    assert_hash_eq_raw(std::ptr::null_mut(), 0, 0, "E1");
}

// E2 — null pointer, seed sweep (seed is still mixed in)
#[test]
fn err_e2_null_ptr_seed_sweep() {
    for seed in [0usize, 1, 2, usize::MAX, usize::MAX - 1, usize::MAX / 2, 1 << 63, 0x5555_5555_5555_5555] {
        assert_hash_eq_raw(std::ptr::null_mut(), 0, seed, &format!("E2 seed={:#x}", seed));
    }
    let mut rng = Rng::new(SEED ^ 0xE2);
    for _ in 0..N {
        let seed = rng.next_u64() as usize;
        assert_hash_eq_raw(std::ptr::null_mut(), 0, seed, "E2 random");
    }
    // A dangling-but-unread pointer must behave identically to null when len==0.
    let dangling = 0x1usize as *mut c_void;
    assert_hash_eq_raw(dangling, 0, 0, "E2 dangling len0");
    assert_hash_eq_raw(usize::MAX as *mut c_void, 0, 12345, "E2 max-ptr len0");
}

// E3 — zero length with a valid pointer
#[test]
fn err_e3_zero_len_valid_ptr() {
    let mut rng = Rng::new(SEED ^ 0xE3);
    let mut buf = vec![0u8; 64];
    for _ in 0..N {
        rng.fill(&mut buf);
        let seed = rng.next_u64() as usize;
        assert_hash_eq(&mut buf, 0, 0, "E3");
        assert_hash_eq(&mut buf, 0, seed, "E3 seeded");
    }
    // Zero length must ignore the buffer contents entirely: same value for
    // two different buffers, in both libraries.
    let mut a = vec![0x00u8; 64];
    let mut b = vec![0xffu8; 64];
    let va = assert_hash_eq(&mut a, 0, 7, "E3 zeros");
    let vb = assert_hash_eq(&mut b, 0, 7, "E3 ffs");
    assert_eq!(va, vb, "E3: len==0 must not read the buffer");
}

// E4 — one step either side of the block boundary
#[test]
fn err_e4_block_boundary() {
    let mut rng = Rng::new(SEED ^ 0xE4);
    let mut buf = vec![0u8; 32];
    for _ in 0..N {
        rng.fill(&mut buf);
        let seed = rng.next_u64() as usize;
        for len in [7usize, 8, 9, 15, 16, 17, 23, 24, 25] {
            assert_hash_eq(&mut buf, len, 0, &format!("E4 len={}", len));
            assert_hash_eq(&mut buf, len, seed, &format!("E4 len={} seeded", len));
        }
    }
}

// E5 — every `switch (len - i)` fall-through arm, incl. arm 0
#[test]
fn err_e5_all_switch_arms() {
    let mut rng = Rng::new(SEED ^ 0xE5);
    let mut buf = vec![0u8; 24];
    for blocks in 0..=2usize {
        for arm in 0..=7usize {
            let len = blocks * 8 + arm;
            for _ in 0..64 {
                rng.fill(&mut buf);
                let seed = rng.next_u64() as usize;
                let ctx = format!("E5 arm={} blocks={} len={}", arm, blocks, len);
                assert_hash_eq(&mut buf, len, 0, &ctx);
                assert_hash_eq(&mut buf, len, seed, &ctx);
            }
            // Deterministic per-arm probe: only the bytes the arm reads set.
            let mut probe = vec![0u8; 24];
            for k in 0..len {
                probe[k] = 0x80 | (k as u8);
            }
            assert_hash_eq(&mut probe, len, 0, &format!("E5 probe arm={} len={}", arm, len));
        }
    }
}

// E6 — tail `d[3] << 24` signed-int overflow -> sign extension
#[test]
fn err_e6_signed_shift_sign_extension_tail() {
    // Exhaustive over d[3] for every tail length that reads it.
    for len in 4..=7usize {
        for v in 0u16..=255 {
            let mut buf = [0u8; 8];
            buf[3] = v as u8;
            let ctx = format!("E6 len={} d[3]={:#02x}", len, v);
            assert_hash_eq(&mut buf, len, 0, &ctx);
        }
    }
    // And with the surrounding bytes non-zero so the OR interaction shows.
    let mut rng = Rng::new(SEED ^ 0xE6);
    for len in 4..=7usize {
        for v in [0x7fu8, 0x80, 0x81, 0xfe, 0xff] {
            for _ in 0..32 {
                let mut buf = [0u8; 8];
                rng.fill(&mut buf);
                buf[3] = v;
                let seed = rng.next_u64() as usize;
                assert_hash_eq(&mut buf, len, seed, &format!("E6 len={} v={:#02x}", len, v));
            }
        }
    }
}

// E7 — body-block `d[3]`/`d[7] << 24` signed-int overflow
#[test]
fn err_e7_signed_shift_sign_extension_body() {
    for v3 in [0x00u8, 0x7f, 0x80, 0xff] {
        for v7 in [0x00u8, 0x7f, 0x80, 0xff] {
            for blocks in 1..=3usize {
                let mut buf = vec![0u8; blocks * 8 + 8];
                for blk in 0..blocks {
                    buf[blk * 8 + 3] = v3;
                    buf[blk * 8 + 7] = v7;
                }
                for tail in 0..=7usize {
                    let len = blocks * 8 + tail;
                    let ctx = format!("E7 v3={:#02x} v7={:#02x} blocks={} len={}", v3, v7, blocks, len);
                    assert_hash_eq(&mut buf, len, 0, &ctx);
                    assert_hash_eq(&mut buf, len, usize::MAX, &ctx);
                }
            }
        }
    }
    // Exhaustive over d[7] in a single block (the `<< 16 << 16` cast path).
    for v in 0u16..=255 {
        let mut buf = [0u8; 8];
        buf[7] = v as u8;
        assert_hash_eq(&mut buf, 8, 0, &format!("E7 d[7]={:#02x}", v));
    }
    // Exhaustive over d[3] in a single block.
    for v in 0u16..=255 {
        let mut buf = [0u8; 8];
        buf[3] = v as u8;
        assert_hash_eq(&mut buf, 8, 0, &format!("E7 d[3]={:#02x}", v));
    }
}

// E8 — extremal seeds (`~seed` at the ends of the range)
#[test]
fn err_e8_extremal_seeds() {
    let mut rng = Rng::new(SEED ^ 0xE8);
    let mut buf = vec![0u8; 40];
    let seeds = [
        0usize,
        1,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        1usize << 63,
        (1usize << 63) - 1,
        0xaaaa_aaaa_aaaa_aaaa,
        0x5555_5555_5555_5555,
    ];
    for &seed in &seeds {
        for len in 0..=33usize {
            rng.fill(&mut buf);
            assert_hash_eq(&mut buf, len, seed, &format!("E8 seed={:#x} len={}", seed, len));
        }
    }
}

// E9 — huge `len` arithmetic must not panic in Rust where C does not trap.
//      A real out-of-bounds read would crash *both*, so we assert only on the
//      boundary that is safely reachable: `len` exactly equal to the
//      allocation, for allocations straddling the block boundary.
#[test]
fn err_e9_no_panic_on_huge_len_arith() {
    let mut rng = Rng::new(SEED ^ 0xE9);
    for cap in 1..=64usize {
        let mut buf = vec![0u8; cap];
        rng.fill(&mut buf);
        // len == cap: reads exactly to the end of the allocation.
        assert_hash_eq(&mut buf, cap, 0, &format!("E9 cap=len={}", cap));
    }
    // The Rust translation uses wrapping arithmetic for `i + sizeof(size_t)`
    // and `len - i`; verify the `len - i` wrap-safe path via len values whose
    // low bits exercise every remainder while `len` is large.
    let mut buf = vec![0u8; 4104];
    rng.fill(&mut buf);
    for tail in 0..=7usize {
        let len = 4096 + tail;
        assert_hash_eq(&mut buf, len, 0, &format!("E9 large len={}", len));
    }
}

// E10 / E13 — `siphash` with extremal `int` inputs; stdout byte-compared
#[test]
fn err_e10_siphash_extremal_init() {
    for init in [c_int::MIN, c_int::MIN + 1, -257, -256, -255, -128, -2, -1, 0, 1, 127, 128, 255, 256, 257, c_int::MAX - 1, c_int::MAX] {
        assert_siphash_stdout_eq(init);
    }
}

// E11 — `init == INT_MAX`: `z++` overflows signed int inside the fill loop
#[test]
fn err_e11_siphash_int_max_overflow() {
    assert_siphash_stdout_eq(c_int::MAX);
    assert_siphash_stdout_eq(c_int::MAX - 63);
    assert_siphash_stdout_eq(c_int::MAX - 32);
    assert_siphash_stdout_eq(c_int::MAX - 1);
    // Cross-check against the low-level entry with the wrapped fill.
    let mut mem = [0u8; 64];
    let mut z: c_int = c_int::MAX;
    for i in 0..64usize {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }
    for len in 0..64usize {
        assert_hash_eq(&mut mem, len, 0, &format!("E11 len={}", len));
    }
}

// E12 — no enum / mode / flag parameter exists in this API.  Recorded as a
//       deliberate no-op so the "out-of-range enum across FFI" class is
//       explicitly covered: the only integer parameters are `size_t` (whole
//       range valid) and `int` (whole range valid, covered by E10/E11).
#[test]
fn err_e12_no_enum_parameters_documented() {
    // Assert the exported symbols take exactly the signatures we believe, by
    // exercising the full boundary of each scalar parameter's range.
    let mut buf = [0u8; 8];
    assert_hash_eq(&mut buf, 0, usize::MIN, "E12 seed=MIN");
    assert_hash_eq(&mut buf, 0, usize::MAX, "E12 seed=MAX");
    assert_hash_eq(&mut buf, 8, usize::MIN, "E12 len=8 seed=MIN");
    assert_hash_eq(&mut buf, 8, usize::MAX, "E12 len=8 seed=MAX");
    assert_siphash_stdout_eq(c_int::MIN);
    assert_siphash_stdout_eq(c_int::MAX);
}
