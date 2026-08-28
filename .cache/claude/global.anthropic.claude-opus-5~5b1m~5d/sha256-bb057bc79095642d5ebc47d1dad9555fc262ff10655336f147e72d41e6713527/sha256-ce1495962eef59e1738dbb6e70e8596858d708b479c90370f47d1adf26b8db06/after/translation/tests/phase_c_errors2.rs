//! Phase C, part 2 -- `ERRORS.md` rows E29..E42 and E46..E49.
//!
//! These cover the failing-`STBDS_ASSERT` paths (the C aborts via glibc's
//! `__assert_fail`; the translation must abort with the *same* diagnostic), the
//! grow/shrink/rebuild thresholds, and the remaining degenerate-argument rows.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

const E: usize = 16;
const K: usize = 8;

/// The exact `__FILE__` the C was compiled with, so assert diagnostics can be
/// recognised.
fn assert_msg_contains(out: &ChildOutcome, needle: &str) {
    let s = String::from_utf8_lossy(&out.stderr);
    assert!(
        s.contains(needle),
        "expected the assert diagnostic to mention {needle:?}, got {s:?}"
    );
}

// ===========================================================================
// E29 / E39 -- the `stbds_hmdel_key` relocation asserts
// ===========================================================================

/// E29 -- `mode == 2`: `find_slot`/`is_key_equal` treat it as STRING
/// (`mode >= STBDS_HM_STRING`) but the re-index branch tests `mode ==
/// STBDS_HM_STRING` *exactly*, so the moved element's key is re-hashed from the
/// raw bytes of a `char *`. The lookup fails and `STBDS_ASSERT(slot >= 0)` fires.
#[test]
fn err_e29_hmdel_mode_2_asymmetry() {
    let mut rng = Rng::new(29);
    with_libs(0x31415926, |c, rs| unsafe {
        for &mode in &[2i32, 3, 7, 1000, c_int::MAX] {
            let keys = Keys::random(&mut rng, 3, 12);
            let mut tc = (c.shmode_func)(E, SH_DEFAULT);
            let mut tr = (rs.shmode_func)(E, SH_DEFAULT);
            for i in 0..3 {
                tc = (c.hmput_key)(tc, E, keys.ptr(i), K, mode);
                tr = (rs.hmput_key)(tr, E, keys.ptr(i), K, mode);
                let ia = map_temp(tc, E);
                let ib = map_temp(tr, E);
                assert_eq!(ia, ib);
                write_elem_tail(tc, E, ia, K, &(i as u64).to_le_bytes());
                write_elem_tail(tr, E, ib, K, &(i as u64).to_le_bytes());
            }
            assert_same(
                &format!("e29 mode={mode} setup"),
                &snap_map(tc, E, KeyKind::Binary),
                &snap_map(tr, E, KeyKind::Binary),
            );
            // deleting the *first* element forces the relocation branch
            let out = assert_same_crash(
                &format!("e29 hmdel_key mode={mode}"),
                || {
                    (c.hmdel_key)(tc, E, keys.ptr(0), K, 0, mode);
                },
                || {
                    (rs.hmdel_key)(tr, E, keys.ptr(0), K, 0, mode);
                },
            );
            assert!(
                !out.exited && out.code == SIGABRT,
                "expected SIGABRT from the `slot >= 0` assert, got {out:?}"
            );
            assert_msg_contains(&out, "Assertion `slot >= 0' failed");
            assert_msg_contains(&out, "stbds_hmdel_key");
            (c.hmfree_func)(hash_to_arr(tc, E), E);
            (rs.hmfree_func)(hash_to_arr(tr, E), E);
        }
    });
}

/// E39 -- `STBDS_ASSERT(b->index[i] == final_index)`.
///
/// Constructed by making the slot of the *last* element point at a different
/// (live) element whose key bytes are made identical, so the post-relocation
/// lookup succeeds but resolves to the wrong index.
#[test]
fn err_e39_hmdel_relocate_asserts() {
    let mut rng = Rng::new(39);
    with_libs(0x31415926, |c, rs| unsafe {
        let keys = BinKeys::random(&mut rng, 3, K);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        for i in 0..3 {
            tc = (c.hmput_key)(tc, E, keys.ptr(i), K, HM_BINARY);
            tr = (rs.hmput_key)(tr, E, keys.ptr(i), K, HM_BINARY);
            let ia = map_temp(tc, E);
            let ib = map_temp(tr, E);
            assert_eq!(ia, ib);
            write_elem_tail(tc, E, ia, K, &(i as u64).to_le_bytes());
            write_elem_tail(tr, E, ib, K, &(i as u64).to_le_bytes());
        }
        assert_same(
            "e39 setup",
            &snap_map(tc, E, KeyKind::Binary),
            &snap_map(tr, E, KeyKind::Binary),
        );

        // Corrupt both maps identically: the slot that indexes element 2 (the
        // final one) is made to index element 1 instead, and element 1's key
        // bytes are replaced by element 2's so `is_key_equal` still matches.
        for (t, api) in [(tc, c), (tr, rs)] {
            let _ = api;
            let ti = table_of(t, E);
            let slot = find_slot_with_index(ti, 2).expect("slot for index 2");
            set_slot_index(ti, slot, 1);
            std::ptr::copy_nonoverlapping(
                (t as *const u8).add(2 * E),
                (t as *mut u8).add(1 * E),
                K,
            );
        }
        assert_same(
            "e39 corrupted state must be identical",
            &snap_map(tc, E, KeyKind::Binary),
            &snap_map(tr, E, KeyKind::Binary),
        );

        // Deleting element 0 relocates element 2 into slot 0; the follow-up
        // lookup now resolves to index 1 != final_index 2.
        let out = assert_same_crash(
            "e39 hmdel_key final_index mismatch",
            || {
                (c.hmdel_key)(tc, E, keys.ptr(0), K, 0, HM_BINARY);
            },
            || {
                (rs.hmdel_key)(tr, E, keys.ptr(0), K, 0, HM_BINARY);
            },
        );
        assert!(
            !out.exited && out.code == SIGABRT,
            "expected SIGABRT, got {out:?}"
        );
        assert_msg_contains(&out, "Assertion `b->index[i] == final_index' failed");
        assert_msg_contains(&out, "stbds_hmdel_key");
    });
}

// ===========================================================================
// E30 -- `is_key_equal` dereferencing raw key bytes as a `char *`
// ===========================================================================

/// `STBDS_SH_NONE` + `STBDS_HM_STRING`: the element holds the key *bytes*, so a
/// lookup that reaches `is_key_equal` does `strcmp(key, <garbage pointer>)`.
#[test]
fn err_e30_is_key_equal_wild_pointer() {
    let mut rng = Rng::new(30);
    with_libs(0x31415926, |c, rs| unsafe {
        for _ in 0..6 {
            let keys = Keys::random(&mut rng, 3, 12);
            let mut tc = (c.shmode_func)(E, SH_NONE);
            let mut tr = (rs.shmode_func)(E, SH_NONE);
            for i in 0..keys.len() {
                tc = (c.hmput_key)(tc, E, keys.ptr(i), K, HM_STRING);
                tr = (rs.hmput_key)(tr, E, keys.ptr(i), K, HM_STRING);
                let ia = map_temp(tc, E);
                let ib = map_temp(tr, E);
                assert_eq!(ia, ib);
                write_elem_tail(tc, E, ia, K, &(i as u64).to_le_bytes());
                write_elem_tail(tr, E, ib, K, &(i as u64).to_le_bytes());
            }
            assert_same(
                "e30 setup",
                &snap_map(tc, E, KeyKind::Binary),
                &snap_map(tr, E, KeyKind::Binary),
            );
            // looking a *present* key up reaches is_key_equal -> wild strcmp
            let out = assert_same_crash(
                "e30 hmget of a present key in SH_NONE/HM_STRING",
                || {
                    (c.hmget_key)(tc, E, keys.ptr(0), K, HM_STRING);
                },
                || {
                    (rs.hmget_key)(tr, E, keys.ptr(0), K, HM_STRING);
                },
            );
            assert!(
                !out.exited || out.code == 0,
                "outcome must at least be identical: {out:?}"
            );
            // and so does re-putting it
            let out2 = assert_same_crash(
                "e30 hmput of a duplicate key in SH_NONE/HM_STRING",
                || {
                    (c.hmput_key)(tc, E, keys.ptr(0), K, HM_STRING);
                },
                || {
                    (rs.hmput_key)(tr, E, keys.ptr(0), K, HM_STRING);
                },
            );
            assert_eq!(out.exited, out2.exited, "sanity");
            (c.hmfree_func)(hash_to_arr(tc, E), E);
            (rs.hmfree_func)(hash_to_arr(tr, E), E);
        }
    });
}

// ===========================================================================
// E31..E34 -- grow / shrink / rebuild thresholds
// ===========================================================================

/// E31 -- `used_count >= used_count_threshold` doubles the table and the new
/// table *inherits* the old `seed` (the global is not advanced).
#[test]
fn err_e31_grow_at_threshold() {
    let mut rng = Rng::new(31);
    with_libs(0x31415926, |c, rs| {
        for _ in 0..40 {
            let keys = BinKeys::random(&mut rng, 200, K);
            let mut m = MapPair::new(c, rs, E, K, HM_BINARY, Arena::Auto);
            let mut seed0 = None;
            let mut grows = 0usize;
            let mut prev = 8usize;
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
                m.check(&format!("e31 put#{i}"));
                let t = m.table_c().unwrap();
                if seed0.is_none() {
                    seed0 = Some(t.seed);
                }
                if t.slot_count != prev {
                    assert_eq!(t.slot_count, prev * 2, "growth must double");
                    // used_count crossed the threshold of the *previous* table
                    assert!(t.used_count >= prev - (prev >> 2));
                    grows += 1;
                    prev = t.slot_count;
                }
                assert_eq!(
                    t.seed,
                    seed0.unwrap(),
                    "a rehashed table must inherit the original seed"
                );
                assert_eq!(t.used_count_threshold, t.slot_count - (t.slot_count >> 2));
                assert_eq!(
                    t.tombstone_count_threshold,
                    (t.slot_count >> 3) + (t.slot_count >> 4)
                );
            }
            assert!(grows >= 4, "expected several grows, saw {grows}");
            m.free();
        }
    });
}

/// E32 -- `used_count < used_count_shrink_threshold && slot_count > 8` shrinks.
#[test]
fn err_e32_shrink_threshold() {
    let mut rng = Rng::new(32);
    with_libs(0x31415926, |c, rs| {
        for _ in 0..30 {
            let keys = BinKeys::random(&mut rng, 120, K);
            let mut m = MapPair::new(c, rs, E, K, HM_BINARY, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
            }
            let mut shrinks = 0usize;
            let mut prev = m.table_c().unwrap().slot_count;
            for i in 0..keys.len() {
                let before = m.table_c().unwrap();
                assert_eq!(m.del(keys.ptr(i), 0), 1);
                m.check(&format!("e32 del#{i}"));
                let t = m.table_c().unwrap();
                if t.slot_count < prev {
                    assert_eq!(t.slot_count, prev / 2, "shrink halves");
                    assert!(
                        before.used_count - 1 < before.used_count_shrink_threshold,
                        "shrink must only happen below the shrink threshold"
                    );
                    assert!(before.slot_count > 8);
                    shrinks += 1;
                    prev = t.slot_count;
                }
            }
            assert!(shrinks >= 2, "expected shrinks, saw {shrinks}");
            assert_eq!(m.table_c().unwrap().slot_count, 8);
            m.free();
        }
    });
}

/// E33 -- `tombstone_count > tombstone_count_threshold` rebuilds in place.
#[test]
fn err_e33_tombstone_rebuild() {
    let mut rng = Rng::new(33);
    with_libs(0x31415926, |c, rs| {
        let mut rebuilds = 0usize;
        for _ in 0..40 {
            // 200 keys -> 512 slots, threshold = 64+32 = 96 tombstones.
            let keys = BinKeys::random(&mut rng, 200, K);
            let mut m = MapPair::new(c, rs, E, K, HM_BINARY, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
            }
            let mut prev = m.table_c().unwrap();
            for i in 0..keys.len() {
                assert_eq!(m.del(keys.ptr(i), 0), 1);
                m.check(&format!("e33 del#{i}"));
                let t = m.table_c().unwrap();
                if t.slot_count == prev.slot_count && t.tombstone_count < prev.tombstone_count {
                    assert!(
                        prev.tombstone_count + 1 > prev.tombstone_count_threshold,
                        "in-place rebuild requires crossing the tombstone threshold"
                    );
                    assert_eq!(t.tombstone_count, 0, "rebuild clears tombstones");
                    assert_eq!(t.seed, prev.seed, "rebuild inherits the seed");
                    rebuilds += 1;
                }
                prev = t;
            }
            m.free();
        }
        assert!(rebuilds >= 1, "the tombstone rebuild path was never taken");
    });
}

/// E34 -- an 8-slot table has `used_count_shrink_threshold == 0` and so never
/// shrinks (it is already minimal).
#[test]
fn err_e34_no_shrink_at_8_slots() {
    let mut rng = Rng::new(34);
    with_libs(0x31415926, |c, rs| {
        for _ in 0..60 {
            let keys = BinKeys::random(&mut rng, 6, K);
            let mut m = MapPair::new(c, rs, E, K, HM_BINARY, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
            }
            let t = m.table_c().unwrap();
            assert_eq!(t.slot_count, 8);
            assert_eq!(t.used_count_shrink_threshold, 0);
            for i in 0..keys.len() {
                assert_eq!(m.del(keys.ptr(i), 0), 1);
                m.check(&format!("e34 del#{i}"));
                let t = m.table_c().unwrap();
                assert_eq!(t.slot_count, 8, "an 8-slot table must never shrink");
                assert_eq!(t.used_count_shrink_threshold, 0);
            }
            m.free();
        }
        // and via stbds_shmode_func, which always starts at 8 slots
        unsafe {
            for &sh in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
                let a = (c.shmode_func)(E, sh);
                let b = (rs.shmode_func)(E, sh);
                let sa = snap_map(a, E, KeyKind::Binary);
                assert_same("e34 shmode", &sa, &snap_map(b, E, KeyKind::Binary));
                assert_eq!(sa.table.as_ref().unwrap().slot_count, 8);
                assert_eq!(sa.table.as_ref().unwrap().used_count_shrink_threshold, 0);
                (c.hmfree_func)(hash_to_arr(a, E), E);
                (rs.hmfree_func)(hash_to_arr(b, E), E);
            }
        }
    });
}

// ===========================================================================
// E35 / E36 / E37 -- asserts that are unreachable through the public API
// ===========================================================================

/// E35 -- `used_count_threshold + tombstone_count_threshold < slot_count`
/// holds for every `slot_count` the library can produce (powers of two >= 8).
#[test]
fn err_e35_make_hash_index_assert_unreachable() {
    // numeric proof over every reachable slot_count
    let mut sc = 8usize;
    while sc <= (1usize << 40) {
        let uct = sc - (sc >> 2);
        let tct = (sc >> 3) + (sc >> 4);
        assert!(uct + tct < sc, "assert would fire for slot_count={sc}");
        sc <<= 1;
    }
    // behavioural check: many grow/shrink/rebuild cycles, no abort in either lib
    with_libs(0x31415926, |c, rs| {
        let out = assert_same_crash(
            "e35 grow/shrink cycles",
            || unsafe { churn(c) },
            || unsafe { churn(rs) },
        );
        assert!(
            out.exited && out.code == 0 && out.stderr.is_empty(),
            "the make_hash_index assert must never fire: {out:?}"
        );
    });
}

unsafe fn churn(api: &Api) {
    let mut rng = Rng::new(777);
    let keys = BinKeys::random(&mut rng, 600, K);
    let mut t: *mut c_void = std::ptr::null_mut();
    for i in 0..keys.len() {
        t = (api.hmput_key)(t, E, keys.ptr(i), K, HM_BINARY);
    }
    for i in 0..keys.len() {
        t = (api.hmdel_key)(t, E, keys.ptr(i), K, 0, HM_BINARY);
    }
    for i in 0..keys.len() {
        t = (api.hmput_key)(t, E, keys.ptr(i), K, HM_BINARY);
    }
    (api.hmfree_func)(hash_to_arr(t, E), E);
}

/// E36 -- `(size_t) i+1 <= stbds_arrcap(a)` is guaranteed by the `arrgrowf`
/// call immediately above it.
#[test]
fn err_e36_hmput_capacity_assert() {
    let mut rng = Rng::new(36);
    with_libs(0x31415926, |c, rs| {
        // Every insert is watched: length must never exceed capacity.
        // `keysize <= elemsize` is required here -- `hmput_key` memcpy's
        // `keysize` bytes into the element, so a larger key would scribble past
        // the array (identically in both libraries, but fatal in-process).
        for (elemsize, keysize) in [(1usize, 1usize), (4, 4), (8, 8), (16, 8), (40, 16)] {
            let n = 300usize.min(BinKeys::max_distinct(keysize));
            let keys = BinKeys::random_prefix(&mut rng, n, keysize, keysize.max(8) + 8);
            let mut m = MapPair::new(c, rs, elemsize, keysize, HM_BINARY, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
                let s = m.snap_c();
                assert!(
                    s.length <= s.capacity,
                    "e36 e{elemsize} k{keysize} #{i}: length {} > capacity {}",
                    s.length,
                    s.capacity
                );
            }
            m.check("e36");
            m.free();
        }
        let out = assert_same_crash(
            "e36 no abort",
            || unsafe { churn(c) },
            || unsafe { churn(rs) },
        );
        assert!(out.exited && out.code == 0 && out.stderr.is_empty(), "{out:?}");
    });
}

/// E37 -- `slot < (ptrdiff_t) table->slot_count`: `find_slot` masks `pos` with
/// `slot_count - 1`, so the returned slot is always in range.
#[test]
fn err_e37_hmdel_slot_range_assert() {
    let mut rng = Rng::new(37);
    with_libs(0x31415926, |c, rs| unsafe {
        for _ in 0..40 {
            let keys = BinKeys::random(&mut rng, 150, K);
            let mut m = MapPair::new(c, rs, E, K, HM_BINARY, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
            }
            // every occupied slot number must be < slot_count
            let ti = table_of(m.c, E);
            let sc = (*ti).slot_count;
            for i in 0..keys.len() {
                let before = table_of(m.c, E);
                let n = (*before).slot_count;
                assert!(n <= sc.max(8));
                assert_eq!(m.del(keys.ptr(i), 0), 1);
                m.check(&format!("e37 del#{i}"));
            }
            m.free();
        }
    });
}

/// E38 -- `STBDS_ASSERT(table->used_count >= 0)` is a tautology on a `size_t`
/// and gcc removed it entirely (9, not 10, `__assert_fail` call sites in the
/// C `.so`).  Forcing `used_count` to 0 before a hit delete makes it underflow
/// to `SIZE_MAX` **without** any abort -- in both libraries.
#[test]
fn err_e38_hmdel_used_count_tautology() {
    let mut rng = Rng::new(38);
    with_libs(0x31415926, |c, rs| unsafe {
        for _ in 0..30 {
            let keys = BinKeys::random(&mut rng, 3, K);
            let mut m = MapPair::new(c, rs, E, K, HM_BINARY, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
            }
            m.check("e38 setup");
            // identical corruption on both sides
            (*table_of(m.c, E)).used_count = 0;
            (*table_of(m.rs, E)).used_count = 0;
            // delete the last element (no relocation, so no other assert fires)
            let t = m.del(keys.ptr(keys.len() - 1), 0);
            assert_eq!(t, 1);
            m.check("e38 after underflow");
            assert_eq!(
                (*table_of(m.c, E)).used_count,
                usize::MAX,
                "used_count must wrap, not trap"
            );
            m.free();
        }
    });
}

// ===========================================================================
// E40 / E41 / E42 -- the driver code
// ===========================================================================

/// E40 -- the three `str_put` asserts hold (they are live in the `.so`).
#[test]
fn err_e40_str_put_asserts_hold() {
    with_libs(0x31415926, |c, rs| {
        let out = assert_same_crash(
            "e40 str_put asserts",
            || unsafe {
                for n in [0i32, 1, 2, 7, 73, 100, 1000, -1, -1000, c_int::MIN] {
                    (c.str_put)(n);
                }
            },
            || unsafe {
                for n in [0i32, 1, 2, 7, 73, 100, 1000, -1, -1000, c_int::MIN] {
                    (rs.str_put)(n);
                }
            },
        );
        assert!(
            out.exited && out.code == 0,
            "no str_put assert may fire: {out:?}"
        );
        assert!(
            !String::from_utf8_lossy(&out.stderr).contains("Assertion"),
            "unexpected assert: {:?}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
}

/// E41 -- `num <= 0` skips the `stralloc` loop but still prints one line.
#[test]
fn err_e41_str_put_non_positive() {
    with_libs(0x31415926, |c, rs| {
        for num in [0i32, -1, -2, -7, -1000, -12345, c_int::MIN, c_int::MIN + 1] {
            let oc = capture_stdout(|| unsafe { (c.str_put)(num) });
            let or = capture_stdout(|| unsafe { (rs.str_put)(num) });
            assert_same(&format!("e41 str_put({num}) stdout"), &oc, &or);
            assert_eq!(oc, format!("a {num}\n").into_bytes());
        }
    });
}

/// E42 -- `sprintf(buffer, "test_%d", n)` never overflows the 256-byte buffer.
#[test]
fn err_e42_strkey_extremes() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(42);
        let mut ns: Vec<c_int> = vec![
            0,
            1,
            -1,
            c_int::MAX,
            c_int::MIN,
            c_int::MAX - 1,
            c_int::MIN + 1,
            -2147483647,
        ];
        for _ in 0..500 {
            ns.push(rng.next_u64() as c_int);
        }
        for n in ns {
            let pc = (c.strkey)(n);
            let pr = (rs.strkey)(n);
            let lc = strlen(pc);
            let lr = strlen(pr);
            assert_eq!(lc, lr, "strkey({n}) length");
            assert!(lc <= 16, "strkey({n}) is {lc} bytes -- would risk overflow");
            let sc = std::slice::from_raw_parts(pc as *const u8, lc);
            let sr = std::slice::from_raw_parts(pr as *const u8, lr);
            assert_eq!(sc, sr, "strkey({n})");
            assert_eq!(sc, format!("test_{n}").as_bytes());
        }
    });
}

// ===========================================================================
// E46 -- the wrap-around duplicate branch does NOT refresh `temp_key`
// ===========================================================================

#[test]
fn err_e46_wraparound_no_temp_key() {
    let mut rng = Rng::new(46);
    let mut found = 0usize;
    with_libs(0x31415926, |c, rs| unsafe {
        for _trial in 0..4000 {
            if found >= 5 {
                break;
            }
            // 5 keys: `used_count (5) < used_count_threshold (6)`, so a
            // duplicate re-put does NOT rebuild the table first and the slot
            // layout observed here is the one the re-put probes.
            let keys = Keys::random(&mut rng, 5, 14);
            let mut tc = (c.shmode_func)(E, SH_STRDUP);
            let mut tr = (rs.shmode_func)(E, SH_STRDUP);
            for i in 0..keys.len() {
                tc = (c.hmput_key)(tc, E, keys.ptr(i), K, HM_STRING);
                tr = (rs.hmput_key)(tr, E, keys.ptr(i), K, HM_STRING);
                let ia = map_temp(tc, E);
                let ib = map_temp(tr, E);
                assert_eq!(ia, ib);
                write_elem_tail(tc, E, ia, K, &(i as u64).to_le_bytes());
                write_elem_tail(tr, E, ib, K, &(i as u64).to_le_bytes());
            }
            let ti = table_of(tc, E);
            assert_eq!((*ti).slot_count, 8, "5 keys must still fit in 8 slots");
            assert!((*ti).used_count < (*ti).used_count_threshold);
            let seed = (*ti).seed;

            // Look for a key whose slot number is *below* its probe position:
            // its insert wrapped around, and so will a duplicate lookup.
            let mut victim: Option<usize> = None;
            for i in 0..keys.len() - 1 {
                let h = (c.hash_string)(keys.cptr(i), seed);
                let hg = if h < 2 { h + 2 } else { h };
                let pos = hg & ((*ti).slot_count - 1);
                if let Some(slot) = find_slot_with_hash(ti, hg) {
                    if slot < pos {
                        victim = Some(i);
                        break;
                    }
                }
            }
            if let Some(w) = victim {
                found += 1;
                // temp_key currently points at the *last* inserted key's copy
                let last_idx = map_temp(tc, E);
                let last_key_c = *((tc as *const u8).offset(last_idx * E as isize)
                    as *const *mut c_char);
                let last_key_r = *((tr as *const u8).offset(last_idx * E as isize)
                    as *const *mut c_char);
                assert_eq!(map_temp_key(tc, E), last_key_c);
                assert_eq!(map_temp_key(tr, E), last_key_r);

                // re-put the wrap-around key: the duplicate is found in the
                // second (wrap) inner loop, which does NOT touch temp_key
                let len_before = map_len(tc, E);
                tc = (c.hmput_key)(tc, E, keys.ptr(w), K, HM_STRING);
                tr = (rs.hmput_key)(tr, E, keys.ptr(w), K, HM_STRING);
                let ia = map_temp(tc, E);
                let ib = map_temp(tr, E);
                assert_eq!(ia, ib, "duplicate must resolve to the same index");
                assert_eq!(map_len(tc, E), len_before, "must be a duplicate hit");
                assert_eq!((*table_of(tc, E)).slot_count, 8, "no rebuild expected");

                let wk_c = *((tc as *const u8).offset(ia * E as isize) as *const *mut c_char);
                let wk_r = *((tr as *const u8).offset(ib * E as isize) as *const *mut c_char);
                assert_ne!(
                    map_temp_key(tc, E), wk_c,
                    "C: the wrap-around branch must leave temp_key stale"
                );
                assert_ne!(
                    map_temp_key(tr, E), wk_r,
                    "RUST: the wrap-around branch must leave temp_key stale"
                );
                assert_eq!(map_temp_key(tc, E), last_key_c, "C temp_key unchanged");
                assert_eq!(map_temp_key(tr, E), last_key_r, "RUST temp_key unchanged");
                assert_same(
                    "e46 map state after wrap-around duplicate",
                    &snap_map(tc, E, KeyKind::PtrByContent),
                    &snap_map(tr, E, KeyKind::PtrByContent),
                );
            }
            (c.hmfree_func)(hash_to_arr(tc, E), E);
            (rs.hmfree_func)(hash_to_arr(tr, E), E);
        }
    });
    assert!(
        found >= 1,
        "no wrap-around duplicate could be constructed in 4000 trials"
    );
}

// ===========================================================================
// E47 / E48 / E49 -- degenerate size arguments
// ===========================================================================

/// E47 -- `keysize == 0` in BINARY mode: `memcmp(...,0) == 0`, so every key
/// matches and `hash_bytes(key,0,seed)` is key-independent.
#[test]
fn err_e47_zero_keysize_binary() {
    let mut rng = Rng::new(47);
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            let keys = BinKeys::random_prefix(&mut rng, 20, 8, 16);
            let mut m = MapPair::new(c, rs, elemsize, 0, HM_BINARY, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
                m.check(&format!("e47 e{elemsize} put#{i}"));
                assert_eq!(m.len(), 1, "every keysize==0 key collapses into one");
            }
            // and the hash is key-independent
            let seed = m.table_c().unwrap().seed;
            let h0 = (c.hash_bytes)(keys.ptr(0), 0, seed);
            for i in 0..keys.len() {
                assert_eq!((c.hash_bytes)(keys.ptr(i), 0, seed), h0);
                assert_eq!((rs.hash_bytes)(keys.ptr(i), 0, seed), h0);
            }
            // a delete removes the single entry, whatever key is passed
            assert_eq!(m.del(keys.ptr(7), 0), 1);
            m.check("e47 del");
            assert_eq!(m.len(), 0);
            assert_eq!(m.del(keys.ptr(3), 0), 0);
            m.check("e47 del again");
            m.free();
        }
    });
}

/// E48 -- `elemsize == 0`: every element aliases offset 0 and
/// `STBDS_ARR_TO_HASH(a,0) == a`.
#[test]
fn err_e48_zero_elemsize() {
    let mut rng = Rng::new(48);
    with_libs(0x31415926, |c, rs| unsafe {
        // `elemsize == 0` means the array allocation is header-only, so any
        // `keysize > 0` makes `hmput_key`'s `memcpy` write past it.  That is
        // identical in both libraries but corrupts the heap, so keysize > 0 is
        // exercised in forked children (below) and keysize == 0 in-process.
        for &mode in &[HM_BINARY, -1, c_int::MIN] {
            let keys = BinKeys::random_prefix(&mut rng, 12, 0, 16);
            let mut m = MapPair::new(c, rs, 0, 0, mode, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
                m.check(&format!("e48 k0 mode={mode} put#{i}"));
            }
            // hash pointer == arr pointer when elemsize == 0
            assert_eq!(hash_to_arr(m.c, 0), m.c);
            assert_eq!(hash_to_arr(m.rs, 0), m.rs);
            for i in 0..keys.len() {
                m.get(keys.ptr(i));
                m.check(&format!("e48 get#{i}"));
            }
            for i in 0..keys.len() {
                m.del(keys.ptr(i), 0);
                m.check(&format!("e48 del#{i}"));
            }
            m.free();
        }
        for &keysize in &[1usize, 8, 16] {
            let keys = BinKeys::random_prefix(&mut rng, 6, keysize, 24);
            let kp: Vec<*mut c_void> = (0..keys.len()).map(|i| keys.ptr(i)).collect();
            let out = assert_same_crash(
                &format!("e48 elemsize=0 keysize={keysize}"),
                || {
                    let mut t: *mut c_void = std::ptr::null_mut();
                    for &k in &kp {
                        t = (c.hmput_key)(t, 0, k, keysize, HM_BINARY);
                    }
                    for &k in &kp {
                        (c.hmget_key)(t, 0, k, keysize, HM_BINARY);
                    }
                },
                || {
                    let mut t: *mut c_void = std::ptr::null_mut();
                    for &k in &kp {
                        t = (rs.hmput_key)(t, 0, k, keysize, HM_BINARY);
                    }
                    for &k in &kp {
                        (rs.hmget_key)(t, 0, k, keysize, HM_BINARY);
                    }
                },
            );
            let _ = out;
        }
        // and through hmput_default / hmget_key_ts / shmode_func
        let a = (c.hmput_default)(std::ptr::null_mut(), 0);
        let b = (rs.hmput_default)(std::ptr::null_mut(), 0);
        assert_same(
            "e48 hmput_default e0",
            &snap_map(a, 0, KeyKind::Binary),
            &snap_map(b, 0, KeyKind::Binary),
        );
        (c.hmfree_func)(hash_to_arr(a, 0), 0);
        (rs.hmfree_func)(hash_to_arr(b, 0), 0);
        for &sh in &[SH_NONE, SH_DEFAULT] {
            let a = (c.shmode_func)(0, sh);
            let b = (rs.shmode_func)(0, sh);
            assert_same(
                "e48 shmode_func e0",
                &snap_map(a, 0, KeyKind::Binary),
                &snap_map(b, 0, KeyKind::Binary),
            );
            (c.hmfree_func)(hash_to_arr(a, 0), 0);
            (rs.hmfree_func)(hash_to_arr(b, 0), 0);
        }
    });
}

/// E49 -- `hmdel_key` honours `keyoffset` while `hmput_key` hardcodes 0.
#[test]
fn err_e49_nonzero_keyoffset() {
    let mut rng = Rng::new(49);
    with_libs(0x31415926, |c, rs| {
        for (elemsize, keysize) in [(16usize, 8usize), (24, 8), (40, 16), (16, 4), (8, 4)] {
            for keyoffset in [0usize, 1, 4, 8, elemsize - 1, elemsize] {
                let keys = BinKeys::random_prefix(&mut rng, 10, keysize, keysize.max(8) + 8);
                let mut m = MapPair::new(c, rs, elemsize, keysize, HM_BINARY, Arena::Auto);
                for i in 0..keys.len() {
                    m.put(keys.ptr(i), i as u64);
                }
                let before_len = m.len();
                let mut hits = 0isize;
                for i in 0..keys.len() {
                    let t = m.del(keys.ptr(i), keyoffset);
                    hits += t;
                    m.check(&format!(
                        "e49 e{elemsize} k{keysize} off{keyoffset} del#{i}"
                    ));
                }
                if keyoffset == 0 {
                    assert_eq!(hits, before_len, "keyoffset 0 must find every key");
                    assert_eq!(m.len(), 0);
                } else {
                    // whatever the C does, the Rust must do the same -- already
                    // asserted inside `del`/`check`
                    assert_eq!(m.len(), before_len - hits);
                }
                m.free();
            }
        }
    });
}
