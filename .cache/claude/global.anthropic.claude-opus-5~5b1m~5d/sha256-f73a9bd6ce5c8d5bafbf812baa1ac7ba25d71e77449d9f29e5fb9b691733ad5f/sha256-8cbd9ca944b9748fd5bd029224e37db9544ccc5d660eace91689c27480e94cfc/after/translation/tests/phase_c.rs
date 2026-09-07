// Phase C — error/rejection-path differential tests, one per ERRORS.md row.
//
// Every row asserts the two libraries return the SAME sentinel / error code /
// state, not merely that both "failed somehow".

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;

const DEFAULT_SEED: usize = 0x31415926;

// ===========================================================================
// Rows 1-3 — stbds_arrgrowf rejection / clamping branches
// ===========================================================================

#[test]
fn err01_arrgrowf_early_return_no_op() {
    let _g = guard();
    let p = pair();
    unsafe {
        for &elemsize in &[1usize, 4, 8, 16, 64] {
            // NULL + (0,0) -> `min_cap <= arrcap(NULL)` -> returns NULL
            diff(
                "1",
                &format!("NULL/(0,0) elemsize={elemsize}"),
                (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0),
                (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0),
            );
            // existing array, request <= capacity -> same pointer, no change
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            diff("1", "capacity of fresh(10)", header(ac).capacity, header(ar).capacity);
            for &(addlen, min_cap) in &[(0usize, 0usize), (0, 1), (0, 10), (3, 4), (10, 0), (5, 5)] {
                let bc = header(ac);
                let br = header(ar);
                let nc = (p.c.arrgrowf)(ac, elemsize, addlen, min_cap);
                let nr = (p.r.arrgrowf)(ar, elemsize, addlen, min_cap);
                assert_eq!(nc, ac, "row 1: C must not realloc for ({addlen},{min_cap})");
                assert_eq!(nr, ar, "row 1: RUST must not realloc for ({addlen},{min_cap})");
                diff("1", "header unchanged", bc, header(nc));
                diff("1", "header unchanged", br, header(nr));
            }
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

#[test]
fn err02_arrgrowf_fresh_zeroes_header() {
    let _g = guard();
    let p = pair();
    unsafe {
        for &elemsize in &[1usize, 3, 8, 24] {
            for &(addlen, min_cap) in &[(1usize, 0usize), (0, 1), (7, 2), (0, 9)] {
                let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                diff(
                    "2",
                    &format!("fresh header ({elemsize},{addlen},{min_cap})"),
                    (header(ac).length, header(ac).capacity, header(ac).temp, header(ac).hash_table.is_null()),
                    (header(ar).length, header(ar).capacity, header(ar).temp, header(ar).hash_table.is_null()),
                );
                diff("2", "length must be 0", header(ac).length, 0usize);
                diff("2", "temp must be 0", header(ac).temp, 0isize);
                diff("2", "hash_table must be NULL", header(ac).hash_table.is_null(), true);
                (p.c.arrfreef)(ac);
                (p.r.arrfreef)(ar);
            }
        }
    }
}

#[test]
fn err03_arrgrowf_min_cap_clamped_to_four() {
    let _g = guard();
    let p = pair();
    unsafe {
        for &(addlen, min_cap, expect) in &[
            (0usize, 1usize, 4usize),
            (1, 0, 4),
            (2, 0, 4),
            (3, 0, 4),
            (0, 3, 4),
            (4, 0, 4),
            (5, 0, 5),
            (0, 7, 7),
        ] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), 8, addlen, min_cap);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), 8, addlen, min_cap);
            diff("3", &format!("clamp ({addlen},{min_cap})"), header(ac).capacity, header(ar).capacity);
            diff("3", &format!("clamp value ({addlen},{min_cap})"), header(ac).capacity, expect);
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

// ===========================================================================
// Rows 4-5 — stbds_hmfree_func rejection branches
// ===========================================================================

#[test]
fn err04_hmfree_null() {
    let _g = guard();
    let p = pair();
    unsafe {
        for &elemsize in &[0usize, 1, 8, 4096] {
            // must be a silent no-op in both
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
    // reaching here at all is the assertion
}

#[test]
fn err05_hmfree_no_hash_table() {
    let _g = guard();
    let p = pair();
    unsafe {
        for &elemsize in &[1usize, 8, 16] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 3, 0);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 3, 0);
            diff("5", "hash_table is NULL (C)", header(ac).hash_table.is_null(), true);
            diff("5", "hash_table is NULL (R)", header(ar).hash_table.is_null(), true);
            (p.c.hmfree_func)(ac, elemsize);
            (p.r.hmfree_func)(ar, elemsize);
        }
    }
}

// ===========================================================================
// Rows 6-11 — lookup miss sentinels
// ===========================================================================

#[test]
fn err06_hmget_key_ts_on_null_map() {
    let _g = guard();
    let p = pair();
    unsafe {
        for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (24, 8), (1, 1)] {
            setup(&p, DEFAULT_SEED);
            let mut key = vec![0x11u8; keysize.max(1)];
            let mut tc: isize = 0x1234;
            let mut tr: isize = 0x1234;
            let ac = (p.c.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                &mut tc,
                STBDS_HM_BINARY,
            );
            let ar = (p.r.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                &mut tr,
                STBDS_HM_BINARY,
            );
            diff("6", "temp sentinel", tc, tr);
            diff("6", "temp sentinel is -1", tc, -1isize);
            diff("6", "returned non-NULL", ac.is_null(), ar.is_null());
            diff("6", "returned non-NULL", ac.is_null(), false);
            diff("6", "snapshot", snapshot(ac, elemsize), snapshot(ar, elemsize));
            diff("6", "no hash table yet", snapshot(ac, elemsize).has_table, false);
            diff("6", "length == 1", snapshot(ac, elemsize).length, 1usize);
            diff("6", "element zeroed", elems(ac, elemsize), vec![0u8; elemsize]);
            diff("6", "element zeroed", elems(ar, elemsize), vec![0u8; elemsize]);
            (p.c.hmfree_func)(to_arr(ac, elemsize), elemsize);
            (p.r.hmfree_func)(to_arr(ar, elemsize), elemsize);
        }
    }
}

#[test]
fn err07_hmget_key_ts_no_hash_table() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    mc.put_default(&(-9i32).to_ne_bytes());
    mr.put_default(&(-9i32).to_ne_bytes());
    diff("7", "no table after hmput_default", mc.snap().has_table, false);
    diff("7", "no table after hmput_default", mr.snap().has_table, false);
    for k in -5i32..20 {
        let tc = mc.geti_ts(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
        let tr = mr.geti_ts(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
        diff("7", &format!("ts sentinel k={k}"), tc, tr);
        diff("7", &format!("ts sentinel is -1 k={k}"), tc, -1isize);
        diff("7", &format!("snap k={k}"), mc.snap(), mr.snap());
    }
    mc.free();
    mr.free();
}

#[test]
fn err08_09_10_hmget_key_ts_miss_both_probe_scans() {
    let _g = guard();
    let p = pair();
    // A heavily loaded table with long probe chains guarantees misses that
    // terminate in the first (`i = pos&MASK ..`) scan AND in the wrap scan.
    for &seed in &[DEFAULT_SEED, 0usize, 1, usize::MAX, 0xABCD1234] {
        setup(&p, seed);
        let (elemsize, keysize) = (8usize, 4usize);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        for k in 0i32..300 {
            mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
            mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
        }
        let mut rng = Rng::new(0xC0008u64.wrapping_add(seed as u64));
        for i in 0..3000 {
            let k = rng.next_u32() as i32;
            if (0..300).contains(&k) {
                continue;
            }
            let tc = mc.geti_ts(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            let tr = mr.geti_ts(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
            diff("8/9/10", &format!("miss#{i} k={k} seed={seed:#x}"), tc, tr);
            diff("8/9/10", &format!("miss#{i} sentinel is -1"), tc, -1isize);
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn err11_hmget_key_writes_sentinel_into_header_temp() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);

    // (a) a == NULL
    let tc = mc.geti(&mut 3i32.to_ne_bytes(), STBDS_HM_BINARY);
    let tr = mr.geti(&mut 3i32.to_ne_bytes(), STBDS_HM_BINARY);
    diff("11", "temp on NULL map", tc, tr);
    diff("11", "temp on NULL map is -1", tc, -1isize);

    // (b) no hash table
    let tc = mc.geti(&mut 3i32.to_ne_bytes(), STBDS_HM_BINARY);
    let tr = mr.geti(&mut 3i32.to_ne_bytes(), STBDS_HM_BINARY);
    diff("11", "temp with no table", tc, tr);
    diff("11", "temp with no table is -1", tc, -1isize);

    // (c) populated, miss
    for k in 0i32..40 {
        mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
        mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
    }
    for k in 100i32..160 {
        let tc = mc.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
        let tr = mr.geti(&mut k.to_ne_bytes(), STBDS_HM_BINARY);
        diff("11", &format!("miss temp k={k}"), tc, tr);
        diff("11", &format!("miss temp is -1 k={k}"), tc, -1isize);
        diff("11", &format!("header temp k={k}"), mc.temp(), mr.temp());
    }
    mc.free();
    mr.free();
}

// ===========================================================================
// Rows 12-14 — stbds_hmdel_key rejection branches
// ===========================================================================

#[test]
fn err12_hmdel_null_map_returns_null() {
    let _g = guard();
    let p = pair();
    unsafe {
        for &(elemsize, keysize, keyoffset, mode) in &[
            (8usize, 4usize, 0usize, 0i32),
            (16, 8, 0, 1),
            (16, 8, 8, 1),
            (8, 4, 0, -5),
            (8, 4, 0, 2),
            (8, 4, 0, i32::MAX),
        ] {
            let mut key = vec![b'k'; keysize.max(1) + 8];
            *key.last_mut().unwrap() = 0;
            let rc = (p.c.hmdel_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
            let rr = (p.r.hmdel_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                keyoffset,
                mode,
            );
            diff("12", &format!("NULL return (mode={mode})"), rc, rr);
            diff("12", &format!("must be NULL (mode={mode})"), rc.is_null(), true);
        }
    }
}

#[test]
fn err13_hmdel_no_hash_table() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    mc.put_default(&(-2i32).to_ne_bytes());
    mr.put_default(&(-2i32).to_ne_bytes());
    let tc_before = mc.t;
    let tr_before = mr.t;
    for k in 0i32..10 {
        let dc = mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
        let dr = mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
        diff("13", &format!("temp for k={k}"), dc, dr);
        diff("13", &format!("temp must be 0 for k={k}"), dc, 0isize);
        diff("13", "pointer unchanged", mc.t == tc_before, mr.t == tr_before);
        diff("13", "pointer unchanged (C)", mc.t == tc_before, true);
        diff("13", &format!("snap k={k}"), mc.snap(), mr.snap());
    }
    mc.free();
    mr.free();
}

#[test]
fn err14_hmdel_missing_key() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    for &seed in &[DEFAULT_SEED, 0usize, 7] {
        setup(&p, seed);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        for k in 0i32..64 {
            mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
            mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
        }
        let before_c = mc.snap();
        let before_r = mr.snap();
        for k in 1000i32..1200 {
            let dc = mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
            let dr = mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
            diff("14", &format!("miss temp k={k} seed={seed:#x}"), dc, dr);
            diff("14", &format!("miss temp is 0 k={k}"), dc, 0isize);
        }
        // length / used_count / tombstone_count must be untouched
        let after_c = mc.snap();
        let after_r = mr.snap();
        diff("14", "C state untouched by misses", (before_c.length, before_c.used_count, before_c.tombstone_count, before_c.slot_count), (after_c.length, after_c.used_count, after_c.tombstone_count, after_c.slot_count));
        diff("14", "RUST state untouched by misses", (before_r.length, before_r.used_count, before_r.tombstone_count, before_r.slot_count), (after_r.length, after_r.used_count, after_r.tombstone_count, after_r.slot_count));
        diff("14", "states agree", after_c, after_r);
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Row 15 — `mode == STBDS_HM_STRING` (exact) gates the strdup free
// ===========================================================================

#[test]
fn err15_strdup_free_only_for_mode_exactly_one() {
    let _g = guard();
    let p = pair();
    const ES: usize = 16;
    const KS: usize = 8;
    // Delete the LAST element so `old_index == final_index` and the re-find
    // (which would abort for mode >= 2) is skipped.
    for &mode in &[1i32, 2, 7, 1000] {
        setup(&p, DEFAULT_SEED);
        let mut keys: Vec<Vec<u8>> = (0..6)
            .map(|i| {
                let mut v = cstring(format!("strdupkey_{i}").as_bytes());
                v.resize(v.len().max(KS + 1), 0);
                v
            })
            .collect();
        let mut mc = Map::new(&p.c, ES, KS);
        let mut mr = Map::new(&p.r, ES, KS);
        mc.shmode(SH_STRDUP);
        mr.shmode(SH_STRDUP);
        for (i, k) in keys.iter_mut().enumerate() {
            let kp = k.as_mut_ptr() as *mut c_char;
            let v = (i as u64).to_ne_bytes();
            mc.sput(kp, &v, mode);
            mr.sput(kp, &v, mode);
        }
        for i in (0..keys.len()).rev() {
            let kp = keys[i].as_mut_ptr() as *mut c_char;
            let dc = mc.sdel(kp, mode, 0);
            let dr = mr.sdel(kp, mode, 0);
            diff("15", &format!("sdel#{i} mode={mode}"), dc, dr);
            diff("15", &format!("sdel#{i} snap mode={mode}"), mc.snap(), mr.snap());
            diff(
                "15",
                &format!("sdel#{i} elems(masked) mode={mode}"),
                mc.elem_bytes_masked(KS),
                mr.elem_bytes_masked(KS),
            );
        }
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Row 19 — `used_count >= 0` is vacuous for size_t; must never abort
// ===========================================================================

#[test]
fn err19_used_count_never_underflows() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    for k in 0i32..20 {
        mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
        mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
    }
    // delete every key, then keep deleting long past exhaustion
    for round in 0..5 {
        for k in 0i32..20 {
            let dc = mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
            let dr = mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0);
            diff("19", &format!("del k={k} round={round}"), dc, dr);
            let sc = mc.snap();
            diff("19", &format!("snap k={k} round={round}"), sc.clone(), mr.snap());
            assert!(
                sc.used_count < 1 << 40,
                "row 19: used_count underflowed to {}",
                sc.used_count
            );
        }
    }
    mc.free();
    mr.free();
}

// ===========================================================================
// Row 20 — the capacity assert inside hmput_key must never fire
// ===========================================================================

#[test]
fn err20_hmput_capacity_assert_never_fires() {
    let _g = guard();
    let p = pair();
    for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (24, 8), (5, 1)] {
        setup(&p, DEFAULT_SEED);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        let n: i32 = if keysize == 1 { 100 } else { 800 };
        for k in 0..n {
            let kb = (k as i64).to_ne_bytes();
            let mut kk = vec![0u8; keysize];
            let n = keysize.min(8);
            kk[..n].copy_from_slice(&kb[..n]);
            let v = vec![(k & 0xFF) as u8; elemsize - keysize];
            let tc = mc.put(&mut kk.clone(), &v, STBDS_HM_BINARY);
            let tr = mr.put(&mut kk, &v, STBDS_HM_BINARY);
            diff("20", &format!("put k={k} es={elemsize}"), tc, tr);
            let sc = mc.snap();
            diff("20", &format!("snap k={k} es={elemsize}"), sc.clone(), mr.snap());
            assert!(sc.length <= sc.capacity, "row 20: length > capacity");
        }
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Rows 22 / 25 — stbds_stralloc assertions and the empty string
// ===========================================================================

#[test]
fn err22_25_stralloc_never_asserts() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xC0022);
    unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        // the empty string on a virgin arena (remaining == 0)
        let mut empty = cstring(b"");
        let pc = (p.c.stralloc)(&mut ac, empty.as_mut_ptr() as *mut c_char);
        let pr = (p.r.stralloc)(&mut ar, empty.as_mut_ptr() as *mut c_char);
        diff("25", "empty string content", cstr(pc), cstr(pr));
        diff("25", "empty string is empty", cstr(pc), Vec::<u8>::new());
        diff("25", "arena remaining", ac.remaining, ar.remaining);
        diff("25", "arena remaining == 511", ac.remaining, 511usize);
        diff("25", "arena block", ac.block, ar.block);
        // random lengths, including the exact block boundaries
        let mut lens: Vec<usize> = vec![0, 1, 2, 510, 511, 512, 513, 1023, 1024, 1025, 2047, 2048];
        for _ in 0..400 {
            lens.push(rng.below(3000));
        }
        for (i, len) in lens.into_iter().enumerate() {
            let mut s = cstring(&rng.ascii(len));
            let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            diff("22", &format!("content #{i} len={len}"), cstr(pc), cstr(pr));
            diff(
                "22",
                &format!("arena #{i} len={len}"),
                (ac.remaining, ac.block, !ac.storage.is_null()),
                (ar.remaining, ar.block, !ar.storage.is_null()),
            );
            assert!(ac.remaining < (1 << 21), "row 22: remaining underflowed");
        }
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

// ===========================================================================
// Row 23 — the oversized-string ("huge block") path
// ===========================================================================

#[test]
fn err23_stralloc_oversized_string() {
    let _g = guard();
    let p = pair();
    unsafe {
        // (a) huge string on a virgin arena -> becomes the head, remaining = 0
        for &len in &[512usize, 513, 1000, 5000, 1 << 20, (1 << 20) + 5] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let mut s = cstring(&vec![b'q'; len]);
            let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            diff("23", &format!("huge content len={len}"), cstr(pc).len(), cstr(pr).len());
            diff("23", &format!("huge content len={len}"), cstr(pc), cstr(pr));
            diff(
                "23",
                &format!("arena after huge len={len}"),
                (ac.remaining, ac.block, !ac.storage.is_null()),
                (ar.remaining, ar.block, !ar.storage.is_null()),
            );
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
        }
        // (b) huge string spliced BEHIND an existing head block
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        let mut small = cstring(b"small");
        (p.c.stralloc)(&mut ac, small.as_mut_ptr() as *mut c_char);
        (p.r.stralloc)(&mut ar, small.as_mut_ptr() as *mut c_char);
        for (i, &len) in [700usize, 2000, 9000, 40000].iter().enumerate() {
            let mut s = cstring(&vec![b'w'; len]);
            let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            diff("23", &format!("splice content #{i} len={len}"), cstr(pc), cstr(pr));
            diff(
                "23",
                &format!("splice arena #{i} len={len}"),
                (ac.remaining, ac.block, !ac.storage.is_null()),
                (ar.remaining, ar.block, !ar.storage.is_null()),
            );
        }
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
        diff("23", "reset after splices", (ac.remaining, ac.block, ac.mode), (ar.remaining, ar.block, ar.mode));
    }
}

// ===========================================================================
// Row 24 — blocksize saturation at STBDS_STRING_ARENA_BLOCKSIZE_MAX
// ===========================================================================

#[test]
fn err24_blocksize_saturation() {
    let _g = guard();
    let p = pair();
    unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        // each oversized string forces a new block and bumps `block` by one
        let mut s = cstring(&vec![b'x'; 300_000]);
        for i in 0..60 {
            let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            diff("24", &format!("content #{i}"), cstr(pc).len(), cstr(pr).len());
            diff("24", &format!("block #{i}"), ac.block, ar.block);
            diff("24", &format!("remaining #{i}"), ac.remaining, ar.remaining);
        }
        // 512 << (block>>1) must have stopped growing at 1<<20 -> block caps at 22
        diff("24", "saturated block value", ac.block, ar.block);
        assert_eq!(ac.block, 22, "row 24: expected block to saturate at 22");
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

// ===========================================================================
// Row 26 — strreset on an empty arena is a zeroing no-op
// ===========================================================================

#[test]
fn err26_strreset_empty_arena() {
    let _g = guard();
    let p = pair();
    unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        for i in 0..5 {
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
            diff(
                "26",
                &format!("reset #{i}"),
                (ac.remaining, ac.block, ac.mode, ac.storage.is_null()),
                (ar.remaining, ar.block, ar.mode, ar.storage.is_null()),
            );
            diff("26", "fully zeroed", (ac.remaining, ac.block, ac.mode, ac.storage.is_null()), (0usize, 0u8, 0u8, true));
        }
        // non-zero block/mode on an arena with no storage must also be cleared
        ac.block = 7;
        ac.mode = 3;
        ar.block = 7;
        ar.mode = 3;
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
        diff(
            "26",
            "block/mode cleared",
            (ac.remaining, ac.block, ac.mode),
            (ar.remaining, ar.block, ar.mode),
        );
        diff("26", "block/mode zero", (ac.block, ac.mode), (0u8, 0u8));
    }
}

// ===========================================================================
// Rows 28 / 29 — out-of-range `mode` enum values across every entry point
// ===========================================================================

#[test]
fn err28_out_of_range_mode_string_side_sentinels() {
    let _g = guard();
    let p = pair();
    const ES: usize = 16;
    const KS: usize = 8;
    for &mode in &[2i32, 7, 1000, i32::MAX, 0x7FFF_FFFE] {
        setup(&p, DEFAULT_SEED);
        let mut keys: Vec<Vec<u8>> = (0..12)
            .map(|i| {
                let mut v = cstring(format!("oormode_{i}_{mode}").as_bytes());
                v.resize(v.len().max(KS + 1), 0);
                v
            })
            .collect();
        let mut mc = Map::new(&p.c, ES, KS);
        let mut mr = Map::new(&p.r, ES, KS);
        // lookups on a NULL map
        let kp = keys[0].as_mut_ptr() as *mut c_char;
        diff("28", &format!("NULL-map sgeti mode={mode}"), mc.sgeti(kp, mode), mr.sgeti(kp, mode));
        diff("28", &format!("NULL-map sgeti is -1 mode={mode}"), mc.temp(), -1isize);
        // now insert
        for (i, k) in keys.iter_mut().enumerate() {
            let kp = k.as_mut_ptr() as *mut c_char;
            let v = (i as u64).to_ne_bytes();
            diff("28", &format!("sput#{i} mode={mode}"), mc.sput(kp, &v, mode), mr.sput(kp, &v, mode));
            diff("28", &format!("sput#{i} snap mode={mode}"), mc.snap(), mr.snap());
        }
        // string path was taken -> string.mode became SH_DEFAULT
        diff("28", &format!("string.mode mode={mode}"), mc.snap().string_mode, mr.snap().string_mode);
        diff("28", &format!("string.mode == SH_DEFAULT mode={mode}"), mc.snap().string_mode, 1u8);
        // misses
        for i in 0..10 {
            let mut miss = cstring(format!("absent_{i}_{mode}").as_bytes());
            miss.resize(miss.len().max(KS + 1), 0);
            let mp = miss.as_mut_ptr() as *mut c_char;
            diff("28", &format!("miss#{i} mode={mode}"), mc.sgeti(mp, mode), mr.sgeti(mp, mode));
            diff("28", &format!("miss#{i} is -1 mode={mode}"), mc.temp(), -1isize);
            diff("28", &format!("miss del#{i} mode={mode}"), mc.sdel(mp, mode, 0), mr.sdel(mp, mode, 0));
            diff("28", &format!("miss del#{i} temp 0 mode={mode}"), mc.temp(), 0isize);
        }
        // delete the final element repeatedly (safe for mode >= 2)
        for i in (0..keys.len()).rev() {
            let kp = keys[i].as_mut_ptr() as *mut c_char;
            diff("28", &format!("sdel#{i} mode={mode}"), mc.sdel(kp, mode, 0), mr.sdel(kp, mode, 0));
            diff("28", &format!("sdel#{i} snap mode={mode}"), mc.snap(), mr.snap());
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn err29_out_of_range_mode_binary_side_sentinels() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    for &mode in &[-1i32, -2, -1000, i32::MIN, i32::MIN + 1] {
        setup(&p, DEFAULT_SEED);
        let mut mc = Map::new(&p.c, elemsize, keysize);
        let mut mr = Map::new(&p.r, elemsize, keysize);
        // NULL-map lookup / delete
        diff("29", &format!("NULL geti mode={mode}"), mc.geti(&mut 1i32.to_ne_bytes(), mode), mr.geti(&mut 1i32.to_ne_bytes(), mode));
        diff("29", &format!("NULL geti is -1 mode={mode}"), mc.temp(), -1isize);
        for k in 0i32..40 {
            diff("29", &format!("put({k}) mode={mode}"), mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), mode), mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), mode));
        }
        diff("29", &format!("string.mode == 0 mode={mode}"), mc.snap().string_mode, 0u8);
        diff("29", &format!("string.mode agree mode={mode}"), mc.snap().string_mode, mr.snap().string_mode);
        for k in 500i32..560 {
            diff("29", &format!("miss geti({k}) mode={mode}"), mc.geti(&mut k.to_ne_bytes(), mode), mr.geti(&mut k.to_ne_bytes(), mode));
            diff("29", &format!("miss is -1 ({k})"), mc.temp(), -1isize);
            diff("29", &format!("miss del({k}) mode={mode}"), mc.del(&mut k.to_ne_bytes(), mode, 0), mr.del(&mut k.to_ne_bytes(), mode, 0));
            diff("29", &format!("miss del temp 0 ({k})"), mc.temp(), 0isize);
        }
        for k in 0i32..40 {
            diff("29", &format!("del({k}) mode={mode}"), mc.del(&mut k.to_ne_bytes(), mode, 0), mr.del(&mut k.to_ne_bytes(), mode, 0));
            diff("29", &format!("del snap({k}) mode={mode}"), mc.snap(), mr.snap());
        }
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Row 30 — out-of-range shmode_func values truncate through unsigned char
// ===========================================================================

#[test]
fn err30_shmode_func_out_of_range() {
    let _g = guard();
    let p = pair();
    const ES: usize = 16;
    const KS: usize = 8;
    for &mode in &[
        0i32, 1, 2, 3, 4, 5, 99, 254, 255, 256, 257, 511, 512, -1, -2, -256, 1000, i32::MIN,
        i32::MAX,
    ] {
        setup(&p, DEFAULT_SEED);
        let mut mc = Map::new(&p.c, ES, KS);
        let mut mr = Map::new(&p.r, ES, KS);
        mc.shmode(mode);
        mr.shmode(mode);
        diff("30", &format!("snapshot for shmode({mode})"), mc.snap(), mr.snap());
        diff(
            "30",
            &format!("string.mode for shmode({mode})"),
            mc.snap().string_mode,
            (mode as u32 & 0xFF) as u8,
        );
        // only 1/2/3 store a pointer; everything else memcpy's raw bytes, so a
        // single distinct insert per map is enough to observe the arm taken
        let mut key = cstring(b"shmode_probe_key");
        let kp = key.as_mut_ptr() as *mut c_char;
        let v = 42u64.to_ne_bytes();
        diff("30", &format!("sput mode={mode}"), mc.sput(kp, &v, STBDS_HM_STRING), mr.sput(kp, &v, STBDS_HM_STRING));
        let sm = mc.snap().string_mode;
        if sm == 1 || sm == 2 || sm == 3 {
            unsafe {
                diff(
                    "30",
                    &format!("stored key (ptr arm) shmode={mode}"),
                    key_string_at(mc.t, ES, 1),
                    key_string_at(mr.t, ES, 1),
                );
            }
        } else {
            // default: memcpy arm -> raw bytes are byte-identical
            diff("30", &format!("stored bytes (memcpy arm) shmode={mode}"), mc.elem_bytes(), mr.elem_bytes());
        }
        diff("30", &format!("snap after sput mode={mode}"), mc.snap(), mr.snap());
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Rows 31-33 — hash entry-point degenerate inputs
// ===========================================================================

#[test]
fn err31_hash_string_empty() {
    let _g = guard();
    let p = pair();
    let mut empty = cstring(b"");
    unsafe {
        for &seed in &[0usize, 1, 2, DEFAULT_SEED, usize::MAX, 1 << 63] {
            diff(
                "31",
                &format!("hash_string(\"\", {seed:#x})"),
                (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed),
                (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed),
            );
        }
    }
}

#[test]
fn err32_hash_bytes_zero_length() {
    let _g = guard();
    let p = pair();
    let mut buf = vec![0xAAu8; 64];
    unsafe {
        for &seed in &[0usize, 1, 2, DEFAULT_SEED, usize::MAX, 1 << 63] {
            let hc = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            let hr = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            diff("32", &format!("hash_bytes(len=0, {seed:#x})"), hc, hr);
            // len == 0 must not read the buffer at all: a NULL pointer works
            let nc = (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let nr = (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            diff("32", &format!("hash_bytes(NULL, 0, {seed:#x})"), nc, nr);
            diff("32", "NULL and buffer agree (C)", hc, nc);
            diff("32", "NULL and buffer agree (RUST)", hr, nr);
        }
    }
}

#[test]
fn err33_hash_bytes_tail_switch_fallthrough() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0xC0033);
    unsafe {
        for len in 0..=40usize {
            for iter in 0..60 {
                // deliberately set high bits in every tail byte position
                let mut buf: Vec<u8> = if iter < 20 {
                    vec![0xFFu8; len.max(1)]
                } else if iter < 40 {
                    (0..len.max(1)).map(|i| if i == 3 { 0xFF } else { 0x01 }).collect()
                } else {
                    rng.bytes(len.max(1))
                };
                for &seed in &[0usize, DEFAULT_SEED, usize::MAX] {
                    diff(
                        "33",
                        &format!("tail len={len} iter={iter} seed={seed:#x}"),
                        (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
                        (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 34-36 — stbds_hmput_default branches
// ===========================================================================

#[test]
fn err34_35_36_hmput_default_branches() {
    let _g = guard();
    let p = pair();
    for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (24, 8)] {
        setup(&p, DEFAULT_SEED);
        unsafe {
            // row 34: a == NULL
            let ac = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let ar = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
            diff("34", "not NULL", ac.is_null(), ar.is_null());
            diff("34", "not NULL", ac.is_null(), false);
            diff("34", "snapshot", snapshot(ac, elemsize), snapshot(ar, elemsize));
            diff("34", "length == 1", snapshot(ac, elemsize).length, 1usize);
            diff("34", "default slot zeroed", elems(ac, elemsize), vec![0u8; elemsize]);
            diff("34", "default slot zeroed", elems(ar, elemsize), vec![0u8; elemsize]);

            // row 36: idempotent
            let ac2 = (p.c.hmput_default)(ac, elemsize);
            let ar2 = (p.r.hmput_default)(ar, elemsize);
            diff("36", "same pointer back (C)", ac2 == ac, true);
            diff("36", "same pointer back (RUST)", ar2 == ar, true);
            diff("36", "snapshot unchanged", snapshot(ac2, elemsize), snapshot(ar2, elemsize));

            // row 35: length forced to 0 -> resurrect branch
            (*header_mut(to_arr(ac2, elemsize))).length = 0;
            (*header_mut(to_arr(ar2, elemsize))).length = 0;
            let ac3 = (p.c.hmput_default)(ac2, elemsize);
            let ar3 = (p.r.hmput_default)(ar2, elemsize);
            diff("35", "snapshot after resurrect", snapshot(ac3, elemsize), snapshot(ar3, elemsize));
            diff("35", "length back to 1", snapshot(ac3, elemsize).length, 1usize);
            diff("35", "elements after resurrect", elems(ac3, elemsize), elems(ar3, elemsize));
            let _ = keysize;
            (p.c.hmfree_func)(to_arr(ac3, elemsize), elemsize);
            (p.r.hmfree_func)(to_arr(ar3, elemsize), elemsize);
        }
    }
}

// ===========================================================================
// Rows 37 / 38 — hm_geti with non-positive and normal `num`
// ===========================================================================

#[test]
fn err37_38_hm_geti_non_positive_and_asserts() {
    let _g = guard();
    let p = pair();
    for &n in &[0i32, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
        setup(&p, DEFAULT_SEED);
        unsafe {
            (p.c.hm_geti)(n);
            (p.r.hm_geti)(n);
        }
        // observable: the number of hash tables created (seed advance)
        let mut mc = Map::new(&p.c, 8, 4);
        let mut mr = Map::new(&p.r, 8, 4);
        mc.put(&mut 1i32.to_ne_bytes(), &1i32.to_ne_bytes(), STBDS_HM_BINARY);
        mr.put(&mut 1i32.to_ne_bytes(), &1i32.to_ne_bytes(), STBDS_HM_BINARY);
        diff("37", &format!("seed after hm_geti({n})"), mc.snap().seed, mr.snap().seed);
        mc.free();
        mr.free();
    }
    // row 38: the internal assertions must hold for a broad range of `num`
    for n in 1i32..200 {
        setup(&p, DEFAULT_SEED);
        unsafe {
            (p.c.hm_geti)(n);
            (p.r.hm_geti)(n);
        }
        let mut mc = Map::new(&p.c, 8, 4);
        let mut mr = Map::new(&p.r, 8, 4);
        mc.put(&mut 1i32.to_ne_bytes(), &1i32.to_ne_bytes(), STBDS_HM_BINARY);
        mr.put(&mut 1i32.to_ne_bytes(), &1i32.to_ne_bytes(), STBDS_HM_BINARY);
        diff("38", &format!("seed after hm_geti({n})"), mc.snap().seed, mr.snap().seed);
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Row 39 — strkey extremes
// ===========================================================================

#[test]
fn err39_strkey_extremes() {
    let _g = guard();
    let p = pair();
    unsafe {
        for &n in &[
            0i32,
            -1,
            1,
            i32::MIN,
            i32::MAX,
            i32::MIN + 1,
            -999999999,
            999999999,
        ] {
            let a = cstr((p.c.strkey)(n));
            let b = cstr((p.r.strkey)(n));
            diff("39", &format!("strkey({n})"), a.clone(), b);
            diff("39", &format!("strkey({n}) text"), a, format!("test_{n}").into_bytes());
        }
    }
}

// ===========================================================================
// Rows 40-42 — hash-index (re)allocation branches
// ===========================================================================

#[test]
fn err40_41_42_table_grow_shrink_rebuild() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);

    // row 40: fresh table (slot_count 8) then doubling
    let mut grow_steps = Vec::new();
    for k in 0i32..1000 {
        diff("40", &format!("put({k})"), mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY), mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY));
        let sc = mc.snap();
        diff("40", &format!("snap put({k})"), sc.clone(), mr.snap());
        diff("40", &format!("threshold put({k})"), (sc.used_count_threshold, sc.tombstone_count_threshold, sc.used_count_shrink_threshold, sc.slot_count_log2), (mr.snap().used_count_threshold, mr.snap().tombstone_count_threshold, mr.snap().used_count_shrink_threshold, mr.snap().slot_count_log2));
        if grow_steps.last() != Some(&sc.slot_count) {
            grow_steps.push(sc.slot_count);
        }
    }
    assert_eq!(grow_steps[0], 8, "row 40: first table must have 8 slots");
    assert!(grow_steps.windows(2).all(|w| w[1] == w[0] * 2), "row 40: {grow_steps:?}");

    // rows 41/42: deleting drives both the shrink and rebuild branches
    let mut shrink_steps = vec![mc.snap().slot_count];
    let mut saw_rebuild = false;
    for k in 0i32..1000 {
        diff("41/42", &format!("del({k})"), mc.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0), mr.del(&mut k.to_ne_bytes(), STBDS_HM_BINARY, 0));
        let sc = mc.snap();
        diff("41/42", &format!("snap del({k})"), sc.clone(), mr.snap());
        let last = *shrink_steps.last().unwrap();
        if sc.slot_count != last {
            shrink_steps.push(sc.slot_count);
        } else if sc.tombstone_count == 0 && sc.used_count > 0 {
            saw_rebuild = true;
        }
    }
    assert!(shrink_steps.len() >= 4, "row 41: {shrink_steps:?}");
    assert!(saw_rebuild, "row 42: same-size rebuild branch never observed");
    mc.free();
    mr.free();
}

// ===========================================================================
// Row 43 — `if (hash < 2) hash += 2` must be applied identically
// ===========================================================================

#[test]
fn err43_hash_low_value_adjustment() {
    let _g = guard();
    let p = pair();
    let (elemsize, keysize) = (8usize, 4usize);
    setup(&p, DEFAULT_SEED);
    let mut mc = Map::new(&p.c, elemsize, keysize);
    let mut mr = Map::new(&p.r, elemsize, keysize);
    unsafe {
        let mut low_seen = 0usize;
        for k in 0i32..400 {
            mc.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
            mr.put(&mut k.to_ne_bytes(), &k.to_ne_bytes(), STBDS_HM_BINARY);
            // recompute the hash independently in BOTH libraries and check the
            // value actually stored in the bucket matches `hash<2 ? hash+2 : hash`
            let seed_c = mc.snap().seed;
            let seed_r = mr.snap().seed;
            diff("43", &format!("table seed k={k}"), seed_c, seed_r);
            let mut kb = k.to_ne_bytes();
            let hc = (p.c.hash_bytes)(kb.as_mut_ptr() as *mut c_void, keysize, seed_c);
            let hr = (p.r.hash_bytes)(kb.as_mut_ptr() as *mut c_void, keysize, seed_r);
            diff("43", &format!("independent hash k={k}"), hc, hr);
            let expect = if hc < 2 { hc + 2 } else { hc };
            if hc < 2 {
                low_seen += 1;
            }
            let idx = mc.temp();
            let sc = mc.snap();
            let sr = mr.snap();
            let found_c = sc.buckets.iter().any(|&(h, i)| h == expect && i == idx);
            let found_r = sr.buckets.iter().any(|&(h, i)| h == expect && i == idx);
            diff("43", &format!("stored hash present k={k}"), found_c, found_r);
            assert!(found_c, "row 43: C bucket does not hold {expect:#x} for index {idx}");
        }
        // brute-force search for a genuinely `< 2` hash (documented as
        // unreachable in practice: 2 / 2^64 per draw)
        let mut rng = Rng::new(0xC0043);
        for _ in 0..200_000 {
            let mut b = rng.bytes(8);
            let s = rng.next_u64() as usize;
            let a = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, s);
            let c = (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, s);
            diff("43", "brute-force hash agreement", a, c);
            if a < 2 {
                low_seen += 1;
            }
        }
        eprintln!("row 43: observed {low_seen} hash values < 2 (expected 0)");
    }
    mc.free();
    mr.free();
}

// ===========================================================================
// Generic FFI boundary checks: NULL pointers, zero/oversized lengths
// ===========================================================================

#[test]
fn generic_null_and_zero_length_boundaries() {
    let _g = guard();
    let p = pair();
    unsafe {
        // hmfree_func(NULL, *)
        (p.c.hmfree_func)(std::ptr::null_mut(), 0);
        (p.r.hmfree_func)(std::ptr::null_mut(), 0);
        // hmdel_key(NULL, ...) with a NULL key too (never dereferenced)
        diff(
            "generic",
            "hmdel_key(NULL, .., NULL key)",
            (p.c.hmdel_key)(std::ptr::null_mut(), 8, std::ptr::null_mut(), 4, 0, 0),
            (p.r.hmdel_key)(std::ptr::null_mut(), 8, std::ptr::null_mut(), 4, 0, 0),
        );
        // hash_bytes with len 0 and a NULL buffer
        for &seed in &[0usize, DEFAULT_SEED, usize::MAX] {
            diff(
                "generic",
                "hash_bytes(NULL,0)",
                (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed),
                (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed),
            );
        }
        // arrgrowf with elemsize 0 (degenerate but well-defined: header only)
        for &(addlen, min_cap) in &[(0usize, 1usize), (4, 0), (0, 8)] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            diff(
                "generic",
                &format!("arrgrowf elemsize=0 ({addlen},{min_cap})"),
                (header(ac).length, header(ac).capacity, header(ac).temp),
                (header(ar).length, header(ar).capacity, header(ar).temp),
            );
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
        // hmget_key_ts with a huge keysize but a matching buffer
        setup(&p, DEFAULT_SEED);
        let elemsize = 4096usize;
        let keysize = 4096usize;
        let mut big = vec![0x5Cu8; keysize];
        let mut tc: isize = 0;
        let mut tr: isize = 0;
        let ac = (p.c.hmget_key_ts)(std::ptr::null_mut(), elemsize, big.as_mut_ptr() as *mut c_void, keysize, &mut tc, 0);
        let ar = (p.r.hmget_key_ts)(std::ptr::null_mut(), elemsize, big.as_mut_ptr() as *mut c_void, keysize, &mut tr, 0);
        diff("generic", "oversized keysize temp", tc, tr);
        let bc = (p.c.hmput_key)(ac, elemsize, big.as_mut_ptr() as *mut c_void, keysize, 0);
        let br = (p.r.hmput_key)(ar, elemsize, big.as_mut_ptr() as *mut c_void, keysize, 0);
        diff("generic", "oversized snapshot", snapshot(bc, elemsize), snapshot(br, elemsize));
        diff("generic", "oversized elems", elems(bc, elemsize), elems(br, elemsize));
        (p.c.hmfree_func)(to_arr(bc, elemsize), elemsize);
        (p.r.hmfree_func)(to_arr(br, elemsize), elemsize);
    }
}
