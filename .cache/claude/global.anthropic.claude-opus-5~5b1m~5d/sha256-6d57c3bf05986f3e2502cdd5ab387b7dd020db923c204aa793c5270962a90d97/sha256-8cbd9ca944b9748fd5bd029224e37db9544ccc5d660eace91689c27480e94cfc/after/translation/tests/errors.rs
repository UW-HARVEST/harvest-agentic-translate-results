//! Phase C — one differential test per ERRORS.md row.  Each asserts the two
//! libraries return the *same* sentinel / error value, not merely that both
//! "failed somehow".
//!
//! Also covers the generic C-API boundaries: NULL pointers, zero lengths,
//! one-past-range values and out-of-range enum ints crossing the FFI boundary.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

const SEED_SET: [usize; 5] = [DEFAULT_SEED, 0, 1, usize::MAX, 0xdead_beef];

// ---------------------------------------------------------------------------
// rows 1–7 : stbds_arrgrowf rejection / clamping ladder
// ---------------------------------------------------------------------------

#[test]
fn err_rows1_7_arrgrowf() {
    let (p, _g) = libs();
    unsafe {
        // row 7 : nothing requested on a fresh array -> NULL from both
        for &es in [0usize, 1, 4, 8, 16, 40, 4096].iter() {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let b = (p.r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(a.is_null() && b.is_null(), "row7 es={es}: both must be NULL");
        }

        // row 4/5 : min_cap < 4 on a fresh array is raised to 4; fresh header
        for &mc in [1usize, 2, 3].iter() {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), 8, 0, mc);
            let b = (p.r.arrgrowf)(std::ptr::null_mut(), 8, 0, mc);
            assert!(!a.is_null() && !b.is_null());
            let (ha, hb) = (hdr_of(a), hdr_of(b));
            assert_eq!(ha.capacity, 4, "row4 C capacity for min_cap={mc}");
            assert_eq!(hb.capacity, 4, "row4 Rust capacity for min_cap={mc}");
            assert_eq!((ha.length, ha.temp), (0, 0), "row5 C fresh header");
            assert_eq!((hb.length, hb.temp), (0, 0), "row5 Rust fresh header");
            assert!(ha.hash_table.is_null() && hb.hash_table.is_null(), "row5 hash_table");
            (p.c.arrfreef)(a);
            (p.r.arrfreef)(b);
        }

        // row 2 : min_len wins over an under-sized min_cap
        for &(addlen, mc) in [(10usize, 1usize), (100, 4), (7, 5), (1000, 999)].iter() {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), 8, addlen, mc);
            let b = (p.r.arrgrowf)(std::ptr::null_mut(), 8, addlen, mc);
            assert_eq!(hdr_of(a).capacity, addlen.max(mc).max(4), "row2 C");
            assert_eq!(hdr_of(b).capacity, addlen.max(mc).max(4), "row2 Rust");
            (p.c.arrfreef)(a);
            (p.r.arrfreef)(b);
        }

        // rows 1/3 : early-out and doubling on an existing array
        let mut a = (p.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        let mut b = (p.r.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        for &mc in [0usize, 1, 2, 3, 4].iter() {
            // row 1 : min_cap <= cap -> the SAME pointer is returned unchanged
            let a2 = (p.c.arrgrowf)(a, 8, 0, mc);
            let b2 = (p.r.arrgrowf)(b, 8, 0, mc);
            assert_eq!(a2, a, "row1 C must return the identical pointer (mc={mc})");
            assert_eq!(b2, b, "row1 Rust must return the identical pointer (mc={mc})");
            assert_eq!(hdr_of(a2).capacity, 4);
            assert_eq!(hdr_of(b2).capacity, 4);
        }
        // row 3 : min_cap = cap+1 -> raised to 2*cap
        a = (p.c.arrgrowf)(a, 8, 0, 5);
        b = (p.r.arrgrowf)(b, 8, 0, 5);
        assert_eq!(hdr_of(a).capacity, 8, "row3 C doubling");
        assert_eq!(hdr_of(b).capacity, 8, "row3 Rust doubling");

        // row 6 : elemsize == 0
        let za = (p.c.arrgrowf)(std::ptr::null_mut(), 0, 0, 1);
        let zb = (p.r.arrgrowf)(std::ptr::null_mut(), 0, 0, 1);
        assert!(!za.is_null() && !zb.is_null(), "row6 both non-NULL");
        assert_eq!(hdr_of(za), hdr_of(zb), "row6 header must match exactly");
        (p.c.arrfreef)(za);
        (p.r.arrfreef)(zb);
        (p.c.arrfreef)(a);
        (p.r.arrfreef)(b);
    }
}

// ---------------------------------------------------------------------------
// rows 8–10, 63 : stbds_hmfree_func
// ---------------------------------------------------------------------------

#[test]
fn err_rows8_10_hmfree_func_null_and_tableless() {
    let (p, _g) = libs();
    unsafe {
        // row 8 : NULL is a no-op in both (must not crash)
        for &es in [0usize, 1, 8, 16, 40].iter() {
            (p.c.hmfree_func)(std::ptr::null_mut(), es);
            (p.r.hmfree_func)(std::ptr::null_mut(), es);
        }

        // rows 9/63 : array with hash_table == NULL
        for &es in [8usize, 16, 24].iter() {
            let a = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let b = (p.r.hmput_default)(std::ptr::null_mut(), es);
            assert!(hdr_of(map_arr(a, es)).hash_table.is_null(), "row9 C table NULL");
            assert!(hdr_of(map_arr(b, es)).hash_table.is_null(), "row9 Rust table NULL");
            (p.c.hmfree_func)(map_arr(a, es), es);
            (p.r.hmfree_func)(map_arr(b, es), es);
        }

        // row 10 : arena mode != SH_STRDUP must NOT free the caller's key
        // pointers.  If it did, the buffers below would be double-freed /
        // corrupted; read them back afterwards to prove they survive.
        for &shm in [SH_NONE, SH_DEFAULT, SH_ARENA].iter() {
            for lib in [&p.c, &p.r] {
                let es = 16usize;
                let mut map = (lib.shmode_func)(es, shm);
                let mut keep: Vec<Box<[u8]>> = Vec::new();
                for i in 0..5u8 {
                    let bx: Box<[u8]> = format!("key_{i}\0").into_bytes().into_boxed_slice();
                    let kp = bx.as_ptr() as *mut c_void;
                    keep.push(bx);
                    map = (lib.hmput_key)(map, es, kp, 8, HM_STRING);
                }
                (lib.hmfree_func)(map_arr(map, es), es);
                for (i, bx) in keep.iter().enumerate() {
                    assert_eq!(
                        &bx[..], format!("key_{i}\0").as_bytes(),
                        "{} shmode={shm}: caller key buffer must be untouched",
                        lib.name
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 11–15 : lookup misses and table-less lookups
// ---------------------------------------------------------------------------

#[test]
fn err_rows11_15_lookup_sentinels() {
    let (p, _g) = libs();
    unsafe {
        let es = 16usize;
        let mut key = [1u8, 2, 3, 4, 0, 0, 0, 0];
        for &gseed in SEED_SET.iter() {
            reseed(p, gseed);

            // row 11 : a == NULL -> allocates, *temp = -1, returns non-NULL
            let mut tc: isize = 0x1234;
            let mut tr: isize = 0x1234;
            let a = (p.c.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                &mut tc,
                HM_BINARY,
            );
            let b = (p.r.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                &mut tr,
                HM_BINARY,
            );
            assert_eq!(tc, -1, "row11 C *temp");
            assert_eq!(tr, -1, "row11 Rust *temp");
            assert!(!a.is_null() && !b.is_null(), "row11 both non-NULL");
            assert_eq!(
                hdr_of(map_arr(a, es)).length,
                hdr_of(map_arr(b, es)).length,
                "row11 length"
            );
            assert_eq!(hdr_of(map_arr(a, es)).length, 1, "row11 length must be 1");

            // row 12 : table == NULL -> *temp = -1, returns a unchanged
            let a2 = (p.c.hmget_key_ts)(a, es, key.as_mut_ptr() as *mut c_void, 4, &mut tc, HM_BINARY);
            let b2 = (p.r.hmget_key_ts)(b, es, key.as_mut_ptr() as *mut c_void, 4, &mut tr, HM_BINARY);
            assert_eq!(a2, a, "row12 C must return a unchanged");
            assert_eq!(b2, b, "row12 Rust must return a unchanged");
            assert_eq!((tc, tr), (-1, -1), "row12 *temp");

            // row 14 : hmget_key also stores it in the header's temp
            let a3 = (p.c.hmget_key)(a, es, key.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            let b3 = (p.r.hmget_key)(b, es, key.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            assert_eq!(
                hdr_of(map_arr(a3, es)).temp,
                hdr_of(map_arr(b3, es)).temp,
                "row14 header temp"
            );
            assert_eq!(hdr_of(map_arr(a3, es)).temp, -1, "row14 temp must be -1");
            (p.c.hmfree_func)(map_arr(a3, es), es);
            (p.r.hmfree_func)(map_arr(b3, es), es);

            // rows 13/15 : real table, key absent -> -1 (never an infinite probe)
            let mut mc = std::ptr::null_mut::<c_void>();
            let mut mr = std::ptr::null_mut::<c_void>();
            for i in 0..7u32 {
                let mut k = i.to_le_bytes();
                mc = (p.c.hmput_key)(mc, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
                mr = (p.r.hmput_key)(mr, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            }
            for i in 1000..1064u32 {
                let mut k = i.to_le_bytes();
                mc = (p.c.hmget_key)(mc, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
                mr = (p.r.hmget_key)(mr, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
                let (x, y) = (hdr_of(map_arr(mc, es)).temp, hdr_of(map_arr(mr, es)).temp);
                assert_eq!(x, y, "row13 temp for absent key {i}");
                assert_eq!(x, -1, "row13 absent key {i} must yield -1");
            }
            (p.c.hmfree_func)(map_arr(mc, es), es);
            (p.r.hmfree_func)(map_arr(mr, es), es);
        }
        reseed(p, DEFAULT_SEED);
    }
}

// ---------------------------------------------------------------------------
// rows 16–17 : the `if (hash < 2) hash += 2` guard
//
// `stbds_hash_string(str, seed)` first does `hash ^= seed`, which for the EMPTY
// string cancels the seed completely; the result is therefore `K + seed` for a
// seed-independent constant K.  So a seed can be computed that makes the hash
// of "" be exactly 0 or exactly 1 — the two values the guard exists for.
// ---------------------------------------------------------------------------

#[test]
fn err_rows16_17_hash_below_two_guard() {
    let (p, _g) = libs();
    unsafe {
        let mut empty = [0u8; 1];
        let k_c = (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, 0);
        let k_r = (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, 0);
        assert_eq!(k_c, k_r, "hash_string(\"\",0) must agree");

        for target in [0usize, 1usize] {
            let magic = target.wrapping_sub(k_c);
            // sanity: the crafted seed really does produce `target`
            assert_eq!(
                (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, magic),
                target,
                "crafted seed must give hash == {target} in C"
            );
            assert_eq!(
                (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, magic),
                target,
                "crafted seed must give hash == {target} in Rust"
            );

            let es = 16usize;
            reseed(p, magic);
            // shmode_func -> make_hash_index(8, NULL) -> table->seed == magic
            let mut mc = (p.c.shmode_func)(es, SH_DEFAULT);
            let mut mr = (p.r.shmode_func)(es, SH_DEFAULT);
            let tc = hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex;
            let tr = hdr_of(map_arr(mr, es)).hash_table as *mut HashIndex;
            assert_eq!((*tc).seed, magic, "C table seed");
            assert_eq!((*tr).seed, magic, "Rust table seed");

            // row 17 : hmput_key must bump the hash to 2 before probing
            mc = (p.c.hmput_key)(mc, es, empty.as_mut_ptr() as *mut c_void, 8, HM_STRING);
            mr = (p.r.hmput_key)(mr, es, empty.as_mut_ptr() as *mut c_void, 8, HM_STRING);
            let sc = idx_snap(hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex).unwrap();
            let sr = idx_snap(hdr_of(map_arr(mr, es)).hash_table as *mut HashIndex).unwrap();
            assert_eq!(sc.buckets[0].hash, sr.buckets[0].hash, "row17 bucket hash[]");
            assert_eq!(sc.buckets[0].index, sr.buckets[0].index, "row17 bucket index[]");
            // hash 0 -> 2 (slot 2), hash 1 -> 3 (slot 3)
            let bumped = target + 2;
            assert_eq!(
                sc.buckets[0].hash[bumped], bumped,
                "row17: hash {target} must be stored as {bumped} (C)"
            );
            assert_eq!(
                sr.buckets[0].hash[bumped], bumped,
                "row17: hash {target} must be stored as {bumped} (Rust)"
            );
            // and no slot may hold the reserved EMPTY(0) / DELETED(1) values
            assert!(
                !sc.buckets[0].hash.iter().enumerate().any(|(i, &h)| h == 1 && sc.buckets[0].index[i] >= 0),
                "row17: an in-use slot must never hold hash 1"
            );

            // row 16 : hm_find_slot must apply the same bump and find the key
            mc = (p.c.hmget_key)(mc, es, empty.as_mut_ptr() as *mut c_void, 8, HM_STRING);
            mr = (p.r.hmget_key)(mr, es, empty.as_mut_ptr() as *mut c_void, 8, HM_STRING);
            let (x, y) = (hdr_of(map_arr(mc, es)).temp, hdr_of(map_arr(mr, es)).temp);
            assert_eq!(x, y, "row16 temp");
            assert_eq!(x, 0, "row16: the empty-string key must be found at index 0");

            // and deleting it must work too
            mc = (p.c.hmdel_key)(mc, es, empty.as_mut_ptr() as *mut c_void, 8, 0, HM_STRING);
            mr = (p.r.hmdel_key)(mr, es, empty.as_mut_ptr() as *mut c_void, 8, 0, HM_STRING);
            assert_eq!(
                hdr_of(map_arr(mc, es)).temp,
                hdr_of(map_arr(mr, es)).temp,
                "row16 delete temp"
            );
            assert_eq!(hdr_of(map_arr(mc, es)).temp, 1, "row16 delete must succeed");
            (p.c.hmfree_func)(map_arr(mc, es), es);
            (p.r.hmfree_func)(map_arr(mr, es), es);
        }
        reseed(p, DEFAULT_SEED);
    }
}

// ---------------------------------------------------------------------------
// rows 18–22 : hmput_key bootstrap, threshold growth, duplicates, tombstones
// ---------------------------------------------------------------------------

#[test]
fn err_rows18_22_hmput_key_paths() {
    let (p, _g) = libs();
    unsafe {
        let es = 16usize;
        for &gseed in SEED_SET.iter() {
            reseed(p, gseed);
            let mut k0 = 0u32.to_le_bytes();
            // row 18/19 : a == NULL bootstraps; table gets 8 slots
            let mut mc = (p.c.hmput_key)(
                std::ptr::null_mut(),
                es,
                k0.as_mut_ptr() as *mut c_void,
                4,
                HM_BINARY,
            );
            let mut mr = (p.r.hmput_key)(
                std::ptr::null_mut(),
                es,
                k0.as_mut_ptr() as *mut c_void,
                4,
                HM_BINARY,
            );
            let sc = idx_snap(hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex).unwrap();
            let sr = idx_snap(hdr_of(map_arr(mr, es)).hash_table as *mut HashIndex).unwrap();
            assert_eq!(sc.slot_count, 8, "row19 C slot_count");
            assert_eq!(sr.slot_count, 8, "row19 Rust slot_count");
            assert_eq!(sc.arena_mode, 0, "row19 C: binary mode -> arena mode 0");
            assert_eq!(sr.arena_mode, 0, "row19 Rust: binary mode -> arena mode 0");
            assert_eq!(sc.used_count_threshold, 6);
            assert_eq!(sr.used_count_threshold, 6);
            assert_eq!(sc.used_count_shrink_threshold, 0, "row43 C forced to 0");
            assert_eq!(sr.used_count_shrink_threshold, 0, "row43 Rust forced to 0");
            assert_eq!(sc.tombstone_count_threshold, 1);
            assert_eq!(sr.tombstone_count_threshold, 1);

            // row 21 : duplicate put -> no new slot, no length change
            let len_c = hdr_of(map_arr(mc, es)).length;
            let uc = sc.used_count;
            mc = (p.c.hmput_key)(mc, es, k0.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            mr = (p.r.hmput_key)(mr, es, k0.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            let sc2 = idx_snap(hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex).unwrap();
            let sr2 = idx_snap(hdr_of(map_arr(mr, es)).hash_table as *mut HashIndex).unwrap();
            assert_eq!(hdr_of(map_arr(mc, es)).length, len_c, "row21 C length");
            assert_eq!(hdr_of(map_arr(mr, es)).length, len_c, "row21 Rust length");
            assert_eq!(sc2.used_count, uc, "row21 C used_count");
            assert_eq!(sr2.used_count, uc, "row21 Rust used_count");
            assert_eq!(
                hdr_of(map_arr(mc, es)).temp,
                hdr_of(map_arr(mr, es)).temp,
                "row21 temp"
            );

            // row 20 : the 6th distinct key crosses used_count_threshold
            for i in 1..6u32 {
                let mut k = i.to_le_bytes();
                mc = (p.c.hmput_key)(mc, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
                mr = (p.r.hmput_key)(mr, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            }
            let sc3 = idx_snap(hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex).unwrap();
            let sr3 = idx_snap(hdr_of(map_arr(mr, es)).hash_table as *mut HashIndex).unwrap();
            assert_eq!(sc3.slot_count, 8, "row20: still 8 slots at used_count 6");
            assert_eq!(sr3.slot_count, 8);
            assert_eq!(sc3.used_count, 6);
            assert_eq!(sr3.used_count, 6);
            let mut k6 = 6u32.to_le_bytes();
            mc = (p.c.hmput_key)(mc, es, k6.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            mr = (p.r.hmput_key)(mr, es, k6.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            let sc4 = idx_snap(hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex).unwrap();
            let sr4 = idx_snap(hdr_of(map_arr(mr, es)).hash_table as *mut HashIndex).unwrap();
            assert_eq!(sc4.slot_count, 16, "row20 C: table must double");
            assert_eq!(sr4.slot_count, 16, "row20 Rust: table must double");
            assert_eq!(sc4, sr4, "row20 index state after doubling");

            // row 22 : delete then re-insert -> the tombstone must be reused
            // (tombstone_count goes 1 -> 0 without used_count exceeding)
            let mut kd = 3u32.to_le_bytes();
            mc = (p.c.hmdel_key)(mc, es, kd.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
            mr = (p.r.hmdel_key)(mr, es, kd.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
            let ta = idx_snap(hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex).unwrap();
            let tb = idx_snap(hdr_of(map_arr(mr, es)).hash_table as *mut HashIndex).unwrap();
            assert_eq!(ta, tb, "row22 index state after delete");
            mc = (p.c.hmput_key)(mc, es, kd.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            mr = (p.r.hmput_key)(mr, es, kd.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            let ua = idx_snap(hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex).unwrap();
            let ub = idx_snap(hdr_of(map_arr(mr, es)).hash_table as *mut HashIndex).unwrap();
            assert_eq!(ua, ub, "row22 index state after tombstone reuse");

            (p.c.hmfree_func)(map_arr(mc, es), es);
            (p.r.hmfree_func)(map_arr(mr, es), es);
        }
        reseed(p, DEFAULT_SEED);
    }
}

// ---------------------------------------------------------------------------
// rows 25–27 : out-of-range `mode` enum values across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err_rows25_26_mode_classification() {
    let (p, _g) = libs();
    unsafe {
        let es = 16usize;
        // A map built with mode == m and one built with mode == 1 must be
        // byte-identical for every m >= 1 (row 25); likewise mode == m and
        // mode == 0 for every m < 0 (row 26).
        for &(m, reference) in [
            (1i32, 1i32),
            (2, 1),
            (3, 1),
            (7, 1),
            (1000, 1),
            (i32::MAX, 1),
            (-1, 0),
            (-2, 0),
            (i32::MIN, 0),
            (0, 0),
        ]
        .iter()
        {
            for &gseed in SEED_SET.iter() {
                // build with `m` in both libs and with `reference` in both libs
                let mut keep: Vec<Box<[u8]>> = Vec::new();
                let mut mk = |i: u32| -> *mut c_void {
                    let bx: Box<[u8]> = format!("k{i:06}\0").into_bytes().into_boxed_slice();
                    let ptr = bx.as_ptr() as *mut c_void;
                    keep.push(bx);
                    ptr
                };
                let keys: Vec<*mut c_void> = (0..20u32).map(&mut mk).collect();

                let mut got: Vec<MapSnap> = Vec::new();
                for &mm in [m, reference].iter() {
                    for lib in [&p.c, &p.r] {
                        (lib.rand_seed)(gseed);
                        let mut map = std::ptr::null_mut::<c_void>();
                        for (i, &k) in keys.iter().enumerate() {
                            map = (lib.hmput_key)(map, es, k, 8, mm);
                            let idx = hdr_of(map_arr(map, es)).temp;
                            let e = (map as *mut u8).offset(es as isize * idx);
                            for j in 8..es {
                                *e.add(j) = (i as u8).wrapping_add(j as u8);
                            }
                        }
                        let kind = if mm >= 1 { KeyKind::CStr } else { KeyKind::Bytes };
                        got.push(map_snap(map, es, 0, 8, kind));
                        (lib.hmfree_func)(map_arr(map, es), es);
                    }
                }
                assert_eq!(got[0], got[1], "mode={m} seed={gseed:#x}: C vs Rust");
                assert_eq!(got[2], got[3], "mode={reference} seed={gseed:#x}: C vs Rust");
                assert_eq!(
                    got[0], got[2],
                    "mode={m} must behave exactly like mode={reference}"
                );
            }
        }
        reseed(p, DEFAULT_SEED);
    }
}

#[test]
fn err_row27_hmdel_tests_mode_equality_not_ge() {
    let (p, _g) = libs();
    // `hmdel_key` gates the strdup-free and the string re-find on
    // `mode == STBDS_HM_STRING` (exactly 1), NOT `mode >= 1`.  With mode == 2
    // on a SH_STRDUP map the key copy is therefore leaked instead of freed and
    // no re-find happens.  Both libraries must agree, including on the
    // resulting index state.
    unsafe {
        let es = 16usize;
        for &gseed in SEED_SET.iter() {
            let mut snaps: Vec<(MapSnap, isize)> = Vec::new();
            for lib in [&p.c, &p.r] {
                (lib.rand_seed)(gseed);
                let mut map = (lib.shmode_func)(es, SH_STRDUP);
                let mut keep: Vec<Box<[u8]>> = Vec::new();
                for i in 0..10u32 {
                    let bx: Box<[u8]> = format!("dup{i:04}\0").into_bytes().into_boxed_slice();
                    let ptr = bx.as_ptr() as *mut c_void;
                    keep.push(bx);
                    map = (lib.hmput_key)(map, es, ptr, 8, HM_STRING);
                    let idx = hdr_of(map_arr(map, es)).temp;
                    let e = (map as *mut u8).offset(es as isize * idx);
                    for j in 8..es {
                        *e.add(j) = i as u8;
                    }
                }
                // delete the LAST element with mode == 2 (old_index ==
                // final_index, so the binary re-find branch is not taken)
                let last: Box<[u8]> = b"dup0009\0".to_vec().into_boxed_slice();
                map = (lib.hmdel_key)(map, es, last.as_ptr() as *mut c_void, 8, 0, 2);
                let t = hdr_of(map_arr(map, es)).temp;
                snaps.push((map_snap(map, es, 0, 8, KeyKind::CStr), t));
                (lib.hmfree_func)(map_arr(map, es), es);
            }
            assert_eq!(snaps[0].1, snaps[1].1, "row27 temp (seed {gseed:#x})");
            assert_eq!(snaps[0].1, 1, "row27 delete must report success");
            assert_eq!(snaps[0].0, snaps[1].0, "row27 map state (seed {gseed:#x})");
        }
        reseed(p, DEFAULT_SEED);
    }
}

// ---------------------------------------------------------------------------
// rows 28–30 : stbds_hmput_default
// ---------------------------------------------------------------------------

#[test]
fn err_rows28_30_hmput_default() {
    let (p, _g) = libs();
    unsafe {
        for &es in [1usize, 8, 16, 24, 40].iter() {
            // row 28 : NULL -> allocated, length 1, non-NULL
            let a = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let b = (p.r.hmput_default)(std::ptr::null_mut(), es);
            assert!(!a.is_null() && !b.is_null(), "row28 es={es} non-NULL");
            let (ha, hb) = (hdr_of(map_arr(a, es)), hdr_of(map_arr(b, es)));
            assert_eq!(ha.length, 1, "row28 C length");
            assert_eq!(hb.length, 1, "row28 Rust length");
            assert_eq!(ha.capacity, hb.capacity, "row28 capacity");
            assert_eq!(ha.temp, hb.temp, "row28 temp");
            let za = std::slice::from_raw_parts(a as *const u8, 0);
            let zb = std::slice::from_raw_parts(b as *const u8, 0);
            assert_eq!(za, zb);
            // element 0 must be zeroed in both
            let e0a = std::slice::from_raw_parts(map_arr(a, es) as *const u8, es);
            let e0b = std::slice::from_raw_parts(map_arr(b, es) as *const u8, es);
            assert_eq!(e0a, e0b, "row28 default element");
            assert!(e0a.iter().all(|&x| x == 0), "row28 default element zeroed");

            // row 30 : non-NULL with length != 0 -> identical pointer back
            let a2 = (p.c.hmput_default)(a, es);
            let b2 = (p.r.hmput_default)(b, es);
            assert_eq!(a2, a, "row30 C identical pointer");
            assert_eq!(b2, b, "row30 Rust identical pointer");

            // row 29 : non-NULL but length == 0 -> re-grow, length back to 1
            (*((map_arr(a, es) as *mut u8).sub(HDR) as *mut Header)).length = 0;
            (*((map_arr(b, es) as *mut u8).sub(HDR) as *mut Header)).length = 0;
            let a3 = (p.c.hmput_default)(a, es);
            let b3 = (p.r.hmput_default)(b, es);
            assert_eq!(
                hdr_of(map_arr(a3, es)).length,
                hdr_of(map_arr(b3, es)).length,
                "row29 length"
            );
            assert_eq!(hdr_of(map_arr(a3, es)).length, 1, "row29 length must be 1");
            assert_eq!(
                hdr_of(map_arr(a3, es)).capacity,
                hdr_of(map_arr(b3, es)).capacity,
                "row29 capacity"
            );
            (p.c.hmfree_func)(map_arr(a3, es), es);
            (p.r.hmfree_func)(map_arr(b3, es), es);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 31–34, 39–42 : stbds_hmdel_key sentinels and rehash triggers
// ---------------------------------------------------------------------------

#[test]
fn err_rows31_34_hmdel_sentinels() {
    let (p, _g) = libs();
    unsafe {
        let es = 16usize;
        let mut key = 5u32.to_le_bytes();

        // row 31 : a == NULL -> NULL
        for &m in [HM_BINARY, HM_STRING, 2, -1].iter() {
            let a = (p.c.hmdel_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                0,
                m,
            );
            let b = (p.r.hmdel_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                0,
                m,
            );
            assert!(a.is_null(), "row31 C must return NULL (mode {m})");
            assert!(b.is_null(), "row31 Rust must return NULL (mode {m})");
        }

        for &gseed in SEED_SET.iter() {
            reseed(p, gseed);
            // row 32 : table == NULL -> returns a, temp == 0
            let a = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let b = (p.r.hmput_default)(std::ptr::null_mut(), es);
            let a2 = (p.c.hmdel_key)(a, es, key.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
            let b2 = (p.r.hmdel_key)(b, es, key.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
            assert_eq!(a2, a, "row32 C returns a");
            assert_eq!(b2, b, "row32 Rust returns a");
            assert_eq!(hdr_of(map_arr(a2, es)).temp, 0, "row32 C temp");
            assert_eq!(hdr_of(map_arr(b2, es)).temp, 0, "row32 Rust temp");
            (p.c.hmfree_func)(map_arr(a2, es), es);
            (p.r.hmfree_func)(map_arr(b2, es), es);

            // rows 33/34 : populated table
            let mut mc = std::ptr::null_mut::<c_void>();
            let mut mr = std::ptr::null_mut::<c_void>();
            for i in 0..10u32 {
                let mut k = i.to_le_bytes();
                mc = (p.c.hmput_key)(mc, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
                mr = (p.r.hmput_key)(mr, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            }
            // row 33 : absent key -> temp == 0, length unchanged
            let len_before = hdr_of(map_arr(mc, es)).length;
            let mut absent = 999u32.to_le_bytes();
            mc = (p.c.hmdel_key)(mc, es, absent.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
            mr = (p.r.hmdel_key)(mr, es, absent.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
            assert_eq!(hdr_of(map_arr(mc, es)).temp, 0, "row33 C temp");
            assert_eq!(hdr_of(map_arr(mr, es)).temp, 0, "row33 Rust temp");
            assert_eq!(hdr_of(map_arr(mc, es)).length, len_before, "row33 C length");
            assert_eq!(hdr_of(map_arr(mr, es)).length, len_before, "row33 Rust length");

            // row 34 : present key -> temp == 1 and all the bookkeeping
            let before_c = idx_snap(hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex).unwrap();
            let mut k5 = 5u32.to_le_bytes();
            mc = (p.c.hmdel_key)(mc, es, k5.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
            mr = (p.r.hmdel_key)(mr, es, k5.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
            assert_eq!(hdr_of(map_arr(mc, es)).temp, 1, "row34 C temp");
            assert_eq!(hdr_of(map_arr(mr, es)).temp, 1, "row34 Rust temp");
            assert_eq!(
                hdr_of(map_arr(mc, es)).length,
                len_before - 1,
                "row34 C length"
            );
            assert_eq!(
                hdr_of(map_arr(mr, es)).length,
                len_before - 1,
                "row34 Rust length"
            );
            let after_c = idx_snap(hdr_of(map_arr(mc, es)).hash_table as *mut HashIndex).unwrap();
            let after_r = idx_snap(hdr_of(map_arr(mr, es)).hash_table as *mut HashIndex).unwrap();
            assert_eq!(after_c, after_r, "row34 index state");
            assert_eq!(
                after_c.used_count,
                before_c.used_count - 1,
                "row34 used_count decremented"
            );
            // the tombstone sentinels HASH_DELETED(1) / INDEX_DELETED(-2) must
            // be present unless the table was immediately rebuilt
            (p.c.hmfree_func)(map_arr(mc, es), es);
            (p.r.hmfree_func)(map_arr(mr, es), es);
        }
        reseed(p, DEFAULT_SEED);
    }
}

#[test]
fn err_rows39_43_delete_rehash_triggers() {
    let (p, _g) = libs();
    unsafe {
        let es = 16usize;
        for &gseed in SEED_SET.iter() {
            for &n in [8usize, 12, 20, 40, 80, 200].iter() {
                reseed(p, gseed);
                let mut mc = std::ptr::null_mut::<c_void>();
                let mut mr = std::ptr::null_mut::<c_void>();
                for i in 0..n as u32 {
                    let mut k = i.to_le_bytes();
                    mc = (p.c.hmput_key)(mc, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
                    mr = (p.r.hmput_key)(mr, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
                    let idx = hdr_of(map_arr(mc, es)).temp;
                    let idx2 = hdr_of(map_arr(mr, es)).temp;
                    for (map, ix) in [(mc, idx), (mr, idx2)] {
                        let e = (map as *mut u8).offset(es as isize * ix);
                        for j in 4..es {
                            *e.add(j) = (i as u8).wrapping_add(j as u8);
                        }
                    }
                }
                // delete in an order that mixes "last element" (row 39) with
                // "middle element" (memmove + re-find) and repeatedly crosses
                // the tombstone (row 42) and shrink (rows 40/41) thresholds
                let order: Vec<u32> = (0..n as u32).rev().step_by(3).chain(0..n as u32).collect();
                for (step, i) in order.iter().enumerate() {
                    let mut k = i.to_le_bytes();
                    mc = (p.c.hmdel_key)(mc, es, k.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
                    mr = (p.r.hmdel_key)(mr, es, k.as_mut_ptr() as *mut c_void, 4, 0, HM_BINARY);
                    let tc = hdr_of(map_arr(mc, es)).temp;
                    let tr = hdr_of(map_arr(mr, es)).temp;
                    assert_eq!(tc, tr, "n={n} s={gseed:#x} step {step}: del temp");
                    let a = map_snap(mc, es, 0, 4, KeyKind::Bytes);
                    let b = map_snap(mr, es, 0, 4, KeyKind::Bytes);
                    assert_eq!(a, b, "n={n} s={gseed:#x} step {step}: full state");
                    // row 41 : an 8-slot table must never shrink
                    if let Some(ix) = &a.idx {
                        assert!(ix.slot_count >= 8, "slot_count must never go below 8");
                        if ix.slot_count == 8 {
                            assert_eq!(
                                ix.used_count_shrink_threshold, 0,
                                "row41: 8-slot table must have shrink threshold 0"
                            );
                        }
                    }
                }
                assert_eq!(hdr_of(map_arr(mc, es)).length, 1, "everything deleted (C)");
                assert_eq!(hdr_of(map_arr(mr, es)).length, 1, "everything deleted (Rust)");
                (p.c.hmfree_func)(map_arr(mc, es), es);
                (p.r.hmfree_func)(map_arr(mr, es), es);
            }
        }
        reseed(p, DEFAULT_SEED);
    }
}

// ---------------------------------------------------------------------------
// rows 45–46 : global seed advance on fresh index, inheritance on rehash
// ---------------------------------------------------------------------------

#[test]
fn err_rows45_46_seed_advance_and_inheritance() {
    let (p, _g) = libs();
    unsafe {
        let es = 16usize;
        for &gseed in SEED_SET.iter() {
            // row 45 : each fresh table consumes and advances the global seed,
            // so successive tables get a deterministic seed sequence
            let mut seq_c = Vec::new();
            let mut seq_r = Vec::new();
            for (lib, out) in [(&p.c, &mut seq_c), (&p.r, &mut seq_r)] {
                (lib.rand_seed)(gseed);
                let mut maps = Vec::new();
                for _ in 0..8 {
                    let m = (lib.shmode_func)(es, SH_DEFAULT);
                    let t = hdr_of(map_arr(m, es)).hash_table as *mut HashIndex;
                    out.push((*t).seed);
                    maps.push(m);
                }
                for m in maps {
                    (lib.hmfree_func)(map_arr(m, es), es);
                }
            }
            assert_eq!(seq_c, seq_r, "row45 seed sequence (start {gseed:#x})");
            assert_eq!(seq_c[0], gseed, "row45 first table uses the seed verbatim");
            assert!(
                seq_c.windows(2).all(|w| w[0] != w[1] || gseed == 0),
                "row45 seed must advance"
            );

            // row 46 : growing a table (ot != NULL) inherits the seed and does
            // NOT advance the global — so the NEXT fresh table's seed equals
            // the seed of a fresh table created without any growth in between.
            let mut next_c = Vec::new();
            let mut next_r = Vec::new();
            for (lib, out) in [(&p.c, &mut next_c), (&p.r, &mut next_r)] {
                (lib.rand_seed)(gseed);
                let mut m = std::ptr::null_mut::<c_void>();
                for i in 0..200u32 {
                    let mut k = i.to_le_bytes();
                    m = (lib.hmput_key)(m, es, k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
                }
                let t = hdr_of(map_arr(m, es)).hash_table as *mut HashIndex;
                out.push((*t).seed); // inherited through every doubling
                let m2 = (lib.shmode_func)(es, SH_DEFAULT);
                let t2 = hdr_of(map_arr(m2, es)).hash_table as *mut HashIndex;
                out.push((*t2).seed); // the next fresh table
                (lib.hmfree_func)(map_arr(m, es), es);
                (lib.hmfree_func)(map_arr(m2, es), es);
            }
            assert_eq!(next_c, next_r, "row46 seeds (start {gseed:#x})");
            assert_eq!(next_c[0], gseed, "row46 grown table keeps the original seed");
            assert_eq!(
                next_c[1], seq_c[1],
                "row46 growth must not advance the global seed"
            );
        }
        reseed(p, DEFAULT_SEED);
    }
}

// ---------------------------------------------------------------------------
// rows 47–52 : string arena boundaries
// ---------------------------------------------------------------------------

#[test]
fn err_rows47_52_stralloc_boundaries() {
    let (p, _g) = libs();
    unsafe {
        // rows 47/48/49/51: exact boundary lengths around the first blocksize
        for &len in [0usize, 1, 2, 510, 511, 512, 513, 1023, 1024, 1025, 1_048_575, 1_048_576, 1_048_577].iter() {
            for prefill in [false, true] {
                let mut ac = Arena::zeroed();
                let mut ar = Arena::zeroed();
                if prefill {
                    let mut pre = vec![b'p'; 8];
                    pre.push(0);
                    let x = (p.c.stralloc)(&mut ac, pre.as_mut_ptr() as *mut c_char);
                    let y = (p.r.stralloc)(&mut ar, pre.as_mut_ptr() as *mut c_char);
                    assert_eq!(cstr_bytes(x), cstr_bytes(y));
                    assert_eq!((ac.remaining, ac.block), (ar.remaining, ar.block));
                }
                let mut s = vec![b'x'; len];
                s.push(0);
                let a = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
                let b = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
                assert_eq!(cstr_bytes(a).len(), len, "row47 len={len} C content length");
                assert_eq!(cstr_bytes(b).len(), len, "row47 len={len} Rust content length");
                assert_eq!(cstr_bytes(a), cstr_bytes(b), "len={len} prefill={prefill}");
                assert_eq!(
                    ac.remaining, ar.remaining,
                    "len={len} prefill={prefill}: remaining"
                );
                assert_eq!(ac.block, ar.block, "len={len} prefill={prefill}: block");
                assert_eq!(
                    ac.storage.is_null(),
                    ar.storage.is_null(),
                    "len={len}: storage NULL-ness"
                );
                // row 49: an oversized FIRST allocation leaves remaining == 0
                if !prefill && len + 1 > 512 {
                    assert_eq!(ac.remaining, 0, "row49 C remaining must be 0 (len={len})");
                    assert_eq!(ar.remaining, 0, "row49 Rust remaining must be 0 (len={len})");
                }
                (p.c.strreset)(&mut ac);
                (p.r.strreset)(&mut ar);
                assert_eq!(ac, Arena::zeroed(), "row52 C reset");
                assert_eq!(ar, Arena::zeroed(), "row52 Rust reset");
            }
        }

        // row 48 : `block` must stop incrementing once blocksize >= 1<<20
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        let mut big = vec![b'B'; 1_200_000];
        big.push(0);
        for i in 0..50usize {
            let a = (p.c.stralloc)(&mut ac, big.as_mut_ptr() as *mut c_char);
            let b = (p.r.stralloc)(&mut ar, big.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(a).len(), 1_200_000);
            assert_eq!(cstr_bytes(a), cstr_bytes(b), "row48 content {i}");
            assert_eq!(ac.block, ar.block, "row48 block {i}");
            assert_eq!(ac.remaining, ar.remaining, "row48 remaining {i}");
        }
        assert_eq!(ac.block, 22, "row48 C block saturates at 22");
        assert_eq!(ar.block, 22, "row48 Rust block saturates at 22");
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);

        // row 51/52 : repeated empty strings then reset, twice over
        for _ in 0..2 {
            for i in 0..2000usize {
                let mut e = [0u8; 1];
                let a = (p.c.stralloc)(&mut ac, e.as_mut_ptr() as *mut c_char);
                let b = (p.r.stralloc)(&mut ar, e.as_mut_ptr() as *mut c_char);
                assert_eq!(cstr_bytes(a), Vec::<u8>::new(), "row51 empty {i}");
                assert_eq!(cstr_bytes(b), Vec::<u8>::new(), "row51 empty {i}");
                assert_eq!(ac.remaining, ar.remaining, "row51 remaining {i}");
                assert_eq!(ac.block, ar.block, "row51 block {i}");
            }
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
            assert_eq!(ac, Arena::zeroed());
            assert_eq!(ar, Arena::zeroed());
        }
    }
}

// ---------------------------------------------------------------------------
// rows 53–58 : hashing boundaries (empty, high-bit, len == 0, all tail cases)
// ---------------------------------------------------------------------------

#[test]
fn err_rows53_58_hash_boundaries() {
    let (p, _g) = libs();
    unsafe {
        // row 53 : empty string
        let mut empty = [0u8; 1];
        for &seed in SEED_SET.iter() {
            assert_eq!(
                (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed),
                (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed),
                "row53 seed={seed:#x}"
            );
        }
        // row 54 : every single byte value 0x01..0xFF as a one-char string
        for byte in 1u16..256 {
            let mut s = [byte as u8, 0];
            for &seed in SEED_SET.iter() {
                assert_eq!(
                    (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
                    (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
                    "row54 byte={byte:#x} seed={seed:#x}"
                );
            }
        }
        // row 55 : len == 0 — the pointer must not even be read; pass a
        // dangling-but-valid one-byte buffer to prove it
        let mut one = [0xAAu8; 1];
        for &seed in SEED_SET.iter() {
            assert_eq!(
                (p.c.hash_bytes)(one.as_mut_ptr() as *mut c_void, 0, seed),
                (p.r.hash_bytes)(one.as_mut_ptr() as *mut c_void, 0, seed),
                "row55 seed={seed:#x}"
            );
        }
        // rows 56/57 : every tail remainder 0..7, with and without the
        // sign-extending high bit in byte 3
        for len in 0..32usize {
            for &fill in [0x00u8, 0x7f, 0x80, 0xff].iter() {
                let mut buf = vec![fill; len + 16];
                for &seed in SEED_SET.iter() {
                    assert_eq!(
                        (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
                        (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
                        "row56/57 len={len} fill={fill:#x} seed={seed:#x}"
                    );
                }
                // isolate byte 3 (the `d[3] << 24` int-overflow path)
                if len >= 4 {
                    let mut b2 = vec![0u8; len + 16];
                    b2[3] = 0x80;
                    for &seed in SEED_SET.iter() {
                        assert_eq!(
                            (p.c.hash_bytes)(b2.as_mut_ptr() as *mut c_void, len, seed),
                            (p.r.hash_bytes)(b2.as_mut_ptr() as *mut c_void, len, seed),
                            "row57 isolated d[3] len={len} seed={seed:#x}"
                        );
                    }
                }
            }
        }
        // row 58 : `len << 56` — lengths that differ only above the low byte
        for &(l1, l2) in [(1usize, 257usize), (2, 258), (0, 256)].iter() {
            let mut buf = vec![0u8; l2 + 16];
            for &seed in SEED_SET.iter() {
                let c1 = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, l1, seed);
                let r1 = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, l1, seed);
                let c2 = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, l2, seed);
                let r2 = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, l2, seed);
                assert_eq!(c1, r1, "row58 len={l1} seed={seed:#x}");
                assert_eq!(c2, r2, "row58 len={l2} seed={seed:#x}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 59–60 : stbds_shmode_func out-of-range mode and elemsize == 0
// ---------------------------------------------------------------------------

#[test]
fn err_rows59_60_shmode_func() {
    let (p, _g) = libs();
    unsafe {
        for &gseed in SEED_SET.iter() {
            // row 59 : (unsigned char) truncation of the mode int
            for &(m, expect) in [
                (0i32, 0u8),
                (1, 1),
                (2, 2),
                (3, 3),
                (4, 4),
                (255, 255),
                (256, 0),
                (257, 1),
                (1000, 232),
                (-1, 255),
                (-256, 0),
                (i32::MAX, 255),
                (i32::MIN, 0),
            ]
            .iter()
            {
                let es = 16usize;
                (p.c.rand_seed)(gseed);
                (p.r.rand_seed)(gseed);
                let a = (p.c.shmode_func)(es, m);
                let b = (p.r.shmode_func)(es, m);
                let ta = hdr_of(map_arr(a, es)).hash_table as *mut HashIndex;
                let tb = hdr_of(map_arr(b, es)).hash_table as *mut HashIndex;
                assert_eq!((*ta).string.mode, expect, "row59 C mode={m}");
                assert_eq!((*tb).string.mode, expect, "row59 Rust mode={m}");
                assert_eq!(idx_snap(ta), idx_snap(tb), "row59 index state mode={m}");
                assert_eq!(
                    hdr_of(map_arr(a, es)),
                    {
                        let mut h = hdr_of(map_arr(b, es));
                        h.hash_table = hdr_of(map_arr(a, es)).hash_table;
                        h
                    },
                    "row59 header mode={m}"
                );
                (p.c.hmfree_func)(map_arr(a, es), es);
                (p.r.hmfree_func)(map_arr(b, es), es);
            }

            // row 60 : elemsize == 0
            (p.c.rand_seed)(gseed);
            (p.r.rand_seed)(gseed);
            let a = (p.c.shmode_func)(0, SH_DEFAULT);
            let b = (p.r.shmode_func)(0, SH_DEFAULT);
            assert_eq!(a.is_null(), b.is_null(), "row60 NULL-ness");
            let ha = hdr_of(map_arr(a, 0));
            let hb = hdr_of(map_arr(b, 0));
            assert_eq!((ha.length, ha.capacity, ha.temp), (1, 4, 0), "row60 C header");
            assert_eq!((hb.length, hb.capacity, hb.temp), (1, 4, 0), "row60 Rust header");
            assert_eq!(
                idx_snap(ha.hash_table as *mut HashIndex),
                idx_snap(hb.hash_table as *mut HashIndex),
                "row60 index"
            );
            (p.c.hmfree_func)(map_arr(a, 0), 0);
            (p.r.hmfree_func)(map_arr(b, 0), 0);
        }
        reseed(p, DEFAULT_SEED);
    }
}

// ---------------------------------------------------------------------------
// rows 62–63 : strkey / helxo edge inputs
// ---------------------------------------------------------------------------

#[test]
fn err_row62_strkey_extremes() {
    let (p, _g) = libs();
    unsafe {
        for &n in [0i32, 1, -1, i32::MAX, i32::MIN, -2147483647, 100000000].iter() {
            let a = (p.c.strkey)(n);
            let b = (p.r.strkey)(n);
            assert_eq!(cstr_bytes(a), cstr_bytes(b), "row62 strkey({n})");
            assert_eq!(cstr_bytes(a), format!("test_{n}").into_bytes());
        }
    }
}

#[test]
fn err_row63_helxo_every_char() {
    let (p, _g) = libs();
    for byte in 0u16..256 {
        let ch = byte as u8 as c_char;
        let a = capture_stdout("errc", || unsafe { (p.c.helxo)(ch) });
        let b = capture_stdout("errr", || unsafe { (p.r.helxo)(ch) });
        assert_eq!(a, b, "row63 helxo({byte:#04x})");
        // The output is always exactly these 5 records; `%c` writes the raw
        // byte, so byte 0x0a produces an extra newline and byte 0x00 an
        // embedded NUL — compare the whole buffer rather than splitting.
        let mut want = Vec::new();
        want.extend_from_slice(b"bob h\nsally e\nfred l\njen ");
        want.push(byte as u8);
        want.extend_from_slice(b"\ndoug o\n");
        assert_eq!(a, want, "row63 helxo({byte:#04x}) exact bytes");
    }
}

// ---------------------------------------------------------------------------
// generic FFI boundary sweep: NULL pointers on every function that documents a
// NULL check, plus zero sizes
// ---------------------------------------------------------------------------

#[test]
fn err_generic_null_and_zero_boundaries() {
    let (p, _g) = libs();
    unsafe {
        // hmfree_func(NULL, *) — no-op
        for &es in [0usize, 1, 8, usize::MAX / 2].iter() {
            (p.c.hmfree_func)(std::ptr::null_mut(), es);
            (p.r.hmfree_func)(std::ptr::null_mut(), es);
        }
        // hmdel_key(NULL, ...) — NULL for every mode / keysize / keyoffset
        for &es in [0usize, 1, 8, 16].iter() {
            for &ks in [0usize, 1, 8, 16].iter() {
                for &ko in [0usize, 1, 8].iter() {
                    for &m in [-1i32, 0, 1, 2, 999].iter() {
                        let a = (p.c.hmdel_key)(std::ptr::null_mut(), es, std::ptr::null_mut(), ks, ko, m);
                        let b = (p.r.hmdel_key)(std::ptr::null_mut(), es, std::ptr::null_mut(), ks, ko, m);
                        assert!(a.is_null() && b.is_null(), "hmdel_key(NULL) es={es} ks={ks} ko={ko} m={m}");
                    }
                }
            }
        }
        // hmget_key_ts(NULL, ...) — allocates, *temp = -1; the key pointer is
        // never dereferenced on this path, so NULL is a legal key here
        for &es in [1usize, 8, 16, 40].iter() {
            for &m in [-1i32, 0, 1, 2, 999].iter() {
                let mut tc: isize = 7;
                let mut tr: isize = 7;
                let a = (p.c.hmget_key_ts)(std::ptr::null_mut(), es, std::ptr::null_mut(), 0, &mut tc, m);
                let b = (p.r.hmget_key_ts)(std::ptr::null_mut(), es, std::ptr::null_mut(), 0, &mut tr, m);
                assert_eq!((tc, tr), (-1, -1), "hmget_key_ts(NULL) es={es} m={m}");
                assert_eq!(
                    hdr_of(map_arr(a, es)),
                    {
                        let mut h = hdr_of(map_arr(b, es));
                        h.hash_table = hdr_of(map_arr(a, es)).hash_table;
                        h
                    },
                    "hmget_key_ts(NULL) header es={es} m={m}"
                );
                (p.c.hmfree_func)(map_arr(a, es), es);
                (p.r.hmfree_func)(map_arr(b, es), es);
            }
        }
        // zero-length hash of a NULL pointer: len == 0 means p is never read
        for &seed in SEED_SET.iter() {
            assert_eq!(
                (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed),
                (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed),
                "hash_bytes(NULL, 0, {seed:#x})"
            );
        }
        reseed(p, DEFAULT_SEED);
    }
}
