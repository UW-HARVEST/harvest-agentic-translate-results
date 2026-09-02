//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Rejections in `stb_ds` are sentinel values (`-1` / `-2` / `NULL` / `temp`
//! flag) or `assert()` aborts, so "same error" means: same returned sentinel,
//! same `temp`, same structural state — or, for the crashing rows, the same
//! termination signal observed in a forked child.

mod common;

use common::*;
use std::collections::HashSet;
use std::ffi::{c_char, c_int, c_void, CString};

// ---------------------------------------------------------------------------
// crash-equivalence helper (rows 5, 6, 52 and the oversized-length boundary)
// ---------------------------------------------------------------------------

/// Runs `f` in a forked child and returns `(exited_code, term_signal)`.
fn child_outcome<F: FnOnce()>(f: F) -> (Option<i32>, Option<i32>) {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut st: c_int = 0;
        assert!(waitpid(pid, &mut st, 0) == pid);
        let exited = (st & 0x7f) == 0;
        if exited {
            (Some((st >> 8) & 0xff), None)
        } else {
            (None, Some(st & 0x7f))
        }
    }
}

fn same_crash(what: &str, cf: impl FnOnce(), rf: impl FnOnce()) {
    let a = child_outcome(cf);
    let b = child_outcome(rf);
    assert_eq!(a, b, "{what}: C outcome {a:?} != Rust outcome {b:?}");
}

/// The intentionally-UB rows (null deref, allocation failure) reach the fault
/// through raw pointer writes.  A `dev`-profile `cdylib` turns some of those
/// into a Rust panic instead, so crash equivalence is only asserted against the
/// release artifact — which is the crate's shipped configuration
/// (`[profile.release] panic = "abort"`, `crate-type = ["cdylib"]`).
fn require_release_so(what: &str) -> bool {
    let p = rust_so_profile();
    if p != "release" {
        eprintln!("SKIP {what}: Rust .so is a `{p}` build; run `cargo build --release`");
        return false;
    }
    true
}

fn distinct_keys(rng: &mut Rng, keysize: usize, n: usize) -> Vec<Vec<u8>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let k = rng.bytes(keysize);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

fn distinct_strings(rng: &mut Rng, n: usize) -> Vec<CString> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let s = rng.ascii_len(1, 24);
        if seen.insert(s.clone()) {
            out.push(s);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// rows 1..4 — stbds_arrgrowf rejections / early-outs
// ---------------------------------------------------------------------------

#[test]
fn e01_arrgrowf_growth_not_needed() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 101);
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 40] {
            let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 64);
            let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 64);
            let cap = arr_capacity(a);
            assert_eq!(cap, arr_capacity(b));
            for _ in 0..200 {
                let min_cap = rng.below(cap + 1);
                let addlen = rng.below(cap - min_cap.min(cap) + 1);
                let a2 = (c.arrgrowf)(a, elemsize, addlen, min_cap);
                let b2 = (r.arrgrowf)(b, elemsize, addlen, min_cap);
                assert_eq!(a2, a, "C must early-out with the same pointer");
                assert_eq!(b2, b, "R must early-out with the same pointer");
                same("e01", &dump_arr(a, 0), &dump_arr(b, 0));
            }
            (c.arrfreef)(a);
            (r.arrfreef)(b);
        }
    }
}

#[test]
fn e02_arrgrowf_fresh_header_init() {
    let (c, r, _g) = both();
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 40] {
            for (addlen, min_cap) in [(0usize, 1usize), (1, 0), (7, 3), (100, 5), (0, 1000)] {
                let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                assert_eq!(arr_length(a), 0);
                assert_eq!(arr_length(b), 0);
                assert!(arr_table(a).is_null() && arr_table(b).is_null());
                assert_eq!(arr_temp(a), 0);
                assert_eq!(arr_temp(b), 0);
                same("e02", &dump_arr(a, 0), &dump_arr(b, 0));
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
        }
    }
}

#[test]
fn e03_arrgrowf_null_null_returns_null() {
    let (c, r, _g) = both();
    unsafe {
        for elemsize in [0usize, 1, 4, 8, 16, usize::MAX] {
            let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(a.is_null(), "C arrgrowf(NULL,{elemsize},0,0)");
            assert!(b.is_null(), "R arrgrowf(NULL,{elemsize},0,0)");
        }
    }
}

#[test]
fn e04_arrgrowf_mincap_bumped_to_four() {
    let (c, r, _g) = both();
    unsafe {
        for elemsize in [1usize, 8, 16] {
            for min_cap in 1..=3usize {
                let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                assert_eq!(arr_capacity(a), 4, "C min_cap<4 must be bumped to 4");
                assert_eq!(arr_capacity(b), 4, "R min_cap<4 must be bumped to 4");
                same("e04", &dump_arr(a, 0), &dump_arr(b, 0));
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
            // addlen 1..3 with min_cap 0 takes the same branch
            for addlen in 1..=3usize {
                let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 0);
                let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 0);
                assert_eq!(arr_capacity(a), 4);
                assert_eq!(arr_capacity(b), 4);
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 5, 6, 52 — the crashing rows, compared through a forked child
// ---------------------------------------------------------------------------

#[test]
fn e05_arrgrowf_allocation_failure_crashes_identically() {
    if !require_release_so("e05") {
        return;
    }
    let (c, r, _g) = both();
    let cg = c.arrgrowf;
    let rg = r.arrgrowf;
    for (elemsize, min_cap) in [
        (usize::MAX / 2, 4usize),
        (1usize << 62, 8),
        (1, usize::MAX / 2),
    ] {
        same_crash(
            &format!("arrgrowf(NULL,{elemsize},0,{min_cap})"),
            || unsafe {
                let _ = cg(std::ptr::null_mut(), elemsize, 0, min_cap);
            },
            || unsafe {
                let _ = rg(std::ptr::null_mut(), elemsize, 0, min_cap);
            },
        );
    }
}

#[test]
fn e06_arrfreef_null_crashes_identically() {
    if !require_release_so("e06") {
        return;
    }
    let (c, r, _g) = both();
    let cf = c.arrfreef;
    let rf = r.arrfreef;
    same_crash(
        "arrfreef(NULL)",
        || unsafe { cf(std::ptr::null_mut()) },
        || unsafe { rf(std::ptr::null_mut()) },
    );
}

#[test]
fn e52_hash_null_pointer_crashes_identically() {
    if !require_release_so("e52") {
        return;
    }
    let (c, r, _g) = both();
    let cb = c.hash_bytes;
    let rb = r.hash_bytes;
    for len in [1usize, 8, 64] {
        same_crash(
            &format!("hash_bytes(NULL,{len},0)"),
            || unsafe {
                let _ = cb(std::ptr::null_mut(), len, 0);
            },
            || unsafe {
                let _ = rb(std::ptr::null_mut(), len, 0);
            },
        );
    }
    let cs = c.hash_string;
    let rs = r.hash_string;
    same_crash(
        "hash_string(NULL,0)",
        || unsafe {
            let _ = cs(std::ptr::null_mut(), 0);
        },
        || unsafe {
            let _ = rs(std::ptr::null_mut(), 0);
        },
    );
}

#[test]
fn e_boundary_oversized_hash_len_crashes_identically() {
    if !require_release_so("e_boundary_oversized") {
        return;
    }
    let (c, r, _g) = both();
    let cb = c.hash_bytes;
    let rb = r.hash_bytes;
    for len in [1usize << 40, usize::MAX / 2] {
        same_crash(
            &format!("hash_bytes(small buf, {len}, 0)"),
            || unsafe {
                let mut b = [0u8; 8];
                let _ = cb(b.as_mut_ptr() as *mut c_void, len, 0);
            },
            || unsafe {
                let mut b = [0u8; 8];
                let _ = rb(b.as_mut_ptr() as *mut c_void, len, 0);
            },
        );
    }
}

#[test]
fn e_boundary_hmget_null_key_crashes_identically() {
    if !require_release_so("e_boundary_null_key") {
        return;
    }
    let (c, r, _g) = both();
    // A NULL key on a *populated* map reaches hash_string / hash_bytes.
    for mode in [HM_BINARY, HM_STRING] {
        let cp = c.hmput_key;
        let cg = c.hmget_key;
        let rp = r.hmput_key;
        let rg = r.hmget_key;
        let cs = c.rand_seed;
        let rs = r.rand_seed;
        same_crash(
            &format!("hmget_key(populated, NULL, mode={mode})"),
            move || unsafe {
                cs(5);
                let k = CString::new("abc").unwrap();
                let m = cp(
                    std::ptr::null_mut(),
                    16,
                    k.as_ptr() as *mut c_void,
                    8,
                    mode,
                );
                let _ = cg(m, 16, std::ptr::null_mut(), 8, mode);
            },
            move || unsafe {
                rs(5);
                let k = CString::new("abc").unwrap();
                let m = rp(
                    std::ptr::null_mut(),
                    16,
                    k.as_ptr() as *mut c_void,
                    8,
                    mode,
                );
                let _ = rg(m, 16, std::ptr::null_mut(), 8, mode);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// rows 7, 20, 30, 31, 32, 33, 45, 55, 56, 57 — documented-only
// ---------------------------------------------------------------------------

/// The `ERRORS.md` rows whose trigger is provably unreachable through the
/// public ABI.  Kept as an explicit, reviewable list rather than silence.
#[test]
fn e_documented_only_rows_are_unreachable() {
    // row 7  — make_hash_index threshold assert: for any power-of-two
    //          slot_count >= 8, (sc - sc/4) + (sc/8 + sc/16) = 0.9375*sc < sc.
    for log2 in 3..=40u32 {
        let sc: usize = 1 << log2;
        let used = sc - (sc >> 2);
        let tomb = (sc >> 3) + (sc >> 4);
        assert!(used + tomb < sc, "slot_count={sc}");
    }
    // row 31 — `table->used_count >= 0` is a tautology for size_t.
    // rows 20, 30, 32, 33, 45 — asserts guarding invariants that the other
    //          tests in this file establish hold (arrgrowf really grows, the
    //          slot index is masked by slot_count-1, the moved element is
    //          always re-findable, `len <= remaining` after a block alloc).
    // rows 55..57 — `str_put`'s own asserts; `tests/configs_driver.rs` proves
    //          they never fire because both libraries print the expected line
    //          and exit normally for every `num` tried.
}

// ---------------------------------------------------------------------------
// rows 8..13 — lookup rejections
// ---------------------------------------------------------------------------

#[test]
fn e08_find_slot_miss_returns_minus_one() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 108);
    unsafe {
        (c.rand_seed)(3);
        (r.rand_seed)(3);
        let mut pc: *mut c_void = std::ptr::null_mut();
        let mut pr: *mut c_void = std::ptr::null_mut();
        let shape = MapShape::bytes(16, 8);
        let present = distinct_keys(&mut rng, 8, 100);
        for (i, k) in present.iter().enumerate() {
            let mut kk = k.clone();
            pc = (c.hmput_key)(pc, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
            pr = (r.hmput_key)(pr, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
            let t = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
            fill_value(pc, shape, (t + 1) as usize, i as u64);
            fill_value(pr, shape, (t + 1) as usize, i as u64);
        }
        for _ in 0..2000 {
            let mut k = rng.bytes(8);
            if present.contains(&k) {
                continue;
            }
            let mut tc: isize = 7;
            let mut tr: isize = 7;
            pc = (c.hmget_key_ts)(pc, 16, k.as_mut_ptr() as *mut c_void, 8, &mut tc, HM_BINARY);
            pr = (r.hmget_key_ts)(pr, 16, k.as_mut_ptr() as *mut c_void, 8, &mut tr, HM_BINARY);
            assert_eq!(tc, INDEX_EMPTY);
            assert_eq!(tr, INDEX_EMPTY);
        }
        hmfree(c, pc, 16);
        hmfree(r, pr, 16);
    }
}

#[test]
fn e09_e10_e11_e12_get_sentinels() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 109);
    unsafe {
        for mode in [HM_BINARY, HM_STRING] {
            // row 9: a == NULL
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let mut k = rng.bytes(8);
            let kp = k.as_mut_ptr() as *mut c_void;
            let mut tc: isize = 99;
            let mut tr: isize = 99;
            let mut pc = (c.hmget_key_ts)(std::ptr::null_mut(), 16, kp, 8, &mut tc, mode);
            let mut pr = (r.hmget_key_ts)(std::ptr::null_mut(), 16, kp, 8, &mut tr, mode);
            assert_eq!(tc, INDEX_EMPTY, "row9 C");
            assert_eq!(tr, INDEX_EMPTY, "row9 R");
            assert!(!pc.is_null() && !pr.is_null());
            same("row9 state", &dump_map(pc, MapShape::bytes(16, 8)), &dump_map(pr, MapShape::bytes(16, 8)));

            // row 10: array exists, hash_table == NULL
            assert!(arr_table((pc as *mut u8).sub(16) as *mut c_void).is_null());
            tc = 99;
            tr = 99;
            pc = (c.hmget_key_ts)(pc, 16, kp, 8, &mut tc, mode);
            pr = (r.hmget_key_ts)(pr, 16, kp, 8, &mut tr, mode);
            assert_eq!(tc, -1, "row10 C");
            assert_eq!(tr, -1, "row10 R");

            // row 12: hmget_key also writes the sentinel into header->temp
            pc = (c.hmget_key)(pc, 16, kp, 8, mode);
            pr = (r.hmget_key)(pr, 16, kp, 8, mode);
            assert_eq!(arr_temp((pc as *mut u8).sub(16) as *mut c_void), -1);
            assert_eq!(arr_temp((pr as *mut u8).sub(16) as *mut c_void), -1);
            hmfree(c, pc, 16);
            hmfree(r, pr, 16);
        }

        // row 11: populated table, absent key
        (c.rand_seed)(2);
        (r.rand_seed)(2);
        let mut pc: *mut c_void = std::ptr::null_mut();
        let mut pr: *mut c_void = std::ptr::null_mut();
        let shape11 = MapShape::strp(16);
        let present = distinct_strings(&mut rng, 50);
        for (i, s) in present.iter().enumerate() {
            pc = (c.hmput_key)(pc, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
            pr = (r.hmput_key)(pr, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
            let t = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
            fill_value(pc, shape11, (t + 1) as usize, i as u64);
            fill_value(pr, shape11, (t + 1) as usize, i as u64);
        }
        for _ in 0..500 {
            let s = rng.ascii_len(25, 10); // cannot collide with the present set
            pc = (c.hmget_key)(pc, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
            pr = (r.hmget_key)(pr, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
            assert_eq!(arr_temp((pc as *mut u8).sub(16) as *mut c_void), INDEX_EMPTY);
            assert_eq!(arr_temp((pr as *mut u8).sub(16) as *mut c_void), INDEX_EMPTY);
        }
        hmfree(c, pc, 16);
        hmfree(r, pr, 16);
    }
}

#[test]
fn e13_out_of_range_mode_get() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 113);
    // mode >= 1 must take the string path, mode <= 0 the binary path, for any
    // int the caller passes across the FFI boundary.
    let string_modes: Vec<c_int> = vec![1, 2, 3, 7, 127, 1000, c_int::MAX];
    let binary_modes: Vec<c_int> = vec![0, -1, -2, -7, -1000, c_int::MIN];
    unsafe {
        for &m in &string_modes {
            (c.rand_seed)(17);
            (r.rand_seed)(17);
            let mut pc = (c.shmode_func)(16, SH_DEFAULT);
            let mut pr = (r.shmode_func)(16, SH_DEFAULT);
            let keys = distinct_strings(&mut rng, 40);
            for (i, s) in keys.iter().enumerate() {
                pc = (c.hmput_key)(pc, 16, s.as_ptr() as *mut c_void, 8, m);
                pr = (r.hmput_key)(pr, 16, s.as_ptr() as *mut c_void, 8, m);
                let tc = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                let tr = arr_temp((pr as *mut u8).sub(16) as *mut c_void);
                assert_eq!(tc, tr, "mode={m} put temp");
                fill_value(pc, MapShape::strp(16), (tc + 1) as usize, i as u64);
                fill_value(pr, MapShape::strp(16), (tr + 1) as usize, i as u64);
            }
            for s in &keys {
                // looking up with mode 1 must find what mode `m` inserted
                pc = (c.hmget_key)(pc, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
                pr = (r.hmget_key)(pr, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
                let tc = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                let tr = arr_temp((pr as *mut u8).sub(16) as *mut c_void);
                assert_eq!(tc, tr, "mode={m} get temp");
                assert!(tc >= 0, "mode={m}: string path expected");
            }
            same(
                &format!("e13 string mode={m}"),
                &dump_map(pc, MapShape::strp(16)),
                &dump_map(pr, MapShape::strp(16)),
            );
            hmfree(c, pc, 16);
            hmfree(r, pr, 16);
        }

        for &m in &binary_modes {
            (c.rand_seed)(19);
            (r.rand_seed)(19);
            let mut pc: *mut c_void = std::ptr::null_mut();
            let mut pr: *mut c_void = std::ptr::null_mut();
            let keys = distinct_keys(&mut rng, 8, 40);
            for (i, k) in keys.iter().enumerate() {
                let mut kk = k.clone();
                pc = (c.hmput_key)(pc, 16, kk.as_mut_ptr() as *mut c_void, 8, m);
                pr = (r.hmput_key)(pr, 16, kk.as_mut_ptr() as *mut c_void, 8, m);
                let tc = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                let tr = arr_temp((pr as *mut u8).sub(16) as *mut c_void);
                assert_eq!(tc, tr, "mode={m} put temp");
                fill_value(pc, MapShape::bytes(16, 8), (tc + 1) as usize, i as u64);
                fill_value(pr, MapShape::bytes(16, 8), (tr + 1) as usize, i as u64);
            }
            // string.mode must have stayed 0 (binary path on creation)
            let t = arr_table((pc as *mut u8).sub(16) as *mut c_void);
            assert_eq!(*t.add(hi::STRING + 17), 0, "mode={m} string.mode");
            for k in &keys {
                let mut kk = k.clone();
                pc = (c.hmget_key)(pc, 16, kk.as_mut_ptr() as *mut c_void, 8, m);
                pr = (r.hmget_key)(pr, 16, kk.as_mut_ptr() as *mut c_void, 8, m);
                let tc = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                let tr = arr_temp((pr as *mut u8).sub(16) as *mut c_void);
                assert_eq!(tc, tr, "mode={m} get temp");
                assert!(tc >= 0, "mode={m}: binary path expected to find the key");
            }
            same(
                &format!("e13 binary mode={m}"),
                &dump_map(pc, MapShape::bytes(16, 8)),
                &dump_map(pr, MapShape::bytes(16, 8)),
            );
            hmfree(c, pc, 16);
            hmfree(r, pr, 16);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 14..16 — stbds_hmput_default
// ---------------------------------------------------------------------------

#[test]
fn e14_e15_e16_hmput_default() {
    let (c, r, _g) = both();
    unsafe {
        for elemsize in [8usize, 16, 40] {
            let shape = MapShape::bytes(elemsize, 8);
            // row 14: a == NULL
            let pc = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let pr = (r.hmput_default)(std::ptr::null_mut(), elemsize);
            assert_eq!(arr_length((pc as *mut u8).sub(elemsize) as *mut c_void), 1);
            assert_eq!(arr_length((pr as *mut u8).sub(elemsize) as *mut c_void), 1);
            same("row14", &dump_map(pc, shape), &dump_map(pr, shape));

            // row 16: length != 0 => no-op
            let pc2 = (c.hmput_default)(pc, elemsize);
            let pr2 = (r.hmput_default)(pr, elemsize);
            assert_eq!(pc2, pc);
            assert_eq!(pr2, pr);
            same("row16", &dump_map(pc2, shape), &dump_map(pr2, shape));

            // row 15: length forced to 0 => re-grow
            *((pc as *mut u8).sub(elemsize + HDR_SIZE) as *mut usize) = 0;
            *((pr as *mut u8).sub(elemsize + HDR_SIZE) as *mut usize) = 0;
            let pc3 = (c.hmput_default)(pc2, elemsize);
            let pr3 = (r.hmput_default)(pr2, elemsize);
            assert_eq!(arr_length((pc3 as *mut u8).sub(elemsize) as *mut c_void), 1);
            assert_eq!(arr_length((pr3 as *mut u8).sub(elemsize) as *mut c_void), 1);
            same("row15", &dump_map(pc3, shape), &dump_map(pr3, shape));

            hmfree(c, pc3, elemsize);
            hmfree(r, pr3, elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 17..19, 21..24 — stbds_hmput_key
// ---------------------------------------------------------------------------

#[test]
fn e17_e18_hmput_key_bootstrap() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 117);
    unsafe {
        for mode in [HM_BINARY, HM_STRING, 5, -3] {
            (c.rand_seed)(23);
            (r.rand_seed)(23);
            let s = rng.ascii(6);
            let pc = (c.hmput_key)(std::ptr::null_mut(), 16, s.as_ptr() as *mut c_void, 8, mode);
            let pr = (r.hmput_key)(std::ptr::null_mut(), 16, s.as_ptr() as *mut c_void, 8, mode);
            {
                let shp = if mode >= HM_STRING {
                    MapShape::strp(16)
                } else {
                    MapShape::bytes(16, 8)
                };
                let t = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                fill_value(pc, shp, (t + 1) as usize, 5);
                fill_value(pr, shp, (t + 1) as usize, 5);
            }
            let tc = arr_table((pc as *mut u8).sub(16) as *mut c_void);
            let tr = arr_table((pr as *mut u8).sub(16) as *mut c_void);
            // row 18: slot_count == 8 and string.mode set from `mode`
            let sc = std::ptr::read_unaligned(tc.add(hi::SLOT_COUNT) as *const usize);
            let sr = std::ptr::read_unaligned(tr.add(hi::SLOT_COUNT) as *const usize);
            assert_eq!(sc, 8);
            assert_eq!(sr, 8);
            let mc = *tc.add(hi::STRING + 17);
            let mr = *tr.add(hi::STRING + 17);
            assert_eq!(mc, mr, "mode={mode} string.mode");
            assert_eq!(
                mc as c_int,
                if mode >= HM_STRING { SH_DEFAULT } else { 0 },
                "mode={mode} string.mode value"
            );
            let shape = if mode >= HM_STRING {
                MapShape::strp(16)
            } else {
                MapShape::bytes(16, 8)
            };
            same(
                &format!("row17/18 mode={mode}"),
                &dump_map(pc, shape),
                &dump_map(pr, shape),
            );
            hmfree(c, pc, 16);
            hmfree(r, pr, 16);
        }
    }
}

#[test]
fn e19_rehash_at_threshold() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 119);
    unsafe {
        (c.rand_seed)(29);
        (r.rand_seed)(29);
        let mut pc: *mut c_void = std::ptr::null_mut();
        let mut pr: *mut c_void = std::ptr::null_mut();
        let keys = distinct_keys(&mut rng, 8, 300);
        let mut seen_growth = 0;
        let mut prev = 0usize;
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            pc = (c.hmput_key)(pc, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
            pr = (r.hmput_key)(pr, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
            let tv = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
            fill_value(pc, MapShape::bytes(16, 8), (tv + 1) as usize, i as u64);
            fill_value(pr, MapShape::bytes(16, 8), (tv + 1) as usize, i as u64);
            let tc = arr_table((pc as *mut u8).sub(16) as *mut c_void);
            let tr = arr_table((pr as *mut u8).sub(16) as *mut c_void);
            let sc = std::ptr::read_unaligned(tc.add(hi::SLOT_COUNT) as *const usize);
            let sr = std::ptr::read_unaligned(tr.add(hi::SLOT_COUNT) as *const usize);
            assert_eq!(sc, sr, "slot_count at insert {i}");
            let uc = std::ptr::read_unaligned(tc.add(hi::USED_COUNT) as *const usize);
            let ur = std::ptr::read_unaligned(tr.add(hi::USED_COUNT) as *const usize);
            assert_eq!(uc, ur);
            if sc != prev {
                seen_growth += 1;
                prev = sc;
            }
        }
        assert!(seen_growth >= 6, "expected several rehashes, saw {seen_growth}");
        hmfree(c, pc, 16);
        hmfree(r, pr, 16);
    }
}

#[test]
fn e21_e22_duplicate_key_branches() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 121);
    unsafe {
        // Re-put existing keys at every probe position so that both the forward
        // in-bucket scan (row 21, which DOES update `temp_key`) and the
        // wrap-around scan (row 22, which does NOT) are taken.
        //
        // `stbds_make_hash_index` leaves `temp_key` uninitialised, so the field
        // is primed to 0 before every call and only compared when the call did
        // not rehash — otherwise both libraries would just be reporting
        // different heap garbage.
        let shape = MapShape::strp(16);
        let mut row21_hits = 0usize;
        let mut row22_hits = 0usize;
        for target in 0..8usize {
            (c.rand_seed)(31);
            (r.rand_seed)(31);
            let mut pc = (c.shmode_func)(16, SH_DEFAULT);
            let mut pr = (r.shmode_func)(16, SH_DEFAULT);
            let seed = {
                let t = map_table(pc, 16);
                std::ptr::read_unaligned(t.add(hi::SEED) as *const usize)
            };
            let mut keys: Vec<CString> = Vec::new();
            let mut tries = 0;
            while keys.len() < 12 && tries < 400_000 {
                tries += 1;
                let s = rng.ascii_len(4, 12);
                if keys.contains(&s) {
                    continue;
                }
                let mut h = (c.hash_string)(s.as_ptr() as *mut c_char, seed);
                let hr = (r.hash_string)(s.as_ptr() as *mut c_char, seed);
                assert_eq!(h, hr, "hash_string diverged while searching keys");
                if h < 2 {
                    h += 2;
                }
                if h & 7 == target {
                    keys.push(s);
                }
            }
            assert!(!keys.is_empty(), "no key found for probe position {target}");
            for (i, s) in keys.iter().enumerate() {
                pc = (c.hmput_key)(pc, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
                pr = (r.hmput_key)(pr, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
                let t = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                assert_eq!(t, arr_temp((pr as *mut u8).sub(16) as *mut c_void));
                fill_value(pc, shape, (t + 1) as usize, i as u64);
                fill_value(pr, shape, (t + 1) as usize, i as u64);
            }
            let len_after_insert = hmlen(pc, 16);
            for round in 0..3 {
                for (i, s) in keys.iter().enumerate() {
                    set_temp_key(pc, 16, 0);
                    set_temp_key(pr, 16, 0);
                    let tab_c_before = map_table(pc, 16);
                    let tab_r_before = map_table(pr, 16);
                    pc = (c.hmput_key)(pc, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
                    pr = (r.hmput_key)(pr, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
                    let tc = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                    let tr = arr_temp((pr as *mut u8).sub(16) as *mut c_void);
                    assert_eq!(tc, tr, "target={target} r{round} #{i} dup temp");
                    assert_eq!(
                        hmlen(pc, 16),
                        len_after_insert,
                        "target={target} r{round} #{i}: duplicate must not grow the map"
                    );
                    fill_value(pc, shape, (tc + 1) as usize, (100 * round + i) as u64);
                    fill_value(pr, shape, (tr + 1) as usize, (100 * round + i) as u64);
                    same(
                        &format!("e21/e22 target={target} r{round} #{i} map"),
                        &dump_map(pc, shape),
                        &dump_map(pr, shape),
                    );
                    let tab_c = map_table(pc, 16);
                    let tab_r = map_table(pr, 16);
                    assert_eq!(
                        tab_c != tab_c_before,
                        tab_r != tab_r_before,
                        "target={target} r{round} #{i}: rehash disagreement"
                    );
                    if tab_c == tab_c_before {
                        let kc = std::ptr::read_unaligned(tab_c as *const usize);
                        let kr = std::ptr::read_unaligned(tab_r as *const usize);
                        assert_eq!(
                            kc, kr,
                            "target={target} r{round} #{i}: temp_key write differs"
                        );
                        if kc == 0 {
                            row22_hits += 1; // wrap-around scan: temp_key untouched
                        } else {
                            assert_eq!(
                                kc, s.as_ptr() as usize,
                                "target={target}: temp_key must be the stored key pointer"
                            );
                            row21_hits += 1; // forward scan: temp_key updated
                        }
                    }
                }
            }
            hmfree(c, pc, 16);
            hmfree(r, pr, 16);
        }
        assert!(row21_hits > 0, "row 21 (forward scan sets temp_key) never taken");
        assert!(
            row22_hits > 0,
            "row 22 (wrap-around scan leaves temp_key) never taken"
        );
    }
}

#[test]
fn e23_unknown_string_mode_takes_memcpy_branch() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 123);
    unsafe {
        // Any `string.mode` outside {1,2,3} hits `default:` -> memcpy(keysize).
        for shmode in [0i32, 4, 5, 17, 100, 255] {
            (c.rand_seed)(37);
            (r.rand_seed)(37);
            let shape = MapShape::bytes(16, 8);
            let mut pc = (c.shmode_func)(16, shmode);
            let mut pr = (r.shmode_func)(16, shmode);
            let tc = arr_table((pc as *mut u8).sub(16) as *mut c_void);
            let tr = arr_table((pr as *mut u8).sub(16) as *mut c_void);
            assert_eq!(*tc.add(hi::STRING + 17), shmode as u8);
            assert_eq!(*tr.add(hi::STRING + 17), shmode as u8);
            let keys = distinct_keys(&mut rng, 8, 60);
            for (i, k) in keys.iter().enumerate() {
                let mut kk = k.clone();
                pc = (c.hmput_key)(pc, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
                pr = (r.hmput_key)(pr, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
                let t = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                assert_eq!(t, arr_temp((pr as *mut u8).sub(16) as *mut c_void));
                fill_value(pc, shape, (t + 1) as usize, i as u64);
                fill_value(pr, shape, (t + 1) as usize, i as u64);
                same(
                    &format!("e23 shmode={shmode} #{i}"),
                    &dump_map(pc, shape),
                    &dump_map(pr, shape),
                );
                // the key bytes must have been memcpy'd verbatim
                let e = (pc as *mut u8).add(16 * t as usize);
                assert_eq!(std::slice::from_raw_parts(e, 8), &k[..]);
            }
            hmfree(c, pc, 16);
            hmfree(r, pr, 16);
        }
    }
}

#[test]
fn e24_keysize_zero() {
    let (c, r, _g) = both();
    unsafe {
        for elemsize in [8usize, 16] {
            (c.rand_seed)(41);
            (r.rand_seed)(41);
            let shape = MapShape::bytes(elemsize, 0);
            let mut pc: *mut c_void = std::ptr::null_mut();
            let mut pr: *mut c_void = std::ptr::null_mut();
            let mut key = [0u8; 8];
            for i in 0..10 {
                key[0] = i as u8;
                // keysize == 0: hash_bytes over 0 bytes, memcmp of 0 bytes
                // (always equal), memcpy of 0 bytes.  Every key is "the same".
                pc = (c.hmput_key)(pc, elemsize, key.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                pr = (r.hmput_key)(pr, elemsize, key.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                let tc = arr_temp((pc as *mut u8).sub(elemsize) as *mut c_void);
                let tr = arr_temp((pr as *mut u8).sub(elemsize) as *mut c_void);
                assert_eq!(tc, tr, "keysize=0 put #{i} temp");
                fill_value(pc, shape, (tc + 1) as usize, i as u64);
                fill_value(pr, shape, (tr + 1) as usize, i as u64);
                same(
                    &format!("e24 e={elemsize} #{i}"),
                    &dump_map(pc, shape),
                    &dump_map(pr, shape),
                );
            }
            assert_eq!(hmlen(pc, elemsize), 1, "all zero-length keys collide");
            hmfree(c, pc, elemsize);
            hmfree(r, pr, elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// row 25 — stbds_shmode_func with out-of-range enum values
// ---------------------------------------------------------------------------

#[test]
fn e25_shmode_func_out_of_range_enum() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 125);
    let modes: Vec<c_int> = vec![
        0,
        1,
        2,
        3,
        4,
        5,
        255,
        256,
        257,
        258,
        259,
        260,
        512,
        -1,
        -2,
        -256,
        c_int::MAX,
        c_int::MIN,
    ];
    unsafe {
        for m in modes {
            (c.rand_seed)(43);
            (r.rand_seed)(43);
            let pc = (c.shmode_func)(16, m);
            let pr = (r.shmode_func)(16, m);
            let tc = arr_table((pc as *mut u8).sub(16) as *mut c_void);
            let tr = arr_table((pr as *mut u8).sub(16) as *mut c_void);
            let mc = *tc.add(hi::STRING + 17);
            let mr = *tr.add(hi::STRING + 17);
            assert_eq!(mc, mr, "shmode_func mode={m}: string.mode differs");
            assert_eq!(
                mc,
                (m as u32 & 0xff) as u8,
                "shmode_func mode={m}: expected (unsigned char) truncation"
            );
            // exercise the map with whichever key ownership `mc` selects
            let truncated = mc as c_int;
            let (mode, kind) = if truncated == SH_STRDUP || truncated == SH_ARENA
                || truncated == SH_DEFAULT
            {
                (HM_STRING, KeyKind::CStrPtr)
            } else {
                (HM_BINARY, KeyKind::Bytes)
            };
            let shape = MapShape {
                elemsize: 16,
                keyoffset: 0,
                keysize: 8,
                kind,
            };
            let mut pc = pc;
            let mut pr = pr;
            for i in 0..20 {
                let kp: *mut c_void;
                let owned_s;
                let mut owned_b;
                if mode == HM_STRING {
                    owned_s = rng.ascii_len(3, 12);
                    kp = owned_s.as_ptr() as *mut c_void;
                    pc = (c.hmput_key)(pc, 16, kp, 8, mode);
                    pr = (r.hmput_key)(pr, 16, kp, 8, mode);
                } else {
                    owned_b = rng.bytes(8);
                    kp = owned_b.as_mut_ptr() as *mut c_void;
                    pc = (c.hmput_key)(pc, 16, kp, 8, mode);
                    pr = (r.hmput_key)(pr, 16, kp, 8, mode);
                }
                let tc2 = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                let tr2 = arr_temp((pr as *mut u8).sub(16) as *mut c_void);
                assert_eq!(tc2, tr2, "mode={m} put #{i}");
                fill_value(pc, shape, (tc2 + 1) as usize, i as u64);
                fill_value(pr, shape, (tr2 + 1) as usize, i as u64);
                same(
                    &format!("e25 mode={m} #{i}"),
                    &dump_map(pc, shape),
                    &dump_map(pr, shape),
                );
            }
            hmfree(c, pc, 16);
            hmfree(r, pr, 16);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 26..29, 34..36 — stbds_hmdel_key
// ---------------------------------------------------------------------------

#[test]
fn e26_hmdel_null_returns_null() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 126);
    unsafe {
        for mode in [HM_BINARY, HM_STRING, 9, -9] {
            for elemsize in [8usize, 16, 40] {
                let mut k = rng.bytes(8);
                let kp = k.as_mut_ptr() as *mut c_void;
                let a = (c.hmdel_key)(std::ptr::null_mut(), elemsize, kp, 8, 0, mode);
                let b = (r.hmdel_key)(std::ptr::null_mut(), elemsize, kp, 8, 0, mode);
                assert!(a.is_null(), "C hmdel_key(NULL) must return NULL");
                assert!(b.is_null(), "R hmdel_key(NULL) must return NULL");
            }
        }
    }
}

#[test]
fn e27_hmdel_no_index() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 127);
    unsafe {
        for mode in [HM_BINARY, HM_STRING] {
            (c.rand_seed)(47);
            (r.rand_seed)(47);
            let mut k = rng.bytes(8);
            let kp = k.as_mut_ptr() as *mut c_void;
            let mut tc: isize = 0;
            let mut tr: isize = 0;
            let pc = (c.hmget_key_ts)(std::ptr::null_mut(), 16, kp, 8, &mut tc, mode);
            let pr = (r.hmget_key_ts)(std::ptr::null_mut(), 16, kp, 8, &mut tr, mode);
            // poison temp so we can prove hmdel_key sets it to 0
            *((pc as *mut u8).sub(16 + HDR_SIZE).add(24) as *mut isize) = 1234;
            *((pr as *mut u8).sub(16 + HDR_SIZE).add(24) as *mut isize) = 1234;
            let a = (c.hmdel_key)(pc, 16, kp, 8, 0, mode);
            let b = (r.hmdel_key)(pr, 16, kp, 8, 0, mode);
            assert_eq!(a, pc, "C must return the map unchanged");
            assert_eq!(b, pr, "R must return the map unchanged");
            assert_eq!(arr_temp((a as *mut u8).sub(16) as *mut c_void), 0);
            assert_eq!(arr_temp((b as *mut u8).sub(16) as *mut c_void), 0);
            same(
                "row27",
                &dump_map(a, MapShape::bytes(16, 8)),
                &dump_map(b, MapShape::bytes(16, 8)),
            );
            hmfree(c, a, 16);
            hmfree(r, b, 16);
        }
    }
}

#[test]
fn e28_hmdel_key_not_found() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 128);
    unsafe {
        (c.rand_seed)(53);
        (r.rand_seed)(53);
        let shape = MapShape::bytes(16, 8);
        let mut pc: *mut c_void = std::ptr::null_mut();
        let mut pr: *mut c_void = std::ptr::null_mut();
        let present = distinct_keys(&mut rng, 8, 80);
        for (i, k) in present.iter().enumerate() {
            let mut kk = k.clone();
            pc = (c.hmput_key)(pc, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
            pr = (r.hmput_key)(pr, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
            let t = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
            fill_value(pc, shape, (t + 1) as usize, i as u64);
            fill_value(pr, shape, (t + 1) as usize, i as u64);
        }
        let len_before = hmlen(pc, 16);
        for i in 0..1000 {
            let mut k = rng.bytes(8);
            if present.contains(&k) {
                continue;
            }
            let kp = k.as_mut_ptr() as *mut c_void;
            let a = (c.hmdel_key)(pc, 16, kp, 8, 0, HM_BINARY);
            let b = (r.hmdel_key)(pr, 16, kp, 8, 0, HM_BINARY);
            assert_eq!(a, pc);
            assert_eq!(b, pr);
            assert_eq!(
                arr_temp((a as *mut u8).sub(16) as *mut c_void),
                0,
                "miss #{i} must report temp == 0"
            );
            assert_eq!(arr_temp((b as *mut u8).sub(16) as *mut c_void), 0);
            assert_eq!(hmlen(pc, 16), len_before);
            same(&format!("row28 #{i}"), &dump_map(pc, shape), &dump_map(pr, shape));
        }
        hmfree(c, pc, 16);
        hmfree(r, pr, 16);
    }
}

#[test]
fn e29_e34_e35_hmdel_found_and_rebuilds() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 129);
    unsafe {
        (c.rand_seed)(59);
        (r.rand_seed)(59);
        let shape = MapShape::bytes(16, 8);
        let mut pc: *mut c_void = std::ptr::null_mut();
        let mut pr: *mut c_void = std::ptr::null_mut();
        let keys = distinct_keys(&mut rng, 8, 400);
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            pc = (c.hmput_key)(pc, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
            pr = (r.hmput_key)(pr, 16, kk.as_mut_ptr() as *mut c_void, 8, HM_BINARY);
            let t = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
            fill_value(pc, shape, (t + 1) as usize, i as u64);
            fill_value(pr, shape, (t + 1) as usize, i as u64);
        }
        let mut shrinks = 0;
        let mut rebuilds = 0;
        let mut prev_sc = {
            let t = arr_table((pc as *mut u8).sub(16) as *mut c_void);
            std::ptr::read_unaligned(t.add(hi::SLOT_COUNT) as *const usize)
        };
        let mut prev_tomb = 0usize;
        for (i, k) in keys.iter().enumerate() {
            let mut kk = k.clone();
            let kp = kk.as_mut_ptr() as *mut c_void;
            let before = hmlen(pc, 16);
            let a = (c.hmdel_key)(pc, 16, kp, 8, 0, HM_BINARY);
            let b = (r.hmdel_key)(pr, 16, kp, 8, 0, HM_BINARY);
            pc = a;
            pr = b;
            assert_eq!(
                arr_temp((pc as *mut u8).sub(16) as *mut c_void),
                1,
                "delete #{i} must report temp == 1"
            );
            assert_eq!(arr_temp((pr as *mut u8).sub(16) as *mut c_void), 1);
            assert_eq!(hmlen(pc, 16), before - 1);
            same(&format!("row29 #{i}"), &dump_map(pc, shape), &dump_map(pr, shape));
            let t = arr_table((pc as *mut u8).sub(16) as *mut c_void);
            let sc = std::ptr::read_unaligned(t.add(hi::SLOT_COUNT) as *const usize);
            let tomb = std::ptr::read_unaligned(t.add(hi::TOMBSTONE_COUNT) as *const usize);
            if sc < prev_sc {
                shrinks += 1;
            }
            if sc == prev_sc && tomb == 0 && prev_tomb > 0 {
                rebuilds += 1;
            }
            prev_sc = sc;
            prev_tomb = tomb;
        }
        assert!(shrinks > 0, "row34 (shrink) never taken");
        assert!(rebuilds > 0, "row35 (same-size rebuild) never taken");
        hmfree(c, pc, 16);
        hmfree(r, pr, 16);
    }
}

#[test]
fn e36_strdup_free_only_for_mode_exactly_one() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 136);
    unsafe {
        // mode == 1 frees the strdup'ed key; mode == 2 hashes/compares as a
        // string but does NOT free.  Delete only the *last* element so the
        // move-down + re-find path (which would reinterpret bytes) is skipped.
        for (mode, shmode) in [
            (HM_STRING, SH_STRDUP),
            (2 as c_int, SH_STRDUP),
            (HM_STRING, SH_DEFAULT),
            (2 as c_int, SH_DEFAULT),
            (7 as c_int, SH_ARENA),
        ] {
            (c.rand_seed)(61);
            (r.rand_seed)(61);
            let shape = MapShape::strp(16);
            let mut pc = (c.shmode_func)(16, shmode);
            let mut pr = (r.shmode_func)(16, shmode);
            let keys: Vec<CString> = distinct_strings(&mut rng, 30);
            for (i, s) in keys.iter().enumerate() {
                pc = (c.hmput_key)(pc, 16, s.as_ptr() as *mut c_void, 8, mode);
                pr = (r.hmput_key)(pr, 16, s.as_ptr() as *mut c_void, 8, mode);
                let t = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                fill_value(pc, shape, (t + 1) as usize, i as u64);
                fill_value(pr, shape, (t + 1) as usize, i as u64);
            }
            for i in (0..keys.len()).rev() {
                let kp = keys[i].as_ptr() as *mut c_void;
                pc = (c.hmdel_key)(pc, 16, kp, 8, 0, mode);
                pr = (r.hmdel_key)(pr, 16, kp, 8, 0, mode);
                let tc = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                let tr = arr_temp((pr as *mut u8).sub(16) as *mut c_void);
                assert_eq!(tc, tr, "mode={mode} sh={shmode} del {i} temp");
                assert_eq!(tc, 1, "mode={mode} sh={shmode}: last-element delete");
                same(
                    &format!("e36 mode={mode} sh={shmode} del {i}"),
                    &dump_map(pc, shape),
                    &dump_map(pr, shape),
                );
            }
            hmfree(c, pc, 16);
            hmfree(r, pr, 16);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 37..39 — stbds_hmfree_func
// ---------------------------------------------------------------------------

#[test]
fn e37_hmfree_null_is_noop() {
    let (c, r, _g) = both();
    unsafe {
        for elemsize in [0usize, 1, 8, 16, 40] {
            (c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

#[test]
fn e38_e39_hmfree_paths() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 138);
    unsafe {
        // row 38: hash_table == NULL
        for elemsize in [8usize, 16, 40] {
            let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            (c.hmfree_func)(a, elemsize);
            (r.hmfree_func)(b, elemsize);
        }
        // row 39: STRDUP frees elements 1..length (element 0 skipped)
        for n in [0usize, 1, 2, 8, 100] {
            (c.rand_seed)(67);
            (r.rand_seed)(67);
            let mut pc = (c.shmode_func)(16, SH_STRDUP);
            let mut pr = (r.shmode_func)(16, SH_STRDUP);
            for (i, s) in distinct_strings(&mut rng, n).iter().enumerate() {
                pc = (c.hmput_key)(pc, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
                pr = (r.hmput_key)(pr, 16, s.as_ptr() as *mut c_void, 8, HM_STRING);
                let t = arr_temp((pc as *mut u8).sub(16) as *mut c_void);
                fill_value(pc, MapShape::strp(16), (t + 1) as usize, i as u64);
                fill_value(pr, MapShape::strp(16), (t + 1) as usize, i as u64);
            }
            assert_eq!(hmlen(pc, 16) as usize, n);
            assert_eq!(hmlen(pr, 16) as usize, n);
            hmfree(c, pc, 16);
            hmfree(r, pr, 16);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 40..47 — the string arena
// ---------------------------------------------------------------------------

#[test]
fn e40_to_e44_e46_stralloc_paths() {
    let (c, r, _g) = both();
    unsafe {
        // row 46: empty string
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let empty = CString::new("").unwrap();
        let mut seen: Vec<*mut c_char> = Vec::new();
        for i in 0..600 {
            let pc = (c.stralloc)(&mut ca, empty.as_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ra, empty.as_ptr() as *mut c_char);
            assert_eq!(*pc, 0, "row46 C");
            assert_eq!(*pr, 0, "row46 R");
            assert!(!seen.contains(&pc), "row46: pointer #{i} reused");
            seen.push(pc);
            same("row46 arena", &dump_arena(&ca), &dump_arena(&ra));
        }
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);

        // rows 40..43: each branch, in order, with the arena state compared
        let cases: Vec<(&str, usize)> = vec![
            ("row41 fresh block", 100),   // len <= blocksize, remaining == 0
            ("row40 carve", 50),          // len <= remaining
            ("row42 oversized w/ head", 5000), // len > blocksize, storage != NULL
            ("row40 carve again", 20),
            ("row41 new block", 400),
        ];
        for (label, len) in cases {
            let s = CString::new(vec![b'z'; len]).unwrap();
            let pc = (c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            same(
                &format!("{label}: content"),
                std::ffi::CStr::from_ptr(pc).to_bytes(),
                std::ffi::CStr::from_ptr(pr).to_bytes(),
            );
            same(&format!("{label}: arena"), &dump_arena(&ca), &dump_arena(&ra));
        }
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);

        // row 43: first string oversized, storage == NULL -> remaining = 0
        let big = CString::new(vec![b'Y'; 4000]).unwrap();
        let pc = (c.stralloc)(&mut ca, big.as_ptr() as *mut c_char);
        let pr = (r.stralloc)(&mut ra, big.as_ptr() as *mut c_char);
        assert_eq!(ca.remaining, 0, "row43 C remaining");
        assert_eq!(ra.remaining, 0, "row43 R remaining");
        same(
            "row43 content",
            std::ffi::CStr::from_ptr(pc).to_bytes(),
            std::ffi::CStr::from_ptr(pr).to_bytes(),
        );
        same("row43 arena", &dump_arena(&ca), &dump_arena(&ra));
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);

        // row 44: `block` saturates
        for _ in 0..40 {
            let n = 512usize << ((ca.block >> 1).min(11));
            let s = CString::new(vec![b'w'; n.min(1 << 20)]).unwrap();
            let _ = (c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            let _ = (r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            same("row44 arena", &dump_arena(&ca), &dump_arena(&ra));
        }
        assert_eq!(ca.block, ra.block, "row44 block counter");
        assert!(ca.block <= 22, "row44: block must saturate, got {}", ca.block);
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        same("row44 after reset", &dump_arena(&ca), &dump_arena(&ra));
    }
}

#[test]
fn e47_strreset_on_empty_arena() {
    let (c, r, _g) = both();
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        for i in 0..20 {
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            assert!(ca.storage.is_null() && ra.storage.is_null());
            assert_eq!(ca.remaining, 0);
            assert_eq!(ra.remaining, 0);
            assert_eq!(ca.block, 0);
            assert_eq!(ra.block, 0);
            assert_eq!(ca.mode, 0);
            assert_eq!(ra.mode, 0);
            same(&format!("row47 #{i}"), &dump_arena(&ca), &dump_arena(&ra));
        }
        // a non-zero mode must also be cleared
        ca.mode = 3;
        ra.mode = 3;
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        assert_eq!(ca.mode, 0);
        assert_eq!(ra.mode, 0);
    }
}

// ---------------------------------------------------------------------------
// rows 48..51 — hash-function edge shapes
// ---------------------------------------------------------------------------

#[test]
fn e48_hash_bytes_len_zero() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 148);
    unsafe {
        let mut b = [0xAAu8; 32];
        for _ in 0..3000 {
            let seed = rng.next_u64() as usize;
            let hc = (c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, seed);
            let hr = (r.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(hc, hr, "hash_bytes(len=0, seed={seed:#x})");
        }
        // len 0 must not read the buffer: a NULL pointer is fine
        let hc = (c.hash_bytes)(std::ptr::null_mut(), 0, 12345);
        let hr = (r.hash_bytes)(std::ptr::null_mut(), 0, 12345);
        assert_eq!(hc, hr);
    }
}

#[test]
fn e49_hash_bytes_sign_extension_tails() {
    let (c, r, _g) = both();
    unsafe {
        // exhaustively: every tail length, every byte position, every high value
        for len in 1..=8usize {
            for pos in 0..len {
                for v in [0x80u8, 0x81, 0xfe, 0xff, 0x7f, 0x01] {
                    let mut b = vec![0u8; 8];
                    b[pos] = v;
                    for seed in [0usize, 1, usize::MAX, 0x31415926] {
                        let hc = (c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                        let hr = (r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                        assert_eq!(hc, hr, "len={len} pos={pos} v={v:#x} seed={seed:#x}");
                    }
                }
            }
        }
        // exhaustive single-byte inputs
        for v in 0..=255u8 {
            let mut b = [v];
            for seed in [0usize, 0x31415926, usize::MAX] {
                let hc = (c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 1, seed);
                let hr = (r.hash_bytes)(b.as_mut_ptr() as *mut c_void, 1, seed);
                assert_eq!(hc, hr, "single byte {v:#x}");
            }
        }
    }
}

#[test]
fn e50_hash_string_empty() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 150);
    unsafe {
        let e = CString::new("").unwrap();
        for _ in 0..3000 {
            let seed = rng.next_u64() as usize;
            let hc = (c.hash_string)(e.as_ptr() as *mut c_char, seed);
            let hr = (r.hash_string)(e.as_ptr() as *mut c_char, seed);
            assert_eq!(hc, hr, "hash_string(\"\", {seed:#x})");
        }
    }
}

#[test]
fn e51_hash_string_high_bit_bytes() {
    let (c, r, _g) = both();
    unsafe {
        for v in 1..=255u8 {
            for len in [1usize, 2, 7, 8, 9, 33] {
                let s = CString::new(vec![v; len]).unwrap();
                for seed in [0usize, 1, usize::MAX, 0x31415926] {
                    let hc = (c.hash_string)(s.as_ptr() as *mut c_char, seed);
                    let hr = (r.hash_string)(s.as_ptr() as *mut c_char, seed);
                    assert_eq!(hc, hr, "byte={v:#x} len={len} seed={seed:#x}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 53, 54 — the driver entry points with out-of-range arguments
// ---------------------------------------------------------------------------

#[test]
fn e53_strkey_negative_and_extremes() {
    let (c, r, _g) = both();
    unsafe {
        for n in [
            0,
            -1,
            -9,
            -10,
            -99,
            -100,
            -1000,
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
        ] {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            let sc = std::ffi::CStr::from_ptr(pc).to_bytes().to_vec();
            let sr = std::ffi::CStr::from_ptr(pr).to_bytes().to_vec();
            same(&format!("strkey({n})"), &sc, &sr);
            assert_eq!(sc, format!("test_{n}").into_bytes());
            assert!(sc.len() < 256, "strkey must fit the 256-byte buffer");
        }
    }
}

#[test]
fn e54_str_put_non_positive() {
    let (c, r, _g) = both();
    for num in [0, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
        unsafe {
            (c.rand_seed)(0x31415926);
            (r.rand_seed)(0x31415926);
        }
        let oc = capture_stdout(&format!("ec{num}"), || unsafe { (c.str_put)(num) });
        let or = capture_stdout(&format!("er{num}"), || unsafe { (r.str_put)(num) });
        same(&format!("str_put({num})"), &oc, &or);
        assert_eq!(oc, format!("a {num}\n").into_bytes());
    }
}
