//! Phase B — CONFIGS.md rows 18..=36, 65, 68: binary-mode hash maps driven
//! through the low-level `stbds_hm*_key` exports of both `.so`s.

mod common;
use common::*;
use std::ffi::c_void;

/// (elemsize, keysize) shapes the C code distinguishes.
const SHAPES: &[(usize, usize)] = &[
    (8, 4),   // { int key; int value; }
    (16, 8),  // { uint64 key; uint64 value; }
    (20, 8),  // { int key[2]; int b,c,d; }  -- key smaller than element
    (16, 4),  // key smaller, padding after
    (1, 1),   // minimum sizes
    (4, 4),   // key == whole element
    (12, 4),  // odd element size -> unaligned strides
    (24, 16), // 16-byte key: two SipHash main-loop iterations
    (32, 24), // 24-byte key: three SipHash main-loop iterations
];

/// A `keysize`-byte key derived from `v`.  For `keysize > 8` the value is
/// spread over the whole key (with a fixed multiplier per 8-byte lane) so that
/// distinct `v` always give distinct keys and the SipHash main loop runs.
fn key_bytes(v: u64, keysize: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(keysize);
    let mut lane = 0u64;
    while out.len() < keysize {
        let x = v
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(lane.wrapping_mul(0x1234_5678_9ABC_DEF1))
            ^ (if lane == 0 { 0 } else { v });
        let bytes = if lane == 0 { v.to_le_bytes() } else { x.to_le_bytes() };
        for b in bytes {
            if out.len() == keysize {
                break;
            }
            out.push(b);
        }
        lane += 1;
    }
    out
}

/// row 18 — fresh map, one insert
#[test]
fn r18_single_insert_fresh_map() {
    let _g = reset_seeds(0x1234_5678);
    for &(elemsize, keysize) in SHAPES {
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        let k = ka.add(&key_bytes(0x42, keysize));
        let i = unsafe { m.put(k, STBDS_HM_BINARY, 1) };
        assert_eq!(i, 0, "first insert must land at index 0");
        assert_eq!(unsafe { map_len(m.tc, elemsize) }, 1);
        assert_eq!(unsafe { map_len(m.tr, elemsize) }, 1);
        unsafe { m.free() };
    }
}

/// rows 19..=22 — N random keys across all element/key shapes
#[test]
fn r19_r22_many_random_keys_all_shapes() {
    for &(elemsize, keysize) in SHAPES {
        for trial in 0..12u64 {
            let _g = reset_seeds(0x0A00_0000 + trial as usize);
            let mut rng = Rng::new(0xC0FFEE_0019 ^ (trial << 40) ^ (elemsize as u64) << 8);
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
            let n = if keysize == 1 { 120 } else { 200 };
            for j in 0..n {
                let v = rng.next_u64();
                let k = ka.add(&key_bytes(v, keysize));
                unsafe { m.put(k, STBDS_HM_BINARY, j) };
            }
            unsafe { m.free() };
        }
    }
}

/// row 23 — duplicate keys (hits the forward *and* the wrap-around duplicate
/// branch of `stbds_hmput_key`)
#[test]
fn r23_duplicate_keys() {
    for &(elemsize, keysize) in SHAPES {
        for trial in 0..8u64 {
            let _g = reset_seeds(0x0B00_0000 + trial as usize);
            let mut rng = Rng::new(0xC0FFEE_0023 ^ (trial << 32) ^ (elemsize as u64));
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
            // small distinct domain -> lots of repeats
            let domain: Vec<u64> = (0..17u64).collect();
            for j in 0..300u64 {
                let v = domain[rng.below(domain.len())];
                let k = ka.add(&key_bytes(v, keysize));
                unsafe { m.put(k, STBDS_HM_BINARY, j) };
            }
            unsafe { m.free() };
        }
    }
}

/// row 24 — dense tables: many distinct keys from a tiny value domain so that
/// probing must walk `pos += step` repeatedly
#[test]
fn r24_dense_probing() {
    for &(elemsize, keysize) in &[(8usize, 1usize), (16, 2), (8, 4)] {
        for trial in 0..6u64 {
            let _g = reset_seeds(0x0C00_0000 + trial as usize);
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
            let n: u64 = if keysize == 1 { 200 } else { 400 };
            for j in 0..n {
                let k = ka.add(&key_bytes(j, keysize));
                unsafe { m.put(k, STBDS_HM_BINARY, j) };
            }
            // look every key up again
            for j in 0..n {
                let k = ka.add(&key_bytes(j, keysize));
                unsafe { m.get(k, STBDS_HM_BINARY) };
            }
            unsafe { m.free() };
        }
    }
}

/// row 25 — `stbds_hmget_key` for present and absent keys
#[test]
fn r25_get_present_and_absent() {
    for &(elemsize, keysize) in SHAPES {
        let _g = reset_seeds(0x0D00_0000 + elemsize);
        let mut rng = Rng::new(0xC0FFEE_0025 ^ (elemsize as u64));
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        let mut present = Vec::new();
        for j in 0..90u64 {
            let v = rng.next_u64() & 0xFFFF;
            present.push(v);
            let k = ka.add(&key_bytes(v, keysize));
            unsafe { m.put(k, STBDS_HM_BINARY, j) };
        }
        for _ in 0..600 {
            let v = if rng.next_u64() & 1 == 0 {
                present[rng.below(present.len())]
            } else {
                rng.next_u64()
            };
            let k = ka.add(&key_bytes(v, keysize));
            unsafe { m.get(k, STBDS_HM_BINARY) };
        }
        unsafe { m.free() };
    }
}

/// row 26 — same through the `_ts` entry point
#[test]
fn r26_get_ts_present_and_absent() {
    for &(elemsize, keysize) in SHAPES {
        let _g = reset_seeds(0x0E00_0000 + elemsize);
        let mut rng = Rng::new(0xC0FFEE_0026 ^ (elemsize as u64));
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        let mut present = Vec::new();
        for j in 0..70u64 {
            let v = rng.next_u64() & 0x3FF;
            present.push(v);
            let k = ka.add(&key_bytes(v, keysize));
            unsafe { m.put(k, STBDS_HM_BINARY, j) };
        }
        for _ in 0..600 {
            let v = if rng.next_u64() & 1 == 0 {
                present[rng.below(present.len())]
            } else {
                rng.next_u64()
            };
            let k = ka.add(&key_bytes(v, keysize));
            let a = unsafe { m.get_ts(k, STBDS_HM_BINARY) };
            // `_ts` must agree with the non-`_ts` variant
            let b = unsafe { m.get(k, STBDS_HM_BINARY) };
            assert_eq!(a, b, "hmget_key_ts and hmget_key disagree");
        }
        unsafe { m.free() };
    }
}

/// row 27 — `hmget_key_ts(NULL, …)` allocates an array with **no** hash table;
/// a second call must take the `table == NULL` branch.
#[test]
fn r27_get_ts_on_null_then_tableless() {
    for &(elemsize, keysize) in SHAPES {
        let _g = reset_seeds(0x0F00_0000 + elemsize);
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        let k = ka.add(&key_bytes(7, keysize));
        let a = unsafe { m.get_ts(k, STBDS_HM_BINARY) };
        assert_eq!(a, -1);
        assert!(!m.tc.is_null() && !m.tr.is_null());
        assert!(!unsafe { m.snap_c() }.has_table);
        // second call: table == NULL branch
        let b = unsafe { m.get_ts(k, STBDS_HM_BINARY) };
        assert_eq!(b, -1);
        // and via hmget_key (writes header temp)
        let c = unsafe { m.get(k, STBDS_HM_BINARY) };
        assert_eq!(c, -1);
        // hmdel_key on a table-less map
        let d = unsafe { m.del(k, STBDS_HM_BINARY) };
        assert_eq!(d, 0);
        // now a put gives it a table
        unsafe { m.put(k, STBDS_HM_BINARY, 9) };
        assert!(unsafe { m.snap_c() }.has_table);
        assert_eq!(unsafe { m.get(k, STBDS_HM_BINARY) }, 0);
        unsafe { m.free() };
    }
}

/// row 28 — delete the last element (`old_index == final_index`)
#[test]
fn r28_delete_last_element() {
    for &(elemsize, keysize) in SHAPES {
        let _g = reset_seeds(0x1000_0000 + elemsize);
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        let keys: Vec<u64> = (1..=5u64).collect();
        for (j, &v) in keys.iter().enumerate() {
            let k = ka.add(&key_bytes(v, keysize));
            unsafe { m.put(k, STBDS_HM_BINARY, j as u64) };
        }
        // delete in reverse insertion order -> always the final element
        for &v in keys.iter().rev() {
            let k = ka.add(&key_bytes(v, keysize));
            let t = unsafe { m.del(k, STBDS_HM_BINARY) };
            assert_eq!(t, 1, "delete of a present key must report 1");
        }
        assert_eq!(unsafe { map_len(m.tc, elemsize) }, 0);
        unsafe { m.free() };
    }
}

/// row 29 — delete a middle element (relocation of the final element + re-find)
#[test]
fn r29_delete_middle_elements() {
    for &(elemsize, keysize) in SHAPES {
        for trial in 0..10u64 {
            let _g = reset_seeds(0x1100_0000 + trial as usize);
            let mut rng = Rng::new(0xC0FFEE_0029 ^ (trial << 24) ^ elemsize as u64);
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
            let mut live: Vec<Vec<u8>> = Vec::new();
            for j in 0..60u64 {
                let kb = key_bytes(rng.next_u64(), keysize);
                if live.contains(&kb) {
                    continue;
                }
                live.push(kb.clone());
                let k = ka.add(&kb);
                unsafe { m.put(k, STBDS_HM_BINARY, j) };
            }
            while !live.is_empty() {
                let idx = rng.below(live.len());
                let kb = live.remove(idx);
                let k = ka.add(&kb);
                let t = unsafe { m.del(k, STBDS_HM_BINARY) };
                assert_eq!(t, 1, "present key {kb:02x?} not found");
            }
            unsafe { m.free() };
        }
    }
}

/// row 30 — interleaved random put/get/del, full snapshot after every op
#[test]
fn r30_interleaved_ops() {
    for &(elemsize, keysize) in SHAPES {
        for trial in 0..6u64 {
            let _g = reset_seeds(0x1200_0000 + trial as usize);
            let mut rng = Rng::new(0xC0FFEE_0030 ^ (trial << 16) ^ elemsize as u64);
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
            let domain: Vec<u64> = (0..40u64).map(|i| i * 0x9E37).collect();
            for j in 0..300u64 {
                let v = domain[rng.below(domain.len())];
                let k = ka.add(&key_bytes(v, keysize));
                match rng.below(3) {
                    0 => {
                        unsafe { m.put(k, STBDS_HM_BINARY, j) };
                    }
                    1 => {
                        unsafe { m.get(k, STBDS_HM_BINARY) };
                    }
                    _ => {
                        unsafe { m.del(k, STBDS_HM_BINARY) };
                    }
                }
            }
            unsafe { m.free() };
        }
    }
}

/// row 31 — shrink path (`used_count < used_count_shrink_threshold && slot_count > 8`)
#[test]
fn r31_shrink_path() {
    for &(elemsize, keysize) in SHAPES {
        for trial in 0..6u64 {
            let _g = reset_seeds(0x1300_0000 + trial as usize);
            let mut rng = Rng::new(0xC0FFEE_0031 ^ (trial << 8) ^ elemsize as u64);
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
            let mut live: Vec<Vec<u8>> = Vec::new();
            // grow well past 8 -> 16 -> 32 -> 64 -> 128 slots
            let mut j = 0u64;
            let target = if keysize == 1 { 100 } else { 100 };
            while live.len() < target {
                let kb = key_bytes(rng.next_u64(), keysize);
                if live.contains(&kb) {
                    continue;
                }
                live.push(kb.clone());
                let k = ka.add(&kb);
                unsafe { m.put(k, STBDS_HM_BINARY, j) };
                j += 1;
            }
            let big = unsafe { m.snap_c() }.slot_count;
            assert!(big >= 128, "expected a grown table, got {big}");
            // now delete almost everything -> repeated shrinks
            while live.len() > 1 {
                let kb = live.remove(rng.below(live.len()));
                let k = ka.add(&kb);
                assert_eq!(unsafe { m.del(k, STBDS_HM_BINARY) }, 1);
            }
            let small = unsafe { m.snap_c() }.slot_count;
            assert!(small < big, "table should have shrunk ({big} -> {small})");
            unsafe { m.free() };
        }
    }
}

/// row 32 — tombstone rebuild (`tombstone_count > tombstone_count_threshold`)
#[test]
fn r32_tombstone_rebuild() {
    for &(elemsize, keysize) in SHAPES {
        for trial in 0..6u64 {
            let _g = reset_seeds(0x1400_0000 + trial as usize);
            let mut rng = Rng::new(0xC0FFEE_0032 ^ (trial << 4) ^ elemsize as u64);
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
            let mut live: Vec<Vec<u8>> = Vec::new();
            // fill to ~70 keys
            let mut j = 0u64;
            let mut ctr = 0u64;
            while live.len() < 70 {
                ctr += 1;
                let kb = key_bytes(ctr.wrapping_mul(0x9E3779B1), keysize);
                if live.contains(&kb) {
                    continue;
                }
                live.push(kb.clone());
                let k = ka.add(&kb);
                unsafe { m.put(k, STBDS_HM_BINARY, j) };
                j += 1;
            }
            // churn: delete one, insert one -> tombstones accumulate until the
            // same-size rebuild fires
            for _ in 0..400 {
                let kb = live.remove(rng.below(live.len()));
                let k = ka.add(&kb);
                assert_eq!(unsafe { m.del(k, STBDS_HM_BINARY) }, 1);
                let mut nb;
                loop {
                    ctr += 1;
                    nb = key_bytes(ctr.wrapping_mul(0x9E3779B1), keysize);
                    if !live.contains(&nb) {
                        break;
                    }
                }
                live.push(nb.clone());
                let k2 = ka.add(&nb);
                unsafe { m.put(k2, STBDS_HM_BINARY, j) };
                j += 1;
            }
            unsafe { m.free() };
        }
    }
}

/// row 33 — `hmput_default(NULL, elemsize)`
#[test]
fn r33_put_default_from_null() {
    let _g = reset_seeds(0x1500_0000);
    for &(elemsize, keysize) in SHAPES {
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        unsafe { m.put_default() };
        let s = unsafe { m.snap_c() };
        assert_eq!(s.length, 1);
        assert!(!s.has_table);
        assert_eq!(unsafe { map_len(m.tc, elemsize) }, 0);
        unsafe { m.free() };
    }
}

/// row 34 — `hmput_default` on a map that already has elements: unchanged
#[test]
fn r34_put_default_noop() {
    for &(elemsize, keysize) in SHAPES {
        let _g = reset_seeds(0x1600_0000 + elemsize);
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        let k = ka.add(&key_bytes(3, keysize));
        unsafe { m.put(k, STBDS_HM_BINARY, 5) };
        let before_c = unsafe { m.snap_c() };
        let tc = m.tc;
        let tr = m.tr;
        unsafe { m.put_default() };
        assert_eq!(m.tc, tc, "C hmput_default must not move the map");
        assert_eq!(m.tr, tr, "Rust hmput_default must not move the map");
        assert_eq!(unsafe { m.snap_c() }, before_c);
        unsafe { m.free() };
    }
}

/// row 35 — `hmput_default` on an array whose length is 0
#[test]
fn r35_put_default_on_zero_length_array() {
    let p = common::libs();
    let _g = reset_seeds(0x1700_0000);
    for &(elemsize, keysize) in SHAPES {
        // build a zero-length array with arrgrowf, then hand its *hash pointer*
        // to hmput_default (the `length == 0` branch)
        let ac = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        let ar = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        let mut m = DualMap {
            tc: unsafe { (ac as *mut u8).add(elemsize) as *mut c_void },
            tr: unsafe { (ar as *mut u8).add(elemsize) as *mut c_void },
            elemsize,
            keysize,
            kr: KeyRepr::Raw,
            ops: 0,
        };
        unsafe { m.put_default() };
        let s = unsafe { m.snap_c() };
        assert_eq!(s.length, 1);
        unsafe { m.free() };
    }
}

/// row 36 — `hmput_default` then real inserts: the default element (index 0)
/// must be preserved
#[test]
fn r36_default_then_inserts() {
    for &(elemsize, keysize) in SHAPES {
        for trial in 0..6u64 {
            let _g = reset_seeds(0x1800_0000 + trial as usize);
            let mut rng = Rng::new(0xC0FFEE_0036 ^ (trial << 12) ^ elemsize as u64);
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
            unsafe { m.put_default() };
            // write the default value like `hmdefault(t,v)` does: t[-1].value
            unsafe {
                write_value(m.tc, elemsize, keysize, -1, 0xDEAD);
                write_value(m.tr, elemsize, keysize, -1, 0xDEAD);
            }
            for j in 0..80u64 {
                let v = rng.next_u64();
                let k = ka.add(&key_bytes(v, keysize));
                unsafe { m.put(k, STBDS_HM_BINARY, j) };
            }
            for j in 0..40u64 {
                let v = rng.next_u64();
                let k = ka.add(&key_bytes(v, keysize));
                unsafe { m.get(k, STBDS_HM_BINARY) };
                unsafe { m.del(k, STBDS_HM_BINARY) };
                let _ = j;
            }
            unsafe { m.free() };
        }
    }
}

/// row 65 — `keysize == 0`: `memcmp(...,0)` is always equal, and
/// `hash_bytes(key,0,seed)` is the same for every key, so every key aliases
/// onto the first one.
#[test]
fn r65_zero_keysize() {
    for &elemsize in &[1usize, 4, 8, 16] {
        let _g = reset_seeds(0x1900_0000 + elemsize);
        let mut rng = Rng::new(0xC0FFEE_0065 ^ elemsize as u64);
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, 0, KeyRepr::Raw);
        for j in 0..40u64 {
            let k = ka.add(&rng.bytes(8));
            unsafe { m.put(k, STBDS_HM_BINARY, j) };
        }
        assert_eq!(
            unsafe { map_len(m.tc, elemsize) },
            1,
            "with keysize==0 every key must alias"
        );
        for _ in 0..20 {
            let k = ka.add(&rng.bytes(8));
            assert_eq!(unsafe { m.get(k, STBDS_HM_BINARY) }, 0);
        }
        let k = ka.add(&rng.bytes(8));
        assert_eq!(unsafe { m.del(k, STBDS_HM_BINARY) }, 1);
        assert_eq!(unsafe { map_len(m.tc, elemsize) }, 0);
        unsafe { m.free() };
    }
}

/// row 68 — long mixed stress run (grow 8→…→512, shrink, rebuild)
#[test]
fn r68_long_stress() {
    for &(elemsize, keysize) in &[(8usize, 4usize), (16, 8), (20, 8)] {
        for trial in 0..3u64 {
            let _g = reset_seeds(0x1A00_0000 + trial as usize);
            let mut rng = Rng::new(0xC0FFEE_0068 ^ (trial << 20) ^ elemsize as u64);
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
            let mut live: Vec<u64> = Vec::new();
            for j in 0..2000u64 {
                let r = rng.below(100);
                if r < 55 || live.is_empty() {
                    let v = rng.next_u64() & 0xFFFF_FFFF;
                    let k = ka.add(&key_bytes(v, keysize));
                    unsafe { m.put(k, STBDS_HM_BINARY, j) };
                    if !live.contains(&v) {
                        live.push(v);
                    }
                } else if r < 80 {
                    let v = live[rng.below(live.len())];
                    let k = ka.add(&key_bytes(v, keysize));
                    unsafe { m.get(k, STBDS_HM_BINARY) };
                } else {
                    let v = live.remove(rng.below(live.len()));
                    let k = ka.add(&key_bytes(v, keysize));
                    unsafe { m.del(k, STBDS_HM_BINARY) };
                }
            }
            unsafe { m.free() };
        }
    }
}
