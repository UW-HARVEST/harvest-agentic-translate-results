//! Phase C — out-of-memory error paths (the 58 `OOM` rows in ERRORS.md).
//!
//! These branches are unreachable with a working allocator, so we reach them the
//! way the library itself documents: by installing a failing allocator through the
//! public `json_set_alloc_funcs` / `json_set_alloc_funcs2` hooks.
//!
//! Strategy: fail the Nth allocation, for N = 1..=K, and compare the observable
//! outcome (return value + full `json_error_t`) between C and Rust for every N.
//! Sweeping N walks the failure point through every internal allocation site of
//! the operation, which is what actually covers the individual OOM rows.
//!
//! It also asserts the two libraries perform the SAME NUMBER of allocations for
//! the same operation — a strong structural equivalence check that no
//! happy-path test can make.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicU64, Ordering};

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// 0 = never fail. Otherwise fail the FAIL_AT'th and every later allocation.
static FAIL_AT: AtomicU64 = AtomicU64::new(0);
static COUNT: AtomicU64 = AtomicU64::new(0);

/// The allocator hooks and the counters are PROCESS-GLOBAL, and
/// `json_set_alloc_funcs` mutates global state inside each loaded library.
/// libtest runs `#[test]` fns on parallel threads, so without this lock the
/// tests corrupt each other's allocation counts and fail spuriously.
/// Every test in this file must take the guard first.
static OOM_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn oom_guard() -> std::sync::MutexGuard<'static, ()> {
    // Recover from a poisoned lock so one failing test does not cascade.
    OOM_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

unsafe extern "C" fn hook_malloc(n: usize) -> *mut c_void {
    let c = COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    let f = FAIL_AT.load(Ordering::SeqCst);
    if f != 0 && c >= f {
        return std::ptr::null_mut();
    }
    malloc(n)
}

unsafe extern "C" fn hook_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    let c = COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    let f = FAIL_AT.load(Ordering::SeqCst);
    if f != 0 && c >= f {
        return std::ptr::null_mut();
    }
    realloc(p, n)
}

unsafe extern "C" fn hook_free(p: *mut c_void) {
    free(p)
}

type MallocFn = unsafe extern "C" fn(usize) -> *mut c_void;
type ReallocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FreeFn = unsafe extern "C" fn(*mut c_void);
type SetAlloc2 = unsafe extern "C" fn(MallocFn, ReallocFn, FreeFn);
type SetAlloc = unsafe extern "C" fn(MallocFn, FreeFn);

/// Install the counting/failing allocator (with a realloc hook, so
/// `jsonp_realloc` takes its direct path).
unsafe fn install_hooks2(lib: &Library) {
    let f: Symbol<SetAlloc2> = sym(lib, "json_set_alloc_funcs2");
    f(hook_malloc, hook_realloc, hook_free);
}

/// Install malloc+free only. This forces `do_realloc == NULL`, so every
/// `jsonp_realloc` goes through the malloc+memcpy+free EMULATION path
/// (`memory.c:45-61`) — a different set of branches (ERRORS.md rows 343, 345).
unsafe fn install_hooks_no_realloc(lib: &Library) {
    let f: Symbol<SetAlloc> = sym(lib, "json_set_alloc_funcs");
    f(hook_malloc, hook_free);
}

fn begin(fail_at: u64) {
    COUNT.store(0, Ordering::SeqCst);
    FAIL_AT.store(fail_at, Ordering::SeqCst);
}

fn end() -> u64 {
    FAIL_AT.store(0, Ordering::SeqCst);
    COUNT.load(Ordering::SeqCst)
}

/// Run `op` under an allocator that fails the Nth allocation, for N in 1..=k,
/// and return one comparable record per N. `op` must clean up after itself as
/// best it can; leaks under simulated OOM are expected and harmless here.
fn oom_sweep<T: PartialEq + std::fmt::Debug>(
    label: &str,
    k: u64,
    with_realloc: bool,
    op: impl Fn(&Library) -> T,
) {
    diff(label, move |lib: &Library| unsafe {
        if with_realloc {
            install_hooks2(lib);
        } else {
            install_hooks_no_realloc(lib);
        }
        // First, the unfailing run: records the TOTAL allocation count, which
        // must itself match between C and Rust.
        begin(0);
        let baseline = op(lib);
        let total = end();

        let base_str = format!("{:?}", baseline);
        // Guard against a hollow test: the operation MUST actually allocate
        // through our hook, otherwise we are not exercising any OOM branch.
        assert!(
            total > 0,
            "[{}] operation performed 0 allocations through the hook — the \
             allocator was not installed, so no OOM row is being covered",
            label
        );

        let mut records = Vec::new();
        records.push((0u64, base_str.clone(), total));
        let mut differed = 0usize;
        for n in 1..=k {
            begin(n);
            let r = op(lib);
            let used = end();
            let s = format!("{:?}", r);
            if s != base_str {
                differed += 1;
            }
            records.push((n, s, used));
        }
        // Guard against a vacuous sweep: injecting failures must change the
        // outcome for at least one N, else every record equals the happy path
        // and the comparison proves nothing about error handling.
        assert!(
            differed > 0,
            "[{}] injecting allocation failures for N=1..={} never changed the \
             result — the sweep is not reaching any failure branch",
            label,
            k
        );

        // Restore the library's default allocator so later tests are unaffected.
        install_hooks2(lib);
        begin(0);
        records
    });
}

// ---------------------------------------------------------------- constructors

#[test]
fn oom_constructors() {
    let _guard = oom_guard();
    // rows 3, 4, 61, 62, 92-94, 113, 118: every heap constructor.
    oom_sweep("oom/constructors", 6, true, |lib: &Library| unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let st: Symbol<FnStr> = sym(lib, "json_string");
        let stn: Symbol<FnStrN> = sym(lib, "json_stringn");
        let stnc: Symbol<FnStr> = sym(lib, "json_string_nocheck");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let real: Symbol<FnReal> = sym(lib, "json_real");

        let o = obj();
        let a = arr();
        let s = st(cs("hello").as_ptr());
        let sn = stn(b"ab\0cd".as_ptr() as *const c_char, 5);
        let snc = stnc(cs("nc").as_ptr());
        let i = int(42);
        let r = real(1.5);
        let out = format!(
            "obj={} arr={} str={} strn={} strnc={} int={} real={}",
            o.is_null(),
            a.is_null(),
            s.is_null(),
            sn.is_null(),
            snc.is_null(),
            i.is_null(),
            r.is_null()
        );
        for p in [o, a, s, sn, snc, i, r] {
            if !p.is_null() {
                decref(lib, p);
            }
        }
        out
    });
}

// ---------------------------------------------------------------- containers

#[test]
fn oom_object_insert_and_rehash() {
    let _guard = oom_guard();
    // rows 18, 39, 325, 326, 328, 329, 330: hashtable_init, init_pair, rehash.
    oom_sweep("oom/object insert+rehash", 30, true, |lib: &Library| unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let osetn: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
        let sz: Symbol<FnSize> = sym(lib, "json_object_size");

        let o = obj();
        if o.is_null() {
            return "object=NULL".to_string();
        }
        let mut rcs = Vec::new();
        // 12 keys crosses the 9th-key rehash boundary.
        for i in 0..12 {
            let k = format!("key{:02}", i);
            let v = int(i);
            rcs.push(osetn(o, k.as_ptr() as *const c_char, k.len(), v));
        }
        let out = format!("size={} rcs={:?} dump={:?}", sz(o), rcs, dumps_to_string(lib, o, JSON_SORT_KEYS));
        decref(lib, o);
        out
    });
}

#[test]
fn oom_array_append_and_grow() {
    let _guard = oom_guard();
    // rows 70, 74, 79, 85: json_array_grow / jsonp_realloc failures.
    for with_realloc in [true, false] {
        oom_sweep(
            &format!("oom/array grow realloc={}", with_realloc),
            24,
            with_realloc,
            |lib: &Library| unsafe {
                let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
                let int: Symbol<FnInt> = sym(lib, "json_integer");
                let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
                let aext: Symbol<FnTwoJson> = sym(lib, "json_array_extend");
                let sz: Symbol<FnSize> = sym(lib, "json_array_size");

                let a = arr();
                if a.is_null() {
                    return "array=NULL".to_string();
                }
                let mut rcs = Vec::new();
                // crosses the 9th-append grow boundary
                for i in 0..12 {
                    rcs.push(aapp(a, int(i)));
                }
                // extend with a big other -> the size+amount side of grow
                let b = arr();
                if !b.is_null() {
                    for i in 0..40 {
                        aapp(b, int(100 + i));
                    }
                    rcs.push(aext(a, b));
                    decref(lib, b);
                }
                let out = format!("size={} rcs={:?} dump={:?}", sz(a), rcs, dumps_to_string(lib, a, 0));
                decref(lib, a);
                out
            },
        );
    }
}

// ---------------------------------------------------------------- encode

#[test]
fn oom_dumps() {
    let _guard = oom_guard();
    // rows 195, 220, 229, 230, 231, 239: strbuffer_init, SORT_KEYS key array,
    // the final jsonp_realloc shrink, and hashtable_init for the parents set.
    for with_realloc in [true, false] {
        for flags in [0usize, JSON_SORT_KEYS, json_indent(2), JSON_SORT_KEYS | json_indent(4)] {
            oom_sweep(
                &format!("oom/dumps flags={:#x} realloc={}", flags, with_realloc),
                40,
                with_realloc,
                move |lib: &Library| unsafe {
                    let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> =
                        sym(lib, "json_object");
                    let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
                    let int: Symbol<FnInt> = sym(lib, "json_integer");
                    let st: Symbol<FnStr> = sym(lib, "json_string");
                    let oset: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
                    let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");

                    // Build with the allocator possibly failing too, which is
                    // realistic and exercises the constructor OOM paths as well.
                    let root = obj();
                    if root.is_null() {
                        return "root=NULL".to_string();
                    }
                    let a = arr();
                    if !a.is_null() {
                        for i in 0..5 {
                            aapp(a, int(i));
                        }
                        oset(root, cs("arr").as_ptr(), a);
                    }
                    for i in 0..6 {
                        // MUST be a CString: json_object_set_new derives the key
                        // length with strlen, and a Rust String is NOT
                        // NUL-terminated — passing `String::as_ptr()` makes the
                        // library read past the end into heap garbage, which both
                        // corrupts the key and (because the garbage differs between
                        // the two libraries) makes this OOM sweep flaky.
                        let k = cs(&format!("k{}", i));
                        oset(root, k.as_ptr(), st(cs("value").as_ptr()));
                    }
                    let d = dumps_to_string(lib, root, flags);
                    // json_dumpb has a different failure return (0)
                    let dumpb: Symbol<FnDumpb> = sym(lib, "json_dumpb");
                    let nb = dumpb(root, std::ptr::null_mut(), 0, flags);
                    decref(lib, root);
                    format!("dumps={:?} dumpb={}", d, nb)
                },
            );
        }
    }
}

// ---------------------------------------------------------------- decode

#[test]
fn oom_loads() {
    let _guard = oom_guard();
    // rows 147, 157, 164, 167, 169, 174, 181, 184: every allocation site in the
    // parser (lex_init strbuffer, json_object/array, setn/append, string steal).
    for with_realloc in [true, false] {
        oom_sweep(
            &format!("oom/loads realloc={}", with_realloc),
            45,
            with_realloc,
            |lib: &Library| unsafe {
                let text = br#"{"alpha":[1,2,3,{"nested":"value"}],"beta":"a longer string here","gamma":1.5e3,"delta":true,"eps":null}"#;
                let (dump, err) = load_then_dump(lib, text, 0, JSON_SORT_KEYS);
                format!("dump={:?} err={:?}", dump, err)
            },
        );
    }
}

#[test]
fn oom_loads_error_message_paths() {
    let _guard = oom_guard();
    // The error-reporting machinery itself allocates (strbuffer for saved_text),
    // so an OOM during error formatting must degrade identically.
    oom_sweep("oom/loads syntax error", 20, true, |lib: &Library| unsafe {
        let mut out = Vec::new();
        for bad in [
            &b"{"[..],
            &b"[1,]"[..],
            &b"{\"a\" 1}"[..],
            &b"[01]"[..],
            &b"[1e999]"[..],
            &b"[\"\\uD800\"]"[..],
            &b"nope"[..],
        ] {
            let (d, e) = load_then_dump(lib, bad, 0, 0);
            out.push(format!("{:?} -> {:?} / {:?}", bad, d, e));
        }
        out.join(" | ")
    });
}

// ---------------------------------------------------------------- copy / update

#[test]
fn oom_deep_copy_and_updates() {
    let _guard = oom_guard();
    // rows 1, 2, 38, 39, 57, 59, 60, 88, 90, 91, 132: loop-check hashtable,
    // deep copy allocations, recursive update.
    oom_sweep("oom/copy+update", 40, true, |lib: &Library| unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let st: Symbol<FnStr> = sym(lib, "json_string");
        let oset: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
        let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        let copy: Symbol<FnCopy> = sym(lib, "json_copy");
        let deep: Symbol<FnDeepCopy> = sym(lib, "json_deep_copy");
        let upd: Symbol<FnTwoJson> = sym(lib, "json_object_update");
        let updr: Symbol<FnTwoJson> = sym(lib, "json_object_update_recursive");

        let src = obj();
        if src.is_null() {
            return "src=NULL".to_string();
        }
        let inner = obj();
        if !inner.is_null() {
            oset(inner, cs("x").as_ptr(), int(1));
            oset(inner, cs("y").as_ptr(), st(cs("str").as_ptr()));
            oset(src, cs("inner").as_ptr(), inner);
        }
        let a = arr();
        if !a.is_null() {
            for i in 0..4 {
                aapp(a, int(i));
            }
            oset(src, cs("arr").as_ptr(), a);
        }

        let c1 = copy(src);
        let c2 = deep(src);

        let dst = obj();
        let (r1, r2) = if dst.is_null() {
            (-99, -99)
        } else {
            oset(dst, cs("keep").as_ptr(), int(7));
            (upd(dst, src), updr(dst, src))
        };

        let out = format!(
            "copy={} deep={} upd={} updr={} deepdump={:?}",
            c1.is_null(),
            c2.is_null(),
            r1,
            r2,
            if c2.is_null() { None } else { dumps_to_string(lib, c2, JSON_SORT_KEYS) }
        );
        for p in [src, c1, c2, dst] {
            if !p.is_null() {
                decref(lib, p);
            }
        }
        out
    });
}

// ---------------------------------------------------------------- pack / unpack

#[test]
fn oom_pack_and_unpack() {
    let _guard = oom_guard();
    // rows 244, 246, 251, 252, 255, 256, 259, 260, 266: pack/unpack allocation
    // sites incl. read_string's strbuffer and unpack_object's key_set.
    type PackEx = unsafe extern "C" fn(*mut json_error_t, usize, *const c_char, ...) -> *mut json_t;
    type UnpackEx =
        unsafe extern "C" fn(*mut json_t, *mut json_error_t, usize, *const c_char, ...) -> c_int;

    oom_sweep("oom/pack+unpack", 40, true, |lib: &Library| unsafe {
        let pack: Symbol<PackEx> = sym(lib, "json_pack_ex");
        let unpack: Symbol<UnpackEx> = sym(lib, "json_unpack_ex");

        let ka = cs("a");
        let kb = cs("b");
        let kc = cs("c");
        let s1 = cs("hello");
        let s2 = cs("world");

        let mut e1 = json_error_t::new();
        // exercise '+' concatenation (strbuffer), nested containers, and 'f'
        let v = pack(
            &mut e1,
            0,
            cs("{s:s+,s:[i,i,i],s:f}").as_ptr(),
            ka.as_ptr(),
            s1.as_ptr(),
            s2.as_ptr(),
            kb.as_ptr(),
            1 as c_int,
            2 as c_int,
            3 as c_int,
            kc.as_ptr(),
            2.5f64,
        );

        let mut out = format!("pack={} err={:?}", v.is_null(), e1.snapshot());

        if !v.is_null() {
            let mut e2 = json_error_t::new();
            let mut got: *const c_char = std::ptr::null();
            let mut i1: c_int = 0;
            let mut i2: c_int = 0;
            let mut i3: c_int = 0;
            let mut d: f64 = 0.0;
            let rc = unpack(
                v,
                &mut e2,
                JSON_STRICT,
                cs("{s:s,s:[i,i,i],s:f}").as_ptr(),
                ka.as_ptr(),
                &mut got,
                kb.as_ptr(),
                &mut i1,
                &mut i2,
                &mut i3,
                kc.as_ptr(),
                &mut d,
            );
            out.push_str(&format!(
                " unpack={} err={:?} got={:?} ints=({},{},{}) d={}",
                rc,
                e2.snapshot(),
                if got.is_null() { "<null>".to_string() } else { cstr_to_string(got) },
                i1,
                i2,
                i3,
                d
            ));
            out.push_str(&format!(" dump={:?}", dumps_to_string(lib, v, JSON_SORT_KEYS)));
            decref(lib, v);
        }
        out
    });
}

#[test]
fn oom_sprintf() {
    let _guard = oom_guard();
    // rows 110-112: json_sprintf's vsnprintf sizing + malloc + UTF-8 validation.
    type Sprintf = unsafe extern "C" fn(*const c_char, ...) -> *mut json_t;
    oom_sweep("oom/sprintf", 8, true, |lib: &Library| unsafe {
        let f: Symbol<Sprintf> = sym(lib, "json_sprintf");
        let fmt = cs("value=%d name=%s");
        let name = cs("abcdefghij");
        let v = f(fmt.as_ptr(), 12345 as c_int, name.as_ptr());
        let out = format!("null={} dump={:?}", v.is_null(), {
            if v.is_null() {
                None
            } else {
                dumps_to_string(lib, v, JSON_ENCODE_ANY)
            }
        });
        if !v.is_null() {
            decref(lib, v);
        }
        out
    });
}

// ---------------------------------------------------------------- realloc modes

#[test]
fn rows343_345_jsonp_realloc_emulation_and_zero_size() {
    let _guard = oom_guard();
    // row 343: do_realloc == NULL and newSize == 0 -> frees ptr, returns NULL.
    // row 345: emulation path where do_malloc returns NULL (old ptr NOT freed).
    type FnMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
    type FnFree = unsafe extern "C" fn(*mut c_void);
    type FnRealloc3 = unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void;

    diff("rows343-345/jsonp_realloc", |lib: &Library| unsafe {
        // Save the current allocators so we can restore them afterwards.
        type GetAlloc2 = unsafe extern "C" fn(*mut MallocFn, *mut ReallocFn, *mut FreeFn);
        let get2: Symbol<GetAlloc2> = sym(lib, "json_get_alloc_funcs2");
        let mut om: MallocFn = hook_malloc;
        let mut or_: ReallocFn = hook_realloc;
        let mut of: FreeFn = hook_free;
        get2(&mut om, &mut or_, &mut of);

        let m: Symbol<FnMalloc> = sym(lib, "jsonp_malloc");
        let fr: Symbol<FnFree> = sym(lib, "jsonp_free");
        let rl: Symbol<FnRealloc3> = sym(lib, "jsonp_realloc");

        let mut out = Vec::new();

        // --- emulation mode: malloc+free only, so do_realloc == NULL
        install_hooks_no_realloc(lib);
        begin(0);
        let p = m(32);
        out.push(format!("emul alloc null={}", p.is_null()));
        // grow via emulation (malloc + memcpy + free)
        let p2 = rl(p, 32, 64);
        out.push(format!("emul grow null={}", p2.is_null()));
        // newSize == 0 in emulation mode -> frees and returns NULL (row 343)
        let p3 = rl(p2, 64, 0);
        out.push(format!("emul newSize0 null={}", p3.is_null()));

        // row 345: emulation malloc fails -> returns NULL, old ptr NOT freed
        let p4 = m(16);
        begin(1); // next allocation fails
        let p5 = rl(p4, 16, 128);
        let _ = end();
        out.push(format!("emul oom null={}", p5.is_null()));
        fr(p4); // still our responsibility, proving it was not freed

        // --- direct mode: a realloc hook IS installed
        install_hooks2(lib);
        begin(0);
        let q = m(32);
        let q2 = rl(q, 32, 64);
        out.push(format!("direct grow null={}", q2.is_null()));
        let q3 = rl(q2, 64, 0);
        out.push(format!("direct newSize0 null={}", q3.is_null()));
        if !q3.is_null() {
            fr(q3);
        }

        // restore whatever was installed before this test
        let set2: Symbol<SetAlloc2> = sym(lib, "json_set_alloc_funcs2");
        set2(om, or_, of);
        begin(0);
        out
    });
}

#[test]
fn rows347_348_get_alloc_funcs_null_outparams() {
    let _guard = oom_guard();
    // row 348: NULL out-parameters are skipped individually (no crash).
    diff("rows347-348/get_alloc_funcs", |lib: &Library| unsafe {
        type GetAlloc = unsafe extern "C" fn(*mut MallocFn, *mut FreeFn);
        type GetAlloc2 = unsafe extern "C" fn(*mut MallocFn, *mut ReallocFn, *mut FreeFn);
        let get: Symbol<GetAlloc> = sym(lib, "json_get_alloc_funcs");
        let get2: Symbol<GetAlloc2> = sym(lib, "json_get_alloc_funcs2");
        let set2: Symbol<SetAlloc2> = sym(lib, "json_set_alloc_funcs2");

        // Install known hooks so the getters have something deterministic to
        // report; we compare only WHICH of our own hooks come back, never the
        // library's default addresses (those differ between the two .so files).
        set2(hook_malloc, hook_realloc, hook_free);

        let mut m: MallocFn = hook_malloc;
        let mut r: ReallocFn = hook_realloc;
        let mut f: FreeFn = hook_free;
        get2(&mut m, &mut r, &mut f);
        let same2 = (
            m as *const () as usize == hook_malloc as *const () as usize,
            r as *const () as usize == hook_realloc as *const () as usize,
            f as *const () as usize == hook_free as *const () as usize,
        );

        let mut m2: MallocFn = hook_malloc;
        let mut f2: FreeFn = hook_free;
        get(&mut m2, &mut f2);
        let same1 = (m2 as *const () as usize == hook_malloc as *const () as usize, f2 as *const () as usize == hook_free as *const () as usize);

        // Each out-param individually NULL must be skipped, not crash.
        get2(std::ptr::null_mut(), &mut r, &mut f);
        get2(&mut m, std::ptr::null_mut(), &mut f);
        get2(&mut m, &mut r, std::ptr::null_mut());
        get2(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        get(std::ptr::null_mut(), &mut f2);
        get(&mut m2, std::ptr::null_mut());
        get(std::ptr::null_mut(), std::ptr::null_mut());

        (same1, same2, "survived-null-outparams")
    });
}
