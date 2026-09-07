//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejection condition, calls BOTH libraries
//! through their `.so` exports, and asserts they produce the SAME sentinel /
//! state (not merely "both failed").

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const SEED: usize = 0x31415926;

fn reseed() {
    let (c, r) = apis();
    unsafe {
        (c.rand_seed)(SEED);
        (r.rand_seed)(SEED);
    }
}

unsafe fn cmp(ctx: &str, mc: &Map, mr: &Map) {
    assert_same(ctx, &mc.snap(), &mr.snap());
}

fn pair(elemsize: usize, keysize: usize, keyoffset: usize, ptrk: bool) -> (Map<'static>, Map<'static>) {
    let (c, r) = apis();
    (
        Map::new(c, elemsize, keysize, keyoffset, ptrk),
        Map::new(r, elemsize, keysize, keyoffset, ptrk),
    )
}

// ===========================================================================
// row 1 / 2 / 3 — stbds_arrgrowf early-outs and minimum capacity
// ===========================================================================

#[test]
fn err01_arrgrowf_already_satisfied_returns_input_pointer() {
    let (c, r) = apis();
    unsafe {
        // NULL in, nothing requested -> NULL out (the early-out on a NULL array)
        let pc = (c.arrgrowf)(std::ptr::null_mut(), 4, 0, 0);
        let pr = (r.arrgrowf)(std::ptr::null_mut(), 4, 0, 0);
        assert_same("arrgrowf(NULL,4,0,0)", &pc.is_null(), &pr.is_null());
        assert!(pc.is_null(), "C must return NULL here");

        let bc = (c.arrgrowf)(std::ptr::null_mut(), 4, 0, 16);
        let br = (r.arrgrowf)(std::ptr::null_mut(), 4, 0, 16);
        for &min_cap in &[0usize, 1, 8, 15, 16] {
            let qc = (c.arrgrowf)(bc, 4, 0, min_cap);
            let qr = (r.arrgrowf)(br, 4, 0, min_cap);
            assert_same("identity", &(qc == bc), &(qr == br));
            assert!(qc == bc, "C must not reallocate for min_cap={min_cap}");
            assert_same("state", &snap_arr(qc, 4, 0), &snap_arr(qr, 4, 0));
        }
        (c.arrfreef)(bc);
        (r.arrfreef)(br);
    }
}

#[test]
fn err02_arrgrowf_from_null_initialises_header() {
    let (c, r) = apis();
    unsafe {
        let pc = (c.arrgrowf)(std::ptr::null_mut(), 8, 3, 0);
        let pr = (r.arrgrowf)(std::ptr::null_mut(), 8, 3, 0);
        let sc = snap_arr(pc, 8, 0);
        let sr = snap_arr(pr, 8, 0);
        assert_same("fresh header", &sc, &sr);
        assert_eq!((sc.length, sc.temp, sc.has_table), (0, 0, false));
        (c.arrfreef)(pc);
        (r.arrfreef)(pr);
    }
}

#[test]
fn err03_arrgrowf_minimum_capacity_is_four() {
    let (c, r) = apis();
    unsafe {
        for &(addlen, min_cap) in &[(1usize, 0usize), (0, 1), (1, 2), (2, 3), (3, 3)] {
            let pc = (c.arrgrowf)(std::ptr::null_mut(), 8, addlen, min_cap);
            let pr = (r.arrgrowf)(std::ptr::null_mut(), 8, addlen, min_cap);
            let sc = snap_arr(pc, 8, 0);
            assert_same(
                &format!("min cap ({addlen},{min_cap})"),
                &sc,
                &snap_arr(pr, 8, 0),
            );
            assert_eq!(sc.capacity, 4, "C rounds up to 4");
            (c.arrfreef)(pc);
            (r.arrfreef)(pr);
        }
    }
}

// ===========================================================================
// row 4 / 5 — stbds_hmfree_func null / no-hash-table
// ===========================================================================

#[test]
fn err04_hmfree_func_null_is_noop() {
    let (c, r) = apis();
    unsafe {
        for elemsize in [0usize, 1, 8, 16] {
            (c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

#[test]
fn err05_hmfree_func_without_hash_table() {
    let (c, r) = apis();
    unsafe {
        for elemsize in [1usize, 8, 16, 32] {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            assert_same(
                "no-table array",
                &snap_arr(ac, elemsize, 0),
                &snap_arr(ar, elemsize, 0),
            );
            (c.hmfree_func)(ac, elemsize);
            (r.hmfree_func)(ar, elemsize);
        }
    }
}

// ===========================================================================
// rows 6 / 7 / 10 / 11 — key-absent -> -1 from both probe scans
// ===========================================================================

#[test]
fn err06_07_10_11_missing_key_returns_minus_one() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    let mut rng = Rng::new(0xdead);
    unsafe {
        // fill so that probing wraps inside buckets and across buckets
        for i in 0..200u32 {
            let mut k = i.to_ne_bytes();
            let v = i.to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
        }
        let mut misses = 0;
        for _ in 0..3000 {
            let mut k = (1_000_000 + rng.next_u32() % 1_000_000).to_ne_bytes();
            let ic = mc.get(&mut k, HM_BINARY);
            let ir = mr.get(&mut k, HM_BINARY);
            assert_same("absent key index", &ic, &ir);
            if ic == -1 {
                misses += 1;
            }
            // hmget_key also writes the sentinel into header->temp
            assert_same("absent key header temp", &mc.temp(), &mr.temp());
            let tc = mc.get_ts(&mut k, HM_BINARY);
            let tr = mr.get_ts(&mut k, HM_BINARY);
            assert_same("absent key _ts", &tc, &tr);
            assert_eq!(tc, -1);
        }
        assert!(misses > 2900, "expected mostly misses, got {misses}");
        cmp("after misses", &mc, &mr);
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// rows 8 / 9 — hmget_key_ts on NULL and on a table-less array
// ===========================================================================

#[test]
fn err08_09_hmget_key_ts_null_and_tableless() {
    let _g = seed_lock();
    reseed();
    for elemsize in [8usize, 16, 32] {
        let (mut mc, mut mr) = pair(elemsize, 4, 0, false);
        unsafe {
            let mut k = 42u32.to_ne_bytes();
            let tc = mc.get_ts(&mut k, HM_BINARY);
            let tr = mr.get_ts(&mut k, HM_BINARY);
            assert_same("ts(NULL) sentinel", &tc, &tr);
            assert_eq!(tc, -1, "C returns STBDS_INDEX_EMPTY");
            cmp("ts(NULL) state", &mc, &mr);
            let s = mc.snap().unwrap();
            assert_eq!((s.length, s.has_table), (1, false));

            // now hash_table is still NULL -> the *other* -1 branch
            let tc = mc.get_ts(&mut k, HM_BINARY);
            let tr = mr.get_ts(&mut k, HM_BINARY);
            assert_same("ts(tableless) sentinel", &tc, &tr);
            assert_eq!(tc, -1);
            cmp("ts(tableless) state", &mc, &mr);

            // hmget_key mirrors the sentinel into header->temp
            let ic = mc.get(&mut k, HM_BINARY);
            let ir = mr.get(&mut k, HM_BINARY);
            assert_same("get(tableless)", &ic, &ir);
            assert_eq!(ic, -1);
            cmp("get(tableless) state", &mc, &mr);
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// rows 12 / 13 — hmput_default on NULL and on a zero-length array
// ===========================================================================

#[test]
fn err12_13_hmput_default_null_and_zero_length() {
    let _g = seed_lock();
    reseed();
    for elemsize in [1usize, 8, 16, 32] {
        let (mut mc, mut mr) = pair(elemsize, 4, 0, false);
        unsafe {
            mc.put_default();
            mr.put_default();
            cmp(&format!("put_default(NULL) es={elemsize}"), &mc, &mr);
            assert_eq!(mc.snap().unwrap().length, 1);
            // idempotent while length > 0
            mc.put_default();
            mr.put_default();
            cmp(&format!("put_default again es={elemsize}"), &mc, &mr);
            assert_eq!(mc.snap().unwrap().length, 1);
            // force the `length == 0` branch
            (*mc.header()).length = 0;
            (*mr.header()).length = 0;
            mc.put_default();
            mr.put_default();
            cmp(&format!("put_default len0 es={elemsize}"), &mc, &mr);
            assert_eq!(mc.snap().unwrap().length, 1);
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// rows 14 / 15 / 16 / 17 — hmdel_key sentinels
// ===========================================================================

#[test]
fn err14_hmdel_key_null_returns_null() {
    let (c, r) = apis();
    unsafe {
        for elemsize in [1usize, 8, 16] {
            for mode in [-1 as c_int, 0, 1, 2, 1000] {
                let mut k = [0u8; 8];
                let pc = (c.hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                let pr = (r.hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                assert_same(
                    &format!("hmdel_key(NULL) es={elemsize} mode={mode}"),
                    &(pc as usize),
                    &(pr as usize),
                );
                assert!(pc.is_null(), "C returns 0");
            }
        }
    }
}

#[test]
fn err15_hmdel_key_without_hash_table() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        mc.put_default();
        mr.put_default();
        let mut k = 3u32.to_ne_bytes();
        assert_same(
            "hmdel tableless",
            &mc.del(&mut k, HM_BINARY),
            &mr.del(&mut k, HM_BINARY),
        );
        assert_eq!(mc.temp(), 0, "C sets temp = 0");
        cmp("hmdel tableless state", &mc, &mr);
        mc.free();
        mr.free();
    }
}

#[test]
fn err16_hmdel_key_missing_key_leaves_everything_untouched() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        for i in 0..50u32 {
            let mut k = i.to_ne_bytes();
            let v = i.to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
        }
        // everything except `temp` (which every hmdel_key call resets to 0)
        let digest = |m: &Map| unsafe {
            let s = m.snap().unwrap();
            (s.length, s.capacity, s.payload, s.index)
        };
        let before_c = digest(&mc);
        let before_r = digest(&mr);
        for i in 1000..1200u32 {
            let mut k = i.to_ne_bytes();
            let dc = mc.del(&mut k, HM_BINARY);
            let dr = mr.del(&mut k, HM_BINARY);
            assert_same(&format!("del missing #{i}"), &dc, &dr);
            assert_eq!(dc, 0, "C reports 'not deleted'");
        }
        // used_count / tombstone_count / length / buckets must be unchanged
        assert_same("del-missing left state alone (C)", &before_c, &digest(&mc));
        assert_same("del-missing left state alone (Rust)", &before_r, &digest(&mr));
        assert_eq!(mc.temp(), 0);
        assert_eq!(mr.temp(), 0);
        cmp("del-missing C vs Rust", &mc, &mr);
        mc.free();
        mr.free();
    }
}

#[test]
fn err17_hmdel_key_present_marks_tombstone() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        for i in 0..20u32 {
            let mut k = i.to_ne_bytes();
            let v = i.to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
        }
        let mut k = 7u32.to_ne_bytes();
        let dc = mc.del(&mut k, HM_BINARY);
        let dr = mr.del(&mut k, HM_BINARY);
        assert_same("del present", &dc, &dr);
        assert_eq!(dc, 1, "C sets temp = 1");
        let s = mc.snap().unwrap();
        cmp("del present state", &mc, &mr);
        let idx = s.index.unwrap();
        let has_tombstone = idx
            .buckets
            .iter()
            .any(|(h, i)| h.iter().zip(i).any(|(&hh, &ii)| hh == 1 && ii == -2));
        assert!(has_tombstone, "C must have written HASH_DELETED/INDEX_DELETED");
        assert_eq!(idx.scalars[4], 1, "tombstone_count");
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// row 18 — `mode == STBDS_HM_STRING` exactly gates the strdup free
// ===========================================================================

#[test]
fn err18_strdup_free_only_for_mode_exactly_one() {
    for (tag, mode) in [("mode=1", HM_STRING), ("mode=2", HM_PTR_TO_STRING)] {
        let _g = seed_lock();
        reseed();
        let (c, r) = apis();
        let mut mc = unsafe { Map::new_shmode(c, 16, 8, 0, SH_STRDUP) };
        let mut mr = unsafe { Map::new_shmode(r, 16, 8, 0, SH_STRDUP) };
        unsafe {
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..6usize {
                let mut k: Vec<u8> = format!("dk{}", i).into_bytes();
                k.push(0);
                keys.push(k);
                let key = keys.last_mut().unwrap();
                let v = (i as u64).to_ne_bytes();
                mc.put(key, &v, 8, mode);
                mr.put(key, &v, 8, mode);
            }
            // delete the LAST array element: identical for both modes except for
            // the free(), which must happen in exactly the same library pair.
            let len = (*mc.header()).length;
            let p = (mc.a as *const u8).add((len - 2) * 16) as *const *const c_char;
            let mut k = read_cstr(*p);
            k.push(0);
            assert_same(
                &format!("{tag} del last"),
                &mc.del(&mut k, mode),
                &mr.del(&mut k, mode),
            );
            cmp(&format!("{tag} del last state"), &mc, &mr);
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// row 19 — the `hash < 2` clamp
// ===========================================================================

#[test]
fn err19_hash_below_two_is_clamped_identically() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    let mut rng = Rng::new(0x1919);
    unsafe {
        // The clamp guarantees no LIVE slot ever carries hash 0 (EMPTY) or 1
        // (DELETED-marker) as a real key hash.  Both libraries must maintain the
        // invariant and agree on every stored hash.
        for i in 0..800 {
            let mut k = rng.next_u32().to_ne_bytes();
            let v = rng.next_u32().to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
            if i % 3 == 0 {
                let mut kd = rng.next_u32().to_ne_bytes();
                assert_same("clamp del", &mc.del(&mut kd, HM_BINARY), &mr.del(&mut kd, HM_BINARY));
            }
            cmp(&format!("clamp #{i}"), &mc, &mr);
        }
        let idx = mc.snap().unwrap().index.unwrap();
        for (h, ix) in &idx.buckets {
            for (&hh, &ii) in h.iter().zip(ix) {
                if ii >= 0 {
                    assert!(hh >= 2, "live slot with hash {hh} < 2");
                }
            }
        }
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// rows 20 / 21 / 22 / 23 / 24 — hmput_key structural branches
// ===========================================================================

#[test]
fn err20_hmput_key_from_null() {
    let _g = seed_lock();
    reseed();
    for elemsize in [8usize, 16] {
        let (mut mc, mut mr) = pair(elemsize, 4, 0, false);
        unsafe {
            let mut k = 1u32.to_ne_bytes();
            // write the WHOLE element tail so no byte of the payload stays
            // uninitialised (hmput_key only memcpy's the `keysize` key bytes)
            let v = vec![0x5au8; elemsize - 4];
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
            cmp("put from NULL", &mc, &mr);
            let s = mc.snap().unwrap();
            assert_eq!((s.length, s.temp, s.has_table), (2, 0, true));
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn err21_22_growth_threshold_and_initial_string_mode() {
    for (mode, expect_mode) in [(HM_BINARY, 0u8), (HM_STRING, 1u8), (HM_PTR_TO_STRING, 1u8), (-5, 0u8)] {
        let _g = seed_lock();
        reseed();
        let ptrk = mode >= HM_STRING;
        let (mut mc, mut mr) = pair(16, 8, 0, ptrk);
        unsafe {
            let mut keys: Vec<Vec<u8>> = Vec::new();
            let mut slot_counts = Vec::new();
            for i in 0..40usize {
                let mut k: Vec<u8> = if ptrk {
                    let mut s = format!("g{:05}", i).into_bytes();
                    s.push(0);
                    s
                } else {
                    (i as u64).to_ne_bytes().to_vec()
                };
                k.resize(k.len().max(8), 0);
                keys.push(k);
                let key = keys.last_mut().unwrap();
                let v = (i as u64).to_ne_bytes();
                mc.put(key, &v, 8, mode);
                mr.put(key, &v, 8, mode);
                cmp(&format!("mode={mode} grow #{i}"), &mc, &mr);
                let idx = mc.snap().unwrap().index.unwrap();
                if i == 0 {
                    assert_eq!(idx.arena_mode, expect_mode, "initial string.mode for mode={mode}");
                }
                slot_counts.push(idx.scalars[0]);
            }
            slot_counts.dedup();
            assert!(
                slot_counts.len() >= 3,
                "expected several rehashes, got {slot_counts:?}"
            );
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn err23_overwrite_existing_key() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        let mut k = 9u32.to_ne_bytes();
        for i in 0..30u32 {
            let v = (0xa000 + i).to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
            cmp(&format!("overwrite #{i}"), &mc, &mr);
            let s = mc.snap().unwrap();
            assert_eq!(s.length, 2, "no new element must be appended");
            assert_eq!(s.index.as_ref().unwrap().scalars[1], 1, "used_count stays 1");
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn err24_tombstone_is_reused() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    let mut rng = Rng::new(0x2424);
    unsafe {
        for i in 0..60u32 {
            let mut k = i.to_ne_bytes();
            let v = i.to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
        }
        let mut saw_reuse = false;
        for i in 0..400u32 {
            let mut kd = (rng.next_u32() % 60).to_ne_bytes();
            let before = mc.snap().unwrap().index.unwrap().scalars[4];
            assert_same("reuse del", &mc.del(&mut kd, HM_BINARY), &mr.del(&mut kd, HM_BINARY));
            let mut ka = (5000 + i).to_ne_bytes();
            let v = i.to_ne_bytes();
            let mid = mc.snap().unwrap().index.unwrap().scalars[4];
            mc.put(&mut ka, &v, 4, HM_BINARY);
            mr.put(&mut ka, &v, 4, HM_BINARY);
            let after = mc.snap().unwrap().index.unwrap().scalars[4];
            if mid > 0 && after < mid {
                saw_reuse = true;
            }
            let _ = before;
            cmp(&format!("reuse #{i}"), &mc, &mr);
        }
        assert!(saw_reuse, "the tombstone-reuse branch was never taken");
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// row 25 — the `default:` (raw memcpy) branch of the string-mode switch
// ===========================================================================

#[test]
fn err25_default_switch_branch_memcpys_the_key() {
    let _g = seed_lock();
    reseed();
    let (c, r) = apis();
    // string.mode == STBDS_SH_NONE together with mode >= STBDS_HM_STRING
    let mut mc = unsafe { Map::new_shmode(c, 16, 8, 0, SH_NONE) };
    let mut mr = unsafe { Map::new_shmode(r, 16, 8, 0, SH_NONE) };
    mc.pointer_keys = false;
    mr.pointer_keys = false;
    unsafe {
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..4usize {
            let mut k: Vec<u8> = format!("m{}", i).into_bytes();
            k.resize(8, 0);
            keys.push(k);
            let key = keys.last_mut().unwrap();
            let v = (i as u64).to_ne_bytes();
            mc.put(key, &v, 8, HM_STRING);
            mr.put(key, &v, 8, HM_STRING);
            cmp(&format!("memcpy branch #{i}"), &mc, &mr);
            // the element must hold the raw key BYTES, not a pointer
            let elem = std::slice::from_raw_parts((mc.a as *const u8).add(i * 16), 8);
            assert_eq!(elem, &keys[i][..8]);
        }
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// rows 26 / 27 / 28 — shrink / rebuild / 8-slot special case
// ===========================================================================

#[test]
fn err26_shrink_threshold() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        for i in 0..300u32 {
            let mut k = i.to_ne_bytes();
            let v = i.to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
        }
        let big = mc.snap().unwrap().index.unwrap().scalars[0];
        let mut shrank = false;
        for i in 0..300u32 {
            let mut k = i.to_ne_bytes();
            assert_same(&format!("shrink del #{i}"), &mc.del(&mut k, HM_BINARY), &mr.del(&mut k, HM_BINARY));
            cmp(&format!("shrink #{i}"), &mc, &mr);
            if mc.snap().unwrap().index.unwrap().scalars[0] < big {
                shrank = true;
            }
        }
        assert!(shrank, "the shrink branch was never taken");
        let final_idx = mc.snap().unwrap().index.unwrap();
        assert_eq!(final_idx.scalars[0], 8, "shrinks down to 8 slots");
        mc.free();
        mr.free();
    }
}

#[test]
fn err27_tombstone_rebuild_same_size() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        for i in 0..200u32 {
            let mut k = i.to_ne_bytes();
            let v = i.to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
        }
        let mut saw_rebuild = false;
        for round in 0..200u32 {
            let before = mc.snap().unwrap().index.unwrap();
            let mut kd = round.to_ne_bytes();
            assert_same(
                &format!("rebuild del #{round}"),
                &mc.del(&mut kd, HM_BINARY),
                &mr.del(&mut kd, HM_BINARY),
            );
            let after = mc.snap().unwrap().index.unwrap();
            if after.scalars[0] == before.scalars[0] && after.scalars[4] == 0 && before.scalars[4] > 0
            {
                saw_rebuild = true;
            }
            let mut ka = (10_000 + round).to_ne_bytes();
            let v = round.to_ne_bytes();
            mc.put(&mut ka, &v, 4, HM_BINARY);
            mr.put(&mut ka, &v, 4, HM_BINARY);
            cmp(&format!("rebuild #{round}"), &mc, &mr);
        }
        assert!(saw_rebuild, "the same-size rebuild branch was never taken");
        mc.free();
        mr.free();
    }
}

#[test]
fn err28_eight_slot_table_never_shrinks() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        let mut k = 1u32.to_ne_bytes();
        let v = 1u32.to_ne_bytes();
        mc.put(&mut k, &v, 4, HM_BINARY);
        mr.put(&mut k, &v, 4, HM_BINARY);
        let idx = mc.snap().unwrap().index.unwrap();
        assert_eq!(idx.scalars[0], 8);
        assert_eq!(idx.scalars[3], 0, "used_count_shrink_threshold forced to 0");
        cmp("8-slot", &mc, &mr);
        assert_same("del from 8-slot", &mc.del(&mut k, HM_BINARY), &mr.del(&mut k, HM_BINARY));
        cmp("8-slot after del", &mc, &mr);
        assert_eq!(mc.snap().unwrap().index.unwrap().scalars[0], 8);
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// rows 29 / 30 / 31 / 32 — string arena
// ===========================================================================

#[test]
fn err29_30_31_stralloc_block_paths() {
    let (c, r) = apis();
    unsafe {
        // fresh arena, remaining == 0 -> new block
        for &len in &[1usize, 511, 512, 513, 1023, 1024, 100_000] {
            let mut ac = StringArena::default();
            let mut ar = StringArena::default();
            let mut s = vec![b'q'; len];
            s.push(0);
            let pc = (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_same(
                &format!("stralloc len={len}"),
                &(ac.remaining, ac.block, ac.mode, read_cstr(pc)),
                &(ar.remaining, ar.block, ar.mode, read_cstr(pr)),
            );
            // `len+1 > 512` takes the over-sized-block path on a fresh arena
            if len + 1 > 512 {
                assert_eq!(ac.remaining, 0, "oversized path sets remaining = 0");
            }
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
        // block-counter saturation at 1<<20
        let mut ac = StringArena::default();
        let mut ar = StringArena::default();
        let mut prev_c = 0u8;
        for i in 0..40_000usize {
            let mut s = vec![b'a'; 200];
            s.push(0);
            (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_same(
                &format!("saturation #{i}"),
                &(ac.remaining, ac.block),
                &(ar.remaining, ar.block),
            );
            prev_c = ac.block;
        }
        assert!(prev_c >= 22, "block counter should saturate at 22, got {prev_c}");
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }
}

#[test]
fn err32_strreset_on_empty_arena() {
    let (c, r) = apis();
    unsafe {
        let mut ac = StringArena::default();
        let mut ar = StringArena::default();
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_same(
            "strreset(empty)",
            &(ac.remaining, ac.block, ac.mode, ac.storage.is_null()),
            &(ar.remaining, ar.block, ar.mode, ar.storage.is_null()),
        );
        // a non-zero but block-less arena (mode/block set, no storage)
        let mut ac = StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 5,
            mode: 3,
        };
        let mut ar = ac;
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_same(
            "strreset(blockless)",
            &(ac.remaining, ac.block, ac.mode),
            &(ar.remaining, ar.block, ar.mode),
        );
        assert_eq!((ac.remaining, ac.block, ac.mode), (0, 0, 0));
    }
}

// ===========================================================================
// row 33 — shmode_func with out-of-enum modes
// ===========================================================================

#[test]
fn err33_shmode_func_enum_truncation() {
    let (c, r) = apis();
    let _g = seed_lock();
    for shmode in [
        -1000 as c_int, -256, -255, -1, 0, 1, 2, 3, 4, 5, 127, 128, 255, 256, 257, 259, 512,
        c_int::MAX, c_int::MIN,
    ] {
        reseed();
        unsafe {
            let ac = (c.shmode_func)(16, shmode);
            let ar = (r.shmode_func)(16, shmode);
            let raw_c = (ac as *mut u8).sub(16) as *mut c_void;
            let raw_r = (ar as *mut u8).sub(16) as *mut c_void;
            let sc = snap_index(raw_c).unwrap();
            let sr = snap_index(raw_r).unwrap();
            assert_same(&format!("shmode_func({shmode})"), &sc, &sr);
            assert_eq!(
                sc.arena_mode,
                (shmode as u32 & 0xff) as u8,
                "C truncates to unsigned char"
            );
            assert_same(
                &format!("shmode_func({shmode}) array"),
                &snap_arr(raw_c, 16, 1),
                &snap_arr(raw_r, 16, 1),
            );
            (c.hmfree_func)(raw_c, 16);
            (r.hmfree_func)(raw_r, 16);
        }
    }
}

// ===========================================================================
// row 34 — is_key_equal mode classes (out-of-range enum values over FFI)
// ===========================================================================

#[test]
fn err34_mode_classes_binary_vs_string() {
    // mode <= 0 must behave EXACTLY like STBDS_HM_BINARY
    let _g = seed_lock();
    let mut reference: Option<Vec<u8>> = None;
    for mode in [0 as c_int, -1, -7, c_int::MIN] {
        reseed();
        let (mut mc, mut mr) = pair(8, 4, 0, false);
        unsafe {
            for i in 0..40u32 {
                let mut k = (i % 25).to_ne_bytes();
                let v = i.to_ne_bytes();
                mc.put(&mut k, &v, 4, mode);
                mr.put(&mut k, &v, 4, mode);
                cmp(&format!("mode={mode} put #{i}"), &mc, &mr);
            }
            for i in 0..40u32 {
                let mut k = i.to_ne_bytes();
                assert_same(
                    &format!("mode={mode} get #{i}"),
                    &mc.get(&mut k, mode),
                    &mr.get(&mut k, mode),
                );
            }
            let payload = mc.snap().unwrap().payload;
            match &reference {
                None => reference = Some(payload),
                Some(rp) => assert_eq!(rp, &payload, "mode {mode} diverged from mode 0"),
            }
            mc.free();
            mr.free();
        }
    }

    // mode >= 1 must behave EXACTLY like STBDS_HM_STRING for hashing/compare
    let mut ref_len: Option<usize> = None;
    for mode in [1 as c_int, 2, 3, 99, c_int::MAX] {
        reseed();
        let (mut mc, mut mr) = pair(16, 8, 0, true);
        unsafe {
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..40usize {
                let mut k: Vec<u8> = format!("s{:04}", i % 25).into_bytes();
                k.push(0);
                keys.push(k);
                let key = keys.last_mut().unwrap();
                let v = (i as u64).to_ne_bytes();
                mc.put(key, &v, 8, mode);
                mr.put(key, &v, 8, mode);
                cmp(&format!("smode={mode} put #{i}"), &mc, &mr);
            }
            for i in 0..40usize {
                let mut k: Vec<u8> = format!("s{:04}", i).into_bytes();
                k.push(0);
                assert_same(
                    &format!("smode={mode} get #{i}"),
                    &mc.get(&mut k, mode),
                    &mr.get(&mut k, mode),
                );
            }
            let len = mc.snap().unwrap().length;
            match ref_len {
                None => ref_len = Some(len),
                Some(l) => assert_eq!(l, len, "mode {mode} diverged from mode 1"),
            }
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// row 35 / 36 — zero-length hash inputs
// ===========================================================================

#[test]
fn err35_hash_bytes_zero_length() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x3535);
    unsafe {
        let mut data = rng.bytes(64);
        for &seed in &[0usize, 1, 2, usize::MAX, 0x31415926] {
            let hc = (c.hash_bytes)(data.as_mut_ptr() as *mut c_void, 0, seed);
            let hr = (r.hash_bytes)(data.as_mut_ptr() as *mut c_void, 0, seed);
            assert_same(&format!("hash_bytes len=0 seed={seed:#x}"), &hc, &hr);
        }
        // len == 0 must not read the buffer at all: a NULL pointer is fine
        for &seed in &[0usize, 1, usize::MAX] {
            let hc = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let hr = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_same("hash_bytes(NULL,0)", &hc, &hr);
        }
    }
}

#[test]
fn err36_hash_string_empty() {
    let (c, r) = apis();
    unsafe {
        let mut empty = [0u8; 1];
        for &seed in &[0usize, 1, usize::MAX, 0x31415926, 0xffff_0000_ffff_0000] {
            let hc = (c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            let hr = (r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            assert_same(&format!("hash_string(\"\") seed={seed:#x}"), &hc, &hr);
        }
    }
}

// ===========================================================================
// row 37 — strkey with negative / extreme ints
// ===========================================================================

#[test]
fn err37_strkey_extremes() {
    let (c, r) = apis();
    unsafe {
        for n in [0 as c_int, -1, -10, -2147483647, c_int::MIN, c_int::MAX] {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            assert_same(&format!("strkey({n})"), &read_cstr(pc), &read_cstr(pr));
        }
        assert_eq!(read_cstr((c.strkey)(c_int::MIN)), b"test_-2147483648".to_vec());
        assert_eq!(read_cstr((r.strkey)(c_int::MIN)), b"test_-2147483648".to_vec());
    }
}

// ===========================================================================
// generic FFI boundaries: keysize 0, NULL key pointers
// ===========================================================================

#[test]
fn err_generic_zero_keysize() {
    let _g = seed_lock();
    reseed();
    // keysize == 0: hash_bytes over 0 bytes is constant and memcmp(...,0) is
    // always equal, so every key collapses onto the first entry.
    let (mut mc, mut mr) = pair(8, 0, 0, false);
    unsafe {
        for i in 0..20u32 {
            let mut k = i.to_ne_bytes();
            // `keysize == 0` means hmput_key memcpy's nothing, so the element's
            // first 8 bytes would stay uninitialised; write the whole element.
            let v = (i as u64).to_ne_bytes();
            mc.put(&mut k, &v, 0, HM_BINARY);
            mr.put(&mut k, &v, 0, HM_BINARY);
            cmp(&format!("keysize=0 put #{i}"), &mc, &mr);
        }
        assert_eq!(mc.snap().unwrap().length, 2, "all keys collapse to one entry");
        let mut k = 0u32.to_ne_bytes();
        assert_same("keysize=0 get", &mc.get(&mut k, HM_BINARY), &mr.get(&mut k, HM_BINARY));
        assert_same("keysize=0 del", &mc.del(&mut k, HM_BINARY), &mr.del(&mut k, HM_BINARY));
        cmp("keysize=0 final", &mc, &mr);
        mc.free();
        mr.free();
    }
}

#[test]
fn err_generic_null_and_empty_string_keys() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(16, 8, 0, true);
    unsafe {
        let mut keys: Vec<Vec<u8>> = vec![b"\0".to_vec(), b"a\0".to_vec(), b"\0".to_vec()];
        for i in 0..keys.len() {
            let v = (i as u64).to_ne_bytes();
            mc.put(&mut keys[i], &v, 8, HM_STRING);
            mr.put(&mut keys[i], &v, 8, HM_STRING);
            cmp(&format!("empty-string key #{i}"), &mc, &mr);
        }
        let mut e = b"\0".to_vec();
        assert_same("get empty key", &mc.get(&mut e, HM_STRING), &mr.get(&mut e, HM_STRING));
        assert_same("del empty key", &mc.del(&mut e, HM_STRING), &mr.del(&mut e, HM_STRING));
        cmp("empty key final", &mc, &mr);
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// row 38 + oversized allocation — crashing paths, compared in a subprocess
// ===========================================================================

fn run_crash_case(case: &str, lib: &str) -> String {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args(["crash_child", "--exact", "--ignored", "--test-threads=1"])
        .env("DIFF_CRASH_CASE", case)
        .env("DIFF_CRASH_LIB", lib)
        .output()
        .expect("failed to spawn child");
    format!("code={:?} signal={:?}", out.status.code(), out.status.signal())
}

#[test]
#[ignore = "child process helper"]
fn crash_child() {
    let case = match std::env::var("DIFF_CRASH_CASE") {
        Ok(v) => v,
        Err(_) => return,
    };
    let which = std::env::var("DIFF_CRASH_LIB").unwrap();
    let (c, r) = apis();
    let api: &Api = if which == "c" { c } else { r };
    unsafe {
        match case.as_str() {
            // ERRORS.md #38: free((header*)NULL - 1)
            "arrfreef_null" => (api.arrfreef)(std::ptr::null_mut()),
            // oversized request: realloc fails, the C code writes the header anyway
            "arrgrowf_oom" => {
                let p = (api.arrgrowf)(std::ptr::null_mut(), 1, 0, 1usize << 62);
                std::hint::black_box(p);
            }
            // ERRORS.md A5/A6: mode == 2 delete of a NON-last element makes the
            // re-find use the binary comparator on a pointer field -> assert fires
            "hmdel_ptr2str_nonlast" => {
                let mut m = Map::new_shmode(api, 16, 8, 0, SH_DEFAULT);
                let mut keys: Vec<Vec<u8>> = Vec::new();
                for i in 0..6usize {
                    let mut k: Vec<u8> = format!("z{}", i).into_bytes();
                    k.push(0);
                    keys.push(k);
                    let key = keys.last_mut().unwrap();
                    let v = (i as u64).to_ne_bytes();
                    m.put(key, &v, 8, HM_PTR_TO_STRING);
                }
                let mut first = keys[0].clone();
                let d = m.del(&mut first, HM_PTR_TO_STRING);
                std::hint::black_box(d);
            }
            other => panic!("unknown crash case {other}"),
        }
    }
    std::process::exit(0);
}

#[test]
fn err38_arrfreef_null_behaves_identically() {
    let sc = run_crash_case("arrfreef_null", "c");
    let sr = run_crash_case("arrfreef_null", "rust");
    assert_same("arrfreef(NULL)", &sc, &sr);
}

#[test]
fn err_generic_oversized_allocation_behaves_identically() {
    let sc = run_crash_case("arrgrowf_oom", "c");
    let sr = run_crash_case("arrgrowf_oom", "rust");
    assert_same("arrgrowf oversized", &sc, &sr);
}

#[test]
fn err_assert_a5_a6_ptr_to_string_nonlast_delete() {
    let sc = run_crash_case("hmdel_ptr2str_nonlast", "c");
    let sr = run_crash_case("hmdel_ptr2str_nonlast", "rust");
    assert_same("hmdel mode=2 non-last", &sc, &sr);
}

// ===========================================================================
// asserts A1-A9: the conditions must hold (and never fire) in BOTH libraries
// ===========================================================================

#[test]
fn err_asserts_a1_a4_invariants_hold_in_both() {
    let _g = seed_lock();
    reseed();
    let mut rng = Rng::new(0xa55e);
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        for i in 0..1500 {
            let mut k = (rng.below(400) as u32).to_ne_bytes();
            match rng.below(4) {
                0..=1 => {
                    let v = rng.next_u32().to_ne_bytes();
                    mc.put(&mut k, &v, 4, HM_BINARY);
                    mr.put(&mut k, &v, 4, HM_BINARY);
                }
                2 => {
                    assert_same("a-inv del", &mc.del(&mut k, HM_BINARY), &mr.del(&mut k, HM_BINARY));
                }
                _ => {
                    assert_same("a-inv get", &mc.get(&mut k, HM_BINARY), &mr.get(&mut k, HM_BINARY));
                }
            }
            cmp(&format!("a-inv #{i}"), &mc, &mr);
            if let Some(idx) = mc.snap().unwrap().index {
                let slot_count = idx.scalars[0];
                // A1
                assert!(idx.scalars[2] + idx.scalars[5] < slot_count);
                // A3: every live index is inside the array
                let len = mc.snap().unwrap().length as i64;
                for (_, ix) in &idx.buckets {
                    for &v in ix {
                        assert!(v >= -2 && v < len, "index {v} out of range (len {len})");
                    }
                }
                // A2 / A4
                assert!(mc.snap().unwrap().length <= mc.snap().unwrap().capacity);
                assert!(idx.scalars[1] <= slot_count);
            }
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn err_asserts_a8_a9_arr_ins_never_aborts() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xa99a);
    unsafe {
        for _ in 0..300 {
            let n = rng.next_u32() as c_int;
            (c.arr_ins)(n);
            (r.arr_ins)(n);
        }
        for n in [0, 1, 4, -1, c_int::MAX, c_int::MIN] {
            (c.arr_ins)(n);
            (r.arr_ins)(n);
        }
    }
}
