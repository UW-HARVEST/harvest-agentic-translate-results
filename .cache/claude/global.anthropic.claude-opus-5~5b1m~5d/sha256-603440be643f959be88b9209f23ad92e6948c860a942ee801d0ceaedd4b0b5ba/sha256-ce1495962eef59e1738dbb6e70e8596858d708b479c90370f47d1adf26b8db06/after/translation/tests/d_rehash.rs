//! Phase D — independent model verification of `stbds_make_hash_index`'s rehash
//! loop, the one remaining place with a first-scan / wrap-around-scan pair:
//!
//! ```c
//! for (;;) {
//!   bucket = &t->storage[pos >> 3];
//!   for (z = pos & 7; z < 8; ++z)  if (bucket->hash[z]==0) { place; goto done; }
//!   limit = pos & 7;
//!   for (z = 0; z < limit; ++z)    if (bucket->hash[z]==0) { place; goto done; }
//!   pos += step; step += 8; pos &= (t->slot_count-1);
//! }
//! ```
//!
//! A reimplementation of that algorithm *from the C source* predicts the exact
//! new bucket array after every grow, shrink and rebuild; both libraries must
//! match the prediction bit for bit.  The test also counts how many entries were
//! placed via the wrap-around scan, proving the branch is genuinely reached.

mod common;
use common::*;

/// Which rehash the C is about to perform, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rehash {
    None,
    Grow(usize),
    Shrink(usize),
    Rebuild(usize),
}

/// `stbds_hmput_key`: `if (table == NULL || used_count >= used_count_threshold)`
unsafe fn rehash_on_put(ti: *const HashIndex) -> Rehash {
    unsafe {
        if (*ti).used_count >= (*ti).used_count_threshold {
            Rehash::Grow((*ti).slot_count * 2)
        } else {
            Rehash::None
        }
    }
}

/// `stbds_hmdel_key`'s tail, evaluated with the counters as they will be *after*
/// `--used_count` / `++tombstone_count`.
unsafe fn rehash_on_del(ti: *const HashIndex) -> Rehash {
    unsafe {
        let uc = (*ti).used_count - 1;
        let tc = (*ti).tombstone_count + 1;
        if uc < (*ti).used_count_shrink_threshold && (*ti).slot_count > 8 {
            Rehash::Shrink((*ti).slot_count >> 1)
        } else if tc > (*ti).tombstone_count_threshold {
            Rehash::Rebuild((*ti).slot_count)
        } else {
            Rehash::None
        }
    }
}

/// Live entries in the exact iteration order `stbds_make_hash_index` uses.
unsafe fn live_entries(ti: *const HashIndex) -> Vec<(usize, isize)> {
    unsafe {
        let mut v = Vec::new();
        for i in 0..((*ti).slot_count >> 3) {
            let b = (*ti).storage.add(i);
            for j in 0..8 {
                if (*b).index[j] >= 0 {
                    v.push(((*b).hash[j], (*b).index[j]));
                }
            }
        }
        v
    }
}

unsafe fn actual_arrays(ti: *const HashIndex) -> (Vec<usize>, Vec<isize>) {
    unsafe {
        let mut hash = Vec::new();
        let mut index = Vec::new();
        for i in 0..((*ti).slot_count >> 3) {
            let b = (*ti).storage.add(i);
            for j in 0..8 {
                hash.push((*b).hash[j]);
                index.push((*b).index[j]);
            }
        }
        (hash, index)
    }
}

/// Place `h`/`idx` using the "first empty slot from `pos`" probe that both
/// `stbds_make_hash_index` and `stbds_hmput_key`'s `found_empty_slot` use.
/// Returns true when the wrap-around scan did the placing.
fn place(hash: &mut [usize], index: &mut [isize], sc: usize, h: usize, idx: isize) -> bool {
    let mut pos = h & (sc - 1);
    let mut step = 8usize;
    loop {
        let base = pos & !7usize;
        for z in (pos & 7)..8 {
            if hash[base + z] == 0 {
                hash[base + z] = h;
                index[base + z] = idx;
                return false;
            }
        }
        for z in 0..(pos & 7) {
            if hash[base + z] == 0 {
                hash[base + z] = h;
                index[base + z] = idx;
                return true;
            }
        }
        pos = pos.wrapping_add(step);
        step += 8;
        pos &= sc - 1;
    }
}

/// The predicted bucket arrays of `stbds_make_hash_index(new_sc, ot)`.
fn model_rehash(entries: &[(usize, isize)], new_sc: usize) -> (Vec<usize>, Vec<isize>, usize) {
    let mut hash = vec![0usize; new_sc];
    let mut index = vec![-1isize; new_sc];
    let mut wraps = 0usize;
    for &(h, idx) in entries {
        if place(&mut hash, &mut index, new_sc, h, idx) {
            wraps += 1;
        }
    }
    (hash, index, wraps)
}

#[track_caller]
unsafe fn check_scalars(ctx: &str, who: &str, ti: *const HashIndex, new_sc: usize, uc: usize) {
    unsafe {
        assert_eq!((*ti).slot_count, new_sc, "[{who}] {ctx}: slot_count");
        assert_eq!((*ti).slot_count_log2, new_sc.trailing_zeros() as usize);
        assert_eq!((*ti).used_count, uc, "[{who}] {ctx}: used_count");
        assert_eq!((*ti).tombstone_count, 0, "[{who}] {ctx}: tombstone_count");
        assert_eq!((*ti).used_count_threshold, new_sc - (new_sc >> 2));
        assert_eq!((*ti).tombstone_count_threshold, (new_sc >> 3) + (new_sc >> 4));
        assert_eq!(
            (*ti).used_count_shrink_threshold,
            if new_sc <= 8 { 0 } else { new_sc >> 2 }
        );
        // the live C assertion (ERRORS.md A1)
        assert!(
            (*ti).used_count_threshold + (*ti).tombstone_count_threshold < (*ti).slot_count,
            "[{who}] {ctx}: STBDS_ASSERT in stbds_make_hash_index would have fired"
        );
    }
}

#[track_caller]
unsafe fn check_arrays(
    ctx: &str,
    who: &str,
    ti: *const HashIndex,
    want_hash: &[usize],
    want_index: &[isize],
) {
    unsafe {
        let (ah, ai) = actual_arrays(ti);
        assert_eq!(want_hash, &ah[..], "[{who}] {ctx}: bucket hash[] differs from the model");
        assert_eq!(want_index, &ai[..], "[{who}] {ctx}: bucket index[] differs from the model");
    }
}

#[test]
fn make_hash_index_rehash_matches_an_independent_model() {
    let _g = lock();
    let (c, r) = both();
    let mut grows = 0usize;
    let mut shrinks = 0usize;
    let mut rebuilds = 0usize;
    let mut wraps_total = 0usize;

    unsafe {
        for seed in 0..150usize {
            sync_seed(seed.wrapping_mul(0x9E37_79B9) | 1);
            let keys: Vec<Vec<u8>> = (0..70)
                .map(|i| bin_key(i as u64 * 6364136223846793005 + 1, 8))
                .collect();
            let mut cd = Drv::shmode(c, 16, 8, HM_BINARY, SH_NONE);
            let mut rd = Drv::shmode(r, 16, 8, HM_BINARY, SH_NONE);
            let mut rng = Rng::new(0xD0D0_0000 + seed as u64);

            for step in 0..340 {
                let k = rng.below(70) as usize;
                // fill, then churn, then drain: the drain phase is what drives
                // `used_count` below `used_count_shrink_threshold` and exercises
                // the shrinking rehash (slot_count 128->64->32->16->8).
                let put_weight = match step {
                    0..=90 => 9,
                    91..=200 => 5,
                    _ => 1,
                };
                let put = rng.below(10) < put_weight;
                let ctx = format!("seed={seed} step={step} k={k} put={put}");

                // -- state and plan BEFORE the call ------------------------
                let before = live_entries(cd.table());
                assert_eq!(before, live_entries(rd.table()), "{ctx}: pre-state");
                let cur_idx = cd.get(&keys[k]);
                assert_eq!(cur_idx, rd.get(&keys[k]), "{ctx}: lookup");
                let hmlen_before = cd.len();
                assert_eq!(hmlen_before, rd.len(), "{ctx}: hmlen");

                let plan = if put {
                    rehash_on_put(cd.table())
                } else if cur_idx >= 0 {
                    rehash_on_del(cd.table())
                } else {
                    Rehash::None
                };
                let plan_r = if put {
                    rehash_on_put(rd.table())
                } else if cur_idx >= 0 {
                    rehash_on_del(rd.table())
                } else {
                    Rehash::None
                };
                assert_eq!(plan, plan_r, "{ctx}: rehash plans disagree");

                // -- the call ----------------------------------------------
                if put {
                    let tag = rng.byte();
                    assert_eq!(cd.put(&keys[k], tag), rd.put(&keys[k], tag), "{ctx}");
                } else {
                    assert_eq!(cd.del(&keys[k], 0), rd.del(&keys[k], 0), "{ctx}");
                }
                eqs(&ctx, &cd.snap(), &rd.snap());

                // -- model the rehash --------------------------------------
                match plan {
                    Rehash::None => {}

                    Rehash::Grow(new_sc) => {
                        grows += 1;
                        // `stbds_hmput_key` rehashes the OLD entries into the new
                        // table and only then probes for the new key's slot.
                        let (mut mh, mut mi, wraps) = model_rehash(&before, new_sc);
                        wraps_total += wraps;
                        let mut uc = before.len();
                        if cur_idx < 0 {
                            // a genuinely new key was appended
                            let new_idx = temp_of(cd.t, 16);
                            let (ah, _) = actual_arrays(cd.table());
                            let mut newh = 0usize;
                            for i in 0..new_sc {
                                if ah[i] != 0 && mh[i] == 0 {
                                    newh = ah[i];
                                }
                            }
                            assert_ne!(newh, 0, "{ctx}: could not locate the new entry");
                            place(&mut mh, &mut mi, new_sc, newh, new_idx);
                            uc += 1;
                        }
                        check_scalars(&ctx, "C", cd.table(), new_sc, uc);
                        check_scalars(&ctx, "Rust", rd.table(), new_sc, uc);
                        check_arrays(&ctx, "C", cd.table(), &mh, &mi);
                        check_arrays(&ctx, "Rust", rd.table(), &mh, &mi);
                    }

                    Rehash::Shrink(new_sc) | Rehash::Rebuild(new_sc) => {
                        if matches!(plan, Rehash::Shrink(_)) {
                            shrinks += 1;
                        } else {
                            rebuilds += 1;
                        }
                        // `stbds_hmdel_key` first tombstones `cur_idx`'s slot and,
                        // when the deleted element was not the last one, retargets
                        // the entry that pointed at `final_index` to `cur_idx`.
                        // The old table is rehashed in that exact state, preserving
                        // iteration order.
                        let final_index = hmlen_before - 1;
                        let survivors: Vec<(usize, isize)> = before
                            .iter()
                            .filter(|&&(_, i)| i != cur_idx)
                            .map(|&(h, i)| if i == final_index { (h, cur_idx) } else { (h, i) })
                            .collect();
                        assert_eq!(survivors.len(), before.len() - 1, "{ctx}: survivor count");
                        let (mh, mi, wraps) = model_rehash(&survivors, new_sc);
                        wraps_total += wraps;
                        check_scalars(&ctx, "C", cd.table(), new_sc, survivors.len());
                        check_scalars(&ctx, "Rust", rd.table(), new_sc, survivors.len());
                        check_arrays(&ctx, "C", cd.table(), &mh, &mi);
                        check_arrays(&ctx, "Rust", rd.table(), &mh, &mi);
                    }
                }
            }
            cd.free();
            rd.free();
        }
    }

    eprintln!(
        "coverage: grows={grows} shrinks={shrinks} rebuilds={rebuilds} \
         wrap-around rehash placements={wraps_total}"
    );
    assert!(grows > 0, "no grow rehash exercised");
    assert!(shrinks > 0, "no shrink rehash exercised");
    assert!(rebuilds > 0, "no rebuild rehash exercised");
    assert!(
        wraps_total > 0,
        "the rehash wrap-around scan was never exercised"
    );
}

/// Same, but with string keys and every `STBDS_SH_*` key-storage mode, so the
/// arena/strdup state that `stbds_make_hash_index` copies (`t->string = ot->string`,
/// `t->seed = ot->seed`) is verified across grows, shrinks and rebuilds too.
#[test]
fn rehash_preserves_seed_and_arena_across_all_modes() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for seed in 0..40usize {
                sync_seed(seed.wrapping_mul(7919) | 1);
                let keys: Vec<Vec<u8>> = (0..60)
                    .map(|i| padded_key(format!("rk{i:04}_{}", "p".repeat(i % 30)).as_bytes()))
                    .collect();
                let mut cd = Drv::shmode(c, 16, 8, HM_STRING, sh);
                let mut rd = Drv::shmode(r, 16, 8, HM_STRING, sh);
                let table_seed = (*cd.table()).seed;
                assert_eq!(table_seed, (*rd.table()).seed);
                let mut rng = Rng::new(0xD1D1_0000 + seed as u64 + sh as u64 * 1000);

                for step in 0..200 {
                    let k = rng.below(60) as usize;
                    let ctx = format!("sh={sh} seed={seed} step={step}");
                    if rng.below(10) < 6 {
                        let tag = rng.byte();
                        assert_eq!(cd.put(&keys[k], tag), rd.put(&keys[k], tag), "{ctx}");
                    } else {
                        assert_eq!(cd.del(&keys[k], 0), rd.del(&keys[k], 0), "{ctx}");
                    }
                    eqs(&ctx, &cd.snap(), &rd.snap());
                    // every rehash must carry the seed over unchanged
                    assert_eq!((*cd.table()).seed, table_seed, "{ctx}: C seed lost");
                    assert_eq!((*rd.table()).seed, table_seed, "{ctx}: Rust seed lost");
                    assert_eq!(
                        (*cd.table()).string.mode,
                        sh as u8,
                        "{ctx}: C string.mode lost"
                    );
                    assert_eq!(
                        (*rd.table()).string.mode,
                        sh as u8,
                        "{ctx}: Rust string.mode lost"
                    );
                }
                // every key must still be findable / absent consistently
                for k in keys.iter() {
                    assert_eq!(cd.get(k), rd.get(k));
                    assert_eq!(cd.get_ts(k), rd.get_ts(k));
                }
                cd.free();
                rd.free();
            }
        }
    }
}
