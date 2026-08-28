//! Phase B rows C1..C13 -- `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed`, all through the `.so` exports.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

const SEEDS: &[usize] = &[
    0,
    1,
    2,
    0x31415926,
    usize::MAX,
    1usize << 63,
    (1usize << 63) | 1,
    0xdead_beef_cafe_babe,
    0x8000_0000,
    0x7fff_ffff,
];

fn hb(c: &Api, rs: &Api, buf: &mut [u8], len: usize, seed: usize) {
    unsafe {
        let p = buf.as_mut_ptr() as *mut c_void;
        let a = (c.hash_bytes)(p, len, seed);
        let b = (rs.hash_bytes)(p, len, seed);
        if a != b {
            panic!(
                "hash_bytes divergence: len={len} seed={seed:#x} buf={:x?}\n  C   ={a:#x}\n  RUST={b:#x}",
                &buf[..len.min(buf.len())]
            );
        }
    }
}

fn hs(c: &Api, rs: &Api, s: &mut [u8], seed: usize) {
    unsafe {
        let p = s.as_mut_ptr() as *mut c_char;
        let a = (c.hash_string)(p, seed);
        let b = (rs.hash_string)(p, seed);
        if a != b {
            panic!(
                "hash_string divergence: seed={seed:#x} s={:x?}\n  C   ={a:#x}\n  RUST={b:#x}",
                s
            );
        }
    }
}

// --- C1 ---------------------------------------------------------------------
#[test]
fn cfg_c1_hash_bytes_len0() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(1);
        let mut buf = [0u8; 32];
        for &seed in SEEDS {
            hb(c, rs, &mut buf, 0, seed);
        }
        for _ in 0..500 {
            let mut b = rng.bytes(16);
            hb(c, rs, &mut b, 0, rng.next_usize());
        }
    });
}

// --- C2 / C3 ----------------------------------------------------------------
#[test]
fn cfg_c2_hash_bytes_tail_1_7() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(2);
        for len in 1..=7usize {
            for &seed in SEEDS {
                // deterministic edge patterns
                for pat in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
                    let mut b = vec![pat; 8];
                    hb(c, rs, &mut b, len, seed);
                }
            }
            for _ in 0..400 {
                let mut b = rng.bytes(8);
                hb(c, rs, &mut b, len, rng.next_usize());
            }
        }
    });
}

#[test]
fn cfg_c3_hash_bytes_tail_high_bit() {
    // exercises the `data |= (d[3] << 24)` int-sign-extension quirk
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(3);
        for len in 4..=7usize {
            for hi in 0x80u8..=0xff {
                let mut b = rng.bytes(8);
                b[3] = hi;
                hb(c, rs, &mut b, len, rng.next_usize());
            }
            for lo in 0x00u8..0x80 {
                let mut b = rng.bytes(8);
                b[3] = lo;
                hb(c, rs, &mut b, len, rng.next_usize());
            }
        }
    });
}

// --- C4 / C5 ----------------------------------------------------------------
#[test]
fn cfg_c4_hash_bytes_len8() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(4);
        for &seed in SEEDS {
            for pat in [0x00u8, 0xff, 0x80, 0x7f] {
                let mut b = vec![pat; 8];
                hb(c, rs, &mut b, 8, seed);
            }
        }
        for _ in 0..1000 {
            let mut b = rng.bytes(8);
            hb(c, rs, &mut b, 8, rng.next_usize());
        }
    });
}

#[test]
fn cfg_c5_hash_bytes_word_high_bits() {
    // both sign-extension sites inside the main loop: d[3] and d[7]
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(5);
        for b3 in [0x00u8, 0x01, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
            for b7 in [0x00u8, 0x01, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
                for &seed in SEEDS {
                    let mut b = rng.bytes(16);
                    b[3] = b3;
                    b[7] = b7;
                    hb(c, rs, &mut b, 8, seed);
                    b[11] = b3;
                    b[15] = b7;
                    hb(c, rs, &mut b, 16, seed);
                }
            }
        }
    });
}

// --- C6 / C7 ----------------------------------------------------------------
#[test]
fn cfg_c6_hash_bytes_mixed_len() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(6);
        for len in 0..=64usize {
            for _ in 0..60 {
                let mut b = rng.bytes(len + 8);
                hb(c, rs, &mut b, len, rng.next_usize());
            }
            for &seed in SEEDS {
                let mut b = vec![0xffu8; len + 8];
                hb(c, rs, &mut b, len, seed);
                let mut b = vec![0x00u8; len + 8];
                hb(c, rs, &mut b, len, seed);
            }
        }
    });
}

#[test]
fn cfg_c7_hash_bytes_large() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(7);
        for _ in 0..60 {
            let n = 512 + rng.below(1024);
            let mut b = rng.bytes(n);
            hb(c, rs, &mut b, n, rng.next_usize());
        }
        let mut b = vec![0xffu8; 4096];
        for &seed in SEEDS {
            hb(c, rs, &mut b, 4096, seed);
        }
    });
}

// --- C8 ---------------------------------------------------------------------
#[test]
fn cfg_c8_hash_bytes_seed_sweep() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(8);
        let mut b = *b"the quick brown fox jumps over the lazy dog!!";
        for &seed in SEEDS {
            for len in 0..b.len() {
                hb(c, rs, &mut b, len, seed);
            }
        }
        for bit in 0..64u32 {
            hb(c, rs, &mut b, 17, 1usize << bit);
            hb(c, rs, &mut b, 17, !(1usize << bit));
        }
        let blen = b.len();
        for _ in 0..2000 {
            let l = rng.below(blen + 1);
            let sd = rng.next_usize();
            hb(c, rs, &mut b, l, sd);
        }
    });
}

// --- C9 / C10 / C11 / C12 ---------------------------------------------------
#[test]
fn cfg_c9_hash_string_empty() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(9);
        let mut s = [0u8; 1];
        for &seed in SEEDS {
            hs(c, rs, &mut s, seed);
        }
        for _ in 0..500 {
            hs(c, rs, &mut s, rng.next_usize());
        }
    });
}

#[test]
fn cfg_c10_hash_string_ascii() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(10);
        for len in 1..=64usize {
            for _ in 0..40 {
                let mut s = rng.cstring(len);
                hs(c, rs, &mut s, rng.next_usize());
            }
            for &seed in SEEDS {
                let mut s = rng.cstring(len);
                hs(c, rs, &mut s, seed);
            }
        }
        // fixed, human-meaningful strings
        for lit in [
            &b"a\0"[..],
            b"ab\0",
            b"abc\0",
            b"test_0\0",
            b"test_123456\0",
            b"\x01\0",
            b"\x7f\0",
        ] {
            let mut s = lit.to_vec();
            for &seed in SEEDS {
                hs(c, rs, &mut s, seed);
            }
        }
    });
}

#[test]
fn cfg_c11_hash_string_high_bytes() {
    // `(unsigned char) *str++` -- bytes >= 0x80 must not sign-extend
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(11);
        for b in 0x80u8..=0xff {
            let mut s = vec![b, 0];
            for &seed in SEEDS {
                hs(c, rs, &mut s, seed);
            }
            let mut s = vec![b, b, b, b, b, b, b, b, 0];
            hs(c, rs, &mut s, 0x31415926);
        }
        for len in 1..=32usize {
            for _ in 0..30 {
                let mut s = rng.cstring_high(len);
                hs(c, rs, &mut s, rng.next_usize());
            }
        }
    });
}

#[test]
fn cfg_c12_hash_string_long() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(12);
        for _ in 0..40 {
            let n = 256 + rng.below(3840);
            let mut s = rng.cstring(n);
            hs(c, rs, &mut s, rng.next_usize());
        }
        let mut s = vec![b'x'; 4097];
        s.push(0);
        for &seed in SEEDS {
            hs(c, rs, &mut s, seed);
        }
    });
}

// --- C13 --------------------------------------------------------------------
#[test]
fn cfg_c13_rand_seed_advance() {
    // Each fresh hash index consumes the global seed and advances it with the
    // LCG  seed = seed*0x27bb2ee687b0b0fd + 0xb504f32d.
    for &start in &[
        0usize,
        1,
        0x31415926,
        usize::MAX,
        1usize << 63,
        0xabcd_ef01_2345_6789,
    ] {
        with_libs(start, |c, rs| unsafe {
            let mut expect = start;
            for _ in 0..8 {
                let tc = (c.shmode_func)(16, SH_ARENA);
                let tr = (rs.shmode_func)(16, SH_ARENA);
                let sc = snap_map(tc, 16, KeyKind::Binary);
                let sr = snap_map(tr, 16, KeyKind::Binary);
                assert_same("shmode_func fresh table", &sc, &sr);
                let seed_c = sc.table.as_ref().unwrap().seed;
                assert_eq!(seed_c, expect, "table seed mismatch vs expected LCG");
                expect = expect
                    .wrapping_mul(0x27bb2ee687b0b0fd)
                    .wrapping_add(0xb504f32d);
                (c.hmfree_func)(hash_to_arr(tc, 16), 16);
                (rs.hmfree_func)(hash_to_arr(tr, 16), 16);
            }
        });
    }
}

#[test]
fn cfg_c13b_rand_seed_affects_map_layout() {
    // The global seed must be honoured identically: build the same map under a
    // sweep of seeds and compare full internal state.
    let mut rng = Rng::new(13);
    let keys = BinKeys::random(&mut rng, 40, 8);
    for &seed in SEEDS {
        with_libs(seed, |c, rs| {
            let mut m = MapPair::new(c, rs, 16, 8, HM_BINARY, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
            }
            m.check("cfg_c13b map under seed sweep");
            m.free();
        });
    }
}
