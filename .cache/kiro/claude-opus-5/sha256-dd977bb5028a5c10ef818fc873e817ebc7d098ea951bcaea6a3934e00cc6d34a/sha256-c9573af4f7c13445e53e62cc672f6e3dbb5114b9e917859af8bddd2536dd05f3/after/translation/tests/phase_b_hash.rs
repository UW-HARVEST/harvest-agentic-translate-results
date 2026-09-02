//! Phase B — CONFIGS.md rows 1..7: the two hash primitives and the global seed.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;

fn hb(p: &Pair, buf: &mut [u8], len: usize, seed: usize) -> (usize, usize) {
    unsafe {
        let ptr = if buf.is_empty() {
            std::ptr::null_mut()
        } else {
            buf.as_mut_ptr() as *mut c_void
        };
        ((p.c.hash_bytes)(ptr, len, seed), (p.rs.hash_bytes)(ptr, len, seed))
    }
}

fn hs(p: &Pair, s: &mut [u8], seed: usize) -> (usize, usize) {
    unsafe {
        let ptr = s.as_mut_ptr() as *mut c_char;
        ((p.c.hash_string)(ptr, seed), (p.rs.hash_string)(ptr, seed))
    }
}

/// Row 1 — `hash_bytes` over lengths 0..=80 with randomized bytes.
#[test]
fn cfg_01_hash_bytes_random_lengths() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED);
    let mut n = 0usize;
    for len in 0..=80usize {
        for _ in 0..64 {
            let mut buf = rng.bytes(len.max(1));
            let (c, r) = hb(p, &mut buf, len, DEFAULT_SEED);
            assert_eq!(c, r, "hash_bytes len={len} buf={buf:?}");
            n += 1;
        }
    }
    assert!(n >= 5000);
}

/// Row 2 — the sign-extension quirk: byte 3 / 7 / 11 with the high bit set.
#[test]
fn cfg_02_hash_bytes_sign_extension_shapes() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 2);
    for len in 0..=24usize {
        for hi_idx in [3usize, 7, 11, 4, 8] {
            for hi in [0x80u8, 0xff, 0x7f, 0x00] {
                for _ in 0..24 {
                    let mut buf = rng.bytes(len.max(16));
                    if hi_idx < buf.len() {
                        buf[hi_idx] = hi;
                    }
                    let (c, r) = hb(p, &mut buf, len, DEFAULT_SEED);
                    assert_eq!(c, r, "len={len} hi_idx={hi_idx} hi={hi:#x} buf={buf:?}");
                }
            }
        }
    }
}

/// Row 3 — degenerate byte patterns.
#[test]
fn cfg_03_hash_bytes_patterns() {
    let (p, _g) = begin(DEFAULT_SEED);
    for len in 0..=80usize {
        let pats: [Vec<u8>; 5] = [
            vec![0x00; len.max(1)],
            vec![0xff; len.max(1)],
            (0..len.max(1)).map(|i| if i % 2 == 0 { 0x00 } else { 0x80 }).collect(),
            (0..len.max(1)).map(|i| i as u8).collect(),
            (0..len.max(1)).map(|i| 0x80u8 | (i as u8)).collect(),
        ];
        for mut b in pats {
            let (c, r) = hb(p, &mut b, len, DEFAULT_SEED);
            assert_eq!(c, r, "len={len} pat={b:?}");
        }
    }
}

/// Row 4 — seed axis (the C hash provably cancels the seed; both must agree).
#[test]
fn cfg_04_hash_bytes_seeds() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 4);
    let mut seeds: Vec<usize> = vec![0, 1, 2, usize::MAX, usize::MAX - 1, DEFAULT_SEED, 1 << 63];
    for _ in 0..32 {
        seeds.push(rng.next_usize());
    }
    for len in 0..=24usize {
        for &s in &seeds {
            for _ in 0..8 {
                let mut buf = rng.bytes(len.max(1));
                let (c, r) = hb(p, &mut buf, len, s);
                assert_eq!(c, r, "len={len} seed={s:#x}");
            }
        }
    }
}

/// Row 5 — `hash_string` over printable ASCII of many lengths × random seeds.
#[test]
fn cfg_05_hash_string_ascii() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 5);
    let lens = [0usize, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 63, 64, 65, 127, 255];
    let mut seeds: Vec<usize> = vec![0, 1, usize::MAX, DEFAULT_SEED];
    for _ in 0..16 {
        seeds.push(rng.next_usize());
    }
    for &l in &lens {
        for &s in &seeds {
            for _ in 0..16 {
                let mut cs = rng.cstring(l);
                let (c, r) = hs(p, &mut cs, s);
                assert_eq!(c, r, "len={l} seed={s:#x} str={cs:?}");
            }
        }
    }
}

/// Row 6 — `hash_string` with bytes >= 0x80 (`(unsigned char)` promotion).
#[test]
fn cfg_06_hash_string_high_bit() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 6);
    for l in 0..=64usize {
        for _ in 0..24 {
            let mut cs = rng.cstring_highbit(l);
            let (c, r) = hs(p, &mut cs, DEFAULT_SEED);
            assert_eq!(c, r, "len={l} str={cs:?}");
        }
        // deterministic all-0x80 / all-0xff variants
        for fill in [0x80u8, 0xffu8, 0x01u8] {
            let mut cs: Vec<u8> = vec![fill; l];
            cs.push(0);
            let (c, r) = hs(p, &mut cs, DEFAULT_SEED);
            assert_eq!(c, r, "fill={fill:#x} len={l}");
        }
    }
}

/// Row 7 — `rand_seed` plus the internal LCG advance performed by
/// `make_hash_index(ot == NULL)`, observed through `shmode_func`.
#[test]
fn cfg_07_seed_lcg_advance() {
    let mut rng = Rng::new(SEED ^ 7);
    let mut seeds: Vec<usize> = vec![0, 1, 2, DEFAULT_SEED, usize::MAX, 1 << 63];
    for _ in 0..16 {
        seeds.push(rng.next_usize());
    }
    for &s0 in &seeds {
        let (p, _g) = begin(s0);
        unsafe {
            // Ten successive fresh tables: each captures the current global seed
            // and then advances it by the fixed LCG.
            let mut cs = Vec::new();
            let mut rs = Vec::new();
            for _ in 0..10 {
                let cp = (p.c.shmode_func)(16, STBDS_SH_ARENA);
                let rp = (p.rs.shmode_func)(16, STBDS_SH_ARENA);
                let ct = (*header(hash_to_arr(cp, 16))).hash_table as *const HashIndex;
                let rt = (*header(hash_to_arr(rp, 16))).hash_table as *const HashIndex;
                cs.push((*ct).seed);
                rs.push((*rt).seed);
                assert_eq!(dump_map(cp, 16, true), dump_map(rp, 16, true), "seed0={s0:#x}");
                (p.c.hmfree_func)(hash_to_arr(cp, 16), 16);
                (p.rs.hmfree_func)(hash_to_arr(rp, 16), 16);
            }
            assert_eq!(cs, rs, "LCG stream mismatch for start seed {s0:#x}");
            assert_eq!(cs[0], s0, "first table must capture the seed verbatim");
        }
    }
}
