//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call goes through `dlsym` on both
//! the C `.so` and the Rust `.so`; the ENTIRE output buffer (sentinel-filled
//! beforehand) plus the returned pointer are compared byte-for-byte, so stray
//! writes past `bin_len * 2 + 1` are caught too.

mod common;

use common::*;

// --- C1..C3: bin_len == 0 (loop body never runs) ---------------------------

#[test]
fn c1_empty_exact_min_len() {
    let bin = [0x11u8; 4]; // non-null but unread
    assert_same("C1", 1, 0, 1, &bin, 0, 0);
}

#[test]
fn c2_empty_null_bin() {
    assert_same("C2", 1, 0, 1, &[], 0, 0);
}

#[test]
fn c3_empty_generous_maxlen() {
    let bin = [0x11u8; 4];
    assert_same("C3", 64, 0, 64, &bin, 0, 0);
}

// --- C4..C9: single byte, nibble-select boundary values --------------------

fn one_byte(label: &str, byte: u8) {
    assert_same(label, 3, 0, 3, &[byte], 0, 1);
}

#[test]
fn c4_single_byte_00() {
    one_byte("C4", 0x00);
}

#[test]
fn c5_single_byte_99() {
    one_byte("C5", 0x99);
}

#[test]
fn c6_single_byte_aa() {
    one_byte("C6", 0xAA);
}

#[test]
fn c7_single_byte_ff() {
    one_byte("C7", 0xFF);
}

#[test]
fn c8_single_byte_9a() {
    one_byte("C8", 0x9A);
}

#[test]
fn c9_single_byte_a9() {
    one_byte("C9", 0xA9);
}

// --- C10: exhaustive over the full b x c cross-product --------------------

#[test]
fn c10_all_256_byte_values_exhaustive() {
    for v in 0u16..=255 {
        let b = v as u8;
        assert_same(&format!("C10[{b:#04x}]"), 3, 0, 3, &[b], 0, 1);
    }
}

// --- C11: slack far past the minimum -------------------------------------

#[test]
fn c11_single_byte_large_slack() {
    let mut rng = Rng::new(0xC011);
    for _ in 0..256 {
        let b = rng.next_u8();
        assert_same("C11", 4096, 0, 4096, &[b], 0, 1);
    }
}

// --- C12/C13: two bytes, exact-min and min+1 -----------------------------

#[test]
fn c12_two_bytes_exact_min_randomized() {
    let mut rng = Rng::new(0xC012);
    for _ in 0..2000 {
        let mut bin = [0u8; 2];
        rng.fill(&mut bin);
        assert_same("C12", 5, 0, 5, &bin, 0, 2);
    }
}

#[test]
fn c13_two_bytes_min_plus_one() {
    let mut rng = Rng::new(0xC013);
    for _ in 0..2000 {
        let mut bin = [0u8; 2];
        rng.fill(&mut bin);
        assert_same("C13", 6, 0, 6, &bin, 0, 2);
    }
}

// --- C14/C15: odd and even lengths --------------------------------------

#[test]
fn c14_odd_lengths_randomized() {
    let mut rng = Rng::new(0xC014);
    for &n in &[3usize, 5, 7, 9, 11, 13, 15, 17, 31, 63] {
        for _ in 0..200 {
            let mut bin = vec![0u8; n];
            rng.fill(&mut bin);
            assert_same(&format!("C14[n={n}]"), n * 2 + 1, 0, n * 2 + 1, &bin, 0, n);
        }
    }
}

#[test]
fn c15_even_lengths_randomized() {
    let mut rng = Rng::new(0xC015);
    for &n in &[4usize, 6, 8, 10, 12, 16, 32, 64] {
        for _ in 0..200 {
            let mut bin = vec![0u8; n];
            rng.fill(&mut bin);
            assert_same(&format!("C15[n={n}]"), n * 2 + 1, 0, n * 2 + 1, &bin, 0, n);
        }
    }
}

// --- C16: every byte value once, in order -------------------------------

#[test]
fn c16_256_bytes_identity_content() {
    let bin: Vec<u8> = (0..256u16).map(|v| v as u8).collect();
    assert_same("C16", 513, 0, 513, &bin, 0, 256);
}

// --- C17/C18: large inputs ---------------------------------------------

#[test]
fn c17_large_exact_min() {
    let mut rng = Rng::new(0xC017);
    for _ in 0..16 {
        let mut bin = vec![0u8; 4096];
        rng.fill(&mut bin);
        assert_same("C17", 8193, 0, 8193, &bin, 0, 4096);
    }
}

#[test]
fn c18_large_generous_slack() {
    let mut rng = Rng::new(0xC018);
    for _ in 0..16 {
        let mut bin = vec![0u8; 4096];
        rng.fill(&mut bin);
        assert_same("C18", 8193 + 977, 0, 8193 + 977, &bin, 0, 4096);
    }
}

// --- C19/C20: length sweeps -------------------------------------------

#[test]
fn c19_length_sweep_exact_min() {
    let mut rng = Rng::new(0xC019);
    for n in 0usize..=64 {
        for _ in 0..64 {
            let mut bin = vec![0u8; n];
            rng.fill(&mut bin);
            assert_same(&format!("C19[n={n}]"), n * 2 + 1, 0, n * 2 + 1, &bin, 0, n);
        }
    }
}

#[test]
fn c20_length_sweep_min_plus_one() {
    let mut rng = Rng::new(0xC020);
    for n in 0usize..=64 {
        for _ in 0..64 {
            let mut bin = vec![0u8; n];
            rng.fill(&mut bin);
            assert_same(&format!("C20[n={n}]"), n * 2 + 2, 0, n * 2 + 2, &bin, 0, n);
        }
    }
}

// --- C21: hex_maxlen = SIZE_MAX over a small real buffer ---------------
// The C only ever writes bin_len*2+1 bytes, so an over-large hex_maxlen is a
// legal (if reckless) call and must not change the output.

#[test]
fn c21_hex_maxlen_size_max() {
    let mut rng = Rng::new(0xC021);
    for n in 0usize..=16 {
        for _ in 0..32 {
            let mut bin = vec![0u8; n];
            rng.fill(&mut bin);
            assert_same(&format!("C21[n={n}]"), n * 2 + 1, 0, usize::MAX, &bin, 0, n);
        }
    }
}

// --- C22..C25: content patterns ---------------------------------------

#[test]
fn c22_all_zero_bytes() {
    let bin = vec![0x00u8; 33];
    assert_same("C22", 67, 0, 67, &bin, 0, 33);
}

#[test]
fn c23_all_ff_bytes() {
    let bin = vec![0xFFu8; 33];
    assert_same("C23", 67, 0, 67, &bin, 0, 33);
}

#[test]
fn c24_alternating_0f_f0() {
    let bin: Vec<u8> = (0..33).map(|i| if i % 2 == 0 { 0x0F } else { 0xF0 }).collect();
    assert_same("C24", 67, 0, 67, &bin, 0, 33);
}

#[test]
fn c25_dense_select_boundary_nibbles() {
    let pat = [0x99u8, 0x9A, 0xA9, 0xAA];
    let bin: Vec<u8> = (0..40).map(|i| pat[i % pat.len()]).collect();
    assert_same("C25", 81, 0, 81, &bin, 0, 40);
}

// --- C26: unaligned pointers -----------------------------------------

#[test]
fn c26_pointer_offsets() {
    let mut rng = Rng::new(0xC026);
    let n = 7usize;
    for hex_off in 0..8usize {
        for bin_off in 0..8usize {
            for _ in 0..32 {
                let mut bin = vec![0u8; n + 8];
                rng.fill(&mut bin);
                assert_same(
                    &format!("C26[h={hex_off},b={bin_off}]"),
                    n * 2 + 1 + 8,
                    hex_off,
                    n * 2 + 1,
                    &bin,
                    bin_off,
                    n,
                );
            }
        }
    }
}

// --- C27/C28: overlapping hex and bin buffers -------------------------
// The C performs no overlap check, so the aliasing behaviour is observable and
// must be reproduced exactly. These need a bespoke driver because `bin` lives
// inside the output buffer.

fn run_overlapping(f: Bin2HexFn, buf_len: usize, bin_at: usize, bin_len: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mut buf = vec![SENTINEL; buf_len];
    rng.fill(&mut buf[bin_at..bin_at + bin_len]);
    let hex_ptr = buf.as_mut_ptr() as *mut i8;
    let bin_ptr = unsafe { buf.as_ptr().add(bin_at) };
    let ret = unsafe { f(hex_ptr, buf_len, bin_ptr, bin_len) };
    assert_eq!(ret, hex_ptr, "overlap case must return hex");
    buf
}

#[test]
fn c27_bin_overlaps_tail_of_hex() {
    let l = libs();
    let n = 16usize;
    let buf_len = n * 2 + 1;
    for seed in 0..64u64 {
        let c = run_overlapping(l.c_bin2hex, buf_len, n, n, 0xC027 ^ seed);
        let r = run_overlapping(l.rust_bin2hex, buf_len, n, n, 0xC027 ^ seed);
        assert_eq!(c, r, "[C27] overlapping-tail output differs (seed {seed})");
    }
}

#[test]
fn c28_bin_overlaps_front_of_hex() {
    let l = libs();
    let n = 8usize;
    let buf_len = n * 2 + 1;
    for seed in 0..64u64 {
        let c = run_overlapping(l.c_bin2hex, buf_len, 0, n, 0xC028 ^ seed);
        let r = run_overlapping(l.rust_bin2hex, buf_len, 0, n, 0xC028 ^ seed);
        assert_eq!(c, r, "[C28] overlapping-front output differs (seed {seed})");
    }
}

// --- C29: return pointer identity ------------------------------------

#[test]
fn c29_return_pointer_identity() {
    let l = libs();
    let mut rng = Rng::new(0xC029);
    for &n in &[0usize, 1, 7, 64] {
        let mut bin = vec![0u8; n];
        rng.fill(&mut bin);
        for f in [l.c_bin2hex, l.rust_bin2hex] {
            let out = run_one(f, n * 2 + 1, 0, n * 2 + 1, &bin, 0, n);
            assert!(out.returned_hex_ptr, "[C29] n={n}: returned pointer != hex");
        }
    }
}

// --- C30: full randomized property sweep -----------------------------

#[test]
fn c30_randomized_property_sweep() {
    let mut rng = Rng::new(0xC030_DEAD_BEEF);
    for case in 0..4000u32 {
        let n = rng.below(512);
        let min = n * 2 + 1;
        let hex_maxlen = min + rng.below(65);
        let buf_len = hex_maxlen;
        let mut bin = vec![0u8; n];
        rng.fill(&mut bin);
        assert_same(
            &format!("C30[case={case},n={n},maxlen={hex_maxlen}]"),
            buf_len,
            0,
            hex_maxlen,
            &bin,
            0,
            n,
        );
    }
}
