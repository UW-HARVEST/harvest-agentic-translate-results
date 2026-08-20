//! Phase B — valid-path differential tests, CONFIGS.md rows 23..36.
//! `stbds_hmdel_key`: swap-with-last, tombstones, rebuild, shrink, keyoffset,
//! and long randomized op streams over all four `string.mode` flavours.
mod common;

use common::*;
use std::ffi::{c_char, c_void};

fn int_map() -> ElemDesc {
    ElemDesc::all_raw(8)
}
fn str_map() -> ElemDesc {
    ElemDesc::ptr_key(16)
}
fn i32k(v: i32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}
fn i64k(v: i64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}
fn rand_key(rng: &mut Rng) -> Vec<u8> {
    let n = 1 + rng.below(24);
    rng.cstr_bytes(n, false)
}

// ---------------------------------------------------------------------------
// row 23 — delete the LAST element (old_index == final_index, no swap)
// ---------------------------------------------------------------------------
#[test]
fn cfg_23_del_last_element() {
    for &n in &[1usize, 2, 5, 6, 7, 13, 40, 200] {
        for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
            let mut m = MapPair::new(int_map(), 4, &format!("del-last n={n} seed={seed:#x}"));
            m.seed(seed);
            for i in 0..n as i32 {
                m.put_binary(&i32k(i), &i32k(i * 7), HM_BINARY);
            }
            // LIFO deletion always hits old_index == final_index
            for i in (0..n as i32).rev() {
                let k = i32k(i);
                let r = m.del(k.as_ptr() as *mut c_void, 4, 0, HM_BINARY);
                assert_eq!(r, 1, "delete of present key must report 1");
                assert_eq!(unsafe { hmlen(m.ct, 8) }, i as isize);
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 24 — delete a MIDDLE element: memmove + BINARY re-find + index fixup
// ---------------------------------------------------------------------------
#[test]
fn cfg_24_del_middle_binary() {
    let mut rng = Rng::new(0xB0_0024);
    for &n in &[2usize, 3, 6, 7, 13, 40, 300] {
        for &seed in &[1usize, 0x3141_5926, 0xabcd_ef01_2345_6789] {
            let mut m = MapPair::new(int_map(), 4, &format!("del-mid n={n} seed={seed:#x}"));
            m.seed(seed);
            for i in 0..n as i32 {
                m.put_binary(&i32k(i), &i32k(i * 13 + 1), HM_BINARY);
            }
            let mut live: Vec<i32> = (0..n as i32).collect();
            while !live.is_empty() {
                let pick = rng.below(live.len());
                let k = live.remove(pick);
                let kb = i32k(k);
                assert_eq!(m.del(kb.as_ptr() as *mut c_void, 4, 0, HM_BINARY), 1);
                assert_eq!(unsafe { hmlen(m.ct, 8) }, live.len() as isize);
                // every surviving key must still be findable at the same index
                for &s in &live {
                    let sb = i32k(s);
                    assert!(
                        m.get(sb.as_ptr() as *mut c_void, 4, HM_BINARY) >= 0,
                        "key {s} lost after deleting {k}"
                    );
                }
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 25 — delete a middle element on a STRING map: the re-find uses
//          `*(char**)elem` (mode == STBDS_HM_STRING exactly)
// ---------------------------------------------------------------------------
#[test]
fn cfg_25_del_middle_string_default() {
    let mut rng = Rng::new(0xB0_0025);
    for &n in &[2usize, 6, 7, 25, 120] {
        let mut m = MapPair::new(str_map(), 8, &format!("del-mid-str n={n}"));
        m.seed(0x3141_5926);
        let mut keys: Vec<*mut c_char> = Vec::new();
        let mut seen: Vec<Vec<u8>> = Vec::new();
        while keys.len() < n {
            let kb = rand_key(&mut rng);
            if seen.contains(&kb) {
                continue;
            }
            seen.push(kb.clone());
            let k = leak_cstr(&kb);
            m.put_string(k, &i64k(keys.len() as i64), HM_STRING);
            keys.push(k);
        }
        while !keys.is_empty() {
            let pick = rng.below(keys.len());
            let k = keys.remove(pick);
            assert_eq!(m.del(k as *mut c_void, 8, 0, HM_STRING), 1);
            for &s in &keys {
                assert!(m.get(s as *mut c_void, 8, HM_STRING) >= 0, "string key lost");
            }
            free_raw(k);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 26 — delete on a SH_STRDUP table: the removed key is freed
// ---------------------------------------------------------------------------
#[test]
fn cfg_26_del_strdup() {
    let mut rng = Rng::new(0xB0_0026);
    for &n in &[1usize, 2, 7, 30, 150] {
        let mut m = MapPair::new(str_map(), 8, &format!("del-strdup n={n}"));
        m.seed(0x3141_5926);
        m.shmode(SH_STRDUP);
        let mut keys: Vec<*mut c_char> = Vec::new();
        let mut seen: Vec<Vec<u8>> = Vec::new();
        while keys.len() < n {
            let kb = rand_key(&mut rng);
            if seen.contains(&kb) {
                continue;
            }
            seen.push(kb.clone());
            let k = leak_cstr(&kb);
            m.put_string(k, &i64k(keys.len() as i64), HM_STRING);
            keys.push(k);
        }
        while !keys.is_empty() {
            let pick = rng.below(keys.len());
            let k = keys.remove(pick);
            assert_eq!(m.del(k as *mut c_void, 8, 0, HM_STRING), 1);
            for &s in &keys {
                assert!(m.get(s as *mut c_void, 8, HM_STRING) >= 0);
            }
            free_raw(k);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 27 — delete then re-insert so the put lands on a TOMBSTONE
// ---------------------------------------------------------------------------
#[test]
fn cfg_27_put_onto_tombstone() {
    let mut rng = Rng::new(0xB0_0027);
    for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
        let mut m = MapPair::new(int_map(), 4, &format!("tombstone seed={seed:#x}"));
        m.seed(seed);
        for i in 0..5i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
        }
        // churn: delete one and immediately put a new key, many times
        for round in 0..400i32 {
            let victim = i32k(round);
            let d = m.del(victim.as_ptr() as *mut c_void, 4, 0, HM_BINARY);
            let _ = d;
            m.put_binary(&i32k(round + 5), &i32k(round), HM_BINARY);
            if round % 37 == 0 {
                let probe = i32k(rng.next_u32() as i32);
                m.get_ts(probe.as_ptr() as *mut c_void, 4, HM_BINARY);
            }
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 28 — exceed tombstone_count_threshold => REBUILD at the same slot_count
// ---------------------------------------------------------------------------
#[test]
fn cfg_28_tombstone_rebuild() {
    // slot_count 64 => used_thr 48, tomb_thr (64>>3)+(64>>4)=8+4=12,
    // shrink_thr 16. Keep used_count above 16 while accumulating >12 tombstones.
    for &seed in &[1usize, 0x3141_5926, 0xdead_beef] {
        let mut m = MapPair::new(int_map(), 4, &format!("rebuild seed={seed:#x}"));
        m.seed(seed);
        for i in 0..40i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
        }
        let slots_before = unsafe {
            (*((*header_of(m.ct, 8)).hash_table as *const HashIndex)).slot_count
        };
        // delete 14 of them (>12) but keep used_count >= shrink threshold.
        // Track `tombstone_count` after every delete: a rebuild is observable
        // as a drop back to 0 while `slot_count` stays the same.
        let mut saw_rebuild = false;
        let mut prev_tomb = 0usize;
        for i in 0..14i32 {
            let k = i32k(i * 2);
            assert_eq!(m.del(k.as_ptr() as *mut c_void, 4, 0, HM_BINARY), 1);
            let ti = unsafe { *((*header_of(m.ct, 8)).hash_table as *const HashIndex) };
            let ri = unsafe { *((*header_of(m.rt, 8)).hash_table as *const HashIndex) };
            assert_eq!(ti.slot_count, ri.slot_count, "slot_count diverged");
            assert_eq!(ti.tombstone_count, ri.tombstone_count, "tombstones diverged");
            assert_eq!(ti.used_count, ri.used_count, "used_count diverged");
            assert_eq!(
                ti.tombstone_count_threshold, ri.tombstone_count_threshold,
                "tombstone threshold diverged"
            );
            if ti.tombstone_count == 0 && prev_tomb > 0 && ti.slot_count == slots_before {
                saw_rebuild = true;
            }
            prev_tomb = ti.tombstone_count;
        }
        let ti = unsafe { *((*header_of(m.ct, 8)).hash_table as *const HashIndex) };
        let ri = unsafe { *((*header_of(m.rt, 8)).hash_table as *const HashIndex) };
        assert_eq!(ti.slot_count, ri.slot_count);
        assert_eq!(ti.tombstone_count, ri.tombstone_count);
        assert_eq!(
            ti.slot_count, slots_before,
            "a tombstone rebuild must keep slot_count"
        );
        assert!(
            saw_rebuild,
            "expected to observe a tombstone rebuild (tombstone_count -> 0 at \
             constant slot_count); slot_count={} tomb_thr={}",
            ti.slot_count, ti.tombstone_count_threshold
        );
        for i in 0..40i32 {
            let k = i32k(i);
            let want_present = i % 2 == 1 || i >= 28;
            let idx = m.get(k.as_ptr() as *mut c_void, 4, HM_BINARY);
            assert_eq!(idx >= 0, want_present, "key {i} presence wrong after rebuild");
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 29 — used_count below shrink threshold with slot_count > 8 => SHRINK
// ---------------------------------------------------------------------------
#[test]
fn cfg_29_shrink() {
    for &seed in &[1usize, 0x3141_5926, 0x1234_5678_9abc_def0] {
        for &n in &[13usize, 25, 50, 100, 400] {
            let mut m = MapPair::new(int_map(), 4, &format!("shrink n={n} seed={seed:#x}"));
            m.seed(seed);
            for i in 0..n as i32 {
                m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
            }
            let big = unsafe {
                (*((*header_of(m.ct, 8)).hash_table as *const HashIndex)).slot_count
            };
            assert!(big > 8);
            // delete everything, walking every shrink/rebuild step
            for i in 0..n as i32 {
                let k = i32k(i);
                assert_eq!(m.del(k.as_ptr() as *mut c_void, 4, 0, HM_BINARY), 1);
            }
            let small = unsafe {
                (*((*header_of(m.ct, 8)).hash_table as *const HashIndex)).slot_count
            };
            assert_eq!(small, 8, "must shrink all the way back to 8 slots");
            assert_eq!(unsafe { hmlen(m.ct, 8) }, 0);
            // and it must be reusable
            for i in 0..n as i32 {
                m.put_binary(&i32k(i + 10_000), &i32k(i), HM_BINARY);
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 30 — slot_count == 8 boundary: used_count_shrink_threshold forced to 0
// ---------------------------------------------------------------------------
#[test]
fn cfg_30_no_shrink_below_8() {
    for &seed in &[0usize, 1, 0x3141_5926] {
        let mut m = MapPair::new(int_map(), 4, &format!("min-slots seed={seed:#x}"));
        m.seed(seed);
        for i in 0..5i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
        }
        unsafe {
            let ti = *((*header_of(m.ct, 8)).hash_table as *const HashIndex);
            let ri = *((*header_of(m.rt, 8)).hash_table as *const HashIndex);
            assert_eq!(ti.slot_count, 8);
            assert_eq!(ti.used_count_shrink_threshold, 0);
            assert_eq!(ri.used_count_shrink_threshold, 0);
        }
        for i in 0..5i32 {
            let k = i32k(i);
            m.del(k.as_ptr() as *mut c_void, 4, 0, HM_BINARY);
            unsafe {
                let ti = *((*header_of(m.ct, 8)).hash_table as *const HashIndex);
                assert_eq!(ti.slot_count, 8, "must never shrink below 8");
            }
        }
        // churn on the 8-slot table for a long time
        for i in 0..500i32 {
            m.put_binary(&i32k(i), &i32k(i), HM_BINARY);
            let k = i32k(i);
            m.del(k.as_ptr() as *mut c_void, 4, 0, HM_BINARY);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 31 — non-zero `keyoffset`
//
// `stbds_hmput_key` always uses keyoffset 0, so a non-zero keyoffset makes
// `stbds_hmdel_key` compare the *value* half of an element. Two shapes:
//  (a) keys that are never found  -> the `slot < 0` early return
//  (b) elements built with key == value, so a keyoffset-4 lookup DOES match
//      and the whole swap-with-last + re-find path runs consistently.
// ---------------------------------------------------------------------------
#[test]
fn cfg_31_keyoffset() {
    let mut rng = Rng::new(0xB0_0031);

    // (a) never-matching keyoffset lookups
    for &seed in &[1usize, 0x3141_5926] {
        let mut m = MapPair::new(int_map(), 4, &format!("keyoff-miss seed={seed:#x}"));
        m.seed(seed);
        for i in 0..50i32 {
            m.put_binary(&i32k(i), &i32k(0x7000_0000 + i), HM_BINARY);
        }
        for _ in 0..2000 {
            let k = i32k(rng.next_u32() as i32);
            m.del(k.as_ptr() as *mut c_void, 4, 4, HM_BINARY);
        }
        m.free();
    }

    // (b) key == value so keyoffset 4 finds real entries
    for &seed in &[1usize, 0x3141_5926, usize::MAX] {
        for &n in &[2usize, 3, 7, 20, 80] {
            let mut m = MapPair::new(int_map(), 4, &format!("keyoff-hit n={n} seed={seed:#x}"));
            m.seed(seed);
            for i in 0..n as i32 {
                m.put_binary(&i32k(i), &i32k(i), HM_BINARY); // key == value
            }
            for i in (0..n as i32).rev() {
                let k = i32k(i);
                m.del(k.as_ptr() as *mut c_void, 4, 4, HM_BINARY);
            }
            m.free();
        }
    }

    // (c) string map with keyoffset 0 vs the pointer field — LIFO only, so the
    //     address-dependent re-find branch is never taken.
    for &n in &[1usize, 4, 12] {
        let mut m = MapPair::new(str_map(), 8, &format!("keyoff-str n={n}"));
        m.seed(0x3141_5926);
        let mut keys = Vec::new();
        let mut seen: Vec<Vec<u8>> = Vec::new();
        while keys.len() < n {
            let kb = rand_key(&mut rng);
            if seen.contains(&kb) {
                continue;
            }
            seen.push(kb.clone());
            let k = leak_cstr(&kb);
            m.put_string(k, &i64k(keys.len() as i64), HM_STRING);
            keys.push(k);
        }
        while let Some(k) = keys.pop() {
            assert_eq!(m.del(k as *mut c_void, 8, 0, HM_STRING), 1);
            free_raw(k);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 32 — full lifecycle: delete everything, re-put, across grow/shrink
// ---------------------------------------------------------------------------
#[test]
fn cfg_32_full_lifecycle() {
    let mut rng = Rng::new(0xB0_0032);
    for &seed in &[1usize, 0x3141_5926] {
        let mut m = MapPair::new(int_map(), 4, &format!("lifecycle seed={seed:#x}"));
        m.seed(seed);
        for cycle in 0..6i32 {
            let n = 1 + rng.below(120) as i32;
            for i in 0..n {
                m.put_binary(&i32k(cycle * 1000 + i), &i32k(i), HM_BINARY);
            }
            let mut live: Vec<i32> = (0..n).map(|i| cycle * 1000 + i).collect();
            while !live.is_empty() {
                let k = live.remove(rng.below(live.len()));
                let kb = i32k(k);
                assert_eq!(m.del(kb.as_ptr() as *mut c_void, 4, 0, HM_BINARY), 1);
            }
            assert_eq!(unsafe { hmlen(m.ct, 8) }, 0);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 33..36 — randomized op streams over all four table flavours,
//               state compared after EVERY op
// ---------------------------------------------------------------------------

fn op_stream_binary(seed: u64, hash_seed: usize, ops: usize, key_space: i32) {
    let mut rng = Rng::new(seed);
    let mut m = MapPair::new(
        int_map(),
        4,
        &format!("stream-bin seed={seed:#x} hs={hash_seed:#x}"),
    );
    m.seed(hash_seed);
    let mut model: Vec<i32> = Vec::new();
    for _ in 0..ops {
        let k = (rng.next_u32() as i32).rem_euclid(key_space);
        let kb = i32k(k);
        match rng.below(10) {
            0..=4 => {
                m.put_binary(&kb, &i32k(rng.next_u32() as i32), HM_BINARY);
                if !model.contains(&k) {
                    model.push(k);
                }
            }
            5..=6 => {
                let idx = m.get(kb.as_ptr() as *mut c_void, 4, HM_BINARY);
                assert_eq!(
                    idx >= 0,
                    model.contains(&k),
                    "presence disagrees with the model for {k}"
                );
            }
            7 => {
                m.get_ts(kb.as_ptr() as *mut c_void, 4, HM_BINARY);
            }
            _ => {
                let r = m.del(kb.as_ptr() as *mut c_void, 4, 0, HM_BINARY);
                let had = model.contains(&k);
                assert_eq!(r, if had { 1 } else { 0 }, "hmdel result for {k}");
                model.retain(|&x| x != k);
            }
        }
        assert_eq!(unsafe { hmlen(m.ct, 8) }, model.len() as isize);
    }
    m.free();
}

#[test]
fn cfg_33_random_op_stream_binary() {
    op_stream_binary(0xB0_0033, 0x3141_5926, 2000, 64);
    op_stream_binary(0xB0_0034, 1, 2000, 12);
    op_stream_binary(0xB0_0035, usize::MAX, 2000, 500);
}

fn op_stream_string(seed: u64, table_mode: i32, ops: usize, key_space: usize) {
    let mut rng = Rng::new(seed);
    let desc = str_map();
    let mut m = MapPair::new(
        desc,
        8,
        &format!("stream-str seed={seed:#x} table={table_mode}"),
    );
    m.seed(0x3141_5926);
    if table_mode >= 0 {
        m.shmode(table_mode);
    }
    // a fixed pool of distinct keys
    let mut pool: Vec<(*mut c_char, Vec<u8>)> = Vec::new();
    let mut seen: Vec<Vec<u8>> = Vec::new();
    while pool.len() < key_space {
        let kb = rand_key(&mut rng);
        if seen.contains(&kb) {
            continue;
        }
        seen.push(kb.clone());
        pool.push((leak_cstr(&kb), kb));
    }
    let mut model: Vec<usize> = Vec::new();
    for _ in 0..ops {
        let ki = rng.below(pool.len());
        let k = pool[ki].0;
        match rng.below(10) {
            0..=4 => {
                m.put_string(k, &i64k(rng.next_u64() as i64), HM_STRING);
                // NOTE: `temp_key` is deliberately NOT checked here. After a
                // delete (or a table shrink/rebuild) the C code leaves
                // `table->temp_key` stale/uninitialised — for SH_STRDUP it can
                // even point at freed memory — so reading it is only valid in
                // the put-only sequences (cfg_13/14/15/18).
                if !model.contains(&ki) {
                    model.push(ki);
                }
            }
            5..=6 => {
                let idx = m.get(k as *mut c_void, 8, HM_STRING);
                assert_eq!(idx >= 0, model.contains(&ki), "presence disagrees");
            }
            7 => {
                m.get_ts(k as *mut c_void, 8, HM_STRING);
            }
            _ => {
                let r = m.del(k as *mut c_void, 8, 0, HM_STRING);
                let had = model.contains(&ki);
                assert_eq!(r, if had { 1 } else { 0 });
                model.retain(|&x| x != ki);
            }
        }
        assert_eq!(unsafe { hmlen(m.ct, 16) }, model.len() as isize);
    }
    m.free();
    for (p, _) in pool {
        free_raw(p);
    }
}

#[test]
fn cfg_34_random_op_stream_string_default() {
    op_stream_string(0xB0_0036, -1, 2000, 40); // auto-created => SH_DEFAULT
    op_stream_string(0xB0_0037, SH_DEFAULT, 2000, 10);
}

#[test]
fn cfg_35_random_op_stream_strdup() {
    op_stream_string(0xB0_0038, SH_STRDUP, 2000, 40);
    op_stream_string(0xB0_0039, SH_STRDUP, 2000, 8);
}

#[test]
fn cfg_36_random_op_stream_arena() {
    op_stream_string(0xB0_003A, SH_ARENA, 2000, 40);
    op_stream_string(0xB0_003B, SH_ARENA, 1500, 8);
}
