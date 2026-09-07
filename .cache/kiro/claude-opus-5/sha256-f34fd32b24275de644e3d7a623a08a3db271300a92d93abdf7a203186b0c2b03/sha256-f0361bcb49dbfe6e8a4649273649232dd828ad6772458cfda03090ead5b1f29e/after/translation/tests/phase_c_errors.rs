//! Phase C — one differential test per row of `ERRORS.md`, plus the generic
//! FFI-boundary cases G1..G7.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void, CStr};

fn bin_kind(keysize: usize, voff: usize, vsize: usize) -> KeyKind {
    KeyKind::Binary {
        keysize,
        value_offset: voff,
        value_size: vsize,
    }
}

fn str_kind(voff: usize, vsize: usize) -> KeyKind {
    KeyKind::StringPtr {
        value_offset: voff,
        value_size: vsize,
    }
}

// ---------------------------------------------------------------------------
// rows 1..4 — stbds_arrgrowf rejections
// ---------------------------------------------------------------------------

/// row 1 — `min_cap <= arrcap(a)` → returns `a` unchanged, no realloc
#[test]
fn e01_arrgrowf_early_out() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        let mut c = (lc.arrgrowf)(std::ptr::null_mut(), 8, 0, 64);
        let mut r = (lr.arrgrowf)(std::ptr::null_mut(), 8, 0, 64);
        (*(c as *mut ArrayHeader).wrapping_sub(1)).length = 10;
        (*(r as *mut ArrayHeader).wrapping_sub(1)).length = 10;
        for (addlen, min_cap) in [(0usize, 0usize), (0, 1), (0, 64), (10, 0), (54, 0), (1, 11)] {
            let c2 = (lc.arrgrowf)(c, 8, addlen, min_cap);
            let r2 = (lr.arrgrowf)(r, 8, addlen, min_cap);
            assert_eq!(c2, c, "C must return `a` unchanged ({addlen},{min_cap})");
            assert_eq!(r2, r, "Rust must return `a` unchanged ({addlen},{min_cap})");
            assert_eq!(arr_snap(c2, 0), arr_snap(r2, 0));
            assert_eq!(arr_snap(c2, 0).length, 10);
            assert_eq!(arr_snap(c2, 0).capacity, 64);
            c = c2;
            r = r2;
        }
        (lc.arrfreef)(c);
        (lr.arrfreef)(r);
    }
}

/// row 2 — `a == NULL` → header initialised to `{0, cap, NULL, 0}`
#[test]
fn e02_arrgrowf_fresh_header() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        for (elemsize, addlen, min_cap) in [
            (1usize, 0usize, 1usize),
            (8, 1, 0),
            (8, 0, 4),
            (16, 7, 3),
            (32, 0, 1000),
            (64, 500, 0),
        ] {
            let c = (lc.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
            let r = (lr.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
            let sc = arr_snap(c, 0);
            assert_eq!(sc, arr_snap(r, 0));
            assert_eq!(sc.length, 0);
            assert_eq!(sc.temp, 0);
            assert!(sc.hash_table_null);
            (lc.arrfreef)(c);
            (lr.arrfreef)(r);
        }
    }
}

/// row 3 — `NULL` + `addlen == 0` + `min_cap == 0` → `NULL`, no allocation
#[test]
fn e03_arrgrowf_returns_null() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        for elemsize in [0usize, 1, 8, 16, usize::MAX] {
            let c = (lc.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let r = (lr.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(c.is_null(), "C should return NULL (elemsize={elemsize})");
            assert!(r.is_null(), "Rust should return NULL (elemsize={elemsize})");
        }
    }
}

/// row 4 — `elemsize == 0`
#[test]
fn e04_arrgrowf_elemsize_zero() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        for (addlen, min_cap) in [(0usize, 1usize), (1, 0), (0, 9), (100, 0)] {
            let c = (lc.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            let r = (lr.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            assert_eq!(arr_snap(c, 0), arr_snap(r, 0), "({addlen},{min_cap})");
            (lc.arrfreef)(c);
            (lr.arrfreef)(r);
        }
    }
}

/// row 5 — `stbds_arrfreef(NULL)` frees `(stbds_array_header *) NULL - 1`.
/// Undefined in C; both libraries must fail the same way (glibc `free()`
/// "invalid pointer" → `SIGABRT`).
#[test]
fn e05_arrfreef_null_crashes_identically() {
    let _g = serial();
    let (lc, lr) = libs();
    let f_c = lc.arrfreef;
    let f_r = lr.arrfreef;
    let (oc, _) = child_run("c_arrfree", move || unsafe { f_c(std::ptr::null_mut()) });
    let (or_, _) = child_run("r_arrfree", move || unsafe { f_r(std::ptr::null_mut()) });
    assert_eq!(oc, or_, "arrfreef(NULL) outcome diverged");
    assert_ne!(oc.signal, 0, "expected a fatal signal, got {oc:?}");
    // and the *defined* case must work in-process on both
    unsafe {
        let c = (lc.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        let r = (lr.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        (lc.arrfreef)(c);
        (lr.arrfreef)(r);
    }
}

/// row 6 — the `stbds_make_hash_index` assertion
/// (`used_count_threshold + tombstone_count_threshold < slot_count`).
/// Unreachable through the public API, so it is provoked by handing the library
/// a table whose `slot_count` is 0; both must abort with the same signal and
/// name the same assertion, function and line.
#[test]
fn e06_make_hash_index_assert() {
    let _g = serial();
    let (lc, lr) = libs();
    let mk = |put: FnHmPutKey, shmode: FnShmodeFunc| {
        move || unsafe {
            let a = shmode(16, STBDS_SH_DEFAULT);
            let t = (*header_of(a, 16)).hash_table as *mut HashIndex;
            // force the "grow" path with slot_count * 2 == 0
            (*t).slot_count = 0;
            (*t).used_count = 1;
            (*t).used_count_threshold = 0;
            let key = b"assertme\0";
            put(a, 16, key.as_ptr() as *mut c_void, 8, STBDS_HM_STRING);
        }
    };
    let (oc, ec) = child_run("c_mhi", mk(lc.hmput_key, lc.shmode_func));
    let (or_, er) = child_run("r_mhi", mk(lr.hmput_key, lr.shmode_func));
    assert_eq!(oc, or_, "make_hash_index assert outcome diverged\nC stderr : {}\nR stderr : {}",
        String::from_utf8_lossy(&ec), String::from_utf8_lossy(&er));
    assert_eq!(oc.signal, 6, "expected SIGABRT, got {oc:?}");
    let (sc, sr) = (
        String::from_utf8_lossy(&ec).to_string(),
        String::from_utf8_lossy(&er).to_string(),
    );
    for needle in [
        "t->used_count_threshold + t->tombstone_count_threshold < t->slot_count",
        "stbds_make_hash_index",
        ":401:",
        "lib.c",
    ] {
        assert!(sc.contains(needle), "C stderr missing {needle:?}: {sc}");
        assert!(sr.contains(needle), "Rust stderr missing {needle:?}: {sr}");
    }
}

// ---------------------------------------------------------------------------
// rows 7..8 — `stbds_hm_find_slot` returning -1 from each of its two scans
// ---------------------------------------------------------------------------

/// rows 7 + 8 — every miss must be reported as `-1` by both libraries; the
/// randomized keys make the probe start land on every `pos & 7` value, so both
/// the upper scan and the wrapped lower scan produce the sentinel.
#[test]
fn e07_e08_find_slot_miss() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0708);
    for trial in 0..25usize {
        reseed(0x2000 + trial);
        let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        for i in 0..50u32 {
            m.put(&(rng.next_u32() | 1).to_le_bytes(), &i.to_le_bytes());
        }
        m.assert_same("populate");
        for _ in 0..400 {
            let k = rng.next_u32() & !1; // even keys were never inserted
            let (a, b) = m.geti(&k.to_le_bytes());
            assert_eq!(a, b, "miss diverged (trial={trial}, k={k:#x})");
            assert_eq!(a, -1, "miss must be -1");
            let (a, b) = m.geti_ts(&k.to_le_bytes());
            assert_eq!((a, b), (-1, -1));
        }
        m.free();
    }
    reseed(DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// rows 9..10 — stbds_hmfree_func guards
// ---------------------------------------------------------------------------

/// row 9 — `hmfree_func(NULL, elemsize)` is a no-op on both
#[test]
fn e09_hmfree_null() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        for elemsize in [0usize, 1, 8, 16, 1024, usize::MAX] {
            (lc.hmfree_func)(std::ptr::null_mut(), elemsize);
            (lr.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

/// row 10 — `hmfree_func` on an array with `hash_table == NULL`
#[test]
fn e10_hmfree_no_table() {
    let _g = serial();
    for elemsize in [8usize, 16, 32] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(elemsize, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        m.put_default();
        let (s, _) = m.snap_pair();
        assert!(s.table.is_none(), "precondition: no hash table");
        m.free(); // must not touch the strdup sweep / strreset paths
        assert!(m.c.is_null() && m.r.is_null());
    }
}

// ---------------------------------------------------------------------------
// rows 11..15 — lookup bootstrap / miss sentinels
// ---------------------------------------------------------------------------

/// row 11 — `hmget_key_ts(NULL, ...)`
#[test]
fn e11_hmget_ts_null() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        for elemsize in [1usize, 8, 16, 32, 64] {
            reseed(DEFAULT_SEED);
            let key = 1234u32.to_le_bytes();
            let mut tc: isize = 0x5a5a;
            let mut tr: isize = 0x5a5a;
            let c = (lc.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                4,
                &mut tc,
                STBDS_HM_BINARY,
            );
            let r = (lr.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                4,
                &mut tr,
                STBDS_HM_BINARY,
            );
            assert_eq!(tc, tr, "temp diverged (elemsize={elemsize})");
            assert_eq!(tc, -1, "must be STBDS_INDEX_EMPTY");
            assert!(!c.is_null() && !r.is_null());
            let kind = KeyKind::RawPrefix { n: elemsize };
            assert_eq!(map_snap(c, elemsize, kind), map_snap(r, elemsize, kind));
            assert_eq!(map_snap(c, elemsize, kind).length, 1);
            (lc.hmfree_func)((c as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (lr.hmfree_func)((r as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// row 12 — `hmget_key_ts` with `hash_table == 0` → `-1`, `a` returned as-is
#[test]
fn e12_hmget_ts_no_table() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 4, 8, STBDS_HM_BINARY, bin_kind(4, 8, 4));
    m.put_default();
    let (pc, pr) = (m.c, m.r);
    for k in [0u32, 1, 7, u32::MAX] {
        let (a, b) = m.geti_ts(&k.to_le_bytes());
        assert_eq!((a, b), (-1, -1), "k={k}");
        assert_eq!(m.c, pc, "C must return `a` unchanged");
        assert_eq!(m.r, pr, "Rust must return `a` unchanged");
    }
    m.assert_same("hmget_key_ts with no table");
    m.free();
}

/// row 13 — `hmget_key_ts` miss on a populated table
#[test]
fn e13_hmget_ts_miss() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for k in 1..30u32 {
        m.put(&(k * 2).to_le_bytes(), &k.to_le_bytes());
    }
    for k in (1..60u32).step_by(2) {
        let (a, b) = m.geti_ts(&k.to_le_bytes());
        assert_eq!((a, b), (-1, -1), "odd key {k} must miss");
    }
    m.free();
}

/// row 14 — `hmget_key` writes the sentinel into `stbds_header(...)->temp`
#[test]
fn e14_hmget_key_temp_on_miss() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for k in 1..30u32 {
        m.put(&(k * 2).to_le_bytes(), &k.to_le_bytes());
    }
    for k in (1..60u32).step_by(2) {
        let (a, b) = m.geti(&k.to_le_bytes());
        assert_eq!((a, b), (-1, -1), "odd key {k}");
        let (sc, sr) = m.snap_pair();
        assert_eq!(sc.temp, -1);
        assert_eq!(sr.temp, -1);
    }
    m.assert_same("hmget_key miss");
    m.free();
}

/// row 15 — `hmget_key(NULL, ...)` sets the *new* header's `temp` to -1
#[test]
fn e15_hmget_key_null_sets_temp() {
    let _g = serial();
    for elemsize in [8usize, 16, 32] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(elemsize, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        let (a, b) = m.geti(&5u32.to_le_bytes());
        assert_eq!((a, b), (-1, -1));
        let (sc, _) = m.snap_pair();
        assert_eq!(sc.length, 1);
        assert_eq!(sc.temp, -1);
        m.assert_same("hmget_key(NULL)");
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 16..18 — stbds_hmput_default
// ---------------------------------------------------------------------------

/// row 16 — `hmput_default(NULL, elemsize)`
#[test]
fn e16_hmput_default_null() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        for elemsize in [1usize, 8, 16, 32, 100] {
            reseed(DEFAULT_SEED);
            let c = (lc.hmput_default)(std::ptr::null_mut(), elemsize);
            let r = (lr.hmput_default)(std::ptr::null_mut(), elemsize);
            let kind = KeyKind::RawPrefix { n: elemsize };
            assert_eq!(map_snap(c, elemsize, kind), map_snap(r, elemsize, kind));
            let s = map_snap(c, elemsize, kind);
            assert_eq!(s.length, 1);
            assert!(s.table.is_none());
            assert_eq!(s.elements[0], ElemSnap::Raw(vec![0u8; elemsize]));
            (lc.hmfree_func)((c as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (lr.hmfree_func)((r as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// row 17 — `hmput_default` on a non-NULL array whose `length == 0`
#[test]
fn e17_hmput_default_length_zero() {
    let _g = serial();
    let (lc, lr) = libs();
    let elemsize = 16usize;
    unsafe {
        reseed(DEFAULT_SEED);
        // build an array with length 0 through the array back-end, then present
        // it as a hash-map handle (`arr + elemsize`).
        let ca = (lc.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        let ra = (lr.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        assert_eq!(arr_snap(ca, 0).length, 0);
        let c = (lc.hmput_default)((ca as *mut u8).add(elemsize) as *mut c_void, elemsize);
        let r = (lr.hmput_default)((ra as *mut u8).add(elemsize) as *mut c_void, elemsize);
        let kind = KeyKind::RawPrefix { n: elemsize };
        assert_eq!(map_snap(c, elemsize, kind), map_snap(r, elemsize, kind));
        assert_eq!(map_snap(c, elemsize, kind).length, 1);
        (lc.hmfree_func)((c as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (lr.hmfree_func)((r as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// row 18 — `hmput_default` returns `a` untouched when `length != 0`
#[test]
fn e18_hmput_default_noop() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 4, 8, STBDS_HM_BINARY, bin_kind(4, 8, 4));
    m.put_default();
    let (pc, pr) = (m.c, m.r);
    let before = m.snap_pair();
    for _ in 0..5 {
        m.put_default();
        assert_eq!(m.c, pc);
        assert_eq!(m.r, pr);
    }
    assert_eq!(before, m.snap_pair());
    m.free();
}

// ---------------------------------------------------------------------------
// rows 19..26 — stbds_hmput_key
// ---------------------------------------------------------------------------

/// row 19 — `hmput_key(NULL, ...)` bootstraps the reserved element
#[test]
fn e19_hmput_key_null_bootstrap() {
    let _g = serial();
    for elemsize in [8usize, 16, 24, 32] {
        reseed(DEFAULT_SEED);
        let voff = elemsize - 4;
        let mut m = MapPair::new(elemsize, 4, voff, STBDS_HM_BINARY, bin_kind(4, voff, 4));
        m.put(&1u32.to_le_bytes(), &2u32.to_le_bytes());
        m.assert_same(&format!("bootstrap elemsize={elemsize}"));
        let (s, _) = m.snap_pair();
        assert_eq!(s.length, 2, "reserved element + 1 real element");
        assert_eq!(s.table.as_ref().unwrap().slot_count, STBDS_BUCKET_LENGTH);
        m.free();
    }
}

/// row 20 — the lazily created table gets `string.mode` from `mode`
#[test]
fn e20_hmput_key_initial_string_mode() {
    let _g = serial();
    // mode >= STBDS_HM_STRING → STBDS_SH_DEFAULT
    for mode in [1i32, 2, 7, i32::MAX] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(16, 8, 8, mode, str_kind(8, 4));
        m.put(&cstr(b"keykey"), &1u32.to_le_bytes());
        m.assert_same(&format!("mode={mode}"));
        let (s, _) = m.snap_pair();
        assert_eq!(
            s.table.as_ref().unwrap().string_mode,
            STBDS_SH_DEFAULT as u8,
            "mode={mode}"
        );
        m.free();
    }
    // mode < STBDS_HM_STRING → 0
    for mode in [0i32, -1, i32::MIN] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(8, 4, 4, mode, bin_kind(4, 4, 4));
        m.put(&7u32.to_le_bytes(), &8u32.to_le_bytes());
        m.assert_same(&format!("mode={mode}"));
        let (s, _) = m.snap_pair();
        assert_eq!(s.table.as_ref().unwrap().string_mode, 0, "mode={mode}");
        m.free();
    }
}

/// row 21 — growth at `used_count >= used_count_threshold`; the rehashed table
/// inherits `string`/`seed` and does NOT re-derive `string.mode` from `mode`
#[test]
fn e21_hmput_key_grow_inherits() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    // create the table in BINARY mode (string.mode == 0) ...
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_BINARY, bin_kind(8, 8, 8));
    // the table is created with 8 slots; the 7th insert sees
    // `used_count (6) >= used_count_threshold (6)` and rehashes into 16.
    for k in 0..7u64 {
        m.put(&k.to_le_bytes(), &k.to_le_bytes());
        m.assert_same(&format!("binary put {k}"));
    }
    let (s, _) = m.snap_pair();
    assert_eq!(s.table.as_ref().unwrap().slot_count, 16, "should have grown");
    assert_eq!(
        s.table.as_ref().unwrap().string_mode,
        0,
        "string.mode must stay 0 across the rehash"
    );
    let seed_before = s.table.as_ref().unwrap().seed;
    for k in 7..40u64 {
        m.put(&k.to_le_bytes(), &k.to_le_bytes());
        m.assert_same(&format!("binary put {k}"));
    }
    let (s, _) = m.snap_pair();
    assert_eq!(
        s.table.as_ref().unwrap().seed,
        seed_before,
        "the per-table seed is inherited by every rehash"
    );
    m.free();
}

/// row 22 — the `(size_t) i+1 <= stbds_arrcap(a)` assertion never fires; the
/// invariant it guards is checked directly after every insert instead.
#[test]
fn e22_hmput_key_capacity_invariant() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0022);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for i in 0..500u32 {
        m.put(&rng.next_u32().to_le_bytes(), &i.to_le_bytes());
        let (sc, sr) = m.snap_pair();
        assert!(sc.length <= sc.capacity, "C length {} > cap {}", sc.length, sc.capacity);
        assert!(sr.length <= sr.capacity, "Rust length {} > cap {}", sr.length, sr.capacity);
        assert_eq!(sc, sr);
    }
    m.free();
}

/// row 23 — duplicate-key hit: `temp` reused, `used_count` unchanged, no new
/// element.  Covers both the upper scan and the wrapped lower scan (many random
/// probe start offsets).
#[test]
fn e23_hmput_key_duplicate() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0023);
    for trial in 0..15usize {
        reseed(0x3000 + trial);
        let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        let keys: Vec<u32> = (0..40).map(|_| rng.next_u32()).collect();
        for (i, k) in keys.iter().enumerate() {
            m.put(&k.to_le_bytes(), &(i as u32).to_le_bytes());
        }
        let before = m.snap_pair();
        let used_before = before.0.table.as_ref().unwrap().used_count;
        let len_before = before.0.length;
        for (i, k) in keys.iter().enumerate() {
            m.put(&k.to_le_bytes(), &(i as u32 + 900).to_le_bytes());
            let (sc, sr) = m.snap_pair();
            assert_eq!(sc, sr, "trial={trial} dup put #{i}");
            assert_eq!(sc.length, len_before, "length changed on duplicate");
            assert_eq!(
                sc.table.as_ref().unwrap().used_count,
                used_before,
                "used_count changed on duplicate"
            );
        }
        m.free();
    }
    reseed(DEFAULT_SEED);
}

/// row 24 — the `switch (table->string.mode)` `default:` arm memcpys the key
#[test]
fn e24_hmput_key_default_arm() {
    let _g = serial();
    // `string.mode` values with no `STBDS_SH_*` case: 0 and >= 4
    for sh in [0i32, 4, 5, 99, 255] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::with_shmode(
            16,
            8,
            8,
            STBDS_HM_STRING,
            KeyKind::RawPrefix { n: 8 },
            sh,
        );
        let (s, _) = m.snap_pair();
        assert_eq!(s.table.as_ref().unwrap().string_mode, sh as u8);
        m.put(&cstr(b"0123456789"), &1u32.to_le_bytes());
        m.assert_same(&format!("default arm sh={sh}"));
        let (s, _) = m.snap_pair();
        assert_eq!(
            s.elements[1],
            ElemSnap::Raw(b"01234567".to_vec()),
            "sh={sh}: the default arm must memcpy `keysize` bytes of the key"
        );
        m.free();
    }
}

/// row 25 — `mode` above the enum range is still "string"
#[test]
fn e25_mode_above_range_is_string() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0025);
    for mode in [2i32, 3, 100, i32::MAX] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(16, 8, 8, mode, str_kind(8, 4));
        let mut keys = Vec::new();
        for i in 0..40u32 {
            let k = cstr(format!("m{mode}_{}", rng.next_u32()).as_bytes());
            keys.push(k.clone());
            m.put(&k, &i.to_le_bytes());
            m.assert_same(&format!("mode={mode} put #{i}"));
        }
        // string comparison semantics: an equal *content* key must hit
        for k in &keys {
            let (a, b) = m.geti(k);
            assert_eq!(a, b, "mode={mode} lookup diverged");
            assert!(a >= 0, "mode={mode}: content-equal key must hit");
        }
        m.free();
    }
}

/// row 26 — negative `mode` is binary
#[test]
fn e26_mode_negative_is_binary() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0026);
    for mode in [-1i32, -2, -1000, i32::MIN] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(8, 4, 4, mode, bin_kind(4, 4, 4));
        for i in 0..40u32 {
            m.put(&rng.next_u32().to_le_bytes(), &i.to_le_bytes());
            m.assert_same(&format!("mode={mode} put #{i}"));
        }
        let (s, _) = m.snap_pair();
        assert_eq!(s.table.as_ref().unwrap().string_mode, 0);
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 27..38 — stbds_hmdel_key
// ---------------------------------------------------------------------------

/// row 27 — `hmdel_key(NULL, ...)` returns NULL
#[test]
fn e27_hmdel_null() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        let key = 1u32.to_le_bytes();
        for (elemsize, keysize, mode) in [
            (8usize, 4usize, 0i32),
            (16, 8, 1),
            (0, 0, 0),
            (32, 8, -1),
            (16, 8, i32::MAX),
        ] {
            let c = (lc.hmdel_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                keysize,
                0,
                mode,
            );
            let r = (lr.hmdel_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                keysize,
                0,
                mode,
            );
            assert!(c.is_null(), "C must return NULL");
            assert!(r.is_null(), "Rust must return NULL");
        }
    }
}

/// row 28 — `hmdel_key` with `hash_table == 0`: `temp = 0`, `a` unchanged
#[test]
fn e28_hmdel_no_table() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 4, 8, STBDS_HM_BINARY, bin_kind(4, 8, 4));
    m.put_default();
    let (pc, pr) = (m.c, m.r);
    for k in [0u32, 1, u32::MAX] {
        let (a, b) = m.del(&k.to_le_bytes());
        assert_eq!((a, b), (0, 0), "k={k}");
        assert_eq!(m.c, pc);
        assert_eq!(m.r, pr);
    }
    m.assert_same("hmdel_key with no table");
    let (s, _) = m.snap_pair();
    assert_eq!(s.temp, 0);
    assert_eq!(s.length, 1);
    m.free();
}

/// row 29 — `hmdel_key` for an absent key
#[test]
fn e29_hmdel_missing() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0029);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for i in 0..60u32 {
        m.put(&(rng.next_u32() | 1).to_le_bytes(), &i.to_le_bytes());
    }
    let before = m.snap_pair();
    for _ in 0..300 {
        let k = rng.next_u32() & !1;
        let (a, b) = m.del(&k.to_le_bytes());
        assert_eq!((a, b), (0, 0), "absent delete must yield 0 (k={k:#x})");
    }
    let after = m.snap_pair();
    assert_eq!(before.0.length, after.0.length);
    assert_eq!(before.0.table, after.0.table);
    assert_eq!(before.0.elements, after.0.elements);
    m.assert_same("absent deletes");
    m.free();
}

/// rows 30..33 — the four `hmdel_key` assertions are unreachable for
/// non-corrupt tables.  Their invariants are asserted directly instead, over a
/// long randomized delete workload.
#[test]
fn e30_e33_hmdel_invariants() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0030);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    let mut live: Vec<u32> = Vec::new();
    for i in 0..400u32 {
        let k = rng.next_u32() % 3000;
        if !live.contains(&k) {
            live.push(k);
        }
        m.put(&k.to_le_bytes(), &i.to_le_bytes());
    }
    while !live.is_empty() {
        let i = rng.below(live.len());
        let k = live.swap_remove(i);
        let (a, b) = m.del(&k.to_le_bytes());
        assert_eq!((a, b), (1, 1), "delete of a live key must yield 1");
        let (sc, sr) = m.snap_pair();
        assert_eq!(sc, sr);
        let t = sc.table.as_ref().unwrap();
        // row 30: every occupied slot index is inside the table
        for (h, ix) in &t.buckets {
            for j in 0..8 {
                if ix[j] >= 0 {
                    assert!(
                        (ix[j] as usize) < sc.length,
                        "slot index {} out of range (length {})",
                        ix[j],
                        sc.length
                    );
                    assert!(h[j] >= 2, "an in-use slot must hold a real hash");
                }
            }
        }
        // rows 31..33: counters stay consistent
        assert_eq!(t.used_count, sc.length - 1, "used_count must track length-1");
        assert!(t.used_count + t.tombstone_count <= t.slot_count);
        // and every remaining key must still be findable at a valid index
        for &lk in &live {
            let (x, y) = m.geti(&lk.to_le_bytes());
            assert_eq!(x, y);
            assert!(x >= 0, "live key {lk} vanished");
        }
    }
    m.free();
}

/// row 34 — `mode == STBDS_HM_STRING` on a `SH_STRDUP` table frees the key
#[test]
fn e34_hmdel_strdup_frees() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0034);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::with_shmode(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4), STBDS_SH_STRDUP);
    let mut keys = Vec::new();
    for i in 0..80u32 {
        let n = 1 + rng.below(30);
        let k = cstr(&rng.ascii(n));
        keys.push(k.clone());
        m.put(&k, &i.to_le_bytes());
    }
    m.assert_same("populate strdup");
    while !keys.is_empty() {
        let i = rng.below(keys.len());
        let k = keys.swap_remove(i);
        let (a, b) = m.del_mode(&k, STBDS_HM_STRING);
        assert_eq!((a, b), (1, 1));
        m.assert_same("strdup delete (mode == 1)");
    }
    m.free();
}

/// row 35 — `mode != STBDS_HM_STRING` on a string table: no free, and the
/// re-lookup of the swapped-in element uses the raw bytes.  Deleting from the
/// tail keeps that path well defined while still selecting the no-free branch.
#[test]
fn e35_hmdel_mode2_no_free() {
    let _g = serial();
    for mode in [2i32, 3, 77, i32::MAX] {
        reseed(DEFAULT_SEED);
        let mut m =
            MapPair::with_shmode(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4), STBDS_SH_STRDUP);
        let keys: Vec<Vec<u8>> = (0..8)
            .map(|i| cstr(format!("nofree_{mode}_{i}").as_bytes()))
            .collect();
        for (i, k) in keys.iter().enumerate() {
            m.put(k, &(i as u32).to_le_bytes());
        }
        m.assert_same(&format!("populate (mode={mode})"));
        for i in (0..keys.len()).rev() {
            let (a, b) = m.del_mode(&keys[i], mode);
            assert_eq!((a, b), (1, 1), "mode={mode} delete #{i}");
            m.assert_same(&format!("mode={mode} delete #{i}"));
        }
        assert_eq!(m.len(), (0, 0));
        m.free();
    }
}

/// row 36 — shrink branch
#[test]
fn e36_hmdel_shrink() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for k in 0..400u32 {
        m.put(&k.to_le_bytes(), &k.to_le_bytes());
    }
    let big = m.snap_pair().0.table.as_ref().unwrap().slot_count;
    let mut shrinks = 0;
    let mut prev = big;
    for k in 0..400u32 {
        m.del(&k.to_le_bytes());
        m.assert_same(&format!("shrink del {k}"));
        let now = m.snap_pair().0.table.as_ref().unwrap().slot_count;
        if now < prev {
            shrinks += 1;
            prev = now;
        }
    }
    assert!(shrinks >= 4, "expected repeated shrinks, saw {shrinks}");
    assert_eq!(prev, 8, "must bottom out at STBDS_BUCKET_LENGTH");
    m.free();
}

/// row 37 — tombstone rebuild branch (same `slot_count`)
#[test]
fn e37_hmdel_tombstone_rebuild() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0037);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    let mut live: Vec<u32> = Vec::new();
    for i in 0..90u32 {
        let k = rng.next_u32() | 1;
        live.push(k);
        m.put(&k.to_le_bytes(), &i.to_le_bytes());
    }
    let mut rebuilds = 0;
    for round in 0..500u32 {
        let before = m.snap_pair().0.table.as_ref().unwrap().clone();
        let i = rng.below(live.len());
        let victim = live.swap_remove(i);
        m.del(&victim.to_le_bytes());
        m.assert_same(&format!("round={round} del"));
        let after = m.snap_pair().0.table.as_ref().unwrap().clone();
        if after.slot_count == before.slot_count && after.tombstone_count == 0 && before.tombstone_count > 0
        {
            rebuilds += 1;
        }
        let k = rng.next_u32() | 1;
        live.push(k);
        m.put(&k.to_le_bytes(), &round.to_le_bytes());
        m.assert_same(&format!("round={round} put"));
    }
    assert!(rebuilds >= 1, "tombstone rebuild never triggered");
    m.free();
}

/// row 38 — deleting the only element (`old_index == final_index`)
#[test]
fn e38_hmdel_only_element() {
    let _g = serial();
    for mode_kind in 0..2 {
        reseed(DEFAULT_SEED);
        if mode_kind == 0 {
            let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
            m.put(&5u32.to_le_bytes(), &6u32.to_le_bytes());
            let (a, b) = m.del(&5u32.to_le_bytes());
            assert_eq!((a, b), (1, 1));
            m.assert_same("binary delete only element");
            assert_eq!(m.len(), (0, 0));
            m.free();
        } else {
            let mut m = MapPair::new(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4));
            m.put(&cstr(b"solo"), &6u32.to_le_bytes());
            let (a, b) = m.del(&cstr(b"solo"));
            assert_eq!((a, b), (1, 1));
            m.assert_same("string delete only element");
            assert_eq!(m.len(), (0, 0));
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 39..44 — the string arena
// ---------------------------------------------------------------------------

/// row 39 — the `len <= a->remaining` assertion never fires for reachable
/// inputs; the invariant is checked around every block boundary instead.
#[test]
fn e39_stralloc_boundary_lengths() {
    let _g = serial();
    let (lc, lr) = libs();
    // walk lengths right around each block size so `remaining` hits 0 exactly
    for start in [1usize, 500, 505, 510, 511, 512, 513] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        unsafe {
            for extra in 0..8usize {
                let n = start + extra;
                let mut s = cstr(&vec![b'q'; n]);
                let pc = (lc.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let pr = (lr.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
                assert_eq!(CStr::from_ptr(pc).to_bytes().len(), n);
                assert_eq!(CStr::from_ptr(pr).to_bytes().len(), n);
                assert_eq!(arena_snap(&ca), arena_snap(&ra), "n={n}");
            }
            (lc.strreset)(&mut ca);
            (lr.strreset)(&mut ra);
        }
    }
}

/// row 40 — `len > blocksize` on an empty arena
#[test]
fn e40_stralloc_oversize_empty() {
    let _g = serial();
    let (lc, lr) = libs();
    for n in [512usize, 513, 1024, 65536] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        unsafe {
            let mut s = cstr(&vec![b'w'; n]);
            let pc = (lc.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
            let pr = (lr.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(pc).to_bytes().len(), n);
            assert_eq!(CStr::from_ptr(pr).to_bytes().len(), n);
            let sc = arena_snap(&ca);
            assert_eq!(sc, arena_snap(&ra), "n={n}");
            assert_eq!(sc.remaining, 0, "n={n}");
            assert_eq!(sc.block, 1);
            assert_eq!(sc.block_count, 1);
            (lc.strreset)(&mut ca);
            (lr.strreset)(&mut ra);
        }
    }
}

/// row 41 — `len > blocksize` on a non-empty arena: spliced in as
/// `storage->next`, `remaining` deliberately untouched
#[test]
fn e41_stralloc_oversize_nonempty() {
    let _g = serial();
    let (lc, lr) = libs();
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    unsafe {
        let mut s = cstr(b"seed");
        (lc.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
        (lr.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
        let rem = arena_snap(&ca).remaining;
        for n in [2000usize, 4000, 9000] {
            let mut big = cstr(&vec![b'B'; n]);
            let pc = (lc.stralloc)(&mut ca, big.as_mut_ptr() as *mut c_char);
            let pr = (lr.stralloc)(&mut ra, big.as_mut_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(pc).to_bytes().len(), n);
            assert_eq!(CStr::from_ptr(pr).to_bytes().len(), n);
            let sc = arena_snap(&ca);
            assert_eq!(sc, arena_snap(&ra), "n={n}");
            assert_eq!(sc.remaining, rem, "remaining must not change (n={n})");
        }
        (lc.strreset)(&mut ca);
        (lr.strreset)(&mut ra);
    }
}

/// row 42 — `a->block` saturates once `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX`
#[test]
fn e42_stralloc_block_saturation() {
    let _g = serial();
    let (lc, lr) = libs();
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    unsafe {
        let mut big = cstr(&vec![b'S'; 2_000_000]);
        for i in 0..40 {
            (lc.stralloc)(&mut ca, big.as_mut_ptr() as *mut c_char);
            (lr.stralloc)(&mut ra, big.as_mut_ptr() as *mut c_char);
            assert_eq!(arena_snap(&ca), arena_snap(&ra), "iteration {i}");
        }
        let s = arena_snap(&ca);
        assert_eq!(
            s.block, 22,
            "block must saturate where 512 << (block>>1) reaches 1<<20"
        );
        (lc.strreset)(&mut ca);
        (lr.strreset)(&mut ra);
        assert_eq!(arena_snap(&ca), arena_snap(&ra));
    }
}

/// row 43 — empty string consumes one byte
#[test]
fn e43_stralloc_empty_string() {
    let _g = serial();
    let (lc, lr) = libs();
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    unsafe {
        for i in 0..600 {
            let mut s = cstr(b"");
            let pc = (lc.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
            let pr = (lr.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(pc).to_bytes(), b"");
            assert_eq!(CStr::from_ptr(pr).to_bytes(), b"");
            assert_eq!(arena_snap(&ca), arena_snap(&ra), "iteration {i}");
        }
        assert_eq!(arena_snap(&ca).remaining, 512 - 600 % 512 - if 600 % 512 == 0 { 0 } else { 0 });
        (lc.strreset)(&mut ca);
        (lr.strreset)(&mut ra);
    }
}

/// row 44 — `strreset` on an arena with `storage == NULL`
#[test]
fn e44_strreset_null_storage() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        for (block, mode, remaining) in [(0u8, 0u8, 0usize), (7, 3, 999), (255, 255, usize::MAX)] {
            let mut ca = StringArena {
                storage: std::ptr::null_mut(),
                remaining,
                block,
                mode,
            };
            let mut ra = ca;
            (lc.strreset)(&mut ca);
            (lr.strreset)(&mut ra);
            assert_eq!(arena_snap(&ca), arena_snap(&ra));
            assert_eq!(
                arena_snap(&ca),
                ArenaSnap {
                    remaining: 0,
                    block: 0,
                    mode: 0,
                    storage_null: true,
                    block_count: 0
                }
            );
        }
    }
}

// ---------------------------------------------------------------------------
// rows 45..48 — hash functions and shmode_func edge inputs
// ---------------------------------------------------------------------------

/// row 45 — `hash_string("")`
#[test]
fn e45_hash_string_empty() {
    let _g = serial();
    let (lc, lr) = libs();
    let mut s = cstr(b"");
    unsafe {
        for seed in [0usize, 1, 2, DEFAULT_SEED, usize::MAX, usize::MAX / 3] {
            assert_eq!(
                (lc.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
                (lr.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
                "seed={seed:#x}"
            );
        }
    }
}

/// row 46 — `hash_bytes(p, 0, seed)` never dereferences `p`
#[test]
fn e46_hash_bytes_len_zero() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        for seed in [0usize, 1, DEFAULT_SEED, usize::MAX] {
            let a = (lc.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let b = (lr.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(a, b, "seed={seed:#x}");
            // dangling-but-unused pointer, same answer
            let c = (lc.hash_bytes)(0xdead_beefusize as *mut c_void, 0, seed);
            let d = (lr.hash_bytes)(0xdead_beefusize as *mut c_void, 0, seed);
            assert_eq!(c, d);
            assert_eq!(a, c, "len 0 must not depend on the pointer");
        }
    }
}

/// row 47 — `is_key_equal` in string mode compares through the *stored* pointer:
/// a content-equal but address-different key must hit, an address-equal but
/// content-different key must not exist.
#[test]
fn e47_is_key_equal_string() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4));
    for i in 0..20u32 {
        m.put(&cstr(format!("dup_{i}").as_bytes()), &i.to_le_bytes());
    }
    m.assert_same("populate");
    for i in 0..20u32 {
        // freshly allocated buffer with the same contents
        let probe = cstr(format!("dup_{i}").as_bytes());
        let (a, b) = m.geti(&probe);
        assert_eq!(a, b);
        assert!(a >= 0, "content-equal key must hit (i={i})");
    }
    for i in 0..20u32 {
        let probe = cstr(format!("dup_{i}x").as_bytes());
        let (a, b) = m.geti(&probe);
        assert_eq!((a, b), (-1, -1), "prefix key must miss (i={i})");
    }
    m.free();
}

/// row 48 — `shmode_func` truncates `mode` to `unsigned char`
#[test]
fn e48_shmode_func_mode_truncation() {
    let _g = serial();
    let (lc, lr) = libs();
    let cases: [(c_int, u8); 10] = [
        (0, 0),
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 4),
        (255, 255),
        (256, 0),
        (257, 1),
        (-1, 255),
        (i32::MAX, 255),
    ];
    for (mode, want) in cases {
        for elemsize in [8usize, 16, 32] {
            reseed(DEFAULT_SEED);
            unsafe {
                let c = (lc.shmode_func)(elemsize, mode);
                let r = (lr.shmode_func)(elemsize, mode);
                let kind = KeyKind::RawPrefix { n: elemsize };
                assert_eq!(
                    map_snap(c, elemsize, kind),
                    map_snap(r, elemsize, kind),
                    "shmode_func({elemsize}, {mode})"
                );
                let s = map_snap(c, elemsize, kind);
                assert_eq!(
                    s.table.as_ref().unwrap().string_mode,
                    want,
                    "shmode_func({elemsize}, {mode}) truncation"
                );
                (lc.hmfree_func)((c as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (lr.hmfree_func)((r as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
    reseed(DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// rows 49..52 — the driver entry points
// ---------------------------------------------------------------------------

/// rows 49..51 — `str_put`'s three assertions must not fire for any `num`, and
/// `num <= 0` must skip the arena loop.  Both libraries must exit normally with
/// identical stdout.
#[test]
fn e49_e51_str_put_asserts_never_fire() {
    let _g = serial();
    let (lc, lr) = libs();
    for num in [i32::MIN, -1000, -1, 0, 1, 2, 300] {
        let fc = lc.str_put;
        let fr = lr.str_put;
        let (oc, ec) = child_run("c_strput", move || unsafe { fc(num) });
        let (or_, er) = child_run("r_strput", move || unsafe { fr(num) });
        assert_eq!(
            oc,
            or_,
            "str_put({num}) outcome diverged\nC: {}\nR: {}",
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er)
        );
        assert_eq!(
            oc,
            ChildOutcome {
                signal: 0,
                exit_code: 0
            },
            "str_put({num}) must not abort (stderr: {})",
            String::from_utf8_lossy(&ec)
        );
        assert!(ec.is_empty(), "C wrote to stderr: {:?}", String::from_utf8_lossy(&ec));
        assert!(er.is_empty(), "Rust wrote to stderr: {:?}", String::from_utf8_lossy(&er));
    }
}

/// row 52 — `strkey` with the longest decimal forms
#[test]
fn e52_strkey_extremes() {
    let _g = serial();
    let (lc, lr) = libs();
    unsafe {
        for n in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            let c = CStr::from_ptr((lc.strkey)(n)).to_bytes().to_vec();
            let r = CStr::from_ptr((lr.strkey)(n)).to_bytes().to_vec();
            assert_eq!(c, r, "strkey({n})");
            assert_eq!(c, format!("test_{n}").into_bytes());
            assert!(c.len() < 256, "must fit the 256-byte static buffer");
        }
    }
}

// ---------------------------------------------------------------------------
// G1..G7 — generic FFI-boundary cases
// ---------------------------------------------------------------------------

/// G1 — NULL `a` into every pointer-taking entry point that guards it
#[test]
fn g1_null_into_every_guarded_entry_point() {
    let _g = serial();
    let (lc, lr) = libs();
    let key = 99u32.to_le_bytes();
    let kp = key.as_ptr() as *mut c_void;
    unsafe {
        for elemsize in [8usize, 16] {
            for mode in [-1i32, 0, 1, 2, i32::MAX] {
                reseed(DEFAULT_SEED);
                // hmdel_key
                assert!((lc.hmdel_key)(std::ptr::null_mut(), elemsize, kp, 4, 0, mode).is_null());
                assert!((lr.hmdel_key)(std::ptr::null_mut(), elemsize, kp, 4, 0, mode).is_null());
                // hmfree_func
                (lc.hmfree_func)(std::ptr::null_mut(), elemsize);
                (lr.hmfree_func)(std::ptr::null_mut(), elemsize);
                // hmput_default
                reseed(DEFAULT_SEED);
                let cd = (lc.hmput_default)(std::ptr::null_mut(), elemsize);
                let rd = (lr.hmput_default)(std::ptr::null_mut(), elemsize);
                let kind = KeyKind::RawPrefix { n: elemsize };
                assert_eq!(map_snap(cd, elemsize, kind), map_snap(rd, elemsize, kind));
                (lc.hmfree_func)((cd as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (lr.hmfree_func)((rd as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                // hmget_key / hmget_key_ts on NULL
                reseed(DEFAULT_SEED);
                let cg = (lc.hmget_key)(std::ptr::null_mut(), elemsize, kp, 4, mode);
                let rg = (lr.hmget_key)(std::ptr::null_mut(), elemsize, kp, 4, mode);
                assert_eq!(map_snap(cg, elemsize, kind), map_snap(rg, elemsize, kind));
                assert_eq!(map_snap(cg, elemsize, kind).temp, -1);
                (lc.hmfree_func)((cg as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (lr.hmfree_func)((rg as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                // hmput_key on NULL (string modes need a NUL-terminated key)
                reseed(DEFAULT_SEED);
                let skey = cstr(b"nullboot");
                let skp = skey.as_ptr() as *mut c_void;
                let (kp2, ks) = if mode >= 1 { (skp, 8usize) } else { (kp, 4usize) };
                let cp = (lc.hmput_key)(std::ptr::null_mut(), elemsize, kp2, ks, mode);
                let rp = (lr.hmput_key)(std::ptr::null_mut(), elemsize, kp2, ks, mode);
                // only the `keysize` bytes `hmput_key` actually writes are
                // defined; the rest of the element is `realloc` garbage in C.
                let kkind = KeyKind::RawPrefix { n: ks };
                assert_eq!(
                    map_snap(cp, elemsize, kkind),
                    map_snap(rp, elemsize, kkind),
                    "hmput_key(NULL) elemsize={elemsize} mode={mode}"
                );
                (lc.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (lr.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
        // arrgrowf(NULL, ...) — already covered; assert the whole grid again
        for elemsize in [0usize, 1, 8] {
            for addlen in [0usize, 1, 4] {
                for min_cap in [0usize, 1, 4, 17] {
                    let c = (lc.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let r = (lr.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    assert_eq!(arr_snap(c, 0), arr_snap(r, 0));
                    if !c.is_null() {
                        (lc.arrfreef)(c);
                        (lr.arrfreef)(r);
                    }
                }
            }
        }
    }
    reseed(DEFAULT_SEED);
}

/// G2 — `keysize == 0`: `memcmp(_,_,0) == 0`, so every key compares equal and
/// the map degenerates to a single entry.
#[test]
fn g2_keysize_zero() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0002);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 0, 4, STBDS_HM_BINARY, bin_kind(0, 4, 4));
    for i in 0..50u32 {
        let k = rng.next_u32().to_le_bytes();
        m.put(&k, &i.to_le_bytes());
        m.assert_same(&format!("keysize=0 put #{i}"));
    }
    assert_eq!(m.len(), (1, 1), "all keys must collapse into one entry");
    let (a, b) = m.geti(&0u32.to_le_bytes());
    assert_eq!(a, b);
    assert_eq!(a, 0, "any key must hit the single entry");
    let (a, b) = m.del(&0u32.to_le_bytes());
    assert_eq!((a, b), (1, 1));
    m.assert_same("keysize=0 delete");
    m.free();
}

/// G3 — `keysize` covering the whole element (larger than a "key field")
#[test]
fn g3_keysize_covers_whole_element() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0003);
    for elemsize in [8usize, 16, 24] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(
            elemsize,
            elemsize,
            0,
            STBDS_HM_BINARY,
            bin_kind(elemsize, 0, elemsize),
        );
        let mut keys = Vec::new();
        for _ in 0..60 {
            let k = rng.bytes(elemsize);
            keys.push(k.clone());
            m.put(&k, &k);
            m.assert_same(&format!("elemsize=keysize={elemsize} put"));
        }
        for k in &keys {
            let (a, b) = m.geti(k);
            assert_eq!(a, b);
            assert!(a >= 0);
        }
        m.free();
    }
}

/// G4 — out-of-range enum `mode` across the FFI for every mode-taking function
#[test]
fn g4_out_of_range_mode() {
    let _g = serial();
    let mut rng = Rng::new(0xC0_0004);
    for mode in [i32::MIN, -1000, -1, 0, 1, 2, 3, 4, 1000, i32::MAX] {
        reseed(DEFAULT_SEED);
        if mode >= 1 {
            // string semantics
            let mut m = MapPair::new(16, 8, 8, mode, str_kind(8, 4));
            let mut keys = Vec::new();
            for i in 0..30u32 {
                let k = cstr(format!("g4_{mode}_{}", rng.next_u32()).as_bytes());
                keys.push(k.clone());
                m.put(&k, &i.to_le_bytes());
                m.assert_same(&format!("mode={mode} put #{i}"));
            }
            for k in &keys {
                let (a, b) = m.geti(k);
                assert_eq!(a, b, "mode={mode} geti");
                let (a2, b2) = m.geti_ts(k);
                assert_eq!(a2, b2, "mode={mode} geti_ts");
            }
            // deletes from the tail keep the `mode != 1` re-lookup branch safe
            for i in (0..keys.len()).rev() {
                let (a, b) = m.del_mode(&keys[i], mode);
                assert_eq!(a, b, "mode={mode} del");
                m.assert_same(&format!("mode={mode} del #{i}"));
            }
            m.free();
        } else {
            let mut m = MapPair::new(8, 4, 4, mode, bin_kind(4, 4, 4));
            let mut keys = Vec::new();
            for i in 0..40u32 {
                let k = rng.next_u32();
                keys.push(k);
                m.put(&k.to_le_bytes(), &i.to_le_bytes());
                m.assert_same(&format!("mode={mode} put #{i}"));
            }
            for &k in &keys {
                let (a, b) = m.geti(&k.to_le_bytes());
                assert_eq!(a, b, "mode={mode} geti");
                let (a2, b2) = m.geti_ts(&k.to_le_bytes());
                assert_eq!(a2, b2, "mode={mode} geti_ts");
            }
            for &k in &keys {
                let (a, b) = m.del_mode(&k.to_le_bytes(), mode);
                assert_eq!(a, b, "mode={mode} del");
                m.assert_same(&format!("mode={mode} del"));
            }
            m.free();
        }
    }
    reseed(DEFAULT_SEED);
}

/// G5 — out-of-range enum into `shmode_func`, then a put that lands in the
/// `default:` arm because the truncated mode is not one of 1/2/3
#[test]
fn g5_shmode_out_of_range_then_put() {
    let _g = serial();
    for (mode, effective) in [(4i32, 4u8), (255, 255), (256, 0), (-1, 255), (i32::MAX, 255)] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::with_shmode(
            24,
            8,
            8,
            STBDS_HM_STRING,
            KeyKind::RawPrefix { n: 8 },
            mode,
        );
        let (s, _) = m.snap_pair();
        assert_eq!(s.table.as_ref().unwrap().string_mode, effective);
        m.put(&cstr(b"abcdefghij"), &1u32.to_le_bytes());
        m.assert_same(&format!("shmode={mode} put"));
        let (s, _) = m.snap_pair();
        assert_eq!(s.elements[1], ElemSnap::Raw(b"abcdefgh".to_vec()));
        m.free();
    }
    reseed(DEFAULT_SEED);
}

/// G6 — `stbds_arrgrowf` with a request the allocator must reject: `realloc`
/// returns NULL and the C then writes through `NULL + sizeof(header)`.  Both
/// libraries must die the same way.
#[test]
fn g6_arrgrowf_allocation_failure() {
    let _g = serial();
    let (lc, lr) = libs();
    let fc = lc.arrgrowf;
    let fr = lr.arrgrowf;
    for (elemsize, min_cap) in [(1usize, usize::MAX - 1000), (8usize, usize::MAX / 8)] {
        let (oc, _) = child_run("c_oom", move || unsafe {
            fc(std::ptr::null_mut(), elemsize, 0, min_cap);
        });
        let (or_, _) = child_run("r_oom", move || unsafe {
            fr(std::ptr::null_mut(), elemsize, 0, min_cap);
        });
        assert_eq!(
            oc, or_,
            "allocation-failure outcome diverged (elemsize={elemsize}, min_cap={min_cap})"
        );
    }
}

/// G7 — `rand_seed` extremes drive identical table seeds
#[test]
fn g7_rand_seed_extremes() {
    let _g = serial();
    let (lc, lr) = libs();
    for seed in [0usize, 1, 2, usize::MAX, usize::MAX - 1, DEFAULT_SEED, 1 << 63] {
        reseed(seed);
        unsafe {
            for i in 0..8 {
                let c = (lc.shmode_func)(16, STBDS_SH_DEFAULT);
                let r = (lr.shmode_func)(16, STBDS_SH_DEFAULT);
                let tc = (*header_of(c, 16)).hash_table as *mut HashIndex;
                let tr = (*header_of(r, 16)).hash_table as *mut HashIndex;
                assert_eq!(
                    table_snap(tc),
                    table_snap(tr),
                    "seed={seed:#x} iteration {i}"
                );
                (lc.hmfree_func)((c as *mut u8).sub(16) as *mut c_void, 16);
                (lr.hmfree_func)((r as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
    reseed(DEFAULT_SEED);
}
