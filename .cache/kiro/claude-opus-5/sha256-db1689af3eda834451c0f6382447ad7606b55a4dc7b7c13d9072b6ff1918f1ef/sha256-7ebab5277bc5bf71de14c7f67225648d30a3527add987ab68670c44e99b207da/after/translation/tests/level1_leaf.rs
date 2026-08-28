//! Level 1: leaf functions with no internal state —
//! `stbds_hash_string`, `stbds_hash_bytes`, `stbds_arrgrowf`, `stbds_arrfreef`,
//! `strkey`.
mod harness;

use harness::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

fn seeds() -> Vec<usize> {
    vec![
        0,
        1,
        2,
        3,
        0x31415926,
        0xffff_ffff,
        0xffff_ffff_ffff_ffff,
        0x8000_0000_0000_0000,
        0x0123_4567_89ab_cdef,
        0xdead_beef_cafe_babe,
        usize::MAX - 1,
        1 << 32,
        0x5555_5555_5555_5555,
        0xaaaa_aaaa_aaaa_aaaa,
    ]
}

fn strings() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"ab".to_vec(),
        b"abc".to_vec(),
        b"abcd".to_vec(),
        b"abcde".to_vec(),
        b"abcdef".to_vec(),
        b"abcdefg".to_vec(),
        b"abcdefgh".to_vec(),
        b"abcdefghi".to_vec(),
        b"test_0".to_vec(),
        b"test_12345".to_vec(),
        b"The quick brown fox jumps over the lazy dog".to_vec(),
        vec![0x80],
        vec![0xff],
        vec![0xff, 0x80, 0x7f, 0x01],
        vec![0xff; 33],
        vec![0x80; 64],
    ];
    // deterministic pseudo-random strings, incl. high-bit bytes
    let mut state: u64 = 0x1234_5678_9abc_def0;
    for len in 1..=48usize {
        let mut s = Vec::with_capacity(len);
        for _ in 0..len {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let b = ((state >> 33) & 0xff) as u8;
            s.push(if b == 0 { 1 } else { b });
        }
        v.push(s);
    }
    v
}

#[test]
fn hash_string_matches() {
    let p = pair();
    for s in strings() {
        let mut buf = s.clone();
        buf.push(0);
        for seed in seeds() {
            let cv = unsafe { (p.c.hash_string)(buf.as_mut_ptr() as *mut c_char, seed) };
            let rv = unsafe { (p.rs.hash_string)(buf.as_mut_ptr() as *mut c_char, seed) };
            assert_eq!(
                cv, rv,
                "stbds_hash_string({:?}, {:#x}) C={:#x} Rust={:#x}",
                String::from_utf8_lossy(&s),
                seed,
                cv,
                rv
            );
        }
    }
}

#[test]
fn hash_bytes_matches() {
    let p = pair();
    // Byte buffers of every length 0..=80, over several fill patterns.
    let mut patterns: Vec<Vec<u8>> = Vec::new();
    patterns.push(vec![0u8; 96]);
    patterns.push(vec![0xffu8; 96]);
    patterns.push((0..96u8).collect());
    patterns.push((0..96u8).map(|i| 0x80 | i).collect());
    patterns.push((0..96u8).rev().collect());
    let mut state: u64 = 0xfeed_face_dead_beef;
    for _ in 0..6 {
        let mut b = Vec::with_capacity(96);
        for _ in 0..96 {
            state = state.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
            b.push(((state >> 29) & 0xff) as u8);
        }
        patterns.push(b);
    }

    for pat in &patterns {
        let mut buf = pat.clone();
        for len in 0..=80usize {
            for seed in seeds() {
                let cv = unsafe { (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                let rv = unsafe { (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                assert_eq!(cv, rv, "stbds_hash_bytes(len={len}, seed={seed:#x})");
            }
        }
    }
}

#[test]
fn hash_bytes_unaligned_offsets() {
    let p = pair();
    let mut buf: Vec<u8> = (0..128u8).map(|i| i.wrapping_mul(37) | 0x81).collect();
    for off in 0..16usize {
        for len in 0..=48usize {
            for seed in [0usize, 0x31415926, usize::MAX] {
                let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut c_void;
                let cv = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
                let rv = unsafe { (p.rs.hash_bytes)(ptr, len, seed) };
                assert_eq!(cv, rv, "hash_bytes(off={off}, len={len}, seed={seed:#x})");
            }
        }
    }
}

/// `stbds_arrgrowf` growth policy + resulting header contents.
#[test]
fn arrgrowf_matches() {
    let p = pair();
    for elemsize in [1usize, 2, 4, 8, 12, 16, 24, 64] {
        for &(addlen, min_cap) in &[
            (0usize, 0usize),
            (0, 1),
            (1, 0),
            (1, 1),
            (2, 0),
            (3, 9),
            (5, 4),
            (0, 4),
            (0, 5),
            (7, 100),
            (100, 7),
        ] {
            let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
            let ra = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
            assert_eq!(
                ca.is_null(),
                ra.is_null(),
                "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap}) nullness mismatch"
            );
            if ca.is_null() {
                // min_cap <= 0 == arrcap(NULL) → the C code returns `a` unchanged.
                continue;
            }
            let cs = unsafe { header_snap(ca) };
            let rs = unsafe { header_snap(ra) };
            assert_eq!(
                cs, rs,
                "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap}) header mismatch"
            );

            // Now grow the same array repeatedly and compare after each step.
            let mut cp = ca;
            let mut rp = ra;
            for step in 0..24usize {
                // pretend the caller pushed elements: bump length like arrput
                unsafe {
                    let ch = (cp as *mut ArrayHeader).offset(-1);
                    let rh = (rp as *mut ArrayHeader).offset(-1);
                    if (*ch).length < (*ch).capacity {
                        (*ch).length += 1;
                        (*rh).length += 1;
                    }
                }
                let add = step % 5;
                let mc = (step * 3) % 17;
                cp = unsafe { (p.c.arrgrowf)(cp, elemsize, add, mc) };
                rp = unsafe { (p.rs.arrgrowf)(rp, elemsize, add, mc) };
                let cs = unsafe { header_snap(cp) };
                let rs = unsafe { header_snap(rp) };
                assert_eq!(
                    cs, rs,
                    "arrgrowf step {step} (elemsize={elemsize}, add={add}, min_cap={mc}) mismatch"
                );
            }
            unsafe {
                (p.c.arrfreef)(cp);
                (p.rs.arrfreef)(rp);
            }
        }
    }
}

/// `strkey` writes into a private static buffer and returns it.
#[test]
fn strkey_matches() {
    let _g = global_lock();
    let p = pair();
    for n in [
        0i32,
        1,
        7,
        9,
        10,
        99,
        100,
        12345,
        -1,
        -42,
        i32::MAX,
        i32::MIN,
        1_000_000,
    ] {
        let cv = unsafe { cstr((p.c.strkey)(n as c_int)) };
        let rv = unsafe { cstr((p.rs.strkey)(n as c_int)) };
        assert_eq!(cv, rv, "strkey({n})");
    }
    // Repeated calls must keep reusing the same buffer address.
    let c1 = unsafe { (p.c.strkey)(1) };
    let c2 = unsafe { (p.c.strkey)(2) };
    let r1 = unsafe { (p.rs.strkey)(1) };
    let r2 = unsafe { (p.rs.strkey)(2) };
    assert_eq!(c1 == c2, r1 == r2, "strkey buffer reuse differs");
}
