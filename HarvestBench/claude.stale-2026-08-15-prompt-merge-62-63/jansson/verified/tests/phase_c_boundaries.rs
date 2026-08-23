//! Phase C — generic FFI boundary tests that every C API has.
//!
//! Covers the boundaries the task calls out explicitly and that happy-path tests
//! miss: NULL pointers, zero/oversized lengths, values one step past a documented
//! range, and — most importantly — OUT-OF-RANGE ENUM VALUES crossing the FFI
//! boundary. A C `enum` accepts any `int`, so a `json_type` with no valid variant
//! is a real input the C handles (it has explicit `default:` branches) and the
//! Rust must handle identically.
//!
//! ERRORS.md rows covered here: 125, 129, 131, 134, 228 (out-of-range json_type),
//! plus the null/length boundaries of rows 5-12, 63-65, 101-102, 114, 119, 123,
//! 126-127, 130, 133, 206, 238, 306-324, 340, 342, 346, 349-357, 364-365.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};

/// json_type values that are NOT valid variants (the enum has 0..=7).
const BAD_TYPES: &[c_int] = &[8, 9, 42, 127, 255, 256, -1, -128, c_int::MAX, c_int::MIN];

/// Allocate a real json_t (so the refcount/alloc bookkeeping is valid), then
/// forcibly stamp an out-of-range `type` into it, exactly as a hostile or buggy
/// caller could across the FFI boundary.
unsafe fn with_bad_type<T>(
    lib: &Library,
    bad: c_int,
    f: impl Fn(&Library, *mut json_t) -> T,
) -> T {
    let int: Symbol<FnInt> = sym(lib, "json_integer");
    let j = int(1234);
    assert!(!j.is_null());
    let saved = (*j).type_;
    (*j).type_ = bad;
    let out = f(lib, j);
    // restore so the value can be freed through the normal path
    (*j).type_ = saved;
    decref(lib, j);
    out
}

#[test]
fn row228_do_dump_default_on_out_of_range_type() {
    for &bad in BAD_TYPES {
        diff(&format!("row228/do_dump type={}", bad), move |lib: &Library| unsafe {
            with_bad_type(lib, bad, |lib, j| {
                let dumps: Symbol<FnDumps> = sym(lib, "json_dumps");
                let p = dumps(j, JSON_ENCODE_ANY);
                let s = if p.is_null() { None } else { Some(cstr_to_string(p)) };
                if !p.is_null() {
                    libc_free(p as *mut c_void);
                }
                // json_dumpb and json_dump_callback must agree too
                let dumpb: Symbol<FnDumpb> = sym(lib, "json_dumpb");
                let mut buf = [0u8; 64];
                let n = dumpb(j, buf.as_mut_ptr() as *mut c_char, 64, JSON_ENCODE_ANY);
                (s, n)
            })
        });
    }
}

#[test]
fn row129_json_equal_default_on_out_of_range_type() {
    for &bad in BAD_TYPES {
        diff(&format!("row129/json_equal type={}", bad), move |lib: &Library| unsafe {
            with_bad_type(lib, bad, |lib, j| {
                let eq: Symbol<FnEqual> = sym(lib, "json_equal");
                let int: Symbol<FnInt> = sym(lib, "json_integer");
                let other = int(1234);
                let other_bad = int(1234);
                (*other_bad).type_ = bad;
                // same bad type on both sides, and bad vs valid
                let a = eq(j, other_bad);
                let b = eq(j, other);
                let c = eq(other, j);
                (*other_bad).type_ = JSON_INTEGER;
                decref(lib, other);
                decref(lib, other_bad);
                (a, b, c)
            })
        });
    }
}

#[test]
fn rows131_134_copy_and_deep_copy_default_on_out_of_range_type() {
    for &bad in BAD_TYPES {
        diff(&format!("rows131/134 copy type={}", bad), move |lib: &Library| unsafe {
            with_bad_type(lib, bad, |lib, j| {
                let copy: Symbol<FnCopy> = sym(lib, "json_copy");
                let deep: Symbol<FnDeepCopy> = sym(lib, "json_deep_copy");
                let c1 = copy(j);
                let c2 = deep(j);
                // Both must be NULL (default: return NULL). If not, report the type.
                let r = (c1.is_null(), c2.is_null());
                if !c1.is_null() {
                    (*c1).type_ = JSON_INTEGER;
                    decref(lib, c1);
                }
                if !c2.is_null() {
                    (*c2).type_ = JSON_INTEGER;
                    decref(lib, c2);
                }
                r
            })
        });
    }
}

#[test]
fn row125_json_delete_default_on_out_of_range_type() {
    // json_delete's `default:` returns WITHOUT freeing. We cannot observe the
    // non-free directly, but we can observe that it does not crash and that the
    // object is still readable afterwards (type field intact).
    for &bad in BAD_TYPES {
        diff(&format!("row125/json_delete type={}", bad), move |lib: &Library| unsafe {
            let int: Symbol<FnInt> = sym(lib, "json_integer");
            let del: Symbol<FnDelete> = sym(lib, "json_delete");
            let j = int(7);
            (*j).type_ = bad;
            del(j); // must be a silent no-op for out-of-range types
            let still_there = (*j).type_;
            (*j).type_ = JSON_INTEGER;
            decref(lib, j);
            still_there
        });
    }
}

#[test]
fn typed_accessors_on_out_of_range_and_mismatched_types() {
    // Every typed getter must reject a wrong/out-of-range type identically.
    for &bad in BAD_TYPES {
        diff(&format!("accessors type={}", bad), move |lib: &Library| unsafe {
            with_bad_type(lib, bad, |lib, j| {
                let iv: Symbol<FnIntVal> = sym(lib, "json_integer_value");
                let rv: Symbol<FnRealVal> = sym(lib, "json_real_value");
                let nv: Symbol<FnRealVal> = sym(lib, "json_number_value");
                let sv: Symbol<FnStrVal> = sym(lib, "json_string_value");
                let sl: Symbol<FnSize> = sym(lib, "json_string_length");
                let osz: Symbol<FnSize> = sym(lib, "json_object_size");
                let asz: Symbol<FnSize> = sym(lib, "json_array_size");
                let iset: Symbol<unsafe extern "C" fn(*mut json_t, json_int_t) -> c_int> =
                    sym(lib, "json_integer_set");
                let rset: Symbol<unsafe extern "C" fn(*mut json_t, f64) -> c_int> =
                    sym(lib, "json_real_set");
                (
                    iv(j),
                    rv(j).to_bits(),
                    nv(j).to_bits(),
                    sv(j).is_null(),
                    sl(j),
                    osz(j),
                    asz(j),
                    iset(j, 5),
                    rset(j, 1.5),
                )
            })
        });
    }
}

#[test]
fn null_pointer_boundaries_everywhere() {
    // Every function that documents a NULL check must return the same sentinel.
    diff("null-pointer surface", |lib: &Library| unsafe {
        let n: *mut json_t = std::ptr::null_mut();
        let nk: *const c_char = std::ptr::null();

        let osz: Symbol<FnSize> = sym(lib, "json_object_size");
        let asz: Symbol<FnSize> = sym(lib, "json_array_size");
        let oget: Symbol<FnObjGet> = sym(lib, "json_object_get");
        let ogetn: Symbol<FnObjGetN> = sym(lib, "json_object_getn");
        let odel: Symbol<FnObjDel> = sym(lib, "json_object_del");
        let odeln: Symbol<FnObjDelN> = sym(lib, "json_object_deln");
        let oclear: Symbol<unsafe extern "C" fn(*mut json_t) -> c_int> =
            sym(lib, "json_object_clear");
        let aget: Symbol<FnArrGet> = sym(lib, "json_array_get");
        let aclear: Symbol<unsafe extern "C" fn(*mut json_t) -> c_int> =
            sym(lib, "json_array_clear");
        let aremove: Symbol<FnArrRemove> = sym(lib, "json_array_remove");
        let sval: Symbol<FnStrVal> = sym(lib, "json_string_value");
        let slen: Symbol<FnSize> = sym(lib, "json_string_length");
        let ival: Symbol<FnIntVal> = sym(lib, "json_integer_value");
        let rval: Symbol<FnRealVal> = sym(lib, "json_real_value");
        let nval: Symbol<FnRealVal> = sym(lib, "json_number_value");
        let eq: Symbol<FnEqual> = sym(lib, "json_equal");
        let copy: Symbol<FnCopy> = sym(lib, "json_copy");
        let deep: Symbol<FnDeepCopy> = sym(lib, "json_deep_copy");
        let del: Symbol<FnDelete> = sym(lib, "json_delete");
        let jstr: Symbol<FnStr> = sym(lib, "json_string");
        let jstrn: Symbol<FnStrN> = sym(lib, "json_stringn");
        let jstrnc: Symbol<FnStr> = sym(lib, "json_string_nocheck");
        let jstrnnc: Symbol<FnStrN> = sym(lib, "json_stringn_nocheck");
        let iter: Symbol<FnIter> = sym(lib, "json_object_iter");
        let iterat: Symbol<unsafe extern "C" fn(*mut json_t, *const c_char) -> *mut c_void> =
            sym(lib, "json_object_iter_at");
        let iternext: Symbol<FnIterNext> = sym(lib, "json_object_iter_next");
        let iterkey: Symbol<FnIterKey> = sym(lib, "json_object_iter_key");
        let iterkeylen: Symbol<FnIterKeyLen> = sym(lib, "json_object_iter_key_len");
        let itervalue: Symbol<FnIterValue> = sym(lib, "json_object_iter_value");
        let k2i: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            sym(lib, "json_object_key_to_iter");
        let dumps: Symbol<FnDumps> = sym(lib, "json_dumps");

        // json_delete(NULL) must be a silent no-op.
        del(n);

        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let o = obj();

        let out: Vec<String> = vec![
            format!("object_size(NULL)={}", osz(n)),
            format!("array_size(NULL)={}", asz(n)),
            format!("object_get(NULL,k)={}", oget(n, cs("k").as_ptr()).is_null()),
            format!("object_get(o,NULL)={}", oget(o, nk).is_null()),
            format!("object_getn(NULL,NULL,0)={}", ogetn(n, nk, 0).is_null()),
            format!("object_del(NULL,k)={}", odel(n, cs("k").as_ptr())),
            format!("object_del(o,NULL)={}", odel(o, nk)),
            format!("object_deln(o,NULL,0)={}", odeln(o, nk, 0)),
            format!("object_clear(NULL)={}", oclear(n)),
            format!("array_get(NULL,0)={}", aget(n, 0).is_null()),
            format!("array_clear(NULL)={}", aclear(n)),
            format!("array_remove(NULL,0)={}", aremove(n, 0)),
            format!("string_value(NULL)={}", sval(n).is_null()),
            format!("string_length(NULL)={}", slen(n)),
            format!("integer_value(NULL)={}", ival(n)),
            format!("real_value(NULL)={:#x}", rval(n).to_bits()),
            format!("number_value(NULL)={:#x}", nval(n).to_bits()),
            format!("equal(NULL,NULL)={}", eq(n, n)),
            format!("equal(NULL,o)={}", eq(n, o as *const json_t)),
            format!("equal(o,NULL)={}", eq(o as *const json_t, n)),
            format!("copy(NULL)={}", copy(n).is_null()),
            format!("deep_copy(NULL)={}", deep(n).is_null()),
            format!("string(NULL)={}", jstr(nk).is_null()),
            format!("stringn(NULL,0)={}", jstrn(nk, 0).is_null()),
            format!("string_nocheck(NULL)={}", jstrnc(nk).is_null()),
            format!("stringn_nocheck(NULL,5)={}", jstrnnc(nk, 5).is_null()),
            format!("object_iter(NULL)={}", iter(n).is_null()),
            format!("iter_at(NULL,NULL)={}", iterat(n, nk).is_null()),
            format!("iter_at(o,NULL)={}", iterat(o, nk).is_null()),
            format!("iter_next(NULL,NULL)={}", iternext(n, std::ptr::null_mut()).is_null()),
            format!("iter_next(o,NULL)={}", iternext(o, std::ptr::null_mut()).is_null()),
            format!("iter_key(NULL)={}", iterkey(std::ptr::null_mut()).is_null()),
            format!("iter_key_len(NULL)={}", iterkeylen(std::ptr::null_mut())),
            format!("iter_value(NULL)={}", itervalue(std::ptr::null_mut()).is_null()),
            format!("key_to_iter(NULL)={}", k2i(nk).is_null()),
            format!("dumps(NULL)={}", dumps(n, JSON_ENCODE_ANY).is_null()),
        ];
        decref(lib, o);
        out
    });
}

#[test]
fn null_and_empty_arguments_to_mutators() {
    diff("null args to mutators", |lib: &Library| unsafe {
        let n: *mut json_t = std::ptr::null_mut();
        let nk: *const c_char = std::ptr::null();
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let oset: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
        let osetn: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
        let osetnc: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new_nocheck");
        let osetnnc: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new_nocheck");
        let aset: Symbol<FnArrSetNew> = sym(lib, "json_array_set_new");
        let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        let ains: Symbol<FnArrSetNew> = sym(lib, "json_array_insert_new");
        let aext: Symbol<FnTwoJson> = sym(lib, "json_array_extend");
        let oupd: Symbol<FnTwoJson> = sym(lib, "json_object_update");
        let oupde: Symbol<FnTwoJson> = sym(lib, "json_object_update_existing");
        let oupdm: Symbol<FnTwoJson> = sym(lib, "json_object_update_missing");
        let oupdr: Symbol<FnTwoJson> = sym(lib, "json_object_update_recursive");
        let sset: Symbol<unsafe extern "C" fn(*mut json_t, *const c_char) -> c_int> =
            sym(lib, "json_string_set");
        let ssetn: Symbol<unsafe extern "C" fn(*mut json_t, *const c_char, usize) -> c_int> =
            sym(lib, "json_string_setn");
        let itersetnew: Symbol<
            unsafe extern "C" fn(*mut json_t, *mut c_void, *mut json_t) -> c_int,
        > = sym(lib, "json_object_iter_set_new");

        let o = obj();
        let a = arr();
        let s: Symbol<FnStr> = sym(lib, "json_string");
        let strv = s(cs("v").as_ptr());

        let out: Vec<(&str, c_int)> = vec![
            // key == NULL / json == NULL / value == NULL
            ("object_set_new(o,NULL,v)", oset(o, nk, int(1))),
            ("object_setn_new(o,NULL,0,v)", osetn(o, nk, 0, int(1))),
            ("object_set_new_nocheck(o,NULL,v)", osetnc(o, nk, int(1))),
            ("object_setn_new_nocheck(o,NULL,0,v)", osetnnc(o, nk, 0, int(1))),
            ("object_setn_new_nocheck(o,k,1,NULL)", osetnnc(o, cs("k").as_ptr(), 1, n)),
            ("object_setn_new_nocheck(NULL,k,1,v)", osetnnc(n, cs("k").as_ptr(), 1, int(1))),
            // NOTE: self-insertion is deliberately NOT tested here. When
            // `json == value` the C does `json_decref(value)` before returning -1
            // (value.c:131/484/537/560), i.e. it CONSUMES the caller's reference —
            // so a self-insert on a refcount-1 container destroys it and every
            // later use in this closure would be a use-after-free. It gets its own
            // test below, which increfs first to account for the consumed ref.
            ("array_set_new(a,0,NULL)", aset(a, 0, n)),
            ("array_append_new(a,NULL)", aapp(a, n)),
            ("array_append_new(NULL,v)", aapp(n, int(1))),
            ("array_insert_new(a,0,NULL)", ains(a, 0, n)),
            ("array_extend(a,NULL)", aext(a, n)),
            ("array_extend(NULL,a)", aext(n, a)),
            ("array_extend(a,obj)", aext(a, o)),
            ("object_update(o,NULL)", oupd(o, n)),
            ("object_update(NULL,o)", oupd(n, o)),
            ("object_update(o,arr)", oupd(o, a)),
            ("object_update_existing(o,NULL)", oupde(o, n)),
            ("object_update_missing(o,NULL)", oupdm(o, n)),
            ("object_update_recursive(o,NULL)", oupdr(o, n)),
            ("object_update_recursive(NULL,o)", oupdr(n, o)),
            ("string_set(s,NULL)", sset(strv, nk)),
            ("string_setn(s,NULL,0)", ssetn(strv, nk, 0)),
            ("string_set(NULL,x)", sset(n, cs("x").as_ptr())),
            ("string_setn(obj,x,1)", ssetn(o, cs("x").as_ptr(), 1)),
            ("iter_set_new(NULL,NULL,v)", itersetnew(n, std::ptr::null_mut(), int(1))),
            ("iter_set_new(o,NULL,v)", itersetnew(o, std::ptr::null_mut(), int(1))),
        ];
        decref(lib, o);
        decref(lib, a);
        decref(lib, strv);
        out
    });
}

#[test]
fn self_insertion_rejected_and_consumes_the_reference() {
    // ERRORS.md rows 17, 68, 73, 77: `json == value` is rejected with -1.
    //
    // The subtle part, and the reason this needs its own test: the C decrefs
    // `value` on that path (`value.c:131/484/537/560`), so the rejection also
    // CONSUMES the caller's reference. We incref first so the container survives
    // the call and we can still observe its refcount and free it exactly once.
    diff("rows17/68/73/77 self-insert", |lib: &Library| unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let oset: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
        let osetn: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
        let osetnc: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new_nocheck");
        let osetnnc: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new_nocheck");
        let aset: Symbol<FnArrSetNew> = sym(lib, "json_array_set_new");
        let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        let ains: Symbol<FnArrSetNew> = sym(lib, "json_array_insert_new");

        let mut out: Vec<String> = Vec::new();

        // --- object self-insert, one fresh object per variant
        for (name, which) in [("set_new", 0), ("setn_new", 1), ("set_new_nocheck", 2), ("setn_new_nocheck", 3)] {
            let o = obj();
            let before = (*o).refcount;
            incref(o); // account for the reference the call will consume
            let k = cs("self");
            let rc = match which {
                0 => oset(o, k.as_ptr(), o),
                1 => osetn(o, k.as_ptr(), 4, o),
                2 => osetnc(o, k.as_ptr(), o),
                _ => osetnnc(o, k.as_ptr(), 4, o),
            };
            let after = (*o).refcount;
            let sz: Symbol<FnSize> = sym(lib, "json_object_size");
            out.push(format!(
                "object_{}: rc={} refcount {}->{} size={} dump={:?}",
                name, rc, before, after, sz(o), dumps_to_string(lib, o, 0)
            ));
            decref(lib, o);
        }

        // --- array self-insert, one fresh array per variant
        for (name, which) in [("append_new", 0), ("insert_new", 1), ("set_new", 2)] {
            let a = arr();
            let int: Symbol<FnInt> = sym(lib, "json_integer");
            if which == 2 {
                aapp(a, int(1)); // set_new needs a valid index to reach the self-check
            }
            let before = (*a).refcount;
            incref(a);
            let rc = match which {
                0 => aapp(a, a),
                1 => ains(a, 0, a),
                _ => aset(a, 0, a),
            };
            let after = (*a).refcount;
            let sz: Symbol<FnSize> = sym(lib, "json_array_size");
            out.push(format!(
                "array_{}: rc={} refcount {}->{} size={} dump={:?}",
                name, rc, before, after, sz(a), dumps_to_string(lib, a, 0)
            ));
            decref(lib, a);
        }
        out
    });

    // Pin the C's actual behavior so this cannot pass vacuously: rejected with
    // -1, and the extra reference we added was consumed (refcount back to 1).
    let l = libs();
    unsafe {
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(&l.c, "json_array");
        let aapp: Symbol<FnArrAppendNew> = sym(&l.c, "json_array_append_new");
        let a = arr();
        incref(a);
        assert_eq!((*a).refcount, 2);
        assert_eq!(aapp(a, a), -1, "self-append must be rejected");
        assert_eq!((*a).refcount, 1, "self-append must consume the reference");
        decref(&l.c, a);
    }
}

#[test]
fn index_one_past_valid_range() {
    // rows 65, 69, 78, 81: index == entries and index > entries.
    diff("index boundaries", |lib: &Library| unsafe {
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        let aget: Symbol<FnArrGet> = sym(lib, "json_array_get");
        let aset: Symbol<FnArrSetNew> = sym(lib, "json_array_set_new");
        let ains: Symbol<FnArrSetNew> = sym(lib, "json_array_insert_new");
        let arem: Symbol<FnArrRemove> = sym(lib, "json_array_remove");

        let a = arr();
        for i in 0..3 {
            aapp(a, int(i));
        }
        // entries == 3
        let out: Vec<String> = vec![
            format!("get(2)={}", aget(a, 2).is_null()),
            format!("get(3)={}", aget(a, 3).is_null()),
            format!("get(4)={}", aget(a, 4).is_null()),
            format!("get(MAX)={}", aget(a, usize::MAX).is_null()),
            // index == entries -> rejected (set is NOT append)
            format!("set(3)={}", aset(a, 3, int(9))),
            format!("set(MAX)={}", aset(a, usize::MAX, int(9))),
            // index == entries -> ALLOWED for insert
            format!("insert(3)={}", ains(a, 3, int(9))),
            format!("insert(5)={}", ains(a, 5, int(9))),
            format!("insert(MAX)={}", ains(a, usize::MAX, int(9))),
            format!("remove(3)={}", arem(a, 3)),
            format!("remove(99)={}", arem(a, 99)),
            format!("remove(MAX)={}", arem(a, usize::MAX)),
            format!("dump={:?}", dumps_to_string(lib, a, 0)),
        ];
        decref(lib, a);
        out
    });
}

#[test]
fn zero_and_oversized_lengths() {
    // rows 93, 100, 109, 313, 327, 340, 346: len 0 and len == (size_t)-1.
    diff("length boundaries", |lib: &Library| unsafe {
        let jstrn: Symbol<FnStrN> = sym(lib, "json_stringn");
        let jstrnnc: Symbol<FnStrN> = sym(lib, "json_stringn_nocheck");
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let osetn: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let ogetn: Symbol<FnObjGetN> = sym(lib, "json_object_getn");
        let odeln: Symbol<FnObjDelN> = sym(lib, "json_object_deln");
        let slen: Symbol<FnSize> = sym(lib, "json_string_length");

        let data = b"abcdef\0ghi";
        let mut out: Vec<String> = Vec::new();

        // len 0 -> empty string (valid)
        let s0 = jstrn(data.as_ptr() as *const c_char, 0);
        out.push(format!("len0 null={} len={}", s0.is_null(), if s0.is_null() { 0 } else { slen(s0) }));
        if !s0.is_null() {
            out.push(format!("{:?}", dumps_to_string(lib, s0, JSON_ENCODE_ANY)));
            decref(lib, s0);
        }
        // NOTE: json_stringn(_, (size_t)-1) is NOT testable — json_stringn calls
        // utf8_check_string(value, len) FIRST, which scans `len` bytes and so reads
        // far out of bounds (UB in the C itself; it segfaults). Only the *_nocheck
        // variant reaches the documented `jsonp_strndup -> malloc(0) -> NULL` path,
        // because it skips the UTF-8 scan. Same reasoning applies to
        // json_object_setn_new / getn / deln with an oversized key_len, which hash
        // the key over `key_len` bytes. Those are ERRORS.md rows marked UB.
        let smn = jstrnnc(data.as_ptr() as *const c_char, usize::MAX);
        out.push(format!("lenMAX_nocheck null={}", smn.is_null()));
        if !smn.is_null() {
            decref(lib, smn);
        }
        // len spanning the embedded NUL
        let s9 = jstrn(data.as_ptr() as *const c_char, 10);
        out.push(format!("len10 null={} len={}", s9.is_null(), if s9.is_null() { 0 } else { slen(s9) }));
        if !s9.is_null() {
            out.push(format!("{:?}", dumps_to_string(lib, s9, JSON_ENCODE_ANY)));
            decref(lib, s9);
        }

        // object keys at the in-bounds length boundaries (0, past an embedded NUL,
        // and exactly the buffer length). Oversized key_len is UB (see note above).
        let o = obj();
        for len in [0usize, 1, 6, 7, 10] {
            out.push(format!(
                "setn len{} = {}",
                len,
                osetn(o, data.as_ptr() as *const c_char, len, int(len as i64))
            ));
        }
        for len in [0usize, 1, 6, 7, 10, 11] {
            out.push(format!(
                "getn len{} null={}",
                len,
                ogetn(o, data.as_ptr() as *const c_char, len).is_null()
            ));
        }
        out.push(format!("deln len0 = {}", odeln(o, data.as_ptr() as *const c_char, 0)));
        out.push(format!("deln len7 = {}", odeln(o, data.as_ptr() as *const c_char, 7)));
        out.push(format!("size={}", {
            let sz: Symbol<FnSize> = sym(lib, "json_object_size");
            sz(o)
        }));
        out.push(format!("{:?}", dumps_to_string(lib, o, JSON_SORT_KEYS)));
        decref(lib, o);
        out
    });
}

#[test]
fn utf8_helper_boundaries() {
    // rows 306-324: the exported UTF-8 helpers at every documented boundary.
    type FnEncode = unsafe extern "C" fn(c_int, *mut c_char, *mut usize) -> c_int;
    type FnCheckFirst = unsafe extern "C" fn(c_char) -> usize;
    type FnCheckFull = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize;
    type FnIterate = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char;
    type FnCheckString = unsafe extern "C" fn(*const c_char, usize) -> c_int;

    diff("utf8_encode boundaries", |lib: &Library| unsafe {
        let f: Symbol<FnEncode> = sym(lib, "utf8_encode");
        let mut out = Vec::new();
        for cp in [
            -1i32,
            -2,
            i32::MIN,
            0,
            1,
            0x7F,
            0x80,
            0x7FF,
            0x800,
            0xD7FF,
            0xD800,
            0xDBFF,
            0xDC00,
            0xDFFF,
            0xE000,
            0xFFFF,
            0x10000,
            0x10FFFE,
            0x10FFFF,
            0x110000,
            0x1FFFFF,
            i32::MAX,
        ] {
            let mut buf = [0u8; 8];
            let mut size: usize = 0;
            let rc = f(cp, buf.as_mut_ptr() as *mut c_char, &mut size);
            out.push((cp, rc, size, buf));
        }
        out
    });

    diff("utf8_check_first all 256 bytes", |lib: &Library| unsafe {
        let f: Symbol<FnCheckFirst> = sym(lib, "utf8_check_first");
        (0..=255u8).map(|b| f(b as c_char)).collect::<Vec<usize>>()
    });

    diff("utf8_check_full boundaries", |lib: &Library| unsafe {
        let f: Symbol<FnCheckFull> = sym(lib, "utf8_check_full");
        let cases: &[&[u8]] = &[
            b"",
            b"a",
            b"\x80",
            b"\xC2\xA9",
            b"\xC0\x80",       // overlong 2-byte
            b"\xC1\xBF",       // overlong 2-byte
            b"\xE0\x80\xA9",   // overlong 3-byte
            b"\xE2\x82\xAC",   // valid 3-byte
            b"\xED\xA0\x80",   // surrogate
            b"\xED\xBF\xBF",   // surrogate
            b"\xEE\x80\x80",   // valid just past surrogates
            b"\xF0\x80\x80\x80", // overlong 4-byte
            b"\xF0\x9D\x84\x9E", // valid 4-byte
            b"\xF4\x8F\xBF\xBF", // U+10FFFF
            b"\xF4\xBF\xBF\xBF", // > U+10FFFF
            b"\xE2\x82",         // truncated
            b"\xC2",             // truncated
            b"\xE2\x82\xAC\xAC", // size 4 with a 3-byte seq
            b"\xF0\x9D\x84",     // truncated 4-byte
        ];
        let mut out = Vec::new();
        for c in cases {
            // Zero-pad to 8 bytes so passing a `size` larger than the sequence is
            // still an in-bounds read (utf8_check_full reads `size` bytes for
            // size 2/3/4). Without this the test itself would be UB.
            let mut pad = [0u8; 8];
            pad[..c.len()].copy_from_slice(c);
            for size in 0..=5usize {
                let mut cp: i32 = -12345;
                let rc = f(pad.as_ptr() as *const c_char, size, &mut cp);
                // also exercise the NULL codepoint out-param
                let rc2 = f(pad.as_ptr() as *const c_char, size, std::ptr::null_mut());
                out.push((c.to_vec(), size, rc, cp, rc2));
            }
        }
        out
    });

    diff("utf8_iterate boundaries", |lib: &Library| unsafe {
        let f: Symbol<FnIterate> = sym(lib, "utf8_iterate");
        let cases: &[&[u8]] = &[
            b"", b"a", b"\x80", b"\xC2\xA9", b"\xE2\x82\xAC", b"\xE2\x82", b"\xF0\x9D\x84\x9E",
            b"\xF0\x9D", b"\xED\xA0\x80", b"\xFF",
        ];
        let mut out = Vec::new();
        for c in cases {
            let mut pad = [0u8; 8];
            pad[..c.len()].copy_from_slice(c);
            for bufsize in 0..=5usize {
                let mut cp: i32 = -999;
                let p = f(pad.as_ptr() as *const c_char, bufsize, &mut cp);
                // Report the advance as an OFFSET, never a raw address: the two
                // libraries live at different base addresses.
                let advanced = if p.is_null() {
                    -1isize
                } else {
                    p as isize - pad.as_ptr() as isize
                };
                out.push((c.to_vec(), bufsize, advanced, cp));
            }
        }
        out
    });

    diff("utf8_check_string boundaries", |lib: &Library| unsafe {
        let f: Symbol<FnCheckString> = sym(lib, "utf8_check_string");
        let cases: &[&[u8]] = &[
            b"",
            b"abc",
            b"a\x00b",
            b"\xC2\xA9",
            b"\xE2\x82\xAC",
            b"\xE2\x82",
            b"\xF0\x9D\x84\x9E",
            b"\xED\xA0\x80",
            b"\xC0\x80",
            b"\xFF\xFE",
        ];
        let mut out = Vec::new();
        for c in cases {
            for len in 0..=c.len() {
                out.push((c.to_vec(), len, f(c.as_ptr() as *const c_char, len)));
            }
        }
        out
    });
}

#[test]
fn utf8_helpers_randomized() {
    // Property-style sweep: random 1-6 byte strings biased toward high bytes,
    // which is where the lead/continuation/overlong/surrogate logic lives.
    type FnCheckFull = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize;
    type FnIterate = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char;
    type FnCheckString = unsafe extern "C" fn(*const c_char, usize) -> c_int;

    diff_n("utf8 randomized", 3000, |lib: &Library, i| unsafe {
        let mut rng = Rng::new(0x0F8_0000 ^ i);
        let n = 1 + rng.below(6) as usize;
        let bytes: Vec<u8> = (0..n)
            .map(|_| {
                // 60% chance of a high byte so multi-byte paths dominate
                if rng.below(10) < 6 {
                    0x80u8.wrapping_add(rng.below(0x80) as u8)
                } else {
                    rng.below(0x80) as u8
                }
            })
            .collect();
        let full: Symbol<FnCheckFull> = sym(lib, "utf8_check_full");
        let it: Symbol<FnIterate> = sym(lib, "utf8_iterate");
        let cs_: Symbol<FnCheckString> = sym(lib, "utf8_check_string");
        let mut cp1: i32 = -1;
        let mut cp2: i32 = -1;
        let r1 = full(bytes.as_ptr() as *const c_char, bytes.len(), &mut cp1);
        let p = it(bytes.as_ptr() as *const c_char, bytes.len(), &mut cp2);
        let adv = if p.is_null() { -1isize } else { p as isize - bytes.as_ptr() as isize };
        let r3 = cs_(bytes.as_ptr() as *const c_char, bytes.len());
        (bytes, r1, cp1, adv, cp2, r3)
    });
}

#[test]
fn memory_helper_boundaries() {
    // rows 340, 342, 346: jsonp_malloc(0) -> NULL, jsonp_free(NULL) no-op,
    // jsonp_strndup with len 0 / (size_t)-1.
    type FnMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
    type FnFree = unsafe extern "C" fn(*mut c_void);
    type FnStrndup = unsafe extern "C" fn(*const c_char, usize) -> *mut c_char;

    diff("memory helpers", |lib: &Library| unsafe {
        let m: Symbol<FnMalloc> = sym(lib, "jsonp_malloc");
        let fr: Symbol<FnFree> = sym(lib, "jsonp_free");
        let sd: Symbol<FnStrndup> = sym(lib, "jsonp_strndup");

        let z = m(0);
        let z_null = z.is_null();
        if !z.is_null() {
            fr(z);
        }
        let p = m(16);
        let p_null = p.is_null();
        fr(p);
        fr(std::ptr::null_mut()); // must be a no-op

        let src = b"hello\0world\0";
        let mut res = Vec::new();
        for len in [0usize, 1, 5, 11, usize::MAX] {
            let d = sd(src.as_ptr() as *const c_char, len);
            if d.is_null() {
                res.push((len, true, vec![]));
            } else {
                let bytes = std::slice::from_raw_parts(d as *const u8, len.min(12)).to_vec();
                res.push((len, false, bytes));
                fr(d as *mut c_void);
            }
        }
        (z_null, p_null, res)
    });
}

#[test]
fn version_boundaries() {
    // rows 367-369.
    diff("version", |lib: &Library| unsafe {
        let vs: Symbol<unsafe extern "C" fn() -> *const c_char> = sym(lib, "jansson_version_str");
        let vc: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            sym(lib, "jansson_version_cmp");
        let s = cstr_to_string(vs());
        let mut cmps = Vec::new();
        for (ma, mi, mc) in [
            (2, 15, 0),
            (2, 15, 1),
            (2, 14, 0),
            (2, 16, 0),
            (1, 0, 0),
            (3, 0, 0),
            (2, 15, -1),
            (0, 0, 0),
            (-1, -1, -1),
            (c_int::MAX, 0, 0),
            (c_int::MIN, 0, 0),
        ] {
            cmps.push(((ma, mi, mc), vc(ma, mi, mc)));
        }
        (s, cmps)
    });
}

#[test]
fn error_struct_boundaries() {
    // rows 349-357: source/text truncation and the "first error wins" rule,
    // observed through json_load_file with a very long path and through two
    // successive failures reusing one json_error_t.
    diff("error struct boundaries", |lib: &Library| unsafe {
        let lf: Symbol<unsafe extern "C" fn(*const c_char, usize, *mut json_error_t) -> *mut json_t> =
            sym(lib, "json_load_file");
        let ls: Symbol<FnLoads> = sym(lib, "json_loads");

        // A path >= JSON_ERROR_SOURCE_LENGTH (80) must be truncated to "..."+tail.
        let long_path = format!("/nonexistent/{}/f.json", "d".repeat(200));
        let lp = cs(&long_path);
        let mut e1 = json_error_t::new();
        let r1 = lf(lp.as_ptr(), 0, &mut e1);

        // Reusing an already-populated json_error_t: the FIRST error must win.
        let mut e2 = json_error_t::new();
        let r2 = ls(cs("[").as_ptr(), 0, &mut e2);
        let first = e2.snapshot();
        let r3 = ls(cs("{,}").as_ptr(), 0, &mut e2);
        let after = e2.snapshot();

        // error == NULL must be accepted everywhere.
        let r4 = ls(cs("[").as_ptr(), 0, std::ptr::null_mut());
        let r5 = lf(lp.as_ptr(), 0, std::ptr::null_mut());

        (
            r1.is_null(),
            e1.snapshot(),
            r2.is_null(),
            first,
            r3.is_null(),
            after,
            r4.is_null(),
            r5.is_null(),
        )
    });
}
