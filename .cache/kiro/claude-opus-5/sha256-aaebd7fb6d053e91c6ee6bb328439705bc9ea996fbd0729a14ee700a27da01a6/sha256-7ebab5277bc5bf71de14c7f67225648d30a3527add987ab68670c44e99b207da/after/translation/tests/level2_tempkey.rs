//! Focused verification of `table->temp_key`, the one piece of state whose
//! handling differs between the two probe loops of `stbds_hmput_key`: the first
//! loop stores it when it finds an existing key, the wrapped-around second loop
//! does not.
//!
//! `temp_key` is never initialised by `stbds_make_hash_index`, so it can legally
//! hold uninitialised heap bytes. To stay sound the test never dereferences it;
//! it only compares the *identity* of the pointer against the caller-owned key
//! buffers. `STBDS_SH_DEFAULT` mode (a plain `shput` on a `NULL` map) stores the
//! caller's pointer verbatim, so a recognised pointer maps back to a known key
//! index and an unrecognised one means "uninitialised — not comparable".

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

/// Which of the caller's key buffers `table->temp_key` points at, if any.
unsafe fn temp_key_slot(t: *mut StrMapEntry, keys: &[*mut c_char]) -> Option<usize> {
    unsafe {
        if t.is_null() {
            return None;
        }
        let h = header(t.sub(1) as *mut c_void);
        if h.hash_table.is_null() {
            return None;
        }
        let tk = *(h.hash_table as *mut *mut c_char);
        keys.iter().position(|k| *k == tk)
    }
}

unsafe fn table_ptr(t: *mut StrMapEntry) -> *mut c_void {
    unsafe {
        if t.is_null() {
            std::ptr::null_mut()
        } else {
            header(t.sub(1) as *mut c_void).hash_table
        }
    }
}

#[test]
fn temp_key_identity_matches_under_random_workload() {
    let _g = serial();
    let (c, r) = apis();

    let mut compared = 0usize;
    let mut skipped = 0usize;

    for &(universe, ops, seed) in &[
        (12usize, 40_000usize, 1u64),
        (24, 40_000, 7),
        (64, 60_000, 0xdead_beef),
        (500, 80_000, 0x1234_5678),
        (3000, 120_000, 0xfeed_face),
    ] {
        reset_seeds(&c, &r, DEFAULT_SEED);

        let mut kc: Vec<CStr8> = (0..universe)
            .map(|i| CStr8::new(&format!("test_{}", i)))
            .collect();
        let mut kr: Vec<CStr8> = (0..universe)
            .map(|i| CStr8::new(&format!("test_{}", i)))
            .collect();
        let pc: Vec<*mut c_char> = kc.iter_mut().map(|k| k.as_ptr()).collect();
        let pr: Vec<*mut c_char> = kr.iter_mut().map(|k| k.as_ptr()).collect();

        let mut tc: *mut StrMapEntry = std::ptr::null_mut();
        let mut tr: *mut StrMapEntry = std::ptr::null_mut();

        let mut state = seed | 1;
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
                let before_c = table_ptr(tc);
                let before_r = table_ptr(tr);
                match op {
                    0 => {
                        tc = (c.hmput_key)(
                            tc as *mut c_void,
                            ELEMSIZE,
                            pc[i] as *mut c_void,
                            KEYSIZE,
                            STBDS_HM_STRING,
                        ) as *mut StrMapEntry;
                        tr = (r.hmput_key)(
                            tr as *mut c_void,
                            ELEMSIZE,
                            pr[i] as *mut c_void,
                            KEYSIZE,
                            STBDS_HM_STRING,
                        ) as *mut StrMapEntry;
                        let ic = header(tc.sub(1) as *mut c_void).temp;
                        let ir = header(tr.sub(1) as *mut c_void).temp;
                        assert_eq!(ic, ir, "step {} put {} index", step, i);
                        (*tc.offset(ic)).value = i as c_int;
                        (*tr.offset(ir)).value = i as c_int;
                    }
                    1 => {
                        tc = (c.hmget_key)(
                            tc as *mut c_void,
                            ELEMSIZE,
                            pc[i] as *mut c_void,
                            KEYSIZE,
                            STBDS_HM_STRING,
                        ) as *mut StrMapEntry;
                        tr = (r.hmget_key)(
                            tr as *mut c_void,
                            ELEMSIZE,
                            pr[i] as *mut c_void,
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
                            pc[i] as *mut c_void,
                            KEYSIZE,
                            0,
                            STBDS_HM_STRING,
                        ) as *mut StrMapEntry;
                        tr = (r.hmdel_key)(
                            tr as *mut c_void,
                            ELEMSIZE,
                            pr[i] as *mut c_void,
                            KEYSIZE,
                            0,
                            STBDS_HM_STRING,
                        ) as *mut StrMapEntry;
                    }
                }
                let _ = (before_c, before_r);

                let sc = temp_key_slot(tc, &pc);
                let sr = temp_key_slot(tr, &pr);
                match (sc, sr) {
                    (Some(_), Some(_)) => {
                        assert_eq!(
                            sc, sr,
                            "step {} (op {}, key {}) temp_key points at different keys \
                             (universe {})",
                            step, op, i, universe
                        );
                        compared += 1;
                    }
                    (None, None) => skipped += 1,
                    _ => {
                        // One side recognised the pointer and the other did not.
                        // That can only happen if one implementation stored
                        // temp_key where the other left it uninitialised.
                        panic!(
                            "step {} (op {}, key {}) temp_key definedness differs: \
                             C {:?}, Rust {:?} (universe {})",
                            step, op, i, sc, sr, universe
                        );
                    }
                }
            }
        }
        unsafe {
            (c.hmfree_func)(tc.sub(1) as *mut c_void, ELEMSIZE);
            (r.hmfree_func)(tr.sub(1) as *mut c_void, ELEMSIZE);
        }
        kc.clear();
        kr.clear();
    }

    // Sanity: the comparison must actually have happened most of the time,
    // otherwise this test would be vacuous.
    assert!(
        compared > skipped,
        "temp_key was comparable only {} times out of {}",
        compared,
        compared + skipped
    );
}
