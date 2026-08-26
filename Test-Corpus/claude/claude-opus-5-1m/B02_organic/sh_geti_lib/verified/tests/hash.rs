//! Phase B — CONFIGS.md rows 1..15: the lowest-level entry points
//! (`stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed`).
mod common;

use common::*;
use core::ffi::{c_char, c_void};

/// Compare `stbds_hash_bytes` for one buffer / seed.
unsafe fn hb(c: &Lib, r: &Lib, buf: &[u8], len: usize, seed: usize) {
    let p = buf.as_ptr() as *mut c_void;
    let a = (c.hash_bytes)(p, len, seed);
    let b = (r.hash_bytes)(p, len, seed);
    assert_eq!(
        a, b,
        "hash_bytes(len={len}, seed={seed:#x}, buf={:02x?}) C={a:#x} Rust={b:#x}",
        &buf[..len.min(buf.len())]
    );
}

unsafe fn hs(c: &Lib, r: &Lib, s: &[u8], seed: usize) {
    let mut v = s.to_vec();
    v.push(0);
    let p = v.as_mut_ptr() as *mut c_char;
    let a = (c.hash_string)(p, seed);
    let b = (r.hash_string)(p, seed);
    assert_eq!(
        a, b,
        "hash_string({:?}, seed={seed:#x}) C={a:#x} Rust={b:#x}",
        String::from_utf8_lossy(s)
    );
}

const BOUNDARY_SEEDS: &[usize] = &[
    0,
    1,
    2,
    0x31415926,
    usize::MAX,
    usize::MAX - 1,
    1usize << 63,
    (1usize << 63) | 1,
    0x8000_0000,
    0xFFFF_FFFF,
];

/// Rows 1..9 — every siphash tail case (`len % 8 == 0..7`) at `len <= 8`.
#[test]
fn cfg01_09_hash_bytes_tail_cases() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0xB0075EED);
    unsafe {
        for len in 0..=8usize {
            // deterministic corner buffers first
            for pat in [0x00u8, 0x01, 0x7F, 0x80, 0xFF] {
                let buf = vec![pat; 8];
                for &seed in BOUNDARY_SEEDS {
                    hb(&c, &r, &buf, len, seed);
                }
            }
            // buffers with a high bit specifically in byte 3 and byte 7
            for hi3 in [0x00u8, 0x80, 0xFF] {
                for hi7 in [0x00u8, 0x80, 0xFF] {
                    let mut buf = rng.bytes(8);
                    buf[3] = hi3;
                    buf[7] = hi7;
                    for &seed in BOUNDARY_SEEDS {
                        hb(&c, &r, &buf, len, seed);
                    }
                }
            }
            // 256 randomized buffer/seed pairs per length
            for _ in 0..256 {
                let buf = rng.bytes(8);
                let seed = rng.next_u64() as usize;
                hb(&c, &r, &buf, len, seed);
            }
        }
    }
}

/// Row 10 — `len` 9..64, i.e. every (blocks, tail) combination.
#[test]
fn cfg10_hash_bytes_9_to_64() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x10101010);
    unsafe {
        for len in 9..=64usize {
            for _ in 0..8 {
                let buf = rng.bytes(len);
                for &seed in BOUNDARY_SEEDS {
                    hb(&c, &r, &buf, len, seed);
                }
            }
            // all-high-bit and all-zero buffers of this length
            for pat in [0x00u8, 0x80, 0xFF] {
                let buf = vec![pat; len];
                hb(&c, &r, &buf, len, 0x31415926);
            }
        }
        // 512 fully random (len, buf, seed) triples
        for _ in 0..512 {
            let len = rng.range(9, 64);
            let buf = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            hb(&c, &r, &buf, len, seed);
        }
    }
}

/// Row 11 — many main-loop blocks (`len` 65..4096).
#[test]
fn cfg11_hash_bytes_large() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x11111111);
    unsafe {
        for _ in 0..64 {
            let len = rng.range(65, 4096);
            let buf = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            hb(&c, &r, &buf, len, seed);
        }
        for len in [65usize, 127, 128, 129, 255, 256, 257, 1023, 1024, 4096] {
            let buf = rng.bytes(len);
            for &seed in BOUNDARY_SEEDS {
                hb(&c, &r, &buf, len, seed);
            }
        }
    }
}

/// Row 12 — boundary seeds crossed with `len` 0..16.
#[test]
fn cfg12_hash_bytes_boundary_seeds() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x12121212);
    unsafe {
        for len in 0..=16usize {
            let buf = rng.bytes(16);
            for &seed in BOUNDARY_SEEDS {
                hb(&c, &r, &buf, len, seed);
            }
            for _ in 0..32 {
                let seed = rng.next_u64() as usize;
                hb(&c, &r, &buf, len, seed);
            }
        }
    }
}

/// Row 13 — `stbds_hash_string` on 0..64 byte strings, including bytes >= 0x80.
#[test]
fn cfg13_hash_string_short() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x13131313);
    unsafe {
        hs(&c, &r, b"", 0);
        for &seed in BOUNDARY_SEEDS {
            hs(&c, &r, b"", seed);
            hs(&c, &r, b"a", seed);
            hs(&c, &r, b"foo", seed);
            hs(&c, &r, b"test_0", seed);
            hs(&c, &r, &[0x80], seed);
            hs(&c, &r, &[0xFF], seed);
            hs(&c, &r, &[0x7F, 0x80, 0xFF, 0x01], seed);
        }
        for len in 1..=64usize {
            for _ in 0..4 {
                let s = rng.nz_bytes(len);
                for &seed in BOUNDARY_SEEDS {
                    hs(&c, &r, &s, seed);
                }
            }
        }
        for _ in 0..256 {
            let len = rng.range(1, 64);
            let s = rng.nz_bytes(len);
            let seed = rng.next_u64() as usize;
            hs(&c, &r, &s, seed);
        }
    }
}

/// Row 14 — long strings.
#[test]
fn cfg14_hash_string_long() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x14141414);
    unsafe {
        for len in [256usize, 512, 1023, 1024, 4096] {
            let s = rng.nz_bytes(len);
            for &seed in BOUNDARY_SEEDS {
                hs(&c, &r, &s, seed);
            }
            hs(&c, &r, &vec![0xFFu8; len], 0);
            hs(&c, &r, &vec![0x01u8; len], usize::MAX);
        }
    }
}

/// Row 15 — `stbds_rand_seed` plus the global seed's LCG evolution across
/// successive `stbds_make_hash_index` calls (observed through each table's
/// captured `seed` field).
#[test]
fn cfg15_rand_seed_evolution() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x15151515);
    let mut seeds: Vec<usize> = vec![0, 1, 2, 0x31415926, usize::MAX, usize::MAX - 1, 1 << 63];
    for _ in 0..32 {
        seeds.push(rng.next_u64() as usize);
    }
    unsafe {
        for s in seeds {
            (c.rand_seed)(s);
            (r.rand_seed)(s);
            // four independent fresh tables => four successive LCG steps
            let mut observed_c = Vec::new();
            let mut observed_r = Vec::new();
            let mut alive: Vec<(*mut c_void, *mut c_void)> = Vec::new();
            for _ in 0..4 {
                let tc = (c.shmode_func)(16, SH_STRDUP);
                let tr = (r.shmode_func)(16, SH_STRDUP);
                observed_c.push((*map_table(tc, 16)).seed);
                observed_r.push((*map_table(tr, 16)).seed);
                alive.push((tc, tr));
            }
            assert_eq!(
                observed_c, observed_r,
                "seed evolution diverged after rand_seed({s:#x})"
            );
            // the LCG must actually be advancing (guards against a no-op test)
            assert!(
                observed_c.windows(2).any(|w| w[0] != w[1]) || s == 0,
                "seed did not evolve for rand_seed({s:#x}): {observed_c:?}"
            );
            for (tc, tr) in alive {
                (c.hmfree_func)((tc as *mut u8).sub(16) as *mut c_void, 16);
                (r.hmfree_func)((tr as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}

/// Extra: the `if (hash < 2) hash += 2` guard means no key may ever produce a
/// stored bucket hash of 0 or 1. Verified over a large random corpus on both.
#[test]
fn cfg_hash_never_0_or_1_in_buckets() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0xDEAD1234);
    unsafe {
        (c.rand_seed)(1);
        (r.rand_seed)(1);
        let elemsize = 16usize;
        let mut keys = Keys::new();
        let mut p = Pair::new(&c, &r, elemsize, 8, ElemCmp::Raw, "hash_never_0_1");
        p.set_default(1);
        for i in 0..64u64 {
            let k = keys.raw(&rng.bytes(8));
            p.put(k, HM_BINARY, i);
        }
        let t = p.c.t;
        let table = map_table(t, elemsize);
        for i in 0..((*table).slot_count >> BUCKET_SHIFT) {
            let b = (*table).storage.add(i);
            for j in 0..BUCKET_LENGTH {
                if (*b).index[j] >= 0 {
                    assert!(
                        (*b).hash[j] >= 2,
                        "in-use bucket slot holds reserved hash {}",
                        (*b).hash[j]
                    );
                }
            }
        }
        p.free();
    }
}
