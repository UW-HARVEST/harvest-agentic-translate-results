//! Phase C — ERRORS.md rows 1–98 (`value.c`): every rejection branch of the
//! object / array / string / number / equality / copy API.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

/// A `json_t` with an out-of-range `type` field, allocated with the library's
/// own allocator so the library may free it.  `json_delete` takes the `default:`
/// branch for these, i.e. it must NOT free them — so the test frees them itself.
unsafe fn forge(api: &Api, ty: c_int) -> Jt {
    unsafe {
        let p = (api.jsonp_malloc)(64) as *mut JsonT;
        (*p).type_ = ty;
        (*p).refcount = 1;
        p
    }
}

unsafe fn unforge(api: &Api, j: Jt) {
    unsafe { (api.jsonp_free)(j as *mut c_void) }
}

/// Every "not the right type" argument, including NULL and forged out-of-range
/// type tags.
unsafe fn wrong_types(api: &Api) -> Vec<Jt> {
    unsafe {
        vec![
            std::ptr::null_mut(),
            (api.json_object)(),
            (api.json_array)(),
            (api.json_string)(cstr("s").as_ptr()),
            (api.json_integer)(1),
            (api.json_real)(1.0),
            (api.json_true)(),
            (api.json_false)(),
            (api.json_null)(),
            forge(api, 8),
            forge(api, 255),
            forge(api, -1),
            forge(api, i32::MAX),
        ]
    }
}

unsafe fn drop_wrong_types(api: &Api, v: Vec<Jt>) {
    unsafe {
        for (i, j) in v.into_iter().enumerate() {
            if i >= 9 {
                unforge(api, j);
            } else {
                decref(api, j);
            }
        }
    }
}

/* ============ rows 1..41: object rejections ============ */

#[test]
fn e_rows_1_41_object_errors() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let bad = wrong_types(api);
            let key = cstr("k");
            let good = (api.json_object)();
            (api.json_object_set_new)(good, cstr("a").as_ptr(), (api.json_integer)(1));
            (api.json_object_set_new)(good, cstr("b").as_ptr(), (api.json_integer)(2));

            for (i, j) in bad.iter().enumerate() {
                // rows 1,2
                out.push(format!("{i} size={}", (api.json_object_size)(*j)));
                // rows 5,20,21..25,27,30,32,38
                out.push(format!(
                    "{i} getn={} iter={} clear={} update={} upd_ex={} upd_mi={} upd_rec={}",
                    (api.json_object_getn)(*j, key.as_ptr(), 1).is_null(),
                    (api.json_object_iter)(*j).is_null(),
                    (api.json_object_clear)(*j),
                    (api.json_object_update)(*j, good),
                    (api.json_object_update_existing)(*j, good),
                    (api.json_object_update_missing)(*j, good),
                    (api.json_object_update_recursive)(*j, good),
                ));
                out.push(format!(
                    "{i} update-other={} upd_ex-other={} upd_mi-other={} upd_rec-other={}",
                    (api.json_object_update)(good, *j),
                    (api.json_object_update_existing)(good, *j),
                    (api.json_object_update_missing)(good, *j),
                    (api.json_object_update_recursive)(good, *j),
                ));
                out.push(format!(
                    "{i} iter_at={} deln={} setn_nc={}",
                    (api.json_object_iter_at)(*j, key.as_ptr()).is_null(),
                    (api.json_object_deln)(*j, key.as_ptr(), 1),
                    (api.json_object_setn_new_nocheck)(
                        *j,
                        key.as_ptr(),
                        1,
                        (api.json_integer)(5)
                    ),
                ));
                // NULL iterator only: a non-NULL bogus iterator would be
                // dereferenced by the C (real UB), so it is not a valid input.
                out.push(format!(
                    "{i} iter_next(NULL)={} iter_set_new(NULL)={}",
                    (api.json_object_iter_next)(*j, std::ptr::null_mut()).is_null(),
                    (api.json_object_iter_set_new)(*j, std::ptr::null_mut(), (api.json_integer)(1)),
                ));
            }

            // rows 3,4: NULL key
            out.push(format!(
                "get(NULL key)={} getn(NULL key)={}",
                (api.json_object_get)(good, std::ptr::null()).is_null(),
                (api.json_object_getn)(good, std::ptr::null(), 0).is_null()
            ));
            // row 6: absent key
            out.push(format!(
                "absent={}",
                (api.json_object_get)(good, cstr("nope").as_ptr()).is_null()
            ));
            // rows 7,9,13,14: NULL key in setters
            for setter in 0..4 {
                let v = (api.json_integer)(1);
                let r = match setter {
                    0 => (api.json_object_set_new)(good, std::ptr::null(), v),
                    1 => (api.json_object_setn_new)(good, std::ptr::null(), 0, v),
                    2 => (api.json_object_set_new_nocheck)(good, std::ptr::null(), v),
                    _ => (api.json_object_setn_new_nocheck)(good, std::ptr::null(), 0, v),
                };
                out.push(format!("setter{setter}(NULL key)={r}"));
            }
            // row 8: NULL value
            out.push(format!(
                "setn_nc(NULL value)={} set(NULL value)={}",
                (api.json_object_setn_new_nocheck)(good, key.as_ptr(), 1, std::ptr::null_mut()),
                (api.json_object_set_new)(good, key.as_ptr(), std::ptr::null_mut())
            ));
            // row 11: self-insert
            out.push(format!(
                "self-insert={}",
                (api.json_object_setn_new_nocheck)(good, key.as_ptr(), 1, good)
            ));
            // row 15: invalid UTF-8 key
            for bad_key in [&b"\xff"[..], &b"\xc2"[..], &b"\xed\xa0\x80"[..], &b"a\xff"[..]] {
                let z = nul_terminated(bad_key);
                out.push(format!(
                    "utf8key {bad_key:?} setn_new={} set_new={}",
                    (api.json_object_setn_new)(good, z.as_ptr(), bad_key.len(), (api.json_integer)(1)),
                    (api.json_object_set_new)(good, z.as_ptr(), (api.json_integer)(1))
                ));
            }
            // rows 16,17: NULL key in del
            out.push(format!(
                "del(NULL)={} deln(NULL)={}",
                (api.json_object_del)(good, std::ptr::null()),
                (api.json_object_deln)(good, std::ptr::null(), 0)
            ));
            // row 19: absent key in del
            out.push(format!(
                "del(absent)={}",
                (api.json_object_del)(good, cstr("zzz").as_ptr())
            ));
            // row 26: recursive cycle
            let cyc = (api.json_object)();
            (api.json_object_set_new)(cyc, cstr("self").as_ptr(), incref(api, cyc));
            let tgt = (api.json_object)();
            (api.json_object_set_new)(tgt, cstr("self").as_ptr(), (api.json_object)());
            out.push(format!(
                "cycle update_recursive={}",
                (api.json_object_update_recursive)(tgt, cyc)
            ));
            decref(api, tgt);
            // break the cycle so the object can actually be released
            (api.json_object_del)(cyc, cstr("self").as_ptr());
            decref(api, cyc);

            // row 28: iterator on an empty object
            let empty = (api.json_object)();
            out.push(format!("empty iter={}", (api.json_object_iter)(empty).is_null()));
            decref(api, empty);
            // rows 29,31: iter_at NULL key / absent key
            out.push(format!(
                "iter_at(NULL)={} iter_at(absent)={}",
                (api.json_object_iter_at)(good, std::ptr::null()).is_null(),
                (api.json_object_iter_at)(good, cstr("zzz").as_ptr()).is_null()
            ));
            // rows 33,34: iter_next NULL / last
            let it = (api.json_object_iter)(good);
            let mut last = it;
            let mut nxt = (api.json_object_iter_next)(good, it);
            while !nxt.is_null() {
                last = nxt;
                nxt = (api.json_object_iter_next)(good, nxt);
            }
            out.push(format!(
                "iter_next(NULL)={} iter_next(last)={}",
                (api.json_object_iter_next)(good, std::ptr::null_mut()).is_null(),
                (api.json_object_iter_next)(good, last).is_null()
            ));
            // rows 35,36,37,41
            out.push(format!(
                "iter_key(NULL)={} iter_key_len(NULL)={} iter_value(NULL)={} key_to_iter(NULL)={}",
                (api.json_object_iter_key)(std::ptr::null_mut()).is_null(),
                (api.json_object_iter_key_len)(std::ptr::null_mut()),
                (api.json_object_iter_value)(std::ptr::null_mut()).is_null(),
                (api.json_object_key_to_iter)(std::ptr::null()).is_null()
            ));
            // rows 39,40
            out.push(format!(
                "iter_set_new(NULL iter)={} iter_set_new(NULL value)={}",
                (api.json_object_iter_set_new)(good, std::ptr::null_mut(), (api.json_integer)(1)),
                (api.json_object_iter_set_new)(good, it, std::ptr::null_mut())
            ));

            out.push(format!("final good = {:?}", dumps(api, good, JSON_SORT_KEYS)));
            decref(api, good);
            drop_wrong_types(api, bad);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "object error row {i}");
    }
}

/* ============ rows 42..62: array rejections ============ */

#[test]
fn e_rows_42_62_array_errors() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let bad = wrong_types(api);
            let arr = (api.json_array)();
            for i in 0..5 {
                (api.json_array_append_new)(arr, (api.json_integer)(i));
            }
            for (i, j) in bad.iter().enumerate() {
                // rows 42,43,46,50,54,57,59,60
                out.push(format!(
                    "{i} size={} get={} set={} append={} insert={} remove={} clear={} extend={} extend2={}",
                    (api.json_array_size)(*j),
                    (api.json_array_get)(*j, 0).is_null(),
                    (api.json_array_set_new)(*j, 0, (api.json_integer)(1)),
                    (api.json_array_append_new)(*j, (api.json_integer)(1)),
                    (api.json_array_insert_new)(*j, 0, (api.json_integer)(1)),
                    (api.json_array_remove)(*j, 0),
                    (api.json_array_clear)(*j),
                    (api.json_array_extend)(*j, arr),
                    (api.json_array_extend)(arr, *j),
                ));
            }
            // rows 44,48,56,58: index boundaries incl. SIZE_MAX
            for idx in [0usize, 4, 5, 6, 100, usize::MAX, usize::MAX - 1] {
                out.push(format!(
                    "idx={idx} get={} set={} insert={} remove={}",
                    (api.json_array_get)(arr, idx).is_null(),
                    (api.json_array_set_new)(arr, idx, (api.json_integer)(9)),
                    (api.json_array_insert_new)(arr, idx, (api.json_integer)(9)),
                    (api.json_array_remove)(arr, idx),
                ));
                out.push(format!("after idx={idx} {:?}", dumps(api, arr, 0)));
            }
            // rows 45,49,53: NULL value
            out.push(format!(
                "NULLvalue set={} append={} insert={}",
                (api.json_array_set_new)(arr, 0, std::ptr::null_mut()),
                (api.json_array_append_new)(arr, std::ptr::null_mut()),
                (api.json_array_insert_new)(arr, 0, std::ptr::null_mut())
            ));
            // rows 47,51,55: self insertion
            out.push(format!(
                "self set={} append={} insert={}",
                (api.json_array_set_new)(arr, 0, arr),
                (api.json_array_append_new)(arr, arr),
                (api.json_array_insert_new)(arr, 0, arr)
            ));
            out.push(format!("final arr = {:?}", dumps(api, arr, 0)));
            decref(api, arr);
            drop_wrong_types(api, bad);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "array error row {i}");
    }
}

/* ============ rows 63..85: string / number rejections ============ */

#[test]
fn e_rows_63_85_scalar_errors() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            // rows 63,64,66,67,68: NULL value
            out.push(format!(
                "string(NULL)={} stringn(NULL)={} string_nc(NULL)={} stringn_nc(NULL)={} own(NULL)={}",
                (api.json_string)(std::ptr::null()).is_null(),
                (api.json_stringn)(std::ptr::null(), 0).is_null(),
                (api.json_string_nocheck)(std::ptr::null()).is_null(),
                (api.json_stringn_nocheck)(std::ptr::null(), 0).is_null(),
                (api.jsonp_stringn_nocheck_own)(std::ptr::null(), 0).is_null()
            ));
            // row 65: invalid UTF-8
            for badb in [
                &b"\xff"[..],
                &b"\xc0\x80"[..],
                &b"\xc2"[..],
                &b"\xed\xa0\x80"[..],
                &b"\xf5\x80\x80\x80"[..],
                &b"\x80"[..],
                &b"a\xffb"[..],
            ] {
                let z = nul_terminated(badb);
                out.push(format!(
                    "utf8 {badb:?} string={} stringn={}",
                    (api.json_string)(z.as_ptr()).is_null(),
                    (api.json_stringn)(z.as_ptr(), badb.len()).is_null()
                ));
            }
            // rows 69..85 over every wrong type
            let bad = wrong_types(api);
            for (i, j) in bad.iter().enumerate() {
                out.push(format!(
                    "{i} sval={} slen={} ival={} rval={:?} nval={:?} iset={} rset={}",
                    (api.json_string_value)(*j).is_null(),
                    (api.json_string_length)(*j),
                    (api.json_integer_value)(*j),
                    (api.json_real_value)(*j).to_bits(),
                    (api.json_number_value)(*j).to_bits(),
                    (api.json_integer_set)(*j, 5),
                    (api.json_real_set)(*j, 5.0)
                ));
                out.push(format!(
                    "{i} sset={} ssetn={} sset_nc={} ssetn_nc={}",
                    (api.json_string_set)(*j, cstr("x").as_ptr()),
                    (api.json_string_setn)(*j, cstr("x").as_ptr(), 1),
                    (api.json_string_set_nocheck)(*j, cstr("x").as_ptr()),
                    (api.json_string_setn_nocheck)(*j, cstr("x").as_ptr(), 1)
                ));
            }
            // rows 71,72,74,75: NULL value in setters
            let s = (api.json_string)(cstr("orig").as_ptr());
            out.push(format!(
                "sset(NULL)={} ssetn(NULL)={} sset_nc(NULL)={} ssetn_nc(NULL)={}",
                (api.json_string_set)(s, std::ptr::null()),
                (api.json_string_setn)(s, std::ptr::null(), 0),
                (api.json_string_set_nocheck)(s, std::ptr::null()),
                (api.json_string_setn_nocheck)(s, std::ptr::null(), 0)
            ));
            // row 76: invalid UTF-8 in setters
            for badb in [&b"\xff"[..], &b"\xc2"[..], &b"\xed\xa0\x80"[..]] {
                let z = nul_terminated(badb);
                out.push(format!(
                    "sset utf8 {badb:?} set={} setn={}",
                    (api.json_string_set)(s, z.as_ptr()),
                    (api.json_string_setn)(s, z.as_ptr(), badb.len())
                ));
            }
            out.push(format!("s still = {:?}", dumps(api, s, JSON_ENCODE_ANY)));
            decref(api, s);
            // rows 79,80: json_real with NaN / +-Inf
            for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                out.push(format!("real({v:?})={}", (api.json_real)(v).is_null()));
            }
            // rows 83,84: json_real_set with NaN / +-Inf
            let r = (api.json_real)(1.0);
            for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                out.push(format!(
                    "real_set({v:?})={} value={:?}",
                    (api.json_real_set)(r, v),
                    (api.json_real_value)(r).to_bits()
                ));
            }
            decref(api, r);
            drop_wrong_types(api, bad);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "scalar error row {i}");
    }
}

/* ============ rows 86..98: equality / copy / delete / loop check ========= */

#[test]
fn e_rows_86_98_equal_copy_delete() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let bad = wrong_types(api);
            // rows 86,87,88,89
            for (i, x) in bad.iter().enumerate() {
                for (k, y) in bad.iter().enumerate() {
                    out.push(format!("eq {i},{k} = {}", (api.json_equal)(*x, *y)));
                }
            }
            // rows 90,91,92,93,94
            for (i, x) in bad.iter().enumerate() {
                let c = (api.json_copy)(*x);
                let d = (api.json_deep_copy)(*x);
                let mut ht = HashtableT::zeroed();
                assert_eq!((api.hashtable_init)(&mut ht), 0);
                let e = (api.do_deep_copy)(*x, &mut ht);
                out.push(format!(
                    "copy {i}: copy_null={} deep_null={} do_deep_null={}",
                    c.is_null(),
                    d.is_null(),
                    e.is_null()
                ));
                (api.hashtable_close)(&mut ht);
                // forged types return NULL / the same pointer; only free real ones
                if i < 9 {
                    if !c.is_null() && c != *x {
                        decref(api, c);
                    }
                    if !d.is_null() && d != *x {
                        decref(api, d);
                    }
                    if !e.is_null() && e != *x {
                        decref(api, e);
                    }
                }
            }
            // row 96: json_delete(NULL)
            (api.json_delete)(std::ptr::null_mut());
            out.push("delete(NULL) survived".into());
            // row 97: json_delete on an out-of-range type must not free
            let f = forge(api, 200);
            (api.json_delete)(f);
            out.push(format!("delete(forged) type still {}", (*f).type_));
            unforge(api, f);
            // row 95: deep copy of a self-referencing container
            let a = (api.json_array)();
            (api.json_array_append_new)(a, incref(api, a));
            out.push(format!("self-array deep_copy={}", (api.json_deep_copy)(a).is_null()));
            (api.json_array_clear)(a);
            decref(api, a);
            let o = (api.json_object)();
            (api.json_object_set_new)(o, cstr("me").as_ptr(), incref(api, o));
            out.push(format!("self-object deep_copy={}", (api.json_deep_copy)(o).is_null()));
            (api.json_object_clear)(o);
            decref(api, o);
            // row 98: jsonp_loop_check duplicate
            let mut ht = HashtableT::zeroed();
            assert_eq!((api.hashtable_init)(&mut ht), 0);
            let v = (api.json_object)();
            let mut key = [0i8; 32];
            let mut kl: usize = 0;
            out.push(format!(
                "loop_check first={} second={}",
                (api.jsonp_loop_check)(&mut ht, v, key.as_mut_ptr(), 32, &mut kl),
                (api.jsonp_loop_check)(&mut ht, v, key.as_mut_ptr(), 32, &mut kl)
            ));
            (api.hashtable_close)(&mut ht);
            decref(api, v);
            drop_wrong_types(api, bad);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "equal/copy error row {i}");
    }
}

#[allow(unused)]
fn _u(_: *const c_char) {}
