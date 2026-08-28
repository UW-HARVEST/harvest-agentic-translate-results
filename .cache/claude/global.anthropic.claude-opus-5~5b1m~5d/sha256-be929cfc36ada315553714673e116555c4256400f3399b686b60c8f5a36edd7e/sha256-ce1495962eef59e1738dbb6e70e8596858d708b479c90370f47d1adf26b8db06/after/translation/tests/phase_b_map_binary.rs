//! Phase B rows B18-B34, B45, B62: binary-mode (`mode < 1`) hash maps driven
//! through the low-level `stbds_hm*_key` entry points.

mod common;
use common::*;
use std::os::raw::c_int;

/// build an element whose first `keysize` bytes are the key
fn mk_elem(key: &[u8], elemsize: usize, fill: &[u8]) -> Vec<u8> {
    let mut e = vec![0u8; elemsize];
    e[..key.len()].copy_from_slice(key);
    let n = (elemsize - key.len()).min(fill.len());
    e[key.len()..key.len() + n].copy_from_slice(&fill[..n]);
    e
}

fn k32(v: i32) -> [u8; 4] {
    v.to_ne_bytes()
}

/// B18 — single insert into a NULL map (table creation)
#[test]
fn cfg_b18_single_insert() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        let key = k32(1234);
        let e = mk_elem(&key, 8, &[9, 9, 9, 9]);
        let i = p.put_struct(&key, &e, HM_BINARY, "B18 put");
        assert_eq!(i, 0, "B18 first index");
        assert_eq!(p.c.hmlen(), 1);
        assert_eq!(p.c.snapshot().table.as_ref().unwrap().slot_count, 8);
        p.geti(&key, HM_BINARY, "B18 get");
        p.free("B18 free");
    });
}

/// B19 — 5 inserts (below `used_count_threshold` = 6, no grow)
#[test]
fn cfg_b19_five_inserts_no_grow() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        for k in 0i32..5 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k * 2));
            p.put_struct(&key, &e, HM_BINARY, &format!("B19 put {k}"));
        }
        let s = p.c.snapshot();
        assert_eq!(s.table.as_ref().unwrap().slot_count, 8, "B19 no grow yet");
        assert_eq!(s.table.as_ref().unwrap().used_count, 5);
        p.free("B19 free");
    });
}

/// B20 — grow boundaries 8→16 (6 elems), 16→32 (12), 32→64 (24)
#[test]
fn cfg_b20_grow_boundaries() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        let mut expected_slots = 8usize;
        for k in 0i32..40 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(!k));
            p.put_struct(&key, &e, HM_BINARY, &format!("B20 put {k}"));
            let s = p.c.snapshot();
            let t = s.table.as_ref().unwrap();
            // grow happens *before* the insert when used_count >= threshold
            let used_before = k as usize;
            if used_before >= expected_slots - (expected_slots >> 2) {
                expected_slots *= 2;
            }
            assert_eq!(
                t.slot_count, expected_slots,
                "B20 slot_count after inserting {k}"
            );
            assert_eq!(t.used_count, k as usize + 1);
        }
        p.free("B20 free");
    });
}

/// B21 — 100 and 500 random distinct keys, deep compare after every op
#[test]
fn cfg_b21_many_random_keys() {
    for &n in &[100usize, 500] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(21 + n as u64);
            let mut p = Pair::new(c, r, Shape::binary(8, 4));
            let mut keys: Vec<i32> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            while keys.len() < n {
                let k = rng.i32();
                if seen.insert(k) {
                    keys.push(k);
                }
            }
            for (i, &k) in keys.iter().enumerate() {
                let key = k32(k);
                let e = mk_elem(&key, 8, &k32(i as i32));
                p.put_struct(&key, &e, HM_BINARY, &format!("B21 n={n} put {i}"));
            }
            for (i, &k) in keys.iter().enumerate() {
                let idx = p.geti(&k32(k), HM_BINARY, &format!("B21 n={n} get {i}"));
                assert!(idx >= 0, "B21 key {k} should be present");
            }
            p.free(&format!("B21 n={n} free"));
        });
    }
}

/// B22 — re-put of existing keys interleaved with new keys
#[test]
fn cfg_b22_reput_existing() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut rng = Rng::new(22);
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        let keys: Vec<i32> = (0..40).map(|i| i * 7 - 100).collect();
        for round in 0..4 {
            for (i, &k) in keys.iter().enumerate() {
                let key = k32(k);
                let e = mk_elem(&key, 8, &k32(rng.i32()));
                p.put_struct(&key, &e, HM_BINARY, &format!("B22 r={round} put {i}"));
            }
        }
        assert_eq!(p.c.hmlen(), 40, "B22 no duplicates");
        p.free("B22 free");
    });
}

/// B23 — get present / absent for map sizes 0,1,5,6,50
#[test]
fn cfg_b23_get_present_absent() {
    for &n in &[0usize, 1, 5, 6, 50] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut p = Pair::new(c, r, Shape::binary(8, 4));
            for k in 0..n as i32 {
                let key = k32(k);
                let e = mk_elem(&key, 8, &k32(k));
                p.put_struct(&key, &e, HM_BINARY, &format!("B23 n={n} put {k}"));
            }
            for k in -5i32..(n as i32 + 5) {
                let got = p.geti(&k32(k), HM_BINARY, &format!("B23 n={n} get {k}"));
                if k >= 0 && k < n as i32 {
                    assert!(got >= 0, "B23 n={n} key {k} present");
                } else {
                    assert_eq!(got, -1, "B23 n={n} key {k} absent");
                }
            }
            p.free(&format!("B23 n={n} free"));
        });
    }
}

/// B24 — `hmget_key_ts` writes `*temp` and (unlike `hmget_key`) does not touch
/// `header->temp` on the existing-map paths.
#[test]
fn cfg_b24_get_ts() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(16, 8));
        // fresh map via the _ts path (a == NULL branch)
        let key = [1u8, 2, 3, 4, 5, 6, 7, 8];
        p.geti_ts(&key, HM_BINARY, "B24 fresh");
        p.assert_same("B24 after fresh ts");
        for k in 0i64..20 {
            let key = k.to_ne_bytes();
            let e = mk_elem(&key, 16, &k.to_ne_bytes());
            p.put_struct(&key, &e, HM_BINARY, &format!("B24 put {k}"));
        }
        for k in -5i64..25 {
            let key = k.to_ne_bytes();
            let ts = p.geti_ts(&key, HM_BINARY, &format!("B24 ts {k}"));
            let g = p.geti(&key, HM_BINARY, &format!("B24 get {k}"));
            assert_eq!(ts, g, "B24 ts vs get for {k}");
        }
        p.free("B24 free");
    });
}

/// B25 — delete: 1-of-1, 1-of-many (memmove + re-find), last element
#[test]
fn cfg_b25_delete_shapes() {
    // 1 of 1
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        let key = k32(77);
        let e = mk_elem(&key, 8, &k32(1));
        p.put_struct(&key, &e, HM_BINARY, "B25a put");
        let d = p.del(&key, HM_BINARY, "B25a del");
        assert_eq!(d, 1, "B25a del reports 1");
        assert_eq!(p.c.hmlen(), 0);
        p.geti(&key, HM_BINARY, "B25a get after del");
        p.free("B25a free");
    });
    // 1 of many, plus deleting the last element (old_index == final_index)
    for &victim in &[0usize, 1, 4, 9] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut p = Pair::new(c, r, Shape::binary(8, 4));
            for k in 0i32..10 {
                let key = k32(k);
                let e = mk_elem(&key, 8, &k32(k * 11));
                p.put_struct(&key, &e, HM_BINARY, &format!("B25b put {k}"));
            }
            let key = k32(victim as i32);
            let d = p.del(&key, HM_BINARY, &format!("B25b del {victim}"));
            assert_eq!(d, 1);
            for k in 0i32..10 {
                let got = p.geti(&k32(k), HM_BINARY, &format!("B25b get {k}"));
                if k as usize == victim {
                    assert_eq!(got, -1);
                } else {
                    assert!(got >= 0);
                }
            }
            p.free("B25b free");
        });
    }
}

/// B26 — deletes until `tombstone_count > tombstone_count_threshold` → rebuild
#[test]
fn cfg_b26_tombstone_rebuild() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        // 5 elements, slot_count 8, tct = 8>>3 + 8>>4 = 1
        for k in 0i32..5 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("B26 put {k}"));
        }
        let mut saw_rebuild = false;
        for k in 0i32..5 {
            let before = p.c.snapshot().table.as_ref().unwrap().tombstone_count;
            p.del(&k32(k), HM_BINARY, &format!("B26 del {k}"));
            let after = p.c.snapshot().table.as_ref().unwrap().tombstone_count;
            if before >= 1 && after == 0 {
                saw_rebuild = true;
            }
        }
        assert!(saw_rebuild, "B26 expected a tombstone rebuild to happen");
        p.free("B26 free");
    });
}

/// B27 — grow then delete until `used_count < used_count_shrink_threshold`
#[test]
fn cfg_b27_shrink() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        for k in 0i32..60 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("B27 put {k}"));
        }
        let big = p.c.snapshot().table.as_ref().unwrap().slot_count;
        assert!(big >= 64, "B27 expected grow, got {big}");
        let mut min_slots = big;
        for k in 0i32..60 {
            p.del(&k32(k), HM_BINARY, &format!("B27 del {k}"));
            let sc = p.c.snapshot().table.as_ref().unwrap().slot_count;
            min_slots = min_slots.min(sc);
        }
        assert!(min_slots < big, "B27 expected a shrink: {min_slots} vs {big}");
        assert_eq!(p.c.hmlen(), 0);
        p.free("B27 free");
    });
}

/// B28 — delete then insert → tombstone reuse
#[test]
fn cfg_b28_tombstone_reuse() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut rng = Rng::new(28);
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        for k in 0i32..30 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("B28 put {k}"));
        }
        for round in 0..40 {
            let victim = rng.below(30) as i32;
            p.del(&k32(victim), HM_BINARY, &format!("B28 r={round} del"));
            let nk = 1000 + round as i32;
            let key = k32(nk);
            let e = mk_elem(&key, 8, &k32(nk));
            p.put_struct(&key, &e, HM_BINARY, &format!("B28 r={round} put"));
        }
        p.free("B28 free");
    });
}

/// B29 — 2000 randomized ops, deep compare after every op
#[test]
fn cfg_b29_random_ops() {
    for seed in 1u64..=5 {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut rng = Rng::new(seed);
            let mut p = Pair::new(c, r, Shape::binary(8, 4));
            let keyspace = 60i32;
            for i in 0..2000 {
                let k = k32(rng.below(keyspace as usize) as i32);
                match rng.below(10) {
                    0..=4 => {
                        let e = mk_elem(&k, 8, &k32(rng.i32()));
                        p.put_struct(&k, &e, HM_BINARY, &format!("B29 s={seed} i={i} put"));
                    }
                    5..=6 => {
                        p.geti(&k, HM_BINARY, &format!("B29 s={seed} i={i} get"));
                    }
                    7 => {
                        p.geti_ts(&k, HM_BINARY, &format!("B29 s={seed} i={i} get_ts"));
                    }
                    8 => {
                        p.del(&k, HM_BINARY, &format!("B29 s={seed} i={i} del"));
                    }
                    _ => {
                        p.put_default(&format!("B29 s={seed} i={i} default"));
                    }
                }
            }
            p.free(&format!("B29 s={seed} free"));
        });
    }
}

/// B30 — keysize 1/2/4/8/16 × varying elemsize
#[test]
fn cfg_b30_keysize_elemsize_matrix() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &ks in &[1usize, 2, 4, 8, 16] {
            for &es in &[ks, ks + 4, ks + 8, 32, 64] {
                if es < ks {
                    continue;
                }
                let mut rng = Rng::new(30 + (ks * 100 + es) as u64);
                let mut p = Pair::new(c, r, Shape::binary(es, ks));
                let mut keys: Vec<Vec<u8>> = Vec::new();
                let mut seen = std::collections::HashSet::new();
                while keys.len() < 40 {
                    let k = rng.bytes(ks);
                    if seen.insert(k.clone()) {
                        keys.push(k);
                    }
                }
                for (i, k) in keys.iter().enumerate() {
                    let filler = rng.bytes(es);
                    let e = mk_elem(k, es, &filler);
                    p.put_struct(k, &e, HM_BINARY, &format!("B30 ks={ks} es={es} put {i}"));
                }
                for (i, k) in keys.iter().enumerate() {
                    let got = p.geti(k, HM_BINARY, &format!("B30 ks={ks} es={es} get {i}"));
                    assert!(got >= 0, "B30 ks={ks} es={es} key {i} present");
                }
                for (i, k) in keys.iter().enumerate().take(20) {
                    p.del(k, HM_BINARY, &format!("B30 ks={ks} es={es} del {i}"));
                }
                p.free(&format!("B30 ks={ks} es={es} free"));
            }
        }
    });
}

/// B31 — `keysize == 0`: `memcmp(_,_,0) == 0` makes every hash-matching slot
/// compare equal, so the map can never hold more than one entry.
#[test]
fn cfg_b31_keysize_zero() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        let mut rng = Rng::new(31);
        let mut p = Pair::new(c, r, Shape::binary(8, 0));
        for i in 0..20 {
            let e = rng.bytes(8);
            let idx = p.put_struct(&[], &e, HM_BINARY, &format!("B31 put {i}"));
            assert_eq!(idx, 0, "B31 always slot 0");
        }
        assert_eq!(p.c.hmlen(), 1, "B31 exactly one entry");
        p.geti(&[], HM_BINARY, "B31 get");
        p.del(&[], HM_BINARY, "B31 del");
        assert_eq!(p.c.hmlen(), 0);
        p.free("B31 free");
    });
}

/// B32 — `hmdel_key` with a non-zero `keyoffset` (put/get always hardcode 0)
#[test]
fn cfg_b32_keyoffset() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &(es, ko) in &[(16usize, 0usize), (16, 4), (16, 8), (24, 8), (24, 16)] {
            let mut rng = Rng::new(32 + (es * 100 + ko) as u64);
            let mut p = Pair::new(c, r, Shape::binary(es, 4).with_keyoffset(ko));
            for k in 0i32..20 {
                let key = k32(k);
                let filler = rng.bytes(es);
                let e = mk_elem(&key, es, &filler);
                p.put_struct(&key, &e, HM_BINARY, &format!("B32 es={es} ko={ko} put {k}"));
            }
            for k in 0i32..25 {
                p.del(&k32(k), HM_BINARY, &format!("B32 es={es} ko={ko} del {k}"));
                p.geti(&k32(k), HM_BINARY, &format!("B32 es={es} ko={ko} get {k}"));
            }
            p.free(&format!("B32 es={es} ko={ko} free"));
        }
    });
}

/// B33 — `hmput_default` in every position
#[test]
fn cfg_b33_hmput_default() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        // (a) on a NULL map
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        p.put_default("B33a first");
        assert_eq!(p.c.hmlen(), 0, "B33a hmlen still 0");
        p.put_default("B33a second (no-op)");
        p.defaults(&[7u8, 7, 7, 7, 8, 8, 8, 8], "B33a defaults write");
        // (b) then real inserts
        for k in 0i32..10 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("B33b put {k}"));
        }
        p.put_default("B33b default after inserts (no-op)");
        p.defaults(&[1u8, 2, 3, 4, 5, 6, 7, 8], "B33b defaults after inserts");
        p.geti(&k32(999), HM_BINARY, "B33b get missing");
        p.free("B33 free");

        // (c) default *after* inserts on a fresh map, then delete everything
        let mut p = Pair::new(c, r, Shape::binary(8, 4));
        for k in 0i32..3 {
            let key = k32(k);
            let e = mk_elem(&key, 8, &k32(k));
            p.put_struct(&key, &e, HM_BINARY, &format!("B33c put {k}"));
        }
        for k in 0i32..3 {
            p.del(&k32(k), HM_BINARY, &format!("B33c del {k}"));
        }
        p.put_default("B33c default on emptied map");
        p.free("B33c free");
    });
}

/// B34 — `hmfree_func` on binary maps of several sizes
#[test]
fn cfg_b34_hmfree_binary() {
    for &n in &[0usize, 1, 6, 50] {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut p = Pair::new(c, r, Shape::binary(8, 4));
            for k in 0..n as i32 {
                let key = k32(k);
                let e = mk_elem(&key, 8, &k32(k));
                p.put_struct(&key, &e, HM_BINARY, &format!("B34 n={n} put {k}"));
            }
            p.assert_same(&format!("B34 n={n} before free"));
            p.free(&format!("B34 n={n} free"));
        });
    }
}

/// B45 — `mode < 1` values must be byte-identical to `mode == 0`
#[test]
fn cfg_b45_negative_modes() {
    let modes: &[c_int] = &[0, -1, -2, -1000, c_int::MIN];
    let mut baseline: Option<MapSnap> = None;
    for &mode in modes {
        with_libs(DEFAULT_SEED, |c, r| unsafe {
            let mut p = Pair::new(c, r, Shape::binary(8, 4));
            for k in 0i32..30 {
                let key = k32(k);
                let e = mk_elem(&key, 8, &k32(k * 3));
                p.put_struct(&key, &e, mode, &format!("B45 mode={mode} put {k}"));
            }
            for k in 0i32..35 {
                p.geti(&k32(k), mode, &format!("B45 mode={mode} get {k}"));
            }
            for k in (0i32..30).step_by(3) {
                p.del(&k32(k), mode, &format!("B45 mode={mode} del {k}"));
            }
            let s = p.c.snapshot();
            match &baseline {
                None => baseline = Some(s),
                Some(b) => assert_eq!(
                    *b, s,
                    "B45 mode={mode} must be identical to mode=0"
                ),
            }
            p.free(&format!("B45 mode={mode} free"));
        });
    }
}

/// B62 — key at offset 0 inside padded elements with random filler
#[test]
fn cfg_b62_element_padding() {
    with_libs(DEFAULT_SEED, |c, r| unsafe {
        for &es in &[16usize, 24, 64] {
            let mut rng = Rng::new(62 + es as u64);
            let mut p = Pair::new(c, r, Shape::binary(es, 4));
            for k in 0i32..50 {
                let key = k32(k);
                let filler = rng.bytes(es);
                let e = mk_elem(&key, es, &filler);
                p.put_struct(&key, &e, HM_BINARY, &format!("B62 es={es} put {k}"));
            }
            // Re-put with different filler but the same key: only `keysize`
            // bytes participate in the compare, so no new elements appear.
            for k in 0i32..50 {
                let key = k32(k);
                let filler = rng.bytes(es);
                let e = mk_elem(&key, es, &filler);
                p.put_struct(&key, &e, HM_BINARY, &format!("B62 es={es} reput {k}"));
            }
            assert_eq!(p.c.hmlen(), 50, "B62 es={es}");
            p.free(&format!("B62 es={es} free"));
        }
    });
}

/// B65 — deterministic multi-bucket probing (axis A4).
///
/// `stbds_hm_find_slot` / `stbds_hmput_key` only reach the
/// `pos += step; step += STBDS_BUCKET_LENGTH` probe continuation when a whole
/// 8-slot bucket is exhausted.  Rather than hoping random keys collide, the keys
/// are *selected* by calling the library's own `stbds_hash_bytes` through the
/// FFI so that 24 of them land in the same bucket of a 1024-slot table.
#[test]
fn cfg_b65_multi_bucket_probing() {
    for seed in 1u64..=3 {
        with_libs(0x1234_0000 + seed as usize, |c, r| unsafe {
            let mut rng = Rng::new(0x65 + seed);
            let mut p = Pair::new(c, r, Shape::binary(16, 8));

            // grow the table to 1024 slots with filler keys
            let mut filler = 0u64;
            while p
                .c
                .snapshot()
                .table
                .as_ref()
                .map(|t| t.slot_count)
                .unwrap_or(0)
                < 1024
            {
                let key = filler.to_ne_bytes();
                let e = mk_elem(&key, 16, &filler.to_ne_bytes());
                p.put_struct(&key, &e, HM_BINARY, &format!("B65 filler {filler}"));
                filler += 1;
                assert!(filler < 4000, "B65 table never reached 1024 slots");
            }
            let t = p.c.snapshot().table.unwrap();
            assert_eq!(t.slot_count, 1024);
            let table_seed = t.seed;
            let used_before = t.used_count;

            // pick 24 keys that all probe into bucket 0, spread over every
            // `pos & 7` start offset so both the forward and the wrap scan run
            let mut colliding: Vec<[u8; 8]> = Vec::new();
            let mut tries = 0u32;
            while colliding.len() < 24 {
                tries += 1;
                assert!(tries < 2_000_000, "B65 could not find colliding keys");
                let k = rng.next_u64() | (1u64 << 63); // keep away from the filler range
                let kb = k.to_ne_bytes();
                let mut h = (c.hash_bytes)(kb.as_ptr() as *mut _, 8, table_seed);
                let h2 = (r.hash_bytes)(kb.as_ptr() as *mut _, 8, table_seed);
                assert_eq!(h, h2, "B65 hash_bytes must already agree");
                if h < 2 {
                    h += 2;
                }
                if (h & 1023) >> 3 == 0 {
                    colliding.push(kb);
                }
            }
            // staying under used_count_threshold keeps slot_count at 1024
            assert!(
                used_before + colliding.len() < t.used_count_threshold,
                "B65 would trigger a grow"
            );

            for (i, k) in colliding.iter().enumerate() {
                let e = mk_elem(k, 16, &(0xAAAA_0000u64 + i as u64).to_ne_bytes());
                p.put_struct(k, &e, HM_BINARY, &format!("B65 s={seed} collide put {i}"));
            }
            assert_eq!(
                p.c.snapshot().table.unwrap().slot_count,
                1024,
                "B65 no grow happened"
            );
            // every colliding key must still be findable through the long probe
            for (i, k) in colliding.iter().enumerate() {
                assert!(
                    p.geti(k, HM_BINARY, &format!("B65 s={seed} collide get {i}")) >= 0,
                    "B65 colliding key {i} lost"
                );
                p.geti_ts(k, HM_BINARY, &format!("B65 s={seed} collide ts {i}"));
            }
            // a *missing* key mapping to the same bucket must terminate at -1
            let mut miss = 0u32;
            let mut checked = 0;
            while checked < 8 {
                miss += 1;
                let k = (0x5555_0000_0000_0000u64 + miss as u64).to_ne_bytes();
                let mut h = (c.hash_bytes)(k.as_ptr() as *mut _, 8, table_seed);
                if h < 2 {
                    h += 2;
                }
                if (h & 1023) >> 3 == 0 {
                    assert_eq!(
                        p.geti(&k, HM_BINARY, &format!("B65 s={seed} miss {checked}")),
                        -1
                    );
                    checked += 1;
                }
                assert!(miss < 1_000_000, "B65 missing-key search failed");
            }
            // delete/reinsert inside the saturated bucket → tombstone reuse on a
            // multi-bucket probe chain
            for (i, k) in colliding.iter().enumerate().step_by(2) {
                assert_eq!(
                    p.del(k, HM_BINARY, &format!("B65 s={seed} collide del {i}")),
                    1
                );
            }
            for (i, k) in colliding.iter().enumerate().step_by(2) {
                let e = mk_elem(k, 16, &(0xBBBB_0000u64 + i as u64).to_ne_bytes());
                p.put_struct(k, &e, HM_BINARY, &format!("B65 s={seed} collide reput {i}"));
            }
            for (i, k) in colliding.iter().enumerate() {
                assert!(p.geti(k, HM_BINARY, &format!("B65 s={seed} final get {i}")) >= 0);
            }
            p.free(&format!("B65 s={seed} free"));
        });
    }
}
