//! Phase B rows 1-10: the two lowest-level entry points (`stbds_hash_bytes`,
//! `stbds_hash_string`) plus the seed plumbing (`stbds_rand_seed`).
mod common;
use common::*;
use std::ffi::{c_char, c_void};

const SEEDS: &[usize] = &[
    0,
    1,
    2,
    0x31415926,
    usize::MAX,
    usize::MAX - 1,
    0x8000_0000_0000_0000,
    0xdead_beef_cafe_babe,
];

fn hb(bytes: &[u8], seed: usize) -> (usize, usize) {
    let l = libs();
    let mut c_buf = bytes.to_vec();
    let mut r_buf = bytes.to_vec();
    unsafe {
        (
            (l.c.hash_bytes)(c_buf.as_mut_ptr() as *mut c_void, bytes.len(), seed),
            (l.r.hash_bytes)(r_buf.as_mut_ptr() as *mut c_void, bytes.len(), seed),
        )
    }
}

fn assert_hb(bytes: &[u8], seed: usize) {
    let (a, b) = hb(bytes, seed);
    assert_eq!(
        a, b,
        "hash_bytes mismatch len={} seed={:#x} bytes={:02x?}",
        bytes.len(),
        seed,
        bytes
    );
}

// row 1
#[test]
fn row01_hash_bytes_len0() {
    let _s = session(0x31415926);
    for &s in SEEDS {
        assert_hb(&[], s);
    }
    let mut rng = Rng::new();
    for _ in 0..200 {
        assert_hb(&[], rng.next_u64() as usize);
    }
}

// row 2
#[test]
fn row02_hash_bytes_tail_only() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(2);
    for len in 1..8usize {
        for &s in SEEDS {
            for _ in 0..40 {
                assert_hb(&rng.bytes(len), s);
            }
        }
    }
}

// row 3
#[test]
fn row03_hash_bytes_len8() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(3);
    for &s in SEEDS {
        for _ in 0..200 {
            assert_hb(&rng.bytes(8), s);
        }
    }
}

// row 4
#[test]
fn row04_hash_bytes_len9_15() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(4);
    for len in 9..16usize {
        for &s in SEEDS {
            for _ in 0..40 {
                assert_hb(&rng.bytes(len), s);
            }
        }
    }
}

// row 5
#[test]
fn row05_hash_bytes_multiword() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(5);
    for &len in &[16usize, 17, 24, 31, 32, 63, 64, 65, 127, 128, 1000] {
        for &s in SEEDS {
            for _ in 0..20 {
                assert_hb(&rng.bytes(len), s);
            }
        }
    }
}

// row 6 -- int sign-extension paths
#[test]
fn row06_hash_bytes_sign_extension() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(6);
    for len in 0..40usize {
        for &s in SEEDS {
            assert_hb(&vec![0x00u8; len], s);
            assert_hb(&vec![0xffu8; len], s);
            assert_hb(&vec![0x80u8; len], s);
            assert_hb(&vec![0x7fu8; len], s);
            // force the high bit at byte 3 and byte 7 of every word
            let mut v = rng.bytes(len);
            for (i, b) in v.iter_mut().enumerate() {
                if i % 8 == 3 || i % 8 == 7 {
                    *b |= 0x80;
                } else {
                    *b &= 0x7f;
                }
            }
            assert_hb(&v, s);
            let mut v2 = rng.bytes(len);
            for (i, b) in v2.iter_mut().enumerate() {
                if i % 8 == 3 || i % 8 == 7 {
                    *b &= 0x7f;
                } else {
                    *b |= 0x80;
                }
            }
            assert_hb(&v2, s);
        }
    }
}

// row 7 -- exhaustive-ish seed sweep with random data
#[test]
fn row07_hash_bytes_seed_sweep() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(7);
    for _ in 0..4000 {
        let len = rng.below(80) as usize;
        let seed = match rng.below(5) {
            0 => 0,
            1 => 1,
            2 => usize::MAX,
            3 => 0x31415926,
            _ => rng.next_u64() as usize,
        };
        assert_hb(&rng.bytes(len), seed);
    }
}

fn assert_hs(s: &[u8], seed: usize) {
    assert_eq!(*s.last().unwrap(), 0, "must be NUL terminated");
    let l = libs();
    let mut cb = s.to_vec();
    let mut rb = s.to_vec();
    let (a, b) = unsafe {
        (
            (l.c.hash_string)(cb.as_mut_ptr() as *mut c_char, seed),
            (l.r.hash_string)(rb.as_mut_ptr() as *mut c_char, seed),
        )
    };
    assert_eq!(a, b, "hash_string mismatch seed={seed:#x} s={s:02x?}");
}

// row 8
#[test]
fn row08_hash_string_ascii() {
    let _s = session(0x31415926);
    assert_hs(b"\0", 0);
    for &s in SEEDS {
        assert_hs(b"\0", s);
        assert_hs(b"a\0", s);
        assert_hs(b"abcdefg\0", s);
        assert_hs(b"abcdefgh\0", s);
        assert_hs(b"abcdefghi\0", s);
        assert_hs(b"test_0\0", s);
    }
    let mut rng = Rng::with(8);
    for len in 0..70usize {
        for &s in SEEDS {
            assert_hs(&rng.cstring(len), s);
        }
    }
}

// row 9 -- high-bit bytes go through `(unsigned char) *str`
#[test]
fn row09_hash_string_high_bytes() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(9);
    for len in 0..70usize {
        for &s in SEEDS {
            let mut v: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u64() & 0x7f) as u8).collect();
            v.push(0);
            assert_hs(&v, s);
            let mut w: Vec<u8> = (0..len).map(|_| 1 + (rng.next_u64() % 255) as u8).collect();
            w.push(0);
            assert_hs(&w, s);
        }
    }
}

// row 10 -- rand_seed drives the seed of the *next* fresh table, and the LCG
// chain thereafter. Build several fresh tables and compare their `seed` fields.
#[test]
fn row10_rand_seed_chain() {
    let _s = session(0x31415926);
    let elemsize = 8usize;
    for &start in &[0usize, 1, 0x31415926, usize::MAX, 0xabcd_1234_5678_9f01] {
        seed_both(start);
        for round in 0..6u64 {
            let mut m = MapPair::null(elemsize, "rand_seed");
            let mut key: u32 = 7;
            m.put(&mut key as *mut u32 as *mut c_void, 4, HM_BINARY);
            // the 4 value bytes are raw realloc memory until written
            unsafe { m.fill_tail(4, round) };
            m.assert_same(&format!("fresh table (start={start:#x} round={round})"));
            assert!(m.snap_c().table.is_some());
            m.free();
        }
    }
}
