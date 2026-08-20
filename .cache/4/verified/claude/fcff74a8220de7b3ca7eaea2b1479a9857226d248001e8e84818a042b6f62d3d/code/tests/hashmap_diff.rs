//! Phase B — CONFIGS.md rows C10..C34 and C42..C45: the hash-map primitives
//! (`stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
//! `stbds_hmdel_key`, `stbds_hmput_default`, `stbds_shmode_func`,
//! `stbds_hmfree_func`) driven directly, at the lowest level, exactly as the
//! `stbds_hm*`/`stbds_sh*` macros drive them.
mod common;

use common::*;
use std::collections::HashSet;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn bin_shapes() -> Vec<Shape> {
    vec![
        Shape::bin(8, 4),
        Shape::bin(16, 4),
        Shape::bin(16, 8),
        Shape::bin(4, 4),
        Shape::bin(8, 8),
        Shape::bin(24, 16),
        Shape::bin(32, 32),
        Shape::bin(1, 1),
        Shape::bin(2, 1),
    ]
}

fn distinct_bin_keys(rng: &mut Rng, keysize: usize, n: usize) -> Vec<Vec<u8>> {
    if keysize == 0 {
        return vec![Vec::new()];
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut guard = 0;
    while out.len() < n && guard < n * 1000 + 1000 {
        guard += 1;
        let k = rng.bytes(keysize);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

fn distinct_str_keys(rng: &mut Rng, n: usize, minlen: usize, maxlen: usize) -> Vec<Vec<u8>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    while out.len() < n {
        let len = minlen + rng.below(maxlen - minlen + 1);
        let k = rng.cstring(len);
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    out
}

fn val(rng: &mut Rng, shape: Shape) -> Vec<u8> {
    rng.bytes(shape.value_len().max(1))
}

// ---------------------------------------------------------------------------
// C10 / C12 / C14 / C42 / C43 — binary mode
// ---------------------------------------------------------------------------

/// C10 — mode 0 (BINARY), implicit creation, all element/key shapes,
/// counts 1..64 (checked after *every* insert).
#[test]
fn cfg_binary_insert_matrix() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x10_0000);
    for shape in bin_shapes() {
        for seed in [0x3141_5926usize, 0, usize::MAX] {
            seed_both(seed);
            let mut m = MapPair::empty(shape);
            m.ctx = format!("binary insert {shape:?} seed={seed:#x}");
            let keys = distinct_bin_keys(&mut rng, shape.keysize, 64);
            unsafe {
                for (i, k) in keys.iter().enumerate() {
                    let v = val(&mut rng, shape);
                    let idx = m.put(k, HM_BINARY, &v, &format!("key#{i}"));
                    assert_eq!(idx, i as isize, "insertion index should be sequential");
                }
                m.free();
            }
        }
    }
    seed_both(0x3141_5926);
}

/// C12 — duplicate keys (re-put): `temp` is the existing index, no growth.
#[test]
fn cfg_binary_duplicate_keys() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x12_0000);
    for shape in [Shape::bin(16, 8), Shape::bin(8, 4), Shape::bin(24, 16)] {
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("binary dup {shape:?}");
        let keys = distinct_bin_keys(&mut rng, shape.keysize, 40);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = val(&mut rng, shape);
                m.put(k, HM_BINARY, &v, &format!("insert#{i}"));
            }
            // re-put every key, in a shuffled order, with fresh values
            for round in 0..3 {
                for i in 0..keys.len() {
                    let j = (i * 7 + round * 13) % keys.len();
                    let v = val(&mut rng, shape);
                    let idx = m.put(&keys[j], HM_BINARY, &v, &format!("reput#{j} r{round}"));
                    assert_eq!(idx, j as isize, "re-put must return the existing index");
                }
            }
            m.free();
        }
    }
}

/// C14 — every growth threshold: 6, 12, 24, 48, 96, 192 inserts.
#[test]
fn cfg_growth_thresholds() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x14_0000);
    let shape = Shape::bin(16, 8);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "growth thresholds".into();
    let keys = distinct_bin_keys(&mut rng, shape.keysize, 400);
    unsafe {
        let mut slot_counts = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            let v = val(&mut rng, shape);
            m.put(k, HM_BINARY, &v, &format!("key#{i}"));
            let t = m.c.table();
            slot_counts.push((*t).slot_count);
        }
        // sanity: the index really did grow through the expected sizes
        for want in [8usize, 16, 32, 64, 128, 256, 512] {
            assert!(
                slot_counts.contains(&want),
                "expected an index of {want} slots somewhere in {:?}",
                &slot_counts[..20.min(slot_counts.len())]
            );
        }
        m.free();
    }
}

/// C42 — all keys equal: one slot, `used_count` stays 1.
#[test]
fn cfg_all_equal_keys() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x42_0000);
    for shape in [Shape::bin(16, 8), Shape::bin(8, 4), Shape::bin(32, 1)] {
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("all-equal keys {shape:?}");
        let k = rng.bytes(shape.keysize);
        unsafe {
            for i in 0..50 {
                let v = val(&mut rng, shape);
                let idx = m.put(&k, HM_BINARY, &v, &format!("put#{i}"));
                assert_eq!(idx, 0);
            }
            assert_eq!((*m.c.table()).used_count, 1);
            m.free();
        }
    }
}

/// C43 — hash-value extremes (`hash < 2` fixup path) and boundary key values.
#[test]
fn cfg_extreme_key_values() {
    let _g = seed_lock();
    let shape = Shape::bin(16, 8);
    for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
        seed_both(seed);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("extreme keys seed={seed:#x}");
        let keys: Vec<Vec<u8>> = vec![
            vec![0x00; 8],
            vec![0xff; 8],
            vec![0, 0, 0, 0, 0, 0, 0, 0x80],
            vec![0x80, 0, 0, 0, 0, 0, 0, 0],
            vec![0x01, 0, 0, 0, 0, 0, 0, 0],
            vec![0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff],
            vec![0x7f; 8],
            vec![0x80; 8],
        ];
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                m.put(k, HM_BINARY, &[i as u8; 8], &format!("key#{i}"));
            }
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(m.get(k, HM_BINARY, &format!("get#{i}")), i as isize);
            }
            m.free();
        }
    }
    seed_both(0x3141_5926);
}

// ---------------------------------------------------------------------------
// C11 / C13 — string mode with implicit creation (string.mode = SH_DEFAULT)
// ---------------------------------------------------------------------------

/// C11 — mode 1 (STRING) with implicit creation.
#[test]
fn cfg_string_default_insert() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x11_0000);
    for elemsize in [16usize, 24, 32, 8] {
        let shape = Shape::str_ptr(elemsize);
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("string SH_DEFAULT elemsize={elemsize}");
        let keys = distinct_str_keys(&mut rng, 64, 0, 40);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = val(&mut rng, shape);
                let idx = m.put_check_temp_key(k, HM_STRING, &v, &format!("key#{i}"));
                assert_eq!(idx, i as isize);
                assert_eq!(
                    (*m.c.table()).string.mode as c_int,
                    SH_DEFAULT,
                    "implicit creation with mode>=1 must select SH_DEFAULT"
                );
            }
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(m.get(k, HM_STRING, &format!("get#{i}")), i as isize);
            }
            m.free();
        }
    }
}

/// C13 — duplicate string keys: `temp` *and* `table->temp_key` are refreshed
/// from the *stored* key pointer (lib.c L732-733).
#[test]
fn cfg_string_duplicate_keys_temp_key() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x13_0000);
    let shape = Shape::str_ptr(16);
    seed_both(0x3141_5926);
    let mut m = MapPair::empty(shape);
    m.ctx = "string dup temp_key".into();
    let keys = distinct_str_keys(&mut rng, 30, 1, 20);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = val(&mut rng, shape);
            m.put_check_temp_key(k, HM_STRING, &v, &format!("insert#{i}"));
        }
        for round in 0..3 {
            for i in 0..keys.len() {
                let j = (i * 11 + round) % keys.len();
                let v = val(&mut rng, shape);
                let idx =
                    m.put_check_temp_key(&keys[j], HM_STRING, &v, &format!("reput#{j} r{round}"));
                assert_eq!(idx, j as isize);
            }
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// C15 — keysize matrix
// ---------------------------------------------------------------------------

/// C15 — `keysize` 0..17 with `elemsize` 32: every siphash tail width and every
/// `memcmp` width.
#[test]
fn cfg_keysize_matrix() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x15_0000);
    for keysize in 0..=17usize {
        let shape = Shape::bin(32, keysize);
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = format!("keysize={keysize}");
        let keys = distinct_bin_keys(&mut rng, keysize, 30);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = val(&mut rng, shape);
                m.put(k, HM_BINARY, &v, &format!("key#{i}"));
            }
            for (i, k) in keys.iter().enumerate() {
                m.get(k, HM_BINARY, &format!("get#{i}"));
                let _ = i;
            }
            // keysize == 0 makes every key compare equal (memcmp(...,0)==0)
            if keysize == 0 {
                assert_eq!(m.c.hmlen(), 1);
                assert_eq!(m.r.hmlen(), 1);
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// C16 / C17 / C18 — the get entry points
// ---------------------------------------------------------------------------

/// C16 — `stbds_hmget_key`: hits and misses interleaved over many map sizes.
#[test]
fn cfg_get_hit_miss_matrix() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x16_0000);
    for shape in [Shape::bin(16, 8), Shape::bin(8, 4), Shape::bin(24, 16)] {
        for n in [0usize, 1, 2, 5, 6, 7, 8, 17, 64] {
            seed_both(0x3141_5926);
            let mut m = MapPair::empty(shape);
            m.ctx = format!("get hit/miss {shape:?} n={n}");
            let keys = distinct_bin_keys(&mut rng, shape.keysize, n);
            let absent = distinct_bin_keys(&mut rng, shape.keysize, 20);
            unsafe {
                for (i, k) in keys.iter().enumerate() {
                    let v = val(&mut rng, shape);
                    m.put(k, HM_BINARY, &v, &format!("key#{i}"));
                }
                for (i, k) in keys.iter().enumerate() {
                    let got = m.get(k, HM_BINARY, &format!("hit#{i}"));
                    assert_eq!(got, i as isize, "hit must return the insertion index");
                }
                for (i, k) in absent.iter().enumerate() {
                    if keys.contains(k) {
                        continue;
                    }
                    let got = m.get(k, HM_BINARY, &format!("miss#{i}"));
                    assert_eq!(got, INDEX_EMPTY, "miss must return -1");
                }
                m.free();
            }
        }
    }
}

/// C17 — `stbds_hmget_key_ts` (caller-supplied `temp`).
#[test]
fn cfg_get_ts_matrix() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x17_0000);
    for shape in [Shape::bin(16, 8), Shape::str_ptr(16)] {
        let mode = if shape.key == KeyRepr::CStr {
            HM_STRING
        } else {
            HM_BINARY
        };
        for n in [0usize, 1, 2, 6, 7, 64] {
            seed_both(0x3141_5926);
            let mut m = MapPair::empty(shape);
            m.ctx = format!("get_ts {shape:?} n={n}");
            let keys = if mode == HM_STRING {
                distinct_str_keys(&mut rng, n, 0, 30)
            } else {
                distinct_bin_keys(&mut rng, shape.keysize, n)
            };
            let absent = if mode == HM_STRING {
                distinct_str_keys(&mut rng, 10, 40, 60)
            } else {
                distinct_bin_keys(&mut rng, shape.keysize, 10)
            };
            unsafe {
                for (i, k) in keys.iter().enumerate() {
                    let v = val(&mut rng, shape);
                    m.put(k, mode, &v, &format!("key#{i}"));
                }
                for (i, k) in keys.iter().enumerate() {
                    assert_eq!(
                        m.get_ts(k, mode, &format!("ts hit#{i}")),
                        i as isize,
                        "hmget_key_ts hit"
                    );
                }
                for (i, k) in absent.iter().enumerate() {
                    if keys.contains(k) {
                        continue;
                    }
                    assert_eq!(
                        m.get_ts(k, mode, &format!("ts miss#{i}")),
                        INDEX_EMPTY,
                        "hmget_key_ts miss"
                    );
                }
                m.free();
            }
        }
    }
}

/// C18 — the first call being a *get* on a NULL map (creation path D).
#[test]
fn cfg_get_creates_map() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x18_0000);
    for shape in [Shape::bin(16, 8), Shape::bin(8, 4), Shape::str_ptr(16)] {
        let mode = if shape.key == KeyRepr::CStr {
            HM_STRING
        } else {
            HM_BINARY
        };
        for use_ts in [false, true] {
            seed_both(0x3141_5926);
            let mut m = MapPair::empty(shape);
            m.ctx = format!("get-creates-map {shape:?} ts={use_ts}");
            let k = if mode == HM_STRING {
                rng.cstring(6)
            } else {
                rng.bytes(shape.keysize)
            };
            unsafe {
                let got = if use_ts {
                    m.get_ts(&k, mode, "first get on NULL map")
                } else {
                    m.get(&k, mode, "first get on NULL map")
                };
                assert_eq!(got, INDEX_EMPTY);
                assert!(!m.c.p.is_null() && !m.r.p.is_null());
                assert_eq!(m.c.len(), 1);
                assert!(m.c.table().is_null(), "no hash index is created by a get");
                // a second get, still without an index
                let got2 = if use_ts {
                    m.get_ts(&k, mode, "second get")
                } else {
                    m.get(&k, mode, "second get")
                };
                assert_eq!(got2, INDEX_EMPTY);
                // now a real put on top of the get-created array
                let v = val(&mut rng, shape);
                let idx = m.put(&k, mode, &v, "put after get-created array");
                assert_eq!(idx, 0);
                assert_eq!(m.get(&k, mode, "get after put"), 0);
                m.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C19 — hmput_default
// ---------------------------------------------------------------------------

/// C19 — `stbds_hmput_default` on a NULL map, on an `hmput_key` map and on a
/// `shmode_func` map, plus the `length == 0` re-entry.
#[test]
fn cfg_hmput_default_paths() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x19_0000);
    let shape = Shape::bin(16, 8);

    // (a) on a NULL map, then puts
    {
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = "hmput_default on NULL".into();
        unsafe {
            m.hmput_default(&[0xAA; 8], "first");
            assert_eq!(m.c.len(), 1);
            assert!(m.c.table().is_null());
            m.hmput_default(&[0xBB; 8], "second (no-op path)");
            assert_eq!(m.c.len(), 1);
            let keys = distinct_bin_keys(&mut rng, 8, 10);
            for (i, k) in keys.iter().enumerate() {
                let v = val(&mut rng, shape);
                m.put(k, HM_BINARY, &v, &format!("key#{i}"));
            }
            m.hmput_default(&[0xCC; 8], "after puts");
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(m.get(k, HM_BINARY, &format!("get#{i}")), i as isize);
            }
            m.free();
        }
    }

    // (b) after hmput_key has created the array
    {
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = "hmput_default after hmput_key".into();
        unsafe {
            let keys = distinct_bin_keys(&mut rng, 8, 7);
            for (i, k) in keys.iter().enumerate() {
                let v = val(&mut rng, shape);
                m.put(k, HM_BINARY, &v, &format!("key#{i}"));
            }
            m.hmput_default(&[0x11; 8], "default");
            m.free();
        }
    }

    // (c) on a shmode_func map
    for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        seed_both(0x3141_5926);
        let s = Shape::str_ptr(16);
        let mut m = MapPair::shmode(s, mode);
        m.ctx = format!("hmput_default on shmode({mode})");
        unsafe {
            m.hmput_default(&[0x22; 8], "default");
            assert_eq!(m.c.len(), 1);
            m.free();
        }
    }

    // (d) `length == 0` re-entry: hand hmput_default an array whose header
    //     length has been forced to 0 (the second branch of the `if`).
    {
        seed_both(0x3141_5926);
        let mut m = MapPair::empty(shape);
        m.ctx = "hmput_default length==0".into();
        unsafe {
            m.hmput_default(&[0x33; 8], "create");
            (*header_of(m.c.p, shape.elemsize)).length = 0;
            (*header_of(m.r.p, shape.elemsize)).length = 0;
            m.hmput_default(&[0x44; 8], "re-enter with length 0");
            assert_eq!(m.c.len(), 1);
            assert_eq!(m.r.len(), 1);
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// C20 / C21 / C22 / C23 — shmode_func string modes
// ---------------------------------------------------------------------------

/// C20 — `stbds_shmode_func` for every declared mode, followed by string puts.
#[test]
fn cfg_shmode_all_modes() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x20_0000);
    for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for elemsize in [16usize, 24, 32] {
            // SH_NONE hits the `default:` memcpy branch, so the element holds
            // 8 raw *bytes of the string*, not a pointer.
            let shape = if mode == SH_NONE {
                Shape::bin(elemsize, 8)
            } else {
                Shape::str_ptr(elemsize)
            };
            seed_both(0x3141_5926);
            let mut m = MapPair::shmode(shape, mode);
            m.ctx = format!("shmode({mode}) elemsize={elemsize}");
            m.check("right after shmode_func");
            unsafe {
                assert_eq!((*m.c.table()).string.mode as c_int, mode);
                assert_eq!((*m.r.table()).string.mode as c_int, mode);
                // keys >= 8 chars so the SH_NONE memcpy only reads defined bytes
                let keys = distinct_str_keys(&mut rng, 40, 9, 30);
                for (i, k) in keys.iter().enumerate() {
                    let v = val(&mut rng, shape);
                    m.put(k, HM_STRING, &v, &format!("key#{i}"));
                }
                if mode != SH_NONE {
                    for (i, k) in keys.iter().enumerate() {
                        assert_eq!(m.get(k, HM_STRING, &format!("get#{i}")), i as isize);
                    }
                }
                m.free();
            }
        }
    }
}

/// C21 — SH_STRDUP full life cycle (keys are `strdup`ed and freed on delete).
#[test]
fn cfg_strdup_mode_lifecycle() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x21_0000);
    let shape = Shape::str_ptr(16);
    seed_both(0x3141_5926);
    let mut m = MapPair::shmode(shape, SH_STRDUP);
    m.ctx = "SH_STRDUP lifecycle".into();
    let keys = distinct_str_keys(&mut rng, 64, 0, 40);
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = val(&mut rng, shape);
            m.put_check_temp_key(k, HM_STRING, &v, &format!("key#{i}"));
        }
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(m.get(k, HM_STRING, &format!("get#{i}")), i as isize);
        }
        // delete half of them (frees the strdup'ed keys)
        for i in (0..keys.len()).step_by(2) {
            assert_eq!(m.del(&keys[i], HM_STRING, &format!("del#{i}")), 1);
        }
        // re-insert them (fresh strdups, tombstone reuse)
        for i in (0..keys.len()).step_by(2) {
            let v = val(&mut rng, shape);
            m.put_check_temp_key(&keys[i], HM_STRING, &v, &format!("reinsert#{i}"));
        }
        m.free();
    }
}

/// C22 — SH_ARENA full life cycle, including keys larger than a block.
#[test]
fn cfg_arena_mode_lifecycle() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x22_0000);
    let shape = Shape::str_ptr(24);
    seed_both(0x3141_5926);
    let mut m = MapPair::shmode(shape, SH_ARENA);
    m.ctx = "SH_ARENA lifecycle".into();
    let mut keys = distinct_str_keys(&mut rng, 50, 0, 40);
    keys.extend(distinct_str_keys(&mut rng, 6, 500, 1200)); // > blocksize
    keys.extend(distinct_str_keys(&mut rng, 20, 100, 300));
    unsafe {
        for (i, k) in keys.iter().enumerate() {
            let v = val(&mut rng, shape);
            m.put_check_temp_key(k, HM_STRING, &v, &format!("key#{i}"));
        }
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(m.get(k, HM_STRING, &format!("get#{i}")), i as isize);
        }
        for i in (0..keys.len()).step_by(3) {
            assert_eq!(m.del(&keys[i], HM_STRING, &format!("del#{i}")), 1);
        }
        for i in (0..keys.len()).step_by(3) {
            let v = val(&mut rng, shape);
            m.put_check_temp_key(&keys[i], HM_STRING, &v, &format!("reinsert#{i}"));
        }
        m.free();
    }
}

/// C23 — SH_NONE + `mode = STBDS_HM_STRING`: the switch `default:` branch
/// `memcpy`s `keysize` *bytes of the string* into the element.
#[test]
fn cfg_sh_none_copies_bytes() {
    let _g = seed_lock();
    let mut rng = Rng::new(0x23_0000);
    for elemsize in [16usize, 32] {
        // element key field = 8 raw bytes (sizeof(char*) passed as keysize)
        let shape = Shape::bin(elemsize, 8);
        seed_both(0x3141_5926);
        let mut m = MapPair::shmode(shape, SH_NONE);
        m.ctx = format!("SH_NONE byte-copy elemsize={elemsize}");
        // distinct keys only: a duplicate key would make `stbds_is_key_equal`
        // `strcmp` the copied bytes as a `char *` (see ERRORS.md E42).
        let keys = distinct_str_keys(&mut rng, 30, 12, 30);
        unsafe {
            for (i, k) in keys.iter().enumerate() {
                let v = val(&mut rng, shape);
                let idx = m.put(k, HM_STRING, &v, &format!("key#{i}"));
                assert_eq!(idx, i as isize);
                // the element really holds the first 8 bytes of the string
                let elem = (m.c.p as *const u8).add(shape.elemsize * (idx as usize + 1) - shape.elemsize);
                let got = std::slice::from_raw_parts(elem, 8);
                assert_eq!(got, &k[..8], "SH_NONE must memcpy the string bytes");
            }
            m.free();
        }
    }
}
