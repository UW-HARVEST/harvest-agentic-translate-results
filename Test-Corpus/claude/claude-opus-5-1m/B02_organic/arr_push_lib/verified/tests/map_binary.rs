//! Phase B, CONFIGS.md rows 20-39: binary-mode (`STBDS_HM_BINARY`) hash map,
//! driven through the low-level exported entry points via the macro-equivalent
//! driver in `common::map`.

mod common;
use common::map::*;
use common::*;
use std::ffi::c_void;

/// A stable arena of key buffers (their heap allocations must outlive the map
/// because `SH_DEFAULT` string mode stores the caller's pointer).
struct Keys(Vec<Vec<u8>>);
impl Keys {
    fn new() -> Self {
        Keys(Vec::new())
    }
    fn add(&mut self, bytes: Vec<u8>) -> *mut c_void {
        self.0.push(bytes);
        self.0.last_mut().unwrap().as_mut_ptr() as *mut c_void
    }
    fn int(&mut self, v: i32) -> *mut c_void {
        self.add(v.to_le_bytes().to_vec())
    }
}

fn le(v: u64, n: usize) -> Vec<u8> {
    v.to_le_bytes()[..n].to_vec()
}

// ---------------------------------------------------------------------------
// rows 20-23 — entry counts at and around every grow threshold
// ---------------------------------------------------------------------------
#[test]
fn cfg20_23_binary_grow_thresholds() {
    // used_count_threshold = slot_count - (slot_count>>2): 6 @8, 12 @16, 24 @32
    for n in [
        1usize, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14, 23, 24, 25, 26, 47, 48, 49, 50, 96, 100,
    ] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg::int_int();
        let mut m = MapPair::empty(p, cfg);
        let mut keys = Keys::new();
        unsafe {
            for i in 0..n {
                let k = keys.int(i as i32 * 3 + 1);
                m.put(k, &le(0xdead0000 + i as u64, 4), &format!("n={n} put {i}"));
                assert_eq_ctx(m.hmlen(&format!("n={n} i={i}")), i as isize + 1, "hmlen");
            }
            // every key must be found, with the same index
            for i in 0..n {
                let mut kb = (i as i32 * 3 + 1).to_le_bytes();
                let idx = m.geti(kb.as_mut_ptr() as *mut c_void, &format!("n={n} get {i}"));
                assert!(idx >= 0, "key {i} lost (n={n})");
                m.check_val(idx, &format!("n={n} val {i}"));
            }
            // absent keys must miss identically
            for i in 0..n + 8 {
                let mut kb = (i as i32 * 3 + 2).to_le_bytes();
                let idx = m.geti(kb.as_mut_ptr() as *mut c_void, &format!("n={n} miss {i}"));
                assert_eq!(idx, -1, "key {i}*3+2 should be absent (n={n})");
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 24 — 500 randomized entries including duplicate keys
// ---------------------------------------------------------------------------
#[test]
fn cfg24_binary_random_500() {
    let (p, _g) = session(0x2424);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut keys = Keys::new();
    let mut r = Rng::new(0x240024);
    unsafe {
        for i in 0..500usize {
            // deliberately narrow key space so duplicates occur often
            let k = keys.int((r.below(200)) as i32);
            m.put(k, &le(r.u64(), 4), &format!("put {i}"));
        }
        for v in 0..250i32 {
            let mut kb = v.to_le_bytes();
            m.geti(kb.as_mut_ptr() as *mut c_void, &format!("get {v}"));
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 25-27 — other element/key shapes
// ---------------------------------------------------------------------------
#[test]
fn cfg25_26_27_binary_other_shapes() {
    // (elemsize, keysize, valoffset, valsize)
    // INVARIANT: keysize <= valoffset and valoffset+valsize == elemsize, so the
    // whole element is written and the snapshot never reads padding (which the
    // C leaves indeterminate).
    let shapes: &[(usize, usize, usize, usize)] = &[
        (16, 8, 8, 8),    // row 25: struct { size_t key; long value; }
        (32, 16, 16, 16), // row 26: struct { int key[4]; int value[4]; }
        (4, 4, 0, 0),     // row 27: element IS the key (no value at all)
        (8, 8, 0, 0),     //         ditto, 8-byte key
        (1, 1, 0, 0),
        (2, 2, 0, 0),
        (40, 40, 0, 0),
        (12, 4, 4, 8),
        (24, 12, 12, 12),
        (64, 32, 32, 32),
        (3, 1, 1, 2),
        (48, 8, 8, 40),
    ];
    for &(es, ks, vo, vs) in shapes {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg {
            elemsize: es,
            keysize: ks,
            keyoffset: 0,
            mode: STBDS_HM_BINARY,
            valoffset: vo,
            valsize: vs,
            force_raw_snap: false,
        };
        let mut m = MapPair::empty(p, cfg);
        let mut keys = Keys::new();
        let mut r = Rng::new(0x250025 + es as u64 * 131 + ks as u64);
        let mut inserted: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for i in 0..200usize {
                let kb = r.bytes(ks);
                inserted.push(kb.clone());
                let k = keys.add(kb);
                let v = r.bytes(vs);
                m.put(k, &v, &format!("es={es} ks={ks} put {i}"));
            }
            for (i, kb) in inserted.iter().enumerate() {
                let mut kb = kb.clone();
                let idx = m.geti(
                    kb.as_mut_ptr() as *mut c_void,
                    &format!("es={es} ks={ks} get {i}"),
                );
                assert!(idx >= 0, "es={es} ks={ks}: key {i} lost");
            }
            for i in 0..50 {
                let mut kb = r.bytes(ks);
                m.geti(
                    kb.as_mut_ptr() as *mut c_void,
                    &format!("es={es} ks={ks} rndget {i}"),
                );
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 28 — the `_ts` variant must NOT write header->temp
// ---------------------------------------------------------------------------
#[test]
fn cfg28_binary_hmget_key_ts() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut keys = Keys::new();
    let mut r = Rng::new(0x280028);
    unsafe {
        // NULL map first: _ts on a NULL map allocates and sets *temp = -1
        let mut k0 = 7i32.to_le_bytes();
        let t = m.geti_ts(k0.as_mut_ptr() as *mut c_void, "ts on NULL map");
        assert_eq!(t, -1);

        for i in 0..300usize {
            let k = keys.int(r.below(120) as i32);
            m.put(k, &le(r.u64(), 4), &format!("put {i}"));
            // read back with _ts, and compare BOTH *temp and header->temp
            let mut kb = (r.below(140) as i32).to_le_bytes();
            m.geti_ts(kb.as_mut_ptr() as *mut c_void, &format!("ts get {i}"));
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 29 — keysize = 0 (memcmp of 0 bytes is always equal)
// ---------------------------------------------------------------------------
#[test]
fn cfg29_binary_keysize_zero() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    // keysize == 0 means hmput_key writes NOTHING into the element, so the
    // driver's value must cover all 8 bytes or the snapshot reads padding.
    let cfg = MapCfg {
        elemsize: 8,
        keysize: 0,
        keyoffset: 0,
        mode: STBDS_HM_BINARY,
        valoffset: 0,
        valsize: 8,
        force_raw_snap: false,
    };
    let mut m = MapPair::empty(p, cfg);
    let mut keys = Keys::new();
    let mut r = Rng::new(0x290029);
    unsafe {
        for i in 0..40usize {
            let k = keys.int(r.u32() as i32);
            let idx = m.put(k, &le(i as u64, 8), &format!("put {i}"));
            // every key collapses onto the single entry
            assert_eq!(idx, 0, "keysize=0 must collapse all keys, put {i}");
        }
        assert_eq!(m.hmlen("keysize0"), 1);
        for i in 0..10 {
            let mut kb = (r.u32() as i32).to_le_bytes();
            let idx = m.geti(kb.as_mut_ptr() as *mut c_void, &format!("get {i}"));
            assert_eq!(idx, 0);
        }
        let mut kb = 0i32.to_le_bytes();
        m.del(kb.as_mut_ptr() as *mut c_void, "del keysize0");
        assert_eq!(m.hmlen("keysize0 after del"), 0);
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 30 — probe paths: forward half, wrap-around half, multi-bucket
// ---------------------------------------------------------------------------
#[test]
fn cfg30_binary_probe_paths() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut wrap_hits = 0usize;
    let mut multi_bucket = 0usize;
    let mut r = Rng::new(0x300030);
    for trial in 0..60usize {
        let mut m = MapPair::empty(p, cfg);
        let mut keys = Keys::new();
        unsafe {
            let n = r.range(1, 40);
            for i in 0..n {
                let k = keys.int(r.u32() as i32);
                m.put(k, &le(r.u64(), 4), &format!("trial {trial} put {i}"));
            }
            // structural coverage: how many live entries sit at a slot index
            // *below* their natural probe position (only reachable through the
            // wrap-around `i < limit` loop)
            let t = m.c.t as *mut u8;
            let h = t.sub(cfg.elemsize).sub(HDR_SIZE);
            let tbl = rd_ptr(h, HDR_HASH_TABLE);
            if !tbl.is_null() {
                let sc = rd_usize(tbl, HI_SLOT_COUNT);
                let storage = rd_ptr(tbl, HI_STORAGE);
                for b in 0..(sc >> 3) {
                    let bp = storage.add(b * BUCKET_SIZE);
                    for j in 0..BUCKET_LENGTH {
                        let hash = rd_usize(bp, j * 8);
                        let idx = rd_isize(bp, 64 + j * 8);
                        if idx >= 0 && hash >= 2 {
                            let natural = hash & (sc - 1);
                            let actual = b * 8 + j;
                            if actual < natural {
                                wrap_hits += 1;
                            }
                            if (actual >> 3) != (natural >> 3) {
                                multi_bucket += 1;
                            }
                        }
                    }
                }
            }
            m.free();
        }
    }
    assert!(
        wrap_hits > 0,
        "test did not cover the wrap-around probe half (wrap_hits=0)"
    );
    eprintln!("probe coverage: wrap_hits={wrap_hits} multi_bucket={multi_bucket}");
}

// ---------------------------------------------------------------------------
// row 31 — hmput_default combined with puts and gets
// ---------------------------------------------------------------------------
#[test]
fn cfg31_binary_hmdefault() {
    for order in 0..3 {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg::int_int();
        let mut m = MapPair::empty(p, cfg);
        let mut keys = Keys::new();
        let mut r = Rng::new(0x310031 + order);
        unsafe {
            if order == 0 {
                m.set_default(&le(0xbadf00d, 4), "default first");
            }
            for i in 0..20usize {
                if order == 1 && i == 5 {
                    m.set_default(&le(0xbadf00d, 4), "default mid");
                }
                let k = keys.int(r.below(30) as i32);
                m.put(k, &le(r.u64(), 4), &format!("order={order} put {i}"));
            }
            if order == 2 {
                m.set_default(&le(0xbadf00d, 4), "default last");
            }
            // a miss must leave header->temp == -1 so that `hmget` reads t[-1]
            let mut kb = 0x7fff_ffffi32.to_le_bytes();
            let idx = m.geti(kb.as_mut_ptr() as *mut c_void, "miss");
            assert_eq!(idx, -1);
            m.check_val(-1, "default slot value");
            // repeated hmdefault on a non-empty map must be a no-op
            m.set_default(&le(0x1234, 4), "default again");
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 32-34 — deletes: only entry, last entry, middle entry
// ---------------------------------------------------------------------------
#[test]
fn cfg32_33_34_binary_delete_positions() {
    for n in [1usize, 2, 3, 5, 6, 7, 12, 13, 20] {
        for del_pos in 0..n {
            let (p, _g) = session(INITIAL_HASH_SEED);
            let cfg = MapCfg::int_int();
            let mut m = MapPair::empty(p, cfg);
            let mut keys = Keys::new();
            unsafe {
                for i in 0..n {
                    let k = keys.int(i as i32 * 7 + 3);
                    m.put(k, &le(i as u64, 4), &format!("n={n} put {i}"));
                }
                let mut kb = (del_pos as i32 * 7 + 3).to_le_bytes();
                let rc = m.del(
                    kb.as_mut_ptr() as *mut c_void,
                    &format!("n={n} del {del_pos}"),
                );
                assert_eq!(rc, 1, "delete of a present key must report 1");
                assert_eq!(m.hmlen(&format!("n={n}")), n as isize - 1);
                // the deleted key is gone, all others remain
                let idx = m.geti(
                    kb.as_mut_ptr() as *mut c_void,
                    &format!("n={n} get deleted {del_pos}"),
                );
                assert_eq!(idx, -1);
                for i in 0..n {
                    if i == del_pos {
                        continue;
                    }
                    let mut k2 = (i as i32 * 7 + 3).to_le_bytes();
                    let idx = m.geti(
                        k2.as_mut_ptr() as *mut c_void,
                        &format!("n={n} survivor {i}"),
                    );
                    assert!(idx >= 0, "n={n}: survivor {i} lost after deleting {del_pos}");
                }
                m.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 35-36 — tombstone rebuild and shrink
// ---------------------------------------------------------------------------
#[test]
fn cfg35_36_binary_rebuild_and_shrink() {
    // tombstone_count_threshold = (sc>>3)+(sc>>4): 1 @8, 3 @16, 6 @32
    // used_count_shrink_threshold = sc>>2 (0 when sc<=8): 4 @16, 8 @32
    for n in [8usize, 16, 20, 30, 40, 60] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg::int_int();
        let mut m = MapPair::empty(p, cfg);
        let mut keys = Keys::new();
        unsafe {
            for i in 0..n {
                let k = keys.int(i as i32);
                m.put(k, &le(i as u64, 4), &format!("n={n} put {i}"));
            }
            let sc_before = slot_count(&m.c, cfg.elemsize);
            // delete every key one at a time: crosses the tombstone threshold
            // repeatedly and then the shrink threshold
            for i in 0..n {
                let mut kb = (i as i32).to_le_bytes();
                let rc = m.del(
                    kb.as_mut_ptr() as *mut c_void,
                    &format!("n={n} del {i} (sc_before={sc_before})"),
                );
                assert_eq!(rc, 1);
                // survivors still reachable
                for j in (i + 1)..n {
                    let mut k2 = (j as i32).to_le_bytes();
                    let idx = m.geti(
                        k2.as_mut_ptr() as *mut c_void,
                        &format!("n={n} after del {i}, survivor {j}"),
                    );
                    assert!(idx >= 0, "n={n}: key {j} lost after deleting {i}");
                }
            }
            assert_eq!(m.hmlen(&format!("n={n} drained")), 0);
            m.free();
        }
    }
}

unsafe fn slot_count(m: &Map, elemsize: usize) -> usize {
    let h = (m.t as *mut u8).sub(elemsize).sub(HDR_SIZE);
    let tbl = rd_ptr(h, HDR_HASH_TABLE);
    if tbl.is_null() {
        0
    } else {
        rd_usize(tbl, HI_SLOT_COUNT)
    }
}

// ---------------------------------------------------------------------------
// row 37 — long randomized churn (grow, shrink, rebuild, tombstone reuse)
// ---------------------------------------------------------------------------
#[test]
fn cfg37_binary_churn() {
    for (seed, keyspace) in [(0x370037u64, 40usize), (0x370038, 200), (0x370039, 1500)] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg::int_int();
        let mut m = MapPair::empty(p, cfg);
        let mut keys = Keys::new();
        let mut r = Rng::new(seed);
        let mut model: std::collections::HashSet<i32> = std::collections::HashSet::new();
        unsafe {
            for op in 0..2000usize {
                let kv = r.below(keyspace) as i32;
                match r.below(10) {
                    0..=4 => {
                        let k = keys.int(kv);
                        m.put(k, &le(r.u64(), 4), &format!("s={seed:#x} op {op} put {kv}"));
                        model.insert(kv);
                    }
                    5..=7 => {
                        let mut kb = kv.to_le_bytes();
                        let idx = m.geti(
                            kb.as_mut_ptr() as *mut c_void,
                            &format!("s={seed:#x} op {op} get {kv}"),
                        );
                        assert_eq!(
                            idx >= 0,
                            model.contains(&kv),
                            "op {op}: presence of {kv} disagrees with the model"
                        );
                    }
                    _ => {
                        let mut kb = kv.to_le_bytes();
                        let rc = m.del(
                            kb.as_mut_ptr() as *mut c_void,
                            &format!("s={seed:#x} op {op} del {kv}"),
                        );
                        assert_eq!(
                            rc,
                            model.contains(&kv) as isize,
                            "op {op}: del({kv}) return code vs model"
                        );
                        model.remove(&kv);
                    }
                }
                assert_eq!(
                    m.hmlen(&format!("op {op}")),
                    model.len() as isize,
                    "op {op}: hmlen vs model"
                );
            }
            // final full verification
            for kv in 0..keyspace as i32 {
                let mut kb = kv.to_le_bytes();
                let idx = m.geti(kb.as_mut_ptr() as *mut c_void, &format!("final get {kv}"));
                assert_eq!(idx >= 0, model.contains(&kv), "final: {kv}");
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 38 — hmdel_key with a non-zero keyoffset (key duplicated at offset 8)
// ---------------------------------------------------------------------------
#[test]
fn cfg38_binary_nonzero_keyoffset() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    // element: [0..4) key (what hmput_key writes), [4..8) key copy -- the
    // offset the delete looks at, [8..12) value.
    // The driver's value spans [4..12) so that a freshly inserted element is
    // fully initialised at the moment the snapshot is taken; `sync` then
    // overwrites [4..8) with the key copy.  Every byte is deterministic.
    let cfg = MapCfg {
        elemsize: 12,
        keysize: 4,
        keyoffset: 4,
        mode: STBDS_HM_BINARY,
        valoffset: 4,
        valsize: 8,
        force_raw_snap: false,
    };
    let mut m = MapPair::empty(p, cfg);
    let mut keys = Keys::new();
    let mut r = Rng::new(0x380038);
    let mut live: Vec<i32> = Vec::new();
    unsafe {
        // `sync_key_copy` mirrors what a struct-with-duplicated-key would do
        let sync = |m: &MapPair| {
            for mm in [&m.c, &m.rs] {
                let len = mm.raw_len();
                for i in 1..len {
                    let e = mm.elem(i as isize - 1);
                    std::ptr::copy_nonoverlapping(e, e.add(4), 4);
                }
            }
        };
        for i in 0..40usize {
            let kv = r.below(30) as i32;
            let k = keys.int(kv);
            m.put(k, &le(r.u64(), 8), &format!("put {i}"));
            sync(&m);
            if !live.contains(&kv) {
                live.push(kv);
            }
        }
        // now delete with keyoffset = 8; the duplicated key makes it consistent
        while let Some(kv) = live.pop() {
            let mut kb = kv.to_le_bytes();
            let rc = m.del(kb.as_mut_ptr() as *mut c_void, &format!("del {kv}"));
            assert_eq!(rc, 1, "del({kv}) with keyoffset=4 should find the key");
            sync(&m);
        }
        assert_eq!(m.hmlen("drained"), 0);
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 39 — hmfree_func after churn
// ---------------------------------------------------------------------------
#[test]
fn cfg39_binary_hmfree_after_churn() {
    for round in 0..10u64 {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg::int_int();
        let mut m = MapPair::empty(p, cfg);
        let mut keys = Keys::new();
        let mut r = Rng::new(0x390039 + round);
        unsafe {
            for i in 0..200usize {
                let k = keys.int(r.below(80) as i32);
                m.put(k, &le(r.u64(), 4), &format!("round {round} put {i}"));
                if r.below(3) == 0 {
                    let mut kb = (r.below(80) as i32).to_le_bytes();
                    m.del(
                        kb.as_mut_ptr() as *mut c_void,
                        &format!("round {round} del {i}"),
                    );
                }
            }
            m.free();
            assert!(m.c.t.is_null() && m.rs.t.is_null());
        }
    }
    // freeing a map that never got a hash table (arrgrowf-only / hmput_default)
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [4usize, 8, 16] {
            let a = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let b = (p.rs.hmput_default)(std::ptr::null_mut(), es);
            assert_snap_eq(
                &snap_map(a, es, KeyRepr::Raw),
                &snap_map(b, es, KeyRepr::Raw),
                &format!("hmput_default(NULL, {es})"),
            );
            (p.c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}
