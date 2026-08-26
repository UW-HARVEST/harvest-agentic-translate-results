//! Phase B, CONFIGS.md rows 65-67: ABI/layout equivalence, global-seed
//! lockstep, and interleaved maps sharing the one global seed.

mod common;
use common::map::*;
use common::*;
use std::ffi::c_void;

// ---------------------------------------------------------------------------
// row 65 — struct layout / ABI
// ---------------------------------------------------------------------------

/// `stbds_array_header` is 32 bytes laid out length/capacity/hash_table/temp.
/// If the size or any offset differed, these reads would not produce the values
/// the C code demonstrably writes.
#[test]
fn cfg65a_array_header_layout() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for es in [1usize, 4, 8, 16, 32, 64] {
            for (lib, tag) in [(&p.c, "C"), (&p.rs, "Rust")] {
                let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 5, 0) as *mut u8;
                let h = a.sub(HDR_SIZE);
                assert_eq!(rd_usize(h, HDR_LENGTH), 0, "{tag} es={es} length@0");
                assert_eq!(rd_usize(h, HDR_CAPACITY), 5, "{tag} es={es} capacity@8");
                assert!(
                    rd_ptr(h, HDR_HASH_TABLE).is_null(),
                    "{tag} es={es} hash_table@16"
                );
                assert_eq!(rd_isize(h, HDR_TEMP), 0, "{tag} es={es} temp@24");
                // growing must preserve length/hash_table/temp and only bump cap
                wr_usize(h, HDR_LENGTH, 3);
                (h.add(HDR_TEMP) as *mut isize).write_unaligned(-42);
                let a2 = (lib.arrgrowf)(a as *mut c_void, es, 100, 0) as *mut u8;
                let h2 = a2.sub(HDR_SIZE);
                assert_eq!(rd_usize(h2, HDR_LENGTH), 3, "{tag} es={es} length preserved");
                assert_eq!(rd_usize(h2, HDR_CAPACITY), 103, "{tag} es={es} capacity");
                assert_eq!(rd_isize(h2, HDR_TEMP), -42, "{tag} es={es} temp preserved");
                (lib.arrfreef)(a2 as *mut c_void);
            }
        }
    }
}

/// `stbds_hash_index` is 104 bytes: `storage` must be exactly
/// `STBDS_ALIGN_FWD((size_t)(t+1), 64)`.  Sampling many tables covers several
/// distinct `t % 64` residues, which makes the check discriminating.
#[test]
fn cfg65b_hash_index_size_and_storage_alignment() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let mut residues_c = std::collections::HashSet::new();
    let mut residues_rs = std::collections::HashSet::new();
    unsafe {
        let mut keep: Vec<(*mut c_void, usize)> = Vec::new();
        for i in 0..400usize {
            let es = 8 + (i % 5) * 8;
            for (lib, tag, residues) in [
                (&p.c, "C", &mut residues_c),
                (&p.rs, "Rust", &mut residues_rs),
            ] {
                let t = (lib.shmode_func)(es, STBDS_SH_ARENA as i32);
                let h = (t as *mut u8).sub(es).sub(HDR_SIZE);
                let tbl = rd_ptr(h, HDR_HASH_TABLE);
                let storage = rd_ptr(tbl, HI_STORAGE);
                residues.insert(tbl as usize % CACHE_LINE);
                assert_eq!(
                    storage as usize,
                    align_fwd(tbl as usize + HI_SIZE, CACHE_LINE),
                    "{tag}: storage != ALIGN_FWD(t+sizeof(stbds_hash_index)=104, 64)"
                );
                assert_eq!(storage as usize % CACHE_LINE, 0, "{tag}: storage misaligned");
                keep.push((t, es));
            }
        }
        for (t, es) in keep {
            // free through whichever library owns it -- both are equivalent for
            // a table created identically; use the matching one by parity
            let _ = t;
            let _ = es;
        }
    }
    assert_eq_ctx(
        {
            let mut v: Vec<usize> = residues_c.iter().copied().collect();
            v.sort();
            v
        },
        {
            let mut v: Vec<usize> = residues_rs.iter().copied().collect();
            v.sort();
            v
        },
        "the set of observed `t % 64` residues",
    );
    assert!(
        residues_c.len() > 1,
        "only one alignment residue observed ({residues_c:?}); the check is weak"
    );
}

/// `stbds_hash_bucket` is 128 bytes (8 hashes then 8 indices).  Scanning the
/// buckets with a 128-byte stride must find exactly `used_count` live entries,
/// and each entry's index must be a valid element index.
#[test]
fn cfg65c_hash_bucket_layout() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut r = Rng::new(0x650065);
    unsafe {
        for n in [1usize, 6, 7, 12, 13, 24, 25, 50, 100] {
            let mut m = MapPair::empty(p, cfg);
            let mut owned: Vec<Vec<u8>> = Vec::new();
            for i in 0..n {
                owned.push((i as i32).to_le_bytes().to_vec());
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                m.put(k, &r.u32().to_le_bytes(), &format!("n={n} put {i}"));
            }
            for (mm, tag) in [(&m.c, "C"), (&m.rs, "Rust")] {
                let h = (mm.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE);
                let tbl = rd_ptr(h, HDR_HASH_TABLE);
                let sc = rd_usize(tbl, HI_SLOT_COUNT);
                let used = rd_usize(tbl, HI_USED_COUNT);
                let tomb = rd_usize(tbl, HI_TOMBSTONE_COUNT);
                let storage = rd_ptr(tbl, HI_STORAGE);
                let mut live = 0usize;
                let hmlen = mm.hmlen();
                for b in 0..(sc >> 3) {
                    let bp = storage.add(b * BUCKET_SIZE);
                    for j in 0..BUCKET_LENGTH {
                        let hash = rd_usize(bp, j * 8);
                        let idx = rd_isize(bp, 64 + j * 8);
                        if idx >= 0 {
                            live += 1;
                            assert!(hash >= 2, "{tag} n={n}: live slot with hash < 2");
                            assert!(idx < hmlen, "{tag} n={n}: index {idx} out of range");
                        } else {
                            assert!(
                                idx == -1 || idx == -2,
                                "{tag} n={n}: bogus index sentinel {idx}"
                            );
                        }
                    }
                }
                assert_eq!(
                    live, used,
                    "{tag} n={n}: 128-byte bucket stride found {live} live slots, \
                     used_count says {used}"
                );
                assert_eq!(tomb, 0, "{tag} n={n}: no deletes yet");
                // slot_count_log2 must match slot_count
                assert_eq!(
                    rd_usize(tbl, HI_SLOT_COUNT_LOG2),
                    sc.trailing_zeros() as usize,
                    "{tag} n={n}: slot_count_log2"
                );
                // the thresholds are exactly the C formulas
                assert_eq!(rd_usize(tbl, HI_USED_COUNT_THRESHOLD), sc - (sc >> 2));
                assert_eq!(
                    rd_usize(tbl, HI_TOMBSTONE_COUNT_THRESHOLD),
                    (sc >> 3) + (sc >> 4)
                );
                assert_eq!(
                    rd_usize(tbl, HI_USED_COUNT_SHRINK_THRESHOLD),
                    if sc <= BUCKET_LENGTH { 0 } else { sc >> 2 }
                );
            }
            m.free();
        }
    }
}

/// `stbds_string_block` is 16 bytes (`next` then `storage[8]`), so the oversize
/// path's `sb->storage` is exactly `sb + 8`.
#[test]
fn cfg65d_string_block_layout() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for (lib, tag) in [(&p.c, "C"), (&p.rs, "Rust")] {
            let mut arena = [0u64; 3];
            let ap = arena.as_mut_ptr() as *mut u8;
            let mut big = vec![b'Z'; 2000];
            big.push(0);
            let ret = (lib.stralloc)(ap as *mut c_void, big.as_mut_ptr() as *mut i8) as *mut u8;
            let storage = rd_ptr(ap, ARENA_STORAGE);
            assert_eq!(
                ret as usize,
                storage as usize + 8,
                "{tag}: sb->storage must be sb+8 (sizeof(stbds_string_block)==16)"
            );
            assert!(rd_ptr(storage, 0).is_null(), "{tag}: sb->next must be NULL");
            assert_eq!(rd_usize(ap, ARENA_REMAINING), 0, "{tag}: remaining");
            assert_eq!(rd_u8(ap, ARENA_BLOCK), 1, "{tag}: block@16");
            (lib.strreset)(ap as *mut c_void);
            assert_eq!(arena, [0u64; 3], "{tag}: strreset must zero all 24 bytes");
        }
    }
}

// ---------------------------------------------------------------------------
// row 66 — global seed lockstep over a full map lifecycle
// ---------------------------------------------------------------------------
/// Every `stbds_make_hash_index(sc, NULL)` advances the ONE global seed, while
/// `ot != NULL` (grow / shrink / rebuild) inherits it.  The two implementations
/// must therefore make the identical *sequence* of index creations.
#[test]
fn cfg66_seed_lockstep_over_lifecycle() {
    for start in [0usize, 1, INITIAL_HASH_SEED, usize::MAX, 0xa5a5_5a5a_1234_4321] {
        let (p, _g) = session(start);
        let cfg = MapCfg::int_int();
        let mut m = MapPair::empty(p, cfg);
        let mut r = Rng::new(0x660066 ^ start as u64);
        let mut owned: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for op in 0..600usize {
                let kv = r.below(150) as i32;
                owned.push(kv.to_le_bytes().to_vec());
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                if r.below(3) == 0 {
                    m.del(k, &format!("start={start:#x} op {op} del"));
                } else {
                    m.put(k, &r.u32().to_le_bytes(), &format!("start={start:#x} op {op} put"));
                }
                // table->seed is inherited across grow/shrink/rebuild, so it
                // must be equal AND equal to `start` for the first generation
                let sc = seed_of(&m.c, cfg.elemsize);
                let sr = seed_of(&m.rs, cfg.elemsize);
                assert_eq_ctx(sc, sr, &format!("start={start:#x} op {op}: table->seed"));
                // `None` while the map is still NULL (a `del` on a NULL map is a
                // no-op that returns NULL, so no table exists yet).
                if let Some(sc) = sc {
                    assert_eq!(sc, start, "table->seed must stay the captured value");
                }
            }
            // the global seed must have advanced exactly once (one fresh index)
            let ga = (p.c.shmode_func)(8, 0);
            let gb = (p.rs.shmode_func)(8, 0);
            let sa = rd_usize(rd_ptr((ga as *mut u8).sub(8).sub(HDR_SIZE), HDR_HASH_TABLE), HI_SEED);
            let sb = rd_usize(rd_ptr((gb as *mut u8).sub(8).sub(HDR_SIZE), HDR_HASH_TABLE), HI_SEED);
            assert_eq_ctx(sa, sb, &format!("start={start:#x}: global seed after lifecycle"));
            let a = 0x27bb_2ee6_87b0_b0fdusize;
            let b = 0xb504_f32dusize;
            assert_eq!(
                sa,
                start.wrapping_mul(a).wrapping_add(b),
                "the global seed must advance by seed*a+b exactly once"
            );
            (p.c.hmfree_func)((ga as *mut u8).sub(8) as *mut c_void, 8);
            (p.rs.hmfree_func)((gb as *mut u8).sub(8) as *mut c_void, 8);
            m.free();
        }
    }
}

unsafe fn seed_of(m: &Map, elemsize: usize) -> Option<usize> {
    if m.t.is_null() {
        return None;
    }
    let tbl = rd_ptr((m.t as *mut u8).sub(elemsize).sub(HDR_SIZE), HDR_HASH_TABLE);
    if tbl.is_null() {
        None
    } else {
        Some(rd_usize(tbl, HI_SEED))
    }
}

// ---------------------------------------------------------------------------
// row 67 — two live maps interleaved: the shared global seed must advance in
//          the same order on both sides
// ---------------------------------------------------------------------------
#[test]
fn cfg67_interleaved_maps_share_the_global_seed() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg_a = MapCfg::int_int();
    let cfg_b = MapCfg {
        elemsize: 16,
        keysize: 8,
        keyoffset: 0,
        mode: STBDS_HM_STRING,
        valoffset: 8,
        valsize: 8,
        force_raw_snap: false,
    };
    let mut ma = MapPair::empty(p, cfg_a);
    let mut mb = MapPair::with_shmode(p, cfg_b, STBDS_SH_STRDUP as i32);
    let mut r = Rng::new(0x670067);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        // mb was created first via shmode_func => it captured the first seed
        let sb0 = seed_of(&mb.c, cfg_b.elemsize);
        assert_eq_ctx(sb0, seed_of(&mb.rs, cfg_b.elemsize), "mb initial seed");
        assert_eq!(sb0, Some(INITIAL_HASH_SEED));

        for op in 0..500usize {
            // alternate between the two maps
            if op % 2 == 0 {
                let kv = r.below(120) as i32;
                owned.push(kv.to_le_bytes().to_vec());
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                if r.below(4) == 0 {
                    ma.del(k, &format!("op {op} A del"));
                } else {
                    ma.put(k, &r.u32().to_le_bytes(), &format!("op {op} A put"));
                }
            } else {
                let kv = r.below(120);
                let mut s = format!("k{kv:04}").into_bytes();
                s.push(0);
                owned.push(s);
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                if r.below(4) == 0 {
                    // reverse-order-safe: mode == 1, so the swap re-lookup works
                    mb.del(k, &format!("op {op} B del"));
                } else {
                    mb.put(k, &r.u64().to_le_bytes(), &format!("op {op} B put"));
                }
            }
            assert_eq_ctx(
                (seed_of(&ma.c, cfg_a.elemsize), seed_of(&mb.c, cfg_b.elemsize)),
                (
                    seed_of(&ma.rs, cfg_a.elemsize),
                    seed_of(&mb.rs, cfg_b.elemsize),
                ),
                &format!("op {op}: both table seeds"),
            );
        }
        // ma's first index was created after mb's, so it holds the 2nd seed
        let a = 0x27bb_2ee6_87b0_b0fdusize;
        let b = 0xb504_f32dusize;
        assert_eq!(
            seed_of(&ma.c, cfg_a.elemsize),
            Some(INITIAL_HASH_SEED.wrapping_mul(a).wrapping_add(b)),
            "the second fresh index must capture the advanced global seed"
        );
        ma.free();
        mb.free();
    }
}
