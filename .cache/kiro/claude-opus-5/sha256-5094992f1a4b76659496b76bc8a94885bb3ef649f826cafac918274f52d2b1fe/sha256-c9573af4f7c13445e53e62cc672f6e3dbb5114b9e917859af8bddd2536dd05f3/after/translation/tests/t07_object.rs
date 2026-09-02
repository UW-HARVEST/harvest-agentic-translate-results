//! Phase B/C — value.c objects + iterators + updates.
//! CONFIGS rows 37-48 · ERRORS rows 1-40.
mod common;
use common::*;
use std::ffi::{c_char, c_void};

unsafe fn mkv(api: &Api, i: usize) -> *mut JsonT {
    unsafe {
        match i % 6 {
            0 => (api.json_integer)(i as i64),
            1 => (api.json_string)(cs(&format!("v{i}")).as_ptr()),
            2 => (api.json_true)(),
            3 => (api.json_null)(),
            4 => {
                let a = (api.json_array)();
                (api.json_array_append_new)(a, (api.json_integer)(i as i64));
                a
            }
            _ => {
                let o = (api.json_object)();
                (api.json_object_set_new_nocheck)(
                    o,
                    cs("inner").as_ptr(),
                    (api.json_integer)(i as i64),
                );
                o
            }
        }
    }
}

unsafe fn build_obj(api: &'static Api, keys: &[String]) -> *mut JsonT {
    unsafe {
        let o = (api.json_object)();
        for (i, k) in keys.iter().enumerate() {
            let kc = cs(k);
            assert_eq!(
                (api.json_object_set_new_nocheck)(o, kc.as_ptr(), mkv(api, i)),
                0
            );
        }
        o
    }
}

/* ---- CONFIGS 37/39/42/43: set/get/size/iterate across rehash ---- */

#[test]
fn json_object_set_get_across_rehash() {
    unsafe {
        for &n in &[0usize, 1, 2, 7, 8, 9, 15, 16, 17, 63, 64, 65, 200] {
            let keys: Vec<String> = (0..n).map(|i| format!("k{i:04}")).collect();
            let co = build_obj(c(), &keys);
            let ro = build_obj(r(), &keys);
            assert_eq!(
                (c().json_object_size)(co),
                (r().json_object_size)(ro),
                "size n={n}"
            );
            assert_eq!((c().json_object_size)(co), n);
            // CONFIGS 43: iteration order must match exactly
            assert_eq!(shape(c(), co), shape(r(), ro), "shape n={n}");
            for k in &keys {
                let kc = cs(k);
                let cg = (c().json_object_get)(co, kc.as_ptr());
                let rg = (r().json_object_get)(ro, kc.as_ptr());
                assert!(!cg.is_null() && !rg.is_null(), "get {k}");
                assert_eq!(shape(c(), cg), shape(r(), rg));
                let cgn = (c().json_object_getn)(co, kc.as_ptr(), k.len());
                let rgn = (r().json_object_getn)(ro, kc.as_ptr(), k.len());
                assert_eq!(cgn, cg);
                assert_eq!(rgn, rg);
                // ERRORS 6: truncated key_len must miss identically
                if k.len() > 1 {
                    let c2 = (c().json_object_getn)(co, kc.as_ptr(), k.len() - 1);
                    let r2 = (r().json_object_getn)(ro, kc.as_ptr(), k.len() - 1);
                    assert_eq!(c2.is_null(), r2.is_null());
                }
            }
            let miss = cs("definitely-absent");
            assert!((c().json_object_get)(co, miss.as_ptr()).is_null());
            assert!((r().json_object_get)(ro, miss.as_ptr()).is_null());
            decref(c(), co);
            decref(r(), ro);
        }
    }
}

/* ---- CONFIGS 38: setn with key_len < strlen, UTF-8 keys ---- */

#[test]
fn json_object_setn_key_len_and_utf8_keys() {
    unsafe {
        let mut rng = Rng::new(0x0B1E_0001);
        for trial in 0..500 {
            let co = (c().json_object)();
            let ro = (r().json_object)();
            let nk = 1 + rng.below(12);
            for i in 0..nk {
                let full = format!("{}{}", rng.utf8(6), rng.key(6));
                let fb = cbytes(full.as_bytes());
                // key_len may cut a multi-byte sequence in half — the C's
                // nocheck path accepts that, the checking path must reject it.
                let kl = rng.below(fb.len());
                let p = fb.as_ptr() as *const c_char;
                let cv = (c().json_object_setn_new_nocheck)(co, p, kl, mkv(c(), i));
                let rv = (r().json_object_setn_new_nocheck)(ro, p, kl, mkv(r(), i));
                assert_eq!(cv, rv, "trial {trial} setn_nocheck ret");
                let cv2 = (c().json_object_setn_new)(co, p, kl, mkv(c(), i + 1));
                let rv2 = (r().json_object_setn_new)(ro, p, kl, mkv(r(), i + 1));
                assert_eq!(cv2, rv2, "trial {trial} setn (checked) ret");
                assert_eq!(shape(c(), co), shape(r(), ro), "trial {trial} shape");
                // getn with the same key_len must find it
                let cg = (c().json_object_getn)(co, p, kl);
                let rg = (r().json_object_getn)(ro, p, kl);
                assert_eq!(cg.is_null(), rg.is_null());
            }
            assert_eq!(
                (c().json_object_size)(co),
                (r().json_object_size)(ro),
                "trial {trial} size"
            );
            decref(c(), co);
            decref(r(), ro);
        }
        // ERRORS 13: invalid UTF-8 key rejected by the checking setter,
        // accepted by the nocheck one.
        for api in both() {
            let o = (api.json_object)();
            let bad = [0xC2u8, 0x00];
            let p = bad.as_ptr() as *const c_char;
            assert_eq!((api.json_object_setn_new)(o, p, 1, (api.json_integer)(1)), -1);
            assert_eq!((api.json_object_size)(o), 0);
            assert_eq!(
                (api.json_object_setn_new_nocheck)(o, p, 1, (api.json_integer)(1)),
                0
            );
            assert_eq!((api.json_object_size)(o), 1);
            // and json_object_set_new (strlen path) with a valid UTF-8 key
            assert_eq!(
                (api.json_object_set_new)(o, cs("héllo").as_ptr(), (api.json_integer)(2)),
                0
            );
            decref(api, o);
        }
    }
}

/* ---- ERRORS 1-14: object setter/getter rejections ---- */

#[test]
fn json_object_setter_getter_rejections() {
    unsafe {
        let nul: *const c_char = std::ptr::null();
        for api in both() {
            let tag = api.tag;
            let o = (api.json_object)();
            let arr = (api.json_array)();
            let s = (api.json_string)(cs("x").as_ptr());

            // ERRORS 1
            assert_eq!((api.json_object_size)(std::ptr::null()), 0, "{tag}");
            assert_eq!((api.json_object_size)(arr), 0);
            assert_eq!((api.json_object_size)(s), 0);
            // ERRORS 2/3/4/5
            assert!((api.json_object_get)(o, nul).is_null());
            assert!((api.json_object_get)(std::ptr::null(), cs("k").as_ptr()).is_null());
            assert!((api.json_object_get)(arr, cs("k").as_ptr()).is_null());
            assert!((api.json_object_getn)(o, nul, 0).is_null());
            assert!((api.json_object_getn)(arr, cs("k").as_ptr(), 1).is_null());
            assert!(
                (api.json_object_getn)(std::ptr::null(), cs("k").as_ptr(), 1).is_null()
            );
            // ERRORS 7
            assert_eq!(
                (api.json_object_set_new_nocheck)(o, nul, (api.json_integer)(1)),
                -1
            );
            // ERRORS 8
            assert_eq!(
                (api.json_object_setn_new_nocheck)(o, cs("k").as_ptr(), 1, std::ptr::null_mut()),
                -1
            );
            // ERRORS 9
            assert_eq!(
                (api.json_object_setn_new_nocheck)(o, nul, 0, (api.json_integer)(1)),
                -1
            );
            // ERRORS 10
            assert_eq!(
                (api.json_object_setn_new_nocheck)(arr, cs("k").as_ptr(), 1, (api.json_integer)(1)),
                -1
            );
            assert_eq!(
                (api.json_object_setn_new_nocheck)(
                    std::ptr::null_mut(),
                    cs("k").as_ptr(),
                    1,
                    (api.json_integer)(1)
                ),
                -1
            );
            // ERRORS 11: json == value
            assert_eq!(
                (api.json_object_setn_new_nocheck)(o, cs("self").as_ptr(), 4, incref(o)),
                -1
            );
            // ERRORS 12/14
            assert_eq!(
                (api.json_object_setn_new)(o, nul, 0, (api.json_integer)(1)),
                -1
            );
            assert_eq!((api.json_object_set_new)(o, nul, (api.json_integer)(1)), -1);
            assert_eq!((api.json_object_size)(o), 0, "{tag}: nothing was inserted");

            // ERRORS 15-18
            assert_eq!((api.json_object_del)(o, nul), -1);
            assert_eq!((api.json_object_deln)(o, nul, 0), -1);
            assert_eq!((api.json_object_deln)(arr, cs("k").as_ptr(), 1), -1);
            assert_eq!(
                (api.json_object_deln)(std::ptr::null_mut(), cs("k").as_ptr(), 1),
                -1
            );
            assert_eq!((api.json_object_del)(o, cs("absent").as_ptr()), -1);
            // ERRORS 19
            assert_eq!((api.json_object_clear)(arr), -1);
            assert_eq!((api.json_object_clear)(std::ptr::null_mut()), -1);
            assert_eq!((api.json_object_clear)(o), 0);

            decref(api, s);
            decref(api, arr);
            decref(api, o);
        }
    }
}

/* ---- CONFIGS 40/41: del + clear + reuse ---- */

#[test]
fn json_object_del_clear_reuse() {
    unsafe {
        for &n in &[0usize, 1, 9, 64] {
            let keys: Vec<String> = (0..n).map(|i| format!("d{i:03}")).collect();
            for pick in [0usize, 1, 2] {
                let co = build_obj(c(), &keys);
                let ro = build_obj(r(), &keys);
                for (i, k) in keys.iter().enumerate() {
                    if i % 3 != pick {
                        continue;
                    }
                    let kc = cs(k);
                    assert_eq!(
                        (c().json_object_del)(co, kc.as_ptr()),
                        (r().json_object_del)(ro, kc.as_ptr()),
                        "del {k}"
                    );
                    // deleting again must fail in both
                    assert_eq!(
                        (c().json_object_del)(co, kc.as_ptr()),
                        (r().json_object_del)(ro, kc.as_ptr())
                    );
                    assert_eq!((c().json_object_del)(co, kc.as_ptr()), -1);
                }
                assert_eq!(shape(c(), co), shape(r(), ro), "n={n} pick={pick} after del");
                // re-add: goes to the END of the ordered list
                for (i, k) in keys.iter().enumerate() {
                    if i % 3 != pick {
                        continue;
                    }
                    let kc = cs(k);
                    assert_eq!(
                        (c().json_object_set_new_nocheck)(co, kc.as_ptr(), mkv(c(), i)),
                        (r().json_object_set_new_nocheck)(ro, kc.as_ptr(), mkv(r(), i))
                    );
                }
                assert_eq!(shape(c(), co), shape(r(), ro), "n={n} pick={pick} after re-add");
                // deln with explicit length
                for k in &keys {
                    let kc = cs(k);
                    assert_eq!(
                        (c().json_object_deln)(co, kc.as_ptr(), k.len()),
                        (r().json_object_deln)(ro, kc.as_ptr(), k.len())
                    );
                }
                assert_eq!((c().json_object_size)(co), 0);
                assert_eq!((r().json_object_size)(ro), 0);
                decref(c(), co);
                decref(r(), ro);
            }
            // clear + reuse
            let co = build_obj(c(), &keys);
            let ro = build_obj(r(), &keys);
            assert_eq!((c().json_object_clear)(co), (r().json_object_clear)(ro));
            assert_eq!(shape(c(), co), shape(r(), ro), "n={n} after clear");
            for (i, k) in keys.iter().enumerate().take(5) {
                let kc = cs(k);
                (c().json_object_set_new_nocheck)(co, kc.as_ptr(), mkv(c(), i));
                (r().json_object_set_new_nocheck)(ro, kc.as_ptr(), mkv(r(), i));
            }
            assert_eq!(shape(c(), co), shape(r(), ro), "n={n} after clear+reuse");
            decref(c(), co);
            decref(r(), ro);
        }
    }
}

/* ---- CONFIGS 43/44 · ERRORS 27-40: iterators ---- */

#[test]
fn json_object_iterators() {
    unsafe {
        let mut rng = Rng::new(0x0B1E_0002);
        for trial in 0..300 {
            let n = rng.below(40);
            let mut keys: Vec<String> = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            for _ in 0..n {
                let k = rng.key(9);
                if seen.insert(k.clone()) {
                    keys.push(k);
                }
            }
            let co = build_obj(c(), &keys);
            let ro = build_obj(r(), &keys);

            // full traversal via iter/iter_next, comparing key bytes + value shape
            let mut ci = (c().json_object_iter)(co);
            let mut ri = (r().json_object_iter)(ro);
            let mut count = 0;
            while !ci.is_null() {
                assert!(!ri.is_null(), "trial {trial}: RUST iter ended early");
                let ckl = (c().json_object_iter_key_len)(ci);
                let rkl = (r().json_object_iter_key_len)(ri);
                assert_eq!(ckl, rkl, "trial {trial} key_len");
                let ck = std::slice::from_raw_parts(
                    (c().json_object_iter_key)(ci) as *const u8,
                    ckl,
                );
                let rk = std::slice::from_raw_parts(
                    (r().json_object_iter_key)(ri) as *const u8,
                    rkl,
                );
                assert_eq!(ck, rk, "trial {trial} key bytes");
                assert_eq!(
                    shape(c(), (c().json_object_iter_value)(ci)),
                    shape(r(), (r().json_object_iter_value)(ri)),
                    "trial {trial} value"
                );

                // CONFIGS 44: key_to_iter must round-trip back to this iterator
                let ck2 = (c().json_object_key_to_iter)((c().json_object_iter_key)(ci));
                let rk2 = (r().json_object_key_to_iter)((r().json_object_iter_key)(ri));
                assert_eq!(ck2, ci, "C key_to_iter round-trip");
                assert_eq!(rk2, ri, "RUST key_to_iter round-trip");

                // iter_at must return the same iterator
                let kstr = cs(std::str::from_utf8(ck).unwrap());
                let cia = (c().json_object_iter_at)(co, kstr.as_ptr());
                let ria = (r().json_object_iter_at)(ro, kstr.as_ptr());
                assert_eq!(cia, ci);
                assert_eq!(ria, ri);

                // iter_set_new mid-traversal
                assert_eq!(
                    (c().json_object_iter_set_new)(co, ci, (c().json_integer)(count as i64)),
                    (r().json_object_iter_set_new)(ro, ri, (r().json_integer)(count as i64))
                );

                ci = (c().json_object_iter_next)(co, ci);
                ri = (r().json_object_iter_next)(ro, ri);
                count += 1;
            }
            assert!(ri.is_null(), "trial {trial}: C iter ended early");
            assert_eq!(count, keys.len());
            assert_eq!(shape(c(), co), shape(r(), ro), "trial {trial} after iter_set");
            decref(c(), co);
            decref(r(), ro);
        }

        // ERRORS 27-40
        for api in both() {
            let arr = (api.json_array)();
            let o = (api.json_object)();
            let nul: *mut c_void = std::ptr::null_mut();
            assert!((api.json_object_iter)(arr).is_null()); // 27
            assert!((api.json_object_iter)(std::ptr::null_mut()).is_null());
            assert!((api.json_object_iter)(o).is_null()); // empty object
            assert!((api.json_object_iter_at)(o, std::ptr::null()).is_null()); // 28
            assert!((api.json_object_iter_at)(arr, cs("k").as_ptr()).is_null()); // 29
            assert!((api.json_object_iter_at)(o, cs("k").as_ptr()).is_null()); // 30
            assert!((api.json_object_iter_next)(arr, nul).is_null()); // 31
            assert!((api.json_object_iter_next)(o, nul).is_null()); // 32
            assert!((api.json_object_iter_key)(nul).is_null()); // 34
            assert_eq!((api.json_object_iter_key_len)(nul), 0); // 35
            assert!((api.json_object_iter_value)(nul).is_null()); // 36
            assert_eq!(
                (api.json_object_iter_set_new)(arr, nul, (api.json_integer)(1)),
                -1
            ); // 37
            assert_eq!(
                (api.json_object_iter_set_new)(o, nul, (api.json_integer)(1)),
                -1
            ); // 38
            // 39: NULL value with a real iterator
            (api.json_object_set_new_nocheck)(o, cs("a").as_ptr(), (api.json_integer)(1));
            let it = (api.json_object_iter)(o);
            assert!(!it.is_null());
            assert_eq!((api.json_object_iter_set_new)(o, it, std::ptr::null_mut()), -1);
            // 33: iter_next at the last element
            assert!((api.json_object_iter_next)(o, it).is_null());
            // 40
            assert!((api.json_object_key_to_iter)(std::ptr::null()).is_null());
            decref(api, o);
            decref(api, arr);
        }
    }
}

/* ---- CONFIGS 45/46/47 · ERRORS 20-23: update variants ---- */

#[test]
fn json_object_update_variants() {
    unsafe {
        let mut rng = Rng::new(0x0B1E_0003);
        for trial in 0..600 {
            let na = rng.below(15);
            let nb = rng.below(15);
            let overlap = rng.below(4);
            let akeys: Vec<String> = (0..na).map(|i| format!("a{i}")).collect();
            let bkeys: Vec<String> = (0..nb)
                .map(|i| {
                    if i < overlap && i < na {
                        format!("a{i}")
                    } else {
                        format!("b{i}")
                    }
                })
                .collect();
            for which in 0..3 {
                let co = build_obj(c(), &akeys);
                let ro = build_obj(r(), &akeys);
                let cb = build_obj(c(), &bkeys);
                let rb = build_obj(r(), &bkeys);
                let f_c = match which {
                    0 => c().json_object_update,
                    1 => c().json_object_update_existing,
                    _ => c().json_object_update_missing,
                };
                let f_r = match which {
                    0 => r().json_object_update,
                    1 => r().json_object_update_existing,
                    _ => r().json_object_update_missing,
                };
                let cv = f_c(co, cb);
                let rv = f_r(ro, rb);
                assert_eq!(cv, rv, "trial {trial} which={which} ret");
                assert_eq!(
                    shape(c(), co),
                    shape(r(), ro),
                    "trial {trial} which={which} result"
                );
                assert_eq!(shape(c(), cb), shape(r(), rb), "other unchanged");
                decref(c(), cb);
                decref(r(), rb);
                decref(c(), co);
                decref(r(), ro);
            }
        }
        // self-update
        for which in 0..3 {
            let keys: Vec<String> = (0..5).map(|i| format!("s{i}")).collect();
            let co = build_obj(c(), &keys);
            let ro = build_obj(r(), &keys);
            let f_c = match which {
                0 => c().json_object_update,
                1 => c().json_object_update_existing,
                _ => c().json_object_update_missing,
            };
            let f_r = match which {
                0 => r().json_object_update,
                1 => r().json_object_update_existing,
                _ => r().json_object_update_missing,
            };
            assert_eq!(f_c(co, co), f_r(ro, ro), "self-update which={which}");
            assert_eq!(shape(c(), co), shape(r(), ro), "self-update which={which}");
            decref(c(), co);
            decref(r(), ro);
        }
        // ERRORS 20-23: non-object args
        for api in both() {
            let o = (api.json_object)();
            let a = (api.json_array)();
            for f in [
                api.json_object_update,
                api.json_object_update_existing,
                api.json_object_update_missing,
                api.json_object_update_recursive,
            ] {
                assert_eq!(f(a, o), -1);
                assert_eq!(f(o, a), -1);
                assert_eq!(f(std::ptr::null_mut(), o), -1);
                assert_eq!(f(o, std::ptr::null_mut()), -1);
            }
            decref(api, a);
            decref(api, o);
        }
    }
}

/* ---- CONFIGS 48 · ERRORS 24-26: recursive update ---- */

#[test]
fn json_object_update_recursive_differential() {
    unsafe {
        let mut rng = Rng::new(0x0B1E_0004);

        unsafe fn nest(api: &'static Api, rng: &mut Rng, depth: u32) -> *mut JsonT {
            unsafe {
                let o = (api.json_object)();
                let n = 1 + rng.below(4);
                for i in 0..n {
                    let k = cs(&format!("k{i}"));
                    let v = if depth > 0 && rng.below(2) == 0 {
                        nest(api, rng, depth - 1)
                    } else if rng.bool() {
                        (api.json_integer)(rng.range_i64(-100, 100))
                    } else {
                        (api.json_string)(cs(&format!("leaf{i}")).as_ptr())
                    };
                    (api.json_object_set_new_nocheck)(o, k.as_ptr(), v);
                }
                o
            }
        }

        for trial in 0..400 {
            // Build the same structure in both by replaying the same RNG stream.
            let s = rng.next_u64();
            let mut r1 = Rng::new(s);
            let mut r2 = Rng::new(s);
            let co = nest(c(), &mut r1, 4);
            let ro = nest(r(), &mut r2, 4);
            let s2 = rng.next_u64();
            let mut r3 = Rng::new(s2);
            let mut r4 = Rng::new(s2);
            let cb = nest(c(), &mut r3, 4);
            let rb = nest(r(), &mut r4, 4);
            assert_eq!(shape(c(), co), shape(r(), ro), "trial {trial} setup a");
            assert_eq!(shape(c(), cb), shape(r(), rb), "trial {trial} setup b");

            let cv = (c().json_object_update_recursive)(co, cb);
            let rv = (r().json_object_update_recursive)(ro, rb);
            assert_eq!(cv, rv, "trial {trial} ret");
            assert_eq!(shape(c(), co), shape(r(), ro), "trial {trial} result");
            decref(c(), cb);
            decref(r(), rb);
            decref(c(), co);
            decref(r(), ro);
        }

        // ERRORS 24/25: cycle handling. The C only recurses into `value` when
        // the destination ALREADY holds an object at that key, so the cycle is
        // only reached for structurally-matching destinations. Compare the two
        // implementations rather than asserting a guessed value.
        let mut cyc_results = Vec::new();
        for api in both() {
            let a = (api.json_object)();
            let b = (api.json_object)();
            (api.json_object_set_new_nocheck)(a, cs("b").as_ptr(), incref(b));
            (api.json_object_set_new_nocheck)(b, cs("a").as_ptr(), incref(a));

            // (i) empty destination: no recursion happens
            let dst = (api.json_object)();
            let r1 = (api.json_object_update_recursive)(dst, a);
            let s1 = shape(api, dst);

            // (ii) destination whose "b" is itself an object => recursion,
            //      and the cycle in `a` is reached.
            let dst2 = (api.json_object)();
            let inner = (api.json_object)();
            (api.json_object_set_new_nocheck)(dst2, cs("b").as_ptr(), inner);
            let r2 = (api.json_object_update_recursive)(dst2, a);

            // (iii) self-recursive update: `a` contains `b` contains `a`
            let r3 = (api.json_object_update_recursive)(a, a);

            cyc_results.push((r1, s1, r2, r3));
        }
        assert_eq!(
            cyc_results[0], cyc_results[1],
            "ERRORS 24/25: recursive-update cycle handling must match"
        );
        // The self-recursive case must be rejected (the loop check fires).
        assert_eq!(cyc_results[0].3, -1, "self-recursive update is rejected");

        // do_object_update_recursive with a caller-supplied parents hashtable
        for api in both() {
            let mut ht = Box::new(Hashtable::default());
            assert_eq!((api.hashtable_init)(&mut *ht), 0);
            let o = (api.json_object)();
            let other = (api.json_object)();
            (api.json_object_set_new_nocheck)(other, cs("x").as_ptr(), (api.json_integer)(1));
            let v = (api.do_object_update_recursive)(o, other, &mut *ht);
            assert_eq!(v, 0, "{}: do_object_update_recursive", api.tag);
            (api.hashtable_close)(&mut *ht);
            decref(api, other);
            decref(api, o);
        }
        // and the same, differentially
        let mut cht = Box::new(Hashtable::default());
        let mut rht = Box::new(Hashtable::default());
        (c().hashtable_init)(&mut *cht);
        (r().hashtable_init)(&mut *rht);
        let keys: Vec<String> = (0..6).map(|i| format!("dr{i}")).collect();
        let co = build_obj(c(), &keys);
        let ro = build_obj(r(), &keys);
        let cb = build_obj(c(), &keys);
        let rb = build_obj(r(), &keys);
        assert_eq!(
            (c().do_object_update_recursive)(co, cb, &mut *cht),
            (r().do_object_update_recursive)(ro, rb, &mut *rht)
        );
        assert_eq!(shape(c(), co), shape(r(), ro));
        assert_eq!(cht.size, rht.size, "parents set left clean");
        (c().hashtable_close)(&mut *cht);
        (r().hashtable_close)(&mut *rht);
        decref(c(), cb);
        decref(r(), rb);
        decref(c(), co);
        decref(r(), ro);
    }
}
