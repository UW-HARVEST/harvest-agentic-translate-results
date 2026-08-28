//! Level 1: leaf functions – hashing, seeding and the array grower.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

fn corpus_bytes() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    // every length from 0..=40 with a deterministic byte pattern, including
    // patterns whose byte 3 / byte 7 have the high bit set (the C code relies
    // on `int` overflow + sign extension there).
    for len in 0..=40usize {
        for pat in 0..4u32 {
            let mut b = Vec::with_capacity(len);
            let mut x: u32 = 0x1234_5678 ^ pat.wrapping_mul(0x9E37_79B9);
            for _ in 0..len {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                let byte = match pat {
                    0 => (x >> 24) as u8,
                    1 => 0xFF,
                    2 => 0x80,
                    _ => (x & 0xFF) as u8,
                };
                b.push(byte);
            }
            v.push(b);
        }
    }
    v
}

fn seeds() -> Vec<usize> {
    vec![
        0,
        1,
        2,
        0x31415926,
        0xFFFF_FFFF,
        usize::MAX,
        usize::MAX / 3,
        0x8000_0000_0000_0000u64 as usize,
        0x0123_4567_89AB_CDEFu64 as usize,
    ]
}

#[test]
fn hash_bytes_matches() {
    let (c, r) = both();
    let corpus = corpus_bytes();
    for seed in seeds() {
        for buf in &corpus {
            let mut cb = buf.clone();
            let mut rb = buf.clone();
            let cv = unsafe { (c.hash_bytes)(cb.as_mut_ptr() as *mut c_void, cb.len(), seed) };
            let rv = unsafe { (r.hash_bytes)(rb.as_mut_ptr() as *mut c_void, rb.len(), seed) };
            assert_eq!(
                cv, rv,
                "stbds_hash_bytes mismatch: seed={seed:#x} len={} buf={:x?}",
                buf.len(),
                buf
            );
        }
    }
}

fn corpus_strings() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"ab".to_vec(),
        b"abc".to_vec(),
        b"test_0".to_vec(),
        b"test_123456".to_vec(),
        b"The quick brown fox jumps over the lazy dog".to_vec(),
    ];
    // strings with high-bit bytes: `(unsigned char) *str++` in C.
    v.push(vec![0x80, 0xFF, 0x7F, 0x01]);
    v.push(vec![0xFFu8; 33]);
    for len in 1..=64usize {
        let mut s = Vec::with_capacity(len);
        let mut x: u32 = 0xDEAD_BEEF ^ len as u32;
        for _ in 0..len {
            x = x.wrapping_mul(1103515245).wrapping_add(12345);
            let mut b = (x >> 16) as u8;
            if b == 0 {
                b = 1;
            }
            s.push(b);
        }
        v.push(s);
    }
    v
}

#[test]
fn hash_string_matches() {
    let (c, r) = both();
    for seed in seeds() {
        for s in corpus_strings() {
            let mut cs = s.clone();
            cs.push(0);
            let mut rs = cs.clone();
            let cv = unsafe { (c.hash_string)(cs.as_mut_ptr() as *mut c_char, seed) };
            let rv = unsafe { (r.hash_string)(rs.as_mut_ptr() as *mut c_char, seed) };
            assert_eq!(cv, rv, "stbds_hash_string mismatch: seed={seed:#x} s={s:x?}");
        }
    }
}

/// `stbds_arrgrowf` decides capacity growth and (re)allocates the header.
/// Compare capacity/length/temp plus payload preservation.
#[test]
fn arrgrowf_matches() {
    let (c, r) = both();
    let elemsizes = [1usize, 2, 4, 8, 16, 24, 128];
    // (addlen, min_cap) call sequences
    let sequences: Vec<Vec<(usize, usize)>> = vec![
        vec![(1, 0)],
        vec![(0, 1)],
        vec![(0, 0)],
        vec![(1, 0), (1, 0), (1, 0), (1, 0), (1, 0), (1, 0), (1, 0), (1, 0)],
        vec![(0, 3), (0, 5), (0, 100), (0, 1)],
        vec![(5, 0), (0, 2), (7, 0), (0, 64), (1, 0)],
        vec![(0, 4), (4, 0), (4, 0), (0, 33), (1, 1), (0, 1000)],
        vec![(3, 7), (9, 2), (1, 1), (0, 0), (2, 40)],
    ];

    for &elemsize in &elemsizes {
            for seq in &sequences {
                let mut ca: *mut u8 = std::ptr::null_mut();
                let mut ra: *mut u8 = std::ptr::null_mut();
                for (step, &(addlen, min_cap)) in seq.iter().enumerate() {
                    unsafe {
                        // write a marker into the existing payload so we can verify
                        // realloc preserved it.
                        for arr in [ca, ra] {
                            if !arr.is_null() {
                                let cap = (*header(arr)).capacity;
                                for k in 0..(cap * elemsize) {
                                    *arr.add(k) = (k as u8).wrapping_add(step as u8);
                                }
                            }
                        }
                        let before_cap = if ca.is_null() {
                            0
                        } else {
                            (*header(ca)).capacity
                        };

                        ca = (c.arrgrowf)(ca as *mut c_void, elemsize, addlen, min_cap) as *mut u8;
                        ra = (r.arrgrowf)(ra as *mut c_void, elemsize, addlen, min_cap) as *mut u8;

                        assert_eq!(
                            ca.is_null(),
                            ra.is_null(),
                            "arrgrowf nullness mismatch elemsize={elemsize} step={step} \
                             addlen={addlen} min_cap={min_cap}"
                        );
                        if ca.is_null() {
                            continue;
                        }

                        let ch = *header(ca);
                        let rh = *header(ra);
                        assert_eq!(
                            (ch.length, ch.capacity, ch.temp, ch.hash_table.is_null()),
                            (rh.length, rh.capacity, rh.temp, rh.hash_table.is_null()),
                            "arrgrowf header mismatch elemsize={elemsize} step={step} \
                             addlen={addlen} min_cap={min_cap}"
                        );
                        // payload comparison over the previously-initialised region
                        let keep = before_cap.min(ch.capacity) * elemsize;
                        let cs = std::slice::from_raw_parts(ca, keep);
                        let rs = std::slice::from_raw_parts(ra, keep);
                        assert_eq!(
                            cs, rs,
                            "arrgrowf payload mismatch elemsize={elemsize} step={step}"
                        );

                        // simulate a length increase like arrput would
                        (*header(ca)).length = ch.capacity.min(ch.length + addlen);
                        (*header(ra)).length = rh.capacity.min(rh.length + addlen);
                    }
                }
                unsafe {
                    if !ca.is_null() {
                        (c.arrfreef)(ca as *mut c_void);
                    }
                    if !ra.is_null() {
                        (r.arrfreef)(ra as *mut c_void);
                    }
                }
            }
    }
}

/// `stbds_rand_seed` sets the global seed consumed by the next fresh hash
/// index; observe it through the `seed` field of a newly created table.
#[test]
fn rand_seed_and_seed_advance_match() {
    let _guard = serial();
    let (c, r) = both();
    let elemsize = 16usize;
    for &s in &[0usize, 1, 0x31415926, usize::MAX, 0xDEAD_BEEF_CAFE_F00Du64 as usize] {
        unsafe {
            (c.rand_seed)(s);
            (r.rand_seed)(s);
            // create several independent tables: each consumes and advances the
            // global seed via the `stbds_hash_seed * a + b` LCG.
            let mut cts: Vec<*mut u8> = Vec::new();
            let mut rts: Vec<*mut u8> = Vec::new();
            for i in 0..6i64 {
                let mut ct: *mut u8 = std::ptr::null_mut();
                let mut rt: *mut u8 = std::ptr::null_mut();
                ct = hmput_bytes(&c, ct, elemsize, &i.to_le_bytes(), &(i * 3).to_le_bytes());
                rt = hmput_bytes(&r, rt, elemsize, &i.to_le_bytes(), &(i * 3).to_le_bytes());
                assert_eq!(
                    snapshot(ct, elemsize, false),
                    snapshot(rt, elemsize, false),
                    "seed advance mismatch: base={s:#x} table={i}"
                );
                cts.push(ct);
                rts.push(rt);
            }
            for (ct, rt) in cts.into_iter().zip(rts) {
                (c.hmfree_func)(ct.sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)(rt.sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}
