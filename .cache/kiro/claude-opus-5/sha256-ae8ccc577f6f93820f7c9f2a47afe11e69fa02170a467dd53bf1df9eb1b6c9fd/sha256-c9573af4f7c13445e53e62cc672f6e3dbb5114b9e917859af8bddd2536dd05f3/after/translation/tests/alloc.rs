//! Phase B section F + the out-of-memory rows of ERRORS.md.
//!
//! Both libraries get their own instrumented allocator (installed through their
//! own `json_set_alloc_funcs` / `json_set_alloc_funcs2` exports).  The
//! allocators are thin wrappers over libc `malloc`/`realloc`/`free`, so
//! switching allocator sets mid-process stays safe, and they record the exact
//! sequence of `(op, size)` calls so the two libraries' allocation behaviour is
//! compared, not just their return values.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

const OP_MALLOC: u8 = b'm';
const OP_REALLOC: u8 = b'r';
const OP_FREE: u8 = b'f';

struct Slot {
    log: Vec<(u8, usize)>,
    n_malloc: usize,
    n_realloc: usize,
    n_free: usize,
    /// When > 0, the N-th malloc/realloc returns NULL.
    fail_at: usize,
    count: usize,
    recording: bool,
}

const EMPTY: Slot = Slot {
    log: Vec::new(),
    n_malloc: 0,
    n_realloc: 0,
    n_free: 0,
    fail_at: 0,
    count: 0,
    recording: false,
};

static mut SLOTS: [Slot; 2] = [EMPTY, EMPTY];

unsafe fn slot(i: usize) -> &'static mut Slot {
    unsafe { &mut (*std::ptr::addr_of_mut!(SLOTS))[i] }
}

unsafe fn do_malloc(i: usize, n: usize) -> *mut c_void {
    unsafe {
        let s = slot(i);
        if s.recording {
            s.log.push((OP_MALLOC, n));
            s.n_malloc += 1;
            s.count += 1;
            if s.fail_at != 0 && s.count == s.fail_at {
                return std::ptr::null_mut();
            }
        }
        (libc().malloc)(n)
    }
}

unsafe fn do_realloc(i: usize, p: *mut c_void, n: usize) -> *mut c_void {
    unsafe {
        let s = slot(i);
        if s.recording {
            s.log.push((OP_REALLOC, n));
            s.n_realloc += 1;
            s.count += 1;
            if s.fail_at != 0 && s.count == s.fail_at {
                return std::ptr::null_mut();
            }
        }
        (libc().realloc)(p, n)
    }
}

unsafe fn do_free(i: usize, p: *mut c_void) {
    unsafe {
        let s = slot(i);
        if s.recording {
            s.log.push((OP_FREE, 0));
            s.n_free += 1;
        }
        (libc().free)(p)
    }
}

unsafe extern "C" fn m0(n: usize) -> *mut c_void {
    unsafe { do_malloc(0, n) }
}
unsafe extern "C" fn m1(n: usize) -> *mut c_void {
    unsafe { do_malloc(1, n) }
}
unsafe extern "C" fn r0(p: *mut c_void, n: usize) -> *mut c_void {
    unsafe { do_realloc(0, p, n) }
}
unsafe extern "C" fn r1(p: *mut c_void, n: usize) -> *mut c_void {
    unsafe { do_realloc(1, p, n) }
}
unsafe extern "C" fn f0(p: *mut c_void) {
    unsafe { do_free(0, p) }
}
unsafe extern "C" fn f1(p: *mut c_void) {
    unsafe { do_free(1, p) }
}

/// Install the instrumented allocators. `with_realloc == false` uses
/// `json_set_alloc_funcs`, which sets `do_realloc = NULL` inside the library and
/// therefore exercises the realloc-emulation branch of `jsonp_realloc`.
unsafe fn install(with_realloc: bool, fail_at: usize) {
    let p = pair();
    unsafe {
        for i in 0..2 {
            let s = slot(i);
            s.log.clear();
            s.n_malloc = 0;
            s.n_realloc = 0;
            s.n_free = 0;
            s.count = 0;
            s.fail_at = fail_at;
            s.recording = true;
        }
        if with_realloc {
            (p.c.json_set_alloc_funcs2)(Some(m0), Some(r0), Some(f0));
            (p.r.json_set_alloc_funcs2)(Some(m1), Some(r1), Some(f1));
        } else {
            (p.c.json_set_alloc_funcs)(Some(m0), Some(f0));
            (p.r.json_set_alloc_funcs)(Some(m1), Some(f1));
        }
    }
}

unsafe fn restore() {
    let p = pair();
    let l = libc();
    unsafe {
        for i in 0..2 {
            slot(i).recording = false;
            slot(i).fail_at = 0;
        }
        (p.c.json_set_alloc_funcs2)(Some(l.malloc), Some(l.realloc), Some(l.free));
        (p.r.json_set_alloc_funcs2)(Some(l.malloc), Some(l.realloc), Some(l.free));
    }
}

unsafe fn logs() -> (Vec<(u8, usize)>, Vec<(u8, usize)>) {
    unsafe { (slot(0).log.clone(), slot(1).log.clone()) }
}

unsafe fn counts() -> ((usize, usize, usize), (usize, usize, usize)) {
    unsafe {
        (
            (slot(0).n_malloc, slot(0).n_realloc, slot(0).n_free),
            (slot(1).n_malloc, slot(1).n_realloc, slot(1).n_free),
        )
    }
}

/* ===================== F1: reading the allocator hooks ===================== */

#[test]
fn f1_get_alloc_funcs() {
    let _g = lock();
    let p = pair();
    unsafe {
        restore();
        // both libraries must report exactly what was installed
        for (api, m, r, f) in [
            (p.c, libc().malloc as usize, libc().realloc as usize, libc().free as usize),
            (p.r, libc().malloc as usize, libc().realloc as usize, libc().free as usize),
        ] {
            let mut mf: MallocFn = None;
            let mut rf: ReallocFn = None;
            let mut ff: FreeFn = None;
            (api.json_get_alloc_funcs2)(&mut mf, &mut rf, &mut ff);
            assert_eq!(mf.map(|x| x as usize), Some(m));
            assert_eq!(rf.map(|x| x as usize), Some(r));
            assert_eq!(ff.map(|x| x as usize), Some(f));
            let mut mf2: MallocFn = None;
            let mut ff2: FreeFn = None;
            (api.json_get_alloc_funcs)(&mut mf2, &mut ff2);
            assert_eq!(mf2.map(|x| x as usize), Some(m));
            assert_eq!(ff2.map(|x| x as usize), Some(f));
            // NULL out-params must be tolerated (ERRORS.md 133/134)
            (api.json_get_alloc_funcs)(std::ptr::null_mut(), std::ptr::null_mut());
            (api.json_get_alloc_funcs2)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
        // json_set_alloc_funcs must clear the realloc hook (memory.c)
        install(false, 0);
        for api in [p.c, p.r] {
            let mut mf: MallocFn = None;
            let mut rf: ReallocFn = None;
            let mut ff: FreeFn = None;
            (api.json_get_alloc_funcs2)(&mut mf, &mut rf, &mut ff);
            assert!(rf.is_none(), "json_set_alloc_funcs must NULL do_realloc");
            assert!(mf.is_some() && ff.is_some());
        }
        restore();
    }
}

/* =========== A14 / F2 / F3: identical allocation traces ================== */

fn workload(api: &Api) {
    unsafe {
        // strings, objects with rehash, arrays with growth, dump, load, pack
        let o = (api.json_object)();
        for i in 0..40 {
            let k = cstr(&format!("key{i:03}"));
            (api.json_object_set_new)(o, k.as_ptr(), (api.json_integer)(i));
        }
        let a = (api.json_array)();
        for i in 0..40 {
            (api.json_array_append_new)(a, (api.json_real)(i as f64 + 0.5));
        }
        (api.json_object_set_new)(o, cstr("arr").as_ptr(), a);
        (api.json_object_set_new)(
            o,
            cstr("str").as_ptr(),
            (api.json_string)(cstr("hello é € 𝄞").as_ptr()),
        );
        for f in [0usize, json_indent(3), JSON_SORT_KEYS, JSON_COMPACT] {
            if let Some(_b) = dumps(api, o, f) {}
        }
        let mut buf = [0i8; 4096];
        (api.json_dumpb)(o, buf.as_mut_ptr(), 4096, 0);
        let txt = cstr("{\"a\":[1,2,3,{\"b\":\"x\"}],\"c\":1.5e-7,\"d\":null}");
        let j = (api.json_loads)(txt.as_ptr(), 0, std::ptr::null_mut());
        let dc = (api.json_deep_copy)(j);
        let sc = (api.json_copy)(j);
        decref(api, sc);
        decref(api, dc);
        decref(api, j);
        let packed = (api.json_pack)(
            cstr("{s:i,s:[s,s],s:f}").as_ptr(),
            cstr("i").as_ptr(),
            7i32,
            cstr("l").as_ptr(),
            cstr("one").as_ptr(),
            cstr("two").as_ptr(),
            cstr("f").as_ptr(),
            2.5f64,
        );
        decref(api, packed);
        // jsonp_realloc directly (A13/A14)
        let mut q = (api.jsonp_malloc)(16);
        q = (api.jsonp_realloc)(q, 16, 64);
        q = (api.jsonp_realloc)(q, 64, 8);
        let z = (api.jsonp_realloc)(q, 8, 0);
        if !z.is_null() {
            (api.jsonp_free)(z);
        }
        // strbuffer growth
        let mut sb = StrbufferT::zeroed();
        if (api.strbuffer_init)(&mut sb) == 0 {
            for _ in 0..200 {
                (api.strbuffer_append_bytes)(&mut sb, b"0123456789".as_ptr() as *const c_char, 10);
            }
            (api.strbuffer_close)(&mut sb);
        }
        decref(api, o);
    }
}

#[test]
fn f2_f3_allocation_traces_match() {
    let _g = lock();
    let p = pair();
    unsafe {
        for with_realloc in [true, false] {
            // warm both dtoa freelists identically before recording
            restore();
            workload(p.c);
            workload(p.r);

            install(with_realloc, 0);
            workload(p.c);
            workload(p.r);
            let (lc, lr) = logs();
            let (cc, cr) = counts();
            restore();
            assert_eq!(
                cc, cr,
                "malloc/realloc/free call counts differ (with_realloc={with_realloc})"
            );
            assert_eq!(
                lc.len(),
                lr.len(),
                "allocation trace length differs (with_realloc={with_realloc})"
            );
            for (i, (x, y)) in lc.iter().zip(lr.iter()).enumerate() {
                assert_eq!(
                    x, y,
                    "allocation trace step {i} differs (with_realloc={with_realloc})"
                );
            }
            assert!(lc.len() > 300, "trace suspiciously short: {}", lc.len());
        }
    }
}

/* =========== F4 + ERRORS.md OOM rows: failing allocator sweep ============ */

/// Each closure drives one operation and returns a comparable observation.
fn oom_probes() -> Vec<(&'static str, fn(&Api) -> String)> {
    vec![
        ("json_object+set", |api| unsafe {
            let o = (api.json_object)();
            let mut r = String::new();
            if o.is_null() {
                return "obj=NULL".into();
            }
            for i in 0..12 {
                let k = cstr(&format!("k{i}"));
                r.push_str(&format!(
                    "{}",
                    (api.json_object_set_new)(o, k.as_ptr(), (api.json_integer)(i))
                ));
            }
            r.push_str(&format!(" size={}", (api.json_object_size)(o)));
            r.push_str(&format!(" dump={:?}", dumps(api, o, JSON_SORT_KEYS)));
            decref(api, o);
            r
        }),
        ("json_array+append", |api| unsafe {
            let a = (api.json_array)();
            if a.is_null() {
                return "arr=NULL".into();
            }
            let mut r = String::new();
            for i in 0..20 {
                r.push_str(&format!(
                    "{}",
                    (api.json_array_append_new)(a, (api.json_integer)(i))
                ));
            }
            r.push_str(&format!(" size={}", (api.json_array_size)(a)));
            r.push_str(&format!(" dump={:?}", dumps(api, a, 0)));
            decref(api, a);
            r
        }),
        ("json_string", |api| unsafe {
            let s = (api.json_string)(cstr("some string value").as_ptr());
            let r = format!("null={} dump={:?}", s.is_null(), dumps(api, s, JSON_ENCODE_ANY));
            decref(api, s);
            r
        }),
        ("json_loads", |api| unsafe {
            let mut e = JsonError::zeroed();
            let j = (api.json_loads)(
                cstr("{\"a\":[1,2,3,\"xyz\"],\"b\":{\"c\":1.5},\"d\":true}").as_ptr(),
                0,
                &mut e,
            );
            let r = format!(
                "null={} code={} text={:?} dump={:?}",
                j.is_null(),
                e.code(),
                e.text_str(),
                dumps(j_api(api), j, JSON_SORT_KEYS)
            );
            decref(api, j);
            r
        }),
        ("json_dumps", |api| unsafe {
            let j = (api.json_loads)(
                cstr("{\"a\":[1,2,3,\"xyz\"],\"b\":{\"c\":1.5}}").as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            let mut r = String::new();
            for f in [0usize, json_indent(2), JSON_SORT_KEYS] {
                r.push_str(&format!("{:?}|", dumps(api, j, f)));
            }
            decref(api, j);
            r
        }),
        ("json_pack", |api| unsafe {
            let mut e = JsonError::zeroed();
            let j = (api.json_pack_ex)(
                &mut e,
                0usize,
                cstr("{s:i,s:[s,s],s:f,s:s+}").as_ptr(),
                cstr("i").as_ptr(),
                7i32,
                cstr("l").as_ptr(),
                cstr("one").as_ptr(),
                cstr("two").as_ptr(),
                cstr("f").as_ptr(),
                2.5f64,
                cstr("cat").as_ptr(),
                cstr("aa").as_ptr(),
                cstr("bb").as_ptr(),
            );
            let r = format!(
                "null={} code={} text={:?} dump={:?}",
                j.is_null(),
                e.code(),
                e.text_str(),
                dumps(api, j, JSON_SORT_KEYS)
            );
            decref(api, j);
            r
        }),
        ("json_unpack_strict", |api| unsafe {
            let j = (api.json_loads)(
                cstr("{\"a\":1,\"b\":2,\"c\":3}").as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            let mut e = JsonError::zeroed();
            let mut iv: c_int = 0;
            let ret = (api.json_unpack_ex)(
                j,
                &mut e,
                JSON_STRICT,
                cstr("{s:i}").as_ptr(),
                cstr("a").as_ptr(),
                &mut iv,
            );
            let r = format!("ret={ret} code={} text={:?}", e.code(), e.text_str());
            decref(api, j);
            r
        }),
        ("json_deep_copy", |api| unsafe {
            let j = (api.json_loads)(
                cstr("{\"a\":[1,2,[3,{\"b\":\"str\"}]],\"c\":{\"d\":[]}}").as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            let c = (api.json_deep_copy)(j);
            let r = format!("null={} dump={:?}", c.is_null(), dumps(api, c, JSON_SORT_KEYS));
            decref(api, c);
            decref(api, j);
            r
        }),
        ("json_object_update_recursive", |api| unsafe {
            let a = (api.json_loads)(
                cstr("{\"x\":{\"y\":{\"z\":1}}}").as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            let b = (api.json_loads)(
                cstr("{\"x\":{\"y\":{\"w\":2},\"v\":3}}").as_ptr(),
                0,
                std::ptr::null_mut(),
            );
            let ret = (api.json_object_update_recursive)(a, b);
            let r = format!("ret={ret} dump={:?}", dumps(api, a, JSON_SORT_KEYS));
            decref(api, a);
            decref(api, b);
            r
        }),
        ("json_sprintf", |api| unsafe {
            let s = (api.json_sprintf)(cstr("value=%d text=%s").as_ptr(), 12345i32, cstr("abc").as_ptr());
            let r = format!("null={} dump={:?}", s.is_null(), dumps(api, s, JSON_ENCODE_ANY));
            decref(api, s);
            r
        }),
        ("strbuffer_growth", |api| unsafe {
            let mut sb = StrbufferT::zeroed();
            let init = (api.strbuffer_init)(&mut sb);
            if init != 0 {
                return "init=-1".into();
            }
            let mut r = String::new();
            for _ in 0..12 {
                r.push_str(&format!(
                    "{}",
                    (api.strbuffer_append_bytes)(&mut sb, b"0123456789".as_ptr() as *const c_char, 10)
                ));
            }
            r.push_str(&format!(" len={} size={}", sb.length, sb.size));
            (api.strbuffer_close)(&mut sb);
            r
        }),
        ("hashtable_rehash", |api| unsafe {
            let mut ht = HashtableT::zeroed();
            if (api.hashtable_init)(&mut ht) != 0 {
                return "ht-init=-1".into();
            }
            let mut r = String::new();
            for i in 0..20 {
                let k = format!("k{i:02}");
                r.push_str(&format!(
                    "{}",
                    (api.hashtable_set)(
                        &mut ht,
                        k.as_ptr() as *const c_char,
                        k.len(),
                        (api.json_integer)(i)
                    )
                ));
            }
            r.push_str(&format!(" size={} order={}", ht.size, ht.order));
            (api.hashtable_close)(&mut ht);
            r
        }),
    ]
}

fn j_api(api: &Api) -> &Api {
    api
}

#[test]
fn f4_failing_allocator_sweep() {
    let _g = lock();
    let p = pair();
    unsafe {
        let mut oom_observed = 0usize;
        for (name, probe) in oom_probes() {
            restore();
            let baseline = probe(p.c);
            for n in 1..=45usize {
                // warm-up with the default allocator so dtoa's freelist state
                // is identical on both sides before the injected failure
                restore();
                probe(p.c);
                probe(p.r);

                install(true, n);
                let a = probe(p.c);
                let b = probe(p.r);
                let (lc, lr) = logs();
                restore();
                assert_eq!(a, b, "OOM divergence: {name} fail_at={n}");
                assert_eq!(
                    lc.len(),
                    lr.len(),
                    "OOM allocation trace length differs: {name} fail_at={n}"
                );
                for (i, (x, y)) in lc.iter().zip(lr.iter()).enumerate() {
                    assert_eq!(x, y, "OOM trace step {i}: {name} fail_at={n}");
                }
                if a != baseline {
                    oom_observed += 1;
                }
            }
            // and the same sweep with the realloc-emulation allocator
            for n in 1..=25usize {
                restore();
                probe(p.c);
                probe(p.r);
                install(false, n);
                let a = probe(p.c);
                let b = probe(p.r);
                let (lc, lr) = logs();
                restore();
                assert_eq!(a, b, "OOM divergence (no realloc): {name} fail_at={n}");
                assert_eq!(lc, lr, "OOM trace differs (no realloc): {name} fail_at={n}");
            }
        }
        assert!(
            oom_observed > 100,
            "the injected allocation failures barely changed behaviour ({oom_observed} \
             observations) — the sweep is not actually exercising the OOM paths"
        );
    }
}

/* =========== F5: restoring the defaults ================== */

#[test]
fn f5_restore_defaults() {
    let _g = lock();
    let p = pair();
    unsafe {
        install(true, 0);
        workload(p.c);
        workload(p.r);
        restore();
        // after restoring, behaviour must be identical again
        let a = dumps(
            p.c,
            (p.c.json_loads)(cstr("[1,2,3]").as_ptr(), 0, std::ptr::null_mut()),
            0,
        );
        let b = dumps(
            p.r,
            (p.r.json_loads)(cstr("[1,2,3]").as_ptr(), 0, std::ptr::null_mut()),
            0,
        );
        assert_eq!(a, b);
        // ERRORS.md 129/130/131/132
        install(false, 0); // do_realloc == NULL
        for api in [p.c, p.r] {
            assert!((api.jsonp_malloc)(0).is_null(), "jsonp_malloc(0) must be NULL");
            (api.jsonp_free)(std::ptr::null_mut());
        }
        let mut res = Vec::new();
        for api in [p.c, p.r] {
            let q = (api.jsonp_malloc)(32);
            let z = (api.jsonp_realloc)(q, 32, 0); // emulation path, newSize == 0
            res.push(z.is_null());
        }
        assert_eq!(res[0], res[1], "jsonp_realloc(.., 0) with NULL do_realloc");
        restore();
    }
}
