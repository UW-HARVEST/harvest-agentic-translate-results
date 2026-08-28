//! Heavy randomised stress that reaches the rarely taken branches of
//! `stbds_hmput_key` / `stbds_hm_find_slot` — in particular the second,
//! wrapped-around probe loop.
//!
//! Per-operation the cheap observables are compared (returned index, header
//! `length` / `capacity` / `temp`, and the looked-up value); the full bucket
//! array and element list are compared periodically so the test stays fast.
//!
//! `table->temp_key` is deliberately *not* compared here: neither
//! `stbds_make_hash_index` nor `stbds_hmdel_key` ever initialises it, so once a
//! delete has rebuilt or shrunk the table it holds uninitialised heap bytes in
//! both libraries. It is compared instead in `level2_hashmap.rs` on delete-free
//! tables, where every operation leaves it well defined.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

fn hdr3(t: *mut StrMapEntry) -> (usize, usize, isize) {
    unsafe {
        if t.is_null() {
            return (0, 0, 0);
        }
        let h = header(t.sub(1) as *mut c_void);
        (h.length, h.capacity, h.temp)
    }
}

fn stress(mode: Option<c_int>, seed: usize, ops: usize, universe: usize, snap_every: usize) {
    let (c, r) = apis();
    reset_seeds(&c, &r, DEFAULT_SEED);

    let mut kc: Vec<CStr8> = (0..universe)
        .map(|i| CStr8::new(&format!("test_{}", i)))
        .collect();
    let mut kr: Vec<CStr8> = (0..universe)
        .map(|i| CStr8::new(&format!("test_{}", i)))
        .collect();

    let mut tc: *mut StrMapEntry = std::ptr::null_mut();
    let mut tr: *mut StrMapEntry = std::ptr::null_mut();

    unsafe {
        if let Some(m) = mode {
            tc = (c.shmode_func)(ELEMSIZE, m) as *mut StrMapEntry;
            tr = (r.shmode_func)(ELEMSIZE, m) as *mut StrMapEntry;
        }
        tc = (c.hmput_default)(tc as *mut c_void, ELEMSIZE) as *mut StrMapEntry;
        tr = (r.hmput_default)(tr as *mut c_void, ELEMSIZE) as *mut StrMapEntry;
        (*tc.offset(-1)).value = -2;
        (*tr.offset(-1)).value = -2;
    }

    let mut state = (seed as u64) | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for step in 0..ops {
        let x = next();
        let i = (x as usize >> 8) % universe;
        let op = x % 3;
        unsafe {
            match op {
                0 => {
                    tc = (c.hmput_key)(
                        tc as *mut c_void,
                        ELEMSIZE,
                        kc[i].as_ptr() as *mut c_void,
                        KEYSIZE,
                        STBDS_HM_STRING,
                    ) as *mut StrMapEntry;
                    tr = (r.hmput_key)(
                        tr as *mut c_void,
                        ELEMSIZE,
                        kr[i].as_ptr() as *mut c_void,
                        KEYSIZE,
                        STBDS_HM_STRING,
                    ) as *mut StrMapEntry;
                    let ic = header(tc.sub(1) as *mut c_void).temp;
                    let ir = header(tr.sub(1) as *mut c_void).temp;
                    assert_eq!(ic, ir, "step {} put {} index", step, i);
                    // the slot the put selected must already carry the key
                    assert_eq!(
                        cstr((*tc.offset(ic)).key),
                        cstr((*tr.offset(ir)).key),
                        "step {} put {} stored key",
                        step,
                        i
                    );
                    (*tc.offset(ic)).value = i as c_int;
                    (*tr.offset(ir)).value = i as c_int;
                }
                1 => {
                    tc = (c.hmget_key)(
                        tc as *mut c_void,
                        ELEMSIZE,
                        kc[i].as_ptr() as *mut c_void,
                        KEYSIZE,
                        STBDS_HM_STRING,
                    ) as *mut StrMapEntry;
                    tr = (r.hmget_key)(
                        tr as *mut c_void,
                        ELEMSIZE,
                        kr[i].as_ptr() as *mut c_void,
                        KEYSIZE,
                        STBDS_HM_STRING,
                    ) as *mut StrMapEntry;
                    let ic = header(tc.sub(1) as *mut c_void).temp;
                    let ir = header(tr.sub(1) as *mut c_void).temp;
                    assert_eq!(ic, ir, "step {} get {} index", step, i);
                    assert_eq!(
                        (*tc.offset(ic)).value,
                        (*tr.offset(ir)).value,
                        "step {} get {} value",
                        step,
                        i
                    );
                }
                _ => {
                    tc = (c.hmdel_key)(
                        tc as *mut c_void,
                        ELEMSIZE,
                        kc[i].as_ptr() as *mut c_void,
                        KEYSIZE,
                        0,
                        STBDS_HM_STRING,
                    ) as *mut StrMapEntry;
                    tr = (r.hmdel_key)(
                        tr as *mut c_void,
                        ELEMSIZE,
                        kr[i].as_ptr() as *mut c_void,
                        KEYSIZE,
                        0,
                        STBDS_HM_STRING,
                    ) as *mut StrMapEntry;
                }
            }
        }
        assert_eq!(
            hdr3(tc),
            hdr3(tr),
            "step {} (op {}, key {}) header mismatch (mode {:?}, seed {:#x})",
            step,
            op,
            i,
            mode,
            seed
        );
        if step % snap_every == 0 {
            let sc = unsafe { snapshot_map(tc) };
            let sr = unsafe { snapshot_map(tr) };
            assert_eq!(sc, sr, "step {} full snapshot mismatch", step);
        }
    }

    let sc = unsafe { snapshot_map(tc) };
    let sr = unsafe { snapshot_map(tr) };
    assert_eq!(sc, sr, "final snapshot mismatch (mode {:?})", mode);

    unsafe {
        (c.hmfree_func)(tc.sub(1) as *mut c_void, ELEMSIZE);
        (r.hmfree_func)(tr.sub(1) as *mut c_void, ELEMSIZE);
    }
    kc.clear();
    kr.clear();
}

#[test]
fn stress_strdup_mode() {
    let _g = serial();
    stress(Some(STBDS_SH_STRDUP), 0x1234_5678, 120_000, 3000, 5000);
}

#[test]
fn stress_arena_mode() {
    let _g = serial();
    stress(Some(STBDS_SH_ARENA), 0xfeed_face, 120_000, 3000, 5000);
}

#[test]
fn stress_default_mode() {
    let _g = serial();
    // No sh_new_*: table->string.mode becomes STBDS_SH_DEFAULT and the caller's
    // pointer is stored verbatim.
    stress(None, 0x9e37_79b9, 80_000, 500, 4000);
}

#[test]
fn stress_small_universe_dense() {
    let _g = serial();
    // A tiny key space keeps the table small and the buckets nearly full, which
    // is where the wrapped-around probe loops are taken most often.
    for &seed in &[1usize, 7, 0xdead_beef] {
        stress(Some(STBDS_SH_STRDUP), seed, 40_000, 24, 2000);
        stress(Some(STBDS_SH_ARENA), seed, 40_000, 40, 2000);
        stress(None, seed, 40_000, 12, 2000);
    }
}
