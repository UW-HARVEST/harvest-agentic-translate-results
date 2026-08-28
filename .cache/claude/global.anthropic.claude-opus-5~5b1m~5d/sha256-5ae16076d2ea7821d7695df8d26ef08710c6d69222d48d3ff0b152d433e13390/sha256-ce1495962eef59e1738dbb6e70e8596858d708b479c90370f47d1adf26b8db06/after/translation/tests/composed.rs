//! Phase B, Group 12 of `CONFIGS.md`: composed, multi-entry-point pipelines and
//! the cross-checking fuzz driver.  These drive the *low-level* exports directly
//! (not just a single convenience wrapper) and compare C vs Rust after every op.

mod common;

use common::*;
use core::ffi::{c_char, c_void};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// C82 — full lifecycle across every (mode, string.mode) combination the C
//       actually distinguishes.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Model {
    label: &'static str,
    spec: Spec,
    mode: i32,
    /// `Some(sh)` -> create the table up-front via `stbds_shmode_func`.
    /// `None`     -> let `stbds_hmput_key` create it implicitly.
    sh: Option<i32>,
    /// Keys are C strings (STRING protocol) rather than inline bytes.
    string_keys: bool,
}

fn models() -> Vec<Model> {
    vec![
        Model {
            label: "BINARY/implicit(SH_NONE) e8k4",
            spec: Spec::bytes(8, 4),
            mode: STBDS_HM_BINARY,
            sh: None,
            string_keys: false,
        },
        Model {
            label: "BINARY/implicit(SH_NONE) e20k8",
            spec: Spec::bytes(20, 8),
            mode: STBDS_HM_BINARY,
            sh: None,
            string_keys: false,
        },
        Model {
            label: "BINARY/explicit SH_NONE e16k8",
            spec: Spec::bytes(16, 8),
            mode: STBDS_HM_BINARY,
            sh: Some(STBDS_SH_NONE),
            string_keys: false,
        },
        Model {
            label: "STRING/implicit SH_DEFAULT e16",
            spec: Spec::bytes(16, 8),
            mode: STBDS_HM_STRING,
            sh: None,
            string_keys: true,
        },
        Model {
            label: "STRING/explicit SH_DEFAULT e16",
            spec: Spec::bytes(16, 8),
            mode: STBDS_HM_STRING,
            sh: Some(STBDS_SH_DEFAULT),
            string_keys: true,
        },
        Model {
            label: "STRING/explicit SH_STRDUP e16",
            spec: Spec::ptr(16),
            mode: STBDS_HM_STRING,
            sh: Some(STBDS_SH_STRDUP),
            string_keys: true,
        },
        Model {
            label: "STRING/explicit SH_ARENA e16",
            spec: Spec::ptr(16),
            mode: STBDS_HM_STRING,
            sh: Some(STBDS_SH_ARENA),
            string_keys: true,
        },
        Model {
            label: "STRING/explicit SH_STRDUP e24",
            spec: Spec::ptr(24),
            mode: STBDS_HM_STRING,
            sh: Some(STBDS_SH_STRDUP),
            string_keys: true,
        },
    ]
}

struct Pipeline<'a> {
    m: Model,
    c: Map<'a>,
    r: Map<'a>,
}

impl<'a> Pipeline<'a> {
    fn new(p: &'a Pair, m: Model, seed: usize) -> Pipeline<'a> {
        reset_seed(p, seed);
        let (c, r) = match m.sh {
            Some(sh) => (
                Map::new_shmode(&p.c, m.spec, m.mode, sh),
                Map::new_shmode(&p.r, m.spec, m.mode, sh),
            ),
            None => (Map::new(&p.c, m.spec, m.mode), Map::new(&p.r, m.spec, m.mode)),
        };
        Pipeline { m, c, r }
    }
    fn restart(&mut self, p: &'a Pair) {
        self.c.hmfree();
        self.r.hmfree();
        let (c, r) = match self.m.sh {
            Some(sh) => (
                Map::new_shmode(&p.c, self.m.spec, self.m.mode, sh),
                Map::new_shmode(&p.r, self.m.spec, self.m.mode, sh),
            ),
            None => (
                Map::new(&p.c, self.m.spec, self.m.mode),
                Map::new(&p.r, self.m.spec, self.m.mode),
            ),
        };
        self.c = c;
        self.r = r;
    }
    fn check(&self, ctx: &str) {
        diff_eq!(self.c.snap(), self.r.snap(), "{} :: {ctx}", self.m.label);
        diff_eq!(
            self.c.hmlen(),
            self.r.hmlen(),
            "{} :: {ctx} hmlen",
            self.m.label
        );
    }
    fn free(&mut self) {
        self.c.hmfree();
        self.r.hmfree();
    }
}

/// A key usable by either protocol: inline bytes or a live C string.
enum Key {
    Bytes(Vec<u8>),
    Str(*mut c_char),
}

fn build_keys(m: &Model, rng: &mut Rng, keys: &mut Keys, n: usize) -> Vec<Key> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut guard = 0usize;
    while out.len() < n && guard < n * 2000 {
        guard += 1;
        if m.string_keys {
            let len = rng.range(1, 30);
            let body = rng.cstr_body(len, false);
            if seen.insert(body.clone()) {
                out.push(Key::Str(keys.add(&body)));
            }
        } else {
            let b = rng.bytes(m.spec.keysize);
            if seen.insert(b.clone()) {
                out.push(Key::Bytes(b));
            }
        }
    }
    assert_eq!(out.len(), n, "key pool for {}", m.label);
    out
}

fn do_put(pl: &mut Pipeline, k: &Key, value: &[u8]) -> isize {
    let (a, b) = match k {
        Key::Bytes(kb) => (pl.c.hmput(kb, value), pl.r.hmput(kb, value)),
        Key::Str(ks) => (pl.c.shput(*ks, value), pl.r.shput(*ks, value)),
    };
    diff_eq!(a, b, "{} :: put index", pl.m.label);
    a
}

fn do_get(pl: &mut Pipeline, k: &Key) -> isize {
    let (a, b) = match k {
        Key::Bytes(kb) => (pl.c.hmgeti(kb), pl.r.hmgeti(kb)),
        Key::Str(ks) => (pl.c.shgeti(*ks), pl.r.shgeti(*ks)),
    };
    diff_eq!(a, b, "{} :: get index", pl.m.label);
    a
}

fn do_get_ts(pl: &mut Pipeline, k: &Key) -> isize {
    let kp = match k {
        Key::Bytes(kb) => kb.as_ptr() as *mut c_void,
        Key::Str(ks) => *ks as *mut c_void,
    };
    let a = pl.c.hmgeti_ts(kp);
    let b = pl.r.hmgeti_ts(kp);
    diff_eq!(a, b, "{} :: get_ts index", pl.m.label);
    a
}

fn do_del(pl: &mut Pipeline, k: &Key, keyoffset: usize) -> isize {
    let kp = match k {
        Key::Bytes(kb) => kb.as_ptr() as *mut c_void,
        Key::Str(ks) => *ks as *mut c_void,
    };
    let a = pl.c.hmdel(kp, keyoffset);
    let b = pl.r.hmdel(kp, keyoffset);
    diff_eq!(a, b, "{} :: del result", pl.m.label);
    a
}

#[test]
fn cfg_c82_full_lifecycle_all_modes() {
    let p = libs();
    for (mi, m) in models().into_iter().enumerate() {
        for &seed in [DEFAULT_SEED, 0usize, 1, usize::MAX, 0xdead_beef].iter() {
            let mut rng = Rng::new((mi as u64) * 977 + seed as u64);
            let mut keys = Keys::new();
            let pool = build_keys(&m, &mut rng, &mut keys, 80);
            let vlen = m.spec.elemsize - m.spec.keysize;
            let mut pl = Pipeline::new(&p, m, seed);
            pl.check("pristine");

            // 1. fill
            for (i, k) in pool.iter().enumerate() {
                let v = rng.bytes(vlen);
                let idx = do_put(&mut pl, k, &v);
                assert_eq!(idx, i as isize, "{} :: fill index", m.label);
                pl.check(&format!("seed={seed:#x} fill {i}"));
            }
            // 2. read all back, both entry points
            for (i, k) in pool.iter().enumerate() {
                assert_eq!(do_get(&mut pl, k), i as isize);
                assert_eq!(do_get_ts(&mut pl, k), i as isize);
                pl.check(&format!("seed={seed:#x} read {i}"));
            }
            // 3. delete a third
            for (i, k) in pool.iter().enumerate() {
                if i % 3 == 0 {
                    assert_eq!(do_del(&mut pl, k, 0), 1);
                    pl.check(&format!("seed={seed:#x} del {i}"));
                }
            }
            // 4. re-insert the deleted ones (tombstone reuse)
            for (i, k) in pool.iter().enumerate() {
                if i % 3 == 0 {
                    let v = rng.bytes(vlen);
                    do_put(&mut pl, k, &v);
                    pl.check(&format!("seed={seed:#x} reinsert {i}"));
                }
            }
            // 5. everything still present
            for k in pool.iter() {
                assert!(do_get(&mut pl, k) >= 0);
            }
            pl.check(&format!("seed={seed:#x} final"));
            pl.free();
        }
    }
    reset_seed(&p, DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// C83 — an array bootstrapped by a *get* (hash_table == NULL) then upgraded
// ---------------------------------------------------------------------------
#[test]
fn cfg_c83_get_bootstrapped_then_put() {
    let p = libs();
    for (elemsize, keysize) in [(8usize, 4usize), (16, 8), (20, 8), (1, 1)] {
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING] {
            // For STRING mode the get on a NULL map never hashes the key
            // (it takes the `a == NULL` fast path), so this is safe.
            reset_seed(&p, DEFAULT_SEED);
            let spec = Spec::bytes(elemsize, keysize);
            let mut cm = Map::new(&p.c, spec, mode);
            let mut rm = Map::new(&p.r, spec, mode);
            let key = vec![0x11u8; keysize];
            // get on NULL: allocates the default slot, temp = -1, no hash table
            let a = cm.hmgeti_ts(key.as_ptr() as *mut c_void);
            let b = rm.hmgeti_ts(key.as_ptr() as *mut c_void);
            diff_eq!(a, b, "e={elemsize} mode={mode} bootstrap ts");
            assert_eq!(a, STBDS_INDEX_EMPTY);
            diff_eq!(cm.snap(), rm.snap(), "e={elemsize} mode={mode} bootstrapped");
            assert!(!cm.snap().hdr.has_table, "no hash table yet");
            assert_eq!(cm.snap().hdr.length, 1);

            // a second get: table is still NULL -> *temp = -1
            let a = cm.hmgeti_ts(key.as_ptr() as *mut c_void);
            let b = rm.hmgeti_ts(key.as_ptr() as *mut c_void);
            diff_eq!(a, b, "e={elemsize} mode={mode} second ts");
            assert_eq!(a, -1);
            diff_eq!(cm.snap(), rm.snap(), "e={elemsize} mode={mode} second get");

            // hmget_key (not _ts) also writes header->temp
            let a = cm.hmgeti(&key);
            let b = rm.hmgeti(&key);
            diff_eq!(a, b, "e={elemsize} mode={mode} hmget_key");
            assert_eq!(a, -1);
            diff_eq!(cm.snap(), rm.snap(), "e={elemsize} mode={mode} hmget_key state");

            // hmdel on the table-less array: temp = 0, unchanged
            let a = cm.hmdel(key.as_ptr() as *mut c_void, 0);
            let b = rm.hmdel(key.as_ptr() as *mut c_void, 0);
            diff_eq!(a, b, "e={elemsize} mode={mode} del no-table");
            assert_eq!(a, 0);
            diff_eq!(cm.snap(), rm.snap(), "e={elemsize} mode={mode} del no-table state");

            if mode == STBDS_HM_BINARY {
                // now a put upgrades it with a real hash index
                for i in 0..20u8 {
                    let k = vec![i; keysize];
                    let v = vec![i ^ 0x5A; elemsize - keysize];
                    let x = cm.hmput(&k, &v);
                    let y = rm.hmput(&k, &v);
                    diff_eq!(x, y, "e={elemsize} upgrade put {i}");
                    diff_eq!(cm.snap(), rm.snap(), "e={elemsize} upgrade state {i}");
                }
                assert!(cm.snap().hdr.has_table);
            }
            cm.hmfree();
            rm.hmfree();
        }
    }
}

// ---------------------------------------------------------------------------
// C84 — hmput_default -> puts -> drain -> get, default value survives
// ---------------------------------------------------------------------------
#[test]
fn cfg_c84_default_survives_drain() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
    let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
    let dflt = (-424242i32).to_ne_bytes();
    cm.hmdefault(&dflt);
    rm.hmdefault(&dflt);
    diff_eq!(cm.snap(), rm.snap(), "C84 default set");
    for i in 0..30i32 {
        cm.hmput(&i.to_ne_bytes(), &(i * 11).to_ne_bytes());
        rm.hmput(&i.to_ne_bytes(), &(i * 11).to_ne_bytes());
        diff_eq!(cm.snap(), rm.snap(), "C84 put {i}");
    }
    for i in 0..30i32 {
        let a = cm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
        let b = rm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
        diff_eq!(a, b, "C84 del {i}");
        diff_eq!(cm.snap(), rm.snap(), "C84 del state {i}");
    }
    assert_eq!(cm.hmlen(), 0);
    // default element must still be there at [-1]
    let s = cm.snap();
    assert_eq!(s.hdr.length, 1);
    diff_eq!(s, rm.snap(), "C84 drained");
    // hmput_default on the live map is a no-op
    let before = cm.snap();
    cm.hmdefault(&dflt);
    rm.hmdefault(&dflt);
    diff_eq!(before, cm.snap(), "C84 hmput_default must be a no-op");
    diff_eq!(cm.snap(), rm.snap(), "C84 after redundant default");
    cm.hmfree();
    rm.hmfree();
}

// ---------------------------------------------------------------------------
// C85 — raw arrgrowf on a hash-map array, then more puts
// ---------------------------------------------------------------------------
#[test]
fn cfg_c85_arrgrowf_on_hash_array() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    for extra_cap in [0usize, 1, 5, 8, 33, 200] {
        reset_seed(&p, DEFAULT_SEED);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        for i in 0..10i32 {
            cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        }
        diff_eq!(cm.snap(), rm.snap(), "C85 cap={extra_cap} filled");
        // arrsetcap on the underlying array (the header moves, hash_table survives)
        unsafe {
            let ca = (p.c.arrgrowf)(cm.raw(), spec.elemsize, 0, extra_cap);
            let ra = (p.r.arrgrowf)(rm.raw(), spec.elemsize, 0, extra_cap);
            cm.t = (ca as *mut u8).add(spec.elemsize) as *mut c_void;
            rm.t = (ra as *mut u8).add(spec.elemsize) as *mut c_void;
        }
        diff_eq!(cm.snap(), rm.snap(), "C85 cap={extra_cap} after arrgrowf");
        // more puts through the (possibly moved) array
        for i in 10..60i32 {
            let a = cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            let b = rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            diff_eq!(a, b, "C85 cap={extra_cap} post put {i}");
            diff_eq!(cm.snap(), rm.snap(), "C85 cap={extra_cap} post state {i}");
        }
        for i in 0..60i32 {
            let a = cm.hmgeti(&i.to_ne_bytes());
            let b = rm.hmgeti(&i.to_ne_bytes());
            diff_eq!(a, b, "C85 cap={extra_cap} get {i}");
            assert_eq!(a, i as isize);
        }
        cm.hmfree();
        rm.hmfree();
    }
}

// ---------------------------------------------------------------------------
// C86 — cross-checking fuzz driver, every model x 3000 ops
// ---------------------------------------------------------------------------
#[test]
fn cfg_c86_fuzz_all_models() {
    let p = libs();
    for (mi, m) in models().into_iter().enumerate() {
        let mut rng = Rng::new(86_000 + mi as u64);
        let mut keys = Keys::new();
        let pool = build_keys(&m, &mut rng, &mut keys, 96);
        let vlen = m.spec.elemsize - m.spec.keysize;
        let mut pl = Pipeline::new(&p, m, DEFAULT_SEED);
        for step in 0..3000usize {
            let k = &pool[rng.below(pool.len())];
            match rng.below(20) {
                0..=7 => {
                    let v = rng.bytes(vlen);
                    do_put(&mut pl, k, &v);
                }
                8..=12 => {
                    do_del(&mut pl, k, 0);
                }
                13..=15 => {
                    do_get(&mut pl, k);
                }
                16..=17 => {
                    do_get_ts(&mut pl, k);
                }
                18 => {
                    let v = rng.bytes(vlen);
                    pl.c.hmdefault(&v);
                    pl.r.hmdefault(&v);
                }
                _ => {
                    if step % 500 == 0 {
                        pl.restart(&p);
                    } else {
                        do_get(&mut pl, k);
                    }
                }
            }
            // cross-check after EVERY op
            pl.check(&format!("fuzz step {step}"));
        }
        pl.free();
    }
    reset_seed(&p, DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// C86b — fuzz with several global seeds, so bucket layouts differ
// ---------------------------------------------------------------------------
#[test]
fn cfg_c86b_fuzz_seed_matrix() {
    let p = libs();
    for &seed in [0usize, 1, 2, 0xdead_beef, usize::MAX, 1 << 63].iter() {
        for (mi, m) in models().into_iter().enumerate().take(6) {
            let mut rng = Rng::new(seed as u64 ^ (mi as u64 * 31));
            let mut keys = Keys::new();
            let pool = build_keys(&m, &mut rng, &mut keys, 48);
            let vlen = m.spec.elemsize - m.spec.keysize;
            let mut pl = Pipeline::new(&p, m, seed);
            for step in 0..700usize {
                let k = &pool[rng.below(pool.len())];
                match rng.below(10) {
                    0..=4 => {
                        let v = rng.bytes(vlen);
                        do_put(&mut pl, k, &v);
                    }
                    5..=7 => {
                        do_del(&mut pl, k, 0);
                    }
                    8 => {
                        do_get(&mut pl, k);
                    }
                    _ => {
                        do_get_ts(&mut pl, k);
                    }
                }
                pl.check(&format!("seed={seed:#x} step={step}"));
            }
            pl.free();
        }
    }
    reset_seed(&p, DEFAULT_SEED);
}
