//! Phase C, ERRORS.md — non-fatal rejection / error / sentinel paths.
//! (Rows whose expected C result is a signal live in `tests/errors_fatal.rs`.)

mod common;
use common::map::*;
use common::*;
use std::ffi::c_void;

fn ints(v: i32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

// ===========================================================================
// row 1 — arrgrowf: min_cap <= arrcap(a) is a no-op returning `a` itself
// ===========================================================================
#[test]
fn err01_arrgrowf_no_grow_returns_same_ptr() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // a == NULL, nothing requested => arrcap(NULL) == 0, 0 <= 0 => return NULL
        for es in [0usize, 1, 4, 8, 4096] {
            let c = (p.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let r = (p.rs.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert_eq_ctx(c.is_null(), r.is_null(), &format!("arrgrowf(NULL,{es},0,0)"));
            assert!(c.is_null(), "C must return NULL");
        }
        // a != NULL, request already fits => the identical pointer comes back
        for es in [1usize, 8, 40] {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), es, 10, 0);
            let b = (p.rs.arrgrowf)(std::ptr::null_mut(), es, 10, 0);
            let cap = rd_usize((a as *mut u8).sub(HDR_SIZE), HDR_CAPACITY);
            assert_eq!(cap, 10);
            for mc in 0..=cap {
                assert_eq!((p.c.arrgrowf)(a, es, 0, mc), a, "C es={es} mc={mc}");
                assert_eq!((p.rs.arrgrowf)(b, es, 0, mc), b, "Rust es={es} mc={mc}");
            }
            // one step past => must grow (different capacity)
            let a2 = (p.c.arrgrowf)(a, es, 0, cap + 1);
            let b2 = (p.rs.arrgrowf)(b, es, 0, cap + 1);
            assert_eq_ctx(
                rd_usize((a2 as *mut u8).sub(HDR_SIZE), HDR_CAPACITY),
                rd_usize((b2 as *mut u8).sub(HDR_SIZE), HDR_CAPACITY),
                &format!("es={es}: one past cap"),
            );
            assert_eq!(rd_usize((a2 as *mut u8).sub(HDR_SIZE), HDR_CAPACITY), 20);
            (p.c.arrfreef)(a2);
            (p.rs.arrfreef)(b2);
        }
    }
}

// ===========================================================================
// row 3 — the exact growth ladder at its boundaries
// ===========================================================================
#[test]
fn err03_arrgrowf_growth_ladder() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // from NULL: cap 0, so `min_cap < 2*0` is never true; only `< 4` applies
        for (al, mc, want) in [
            (1usize, 0usize, 4usize),
            (2, 0, 4),
            (3, 0, 4),
            (4, 0, 4),
            (5, 0, 5),
            (0, 1, 4),
            (0, 3, 4),
            (0, 4, 4),
            (0, 5, 5),
            (2, 3, 4),
            (7, 3, 7),
            (3, 7, 7),
        ] {
            let c = (p.c.arrgrowf)(std::ptr::null_mut(), 8, al, mc);
            let r = (p.rs.arrgrowf)(std::ptr::null_mut(), 8, al, mc);
            let cc = rd_usize((c as *mut u8).sub(HDR_SIZE), HDR_CAPACITY);
            let rc = rd_usize((r as *mut u8).sub(HDR_SIZE), HDR_CAPACITY);
            assert_eq_ctx(cc, rc, &format!("ladder NULL al={al} mc={mc}"));
            assert_eq!(cc, want, "C ladder NULL al={al} mc={mc}");
            (p.c.arrfreef)(c);
            (p.rs.arrfreef)(r);
        }
        // from cap=4: `min_cap < 8` snaps up to 8; `>= 8` is used as-is
        for (mc, want) in [(5usize, 8usize), (6, 8), (7, 8), (8, 8), (9, 9), (100, 100)] {
            let c = (p.c.arrgrowf)(std::ptr::null_mut(), 8, 4, 0);
            let r = (p.rs.arrgrowf)(std::ptr::null_mut(), 8, 4, 0);
            let c2 = (p.c.arrgrowf)(c, 8, 0, mc);
            let r2 = (p.rs.arrgrowf)(r, 8, 0, mc);
            let cc = rd_usize((c2 as *mut u8).sub(HDR_SIZE), HDR_CAPACITY);
            let rc = rd_usize((r2 as *mut u8).sub(HDR_SIZE), HDR_CAPACITY);
            assert_eq_ctx(cc, rc, &format!("ladder cap4 mc={mc}"));
            assert_eq!(cc, want);
            (p.c.arrfreef)(c2);
            (p.rs.arrfreef)(r2);
        }
    }
}

// ===========================================================================
// row 5 — make_hash_index's threshold assert is unreachable (slot_count >= 8)
// ===========================================================================
#[test]
fn err05_make_hash_index_assert_unreachable() {
    // `STBDS_ASSERT(uct + tct < slot_count)` fails only for slot_count <= 2:
    //   sc=1 -> 1+0 < 1 false ; sc=2 -> 2+0 < 2 false ; sc=4 -> 3+0 < 4 ok
    for sc in [1usize, 2] {
        let uct = sc - (sc >> 2);
        let tct = (sc >> 3) + (sc >> 4);
        assert!(uct + tct >= sc, "sc={sc} should be the failing case");
    }
    for sc in [4usize, 8, 16, 32, 64, 128, 1024] {
        let uct = sc - (sc >> 2);
        let tct = (sc >> 3) + (sc >> 4);
        assert!(uct + tct < sc, "sc={sc} must satisfy the assert");
    }
    // No public entry point can produce slot_count < 8:
    //   shmode_func   -> STBDS_BUCKET_LENGTH == 8
    //   hmput_key     -> 8 (first table) or slot_count*2
    //   hmdel_key     -> slot_count>>1, guarded by `slot_count > 8`
    // Verified empirically over a long churn on both implementations.
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut r = Rng::new(0x050005);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        let mut min_sc = usize::MAX;
        for op in 0..3000usize {
            owned.push(ints(r.below(60) as i32));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            if r.below(2) == 0 {
                m.put(k, &r.u32().to_le_bytes(), &format!("op {op}"));
            } else {
                m.del(k, &format!("op {op}"));
            }
            for (mm, tag) in [(&m.c, "C"), (&m.rs, "Rust")] {
                if mm.t.is_null() {
                    continue;
                }
                let tbl = rd_ptr(
                    (mm.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
                    HDR_HASH_TABLE,
                );
                if tbl.is_null() {
                    continue;
                }
                let sc = rd_usize(tbl, HI_SLOT_COUNT);
                min_sc = min_sc.min(sc);
                assert!(sc >= 8, "{tag} op {op}: slot_count dropped to {sc}");
                assert!(sc.is_power_of_two(), "{tag} op {op}: slot_count {sc}");
            }
        }
        assert_eq!(min_sc, 8);
        m.free();
    }
}

// ===========================================================================
// rows 6, 7 — hmfree_func null / no-hash-table
// ===========================================================================
#[test]
fn err06_hmfree_null_is_noop() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [0usize, 1, 8, 4096] {
            (p.c.hmfree_func)(std::ptr::null_mut(), es);
            (p.rs.hmfree_func)(std::ptr::null_mut(), es);
        }
    }
    // reaching here without a crash on either side IS the assertion
}

#[test]
fn err07_hmfree_no_hash_table() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [1usize, 4, 8, 16, 64] {
            // an array built by arrgrowf only: hash_table == NULL
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), es, 3, 0);
            let b = (p.rs.arrgrowf)(std::ptr::null_mut(), es, 3, 0);
            assert!(rd_ptr((a as *mut u8).sub(HDR_SIZE), HDR_HASH_TABLE).is_null());
            assert!(rd_ptr((b as *mut u8).sub(HDR_SIZE), HDR_HASH_TABLE).is_null());
            (p.c.hmfree_func)(a, es);
            (p.rs.hmfree_func)(b, es);
            // and one built by hmput_default (also no hash table)
            let a = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let b = (p.rs.hmput_default)(std::ptr::null_mut(), es);
            assert!(rd_ptr((a as *mut u8).sub(es).sub(HDR_SIZE), HDR_HASH_TABLE).is_null());
            assert!(rd_ptr((b as *mut u8).sub(es).sub(HDR_SIZE), HDR_HASH_TABLE).is_null());
            (p.c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// ===========================================================================
// rows 8, 9 — find_slot misses in the forward and in the wrap-around half
// ===========================================================================
#[test]
fn err08_09_find_slot_miss_both_halves() {
    // A miss returns -1 whichever inner loop spots the empty slot.  Which loop
    // it is depends on `pos & 7`; drive enough randomized misses to cover both,
    // classifying each with the exact probe replay.
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut forward_miss = 0usize;
    let mut wrap_miss = 0usize;
    let mut r = Rng::new(0x080009);
    unsafe {
        for trial in 0..80usize {
            let mut m = MapPair::empty(p, cfg);
            let mut owned: Vec<Vec<u8>> = Vec::new();
            let n = r.range(1, 40);
            let mut present = std::collections::HashSet::new();
            for i in 0..n {
                let kv = r.below(2000) as i32;
                present.insert(kv);
                owned.push(ints(kv));
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                m.put(k, &r.u32().to_le_bytes(), &format!("trial {trial} put {i}"));
            }
            let tbl = rd_ptr(
                (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
                HDR_HASH_TABLE,
            );
            let sc = rd_usize(tbl, HI_SLOT_COUNT);
            let seed = rd_usize(tbl, HI_SEED);
            let storage = rd_ptr(tbl, HI_STORAGE);
            let mut bhash = vec![0usize; sc];
            for b in 0..(sc >> 3) {
                let bp = storage.add(b * BUCKET_SIZE);
                for j in 0..BUCKET_LENGTH {
                    bhash[b * 8 + j] = rd_usize(bp, j * 8);
                }
            }
            for _ in 0..40 {
                let kv = r.below(4000) as i32 + 100_000;
                if present.contains(&kv) {
                    continue;
                }
                let mut kb = ints(kv);
                let idx = m.geti(
                    kb.as_mut_ptr() as *mut c_void,
                    &format!("trial {trial} miss {kv}"),
                );
                assert_eq!(idx, -1, "an absent key must yield the -1 sentinel");
                // classify: which loop found the empty slot?
                let mut h = (p.c.hash_bytes)(kb.as_mut_ptr() as *mut c_void, 4, seed);
                if h < 2 {
                    h += 2;
                }
                match miss_half(h, sc, &bhash) {
                    Some(true) => wrap_miss += 1,
                    Some(false) => forward_miss += 1,
                    None => {}
                }
            }
            m.free();
        }
    }
    eprintln!("miss coverage: forward={forward_miss} wrap={wrap_miss}");
    assert!(forward_miss > 0, "row 8 (forward-half miss) never exercised");
    assert!(wrap_miss > 0, "row 9 (wrap-around-half miss) never exercised");
}

/// Replay the probe loop for a *missing* key: returns `Some(true)` if the
/// terminating `STBDS_HASH_EMPTY` slot is found by the wrap-around inner loop,
/// `Some(false)` if by the forward one.
fn miss_half(hash: usize, sc: usize, bhash: &[usize]) -> Option<bool> {
    let mut pos = hash & (sc - 1);
    let mut step = BUCKET_LENGTH;
    for _ in 0..(4 * sc + 64) {
        let b = pos >> 3;
        for i in (pos & 7)..BUCKET_LENGTH {
            if bhash[b * 8 + i] == 0 {
                return Some(false);
            }
        }
        for i in 0..(pos & 7) {
            if bhash[b * 8 + i] == 0 {
                return Some(true);
            }
        }
        pos = pos.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        pos &= sc - 1;
    }
    None
}

// ===========================================================================
// rows 10, 11, 12, 55 — hmget_key_ts
// ===========================================================================
#[test]
fn err10_hmget_ts_null_table() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [1usize, 4, 8, 16, 64] {
            let mut key = ints(1234);
            let mut ct: isize = 0x5a5a;
            let mut rt: isize = 0x5a5a;
            let a = (p.c.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                &mut ct,
                STBDS_HM_BINARY,
            );
            let b = (p.rs.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                &mut rt,
                STBDS_HM_BINARY,
            );
            assert_eq_ctx(ct, rt, &format!("es={es}: *temp on a NULL map"));
            assert_eq!(ct, -1, "STBDS_INDEX_EMPTY");
            assert!(!a.is_null() && !b.is_null(), "must allocate");
            assert_snap_eq(
                &snap_map(a, es, KeyRepr::Raw),
                &snap_map(b, es, KeyRepr::Raw),
                &format!("es={es}: hmget_key_ts(NULL)"),
            );
            // length 1, element zeroed, no hash table
            let h = (a as *mut u8).sub(es).sub(HDR_SIZE);
            assert_eq!(rd_usize(h, HDR_LENGTH), 1);
            assert!(rd_ptr(h, HDR_HASH_TABLE).is_null());
            assert_eq!(
                std::slice::from_raw_parts((a as *mut u8).sub(es), es),
                vec![0u8; es].as_slice()
            );
            (p.c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn err11_hmget_ts_no_hash_table() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [4usize, 8, 16] {
            // hmput_default gives an array with length 1 and NO hash table
            let a = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let b = (p.rs.hmput_default)(std::ptr::null_mut(), es);
            let mut key = ints(99);
            let mut ct: isize = 0x1234;
            let mut rt: isize = 0x1234;
            let a2 = (p.c.hmget_key_ts)(a, es, key.as_mut_ptr() as *mut c_void, 4, &mut ct, 0);
            let b2 = (p.rs.hmget_key_ts)(b, es, key.as_mut_ptr() as *mut c_void, 4, &mut rt, 0);
            assert_eq_ctx(ct, rt, &format!("es={es}: *temp with hash_table == NULL"));
            assert_eq!(ct, -1);
            assert_eq!(a2, a, "must return `a` unchanged");
            assert_eq!(b2, b, "must return `a` unchanged");
            // header->temp is NOT written by the _ts variant
            assert_eq_ctx(
                rd_isize((a as *mut u8).sub(es).sub(HDR_SIZE), HDR_TEMP),
                rd_isize((b as *mut u8).sub(es).sub(HDR_SIZE), HDR_TEMP),
                "header->temp untouched",
            );
            (p.c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn err12_hmget_ts_key_absent() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for i in 0..20i32 {
            owned.push(ints(i * 2));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            m.put(k, &(i as u32).to_le_bytes(), &format!("put {i}"));
        }
        for i in 0..20i32 {
            let mut kb = ints(i * 2 + 1);
            let t = m.geti_ts(kb.as_mut_ptr() as *mut c_void, &format!("absent {i}"));
            assert_eq!(t, -1, "absent key must set *temp = STBDS_INDEX_EMPTY");
        }
        m.free();
    }
}

#[test]
fn err55_hmget_ts_temp_written_through_caller_pointer() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // a boxed target proves the callee writes through the caller's pointer
        let mut ct = Box::new(0x7fff_ffff_ffff_ffffisize);
        let mut rt = Box::new(0x7fff_ffff_ffff_ffffisize);
        let mut key = ints(5);
        let a = (p.c.hmget_key_ts)(
            std::ptr::null_mut(),
            8,
            key.as_mut_ptr() as *mut c_void,
            4,
            &mut *ct,
            0,
        );
        let b = (p.rs.hmget_key_ts)(
            std::ptr::null_mut(),
            8,
            key.as_mut_ptr() as *mut c_void,
            4,
            &mut *rt,
            0,
        );
        assert_eq_ctx(*ct, *rt, "*temp through a heap pointer");
        assert_eq!(*ct, -1);
        (p.c.hmfree_func)((a as *mut u8).sub(8) as *mut c_void, 8);
        (p.rs.hmfree_func)((b as *mut u8).sub(8) as *mut c_void, 8);
    }
}

// ===========================================================================
// rows 13, 14 — hmget_key writes header->temp (unlike the _ts variant)
// ===========================================================================
#[test]
fn err13_hmget_key_null() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [1usize, 4, 8, 16, 64] {
            let mut key = ints(7);
            let a = (p.c.hmget_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                0,
            );
            let b = (p.rs.hmget_key)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                4,
                0,
            );
            assert_snap_eq(
                &snap_map(a, es, KeyRepr::Raw),
                &snap_map(b, es, KeyRepr::Raw),
                &format!("es={es}: hmget_key(NULL)"),
            );
            assert_eq_ctx(
                rd_isize((a as *mut u8).sub(es).sub(HDR_SIZE), HDR_TEMP),
                rd_isize((b as *mut u8).sub(es).sub(HDR_SIZE), HDR_TEMP),
                &format!("es={es}: header->temp"),
            );
            assert_eq!(rd_isize((a as *mut u8).sub(es).sub(HDR_SIZE), HDR_TEMP), -1);
            (p.c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn err14_hmget_key_absent_temp() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for i in 0..30i32 {
            owned.push(ints(i * 3));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            m.put(k, &(i as u32).to_le_bytes(), &format!("put {i}"));
        }
        for i in 0..40i32 {
            let mut kb = ints(i * 3 + 1);
            let idx = m.geti(kb.as_mut_ptr() as *mut c_void, &format!("absent {i}"));
            assert_eq!(idx, -1, "header->temp must be -1 for an absent key");
        }
        m.free();
    }
}

// ===========================================================================
// rows 15, 16, 17 — hmput_default
// ===========================================================================
#[test]
fn err15_16_17_hmput_default() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [1usize, 4, 8, 16, 64] {
            // row 15: a == NULL
            let a = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let b = (p.rs.hmput_default)(std::ptr::null_mut(), es);
            assert_snap_eq(
                &snap_map(a, es, KeyRepr::Raw),
                &snap_map(b, es, KeyRepr::Raw),
                &format!("es={es}: hmput_default(NULL)"),
            );
            assert_eq!(rd_usize((a as *mut u8).sub(es).sub(HDR_SIZE), HDR_LENGTH), 1);

            // row 17: a != NULL and length != 0 => the identical pointer back
            assert_eq!((p.c.hmput_default)(a, es), a, "es={es}: C no-op");
            assert_eq!((p.rs.hmput_default)(b, es), b, "es={es}: Rust no-op");
            let before_c = snap_map(a, es, KeyRepr::Raw);
            let before_r = snap_map(b, es, KeyRepr::Raw);
            for _ in 0..5 {
                assert_eq!((p.c.hmput_default)(a, es), a);
                assert_eq!((p.rs.hmput_default)(b, es), b);
            }
            assert_snap_eq(&before_c, &snap_map(a, es, KeyRepr::Raw), "C untouched");
            assert_snap_eq(&before_r, &snap_map(b, es, KeyRepr::Raw), "Rust untouched");
            (p.c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);

            // row 16: a != NULL but header->length == 0
            let ra = (p.c.arrgrowf)(std::ptr::null_mut(), es, 4, 0);
            let rb = (p.rs.arrgrowf)(std::ptr::null_mut(), es, 4, 0);
            assert_eq!(rd_usize((ra as *mut u8).sub(HDR_SIZE), HDR_LENGTH), 0);
            // fabricate the `t` pointer a caller would hold
            let ta = (ra as *mut u8).add(es) as *mut c_void;
            let tb = (rb as *mut u8).add(es) as *mut c_void;
            let a2 = (p.c.hmput_default)(ta, es);
            let b2 = (p.rs.hmput_default)(tb, es);
            assert_snap_eq(
                &snap_map(a2, es, KeyRepr::Raw),
                &snap_map(b2, es, KeyRepr::Raw),
                &format!("es={es}: hmput_default(length==0)"),
            );
            assert_eq!(rd_usize((a2 as *mut u8).sub(es).sub(HDR_SIZE), HDR_LENGTH), 1);
            (p.c.hmfree_func)((a2 as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b2 as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// ===========================================================================
// rows 18, 19, 20 — hmput_key bootstrap, first table, grow threshold
// ===========================================================================
#[test]
fn err18_19_20_hmput_key_bootstrap_and_grow() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for &mode in &[STBDS_HM_BINARY, STBDS_HM_STRING] {
            let es = 16usize;
            let ks = 8usize;
            let mut key: Vec<u8> = b"abcdefg\0".to_vec();
            let a = (p.c.hmput_key)(std::ptr::null_mut(), es, key.as_mut_ptr() as *mut c_void, ks, mode);
            let b = (p.rs.hmput_key)(std::ptr::null_mut(), es, key.as_mut_ptr() as *mut c_void, ks, mode);
            // hmput_key only initialises the first `ks` bytes of the element;
            // write the "value" half too, so the snapshot never compares the
            // indeterminate padding the C leaves behind.
            for t in [a, b] {
                let idx = rd_isize((t as *mut u8).sub(es).sub(HDR_SIZE), HDR_TEMP);
                let e = (t as *mut u8).offset(idx * es as isize);
                std::ptr::write_bytes(e.add(ks), 0xA7, es - ks);
            }
            let repr = if mode >= STBDS_HM_STRING {
                KeyRepr::StrPtr { keyoffset: 0 }
            } else {
                KeyRepr::Raw
            };
            assert_snap_eq(
                &snap_map(a, es, repr),
                &snap_map(b, es, repr),
                &format!("mode={mode}: hmput_key(NULL)"),
            );
            // row 19: slot_count == 8 and string.mode is derived from `mode`
            for (t, tag) in [(a, "C"), (b, "Rust")] {
                let tbl = rd_ptr((t as *mut u8).sub(es).sub(HDR_SIZE), HDR_HASH_TABLE);
                assert_eq!(rd_usize(tbl, HI_SLOT_COUNT), 8, "{tag} mode={mode}");
                assert_eq!(rd_usize(tbl, HI_USED_COUNT), 1, "{tag} mode={mode}");
                assert_eq!(
                    rd_u8(tbl.add(HI_STRING), ARENA_MODE),
                    if mode >= STBDS_HM_STRING {
                        STBDS_SH_DEFAULT
                    } else {
                        STBDS_SH_NONE
                    },
                    "{tag} mode={mode}: nt->string.mode"
                );
            }
            (p.c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }

    // row 20: the grow happens exactly when used_count reaches sc - (sc>>2)
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        let mut seen: Vec<(usize, usize)> = Vec::new(); // (n_entries, slot_count)
        for i in 0..60i32 {
            owned.push(ints(i));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            m.put(k, &(i as u32).to_le_bytes(), &format!("put {i}"));
            let tbl = rd_ptr(
                (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
                HDR_HASH_TABLE,
            );
            seen.push((i as usize + 1, rd_usize(tbl, HI_SLOT_COUNT)));
        }
        // used_count >= threshold is checked BEFORE the insert, so the table
        // doubles on the insert *after* used_count hits the threshold:
        // 8 slots hold 6 -> the 7th put grows to 16; 16 holds 12 -> 13th -> 32
        assert_eq!(seen[5], (6, 8), "6 entries still fit 8 slots");
        assert_eq!(seen[6], (7, 16), "the 7th put must double to 16");
        assert_eq!(seen[11], (12, 16));
        assert_eq!(seen[12], (13, 32), "the 13th put must double to 32");
        assert_eq!(seen[23], (24, 32));
        assert_eq!(seen[24], (25, 64), "the 25th put must double to 64");
        m.free();
    }
}

// ===========================================================================
// row 22 — duplicate put
// ===========================================================================
#[test]
fn err22_hmput_key_duplicate() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for i in 0..5i32 {
            owned.push(ints(i));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            m.put(k, &(i as u32).to_le_bytes(), &format!("put {i}"));
        }
        let len_before = m.hmlen("before dup");
        let tbl_c = rd_ptr(
            (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
            HDR_HASH_TABLE,
        );
        let used_before = rd_usize(tbl_c, HI_USED_COUNT);
        for i in 0..5i32 {
            owned.push(ints(i));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            let idx = m.put(k, &(0xff00 + i as u32).to_le_bytes(), &format!("dup {i}"));
            assert_eq!(idx, i as isize, "dup put must resolve to the original index");
        }
        assert_eq!(m.hmlen("after dup"), len_before, "length must not change");
        let tbl_c = rd_ptr(
            (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
            HDR_HASH_TABLE,
        );
        assert_eq!(
            rd_usize(tbl_c, HI_USED_COUNT),
            used_before,
            "used_count must not change"
        );
        m.free();
    }
}

// ===========================================================================
// row 23 — an insert reuses a tombstoned slot
// ===========================================================================
#[test]
fn err23_hmput_key_reuses_tombstone() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut reused = 0usize;
    let mut r = Rng::new(0x230023);
    unsafe {
        for trial in 0..120usize {
            let mut m = MapPair::empty(p, cfg);
            let mut owned: Vec<Vec<u8>> = Vec::new();
            // build up, delete some (creating tombstones), then insert again
            for i in 0..14i32 {
                owned.push(ints(i));
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                m.put(k, &(i as u32).to_le_bytes(), &format!("t{trial} put {i}"));
            }
            for i in [1i32, 4, 9] {
                owned.push(ints(i));
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                m.del(k, &format!("t{trial} del {i}"));
            }
            let tbl = rd_ptr(
                (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
                HDR_HASH_TABLE,
            );
            let tomb_before = rd_usize(tbl, HI_TOMBSTONE_COUNT);
            for i in 0..6 {
                let kv = 100_000 + (r.below(1_000_000) as i32);
                owned.push(ints(kv));
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                m.put(k, &r.u32().to_le_bytes(), &format!("t{trial} refill {i}"));
            }
            let tbl = rd_ptr(
                (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
                HDR_HASH_TABLE,
            );
            let tomb_after = rd_usize(tbl, HI_TOMBSTONE_COUNT);
            if tomb_before > 0 && tomb_after < tomb_before {
                reused += 1;
            }
            m.free();
        }
    }
    eprintln!("tombstone reuse observed in {reused} trials");
    assert!(reused > 0, "never observed a tombstone being reused");
}

// ===========================================================================
// rows 24, 25, 26, 57, 58 — hmdel_key rejections
// ===========================================================================
#[test]
fn err24_hmdel_null_returns_null() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [0usize, 1, 8, 64] {
            for &mode in &[-1i32, 0, 1, 2, i32::MAX, i32::MIN] {
                for &ks in &[0usize, 4, 8] {
                    for &ko in &[0usize, 4, 1000] {
                        let mut key = ints(1);
                        let a = (p.c.hmdel_key)(
                            std::ptr::null_mut(),
                            es,
                            key.as_mut_ptr() as *mut c_void,
                            ks,
                            ko,
                            mode,
                        );
                        let b = (p.rs.hmdel_key)(
                            std::ptr::null_mut(),
                            es,
                            key.as_mut_ptr() as *mut c_void,
                            ks,
                            ko,
                            mode,
                        );
                        assert_eq_ctx(
                            (a.is_null(), b.is_null()),
                            (true, true),
                            &format!("hmdel_key(NULL, es={es}, ks={ks}, ko={ko}, mode={mode})"),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn err25_hmdel_no_hash_table() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [4usize, 8, 16] {
            let a = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let b = (p.rs.hmput_default)(std::ptr::null_mut(), es);
            // pre-set header->temp to something non-zero so we can see the store
            (((a as *mut u8).sub(es).sub(HDR_SIZE).add(HDR_TEMP)) as *mut isize)
                .write_unaligned(1234);
            (((b as *mut u8).sub(es).sub(HDR_SIZE).add(HDR_TEMP)) as *mut isize)
                .write_unaligned(1234);
            let mut key = ints(5);
            let a2 = (p.c.hmdel_key)(a, es, key.as_mut_ptr() as *mut c_void, 4, 0, 0);
            let b2 = (p.rs.hmdel_key)(b, es, key.as_mut_ptr() as *mut c_void, 4, 0, 0);
            assert_eq!(a2, a, "must return `a`");
            assert_eq!(b2, b, "must return `a`");
            assert_eq_ctx(
                rd_isize((a as *mut u8).sub(es).sub(HDR_SIZE), HDR_TEMP),
                rd_isize((b as *mut u8).sub(es).sub(HDR_SIZE), HDR_TEMP),
                &format!("es={es}: stbds_temp set to 0"),
            );
            assert_eq!(rd_isize((a as *mut u8).sub(es).sub(HDR_SIZE), HDR_TEMP), 0);
            (p.c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn err26_hmdel_key_absent() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for i in 0..20i32 {
            owned.push(ints(i * 2));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            m.put(k, &(i as u32).to_le_bytes(), &format!("put {i}"));
        }
        let len_before = m.hmlen("before");
        let tbl = rd_ptr(
            (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
            HDR_HASH_TABLE,
        );
        let used_before = rd_usize(tbl, HI_USED_COUNT);
        let tomb_before = rd_usize(tbl, HI_TOMBSTONE_COUNT);
        for i in 0..20i32 {
            let mut kb = ints(i * 2 + 1);
            let rc = m.del(kb.as_mut_ptr() as *mut c_void, &format!("absent {i}"));
            assert_eq!(rc, 0, "deleting an absent key must yield the 0 sentinel");
        }
        assert_eq!(m.hmlen("after"), len_before);
        let tbl = rd_ptr(
            (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
            HDR_HASH_TABLE,
        );
        assert_eq!(rd_usize(tbl, HI_USED_COUNT), used_before);
        assert_eq!(rd_usize(tbl, HI_TOMBSTONE_COUNT), tomb_before);
        m.free();
    }
}

#[test]
fn err57_hmdel_last_element() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    unsafe {
        for n in [1usize, 2, 5, 13, 30] {
            let mut m = MapPair::empty(p, cfg);
            let mut owned: Vec<Vec<u8>> = Vec::new();
            for i in 0..n {
                owned.push(ints(i as i32));
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                m.put(k, &(i as u32).to_le_bytes(), &format!("n={n} put {i}"));
            }
            // reverse order => old_index == final_index every time, so the
            // memmove / re-lookup / assert block is skipped
            for i in (0..n).rev() {
                let mut kb = ints(i as i32);
                let rc = m.del(kb.as_mut_ptr() as *mut c_void, &format!("n={n} del last {i}"));
                assert_eq!(rc, 1);
            }
            assert_eq!(m.hmlen("drained"), 0);
            m.free();
        }
    }
}

#[test]
fn err58_hmdel_twice() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        owned.push(ints(42));
        let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
        m.put(k, &7u32.to_le_bytes(), "put 42");
        let mut kb = ints(42);
        assert_eq!(m.del(kb.as_mut_ptr() as *mut c_void, "del once"), 1);
        assert_eq!(m.hmlen("after first del"), 0);
        for i in 0..5 {
            assert_eq!(
                m.del(kb.as_mut_ptr() as *mut c_void, &format!("del again {i}")),
                0,
                "deleting an already-deleted key must yield 0"
            );
            assert_eq!(m.hmlen("after repeat del"), 0);
        }
        m.free();
    }
}

// ===========================================================================
// rows 21, 27, 28 — the tautological / unreachable asserts in hmdel_key
// ===========================================================================
#[test]
fn err21_27_28_hmdel_and_hmput_invariant_asserts() {
    // row 21: `(size_t)i+1 <= arrcap(a)` -- arrgrowf(a,es,1,0) guarantees
    //         capacity >= length+1, so this can never fire.
    // row 27: `slot < (ptrdiff_t)table->slot_count` -- find_slot returns
    //         `(pos & ~7) + i` with `pos &= slot_count-1` and `i < 8`.
    // row 28: `table->used_count >= 0` -- used_count is `size_t`, a tautology.
    // All three are verified as *invariants* over a long churn.
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut r = Rng::new(0x212728);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for op in 0..4000usize {
            owned.push(ints(r.below(90) as i32));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            if r.below(2) == 0 {
                m.put(k, &r.u32().to_le_bytes(), &format!("op {op}"));
            } else {
                m.del(k, &format!("op {op}"));
            }
            for (mm, tag) in [(&m.c, "C"), (&m.rs, "Rust")] {
                if mm.t.is_null() {
                    continue;
                }
                let h = (mm.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE);
                let len = rd_usize(h, HDR_LENGTH);
                let cap = rd_usize(h, HDR_CAPACITY);
                assert!(len <= cap, "{tag} op {op}: row 21 invariant len {len} > cap {cap}");
                let tbl = rd_ptr(h, HDR_HASH_TABLE);
                if tbl.is_null() {
                    continue;
                }
                let sc = rd_usize(tbl, HI_SLOT_COUNT);
                let used = rd_usize(tbl, HI_USED_COUNT);
                let tomb = rd_usize(tbl, HI_TOMBSTONE_COUNT);
                // row 28: used_count is size_t; it must never have wrapped
                assert!(used <= sc, "{tag} op {op}: used_count {used} > slot_count {sc}");
                assert!(tomb <= sc, "{tag} op {op}: tombstone_count {tomb} > {sc}");
                assert_eq!(used, len - 1, "{tag} op {op}: used_count vs length");
                // row 27: every stored slot index is inside the table
                let storage = rd_ptr(tbl, HI_STORAGE);
                for b in 0..(sc >> 3) {
                    let bp = storage.add(b * BUCKET_SIZE);
                    for j in 0..BUCKET_LENGTH {
                        let idx = rd_isize(bp, 64 + j * 8);
                        assert!(
                            idx >= -2 && idx < (len as isize) - 1,
                            "{tag} op {op}: bucket index {idx} out of range (len {len})"
                        );
                    }
                }
            }
        }
        m.free();
    }
}

// ===========================================================================
// row 31 — hmdel_key frees the strdup'd key only for mode == 1 exactly
// ===========================================================================
#[test]
fn err31_hmdel_mode_exactly_1_frees() {
    // mode == 1 + SH_STRDUP => the key is freed.
    // mode >= 2 + SH_STRDUP => it is NOT freed (`mode == STBDS_HM_STRING` fails).
    // Both must behave identically in C and Rust; deleting in reverse order
    // avoids the `mode >= 2` swap-with-last abort (ERRORS row 29).
    for &mode in &[1i32, 2, 3, 77] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg {
            elemsize: 16,
            keysize: 8,
            keyoffset: 0,
            mode,
            valoffset: 8,
            valsize: 8,
            force_raw_snap: false,
        };
        let mut m = MapPair::with_shmode(p, cfg, STBDS_SH_STRDUP as i32);
        let mut owned: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for i in 0..10usize {
                let mut s = format!("dupkey_{i}").into_bytes();
                s.push(0);
                owned.push(s);
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                m.put(k, &(i as u64).to_le_bytes(), &format!("mode={mode} put {i}"));
            }
            for i in (0..10usize).rev() {
                let mut s = format!("dupkey_{i}").into_bytes();
                s.push(0);
                owned.push(s);
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                assert_eq!(
                    m.del(k, &format!("mode={mode} del {i}")),
                    1,
                    "mode={mode}: del {i}"
                );
            }
            m.free();
        }
    }
}

// ===========================================================================
// rows 32, 33 — shrink and tombstone rebuild
// ===========================================================================
#[test]
fn err32_33_hmdel_shrink_and_rebuild() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    let mut shrinks = 0usize;
    let mut rebuilds = 0usize;
    unsafe {
        for i in 0..60i32 {
            owned.push(ints(i));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            m.put(k, &(i as u32).to_le_bytes(), &format!("put {i}"));
        }
        let mut prev_sc = table_field(&m.c, cfg.elemsize, HI_SLOT_COUNT).unwrap();
        let mut prev_tomb = table_field(&m.c, cfg.elemsize, HI_TOMBSTONE_COUNT).unwrap();
        for i in 0..60i32 {
            owned.push(ints(i));
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            assert_eq!(m.del(k, &format!("del {i}")), 1);
            let sc = table_field(&m.c, cfg.elemsize, HI_SLOT_COUNT).unwrap();
            let tomb = table_field(&m.c, cfg.elemsize, HI_TOMBSTONE_COUNT).unwrap();
            let used = table_field(&m.c, cfg.elemsize, HI_USED_COUNT).unwrap();
            let ucst = table_field(&m.c, cfg.elemsize, HI_USED_COUNT_SHRINK_THRESHOLD).unwrap();
            if sc < prev_sc {
                shrinks += 1;
                assert_eq!(sc, prev_sc >> 1, "a shrink must halve slot_count");
                assert!(prev_sc > 8, "shrink only happens above 8 slots");
                assert_eq!(tomb, 0, "a rebuilt index has no tombstones");
            } else if tomb == 0 && prev_tomb > 0 {
                rebuilds += 1;
                assert_eq!(sc, prev_sc, "a rebuild keeps slot_count");
            }
            let _ = (used, ucst);
            prev_sc = sc;
            prev_tomb = tomb;
        }
        eprintln!("shrinks={shrinks} rebuilds={rebuilds}");
        assert!(shrinks > 0, "row 32 (shrink) never exercised");
        assert!(rebuilds > 0, "row 33 (tombstone rebuild) never exercised");
        m.free();
    }
}

unsafe fn table_field(m: &Map, elemsize: usize, off: usize) -> Option<usize> {
    if m.t.is_null() {
        return None;
    }
    let tbl = rd_ptr((m.t as *mut u8).sub(elemsize).sub(HDR_SIZE), HDR_HASH_TABLE);
    if tbl.is_null() {
        None
    } else {
        Some(rd_usize(tbl, off))
    }
}

// ===========================================================================
// row 31 (leak half) — `hmdel_key` frees the strdup'd key ONLY for `mode == 1`
// ===========================================================================
/// The `mode == STBDS_HM_STRING` test at `lib.c:836` means a `mode >= 2` delete
/// *leaks* the `stbds_strdup`'d key. A leak has no return value, so it is
/// detected through the allocator: glibc's tcache is LIFO per size class, so if
/// the key's chunk was freed the very next `stbds_strdup` of the same length
/// gets that exact address back; if it was leaked, it does not.
///
/// The key length is chosen so its size class collides with nothing else the
/// library allocates (an array header block or a `stbds_hash_index`).
fn strdup_chunk_recycled(lib: &Lib, mode: i32) -> bool {
    const ES: usize = 16;
    const KS: usize = 8;
    const KEYLEN: usize = 100; // -> a 101-byte malloc, a size class of its own

    // IMPORTANT: every key buffer is allocated UP FRONT. If the test allocated a
    // 101-byte buffer between the free inside `hmdel_key` and the `stbds_strdup`
    // inside the following `hmput_key`, the test itself would take the recycled
    // chunk and the detector would always report "not recycled".
    let mut owned: Vec<Vec<u8>> = Vec::with_capacity(8);
    for tag in [b'a', b'b', b'c', b'c', b'd'] {
        let mut v = Vec::with_capacity(KEYLEN + 1);
        v.extend(std::iter::repeat(tag).take(KEYLEN));
        v.push(0);
        owned.push(v);
    }
    let ptrs: Vec<*mut c_void> = owned
        .iter_mut()
        .map(|v| v.as_mut_ptr() as *mut c_void)
        .collect();

    unsafe {
        let mut t = (lib.shmode_func)(ES, STBDS_SH_STRDUP as i32);
        // three entries; the LAST one is deleted, so old_index == final_index
        // and the `mode >= 2` swap-with-last abort path is avoided
        for i in 0..3 {
            t = (lib.hmput_key)(t, ES, ptrs[i], KS, mode);
        }
        let hdr = (t as *mut u8).sub(ES).sub(HDR_SIZE);
        let last_idx = rd_isize(hdr, HDR_TEMP);
        let p_last = rd_ptr((t as *mut u8).offset(last_idx * ES as isize), 0);

        // delete it: for mode == 1 the strdup'd key is freed, for mode >= 2 not
        t = (lib.hmdel_key)(t, ES, ptrs[3], KS, 0, mode);

        // the very next stbds_strdup of the same length -- no intervening
        // allocation of this size class
        t = (lib.hmput_key)(t, ES, ptrs[4], KS, mode);
        let hdr = (t as *mut u8).sub(ES).sub(HDR_SIZE);
        let new_idx = rd_isize(hdr, HDR_TEMP);
        let p_new = rd_ptr((t as *mut u8).offset(new_idx * ES as isize), 0);

        let recycled = p_new == p_last;
        (lib.hmfree_func)((t as *mut u8).sub(ES) as *mut c_void, ES);
        recycled
    }
}

#[test]
fn err31b_hmdel_strdup_free_only_for_mode_1() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    // mode == 1 exactly: the key IS freed, so the chunk comes straight back
    let c1 = strdup_chunk_recycled(&p.c, 1);
    let r1 = strdup_chunk_recycled(&p.rs, 1);
    assert_eq_ctx(c1, r1, "mode=1: strdup'd key freed by hmdel_key?");
    assert!(
        c1,
        "sanity: for mode == 1 the C frees the key, so glibc's tcache must \
         hand the same chunk back (if this fails the detector needs revisiting)"
    );
    // mode >= 2: `mode == STBDS_HM_STRING` is false, so the key is LEAKED
    for mode in [2i32, 3, 100, i32::MAX] {
        let c = strdup_chunk_recycled(&p.c, mode);
        let r = strdup_chunk_recycled(&p.rs, mode);
        assert_eq_ctx(c, r, &format!("mode={mode}: strdup'd key freed?"));
        assert!(
            !c,
            "mode={mode}: the C must NOT free the key (it checks `mode == 1`), \
             so the chunk must not be recycled"
        );
    }
}
