//! Phase B — valid-path differential tests, CONFIGS.md rows 1..8
//! (`stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed`,
//!  `stbds_arrgrowf`, `stbds_arrfreef`).
mod common;

use common::*;
use std::ffi::{c_char, c_void};

const SEEDS: [usize; 5] = [0, 1, 0x3141_5926, usize::MAX, 0xdead_beef_cafe_0123];

// ---------------------------------------------------------------------------
// row 1 — hash_bytes over len 0..=64 x random bytes x seed sweep
// ---------------------------------------------------------------------------
#[test]
fn cfg_01_hash_bytes_len_sweep() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0001);
    let mut n = 0usize;
    for len in 0..=64usize {
        for _ in 0..40 {
            let buf = rng.bytes(len);
            for &seed in SEEDS.iter() {
                unsafe {
                    let ch = (l.c.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                    let rh = (l.r.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                    assert_eq!(
                        ch, rh,
                        "hash_bytes(len={len}, seed={seed:#x}, buf={buf:02x?}) C={ch:#x} RUST={rh:#x}"
                    );
                }
                n += 1;
            }
            // random seed too
            let seed = rng.next_u64() as usize;
            unsafe {
                let ch = (l.c.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                let rh = (l.r.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                assert_eq!(ch, rh, "hash_bytes(len={len}, seed={seed:#x})");
            }
            n += 1;
        }
    }
    assert!(n > 15_000, "expected many samples, got {n}");
}

// ---------------------------------------------------------------------------
// row 2 — tail `switch` fall-through cases 1..7 with the high bit set in the
//         lanes that make the C `int` arithmetic go negative (d[3], d[6], d[7])
// ---------------------------------------------------------------------------
#[test]
fn cfg_02_hash_bytes_tail_sign_extension() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0002);
    for total_full in [0usize, 1, 2, 3] {
        for rem in 1..=7usize {
            let len = total_full * 8 + rem;
            for pattern in 0..256u32 {
                let mut buf = rng.bytes(len);
                // force the "interesting" lanes
                for (i, b) in buf.iter_mut().enumerate() {
                    let bit = (pattern >> (i % 8)) & 1;
                    if bit == 1 {
                        *b |= 0x80;
                    } else {
                        *b &= 0x7f;
                    }
                }
                for &seed in SEEDS.iter() {
                    unsafe {
                        let ch = (l.c.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                        let rh = (l.r.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                        assert_eq!(
                            ch, rh,
                            "tail sign-ext: len={len} rem={rem} seed={seed:#x} buf={buf:02x?}"
                        );
                    }
                }
            }
        }
    }
    // explicit worst cases: every tail byte 0xFF / 0x80 / 0x7F
    for fill in [0xffu8, 0x80, 0x7f, 0x00] {
        for len in 0..=23usize {
            let buf = vec![fill; len];
            for &seed in SEEDS.iter() {
                unsafe {
                    let ch = (l.c.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                    let rh = (l.r.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                    assert_eq!(ch, rh, "fill={fill:#x} len={len} seed={seed:#x}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 3 — main loop, exact multiples of 8, high bit set in lanes 3 and 7
// ---------------------------------------------------------------------------
#[test]
fn cfg_03_hash_bytes_main_loop_sign_extension() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0003);
    for blocks in 1..=32usize {
        let len = blocks * 8;
        for variant in 0..24 {
            let mut buf = rng.bytes(len);
            match variant % 6 {
                0 => {}
                1 => buf.iter_mut().for_each(|b| *b |= 0x80),
                2 => buf.iter_mut().for_each(|b| *b &= 0x7f),
                3 => buf
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, b)| *b = if i % 8 == 3 { 0xff } else { 0x01 }),
                4 => buf
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, b)| *b = if i % 8 == 7 { 0xff } else { 0x01 }),
                _ => buf.iter_mut().for_each(|b| *b = 0),
            }
            for &seed in SEEDS.iter() {
                unsafe {
                    let ch = (l.c.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                    let rh = (l.r.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed);
                    assert_eq!(ch, rh, "main-loop: len={len} variant={variant} seed={seed:#x}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 4 — hash_string over many shapes
// ---------------------------------------------------------------------------
#[test]
fn cfg_04_hash_string_sweep() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0004);
    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"ab".to_vec(),
        b"test_0".to_vec(),
        b"test_1".to_vec(),
        b"12345678".to_vec(),
        vec![0xffu8; 1],
        vec![0xffu8; 7],
        vec![0x80u8; 33],
        vec![0x01u8; 200],
        "héllo wörld ✓".as_bytes().to_vec(),
    ];
    for len in 0..=80usize {
        cases.push(rng.cstr_bytes(len, false));
        cases.push(rng.cstr_bytes(len, true));
    }
    for c in &cases {
        let mut owned = c.clone();
        owned.push(0);
        for &seed in SEEDS.iter() {
            unsafe {
                let ch = (l.c.hash_string)(owned.as_ptr() as *mut c_char, seed);
                let rh = (l.r.hash_string)(owned.as_ptr() as *mut c_char, seed);
                assert_eq!(ch, rh, "hash_string({c:02x?}, {seed:#x})");
            }
        }
        for _ in 0..4 {
            let seed = rng.next_u64() as usize;
            unsafe {
                let ch = (l.c.hash_string)(owned.as_ptr() as *mut c_char, seed);
                let rh = (l.r.hash_string)(owned.as_ptr() as *mut c_char, seed);
                assert_eq!(ch, rh, "hash_string({c:02x?}, {seed:#x})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 5 — the *global* seed: `stbds_rand_seed` then observe the seed that a
//         freshly created table picks up, and how the global advances.
// ---------------------------------------------------------------------------
#[test]
fn cfg_05_global_seed_advance() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0005);
    let mut seeds: Vec<usize> = vec![0, 1, 2, 0x3141_5926, usize::MAX, usize::MAX - 1];
    for _ in 0..24 {
        seeds.push(rng.next_u64() as usize);
    }
    for s in seeds {
        unsafe {
            (l.c.rand_seed)(s);
            (l.r.rand_seed)(s);
        }
        // Ten successive fresh tables => ten successive global-seed values.
        let mut ct: Vec<usize> = Vec::new();
        let mut rt: Vec<usize> = Vec::new();
        for _ in 0..10 {
            unsafe {
                let c = (l.c.shmode_func)(16, SH_DEFAULT) as *mut u8;
                let r = (l.r.shmode_func)(16, SH_DEFAULT) as *mut u8;
                let ch = (*header_of(c, 16)).hash_table as *const HashIndex;
                let rh = (*header_of(r, 16)).hash_table as *const HashIndex;
                ct.push((*ch).seed);
                rt.push((*rh).seed);
                (l.c.hmfree_func)(c.sub(16) as *mut c_void, 16);
                (l.r.hmfree_func)(r.sub(16) as *mut c_void, 16);
            }
        }
        assert_eq!(ct, rt, "global seed chain mismatch starting from {s:#x}");
        assert_eq!(ct[0], s, "first table must take the seed verbatim");
    }
}

// ---------------------------------------------------------------------------
// arrgrowf helpers
// ---------------------------------------------------------------------------

unsafe fn arr_header(a: *mut u8) -> *mut ArrHeader {
    unsafe { a.sub(HEADER_SIZE) as *mut ArrHeader }
}

unsafe fn arr_snapshot(a: *mut u8, elemsize: usize, len_bytes: usize) -> String {
    unsafe {
        if a.is_null() {
            return "NULL".into();
        }
        let h = &*arr_header(a);
        let body = std::slice::from_raw_parts(a, len_bytes.min(elemsize * h.length));
        format!(
            "len={} cap={} temp={} table={} body={:02x?}",
            h.length,
            h.capacity,
            h.temp,
            if h.hash_table.is_null() { "no" } else { "yes" },
            body
        )
    }
}

/// `stbds_arrgrow(a, addlen, min_cap)`
unsafe fn arrgrow(f: FnArrGrowf, a: *mut u8, elemsize: usize, addlen: usize, min_cap: usize) -> *mut u8 {
    unsafe { f(a as *mut c_void, elemsize, addlen, min_cap) as *mut u8 }
}

/// `stbds_arrmaybegrow(a, n)` + `length += n`, i.e. the guts of `arrput`.
unsafe fn arr_push(f: FnArrGrowf, a: *mut u8, elemsize: usize, val: &[u8]) -> *mut u8 {
    unsafe {
        let mut a = a;
        let need = if a.is_null() {
            true
        } else {
            (*arr_header(a)).length + 1 > (*arr_header(a)).capacity
        };
        if need {
            a = arrgrow(f, a, elemsize, 1, 0);
        }
        let idx = (*arr_header(a)).length;
        std::ptr::copy_nonoverlapping(val.as_ptr(), a.add(elemsize * idx), elemsize.min(val.len()));
        (*arr_header(a)).length = idx + 1;
        a
    }
}

// ---------------------------------------------------------------------------
// row 6 — arrgrowf from NULL: fresh-alloc / cap-floor-4 / no-op
// ---------------------------------------------------------------------------
#[test]
fn cfg_06_arrgrowf_from_null() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0006);
    for _ in 0..3000 {
        let elemsize = 1 + rng.below(64);
        let addlen = rng.below(9);
        let min_cap = rng.below(41);
        unsafe {
            let ca = arrgrow(l.c.arrgrowf, std::ptr::null_mut(), elemsize, addlen, min_cap);
            let ra = arrgrow(l.r.arrgrowf, std::ptr::null_mut(), elemsize, addlen, min_cap);
            assert_eq!(
                ca.is_null(),
                ra.is_null(),
                "arrgrowf(NULL,{elemsize},{addlen},{min_cap}) NULL-ness mismatch"
            );
            if ca.is_null() {
                // min_cap <= arrcap(NULL) == 0  =>  returns NULL, no allocation
                assert_eq!(addlen, 0);
                assert_eq!(min_cap, 0);
                continue;
            }
            let cs = arr_snapshot(ca, elemsize, 0);
            let rs = arr_snapshot(ra, elemsize, 0);
            assert_eq!(
                cs, rs,
                "arrgrowf(NULL,{elemsize},{addlen},{min_cap})\nC   : {cs}\nRUST: {rs}"
            );
            (l.c.arrfreef)(ca as *mut c_void);
            (l.r.arrfreef)(ra as *mut c_void);
        }
    }
    // the exact boundary values
    for &(es, add, cap) in &[
        (8usize, 0usize, 0usize),
        (8, 0, 1),
        (8, 1, 0),
        (8, 3, 0),
        (8, 4, 0),
        (8, 5, 0),
        (8, 0, 3),
        (8, 0, 4),
        (8, 0, 5),
        (1, 0, 0),
        (1, 1, 1),
        (64, 8, 8),
    ] {
        unsafe {
            let ca = arrgrow(l.c.arrgrowf, std::ptr::null_mut(), es, add, cap);
            let ra = arrgrow(l.r.arrgrowf, std::ptr::null_mut(), es, add, cap);
            assert_eq!(ca.is_null(), ra.is_null(), "({es},{add},{cap})");
            if !ca.is_null() {
                assert_eq!(
                    arr_snapshot(ca, es, 0),
                    arr_snapshot(ra, es, 0),
                    "({es},{add},{cap})"
                );
                (l.c.arrfreef)(ca as *mut c_void);
                (l.r.arrfreef)(ra as *mut c_void);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 7 — repeated growth of an existing array: doubling vs explicit min_cap
// ---------------------------------------------------------------------------
#[test]
fn cfg_07_arrgrowf_repeated() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0007);
    for trial in 0..200 {
        let elemsize = 1 + rng.below(32);
        unsafe {
            let mut ca: *mut u8 = std::ptr::null_mut();
            let mut ra: *mut u8 = std::ptr::null_mut();
            for step in 0..30 {
                let addlen = rng.below(6);
                let min_cap = if step % 3 == 0 { rng.below(100) } else { 0 };
                ca = arrgrow(l.c.arrgrowf, ca, elemsize, addlen, min_cap);
                ra = arrgrow(l.r.arrgrowf, ra, elemsize, addlen, min_cap);
                assert_eq!(ca.is_null(), ra.is_null());
                if ca.is_null() {
                    continue;
                }
                // grow the logical length like arraddn does
                let n = addlen.min((*arr_header(ca)).capacity - (*arr_header(ca)).length);
                (*arr_header(ca)).length += n;
                (*arr_header(ra)).length += n;
                // fill the new bytes deterministically so `body` is comparable
                let len = (*arr_header(ca)).length;
                for i in 0..len {
                    let b = ((trial * 31 + step * 7 + i) & 0xff) as u8;
                    std::ptr::write_bytes(ca.add(elemsize * i), b, elemsize);
                    std::ptr::write_bytes(ra.add(elemsize * i), b, elemsize);
                }
                let cs = arr_snapshot(ca, elemsize, usize::MAX);
                let rs = arr_snapshot(ra, elemsize, usize::MAX);
                assert_eq!(
                    cs, rs,
                    "trial={trial} step={step} elemsize={elemsize} addlen={addlen} min_cap={min_cap}\nC   : {cs}\nRUST: {rs}"
                );
            }
            if !ca.is_null() {
                (l.c.arrfreef)(ca as *mut c_void);
                (l.r.arrfreef)(ra as *mut c_void);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 8 — arrput emulation for 0,1,2,3,4,5,8,17,100,1000 elements
// ---------------------------------------------------------------------------
#[test]
fn cfg_08_arrput_sequences() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0008);
    for &elemsize in &[1usize, 2, 4, 8, 16] {
        for &count in &[0usize, 1, 2, 3, 4, 5, 8, 17, 100, 1000] {
            unsafe {
                let mut ca: *mut u8 = std::ptr::null_mut();
                let mut ra: *mut u8 = std::ptr::null_mut();
                for i in 0..count {
                    let v = rng.bytes(elemsize);
                    ca = arr_push(l.c.arrgrowf, ca, elemsize, &v);
                    ra = arr_push(l.r.arrgrowf, ra, elemsize, &v);
                    let cs = arr_snapshot(ca, elemsize, usize::MAX);
                    let rs = arr_snapshot(ra, elemsize, usize::MAX);
                    assert_eq!(
                        cs, rs,
                        "arrput elemsize={elemsize} i={i}\nC   : {cs}\nRUST: {rs}"
                    );
                }
                // arrpop: length--, read last
                let mut popped_c = Vec::new();
                let mut popped_r = Vec::new();
                while !ca.is_null() && (*arr_header(ca)).length > 0 {
                    let cl = (*arr_header(ca)).length - 1;
                    (*arr_header(ca)).length = cl;
                    (*arr_header(ra)).length = cl;
                    popped_c
                        .push(std::slice::from_raw_parts(ca.add(elemsize * cl), elemsize).to_vec());
                    popped_r
                        .push(std::slice::from_raw_parts(ra.add(elemsize * cl), elemsize).to_vec());
                }
                assert_eq!(popped_c, popped_r, "arrpop elemsize={elemsize} count={count}");
                if !ca.is_null() {
                    (l.c.arrfreef)(ca as *mut c_void);
                    (l.r.arrfreef)(ra as *mut c_void);
                }
            }
        }
    }
}
