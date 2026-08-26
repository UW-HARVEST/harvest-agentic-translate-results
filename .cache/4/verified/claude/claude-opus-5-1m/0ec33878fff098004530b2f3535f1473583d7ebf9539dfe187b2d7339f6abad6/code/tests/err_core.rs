//! Differential tests for the ERROR SURFACE of the core runtime.
//! Covers ERRORS.md rows 1-258:
//!   jsrun.c 1-132, jsstate.c 133-152, jserror.c 153-166, jsvalue.c 167-204,
//!   jsproperty.c 205-220, jsintern.c 221-230, jsgc.c 231-258.
//!
//! Every call goes through the two `.so` exports via `tests/common/mod.rs`.
//! Anything that can throw is driven from inside a cfunction invoked with
//! `js_pcall` (the `probe()` pattern), so a `js_throw` always finds a handler
//! instead of reaching `abort()` with `trytop == 0`.
//!
//! Set `MUJS_DUMP=1` to print every transcript.
//!
//! ==========================================================================
//! ROWS DELIBERATELY NOT DRIVEN, and why (each verified against the C source)
//! ==========================================================================
//!
//! * row 41 -- jsrun.c:550-552.  The `js_try` handler inside
//!   `jsR_unflattenarray` sets `obj->properties = NULL` and rethrows.  A NULL
//!   property tree is then dereferenced unconditionally by jsproperty.c:48
//!   (`lookup`), jsgc.c:101 (`jsG_scanobject`) and jsgc.c:35
//!   (`jsG_freeobject`), so reaching that handler poisons the state and every
//!   later property access / `js_gc` / `js_freestate` performs a NULL
//!   dereference.  Undefined behaviour; not testable in-process.
//!
//! * rows 49, 50, 52 -- jsrun.c:673/674/678.  Live `assert()`s inside the
//!   static `jsR_setarrayindex`.  Both call sites (jsrun.c:722 `jsR_setproperty`
//!   and jsrun.c:806 `jsR_setindex`) first test
//!   `u.a.simple && k >= 0 && k <= flat_length`, so the asserts cannot be
//!   violated through any exported entry point.  `jsR_setarrayindex` itself is
//!   `static` and not exported.
//!
//! * row 51 -- jsrun.c:675 `newlen > JS_ARRAYLIMIT` in `jsR_setarrayindex`.
//!   Reaching it needs `flat_length >= 1<<26`, i.e. 64M live `js_Value`s = 1 GiB
//!   of flat array data, because `jsR_setarrayindex` is only ever called with
//!   `k <= flat_length` and `flat_length` grows one element at a time
//!   (jsrun.c:678 asserts exactly that).  Not a bounded-work test.  The sibling
//!   "array too large" site that *is* reachable (jsrun.c:708, row 54) is driven
//!   in `t_array_length_errors`.
//!
//! * row 40 -- jsrun.c:541 `js_pushrune` with `rune < 0`.  Unreachable: the only
//!   caller is jsrun.c:596, guarded by `k >= 0 && k < obj->u.s.length`, and
//!   `u.s.length` is `js_utflen()` (jsstring.c:49) which uses *exactly* the same
//!   rune accounting as `js_runeat` (jsstring.c:20) -- both count a rune
//!   >= 0x10000 as two positions and stop at the NUL.  So every `k` in
//!   `[0, u.s.length)` is addressable and `js_runeat` never returns EOF there.
//!   `t_string_wrapper_indices` drives the whole in-range/out-of-range boundary
//!   to show the two libraries agree on it anyway.
//!
//! * row 66 -- jsrun.c:792 `goto readonly` from the ref returned by
//!   `jsV_setproperty`.  Unreachable: that line is only reached when
//!   `!ref || !own` held at jsrun.c:780, i.e. `name` is NOT an own property of
//!   `obj`.  `jsV_setproperty` (jsproperty.c:221) then either inserts a fresh
//!   `newproperty` (jsproperty.c:35 sets `atts = 0`) or, for a non-extensible
//!   object, returns `lookup(obj->properties, name)` which is NULL for exactly
//!   the same reason `own` was 0 (`jsV_getpropertyx` uses the same `lookup`).
//!   So the ref reaching jsrun.c:790 can never carry `JS_READONLY`.  The
//!   READONLY forks that ARE reachable (jsrun.c:775 row 63 and jsrun.c:800
//!   row 67) are driven in `t_setproperty_readonly_forks`.
//!
//! * row 101 -- jsrun.c:1160 `jsR_savescope` env-stack overflow.  Unreachable:
//!   `envtop <= tracetop` is invariant, because the only `jsR_savescope` call
//!   sites (jsrun.c:1176, 1201, 1243) all sit inside a `js_call` branch that
//!   ran `jsR_pushtrace` first (jsrun.c:1315/1322/1326), and `js_throw`
//!   restores both counters together (jsrun.c:1471-1472).  `jsR_pushtrace`
//!   trips at `tracetop + 1 == JS_ENVLIMIT` (tracetop == 1023) while
//!   `jsR_savescope` needs `envtop + 1 >= JS_ENVLIMIT` (envtop == 1023), so the
//!   `js_error(J, "call stack overflow")` of row 105 always fires first.  Row
//!   105 is driven in `t_call_stack_overflow`; the *other* `js_stackoverflow`
//!   caller (CHECKSTACK) is driven exhaustively in
//!   `t_value_stack_overflow_matrix`.
//!
//! * row 108 -- jsrun.c:1314.  ERRORS.md itself notes it is unreachable given
//!   row 107: `js_iscallable` (jsrun.c:244) accepts exactly the three classes
//!   the `if` chain dispatches on.
//!
//! * row 143 -- jsstate.c:102, `js_tryboolean`'s setjmp path.  Unreachable:
//!   `js_toboolean` (jsrun.c:318) is `jsV_toboolean` (jsvalue.c:152), a total
//!   switch over `v->t.type` that only reads `shrstr[0]` / `litstr[0]` /
//!   `memstr->p[0]` / `u.boolean` / `u.number`.  It allocates nothing and calls
//!   nothing, so ToBoolean can never throw.  The `js_ptry` half of
//!   `js_tryboolean` (row 142) IS reachable and is driven by `t_try_limits`, and
//!   `t_ffi_try_defaults` shows `js_tryboolean` returning the real value even
//!   for receivers whose `toString` / `valueOf` throw.
//!
//! * rows 147, 148 -- jsstate.c:191/192.  `assert(sizeof(js_Value) == 16)` and
//!   `assert(soffsetof(js_Value, t.type) == 15)` are properties of the BUILD,
//!   not of any input; they hold in this build (both libraries agree on every
//!   `js_pushlstring` short-string boundary, see `t_shrstr_boundary`).  No input
//!   can make them fail.
//!
//! * row 218 -- jsproperty.c:325 `assert(!obj->u.a.simple)` in
//!   `jsV_resizearray`.  The only in-library caller (jsrun.c:715) is inside an
//!   `else` whose `if` tested `obj->u.a.simple`, so the assert is unreachable
//!   through the public API.  `jsV_resizearray` IS exported, so it can be called
//!   directly on a simple array -- but that is a deliberate invariant violation
//!   whose C outcome is `abort()`, and the Rust translation carries no
//!   `assert!`s at all (it matches an `-DNDEBUG` build), so such a call is a
//!   known, documented, non-input-driven difference rather than a bug.
//!
//! * row 256 -- jsgc.c:255 `100*gtot/ntot` with `ntot == 0`.  Unreachable from
//!   `js_newstate`: `ntot = nenv+nfun+nobj+nstr+nprop` and `js_newstate`
//!   unconditionally creates `J->R`, `J->G`, the global environment and the
//!   whole builtin tree before returning (jsstate.c:229-234), so `J->gcobj` and
//!   `J->gcenv` are never both empty for any state a caller can hold.
//!   `t_gc_sweep_report` asserts `ntot > 0` for every state it collects.
//!
//! * row 257 -- jsgc.c:255 report line longer than 255 bytes.  Unreachable:
//!   the format string is 62 fixed bytes plus 11 `%d` conversions of `unsigned
//!   int` values, i.e. at most 62 + 11*11 = 183 bytes.  `t_gc_sweep_report`
//!   compares the full report text for many states instead.

#![allow(unused_unsafe, clippy::too_many_arguments)]

mod common;
use common::*;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_int, c_void, CString};
use std::rc::Rc;

/* ----------------------------------------------------------- name literals */

macro_rules! cn {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

const N_JOB: *const c_char = cn!("job");
const N_TAG: *const c_char = cn!("udtag");
const N_TAG2: *const c_char = cn!("othertag");
const PAYLOAD: *const c_char = cn!("PAY");

/* ------------------------------------------------------------- diff driver */

/// Run `f` against the C library and then the Rust library with a fresh output
/// buffer each time, and assert the two transcripts are byte-identical.
fn diff2(tag: &str, f: impl Fn(&Lib) -> String) -> String {
    let p = libs();
    let a = {
        out_clear();
        set_cur(&p.c);
        let r = f(&p.c);
        format!("{r}\n--out--\n{}", out_take())
    };
    let b = {
        out_clear();
        set_cur(&p.rs);
        let r = f(&p.rs);
        format!("{r}\n--out--\n{}", out_take())
    };
    if std::env::var_os("MUJS_DUMP").is_some() {
        println!("=== [{tag}] ===\n{a}");
    }
    if a != b {
        let (ab, bb) = (a.as_bytes(), b.as_bytes());
        let mut i = 0;
        while i < ab.len() && i < bb.len() && ab[i] == bb[i] {
            i += 1;
        }
        let lo = i.saturating_sub(200);
        panic!(
            "divergence in [{tag}] at byte {i}\n\
             ...C  : {:?}\n...RS : {:?}\n--- full C ---\n{a}\n--- full RS ---\n{b}",
            String::from_utf8_lossy(&ab[lo..(i + 240).min(ab.len())]),
            String::from_utf8_lossy(&bb[lo..(i + 240).min(bb.len())]),
        );
    }
    a
}

/* ---------------------------------------------------------------- helpers */

unsafe fn drain_to(l: &Lib, j: JS, base: c_int) {
    let t = l.js_gettop(j);
    if t > base {
        l.js_pop(j, t - base);
    }
}

/// Everything observable about the value at `idx` WITHOUT calling anything that
/// can throw: `js_typeof` / `js_type` / the predicates / `js_toboolean`
/// (`jsV_toboolean`, jsvalue.c:152, has no throwing branch) plus the two
/// protected stringifiers.  Enough to tell a bare `JS_TLITSTR` (row 1-4) from a
/// real `JS_CERROR` object apart.
unsafe fn vshape(l: &Lib, j: JS, idx: c_int) -> String {
    let mut s = format!(
        "ty={} t={} err={} str={} obj={} undef={} null={} b={}",
        from_c(l.js_typeof(j, idx)),
        l.js_type(j, idx),
        l.pred("js_iserror", j, idx),
        l.pred("js_isstring", j, idx),
        l.pred("js_isobject", j, idx),
        l.pred("js_isundefined", j, idx),
        l.pred("js_isnull", j, idx),
        l.js_toboolean(j, idx),
    );
    // `js_tostring` REWRITES the slot it is handed for numbers (jsvalue.c:344)
    // and objects (jsvalue.c:360), and `js_torepr` does the same (jsrepr.c:271),
    // so stringify COPIES and leave the original value intact.
    l.js_copy(j, idx);
    s.push_str(&format!(
        " s={:?}",
        from_c(l.js_trystring(j, -1, cn!("<nostring>")))
    ));
    l.js_pop(j, 1);
    l.js_copy(j, idx);
    s.push_str(&format!(" r={:?}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
    l.js_pop(j, 1);
    s
}

/// `vshape` minus the `js_tryrepr` field.  `js_tryrepr` (jsrepr.c:278) installs
/// its handler with a bare `js_try` and NO `js_ptry` guard -- unlike
/// `js_trystring` (jsstate.c:50) -- so at `trytop == JS_TRYLIMIT` it throws
/// instead of returning the caller's default.  Use this wherever the try stack
/// is deliberately full.
unsafe fn vshape_notry(l: &Lib, j: JS, idx: c_int) -> String {
    let mut s = format!(
        "ty={} t={} err={} str={} obj={} undef={} null={} b={}",
        from_c(l.js_typeof(j, idx)),
        l.js_type(j, idx),
        l.pred("js_iserror", j, idx),
        l.pred("js_isstring", j, idx),
        l.pred("js_isobject", j, idx),
        l.pred("js_isundefined", j, idx),
        l.pred("js_isnull", j, idx),
        l.js_toboolean(j, idx),
    );
    l.js_copy(j, idx);
    s.push_str(&format!(
        " s={:?}",
        from_c(l.js_trystring(j, -1, cn!("<nostring>")))
    ));
    l.js_pop(j, 1);
    s
}

/* ------------------------------------------------------- the generic probe */

thread_local! {
    /// The job the next `probe()` runs inside a protected cfunction frame.
    /// `Rc` (not `Box`) so the borrow can be released BEFORE the job runs: a
    /// job that throws `longjmp`s out and would otherwise leave the `RefCell`
    /// permanently borrowed.
    static JOB: RefCell<Option<Rc<dyn Fn(&Lib, JS) -> String>>> =
        const { RefCell::new(None) };
}

unsafe extern "C" fn cf_job(j: JS) {
    let l = cur();
    let f = JOB.with(|b| b.borrow().clone()).expect("no job set");
    let s = f(l, j);
    let cs = cstr(&s);
    l.js_pushstring(j, cs.as_ptr());
}

/// Run `f` inside `js_pcall` and return a transcript of `(rc, thrown-or-result,
/// stack delta)`.  NOTHING that can throw may be called outside this.
unsafe fn probe(l: &Lib, j: JS, f: Rc<dyn Fn(&Lib, JS) -> String>) -> String {
    JOB.with(|b| *b.borrow_mut() = Some(f));
    let base = l.js_gettop(j);
    l.js_newcfunction(j, Some(cf_job), N_JOB, 0);
    l.js_pushundefined(j);
    let rc = l.js_pcall(j, 0);
    let v = vshape(l, j, -1);
    l.js_pop(j, 1);
    let after = l.js_gettop(j);
    let r = format!("[rc={rc} {v} top {base}->{after}]");
    drain_to(l, j, base);
    r
}

macro_rules! job {
    (|$l:ident, $j:ident| $body:block) => {
        std::rc::Rc::new(move |$l: &Lib, $j: JS| -> String { unsafe { $body } })
            as std::rc::Rc<dyn Fn(&Lib, JS) -> String>
    };
}

thread_local! {
    /// Progress breadcrumbs a job leaves behind, so that a job which throws
    /// (and therefore never returns a transcript) still records HOW FAR it got.
    static MARK: RefCell<String> = const { RefCell::new(String::new()) };
}

fn mark(s: &str) {
    MARK.with(|m| m.borrow_mut().push_str(s));
}

/// Replace (rather than append to) the breadcrumb, for loops.
fn mark_set(s: &str) {
    MARK.with(|m| {
        let mut b = m.borrow_mut();
        b.clear();
        b.push_str(s);
    });
}

fn mark_take() -> String {
    MARK.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

/// One-shot: fresh state, run `f` protected, free the state.
fn probe_state(tag: &str, flags: c_int, f: impl Fn() -> Rc<dyn Fn(&Lib, JS) -> String>) -> String {
    diff2(tag, move |l| unsafe {
        let _ = mark_take();
        let j = new_state(l, flags);
        let r = probe(l, j, f());
        let m = mark_take();
        let t = l.js_gettop(j);
        l.js_gc(j, 0);
        l.js_freestate(j);
        format!("{r} mark={m} endtop={t}")
    })
}

/* =========================================================================
 *  Rows 1-4: the four bare-string throws.
 *  jsrun.c:19/27/35/43 push a JS_TLITSTR (NOT an Error object) and js_throw.
 * ========================================================================= */

/// The four literal-string throws, observed both from JS (`catch(e)`) and from
/// the C API (`js_pcall`'s error slot).  A JS_TLITSTR is `typeof "string"`,
/// `e instanceof Error === false` and `e.message === undefined`.
#[test]
fn t_bare_string_throw_shapes() {
    // In JS: the shape helper reports typeof / instanceof / message / identity.
    const SHAPE: &str = "function shape(e){ \
        var nil = (e === null || e === undefined); \
        return typeof e + '|' + (e instanceof Error) + '|' + (e === null) + '|' + \
        String(e) + '|' + (nil ? 'nil' : typeof e.message) + '|' + \
        (nil ? 'nil' : String(e.name)) + '|' + \
        (typeof e === 'string' ? e.length : -1) }";

    // row 2 + row 9: value-stack overflow -> "stack overflow".
    // Reached WITHOUT native recursion by making one call push more than
    // JS_STACKSIZE arguments (jsfunction.c `Fp_apply` pushes `length` values,
    // each through js_getindex -> CHECKSTACK).
    for n in [10usize, 4000, 4090, 4096, 5000, 20000] {
        diff_dostring(
            0,
            &format!(
                "{SHAPE} try {{ print('r', Math.max.apply(null, new Array({n}))) }} \
                 catch (e) {{ print('so', shape(e)) }}"
            ),
        );
        diff_dostring(
            JS_STRICT,
            &format!(
                "{SHAPE} try {{ print('r', Math.min.apply(null, new Array({n}))) }} \
                 catch (e) {{ print('so2', shape(e)) }}"
            ),
        );
    }
    // row 4 + row 122: instruction budget -> "script ran too long"
    diff2("runlimit bare string", |l| unsafe {
        let j = new_state(l, 0);
        l.js_setlimit(j, 400, 0);
        let cs = cstr(&format!(
            "{SHAPE} try {{ var s=0; for(;;) ++s }} catch (e) {{ print('rl', shape(e)) }}"
        ));
        let rc = l.js_dostring(j, cs.as_ptr());
        l.js_freestate(j);
        format!("rc={rc}")
    });
    // row 3 + row 5: memlimit -> "out of memory"
    for lim in [1, 2, 17, 64, 500, 4096, 65536] {
        diff2(&format!("memlimit {lim} bare string"), move |l| unsafe {
            let j = new_state(l, 0);
            l.js_setlimit(j, 0, lim);
            let cs = cstr(&format!(
                "{SHAPE} try {{ var a=[]; for(var i=0;i<20000;++i) a.push('x'+i) }} \
                 catch (e) {{ print('oom', shape(e)) }}"
            ));
            let rc = l.js_dostring(j, cs.as_ptr());
            l.js_freestate(j);
            format!("rc={rc}")
        });
    }
    // row 1 + row 115: exception-stack overflow -> "exception stack overflow"
    for n in [60usize, 61, 62, 63, 64, 65, 66, 70] {
        let mut src = String::from(SHAPE);
        src.push(' ');
        for _ in 0..n {
            src.push_str("try{");
        }
        src.push_str("print('body reached')");
        for i in 0..n {
            src.push_str(&format!("}}catch(e){{print('c{i}', shape(e))}}"));
        }
        diff_dostring(0, &src);
        diff_dostring(JS_STRICT, &src);
    }

    // and from the C API: each of the four, with the thrown value inspected
    // through the non-throwing predicates.
    probe_state("bare: stack overflow via CHECKSTACK", 0, || {
        job!(|l, j| {
            let pushnum: unsafe extern "C" fn(JS, f64) = l.raw2("js_pushnumber");
            let mut i = 0;
            while i < 8192 {
                pushnum(j, i as f64);
                i += 1;
            }
            "no overflow".to_string()
        })
    });
    probe_state("bare: out of memory via js_malloc", 0, || {
        job!(|l, j| {
            l.js_setlimit(j, 0, 64);
            let p = l.js_malloc(j, 4096);
            format!("malloc returned {}", !p.is_null())
        })
    });
    probe_state("bare: exception stack overflow via js_savetry", 0, || {
        job!(|l, j| {
            // js_savetry pushes a frame; balance it immediately so no later
            // throw can longjmp into a jmp_buf we never setjmp'd.
            let mut n = 0;
            loop {
                let b = l.js_savetry(j);
                l.js_endtry(j);
                n += 1;
                if n > 200 || b.is_null() {
                    break;
                }
            }
            format!("savetry rounds={n}")
        })
    });
}

/* =========================================================================
 *  Rows 5-8: js_malloc / js_realloc, memlimit and a failing host allocator.
 * ========================================================================= */

/// Only fails for one magic size, so no *internal* allocation is ever affected
/// and the state stays consistent up to the deliberate failure.
const MAGIC_FAIL_SIZE: c_int = 999_983;

#[repr(C)]
struct FailCtx {
    live: i64,
    nfail: u64,
}

unsafe extern "C" fn magic_alloc(actx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    extern "C" {
        fn free(p: *mut c_void);
        fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    }
    let cx = if actx.is_null() {
        std::ptr::null_mut()
    } else {
        actx as *mut FailCtx
    };
    if size == 0 {
        if !ptr.is_null() && !cx.is_null() {
            (*cx).live -= 1;
        }
        free(ptr);
        return std::ptr::null_mut();
    }
    if size == MAGIC_FAIL_SIZE {
        if !cx.is_null() {
            (*cx).nfail += 1;
        }
        return std::ptr::null_mut();
    }
    let p = realloc(ptr, size as usize);
    if !p.is_null() && ptr.is_null() && !cx.is_null() {
        (*cx).live += 1;
    }
    p
}

#[test]
fn t_malloc_realloc_limits() {
    let mut rng = Rng::new(0x5A1E_0005);
    // rows 5 / 7: `size >= memlimit` in js_malloc / js_realloc
    let mut cases: Vec<(c_int, c_int)> = vec![
        (1, 1),
        (1, 0),
        (1, 2),
        (16, 15),
        (16, 16),
        (16, 17),
        (1024, 1023),
        (1024, 1024),
        (1024, 1025),
        (i32::MAX, i32::MAX),
        (i32::MAX, 1),
    ];
    for _ in 0..40 {
        let m = 1 + rng.below(4096) as c_int;
        cases.push((m, rng.range(-8, 8192) as c_int));
    }
    for (memlimit, size) in cases {
        probe_state(&format!("js_malloc memlimit={memlimit} size={size}"), 0, move || {
            job!(|l, j| {
                l.js_setlimit(j, 0, memlimit);
                let p = l.js_malloc(j, size);
                let r = format!("malloc(size={size}) null={}", p.is_null());
                if !p.is_null() {
                    l.js_free(j, p);
                }
                r
            })
        });
        probe_state(&format!("js_realloc memlimit={memlimit} size={size}"), 0, move || {
            job!(|l, j| {
                let p0 = l.js_malloc(j, 8);
                l.js_setlimit(j, 0, memlimit);
                let p = l.js_realloc(j, p0, size);
                let r = format!("realloc(size={size}) null={}", p.is_null());
                if !p.is_null() {
                    l.js_setlimit(j, 0, 0);
                    l.js_free(j, p);
                }
                r
            })
        });
    }

    // rows 6 / 8: the host allocator returns NULL
    diff2("js_malloc host alloc NULL", |l| unsafe {
        let mut cx = FailCtx { live: 0, nfail: 0 };
        set_cur(l);
        let j = l.js_newstate(
            Some(magic_alloc),
            &mut cx as *mut FailCtx as *mut c_void,
            0,
        );
        assert!(!j.is_null(), "{}: state with magic allocator", l.name);
        l.js_setreport(j, Some(report_cb));
        let r = probe(
            l,
            j,
            job!(|l, j| {
                let p = l.js_malloc(j, MAGIC_FAIL_SIZE);
                format!("malloc null={}", p.is_null())
            }),
        );
        let r2 = probe(
            l,
            j,
            job!(|l, j| {
                let p0 = l.js_malloc(j, 32);
                let p = l.js_realloc(j, p0, MAGIC_FAIL_SIZE);
                format!("realloc null={}", p.is_null())
            }),
        );
        l.js_freestate(j);
        format!("malloc={r} realloc={r2} nfail={}", cx.nfail)
    });
}

/* =========================================================================
 *  Rows 9-21, 33-35, 124: CHECKSTACK / JS_STACKSIZE == 4096.
 * ========================================================================= */

thread_local! {
    static PUSHN: Cell<c_int> = const { Cell::new(0) };
}

/// The pushing entry points whose CHECKSTACK is a distinct ERRORS.md row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Push {
    Value,       // row 10
    Undefined,   // row 11
    Null,        // row 12
    Boolean,     // row 13
    Number,      // row 14
    String,      // row 16
    LString,     // row 18
    Literal,     // row 19
    Object,      // row 20 (js_pushglobal -> js_pushobject)
    Current,     // row 21
    Copy,        // row 33
    Dup,         // row 34
    Dup2,        // row 35
    GetIndexFlat, // row 10 again, through jsR_hasindex's js_pushvalue
    GetLocalLw,  // row 124 (OP_GETLOCAL, lightweight)
}

const ALL_PUSH: &[Push] = &[
    Push::Value,
    Push::Undefined,
    Push::Null,
    Push::Boolean,
    Push::Number,
    Push::String,
    Push::LString,
    Push::Literal,
    Push::Object,
    Push::Current,
    Push::Copy,
    Push::Dup,
    Push::Dup2,
    Push::GetIndexFlat,
    Push::GetLocalLw,
];

/// Find the exact number of `js_pushnumber`s a fresh cfunction frame can take.
fn stack_capacity(l: &Lib) -> c_int {
    unsafe {
        let j = new_state(l, 0);
        PUSHN.with(|c| c.set(-1));
        let _ = probe(
            l,
            j,
            job!(|l, j| {
                let pushnum: unsafe extern "C" fn(JS, f64) = l.raw2("js_pushnumber");
                let mut i: c_int = 0;
                while i < 8192 {
                    PUSHN.with(|c| c.set(i));
                    pushnum(j, i as f64);
                    i += 1;
                }
                "no overflow".to_string()
            }),
        );
        l.js_freestate(j);
        PUSHN.with(|c| c.get())
    }
}

#[test]
fn t_value_stack_overflow_matrix() {
    let p = libs();
    let cc = stack_capacity(&p.c);
    let cr = stack_capacity(&p.rs);
    assert_eq!(cc, cr, "value stack capacity differs (C {cc} vs RS {cr})");
    assert!(cc > 4000 && cc < 4096, "unexpected capacity {cc}");

    for &op in ALL_PUSH {
        // delta 0 => the op itself is the push that must overflow; larger deltas
        // give the multi-slot ops (js_dup2, js_call) their headroom back, so the
        // sweep pins down the EXACT slot count each entry point needs.
        for delta in 0..8i32 {
            let fill = cc - delta;
            probe_state(&format!("checkstack {op:?} cap-{delta}"), 0, move || {
                job!(|l, j| {
                    let pushnum: unsafe extern "C" fn(JS, f64) = l.raw2("js_pushnumber");
                    // things that must exist BEFORE the stack is full
                    let mut prep = String::new();
                    match op {
                        Push::GetIndexFlat => {
                            l.js_newarray(j);
                            l.js_pushnumber(j, 7.0);
                            l.js_setindex(j, -2, 0);
                            // leave the array at relative index 1
                        }
                        Push::GetLocalLw => {
                            let cs = cstr("(function(a){return a})");
                            let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
                            prep.push_str(&format!("load={rc} "));
                            l.js_pushundefined(j);
                            let rc2 = l.js_pcall(j, 0);
                            prep.push_str(&format!("call={rc2} "));
                            // the lightweight function object is at index 1
                        }
                        _ => {}
                    }
                    let have = l.js_gettop(j);
                    let mut i: c_int = have;
                    while i < fill {
                        pushnum(j, i as f64);
                        i += 1;
                    }
                    let before = l.js_gettop(j);
                    match op {
                        Push::Value => {
                            let pv: unsafe extern "C" fn(JS, u64, u64) = l.raw2("js_pushvalue");
                            let tov: unsafe extern "C" fn(JS, c_int) -> *const u64 =
                                l.raw2("js_tovalue");
                            let v = tov(j, -1);
                            pv(j, *v, *v.add(1));
                        }
                        Push::Undefined => l.js_pushundefined(j),
                        Push::Null => l.js_pushnull(j),
                        Push::Boolean => l.js_pushboolean(j, 1),
                        Push::Number => l.js_pushnumber(j, 1.5),
                        Push::String => l.js_pushstring(j, cn!("a string longer than 15 bytes")),
                        Push::LString => l.js_pushlstring(j, cn!("abc"), 3),
                        Push::Literal => l.js_pushliteral(j, cn!("lit")),
                        Push::Object => l.js_pushglobal(j),
                        Push::Current => l.js_currentfunction(j),
                        Push::Copy => l.js_copy(j, 0),
                        Push::Dup => l.nullary("js_dup", j),
                        Push::Dup2 => l.nullary("js_dup2", j),
                        Push::GetIndexFlat => l.js_getindex(j, 1, 0),
                        Push::GetLocalLw => {
                            // callee, this, one argument -> then OP_GETLOCAL 0
                            // inside the lightweight body needs one more slot.
                            // The breadcrumbs say WHICH of the four pushes threw.
                            mark("copy;");
                            l.js_copy(j, 1);
                            mark("this;");
                            l.js_pushundefined(j);
                            mark("arg;");
                            l.js_pushnumber(j, 3.0);
                            mark("call;");
                            l.js_call(j, 1);
                            mark("done;");
                        }
                    }
                    format!("{prep}{op:?} ok before={before} after={}", l.js_gettop(j))
                })
            });
        }
    }
}

/* =========================================================================
 *  Rows 15, 17: JS_STRLIMIT == 1<<28.
 * ========================================================================= */

const JS_STRLIMIT: i64 = 1 << 28;

#[test]
fn t_string_length_limit() {
    // row 17: js_pushlstring rejects n > JS_STRLIMIT *before* touching the
    // buffer (jsrun.c:165), so a 3-byte buffer with a giant `n` is well
    // defined.  n == JS_STRLIMIT is accepted and then fails in js_malloc, so
    // close the memlimit and observe "out of memory" instead of a 256 MiB copy.
    let mut ns: Vec<i64> = vec![
        JS_STRLIMIT - 1,
        JS_STRLIMIT,
        JS_STRLIMIT + 1,
        JS_STRLIMIT + 2,
        i32::MAX as i64,
        16,
        15,
        0,
    ];
    let mut rng = Rng::new(0x5751_1234);
    for _ in 0..12 {
        ns.push(JS_STRLIMIT + rng.range(-4, 5));
    }
    for n in ns {
        probe_state(&format!("js_pushlstring n={n}"), 0, move || {
            job!(|l, j| {
                l.js_setlimit(j, 0, 4096);
                l.js_pushlstring(j, cn!("abc"), n as c_int);
                format!("pushed top={} ty={}", l.js_gettop(j), from_c(l.js_typeof(j, -1)))
            })
        });
    }

    // row 15: js_pushstring uses strlen(), so this one needs a real >256 MiB
    // NUL-terminated buffer.  Build it once and use it for both libraries.
    let big: Vec<u8> = {
        let mut v = vec![b'a'; (JS_STRLIMIT + 1) as usize];
        v.push(0);
        v
    };
    let bp = big.as_ptr() as usize;
    probe_state("js_pushstring strlen>JS_STRLIMIT", 0, move || {
        job!(|l, j| {
            l.js_pushstring(j, bp as *const c_char);
            format!("pushed top={}", l.js_gettop(j))
        })
    });
    // exactly JS_STRLIMIT is allowed past the range check and then hits the
    // memlimit inside jsV_newmemstring.
    let exact = bp + 1; // one byte shorter => strlen == JS_STRLIMIT
    probe_state("js_pushstring strlen==JS_STRLIMIT", 0, move || {
        job!(|l, j| {
            l.js_setlimit(j, 0, 4096);
            l.js_pushstring(j, exact as *const c_char);
            format!("pushed top={}", l.js_gettop(j))
        })
    });
    drop(big);
}

/// The short-string boundary that makes rows 147/148 (`sizeof(js_Value) == 16`,
/// tag at offset 15) observable: strings of exactly 15 bytes are JS_TSHRSTR,
/// 16 bytes are JS_TMEMSTR.
#[test]
fn t_shrstr_boundary() {
    for n in 0..24usize {
        let s = "x".repeat(n);
        diff2(&format!("shrstr n={n}"), move |l| unsafe {
            let j = new_state(l, 0);
            let cs = cstr(&s);
            l.js_pushstring(j, cs.as_ptr());
            l.js_pushlstring(j, cs.as_ptr(), n as c_int);
            let r = format!(
                "push={} lpush={} eq={} len={}",
                from_c(l.js_tryrepr(j, -2, ERRSTR)),
                from_c(l.js_tryrepr(j, -1, ERRSTR)),
                l.nullary_i("js_strictequal", j),
                from_c(l.js_tostring(j, -1)).len(),
            );
            l.js_pop(j, 2);
            l.js_freestate(j);
            r
        });
    }
}

/* =========================================================================
 *  Rows 22-24: no active call frame, and stackidx out of range.
 * ========================================================================= */

#[test]
fn t_currentfunction_no_frame() {
    // row 22: BOT == 0 -> js_currentfunction pushes undefined
    // row 23: BOT == 0 -> js_currentfunctiondata returns NULL
    diff2("currentfunction at top level", |l| unsafe {
        let j = new_state(l, 0);
        let d0 = l.js_currentfunctiondata(j);
        l.js_currentfunction(j);
        let r = format!(
            "data0={} cf={} top={}",
            d0.is_null(),
            vshape(l, j, -1),
            l.js_gettop(j)
        );
        l.js_pop(j, 1);
        l.js_freestate(j);
        r
    });
    // and inside a real frame BOT > 0
    probe_state("currentfunction in a frame", 0, || {
        job!(|l, j| {
            let d = l.js_currentfunctiondata(j);
            l.js_currentfunction(j);
            let r = format!(
                "data_null={} cf_ty={} callable={}",
                d.is_null(),
                from_c(l.js_typeof(j, -1)),
                l.pred("js_iscallable", j, -1)
            );
            l.js_pop(j, 1);
            r
        })
    });
    // js_newcfunctionx data really does come back
    diff2("currentfunctiondata payload", |l| unsafe {
        let j = new_state(l, 0);
        l.js_newcfunctionx(j, Some(cf_data), cn!("df"), 0, PAYLOAD as *mut c_void, None);
        l.js_pushundefined(j);
        let rc = l.js_pcall(j, 0);
        let r = format!("rc={rc} {}", from_c(l.js_tryrepr(j, -1, ERRSTR)));
        l.js_pop(j, 1);
        l.js_freestate(j);
        r
    });
}

unsafe extern "C" fn cf_data(j: JS) {
    let l = cur();
    let d = l.js_currentfunctiondata(j);
    let s = if d.is_null() {
        "NULL".to_string()
    } else {
        from_c(d as *const c_char)
    };
    let cs = cstr(&format!("data={s}"));
    l.js_pushstring(j, cs.as_ptr());
}

#[test]
fn t_stackidx_out_of_range() {
    // row 24: any normalised idx outside [0, TOP) reads a static undefined
    let idxs: Vec<c_int> = vec![
        0, 1, 2, 3, 4, 5, 10, 100, 4095, 4096, 100000, i32::MAX, -1, -2, -3, -4, -5, -10,
        -100, -4095, -4096, -100000, i32::MIN, i32::MIN + 1,
    ];
    for idx in idxs {
        probe_state(&format!("stackidx {idx}"), 0, move || {
            job!(|l, j| {
                l.js_pushnumber(j, 1.0);
                l.js_pushstring(j, cn!("two"));
                format!(
                    "top={} {}",
                    l.js_gettop(j),
                    vshape(l, j, idx)
                )
            })
        });
    }
}

/* =========================================================================
 *  Rows 25-28: js_toregexp / js_touserdata / jsR_tofunction.
 * ========================================================================= */

/// Every value shape used as the receiver of a throwing conversion.
const SHAPES: &[&str] = &[
    "undefined",
    "null",
    "true",
    "false",
    "0",
    "-0",
    "NaN",
    "1.5",
    "''",
    "'x'",
    "({})",
    "[]",
    "[1,2]",
    "(function(){})",
    "/re/g",
    "new String('s')",
    "new Number(2)",
    "new Boolean(true)",
    "new Date(0)",
    "new Error('e')",
    "Math",
    "JSON",
    "this",
];

#[test]
fn t_toregexp_touserdata_tofunction() {
    for (si, s) in SHAPES.iter().enumerate() {
        // row 25: js_toregexp "not a regexp"
        probe_state(&format!("toregexp {s}"), 0, move || {
            let s = SHAPES[si];
            job!(|l, j| {
                let rc = push_expr(l, j, s);
                let tr: unsafe extern "C" fn(JS, c_int) -> *mut c_void = l.raw2("js_toregexp");
                let p = tr(j, -1);
                format!("push={rc} regexp_null={}", p.is_null())
            })
        });
        // row 26: js_touserdata "not a %s" -- exercises the varargs trampoline
        for tag in [N_TAG, N_TAG2] {
            probe_state(
                &format!("touserdata {s} tag={}", unsafe { from_c(tag) }),
                0,
                move || {
                    let s = SHAPES[si];
                    job!(|l, j| {
                        let rc = push_expr(l, j, s);
                        let d = l.js_touserdata(j, -1, tag);
                        format!("push={rc} data_null={}", d.is_null())
                    })
                },
            );
        }
    }
    // a real userdata: matching and mismatching tags
    probe_state("touserdata real match", 0, || {
        job!(|l, j| {
            l.js_newobject(j);
            l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, None);
            let a = l.js_touserdata(j, -1, N_TAG);
            format!("match={} isud={}", from_c(a as *const c_char), l.js_isuserdata(j, -1, N_TAG))
        })
    });
    probe_state("touserdata real mismatch", 0, || {
        job!(|l, j| {
            l.js_newobject(j);
            l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, None);
            let a = l.js_touserdata(j, -1, N_TAG2);
            format!("unreached {}", a.is_null())
        })
    });
    // js_touserdata with a NULL tag: the strcmp is skipped for a non-userdata
    // value, and glibc's vsnprintf renders a NULL "%s" as "(null)".  Well
    // defined for both libraries because both use the platform vsnprintf.
    probe_state("touserdata NULL tag on non-userdata", 0, || {
        job!(|l, j| {
            l.js_pushnumber(j, 1.0);
            let d = l.js_touserdata(j, -1, std::ptr::null());
            format!("unreached {}", d.is_null())
        })
    });

    // rows 27 / 28: jsR_tofunction via js_defaccessor
    for (si, _s) in SHAPES.iter().enumerate() {
        for (gi, _g) in SHAPES.iter().enumerate() {
            if (si * 31 + gi) % 7 != 0 {
                continue; // keep the cross product bounded
            }
            probe_state(
                &format!("defaccessor get={} set={}", SHAPES[si], SHAPES[gi]),
                0,
                move || {
                    job!(|l, j| {
                        l.js_newobject(j);
                        let a = push_expr(l, j, SHAPES[si]);
                        let b = push_expr(l, j, SHAPES[gi]);
                        l.js_defaccessor(j, -3, cn!("acc"), 0);
                        let has = l.js_hasproperty(j, -1, cn!("acc"));
                        let mut r = format!("a={a} b={b} has={has}");
                        if has != 0 {
                            r.push_str(&format!(" v={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                            l.js_pop(j, 1);
                        }
                        r
                    })
                },
            );
        }
    }
    // through JS, both modes
    for g in ["undefined", "null", "0", "'x'", "({})", "(function(){return 1})"] {
        for s in ["undefined", "null", "1", "(function(v){})"] {
            let src = format!(
                "var o={{}}; try {{ Object.defineProperty(o,'p',{{get:{g},set:{s}}}); \
                 print('ok', o.p) }} catch (e) {{ print('E', e) }}"
            );
            diff_dostring(0, &src);
            diff_dostring(JS_STRICT, &src);
        }
    }
}

/// `js_ploadstring` + `js_pcall` of an expression, leaving its value (or the
/// error) on the stack.  Returns the composite return code.
unsafe fn push_expr(l: &Lib, j: JS, src: &str) -> c_int {
    let cs = cstr(src);
    let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
    if rc != 0 {
        return 100 + rc;
    }
    l.js_pushundefined(j);
    l.js_pcall(j, 0)
}

/* =========================================================================
 *  Rows 29-32: js_pop / js_remove / js_insert / js_replace.
 * ========================================================================= */

#[test]
fn t_stack_manip_errors() {
    let mut rng = Rng::new(0x2900_3200);
    let mut ns: Vec<c_int> = vec![-3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 100, i32::MAX, i32::MIN];
    for _ in 0..20 {
        ns.push(rng.range(-8, 12) as c_int);
    }
    for have in 0..4i32 {
        for n in ns.clone() {
            // row 29: js_pop below BOT -> clamps then Error "stack underflow!"
            probe_state(&format!("js_pop have={have} n={n}"), 0, move || {
                job!(|l, j| {
                    for i in 0..have {
                        l.js_pushnumber(j, i as f64);
                    }
                    let before = l.js_gettop(j);
                    l.js_pop(j, n);
                    format!("pop before={before} after={}", l.js_gettop(j))
                })
            });
            // rows 30 / 32: js_remove / js_replace outside [BOT, TOP)
            probe_state(&format!("js_remove have={have} idx={n}"), 0, move || {
                job!(|l, j| {
                    for i in 0..have {
                        l.js_pushnumber(j, i as f64);
                    }
                    let before = l.js_gettop(j);
                    l.js_remove(j, n);
                    format!("remove before={before} after={}", l.js_gettop(j))
                })
            });
            probe_state(&format!("js_replace have={have} idx={n}"), 0, move || {
                job!(|l, j| {
                    for i in 0..have {
                        l.js_pushnumber(j, i as f64);
                    }
                    l.js_pushstring(j, cn!("REPL"));
                    let before = l.js_gettop(j);
                    l.js_replace(j, n);
                    let mut r = format!("replace before={before} after={}", l.js_gettop(j));
                    for k in 0..l.js_gettop(j) {
                        r.push_str(&format!(" {}={}", k, from_c(l.js_trystring(j, k, ERRSTR))));
                    }
                    r
                })
            });
            // row 31: js_insert is unconditionally js_error("not implemented yet")
            probe_state(&format!("js_insert have={have} idx={n}"), 0, move || {
                job!(|l, j| {
                    for i in 0..have {
                        l.js_pushnumber(j, i as f64);
                    }
                    l.js_insert(j, n);
                    "insert returned!".to_string()
                })
            });
        }
    }
}

/* =========================================================================
 *  Rows 36-39: js_isarrayindex.
 * ========================================================================= */

#[test]
fn t_isarrayindex() {
    let mut names: Vec<String> = vec![
        String::new(),      // row 36
        "0".into(),
        "00".into(),        // row 37
        "01".into(),        // row 37
        "0x1".into(),       // row 37
        "0.5".into(),
        "1".into(),
        "9".into(),
        "10".into(),
        "999999999".into(), // 9 digits, ok
        "1000000000".into(),
        "2147483647".into(), // row 38
        "2147483648".into(), // row 38
        "12345678901".into(), // row 38
        "99999999999999999999".into(),
        "-1".into(),        // row 39
        "+1".into(),        // row 39
        " 1".into(),        // row 39
        "1 ".into(),        // row 39
        "1e3".into(),       // row 39
        "a".into(),
        "1a".into(),
        "a1".into(),
        "length".into(),
        "\u{7f}".into(),
        "1\u{80}".into(),
    ];
    let mut rng = Rng::new(0x3600_3900);
    for _ in 0..600 {
        names.push(rng.ascii_string(12));
    }
    for _ in 0..300 {
        // digit-heavy strings around the INT_MAX/10 threshold
        let n = 1 + rng.below(12) as usize;
        let s: String = (0..n)
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        names.push(s);
    }
    diff2("js_isarrayindex", |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for n in &names {
            let cs = cstr(n);
            let mut idx: c_int = -12345;
            let rc = l.js_isarrayindex(j, cs.as_ptr(), &mut idx);
            r.push_str(&format!("{n:?}=>{rc}/{idx}\n"));
        }
        l.js_freestate(j);
        r
    });
    // and the same names as real property keys, in both modes
    for n in ["", "0", "00", "01", "1", "2147483648", "-1", "1e3", "length"] {
        let src = format!(
            "var a=[10,20,30]; a[{n:?}]=99; print(a.length, JSON.stringify(a), a[{n:?}], \
             Object.getOwnPropertyNames(a).join('|'))"
        );
        diff_dostring(0, &src);
        diff_dostring(JS_STRICT, &src);
    }
}

/* =========================================================================
 *  Rows 42-48: property / index reads that miss.
 * ========================================================================= */

#[test]
fn t_property_miss() {
    let keys = [
        "length", "0", "1", "2", "3", "-1", "00", "01", "1.5", "nope", "source", "global",
        "ignoreCase", "multiline", "lastIndex", "toString", "constructor", "",
    ];
    for s in SHAPES {
        for k in keys {
            probe_state(&format!("hasproperty {s}[{k}]"), 0, move || {
                job!(|l, j| {
                    let rc = push_expr(l, j, s);
                    let cs = cstr(k);
                    let before = l.js_gettop(j);
                    let h = l.js_hasproperty(j, -1, cs.as_ptr());
                    let mut r = format!("push={rc} has={h} d={}", l.js_gettop(j) - before);
                    if l.js_gettop(j) > before {
                        r.push_str(&format!(" v={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                        l.js_pop(j, 1);
                    }
                    // row 46: a miss through js_getproperty pushes undefined
                    l.js_getproperty(j, -1, cs.as_ptr());
                    r.push_str(&format!(" get={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                    l.js_pop(j, 1);
                    r
                })
            });
        }
        // rows 47 / 48: js_hasindex / js_getindex
        for i in [-2i32, -1, 0, 1, 2, 3, 4, 100, i32::MAX, i32::MIN] {
            probe_state(&format!("hasindex {s}[{i}]"), 0, move || {
                job!(|l, j| {
                    let rc = push_expr(l, j, s);
                    let before = l.js_gettop(j);
                    let h = l.js_hasindex(j, -1, i);
                    let mut r = format!("push={rc} has={h} d={}", l.js_gettop(j) - before);
                    if l.js_gettop(j) > before {
                        r.push_str(&format!(" v={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                        l.js_pop(j, 1);
                    }
                    l.js_getindex(j, -1, i);
                    r.push_str(&format!(" get={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                    l.js_pop(j, 1);
                    r
                })
            });
        }
    }
    // row 42: a flat array's out-of-range index does NOT consult the prototype
    for src in [
        "Array.prototype[5]='proto'; var a=[1,2]; print(a[5], 5 in a, a.length)",
        "Array.prototype[1]='proto'; var a=[1,2]; print(a[1], 1 in a)",
        "Array.prototype[5]='proto'; var a=[1,2]; a[9]=9; print(a[5], 5 in a, a.length)",
        "Object.prototype.x='proto'; var a=[1,2]; print(a.x)",
        // row 43: a string wrapper's out-of-range index DOES fall through
        "String.prototype[9]='proto'; var s=new String('ab'); print(s[9], s[0], s.length)",
        "String.prototype[1]='proto'; var s=new String('ab'); print(s[1])",
        // rows 44 / 45
        "var o={}; print(o.nope, 'nope' in o, Object.getPrototypeOf(o)===Object.prototype)",
        "var o=Object.create(null); print(o.nope, 'nope' in o)",
    ] {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
    }
}

/// The CSTRING wrapper index boundary in `jsR_hasproperty` (jsrun.c:594-599),
/// including astral runes where `js_utflen` counts 2 -- this is the guard that
/// makes row 40 unreachable.
#[test]
fn t_string_wrapper_indices() {
    let strs = [
        "", "a", "ab", "\u{7f}", "\u{80}", "\u{7ff}", "\u{800}", "\u{ffff}", "\u{10000}",
        "\u{10ffff}", "a\u{10000}b", "\u{10000}\u{10000}", "ab\u{10ffff}",
    ];
    for s in strs {
        let esc: String = s
            .chars()
            .map(|c| format!("\\u{{{:x}}}", c as u32))
            .collect();
        let src = format!(
            "var s=new String('{esc}'); var r=[s.length]; \
             for (var i=-2;i<s.length+3;++i) r.push(i+':'+(i in s)+':'+s[i]); \
             print(r.join(' '))"
        );
        diff_dostring(0, &src);
        // and through the C API, so js_pushrune's argument is exercised directly
        probe_state(&format!("strwrap {esc}"), 0, move || {
            let owned = s.to_string();
            job!(|l, j| {
                let cs = cstr(&owned);
                l.js_newstring(j, cs.as_ptr());
                let n = l.js_getlength(j, -1);
                let mut r = format!("len={n}");
                for i in -2..(n + 3) {
                    let before = l.js_gettop(j);
                    let h = l.js_hasindex(j, -1, i);
                    if l.js_gettop(j) > before {
                        r.push_str(&format!(
                            " {i}={h}:{}",
                            from_c(l.js_tryrepr(j, -1, ERRSTR))
                        ));
                        l.js_pop(j, 1);
                    } else {
                        r.push_str(&format!(" {i}={h}:_"));
                    }
                }
                r
            })
        });
    }
}

/* =========================================================================
 *  Rows 53-54: array `length` assignment.
 * ========================================================================= */

const JS_ARRAYLIMIT: i64 = 1 << 26;

#[test]
fn t_array_length_errors() {
    let mut vals: Vec<String> = vec![
        "0".into(),
        "1".into(),
        "3".into(),
        "1.5".into(),
        "-1".into(),
        "-0".into(),
        "NaN".into(),
        "Infinity".into(),
        "-Infinity".into(),
        "'foo'".into(),
        "'3'".into(),
        "'3.5'".into(),
        "''".into(),
        "null".into(),
        "undefined".into(),
        "true".into(),
        "false".into(),
        "{}".into(),
        "[]".into(),
        "[7]".into(),
        "2147483647".into(),
        "2147483648".into(),
        "-2147483648".into(),
        "-2147483649".into(),
        "4294967296".into(),
        format!("{}", JS_ARRAYLIMIT - 1),
        format!("{}", JS_ARRAYLIMIT),
        format!("{}", JS_ARRAYLIMIT + 1),
        format!("{}", JS_ARRAYLIMIT + 2),
        format!("{}", JS_ARRAYLIMIT * 4),
    ];
    let mut rng = Rng::new(0x5300_5400);
    for _ in 0..24 {
        vals.push(format!("{}", JS_ARRAYLIMIT + rng.range(-3, 4)));
    }
    for _ in 0..24 {
        vals.push(format!("{}", rng.range(-40, 40)));
    }
    for v in &vals {
        // flat and unflattened arrays take different branches (jsrun.c:710/714)
        for pre in ["", "a[10]=1;", "Object.defineProperty(a,'0',{value:1});"] {
            let src = format!(
                "var a=[1,2,3]; {pre} try {{ a.length = {v}; \
                 print('ok', a.length, a[0], a[2]) }} catch (e) {{ print('E', e) }}"
            );
            diff_dostring(0, &src);
            diff_dostring(JS_STRICT, &src);
        }
    }
    // and through js_setlength, which bypasses ToNumber
    let mut lens: Vec<c_int> = vec![
        0,
        1,
        3,
        -1,
        -2,
        i32::MAX,
        i32::MIN,
        JS_ARRAYLIMIT as c_int,
        (JS_ARRAYLIMIT + 1) as c_int,
        (JS_ARRAYLIMIT - 1) as c_int,
    ];
    for _ in 0..16 {
        lens.push(rng.range(-100, 100) as c_int);
    }
    for n in lens {
        probe_state(&format!("js_setlength {n}"), 0, move || {
            job!(|l, j| {
                l.js_newarray(j);
                for i in 0..3 {
                    l.js_pushnumber(j, i as f64);
                    l.js_setindex(j, -2, i);
                }
                l.js_setlength(j, -1, n);
                format!("len={}", l.js_getlength(j, -1))
            })
        });
    }
}

/* =========================================================================
 *  Rows 55-67: jsR_setproperty, every strict/sloppy fork.
 * ========================================================================= */

#[test]
fn t_setproperty_readonly_forks() {
    // rows 55-60 + 67: CSTRING length / in-range index, CREGEXP source /
    // global / ignoreCase / multiline; row 749 (lastIndex) is NOT readonly.
    let targets = [
        ("new String('abcd')", &["length", "0", "1", "3", "4", "5", "-1", "x"][..]),
        (
            "/pat/gim",
            &["source", "global", "ignoreCase", "multiline", "lastIndex", "x"][..],
        ),
        ("/pat/", &["source", "global", "ignoreCase", "multiline", "lastIndex"][..]),
        ("[1,2,3]", &["length", "0", "3", "9", "x"][..]),
        ("({})", &["x"][..]),
    ];
    for (t, keys) in targets {
        for k in keys {
            for flags in [0, JS_STRICT] {
                let src = format!(
                    "var t = {t}; try {{ t[{k:?}] = 'W'; \
                     print('ok', t[{k:?}], String(t), t.lastIndex) }} \
                     catch (e) {{ print('E', e) }}"
                );
                diff_dostring(flags, &src);
            }
            // through js_setproperty, where the receiver is a real object
            for flags in [0, JS_STRICT] {
                probe_state(
                    &format!("js_setproperty {t}[{k}] flags={flags}"),
                    flags,
                    move || {
                        job!(|l, j| {
                            let rc = push_expr(l, j, t);
                            let cs = cstr(k);
                            l.js_pushstring(j, cn!("W"));
                            l.js_setproperty(j, -2, cs.as_ptr());
                            let h = l.js_hasproperty(j, -1, cs.as_ptr());
                            let mut r = format!("push={rc} has={h}");
                            if h != 0 {
                                r.push_str(&format!(" v={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                                l.js_pop(j, 1);
                            }
                            r
                        })
                    },
                );
            }
        }
    }

    // row 62: strict + getter without setter
    // row 63: JS_READONLY own property
    // row 67: the readonly label
    for flags in [0, JS_STRICT] {
        for src in [
            "var o={get p(){return 1}}; o.p=2; print(o.p)",
            "var o={get p(){return 1}, set p(v){print('set',v)}}; o.p=2; print(o.p)",
            "var o={}; Object.defineProperty(o,'p',{value:1,writable:false}); o.p=2; print(o.p)",
            "var o={}; Object.defineProperty(o,'p',{value:1,writable:true}); o.p=2; print(o.p)",
            "var p={}; Object.defineProperty(p,'q',{value:1,writable:false}); \
             var o=Object.create(p); o.q=2; print(o.q, Object.getOwnPropertyNames(o).length)",
            "var p={get q(){return 7}}; var o=Object.create(p); o.q=2; print(o.q)",
            "var o={}; Object.defineProperty(o,'p',{get:function(){return 1}}); o.p=2; print(o.p)",
            // row 64 / 65: transient receiver (assigning on a primitive)
            "var s='abc'; s.x=1; print(s.x)",
            "var n=1; n.x=1; print(n.x)",
            "var b=true; b.x=1; print(b.x)",
            "'abc'.x=1; print('done')",
            "(1).x=1; print('done')",
        ] {
            let wrapped = format!("try {{ {src} }} catch (e) {{ print('E', e) }}");
            diff_dostring(flags, &wrapped);
            diff_eval(flags, &wrapped);
        }
        // rows 64/65 through the C API: js_setproperty on a primitive slot
        for s in ["'abc'", "1", "true", "undefined", "null", "({})"] {
            probe_state(
                &format!("transient js_setproperty {s} flags={flags}"),
                flags,
                move || {
                    job!(|l, j| {
                        let rc = push_expr(l, j, s);
                        l.js_pushstring(j, cn!("V"));
                        l.js_setproperty(j, -2, cn!("tp"));
                        let ty = from_c(l.js_typeof(j, -1));
                        let h = l.js_hasproperty(j, -1, cn!("tp"));
                        let mut r = format!("push={rc} ty={ty} has={h}");
                        if h != 0 {
                            r.push_str(&format!(" v={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                            l.js_pop(j, 1);
                        }
                        r
                    })
                },
            );
            probe_state(
                &format!("transient js_setindex {s} flags={flags}"),
                flags,
                move || {
                    job!(|l, j| {
                        let rc = push_expr(l, j, s);
                        l.js_pushstring(j, cn!("V"));
                        l.js_setindex(j, -2, 0);
                        format!(
                            "push={rc} ty={} has0={}",
                            from_c(l.js_typeof(j, -1)),
                            l.js_hasindex(j, -1, 0)
                        )
                    })
                },
            );
        }
    }
}

/* ------------------------------------------------------- userdata hooks */

thread_local! {
    static HOOKLOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn hook(s: String) {
    HOOKLOG.with(|h| h.borrow_mut().push(s));
}

fn hooks_take() -> String {
    HOOKLOG.with(|h| std::mem::take(&mut *h.borrow_mut()).join(","))
}

unsafe extern "C" fn ud_has(j: JS, _d: *mut c_void, name: *const c_char) -> c_int {
    let n = from_c(name);
    if n.starts_with('h') {
        cur().js_pushnumber(j, 42.0);
        hook(format!("has({n})->1"));
        return 1;
    }
    hook(format!("has({n})->0"));
    0
}

unsafe extern "C" fn ud_put(_j: JS, _d: *mut c_void, name: *const c_char) -> c_int {
    let n = from_c(name);
    let r = n.starts_with('p') as c_int;
    hook(format!("put({n})->{r}"));
    r
}

unsafe extern "C" fn ud_del(_j: JS, _d: *mut c_void, name: *const c_char) -> c_int {
    let n = from_c(name);
    let r = n.starts_with('d') as c_int;
    hook(format!("del({n})->{r}"));
    r
}

unsafe extern "C" fn ud_fin(_j: JS, d: *mut c_void) {
    hook(format!("fin({})", from_c(d as *const c_char)));
}

/// Rows 61 / 76 / 91: the userdata `put` / `delete` callbacks short-circuit
/// jsR_setproperty / jsR_defproperty / jsR_delproperty.
#[test]
fn t_userdata_hooks_shortcircuit() {
    let keys = ["put_me", "px", "hx", "dx", "plain", "length", "0"];
    for k in keys {
        for flags in [0, JS_STRICT] {
            diff2(&format!("ud hooks {k} flags={flags}"), move |l| unsafe {
                let _ = hooks_take();
                let j = new_state(l, flags);
                let r = probe(
                    l,
                    j,
                    job!(|l, j| {
                        l.js_newobject(j);
                        l.js_newuserdatax(
                            j,
                            N_TAG,
                            PAYLOAD as *mut c_void,
                            Some(ud_has),
                            Some(ud_put),
                            Some(ud_del),
                            Some(ud_fin),
                        );
                        let cs = cstr(k);
                        let mut r = String::new();
                        // set
                        l.js_pushstring(j, cn!("SV"));
                        l.js_setproperty(j, -2, cs.as_ptr());
                        r.push_str(&format!("set|{}|", hooks_take()));
                        // def
                        l.js_pushstring(j, cn!("DV"));
                        l.js_defproperty(j, -2, cs.as_ptr(), 0);
                        r.push_str(&format!("def|{}|", hooks_take()));
                        // has
                        let h = l.js_hasproperty(j, -1, cs.as_ptr());
                        if h != 0 {
                            r.push_str(&format!("v={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                            l.js_pop(j, 1);
                        }
                        r.push_str(&format!("has={h}|{}|", hooks_take()));
                        // del
                        l.js_delproperty(j, -1, cs.as_ptr());
                        r.push_str(&format!("del|{}|", hooks_take()));
                        r
                    }),
                );
                l.js_gc(j, 0);
                l.js_freestate(j);
                format!("{r} hooks_after={}", hooks_take())
            });
        }
    }
}

/* =========================================================================
 *  Rows 68-82: jsR_defproperty.
 * ========================================================================= */

#[test]
fn t_defproperty_forks() {
    let targets = [
        "[1,2,3]",
        "new String('abcd')",
        "/pat/gim",
        "({})",
        "(function(){})",
        "new Number(1)",
    ];
    let keys = [
        "length", "0", "1", "3", "4", "source", "global", "ignoreCase", "multiline",
        "lastIndex", "x",
    ];
    // js_defproperty passes throw=1 (jsrun.c:1017), so the readonly label
    // throws even in sloppy mode (row 82); js_defglobal passes throw=0.
    for t in targets {
        for k in keys {
            for flags in [0, JS_STRICT] {
                probe_state(
                    &format!("js_defproperty {t}[{k}] flags={flags}"),
                    flags,
                    move || {
                        job!(|l, j| {
                            let rc = push_expr(l, j, t);
                            let cs = cstr(k);
                            l.js_pushstring(j, cn!("D"));
                            l.js_defproperty(j, -2, cs.as_ptr(), 0);
                            let h = l.js_hasproperty(j, -1, cs.as_ptr());
                            let mut r = format!("push={rc} has={h}");
                            if h != 0 {
                                r.push_str(&format!(" v={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                                l.js_pop(j, 1);
                            }
                            r.push_str(&format!(" names={}", own_names(l, j, -1)));
                            r
                        })
                    },
                );
                // Object.defineProperty goes through js_defproperty + js_defaccessor
                let src = format!(
                    "var t={t}; try {{ Object.defineProperty(t,{k:?},{{value:'D'}}); \
                     print('ok', t[{k:?}]) }} catch (e) {{ print('E', e) }}"
                );
                diff_dostring(flags, &src);
                let src2 = format!(
                    "var t={t}; try {{ Object.defineProperty(t,{k:?},\
                     {{get:function(){{return 'G'}}}}); print('ok', t[{k:?}]) }} \
                     catch (e) {{ print('E', e) }}"
                );
                diff_dostring(flags, &src2);
            }
        }
    }

    // rows 78 / 79 / 80: value / getter / setter supplied for an existing
    // READONLY or DONTCONF property, strict only.
    for flags in [0, JS_STRICT] {
        for src in [
            "var o={}; Object.defineProperty(o,'p',{value:1}); \
             Object.defineProperty(o,'p',{value:2}); print(o.p)",
            "var o={}; Object.defineProperty(o,'p',{value:1,writable:true}); \
             Object.defineProperty(o,'p',{value:2}); print(o.p)",
            "var o={}; Object.defineProperty(o,'p',{value:1,configurable:true}); \
             Object.defineProperty(o,'p',{get:function(){return 9}}); print(o.p)",
            "var o={}; Object.defineProperty(o,'p',{value:1}); \
             Object.defineProperty(o,'p',{get:function(){return 9}}); print(o.p)",
            "var o={}; Object.defineProperty(o,'p',{value:1}); \
             Object.defineProperty(o,'p',{set:function(v){}}); print(o.p)",
            "var o={}; Object.defineProperty(o,'p',{value:1,configurable:true,writable:true}); \
             Object.defineProperty(o,'p',{set:function(v){print('s',v)}}); o.p=3; print(o.p)",
            // row 77: jsV_setproperty returns NULL on a sealed object
            "var o={}; Object.preventExtensions(o); \
             Object.defineProperty(o,'nw',{value:1}); print(o.nw, Object.isExtensible(o))",
            "var o={a:1}; Object.preventExtensions(o); \
             Object.defineProperty(o,'a',{value:2}); print(o.a)",
            "var o={}; Object.seal(o); Object.defineProperty(o,'nw',{value:1}); print(o.nw)",
            "var o={}; Object.freeze(o); Object.defineProperty(o,'nw',{value:1}); print(o.nw)",
        ] {
            let w = format!("try {{ {src} }} catch (e) {{ print('E', e) }}");
            diff_dostring(flags, &w);
        }
        // row 81: the readonly label with NEITHER strict NOR throw.  The only
        // reachable throw=0 site that can hit it is js_initvar (jsrun.c:1087)
        // running against a `with` scope object whose class special-cases the
        // name -- non-strict eval keeps J->E (jsstate.c:124, scope == NULL).
        for src in [
            "with([1,2,3]) { eval('var length'); print('a', length) }",
            "with(new String('abcd')) { eval('var length'); print('b', length) }",
            "with(/pat/g) { eval('var source'); print('c', source) }",
            "with(/pat/g) { eval('var global'); print('d', global) }",
            "with(/pat/g) { eval('var lastIndex'); print('e', lastIndex) }",
            "with(/pat/g) { eval('var ignoreCase, multiline'); print('f', ignoreCase) }",
            "with({}) { eval('var length'); print('g', length) }",
            "with([1,2,3]) { eval('var xx = 5'); print('h', xx, this.xx) }",
        ] {
            if flags == JS_STRICT {
                continue; // `with` is a SyntaxError in strict mode -- covered below
            }
            let w = format!("try {{ {src} }} catch (e) {{ print('E', e) }}");
            diff_dostring(flags, &w);
        }
    }
    // `with` in strict mode is rejected by the compiler
    diff_dostring(JS_STRICT, "with([1]) { print(length) }");
    diff_dostring(0, "'use strict'; with([1]) { print(length) }");

    // js_defglobal / js_initvar with every attribute combination
    for atts in [0, 1, 2, 3, 4, 5, 6, 7] {
        probe_state(&format!("js_defglobal atts={atts}"), 0, move || {
            job!(|l, j| {
                l.js_pushnumber(j, 1.0);
                l.js_defglobal(j, cn!("gv"), atts);
                l.js_pushnumber(j, 2.0);
                l.js_defglobal(j, cn!("gv"), atts);
                l.js_getglobal(j, cn!("gv"));
                let r = format!("gv={}", from_c(l.js_tryrepr(j, -1, ERRSTR)));
                l.js_pop(j, 1);
                r
            })
        });
    }
}

/// The own enumerable+non-enumerable property names of the value at `idx`,
/// rendered stably.  Uses `js_pushiterator(own=1)` so no prototype noise.
unsafe fn own_names(l: &Lib, j: JS, idx: c_int) -> String {
    let mut v: Vec<String> = vec![];
    l.js_copy(j, idx);
    l.js_pushiterator(j, -1, 1);
    loop {
        let p = l.js_nextiterator(j, -1);
        if p.is_null() {
            break;
        }
        v.push(from_c(p));
        if v.len() > 64 {
            v.push("...".into());
            break;
        }
    }
    l.js_pop(j, 2);
    format!("[{}]", v.join(","))
}

/* =========================================================================
 *  Rows 83-95: jsR_delproperty.
 * ========================================================================= */

#[test]
fn t_delproperty_forks() {
    let targets = [
        "[1,2,3]",
        "new String('abcd')",
        "/pat/gim",
        "({a:1})",
        "(function(){})",
        "Math",
    ];
    let keys = [
        "length", "0", "1", "3", "4", "source", "global", "ignoreCase", "multiline",
        "lastIndex", "a", "nope", "abs",
    ];
    for t in targets {
        for k in keys {
            for flags in [0, JS_STRICT] {
                // row 94 vs 95: non-strict returns 0, strict throws
                let src = format!(
                    "var t={t}; try {{ print('r', delete t[{k:?}], t[{k:?}]) }} \
                     catch (e) {{ print('E', e) }}"
                );
                diff_dostring(flags, &src);
                probe_state(
                    &format!("js_delproperty {t}[{k}] flags={flags}"),
                    flags,
                    move || {
                        job!(|l, j| {
                            let rc = push_expr(l, j, t);
                            let cs = cstr(k);
                            l.js_delproperty(j, -1, cs.as_ptr());
                            format!(
                                "push={rc} has={} names={}",
                                l.js_hasproperty(j, -1, cs.as_ptr()),
                                own_names(l, j, -1)
                            )
                        })
                    },
                );
            }
        }
    }
    // rows 92 / 93: missing own property still "succeeds"; DONTCONF does not
    for flags in [0, JS_STRICT] {
        for src in [
            "var o={}; print(delete o.nope)",
            "var o={a:1}; print(delete o.a, 'a' in o)",
            "var o={}; Object.defineProperty(o,'p',{value:1,configurable:true}); \
             print(delete o.p, 'p' in o)",
            "var o={}; Object.defineProperty(o,'p',{value:1}); print(delete o.p, 'p' in o)",
            "var p={a:1}; var o=Object.create(p); print(delete o.a, o.a)",
            "print(delete Math.abs, Math.abs)",
            "print(delete this.Math, typeof Math)",
            "var a=[1,2,3]; print(delete a[2], a.length, JSON.stringify(a))",
            "var a=[1,2,3]; print(delete a[0], a.length, JSON.stringify(a))",
            "var a=[1,2,3]; a[9]=9; print(delete a[2], a.length)",
        ] {
            let w = format!("try {{ {src} }} catch (e) {{ print('E', e) }}");
            diff_dostring(flags, &w);
        }
    }
    // js_delindex / js_delglobal / js_delregistry
    for i in [-1i32, 0, 1, 2, 3, 100, i32::MAX, i32::MIN] {
        for flags in [0, JS_STRICT] {
            probe_state(&format!("js_delindex {i} flags={flags}"), flags, move || {
                job!(|l, j| {
                    l.js_newarray(j);
                    for k in 0..3 {
                        l.js_pushnumber(j, k as f64);
                        l.js_setindex(j, -2, k);
                    }
                    l.js_delindex(j, -1, i);
                    format!(
                        "len={} names={}",
                        l.js_getlength(j, -1),
                        own_names(l, j, -1)
                    )
                })
            });
        }
    }
}

/* =========================================================================
 *  Rows 96-100: js_hasvar / js_setvar / js_delvar.
 * ========================================================================= */

#[test]
fn t_var_ops() {
    for flags in [0, JS_STRICT] {
        for src in [
            // rows 96 / 97 / 126: not found anywhere
            "print(typeof nosuchvar)",
            "print(nosuchvar)",
            "try { print(nosuchvar) } catch (e) { print('E', e) }",
            // row 99: strict assignment to an undeclared variable
            "try { undeclared = 1; print('ok', undeclared) } catch (e) { print('E', e) }",
            "function f(){ inner = 2 } try { f(); print('ok', inner) } catch (e) { print('E', e) }",
            // row 100: delete of a DONTCONF var
            "var a = 1; try { print('d', delete a, typeof a) } catch (e) { print('E', e) }",
            "function g(){}; try { print('d', delete g, typeof g) } catch (e) { print('E', e) }",
            "try { print('d', delete nosuchvar) } catch (e) { print('E', e) }",
            "this.q = 1; try { print('d', delete q, typeof q) } catch (e) { print('E', e) }",
            // getter/setter vars via the global object
            "Object.defineProperty(this,'gv',{get:function(){return 5}}); print(gv)",
            "Object.defineProperty(this,'sv',{set:function(v){print('set',v)}, \
             get:function(){return 1}}); sv = 3; print(sv)",
            "Object.defineProperty(this,'ro',{value:1}); \
             try { ro = 2; print('ok', ro) } catch (e) { print('E', e) }",
            // row 125: OP_GETLOCAL's NON-lightweight branch raising
            // ReferenceError.  A local can only vanish from the scope chain if
            // `jsR_callscript` skipped `js_initvar` for it (jsrun.c:1250, the
            // Bug 701886 guard), which needs the name to already exist WITHOUT
            // JS_DONTCONF -- i.e. created by a plain assignment first -- and then
            // to be deleted.  A non-strict `eval` keeps J->E (jsstate.c:124), so
            // its script-level `var` is a local of a non-lightweight function.
            "this.b = 1; try { print(eval('var b; delete b; b')) } \
             catch (e) { print('E', e) }",
            "this.c = 1; try { print(eval('var c; delete c; typeof c')) } \
             catch (e) { print('E', e) }",
            "this.d = 1; try { print(eval('var d; d')) } catch (e) { print('E', e) }",
            "var e2 = 1; try { print(eval('var e2; delete e2; e2')) } \
             catch (e) { print('E', e) }",
            "this.f2 = 1; try { print(eval('var f2; delete f2; delete f2; f2')) } \
             catch (e) { print('E', e) }",
            "this.g2 = 1; try { print(eval('var g2; delete g2; g2 = 7; g2')) } \
             catch (e) { print('E', e) }",
        ] {
            diff_dostring(flags, src);
            diff_eval(flags, src);
        }
    }
    // row 98 through the C API: a READONLY global assigned in strict mode
    for atts in [0, 1, 2, 3, 4, 5, 6, 7] {
        for flags in [0, JS_STRICT] {
            probe_state(
                &format!("setvar readonly atts={atts} flags={flags}"),
                flags,
                move || {
                    job!(|l, j| {
                        l.js_pushnumber(j, 1.0);
                        l.js_defglobal(j, cn!("rv"), atts);
                        let cs = cstr("rv = 99; 'assigned:' + rv");
                        let rc = push_expr(l, j, "0");
                        l.js_pop(j, 1);
                        let lrc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
                        let mut r = format!("pre={rc} load={lrc}");
                        if lrc == 0 {
                            l.js_pushundefined(j);
                            let crc = l.js_pcall(j, 0);
                            r.push_str(&format!(
                                " call={crc} v={}",
                                from_c(l.js_tryrepr(j, -1, ERRSTR))
                            ));
                            l.js_pop(j, 1);
                        }
                        l.js_getglobal(j, cn!("rv"));
                        r.push_str(&format!(" rv={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                        l.js_pop(j, 1);
                        r
                    })
                },
            );
        }
    }
}

/* =========================================================================
 *  Rows 102-114: calls.
 * ========================================================================= */

unsafe extern "C" fn cf_nopush(_j: JS) {
    /* row 104: return without pushing anything */
}

/// A getter/plain cfunction that returns a fixed string.  Deliberately does NOT
/// stringify its receiver: used as an accessor, `js_torepr` of the receiver would
/// re-enter the getter (jsrepr.c walks accessors) and the recursion only stops
/// when `js_tryrepr` runs the try stack out at JS_TRYLIMIT -- correct in both
/// libraries, but exponentially slow.
unsafe extern "C" fn cf_ret(j: JS) {
    let l = cur();
    l.js_pushstring(j, cn!("GOT"));
}

unsafe extern "C" fn cf_argdump(j: JS) {
    let l = cur();
    let top = l.js_gettop(j);
    let mut s = format!("top={top}");
    for i in 0..top {
        s.push_str(&format!(" {}={}", i, from_c(l.js_tryrepr(j, i, ERRSTR))));
    }
    let cs = cstr(&s);
    l.js_pushstring(j, cs.as_ptr());
}

#[test]
fn t_call_paths() {
    // row 103: fewer arguments than the declared length -> padded with undefined
    // row 104: a cfunction that pushes nothing -> undefined result
    for len in [0, 1, 2, 3, 5] {
        for nargs in [0, 1, 2, 3, 5] {
            probe_state(&format!("cfunc len={len} nargs={nargs}"), 0, move || {
                job!(|l, j| {
                    l.js_newcfunction(j, Some(cf_argdump), cn!("ad"), len);
                    l.js_pushundefined(j);
                    for i in 0..nargs {
                        l.js_pushnumber(j, (i + 1) as f64);
                    }
                    l.js_call(j, nargs);
                    let r = format!("res={}", from_c(l.js_tryrepr(j, -1, ERRSTR)));
                    l.js_pop(j, 1);
                    r
                })
            });
            probe_state(&format!("cfunc nopush len={len} nargs={nargs}"), 0, move || {
                job!(|l, j| {
                    l.js_newcfunction(j, Some(cf_nopush), cn!("np"), len);
                    l.js_pushundefined(j);
                    for i in 0..nargs {
                        l.js_pushnumber(j, (i + 1) as f64);
                    }
                    l.js_call(j, nargs);
                    format!(
                        "res={} ty={} top={}",
                        from_c(l.js_tryrepr(j, -1, ERRSTR)),
                        from_c(l.js_typeof(j, -1)),
                        l.js_gettop(j)
                    )
                })
            });
        }
    }
    // row 102: lightweight call with more arguments than declared params
    for src in [
        "print((function(){return arguments ? 1 : 0})(1,2,3))",
        "print((function(a){return a})(1,2,3))",
        "print((function(){return 'lw'})(1,2,3,4,5))",
        "print((function(a,b){return a+b})(1))",
        "print((function(a,b){return [a,b].join(',')})())",
        "function f(a){ return a } print(f(1,2,3), f(), f(1))",
        "function g(){ var x=1; return x } print(g(1,2,3,4,5,6,7,8,9,10))",
    ] {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
    }

    // row 106: js_call with a negative argument count
    let mut ns: Vec<c_int> = vec![-1, -2, -3, -100, i32::MIN, 0, 1];
    let mut rng = Rng::new(0x1060_0000);
    for _ in 0..16 {
        ns.push(rng.range(-64, 3) as c_int);
    }
    for n in ns {
        probe_state(&format!("js_call n={n}"), 0, move || {
            job!(|l, j| {
                l.js_newcfunction(j, Some(cf_argdump), cn!("ad"), 0);
                l.js_pushundefined(j);
                l.js_call(j, n);
                format!("returned top={}", l.js_gettop(j))
            })
        });
        probe_state(&format!("js_construct n={n}"), 0, move || {
            job!(|l, j| {
                l.js_newcfunction(j, Some(cf_argdump), cn!("ad"), 0);
                l.js_construct(j, n);
                format!("returned top={}", l.js_gettop(j))
            })
        });
    }

    // rows 107 / 109: "%s is not callable" for every value shape
    for (si, _) in SHAPES.iter().enumerate() {
        probe_state(&format!("js_call noncallable {}", SHAPES[si]), 0, move || {
            job!(|l, j| {
                let rc = push_expr(l, j, SHAPES[si]);
                l.js_pushundefined(j);
                l.js_call(j, 0);
                format!("push={rc} returned")
            })
        });
        probe_state(
            &format!("js_construct noncallable {}", SHAPES[si]),
            0,
            move || {
                job!(|l, j| {
                    let rc = push_expr(l, j, SHAPES[si]);
                    l.js_construct(j, 0);
                    format!("push={rc} returned")
                })
            },
        );
        let src = format!(
            "try {{ var f = {}; f() }} catch (e) {{ print('call', e) }} \
             try {{ var g = {}; new g() }} catch (e) {{ print('new', e) }}",
            SHAPES[si], SHAPES[si]
        );
        diff_dostring(0, &src);
        diff_dostring(JS_STRICT, &src);
    }

    // rows 110 / 111: prototype not an object, and a non-object return value
    for src in [
        "function F(){}; F.prototype = 1; var o = new F(); \
         print(Object.getPrototypeOf(o) === Object.prototype)",
        "function F(){}; F.prototype = null; var o = new F(); \
         print(Object.getPrototypeOf(o) === Object.prototype)",
        "function F(){}; F.prototype = 'str'; print(Object.getPrototypeOf(new F()) === Object.prototype)",
        "function F(){}; F.prototype = undefined; print(Object.getPrototypeOf(new F()) === Object.prototype)",
        "function F(){}; delete F.prototype; print(Object.getPrototypeOf(new F()) === Object.prototype)",
        "function F(){ return 1 }; print(typeof new F())",
        "function F(){ return 'x' }; print(typeof new F())",
        "function F(){ return null }; print(typeof new F())",
        "function F(){ return undefined }; print(typeof new F())",
        "function F(){ return {tag:'ret'} }; print(new F().tag)",
        "function F(){ return [1,2] }; print(new F().length)",
        "function F(){ this.a=1; return 5 }; print(new F().a)",
    ] {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
    }
    // row 110 through the C API too
    for proto in ["1", "null", "undefined", "'s'", "({})", "[]"] {
        probe_state(&format!("construct proto={proto}"), 0, move || {
            job!(|l, j| {
                let cs = cstr(&format!("(function(){{ var F=function(){{}}; F.prototype={proto}; return F }})()"));
                let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
                if rc != 0 {
                    return format!("load={rc}");
                }
                l.js_pushundefined(j);
                let crc = l.js_pcall(j, 0);
                l.js_construct(j, 0);
                format!(
                    "call={crc} ty={} names={}",
                    from_c(l.js_typeof(j, -1)),
                    own_names(l, j, -1)
                )
            })
        });
    }

    // row 112: js_eval with a non-string on top -> returns immediately
    for s in SHAPES {
        probe_state(&format!("js_eval {s}"), 0, move || {
            job!(|l, j| {
                l.js_pushglobal(j); // js_eval does js_copy(J, 0) for `this`
                let rc = push_expr(l, j, s);
                let before = l.js_gettop(j);
                l.js_eval(j);
                format!(
                    "push={rc} before={before} after={} ty={} v={}",
                    l.js_gettop(j),
                    from_c(l.js_typeof(j, -1)),
                    from_c(l.js_tryrepr(j, -1, ERRSTR))
                )
            })
        });
    }
    // rows 113 / 114: js_pconstruct / js_pcall catch and trim the stack
    for s in ["(function(){throw 'boom'})", "(function(){return 7})", "1", "undefined"] {
        for extra in [0, 1, 2] {
            diff2(&format!("pcall/pconstruct {s} extra={extra}"), move |l| unsafe {
                let j = new_state(l, 0);
                let mut r = String::new();
                for which in 0..2 {
                    let base = l.js_gettop(j);
                    // `js_pconstruct` computes savetop = TOP - n - 2 (jsrun.c:1402)
                    // even though `js_construct` only wants the callee at -n-1,
                    // so on the error path it writes ONE SLOT BELOW the callee.
                    // Keep two guard slots underneath so that write always lands
                    // inside memory we own (with no guards and an empty stack it
                    // would be STACK[-1] -- out of bounds, so not tested).
                    l.js_pushstring(j, cn!("GUARD0"));
                    l.js_pushstring(j, cn!("GUARD1"));
                    // sentinels that js_pcall / js_pconstruct must leave alone
                    for i in 0..extra {
                        l.js_pushnumber(j, 900.0 + i as f64);
                    }
                    let prc = push_expr(l, j, s);
                    // js_pcall wants  [callee][this][arg...]
                    // js_pconstruct wants [callee][arg...]
                    if which == 0 {
                        l.js_pushundefined(j);
                    }
                    for i in 0..extra {
                        l.js_pushnumber(j, i as f64);
                    }
                    let rc = if which == 0 {
                        l.js_pcall(j, extra)
                    } else {
                        l.js_pconstruct(j, extra)
                    };
                    r.push_str(&format!(
                        " w{which}[push={prc} rc={rc} d={} v={}",
                        l.js_gettop(j) - base,
                        from_c(l.js_tryrepr(j, -1, ERRSTR))
                    ));
                    // the sentinels below the frame must be untouched
                    for k in 0..(l.js_gettop(j) - base) {
                        r.push_str(&format!(
                            " {k}={}",
                            from_c(l.js_trystring(j, base + k, ERRSTR))
                        ));
                    }
                    r.push(']');
                    drain_to(l, j, base);
                }
                l.js_freestate(j);
                r
            });
        }
    }
}

/// Row 105: `jsR_pushtrace` -> Error "call stack overflow" at 1024 frames.
#[test]
fn t_call_stack_overflow() {
    with_big_stack(|| {
        for src in [
            "function f(n){ return f(n+1) } \
             try { f(0) } catch (e) { print('E', typeof e, e instanceof Error, String(e)) }",
            "var d=0; function f(){ ++d; f() } \
             try { f() } catch (e) { print('E2', String(e), d > 500) }",
            "function f(n){ if (n<=0) return 0; return 1+f(n-1) } \
             try { print(f(900)) } catch (e) { print('E3', String(e)) }",
            "function f(n){ if (n<=0) return 0; return 1+f(n-1) } \
             try { print(f(1100)) } catch (e) { print('E4', String(e)) }",
            "function g(){ return (function(){ return g() })() } \
             try { g() } catch (e) { print('E5', String(e)) }",
        ] {
            diff_dostring(0, src);
            diff_dostring(JS_STRICT, src);
        }
        // the exact depth at which it trips must match
        diff2("call depth limit", |l| unsafe {
            let j = new_state(l, 0);
            let cs = cstr(
                "var max=0; function f(n){ if (n>max) max=n; f(n+1) } \
                 try { f(0) } catch (e) { } max",
            );
            let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
            l.js_pushundefined(j);
            let crc = l.js_pcall(j, 0);
            let r = format!(
                "load={rc} call={crc} max={}",
                from_c(l.js_tryrepr(j, -1, ERRSTR))
            );
            l.js_pop(j, 1);
            l.js_freestate(j);
            r
        });
    });
}

/* =========================================================================
 *  Rows 115-119, 133-146: the try stack.
 * ========================================================================= */

/// Everything reachable at `trytop == JS_TRYLIMIT` from a cfunction: js_ptry
/// (rows 133-146) and js_savetry (row 116).
#[derive(Clone, Copy, Debug)]
enum TryOp {
    PLoadString,
    PLoadStringBad,
    TryString,
    TryNumber,
    TryInteger,
    TryBoolean,
    DoString,
    DoStringBad,
    SaveTry,
    PCall,
    PConstruct,
}

const ALL_TRYOP: &[TryOp] = &[
    TryOp::PLoadString,
    TryOp::PLoadStringBad,
    TryOp::TryString,
    TryOp::TryNumber,
    TryOp::TryInteger,
    TryOp::TryBoolean,
    TryOp::DoString,
    TryOp::DoStringBad,
    TryOp::SaveTry,
    TryOp::PCall,
    TryOp::PConstruct,
];

thread_local! {
    static TRYOP: Cell<usize> = const { Cell::new(0) };
}

unsafe extern "C" fn cf_tryop(j: JS) {
    let l = cur();
    let op = ALL_TRYOP[TRYOP.with(|c| c.get())];
    let mut s = format!("{op:?}");
    let base = l.js_gettop(j);
    match op {
        TryOp::PLoadString => {
            let cs = cstr("1+1");
            let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
            s.push_str(&format!(" rc={rc} d={}", l.js_gettop(j) - base));
            if l.js_gettop(j) > base {
                s.push_str(&format!(" v={}", vshape_notry(l, j, -1)));
            }
        }
        TryOp::PLoadStringBad => {
            let cs = cstr("1 +* 2");
            let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
            s.push_str(&format!(" rc={rc} d={}", l.js_gettop(j) - base));
            if l.js_gettop(j) > base {
                s.push_str(&format!(" v={}", vshape_notry(l, j, -1)));
            }
        }
        TryOp::TryString => {
            l.js_pushnumber(j, 42.5);
            let r = l.js_trystring(j, -1, cn!("DEFAULT"));
            s.push_str(&format!(" s={:?} d={}", from_c(r), l.js_gettop(j) - base));
        }
        TryOp::TryNumber => {
            l.js_pushstring(j, cn!("17"));
            let r = l.js_trynumber(j, -1, -1.5);
            s.push_str(&format!(" n={r} d={}", l.js_gettop(j) - base));
        }
        TryOp::TryInteger => {
            l.js_pushstring(j, cn!("17.9"));
            let r = l.js_tryinteger(j, -1, -7);
            s.push_str(&format!(" i={r} d={}", l.js_gettop(j) - base));
        }
        TryOp::TryBoolean => {
            l.js_pushstring(j, cn!("x"));
            let r = l.js_tryboolean(j, -1, -3);
            s.push_str(&format!(" b={r} d={}", l.js_gettop(j) - base));
        }
        TryOp::DoString => {
            let cs = cstr("print('inner dostring')");
            let rc = l.js_dostring(j, cs.as_ptr());
            s.push_str(&format!(" rc={rc} d={}", l.js_gettop(j) - base));
        }
        TryOp::DoStringBad => {
            let cs = cstr("throw new TypeError('inner')");
            let rc = l.js_dostring(j, cs.as_ptr());
            s.push_str(&format!(" rc={rc} d={}", l.js_gettop(j) - base));
        }
        TryOp::SaveTry => {
            let b = l.js_savetry(j);
            l.js_endtry(j);
            s.push_str(&format!(" buf_null={} d={}", b.is_null(), l.js_gettop(j) - base));
        }
        TryOp::PCall => {
            l.js_newcfunction(j, Some(cf_nopush), cn!("np"), 0);
            l.js_pushundefined(j);
            let rc = l.js_pcall(j, 0);
            s.push_str(&format!(" rc={rc} d={}", l.js_gettop(j) - base));
        }
        TryOp::PConstruct => {
            l.js_newcfunction(j, Some(cf_nopush), cn!("np"), 0);
            let rc = l.js_pconstruct(j, 0);
            s.push_str(&format!(" rc={rc} d={}", l.js_gettop(j) - base));
        }
    }
    let t = l.js_gettop(j);
    if t > base {
        l.js_pop(j, t - base);
    }
    let cs = cstr(&s);
    l.js_pushstring(j, cs.as_ptr());
}

#[test]
fn t_try_limits() {
    for (oi, op) in ALL_TRYOP.iter().enumerate() {
        for n in [0usize, 1, 55, 60, 61, 62, 63, 64, 65, 70] {
            // n nested JS try blocks, then the C-level operation.
            let mut src = String::new();
            for _ in 0..n {
                src.push_str("try{");
            }
            src.push_str("print('P', probe())");
            for i in 0..n {
                src.push_str(&format!(
                    "}}catch(e){{print('c{i}', typeof e, e instanceof Error, String(e))}}"
                ));
            }
            diff2(&format!("tryop {op:?} nest={n}"), move |l| unsafe {
                TRYOP.with(|c| c.set(oi));
                let j = new_state(l, 0);
                l.js_newcfunction(j, Some(cf_tryop), cn!("probe"), 0);
                l.js_setglobal(j, cn!("probe"));
                let cs = cstr(&src);
                let rc = l.js_dostring(j, cs.as_ptr());
                let r = format!("rc={rc} top={}", l.js_gettop(j));
                l.js_freestate(j);
                r
            });
        }
    }
}

/// Rows 117 / 118 / 119: `js_endtry` at `trytop == 0` throws Error
/// "endtry: exception stack underflow", and a `js_throw` without a handler
/// runs the panic hook and then `abort()`s.  Both end the process, so they run
/// in a forked copy of this test binary.
#[test]
fn t_endtry_underflow_child() {
    let Ok(spec) = std::env::var("MUJS_ERRCORE_CHILD") else {
        return;
    };
    let (which, mode) = spec.split_once(':').expect("spec");
    let p = libs();
    let l = if which == "c" { &p.c } else { &p.rs };
    unsafe {
        set_cur(l);
        let j = l.js_newstate(None, std::ptr::null_mut(), 0);
        assert!(!j.is_null());
        l.js_setreport(j, Some(stderr_report));
        match mode {
            // row 117 -> js_error -> js_throw with trytop == 0 -> rows 118/119
            "endtry" => l.js_endtry(j),
            // row 119 with the panic hook cleared
            "nopanic" => {
                l.js_atpanic(j, None);
                l.js_pushstring(j, cn!("no-handler"));
                l.js_throw(j);
            }
            // row 118 with the default panic hook
            "throw" => {
                l.js_pushstring(j, cn!("no-handler"));
                l.js_throw(j);
            }
            _ => panic!("bad mode"),
        }
    }
    unreachable!("must not return");
}

unsafe extern "C" fn stderr_report(_j: JS, msg: *const c_char) {
    let s = format!("REPORT:{}\n", from_c(msg));
    extern "C" {
        fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    }
    write(2, s.as_ptr() as *const c_void, s.len());
}

#[test]
fn t_endtry_and_unhandled_throw() {
    if std::env::var_os("MUJS_ERRCORE_CHILD").is_some() {
        return;
    }
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    let exe = std::env::current_exe().expect("current_exe");
    for mode in ["endtry", "throw", "nopanic"] {
        let mut res = vec![];
        for which in ["c", "rs"] {
            let mut cmd = std::process::Command::new(&exe);
            cmd.args([
                "t_endtry_underflow_child",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("MUJS_ERRCORE_CHILD", format!("{which}:{mode}"))
            .env("RUST_BACKTRACE", "0");
            unsafe {
                cmd.pre_exec(|| {
                    #[repr(C)]
                    struct RLimit {
                        cur: u64,
                        max: u64,
                    }
                    extern "C" {
                        fn setrlimit(res: c_int, l: *const RLimit) -> c_int;
                    }
                    let rl = RLimit { cur: 0, max: 0 };
                    setrlimit(4 /* RLIMIT_CORE */, &rl);
                    Ok(())
                });
            }
            let out = cmd.output().expect("spawn child");
            let err = String::from_utf8_lossy(&out.stderr).into_owned();
            let marks: Vec<&str> = err.lines().filter(|l| l.starts_with("REPORT:")).collect();
            res.push(format!("signal={:?} marks={:?}", out.status.signal(), marks));
        }
        assert_eq!(res[0], res[1], "unhandled-throw ({mode}) divergence");
        assert!(
            res[0].contains("signal=Some(6)"),
            "expected SIGABRT for {mode}: {}",
            res[0]
        );
        if mode == "endtry" || mode == "throw" {
            assert!(
                res[0].contains("REPORT:uncaught exception"),
                "{mode} must report through the panic hook: {}",
                res[0]
            );
        } else {
            assert!(
                !res[0].contains("REPORT:"),
                "{mode} must not report: {}",
                res[0]
            );
        }
    }
}

/* =========================================================================
 *  Rows 120-132: the interpreter loop.
 * ========================================================================= */

#[test]
fn t_run_loop_paths() {
    // rows 120 / 121: jsR_isindex
    let keys = [
        "0", "1", "1.5", "-1", "-0", "NaN", "Infinity", "-Infinity", "4294967296",
        "2147483647", "2147483648", "1e21", "'0'", "'1'", "'1.5'", "true", "false",
        "null", "undefined", "{}", "[]", "[1]",
    ];
    for k in keys {
        for t in ["[10,20,30]", "({})", "new String('abc')", "/re/g"] {
            let src = format!(
                "var t={t}; try {{ t[{k}] = 'V'; \
                 print('r', t[{k}], t.length, Object.getOwnPropertyNames(t).join('|')) }} \
                 catch (e) {{ print('E', e) }}"
            );
            diff_dostring(0, &src);
            diff_dostring(JS_STRICT, &src);
        }
    }
    // row 128: `in` against a non-object
    for r in SHAPES {
        let src = format!("try {{ print('x' in {r}) }} catch (e) {{ print('E', e) }}");
        diff_dostring(0, &src);
        diff_dostring(JS_STRICT, &src);
    }
    // rows 129 / 130 / 131: for..in over non-coercible / non-object / exhausted
    for r in SHAPES {
        let src = format!(
            "var n=0; try {{ for (var k in {r}) ++n; print('n', n) }} \
             catch (e) {{ print('E', e) }}"
        );
        diff_dostring(0, &src);
        diff_dostring(JS_STRICT, &src);
    }
    // row 127: OP_HASVAR (typeof) pushes undefined instead of throwing
    for src in [
        "print(typeof nope)",
        "print(typeof nope === 'undefined')",
        "var x; print(typeof x)",
        "print(typeof typeof nope)",
        "if (typeof nope == 'undefined') print('ok')",
    ] {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
    }
    // rows 122 / 132: runlimit and OP_THROW
    for lim in [1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 1000] {
        diff2(&format!("runlimit {lim}"), move |l| unsafe {
            let j = new_state(l, 0);
            l.js_setlimit(j, lim, 0);
            let cs = cstr(
                "try { throw new Error('thrown') } catch (e) { print('c', String(e)) } \
                 var s=0; for (var i=0;i<50;++i) s+=i; print(s)",
            );
            let rc = l.js_dostring(j, cs.as_ptr());
            l.js_freestate(j);
            format!("rc={rc}")
        });
    }
    for src in [
        "throw 1",
        "throw 'str'",
        "throw null",
        "throw undefined",
        "throw {a:1}",
        "throw new Error('e')",
        "try { throw 1 } finally { print('fin') }",
        "try { try { throw 1 } finally { print('f1') } } catch (e) { print('c', e) }",
        "function f(){ throw 'inner' } try { f() } catch (e) { print('c', e) }",
    ] {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
    }
}

/// Row 123 + row 255: `J->gccounter > J->gcthresh` forces a `js_gc` from inside
/// `jsR_run`, and the new threshold is `remaining * JS_GCFACTOR`.  Observable
/// as the exact allocator call counts for a fixed script.
#[test]
fn t_forced_gc_in_run_loop() {
    #[repr(C)]
    #[derive(Default)]
    struct Cnt {
        nalloc: u64,
        nfree: u64,
        nrealloc: u64,
        live: i64,
        peak: i64,
    }
    unsafe extern "C" fn counting(actx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
        extern "C" {
            fn free(p: *mut c_void);
            fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
        }
        let cx = &mut *(actx as *mut Cnt);
        if size == 0 {
            if !ptr.is_null() {
                cx.nfree += 1;
                cx.live -= 1;
            }
            free(ptr);
            return std::ptr::null_mut();
        }
        let p = realloc(ptr, size as usize);
        if !p.is_null() {
            if ptr.is_null() {
                cx.nalloc += 1;
                cx.live += 1;
                if cx.live > cx.peak {
                    cx.peak = cx.live;
                }
            } else {
                cx.nrealloc += 1;
            }
        }
        p
    }
    for src in [
        "var a=[]; for (var i=0;i<2000;++i) a.push({k:i}); print(a.length)",
        "var s=''; for (var i=0;i<400;++i) s+='xy'; print(s.length)",
        "for (var i=0;i<3000;++i) { var o={a:i,b:i+1}; } print('done')",
        "var n=0; for (var i=0;i<1500;++i) { n += ('k'+i).length } print(n)",
        "var f=[]; for (var i=0;i<300;++i) f.push(function(){return i}); print(f.length)",
    ] {
        diff2(&format!("forced gc {src}"), move |l| unsafe {
            let mut cx = Cnt::default();
            set_cur(l);
            let j = l.js_newstate(Some(counting), &mut cx as *mut Cnt as *mut c_void, 0);
            assert!(!j.is_null());
            l.js_setreport(j, Some(report_cb));
            l.js_newcfunction(j, Some(print_cb), PRINT, 1);
            l.js_setglobal(j, PRINT);
            let cs = cstr(src);
            let rc = l.js_dostring(j, cs.as_ptr());
            l.js_freestate(j);
            format!(
                "rc={rc} nalloc={} nfree={} nrealloc={} live={}",
                cx.nalloc, cx.nfree, cx.nrealloc, cx.live
            )
        });
    }
}

/* =========================================================================
 *  Rows 144-152: jsstate.c beyond the try stack.
 * ========================================================================= */

#[test]
fn t_loadstring_paths() {
    // row 144: jsP_freeparse + rethrow on a parse or compile error
    let bad = [
        "1 +* 2",
        "var",
        "function",
        "{",
        "}",
        "'unterminated",
        "/*unterminated",
        "1 = 2",
        "for (;;",
        "var 1x = 1",
        "return 1",
        "break",
        "continue",
        "a: a: 1",
        "function f(a,a){'use strict'}",
        "delete x",
        "with(1){}",
        "\u{0}",
        "0x",
        "1e",
        ".e3",
        "var x = ;",
        "({a:1,",
        "new",
        "typeof",
    ];
    for s in bad {
        diff_dostring(0, s);
        diff_dostring(JS_STRICT, s);
        diff_eval(0, s);
        diff_eval(JS_STRICT, s);
        diff2(&format!("ploadstring bad {s:?}"), move |l| unsafe {
            let j = new_state(l, 0);
            let cs = cstr(s);
            let mut r = String::new();
            for _ in 0..3 {
                let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
                r.push_str(&format!("rc={rc} v={} ", vshape(l, j, -1)));
                l.js_pop(j, 1);
            }
            r.push_str(&format!("top={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
    }
    // row 146: js_dostring reports whatever js_trystring produces, or "Error"
    for s in [
        "throw new Error('plain')",
        "throw 'a string'",
        "throw 1",
        "throw null",
        "throw undefined",
        "throw {}",
        "throw Object.create(null)",
        "throw {toString: function(){ throw 'nested' }}",
        "throw {get message(){ throw 'g' }, toString: function(){ throw 't' }}",
        "throw {toString: function(){ return 'stringified' }}",
        "throw {toString: 1}",
        "nosuch()",
        "null.x",
        "undefined.x",
    ] {
        diff_dostring(0, s);
        diff_dostring(JS_STRICT, s);
    }
}

#[test]
fn t_newstate_alloc_and_report() {
    // rows 149 / 150 / 151: allocator failure at each call index
    #[repr(C)]
    struct FailAt {
        n: u64,
        fail_at: u64,
        live: i64,
    }
    unsafe extern "C" fn failing(actx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
        extern "C" {
            fn free(p: *mut c_void);
            fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
        }
        let cx = &mut *(actx as *mut FailAt);
        if size == 0 {
            if !ptr.is_null() {
                cx.live -= 1;
            }
            free(ptr);
            return std::ptr::null_mut();
        }
        cx.n += 1;
        if cx.n == cx.fail_at {
            return std::ptr::null_mut();
        }
        let p = realloc(ptr, size as usize);
        if !p.is_null() && ptr.is_null() {
            cx.live += 1;
        }
        p
    }
    for n in [1u64, 2, 3, 4, 5, 8, 16, 32, 64, 128, 256, 512, 1024] {
        diff2(&format!("newstate fail at {n}"), move |l| unsafe {
            let mut cx = FailAt {
                n: 0,
                fail_at: n,
                live: 0,
            };
            set_cur(l);
            let j = l.js_newstate(Some(failing), &mut cx as *mut FailAt as *mut c_void, 0);
            let isnull = j.is_null();
            if !isnull {
                l.js_freestate(j);
            }
            format!("null={isnull} calls={} live={}", cx.n, cx.live)
        });
    }
    // row 152: js_report with the reporter cleared
    diff2("js_report NULL hook", |l| unsafe {
        let j = new_state(l, 0);
        l.js_report(j, cn!("with hook"));
        l.js_setreport(j, None);
        l.js_report(j, cn!("hook cleared"));
        let cs = cstr("throw new Error('silent')");
        let rc = l.js_dostring(j, cs.as_ptr());
        l.js_setreport(j, Some(report_cb));
        l.js_report(j, cn!("hook back"));
        l.js_freestate(j);
        format!("rc={rc}")
    });
    // row 258: js_freestate(NULL) is a no-op
    diff2("js_freestate NULL", |l| unsafe {
        l.js_freestate(std::ptr::null_mut());
        l.js_freestate(std::ptr::null_mut());
        "survived".to_string()
    });
    // js_newstate flags: only bit 0 (JS_STRICT) is defined
    let mut rng = Rng::new(0xFEED_F1A6);
    let mut flags: Vec<c_int> = vec![
        0,
        1,
        2,
        3,
        4,
        7,
        8,
        0xFF,
        0xFFFF,
        -1,
        i32::MIN,
        i32::MAX,
        0x1000_0000,
    ];
    for _ in 0..24 {
        flags.push(rng.next_u32() as c_int);
    }
    for f in flags {
        diff2(&format!("newstate flags={f:#x}"), move |l| unsafe {
            let j = new_state(l, f);
            let cs = cstr("try { undeclared_x = 1; print('sloppy') } catch (e) { print('strict', e) }");
            let rc = l.js_dostring(j, cs.as_ptr());
            l.js_freestate(j);
            format!("rc={rc}")
        });
    }
}

/* =========================================================================
 *  Rows 153-166: jserror.c -- stack traces, Ep_toString, and the SEVEN
 *  printf-style variadic entry points (naked-asm trampolines in src/lib.rs).
 * ========================================================================= */

/// The seven `DERROR(name, Name)` throwers (jserror.c:101-107).
const VA_THROWERS: &[&str] = &[
    "js_error",
    "js_evalerror",
    "js_rangeerror",
    "js_referenceerror",
    "js_syntaxerror",
    "js_typeerror",
    "js_urierror",
];

/// The seven matching non-variadic constructors.
const NEW_ERRORS: &[&str] = &[
    "js_newerror",
    "js_newevalerror",
    "js_newrangeerror",
    "js_newreferenceerror",
    "js_newsyntaxerror",
    "js_newtypeerror",
    "js_newurierror",
];

type VaFn = unsafe extern "C" fn(JS, *const c_char, ...);

/// How many distinct variadic call shapes `call_va` knows.
const VA_CASES: usize = 34;

/// Drive one exact variadic call shape across the FFI.  Everything here is a
/// REAL C varargs call: register-passed args, SSE args, and args that spill onto
/// the stack past the 6 GP / 8 SSE registers.
unsafe fn call_va(f: VaFn, j: JS, case: usize, long_s: *const c_char, huge_fmt: *const c_char) {
    const A: *const c_char = cn!("Aaa");
    const B: *const c_char = cn!("Bbb");
    const C: *const c_char = cn!("Ccc");
    const D: *const c_char = cn!("Ddd");
    const E: *const c_char = cn!("Eee");
    const F_: *const c_char = cn!("Fff");
    const G: *const c_char = cn!("Ggg");
    const H: *const c_char = cn!("Hhh");
    const I: *const c_char = cn!("Iii");
    const K: *const c_char = cn!("Kkk");
    match case {
        /* 0 args -- the trampoline must still build a valid va_list */
        0 => f(j, cn!("no conversions at all")),
        1 => f(j, cn!("")),
        2 => f(j, cn!("100%% done")),
        /* single register args */
        3 => f(j, cn!("one string: %s"), A),
        4 => f(j, cn!("one int: %d"), 42 as c_int),
        5 => f(j, cn!("one negative: %d"), -42 as c_int),
        6 => f(j, cn!("int min: %d"), i32::MIN),
        7 => f(j, cn!("one double: %f"), 1.5f64),
        8 => f(j, cn!("one double g: %g"), 1.0e-7f64),
        9 => f(j, cn!("char: %c|"), 'Z' as c_int),
        10 => f(
            j,
            cn!("hex/oct/uns: %x %o %u"),
            255 as c_int,
            8 as c_int,
            u32::MAX as c_int,
        ),
        11 => f(
            j,
            cn!("width/prec: %8s|%-8s|%08d|%.3f"),
            A,
            B,
            7 as c_int,
            3.14159f64,
        ),
        /* mixtures that still fit in registers */
        12 => f(j, cn!("%s = %d"), A, 7 as c_int),
        13 => f(j, cn!("%s %s %s %s"), A, B, C, D),
        14 => f(j, cn!("%d %d %d %d"), 1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int),
        15 => f(j, cn!("%s %d %f"), A, 5 as c_int, 2.5f64),
        /* GP spill: 10 pointers, only 4 fit in registers after (J, fmt) */
        16 => f(
            j,
            cn!("%s %s %s %s %s %s %s %s %s %s"),
            A, B, C, D, E, F_, G, H, I, K,
        ),
        17 => f(
            j,
            cn!("%d %d %d %d %d %d %d %d %d %d %d %d"),
            1 as c_int, 2 as c_int, 3 as c_int, 4 as c_int, 5 as c_int, 6 as c_int,
            7 as c_int, 8 as c_int, 9 as c_int, 10 as c_int, 11 as c_int, 12 as c_int,
        ),
        /* SSE spill: 10 doubles, only 8 fit in XMM0-7 */
        18 => f(
            j,
            cn!("%g %g %g %g %g %g %g %g %g %g"),
            1.0f64, 2.0f64, 3.0f64, 4.0f64, 5.0f64, 6.0f64, 7.0f64, 8.0f64, 9.0f64, 10.0f64,
        ),
        19 => f(
            j,
            cn!("%.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f"),
            0.5f64, 1.5f64, 2.5f64, 3.5f64, 4.5f64, 5.5f64, 6.5f64, 7.5f64, 8.5f64,
            9.5f64, 10.5f64, 11.5f64,
        ),
        /* both classes spilling at once, interleaved */
        20 => f(
            j,
            cn!("%s%g %s%g %s%g %s%g %s%g %s%g"),
            A, 1.0f64, B, 2.0f64, C, 3.0f64, D, 4.0f64, E, 5.0f64, F_, 6.0f64,
        ),
        21 => f(
            j,
            cn!("%d%g %d%g %d%g %d%g %d%g %d%g %d%g %d%g %d%g %d%g"),
            1 as c_int, 1.25f64, 2 as c_int, 2.25f64, 3 as c_int, 3.25f64,
            4 as c_int, 4.25f64, 5 as c_int, 5.25f64, 6 as c_int, 6.25f64,
            7 as c_int, 7.25f64, 8 as c_int, 8.25f64, 9 as c_int, 9.25f64,
            10 as c_int, 10.25f64,
        ),
        /* long / long long width specifiers */
        22 => f(j, cn!("%ld %lld"), -1234567890123i64, 987654321098765i64),
        /* precision / width taken from an argument */
        23 => f(j, cn!("[%.*s]"), 2 as c_int, A),
        24 => f(j, cn!("[%*d]"), 6 as c_int, 42 as c_int),
        /* NULL "%s": glibc renders "(null)"; both libraries use the same libc */
        25 => f(j, cn!("null string: %s"), std::ptr::null::<c_char>()),
        26 => f(j, cn!("%s and %s"), std::ptr::null::<c_char>(), A),
        /* TRUNCATION: message longer than the 256 byte buffer (row 159) */
        27 => f(j, cn!("long arg: %s"), long_s),
        28 => f(j, cn!("%s%s%s"), long_s, long_s, long_s),
        29 => f(j, huge_fmt),
        30 => f(j, huge_fmt, A, 1 as c_int, 2.0f64),
        /* exactly at / around the 255-byte cut */
        31 => f(j, cn!("%.255s"), long_s),
        32 => f(j, cn!("%.254s!"), long_s),
        33 => f(j, cn!("%.256s"), long_s),
        _ => unreachable!(),
    }
}

thread_local! {
    static VA_CASE: Cell<usize> = const { Cell::new(0) };
    static VA_NAME: RefCell<String> = const { RefCell::new(String::new()) };
}

/// 300 'v's and a 300-byte conversion-free format string, both NUL terminated.
fn long_strings() -> (CString, CString) {
    let long_s = CString::new("v".repeat(300)).unwrap();
    let mut f = String::new();
    while f.len() < 300 {
        f.push_str("FMT-");
    }
    (long_s, CString::new(f).unwrap())
}

unsafe extern "C" fn cf_va(j: JS) {
    let l = cur();
    let name = VA_NAME.with(|n| n.borrow().clone());
    let case = VA_CASE.with(|c| c.get());
    let (long_s, huge_fmt) = long_strings();
    let f: VaFn = match name.as_str() {
        "js_error" => l.raw2("js_error"),
        "js_evalerror" => l.raw2("js_evalerror"),
        "js_rangeerror" => l.raw2("js_rangeerror"),
        "js_referenceerror" => l.raw2("js_referenceerror"),
        "js_syntaxerror" => l.raw2("js_syntaxerror"),
        "js_typeerror" => l.raw2("js_typeerror"),
        "js_urierror" => l.raw2("js_urierror"),
        _ => unreachable!(),
    };
    call_va(f, j, case, long_s.as_ptr(), huge_fmt.as_ptr());
    // JS_NORETURN in C: reaching here at all is a divergence
    l.js_pushstring(j, cn!("VARARG THROWER RETURNED"));
}

/// Rows 159-166: every variadic thrower, every call shape, checked for the
/// resulting error class AND the exact formatted (and truncated) message.
#[test]
fn t_error_varargs_ffi() {
    for name in VA_THROWERS {
        for case in 0..VA_CASES {
            diff2(&format!("va {name} case={case}"), move |l| unsafe {
                VA_NAME.with(|n| *n.borrow_mut() = name.to_string());
                VA_CASE.with(|c| c.set(case));
                let j = new_state(l, 0);
                let base = l.js_gettop(j);
                l.js_newcfunction(j, Some(cf_va), cn!("va"), 0);
                l.js_pushundefined(j);
                let rc = l.js_pcall(j, 0);
                let mut r = format!("rc={rc} {}", vshape(l, j, -1));
                l.js_setregistry(j, cn!("err"));
                r.push_str(&format!(" msg={}", read_msg(l, j)));
                drain_to(l, j, base);
                l.js_freestate(j);
                r
            });
        }
    }
}

/// `registry.err.message` + its length + `name`, read inside a protected frame.
unsafe fn read_msg(l: &Lib, j: JS) -> String {
    probe(
        l,
        j,
        job!(|l, j| {
            l.js_getregistry(j, cn!("err"));
            if l.pred("js_isobject", j, -1) == 0 {
                return format!("nonobject:{}", from_c(l.js_trystring(j, -1, ERRSTR)));
            }
            let h = l.js_hasproperty(j, -1, cn!("message"));
            if h == 0 {
                return "<no message>".to_string();
            }
            let s = from_c(l.js_tostring(j, -1));
            l.js_pop(j, 1);
            let n = l.js_hasproperty(j, -1, cn!("name"));
            let nm = if n != 0 {
                let v = from_c(l.js_tostring(j, -1));
                l.js_pop(j, 1);
                v
            } else {
                "<noname>".to_string()
            };
            format!("name={nm} len={} m={s:?}", s.len())
        }),
    )
}

/// `registry.err[prop]`, read inside a protected frame.
unsafe fn read_prop(l: &Lib, j: JS, prop: &str) -> String {
    let p = prop.to_string();
    probe(
        l,
        j,
        job!(|l, j| {
            let cs = cstr(&p);
            l.js_getregistry(j, cn!("err"));
            let h = l.js_hasproperty(j, -1, cs.as_ptr());
            if h == 0 {
                return "<absent>".to_string();
            }
            let s = from_c(l.js_tostring(j, -1));
            format!("len={} {s:?}", s.len())
        }),
    )
}

/// Rows 160-166 through the non-variadic constructors, with normal / empty /
/// 255 / 256 / 257-byte / embedded-`%` messages.  `js_new*error` does NOT throw,
/// so it can be called straight from the test; at `tracetop == 0` that is also
/// row 153 (`jsB_stacktrace` returns 0, so no `stackTrace` property).
#[test]
fn t_newerror_family() {
    let mut msgs: Vec<String> = vec![
        String::new(),
        "plain".into(),
        "with %s and %d and %%".into(),
        "%n%n%n".into(),
        "100%".into(),
        "\ttabs\nand\nnewlines".into(),
        "\u{4e2d}\u{6587}".into(),
    ];
    for n in [1usize, 14, 15, 16, 17, 254, 255, 256, 257, 258, 300, 1024] {
        msgs.push("m".repeat(n));
    }
    let mut rng = Rng::new(0x9E37_79B9);
    for _ in 0..20 {
        msgs.push(rng.ascii_string(40));
    }
    for name in NEW_ERRORS {
        for m in &msgs {
            let m = m.clone();
            diff2(&format!("{name} len={}", m.len()), move |l| unsafe {
                let j = new_state(l, 0);
                let cs = cstr(&m);
                // row 153: tracetop == 0 here, so no stackTrace is attached
                l.newerror(name, j, cs.as_ptr());
                let mut r = format!("top={} {}", l.js_gettop(j), vshape(l, j, -1));
                l.js_setregistry(j, cn!("err"));
                r.push_str(&format!(" {}", read_msg(l, j)));
                r.push_str(&format!(" trace={}", read_prop(l, j, "stackTrace")));
                r.push_str(&format!(" stack={}", read_prop(l, j, "stack")));
                // and again from inside a call frame, where tracetop > 0
                let r2 = probe(
                    l,
                    j,
                    job!(|l, j| {
                        let cs2 = cstr("inner");
                        l.newerror(name, j, cs2.as_ptr());
                        // read the properties FIRST: js_tostring rewrites the
                        // slot it is handed (jsvalue.c:360)
                        let h = l.js_hasproperty(j, -1, cn!("stackTrace"));
                        let t = if h != 0 {
                            let v = from_c(l.js_tostring(j, -1));
                            l.js_pop(j, 1);
                            v
                        } else {
                            "<none>".to_string()
                        };
                        let s = from_c(l.js_trystring(j, -1, ERRSTR));
                        format!("s={s:?} trace={t:?}")
                    }),
                );
                r.push_str(&format!(" inframe={r2}"));
                l.js_freestate(j);
                r
            });
        }
    }
}

/// Rows 153 / 154: the stack-trace builder -- no frames left after `skip`, and
/// a single trace line longer than 255 bytes (snprintf into `char buf[256]`).
#[test]
fn t_stacktrace_truncation() {
    for (fnlen, filelen) in [
        (0usize, 0usize),
        (4, 8),
        (100, 100),
        (240, 8),
        (250, 8),
        (255, 8),
        (260, 8),
        (300, 8),
        (8, 240),
        (8, 250),
        (8, 260),
        (8, 400),
        (300, 400),
    ] {
        let fname = format!("f{}", "N".repeat(fnlen));
        let file = format!("{}.js", "P".repeat(filelen));
        for depth in [1usize, 2, 3] {
            // depth separate functions, innermost first, so the trace has
            // `depth` frames each carrying the (very long) name and filename.
            let mut src = String::new();
            src.push_str(&format!(
                "function {fname}{}(){{ return new Error('boom') }} ",
                depth - 1
            ));
            for d in (0..depth - 1).rev() {
                src.push_str(&format!(
                    "function {fname}{d}(){{ return {fname}{}() }} ",
                    d + 1
                ));
            }
            src.push_str(&format!("{fname}0()"));
            let file2 = file.clone();
            diff2(
                &format!("stacktrace fn={fnlen} file={filelen} depth={depth}"),
                move |l| unsafe {
                    let j = new_state(l, 0);
                    let cf = cstr(&file2);
                    let cs = cstr(&src);
                    let rc = l.js_ploadstring(j, cf.as_ptr(), cs.as_ptr());
                    let mut r = format!("load={rc}");
                    if rc == 0 {
                        l.js_pushundefined(j);
                        let crc = l.js_pcall(j, 0);
                        r.push_str(&format!(" call={crc}"));
                        l.js_setregistry(j, cn!("err"));
                        r.push_str(&format!(" trace={}", read_prop(l, j, "stackTrace")));
                        r.push_str(&format!(" stack={}", read_prop(l, j, "stack")));
                        r.push_str(&format!(" msg={}", read_msg(l, j)));
                    } else {
                        r.push_str(&format!(" err={}", vshape(l, j, -1)));
                        l.js_pop(j, 1);
                    }
                    l.js_freestate(j);
                    r
                },
            );
        }
    }
    // deep traces, where jsB_stacktrace concatenates many lines
    for depth in [1usize, 2, 5, 20, 60] {
        let mut src = String::from("function mk(){ return new Error('deep') } ");
        for d in 0..depth {
            let callee = if d == 0 {
                "mk".to_string()
            } else {
                format!("f{}", d - 1)
            };
            src.push_str(&format!("function f{d}(){{ return {callee}() }} "));
        }
        src.push_str(&format!(
            "var e = f{}(); print(e.stackTrace.split('\\n').length, e.stackTrace.length)",
            depth - 1
        ));
        diff_dostring(0, &src);
        diff_dostring(JS_STRICT, &src);
    }
    // row 153 from JS
    for src in [
        "var e = new Error('x'); print('stackTrace' in e, typeof e.stackTrace)",
        "var e = new Error(); print('message' in e, e.message, String(e))",
        "print(Object.getOwnPropertyNames(new Error('x')).sort().join('|'))",
        "function f(){ return new Error('d') } print(f().stackTrace)",
        "print(new Error('t').stack)",
    ] {
        diff_dostring(0, src);
    }
}

/// Rows 155-158: Ep_toString / jsB_ErrorX.
#[test]
fn t_error_prototype_tostring() {
    for recv in [
        "new Error('m')",
        "new TypeError('t')",
        "({})",
        "({name:'N',message:'M'})",
        "({name:'',message:'M'})",
        "({name:'N',message:''})",
        "({name:'',message:''})",
        "Object.create(null)",
        "({get name(){return 'G'}, message:'M'})",
        "({get name(){throw 'gn'}})",
        "({get message(){throw 'gm'}})",
        "({name:1,message:2})",
        "({name:{},message:{}})",
        "1",
        "'s'",
        "true",
        "null",
        "undefined",
        "[]",
        "(function(){})",
    ] {
        for src in [
            format!("print(Error.prototype.toString.call({recv}))"),
            format!("print(String({recv}))"),
            format!(
                "var d = Object.getOwnPropertyDescriptor(Error.prototype,'stack'); \
                 print(typeof d.get); print(d.get.call({recv}))"
            ),
        ] {
            let w = format!("try {{ {src} }} catch (e) {{ print('E', e) }}");
            diff_dostring(0, &w);
            diff_dostring(JS_STRICT, &w);
        }
    }
    // row 158: `new Error(arg)` with argument 1 undefined / absent
    for ctor in [
        "Error", "EvalError", "RangeError", "ReferenceError", "SyntaxError", "TypeError",
        "URIError",
    ] {
        for arg in ["", "undefined", "null", "''", "0", "'m'", "{}", "[]", "false"] {
            let src = format!(
                "var e = new {ctor}({arg}); \
                 print('{ctor}', 'message' in e, e.message, e.name, String(e), \
                 Object.getOwnPropertyNames(e).sort().join('|'));\
                 var f = {ctor}({arg}); print('call', f instanceof {ctor}, String(f))"
            );
            diff_dostring(0, &src);
            diff_dostring(JS_STRICT, &src);
        }
    }
}

/* =========================================================================
 *  Rows 167-186: jsvalue.c numeric conversions (all directly exported).
 * ========================================================================= */

#[test]
fn t_strtol_invalid_digits() {
    let mut strs: Vec<String> = vec![
        String::new(),
        "0".into(),
        "9".into(),
        "a".into(),
        "z".into(),
        "Z".into(),
        "A".into(),
        "10".into(),
        "ff".into(),
        "FF".into(),
        "0x10".into(),
        "-5".into(),
        "+5".into(),
        " 5".into(),
        "5 ".into(),
        "1_2".into(),
        "12.5".into(),
        "1e3".into(),
        "\u{7f}".into(),
        "9999999999999999999999".into(),
        "zzzzzzzzzzzzz".into(),
        "!".into(),
        "~".into(),
        "@".into(),
        "`".into(),
        "{".into(),
        "[".into(),
    ];
    let mut rng = Rng::new(0x1670_0000);
    for _ in 0..300 {
        strs.push(rng.ascii_string(10));
    }
    // base 0..80 is well defined: the widest table entry is 80, so
    // `table[c] < base` is false for the NUL terminator for every base <= 80.
    // base 81 and up would walk past the end of the string -- C UB, not tested.
    let bases: Vec<c_int> = vec![
        0, 1, 2, 3, 8, 9, 10, 11, 16, 17, 35, 36, 37, 40, 60, 79, 80, -1, -10,
    ];
    diff2("js_strtol", move |l| unsafe {
        let mut r = String::new();
        for s in &strs {
            let cs = cstr(s);
            for &b in &bases {
                let mut ep: *mut c_char = std::ptr::null_mut();
                let v = l.js_strtol(cs.as_ptr(), &mut ep, b);
                let off = if ep.is_null() {
                    -1
                } else {
                    ep as isize - cs.as_ptr() as isize
                };
                r.push_str(&format!("{s:?}/{b}={:016x}@{off} ", fbits(v)));
            }
            // and with a NULL end pointer, which the C explicitly tests for
            let v2 = l.js_strtol(cs.as_ptr(), std::ptr::null_mut(), 10);
            r.push_str(&format!("nullep={:016x}\n", fbits(v2)));
        }
        r
    });
}

#[test]
fn t_numbertointeger_and_int32() {
    let mut xs: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1.5,
        -1.5,
        i32::MAX as f64,
        i32::MAX as f64 + 1.0,
        i32::MAX as f64 - 0.5,
        i32::MIN as f64,
        i32::MIN as f64 - 1.0,
        i32::MIN as f64 + 0.5,
        2147483647.9,
        -2147483648.9,
        4294967295.0,
        4294967296.0,
        4294967297.0,
        -4294967296.0,
        2147483648.0,
        -2147483649.0,
        1e300,
        -1e300,
        5e-324,
        -5e-324,
        1e21,
        65535.0,
        65536.0,
        32767.0,
        32768.0,
        -32768.0,
        -32769.0,
        f64::MAX,
        f64::MIN,
    ];
    let mut rng = Rng::new(0x1680_1730);
    for _ in 0..2500 {
        xs.push(rng.f64_sane());
    }
    for _ in 0..800 {
        xs.push(rng.f64_any());
    }
    let xs2 = xs.clone();
    diff2("jsV_numbertointeger family", move |l| unsafe {
        let mut r = String::new();
        for &x in &xs {
            r.push_str(&format!(
                "{:016x}=>{} {} {} {} {}\n",
                fbits(x),
                l.jsV_numbertointeger(x),
                l.jsV_numbertoint32(x),
                l.jsV_numbertouint32(x),
                l.jsV_numbertoint16(x),
                l.jsV_numbertouint16(x),
            ));
        }
        r
    });
    // and through the stack conversions
    diff2("js_tointeger family", move |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for &x in xs2.iter().take(500) {
            l.js_pushnumber(j, x);
            r.push_str(&format!(
                "{:016x}=>{} {} {} {} {} {}\n",
                fbits(x),
                l.js_tointeger(j, -1),
                l.js_toint32(j, -1),
                l.js_touint32(j, -1),
                l.js_toint16(j, -1),
                l.js_touint16(j, -1),
                l.js_toboolean(j, -1),
            ));
            l.js_pop(j, 1);
        }
        l.js_freestate(j);
        r
    });
}

#[test]
fn t_stringtofloat_and_stringtonumber() {
    let mut strs: Vec<String> = vec![
        String::new(),
        ".".into(),
        "e5".into(),
        "+".into(),
        "-".into(),
        "+.".into(),
        "-.".into(),
        ".e3".into(),
        "e".into(),
        "E".into(),
        "0".into(),
        "-0".into(),
        "+0".into(),
        "1".into(),
        "1.".into(),
        ".1".into(),
        "1.5".into(),
        "1e3".into(),
        "1E3".into(),
        "1e+3".into(),
        "1e-3".into(),
        "1e".into(),
        "1e+".into(),
        "1e-".into(),
        "0x".into(),
        "0X".into(),
        "0x0".into(),
        "0x10".into(),
        "0X1f".into(),
        "0xg".into(),
        "00x10".into(),
        "Infinity".into(),
        "+Infinity".into(),
        "-Infinity".into(),
        "Infinity ".into(),
        " Infinity".into(),
        "Infinityx".into(),
        "infinity".into(),
        "12abc".into(),
        "1 2".into(),
        "  12  ".into(),
        "\t\n\r 12 \t\n".into(),
        "\u{b}12".into(),
        "\u{c}12".into(),
        "1e1000".into(),
        "1e-1000".into(),
        "-1e1000".into(),
        "99999999999999999999999".into(),
        "0.000000000000000000001".into(),
        "NaN".into(),
        "nan".into(),
        "null".into(),
        "true".into(),
    ];
    let mut rng = Rng::new(0x1810_1830);
    for _ in 0..600 {
        strs.push(rng.ascii_string(12));
    }
    for _ in 0..400 {
        let mut s = String::new();
        for _ in 0..(1 + rng.below(8)) {
            s.push(match rng.below(8) {
                0 => '0',
                1 => '.',
                2 => 'e',
                3 => '+',
                4 => '-',
                5 => 'x',
                6 => ' ',
                _ => (b'0' + rng.below(10) as u8) as char,
            });
        }
        strs.push(s);
    }
    diff2("js_stringtofloat / jsV_stringtonumber", move |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for s in &strs {
            let cs = cstr(s);
            let mut ep: *mut c_char = std::ptr::null_mut();
            let v = l.js_stringtofloat(cs.as_ptr(), &mut ep);
            let off = if ep.is_null() {
                -1
            } else {
                ep as isize - cs.as_ptr() as isize
            };
            let n = l.jsV_stringtonumber(j, cs.as_ptr());
            r.push_str(&format!(
                "{s:?} f={:016x}@{off} n={:016x}\n",
                fbits(v),
                fbits(n)
            ));
        }
        l.js_freestate(j);
        r
    });
}

#[test]
fn t_numbertostring() {
    let mut xs: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0,
        -1.0,
        0.1,
        1e21,
        1e-7,
        1e-6,
        123456789012345680000.0,
        5e-324,
        f64::MAX,
        i32::MAX as f64,
        i32::MIN as f64,
        i32::MAX as f64 + 1.0,
        i32::MIN as f64 - 1.0,
    ];
    let mut rng = Rng::new(0x1840_1860);
    for _ in 0..1500 {
        xs.push(rng.f64_sane());
    }
    diff2("jsV_numbertostring", move |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for &x in &xs {
            let mut buf = [0u8; 64];
            let p = l.jsV_numbertostring(j, buf.as_mut_ptr() as *mut c_char, x);
            // whether the STATIC literal or the caller buffer was returned is
            // itself observable and part of the contract (jsvalue.c:275-277)
            let isbuf = p == buf.as_ptr() as *const c_char;
            r.push_str(&format!("{:016x}=>{:?} buf={isbuf}\n", fbits(x), from_c(p)));
        }
        l.js_freestate(j);
        r
    });
}

/* =========================================================================
 *  Rows 174-180: jsV_toString / jsV_valueOf / jsV_toprimitive.
 * ========================================================================= */

/// JS_HNONE / JS_HNUMBER / JS_HSTRING are 0/1/2 (jsi.h:297).  Any other int is
/// well defined (it just takes the valueOf-first branch), so out-of-range hints
/// are part of the FFI surface.
const HINTS: &[c_int] = &[0, 1, 2, 3, 255, -1, i32::MIN, i32::MAX];

const PRIM_OBJS: &[&str] = &[
    "({})",
    "[]",
    "[1,2]",
    "Object.create(null)",
    "({toString:null, valueOf:null})",
    "({toString:1, valueOf:2})",
    "({toString:function(){return 'TS'}})",
    "({valueOf:function(){return 7}})",
    "({toString:function(){return 'TS'}, valueOf:function(){return 7}})",
    "({toString:function(){return {}}, valueOf:function(){return {}}})",
    "({toString:function(){return {}}, valueOf:function(){return 5}})",
    "({toString:function(){return 'TS'}, valueOf:function(){return {}}})",
    "({toString:function(){throw 'ts-throw'}})",
    "({valueOf:function(){throw 'vo-throw'}})",
    "new Date(0)",
    "new Number(3)",
    "new String('s')",
    "new Boolean(false)",
    "/re/g",
    "(function(){})",
    "1",
    "'s'",
    "null",
    "undefined",
    "true",
    "Math",
];

#[test]
fn t_toprimitive() {
    for (oi, _) in PRIM_OBJS.iter().enumerate() {
        for &h in HINTS {
            for flags in [0, JS_STRICT] {
                probe_state(
                    &format!("toprimitive {} hint={h} flags={flags}", PRIM_OBJS[oi]),
                    flags,
                    move || {
                        job!(|l, j| {
                            let rc = push_expr(l, j, PRIM_OBJS[oi]);
                            let tp: unsafe extern "C" fn(JS, c_int, c_int) =
                                l.raw2("js_toprimitive");
                            tp(j, -1, h);
                            format!("push={rc} {}", vshape(l, j, -1))
                        })
                    },
                );
            }
        }
    }
}

#[test]
fn t_toprimitive_through_operators() {
    for o in PRIM_OBJS {
        for flags in [0, JS_STRICT] {
            for expr in [
                format!("String({o})"),
                format!("Number({o})"),
                format!("({o}) + ''"),
                format!("({o}) + 1"),
                format!("({o}) * 2"),
                format!("({o}) < 1"),
                format!("({o}) == 1"),
                format!("[{o}].join(',')"),
                format!("JSON.stringify({o})"),
                format!("({{}})[{o}]"),
            ] {
                let w = format!("try {{ print({expr}) }} catch (e) {{ print('E', e) }}");
                diff_dostring(flags, &w);
            }
        }
    }
}

/* =========================================================================
 *  Rows 187-196: jsV_toobject, js_newobjectx, js_newuserdatax, js_instanceof.
 * ========================================================================= */

#[test]
fn t_toobject_errors() {
    for (si, _) in SHAPES.iter().enumerate() {
        probe_state(&format!("js_toobject {}", SHAPES[si]), 0, move || {
            job!(|l, j| {
                let rc = push_expr(l, j, SHAPES[si]);
                let to: unsafe extern "C" fn(JS, c_int) -> *mut c_void = l.raw2("js_toobject");
                let o = to(j, -1);
                format!(
                    "push={rc} null={} ty_after={}",
                    o.is_null(),
                    from_c(l.js_typeof(j, -1))
                )
            })
        });
    }
    for src in [
        "try { undefined.x } catch (e) { print('E', e) }",
        "try { null.x } catch (e) { print('E', e) }",
        "try { undefined.x = 1 } catch (e) { print('E', e) }",
        "try { null.x = 1 } catch (e) { print('E', e) }",
        "try { delete undefined.x } catch (e) { print('E', e) }",
        "try { for (var k in undefined) print(k) } catch (e) { print('E', e) }",
        "try { Object(undefined).x } catch (e) { print('E', e) }",
        "try { undefined[0] } catch (e) { print('E', e) }",
        "try { null[0] = 1 } catch (e) { print('E', e) }",
        "try { with(undefined){} } catch (e) { print('E', e) }",
        "try { with(null){} } catch (e) { print('E', e) }",
        "try { Object.keys(null) } catch (e) { print('E', e) }",
        "try { undefined.toString() } catch (e) { print('E', e) }",
    ] {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
    }
}

#[test]
fn t_newobjectx_userdatax_proto() {
    // rows 189 / 191: a non-object prototype argument yields prototype == NULL
    for (si, _) in SHAPES.iter().enumerate() {
        probe_state(&format!("js_newobjectx proto={}", SHAPES[si]), 0, move || {
            job!(|l, j| {
                let rc = push_expr(l, j, SHAPES[si]);
                let before = l.js_gettop(j);
                l.js_newobjectx(j);
                let mut r = format!("push={rc} d={}", l.js_gettop(j) - before);
                r.push_str(&format!(" ty={}", from_c(l.js_typeof(j, -1))));
                let h = l.js_hasproperty(j, -1, cn!("toString"));
                r.push_str(&format!(" hasTS={h}"));
                if h != 0 {
                    l.js_pop(j, 1);
                }
                r
            })
        });
        probe_state(&format!("js_newuserdata proto={}", SHAPES[si]), 0, move || {
            job!(|l, j| {
                let rc = push_expr(l, j, SHAPES[si]);
                let before = l.js_gettop(j);
                l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, None);
                let h = l.js_hasproperty(j, -1, cn!("toString"));
                let r = format!(
                    "push={rc} d={} isud={} hasTS={h}",
                    l.js_gettop(j) - before,
                    l.js_isuserdata(j, -1, N_TAG),
                );
                if h != 0 {
                    l.js_pop(j, 1);
                }
                r
            })
        });
    }
}

/// Rows 190 / 192: `js_newcfunctionx` / `js_newuserdatax` run `finalize(J, data)`
/// from their `js_try` handler and rethrow.  Squeeze the memlimit shut to force
/// the allocation failure.
#[test]
fn t_oom_finalize() {
    for lim in [1i32, 8, 32, 64, 128, 256, 512, 1024, 4096] {
        diff2(&format!("oom finalize cfunctionx lim={lim}"), move |l| unsafe {
            let _ = hooks_take();
            let j = new_state(l, 0);
            let r = probe(
                l,
                j,
                job!(|l, j| {
                    l.js_setlimit(j, 0, lim);
                    l.js_newcfunctionx(
                        j,
                        Some(cf_nopush),
                        cn!("oomf"),
                        0,
                        PAYLOAD as *mut c_void,
                        Some(ud_fin),
                    );
                    "constructed".to_string()
                }),
            );
            let hooks = hooks_take();
            l.js_setlimit(j, 0, 0);
            l.js_freestate(j);
            format!("{r} hooks={hooks} after={}", hooks_take())
        });
        diff2(&format!("oom finalize userdatax lim={lim}"), move |l| unsafe {
            let _ = hooks_take();
            let j = new_state(l, 0);
            let r = probe(
                l,
                j,
                job!(|l, j| {
                    l.js_newobject(j);
                    l.js_setlimit(j, 0, lim);
                    l.js_newuserdatax(
                        j,
                        N_TAG,
                        PAYLOAD as *mut c_void,
                        Some(ud_has),
                        Some(ud_put),
                        Some(ud_del),
                        Some(ud_fin),
                    );
                    "constructed".to_string()
                }),
            );
            let hooks = hooks_take();
            l.js_setlimit(j, 0, 0);
            l.js_freestate(j);
            format!("{r} hooks={hooks} after={}", hooks_take())
        });
    }
}

#[test]
fn t_instanceof() {
    const LHS: &[&str] = &[
        "1", "'s'", "true", "null", "undefined", "({})", "[]", "(function(){})",
        "new Error('e')", "new Date(0)", "/re/", "Object.create(null)",
    ];
    const RHS: &[&str] = &[
        "Object", "Array", "Function", "Error", "TypeError", "Date", "RegExp", "1", "'s'",
        "null", "undefined", "({})", "[]", "(function(){})",
        "(function(){ var f=function(){}; f.prototype = 1; return f })()",
        "(function(){ var f=function(){}; f.prototype = null; return f })()",
        "(function(){ var f=function(){}; delete f.prototype; return f })()",
        "(function(){ var f=function(){}; f.prototype = {}; return f })()",
    ];
    for (ai, _) in LHS.iter().enumerate() {
        for (bi, _) in RHS.iter().enumerate() {
            let src = format!(
                "try {{ print({} instanceof {}) }} catch (e) {{ print('E', e) }}",
                LHS[ai], RHS[bi]
            );
            diff_dostring(0, &src);
            probe_state(
                &format!("js_instanceof {} {}", LHS[ai], RHS[bi]),
                0,
                move || {
                    job!(|l, j| {
                        let ra = push_expr(l, j, LHS[ai]);
                        let rb = push_expr(l, j, RHS[bi]);
                        let r = l.nullary_i("js_instanceof", j);
                        format!("a={ra} b={rb} r={r} top={}", l.js_gettop(j))
                    })
                },
            );
        }
    }
    // row 196: chain exhausted
    for src in [
        "function A(){}; function B(){}; B.prototype = Object.create(A.prototype); \
         var b = new B(); print(b instanceof B, b instanceof A, b instanceof Object)",
        "var o = Object.create(null); print(o instanceof Object)",
        "print(Object.prototype instanceof Object)",
    ] {
        diff_dostring(0, src);
    }
}

/* =========================================================================
 *  Rows 197-204: js_concat / js_compare / js_equal / js_strictequal.
 * ========================================================================= */

#[test]
fn t_concat_limits() {
    // row 198: the temporary buffer allocation exceeds the memlimit
    for lim in [1i32, 4, 8, 16, 64, 256, 1024, 4096, 16384] {
        for a in ["''", "'a'", "'abcdefgh'", "1", "{}", "[1,2]"] {
            let src = format!(
                "try {{ var s = ({a}) + 'tail'; print('ok', String(s).length) }} \
                 catch (e) {{ print('E', typeof e, String(e)) }}"
            );
            diff2(&format!("concat lim={lim} a={a}"), move |l| unsafe {
                let j = new_state(l, 0);
                l.js_setlimit(j, 0, lim);
                let cs = cstr(&src);
                let rc = l.js_dostring(j, cs.as_ptr());
                l.js_setlimit(j, 0, 0);
                l.js_freestate(j);
                format!("rc={rc}")
            });
        }
    }
    // rows 197 / 199 via the exported js_concat: repeated doubling eventually
    // trips the JS_STRLIMIT range check inside js_pushstring.
    probe_state("js_concat to JS_STRLIMIT", 0, || {
        job!(|l, j| {
            l.js_pushstring(j, cn!("0123456789abcdef"));
            let mut n = 0;
            loop {
                l.js_copy(j, -1);
                l.nullary("js_concat", j);
                n += 1;
                if n > 40 {
                    break;
                }
            }
            format!("rounds={n}")
        })
    });
    // and the numeric branch
    diff2("js_concat numeric", |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for (a, b) in [
            ("1", "2"),
            ("'a'", "1"),
            ("1", "'a'"),
            ("''", "''"),
            ("null", "null"),
            ("undefined", "1"),
            ("({})", "({})"),
            ("[]", "[]"),
            ("[1]", "[2]"),
            ("true", "false"),
            ("Object.create(null)", "1"),
        ] {
            let (a2, b2) = (a.to_string(), b.to_string());
            let t = probe(
                l,
                j,
                job!(|l, j| {
                    let ra = push_expr(l, j, &a2);
                    let rb = push_expr(l, j, &b2);
                    l.nullary("js_concat", j);
                    format!("a={ra} b={rb} v={}", from_c(l.js_tryrepr(j, -1, ERRSTR)))
                }),
            );
            r.push_str(&format!(" {a}+{b}={t}"));
        }
        l.js_freestate(j);
        r
    });
}

#[test]
fn t_compare_equal_strictequal() {
    const VALS: &[&str] = &[
        "undefined", "null", "true", "false", "0", "-0", "1", "NaN", "Infinity",
        "-Infinity", "''", "'0'", "'1'", "'a'", "'NaN'", "({})", "[]", "[0]", "[1]",
        "new Number(1)", "new String('1')", "new Boolean(false)", "Object.create(null)",
        "({valueOf:function(){return 1}})", "({toString:function(){return '1'}})",
    ];
    for (ai, _) in VALS.iter().enumerate() {
        for (bi, _) in VALS.iter().enumerate() {
            probe_state(&format!("cmp {} {}", VALS[ai], VALS[bi]), 0, move || {
                job!(|l, j| {
                    let ra = push_expr(l, j, VALS[ai]);
                    let rb = push_expr(l, j, VALS[bi]);
                    let mut okay: c_int = -1;
                    let c = l.js_compare(j, &mut okay);
                    let e = l.nullary_i("js_equal", j);
                    let s = l.nullary_i("js_strictequal", j);
                    format!("a={ra} b={rb} cmp={c} okay={okay} eq={e} seq={s}")
                })
            });
            let src = format!(
                "try {{ print(({a})<({b}), ({a})>({b}), ({a})<=({b}), ({a})>=({b}), \
                 ({a})==({b}), ({a})!=({b}), ({a})===({b}), ({a})!==({b})) }} \
                 catch (e) {{ print('E', e) }}",
                a = VALS[ai],
                b = VALS[bi]
            );
            diff_dostring(0, &src);
        }
    }
}

/* =========================================================================
 *  Rows 205-220: jsproperty.c.
 * ========================================================================= */

#[test]
fn t_property_tree_and_extensibility() {
    // rows 205 / 207 / 208: lookup and prototype-chain misses
    // row 206: inserting an existing name reuses the node
    diff2("property tree reuse", |l| unsafe {
        let j = new_state(l, 0);
        let r = probe(
            l,
            j,
            job!(|l, j| {
                l.js_newobject(j);
                let mut out = String::new();
                let names: Vec<String> = (0..40).map(|i| format!("k{i:02}")).collect();
                for n in &names {
                    let cs = cstr(n);
                    l.js_pushnumber(j, 1.0);
                    l.js_setproperty(j, -2, cs.as_ptr());
                }
                out.push_str(&format!("first={}", own_names(l, j, -1)));
                // re-insert every name; the set must not change
                for n in &names {
                    let cs = cstr(n);
                    l.js_pushnumber(j, 2.0);
                    l.js_setproperty(j, -2, cs.as_ptr());
                }
                out.push_str(&format!(" again={}", own_names(l, j, -1)));
                for n in ["", "k", "k000", "zzz", "k99"] {
                    let cs = cstr(n);
                    out.push_str(&format!(
                        " miss({n})={}",
                        l.js_hasproperty(j, -1, cs.as_ptr())
                    ));
                }
                // deletion order stresses unlinkproperty/skew/split
                for n in names.iter().rev() {
                    let cs = cstr(n);
                    l.js_delproperty(j, -1, cs.as_ptr());
                }
                out.push_str(&format!(" after_del={}", own_names(l, j, -1)));
                out
            }),
        );
        l.js_gc(j, 0);
        l.js_freestate(j);
        r
    });
    // the exported low-level tree entry points
    diff2("jsV_getproperty family", |l| unsafe {
        let j = new_state(l, 0);
        let r = probe(
            l,
            j,
            job!(|l, j| {
                let rc = push_expr(
                    l,
                    j,
                    "(function(){ var p={pp:1}; Object.defineProperty(p,'hid',\
                     {value:2,enumerable:false}); var o=Object.create(p); o.own=3; \
                     Object.defineProperty(o,'ohid',{value:4,enumerable:false}); return o })()",
                );
                let to: unsafe extern "C" fn(JS, c_int) -> *mut c_void = l.raw2("js_toobject");
                let gp: unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> *mut c_void =
                    l.raw2("jsV_getproperty");
                let gop: unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> *mut c_void =
                    l.raw2("jsV_getownproperty");
                let gpx: unsafe extern "C" fn(
                    JS,
                    *mut c_void,
                    *const c_char,
                    *mut c_int,
                ) -> *mut c_void = l.raw2("jsV_getpropertyx");
                let o = to(j, -1);
                let mut out = format!("push={rc}");
                for n in ["own", "ohid", "pp", "hid", "nope", "", "toString"] {
                    let cs = cstr(n);
                    let mut own: c_int = -1;
                    let a = gp(j, o, cs.as_ptr());
                    let b = gop(j, o, cs.as_ptr());
                    let c = gpx(j, o, cs.as_ptr(), &mut own);
                    out.push_str(&format!(
                        " {n}: get={} own={} x={}/own={own}",
                        a.is_null(),
                        b.is_null(),
                        c.is_null()
                    ));
                }
                out
            }),
        );
        l.js_freestate(j);
        r
    });
    // rows 209 / 210 / 213 / 214: DONTENUM and shadowing in the iterators
    for src in [
        "var p={a:1,b:2}; var o=Object.create(p); o.b=3; o.c=4; \
         var k=[]; for (var n in o) k.push(n); print(k.sort().join('|'))",
        "var p={a:1}; Object.defineProperty(p,'h',{value:1,enumerable:false}); \
         var o=Object.create(p); var k=[]; for (var n in o) k.push(n); print(k.join('|'))",
        "var o={}; Object.defineProperty(o,'h',{value:1,enumerable:false}); o.v=2; \
         var k=[]; for (var n in o) k.push(n); \
         print(k.join('|'), Object.keys(o).join('|'), \
         Object.getOwnPropertyNames(o).sort().join('|'))",
        "var p={}; Object.defineProperty(p,'x',{value:1,enumerable:false}); \
         var o=Object.create(p); o.x=2; var k=[]; for (var n in o) k.push(n); print(k.join('|'))",
        "var p={x:1}; var o=Object.create(p); \
         Object.defineProperty(o,'x',{value:2,enumerable:false}); \
         var k=[]; for (var n in o) k.push(n); print(k.join('|'))",
        "var k=[]; for (var n in Math) k.push(n); print(k.length)",
        "var k=[]; for (var n in 'abc') k.push(n); print(k.join('|'))",
        "var k=[]; for (var n in [1,2,3]) k.push(n); print(k.join('|'))",
        "var a=[1,2,3]; a.x=1; var k=[]; for (var n in a) k.push(n); print(k.join('|'))",
    ] {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
    }
    // rows 211 / 212: non-extensible objects, strict and sloppy
    for flags in [0, JS_STRICT] {
        for src in [
            "var o={}; Object.preventExtensions(o); o.n=1; print(o.n, Object.isExtensible(o))",
            "var o={a:1}; Object.preventExtensions(o); o.a=2; print(o.a)",
            "var o={a:1}; Object.seal(o); o.a=2; o.b=3; print(o.a, o.b)",
            "var o={a:1}; Object.freeze(o); o.a=2; o.b=3; print(o.a, o.b)",
            "var o={}; Object.preventExtensions(o); print(delete o.nope)",
            "var p={}; Object.preventExtensions(p); var o=Object.create(p); o.n=1; print(o.n)",
            "var a=[1,2]; Object.preventExtensions(a); a[5]=5; print(a.length, a[5])",
            "var a=[1,2]; Object.preventExtensions(a); a[2]=3; print(a.length, a[2])",
            "var a=[1,2]; Object.freeze(a); a[0]=9; print(a[0], a.length)",
        ] {
            let w = format!("try {{ {src} }} catch (e) {{ print('E', e) }}");
            diff_dostring(flags, &w);
        }
        // and the exported jsV_setproperty on a non-extensible object
        probe_state(&format!("jsV_setproperty nonext flags={flags}"), flags, || {
            job!(|l, j| {
                let rc = push_expr(
                    l,
                    j,
                    "(function(){var o={a:1}; Object.preventExtensions(o); return o})()",
                );
                let to: unsafe extern "C" fn(JS, c_int) -> *mut c_void = l.raw2("js_toobject");
                let sp: unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> *mut c_void =
                    l.raw2("jsV_setproperty");
                let o = to(j, -1);
                let a = sp(j, o, cn!("a"));
                let b = sp(j, o, cn!("brand-new"));
                format!("push={rc} a_null={} b_null={}", a.is_null(), b.is_null())
            })
        });
    }
}

#[test]
fn t_iterators() {
    // row 215: js_nextiterator on something that is not an iterator
    for (si, _) in SHAPES.iter().enumerate() {
        probe_state(&format!("nextiterator on {}", SHAPES[si]), 0, move || {
            job!(|l, j| {
                let rc = push_expr(l, j, SHAPES[si]);
                let p = l.js_nextiterator(j, -1);
                format!("push={rc} p={:?}", from_c(p))
            })
        });
    }
    // rows 216 / 217: names deleted mid-iteration are skipped; exhaustion -> NULL
    diff2("iterator mid-delete", |l| unsafe {
        let j = new_state(l, 0);
        let r = probe(
            l,
            j,
            job!(|l, j| {
                l.js_newobject(j);
                for n in ["a", "b", "c", "d", "e"] {
                    let cs = cstr(n);
                    l.js_pushnumber(j, 1.0);
                    l.js_setproperty(j, -2, cs.as_ptr());
                }
                l.js_pushiterator(j, -1, 1);
                let mut seen: Vec<String> = vec![];
                loop {
                    let p = l.js_nextiterator(j, -1);
                    if p.is_null() {
                        break;
                    }
                    seen.push(from_c(p));
                    // delete later names so jsV_getproperty misses them
                    for n in ["c", "d"] {
                        let cs = cstr(n);
                        l.js_delproperty(j, -2, cs.as_ptr());
                    }
                    if seen.len() > 20 {
                        break;
                    }
                }
                let a = l.js_nextiterator(j, -1);
                let b = l.js_nextiterator(j, -1);
                format!(
                    "seen={} exhausted={} {}",
                    seen.join(","),
                    a.is_null(),
                    b.is_null()
                )
            }),
        );
        l.js_freestate(j);
        r
    });
    // js_pushiterator's `own` flag takes any int (C accepts any int for a bool)
    for own in [0i32, 1, 2, -1, 255, i32::MIN, i32::MAX] {
        for s in ["({a:1})", "[1,2]", "new String('ab')", "Object.create({p:1})", "Math"] {
            probe_state(&format!("pushiterator own={own} {s}"), 0, move || {
                job!(|l, j| {
                    let rc = push_expr(l, j, s);
                    l.js_pushiterator(j, -1, own);
                    let mut v: Vec<String> = vec![];
                    loop {
                        let p = l.js_nextiterator(j, -1);
                        if p.is_null() {
                            break;
                        }
                        v.push(from_c(p));
                        if v.len() > 80 {
                            v.push("...".to_string());
                            break;
                        }
                    }
                    format!("push={rc} names=[{}]", v.join(","))
                })
            });
        }
    }
    // for..in over the array index range (io->u.iter.n)
    for src in [
        "var a=[1,2,3]; delete a[1]; var k=[]; for (var n in a) k.push(n); print(k.join('|'))",
        "var a=[1,2,3]; a.length=1; var k=[]; for (var n in a) k.push(n); print(k.join('|'))",
        "var a=[]; a[3]=1; var k=[]; for (var n in a) k.push(n); print(k.join('|'), a.length)",
        "var s=new String(''); var k=[]; for (var n in s) k.push(n); print(k.length)",
    ] {
        diff_dostring(0, src);
    }
}

#[test]
fn t_resizearray() {
    // rows 219 / 220: growing / unchanged newlen keeps everything, and keys that
    // are not the canonical decimal form of their number are preserved.
    for pre in [
        "a['01']=1;",
        "a['1e2']=1;",
        "a['x']=1;",
        "a[' 1']=1;",
        "a['-1']=1;",
        "a['0']=1;",
        "a['10']=1;",
        "a['1.0']=1;",
        "a['4294967296']=1;",
        "",
    ] {
        for newlen in ["0", "1", "3", "20", "100"] {
            for mk in [
                "var a=[1,2,3]; a[50]=50;",
                "var a=[]; for (var i=0;i<8;++i) a[i]=i; a[100]=100;",
            ] {
                let src = format!(
                    "{mk} {pre} a.length = {newlen}; \
                     print(a.length, Object.getOwnPropertyNames(a).sort().join('|'))"
                );
                diff_dostring(0, &src);
                diff_dostring(JS_STRICT, &src);
            }
        }
    }
    // and directly through the exported jsV_resizearray on a NON-simple array
    // (the assert at jsproperty.c:325 requires !simple; see the header note)
    for newlen in [-1i32, 0, 1, 2, 3, 100, i32::MAX, i32::MIN] {
        probe_state(&format!("jsV_resizearray {newlen}"), 0, move || {
            job!(|l, j| {
                let rc = push_expr(
                    l,
                    j,
                    "(function(){ var a=[1,2,3]; a[50]=50; a['01']=1; a['x']=2; return a })()",
                );
                let to: unsafe extern "C" fn(JS, c_int) -> *mut c_void = l.raw2("js_toobject");
                let rz: unsafe extern "C" fn(JS, *mut c_void, c_int) = l.raw2("jsV_resizearray");
                let o = to(j, -1);
                rz(j, o, newlen);
                format!(
                    "push={rc} len={} names={}",
                    l.js_getlength(j, -1),
                    own_names(l, j, -1)
                )
            })
        });
    }
}

/* =========================================================================
 *  Rows 221-230: jsintern.c.
 * ========================================================================= */

#[test]
fn t_intern() {
    // rows 225 / 230: the first intern initialises the tree; an already interned
    // string comes back as the SAME pointer.
    diff2("js_intern identity", |l| unsafe {
        let j = new_state(l, 0);
        let names: Vec<String> = {
            let mut v: Vec<String> = vec![
                String::new(),
                "a".to_string(),
                "b".to_string(),
                "aa".to_string(),
                "ab".to_string(),
                "z".to_string(),
                "\u{7f}".to_string(),
                "m".repeat(300),
            ];
            let mut rng = Rng::new(0x2250_2300);
            for _ in 0..80 {
                v.push(rng.ascii_string(8));
            }
            v
        };
        let mut first: Vec<usize> = vec![];
        let mut r = String::new();
        for n in &names {
            let cs = cstr(n);
            let p = l.js_intern(j, cs.as_ptr());
            first.push(p as usize);
            r.push_str(&format!("{:?}=>{:?} ", n, from_c(p)));
        }
        let mut same = 0;
        for (i, n) in names.iter().enumerate() {
            let cs = cstr(n);
            let p = l.js_intern(j, cs.as_ptr());
            if p as usize == first[i] {
                same += 1;
            }
        }
        r.push_str(&format!("\nsame={same}/{}", names.len()));
        l.js_freestate(j);
        r
    });
    // row 224: the string-node allocation fails
    for lim in [1i32, 8, 16, 32, 64, 256, 1024] {
        probe_state(&format!("intern oom lim={lim}"), 0, move || {
            job!(|l, j| {
                l.js_setlimit(j, 0, lim);
                let cs = cstr("a string to intern");
                let p = l.js_intern(j, cs.as_ptr());
                let r = format!("interned={:?}", from_c(p));
                l.js_setlimit(j, 0, 0);
                r
            })
        });
    }
    // rows 221 / 222: js_putc's first allocation and its doubling realloc
    for lim in [1i32, 2, 8, 16, 17, 24, 32, 33, 48, 64, 96, 128, 256, 1024, 8192] {
        for n in [1usize, 8, 16, 17, 32, 33, 64, 100] {
            probe_state(&format!("js_putc lim={lim} n={n}"), 0, move || {
                job!(|l, j| {
                    let putc: unsafe extern "C" fn(JS, *mut *mut c_void, c_int) =
                        l.raw2("js_putc");
                    let puts: unsafe extern "C" fn(JS, *mut *mut c_void, *const c_char) =
                        l.raw2("js_puts");
                    let mut sb: *mut c_void = std::ptr::null_mut();
                    l.js_setlimit(j, 0, lim);
                    // The breadcrumb says WHICH putc failed: index 0 is the
                    // first js_malloc (row 221), index 64 is the doubling
                    // js_realloc (row 222, `sb->n == sb->m == 64`).
                    for i in 0..n {
                        mark_set(&format!("putc{i}"));
                        putc(j, &mut sb, (b'a' + (i % 26) as u8) as c_int);
                    }
                    mark_set("puts");
                    puts(j, &mut sb, cn!("tail"));
                    let r = format!("buffered n={n} null={}", sb.is_null());
                    l.js_setlimit(j, 0, 0);
                    if !sb.is_null() {
                        l.js_free(j, sb);
                    }
                    r
                })
            });
        }
    }
}

/// Row 223: `jsS_newstringnode` with `strlen(string) > JS_STRLIMIT`.
#[test]
fn t_intern_string_limit() {
    let big: Vec<u8> = {
        let mut v = vec![b'i'; (JS_STRLIMIT + 1) as usize];
        v.push(0);
        v
    };
    let bp = big.as_ptr() as usize;
    probe_state("intern strlen>JS_STRLIMIT", 0, move || {
        job!(|l, j| {
            let p = l.js_intern(j, bp as *const c_char);
            format!("interned null={}", p.is_null())
        })
    });
    drop(big);
}

/// Rows 226-229: `jsS_dumpstrings` / `jsS_freestrings` sentinel handling.  A
/// FRESH state has NOT interned anything (`jsB_init` only uses `js_pushliteral`
/// with static C literals), so `J->strings == NULL` there -- that is row 227 and
/// row 229 exactly.
#[test]
fn t_dumpstrings_sentinel() {
    let p = libs();
    let mut outs: Vec<String> = vec![];
    for l in [&p.c, &p.rs] {
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let path = format!("{dir}/errcore_dumpstr_{}.txt", l.name);
        let text = capture_stdout(&path, || unsafe {
            set_cur(l);
            // (a) brand new state: nothing interned
            let j = l.js_newstate(None, std::ptr::null_mut(), 0);
            l.jsS_dumpstrings(j);
            l.js_freestate(j); // row 229: nothing to free
            // (b) exactly one interned string: the root is a leaf whose children
            //     are both the sentinel (rows 226 / 228)
            let j = l.js_newstate(None, std::ptr::null_mut(), 0);
            l.js_intern(j, cn!("only"));
            l.jsS_dumpstrings(j);
            l.js_freestate(j);
            // (c) a real tree
            let j = l.js_newstate(None, std::ptr::null_mut(), 0);
            for k in 0..24 {
                let cs = CString::new(format!("name{k:02}")).unwrap();
                l.js_intern(j, cs.as_ptr());
            }
            l.jsS_dumpstrings(j);
            l.js_freestate(j);
        });
        outs.push(text);
    }
    // libtest writes its own `test <name> ... ok` progress lines straight to
    // fd 1 from OTHER threads while our redirect is in place, so keep only the
    // lines `jsS_dumpstrings` itself can emit: the block delimiters and the
    // `<level>: <tabs>'<string>'` node lines (jsintern.c:100-103).
    let mine = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| {
                *l == "interned strings {"
                    || *l == "}"
                    || (l.split_once(": ").is_some_and(|(a, _)| {
                        !a.is_empty() && a.chars().all(|c| c.is_ascii_digit())
                    }))
            })
            .map(|l| l.to_string())
            .collect()
    };
    let a = mine(&outs[0]);
    let b = mine(&outs[1]);
    assert_eq!(a, b, "jsS_dumpstrings divergence");
    assert!(
        a.contains(&"interned strings {".to_string()),
        "no dump produced: {:?}",
        outs[0]
    );
    // the FIRST of the three dumps -- the fresh state -- must be the EMPTY
    // block (row 227: J->strings is still NULL because jsB_init interns nothing)
    assert_eq!(
        &a[0..2],
        &["interned strings {".to_string(), "}".to_string()],
        "a fresh state should have no interned strings: {:?}",
        outs[0]
    );
    // and the third dump must be a real tree
    assert!(
        a.len() > 24,
        "expected a populated tree in the third dump: {a:?}"
    );
}

/// Serialises EVERY writer of the process-wide stdout in this test binary.
///
/// Two kinds of test here write to fd 1 from inside the libraries:
///   * `jsS_dumpstrings` (jsintern.c:100-107), captured by
///     `t_dumpstrings_sentinel` via a `dup2` on fd 1, and
///   * `js_gc(J, 1)` (jsgc.c:255), whose report is the subject of the
///     `t_gc_*` tests.
///
/// `jsS_dumpstrings` builds each node line with THREE separate stdio calls
/// (`printf("%d: ")`, `putchar('\t')` per level, `printf("'%s'\n")`), so a
/// concurrent `js_gc(J, 1)` report can be spliced into the MIDDLE of a line.
/// libtest runs tests as parallel threads of one process, so both must hold
/// this lock for their whole duration.
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn stdout_guard() -> std::sync::MutexGuard<'static, ()> {
    STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn capture_stdout<F: FnOnce()>(path: &str, f: F) -> String {
    let _guard = stdout_guard();
    use std::os::unix::io::AsRawFd;
    extern "C" {
        fn dup(fd: c_int) -> c_int;
        fn dup2(old: c_int, new: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
        fn fflush(f: *mut c_void) -> c_int;
    }
    let file = std::fs::File::create(path).expect("create capture file");
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        dup2(file.as_raw_fd(), 1);
        f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);
    std::fs::read_to_string(path).unwrap_or_default()
}

/* =========================================================================
 *  Rows 231-258: jsgc.c.
 * ========================================================================= */

#[repr(C)]
#[derive(Default)]
struct GcCnt {
    nalloc: u64,
    nrealloc: u64,
    nfree: u64,
    live: i64,
}

unsafe extern "C" fn gc_alloc(actx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    extern "C" {
        fn free(p: *mut c_void);
        fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    }
    let cx = &mut *(actx as *mut GcCnt);
    if size == 0 {
        if !ptr.is_null() {
            cx.nfree += 1;
            cx.live -= 1;
        }
        free(ptr);
        return std::ptr::null_mut();
    }
    let p = realloc(ptr, size as usize);
    if !p.is_null() {
        if ptr.is_null() {
            cx.nalloc += 1;
            cx.live += 1;
        } else {
            cx.nrealloc += 1;
        }
    }
    p
}

/// Rows 231-237 (free paths) and 238-255 (mark + sweep).  The `js_gc(J, 1)`
/// report line names the exact number of envs / funs / objs / props / strs seen
/// and freed, and the tracking allocator pins down every `js_free` the free
/// paths do or do not perform (row 234's inline short string and row 235's
/// already-unflattened array are precisely the two skipped frees).
#[test]
fn t_gc_free_and_sweep() {
    // js_gc(J, 1) prints to stdout; see STDOUT_LOCK.
    let _stdout = stdout_guard();
    let setups: &[&str] = &[
        "0",
        // row 234: short (inline) vs long (heap) CSTRING payloads
        "var a=[]; for (var i=0;i<40;++i) a.push(new String('short')); 0",
        "var a=[]; for (var i=0;i<40;++i) a.push(new String('a string well over \
         fifteen bytes long indeed')); 0",
        "for (var i=0;i<40;++i) { var s = new String('tmp'+i) } 0",
        "for (var i=0;i<40;++i) { var s = new String('a long temporary string '+i) } 0",
        // row 235: simple vs unflattened arrays
        "var a=[]; for (var i=0;i<40;++i) a.push([1,2,3]); 0",
        "var a=[]; for (var i=0;i<40;++i) { var b=[1,2,3]; b[9]=9; a.push(b) } 0",
        "for (var i=0;i<40;++i) { var b=[1,2,3]; b[9]=9 } 0",
        // rows 231 / 233 / 245: sentinel-only vs populated property trees
        "var a=[]; for (var i=0;i<40;++i) a.push({}); 0",
        "var a=[]; for (var i=0;i<40;++i) { var o={}; for (var k=0;k<10;++k) \
         o['p'+k]=k; a.push(o) } 0",
        "for (var i=0;i<40;++i) { var o={}; for (var k=0;k<10;++k) o['p'+k]=k } 0",
        // row 232: iterators, with and without names
        "for (var i=0;i<20;++i) { for (var k in {}) {} } 0",
        "for (var i=0;i<20;++i) { for (var k in {a:1,b:2,c:3}) {} } 0",
        // rows 238-240 / 248 / 249: functions, closures and scope chains
        "var f=[]; for (var i=0;i<30;++i) f.push(function(){ return function(){ return i } }); 0",
        "for (var i=0;i<30;++i) { (function(){ var x=i; return function(){return x} })() } 0",
        "function outer(){ function a(){ function b(){ return 1 } return b } return a } \
         for (var i=0;i<20;++i) outer()(); 0",
        "var g; function mk(){ var v=1; g = function(){ return v }; return g } \
         for (var i=0;i<20;++i) mk(); 0",
        // rows 241-244: memstr / object / getter / setter properties
        "var o={}; for (var i=0;i<30;++i) o['k'+i] = 'a value long enough to be a memstr '+i; 0",
        "var o={}; for (var i=0;i<20;++i) Object.defineProperty(o,'g'+i, \
         {get:function(){return 1}, set:function(v){}, configurable:true}); 0",
        // rows 246 / 247: prototype chains and iterator targets
        "var p={a:1}; var a=[]; for (var i=0;i<30;++i) a.push(Object.create(p)); 0",
        "var a=[]; for (var i=0;i<20;++i) a.push(Object.create(null)); 0",
        // row 254: unreachable js_Strings
        "for (var i=0;i<200;++i) { var s = 'a fairly long string value '+i } 0",
        // regexps carry a Reprog and a strdup'd source
        "var a=[]; for (var i=0;i<20;++i) a.push(new RegExp('a'+i+'b*','gi')); 0",
        "for (var i=0;i<20;++i) { var r = new RegExp('x'+i,'m') } 0",
        // dates, errors, arguments objects
        "var a=[]; for (var i=0;i<20;++i) a.push(new Date(i)); 0",
        "var a=[]; for (var i=0;i<20;++i) a.push(new Error('e'+i)); 0",
        "function f(){ return arguments } var a=[]; for (var i=0;i<20;++i) a.push(f(1,2,3)); 0",
    ];
    for src in setups {
        diff2(&format!("gc {src}"), move |l| unsafe {
            let mut cx = GcCnt::default();
            set_cur(l);
            let j = l.js_newstate(Some(gc_alloc), &mut cx as *mut GcCnt as *mut c_void, 0);
            assert!(!j.is_null());
            l.js_setreport(j, Some(report_cb));
            l.js_newcfunction(j, Some(print_cb), PRINT, 1);
            l.js_setglobal(j, PRINT);
            let cs = cstr(src);
            let rc = l.js_dostring(j, cs.as_ptr());
            let a0 = (cx.nalloc, cx.nfree, cx.nrealloc, cx.live);
            // rows 250-255: two reporting collections; the second must be stable
            l.js_gc(j, 1);
            let a1 = (cx.nalloc, cx.nfree, cx.nrealloc, cx.live);
            l.js_gc(j, 1);
            let a2 = (cx.nalloc, cx.nfree, cx.nrealloc, cx.live);
            // drop every global the setups may have created and collect again
            let cs2 = cstr(
                "a = undefined; f = undefined; o = undefined; g = undefined; \
                 s = undefined; r = undefined; b = undefined; p = undefined; 0",
            );
            let rc2 = l.js_dostring(j, cs2.as_ptr());
            l.js_gc(j, 1);
            let a3 = (cx.nalloc, cx.nfree, cx.nrealloc, cx.live);
            l.js_freestate(j);
            format!(
                "rc={rc} rc2={rc2} a0={a0:?} a1={a1:?} a2={a2:?} a3={a3:?} final_live={}",
                cx.live
            )
        });
    }
}

/// Row 253 specifically: an unreachable object with a HOST FINALIZER is freed by
/// the sweep, which runs the finalizer.  Rows 236 / 237: a NULL finalizer is
/// simply not called.
#[test]
fn t_gc_runs_host_finalizers() {
    // js_gc(J, 1) prints to stdout; see STDOUT_LOCK.
    let _stdout = stdout_guard();
    for withfin in [false, true] {
        for keep in [false, true] {
            diff2(
                &format!("gc finalizer fin={withfin} keep={keep}"),
                move |l| unsafe {
                    let _ = hooks_take();
                    let j = new_state(l, 0);
                    let fin: js_Finalize = if withfin { Some(ud_fin) } else { None };
                    let r = probe(
                        l,
                        j,
                        job!(|l, j| {
                            for _ in 0..5 {
                                l.js_newobject(j);
                                l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, fin);
                                l.js_newcfunctionx(
                                    j,
                                    Some(cf_nopush),
                                    cn!("cfx"),
                                    0,
                                    PAYLOAD as *mut c_void,
                                    fin,
                                );
                                if keep {
                                    l.js_setglobal(j, cn!("keptf"));
                                    l.js_setglobal(j, cn!("keptu"));
                                } else {
                                    l.js_pop(j, 2);
                                }
                            }
                            format!("built keep={keep}")
                        }),
                    );
                    let built = hooks_take();
                    l.js_gc(j, 1);
                    let after_gc = hooks_take();
                    l.js_gc(j, 1);
                    let after_gc2 = hooks_take();
                    l.js_freestate(j);
                    let after_free = hooks_take();
                    format!("{r} built={built} gc={after_gc} gc2={after_gc2} free={after_free}")
                },
            );
        }
    }
}

/// Rows 250-255: the report text itself, and the fact that `ntot` is never 0
/// (which is why row 256's division by zero is unreachable from `js_newstate`).
#[test]
fn t_gc_sweep_report() {
    // js_gc(J, 1) prints to stdout; see STDOUT_LOCK.
    let _stdout = stdout_guard();
    for flags in [0, JS_STRICT] {
        for reps in [0i32, 1, 2, -1, 255, i32::MIN, i32::MAX] {
            diff2(&format!("gc report={reps} flags={flags}"), move |l| unsafe {
                let j = new_state(l, flags);
                let cs = cstr(
                    "var a=[]; for (var i=0;i<200;++i) a.push({k:'v'+i}); \
                     for (var i=0;i<200;++i) { var t={} } 0",
                );
                let rc = l.js_dostring(j, cs.as_ptr());
                out_clear();
                l.js_gc(j, reps);
                let first = out_take();
                l.js_gc(j, reps);
                let second = out_take();
                l.js_freestate(j);
                format!("rc={rc} first={first:?} second={second:?}")
            });
        }
    }
    // ntot > 0 for a fresh state, so row 256 (100*gtot/ntot with ntot == 0)
    // cannot be reached from any state js_newstate can hand out.
    let p = libs();
    for l in [&p.c, &p.rs] {
        unsafe {
            out_clear();
            let j = new_state(l, 0);
            l.js_gc(j, 1);
            let txt = out_take();
            l.js_freestate(j);
            assert!(
                txt.starts_with("[report] garbage collected"),
                "{}: unexpected gc report {txt:?}",
                l.name
            );
            let nums: Vec<i64> = txt
                .split(|c: char| !c.is_ascii_digit())
                .filter(|s| !s.is_empty())
                .map(|s| s.parse().unwrap_or(0))
                .collect();
            assert!(
                nums.iter().sum::<i64>() > 0,
                "{}: ntot == 0 in {txt:?}",
                l.name
            );
        }
    }
}

/* =========================================================================
 *  Generic FFI-boundary abuse for the entry points above.
 * ========================================================================= */

/// Out-of-range "enum" values, oversized / negative lengths, and one-past-the-
/// documented-range indices.  NULL `const char *` arguments are only used where
/// the C provably does not dereference them; the rest are C UB and named here:
///
///  * `js_pushstring(J, NULL)` / `js_pushliteral(J, NULL)` -- `strlen(NULL)` at
///    jsrun.c:147 (and `jsV_tostring` later for the literal).
///  * `js_setproperty(J, i, NULL)` and every other `const char *name` -- the
///    first thing `jsR_*property` does is `strcmp(name, ...)` (jsrun.c:574).
///  * `js_intern(J, NULL)` -- `strlen` at jsintern.c:45.
///  * `js_ploadstring(J, NULL, src)` -- `js_intern(J, J->filename)` at
///    jscompile.c:59.
///  * `js_ploadstring(J, file, NULL)` / `js_dostring(J, NULL)` -- the lexer
///    dereferences `source` immediately in `jsY_initlex`.
///  * `js_pushlstring(J, p, n)` with `n < 0` -- jsrun.c:170's
///    `while (n--) *s++ = *v++` runs ~2^32 times into a 16 byte buffer.
///  * `js_newuserdata(J, NULL, ...)` then `js_isuserdata` -- the tag is
///    `strcmp`d at jsrun.c:270.
#[test]
fn t_ffi_boundary_abuse() {
    let mut rng = Rng::new(0xFF10_ABCD);

    // js_defproperty / js_defaccessor attribute bitmasks: the C does
    // `ref->atts |= atts` with no validation, so every int is legal.
    let mut atts: Vec<c_int> = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 64, 255, 256, -1, -2, -8, i32::MIN,
        i32::MAX, 0x1000_0000,
    ];
    for _ in 0..20 {
        atts.push(rng.next_u32() as c_int);
    }
    for a in atts {
        probe_state(&format!("atts={a:#x}"), 0, move || {
            job!(|l, j| {
                l.js_newobject(j);
                l.js_pushnumber(j, 1.0);
                l.js_defproperty(j, -2, cn!("p"), a);
                let mut r = format!("names={}", own_names(l, j, -1));
                l.js_pushnumber(j, 2.0);
                l.js_setproperty(j, -2, cn!("p"));
                l.js_getproperty(j, -1, cn!("p"));
                r.push_str(&format!(" after_set={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                l.js_pop(j, 1);
                l.js_delproperty(j, -1, cn!("p"));
                let h = l.js_hasproperty(j, -1, cn!("p"));
                r.push_str(&format!(" after_del={h}"));
                if h != 0 {
                    l.js_pop(j, 1);
                }
                l.js_newcfunction(j, Some(cf_ret), cn!("g"), 0);
                l.js_pushnull(j);
                l.js_defaccessor(j, -3, cn!("acc"), a);
                l.js_getproperty(j, -1, cn!("acc"));
                r.push_str(&format!(" acc={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                l.js_pop(j, 1);
                r.push_str(&format!(" names2={}", own_names(l, j, -1)));
                r
            })
        });
    }

}

#[test]
fn t_ffi_regexp_flags() {
    let mut rng = Rng::new(0xFF10_ABCD);
    // js_newregexp flags: only bits 0..2 are defined (JS_REGEXP_G/I/M)
    let mut rf: Vec<c_int> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 15, 255, 0xFFFF, -1, i32::MIN];
    for _ in 0..10 {
        rf.push(rng.next_u32() as c_int);
    }
    for f in rf {
        probe_state(&format!("newregexp flags={f:#x}"), 0, move || {
            job!(|l, j| {
                l.js_newregexp(j, cn!("a(b)c"), f);
                let mut r = format!("ty={}", from_c(l.js_typeof(j, -1)));
                for k in ["source", "global", "ignoreCase", "multiline", "lastIndex"] {
                    let cs = cstr(k);
                    let h = l.js_hasproperty(j, -1, cs.as_ptr());
                    if h != 0 {
                        r.push_str(&format!(" {k}={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                        l.js_pop(j, 1);
                    }
                }
                r
            })
        });
    }

}

#[test]
fn t_ffi_predicate_index() {
    let mut _rng = Rng::new(0xFF10_ABCD);
    // js_type / js_typeof / every predicate with indices one step past the range
    diff2("predicate index sweep", |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for i in [-3i32, -2, -1, 0, 1, 2, 3, 4, i32::MAX, i32::MIN, 4096, -4096] {
            l.js_pushnumber(j, 1.0);
            l.js_pushstring(j, cn!("s"));
            r.push_str(&format!(
                "{i}: ty={} t={} {} {} {} {} {} {} b={}\n",
                from_c(l.js_typeof(j, i)),
                l.js_type(j, i),
                l.pred("js_isdefined", j, i),
                l.pred("js_isundefined", j, i),
                l.pred("js_isnull", j, i),
                l.pred("js_isobject", j, i),
                l.pred("js_iscallable", j, i),
                l.pred("js_iscoercible", j, i),
                l.js_toboolean(j, i),
            ));
            l.js_pop(j, 2);
        }
        l.js_freestate(j);
        r
    });

}

#[test]
fn t_ffi_index_abuse() {
    let mut _rng = Rng::new(0xFF10_ABCD);
    // js_getlength / js_setlength / js_getindex / js_setindex / js_delindex with
    // indices one step past the documented range.
    for i in [
        -1i32,
        0,
        1,
        2,
        3,
        (JS_ARRAYLIMIT - 1) as c_int,
        JS_ARRAYLIMIT as c_int,
        (JS_ARRAYLIMIT + 1) as c_int,
        i32::MAX,
        i32::MIN,
    ] {
        probe_state(&format!("index abuse {i}"), 0, move || {
            job!(|l, j| {
                l.js_newarray(j);
                l.js_pushnumber(j, 1.0);
                l.js_setindex(j, -2, 0);
                let mut r = String::new();
                let h = l.js_hasindex(j, -1, i);
                if h != 0 {
                    r.push_str(&format!("has={h} v={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                    l.js_pop(j, 1);
                } else {
                    r.push_str(&format!("has={h} "));
                }
                l.js_getindex(j, -1, i);
                r.push_str(&format!("get={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                l.js_pop(j, 1);
                l.js_pushnumber(j, 9.0);
                l.js_setindex(j, -2, i);
                r.push_str(&format!("len={} ", l.js_getlength(j, -1)));
                l.js_delindex(j, -1, i);
                r.push_str(&format!(
                    "dlen={} names={}",
                    l.js_getlength(j, -1),
                    own_names(l, j, -1)
                ));
                r
            })
        });
    }

}

#[test]
fn t_ffi_try_defaults() {
    let mut _rng = Rng::new(0xFF10_ABCD);
    // js_trystring with a NULL default: the C returns it verbatim (jsstate.c:52)
    probe_state("trystring NULL default", 0, || {
        job!(|l, j| {
            let rc = push_expr(l, j, "({toString:function(){throw 'x'}})");
            let s = l.js_trystring(j, -1, std::ptr::null());
            format!("push={rc} null={} s={:?}", s.is_null(), from_c(s))
        })
    });
    // js_tryrepr / js_trynumber / js_tryinteger / js_tryboolean defaults
    diff2("try* defaults", |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for s in [
            "({toString:function(){throw 'x'}})",
            "({valueOf:function(){throw 'y'}})",
            "1",
            "'s'",
            "Object.create(null)",
        ] {
            let base = l.js_gettop(j);
            let rc = push_expr(l, j, s);
            r.push_str(&format!(
                "{s}: rc={rc} n={} i={} b={} s={:?} rp={:?}\n",
                l.js_trynumber(j, -1, -1.5),
                l.js_tryinteger(j, -1, -7),
                l.js_tryboolean(j, -1, -3),
                from_c(l.js_trystring(j, -1, cn!("DS"))),
                from_c(l.js_tryrepr(j, -1, cn!("DR"))),
            ));
            drain_to(l, j, base);
        }
        l.js_freestate(j);
        r
    });

    // js_isuserdata with a NULL tag against a NON-userdata value: the C returns
    // 0 before touching the tag (jsrun.c:269-271).
    diff2("isuserdata NULL tag", |l| unsafe {
        let j = new_state(l, 0);
        l.js_pushnumber(j, 1.0);
        l.js_newobject(j);
        let r = format!(
            "num={} obj={}",
            l.js_isuserdata(j, -2, std::ptr::null()),
            l.js_isuserdata(j, -1, std::ptr::null())
        );
        l.js_pop(j, 2);
        l.js_freestate(j);
        r
    });

}

#[test]
fn t_ffi_setlimit() {
    let mut rng = Rng::new(0xFF10_ABCD);
    // js_setlimit with every extreme, then a script
    let mut pairs: Vec<(c_int, c_int)> = vec![
        (0, 0),
        (-1, -1),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        (1, 1),
        (1, i32::MAX),
        (i32::MAX, 1),
    ];
    for _ in 0..12 {
        pairs.push((rng.next_u32() as c_int, rng.next_u32() as c_int));
    }
    for (r0, m0) in pairs {
        diff2(&format!("setlimit({r0},{m0})"), move |l| unsafe {
            let j = new_state(l, 0);
            l.js_setlimit(j, r0, m0);
            let cs = cstr("var s=0; for (var i=0;i<80;++i) s+=i; print(s)");
            let rc = l.js_dostring(j, cs.as_ptr());
            l.js_setlimit(j, 0, 0);
            l.js_freestate(j);
            format!("rc={rc}")
        });
    }
}
