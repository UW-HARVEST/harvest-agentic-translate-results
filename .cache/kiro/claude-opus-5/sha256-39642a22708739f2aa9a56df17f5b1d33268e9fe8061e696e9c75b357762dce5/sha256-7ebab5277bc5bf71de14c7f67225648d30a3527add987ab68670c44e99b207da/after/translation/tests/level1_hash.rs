//! Level 1: the leaf, side-effect-free functions — `stbds_hash_string`,
//! `stbds_hash_bytes`, `stbds_rand_seed` and `strkey`.

mod harness;

use harness::*;
use std::ffi::{c_char, c_void};

fn cbuf(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

const SEEDS: [usize; 12] = [
    0,
    1,
    2,
    0x31415926,
    0xdeadbeef,
    0xffff_ffff,
    0x1_0000_0000,
    usize::MAX,
    usize::MAX - 1,
    0x8000_0000_0000_0000,
    0x0f0e_0d0c_0b0a_0908,
    0x7fff_ffff_ffff_ffff,
];

#[test]
fn hash_string_matches() {
    let p = pair();
    let mut cases: Vec<String> = vec![
        String::new(),
        "a".into(),
        "ab".into(),
        "abc".into(),
        "test_0".into(),
        "test_-1".into(),
        "hello, world".into(),
        "\u{7f}\u{7f}\u{7f}".into(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    ];
    // include high-bit bytes (they go through `(unsigned char) *str++`)
    cases.push(String::from_utf8(vec![0xc3, 0xbf, 0xc3, 0xbe]).unwrap());
    for n in 0..64 {
        cases.push(format!("key_{n}_{}", "x".repeat(n % 17)));
    }

    for s in &cases {
        let mut buf = cbuf(s);
        for &seed in SEEDS.iter() {
            let a = unsafe { (p.c.hash_string)(buf.as_mut_ptr(), seed) };
            let b = unsafe { (p.r.hash_string)(buf.as_mut_ptr(), seed) };
            assert_eq!(a, b, "hash_string({s:?}, {seed:#x})");
        }
    }
}

#[test]
fn hash_bytes_matches() {
    let p = pair();

    // deterministic pseudo-random byte source
    let mut state: u64 = 0x243f_6a88_85a3_08d3;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 24) as u8
    };

    for len in 0..=72usize {
        for trial in 0..6 {
            let mut data: Vec<u8> = (0..len)
                .map(|i| match trial {
                    0 => 0,
                    1 => 0xff,
                    2 => 0x80,
                    3 => i as u8,
                    4 => (0xff - i) as u8,
                    _ => next(),
                })
                .collect();
            for &seed in SEEDS.iter() {
                let a = unsafe {
                    (p.c.hash_bytes)(data.as_mut_ptr() as *mut c_void, len, seed)
                };
                let b = unsafe {
                    (p.r.hash_bytes)(data.as_mut_ptr() as *mut c_void, len, seed)
                };
                assert_eq!(
                    a, b,
                    "hash_bytes(len={len}, trial={trial}, seed={seed:#x}) data={data:?}"
                );
            }
        }
    }
}

#[test]
fn hash_bytes_zero_length_null_pointer() {
    let p = pair();
    for &seed in SEEDS.iter() {
        let a = unsafe { (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        let b = unsafe { (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        assert_eq!(a, b, "hash_bytes(NULL, 0, {seed:#x})");
    }
}

#[test]
fn strkey_matches() {
    let p = pair();
    for n in [
        0, 1, -1, 7, 42, -42, 1000, -1000, i32::MAX, i32::MIN, 123456789, -123456789,
    ] {
        let a = unsafe { snap::cstr((p.c.strkey)(n)) };
        let b = unsafe { snap::cstr((p.r.strkey)(n)) };
        assert_eq!(a, b, "strkey({n})");
        assert_eq!(a, Some(format!("test_{n}").into_bytes()));
    }
}

/// `stbds_rand_seed` is only observable through the seed a freshly created hash
/// index picks up; drive it via `stbds_shmode_func`.
#[test]
fn rand_seed_matches() {
    let p = pair();
    for &seed in SEEDS.iter() {
        unsafe {
            (p.c.rand_seed)(seed);
            (p.r.rand_seed)(seed);
            // three consecutive tables so the seed-advance recurrence is checked
            for _ in 0..3 {
                let ct = (p.c.shmode_func)(16, STBDS_SH_DEFAULT);
                let rt = (p.r.shmode_func)(16, STBDS_SH_DEFAULT);
                let cs = snap::snap_map(ct, 16, snap::KeyKind::StrPtr);
                let rs = snap::snap_map(rt, 16, snap::KeyKind::StrPtr);
                assert_eq!(cs.index.as_ref().unwrap().seed, rs.index.as_ref().unwrap().seed);
                assert_eq!(cs, rs, "shmode_func table after rand_seed({seed:#x})");
                (p.c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
                (p.r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}
