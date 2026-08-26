//! Phase B — CONFIGS.md rows 21..47: the binary hash map driven through its
//! *lowest-level* entry points (`stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_hmdel_key`,
//! `stbds_hmfree_func`) exactly the way the `stbds_hm*` macros drive them.

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// (elemsize, keysize) shapes the C code distinguishes.
const SHAPES: [(usize, usize); 10] = [
    (8, 4),   // hm_geti's `struct { int key; int value; }`
    (8, 8),   // key fills the element
    (16, 8),
    (16, 4),  // memcpy copies 4 of 16 bytes
    (24, 16), // stbds_struct2-ish `int key[2]` + payload
    (32, 32),
    (4, 4),
    (2, 2),
    (1, 1),
    (64, 8),
];

fn key_bytes(k: u64, keysize: usize) -> Vec<u8> {
    let le = k.to_le_bytes();
    let mut v = vec![0u8; keysize];
    for i in 0..keysize.min(8) {
        v[i] = le[i];
    }
    for i in 8..keysize {
        v[i] = (k as u8).wrapping_mul(i as u8).wrapping_add(0x5a);
    }
    v
}

fn payload_bytes(k: u64, n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| (k as u8).wrapping_mul(3).wrapping_add(i as u8).wrapping_add(0xA5))
        .collect()
}

/// Drives the C `.so` and the Rust `.so` in lock-step through one map.
struct Duo {
    c: Api,
    r: Api,
    cfg: MapCfg,
    ct: *mut c_void,
    rt: *mut c_void,
}

impl Duo {
    fn new(cfg: MapCfg, seed: usize) -> Duo {
        let (c, r) = load_both();
        unsafe { pin_seed(&c, &r, seed) };
        Duo { c, r, cfg, ct: std::ptr::null_mut(), rt: std::ptr::null_mut() }
    }

    unsafe fn check(&self, ctx: &str) {
        diff_eq(
            ctx,
            &snapshot_map(self.ct, self.cfg.elemsize, self.cfg.kind),
            &snapshot_map(self.rt, self.cfg.elemsize, self.cfg.kind),
        );
    }

    unsafe fn put(&mut self, k: u64, ctx: &str) {
        let key = key_bytes(k, self.cfg.keysize);
        let pl = payload_bytes(k, self.cfg.elemsize.saturating_sub(self.cfg.keysize));
        self.ct = map_put_binary(&self.c, self.ct, &self.cfg, &key, &pl);
        self.rt = map_put_binary(&self.r, self.rt, &self.cfg, &key, &pl);
        self.check(&format!("{ctx} put({k})"));
    }

    unsafe fn get(&mut self, k: u64, ctx: &str) -> isize {
        let mut key = key_bytes(k, self.cfg.keysize);
        let (ct, ci) = map_geti(&self.c, self.ct, &self.cfg, &mut key);
        let mut key = key_bytes(k, self.cfg.keysize);
        let (rt, ri) = map_geti(&self.r, self.rt, &self.cfg, &mut key);
        self.ct = ct;
        self.rt = rt;
        diff_eq_val(&format!("{ctx} geti({k}) index"), ci, ri);
        self.check(&format!("{ctx} geti({k})"));
        if ci >= 0 {
            let ce = (self.ct as *const u8).wrapping_offset(ci * self.cfg.elemsize as isize);
            let re = (self.rt as *const u8).wrapping_offset(ri * self.cfg.elemsize as isize);
            diff_eq_val(
                &format!("{ctx} geti({k}) element"),
                std::slice::from_raw_parts(ce, self.cfg.elemsize).to_vec(),
                std::slice::from_raw_parts(re, self.cfg.elemsize).to_vec(),
            );
        }
        ci
    }

    unsafe fn get_ts(&mut self, k: u64, ctx: &str) -> isize {
        let mut key = key_bytes(k, self.cfg.keysize);
        let (ct, ci) = map_geti_ts(&self.c, self.ct, &self.cfg, &mut key);
        let mut key = key_bytes(k, self.cfg.keysize);
        let (rt, ri) = map_geti_ts(&self.r, self.rt, &self.cfg, &mut key);
        self.ct = ct;
        self.rt = rt;
        diff_eq_val(&format!("{ctx} geti_ts({k}) index"), ci, ri);
        self.check(&format!("{ctx} geti_ts({k})"));
        ci
    }

    unsafe fn del(&mut self, k: u64, ctx: &str) -> isize {
        let mut key = key_bytes(k, self.cfg.keysize);
        let (ct, cr) = map_del(&self.c, self.ct, &self.cfg, &mut key);
        let mut key = key_bytes(k, self.cfg.keysize);
        let (rt, rr) = map_del(&self.r, self.rt, &self.cfg, &mut key);
        diff_eq_val(&format!("{ctx} del({k}) null-ness"), ct.is_null(), rt.is_null());
        self.ct = ct;
        self.rt = rt;
        diff_eq_val(&format!("{ctx} del({k}) result"), cr, rr);
        self.check(&format!("{ctx} del({k})"));
        cr
    }

    unsafe fn put_default(&mut self, val: u8, ctx: &str) {
        self.ct = (self.c.hmput_default)(self.ct, self.cfg.elemsize);
        self.rt = (self.r.hmput_default)(self.rt, self.cfg.elemsize);
        // `hmdefault(t,v)`: `(t)[-1].value = v`
        if self.cfg.elemsize > self.cfg.keysize {
            let n = self.cfg.elemsize - self.cfg.keysize;
            let v = vec![val; n];
            let ce = (self.ct as *mut u8).wrapping_sub(self.cfg.elemsize);
            let re = (self.rt as *mut u8).wrapping_sub(self.cfg.elemsize);
            std::ptr::copy_nonoverlapping(v.as_ptr(), ce.wrapping_add(self.cfg.keysize), n);
            std::ptr::copy_nonoverlapping(v.as_ptr(), re.wrapping_add(self.cfg.keysize), n);
        }
        self.check(&format!("{ctx} put_default({val})"));
    }

    unsafe fn free(&mut self) {
        map_free(&self.c, self.ct, self.cfg.elemsize);
        map_free(&self.r, self.rt, self.cfg.elemsize);
        self.ct = std::ptr::null_mut();
        self.rt = std::ptr::null_mut();
    }
}

// ---------------------------------------------------------- rows 21..23
#[test]
fn row21_23_hmput_default() {
    let _g = global_lock();
    for &(es, ks) in &SHAPES {
        let mut d = Duo::new(MapCfg::binary(es, ks), 0x31415926);
        unsafe {
            // row 21: a == NULL -> create
            d.put_default(0x7e, "row21");
            // row 23: a != NULL and length != 0 -> unchanged / idempotent
            d.put_default(0x7e, "row23 idempotent");
            let before_c = snapshot_map(d.ct, es, KeyKind::Raw);
            d.ct = (d.c.hmput_default)(d.ct, es);
            d.rt = (d.r.hmput_default)(d.rt, es);
            let after_c = snapshot_map(d.ct, es, KeyKind::Raw);
            diff_eq_val("row23 truly unchanged", before_c, after_c);
            d.check("row23");
            d.free();
        }
    }
}

#[test]
fn row22_hmput_default_on_length_zero_array() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        pin_seed(&c, &r, 7);
        for &(es, _ks) in &SHAPES {
            // A raw array straight out of arrgrowf has length == 0; converting it
            // to a "map pointer" and calling hmput_default hits the
            // `stbds_header(HASH_TO_ARR(a))->length == 0` branch.
            let ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let ct = (c.hmput_default)(arr_to_hash(ca, es), es);
            let rt = (r.hmput_default)(arr_to_hash(ra, es), es);
            diff_eq(
                &format!("row22 es={es}"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
            (c.hmfree_func)(hash_to_arr(ct, es), es);
            (r.hmfree_func)(hash_to_arr(rt, es), es);
        }
    }
}

// ---------------------------------------------------------- rows 23..24
#[test]
fn row23_24_get_on_tableless_maps() {
    let _g = global_lock();
    for &(es, ks) in &SHAPES {
        let cfg = MapCfg::binary(es, ks);
        // (a) get on a completely NULL map
        let mut d = Duo::new(cfg, 0xabc);
        unsafe {
            let i = d.get(1, "row23 null-map");
            assert_eq!(i, -1, "C must report STBDS_INDEX_EMPTY");
            d.free();
        }
        // (b) get_ts on a completely NULL map
        let mut d = Duo::new(cfg, 0xabc);
        unsafe {
            let i = d.get_ts(1, "row23 null-map ts");
            assert_eq!(i, -1);
            d.free();
        }
        // (c) map with a default element but no hash table at all
        let mut d = Duo::new(cfg, 0xabc);
        unsafe {
            d.put_default(0x11, "row24 setup");
            let i = d.get(42, "row24 no-table");
            assert_eq!(i, -1);
            let i = d.get_ts(42, "row24 no-table ts");
            assert_eq!(i, -1);
            d.free();
        }
    }
}

// ---------------------------------------------------------- rows 25..33
fn insert_sweep(tag: &str, counts: &[u64], shapes: &[(usize, usize)]) {
    for &(es, ks) in shapes {
        for &n in counts {
            let mut d = Duo::new(MapCfg::binary(es, ks), 0x31415926);
            unsafe {
                d.put_default(0xEE, &format!("{tag} es={es} ks={ks} n={n} setup"));
                for k in 0..n {
                    d.put(k, &format!("{tag} es={es} ks={ks} n={n}"));
                }
                // read every inserted key back plus misses
                for k in 0..n {
                    let i = d.get(k, &format!("{tag} readback es={es} ks={ks} n={n}"));
                    assert!(i >= 0, "inserted key {k} must be found");
                }
                for k in n..(n + 8) {
                    let i = d.get(k, &format!("{tag} miss es={es} ks={ks} n={n}"));
                    assert_eq!(i, -1, "absent key {k} must report -1");
                }
                d.free();
            }
        }
    }
}

#[test]
fn row25_inserts_across_first_rehash() {
    let _g = global_lock();
    insert_sweep("row25", &[0, 1, 2, 3, 4, 5, 6, 7, 8], &[(8, 4)]);
}

#[test]
fn row26_inserts_across_second_rehash() {
    let _g = global_lock();
    insert_sweep("row26", &[9, 10, 11, 12, 13, 14, 24, 25], &[(8, 4)]);
}

#[test]
fn row27_many_inserts_repeated_rehash() {
    let _g = global_lock();
    insert_sweep("row27", &[50, 100, 300], &[(8, 4)]);
}

#[test]
fn row28_33_all_element_shapes() {
    let _g = global_lock();
    // keysize 1/2 have a tiny key space, so keep the counts below it
    insert_sweep("row33-tiny", &[0, 1, 2, 5, 6, 7, 13, 60], &[(2, 2), (1, 1)]);
    insert_sweep(
        "row28-32",
        &[0, 1, 5, 6, 7, 12, 13, 40],
        &[(8, 8), (16, 8), (16, 4), (24, 16), (32, 32), (4, 4), (64, 8)],
    );
}

// --------------------------------------------------------------- row 34
#[test]
fn row34_duplicate_inserts_update_in_place() {
    let _g = global_lock();
    for &(es, ks) in &SHAPES {
        let mut d = Duo::new(MapCfg::binary(es, ks), 5);
        unsafe {
            d.put_default(0x01, "row34 setup");
            for k in 0..20u64 {
                d.put(k, "row34 first");
            }
            let len_before = hm_len(d.ct, es);
            for k in 0..20u64 {
                d.put(k, "row34 duplicate");
            }
            diff_eq_val("row34 length unchanged", len_before, hm_len(d.ct, es));
            diff_eq_val("row34 rust length", hm_len(d.ct, es), hm_len(d.rt, es));
            // and again in reverse order (hits the wrap-around inner scan too)
            for k in (0..20u64).rev() {
                d.put(k, "row34 duplicate rev");
            }
            d.free();
        }
    }
}

// --------------------------------------------------------------- row 35
#[test]
fn row35_get_key_vs_get_key_ts() {
    let _g = global_lock();
    let mut rng = Rng::new(35);
    for &(es, ks) in &[(8usize, 4usize), (16, 8), (24, 16)] {
        let mut d = Duo::new(MapCfg::binary(es, ks), 0x77);
        unsafe {
            d.put_default(0x02, "row35 setup");
            for k in 0..40u64 {
                d.put(k * 3, "row35 setup");
            }
            for _ in 0..600 {
                let k = rng.below(140);
                let a = d.get(k, "row35 get");
                let b = d.get_ts(k, "row35 get_ts");
                diff_eq_val("row35 get vs get_ts agree", a, b);
            }
            d.free();
        }
    }
}

// ---------------------------------------------------------- rows 36..42
#[test]
fn row36_delete_absent_key() {
    let _g = global_lock();
    for &(es, ks) in &SHAPES {
        let mut d = Duo::new(MapCfg::binary(es, ks), 9);
        unsafe {
            // (a) delete from a NULL map -> returns NULL
            let mut key = key_bytes(3, ks);
            let (ct, _) = map_del(&d.c, d.ct, &d.cfg, &mut key);
            let mut key = key_bytes(3, ks);
            let (rt, _) = map_del(&d.r, d.rt, &d.cfg, &mut key);
            diff_eq_val("row36 del(NULL) is NULL", ct.is_null(), rt.is_null());
            assert!(ct.is_null());

            // (b) delete a key that was never inserted
            d.put_default(0x03, "row36 setup");
            for k in 0..10u64 {
                d.put(k, "row36 setup");
            }
            let len = hm_len(d.ct, es);
            for k in 100..110u64 {
                let rv = d.del(k, "row36 absent");
                diff_eq_val("row36 absent -> 0", rv, 0);
            }
            diff_eq_val("row36 length untouched", len, hm_len(d.ct, es));
            d.free();
        }
    }
}

#[test]
fn row37_delete_last_element() {
    let _g = global_lock();
    for &(es, ks) in &SHAPES {
        for &n in &[1u64, 2, 5, 6, 7, 13, 30] {
            let mut d = Duo::new(MapCfg::binary(es, ks), 0x1234);
            unsafe {
                d.put_default(0x04, "row37 setup");
                for k in 0..n {
                    d.put(k, "row37 setup");
                }
                // delete in reverse insertion order: always old_index==final_index
                for k in (0..n).rev() {
                    let rv = d.del(k, &format!("row37 n={n} last"));
                    diff_eq_val("row37 del hit -> 1", rv, 1);
                }
                d.free();
            }
        }
    }
}

#[test]
fn row38_delete_middle_element() {
    let _g = global_lock();
    for &(es, ks) in &SHAPES {
        for &n in &[2u64, 3, 6, 7, 13, 30] {
            let mut d = Duo::new(MapCfg::binary(es, ks), 0x1234);
            unsafe {
                d.put_default(0x05, "row38 setup");
                for k in 0..n {
                    d.put(k, "row38 setup");
                }
                // forward order: the first delete always moves the last element
                for k in 0..n {
                    d.del(k, &format!("row38 n={n} middle"));
                    // every surviving key must still be reachable
                    for j in (k + 1)..n {
                        let i = d.get(j, &format!("row38 n={n} survivor"));
                        assert!(i >= 0, "key {j} lost after deleting {k}");
                    }
                }
                d.free();
            }
        }
    }
}

#[test]
fn row39_41_shrink_and_tombstone_rebuild() {
    let _g = global_lock();
    // n=7  -> 16 slots; deleting down to used_count<4 shrinks to 8
    // n=13 -> 32 slots; deleting further shrinks 32->16->8
    for &n in &[6u64, 7, 12, 13, 25, 30, 60] {
        for &(es, ks) in &[(8usize, 4usize), (16, 8), (24, 16)] {
            let mut d = Duo::new(MapCfg::binary(es, ks), 0xfeed);
            unsafe {
                let mut max_slots = 0usize;
                let mut max_tombs = 0usize;
                let mut shrinks = 0usize;
                let mut prev_slots = 0usize;
                let mut track = |d: &Duo| {
                    if let Some(t) = map_table(d.ct, es) {
                        max_slots = max_slots.max(t.slot_count);
                        max_tombs = max_tombs.max(t.tombstone_count);
                        if prev_slots != 0 && t.slot_count < prev_slots {
                            shrinks += 1;
                        }
                        prev_slots = t.slot_count;
                    }
                };
                d.put_default(0x06, "row39 setup");
                for k in 0..n {
                    d.put(k, &format!("row39 n={n} setup"));
                    track(&d);
                }
                // delete every other key first (creates tombstones), then the rest
                for k in (0..n).step_by(2) {
                    d.del(k, &format!("row39 n={n} evens"));
                    track(&d);
                }
                for k in (1..n).step_by(2) {
                    d.del(k, &format!("row39 n={n} odds"));
                    track(&d);
                }
                diff_eq_val("row39 empty", hm_len(d.ct, es), 0);
                diff_eq_val("row39 empty rust", hm_len(d.rt, es), 0);
                // --- coverage assertions: prove the paths were really taken ---
                // rehash fires on the insert where used_count reaches
                // used_count_threshold = slot_count - slot_count/4:
                //   8 slots -> thr 6  -> 7th insert  => 16 slots
                //  16 slots -> thr 12 -> 13th insert => 32 slots
                //  32 slots -> thr 24 -> 25th insert => 64 slots
                //  64 slots -> thr 48 -> 49th insert => 128 slots
                if n >= 7 {
                    assert!(max_slots >= 16, "n={n}: rehash 8->16 never happened");
                    assert!(shrinks > 0, "n={n}: shrink path never taken");
                    assert_eq!(prev_slots, 8, "n={n}: table did not shrink back to 8 slots");
                }
                if n >= 13 {
                    assert!(max_slots >= 32, "n={n}: rehash 16->32 never happened");
                }
                if n >= 25 {
                    assert!(max_slots >= 64, "n={n}: rehash 32->64 never happened");
                }
                if n >= 49 {
                    assert!(max_slots >= 128, "n={n}: rehash 64->128 never happened");
                }
                assert!(max_tombs > 0, "n={n}: no tombstone was ever created");
                println!(
                    "row39 n={n} es={es}: max_slots={max_slots} max_tombs={max_tombs} shrinks={shrinks}"
                );
                d.free();
            }
        }
    }
}

#[test]
fn row42_reinsert_into_tombstones() {
    let _g = global_lock();
    for &(es, ks) in &[(8usize, 4usize), (16, 8)] {
        let mut d = Duo::new(MapCfg::binary(es, ks), 0x4242);
        unsafe {
            d.put_default(0x07, "row42 setup");
            for round in 0..6 {
                for k in 0..20u64 {
                    d.put(k, &format!("row42 r={round} put"));
                }
                for k in 0..20u64 {
                    d.del(k, &format!("row42 r={round} del"));
                }
            }
            d.free();
        }
    }
}

// --------------------------------------------------------------- row 43
#[test]
fn row43_randomized_mixed_pipeline() {
    let _g = global_lock();
    for &(es, ks) in &[(8usize, 4usize), (16, 8), (24, 16)] {
        let mut rng = Rng::new(43 + es as u64);
        let mut d = Duo::new(MapCfg::binary(es, ks), 0xdeadbeef);
        unsafe {
            let mut live: std::collections::BTreeSet<u64> = Default::default();
            for step in 0..4000u32 {
                let ctx = format!("row43 es={es} ks={ks} step={step}");
                // small key space -> heavy collisions, tombstones, grows, shrinks
                let k = rng.below(24);
                match rng.below(10) {
                    0..=3 => {
                        d.put(k, &ctx);
                        live.insert(k);
                    }
                    4..=5 => {
                        let i = d.get(k, &ctx);
                        diff_eq_val(&format!("{ctx} presence"), i >= 0, live.contains(&k));
                    }
                    6 => {
                        let i = d.get_ts(k, &ctx);
                        diff_eq_val(&format!("{ctx} presence ts"), i >= 0, live.contains(&k));
                    }
                    7..=8 => {
                        let rv = d.del(k, &ctx);
                        diff_eq_val(
                            &format!("{ctx} del result"),
                            rv,
                            if live.remove(&k) { 1 } else { 0 },
                        );
                    }
                    _ => {
                        d.put_default(rng.byte(), &ctx);
                    }
                }
            }
            d.free();
        }
    }
}

// --------------------------------------------------------------- row 70
/// Insert-driven rehashes always land ~3 entries per 8-slot bucket, which is too
/// sparse for `stbds_make_hash_index`'s *quadratic* probe (`pos += step;
/// step += STBDS_BUCKET_LENGTH`) to ever need more than one extra step.  The
/// tombstone rebuild (`make_hash_index(slot_count, table)`) instead re-inserts a
/// nearly-full table into the same number of slots (~6 entries per bucket), so
/// buckets do overflow and the probe chain is walked several times.  This row
/// drives exactly that: fill a large table, then churn `delete one / insert a
/// brand new one` so that `tombstone_count` climbs past its threshold while
/// `used_count` stays high.
#[test]
fn row70_high_load_tombstone_rebuild_probing() {
    let _g = global_lock();
    for &(es, ks) in &[(8usize, 4usize), (16, 8)] {
        let mut d = Duo::new(MapCfg::binary(es, ks), 0x7010);
        unsafe {
            let n0: u64 = 760; // -> 1024 slots, used_count_threshold 768
            d.put_default(0x70, "row70 setup");
            for k in 0..n0 {
                let key = key_bytes(k, ks);
                let pl = payload_bytes(k, es.saturating_sub(ks));
                d.ct = map_put_binary(&d.c, d.ct, &d.cfg, &key, &pl);
                d.rt = map_put_binary(&d.r, d.rt, &d.cfg, &key, &pl);
            }
            d.check("row70 after bulk fill");
            let ti = map_table(d.ct, es).unwrap();
            println!(
                "row70 es={es}: slots={} used={} used_thr={} tomb_thr={}",
                ti.slot_count, ti.used_count, ti.used_count_threshold, ti.tombstone_count_threshold
            );
            assert!(ti.slot_count >= 1024, "table too small: {}", ti.slot_count);

            let mut rebuilds = 0usize;
            let mut max_tombs = 0usize;
            let mut max_used = ti.used_count;
            for i in 0..600u64 {
                let pre = map_table(d.ct, es).unwrap();
                // delete an existing key
                let mut key = key_bytes(i, ks);
                let (nct, cd) = map_del(&d.c, d.ct, &d.cfg, &mut key);
                let mut key = key_bytes(i, ks);
                let (nrt, rd) = map_del(&d.r, d.rt, &d.cfg, &mut key);
                d.ct = nct;
                d.rt = nrt;
                diff_eq_val(&format!("row70 del({i})"), cd, rd);
                // insert a brand-new key (does not usually land on the tombstone)
                let nk = 100_000 + i;
                let key = key_bytes(nk, ks);
                let pl = payload_bytes(nk, es.saturating_sub(ks));
                d.ct = map_put_binary(&d.c, d.ct, &d.cfg, &key, &pl);
                d.rt = map_put_binary(&d.r, d.rt, &d.cfg, &key, &pl);

                let post = map_table(d.ct, es).unwrap();
                if post.tombstone_count < pre.tombstone_count && post.slot_count == pre.slot_count {
                    rebuilds += 1;
                }
                max_tombs = max_tombs.max(post.tombstone_count);
                max_used = max_used.max(post.used_count);
                if i % 5 == 0 {
                    d.check(&format!("row70 churn i={i}"));
                }
            }
            d.check("row70 after churn");
            println!(
                "row70 es={es}: rebuilds={rebuilds} max_tombs={max_tombs} max_used={max_used}"
            );
            assert!(rebuilds > 0, "no tombstone rebuild happened at high load");
            // read every surviving key back
            for k in 600..n0 {
                let mut key = key_bytes(k, ks);
                let (nct, ci) = map_geti(&d.c, d.ct, &d.cfg, &mut key);
                let mut key = key_bytes(k, ks);
                let (nrt, ri) = map_geti(&d.r, d.rt, &d.cfg, &mut key);
                d.ct = nct;
                d.rt = nrt;
                diff_eq_val(&format!("row70 readback({k})"), ci, ri);
                assert!(ci >= 0, "key {k} lost");
            }
            for i in 0..600u64 {
                let mut key = key_bytes(100_000 + i, ks);
                let (nct, ci) = map_geti(&d.c, d.ct, &d.cfg, &mut key);
                let mut key = key_bytes(100_000 + i, ks);
                let (nrt, ri) = map_geti(&d.r, d.rt, &d.cfg, &mut key);
                d.ct = nct;
                d.rt = nrt;
                diff_eq_val(&format!("row70 readback-new({i})"), ci, ri);
                assert!(ci >= 0, "new key {} lost", 100_000 + i);
            }
            d.check("row70 final");
            d.free();
        }
    }
}

/// Row 70b: the same high-load rebuild driven under many different global seeds,
/// so the bucket occupancy distribution (and therefore the length of the probe
/// chains taken inside `stbds_make_hash_index`) varies a lot.
#[test]
fn row70b_high_load_rebuild_many_seeds() {
    let _g = global_lock();
    let es = 8usize;
    let ks = 4usize;
    for seed in 0..12u64 {
        let mut d = Duo::new(MapCfg::binary(es, ks), 0x9000 + seed as usize * 7919);
        unsafe {
            d.put_default(0x71, "row70b setup");
            for k in 0..380u64 {
                let key = key_bytes(k, ks);
                let pl = payload_bytes(k, es - ks);
                d.ct = map_put_binary(&d.c, d.ct, &d.cfg, &key, &pl);
                d.rt = map_put_binary(&d.r, d.rt, &d.cfg, &key, &pl);
            }
            for i in 0..300u64 {
                let mut key = key_bytes(i, ks);
                let (nct, cd) = map_del(&d.c, d.ct, &d.cfg, &mut key);
                let mut key = key_bytes(i, ks);
                let (nrt, rd) = map_del(&d.r, d.rt, &d.cfg, &mut key);
                d.ct = nct;
                d.rt = nrt;
                diff_eq_val(&format!("row70b seed={seed} del({i})"), cd, rd);
                let nk = 50_000 + i;
                let key = key_bytes(nk, ks);
                let pl = payload_bytes(nk, es - ks);
                d.ct = map_put_binary(&d.c, d.ct, &d.cfg, &key, &pl);
                d.rt = map_put_binary(&d.r, d.rt, &d.cfg, &key, &pl);
                if i % 17 == 0 {
                    d.check(&format!("row70b seed={seed} i={i}"));
                }
            }
            d.check(&format!("row70b seed={seed} final"));
            d.free();
        }
    }
}

// --------------------------------------------------------------- row 44
#[test]
fn row44_hmdel_key_nonzero_keyoffset() {
    let _g = global_lock();
    // `stbds_hmput_key` always uses keyoffset 0, so a non-zero keyoffset in
    // `stbds_hmdel_key` compares the wrong bytes of the element.  The result
    // must still be bit-identical between the two libraries.  All offsets stay
    // *inside* the element so only bytes the test itself wrote are read.
    for &ko in &[0usize, 4, 8, 12] {
        let mut cfg = MapCfg::binary(16, 4);
        cfg.del_keyoffset = ko;
        let mut d = Duo::new(cfg, 0x5150);
        unsafe {
            d.put_default(0x08, &format!("row44 ko={ko} setup"));
            for k in 0..24u64 {
                d.put(k, &format!("row44 ko={ko} setup"));
            }
            for k in 0..24u64 {
                d.del(k, &format!("row44 ko={ko}"));
            }
            for k in 0..24u64 {
                d.get(k, &format!("row44 ko={ko} after"));
            }
            d.free();
        }
    }
}

// ---------------------------------------------------------- rows 45..46
#[test]
fn row45_46_hmfree_func_variants() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &(es, ks) in &SHAPES {
            // row 45(a): table present, length 1 (default element only)
            pin_seed(&c, &r, 0x99);
            let cfg = MapCfg::binary(es, ks);
            let key = key_bytes(1, ks);
            let pl = payload_bytes(1, es.saturating_sub(ks));
            let ct = map_put_binary(&c, std::ptr::null_mut(), &cfg, &key, &pl);
            let rt = map_put_binary(&r, std::ptr::null_mut(), &cfg, &key, &pl);
            diff_eq(
                &format!("row45 es={es} pre-free"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
            (c.hmfree_func)(hash_to_arr(ct, es), es);
            (r.hmfree_func)(hash_to_arr(rt, es), es);

            // row 45(b): table present, many elements
            pin_seed(&c, &r, 0x99);
            let mut ct: *mut c_void = std::ptr::null_mut();
            let mut rt: *mut c_void = std::ptr::null_mut();
            for k in 0..30u64 {
                let key = key_bytes(k, ks);
                let pl = payload_bytes(k, es.saturating_sub(ks));
                ct = map_put_binary(&c, ct, &cfg, &key, &pl);
                rt = map_put_binary(&r, rt, &cfg, &key, &pl);
            }
            (c.hmfree_func)(hash_to_arr(ct, es), es);
            (r.hmfree_func)(hash_to_arr(rt, es), es);

            // row 46: default element but no hash table at all
            let ct = (c.hmput_default)(std::ptr::null_mut(), es);
            let rt = (r.hmput_default)(std::ptr::null_mut(), es);
            diff_eq(
                &format!("row46 es={es}"),
                &snapshot_map(ct, es, KeyKind::Raw),
                &snapshot_map(rt, es, KeyKind::Raw),
            );
            (c.hmfree_func)(hash_to_arr(ct, es), es);
            (r.hmfree_func)(hash_to_arr(rt, es), es);
        }
    }
}

// --------------------------------------------------------------- row 47
#[test]
fn row47_binary_mode_classification() {
    let _g = global_lock();
    // Any `mode <= 0` must behave exactly like STBDS_HM_BINARY (0).
    let reference: Vec<String> = {
        let mut cfg = MapCfg::binary(16, 8);
        cfg.mode = STBDS_HM_BINARY;
        let mut d = Duo::new(cfg, 0x2024);
        let mut out = Vec::new();
        unsafe {
            d.put_default(0x09, "row47 ref setup");
            for k in 0..25u64 {
                d.put(k, "row47 ref");
                out.push(snapshot_map(d.ct, 16, KeyKind::Raw));
            }
            for k in 0..25u64 {
                d.del(k, "row47 ref del");
                out.push(snapshot_map(d.ct, 16, KeyKind::Raw));
            }
            d.free();
        }
        out
    };

    for &mode in &[0 as c_int, -1, -2, -1000, c_int::MIN] {
        let mut cfg = MapCfg::binary(16, 8);
        cfg.mode = mode;
        let mut d = Duo::new(cfg, 0x2024);
        let mut out = Vec::new();
        unsafe {
            d.put_default(0x09, &format!("row47 mode={mode} setup"));
            for k in 0..25u64 {
                d.put(k, &format!("row47 mode={mode}"));
                out.push(snapshot_map(d.ct, 16, KeyKind::Raw));
            }
            for k in 0..25u64 {
                d.del(k, &format!("row47 mode={mode} del"));
                out.push(snapshot_map(d.ct, 16, KeyKind::Raw));
            }
            d.free();
        }
        assert_eq!(out.len(), reference.len());
        for (i, (a, b)) in reference.iter().zip(out.iter()).enumerate() {
            diff_eq(&format!("row47 mode={mode} step {i} vs mode=0"), a, b);
        }
    }
}
