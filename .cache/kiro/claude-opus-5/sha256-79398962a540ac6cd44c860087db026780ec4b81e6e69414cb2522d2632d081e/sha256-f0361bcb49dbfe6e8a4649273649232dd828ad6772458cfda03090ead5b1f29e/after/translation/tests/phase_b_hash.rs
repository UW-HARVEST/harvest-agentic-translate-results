//! Phase B rows 1–10 — pure hashing (the lowest-level entry points).

mod common;
use common::*;

const SEEDS: [usize; 5] = [0, 1, 0x3141_5926, usize::MAX, 0xDEAD_BEEF_CAFE_F00D];

fn hb(c: &Impl, r: &Impl, data: &mut [u8], len: usize, seed: usize, ctx: &str) {
    unsafe {
        let hc = (c.hash_bytes)(data.as_mut_ptr() as *mut _, len, seed);
        let hr = (r.hash_bytes)(data.as_mut_ptr() as *mut _, len, seed);
        assert_eq!(
            hc, hr,
            "hash_bytes({}) len={} seed={:#x} bytes={:02x?}",
            ctx,
            len,
            seed,
            &data[..len.min(data.len())]
        );
    }
}

/// CONFIGS row 1 — `len == 0`, boundary + random seeds.
#[test]
fn cfg_01_hash_bytes_len0() {
    run(1, |c, r| {
        let mut rng = Rng::new(0x5eed_1234);
        let mut data = [0u8; 8];
        for s in SEEDS {
            hb(c, r, &mut data, 0, s, "len0");
        }
        for _ in 0..64 {
            hb(c, r, &mut data, 0, rng.next_u64() as usize, "len0-rand");
        }
    });
}

/// CONFIGS row 2 — tail-only lengths 1..7 (each `switch` fall-through arm).
#[test]
fn cfg_02_hash_bytes_tail_only() {
    run(1, |c, r| {
        let mut rng = Rng::new(0x5eed_1235);
        for len in 1..8usize {
            for s in SEEDS {
                for _ in 0..64 {
                    let mut d = rng.bytes(8);
                    hb(c, r, &mut d, len, s, "tail");
                }
            }
        }
    });
}

/// CONFIGS row 3 — exact multiples of 8 (main loop only).
#[test]
fn cfg_03_hash_bytes_multiples_of_8() {
    run(1, |c, r| {
        let mut rng = Rng::new(0x5eed_1236);
        for len in [8usize, 16, 24, 32, 64, 128, 256] {
            for s in SEEDS {
                for _ in 0..32 {
                    let mut d = rng.bytes(len);
                    hb(c, r, &mut d, len, s, "mult8");
                }
            }
        }
    });
}

/// CONFIGS row 4 — non-multiples 9..71 (main loop + tail).
#[test]
fn cfg_04_hash_bytes_mixed_lengths() {
    run(1, |c, r| {
        let mut rng = Rng::new(0x5eed_1237);
        for len in 9..72usize {
            for s in [0usize, 0x3141_5926, usize::MAX] {
                for _ in 0..24 {
                    let mut d = rng.bytes(len);
                    hb(c, r, &mut d, len, s, "mixed");
                }
            }
        }
    });
}

/// CONFIGS row 5 — high-bit bytes at d[3]/d[7] (the sign-extension loader path).
#[test]
fn cfg_05_hash_bytes_sign_extension() {
    run(1, |c, r| {
        let mut rng = Rng::new(0x5eed_1238);
        for len in [4usize, 5, 6, 7, 8, 9, 12, 15, 16, 17, 24, 31, 32] {
            for hi3 in [0x80u8, 0xFF, 0x7F] {
                for hi7 in [0x80u8, 0xFF, 0x7F] {
                    for _ in 0..16 {
                        let mut d = rng.bytes(len.max(8) + 8);
                        d[3] = hi3;
                        if d.len() > 7 {
                            d[7] = hi7;
                        }
                        if d.len() > 11 {
                            d[11] = hi3;
                        }
                        if d.len() > 15 {
                            d[15] = hi7;
                        }
                        for s in SEEDS {
                            hb(c, r, &mut d, len, s, "signext");
                        }
                    }
                }
            }
        }
    });
}

/// CONFIGS row 6 — all-zero and all-0xFF buffers, len 0..40.
#[test]
fn cfg_06_hash_bytes_extreme_buffers() {
    run(1, |c, r| {
        for len in 0..41usize {
            for fill in [0x00u8, 0xFF, 0x80, 0x7F] {
                let mut d = vec![fill; len.max(1)];
                for s in SEEDS {
                    hb(c, r, &mut d, len, s, "extreme");
                }
            }
        }
    });
}

/// CONFIGS row 7 — `hash_string("")`.
#[test]
fn cfg_07_hash_string_empty() {
    run(1, |c, r| unsafe {
        let mut e = [0i8; 1];
        for s in SEEDS {
            let hc = (c.hash_string)(e.as_mut_ptr(), s);
            let hr = (r.hash_string)(e.as_mut_ptr(), s);
            assert_eq!(hc, hr, "hash_string(\"\") seed={:#x}", s);
        }
    });
}

/// CONFIGS row 8 — random ASCII strings, length 1..64.
#[test]
fn cfg_08_hash_string_ascii() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0x5eed_1239);
        for len in 1..65usize {
            for s in SEEDS {
                for _ in 0..16 {
                    let mut v = rng.ascii(len);
                    let hc = (c.hash_string)(v.as_mut_ptr() as *mut i8, s);
                    let hr = (r.hash_string)(v.as_mut_ptr() as *mut i8, s);
                    assert_eq!(
                        hc,
                        hr,
                        "hash_string len={} seed={:#x} s={:?}",
                        len,
                        s,
                        String::from_utf8_lossy(&v[..len])
                    );
                }
            }
        }
    });
}

/// CONFIGS row 9 — bytes 0x80..0xFF (the `(unsigned char)` cast path).
#[test]
fn cfg_09_hash_string_high_bytes() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0x5eed_123a);
        for len in 1..48usize {
            for _ in 0..24 {
                // non-zero bytes across the whole 1..=255 range
                let mut v: Vec<u8> = (0..len)
                    .map(|_| 1u8 + (rng.next_u64() % 255) as u8)
                    .collect();
                v.push(0);
                for s in SEEDS {
                    let hc = (c.hash_string)(v.as_mut_ptr() as *mut i8, s);
                    let hr = (r.hash_string)(v.as_mut_ptr() as *mut i8, s);
                    assert_eq!(hc, hr, "hash_string high-bytes len={} v={:02x?}", len, &v);
                }
            }
        }
    });
}

/// CONFIGS row 10 — `stbds_rand_seed` drives the seed stored in a fresh index,
/// and the seed advance formula `seed = seed*a + b` must match step for step.
#[test]
fn cfg_10_rand_seed_and_index_seed_advance() {
    for gseed in [0usize, 1, 0x3141_5926, usize::MAX, 0xABCD_EF01_2345_6789] {
        run(gseed, |c, r| unsafe {
            // Each shmode_func makes a fresh index (ot == NULL) and consumes /
            // advances the global seed.  Ten in a row exposes any divergence in
            // the multiply-add step.
            let mut maps_c = Vec::new();
            let mut maps_r = Vec::new();
            for i in 0..10 {
                let mc = (c.shmode_func)(16, SH_ARENA);
                let mr = (r.shmode_func)(16, SH_ARENA);
                let hc = header_of(mc, 16);
                let hr = header_of(mr, 16);
                let tc = (*hc).hash_table as *mut HashIndex;
                let tr = (*hr).hash_table as *mut HashIndex;
                assert_eq!(
                    (*tc).seed,
                    (*tr).seed,
                    "index seed #{} diverged for global seed {:#x}",
                    i,
                    gseed
                );
                maps_c.push(mc);
                maps_r.push(mr);
            }
            for (i, m) in maps_c.iter().enumerate() {
                (c.hmfree_func)((*m as *mut u8).wrapping_sub(16) as *mut _, 16);
                let m2 = maps_r[i];
                (r.hmfree_func)((m2 as *mut u8).wrapping_sub(16) as *mut _, 16);
            }
        });
    }
}
