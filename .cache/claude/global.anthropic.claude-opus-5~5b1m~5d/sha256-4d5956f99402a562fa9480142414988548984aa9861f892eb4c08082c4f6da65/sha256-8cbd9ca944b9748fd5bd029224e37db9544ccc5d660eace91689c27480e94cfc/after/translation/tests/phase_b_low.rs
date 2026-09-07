//! Phase B — differential tests for the lowest-level exported entry points:
//! hashing, dynamic-array growth, the string arena, and `strkey`.
//!
//! CONFIGS.md rows 1-7, 24-26, 28.

mod common;

use common::*;
use std::ffi::{c_char, c_void, CStr, CString};

// ===========================================================================
// Row 1 / 2 — stbds_hash_bytes across every length & tail case, many seeds
// ===========================================================================

#[test]
fn cfg_01_hash_bytes_all_lengths() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0xB0BA_CAFE);
    let seeds: [usize; 6] = [0, 1, 0x3141_5926, usize::MAX, 0x8000_0000_0000_0000, 0xDEAD_BEEF];
    for &seed in &seeds {
        for len in 0..=64usize {
            for _ in 0..8 {
                let mut buf = rng.bytes(len);
                // guarantee the sign-extension path is hit sometimes
                if len >= 4 && (len % 3 == 0) {
                    let l = buf.len();
                    buf[l - 1] |= 0x80;
                }
                let ptr = buf.as_mut_ptr() as *mut c_void;
                let hc = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
                let hr = unsafe { (p.r.hash_bytes)(ptr, len, seed) };
                assert_eq!(
                    hc, hr,
                    "hash_bytes(len={len}, seed={seed:#x}, bytes={buf:02x?}) C={hc:#x} R={hr:#x}"
                );
            }
        }
    }
}

#[test]
fn cfg_02_hash_bytes_random() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for _ in 0..512 {
        let len = rng.below(257);
        let seed = rng.next_u64() as usize;
        let mut buf = rng.bytes(len);
        let ptr = buf.as_mut_ptr() as *mut c_void;
        let hc = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
        let hr = unsafe { (p.r.hash_bytes)(ptr, len, seed) };
        assert_eq!(hc, hr, "hash_bytes(len={len}, seed={seed:#x})");
    }
}

// ===========================================================================
// Row 3 — stbds_hash_string
// ===========================================================================

#[test]
fn cfg_03_hash_string_random() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0xFEED_FACE);
    let seeds: [usize; 5] = [0, 1, 0x3141_5926, usize::MAX, 0xA5A5_A5A5_A5A5_A5A5];
    for &seed in &seeds {
        for len in 0..=64usize {
            for _ in 0..4 {
                let s = rng.cstring(len);
                let hc = unsafe { (p.c.hash_string)(s.as_ptr() as *mut c_char, seed) };
                let hr = unsafe { (p.r.hash_string)(s.as_ptr() as *mut c_char, seed) };
                assert_eq!(hc, hr, "hash_string(len={len}, seed={seed:#x}, {:02x?})", s.as_bytes());
            }
        }
    }
    // pure-ASCII and all-high-bit corner strings
    for s in ["", "a", "ab", "test_0", "test_2147483647", "\u{7f}", "aaaaaaaaaaaaaaaaaaaaaaaa"] {
        let cs = CString::new(s).unwrap();
        for &seed in &seeds {
            let hc = unsafe { (p.c.hash_string)(cs.as_ptr() as *mut c_char, seed) };
            let hr = unsafe { (p.r.hash_string)(cs.as_ptr() as *mut c_char, seed) };
            assert_eq!(hc, hr, "hash_string({s:?}, {seed:#x})");
        }
    }
    for b in 0x80u8..=0xFF {
        let cs = CString::new(vec![b, b, b]).unwrap();
        let hc = unsafe { (p.c.hash_string)(cs.as_ptr() as *mut c_char, 7) };
        let hr = unsafe { (p.r.hash_string)(cs.as_ptr() as *mut c_char, 7) };
        assert_eq!(hc, hr, "hash_string(high byte {b:#x})");
    }
}

// ===========================================================================
// Row 4 — stbds_rand_seed and the global seed advance
// ===========================================================================

#[test]
fn cfg_04_rand_seed_advance() {
    let (_g, p) = libs();
    for &s in &[0usize, 1, 7, 0x3141_5926, usize::MAX, 0xDEAD_BEEF_CAFE_F00D] {
        unsafe {
            (p.c.rand_seed)(s);
            (p.r.rand_seed)(s);
            // Three successive fresh tables: seeds must advance identically.
            let mut cs = Vec::new();
            let mut rs = Vec::new();
            let mut ct = Vec::new();
            let mut rt = Vec::new();
            for _ in 0..3 {
                let tc = (p.c.shmode_func)(16, STBDS_SH_STRDUP);
                let tr = (p.r.shmode_func)(16, STBDS_SH_STRDUP);
                cs.push(snap(tc, 16, KeyKind::StrPtr { off: 0, cmp_end: 12 }).table.unwrap().seed);
                rs.push(snap(tr, 16, KeyKind::StrPtr { off: 0, cmp_end: 12 }).table.unwrap().seed);
                ct.push(tc);
                rt.push(tr);
            }
            assert_eq!(cs, rs, "table seed sequence for rand_seed({s:#x})");
            for (tc, tr) in ct.iter().zip(rt.iter()) {
                (p.c.hmfree_func)((*tc as *mut u8).sub(16) as *mut c_void, 16);
                (p.r.hmfree_func)((*tr as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}

// ===========================================================================
// Rows 5-7 — stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

#[test]
fn cfg_05_arrgrowf_fresh() {
    let (_g, p) = libs();
    for &elemsize in &[1usize, 4, 8, 12, 16, 24] {
        for &addlen in &[0usize, 1, 2, 7, 100] {
            for &min_cap in &[0usize, 1, 3, 4, 5, 64] {
                unsafe {
                    let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    assert_eq!(
                        ac.is_null(),
                        ar.is_null(),
                        "NULL-ness must agree for arrgrowf(NULL, es={elemsize}, addlen={addlen}, min_cap={min_cap})"
                    );
                    if ac.is_null() {
                        // `min_cap <= stbds_arrcap(NULL) == 0` short-circuits
                        // and returns the NULL input unchanged.
                        assert_eq!(addlen, 0);
                        assert_eq!(min_cap, 0);
                        continue;
                    }
                    let sc = arr_snap(ac, 0);
                    let sr = arr_snap(ar, 0);
                    assert_eq!(
                        sc, sr,
                        "arrgrowf(NULL, es={elemsize}, addlen={addlen}, min_cap={min_cap})"
                    );
                    (p.c.arrfreef)(ac);
                    (p.r.arrfreef)(ar);
                }
            }
        }
    }
}

#[test]
fn cfg_06_arrgrowf_regrow() {
    let (_g, p) = libs();
    for &elemsize in &[1usize, 4, 8, 16] {
        unsafe {
            let mut ac: *mut c_void = std::ptr::null_mut();
            let mut ar: *mut c_void = std::ptr::null_mut();
            for step in 0..200usize {
                // emulate stbds_arrput's arrmaybegrow(a,1)
                let bump = |a: &mut *mut c_void, f: FnArrGrowf| {
                    let need = if a.is_null() {
                        true
                    } else {
                        let h = (*a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
                        (*h).length + 1 > (*h).capacity
                    };
                    if need {
                        *a = f(*a, elemsize, 1, 0);
                    }
                    let h = (*a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
                    (*h).length += 1;
                };
                bump(&mut ac, p.c.arrgrowf);
                bump(&mut ar, p.r.arrgrowf);
                assert_eq!(
                    arr_snap(ac, 0),
                    arr_snap(ar, 0),
                    "arrgrowf regrow es={elemsize} step={step}"
                );
            }
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

#[test]
fn cfg_07_arrgrowf_payload_arrfree() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0x0BAD_F00D);
    let elemsize = 8usize;
    unsafe {
        let mut ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        let mut ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        let mut payload: Vec<u8> = Vec::new();
        for n in 0..300usize {
            let chunk = rng.bytes(elemsize);
            // grow if needed
            let cap = |a: *mut c_void| {
                (*((a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).capacity
            };
            if n + 1 > cap(ac) {
                ac = (p.c.arrgrowf)(ac, elemsize, 1, 0);
            }
            if n + 1 > cap(ar) {
                ar = (p.r.arrgrowf)(ar, elemsize, 1, 0);
            }
            std::ptr::copy_nonoverlapping(
                chunk.as_ptr(),
                (ac as *mut u8).add(n * elemsize),
                elemsize,
            );
            std::ptr::copy_nonoverlapping(
                chunk.as_ptr(),
                (ar as *mut u8).add(n * elemsize),
                elemsize,
            );
            payload.extend_from_slice(&chunk);
            (*((ac as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length = n + 1;
            (*((ar as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length = n + 1;

            let sc = arr_snap(ac, (n + 1) * elemsize);
            let sr = arr_snap(ar, (n + 1) * elemsize);
            assert_eq!(sc, sr, "payload preserved through growth at n={n}");
            assert_eq!(sc.payload, payload, "C payload wrong at n={n}");
        }
        (p.c.arrfreef)(ac);
        (p.r.arrfreef)(ar);
    }
}

// ===========================================================================
// Rows 24-26 — stbds_stralloc / stbds_strreset
// ===========================================================================

/// Allocate `s` in both arenas and assert the copies and arena state match.
unsafe fn stralloc_both(
    p: &Pair,
    sac: &mut StringArena,
    sar: &mut StringArena,
    s: &CStr,
) {
    let pc = (p.c.stralloc)(sac, s.as_ptr() as *mut c_char);
    let pr = (p.r.stralloc)(sar, s.as_ptr() as *mut c_char);
    assert!(!pc.is_null() && !pr.is_null());
    assert_eq!(CStr::from_ptr(pc).to_bytes(), s.to_bytes(), "C stralloc content");
    assert_eq!(CStr::from_ptr(pr).to_bytes(), s.to_bytes(), "Rust stralloc content");
    assert_eq!(
        arena_snap(sac),
        arena_snap(sar),
        "arena state after stralloc(len={})",
        s.to_bytes().len()
    );
}

#[test]
fn cfg_24_stralloc_mixed_lengths() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0xA11C_E5EE);
    for round in 0..8 {
        let mut sac = StringArena::zeroed();
        let mut sar = StringArena::zeroed();
        let mut kept: Vec<(*mut c_char, *mut c_char, Vec<u8>)> = Vec::new();
        for i in 0..400usize {
            let len = match i % 7 {
                0 => rng.below(16),
                1 => rng.below(64),
                2 => 511,
                3 => 512,
                4 => 513,
                5 => rng.below(600),
                _ => rng.below(2000),
            };
            let s = rng.cstring(len);
            unsafe {
                let pc = (p.c.stralloc)(&mut sac, s.as_ptr() as *mut c_char);
                let pr = (p.r.stralloc)(&mut sar, s.as_ptr() as *mut c_char);
                assert_eq!(CStr::from_ptr(pc).to_bytes(), s.as_bytes());
                assert_eq!(CStr::from_ptr(pr).to_bytes(), s.as_bytes());
                assert_eq!(
                    arena_snap(&sac),
                    arena_snap(&sar),
                    "round {round} i={i} len={len}"
                );
                kept.push((pc, pr, s.as_bytes().to_vec()));
            }
        }
        // every earlier allocation must still be intact in both arenas
        unsafe {
            for (pc, pr, want) in &kept {
                assert_eq!(CStr::from_ptr(*pc).to_bytes(), &want[..], "C arena clobbered");
                assert_eq!(CStr::from_ptr(*pr).to_bytes(), &want[..], "Rust arena clobbered");
            }
            (p.c.strreset)(&mut sac);
            (p.r.strreset)(&mut sar);
            assert_eq!(arena_snap(&sac), arena_snap(&sar), "after strreset");
        }
    }
}

#[test]
fn cfg_25_stralloc_block_growth() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0x5EED_1234);
    let mut sac = StringArena::zeroed();
    let mut sar = StringArena::zeroed();
    for i in 0..5000usize {
        let n = 1 + rng.below(40);
        let s = rng.cstring(n);
        unsafe {
            stralloc_both(p, &mut sac, &mut sar, &s);
        }
        if i % 500 == 0 {
            unsafe {
                assert_eq!(arena_snap(&sac), arena_snap(&sar), "block growth at i={i}");
            }
        }
    }
    unsafe {
        let a = arena_snap(&sac);
        assert!(a.block >= 10, "expected the block index to climb, got {}", a.block);
        (p.c.strreset)(&mut sac);
        (p.r.strreset)(&mut sar);
        assert_eq!(arena_snap(&sac), arena_snap(&sar));
    }
}

#[test]
fn cfg_26_stralloc_strreset_cycle() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0xC0FF_EE01);
    let mut sac = StringArena::zeroed();
    let mut sar = StringArena::zeroed();
    for cycle in 0..6 {
        for _ in 0..300 {
            let n = rng.below(700);
            let s = rng.cstring(n);
            unsafe {
                stralloc_both(p, &mut sac, &mut sar, &s);
            }
        }
        unsafe {
            (p.c.strreset)(&mut sac);
            (p.r.strreset)(&mut sar);
            let a = arena_snap(&sac);
            assert_eq!(a, arena_snap(&sar), "cycle {cycle}");
            assert_eq!(
                a,
                ArenaSnap {
                    remaining: 0,
                    block: 0,
                    mode: 0,
                    has_storage: false,
                    block_chain_len: 0
                },
                "strreset must zero the arena"
            );
        }
    }
}

// ===========================================================================
// Row 28 — strkey
// ===========================================================================

#[test]
fn cfg_28_strkey_values() {
    let (_g, p) = libs();
    let mut vals: Vec<i32> =
        vec![0, 1, 9, 10, 99, 100, 12345, -1, -12345, i32::MAX, i32::MIN, -100, 1000000];
    let mut rng = Rng::new(0x7777);
    for _ in 0..200 {
        vals.push(rng.next_u32() as i32);
    }
    for n in vals {
        unsafe {
            let pc = (p.c.strkey)(n);
            let pr = (p.r.strkey)(n);
            assert_eq!(
                CStr::from_ptr(pc).to_bytes(),
                CStr::from_ptr(pr).to_bytes(),
                "strkey({n})"
            );
            assert_eq!(
                CStr::from_ptr(pc).to_str().unwrap(),
                format!("test_{n}"),
                "strkey({n}) content"
            );
        }
    }
}
