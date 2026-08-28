//! Level 1: leaf functions with no internal state dependencies.
//! `stbds_hash_bytes`, `stbds_hash_string`, `stbds_arrgrowf`, `stbds_arrfreef`,
//! `strkey`.

mod common;

use common::*;
use std::ffi::c_void;

#[test]
fn hash_bytes_matches() {
    let _g = guard();
    let libs = libs();
    let mut rng = Rng::new(0xDEAD_BEEF_1234_5678);

    // Every length 0..=64 crosses the 8-byte main loop and each of the
    // `switch (len - i)` fall-through cases.
    let seeds: [usize; 8] = [
        0,
        1,
        0x3141_5926,
        usize::MAX,
        0x8000_0000_0000_0000,
        0xFFFF_FFFF,
        0xA5A5_A5A5_A5A5_A5A5,
        12345,
    ];

    for len in 0usize..=64 {
        for &seed in &seeds {
            // Deterministic bytes, plus a few adversarial patterns.
            let mut variants: Vec<Vec<u8>> = Vec::new();
            variants.push(vec![0u8; len]);
            variants.push(vec![0xFFu8; len]);
            variants.push((0..len).map(|i| (i * 31 + 7) as u8).collect());
            variants.push((0..len).map(|_| rng.next_u64() as u8).collect());
            // High-bit-set bytes exercise the sign-extension in `d[3] << 24`.
            variants.push((0..len).map(|i| 0x80u8 | (i as u8)).collect());

            for v in &variants {
                let mut buf = v.clone();
                buf.push(0); // keep a valid pointer for len == 0
                let p = buf.as_mut_ptr() as *mut c_void;
                let a = unsafe { libs.c.hash_bytes(p, len, seed) };
                let b = unsafe { libs.rs.hash_bytes(p, len, seed) };
                assert_eq!(
                    a, b,
                    "hash_bytes mismatch len={len} seed={seed:#x} bytes={v:?}"
                );
            }
        }
    }
}

#[test]
fn hash_bytes_long_inputs() {
    let _g = guard();
    let libs = libs();
    let mut rng = Rng::new(99);
    for len in [65usize, 100, 127, 128, 255, 256, 1000, 4096] {
        let mut buf: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        buf.push(0);
        let p = buf.as_mut_ptr() as *mut c_void;
        for seed in [0usize, 7, usize::MAX, 0x0123_4567_89AB_CDEF] {
            let a = unsafe { libs.c.hash_bytes(p, len, seed) };
            let b = unsafe { libs.rs.hash_bytes(p, len, seed) };
            assert_eq!(a, b, "hash_bytes mismatch len={len} seed={seed:#x}");
        }
    }
}

#[test]
fn hash_string_matches() {
    let _g = guard();
    let libs = libs();
    let mut rng = Rng::new(0x1122_3344_5566_7788);

    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"ab".to_vec(),
        b"test_0".to_vec(),
        b"test_12345".to_vec(),
        b"the quick brown fox jumps over the lazy dog".to_vec(),
        vec![0xFF; 1],
        vec![0xFF; 17],
        vec![0x80; 9],
    ];
    for len in 1usize..40 {
        inputs.push((0..len).map(|_| ((rng.next_u64() % 255) + 1) as u8).collect());
    }
    // Long string to exercise many rotate/add rounds.
    inputs.push((0..1000).map(|i| ((i % 255) + 1) as u8).collect());

    for inp in &inputs {
        let mut buf = CStrBuf::from_bytes(inp);
        let p = buf.as_ptr();
        for seed in [
            0usize,
            1,
            0x3141_5926,
            usize::MAX,
            0x8000_0000_0000_0000,
            0xDEAD_BEEF,
        ] {
            let a = unsafe { libs.c.hash_string(p, seed) };
            let b = unsafe { libs.rs.hash_string(p, seed) };
            assert_eq!(
                a,
                b,
                "hash_string mismatch seed={seed:#x} str={:?}",
                String::from_utf8_lossy(inp)
            );
        }
    }
}

#[test]
fn arrgrowf_from_null() {
    let _g = guard();
    let libs = libs();
    for elemsize in [1usize, 2, 4, 8, 16, 20, 24] {
        for addlen in [0usize, 1, 2, 3, 4, 5, 7, 8, 100, 1000] {
            for min_cap in [0usize, 1, 2, 3, 4, 5, 8, 64, 4096] {
                unsafe {
                    let a = libs.c.arrgrowf(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let b = libs.rs.arrgrowf(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let sa = snap_arr(a, Fmt::Raw, 0);
                    let sb = snap_arr(b, Fmt::Raw, 0);
                    assert_eq!(
                        sa, sb,
                        "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap}) mismatch"
                    );
                    // `arrgrowf` returns its input untouched when no growth is
                    // required, which for a NULL input means NULL - and
                    // `arrfreef(NULL)` is undefined in the C original.
                    if !a.is_null() {
                        libs.c.arrfreef(a as *mut c_void);
                    }
                    if !b.is_null() {
                        libs.rs.arrfreef(b as *mut c_void);
                    }
                }
            }
        }
    }
}

#[test]
fn arrgrowf_repeated_growth() {
    let _g = guard();
    let libs = libs();
    let elemsize = 4usize;
    unsafe {
        let mut a = libs.c.arrgrowf(std::ptr::null_mut(), elemsize, 1, 0);
        let mut b = libs.rs.arrgrowf(std::ptr::null_mut(), elemsize, 1, 0);
        assert_eq!(snap_arr(a, Fmt::Raw, 0), snap_arr(b, Fmt::Raw, 0));

        // Emulate `arrput` growth: push one element at a time, and record the
        // header after every step. Element payloads are compared too.
        for i in 0..2000i32 {
            let ha = (a as *mut ArrHeader).offset(-1);
            let hb = (b as *mut ArrHeader).offset(-1);
            if (*ha).length + 1 > (*ha).capacity {
                a = libs.c.arrgrowf(a as *mut c_void, elemsize, 1, 0);
            }
            if (*hb).length + 1 > (*hb).capacity {
                b = libs.rs.arrgrowf(b as *mut c_void, elemsize, 1, 0);
            }
            let ha = (a as *mut ArrHeader).offset(-1);
            let hb = (b as *mut ArrHeader).offset(-1);
            (a as *mut i32).add((*ha).length).write(i);
            (*ha).length += 1;
            (b as *mut i32).add((*hb).length).write(i);
            (*hb).length += 1;

            let n = (*ha).length;
            assert_eq!(
                snap_arr(a, Fmt::Raw, n / 2),
                snap_arr(b, Fmt::Raw, n / 2),
                "growth mismatch at i={i}"
            );
        }
        libs.c.arrfreef(a as *mut c_void);
        libs.rs.arrfreef(b as *mut c_void);
    }
}

#[test]
fn arrgrowf_addlen_and_mincap_on_existing() {
    let _g = guard();
    let libs = libs();
    let elemsize = 8usize;
    for &(addlen, min_cap) in &[
        (0usize, 0usize),
        (0, 1),
        (0, 4),
        (0, 5),
        (1, 0),
        (3, 0),
        (4, 0),
        (5, 0),
        (0, 9),
        (9, 3),
        (100, 7),
        (0, 1024),
    ] {
        unsafe {
            // Start from a 4-capacity array with length 2.
            let mut a = libs.c.arrgrowf(std::ptr::null_mut(), elemsize, 2, 0);
            let mut b = libs.rs.arrgrowf(std::ptr::null_mut(), elemsize, 2, 0);
            (*(a as *mut ArrHeader).offset(-1)).length = 2;
            (*(b as *mut ArrHeader).offset(-1)).length = 2;
            std::ptr::write_bytes(a, 0xAB, 2 * elemsize);
            std::ptr::write_bytes(b, 0xAB, 2 * elemsize);

            a = libs.c.arrgrowf(a as *mut c_void, elemsize, addlen, min_cap);
            b = libs.rs.arrgrowf(b as *mut c_void, elemsize, addlen, min_cap);
            assert_eq!(
                snap_arr(a, Fmt::Raw, 2),
                snap_arr(b, Fmt::Raw, 2),
                "arrgrowf(existing, {elemsize}, {addlen}, {min_cap}) mismatch"
            );
            libs.c.arrfreef(a as *mut c_void);
            libs.rs.arrfreef(b as *mut c_void);
        }
    }
}

#[test]
fn strkey_matches() {
    let _g = guard();
    let libs = libs();
    let mut rng = Rng::new(0x5EED);
    let mut cases: Vec<i32> = vec![0, 1, -1, 9, 10, 99, 100, 12345, i32::MAX, i32::MIN, -42];
    for _ in 0..200 {
        cases.push(rng.next_i32());
    }
    for n in cases {
        unsafe {
            let a = read_cstr(libs.c.strkey(n)).expect("non-null");
            let b = read_cstr(libs.rs.strkey(n)).expect("non-null");
            assert_eq!(
                a,
                b,
                "strkey({n}) mismatch: {:?} vs {:?}",
                String::from_utf8_lossy(&a),
                String::from_utf8_lossy(&b)
            );
        }
    }
}

#[test]
fn strkey_returns_same_static_buffer() {
    let _g = guard();
    // `strkey` writes into a file-static buffer and returns it, so successive
    // calls must return the same address within one library.
    let libs = libs();
    unsafe {
        let p1 = libs.c.strkey(1);
        let p2 = libs.c.strkey(2);
        assert_eq!(p1, p2, "C strkey buffer moved");
        let q1 = libs.rs.strkey(1);
        let q2 = libs.rs.strkey(2);
        assert_eq!(q1, q2, "Rust strkey buffer moved");
    }
}
