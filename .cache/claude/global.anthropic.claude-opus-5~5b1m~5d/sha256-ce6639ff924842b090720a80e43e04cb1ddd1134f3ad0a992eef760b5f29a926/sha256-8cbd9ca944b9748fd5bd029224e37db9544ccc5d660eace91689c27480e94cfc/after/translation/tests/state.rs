//! CONFIGS.md rows 1-10: state creation / destruction, allocator, context,
//! report, panic, gc and limits.
//!
//! Every test runs the identical closure against the C `libmujs.so` and the
//! Rust `libmujs.so` in separate forked children and compares captured
//! stdout+stderr bytes and the exit status.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

/* ------------------------------------------------------------------------- */
/* printing (always flushed so that library writes to stderr interleave
 * deterministically with our own writes to the shared capture fd)           */
/* ------------------------------------------------------------------------- */

unsafe fn fl() {
    libc::fflush(null_mut());
}

unsafe fn pl(s: &str) {
    p_line(s);
    fl();
}

unsafe fn pi(l: &str, v: c_int) {
    p_int(l, v);
    fl();
}

unsafe fn ps(l: &str, s: *const c_char) {
    p_str(l, s);
    fl();
}

unsafe fn pp(l: &str, p: *const c_void) {
    p_ptr_nonnull(l, p);
    fl();
}

unsafe fn pi64(l: &str, v: i64) {
    libc::printf(cs("%s=%lld\n").as_ptr(), cs(l).as_ptr(), v as libc::c_longlong);
    fl();
}

unsafe fn phash(l: &str, v: u64) {
    libc::printf(cs("%s=%016llx\n").as_ptr(), cs(l).as_ptr(), v as libc::c_ulonglong);
    fl();
}

/* ------------------------------------------------------------------------- */
/* "current api" so that C callbacks can call back into the right library     */
/* ------------------------------------------------------------------------- */

static CUR: AtomicPtr<Api> = AtomicPtr::new(null_mut());

fn set_cur(api: &Api) {
    CUR.store(api as *const Api as *mut Api, Ordering::SeqCst);
}

unsafe fn cur() -> &'static Api {
    &*(CUR.load(Ordering::SeqCst) as *const Api)
}

/// `print(...)` for scripts — the library itself provides none.
unsafe extern "C" fn cf_print(J: JS) {
    let api = cur();
    let top = (api.js_gettop)(J);
    let mut i = 1;
    while i < top {
        if i > 1 {
            libc::putchar(b' ' as c_int);
        }
        let s = (api.js_tostring)(J, i);
        libc::printf(cs("%s").as_ptr(), s);
        i += 1;
    }
    libc::putchar(b'\n' as c_int);
    fl();
    (api.js_pushundefined)(J);
}

unsafe fn install_print(api: &Api, J: JS) {
    (api.js_newcfunction)(J, Some(cf_print), cs("print").as_ptr(), 1);
    (api.js_setglobal)(J, cs("print").as_ptr());
}

unsafe fn mkstate(api: &Api, flags: c_int) -> JS {
    let J = newstate(api, flags);
    install_print(api, J);
    J
}

/* ------------------------------------------------------------------------- */
/* counting allocators (row 3)                                               */
/* ------------------------------------------------------------------------- */

struct Counters {
    nev: AtomicU64,
    n_new: AtomicU64,
    n_realloc: AtomicU64,
    n_free: AtomicU64,
    n_freenull: AtomicU64,
    bytes: AtomicU64,
    maxsz: AtomicU64,
    hash: AtomicU64,
    trace: AtomicU64,
}

impl Counters {
    const fn new() -> Counters {
        Counters {
            nev: AtomicU64::new(0),
            n_new: AtomicU64::new(0),
            n_realloc: AtomicU64::new(0),
            n_free: AtomicU64::new(0),
            n_freenull: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            maxsz: AtomicU64::new(0),
            hash: AtomicU64::new(0xcbf2_9ce4_8422_2325),
            trace: AtomicU64::new(0),
        }
    }
    fn reset(&self, trace: u64) {
        self.nev.store(0, Ordering::SeqCst);
        self.n_new.store(0, Ordering::SeqCst);
        self.n_realloc.store(0, Ordering::SeqCst);
        self.n_free.store(0, Ordering::SeqCst);
        self.n_freenull.store(0, Ordering::SeqCst);
        self.bytes.store(0, Ordering::SeqCst);
        self.maxsz.store(0, Ordering::SeqCst);
        self.hash.store(0xcbf2_9ce4_8422_2325, Ordering::SeqCst);
        self.trace.store(trace, Ordering::SeqCst);
    }
    unsafe fn dump(&self, label: &str) {
        libc::printf(cs("--- %s ---\n").as_ptr(), cs(label).as_ptr());
        fl();
        pi64("events", self.nev.load(Ordering::SeqCst) as i64);
        pi64("n_new", self.n_new.load(Ordering::SeqCst) as i64);
        pi64("n_realloc", self.n_realloc.load(Ordering::SeqCst) as i64);
        pi64("n_free", self.n_free.load(Ordering::SeqCst) as i64);
        pi64("n_freenull", self.n_freenull.load(Ordering::SeqCst) as i64);
        pi64(
            "live",
            self.n_new.load(Ordering::SeqCst) as i64 - self.n_free.load(Ordering::SeqCst) as i64,
        );
        pi64("bytes_requested", self.bytes.load(Ordering::SeqCst) as i64);
        pi64("max_size", self.maxsz.load(Ordering::SeqCst) as i64);
        phash("seq_hash", self.hash.load(Ordering::SeqCst));
    }
}

static CNT_A: Counters = Counters::new();
static CNT_B: Counters = Counters::new();

unsafe fn account(c: &Counters, ptr: *mut c_void, size: c_int) {
    let n = c.nev.fetch_add(1, Ordering::SeqCst);
    let kind: u64 = if size == 0 {
        if ptr.is_null() {
            c.n_freenull.fetch_add(1, Ordering::SeqCst);
            0
        } else {
            c.n_free.fetch_add(1, Ordering::SeqCst);
            1
        }
    } else if ptr.is_null() {
        c.n_new.fetch_add(1, Ordering::SeqCst);
        c.bytes.fetch_add(size as u64, Ordering::SeqCst);
        2
    } else {
        c.n_realloc.fetch_add(1, Ordering::SeqCst);
        c.bytes.fetch_add(size as u64, Ordering::SeqCst);
        3
    };
    if size > 0 && (size as u64) > c.maxsz.load(Ordering::SeqCst) {
        c.maxsz.store(size as u64, Ordering::SeqCst);
    }
    /* FNV-1a over the (kind, size) event sequence */
    let mut h = c.hash.load(Ordering::SeqCst);
    let word = (kind << 40) ^ (size as u32 as u64);
    for i in 0..8 {
        h ^= (word >> (8 * i)) & 0xff;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    c.hash.store(h, Ordering::SeqCst);

    if c.trace.load(Ordering::SeqCst) > n {
        let k = match kind {
            0 => "free(null)",
            1 => "free",
            2 => "new",
            _ => "realloc",
        };
        libc::printf(
            cs("ev%02llu %s size=%d\n").as_ptr(),
            n as libc::c_ulonglong,
            cs(k).as_ptr(),
            size,
        );
        fl();
    }
}

/// realloc-style allocator mirroring `js_defaultalloc`, counting into a static.
unsafe extern "C" fn alloc_static(_actx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    account(&CNT_A, ptr, size);
    if size == 0 {
        libc::free(ptr);
        return null_mut();
    }
    libc::realloc(ptr, size as usize)
}

/// Same, but the counters live behind `actx`.
unsafe extern "C" fn alloc_actx(actx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    let c = &*(actx as *const Counters);
    account(c, ptr, size);
    if size == 0 {
        libc::free(ptr);
        return null_mut();
    }
    libc::realloc(ptr, size as usize)
}

/* ------------------------------------------------------------------------- */
/* report / panic callbacks                                                  */
/* ------------------------------------------------------------------------- */

/// Compare two `js_Panic` slots without tripping the fn-pointer-comparison lint.
fn same_fn(a: Option<Panic>, b: Option<Panic>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => std::ptr::fn_addr_eq(x, y),
        _ => false,
    }
}

unsafe extern "C" fn cf_report(_J: JS, msg: *const c_char) {
    if msg.is_null() {
        pl("REPORT[<null>]");
    } else {
        libc::printf(cs("REPORT[%s]\n").as_ptr(), msg);
        fl();
    }
}

unsafe extern "C" fn cf_report2(_J: JS, msg: *const c_char) {
    libc::printf(cs("R2<%s>\n").as_ptr(), msg);
    fl();
}

unsafe extern "C" fn cf_panic(_J: JS) {
    pl("CUSTOM-PANIC");
}

unsafe extern "C" fn cf_panic2(_J: JS) {
    pl("CUSTOM-PANIC-2");
}

/* ------------------------------------------------------------------------- */
/* scripts                                                                   */
/* ------------------------------------------------------------------------- */

const PROBE: &str = concat!(
    "var r;",
    "try { zzz_undeclared_probe = 1; r = 'assigned'; } catch (e) { r = 'threw:' + e; }",
    "print('undecl', r);",
    "print('this', (function () { return typeof this; })());",
    "var o = {};",
    "Object.defineProperty(o, 'ro', { value: 7, writable: false });",
    "try { o.ro = 9; r = 'silent'; } catch (e) { r = 'threw:' + e; }",
    "print('readonly', r, o.ro);",
);

/// Runs the strict-mode probe and prints the observable effects of `J->strict`.
unsafe fn probe_strict(api: &Api, J: JS) {
    let s = cs(PROBE);
    let rc = (api.js_dostring)(J, s.as_ptr());
    pi("probe_rc", rc);
    pi("probe_top", (api.js_gettop)(J));
}

/* ========================================================================= */
/* row 1: js_newstate / js_freestate, default alloc, flags = 0               */
/* ========================================================================= */

#[test]
fn cfg01_newstate_freestate_default() {
    diff("cfg01_newstate_freestate_default", |api| unsafe {
        set_cur(api);

        /* single state, inspect the fresh defaults */
        let J = (api.js_newstate)(None, null_mut(), 0);
        pp("J", J);
        pi("top", (api.js_gettop)(J));
        pp("uctx", (api.js_getcontext)(J));
        pi("isdefined(0)", (api.js_isdefined)(J, 0));
        install_print(api, J);
        let s = cs("print('hello', 1 + 2, typeof this);");
        pi("rc", (api.js_dostring)(J, s.as_ptr()));
        pi("top_after", (api.js_gettop)(J));
        probe_strict(api, J);
        (api.js_freestate)(J);
        pl("freed");

        /* js_freestate(NULL) is a documented no-op */
        (api.js_freestate)(null_mut());
        pl("freed-null");

        /* many create/destroy cycles */
        let mut rng = Rng::new(0x1111_2222_3333_4444);
        let mut ok = 0;
        for i in 0..200 {
            let J = (api.js_newstate)(None, null_mut(), 0);
            if J.is_null() {
                pi("null_state_at", i);
                break;
            }
            ok += 1;
            /* touch the state a bit, driven by the rng so the work varies */
            let n = rng.range(4) as c_int;
            (api.js_pushnumber)(J, n as f64);
            (api.js_pushstring)(J, cs("abc").as_ptr());
            let t = (api.js_gettop)(J);
            if t != 2 {
                pi("bad_top", t);
            }
            (api.js_freestate)(J);
        }
        pi("cycles_ok", ok);
    });
}

/* ========================================================================= */
/* row 2: flags = JS_STRICT                                                  */
/* ========================================================================= */

#[test]
fn cfg02_newstate_strict() {
    diff("cfg02_newstate_strict", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), JS_STRICT);
        pp("J", J);
        pi("top", (api.js_gettop)(J));
        install_print(api, J);

        /* the flag must make the whole script strict */
        probe_strict(api, J);

        /* "use strict" inside a function of a non-strict-flag state must have
         * the same local effect */
        let s = cs("print('fn-this', (function () { \"use strict\"; return typeof this; })());");
        pi("rc_usestrict", (api.js_dostring)(J, s.as_ptr()));

        /* strict-only syntax errors */
        let bad = cs("function f(a, a) { return a; }");
        pi("rc_dupargs", (api.js_dostring)(J, bad.as_ptr()));
        let del = cs("var q = 1; print('delvar', delete q);");
        pi("rc_delvar", (api.js_dostring)(J, del.as_ptr()));
        let oct = cs("print('octal', 0123);");
        pi("rc_octal", (api.js_dostring)(J, oct.as_ptr()));

        pi("top_end", (api.js_gettop)(J));
        (api.js_freestate)(J);
        pl("freed");

        /* many strict states in a row */
        for _ in 0..50 {
            let J = (api.js_newstate)(None, null_mut(), JS_STRICT);
            pp("Ji", J);
            (api.js_freestate)(J);
        }
    });
}

/* ========================================================================= */
/* row 3: custom js_Alloc + actx, allocation trace compared                  */
/* ========================================================================= */

#[test]
fn cfg03_custom_alloc_actx() {
    diff("cfg03_custom_alloc_actx", |api| unsafe {
        set_cur(api);

        /* (a) bare newstate + freestate, first 12 events traced */
        CNT_A.reset(12);
        let J = (api.js_newstate)(Some(alloc_static), null_mut(), 0);
        pp("Ja", J);
        CNT_A.dump("a: after newstate");
        (api.js_freestate)(J);
        CNT_A.dump("a: after freestate");

        /* (b) same, but the counters are reached through actx */
        CNT_B.reset(12);
        let actx = &CNT_B as *const Counters as *mut c_void;
        let J = (api.js_newstate)(Some(alloc_actx), actx, 0);
        pp("Jb", J);
        CNT_B.dump("b: after newstate");
        install_print(api, J);
        CNT_B.dump("b: after install_print");
        let s = cs(
            "var a = []; for (var i = 0; i < 40; ++i) a.push({ k: i, s: 'value' + i }); \
             print('len', a.length, a[39].s);",
        );
        pi("rc", (api.js_dostring)(J, s.as_ptr()));
        CNT_B.dump("b: after script");
        (api.js_gc)(J, 0);
        CNT_B.dump("b: after gc");
        (api.js_freestate)(J);
        CNT_B.dump("b: after freestate");

        /* (c) strict state through the custom allocator */
        CNT_A.reset(0);
        let J = (api.js_newstate)(Some(alloc_static), null_mut(), JS_STRICT);
        pp("Jc", J);
        install_print(api, J);
        probe_strict(api, J);
        (api.js_freestate)(J);
        CNT_A.dump("c: after freestate");
    });
}

/* ========================================================================= */
/* row 4: out-of-range flag bits                                             */
/* ========================================================================= */

#[test]
fn cfg04_newstate_unknown_flags() {
    diff("cfg04_newstate_unknown_flags", |api| unsafe {
        set_cur(api);
        for &f in &[0i32, 1, 2, 3, 4, 0xFF, -1, i32::MIN, i32::MAX] {
            pi("flags", f);
            let J = (api.js_newstate)(None, null_mut(), f);
            pp("J", J);
            install_print(api, J);
            probe_strict(api, J);
            (api.js_freestate)(J);
        }

        /* property style: many random flag words */
        let mut rng = Rng::new(0x0bad_c0de_dead_beef);
        let short = cs(
            "var r; try { yyy_undeclared = 1; r = 'assigned'; } catch (e) { r = 'threw'; } \
             print(r, (function () { return typeof this; })());",
        );
        for i in 0..150 {
            let f = rng.next_u32() as i32;
            let J = (api.js_newstate)(None, null_mut(), f);
            if J.is_null() {
                pi("null_state_at", i);
                break;
            }
            install_print(api, J);
            libc::printf(cs("i=%d flags=%d rc=").as_ptr(), i as c_int, f);
            fl();
            let rc = (api.js_dostring)(J, short.as_ptr());
            pi("", rc);
            (api.js_freestate)(J);
        }
    });
}

/* ========================================================================= */
/* row 5: js_setcontext / js_getcontext                                      */
/* ========================================================================= */

static CTX_SLOT: [u64; 4] = [0xdead, 0xbeef, 0xfeed, 0xface];

#[test]
fn cfg05_context_roundtrip() {
    diff("cfg05_context_roundtrip", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);

        /* default */
        pp("initial", (api.js_getcontext)(J));

        /* non-NULL */
        let p = &CTX_SLOT as *const [u64; 4] as *mut c_void;
        (api.js_setcontext)(J, p);
        let g = (api.js_getcontext)(J);
        pp("after_set", g);
        pi("same", (g == p) as c_int);

        /* NULL again */
        (api.js_setcontext)(J, null_mut());
        let g = (api.js_getcontext)(J);
        pp("after_null", g);
        pi("is_null", g.is_null() as c_int);

        /* alternating round-trips over many distinct pointers */
        let mut rng = Rng::new(0x5555_aaaa_5555_aaaa);
        let base = &CTX_SLOT as *const [u64; 4] as usize;
        let mut mismatch = 0;
        for i in 0..500 {
            let want: *mut c_void = if i % 7 == 0 {
                null_mut()
            } else {
                (base + (rng.range(4) as usize) * 8) as *mut c_void
            };
            (api.js_setcontext)(J, want);
            let got = (api.js_getcontext)(J);
            if got != want {
                mismatch += 1;
            }
        }
        pi("mismatches", mismatch);

        /* the context survives running a script */
        (api.js_setcontext)(J, p);
        install_print(api, J);
        pi("rc", (api.js_dostring)(J, cs("print('ctx-script');").as_ptr()));
        pi("same_after_script", ((api.js_getcontext)(J) == p) as c_int);

        (api.js_freestate)(J);
        pl("freed");
    });
}

/* ========================================================================= */
/* row 6: js_setreport + js_report                                           */
/* ========================================================================= */

#[test]
fn cfg06_setreport_report() {
    diff("cfg06_setreport_report", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        install_print(api, J);

        /* default report (stderr) */
        pl("-- default report --");
        (api.js_report)(J, cs("plain message").as_ptr());
        (api.js_report)(J, cs("").as_ptr());
        (api.js_report)(J, cs("with\ttab and % percent %s").as_ptr());
        pi("rc_throw_default", (api.js_dostring)(J, cs("throw new Error('boom');").as_ptr()));
        pi("rc_syntax_default", (api.js_dostring)(J, cs("var = ;").as_ptr()));

        /* custom report */
        pl("-- custom report --");
        (api.js_setreport)(J, Some(cf_report));
        (api.js_report)(J, cs("plain message").as_ptr());
        (api.js_report)(J, cs("").as_ptr());
        pi("rc_throw_custom", (api.js_dostring)(J, cs("throw new Error('boom');").as_ptr()));
        pi("rc_type_custom", (api.js_dostring)(J, cs("null.x;").as_ptr()));
        pi("rc_ref_custom", (api.js_dostring)(J, cs("nosuchthing();").as_ptr()));
        pi("rc_range_custom", (api.js_dostring)(J, cs("(1).toFixed(99);").as_ptr()));
        pi("rc_uri_custom", (api.js_dostring)(J, cs("decodeURI('%');").as_ptr()));
        pi("rc_syntax_custom", (api.js_dostring)(J, cs("function (").as_ptr()));
        pi("rc_throwstr_custom", (api.js_dostring)(J, cs("throw 'a string';").as_ptr()));
        pi("rc_throwobj_custom", (api.js_dostring)(J, cs("throw {};").as_ptr()));
        pi("rc_deep_custom", (api.js_dostring)(J, cs("function f() { f(); } f();").as_ptr()));

        /* second custom report replaces the first */
        pl("-- second custom report --");
        (api.js_setreport)(J, Some(cf_report2));
        (api.js_report)(J, cs("switched").as_ptr());
        pi("rc_throw_r2", (api.js_dostring)(J, cs("throw new TypeError('t');").as_ptr()));

        /* NULL report => js_report is a no-op */
        pl("-- null report --");
        (api.js_setreport)(J, None);
        (api.js_report)(J, cs("swallowed").as_ptr());
        pi("rc_throw_none", (api.js_dostring)(J, cs("throw new Error('quiet');").as_ptr()));
        pl("-- done --");

        pi("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
        pl("freed");
    });

    /* property style: many generated messages through the custom report */
    diff("cfg06_report_many", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        (api.js_setreport)(J, Some(cf_report));
        let mut rng = Rng::new(0x2468_ace0_1357_bd9f);
        for i in 0..400 {
            let n = (rng.range(24) + 1) as usize;
            let mut b = String::new();
            for _ in 0..n {
                let c = (b'a' + (rng.range(26) as u8)) as char;
                b.push(c);
            }
            libc::printf(cs("i=%d n=%d\n").as_ptr(), i as c_int, n as c_int);
            fl();
            (api.js_report)(J, cs(&b).as_ptr());
        }
        (api.js_freestate)(J);
        pl("freed");
    });
}

/* ========================================================================= */
/* row 7: js_atpanic                                                         */
/* ========================================================================= */

#[test]
fn cfg07_atpanic() {
    /* (a) install / inspect / restore, no panic triggered */
    diff("cfg07_atpanic_install", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        install_print(api, J);

        let mine: Panic = cf_panic;
        let mine2: Panic = cf_panic2;

        let prev = (api.js_atpanic)(J, Some(mine));
        pp("prev_default", prev.map_or(std::ptr::null(), |f| f as *const c_void));
        pi("prev_is_some", prev.is_some() as c_int);

        let prev2 = (api.js_atpanic)(J, Some(mine2));
        pp("prev_mine", prev2.map_or(std::ptr::null(), |f| f as *const c_void));
        pi("prev2_is_mine", same_fn(prev2, Some(mine)) as c_int);

        let prev3 = (api.js_atpanic)(J, None);
        pi("prev3_is_mine2", same_fn(prev3, Some(mine2)) as c_int);

        let prev4 = (api.js_atpanic)(J, prev);
        pi("prev4_is_none", prev4.is_none() as c_int);

        let prev5 = (api.js_atpanic)(J, prev);
        pi("prev5_is_default", same_fn(prev5, prev) as c_int);
        pp("restored_default", prev5.map_or(std::ptr::null(), |f| f as *const c_void));

        /* the state still works after all that shuffling */
        pi("rc", (api.js_dostring)(J, cs("print('after-atpanic', 6 * 7);").as_ptr()));
        pi("rc_caught", (api.js_dostring)(J, cs("throw new Error('caught');").as_ptr()));

        (api.js_freestate)(J);
        pl("freed");
    });

    /* (b) custom panic actually reached: uncaught js_throw from C.
     * js_throw calls the panic hook and then abort(), so BOTH children must
     * print the same line and die with the same signal. */
    diff("cfg07_atpanic_fire", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        (api.js_setreport)(J, Some(cf_report));
        let prev = (api.js_atpanic)(J, Some(cf_panic as Panic));
        pi("prev_is_some", prev.is_some() as c_int);
        pl("throwing");
        (api.js_newerror)(J, cs("uncaught-from-c").as_ptr());
        pi("top_before_throw", (api.js_gettop)(J));
        (api.js_throw)(J);
        pl("UNREACHABLE");
    });

    /* (c) default panic reached (reports "uncaught exception", then abort) */
    diff("cfg07_atpanic_default_fire", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        (api.js_setreport)(J, Some(cf_report));
        pl("throwing");
        (api.js_pushstring)(J, cs("plain-string-exception").as_ptr());
        (api.js_throw)(J);
        pl("UNREACHABLE");
    });

    /* (d) panic hook = NULL: js_throw falls straight through to abort() */
    diff("cfg07_atpanic_null_fire", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        (api.js_setreport)(J, Some(cf_report));
        let prev = (api.js_atpanic)(J, None);
        pi("prev_is_some", prev.is_some() as c_int);
        pl("throwing");
        (api.js_pushnumber)(J, 42.0);
        (api.js_throw)(J);
        pl("UNREACHABLE");
    });
}

/* ========================================================================= */
/* row 8: js_gc(J, 0) and js_gc(J, 1)                                        */
/* ========================================================================= */

#[test]
fn cfg08_gc() {
    diff("cfg08_gc", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        (api.js_setreport)(J, Some(cf_report));
        install_print(api, J);

        pl("-- fresh state --");
        (api.js_gc)(J, 0);
        pl("gc0 done");
        (api.js_gc)(J, 1);
        (api.js_gc)(J, 1);
        (api.js_gc)(J, 0);
        (api.js_gc)(J, 1);

        pl("-- after allocating objects and strings --");
        let s = cs(
            "var keep = []; \
             for (var i = 0; i < 300; ++i) keep.push({ i: i, s: 'string number ' + i }); \
             var junk = []; \
             for (var i = 0; i < 300; ++i) junk.push({ i: i, s: 'junk string ' + i }); \
             print('lens', keep.length, junk.length);",
        );
        pi("rc", (api.js_dostring)(J, s.as_ptr()));
        (api.js_gc)(J, 1);
        pl("-- drop junk --");
        pi("rc_drop", (api.js_dostring)(J, cs("junk = null;").as_ptr()));
        (api.js_gc)(J, 1);
        (api.js_gc)(J, 1);
        pl("-- drop everything --");
        pi("rc_drop2", (api.js_dostring)(J, cs("keep = null;").as_ptr()));
        (api.js_gc)(J, 1);
        (api.js_gc)(J, 1);

        pl("-- interleaved gc while allocating --");
        for i in 0..12 {
            let src = cs("var t = []; for (var i = 0; i < 60; ++i) t.push('s' + i); t = null;");
            let rc = (api.js_dostring)(J, src.as_ptr());
            pi("rc_i", rc);
            (api.js_gc)(J, (i % 2) as c_int);
        }

        pi("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
        pl("freed");
    });

    /* gc on a state with values live on the C stack, and gc with report on a
     * bare state created many times */
    diff("cfg08_gc_cycles", |api| unsafe {
        set_cur(api);
        let mut rng = Rng::new(0x1357_9bdf_0246_8ace);
        for i in 0..40 {
            let J = (api.js_newstate)(None, null_mut(), 0);
            (api.js_setreport)(J, Some(cf_report));
            install_print(api, J);
            let n = rng.range(50) + 1;
            let src = format!(
                "var a = []; for (var i = 0; i < {}; ++i) a.push([i, 'x' + i]); print('n', a.length);",
                n
            );
            libc::printf(cs("iter=%d n=%d\n").as_ptr(), i as c_int, n as c_int);
            fl();
            pi("rc", (api.js_dostring)(J, cs(&src).as_ptr()));
            (api.js_gc)(J, 1);
            (api.js_pushobject)(J, (api.js_toobject)(J, -1));
            (api.js_gc)(J, 1);
            (api.js_pop)(J, (api.js_gettop)(J));
            (api.js_gc)(J, 1);
            (api.js_freestate)(J);
        }
    });
}

/* ========================================================================= */
/* row 9: js_setlimit runlimit                                               */
/* ========================================================================= */

#[test]
fn cfg09_setlimit_runlimit() {
    diff("cfg09_setlimit_runlimit", |api| unsafe {
        set_cur(api);

        /* (a) under the limit */
        let J = mkstate(api, 0);
        (api.js_setreport)(J, Some(cf_report));
        (api.js_setlimit)(J, 2_000_000, 0);
        pl("-- under limit --");
        let ok = cs("var n = 0; for (var i = 0; i < 1000; ++i) n += i; print('sum', n);");
        pi("rc_ok", (api.js_dostring)(J, ok.as_ptr()));
        (api.js_freestate)(J);

        /* (b) over the limit: unbounded loop, stopped by the runlimit */
        let J = mkstate(api, 0);
        (api.js_setreport)(J, Some(cf_report));
        (api.js_setlimit)(J, 2_000_000, 0);
        pl("-- over limit --");
        let bad = cs("var n = 0; for (;;) n += 1; print('never', n);");
        pi("rc_bad", (api.js_dostring)(J, bad.as_ptr()));
        pi("top_after", (api.js_gettop)(J));
        /* the counter stays at 1, so the next script fails immediately */
        pi("rc_after", (api.js_dostring)(J, cs("print('tiny');").as_ptr()));
        /* clearing the limit revives the state */
        (api.js_setlimit)(J, 0, 0);
        pi("rc_revived", (api.js_dostring)(J, cs("print('revived', 1 + 1);").as_ptr()));
        (api.js_freestate)(J);

        /* (c) runlimit == 1 => the very first instruction errors */
        let J = mkstate(api, 0);
        (api.js_setreport)(J, Some(cf_report));
        (api.js_setlimit)(J, 1, 0);
        pl("-- runlimit 1 --");
        pi("rc_one", (api.js_dostring)(J, cs("print('nope');").as_ptr()));
        (api.js_freestate)(J);

        /* (d) negative runlimit means "no limit" (only > 0 is checked) */
        let J = mkstate(api, 0);
        (api.js_setreport)(J, Some(cf_report));
        (api.js_setlimit)(J, -5, 0);
        pl("-- negative runlimit --");
        let loop_ = cs("var n = 0; for (var i = 0; i < 200; ++i) n += i; print('sum', n);");
        pi("rc_neg", (api.js_dostring)(J, loop_.as_ptr()));
        (api.js_setlimit)(J, i32::MAX, 0);
        pi("rc_max", (api.js_dostring)(J, loop_.as_ptr()));
        (api.js_freestate)(J);
        pl("done");
    });

    /* property style: sweep the exact instruction budget so that the boundary
     * between "completes" and "script ran too long" is compared. */
    diff("cfg09_runlimit_sweep", |api| unsafe {
        set_cur(api);
        let mut rng = Rng::new(0x00c0_ffee_0bad_f00d);
        let src = cs(
            "var n = 0; for (var i = 0; i < 40; ++i) n += i; \
             function f(x) { return x * 2; } n = f(n); print('sum', n);",
        );
        for i in 0..200 {
            let lim = (rng.range(900) + 1) as c_int;
            let J = mkstate(api, 0);
            (api.js_setreport)(J, Some(cf_report));
            (api.js_setlimit)(J, lim, 0);
            libc::printf(cs("i=%d lim=%d\n").as_ptr(), i as c_int, lim);
            fl();
            let rc = (api.js_dostring)(J, src.as_ptr());
            pi("rc", rc);
            pi("top", (api.js_gettop)(J));
            (api.js_freestate)(J);
        }
    });
}

/* ========================================================================= */
/* row 10: js_setlimit memlimit + direct js_malloc/realloc/free/strdup       */
/* ========================================================================= */

#[test]
fn cfg10_setlimit_memlimit() {
    /* (a) memlimit not hit */
    diff("cfg10_memlimit_ok", |api| unsafe {
        set_cur(api);
        let J = mkstate(api, 0);
        (api.js_setreport)(J, Some(cf_report));
        (api.js_setlimit)(J, 0, 1 << 26);
        let s = cs("var a = []; for (var i = 0; i < 100; ++i) a.push('s' + i); print('n', a.length);");
        pi("rc_ok", (api.js_dostring)(J, s.as_ptr()));
        pi("top", (api.js_gettop)(J));
        /* negative memlimit means "no limit" */
        (api.js_setlimit)(J, 0, -1);
        pi("rc_neg", (api.js_dostring)(J, s.as_ptr()));
        (api.js_setlimit)(J, 0, i32::MAX);
        pi("rc_max", (api.js_dostring)(J, s.as_ptr()));
        (api.js_freestate)(J);
        pl("freed");
    });

    /* (b) memlimit hit inside a script (the throw is caught by js_dostring) */
    diff("cfg10_memlimit_hit", |api| unsafe {
        set_cur(api);
        for &lim in &[1i32, 2, 16, 64, 512, 4096, 65536] {
            let J = mkstate(api, 0);
            (api.js_setreport)(J, Some(cf_report));
            pi("memlimit", lim);
            (api.js_setlimit)(J, 0, lim);
            let s = cs(
                "var a = []; for (var i = 0; i < 400; ++i) a.push({ k: i, s: 'long string value ' + i }); \
                 print('n', a.length);",
            );
            pi("rc", (api.js_dostring)(J, s.as_ptr()));
            pi("top", (api.js_gettop)(J));
            /* the limit is exhausted: a second run must fail too */
            pi("rc2", (api.js_dostring)(J, s.as_ptr()));
            (api.js_freestate)(J);
            pl("--");
        }
    });

    /* (c) direct js_malloc / js_realloc / js_free / js_strdup, limit not hit */
    diff("cfg10_direct_alloc", |api| unsafe {
        set_cur(api);
        let J = mkstate(api, 0);
        (api.js_setreport)(J, Some(cf_report));
        (api.js_setlimit)(J, 0, 1 << 24);

        let p = (api.js_malloc)(J, 64);
        pp("malloc64", p);
        let p = (api.js_realloc)(J, p, 4096);
        pp("realloc4096", p);
        let p = (api.js_realloc)(J, p, 8);
        pp("realloc8", p);
        (api.js_free)(J, p);
        pl("freed p");
        (api.js_free)(J, null_mut());
        pl("freed null");

        let d = (api.js_strdup)(J, cs("hello, mujs").as_ptr());
        ps("strdup", d as *const c_char);
        (api.js_free)(J, d as *mut c_void);

        let e = (api.js_strdup)(J, cs("").as_ptr());
        ps("strdup_empty", e as *const c_char);
        (api.js_free)(J, e as *mut c_void);

        /* many random sizes, all comfortably below the limit */
        let mut rng = Rng::new(0x7777_8888_9999_aaaa);
        let mut live: Vec<*mut c_void> = Vec::new();
        for i in 0..300 {
            let n = (rng.range(200) + 1) as c_int;
            let q = (api.js_malloc)(J, n);
            if q.is_null() {
                pi("null_at", i);
                break;
            }
            live.push(q);
        }
        pi("live", live.len() as c_int);
        for q in live.drain(..) {
            (api.js_free)(J, q);
        }
        pl("all freed");

        /* the state still runs scripts */
        pi("rc", (api.js_dostring)(J, cs("print('still-alive', 3 + 4);").as_ptr()));
        (api.js_freestate)(J);
        pl("freed state");
    });

    /* (d) direct js_malloc overrunning the memlimit: the "out of memory"
     * throw has no try frame, so the panic hook runs and abort() follows.
     * Both children must print the same trace and die identically. */
    diff("cfg10_direct_alloc_overrun", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        (api.js_setreport)(J, Some(cf_report));
        (api.js_setlimit)(J, 0, 1000);
        for i in 0..40 {
            pi("i", i);
            let q = (api.js_malloc)(J, 100);
            pp("q", q);
        }
        pl("UNREACHABLE");
    });

    /* (e) js_malloc(size >= memlimit) trips on the very first call */
    diff("cfg10_direct_alloc_overrun2", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        (api.js_setreport)(J, Some(cf_report));
        (api.js_atpanic)(J, Some(cf_panic as Panic));
        (api.js_setlimit)(J, 0, 128);
        pl("about to malloc 4096");
        let q = (api.js_malloc)(J, 4096);
        pp("q", q);
        pl("UNREACHABLE");
    });

    /* (f) js_strdup over the memlimit */
    diff("cfg10_strdup_overrun", |api| unsafe {
        set_cur(api);
        let J = (api.js_newstate)(None, null_mut(), 0);
        (api.js_setreport)(J, Some(cf_report));
        (api.js_atpanic)(J, Some(cf_panic2 as Panic));
        (api.js_setlimit)(J, 0, 8);
        pl("about to strdup");
        let d = (api.js_strdup)(J, cs("0123456789abcdef").as_ptr());
        ps("strdup", d as *const c_char);
        pl("UNREACHABLE");
    });

    /* property style: sweep memlimits around the point where a fixed script
     * starts to fail. */
    diff("cfg10_memlimit_sweep", |api| unsafe {
        set_cur(api);
        let mut rng = Rng::new(0xfeed_face_cafe_b0ba);
        let src = cs("var s = ''; for (var i = 0; i < 20; ++i) s += i; print('len', s.length);");
        for i in 0..200 {
            let lim = (rng.range(20000) + 1) as c_int;
            let J = mkstate(api, 0);
            (api.js_setreport)(J, Some(cf_report));
            (api.js_setlimit)(J, 0, lim);
            libc::printf(cs("i=%d lim=%d\n").as_ptr(), i as c_int, lim);
            fl();
            let rc = (api.js_dostring)(J, src.as_ptr());
            pi("rc", rc);
            pi("top", (api.js_gettop)(J));
            (api.js_freestate)(J);
        }
    });
}
