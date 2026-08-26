//! Phase B — differential tests for the value API (CONFIGS.md rows 42..75).

mod common;
use common::tree::*;
use common::*;
use std::os::raw::{c_char, c_void};
use std::ptr;

/* ---------------------------------------------- rows 42..46: scalars ---- */

#[test]
fn cfg42and43_containers_and_singletons() {
    diff("cfg42-43 containers/singletons", |api, rec| unsafe {
        let o = (api.json_object)();
        rec.json("object", o);
        rec.tag_u("object_size", (api.json_object_size)(o));
        let a = (api.json_array)();
        rec.json("array", a);
        rec.tag_u("array_size", (api.json_array_size)(a));
        decref(api, o);
        decref(api, a);

        for _ in 0..3 {
            rec.json("true", (api.json_true)());
            rec.json("false", (api.json_false)());
            rec.json("null", (api.json_null)());
        }
        // singletons must be stable addresses within one library
        rec.tag_i(
            "true_stable",
            ((api.json_true)() == (api.json_true)()) as i64,
        );
        rec.tag_i(
            "distinct",
            ((api.json_true)() != (api.json_false)()
                && (api.json_false)() != (api.json_null)()) as i64,
        );
        // json_delete() called *directly* on singletons must be a no-op
        // (ERRORS.md row 101)
        (api.json_delete)((api.json_true)());
        (api.json_delete)((api.json_false)());
        (api.json_delete)((api.json_null)());
        rec.json("true_after_delete", (api.json_true)());
    });
}

#[test]
fn cfg44_integers() {
    diff("cfg44 integers", |api, rec| unsafe {
        let mut vals: Vec<i64> = vec![0, 1, -1, 2, -2, i64::MIN, i64::MAX, i64::MIN + 1, 1 << 53];
        let mut rng = Rng::new(0x4400);
        for _ in 0..400 {
            vals.push(rng.next_u64() as i64);
        }
        for v in vals {
            let j = (api.json_integer)(v);
            rec.json("j", j);
            rec.tag_i("val", (api.json_integer_value)(j));
            rec.tag_f("num", (api.json_number_value)(j));
            rec.tag_i("set", (api.json_integer_set)(j, v ^ 0x5555) as i64);
            rec.tag_i("val2", (api.json_integer_value)(j));
            rec_dump_all(api, rec, "d", j);
            decref(api, j);
        }
    });
}

#[test]
fn cfg45_reals() {
    diff("cfg45 reals", |api, rec| unsafe {
        let mut vals: Vec<f64> = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            f64::MIN_POSITIVE,
            5e-324,
            f64::MAX,
            -f64::MAX,
            1e16,
            1e17,
            1e-4,
            1e-5,
        ];
        let mut rng = Rng::new(0x4500);
        for _ in 0..400 {
            vals.push(rng.f64_interesting());
        }
        for v in vals {
            let j = (api.json_real)(v);
            rec.json("j", j);
            if !j.is_null() {
                rec.tag_f("val", (api.json_real_value)(j));
                rec.tag_f("num", (api.json_number_value)(j));
                rec.tag_i("set", (api.json_real_set)(j, -v) as i64);
                rec.tag_f("val2", (api.json_real_value)(j));
                rec_dump_all(api, rec, "d", j);
                decref(api, j);
            }
        }
        // non-finite rejected
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -f64::NAN] {
            rec.json("nonfinite", (api.json_real)(v));
        }
    });
}

#[test]
fn cfg46_number_value_all_types() {
    diff("cfg46 json_number_value", |api, rec| unsafe {
        let specs = [
            Spec::Int(-42),
            Spec::Real(1.5),
            Spec::Str("7".into()),
            Spec::True,
            Spec::False,
            Spec::Null,
            Spec::Arr(vec![]),
            Spec::Obj(vec![]),
        ];
        for s in &specs {
            let j = build(api, s);
            rec.tag_f("num", (api.json_number_value)(j));
            rec.tag_i("int", (api.json_integer_value)(j));
            rec.tag_f("real", (api.json_real_value)(j));
            rec.cstring("str", (api.json_string_value)(j));
            rec.tag_u("slen", (api.json_string_length)(j));
            decref(api, j);
        }
        rec.tag_f("num_null", (api.json_number_value)(ptr::null()));
        rec.tag_i("int_null", (api.json_integer_value)(ptr::null()));
        rec.tag_f("real_null", (api.json_real_value)(ptr::null()));
    });
}

/* ---------------------------------------------- rows 47..51: strings --- */

#[test]
fn cfg47to51_strings() {
    diff("cfg47-51 strings", |api, rec| unsafe {
        let mut rng = Rng::new(0x4700);
        // row 47/48: json_string / json_stringn
        let mut cases: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"hello world".to_vec(),
            "\u{80}\u{7FF}\u{800}\u{FFFF}\u{10000}\u{10FFFF}".as_bytes().to_vec(),
            b"tab\tnl\nquote\"back\\slash/".to_vec(),
            b"a\0b".to_vec(),
            b"\xff\xfe".to_vec(),
            b"\xc2".to_vec(),
            b"\xed\xa0\x80".to_vec(),
        ];
        for _ in 0..200 {
            let n = rng.below(20);
            cases.push(rng.utf8(n).into_bytes());
        }
        for _ in 0..100 {
            let n = rng.below(10);
            cases.push(rng.bytes(n, false));
        }
        for b in &cases {
            let z = cbuf(b);
            // json_string (NUL terminated, UTF-8 checked)
            let j1 = (api.json_string)(z.as_ptr() as *const c_char);
            rec.json("j1", j1);
            if !j1.is_null() {
                rec.cstring("j1v", (api.json_string_value)(j1));
                rec.tag_u("j1l", (api.json_string_length)(j1));
                rec_dump_all(api, rec, "j1d", j1);
                decref(api, j1);
            }
            // json_stringn with several lengths
            for l in [b.len(), b.len() + 1, b.len().saturating_sub(1), 0] {
                if l > z.len() {
                    continue;
                }
                let j = (api.json_stringn)(z.as_ptr() as *const c_char, l);
                rec.json("j2", j);
                if !j.is_null() {
                    rec.tag_u("j2l", (api.json_string_length)(j));
                    let v = (api.json_string_value)(j);
                    rec.tag_bytes("j2v", std::slice::from_raw_parts(v as *const u8, l + 1));
                    decref(api, j);
                }
                // row 49: nocheck variants accept anything
                let j3 = (api.json_stringn_nocheck)(z.as_ptr() as *const c_char, l);
                rec.json("j3", j3);
                if !j3.is_null() {
                    rec.tag_u("j3l", (api.json_string_length)(j3));
                    let v = (api.json_string_value)(j3);
                    rec.tag_bytes("j3v", std::slice::from_raw_parts(v as *const u8, l + 1));
                    decref(api, j3);
                }
            }
            let j4 = (api.json_string_nocheck)(z.as_ptr() as *const c_char);
            rec.json("j4", j4);
            if !j4.is_null() {
                rec.tag_u("j4l", (api.json_string_length)(j4));
                decref(api, j4);
            }
        }

        // row 50: jsonp_stringn_nocheck_own
        for b in [b"owned".to_vec(), b"".to_vec(), b"\xff".to_vec()] {
            let p = (api.jsonp_malloc)(b.len() + 1);
            ptr::copy_nonoverlapping(b.as_ptr(), p as *mut u8, b.len());
            *(p as *mut u8).add(b.len()) = 0;
            let j = (api.jsonp_stringn_nocheck_own)(p as *const c_char, b.len());
            rec.json("own", j);
            rec.tag_u("own_len", (api.json_string_length)(j));
            rec.cstring("own_val", (api.json_string_value)(j));
            decref(api, j);
        }
        // own with len 0 and a 1-byte buffer
        let p = (api.jsonp_malloc)(1);
        *(p as *mut u8) = 0;
        let j = (api.jsonp_stringn_nocheck_own)(p as *const c_char, 0);
        rec.json("own0", j);
        decref(api, j);

        // row 51: setters
        for setter in 0..4 {
            let base = cs("original");
            let j = (api.json_string)(base.as_ptr());
            for repl in [
                b"".to_vec(),
                b"x".to_vec(),
                b"much longer replacement value".to_vec(),
                "\u{263A}".as_bytes().to_vec(),
                b"\xff\xff".to_vec(),
                b"a\0b".to_vec(),
            ] {
                let z = cbuf(&repl);
                let r = match setter {
                    0 => (api.json_string_set)(j, z.as_ptr() as *const c_char),
                    1 => (api.json_string_setn)(j, z.as_ptr() as *const c_char, repl.len()),
                    2 => (api.json_string_set_nocheck)(j, z.as_ptr() as *const c_char),
                    _ => {
                        (api.json_string_setn_nocheck)(j, z.as_ptr() as *const c_char, repl.len())
                    }
                };
                rec.tag_i("set", r as i64);
                rec.tag_u("len", (api.json_string_length)(j));
                let v = (api.json_string_value)(j);
                if !v.is_null() {
                    rec.tag_bytes(
                        "val",
                        std::slice::from_raw_parts(
                            v as *const u8,
                            (api.json_string_length)(j) + 1,
                        ),
                    );
                }
            }
            decref(api, j);
        }
    });
}

/* ------------------------------------------ rows 52..62: objects ------- */

unsafe fn rec_object(rec: &mut Rec, api: &Api, tag: &str, o: *mut Json) {
    rec.tag_u(&format!("{tag}.size"), (api.json_object_size)(o));
    let mut it = (api.json_object_iter)(o);
    let mut n = 0;
    while !it.is_null() {
        let k = (api.json_object_iter_key)(it);
        let kl = (api.json_object_iter_key_len)(it);
        rec.tag_bytes(
            &format!("{tag}.k{n}"),
            std::slice::from_raw_parts(k as *const u8, kl),
        );
        let v = (api.json_object_iter_value)(it);
        rec.json(&format!("{tag}.v{n}"), v);
        // key_to_iter must round-trip
        let it2 = (api.json_object_key_to_iter)(k);
        rec.tag_i(&format!("{tag}.k2i{n}"), (it2 == it) as i64);
        it = (api.json_object_iter_next)(o, it);
        n += 1;
    }
    rec.tag_i(&format!("{tag}.n"), n);
    if let Some(d) = dumps(api, o, JSON_ENCODE_ANY) {
        rec.tag_bytes(&format!("{tag}.dump"), &d);
    } else {
        rec.line(&format!("{tag}.dump=NULL"));
    }
}

#[test]
fn cfg52to57_object_basics() {
    diff("cfg52-57 objects", |api, rec| unsafe {
        let o = (api.json_object)();
        // row 52: cross both rehash points
        for i in 0..64i64 {
            let key = format!("k{i:03}");
            let k = cs(&key);
            rec.tag_i(
                "set",
                (api.json_object_set_new)(o, k.as_ptr(), (api.json_integer)(i)) as i64,
            );
            if [0, 1, 7, 8, 9, 16, 17, 63].contains(&i) {
                rec_object(rec, api, &format!("s{i}"), o);
            }
        }
        // overwrite
        for i in 0..5i64 {
            let key = format!("k{i:03}");
            let k = cs(&key);
            rec.tag_i(
                "ow",
                (api.json_object_set_new)(o, k.as_ptr(), (api.json_integer)(-i)) as i64,
            );
        }
        rec_object(rec, api, "overwritten", o);
        // get / getn
        for probe in ["k000", "k063", "k064", "", "k0"] {
            let k = cs(probe);
            rec.json("get", (api.json_object_get)(o, k.as_ptr()));
            for kl in [probe.len(), probe.len() + 1] {
                rec.json("getn", (api.json_object_getn)(o, k.as_ptr(), kl));
            }
        }
        // row 54: delete
        for probe in ["k000", "k031", "k063", "k000", "nope"] {
            let k = cs(probe);
            rec.tag_i("del", (api.json_object_del)(o, k.as_ptr()) as i64);
        }
        rec_object(rec, api, "deleted", o);
        for probe in ["k001", "nope"] {
            let k = cs(probe);
            rec.tag_i(
                "deln",
                (api.json_object_deln)(o, k.as_ptr(), probe.len()) as i64,
            );
        }
        rec_object(rec, api, "delnd", o);

        // row 56: iter_at + row 57: iter_set_new
        let k = cs("k010");
        let it = (api.json_object_iter_at)(o, k.as_ptr());
        rec.tag_ptr_null("iter_at", it);
        if !it.is_null() {
            rec.tag_i(
                "iter_set",
                (api.json_object_iter_set_new)(o, it, (api.json_integer)(999)) as i64,
            );
            rec.json("iter_val", (api.json_object_iter_value)(it));
        }
        rec_object(rec, api, "iter_set", o);

        // row 55: clear + reuse
        rec.tag_i("clear", (api.json_object_clear)(o) as i64);
        rec_object(rec, api, "cleared", o);
        for i in 0..12i64 {
            let key = format!("r{i}");
            let k = cs(&key);
            rec.tag_i(
                "refill",
                (api.json_object_set_new)(o, k.as_ptr(), (api.json_integer)(i)) as i64,
            );
        }
        rec_object(rec, api, "refilled", o);
        decref(api, o);

        // row 53: keys with embedded NUL / invalid UTF-8 via nocheck
        let o2 = (api.json_object)();
        let keys: &[&[u8]] = &[b"", b"a", b"a\0b", b"\xff\xfe", b"\xc2\x80", b"a\0b"];
        for (i, kb) in keys.iter().enumerate() {
            let z = cbuf(kb);
            rec.tag_i(
                "setn_nocheck",
                (api.json_object_setn_new_nocheck)(
                    o2,
                    z.as_ptr() as *const c_char,
                    kb.len(),
                    (api.json_integer)(i as i64),
                ) as i64,
            );
            // the UTF-8 checking variant
            rec.tag_i(
                "setn_checked",
                (api.json_object_setn_new)(
                    o2,
                    z.as_ptr() as *const c_char,
                    kb.len(),
                    (api.json_integer)(100 + i as i64),
                ) as i64,
            );
        }
        rec_object(rec, api, "binkeys", o2);
        for kb in keys.iter() {
            let z = cbuf(kb);
            rec.json(
                "getn",
                (api.json_object_getn)(o2, z.as_ptr() as *const c_char, kb.len()),
            );
        }
        decref(api, o2);

        // randomised mixed workload
        let mut rng = Rng::new(0x5200);
        let o3 = (api.json_object)();
        for step in 0..500i64 {
            let key = format!("q{}", rng.below(30));
            let k = cs(&key);
            match rng.below(5) {
                0 | 1 => rec.tag_i(
                    "r_set",
                    (api.json_object_set_new)(o3, k.as_ptr(), (api.json_integer)(step)) as i64,
                ),
                2 => rec.tag_i("r_del", (api.json_object_del)(o3, k.as_ptr()) as i64),
                3 => rec.json("r_get", (api.json_object_get)(o3, k.as_ptr())),
                _ => {
                    let it = (api.json_object_iter_at)(o3, k.as_ptr());
                    if it.is_null() {
                        rec.line("r_iter=NULL");
                    } else {
                        rec.tag_i(
                            "r_iter_set",
                            (api.json_object_iter_set_new)(o3, it, (api.json_integer)(-step))
                                as i64,
                        );
                    }
                }
            }
            rec.tag_u("r_size", (api.json_object_size)(o3));
        }
        rec_object(rec, api, "rand", o3);
        decref(api, o3);
    });
}

#[test]
fn cfg58to61_object_update() {
    diff("cfg58-61 object update", |api, rec| unsafe {
        let variants: &[(&str, u32)] = &[
            ("update", 0),
            ("existing", 1),
            ("missing", 2),
            ("recursive", 3),
        ];
        let mut rng = Rng::new(0x5800);
        for (name, which) in variants {
            for _ in 0..60 {
                let sa = rand_container(&mut rng, 2);
                let sb = rand_container(&mut rng, 2);
                let a = build(api, &sa);
                let b = build(api, &sb);
                let r = match which {
                    0 => (api.json_object_update)(a, b),
                    1 => (api.json_object_update_existing)(a, b),
                    2 => (api.json_object_update_missing)(a, b),
                    _ => (api.json_object_update_recursive)(a, b),
                };
                rec.tag_i(&format!("{name}.ret"), r as i64);
                rec_dump_all(api, rec, &format!("{name}.a"), a);
                rec_dump_all(api, rec, &format!("{name}.b"), b);
                decref(api, a);
                decref(api, b);
            }
            // deterministic nested-merge shapes
            for (ta, tb) in [
                (r#"{"a":{"b":1,"c":2}}"#, r#"{"a":{"c":3,"d":4}}"#),
                (r#"{"a":{"b":1}}"#, r#"{"a":5}"#),
                (r#"{"a":5}"#, r#"{"a":{"b":1}}"#),
                (r#"{}"#, r#"{"a":1}"#),
                (r#"{"a":1}"#, r#"{}"#),
                (
                    r#"{"x":{"y":{"z":{"w":1}}}}"#,
                    r#"{"x":{"y":{"z":{"v":2},"q":3}}}"#,
                ),
            ] {
                let ca = cs(ta);
                let cb = cs(tb);
                let a = (api.json_loads)(ca.as_ptr(), 0, ptr::null_mut());
                let b = (api.json_loads)(cb.as_ptr(), 0, ptr::null_mut());
                let r = match which {
                    0 => (api.json_object_update)(a, b),
                    1 => (api.json_object_update_existing)(a, b),
                    2 => (api.json_object_update_missing)(a, b),
                    _ => (api.json_object_update_recursive)(a, b),
                };
                rec.tag_i(&format!("{name}.det_ret"), r as i64);
                rec_dump_all(api, rec, &format!("{name}.det_a"), a);
                decref(api, a);
                decref(api, b);
            }
            // self update
            let cs1 = cs(r#"{"a":1,"b":{"c":2}}"#);
            let a = (api.json_loads)(cs1.as_ptr(), 0, ptr::null_mut());
            let r = match which {
                0 => (api.json_object_update)(a, a),
                1 => (api.json_object_update_existing)(a, a),
                2 => (api.json_object_update_missing)(a, a),
                _ => (api.json_object_update_recursive)(a, a),
            };
            rec.tag_i(&format!("{name}.self_ret"), r as i64);
            rec_dump_all(api, rec, &format!("{name}.self"), a);
            decref(api, a);
        }
    });
}

#[test]
fn cfg62_do_object_update_recursive_lowlevel() {
    diff("cfg62 do_object_update_recursive", |api, rec| unsafe {
        for (ta, tb) in [
            (r#"{"a":{"b":1}}"#, r#"{"a":{"c":2}}"#),
            (r#"{}"#, r#"{"a":{"b":{"c":1}}}"#),
            (r#"{"a":1}"#, r#"{"a":{"b":1}}"#),
        ] {
            for preseed in [false, true] {
                let mut ht = Hashtable::zeroed();
                assert_eq!((api.hashtable_init)(&mut ht), 0);
                let ca = cs(ta);
                let cb = cs(tb);
                let a = (api.json_loads)(ca.as_ptr(), 0, ptr::null_mut());
                let b = (api.json_loads)(cb.as_ptr(), 0, ptr::null_mut());
                if preseed {
                    // pretend `b` is already on the parent stack -> cycle detected
                    let mut key = [0u8; 19];
                    let mut kl = 0usize;
                    (api.jsonp_loop_check)(
                        &mut ht,
                        b,
                        key.as_mut_ptr() as *mut c_char,
                        19,
                        &mut kl,
                    );
                }
                let r = (api.do_object_update_recursive)(a, b, &mut ht);
                rec.tag_i("ret", r as i64);
                rec.tag_u("parents_size", ht.size);
                rec_dump_all(api, rec, "a", a);
                decref(api, a);
                decref(api, b);
                (api.hashtable_close)(&mut ht);
            }
        }
        // non-object arguments
        let mut ht = Hashtable::zeroed();
        assert_eq!((api.hashtable_init)(&mut ht), 0);
        let arr = (api.json_array)();
        let obj = (api.json_object)();
        rec.tag_i(
            "arr_obj",
            (api.do_object_update_recursive)(arr, obj, &mut ht) as i64,
        );
        rec.tag_i(
            "obj_arr",
            (api.do_object_update_recursive)(obj, arr, &mut ht) as i64,
        );
        rec.tag_i(
            "null_obj",
            (api.do_object_update_recursive)(ptr::null_mut(), obj, &mut ht) as i64,
        );
        rec.tag_i(
            "obj_null",
            (api.do_object_update_recursive)(obj, ptr::null_mut(), &mut ht) as i64,
        );
        decref(api, arr);
        decref(api, obj);
        (api.hashtable_close)(&mut ht);
    });
}

/* ------------------------------------------ rows 63..69: arrays -------- */

unsafe fn rec_array(rec: &mut Rec, api: &Api, tag: &str, a: *mut Json) {
    let n = (api.json_array_size)(a);
    rec.tag_u(&format!("{tag}.size"), n);
    for i in 0..n {
        rec.json(&format!("{tag}.e{i}"), (api.json_array_get)(a, i));
    }
    // one past the end and SIZE_MAX
    rec.json(&format!("{tag}.oob"), (api.json_array_get)(a, n));
    rec.json(&format!("{tag}.oobmax"), (api.json_array_get)(a, usize::MAX));
    if let Some(d) = dumps(api, a, JSON_ENCODE_ANY) {
        rec.tag_bytes(&format!("{tag}.dump"), &d);
    } else {
        rec.line(&format!("{tag}.dump=NULL"));
    }
}

#[test]
fn cfg63to69_arrays() {
    diff("cfg63-69 arrays", |api, rec| unsafe {
        // row 63: append across the doubling boundaries
        let a = (api.json_array)();
        for i in 0..64i64 {
            rec.tag_i(
                "app",
                (api.json_array_append_new)(a, (api.json_integer)(i)) as i64,
            );
            if [0, 1, 7, 8, 9, 15, 16, 17, 63].contains(&i) {
                rec_array(rec, api, &format!("a{i}"), a);
            }
        }
        // row 65: set at every index
        for i in 0..(api.json_array_size)(a) {
            rec.tag_i(
                "set",
                (api.json_array_set_new)(a, i, (api.json_integer)(-(i as i64))) as i64,
            );
        }
        rec_array(rec, api, "allset", a);
        // out-of-range set
        for i in [(api.json_array_size)(a), usize::MAX] {
            rec.tag_i(
                "set_oob",
                (api.json_array_set_new)(a, i, (api.json_integer)(1)) as i64,
            );
        }
        // row 66: remove first/middle/last until empty
        while (api.json_array_size)(a) > 0 {
            let n = (api.json_array_size)(a);
            let idx = match n % 3 {
                0 => 0,
                1 => n - 1,
                _ => n / 2,
            };
            rec.tag_i("rm", (api.json_array_remove)(a, idx) as i64);
            rec.tag_u("rm_size", (api.json_array_size)(a));
        }
        rec_array(rec, api, "emptied", a);
        rec.tag_i("rm_empty", (api.json_array_remove)(a, 0) as i64);

        // row 64: insert
        for idx in [0usize, 0, 1, 2, 1, 0] {
            rec.tag_i(
                "ins",
                (api.json_array_insert_new)(a, idx, (api.json_integer)(idx as i64 + 100)) as i64,
            );
            rec_array(rec, api, &format!("ins{idx}"), a);
        }
        // insert at entries (== append) and beyond
        let n = (api.json_array_size)(a);
        rec.tag_i(
            "ins_end",
            (api.json_array_insert_new)(a, n, (api.json_integer)(777)) as i64,
        );
        rec.tag_i(
            "ins_past",
            (api.json_array_insert_new)(a, (api.json_array_size)(a) + 1, (api.json_integer)(1))
                as i64,
        );
        rec.tag_i(
            "ins_max",
            (api.json_array_insert_new)(a, usize::MAX, (api.json_integer)(1)) as i64,
        );
        rec_array(rec, api, "inserted", a);
        // insert across a growth boundary
        for i in 0..30i64 {
            (api.json_array_insert_new)(a, 0, (api.json_integer)(i));
        }
        rec_array(rec, api, "ins_grown", a);

        // row 67: clear + reuse
        rec.tag_i("clear", (api.json_array_clear)(a) as i64);
        rec_array(rec, api, "cleared", a);
        for i in 0..5i64 {
            (api.json_array_append_new)(a, (api.json_integer)(i));
        }
        rec_array(rec, api, "reused", a);
        decref(api, a);

        // row 68: extend
        for (na, nb) in [
            (0usize, 0usize),
            (0, 1),
            (1, 0),
            (3, 5),
            (7, 1),
            (8, 1),
            (8, 8),
            (16, 20),
        ] {
            let x = (api.json_array)();
            let y = (api.json_array)();
            for i in 0..na {
                (api.json_array_append_new)(x, (api.json_integer)(i as i64));
            }
            for i in 0..nb {
                (api.json_array_append_new)(y, (api.json_integer)(1000 + i as i64));
            }
            rec.tag_i("ext", (api.json_array_extend)(x, y) as i64);
            rec_array(rec, api, &format!("ext{na}_{nb}"), x);
            rec_array(rec, api, &format!("extother{na}_{nb}"), y);
            decref(api, x);
            decref(api, y);
        }
        // self extend
        let z = (api.json_array)();
        for i in 0..4i64 {
            (api.json_array_append_new)(z, (api.json_integer)(i));
        }
        rec.tag_i("self_ext", (api.json_array_extend)(z, z) as i64);
        rec_array(rec, api, "self_ext", z);
        decref(api, z);

        // randomised workload
        let mut rng = Rng::new(0x6300);
        let w = (api.json_array)();
        for step in 0..600i64 {
            let n = (api.json_array_size)(w);
            let idx = if n == 0 { 0 } else { rng.below(n + 2) };
            match rng.below(5) {
                0 | 1 => rec.tag_i(
                    "r_app",
                    (api.json_array_append_new)(w, (api.json_integer)(step)) as i64,
                ),
                2 => rec.tag_i(
                    "r_ins",
                    (api.json_array_insert_new)(w, idx, (api.json_integer)(step)) as i64,
                ),
                3 => rec.tag_i("r_rm", (api.json_array_remove)(w, idx) as i64),
                _ => rec.tag_i(
                    "r_set",
                    (api.json_array_set_new)(w, idx, (api.json_integer)(step)) as i64,
                ),
            }
            rec.tag_u("r_size", (api.json_array_size)(w));
        }
        rec_array(rec, api, "rand", w);
        decref(api, w);
    });
}

/* ------------------------------------------ rows 70..74: equal/copy ---- */

#[test]
fn cfg70_json_equal() {
    diff("cfg70 json_equal", |api, rec| unsafe {
        let specs = vec![
            Spec::Null,
            Spec::True,
            Spec::False,
            Spec::Int(0),
            Spec::Int(1),
            Spec::Int(i64::MIN),
            Spec::Real(0.0),
            Spec::Real(-0.0),
            Spec::Real(1.0),
            Spec::Str("".into()),
            Spec::Str("a".into()),
            Spec::StrRaw(b"a\0b".to_vec()),
            Spec::StrRaw(b"a\0c".to_vec()),
            Spec::Arr(vec![]),
            Spec::Arr(vec![Spec::Int(1)]),
            Spec::Arr(vec![Spec::Int(1), Spec::Int(2)]),
            Spec::Obj(vec![]),
            Spec::Obj(vec![(b"a".to_vec(), Spec::Int(1))]),
            Spec::Obj(vec![
                (b"a".to_vec(), Spec::Int(1)),
                (b"b".to_vec(), Spec::Int(2)),
            ]),
            Spec::Obj(vec![
                (b"b".to_vec(), Spec::Int(2)),
                (b"a".to_vec(), Spec::Int(1)),
            ]),
        ];
        let vals: Vec<*mut Json> = specs.iter().map(|s| build(api, s)).collect();
        for (i, a) in vals.iter().enumerate() {
            for (j, b) in vals.iter().enumerate() {
                rec.tag_i(
                    &format!("eq{i}_{j}"),
                    (api.json_equal)(*a, *b) as i64,
                );
            }
            rec.tag_i(&format!("eq_null_{i}"), (api.json_equal)(*a, ptr::null()) as i64);
            rec.tag_i(&format!("null_eq_{i}"), (api.json_equal)(ptr::null(), *a) as i64);
        }
        rec.tag_i("null_null", (api.json_equal)(ptr::null(), ptr::null()) as i64);
        for v in vals {
            decref(api, v);
        }
        // randomised: equal trees must compare equal, and their copies too
        let mut rng = Rng::new(0x7000);
        for _ in 0..150 {
            let s = rand_spec(&mut rng, 3);
            let a = build(api, &s);
            let b = build(api, &s);
            rec.tag_i("same_spec", (api.json_equal)(a, b) as i64);
            let c = (api.json_deep_copy)(a);
            rec.tag_i("deep_eq", (api.json_equal)(a, c) as i64);
            let d = (api.json_copy)(a);
            rec.tag_i("shallow_eq", (api.json_equal)(a, d) as i64);
            decref(api, a);
            decref(api, b);
            decref(api, c);
            decref(api, d);
        }
    });
}

#[test]
fn cfg71to73_copy() {
    diff("cfg71-73 copy", |api, rec| unsafe {
        let mut rng = Rng::new(0x7100);
        let mut specs: Vec<Spec> = vec![
            Spec::Null,
            Spec::True,
            Spec::False,
            Spec::Int(-9),
            Spec::Real(2.5),
            Spec::Str("txt".into()),
            Spec::StrRaw(b"a\0b".to_vec()),
            Spec::Arr(vec![]),
            Spec::Obj(vec![]),
            Spec::Arr(vec![Spec::Obj(vec![(b"k".to_vec(), Spec::Arr(vec![Spec::Int(1)]))])]),
        ];
        for _ in 0..150 {
            specs.push(rand_spec(&mut rng, 3));
        }
        for s in &specs {
            let a = build(api, s);
            rec.json("orig", a);
            let sh = (api.json_copy)(a);
            rec.json("shallow", sh);
            rec_dump_all(api, rec, "shallow_d", sh);
            // refcounts of the *shared children* are observable
            if !a.is_null() && (*a).type_ == JSON_ARRAY {
                let n = (api.json_array_size)(a);
                for i in 0..n {
                    rec.json(&format!("child{i}"), (api.json_array_get)(a, i));
                }
            }
            let dp = (api.json_deep_copy)(a);
            rec.json("deep", dp);
            rec_dump_all(api, rec, "deep_d", dp);
            rec.tag_i("deep_equal", (api.json_equal)(a, dp) as i64);
            decref(api, dp);
            decref(api, sh);
            decref(api, a);
        }
        // row 73: do_deep_copy with a caller supplied parents table
        for s in &specs[..10] {
            let mut ht = Hashtable::zeroed();
            assert_eq!((api.hashtable_init)(&mut ht), 0);
            let a = build(api, s);
            let c = (api.do_deep_copy)(a, &mut ht);
            rec.json("ddc", c);
            rec.tag_u("ddc_parents", ht.size);
            rec_dump_all(api, rec, "ddc_d", c);
            decref(api, c);
            decref(api, a);
            (api.hashtable_close)(&mut ht);
        }
        rec.json("copy_null", (api.json_copy)(ptr::null_mut()));
        rec.json("deep_null", (api.json_deep_copy)(ptr::null()));
        let mut ht = Hashtable::zeroed();
        assert_eq!((api.hashtable_init)(&mut ht), 0);
        rec.json("ddc_null", (api.do_deep_copy)(ptr::null(), &mut ht));
        (api.hashtable_close)(&mut ht);
    });
}

#[test]
fn cfg74_json_delete() {
    diff("cfg74 json_delete", |api, rec| unsafe {
        // deleting each container type must not crash and must free children
        let mut rng = Rng::new(0x7400);
        for _ in 0..200 {
            let s = rand_spec(&mut rng, 3);
            let a = build(api, &s);
            (api.json_delete)(a);
            rec.line("deleted");
        }
        // ERRORS.md row 100: json_delete(NULL) is a no-op
        (api.json_delete)(ptr::null_mut());
        rec.line("null_ok");
    });
}

/* ------------------------------------------------ row 75: sprintf ------ */

#[test]
fn cfg75_json_sprintf() {
    diff("cfg75 json_sprintf", |api, rec| unsafe {
        let f1 = cs("");
        rec.json("empty", (api.json_sprintf)(f1.as_ptr()));
        let j = (api.json_sprintf)(f1.as_ptr());
        rec.cstring("empty_val", (api.json_string_value)(j));
        rec.tag_u("empty_len", (api.json_string_length)(j));
        decref(api, j);

        let f2 = cs("hello %s, %d times, %.3f, %%, %c");
        let arg = cs("world");
        let j = (api.json_sprintf)(f2.as_ptr(), arg.as_ptr(), 42i32, 1.5f64, b'X' as i32);
        rec.json("j", j);
        rec.cstring("v", (api.json_string_value)(j));
        rec.tag_u("l", (api.json_string_length)(j));
        rec_dump_all(api, rec, "d", j);
        decref(api, j);

        // long result (forces the malloc path)
        let f3 = cs("%s");
        let long: String = (0..5000).map(|i| (b'a' + (i % 26) as u8) as char).collect();
        let lc = cs(&long);
        let j = (api.json_sprintf)(f3.as_ptr(), lc.as_ptr());
        rec.json("long", j);
        rec.tag_u("long_len", (api.json_string_length)(j));
        decref(api, j);

        // multi-byte UTF-8 through %s
        let u = cs("héllo → ☃ 𝄞");
        let j = (api.json_sprintf)(f3.as_ptr(), u.as_ptr());
        rec.json("utf8", j);
        rec.cstring("utf8_v", (api.json_string_value)(j));
        rec_dump_all(api, rec, "utf8_d", j);
        decref(api, j);

        // invalid UTF-8 through %s -> NULL (ERRORS.md row 87)
        let bad = cbuf(b"\xff\xfe");
        let j = (api.json_sprintf)(f3.as_ptr(), bad.as_ptr() as *const c_char);
        rec.json("invalid_utf8", j);
        decref(api, j);

        // many numeric formats
        let f4 = cs("%d|%i|%u|%x|%X|%o|%e|%E|%g|%G|%ld|%lld|%zu|%p_ok");
        let j = (api.json_sprintf)(
            f4.as_ptr(),
            -1i32,
            2i32,
            3u32,
            255u32,
            255u32,
            8u32,
            1.5f64,
            1.5f64,
            0.0001f64,
            123456789.0f64,
            -5i64,
            -6i64,
            7usize,
            ptr::null::<c_void>(),
        );
        rec.json("numfmt", j);
        // %p prints a pointer; NULL renders identically in both libraries
        rec.cstring("numfmt_v", (api.json_string_value)(j));
        decref(api, j);
    });
}
