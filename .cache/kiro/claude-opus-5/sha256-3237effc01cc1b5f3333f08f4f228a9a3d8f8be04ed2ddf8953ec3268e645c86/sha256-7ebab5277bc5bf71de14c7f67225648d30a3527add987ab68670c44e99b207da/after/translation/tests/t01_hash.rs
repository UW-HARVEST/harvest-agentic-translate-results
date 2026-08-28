//! Level 1: the pure leaf functions -- `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed` and `strkey`.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

fn seeds() -> Vec<usize> {
    vec![
        0,
        1,
        2,
        0xff,
        DEFAULT_SEED,
        0x8000_0000,
        0x0000_0000_ffff_ffff,
        0xffff_ffff_ffff_ffff,
        0x1234_5678_9abc_def0,
        0xdead_beef_cafe_babe,
        usize::MAX - 1,
    ]
}

#[test]
fn hash_bytes_matches() {
    let p = load_pair();

    // deterministic pseudo-random payload
    let mut buf = [0u8; 300];
    let mut x: u64 = 0x243f_6a88_85a3_08d3;
    for b in buf.iter_mut() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = (x >> 24) as u8;
    }

    let mut total = 0usize;
    for &seed in &seeds() {
        // every length 0..=200 and a few longer ones, at 8 different offsets so
        // that both the aligned fast path and every switch fall-through arm run
        for off in 0..8usize {
            for len in 0..=200usize {
                let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut c_void;
                let cv = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
                let rv = unsafe { (p.r.hash_bytes)(ptr, len, seed) };
                assert_eq!(
                    cv, rv,
                    "hash_bytes(off={off}, len={len}, seed={seed:#x}): C {cv:#x} != Rust {rv:#x}"
                );
                total += 1;
            }
        }
    }
    assert!(total > 10_000);
}

#[test]
fn hash_bytes_all_byte_values() {
    let p = load_pair();
    // exercise the sign-extension quirks: high bit set in each position
    for len in 1..=9usize {
        for v in 0..=255u8 {
            let mut buf = [v; 16];
            for i in 0..len {
                buf[i] = if i % 2 == 0 { v } else { !v };
            }
            for &seed in &[0usize, DEFAULT_SEED, usize::MAX] {
                let ptr = buf.as_mut_ptr() as *mut c_void;
                let cv = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
                let rv = unsafe { (p.r.hash_bytes)(ptr, len, seed) };
                assert_eq!(cv, rv, "hash_bytes(len={len}, fill={v:#x}, seed={seed:#x})");
            }
        }
    }
}

#[test]
fn hash_bytes_int_keys() {
    // the exact call shape used by the hash-map path: keysize == sizeof(int)
    let p = load_pair();
    for &seed in &seeds() {
        for k in -5000i32..5000i32 {
            let mut key = k;
            let ptr = &mut key as *mut i32 as *mut c_void;
            let cv = unsafe { (p.c.hash_bytes)(ptr, 4, seed) };
            let rv = unsafe { (p.r.hash_bytes)(ptr, 4, seed) };
            assert_eq!(cv, rv, "hash_bytes(int {k}, seed={seed:#x})");
        }
    }
}

#[test]
fn hash_string_matches() {
    let p = load_pair();

    let mut strings: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"a\0".to_vec(),
        b"ab\0".to_vec(),
        b"test_0\0".to_vec(),
        b"test_123456\0".to_vec(),
        b"\xff\xfe\xfd\xfc\0".to_vec(),
        b"\x80\x80\x80\x80\x80\x80\x80\x80\0".to_vec(),
        b"The quick brown fox jumps over the lazy dog\0".to_vec(),
    ];
    // long strings and every single-byte value
    for v in 1..=255u8 {
        strings.push(vec![v, 0]);
        strings.push(vec![v, v, v, 0]);
    }
    for n in 0..64usize {
        let mut s: Vec<u8> = (0..n).map(|i| ((i * 37 + 13) % 255 + 1) as u8).collect();
        s.push(0);
        strings.push(s);
    }

    for &seed in &seeds() {
        for s in strings.iter_mut() {
            let ptr = s.as_mut_ptr() as *mut c_char;
            let cv = unsafe { (p.c.hash_string)(ptr, seed) };
            let rv = unsafe { (p.r.hash_string)(ptr, seed) };
            assert_eq!(
                cv, rv,
                "hash_string({:?}, seed={seed:#x})",
                String::from_utf8_lossy(&s[..s.len() - 1])
            );
        }
    }
}

#[test]
fn strkey_matches() {
    let p = load_pair();
    for n in [
        0i32,
        1,
        9,
        10,
        99,
        100,
        12345,
        -1,
        -999,
        i32::MAX,
        i32::MIN,
    ] {
        let cp = unsafe { (p.c.strkey)(n) };
        let rp = unsafe { (p.r.strkey)(n) };
        let cs = unsafe { std::ffi::CStr::from_ptr(cp) };
        let rs = unsafe { std::ffi::CStr::from_ptr(rp) };
        assert_eq!(cs.to_bytes(), rs.to_bytes(), "strkey({n})");
    }
    // the returned buffer is a static, so it must be stable across calls
    let a = unsafe { (p.r.strkey)(7) };
    let b = unsafe { (p.r.strkey)(8) };
    assert_eq!(a, b, "strkey must return the same static buffer");
}

#[test]
fn rand_seed_is_honoured() {
    // stbds_rand_seed has no observable return; it is verified indirectly by
    // checking that a fresh table built after seeding picks up the value.
    let p = load_pair();
    for &seed in &seeds() {
        p.reset_seed(seed);
        let elemsize = 8usize;
        let ct = unsafe { (p.c.hmput_key)(std::ptr::null_mut(), elemsize, [1i32].as_mut_ptr() as *mut c_void, 4, STBDS_HM_BINARY) };
        let rt = unsafe { (p.r.hmput_key)(std::ptr::null_mut(), elemsize, [1i32].as_mut_ptr() as *mut c_void, 4, STBDS_HM_BINARY) };
        let cs = unsafe { snapshot_map(ct, elemsize, false) };
        let rs = unsafe { snapshot_map(rt, elemsize, false) };
        assert_bytes_eq(&format!("table seeded with {seed:#x}"), &cs, &rs);
        unsafe {
            (p.c.hmfree_func)((ct as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((rt as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}
