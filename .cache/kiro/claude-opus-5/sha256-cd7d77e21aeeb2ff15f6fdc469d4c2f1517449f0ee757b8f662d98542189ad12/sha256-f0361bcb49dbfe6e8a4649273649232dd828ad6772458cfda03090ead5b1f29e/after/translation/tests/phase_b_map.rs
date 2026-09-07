//! Phase B — CONFIGS.md rows 23-73 and 87-90: the hash-map entry points.
//!
//! Every operation goes through both `.so` export tables and the *complete*
//! observable state is compared after each call: `stbds_array_header`
//! (length / capacity / hash_table presence / temp), every element byte, and
//! every `stbds_hash_index` field plus every bucket slot (`hash[]`,`index[]`).

mod common;

use common::*;

fn k32(v: u32) -> [u8; 4] {
    v.to_ne_bytes()
}

/// rows 23-27: `stbds_shmode_func` for every mode and element size.
#[test]
fn cfg_23_27_shmode_func() {
    let _g = lock();
    for &seed in &[DEFAULT_SEED, 0usize, 1, usize::MAX] {
        for elemsize in [8usize, 12, 16, 24, 32, 64] {
            for mode in [
                STBDS_SH_NONE,
                STBDS_SH_DEFAULT,
                STBDS_SH_STRDUP,
                STBDS_SH_ARENA,
            ] {
                reset_seeds(seed);
                let mut m = MapPair::from_shmode(elemsize, 8, mode, KeyKind::Binary);
                m.check(&format!("rows23-27 e={elemsize} mode={mode} seed={seed:#x}"));
                m.free();
            }
        }
    }
}

/// rows 28-30: `stbds_hmput_default`.
#[test]
fn cfg_28_30_hmput_default() {
    let _g = lock();
    for elemsize in [8usize, 16, 24] {
        // row 28: from NULL
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, 4, KeyKind::Binary);
        m.put_default("row28 first");
        m.set_default_tail(0xA1);
        m.check("row28 after default payload");
        // row 29: already created
        m.put_default("row29 second");
        m.put_default("row29 third");
        // row 29b: the `length == 0` arm of the same `if` — only reachable when
        // a consumer resets the length behind the library's back.
        unsafe {
            let craw = (m.ch as *mut u8).sub(elemsize);
            let rraw = (m.rh as *mut u8).sub(elemsize);
            std::ptr::write_unaligned(craw.sub(HEADER) as *mut usize, 0);
            std::ptr::write_unaligned(rraw.sub(HEADER) as *mut usize, 0);
        }
        m.put_default("row29 after length=0");
        m.set_default_tail(0xA3);
        m.check("row29 length=0 state");
        m.free();

        // row 30: handle produced by hmget_key(NULL) then hmput_default
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, 4, KeyKind::Binary);
        same("row30 miss", m.get("row30 get on NULL", &k32(7), STBDS_HM_BINARY), -1);
        m.set_default_tail(0xA2);
        m.put_default("row30 default after get");
        m.check("row30 state");
        m.free();
    }
}

/// rows 31-32: lookups on an unborn map.
#[test]
fn cfg_31_32_lookup_on_null() {
    let _g = lock();
    for elemsize in [8usize, 16] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, 4, KeyKind::Binary);
        let t = m.get("row31", &k32(1), STBDS_HM_BINARY);
        assert_eq!(t, -1, "row31: C must report -1");
        m.free();

        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, 4, KeyKind::Binary);
        let t = m.get_ts("row32", &k32(1), STBDS_HM_BINARY);
        assert_eq!(t, -1, "row32: C must report -1");
        // second call now goes down the "table == NULL" branch
        let t = m.get_ts("row32 again", &k32(2), STBDS_HM_BINARY);
        assert_eq!(t, -1);
        m.free();
    }
}

/// rows 33-39, 47: binary maps at every growth boundary, with updates and
/// wrapped-bucket probing, plus out-of-range negative `mode` values.
#[test]
fn cfg_33_39_47_binary_growth() {
    let _g = lock();
    let mut rng = Rng::new(0xD0);

    for mode in [STBDS_HM_BINARY, -1, i32::MIN] {
        for &n in &[1usize, 2, 5, 6, 7, 8, 12, 13, 16, 17, 50, 200] {
            reset_seeds(DEFAULT_SEED);
            let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
            for i in 0..n {
                m.put(&format!("rows33-36 mode={mode} n={n} put{i}"), &k32(i as u32), mode);
            }
            // every present key must be found, every absent key must miss
            for i in 0..n {
                let t = m.get(&format!("rows33-36 get{i}"), &k32(i as u32), mode);
                assert!(t >= 0, "present key {i} reported missing by C");
            }
            for i in n..n + 20 {
                let t = m.get(&format!("rows33-36 miss{i}"), &k32(i as u32), mode);
                assert_eq!(t, -1, "absent key {i} reported present by C");
            }
            // row 38: update path
            for i in 0..n {
                m.put(&format!("row38 update{i}"), &k32(i as u32), mode);
            }
            m.free();
        }
    }

    // rows 36-37, 39: many random keys => several grows, dense buckets, both
    // the straight and the wrapped in-bucket scans.
    for &n in &[200usize, 2000] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
        let mut keys = Vec::new();
        for i in 0..n {
            let k = rng.next_u32();
            keys.push(k);
            m.put(&format!("rows36-37 n={n} put{i}"), &k32(k), STBDS_HM_BINARY);
        }
        for (i, &k) in keys.iter().enumerate() {
            let t = m.get(&format!("rows36-37 get{i}"), &k32(k), STBDS_HM_BINARY);
            assert!(t >= 0);
        }
        for i in 0..200 {
            m.get(&format!("rows36-37 rndmiss{i}"), &k32(rng.next_u32()), STBDS_HM_BINARY);
        }
        m.free();
    }

    // row 39: small key space in a big table -> guaranteed dense clusters
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    for i in 0..300u32 {
        m.put(&format!("row39 put{i}"), &k32(i % 40), STBDS_HM_BINARY);
    }
    for i in 0..60u32 {
        m.get(&format!("row39 get{i}"), &k32(i), STBDS_HM_BINARY);
    }
    m.free();
}

/// rows 40-46, 49-51: every key/element size class.
#[test]
fn cfg_40_46_key_and_elem_sizes() {
    let _g = lock();
    let mut rng = Rng::new(0xD4);

    // (elemsize, keysize)
    let shapes: &[(usize, usize)] = &[
        (8, 1),   // row 40
        (8, 2),   // row 41
        (16, 8),  // row 42
        (24, 16), // row 43
        (8, 3),   // row 44
        (8, 5),   // row 44
        (16, 7),  // row 44
        (4, 4),   // row 45
        (64, 4),  // row 46
        (16, 16), // row 51 (keysize == elemsize, covers the payload)
        (8, 0),   // row 50
        (0, 0),   // row 49
    ];

    for &(elemsize, keysize) in shapes {
        for &n in &[1usize, 6, 40, 300] {
            reset_seeds(DEFAULT_SEED);
            let mut m = MapPair::new_null(elemsize, keysize, KeyKind::Binary);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..n {
                let k = if keysize == 0 {
                    Vec::new()
                } else {
                    rng.bytes(keysize)
                };
                keys.push(k.clone());
                m.put(
                    &format!("rows40-51 e={elemsize} k={keysize} n={n} put{i}"),
                    &k,
                    STBDS_HM_BINARY,
                );
            }
            for (i, k) in keys.iter().enumerate() {
                m.get(
                    &format!("rows40-51 e={elemsize} k={keysize} get{i}"),
                    k,
                    STBDS_HM_BINARY,
                );
            }
            for i in 0..20 {
                let k = if keysize == 0 {
                    Vec::new()
                } else {
                    rng.bytes(keysize)
                };
                m.get(
                    &format!("rows40-51 e={elemsize} k={keysize} rnd{i}"),
                    &k,
                    STBDS_HM_BINARY,
                );
            }
            // delete a few, including the degenerate shapes
            for (i, k) in keys.iter().enumerate().take(10) {
                m.del(
                    &format!("rows40-51 e={elemsize} k={keysize} del{i}"),
                    k,
                    0,
                    STBDS_HM_BINARY,
                );
            }
            m.free();
        }
    }
}

/// rows 48, 53-55: string maps with `string.mode == STBDS_SH_DEFAULT`
/// (created implicitly by `hmput_key` with `mode >= STBDS_HM_STRING`).
#[test]
fn cfg_48_53_55_string_default_mode() {
    let _g = lock();
    let mut rng = Rng::new(0xD8);

    for mode in [STBDS_HM_STRING, 2, 127, i32::MAX] {
        for &n in &[1usize, 5, 6, 50, 500] {
            reset_seeds(DEFAULT_SEED);
            let mut m =
                MapPair::new_null(16, 8, KeyKind::StringPtr { keyoffset: 0 });
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..n {
                let s = rng.ascii_cstring_range(1, 24);
                keys.push(s.clone());
                m.put(&format!("row48 mode={mode} n={n} put{i}"), &s, mode);
            }
            for (i, s) in keys.iter().enumerate() {
                let t = m.get(&format!("row48 mode={mode} get{i}"), s, mode);
                assert!(t >= 0, "present string key reported missing by C");
            }
            for i in 0..30 {
                let s = rng.ascii_cstring_range(25, 40);
                m.get(&format!("row48 mode={mode} miss{i}"), &s, mode);
            }
            // row 55: re-put duplicates (update path, sets stbds_temp_key)
            for (i, s) in keys.iter().enumerate() {
                m.put(&format!("row55 mode={mode} reput{i}"), s, mode);
            }
            m.free();
        }
    }

    // row 54: keys shaped exactly like the C driver's `strkey`
    reset_seeds(DEFAULT_SEED);
    let (c, _r) = libs();
    let mut m = MapPair::new_null(16, 8, KeyKind::StringPtr { keyoffset: 0 });
    let mut ns: Vec<i32> = vec![0, 1, -1, 12345, -12345, i32::MAX, i32::MIN];
    for _ in 0..200 {
        ns.push(rng.next_u32() as i32);
    }
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for &v in &ns {
        let s = unsafe {
            let p = (c.strkey)(v);
            let mut b = Vec::new();
            let mut q = p as *const u8;
            while *q != 0 {
                b.push(*q);
                q = q.add(1);
            }
            b.push(0);
            b
        };
        keys.push(s);
    }
    for (i, s) in keys.iter().enumerate() {
        m.put(&format!("row54 put{i}"), s, STBDS_HM_STRING);
    }
    for (i, s) in keys.iter().enumerate() {
        m.get(&format!("row54 get{i}"), s, STBDS_HM_STRING);
    }
    m.free();
}

/// rows 49-52: string maps created with an explicit arena mode.
#[test]
fn cfg_49_52_strdup_and_arena_modes() {
    let _g = lock();
    let mut rng = Rng::new(0xDC);

    for sh_mode in [STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for &n in &[1usize, 6, 60, 400] {
            reset_seeds(DEFAULT_SEED);
            let mut m = MapPair::from_shmode(
                16,
                8,
                sh_mode,
                KeyKind::StringPtr { keyoffset: 0 },
            );
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..n {
                let s = rng.ascii_cstring_range(1, 30);
                keys.push(s.clone());
                m.put(
                    &format!("rows49-50 sh={sh_mode} n={n} put{i}"),
                    &s,
                    STBDS_HM_STRING,
                );
            }
            for (i, s) in keys.iter().enumerate() {
                let t = m.get(&format!("rows49-50 sh={sh_mode} get{i}"), s, STBDS_HM_STRING);
                assert!(t >= 0);
            }
            for i in 0..20 {
                let s = rng.ascii_cstring_range(31, 50);
                m.get(&format!("rows49-50 sh={sh_mode} miss{i}"), &s, STBDS_HM_STRING);
            }
            m.free();
        }
    }

    // row 51: arena with long keys forcing the oversize-block branch, mixed
    // with short keys.
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::from_shmode(16, 8, STBDS_SH_ARENA, KeyKind::StringPtr { keyoffset: 0 });
    for i in 0..200 {
        let n = if i % 3 == 0 {
            rng.range(600, 2500)
        } else {
            rng.range(1, 30)
        };
        let s = rng.ascii_cstring(n);
        m.put(&format!("row51 put{i}"), &s, STBDS_HM_STRING);
    }
    m.free();

    // row 52: walk the arena `block` counter towards saturation with many
    // moderately sized keys.
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::from_shmode(16, 8, STBDS_SH_ARENA, KeyKind::StringPtr { keyoffset: 0 });
    for i in 0..1500 {
        let s = rng.ascii_cstring_range(200, 400);
        m.put(&format!("row52 put{i}"), &s, STBDS_HM_STRING);
    }
    m.free();

    // row 53: mode 2 on STRDUP and DEFAULT tables (string hashing, but the
    // strdup-free in hmdel_key is skipped because `mode == 1` is false)
    for sh_mode in [STBDS_SH_STRDUP, STBDS_SH_DEFAULT] {
        reset_seeds(DEFAULT_SEED);
        let mut m =
            MapPair::from_shmode(16, 8, sh_mode, KeyKind::StringPtr { keyoffset: 0 });
        let mut keys = Vec::new();
        for i in 0..60 {
            let s = rng.ascii_cstring_range(1, 20);
            keys.push(s.clone());
            m.put(&format!("row53 sh={sh_mode} put{i}"), &s, 2);
        }
        for (i, s) in keys.iter().enumerate() {
            m.get(&format!("row53 sh={sh_mode} get{i}"), s, 2);
        }
        m.free();
    }
}

/// rows 56-64: deletion in binary mode — every move / shrink / tombstone path.
#[test]
fn cfg_56_64_binary_deletes() {
    let _g = lock();
    let mut rng = Rng::new(0xE0);

    // row 56: delete the last entry (no move)
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    for i in 0..5u32 {
        m.put(&format!("row56 put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    for i in (0..5u32).rev() {
        m.del(&format!("row56 del{i}"), &k32(i), 0, STBDS_HM_BINARY);
    }
    m.free();

    // row 57: delete from the middle (move + re-find)
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    for i in 0..9u32 {
        m.put(&format!("row57 put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    for i in [3u32, 1, 5, 0, 7] {
        let t = m.del(&format!("row57 del{i}"), &k32(i), 0, STBDS_HM_BINARY);
        assert_eq!(t, 1, "row57: C must report temp == 1 on a successful delete");
        m.get(&format!("row57 check{i}"), &k32(i), STBDS_HM_BINARY);
    }
    m.free();

    // rows 58-59, 62, 64: delete everything in forward and reverse order at
    // several sizes (covers slot_count == 8 no-shrink and repeated shrinks)
    for &n in &[1usize, 5, 6, 7, 12, 13, 200] {
        for reverse in [false, true] {
            reset_seeds(DEFAULT_SEED);
            let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
            for i in 0..n {
                m.put(
                    &format!("rows58-64 n={n} rev={reverse} put{i}"),
                    &k32(i as u32),
                    STBDS_HM_BINARY,
                );
            }
            let order: Vec<usize> = if reverse {
                (0..n).rev().collect()
            } else {
                (0..n).collect()
            };
            for &i in &order {
                m.del(
                    &format!("rows58-64 n={n} rev={reverse} del{i}"),
                    &k32(i as u32),
                    0,
                    STBDS_HM_BINARY,
                );
            }
            for i in 0..n {
                let t = m.get(
                    &format!("rows58-64 n={n} rev={reverse} miss{i}"),
                    &k32(i as u32),
                    STBDS_HM_BINARY,
                );
                assert_eq!(t, -1, "deleted key still present in C");
            }
            m.free();
        }
    }

    // row 60: 500 entries deleted in random order
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    let mut keys: Vec<u32> = (0..500).map(|_| rng.next_u32()).collect();
    keys.sort_unstable();
    keys.dedup();
    for (i, &k) in keys.iter().enumerate() {
        m.put(&format!("row60 put{i}"), &k32(k), STBDS_HM_BINARY);
    }
    let mut order: Vec<usize> = (0..keys.len()).collect();
    for i in (1..order.len()).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    for &i in &order {
        m.del(&format!("row60 del{i}"), &k32(keys[i]), 0, STBDS_HM_BINARY);
    }
    m.free();

    // row 63: tombstone-rebuild — repeatedly delete and re-put the same keys
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    for i in 0..6u32 {
        m.put(&format!("row63 seed put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    for round in 0..80u32 {
        let k = 100 + round;
        m.put(&format!("row63 put{round}"), &k32(k), STBDS_HM_BINARY);
        m.del(&format!("row63 del{round}"), &k32(k), 0, STBDS_HM_BINARY);
    }
    m.free();

    // row 61: 2000 random put/get/del operations on one map
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(16, 4, KeyKind::Binary);
    let mut live: Vec<u32> = Vec::new();
    for op in 0..2000 {
        match rng.below(10) {
            0..=4 => {
                let k = rng.next_u32() % 700;
                live.push(k);
                m.put(&format!("row61 op{op} put"), &k32(k), STBDS_HM_BINARY);
            }
            5..=7 => {
                let k = if !live.is_empty() && rng.below(2) == 0 {
                    live[rng.below(live.len())]
                } else {
                    rng.next_u32() % 700
                };
                m.get(&format!("row61 op{op} get"), &k32(k), STBDS_HM_BINARY);
            }
            _ => {
                let k = if !live.is_empty() && rng.below(2) == 0 {
                    let ix = rng.below(live.len());
                    live.swap_remove(ix)
                } else {
                    rng.next_u32() % 700
                };
                m.del(&format!("row61 op{op} del"), &k32(k), 0, STBDS_HM_BINARY);
            }
        }
    }
    m.free();
}

/// rows 65-68: deletion in string mode for every arena mode.
#[test]
fn cfg_65_68_string_deletes() {
    let _g = lock();
    let mut rng = Rng::new(0xE4);

    // row 65: SH_DEFAULT
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(16, 8, KeyKind::StringPtr { keyoffset: 0 });
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..100 {
        let s = rng.ascii_cstring_range(1, 20);
        if keys.iter().any(|k| k == &s) {
            continue;
        }
        keys.push(s.clone());
        m.put(&format!("row65 put{i}"), &s, STBDS_HM_STRING);
    }
    for (i, s) in keys.iter().enumerate() {
        if i % 3 == 0 {
            m.del(&format!("row65 del{i}"), s, 0, STBDS_HM_STRING);
        }
    }
    for (i, s) in keys.iter().enumerate() {
        m.get(&format!("row65 get{i}"), s, STBDS_HM_STRING);
    }
    m.free();

    // rows 66-67: STRDUP (frees the duped key) and ARENA (does not)
    for sh_mode in [STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        reset_seeds(DEFAULT_SEED);
        let mut m =
            MapPair::from_shmode(16, 8, sh_mode, KeyKind::StringPtr { keyoffset: 0 });
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..120 {
            let s = rng.ascii_cstring_range(1, 24);
            if keys.iter().any(|k| k == &s) {
                continue;
            }
            keys.push(s.clone());
            m.put(&format!("rows66-67 sh={sh_mode} put{i}"), &s, STBDS_HM_STRING);
        }
        for (i, s) in keys.iter().enumerate() {
            m.del(&format!("rows66-67 sh={sh_mode} del{i}"), s, 0, STBDS_HM_STRING);
        }
        // re-insert the same texts after every key was freed / abandoned
        for (i, s) in keys.iter().enumerate() {
            m.put(&format!("rows66-67 sh={sh_mode} reput{i}"), s, STBDS_HM_STRING);
        }
        for (i, s) in keys.iter().enumerate() {
            m.get(&format!("rows66-67 sh={sh_mode} get{i}"), s, STBDS_HM_STRING);
        }
        m.free();
    }

    // row 68: mode 2 against STRDUP and DEFAULT tables -> string hashing for
    // the lookup, but `mode == STBDS_HM_STRING` is false, so `hmdel_key` skips
    // the strdup free AND takes the `else` branch of the re-find, which passes
    // the element *address* instead of the stored `char*`. That branch always
    // fails its `assert(slot >= 0)`, so it is an ERRORS.md row (53), not a
    // valid path; here only the `old_index == final_index` case is exercised,
    // which never re-finds. Deleting in reverse insertion order guarantees it.
    for sh_mode in [STBDS_SH_STRDUP, STBDS_SH_DEFAULT] {
        reset_seeds(DEFAULT_SEED);
        let mut m =
            MapPair::from_shmode(16, 8, sh_mode, KeyKind::StringPtr { keyoffset: 0 });
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..60 {
            let s = rng.ascii_cstring_range(1, 20);
            if keys.iter().any(|k| k == &s) {
                continue;
            }
            keys.push(s.clone());
            m.put(&format!("row68 sh={sh_mode} put{i}"), &s, 2);
        }
        for (i, s) in keys.iter().enumerate() {
            m.get(&format!("row68 sh={sh_mode} get{i}"), s, 2);
        }
        for (i, s) in keys.iter().enumerate().rev() {
            let t = m.del(&format!("row68 sh={sh_mode} del{i}"), s, 0, 2);
            assert_eq!(t, 1, "row68: C must report a successful delete");
        }
        m.free();
    }
}

/// row 69: `stbds_hmget_key_ts` on populated maps, hit / miss / after delete.
#[test]
fn cfg_69_hmget_key_ts() {
    let _g = lock();
    let mut rng = Rng::new(0xE8);

    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    for i in 0..80u32 {
        m.put(&format!("row69 put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    for i in 0..120u32 {
        let ts = m.get_ts(&format!("row69 ts{i}"), &k32(i), STBDS_HM_BINARY);
        let g = m.get(&format!("row69 g{i}"), &k32(i), STBDS_HM_BINARY);
        assert_eq!(ts, g, "row69: hmget_key_ts and hmget_key disagree in C");
    }
    for i in (0..80u32).step_by(3) {
        m.del(&format!("row69 del{i}"), &k32(i), 0, STBDS_HM_BINARY);
        m.get_ts(&format!("row69 ts-after-del{i}"), &k32(i), STBDS_HM_BINARY);
    }
    m.free();

    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::from_shmode(16, 8, STBDS_SH_ARENA, KeyKind::StringPtr { keyoffset: 0 });
    let mut keys = Vec::new();
    for i in 0..60 {
        let s = rng.ascii_cstring_range(1, 18);
        keys.push(s.clone());
        m.put(&format!("row69 s-put{i}"), &s, STBDS_HM_STRING);
    }
    for (i, s) in keys.iter().enumerate() {
        m.get_ts(&format!("row69 s-ts{i}"), s, STBDS_HM_STRING);
    }
    m.free();
}

/// rows 70-73: `stbds_hmfree_func` for every table shape.
#[test]
fn cfg_70_73_hmfree_func() {
    let _g = lock();
    let mut rng = Rng::new(0xEC);

    // row 70: binary tables of several sizes
    for &n in &[0usize, 1, 200] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
        for i in 0..n {
            m.put(&format!("row70 n={n} put{i}"), &k32(i as u32), STBDS_HM_BINARY);
        }
        m.free();
    }

    // rows 71-72: STRDUP and ARENA tables with entries
    for sh_mode in [STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for &n in &[1usize, 50, 300] {
            reset_seeds(DEFAULT_SEED);
            let mut m =
                MapPair::from_shmode(16, 8, sh_mode, KeyKind::StringPtr { keyoffset: 0 });
            for i in 0..n {
                let s = rng.ascii_cstring_range(1, 800);
                m.put(&format!("rows71-72 sh={sh_mode} n={n} put{i}"), &s, STBDS_HM_STRING);
            }
            m.free();
        }
    }

    // row 73: handles with hash_table == NULL
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(16, 4, KeyKind::Binary);
    m.put_default("row73 default");
    m.set_default_tail(0x5A);
    m.free();

    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(16, 4, KeyKind::Binary);
    m.get("row73 get on NULL", &k32(3), STBDS_HM_BINARY);
    m.free();
}

/// rows 87-90: full pipelines, state compared after every step.
#[test]
fn cfg_87_90_pipelines() {
    let _g = lock();
    let mut rng = Rng::new(0xF0);

    // row 87: STRDUP table, 300 strkey-shaped puts, random gets, random deletes
    reset_seeds(DEFAULT_SEED);
    let (c, _r) = libs();
    let mut m =
        MapPair::from_shmode(24, 8, STBDS_SH_STRDUP, KeyKind::StringPtr { keyoffset: 0 });
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..300i32 {
        let s = unsafe {
            let p = (c.strkey)(i * 7 - 13);
            let mut b = Vec::new();
            let mut q = p as *const u8;
            while *q != 0 {
                b.push(*q);
                q = q.add(1);
            }
            b.push(0);
            b
        };
        keys.push(s.clone());
        m.put(&format!("row87 put{i}"), &s, STBDS_HM_STRING);
    }
    for i in 0..300 {
        let s = &keys[rng.below(keys.len())];
        m.get(&format!("row87 get{i}"), s, STBDS_HM_STRING);
    }
    let mut order: Vec<usize> = (0..keys.len()).collect();
    for i in (1..order.len()).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    for &i in &order {
        m.del(&format!("row87 del{i}"), &keys[i], 0, STBDS_HM_STRING);
    }
    m.free();

    // row 88: hmget_key(NULL) -> hmput_default -> binary puts -> mixed ops
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(16, 4, KeyKind::Binary);
    m.get("row88 initial get", &k32(1), STBDS_HM_BINARY);
    m.put_default("row88 default");
    m.set_default_tail(0x77);
    let mut live: Vec<u32> = Vec::new();
    for i in 0..300u32 {
        m.put(&format!("row88 put{i}"), &k32(i), STBDS_HM_BINARY);
        live.push(i);
    }
    for op in 0..600 {
        match rng.below(3) {
            0 => {
                let k = rng.next_u32() % 400;
                m.get_ts(&format!("row88 op{op} ts"), &k32(k), STBDS_HM_BINARY);
            }
            1 => {
                let k = rng.next_u32() % 400;
                m.get(&format!("row88 op{op} get"), &k32(k), STBDS_HM_BINARY);
            }
            _ => {
                if !live.is_empty() {
                    let ix = rng.below(live.len());
                    let k = live.swap_remove(ix);
                    m.del(&format!("row88 op{op} del"), &k32(k), 0, STBDS_HM_BINARY);
                }
            }
        }
    }
    m.free();

    // row 89: ARENA table, mixed short/long keys, deletes, re-puts
    reset_seeds(DEFAULT_SEED);
    let mut m =
        MapPair::from_shmode(16, 8, STBDS_SH_ARENA, KeyKind::StringPtr { keyoffset: 0 });
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..250 {
        let n = if i % 5 == 0 {
            rng.range(700, 3000)
        } else {
            rng.range(1, 40)
        };
        let s = rng.ascii_cstring(n);
        if keys.iter().any(|k| k == &s) {
            continue;
        }
        keys.push(s.clone());
        m.put(&format!("row89 put{i}"), &s, STBDS_HM_STRING);
    }
    for (i, s) in keys.iter().enumerate() {
        if i % 2 == 0 {
            m.del(&format!("row89 del{i}"), s, 0, STBDS_HM_STRING);
        }
    }
    for (i, s) in keys.iter().enumerate() {
        m.put(&format!("row89 reput{i}"), s, STBDS_HM_STRING);
    }
    m.free();

    // row 90: keysize == 0 (all keys collide) and the fully degenerate shape
    for (elemsize, keysize) in [(8usize, 0usize), (0, 0)] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, keysize, KeyKind::Binary);
        for i in 0..40 {
            m.put(&format!("row90 e={elemsize} put{i}"), &[], STBDS_HM_BINARY);
            m.get(&format!("row90 e={elemsize} get{i}"), &[], STBDS_HM_BINARY);
        }
        for i in 0..5 {
            m.del(&format!("row90 e={elemsize} del{i}"), &[], 0, STBDS_HM_BINARY);
        }
        m.free();
    }
}

/// row 91: `stbds_temp_key` (`table->temp_key`). The C code writes it from
/// `stbds_hmput_key` in string mode — on the new-insert path for all three
/// arena modes, and on the "found existing key" path of the *upper* bucket scan
/// only (the wrapped scan deliberately does not, and that quirk is preserved).
///
/// The comparison is restricted to put-only workloads on tables whose keys are
/// never freed, because `hmdel_key` in `STBDS_SH_STRDUP` mode frees the key
/// without clearing `temp_key`, leaving it dangling in both libraries.
#[test]
fn cfg_91_temp_key() {
    let _g = lock();
    let mut rng = Rng::new(0xF4);

    // implicit STBDS_SH_DEFAULT table (keys stay owned by the caller)
    for &n in &[1usize, 6, 40, 300] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(16, 8, KeyKind::StringPtr { keyoffset: 0 });
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..n {
            let s = rng.ascii_cstring_range(1, 24);
            keys.push(s.clone());
            m.put(&format!("row91 default n={n} put{i}"), &s, STBDS_HM_STRING);
            // temp_key is defined from the first string put onwards
            m.cmp_temp_key = true;
            m.check(&format!("row91 default n={n} temp_key after put{i}"));
        }
        // update path (upper scan writes temp_key, wrapped scan does not)
        for (i, s) in keys.iter().enumerate() {
            m.put(&format!("row91 default reput{i}"), s, STBDS_HM_STRING);
        }
        // gets must not touch temp_key
        for (i, s) in keys.iter().enumerate() {
            m.get(&format!("row91 default get{i}"), s, STBDS_HM_STRING);
        }
        m.free();
    }

    // explicit STBDS_SH_ARENA table (arena memory is never freed per key)
    for &n in &[1usize, 6, 40, 300] {
        reset_seeds(DEFAULT_SEED);
        let mut m =
            MapPair::from_shmode(16, 8, STBDS_SH_ARENA, KeyKind::StringPtr { keyoffset: 0 });
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..n {
            let s = rng.ascii_cstring_range(1, 900);
            keys.push(s.clone());
            m.put(&format!("row91 arena n={n} put{i}"), &s, STBDS_HM_STRING);
            m.cmp_temp_key = true;
            m.check(&format!("row91 arena n={n} temp_key after put{i}"));
        }
        for (i, s) in keys.iter().enumerate() {
            m.put(&format!("row91 arena reput{i}"), s, STBDS_HM_STRING);
        }
        m.free();
    }

    // STBDS_SH_STRDUP, put-only (no deletes => no freed keys)
    for &n in &[1usize, 6, 40, 200] {
        reset_seeds(DEFAULT_SEED);
        let mut m =
            MapPair::from_shmode(24, 8, STBDS_SH_STRDUP, KeyKind::StringPtr { keyoffset: 0 });
        for i in 0..n {
            let s = rng.ascii_cstring_range(1, 30);
            m.put(&format!("row91 strdup n={n} put{i}"), &s, STBDS_HM_STRING);
            m.cmp_temp_key = true;
            m.check(&format!("row91 strdup n={n} temp_key after put{i}"));
        }
        m.cmp_temp_key = false;
        m.free();
    }
}

/// row 93: string-keyed maps under several global seeds. Because
/// `stbds_hash_bytes` ignores its seed (the siphash seed-cancellation quirk) but
/// `stbds_hash_string` does not, string maps are the only place where the seed
/// actually changes the bucket layout, the probe order and the growth history.
#[test]
fn cfg_93_string_maps_under_varied_seeds() {
    let _g = lock();
    let mut rng = Rng::new(0xF8);
    let mut seeds: Vec<usize> = vec![0, 1, 2, DEFAULT_SEED, usize::MAX, usize::MAX - 1];
    for _ in 0..6 {
        seeds.push(rng.next_u64() as usize);
    }

    for &seed in &seeds {
        for sh_mode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            reset_seeds(seed);
            let mut m =
                MapPair::from_shmode(16, 8, sh_mode, KeyKind::StringPtr { keyoffset: 0 });
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..150 {
                let s = rng.ascii_cstring_range(1, 26);
                if keys.iter().any(|k| k == &s) {
                    continue;
                }
                keys.push(s.clone());
                m.put(
                    &format!("row93 seed={seed:#x} sh={sh_mode} put{i}"),
                    &s,
                    STBDS_HM_STRING,
                );
            }
            for (i, s) in keys.iter().enumerate() {
                let t = m.get(
                    &format!("row93 seed={seed:#x} sh={sh_mode} get{i}"),
                    s,
                    STBDS_HM_STRING,
                );
                assert!(t >= 0, "row93: present key reported missing by C");
            }
            for i in 0..40 {
                let s = rng.ascii_cstring_range(27, 40);
                m.get(
                    &format!("row93 seed={seed:#x} sh={sh_mode} miss{i}"),
                    &s,
                    STBDS_HM_STRING,
                );
            }
            for (i, s) in keys.iter().enumerate() {
                if i % 3 == 0 {
                    m.del(
                        &format!("row93 seed={seed:#x} sh={sh_mode} del{i}"),
                        s,
                        0,
                        STBDS_HM_STRING,
                    );
                }
            }
            m.free();
        }
    }
}
