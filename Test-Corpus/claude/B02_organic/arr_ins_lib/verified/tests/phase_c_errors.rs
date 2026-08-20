//! Phase C — one differential test per row of ERRORS.md (in-process rows).
//! Rows whose trigger kills the process live in `tests/phase_c_crashes.rs`.
mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

unsafe fn hdr(a: *mut c_void) -> Header {
    *(((a as *mut u8).sub(HEADER_SIZE)) as *mut Header)
}
unsafe fn map_hdr(hp: *mut c_void, elemsize: usize) -> Header {
    hdr((hp as *mut u8).sub(elemsize) as *mut c_void)
}
unsafe fn map_idx(hp: *mut c_void, elemsize: usize) -> Option<HashIndex> {
    let h = map_hdr(hp, elemsize);
    if h.hash_table.is_null() {
        None
    } else {
        Some(*(h.hash_table as *mut HashIndex))
    }
}

// ============================================================ row 1
#[test]
fn err_01_arrgrowf_nogrow_returns_same_pointer() {
    let s = session();
    for &es in [1usize, 4, 8, 16, 64].iter() {
        unsafe {
            // a == NULL, min_cap == 0 -> stays NULL, no allocation
            assert!((s.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0).is_null());
            assert!((s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, 0).is_null());

            let c = (s.c.arrgrowf)(std::ptr::null_mut(), es, 0, 16);
            let r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, 16);
            for min_cap in [0usize, 1, 8, 15, 16] {
                assert_eq!((s.c.arrgrowf)(c, es, 0, min_cap), c);
                assert_eq!((s.rust.arrgrowf)(r, es, 0, min_cap), r);
            }
            assert_same(
                "row1 no-grow",
                &dump_array(c, es, 0),
                &dump_array(r, es, 0),
            );
            (s.c.arrfreef)(c);
            (s.rust.arrfreef)(r);
        }
    }
}

// ============================================================ row 3
#[test]
fn err_03_arrgrowf_size_overflow_wraps() {
    let s = session();
    // `min_len = stbds_arrlen(a) + addlen` is size_t arithmetic. With a length
    // of 4 and addlen close to SIZE_MAX the sum WRAPS to a tiny value, which
    // then satisfies `min_cap <= stbds_arrcap(a)` and the function must return
    // the array unchanged. A saturating implementation would try to allocate
    // ~16 EiB instead.
    for &es in [1usize, 4, 8, 16].iter() {
        unsafe {
            let c = (s.c.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            let r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            (*(((c as *mut u8).sub(HEADER_SIZE)) as *mut Header)).length = 4;
            (*(((r as *mut u8).sub(HEADER_SIZE)) as *mut Header)).length = 4;
            // length(4) + addlen wraps to 3, 2, 1, 0 respectively
            for addlen in [
                usize::MAX,
                usize::MAX - 1,
                usize::MAX - 2,
                usize::MAX - 3,
            ] {
                let c2 = (s.c.arrgrowf)(c, es, addlen, 0);
                let r2 = (s.rust.arrgrowf)(r, es, addlen, 0);
                assert_eq!(
                    c2, c,
                    "C: min_len must wrap (es={} addlen={:#x})",
                    es, addlen
                );
                assert_eq!(
                    r2, r,
                    "RUST: min_len must wrap (es={} addlen={:#x})",
                    es, addlen
                );
                assert_same(
                    &format!("row3 wrap es={} addlen={:#x}", es, addlen),
                    &dump_array(c2, es, 0),
                    &dump_array(r2, es, 0),
                );
            }
            (s.c.arrfreef)(c);
            (s.rust.arrfreef)(r);
        }
    }
    // addlen == SIZE_MAX-4 with length 4 gives min_len == SIZE_MAX exactly (no
    // wrap of the sum), and then `elemsize*min_cap + 32` wraps to 24 for
    // elemsize 8, i.e. realloc SHRINKS the block below the header size. Only
    // `length` and `capacity` stay inside the new allocation, so only those two
    // fields may be compared.
    unsafe {
        let c = (s.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        let r = (s.rust.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        (*(((c as *mut u8).sub(HEADER_SIZE)) as *mut Header)).length = 4;
        (*(((r as *mut u8).sub(HEADER_SIZE)) as *mut Header)).length = 4;
        let c2 = (s.c.arrgrowf)(c, 8, usize::MAX - 4, 0);
        let r2 = (s.rust.arrgrowf)(r, 8, usize::MAX - 4, 0);
        assert!(!c2.is_null() && !r2.is_null());
        assert_eq!(hdr(c2).capacity, usize::MAX, "C capacity after product wrap");
        assert_eq!(hdr(r2).capacity, usize::MAX, "RUST capacity after product wrap");
        assert_eq!(hdr(c2).length, 4);
        assert_eq!(hdr(r2).length, 4);
        (s.c.arrfreef)(c2);
        (s.rust.arrfreef)(r2);
    }
    // `elemsize * min_cap + sizeof(header)` also wraps: pick the product so it
    // wraps to exactly `sizeof(stbds_array_header)`.
    for (es, addlen) in [(16usize, 1usize << 60), (8, 1usize << 61), (32, 1usize << 59)] {
        unsafe {
            let c = (s.c.arrgrowf)(std::ptr::null_mut(), es, addlen, 0);
            let r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, addlen, 0);
            assert!(!c.is_null() && !r.is_null());
            assert_eq!(hdr(c).capacity, addlen);
            assert_eq!(hdr(r).capacity, addlen);
            assert_same(
                &format!("row3 product wrap es={} addlen={:#x}", es, addlen),
                &dump_array(c, es, 0),
                &dump_array(r, es, 0),
            );
            (s.c.arrfreef)(c);
            (s.rust.arrfreef)(r);
        }
    }
}

// ============================================================ row 4
#[test]
fn err_04_arrgrowf_min_capacity_boundary() {
    let s = session();
    for &es in [1usize, 4, 8, 16].iter() {
        for min_cap in [0usize, 1, 2, 3, 4, 5] {
            unsafe {
                let c = (s.c.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
                let r = (s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
                if min_cap == 0 {
                    assert!(c.is_null() && r.is_null());
                    continue;
                }
                let want = if min_cap < 4 { 4 } else { min_cap };
                assert_eq!(hdr(c).capacity, want, "C es={} min_cap={}", es, min_cap);
                assert_eq!(hdr(r).capacity, want, "RUST es={} min_cap={}", es, min_cap);
                (s.c.arrfreef)(c);
                (s.rust.arrfreef)(r);
            }
        }
    }
}

// ============================================================ row 6
#[test]
fn err_06_make_hash_index_thresholds_never_assert() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 6);
    let lay = L_I2I;
    // grow through slot_count 8, 16, 32, ..., 1024 and check the invariant the
    // C STBDS_ASSERT enforces at every step.
    unsafe {
        let mut cp: *mut c_void = std::ptr::null_mut();
        let mut rp: *mut c_void = std::ptr::null_mut();
        let mut seen: Vec<usize> = Vec::new();
        for i in 0..600u32 {
            let key = rng.next_u32().to_ne_bytes();
            let val = i.to_ne_bytes();
            cp = map_put_binary(s.c, cp, lay, &key, &val, HM_BINARY);
            rp = map_put_binary(s.rust, rp, lay, &key, &val, HM_BINARY);
            let ci = map_idx(cp, lay.elemsize).unwrap();
            let ri = map_idx(rp, lay.elemsize).unwrap();
            assert_eq!(ci.slot_count, ri.slot_count);
            assert_eq!(ci.used_count_threshold, ri.used_count_threshold);
            assert_eq!(ci.tombstone_count_threshold, ri.tombstone_count_threshold);
            assert_eq!(
                ci.used_count_shrink_threshold,
                ri.used_count_shrink_threshold
            );
            assert!(
                ci.used_count_threshold + ci.tombstone_count_threshold < ci.slot_count,
                "STBDS_ASSERT invariant broken at slot_count={}",
                ci.slot_count
            );
            if !seen.contains(&ci.slot_count) {
                seen.push(ci.slot_count);
            }
        }
        assert!(
            seen.len() >= 7,
            "expected to walk many slot_counts, saw {:?}",
            seen
        );
        assert_eq!(seen[0], 8);
        map_free(s.c, cp, lay);
        map_free(s.rust, rp, lay);
    }
}

// ============================================================ rows 7/8/10
#[test]
fn err_07_08_10_find_slot_miss_and_binary_key_mismatch() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 7);
    // A keysize-1 map filled with all 256 possible keys maximises probe-chain
    // length, so misses terminate in both the upper scan and the wrap-around
    // (`limit`) scan, and `memcmp` mismatches are hit constantly.
    let lay = L_B1;
    unsafe {
        let mut cp: *mut c_void = std::ptr::null_mut();
        let mut rp: *mut c_void = std::ptr::null_mut();
        let mut present = [false; 256];
        for b in 0u8..=255 {
            if b % 3 == 0 {
                continue; // leave a third of the key space absent
            }
            present[b as usize] = true;
            let val = rng.bytes(lay.elemsize - 1);
            cp = map_put_binary(s.c, cp, lay, &[b], &val, HM_BINARY);
            rp = map_put_binary(s.rust, rp, lay, &[b], &val, HM_BINARY);
        }
        for b in 0u8..=255 {
            let mut kc = [b];
            let mut kr = [b];
            let (c2, ci) = map_geti(s.c, cp, lay, kc.as_mut_ptr() as *mut c_void, HM_BINARY);
            let (r2, ri) = map_geti(s.rust, rp, lay, kr.as_mut_ptr() as *mut c_void, HM_BINARY);
            cp = c2;
            rp = r2;
            assert_eq!(ci, ri, "key {} lookup index differs", b);
            if present[b as usize] {
                assert!(ci >= 0);
            } else {
                assert_eq!(ci, -1, "absent key {} must return the -1 sentinel", b);
            }
        }
        assert_same(
            "rows 7/8/10 final state",
            &dump_map(cp, DumpOpts::raw(lay.elemsize)),
            &dump_map(rp, DumpOpts::raw(lay.elemsize)),
        );
        map_free(s.c, cp, lay);
        map_free(s.rust, rp, lay);
    }
}

// ============================================================ row 9
#[test]
fn err_09_key_mismatch_string_mode() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 9);
    let lay = L_STR;
    let mut bufs: Vec<Box<[u8]>> = Vec::new();
    let mk = |t: &[u8], bufs: &mut Vec<Box<[u8]>>| -> *mut c_char {
        let mut v = t.to_vec();
        v.push(0);
        while v.len() < 32 {
            v.push(0);
        }
        let mut b = v.into_boxed_slice();
        let p = b.as_mut_ptr() as *mut c_char;
        bufs.push(b);
        p
    };
    unsafe {
        let mut cp = (s.c.shmode_func)(lay.elemsize, SH_STRDUP);
        let mut rp = (s.rust.shmode_func)(lay.elemsize, SH_STRDUP);
        // keys that share long prefixes so `strcmp` has to work for its answer
        let mut present: Vec<Vec<u8>> = Vec::new();
        for i in 0..120usize {
            let t = format!("prefix-common-{:04}", i).into_bytes();
            present.push(t.clone());
            let p = mk(&t, &mut bufs);
            let v = rng.bytes(lay.elemsize - 8);
            cp = map_put_string(s.c, cp, lay, p, &v, HM_STRING);
            rp = map_put_string(s.rust, rp, lay, p, &v, HM_STRING);
        }
        for i in 0..300usize {
            let t = format!("prefix-common-{:04}", i).into_bytes();
            let p = mk(&t, &mut bufs) as *mut c_void;
            let (c2, ci) = map_geti(s.c, cp, lay, p, HM_STRING);
            let (r2, ri) = map_geti(s.rust, rp, lay, p, HM_STRING);
            cp = c2;
            rp = r2;
            assert_eq!(ci, ri, "string lookup {} differs", i);
            if present.contains(&t) {
                assert!(ci >= 0);
            } else {
                assert_eq!(ci, -1, "absent string {:?} must return -1", t);
            }
        }
        // near-misses: one character off
        for i in 0..60usize {
            let t = format!("prefix-common-{:04}X", i).into_bytes();
            let p = mk(&t, &mut bufs) as *mut c_void;
            let (c2, ci) = map_geti(s.c, cp, lay, p, HM_STRING);
            let (r2, ri) = map_geti(s.rust, rp, lay, p, HM_STRING);
            cp = c2;
            rp = r2;
            assert_eq!(ci, ri);
            assert_eq!(ci, -1);
        }
        map_free(s.c, cp, lay);
        map_free(s.rust, rp, lay);
    }
}

// ============================================================ rows 11/12
#[test]
fn err_11_12_hmfree_null_and_no_hash_table() {
    let s = session();
    unsafe {
        // row 11 - NULL is a no-op for every elemsize
        for es in [0usize, 1, 8, 16, 1024, usize::MAX] {
            (s.c.hmfree_func)(std::ptr::null_mut(), es);
            (s.rust.hmfree_func)(std::ptr::null_mut(), es);
        }
        // row 12 - array with hash_table == NULL, built two different ways
        for es in [1usize, 8, 16, 40] {
            let c = (s.c.hmput_default)(std::ptr::null_mut(), es);
            let r = (s.rust.hmput_default)(std::ptr::null_mut(), es);
            assert!(map_hdr(c, es).hash_table.is_null());
            assert!(map_hdr(r, es).hash_table.is_null());
            (s.c.hmfree_func)((c as *mut u8).sub(es) as *mut c_void, es);
            (s.rust.hmfree_func)((r as *mut u8).sub(es) as *mut c_void, es);

            let mut key = vec![0u8; es.min(8)];
            let c = (s.c.hmget_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                key.len(),
                HM_BINARY,
            );
            let r = (s.rust.hmget_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                key.len(),
                HM_BINARY,
            );
            assert!(map_hdr(c, es).hash_table.is_null());
            assert!(map_hdr(r, es).hash_table.is_null());
            (s.c.hmfree_func)((c as *mut u8).sub(es) as *mut c_void, es);
            (s.rust.hmfree_func)((r as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// ============================================================ rows 13/14/15/16
#[test]
fn err_13_14_15_16_hmget_sentinels() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 13);
    for &lay in BINARY_LAYOUTS.iter() {
        let mut key = rng.bytes(lay.keysize);
        unsafe {
            // row 13: hmget_key_ts on a NULL array
            let mut ct: isize = 12345;
            let mut rt: isize = 12345;
            let c = (s.c.hmget_key_ts)(
                std::ptr::null_mut(),
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                &mut ct,
                HM_BINARY,
            );
            let r = (s.rust.hmget_key_ts)(
                std::ptr::null_mut(),
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                &mut rt,
                HM_BINARY,
            );
            assert_eq!(ct, -1, "C row13 *temp");
            assert_eq!(rt, -1, "RUST row13 *temp");
            assert!(!c.is_null() && !r.is_null());
            assert_same(
                "row13",
                &dump_map(c, DumpOpts::raw(lay.elemsize)),
                &dump_map(r, DumpOpts::raw(lay.elemsize)),
            );

            // row 14: same array, hash_table still NULL
            let mut ct2: isize = 999;
            let mut rt2: isize = 999;
            let c2 = (s.c.hmget_key_ts)(
                c,
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                &mut ct2,
                HM_BINARY,
            );
            let r2 = (s.rust.hmget_key_ts)(
                r,
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                &mut rt2,
                HM_BINARY,
            );
            assert_eq!(c2, c, "row14 must return `a` unchanged");
            assert_eq!(r2, r, "row14 must return `a` unchanged");
            assert_eq!(ct2, -1);
            assert_eq!(rt2, -1);
            // hmget_key_ts must NOT touch the header temp
            assert_eq!(map_hdr(c2, lay.elemsize).temp, 0);
            assert_eq!(map_hdr(r2, lay.elemsize).temp, 0);

            // row 16: hmget_key writes the sentinel into the header instead
            let c3 = (s.c.hmget_key)(
                c2,
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                HM_BINARY,
            );
            let r3 = (s.rust.hmget_key)(
                r2,
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                HM_BINARY,
            );
            assert_eq!(map_hdr(c3, lay.elemsize).temp, -1);
            assert_eq!(map_hdr(r3, lay.elemsize).temp, -1);
            map_free(s.c, c3, lay);
            map_free(s.rust, r3, lay);
        }

        // row 15: real table, key absent -> *temp == -1 and `a` returned
        unsafe {
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            let mut inserted: Vec<Vec<u8>> = Vec::new();
            for _ in 0..12 {
                let k = rng.bytes(lay.keysize);
                if inserted.contains(&k) {
                    continue;
                }
                inserted.push(k.clone());
                let v = rng.bytes(lay.elemsize - lay.keysize);
                cp = map_put_binary(s.c, cp, lay, &k, &v, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &k, &v, HM_BINARY);
            }
            for _ in 0..100 {
                let mut k = rng.bytes(lay.keysize);
                if inserted.contains(&k) {
                    continue;
                }
                let mut ct: isize = 7;
                let mut rt: isize = 7;
                let c = (s.c.hmget_key_ts)(
                    cp,
                    lay.elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    lay.keysize,
                    &mut ct,
                    HM_BINARY,
                );
                let r = (s.rust.hmget_key_ts)(
                    rp,
                    lay.elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    lay.keysize,
                    &mut rt,
                    HM_BINARY,
                );
                assert_eq!(c, cp);
                assert_eq!(r, rp);
                assert_eq!(ct, -1);
                assert_eq!(rt, -1);
            }
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }
}

// ============================================================ rows 17/18
#[test]
fn err_17_18_hmput_default_paths() {
    let s = session();
    for es in [1usize, 4, 8, 16, 40, 128] {
        unsafe {
            // row 17: NULL
            let c = (s.c.hmput_default)(std::ptr::null_mut(), es);
            let r = (s.rust.hmput_default)(std::ptr::null_mut(), es);
            assert_eq!(map_hdr(c, es).length, 1);
            assert_eq!(map_hdr(r, es).length, 1);
            assert_same(
                "row17",
                &dump_map(c, DumpOpts::raw(es)),
                &dump_map(r, DumpOpts::raw(es)),
            );
            (s.c.hmfree_func)((c as *mut u8).sub(es) as *mut c_void, es);
            (s.rust.hmfree_func)((r as *mut u8).sub(es) as *mut c_void, es);

            // row 18: length == 0 array
            let ca = (s.c.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            let ra = (s.rust.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            // poison the element region so the memset is observable
            std::ptr::write_bytes(ca as *mut u8, 0xAB, es);
            std::ptr::write_bytes(ra as *mut u8, 0xAB, es);
            let c = (s.c.hmput_default)((ca as *mut u8).add(es) as *mut c_void, es);
            let r = (s.rust.hmput_default)((ra as *mut u8).add(es) as *mut c_void, es);
            assert_eq!(map_hdr(c, es).length, 1);
            assert_eq!(map_hdr(r, es).length, 1);
            assert_same(
                "row18",
                &dump_map(c, DumpOpts::raw(es)),
                &dump_map(r, DumpOpts::raw(es)),
            );
            // element 0 must be all zeroes after the memset
            assert!(std::slice::from_raw_parts((c as *mut u8).sub(es), es)
                .iter()
                .all(|&b| b == 0));
            assert!(std::slice::from_raw_parts((r as *mut u8).sub(es), es)
                .iter()
                .all(|&b| b == 0));
            (s.c.hmfree_func)((c as *mut u8).sub(es) as *mut c_void, es);
            (s.rust.hmfree_func)((r as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// ============================================================ rows 19/20/21/22
#[test]
fn err_19_20_21_22_hmput_key_allocation_and_growth() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 19);
    let lay = L_I2I;
    unsafe {
        // row 19/20: a == NULL -> default slot + first index (slot_count 8)
        let mut key = 42u32.to_ne_bytes();
        let cp = (s.c.hmput_key)(
            std::ptr::null_mut(),
            lay.elemsize,
            key.as_mut_ptr() as *mut c_void,
            lay.keysize,
            HM_BINARY,
        );
        let rp = (s.rust.hmput_key)(
            std::ptr::null_mut(),
            lay.elemsize,
            key.as_mut_ptr() as *mut c_void,
            lay.keysize,
            HM_BINARY,
        );
        let ci = map_idx(cp, lay.elemsize).unwrap();
        let ri = map_idx(rp, lay.elemsize).unwrap();
        assert_eq!(ci.slot_count, 8);
        assert_eq!(ri.slot_count, 8);
        assert_eq!(ci.string.mode, 0, "binary mode -> string.mode 0");
        assert_eq!(ri.string.mode, 0);
        assert_eq!(ci.used_count_shrink_threshold, 0, "slot_count 8 -> 0");
        assert_eq!(ri.used_count_shrink_threshold, 0);
        map_free(s.c, cp, lay);
        map_free(s.rust, rp, lay);

        // row 20 for string mode -> string.mode == STBDS_SH_DEFAULT
        let mut skey = *b"abc\0\0\0\0\0";
        let cp = (s.c.hmput_key)(
            std::ptr::null_mut(),
            lay.elemsize,
            skey.as_mut_ptr() as *mut c_void,
            lay.keysize,
            HM_STRING,
        );
        let rp = (s.rust.hmput_key)(
            std::ptr::null_mut(),
            lay.elemsize,
            skey.as_mut_ptr() as *mut c_void,
            lay.keysize,
            HM_STRING,
        );
        assert_eq!(map_idx(cp, lay.elemsize).unwrap().string.mode, SH_DEFAULT as u8);
        assert_eq!(map_idx(rp, lay.elemsize).unwrap().string.mode, SH_DEFAULT as u8);
        map_free(s.c, cp, lay);
        map_free(s.rust, rp, lay);

        // row 21: the grow happens exactly when used_count >= used_count_threshold
        // and the seed / string arena are INHERITED, not re-randomised.
        let mut cp: *mut c_void = std::ptr::null_mut();
        let mut rp: *mut c_void = std::ptr::null_mut();
        let mut first_seed: Option<usize> = None;
        let mut grows = 0usize;
        let mut prev_slots = 0usize;
        for i in 0..40u32 {
            let k = rng.next_u32().to_ne_bytes();
            let v = i.to_ne_bytes();
            cp = map_put_binary(s.c, cp, lay, &k, &v, HM_BINARY);
            rp = map_put_binary(s.rust, rp, lay, &k, &v, HM_BINARY);
            let ci = map_idx(cp, lay.elemsize).unwrap();
            let ri = map_idx(rp, lay.elemsize).unwrap();
            assert_eq!(ci.seed, ri.seed, "seed diverged at put {}", i);
            assert_eq!(ci.slot_count, ri.slot_count);
            assert_eq!(ci.used_count, ri.used_count);
            match first_seed {
                None => first_seed = Some(ci.seed),
                Some(sd) => assert_eq!(
                    ci.seed, sd,
                    "the seed must be inherited across grows (put {})",
                    i
                ),
            }
            if prev_slots != 0 && ci.slot_count != prev_slots {
                grows += 1;
                assert_eq!(ci.slot_count, prev_slots * 2);
            }
            prev_slots = ci.slot_count;
            // row 22: the capacity STBDS_ASSERT must hold at every step
            let h = map_hdr(cp, lay.elemsize);
            assert!(h.length <= h.capacity);
            let h = map_hdr(rp, lay.elemsize);
            assert!(h.length <= h.capacity);
        }
        assert!(grows >= 2, "expected at least two table grows, saw {}", grows);
        map_free(s.c, cp, lay);
        map_free(s.rust, rp, lay);
    }
}

// ============================================================ rows 23/24/25
#[test]
fn err_23_24_25_hmdel_sentinels() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 23);
    for &lay in BINARY_LAYOUTS.iter() {
        let mut key = rng.bytes(lay.keysize);
        unsafe {
            // row 23: a == NULL -> NULL
            for mode in [HM_BINARY, HM_STRING, -7, 5] {
                let c = (s.c.hmdel_key)(
                    std::ptr::null_mut(),
                    lay.elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    lay.keysize,
                    0,
                    mode,
                );
                let r = (s.rust.hmdel_key)(
                    std::ptr::null_mut(),
                    lay.elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    lay.keysize,
                    0,
                    mode,
                );
                assert!(c.is_null() && r.is_null(), "mode={}", mode);
            }
            // row 24: no hash table -> temp = 0, `a` returned unchanged
            let cp = (s.c.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            let rp = (s.rust.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            (*(((cp as *mut u8).sub(lay.elemsize + HEADER_SIZE)) as *mut Header)).temp = 77;
            (*(((rp as *mut u8).sub(lay.elemsize + HEADER_SIZE)) as *mut Header)).temp = 77;
            let c = (s.c.hmdel_key)(
                cp,
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                0,
                HM_BINARY,
            );
            let r = (s.rust.hmdel_key)(
                rp,
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                0,
                HM_BINARY,
            );
            assert_eq!(c, cp);
            assert_eq!(r, rp);
            assert_eq!(map_hdr(c, lay.elemsize).temp, 0, "row24 temp must be reset");
            assert_eq!(map_hdr(r, lay.elemsize).temp, 0);
            assert_eq!(map_hdr(c, lay.elemsize).length, 1, "length untouched");
            assert_eq!(map_hdr(r, lay.elemsize).length, 1);
            map_free(s.c, c, lay);
            map_free(s.rust, r, lay);
        }

        // row 25: key absent -> temp = 0, nothing changes
        unsafe {
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            let mut inserted: Vec<Vec<u8>> = Vec::new();
            for _ in 0..15 {
                let k = rng.bytes(lay.keysize);
                if inserted.contains(&k) {
                    continue;
                }
                inserted.push(k.clone());
                let v = rng.bytes(lay.elemsize - lay.keysize);
                cp = map_put_binary(s.c, cp, lay, &k, &v, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &k, &v, HM_BINARY);
            }
            let before_c = dump_map(cp, DumpOpts::raw(lay.elemsize));
            for _ in 0..80 {
                let mut k = rng.bytes(lay.keysize);
                if inserted.contains(&k) {
                    continue;
                }
                let (c, ct) = map_del(
                    s.c,
                    cp,
                    lay,
                    k.as_mut_ptr() as *mut c_void,
                    0,
                    HM_BINARY,
                );
                let (r, rt) = map_del(
                    s.rust,
                    rp,
                    lay,
                    k.as_mut_ptr() as *mut c_void,
                    0,
                    HM_BINARY,
                );
                assert_eq!(c, cp);
                assert_eq!(r, rp);
                assert_eq!(ct, 0);
                assert_eq!(rt, 0);
            }
            let mut after_c = dump_map(cp, DumpOpts::raw(lay.elemsize));
            // only `temp` may have changed (77 -> 0); normalise it away
            let norm = |s: &str| -> String {
                s.split(' ')
                    .filter(|t| !t.starts_with("temp="))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            after_c = norm(&after_c);
            assert_eq!(norm(&before_c), after_c, "absent delete changed the map");
            assert_same(
                "row25",
                &dump_map(cp, DumpOpts::raw(lay.elemsize)),
                &dump_map(rp, DumpOpts::raw(lay.elemsize)),
            );
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }
}

// ============================================================ rows 26/27/29
#[test]
fn err_26_27_29_hmdel_internal_asserts_never_fire() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 26);
    let lay = L_S1;
    unsafe {
        let mut cp: *mut c_void = std::ptr::null_mut();
        let mut rp: *mut c_void = std::ptr::null_mut();
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for _ in 0..200 {
            let k = rng.bytes(lay.keysize);
            if keys.contains(&k) {
                continue;
            }
            keys.push(k.clone());
            let v = rng.bytes(lay.elemsize - lay.keysize);
            cp = map_put_binary(s.c, cp, lay, &k, &v, HM_BINARY);
            rp = map_put_binary(s.rust, rp, lay, &k, &v, HM_BINARY);
        }
        // 1000 random delete/insert operations; if any of the three
        // STBDS_ASSERTs could fire, the process would die with SIGABRT.
        for _ in 0..1000 {
            let i = rng.below(keys.len());
            let mut k = keys[i].clone();
            if rng.below(2) == 0 {
                let (c, ct) = map_del(s.c, cp, lay, k.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                let (r, rt) =
                    map_del(s.rust, rp, lay, k.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                cp = c;
                rp = r;
                assert_eq!(ct, rt);
            } else {
                let v = rng.bytes(lay.elemsize - lay.keysize);
                cp = map_put_binary(s.c, cp, lay, &keys[i], &v, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &keys[i], &v, HM_BINARY);
            }
            let ci = map_idx(cp, lay.elemsize).unwrap();
            let ri = map_idx(rp, lay.elemsize).unwrap();
            // row 26: every slot index handed out is < slot_count
            for slot in 0..ci.slot_count {
                let b = &*ci.storage.add(slot >> BUCKET_SHIFT);
                let idx = b.index[slot & BUCKET_MASK];
                assert!(idx >= -2, "invalid index marker {}", idx);
            }
            // row 27: used_count is size_t, so `>= 0` is always true; the two
            // implementations must nevertheless agree on the value
            assert_eq!(ci.used_count, ri.used_count);
            assert_eq!(ci.tombstone_count, ri.tombstone_count);
        }
        assert_same(
            "rows 26/27/29",
            &dump_map(cp, DumpOpts::raw(lay.elemsize)),
            &dump_map(rp, DumpOpts::raw(lay.elemsize)),
        );
        map_free(s.c, cp, lay);
        map_free(s.rust, rp, lay);
    }
}

// ============================================================ rows 30/31/50/51
#[test]
fn err_30_31_50_51_shrink_rebuild_tombstone_boundaries() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 30);
    let lay = L_I2I;
    let mut shrinks = 0usize;
    let mut rebuilds = 0usize;
    let mut tombstone_reuses = 0usize;
    let mut no_compaction = 0usize;
    let mut with_compaction = 0usize;

    for trial in 0..8u64 {
        unsafe {
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for _ in 0..120 {
                let k = rng.next_u32().to_ne_bytes().to_vec();
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k.clone());
                let v = rng.next_u32().to_ne_bytes();
                cp = map_put_binary(s.c, cp, lay, &k, &v, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &k, &v, HM_BINARY);
            }
            let mut order: Vec<usize> = (0..keys.len()).collect();
            if trial % 3 == 1 {
                order.reverse();
            } else if trial % 3 == 2 {
                for i in (1..order.len()).rev() {
                    let j = rng.below(i + 1);
                    order.swap(i, j);
                }
            }
            for &i in order.iter() {
                let before_c = map_idx(cp, lay.elemsize).unwrap();
                let before_len = map_hdr(cp, lay.elemsize).length;
                // which element index does this key occupy?
                let mut k = keys[i].clone();
                let (c1, idx) = map_geti(s.c, cp, lay, k.as_mut_ptr() as *mut c_void, HM_BINARY);
                let (r1, ridx) =
                    map_geti(s.rust, rp, lay, k.as_mut_ptr() as *mut c_void, HM_BINARY);
                cp = c1;
                rp = r1;
                assert_eq!(idx, ridx);
                let final_index = before_len as isize - 2;
                if idx == final_index {
                    no_compaction += 1;
                } else if idx >= 0 {
                    with_compaction += 1;
                }

                let (c, ct) = map_del(s.c, cp, lay, k.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                let (r, rt) =
                    map_del(s.rust, rp, lay, k.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                cp = c;
                rp = r;
                assert_eq!(ct, rt);
                let after_c = map_idx(cp, lay.elemsize).unwrap();
                let after_r = map_idx(rp, lay.elemsize).unwrap();
                assert_eq!(after_c.slot_count, after_r.slot_count);
                assert_eq!(after_c.used_count, after_r.used_count);
                assert_eq!(after_c.tombstone_count, after_r.tombstone_count);
                if after_c.slot_count < before_c.slot_count {
                    shrinks += 1;
                    assert_eq!(after_c.slot_count, before_c.slot_count / 2);
                    assert!(after_c.slot_count >= BUCKET_LENGTH);
                } else if before_c.tombstone_count + 1 > before_c.tombstone_count_threshold
                    && after_c.tombstone_count == 0
                {
                    rebuilds += 1;
                }
                assert_same(
                    &format!("rows30/31 trial={} del key {}", trial, i),
                    &dump_map(cp, DumpOpts::raw(lay.elemsize)),
                    &dump_map(rp, DumpOpts::raw(lay.elemsize)),
                );
            }
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }

    // row 51: tombstone reuse on insert
    for trial in 0..80u64 {
        unsafe {
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for _ in 0..30 {
                let k = rng.next_u32().to_ne_bytes().to_vec();
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k.clone());
                let v = rng.next_u32().to_ne_bytes();
                cp = map_put_binary(s.c, cp, lay, &k, &v, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &k, &v, HM_BINARY);
            }
            for _ in 0..30 {
                let i = rng.below(keys.len());
                let mut k = keys[i].clone();
                let (c, _) = map_del(s.c, cp, lay, k.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                let (r, _) = map_del(s.rust, rp, lay, k.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                cp = c;
                rp = r;
                let tb_c = map_idx(cp, lay.elemsize).unwrap().tombstone_count;
                let tb_r = map_idx(rp, lay.elemsize).unwrap().tombstone_count;
                assert_eq!(tb_c, tb_r);
                if tb_c == 0 {
                    continue;
                }
                // insert a brand new key; if its probe hits the tombstone the
                // tombstone count goes down instead of staying put
                let nk = rng.next_u32().to_ne_bytes().to_vec();
                if keys.contains(&nk) {
                    continue;
                }
                keys.push(nk.clone());
                let v = rng.next_u32().to_ne_bytes();
                cp = map_put_binary(s.c, cp, lay, &nk, &v, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &nk, &v, HM_BINARY);
                let ac = map_idx(cp, lay.elemsize).unwrap();
                let ar = map_idx(rp, lay.elemsize).unwrap();
                assert_eq!(ac.tombstone_count, ar.tombstone_count);
                assert_eq!(ac.used_count, ar.used_count);
                if ac.tombstone_count < tb_c && ac.slot_count == map_idx(cp, lay.elemsize).unwrap().slot_count
                {
                    tombstone_reuses += 1;
                }
                assert_same(
                    &format!("row51 trial={}", trial),
                    &dump_map(cp, DumpOpts::raw(lay.elemsize)),
                    &dump_map(rp, DumpOpts::raw(lay.elemsize)),
                );
            }
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }

    eprintln!(
        "branch coverage: shrinks={} rebuilds={} tombstone_reuses={} no_compaction={} with_compaction={}",
        shrinks, rebuilds, tombstone_reuses, no_compaction, with_compaction
    );
    assert!(shrinks > 0, "the shrink branch (row 30) was never reached");
    assert!(rebuilds > 0, "the rebuild branch (row 31) was never reached");
    assert!(
        tombstone_reuses > 0,
        "the tombstone-reuse branch (row 51) was never reached"
    );
    assert!(
        no_compaction > 0,
        "the old_index == final_index branch (row 50) was never reached"
    );
    assert!(
        with_compaction > 0,
        "the compaction branch (row 50 complement) was never reached"
    );
}

// ============================================================ row 32
#[test]
fn err_32_hmdel_strdup_free_branch() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 32);
    let lay = L_STR;
    let mut bufs: Vec<Box<[u8]>> = Vec::new();
    unsafe {
        for (shm, mode) in [
            (SH_STRDUP, HM_STRING), // frees the dup
            (SH_STRDUP, 2),         // mode != STBDS_HM_STRING -> does NOT free
            (SH_ARENA, HM_STRING),  // arena keys are never freed here
            (SH_DEFAULT, HM_STRING),
        ] {
            let mut cp = (s.c.shmode_func)(lay.elemsize, shm);
            let mut rp = (s.rust.shmode_func)(lay.elemsize, shm);
            let mut ptrs: Vec<*mut c_char> = Vec::new();
            for i in 0..20usize {
                let mut v = format!("strdup-key-{:03}", i).into_bytes();
                v.push(0);
                while v.len() < 32 {
                    v.push(0);
                }
                let mut b = v.into_boxed_slice();
                let p = b.as_mut_ptr() as *mut c_char;
                bufs.push(b);
                ptrs.push(p);
                let val = rng.bytes(lay.elemsize - 8);
                cp = map_put_string(s.c, cp, lay, p, &val, HM_STRING);
                rp = map_put_string(s.rust, rp, lay, p, &val, HM_STRING);
            }
            // delete in reverse insertion order: old_index == final_index, so
            // the compaction/re-find path (which is ill-defined for mode != 1)
            // is never taken and the only difference is the free branch.
            for &p in ptrs.iter().rev() {
                let (c, ct) = map_del(s.c, cp, lay, p as *mut c_void, 0, mode);
                let (r, rt) = map_del(s.rust, rp, lay, p as *mut c_void, 0, mode);
                cp = c;
                rp = r;
                assert_eq!(ct, rt, "shm={} mode={}", shm, mode);
                assert_same(
                    &format!("row32 shm={} mode={}", shm, mode),
                    &dump_map(cp, DumpOpts::strptr(lay.elemsize)),
                    &dump_map(rp, DumpOpts::strptr(lay.elemsize)),
                );
            }
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }
}

// ============================================================ rows 34/35/36/38
#[test]
fn err_34_35_36_38_arena_branches() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 34);
    unsafe {
        let mut ca = Box::new(Arena::zeroed());
        let mut ra = Box::new(Arena::zeroed());
        let alloc = |ca: &mut Arena, ra: &mut Arena, text: &[u8]| -> (usize, u8, usize, u8) {
            let mut buf = text.to_vec();
            buf.push(0);
            let cp = (s.c.stralloc)(ca, buf.as_mut_ptr() as *mut c_char);
            let rp = (s.rust.stralloc)(ra, buf.as_mut_ptr() as *mut c_char);
            assert_eq!(
                std::ffi::CStr::from_ptr(cp).to_bytes(),
                text,
                "C stralloc content"
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(rp).to_bytes(),
                text,
                "RUST stralloc content"
            );
            assert_eq!(ca.remaining, ra.remaining, "remaining diverged");
            assert_eq!(ca.block, ra.block, "block diverged");
            (ca.remaining, ca.block, ra.remaining, ra.block)
        };

        // row 34: fast path, block counter untouched
        alloc(&mut ca, &mut ra, b"first");
        let (rem0, blk0, _, _) = (ca.remaining, ca.block, 0usize, 0u8);
        for _ in 0..20 {
            alloc(&mut ca, &mut ra, b"tiny");
        }
        assert_eq!(ca.block, blk0, "fast path must not bump `block`");
        assert!(ca.remaining < rem0);

        // row 36: oversized string spliced after the head
        let big = vec![b'B'; 5000];
        let rem_before = ca.remaining;
        alloc(&mut ca, &mut ra, &big);
        assert_eq!(
            ca.remaining, rem_before,
            "oversized-after-head must not touch remaining"
        );
        assert_eq!(ra.remaining, rem_before);

        (s.c.strreset)(&mut *ca);
        (s.rust.strreset)(&mut *ra);

        // row 36 (second shape): oversized on an empty arena becomes the head
        alloc(&mut ca, &mut ra, &big);
        assert_eq!(ca.remaining, 0);
        assert_eq!(ra.remaining, 0);

        (s.c.strreset)(&mut *ca);
        (s.rust.strreset)(&mut *ra);

        // row 35: the block counter saturates at 22 (512 << 11 == 1 MiB == MAX)
        for i in 0..22usize {
            let bs = 512usize << (ca.block as usize >> 1);
            let t = vec![b'a' + (i % 26) as u8; bs - 1];
            alloc(&mut ca, &mut ra, &t);
        }
        assert_eq!(ca.block, 22);
        assert_eq!(ra.block, 22);
        for _ in 0..3 {
            let t = vec![b'z'; (1 << 20) - 1];
            alloc(&mut ca, &mut ra, &t);
            assert_eq!(ca.block, 22, "block must stay at 22");
            assert_eq!(ra.block, 22);
        }

        // row 38: strreset on a populated and then on an empty arena
        (s.c.strreset)(&mut *ca);
        (s.rust.strreset)(&mut *ra);
        assert_same("row38 after reset", &dump_arena(&*ca), &dump_arena(&*ra));
        assert!(ca.storage.is_null() && ca.remaining == 0 && ca.block == 0 && ca.mode == 0);
        for _ in 0..5 {
            (s.c.strreset)(&mut *ca);
            (s.rust.strreset)(&mut *ra);
            assert_same("row38 idempotent", &dump_arena(&*ca), &dump_arena(&*ra));
        }
        // `mode` must survive stralloc but be cleared by strreset
        ca.mode = 3;
        ra.mode = 3;
        alloc(&mut ca, &mut ra, b"mode-check");
        assert_eq!(ca.mode, 3);
        assert_eq!(ra.mode, 3);
        (s.c.strreset)(&mut *ca);
        (s.rust.strreset)(&mut *ra);
        assert_eq!(ca.mode, 0);
        assert_eq!(ra.mode, 0);
        let _ = rng.next_u64();
    }
}

// ============================================================ row 37
#[test]
fn err_37_stralloc_shift_out_of_range() {
    let s = session();
    // `a->block` is an unsigned char, so `blocksize >> 1` reaches 127. Values
    // 110..=127 make `512 << (block>>1)` overflow to 0 (well defined unsigned
    // wrap-around in C); values >= 128 shift by >= 64, which x86-64 `shl`
    // resolves by masking the count to 6 bits.
    let mut blocks: Vec<u8> = (0u8..=27).collect();
    blocks.extend(110u8..=127);
    blocks.extend(128u8..=137);
    blocks.extend(246u8..=255);

    for &blk in blocks.iter() {
        unsafe {
            let mut ca = Box::new(Arena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: blk,
                mode: 0,
            });
            let mut ra = Box::new(Arena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: blk,
                mode: 0,
            });
            let mut buf = b"shift-probe\0".to_vec();
            let cp = (s.c.stralloc)(&mut *ca, buf.as_mut_ptr() as *mut c_char);
            let rp = (s.rust.stralloc)(&mut *ra, buf.as_mut_ptr() as *mut c_char);
            assert_eq!(
                std::ffi::CStr::from_ptr(cp).to_bytes(),
                b"shift-probe",
                "C content (block={})",
                blk
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(rp).to_bytes(),
                b"shift-probe",
                "RUST content (block={})",
                blk
            );
            assert_same(
                &format!("row37 block={}", blk),
                &dump_arena(&*ca),
                &dump_arena(&*ra),
            );
            (s.c.strreset)(&mut *ca);
            (s.rust.strreset)(&mut *ra);
        }
    }
}

// ============================================================ row 39
#[test]
fn err_39_hash_low_values_renormalised() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 39);
    // The `if (hash < 2) hash += 2` renormalisation guards the EMPTY(0) /
    // DELETED(1) markers. It can only trigger if siphash/hash_string returns 0
    // or 1, which never happens in practice; what CAN be proven is that both
    // libraries compute bit-identical hashes (so they would renormalise at the
    // same time) and that no marker value is ever produced.
    let mut zero_or_one = 0usize;
    for _ in 0..200_000 {
        let seed = rng.next_usize();
        let len = rng.below(17);
        let mut buf = rng.bytes(len.max(1));
        unsafe {
            let a = (s.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
            let b = (s.rust.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
            assert_eq!(a, b);
            if a < 2 {
                zero_or_one += 1;
            }
            let mut sbuf = rng.cstring(len);
            let a = (s.c.hash_string)(sbuf.as_mut_ptr() as *mut c_char, seed);
            let b = (s.rust.hash_string)(sbuf.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(a, b);
            if a < 2 {
                zero_or_one += 1;
            }
        }
    }
    eprintln!(
        "row39: {} of 400000 hashes landed on a marker value (expected 0)",
        zero_or_one
    );
}

// ============================================================ row 40
#[test]
fn err_40_shmode_out_of_range_enum() {
    let s = session();
    let lay = L_I2I;
    let modes: [c_int; 18] = [
        -1,
        -2,
        -128,
        -255,
        -256,
        -257,
        0,
        1,
        2,
        3,
        4,
        5,
        255,
        256,
        257,
        512,
        i32::MIN,
        i32::MAX,
    ];
    for &m in modes.iter() {
        unsafe {
            let c = (s.c.shmode_func)(lay.elemsize, m);
            let r = (s.rust.shmode_func)(lay.elemsize, m);
            let ci = map_idx(c, lay.elemsize).unwrap();
            let ri = map_idx(r, lay.elemsize).unwrap();
            let want = (m as u32 & 0xFF) as u8;
            assert_eq!(ci.string.mode, want, "C truncation of mode {}", m);
            assert_eq!(ri.string.mode, want, "RUST truncation of mode {}", m);
            assert_same(
                &format!("row40 shmode({})", m),
                &dump_map(c, DumpOpts::raw(lay.elemsize)),
                &dump_map(r, DumpOpts::raw(lay.elemsize)),
            );
            map_free(s.c, c, lay);
            map_free(s.rust, r, lay);
        }
    }
}

// ============================================================ rows 41/42
#[test]
fn err_41_42_out_of_range_hm_modes() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 41);
    let lay = L_I2I;
    // mode <= 0 is binary, mode >= 1 is string: verify the *selection* by
    // checking string.mode of the auto-created index.
    for &m in [
        i32::MIN,
        -1000,
        -2,
        -1,
        0,
        1,
        2,
        3,
        255,
        256,
        1000,
        i32::MAX,
    ]
    .iter()
    {
        unsafe {
            let mut key = [0u8; 16];
            key[0] = b'k';
            key[1] = b'0' + (m.unsigned_abs() % 10) as u8;
            let c = (s.c.hmput_key)(
                std::ptr::null_mut(),
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                m,
            );
            let r = (s.rust.hmput_key)(
                std::ptr::null_mut(),
                lay.elemsize,
                key.as_mut_ptr() as *mut c_void,
                lay.keysize,
                m,
            );
            let ci = map_idx(c, lay.elemsize).unwrap();
            let ri = map_idx(r, lay.elemsize).unwrap();
            let want = if m >= HM_STRING { SH_DEFAULT as u8 } else { 0u8 };
            assert_eq!(ci.string.mode, want, "C mode-selection for {}", m);
            assert_eq!(ri.string.mode, want, "RUST mode-selection for {}", m);
            map_free(s.c, c, lay);
            map_free(s.rust, r, lay);
        }
    }
    // row 42: `default:` memcpy branch, driven by every string.mode value that
    // is neither SH_DEFAULT, SH_STRDUP nor SH_ARENA
    for &shm in [0i32, 4, 5, 6, 99, 255, 256, -1].iter() {
        unsafe {
            let mut cp = (s.c.shmode_func)(lay.elemsize, shm);
            let mut rp = (s.rust.shmode_func)(lay.elemsize, shm);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for _ in 0..20 {
                let k = rng.next_u32().to_ne_bytes().to_vec();
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k.clone());
                let v = rng.next_u32().to_ne_bytes();
                cp = map_put_binary(s.c, cp, lay, &k, &v, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &k, &v, HM_BINARY);
            }
            assert_same(
                &format!("row42 shm={}", shm),
                &dump_map(cp, DumpOpts::raw(lay.elemsize)),
                &dump_map(rp, DumpOpts::raw(lay.elemsize)),
            );
            for k in keys.iter() {
                let mut kk = k.clone();
                let (c, ci) = map_geti(s.c, cp, lay, kk.as_mut_ptr() as *mut c_void, HM_BINARY);
                let (r, ri) = map_geti(s.rust, rp, lay, kk.as_mut_ptr() as *mut c_void, HM_BINARY);
                cp = c;
                rp = r;
                assert_eq!(ci, ri);
                assert!(ci >= 0, "memcpy'd key not findable (shm={})", shm);
            }
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }
}

// ============================================================ rows 43/44
#[test]
fn err_43_44_hash_bytes_boundaries() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 43);
    unsafe {
        // row 43: len == 0 with a NULL pointer (never dereferenced by the C)
        for seed in [0usize, 1, usize::MAX, 0x3141_5926] {
            let a = (s.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let b = (s.rust.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(a, b, "hash_bytes(NULL, 0, {:#x})", seed);
        }
        // row 44: every remainder for every block count 0..8
        for len in 0..=71usize {
            for seed in [0usize, 1, 2, usize::MAX, 0x0102_0304_0506_0708] {
                let mut buf = rng.bytes(len.max(1));
                let a = (s.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                let b = (s.rust.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                assert_eq!(a, b, "len={} seed={:#x}", len, seed);
                // and the same buffer with every byte high-bit set
                let mut hi: Vec<u8> = buf.iter().map(|b| b | 0x80).collect();
                let a = (s.c.hash_bytes)(hi.as_mut_ptr() as *mut c_void, len, seed);
                let b = (s.rust.hash_bytes)(hi.as_mut_ptr() as *mut c_void, len, seed);
                assert_eq!(a, b, "high-bit len={} seed={:#x}", len, seed);
            }
        }
    }
}

// ============================================================ rows 45/46
#[test]
fn err_45_46_hash_string_boundaries() {
    let s = session();
    unsafe {
        // row 45: empty string
        for seed in [0usize, 1, 2, usize::MAX, 0x3141_5926, 0x8000_0000_0000_0000] {
            let mut e = [0u8; 1];
            let a = (s.c.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            let b = (s.rust.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(a, b, "hash_string(\"\", {:#x})", seed);
        }
        // row 46: `(unsigned char) *str` on a platform where char is signed
        for byte in 1u8..=255 {
            for seed in [0usize, 1, usize::MAX, 0x3141_5926] {
                for reps in [1usize, 2, 7, 8, 9, 33] {
                    let mut buf = vec![byte; reps];
                    buf.push(0);
                    let a = (s.c.hash_string)(buf.as_mut_ptr() as *mut c_char, seed);
                    let b = (s.rust.hash_string)(buf.as_mut_ptr() as *mut c_char, seed);
                    assert_eq!(a, b, "byte={:#02x} reps={} seed={:#x}", byte, reps, seed);
                }
            }
        }
    }
}

// ============================================================ row 47
#[test]
fn err_47_strkey_int_extremes() {
    let s = session();
    unsafe {
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
            i32::MAX,
            i32::MAX - 1,
            i32::MIN,
            i32::MIN + 1,
            -2147483647,
            2147483647,
        ] {
            let cp = (s.c.strkey)(n);
            let rp = (s.rust.strkey)(n);
            let cb = std::slice::from_raw_parts(cp as *const u8, 256);
            let rb = std::slice::from_raw_parts(rp as *const u8, 256);
            assert_eq!(cb, rb, "strkey({}) full 256-byte buffer", n);
        }
        // long -> short transition leaves residue in the static buffer; it must
        // be identical in both libraries
        (s.c.strkey)(i32::MIN);
        (s.rust.strkey)(i32::MIN);
        let cp = (s.c.strkey)(7);
        let rp = (s.rust.strkey)(7);
        let cb = std::slice::from_raw_parts(cp as *const u8, 256);
        let rb = std::slice::from_raw_parts(rp as *const u8, 256);
        assert_eq!(cb, rb, "strkey residue after INT_MIN -> 7");
    }
}

// ============================================================ rows 48/49
#[test]
fn err_48_49_arr_ins_asserts_never_fire() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 48);
    unsafe {
        for n in [0i32, 1, 2, 3, 4, 5, -1, -4, i32::MAX, i32::MIN] {
            (s.c.arr_ins)(n);
            (s.rust.arr_ins)(n);
        }
        for _ in 0..5000 {
            let n = rng.next_u32() as i32;
            (s.c.arr_ins)(n);
            (s.rust.arr_ins)(n);
        }
    }
}

// ============================================================ row 52
#[test]
fn err_52_hmput_wrap_scan_skips_temp_key() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 52);
    let lay = L_STR;
    let mut stale_hits = 0usize;
    let mut fresh_hits = 0usize;

    // 20 keys => slot_count 32, used_count_threshold 24. `used_count` (20)
    // stays below the threshold, so the re-put phase can never trigger a table
    // grow. That matters because `stbds_make_hash_index` leaves `temp_key`
    // UNINITIALISED, and a wrap-around-scan match deliberately does not write
    // it — so after a grow the field would be genuine garbage in both
    // libraries and could not be compared.
    const N: usize = 20;

    for trial in 0..60u64 {
        let mut bufs: Vec<Box<[u8]>> = Vec::new();
        let mut ptrs: Vec<*mut c_char> = Vec::new();
        for i in 0..N {
            let mut v = format!("wrap-{}-{}", trial, i).into_bytes();
            v.push(0);
            while v.len() < 32 {
                v.push(0);
            }
            let mut b = v.into_boxed_slice();
            let p = b.as_mut_ptr() as *mut c_char;
            bufs.push(b);
            ptrs.push(p);
        }
        unsafe {
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            // inserts always end in the found_empty_slot branch, which DOES
            // write temp_key, so the field is well defined after every insert.
            for &p in ptrs.iter() {
                let v = rng.bytes(lay.elemsize - 8);
                cp = map_put_string(s.c, cp, lay, p, &v, HM_STRING);
                rp = map_put_string(s.rust, rp, lay, p, &v, HM_STRING);
                let ctk = map_idx(cp, lay.elemsize).unwrap().temp_key;
                let rtk = map_idx(rp, lay.elemsize).unwrap().temp_key;
                assert_eq!(
                    ctk, p,
                    "C: insert must set temp_key to the stored key pointer"
                );
                assert_eq!(
                    rtk, p,
                    "RUST: insert must set temp_key to the stored key pointer"
                );
            }
            let slots = map_idx(cp, lay.elemsize).unwrap().slot_count;
            assert_eq!(slots, 32, "expected slot_count 32 for {} keys", N);
            assert_eq!(map_idx(rp, lay.elemsize).unwrap().slot_count, 32);

            // re-put existing keys: hmput_key sets temp_key only when the match
            // is found in the *upper* scan; the wrap-around scan deliberately
            // leaves the previous value in place.
            for _ in 0..150 {
                let i = rng.below(ptrs.len());
                let p = ptrs[i];
                let v = rng.bytes(lay.elemsize - 8);
                cp = map_put_string(s.c, cp, lay, p, &v, HM_STRING);
                rp = map_put_string(s.rust, rp, lay, p, &v, HM_STRING);
                let ci = map_idx(cp, lay.elemsize).unwrap();
                let ri = map_idx(rp, lay.elemsize).unwrap();
                assert_eq!(ci.slot_count, slots, "unexpected grow during re-put");
                assert_eq!(ri.slot_count, slots);
                assert_eq!(
                    ci.temp_key as usize, ri.temp_key as usize,
                    "temp_key pointer diverged (trial={} key={})",
                    trial, i
                );
                if ci.temp_key == p {
                    fresh_hits += 1;
                } else {
                    stale_hits += 1;
                    // the stale value must still be one of the live key
                    // pointers - i.e. a real leftover, not garbage
                    assert!(
                        ptrs.contains(&ci.temp_key),
                        "stale temp_key is not a live key pointer"
                    );
                }
                assert_same(
                    &format!("row52 trial={} key={}", trial, i),
                    &dump_map(
                        cp,
                        DumpOpts::strptr(lay.elemsize)
                            .with_temp_key()
                            .with_ptr_identity(),
                    ),
                    &dump_map(
                        rp,
                        DumpOpts::strptr(lay.elemsize)
                            .with_temp_key()
                            .with_ptr_identity(),
                    ),
                );
            }
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }
    eprintln!(
        "row52: temp_key fresh={} stale={} (stale == the wrap-around scan branch)",
        fresh_hits, stale_hits
    );
    assert!(fresh_hits > 0, "the upper-scan temp_key branch never ran");
    assert!(
        stale_hits > 0,
        "the wrap-around scan branch (which skips temp_key) never ran"
    );
}
