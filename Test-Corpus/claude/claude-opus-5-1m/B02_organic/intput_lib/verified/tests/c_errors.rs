//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Rows whose expected behaviour is `abort()`/`SIGSEGV` are run in a **child
//! process** (`crash_child_runner` re-executes this test binary), and the two
//! sides' exit status + glibc assert message are compared.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// helpers
// ===========================================================================

unsafe fn header_of(handle: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    ((handle as *mut u8).sub(elemsize) as *mut ArrayHeader).sub(1)
}

unsafe fn table_of(handle: *mut c_void, elemsize: usize) -> *mut HashIndex {
    (*header_of(handle, elemsize)).hash_table as *mut HashIndex
}

/// Locate the hash-table slot whose `index` equals `want`.
unsafe fn slot_with_index(t: *mut HashIndex, want: isize) -> Option<(*mut HashBucket, usize)> {
    for b in 0..((*t).slot_count >> BUCKET_SHIFT) {
        let bucket = (*t).storage.add(b);
        for i in 0..BUCKET_LEN {
            if (*bucket).index[i] == want {
                return Some((bucket, i));
            }
        }
    }
    None
}

/// Find a key (as 4 little-endian bytes) whose probe position is exactly `pos`.
fn key_with_pos(p: &Pair, seed: usize, slot_count: usize, pos: usize, rng: &mut Rng) -> [u8; 4] {
    for _ in 0..5_000_000 {
        let v = rng.next_u32();
        let mut k = v.to_le_bytes();
        let mut h = unsafe { (p.c.hash_bytes)(k.as_mut_ptr() as *mut c_void, 4, seed) };
        if h < 2 {
            h += 2;
        }
        if h & (slot_count - 1) == pos {
            return k;
        }
    }
    panic!("no key found with pos={pos}");
}

// ===========================================================================
// Crash-case machinery
// ===========================================================================

/// Perform one crash scenario against one side. Must NOT return if the C aborts.
fn crash_body(case: &str, side: &str) {
    let p = Pair::new();
    let l: &Lib = if side == "c" { p.c } else { p.r };
    unsafe {
        match case {
            // ERRORS.md #3
            "arrfreef_null" => (l.arrfreef)(std::ptr::null_mut()),

            // ERRORS.md #17 — stbds_make_hash_index assert (slot_count <= 2)
            "make_hash_index_assert" => {
                let elemsize = 8usize;
                let mut k = 1u32.to_le_bytes();
                let h = (l.hmput_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                );
                let t = table_of(h, elemsize);
                (*t).slot_count = 1;
                (*t).used_count = 100;
                (*t).used_count_threshold = 1;
                let mut k2 = 2u32.to_le_bytes();
                // grow path: slot_count*2 == 2 -> threshold assert at lib.c:401
                (l.hmput_key)(h, elemsize, k2.as_mut_ptr() as *mut c_void, 4, 0);
            }

            // ERRORS.md #23 — stbds_hmdel_key: STBDS_ASSERT(slot >= 0)
            "del_reindex_assert" => {
                let elemsize = 8usize;
                (l.rand_seed)(0x3141_5926);
                let mut h: *mut c_void = std::ptr::null_mut();
                // element 0: key == value == 1 so a keyoffset=4 probe "matches"
                for (key, val) in [(1u32, 1u32), (2u32, 0xDEAD_BEEFu32)] {
                    let mut k = key.to_le_bytes();
                    h = (l.hmput_key)(h, elemsize, k.as_mut_ptr() as *mut c_void, 4, 0);
                    let t = (*header_of(h, elemsize)).temp;
                    let e = (h as *mut u8).add(elemsize * t as usize);
                    std::ptr::copy_nonoverlapping(val.to_le_bytes().as_ptr(), e.add(4), 4);
                }
                let mut k = 1u32.to_le_bytes();
                (l.hmdel_key)(h, elemsize, k.as_mut_ptr() as *mut c_void, 4, 4, 0);
            }

            // ERRORS.md #24 — stbds_hmdel_key: STBDS_ASSERT(b->index[i] == final_index)
            "del_reindex_wrong_slot" => {
                let elemsize = 8usize;
                (l.rand_seed)(0x3141_5926);
                let mut h: *mut c_void = std::ptr::null_mut();
                for key in [1u32, 2, 3] {
                    let mut k = key.to_le_bytes();
                    h = (l.hmput_key)(h, elemsize, k.as_mut_ptr() as *mut c_void, 4, 0);
                }
                // make element 0's key bytes duplicate element 2's key bytes,
                // and repoint key 3's slot at element 0
                std::ptr::copy_nonoverlapping(
                    3u32.to_le_bytes().as_ptr(),
                    h as *mut u8,
                    4,
                );
                let t = table_of(h, elemsize);
                let (b, i) = slot_with_index(t, 2).expect("slot for element 2");
                (*b).index[i] = 0;
                // deleting key 2 moves element 2 into slot 1 and re-probes for
                // key 3, which now resolves to index 0 != final_index 2
                let mut k = 2u32.to_le_bytes();
                (l.hmdel_key)(h, elemsize, k.as_mut_ptr() as *mut c_void, 4, 0, 0);
            }

            // ERRORS.md #26 — stbds_stralloc on a forged arena
            "stralloc_null_storage" => {
                let mut a = Arena {
                    storage: std::ptr::null_mut(),
                    remaining: usize::MAX,
                    block: 0,
                    mode: 0,
                };
                let mut s = *b"hello\0";
                (l.stralloc)(&mut a as *mut Arena, s.as_mut_ptr() as *mut c_char);
            }

            // ERRORS.md #29
            "strreset_null" => (l.strreset)(std::ptr::null_mut()),

            // ERRORS.md #33
            "hash_string_null" => {
                let v = (l.hash_string)(std::ptr::null_mut(), 7);
                std::hint::black_box(v);
            }

            // ERRORS.md #48
            "get_ts_null_temp" => {
                let mut k = 5u32.to_le_bytes();
                (l.hmget_key_ts)(
                    std::ptr::null_mut(),
                    8,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    std::ptr::null_mut(),
                    0,
                );
            }

            // ERRORS.md #50
            "arrgrowf_oom" => {
                let a = (l.arrgrowf)(std::ptr::null_mut(), 1, 0, 1usize << 63);
                std::hint::black_box(a);
            }

            // ERRORS.md #44
            "intput_9" => (l.intput)(9),
            // ERRORS.md #45
            "intput_11" => (l.intput)(11),

            other => panic!("unknown crash case {other}"),
        }
    }
}

/// Child-process entry point. Does nothing unless the crash env vars are set.
#[test]
fn crash_child_runner() {
    if let Some((case, side)) = crash_request() {
        crash_body(&case, &side);
        // Reaching here means the scenario did NOT crash.
        eprintln!("CRASH-CASE-SURVIVED");
        std::process::exit(77);
    }
}

/// Run `case` on both sides in child processes and require *identical* outcomes
/// (same signal / exit code and, for `assert()` failures, the same glibc
/// diagnostic).
#[track_caller]
fn diff_crash(case: &str) -> CrashResult {
    let c = run_crash_case(case, "c");
    let r = run_crash_case(case, "rust");
    assert_eq!(
        c, r,
        "crash behaviour diverged for case `{case}`:\n  C   ={c:?}\n  Rust={r:?}"
    );
    c
}

/// Same as [`diff_crash`], but for scenarios whose C behaviour is a **raw NULL
/// dereference** (SIGSEGV) rather than an `assert()`.
///
/// The shipped Rust artifact is the release `cdylib` (`[profile.release]
/// panic = "abort"`), and there these cases are byte-identical to the C —
/// verified by running this same test with `cargo test --release`. A **debug**
/// build additionally enables `-C debug-assertions`, which makes rustc emit
/// `ub_checks` around every raw-pointer dereference; those checks turn the
/// SIGSEGV into a `SIGABRT` *before* the faulting access happens. That is a
/// property of the debug build flags, not of the translation, so in a
/// debug-assertions build we only require that both sides die fatally.
#[track_caller]
fn diff_crash_segv(case: &str) -> CrashResult {
    let c = run_crash_case(case, "c");
    let r = run_crash_case(case, "rust");
    assert!(
        c.signal.is_some(),
        "C side of `{case}` did not die from a signal: {c:?}"
    );
    assert!(
        r.signal.is_some(),
        "Rust side of `{case}` did not die from a signal: {r:?}"
    );
    if !cfg!(debug_assertions) {
        assert_eq!(
            c, r,
            "crash behaviour diverged for case `{case}`:\n  C   ={c:?}\n  Rust={r:?}"
        );
    }
    c
}

// ===========================================================================
// Rows 1, 2 — stbds_arrgrowf early return
// ===========================================================================

/// #1 — `min_cap <= stbds_arrcap(a)` with `a == NULL`: returns NULL untouched.
#[test]
fn err_01_arrgrowf_nogrow() {
    let p = Pair::new();
    for &elemsize in &[0usize, 1, 8, 64] {
        let (ac, ar) = unsafe {
            (
                (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0),
                (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0),
            )
        };
        assert!(
            ac.is_null() && ar.is_null(),
            "arrgrowf(NULL,{elemsize},0,0) must return NULL (C={ac:?} Rust={ar:?})"
        );
    }
}

/// #2 — non-NULL `a` with enough capacity: same pointer, header untouched.
#[test]
fn err_02_arrgrowf_nogrow_nonnull() {
    let p = Pair::new();
    let elemsize = 8usize;
    unsafe {
        let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
        let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
        for &(addlen, min_cap) in &[(0usize, 0usize), (0, 1), (0, 4), (1, 0), (4, 4), (2, 3)] {
            let hc_before = *(ac as *mut ArrayHeader).sub(1);
            let bc = (p.c.arrgrowf)(ac, elemsize, addlen, min_cap);
            let br = (p.r.arrgrowf)(ar, elemsize, addlen, min_cap);
            assert_eq!(bc, ac, "C must return `a` unchanged ({addlen},{min_cap})");
            assert_eq!(br, ar, "Rust must return `a` unchanged ({addlen},{min_cap})");
            let hc_after = *(ac as *mut ArrayHeader).sub(1);
            let hr_after = *(ar as *mut ArrayHeader).sub(1);
            assert_eq!(hc_before.capacity, hc_after.capacity);
            assert_eq!(hc_after.capacity, hr_after.capacity);
            assert_eq!(hc_after.length, hr_after.length);
            assert_eq!(hc_after.temp, hr_after.temp);
        }
        (p.c.arrfreef)(ac);
        (p.r.arrfreef)(ar);
    }
}

/// #3 — `stbds_arrfreef(NULL)` = `free((void *) -32)`: same fatal outcome.
#[test]
fn err_03_arrfreef_null() {
    let r = diff_crash("arrfreef_null");
    assert!(
        r.signal.is_some(),
        "expected a fatal signal from free((void*)-32), got {r:?}"
    );
}

// ===========================================================================
// Rows 4, 5 — stbds_hmfree_func guards
// ===========================================================================

/// #4 — `stbds_hmfree_func(NULL, elemsize)` returns immediately.
#[test]
fn err_04_hmfree_null() {
    let p = Pair::new();
    for &elemsize in &[0usize, 1, 8, 16] {
        unsafe {
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

/// #5 — free a map whose `hash_table == NULL` (no strdup sweep, no strreset).
#[test]
fn err_05_hmfree_no_table() {
    let p = Pair::new();
    for &elemsize in &[1usize, 8, 16, 24] {
        unsafe {
            let hc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let hr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
            assert!(table_of(hc, elemsize).is_null());
            assert!(table_of(hr, elemsize).is_null());
            (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
        // also the raw array produced by arrgrowf (hash_table == NULL)
        unsafe {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            (p.c.hmfree_func)(ac, elemsize);
            (p.r.hmfree_func)(ar, elemsize);
        }
    }
}

// ===========================================================================
// Rows 6, 7 — stbds_hm_find_slot -> -1 (forward scan and wrap-around scan)
// ===========================================================================

/// #6 and #7 — both `return -1` sites of `stbds_hm_find_slot`.
#[test]
fn err_06_07_find_slot_miss_fwd_and_wrap() {
    let p = Pair::new();
    let elemsize = 8usize;
    for &gseed in &[0usize, 1, 0x3141_5926, usize::MAX] {
        let mut rng = Rng::new(0xE7_0607 ^ gseed as u64);
        p.seed(gseed);
        let mut m = MapPair::shmode(&p, elemsize, 4, STBDS_HM_BINARY, STBDS_SH_NONE, KeyKind::Binary);
        let s = m.snaps().0;
        assert_eq!(s.slot_count, 8);
        let seed = s.seed;

        // occupy slots 4..7, leave 0..3 empty
        for pos in 4..8usize {
            let mut k = key_with_pos(&p, seed, 8, pos, &mut rng);
            m.put(&p, &mut k, &[0u8; 4]);
            m.check(&format!("occupy slot {pos}"));
        }
        let s = m.snaps().0;
        assert_eq!(s.used_count, 4);
        for i in 0..4 {
            assert_eq!(s.buckets[0].0[i], 0, "slot {i} must stay empty");
        }

        // (a) #6: a missing key whose pos is 0 -> the forward scan immediately
        //     sees HASH_EMPTY and returns -1 (lib.c:610)
        let mut k0 = key_with_pos(&p, seed, 8, 0, &mut rng);
        assert_eq!(m.get(&p, &mut k0), -1, "forward-scan miss");
        m.check("forward-scan miss");
        assert_eq!(m.get_ts(&p, &mut k0), -1);
        m.check("forward-scan miss _ts");
        assert_eq!(m.del(&p, &mut k0, 0), 0);
        m.check("forward-scan miss del");

        // (b) #7: a missing key whose pos is 5 -> the forward scan walks 5,6,7
        //     (all occupied, none matching), then the wrap-around scan hits the
        //     empty slot 0 and returns -1 (lib.c:621)
        let mut k5 = key_with_pos(&p, seed, 8, 5, &mut rng);
        assert_eq!(m.get(&p, &mut k5), -1, "wrap-scan miss");
        m.check("wrap-scan miss");
        assert_eq!(m.get_ts(&p, &mut k5), -1);
        m.check("wrap-scan miss _ts");
        assert_eq!(m.del(&p, &mut k5, 0), 0);
        m.check("wrap-scan miss del");

        m.free(&p);
    }
}

// ===========================================================================
// Rows 8..12 — stbds_hmget_key(_ts) miss paths
// ===========================================================================

/// #8 — `stbds_hmget_key_ts(NULL, …)`.
#[test]
fn err_08_get_ts_null() {
    let p = Pair::new();
    for &elemsize in &[1usize, 8, 16, 24] {
        let mut k = 42u32.to_le_bytes();
        let mut tc: isize = 0x5A5A;
        let mut tr: isize = 0x5A5A;
        let (hc, hr) = unsafe {
            (
                (p.c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    &mut tc,
                    0,
                ),
                (p.r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    &mut tr,
                    0,
                ),
            )
        };
        assert_eq!(tc, -1, "C temp");
        assert_eq!(tr, -1, "Rust temp");
        assert!(!hc.is_null() && !hr.is_null());
        let (sc, sr) = unsafe {
            (
                snap_map(hc, elemsize, KeyKind::Binary, false),
                snap_map(hr, elemsize, KeyKind::Binary, false),
            )
        };
        eq_snap("hmget_key_ts(NULL)", &sc, &sr);
        assert_eq!(sc.length, 1);
        assert_eq!(sc.capacity, 4);
        assert!(!sc.has_table);
        assert_eq!(sc.elems[0], vec![0u8; elemsize], "element 0 must be zeroed");
        unsafe {
            (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// #9 — `hash_table == NULL`: `*temp = -1`, `a` returned unchanged.
#[test]
fn err_09_get_ts_no_table() {
    let p = Pair::new();
    let elemsize = 16usize;
    unsafe {
        let hc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
        let hr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
        for key in [0u32, 1, 7, 0xFFFF_FFFF] {
            let mut k = key.to_le_bytes();
            let mut tc: isize = 5;
            let mut tr: isize = 5;
            let bc = (p.c.hmget_key_ts)(hc, elemsize, k.as_mut_ptr() as *mut c_void, 4, &mut tc, 0);
            let br = (p.r.hmget_key_ts)(hr, elemsize, k.as_mut_ptr() as *mut c_void, 4, &mut tr, 0);
            assert_eq!((tc, bc == hc), (-1, true), "C side");
            assert_eq!((tr, br == hr), (-1, true), "Rust side");
            assert!(table_of(hc, elemsize).is_null(), "no table must be created");
            assert!(table_of(hr, elemsize).is_null(), "no table must be created");
        }
        (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// #10 — key not present: `*temp = -1`, state unchanged.
#[test]
fn err_10_get_ts_missing_key() {
    let p = Pair::new();
    let mut rng = Rng::new(0xE7_10);
    for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (24, 16)] {
        p.seed(0x3141_5926);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        for i in 0..20u64 {
            let mut k = vec![0u8; keysize];
            k[..8.min(keysize)].copy_from_slice(&i.to_le_bytes()[..8.min(keysize)]);
            m.put(&p, &mut k, &vec![0xAAu8; elemsize.saturating_sub(keysize)]);
        }
        let before = m.snaps().0;
        for _ in 0..200 {
            let mut k = vec![0u8; keysize];
            let v = rng.next_u64() | 0xFFFF_0000_0000_0000;
            k[..8.min(keysize)].copy_from_slice(&v.to_le_bytes()[..8.min(keysize)]);
            assert_eq!(m.get_ts(&p, &mut k), -1, "miss must be -1");
            m.check("get_ts miss");
            let now = m.snaps().0;
            assert_eq!(now.length, before.length);
            assert_eq!(now.used_count, before.used_count);
            assert_eq!(now.tombstone_count, before.tombstone_count);
            assert_eq!(now.buckets, before.buckets, "buckets must not change");
        }
        m.free(&p);
    }
}

/// #11 — `stbds_hmget_key` additionally stores the `-1` in the header's `temp`.
#[test]
fn err_11_get_key_miss_temp() {
    let p = Pair::new();
    let mut rng = Rng::new(0xE7_11);
    let elemsize = 8usize;
    p.seed(1);
    let mut m = MapPair::null(elemsize, 4, STBDS_HM_BINARY, KeyKind::Binary);
    for i in 0..10u32 {
        let mut k = i.to_le_bytes();
        m.put(&p, &mut k, &[1u8, 2, 3, 4]);
    }
    for _ in 0..200 {
        let mut k = (rng.next_u32() | 0x8000_0000).to_le_bytes();
        let t = m.get(&p, &mut k);
        assert_eq!(t, -1);
        let (sc, sr) = m.snaps();
        eq_snap("get miss header temp", &sc, &sr);
        assert_eq!(sc.temp, -1, "hmget_key must write -1 into the header");
    }
    m.free(&p);
}

/// #12 — `stbds_hmget_key(NULL, …)` writes `temp` into the fresh header.
#[test]
fn err_12_get_key_null() {
    let p = Pair::new();
    for &elemsize in &[1usize, 8, 16, 24] {
        let mut k = 3u32.to_le_bytes();
        let (hc, hr) = unsafe {
            (
                (p.c.hmget_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                ),
                (p.r.hmget_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                ),
            )
        };
        let (sc, sr) = unsafe {
            (
                snap_map(hc, elemsize, KeyKind::Binary, false),
                snap_map(hr, elemsize, KeyKind::Binary, false),
            )
        };
        eq_snap("hmget_key(NULL)", &sc, &sr);
        assert_eq!((sc.temp, sc.length, sc.capacity), (-1, 1, 4));
        unsafe {
            (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// Rows 13..15 — null-handle acceptance
// ===========================================================================

/// #13 — `stbds_hmput_default(NULL, …)`.
#[test]
fn err_13_put_default_null() {
    let p = Pair::new();
    for &elemsize in &[0usize, 1, 8, 16, 24] {
        let (hc, hr) = unsafe {
            (
                (p.c.hmput_default)(std::ptr::null_mut(), elemsize),
                (p.r.hmput_default)(std::ptr::null_mut(), elemsize),
            )
        };
        let (sc, sr) = unsafe {
            (
                snap_map(hc, elemsize, KeyKind::Binary, false),
                snap_map(hr, elemsize, KeyKind::Binary, false),
            )
        };
        eq_snap("hmput_default(NULL)", &sc, &sr);
        assert_eq!((sc.length, sc.capacity), (1, 4));
        unsafe {
            (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// #14 — `length == 0` (forged) makes `stbds_hmput_default` re-initialise.
#[test]
fn err_14_put_default_len0() {
    let p = Pair::new();
    let elemsize = 16usize;
    unsafe {
        let mut hc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
        let mut hr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
        // scribble over element 0 and force length back to 0
        std::ptr::write_bytes(hc as *mut u8, 0xCD, elemsize);
        std::ptr::write_bytes(hr as *mut u8, 0xCD, elemsize);
        (*header_of(hc, elemsize)).length = 0;
        (*header_of(hr, elemsize)).length = 0;
        hc = (p.c.hmput_default)(hc, elemsize);
        hr = (p.r.hmput_default)(hr, elemsize);
        let (sc, sr) = (
            snap_map(hc, elemsize, KeyKind::Binary, false),
            snap_map(hr, elemsize, KeyKind::Binary, false),
        );
        eq_snap("hmput_default(length==0)", &sc, &sr);
        assert_eq!(sc.length, 1);
        assert_eq!(sc.elems[0], vec![0u8; elemsize], "element 0 must be re-zeroed");
        (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// #15 — `stbds_hmput_key(NULL, …)` bootstraps the map.
#[test]
fn err_15_put_null() {
    let p = Pair::new();
    for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (24, 16)] {
        p.seed(0x3141_5926);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        let mut k = vec![9u8; keysize];
        let t = m.put(&p, &mut k, &vec![7u8; elemsize - keysize]);
        assert_eq!(t, 0, "first insert gets temp 0");
        m.check("hmput_key(NULL)");
        let s = m.snaps().0;
        assert_eq!((s.length, s.capacity, s.slot_count, s.used_count), (2, 4, 8, 1));
        m.free(&p);
    }
}

/// #16 / #21 — the two "unreachable" asserts in `stbds_hmput_key` (L778) and
/// `stbds_hmdel_key` (L828). Evidence: heavy randomized traffic through exactly
/// those code paths never aborts on either side.
#[test]
fn err_16_21_unreachable_asserts_never_fire() {
    let p = Pair::new();
    for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (24, 16)] {
        for &gseed in &[0usize, 0x3141_5926, usize::MAX] {
            let mut rng = Rng::new(0xE7_1621 ^ gseed as u64 ^ elemsize as u64);
            p.seed(gseed);
            let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
            let mut live: Vec<u64> = Vec::new();
            for _ in 0..1500 {
                if live.len() < 4 || rng.next_u64() % 2 == 0 {
                    let v = rng.next_u64();
                    let mut k = vec![0u8; keysize];
                    let n = 8.min(keysize);
                    k[..n].copy_from_slice(&v.to_le_bytes()[..n]);
                    m.put(&p, &mut k, &vec![0x5Au8; elemsize - keysize]);
                    live.push(v);
                } else {
                    let i = rng.below(live.len());
                    let v = live.swap_remove(i);
                    let mut k = vec![0u8; keysize];
                    let n = 8.min(keysize);
                    k[..n].copy_from_slice(&v.to_le_bytes()[..n]);
                    m.del(&p, &mut k, 0);
                }
            }
            m.check("unreachable-assert fuzz");
            m.free(&p);
        }
    }
}

/// #17 — `stbds_make_hash_index` threshold assert (reached via a forged
/// `slot_count`).
#[test]
fn err_17_make_hash_index_assert() {
    let r = diff_crash("make_hash_index_assert");
    assert_eq!(r.signal, Some(6), "expected SIGABRT, got {r:?}");
    let msg = r.assert_msg.as_deref().unwrap_or("");
    assert!(
        msg.contains("lib.c:401")
            && msg.contains("stbds_make_hash_index")
            && msg.contains(
                "t->used_count_threshold + t->tombstone_count_threshold < t->slot_count"
            ),
        "unexpected assert message: {msg:?}"
    );
}

// ===========================================================================
// Rows 18..20, 40 — stbds_hmdel_key rejection paths
// ===========================================================================

/// #18 — `stbds_hmdel_key(NULL, …)` returns NULL.
#[test]
fn err_18_del_null() {
    let p = Pair::new();
    for &elemsize in &[1usize, 8, 16] {
        for &mode in &[-1 as c_int, 0, 1, 2, c_int::MAX, c_int::MIN] {
            let mut k = 1u32.to_le_bytes();
            let (bc, br) = unsafe {
                (
                    (p.c.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        k.as_mut_ptr() as *mut c_void,
                        4,
                        0,
                        mode,
                    ),
                    (p.r.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        k.as_mut_ptr() as *mut c_void,
                        4,
                        0,
                        mode,
                    ),
                )
            };
            assert!(
                bc.is_null() && br.is_null(),
                "hmdel_key(NULL) must return NULL (mode={mode}) C={bc:?} Rust={br:?}"
            );
        }
    }
}

/// #19 — `hash_table == NULL`: `temp = 0`, `a` returned, nothing removed.
#[test]
fn err_19_del_no_table() {
    let p = Pair::new();
    let elemsize = 16usize;
    unsafe {
        let hc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
        let hr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
        (*header_of(hc, elemsize)).temp = 12345;
        (*header_of(hr, elemsize)).temp = 12345;
        for key in [0u32, 1, 0xDEAD_BEEF] {
            let mut k = key.to_le_bytes();
            let bc = (p.c.hmdel_key)(hc, elemsize, k.as_mut_ptr() as *mut c_void, 4, 0, 0);
            let br = (p.r.hmdel_key)(hr, elemsize, k.as_mut_ptr() as *mut c_void, 4, 0, 0);
            assert_eq!(bc, hc);
            assert_eq!(br, hr);
            let (sc, sr) = (
                snap_map(hc, elemsize, KeyKind::Binary, false),
                snap_map(hr, elemsize, KeyKind::Binary, false),
            );
            eq_snap("hmdel_key without a table", &sc, &sr);
            assert_eq!(sc.temp, 0, "temp must be reset to 0");
            assert_eq!(sc.length, 1, "nothing may be removed");
        }
        (p.c.hmfree_func)((hc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((hr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// #20 — deleting an absent key: `temp = 0`, everything unchanged.
#[test]
fn err_20_del_missing_key() {
    let p = Pair::new();
    let mut rng = Rng::new(0xE7_20);
    for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (24, 16)] {
        p.seed(0x3141_5926);
        let mut m = MapPair::null(elemsize, keysize, STBDS_HM_BINARY, KeyKind::Binary);
        for i in 0..15u64 {
            let mut k = vec![0u8; keysize];
            let n = 8.min(keysize);
            k[..n].copy_from_slice(&i.to_le_bytes()[..n]);
            m.put(&p, &mut k, &vec![3u8; elemsize - keysize]);
        }
        let before = m.snaps().0;
        for _ in 0..200 {
            let mut k = vec![0u8; keysize];
            let v = rng.next_u64() | 0xFF00_0000_0000_0000;
            let n = 8.min(keysize);
            k[..n].copy_from_slice(&v.to_le_bytes()[..n]);
            assert_eq!(m.del(&p, &mut k, 0), 0, "absent delete must report 0");
            m.check("absent delete");
            let now = m.snaps().0;
            assert_eq!(now.length, before.length);
            assert_eq!(now.used_count, before.used_count);
            assert_eq!(now.tombstone_count, before.tombstone_count);
            assert_eq!(now.buckets, before.buckets);
            assert_eq!(now.temp, 0);
        }
        m.free(&p);
    }
}

/// #23 — `stbds_hmdel_key`: `STBDS_ASSERT(slot >= 0)` after the re-index probe.
#[test]
fn err_23_del_reindex_assert() {
    let r = diff_crash("del_reindex_assert");
    assert_eq!(r.signal, Some(6), "expected SIGABRT, got {r:?}");
    let msg = r.assert_msg.as_deref().unwrap_or("");
    assert!(
        msg.contains("lib.c:846") && msg.contains("stbds_hmdel_key") && msg.contains("slot >= 0"),
        "unexpected assert message: {msg:?}"
    );
}

/// #24 — `stbds_hmdel_key`: `STBDS_ASSERT(b->index[i] == final_index)`.
#[test]
fn err_24_del_reindex_wrong_slot() {
    let r = diff_crash("del_reindex_wrong_slot");
    assert_eq!(r.signal, Some(6), "expected SIGABRT, got {r:?}");
    let msg = r.assert_msg.as_deref().unwrap_or("");
    assert!(
        msg.contains("lib.c:849")
            && msg.contains("stbds_hmdel_key")
            && msg.contains("b->index[i] == final_index"),
        "unexpected assert message: {msg:?}"
    );
}

/// #40 — `keyoffset != 0` while keys live at offset 0: probe misses, no delete.
#[test]
fn err_40_del_keyoffset_mismatch() {
    let p = Pair::new();
    let elemsize = 8usize;
    p.seed(0x3141_5926);
    let mut m = MapPair::null(elemsize, 4, STBDS_HM_BINARY, KeyKind::Binary);
    let mut keys = Vec::new();
    for i in 1..=20u32 {
        let mut k = i.to_le_bytes();
        // value != key so the offset-4 memcmp cannot accidentally match
        m.put(&p, &mut k, &(!i).to_le_bytes());
        keys.push(i);
    }
    let before = m.snaps().0;
    for &i in &keys {
        let mut k = i.to_le_bytes();
        assert_eq!(
            m.del(&p, &mut k, 4),
            0,
            "delete with a mismatched keyoffset must be a no-op"
        );
        m.check("keyoffset mismatch delete");
        let now = m.snaps().0;
        assert_eq!(now.length, before.length);
        assert_eq!(now.used_count, before.used_count);
        assert_eq!(now.buckets, before.buckets);
    }
    // and the map is still fully intact
    for &i in &keys {
        let mut k = i.to_le_bytes();
        assert!(m.get(&p, &mut k) >= 0);
        m.check("intact after mismatched deletes");
    }
    m.free(&p);
}

// ===========================================================================
// Rows 26, 29, 33, 44, 45, 48, 50 — remaining fatal paths
// ===========================================================================

/// #26 — `stbds_stralloc` on a forged arena (`storage == NULL`,
/// `remaining == SIZE_MAX`) skips the grow branch and writes through a bogus
/// pointer.
#[test]
fn err_26_stralloc_null_storage() {
    let r = diff_crash("stralloc_null_storage");
    assert!(
        r.signal.is_some(),
        "expected a fatal signal from stralloc on a NULL-storage arena, got {r:?}"
    );
}

/// #29 — `stbds_strreset(NULL)`.
#[test]
fn err_29_strreset_null() {
    let r = diff_crash_segv("strreset_null");
    assert!(r.signal.is_some(), "expected a fatal signal, got {r:?}");
}

/// #33 — `stbds_hash_string(NULL, seed)`.
#[test]
fn err_33_hash_string_null() {
    let r = diff_crash_segv("hash_string_null");
    assert!(r.signal.is_some(), "expected a fatal signal, got {r:?}");
}

/// #44 — `intput(9)` trips the `hmget(intmap, num) == 7` assert at lib.c:955.
#[test]
fn err_44_intput_9_aborts() {
    let r = diff_crash("intput_9");
    assert_eq!(r.signal, Some(6), "expected SIGABRT, got {r:?}");
    let msg = r.assert_msg.as_deref().unwrap_or("");
    assert!(
        msg.contains("lib.c:955")
            && msg.contains("intput")
            && msg.contains("hmget(intmap, num) == 7"),
        "unexpected assert message: {msg:?}"
    );
}

/// #45 — `intput(11)` trips the same assert (key 11's value was overwritten to 3).
#[test]
fn err_45_intput_11_aborts() {
    let r = diff_crash("intput_11");
    assert_eq!(r.signal, Some(6), "expected SIGABRT, got {r:?}");
    let msg = r.assert_msg.as_deref().unwrap_or("");
    assert!(
        msg.contains("lib.c:955")
            && msg.contains("intput")
            && msg.contains("hmget(intmap, num) == 7"),
        "unexpected assert message: {msg:?}"
    );
}

/// #48 — `stbds_hmget_key_ts` with a NULL `temp` out-parameter.
#[test]
fn err_48_get_ts_null_temp() {
    let r = diff_crash_segv("get_ts_null_temp");
    assert!(r.signal.is_some(), "expected a fatal signal, got {r:?}");
}

/// #50 — `realloc` failure inside `stbds_arrgrowf`: `b = NULL + 32` is then
/// dereferenced to write the header.
#[test]
fn err_50_arrgrowf_oom() {
    let r = diff_crash_segv("arrgrowf_oom");
    assert!(
        r.signal.is_some(),
        "expected a fatal signal writing the header through NULL+32, got {r:?}"
    );
}
