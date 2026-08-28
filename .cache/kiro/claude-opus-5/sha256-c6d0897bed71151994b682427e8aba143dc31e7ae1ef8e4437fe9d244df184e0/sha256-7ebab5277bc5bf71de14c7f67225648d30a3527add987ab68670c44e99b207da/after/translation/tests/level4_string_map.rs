//! Level 4: string-keyed maps. Covers all three `string.mode` values that
//! `stbds_hmput_key` can be in - `STBDS_SH_DEFAULT` (borrowed key pointer,
//! selected implicitly by `shput` on a fresh map), `STBDS_SH_STRDUP` and
//! `STBDS_SH_ARENA` (both selected up-front via `stbds_shmode_func`).

mod common;

use common::*;
use std::ffi::{c_char, c_void};

const ES: usize = 16; // sizeof(struct { char *key; int value; })
const KS: usize = 8; // sizeof(t->key)

unsafe fn reseed(seed: usize) {
    let libs = libs();
    libs.c.rand_seed(seed);
    libs.rs.rand_seed(seed);
}

/// `stbds_temp_key(t-1)` == `*(char **) stbds_header(t-1)->hash_table`
unsafe fn temp_key_str(t: *mut u8) -> Option<Vec<u8>> {
    let raw = t.sub(ES);
    let h = (raw as *mut ArrHeader).offset(-1);
    let ht = (*h).hash_table as *mut *mut c_char;
    if ht.is_null() {
        return None;
    }
    read_cstr(*ht)
}

unsafe fn map_len(t: *mut u8) -> usize {
    if t.is_null() {
        return 0;
    }
    (*((t.sub(ES)) as *mut ArrHeader).offset(-1)).length
}

/// Pool of NUL-terminated keys that outlives the maps (required by
/// `STBDS_SH_DEFAULT`, which stores the caller's pointer verbatim).
struct Keys(Vec<Vec<u8>>);

impl Keys {
    fn new(n: usize) -> Self {
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let mut b = format!("test_{}", i).into_bytes();
            b.push(0);
            v.push(b);
        }
        Keys(v)
    }
    fn with_names(names: &[&str]) -> Self {
        Keys(
            names
                .iter()
                .map(|s| {
                    let mut b = s.as_bytes().to_vec();
                    b.push(0);
                    b
                })
                .collect(),
        )
    }
    fn ptr(&mut self, i: usize) -> *mut c_char {
        self.0[i].as_mut_ptr() as *mut c_char
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

/// Create the pair of maps for a given string mode.
/// `None` means "start from NULL", which makes `hmput_key` pick SH_DEFAULT.
unsafe fn make_pair(mode: Option<i32>) -> (*mut u8, *mut u8) {
    let libs = libs();
    match mode {
        None => (std::ptr::null_mut(), std::ptr::null_mut()),
        Some(m) => (libs.c.shmode_func(ES, m), libs.rs.shmode_func(ES, m)),
    }
}

fn mode_name(m: Option<i32>) -> &'static str {
    match m {
        None => "implicit SH_DEFAULT",
        Some(SH_NONE) => "SH_NONE",
        Some(SH_DEFAULT) => "SH_DEFAULT",
        Some(SH_STRDUP) => "SH_STRDUP",
        Some(SH_ARENA) => "SH_ARENA",
        _ => "?",
    }
}

#[test]
fn string_map_inserts_all_modes() {
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::StrKV;

    for mode in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        let mut keys = Keys::new(500);
        unsafe {
            reseed(0x3141_5926);
            let (mut ct, mut rt) = make_pair(mode);
            assert_eq!(
                snap_hm(ct, fmt),
                snap_hm(rt, fmt),
                "{}: initial state",
                mode_name(mode)
            );

            for i in 0..keys.len() {
                let before = map_len(ct);
                let k = keys.ptr(i);
                ct = shput(&libs.c, ct, k, i as i32);
                rt = shput(&libs.rs, rt, k, i as i32);
                assert_eq!(
                    snap_hm(ct, fmt),
                    snap_hm(rt, fmt),
                    "{}: shput #{i}",
                    mode_name(mode)
                );
                if map_len(ct) > before {
                    assert_eq!(
                        temp_key_str(ct),
                        temp_key_str(rt),
                        "{}: temp_key after new insert #{i}",
                        mode_name(mode)
                    );
                }
            }

            // Lookups: present and absent.
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                let (c2, ci) = shgeti(&libs.c, ct, k);
                let (r2, ri) = shgeti(&libs.rs, rt, k);
                ct = c2;
                rt = r2;
                assert_eq!(ci, ri, "{}: shgeti #{i}", mode_name(mode));
                assert!(ci >= 0, "{}: key #{i} lost", mode_name(mode));
            }
            let mut absent = Keys::with_names(&["", "nope", "test_", "test_99999", "TEST_1"]);
            for i in 0..absent.len() {
                let k = absent.ptr(i);
                let (c2, ci) = shgeti(&libs.c, ct, k);
                let (r2, ri) = shgeti(&libs.rs, rt, k);
                ct = c2;
                rt = r2;
                assert_eq!(ci, ri, "{}: absent shgeti #{i}", mode_name(mode));
                assert_eq!(ci, -1);
            }

            libs.c.hmfree_func(ct.sub(ES) as *mut c_void, ES);
            libs.rs.hmfree_func(rt.sub(ES) as *mut c_void, ES);
        }
    }
}

#[test]
fn string_map_overwrite_existing_all_modes() {
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::StrKV;

    for mode in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        let mut keys = Keys::new(80);
        unsafe {
            reseed(0xFACE);
            let (mut ct, mut rt) = make_pair(mode);
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                ct = shput(&libs.c, ct, k, i as i32);
                rt = shput(&libs.rs, rt, k, i as i32);
            }
            // Second pass hits the "key already present" branch, which in
            // string mode also refreshes `temp_key` from the stored pointer.
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                ct = shput(&libs.c, ct, k, 1000 + i as i32);
                rt = shput(&libs.rs, rt, k, 1000 + i as i32);
                assert_eq!(
                    temp_of(ct, ES),
                    temp_of(rt, ES),
                    "{}: overwrite index #{i}",
                    mode_name(mode)
                );
                assert_eq!(
                    snap_hm(ct, fmt),
                    snap_hm(rt, fmt),
                    "{}: overwrite #{i}",
                    mode_name(mode)
                );
            }
            libs.c.hmfree_func(ct.sub(ES) as *mut c_void, ES);
            libs.rs.hmfree_func(rt.sub(ES) as *mut c_void, ES);
        }
    }
}

#[test]
fn string_map_deletes_all_modes() {
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::StrKV;

    for mode in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        for order in ["forward", "backward", "strided"] {
            let mut keys = Keys::new(200);
            unsafe {
                reseed(0x5150);
                let (mut ct, mut rt) = make_pair(mode);
                for i in 0..keys.len() {
                    let k = keys.ptr(i);
                    ct = shput(&libs.c, ct, k, i as i32);
                    rt = shput(&libs.rs, rt, k, i as i32);
                }
                assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt));

                let idx: Vec<usize> = match order {
                    "forward" => (0..keys.len()).collect(),
                    "backward" => (0..keys.len()).rev().collect(),
                    _ => {
                        let mut v = Vec::new();
                        for s in 0..5 {
                            for i in (s..keys.len()).step_by(5) {
                                v.push(i);
                            }
                        }
                        v
                    }
                };

                for (step, i) in idx.iter().enumerate() {
                    let k = keys.ptr(*i);
                    let (c2, cr) = shdel(&libs.c, ct, k);
                    let (r2, rr) = shdel(&libs.rs, rt, k);
                    ct = c2;
                    rt = r2;
                    assert_eq!(
                        cr,
                        rr,
                        "{} {order}: shdel result step {step} key #{i}",
                        mode_name(mode)
                    );
                    assert_eq!(
                        snap_hm(ct, fmt),
                        snap_hm(rt, fmt),
                        "{} {order}: state after deleting #{i} (step {step})",
                        mode_name(mode)
                    );
                }
                // Absent deletes.
                let mut absent = Keys::with_names(&["zzz", "", "test_0"]);
                for i in 0..absent.len() {
                    let k = absent.ptr(i);
                    let (c2, cr) = shdel(&libs.c, ct, k);
                    let (r2, rr) = shdel(&libs.rs, rt, k);
                    ct = c2;
                    rt = r2;
                    assert_eq!(cr, rr, "{} {order}: absent shdel", mode_name(mode));
                    assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt));
                }
                libs.c.hmfree_func(ct.sub(ES) as *mut c_void, ES);
                libs.rs.hmfree_func(rt.sub(ES) as *mut c_void, ES);
            }
        }
    }
}

#[test]
fn string_map_random_mix_all_modes() {
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::StrKV;

    for mode in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        // A small key space forces heavy reuse, tombstone churn, shrink and
        // rebuild, plus re-insertion of previously freed strdup'd keys.
        let mut keys = Keys::new(60);
        unsafe {
            reseed(0xDEFACED);
            let (mut ct, mut rt) = make_pair(mode);
            let mut rng = Rng::new(0x1357_9BDF);
            for step in 0..2500 {
                let i = rng.below(keys.len() as u64) as usize;
                let k = keys.ptr(i);
                match rng.below(10) {
                    0..=4 => {
                        let v = rng.next_i32();
                        ct = shput(&libs.c, ct, k, v);
                        rt = shput(&libs.rs, rt, k, v);
                    }
                    5..=7 => {
                        let (c2, ci) = shgeti(&libs.c, ct, k);
                        let (r2, ri) = shgeti(&libs.rs, rt, k);
                        ct = c2;
                        rt = r2;
                        assert_eq!(ci, ri, "{} step {step}: get #{i}", mode_name(mode));
                    }
                    _ => {
                        let (c2, cr) = shdel(&libs.c, ct, k);
                        let (r2, rr) = shdel(&libs.rs, rt, k);
                        ct = c2;
                        rt = r2;
                        assert_eq!(cr, rr, "{} step {step}: del #{i}", mode_name(mode));
                    }
                }
                assert_eq!(
                    snap_hm(ct, fmt),
                    snap_hm(rt, fmt),
                    "{} step {step}",
                    mode_name(mode)
                );
            }
            libs.c.hmfree_func(ct.sub(ES) as *mut c_void, ES);
            libs.rs.hmfree_func(rt.sub(ES) as *mut c_void, ES);
        }
    }
}

#[test]
fn string_map_long_and_odd_keys() {
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::StrKV;

    let mut names: Vec<String> = vec![
        String::new(),
        "a".into(),
        "aa".into(),
        "ab".into(),
        "ba".into(),
        "\u{7f}".into(),
        "0123456789abcdef".into(),
    ];
    // Keys long enough to spill an arena block, and near-identical prefixes.
    names.push("x".repeat(511));
    names.push("x".repeat(512));
    names.push("x".repeat(513));
    names.push("y".repeat(5000));
    for i in 0..50 {
        names.push(format!("prefix_{}{}", "z".repeat(i), i));
    }
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();

    for mode in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        let mut keys = Keys::with_names(&refs);
        unsafe {
            reseed(0x2468_ACE0);
            let (mut ct, mut rt) = make_pair(mode);
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                ct = shput(&libs.c, ct, k, i as i32);
                rt = shput(&libs.rs, rt, k, i as i32);
                assert_eq!(
                    snap_hm(ct, fmt),
                    snap_hm(rt, fmt),
                    "{}: odd-key shput #{i}",
                    mode_name(mode)
                );
            }
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                let (c2, ci) = shgeti(&libs.c, ct, k);
                let (r2, ri) = shgeti(&libs.rs, rt, k);
                ct = c2;
                rt = r2;
                assert_eq!(ci, ri, "{}: odd-key shgeti #{i}", mode_name(mode));
            }
            libs.c.hmfree_func(ct.sub(ES) as *mut c_void, ES);
            libs.rs.hmfree_func(rt.sub(ES) as *mut c_void, ES);
        }
    }
}

#[test]
fn shputs_uses_temp_key() {
    // `stbds_shputs` copies the whole struct and then rewrites `.key` from
    // `stbds_temp_key`, so the stored pointer must come from the library.
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::StrKV;

    for mode in [Some(SH_STRDUP), Some(SH_ARENA)] {
        let mut keys = Keys::new(120);
        unsafe {
            reseed(0x1010);
            let (mut ct, mut rt) = make_pair(mode);
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                for (lib, t) in [(&libs.c, &mut ct), (&libs.rs, &mut rt)] {
                    let nt = lib.hmput_key(*t as *mut c_void, ES, k as *mut c_void, KS, HM_STRING);
                    let temp = temp_of(nt, ES);
                    let slot = nt.offset(temp * ES as isize);
                    // (t)[temp] = s;  then  (t)[temp].key = temp_key
                    let tk = *((*(((nt.sub(ES)) as *mut ArrHeader).offset(-1))).hash_table
                        as *mut *mut c_char);
                    (slot as *mut *mut c_char).write_unaligned(k);
                    (slot.add(8) as *mut i32).write_unaligned(i as i32);
                    (slot as *mut *mut c_char).write_unaligned(tk);
                    *t = nt;
                }
                assert_eq!(
                    snap_hm(ct, fmt),
                    snap_hm(rt, fmt),
                    "{}: shputs #{i}",
                    mode_name(mode)
                );
            }
            libs.c.hmfree_func(ct.sub(ES) as *mut c_void, ES);
            libs.rs.hmfree_func(rt.sub(ES) as *mut c_void, ES);
        }
    }
}

#[test]
fn arena_mode_table_arena_state_matches() {
    // In SH_ARENA mode the keys live in the table's own `stbds_string_arena`,
    // so the arena bookkeeping inside `stbds_hash_index` must track exactly.
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::StrKV;
    let mut keys = Keys::new(400);
    unsafe {
        reseed(0xA0A0);
        let (mut ct, mut rt) = make_pair(Some(SH_ARENA));
        for i in 0..keys.len() {
            let k = keys.ptr(i);
            ct = shput(&libs.c, ct, k, i as i32);
            rt = shput(&libs.rs, rt, k, i as i32);
            let cs = snap_hm(ct, fmt);
            let rs = snap_hm(rt, fmt);
            assert_eq!(cs, rs, "arena shput #{i}");
            let ctab = cs.table.as_ref().unwrap();
            assert_eq!(ctab.string_mode, SH_ARENA as u8);
        }
        libs.c.hmfree_func(ct.sub(ES) as *mut c_void, ES);
        libs.rs.hmfree_func(rt.sub(ES) as *mut c_void, ES);
    }
}
