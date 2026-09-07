//! Phase D catch-all: long randomized differential runs that mix every axis
//! from `CONFIGS.md` at once, plus explicit coverage assertions for the few
//! branches that only a specific state sequence can reach.

mod common;
use common::*;

/// Mixed binary workload across many (elemsize, keysize) shapes and modes.
#[test]
fn d1_fuzz_binary_mixed() {
    let s = session(0x31415926);
    let shapes: &[(usize, usize)] = &[
        (16, 4),
        (16, 8),
        (16, 16),
        (24, 8),
        (24, 24),
        (32, 8),
        (8, 8),
        (8, 4),
        (4, 4),
        (2, 2),
        (1, 1),
        (40, 12),
    ];
    for (si, (elemsize, keysize)) in shapes.iter().enumerate() {
        for mode in [0i32, -1, i32::MIN] {
            let seed = 0xD1000u64
                .wrapping_add((si as u64) << 8)
                .wrapping_add(mode as i64 as u64);
            let mut d = Driver::lazy(&s, *elemsize, *keysize, KeyKind::Binary);
            let mut rng = Rng::new(seed);
            let mut live: Vec<Vec<u8>> = Vec::new();
            // Small key space => frequent duplicates, collisions, tombstone reuse.
            let space = 1usize << (4 * (*keysize).min(2));
            for op in 0..1200usize {
                let ctx = format!("D1 s={si} e={elemsize} k={keysize} m={mode} op#{op}");
                match rng.below(12) {
                    0..=5 => {
                        let mut k = vec![0u8; *keysize];
                        let v = rng.below(space.max(2));
                        for (i, b) in k.iter_mut().enumerate() {
                            *b = v.checked_shr(8 * i as u32).unwrap_or(0) as u8;
                        }
                        d.hmput(&ctx, &k, &rng.bytes(*elemsize), mode);
                        if !live.contains(&k) {
                            live.push(k);
                        }
                    }
                    6..=8 => {
                        let k = if live.is_empty() || rng.below(4) == 0 {
                            rng.bytes(*keysize)
                        } else {
                            live[rng.below(live.len())].clone()
                        };
                        if rng.below(2) == 0 {
                            d.hmgeti(&ctx, &k, mode);
                        } else {
                            d.hmgeti_ts(&ctx, &k, mode);
                        }
                    }
                    9 => {
                        d.hmdefault(&ctx, &rng.bytes(*elemsize));
                    }
                    _ => {
                        if !live.is_empty() {
                            let i = rng.below(live.len());
                            let k = live.remove(i);
                            d.hmdel(&ctx, &k, 0, mode);
                        } else {
                            d.hmdel(&ctx, &rng.bytes(*keysize), 0, mode);
                        }
                    }
                }
                d.check(&ctx);
            }
            for (i, k) in live.iter().enumerate() {
                let ctx = format!("D1 s={si} m={mode} final#{i}");
                assert!(d.hmgeti(&ctx, k, mode) >= 0, "{ctx}: live key vanished");
                d.check(&ctx);
            }
            d.free();
        }
    }
}

/// Mixed string workload across every `string.mode` that stores a real pointer.
#[test]
fn d2_fuzz_string_mixed() {
    let s = session(0x31415926);
    for table_mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for elemsize in [16usize, 24, 32] {
            for op_mode in [1i32, 2, 9] {
                let seed = 0xD2000u64
                    .wrapping_add((table_mode as u64) << 16)
                    .wrapping_add((elemsize as u64) << 8)
                    .wrapping_add(op_mode as u64);
                let mut d = Driver::shmode(&s, elemsize, 8, KeyKind::StringAt(0), table_mode);
                let mut rng = Rng::new(seed);
                let mut live: Vec<Vec<u8>> = Vec::new();
                for op in 0..700usize {
                    let ctx = format!("D2 t={table_mode} e={elemsize} m={op_mode} op#{op}");
                    match rng.below(12) {
                        0..=5 => {
                            // Mix of tiny, medium and long keys from a small space.
                            let k = match rng.below(3) {
                                0 => format!("{}", rng.below(60)).into_bytes(),
                                1 => format!("key-{}", rng.below(120)).into_bytes(),
                                _ => {
                                    let n = rng.below(80);
                                    let mut v = vec![b'L'; rng.range(1, 200)];
                                    v.extend_from_slice(format!("{n}").as_bytes());
                                    v
                                }
                            };
                            d.shput(&ctx, &k, &rng.bytes(elemsize), op_mode);
                            if !live.contains(&k) {
                                live.push(k);
                            }
                        }
                        6..=8 => {
                            let k = if live.is_empty() || rng.below(4) == 0 {
                                format!("~miss~{}", rng.below(500)).into_bytes()
                            } else {
                                live[rng.below(live.len())].clone()
                            };
                            if rng.below(2) == 0 {
                                d.shgeti(&ctx, &k, op_mode);
                            } else {
                                d.shgeti_ts(&ctx, &k, op_mode);
                            }
                        }
                        9 => {
                            d.hmdefault(&ctx, &rng.bytes(elemsize));
                        }
                        _ => {
                            if !live.is_empty() {
                                let i = rng.below(live.len());
                                let k = live.remove(i);
                                // `hmdel_key` tests `mode == STBDS_HM_STRING`
                                // EXACTLY (lib.c:836, :842).  With `mode` outside
                                // {0,1} and `old_index != final_index` the C's
                                // re-lookup hashes the key POINTER BYTES, fails,
                                // and trips `assert(slot >= 0)` at lib.c:846 -- a
                                // real, reachable abort covered by
                                // `x1_assert_slot_ge_zero_aborts_identically` in
                                // phase_c_asserts.rs.  Random fuzzing must stay
                                // out of that state, so the delete mode is pinned
                                // to 1 here.
                                d.shdel(&ctx, &k, 0, 1);
                            }
                        }
                    }
                    d.check(&ctx);
                }
                for (i, k) in live.iter().enumerate() {
                    let ctx = format!("D2 t={table_mode} final#{i}");
                    assert!(d.shgeti(&ctx, k, op_mode) >= 0, "{ctx}: live key vanished");
                    d.check(&ctx);
                }
                d.free();
            }
        }
    }
}

/// Coverage probes: the branches that need a specific state sequence.
#[test]
fn d3_branch_coverage_probes() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    let keysize = 8usize;

    // (a) `hmput_key` TOMBSTONE REUSE (lib.c:766-769) and
    // (b) `make_hash_index` rehash-on-grow (`ot != NULL`).
    let mut tombstone_reuse = 0usize;
    let mut grows = 0usize;
    {
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0xD3);
        let mut live: Vec<Vec<u8>> = Vec::new();
        for op in 0..3000usize {
            let ctx = format!("D3a op#{op}");
            let before = d.snap_c().table;
            if rng.below(2) == 0 || live.is_empty() {
                let k = rng.bytes(keysize);
                d.hmput(&ctx, &k, &rng.bytes(elemsize), HM_BINARY);
                if !live.contains(&k) {
                    live.push(k);
                }
                if let (Some(b), Some(a)) = (before.as_ref(), d.snap_c().table.as_ref()) {
                    if a.tombstone_count + 1 == b.tombstone_count
                        && a.used_count == b.used_count + 1
                    {
                        tombstone_reuse += 1;
                    }
                    if a.slot_count == b.slot_count * 2 {
                        grows += 1;
                    }
                }
            } else {
                let i = rng.below(live.len());
                let k = live.remove(i);
                d.hmdel(&ctx, &k, 0, HM_BINARY);
            }
            d.check(&ctx);
        }
        d.free();
    }
    assert!(
        tombstone_reuse > 0,
        "D3: the hmput_key tombstone-reuse branch (lib.c:766) was never taken"
    );
    assert!(
        grows > 0,
        "D3: the make_hash_index rehash-on-grow path (ot != NULL) was never taken"
    );

    // (c) the WRAPPED bucket scan (`for i = 0; i < limit`) in both
    //     `hm_find_slot` and `hmput_key`: needs completely full buckets.
    {
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0xD3C);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..600usize {
            let k = rng.bytes(keysize);
            if keys.contains(&k) {
                continue;
            }
            d.hmput(&format!("D3c put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
            keys.push(k);
        }
        let t = d.snap_c().table.unwrap();
        let full_buckets = t
            .hashes
            .chunks(8)
            .filter(|b| b.iter().all(|h| *h != 0))
            .count();
        assert!(
            full_buckets > 0,
            "D3c: no completely full bucket -> the wrapped scan may be unreachable"
        );
        for (i, k) in keys.iter().enumerate() {
            let ctx = format!("D3c get#{i}");
            assert!(d.hmgeti(&ctx, k, HM_BINARY) >= 0, "{ctx}");
            d.check(&ctx);
        }
        d.check("D3c final");
        d.free();
    }

    // (d) the `hash < 2 -> hash += 2` fix-up (lib.c:596, :719).
    {
        s.seed(7);
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(1);
        let k0 = 0u64.to_ne_bytes().to_vec();
        d.hmput("D3d bootstrap", &k0, &rng.bytes(elemsize), HM_BINARY);
        let table_seed = d.snap_c().table.unwrap().seed;
        let mut found = None;
        for v in 0u64..2_000_000 {
            let k = v.to_ne_bytes();
            let p = k.as_ptr() as *mut std::ffi::c_void;
            let h = unsafe { (s.c.hash_bytes)(p, keysize, table_seed) };
            let hr = unsafe { (s.r.hash_bytes)(p, keysize, table_seed) };
            assert_eq!(h, hr, "D3d: hash_bytes diverged for {v}");
            if h < 2 {
                found = Some(v);
                break;
            }
        }
        if let Some(v) = found {
            let k = v.to_ne_bytes().to_vec();
            let ctx = format!("D3d low-hash key {v}");
            d.hmput(&ctx, &k, &rng.bytes(elemsize), HM_BINARY);
            d.check(&ctx);
            assert!(d.hmgeti(&ctx, &k, HM_BINARY) >= 0, "{ctx}: must be found");
            d.check(&ctx);
            assert_eq!(d.hmdel(&ctx, &k, 0, HM_BINARY), 1, "{ctx}: must delete");
            d.check(&ctx);
        }
        // Not finding one in 2e6 tries is fine: the 2e6 differential hash
        // comparisons above still ran.
        d.free();
    }
}
