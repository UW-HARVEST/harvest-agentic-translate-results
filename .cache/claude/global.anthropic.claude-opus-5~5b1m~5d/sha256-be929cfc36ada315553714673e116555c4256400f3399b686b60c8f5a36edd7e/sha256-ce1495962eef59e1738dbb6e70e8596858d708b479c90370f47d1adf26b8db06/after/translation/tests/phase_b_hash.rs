//! Phase B rows B9-B17: `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed` + seed propagation/advance.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;

const SEEDS: &[usize] = &[
    0,
    1,
    2,
    0x3141_5926,
    0xdead_beef,
    0x8000_0000_0000_0000,
    usize::MAX,
    usize::MAX - 1,
];

fn hb(api: &Api, buf: &mut [u8], len: usize, seed: usize) -> usize {
    unsafe { (api.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) }
}

fn hs(api: &Api, s: &mut Vec<u8>, seed: usize) -> usize {
    // s must be NUL terminated
    unsafe { (api.hash_string)(s.as_mut_ptr() as *mut c_char, seed) }
}

/// B10 — len == 0, with NULL and non-NULL p
#[test]
fn cfg_b10_hash_bytes_zero_len() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &seed in SEEDS {
            let hc = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let hr = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(hc, hr, "B10 NULL p seed={seed:#x}");
            let mut buf = [0xAAu8; 16];
            let hc = hb(c, &mut buf, 0, seed);
            let hr = hb(r, &mut buf, 0, seed);
            assert_eq!(hc, hr, "B10 non-NULL p seed={seed:#x}");
        }
    });
}

/// B11 — len 1..=8 (every tail residue plus one full block)
#[test]
fn cfg_b11_hash_bytes_len_1_8() {
    with_libs(DEFAULT_SEED, |c, r| {
        let mut rng = Rng::new(11);
        for len in 1usize..=8 {
            for _ in 0..200 {
                let mut buf = rng.bytes(len.max(8) + 8);
                for &seed in SEEDS {
                    let hc = hb(c, &mut buf, len, seed);
                    let hr = hb(r, &mut buf, len, seed);
                    assert_eq!(hc, hr, "B11 len={len} seed={seed:#x} buf={buf:?}");
                }
            }
        }
    });
}

/// B12 — len 9..=40 (full blocks + every tail residue)
#[test]
fn cfg_b12_hash_bytes_len_9_40() {
    with_libs(DEFAULT_SEED, |c, r| {
        let mut rng = Rng::new(12);
        for len in 9usize..=40 {
            for _ in 0..40 {
                let mut buf = rng.bytes(len + 8);
                for &seed in &SEEDS[..4] {
                    let hc = hb(c, &mut buf, len, seed);
                    let hr = hb(r, &mut buf, len, seed);
                    assert_eq!(hc, hr, "B12 len={len} seed={seed:#x}");
                }
            }
        }
    });
}

/// B13 — big lengths and extreme byte patterns (sign-extension paths)
#[test]
fn cfg_b13_hash_bytes_big_and_extremes() {
    with_libs(DEFAULT_SEED, |c, r| {
        let mut rng = Rng::new(13);
        for &len in &[41usize, 63, 64, 65, 100, 127, 255, 256, 1000, 1023] {
            let mut buf = rng.bytes(len + 8);
            for &seed in SEEDS {
                let hc = hb(c, &mut buf, len, seed);
                let hr = hb(r, &mut buf, len, seed);
                assert_eq!(hc, hr, "B13 random len={len} seed={seed:#x}");
            }
            for &fill in &[0x00u8, 0xFF, 0x80, 0x7F, 0x01] {
                let mut buf = vec![fill; len + 8];
                for &seed in SEEDS {
                    let hc = hb(c, &mut buf, len, seed);
                    let hr = hb(r, &mut buf, len, seed);
                    assert_eq!(hc, hr, "B13 fill={fill:#x} len={len} seed={seed:#x}");
                }
            }
        }
        // Bytes with the high bit set in *every* tail position (the
        // `data |= (d[3] << 24)` int-promotion sign extension).
        for len in 1usize..=8 {
            for pos in 0..len {
                for &hi in &[0x80u8, 0xFF] {
                    let mut buf = vec![0u8; 16];
                    buf[pos] = hi;
                    for &seed in SEEDS {
                        let hc = hb(c, &mut buf, len, seed);
                        let hr = hb(r, &mut buf, len, seed);
                        assert_eq!(hc, hr, "B13 hi byte len={len} pos={pos} hi={hi:#x}");
                    }
                }
            }
        }
    });
}

/// B14 — unaligned p
#[test]
fn cfg_b14_hash_bytes_unaligned() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut rng = Rng::new(14);
        let mut backing = rng.bytes(128);
        for off in 0usize..8 {
            for len in 0usize..=24 {
                for &seed in &SEEDS[..4] {
                    let p = backing.as_mut_ptr().add(off) as *mut c_void;
                    let hc = (c.hash_bytes)(p, len, seed);
                    let hr = (r.hash_bytes)(p, len, seed);
                    assert_eq!(hc, hr, "B14 off={off} len={len} seed={seed:#x}");
                }
            }
        }
    });
}

/// B15 — hash_string: empty, short, random ASCII
#[test]
fn cfg_b15_hash_string_ascii() {
    with_libs(DEFAULT_SEED, |c, r| {
        let mut rng = Rng::new(15);
        // empty
        for &seed in SEEDS {
            let mut s = vec![0u8];
            let hc = hs(c, &mut s, seed);
            let hr = hs(r, &mut s, seed);
            assert_eq!(hc, hr, "B15 empty seed={seed:#x}");
        }
        for len in 1usize..=64 {
            for _ in 0..20 {
                let mut s = rng.ascii(len);
                s.push(0);
                for &seed in &SEEDS[..4] {
                    let hc = hs(c, &mut s, seed);
                    let hr = hs(r, &mut s, seed);
                    assert_eq!(hc, hr, "B15 len={len} seed={seed:#x}");
                }
            }
        }
    });
}

/// B16 — hash_string with bytes >= 0x80 (`(unsigned char) *str++`)
#[test]
fn cfg_b16_hash_string_high_bytes() {
    with_libs(DEFAULT_SEED, |c, r| {
        let mut rng = Rng::new(16);
        for len in 1usize..=32 {
            for _ in 0..20 {
                let mut s = rng.cstr_bytes(len);
                s.push(0);
                for &seed in SEEDS {
                    let hc = hs(c, &mut s, seed);
                    let hr = hs(r, &mut s, seed);
                    assert_eq!(hc, hr, "B16 len={len} seed={seed:#x} s={s:?}");
                }
            }
        }
        // every single byte value on its own
        for b in 1u8..=255 {
            let mut s = vec![b, 0];
            for &seed in SEEDS {
                let hc = hs(c, &mut s, seed);
                let hr = hs(r, &mut s, seed);
                assert_eq!(hc, hr, "B16 single byte {b}");
            }
        }
    });
}

/// B17 — long strings
#[test]
fn cfg_b17_hash_string_long() {
    with_libs(DEFAULT_SEED, |c, r| {
        let mut rng = Rng::new(17);
        for &len in &[128usize, 256, 1000, 4096] {
            let mut s = rng.cstr_bytes(len);
            s.push(0);
            for &seed in SEEDS {
                let hc = hs(c, &mut s, seed);
                let hr = hs(r, &mut s, seed);
                assert_eq!(hc, hr, "B17 len={len} seed={seed:#x}");
            }
        }
    });
}

/// B9 — rand_seed: the seed the table records, and the global advance
#[test]
fn cfg_b9_rand_seed_and_advance() {
    for &seed in &[
        0usize,
        1,
        0x3141_5926,
        0xdead_beef,
        usize::MAX,
        0x8000_0000_0000_0000,
        12345,
        0xffff_ffff,
    ] {
        with_libs(seed, |c, r| unsafe {
            // Five *fresh* tables → five successive global seed values.
            let mut cseeds = Vec::new();
            let mut rseeds = Vec::new();
            for i in 0..5 {
                let mut m_c = Map::new(c, Shape::binary(8, 4));
                let mut m_r = Map::new(r, Shape::binary(8, 4));
                let key = 42i32.to_ne_bytes();
                let mut kc = key;
                let mut kr = key;
                let elem = [0u8; 8];
                m_c.put_struct(kc.as_mut_ptr() as *mut c_void, &elem, HM_BINARY);
                m_r.put_struct(kr.as_mut_ptr() as *mut c_void, &elem, HM_BINARY);
                let sc = m_c.snapshot();
                let sr = m_r.snapshot();
                assert_eq!(sc, sr, "B9 seed={seed:#x} table {i}");
                cseeds.push(sc.table.unwrap().seed);
                rseeds.push(sr.table.unwrap().seed);
                m_c.free();
                m_r.free();
            }
            assert_eq!(cseeds, rseeds, "B9 seed advance chain seed={seed:#x}");
            // the chain must actually advance (otherwise the test is vacuous)
            assert!(
                cseeds.windows(2).any(|w| w[0] != w[1]),
                "B9 seed chain did not advance"
            );
        });
    }
}

/// B9b — `stbds_rand_seed` is honoured mid-stream: reseed between tables.
#[test]
fn cfg_b9b_reseed_midstream() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut rng = Rng::new(99);
        for i in 0..20 {
            let s = rng.next_u64() as usize;
            (c.rand_seed)(s);
            (r.rand_seed)(s);
            let mut m_c = Map::new(c, Shape::binary(8, 4));
            let mut m_r = Map::new(r, Shape::binary(8, 4));
            for k in 0i32..10 {
                let mut elem = [0u8; 8];
                elem[..4].copy_from_slice(&k.to_ne_bytes());
                let mut kc = k.to_ne_bytes();
                let mut kr = k.to_ne_bytes();
                m_c.put_struct(kc.as_mut_ptr() as *mut c_void, &elem, HM_BINARY);
                m_r.put_struct(kr.as_mut_ptr() as *mut c_void, &elem, HM_BINARY);
            }
            assert_eq!(m_c.snapshot(), m_r.snapshot(), "B9b i={i} s={s:#x}");
            m_c.free();
            m_r.free();
        }
    });
}
