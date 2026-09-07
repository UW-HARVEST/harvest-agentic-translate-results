//! Phase C — error/rejection-path differential tests.
//!
//! One test per ERRORS.md row that does NOT crash the process.  The crashing /
//! aborting rows (6b, 7, 16, 35, 36, 49, 51) live in `phase_c_crash.rs`, which
//! compares the two libraries' exit status out of process.

mod common;
use common::*;
use std::ffi::{c_void, CStr};

fn i32k(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

#[track_caller]
unsafe fn eq_arr(ctx: &str, cp: *mut c_void, rp: *mut c_void, elemsize: usize) {
    assert_eq!(cp.is_null(), rp.is_null(), "[{ctx}] nullness");
    if cp.is_null() {
        return;
    }
    eq_snap(
        ctx,
        &snap_arr(cp, elemsize, ElemFmt::Raw, false),
        &snap_arr(rp, elemsize, ElemFmt::Raw, false),
    );
}

// ---------------------------------------------------------------------------
// rows 1-6: stbds_arrgrowf
// ---------------------------------------------------------------------------

#[test]
fn err01_arrgrowf_nothing_to_grow_returns_input() {
    let (p, _g) = libs();
    for elemsize in [1usize, 4, 8, 24] {
        unsafe {
            let cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
            let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
            let before_c = snap_arr(cp, elemsize, ElemFmt::Raw, false);
            for min_cap in [0usize, 1, 8, 15, 16] {
                let c2 = (p.c.arrgrowf)(cp, elemsize, 0, min_cap);
                let r2 = (p.rs.arrgrowf)(rp, elemsize, 0, min_cap);
                assert_eq!(c2, cp, "C must return the same pointer (min_cap={min_cap})");
                assert_eq!(r2, rp, "Rust must return the same pointer");
                let after_c = snap_arr(c2, elemsize, ElemFmt::Raw, false);
                assert_eq!(before_c, after_c, "C header must be untouched");
                eq_arr("arrgrowf nop", c2, r2, elemsize);
            }
            // one step past the valid no-op range: capacity must double
            // (realloc may or may not return the same address, so only the
            // observable header state is compared)
            let c2 = (p.c.arrgrowf)(cp, elemsize, 0, 17);
            let r2 = (p.rs.arrgrowf)(rp, elemsize, 0, 17);
            eq_arr("arrgrowf 17", c2, r2, elemsize);
            assert_eq!(snap_arr(c2, elemsize, ElemFmt::Raw, false).capacity, 32);
            (p.c.arrfreef)(c2);
            (p.rs.arrfreef)(r2);
        }
    }
}

#[test]
fn err02_arrgrowf_null_initialises_header() {
    let (p, _g) = libs();
    for elemsize in [1usize, 4, 16, 24] {
        unsafe {
            let cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 9);
            let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 9);
            let cs = snap_arr(cp, elemsize, ElemFmt::Raw, false);
            eq_arr("arrgrowf(NULL)", cp, rp, elemsize);
            assert_eq!(cs.length, 0);
            assert_eq!(cs.capacity, 9);
            assert_eq!(cs.temp, 0);
            assert!(!cs.has_table);
            (p.c.arrfreef)(cp);
            (p.rs.arrfreef)(rp);
        }
    }
}

#[test]
fn err03_04_arrgrowf_capacity_policy() {
    let (p, _g) = libs();
    let elemsize = 4usize;
    unsafe {
        // row 4: cap 0 and min_cap < 4 => floor of 4
        for min_cap in 1usize..=3 {
            let cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
            let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
            eq_arr("floor4", cp, rp, elemsize);
            assert_eq!(snap_arr(cp, elemsize, ElemFmt::Raw, false).capacity, 4);
            (p.c.arrfreef)(cp);
            (p.rs.arrfreef)(rp);
        }
        // row 3: min_cap inside (cap, 2*cap) => forced to 2*cap
        for (min_cap, want) in [(11usize, 20usize), (19, 20), (20, 20), (21, 21), (100, 100)] {
            // fresh capacity-10 base per iteration (arrgrowf may realloc in place)
            let cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            eq_arr("base10", cp, rp, elemsize);
            assert_eq!(snap_arr(cp, elemsize, ElemFmt::Raw, false).capacity, 10);
            let c2 = (p.c.arrgrowf)(cp, elemsize, 0, min_cap);
            let r2 = (p.rs.arrgrowf)(rp, elemsize, 0, min_cap);
            eq_arr("policy", c2, r2, elemsize);
            assert_eq!(
                snap_arr(c2, elemsize, ElemFmt::Raw, false).capacity,
                want,
                "min_cap={min_cap}"
            );
            (p.c.arrfreef)(c2);
            (p.rs.arrfreef)(r2);
        }
    }
}

#[test]
fn err05_arrgrowf_zero_elemsize() {
    let (p, _g) = libs();
    unsafe {
        for min_cap in [1usize, 4, 100, 1 << 20] {
            let cp = (p.c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            eq_arr(&format!("elemsize=0 min_cap={min_cap}"), cp, rp, 0);
            assert_eq!(snap_arr(cp, 0, ElemFmt::Raw, false).capacity, min_cap.max(4));
            (p.c.arrfreef)(cp);
            (p.rs.arrfreef)(rp);
        }
        // chained growth with elemsize 0
        let mut cp = (p.c.arrgrowf)(std::ptr::null_mut(), 0, 1, 0);
        let mut rp = (p.rs.arrgrowf)(std::ptr::null_mut(), 0, 1, 0);
        for _ in 0..10 {
            cp = (p.c.arrgrowf)(cp, 0, 5, 0);
            rp = (p.rs.arrgrowf)(rp, 0, 5, 0);
            (*(cp as *mut ArrHeader).sub(1)).length += 5;
            (*(rp as *mut ArrHeader).sub(1)).length += 5;
            eq_arr("elemsize=0 chain", cp, rp, 0);
        }
        (p.c.arrfreef)(cp);
        (p.rs.arrfreef)(rp);
    }
}

// ERRORS.md row 6 (`arrlen + addlen` / `elemsize * min_cap + 32` wrap-around)
// corrupts the heap in C — `elemsize * min_cap + sizeof(header)` can wrap down
// below 32 bytes, so the header write runs off the end of the allocation.  It is
// therefore verified out of process in `phase_c_crash.rs`
// (`overflow_wrap_es4` / `overflow_wrap_es16` / `overflow_realloc_fail`).

// ---------------------------------------------------------------------------
// rows 8, 9: stbds_hmfree_func
// ---------------------------------------------------------------------------

#[test]
fn err08_hmfree_null_is_nop() {
    let (p, _g) = libs();
    unsafe {
        for elemsize in [0usize, 1, 4, 16, 24, usize::MAX] {
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.rs.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

#[test]
fn err09_hmfree_tableless_array() {
    let (p, _g) = libs();
    unsafe {
        for elemsize in [1usize, 4, 16, 24] {
            for addlen in [0usize, 1, 5] {
                let cp = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 1);
                let rp = (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 1);
                assert!(!snap_arr(cp, elemsize, ElemFmt::Raw, false).has_table);
                (p.c.hmfree_func)(cp, elemsize);
                (p.rs.hmfree_func)(rp, elemsize);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 10, 11: stbds_hm_find_slot returning -1 from both scan halves
// ---------------------------------------------------------------------------

#[test]
fn err10_11_find_slot_miss_both_scan_halves() {
    // The upper scan (`i = pos & 7 .. 8`) and the wrapped lower scan
    // (`i = 0 .. pos & 7`) each have their own `return -1`.  Many random
    // seeds x table sizes x probe keys drive both.
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 1011);
    for trial in 0..40u64 {
        let gseed = rng.next_u64() as usize;
        reset_seed(p, gseed);
        let mut m = Maps::empty(p, cfg_struct());
        let mut keys = Vec::new();
        for _ in 0..(1 + rng.below(60)) {
            let k = rng.next_i32();
            if keys.contains(&k) {
                continue;
            }
            keys.push(k);
            m.put_binary(&i32k(k), &rng.bytes(12));
        }
        let mut misses = 0;
        for _ in 0..400 {
            let k = rng.next_i32();
            if keys.contains(&k) {
                continue;
            }
            assert_eq!(
                m.get(&i32k(k)),
                -1,
                "trial={trial} seed={gseed:#x}: absent key {k} must miss"
            );
            misses += 1;
        }
        assert!(misses > 300);
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 12-15: hmget_key / hmget_key_ts sentinels
// ---------------------------------------------------------------------------

#[test]
fn err12_13_14_15_hmget_sentinels() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 12);
    for elemsize in [8usize, 16, 24] {
        for mode in [HM_BINARY, HM_STRING, 2, -1, i32::MAX, i32::MIN] {
            unsafe {
                let key = CKey::new(&rng.nonnul(8));
                // row 12: a == NULL
                let mut ct = 999isize;
                let mut rt = 999isize;
                let cp = (p.c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.ptr() as *mut c_void,
                    8,
                    &mut ct,
                    mode,
                );
                let rp = (p.rs.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.ptr() as *mut c_void,
                    8,
                    &mut rt,
                    mode,
                );
                assert_eq!((ct, rt), (-1, -1), "row12 es={elemsize} mode={mode}");
                let cs = snap_hash(cp, elemsize, ElemFmt::Raw);
                eq_snap("row12", &cs, &snap_hash(rp, elemsize, ElemFmt::Raw));
                assert_eq!(cs.length, 1);
                assert!(!cs.has_table);

                // row 13: table == NULL
                let mut ct = 999isize;
                let mut rt = 999isize;
                let c2 = (p.c.hmget_key_ts)(
                    cp,
                    elemsize,
                    key.ptr() as *mut c_void,
                    8,
                    &mut ct,
                    mode,
                );
                let r2 = (p.rs.hmget_key_ts)(
                    rp,
                    elemsize,
                    key.ptr() as *mut c_void,
                    8,
                    &mut rt,
                    mode,
                );
                assert_eq!((ct, rt), (-1, -1), "row13");
                assert_eq!(c2, cp);
                assert_eq!(r2, rp);
                eq_snap(
                    "row13",
                    &snap_hash(c2, elemsize, ElemFmt::Raw),
                    &snap_hash(r2, elemsize, ElemFmt::Raw),
                );

                // row 15: hmget_key mirrors both into header->temp
                let c3 = (p.c.hmget_key)(cp, elemsize, key.ptr() as *mut c_void, 8, mode);
                let r3 = (p.rs.hmget_key)(rp, elemsize, key.ptr() as *mut c_void, 8, mode);
                assert_eq!(snap_hash(c3, elemsize, ElemFmt::Raw).temp, -1);
                eq_snap(
                    "row15",
                    &snap_hash(c3, elemsize, ElemFmt::Raw),
                    &snap_hash(r3, elemsize, ElemFmt::Raw),
                );
                (p.c.hmfree_func)((c3 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (p.rs.hmfree_func)((r3 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
    // row 14: present table, absent key
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    for i in 0..20 {
        m.put_binary(&i32k(i), &rng.bytes(12));
    }
    for k in [-1i32, 20, 21, 1000, i32::MIN, i32::MAX] {
        assert_eq!(m.get(&i32k(k)), -1, "row14 key={k}");
        assert_eq!(m.get_ts(&i32k(k)), -1, "row14 ts key={k}");
    }
    m.free();
}

// ---------------------------------------------------------------------------
// rows 17-19: hmput_default
// ---------------------------------------------------------------------------

#[test]
fn err17_18_19_hmput_default_paths() {
    let (p, _g) = libs();
    unsafe {
        for elemsize in [1usize, 8, 16, 24] {
            // row 17
            let cp = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let rp = (p.rs.hmput_default)(std::ptr::null_mut(), elemsize);
            let cs = snap_hash(cp, elemsize, ElemFmt::Raw);
            eq_snap("row17", &cs, &snap_hash(rp, elemsize, ElemFmt::Raw));
            assert_eq!(cs.length, 1);
            assert!(cs.elem0.iter().all(|&b| b == 0));
            // row 19: force raw length back to 0 and call again
            (*((cp as *mut u8).sub(elemsize) as *mut ArrHeader).sub(1)).length = 0;
            (*((rp as *mut u8).sub(elemsize) as *mut ArrHeader).sub(1)).length = 0;
            let c2 = (p.c.hmput_default)(cp, elemsize);
            let r2 = (p.rs.hmput_default)(rp, elemsize);
            eq_snap(
                "row19",
                &snap_hash(c2, elemsize, ElemFmt::Raw),
                &snap_hash(r2, elemsize, ElemFmt::Raw),
            );
            assert_eq!(snap_hash(c2, elemsize, ElemFmt::Raw).length, 1);
            // row 18: NOP on a non-empty map
            let c3 = (p.c.hmput_default)(c2, elemsize);
            let r3 = (p.rs.hmput_default)(r2, elemsize);
            assert_eq!(c3, c2);
            assert_eq!(r3, r2);
            eq_snap(
                "row18",
                &snap_hash(c3, elemsize, ElemFmt::Raw),
                &snap_hash(r3, elemsize, ElemFmt::Raw),
            );
            (p.c.hmfree_func)((c3 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.rs.hmfree_func)((r3 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 20-22: hmput_key table creation & growth
// ---------------------------------------------------------------------------

#[test]
fn err20_21_22_hmput_key_table_creation_and_growth() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 20);
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    // row 20/21: first put allocates the array and an 8-slot table
    m.put_binary(&i32k(1), &rng.bytes(12));
    let t = m.snap_c().table.clone().unwrap();
    assert_eq!(t.slot_count, 8);
    assert_eq!(t.used_count_threshold, 6);
    assert_eq!(t.tombstone_count_threshold, 1);
    // row 43 of ERRORS.md: 8-slot tables never shrink
    assert_eq!(t.used_count_shrink_threshold, 0);
    assert_eq!(t.arena_mode, 0, "BINARY mode => string.mode 0");
    // row 22: growth exactly when used_count reaches the threshold
    let mut expect = 8usize;
    for k in 2..=200i32 {
        m.put_binary(&i32k(k), &rng.bytes(12));
        let t = m.snap_c().table.clone().unwrap();
        if t.slot_count != expect {
            assert_eq!(t.slot_count, expect * 2, "table must exactly double");
            expect = t.slot_count;
        }
        assert!(t.used_count < t.used_count_threshold || t.used_count == t.used_count_threshold);
    }
    m.free();

    // STRING mode: implicit creation must set string.mode = SH_DEFAULT
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_string());
    m.put_string(b"abc", &rng.bytes(8));
    assert_eq!(
        m.snap_c().table.as_ref().unwrap().arena_mode,
        SH_DEFAULT as u8
    );
    m.free();
}

// ---------------------------------------------------------------------------
// row 23: the `if (hash < 2) hash += 2` fixup
// ---------------------------------------------------------------------------

#[test]
fn err23_hash_below_2_fixup() {
    // The fixup only fires for keys whose raw siphash / string hash is 0 or 1,
    // i.e. with probability 2^-63 per key.  A bounded search confirms it is
    // unreachable in practice AND that the two hash implementations agree on
    // every candidate examined, which is what makes the fixup equivalent.
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 23);
    let mut found = 0usize;
    for _ in 0..200_000 {
        let mut b = rng.bytes(8);
        let seed = rng.next_u64() as usize;
        unsafe {
            let x = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, seed);
            let y = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, seed);
            assert_eq!(x, y);
            if x < 2 {
                found += 1;
            }
        }
        let n = 1 + rng.below(16);
        let k = CKey::new(&rng.nonnul(n));
        unsafe {
            let x = (p.c.hash_string)(k.ptr(), seed);
            let y = (p.rs.hash_string)(k.ptr(), seed);
            assert_eq!(x, y);
            if x < 2 {
                found += 1;
            }
        }
    }
    assert_eq!(found, 0, "unexpectedly found a hash < 2 candidate");

    // The observable purpose of the fixup is that no live entry can ever carry
    // the HASH_EMPTY (0) or HASH_DELETED (1) sentinel.  Verify that invariant
    // holds identically in both libraries over a churn workload.
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let mut live: Vec<i32> = Vec::new();
    for _ in 0..600 {
        let k = (rng.below(80) as i32) * 5;
        if rng.next_u64() % 3 == 0 {
            m.del(&i32k(k));
            live.retain(|&x| x != k);
        } else {
            m.put_binary(&i32k(k), &rng.bytes(12));
            if !live.contains(&k) {
                live.push(k);
            }
        }
        for snap in [m.snap_c(), m.snap_r()] {
            let t = snap.table.unwrap();
            for (h, idx) in &t.buckets {
                for j in 0..8 {
                    if idx[j] >= 0 {
                        assert!(h[j] >= 2, "in-use slot carries a sentinel hash {}", h[j]);
                    }
                }
            }
        }
    }
    m.free();
}

// ---------------------------------------------------------------------------
// row 24: tombstone reuse in hmput_key
// ---------------------------------------------------------------------------

#[test]
fn err24_tombstone_reuse() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 24);
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let keys: Vec<i32> = (0..200).collect();
    for &k in &keys {
        m.put_binary(&i32k(k), &rng.bytes(12));
    }
    let mut reuses = 0usize;
    for round in 0..10 {
        for &k in &keys {
            assert_eq!(m.del(&i32k(k)), 1);
            let before = m.snap_c().table.clone().unwrap();
            m.put_binary(&i32k(k), &rng.bytes(12));
            let after = m.snap_c().table.clone().unwrap();
            if after.tombstone_count < before.tombstone_count
                && after.slot_count == before.slot_count
            {
                reuses += 1;
            }
        }
        let _ = round;
    }
    assert!(reuses > 0, "expected the tombstone-reuse branch to be taken");
    m.free();
}

// ---------------------------------------------------------------------------
// rows 25, 33, 34, 42, 45: asserts that must never fire
// ---------------------------------------------------------------------------

#[test]
fn err25_33_42_45_live_asserts_never_fire() {
    // `STBDS_ASSERT` is live in the C build (NDEBUG is not defined) and
    // `assert!` is live in the Rust build.  A heavy mixed workload must not
    // trip any of them in either library.
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 25);
    for trial in 0..8u64 {
        reset_seed(p, rng.next_u64() as usize);
        let mut m = Maps::empty(p, cfg_struct());
        let mut live: Vec<i32> = Vec::new();
        for _ in 0..3000 {
            let k = rng.next_i32() % 500;
            match rng.below(3) {
                0 => {
                    m.put_binary(&i32k(k), &rng.bytes(12));
                    if !live.contains(&k) {
                        live.push(k);
                    }
                }
                1 => {
                    m.del(&i32k(k));
                    live.retain(|&x| x != k);
                }
                _ => {
                    m.get(&i32k(k));
                }
            }
            // ERRORS.md row 42/43: the make_hash_index invariants
            if let Some(t) = m.snap_c().table.clone() {
                assert!(t.used_count_threshold + t.tombstone_count_threshold < t.slot_count);
                if t.slot_count <= 8 {
                    assert_eq!(t.used_count_shrink_threshold, 0);
                }
            }
        }
        let _ = trial;
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 26-29: hmput_key storage-mode and keysize edge cases
// ---------------------------------------------------------------------------

#[test]
fn err26_default_memcpy_branch() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 26);
    let cfg = MapCfg {
        elemsize: 16,
        keysize: 8,
        keyoffset: 0,
        put_mode: HM_BINARY,
        del_mode: HM_BINARY,
        fmt: ElemFmt::Raw,
        value_off: 8,
        value_len: 8,
    };
    // string.mode values that are NOT 1/2/3 all take the `default:` branch
    for shmode in [0i32, 4, 5, 100, 255] {
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::shmode(p, cfg, shmode);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for _ in 0..40 {
            let k = rng.bytes(8);
            if keys.contains(&k) {
                continue;
            }
            keys.push(k.clone());
            let idx = m.put_binary(&k, &rng.bytes(8));
            // the key bytes must have been memcpy'd verbatim into the element
            unsafe {
                let e = std::slice::from_raw_parts(
                    (m.ch as *const u8).add(16 * idx as usize),
                    8,
                );
                assert_eq!(e, &k[..], "shmode={shmode} memcpy branch");
                let e = std::slice::from_raw_parts(
                    (m.rh as *const u8).add(16 * idx as usize),
                    8,
                );
                assert_eq!(e, &k[..], "shmode={shmode} memcpy branch (Rust)");
            }
        }
        m.free();
    }
}

#[test]
fn err27_28_mode_boundaries() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 27);
    // one step either side of the STBDS_HM_STRING boundary
    for (mode, expect_string) in [
        (i32::MIN, false),
        (-2, false),
        (-1, false),
        (0, false),
        (1, true),
        (2, true),
        (3, true),
        (1000, true),
        (i32::MAX, true),
    ] {
        reset_seed(p, DEFAULT_SEED);
        let mut cfg = if expect_string {
            cfg_string()
        } else {
            cfg_struct()
        };
        cfg.put_mode = mode;
        cfg.del_mode = mode;
        let mut m = Maps::empty(p, cfg);
        if expect_string {
            m.put_string(b"boundary-probe", &rng.bytes(8));
        } else {
            m.put_binary(&i32k(7), &rng.bytes(12));
        }
        let got = m.snap_c().table.as_ref().unwrap().arena_mode;
        let want = if expect_string { SH_DEFAULT as u8 } else { 0u8 };
        assert_eq!(got, want, "mode={mode} string.mode");
        assert_eq!(m.snap_r().table.as_ref().unwrap().arena_mode, want);
        m.free();
    }
}

#[test]
fn err29_keysize_zero_aliases_everything() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 29);
    let cfg = MapCfg {
        elemsize: 16,
        keysize: 0,
        keyoffset: 0,
        put_mode: HM_BINARY,
        del_mode: HM_BINARY,
        fmt: ElemFmt::Raw,
        value_off: 0,
        value_len: 16,
    };
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg);
    let idx0 = m.put_binary(&[], &rng.bytes(16));
    for _ in 0..50 {
        // every distinct "key" hashes the same 0 bytes and memcmp's equal
        let i = m.put_binary(&[], &rng.bytes(16));
        assert_eq!(i, idx0, "keysize=0 must alias to one entry");
    }
    assert_eq!(m.snap_c().length, 2);
    assert_eq!(m.get(&[]), idx0);
    assert_eq!(m.del(&[]), 1);
    assert_eq!(m.get(&[]), -1);
    m.free();
}

// ---------------------------------------------------------------------------
// rows 30-32, 37-41: hmdel_key
// ---------------------------------------------------------------------------

#[test]
fn err30_hmdel_null_returns_null() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 30);
    unsafe {
        for elemsize in [0usize, 1, 8, 16, 24] {
            for mode in [HM_BINARY, HM_STRING, 2, -1, i32::MAX, i32::MIN] {
                for keyoffset in [0usize, 4, 8] {
                    let key = CKey::new(&rng.nonnul(8));
                    let cp = (p.c.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.ptr() as *mut c_void,
                        8,
                        keyoffset,
                        mode,
                    );
                    let rp = (p.rs.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.ptr() as *mut c_void,
                        8,
                        keyoffset,
                        mode,
                    );
                    assert!(cp.is_null(), "C hmdel_key(NULL) must return NULL");
                    assert!(rp.is_null(), "Rust hmdel_key(NULL) must return NULL");
                }
            }
        }
    }
}

#[test]
fn err31_hmdel_tableless_sets_temp_zero() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 31);
    unsafe {
        for elemsize in [8usize, 16, 24] {
            for mode in [HM_BINARY, HM_STRING, 2] {
                let cp = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
                let rp = (p.rs.hmput_default)(std::ptr::null_mut(), elemsize);
                // poison header->temp so the write to 0 is observable
                (*((cp as *mut u8).sub(elemsize) as *mut ArrHeader).sub(1)).temp = 4242;
                (*((rp as *mut u8).sub(elemsize) as *mut ArrHeader).sub(1)).temp = 4242;
                let key = CKey::new(&rng.nonnul(8));
                let c2 = (p.c.hmdel_key)(
                    cp,
                    elemsize,
                    key.ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                let r2 = (p.rs.hmdel_key)(
                    rp,
                    elemsize,
                    key.ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                assert_eq!(c2, cp);
                assert_eq!(r2, rp);
                let cs = snap_hash(c2, elemsize, ElemFmt::Raw);
                eq_snap("row31", &cs, &snap_hash(r2, elemsize, ElemFmt::Raw));
                assert_eq!(cs.temp, 0, "hmdel_key must reset header->temp to 0");
                (p.c.hmfree_func)((c2 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (p.rs.hmfree_func)((r2 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

#[test]
fn err32_hmdel_absent_key_is_nop() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 32);
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let keys: Vec<i32> = (0..50).map(|i| i * 11).collect();
    for &k in &keys {
        m.put_binary(&i32k(k), &rng.bytes(12));
    }
    let before = m.snap_c();
    for k in [1i32, 2, 3, -5, 1000, i32::MIN, i32::MAX] {
        assert_eq!(m.del(&i32k(k)), 0, "absent delete must return 0");
    }
    let after = m.snap_c();
    assert_eq!(before.length, after.length, "length must be unchanged");
    assert_eq!(
        before.table.as_ref().unwrap().used_count,
        after.table.as_ref().unwrap().used_count
    );
    assert_eq!(
        before.table.as_ref().unwrap().tombstone_count,
        after.table.as_ref().unwrap().tombstone_count
    );
    m.free();
}

#[test]
fn err37_mode2_skips_strdup_free() {
    // With `mode == 2` and `string.mode == SH_STRDUP`, hmdel_key does NOT free
    // the duplicate (the check is `mode == STBDS_HM_STRING`, i.e. exactly 1).
    // Both libraries must take the identical branch; tail-only deletes keep the
    // rest of the function on its defined path.
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 37);
    for &del_mode in &[1i32, 2, 3, 1000] {
        reset_seed(p, DEFAULT_SEED);
        let mut cfg = cfg_string();
        cfg.del_mode = del_mode;
        let mut m = Maps::shmode(p, cfg, SH_STRDUP);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..40 {
            let k = format!("strdup-key-{i}").into_bytes();
            keys.push(k.clone());
            m.put_string(&k, &rng.bytes(8));
        }
        while let Some(k) = keys.pop() {
            assert_eq!(m.del(&k), 1, "del_mode={del_mode}");
        }
        m.free();
    }
}

#[test]
fn err38_39_shrink_and_rebuild_branches() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 38);

    // row 38: shrink
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let mut keys: Vec<i32> = Vec::new();
    while keys.len() < 200 {
        let k = rng.next_i32();
        if !keys.contains(&k) {
            keys.push(k);
            m.put_binary(&i32k(k), &rng.bytes(12));
        }
    }
    let mut shrinks = 0usize;
    let mut prev = m.snap_c().table.as_ref().unwrap().slot_count;
    while let Some(k) = keys.pop() {
        m.del(&i32k(k));
        let now = m.snap_c().table.as_ref().unwrap().slot_count;
        if now < prev {
            shrinks += 1;
            assert_eq!(now, prev / 2, "shrink must halve slot_count");
            prev = now;
        }
    }
    assert!(shrinks >= 3, "expected >=3 shrinks, saw {shrinks}");
    assert_eq!(prev, 8, "must bottom out at STBDS_BUCKET_LENGTH");
    m.free();

    // row 39: same-size rebuild driven by tombstone_count_threshold
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let mut keys: Vec<i32> = Vec::new();
    while keys.len() < 700 {
        let k = rng.next_i32();
        if !keys.contains(&k) {
            keys.push(k);
            m.put_binary(&i32k(k), &rng.bytes(12));
        }
    }
    let mut rebuilds = 0usize;
    let mut prev_t = 0usize;
    for _ in 0..250 {
        let k = keys.pop().unwrap();
        m.del(&i32k(k));
        let t = m.snap_c().table.clone().unwrap();
        if t.tombstone_count < prev_t && t.slot_count == 1024 {
            rebuilds += 1;
        }
        prev_t = t.tombstone_count;
    }
    assert!(rebuilds >= 1, "expected a same-size rebuild");
    m.free();
}

#[test]
fn err40_41_delete_tail_and_single_entry() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 40);
    // row 41: exactly one entry
    for mode in [HM_BINARY] {
        reset_seed(p, DEFAULT_SEED);
        let mut cfg = cfg_struct();
        cfg.put_mode = mode;
        cfg.del_mode = mode;
        let mut m = Maps::empty(p, cfg);
        m.put_binary(&i32k(42), &rng.bytes(12));
        let t = m.snap_c().table.clone().unwrap();
        assert_eq!(t.used_count, 1);
        assert_eq!(m.snap_c().length, 2);
        assert_eq!(m.del(&i32k(42)), 1);
        let t = m.snap_c().table.clone().unwrap();
        assert_eq!(t.used_count, 0);
        assert_eq!(t.tombstone_count, 1);
        assert_eq!(m.snap_c().length, 1);
        assert_eq!(m.get(&i32k(42)), -1);
        m.free();
    }
    // row 40: old_index == final_index for every delete
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    let mut keys: Vec<i32> = (0..80).map(|i| i * 17 + 3).collect();
    for &k in &keys {
        m.put_binary(&i32k(k), &rng.bytes(12));
    }
    while let Some(k) = keys.pop() {
        assert_eq!(m.del(&i32k(k)), 1);
    }
    m.free();
}

// ---------------------------------------------------------------------------
// rows 44, 46-48, 50: shmode_func and the string arena
// ---------------------------------------------------------------------------

#[test]
fn err44_shmode_out_of_range_truncation() {
    let (p, _g) = libs();
    unsafe {
        for shmode in [
            0i32, 1, 2, 3, 4, 5, 255, 256, 257, 258, 259, 260, 511, 512, 513, -1, -2, -255, -256,
            -257, i32::MIN, i32::MAX,
        ] {
            reset_seed(p, DEFAULT_SEED);
            let ch = (p.c.shmode_func)(16, shmode);
            let rh = (p.rs.shmode_func)(16, shmode);
            let cs = snap_hash(ch, 16, ElemFmt::Raw);
            let rs = snap_hash(rh, 16, ElemFmt::Raw);
            eq_snap(&format!("shmode_func({shmode})"), &cs, &rs);
            let want = (shmode as u32 & 0xff) as u8;
            assert_eq!(
                cs.table.as_ref().unwrap().arena_mode,
                want,
                "shmode_func({shmode}) must truncate to {want}"
            );
            assert_eq!(cs.length, 1);
            assert_eq!(cs.table.as_ref().unwrap().slot_count, 8);
            (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
            (p.rs.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

#[test]
fn err46_47_48_50_arena_boundaries() {
    let (p, _g) = libs();
    let mut arena_c = StringArena {
        storage: std::ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut arena_r = arena_c;
    unsafe {
        // row 48: empty string into a fresh arena
        let k = CKey::new(b"");
        let cp = (p.c.stralloc)(&mut arena_c as *mut StringArena as *mut c_void, k.ptr());
        let rp = (p.rs.stralloc)(&mut arena_r as *mut StringArena as *mut c_void, k.ptr());
        assert_eq!(CStr::from_ptr(cp).to_bytes(), b"");
        assert_eq!(CStr::from_ptr(rp).to_bytes(), b"");
        assert_eq!(snap_arena(&arena_c), snap_arena(&arena_r));
        assert_eq!(arena_c.remaining, 511);
        assert_eq!(arena_c.block, 1);

        // row 46: len exactly at / one past the blocksize boundary
        for len in [510usize, 511, 512, 513] {
            let mut a = StringArena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            let mut b = a;
            let s = vec![b'x'; len];
            let k = CKey::new(&s);
            let x = (p.c.stralloc)(&mut a as *mut StringArena as *mut c_void, k.ptr());
            let y = (p.rs.stralloc)(&mut b as *mut StringArena as *mut c_void, k.ptr());
            assert_eq!(CStr::from_ptr(x).to_bytes().len(), len);
            assert_eq!(CStr::from_ptr(y).to_bytes().len(), len);
            assert_eq!(
                snap_arena(&a),
                snap_arena(&b),
                "boundary len={len} (len+1 vs blocksize 512)"
            );
            if len + 1 > 512 {
                assert_eq!(a.remaining, 0, "oversized => remaining 0");
            } else {
                assert_eq!(a.remaining, 512 - (len + 1));
            }
            (p.c.strreset)(&mut a as *mut StringArena as *mut c_void);
            (p.rs.strreset)(&mut b as *mut StringArena as *mut c_void);
        }

        // row 47: block counter saturation is checked in phase_b_arena row56;
        // here verify the saturation constant boundary directly by driving
        // `block` up with allocations that exactly exhaust each block.
        let mut a = StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut b = a;
        for _ in 0..3000 {
            let s = vec![b'q'; 400];
            let k = CKey::new(&s);
            (p.c.stralloc)(&mut a as *mut StringArena as *mut c_void, k.ptr());
            (p.rs.stralloc)(&mut b as *mut StringArena as *mut c_void, k.ptr());
            assert_eq!(snap_arena(&a), snap_arena(&b));
            assert!(a.block <= 23, "block must saturate at 23, got {}", a.block);
        }
        (p.c.strreset)(&mut a as *mut StringArena as *mut c_void);
        (p.rs.strreset)(&mut b as *mut StringArena as *mut c_void);

        // row 50: strreset on a fresh arena, and twice in a row
        (p.c.strreset)(&mut arena_c as *mut StringArena as *mut c_void);
        (p.rs.strreset)(&mut arena_r as *mut StringArena as *mut c_void);
        assert_eq!(snap_arena(&arena_c), snap_arena(&arena_r));
        (p.c.strreset)(&mut arena_c as *mut StringArena as *mut c_void);
        (p.rs.strreset)(&mut arena_r as *mut StringArena as *mut c_void);
        assert_eq!(snap_arena(&arena_c), snap_arena(&arena_r));
        assert_eq!(
            snap_arena(&arena_c),
            ArenaSnap {
                remaining: 0,
                block: 0,
                mode: 0,
                block_count: 0
            }
        );
    }
}

// ---------------------------------------------------------------------------
// rows 52-59: hash-function and seed boundaries
// ---------------------------------------------------------------------------

#[test]
fn err52_53_hash_string_boundaries() {
    let (p, _g) = libs();
    unsafe {
        // row 52: empty string
        let k = CKey::new(b"");
        for seed in [0usize, 1, DEFAULT_SEED, usize::MAX] {
            assert_eq!((p.c.hash_string)(k.ptr(), seed), (p.rs.hash_string)(k.ptr(), seed));
        }
        // row 53: bytes >= 0x80 must be added as unsigned, not sign-extended
        for v in 0x80u16..=0xff {
            let k = CKey::new(&[v as u8, v as u8, v as u8]);
            for seed in [0usize, DEFAULT_SEED, usize::MAX] {
                assert_eq!(
                    (p.c.hash_string)(k.ptr(), seed),
                    (p.rs.hash_string)(k.ptr(), seed),
                    "byte {v:#02x} seed {seed:#x}"
                );
            }
        }
        // a long all-high-bit string
        let k = CKey::new(&vec![0xffu8; 200]);
        assert_eq!(
            (p.c.hash_string)(k.ptr(), DEFAULT_SEED),
            (p.rs.hash_string)(k.ptr(), DEFAULT_SEED)
        );
    }
}

#[test]
fn err54_55_56_57_58_hash_bytes_boundaries() {
    let (p, _g) = libs();
    unsafe {
        // row 58: NULL pointer with len 0
        for seed in [0usize, 1, DEFAULT_SEED, usize::MAX] {
            assert_eq!(
                (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed),
                (p.rs.hash_bytes)(std::ptr::null_mut(), 0, seed),
                "hash_bytes(NULL,0,{seed:#x})"
            );
        }
        // rows 54-57: exhaustive over every tail length and every byte value at
        // the sign-extension-critical positions
        for len in 0usize..=8 {
            for pos in 0..len {
                for v in 0u16..=255 {
                    let mut b = vec![0x7fu8; len.max(1)];
                    b[pos] = v as u8;
                    for seed in [0usize, DEFAULT_SEED, usize::MAX] {
                        let x = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                        let y = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                        assert_eq!(x, y, "len={len} pos={pos} v={v:#02x} seed={seed:#x}");
                    }
                }
            }
        }
        // len one step past a block boundary in both directions
        for len in [7usize, 8, 9, 15, 16, 17, 63, 64, 65] {
            let mut b = vec![0x80u8; len];
            for (i, x) in b.iter_mut().enumerate() {
                *x = (0x80 + i) as u8;
            }
            assert_eq!(
                (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, DEFAULT_SEED),
                (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, DEFAULT_SEED),
                "len={len}"
            );
        }
    }
}

#[test]
fn err59_rand_seed_extremes() {
    let (p, _g) = libs();
    unsafe {
        for seed in [0usize, 1, usize::MAX, usize::MAX - 1, 1 << 63, DEFAULT_SEED] {
            reset_seed(p, seed);
            // 4 successive tables observe the full seed*a+b chain with wrap-around
            let mut cs = Vec::new();
            let mut rs = Vec::new();
            let mut ch = Vec::new();
            let mut rh = Vec::new();
            for _ in 0..4 {
                let c = (p.c.shmode_func)(16, SH_NONE);
                let r = (p.rs.shmode_func)(16, SH_NONE);
                cs.push(snap_hash(c, 16, ElemFmt::Raw).table.unwrap().seed);
                rs.push(snap_hash(r, 16, ElemFmt::Raw).table.unwrap().seed);
                ch.push(c);
                rh.push(r);
            }
            assert_eq!(cs, rs, "seed chain for rand_seed({seed:#x})");
            assert_eq!(cs[0], seed, "first table must use the seed verbatim");
            for (c, r) in ch.into_iter().zip(rh) {
                (p.c.hmfree_func)((c as *mut u8).sub(16) as *mut c_void, 16);
                (p.rs.hmfree_func)((r as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
    reset_seed(p, DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// rows 60-63: strkey / arr_ins
// ---------------------------------------------------------------------------

#[test]
fn err60_strkey_extremes() {
    let (p, _g) = libs();
    unsafe {
        for n in [
            0i32,
            1,
            -1,
            9,
            -9,
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
            -1000000000,
            1000000000,
        ] {
            let a = CStr::from_ptr((p.c.strkey)(n)).to_bytes().to_vec();
            let b = CStr::from_ptr((p.rs.strkey)(n)).to_bytes().to_vec();
            assert_eq!(a, b, "strkey({n})");
            assert_eq!(a, format!("test_{n}").into_bytes());
            assert!(a.len() < 256, "must fit the 256-byte static buffer");
        }
    }
}

#[test]
fn err61_62_63_arr_ins_asserts() {
    let (p, _g) = libs();
    unsafe {
        // row 63 first: num == 4 aliases the sentinel that row 62 checks
        for n in [4i32, 0, 1, -1, i32::MIN, i32::MAX, 5, -4] {
            (p.c.arr_ins)(n);
            (p.rs.arr_ins)(n);
        }
        let mut rng = Rng::new(0xC0FFEE ^ 61);
        for _ in 0..2000 {
            let n = rng.next_i32();
            (p.c.arr_ins)(n);
            (p.rs.arr_ins)(n);
        }
    }
}
