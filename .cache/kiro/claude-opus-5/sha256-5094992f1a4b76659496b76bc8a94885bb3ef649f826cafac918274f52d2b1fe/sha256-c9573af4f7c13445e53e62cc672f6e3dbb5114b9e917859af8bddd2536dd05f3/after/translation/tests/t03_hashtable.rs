//! Phase B/C — hashtable.c (the lowest-level entry points).
//! CONFIGS rows 9-12, 16 · ERRORS rows 123, 125, 126, 127.
mod common;
use common::*;
use std::ffi::{c_char, c_void};

struct Ht {
    api: &'static Api,
    ht: Box<Hashtable>,
}

impl Ht {
    unsafe fn new(api: &'static Api) -> Ht {
        unsafe {
            let mut ht = Box::new(Hashtable::default());
            assert_eq!((api.hashtable_init)(&mut *ht), 0, "{}: init", api.tag);
            Ht { api, ht }
        }
    }
    unsafe fn set(&mut self, key: &[u8], val: *mut JsonT) -> i32 {
        unsafe {
            (self.api.hashtable_set)(
                &mut *self.ht,
                key.as_ptr() as *const c_char,
                key.len(),
                val,
            )
        }
    }
    unsafe fn get(&mut self, key: &[u8]) -> *mut c_void {
        unsafe {
            (self.api.hashtable_get)(&mut *self.ht, key.as_ptr() as *const c_char, key.len())
        }
    }
    unsafe fn del(&mut self, key: &[u8]) -> i32 {
        unsafe {
            (self.api.hashtable_del)(&mut *self.ht, key.as_ptr() as *const c_char, key.len())
        }
    }
    /// Full ordered traversal, as (key bytes, value type) pairs.
    unsafe fn walk(&mut self) -> Vec<(Vec<u8>, i32)> {
        unsafe {
            let mut out = Vec::new();
            let mut it = (self.api.hashtable_iter)(&mut *self.ht);
            while !it.is_null() {
                let kp = (self.api.hashtable_iter_key)(it);
                let kl = (self.api.hashtable_iter_key_len)(it);
                let kb = std::slice::from_raw_parts(kp as *const u8, kl).to_vec();
                let v = (self.api.hashtable_iter_value)(it) as *mut JsonT;
                let ty = if v.is_null() { -1 } else { (*v).type_ };
                out.push((kb, ty));
                it = (self.api.hashtable_iter_next)(&mut *self.ht, it);
            }
            out
        }
    }
    unsafe fn close(&mut self) {
        unsafe { (self.api.hashtable_close)(&mut *self.ht) }
    }
}

/// Deterministic value for a key so both libs store comparable things.
unsafe fn val_for(api: &Api, n: usize) -> *mut JsonT {
    unsafe {
        match n % 5 {
            0 => (api.json_null)(),
            1 => (api.json_true)(),
            2 => (api.json_false)(),
            3 => (api.json_integer)(n as i64),
            _ => (api.json_array)(),
        }
    }
}

/* ---- CONFIGS 9: 0,1,7,8,9,64,200 keys → forces hashtable_do_rehash ---- */

#[test]
fn hashtable_lifecycle_across_rehash_boundaries() {
    unsafe {
        for &n in &[0usize, 1, 2, 7, 8, 9, 15, 16, 17, 63, 64, 65, 200] {
            let mut ch = Ht::new(c());
            let mut rh = Ht::new(r());
            let keys: Vec<Vec<u8>> = (0..n).map(|i| format!("key{i:04}").into_bytes()).collect();

            for (i, k) in keys.iter().enumerate() {
                let cv = ch.set(k, val_for(c(), i));
                let rv = rh.set(k, val_for(r(), i));
                assert_eq!(cv, rv, "n={n} set {i}");
                assert_eq!(ch.ht.size, rh.ht.size, "n={n} size after set {i}");
                assert_eq!(ch.ht.order, rh.ht.order, "n={n} order after set {i}");
            }
            assert_eq!(ch.walk(), rh.walk(), "n={n} ordered traversal");

            for (i, k) in keys.iter().enumerate() {
                let cf = !ch.get(k).is_null();
                let rf = !rh.get(k).is_null();
                assert_eq!(cf, rf, "n={n} get {i}");
                assert!(cf, "n={n} key {i} must be present");
            }
            // ERRORS 125: missing key => NULL
            assert!(ch.get(b"nope").is_null());
            assert!(rh.get(b"nope").is_null());
            // ERRORS 126: iter_at on missing key => NULL
            let cia = (c().hashtable_iter_at)(&mut *ch.ht, b"nope".as_ptr() as *const c_char, 4);
            let ria = (r().hashtable_iter_at)(&mut *rh.ht, b"nope".as_ptr() as *const c_char, 4);
            assert_eq!(cia.is_null(), ria.is_null());
            assert!(cia.is_null());
            // ERRORS 123: del on missing key => -1
            assert_eq!(ch.del(b"nope"), rh.del(b"nope"));
            assert_eq!(ch.del(b"nope"), -1);

            // delete every other key, then re-walk
            for (i, k) in keys.iter().enumerate() {
                if i % 2 == 0 {
                    assert_eq!(ch.del(k), rh.del(k), "n={n} del {i}");
                }
            }
            assert_eq!(ch.ht.size, rh.ht.size, "n={n} size after dels");
            assert_eq!(ch.walk(), rh.walk(), "n={n} traversal after dels");

            (c().hashtable_clear)(&mut *ch.ht);
            (r().hashtable_clear)(&mut *rh.ht);
            assert_eq!(ch.ht.size, rh.ht.size, "n={n} size after clear");
            assert_eq!(ch.walk(), rh.walk(), "n={n} traversal after clear");
            assert!(ch.walk().is_empty());

            // reuse after clear
            for (i, k) in keys.iter().enumerate().take(5) {
                assert_eq!(ch.set(k, val_for(c(), i)), rh.set(k, val_for(r(), i)));
            }
            assert_eq!(ch.walk(), rh.walk(), "n={n} traversal after reuse");

            ch.close();
            rh.close();
        }
    }
}

/* ---- CONFIGS 10: duplicate-key overwrite path ---- */

#[test]
fn hashtable_set_overwrites_duplicate_key() {
    unsafe {
        let mut ch = Ht::new(c());
        let mut rh = Ht::new(r());
        for round in 0..5 {
            for i in 0..12usize {
                let k = format!("dup{}", i % 4).into_bytes();
                let cv = ch.set(&k, (c().json_integer)((round * 100 + i) as i64));
                let rv = rh.set(&k, (r().json_integer)((round * 100 + i) as i64));
                assert_eq!(cv, rv);
                assert_eq!(ch.ht.size, rh.ht.size, "size must stay at 4");
            }
        }
        assert_eq!(ch.ht.size, 4);
        // compare the actual integer values through the iterator
        let cw: Vec<(Vec<u8>, i64)> = {
            let mut o = Vec::new();
            let mut it = (c().hashtable_iter)(&mut *ch.ht);
            while !it.is_null() {
                let kp = (c().hashtable_iter_key)(it);
                let kl = (c().hashtable_iter_key_len)(it);
                let v = (c().hashtable_iter_value)(it) as *mut JsonT;
                o.push((
                    std::slice::from_raw_parts(kp as *const u8, kl).to_vec(),
                    (c().json_integer_value)(v),
                ));
                it = (c().hashtable_iter_next)(&mut *ch.ht, it);
            }
            o
        };
        let rw: Vec<(Vec<u8>, i64)> = {
            let mut o = Vec::new();
            let mut it = (r().hashtable_iter)(&mut *rh.ht);
            while !it.is_null() {
                let kp = (r().hashtable_iter_key)(it);
                let kl = (r().hashtable_iter_key_len)(it);
                let v = (r().hashtable_iter_value)(it) as *mut JsonT;
                o.push((
                    std::slice::from_raw_parts(kp as *const u8, kl).to_vec(),
                    (r().json_integer_value)(v),
                ));
                it = (r().hashtable_iter_next)(&mut *rh.ht, it);
            }
            o
        };
        assert_eq!(cw, rw, "duplicate-key overwrite values");
        ch.close();
        rh.close();
    }
}

/* ---- CONFIGS 11: iter_at / iter_next / iter_set / ERRORS 127 ---- */

#[test]
fn hashtable_iterators_random() {
    unsafe {
        let mut rng = Rng::new(0x4711_0001);
        for trial in 0..200 {
            let mut ch = Ht::new(c());
            let mut rh = Ht::new(r());
            let n = rng.below(65);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            for i in 0..n {
                let k = rng.key(10).into_bytes();
                if !seen.insert(k.clone()) {
                    continue;
                }
                keys.push(k.clone());
                assert_eq!(ch.set(&k, val_for(c(), i)), rh.set(&k, val_for(r(), i)));
            }
            assert_eq!(ch.walk(), rh.walk(), "trial {trial} walk");

            // iter_at on every key must land on that key
            for k in &keys {
                let ci = (c().hashtable_iter_at)(
                    &mut *ch.ht,
                    k.as_ptr() as *const c_char,
                    k.len(),
                );
                let ri = (r().hashtable_iter_at)(
                    &mut *rh.ht,
                    k.as_ptr() as *const c_char,
                    k.len(),
                );
                assert_eq!(ci.is_null(), ri.is_null());
                assert!(!ci.is_null());
                let ckl = (c().hashtable_iter_key_len)(ci);
                let rkl = (r().hashtable_iter_key_len)(ri);
                assert_eq!(ckl, rkl);
                let ckb = std::slice::from_raw_parts(
                    (c().hashtable_iter_key)(ci) as *const u8,
                    ckl,
                );
                let rkb = std::slice::from_raw_parts(
                    (r().hashtable_iter_key)(ri) as *const u8,
                    rkl,
                );
                assert_eq!(ckb, rkb);
                assert_eq!(ckb, &k[..]);

                // iter_set through the iterator
                (c().hashtable_iter_set)(ci, (c().json_integer)(42));
                (r().hashtable_iter_set)(ri, (r().json_integer)(42));
                assert_eq!(
                    (c().json_integer_value)((c().hashtable_iter_value)(ci) as *mut JsonT),
                    (r().json_integer_value)((r().hashtable_iter_value)(ri) as *mut JsonT)
                );
            }

            // ERRORS 127: iter_next at the last element => NULL
            if !keys.is_empty() {
                let mut ci = (c().hashtable_iter)(&mut *ch.ht);
                let mut ri = (r().hashtable_iter)(&mut *rh.ht);
                let mut steps = 0;
                while !ci.is_null() {
                    ci = (c().hashtable_iter_next)(&mut *ch.ht, ci);
                    ri = (r().hashtable_iter_next)(&mut *rh.ht, ri);
                    assert_eq!(ci.is_null(), ri.is_null(), "trial {trial} step {steps}");
                    steps += 1;
                }
                assert!(ri.is_null());
                assert_eq!(steps, keys.len());
            } else {
                assert!((c().hashtable_iter)(&mut *ch.ht).is_null());
                assert!((r().hashtable_iter)(&mut *rh.ht).is_null());
            }

            ch.close();
            rh.close();
        }
    }
}

/* ---- CONFIGS 12: binary keys, key_len != strlen, embedded NULs ---- */

#[test]
fn hashtable_binary_keys() {
    unsafe {
        let mut rng = Rng::new(0x4711_0002);
        for trial in 0..300 {
            let mut ch = Ht::new(c());
            let mut rh = Ht::new(r());
            let n = 1 + rng.below(20);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            for i in 0..n {
                let m = rng.below(12);
                let mut k = rng.bytes(m);
                if rng.bool() && !k.is_empty() {
                    k[0] = 0; // leading NUL
                }
                if rng.bool() && k.len() > 1 {
                    let idx = rng.below(k.len());
                    k[idx] = 0; // interior NUL
                }
                if !seen.insert(k.clone()) {
                    continue;
                }
                keys.push(k.clone());
                let cv = ch.set(&k, val_for(c(), i));
                let rv = rh.set(&k, val_for(r(), i));
                assert_eq!(cv, rv, "trial {trial} binary set");
            }
            assert_eq!(ch.ht.size, rh.ht.size);
            assert_eq!(ch.walk(), rh.walk(), "trial {trial} binary walk");
            for k in &keys {
                assert_eq!(ch.get(k).is_null(), rh.get(k).is_null());
                assert!(!ch.get(k).is_null());
                // truncated key_len must (usually) miss, and must miss identically
                if k.len() > 1 {
                    let short = &k[..k.len() - 1];
                    let cg = (c().hashtable_get)(
                        &mut *ch.ht,
                        short.as_ptr() as *const c_char,
                        short.len(),
                    );
                    let rg = (r().hashtable_get)(
                        &mut *rh.ht,
                        short.as_ptr() as *const c_char,
                        short.len(),
                    );
                    assert_eq!(cg.is_null(), rg.is_null(), "truncated key {k:02x?}");
                }
            }
            for k in &keys {
                assert_eq!(ch.del(k), rh.del(k));
            }
            assert_eq!(ch.ht.size, 0);
            assert_eq!(rh.ht.size, 0);
            ch.close();
            rh.close();
        }
    }
}

/* ---- CONFIGS 16: json_object_seed / hashtable_seed ---- */

#[test]
fn object_seed_is_idempotent_and_shared() {
    unsafe {
        // Already seeded by the harness with 0x5eed1234; re-seeding must be a
        // no-op in both libraries (the C only seeds when hashtable_seed == 0).
        (c().json_object_seed)(0xdead_beef);
        (r().json_object_seed)(0xdead_beef);
        assert_eq!(c().hashtable_seed(), 0x5eed_1234);
        assert_eq!(r().hashtable_seed(), 0x5eed_1234);
        (c().json_object_seed)(0);
        (r().json_object_seed)(0);
        assert_eq!(c().hashtable_seed(), 0x5eed_1234);
        assert_eq!(r().hashtable_seed(), 0x5eed_1234);
        // With identical seeds, identical insertion sequences must produce
        // identical bucket-order-dependent hash values. hash_str is only
        // observable through iteration order after a rehash, exercised above,
        // plus jsonp_loop_check keys below.
        let mut ch = Ht::new(c());
        let mut rh = Ht::new(r());
        for i in 0..300usize {
            let k = format!("seedcheck-{i}").into_bytes();
            ch.set(&k, (c().json_null)());
            rh.set(&k, (r().json_null)());
        }
        assert_eq!(ch.walk(), rh.walk());
        assert_eq!(ch.ht.order, rh.ht.order);
        ch.close();
        rh.close();
    }
}

/* ---- CONFIGS 52 / ERRORS 93: jsonp_loop_check ---- */

#[test]
fn jsonp_loop_check_differential() {
    unsafe {
        let mut ch = Ht::new(c());
        let mut rh = Ht::new(r());
        // Distinct nodes: first insert succeeds (0), second on the same node
        // fails (-1).  The key text is "%p" of the pointer, so it differs
        // between libraries; only the return value and key length are
        // comparable, plus the fact that the key is "0x…".
        let cn: Vec<*mut JsonT> = (0..8).map(|i| (c().json_integer)(i)).collect();
        let rn: Vec<*mut JsonT> = (0..8).map(|i| (r().json_integer)(i)).collect();
        for i in 0..8 {
            let mut ck = [0i8; 32];
            let mut rk = [0i8; 32];
            let mut cl = 0usize;
            let mut rl = 0usize;
            let cv = (c().jsonp_loop_check)(&mut *ch.ht, cn[i], ck.as_mut_ptr(), 19, &mut cl);
            let rv = (r().jsonp_loop_check)(&mut *rh.ht, rn[i], rk.as_mut_ptr(), 19, &mut rl);
            assert_eq!(cv, rv, "loop_check first insert #{i}");
            assert_eq!(cv, 0);
            let cks = std::ffi::CStr::from_ptr(ck.as_ptr()).to_bytes().to_vec();
            let rks = std::ffi::CStr::from_ptr(rk.as_ptr()).to_bytes().to_vec();
            assert!(cks.starts_with(b"0x"), "C key {cks:?}");
            assert!(rks.starts_with(b"0x"), "RUST key {rks:?}");
            assert_eq!(cl, cks.len(), "C key_len out-param");
            assert_eq!(rl, rks.len(), "RUST key_len out-param");

            // ERRORS 93: the same node again => -1
            let cv2 = (c().jsonp_loop_check)(&mut *ch.ht, cn[i], ck.as_mut_ptr(), 19, &mut cl);
            let rv2 = (r().jsonp_loop_check)(&mut *rh.ht, rn[i], rk.as_mut_ptr(), 19, &mut rl);
            assert_eq!(cv2, rv2, "loop_check repeat #{i}");
            assert_eq!(cv2, -1);
        }
        assert_eq!(ch.ht.size, rh.ht.size);
        // NULL key_len_out must be accepted
        let mut ck = [0i8; 32];
        let mut rk = [0i8; 32];
        let cv = (c().jsonp_loop_check)(
            &mut *ch.ht,
            cn[0],
            ck.as_mut_ptr(),
            19,
            std::ptr::null_mut(),
        );
        let rv = (r().jsonp_loop_check)(
            &mut *rh.ht,
            rn[0],
            rk.as_mut_ptr(),
            19,
            std::ptr::null_mut(),
        );
        assert_eq!(cv, rv);
        ch.close();
        rh.close();
        for p in cn {
            decref(c(), p);
        }
        for p in rn {
            decref(r(), p);
        }
    }
}
