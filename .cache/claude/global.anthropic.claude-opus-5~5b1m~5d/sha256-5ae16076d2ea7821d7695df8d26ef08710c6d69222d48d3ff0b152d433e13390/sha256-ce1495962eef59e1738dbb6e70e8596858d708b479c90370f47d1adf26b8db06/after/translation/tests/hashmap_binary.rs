//! Phase B, Group 5 of `CONFIGS.md`: the BINARY hash map
//! (`mode == STBDS_HM_BINARY`) driven through `stbds_hmput_key`,
//! `stbds_hmget_key`, `stbds_hmget_key_ts`, `stbds_hmput_default`,
//! `stbds_hmdel_key`, `stbds_hmfree_func`.

mod common;

use common::*;
use core::ffi::c_void;
use std::collections::HashSet;

/// `{ int key; int value; }`
const S_INT: Spec = Spec::bytes(8, 4);

fn k4(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}
fn v4(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

struct Duo<'a> {
    c: Map<'a>,
    r: Map<'a>,
}

impl<'a> Duo<'a> {
    fn new(p: &'a Pair, spec: Spec, mode: i32, seed: usize) -> Duo<'a> {
        reset_seed(p, seed);
        Duo {
            c: Map::new(&p.c, spec, mode),
            r: Map::new(&p.r, spec, mode),
        }
    }
    fn new_shmode(p: &'a Pair, spec: Spec, mode: i32, sh: i32, seed: usize) -> Duo<'a> {
        reset_seed(p, seed);
        Duo {
            c: Map::new_shmode(&p.c, spec, mode, sh),
            r: Map::new_shmode(&p.r, spec, mode, sh),
        }
    }
    fn put(&mut self, key: &[u8], value: &[u8]) -> isize {
        let a = self.c.hmput(key, value);
        let b = self.r.hmput(key, value);
        diff_eq!(a, b, "hmput index");
        a
    }
    fn geti(&mut self, key: &[u8]) -> isize {
        let a = self.c.hmgeti(key);
        let b = self.r.hmgeti(key);
        diff_eq!(a, b, "hmgeti index");
        a
    }
    fn geti_ts(&mut self, key: &[u8]) -> isize {
        let a = self.c.hmgeti_ts(key.as_ptr() as *mut c_void);
        let b = self.r.hmgeti_ts(key.as_ptr() as *mut c_void);
        diff_eq!(a, b, "hmgeti_ts index");
        a
    }
    fn del(&mut self, key: &[u8], keyoffset: usize) -> isize {
        let a = self.c.hmdel(key.as_ptr() as *mut c_void, keyoffset);
        let b = self.r.hmdel(key.as_ptr() as *mut c_void, keyoffset);
        diff_eq!(a, b, "hmdel result");
        a
    }
    fn check(&self, ctx: &str) {
        diff_eq!(self.c.snap(), self.r.snap(), "{ctx}");
        diff_eq!(self.c.hmlen(), self.r.hmlen(), "{ctx} hmlen");
    }
    fn free(&mut self) {
        self.c.hmfree();
        self.r.hmfree();
    }
}

// ---------------------------------------------------------------------------
// C28 / C29 / C30 / C31 / C32 — element counts around the grow thresholds
// ---------------------------------------------------------------------------
#[test]
fn cfg_c28_to_c32_counts() {
    let p = libs();
    for n in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 12, 13, 14, 24, 25, 26, 48, 49, 50] {
        let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
        d.check(&format!("n={n} empty"));
        for i in 0..n {
            let idx = d.put(&k4(i as i32), &v4(i as i32 * 7 + 1));
            assert_eq!(idx, i as isize, "insert order index");
            d.check(&format!("n={n} after put {i}"));
        }
        // read everything back
        for i in 0..n {
            let g = d.geti(&k4(i as i32));
            assert_eq!(g, i as isize, "get should find key {i}");
            d.check(&format!("n={n} after get {i}"));
        }
        // absent keys
        for i in n..n + 5 {
            let g = d.geti(&k4(i as i32));
            assert_eq!(g, -1, "absent key {i} must yield -1");
            d.check(&format!("n={n} after miss {i}"));
        }
        d.free();
    }
}

// ---------------------------------------------------------------------------
// C33 — 1000 random distinct keys
// ---------------------------------------------------------------------------
#[test]
fn cfg_c33_many_random_keys() {
    let p = libs();
    let mut rng = Rng::new(33);
    let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
    let mut keys: Vec<i32> = Vec::new();
    let mut seen = HashSet::new();
    while keys.len() < 1000 {
        let k = rng.next_u32() as i32;
        if seen.insert(k) {
            keys.push(k);
        }
    }
    for (i, &k) in keys.iter().enumerate() {
        d.put(&k4(k), &v4(i as i32));
        if i % 37 == 0 || i > 990 {
            d.check(&format!("bulk put {i}"));
        }
    }
    d.check("bulk put final");
    for (i, &k) in keys.iter().enumerate() {
        let g = d.geti(&k4(k));
        assert_eq!(g, i as isize, "key {k} at index {i}");
    }
    d.check("bulk get final");
    d.free();
}

// ---------------------------------------------------------------------------
// C34 — duplicate keys / re-put
// ---------------------------------------------------------------------------
#[test]
fn cfg_c34_duplicate_keys() {
    let p = libs();
    let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
    for i in 0..30i32 {
        d.put(&k4(i), &v4(i));
    }
    d.check("30 inserted");
    // re-put each key with a new value: length must not change
    for i in 0..30i32 {
        let idx = d.put(&k4(i), &v4(1000 + i));
        assert_eq!(idx, i as isize, "re-put must reuse the slot");
        d.check(&format!("re-put {i}"));
    }
    assert_eq!(d.c.hmlen(), 30);
    // interleave duplicates and new keys
    let mut rng = Rng::new(34);
    for step in 0..300 {
        let k = rng.below(45) as i32;
        d.put(&k4(k), &v4(rng.next_u32() as i32));
        d.check(&format!("interleave step {step}"));
    }
    d.free();
}

// ---------------------------------------------------------------------------
// C35 — present/absent lookups interleaved
// ---------------------------------------------------------------------------
#[test]
fn cfg_c35_present_absent() {
    let p = libs();
    let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
    for i in 0..40i32 {
        d.put(&k4(i * 2), &v4(i));
    }
    for i in -5..90i32 {
        let g = d.geti(&k4(i));
        let expect = if i >= 0 && i % 2 == 0 && i / 2 < 40 {
            i as isize / 2
        } else {
            -1
        };
        assert_eq!(g, expect, "geti({i})");
        d.check(&format!("geti({i})"));
    }
    d.free();
}

// ---------------------------------------------------------------------------
// C36 — hmget_key_ts (out-param) leaves header->temp alone
// ---------------------------------------------------------------------------
#[test]
fn cfg_c36_hmget_key_ts() {
    let p = libs();
    let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
    for i in 0..25i32 {
        d.put(&k4(i), &v4(i));
    }
    // Pin header->temp to a known value with a get, then verify _ts doesn't change it.
    let pinned = d.geti(&k4(7));
    assert_eq!(pinned, 7);
    for i in -3..30i32 {
        let ts = d.geti_ts(&k4(i));
        let expect = if (0..25).contains(&i) { i as isize } else { -1 };
        assert_eq!(ts, expect, "geti_ts({i})");
        diff_eq!(d.c.temp(), d.r.temp(), "header temp after ts get {i}");
        assert_eq!(d.c.temp(), pinned, "hmget_key_ts must not touch header->temp");
        d.check(&format!("ts {i}"));
    }
    d.free();
}

// ---------------------------------------------------------------------------
// C37 — hmput_default / hmdefault
// ---------------------------------------------------------------------------
#[test]
fn cfg_c37_hmdefault() {
    let p = libs();
    // on a NULL map
    let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
    d.c.hmdefault(&v4(-999));
    d.r.hmdefault(&v4(-999));
    d.check("hmdefault on NULL map");
    // then insert and read back
    for i in 0..15i32 {
        d.put(&k4(i), &v4(i * 3));
        d.check(&format!("after default, put {i}"));
    }
    // hmdefault again on a live map must be a no-op except for the value write
    d.c.hmdefault(&v4(-1234));
    d.r.hmdefault(&v4(-1234));
    d.check("hmdefault on live map");
    // hmgetp of a missing key -> index -1 -> element [-1] is the default
    let g = d.geti(&k4(9999));
    assert_eq!(g, -1);
    d.check("miss after default");
    d.free();

    // hmdefaults (whole struct)
    let mut d2 = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
    let whole: Vec<u8> = vec![0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44];
    d2.c.hmdefaults(&whole);
    d2.r.hmdefaults(&whole);
    d2.check("hmdefaults on NULL map");
    for i in 0..8i32 {
        d2.put(&k4(i), &v4(i));
    }
    d2.check("hmdefaults then puts");
    d2.free();
}

// ---------------------------------------------------------------------------
// C38 / C39 — deletion basics
// ---------------------------------------------------------------------------
#[test]
fn cfg_c38_c39_delete() {
    let p = libs();
    // delete one of ten
    for target in 0..10i32 {
        let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
        for i in 0..10i32 {
            d.put(&k4(i), &v4(i * 5));
        }
        let r = d.del(&k4(target), 0);
        assert_eq!(r, 1, "delete of present key reports 1");
        d.check(&format!("deleted {target}"));
        assert_eq!(d.c.hmlen(), 9);
        // everything else still findable
        for i in 0..10i32 {
            let g = d.geti(&k4(i));
            if i == target {
                assert_eq!(g, -1, "deleted key must be gone");
            } else {
                assert!(g >= 0, "key {i} must survive");
            }
            d.check(&format!("post-del get {i}"));
        }
        // deleting again reports 0
        let r2 = d.del(&k4(target), 0);
        assert_eq!(r2, 0);
        d.check("double delete");
        // deleting a never-present key reports 0
        let r3 = d.del(&k4(12345), 0);
        assert_eq!(r3, 0);
        d.check("delete missing");
        d.free();
    }

    // drain completely, then refill
    let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
    for i in 0..10i32 {
        d.put(&k4(i), &v4(i));
    }
    for i in 0..10i32 {
        d.del(&k4(i), 0);
        d.check(&format!("drain {i}"));
    }
    assert_eq!(d.c.hmlen(), 0);
    d.check("drained");
    for i in 0..10i32 {
        d.put(&k4(100 + i), &v4(i));
        d.check(&format!("refill {i}"));
    }
    d.free();

    // delete in reverse insertion order (always the physically-last element)
    let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
    for i in 0..20i32 {
        d.put(&k4(i), &v4(i));
    }
    for i in (0..20i32).rev() {
        d.del(&k4(i), 0);
        d.check(&format!("reverse drain {i}"));
    }
    d.free();
}

// ---------------------------------------------------------------------------
// C40 — shrink chain 64 -> 32 -> 16 -> 8
// ---------------------------------------------------------------------------
#[test]
fn cfg_c40_shrink_chain() {
    let p = libs();
    let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
    for i in 0..40i32 {
        d.put(&k4(i), &v4(i));
    }
    d.check("40 inserted");
    let sc = d.c.snap().idx.slot_count;
    assert_eq!(sc, 64, "40 inserts should land in a 64-slot table");
    let mut counts = Vec::new();
    for i in 0..40i32 {
        d.del(&k4(i), 0);
        let s = d.c.snap();
        counts.push(s.idx.slot_count);
        d.check(&format!("shrink drain {i}"));
    }
    // must have shrunk all the way to the 8-slot floor
    assert_eq!(*counts.last().unwrap(), 8, "should shrink to 8: {counts:?}");
    assert!(counts.contains(&32) && counts.contains(&16));
    d.free();
}

// ---------------------------------------------------------------------------
// C41 — churn from a small key pool (tombstone rebuild + reuse)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c41_churn() {
    let p = libs();
    for (pool, seed) in [(8usize, 1u64), (16, 2), (64, 3), (200, 4)] {
        let mut rng = Rng::new(seed);
        let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
        for step in 0..2000 {
            let k = rng.below(pool) as i32;
            match rng.below(10) {
                0..=4 => {
                    d.put(&k4(k), &v4(step as i32));
                }
                5..=7 => {
                    d.del(&k4(k), 0);
                }
                8 => {
                    d.geti(&k4(k));
                }
                _ => {
                    d.geti_ts(&k4(k));
                }
            }
            if step % 13 == 0 {
                d.check(&format!("pool={pool} step={step}"));
            }
        }
        d.check(&format!("pool={pool} final"));
        d.free();
    }
}

// ---------------------------------------------------------------------------
// C42 / C43 / C44 / C45 — other element/key shapes
// ---------------------------------------------------------------------------
fn shape_exercise(p: &Pair, spec: Spec, label: &str, nkeys: usize, seed: u64) {
    let mut rng = Rng::new(seed);
    let mut d = Duo::new(p, spec, STBDS_HM_BINARY, DEFAULT_SEED);
    let vlen = spec.elemsize - spec.keysize;
    let mut keys: Vec<Vec<u8>> = Vec::new();
    let mut seen = HashSet::new();
    let mut guard = 0;
    while keys.len() < nkeys && guard < nkeys * 1000 {
        guard += 1;
        let k = rng.bytes(spec.keysize);
        if seen.insert(k.clone()) {
            keys.push(k);
        }
    }
    for (i, k) in keys.iter().enumerate() {
        let v = rng.bytes(vlen);
        let idx = d.put(k, &v);
        assert_eq!(idx, i as isize, "{label}: insertion index");
        d.check(&format!("{label} put {i}"));
    }
    for (i, k) in keys.iter().enumerate() {
        assert_eq!(d.geti(k), i as isize, "{label}: get {i}");
    }
    d.check(&format!("{label} all gets"));
    // delete every other
    for (i, k) in keys.iter().enumerate() {
        if i % 2 == 0 {
            d.del(k.as_slice(), 0);
            d.check(&format!("{label} del {i}"));
        }
    }
    for (i, k) in keys.iter().enumerate() {
        let g = d.geti(k);
        if i % 2 == 0 {
            assert_eq!(g, -1, "{label}: deleted {i}");
        } else {
            assert!(g >= 0, "{label}: survivor {i}");
        }
    }
    d.check(&format!("{label} final"));
    d.free();
}

#[test]
fn cfg_c42_struct2_shape() {
    let p = libs();
    // typedef struct { int key[2],b,c,d; } stbds_struct2;
    shape_exercise(&p, Spec::bytes(20, 8), "struct2(20,8)", 200, 42);
}

#[test]
fn cfg_c43_whole_elem_is_key() {
    let p = libs();
    shape_exercise(&p, Spec::bytes(16, 16), "key==elem(16,16)", 200, 43);
    shape_exercise(&p, Spec::bytes(4, 4), "key==elem(4,4)", 200, 431);
}

#[test]
fn cfg_c44_tiny_elems() {
    let p = libs();
    shape_exercise(&p, Spec::bytes(1, 1), "u8(1,1)", 120, 44);
    shape_exercise(&p, Spec::bytes(2, 1), "u8key(2,1)", 120, 441);
    shape_exercise(&p, Spec::bytes(2, 2), "u16(2,2)", 200, 442);
    shape_exercise(&p, Spec::bytes(3, 1), "odd(3,1)", 120, 443);
}

#[test]
fn cfg_c45_usize_keys() {
    let p = libs();
    shape_exercise(&p, Spec::bytes(8, 8), "usize(8,8)", 300, 45);
    shape_exercise(&p, Spec::bytes(16, 8), "usize+val(16,8)", 300, 451);
    // extreme key values
    let mut d = Duo::new(&p, Spec::bytes(16, 8), STBDS_HM_BINARY, DEFAULT_SEED);
    for k in [0usize, 1, 2, usize::MAX, usize::MAX - 1, 1 << 63, (1 << 63) | 1] {
        d.put(&k.to_ne_bytes(), &[0xEE; 8]);
        d.check(&format!("extreme key {k:#x}"));
    }
    for k in [0usize, 1, 2, usize::MAX, usize::MAX - 1, 1 << 63, (1 << 63) | 1] {
        assert!(d.geti(&k.to_ne_bytes()) >= 0);
    }
    d.check("extremes final");
    d.free();
}

// ---------------------------------------------------------------------------
// C46 — the `hash < 2` branch (E45), forced through hash_string
// ---------------------------------------------------------------------------
#[test]
fn cfg_c46_hash_below_2() {
    let p = libs();
    // hash_string("", seed) == K + seed, so pick seed = target - K.
    let empty = [0u8];
    let k_c = unsafe { (p.c.hash_string)(empty.as_ptr() as *mut _, 0) };
    let k_r = unsafe { (p.r.hash_string)(empty.as_ptr() as *mut _, 0) };
    diff_eq!(k_c, k_r, "hash_string(\"\",0) constant");

    for target in [0usize, 1] {
        let seed = target.wrapping_sub(k_c);
        // sanity: the hash really is < 2 for this seed
        let h = unsafe { (p.c.hash_string)(empty.as_ptr() as *mut _, seed) };
        assert_eq!(h, target, "seed {seed:#x} should make hash_string(\"\") == {target}");

        let spec = Spec::ptr(16);
        let mut d = Duo::new_shmode(&p, spec, STBDS_HM_STRING, STBDS_SH_DEFAULT, seed);
        assert_eq!(d.c.snap().idx.seed, seed, "table must use the seed we set");
        let mut keys = Keys::new();
        let ek = keys.add(b"");
        // put with the hash-0/hash-1 key
        let a = d.c.shput(ek, &[1u8, 2, 3, 4, 5, 6, 7, 8]);
        let b = d.r.shput(ek, &[1u8, 2, 3, 4, 5, 6, 7, 8]);
        diff_eq!(a, b, "shput of hash<2 key");
        let s = d.c.snap();
        // hash stored in the bucket must be target+2
        let stored: Vec<usize> = s.idx.slots.iter().map(|x| x.0).filter(|&h| h != 0).collect();
        assert_eq!(stored, vec![target + 2], "hash<2 must be bumped by 2");
        d.check(&format!("hash<2 target={target}"));
        // and the key must be findable through hm_find_slot too
        let g1 = d.c.shgeti(ek);
        let g2 = d.r.shgeti(ek);
        diff_eq!(g1, g2, "shgeti of hash<2 key");
        assert_eq!(g1, 0);
        d.check(&format!("hash<2 get target={target}"));
        // and deletable
        let d1 = d.c.hmdel(ek as *mut c_void, 0);
        let d2 = d.r.hmdel(ek as *mut c_void, 0);
        diff_eq!(d1, d2, "shdel of hash<2 key");
        assert_eq!(d1, 1);
        d.check(&format!("hash<2 del target={target}"));
        d.free();
    }
    reset_seed(&p, DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// C47 — hmfree_func on a populated BINARY map
// ---------------------------------------------------------------------------
#[test]
fn cfg_c47_hmfree_binary() {
    let p = libs();
    for n in [0usize, 1, 7, 40, 500] {
        let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, DEFAULT_SEED);
        for i in 0..n {
            d.put(&k4(i as i32), &v4(i as i32));
        }
        d.check(&format!("n={n} before free"));
        d.free();
        assert!(d.c.t.is_null() && d.r.t.is_null());
        // reuse after free
        for i in 0..5i32 {
            d.put(&k4(i), &v4(i));
        }
        d.check(&format!("n={n} after reuse"));
        d.free();
    }
}

// ---------------------------------------------------------------------------
// C48 — everything again under several global seeds
// ---------------------------------------------------------------------------
#[test]
fn cfg_c48_seed_matrix() {
    let p = libs();
    for &seed in [0usize, 1, 0xdead_beef, usize::MAX, 0x3141_5926, 1 << 63].iter() {
        let mut rng = Rng::new(seed as u64 ^ 0xABCD);
        let mut d = Duo::new(&p, S_INT, STBDS_HM_BINARY, seed);
        for i in 0..120i32 {
            d.put(&k4(i), &v4(i));
        }
        d.check(&format!("seed={seed:#x} filled"));
        for step in 0..400 {
            let k = rng.below(160) as i32;
            match rng.below(4) {
                0 | 1 => {
                    d.put(&k4(k), &v4(step));
                }
                2 => {
                    d.del(&k4(k), 0);
                }
                _ => {
                    d.geti(&k4(k));
                }
            }
            if step % 17 == 0 {
                d.check(&format!("seed={seed:#x} step={step}"));
            }
        }
        d.check(&format!("seed={seed:#x} final"));
        d.free();
    }
    reset_seed(&p, DEFAULT_SEED);
}
