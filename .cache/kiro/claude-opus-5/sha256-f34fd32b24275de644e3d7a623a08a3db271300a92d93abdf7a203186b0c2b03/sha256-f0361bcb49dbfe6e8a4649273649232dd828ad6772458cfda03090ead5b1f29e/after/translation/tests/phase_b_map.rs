//! Phase B — `CONFIGS.md` rows 33..38 and 44..80: the hash-map back-end driven
//! through its lowest-level exports (`stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_hmdel_key`,
//! `stbds_shmode_func`, `stbds_hmfree_func`), exactly as the `stbds_hm*` /
//! `stbds_sh*` macros do.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

fn bin_kind(keysize: usize, voff: usize, vsize: usize) -> KeyKind {
    KeyKind::Binary {
        keysize,
        value_offset: voff,
        value_size: vsize,
    }
}

fn str_kind(voff: usize, vsize: usize) -> KeyKind {
    KeyKind::StringPtr {
        value_offset: voff,
        value_size: vsize,
    }
}

// ---------------------------------------------------------------------------
// rows 33..38 — bootstrap paths
// ---------------------------------------------------------------------------

/// row 33 — `hmput_default(NULL, elemsize)`
#[test]
fn row33_hmput_default_from_null() {
    let _g = serial();
    for elemsize in [8usize, 16, 24, 32, 64] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(elemsize, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        m.put_default();
        m.assert_same(&format!("hmput_default(NULL, {elemsize})"));
        let (a, _) = m.snap_pair();
        assert_eq!(a.length, 1);
        assert!(a.table.is_none());
        m.free();
    }
}

/// row 34 — `hmput_default` on a populated map is a no-op
#[test]
fn row34_hmput_default_noop() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    m.put_default();
    let before = m.snap_pair();
    let (pc, pr) = (m.c, m.r);
    m.put_default();
    assert_eq!(m.c, pc, "C hmput_default reallocated");
    assert_eq!(m.r, pr, "Rust hmput_default reallocated");
    assert_eq!(before, m.snap_pair());
    m.assert_same("hmput_default no-op");
    m.free();
}

/// row 35 — `hmput_key` then `hmput_default` (length != 0 → no-op)
#[test]
fn row35_hmput_default_after_put() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for k in 0..5u32 {
        m.put(&k.to_le_bytes(), &(k * 3).to_le_bytes());
    }
    m.assert_same("after 5 puts");
    let before = m.snap_pair();
    m.put_default();
    assert_eq!(before, m.snap_pair(), "hmput_default must not change state");
    m.assert_same("hmput_default after puts");
    m.free();
}

/// row 36 — `hmget_key_ts(NULL, ...)` bootstraps and reports a miss
#[test]
fn row36_hmget_key_ts_from_null() {
    let _g = serial();
    for elemsize in [8usize, 16, 32] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(elemsize, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        let (tc, tr) = m.geti_ts(&7u32.to_le_bytes());
        assert_eq!(tc, tr, "temp diverged (elemsize={elemsize})");
        assert_eq!(tc, -1);
        m.assert_same("hmget_key_ts(NULL)");
        let (a, _) = m.snap_pair();
        assert_eq!(a.length, 1);
        assert!(a.table.is_none());
        m.free();
    }
}

/// row 37 — `hmget_key_ts` on a map that has no hash table yet
#[test]
fn row37_hmget_key_ts_no_table() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 4, 8, STBDS_HM_BINARY, bin_kind(4, 8, 4));
    m.put_default();
    for k in [0u32, 1, 0xffff_ffff] {
        let (tc, tr) = m.geti_ts(&k.to_le_bytes());
        assert_eq!(tc, tr);
        assert_eq!(tc, -1, "must be a miss");
    }
    m.assert_same("hmget_key_ts, table == NULL");
    m.free();
}

/// row 38 — the same shapes observed through `stbds_header(...)->temp`
#[test]
fn row38_hmget_key_temp_field() {
    let _g = serial();
    for elemsize in [8usize, 16, 32] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(elemsize, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        let (tc, tr) = m.geti(&11u32.to_le_bytes());
        assert_eq!((tc, tr), (-1, -1), "elemsize={elemsize}");
        m.assert_same("hmget_key(NULL)");
        // now via hmput_default (table == NULL)
        let (tc, tr) = m.geti(&11u32.to_le_bytes());
        assert_eq!((tc, tr), (-1, -1));
        m.assert_same("hmget_key, table == NULL");
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 44..60 — binary maps
// ---------------------------------------------------------------------------

/// row 44 — one insert, `keysize = 4`, `elemsize = 8`
#[test]
fn row44_binary_single_insert() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    m.put(&42u32.to_le_bytes(), &99u32.to_le_bytes());
    m.assert_same("single binary insert");
    assert_eq!(m.len(), (1, 1));
    let (tc, tr) = m.geti(&42u32.to_le_bytes());
    assert_eq!((tc, tr), (0, 0));
    m.assert_same("get after single insert");
    m.free();
}

/// row 45 — inserts 1..8 with `elemsize = 16`: the table grows at used_count 6
#[test]
fn row45_binary_grow_at_threshold() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 4, 8, STBDS_HM_BINARY, bin_kind(4, 8, 4));
    for k in 0..8u32 {
        m.put(&k.to_le_bytes(), &(k ^ 0xa5a5_a5a5).to_le_bytes());
        m.assert_same(&format!("binary insert {k}"));
    }
    let (a, _) = m.snap_pair();
    assert_eq!(a.table.as_ref().unwrap().slot_count, 16, "must have grown");
    m.free();
}

/// row 46 — 1..300 random inserts (several ×2 growths)
#[test]
fn row46_binary_many_inserts() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0046);
    for elemsize in [8usize, 16] {
        reseed(DEFAULT_SEED);
        let voff = elemsize - 4;
        let mut m = MapPair::new(elemsize, 4, voff, STBDS_HM_BINARY, bin_kind(4, voff, 4));
        for i in 0..300u32 {
            let k = rng.next_u32();
            m.put(&k.to_le_bytes(), &i.to_le_bytes());
            m.assert_same(&format!("insert #{i} (elemsize={elemsize})"));
        }
        m.free();
    }
}

/// row 47 — 8-byte (`size_t`-sized) keys
#[test]
fn row47_binary_keysize8() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0047);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_BINARY, bin_kind(8, 8, 8));
    for i in 0..200u64 {
        let k = rng.next_u64();
        m.put(&k.to_le_bytes(), &i.to_le_bytes());
        m.assert_same(&format!("keysize=8 insert #{i}"));
    }
    m.free();
}

/// row 48 — 1-byte keys → heavy duplicate traffic
#[test]
fn row48_binary_keysize1() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0048);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 1, 4, STBDS_HM_BINARY, bin_kind(1, 4, 4));
    for i in 0..600u32 {
        let k = [rng.next_u32() as u8];
        m.put(&k, &i.to_le_bytes());
        m.assert_same(&format!("keysize=1 insert #{i}"));
    }
    assert!(m.len().0 <= 256);
    m.free();
}

/// row 49 — 16-byte keys (multi-block siphash), `elemsize = 24`
#[test]
fn row49_binary_keysize16() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0049);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(24, 16, 16, STBDS_HM_BINARY, bin_kind(16, 16, 8));
    for i in 0..200u64 {
        let k = rng.bytes(16);
        m.put(&k, &i.to_le_bytes());
        m.assert_same(&format!("keysize=16 insert #{i}"));
    }
    m.free();
}

/// row 50 — duplicate keys re-inserted (`temp` reused, no new slot)
#[test]
fn row50_binary_duplicates() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for k in 0..20u32 {
        m.put(&k.to_le_bytes(), &k.to_le_bytes());
    }
    m.assert_same("20 inserts");
    let len_before = m.len();
    for round in 0..5u32 {
        for k in 0..20u32 {
            m.put(&k.to_le_bytes(), &(k * 100 + round).to_le_bytes());
            m.assert_same(&format!("dup put k={k} round={round}"));
        }
    }
    assert_eq!(m.len(), len_before, "duplicates must not change the length");
    m.free();
}

/// row 51 — keys engineered so probing wraps inside the bucket
/// (`pos & BUCKET_MASK != 0`, exercising the second, "limit", scan)
#[test]
fn row51_binary_bucket_wrap() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0051);
    // Small tables plus many keys forces `pos & 7 != 0` starts and full-bucket
    // wrap-arounds; the state comparison catches any probe-order difference.
    for trial in 0..20 {
        reseed(0x1000 + trial as usize);
        let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        for i in 0..40u32 {
            let k = rng.next_u32() & 0xff;
            m.put(&k.to_le_bytes(), &i.to_le_bytes());
            m.assert_same(&format!("trial={trial} i={i}"));
            // interleave lookups so find_slot's two scans are exercised too
            let probe = rng.next_u32() & 0xff;
            let (a, b) = m.geti(&probe.to_le_bytes());
            assert_eq!(a, b, "geti diverged trial={trial} i={i}");
        }
        m.free();
    }
    reseed(DEFAULT_SEED);
}

/// row 52 — `hmget_key` hits and misses on a populated table
#[test]
fn row52_binary_get_hit_miss() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0052);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    let mut present = Vec::new();
    for i in 0..150u32 {
        let k = rng.next_u32() | 1; // keep 0 out of the set
        present.push(k);
        m.put(&k.to_le_bytes(), &i.to_le_bytes());
    }
    m.assert_same("populate");
    for &k in &present {
        let (a, b) = m.geti(&k.to_le_bytes());
        assert_eq!(a, b, "hit diverged for {k:#x}");
        assert!(a >= 0, "expected hit for {k:#x}");
    }
    for _ in 0..300 {
        let k = rng.next_u32() & !1; // even → never inserted
        let (a, b) = m.geti(&k.to_le_bytes());
        assert_eq!(a, b, "miss diverged for {k:#x}");
    }
    let (a, b) = m.geti(&0u32.to_le_bytes());
    assert_eq!((a, b), (-1, -1));
    m.assert_same("after gets");
    m.free();
}

/// row 53 — `hmget_key_ts` hits and misses through the out-param
#[test]
fn row53_binary_get_ts_hit_miss() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0053);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_BINARY, bin_kind(8, 8, 8));
    let mut present = Vec::new();
    for i in 0..120u64 {
        let k = rng.next_u64() | 1;
        present.push(k);
        m.put(&k.to_le_bytes(), &i.to_le_bytes());
    }
    for &k in &present {
        let (a, b) = m.geti_ts(&k.to_le_bytes());
        assert_eq!(a, b);
        assert!(a >= 0);
    }
    for _ in 0..300 {
        let k = rng.next_u64() & !1;
        let (a, b) = m.geti_ts(&k.to_le_bytes());
        assert_eq!(a, b);
    }
    m.assert_same("after get_ts");
    m.free();
}

/// row 54 — delete the last element (`old_index == final_index`)
#[test]
fn row54_binary_del_last() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for k in 0..5u32 {
        m.put(&k.to_le_bytes(), &k.to_le_bytes());
    }
    // element index 4 is the last one
    let (a, b) = m.del(&4u32.to_le_bytes());
    assert_eq!((a, b), (1, 1), "hmdel must report 1");
    m.assert_same("delete last element");
    assert_eq!(m.len(), (4, 4));
    m.free();
}

/// row 55 — delete a middle element (swap-with-last + re-lookup)
#[test]
fn row55_binary_del_middle() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for k in 0..5u32 {
        m.put(&k.to_le_bytes(), &(k + 1000).to_le_bytes());
    }
    for k in [0u32, 2, 1] {
        let (a, b) = m.del(&k.to_le_bytes());
        assert_eq!((a, b), (1, 1));
        m.assert_same(&format!("delete middle k={k}"));
    }
    m.free();
}

/// row 56 — delete a key that is not present
#[test]
fn row56_binary_del_missing() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for k in 1..6u32 {
        m.put(&k.to_le_bytes(), &k.to_le_bytes());
    }
    let before = m.snap_pair();
    for k in [0u32, 100, 0xffff_ffff] {
        let (a, b) = m.del(&k.to_le_bytes());
        assert_eq!((a, b), (0, 0), "missing delete must report 0 (k={k})");
    }
    // Everything except `temp` (which `hmdel_key` unconditionally zeroes) must
    // be untouched, and both libraries must agree.
    let after = m.snap_pair();
    assert_eq!(before.0.length, after.0.length);
    assert_eq!(before.0.capacity, after.0.capacity);
    assert_eq!(before.0.table, after.0.table);
    assert_eq!(before.0.elements, after.0.elements);
    assert_eq!(after.0.temp, 0, "hmdel_key sets temp = 0 on a miss");
    m.assert_same("delete missing");
    m.free();
}

/// row 57 — delete until the table shrinks
#[test]
fn row57_binary_del_shrink() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    for k in 0..200u32 {
        m.put(&k.to_le_bytes(), &k.to_le_bytes());
    }
    let big = m.snap_pair().0.table.as_ref().unwrap().slot_count;
    assert!(big >= 256, "table should have grown, got {big}");
    for k in 0..200u32 {
        let (a, b) = m.del(&k.to_le_bytes());
        assert_eq!((a, b), (1, 1), "delete k={k}");
        m.assert_same(&format!("shrink delete k={k}"));
    }
    let (s, _) = m.snap_pair();
    assert_eq!(s.table.as_ref().unwrap().slot_count, 8, "must shrink to 8");
    assert_eq!(m.len(), (0, 0));
    m.free();
}

/// row 58 — delete/insert churn that trips the tombstone rebuild
#[test]
fn row58_binary_tombstone_rebuild() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0058);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    let mut live: Vec<u32> = Vec::new();
    for i in 0..60u32 {
        let k = rng.next_u32() | 1;
        live.push(k);
        m.put(&k.to_le_bytes(), &i.to_le_bytes());
    }
    for round in 0..400u32 {
        // keep used_count roughly constant so the *tombstone* threshold, not
        // the shrink threshold, is what fires
        let idx = rng.below(live.len());
        let victim = live.swap_remove(idx);
        let (a, b) = m.del(&victim.to_le_bytes());
        assert_eq!(a, b, "del diverged at round {round}");
        m.assert_same(&format!("churn del round={round}"));
        let k = rng.next_u32() | 1;
        live.push(k);
        m.put(&k.to_le_bytes(), &round.to_le_bytes());
        m.assert_same(&format!("churn put round={round}"));
    }
    m.free();
}

/// row 59 — randomized put/get/del pipeline, `keysize = 4`
#[test]
fn row59_binary_pipeline_keysize4() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0059);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
    let mut live: Vec<u32> = Vec::new();
    for op in 0..2000u32 {
        match rng.below(10) {
            0..=4 => {
                let k = rng.next_u32() % 5000;
                if !live.contains(&k) {
                    live.push(k);
                }
                m.put(&k.to_le_bytes(), &op.to_le_bytes());
                m.assert_same(&format!("op={op} put"));
            }
            5..=7 => {
                let k = if !live.is_empty() && rng.below(2) == 0 {
                    live[rng.below(live.len())]
                } else {
                    rng.next_u32()
                };
                let (a, b) = m.geti(&k.to_le_bytes());
                assert_eq!(a, b, "op={op} geti diverged");
                m.assert_same(&format!("op={op} get"));
            }
            _ => {
                let k = if !live.is_empty() && rng.below(4) != 0 {
                    let i = rng.below(live.len());
                    live.swap_remove(i)
                } else {
                    rng.next_u32()
                };
                let (a, b) = m.del(&k.to_le_bytes());
                assert_eq!(a, b, "op={op} del diverged");
                m.assert_same(&format!("op={op} del"));
            }
        }
    }
    m.free();
}

/// row 60 — same pipeline, `keysize = 8`, `elemsize = 32`
#[test]
fn row60_binary_pipeline_keysize8() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0060);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(32, 8, 8, STBDS_HM_BINARY, bin_kind(8, 8, 24));
    let mut live: Vec<u64> = Vec::new();
    for op in 0..1500u64 {
        match rng.below(10) {
            0..=4 => {
                let k = rng.next_u64() % 4000;
                if !live.contains(&k) {
                    live.push(k);
                }
                let mut val = vec![0u8; 24];
                val[..8].copy_from_slice(&op.to_le_bytes());
                val[8..16].copy_from_slice(&(op * 7).to_le_bytes());
                val[16..].copy_from_slice(&(op ^ 0xdead).to_le_bytes());
                m.put(&k.to_le_bytes(), &val);
                m.assert_same(&format!("op={op} put"));
            }
            5..=7 => {
                let k = if !live.is_empty() && rng.below(2) == 0 {
                    live[rng.below(live.len())]
                } else {
                    rng.next_u64()
                };
                let (a, b) = m.geti_ts(&k.to_le_bytes());
                assert_eq!(a, b, "op={op} geti_ts diverged");
                m.assert_same(&format!("op={op} get"));
            }
            _ => {
                let k = if !live.is_empty() && rng.below(4) != 0 {
                    let i = rng.below(live.len());
                    live.swap_remove(i)
                } else {
                    rng.next_u64()
                };
                let (a, b) = m.del(&k.to_le_bytes());
                assert_eq!(a, b, "op={op} del diverged");
                m.assert_same(&format!("op={op} del"));
            }
        }
    }
    m.free();
}

// ---------------------------------------------------------------------------
// rows 61..65 — string maps with the implicit STBDS_SH_DEFAULT mode
// ---------------------------------------------------------------------------

/// row 61 — the table auto-created by `hmput_key(mode = 1)` gets `SH_DEFAULT`
#[test]
fn row61_string_default_mode() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4));
    m.put(&cstr(b"alpha"), &7u32.to_le_bytes());
    m.assert_same("first string put");
    let (s, _) = m.snap_pair();
    assert_eq!(
        s.table.as_ref().unwrap().string_mode,
        STBDS_SH_DEFAULT as u8
    );
    m.free();
}

/// row 62 — many distinct random string keys
#[test]
fn row62_string_default_many() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0062);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4));
    for i in 0..200u32 {
        let n = 1 + rng.below(24);
        let key = cstr(&rng.ascii(n));
        m.put(&key, &i.to_le_bytes());
        m.assert_same(&format!("string put #{i}"));
    }
    m.free();
}

/// row 63 — duplicate keys via `shputs` (writes `stbds_temp_key` back)
#[test]
fn row63_string_default_shputs_duplicates() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4));
    let keys: Vec<Vec<u8>> = (0..30).map(|i| cstr(format!("key_{i}").as_bytes())).collect();
    for (i, k) in keys.iter().enumerate() {
        m.puts_string(k, &(i as u32).to_le_bytes());
        m.assert_same(&format!("shputs #{i}"));
    }
    for round in 0..4u32 {
        for (i, k) in keys.iter().enumerate() {
            m.puts_string(k, &(1000 * round + i as u32).to_le_bytes());
            m.assert_same(&format!("shputs dup round={round} #{i}"));
        }
    }
    m.free();
}

/// row 64 — `hmget_key(mode = 1)` hits and misses
#[test]
fn row64_string_default_get() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0064);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4));
    let mut keys = Vec::new();
    for i in 0..150u32 {
        let k = cstr(format!("k{}", rng.next_u32()).as_bytes());
        keys.push(k.clone());
        m.put(&k, &i.to_le_bytes());
    }
    for k in &keys {
        let (a, b) = m.geti(k);
        assert_eq!(a, b, "hit diverged");
        assert!(a >= 0);
    }
    for _ in 0..300 {
        let k = cstr(format!("absent-{}", rng.next_u32()).as_bytes());
        let (a, b) = m.geti(&k);
        assert_eq!(a, b, "miss diverged");
        assert_eq!(a, -1);
    }
    m.assert_same("string gets");
    m.free();
}

/// row 65 — `hmdel_key(mode = 1)` on a `SH_DEFAULT` map (no strdup free)
#[test]
fn row65_string_default_del() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0065);
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4));
    let mut keys = Vec::new();
    for i in 0..120u32 {
        let k = cstr(format!("key{i}").as_bytes());
        keys.push(k.clone());
        m.put(&k, &i.to_le_bytes());
    }
    while !keys.is_empty() {
        let i = rng.below(keys.len());
        let k = keys.swap_remove(i);
        let (a, b) = m.del(&k);
        assert_eq!((a, b), (1, 1), "delete {k:?}");
        m.assert_same("string delete");
    }
    assert_eq!(m.len(), (0, 0));
    m.free();
}

// ---------------------------------------------------------------------------
// rows 66..70 — STBDS_SH_STRDUP
// ---------------------------------------------------------------------------

fn strdup_map() -> MapPair {
    MapPair::with_shmode(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4), STBDS_SH_STRDUP)
}

/// row 66 — `shmode_func(_, SH_STRDUP)` then inserts
#[test]
fn row66_strdup_basic() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = strdup_map();
    m.assert_same("fresh strdup map");
    let (s, _) = m.snap_pair();
    assert_eq!(s.table.as_ref().unwrap().string_mode, STBDS_SH_STRDUP as u8);
    for i in 0..10u32 {
        m.put(&cstr(format!("dup{i}").as_bytes()), &i.to_le_bytes());
        m.assert_same(&format!("strdup put #{i}"));
    }
    m.free();
}

/// row 67 — 200 random keys, growth
#[test]
fn row67_strdup_many() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0067);
    reseed(DEFAULT_SEED);
    let mut m = strdup_map();
    for i in 0..200u32 {
        let n = 1 + rng.below(40);
        m.put(&cstr(&rng.ascii(n)), &i.to_le_bytes());
        m.assert_same(&format!("strdup put #{i}"));
    }
    m.free();
}

/// row 68 — `hmdel_key(mode == 1)` frees the duplicated key; swap + re-lookup
#[test]
fn row68_strdup_del_mode1() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0068);
    reseed(DEFAULT_SEED);
    let mut m = strdup_map();
    let mut keys = Vec::new();
    for i in 0..120u32 {
        let k = cstr(format!("sd{i}").as_bytes());
        keys.push(k.clone());
        m.put(&k, &i.to_le_bytes());
    }
    m.assert_same("populate strdup");
    while !keys.is_empty() {
        let i = rng.below(keys.len());
        let k = keys.swap_remove(i);
        let (a, b) = m.del_mode(&k, STBDS_HM_STRING);
        assert_eq!((a, b), (1, 1));
        m.assert_same("strdup delete mode=1");
    }
    m.free();
}

/// row 69 — `hmdel_key(mode = 2)`: still "string" for hashing, but *not*
/// `== STBDS_HM_STRING`, so the key is NOT freed and the re-lookup of the moved
/// element goes through the raw-bytes path
#[test]
fn row69_strdup_del_mode2() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = strdup_map();
    let keys: Vec<Vec<u8>> = (0..6).map(|i| cstr(format!("m2_{i}").as_bytes())).collect();
    for (i, k) in keys.iter().enumerate() {
        m.put(k, &(i as u32).to_le_bytes());
    }
    m.assert_same("populate for mode=2 delete");
    // Delete the LAST element: `old_index == final_index`, so the divergent
    // re-lookup branch is not taken and the operation stays well-defined while
    // still exercising the `mode != STBDS_HM_STRING` no-free path.
    for i in (0..keys.len()).rev() {
        let (a, b) = m.del_mode(&keys[i], 2);
        assert_eq!((a, b), (1, 1), "mode=2 delete of #{i}");
        m.assert_same(&format!("mode=2 delete #{i}"));
    }
    assert_eq!(m.len(), (0, 0));
    m.free();
}

/// row 70 — `hmfree_func` sweeps and frees every duplicated key
#[test]
fn row70_strdup_free_sweep() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0070);
    for trial in 0..8 {
        reseed(DEFAULT_SEED + trial);
        let mut m = strdup_map();
        for i in 0..90u32 {
            let n = 1 + rng.below(30);
            m.put(&cstr(&rng.ascii(n)), &i.to_le_bytes());
        }
        m.assert_same("before free");
        m.free();
        assert!(m.c.is_null() && m.r.is_null());
    }
}

// ---------------------------------------------------------------------------
// rows 71..73 — STBDS_SH_ARENA
// ---------------------------------------------------------------------------

fn arena_map() -> MapPair {
    MapPair::with_shmode(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4), STBDS_SH_ARENA)
}

/// row 71 — `shmode_func(_, SH_ARENA)` then inserts
#[test]
fn row71_arena_basic() {
    let _g = serial();
    reseed(DEFAULT_SEED);
    let mut m = arena_map();
    let (s, _) = m.snap_pair();
    assert_eq!(s.table.as_ref().unwrap().string_mode, STBDS_SH_ARENA as u8);
    for i in 0..10u32 {
        m.put(&cstr(format!("ar{i}").as_bytes()), &i.to_le_bytes());
        m.assert_same(&format!("arena put #{i}"));
    }
    m.free();
}

/// row 72 — mixed key lengths incl. > 512 (arena over-sized block path)
#[test]
fn row72_arena_mixed_lengths() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0072);
    reseed(DEFAULT_SEED);
    let mut m = arena_map();
    for i in 0..200u32 {
        let n = match rng.below(8) {
            0 => 600 + rng.below(2000),
            1 => 400 + rng.below(200),
            _ => 1 + rng.below(40),
        };
        m.put(&cstr(&rng.ascii(n)), &i.to_le_bytes());
        m.assert_same(&format!("arena put #{i} (len {n})"));
    }
    m.free();
}

/// row 73 — deletes plus `hmfree_func` (which calls `strreset` on the arena)
#[test]
fn row73_arena_del_and_free() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0073);
    reseed(DEFAULT_SEED);
    let mut m = arena_map();
    let mut keys = Vec::new();
    for i in 0..120u32 {
        let n = 1 + rng.below(50);
        let k = cstr(&rng.ascii(n));
        keys.push(k.clone());
        m.put(&k, &i.to_le_bytes());
    }
    m.assert_same("populate arena map");
    for _ in 0..60 {
        let i = rng.below(keys.len());
        let k = keys.swap_remove(i);
        let (a, b) = m.del(&k);
        assert_eq!(a, b);
        m.assert_same("arena delete");
    }
    m.free();
}

/// row 74 — `STBDS_SH_NONE` + `mode = STBDS_HM_STRING`: the `switch` default arm
/// `memcpy`s `keysize` bytes of the *string body* into the element.  Exactly one
/// insert is well-defined (no `strcmp` against the bogus stored pointer runs),
/// which is what the C does, so that is what is compared.
#[test]
fn row74_shnone_string_mode_default_arm() {
    let _g = serial();
    for elemsize in [16usize, 24, 32] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::with_shmode(
            elemsize,
            8,
            8,
            STBDS_HM_STRING,
            KeyKind::RawPrefix { n: 8 },
            STBDS_SH_NONE,
        );
        let (s, _) = m.snap_pair();
        assert_eq!(s.table.as_ref().unwrap().string_mode, 0);
        // >= 8 bytes of body so the 8-byte memcpy stays inside the string
        m.put(&cstr(b"0123456789abcdef"), &1u32.to_le_bytes());
        m.assert_same(&format!("SH_NONE string put (elemsize={elemsize})"));
        let (s, _) = m.snap_pair();
        assert_eq!(
            s.elements[1],
            ElemSnap::Raw(b"01234567".to_vec()),
            "the default arm must memcpy the string body"
        );
        m.free();
    }
}

/// row 75 — `shmode_func` over the `elemsize` × `sh_mode` cross-product
#[test]
fn row75_shmode_func_cross_product() {
    let _g = serial();
    for elemsize in [8usize, 16, 24, 32] {
        for sh_mode in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            reseed(DEFAULT_SEED);
            let (lc, lr) = libs();
            unsafe {
                let c = (lc.shmode_func)(elemsize, sh_mode);
                let r = (lr.shmode_func)(elemsize, sh_mode);
                let kind = KeyKind::RawPrefix { n: elemsize };
                assert_eq!(
                    map_snap(c, elemsize, kind),
                    map_snap(r, elemsize, kind),
                    "shmode_func({elemsize}, {sh_mode})"
                );
                let s = map_snap(c, elemsize, kind);
                assert_eq!(s.length, 1);
                assert_eq!(s.table.as_ref().unwrap().string_mode, sh_mode as u8);
                assert_eq!(s.table.as_ref().unwrap().slot_count, 8);
                (lc.hmfree_func)((c as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (lr.hmfree_func)((r as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

/// rows 76..78 — full randomized pipelines for each string-arena mode
fn string_pipeline(sh: Option<c_int>, seed: u64, ops: u32, tag: &str) {
    let mut rng = Rng::new(seed);
    reseed(DEFAULT_SEED);
    let mut m = match sh {
        Some(mode) => MapPair::with_shmode(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4), mode),
        None => MapPair::new(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4)),
    };
    let mut live: Vec<Vec<u8>> = Vec::new();
    for op in 0..ops {
        match rng.below(10) {
            0..=4 => {
                let k = if !live.is_empty() && rng.below(3) == 0 {
                    live[rng.below(live.len())].clone()
                } else {
                    let n = 1 + rng.below(30);
                    let k = cstr(&rng.ascii(n));
                    live.push(k.clone());
                    k
                };
                m.put(&k, &op.to_le_bytes());
                m.assert_same(&format!("{tag} op={op} put"));
            }
            5..=7 => {
                let k = if !live.is_empty() && rng.below(2) == 0 {
                    live[rng.below(live.len())].clone()
                } else {
                    cstr(format!("absent{}", rng.next_u32()).as_bytes())
                };
                let (a, b) = m.geti(&k);
                assert_eq!(a, b, "{tag} op={op} geti diverged");
                m.assert_same(&format!("{tag} op={op} get"));
            }
            _ => {
                let k = if !live.is_empty() && rng.below(4) != 0 {
                    let i = rng.below(live.len());
                    live.swap_remove(i)
                } else {
                    cstr(format!("absent{}", rng.next_u32()).as_bytes())
                };
                let (a, b) = m.del_mode(&k, STBDS_HM_STRING);
                assert_eq!(a, b, "{tag} op={op} del diverged");
                m.assert_same(&format!("{tag} op={op} del"));
            }
        }
    }
    m.free();
}

/// row 76 — `SH_STRDUP` pipeline
#[test]
fn row76_pipeline_strdup() {
    let _g = serial();
    string_pipeline(Some(STBDS_SH_STRDUP), 0xB2_0076, 1000, "strdup");
}

/// row 77 — `SH_ARENA` pipeline
#[test]
fn row77_pipeline_arena() {
    let _g = serial();
    string_pipeline(Some(STBDS_SH_ARENA), 0xB2_0077, 1000, "arena");
}

/// row 78 — implicit `SH_DEFAULT` pipeline
#[test]
fn row78_pipeline_default() {
    let _g = serial();
    string_pipeline(None, 0xB2_0078, 1000, "default");
}

/// row 79 — `rand_seed` interaction: the per-table seed changes the probe order
#[test]
fn row79_seed_interaction() {
    let _g = serial();
    for &start in &[0usize, 1, DEFAULT_SEED, usize::MAX, 0x1234_5678_9abc_def0] {
        // binary map
        reseed(start);
        let mut rng = Rng::new(0xB2_0079 ^ start as u64);
        let mut m = MapPair::new(8, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        for i in 0..120u32 {
            m.put(&(rng.next_u32() % 500).to_le_bytes(), &i.to_le_bytes());
            m.assert_same(&format!("seed={start:#x} binary put #{i}"));
        }
        m.free();

        // string map, each arena mode
        for sh in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            reseed(start);
            let mut m = MapPair::with_shmode(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4), sh);
            for i in 0..120u32 {
                let n = 1 + rng.below(20);
                m.put(&cstr(&rng.ascii(n)), &i.to_le_bytes());
                m.assert_same(&format!("seed={start:#x} sh={sh} put #{i}"));
            }
            m.free();
        }
    }
    reseed(DEFAULT_SEED);
}

/// row 80 — `hmfree_func` for every table state × arena mode
#[test]
fn row80_hmfree_matrix() {
    let _g = serial();
    let (lc, lr) = libs();
    // (a) table == NULL, produced by hmput_default
    for elemsize in [8usize, 16, 32] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(elemsize, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        m.put_default();
        m.assert_same("hmput_default before free");
        m.free();
    }
    // (b) table == NULL, produced by hmget_key_ts(NULL, ...)
    for elemsize in [8usize, 16, 32] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::new(elemsize, 4, 4, STBDS_HM_BINARY, bin_kind(4, 4, 4));
        let _ = m.geti_ts(&1u32.to_le_bytes());
        m.free();
    }
    // (c) freshly created tables in every arena mode, never populated
    for sh in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        reseed(DEFAULT_SEED);
        unsafe {
            let c = (lc.shmode_func)(16, sh);
            let r = (lr.shmode_func)(16, sh);
            (lc.hmfree_func)((c as *mut u8).sub(16) as *mut c_void, 16);
            (lr.hmfree_func)((r as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
    // (d) populated tables in each arena mode
    let mut rng = Rng::new(0xB2_0080);
    for sh in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        reseed(DEFAULT_SEED);
        let mut m = MapPair::with_shmode(16, 8, 8, STBDS_HM_STRING, str_kind(8, 4), sh);
        for i in 0..60u32 {
            let n = 1 + rng.below(700);
            m.put(&cstr(&rng.ascii(n)), &i.to_le_bytes());
        }
        m.assert_same(&format!("populated sh={sh} before free"));
        m.free();
    }
    // (e) binary map with values, freed
    reseed(DEFAULT_SEED);
    let mut m = MapPair::new(16, 8, 8, STBDS_HM_BINARY, bin_kind(8, 8, 8));
    for i in 0..80u64 {
        m.put(&rng.next_u64().to_le_bytes(), &i.to_le_bytes());
    }
    m.free();
}
