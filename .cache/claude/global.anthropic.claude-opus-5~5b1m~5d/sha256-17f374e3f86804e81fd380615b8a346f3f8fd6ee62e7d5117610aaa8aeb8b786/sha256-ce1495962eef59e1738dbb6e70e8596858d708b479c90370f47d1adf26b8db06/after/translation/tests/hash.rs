//! Phase B — CONFIGS.md rows 1..=12: the two hash primitives and the global
//! seed state, exercised through the `.so` exports of both libraries.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

const SEEDS: &[usize] = &[
    0,
    1,
    2,
    0x31415926,
    0xFFFF_FFFF,
    0x8000_0000_0000_0000,
    usize::MAX,
    0x0706_0504_0302_0100,
];

fn both_hash_bytes(buf: &mut [u8], seed: usize) {
    let p = common::libs();
    let ptr = if buf.is_empty() {
        std::ptr::null_mut()
    } else {
        buf.as_mut_ptr() as *mut c_void
    };
    let hc = unsafe { (p.c.hash_bytes)(ptr, buf.len(), seed) };
    let hr = unsafe { (p.r.hash_bytes)(ptr, buf.len(), seed) };
    assert_eq!(
        hc, hr,
        "stbds_hash_bytes mismatch: len={} seed={:#x} bytes={:02x?}",
        buf.len(),
        seed,
        buf
    );
}

/// row 1 — len == 0, p == NULL
#[test]
fn r01_hash_bytes_zero_len_null_ptr() {
    let p = common::libs();
    for &seed in SEEDS {
        let hc = unsafe { (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        let hr = unsafe { (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        assert_eq!(hc, hr, "seed={seed:#x}");
    }
    // also with a real (non-null) pointer and len 0
    let mut b = [0u8; 8];
    for &seed in SEEDS {
        let hc = unsafe { (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, seed) };
        let hr = unsafe { (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, seed) };
        assert_eq!(hc, hr, "seed={seed:#x}");
    }
}

/// rows 2,3 — len 1..=8 (all `switch` fall-through cases + the exact-8 case)
#[test]
fn r02_r03_hash_bytes_short_lengths() {
    let mut rng = Rng::new(0xA5A5_0001);
    for len in 1..=8usize {
        for _ in 0..400 {
            let mut b = rng.bytes(len);
            for &seed in SEEDS {
                both_hash_bytes(&mut b, seed);
            }
            let s = rng.next_u64() as usize;
            both_hash_bytes(&mut b, s);
        }
    }
}

/// row 4 — len 9..=64: k main-loop iterations + every tail remainder
#[test]
fn r04_hash_bytes_medium_lengths() {
    let mut rng = Rng::new(0xA5A5_0002);
    for len in 9..=64usize {
        for _ in 0..80 {
            let mut b = rng.bytes(len);
            for &seed in SEEDS {
                both_hash_bytes(&mut b, seed);
            }
        }
    }
}

/// row 5 — high bit set at offsets 3 and 7 of every 8-byte group
/// (the `int` sign-extension quirk in the main loop)
#[test]
fn r05_hash_bytes_sign_extension_main_loop() {
    let mut rng = Rng::new(0xA5A5_0003);
    for len in [8usize, 16, 24, 32, 40, 64, 128] {
        for _ in 0..200 {
            let mut b = rng.bytes(len);
            for i in 0..len {
                if i % 8 == 3 || i % 8 == 7 {
                    b[i] |= 0x80;
                }
            }
            for &seed in SEEDS {
                both_hash_bytes(&mut b, seed);
            }
        }
    }
    // and the complementary case: high bit cleared everywhere
    for len in [8usize, 16, 24, 32] {
        for _ in 0..200 {
            let mut b = rng.bytes(len);
            for x in b.iter_mut() {
                *x &= 0x7f;
            }
            for &seed in SEEDS {
                both_hash_bytes(&mut b, seed);
            }
        }
    }
}

/// row 6 — tail cases 4..=7 with `d[3] >= 0x80` (sign extension in `case 4:`)
#[test]
fn r06_hash_bytes_sign_extension_tail() {
    let mut rng = Rng::new(0xA5A5_0004);
    for extra in 4..=7usize {
        for base in [0usize, 8, 16] {
            let len = base + extra;
            for _ in 0..300 {
                let mut b = rng.bytes(len);
                b[base + 3] |= 0x80;
                if extra >= 5 {
                    b[base + 4] |= 0x80;
                }
                if extra >= 6 {
                    b[base + 5] |= 0x80;
                }
                if extra >= 7 {
                    b[base + 6] |= 0x80;
                }
                for &seed in SEEDS {
                    both_hash_bytes(&mut b, seed);
                }
                // also with those bytes forced low
                let mut c = b.clone();
                for x in c.iter_mut() {
                    *x &= 0x7f;
                }
                for &seed in SEEDS {
                    both_hash_bytes(&mut c, seed);
                }
            }
        }
    }
}

/// row 7 — random seeds across the whole `size_t` range
#[test]
fn r07_hash_bytes_random_seeds() {
    let mut rng = Rng::new(0xA5A5_0005);
    for _ in 0..4000 {
        let len = rng.below(40);
        let mut b = rng.bytes(len);
        let seed = rng.next_u64() as usize;
        both_hash_bytes(&mut b, seed);
    }
    // extreme seeds with 0xff / 0x00 filled buffers
    for len in 0..=33usize {
        for fill in [0x00u8, 0xff, 0x80, 0x7f] {
            let mut b = vec![fill; len];
            for &seed in SEEDS {
                both_hash_bytes(&mut b, seed);
            }
        }
    }
}

/// row 8 — large buffers
#[test]
fn r08_hash_bytes_large_buffers() {
    let mut rng = Rng::new(0xA5A5_0006);
    for len in [1024usize, 1025, 2047, 2048, 4096, 4097] {
        for _ in 0..8 {
            let mut b = rng.bytes(len);
            for &seed in SEEDS {
                both_hash_bytes(&mut b, seed);
            }
        }
    }
}

fn both_hash_string(s: &mut [u8], seed: usize) {
    let p = common::libs();
    assert_eq!(*s.last().unwrap(), 0, "must be NUL terminated");
    let hc = unsafe { (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
    let hr = unsafe { (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
    assert_eq!(
        hc, hr,
        "stbds_hash_string mismatch: seed={seed:#x} s={:02x?}",
        s
    );
}

/// row 9 — empty string
#[test]
fn r09_hash_string_empty() {
    let mut rng = Rng::new(0xA5A5_0007);
    let mut e = [0u8; 1];
    for &seed in SEEDS {
        both_hash_string(&mut e, seed);
    }
    for _ in 0..2000 {
        let seed = rng.next_u64() as usize;
        both_hash_string(&mut e, seed);
    }
}

/// row 10 — ASCII strings, length 1..=64
#[test]
fn r10_hash_string_ascii() {
    let mut rng = Rng::new(0xA5A5_0008);
    for len in 1..=64usize {
        for _ in 0..60 {
            let mut s = rng.cstring(len);
            for &seed in SEEDS {
                both_hash_string(&mut s, seed);
            }
        }
    }
}

/// row 11 — strings containing bytes 0x80..=0xFF (the `(unsigned char)` cast)
#[test]
fn r11_hash_string_high_bytes() {
    let mut rng = Rng::new(0xA5A5_0009);
    for len in 1..=48usize {
        for _ in 0..60 {
            let mut s = rng.cstring_high(len);
            for &seed in SEEDS {
                both_hash_string(&mut s, seed);
            }
        }
    }
    // long strings: many rotate/add rounds
    let mut rng2 = Rng::new(0xA5A5_1009);
    for len in [255usize, 256, 511, 512, 1024, 4095, 4096, 8193] {
        for _ in 0..4 {
            let mut s = rng2.cstring(len);
            for &seed in SEEDS {
                both_hash_string(&mut s, seed);
            }
            let mut s2 = rng2.cstring_high(len);
            for &seed in SEEDS {
                both_hash_string(&mut s2, seed);
            }
        }
    }
    // all-0xff strings of every length
    for len in 1..=40usize {
        let mut s = vec![0xffu8; len];
        s.push(0);
        for &seed in SEEDS {
            both_hash_string(&mut s, seed);
        }
    }
}

/// row 12 — `stbds_rand_seed` + the global seed advance performed by every
/// fresh `stbds_make_hash_index`.  Observed through `table->seed` of maps
/// created back-to-back.
#[test]
fn r12_rand_seed_and_global_seed_advance() {
    let _g = lock_libs();
    let p = common::libs();
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut rng = Rng::new(0xA5A5_000A);

    for trial in 0..40 {
        let seed = if trial == 0 {
            0usize
        } else if trial == 1 {
            usize::MAX
        } else {
            rng.next_u64() as usize
        };
        unsafe {
            (p.c.rand_seed)(seed);
            (p.r.rand_seed)(seed);
        }
        // create 6 fresh maps in a row; each one consumes/advances the global
        // seed, so the sequence of table->seed values must match exactly.
        let mut c_maps = Vec::new();
        let mut r_maps = Vec::new();
        for i in 0..6u64 {
            let mut k = (i as u64).to_le_bytes();
            let tc = unsafe {
                (p.c.hmput_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    keysize,
                    STBDS_HM_BINARY,
                )
            };
            let tr = unsafe {
                (p.r.hmput_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    keysize,
                    STBDS_HM_BINARY,
                )
            };
            unsafe {
                let ic = put_value(tc, elemsize, keysize, i);
                let ir = put_value(tr, elemsize, keysize, i);
                assert_eq!(ic, ir, "stbds_temp mismatch");
            }
            let sc = unsafe { map_snap(tc, elemsize, keysize, KeyRepr::Raw) };
            let sr = unsafe { map_snap(tr, elemsize, keysize, KeyRepr::Raw) };
            assert_eq!(sc, sr, "seed={seed:#x} map#{i}");
            c_maps.push(tc);
            r_maps.push(tr);
        }
        for (tc, tr) in c_maps.into_iter().zip(r_maps) {
            unsafe {
                (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
    // leave both libraries at the documented default so other tests are stable
    unsafe {
        (p.c.rand_seed)(0x31415926);
        (p.r.rand_seed)(0x31415926);
    }
}
