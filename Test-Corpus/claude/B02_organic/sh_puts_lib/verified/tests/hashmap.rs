//! Phase B — CONFIGS.md rows 23..44 and 48: the hash-map pipeline driven
//! through the low-level exported entry points
//! (`stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
//! `stbds_hmput_default`, `stbds_hmdel_key`, `stbds_shmode_func`,
//! `stbds_hmfree_func`), exactly as the `stbds_hm*` / `stbds_sh*` macros do.

mod common;
use common::*;

// ---------------------------------------------------------------- row 23
#[test]
fn binary_single() {
    let _g = lock();
    sync_seed(0x3141_5926);
    let mut m = Dual::new(8, false);
    let (a, b) = m.put_bin(&le32(42), 4, &le32(7), HM_BINARY);
    assert_eq!((a, b), (0, 0), "first insert must report index 0");
    m.check("binary_single put");
    assert_eq!(m.len(), (1, 1));
    let (a, b) = m.get(&le32(42), 4, HM_BINARY, false);
    assert_eq!(a, b);
    assert_eq!(a, 0);
    m.check("binary_single get hit");
    let (a, b) = m.get(&le32(43), 4, HM_BINARY, false);
    assert_eq!(a, b);
    assert_eq!(a, -1);
    m.check("binary_single get miss");
    m.free();
}

// ---------------------------------------------------------------- row 24
#[test]
fn binary_many_i32() {
    let _g = lock();
    sync_seed(0x3141_5926);
    let mut rng = Rng::new(0x2401);
    let mut m = Dual::new(8, false);
    let mut inserted: Vec<i32> = Vec::new();
    for i in 0..1000usize {
        // narrow key domain -> plenty of duplicate keys
        let k = (rng.next_u32() % 700) as i32 - 350;
        let (a, b) = m.put_bin(&le32(k), 4, &le32(i as i32), HM_BINARY);
        assert_eq!(a, b, "put index diverged at #{i} key {k}");
        m.check(&format!("many_i32 put #{i} key {k}"));
        if !inserted.contains(&k) {
            inserted.push(k);
        }
        assert_eq!(m.len().0, inserted.len() as isize, "length wrong at #{i}");
    }
    for (i, &k) in inserted.iter().enumerate() {
        let (a, b) = m.get(&le32(k), 4, HM_BINARY, false);
        assert_eq!(a, b, "get index diverged for key {k} (#{i})");
        assert!(a >= 0, "key {k} should be present");
    }
    for k in [-100000i32, 100000, i32::MIN, i32::MAX, 351, -351] {
        let (a, b) = m.get(&le32(k), 4, HM_BINARY, false);
        assert_eq!((a, b), (-1, -1), "key {k} must be absent");
    }
    m.check("many_i32 after gets");
    m.free();
}

// ---------------------------------------------------------------- row 25
#[test]
fn binary_many_i64() {
    let _g = lock();
    sync_seed(1);
    let mut rng = Rng::new(0x2502);
    let mut m = Dual::new(16, false);
    for i in 0..800usize {
        let k = (rng.next_u64() % 500) as i64 - 250;
        let (a, b) = m.put_bin(&le64(k), 8, &le64(i as i64 * 3), HM_BINARY);
        assert_eq!(a, b, "put diverged #{i}");
        m.check(&format!("many_i64 put #{i}"));
    }
    for i in 0..800usize {
        let k = (i as i64 % 600) - 250;
        let (a, b) = m.get(&le64(k), 8, HM_BINARY, false);
        assert_eq!(a, b, "get diverged for {k}");
    }
    m.check("many_i64 after gets");
    m.free();
}

// ---------------------------------------------------------------- row 26
#[test]
fn binary_compound_key() {
    let _g = lock();
    sync_seed(0xdead_beef);
    let mut rng = Rng::new(0x2603);
    let mut m = Dual::new(32, false);
    for i in 0..500usize {
        let mut key = Vec::new();
        key.extend_from_slice(&le32((rng.next_u32() % 40) as i32));
        key.extend_from_slice(&le32((rng.next_u32() % 40) as i32));
        key.extend_from_slice(&le64(0)); // padding, part of the 16-byte key
        let payload = rng.bytes(16);
        let (a, b) = m.put_bin(&key, 16, &payload, HM_BINARY);
        assert_eq!(a, b, "compound put diverged #{i}");
        m.check(&format!("compound put #{i}"));
    }
    m.free();
}

// ---------------------------------------------------------------- row 27
#[test]
fn binary_tiny_keys() {
    let _g = lock();
    for &keysize in &[1usize, 2] {
        sync_seed(0x1234_5678);
        let mut rng = Rng::new(0x2700 + keysize as u64);
        let mut m = Dual::new(8, false);
        for i in 0..400usize {
            let key = rng.bytes(keysize);
            let payload = rng.bytes(8 - keysize);
            let (a, b) = m.put_bin(&key, keysize, &payload, HM_BINARY);
            assert_eq!(a, b, "tiny put diverged ks={keysize} #{i}");
            m.check(&format!("tiny put ks={keysize} #{i}"));
        }
        for v in 0..=255u8 {
            let key = if keysize == 1 { vec![v] } else { vec![v, v ^ 0x5a] };
            let (a, b) = m.get(&key, keysize, HM_BINARY, false);
            assert_eq!(a, b, "tiny get diverged ks={keysize} v={v}");
        }
        m.check(&format!("tiny after gets ks={keysize}"));
        m.free();
    }
}

// ---------------------------------------------------------------- row 28
#[test]
fn get_ts_vs_get() {
    let _g = lock();
    sync_seed(7);
    let mut rng = Rng::new(0x2801);
    let mut m = Dual::new(16, false);
    for i in 0..200usize {
        let k = (rng.next_u64() % 300) as i64;
        m.put_bin(&le64(k), 8, &le64(i as i64), HM_BINARY);
    }
    m.check("get_ts setup");
    for k in 0..400i64 {
        // hmget_key_ts must NOT touch header->temp
        let temp_before = unsafe { (map_temp(m.c, 16), map_temp(m.r, 16)) };
        let (a, b) = m.get_ts(&le64(k), 8, HM_BINARY, false);
        assert_eq!(a, b, "get_ts diverged for {k}");
        let temp_after = unsafe { (map_temp(m.c, 16), map_temp(m.r, 16)) };
        assert_eq!(temp_before, temp_after, "get_ts must not write header->temp");
        let (c, d) = m.get(&le64(k), 8, HM_BINARY, false);
        assert_eq!((c, d), (a, b), "get and get_ts disagree for {k}");
        m.check(&format!("get_ts k={k}"));
    }
    m.free();
}

// ---------------------------------------------------------------- row 29
#[test]
fn default_value() {
    let _g = lock();
    sync_seed(0x99);
    let mut m = Dual::new(16, false);
    // hmdefault on a NULL map: creates the sentinel, no hash table yet
    let mut sentinel = le64(-1);
    sentinel.extend_from_slice(&le64(0xdead));
    m.put_default(&sentinel);
    m.check("hmdefault on NULL map");
    assert_eq!(m.len(), (0, 0));
    // lookups against a table-less map
    let (a, b) = m.get(&le64(5), 8, HM_BINARY, false);
    assert_eq!((a, b), (-1, -1));
    m.check("get on table-less map");
    // now insert and make sure the sentinel survives
    for i in 0..40i64 {
        m.put_bin(&le64(i), 8, &le64(i * 100), HM_BINARY);
        m.check(&format!("default_value put {i}"));
    }
    // hmdefault again on a non-empty map: must be a no-op allocation-wise
    let before = (m.c, m.r);
    m.put_default(&sentinel);
    assert_eq!((m.c, m.r), before, "hmput_default must not move the map");
    m.check("hmdefault on non-empty map");
    let (a, b) = m.get(&le64(1000), 8, HM_BINARY, false);
    assert_eq!((a, b), (-1, -1));
    m.check("miss keeps default");
    m.free();
}

// ---------------------------------------------------------------- row 30
#[test]
fn del_last() {
    let _g = lock();
    sync_seed(0x5150);
    let mut m = Dual::new(16, false);
    for i in 0..5i64 {
        m.put_bin(&le64(i), 8, &le64(i), HM_BINARY);
    }
    m.check("del_last setup");
    // delete in reverse insertion order -> always old_index == final_index
    for i in (0..5i64).rev() {
        let (a, b) = m.del(&le64(i), 8, 0, HM_BINARY, false);
        assert_eq!((a, b), (1, 1), "delete of {i} must report 1");
        m.check(&format!("del_last {i}"));
    }
    assert_eq!(m.len(), (0, 0));
    // deleting from an empty (but table-carrying) map
    let (a, b) = m.del(&le64(0), 8, 0, HM_BINARY, false);
    assert_eq!((a, b), (0, 0));
    m.check("del from emptied map");
    m.free();
}

// ---------------------------------------------------------------- row 31
#[test]
fn del_middle() {
    let _g = lock();
    sync_seed(0x5151);
    let mut m = Dual::new(16, false);
    for i in 0..5i64 {
        m.put_bin(&le64(i), 8, &le64(i * 11), HM_BINARY);
    }
    m.check("del_middle setup");
    // delete in insertion order -> old_index != final_index (memmove + fix-up)
    for i in 0..5i64 {
        let (a, b) = m.del(&le64(i), 8, 0, HM_BINARY, false);
        assert_eq!((a, b), (1, 1));
        m.check(&format!("del_middle {i}"));
        for j in (i + 1)..5 {
            let (x, y) = m.get(&le64(j), 8, HM_BINARY, false);
            assert_eq!(x, y, "post-delete lookup of {j} diverged");
            assert!(x >= 0, "{j} must still be present after deleting {i}");
        }
    }
    m.free();
}

// ---------------------------------------------------------------- row 32
#[test]
fn binary_churn() {
    let _g = lock();
    for seed in [0usize, 0x3141_5926, usize::MAX] {
        sync_seed(seed);
        let mut rng = Rng::new(0x3200 ^ seed as u64);
        let mut m = Dual::new(16, false);
        let mut live: Vec<i64> = Vec::new();
        for step in 0..3000usize {
            let op = rng.below(100);
            if op < 45 {
                let k = (rng.next_u64() % 200) as i64;
                let (a, b) = m.put_bin(&le64(k), 8, &le64(step as i64), HM_BINARY);
                assert_eq!(a, b, "churn put diverged step {step}");
                if !live.contains(&k) {
                    live.push(k);
                }
            } else if op < 80 {
                // delete an existing key most of the time, a missing one sometimes
                let k = if !live.is_empty() && rng.below(4) != 0 {
                    live[rng.below(live.len())]
                } else {
                    (rng.next_u64() % 400) as i64
                };
                let (a, b) = m.del(&le64(k), 8, 0, HM_BINARY, false);
                assert_eq!(a, b, "churn del diverged step {step} key {k}");
                live.retain(|&x| x != k);
            } else {
                let k = (rng.next_u64() % 400) as i64;
                let (a, b) = m.get(&le64(k), 8, HM_BINARY, false);
                assert_eq!(a, b, "churn get diverged step {step} key {k}");
            }
            m.check(&format!("churn seed={seed:#x} step {step}"));
            assert_eq!(m.len().0, live.len() as isize, "length wrong at step {step}");
        }
        // the table must actually have grown, shrunk and rebuilt during this run
        m.free();
    }
}

// ---------------------------------------------------------------- row 33
#[test]
fn tombstone_reuse() {
    let _g = lock();
    sync_seed(0xabcd);
    let mut m = Dual::new(16, false);
    // fill enough to grow the table, delete, re-insert onto the tombstones
    for i in 0..30i64 {
        m.put_bin(&le64(i), 8, &le64(i), HM_BINARY);
    }
    m.check("tombstone setup");
    for i in 0..10i64 {
        m.del(&le64(i), 8, 0, HM_BINARY, false);
        m.check(&format!("tombstone del {i}"));
    }
    for i in 0..10i64 {
        let (a, b) = m.put_bin(&le64(i), 8, &le64(i + 1000), HM_BINARY);
        assert_eq!(a, b, "tombstone reinsert {i} diverged");
        m.check(&format!("tombstone reinsert {i}"));
    }
    for i in 0..30i64 {
        let (a, b) = m.get(&le64(i), 8, HM_BINARY, false);
        assert_eq!(a, b);
        assert!(a >= 0, "{i} must be present");
    }
    m.check("tombstone final");
    m.free();
}

// ---------------------------------------------------------------- row 34
#[test]
fn string_default_mode() {
    let _g = lock();
    sync_seed(0x3141_5926);
    let mut rng = Rng::new(0x3401);
    let mut m = Dual::new(16, true);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..600usize {
        let key = if !keys.is_empty() && rng.below(3) == 0 {
            keys[rng.below(keys.len())].clone()
        } else {
            rng.cbytes_len(1, 12, b'a', b'z')
        };
        let (a, b) = m.put_str(&key, &le64(i as i64), HM_STRING);
        assert_eq!(a, b, "string put diverged #{i}");
        m.check(&format!("string_default put #{i}"));
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    for k in &keys {
        let (a, b) = m.get(k, 8, HM_STRING, true);
        assert_eq!(a, b, "string get diverged");
        assert!(a >= 0);
    }
    for k in [b"".to_vec(), b"zzzzzzzzzzzzzzzz".to_vec(), b"ABC".to_vec()] {
        let (a, b) = m.get(&k, 8, HM_STRING, true);
        assert_eq!(a, b);
    }
    m.check("string_default after gets");
    m.free();
}

// ---------------------------------------------------------------- row 35
#[test]
fn string_strdup_mode() {
    let _g = lock();
    sync_seed(0x1111);
    let mut rng = Rng::new(0x3502);
    let mut m = Dual::new(16, true);
    m.shmode(SH_STRDUP);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..300usize {
        let key = rng.cbytes_len(1, 10, b'A', b'Z');
        let (a, b) = m.put_str(&key, &le64(i as i64), HM_STRING);
        assert_eq!(a, b, "strdup put diverged #{i}");
        m.check(&format!("strdup put #{i}"));
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    // hmdel_key must free the strdup'ed key (mode == STBDS_HM_STRING exactly)
    for (i, k) in keys.clone().iter().enumerate() {
        if i % 3 == 0 {
            let (a, b) = m.del(k, 8, 0, HM_STRING, true);
            assert_eq!((a, b), (1, 1), "strdup delete diverged");
            m.check(&format!("strdup del #{i}"));
        }
    }
    for (i, k) in keys.iter().enumerate() {
        let (a, b) = m.get(k, 8, HM_STRING, true);
        assert_eq!(a, b, "strdup get diverged #{i}");
        assert_eq!(a >= 0, i % 3 != 0, "presence wrong for #{i}");
    }
    m.check("strdup final");
    m.free(); // frees every remaining strdup'ed key
}

// ---------------------------------------------------------------- row 36
#[test]
fn string_arena_mode() {
    let _g = lock();
    sync_seed(0x2222);
    let mut rng = Rng::new(0x3603);
    let mut m = Dual::new(16, true);
    m.shmode(SH_ARENA);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..400usize {
        // mixture of short keys and keys longer than the arena block size
        let len = if i % 37 == 0 { 600 + rng.below(200) } else { 1 + rng.below(20) };
        let key = rng.cbytes(len, b'a', b'z');
        let (a, b) = m.put_str(&key, &le64(i as i64), HM_STRING);
        assert_eq!(a, b, "arena put diverged #{i}");
        // the arena `block` / `remaining` trace is part of the snapshot
        m.check(&format!("arena put #{i} len={len}"));
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    for k in &keys {
        let (a, b) = m.get(k, 8, HM_STRING, true);
        assert_eq!(a, b);
        assert!(a >= 0);
    }
    for (i, k) in keys.clone().iter().enumerate() {
        if i % 5 == 0 {
            let (a, b) = m.del(k, 8, 0, HM_STRING, true);
            assert_eq!((a, b), (1, 1));
            m.check(&format!("arena del #{i}"));
        }
    }
    m.check("arena final");
    m.free(); // strreset() on the arena
}

// ---------------------------------------------------------------- row 37
#[test]
fn string_none_mode() {
    let _g = lock();
    sync_seed(0x3333);
    // string.mode == STBDS_SH_NONE with `mode = STBDS_HM_STRING`: the key is
    // hashed/compared as a string but *stored* with the `default:` raw
    // `memcpy(keysize)` arm, i.e. the first 8 bytes of the key text land in the
    // key field.  Keys are >= 8 bytes so the memcpy stays in bounds, and the
    // keys are distinct enough that no bucket-hash collision forces the
    // (then wild) `strcmp`.
    let mut m = Dual::new(16, false); // key field holds *text*, not a pointer
    m.shmode(SH_NONE);
    for (i, k) in [b"aaaaaaaa", b"bbbbbbbb", b"cccccccc"].iter().enumerate() {
        let (a, b) = m.put_str(&k[..], &le64(i as i64), HM_STRING);
        assert_eq!(a, b, "SH_NONE put diverged #{i}");
        m.check(&format!("SH_NONE put #{i}"));
    }
    unsafe {
        let t = map_table(m.c, 16);
        assert_eq!((*t).used_count, 3, "unexpected bucket collision");
        assert_eq!((*t).string.mode, 0);
    }
    m.free();
}

// ---------------------------------------------------------------- row 38
#[test]
fn string_default_explicit() {
    let _g = lock();
    sync_seed(0x4444);
    let mut rng = Rng::new(0x3804);
    let mut m = Dual::new(8, true); // key-only element
    m.shmode(SH_DEFAULT);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..250usize {
        let key = rng.cbytes_len(1, 15, b'0', b'9');
        let (a, b) = m.put_str(&key, &[], HM_STRING);
        assert_eq!(a, b, "explicit-default put diverged #{i}");
        m.check(&format!("explicit-default put #{i}"));
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    for k in &keys {
        let (a, b) = m.get(k, 8, HM_STRING, true);
        assert_eq!(a, b);
        assert!(a >= 0);
    }
    for (i, k) in keys.clone().iter().enumerate() {
        if i % 4 == 1 {
            let (a, b) = m.del(k, 8, 0, HM_STRING, true);
            assert_eq!((a, b), (1, 1));
            m.check(&format!("explicit-default del #{i}"));
        }
    }
    m.check("explicit-default final");
    m.free();
}

// ---------------------------------------------------------------- row 39
#[test]
fn string_dup_keys() {
    let _g = lock();
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        sync_seed(0x5555);
        let mut m = Dual::new(16, true);
        m.shmode(mode);
        // same content, different addresses each time (`put_str` allocates a
        // fresh buffer per call) -> exercises `strcmp` equality and the
        // `temp_key` update path
        for i in 0..40usize {
            let (a, b) = m.put_str(b"duplicate-key", &le64(i as i64), HM_STRING);
            assert_eq!(a, b, "dup put diverged (mode {mode}) #{i}");
            assert_eq!(a, 0, "duplicate must map to index 0");
            m.check(&format!("dup put mode={mode} #{i}"));
        }
        assert_eq!(m.len(), (1, 1));
        // `shputs`-style store: key := stbds_temp_key
        for i in 0..10usize {
            let (a, b) = m.puts_str(b"duplicate-key", &le64(1000 + i as i64), HM_STRING);
            assert_eq!(a, b);
            m.check(&format!("dup puts mode={mode} #{i}"));
        }
        m.free();
    }
}

// ---------------------------------------------------------------- row 40
#[test]
fn string_edge_keys() {
    let _g = lock();
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        sync_seed(0x6666);
        let mut m = Dual::new(16, true);
        m.shmode(mode);
        let keys: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"ab".to_vec(),
            b"abc".to_vec(),
            vec![b'k'; 255],
            vec![b'k'; 256],
            vec![b'k'; 257],
            vec![0xffu8; 32],
            vec![0x80u8; 33],
            b"prefix".to_vec(),
            b"prefixprefix".to_vec(),
            b"prefixprefixprefix".to_vec(),
        ];
        for (i, k) in keys.iter().enumerate() {
            let (a, b) = m.put_str(k, &le64(i as i64), HM_STRING);
            assert_eq!(a, b, "edge put diverged mode={mode} #{i}");
            m.check(&format!("edge put mode={mode} #{i}"));
        }
        for (i, k) in keys.iter().enumerate() {
            let (a, b) = m.get(k, 8, HM_STRING, true);
            assert_eq!(a, b, "edge get diverged mode={mode} #{i}");
            assert!(a >= 0);
        }
        for (i, k) in keys.iter().enumerate() {
            let (a, b) = m.del(k, 8, 0, HM_STRING, true);
            assert_eq!((a, b), (1, 1), "edge del diverged mode={mode} #{i}");
            m.check(&format!("edge del mode={mode} #{i}"));
        }
        assert_eq!(m.len(), (0, 0));
        m.free();
    }
}

// ---------------------------------------------------------------- row 41
#[test]
fn string_churn() {
    let _g = lock();
    for mode in [SH_STRDUP, SH_ARENA, SH_DEFAULT] {
        sync_seed(0x7777 ^ mode as usize);
        let mut rng = Rng::new(0x4100 + mode as u64);
        let mut m = Dual::new(16, true);
        m.shmode(mode);
        let mut live: Vec<Vec<u8>> = Vec::new();
        for step in 0..2000usize {
            let op = rng.below(100);
            if op < 50 {
                let key = rng.cbytes_len(1, 6, b'a', b'f');
                let (a, b) = m.put_str(&key, &le64(step as i64), HM_STRING);
                assert_eq!(a, b, "string churn put diverged mode={mode} step {step}");
                if !live.contains(&key) {
                    live.push(key);
                }
            } else if op < 85 {
                let key = if !live.is_empty() && rng.below(4) != 0 {
                    live[rng.below(live.len())].clone()
                } else {
                    rng.cbytes_len(1, 6, b'a', b'f')
                };
                let (a, b) = m.del(&key, 8, 0, HM_STRING, true);
                assert_eq!(a, b, "string churn del diverged mode={mode} step {step}");
                live.retain(|x| x != &key);
            } else {
                let key = rng.cbytes_len(1, 6, b'a', b'f');
                let (a, b) = m.get(&key, 8, HM_STRING, true);
                assert_eq!(a, b, "string churn get diverged mode={mode} step {step}");
            }
            m.check(&format!("string churn mode={mode} step {step}"));
            assert_eq!(m.len().0, live.len() as isize, "len wrong mode={mode} step {step}");
        }
        m.free();
    }
}

// ---------------------------------------------------------------- row 42
#[test]
fn del_keyoffset() {
    let _g = lock();
    sync_seed(0x8888);
    let mut m = Dual::new(16, false);
    // payload deliberately never equals the key, so the mismatched keyoffset
    // can only ever miss
    for i in 0..20i64 {
        m.put_bin(&le64(i), 8, &le64(!i ^ 0x5555_5555_5555_5555), HM_BINARY);
    }
    m.check("del_keyoffset setup");
    for i in 0..20i64 {
        let (a, b) = m.del(&le64(i), 8, 8, HM_BINARY, false);
        assert_eq!(a, b, "keyoffset=8 delete diverged for {i}");
        assert_eq!((a, b), (0, 0), "keyoffset=8 must miss");
        m.check(&format!("del keyoffset=8 {i}"));
    }
    assert_eq!(m.len(), (20, 20), "nothing may have been deleted");
    // keyoffset = 0 works normally
    for i in 0..20i64 {
        let (a, b) = m.del(&le64(i), 8, 0, HM_BINARY, false);
        assert_eq!((a, b), (1, 1));
        m.check(&format!("del keyoffset=0 {i}"));
    }
    assert_eq!(m.len(), (0, 0));
    m.free();
}

// ---------------------------------------------------------------- row 43
#[test]
fn free_all_modes() {
    let _g = lock();
    for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        // empty map (length == 1, no user elements)
        sync_seed(0x9999);
        let mut m = Dual::new(16, false);
        m.shmode(mode);
        m.check(&format!("fresh shmode({mode})"));
        m.free();

        // map with elements
        sync_seed(0x9999);
        let mut m = Dual::new(16, mode != SH_NONE);
        m.shmode(mode);
        for i in 0..12usize {
            let key: Vec<u8> = format!("key-{i:03}").into_bytes();
            m.put_str(&key, &le64(i as i64), HM_STRING);
            m.check(&format!("free_all_modes mode={mode} put #{i}"));
        }
        m.free();
    }
    // binary map created implicitly by hmput_key (string.mode == 0)
    sync_seed(0x9999);
    let mut m = Dual::new(16, false);
    for i in 0..12i64 {
        m.put_bin(&le64(i), 8, &le64(i), HM_BINARY);
    }
    m.check("implicit binary map");
    m.free();
    // map without a hash table at all (hmput_default only)
    sync_seed(0x9999);
    let mut m = Dual::new(16, false);
    m.put_default(&vec![0u8; 16]);
    m.check("table-less map");
    m.free();
}

// ---------------------------------------------------------------- row 44
#[test]
fn seeded_workloads() {
    let _g = lock();
    for seed in [0usize, 1, 2, 0x3141_5926, usize::MAX, usize::MAX - 1, 0xdead_beef] {
        sync_seed(seed);
        let mut rng = Rng::new(0x4400 ^ seed as u64);
        let mut m = Dual::new(16, false);
        for i in 0..300usize {
            let k = (rng.next_u64() % 120) as i64;
            m.put_bin(&le64(k), 8, &le64(i as i64), HM_BINARY);
            m.check(&format!("seeded seed={seed:#x} put #{i}"));
        }
        for i in 0..120i64 {
            let (a, b) = m.get(&le64(i), 8, HM_BINARY, false);
            assert_eq!(a, b, "seeded get diverged seed={seed:#x} key {i}");
        }
        for i in 0..120i64 {
            let (a, b) = m.del(&le64(i), 8, 0, HM_BINARY, false);
            assert_eq!(a, b, "seeded del diverged seed={seed:#x} key {i}");
            m.check(&format!("seeded seed={seed:#x} del {i}"));
        }
        m.free();
    }
}

// ---------------------------------------------------------------- row 48
#[test]
fn pipeline_random() {
    let _g = lock();
    let mut outer = Rng::new(0x4800);
    for run in 0..30usize {
        let seed = outer.next_u64() as usize;
        sync_seed(seed);
        let mut rng = Rng::new(seed as u64 ^ 0xfeed);
        let elemsize = [8usize, 16, 24, 32][run % 4];
        let keysize = [4usize, 8, 8, 16][run % 4];
        let mut m = Dual::new(elemsize, false);

        // 1. hmdefault
        let mut sentinel = vec![0u8; elemsize];
        for (i, b) in sentinel.iter_mut().enumerate() {
            *b = (0xa0 + i) as u8;
        }
        m.put_default(&sentinel);
        m.check(&format!("pipeline run {run} default"));

        // 2. N puts
        let n = 20 + rng.below(200);
        let mut live: Vec<Vec<u8>> = Vec::new();
        for i in 0..n {
            let mut key = rng.bytes(keysize);
            if !live.is_empty() && rng.below(3) == 0 {
                key = live[rng.below(live.len())].clone();
            }
            // shrink the key domain so duplicates and probe collisions happen
            key[0] &= 0x1f;
            let payload = rng.bytes(elemsize - keysize);
            let (a, b) = m.put_bin(&key, keysize, &payload, HM_BINARY);
            assert_eq!(a, b, "pipeline run {run} put #{i} diverged");
            if !live.contains(&key) {
                live.push(key);
            }
            m.check(&format!("pipeline run {run} put #{i}"));
        }

        // 3. hmget_key_ts over everything, present and absent
        for k in live.clone() {
            let (a, b) = m.get_ts(&k, keysize, HM_BINARY, false);
            assert_eq!(a, b, "pipeline run {run} get_ts diverged");
            assert!(a >= 0);
        }
        for _ in 0..50 {
            let mut k = rng.bytes(keysize);
            k[0] |= 0x80;
            let (a, b) = m.get_ts(&k, keysize, HM_BINARY, false);
            assert_eq!(a, b);
        }
        m.check(&format!("pipeline run {run} after get_ts"));

        // 4. M deletes
        let mut order = live.clone();
        for i in (1..order.len()).rev() {
            order.swap(i, rng.below(i + 1));
        }
        for (i, k) in order.iter().enumerate() {
            if i % 3 == 2 {
                continue;
            }
            let (a, b) = m.del(k, keysize, 0, HM_BINARY, false);
            assert_eq!(a, b, "pipeline run {run} del #{i} diverged");
            m.check(&format!("pipeline run {run} del #{i}"));
        }

        // 5. free
        m.free();
        assert!(m.checks > 0);
    }
}

// ---------------------------------------------------------------------------
// Branch-coverage confirmation: the randomised workloads above are only
// meaningful if they actually drive the table through growth, tombstone
// rebuilds and shrinking.  Prove it (and that C and Rust take the transitions
// at exactly the same step).
// ---------------------------------------------------------------------------
#[test]
fn table_lifecycle_coverage() {
    let _g = lock();
    let es = 16usize;
    let mut grows = 0usize;
    let mut shrinks = 0usize;
    let mut rebuilds = 0usize;
    let mut tombstone_reuses = 0usize;
    for seed in [0usize, 0x3141_5926, 42] {
        sync_seed(seed);
        let mut rng = Rng::new(0xC0DE ^ seed as u64);
        let mut m = Dual::new(es, false);
        let mut live: Vec<i64> = Vec::new();
        let mut prev: Option<(usize, usize, usize)> = None; // slot_count, tombstones, used
        for step in 0..4000usize {
            if rng.below(100) < 55 {
                let k = (rng.next_u64() % 400) as i64;
                m.put_bin(&le64(k), 8, &le64(k), HM_BINARY);
                if !live.contains(&k) {
                    live.push(k);
                }
            } else if !live.is_empty() {
                let k = live[rng.below(live.len())];
                m.del(&le64(k), 8, 0, HM_BINARY, false);
                live.retain(|&x| x != k);
            }
            m.check(&format!("lifecycle seed={seed:#x} step {step}"));
            if m.c.is_null() {
                continue;
            }
            unsafe {
                let t = map_table(m.c, es);
                if t.is_null() {
                    continue;
                }
                let cur = ((*t).slot_count, (*t).tombstone_count, (*t).used_count);
                if let Some(p) = prev {
                    if cur.0 > p.0 {
                        grows += 1;
                    } else if cur.0 < p.0 {
                        shrinks += 1;
                    } else if p.1 > 0 && cur.1 == 0 && cur.2 >= p.2 {
                        rebuilds += 1;
                    } else if cur.1 < p.1 && cur.2 > p.2 {
                        tombstone_reuses += 1;
                    }
                }
                prev = Some(cur);
            }
        }
        m.free();
    }
    assert!(grows > 0, "table never grew");
    assert!(shrinks > 0, "table never shrank");
    assert!(rebuilds > 0, "tombstone rebuild never happened");
    assert!(tombstone_reuses > 0, "tombstone reuse never happened");
}
