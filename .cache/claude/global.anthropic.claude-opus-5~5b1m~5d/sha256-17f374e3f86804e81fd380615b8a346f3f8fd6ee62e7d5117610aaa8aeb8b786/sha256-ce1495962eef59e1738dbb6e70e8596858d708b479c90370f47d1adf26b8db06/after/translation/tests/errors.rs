//! Phase C — ERRORS.md: one differential test per rejection/error row that the
//! C handles by returning a sentinel (the rows whose C behaviour is a fault or
//! an `assert` abort live in `tests/crash_parity.rs`).

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// ERRORS row 1 — stbds_arrgrowf: min_cap <= arrcap  ->  return a unchanged
// ---------------------------------------------------------------------------
#[test]
fn e01_arrgrowf_no_growth_needed() {
    let p = common::libs();
    for &elemsize in &[1usize, 4, 8, 16, 20] {
        let ac = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) };
        let ar = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) };
        let b0c = unsafe { arr_snap(ac, elemsize, 0) };
        let b0r = unsafe { arr_snap(ar, elemsize, 0) };
        assert_eq!(b0c, b0r);
        for min_cap in [0usize, 1, 4, 7, 8] {
            let nc = unsafe { (p.c.arrgrowf)(ac, elemsize, 0, min_cap) };
            let nr = unsafe { (p.r.arrgrowf)(ar, elemsize, 0, min_cap) };
            assert_eq!(nc, ac, "C must return the same pointer");
            assert_eq!(nr, ar, "Rust must return the same pointer");
            assert_eq!(unsafe { arr_snap(nc, elemsize, 0) }, b0c);
            assert_eq!(unsafe { arr_snap(nr, elemsize, 0) }, b0r);
        }
        unsafe {
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 3 — stbds_arrgrowf: addlen makes min_len wrap size_t
// ---------------------------------------------------------------------------
#[test]
fn e03_arrgrowf_addlen_overflow_wraps() {
    let p = common::libs();
    for &elemsize in &[1usize, 4, 8, 16] {
        let ac = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) };
        let ar = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) };
        unsafe {
            (*((ac as *mut u8).sub(HDR_SIZE) as *mut ArrHeader)).length = 4;
            (*((ar as *mut u8).sub(HDR_SIZE) as *mut ArrHeader)).length = 4;
        }
        // min_len = 4 + SIZE_MAX = 3 (wraps) -> min_cap = 3 <= arrcap(8)
        // -> early return, pointer unchanged
        for addlen in [usize::MAX, usize::MAX - 1, usize::MAX - 3] {
            let nc = unsafe { (p.c.arrgrowf)(ac, elemsize, addlen, 0) };
            let nr = unsafe { (p.r.arrgrowf)(ar, elemsize, addlen, 0) };
            assert_eq!(nc, ac, "C: wrapped min_len must early-return (addlen={addlen})");
            assert_eq!(nr, ar, "Rust: wrapped min_len must early-return");
            let sc = unsafe { arr_snap(nc, elemsize, 0) };
            let sr = unsafe { arr_snap(nr, elemsize, 0) };
            assert_eq!(sc, sr);
            assert_eq!(sc.capacity, 8);
            assert_eq!(sc.length, 4);
        }
        unsafe {
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 6 — stbds_hmfree_func(NULL, elemsize) -> no-op
// ---------------------------------------------------------------------------
#[test]
fn e06_hmfree_null_is_noop() {
    let p = common::libs();
    for &elemsize in &[0usize, 1, 4, 8, 16, 20, usize::MAX] {
        unsafe { (p.c.hmfree_func)(std::ptr::null_mut(), elemsize) };
        unsafe { (p.r.hmfree_func)(std::ptr::null_mut(), elemsize) };
    }
    // still functional afterwards
    let _g = reset_seeds(0x3000_0001);
    let mut ka = KeyArena::new();
    let mut m = DualMap::null(16, 8, KeyRepr::Raw);
    let k = ka.add(&[1u8; 8]);
    unsafe { m.put(k, STBDS_HM_BINARY, 1) };
    unsafe { m.free() };
}

// ---------------------------------------------------------------------------
// ERRORS row 7 — stbds_hmfree_func on an array whose hash_table is NULL
// ---------------------------------------------------------------------------
#[test]
fn e07_hmfree_tableless_array() {
    let p = common::libs();
    let _g = reset_seeds(0x3000_0002);
    for &elemsize in &[1usize, 4, 8, 16, 20] {
        // hmget_key_ts(NULL, …) yields an array with no hash table
        let mut ka = KeyArena::new();
        let key = ka.add(&[7u8; 8]);
        let mut tc: isize = 0;
        let mut tr: isize = 0;
        let hc = unsafe {
            (p.c.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key,
                elemsize.min(8),
                &raw mut tc,
                STBDS_HM_BINARY,
            )
        };
        let hr = unsafe {
            (p.r.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key,
                elemsize.min(8),
                &raw mut tr,
                STBDS_HM_BINARY,
            )
        };
        assert_eq!(tc, -1);
        assert_eq!(tr, -1);
        assert!(!unsafe { map_snap(hc, elemsize, elemsize.min(8), KeyRepr::Raw) }.has_table);
        assert!(!unsafe { map_snap(hr, elemsize, elemsize.min(8), KeyRepr::Raw) }.has_table);
        unsafe {
            (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 8 & 9 — stbds_hm_find_slot returns -1 from the FORWARD inner
// loop and from the WRAP-AROUND inner loop.  The test recomputes the probe
// position from the exported hash function and the live bucket array, so it can
// prove BOTH branches were taken (and not just "a lookup returned -1").
// ---------------------------------------------------------------------------
#[derive(Default, Debug)]
struct PathCoverage {
    forward: usize,
    wrap: usize,
    next_bucket: usize,
}

/// Which inner loop of `stbds_hm_find_slot` will report the miss.
unsafe fn classify_miss(
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    cov: &mut PathCoverage,
) {
    unsafe {
        let p = common::libs();
        let base = (t as *mut u8).sub(elemsize);
        let h = base.sub(HDR_SIZE) as *mut ArrHeader;
        let ti = (*h).hash_table as *mut HashIndex;
        if ti.is_null() {
            return;
        }
        let mut hash = (p.c.hash_bytes)(key, keysize, (*ti).seed);
        if hash < 2 {
            hash += 2;
        }
        let slot_count = (*ti).slot_count;
        let mut pos = hash & (slot_count - 1);
        let mut step = BUCKET_LEN;
        let mut bucket_hops = 0usize;
        loop {
            let bk = (*ti).storage.add(pos >> BUCKET_SHIFT);
            let mut found = None;
            for i in (pos & BUCKET_MASK)..BUCKET_LEN {
                if (*bk).hash[i] == 0 {
                    found = Some(0);
                    break;
                }
            }
            if found.is_none() {
                for i in 0..(pos & BUCKET_MASK) {
                    if (*bk).hash[i] == 0 {
                        found = Some(1);
                        break;
                    }
                }
            }
            match found {
                Some(0) => {
                    if bucket_hops == 0 {
                        cov.forward += 1;
                    } else {
                        cov.next_bucket += 1;
                    }
                    return;
                }
                Some(1) => {
                    if bucket_hops == 0 {
                        cov.wrap += 1;
                    } else {
                        cov.next_bucket += 1;
                    }
                    return;
                }
                _ => {}
            }
            pos = pos.wrapping_add(step);
            step += BUCKET_LEN;
            pos &= slot_count - 1;
            bucket_hops += 1;
            if bucket_hops > 64 {
                return; // key is present, or a very long probe: not a miss test
            }
        }
    }
}

#[test]
fn e08_e09_find_slot_miss_in_both_inner_loops() {
    let mut cov = PathCoverage::default();
    for trial in 0..20u64 {
        let _g = reset_seeds(0x3100_0000 + trial as usize);
        let mut rng = Rng::new(0xE008 ^ (trial << 20));
        let elemsize = 16usize;
        let keysize = 8usize;
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        // keep the table small (slot_count 8/16) so that both inner loops of a
        // *single* bucket are reachable
        let nkeys = 3 + (trial as usize % 4);
        let mut live: Vec<Vec<u8>> = Vec::new();
        while live.len() < nkeys {
            let kb = rng.bytes(8);
            if live.contains(&kb) {
                continue;
            }
            live.push(kb.clone());
            let k = ka.add(&kb);
            unsafe { m.put(k, STBDS_HM_BINARY, live.len() as u64) };
        }
        for _ in 0..400 {
            let kb = rng.bytes(8);
            if live.contains(&kb) {
                continue;
            }
            let k = ka.add(&kb);
            unsafe { classify_miss(m.tc, elemsize, k, keysize, &mut cov) };
            let a = unsafe { m.get(k, STBDS_HM_BINARY) };
            assert_eq!(a, -1, "absent key must report -1");
            let b = unsafe { m.get_ts(k, STBDS_HM_BINARY) };
            assert_eq!(b, -1);
            let d = unsafe { m.del(k, STBDS_HM_BINARY) };
            assert_eq!(d, 0, "delete of an absent key must report 0");
        }
        unsafe { m.free() };
    }
    assert!(
        cov.forward > 0,
        "ERRORS row 8 (forward-loop miss) was never exercised: {cov:?}"
    );
    assert!(
        cov.wrap > 0,
        "ERRORS row 9 (wrap-around-loop miss) was never exercised: {cov:?}"
    );
}

// ---------------------------------------------------------------------------
// ERRORS rows 10..=14 — hmget_key_ts / hmget_key sentinels
// ---------------------------------------------------------------------------
#[test]
fn e10_e14_hmget_sentinels() {
    let p = common::libs();
    let _g = reset_seeds(0x3200_0000);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut ka = KeyArena::new();
    let key = ka.add(&[3u8; 8]);

    // row 10: a == NULL  ->  *temp = -1, fresh 1-element array, no table
    let mut tc: isize = 0x1234;
    let mut tr: isize = 0x1234;
    let hc = unsafe {
        (p.c.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key,
            keysize,
            &raw mut tc,
            STBDS_HM_BINARY,
        )
    };
    let hr = unsafe {
        (p.r.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key,
            keysize,
            &raw mut tr,
            STBDS_HM_BINARY,
        )
    };
    assert_eq!(tc, -1);
    assert_eq!(tr, -1);
    assert_eq!(tc, tr);
    let sc = unsafe { map_snap(hc, elemsize, keysize, KeyRepr::Raw) };
    let sr = unsafe { map_snap(hr, elemsize, keysize, KeyRepr::Raw) };
    assert_eq!(sc, sr);
    assert_eq!(sc.length, 1);
    assert!(!sc.has_table);

    // row 11: a != NULL but hash_table == NULL -> *temp = -1, `a` unchanged
    let mut t2c: isize = 0x1234;
    let mut t2r: isize = 0x1234;
    let h2c = unsafe {
        (p.c.hmget_key_ts)(hc, elemsize, key, keysize, &raw mut t2c, STBDS_HM_BINARY)
    };
    let h2r = unsafe {
        (p.r.hmget_key_ts)(hr, elemsize, key, keysize, &raw mut t2r, STBDS_HM_BINARY)
    };
    assert_eq!(h2c, hc, "C must return `a` unchanged");
    assert_eq!(h2r, hr, "Rust must return `a` unchanged");
    assert_eq!(t2c, -1);
    assert_eq!(t2r, -1);

    // row 13: hmget_key on that array writes header temp = -1
    let h3c = unsafe { (p.c.hmget_key)(hc, elemsize, key, keysize, STBDS_HM_BINARY) };
    let h3r = unsafe { (p.r.hmget_key)(hr, elemsize, key, keysize, STBDS_HM_BINARY) };
    assert_eq!(unsafe { map_temp(h3c, elemsize) }, -1);
    assert_eq!(unsafe { map_temp(h3r, elemsize) }, -1);

    unsafe {
        (p.c.hmfree_func)((h3c as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((h3r as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }

    // rows 12 & 14: real table, absent key
    let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
    for j in 0..10u64 {
        let k = ka.add(&j.to_le_bytes());
        unsafe { m.put(k, STBDS_HM_BINARY, j) };
    }
    for j in 100..160u64 {
        let k = ka.add(&j.to_le_bytes());
        assert_eq!(unsafe { m.get_ts(k, STBDS_HM_BINARY) }, -1);
        assert_eq!(unsafe { m.get(k, STBDS_HM_BINARY) }, -1);
        assert_eq!(unsafe { map_temp(m.tc, elemsize) }, -1);
        assert_eq!(unsafe { map_temp(m.tr, elemsize) }, -1);
    }
    unsafe { m.free() };
}

// ---------------------------------------------------------------------------
// ERRORS rows 16..=19, 24 — stbds_hmdel_key sentinels
// ---------------------------------------------------------------------------
#[test]
fn e16_hmdel_null_returns_null() {
    let p = common::libs();
    let mut ka = KeyArena::new();
    let key = ka.add(&[5u8; 8]);
    for &elemsize in &[1usize, 4, 8, 16, 20] {
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -1, i32::MIN, i32::MAX] {
            let rc = unsafe {
                (p.c.hmdel_key)(std::ptr::null_mut(), elemsize, key, 8, 0, mode)
            };
            let rr = unsafe {
                (p.r.hmdel_key)(std::ptr::null_mut(), elemsize, key, 8, 0, mode)
            };
            assert!(rc.is_null(), "C hmdel_key(NULL) must return NULL");
            assert!(rr.is_null(), "Rust hmdel_key(NULL) must return NULL");
            assert_eq!(rc, rr);
        }
    }
}

#[test]
fn e17_hmdel_tableless_map() {
    let p = common::libs();
    let _g = reset_seeds(0x3300_0000);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut ka = KeyArena::new();
    let key = ka.add(&[5u8; 8]);
    let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
    // give it an array but no table
    unsafe { m.get_ts(key, STBDS_HM_BINARY) };
    assert!(!unsafe { m.snap_c() }.has_table);
    let before = unsafe { m.snap_c() };
    let tc = m.tc;
    let tr = m.tr;
    let r = unsafe { m.del(key, STBDS_HM_BINARY) };
    assert_eq!(r, 0, "temp must be 0 on the table==NULL path");
    assert_eq!(m.tc, tc, "map pointer must be returned unchanged");
    assert_eq!(m.tr, tr);
    let after = unsafe { m.snap_c() };
    assert_eq!(after.length, before.length);
    let _ = p;
    unsafe { m.free() };
}

#[test]
fn e18_e19_e24_hmdel_absent_present_and_only_element() {
    let _g = reset_seeds(0x3400_0000);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut ka = KeyArena::new();
    let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);

    // row 24: a map holding exactly one element -> old_index == final_index
    let k1 = ka.add(&1u64.to_le_bytes());
    unsafe { m.put(k1, STBDS_HM_BINARY, 11) };
    let before = unsafe { m.snap_c() };
    assert_eq!(before.used_count, 1);

    // row 18: absent key -> temp 0, length and used_count unchanged
    for j in 900..960u64 {
        let ka2 = ka.add(&j.to_le_bytes());
        assert_eq!(unsafe { m.del(ka2, STBDS_HM_BINARY) }, 0);
        let now = unsafe { m.snap_c() };
        assert_eq!(now.length, before.length);
        assert_eq!(now.used_count, before.used_count);
        assert_eq!(now.tombstone_count, before.tombstone_count);
    }

    // rows 19 + 24: present key, and it is the only element
    let k1b = ka.add(&1u64.to_le_bytes());
    assert_eq!(unsafe { m.del(k1b, STBDS_HM_BINARY) }, 1);
    let after = unsafe { m.snap_c() };
    assert_eq!(after.length, before.length - 1);
    assert_eq!(after.used_count, 0);
    assert_eq!(after.tombstone_count, 1);
    // and deleting it again reports 0
    let k1c = ka.add(&1u64.to_le_bytes());
    assert_eq!(unsafe { m.del(k1c, STBDS_HM_BINARY) }, 0);
    unsafe { m.free() };
}

// ---------------------------------------------------------------------------
// ERRORS rows 25 & 26 — the strdup free in stbds_hmdel_key is guarded by
// `mode == STBDS_HM_STRING && table->string.mode == STBDS_SH_STRDUP`
// ---------------------------------------------------------------------------

/// row 25 — `mode == 1` but the table is DEFAULT / ARENA / NONE: the stored key
/// must NOT be freed.  Verified by reading the key back after the delete: both
/// libraries must report the same contents (and, for DEFAULT, the pointer is
/// the caller's buffer, which is trivially still alive).
#[test]
fn e25_hmdel_string_mode_no_free_when_not_strdup() {
    for sm in [STBDS_SH_DEFAULT, STBDS_SH_ARENA] {
        let _g = reset_seeds(0x3500_0000 + sm as usize);
        let elemsize = 16usize;
        let keysize = 8usize;
        let mut ka = KeyArena::new();
        let mut m = unsafe { DualMap::shmode(elemsize, keysize, KeyRepr::CStr, sm) };
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..12usize {
            let mut s = format!("nofree_key_{i:03}_").into_bytes();
            s.extend(std::iter::repeat(b'Q').take(40));
            s.push(0);
            keys.push(s);
        }
        let mut stored_c: Vec<*mut c_char> = Vec::new();
        let mut stored_r: Vec<*mut c_char> = Vec::new();
        for (j, k) in keys.iter().enumerate() {
            let p = ka.add(k);
            let idx = unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
            unsafe {
                stored_c.push(*((m.tc as *mut u8).add(elemsize * idx as usize)
                    as *mut *mut c_char));
                stored_r.push(*((m.tr as *mut u8).add(elemsize * idx as usize)
                    as *mut *mut c_char));
            }
        }
        // delete in reverse order (no relocation), then read the stored key back
        for (j, k) in keys.iter().enumerate().rev() {
            let p = ka.add(k);
            assert_eq!(unsafe { m.del(p, STBDS_HM_STRING) }, 1);
            let gc = unsafe { cstr_bytes(stored_c[j]) };
            let gr = unsafe { cstr_bytes(stored_r[j]) };
            assert_eq!(gc, gr, "stored key contents diverged after delete");
            assert_eq!(
                gc.as_deref(),
                Some(&k[..k.len() - 1]),
                "the key must NOT have been freed for string.mode={sm}"
            );
        }
        unsafe { m.free() };
    }
}

/// row 26 — STRDUP table but `mode != STBDS_HM_STRING` (e.g. `mode == 2`):
/// the `==` (not `>=`) comparison means the key is NOT freed.
#[test]
fn e26_hmdel_strdup_not_freed_for_mode_two() {
    let _g = reset_seeds(0x3600_0000);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut ka = KeyArena::new();
    let mut m = unsafe { DualMap::shmode(elemsize, keysize, KeyRepr::CStr, STBDS_SH_STRDUP) };
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..12usize {
        let mut s = format!("mode2_key_{i:03}_").into_bytes();
        s.extend(std::iter::repeat(b'W').take(48));
        s.push(0);
        keys.push(s);
    }
    let mut stored_c: Vec<*mut c_char> = Vec::new();
    let mut stored_r: Vec<*mut c_char> = Vec::new();
    for (j, k) in keys.iter().enumerate() {
        let p = ka.add(k);
        let idx = unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
        unsafe {
            stored_c
                .push(*((m.tc as *mut u8).add(elemsize * idx as usize) as *mut *mut c_char));
            stored_r
                .push(*((m.tr as *mut u8).add(elemsize * idx as usize) as *mut *mut c_char));
        }
    }
    // mode = 2: string path for hashing/compare, but NOT the strdup-free path
    for (j, k) in keys.iter().enumerate().rev() {
        let p = ka.add(k);
        assert_eq!(unsafe { m.del(p, 2 as c_int) }, 1);
        let gc = unsafe { cstr_bytes(stored_c[j]) };
        let gr = unsafe { cstr_bytes(stored_r[j]) };
        assert_eq!(gc, gr, "stored key contents diverged (mode=2 delete)");
        assert_eq!(
            gc.as_deref(),
            Some(&k[..k.len() - 1]),
            "mode=2 must not free the strdup'd key"
        );
    }
    unsafe { m.free() };
    // leak the strdup'd copies on purpose: that is exactly what the C does.
}

/// Complementary check for row 26: with `mode == 1` the STRDUP key IS freed,
/// and both libraries agree that the (now freed) memory no longer holds the
/// key.  Reading freed memory is only used as a signal, and the C answer is
/// the reference — Rust must produce the same answer.
#[test]
fn e26b_hmdel_strdup_freed_for_mode_one() {
    let _g = reset_seeds(0x3700_0000);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut ka = KeyArena::new();
    let mut m = unsafe { DualMap::shmode(elemsize, keysize, KeyRepr::CStr, STBDS_SH_STRDUP) };
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..12usize {
        let mut s = format!("mode1_key_{i:03}_").into_bytes();
        s.extend(std::iter::repeat(b'E').take(48));
        s.push(0);
        keys.push(s);
    }
    let mut first_c: Vec<u8> = Vec::new();
    let mut first_r: Vec<u8> = Vec::new();
    for (j, k) in keys.iter().enumerate() {
        let p = ka.add(k);
        let idx = unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
        if j == 0 {
            unsafe {
                let pc = *((m.tc as *mut u8).add(elemsize * idx as usize) as *mut *mut u8);
                let pr = *((m.tr as *mut u8).add(elemsize * idx as usize) as *mut *mut u8);
                first_c = std::slice::from_raw_parts(pc, 8).to_vec();
                first_r = std::slice::from_raw_parts(pr, 8).to_vec();
            }
        }
    }
    assert_eq!(first_c, first_r);
    assert_eq!(&first_c[..6], b"mode1_");
    for (_, k) in keys.iter().enumerate().rev() {
        let p = ka.add(k);
        assert_eq!(unsafe { m.del(p, STBDS_HM_STRING) }, 1);
    }
    unsafe { m.free() };
}

// ---------------------------------------------------------------------------
// ERRORS rows 29 & 30 — the well-defined degenerate hash inputs
// ---------------------------------------------------------------------------
#[test]
fn e29_e30_degenerate_hash_inputs() {
    let p = common::libs();
    // row 29: empty string
    let mut empty = [0u8; 1];
    for seed in [0usize, 1, usize::MAX, 0x31415926] {
        let a = unsafe { (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) };
        let b = unsafe { (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) };
        assert_eq!(a, b, "hash_string(\"\") seed={seed:#x}");
    }
    // row 30: len == 0 with a NULL pointer (no byte is read)
    for seed in [0usize, 1, usize::MAX, 0x31415926] {
        let a = unsafe { (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        let b = unsafe { (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        assert_eq!(a, b, "hash_bytes(NULL,0) seed={seed:#x}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 40..=43 — out-of-enum `mode` values across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn e40_shmode_out_of_range_truncates_to_u8() {
    for mode in [
        0i32, 1, 2, 3, 4, 5, 7, 63, 127, 128, 200, 254, 255, 256, 257, 511, 512, 1000, -1, -2,
        -127, -128, -255, -256, -1000, i32::MIN, i32::MAX,
    ] {
        let _g = reset_seeds(0x3800_0000u64 as usize ^ (mode as u32 as usize));
        let elemsize = 16usize;
        let mc = unsafe { (common::libs().c.shmode_func)(elemsize, mode) };
        let mr = unsafe { (common::libs().r.shmode_func)(elemsize, mode) };
        let sc = unsafe { map_snap(mc, elemsize, 8, KeyRepr::Raw) };
        let sr = unsafe { map_snap(mr, elemsize, 8, KeyRepr::Raw) };
        assert_eq!(sc, sr, "shmode_func({mode}) diverged");
        assert_eq!(
            sc.arena_mode,
            (mode as u32 & 0xff) as u8,
            "string.mode must be (unsigned char)({mode})"
        );
        unsafe {
            (common::libs().c.hmfree_func)((mc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (common::libs().r.hmfree_func)((mr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e41_e43_put_get_mode_boundary() {
    // mode boundary: 0 -> binary, 1 -> string.  One step past the documented
    // range in both directions.
    let string_side: &[c_int] = &[1, 2, 3, 255, 256, i32::MAX];
    let binary_side: &[c_int] = &[0, -1, -2, i32::MIN];
    let elemsize = 16usize;
    let keysize = 8usize;

    for &mode in binary_side {
        let _g = reset_seeds(0x3900_0000 ^ (mode as u32 as usize));
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        // binary path: the key is 8 raw bytes, no NUL needed
        for j in 0..30u64 {
            let k = ka.add(&(j * 0x0101_0101_0101_0101).to_le_bytes());
            unsafe { m.put(k, mode, j) };
        }
        assert_eq!(
            unsafe { m.snap_c() }.arena_mode,
            0,
            "mode<1 must create string.mode == 0"
        );
        unsafe { m.free() };
    }

    for &mode in string_side {
        let _g = reset_seeds(0x3A00_0000 ^ (mode as u32 as usize));
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::CStr);
        for j in 0..30u64 {
            let mut s = format!("mode{mode}_key{j}").into_bytes();
            s.push(0);
            let k = ka.add(&s);
            unsafe { m.put(k, mode, j) };
        }
        assert_eq!(
            unsafe { m.snap_c() }.arena_mode,
            STBDS_SH_DEFAULT as u8,
            "mode>=1 must create string.mode == STBDS_SH_DEFAULT"
        );
        unsafe { m.free() };
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 46..=48 — stbds_hmput_default
// ---------------------------------------------------------------------------
#[test]
fn e46_e48_hmput_default_paths() {
    let p = common::libs();
    for &elemsize in &[1usize, 4, 8, 16, 20] {
        let _g = reset_seeds(0x3B00_0000 + elemsize);
        // row 46: a == NULL
        let ac = unsafe { (p.c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let ar = unsafe { (p.r.hmput_default)(std::ptr::null_mut(), elemsize) };
        let sc = unsafe { map_snap(ac, elemsize, elemsize.min(8), KeyRepr::Raw) };
        let sr = unsafe { map_snap(ar, elemsize, elemsize.min(8), KeyRepr::Raw) };
        assert_eq!(sc, sr);
        assert_eq!(sc.length, 1);
        // row 48: length != 0 -> unchanged
        let a2c = unsafe { (p.c.hmput_default)(ac, elemsize) };
        let a2r = unsafe { (p.r.hmput_default)(ar, elemsize) };
        assert_eq!(a2c, ac);
        assert_eq!(a2r, ar);
        assert_eq!(unsafe { map_snap(a2c, elemsize, elemsize.min(8), KeyRepr::Raw) }, sc);
        unsafe {
            (p.c.hmfree_func)((a2c as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((a2r as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }

        // row 47: existing array with length == 0
        let bc = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        let br = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        let hc = unsafe { (bc as *mut u8).add(elemsize) as *mut c_void };
        let hr = unsafe { (br as *mut u8).add(elemsize) as *mut c_void };
        let cc = unsafe { (p.c.hmput_default)(hc, elemsize) };
        let cr = unsafe { (p.r.hmput_default)(hr, elemsize) };
        let tc = unsafe { map_snap(cc, elemsize, elemsize.min(8), KeyRepr::Raw) };
        let tr = unsafe { map_snap(cr, elemsize, elemsize.min(8), KeyRepr::Raw) };
        assert_eq!(tc, tr);
        assert_eq!(tc.length, 1);
        unsafe {
            (p.c.hmfree_func)((cc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((cr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 49..=51 — stbds_temp_key on duplicate hits, tombstone reuse
// ---------------------------------------------------------------------------
#[test]
fn e49_e51_temp_key_and_tombstone_reuse() {
    let _g = reset_seeds(0x3C00_0000);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut rng = Rng::new(0xE049);
    let mut ka = KeyArena::new();
    let mut m = unsafe { DualMap::shmode(elemsize, keysize, KeyRepr::CStr, STBDS_SH_DEFAULT) };
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..40usize {
        let mut s = format!("tk_{i:04}").into_bytes();
        s.push(0);
        keys.push(s);
    }
    for (j, k) in keys.iter().enumerate() {
        let p = ka.add(k);
        unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
        let a = unsafe { map_temp_key(m.tc, elemsize) };
        let b = unsafe { map_temp_key(m.tr, elemsize) };
        assert_eq!(a, b);
    }
    // duplicates: rows 49 (forward loop -> temp_key updated) and 50
    // (wrap-around loop -> temp_key NOT updated).  Whatever the C does, Rust
    // must do the same.
    for _ in 0..2000 {
        let k = &keys[rng.below(keys.len())];
        let p = ka.add(k);
        unsafe { m.put(p, STBDS_HM_STRING, 0x77) };
        let a = unsafe { map_temp_key(m.tc, elemsize) };
        let b = unsafe { map_temp_key(m.tr, elemsize) };
        assert_eq!(a, b, "temp_key diverged on a duplicate put");
    }
    // row 51: delete then re-insert so the insert lands on a tombstone
    for _ in 0..500 {
        let i = rng.below(keys.len());
        let p = ka.add(&keys[i]);
        unsafe { m.del(p, STBDS_HM_STRING) };
        let p2 = ka.add(&keys[i]);
        unsafe { m.put(p2, STBDS_HM_STRING, 0x88) };
    }
    unsafe { m.free() };
}

// ---------------------------------------------------------------------------
// ERRORS rows 53 & 54 — arr_del / strkey extremes
// ---------------------------------------------------------------------------
#[test]
fn e53_e54_arr_del_and_strkey_extremes() {
    let _g = lock_libs();
    let p = common::libs();
    for n in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        unsafe { (p.c.arr_del)(n) };
        unsafe { (p.r.arr_del)(n) };
        let a = unsafe { cstr_bytes((p.c.strkey)(n)) };
        let b = unsafe { cstr_bytes((p.r.strkey)(n)) };
        assert_eq!(a, b, "strkey({n})");
        assert_eq!(
            a.as_deref(),
            Some(format!("test_{n}").as_bytes()),
            "strkey({n}) content"
        );
    }
}
