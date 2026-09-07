//! Phase B rows 16–32 — hash map with binary keys (`STBDS_HM_BINARY`).

mod common;
use common::*;
use std::ffi::c_void;

fn cmp(c_map: *mut c_void, r_map: *mut c_void, es: usize, ctx: &str) {
    unsafe {
        let sc = snap_map(c_map, es, ElemFmt::Raw, false);
        let sr = snap_map(r_map, es, ElemFmt::Raw, false);
        assert_snap_eq(&sc, &sr, ctx);
    }
}

/// CONFIGS row 16 — bootstrap from NULL, one key, hit + miss.
#[test]
fn cfg_16_hmput_bootstrap_single() {
    run(1, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let key = [0x11u8, 0x22, 0x33, 0x44];
        let mut mc = hm_put(c, std::ptr::null_mut(), es, &key, ks, &[9, 9, 9, 9], HM_BINARY);
        let mut mr = hm_put(r, std::ptr::null_mut(), es, &key, ks, &[9, 9, 9, 9], HM_BINARY);
        cmp(mc, mr, es, "row16 after insert");

        let (m1, tc) = hm_get(c, mc, es, &key, ks, HM_BINARY);
        let (m2, tr) = hm_get(r, mr, es, &key, ks, HM_BINARY);
        mc = m1;
        mr = m2;
        assert_eq!(tc, tr, "row16 hit index");
        assert_eq!(tc, 0, "expected first element at index 0");

        let miss = [0xAAu8, 0xBB, 0xCC, 0xDD];
        let (m1, tc) = hm_get(c, mc, es, &miss, ks, HM_BINARY);
        let (m2, tr) = hm_get(r, mr, es, &miss, ks, HM_BINARY);
        mc = m1;
        mr = m2;
        assert_eq!(tc, tr, "row16 miss index");
        assert_eq!(tc, -1, "expected STBDS_INDEX_EMPTY for a missing key");
        cmp(mc, mr, es, "row16 after gets");

        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// CONFIGS row 17 — 1..40 random u32 keys (crosses 8→16→32→64 growth) + misses.
#[test]
fn cfg_17_hmput_growth_u32() {
    for n in 1..41usize {
        run(0x3141_5926, |c, r| unsafe {
            let (es, ks) = (8usize, 4usize);
            let mut rng = Rng::new(0x5eed_3000 + n as u64);
            let mut mc: *mut c_void = std::ptr::null_mut();
            let mut mr: *mut c_void = std::ptr::null_mut();
            let mut keys = Vec::new();
            for i in 0..n {
                let k = rng.next_u32().to_le_bytes();
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k);
                let pay = (i as u32).to_le_bytes();
                mc = hm_put(c, mc, es, &k, ks, &pay, HM_BINARY);
                mr = hm_put(r, mr, es, &k, ks, &pay, HM_BINARY);
                cmp(mc, mr, es, &format!("row17 n={} after insert #{}", n, i));
            }
            for k in &keys {
                let (m1, tc) = hm_get(c, mc, es, k, ks, HM_BINARY);
                let (m2, tr) = hm_get(r, mr, es, k, ks, HM_BINARY);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr, "row17 n={} get {:02x?}", n, k);
                assert!(tc >= 0, "row17 key {:02x?} should be present", k);
            }
            for _ in 0..40 {
                let k = rng.next_u32().to_le_bytes();
                if keys.contains(&k) {
                    continue;
                }
                let (m1, tc) = hm_get(c, mc, es, &k, ks, HM_BINARY);
                let (m2, tr) = hm_get(r, mr, es, &k, ks, HM_BINARY);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr, "row17 miss {:02x?}", k);
                assert_eq!(tc, -1);
            }
            cmp(mc, mr, es, &format!("row17 n={} final", n));
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

/// CONFIGS row 18 — `keysize=8`, `elemsize=16`, up to 64 keys.
#[test]
fn cfg_18_hmput_u64_keys() {
    for n in [1usize, 2, 5, 6, 7, 8, 12, 13, 24, 25, 48, 49, 64] {
        run(0x3141_5926, |c, r| unsafe {
            let (es, ks) = (16usize, 8usize);
            let mut rng = Rng::new(0x5eed_4000 + n as u64);
            let mut mc: *mut c_void = std::ptr::null_mut();
            let mut mr: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<[u8; 8]> = Vec::new();
            for i in 0..n {
                let k = rng.next_u64().to_le_bytes();
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k);
                let pay = (i as u64).to_le_bytes();
                mc = hm_put(c, mc, es, &k, ks, &pay, HM_BINARY);
                mr = hm_put(r, mr, es, &k, ks, &pay, HM_BINARY);
                cmp(mc, mr, es, &format!("row18 n={} insert #{}", n, i));
            }
            for k in &keys {
                let (m1, tc) = hm_get(c, mc, es, k, ks, HM_BINARY);
                let (m2, tr) = hm_get(r, mr, es, k, ks, HM_BINARY);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr);
            }
            cmp(mc, mr, es, &format!("row18 n={} final", n));
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

/// CONFIGS row 19 — `keysize=1`, all 256 byte values (heavy probing).
#[test]
fn cfg_19_hmput_byte_keys_all_256() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (4usize, 1usize);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        for b in 0..256usize {
            let k = [b as u8];
            let pay = [(b >> 8) as u8, b as u8, 0x5A];
            mc = hm_put(c, mc, es, &k, ks, &pay, HM_BINARY);
            mr = hm_put(r, mr, es, &k, ks, &pay, HM_BINARY);
            cmp(mc, mr, es, &format!("row19 insert byte {}", b));
        }
        for b in 0..256usize {
            let (m1, tc) = hm_get(c, mc, es, &[b as u8], ks, HM_BINARY);
            let (m2, tr) = hm_get(r, mr, es, &[b as u8], ks, HM_BINARY);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "row19 get byte {}", b);
            assert!(tc >= 0);
        }
        cmp(mc, mr, es, "row19 final");
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// CONFIGS row 20 — `keysize=16`, `elemsize=24`.
#[test]
fn cfg_20_hmput_wide_keys() {
    for n in [1usize, 6, 7, 8, 16, 32] {
        run(0x3141_5926, |c, r| unsafe {
            let (es, ks) = (24usize, 16usize);
            let mut rng = Rng::new(0x5eed_5000 + n as u64);
            let mut mc: *mut c_void = std::ptr::null_mut();
            let mut mr: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..n {
                let k = rng.bytes(ks);
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k.clone());
                let pay = rng.bytes(es - ks);
                mc = hm_put(c, mc, es, &k, ks, &pay, HM_BINARY);
                mr = hm_put(r, mr, es, &k, ks, &pay, HM_BINARY);
                cmp(mc, mr, es, &format!("row20 n={} insert #{}", n, i));
            }
            for k in &keys {
                let (m1, tc) = hm_get(c, mc, es, k, ks, HM_BINARY);
                let (m2, tr) = hm_get(r, mr, es, k, ks, HM_BINARY);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr);
            }
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

/// CONFIGS row 21 — odd `keysize` 3/5/7 (`memcmp` + hash tail).
#[test]
fn cfg_21_hmput_odd_keysizes() {
    for ks in [3usize, 5, 7] {
        for n in [1usize, 7, 8, 20, 32] {
            run(0x3141_5926, |c, r| unsafe {
                let es = ks + 5;
                let mut rng = Rng::new(0x5eed_6000 + (ks * 100 + n) as u64);
                let mut mc: *mut c_void = std::ptr::null_mut();
                let mut mr: *mut c_void = std::ptr::null_mut();
                let mut keys: Vec<Vec<u8>> = Vec::new();
                for i in 0..n {
                    let k = rng.bytes(ks);
                    if keys.contains(&k) {
                        continue;
                    }
                    keys.push(k.clone());
                    let pay = rng.bytes(es - ks);
                    mc = hm_put(c, mc, es, &k, ks, &pay, HM_BINARY);
                    mr = hm_put(r, mr, es, &k, ks, &pay, HM_BINARY);
                    cmp(mc, mr, es, &format!("row21 ks={} n={} insert #{}", ks, n, i));
                }
                for k in &keys {
                    let (m1, tc) = hm_get(c, mc, es, k, ks, HM_BINARY);
                    let (m2, tr) = hm_get(r, mr, es, k, ks, HM_BINARY);
                    mc = m1;
                    mr = m2;
                    assert_eq!(tc, tr);
                }
                hm_free(c, mc, es);
                hm_free(r, mr, es);
            });
        }
    }
}

/// CONFIGS row 22 — re-put an existing key (update path, `length` unchanged).
#[test]
fn cfg_22_hmput_update_existing() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut rng = Rng::new(0x5eed_7000);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        let keys: Vec<[u8; 4]> = (0..20).map(|_| rng.next_u32().to_le_bytes()).collect();
        for (i, k) in keys.iter().enumerate() {
            let pay = (i as u32).to_le_bytes();
            mc = hm_put(c, mc, es, k, ks, &pay, HM_BINARY);
            mr = hm_put(r, mr, es, k, ks, &pay, HM_BINARY);
        }
        cmp(mc, mr, es, "row22 baseline");
        // update every key twice with fresh payloads
        for round in 0..2 {
            for (i, k) in keys.iter().enumerate() {
                let pay = ((i as u32) ^ (0x1000 * (round + 1))).to_le_bytes();
                mc = hm_put(c, mc, es, k, ks, &pay, HM_BINARY);
                mr = hm_put(r, mr, es, k, ks, &pay, HM_BINARY);
                cmp(mc, mr, es, &format!("row22 round {} update #{}", round, i));
            }
        }
        assert_eq!(hm_len(mc, es), hm_len(mr, es));
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// CONFIGS row 23 — the `_ts` lookup variant (index through `*temp`).
#[test]
fn cfg_23_hmget_key_ts() {
    for n in [1usize, 8, 20, 40] {
        run(0x3141_5926, |c, r| unsafe {
            let (es, ks) = (8usize, 4usize);
            let mut rng = Rng::new(0x5eed_8000 + n as u64);
            let mut mc: *mut c_void = std::ptr::null_mut();
            let mut mr: *mut c_void = std::ptr::null_mut();
            let mut keys: Vec<[u8; 4]> = Vec::new();
            for i in 0..n {
                let k = rng.next_u32().to_le_bytes();
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k);
                let pay = (i as u32).to_le_bytes();
                mc = hm_put(c, mc, es, &k, ks, &pay, HM_BINARY);
                mr = hm_put(r, mr, es, &k, ks, &pay, HM_BINARY);
            }
            for k in &keys {
                let (m1, tc) = hm_get_ts(c, mc, es, k, ks, HM_BINARY);
                let (m2, tr) = hm_get_ts(r, mr, es, k, ks, HM_BINARY);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr, "row23 _ts hit {:02x?}", k);
                assert!(tc >= 0);
            }
            for _ in 0..30 {
                let k = rng.next_u32().to_le_bytes();
                if keys.contains(&k) {
                    continue;
                }
                let (m1, tc) = hm_get_ts(c, mc, es, &k, ks, HM_BINARY);
                let (m2, tr) = hm_get_ts(r, mr, es, &k, ks, HM_BINARY);
                mc = m1;
                mr = m2;
                assert_eq!(tc, tr, "row23 _ts miss");
                assert_eq!(tc, -1);
            }
            cmp(mc, mr, es, "row23 final");
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

/// CONFIGS row 24 — `hmput_default` then `hmput_key` (slot `[-1]` preserved).
#[test]
fn cfg_24_hmput_default_then_put() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut mc = (c.hmput_default)(std::ptr::null_mut(), es);
        let mut mr = (r.hmput_default)(std::ptr::null_mut(), es);
        cmp(mc, mr, es, "row24 after hmput_default");

        // write the default value into slot [-1] (stbds_hmdefault)
        let dflt = [0xDEu8, 0xAD, 0xBE, 0xEF];
        std::ptr::copy_nonoverlapping(dflt.as_ptr(), (mc as *mut u8).sub(es).add(ks), 4);
        std::ptr::copy_nonoverlapping(dflt.as_ptr(), (mr as *mut u8).sub(es).add(ks), 4);

        let mut rng = Rng::new(0x5eed_9000);
        for i in 0..25 {
            let k = rng.next_u32().to_le_bytes();
            let pay = (i as u32).to_le_bytes();
            mc = hm_put(c, mc, es, &k, ks, &pay, HM_BINARY);
            mr = hm_put(r, mr, es, &k, ks, &pay, HM_BINARY);
            cmp(mc, mr, es, &format!("row24 insert #{}", i));
            // the default element must survive every rehash/realloc
            let dc = std::slice::from_raw_parts((mc as *const u8).sub(es), es);
            let dr = std::slice::from_raw_parts((mr as *const u8).sub(es), es);
            assert_eq!(dc, dr, "row24 default slot diverged at #{}", i);
        }
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// CONFIGS row 25 — `hmput_default` on a non-empty map (no-op branch).
#[test]
fn cfg_25_hmput_default_noop() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut mc = hm_put(c, std::ptr::null_mut(), es, &[1, 2, 3, 4], ks, &[0; 4], HM_BINARY);
        let mut mr = hm_put(r, std::ptr::null_mut(), es, &[1, 2, 3, 4], ks, &[0; 4], HM_BINARY);
        for _ in 0..5 {
            let nc = (c.hmput_default)(mc, es);
            let nr = (r.hmput_default)(mr, es);
            assert_eq!(nc == mc, nr == mr, "row25 identity mismatch");
            assert!(nc == mc, "expected the no-op branch");
            mc = nc;
            mr = nr;
            cmp(mc, mr, es, "row25 no-op");
        }
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// CONFIGS row 26 — delete the last element (`old_index == final_index`).
#[test]
fn cfg_26_hmdel_last_element() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut rng = Rng::new(0x5eed_a000);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        let mut keys: Vec<[u8; 4]> = Vec::new();
        for i in 0..12 {
            let k = rng.next_u32().to_le_bytes();
            keys.push(k);
            mc = hm_put(c, mc, es, &k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
            mr = hm_put(r, mr, es, &k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
        }
        // pop from the back: each delete targets the final element
        for i in (0..12).rev() {
            let (m1, tc) = hm_del(c, mc, es, &keys[i], ks, 0, HM_BINARY);
            let (m2, tr) = hm_del(r, mr, es, &keys[i], ks, 0, HM_BINARY);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "row26 del temp #{}", i);
            assert_eq!(tc, 1, "row26 expected a successful delete");
            cmp(mc, mr, es, &format!("row26 after del #{}", i));
        }
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// CONFIGS row 27 — delete a middle element (swap-with-last + re-find + patch).
#[test]
fn cfg_27_hmdel_middle_element() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut rng = Rng::new(0x5eed_b000);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        let mut keys: Vec<[u8; 4]> = Vec::new();
        for i in 0..20 {
            let k = rng.next_u32().to_le_bytes();
            keys.push(k);
            mc = hm_put(c, mc, es, &k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
            mr = hm_put(r, mr, es, &k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
        }
        // always delete the front-most surviving key => never the final element
        for i in 0..19 {
            let (m1, tc) = hm_del(c, mc, es, &keys[i], ks, 0, HM_BINARY);
            let (m2, tr) = hm_del(r, mr, es, &keys[i], ks, 0, HM_BINARY);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "row27 del temp #{}", i);
            cmp(mc, mr, es, &format!("row27 after del #{}", i));
            // every remaining key must still be findable, identically
            for k in &keys[(i + 1)..] {
                let (m3, gc) = hm_get(c, mc, es, k, ks, HM_BINARY);
                let (m4, gr) = hm_get(r, mr, es, k, ks, HM_BINARY);
                mc = m3;
                mr = m4;
                assert_eq!(gc, gr, "row27 get after del #{} key {:02x?}", i, k);
                assert!(gc >= 0, "row27 key {:02x?} lost after delete", k);
            }
        }
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// CONFIGS rows 28 + 30 — randomized 200-op churn (drives shrink + tombstone reuse).
#[test]
fn cfg_28_30_random_churn_binary() {
    for trial in 0..8u64 {
        run(0x3141_5926, |c, r| unsafe {
            let (es, ks) = (8usize, 4usize);
            let mut rng = Rng::new(0x5eed_c000 + trial);
            let mut mc: *mut c_void = std::ptr::null_mut();
            let mut mr: *mut c_void = std::ptr::null_mut();
            let mut live: Vec<[u8; 4]> = Vec::new();
            let pool: Vec<[u8; 4]> = (0..60u32).map(|i| i.wrapping_mul(0x9E37_79B9).to_le_bytes()).collect();

            for op in 0..200usize {
                let choice = rng.below(100);
                if choice < 50 || live.is_empty() {
                    let k = pool[rng.below(pool.len())];
                    let pay = rng.next_u32().to_le_bytes();
                    mc = hm_put(c, mc, es, &k, ks, &pay, HM_BINARY);
                    mr = hm_put(r, mr, es, &k, ks, &pay, HM_BINARY);
                    if !live.contains(&k) {
                        live.push(k);
                    }
                } else if choice < 85 {
                    let idx = rng.below(live.len());
                    let k = live[idx];
                    let (m1, tc) = hm_del(c, mc, es, &k, ks, 0, HM_BINARY);
                    let (m2, tr) = hm_del(r, mr, es, &k, ks, 0, HM_BINARY);
                    mc = m1;
                    mr = m2;
                    assert_eq!(tc, tr, "trial {} op {} del temp", trial, op);
                    live.remove(idx);
                } else {
                    let k = pool[rng.below(pool.len())];
                    let (m1, tc) = hm_get(c, mc, es, &k, ks, HM_BINARY);
                    let (m2, tr) = hm_get(r, mr, es, &k, ks, HM_BINARY);
                    mc = m1;
                    mr = m2;
                    assert_eq!(tc, tr, "trial {} op {} get temp", trial, op);
                    assert_eq!(tc >= 0, live.contains(&k), "presence mismatch vs model");
                }
                cmp(mc, mr, es, &format!("churn trial {} op {}", trial, op));
            }
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

/// CONFIGS row 29 — delete-heavy sequence crossing `tombstone_count_threshold`.
#[test]
fn cfg_29_tombstone_rebuild() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (8usize, 4usize);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        // Fill to a large table, then add/remove one key at a time so
        // tombstone_count climbs without used_count dropping below the shrink
        // threshold.
        let keys: Vec<[u8; 4]> = (0..200u32).map(|i| i.wrapping_mul(2_654_435_761).to_le_bytes()).collect();
        for (i, k) in keys.iter().enumerate().take(120) {
            mc = hm_put(c, mc, es, k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
            mr = hm_put(r, mr, es, k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
        }
        cmp(mc, mr, es, "row29 filled");
        for round in 0..80usize {
            let victim = keys[round % 120];
            let (m1, tc) = hm_del(c, mc, es, &victim, ks, 0, HM_BINARY);
            let (m2, tr) = hm_del(r, mr, es, &victim, ks, 0, HM_BINARY);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "row29 del round {}", round);
            cmp(mc, mr, es, &format!("row29 after del round {}", round));
            mc = hm_put(c, mc, es, &victim, ks, &[0xEE; 4], HM_BINARY);
            mr = hm_put(r, mr, es, &victim, ks, &[0xEE; 4], HM_BINARY);
            cmp(mc, mr, es, &format!("row29 after reinsert round {}", round));
        }
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// CONFIGS row 31 — non-zero `keyoffset` in `stbds_hmdel_key`.
///
/// Models `struct { int pad; int key; }` — the delete path is the only one that
/// takes a `keyoffset` (`hmput`/`hmget` hard-code 0), so keys are placed at
/// offset 0 for insertion and the *same* offset is used for deletion to keep
/// the two consistent; the non-zero case is driven by a layout where the key
/// sits at offset 4 and `hmput_key` is called with a pre-shifted element.
#[test]
fn cfg_31_hmdel_keyoffset() {
    run(0x3141_5926, |c, r| unsafe {
        let (es, ks) = (16usize, 4usize);
        // Insert with hmput_key (writes the key at offset 0) then also mirror
        // the key at offset 8, and delete using keyoffset = 8.
        let mut rng = Rng::new(0x5eed_d000);
        let mut mc: *mut c_void = std::ptr::null_mut();
        let mut mr: *mut c_void = std::ptr::null_mut();
        let mut keys: Vec<[u8; 4]> = Vec::new();
        for i in 0..24 {
            let k = rng.next_u32().to_le_bytes();
            if keys.contains(&k) {
                continue;
            }
            keys.push(k);
            let mut pay = vec![0u8; es - ks];
            // mirror the key at element offset 8 => payload offset 4
            pay[4..8].copy_from_slice(&k);
            pay[0..4].copy_from_slice(&(i as u32).to_le_bytes());
            mc = hm_put(c, mc, es, &k, ks, &pay, HM_BINARY);
            mr = hm_put(r, mr, es, &k, ks, &pay, HM_BINARY);
            cmp(mc, mr, es, &format!("row31 insert #{}", i));
        }
        for (i, k) in keys.iter().enumerate() {
            let (m1, tc) = hm_del(c, mc, es, k, ks, 8, HM_BINARY);
            let (m2, tr) = hm_del(r, mr, es, k, ks, 8, HM_BINARY);
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr, "row31 del#{} keyoffset=8", i);
            cmp(mc, mr, es, &format!("row31 after del #{}", i));
        }
        hm_free(c, mc, es);
        hm_free(r, mr, es);
    });
}

/// CONFIGS row 32 — free a binary map with a live table (no key frees).
#[test]
fn cfg_32_hmfree_binary() {
    run(0x3141_5926, |c, r| unsafe {
        for n in [0usize, 1, 8, 40] {
            let (es, ks) = (8usize, 4usize);
            let mut rng = Rng::new(0x5eed_e000 + n as u64);
            let mut mc: *mut c_void = std::ptr::null_mut();
            let mut mr: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                let k = rng.next_u32().to_le_bytes();
                mc = hm_put(c, mc, es, &k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
                mr = hm_put(r, mr, es, &k, ks, &(i as u32).to_le_bytes(), HM_BINARY);
            }
            if n > 0 {
                let tc = (*header_of(mc, es)).hash_table;
                let tr = (*header_of(mr, es)).hash_table;
                assert_eq!(tc.is_null(), tr.is_null());
                assert_eq!(
                    (*(tc as *mut HashIndex)).string.mode,
                    (*(tr as *mut HashIndex)).string.mode,
                    "binary map string.mode must be 0 in both"
                );
                assert_eq!((*(tc as *mut HashIndex)).string.mode, 0);
            }
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        }
    });
}
