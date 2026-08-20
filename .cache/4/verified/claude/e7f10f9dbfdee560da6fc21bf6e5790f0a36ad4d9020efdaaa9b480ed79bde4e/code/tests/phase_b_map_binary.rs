//! Phase B — CONFIGS.md rows 24-34, 43-45, 47-55, 71.
//! Binary-key hash maps driven through `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmdel_key`, `stbds_hmfree_func`.
mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// Drives the same operation on both libraries and compares the complete
/// internal state after every single call.
struct Dual<'a> {
    s: &'a Session,
    lay: Layout,
    c: *mut c_void,
    r: *mut c_void,
    label: String,
    step: usize,
}

impl<'a> Dual<'a> {
    fn new(s: &'a Session, lay: Layout, label: &str) -> Dual<'a> {
        Dual {
            s,
            lay,
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            label: label.to_string(),
            step: 0,
        }
    }

    fn from_shmode(s: &'a Session, lay: Layout, label: &str, sh_mode: c_int) -> Dual<'a> {
        unsafe {
            Dual {
                s,
                lay,
                c: (s.c.shmode_func)(lay.elemsize, sh_mode),
                r: (s.rust.shmode_func)(lay.elemsize, sh_mode),
                label: label.to_string(),
                step: 0,
            }
        }
    }

    fn opts(&self) -> DumpOpts {
        DumpOpts::raw(self.lay.elemsize)
    }

    #[track_caller]
    fn check(&mut self, what: &str) {
        unsafe {
            let c = dump_map(self.c, self.opts());
            let r = dump_map(self.r, self.opts());
            assert_same(
                &format!("{} [{} step {}] {}", self.label, self.lay.name, self.step, what),
                &c,
                &r,
            );
        }
        self.step += 1;
    }

    fn put(&mut self, key: &[u8], val: &[u8], mode: c_int) {
        unsafe {
            self.c = map_put_binary(self.s.c, self.c, self.lay, key, val, mode);
            self.r = map_put_binary(self.s.rust, self.r, self.lay, key, val, mode);
        }
        self.check(&format!("put({:?})", key));
    }

    fn get(&mut self, key: &[u8], mode: c_int) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        unsafe {
            let (c, ci) = map_geti(self.s.c, self.c, self.lay, kc.as_mut_ptr() as *mut c_void, mode);
            let (r, ri) =
                map_geti(self.s.rust, self.r, self.lay, kr.as_mut_ptr() as *mut c_void, mode);
            self.c = c;
            self.r = r;
            assert_eq!(
                ci, ri,
                "{} [{}] hmget_key({:?}) index differs (C={} RUST={})",
                self.label, self.lay.name, key, ci, ri
            );
            self.check(&format!("get({:?})->{}", key, ci));
            ci
        }
    }

    fn get_ts(&mut self, key: &[u8], mode: c_int) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        unsafe {
            let (c, ct, chdr) =
                map_geti_ts(self.s.c, self.c, self.lay, kc.as_mut_ptr() as *mut c_void, mode);
            let (r, rt, rhdr) = map_geti_ts(
                self.s.rust,
                self.r,
                self.lay,
                kr.as_mut_ptr() as *mut c_void,
                mode,
            );
            self.c = c;
            self.r = r;
            assert_eq!(
                ct, rt,
                "{} [{}] hmget_key_ts({:?}) *temp differs (C={} RUST={})",
                self.label, self.lay.name, key, ct, rt
            );
            assert_eq!(
                chdr, rhdr,
                "{} [{}] hmget_key_ts({:?}) header temp differs (C={} RUST={})",
                self.label, self.lay.name, key, chdr, rhdr
            );
            self.check(&format!("get_ts({:?})->{}", key, ct));
            ct
        }
    }

    fn del(&mut self, key: &[u8], keyoffset: usize, mode: c_int) -> isize {
        let mut kc = key.to_vec();
        let mut kr = key.to_vec();
        unsafe {
            let (c, ct) = map_del(
                self.s.c,
                self.c,
                self.lay,
                kc.as_mut_ptr() as *mut c_void,
                keyoffset,
                mode,
            );
            let (r, rt) = map_del(
                self.s.rust,
                self.r,
                self.lay,
                kr.as_mut_ptr() as *mut c_void,
                keyoffset,
                mode,
            );
            assert_eq!(
                c.is_null(),
                r.is_null(),
                "{} hmdel_key nullness differs",
                self.label
            );
            self.c = c;
            self.r = r;
            assert_eq!(
                ct, rt,
                "{} [{}] hmdel_key({:?}, keyoffset={}) temp differs (C={} RUST={})",
                self.label, self.lay.name, key, keyoffset, ct, rt
            );
            self.check(&format!("del({:?},ko={})->{}", key, keyoffset, ct));
            ct
        }
    }

    fn free(self) {
        unsafe {
            map_free(self.s.c, self.c, self.lay);
            map_free(self.s.rust, self.r, self.lay);
        }
    }
}

fn mk_key(rng: &mut Rng, lay: Layout) -> Vec<u8> {
    rng.bytes(lay.keysize)
}

fn mk_val(rng: &mut Rng, lay: Layout) -> Vec<u8> {
    rng.bytes(lay.elemsize - lay.keysize)
}

/// Distinct keys (unique in the `keysize`-byte sense).
fn distinct_keys(rng: &mut Rng, lay: Layout, n: usize) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    if lay.keysize == 1 {
        // only 256 possible values
        let mut vals: Vec<u8> = (0u8..=255).collect();
        for i in (1..vals.len()).rev() {
            let j = rng.below(i + 1);
            vals.swap(i, j);
        }
        for i in 0..n.min(256) {
            out.push(vec![vals[i]]);
        }
        return out;
    }
    while out.len() < n {
        let k = mk_key(rng, lay);
        if !out.contains(&k) {
            out.push(k);
        }
    }
    out
}

// --- row 24 --------------------------------------------------------------
#[test]
fn cfg_24_binary_first_insert() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 24);
    for &lay in BINARY_LAYOUTS.iter() {
        for _ in 0..20 {
            let mut d = Dual::new(&s, lay, "first-insert");
            let k = mk_key(&mut rng, lay);
            let v = mk_val(&mut rng, lay);
            d.put(&k, &v, HM_BINARY);
            d.free();
        }
    }
}

// --- rows 25/26/27 -------------------------------------------------------
#[test]
fn cfg_25_26_27_binary_grow_ladder() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 25);
    for &lay in BINARY_LAYOUTS.iter() {
        for n in [1usize, 2, 3, 4, 5, 6, 7, 8, 12, 13, 14, 25, 50, 100] {
            let keys = distinct_keys(&mut rng, lay, n);
            let mut d = Dual::new(&s, lay, &format!("grow n={}", n));
            for k in keys.iter() {
                let v = mk_val(&mut rng, lay);
                d.put(k, &v, HM_BINARY);
            }
            // every inserted key must be findable
            for k in keys.iter() {
                let i = d.get(k, HM_BINARY);
                assert!(i >= 0, "key {:?} not found in a map of {}", k, n);
            }
            d.free();
        }
    }
}

#[test]
fn cfg_27_binary_large_maps() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 27);
    for &lay in [L_I2I, L_U2U, L_S1].iter() {
        for n in [200usize, 1000] {
            let keys = distinct_keys(&mut rng, lay, n);
            let mut d = Dual::new(&s, lay, &format!("large n={}", n));
            // only compare the full state every 32 puts (the dumps are big)
            unsafe {
                for (i, k) in keys.iter().enumerate() {
                    let v = mk_val(&mut rng, lay);
                    d.c = map_put_binary(s.c, d.c, lay, k, &v, HM_BINARY);
                    d.r = map_put_binary(s.rust, d.r, lay, k, &v, HM_BINARY);
                    if i % 32 == 0 || i + 1 == keys.len() {
                        d.check(&format!("bulk put #{}", i));
                    }
                }
            }
            for k in keys.iter() {
                let i = d.get(k, HM_BINARY);
                assert!(i >= 0);
            }
            // absent keys
            for _ in 0..200 {
                let k = mk_key(&mut rng, lay);
                if keys.contains(&k) {
                    continue;
                }
                assert_eq!(d.get(&k, HM_BINARY), -1);
            }
            d.free();
        }
    }
}

// --- row 28 --------------------------------------------------------------
#[test]
fn cfg_28_binary_duplicate_keys() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 28);
    for &lay in BINARY_LAYOUTS.iter() {
        let keys = distinct_keys(&mut rng, lay, 40);
        let mut d = Dual::new(&s, lay, "duplicates");
        for k in keys.iter() {
            let v = mk_val(&mut rng, lay);
            d.put(k, &v, HM_BINARY);
        }
        // 400 random re-puts of already-present keys, interleaved with new ones
        for i in 0..400 {
            if i % 7 == 0 {
                let k = mk_key(&mut rng, lay);
                let v = mk_val(&mut rng, lay);
                d.put(&k, &v, HM_BINARY);
            } else {
                let k = keys[rng.below(keys.len())].clone();
                let v = mk_val(&mut rng, lay);
                d.put(&k, &v, HM_BINARY);
            }
        }
        d.free();
    }
}

// --- rows 29-34: every layout across the grow boundary -------------------
#[test]
fn cfg_29_34_all_binary_layouts() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 29);
    for &lay in BINARY_LAYOUTS.iter() {
        for n in [1usize, 6, 7, 13, 30] {
            let keys = distinct_keys(&mut rng, lay, n);
            let mut d = Dual::new(&s, lay, &format!("layout n={}", n));
            for k in keys.iter() {
                let v = mk_val(&mut rng, lay);
                d.put(k, &v, HM_BINARY);
            }
            for k in keys.iter() {
                assert!(d.get(k, HM_BINARY) >= 0);
                assert!(d.get_ts(k, HM_BINARY) >= 0);
            }
            // delete every second key, then look everything up again
            for (i, k) in keys.iter().enumerate() {
                if i % 2 == 0 {
                    d.del(k, 0, HM_BINARY);
                }
            }
            for (i, k) in keys.iter().enumerate() {
                let got = d.get(k, HM_BINARY);
                if i % 2 == 0 {
                    assert_eq!(got, -1, "deleted key {:?} still present", k);
                } else {
                    assert!(got >= 0, "surviving key {:?} lost", k);
                }
            }
            d.free();
        }
    }
}

// --- row 32: keysize 1 collision pressure --------------------------------
#[test]
fn cfg_32_keysize_one_collision_pressure() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 32);
    let lay = L_B1;
    let mut d = Dual::new(&s, lay, "keysize1");
    for _ in 0..400 {
        let k = vec![(rng.next_u64() & 0xFF) as u8];
        let v = mk_val(&mut rng, lay);
        d.put(&k, &v, HM_BINARY);
    }
    for b in 0u8..=255 {
        d.get(&[b], HM_BINARY);
        d.get_ts(&[b], HM_BINARY);
    }
    for b in 0u8..=255 {
        d.del(&[b], 0, HM_BINARY);
    }
    d.free();
}

// --- row 43 --------------------------------------------------------------
#[test]
fn cfg_43_hmget_key_on_null() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 43);
    for &lay in BINARY_LAYOUTS.iter() {
        for mode in [HM_BINARY, -1, i32::MIN] {
            let mut d = Dual::new(&s, lay, "get-null");
            let k = mk_key(&mut rng, lay);
            let idx = d.get(&k, mode);
            assert_eq!(idx, -1, "hmget_key(NULL) must yield temp == -1");
            assert!(!d.c.is_null() && !d.r.is_null());
            d.free();
        }
    }
}

// --- row 44 --------------------------------------------------------------
#[test]
fn cfg_44_hmget_key_without_hash_table() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 44);
    for &lay in BINARY_LAYOUTS.iter() {
        unsafe {
            let mut d = Dual::new(&s, lay, "get-no-table");
            d.c = (s.c.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            d.r = (s.rust.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            d.check("after hmput_default");
            for _ in 0..20 {
                let k = mk_key(&mut rng, lay);
                assert_eq!(d.get(&k, HM_BINARY), -1);
                assert_eq!(d.get_ts(&k, HM_BINARY), -1);
            }
            d.free();
        }
    }
}

// --- rows 45/47: present + absent lookups at every slot_count ------------
#[test]
fn cfg_45_47_lookups_at_all_table_sizes() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 45);
    let lay = L_S2;
    // 1 -> 6 -> 12 -> 24 -> 48 -> 96 entries drives slot_count 8..256
    for n in [1usize, 6, 12, 24, 48, 96] {
        let keys = distinct_keys(&mut rng, lay, n);
        let mut d = Dual::new(&s, lay, &format!("lookup n={}", n));
        for k in keys.iter() {
            let v = mk_val(&mut rng, lay);
            d.put(k, &v, HM_BINARY);
        }
        for k in keys.iter() {
            assert!(d.get(k, HM_BINARY) >= 0);
            assert!(d.get_ts(k, HM_BINARY) >= 0);
        }
        for _ in 0..100 {
            let k = mk_key(&mut rng, lay);
            if keys.contains(&k) {
                continue;
            }
            assert_eq!(d.get(&k, HM_BINARY), -1);
            assert_eq!(d.get_ts(&k, HM_BINARY), -1);
        }
        d.free();
    }
}

// --- row 48 --------------------------------------------------------------
#[test]
fn cfg_48_hmdel_on_null() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 48);
    for &lay in BINARY_LAYOUTS.iter() {
        for mode in [HM_BINARY, HM_STRING, -1, 2, i32::MAX, i32::MIN] {
            let mut k = mk_key(&mut rng, lay);
            unsafe {
                let c = (s.c.hmdel_key)(
                    std::ptr::null_mut(),
                    lay.elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    lay.keysize,
                    0,
                    mode,
                );
                let r = (s.rust.hmdel_key)(
                    std::ptr::null_mut(),
                    lay.elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    lay.keysize,
                    0,
                    mode,
                );
                assert!(c.is_null(), "C hmdel_key(NULL, mode={}) must return NULL", mode);
                assert!(r.is_null(), "RUST hmdel_key(NULL, mode={}) must return NULL", mode);
            }
        }
    }
}

// --- row 49 --------------------------------------------------------------
#[test]
fn cfg_49_hmdel_without_hash_table() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 49);
    for &lay in BINARY_LAYOUTS.iter() {
        unsafe {
            let mut d = Dual::new(&s, lay, "del-no-table");
            d.c = (s.c.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            d.r = (s.rust.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            for _ in 0..10 {
                let k = mk_key(&mut rng, lay);
                assert_eq!(d.del(&k, 0, HM_BINARY), 0);
            }
            d.free();
        }
    }
}

// --- row 50 --------------------------------------------------------------
#[test]
fn cfg_50_hmdel_absent_key() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 50);
    for &lay in BINARY_LAYOUTS.iter() {
        let keys = distinct_keys(&mut rng, lay, 20);
        let mut d = Dual::new(&s, lay, "del-absent");
        for k in keys.iter() {
            let v = mk_val(&mut rng, lay);
            d.put(k, &v, HM_BINARY);
        }
        for _ in 0..100 {
            let k = mk_key(&mut rng, lay);
            if keys.contains(&k) {
                continue;
            }
            assert_eq!(d.del(&k, 0, HM_BINARY), 0, "absent delete must set temp = 0");
        }
        d.free();
    }
}

// --- rows 51/52 ----------------------------------------------------------
#[test]
fn cfg_51_52_hmdel_positions() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 51);
    for &lay in BINARY_LAYOUTS.iter() {
        for n in [1usize, 2, 3, 7, 13, 40] {
            for which in 0..3usize {
                let keys = distinct_keys(&mut rng, lay, n);
                let mut d = Dual::new(&s, lay, &format!("del-pos n={} which={}", n, which));
                for k in keys.iter() {
                    let v = mk_val(&mut rng, lay);
                    d.put(k, &v, HM_BINARY);
                }
                let target = match which {
                    0 => keys.len() - 1, // last live element: no compaction
                    1 => 0,              // first: compaction
                    _ => keys.len() / 2, // middle
                };
                d.del(&keys[target], 0, HM_BINARY);
                for (i, k) in keys.iter().enumerate() {
                    let got = d.get(k, HM_BINARY);
                    if i == target {
                        assert_eq!(got, -1);
                    } else {
                        assert!(got >= 0, "key {} lost after deleting {}", i, target);
                    }
                }
                d.free();
            }
        }
    }
}

// --- row 53 --------------------------------------------------------------
#[test]
fn cfg_53_hmdel_nonzero_keyoffset() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 53);
    // keyoffset must satisfy keyoffset + keysize <= elemsize
    let cases: [(Layout, usize); 6] = [
        (L_S1, 4),
        (L_S1, 8),
        (L_S1, 12),
        (L_S2, 8),
        (L_BIG, 4),
        (L_BIG, 64),
    ];
    for (lay, keyoffset) in cases {
        let keys = distinct_keys(&mut rng, lay, 12);
        let mut d = Dual::new(&s, lay, &format!("keyoffset={}", keyoffset));
        for k in keys.iter() {
            // values are all 0xFF so they can never accidentally equal a key
            let v = vec![0xFFu8; lay.elemsize - lay.keysize];
            d.put(k, &v, HM_BINARY);
        }
        for k in keys.iter() {
            // key stored at offset 0, compared at `keyoffset` -> always a miss
            assert_eq!(
                d.del(k, keyoffset, HM_BINARY),
                0,
                "delete with keyoffset={} must miss",
                keyoffset
            );
        }
        // keyoffset 0 still works afterwards
        for k in keys.iter() {
            assert_eq!(d.del(k, 0, HM_BINARY), 1);
        }
        d.free();
    }
}

// --- row 54 --------------------------------------------------------------
#[test]
fn cfg_54_hmdel_all_shrink_ladder() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 54);
    for &lay in [L_I2I, L_S1, L_U2U].iter() {
        for order in 0..3usize {
            let n = 100usize;
            let keys = distinct_keys(&mut rng, lay, n);
            let mut d = Dual::new(&s, lay, &format!("del-all order={}", order));
            for k in keys.iter() {
                let v = mk_val(&mut rng, lay);
                d.put(k, &v, HM_BINARY);
            }
            let mut order_idx: Vec<usize> = (0..n).collect();
            match order {
                0 => {}
                1 => order_idx.reverse(),
                _ => {
                    for i in (1..n).rev() {
                        let j = rng.below(i + 1);
                        order_idx.swap(i, j);
                    }
                }
            }
            for &i in order_idx.iter() {
                assert_eq!(d.del(&keys[i], 0, HM_BINARY), 1);
            }
            // everything gone
            for k in keys.iter() {
                assert_eq!(d.get(k, HM_BINARY), -1);
            }
            d.free();
        }
    }
}

// --- row 55 --------------------------------------------------------------
#[test]
fn cfg_55_delete_then_reinsert_tombstone_reuse() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 55);
    for &lay in [L_I2I, L_S2, L_B1].iter() {
        let keys = distinct_keys(&mut rng, lay, 30);
        let mut d = Dual::new(&s, lay, "tombstone-reuse");
        for k in keys.iter() {
            let v = mk_val(&mut rng, lay);
            d.put(k, &v, HM_BINARY);
        }
        for round in 0..40 {
            let i = rng.below(keys.len());
            d.del(&keys[i], 0, HM_BINARY);
            let v = mk_val(&mut rng, lay);
            d.put(&keys[i], &v, HM_BINARY);
            let _ = round;
        }
        for k in keys.iter() {
            assert!(d.get(k, HM_BINARY) >= 0);
        }
        d.free();
    }
}

// --- row 71: mixed randomized workload -----------------------------------
#[test]
fn cfg_71_mixed_binary_workload() {
    let s = session();
    for &lay in [L_I2I, L_S1, L_S2, L_ODD].iter() {
        let mut rng = Rng::new(TEST_SEED ^ 71 ^ (lay.elemsize as u64));
        let pool = distinct_keys(&mut rng, lay, 60);
        let mut d = Dual::new(&s, lay, "mixed");
        for _ in 0..2000 {
            let k = pool[rng.below(pool.len())].clone();
            match rng.below(6) {
                0 | 1 => {
                    let v = mk_val(&mut rng, lay);
                    d.put(&k, &v, HM_BINARY);
                }
                2 => {
                    d.get(&k, HM_BINARY);
                }
                3 => {
                    d.get_ts(&k, HM_BINARY);
                }
                4 => {
                    d.del(&k, 0, HM_BINARY);
                }
                _ => {
                    unsafe {
                        let c = (s.c.hmput_default)(d.c, lay.elemsize);
                        let r = (s.rust.hmput_default)(d.r, lay.elemsize);
                        d.c = c;
                        d.r = r;
                    }
                    d.check("hmput_default");
                }
            }
        }
        d.free();
    }
}

// --- rows 59/60/61 for binary tables ------------------------------------
#[test]
fn cfg_59_60_61_hmfree_binary() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 59);
    // row 59: NULL
    unsafe {
        (s.c.hmfree_func)(std::ptr::null_mut(), 8);
        (s.rust.hmfree_func)(std::ptr::null_mut(), 8);
        (s.c.hmfree_func)(std::ptr::null_mut(), 0);
        (s.rust.hmfree_func)(std::ptr::null_mut(), 0);
    }
    // row 60: no hash table
    for &lay in BINARY_LAYOUTS.iter() {
        unsafe {
            let c = (s.c.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            let r = (s.rust.hmput_default)(std::ptr::null_mut(), lay.elemsize);
            (s.c.hmfree_func)((c as *mut u8).sub(lay.elemsize) as *mut c_void, lay.elemsize);
            (s.rust.hmfree_func)((r as *mut u8).sub(lay.elemsize) as *mut c_void, lay.elemsize);
        }
    }
    // row 61 (SH_NONE part): tables with 0/1/100 entries
    for &lay in BINARY_LAYOUTS.iter() {
        for n in [0usize, 1, 7, 100] {
            let keys = distinct_keys(&mut rng, lay, n);
            let mut d = Dual::new(&s, lay, "hmfree");
            for k in keys.iter() {
                let v = mk_val(&mut rng, lay);
                d.put(k, &v, HM_BINARY);
            }
            d.free();
        }
    }
    // explicit SH_NONE table via stbds_shmode_func, binary keys
    for &lay in BINARY_LAYOUTS.iter() {
        let mut d = Dual::from_shmode(&s, lay, "shmode-none", SH_NONE);
        d.check("fresh SH_NONE table");
        let keys = distinct_keys(&mut rng, lay, 20);
        for k in keys.iter() {
            let v = mk_val(&mut rng, lay);
            d.put(k, &v, HM_BINARY);
        }
        d.free();
    }
}

// --- row 42: out-of-range stbds_shmode_func modes, binary keys -----------
#[test]
fn cfg_42_shmode_out_of_range_modes() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 42);
    let modes: [c_int; 12] = [
        SH_NONE, 4, 5, 6, 255, 256, 257, 512, -1, -256, i32::MIN, i32::MAX,
    ];
    for &lay in [L_I2I, L_S1, L_U2U].iter() {
        for &m in modes.iter() {
            let mut d = Dual::from_shmode(&s, lay, &format!("shmode({})", m), m);
            d.check(&format!("fresh table shmode={}", m));
            let keys = distinct_keys(&mut rng, lay, 15);
            for k in keys.iter() {
                let v = mk_val(&mut rng, lay);
                d.put(k, &v, HM_BINARY);
            }
            for k in keys.iter() {
                assert!(d.get(k, HM_BINARY) >= 0);
            }
            for k in keys.iter() {
                assert_eq!(d.del(k, 0, HM_BINARY), 1);
            }
            d.free();
        }
    }
}

// --- row 73 (binary half): out-of-range HM modes ------------------------
#[test]
fn cfg_73_out_of_range_hm_modes_binary() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 73);
    // every mode <= 0 must behave exactly like STBDS_HM_BINARY
    let modes: [c_int; 5] = [0, -1, -2, -1000, i32::MIN];
    for &lay in [L_I2I, L_S2, L_BIG].iter() {
        for &m in modes.iter() {
            let keys = distinct_keys(&mut rng, lay, 20);
            let mut d = Dual::new(&s, lay, &format!("hm-mode({})", m));
            for k in keys.iter() {
                let v = mk_val(&mut rng, lay);
                d.put(k, &v, m);
            }
            for k in keys.iter() {
                assert!(d.get(k, m) >= 0);
                assert!(d.get_ts(k, m) >= 0);
            }
            for k in keys.iter() {
                assert_eq!(d.del(k, 0, m), 1);
            }
            d.free();
        }
    }
}
