//! Phase C — error-path differential tests for `value.c`
//! (ERRORS.md rows 1..115).

mod common;
use common::tree::*;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

/// One value of every valid type plus forged out-of-range type tags and NULL.
/// Rows 5, 8, 13, 21, 23..28, 32, 35, 37, 43, 49, 50, 53, 57, 61, 65, 67..69,
/// 78, 79, 81, 90, 91, 96, 97, 99, 101, 104, 105, 107, 112, 294.
unsafe fn zoo(api: &Api) -> Vec<(&'static str, *mut Json)> {
    vec![
        ("object", (api.json_object)()),
        ("array", (api.json_array)()),
        ("string", (api.json_string)(cs("s").as_ptr())),
        ("integer", (api.json_integer)(1)),
        ("real", (api.json_real)(1.0)),
        ("true", (api.json_true)()),
        ("false", (api.json_false)()),
        ("null", (api.json_null)()),
        ("forged8", forge_json(api, 8, 1)),
        ("forged255", forge_json(api, 255, 1)),
        ("forged_neg", forge_json(api, -1, 1)),
        ("forged_max", forge_json(api, c_int::MAX, 1)),
        ("NULL", ptr::null_mut()),
    ]
}

/* --------------------------------- type-guard matrix over every getter --- */

#[test]
fn err_type_guards_matrix() {
    diff("ERRORS type-guard matrix", |api, rec| unsafe {
        let vals = zoo(api);
        let k = cs("key");
        for (name, v) in &vals {
            let v = *v;
            // row 4/5/49/79/90/96/99: size / length / value getters
            rec.tag_u(&format!("{name}.object_size"), (api.json_object_size)(v));
            rec.tag_u(&format!("{name}.array_size"), (api.json_array_size)(v));
            rec.tag_u(&format!("{name}.string_length"), (api.json_string_length)(v));
            rec.cstring(&format!("{name}.string_value"), (api.json_string_value)(v));
            rec.tag_i(&format!("{name}.integer_value"), (api.json_integer_value)(v));
            rec.tag_f(&format!("{name}.real_value"), (api.json_real_value)(v));
            rec.tag_f(&format!("{name}.number_value"), (api.json_number_value)(v));

            // rows 6..9: object lookup
            rec.json(&format!("{name}.object_get"), (api.json_object_get)(v, k.as_ptr()));
            rec.json(
                &format!("{name}.object_get_nullkey"),
                (api.json_object_get)(v, ptr::null()),
            );
            rec.json(
                &format!("{name}.object_getn"),
                (api.json_object_getn)(v, k.as_ptr(), 3),
            );
            rec.json(
                &format!("{name}.object_getn_nullkey"),
                (api.json_object_getn)(v, ptr::null(), 3),
            );

            // rows 19..23: deletion / clear
            rec.tag_i(
                &format!("{name}.object_del"),
                (api.json_object_del)(v, k.as_ptr()) as i64,
            );
            rec.tag_i(
                &format!("{name}.object_del_nullkey"),
                (api.json_object_del)(v, ptr::null()) as i64,
            );
            rec.tag_i(
                &format!("{name}.object_deln"),
                (api.json_object_deln)(v, k.as_ptr(), 3) as i64,
            );
            rec.tag_i(
                &format!("{name}.object_deln_nullkey"),
                (api.json_object_deln)(v, ptr::null(), 3) as i64,
            );
            rec.tag_i(
                &format!("{name}.object_clear"),
                (api.json_object_clear)(v) as i64,
            );
            rec.tag_i(
                &format!("{name}.array_clear"),
                (api.json_array_clear)(v) as i64,
            );

            // rows 32..46: iteration
            rec.tag_ptr_null(&format!("{name}.iter"), (api.json_object_iter)(v));
            rec.tag_ptr_null(
                &format!("{name}.iter_at"),
                (api.json_object_iter_at)(v, k.as_ptr()),
            );
            rec.tag_ptr_null(
                &format!("{name}.iter_at_nullkey"),
                (api.json_object_iter_at)(v, ptr::null()),
            );
            rec.tag_ptr_null(
                &format!("{name}.iter_next_null"),
                (api.json_object_iter_next)(v, ptr::null_mut()),
            );
            rec.tag_i(
                &format!("{name}.iter_set_null_iter"),
                (api.json_object_iter_set_new)(v, ptr::null_mut(), (api.json_integer)(1)) as i64,
            );
            rec.tag_i(
                &format!("{name}.iter_set_null_value"),
                (api.json_object_iter_set_new)(v, 1usize as *mut c_void, ptr::null_mut()) as i64,
            );

            // rows 50..70: array accessors
            for idx in [0usize, 1, usize::MAX] {
                rec.json(
                    &format!("{name}.array_get{idx}"),
                    (api.json_array_get)(v, idx),
                );
                rec.tag_i(
                    &format!("{name}.array_set{idx}"),
                    (api.json_array_set_new)(v, idx, (api.json_integer)(1)) as i64,
                );
                rec.tag_i(
                    &format!("{name}.array_set{idx}_nullval"),
                    (api.json_array_set_new)(v, idx, ptr::null_mut()) as i64,
                );
                rec.tag_i(
                    &format!("{name}.array_insert{idx}"),
                    (api.json_array_insert_new)(v, idx, (api.json_integer)(1)) as i64,
                );
                rec.tag_i(
                    &format!("{name}.array_remove{idx}"),
                    (api.json_array_remove)(v, idx) as i64,
                );
            }
            rec.tag_i(
                &format!("{name}.array_append"),
                (api.json_array_append_new)(v, (api.json_integer)(1)) as i64,
            );
            rec.tag_i(
                &format!("{name}.array_append_nullval"),
                (api.json_array_append_new)(v, ptr::null_mut()) as i64,
            );

            // rows 80..86: string setters
            let sv = cs("new");
            rec.tag_i(
                &format!("{name}.string_set"),
                (api.json_string_set)(v, sv.as_ptr()) as i64,
            );
            rec.tag_i(
                &format!("{name}.string_set_null"),
                (api.json_string_set)(v, ptr::null()) as i64,
            );
            rec.tag_i(
                &format!("{name}.string_setn"),
                (api.json_string_setn)(v, sv.as_ptr(), 3) as i64,
            );
            rec.tag_i(
                &format!("{name}.string_setn_null"),
                (api.json_string_setn)(v, ptr::null(), 3) as i64,
            );
            rec.tag_i(
                &format!("{name}.string_set_nocheck"),
                (api.json_string_set_nocheck)(v, sv.as_ptr()) as i64,
            );
            rec.tag_i(
                &format!("{name}.string_set_nocheck_null"),
                (api.json_string_set_nocheck)(v, ptr::null()) as i64,
            );
            rec.tag_i(
                &format!("{name}.string_setn_nocheck"),
                (api.json_string_setn_nocheck)(v, sv.as_ptr(), 3) as i64,
            );
            rec.tag_i(
                &format!("{name}.string_setn_nocheck_null"),
                (api.json_string_setn_nocheck)(v, ptr::null(), 3) as i64,
            );

            // rows 91/97/98: numeric setters
            rec.tag_i(
                &format!("{name}.integer_set"),
                (api.json_integer_set)(v, 7) as i64,
            );
            rec.tag_i(&format!("{name}.real_set"), (api.json_real_set)(v, 7.0) as i64);
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                rec.tag_i(
                    &format!("{name}.real_set_bad"),
                    (api.json_real_set)(v, bad) as i64,
                );
            }

            // rows 106/107/111/112: copies
            let c = (api.json_copy)(v);
            rec.json(&format!("{name}.copy"), c);
            if !c.is_null() && c != v {
                decref(api, c);
            }
            let d = (api.json_deep_copy)(v);
            rec.json(&format!("{name}.deep_copy"), d);
            if !d.is_null() && d != v {
                decref(api, d);
            }

            // rows 24..28: object updates against every type
            for (oname, o) in &vals {
                let o = *o;
                rec.tag_i(
                    &format!("{name}_{oname}.update"),
                    (api.json_object_update)(v, o) as i64,
                );
                rec.tag_i(
                    &format!("{name}_{oname}.update_existing"),
                    (api.json_object_update_existing)(v, o) as i64,
                );
                rec.tag_i(
                    &format!("{name}_{oname}.update_missing"),
                    (api.json_object_update_missing)(v, o) as i64,
                );
                rec.tag_i(
                    &format!("{name}_{oname}.update_recursive"),
                    (api.json_object_update_recursive)(v, o) as i64,
                );
                // rows 68..69: array extend
                rec.tag_i(
                    &format!("{name}_{oname}.extend"),
                    (api.json_array_extend)(v, o) as i64,
                );
                // rows 102..105: equality
                rec.tag_i(&format!("{name}_{oname}.equal"), (api.json_equal)(v, o) as i64);
            }
        }
        // rows 40..42/46: iterator accessors with NULL
        rec.cstring("iter_key_null", (api.json_object_iter_key)(ptr::null_mut()));
        rec.tag_u(
            "iter_key_len_null",
            (api.json_object_iter_key_len)(ptr::null_mut()),
        );
        rec.json(
            "iter_value_null",
            (api.json_object_iter_value)(ptr::null_mut()),
        );
        rec.tag_ptr_null(
            "key_to_iter_null",
            (api.json_object_key_to_iter)(ptr::null()),
        );
    });
}

/* ------------------- rows 10..18: object insertion rejections ----------- */

#[test]
fn err10to18_object_set_rejections() {
    diff("ERRORS 10-18 object set", |api, rec| unsafe {
        let o = (api.json_object)();
        let k = cs("k");

        // row 11: value == NULL
        rec.tag_i(
            "setn_nocheck_nullval",
            (api.json_object_setn_new_nocheck)(o, k.as_ptr(), 1, ptr::null_mut()) as i64,
        );
        rec.tag_i(
            "setn_nullval",
            (api.json_object_setn_new)(o, k.as_ptr(), 1, ptr::null_mut()) as i64,
        );
        rec.tag_i(
            "set_nullval",
            (api.json_object_set_new)(o, k.as_ptr(), ptr::null_mut()) as i64,
        );
        rec.tag_i(
            "set_nocheck_nullval",
            (api.json_object_set_new_nocheck)(o, k.as_ptr(), ptr::null_mut()) as i64,
        );

        // rows 10/12/16/17: key == NULL (value must be decreffed)
        for (tag, r) in [
            ("set_new_nullkey", {
                let v = (api.json_integer)(1);
                (api.json_object_set_new)(o, ptr::null(), v)
            }),
            ("set_new_nocheck_nullkey", {
                let v = (api.json_integer)(1);
                (api.json_object_set_new_nocheck)(o, ptr::null(), v)
            }),
            ("setn_new_nullkey", {
                let v = (api.json_integer)(1);
                (api.json_object_setn_new)(o, ptr::null(), 1, v)
            }),
            ("setn_new_nocheck_nullkey", {
                let v = (api.json_integer)(1);
                (api.json_object_setn_new_nocheck)(o, ptr::null(), 1, v)
            }),
        ] {
            rec.tag_i(tag, r as i64);
        }

        // row 14: json == value (needs an extra reference, as json_object_set does)
        incref(api, o);
        rec.tag_i(
            "self_insert",
            (api.json_object_setn_new_nocheck)(o, k.as_ptr(), 1, o) as i64,
        );
        rec.json("o_after_self_insert", o);

        // row 18: invalid UTF-8 key rejected by the checking variant
        let bad_keys: &[&[u8]] = &[
            b"\xff",
            b"\xc2",
            b"\xc0\x80",
            b"\xed\xa0\x80",
            b"\xf5\x80\x80\x80",
            b"a\xffb",
        ];
        for kb in bad_keys {
            let z = cbuf(kb);
            let v = (api.json_integer)(1);
            rec.tag_i(
                "setn_new_badutf8",
                (api.json_object_setn_new)(o, z.as_ptr() as *const c_char, kb.len(), v) as i64,
            );
            let v = (api.json_integer)(1);
            rec.tag_i(
                "set_new_badutf8",
                (api.json_object_set_new)(o, z.as_ptr() as *const c_char, v) as i64,
            );
            // nocheck accepts it
            let v = (api.json_integer)(2);
            rec.tag_i(
                "setn_nocheck_badutf8",
                (api.json_object_setn_new_nocheck)(o, z.as_ptr() as *const c_char, kb.len(), v)
                    as i64,
            );
        }
        rec.tag_u("size", (api.json_object_size)(o));
        rec_dump_all(api, rec, "o", o);
        decref(api, o);
    });
}

/* --------------------- rows 52..70: array insertion rejections ---------- */

#[test]
fn err52to70_array_rejections() {
    diff("ERRORS 52-70 array", |api, rec| unsafe {
        let a = (api.json_array)();
        for i in 0..3i64 {
            (api.json_array_append_new)(a, (api.json_integer)(i));
        }
        // rows 54/58/62: json == value
        incref(api, a);
        rec.tag_i("set_self", (api.json_array_set_new)(a, 0, a) as i64);
        incref(api, a);
        rec.tag_i("append_self", (api.json_array_append_new)(a, a) as i64);
        incref(api, a);
        rec.tag_i("insert_self", (api.json_array_insert_new)(a, 0, a) as i64);
        rec.json("a_after_self", a);

        // rows 55/63/66: index out of range
        for idx in [3usize, 4, 100, usize::MAX, usize::MAX - 1] {
            rec.tag_i(
                &format!("set{idx}"),
                (api.json_array_set_new)(a, idx, (api.json_integer)(1)) as i64,
            );
            rec.tag_i(
                &format!("insert{idx}"),
                (api.json_array_insert_new)(a, idx, (api.json_integer)(1)) as i64,
            );
            rec.tag_i(&format!("remove{idx}"), (api.json_array_remove)(a, idx) as i64);
            rec.json(&format!("get{idx}"), (api.json_array_get)(a, idx));
        }
        // insert at exactly `entries` is legal
        rec.tag_i(
            "insert_at_entries",
            (api.json_array_insert_new)(a, (api.json_array_size)(a), (api.json_integer)(9)) as i64,
        );
        rec_dump_all(api, rec, "a", a);
        // empty array: every index is out of range
        let e = (api.json_array)();
        for idx in [0usize, 1, usize::MAX] {
            rec.json(&format!("empty_get{idx}"), (api.json_array_get)(e, idx));
            rec.tag_i(
                &format!("empty_set{idx}"),
                (api.json_array_set_new)(e, idx, (api.json_integer)(1)) as i64,
            );
            rec.tag_i(
                &format!("empty_remove{idx}"),
                (api.json_array_remove)(e, idx) as i64,
            );
        }
        rec.tag_i(
            "empty_insert0",
            (api.json_array_insert_new)(e, 0, (api.json_integer)(1)) as i64,
        );
        decref(api, e);
        decref(api, a);
    });
}

/* ------------------------ rows 71..77, 87: string rejections ------------ */

#[test]
fn err71to87_string_rejections() {
    diff("ERRORS 71-87 strings", |api, rec| unsafe {
        // rows 71/74/75/76: NULL value
        rec.json("string_null", (api.json_string)(ptr::null()));
        rec.json("stringn_null", (api.json_stringn)(ptr::null(), 0));
        rec.json("stringn_null5", (api.json_stringn)(ptr::null(), 5));
        rec.json("string_nocheck_null", (api.json_string_nocheck)(ptr::null()));
        rec.json(
            "stringn_nocheck_null",
            (api.json_stringn_nocheck)(ptr::null(), 0),
        );
        rec.json("own_null", (api.jsonp_stringn_nocheck_own)(ptr::null(), 0));

        // row 77: every utf8_check_string failure class
        let bad: &[&[u8]] = &[
            b"\x80",
            b"\xbf",
            b"\xc0\x80",
            b"\xc1\xbf",
            b"\xf5\x80\x80\x80",
            b"\xff",
            b"\xfe",
            b"\xc2",
            b"\xc2\x41",
            b"\xe0\xa0",
            b"\xe0\x80\x80",
            b"\xed\xa0\x80",
            b"\xed\xbf\xbf",
            b"\xf0\x80\x80\x80",
            b"\xf4\x90\x80\x80",
            b"\xf0\x90\x80",
            b"ok\xffbad",
        ];
        for b in bad {
            let z = cbuf(b);
            rec.json(
                "stringn_bad",
                (api.json_stringn)(z.as_ptr() as *const c_char, b.len()),
            );
            rec.json("string_bad", (api.json_string)(z.as_ptr() as *const c_char));
            // nocheck accepts
            let ok = (api.json_stringn_nocheck)(z.as_ptr() as *const c_char, b.len());
            rec.json("stringn_nocheck_bad", ok);
            rec.tag_u("len", (api.json_string_length)(ok));
            // dumping invalid UTF-8 fails (row 119)
            match dumps(api, ok, JSON_ENCODE_ANY) {
                None => rec.line("dump=NULL"),
                Some(d) => rec.tag_bytes("dump", &d),
            }
            decref(api, ok);

            // rows 86: json_string_setn rejects it too
            let s = (api.json_string)(cs("orig").as_ptr());
            rec.tag_i(
                "setn_bad",
                (api.json_string_setn)(s, z.as_ptr() as *const c_char, b.len()) as i64,
            );
            rec.tag_i(
                "set_bad",
                (api.json_string_set)(s, z.as_ptr() as *const c_char) as i64,
            );
            rec.cstring("still", (api.json_string_value)(s));
            decref(api, s);
        }

        // row 87: json_sprintf with an invalid-UTF-8 result
        let f = cs("%s");
        for b in [&b"\xff"[..], &b"a\xc2b"[..], &b"\xed\xa0\x80"[..]] {
            let z = cbuf(b);
            rec.json(
                "sprintf_bad",
                (api.json_sprintf)(f.as_ptr(), z.as_ptr() as *const c_char),
            );
        }
    });
}

/* ------------------------------ rows 92..98: non-finite reals ----------- */

#[test]
fn err92to98_non_finite_reals() {
    diff("ERRORS 92-98 non-finite reals", |api, rec| unsafe {
        let bad = [
            f64::NAN,
            -f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
            f64::from_bits(0xFFF8_0000_0000_0000),
            f64::from_bits(0x7FF8_0000_0000_0000),
        ];
        for v in bad {
            rec.json("real", (api.json_real)(v));
            let r = (api.json_real)(1.0);
            rec.tag_i("real_set", (api.json_real_set)(r, v) as i64);
            rec.tag_f("unchanged", (api.json_real_value)(r));
            decref(api, r);
        }
        // finite extremes are accepted
        for v in [f64::MAX, -f64::MAX, 5e-324, -5e-324, 0.0, -0.0] {
            let j = (api.json_real)(v);
            rec.json("finite", j);
            rec.tag_f("val", (api.json_real_value)(j));
            decref(api, j);
        }
    });
}

/* ------------------- rows 1, 29, 31, 113..115: cycle detection ---------- */

#[test]
fn err1_29_31_113to115_cycles() {
    diff("ERRORS cycles", |api, rec| unsafe {
        // ---- array cycle ----
        let a = (api.json_array)();
        let b = (api.json_array)();
        rec.tag_i("app_b", (api.json_array_append_new)(a, b) as i64);
        incref(api, a);
        rec.tag_i("app_a", (api.json_array_append_new)(b, a) as i64);
        rec.json("a", a);
        rec.json("b", b);
        // row 114/115: deep copy of a cycle
        rec.json("deep_copy", (api.json_deep_copy)(a));
        rec.json("deep_copy_b", (api.json_deep_copy)(b));
        // row 124: dumping a cycle
        for f in [
            0usize,
            JSON_ENCODE_ANY,
            JSON_SORT_KEYS,
            json_indent(2),
            JSON_COMPACT,
        ] {
            match dumps(api, a, f) {
                None => rec.line("dump=NULL"),
                Some(d) => rec.tag_bytes("dump", &d),
            }
            rec.tag_u("dumpb", (api.json_dumpb)(a, ptr::null_mut(), 0, f));
        }
        // break the cycle
        (api.json_array_clear)(b);
        decref(api, a);

        // ---- object cycle ----
        let o = (api.json_object)();
        let p = (api.json_object)();
        let k = cs("k");
        rec.tag_i(
            "set_p",
            (api.json_object_set_new)(o, k.as_ptr(), p) as i64,
        );
        incref(api, o);
        rec.tag_i(
            "set_o",
            (api.json_object_set_new)(p, k.as_ptr(), o) as i64,
        );
        // row 113/115
        rec.json("obj_deep_copy", (api.json_deep_copy)(o));
        rec.json("obj_deep_copy_p", (api.json_deep_copy)(p));
        // row 125
        for f in [0usize, JSON_SORT_KEYS, json_indent(2), JSON_COMPACT] {
            match dumps(api, o, f) {
                None => rec.line("obj_dump=NULL"),
                Some(d) => rec.tag_bytes("obj_dump", &d),
            }
        }
        // row 29/31: recursive update on a cyclic graph
        rec.tag_i(
            "update_recursive_self",
            (api.json_object_update_recursive)(o, o) as i64,
        );
        rec.tag_i(
            "update_recursive_p",
            (api.json_object_update_recursive)(o, p) as i64,
        );
        // plain updates do not recurse, so they succeed
        rec.tag_i("update_self", (api.json_object_update)(o, o) as i64);
        (api.json_object_clear)(p);
        decref(api, o);

        // row 1: jsonp_loop_check duplicate
        let mut ht = Hashtable::zeroed();
        assert_eq!((api.hashtable_init)(&mut ht), 0);
        let x = (api.json_array)();
        let mut key = [0u8; 19];
        let mut kl = 0usize;
        rec.tag_i(
            "loop1",
            (api.jsonp_loop_check)(&mut ht, x, key.as_mut_ptr() as *mut c_char, 19, &mut kl) as i64,
        );
        rec.tag_i(
            "loop2",
            (api.jsonp_loop_check)(&mut ht, x, key.as_mut_ptr() as *mut c_char, 19, &mut kl) as i64,
        );
        rec.tag_i(
            "loop3",
            (api.jsonp_loop_check)(&mut ht, x, key.as_mut_ptr() as *mut c_char, 19, &mut kl) as i64,
        );
        (api.hashtable_close)(&mut ht);
        decref(api, x);

        // rows 113/114 through the low-level do_deep_copy with a pre-seeded table
        let mut ht = Hashtable::zeroed();
        assert_eq!((api.hashtable_init)(&mut ht), 0);
        let arr = (api.json_array)();
        (api.json_array_append_new)(arr, (api.json_integer)(1));
        let mut kl = 0usize;
        (api.jsonp_loop_check)(
            &mut ht,
            arr,
            key.as_mut_ptr() as *mut c_char,
            19,
            &mut kl,
        );
        rec.json("ddc_preseeded", (api.do_deep_copy)(arr, &mut ht));
        (api.hashtable_close)(&mut ht);
        decref(api, arr);

        let mut ht = Hashtable::zeroed();
        assert_eq!((api.hashtable_init)(&mut ht), 0);
        let obj = (api.json_object)();
        (api.json_object_set_new)(obj, k.as_ptr(), (api.json_integer)(1));
        let mut kl = 0usize;
        (api.jsonp_loop_check)(
            &mut ht,
            obj,
            key.as_mut_ptr() as *mut c_char,
            19,
            &mut kl,
        );
        rec.json("ddc_obj_preseeded", (api.do_deep_copy)(obj, &mut ht));
        (api.hashtable_close)(&mut ht);
        decref(api, obj);
    });
}

/* ----------- rows 2,3,15,30,47,48,59,64,70,72,73,83,88,89,95,108,109,110 - */

#[test]
fn err_oom_constructors() {
    diff("ERRORS constructor OOM", |api, rec| unsafe {
        // rows 2/3: json_object
        oom_sweep(api, rec, "json_object", 8, |api, rec| unsafe {
            let o = (api.json_object)();
            rec.json("o", o);
            decref(api, o);
        });
        // rows 47/48: json_array
        oom_sweep(api, rec, "json_array", 8, |api, rec| unsafe {
            let a = (api.json_array)();
            rec.json("a", a);
            decref(api, a);
        });
        // row 89: json_integer
        oom_sweep(api, rec, "json_integer", 8, |api, rec| unsafe {
            let j = (api.json_integer)(5);
            rec.json("j", j);
            decref(api, j);
        });
        // row 95: json_real
        oom_sweep(api, rec, "json_real", 8, |api, rec| unsafe {
            let j = (api.json_real)(5.5);
            rec.json("j", j);
            decref(api, j);
        });
        // rows 72/73: string_create (strndup then header malloc)
        oom_sweep(api, rec, "json_string", 8, |api, rec| unsafe {
            let s = cs("some text");
            let j = (api.json_string)(s.as_ptr());
            rec.json("j", j);
            rec.cstring("v", (api.json_string_value)(j));
            decref(api, j);
        });
        // row 83: json_string_setn_nocheck strndup failure
        oom_sweep(api, rec, "string_setn", 12, |api, rec| unsafe {
            let s = cs("orig");
            let j = (api.json_string)(s.as_ptr());
            if !j.is_null() {
                let n = cs("replacement");
                rec.tag_i("set", (api.json_string_setn_nocheck)(j, n.as_ptr(), 11) as i64);
                rec.cstring("v", (api.json_string_value)(j));
            }
            rec.json("j", j);
            decref(api, j);
        });
        // row 15: hashtable_set failure inside json_object_setn_new_nocheck
        oom_sweep(api, rec, "object_set", 24, |api, rec| unsafe {
            let o = (api.json_object)();
            if !o.is_null() {
                for i in 0..3i64 {
                    let k = cs(&format!("k{i}"));
                    rec.tag_i(
                        "set",
                        (api.json_object_set_new)(o, k.as_ptr(), (api.json_integer)(i)) as i64,
                    );
                }
                rec.tag_u("size", (api.json_object_size)(o));
            }
            rec.json("o", o);
            decref(api, o);
        });
        // rows 59/64: json_array_grow failure (array is full at 8 entries)
        oom_sweep(api, rec, "array_grow", 24, |api, rec| unsafe {
            let a = (api.json_array)();
            if !a.is_null() {
                for i in 0..9i64 {
                    rec.tag_i(
                        "app",
                        (api.json_array_append_new)(a, (api.json_integer)(i)) as i64,
                    );
                }
                rec.tag_u("size", (api.json_array_size)(a));
            }
            rec.json("a", a);
            decref(api, a);
        });
        oom_sweep(api, rec, "array_insert_grow", 24, |api, rec| unsafe {
            let a = (api.json_array)();
            if !a.is_null() {
                for i in 0..9i64 {
                    rec.tag_i(
                        "ins",
                        (api.json_array_insert_new)(a, 0, (api.json_integer)(i)) as i64,
                    );
                }
                rec.tag_u("size", (api.json_array_size)(a));
            }
            rec.json("a", a);
            decref(api, a);
        });
        // row 70: json_array_extend grow failure
        oom_sweep(api, rec, "array_extend", 32, |api, rec| unsafe {
            let a = (api.json_array)();
            let b = (api.json_array)();
            if !a.is_null() && !b.is_null() {
                for i in 0..8i64 {
                    (api.json_array_append_new)(b, (api.json_integer)(i));
                }
                rec.tag_i("ext", (api.json_array_extend)(a, b) as i64);
                rec.tag_u("size", (api.json_array_size)(a));
            }
            decref(api, a);
            decref(api, b);
        });
        // row 88: json_sprintf buffer allocation failure
        oom_sweep(api, rec, "sprintf", 8, |api, rec| unsafe {
            let f = cs("%s-%d");
            let a = cs("arg");
            let j = (api.json_sprintf)(f.as_ptr(), a.as_ptr(), 12i32);
            rec.json("j", j);
            rec.cstring("v", (api.json_string_value)(j));
            decref(api, j);
        });
        // rows 108/109: json_copy of a container
        oom_sweep(api, rec, "copy_obj", 24, |api, rec| unsafe {
            let s = cs(r#"{"a":1,"b":[1,2]}"#);
            let o = (api.json_loads)(s.as_ptr(), 0, ptr::null_mut());
            if !o.is_null() {
                alloc_reset_keep();
                let c = (api.json_copy)(o);
                rec.json("c", c);
                rec_dump_all(api, rec, "c", c);
                decref(api, c);
            }
            decref(api, o);
        });
        // row 110: json_deep_copy hashtable_init failure
        oom_sweep(api, rec, "deep_copy", 40, |api, rec| unsafe {
            let s = cs(r#"{"a":1,"b":[1,2,"x"]}"#);
            let o = (api.json_loads)(s.as_ptr(), 0, ptr::null_mut());
            if !o.is_null() {
                let c = (api.json_deep_copy)(o);
                rec.json("c", c);
                rec_dump_all(api, rec, "c", c);
                decref(api, c);
            }
            decref(api, o);
        });
        // row 30: json_object_update_recursive hashtable_init failure
        oom_sweep(api, rec, "update_recursive", 40, |api, rec| unsafe {
            let s1 = cs(r#"{"a":{"b":1}}"#);
            let s2 = cs(r#"{"a":{"c":2},"d":3}"#);
            let o = (api.json_loads)(s1.as_ptr(), 0, ptr::null_mut());
            let p = (api.json_loads)(s2.as_ptr(), 0, ptr::null_mut());
            if !o.is_null() && !p.is_null() {
                rec.tag_i("ret", (api.json_object_update_recursive)(o, p) as i64);
                rec_dump_all(api, rec, "o", o);
            }
            decref(api, o);
            decref(api, p);
        });
    });
}

/// `oom_sweep` resets the counter between runs; this is a no-op placeholder so
/// the closure above reads naturally.
fn alloc_reset_keep() {}

/* ------------------------------ row 100/101: json_delete edge cases ----- */

#[test]
fn err100and101_json_delete() {
    diff("ERRORS 100-101 json_delete", |api, rec| unsafe {
        (api.json_delete)(ptr::null_mut());
        (api.json_delete)((api.json_true)());
        (api.json_delete)((api.json_false)());
        (api.json_delete)((api.json_null)());
        rec.json("true_alive", (api.json_true)());
        rec.json("false_alive", (api.json_false)());
        rec.json("null_alive", (api.json_null)());
        // forged out-of-range tags hit `default: return` and are not freed
        for t in [8, 9, 255, -1, c_int::MAX, c_int::MIN] {
            let p = forge_json(api, t, 1);
            (api.json_delete)(p);
            rec.json("forged_after_delete", p);
            (api.jsonp_free)(p as *mut c_void);
        }
        rec.line("ok");
    });
}

/* --------------------------- row 294: forged tags through the encoder --- */

#[test]
fn err294_forged_type_tags_everywhere() {
    diff("ERRORS 294 forged tags", |api, rec| unsafe {
        for t in [8, 9, 100, 255, 256, -1, c_int::MIN, c_int::MAX] {
            let p = forge_json(api, t, 1);
            rec.json("forged", p);
            for f in [
                0usize,
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_COMPACT,
                JSON_ENCODE_ANY | JSON_SORT_KEYS,
                usize::MAX,
            ] {
                match dumps(api, p, f) {
                    None => rec.line("dumps=NULL"),
                    Some(d) => rec.tag_bytes("dumps", &d),
                }
                rec.tag_u("dumpb", (api.json_dumpb)(p, ptr::null_mut(), 0, f));
                rec.tag_i(
                    "dump_cb",
                    (api.json_dump_callback)(p, None, ptr::null_mut(), f) as i64,
                );
            }
            rec.json("copy", (api.json_copy)(p));
            rec.json("deep_copy", (api.json_deep_copy)(p));
            rec.tag_i("equal_self", (api.json_equal)(p, p) as i64);
            let q = forge_json(api, t, 1);
            rec.tag_i("equal_other", (api.json_equal)(p, q) as i64);
            // containers holding a forged child
            let a = (api.json_array)();
            (api.json_array_append_new)(a, p);
            match dumps(api, a, 0) {
                None => rec.line("arr_dump=NULL"),
                Some(d) => rec.tag_bytes("arr_dump", &d),
            }
            rec.json("arr_deep_copy", (api.json_deep_copy)(a));
            (api.json_array_clear)(a);
            decref(api, a);
            (api.jsonp_free)(p as *mut c_void);
            (api.jsonp_free)(q as *mut c_void);
        }
    });
}

/* ------------------------- rows 99/102..105: equality / number edge ----- */

#[test]
fn err102to105_equal_edge_cases() {
    diff("ERRORS 102-105 json_equal", |api, rec| unsafe {
        let mut rng = Rng::new(0xE102);
        for _ in 0..200 {
            let s1 = rand_spec(&mut rng, 3);
            let s2 = rand_spec(&mut rng, 3);
            let a = build(api, &s1);
            let b = build(api, &s2);
            rec.tag_i("eq", (api.json_equal)(a, b) as i64);
            rec.tag_i("eq_null_a", (api.json_equal)(ptr::null(), b) as i64);
            rec.tag_i("eq_null_b", (api.json_equal)(a, ptr::null()) as i64);
            decref(api, a);
            decref(api, b);
        }
        rec.tag_i("null_null", (api.json_equal)(ptr::null(), ptr::null()) as i64);
    });
}
