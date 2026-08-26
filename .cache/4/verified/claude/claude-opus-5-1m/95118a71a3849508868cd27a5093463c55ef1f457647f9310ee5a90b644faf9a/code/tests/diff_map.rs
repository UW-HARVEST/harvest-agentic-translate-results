//! Phase B — differential tests for the hash-map entry points driven the way a
//! real consumer drives them (`stbds_hmput_key` / `stbds_hmget_key[_ts]` /
//! `stbds_hmdel_key` / `stbds_hmfree_func` / `stbds_shmode_func`), through
//! composed put/get/del pipelines rather than one call at a time.
//!
//! Covers CONFIGS.md rows 30, 37–70.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// A pair of identical maps, one per implementation, kept in lock-step.
// ---------------------------------------------------------------------------

struct Pair<'a> {
    c: Map<'a>,
    r: Map<'a>,
    label: String,
    op: usize,
}

impl<'a> Pair<'a> {
    fn new(c: &'a Api, r: &'a Api, es: usize, kk: KeyKind, label: &str) -> Pair<'a> {
        Pair { c: Map::new(c, es, kk), r: Map::new(r, es, kk), label: label.to_string(), op: 0 }
    }

    fn new_shmode(c: &'a Api, r: &'a Api, es: usize, kk: KeyKind, mode: c_int, label: &str) -> Pair<'a> {
        Pair {
            c: Map::new_shmode(c, es, kk, mode),
            r: Map::new_shmode(r, es, kk, mode),
            label: label.to_string(),
            op: 0,
        }
    }

    unsafe fn check(&mut self, what: &str) {
        let cd = self.c.dump();
        let rd = self.r.dump();
        same(&format!("{} [op #{} {}]", self.label, self.op, what), &cd, &rd);
        self.op += 1;
    }

    unsafe fn put(&mut self, key: *mut u8, keysize: usize, mode: c_int, pat: u8) -> isize {
        let ci = self.c.put(key, keysize, mode, pat);
        let ri = self.r.put(key, keysize, mode, pat);
        assert_eq!(ci, ri, "{} [op #{}] put index", self.label, self.op);
        self.check("put");
        ci
    }

    /// put + compare `stbds_temp_key` (only valid for `mode >= STBDS_HM_STRING`)
    unsafe fn put_str(&mut self, key: *mut u8, keysize: usize, mode: c_int, pat: u8) -> isize {
        let i = self.put(key, keysize, mode, pat);
        let ck = cstr_to_vec(temp_key(self.c.t, self.c.elemsize));
        let rk = cstr_to_vec(temp_key(self.r.t, self.r.elemsize));
        assert_eq!(ck, rk, "{} [op #{}] temp_key", self.label, self.op);
        i
    }

    unsafe fn get(&mut self, key: *mut u8, keysize: usize, mode: c_int) -> isize {
        let ci = self.c.get(key, keysize, mode);
        let ri = self.r.get(key, keysize, mode);
        assert_eq!(ci, ri, "{} [op #{}] get index", self.label, self.op);
        self.check("get");
        ci
    }

    unsafe fn get_ts(&mut self, key: *mut u8, keysize: usize, mode: c_int) -> isize {
        let ci = self.c.get_ts(key, keysize, mode);
        let ri = self.r.get_ts(key, keysize, mode);
        assert_eq!(ci, ri, "{} [op #{}] get_ts temp", self.label, self.op);
        self.check("get_ts");
        ci
    }

    unsafe fn del(&mut self, key: *mut u8, keysize: usize, keyoffset: usize, mode: c_int) -> isize {
        let ci = self.c.del(key, keysize, keyoffset, mode);
        let ri = self.r.del(key, keysize, keyoffset, mode);
        assert_eq!(ci, ri, "{} [op #{}] del result", self.label, self.op);
        assert_eq!(
            self.c.t.is_null(),
            self.r.t.is_null(),
            "{} [op #{}] del NULL-ness",
            self.label,
            self.op
        );
        self.check("del");
        ci
    }

    unsafe fn put_default(&mut self) {
        self.c.put_default();
        self.r.put_default();
        self.check("put_default");
    }

    unsafe fn slot_count(&self) -> usize {
        if self.c.t.is_null() {
            return 0;
        }
        let h = self.c.arr().sub(HEADER_SIZE) as *const ArrayHeader;
        let ht = (*h).hash_table as *const HashIndex;
        if ht.is_null() {
            0
        } else {
            (*ht).slot_count
        }
    }

    unsafe fn tombstones(&self) -> usize {
        if self.c.t.is_null() {
            return 0;
        }
        let h = self.c.arr().sub(HEADER_SIZE) as *const ArrayHeader;
        let ht = (*h).hash_table as *const HashIndex;
        if ht.is_null() {
            0
        } else {
            (*ht).tombstone_count
        }
    }

    unsafe fn free(&mut self) {
        self.c.free();
        self.r.free();
        self.check("free");
    }
}

// ---------------------------------------------------------------------------
// key material
// ---------------------------------------------------------------------------

/// `n` pairwise-distinct binary keys of `keysize` bytes.
fn bin_keys(rng: &mut Rng, n: usize, keysize: usize) -> Vec<Vec<u8>> {
    let mut v = Vec::with_capacity(n);
    let mut seen = std::collections::HashSet::new();
    let mut tries = 0usize;
    while v.len() < n {
        let k = rng.bytes(keysize);
        if seen.insert(k.clone()) {
            v.push(k);
        }
        tries += 1;
        assert!(tries < 1_000_000, "cannot build {n} distinct {keysize}-byte keys");
    }
    v
}

/// `n` pairwise-distinct NUL-terminated keys, cycling through `lens`. A length
/// that cannot yield enough distinct strings (e.g. 0, which only admits `""`)
/// is widened until it can, so the requested lengths are still all exercised.
fn str_keys(rng: &mut Rng, n: usize, lens: &[usize]) -> Vec<Vec<u8>> {
    let mut v = Vec::with_capacity(n);
    let mut seen = std::collections::HashSet::new();
    let mut i = 0;
    while v.len() < n {
        let mut l = lens[i % lens.len()];
        i += 1;
        let mut tries = 0usize;
        loop {
            let k = rng.cstring(l);
            if seen.insert(k.clone()) {
                v.push(k);
                break;
            }
            tries += 1;
            if tries > 64 {
                l += 1;
                tries = 0;
            }
            assert!(l < 100_000, "cannot build {n} distinct keys");
        }
    }
    v
}

/// present keys + guaranteed-absent keys, drawn from one distinct pool
fn bin_keys_split(rng: &mut Rng, n: usize, extra: usize, keysize: usize) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut all = bin_keys(rng, n + extra, keysize);
    let absent = all.split_off(n);
    (all, absent)
}

fn str_keys_split(rng: &mut Rng, n: usize, extra: usize, lens: &[usize]) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut all = str_keys(rng, n + extra, lens);
    let absent = all.split_off(n);
    (all, absent)
}

// ===========================================================================
// F. binary-keyed maps  (CONFIGS rows 37–56)
// ===========================================================================

/// put every key, read every key back, then read a batch of absent keys.
fn put_get_case(label: &str, es: usize, keysize: usize, n: usize, rng_seed: u64) {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(rng_seed);
    let (mut keys, mut absent) = bin_keys_split(&mut rng, n, 8, keysize);
    let mut p = Pair::new(c, r, es, KeyKind::Raw, label);
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            let idx = p.put(k.as_mut_ptr(), keysize, STBDS_HM_BINARY, (i as u8).wrapping_mul(17));
            assert_eq!(idx, i as isize, "{label}: insert index");
        }
        for (i, k) in keys.iter_mut().enumerate() {
            assert_eq!(
                p.get(k.as_mut_ptr(), keysize, STBDS_HM_BINARY),
                i as isize,
                "{label}: lookup of key #{i}"
            );
            assert_eq!(p.get_ts(k.as_mut_ptr(), keysize, STBDS_HM_BINARY), i as isize);
        }
        for k in absent.iter_mut() {
            // absent keys may coincide with a present one only if bin_keys
            // produced a duplicate, which it cannot.
            assert_eq!(p.get(k.as_mut_ptr(), keysize, STBDS_HM_BINARY), -1);
            assert_eq!(p.get_ts(k.as_mut_ptr(), keysize, STBDS_HM_BINARY), -1);
        }
        p.free();
    }
}

#[test]
fn cfg37_38_39_binary_small_counts() {
    // 1 key, below the growth threshold, exactly at it
    for n in [1usize, 2, 5, 6, 7] {
        put_get_case(&format!("es=8 ks=4 n={n}"), 8, 4, n, 100 + n as u64);
    }
}

#[test]
fn cfg40_binary_multiple_growths() {
    for n in [12usize, 13, 24, 25, 48, 49, 100, 200] {
        put_get_case(&format!("es=8 ks=4 n={n}"), 8, 4, n, 200 + n as u64);
    }
}

#[test]
fn cfg41_binary_es16_ks8() {
    for n in [0usize, 1, 3, 6, 12, 17, 33, 64] {
        put_get_case(&format!("es=16 ks=8 n={n}"), 16, 8, n, 300 + n as u64);
    }
}

#[test]
fn cfg42_binary_es20_ks8() {
    // {int key[2]; int b,c,d;} -- element size is not a multiple of 8
    for n in [0usize, 1, 6, 13, 40] {
        put_get_case(&format!("es=20 ks=8 n={n}"), 20, 8, n, 400 + n as u64);
    }
}

#[test]
fn cfg43_binary_odd_keysizes() {
    for ks in [1usize, 2, 3, 4, 5, 6, 7, 8, 9, 12, 16, 17] {
        for es in [ks, ks + 1, ks + 8] {
            put_get_case(&format!("es={es} ks={ks}"), es, ks, 20, 500 + (ks * 100 + es) as u64);
        }
    }
}

#[test]
fn cfg44_binary_duplicate_puts() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(600);
    let mut keys = bin_keys(&mut rng, 10, 8);
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "duplicate puts");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, i as u8);
        }
        // re-put every key several times: no growth, index unchanged
        for round in 0..3u8 {
            for (i, k) in keys.iter_mut().enumerate() {
                let idx = p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, round.wrapping_mul(31) ^ i as u8);
                assert_eq!(idx, i as isize, "re-put must return the existing index");
            }
        }
        assert_eq!(p.c.len(), 10, "no new elements");
        p.free();
    }
}

/// Find `n` distinct `keysize`-byte keys whose (fixed-up) hash has
/// `hash & mask == want`, so that they all probe the same bucket.
fn colliding_bin_keys(api: &Api, seed: usize, keysize: usize, mask: usize, want: usize, n: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cand: u64 = 1;
    while out.len() < n {
        let mut k = vec![0u8; keysize];
        let b = cand.to_le_bytes();
        for i in 0..keysize.min(8) {
            k[i] = b[i];
        }
        cand += 1;
        assert!(cand < 50_000_000, "could not find enough colliding keys");
        let mut h = unsafe { (api.hash_bytes)(k.as_ptr() as *mut c_void, keysize, seed) };
        if h < 2 {
            h += 2;
        }
        if h & mask == want && !out.contains(&k) {
            out.push(k);
        }
    }
    out
}

#[test]
fn cfg45_binary_bucket_collisions() {
    // Every key probes slot 5 of bucket 0 at slot_count 8 and 16, so the
    // wrap-around (`limit`) loop and the `pos += step` re-probe both run.
    const SEED: usize = 0x3141_5926;
    let (_g, c, r) = scenario(SEED);
    let mut keys = colliding_bin_keys(c, SEED, 8, 15, 5, 60);
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "bucket collisions");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, i as u8);
        }
        for (i, k) in keys.iter_mut().enumerate() {
            assert_eq!(p.get(k.as_mut_ptr(), 8, STBDS_HM_BINARY), i as isize);
        }
        // delete them all (heavy tombstone traffic in one bucket)
        for k in keys.iter_mut() {
            assert_eq!(p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_BINARY), 1);
        }
        assert_eq!(p.c.len(), 0);
        p.free();
    }
}

#[test]
fn cfg46_47_get_on_empty_and_indexless_maps() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut key = 0x1122_3344_5566_7788u64.to_le_bytes();
    // (a) NULL map
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "get on NULL map");
    unsafe {
        assert_eq!(p.get(key.as_mut_ptr(), 8, STBDS_HM_BINARY), -1);
        assert_eq!(p.get_ts(key.as_mut_ptr(), 8, STBDS_HM_BINARY), -1);
        p.free();
    }
    // (b) map with elements but no hash index (built by hmput_default)
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "get on index-less map");
    unsafe {
        p.put_default();
        assert_eq!(p.get(key.as_mut_ptr(), 8, STBDS_HM_BINARY), -1);
        assert_eq!(p.get_ts(key.as_mut_ptr(), 8, STBDS_HM_BINARY), -1);
        p.free();
    }
}

#[test]
fn cfg48_del_last_element() {
    // deleting the most recently inserted key hits `old_index == final_index`
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(700);
    let mut keys = bin_keys(&mut rng, 20, 8);
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "del last element");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, i as u8);
        }
        for k in keys.iter_mut().rev() {
            assert_eq!(p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_BINARY), 1);
        }
        assert_eq!(p.c.len(), 0);
        p.free();
    }
}

#[test]
fn cfg49_del_swap_with_last() {
    // deleting in insertion order hits `old_index != final_index` every time
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(701);
    let mut keys = bin_keys(&mut rng, 20, 8);
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "del swap with last");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, i as u8);
        }
        for k in keys.iter_mut() {
            assert_eq!(p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_BINARY), 1);
        }
        p.free();
    }
}

#[test]
fn cfg50_del_then_reput_reuses_tombstone() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(702);
    let mut keys = bin_keys(&mut rng, 12, 8);
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "tombstone reuse");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, i as u8);
        }
        for round in 0..4u8 {
            for k in keys.iter_mut() {
                p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_BINARY);
                p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, round);
            }
        }
        p.free();
    }
}

#[test]
fn cfg51_52_del_rebuild_and_shrink() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(703);
    let mut keys = bin_keys(&mut rng, 60, 8);
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "rebuild + shrink");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, i as u8);
        }
        let big = p.slot_count();
        assert!(big >= 64, "expected the table to have grown, slot_count={big}");
        let mut saw_rebuild = false;
        let mut saw_shrink = false;
        let mut prev_slots = big;
        for k in keys.iter_mut() {
            let prev_tomb = p.tombstones();
            p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_BINARY);
            let now = p.slot_count();
            if now < prev_slots {
                saw_shrink = true;
            } else if p.tombstones() == 0 && prev_tomb > 0 {
                saw_rebuild = true;
            }
            prev_slots = now;
        }
        assert!(saw_shrink, "the shrink branch (lib.c:854) was never taken");
        assert!(saw_rebuild, "the tombstone-rebuild branch (lib.c:858) was never taken");
        assert_eq!(p.slot_count(), 8, "shrinks all the way back to 8 slots");
        p.free();
    }
}

#[test]
fn cfg53_del_down_to_empty_then_reuse() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(704);
    let mut keys = bin_keys(&mut rng, 30, 8);
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "empty then reuse");
    unsafe {
        for pass in 0..3u8 {
            for (i, k) in keys.iter_mut().enumerate() {
                p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, pass ^ i as u8);
            }
            for k in keys.iter_mut() {
                p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_BINARY);
            }
            assert_eq!(p.c.len(), 0);
        }
        p.free();
    }
}

#[test]
fn cfg54_binary_randomized_workload() {
    for (case, es, ks) in [(0u64, 16usize, 8usize), (1, 8, 4), (2, 20, 8), (3, 24, 16)] {
        let (_g, c, r) = scenario(0x3141_5926);
        let mut rng = Rng::new(800 + case);
        let mut keys = bin_keys(&mut rng, 40, ks);
        let mut live: Vec<bool> = vec![false; keys.len()];
        let mut p = Pair::new(c, r, es, KeyKind::Raw, &format!("random workload es={es} ks={ks}"));
        unsafe {
            for step in 0..300usize {
                let i = rng.below(keys.len());
                let k = keys[i].as_mut_ptr();
                match rng.below(4) {
                    0 | 1 => {
                        let idx = p.put(k, ks, STBDS_HM_BINARY, (step % 251) as u8);
                        assert!(idx >= 0);
                        live[i] = true;
                    }
                    2 => {
                        let idx = p.get(k, ks, STBDS_HM_BINARY);
                        assert_eq!(idx >= 0, live[i], "presence disagrees with the model at step {step}");
                    }
                    _ => {
                        let res = p.del(k, ks, 0, STBDS_HM_BINARY);
                        assert_eq!(res, live[i] as isize, "del result at step {step}");
                        live[i] = false;
                    }
                }
            }
            let want = live.iter().filter(|x| **x).count() as isize;
            assert_eq!(p.c.len(), want, "final length");
            p.free();
        }
    }
}

#[test]
fn cfg55_del_with_nonzero_keyoffset() {
    for keyoffset in [0usize, 4, 8] {
        let (_g, c, r) = scenario(0x3141_5926);
        let mut rng = Rng::new(900 + keyoffset as u64);
        let mut keys = bin_keys(&mut rng, 16, 8);
        let mut p = Pair::new(c, r, 16, KeyKind::Raw, &format!("del keyoffset={keyoffset}"));
        unsafe {
            for (i, k) in keys.iter_mut().enumerate() {
                p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, i as u8);
            }
            for k in keys.iter_mut() {
                p.del(k.as_mut_ptr(), 8, keyoffset, STBDS_HM_BINARY);
            }
            p.free();
        }
    }
}

#[test]
fn cfg56_hmfree_variants() {
    let (_g, c, r) = scenario(0x3141_5926);
    // (a) NULL
    unsafe {
        (c.hmfree_func)(std::ptr::null_mut(), 16);
        (r.hmfree_func)(std::ptr::null_mut(), 16);
    }
    // (b) array without a hash index
    unsafe {
        let ca = (c.arrgrowf)(std::ptr::null_mut(), 16, 0, 4);
        let ra = (r.arrgrowf)(std::ptr::null_mut(), 16, 0, 4);
        (c.hmfree_func)(ca, 16);
        (r.hmfree_func)(ra, 16);
    }
    // (c) fully populated SH_NONE map
    let mut rng = Rng::new(1000);
    let mut keys = bin_keys(&mut rng, 20, 8);
    let mut p = Pair::new(c, r, 16, KeyKind::Raw, "hmfree populated");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, i as u8);
        }
        p.free();
        assert!(p.c.t.is_null() && p.r.t.is_null());
    }
}

// ===========================================================================
// G. string-keyed maps  (CONFIGS rows 30, 57–70)
// ===========================================================================

/// `shmode` = `None` for a map that `stbds_hmput_key` bootstraps itself
/// (→ `SH_DEFAULT`), or `Some(m)` for one built by `stbds_shmode_func`.
fn str_case(label: &str, es: usize, shmode: Option<c_int>, mode: c_int, n: usize, lens: &[usize], seed: u64) {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(seed);
    let (mut keys, mut absent) = str_keys_split(&mut rng, n, 6, lens);
    let mut p = match shmode {
        None => Pair::new(c, r, es, KeyKind::StrPtr, label),
        Some(m) => Pair::new_shmode(c, r, es, KeyKind::StrPtr, m, label),
    };
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            let idx = p.put_str(k.as_mut_ptr(), 8, mode, (i as u8).wrapping_mul(23));
            assert_eq!(idx, i as isize, "{label}: insert index");
        }
        for (i, k) in keys.iter_mut().enumerate() {
            assert_eq!(p.get(k.as_mut_ptr(), 8, mode), i as isize, "{label}: lookup #{i}");
            assert_eq!(p.get_ts(k.as_mut_ptr(), 8, mode), i as isize);
        }
        for k in absent.iter_mut() {
            assert_eq!(p.get(k.as_mut_ptr(), 8, mode), -1, "{label}: absent lookup");
        }
        p.free();
    }
}

const STR_LENS: &[usize] = &[0, 1, 2, 7, 8, 9, 15, 16, 31, 63];

#[test]
fn cfg57_string_sh_default_implicit() {
    for n in [1usize, 2, 5, 6, 7, 20, 50] {
        str_case(
            &format!("SH_DEFAULT implicit n={n}"),
            16,
            None,
            STBDS_HM_STRING,
            n,
            STR_LENS,
            1100 + n as u64,
        );
    }
}

#[test]
fn cfg58_string_sh_strdup() {
    for n in [1usize, 5, 6, 20, 50] {
        str_case(
            &format!("SH_STRDUP n={n}"),
            16,
            Some(SH_STRDUP),
            STBDS_HM_STRING,
            n,
            STR_LENS,
            1200 + n as u64,
        );
    }
}

#[test]
fn cfg59_string_sh_arena() {
    for n in [1usize, 5, 6, 20, 50] {
        str_case(
            &format!("SH_ARENA n={n}"),
            16,
            Some(SH_ARENA),
            STBDS_HM_STRING,
            n,
            STR_LENS,
            1300 + n as u64,
        );
    }
}

#[test]
fn cfg60_string_mode_on_sh_none_map() {
    // string.mode == SH_NONE takes the `default:` arm of the switch at
    // lib.c:785, i.e. `memcpy(elem, key, keysize)` -- it copies the first
    // `keysize` BYTES OF THE STRING into the element, not the pointer. Any
    // later lookup would strcmp() through those bytes as if they were a
    // pointer, so only puts of distinct keys are well-defined here.
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(1400);
    let mut keys = str_keys(&mut rng, 4, &[16, 20, 24, 32]);
    let mut p = Pair::new_shmode(c, r, 16, KeyKind::Raw, SH_NONE, "mode=1 on SH_NONE map");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put(k.as_mut_ptr(), 8, STBDS_HM_STRING, i as u8);
        }
        p.free();
    }
}

#[test]
fn cfg61_string_duplicate_puts_and_temp_key() {
    for shmode in [None, Some(SH_STRDUP), Some(SH_ARENA)] {
        let (_g, c, r) = scenario(0x3141_5926);
        let mut rng = Rng::new(1500);
        let mut keys = str_keys(&mut rng, 12, STR_LENS);
        let label = format!("string duplicate puts shmode={shmode:?}");
        let mut p = match shmode {
            None => Pair::new(c, r, 16, KeyKind::StrPtr, &label),
            Some(m) => Pair::new_shmode(c, r, 16, KeyKind::StrPtr, m, &label),
        };
        unsafe {
            for (i, k) in keys.iter_mut().enumerate() {
                p.put_str(k.as_mut_ptr(), 8, STBDS_HM_STRING, i as u8);
            }
            for round in 0..3u8 {
                for (i, k) in keys.iter_mut().enumerate() {
                    let idx = p.put_str(k.as_mut_ptr(), 8, STBDS_HM_STRING, round ^ i as u8);
                    assert_eq!(idx, i as isize, "re-put index");
                }
            }
            assert_eq!(p.c.len(), 12);
            p.free();
        }
    }
}

#[test]
fn cfg62_30_string_key_lengths_and_arena_blocks() {
    // key lengths straddling the 512-byte arena block size drive
    // stbds_stralloc's three paths through stbds_hmput_key
    for shmode in [None, Some(SH_STRDUP), Some(SH_ARENA)] {
        for lens in [
            &[0usize][..],
            &[1][..],
            &[7, 8, 9][..],
            &[15, 16, 17][..],
            &[63, 64][..],
            &[500, 511, 512, 513][..],
            &[1000, 4096][..],
            &[511, 512, 513, 1, 2, 900][..],
        ] {
            let label = format!("key lens {lens:?} shmode={shmode:?}");
            str_case(&label, 16, shmode, STBDS_HM_STRING, 12, lens, 1600 + lens[0] as u64);
        }
    }
}

#[test]
fn cfg63_string_del_both_index_cases() {
    for shmode in [None, Some(SH_STRDUP), Some(SH_ARENA)] {
        for reverse in [false, true] {
            let (_g, c, r) = scenario(0x3141_5926);
            let mut rng = Rng::new(1700);
            let mut keys = str_keys(&mut rng, 20, STR_LENS);
            let label = format!("string del shmode={shmode:?} reverse={reverse}");
            let mut p = match shmode {
                None => Pair::new(c, r, 16, KeyKind::StrPtr, &label),
                Some(m) => Pair::new_shmode(c, r, 16, KeyKind::StrPtr, m, &label),
            };
            unsafe {
                for (i, k) in keys.iter_mut().enumerate() {
                    p.put_str(k.as_mut_ptr(), 8, STBDS_HM_STRING, i as u8);
                }
                if reverse {
                    for k in keys.iter_mut().rev() {
                        assert_eq!(p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_STRING), 1);
                    }
                } else {
                    for k in keys.iter_mut() {
                        assert_eq!(p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_STRING), 1);
                    }
                }
                assert_eq!(p.c.len(), 0);
                p.free();
            }
        }
    }
}

#[test]
fn cfg64_strdup_map_deleted_with_mode2_does_not_free() {
    // lib.c:836 tests `mode == STBDS_HM_STRING`, so mode == 2 skips the free.
    // The key is leaked -- identically in both implementations.
    //
    // Deletion happens in REVERSE insertion order so that
    // `old_index == final_index` and the index fix-up at lib.c:839-851 is
    // skipped: with mode == 2 that fix-up hashes the raw element bytes as a
    // string and trips `assert(slot >= 0)`. That abort is a real part of the
    // surface and is covered (differentially, in a subprocess) by
    // `err32_del_mode2_refind_aborts` in tests/diff_errors.rs.
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(1800);
    let mut keys = str_keys(&mut rng, 10, STR_LENS);
    let mut p = Pair::new_shmode(c, r, 16, KeyKind::StrPtr, SH_STRDUP, "STRDUP del with mode=2");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put_str(k.as_mut_ptr(), 8, STBDS_HM_STRING, i as u8);
        }
        for k in keys.iter_mut().rev() {
            assert_eq!(p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_PTR_TO_STRING), 1);
        }
        assert_eq!(p.c.len(), 0);
        p.free();
    }
}

#[test]
fn cfg65_mode_ptr_to_string() {
    for n in [1usize, 6, 20] {
        str_case(
            &format!("mode=2 n={n}"),
            16,
            None,
            STBDS_HM_PTR_TO_STRING,
            n,
            STR_LENS,
            1900 + n as u64,
        );
    }
}

#[test]
fn cfg66_mode_out_of_range_positive() {
    for mode in [3i32, 4, 99, 0x7FFF, i32::MAX] {
        str_case(
            &format!("mode={mode} (string path)"),
            16,
            None,
            mode,
            12,
            STR_LENS,
            2000 + (mode as u32 % 1000) as u64,
        );
    }
}

#[test]
fn cfg67_mode_out_of_range_negative() {
    // mode < STBDS_HM_STRING -> the *binary* path, whatever the value
    for mode in [-1i32, -2, -1000, i32::MIN] {
        let (_g, c, r) = scenario(0x3141_5926);
        let mut rng = Rng::new(2100 + (mode as u32 % 1000) as u64);
        let (mut keys, mut absent) = bin_keys_split(&mut rng, 16, 4, 8);
        let mut p = Pair::new(c, r, 16, KeyKind::Raw, &format!("mode={mode} (binary path)"));
        unsafe {
            for (i, k) in keys.iter_mut().enumerate() {
                assert_eq!(p.put(k.as_mut_ptr(), 8, mode, i as u8), i as isize);
            }
            for (i, k) in keys.iter_mut().enumerate() {
                assert_eq!(p.get(k.as_mut_ptr(), 8, mode), i as isize);
            }
            for k in absent.iter_mut() {
                assert_eq!(p.get(k.as_mut_ptr(), 8, mode), -1);
            }
            for k in keys.iter_mut() {
                assert_eq!(p.del(k.as_mut_ptr(), 8, 0, mode), 1);
            }
            p.free();
        }
    }
}

#[test]
fn cfg68_string_randomized_workload() {
    for (case, shmode) in [(0u64, None), (1, Some(SH_DEFAULT)), (2, Some(SH_STRDUP)), (3, Some(SH_ARENA))] {
        let (_g, c, r) = scenario(0x3141_5926);
        let mut rng = Rng::new(2200 + case);
        let mut keys = str_keys(&mut rng, 40, &[0, 1, 5, 8, 17, 40, 200, 600]);
        let mut live: Vec<bool> = vec![false; keys.len()];
        let label = format!("string random workload shmode={shmode:?}");
        let mut p = match shmode {
            None => Pair::new(c, r, 16, KeyKind::StrPtr, &label),
            Some(m) => Pair::new_shmode(c, r, 16, KeyKind::StrPtr, m, &label),
        };
        unsafe {
            for step in 0..300usize {
                let i = rng.below(keys.len());
                let k = keys[i].as_mut_ptr();
                match rng.below(4) {
                    0 | 1 => {
                        assert!(p.put_str(k, 8, STBDS_HM_STRING, (step % 251) as u8) >= 0);
                        live[i] = true;
                    }
                    2 => {
                        assert_eq!(p.get(k, 8, STBDS_HM_STRING) >= 0, live[i], "step {step}");
                    }
                    _ => {
                        assert_eq!(p.del(k, 8, 0, STBDS_HM_STRING), live[i] as isize, "step {step}");
                        live[i] = false;
                    }
                }
            }
            let want = live.iter().filter(|x| **x).count() as isize;
            assert_eq!(p.c.len(), want);
            p.free();
        }
    }
}

#[test]
fn cfg69_hmfree_string_modes() {
    for shmode in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        let (_g, c, r) = scenario(0x3141_5926);
        let mut rng = Rng::new(2300);
        let mut keys = str_keys(&mut rng, 25, &[1, 8, 40, 600, 1200]);
        let label = format!("hmfree shmode={shmode:?}");
        let mut p = match shmode {
            None => Pair::new(c, r, 16, KeyKind::StrPtr, &label),
            Some(m) => Pair::new_shmode(c, r, 16, KeyKind::StrPtr, m, &label),
        };
        unsafe {
            for (i, k) in keys.iter_mut().enumerate() {
                p.put_str(k.as_mut_ptr(), 8, STBDS_HM_STRING, i as u8);
            }
            // delete a few first so hmfree_func also walks a shortened array
            for k in keys.iter_mut().take(5) {
                p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_STRING);
            }
            p.free();
        }
    }
}

/// Distinct NUL-terminated strings whose (fixed-up) `stbds_hash_string` value
/// satisfies `hash & mask == want`.
fn colliding_str_keys(api: &Api, seed: usize, mask: usize, want: usize, n: usize) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut cand: u64 = 0;
    while out.len() < n {
        let mut s = format!("k{cand}").into_bytes();
        s.push(0);
        cand += 1;
        assert!(cand < 50_000_000, "could not find enough colliding string keys");
        let mut h = unsafe { (api.hash_string)(s.as_ptr() as *mut std::ffi::c_char, seed) };
        if h < 2 {
            h += 2;
        }
        if h & mask == want {
            out.push(s);
        }
    }
    out
}

#[test]
fn cfg70_string_bucket_collisions() {
    const SEED: usize = 0x3141_5926;
    let (_g, c, r) = scenario(SEED);
    let mut keys = colliding_str_keys(c, SEED, 15, 5, 50);
    let mut p = Pair::new(c, r, 16, KeyKind::StrPtr, "string bucket collisions");
    unsafe {
        for (i, k) in keys.iter_mut().enumerate() {
            p.put_str(k.as_mut_ptr(), 8, STBDS_HM_STRING, i as u8);
        }
        for (i, k) in keys.iter_mut().enumerate() {
            assert_eq!(p.get(k.as_mut_ptr(), 8, STBDS_HM_STRING), i as isize);
        }
        for k in keys.iter_mut() {
            assert_eq!(p.del(k.as_mut_ptr(), 8, 0, STBDS_HM_STRING), 1);
        }
        p.free();
    }
}
