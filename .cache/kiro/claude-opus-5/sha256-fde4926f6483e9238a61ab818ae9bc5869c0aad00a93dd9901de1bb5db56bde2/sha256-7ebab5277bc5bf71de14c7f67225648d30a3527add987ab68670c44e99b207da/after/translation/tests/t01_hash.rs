//! Lowest level: the pure hash functions and the global seed setter.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

#[test]
fn hash_bytes_matches() {
    let p = load_pair();
    let mut rng = Rng::new(0xDEAD_BEEF);

    // exhaustively cover every length class 0..=64 (the siphash tail switch has
    // one case per `len % 8`), several seeds, and many random payloads.
    for len in 0..=64usize {
        for trial in 0..24 {
            let mut buf = rng.bytes(len.max(1));
            if trial % 3 == 0 {
                // force high bytes so the C `int` sign-extension quirk is hit
                for b in buf.iter_mut() {
                    *b |= 0x80;
                }
            }
            if trial % 5 == 0 {
                for b in buf.iter_mut() {
                    *b = 0xff;
                }
            }
            for &seed in &[
                0usize,
                1,
                0x3141_5926,
                0xffff_ffff_ffff_ffff,
                0x8000_0000_0000_0000,
                rng.next_u64() as usize,
            ] {
                let ptr = buf.as_mut_ptr() as *mut c_void;
                let cv = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
                let rv = unsafe { (p.r.hash_bytes)(ptr, len, seed) };
                assert_eq!(
                    cv, rv,
                    "hash_bytes(len={len}, seed={seed:#x}, buf={:02x?})",
                    &buf[..len.min(buf.len())]
                );
            }
        }
    }
}

#[test]
fn hash_bytes_all_single_byte_values() {
    let p = load_pair();
    for b in 0..=255u8 {
        for len in 1..=9usize {
            let mut buf = vec![b; len];
            let ptr = buf.as_mut_ptr() as *mut c_void;
            for &seed in &[0usize, 7, 0x3141_5926, usize::MAX] {
                let cv = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
                let rv = unsafe { (p.r.hash_bytes)(ptr, len, seed) };
                assert_eq!(cv, rv, "hash_bytes(byte={b:#x}, len={len}, seed={seed:#x})");
            }
        }
    }
}

#[test]
fn hash_string_matches() {
    let p = load_pair();
    let mut rng = Rng::new(0x1234_5678);

    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"ab".to_vec(),
        b"test_0".to_vec(),
        b"test_123456".to_vec(),
        b"the quick brown fox jumps over the lazy dog".to_vec(),
        vec![0x80],
        vec![0xff; 33],
        vec![0x7f, 0x80, 0x81, 0xfe, 0xff],
    ];
    for len in 1..40usize {
        let mut v = rng.bytes(len);
        for b in v.iter_mut() {
            if *b == 0 {
                *b = 1;
            }
        }
        cases.push(v);
        let mut hi = rng.bytes(len);
        for b in hi.iter_mut() {
            *b |= 0x80;
        }
        cases.push(hi);
    }

    for case in &cases {
        let mut buf = cbuf_bytes(case);
        for &seed in &[
            0usize,
            1,
            0x3141_5926,
            usize::MAX,
            0x8000_0000_0000_0000,
            0xdead_beef_cafe_babe,
        ] {
            let ptr = buf.as_mut_ptr() as *mut c_char;
            let cv = unsafe { (p.c.hash_string)(ptr, seed) };
            let rv = unsafe { (p.r.hash_string)(ptr, seed) };
            assert_eq!(cv, rv, "hash_string({case:02x?}, seed={seed:#x})");
        }
    }
}

#[test]
fn rand_seed_affects_new_tables_identically() {
    // stbds_rand_seed only writes the file-static seed; its effect is observable
    // through the seed stored in a freshly created hash index.
    let p = load_pair();
    let elemsize = 16usize;

    for &seed in &[0usize, 1, 42, 0x3141_5926, usize::MAX] {
        unsafe {
            (p.c.rand_seed)(seed);
            (p.r.rand_seed)(seed);

            let ct = (p.c.shmode_func)(elemsize, SH_STRDUP);
            let rt = (p.r.shmode_func)(elemsize, SH_STRDUP);

            let craw = raw_of(ct, elemsize);
            let rraw = raw_of(rt, elemsize);
            assert_eq!(
                dump_map(craw, elemsize, false),
                dump_map(rraw, elemsize, false),
                "shmode_func after rand_seed({seed:#x})"
            );

            (p.c.hmfree_func)(craw, elemsize);
            (p.r.hmfree_func)(rraw, elemsize);
        }
    }
}
