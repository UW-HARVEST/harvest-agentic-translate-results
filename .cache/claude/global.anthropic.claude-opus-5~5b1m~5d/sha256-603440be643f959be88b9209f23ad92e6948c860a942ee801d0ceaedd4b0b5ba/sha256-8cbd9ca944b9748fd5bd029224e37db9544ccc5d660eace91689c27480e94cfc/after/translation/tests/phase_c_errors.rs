//! Phase C: one differential test per row of `ERRORS.md`.
//! Every test constructs the exact invalid input / rejection condition and
//! asserts that BOTH `.so`s return the SAME sentinel, error index or state.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// =========================================================== E1
#[test]
fn e1_arrgrowf_no_grow() {
    let s = session(1);
    for elemsize in [1usize, 8, 16, 24] {
        unsafe {
            let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let cap = header(ca).capacity;
            assert_eq!(cap, header(ra).capacity);
            // every min_cap <= cap must be rejected with an unchanged pointer
            for req in [0usize, 1, cap - 1, cap] {
                let c2 = (s.c.arrgrowf)(ca, elemsize, 0, req);
                let r2 = (s.r.arrgrowf)(ra, elemsize, 0, req);
                assert_eq!(c2, ca, "E1 e={elemsize} req={req}: C reallocated");
                assert_eq!(r2, ra, "E1 e={elemsize} req={req}: Rust reallocated");
                assert_eq!(header(ca).capacity, cap, "E1: C capacity changed");
                assert_eq!(header(ra).capacity, cap, "E1: Rust capacity changed");
                assert_eq!(header(ca).length, header(ra).length);
                assert_eq!(header(ca).temp, header(ra).temp);
            }
            (s.c.arrfreef)(ca);
            (s.r.arrfreef)(ra);
        }
    }
}

// =========================================================== E2
#[test]
fn e2_arrgrowf_null_zero() {
    let s = session(1);
    for elemsize in [0usize, 1, 8, 16, 1024] {
        unsafe {
            let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(ca.is_null(), "E2 e={elemsize}: C must return NULL");
            assert!(ra.is_null(), "E2 e={elemsize}: Rust must return NULL");
        }
    }
}

// =========================================================== E3
#[test]
fn e3_arrgrowf_min_cap_4() {
    let s = session(1);
    for elemsize in [1usize, 8, 16] {
        for (addlen, min_cap) in [(0usize, 1usize), (0, 2), (0, 3), (0, 4), (1, 0), (2, 0), (3, 0)] {
            unsafe {
                let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ctx = format!("E3 e={elemsize} add={addlen} cap={min_cap}");
                assert_eq!(header(ca).capacity, 4, "{ctx}: C capacity must clamp to 4");
                assert_eq!(header(ra).capacity, 4, "{ctx}: Rust capacity must clamp to 4");
                assert_eq!(header(ca).length, 0, "{ctx}: C length");
                assert_eq!(header(ra).length, 0, "{ctx}: Rust length");
                assert_eq!(header(ca).temp, 0, "{ctx}: C temp");
                assert_eq!(header(ra).temp, 0, "{ctx}: Rust temp");
                assert!(header(ca).hash_table.is_null(), "{ctx}: C hash_table");
                assert!(header(ra).hash_table.is_null(), "{ctx}: Rust hash_table");
                (s.c.arrfreef)(ca);
                (s.r.arrfreef)(ra);
            }
        }
    }
}

// =========================================================== E4
#[test]
fn e4_arrgrowf_elemsize_zero() {
    let s = session(1);
    unsafe {
        for min_cap in [1usize, 4, 5, 1000] {
            let ca = (s.c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let ra = (s.r.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let ctx = format!("E4 min_cap={min_cap}");
            assert!(!ca.is_null() && !ra.is_null(), "{ctx}: non-NULL expected");
            assert_eq!(header(ca).capacity, header(ra).capacity, "{ctx}: capacity");
            assert_eq!(header(ca).length, header(ra).length, "{ctx}: length");
            assert_eq!(header(ca).temp, header(ra).temp, "{ctx}: temp");
            // growing a zero-elemsize array further
            let c2 = (s.c.arrgrowf)(ca, 0, 100, 0);
            let r2 = (s.r.arrgrowf)(ra, 0, 100, 0);
            assert_eq!(header(c2).capacity, header(r2).capacity, "{ctx}: grown capacity");
            (s.c.arrfreef)(c2);
            (s.r.arrfreef)(r2);
        }
    }
}

// =========================================================== E5 (documented)
/// `stbds_arrfreef(NULL)` computes `free((char*)NULL - 32)`, an invalid free that
/// aborts glibc.  It is UB in the C and is therefore only documented, not run.
/// The Rust performs the byte-identical `wrapping_sub(32)` + `free`.
#[test]
fn e5_arrfreef_null_is_documented_not_run() {
    // Assert only that both libraries expose the symbol with the same signature.
    let s = session(1);
    let cf = s.c.arrfreef as usize;
    let rf = s.r.arrfreef as usize;
    assert_ne!(cf, 0);
    assert_ne!(rf, 0);
    assert_ne!(cf, rf, "the two .so must provide distinct implementations");
}

// =========================================================== E6
#[test]
fn e6_hmfree_null() {
    let s = session(1);
    unsafe {
        for elemsize in [0usize, 1, 16, 24] {
            (s.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (s.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

// =========================================================== E7
#[test]
fn e7_hmfree_no_table() {
    let s = session(1);
    unsafe {
        for elemsize in [1usize, 8, 16, 24] {
            let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 5, 0);
            let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 5, 0);
            assert!(header(ca).hash_table.is_null());
            assert!(header(ra).hash_table.is_null());
            // must not touch the (absent) table and must not crash
            (s.c.hmfree_func)(ca, elemsize);
            (s.r.hmfree_func)(ra, elemsize);
        }
    }
}

// =========================================================== E8 / E9 / E12
#[test]
fn e9_get_absent_key() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    for keysize in [1usize, 2, 4, 8, 16] {
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0xE9 + keysize as u64);
        let mut present: Vec<Vec<u8>> = Vec::new();
        // Populate enough to force wrap-around probing (upper + lower scans).
        for i in 0..(if keysize == 1 { 200 } else { 400 }) {
            let mut k = rng.bytes(keysize);
            k[0] |= 0x80; // reserve the 0x00..0x7F half for the absent keys
            d.hmput(&format!("E9 put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
            if !present.contains(&k) {
                present.push(k);
            }
        }
        for i in 0..200usize {
            let mut k = rng.bytes(keysize);
            k[0] &= 0x7F; // guaranteed absent
            let ctx = format!("E9 k={keysize} miss#{i}");
            assert_eq!(d.hmgeti(&ctx, &k, HM_BINARY), -1, "{ctx}: must be -1");
            assert_eq!(d.hmgeti_ts(&ctx, &k, HM_BINARY), -1, "{ctx}: _ts must be -1");
            d.check(&ctx);
        }
        d.free();
    }
}

// =========================================================== E10
#[test]
fn e10_hmget_ts_null_a() {
    let s = session(0x31415926);
    for elemsize in [1usize, 8, 16, 24] {
        for keysize in [0usize, 1, 4, 8] {
            unsafe {
                let key = [0xABu8; 16];
                let kp = key.as_ptr() as *mut c_void;
                let mut ct: isize = 0x1234;
                let mut rt: isize = 0x1234;
                let cm = (s.c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    kp,
                    keysize,
                    &mut ct,
                    HM_BINARY,
                );
                let rm = (s.r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    kp,
                    keysize,
                    &mut rt,
                    HM_BINARY,
                );
                let ctx = format!("E10 e={elemsize} k={keysize}");
                assert_eq!(ct, -1, "{ctx}: C *temp must be STBDS_INDEX_EMPTY");
                assert_eq!(rt, -1, "{ctx}: Rust *temp must be STBDS_INDEX_EMPTY");
                assert!(!cm.is_null() && !rm.is_null(), "{ctx}: non-NULL map expected");
                let cs = map_snap(cm, elemsize, KeyKind::Binary);
                let rs = map_snap(rm, elemsize, KeyKind::Binary);
                assert_snap_eq(&ctx, &cs, &rs);
                assert_eq!(cs.length, 1, "{ctx}: length must be 1");
                assert!(cs.table.is_none(), "{ctx}: no hash table must exist yet");
                assert!(cs.elems[0].0.iter().all(|b| *b == 0), "{ctx}: elem 0 zeroed");
                (s.c.hmfree_func)(raw_of(cm, elemsize), elemsize);
                (s.r.hmfree_func)(raw_of(rm, elemsize), elemsize);
            }
        }
    }
}

// =========================================================== E11
#[test]
fn e11_hmget_ts_no_table() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    unsafe {
        // array with length>=1 but hash_table == NULL
        let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
        let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
        let cm = (ca as *mut u8).add(elemsize) as *mut c_void;
        let rm = (ra as *mut u8).add(elemsize) as *mut c_void;
        let key = [7u8; 8];
        let kp = key.as_ptr() as *mut c_void;
        for mode in [HM_BINARY, HM_STRING, 2, -1] {
            let mut ct: isize = 99;
            let mut rt: isize = 99;
            let c2 = (s.c.hmget_key_ts)(cm, elemsize, kp, 8, &mut ct, mode);
            let r2 = (s.r.hmget_key_ts)(rm, elemsize, kp, 8, &mut rt, mode);
            let ctx = format!("E11 mode={mode}");
            assert_eq!(ct, -1, "{ctx}: C *temp");
            assert_eq!(rt, -1, "{ctx}: Rust *temp");
            assert_eq!(c2, cm, "{ctx}: C must return `a` unchanged");
            assert_eq!(r2, rm, "{ctx}: Rust must return `a` unchanged");
        }
        (s.c.arrfreef)(ca);
        (s.r.arrfreef)(ra);
    }
}

// =========================================================== E13
#[test]
fn e13_hmget_temp_written() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    unsafe {
        // (a) bootstrap: hmget_key must write *temp into the NEW header
        let key = [3u8; 4];
        let kp = key.as_ptr() as *mut c_void;
        let cm = (s.c.hmget_key)(std::ptr::null_mut(), elemsize, kp, 4, HM_BINARY);
        let rm = (s.r.hmget_key)(std::ptr::null_mut(), elemsize, kp, 4, HM_BINARY);
        assert_eq!(header(raw_of(cm, elemsize)).temp, -1, "E13a: C temp");
        assert_eq!(header(raw_of(rm, elemsize)).temp, -1, "E13a: Rust temp");
        assert_snap_eq(
            "E13a",
            &map_snap(cm, elemsize, KeyKind::Binary),
            &map_snap(rm, elemsize, KeyKind::Binary),
        );
        (s.c.hmfree_func)(raw_of(cm, elemsize), elemsize);
        (s.r.hmfree_func)(raw_of(rm, elemsize), elemsize);
    }
    // (b) no-table and (c) absent-key: temp must become -1 in both
    let mut d = Driver::lazy(&s, elemsize, 4, KeyKind::Binary);
    let k = 5u32.to_ne_bytes().to_vec();
    assert_eq!(d.hmgeti("E13b", &k, HM_BINARY), -1);
    d.check("E13b");
    let mut rng = Rng::new(0xE13);
    for i in 0..30 {
        d.hmput(&format!("E13 put#{i}"), &(i as u32).to_ne_bytes().to_vec(), &rng.bytes(elemsize), HM_BINARY);
    }
    for i in 1000..1030u32 {
        let ctx = format!("E13c miss {i}");
        assert_eq!(d.hmgeti(&ctx, &i.to_ne_bytes().to_vec(), HM_BINARY), -1, "{ctx}");
        d.check(&ctx);
    }
    d.free();
}

// =========================================================== E14 / E15 / E16
#[test]
fn e14_hmput_default_null() {
    let s = session(1);
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 24] {
            let cm = (s.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let rm = (s.r.hmput_default)(std::ptr::null_mut(), elemsize);
            let ctx = format!("E14 e={elemsize}");
            assert!(!cm.is_null() && !rm.is_null(), "{ctx}");
            let cs = map_snap(cm, elemsize, KeyKind::Binary);
            let rs = map_snap(rm, elemsize, KeyKind::Binary);
            assert_snap_eq(&ctx, &cs, &rs);
            assert_eq!(cs.length, 1, "{ctx}: length");
            assert!(cs.elems[0].0.iter().all(|b| *b == 0), "{ctx}: elem zeroed");
            (s.c.arrfreef)(raw_of(cm, elemsize));
            (s.r.arrfreef)(raw_of(rm, elemsize));
        }
    }
}

#[test]
fn e15_hmput_default_len0() {
    let s = session(1);
    unsafe {
        for elemsize in [1usize, 8, 16, 24] {
            let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 6);
            let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 6);
            assert_eq!(header(ca).length, 0);
            let cm = (s.c.hmput_default)((ca as *mut u8).add(elemsize) as *mut c_void, elemsize);
            let rm = (s.r.hmput_default)((ra as *mut u8).add(elemsize) as *mut c_void, elemsize);
            let ctx = format!("E15 e={elemsize}");
            let cs = map_snap(cm, elemsize, KeyKind::Binary);
            let rs = map_snap(rm, elemsize, KeyKind::Binary);
            assert_snap_eq(&ctx, &cs, &rs);
            assert_eq!(cs.length, 1, "{ctx}: length must become 1");
            (s.c.arrfreef)(raw_of(cm, elemsize));
            (s.r.arrfreef)(raw_of(rm, elemsize));
        }
    }
}

#[test]
fn e16_hmput_default_passthru() {
    let s = session(1);
    unsafe {
        for elemsize in [1usize, 8, 16, 24] {
            let cm0 = (s.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let rm0 = (s.r.hmput_default)(std::ptr::null_mut(), elemsize);
            for round in 0..4 {
                let cm = (s.c.hmput_default)(cm0, elemsize);
                let rm = (s.r.hmput_default)(rm0, elemsize);
                let ctx = format!("E16 e={elemsize} round={round}");
                assert_eq!(cm, cm0, "{ctx}: C must pass the pointer through");
                assert_eq!(rm, rm0, "{ctx}: Rust must pass the pointer through");
                assert_eq!(header(raw_of(cm, elemsize)).length, 1, "{ctx}: C length");
                assert_eq!(header(raw_of(rm, elemsize)).length, 1, "{ctx}: Rust length");
            }
            (s.c.arrfreef)(raw_of(cm0, elemsize));
            (s.r.arrfreef)(raw_of(rm0, elemsize));
        }
    }
}

// =========================================================== E17
#[test]
fn e17_hmdel_null() {
    let s = session(1);
    unsafe {
        let key = [1u8; 8];
        let kp = key.as_ptr() as *mut c_void;
        for elemsize in [1usize, 16, 24] {
            for mode in [HM_BINARY, HM_STRING, 2, -1, c_int::MAX, c_int::MIN] {
                let c = (s.c.hmdel_key)(std::ptr::null_mut(), elemsize, kp, 8, 0, mode);
                let r = (s.r.hmdel_key)(std::ptr::null_mut(), elemsize, kp, 8, 0, mode);
                let ctx = format!("E17 e={elemsize} mode={mode}");
                assert!(c.is_null(), "{ctx}: C must return 0/NULL");
                assert!(r.is_null(), "{ctx}: Rust must return 0/NULL");
            }
        }
    }
}

// =========================================================== E18
#[test]
fn e18_hmdel_no_table() {
    let s = session(1);
    let elemsize = 16usize;
    unsafe {
        let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
        let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
        (*((ca as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).temp = 0x7777;
        (*((ra as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).temp = 0x7777;
        let cm = (ca as *mut u8).add(elemsize) as *mut c_void;
        let rm = (ra as *mut u8).add(elemsize) as *mut c_void;
        let key = [9u8; 8];
        let kp = key.as_ptr() as *mut c_void;
        for mode in [HM_BINARY, HM_STRING, 2, -5] {
            let c2 = (s.c.hmdel_key)(cm, elemsize, kp, 8, 0, mode);
            let r2 = (s.r.hmdel_key)(rm, elemsize, kp, 8, 0, mode);
            let ctx = format!("E18 mode={mode}");
            assert_eq!(c2, cm, "{ctx}: C returns `a`");
            assert_eq!(r2, rm, "{ctx}: Rust returns `a`");
            assert_eq!(header(ca).temp, 0, "{ctx}: C must zero temp");
            assert_eq!(header(ra).temp, 0, "{ctx}: Rust must zero temp");
            // `arrgrowf` only reserves capacity; `length` stays 0.
            assert_eq!(header(ca).length, 0, "{ctx}: C length unchanged");
            assert_eq!(header(ra).length, 0, "{ctx}: Rust length unchanged");
            assert_eq!(header(ca).capacity, header(ra).capacity, "{ctx}: capacity");
        }
        (s.c.arrfreef)(ca);
        (s.r.arrfreef)(ra);
    }
}

// =========================================================== E19
#[test]
fn e19_hmdel_absent() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    for keysize in [1usize, 4, 8] {
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0xE19 + keysize as u64);
        for i in 0..120usize {
            let mut k = rng.bytes(keysize);
            k[0] |= 0x80;
            d.hmput(&format!("E19 put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
        }
        let before = d.snap_c();
        for i in 0..100usize {
            let mut k = rng.bytes(keysize);
            k[0] &= 0x7F;
            let ctx = format!("E19 k={keysize} del-absent#{i}");
            assert_eq!(d.hmdel(&ctx, &k, 0, HM_BINARY), 0, "{ctx}: temp must be 0");
            d.check(&ctx);
        }
        let after = d.snap_c();
        assert_eq!(before.length, after.length, "E19: length must not change");
        assert_eq!(
            before.table.as_ref().map(|t| t.used_count),
            after.table.as_ref().map(|t| t.used_count),
            "E19: used_count must not change"
        );
        assert_eq!(
            before.table.as_ref().map(|t| t.tombstone_count),
            after.table.as_ref().map(|t| t.tombstone_count),
            "E19: tombstone_count must not change"
        );
        d.free();
    }
}

// =========================================================== E20
#[test]
fn e20_hmdel_present() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
    let mut rng = Rng::new(0xE20);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..60usize {
        let k = rng.bytes(keysize);
        if keys.contains(&k) {
            continue;
        }
        d.hmput(&format!("E20 put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
        keys.push(k);
    }
    for (i, k) in keys.iter().enumerate() {
        let before_len = d.snap_c().length;
        let ctx = format!("E20 del#{i}");
        assert_eq!(d.hmdel(&ctx, k, 0, HM_BINARY), 1, "{ctx}: temp must be 1");
        let after = d.snap_c();
        assert_eq!(after.length, before_len - 1, "{ctx}: length must drop by 1");
        d.check(&ctx);
        // a tombstone (hash == STBDS_HASH_DELETED, index == STBDS_INDEX_DELETED)
        // must exist unless the table was just rebuilt/shrunk
        let t = after.table.as_ref().unwrap();
        let tombs = t
            .hashes
            .iter()
            .zip(t.indices.iter())
            .filter(|(h, ix)| **h == 1 && **ix == -2)
            .count();
        assert_eq!(
            tombs, t.tombstone_count,
            "{ctx}: tombstone slots must match tombstone_count"
        );
        // deleting the same key again must now be a no-op
        assert_eq!(d.hmdel(&format!("{ctx} again"), k, 0, HM_BINARY), 0, "{ctx} again");
        d.check(&format!("{ctx} again"));
    }
    d.free();
}

// =========================================================== E21 / E22
#[test]
fn e21_e22_hmdel_shrink_and_rebuild() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
    let mut rng = Rng::new(0xE21);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..400usize {
        let k = rng.bytes(keysize);
        if keys.contains(&k) {
            continue;
        }
        d.hmput(&format!("E21 put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
        keys.push(k);
    }
    let mut shrinks = 0usize;
    let mut rebuilds = 0usize;
    while !keys.is_empty() {
        let i = rng.below(keys.len());
        let k = keys.remove(i);
        let before = d.snap_c().table.unwrap();
        let ctx = format!("E21 del len={}", keys.len());
        assert_eq!(d.hmdel(&ctx, &k, 0, HM_BINARY), 1, "{ctx}");
        d.check(&ctx);
        let after = d.snap_c().table.unwrap();
        if after.slot_count == before.slot_count / 2 {
            shrinks += 1;
            assert_eq!(after.tombstone_count, 0, "{ctx}: shrink must clear tombstones");
            assert!(
                after.slot_count >= 8,
                "{ctx}: shrink must never go below STBDS_BUCKET_LENGTH"
            );
        } else if after.slot_count == before.slot_count && before.tombstone_count > 0 && after.tombstone_count == 0
        {
            rebuilds += 1;
        }
    }
    assert!(shrinks > 0, "E21: shrink branch (lib.c:854) never taken");
    assert!(rebuilds > 0, "E22: rebuild branch (lib.c:858) never taken");
    // slot_count must be back down to the 8-slot minimum
    assert_eq!(d.snap_c().table.unwrap().slot_count, 8, "E21: final slot_count");
    d.check("E21 final");
    d.free();
}

// =========================================================== E23
#[test]
fn e23_hash_string_empty() {
    let s = session(1);
    let cs = CStrBuf::new(b"");
    for seed in [0usize, 1, 2, 0x31415926, usize::MAX, usize::MAX - 1, 1 << 63] {
        unsafe {
            assert_eq!(
                (s.c.hash_string)(cs.ptr(), seed),
                (s.r.hash_string)(cs.ptr(), seed),
                "E23 seed={seed:#x}"
            );
        }
    }
}

// =========================================================== E24 / G1
#[test]
fn e24_hash_bytes_len0() {
    let s = session(1);
    for seed in [0usize, 1, 0x31415926, usize::MAX, 1 << 63] {
        unsafe {
            // NULL pointer with len == 0: nothing may be dereferenced.
            assert_eq!(
                (s.c.hash_bytes)(std::ptr::null_mut(), 0, seed),
                (s.r.hash_bytes)(std::ptr::null_mut(), 0, seed),
                "E24 NULL seed={seed:#x}"
            );
            // Dangling / unaligned pointer, still len == 0.
            let odd = 0x1usize as *mut c_void;
            assert_eq!(
                (s.c.hash_bytes)(odd, 0, seed),
                (s.r.hash_bytes)(odd, 0, seed),
                "E24 dangling seed={seed:#x}"
            );
        }
    }
}

// =========================================================== E25 / G2
#[test]
fn e25_hash_bytes_all_remainders() {
    let s = session(1);
    let mut rng = Rng::new(0xE25);
    let buf = rng.bytes(256);
    let p = buf.as_ptr() as *mut c_void;
    // every `switch (len - i)` arm 0..=7, at every block count 0..=10
    for blocks in 0..=10usize {
        for rem in 0..=7usize {
            let len = blocks * 8 + rem;
            for seed in [0usize, 1, usize::MAX, 0x31415926] {
                unsafe {
                    assert_eq!(
                        (s.c.hash_bytes)(p, len, seed),
                        (s.r.hash_bytes)(p, len, seed),
                        "E25 blocks={blocks} rem={rem} len={len} seed={seed:#x}"
                    );
                }
            }
        }
    }
    // boundary lengths called out by G2
    for len in [1usize, 7, 8, 9, 15, 16, 17, 63, 64, 65, 127, 128] {
        unsafe {
            assert_eq!(
                (s.c.hash_bytes)(p, len, 0),
                (s.r.hash_bytes)(p, len, 0),
                "G2 len={len}"
            );
        }
    }
}

// =========================================================== E26
#[test]
fn e26_stralloc_oversize() {
    let s = session(1);
    unsafe {
        for len in [512usize, 600, 1024, 5000, 1 << 20] {
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            let body = vec![b'q'; len];
            let cs = CStrBuf::new(&body);
            let cp = (s.c.stralloc)(&mut ca, cs.ptr());
            let rp = (s.r.stralloc)(&mut ra, cs.ptr());
            let ctx = format!("E26 len={len}");
            assert_eq!(read_cstr(cp), body, "{ctx}: C content");
            assert_eq!(read_cstr(rp), body, "{ctx}: Rust content");
            // fresh-arena oversize sub-case forces remaining back to 0
            assert_eq!(ca.remaining, 0, "{ctx}: C remaining");
            assert_eq!(ra.remaining, 0, "{ctx}: Rust remaining");
            assert_eq!(ca.block, ra.block, "{ctx}: block");
            assert_eq!(ca.block, 1, "{ctx}: block must have advanced once");
            assert!(!ca.storage.is_null() && !ra.storage.is_null(), "{ctx}: storage");
            (s.c.strreset)(&mut ca);
            (s.r.strreset)(&mut ra);
        }
    }
}

// =========================================================== E27
#[test]
fn e27_stralloc_block_saturates() {
    let s = session(1);
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let cs = CStrBuf::new(&vec![b'w'; 511]);
        for i in 0..9000usize {
            let cp = (s.c.stralloc)(&mut ca, cs.ptr());
            let rp = (s.r.stralloc)(&mut ra, cs.ptr());
            assert_eq!(read_cstr(cp).len(), 511, "E27 #{i}: C length");
            assert_eq!(read_cstr(rp).len(), 511, "E27 #{i}: Rust length");
            assert_eq!(ca.block, ra.block, "E27 #{i}: block diverged");
            assert_eq!(ca.remaining, ra.remaining, "E27 #{i}: remaining diverged");
            assert!(ca.block <= 22, "E27 #{i}: block exceeded saturation ({})", ca.block);
        }
        assert_eq!(ca.block, 22, "E27: block must saturate at 22");
        assert_eq!(ra.block, 22, "E27: Rust block must saturate at 22");
        (s.c.strreset)(&mut ca);
        (s.r.strreset)(&mut ra);
    }
}

// =========================================================== E28
#[test]
fn e28_stralloc_empty() {
    let s = session(1);
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let cs = CStrBuf::new(b"");
        for i in 0..600usize {
            let cp = (s.c.stralloc)(&mut ca, cs.ptr());
            let rp = (s.r.stralloc)(&mut ra, cs.ptr());
            assert_eq!(read_cstr(cp), b"", "E28 #{i}: C content");
            assert_eq!(read_cstr(rp), b"", "E28 #{i}: Rust content");
            assert_eq!(ca.remaining, ra.remaining, "E28 #{i}: remaining");
            assert_eq!(ca.block, ra.block, "E28 #{i}: block");
        }
        (s.c.strreset)(&mut ca);
        (s.r.strreset)(&mut ra);
    }
}

// =========================================================== E29 / E30
#[test]
fn e29_strreset_empty() {
    let s = session(1);
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        for round in 0..5 {
            (s.c.strreset)(&mut ca);
            (s.r.strreset)(&mut ra);
            let ctx = format!("E29 round={round}");
            assert_eq!(ca.remaining, 0, "{ctx}: C remaining");
            assert_eq!(ra.remaining, 0, "{ctx}: Rust remaining");
            assert_eq!(ca.block, 0, "{ctx}: C block");
            assert_eq!(ra.block, 0, "{ctx}: Rust block");
            assert_eq!(ca.mode, 0, "{ctx}: C mode");
            assert_eq!(ra.mode, 0, "{ctx}: Rust mode");
            assert!(ca.storage.is_null(), "{ctx}: C storage");
            assert!(ra.storage.is_null(), "{ctx}: Rust storage");
        }
        // E30: reset must also wipe a non-zero `mode`
        ca.mode = 3;
        ra.mode = 3;
        (s.c.strreset)(&mut ca);
        (s.r.strreset)(&mut ra);
        assert_eq!(ca.mode, 0, "E30: C mode must be memset to 0");
        assert_eq!(ra.mode, 0, "E30: Rust mode must be memset to 0");
    }
}

// =========================================================== E31 / G9
#[test]
fn e31_strkey_boundaries() {
    let s = session(1);
    for n in [
        0i32,
        1,
        -1,
        9,
        -9,
        10,
        -10,
        99,
        -99,
        100,
        -100,
        999,
        1000,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
    ] {
        unsafe {
            let c = read_cstr((s.c.strkey)(n));
            let r = read_cstr((s.r.strkey)(n));
            assert_eq!(c, r, "E31 strkey({n})");
            assert_eq!(String::from_utf8_lossy(&c), format!("test_{n}"), "E31 content");
            // the returned pointer must be a stable static buffer per library
            let c2 = (s.c.strkey)(n);
            let c3 = (s.c.strkey)(n);
            assert_eq!(c2, c3, "E31: C buffer address must be stable");
            let r2 = (s.r.strkey)(n);
            let r3 = (s.r.strkey)(n);
            assert_eq!(r2, r3, "E31: Rust buffer address must be stable");
        }
    }
}

// =========================================================== E32 / G8
#[test]
fn e32_sh_puts_nonpositive() {
    let s = session(0x31415926);
    for num in [0i32, -1, -2, -7, -100, -1000, i32::MIN, i32::MIN + 1] {
        s.seed(0x31415926);
        let cout = capture_stdout(|| unsafe { (s.c.sh_puts)(num) });
        s.seed(0x31415926);
        let rout = capture_stdout(|| unsafe { (s.r.sh_puts)(num) });
        assert_eq!(
            cout,
            rout,
            "E32 sh_puts({num}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );
        assert_eq!(
            String::from_utf8_lossy(&cout),
            format!("a {num}\n"),
            "E32 sh_puts({num}) content"
        );
    }
}
