//! Phase B: binary-mode hash tables.
//! CONFIGS rows 21–28, 33, 39–40, 43–54, 59, 60, 64.

mod common;
use common::*;
use std::ffi::c_void;

fn fresh(elemsize: usize, keysize: usize, mode: i32) -> Dual {
    set_seed(DEFAULT_SEED);
    Dual::new(elemsize, keysize, mode, KeyRepr::Bytes)
}

// --- rows 21/22/23/24: insert counts around the grow threshold ------------

fn insert_n(row: &str, n: usize, elemsize: usize, keysize: usize, rngseed: u64) {
    let _g = serial();
    let mut rng = Rng::new(rngseed);
    let mut keys = Keys::binary(&mut rng, n, keysize);
    let mut d = fresh(elemsize, keysize, HM_BINARY);
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        d.put(k, &format!("row{row} insert i={i} n={n} es={elemsize} ks={keysize}"));
    }
    let s = d.snap_c();
    assert_eq!(s.length, keys.len() + 1, "row{row}: length");
    // every key must be findable, with the index it was inserted at
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        let t = d.get(k, &format!("row{row} get i={i}"));
        assert_eq!(t, i as isize, "row{row}: index of key {i}");
    }
    d.free();
}

#[test]
fn row21_binary_single_insert() {
    insert_n("21", 1, 8, 8, 0x2101);
}

#[test]
fn row22_binary_six_inserts_at_threshold() {
    insert_n("22", 6, 8, 8, 0x2201);
    // used_count_threshold for 8 slots is 8 - 2 = 6, so the 6th insert is the
    // last one that fits before the doubling.
    let _g = serial();
    let mut rng = Rng::new(0x2202);
    let mut keys = Keys::binary(&mut rng, 6, 8);
    let mut d = fresh(8, 8, HM_BINARY);
    for i in 0..6 {
        let k = keys.ptr(i);
        d.put(k, "row22");
        let s = d.snap_c();
        assert_eq!(s.slot_count, 8, "row22: must not grow within the first 6 inserts");
        assert_eq!(s.used_count_threshold, 6);
        assert_eq!(s.used_count_shrink_threshold, 0, "row22: 8 slots => shrink disabled");
    }
    d.free();
}

#[test]
fn row23_binary_seventh_insert_grows() {
    let _g = serial();
    let mut rng = Rng::new(0x2301);
    let mut keys = Keys::binary(&mut rng, 7, 8);
    let mut d = fresh(8, 8, HM_BINARY);
    for i in 0..7 {
        let k = keys.ptr(i);
        d.put(k, &format!("row23 i={i}"));
    }
    let s = d.snap_c();
    assert_eq!(s.slot_count, 16, "row23: 7th insert must double the table");
    assert_eq!(s.used_count, 7);
    assert_eq!(s.used_count_threshold, 12);
    assert_eq!(s.tombstone_count_threshold, 3);
    assert_eq!(s.used_count_shrink_threshold, 4);
    d.free();
}

#[test]
fn row24_binary_1000_inserts() {
    insert_n("24", 1000, 8, 8, 0x2401);
}

// --- row 25: elemsize x keysize matrix ------------------------------------

#[test]
fn row25_binary_elemsize_keysize_matrix() {
    for &es in &[4usize, 12, 16, 24] {
        for &ks in &[1usize, 2, 4, 8, 16] {
            if ks > es {
                continue;
            }
            insert_n("25", 200, es, ks, 0x2500 ^ (es as u64) << 8 ^ ks as u64);
        }
    }
}

// --- row 26: keysize == 0 -------------------------------------------------

#[test]
fn row26_binary_keysize_zero() {
    let _g = serial();
    let mut rng = Rng::new(0x2601);
    let mut d = fresh(8, 0, HM_BINARY);
    let mut bufs: Vec<Box<[u8]>> = (0..20).map(|_| rng.bytes(8).into_boxed_slice()).collect();
    for i in 0..bufs.len() {
        let k = bufs[i].as_mut_ptr() as *mut c_void;
        d.put(k, &format!("row26 i={i}"));
    }
    // memcmp(...,0) == 0 always, and every key hashes identically, so exactly
    // one element can ever exist
    let s = d.snap_c();
    assert_eq!(s.length, 2, "row26: expected the default slot + one element");
    assert_eq!(s.used_count, 1);
    for i in 0..bufs.len() {
        let k = bufs[i].as_mut_ptr() as *mut c_void;
        assert_eq!(d.get(k, "row26 get"), 0);
    }
    d.free();
}

// --- rows 27/28: duplicate inserts (both sub-loops) -----------------------

#[test]
fn row27_binary_reput_duplicates() {
    let _g = serial();
    let mut rng = Rng::new(0x2701);
    let n = 500;
    let mut keys = Keys::binary(&mut rng, n, 8);
    let mut d = fresh(16, 8, HM_BINARY);
    for i in 0..n {
        let k = keys.ptr(i);
        d.put(k, &format!("row27 put i={i}"));
    }
    let before = d.snap_c();
    // re-put every key in a shuffled order: no insert may happen
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        order.swap(i, rng.below(i + 1));
    }
    for &i in &order {
        let k = keys.ptr(i);
        let t = d.put(k, &format!("row27 reput i={i}"));
        assert_eq!(t, i as isize, "row27: re-put must land on the existing index");
    }
    let after = d.snap_c();
    assert_eq!(before.length, after.length, "row27: no growth on re-put");
    assert_eq!(before.used_count, after.used_count);
    assert_eq!(before.slots, after.slots, "row27: bucket contents unchanged");
    d.free();
}

#[test]
fn row28_binary_wraparound_subloop_duplicates() {
    // Enough elements + a small stride that buckets fill up, so that the
    // `for (i = 0; i < limit; ++i)` wrap-around sub-loop is the one that finds
    // the duplicate. Both libraries must classify every key the same way.
    let _g = serial();
    let mut rng = Rng::new(0x2801);
    let n = 2000;
    let mut keys = Keys::binary(&mut rng, n, 8);
    let mut d = fresh(16, 8, HM_BINARY);
    for i in 0..n {
        let k = keys.ptr(i);
        d.put(k, &format!("row28 put i={i}"));
    }
    for i in 0..n {
        let k = keys.ptr(i);
        assert_eq!(d.put(k, &format!("row28 reput i={i}")), i as isize);
        assert_eq!(d.get(k, &format!("row28 get i={i}")), i as isize);
    }
    d.free();
}

// --- row 33: out-of-range negative `mode` behaves as binary ---------------

#[test]
fn row33_negative_mode_is_binary() {
    for &mode in &[-1i32, -2, -1000, i32::MIN] {
        let _g = serial();
        let mut rng = Rng::new(0x3300 ^ (mode as u32 as u64));
        let mut keys = Keys::binary(&mut rng, 100, 8);
        set_seed(DEFAULT_SEED);
        let mut d = Dual::new(16, 8, mode, KeyRepr::Bytes);
        for i in 0..keys.len() {
            let k = keys.ptr(i);
            d.put(k, &format!("row33 mode={mode} put i={i}"));
        }
        assert_eq!(d.snap_c().arena_mode, 0, "row33: fresh table string.mode must be 0");
        for i in 0..keys.len() {
            let k = keys.ptr(i);
            assert_eq!(d.get(k, "row33 get"), i as isize);
        }
        d.free();
    }
}

// --- rows 39/40: hmget_key present / absent -------------------------------

#[test]
fn row39_40_binary_get_present_and_absent() {
    let _g = serial();
    let mut rng = Rng::new(0x3901);
    let n = 400;
    let mut keys = Keys::binary(&mut rng, n * 2, 8);
    let mut d = fresh(16, 8, HM_BINARY);
    for i in 0..n {
        let k = keys.ptr(i);
        d.put(k, &format!("row39 put i={i}"));
    }
    for i in 0..n {
        let k = keys.ptr(i);
        assert_eq!(d.get(k, "row39 present"), i as isize);
    }
    for i in n..2 * n {
        let k = keys.ptr(i);
        assert_eq!(d.get(k, "row40 absent"), -1, "row40: absent key must yield -1");
    }
    d.free();
}

// --- row 43/44: hmget_key_ts ---------------------------------------------

#[test]
fn row43_binary_get_ts() {
    let _g = serial();
    let mut rng = Rng::new(0x4301);
    let n = 300;
    let mut keys = Keys::binary(&mut rng, n * 2, 8);
    let mut d = fresh(16, 8, HM_BINARY);
    for i in 0..n {
        let k = keys.ptr(i);
        d.put(k, "row43 put");
    }
    // header->temp must be left untouched by hmget_key_ts
    let temp_before_c = d.snap_c().temp;
    for i in 0..n {
        let k = keys.ptr(i);
        assert_eq!(d.get_ts(k, "row43 present"), i as isize);
    }
    for i in n..2 * n {
        let k = keys.ptr(i);
        assert_eq!(d.get_ts(k, "row43 absent"), -1);
    }
    assert_eq!(d.snap_c().temp, temp_before_c, "row43: header->temp must not change");
    assert_eq!(d.snap_r().temp, temp_before_c);
    d.free();
}

#[test]
fn row44_get_ts_bootstrap_from_null() {
    let _g = serial();
    let (c, r) = libs();
    for &es in &[1usize, 8, 16, 24] {
        set_seed(DEFAULT_SEED);
        unsafe {
            let mut tc: isize = 0x1234;
            let mut tr: isize = 0x1234;
            let a = (c.hmget_key_ts)(std::ptr::null_mut(), es, std::ptr::null_mut(), 8, &mut tc, HM_BINARY);
            let b = (r.hmget_key_ts)(std::ptr::null_mut(), es, std::ptr::null_mut(), 8, &mut tr, HM_BINARY);
            let ctx = format!("row44 es={es}");
            assert_eq!(tc, tr, "{ctx}: *temp");
            assert_eq!(tc, -1, "{ctx}: *temp must be STBDS_INDEX_EMPTY");
            assert_eq!(snapshot(a, es), snapshot(b, es), "{ctx}");
            assert_eq!(snapshot(a, es).length, 1, "{ctx}");
            let ea = std::slice::from_raw_parts((a as *mut u8).sub(es), es).to_vec();
            let eb = std::slice::from_raw_parts((b as *mut u8).sub(es), es).to_vec();
            assert_eq!(ea, eb, "{ctx}: bootstrap element bytes");
            assert!(ea.iter().all(|&x| x == 0), "{ctx}: must be zeroed");
            (c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// --- rows 45..54: deletion patterns --------------------------------------

fn del_pattern(row: &str, n: usize, order: impl Fn(&mut Rng, usize) -> Vec<usize>, rngseed: u64) {
    let _g = serial();
    let mut rng = Rng::new(rngseed);
    let mut keys = Keys::binary(&mut rng, n, 8);
    let mut d = fresh(16, 8, HM_BINARY);
    for i in 0..n {
        let k = keys.ptr(i);
        d.put(k, &format!("row{row} put i={i}"));
    }
    let ord = order(&mut rng, n);
    let mut live: Vec<usize> = (0..n).collect();
    for (step, &i) in ord.iter().enumerate() {
        let k = keys.ptr(i);
        let t = d.del(k, 0, &format!("row{row} del step={step} key={i}"));
        assert_eq!(t, 1, "row{row}: delete of a present key must report 1");
        live.retain(|&x| x != i);
        assert_eq!(d.snap_c().length, live.len() + 1, "row{row}: length after step {step}");
        // deleting again must be a no-op
        let t2 = d.del(k, 0, &format!("row{row} redel step={step}"));
        assert_eq!(t2, 0, "row{row}: second delete must report 0");
        // all remaining keys must still be findable
        if step % 37 == 0 || live.len() < 12 {
            for &j in &live {
                let kj = keys.ptr(j);
                let tj = d.get(kj, &format!("row{row} get {j} after step {step}"));
                assert!(tj >= 0, "row{row}: key {j} lost after deleting {i}");
            }
        }
    }
    assert_eq!(d.snap_c().length, 1, "row{row}: only the default slot must remain");
    d.free();
}

#[test]
fn row45_delete_last() {
    // deleting the most recently inserted element hits `old_index == final_index`
    del_pattern("45", 200, |_r, n| (0..n).rev().collect(), 0x4501);
}

#[test]
fn row46_delete_first() {
    del_pattern("46", 200, |_r, n| (0..n).collect(), 0x4601);
}

#[test]
fn row47_delete_middle() {
    del_pattern("47", 201, |_r, n| {
        let mut v = Vec::new();
        let mut lo = 0;
        let mut hi = n - 1;
        while lo <= hi {
            v.push((lo + hi) / 2);
            let m = (lo + hi) / 2;
            if v.len() >= n {
                break;
            }
            if m == lo {
                lo += 1;
            } else {
                hi = m - 1;
            }
        }
        // fill in the rest
        for i in 0..n {
            if !v.contains(&i) {
                v.push(i);
            }
        }
        v
    }, 0x4701);
}

#[test]
fn row48_delete_insertion_order() {
    del_pattern("48", 100, |_r, n| (0..n).collect(), 0x4801);
}

#[test]
fn row49_delete_reverse_order() {
    del_pattern("49", 100, |_r, n| (0..n).rev().collect(), 0x4901);
}

#[test]
fn row50_delete_random_order_1000() {
    del_pattern("50", 1000, |r, n| {
        let mut v: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            v.swap(i, r.below(i + 1));
        }
        v
    }, 0x5001);
}

#[test]
fn row51_delete_then_reinsert_tombstone_reuse() {
    let _g = serial();
    let mut rng = Rng::new(0x5101);
    let n = 300;
    let mut keys = Keys::binary(&mut rng, n, 8);
    let mut d = fresh(16, 8, HM_BINARY);
    for i in 0..n {
        let k = keys.ptr(i);
        d.put(k, "row51 put");
    }
    for round in 0..6 {
        // delete a third, re-insert them
        let victims: Vec<usize> = (0..n).filter(|x| x % 3 == round % 3).collect();
        for &i in &victims {
            let k = keys.ptr(i);
            assert_eq!(d.del(k, 0, &format!("row51 del r={round} i={i}")), 1);
        }
        for &i in &victims {
            let k = keys.ptr(i);
            d.put(k, &format!("row51 reput r={round} i={i}"));
        }
        assert_eq!(d.snap_c().length, n + 1, "row51: length restored");
        for i in 0..n {
            let k = keys.ptr(i);
            assert!(d.get(k, "row51 get") >= 0);
        }
    }
    d.free();
}

#[test]
fn row52_tombstone_rebuild_same_size() {
    // 32 slots: tombstone_count_threshold = 4 + 2 = 6, shrink threshold = 8.
    let _g = serial();
    let mut rng = Rng::new(0x5201);
    let mut keys = Keys::binary(&mut rng, 20, 8);
    let mut d = fresh(16, 8, HM_BINARY);
    for i in 0..20 {
        let k = keys.ptr(i);
        d.put(k, "row52 put");
    }
    // 8 slots -> 16 at the 7th insert -> 32 at the 13th; threshold(32) = 24
    assert_eq!(d.snap_c().slot_count, 32, "row52: 20 inserts => 32 slots");
    let mut seen_rebuild = false;
    let mut prev = d.snap_c();
    for i in 0..20 {
        let k = keys.ptr(i);
        d.del(k, 0, &format!("row52 del i={i}"));
        let now = d.snap_c();
        if now.tombstone_count < prev.tombstone_count {
            seen_rebuild = true;
        }
        prev = now;
    }
    assert!(seen_rebuild, "row52: expected at least one tombstone purge (rebuild/shrink)");
    d.free();
}

#[test]
fn row53_shrink_halves_table() {
    let _g = serial();
    let mut rng = Rng::new(0x5301);
    let n = 400;
    let mut keys = Keys::binary(&mut rng, n, 8);
    let mut d = fresh(16, 8, HM_BINARY);
    for i in 0..n {
        let k = keys.ptr(i);
        d.put(k, "row53 put");
    }
    let peak = d.snap_c().slot_count;
    assert!(peak >= 512, "row53: expected a large table, got {peak}");
    let mut shrinks = 0;
    let mut prev = peak;
    for i in 0..n {
        let k = keys.ptr(i);
        d.del(k, 0, &format!("row53 del i={i}"));
        let now = d.snap_c().slot_count;
        if now < prev {
            shrinks += 1;
        }
        prev = now;
    }
    assert!(shrinks >= 3, "row53: expected several shrinks, saw {shrinks}");
    assert_eq!(prev, 8, "row53: must bottom out at 8 slots");
    d.free();
}

#[test]
fn row54_never_shrinks_below_8() {
    let _g = serial();
    let mut rng = Rng::new(0x5401);
    let mut keys = Keys::binary(&mut rng, 5, 8);
    let mut d = fresh(16, 8, HM_BINARY);
    for i in 0..5 {
        let k = keys.ptr(i);
        d.put(k, "row54 put");
    }
    assert_eq!(d.snap_c().slot_count, 8);
    assert_eq!(d.snap_c().used_count_shrink_threshold, 0);
    for i in 0..5 {
        let k = keys.ptr(i);
        d.del(k, 0, &format!("row54 del i={i}"));
        assert_eq!(d.snap_c().slot_count, 8, "row54: must stay at 8 slots");
    }
    d.free();
}

// --- row 59: non-zero keyoffset in hmdel_key -----------------------------

#[test]
fn row59_hmdel_key_with_keyoffset() {
    // Elements are 16 bytes: key at 0..8 (written by hmput_key) and a *copy* of
    // the key at 8..16 (written by us), so a delete with keyoffset == 8 finds
    // and relocates elements exactly the same way a keyoffset == 0 delete would.
    let (c, r) = libs();
    for &keyoffset in &[0usize, 8] {
        let _g = serial();
        set_seed(DEFAULT_SEED);
        let mut rng = Rng::new(0x5900 ^ keyoffset as u64);
        let n = 300;
        let mut keys = Keys::binary(&mut rng, n, 8);
        let es = 16usize;
        let mut cp: *mut c_void = std::ptr::null_mut();
        let mut rp: *mut c_void = std::ptr::null_mut();
        unsafe {
            for i in 0..n {
                let k = keys.ptr(i);
                cp = (c.hmput_key)(cp, es, k, 8, HM_BINARY);
                rp = (r.hmput_key)(rp, es, k, 8, HM_BINARY);
                assert_eq!(snapshot(cp, es), snapshot(rp, es), "row59 put i={i}");
                let idx = (*((cp as *mut u8).sub(es) as *mut ArrayHeader).offset(-1)).temp;
                for hp in [cp, rp] {
                    let e = (hp as *mut u8).offset(es as isize * idx);
                    std::ptr::copy_nonoverlapping(k as *const u8, e.add(8), 8);
                }
            }
            let mut order: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                order.swap(i, rng.below(i + 1));
            }
            for (step, &i) in order.iter().enumerate() {
                let k = keys.ptr(i);
                let a = (c.hmdel_key)(cp, es, k, 8, keyoffset, HM_BINARY);
                let b = (r.hmdel_key)(rp, es, k, 8, keyoffset, HM_BINARY);
                assert_eq!(a.is_null(), b.is_null());
                cp = a;
                rp = b;
                let ctx = format!("row59 keyoffset={keyoffset} step={step} key={i}");
                assert_eq!(snapshot(cp, es), snapshot(rp, es), "{ctx}");
                let ec = elem_bytes(cp, es, snapshot(cp, es).length);
                let er = elem_bytes(rp, es, snapshot(rp, es).length);
                assert_eq!(ec, er, "{ctx}: element bytes");
            }
            assert_eq!(snapshot(cp, es).length, 1, "row59 keyoffset={keyoffset}: all deleted");
            (c.hmfree_func)((cp as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((rp as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// --- rows 60/64: hmfree_func on binary tables ---------------------------

#[test]
fn row60_hmfree_binary_table() {
    let _g = serial();
    let mut rng = Rng::new(0x6001);
    for &n in &[0usize, 1, 7, 100] {
        let mut keys = Keys::binary(&mut rng, n.max(1), 8);
        let mut d = fresh(16, 8, HM_BINARY);
        for i in 0..n {
            let k = keys.ptr(i);
            d.put(k, "row60 put");
        }
        if n > 0 {
            assert_eq!(d.snap_c().arena_mode, 0, "row60: SH_NONE");
        }
        d.free();
        assert!(d.cp.is_null());
    }
}

#[test]
fn row64_hmfree_array_without_table() {
    let (c, r) = libs();
    for &es in &[1usize, 8, 16] {
        unsafe {
            let a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            let b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            assert_eq!(snapshot_raw(a), snapshot_raw(b));
            assert!(!snapshot_raw(a).has_table);
            // hmfree_func takes the raw (unbiased) pointer
            (c.hmfree_func)(a, es);
            (r.hmfree_func)(b, es);
        }
    }
}
