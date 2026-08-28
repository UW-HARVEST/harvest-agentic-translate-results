//! Edge cases that the main suites do not reach: unaligned hash input,
//! `keysize == 0`, string-mode `hmget_key_ts`, `mode > STBDS_HM_STRING`, and
//! `stbds_temp_key` after an *overwrite* (where the C code deliberately leaves it
//! stale in its wrap-around probe loop).

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

const KEYPTR_SIZE: usize = std::mem::size_of::<*mut c_char>();

unsafe fn temp_of(t: *mut c_void, elemsize: usize) -> isize {
    (*header_of(raw_of(t, elemsize))).temp
}

unsafe fn unaligned_hash_bytes(p: &Pair) {
    // 8 KiB scratch so every start offset 0..64 can be used.
    let mut buf = Rng::new(0x9E37_79B9).bytes(8192);
    for off in 0..64usize {
        for &len in &[0usize, 1, 3, 7, 8, 9, 15, 16, 17, 31, 63, 64, 129] {
            let ptr_ = buf.as_mut_ptr().add(off) as *mut c_void;
            for &seed in &[0usize, 0x3141_5926, usize::MAX] {
                let cv = (p.c.hash_bytes)(ptr_, len, seed);
                let rv = (p.r.hash_bytes)(ptr_, len, seed);
                assert_eq!(cv, rv, "unaligned hash_bytes(off={off}, len={len}, seed={seed:#x})");
            }
        }
    }
}

unsafe fn zero_keysize_map(p: &Pair) {
    // keysize == 0: every key hashes the same and memcmp(...,0) always matches,
    // so the map collapses to a single entry. Well defined, just unusual.
    let elemsize = 8usize;
    let cmp_elemsize = elemsize;
    (p.c.rand_seed)(0x3141_5926);
    (p.r.rand_seed)(0x3141_5926);

    let mut dummy = [0u8; 8];
    let mut ct: *mut c_void = ptr::null_mut();
    let mut rt: *mut c_void = ptr::null_mut();

    for i in 0..10u8 {
        dummy[0] = i;
        ct = (p.c.hmput_key)(ct, elemsize, dummy.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
        rt = (p.r.hmput_key)(rt, elemsize, dummy.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
        let cti = temp_of(ct, elemsize);
        let rti = temp_of(rt, elemsize);
        assert_eq!(cti, rti, "keysize=0 put #{i} temp");
        // write the whole element so no padding stays uninitialised
        ptr::write_bytes((ct as *mut u8).add(elemsize * cti as usize), i, elemsize);
        ptr::write_bytes((rt as *mut u8).add(elemsize * rti as usize), i, elemsize);
        let cf = fingerprint(raw_of(ct, cmp_elemsize), cmp_elemsize, false);
        let rf = fingerprint(raw_of(rt, cmp_elemsize), cmp_elemsize, false);
        assert_eq!(cf, rf, "keysize=0 state after put #{i}");
    }

    let cd = (p.c.hmdel_key)(ct, elemsize, dummy.as_mut_ptr() as *mut c_void, 0, 0, HM_BINARY);
    let rd = (p.r.hmdel_key)(rt, elemsize, dummy.as_mut_ptr() as *mut c_void, 0, 0, HM_BINARY);
    assert_eq!(
        fingerprint(raw_of(cd, elemsize), elemsize, false),
        fingerprint(raw_of(rd, elemsize), elemsize, false),
        "keysize=0 state after del"
    );

    (p.c.hmfree_func)(raw_of(cd, elemsize), elemsize);
    (p.r.hmfree_func)(raw_of(rd, elemsize), elemsize);
}

unsafe fn string_get_ts_and_high_modes(p: &Pair) {
    let elemsize = KEYPTR_SIZE + 8;
    (p.c.rand_seed)(0x3141_5926);
    (p.r.rand_seed)(0x3141_5926);

    let mut keys: Vec<Vec<c_char>> = (0..80).map(|i| cbuf(&format!("s{i}"))).collect();
    let mut ct: *mut c_void = ptr::null_mut();
    let mut rt: *mut c_void = ptr::null_mut();

    // `mode = 2` (STBDS_HM_PTR_TO_STRING) must behave like STRING for put/get,
    // because the C code tests `mode >= STBDS_HM_STRING`.
    for (i, k) in keys.iter_mut().enumerate() {
        let mode = if i % 2 == 0 { HM_STRING } else { 2 };
        let kp = k.as_mut_ptr();
        ct = (p.c.hmput_key)(ct, elemsize, kp as *mut c_void, KEYPTR_SIZE, mode);
        rt = (p.r.hmput_key)(rt, elemsize, kp as *mut c_void, KEYPTR_SIZE, mode);
        let cti = temp_of(ct, elemsize);
        let rti = temp_of(rt, elemsize);
        assert_eq!(cti, rti, "mode={mode} put #{i} temp");
        *((ct as *mut u8).add(elemsize * cti as usize).add(KEYPTR_SIZE) as *mut c_int) = i as c_int;
        *((rt as *mut u8).add(elemsize * rti as usize).add(KEYPTR_SIZE) as *mut c_int) = i as c_int;
        assert_eq!(
            fingerprint(raw_of(ct, elemsize), elemsize, true),
            fingerprint(raw_of(rt, elemsize), elemsize, true),
            "mode={mode} state after put #{i}"
        );
        assert_eq!(
            temp_key_str(raw_of(ct, elemsize)),
            temp_key_str(raw_of(rt, elemsize)),
            "mode={mode} temp_key after put #{i}"
        );
    }

    // string-mode hmget_key_ts, hits and misses
    let mut misses: Vec<Vec<c_char>> = (80..160).map(|i| cbuf(&format!("s{i}"))).collect();
    for (i, k) in keys.iter_mut().chain(misses.iter_mut()).enumerate() {
        let kp = k.as_mut_ptr();
        for &mode in &[HM_STRING, 2] {
            let mut ctemp: isize = 0x7777;
            let mut rtemp: isize = 0x7777;
            ct = (p.c.hmget_key_ts)(
                ct,
                elemsize,
                kp as *mut c_void,
                KEYPTR_SIZE,
                &mut ctemp,
                mode,
            );
            rt = (p.r.hmget_key_ts)(
                rt,
                elemsize,
                kp as *mut c_void,
                KEYPTR_SIZE,
                &mut rtemp,
                mode,
            );
            assert_eq!(ctemp, rtemp, "hmget_key_ts(mode={mode}) #{i}");
        }
    }

    // Overwrites: `temp_key` is left stale by the C wrap-around probe loop; the
    // Rust port must reproduce that asymmetry exactly.
    for (i, k) in keys.iter_mut().enumerate() {
        let kp = k.as_mut_ptr();
        ct = (p.c.hmput_key)(ct, elemsize, kp as *mut c_void, KEYPTR_SIZE, HM_STRING);
        rt = (p.r.hmput_key)(rt, elemsize, kp as *mut c_void, KEYPTR_SIZE, HM_STRING);
        assert_eq!(
            temp_of(ct, elemsize),
            temp_of(rt, elemsize),
            "overwrite #{i} temp"
        );
        assert_eq!(
            temp_key_str(raw_of(ct, elemsize)),
            temp_key_str(raw_of(rt, elemsize)),
            "overwrite #{i} temp_key (stale-vs-refreshed must agree)"
        );
        assert_eq!(
            fingerprint(raw_of(ct, elemsize), elemsize, true),
            fingerprint(raw_of(rt, elemsize), elemsize, true),
            "overwrite #{i} state"
        );
    }

    (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
    (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);
    drop(keys.pop());
    drop(misses.pop());
}

unsafe fn hmget_before_any_put(p: &Pair) {
    // hmget_key on a map that has a default element but no hash table at all.
    let elemsize = 16usize;
    (p.c.rand_seed)(11);
    (p.r.rand_seed)(11);

    let ct = (p.c.hmput_default)(ptr::null_mut(), elemsize);
    let rt = (p.r.hmput_default)(ptr::null_mut(), elemsize);
    let mut key = [0xABu8; 8];

    for _ in 0..3 {
        let mut ctemp: isize = 5;
        let mut rtemp: isize = 5;
        let c2 = (p.c.hmget_key_ts)(
            ct,
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut ctemp,
            HM_BINARY,
        );
        let r2 = (p.r.hmget_key_ts)(
            rt,
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            8,
            &mut rtemp,
            HM_BINARY,
        );
        assert_eq!(ctemp, rtemp, "hmget_key_ts on table-less map");
        assert_eq!(c2, ct, "C hmget_key_ts moved the map");
        assert_eq!(r2, rt, "Rust hmget_key_ts moved the map");
        assert_eq!(
            fingerprint(raw_of(ct, elemsize), elemsize, false),
            fingerprint(raw_of(rt, elemsize), elemsize, false)
        );
    }

    (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
    (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);
}

#[test]
fn edge_cases_match_c() {
    let p = load_pair();
    unsafe {
        unaligned_hash_bytes(&p);
        zero_keysize_map(&p);
        string_get_ts_and_high_modes(&p);
        hmget_before_any_put(&p);
    }
}
