//! Phase B, Groups 1-3 of `CONFIGS.md`:
//! `stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed` + the
//! `make_hash_index` seed chain.
//!
//! Both libraries are loaded via `libloading` and called only through their
//! exported symbols.

mod common;

use common::*;
use core::ffi::{c_char, c_void};

fn hb(p: &Pair, buf: &[u8], len: usize, seed: usize) -> (usize, usize) {
    unsafe {
        let ptr = if buf.is_empty() {
            std::ptr::null_mut()
        } else {
            buf.as_ptr() as *mut c_void
        };
        ((p.c.hash_bytes)(ptr, len, seed), (p.r.hash_bytes)(ptr, len, seed))
    }
}

fn hs(p: &Pair, s: &[u8], seed: usize) -> (usize, usize) {
    // s must be NUL terminated
    assert_eq!(*s.last().unwrap(), 0);
    unsafe {
        let ptr = s.as_ptr() as *mut c_char;
        (
            (p.c.hash_string)(ptr, seed),
            (p.r.hash_string)(ptr, seed),
        )
    }
}

const SEEDS: [usize; 10] = [
    0,
    1,
    2,
    0x3141_5926,
    0xdead_beef,
    0x8000_0000_0000_0000,
    0xffff_ffff_ffff_ffff,
    0x0f0e_0d0c_0b0a_0908,
    0x7fff_ffff_ffff_ffff,
    0xaaaa_aaaa_aaaa_aaaa,
];

// ---------------------------------------------------------------------------
// C1 — len == 0, p == NULL
// ---------------------------------------------------------------------------
#[test]
fn cfg_c1_hash_bytes_len0_null() {
    let p = libs();
    let mut rng = Rng::new(1);
    for i in 0..64 {
        let seed = if i < SEEDS.len() {
            SEEDS[i]
        } else {
            rng.next_u64() as usize
        };
        let (c, r) = hb(&p, &[], 0, seed);
        diff_eq!(c, r, "hash_bytes(NULL, 0, {:#x})", seed);
    }
    // also len 0 with a non-null pointer
    let buf = [1u8, 2, 3, 4, 5, 6, 7, 8];
    for &seed in SEEDS.iter() {
        let (c, r) = hb(&p, &buf, 0, seed);
        diff_eq!(c, r, "hash_bytes(buf, 0, {:#x})", seed);
    }
}

// ---------------------------------------------------------------------------
// C2 / C3 — len 1..7, one row per `switch` case, low bytes and high bytes
// ---------------------------------------------------------------------------
#[test]
fn cfg_c2_c3_hash_bytes_tail_cases() {
    let p = libs();
    let mut rng = Rng::new(2);
    for len in 1..=7usize {
        // low bytes only
        for _ in 0..200 {
            let buf: Vec<u8> = (0..8).map(|_| (rng.next_u32() % 0x80) as u8).collect();
            for &seed in SEEDS.iter() {
                let (c, r) = hb(&p, &buf, len, seed);
                diff_eq!(c, r, "hash_bytes(low,{len},{seed:#x}) buf={buf:?}");
            }
        }
        // force the high bit at every offset (int sign-extension quirk)
        for off in 0..len {
            for _ in 0..64 {
                let mut buf: Vec<u8> = rng.bytes(8);
                buf[off] |= 0x80;
                for &seed in SEEDS.iter() {
                    let (c, r) = hb(&p, &buf, len, seed);
                    diff_eq!(c, r, "hash_bytes(high@{off},{len},{seed:#x}) buf={buf:?}");
                }
            }
        }
        // all bytes high
        let buf = vec![0xFFu8; 8];
        for &seed in SEEDS.iter() {
            let (c, r) = hb(&p, &buf, len, seed);
            diff_eq!(c, r, "hash_bytes(0xff,{len},{seed:#x})");
        }
    }
}

// ---------------------------------------------------------------------------
// C4 / C5 / C6 — main-loop boundaries
// ---------------------------------------------------------------------------
#[test]
fn cfg_c4_c5_c6_hash_bytes_main_loop() {
    let p = libs();
    let mut rng = Rng::new(3);
    for len in [8usize, 9, 10, 11, 12, 13, 14, 15, 16, 17, 23, 24, 25, 31, 32, 33] {
        for _ in 0..200 {
            let buf: Vec<u8> = rng.bytes(len + 8);
            for &seed in SEEDS.iter() {
                let (c, r) = hb(&p, &buf, len, seed);
                diff_eq!(c, r, "hash_bytes(rand,{len},{seed:#x})");
            }
        }
        // high bit at every byte position
        for off in 0..len {
            let mut buf = vec![0x01u8; len + 8];
            buf[off] = 0xFF;
            let (c, r) = hb(&p, &buf, len, DEFAULT_SEED);
            diff_eq!(c, r, "hash_bytes(hi@{off},{len})");
        }
    }
}

// ---------------------------------------------------------------------------
// C7 — 2000 fully random cases, len 1..256
// ---------------------------------------------------------------------------
#[test]
fn cfg_c7_hash_bytes_random() {
    let p = libs();
    let mut rng = Rng::new(7);
    for _ in 0..2000 {
        let len = rng.range(1, 256);
        let buf = rng.bytes(len);
        let seed = rng.next_u64() as usize;
        let (c, r) = hb(&p, &buf, len, seed);
        diff_eq!(c, r, "hash_bytes(rand,{len},{seed:#x})");
    }
}

// ---------------------------------------------------------------------------
// C8 — long messages
// ---------------------------------------------------------------------------
#[test]
fn cfg_c8_hash_bytes_long() {
    let p = libs();
    let mut rng = Rng::new(8);
    let buf = rng.bytes(4096);
    for &seed in SEEDS.iter() {
        for len in [4096usize, 4095, 4089, 2048, 1000, 999] {
            let (c, r) = hb(&p, &buf, len, seed);
            diff_eq!(c, r, "hash_bytes(4k,{len},{seed:#x})");
        }
    }
}

// ---------------------------------------------------------------------------
// C9 — degenerate buffers
// ---------------------------------------------------------------------------
#[test]
fn cfg_c9_hash_bytes_degenerate() {
    let p = libs();
    for fill in [0x00u8, 0xFF, 0x80, 0x7F, 0x01] {
        let buf = vec![fill; 64];
        for len in 0..=64usize {
            for &seed in SEEDS.iter() {
                let (c, r) = hb(&p, &buf, len, seed);
                diff_eq!(c, r, "hash_bytes({fill:#x}*{len},{seed:#x})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C10 — hash_string("")
// ---------------------------------------------------------------------------
#[test]
fn cfg_c10_hash_string_empty() {
    let p = libs();
    let mut rng = Rng::new(10);
    let empty = [0u8];
    for i in 0..64 {
        let seed = if i < SEEDS.len() {
            SEEDS[i]
        } else {
            rng.next_u64() as usize
        };
        let (c, r) = hs(&p, &empty, seed);
        diff_eq!(c, r, "hash_string(\"\", {seed:#x})");
    }
}

// ---------------------------------------------------------------------------
// C11 — every single-byte string
// ---------------------------------------------------------------------------
#[test]
fn cfg_c11_hash_string_single_bytes() {
    let p = libs();
    for b in 1u16..=255 {
        let s = [b as u8, 0];
        for &seed in SEEDS.iter() {
            let (c, r) = hs(&p, &s, seed);
            diff_eq!(c, r, "hash_string([{b:#x}], {seed:#x})");
        }
    }
}

// ---------------------------------------------------------------------------
// C12 / C13 — random ASCII and random full-byte-range strings
// ---------------------------------------------------------------------------
#[test]
fn cfg_c12_c13_hash_string_random() {
    let p = libs();
    let mut rng = Rng::new(12);
    for full in [false, true] {
        for _ in 0..2000 {
            let n = rng.range(1, 64);
            let mut s = rng.cstr_body(n, full);
            s.push(0);
            let seed = rng.next_u64() as usize;
            let (c, r) = hs(&p, &s, seed);
            diff_eq!(c, r, "hash_string(rand full={full}, len={n}, {seed:#x})");
        }
    }
}

// ---------------------------------------------------------------------------
// C14 — very long string
// ---------------------------------------------------------------------------
#[test]
fn cfg_c14_hash_string_long() {
    let p = libs();
    let mut rng = Rng::new(14);
    for full in [false, true] {
        let mut s = rng.cstr_body(4096, full);
        s.push(0);
        for &seed in SEEDS.iter() {
            let (c, r) = hs(&p, &s, seed);
            diff_eq!(c, r, "hash_string(4k full={full}, {seed:#x})");
        }
    }
}

// ---------------------------------------------------------------------------
// C15 — strkey() output fed into hash_string, cross-library
// ---------------------------------------------------------------------------
#[test]
fn cfg_c15_hash_string_of_strkey() {
    let p = libs();
    for n in -20i32..=1000 {
        unsafe {
            let cp = (p.c.strkey)(n);
            let rp = (p.r.strkey)(n);
            let cs = read_cstr(cp);
            let rs = read_cstr(rp);
            diff_eq!(cs.clone(), rs.clone(), "strkey({n}) text");
            // hash both libraries' own buffer AND the other one's, so a
            // divergence in either strkey or hash_string is caught.
            for &seed in SEEDS.iter() {
                let a = (p.c.hash_string)(cp, seed);
                let b = (p.r.hash_string)(rp, seed);
                diff_eq!(a, b, "hash_string(strkey({n}), {seed:#x})");
                let a2 = (p.c.hash_string)(rp, seed);
                let b2 = (p.r.hash_string)(cp, seed);
                diff_eq!(a2, b2, "hash_string(cross strkey({n}), {seed:#x})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C16 / C17 — rand_seed and the make_hash_index LCG chain
// ---------------------------------------------------------------------------
fn seed_chain(lib: &Lib, initial: usize, n: usize) -> Vec<usize> {
    let mut out = Vec::new();
    unsafe {
        (lib.rand_seed)(initial);
        let mut tables = Vec::new();
        for _ in 0..n {
            let t = (lib.shmode_func)(16, STBDS_SH_ARENA);
            let raw = (t as *mut u8).sub(16) as *mut c_void;
            let idx = (*hdr_of(raw)).hash_table as *mut CHashIndex;
            out.push((*idx).seed);
            tables.push((raw, 16usize));
        }
        for (raw, e) in tables {
            (lib.hmfree_func)(raw, e);
        }
    }
    out
}

#[test]
fn cfg_c16_c17_seed_chain() {
    let p = libs();
    let mut rng = Rng::new(17);
    let mut seeds: Vec<usize> = SEEDS.to_vec();
    for _ in 0..32 {
        seeds.push(rng.next_u64() as usize);
    }
    for &s in seeds.iter() {
        let c = seed_chain(&p.c, s, 9);
        let r = seed_chain(&p.r, s, 9);
        diff_eq!(c.clone(), r.clone(), "seed chain from {s:#x}");
        // the first table always uses the seed verbatim
        assert_eq!(c[0], s, "first table seed should be the value just set");
    }
    reset_seed(&p, DEFAULT_SEED);
}
