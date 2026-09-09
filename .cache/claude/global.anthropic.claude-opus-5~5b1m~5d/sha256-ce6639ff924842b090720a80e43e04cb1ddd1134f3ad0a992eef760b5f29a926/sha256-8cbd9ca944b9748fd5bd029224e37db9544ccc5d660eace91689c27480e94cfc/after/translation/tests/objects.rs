//! Differential tests for objects / arrays / properties / iterators /
//! registry / globals — CONFIGS.md rows 29-45 and 51-52.

#![allow(dead_code, non_snake_case, unused_unsafe)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::{null, null_mut};

extern "C" {
    fn fflush(f: *mut c_void) -> c_int;
}

unsafe fn flush() {
    fflush(null_mut());
}


/* ---------------------------------------------------------------- js_type */

const T_UNDEF: c_int = 0;
const T_NULL: c_int = 1;
const T_BOOL: c_int = 2;
const T_NUM: c_int = 3;
const T_STR: c_int = 4;
const T_FUN: c_int = 5;
const T_OBJ: c_int = 6;

/* ------------------------------------------------- protected call support */

static mut CUR_API: *const Api = null();
static mut BODY: Option<&'static (dyn Fn(&Api, JS) + 'static)> = None;

unsafe extern "C" fn body_tramp(J: JS) {
    let api = &*CUR_API;
    if let Some(f) = BODY {
        f(api, J);
    }
}

/// Run `f` as the body of a C function invoked through `js_pcall`, so that any
/// exception it raises is caught instead of aborting the process.
unsafe fn pcall<F: Fn(&Api, JS) + 'static>(api: &Api, J: JS, tag: &str, f: F) {
    CUR_API = api as *const Api;
    BODY = Some(Box::leak(Box::new(f)));
    let name = cs("body");
    let top0 = (api.js_gettop)(J);
    (api.js_newcfunction)(J, Some(body_tramp), name.as_ptr(), 0);
    (api.js_pushundefined)(J);
    let rc = (api.js_pcall)(J, 0);
    p_int(&format!("{}.rc", tag), rc);
    let t = (api.js_type)(J, -1);
    p_int(&format!("{}.res_type", tag), t);
    if rc != 0 {
        /* copy so that js_tostring's in-place conversion hits the copy */
        (api.js_copy)(J, -1);
        p_str(&format!("{}.err", tag), (api.js_tostring)(J, -1));
        (api.js_pop)(J, 1);
    }
    (api.js_pop)(J, 1);
    p_int(&format!("{}.topdelta", tag), (api.js_gettop)(J) - top0);
    BODY = None;
}

/* ------------------------------------------------------------- dump utils */

unsafe fn dumpv(api: &Api, J: JS, idx: c_int, label: &str) {
    p_str(&format!("{}.typeof", label), (api.js_typeof)(J, idx));
    let t = (api.js_type)(J, idx);
    p_int(&format!("{}.type", label), t);
    match t {
        T_BOOL => p_int(&format!("{}.b", label), (api.js_toboolean)(J, idx)),
        T_NUM => p_num(&format!("{}.n", label), (api.js_tonumber)(J, idx)),
        T_STR => p_str(&format!("{}.s", label), (api.js_tostring)(J, idx)),
        T_FUN | T_OBJ => {
            p_int(&format!("{}.isarray", label), (api.js_isarray)(J, idx));
            p_int(&format!("{}.iserror", label), (api.js_iserror)(J, idx));
            p_int(&format!("{}.iscallable", label), (api.js_iscallable)(J, idx));
            p_int(&format!("{}.isregexp", label), (api.js_isregexp)(J, idx));
        }
        _ => {}
    }
}

unsafe fn preds(api: &Api, J: JS, idx: c_int, label: &str) {
    p_str(&format!("{}.typeof", label), (api.js_typeof)(J, idx));
    p_int(&format!("{}.type", label), (api.js_type)(J, idx));
    p_int(&format!("{}.isdefined", label), (api.js_isdefined)(J, idx));
    p_int(&format!("{}.isundefined", label), (api.js_isundefined)(J, idx));
    p_int(&format!("{}.isnull", label), (api.js_isnull)(J, idx));
    p_int(&format!("{}.isboolean", label), (api.js_isboolean)(J, idx));
    p_int(&format!("{}.isnumber", label), (api.js_isnumber)(J, idx));
    p_int(&format!("{}.isstring", label), (api.js_isstring)(J, idx));
    p_int(&format!("{}.isprimitive", label), (api.js_isprimitive)(J, idx));
    p_int(&format!("{}.isobject", label), (api.js_isobject)(J, idx));
    p_int(&format!("{}.isarray", label), (api.js_isarray)(J, idx));
    p_int(&format!("{}.isregexp", label), (api.js_isregexp)(J, idx));
    p_int(&format!("{}.iscoercible", label), (api.js_iscoercible)(J, idx));
    p_int(&format!("{}.iscallable", label), (api.js_iscallable)(J, idx));
    p_int(&format!("{}.iserror", label), (api.js_iserror)(J, idx));
    p_int(&format!("{}.isnumberobject", label), (api.js_isnumberobject)(J, idx));
    p_int(&format!("{}.isstringobject", label), (api.js_isstringobject)(J, idx));
    p_int(&format!("{}.isbooleanobject", label), (api.js_isbooleanobject)(J, idx));
    p_int(&format!("{}.isdateobject", label), (api.js_isdateobject)(J, idx));
}

/// Enumerate all keys of the value at absolute index `idx`.
unsafe fn dumpkeys(api: &Api, J: JS, idx: c_int, own: c_int, label: &str) {
    (api.js_pushiterator)(J, idx, own);
    let it = (api.js_gettop)(J) - 1;
    let mut n = 0;
    loop {
        let k = (api.js_nextiterator)(J, it);
        if k.is_null() {
            break;
        }
        p_str(&format!("{}.key[{}]", label, n), k);
        n += 1;
        if n > 4000 {
            p_line("KEYCAP");
            break;
        }
    }
    p_int(&format!("{}.nkeys", label), n);
    (api.js_pop)(J, 1);
}

/// has + get for one index.
unsafe fn dumpidx(api: &Api, J: JS, idx: c_int, i: c_int, label: &str) {
    let h = (api.js_hasindex)(J, idx, i);
    p_int(&format!("{}[{}].has", label, i), h);
    if h != 0 {
        dumpv(api, J, -1, &format!("{}[{}].hv", label, i));
        (api.js_pop)(J, 1);
    }
    (api.js_getindex)(J, idx, i);
    dumpv(api, J, -1, &format!("{}[{}]", label, i));
    (api.js_pop)(J, 1);
}

/// get only (cheaper for long arrays).
unsafe fn dumpidx_get(api: &Api, J: JS, idx: c_int, i: c_int, label: &str) {
    (api.js_getindex)(J, idx, i);
    dumpv(api, J, -1, &format!("{}[{}]", label, i));
    (api.js_pop)(J, 1);
}

unsafe fn dumparray(api: &Api, J: JS, idx: c_int, label: &str, extra: &[c_int]) {
    let len = (api.js_getlength)(J, idx);
    p_int(&format!("{}.length", label), len);
    let full = len <= 64;
    let mut i = 0;
    while i < len && i < 4096 {
        if full {
            dumpidx(api, J, idx, i, label);
        } else {
            dumpidx_get(api, J, idx, i, label);
        }
        i += 1;
    }
    for &e in extra {
        dumpidx(api, J, idx, e, label);
    }
}

unsafe fn dumpprop(api: &Api, J: JS, idx: c_int, name: &str, label: &str) {
    let n = cs(name);
    let h = (api.js_hasproperty)(J, idx, n.as_ptr());
    p_int(&format!("{}.{}.has", label, name), h);
    if h != 0 {
        dumpv(api, J, -1, &format!("{}.{}.hv", label, name));
        (api.js_pop)(J, 1);
    }
    (api.js_getproperty)(J, idx, n.as_ptr());
    dumpv(api, J, -1, &format!("{}.{}", label, name));
    (api.js_pop)(J, 1);
}

unsafe fn setprop_num(api: &Api, J: JS, idx: c_int, name: &str, v: f64) {
    let n = cs(name);
    (api.js_pushnumber)(J, v);
    (api.js_setproperty)(J, idx, n.as_ptr());
}

unsafe fn setprop_str(api: &Api, J: JS, idx: c_int, name: &str, v: &str) {
    let n = cs(name);
    let s = cs(v);
    (api.js_pushstring)(J, s.as_ptr());
    (api.js_setproperty)(J, idx, n.as_ptr());
}

/// Push a fresh flat array [0*10, 1*10, ...] with `n` elements; returns its
/// absolute stack index.
unsafe fn mkflat(api: &Api, J: JS, n: c_int) -> c_int {
    (api.js_newarray)(J);
    let a = (api.js_gettop)(J) - 1;
    for i in 0..n {
        (api.js_pushnumber)(J, (i as f64) * 10.0);
        (api.js_setindex)(J, a, i);
    }
    a
}

/// Push an array that has been forced out of the simple representation.
unsafe fn mksparse(api: &Api, J: JS, ids: &[c_int]) -> c_int {
    (api.js_newarray)(J);
    let a = (api.js_gettop)(J) - 1;
    for &i in ids {
        (api.js_pushnumber)(J, (i as f64) + 0.5);
        (api.js_setindex)(J, a, i);
    }
    a
}

unsafe fn dostr(api: &Api, J: JS, tag: &str, src: &str) {
    let s = cs(src);
    let rc = (api.js_dostring)(J, s.as_ptr());
    p_int(&format!("{}.dostring_rc", tag), rc);
    p_int(&format!("{}.top", tag), (api.js_gettop)(J));
    flush();
}

/* =============================== row 29 =============================== */

#[test]
fn cfg29_newobject_setget() {
    diff("cfg29", |api| unsafe {
        let J = newstate(api, 0);
        p_int("top0", (api.js_gettop)(J));

        /* --- js_newobject --- */
        (api.js_newobject)(J);
        let o = (api.js_gettop)(J) - 1;
        p_int("top1", (api.js_gettop)(J));
        preds(api, J, o, "o");

        setprop_num(api, J, o, "a", 1.0);
        setprop_str(api, J, o, "b", "two");
        (api.js_pushboolean)(J, 1);
        let c = cs("c");
        (api.js_setproperty)(J, o, c.as_ptr());
        (api.js_pushnull)(J);
        let d = cs("d");
        (api.js_setproperty)(J, o, d.as_ptr());
        (api.js_pushundefined)(J);
        let e = cs("e");
        (api.js_setproperty)(J, o, e.as_ptr());
        p_int("top2", (api.js_gettop)(J));

        for n in ["a", "b", "c", "d", "e", "missing", "toString"] {
            dumpprop(api, J, o, n, "o");
        }
        dumpkeys(api, J, o, 1, "o.own");
        dumpkeys(api, J, o, 0, "o.all");

        /* overwrite + delete */
        setprop_num(api, J, o, "a", 42.0);
        dumpprop(api, J, o, "a", "o2");
        let a = cs("a");
        (api.js_delproperty)(J, o, a.as_ptr());
        dumpprop(api, J, o, "a", "o3");
        dumpkeys(api, J, o, 1, "o3.own");
        p_int("top3", (api.js_gettop)(J));

        /* --- js_newobjectx with an object prototype --- */
        (api.js_copy)(J, o);
        (api.js_newobjectx)(J);
        let x = (api.js_gettop)(J) - 1;
        preds(api, J, x, "x");
        for n in ["b", "c", "a", "missing"] {
            dumpprop(api, J, x, n, "x");
        }
        dumpkeys(api, J, x, 1, "x.own");
        dumpkeys(api, J, x, 0, "x.all");
        setprop_num(api, J, x, "own1", 7.0);
        dumpprop(api, J, x, "own1", "x");
        dumpkeys(api, J, x, 1, "x2.own");

        /* --- js_newobjectx with a non-object on the stack (prototype NULL) */
        (api.js_pushnumber)(J, 5.0);
        (api.js_newobjectx)(J);
        let y = (api.js_gettop)(J) - 1;
        preds(api, J, y, "y");
        for n in ["toString", "missing"] {
            dumpprop(api, J, y, n, "y");
        }
        setprop_num(api, J, y, "z", 1.0);
        dumpprop(api, J, y, "z", "y");
        dumpkeys(api, J, y, 1, "y.own");
        dumpkeys(api, J, y, 0, "y.all");

        (api.js_pushundefined)(J);
        (api.js_newobjectx)(J);
        let y2 = (api.js_gettop)(J) - 1;
        preds(api, J, y2, "y2");
        dumpkeys(api, J, y2, 0, "y2.all");

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 30 =============================== */

#[test]
fn cfg30_flat_array_setindex() {
    diff("cfg30", |api| unsafe {
        let J = newstate(api, 0);
        for n in [0, 1, 2, 7, 8, 9, 64, 1000] {
            p_int("n", n);
            let a = mkflat(api, J, n);
            p_int("top", (api.js_gettop)(J));
            preds(api, J, a, &format!("a{}", n));
            dumparray(api, J, a, &format!("a{}", n), &[n, n + 1, n + 100]);
            dumpkeys(api, J, a, 1, &format!("a{}.own", n));
            dumpkeys(api, J, a, 0, &format!("a{}.all", n));
            dumpprop(api, J, a, "length", &format!("a{}", n));
            (api.js_pop)(J, 1);
        }
        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 31 =============================== */

#[test]
fn cfg31_sparse_array_setindex() {
    diff("cfg31", |api| unsafe {
        let J = newstate(api, 0);

        (api.js_newarray)(J);
        let a = (api.js_gettop)(J) - 1;
        for (step, i) in [0, 5, 3, 100].iter().enumerate() {
            (api.js_pushnumber)(J, (*i as f64) + 0.5);
            (api.js_setindex)(J, a, *i);
            p_int(&format!("step{}.len", step), (api.js_getlength)(J, a));
            p_int(&format!("step{}.top", step), (api.js_gettop)(J));
            dumpkeys(api, J, a, 1, &format!("step{}.own", step));
        }

        dumparray(api, J, a, "a", &[100, 101, 1000, -1]);
        dumpkeys(api, J, a, 1, "a.own");
        dumpkeys(api, J, a, 0, "a.all");
        dumpprop(api, J, a, "length", "a");
        for n in ["0", "3", "5", "100", "1", "2"] {
            dumpprop(api, J, a, n, "a");
        }

        /* still-simple array that gets an out of range index straight away */
        let b = mksparse(api, J, &[7]);
        dumparray(api, J, b, "b", &[7, 8]);
        dumpkeys(api, J, b, 1, "b.own");

        /* descending writes */
        (api.js_newarray)(J);
        let c = (api.js_gettop)(J) - 1;
        for i in [4, 3, 2, 1, 0] {
            (api.js_pushnumber)(J, i as f64);
            (api.js_setindex)(J, c, i);
            p_int(&format!("c{}.len", i), (api.js_getlength)(J, c));
        }
        dumparray(api, J, c, "c", &[5]);
        dumpkeys(api, J, c, 1, "c.own");

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 32 =============================== */

#[test]
fn cfg32_array_named_property() {
    diff("cfg32", |api| unsafe {
        let J = newstate(api, 0);

        /* js_setproperty with a named key */
        let a = mkflat(api, J, 3);
        setprop_str(api, J, a, "foo", "bar");
        p_int("a.len", (api.js_getlength)(J, a));
        dumparray(api, J, a, "a", &[3]);
        dumpprop(api, J, a, "foo", "a");
        dumpkeys(api, J, a, 1, "a.own");
        dumpkeys(api, J, a, 0, "a.all");
        /* keep writing indices afterwards */
        (api.js_pushnumber)(J, 99.0);
        (api.js_setindex)(J, a, 3);
        dumparray(api, J, a, "a2", &[4]);
        dumpkeys(api, J, a, 1, "a2.own");

        /* js_defproperty with a named key (unflattens) */
        let b = mkflat(api, J, 4);
        (api.js_pushnumber)(J, 5.0);
        let nm = cs("named");
        (api.js_defproperty)(J, b, nm.as_ptr(), 0);
        p_int("b.len", (api.js_getlength)(J, b));
        dumparray(api, J, b, "b", &[4]);
        dumpprop(api, J, b, "named", "b");
        dumpkeys(api, J, b, 1, "b.own");
        dumpkeys(api, J, b, 0, "b.all");
        (api.js_pushnumber)(J, 40.0);
        (api.js_setindex)(J, b, 4);
        dumparray(api, J, b, "b2", &[5]);
        dumpkeys(api, J, b, 1, "b2.own");

        /* named key that looks like a float / negative index */
        let c = mkflat(api, J, 2);
        setprop_str(api, J, c, "1.5", "f");
        setprop_str(api, J, c, "-1", "neg");
        setprop_str(api, J, c, "01", "lead0");
        setprop_str(api, J, c, "", "empty");
        p_int("c.len", (api.js_getlength)(J, c));
        dumparray(api, J, c, "c", &[2]);
        for n in ["1.5", "-1", "01", ""] {
            dumpprop(api, J, c, n, "c");
        }
        dumpkeys(api, J, c, 1, "c.own");

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 33 =============================== */

#[test]
fn cfg33_get_set_length() {
    diff("cfg33", |api| unsafe {
        let J = newstate(api, 0);

        /* ---- flat array ---- */
        let a = mkflat(api, J, 5);
        p_int("a.len0", (api.js_getlength)(J, a));
        for &l in [8, 3, 0, 4, 100000].iter() {
            (api.js_setlength)(J, a, l);
            p_int(&format!("a.setlen{}", l), (api.js_getlength)(J, a));
            if l > 1000 {
                for i in [0, 1, 4, 5, 99999, 100000] {
                    dumpidx(api, J, a, i, &format!("a.after{}", l));
                }
            } else {
                dumparray(api, J, a, &format!("a.after{}", l), &[l, l + 1]);
            }
            dumpkeys(api, J, a, 1, &format!("a.after{}.own", l));
        }
        /* write after the huge length */
        (api.js_pushnumber)(J, 1.0);
        (api.js_setindex)(J, a, 0);
        p_int("a.len_after_write", (api.js_getlength)(J, a));
        dumpidx(api, J, a, 0, "a.w");
        (api.js_pop)(J, 1);

        /* ---- unflattened array ---- */
        let b = mksparse(api, J, &[0, 1, 2, 9]);
        p_int("b.len0", (api.js_getlength)(J, b));
        dumpkeys(api, J, b, 1, "b.own0");
        for &l in [20, 10, 5, 0, 3, 100000].iter() {
            (api.js_setlength)(J, b, l);
            p_int(&format!("b.setlen{}", l), (api.js_getlength)(J, b));
            if l > 1000 {
                for i in [0, 1, 2, 9, 99999] {
                    dumpidx(api, J, b, i, &format!("b.after{}", l));
                }
            } else {
                dumparray(api, J, b, &format!("b.after{}", l), &[l, l + 1]);
            }
            dumpkeys(api, J, b, 1, &format!("b.after{}.own", l));
        }
        (api.js_pop)(J, 1);

        /* ---- many properties so resizearray takes the delete loop ---- */
        let c = mksparse(api, J, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        p_int("c.len0", (api.js_getlength)(J, c));
        (api.js_setlength)(J, c, 4);
        p_int("c.len1", (api.js_getlength)(J, c));
        dumparray(api, J, c, "c", &[4, 11]);
        dumpkeys(api, J, c, 1, "c.own");
        (api.js_pop)(J, 1);

        /* ---- js_setlength on a plain object ---- */
        (api.js_newobject)(J);
        let d = (api.js_gettop)(J) - 1;
        (api.js_setlength)(J, d, 7);
        p_int("d.len", (api.js_getlength)(J, d));
        dumpprop(api, J, d, "length", "d");
        dumpkeys(api, J, d, 1, "d.own");
        (api.js_pop)(J, 1);

        /* ---- invalid lengths (throw) ---- */
        pcall(api, J, "neg_flat", |api, J| {
            let a = mkflat(api, J, 3);
            (api.js_setlength)(J, a, -1);
            p_int("unreached", (api.js_getlength)(J, a));
        });
        pcall(api, J, "neg_sparse", |api, J| {
            let a = mksparse(api, J, &[0, 4]);
            (api.js_setlength)(J, a, -5);
            p_int("unreached", (api.js_getlength)(J, a));
        });
        pcall(api, J, "toolarge", |api, J| {
            let a = mkflat(api, J, 3);
            (api.js_setlength)(J, a, (1 << 26) + 1);
            p_int("len", (api.js_getlength)(J, a));
        });
        pcall(api, J, "atlimit", |api, J| {
            let a = mkflat(api, J, 3);
            (api.js_setlength)(J, a, 1 << 26);
            p_int("len", (api.js_getlength)(J, a));
        });

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 34 =============================== */

#[test]
fn cfg34_lowlevel_resize_unflatten() {
    /* (a) unflatten + resize on a non-simple array object */
    diff("cfg34a", |api| unsafe {
        let J = newstate(api, 0);
        let a = mkflat(api, J, 5);
        let obj = (api.js_toobject)(J, a);
        p_ptr_nonnull("obj", obj);
        dumpkeys(api, J, a, 1, "flat.own");

        (api.jsR_unflattenarray)(J, obj);
        p_line("unflattened");
        p_int("len", (api.js_getlength)(J, a));
        dumparray(api, J, a, "u", &[5]);
        dumpkeys(api, J, a, 1, "u.own");

        /* unflatten again: no-op */
        (api.jsR_unflattenarray)(J, obj);
        p_int("len2", (api.js_getlength)(J, a));
        dumpkeys(api, J, a, 1, "u2.own");

        /* grow */
        (api.jsV_resizearray)(J, obj, 20);
        p_int("len_grow", (api.js_getlength)(J, a));
        dumparray(api, J, a, "g", &[20]);
        dumpkeys(api, J, a, 1, "g.own");

        /* shrink mid-way */
        (api.jsV_resizearray)(J, obj, 3);
        p_int("len_shrink", (api.js_getlength)(J, a));
        dumparray(api, J, a, "s", &[3, 4]);
        dumpkeys(api, J, a, 1, "s.own");

        /* shrink to 0 */
        (api.jsV_resizearray)(J, obj, 0);
        p_int("len_zero", (api.js_getlength)(J, a));
        dumparray(api, J, a, "z", &[0, 1]);
        dumpkeys(api, J, a, 1, "z.own");

        /* grow again, then shrink through the iterator path
         * (length > count*2) */
        (api.jsV_resizearray)(J, obj, 1000);
        (api.js_pushnumber)(J, 7.0);
        (api.js_setindex)(J, a, 900);
        p_int("len_900", (api.js_getlength)(J, a));
        dumpkeys(api, J, a, 1, "it.own");
        (api.jsV_resizearray)(J, obj, 100);
        p_int("len_after_it", (api.js_getlength)(J, a));
        dumpkeys(api, J, a, 1, "it2.own");
        dumpidx(api, J, a, 900, "it2");

        /* jsR_unflattenarray on a non-array object: no-op */
        (api.js_newobject)(J);
        let o = (api.js_gettop)(J) - 1;
        let oo = (api.js_toobject)(J, o);
        setprop_num(api, J, o, "k", 1.0);
        (api.jsR_unflattenarray)(J, oo);
        dumpprop(api, J, o, "k", "plain");
        dumpkeys(api, J, o, 1, "plain.own");

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    /* (b) jsV_resizearray on a *simple* array: the C source asserts
     * `!obj->u.a.simple` and the C library is built with assertions, so the
     * process dies with SIGABRT.  The Rust translation keeps the check but
     * calls abort() directly, which prints no diagnostic — so only the exit
     * status and the output produced *before* the call can be compared here
     * (glibc's assert message also embeds the absolute path of jsproperty.c,
     * which can never match). */
    let body = |api: &Api| unsafe {
        let J = newstate(api, 0);
        let a = mkflat(api, J, 4);
        let obj = (api.js_toobject)(J, a);
        p_ptr_nonnull("obj", obj);
        p_int("len_before", (api.js_getlength)(J, a));
        p_line("calling jsV_resizearray on simple array");
        flush();
        (api.jsV_resizearray)(J, obj, 2);
        p_line("survived");
        p_int("len_after", (api.js_getlength)(J, a));
        dumparray(api, J, a, "after", &[2, 3, 4]);
        dumpkeys(api, J, a, 1, "after.own");
        flush();
        (api.js_freestate)(J);
    };
    let l = libs();
    let (oc, sc) = capture("cfg34b_c", || body(&l.c));
    let (or, sr) = capture("cfg34b_r", || body(&l.r));
    let sc_ = decode_status(sc);
    let sr_ = decode_status(sr);
    let ac = String::from_utf8_lossy(&oc);
    let ar = String::from_utf8_lossy(&or);
    /* both must die the same way, and neither may get past the call */
    assert_eq!(
        sc_, sr_,
        "jsV_resizearray on a simple array: status differs\nC:\n{}\nR:\n{}",
        ac, ar
    );
    assert_eq!(sc_, "signal 6", "expected SIGABRT, got {}\n{}", sc_, ac);
    assert!(!ac.contains("survived"), "C survived the assert:\n{}", ac);
    assert!(!ar.contains("survived"), "Rust survived the assert:\n{}", ar);
    let pre_c: Vec<&str> = ac.lines().take(3).collect();
    let pre_r: Vec<&str> = ar.lines().take(3).collect();
    assert_eq!(pre_c, pre_r, "output before the abort differs");
}

/* =============================== row 35 =============================== */

#[test]
fn cfg35_hasindex_getindex_delindex() {
    diff("cfg35", |api| unsafe {
        let J = newstate(api, 0);
        let idxs: [c_int; 8] = [0, 4, 7, 8, 100, 1 << 20, -1, -2147483648];

        for &k in idxs.iter() {
            p_int("case", k);

            /* flat array */
            let a = mkflat(api, J, 8);
            p_int("flat.has", (api.js_hasindex)(J, a, k));
            if (api.js_hasindex)(J, a, k) != 0 {
                (api.js_pop)(J, 2);
            }
            (api.js_getindex)(J, a, k);
            dumpv(api, J, -1, "flat.get");
            (api.js_pop)(J, 1);
            (api.js_delindex)(J, a, k);
            p_int("flat.len_after_del", (api.js_getlength)(J, a));
            dumparray(api, J, a, "flat.after", &[8, 9]);
            dumpkeys(api, J, a, 1, "flat.after.own");
            (api.js_pop)(J, 1);

            /* unflattened array */
            let b = mksparse(api, J, &[0, 1, 2, 3, 4, 5, 6, 7, 20]);
            p_int("sp.has", (api.js_hasindex)(J, b, k));
            if (api.js_hasindex)(J, b, k) != 0 {
                (api.js_pop)(J, 2);
            }
            (api.js_getindex)(J, b, k);
            dumpv(api, J, -1, "sp.get");
            (api.js_pop)(J, 1);
            (api.js_delindex)(J, b, k);
            p_int("sp.len_after_del", (api.js_getlength)(J, b));
            dumparray(api, J, b, "sp.after", &[20, 21]);
            dumpkeys(api, J, b, 1, "sp.after.own");
            (api.js_pop)(J, 1);
        }

        /* delete every element of a flat array from the front */
        let c = mkflat(api, J, 5);
        for i in 0..5 {
            (api.js_delindex)(J, c, i);
            p_int(&format!("front{}.len", i), (api.js_getlength)(J, c));
            dumpkeys(api, J, c, 1, &format!("front{}.own", i));
        }
        dumparray(api, J, c, "front", &[5]);
        (api.js_pop)(J, 1);

        /* delete every element of a flat array from the back */
        let d = mkflat(api, J, 5);
        for i in (0..5).rev() {
            (api.js_delindex)(J, d, i);
            p_int(&format!("back{}.len", i), (api.js_getlength)(J, d));
            dumpkeys(api, J, d, 1, &format!("back{}.own", i));
        }
        dumparray(api, J, d, "back", &[5]);
        (api.js_pop)(J, 1);

        /* on a plain object and a string object */
        (api.js_newobject)(J);
        let e = (api.js_gettop)(J) - 1;
        setprop_num(api, J, e, "0", 1.0);
        setprop_num(api, J, e, "3", 2.0);
        for k in [0, 1, 3, 1 << 20, -1] {
            dumpidx(api, J, e, k, "obj");
        }
        (api.js_delindex)(J, e, 0);
        dumpkeys(api, J, e, 1, "obj.own");
        (api.js_pop)(J, 1);

        let sv = cs("abc");
        (api.js_newstring)(J, sv.as_ptr());
        let f = (api.js_gettop)(J) - 1;
        for k in [0, 2, 3, -1] {
            dumpidx(api, J, f, k, "strobj");
        }
        (api.js_pop)(J, 1);

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 36 =============================== */

unsafe fn cfg36_block(api: &Api, J: JS, atts: c_int, strict: bool) {
    let tag = format!("atts{}", atts);
    (api.js_newobject)(J);
    let o = (api.js_gettop)(J) - 1;
    if strict {
        /* the protected bodies below run in their own frame, so they pick the
         * object up from the registry */
        (api.js_copy)(J, o);
        let k = cs("cfg36obj");
        (api.js_setregistry)(J, k.as_ptr());
    }

    let p = cs("p");
    (api.js_pushnumber)(J, 100.0);
    (api.js_defproperty)(J, o, p.as_ptr(), atts);
    dumpprop(api, J, o, "p", &tag);
    dumpkeys(api, J, o, 1, &format!("{}.def.own", tag));

    /* redefine with a different value */
    if strict {
        pcall(api, J, &format!("{}.redef", tag), move |api, J| {
            let o = 0; /* `this` is undefined; use the registry instead */
            let _ = o;
            let k = cs("cfg36obj");
            (api.js_getregistry)(J, k.as_ptr());
            let oi = (api.js_gettop)(J) - 1;
            let p = cs("p");
            (api.js_pushnumber)(J, 200.0);
            (api.js_defproperty)(J, oi, p.as_ptr(), 0);
            dumpprop(api, J, oi, "p", "redef");
        });
    } else {
        (api.js_pushnumber)(J, 200.0);
        (api.js_defproperty)(J, o, p.as_ptr(), 0);
        dumpprop(api, J, o, "p", &format!("{}.redef", tag));
    }

    /* setproperty */
    if strict {
        pcall(api, J, &format!("{}.set", tag), move |api, J| {
            let k = cs("cfg36obj");
            (api.js_getregistry)(J, k.as_ptr());
            let oi = (api.js_gettop)(J) - 1;
            let p = cs("p");
            (api.js_pushnumber)(J, 300.0);
            (api.js_setproperty)(J, oi, p.as_ptr());
            dumpprop(api, J, oi, "p", "set");
        });
    } else {
        (api.js_pushnumber)(J, 300.0);
        (api.js_setproperty)(J, o, p.as_ptr());
        dumpprop(api, J, o, "p", &format!("{}.set", tag));
    }

    /* delproperty */
    if strict {
        pcall(api, J, &format!("{}.del", tag), move |api, J| {
            let k = cs("cfg36obj");
            (api.js_getregistry)(J, k.as_ptr());
            let oi = (api.js_gettop)(J) - 1;
            let p = cs("p");
            (api.js_delproperty)(J, oi, p.as_ptr());
            dumpprop(api, J, oi, "p", "del");
        });
    } else {
        (api.js_delproperty)(J, o, p.as_ptr());
        dumpprop(api, J, o, "p", &format!("{}.del", tag));
    }

    dumpkeys(api, J, o, 1, &format!("{}.end.own", tag));
    dumpkeys(api, J, o, 0, &format!("{}.end.all", tag));
    p_int(&format!("{}.top", tag), (api.js_gettop)(J));
    (api.js_pop)(J, 1);
}

#[test]
fn cfg36_defproperty_atts() {
    diff("cfg36_nonstrict", |api| unsafe {
        let J = newstate(api, 0);
        for atts in 0..=7 {
            cfg36_block(api, J, atts, false);
        }
        /* several properties with mixed attributes, then enumerate */
        (api.js_newobject)(J);
        let o = (api.js_gettop)(J) - 1;
        for atts in 0..=7 {
            let n = cs(&format!("k{}", atts));
            (api.js_pushnumber)(J, atts as f64);
            (api.js_defproperty)(J, o, n.as_ptr(), atts);
        }
        dumpkeys(api, J, o, 1, "mixed.own");
        dumpkeys(api, J, o, 0, "mixed.all");
        for atts in 0..=7 {
            dumpprop(api, J, o, &format!("k{}", atts), "mixed");
        }
        (api.js_pop)(J, 1);
        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    diff("cfg36_strict", |api| unsafe {
        let J = newstate(api, JS_STRICT);
        for atts in 0..=7 {
            cfg36_block(api, J, atts, true);
        }
        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 37 =============================== */

unsafe extern "C" fn getter_fn(J: JS) {
    let api = &*CUR_API;
    p_line("GETTER");
    let h = cs("hidden");
    (api.js_getproperty)(J, 0, h.as_ptr());
    dumpv(api, J, -1, "getter.hidden");
    /* return hidden * 2 if numeric, else the string "G" */
    if (api.js_isnumber)(J, -1) != 0 {
        let v = (api.js_tonumber)(J, -1);
        (api.js_pop)(J, 1);
        (api.js_pushnumber)(J, v * 2.0);
    } else {
        (api.js_pop)(J, 1);
        let s = cs("G");
        (api.js_pushstring)(J, s.as_ptr());
    }
}

unsafe extern "C" fn setter_fn(J: JS) {
    let api = &*CUR_API;
    p_line("SETTER");
    p_int("setter.top", (api.js_gettop)(J));
    dumpv(api, J, 1, "setter.arg");
    (api.js_copy)(J, 1);
    let h = cs("hidden");
    (api.js_setproperty)(J, 0, h.as_ptr());
}

unsafe fn cfg37_block(api: &Api, J: JS, which: c_int, atts: c_int) {
    let tag = format!("acc{}_{}", which, atts);
    (api.js_newobject)(J);
    let o = (api.js_gettop)(J) - 1;

    let gname = cs("g");
    let sname = cs("s");
    if which == 0 || which == 1 {
        (api.js_newcfunction)(J, Some(getter_fn), gname.as_ptr(), 0);
    } else {
        (api.js_pushnull)(J);
    }
    if which == 0 || which == 2 {
        (api.js_newcfunction)(J, Some(setter_fn), sname.as_ptr(), 1);
    } else {
        (api.js_pushnull)(J);
    }
    let acc = cs("acc");
    (api.js_defaccessor)(J, o, acc.as_ptr(), atts);
    p_int(&format!("{}.top_after_def", tag), (api.js_gettop)(J));

    dumpkeys(api, J, o, 1, &format!("{}.own", tag));
    dumpkeys(api, J, o, 0, &format!("{}.all", tag));

    /* read through the getter */
    dumpprop(api, J, o, "acc", &tag);

    /* write through the setter */
    (api.js_pushnumber)(J, 21.0);
    (api.js_setproperty)(J, o, acc.as_ptr());
    dumpprop(api, J, o, "hidden", &tag);
    dumpprop(api, J, o, "acc", &format!("{}.after_set", tag));
    dumpkeys(api, J, o, 1, &format!("{}.after_set.own", tag));

    /* delete the accessor */
    (api.js_delproperty)(J, o, acc.as_ptr());
    dumpprop(api, J, o, "acc", &format!("{}.after_del", tag));
    dumpkeys(api, J, o, 1, &format!("{}.after_del.own", tag));
    p_int(&format!("{}.topend", tag), (api.js_gettop)(J));
    (api.js_pop)(J, 1);
}

#[test]
fn cfg37_defaccessor() {
    diff("cfg37", |api| unsafe {
        CUR_API = api as *const Api;
        let J = newstate(api, 0);
        for which in 0..3 {
            for atts in [0, JS_DONTENUM, JS_READONLY, JS_DONTCONF] {
                cfg37_block(api, J, which, atts);
            }
        }

        /* accessor defined through a script, then used from C */
        dostr(
            api,
            J,
            "script",
            "var o = {}; var log = []; \
             Object.defineProperty(o, 'x', { \
               get: function () { return 11; }, \
               set: function (v) { this.y = v + 1; }, \
               enumerable: true });",
        );
        let on = cs("o");
        (api.js_getglobal)(J, on.as_ptr());
        let o = (api.js_gettop)(J) - 1;
        dumpprop(api, J, o, "x", "so");
        setprop_num(api, J, o, "x", 5.0);
        dumpprop(api, J, o, "y", "so");
        dumpkeys(api, J, o, 1, "so.own");
        (api.js_pop)(J, 1);

        /* strict: writing to a getter-only accessor throws */
        (api.js_freestate)(J);
        let J = newstate(api, JS_STRICT);
        CUR_API = api as *const Api;
        pcall(api, J, "strict_getter_only", |api, J| {
            (api.js_newobject)(J);
            let o = (api.js_gettop)(J) - 1;
            let g = cs("g");
            (api.js_newcfunction)(J, Some(getter_fn), g.as_ptr(), 0);
            (api.js_pushnull)(J);
            let acc = cs("acc");
            (api.js_defaccessor)(J, o, acc.as_ptr(), 0);
            dumpprop(api, J, o, "acc", "sgo");
            (api.js_pushnumber)(J, 1.0);
            (api.js_setproperty)(J, o, acc.as_ptr());
            p_line("unreached");
        });
        pcall(api, J, "strict_dontconf", |api, J| {
            (api.js_newobject)(J);
            let o = (api.js_gettop)(J) - 1;
            let g = cs("g");
            (api.js_newcfunction)(J, Some(getter_fn), g.as_ptr(), 0);
            (api.js_pushnull)(J);
            let acc = cs("acc");
            (api.js_defaccessor)(J, o, acc.as_ptr(), JS_DONTCONF);
            (api.js_newcfunction)(J, Some(getter_fn), g.as_ptr(), 0);
            (api.js_pushnull)(J);
            (api.js_defaccessor)(J, o, acc.as_ptr(), 0);
            p_line("unreached");
        });
        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 38 =============================== */

#[test]
fn cfg38_property_own_inherited_missing() {
    diff("cfg38", |api| unsafe {
        let J = newstate(api, 0);
        dostr(
            api,
            J,
            "setup",
            "var P = { a: 1, shadow: 'proto', 0: 'p0' }; \
             function F() {} \
             F.prototype = P; \
             var C = new F(); \
             C.b = 2; C.shadow = 'own'; C[1] = 'c1'; C[''] = 'empty';",
        );
        let cn = cs("C");
        (api.js_getglobal)(J, cn.as_ptr());
        let o = (api.js_gettop)(J) - 1;
        preds(api, J, o, "C");
        dumpkeys(api, J, o, 1, "C.own");
        dumpkeys(api, J, o, 0, "C.all");

        let long: String = std::iter::repeat('x').take(200).collect();
        let names: Vec<String> = vec![
            "b".into(),
            "a".into(),
            "shadow".into(),
            "missing".into(),
            "0".into(),
            "1".into(),
            "2".into(),
            "".into(),
            "toString".into(),
            "hasOwnProperty".into(),
            "constructor".into(),
            long,
            "ключ✓".into(),
            "é".into(),
            "1.5".into(),
            "-3".into(),
            "4294967295".into(),
            "01".into(),
            "0x10".into(),
            " ".into(),
            "a\tb".into(),
        ];

        for name in names.iter() {
            let label = if name.len() > 32 {
                format!("long{}", name.len())
            } else {
                format!("n<{}>", name)
            };
            let n = cs(name);
            let mut k: c_int = -1;
            p_int(
                &format!("{}.isarrayindex", label),
                (api.js_isarrayindex)(J, n.as_ptr(), &mut k),
            );
            p_int(&format!("{}.arrayindex", label), k);

            let h = (api.js_hasproperty)(J, o, n.as_ptr());
            p_int(&format!("{}.has", label), h);
            if h != 0 {
                dumpv(api, J, -1, &format!("{}.hv", label));
                (api.js_pop)(J, 1);
            }
            (api.js_getproperty)(J, o, n.as_ptr());
            dumpv(api, J, -1, &format!("{}.get", label));
            (api.js_pop)(J, 1);

            /* write, then re-read (may create an own property) */
            let v = cs("SET");
            (api.js_pushstring)(J, v.as_ptr());
            (api.js_setproperty)(J, o, n.as_ptr());
            (api.js_getproperty)(J, o, n.as_ptr());
            dumpv(api, J, -1, &format!("{}.get2", label));
            (api.js_pop)(J, 1);

            /* delete, then re-read (inherited values reappear) */
            (api.js_delproperty)(J, o, n.as_ptr());
            let h2 = (api.js_hasproperty)(J, o, n.as_ptr());
            p_int(&format!("{}.has3", label), h2);
            if h2 != 0 {
                (api.js_pop)(J, 1);
            }
            (api.js_getproperty)(J, o, n.as_ptr());
            dumpv(api, J, -1, &format!("{}.get3", label));
            (api.js_pop)(J, 1);

            /* delete again (already gone) */
            (api.js_delproperty)(J, o, n.as_ptr());
            (api.js_getproperty)(J, o, n.as_ptr());
            dumpv(api, J, -1, &format!("{}.get4", label));
            (api.js_pop)(J, 1);
        }
        dumpkeys(api, J, o, 1, "C.end.own");
        dumpkeys(api, J, o, 0, "C.end.all");

        /* on the prototype itself */
        let pn = cs("P");
        (api.js_getglobal)(J, pn.as_ptr());
        let p = (api.js_gettop)(J) - 1;
        for n in ["a", "shadow", "0", "b"] {
            dumpprop(api, J, p, n, "P");
        }
        dumpkeys(api, J, p, 1, "P.own");
        (api.js_pop)(J, 1);

        /* transient object: setproperty on a primitive */
        (api.js_pushnumber)(J, 5.0);
        let t = (api.js_gettop)(J) - 1;
        let tn = cs("q");
        (api.js_pushnumber)(J, 1.0);
        (api.js_setproperty)(J, t, tn.as_ptr());
        dumpv(api, J, t, "prim_after_set");
        dumpprop(api, J, t, "q", "prim");
        (api.js_pop)(J, 1);

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 39 =============================== */

#[test]
fn cfg39_lowlevel_properties() {
    diff("cfg39", |api| unsafe {
        let J = newstate(api, 0);

        /* --- plain object with many tree properties --- */
        (api.js_newobject)(J);
        let oi = (api.js_gettop)(J) - 1;
        let obj = (api.js_toobject)(J, oi);
        p_ptr_nonnull("obj", obj);

        let mut rng = Rng::new(0xC0FFEE);
        let mut names: Vec<String> = Vec::new();
        for _ in 0..200 {
            names.push(format!("k{}", rng.next_u32() % 5000));
        }
        for (i, nm) in names.iter().enumerate() {
            let n = cs(nm);
            let r = (api.jsV_setproperty)(J, obj, n.as_ptr());
            p_ptr_nonnull(&format!("ins{}", i), r);
            /* give it a value through the public API */
            (api.js_pushnumber)(J, i as f64);
            (api.js_setproperty)(J, oi, n.as_ptr());
        }
        dumpkeys(api, J, oi, 1, "tree.own");

        let mut missing: Vec<String> = Vec::new();
        for i in 0..20 {
            missing.push(format!("zz{}", i));
        }

        for nm in names.iter().chain(missing.iter()) {
            let n = cs(nm);
            let a = (api.jsV_getownproperty)(J, obj, n.as_ptr());
            let b = (api.jsV_getproperty)(J, obj, n.as_ptr());
            let mut own: c_int = -1;
            let c = (api.jsV_getpropertyx)(J, obj, n.as_ptr(), &mut own);
            p_ptr_nonnull(&format!("{}.own", nm), a);
            p_ptr_nonnull(&format!("{}.proto", nm), b);
            p_ptr_nonnull(&format!("{}.x", nm), c);
            p_int(&format!("{}.ownflag", nm), own);
        }

        /* inherited names */
        for nm in ["toString", "valueOf", "hasOwnProperty", "nope"] {
            let n = cs(nm);
            p_ptr_nonnull(
                &format!("inh.{}.own", nm),
                (api.jsV_getownproperty)(J, obj, n.as_ptr()),
            );
            p_ptr_nonnull(
                &format!("inh.{}.proto", nm),
                (api.jsV_getproperty)(J, obj, n.as_ptr()),
            );
            let mut own: c_int = -1;
            p_ptr_nonnull(
                &format!("inh.{}.x", nm),
                (api.jsV_getpropertyx)(J, obj, n.as_ptr(), &mut own),
            );
            p_int(&format!("inh.{}.ownflag", nm), own);
        }

        /* delete every other name and look them all up again */
        for (i, nm) in names.iter().enumerate() {
            if i % 2 == 0 {
                let n = cs(nm);
                (api.jsV_delproperty)(J, obj, n.as_ptr());
            }
        }
        for nm in names.iter() {
            let n = cs(nm);
            let mut own: c_int = -1;
            p_ptr_nonnull(
                &format!("del.{}.own", nm),
                (api.jsV_getownproperty)(J, obj, n.as_ptr()),
            );
            p_ptr_nonnull(
                &format!("del.{}.x", nm),
                (api.jsV_getpropertyx)(J, obj, n.as_ptr(), &mut own),
            );
            p_int(&format!("del.{}.ownflag", nm), own);
        }
        dumpkeys(api, J, oi, 1, "tree.after_del.own");
        /* delete a name twice / delete a missing name */
        let g = cs("zz0");
        (api.jsV_delproperty)(J, obj, g.as_ptr());
        (api.jsV_delproperty)(J, obj, g.as_ptr());
        p_ptr_nonnull("zz0.after", (api.jsV_getownproperty)(J, obj, g.as_ptr()));
        (api.js_pop)(J, 1);

        /* --- flat array object --- */
        let ai = mkflat(api, J, 5);
        let arr = (api.js_toobject)(J, ai);
        for nm in ["0", "1", "4", "5", "length", "join"] {
            let n = cs(nm);
            let mut own: c_int = -1;
            p_ptr_nonnull(
                &format!("arr.{}.own", nm),
                (api.jsV_getownproperty)(J, arr, n.as_ptr()),
            );
            p_ptr_nonnull(
                &format!("arr.{}.proto", nm),
                (api.jsV_getproperty)(J, arr, n.as_ptr()),
            );
            p_ptr_nonnull(
                &format!("arr.{}.x", nm),
                (api.jsV_getpropertyx)(J, arr, n.as_ptr(), &mut own),
            );
            p_int(&format!("arr.{}.ownflag", nm), own);
        }
        /* jsV_setproperty on a simple array inserts into the tree while the
         * flat part stays live */
        let z = cs("0");
        p_ptr_nonnull("arr.set0", (api.jsV_setproperty)(J, arr, z.as_ptr()));
        dumparray(api, J, ai, "arr.after_set", &[5]);
        dumpkeys(api, J, ai, 1, "arr.after_set.own");
        p_ptr_nonnull("arr.0.own2", (api.jsV_getownproperty)(J, arr, z.as_ptr()));
        (api.jsV_delproperty)(J, arr, z.as_ptr());
        dumparray(api, J, ai, "arr.after_del", &[5]);
        dumpkeys(api, J, ai, 1, "arr.after_del.own");
        (api.js_pop)(J, 1);

        /* --- unflattened array object --- */
        let bi = mksparse(api, J, &[0, 3]);
        let barr = (api.js_toobject)(J, bi);
        for nm in ["0", "1", "3", "length"] {
            let n = cs(nm);
            let mut own: c_int = -1;
            p_ptr_nonnull(
                &format!("sp.{}.own", nm),
                (api.jsV_getownproperty)(J, barr, n.as_ptr()),
            );
            p_ptr_nonnull(
                &format!("sp.{}.x", nm),
                (api.jsV_getpropertyx)(J, barr, n.as_ptr(), &mut own),
            );
            p_int(&format!("sp.{}.ownflag", nm), own);
        }
        let one = cs("1");
        p_ptr_nonnull("sp.set1", (api.jsV_setproperty)(J, barr, one.as_ptr()));
        dumparray(api, J, bi, "sp.after_set", &[4]);
        dumpkeys(api, J, bi, 1, "sp.after_set.own");
        (api.js_pop)(J, 1);

        /* --- empty name and long name --- */
        (api.js_newobject)(J);
        let ei = (api.js_gettop)(J) - 1;
        let eobj = (api.js_toobject)(J, ei);
        let long: String = std::iter::repeat('y').take(300).collect();
        for nm in ["", "é", &long] {
            let n = cs(nm);
            p_ptr_nonnull(
                &format!("e{}.set", nm.len()),
                (api.jsV_setproperty)(J, eobj, n.as_ptr()),
            );
            p_ptr_nonnull(
                &format!("e{}.own", nm.len()),
                (api.jsV_getownproperty)(J, eobj, n.as_ptr()),
            );
        }
        dumpkeys(api, J, ei, 1, "e.own");
        (api.js_pop)(J, 1);

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 40 =============================== */

#[test]
fn cfg40_jsV_newobject_classes() {
    diff("cfg40", |api| unsafe {
        let J = newstate(api, 0);

        /* fetch Object.prototype first: no interpreter runs afterwards */
        let on = cs("Object");
        (api.js_getglobal)(J, on.as_ptr());
        let pn = cs("prototype");
        (api.js_getproperty)(J, -1, pn.as_ptr());
        let protoidx = (api.js_gettop)(J) - 1;
        let proto = (api.js_toobject)(J, protoidx);
        p_ptr_nonnull("Object.prototype", proto);

        let classes: [c_int; 19] = [
            -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 99,
        ];
        for &cl in classes.iter() {
            for withproto in 0..2 {
                let pr = if withproto == 0 { null_mut() } else { proto };
                let o = (api.jsV_newobject)(J, cl, pr);
                let tag = format!("c{}p{}", cl, withproto);
                p_ptr_nonnull(&format!("{}.obj", tag), o);
                (api.js_pushobject)(J, o);
                preds(api, J, (api.js_gettop)(J) - 1, &tag);
                p_int(&format!("{}.top", tag), (api.js_gettop)(J));
                (api.js_pop)(J, 1);
            }
        }
        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 41 =============================== */

unsafe fn cfg41_shapes(api: &Api, J: JS) -> Vec<(String, c_int)> {
    let mut out: Vec<(String, c_int)> = Vec::new();

    /* plain object with own, DONTENUM and inherited properties */
    dostr(
        api,
        J,
        "setup41",
        "var P = { p1: 1, p2: 2 }; function F() {} F.prototype = P; \
         var O = new F(); O.a = 1; O.b = 2;",
    );
    let on = cs("O");
    (api.js_getglobal)(J, on.as_ptr());
    let o = (api.js_gettop)(J) - 1;
    let hidden = cs("hid");
    (api.js_pushnumber)(J, 9.0);
    (api.js_defproperty)(J, o, hidden.as_ptr(), JS_DONTENUM);
    out.push(("plain".into(), o));

    /* flat array */
    let a = mkflat(api, J, 4);
    out.push(("flat".into(), a));

    /* sparse array */
    let s = mksparse(api, J, &[0, 3, 7]);
    out.push(("sparse".into(), s));

    /* array with a named property */
    let m = mkflat(api, J, 2);
    setprop_str(api, J, m, "name", "v");
    out.push(("mixedarr".into(), m));

    /* string object */
    let sv = cs("abc");
    (api.js_newstring)(J, sv.as_ptr());
    out.push(("strobj".into(), (api.js_gettop)(J) - 1));

    /* utf-8 string object */
    let uv = cs("héllo");
    (api.js_newstring)(J, uv.as_ptr());
    out.push(("utf8obj".into(), (api.js_gettop)(J) - 1));

    /* number / boolean wrappers */
    (api.js_newnumber)(J, 3.5);
    out.push(("numobj".into(), (api.js_gettop)(J) - 1));
    (api.js_newboolean)(J, 1);
    out.push(("boolobj".into(), (api.js_gettop)(J) - 1));

    /* primitives: js_toobject converts the slot in place */
    (api.js_pushnumber)(J, 7.0);
    out.push(("primnum".into(), (api.js_gettop)(J) - 1));
    let pv = cs("xy");
    (api.js_pushstring)(J, pv.as_ptr());
    out.push(("primstr".into(), (api.js_gettop)(J) - 1));
    (api.js_pushboolean)(J, 0);
    out.push(("primbool".into(), (api.js_gettop)(J) - 1));

    /* the global object and an empty object */
    (api.js_newobject)(J);
    out.push(("empty".into(), (api.js_gettop)(J) - 1));

    out
}

#[test]
fn cfg41_pushiterator() {
    diff("cfg41", |api| unsafe {
        let J = newstate(api, 0);
        let shapes = cfg41_shapes(api, J);
        for (name, idx) in shapes.iter() {
            for own in 0..2 {
                dumpkeys(api, J, *idx, own, &format!("{}.own{}", name, own));
            }
            dumpv(api, J, *idx, &format!("{}.val", name));
            p_int(&format!("{}.isobject", name), (api.js_isobject)(J, *idx));
        }
        p_int("topN", (api.js_gettop)(J));

        /* iterating past the end keeps returning NULL */
        let a = mkflat(api, J, 2);
        (api.js_pushiterator)(J, a, 1);
        let it = (api.js_gettop)(J) - 1;
        for i in 0..5 {
            let k = (api.js_nextiterator)(J, it);
            if k.is_null() {
                p_int(&format!("past{}", i), -1);
            } else {
                p_str(&format!("past{}", i), k);
            }
        }
        /* deleting during iteration */
        let b = mksparse(api, J, &[0, 1, 2, 3]);
        (api.js_pushiterator)(J, b, 1);
        let it2 = (api.js_gettop)(J) - 1;
        let k1 = (api.js_nextiterator)(J, it2);
        if !k1.is_null() {
            p_str("del.k1", k1);
        }
        (api.js_delindex)(J, b, 1);
        (api.js_delindex)(J, b, 2);
        loop {
            let k = (api.js_nextiterator)(J, it2);
            if k.is_null() {
                break;
            }
            p_str("del.k", k);
        }
        p_int("topN2", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 42 =============================== */

#[test]
fn cfg42_jsV_newiterator() {
    diff("cfg42", |api| unsafe {
        let J = newstate(api, 0);
        let shapes = cfg41_shapes(api, J);
        for (name, idx) in shapes.iter() {
            for own in 0..2 {
                let obj = (api.js_toobject)(J, *idx);
                let it = (api.jsV_newiterator)(J, obj, own);
                p_ptr_nonnull(&format!("{}.own{}.iter", name, own), it);
                let mut n = 0;
                loop {
                    let k = (api.jsV_nextiterator)(J, it);
                    if k.is_null() {
                        break;
                    }
                    p_str(&format!("{}.own{}.key[{}]", name, own, n), k);
                    n += 1;
                    if n > 4000 {
                        break;
                    }
                }
                p_int(&format!("{}.own{}.nkeys", name, own), n);
                /* exhausted iterator keeps returning NULL */
                p_ptr_nonnull(
                    &format!("{}.own{}.end", name, own),
                    (api.jsV_nextiterator)(J, it) as *const c_void,
                );
            }
        }
        p_int("top", (api.js_gettop)(J));

        /* jsV_nextiterator on something that is not an iterator */
        pcall(api, J, "notiter", |api, J| {
            (api.js_newobject)(J);
            let o = (api.js_gettop)(J) - 1;
            let obj = (api.js_toobject)(J, o);
            let s = (api.jsV_nextiterator)(J, obj);
            p_ptr_nonnull("unreached", s as *const c_void);
        });
        pcall(api, J, "notiter_array", |api, J| {
            let a = mkflat(api, J, 2);
            let obj = (api.js_toobject)(J, a);
            let s = (api.jsV_nextiterator)(J, obj);
            p_ptr_nonnull("unreached", s as *const c_void);
        });

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 43 =============================== */

#[test]
fn cfg43_globals() {
    diff("cfg43_nonstrict", |api| unsafe {
        let J = newstate(api, 0);
        let known = ["Object", "Math", "Array", "String", "undefined", "NaN", "Infinity", "print", "nosuch"];

        for n in known.iter() {
            let c = cs(n);
            (api.js_getglobal)(J, c.as_ptr());
            dumpv(api, J, -1, &format!("get.{}", n));
            (api.js_pop)(J, 1);
        }

        /* setglobal on a new name and on an existing one */
        let nn = cs("myglobal");
        (api.js_pushnumber)(J, 17.0);
        (api.js_setglobal)(J, nn.as_ptr());
        p_int("top_after_setglobal", (api.js_gettop)(J));
        (api.js_getglobal)(J, nn.as_ptr());
        dumpv(api, J, -1, "myglobal");
        (api.js_pop)(J, 1);

        let mn = cs("Math");
        (api.js_pushnumber)(J, 1.0);
        (api.js_setglobal)(J, mn.as_ptr());
        (api.js_getglobal)(J, mn.as_ptr());
        dumpv(api, J, -1, "Math_after_set");
        (api.js_pop)(J, 1);

        let un = cs("undefined");
        (api.js_pushnumber)(J, 5.0);
        (api.js_setglobal)(J, un.as_ptr());
        (api.js_getglobal)(J, un.as_ptr());
        dumpv(api, J, -1, "undefined_after_set");
        (api.js_pop)(J, 1);

        /* defglobal with every attribute combination */
        for atts in 0..=7 {
            let name = format!("g{}", atts);
            let c = cs(&name);
            (api.js_pushnumber)(J, atts as f64);
            (api.js_defglobal)(J, c.as_ptr(), atts);
            p_int(&format!("{}.top", name), (api.js_gettop)(J));
            (api.js_getglobal)(J, c.as_ptr());
            dumpv(api, J, -1, &format!("{}.get", name));
            (api.js_pop)(J, 1);

            /* set it */
            (api.js_pushnumber)(J, 100.0 + atts as f64);
            (api.js_setglobal)(J, c.as_ptr());
            (api.js_getglobal)(J, c.as_ptr());
            dumpv(api, J, -1, &format!("{}.get2", name));
            (api.js_pop)(J, 1);

            /* delete it */
            (api.js_delglobal)(J, c.as_ptr());
            (api.js_getglobal)(J, c.as_ptr());
            dumpv(api, J, -1, &format!("{}.get3", name));
            (api.js_pop)(J, 1);
        }

        /* enumerate the global object's own enumerable keys */
        (api.js_pushglobal)(J);
        let g = (api.js_gettop)(J) - 1;
        dumpkeys(api, J, g, 1, "G.own");
        (api.js_pop)(J, 1);

        /* delete a builtin, then read it back */
        let on = cs("Object");
        (api.js_delglobal)(J, on.as_ptr());
        (api.js_getglobal)(J, on.as_ptr());
        dumpv(api, J, -1, "Object_after_del");
        (api.js_pop)(J, 1);

        /* delete a name that does not exist */
        let zn = cs("nosuch2");
        (api.js_delglobal)(J, zn.as_ptr());
        (api.js_getglobal)(J, zn.as_ptr());
        dumpv(api, J, -1, "nosuch2");
        (api.js_pop)(J, 1);

        /* empty name */
        let en = cs("");
        (api.js_pushnumber)(J, 3.0);
        (api.js_setglobal)(J, en.as_ptr());
        (api.js_getglobal)(J, en.as_ptr());
        dumpv(api, J, -1, "emptyname");
        (api.js_pop)(J, 1);

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });

    diff("cfg43_strict", |api| unsafe {
        let J = newstate(api, JS_STRICT);
        for atts in 0..=7 {
            let name = format!("s{}", atts);
            pcall(api, J, &name, move |api, J| {
                let c = cs(&format!("s{}", atts));
                (api.js_pushnumber)(J, atts as f64);
                (api.js_defglobal)(J, c.as_ptr(), atts);
                (api.js_getglobal)(J, c.as_ptr());
                dumpv(api, J, -1, "def");
                (api.js_pop)(J, 1);
                (api.js_pushnumber)(J, 9.0);
                (api.js_setglobal)(J, c.as_ptr());
                (api.js_getglobal)(J, c.as_ptr());
                dumpv(api, J, -1, "set");
                (api.js_pop)(J, 1);
                (api.js_delglobal)(J, c.as_ptr());
                (api.js_getglobal)(J, c.as_ptr());
                dumpv(api, J, -1, "del");
                (api.js_pop)(J, 1);
            });
        }
        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 44 =============================== */

#[test]
fn cfg44_registry() {
    diff("cfg44", |api| unsafe {
        let J = newstate(api, 0);

        let k1 = cs("key1");
        (api.js_pushnumber)(J, 11.0);
        (api.js_setregistry)(J, k1.as_ptr());
        p_int("top_after_set", (api.js_gettop)(J));
        (api.js_getregistry)(J, k1.as_ptr());
        dumpv(api, J, -1, "key1");
        (api.js_pop)(J, 1);

        /* overwrite with a different type */
        let sv = cs("hello");
        (api.js_pushstring)(J, sv.as_ptr());
        (api.js_setregistry)(J, k1.as_ptr());
        (api.js_getregistry)(J, k1.as_ptr());
        dumpv(api, J, -1, "key1b");
        (api.js_pop)(J, 1);

        /* object value */
        (api.js_newobject)(J);
        let o = (api.js_gettop)(J) - 1;
        setprop_num(api, J, o, "z", 3.0);
        (api.js_setregistry)(J, k1.as_ptr());
        (api.js_getregistry)(J, k1.as_ptr());
        dumpv(api, J, -1, "key1c");
        dumpprop(api, J, (api.js_gettop)(J) - 1, "z", "key1c");
        (api.js_pop)(J, 1);

        /* missing key */
        let miss = cs("nokey");
        (api.js_getregistry)(J, miss.as_ptr());
        dumpv(api, J, -1, "nokey");
        (api.js_pop)(J, 1);

        /* empty string key */
        let ek = cs("");
        (api.js_pushnumber)(J, 42.0);
        (api.js_setregistry)(J, ek.as_ptr());
        (api.js_getregistry)(J, ek.as_ptr());
        dumpv(api, J, -1, "emptykey");
        (api.js_pop)(J, 1);

        /* index-like key and utf-8 key */
        for k in ["0", "123", "ключ", "a b"] {
            let c = cs(k);
            (api.js_pushstring)(J, cs(&format!("v-{}", k)).as_ptr());
            (api.js_setregistry)(J, c.as_ptr());
            (api.js_getregistry)(J, c.as_ptr());
            dumpv(api, J, -1, &format!("reg.{}", k));
            (api.js_pop)(J, 1);
        }

        /* delete */
        (api.js_delregistry)(J, k1.as_ptr());
        (api.js_getregistry)(J, k1.as_ptr());
        dumpv(api, J, -1, "key1_after_del");
        (api.js_pop)(J, 1);

        /* delete twice / delete missing */
        (api.js_delregistry)(J, k1.as_ptr());
        (api.js_delregistry)(J, miss.as_ptr());
        (api.js_getregistry)(J, miss.as_ptr());
        dumpv(api, J, -1, "nokey2");
        (api.js_pop)(J, 1);

        (api.js_delregistry)(J, ek.as_ptr());
        (api.js_getregistry)(J, ek.as_ptr());
        dumpv(api, J, -1, "emptykey_after_del");
        (api.js_pop)(J, 1);

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 45 =============================== */

#[test]
fn cfg45_ref_unref() {
    diff("cfg45", |api| unsafe {
        let J = newstate(api, 0);

        /* primitives: the ref string is deterministic */
        (api.js_pushnumber)(J, 42.0);
        let r1 = (api.js_ref)(J);
        p_str("ref.num", r1);
        p_int("top_after_ref", (api.js_gettop)(J));
        (api.js_getregistry)(J, r1);
        dumpv(api, J, -1, "num.readback");
        (api.js_pop)(J, 1);

        let sv = cs("a string value");
        (api.js_pushstring)(J, sv.as_ptr());
        let r2 = (api.js_ref)(J);
        p_str("ref.str", r2);
        (api.js_getregistry)(J, r2);
        dumpv(api, J, -1, "str.readback");
        (api.js_pop)(J, 1);

        (api.js_pushboolean)(J, 1);
        let r3 = (api.js_ref)(J);
        p_str("ref.true", r3);
        (api.js_getregistry)(J, r3);
        dumpv(api, J, -1, "true.readback");
        (api.js_pop)(J, 1);

        (api.js_pushboolean)(J, 0);
        let r4 = (api.js_ref)(J);
        p_str("ref.false", r4);
        (api.js_getregistry)(J, r4);
        dumpv(api, J, -1, "false.readback");
        (api.js_pop)(J, 1);

        (api.js_pushundefined)(J);
        let r5 = (api.js_ref)(J);
        p_str("ref.undefined", r5);
        (api.js_getregistry)(J, r5);
        dumpv(api, J, -1, "undefined.readback");
        (api.js_pop)(J, 1);

        (api.js_pushnull)(J);
        let r6 = (api.js_ref)(J);
        p_str("ref.null", r6);
        (api.js_getregistry)(J, r6);
        dumpv(api, J, -1, "null.readback");
        (api.js_pop)(J, 1);

        /* object: the ref string is the object address, so only its
         * non-nullness is printed */
        (api.js_newobject)(J);
        let o = (api.js_gettop)(J) - 1;
        setprop_num(api, J, o, "marker", 5.0);
        let r7 = (api.js_ref)(J);
        p_ptr_nonnull("ref.obj", r7 as *const c_void);
        (api.js_getregistry)(J, r7);
        dumpv(api, J, -1, "obj.readback");
        dumpprop(api, J, (api.js_gettop)(J) - 1, "marker", "obj.readback");
        (api.js_pop)(J, 1);

        /* an array and a second number to advance nextref */
        let a = mkflat(api, J, 3);
        let _ = a;
        let r8 = (api.js_ref)(J);
        p_ptr_nonnull("ref.arr", r8 as *const c_void);
        (api.js_getregistry)(J, r8);
        dumpv(api, J, -1, "arr.readback");
        (api.js_pop)(J, 1);

        (api.js_pushnumber)(J, -1.5);
        let r9 = (api.js_ref)(J);
        p_str("ref.num2", r9);
        (api.js_getregistry)(J, r9);
        dumpv(api, J, -1, "num2.readback");
        (api.js_pop)(J, 1);

        /* two refs to the same object share a key */
        (api.js_getregistry)(J, r7);
        let r10 = (api.js_ref)(J);
        p_int("same_obj_ref", if r10 == r7 { 1 } else { 0 });

        /* unref everything and read back */
        for (name, r) in [
            ("num", r1),
            ("str", r2),
            ("true", r3),
            ("false", r4),
            ("undefined", r5),
            ("null", r6),
            ("obj", r7),
            ("arr", r8),
            ("num2", r9),
        ] {
            (api.js_unref)(J, r);
            (api.js_getregistry)(J, r);
            dumpv(api, J, -1, &format!("{}.after_unref", name));
            (api.js_pop)(J, 1);
            /* unref twice */
            (api.js_unref)(J, r);
            (api.js_getregistry)(J, r);
            dumpv(api, J, -1, &format!("{}.after_unref2", name));
            (api.js_pop)(J, 1);
        }

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 51 =============================== */

unsafe fn cfg51_dump(api: &Api, J: JS, idx: c_int, label: &str) {
    preds(api, J, idx, label);
    p_int(&format!("{}.toboolean", label), (api.js_toboolean)(J, idx));
    p_num(&format!("{}.tonumber", label), (api.js_tonumber)(J, idx));
    (api.js_copy)(J, idx);
    p_str(&format!("{}.tostring", label), (api.js_tostring)(J, -1));
    (api.js_pop)(J, 1);
    dumpprop(api, J, idx, "length", label);
    dumpprop(api, J, idx, "valueOf", label);
    dumpkeys(api, J, idx, 1, &format!("{}.own", label));
}

#[test]
fn cfg51_wrapper_objects() {
    diff("cfg51", |api| unsafe {
        let J = newstate(api, 0);

        for b in [0, 1] {
            (api.js_newboolean)(J, b);
            let i = (api.js_gettop)(J) - 1;
            cfg51_dump(api, J, i, &format!("bool{}", b));
            (api.js_pop)(J, 1);
        }

        let nums: [f64; 7] = [
            0.0,
            -0.0,
            3.5,
            -17.0,
            f64::NAN,
            f64::INFINITY,
            1e21,
        ];
        for (k, n) in nums.iter().enumerate() {
            (api.js_newnumber)(J, *n);
            let i = (api.js_gettop)(J) - 1;
            cfg51_dump(api, J, i, &format!("num{}", k));
            (api.js_pop)(J, 1);
        }

        for (k, s) in ["", "hi", "0", "héllo", "12.5"].iter().enumerate() {
            let c = cs(s);
            (api.js_newstring)(J, c.as_ptr());
            let i = (api.js_gettop)(J) - 1;
            cfg51_dump(api, J, i, &format!("str{}", k));
            dumpidx(api, J, i, 0, &format!("str{}", k));
            dumpidx(api, J, i, 1, &format!("str{}", k));
            (api.js_pop)(J, 1);
        }

        /* primitives for contrast */
        (api.js_pushboolean)(J, 1);
        cfg51_dump(api, J, (api.js_gettop)(J) - 1, "primbool");
        (api.js_pop)(J, 1);
        (api.js_pushnumber)(J, 3.5);
        cfg51_dump(api, J, (api.js_gettop)(J) - 1, "primnum");
        (api.js_pop)(J, 1);
        let pv = cs("hi");
        (api.js_pushstring)(J, pv.as_ptr());
        cfg51_dump(api, J, (api.js_gettop)(J) - 1, "primstr");
        (api.js_pop)(J, 1);

        /* a Date object built from script, plus a plain object */
        dostr(api, J, "date", "var D = new Date(0); var PO = {};");
        let dn = cs("D");
        (api.js_getglobal)(J, dn.as_ptr());
        let d = (api.js_gettop)(J) - 1;
        preds(api, J, d, "date");
        (api.js_pop)(J, 1);
        let pn = cs("PO");
        (api.js_getglobal)(J, pn.as_ptr());
        preds(api, J, (api.js_gettop)(J) - 1, "plain");
        (api.js_pop)(J, 1);

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

/* =============================== row 52 =============================== */

#[test]
fn cfg52_errors() {
    diff("cfg52", |api| unsafe {
        let J = newstate(api, 0);

        let msgs = ["boom", "", "ünïcödé ✓"];
        for (mi, m) in msgs.iter().enumerate() {
            let cm = cs(m);
            let ctors: [(&str, unsafe extern "C" fn(JS, *const c_char)); 7] = [
                ("error", api.js_newerror),
                ("evalerror", api.js_newevalerror),
                ("rangeerror", api.js_newrangeerror),
                ("referenceerror", api.js_newreferenceerror),
                ("syntaxerror", api.js_newsyntaxerror),
                ("typeerror", api.js_newtypeerror),
                ("urierror", api.js_newurierror),
            ];
            for (name, f) in ctors.iter() {
                let tag = format!("{}.m{}", name, mi);
                f(J, cm.as_ptr());
                let e = (api.js_gettop)(J) - 1;
                preds(api, J, e, &tag);
                (api.js_copy)(J, e);
                p_str(&format!("{}.tostring", tag), (api.js_tostring)(J, -1));
                (api.js_pop)(J, 1);
                for pn in ["name", "message", "stackTrace", "constructor"] {
                    dumpprop(api, J, e, pn, &tag);
                }
                dumpkeys(api, J, e, 1, &format!("{}.own", tag));
                dumpkeys(api, J, e, 0, &format!("{}.all", tag));
                p_int(&format!("{}.top", tag), (api.js_gettop)(J));
                (api.js_pop)(J, 1);
            }
        }

        /* the "stack" accessor (built from toString + stackTrace) */
        let cm = cs("stacky");
        (api.js_newerror)(J, cm.as_ptr());
        let e = (api.js_gettop)(J) - 1;
        dumpprop(api, J, e, "stack", "stackprop");
        (api.js_pop)(J, 1);

        /* instanceof / prototype identity checks through script */
        dostr(api, J, "setup52", "var results = [];");
        let cm2 = cs("m2");
        (api.js_newtypeerror)(J, cm2.as_ptr());
        let te = (api.js_gettop)(J) - 1;
        let gn = cs("theError");
        (api.js_copy)(J, te);
        (api.js_setglobal)(J, gn.as_ptr());
        dostr(
            api,
            J,
            "check52",
            "results = [theError instanceof TypeError, \
                        theError instanceof Error, \
                        theError instanceof RangeError, \
                        String(theError), theError.name, theError.message];",
        );
        let rn = cs("results");
        (api.js_getglobal)(J, rn.as_ptr());
        let r = (api.js_gettop)(J) - 1;
        dumparray(api, J, r, "results", &[6]);
        (api.js_pop)(J, 1);
        (api.js_pop)(J, 1);

        /* thrown and caught through pcall */
        pcall(api, J, "throw_typeerror", |api, J| {
            let m = cs("thrown by hand");
            (api.js_newtypeerror)(J, m.as_ptr());
            (api.js_throw)(J);
        });

        p_int("topN", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

