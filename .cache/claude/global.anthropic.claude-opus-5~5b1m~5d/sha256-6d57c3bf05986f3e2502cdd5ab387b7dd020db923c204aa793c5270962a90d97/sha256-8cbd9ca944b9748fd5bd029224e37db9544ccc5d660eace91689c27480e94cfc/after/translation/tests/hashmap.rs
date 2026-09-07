//! Phase B — the hash-map pipeline driven through the low-level exported
//! entry points exactly as the `stbds_hmput` / `stbds_shput` / `stbds_hmget` /
//! `stbds_hmdel` macros do.  CONFIGS.md rows 20–63.
//!
//! Every operation is applied to BOTH libraries and the *entire* observable
//! state is then compared: array header (length / capacity / temp), every
//! element's bytes, the hash index (slot_count, used_count, thresholds, seed,
//! log2, arena state) and every one of the 8 hash/index entries of every
//! bucket.  Heap addresses are never compared — only contents.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// harness
// ===========================================================================

#[derive(Copy, Clone)]
struct Cfg {
    es: usize,
    ks: usize,
    /// `mode` passed to hmput_key / hmget_key / hmdel_key
    mode: c_int,
    kind: KeyKind,
}

impl Cfg {
    /// offset at which the test writes the value payload
    fn voff(&self) -> usize {
        match self.kind {
            KeyKind::Bytes => self.ks,
            KeyKind::CStr => std::mem::size_of::<usize>(),
        }
    }
    /// keysize used for snapshotting
    fn snap_ks(&self) -> usize {
        match self.kind {
            KeyKind::Bytes => self.ks,
            KeyKind::CStr => std::mem::size_of::<usize>(),
        }
    }
}

struct Maps {
    c: *mut c_void,
    r: *mut c_void,
}

impl Maps {
    fn new() -> Maps {
        Maps {
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
        }
    }
}

unsafe fn temp_of(map: *mut c_void, es: usize) -> isize {
    hdr_of(map_arr(map, es)).temp
}

/// Writes the value payload over the whole non-key remainder of element `idx`
/// so that no byte of a live element is ever uninitialised (the real
/// `stbds_hmput` macro leaves struct padding undefined, which is not
/// comparable across two heaps).
unsafe fn write_value(map: *mut c_void, cfg: &Cfg, idx: isize, tag_byte: u8) {
    let voff = cfg.voff();
    if voff >= cfg.es {
        return;
    }
    let e = (map as *mut u8).offset((cfg.es as isize) * idx);
    for k in voff..cfg.es {
        *e.add(k) = tag_byte.wrapping_add((k as u8).wrapping_mul(31));
    }
}

fn check(_p: &Pair, m: &Maps, cfg: &Cfg, keyoffset: usize, tag: &str) {
    unsafe {
        let a = map_snap(m.c, cfg.es, keyoffset, cfg.snap_ks(), cfg.kind);
        let b = map_snap(m.r, cfg.es, keyoffset, cfg.snap_ks(), cfg.kind);
        if a != b {
            // produce a focused diff
            assert_eq!(a.is_null, b.is_null, "{tag}: NULL-ness");
            assert_eq!(a.length, b.length, "{tag}: header.length");
            assert_eq!(a.capacity, b.capacity, "{tag}: header.capacity");
            assert_eq!(a.temp, b.temp, "{tag}: header.temp");
            assert_eq!(a.elems.len(), b.elems.len(), "{tag}: element count");
            for i in 0..a.elems.len() {
                assert_eq!(a.elems[i].1, b.elems[i].1, "{tag}: element {i} key");
                assert_eq!(a.elems[i].0, b.elems[i].0, "{tag}: element {i} payload");
            }
            match (&a.idx, &b.idx) {
                (Some(x), Some(y)) => {
                    assert_eq!(x.slot_count, y.slot_count, "{tag}: slot_count");
                    assert_eq!(x.used_count, y.used_count, "{tag}: used_count");
                    assert_eq!(
                        x.used_count_threshold, y.used_count_threshold,
                        "{tag}: used_count_threshold"
                    );
                    assert_eq!(
                        x.used_count_shrink_threshold, y.used_count_shrink_threshold,
                        "{tag}: used_count_shrink_threshold"
                    );
                    assert_eq!(x.tombstone_count, y.tombstone_count, "{tag}: tombstone_count");
                    assert_eq!(
                        x.tombstone_count_threshold, y.tombstone_count_threshold,
                        "{tag}: tombstone_count_threshold"
                    );
                    assert_eq!(x.seed, y.seed, "{tag}: seed");
                    assert_eq!(x.slot_count_log2, y.slot_count_log2, "{tag}: slot_count_log2");
                    assert_eq!(x.arena_remaining, y.arena_remaining, "{tag}: arena.remaining");
                    assert_eq!(x.arena_block, y.arena_block, "{tag}: arena.block");
                    assert_eq!(x.arena_mode, y.arena_mode, "{tag}: arena.mode");
                    assert_eq!(
                        x.arena_has_storage, y.arena_has_storage,
                        "{tag}: arena.storage NULL-ness"
                    );
                    for (i, (bx, by)) in x.buckets.iter().zip(y.buckets.iter()).enumerate() {
                        assert_eq!(bx.hash, by.hash, "{tag}: bucket {i} hash[]");
                        assert_eq!(bx.index, by.index, "{tag}: bucket {i} index[]");
                    }
                }
                _ => panic!("{tag}: hash_table NULL-ness differs"),
            }
            panic!("{tag}: snapshots differ but no field-level diff found");
        }
    }
}

/// Emulates `stbds_hmput(t,k,v)` / `stbds_shput(t,k,v)`.
unsafe fn op_put(p: &Pair, m: &mut Maps, cfg: &Cfg, key: *mut c_void, tag_byte: u8) -> (isize, isize) {
    let len_before = if m.c.is_null() {
        0
    } else {
        hdr_of(map_arr(m.c, cfg.es)).length
    };
    m.c = (p.c.hmput_key)(m.c, cfg.es, key, cfg.ks, cfg.mode);
    m.r = (p.r.hmput_key)(m.r, cfg.es, key, cfg.ks, cfg.mode);
    let ic = temp_of(m.c, cfg.es);
    let ir = temp_of(m.r, cfg.es);
    // `stbds_temp_key` is written by hmput_key on the found_empty_slot path
    // (i.e. a genuinely new element) whenever `string.mode` stores a pointer.
    // On the duplicate-key path the C only assigns it from the FIRST of the two
    // probe loops (lib.c:732-733) and never from the wrap-around loop
    // (lib.c:746-751), and `stbds_make_hash_index` never initialises the field
    // at all — so it is well-defined only for a fresh insertion.
    let len_after = hdr_of(map_arr(m.c, cfg.es)).length;
    if cfg.kind == KeyKind::CStr && cfg.mode >= HM_STRING && len_after > len_before {
        assert_eq!(
            temp_key_str(m.c, cfg.es),
            temp_key_str(m.r, cfg.es),
            "stbds_temp_key after a fresh hmput_key differs"
        );
    }
    write_value(m.c, cfg, ic, tag_byte);
    write_value(m.r, cfg, ir, tag_byte);
    (ic, ir)
}

/// Emulates `stbds_hmgeti(t,k)` (which mutates `t` and stores the index in the
/// header's `temp`).
unsafe fn op_get(p: &Pair, m: &mut Maps, cfg: &Cfg, key: *mut c_void) -> (isize, isize) {
    m.c = (p.c.hmget_key)(m.c, cfg.es, key, cfg.ks, cfg.mode);
    m.r = (p.r.hmget_key)(m.r, cfg.es, key, cfg.ks, cfg.mode);
    (temp_of(m.c, cfg.es), temp_of(m.r, cfg.es))
}

/// Emulates `stbds_hmgeti_ts(t,k,temp)`.
unsafe fn op_get_ts(p: &Pair, m: &mut Maps, cfg: &Cfg, key: *mut c_void) -> (isize, isize) {
    let mut tc: isize = 0x5a5a;
    let mut tr: isize = 0x5a5a;
    m.c = (p.c.hmget_key_ts)(m.c, cfg.es, key, cfg.ks, &mut tc, cfg.mode);
    m.r = (p.r.hmget_key_ts)(m.r, cfg.es, key, cfg.ks, &mut tr, cfg.mode);
    (tc, tr)
}

/// Emulates `stbds_hmdel(t,k)`.
unsafe fn op_del(
    p: &Pair,
    m: &mut Maps,
    cfg: &Cfg,
    key: *mut c_void,
    keyoffset: usize,
) -> (isize, isize) {
    m.c = (p.c.hmdel_key)(m.c, cfg.es, key, cfg.ks, keyoffset, cfg.mode);
    m.r = (p.r.hmdel_key)(m.r, cfg.es, key, cfg.ks, keyoffset, cfg.mode);
    let rc = if m.c.is_null() {
        0
    } else {
        temp_of(m.c, cfg.es)
    };
    let rr = if m.r.is_null() {
        0
    } else {
        temp_of(m.r, cfg.es)
    };
    (rc, rr)
}

unsafe fn free_maps(p: &Pair, m: &mut Maps, es: usize) {
    if !m.c.is_null() {
        (p.c.hmfree_func)(map_arr(m.c, es), es);
        m.c = std::ptr::null_mut();
    }
    if !m.r.is_null() {
        (p.r.hmfree_func)(map_arr(m.r, es), es);
        m.r = std::ptr::null_mut();
    }
}

/// Stable-address pool of NUL-terminated keys (needed for SH_DEFAULT, which
/// stores the caller's pointer).
struct KeyPool {
    raw: Vec<Box<[u8]>>,
}

impl KeyPool {
    fn new() -> KeyPool {
        KeyPool { raw: Vec::new() }
    }
    fn push(&mut self, mut b: Vec<u8>) -> *mut c_void {
        if !b.ends_with(&[0]) {
            b.push(0);
        }
        let bx: Box<[u8]> = b.into_boxed_slice();
        let ptr = bx.as_ptr() as *mut c_void;
        self.raw.push(bx);
        ptr
    }
    fn push_bytes(&mut self, b: Vec<u8>) -> *mut c_void {
        let bx: Box<[u8]> = b.into_boxed_slice();
        let ptr = bx.as_ptr() as *mut c_void;
        self.raw.push(bx);
        ptr
    }
}

const SEED_SET: [usize; 5] = [DEFAULT_SEED, 0, 1, usize::MAX, 0xdead_beef];

// ===========================================================================
// rows 20–22 : stbds_hmput_default
// ===========================================================================

#[test]
fn rows20_22_hmput_default() {
    let (p, _g) = libs();
    for &gseed in SEED_SET.iter() {
        reseed(p, gseed);
        for &es in [8usize, 16, 24, 40].iter() {
            unsafe {
                // row 20: from NULL
                let mut m = Maps::new();
                m.c = (p.c.hmput_default)(std::ptr::null_mut(), es);
                m.r = (p.r.hmput_default)(std::ptr::null_mut(), es);
                assert!(!m.c.is_null() && !m.r.is_null());
                let cfg = Cfg {
                    es,
                    ks: 4,
                    mode: HM_BINARY,
                    kind: KeyKind::Bytes,
                };
                // the default element (index 0) is memset to 0; write the
                // default value the same way `hmdefault` does
                for k in 0..es {
                    *(m.c as *mut u8).sub(es).add(k) = (k as u8).wrapping_mul(3);
                    *(m.r as *mut u8).sub(es).add(k) = (k as u8).wrapping_mul(3);
                }
                check(p, &m, &cfg, 0, &format!("hmput_default es={es} seed={gseed:#x}"));

                // row 21: second call returns the SAME pointer
                let c2 = (p.c.hmput_default)(m.c, es);
                let r2 = (p.r.hmput_default)(m.r, es);
                assert_eq!(c2, m.c, "C hmput_default must be idempotent");
                assert_eq!(r2, m.r, "Rust hmput_default must be idempotent");

                // row 22: hash_table is still NULL -> hmget must report -1
                let mut kp = KeyPool::new();
                let key = kp.push_bytes(vec![1u8, 2, 3, 4]);
                let (a, b) = op_get(p, &mut m, &cfg, key);
                assert_eq!(a, b, "hmget on table-less map");
                assert_eq!(a, -1, "hmget on table-less map must yield -1");
                let (a, b) = op_get_ts(p, &mut m, &cfg, key);
                assert_eq!((a, b), (-1, -1), "hmget_key_ts on table-less map");
                // row 32(err): hmdel on table-less map -> returns a, temp == 0
                let (a, b) = op_del(p, &mut m, &cfg, key, 0);
                assert_eq!((a, b), (0, 0), "hmdel on table-less map");
                assert!(!m.c.is_null() && !m.r.is_null());

                // row 63: hmfree on a map with hash_table == NULL
                free_maps(p, &mut m, es);
            }
        }
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 23–33 : binary maps, growth boundaries, lookups
// ===========================================================================

fn bin_key(kp: &mut KeyPool, ks: usize, v: u64) -> *mut c_void {
    let mut b = vec![0u8; ks];
    for i in 0..ks.min(8) {
        b[i] = (v >> (8 * i)) as u8;
    }
    kp.push_bytes(b)
}

fn run_binary_insert_n(p: &Pair, gseed: usize, es: usize, ks: usize, n: u64, tag: &str) {
    reseed(p, gseed);
    let cfg = Cfg {
        es,
        ks,
        mode: HM_BINARY,
        kind: KeyKind::Bytes,
    };
    let mut kp = KeyPool::new();
    let mut m = Maps::new();
    unsafe {
        for i in 0..n {
            let key = bin_key(&mut kp, ks, i.wrapping_mul(0x9E37_79B9) ^ 0x1234);
            let (a, b) = op_put(p, &mut m, &cfg, key, i as u8);
            assert_eq!(a, b, "{tag}: put #{i} temp");
            check(p, &m, &cfg, 0, &format!("{tag}: after put #{i}"));
        }
        // every key must be findable, in both, with the same index
        for i in 0..n {
            let key = bin_key(&mut kp, ks, i.wrapping_mul(0x9E37_79B9) ^ 0x1234);
            let (a, b) = op_get(p, &mut m, &cfg, key);
            assert_eq!(a, b, "{tag}: get #{i}");
            assert!(a >= 0, "{tag}: key #{i} must be present (got {a})");
            let (a, b) = op_get_ts(p, &mut m, &cfg, key);
            assert_eq!(a, b, "{tag}: get_ts #{i}");
        }
        // absent keys — chosen so their `ks`-byte encoding cannot collide with
        // any inserted key (which matters for ks == 1, where the key space is
        // only 256 wide)
        let present: std::collections::HashSet<Vec<u8>> = (0..n)
            .map(|i| {
                let mut b = vec![0u8; ks];
                let v = i.wrapping_mul(0x9E37_79B9) ^ 0x1234;
                for j in 0..ks.min(8) {
                    b[j] = (v >> (8 * j)) as u8;
                }
                b
            })
            .collect();
        let mut checked = 0usize;
        let mut probe = 0u64;
        while checked < n.max(4) as usize && probe < 100_000 {
            let mut b = vec![0u8; ks];
            let v = 0xFFFF_0000u64 + probe;
            for j in 0..ks.min(8) {
                b[j] = (v >> (8 * j)) as u8;
            }
            probe += 1;
            if present.contains(&b) {
                continue;
            }
            let key = kp.push_bytes(b);
            let (a, b2) = op_get(p, &mut m, &cfg, key);
            assert_eq!(a, b2, "{tag}: get-absent #{checked}");
            assert_eq!(a, -1, "{tag}: absent key must yield -1");
            checked += 1;
        }
        check(p, &m, &cfg, 0, &format!("{tag}: final"));
        free_maps(p, &mut m, es);
    }
}

#[test]
fn rows23_28_binary_counts_and_keysizes() {
    let (p, _g) = libs();
    for &gseed in SEED_SET.iter() {
        // rows 23/24/25/26: element-count boundaries around the growth
        // threshold (6 for 8 slots) and beyond
        for &n in [0u64, 1, 2, 5, 6, 7, 8, 9, 16, 17, 100].iter() {
            run_binary_insert_n(p, gseed, 16, 4, n, &format!("bin es16 ks4 n={n} s={gseed:#x}"));
        }
        // row 28: every key size
        for &(es, ks) in [(8usize, 1usize), (8, 2), (8, 4), (16, 8), (24, 16), (32, 16)].iter() {
            for &n in [1u64, 6, 9, 40].iter() {
                run_binary_insert_n(
                    p,
                    gseed,
                    es,
                    ks,
                    n,
                    &format!("bin es{es} ks{ks} n={n} s={gseed:#x}"),
                );
            }
        }
        // row 29: keysize == elemsize (no value payload)
        for &n in [1u64, 6, 9, 40].iter() {
            run_binary_insert_n(p, gseed, 8, 8, n, &format!("bin es==ks n={n} s={gseed:#x}"));
            run_binary_insert_n(p, gseed, 4, 4, n, &format!("bin es==ks4 n={n} s={gseed:#x}"));
        }
    }
    reseed(p, DEFAULT_SEED);
}

#[test]
fn rows27_30_31_binary_duplicates_and_seeds() {
    let (p, _g) = libs();
    let cfg = Cfg {
        es: 16,
        ks: 4,
        mode: HM_BINARY,
        kind: KeyKind::Bytes,
    };
    for &gseed in SEED_SET.iter() {
        reseed(p, gseed);
        let mut rng = Rng::new(0x8001 ^ gseed as u64);
        let mut kp = KeyPool::new();
        let mut m = Maps::new();
        unsafe {
            // 1000 random keys drawn from a small space -> many duplicates
            for i in 0..1000u64 {
                let v = rng.below(120) as u64;
                let key = bin_key(&mut kp, cfg.ks, v);
                let (a, b) = op_put(p, &mut m, &cfg, key, i as u8);
                assert_eq!(a, b, "dup-stream put #{i} (key {v})");
                if i % 37 == 0 {
                    check(p, &m, &cfg, 0, &format!("dup-stream after put #{i}"));
                }
            }
            check(p, &m, &cfg, 0, "dup-stream final");
            free_maps(p, &mut m, cfg.es);
        }
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 34–43 : deletion — memmove fix-up, tombstone rebuild, shrink
// ===========================================================================

#[test]
fn rows34_36_39_delete_last_middle_absent() {
    let (p, _g) = libs();
    let cfg = Cfg {
        es: 16,
        ks: 4,
        mode: HM_BINARY,
        kind: KeyKind::Bytes,
    };
    for &gseed in SEED_SET.iter() {
        for &n in [1usize, 2, 5, 6, 9, 20].iter() {
            for which in ["last", "first", "middle", "absent"] {
                reseed(p, gseed);
                let mut kp = KeyPool::new();
                let mut m = Maps::new();
                unsafe {
                    for i in 0..n as u64 {
                        let key = bin_key(&mut kp, cfg.ks, i * 7 + 3);
                        op_put(p, &mut m, &cfg, key, i as u8);
                    }
                    let target = match which {
                        "last" => (n as u64 - 1) * 7 + 3,
                        "first" => 3,
                        "middle" => (n as u64 / 2) * 7 + 3,
                        _ => 0xDEAD_BEEF,
                    };
                    let key = bin_key(&mut kp, cfg.ks, target);
                    let (a, b) = op_del(p, &mut m, &cfg, key, 0);
                    assert_eq!(a, b, "del {which} n={n} s={gseed:#x}: temp");
                    let expect = if which == "absent" { 0 } else { 1 };
                    assert_eq!(a, expect, "del {which} n={n}: temp must be {expect}");
                    check(p, &m, &cfg, 0, &format!("del {which} n={n} s={gseed:#x}"));
                    // everything else must still be findable and identical
                    for i in 0..n as u64 {
                        let k2 = bin_key(&mut kp, cfg.ks, i * 7 + 3);
                        let (x, y) = op_get(p, &mut m, &cfg, k2);
                        assert_eq!(x, y, "after del {which}: get #{i}");
                    }
                    check(p, &m, &cfg, 0, &format!("del {which} n={n} after gets"));
                    free_maps(p, &mut m, cfg.es);
                }
            }
        }
    }
    reseed(p, DEFAULT_SEED);
}

#[test]
fn rows37_38_41_43_tombstone_rebuild_and_shrink() {
    let (p, _g) = libs();
    let cfg = Cfg {
        es: 16,
        ks: 4,
        mode: HM_BINARY,
        kind: KeyKind::Bytes,
    };
    for &gseed in SEED_SET.iter() {
        for &n in [8usize, 20, 40, 100, 300].iter() {
            reseed(p, gseed);
            let mut kp = KeyPool::new();
            let mut m = Maps::new();
            unsafe {
                for i in 0..n as u64 {
                    let key = bin_key(&mut kp, cfg.ks, i.wrapping_mul(0x2545_F491));
                    op_put(p, &mut m, &cfg, key, i as u8);
                }
                check(p, &m, &cfg, 0, &format!("shrink n={n} s={gseed:#x} filled"));
                // delete everything, one at a time: crosses tombstone-rebuild
                // and shrink thresholds repeatedly
                for i in 0..n as u64 {
                    let key = bin_key(&mut kp, cfg.ks, i.wrapping_mul(0x2545_F491));
                    let (a, b) = op_del(p, &mut m, &cfg, key, 0);
                    assert_eq!(a, b, "shrink n={n}: del #{i} temp");
                    assert_eq!(a, 1, "shrink n={n}: del #{i} must succeed");
                    check(p, &m, &cfg, 0, &format!("shrink n={n} s={gseed:#x} after del #{i}"));
                }
                // row 39: reinsert into the tombstoned table
                for i in 0..n as u64 {
                    let key = bin_key(&mut kp, cfg.ks, i.wrapping_mul(0x2545_F491));
                    let (a, b) = op_put(p, &mut m, &cfg, key, 0xAA);
                    assert_eq!(a, b, "reinsert n={n}: put #{i} temp");
                    check(p, &m, &cfg, 0, &format!("reinsert n={n} s={gseed:#x} put #{i}"));
                }
                free_maps(p, &mut m, cfg.es);
            }
        }
    }
    reseed(p, DEFAULT_SEED);
}

#[test]
fn row40_hmdel_nonzero_keyoffset() {
    let (p, _g) = libs();
    // hmput_key always writes the key at offset 0, so to exercise
    // hmdel_key's `keyoffset` parameter self-consistently the test mirrors
    // the key bytes to offset 8 as well and deletes with keyoffset = 8.
    let cfg = Cfg {
        es: 16,
        ks: 4,
        mode: HM_BINARY,
        kind: KeyKind::Bytes,
    };
    for &gseed in SEED_SET.iter() {
        reseed(p, gseed);
        let mut kp = KeyPool::new();
        let mut m = Maps::new();
        let n = 20u64;
        unsafe {
            for i in 0..n {
                let key = bin_key(&mut kp, cfg.ks, i * 11 + 1);
                let (ic, ir) = op_put(p, &mut m, &cfg, key, i as u8);
                // mirror the key to offset 8 in both
                for (map, idx) in [(m.c, ic), (m.r, ir)] {
                    let e = (map as *mut u8).offset(cfg.es as isize * idx);
                    std::ptr::copy_nonoverlapping(key as *const u8, e.add(8), cfg.ks);
                    for k in (8 + cfg.ks)..cfg.es {
                        *e.add(k) = 0;
                    }
                }
            }
            check(p, &m, &cfg, 8, "keyoffset=8 filled");
            for i in 0..n {
                let key = bin_key(&mut kp, cfg.ks, i * 11 + 1);
                let (a, b) = op_del(p, &mut m, &cfg, key, 8);
                assert_eq!(a, b, "keyoffset=8 del #{i}");
                assert_eq!(a, 1, "keyoffset=8 del #{i} must succeed");
                check(p, &m, &cfg, 8, &format!("keyoffset=8 after del #{i}"));
            }
            free_maps(p, &mut m, cfg.es);
        }
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 41–43 : long randomised binary streams at three key widths
// ===========================================================================

fn random_binary_stream(p: &Pair, gseed: usize, es: usize, ks: usize, ops: usize, rseed: u64) {
    reseed(p, gseed);
    let cfg = Cfg {
        es,
        ks,
        mode: HM_BINARY,
        kind: KeyKind::Bytes,
    };
    let mut rng = Rng::new(rseed);
    let mut kp = KeyPool::new();
    let mut m = Maps::new();
    let space = 90u64;
    unsafe {
        for step in 0..ops {
            let v = rng.below(space as usize) as u64;
            let key = bin_key(&mut kp, ks, v.wrapping_mul(0x27BB_2EE6_87B0_B0FD));
            match rng.below(10) {
                0..=4 => {
                    let (a, b) = op_put(p, &mut m, &cfg, key, rng.u8());
                    assert_eq!(a, b, "stream es{es} ks{ks} step {step}: put temp");
                }
                5..=6 => {
                    let (a, b) = op_get(p, &mut m, &cfg, key);
                    assert_eq!(a, b, "stream es{es} ks{ks} step {step}: get temp");
                }
                7 => {
                    let (a, b) = op_get_ts(p, &mut m, &cfg, key);
                    assert_eq!(a, b, "stream es{es} ks{ks} step {step}: get_ts temp");
                }
                _ => {
                    let (a, b) = op_del(p, &mut m, &cfg, key, 0);
                    assert_eq!(a, b, "stream es{es} ks{ks} step {step}: del temp");
                }
            }
            check(
                p,
                &m,
                &cfg,
                0,
                &format!("stream es{es} ks{ks} s={gseed:#x} step {step}"),
            );
        }
        free_maps(p, &mut m, es);
    }
}

#[test]
fn rows41_43_random_binary_streams() {
    let (p, _g) = libs();
    for (i, &gseed) in SEED_SET.iter().enumerate() {
        random_binary_stream(p, gseed, 16, 4, 2000, 0x9001 + i as u64);
        random_binary_stream(p, gseed, 24, 8, 1200, 0x9101 + i as u64);
        random_binary_stream(p, gseed, 32, 16, 1200, 0x9201 + i as u64);
        random_binary_stream(p, gseed, 4, 4, 800, 0x9301 + i as u64);
        random_binary_stream(p, gseed, 8, 1, 800, 0x9401 + i as u64);
        random_binary_stream(p, gseed, 8, 2, 800, 0x9501 + i as u64);
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 44–48, 53 : string maps, SH_DEFAULT (implicit and explicit)
// ===========================================================================

fn str_keys(rng: &mut Rng, n: usize, kp: &mut KeyPool, high_bit: bool, minlen: usize, maxlen: usize) -> Vec<*mut c_void> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let l = rng.range(minlen, maxlen);
        let s = rng.cstr_bytes(l, high_bit);
        if !seen.insert(s.clone()) {
            continue;
        }
        out.push(kp.push(s));
    }
    out
}

/// Runs a full put/get/del cycle over a string-keyed map.
/// `arena_mode`: `None` -> implicit SH_DEFAULT via `hmput_key(NULL,..)`,
/// `Some(m)` -> map pre-created with `stbds_shmode_func(es, m)`.
fn run_string_map(
    p: &Pair,
    gseed: usize,
    es: usize,
    arena_mode: Option<c_int>,
    mode: c_int,
    n: usize,
    high_bit: bool,
    minlen: usize,
    maxlen: usize,
    with_empty: bool,
    tag: &str,
) {
    reseed(p, gseed);
    let cfg = Cfg {
        es,
        ks: std::mem::size_of::<usize>(),
        mode,
        kind: KeyKind::CStr,
    };
    let mut rng = Rng::new(0xA001 ^ (gseed as u64) ^ (n as u64) ^ (es as u64));
    let mut kp = KeyPool::new();
    let mut m = Maps::new();
    unsafe {
        if let Some(am) = arena_mode {
            m.c = (p.c.shmode_func)(es, am);
            m.r = (p.r.shmode_func)(es, am);
            check(p, &m, &cfg, 0, &format!("{tag}: after shmode_func"));
        }
        let mut keys = str_keys(&mut rng, n, &mut kp, high_bit, minlen, maxlen);
        if with_empty {
            keys.push(kp.push(vec![0u8]));
        }
        for (i, &k) in keys.iter().enumerate() {
            let (a, b) = op_put(p, &mut m, &cfg, k, i as u8);
            assert_eq!(a, b, "{tag}: put #{i} temp");
            check(p, &m, &cfg, 0, &format!("{tag}: after put #{i}"));
        }
        // row 46: duplicate keys — same content, DIFFERENT buffers, so
        // SH_DEFAULT must keep the original pointer
        for (i, &k) in keys.iter().enumerate() {
            let content = {
                let mut v = cstr_bytes(k as *const c_char);
                v.push(0);
                v
            };
            let dup = kp.push(content);
            let (a, b) = op_put(p, &mut m, &cfg, dup, 0x77);
            assert_eq!(a, b, "{tag}: dup put #{i} temp");
            let _ = k;
            check(p, &m, &cfg, 0, &format!("{tag}: after dup put #{i}"));
        }
        // lookups
        for (i, &k) in keys.iter().enumerate() {
            let (a, b) = op_get(p, &mut m, &cfg, k);
            assert_eq!(a, b, "{tag}: get #{i}");
            assert!(a >= 0, "{tag}: key #{i} must be present");
            let (a, b) = op_get_ts(p, &mut m, &cfg, k);
            assert_eq!(a, b, "{tag}: get_ts #{i}");
        }
        // absent lookups
        for i in 0..4usize {
            let miss = kp.push(format!("__absent_{i}__").into_bytes());
            let (a, b) = op_get(p, &mut m, &cfg, miss);
            assert_eq!(a, b, "{tag}: get-absent #{i}");
            assert_eq!(a, -1, "{tag}: absent must be -1");
        }
        check(p, &m, &cfg, 0, &format!("{tag}: before deletes"));
        // delete every key (exercises the strdup-free / memmove / rebuild paths)
        for (i, &k) in keys.iter().enumerate() {
            let (a, b) = op_del(p, &mut m, &cfg, k, 0);
            assert_eq!(a, b, "{tag}: del #{i} temp");
            assert_eq!(a, 1, "{tag}: del #{i} must succeed");
            check(p, &m, &cfg, 0, &format!("{tag}: after del #{i}"));
        }
        // delete absent
        let miss = kp.push(b"__nope__".to_vec());
        let (a, b) = op_del(p, &mut m, &cfg, miss, 0);
        assert_eq!((a, b), (0, 0), "{tag}: del-absent");
        // reinsert
        for (i, &k) in keys.iter().enumerate() {
            let (a, b) = op_put(p, &mut m, &cfg, k, 0xBB);
            assert_eq!(a, b, "{tag}: reinsert #{i} temp");
            check(p, &m, &cfg, 0, &format!("{tag}: after reinsert #{i}"));
        }
        free_maps(p, &mut m, es);
    }
}

#[test]
fn rows44_48_string_default_mode() {
    let (p, _g) = libs();
    for &gseed in SEED_SET.iter() {
        for &n in [1usize, 5, 6, 7, 9, 40].iter() {
            // row 44/45: implicit SH_DEFAULT
            run_string_map(
                p, gseed, 16, None, HM_STRING, n, false, 1, 12, false,
                &format!("shdefault-implicit n={n} s={gseed:#x}"),
            );
            // row 47: including an empty-string key
            run_string_map(
                p, gseed, 24, None, HM_STRING, n, false, 1, 12, true,
                &format!("shdefault-empty n={n} s={gseed:#x}"),
            );
            // row 48: high-bit key bytes
            run_string_map(
                p, gseed, 16, None, HM_STRING, n, true, 1, 20, false,
                &format!("shdefault-hibit n={n} s={gseed:#x}"),
            );
            // row 53: explicit SH_DEFAULT via shmode_func
            run_string_map(
                p, gseed, 16, Some(SH_DEFAULT), HM_STRING, n, false, 1, 12, false,
                &format!("shdefault-explicit n={n} s={gseed:#x}"),
            );
        }
        // larger map, multiple table doublings
        run_string_map(
            p, gseed, 16, None, HM_STRING, 100, false, 1, 24, true,
            &format!("shdefault-100 s={gseed:#x}"),
        );
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 49–51 : SH_STRDUP and SH_ARENA
// ===========================================================================

#[test]
fn row49_sh_strdup() {
    let (p, _g) = libs();
    for &gseed in SEED_SET.iter() {
        for &n in [1usize, 6, 9, 40].iter() {
            run_string_map(
                p, gseed, 16, Some(SH_STRDUP), HM_STRING, n, false, 1, 12, true,
                &format!("strdup n={n} s={gseed:#x}"),
            );
        }
        run_string_map(
            p, gseed, 24, Some(SH_STRDUP), HM_STRING, 100, true, 1, 40, false,
            &format!("strdup-100 s={gseed:#x}"),
        );
    }
    reseed(p, DEFAULT_SEED);
}

#[test]
fn rows50_51_sh_arena() {
    let (p, _g) = libs();
    for &gseed in SEED_SET.iter() {
        for &n in [1usize, 6, 9, 40].iter() {
            run_string_map(
                p, gseed, 16, Some(SH_ARENA), HM_STRING, n, false, 1, 12, true,
                &format!("arena n={n} s={gseed:#x}"),
            );
        }
        // row 51: keys long enough to blow past the 512-byte first block and
        // to hit the oversized-dedicated-block path inside stralloc
        run_string_map(
            p, gseed, 24, Some(SH_ARENA), HM_STRING, 60, false, 400, 900, false,
            &format!("arena-long s={gseed:#x}"),
        );
        run_string_map(
            p, gseed, 24, Some(SH_ARENA), HM_STRING, 30, true, 1, 2000, false,
            &format!("arena-mixed s={gseed:#x}"),
        );
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 52, 56 : string.mode values that reach hmput_key's `default:` branch
//
// With `string.mode` not in {SH_DEFAULT, SH_STRDUP, SH_ARENA} the C `default:`
// arm memcpy's `keysize` RAW BYTES of the key into the element instead of
// storing a pointer.  A later lookup of a *present* key would then make the C
// dereference those bytes as a `char*` (guaranteed crash), so this row inserts
// distinct keys only — exactly what the C can survive — and compares the full
// resulting state.
// ===========================================================================

fn run_default_branch_mode(p: &Pair, gseed: usize, sh_mode: c_int, es: usize, n: usize, tag: &str) {
    reseed(p, gseed);
    let cfg = Cfg {
        es,
        ks: std::mem::size_of::<usize>(),
        mode: HM_STRING,
        kind: KeyKind::Bytes, // the element holds raw key bytes, not a pointer
    };
    let mut rng = Rng::new(0xB001 ^ gseed as u64 ^ sh_mode as u64);
    let mut kp = KeyPool::new();
    let mut m = Maps::new();
    unsafe {
        m.c = (p.c.shmode_func)(es, sh_mode);
        m.r = (p.r.shmode_func)(es, sh_mode);
        check(p, &m, &cfg, 0, &format!("{tag}: after shmode_func({sh_mode})"));
        // distinct, long-enough keys so the memcpy of `keysize` bytes is defined
        let keys = str_keys(&mut rng, n, &mut kp, false, 16, 32);
        for (i, &k) in keys.iter().enumerate() {
            let (a, b) = op_put(p, &mut m, &cfg, k, i as u8);
            assert_eq!(a, b, "{tag}: put #{i} temp");
            check(p, &m, &cfg, 0, &format!("{tag}: after put #{i}"));
        }
        free_maps(p, &mut m, es);
    }
}

#[test]
fn rows52_56_shmode_out_of_range_and_none() {
    let (p, _g) = libs();
    for &gseed in SEED_SET.iter() {
        for &shm in [
            SH_NONE,
            4,
            5,
            255,
            256,   // -> (unsigned char) 0
            1000,  // -> 232
            -1,    // -> 255
            i32::MIN,
            i32::MAX,
        ]
        .iter()
        {
            for &n in [1usize, 6, 9, 20].iter() {
                run_default_branch_mode(
                    p,
                    gseed,
                    shm,
                    24,
                    n,
                    &format!("shmode={shm} n={n} s={gseed:#x}"),
                );
            }
        }
        // row 60(err): elemsize == 0 through shmode_func
        unsafe {
            reseed(p, gseed);
            let a = (p.c.shmode_func)(0, SH_DEFAULT);
            let b = (p.r.shmode_func)(0, SH_DEFAULT);
            assert_eq!(a.is_null(), b.is_null(), "shmode_func(0,..) NULL-ness");
            let ha = hdr_of(map_arr(a, 0));
            let hb = hdr_of(map_arr(b, 0));
            assert_eq!(ha.length, hb.length, "shmode_func(0): length");
            assert_eq!(ha.capacity, hb.capacity, "shmode_func(0): capacity");
            assert_eq!(ha.temp, hb.temp, "shmode_func(0): temp");
            let ia = idx_snap(ha.hash_table as *mut HashIndex);
            let ib = idx_snap(hb.hash_table as *mut HashIndex);
            assert_eq!(ia, ib, "shmode_func(0): hash index");
            (p.c.hmfree_func)(map_arr(a, 0), 0);
            (p.r.hmfree_func)(map_arr(b, 0), 0);
        }
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 54–55 : out-of-range `mode` int across the FFI boundary
// ===========================================================================

#[test]
fn row54_mode_ge_2_is_string_in_put_but_binary_in_del() {
    let (p, _g) = libs();
    // mode >= 1 -> string path in put/get.  In hmdel_key the C tests
    // `mode == STBDS_HM_STRING` exactly, so mode >= 2 takes the *binary*
    // re-find branch.  That branch reads the element bytes as a key, which is
    // only well-defined when the deleted element is the LAST one (no memmove /
    // re-find happens at all) — so only that case is exercised.
    for &mode in [2i32, 3, 7, 1000, i32::MAX].iter() {
        for &gseed in SEED_SET.iter() {
            reseed(p, gseed);
            let cfg = Cfg {
                es: 16,
                ks: std::mem::size_of::<usize>(),
                mode,
                kind: KeyKind::CStr,
            };
            let mut rng = Rng::new(0xC001 ^ mode as u64 ^ gseed as u64);
            let mut kp = KeyPool::new();
            let mut m = Maps::new();
            unsafe {
                let keys = str_keys(&mut rng, 12, &mut kp, false, 1, 16);
                for (i, &k) in keys.iter().enumerate() {
                    let (a, b) = op_put(p, &mut m, &cfg, k, i as u8);
                    assert_eq!(a, b, "mode={mode} put #{i}");
                    check(p, &m, &cfg, 0, &format!("mode={mode} after put #{i}"));
                }
                // string path in get
                for (i, &k) in keys.iter().enumerate() {
                    let (a, b) = op_get(p, &mut m, &cfg, k);
                    assert_eq!(a, b, "mode={mode} get #{i}");
                    assert!(a >= 0, "mode={mode} key #{i} must be found (string path)");
                }
                // delete from the back so old_index == final_index every time
                for (i, &k) in keys.iter().enumerate().rev() {
                    let (a, b) = op_del(p, &mut m, &cfg, k, 0);
                    assert_eq!(a, b, "mode={mode} del #{i}");
                    assert_eq!(a, 1, "mode={mode} del #{i} must succeed");
                    check(p, &m, &cfg, 0, &format!("mode={mode} after del #{i}"));
                }
                free_maps(p, &mut m, cfg.es);
            }
        }
    }
    reseed(p, DEFAULT_SEED);
}

#[test]
fn row55_negative_mode_is_binary() {
    let (p, _g) = libs();
    for &mode in [-1i32, -7, i32::MIN].iter() {
        for &gseed in SEED_SET.iter() {
            reseed(p, gseed);
            let cfg = Cfg {
                es: 16,
                ks: 4,
                mode,
                kind: KeyKind::Bytes,
            };
            let mut kp = KeyPool::new();
            let mut m = Maps::new();
            unsafe {
                for i in 0..30u64 {
                    let key = bin_key(&mut kp, cfg.ks, i * 13 + 5);
                    let (a, b) = op_put(p, &mut m, &cfg, key, i as u8);
                    assert_eq!(a, b, "mode={mode} put #{i}");
                    check(p, &m, &cfg, 0, &format!("mode={mode} after put #{i}"));
                }
                for i in 0..30u64 {
                    let key = bin_key(&mut kp, cfg.ks, i * 13 + 5);
                    let (a, b) = op_get(p, &mut m, &cfg, key);
                    assert_eq!(a, b, "mode={mode} get #{i}");
                    assert!(a >= 0, "mode={mode} binary lookup must find key #{i}");
                }
                for i in 0..30u64 {
                    let key = bin_key(&mut kp, cfg.ks, i * 13 + 5);
                    let (a, b) = op_del(p, &mut m, &cfg, key, 0);
                    assert_eq!(a, b, "mode={mode} del #{i}");
                    check(p, &m, &cfg, 0, &format!("mode={mode} after del #{i}"));
                }
                free_maps(p, &mut m, cfg.es);
            }
        }
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 57–59 : long randomised string streams over all three arena modes
// ===========================================================================

fn random_string_stream(
    p: &Pair,
    gseed: usize,
    arena_mode: Option<c_int>,
    es: usize,
    ops: usize,
    rseed: u64,
    maxlen: usize,
    tag: &str,
) {
    reseed(p, gseed);
    let cfg = Cfg {
        es,
        ks: std::mem::size_of::<usize>(),
        mode: HM_STRING,
        kind: KeyKind::CStr,
    };
    let mut rng = Rng::new(rseed);
    let mut kp = KeyPool::new();
    let mut m = Maps::new();
    unsafe {
        if let Some(am) = arena_mode {
            m.c = (p.c.shmode_func)(es, am);
            m.r = (p.r.shmode_func)(es, am);
        }
        // a fixed pool of distinct key *contents*; each use gets a fresh
        // buffer so SH_DEFAULT's pointer-retention behaviour is observable
        let pool = str_keys(&mut rng, 60, &mut kp, false, 1, maxlen);
        let contents: Vec<Vec<u8>> = pool.iter().map(|&k| cstr_bytes(k as *const c_char)).collect();
        for step in 0..ops {
            let j = rng.below(contents.len());
            let mut fresh = contents[j].clone();
            fresh.push(0);
            let key = kp.push(fresh);
            match rng.below(10) {
                0..=4 => {
                    let (a, b) = op_put(p, &mut m, &cfg, key, rng.u8());
                    assert_eq!(a, b, "{tag} step {step}: put temp");
                }
                5..=6 => {
                    let (a, b) = op_get(p, &mut m, &cfg, key);
                    assert_eq!(a, b, "{tag} step {step}: get temp");
                }
                7 => {
                    let (a, b) = op_get_ts(p, &mut m, &cfg, key);
                    assert_eq!(a, b, "{tag} step {step}: get_ts temp");
                }
                _ => {
                    let (a, b) = op_del(p, &mut m, &cfg, key, 0);
                    assert_eq!(a, b, "{tag} step {step}: del temp");
                }
            }
            check(p, &m, &cfg, 0, &format!("{tag} step {step}"));
        }
        free_maps(p, &mut m, es);
    }
}

#[test]
fn row57_random_stream_sh_default() {
    let (p, _g) = libs();
    for (i, &gseed) in SEED_SET.iter().enumerate() {
        random_string_stream(p, gseed, None, 16, 1000, 0xD001 + i as u64, 12, "str-implicit");
        random_string_stream(
            p,
            gseed,
            Some(SH_DEFAULT),
            24,
            800,
            0xD101 + i as u64,
            24,
            "str-explicit",
        );
    }
    reseed(p, DEFAULT_SEED);
}

#[test]
fn row58_random_stream_sh_strdup() {
    let (p, _g) = libs();
    for (i, &gseed) in SEED_SET.iter().enumerate() {
        random_string_stream(
            p,
            gseed,
            Some(SH_STRDUP),
            16,
            1000,
            0xE001 + i as u64,
            16,
            "strdup-stream",
        );
        random_string_stream(
            p,
            gseed,
            Some(SH_STRDUP),
            32,
            600,
            0xE101 + i as u64,
            300,
            "strdup-stream-long",
        );
    }
    reseed(p, DEFAULT_SEED);
}

#[test]
fn row59_random_stream_sh_arena() {
    let (p, _g) = libs();
    for (i, &gseed) in SEED_SET.iter().enumerate() {
        random_string_stream(
            p,
            gseed,
            Some(SH_ARENA),
            16,
            1000,
            0xF001 + i as u64,
            16,
            "arena-stream",
        );
        random_string_stream(
            p,
            gseed,
            Some(SH_ARENA),
            32,
            600,
            0xF101 + i as u64,
            900,
            "arena-stream-long",
        );
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 60–63 : stbds_hmfree_func over every arena mode
// ===========================================================================

#[test]
fn rows60_63_hmfree_all_modes() {
    let (p, _g) = libs();
    for &gseed in SEED_SET.iter() {
        for arena_mode in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            for &n in [0usize, 1, 6, 100].iter() {
                reseed(p, gseed);
                let es = 24usize;
                let cfg = Cfg {
                    es,
                    ks: std::mem::size_of::<usize>(),
                    mode: HM_STRING,
                    kind: KeyKind::CStr,
                };
                let mut rng = Rng::new(0x1234 ^ gseed as u64 ^ n as u64);
                let mut kp = KeyPool::new();
                let mut m = Maps::new();
                unsafe {
                    if let Some(am) = arena_mode {
                        m.c = (p.c.shmode_func)(es, am);
                        m.r = (p.r.shmode_func)(es, am);
                    }
                    let keys = str_keys(&mut rng, n, &mut kp, false, 1, 700);
                    for (i, &k) in keys.iter().enumerate() {
                        op_put(p, &mut m, &cfg, k, i as u8);
                    }
                    check(
                        p,
                        &m,
                        &cfg,
                        0,
                        &format!("hmfree am={arena_mode:?} n={n} s={gseed:#x}"),
                    );
                    free_maps(p, &mut m, es);
                }
            }
        }
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// row 78 : several live maps + raw arrays interleaved in one "program"
//
// Real consumers keep more than one container alive at once.  Because each
// fresh `stbds_hash_index` consumes and then advances the single process-global
// `stbds_hash_seed` (lib.c:410-412), the seed a given map ends up with depends
// on the interleaving of *all* container creations — state a per-map test can
// never reach.
// ===========================================================================

#[test]
fn row78_interleaved_multiple_maps_and_arrays() {
    let (p, _g) = libs();
    for (t, &gseed) in SEED_SET.iter().enumerate() {
        reseed(p, gseed);
        let mut rng = Rng::new(0x7E78 ^ gseed as u64 ^ t as u64);

        // five maps with different shapes / arena modes, all alive at once
        let cfgs = [
            (Cfg { es: 16, ks: 4, mode: HM_BINARY, kind: KeyKind::Bytes }, None),
            (Cfg { es: 24, ks: 8, mode: HM_BINARY, kind: KeyKind::Bytes }, None),
            (Cfg { es: 16, ks: 8, mode: HM_STRING, kind: KeyKind::CStr }, None),
            (Cfg { es: 24, ks: 8, mode: HM_STRING, kind: KeyKind::CStr }, Some(SH_STRDUP)),
            (Cfg { es: 32, ks: 8, mode: HM_STRING, kind: KeyKind::CStr }, Some(SH_ARENA)),
        ];
        let mut maps: Vec<Maps> = Vec::new();
        let mut kp = KeyPool::new();
        unsafe {
            for (cfg, am) in cfgs.iter() {
                let mut m = Maps::new();
                if let Some(a) = am {
                    m.c = (p.c.shmode_func)(cfg.es, *a);
                    m.r = (p.r.shmode_func)(cfg.es, *a);
                }
                maps.push(m);
            }
            // two raw arrays too, grown in the middle of everything
            let mut arr_c = std::ptr::null_mut::<c_void>();
            let mut arr_r = std::ptr::null_mut::<c_void>();

            for step in 0..1500usize {
                let which = rng.below(cfgs.len());
                let (cfg, _) = &cfgs[which];
                let key = if cfg.kind == KeyKind::CStr {
                    let n = rng.range(1, 30);
                    let s = rng.cstr_bytes(n, false);
                    kp.push(s)
                } else {
                    bin_key(&mut kp, cfg.ks, rng.below(80) as u64)
                };
                match rng.below(12) {
                    0..=5 => {
                        let (a, b) = op_put(p, &mut maps[which], cfg, key, rng.u8());
                        assert_eq!(a, b, "row78 s={gseed:#x} step {step}: put map {which}");
                    }
                    6..=7 => {
                        let (a, b) = op_get(p, &mut maps[which], cfg, key);
                        assert_eq!(a, b, "row78 s={gseed:#x} step {step}: get map {which}");
                    }
                    8 => {
                        let (a, b) = op_get_ts(p, &mut maps[which], cfg, key);
                        assert_eq!(a, b, "row78 s={gseed:#x} step {step}: get_ts map {which}");
                    }
                    9..=10 => {
                        let (a, b) = op_del(p, &mut maps[which], cfg, key, 0);
                        assert_eq!(a, b, "row78 s={gseed:#x} step {step}: del map {which}");
                    }
                    _ => {
                        // interleave a raw array growth (shares no state, but
                        // proves the two allocators stay in lockstep)
                        let addlen = rng.below(8);
                        let mc = rng.below(20);
                        arr_c = (p.c.arrgrowf)(arr_c, 8, addlen, mc);
                        arr_r = (p.r.arrgrowf)(arr_r, 8, addlen, mc);
                        assert_eq!(
                            arr_c.is_null(),
                            arr_r.is_null(),
                            "row78 step {step}: arrgrowf NULL-ness"
                        );
                        if !arr_c.is_null() {
                            assert_eq!(
                                hdr_of(arr_c).capacity,
                                hdr_of(arr_r).capacity,
                                "row78 step {step}: arr capacity"
                            );
                            let l = rng.below(hdr_of(arr_c).capacity + 1);
                            (*((arr_c as *mut u8).sub(HDR) as *mut Header)).length = l;
                            (*((arr_r as *mut u8).sub(HDR) as *mut Header)).length = l;
                        }
                    }
                }
                // ALL maps must match after every step, not just the touched one
                for (i, (cfg2, _)) in cfgs.iter().enumerate() {
                    check(
                        p,
                        &maps[i],
                        cfg2,
                        0,
                        &format!("row78 s={gseed:#x} step {step}: map {i}"),
                    );
                }
            }
            for (i, (cfg, _)) in cfgs.iter().enumerate() {
                free_maps(p, &mut maps[i], cfg.es);
            }
            if !arr_c.is_null() {
                (p.c.arrfreef)(arr_c);
            }
            if !arr_r.is_null() {
                (p.r.arrfreef)(arr_r);
            }
        }
    }
    reseed(p, DEFAULT_SEED);
}
