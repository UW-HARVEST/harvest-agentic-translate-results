//! Phase C — out-of-memory branches, reached through the public allocator
//! hooks (`json_set_alloc_funcs2`). These are the only way to exercise the
//! `json_error_out_of_memory` / `!ptr` failure paths that the C guards
//! everywhere.
//!
//! ERRORS rows 116, 119-121 (allocator side), 138, 145, 220 and every
//! `if (!x) return NULL/-1` after a `jsonp_malloc` / `jsonp_realloc`.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicIsize, Ordering};

unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
}

/// Independent budgets so the two libraries cannot interfere with each other.
static C_BUDGET: AtomicIsize = AtomicIsize::new(isize::MAX);
static R_BUDGET: AtomicIsize = AtomicIsize::new(isize::MAX);
static C_CALLS: AtomicIsize = AtomicIsize::new(0);
static R_CALLS: AtomicIsize = AtomicIsize::new(0);

macro_rules! budget_alloc {
    ($m:ident, $re:ident, $f:ident, $budget:ident, $calls:ident) => {
        unsafe extern "C" fn $m(n: usize) -> *mut c_void {
            $calls.fetch_add(1, Ordering::SeqCst);
            if $budget.fetch_sub(1, Ordering::SeqCst) <= 0 {
                return std::ptr::null_mut();
            }
            unsafe { libc_malloc(n) }
        }
        unsafe extern "C" fn $re(p: *mut c_void, n: usize) -> *mut c_void {
            $calls.fetch_add(1, Ordering::SeqCst);
            if $budget.fetch_sub(1, Ordering::SeqCst) <= 0 {
                return std::ptr::null_mut();
            }
            unsafe { libc_realloc(p, n) }
        }
        unsafe extern "C" fn $f(p: *mut c_void) {
            unsafe { libc_free(p) }
        }
    };
}

budget_alloc!(c_malloc, c_realloc, c_free, C_BUDGET, C_CALLS);
budget_alloc!(r_malloc, r_realloc, r_free, R_BUDGET, R_CALLS);

struct Hooks {
    cm: Option<MallocFn>,
    cr: Option<ReallocFn>,
    cf: Option<FreeFn>,
    rm: Option<MallocFn>,
    rr: Option<ReallocFn>,
    rf: Option<FreeFn>,
}

unsafe fn install() -> Hooks {
    unsafe {
        let mut cm = None;
        let mut cr = None;
        let mut cf = None;
        let mut rm = None;
        let mut rr = None;
        let mut rf = None;
        (c().json_get_alloc_funcs2)(&mut cm, &mut cr, &mut cf);
        (r().json_get_alloc_funcs2)(&mut rm, &mut rr, &mut rf);
        (c().json_set_alloc_funcs2)(Some(c_malloc), Some(c_realloc), Some(c_free));
        (r().json_set_alloc_funcs2)(Some(r_malloc), Some(r_realloc), Some(r_free));
        Hooks { cm, cr, cf, rm, rr, rf }
    }
}

unsafe fn restore(h: &Hooks) {
    unsafe {
        C_BUDGET.store(isize::MAX, Ordering::SeqCst);
        R_BUDGET.store(isize::MAX, Ordering::SeqCst);
        (c().json_set_alloc_funcs2)(h.cm, h.cr, h.cf);
        (r().json_set_alloc_funcs2)(h.rm, h.rr, h.rf);
    }
}

fn set_budget(n: isize) {
    C_BUDGET.store(n, Ordering::SeqCst);
    R_BUDGET.store(n, Ordering::SeqCst);
    C_CALLS.store(0, Ordering::SeqCst);
    R_CALLS.store(0, Ordering::SeqCst);
}

fn calls() -> (isize, isize) {
    (C_CALLS.load(Ordering::SeqCst), R_CALLS.load(Ordering::SeqCst))
}

/// Runs `op` for allocation budgets `0..=max` and asserts C and Rust agree on
/// every observable, including how many allocations they needed.
fn sweep<F>(what: &str, max: isize, mut op: F)
where
    F: FnMut(&'static Api) -> String,
{
    for budget in 0..=max {
        set_budget(budget);
        let cs_ = op(c());
        let rs_ = op(r());
        let (cc, rc) = calls();
        assert_eq!(
            cs_, rs_,
            "{what}: budget={budget}\n  C   = {cs_}\n  RUST= {rs_}"
        );
        assert_eq!(
            cc, rc,
            "{what}: budget={budget} allocation count C={cc} RUST={rc}"
        );
    }
}

/* ---------------- constructors ---------------- */

#[test]
fn oom_constructors() {
    let _g = dtoa_guard();
    unsafe {
        let h = install();

        sweep("json_object()", 6, |api| {
            let p = (api.json_object)();
            let s = format!("{}", p.is_null());
            if !p.is_null() {
                decref(api, p);
            }
            s
        });

        sweep("json_array()", 6, |api| {
            let p = (api.json_array)();
            let s = format!("{}", p.is_null());
            if !p.is_null() {
                decref(api, p);
            }
            s
        });

        sweep("json_string()", 6, |api| {
            let v = cs("some string value");
            let p = (api.json_string)(v.as_ptr());
            let s = format!("{}", p.is_null());
            if !p.is_null() {
                decref(api, p);
            }
            s
        });

        sweep("json_stringn_nocheck(len 0)", 6, |api| {
            let p = (api.json_stringn_nocheck)(c"".as_ptr(), 0);
            let s = format!("{}", p.is_null());
            if !p.is_null() {
                decref(api, p);
            }
            s
        });

        sweep("json_integer()", 4, |api| {
            let p = (api.json_integer)(42);
            let s = format!("{}", p.is_null());
            if !p.is_null() {
                decref(api, p);
            }
            s
        });

        sweep("json_real()", 4, |api| {
            let p = (api.json_real)(1.5);
            let s = format!("{}", p.is_null());
            if !p.is_null() {
                decref(api, p);
            }
            s
        });

        sweep("jsonp_malloc/strndup", 4, |api| {
            let a = (api.jsonp_malloc)(32);
            let b = (api.jsonp_strndup)(c"abc".as_ptr(), 3);
            let s = format!("{} {}", a.is_null(), b.is_null());
            (api.jsonp_free)(a);
            (api.jsonp_free)(b as *mut c_void);
            s
        });

        sweep("strbuffer_init + append", 8, |api| {
            let mut sb = StrBuffer::default();
            let rc = (api.strbuffer_init)(&mut sb);
            let mut out = format!("init={rc}");
            if rc == 0 {
                let data = [b'x' as c_char; 200];
                let a1 = (api.strbuffer_append_bytes)(&mut sb, data.as_ptr(), 200);
                let a2 = (api.strbuffer_append_bytes)(&mut sb, data.as_ptr(), 200);
                out.push_str(&format!(" a1={a1} a2={a2} len={} size={}", sb.length, sb.size));
                (api.strbuffer_close)(&mut sb);
            }
            out
        });

        sweep("hashtable_init", 4, |api| {
            let mut ht = Box::new(Hashtable::default());
            let rc = (api.hashtable_init)(&mut *ht);
            let out = format!("init={rc}");
            if rc == 0 {
                (api.hashtable_close)(&mut *ht);
            }
            out
        });

        restore(&h);
    }
}

/* ---------------- growth paths ---------------- */

#[test]
fn oom_array_growth_and_object_rehash() {
    let _g = dtoa_guard();
    unsafe {
        let h = install();

        // json_array_grow / json_array_append_new: the 9th append reallocs.
        sweep("array append past size 8", 40, |api| {
            let a = (api.json_array)();
            if a.is_null() {
                return "array=NULL".into();
            }
            let mut rcs = String::new();
            for i in 0..14 {
                let v = (api.json_integer)(i);
                rcs.push_str(&format!("{},", (api.json_array_append_new)(a, v)));
            }
            let out = format!("size={} rcs={rcs}", (api.json_array_size)(a));
            decref(api, a);
            out
        });

        // json_array_extend also grows
        sweep("array extend", 60, |api| {
            let a = (api.json_array)();
            let b = (api.json_array)();
            if a.is_null() || b.is_null() {
                return format!("a={} b={}", a.is_null(), b.is_null());
            }
            for i in 0..10 {
                (api.json_array_append_new)(b, (api.json_integer)(i));
            }
            let rc = (api.json_array_extend)(a, b);
            let out = format!("rc={rc} size={}", (api.json_array_size)(a));
            decref(api, a);
            decref(api, b);
            out
        });

        // json_object_set: init_pair malloc + hashtable_do_rehash at load 1
        sweep("object set past 8 keys (rehash)", 60, |api| {
            let o = (api.json_object)();
            if o.is_null() {
                return "object=NULL".into();
            }
            let mut rcs = String::new();
            for i in 0..14 {
                let k = cs(&format!("key{i:02}"));
                let v = (api.json_integer)(i);
                rcs.push_str(&format!("{},", (api.json_object_set_new_nocheck)(o, k.as_ptr(), v)));
            }
            let out = format!("size={} rcs={rcs}", (api.json_object_size)(o));
            decref(api, o);
            out
        });

        restore(&h);
    }
}

/* ---------------- dumps / loads / pack ---------------- */

#[test]
fn oom_dumps_and_loads() {
    let _g = dtoa_guard();
    unsafe {
        // Warm both dtoa freelists identically BEFORE installing the budgeted
        // allocator, so the cached Bigints don't skew the allocation counts.
        for api in both() {
            let j = (api.json_real)(1.0 / 3.0);
            let d = (api.json_dumps)(j, JSON_ENCODE_ANY);
            (api.jsonp_free)(d as *mut c_void);
            decref(api, j);
        }
        let h = install();

        // ERRORS 138: json_dumps returns NULL when the pipeline OOMs.
        sweep("json_dumps of a small object", 60, |api| {
            let o = (api.json_object)();
            if o.is_null() {
                return "object=NULL".into();
            }
            (api.json_object_set_new_nocheck)(o, cs("a").as_ptr(), (api.json_integer)(1));
            (api.json_object_set_new_nocheck)(o, cs("b").as_ptr(), (api.json_string)(cs("xx").as_ptr()));
            let d = (api.json_dumps)(o, 0);
            let out = if d.is_null() {
                "dump=NULL".to_string()
            } else {
                let s = std::ffi::CStr::from_ptr(d).to_bytes().to_vec();
                (api.jsonp_free)(d as *mut c_void);
                format!("dump={}", String::from_utf8_lossy(&s))
            };
            decref(api, o);
            out
        });

        // ERRORS 139: json_dumpb returns 0 on failure
        sweep("json_dumpb", 40, |api| {
            let a = (api.json_array)();
            if a.is_null() {
                return "array=NULL".into();
            }
            for i in 0..3 {
                (api.json_array_append_new)(a, (api.json_integer)(i));
            }
            let mut buf = [0i8; 64];
            let n = (api.json_dumpb)(a, buf.as_mut_ptr(), 64, 0);
            let out = format!("n={n} buf={:?}", &buf[..n.min(64)]);
            decref(api, a);
            out
        });

        // ERRORS 145: SORT_KEYS allocates the key array
        sweep("json_dumps SORT_KEYS", 80, |api| {
            let o = (api.json_object)();
            if o.is_null() {
                return "object=NULL".into();
            }
            for i in 0..5 {
                let k = cs(&format!("k{i}"));
                (api.json_object_set_new_nocheck)(o, k.as_ptr(), (api.json_integer)(i));
            }
            let d = (api.json_dumps)(o, JSON_SORT_KEYS);
            let out = if d.is_null() {
                "dump=NULL".to_string()
            } else {
                let s = std::ffi::CStr::from_ptr(d).to_bytes().to_vec();
                (api.jsonp_free)(d as *mut c_void);
                format!("dump={}", String::from_utf8_lossy(&s))
            };
            decref(api, o);
            out
        });

        // json_loads under a shrinking budget
        for src in [
            r#"{"a":1}"#,
            r#"[1,2,3]"#,
            r#"{"a":"str","b":[1,2],"c":{"d":true}}"#,
            r#"["\u00e9\u20ac"]"#,
        ] {
            let s = cs(src);
            sweep(&format!("json_loads({src:?})"), 70, |api| {
                let mut err = JsonError::default();
                let j = (api.json_loads)(s.as_ptr(), 0, &mut err);
                let out = if j.is_null() {
                    format!("NULL err={:?}", err.snapshot())
                } else {
                    let sh = shape(api, j);
                    format!("ok {sh} err={:?}", err.snapshot())
                };
                decref(api, j);
                out
            });
        }

        restore(&h);
    }
}

/* ---------------- pack / unpack OOM (ERRORS 220 etc.) ---------------- */

#[test]
fn oom_pack_and_unpack() {
    let _g = dtoa_guard();
    unsafe {
        for api in both() {
            let j = (api.json_real)(1.0 / 3.0);
            let d = (api.json_dumps)(j, JSON_ENCODE_ANY);
            (api.jsonp_free)(d as *mut c_void);
            decref(api, j);
        }
        let h = install();

        let fmt = cs("{s:i,s:s,s:[i,i]}");
        let k1 = cs("i");
        let k2 = cs("s");
        let k3 = cs("a");
        let sv = cs("value");
        sweep("json_pack_ex nested", 80, |api| {
            let mut err = JsonError::default();
            let p = (api.json_pack_ex)(
                &mut err, 0, fmt.as_ptr(),
                k1.as_ptr(), 1i32,
                k2.as_ptr(), sv.as_ptr(),
                k3.as_ptr(), 2i32, 3i32,
            );
            let out = if p.is_null() {
                format!("NULL err={:?}", err.snapshot())
            } else {
                let sh = shape(api, p);
                format!("ok {sh} err={:?}", err.snapshot())
            };
            decref(api, p);
            out
        });

        // ERRORS 220: pack_integer OOM => json_error_out_of_memory
        let ifmt = cs("i");
        sweep("json_pack_ex(\"i\") OOM", 4, |api| {
            let mut err = JsonError::default();
            let p = (api.json_pack_ex)(&mut err, 0, ifmt.as_ptr(), 7i32);
            let out = if p.is_null() {
                format!("NULL code={} text={:?}", err.code(), err.text_str())
            } else {
                decref(api, p);
                "ok".to_string()
            };
            out
        });

        // read_string's strbuffer OOM path (s+ concatenation)
        let cfmt = cs("s++");
        let a = cs("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let b = cs("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let d = cs("cccccccccccccccccccccccccccccc");
        sweep("json_pack_ex(\"s++\") OOM", 20, |api| {
            let mut err = JsonError::default();
            let p = (api.json_pack_ex)(
                &mut err, 0, cfmt.as_ptr(), a.as_ptr(), b.as_ptr(), d.as_ptr(),
            );
            let out = if p.is_null() {
                format!("NULL code={} text={:?}", err.code(), err.text_str())
            } else {
                let sh = shape(api, p);
                decref(api, p);
                format!("ok {sh}")
            };
            out
        });

        // unpack_object's key_set hashtable OOM (ERRORS: json_error_out_of_memory)
        let ufmt = cs("{s:i!}");
        let uk = cs("a");
        // Build the roots BEFORE constraining the budget.
        C_BUDGET.store(isize::MAX, Ordering::SeqCst);
        R_BUDGET.store(isize::MAX, Ordering::SeqCst);
        let croot = {
            let s = cs(r#"{"a":1,"b":2}"#);
            (c().json_loads)(s.as_ptr(), 0, std::ptr::null_mut())
        };
        let rroot = {
            let s = cs(r#"{"a":1,"b":2}"#);
            (r().json_loads)(s.as_ptr(), 0, std::ptr::null_mut())
        };
        assert!(!croot.is_null() && !rroot.is_null());
        for budget in 0..20isize {
            set_budget(budget);
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let mut ci: c_int = 0;
            let mut ri: c_int = 0;
            let crc = (c().json_unpack_ex)(croot, &mut ce, 0, ufmt.as_ptr(), uk.as_ptr(), &mut ci);
            let rrc = (r().json_unpack_ex)(rroot, &mut re, 0, ufmt.as_ptr(), uk.as_ptr(), &mut ri);
            assert_eq!(crc, rrc, "unpack OOM budget={budget} rc");
            assert_eq!(ce.snapshot(), re.snapshot(), "unpack OOM budget={budget} error");
            assert_eq!(ci, ri, "unpack OOM budget={budget} output");
            assert_eq!(calls().0, calls().1, "unpack OOM budget={budget} alloc count");
        }
        C_BUDGET.store(isize::MAX, Ordering::SeqCst);
        R_BUDGET.store(isize::MAX, Ordering::SeqCst);
        decref(c(), croot);
        decref(r(), rroot);

        restore(&h);
    }
}

/* ---------------- deep copy / update under OOM ---------------- */

#[test]
fn oom_copy_and_update() {
    let _g = dtoa_guard();
    unsafe {
        let h = install();

        sweep("json_deep_copy", 90, |api| {
            // Build the source with an unconstrained budget, then constrain.
            let saved_c = C_BUDGET.swap(isize::MAX, Ordering::SeqCst);
            let saved_r = R_BUDGET.swap(isize::MAX, Ordering::SeqCst);
            let src = cs(r#"{"a":[1,2,{"b":"s"}],"c":{"d":[true,null]}}"#);
            let root = (api.json_loads)(src.as_ptr(), 0, std::ptr::null_mut());
            assert!(!root.is_null());
            C_BUDGET.store(saved_c, Ordering::SeqCst);
            R_BUDGET.store(saved_r, Ordering::SeqCst);

            let cp = (api.json_deep_copy)(root);
            let out = if cp.is_null() {
                "NULL".to_string()
            } else {
                let s = shape(api, cp);
                decref(api, cp);
                s
            };
            let saved_c = C_BUDGET.swap(isize::MAX, Ordering::SeqCst);
            let saved_r = R_BUDGET.swap(isize::MAX, Ordering::SeqCst);
            decref(api, root);
            C_BUDGET.store(saved_c, Ordering::SeqCst);
            R_BUDGET.store(saved_r, Ordering::SeqCst);
            out
        });

        sweep("json_object_update_recursive", 90, |api| {
            let saved_c = C_BUDGET.swap(isize::MAX, Ordering::SeqCst);
            let saved_r = R_BUDGET.swap(isize::MAX, Ordering::SeqCst);
            let s1 = cs(r#"{"a":{"x":1},"b":2}"#);
            let s2 = cs(r#"{"a":{"y":3},"c":4}"#);
            let dst = (api.json_loads)(s1.as_ptr(), 0, std::ptr::null_mut());
            let other = (api.json_loads)(s2.as_ptr(), 0, std::ptr::null_mut());
            assert!(!dst.is_null() && !other.is_null());
            C_BUDGET.store(saved_c, Ordering::SeqCst);
            R_BUDGET.store(saved_r, Ordering::SeqCst);

            let rc = (api.json_object_update_recursive)(dst, other);
            let out = format!("rc={rc} dst={}", shape(api, dst));

            let saved_c = C_BUDGET.swap(isize::MAX, Ordering::SeqCst);
            let saved_r = R_BUDGET.swap(isize::MAX, Ordering::SeqCst);
            decref(api, dst);
            decref(api, other);
            C_BUDGET.store(saved_c, Ordering::SeqCst);
            R_BUDGET.store(saved_r, Ordering::SeqCst);
            out
        });

        restore(&h);
    }
}
