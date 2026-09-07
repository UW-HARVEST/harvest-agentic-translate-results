//! Phase B rows C1–C10: the lowest-level entry points — `stbds_hash_bytes`,
//! `stbds_hash_string`, `stbds_rand_seed`, `stbds_arrgrowf`, `stbds_arrfreef`.

mod common;
use common::*;
use std::ffi::c_void;

// ---------------------------------------------------------------------- C1
#[test]
fn c1_hash_bytes_all_lengths() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xC1);
    for len in 0..=80usize {
        for _ in 0..8 {
            let buf = rng.bytes(len.max(1));
            let p = buf.as_ptr() as *mut c_void;
            unsafe {
                let a = (s.c.hash_bytes)(p, len, 0);
                let b = (s.r.hash_bytes)(p, len, 0);
                assert_eq!(a, b, "hash_bytes len={len} buf={buf:02x?}");
            }
        }
    }
}

// ---------------------------------------------------------------------- C2
#[test]
fn c2_hash_bytes_random_seeds() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xC2);
    let mut seeds: Vec<usize> = vec![
        0,
        1,
        2,
        usize::MAX,
        usize::MAX - 1,
        0x31415926,
        0x8000_0000_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF,
        0x1_0000_0000,
    ];
    for _ in 0..200 {
        seeds.push(rng.next_u64() as usize);
    }
    for seed in seeds {
        for _ in 0..25 {
            let len = rng.below(65);
            let buf = rng.bytes(len.max(1));
            let p = buf.as_ptr() as *mut c_void;
            unsafe {
                assert_eq!(
                    (s.c.hash_bytes)(p, len, seed),
                    (s.r.hash_bytes)(p, len, seed),
                    "hash_bytes seed={seed:#x} len={len}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------- C3
#[test]
fn c3_hash_bytes_high_bit_bytes() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xC3);
    // Forces the `(d[3] << 24)` / `(d[7] << 24)` signed-int sign-extension paths.
    for len in 0..=40usize {
        for _ in 0..10 {
            let buf: Vec<u8> = (0..len.max(1)).map(|_| 0x80 | rng.byte()).collect();
            let p = buf.as_ptr() as *mut c_void;
            unsafe {
                assert_eq!(
                    (s.c.hash_bytes)(p, len, 0),
                    (s.r.hash_bytes)(p, len, 0),
                    "high-bit hash_bytes len={len} buf={buf:02x?}"
                );
            }
        }
    }
    // Extremes: all-0x00, all-0xFF, all-0x80, all-0x7F at every length.
    for pat in [0x00u8, 0xFF, 0x80, 0x7F, 0x01] {
        for len in 0..=40usize {
            let buf = vec![pat; len.max(1)];
            let p = buf.as_ptr() as *mut c_void;
            unsafe {
                assert_eq!(
                    (s.c.hash_bytes)(p, len, 0),
                    (s.r.hash_bytes)(p, len, 0),
                    "pattern {pat:#x} len={len}"
                );
                assert_eq!(
                    (s.c.hash_bytes)(p, len, usize::MAX),
                    (s.r.hash_bytes)(p, len, usize::MAX),
                    "pattern {pat:#x} len={len} seed=MAX"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------- C4
#[test]
fn c4_hash_string_random() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xC4);
    let seeds: Vec<usize> = vec![
        0,
        1,
        usize::MAX,
        0x31415926,
        0x8000_0000_0000_0000,
        0xDEAD_BEEF_CAFE_BABE,
    ];
    for seed in seeds {
        for len in 0..=64usize {
            for _ in 0..4 {
                let body = rng.ascii(len);
                let cs = CStrBuf::new(&body);
                unsafe {
                    assert_eq!(
                        (s.c.hash_string)(cs.ptr(), seed),
                        (s.r.hash_string)(cs.ptr(), seed),
                        "hash_string seed={seed:#x} {:?}",
                        String::from_utf8_lossy(&body)
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- C5
#[test]
fn c5_hash_string_high_bytes() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xC5);
    for len in 0..=48usize {
        for _ in 0..10 {
            let body: Vec<u8> = (0..len)
                .map(|_| {
                    let b = 0x80 | rng.byte();
                    if b == 0 { 0x80 } else { b }
                })
                .collect();
            let cs = CStrBuf::new(&body);
            unsafe {
                assert_eq!(
                    (s.c.hash_string)(cs.ptr(), 0),
                    (s.r.hash_string)(cs.ptr(), 0),
                    "high-byte hash_string len={len}"
                );
            }
        }
    }
    for pat in [0xFFu8, 0x80, 0x7F, 0x01] {
        for len in 0..=48usize {
            let cs = CStrBuf::new(&vec![pat; len]);
            unsafe {
                assert_eq!(
                    (s.c.hash_string)(cs.ptr(), 0x31415926),
                    (s.r.hash_string)(cs.ptr(), 0x31415926),
                    "pattern {pat:#x} len={len}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------- C6 / C56
#[test]
fn c6_rand_seed_and_global_advance() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    for seed in [0usize, 1, 2, usize::MAX, 0x31415926, 0xDEAD_BEEF] {
        s.seed(seed);
        // Create 6 tables in a row: each `make_hash_index(_, NULL)` consumes the
        // global seed and then advances it by `seed*a + b`.
        for round in 0..6 {
            unsafe {
                let cm = (s.c.shmode_func)(elemsize, SH_ARENA);
                let rm = (s.r.shmode_func)(elemsize, SH_ARENA);
                let cs = map_snap(cm, elemsize, KeyKind::StringAt(0));
                let rs = map_snap(rm, elemsize, KeyKind::StringAt(0));
                assert_snap_eq(&format!("C6 seed={seed:#x} round={round}"), &cs, &rs);
                (s.c.hmfree_func)(raw_of(cm, elemsize), elemsize);
                (s.r.hmfree_func)(raw_of(rm, elemsize), elemsize);
            }
        }
    }
}

// ---------------------------------------------------------------------- C7
#[test]
fn c7_arrgrowf_from_null_cross_product() {
    let s = session(1);
    for elemsize in [1usize, 4, 8, 16] {
        for addlen in [0usize, 1, 2, 5, 17] {
            for min_cap in [0usize, 1, 3, 4, 5, 9, 64] {
                unsafe {
                    let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ctx = format!("C7 e={elemsize} add={addlen} cap={min_cap}");
                    assert_eq!(ca.is_null(), ra.is_null(), "{ctx}: null-ness");
                    if ca.is_null() {
                        continue;
                    }
                    let ch = header(ca);
                    let rh = header(ra);
                    assert_eq!(ch.length, rh.length, "{ctx}: length");
                    assert_eq!(ch.capacity, rh.capacity, "{ctx}: capacity");
                    assert_eq!(ch.temp, rh.temp, "{ctx}: temp");
                    assert!(ch.hash_table.is_null() && rh.hash_table.is_null(), "{ctx}: table");
                    (s.c.arrfreef)(ca);
                    (s.r.arrfreef)(ra);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- C8
#[test]
fn c8_arrgrowf_incremental_growth() {
    let s = session(1);
    for elemsize in [1usize, 2, 4, 8, 16, 24] {
        unsafe {
            let mut ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            let mut ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            let mut rng = Rng::new(0xC8 ^ elemsize as u64);
            for step in 0..200usize {
                // Emulate `arrput`: write a payload, bump the length, maybe grow.
                let payload = rng.bytes(elemsize);
                let len = header(ca).length;
                assert_eq!(len, header(ra).length, "C8 e={elemsize} step={step} len");
                if len + 1 > header(ca).capacity {
                    ca = (s.c.arrgrowf)(ca, elemsize, 1, 0);
                    ra = (s.r.arrgrowf)(ra, elemsize, 1, 0);
                }
                std::ptr::copy_nonoverlapping(
                    payload.as_ptr(),
                    (ca as *mut u8).add(len * elemsize),
                    elemsize,
                );
                std::ptr::copy_nonoverlapping(
                    payload.as_ptr(),
                    (ra as *mut u8).add(len * elemsize),
                    elemsize,
                );
                (*((ca as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length = len + 1;
                (*((ra as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length = len + 1;

                let ctx = format!("C8 e={elemsize} step={step}");
                assert_eq!(header(ca).capacity, header(ra).capacity, "{ctx}: capacity");
                assert_eq!(header(ca).length, header(ra).length, "{ctx}: length");
                let n = header(ca).length * elemsize;
                assert_eq!(
                    std::slice::from_raw_parts(ca as *const u8, n),
                    std::slice::from_raw_parts(ra as *const u8, n),
                    "{ctx}: payload"
                );
            }
            (s.c.arrfreef)(ca);
            (s.r.arrfreef)(ra);
        }
    }
}

// ---------------------------------------------------------------------- C9
#[test]
fn c9_arrgrowf_jump_and_noop() {
    let s = session(1);
    let elemsize = 8usize;
    unsafe {
        let mut ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        let mut ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        assert_eq!(header(ca).capacity, 4);
        assert_eq!(header(ra).capacity, 4);

        // no-op: min_cap <= arrcap -> same pointer back
        let ca2 = (s.c.arrgrowf)(ca, elemsize, 0, 4);
        let ra2 = (s.r.arrgrowf)(ra, elemsize, 0, 4);
        assert_eq!(ca2, ca, "C9: C returned a different pointer for the no-op");
        assert_eq!(ra2, ra, "C9: Rust returned a different pointer for the no-op");
        let ca3 = (s.c.arrgrowf)(ca, elemsize, 0, 0);
        let ra3 = (s.r.arrgrowf)(ra, elemsize, 0, 0);
        assert_eq!(ca3, ca);
        assert_eq!(ra3, ra);

        // jump growth well past 2*cap
        for target in [5usize, 9, 10, 100, 101, 1000, 4096, 4097] {
            ca = (s.c.arrgrowf)(ca, elemsize, 0, target);
            ra = (s.r.arrgrowf)(ra, elemsize, 0, target);
            assert_eq!(
                header(ca).capacity,
                header(ra).capacity,
                "C9: capacity after min_cap={target}"
            );
        }
        // addlen driving min_len > min_cap
        for addlen in [1usize, 3, 5000, 100000] {
            ca = (s.c.arrgrowf)(ca, elemsize, addlen, 0);
            ra = (s.r.arrgrowf)(ra, elemsize, addlen, 0);
            assert_eq!(
                header(ca).capacity,
                header(ra).capacity,
                "C9: capacity after addlen={addlen}"
            );
        }
        (s.c.arrfreef)(ca);
        (s.r.arrfreef)(ra);
    }
}

// ---------------------------------------------------------------------- C10
#[test]
fn c10_arrgrowf_elemsizes_roundtrip() {
    let s = session(1);
    let mut rng = Rng::new(0xC10);
    for elemsize in [1usize, 2, 3, 4, 5, 7, 8, 12, 16, 24, 32, 64] {
        unsafe {
            let n = 37usize;
            let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, n, 0);
            let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, n, 0);
            let ctx = format!("C10 e={elemsize}");
            assert_eq!(header(ca).capacity, header(ra).capacity, "{ctx}: capacity");
            let data = rng.bytes(n * elemsize);
            std::ptr::copy_nonoverlapping(data.as_ptr(), ca as *mut u8, data.len());
            std::ptr::copy_nonoverlapping(data.as_ptr(), ra as *mut u8, data.len());
            assert_eq!(
                std::slice::from_raw_parts(ca as *const u8, data.len()),
                std::slice::from_raw_parts(ra as *const u8, data.len()),
                "{ctx}: roundtrip"
            );
            (s.c.arrfreef)(ca);
            (s.r.arrfreef)(ra);
        }
    }
}
