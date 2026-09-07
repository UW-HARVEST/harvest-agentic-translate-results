//! Phase C — one differential test per row of `ERRORS.md` (the rows not
//! already covered in `hashmap_binary.rs` / `arena.rs`).
//!
//! Every test constructs the exact invalid input / boundary condition, calls
//! BOTH libraries through their `.so` exports, and asserts they reject
//! identically -- the same sentinel value, the same NULL return, or the same
//! fatal signal.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

const SEED: u64 = 0x5EED_1234;

fn bkey(v: u64, n: usize) -> Vec<u8> {
    let b = v.to_le_bytes();
    (0..n).map(|i| b[i % 8]).collect()
}

// ===========================================================================
// Rows 1-4 — stbds_arrgrowf
// ===========================================================================

#[test]
fn err_01_arrgrowf_noop() {
    let h = setup(0x3141_5926);
    for elemsize in [1usize, 4, 8, 16] {
        unsafe {
            let mut ac = (h.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 100);
            let mut ar = (h.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 100);
            for min_cap in [0usize, 1, 50, 99, 100] {
                let pc = ac;
                let pr = ar;
                ac = (h.c.arrgrowf)(ac, elemsize, 0, min_cap);
                ar = (h.r.arrgrowf)(ar, elemsize, 0, min_cap);
                assert_eq!(pc, ac, "C must return `a` unchanged (min_cap={})", min_cap);
                assert_eq!(pr, ar, "Rust must return `a` unchanged (min_cap={})", min_cap);
                assert_same(
                    "arrgrowf no-op",
                    &describe_arr(ac, elemsize, 0),
                    &describe_arr(ar, elemsize, 0),
                );
            }
            (h.c.arrfreef)(ac);
            (h.r.arrfreef)(ar);
        }
    }
}

#[test]
fn err_02_arrgrowf_zero_zero() {
    let h = setup(0x3141_5926);
    for elemsize in [0usize, 1, 4, 8, 16, 4096] {
        unsafe {
            // (addlen, min_cap) == (0, 0): `min_cap <= stbds_arrcap(NULL)` holds,
            // so the C returns `a` (NULL) without allocating anything.
            let ac = (h.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let ar = (h.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(ac.is_null(), "C arrgrowf(NULL,{},0,0) must return NULL", elemsize);
            assert!(ar.is_null(), "Rust arrgrowf(NULL,{},0,0) must return NULL", elemsize);

            // The smallest request that does allocate.
            let ac = (h.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            let ar = (h.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            assert!(!ac.is_null() && !ar.is_null());
            assert_same(
                &format!("arrgrowf(NULL,{},0,1)", elemsize),
                &describe_arr(ac, elemsize, 0),
                &describe_arr(ar, elemsize, 0),
            );
            let hd = &*header_of(ac);
            assert_eq!(
                (hd.length, hd.capacity, hd.temp),
                (0, 4, 0),
                "min_cap must be forced up to 4"
            );
            assert!(hd.hash_table.is_null());
            (h.c.arrfreef)(ac);
            (h.r.arrfreef)(ar);

            // addlen alone also forces an allocation.
            let ac = (h.c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            let ar = (h.r.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            assert_same(
                &format!("arrgrowf(NULL,{},1,0)", elemsize),
                &describe_arr(ac, elemsize, 0),
                &describe_arr(ar, elemsize, 0),
            );
            (h.c.arrfreef)(ac);
            (h.r.arrfreef)(ar);
        }
    }
}

#[test]
fn err_03_arrgrowf_elemsize_zero() {
    let h = setup(0x3141_5926);
    for (addlen, min_cap) in [(0usize, 0usize), (0, 1), (0, 9), (3, 0), (7, 2), (0, 10_000)] {
        unsafe {
            let ac = (h.c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            let ar = (h.r.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            assert_eq!(
                ac.is_null(),
                ar.is_null(),
                "NULL-ness mismatch for add={} min={}",
                addlen,
                min_cap
            );
            assert_same(
                &format!("arrgrowf elemsize=0 add={} min={}", addlen, min_cap),
                &describe_arr(ac, 0, 0),
                &describe_arr(ar, 0, 0),
            );
            if !ac.is_null() {
                (h.c.arrfreef)(ac);
                (h.r.arrfreef)(ar);
            }
        }
    }
}

#[test]
fn err_04_arrgrowf_overflow() {
    // An allocation size that realloc cannot satisfy: `b = NULL + 32` is then
    // dereferenced by `stbds_header(b)->length = 0`. Both libraries must die
    // the same way.
    let h = setup(0x3141_5926);
    for (elemsize, min_cap) in [
        (1usize, usize::MAX / 2),
        (1, usize::MAX - 64),
        (1 << 40, 1 << 20),
    ] {
        let dc = run_in_child(|| unsafe {
            (h.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
        });
        let dr = run_in_child(|| unsafe {
            (h.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
        });
        assert_eq!(
            dc, dr,
            "arrgrowf overflow death mismatch (elemsize={}, min_cap={})",
            elemsize, min_cap
        );
    }
}

// ===========================================================================
// Rows 5-14 — hmfree / hmget / hmput_default / hmput_key entry conditions
// ===========================================================================

#[test]
fn err_05_hmfree_null() {
    let h = setup(0x3141_5926);
    for elemsize in [0usize, 1, 8, 16] {
        // must be a silent no-op, not a crash
        let dc = run_in_child(|| unsafe { (h.c.hmfree_func)(std::ptr::null_mut(), elemsize) });
        let dr = run_in_child(|| unsafe { (h.r.hmfree_func)(std::ptr::null_mut(), elemsize) });
        assert_eq!(dc, Death::Exited(0));
        assert_eq!(dr, Death::Exited(0));
        unsafe {
            (h.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (h.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

#[test]
fn err_06_get_missing_key() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 6);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let present: Vec<Vec<u8>> = (0..40u64).map(|i| bkey(i * 977, 4)).collect();
    for (i, kb) in present.iter().enumerate() {
        let k = keys.add_bytes(kb);
        m.put(k, kb, HM_BINARY, i as u8);
    }
    let mut tested = 0;
    for _ in 0..2000 {
        let kb = bkey(rng.next_u64(), 4);
        if present.contains(&kb) {
            continue;
        }
        let k = keys.add_bytes(&kb);
        assert_eq!(m.get(k, HM_BINARY), STBDS_INDEX_EMPTY, "must be -1");
        assert_eq!(m.get_ts(k, HM_BINARY), STBDS_INDEX_EMPTY);
        tested += 1;
    }
    assert!(tested > 100);
    m.check("get missing");
    m.free();
}

#[test]
fn err_07_get_ts_null_a() {
    let h = setup(0x3141_5926);
    for elemsize in [1usize, 8, 16, 32] {
        for mode in [HM_BINARY, HM_STRING, -1, 5] {
            let mut keys = Keys::new();
            let k = keys.add_str("anything");
            unsafe {
                let mut tc: isize = 12345;
                let mut tr: isize = 12345;
                let mc = (h.c.hmget_key_ts)(std::ptr::null_mut(), elemsize, k, 4, &mut tc, mode);
                let mr = (h.r.hmget_key_ts)(std::ptr::null_mut(), elemsize, k, 4, &mut tr, mode);
                assert_eq!(tc, STBDS_INDEX_EMPTY, "C temp must be -1");
                assert_eq!(tr, STBDS_INDEX_EMPTY, "Rust temp must be -1");
                assert!(!mc.is_null() && !mr.is_null(), "must return a fresh array");
                let dc = describe_map(mc, elemsize, KeyRepr::Bytes);
                let dr = describe_map(mr, elemsize, KeyRepr::Bytes);
                assert_same(&format!("get_ts(NULL) e={} mode={}", elemsize, mode), &dc, &dr);
                (h.c.hmfree_func)((mc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (h.r.hmfree_func)((mr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

#[test]
fn err_08_get_ts_no_table() {
    // a != NULL but header->hash_table == 0 (an array made by hmput_default)
    let h = setup(0x3141_5926);
    for elemsize in [8usize, 16] {
        let mut m = MapPair::new(&h.c, &h.r, elemsize, 4, false);
        m.put_default();
        unsafe {
            for mm in [m.mc, m.mr] {
                let hd = &*header_of((mm as *mut u8).sub(elemsize) as *mut c_void);
                assert!(hd.hash_table.is_null(), "precondition: no hash table");
            }
        }
        let mut keys = Keys::new();
        let kb = bkey(7, 4);
        let k = keys.add_bytes(&kb);
        let pc = m.mc;
        let pr = m.mr;
        assert_eq!(m.get_ts(k, HM_BINARY), -1);
        assert_eq!(m.mc, pc, "C must return `a` unchanged");
        assert_eq!(m.mr, pr, "Rust must return `a` unchanged");
        m.check("get_ts no table");
        m.free();
    }
}

#[test]
fn err_09_get_key_sentinel() {
    // stbds_hmget_key additionally stores the sentinel into header->temp
    let h = setup(0x3141_5926);
    let elemsize = 16usize;
    // (a) fresh map
    let mut m = MapPair::new(&h.c, &h.r, elemsize, 4, false);
    let mut keys = Keys::new();
    let kb = bkey(1, 4);
    let k = keys.add_bytes(&kb);
    assert_eq!(m.get(k, HM_BINARY), -1);
    m.check("get_key sentinel fresh");
    // (b) no table
    let mut m2 = MapPair::new(&h.c, &h.r, elemsize, 4, false);
    m2.put_default();
    assert_eq!(m2.get(k, HM_BINARY), -1);
    m2.check("get_key sentinel no table");
    // (c) populated but missing
    let ks: Vec<Vec<u8>> = (0..20u64).map(|i| bkey(100 + i, 4)).collect();
    for (i, kk) in ks.iter().enumerate() {
        let p = keys.add_bytes(kk);
        m2.put(p, kk, HM_BINARY, i as u8);
    }
    let miss = bkey(999_999, 4);
    let mp = keys.add_bytes(&miss);
    assert_eq!(m2.get(mp, HM_BINARY), -1);
    m2.check("get_key sentinel missing");
    m.free();
    m2.free();
}

#[test]
fn err_10_put_default_null() {
    let h = setup(0x3141_5926);
    for elemsize in [1usize, 8, 16, 64] {
        unsafe {
            let mc = (h.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let mr = (h.r.hmput_default)(std::ptr::null_mut(), elemsize);
            assert!(!mc.is_null() && !mr.is_null());
            let raw_c = (mc as *mut u8).sub(elemsize) as *mut c_void;
            let raw_r = (mr as *mut u8).sub(elemsize) as *mut c_void;
            assert_eq!((*header_of(raw_c)).length, 1);
            assert_eq!((*header_of(raw_r)).length, 1);
            assert_same(
                &format!("hmput_default(NULL) e={}", elemsize),
                &describe_map(mc, elemsize, KeyRepr::Bytes),
                &describe_map(mr, elemsize, KeyRepr::Bytes),
            );
            (h.c.hmfree_func)(raw_c, elemsize);
            (h.r.hmfree_func)(raw_r, elemsize);
        }
    }
}

#[test]
fn err_11_put_default_len0() {
    // a != NULL and header(HASH_TO_ARR(a))->length == 0 -> still takes the
    // grow branch. Forge that state with stbds_arrgrowf.
    let h = setup(0x3141_5926);
    for elemsize in [8usize, 16] {
        unsafe {
            let base_c = (h.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            let base_r = (h.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            assert_eq!((*header_of(base_c)).length, 0, "precondition: length 0");
            let ac = (base_c as *mut u8).add(elemsize) as *mut c_void;
            let ar = (base_r as *mut u8).add(elemsize) as *mut c_void;
            let mc = (h.c.hmput_default)(ac, elemsize);
            let mr = (h.r.hmput_default)(ar, elemsize);
            let raw_c = (mc as *mut u8).sub(elemsize) as *mut c_void;
            let raw_r = (mr as *mut u8).sub(elemsize) as *mut c_void;
            assert_eq!((*header_of(raw_c)).length, 1);
            assert_eq!((*header_of(raw_r)).length, 1);
            assert_same(
                &format!("hmput_default(len0) e={}", elemsize),
                &describe_map(mc, elemsize, KeyRepr::Bytes),
                &describe_map(mr, elemsize, KeyRepr::Bytes),
            );
            (h.c.hmfree_func)(raw_c, elemsize);
            (h.r.hmfree_func)(raw_r, elemsize);
        }
    }
}

#[test]
fn err_12_put_null_a() {
    let h = setup(0x3141_5926);
    for elemsize in [8usize, 16, 32] {
        for mode in [HM_BINARY, HM_STRING, -3, 9] {
            let mut m = MapPair::new(&h.c, &h.r, elemsize, 8, mode >= HM_STRING);
            let mut keys = Keys::new();
            let k = keys.add_str("bootstrap");
            let t = m.put(k, b"bootstrap", mode, 0x77);
            assert_eq!(t, 0, "first insert into a NULL map lands at index 0");
            m.check(&format!("put(NULL) e={} mode={}", elemsize, mode));
            m.free();
        }
    }
}

#[test]
fn err_13_put_grow_threshold() {
    // used_count_threshold for 8 slots is 6: the 6th insert must rehash to 16.
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let mut seq_c = Vec::new();
    let mut seq_r = Vec::new();
    for i in 0..60u64 {
        let kb = bkey(i.wrapping_mul(0x1234_5678_9ABC_DEF1), 4);
        let k = keys.add_bytes(&kb);
        m.put(k, &kb, HM_BINARY, i as u8);
        unsafe {
            for (v, mm) in [(&mut seq_c, m.mc), (&mut seq_r, m.mr)] {
                let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
                let t = &*(hd.hash_table as *const HashIndex);
                v.push((t.slot_count, t.used_count, t.used_count_threshold));
            }
        }
        m.check(&format!("grow threshold {}", i));
    }
    assert_eq!(seq_c, seq_r, "growth threshold sequence mismatch");
    // The threshold is tested on ENTRY to stbds_hmput_key, so the table is
    // still 8 slots when used_count reaches the threshold (6); the *next*
    // insert is the one that rehashes to 16.
    assert_eq!(seq_c[0], (8, 1, 6));
    assert_eq!(seq_c[4], (8, 5, 6));
    assert_eq!(seq_c[5], (8, 6, 6), "6th insert fills the table to threshold");
    assert_eq!(seq_c[6], (16, 7, 12), "7th insert must rehash to 16 slots");
    m.free();
}

#[test]
fn err_14_put_duplicate() {
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let kb = bkey(0xABCD, 4);
    let k = keys.add_bytes(&kb);
    let t0 = m.put(k, &kb, HM_BINARY, 1);
    let len0 = unsafe { (*header_of(m.raw(m.mc))).length };
    for i in 0..50u8 {
        let k2 = keys.add_bytes(&kb);
        let t = m.put(k2, &kb, HM_BINARY, i);
        assert_eq!(t, t0, "duplicate put must reuse the existing index");
        let len = unsafe { (*header_of(m.raw(m.mc))).length };
        assert_eq!(len, len0, "duplicate put must not append");
        m.check("duplicate put");
    }
    m.free();
}

// ===========================================================================
// Rows 17-19 — out-of-range `mode`
// ===========================================================================

#[test]
fn err_17_mode_negative() {
    // mode < 1 -> binary path; nt->string.mode set to 0
    let h = setup(0x3141_5926);
    for mode in [-1 as c_int, -2, -1000, c_int::MIN] {
        let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
        let mut keys = Keys::new();
        let kb = bkey(0x5555, 4);
        let k = keys.add_bytes(&kb);
        m.put(k, &kb, mode, 3);
        unsafe {
            for (nm, mm) in [("C", m.mc), ("RUST", m.mr)] {
                let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
                let t = &*(hd.hash_table as *const HashIndex);
                assert_eq!(t.string.mode, 0, "{}: mode {} must give string.mode 0", nm, mode);
            }
        }
        assert!(m.get(k, mode) >= 0);
        assert_eq!(m.del(k, 0, mode), 1);
        assert_eq!(m.del(k, 0, mode), 0);
        m.check(&format!("mode {} binary path", mode));
        m.free();
    }
}

#[test]
fn err_18_mode_gt_one() {
    // mode > 1 -> string path; nt->string.mode set to STBDS_SH_DEFAULT
    let h = setup(0x3141_5926);
    for mode in [2 as c_int, 3, 4, 1000, c_int::MAX] {
        let mut m = MapPair::new(&h.c, &h.r, 16, 8, true);
        let mut keys = Keys::new();
        let k = keys.add_str("string-ish");
        m.put(k, &[], mode, 3);
        unsafe {
            for (nm, mm) in [("C", m.mc), ("RUST", m.mr)] {
                let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
                let t = &*(hd.hash_table as *const HashIndex);
                assert_eq!(t.string.mode, 1, "{}: mode {} must give SH_DEFAULT", nm, mode);
            }
        }
        let k2 = keys.add_str("string-ish");
        assert!(m.get(k2, mode) >= 0, "must be found via strcmp, not pointer ==");
        m.check(&format!("mode {} string path", mode));
        m.free();
    }
}

#[test]
fn err_19_del_mode_two() {
    // stbds_hmdel_key gates the strdup-free and the swap re-find on
    // `mode == STBDS_HM_STRING` (exactly 1) while stbds_hm_find_slot uses
    // `mode >= STBDS_HM_STRING`. With mode == 2 the re-find therefore takes the
    // BINARY branch and hands find_slot the *address* of the element, whose
    // contents (a `char *`) then get hashed as a string. Deterministic --
    // both libraries are given the same key pointers -- and it trips
    // STBDS_ASSERT. Run in a child and require identical deaths.
    let h = setup(0x3141_5926);
    for mode in [2 as c_int, 3, 77] {
        let body = |lib: &Lib| {
            let elemsize = 16usize;
            let mut keys = Keys::new();
            let mut m: *mut c_void = std::ptr::null_mut();
            let mut first: *mut c_void = std::ptr::null_mut();
            for i in 0..6 {
                let k = keys.add_str(&format!("delmode_{}", i));
                if i == 0 {
                    first = k;
                }
                unsafe {
                    m = (lib.hmput_key)(m, elemsize, k, 8, mode);
                }
            }
            unsafe {
                // delete the FIRST element -> old_index != final_index -> swap
                (lib.hmdel_key)(m, elemsize, first, 8, 0, mode);
            }
            std::mem::forget(keys);
        };
        let dc = run_in_child(|| body(&h.c));
        let dr = run_in_child(|| body(&h.r));
        assert_eq!(dc, dr, "hmdel_key mode={} death mismatch", mode);
    }
}

// ===========================================================================
// Rows 20-31 — stbds_hmdel_key
// ===========================================================================

#[test]
fn err_20_del_null_a() {
    let h = setup(0x3141_5926);
    let mut keys = Keys::new();
    let k = keys.add_str("k");
    for elemsize in [0usize, 1, 8, 16] {
        for mode in [HM_BINARY, HM_STRING, -1, 6] {
            for keyoffset in [0usize, 4, 8] {
                unsafe {
                    let rc = (h.c.hmdel_key)(std::ptr::null_mut(), elemsize, k, 8, keyoffset, mode);
                    let rr = (h.r.hmdel_key)(std::ptr::null_mut(), elemsize, k, 8, keyoffset, mode);
                    assert!(rc.is_null(), "C hmdel_key(NULL) must return NULL");
                    assert!(rr.is_null(), "Rust hmdel_key(NULL) must return NULL");
                }
            }
        }
    }
}

#[test]
fn err_21_del_no_table() {
    let h = setup(0x3141_5926);
    for elemsize in [8usize, 16] {
        let mut m = MapPair::new(&h.c, &h.r, elemsize, 4, false);
        m.put_default();
        // pre-set temp to something else so the "temp = 0" write is observable
        unsafe {
            (*header_of(m.raw(m.mc))).temp = 42;
            (*header_of(m.raw(m.mr))).temp = 42;
        }
        let mut keys = Keys::new();
        let kb = bkey(3, 4);
        let k = keys.add_bytes(&kb);
        let pc = m.mc;
        assert_eq!(m.del(k, 0, HM_BINARY), 0, "temp must be reset to 0");
        assert_eq!(m.mc, pc, "must return `a` unchanged");
        m.check("del no table");
        m.free();
    }
}

#[test]
fn err_22_del_missing_key() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 22);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let present: Vec<Vec<u8>> = (0..60u64).map(|i| bkey(i * 31 + 5, 4)).collect();
    for (i, kb) in present.iter().enumerate() {
        let k = keys.add_bytes(kb);
        m.put(k, kb, HM_BINARY, i as u8);
    }
    let snapshot = m.desc().0;
    for _ in 0..1000 {
        let kb = bkey(rng.next_u64(), 4);
        if present.contains(&kb) {
            continue;
        }
        let k = keys.add_bytes(&kb);
        assert_eq!(m.del(k, 0, HM_BINARY), 0, "missing delete -> temp 0");
        m.check("del missing");
    }
    // header->temp is the only thing that may have changed
    let after = m.desc().0;
    assert_eq!(
        snapshot.replace("temp=0", "temp=X").split('\n').skip(1).collect::<Vec<_>>(),
        after.replace("temp=0", "temp=X").split('\n').skip(1).collect::<Vec<_>>(),
        "a failed delete must not mutate the table"
    );
    m.free();
}

#[test]
fn err_23_del_present_sentinels() {
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let ks: Vec<Vec<u8>> = (0..6u64).map(|i| bkey(i * 13 + 2, 4)).collect();
    for (i, kb) in ks.iter().enumerate() {
        let k = keys.add_bytes(kb);
        m.put(k, kb, HM_BINARY, i as u8);
    }
    let len_before = unsafe { (*header_of(m.raw(m.mc))).length };
    let k = keys.add_bytes(&ks[2]);
    assert_eq!(m.del(k, 0, HM_BINARY), 1, "successful delete -> temp 1");
    let len_after = unsafe { (*header_of(m.raw(m.mc))).length };
    assert_eq!(len_after, len_before - 1);
    // the vacated slot must carry HASH_DELETED / INDEX_DELETED in both libs
    unsafe {
        for (nm, mm) in [("C", m.mc), ("RUST", m.mr)] {
            let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
            let t = &*(hd.hash_table as *const HashIndex);
            let mut found = false;
            for b in 0..(t.slot_count >> 3) {
                let bk = &*t.storage.add(b);
                for i in 0..8 {
                    if bk.hash[i] == STBDS_HASH_DELETED {
                        assert_eq!(bk.index[i], STBDS_INDEX_DELETED, "{}", nm);
                        found = true;
                    }
                }
            }
            assert!(found, "{}: no tombstone slot after delete", nm);
            assert_eq!(t.tombstone_count, 1);
        }
    }
    m.check("del sentinels");
    m.free();
}

#[test]
fn err_24_25_26_del_invariants() {
    // STBDS_ASSERT(slot < slot_count), STBDS_ASSERT(slot >= 0) on the swap
    // re-find, and STBDS_ASSERT(b->index[i] == final_index). None is reachable
    // through the public API; assert them as invariants across a delete storm,
    // and require both libraries to survive it.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 24);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let mut live: Vec<Vec<u8>> = Vec::new();
    for op in 0..3000u64 {
        if live.is_empty() || rng.below(3) != 0 {
            let kb = bkey(rng.next_u64(), 4);
            let k = keys.add_bytes(&kb);
            m.put(k, &kb, HM_BINARY, (op & 0xff) as u8);
            if !live.contains(&kb) {
                live.push(kb);
            }
        } else {
            let idx = rng.below(live.len());
            let kb = live.swap_remove(idx);
            let k = keys.add_bytes(&kb);
            assert_eq!(m.del(k, 0, HM_BINARY), 1, "live key must delete");
        }
        // every in-use slot must hold a hash >= 2 and an index < length-1
        unsafe {
            for mm in [m.mc, m.mr] {
                let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
                let t = &*(hd.hash_table as *const HashIndex);
                let mut in_use = 0usize;
                for b in 0..(t.slot_count >> 3) {
                    let bk = &*t.storage.add(b);
                    for i in 0..8 {
                        if bk.index[i] >= 0 {
                            in_use += 1;
                            assert!(bk.hash[i] >= 2, "in-use slot with hash < 2");
                            assert!(
                                (bk.index[i] as usize) < hd.length,
                                "index {} out of range (length {})",
                                bk.index[i],
                                hd.length
                            );
                        } else {
                            assert!(
                                bk.index[i] == STBDS_INDEX_EMPTY
                                    || bk.index[i] == STBDS_INDEX_DELETED
                            );
                        }
                    }
                }
                assert_eq!(in_use, t.used_count, "used_count out of sync");
                assert_eq!(in_use + 1, hd.length, "length out of sync");
            }
        }
        m.check(&format!("del invariants op {}", op));
        if keys.bufs.len() > 6000 {
            keys.bufs.clear();
        }
    }
    m.free();
}

#[test]
fn err_27_del_shrink() {
    // used_count < used_count_shrink_threshold (slot_count/4) && slot_count > 8
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let ks: Vec<Vec<u8>> = (0..80u64).map(|i| bkey(i.wrapping_mul(0xDEAD_BEEF), 4)).collect();
    for (i, kb) in ks.iter().enumerate() {
        let k = keys.add_bytes(kb);
        m.put(k, kb, HM_BINARY, i as u8);
    }
    let big = unsafe {
        let hd = &*header_of(m.raw(m.mc));
        (*(hd.hash_table as *const HashIndex)).slot_count
    };
    assert!(big > 8, "precondition: table grew past 8 slots");
    let mut shrinks = 0;
    let mut prev = big;
    let mut seq_c = Vec::new();
    let mut seq_r = Vec::new();
    for kb in ks.iter() {
        let k = keys.add_bytes(kb);
        m.del(k, 0, HM_BINARY);
        unsafe {
            for (v, mm) in [(&mut seq_c, m.mc), (&mut seq_r, m.mr)] {
                let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
                let t = &*(hd.hash_table as *const HashIndex);
                v.push((t.slot_count, t.used_count, t.tombstone_count));
            }
            let hd = &*header_of(m.raw(m.mc));
            let t = &*(hd.hash_table as *const HashIndex);
            if t.slot_count < prev {
                shrinks += 1;
            }
            prev = t.slot_count;
            assert!(t.slot_count >= 8, "must never shrink below 8 slots");
        }
        m.check("del shrink");
    }
    assert_eq!(seq_c, seq_r, "shrink sequence mismatch");
    assert!(shrinks > 0, "the shrink branch was never taken");
    assert_eq!(prev, 8, "should have shrunk all the way back to 8");
    m.free();
}

#[test]
fn err_28_del_tombstone_rebuild() {
    // tombstone_count > tombstone_count_threshold (slot_count/8 + slot_count/16)
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let mut rebuilds = 0;
    let mut seq_c = Vec::new();
    let mut seq_r = Vec::new();
    // keep used_count high (re-inserting) while accumulating tombstones, so the
    // shrink branch stays out of the way and the rebuild branch fires
    for round in 0..40u64 {
        for i in 0..40u64 {
            let kb = bkey(round * 1000 + i, 4);
            let k = keys.add_bytes(&kb);
            m.put(k, &kb, HM_BINARY, i as u8);
        }
        for i in 0..20u64 {
            let kb = bkey(round * 1000 + i, 4);
            let k = keys.add_bytes(&kb);
            let prev_tomb = unsafe {
                let hd = &*header_of(m.raw(m.mc));
                (*(hd.hash_table as *const HashIndex)).tombstone_count
            };
            m.del(k, 0, HM_BINARY);
            unsafe {
                for (v, mm) in [(&mut seq_c, m.mc), (&mut seq_r, m.mr)] {
                    let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
                    let t = &*(hd.hash_table as *const HashIndex);
                    v.push((t.slot_count, t.used_count, t.tombstone_count));
                }
                let hd = &*header_of(m.raw(m.mc));
                let t = &*(hd.hash_table as *const HashIndex);
                if t.tombstone_count == 0 && prev_tomb > 0 {
                    rebuilds += 1;
                }
            }
            m.check("del tombstone rebuild");
        }
    }
    assert_eq!(seq_c, seq_r, "tombstone/rebuild sequence mismatch");
    assert!(rebuilds > 0, "the tombstone-rebuild branch was never taken");
    m.free();
}

#[test]
fn err_29_del_last_element() {
    // old_index == final_index -> skips the memmove and the re-find entirely
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let ks: Vec<Vec<u8>> = (0..40u64).map(|i| bkey(i * 17 + 1, 4)).collect();
    for (i, kb) in ks.iter().enumerate() {
        let k = keys.add_bytes(kb);
        m.put(k, kb, HM_BINARY, i as u8);
    }
    // delete in reverse insertion order: always the last element
    for kb in ks.iter().rev() {
        let k = keys.add_bytes(kb);
        assert_eq!(m.del(k, 0, HM_BINARY), 1);
        m.check("del last element");
    }
    let len = unsafe { (*header_of(m.raw(m.mc))).length };
    assert_eq!(len, 1, "only the default element should remain");
    m.free();
}

#[test]
fn err_30_del_keyoffset_oob() {
    // keyoffset up to and beyond `elemsize - keysize`. Unused capacity is
    // scrubbed to a fixed pattern in both libraries by `MapPair::check`, so
    // reads that stray past an element are still fully deterministic.
    let h = setup(0x3141_5926);
    let elemsize = 16usize;
    let keysize = 4usize;
    for keyoffset in [0usize, 1, 7, 11, 12, 13, 15, 16, 20, 28] {
        let mut rng = Rng::new(SEED ^ 30 ^ keyoffset as u64);
        let mut m = MapPair::new(&h.c, &h.r, elemsize, keysize, false);
        let mut keys = Keys::new();
        let ks: Vec<Vec<u8>> = (0..40u64).map(|_| bkey(rng.next_u64(), keysize)).collect();
        for (i, kb) in ks.iter().enumerate() {
            let k = keys.add_bytes(kb);
            m.put(k, kb, HM_BINARY, i as u8);
        }
        m.check(&format!("keyoffset={} setup", keyoffset));
        for kb in ks.iter() {
            let k = keys.add_bytes(kb);
            m.del(k, keyoffset, HM_BINARY);
            m.check(&format!("keyoffset={} del", keyoffset));
        }
        m.free();
    }
}

#[test]
fn err_31_find_probes_past_tombstone() {
    // A tombstone must NOT terminate a probe: after deleting keys, every
    // remaining key must still be findable in both libraries.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 31);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let ks: Vec<Vec<u8>> = (0..300u64).map(|_| bkey(rng.next_u64(), 4)).collect();
    for (i, kb) in ks.iter().enumerate() {
        let k = keys.add_bytes(kb);
        m.put(k, kb, HM_BINARY, i as u8);
    }
    let mut live: Vec<Vec<u8>> = ks.clone();
    live.dedup();
    let mut saw_tombstones = false;
    for _ in 0..80 {
        let idx = rng.below(live.len());
        let kb = live.swap_remove(idx);
        let k = keys.add_bytes(&kb);
        m.del(k, 0, HM_BINARY);
        unsafe {
            let hd = &*header_of(m.raw(m.mc));
            let t = &*(hd.hash_table as *const HashIndex);
            if t.tombstone_count > 0 {
                saw_tombstones = true;
            }
        }
        for kb2 in live.iter() {
            let k2 = keys.add_bytes(kb2);
            assert!(m.get(k2, HM_BINARY) >= 0, "probe stopped at a tombstone");
        }
        m.check("probe past tombstone");
        if keys.bufs.len() > 30000 {
            keys.bufs.clear();
        }
    }
    assert!(saw_tombstones, "no tombstone was ever created");
    m.free();
}

#[test]
fn err_32_hash_lt_two() {
    // `if (hash < 2) hash += 2;` guarantees a live slot never carries
    // HASH_EMPTY(0) or HASH_DELETED(1). A key that actually hashes below 2
    // cannot be found by search (the space is 2^64), so the rule is verified
    // as the invariant it exists to maintain, over both key kinds.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 32);
    for (label, string_mode) in [("binary", false), ("string", true)] {
        let mode = if string_mode { HM_STRING } else { HM_BINARY };
        let mut m = MapPair::new(&h.c, &h.r, 16, 8, string_mode);
        let mut keys = Keys::new();
        for i in 0..600u64 {
            let kb = bkey(rng.next_u64(), 8);
            let k = if string_mode {
                keys.add_str(&format!("hk_{}", i))
            } else {
                keys.add_bytes(&kb)
            };
            m.put(k, &kb, mode, i as u8);
            unsafe {
                for mm in [m.mc, m.mr] {
                    let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
                    let t = &*(hd.hash_table as *const HashIndex);
                    for b in 0..(t.slot_count >> 3) {
                        let bk = &*t.storage.add(b);
                        for j in 0..8 {
                            if bk.index[j] >= 0 {
                                assert!(bk.hash[j] >= 2, "{}: live slot with hash < 2", label);
                            }
                        }
                    }
                }
            }
        }
        m.check(&format!("hash<2 invariant {}", label));
        m.free();
    }
}

// ===========================================================================
// Rows 33-39 — hash function boundaries
// ===========================================================================

#[test]
fn err_33_hash_bytes_len0() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 33);
    let mut buf = rng.bytes(64);
    let p = buf.as_mut_ptr() as *mut c_void;
    for seed in [0usize, 1, 2, 0x3141_5926, usize::MAX, rng.next_u64() as usize] {
        unsafe {
            let a = (h.c.hash_bytes)(p, 0, seed);
            let b = (h.r.hash_bytes)(p, 0, seed);
            assert_eq!(a, b, "hash_bytes(len=0, seed={:#x})", seed);
            // must depend only on the seed, not on the buffer
            let mut other = vec![0xFFu8; 64];
            let a2 = (h.c.hash_bytes)(other.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(a, a2, "len=0 must not read the buffer");
        }
    }
}

#[test]
fn err_34_hash_bytes_null_len0() {
    // len == 0 never dereferences `p`, so NULL must be accepted.
    let h = setup(0x3141_5926);
    for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
        let dc = run_in_child(|| unsafe {
            (h.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
        });
        let dr = run_in_child(|| unsafe {
            (h.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
        });
        assert_eq!(dc, Death::Exited(0), "C must not crash on (NULL, 0)");
        assert_eq!(dr, Death::Exited(0), "Rust must not crash on (NULL, 0)");
        unsafe {
            let a = (h.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let b = (h.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(a, b);
            let mut buf = vec![0u8; 8];
            let c = (h.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(a, c, "(NULL,0) must equal (buf,0)");
        }
    }
}

#[test]
fn err_35_hash_sign_extension() {
    // `d[3] << 24` / `d[7] << 24` are signed `int` expressions that sign-extend
    // into the top half of `size_t`. Exhaustive over the critical bytes.
    let h = setup(0x3141_5926);
    for len in [4usize, 5, 6, 7, 8, 9, 12, 16] {
        for pos in 0..len {
            for v in [0x00u8, 0x01, 0x7F, 0x80, 0x81, 0xFE, 0xFF] {
                let mut buf = vec![0u8; 24];
                buf[pos] = v;
                for seed in [0usize, 0x3141_5926, usize::MAX] {
                    unsafe {
                        let a = (h.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                        let b = (h.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                        assert_eq!(
                            a, b,
                            "sign-extension mismatch len={} pos={} v={:#x} seed={:#x}",
                            len, pos, v, seed
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn err_36_hash_tail_all_arms() {
    // All seven fall-through arms of the tail `switch`, with every byte value
    // in the arm-selecting position.
    let h = setup(0x3141_5926);
    for rem in 1usize..=7 {
        for len in [rem, 8 + rem, 16 + rem, 64 + rem] {
            for v in 0u16..=255 {
                let mut buf = vec![0x5Au8; len + 8];
                buf[len - 1] = v as u8;
                unsafe {
                    let a = (h.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 0x3141_5926);
                    let b = (h.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 0x3141_5926);
                    assert_eq!(a, b, "tail arm rem={} len={} v={:#x}", rem, len, v);
                }
            }
        }
    }
}

#[test]
fn err_37_hash_bytes_oversized_len() {
    // len == SIZE_MAX (and other absurd lengths): the `i + 8 <= len` loop walks
    // off the end of the buffer until it hits an unmapped page. Both libraries
    // must die the same way. Run in a child process.
    let h = setup(0x3141_5926);
    for len in [usize::MAX, usize::MAX - 1, 1usize << 40] {
        let dc = run_in_child(|| unsafe {
            let mut buf = vec![0u8; 64];
            (h.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 0x3141_5926);
        });
        let dr = run_in_child(|| unsafe {
            let mut buf = vec![0u8; 64];
            (h.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 0x3141_5926);
        });
        assert_eq!(dc, dr, "hash_bytes(len={}) death mismatch", len);
        assert_ne!(dc, Death::Exited(0), "expected a fatal signal for len={}", len);
    }
}

#[test]
fn err_38_hash_string_empty() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 38);
    let mut empty = vec![0u8; 1];
    let p = empty.as_mut_ptr() as *mut c_char;
    for seed in [0usize, 1, 2, 0x3141_5926, usize::MAX, rng.next_u64() as usize] {
        unsafe {
            let a = (h.c.hash_string)(p, seed);
            let b = (h.r.hash_string)(p, seed);
            assert_eq!(a, b, "hash_string(\"\", {:#x})", seed);
        }
    }
}

#[test]
fn err_39_hash_string_high_bytes() {
    // `(unsigned char) *str++` zero-extends -- unlike hash_bytes, which
    // sign-extends. Exhaustive over every non-NUL byte value.
    let h = setup(0x3141_5926);
    for v in 1u16..=255 {
        for len in [1usize, 2, 7, 8, 9, 33] {
            let mut buf = vec![v as u8; len + 1];
            buf[len] = 0;
            for seed in [0usize, 0x3141_5926, usize::MAX] {
                unsafe {
                    let a = (h.c.hash_string)(buf.as_mut_ptr() as *mut c_char, seed);
                    let b = (h.r.hash_string)(buf.as_mut_ptr() as *mut c_char, seed);
                    assert_eq!(a, b, "hash_string byte={:#x} len={} seed={:#x}", v, len, seed);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 40, 46-48 — arena / shmode / NULL key
// ===========================================================================

#[test]
fn err_40_stralloc_forged_arena() {
    // storage == NULL with remaining > 0 skips the allocation branch and then
    // dereferences a->storage. Both libraries must die identically.
    let h = setup(0x3141_5926);
    for remaining in [1usize, 2, 1000] {
        let body = |lib: &Lib| {
            let mut a = StringArena {
                storage: std::ptr::null_mut(),
                remaining,
                block: 0,
                mode: 0,
            };
            let mut s = b"ab\0".to_vec();
            unsafe {
                (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            }
        };
        let dc = run_in_child(|| body(&h.c));
        let dr = run_in_child(|| body(&h.r));
        assert_eq!(dc, dr, "forged arena (remaining={}) death mismatch", remaining);
    }
}

#[test]
fn err_46_shmode_out_of_range() {
    let h = setup(0x3141_5926);
    for mode in [4 as c_int, 5, 100, 255, 256, 257, -1, -256, c_int::MAX, c_int::MIN] {
        let elemsize = 16usize;
        unsafe {
            let mc = (h.c.shmode_func)(elemsize, mode);
            let mr = (h.r.shmode_func)(elemsize, mode);
            let raw_c = (mc as *mut u8).sub(elemsize) as *mut c_void;
            let raw_r = (mr as *mut u8).sub(elemsize) as *mut c_void;
            let expect = (mode as u32 & 0xff) as u8;
            for (nm, raw) in [("C", raw_c), ("RUST", raw_r)] {
                let t = &*((*header_of(raw)).hash_table as *const HashIndex);
                assert_eq!(t.string.mode, expect, "{}: (unsigned char){} ", nm, mode);
            }
            assert_same(
                &format!("shmode_func out-of-range {}", mode),
                &describe_map(mc, elemsize, KeyRepr::Bytes),
                &describe_map(mr, elemsize, KeyRepr::Bytes),
            );
            (h.c.hmfree_func)(raw_c, elemsize);
            (h.r.hmfree_func)(raw_r, elemsize);
        }
    }
}

#[test]
fn err_47_shmode_elemsize_zero() {
    let h = setup(0x3141_5926);
    for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        unsafe {
            let mc = (h.c.shmode_func)(0, mode);
            let mr = (h.r.shmode_func)(0, mode);
            // elemsize 0: HASH_TO_ARR is the identity, length forced to 1
            assert_eq!((*header_of(mc)).length, 1);
            assert_eq!((*header_of(mr)).length, 1);
            assert_same(
                &format!("shmode_func elemsize=0 mode={}", mode),
                &describe_map(mc, 0, KeyRepr::Bytes),
                &describe_map(mr, 0, KeyRepr::Bytes),
            );
            (h.c.hmfree_func)(mc, 0);
            (h.r.hmfree_func)(mr, 0);
        }
    }
}

#[test]
fn err_48_string_null_key() {
    // A NULL key in string mode: stbds_hash_string dereferences it immediately.
    let h = setup(0x3141_5926);
    for mode in [HM_STRING, 2 as c_int] {
        let body = |lib: &Lib| unsafe {
            let elemsize = 16usize;
            let mut m: *mut c_void = std::ptr::null_mut();
            let mut k = b"seed\0".to_vec();
            m = (lib.hmput_key)(m, elemsize, k.as_mut_ptr() as *mut c_void, 8, mode);
            (lib.hmput_key)(m, elemsize, std::ptr::null_mut(), 8, mode);
        };
        let dc = run_in_child(|| body(&h.c));
        let dr = run_in_child(|| body(&h.r));
        assert_eq!(dc, dr, "NULL string key death mismatch (mode={})", mode);
        assert_ne!(dc, Death::Exited(0), "expected a fatal signal");

        // and on the get path
        let gbody = |lib: &Lib| unsafe {
            let elemsize = 16usize;
            let mut m: *mut c_void = std::ptr::null_mut();
            let mut k = b"seed\0".to_vec();
            m = (lib.hmput_key)(m, elemsize, k.as_mut_ptr() as *mut c_void, 8, mode);
            (lib.hmget_key)(m, elemsize, std::ptr::null_mut(), 8, mode);
        };
        let gc = run_in_child(|| gbody(&h.c));
        let gr = run_in_child(|| gbody(&h.r));
        assert_eq!(gc, gr, "NULL string key (get) death mismatch (mode={})", mode);
    }
}

// ===========================================================================
// Rows 49-51 — arr_push / strkey boundaries
// ===========================================================================

#[test]
fn err_49_arr_push_nonpositive() {
    let h = setup(0x3141_5926);
    for num in [0i32, -1, -2, -50, -51, -1000, i32::MIN] {
        let dc = run_in_child(|| unsafe { (h.c.arr_push)(num) });
        let dr = run_in_child(|| unsafe { (h.r.arr_push)(num) });
        assert_eq!(dc, dr, "arr_push({}) death mismatch", num);
        assert_eq!(dc, Death::Exited(0), "arr_push({}) must be a silent no-op", num);
        unsafe {
            (h.c.arr_push)(num);
            (h.r.arr_push)(num);
        }
    }
}

#[test]
fn err_50_arr_push_large() {
    // `i += 50` with the inner O(i) push loop makes num anywhere near INT_MAX
    // computationally infeasible (~10^15 pushes), so the largest tractable
    // values are used. The arithmetic is byte-identical either way.
    let h = setup(0x3141_5926);
    for num in [4999i32, 5000, 5001, 10_000, 20_000] {
        let dc = run_in_child(|| unsafe { (h.c.arr_push)(num) });
        let dr = run_in_child(|| unsafe { (h.r.arr_push)(num) });
        assert_eq!(dc, dr, "arr_push({}) death mismatch", num);
        assert_eq!(dc, Death::Exited(0));
    }
}

#[test]
fn err_51_strkey_negative() {
    let h = setup(0x3141_5926);
    for n in [-1i32, -9, -10, -99, -100, -1000, -2_147_483_647, i32::MIN] {
        unsafe {
            let pc = (h.c.strkey)(n);
            let pr = (h.r.strkey)(n);
            let sc = std::ffi::CStr::from_ptr(pc).to_bytes().to_vec();
            let sr = std::ffi::CStr::from_ptr(pr).to_bytes().to_vec();
            assert_eq!(sc, sr, "strkey({})", n);
            assert_eq!(
                sc,
                format!("test_{}", n).into_bytes(),
                "strkey({}) content",
                n
            );
            assert!(sc.len() < 256, "must fit the 256-byte static buffer");
        }
    }
}
