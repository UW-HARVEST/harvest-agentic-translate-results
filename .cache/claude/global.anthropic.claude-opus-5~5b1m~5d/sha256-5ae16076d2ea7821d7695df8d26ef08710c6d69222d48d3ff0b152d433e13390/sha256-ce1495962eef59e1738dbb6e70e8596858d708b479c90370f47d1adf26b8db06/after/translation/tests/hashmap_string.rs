//! Phase B, Groups 6-9 of `CONFIGS.md`: the STRING hash map across every
//! `string.mode` (`SH_NONE` / `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA`), driven
//! through `stbds_shmode_func`, `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmdel_key`, `stbds_hmfree_func`.

mod common;

use common::*;
use core::ffi::{c_char, c_void};
use std::collections::HashSet;

/// `{ char *key; int value; }` — 16 bytes with padding, key at offset 0.
fn spec_default() -> Spec {
    // SH_DEFAULT / SH_NONE store either the caller pointer or raw bytes; compare
    // the key field verbatim so pointer identity is checked too.
    Spec::bytes(16, 8)
}
fn spec_owned() -> Spec {
    // SH_STRDUP / SH_ARENA store library-owned copies; compare string contents.
    Spec::ptr(16)
}

struct SDuo<'a> {
    c: Map<'a>,
    r: Map<'a>,
}

impl<'a> SDuo<'a> {
    fn new_shmode(p: &'a Pair, spec: Spec, mode: i32, sh: i32, seed: usize) -> SDuo<'a> {
        reset_seed(p, seed);
        SDuo {
            c: Map::new_shmode(&p.c, spec, mode, sh),
            r: Map::new_shmode(&p.r, spec, mode, sh),
        }
    }
    /// Implicit table creation (so `hmput_key` picks `string.mode` itself).
    fn new_implicit(p: &'a Pair, spec: Spec, mode: i32, seed: usize) -> SDuo<'a> {
        reset_seed(p, seed);
        SDuo {
            c: Map::new(&p.c, spec, mode),
            r: Map::new(&p.r, spec, mode),
        }
    }
    fn put(&mut self, key: *mut c_char, value: &[u8]) -> isize {
        let a = self.c.shput(key, value);
        let b = self.r.shput(key, value);
        diff_eq!(a, b, "shput index");
        a
    }
    fn puts(&mut self, key: *mut c_char, whole: &[u8]) -> isize {
        let a = self.c.shputs(key, whole);
        let b = self.r.shputs(key, whole);
        diff_eq!(a, b, "shputs index");
        a
    }
    fn geti(&mut self, key: *mut c_char) -> isize {
        let a = self.c.shgeti(key);
        let b = self.r.shgeti(key);
        diff_eq!(a, b, "shgeti index");
        a
    }
    fn geti_ts(&mut self, key: *mut c_char) -> isize {
        let a = self.c.hmgeti_ts(key as *mut c_void);
        let b = self.r.hmgeti_ts(key as *mut c_void);
        diff_eq!(a, b, "shgeti_ts index");
        a
    }
    fn del(&mut self, key: *mut c_char, keyoffset: usize) -> isize {
        let a = self.c.hmdel(key as *mut c_void, keyoffset);
        let b = self.r.hmdel(key as *mut c_void, keyoffset);
        diff_eq!(a, b, "shdel result");
        a
    }
    fn check(&self, ctx: &str) {
        diff_eq!(self.c.snap(), self.r.snap(), "{ctx}");
        diff_eq!(self.c.hmlen(), self.r.hmlen(), "{ctx} hmlen");
    }
    fn check_temp_key(&self, ctx: &str) {
        diff_eq!(
            self.c.snap_temp_key(),
            self.r.snap_temp_key(),
            "{ctx} temp_key contents"
        );
    }
    fn free(&mut self) {
        self.c.hmfree();
        self.r.hmfree();
    }
}

fn key_pool(rng: &mut Rng, keys: &mut Keys, n: usize, minlen: usize, maxlen: usize) -> Vec<*mut c_char> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut guard = 0usize;
    while out.len() < n && guard < n * 1000 {
        guard += 1;
        let len = rng.range(minlen, maxlen);
        let body = rng.cstr_body(len, false);
        if seen.insert(body.clone()) {
            out.push(keys.add(&body));
        }
    }
    assert_eq!(out.len(), n, "could not build key pool");
    out
}

// ===========================================================================
// Group 6 — implicit SH_DEFAULT
// ===========================================================================

// C49 / C50 / C51
#[test]
fn cfg_c49_c50_c51_sh_default_counts() {
    let p = libs();
    for n in [0usize, 1, 2, 5, 6, 7, 8, 12, 13, 40, 100] {
        let mut keys = Keys::new();
        let mut rng = Rng::new(49 + n as u64);
        let pool = key_pool(&mut rng, &mut keys, n.max(1), 1, 24);
        let mut d = SDuo::new_implicit(&p, spec_default(), STBDS_HM_STRING, DEFAULT_SEED);
        d.check(&format!("n={n} empty"));
        for i in 0..n {
            let idx = d.put(pool[i], &(i as u64).to_ne_bytes());
            assert_eq!(idx, i as isize);
            d.check(&format!("n={n} put {i}"));
            d.check_temp_key(&format!("n={n} put {i}"));
            if i == 0 {
                // first put created the table implicitly -> SH_DEFAULT
                assert_eq!(d.c.snap().idx.arena.mode, 1, "implicit string.mode");
            }
        }
        for i in 0..n {
            assert_eq!(d.geti(pool[i]), i as isize);
            d.check(&format!("n={n} get {i}"));
        }
        d.free();
    }

    // 500 random distinct strings
    let mut keys = Keys::new();
    let mut rng = Rng::new(51);
    let pool = key_pool(&mut rng, &mut keys, 500, 1, 40);
    let mut d = SDuo::new_implicit(&p, spec_default(), STBDS_HM_STRING, DEFAULT_SEED);
    for (i, &k) in pool.iter().enumerate() {
        d.put(k, &(i as u64).to_ne_bytes());
        if i % 29 == 0 {
            d.check(&format!("bulk put {i}"));
        }
    }
    d.check("bulk put final");
    for (i, &k) in pool.iter().enumerate() {
        assert_eq!(d.geti(k), i as isize, "bulk get {i}");
    }
    d.check("bulk get final");
    d.free();
}

// C52 — duplicate key re-put writes temp_key from the *stored* pointer
#[test]
fn cfg_c52_sh_default_dup_temp_key() {
    let p = libs();
    let mut keys = Keys::new();
    let mut rng = Rng::new(52);
    let pool = key_pool(&mut rng, &mut keys, 60, 3, 20);
    let mut d = SDuo::new_implicit(&p, spec_default(), STBDS_HM_STRING, DEFAULT_SEED);
    for (i, &k) in pool.iter().enumerate() {
        d.put(k, &(i as u64).to_ne_bytes());
    }
    d.check("filled");
    // Re-put using an *equal but distinct* buffer: the library must keep the
    // originally-stored pointer.  Note the E19/E20 asymmetry in the C: only the
    // *upper* half-scan of the probe writes `temp_key`; a duplicate found in the
    // wrap-around half-scan leaves the previous `temp_key` in place.  Both
    // libraries must reproduce that identically.
    let mut alt = Keys::new();
    let pool_set: HashSet<usize> = pool.iter().map(|&k| k as usize).collect();
    let mut fresh = 0usize;
    let mut stale = 0usize;
    for (i, &k) in pool.iter().enumerate() {
        let text = unsafe { read_cstr(k) };
        let k2 = alt.add(&text);
        let idx = d.put(k2, &(0xAA00u64 + i as u64).to_ne_bytes());
        assert_eq!(idx, i as isize, "dup re-put index");
        d.check(&format!("dup re-put {i}"));
        d.check_temp_key(&format!("dup re-put {i}"));
        unsafe {
            let ct = raw_temp_key(d.c.raw());
            let rt = raw_temp_key(d.r.raw());
            assert_eq!(ct, rt, "SH_DEFAULT temp_key pointer must be identical");
            // never the caller's fresh buffer, always a previously-stored key
            assert_ne!(ct as *const c_char, k2 as *const c_char);
            assert!(
                pool_set.contains(&(ct as usize)),
                "temp_key must point at a stored key"
            );
            if ct == k {
                fresh += 1;
            } else {
                stale += 1;
            }
        }
    }
    // Both branches of the C quirk must actually have been exercised.
    assert!(fresh > 0, "no upper-half duplicate hits were exercised");
    assert!(
        stale > 0,
        "no wrap-around duplicate hits (E20 quirk) were exercised: fresh={fresh}"
    );
    d.free();
}

// C53 — deletion of string keys
#[test]
fn cfg_c53_sh_default_delete() {
    let p = libs();
    let mut keys = Keys::new();
    let mut rng = Rng::new(53);
    let pool = key_pool(&mut rng, &mut keys, 30, 1, 16);
    for target in [0usize, 1, 15, 29] {
        let mut d = SDuo::new_implicit(&p, spec_default(), STBDS_HM_STRING, DEFAULT_SEED);
        for (i, &k) in pool.iter().enumerate() {
            d.put(k, &(i as u64).to_ne_bytes());
        }
        assert_eq!(d.del(pool[target], 0), 1);
        d.check(&format!("deleted {target}"));
        assert_eq!(d.del(pool[target], 0), 0, "double delete");
        d.check(&format!("double deleted {target}"));
        let mut absent = Keys::new();
        let ak = absent.add(b"definitely-not-present-xyzzy");
        assert_eq!(d.del(ak, 0), 0, "absent delete");
        d.check("absent delete");
        for (i, &k) in pool.iter().enumerate() {
            let g = d.geti(k);
            if i == target {
                assert_eq!(g, -1);
            } else {
                assert!(g >= 0);
            }
        }
        d.check(&format!("post-del gets {target}"));
        d.free();
    }
    // drain in reverse
    let mut d = SDuo::new_implicit(&p, spec_default(), STBDS_HM_STRING, DEFAULT_SEED);
    for (i, &k) in pool.iter().enumerate() {
        d.put(k, &(i as u64).to_ne_bytes());
    }
    for &k in pool.iter().rev() {
        d.del(k, 0);
        d.check("reverse drain");
    }
    assert_eq!(d.c.hmlen(), 0);
    d.free();
}

// C54 — 1500 randomized ops from a 64-string pool
#[test]
fn cfg_c54_sh_default_churn() {
    let p = libs();
    let mut keys = Keys::new();
    let mut rng = Rng::new(54);
    let pool = key_pool(&mut rng, &mut keys, 64, 1, 20);
    let mut d = SDuo::new_implicit(&p, spec_default(), STBDS_HM_STRING, DEFAULT_SEED);
    for step in 0..1500 {
        let k = pool[rng.below(pool.len())];
        match rng.below(10) {
            0..=4 => {
                d.put(k, &(step as u64).to_ne_bytes());
            }
            5..=7 => {
                d.del(k, 0);
            }
            8 => {
                d.geti(k);
            }
            _ => {
                d.geti_ts(k);
            }
        }
        if step % 11 == 0 {
            d.check(&format!("churn step {step}"));
        }
    }
    d.check("churn final");
    d.free();
}

// C55 — prefix/suffix/empty keys
#[test]
fn cfg_c55_prefix_suffix_empty() {
    let p = libs();
    let mut keys = Keys::new();
    let texts: Vec<&[u8]> = vec![
        b"", b"a", b"ab", b"abc", b"abcd", b"b", b"bc", b"bcd", b"abcde", b"aa", b"aaa",
        b"zzzzzzzzzzzzzzzzzzzz", b"zzzzzzzzzzzzzzzzzzz", b"\x01", b"\xff", b"\xff\xfe",
    ];
    let pool: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t)).collect();
    let mut d = SDuo::new_implicit(&p, spec_default(), STBDS_HM_STRING, DEFAULT_SEED);
    for (i, &k) in pool.iter().enumerate() {
        d.put(k, &(i as u64).to_ne_bytes());
        d.check(&format!("prefix put {i}"));
    }
    for (i, &k) in pool.iter().enumerate() {
        assert_eq!(d.geti(k), i as isize, "prefix get {i}");
        d.check(&format!("prefix get {i}"));
    }
    // lookups with fresh, equal buffers
    let mut alt = Keys::new();
    for (i, t) in texts.iter().enumerate() {
        let k2 = alt.add(t);
        assert_eq!(d.geti(k2), i as isize, "prefix get-by-copy {i}");
    }
    d.check("prefix final");
    d.free();
}

// C56 — hmfree_func on an SH_DEFAULT string map
#[test]
fn cfg_c56_hmfree_sh_default() {
    let p = libs();
    for n in [0usize, 1, 9, 100] {
        let mut keys = Keys::new();
        let mut rng = Rng::new(56 + n as u64);
        let pool = key_pool(&mut rng, &mut keys, n.max(1), 1, 20);
        let mut d = SDuo::new_implicit(&p, spec_default(), STBDS_HM_STRING, DEFAULT_SEED);
        for i in 0..n {
            d.put(pool[i], &(i as u64).to_ne_bytes());
        }
        d.check(&format!("n={n} pre-free"));
        d.free();
        for i in 0..3.min(n) {
            d.put(pool[i], &(i as u64).to_ne_bytes());
        }
        d.check(&format!("n={n} reuse"));
        d.free();
    }
}

// ===========================================================================
// Group 7 — SH_STRDUP
// ===========================================================================

// C57
#[test]
fn cfg_c57_shmode_strdup_pristine() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 16, 20, 64] {
        for sh in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            reset_seed(&p, DEFAULT_SEED);
            let spec = Spec::bytes(elemsize, elemsize.min(8));
            let mut cm = Map::new_shmode(&p.c, spec, STBDS_HM_STRING, sh);
            let mut rm = Map::new_shmode(&p.r, spec, STBDS_HM_STRING, sh);
            diff_eq!(cm.snap(), rm.snap(), "shmode_func(e={elemsize}, sh={sh}) pristine");
            assert_eq!(cm.snap().idx.arena.mode as i32, sh);
            assert_eq!(cm.snap().hdr.length, 1);
            assert_eq!(cm.snap().hdr.capacity, 4);
            assert_eq!(cm.snap().idx.slot_count, 8);
            cm.hmfree();
            rm.hmfree();
        }
    }
}

// C58 / C59 / C60 / C61
#[test]
fn cfg_c58_strdup_inserts() {
    let p = libs();
    for n in [1usize, 7, 40, 400] {
        let mut keys = Keys::new();
        let mut rng = Rng::new(58 + n as u64);
        let pool = key_pool(&mut rng, &mut keys, n, 1, 30);
        let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_STRDUP, DEFAULT_SEED);
        for i in 0..n {
            d.put(pool[i], &(i as u64).to_ne_bytes());
            if i % 7 == 0 {
                d.check(&format!("strdup n={n} put {i}"));
                d.check_temp_key(&format!("strdup n={n} put {i}"));
            }
        }
        d.check(&format!("strdup n={n} filled"));
        // the library owns its own copies: look up with fresh buffers
        let mut alt = Keys::new();
        for i in 0..n {
            let text = unsafe { read_cstr(pool[i]) };
            let k2 = alt.add(&text);
            assert_eq!(d.geti(k2), i as isize, "strdup get-by-copy {i}");
        }
        d.check(&format!("strdup n={n} gets"));
        // stored pointers must NOT be the caller's
        unsafe {
            let s = d.c.snap();
            for i in 1..s.hdr.length {
                let e = (d.c.raw() as *const u8).add(i * 16);
                let ptr = *(e as *const *const c_char);
                assert!(
                    !pool.iter().any(|&k| k as *const c_char == ptr),
                    "SH_STRDUP must store its own copy"
                );
            }
        }
        d.free();
    }
}

#[test]
fn cfg_c59_c60_strdup_delete_and_churn() {
    let p = libs();
    // delete present keys (the strdup'd key is freed)
    let mut keys = Keys::new();
    let mut rng = Rng::new(59);
    let pool = key_pool(&mut rng, &mut keys, 30, 1, 20);
    for target in [0usize, 1, 14, 29] {
        let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_STRDUP, DEFAULT_SEED);
        for i in 0..30 {
            d.put(pool[i], &(i as u64).to_ne_bytes());
        }
        assert_eq!(d.del(pool[target], 0), 1);
        d.check(&format!("strdup del {target}"));
        assert_eq!(d.del(pool[target], 0), 0);
        d.check(&format!("strdup double-del {target}"));
        for i in 0..30 {
            let g = d.geti(pool[i]);
            if i == target {
                assert_eq!(g, -1);
            } else {
                assert!(g >= 0);
            }
        }
        d.check(&format!("strdup post-del {target}"));
        d.free();
    }

    // 1000 randomized ops
    let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_STRDUP, DEFAULT_SEED);
    for step in 0..1000 {
        let k = pool[rng.below(pool.len())];
        match rng.below(9) {
            0..=3 => {
                d.put(k, &(step as u64).to_ne_bytes());
            }
            4..=6 => {
                d.del(k, 0);
            }
            _ => {
                d.geti(k);
            }
        }
        if step % 9 == 0 {
            d.check(&format!("strdup churn {step}"));
        }
    }
    d.check("strdup churn final");
    d.free();
}

#[test]
fn cfg_c61_hmfree_strdup() {
    let p = libs();
    for n in [0usize, 1, 8, 200] {
        let mut keys = Keys::new();
        let mut rng = Rng::new(61 + n as u64);
        let pool = key_pool(&mut rng, &mut keys, n.max(1), 1, 25);
        let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_STRDUP, DEFAULT_SEED);
        for i in 0..n {
            d.put(pool[i], &(i as u64).to_ne_bytes());
        }
        d.check(&format!("strdup n={n} pre-free"));
        d.free(); // must free every duplicated key
        // fresh table, reuse
        let mut d2 = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_STRDUP, DEFAULT_SEED);
        for i in 0..n.min(4) {
            d2.put(pool[i], &(i as u64).to_ne_bytes());
        }
        d2.check(&format!("strdup n={n} reuse"));
        d2.free();
    }
}

// C62 — shputs on a strdup table (`.key` re-pointed at temp_key)
#[test]
fn cfg_c58b_strdup_shputs() {
    let p = libs();
    let mut keys = Keys::new();
    let mut rng = Rng::new(582);
    let pool = key_pool(&mut rng, &mut keys, 50, 1, 20);
    let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_STRDUP, DEFAULT_SEED);
    for (i, &k) in pool.iter().enumerate() {
        let mut whole = vec![0u8; 16];
        whole[8..16].copy_from_slice(&(i as u64).to_ne_bytes());
        d.puts(k, &whole);
        d.check(&format!("shputs {i}"));
        d.check_temp_key(&format!("shputs {i}"));
    }
    for (i, &k) in pool.iter().enumerate() {
        assert_eq!(d.geti(k), i as isize, "shputs get {i}");
    }
    d.check("shputs final");
    d.free();
}

// ===========================================================================
// Group 8 — SH_ARENA
// ===========================================================================

#[test]
fn cfg_c62_c63_arena_blocks() {
    let p = libs();
    let mut keys = Keys::new();
    let mut rng = Rng::new(63);
    // ~120 keys of ~10 bytes = ~1300 bytes -> at least 3 blocks (512,512,1024)
    let pool = key_pool(&mut rng, &mut keys, 300, 8, 14);
    let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_ARENA, DEFAULT_SEED);
    diff_eq!(d.c.snap(), d.r.snap(), "arena pristine");
    for (i, &k) in pool.iter().enumerate() {
        d.put(k, &(i as u64).to_ne_bytes());
        d.check(&format!("arena put {i}"));
        d.check_temp_key(&format!("arena put {i}"));
    }
    let s = d.c.snap();
    assert!(s.idx.arena.chain_len >= 3, "expected several blocks: {:?}", s.idx.arena);
    assert!(s.idx.arena.block >= 3);
    for (i, &k) in pool.iter().enumerate() {
        assert_eq!(d.geti(k), i as isize, "arena get {i}");
    }
    d.check("arena gets");
    d.free();
}

#[test]
fn cfg_c64_arena_oversized_keys() {
    let p = libs();
    let mut keys = Keys::new();
    let mut rng = Rng::new(64);
    let mut pool: Vec<*mut c_char> = Vec::new();
    // alternate short keys and keys longer than the current blocksize
    for i in 0..40 {
        if i % 3 == 0 {
            let body = rng.cstr_body(600 + i * 37, false);
            pool.push(keys.add(&body));
        } else {
            let n = rng.range(1, 20);
            let body = rng.cstr_body(n, false);
            pool.push(keys.add(&body));
        }
    }
    let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_ARENA, DEFAULT_SEED);
    for (i, &k) in pool.iter().enumerate() {
        d.put(k, &(i as u64).to_ne_bytes());
        d.check(&format!("arena oversized put {i}"));
    }
    for (i, &k) in pool.iter().enumerate() {
        assert_eq!(d.geti(k), i as isize, "arena oversized get {i}");
    }
    d.check("arena oversized final");
    d.free();
}

#[test]
fn cfg_c65_c66_arena_random_and_churn() {
    let p = libs();
    let mut keys = Keys::new();
    let mut rng = Rng::new(65);
    let pool = key_pool(&mut rng, &mut keys, 400, 1, 40);
    let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_ARENA, DEFAULT_SEED);
    for (i, &k) in pool.iter().enumerate() {
        d.put(k, &(i as u64).to_ne_bytes());
        if i % 13 == 0 {
            d.check(&format!("arena rand put {i}"));
        }
    }
    d.check("arena rand filled");
    d.free();

    let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_ARENA, DEFAULT_SEED);
    for step in 0..1000 {
        let k = pool[rng.below(64)];
        match rng.below(9) {
            0..=3 => {
                d.put(k, &(step as u64).to_ne_bytes());
            }
            4..=6 => {
                d.del(k, 0);
            }
            _ => {
                d.geti(k);
            }
        }
        if step % 9 == 0 {
            d.check(&format!("arena churn {step}"));
        }
    }
    d.check("arena churn final");
    d.free();
}

#[test]
fn cfg_c67_hmfree_arena() {
    let p = libs();
    for n in [0usize, 1, 10, 300] {
        let mut keys = Keys::new();
        let mut rng = Rng::new(67 + n as u64);
        let mut pool = key_pool(&mut rng, &mut keys, n.max(1), 1, 30);
        if n > 0 {
            // include one oversized key so strreset walks a spliced block too
            let big = rng.cstr_body(3000, false);
            pool.push(keys.add(&big));
        }
        let mut d = SDuo::new_shmode(&p, spec_owned(), STBDS_HM_STRING, STBDS_SH_ARENA, DEFAULT_SEED);
        for (i, &k) in pool.iter().enumerate().take(if n == 0 { 0 } else { pool.len() }) {
            d.put(k, &(i as u64).to_ne_bytes());
        }
        d.check(&format!("arena n={n} pre-free"));
        d.free();
    }
}

// ===========================================================================
// Group 9 — SH_NONE and explicit SH_DEFAULT
// ===========================================================================

// C68 — explicit SH_NONE table + BINARY puts (the `default: memcpy` branch)
#[test]
fn cfg_c68_sh_none_binary() {
    let p = libs();
    let mut rng = Rng::new(68);
    for (elemsize, keysize) in [(8usize, 4usize), (16, 8), (20, 8), (4, 4), (1, 1)] {
        let spec = Spec::bytes(elemsize, keysize);
        reset_seed(&p, DEFAULT_SEED);
        let mut cm = Map::new_shmode(&p.c, spec, STBDS_HM_BINARY, STBDS_SH_NONE);
        let mut rm = Map::new_shmode(&p.r, spec, STBDS_HM_BINARY, STBDS_SH_NONE);
        diff_eq!(cm.snap(), rm.snap(), "SH_NONE e={elemsize} pristine");
        let mut seen = HashSet::new();
        let mut ks: Vec<Vec<u8>> = Vec::new();
        while ks.len() < 60 {
            let k = rng.bytes(keysize);
            if seen.insert(k.clone()) {
                ks.push(k);
            }
        }
        for (i, k) in ks.iter().enumerate() {
            let v = rng.bytes(elemsize - keysize);
            let a = cm.hmput(k, &v);
            let b = rm.hmput(k, &v);
            diff_eq!(a, b, "SH_NONE e={elemsize} put {i}");
            diff_eq!(cm.snap(), rm.snap(), "SH_NONE e={elemsize} state {i}");
        }
        for (i, k) in ks.iter().enumerate() {
            let a = cm.hmgeti(k);
            let b = rm.hmgeti(k);
            diff_eq!(a, b, "SH_NONE e={elemsize} get {i}");
            assert_eq!(a, i as isize);
        }
        diff_eq!(cm.snap(), rm.snap(), "SH_NONE e={elemsize} final");
        cm.hmfree();
        rm.hmfree();
    }
}

// C69 — explicit SH_NONE table + STRING mode: hash_string/strcmp dispatch but
// `memcpy` storage.  Only distinct keys are inserted (a duplicate would make
// the C `stbds_is_key_equal` reinterpret the copied string bytes as a `char *`
// and dereference them — undefined in the C itself).
#[test]
fn cfg_c69_sh_none_string_mode() {
    let p = libs();
    let mut keys = Keys::new();
    let mut rng = Rng::new(69);
    let pool = key_pool(&mut rng, &mut keys, 60, 12, 40);
    let spec = Spec::bytes(16, 8);
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new_shmode(&p.c, spec, STBDS_HM_STRING, STBDS_SH_NONE);
    let mut rm = Map::new_shmode(&p.r, spec, STBDS_HM_STRING, STBDS_SH_NONE);
    diff_eq!(cm.snap(), rm.snap(), "SH_NONE+STRING pristine");
    for (i, &k) in pool.iter().enumerate() {
        let a = cm.shput(k, &(i as u64).to_ne_bytes());
        let b = rm.shput(k, &(i as u64).to_ne_bytes());
        diff_eq!(a, b, "SH_NONE+STRING put {i}");
        diff_eq!(cm.snap(), rm.snap(), "SH_NONE+STRING state {i}");
    }
    // absent-key lookups are safe (no hash match -> is_key_equal never runs)
    let mut absent = Keys::new();
    for i in 0..20 {
        let ak = absent.add(format!("absent-key-number-{i}").as_bytes());
        let a = cm.shgeti(ak);
        let b = rm.shgeti(ak);
        diff_eq!(a, b, "SH_NONE+STRING miss {i}");
        assert_eq!(a, -1);
        diff_eq!(cm.snap(), rm.snap(), "SH_NONE+STRING miss state {i}");
    }
    cm.hmfree();
    rm.hmfree();
}

// C70 — explicit SH_DEFAULT table
#[test]
fn cfg_c70_explicit_sh_default() {
    let p = libs();
    let mut keys = Keys::new();
    let mut rng = Rng::new(70);
    let pool = key_pool(&mut rng, &mut keys, 120, 1, 30);
    let mut d = SDuo::new_shmode(&p, spec_default(), STBDS_HM_STRING, STBDS_SH_DEFAULT, DEFAULT_SEED);
    assert_eq!(d.c.snap().idx.arena.mode, 1);
    for (i, &k) in pool.iter().enumerate() {
        d.put(k, &(i as u64).to_ne_bytes());
        d.check(&format!("explicit SH_DEFAULT put {i}"));
        d.check_temp_key(&format!("explicit SH_DEFAULT put {i}"));
    }
    for (i, &k) in pool.iter().enumerate() {
        assert_eq!(d.geti(k), i as isize);
    }
    d.check("explicit SH_DEFAULT gets");
    for &k in pool.iter().step_by(3) {
        d.del(k, 0);
        d.check("explicit SH_DEFAULT del");
    }
    d.check("explicit SH_DEFAULT final");
    d.free();
}
