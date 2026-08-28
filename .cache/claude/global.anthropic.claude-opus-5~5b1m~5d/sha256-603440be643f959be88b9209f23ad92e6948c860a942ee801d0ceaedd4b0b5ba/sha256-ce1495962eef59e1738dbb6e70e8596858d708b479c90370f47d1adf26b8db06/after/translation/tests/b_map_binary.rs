//! Phase B — CONFIGS.md rows C35..C50: the hash map with **binary** keys,
//! driven through the low-level `stbds_hm*_key` entry points exactly as
//! stb_ds.h's `hmput`/`hmgeti`/`hmdel`/`hmdefault`/`hmfree` macros do.
//!
//! After every single operation the complete observable state is compared:
//! header (`length`/`capacity`/`temp`), the whole `stbds_hash_index`
//! (`slot_count`, `used_count`, all three thresholds, `tombstone_count`, `seed`,
//! `slot_count_log2`, the embedded arena), every bucket's `hash[8]`/`index[8]`,
//! and every live element's bytes.

mod common;
use common::*;

/// (elemsize, keysize) pairs mirroring realistic `struct { K key; V value; }`.
const GEOM: &[(usize, usize)] = &[
    (8, 4),   // struct { int key; int value; }
    (8, 8),   // struct { size_t key; }  / { void* key; }
    (16, 4),
    (16, 8),
    (20, 8),  // struct { int key[2]; int b,c,d; }
    (24, 16),
    (32, 8),
    (64, 16),
    (4, 4),
    (2, 2),
    (1, 1),
];

fn keys_for(ks: usize, n: usize) -> Vec<Vec<u8>> {
    (0..n).map(|i| bin_key(i as u64 * 0x9E37_79B9 + 1, ks)).collect()
}

fn drivers(es: usize, ks: usize, mode: i32) -> (Drv, Drv) {
    let (c, r) = both();
    (Drv::empty(c, es, ks, mode), Drv::empty(r, es, ks, mode))
}

// ---------------------------------------------------------------------------
// C35 — first insert into a NULL map
// ---------------------------------------------------------------------------
#[test]
fn c35_first_insert_from_null() {
    let _g = lock();
    for &(es, ks) in GEOM {
        sync_seed(0x1111);
        let keys = keys_for(ks, 1);
        let (cd, rd) = drivers(es, ks, HM_BINARY);
        run_ops(
            &format!("first-insert es={es} ks={ks}"),
            cd,
            rd,
            &keys,
            &[Op::Len, Op::Put(0, 0xA0), Op::Len, Op::Get(0), Op::GetTs(0)],
        );
    }
}

// ---------------------------------------------------------------------------
// C36/C37 — 1..64 distinct keys: crosses used_count_threshold at
//           slot_count 8→16→32→64→128 (A1, A2)
// ---------------------------------------------------------------------------
#[test]
fn c36_c37_growth_chain() {
    let _g = lock();
    for &(es, ks) in GEOM {
        let n = if ks == 1 { 60 } else { 60 };
        for &seed in &[0usize, 1, DEFAULT_SEED, 0xdead_beef] {
            sync_seed(seed);
            let keys = keys_for(ks, n);
            let (cd, rd) = drivers(es, ks, HM_BINARY);
            let mut ops = Vec::new();
            for i in 0..n {
                ops.push(Op::Put(i, (i as u8).wrapping_mul(13)));
                ops.push(Op::Len);
                // look every key up again as we go
                ops.push(Op::Get(i));
                if i > 0 {
                    ops.push(Op::Get(i / 2));
                    ops.push(Op::GetTs(0));
                }
            }
            run_ops(
                &format!("growth es={es} ks={ks} seed={seed:#x}"),
                cd,
                rd,
                &keys,
                &ops,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C38/C39 — re-put of an existing key (first scan hit, and the wrap-around
//           second scan where the C deliberately skips `temp_key`)
// ---------------------------------------------------------------------------
#[test]
fn c38_c39_reput_existing_key() {
    let _g = lock();
    // Many seeds => the probe position `pos & 7` varies, so both the
    // `for (i = pos&7 .. 7)` scan and the `for (i = 0 .. pos&7)` scan are hit.
    for seed in 0..48usize {
        for &(es, ks) in &[(8usize, 4usize), (16, 8), (24, 16)] {
            sync_seed(seed * 0x1000_0001 + 7);
            let keys = keys_for(ks, 30);
            let (cd, rd) = drivers(es, ks, HM_BINARY);
            let mut ops = Vec::new();
            for i in 0..30 {
                ops.push(Op::Put(i, i as u8));
            }
            // re-put every key twice with new values
            for round in 0..2u8 {
                for i in 0..30 {
                    ops.push(Op::Put(i, 0x40 + round * 0x20 + i as u8));
                    ops.push(Op::Get(i));
                }
            }
            run_ops(
                &format!("reput seed={seed} es={es} ks={ks}"),
                cd,
                rd,
                &keys,
                &ops,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C40/C41 — hmget_key_ts and hmget_key for absent / present keys and for the
//           NULL-map and NULL-table states (R8..R11)
// ---------------------------------------------------------------------------
#[test]
fn c40_c41_lookup_states() {
    let _g = lock();
    for &(es, ks) in GEOM {
        sync_seed(0x2222);
        let keys = keys_for(ks, 40);
        let (cd, rd) = drivers(es, ks, HM_BINARY);
        let mut ops = vec![
            // absent lookup on a NULL map (allocates the sentinel element)
            Op::GetTs(0),
            Op::Get(1),
            Op::Len,
        ];
        for i in 0..20 {
            ops.push(Op::Put(i, i as u8));
        }
        for i in 0..40 {
            ops.push(Op::Get(i)); // 0..19 present, 20..39 absent
            ops.push(Op::GetTs(i));
        }
        run_ops(&format!("lookup es={es} ks={ks}"), cd, rd, &keys, &ops);
    }
}

// ---------------------------------------------------------------------------
// C42 — delete the LAST element (old_index == final_index, no relocation)
// C43 — delete a middle/first element (relocation memmove + index patch, A5/A6)
// ---------------------------------------------------------------------------
#[test]
fn c42_c43_delete_last_and_middle() {
    let _g = lock();
    for seed in 0..24usize {
        for &(es, ks) in &[(8usize, 4usize), (16, 8), (20, 8), (24, 16), (64, 16)] {
            // delete last
            sync_seed(seed * 977 + 1);
            let keys = keys_for(ks, 12);
            let (cd, rd) = drivers(es, ks, HM_BINARY);
            let mut ops: Vec<Op> = (0..12).map(|i| Op::Put(i, i as u8)).collect();
            for i in (0..12).rev() {
                ops.push(Op::Del(i, 0));
                ops.push(Op::Len);
                for j in 0..12 {
                    ops.push(Op::Get(j));
                }
            }
            run_ops(
                &format!("del-last seed={seed} es={es} ks={ks}"),
                cd,
                rd,
                &keys,
                &ops,
            );

            // delete first (always relocates the last element)
            sync_seed(seed * 977 + 1);
            let keys = keys_for(ks, 12);
            let (cd, rd) = drivers(es, ks, HM_BINARY);
            let mut ops: Vec<Op> = (0..12).map(|i| Op::Put(i, i as u8)).collect();
            for i in 0..12 {
                ops.push(Op::Del(i, 0));
                ops.push(Op::Len);
                for j in 0..12 {
                    ops.push(Op::Get(j));
                }
            }
            run_ops(
                &format!("del-first seed={seed} es={es} ks={ks}"),
                cd,
                rd,
                &keys,
                &ops,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C44 — tombstone_count > tombstone_count_threshold  =>  rebuild same slot_count
//       (slot_count 8: tct == 1, so the 2nd delete rebuilds)
// C45 — used_count < used_count_shrink_threshold && slot_count > 8  =>  shrink
// ---------------------------------------------------------------------------
#[test]
fn c44_c45_rebuild_and_shrink() {
    let _g = lock();
    // rebuild at slot_count 8
    for seed in 0..16usize {
        sync_seed(seed * 31 + 5);
        let keys = keys_for(8, 5);
        let (cd, rd) = drivers(16, 8, HM_BINARY);
        let ops = vec![
            Op::Put(0, 1),
            Op::Put(1, 2),
            Op::Put(2, 3),
            Op::Put(3, 4),
            Op::Del(3, 0), // tc = 1  (== tct, no rebuild)
            Op::Del(2, 0), // tc = 2  >  tct  => rebuild, slot_count stays 8
            Op::Get(0),
            Op::Get(1),
            Op::Get(2),
            Op::Get(3),
            Op::Put(4, 9),
            Op::Len,
        ];
        run_ops(&format!("rebuild8 seed={seed}"), cd, rd, &keys, &ops);
    }

    // shrink 16 -> 8 and 32 -> 16
    for seed in 0..16usize {
        for n in [7usize, 13, 26] {
            sync_seed(seed * 131 + 11);
            let keys = keys_for(8, n);
            let (cd, rd) = drivers(16, 8, HM_BINARY);
            let mut ops: Vec<Op> = (0..n).map(|i| Op::Put(i, i as u8)).collect();
            // delete everything; crosses every tombstone/shrink threshold
            for i in 0..n {
                ops.push(Op::Del(i, 0));
                ops.push(Op::Len);
                for j in 0..n {
                    ops.push(Op::Get(j));
                }
            }
            // and reinsert into the shrunk table
            for i in 0..n {
                ops.push(Op::Put(i, 0x80 + i as u8));
            }
            run_ops(&format!("shrink n={n} seed={seed}"), cd, rd, &keys, &ops);
        }
    }
}

// ---------------------------------------------------------------------------
// C46 — delete everything then re-insert: tombstone reuse (`pos = tombstone`,
//       `--tombstone_count`) — R36
// ---------------------------------------------------------------------------
#[test]
fn c46_tombstone_reuse() {
    let _g = lock();
    for seed in 0..32usize {
        sync_seed(seed * 7919 + 3);
        let keys = keys_for(8, 20);
        let (cd, rd) = drivers(16, 8, HM_BINARY);
        let mut ops = Vec::new();
        for cycle in 0..4u8 {
            for i in 0..5 {
                ops.push(Op::Put(i, cycle * 16 + i as u8));
            }
            for i in 0..5 {
                ops.push(Op::Del(i, 0));
            }
            for i in 0..5 {
                ops.push(Op::Get(i));
            }
            ops.push(Op::Len);
        }
        run_ops(&format!("tombstone-reuse seed={seed}"), cd, rd, &keys, &ops);
    }
}

// ---------------------------------------------------------------------------
// C47 — every (elemsize, keysize) geometry through a full put/get/del cycle
// C48 — keyoffset 0 vs non-zero on hmdel_key (R15)
// ---------------------------------------------------------------------------
#[test]
fn c47_c48_geometry_and_keyoffset() {
    let _g = lock();
    for &(es, ks) in GEOM {
        for &off in &[0usize, 1, 2, 4, 8] {
            if off + ks > es {
                continue;
            }
            sync_seed(0x4747);
            let keys = keys_for(ks, 10);
            let (cd, rd) = drivers(es, ks, HM_BINARY);
            let mut ops: Vec<Op> = (0..10).map(|i| Op::Put(i, i as u8)).collect();
            for i in 0..10 {
                ops.push(Op::Del(i, off));
                ops.push(Op::Len);
                ops.push(Op::Get(i));
            }
            run_ops(
                &format!("geom es={es} ks={ks} keyoffset={off}"),
                cd,
                rd,
                &keys,
                &ops,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C49 — hmput_default in every position (R16)
// ---------------------------------------------------------------------------
#[test]
fn c49_hmput_default() {
    let _g = lock();
    for &(es, ks) in GEOM {
        sync_seed(0x4949);
        let keys = keys_for(ks, 8);
        let (cd, rd) = drivers(es, ks, HM_BINARY);
        run_ops(
            &format!("default es={es} ks={ks}"),
            cd,
            rd,
            &keys,
            &[
                Op::Default(0x11), // on NULL
                Op::Len,
                Op::Get(0), // table is NULL => temp == -1
                Op::Default(0x22),
                Op::Put(0, 0x33),
                Op::Default(0x44), // length != 0 => unchanged
                Op::Get(0),
                Op::Put(1, 0x55),
                Op::Default(0x66),
                Op::Del(0, 0),
                Op::Default(0x77),
                Op::Len,
            ],
        );
    }

    // hmput_default *after* every element has been deleted (length becomes 1,
    // not 0, so the `length == 0` branch stays untaken)
    for &(es, ks) in &[(8usize, 4usize), (16, 8)] {
        sync_seed(0x494A);
        let keys = keys_for(ks, 4);
        let (cd, rd) = drivers(es, ks, HM_BINARY);
        let ops = vec![
            Op::Put(0, 1),
            Op::Put(1, 2),
            Op::Del(0, 0),
            Op::Del(1, 0),
            Op::Len,
            Op::Default(0x88),
            Op::Len,
            Op::Put(2, 3),
        ];
        run_ops(&format!("default-after-empty es={es}"), cd, rd, &keys, &ops);
    }
}

// ---------------------------------------------------------------------------
// C50 — hmfree_func for every table state (R4, R5)
// ---------------------------------------------------------------------------
#[test]
fn c50_hmfree_states() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        // (a) NULL — must be a no-op in both
        (c.hmfree_func)(std::ptr::null_mut(), 16);
        (r.hmfree_func)(std::ptr::null_mut(), 16);

        // (b) an array that never got a hash table (hash_table == NULL)
        for &es in &[8usize, 16, 24] {
            let ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            (*header(ca)).length = 1;
            (*header(ra)).length = 1;
            (c.hmfree_func)(ca, es);
            (r.hmfree_func)(ra, es);
        }

        // (c) real maps in every string.mode, populated then freed
        for &sh in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &mode in &[HM_BINARY] {
                sync_seed(0x5050);
                let keys = keys_for(8, 12);
                let mut cd = Drv::shmode(c, 16, 8, mode, sh);
                let mut rd = Drv::shmode(r, 16, 8, mode, sh);
                for (i, k) in keys.iter().enumerate() {
                    let a = cd.put(k, i as u8);
                    let b = rd.put(k, i as u8);
                    assert_eq!(a, b, "sh={sh} put#{i}");
                    eqs(&format!("hmfree-prep sh={sh} #{i}"), &cd.snap(), &rd.snap());
                }
                cd.free();
                rd.free();
            }
        }

        // (d) map created only by hmget_key on NULL (length 1, no table)
        for &es in &[8usize, 16] {
            let k = bin_key(7, 8);
            let ct = (c.hmget_key)(
                std::ptr::null_mut(),
                es,
                k.as_ptr() as *mut std::ffi::c_void,
                8,
                HM_BINARY,
            );
            let rt = (r.hmget_key)(
                std::ptr::null_mut(),
                es,
                k.as_ptr() as *mut std::ffi::c_void,
                8,
                HM_BINARY,
            );
            eqs(
                &format!("get-on-null es={es}"),
                &snap_map(ct, es, KeyKind::Bin),
                &snap_map(rt, es, KeyKind::Bin),
            );
            (c.hmfree_func)(hash_to_arr(ct, es), es);
            (r.hmfree_func)(hash_to_arr(rt, es), es);
        }
    }
}

// ---------------------------------------------------------------------------
// Long randomized property-style walks over the whole binary API surface.
// ---------------------------------------------------------------------------
#[test]
fn c35_c50_randomized_walks() {
    let _g = lock();
    for trial in 0..120u64 {
        let mut rng = Rng::new(0xB1A0_0000 + trial);
        let (es, ks) = GEOM[(trial as usize) % GEOM.len()];
        let nkeys = 8 + (rng.below(60) as usize);
        let keys = keys_for(ks, nkeys);
        sync_seed(rng.next_u64() as usize);

        let mut ops = Vec::new();
        for _ in 0..160 {
            let k = rng.below(nkeys as u64) as usize;
            ops.push(match rng.below(12) {
                0..=4 => Op::Put(k, rng.byte()),
                5 | 6 => Op::Get(k),
                7 => Op::GetTs(k),
                8..=10 => Op::Del(k, 0),
                _ => Op::Len,
            });
        }
        let (cd, rd) = drivers(es, ks, HM_BINARY);
        run_ops(
            &format!("walk trial={trial} es={es} ks={ks} nkeys={nkeys}"),
            cd,
            rd,
            &keys,
            &ops,
        );
    }
}
