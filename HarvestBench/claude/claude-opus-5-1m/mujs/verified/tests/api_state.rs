//! Differential tests for the state / call / registry / GC surface of the
//! public API.  Covers CONFIGS.md rows 1-40 and 183-197.
//!
//! Everything goes through the two `.so` exports via `tests/common/mod.rs`.
//!
//! Rows that are deliberately *not* driven, with the reason:
//!
//! * `jsrun.c:1400` `js_pconstruct` computes `savetop = TOP - n - 2` while
//!   `js_construct` wants the callee at `-n-1`, i.e. `savetop` names the slot
//!   *below* the callee.  Calling `js_pconstruct` with the callee as the very
//!   first stack slot therefore makes the error path write `STACK[-1]`, an
//!   out-of-bounds write.  Every `js_pconstruct` test below keeps a sentinel
//!   value underneath the callee so that slot is in range.
//! * `jsgc.c:254` divides by `ntot` (`100*gtot/ntot`).  `ntot` is only 0 for a
//!   state with no environments, functions, objects, properties and strings at
//!   all, which `js_newstate` can never produce, so the division is safe for
//!   every state reachable through the public API and is exercised below.
//! * `jsrun.c:1465` `js_throw` with `trytop == 0` runs the panic hook and then
//!   `abort()`s.  That cannot be observed in-process, so it is driven in a
//!   forked copy of this test binary (`t_panic_default_vs_custom`).
//! * `jsrun.c:1446` `js_savetry` at `trytop == JS_TRYLIMIT` calls
//!   `js_trystackoverflow`, which throws into `trybuf[trytop-1]`.  The tests
//!   below never push a 65th frame with `js_savetry` directly (there is no
//!   `setjmp` behind those frames), they reach the limit and then use the
//!   *protected* entry points (`js_ploadstring`, `js_dostring`, `js_trystring`)
//!   whose `js_ptry` guard returns without throwing.  The `js_trystackoverflow`
//!   path itself is driven through nested JS `try` blocks and nested
//!   `js_pcall`s, where a real `setjmp` frame is always underneath.
//! * `jsrepr.c:275` `js_tryrepr` is the one `js_try*` helper *without* a
//!   `js_ptry` guard, so at `trytop == JS_TRYLIMIT` it throws instead of
//!   returning the caller's default.  The try-limit tests therefore inspect
//!   values with `safeview()` (`js_type` / `js_typeof` / `js_tostring` on a
//!   value already known to be a string), none of which can throw.
//! * `jsbuiltin.c:205` stores `js_regcompx(...)` into
//!   `RegExp_prototype->u.r.prog` without a NULL check, and `regexp.c:903`
//!   turns an allocation failure into a NULL return rather than a throw.  An
//!   allocator that fails during that one compile therefore yields a *usable*
//!   state with a NULL prog, so `t_newstate_alloc_fails` compares the C and
//!   Rust decision for every failing allocation index instead of insisting on
//!   a NULL state.

mod common;
use common::*;
use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::null_mut;

/* ------------------------------------------------------------------ libc */

extern "C" {
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(a: c_int, b: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
}

/* ----------------------------------------------------------- name literals */

macro_rules! cn {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

const N_GV: *const c_char = cn!("gv");
const N_X: *const c_char = cn!("x");
const N_UD: *const c_char = cn!("ud");
const N_TAG: *const c_char = cn!("mytag");
const N_INNER: *const c_char = cn!("inner");
const N_PROBE: *const c_char = cn!("probe");
const N_CTOR: *const c_char = cn!("Ctor");
const N_VOIDF: *const c_char = cn!("voidf");
const N_THREE: *const c_char = cn!("three");
const N_DATAF: *const c_char = cn!("dataf");
const N_REC: *const c_char = cn!("rec");
const PAYLOAD: *const c_char = cn!("payload-A");

/* ------------------------------------------------------- tracking allocator */

const ACTX_MAGIC: u64 = 0x5a5a_1234_dead_beef;

#[repr(C)]
#[derive(Default)]
struct Actx {
    magic: u64,
    live: i64,
    nalloc: u64,
    nrealloc: u64,
    nfree: u64,
    ncalls: u64,
    fail_at: u64,
    failed: u64,
    bad_actx: u64,
}

impl Actx {
    fn new(fail_at: u64) -> Box<Actx> {
        Box::new(Actx {
            magic: ACTX_MAGIC,
            fail_at,
            ..Default::default()
        })
    }
}

/// A `js_Alloc` that (a) proves `actx` is threaded through untouched,
/// (b) counts every allocating / freeing call so leaks can be compared, and
/// (c) can be made to return NULL on exactly the Nth allocating call.
unsafe extern "C" fn tracking_alloc(
    actx: *mut c_void,
    ptr: *mut c_void,
    size: c_int,
) -> *mut c_void {
    if actx.is_null() {
        // cannot record anything; fall back to plain realloc semantics
        if size == 0 {
            free(ptr);
            return null_mut();
        }
        return realloc(ptr, size as usize);
    }
    let cx = &mut *(actx as *mut Actx);
    if cx.magic != ACTX_MAGIC {
        cx.bad_actx += 1;
    }
    if size == 0 {
        if !ptr.is_null() {
            cx.nfree += 1;
            cx.live -= 1;
        }
        free(ptr);
        return null_mut();
    }
    cx.ncalls += 1;
    if cx.fail_at != 0 && cx.ncalls == cx.fail_at {
        cx.failed += 1;
        return null_mut();
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

/* ------------------------------------------------------------- diff driver */

/// Run `f` against the C library and then the Rust library, wrapping each run
/// in a fresh output buffer, and assert the two transcripts are byte-identical.
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
    assert_eq!(a, b, "divergence in [{tag}]");
    a
}

/// Snapshot of the top of the stack plus the frame size.
unsafe fn snap(l: &Lib, j: JS) -> String {
    format!(
        "top={} v={} ty={}",
        l.js_gettop(j),
        from_c(l.js_tryrepr(j, -1, ERRSTR)),
        from_c(l.js_typeof(j, -1))
    )
}

/// Describe the top of the stack without going through any `js_try`-based
/// helper.  `js_tryrepr` (jsrepr.c:275) has *no* `js_ptry` guard, so calling it
/// at `trytop == JS_TRYLIMIT` throws into the enclosing frame; the try-limit
/// tests must therefore inspect values with these non-throwing accessors only.
unsafe fn safeview(l: &Lib, j: JS) -> String {
    let mut s = format!(
        "type={} ty={}",
        l.js_type(j, -1),
        from_c(l.js_typeof(j, -1))
    );
    if l.pred("js_isstring", j, -1) != 0 {
        s.push_str(&format!(" s={:?}", from_c(l.js_tostring(j, -1))));
    }
    s
}

/// `js_ref` renders an object as `"%p"` of its address (jsrun.c:949), which is
/// necessarily different in the two processes' heaps.  Replace it with a stable
/// token so the rest of the transcript can still be compared byte-for-byte.
fn norm_ref(name: &str) -> String {
    if name.starts_with("0x") || name == "(nil)" {
        "<ptr>".to_string()
    } else {
        name.to_string()
    }
}

/// Pop back down to `base` without ever asking `js_pop` for more than there is
/// (`js_pop` past the frame raises js_error "stack underflow!").
unsafe fn drain_to(l: &Lib, j: JS, base: c_int) {
    let t = l.js_gettop(j);
    if t > base {
        l.js_pop(j, t - base);
    }
}

/// `js_ploadstring` + `js_pcall` of an expression, leaving its value on the
/// stack.  Returns the composite return code.
unsafe fn push_expr(l: &Lib, j: JS, src: &str) -> c_int {
    let cs = cstr(src);
    let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
    if rc != 0 {
        return 100 + rc;
    }
    l.js_pushundefined(j);
    l.js_pcall(j, 0)
}

/* ---------------------------------------------------------- C callbacks */

/// Records the frame it was called in, then returns a string.
unsafe extern "C" fn cf_probe(j: JS) {
    let l = cur();
    let top = l.js_gettop(j);
    let mut s = format!("[probe top={top} this={}", from_c(l.js_typeof(j, 0)));
    for i in 1..top {
        s.push_str(&format!(
            " a{i}={}:{}",
            from_c(l.js_typeof(j, i)),
            from_c(l.js_tryrepr(j, i, ERRSTR))
        ));
    }
    s.push_str("]\n");
    out_push(s.as_bytes());
    l.js_pushstring(j, cn!("probe-result"));
}

/// Pushes nothing at all (row 26: TOP == save_top -> undefined result).
unsafe extern "C" fn cf_void(j: JS) {
    let l = cur();
    out_push(format!("[void top={}]\n", l.js_gettop(j)).as_bytes());
}

/// Pushes three values (row 26: only the topmost survives).
unsafe extern "C" fn cf_three(j: JS) {
    let l = cur();
    out_push(format!("[three top={}]\n", l.js_gettop(j)).as_bytes());
    l.js_pushnumber(j, 1.0);
    l.js_pushstring(j, cn!("two"));
    l.js_pushboolean(j, 1);
}

/// A C constructor body: `this` is null, it builds its own object.
unsafe extern "C" fn cc_probe(j: JS) {
    let l = cur();
    let top = l.js_gettop(j);
    out_push(
        format!(
            "[cc top={top} this={}:{}]\n",
            from_c(l.js_typeof(j, 0)),
            from_c(l.js_tryrepr(j, 0, ERRSTR))
        )
        .as_bytes(),
    );
    l.js_newobject(j);
    l.js_pushnumber(j, top as f64);
    l.js_setproperty(j, -2, cn!("n"));
}

/// Row 56 / 79: js_currentfunction + js_currentfunctiondata inside a frame.
unsafe extern "C" fn cf_data(j: JS) {
    let l = cur();
    let d = l.js_currentfunctiondata(j);
    let ds = if d.is_null() {
        "<NULL>".to_string()
    } else {
        from_c(d as *const c_char)
    };
    l.js_currentfunction(j);
    out_push(
        format!(
            "[data={ds} fn={} callable={} same={}]\n",
            from_c(l.js_typeof(j, -1)),
            l.pred("js_iscallable", j, -1),
            (d == PAYLOAD as *mut c_void) as i32
        )
        .as_bytes(),
    );
    l.js_pop(j, 1);
    l.js_pushstring(j, cn!("dataf-ok"));
}

thread_local! {
    static REDEF_ATTS: Cell<c_int> = const { Cell::new(0) };
    static FIN_LIMIT: Cell<c_int> = const { Cell::new(0) };
}

/// Row 79 / row 12: `js_newcfunctionx` wraps its object construction in a
/// `js_try` whose handler runs `finalize(J, data)` and then rethrows.  The only
/// way to reach it through the public API is to run out of memory, so squeeze
/// `js_setlimit`'s memlimit shut first.  Must be called in a protected frame.
unsafe extern "C" fn cf_oom_cfunctionx(j: JS) {
    let l = cur();
    l.js_setlimit(j, 0, FIN_LIMIT.with(|c| c.get()));
    l.js_newcfunctionx(
        j,
        Some(cf_void),
        N_VOIDF,
        0,
        PAYLOAD as *mut c_void,
        Some(fin_cb),
    );
}

/// The same for `js_newuserdatax` (jsvalue.c:548).
unsafe extern "C" fn cf_oom_userdatax(j: JS) {
    let l = cur();
    l.js_newobject(j); // prototype, allocated before the limit closes
    l.js_setlimit(j, 0, FIN_LIMIT.with(|c| c.get()));
    l.js_newuserdatax(
        j,
        N_TAG,
        PAYLOAD as *mut c_void,
        Some(ud_has),
        Some(ud_put),
        Some(ud_del),
        Some(fin_cb),
    );
}

/// Protected `js_defglobal` re-definition (row 189): jsR_defproperty raises a
/// typeerror when the existing property carries JS_DONTCONF.
unsafe extern "C" fn cf_redef(j: JS) {
    let l = cur();
    l.js_pushstring(j, cn!("second"));
    l.js_defglobal(j, N_GV, REDEF_ATTS.with(|c| c.get()));
    l.js_getglobal(j, N_GV);
}

/// Protected `js_setglobal` (throws for a JS_READONLY global in strict mode).
unsafe extern "C" fn cf_setgv(j: JS) {
    let l = cur();
    l.js_pushnumber(j, 123.0);
    l.js_setglobal(j, N_GV);
    l.js_getglobal(j, N_GV);
}

/// Protected `js_delglobal` (throws for a JS_DONTCONF global in strict mode).
unsafe extern "C" fn cf_delgv(j: JS) {
    let l = cur();
    l.js_delglobal(j, N_GV);
    l.js_getglobal(j, N_GV);
}

unsafe extern "C" fn fin_cb(_j: JS, data: *mut c_void) {
    let ds = if data.is_null() {
        "<NULL>".to_string()
    } else {
        from_c(data as *const c_char)
    };
    out_push(format!("[finalize {ds}]\n").as_bytes());
}

/* ------------------------------------------------------ userdata hooks */

unsafe extern "C" fn ud_has(j: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let n = from_c(name);
    out_push(format!("[has {} {}]\n", from_c(data as *const c_char), n).as_bytes());
    if n.starts_with("magic") {
        cur().js_pushnumber(j, 42.0);
        return 1;
    }
    0
}

unsafe extern "C" fn ud_put(_j: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let n = from_c(name);
    out_push(format!("[put {} {}]\n", from_c(data as *const c_char), n).as_bytes());
    n.starts_with("ro") as c_int
}

unsafe extern "C" fn ud_del(_j: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let n = from_c(name);
    out_push(format!("[del {} {}]\n", from_c(data as *const c_char), n).as_bytes());
    n.starts_with("del") as c_int
}

/* --------------------------------------------- js_call / js_construct thunk */

thread_local! {
    /// 0 = js_call, 1 = js_construct
    static INNER_MODE: Cell<i32> = const { Cell::new(0) };
    /// depth at which the recursive js_pcall driver stops
    static REC_TARGET: Cell<i32> = const { Cell::new(0) };
    static REC_DEPTH: Cell<i32> = const { Cell::new(0) };
    /// source handed to the js_loadeval / js_eval thunks
    static EVAL_SRC: Cell<*const c_char> = const { Cell::new(std::ptr::null()) };
}

/// `inner(callee, a1, a2, ...)` -- performs the *unprotected* `js_call` or
/// `js_construct` from inside a protected cfunction frame.
unsafe extern "C" fn cf_inner(j: JS) {
    let l = cur();
    let top = l.js_gettop(j);
    let nargs = if top >= 2 { top - 2 } else { 0 };
    l.js_copy(j, 1); // callee
    let mode = INNER_MODE.with(|m| m.get());
    if mode == 0 {
        l.js_pushundefined(j); // this
    }
    for i in 0..nargs {
        l.js_copy(j, 2 + i);
    }
    if mode == 0 {
        l.js_call(j, nargs);
    } else {
        l.js_construct(j, nargs);
    }
}

/// `rec()` -- recursively `js_pcall`s itself so that `trytop` grows by one per
/// level, then exercises the protected entry points at the limit.
unsafe extern "C" fn cf_rec(j: JS) {
    let l = cur();
    let d = REC_DEPTH.with(|c| {
        let v = c.get() + 1;
        c.set(v);
        v
    });
    if d >= REC_TARGET.with(|c| c.get()) {
        let base = l.js_gettop(j);
        let src = cstr("1+1");
        let rc = l.js_ploadstring(j, FILENAME, src.as_ptr());
        out_push(
            format!(
                "[at depth {d}: ploadstring rc={rc} {}]\n",
                safeview(l, j)
            )
            .as_bytes(),
        );
        drain_to(l, j, base);
        let rc2 = l.js_dostring(j, src.as_ptr());
        out_push(format!("[at depth {d}: dostring rc={rc2}]\n").as_bytes());
        drain_to(l, j, base);
        l.js_pushnumber(j, d as f64);
    } else {
        l.js_getglobal(j, N_REC);
        l.js_pushundefined(j);
        let rc = l.js_pcall(j, 0);
        if rc != 0 {
            out_push(format!("[depth {d}: pcall rc={rc} {}]\n", safeview(l, j)).as_bytes());
        }
    }
    REC_DEPTH.with(|c| c.set(c.get() - 1));
}

/// Protected `js_loadeval` + `js_call` thunk (rows 17/18/34).
unsafe extern "C" fn cf_loadeval(j: JS) {
    let l = cur();
    let src = EVAL_SRC.with(|c| c.get());
    l.js_loadeval(j, cn!("(loadeval)"), src);
    let scriptty = from_c(l.js_typeof(j, -1));
    l.js_pushundefined(j);
    l.js_call(j, 0);
    out_push(
        format!(
            "[loadeval script={scriptty} result={}]\n",
            from_c(l.js_tryrepr(j, -1, ERRSTR))
        )
        .as_bytes(),
    );
}

/// Protected `js_eval` thunk (row 22).  Argument 1 is copied to the top and
/// `js_eval` is invoked on it.
unsafe extern "C" fn cf_eval(j: JS) {
    let l = cur();
    let before = l.js_gettop(j);
    l.js_copy(j, 1);
    l.js_eval(j);
    out_push(
        format!(
            "[eval before={before} after={} v={}]\n",
            l.js_gettop(j),
            from_c(l.js_tryrepr(j, -1, ERRSTR))
        )
        .as_bytes(),
    );
}

/// Report hook that writes straight to fd 2 (used by the forked panic child).
unsafe extern "C" fn stderr_report(_j: JS, msg: *const c_char) {
    let s = format!("REPORT:{}\n", from_c(msg));
    write(2, s.as_ptr() as *const c_void, s.len());
}

unsafe extern "C" fn custom_panic(_j: JS) {
    let s = "CUSTOMPANIC\n";
    write(2, s.as_ptr() as *const c_void, s.len());
}

/* --------------------------------------------------------- stderr capture */

static STDERR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Redirect fd 2 into a pipe for the duration of `f` and return what was
/// written.  Used to observe `js_defaultreport`, which writes to stderr.
fn capture_stderr(f: impl FnOnce()) -> String {
    let _g = STDERR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let mut fds = [0 as c_int; 2];
        if pipe(fds.as_mut_ptr()) != 0 {
            f();
            return "<pipe failed>".into();
        }
        let saved = dup(2);
        dup2(fds[1], 2);
        close(fds[1]);
        f();
        dup2(saved, 2);
        close(saved);
        let mut buf = vec![0u8; 1 << 16];
        let n = read(fds[0], buf.as_mut_ptr() as *mut c_void, buf.len());
        close(fds[0]);
        let n = if n < 0 { 0 } else { n as usize };
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }
}

/* ====================================================================== */
/*  Rows 1, 2, 4: js_newstate flags                                        */
/* ====================================================================== */

#[test]
fn t_newstate_flags() {
    // strictness is observable through undeclared assignment, `delete` of a
    // binding, duplicate parameters, octal literals and `with`.
    let probes = [
        "zz = 1; print(zz)",
        "var a = 1; print(delete a)",
        "print((function(){ return typeof this })())",
        "print((function(a,a){ return a })(1,2))",
        "print(function(){ return 010 }())",
        "var o={p:1}; with(o) print(p)",
        "print(typeof arguments)",
        "function f(){ return this } print(typeof f())",
        "print((function(){ 'use strict'; return typeof this })())",
        "eval('yy = 3'); print(typeof yy)",
    ];
    // flags 0 / JS_STRICT / extra bits above JS_STRICT (row 4)
    for flags in [0, JS_STRICT, 0x6, 0x7, -1, 0x1000_0000] {
        for src in probes {
            diff_dostring(flags, src);
            diff_eval(flags, src);
        }
    }
}

/* ====================================================================== */
/*  Row 3: custom js_Alloc with a non-NULL actx                            */
/* ====================================================================== */

#[test]
fn t_newstate_custom_alloc() {
    let p = libs();
    let mut rng = Rng::new(0xA110_C8);
    let scripts = [
        "print(1+1)",
        "var a=[]; for (var i=0;i<200;++i) a.push('s'+i); print(a.length, a[199])",
        "print(JSON.stringify({a:[1,2,3],b:'x'}))",
        "print('abcdefghijklmnopqrstuvwxyz'.toUpperCase())",
        "print(/(a+)(b+)/.exec('xaaabbbz'))",
        "print(new Date(0).getTime())",
    ];
    for src in scripts {
        let mut recs = vec![];
        for l in [&p.c, &p.rs] {
            out_clear();
            let mut cx = Actx::new(0);
            let ap = &mut *cx as *mut Actx as *mut c_void;
            unsafe {
                set_cur(l);
                let j = l.js_newstate(Some(tracking_alloc), ap, 0);
                assert!(!j.is_null(), "{}: newstate w/ custom alloc", l.name);
                l.js_setreport(j, Some(report_cb));
                l.js_newcfunction(j, Some(print_cb), PRINT, 1);
                l.js_setglobal(j, PRINT);
                let cs = cstr(src);
                let rc = l.js_dostring(j, cs.as_ptr());
                let top = l.js_gettop(j);
                l.js_freestate(j);
                recs.push(format!(
                    "rc={rc} top={top} live={} bad={} out={}",
                    cx.live,
                    cx.bad_actx,
                    out_take()
                ));
            }
            // the custom allocator must have been used for everything, and
            // js_freestate must have handed every block back
            assert_eq!(cx.live, 0, "{}: {} live blocks leaked", l.name, cx.live);
            assert_eq!(cx.bad_actx, 0, "{}: actx was corrupted", l.name);
            assert!(cx.nalloc > 100, "{}: only {} allocs", l.name, cx.nalloc);
        }
        assert_eq!(recs[0], recs[1], "custom-alloc divergence for {src}");
    }

    // random actx pointers must be handed back verbatim; actx is independent of
    // js_setcontext's uctx (row 5)
    for _ in 0..8 {
        let tagbits = rng.next_u64();
        let mut cx = Actx::new(0);
        cx.live = 0;
        let ap = &mut *cx as *mut Actx as *mut c_void;
        for l in [&p.c, &p.rs] {
            unsafe {
                set_cur(l);
                let j = l.js_newstate(Some(tracking_alloc), ap, 0);
                assert!(!j.is_null());
                let u = tagbits as usize as *mut c_void;
                l.js_setcontext(j, u);
                assert_eq!(l.js_getcontext(j), u, "{}: uctx round-trip", l.name);
                l.js_freestate(j);
            }
        }
        assert_eq!(cx.live, 0);
        assert_eq!(cx.bad_actx, 0);
    }
}

/* ====================================================================== */
/*  Row 3 / "js_newstate returns NULL": allocator failing on the Nth call  */
/* ====================================================================== */

#[test]
fn t_newstate_alloc_fails() {
    let p = libs();

    // 1st allocating call is the js_State itself, 2nd is the value stack.
    // Both must be handled without touching the allocator again except to free.
    for n in [1u64, 2] {
        for l in [&p.c, &p.rs] {
            let mut cx = Actx::new(n);
            let ap = &mut *cx as *mut Actx as *mut c_void;
            unsafe {
                let j = l.js_newstate(Some(tracking_alloc), ap, 0);
                assert!(j.is_null(), "{}: newstate must fail at alloc #{n}", l.name);
            }
            assert_eq!(cx.failed, 1, "{}: fail_at={n} not hit", l.name);
            assert_eq!(cx.live, 0, "{}: fail_at={n} leaked", l.name);
        }
    }

    // Every later failure happens inside jsB_init, where js_malloc raises
    // "out of memory" and longjmps back to js_newstate's own js_try, which
    // js_freestate()s and returns NULL.  Sweep a wide range of N.
    let mut ncalls = vec![];
    for l in [&p.c, &p.rs] {
        let mut cx = Actx::new(0);
        let ap = &mut *cx as *mut Actx as *mut c_void;
        unsafe {
            let j = l.js_newstate(Some(tracking_alloc), ap, 0);
            assert!(!j.is_null());
            l.js_freestate(j);
        }
        assert_eq!(cx.live, 0);
        ncalls.push(cx.ncalls);
    }
    assert_eq!(
        ncalls[0], ncalls[1],
        "js_newstate performs a different number of allocations in C ({}) and Rust ({})",
        ncalls[0], ncalls[1]
    );
    let total = ncalls[0];
    assert!(total > 300, "expected many allocations, got {total}");

    let mut rng = Rng::new(0xDEAD_10);
    let mut probes: Vec<u64> = (3..40).collect();
    probes.extend([total / 4, total / 2, total - 2, total - 1, total]);
    for _ in 0..40 {
        probes.push(3 + (rng.next_u64() % (total - 3)));
    }
    probes.sort_unstable();
    probes.dedup();
    // NOTE: not every failing allocation aborts construction. jsbuiltin.c:205
    // assigns `js_regcompx(...)` to RegExp_prototype->u.r.prog without checking
    // for NULL, and regexp.c:903 turns an allocation failure into a plain NULL
    // return rather than a throw, so a failure inside the "(?:)" compile leaves
    // a usable state with a NULL prog.  What must match is the *decision*, so
    // compare C against Rust for every N instead of demanding NULL.
    let mut nnull = 0;
    for n in &probes {
        let n = *n;
        let mut res = vec![];
        for l in [&p.c, &p.rs] {
            let mut cx = Actx::new(n);
            let ap = &mut *cx as *mut Actx as *mut c_void;
            let isnull = unsafe {
                let j = l.js_newstate(Some(tracking_alloc), ap, 0);
                let r = j.is_null();
                if !r {
                    l.js_freestate(j);
                }
                r
            };
            res.push(format!(
                "null={isnull} failed={} live={}",
                cx.failed, cx.live
            ));
        }
        assert_eq!(res[0], res[1], "alloc failure at call #{n} diverges");
        assert!(
            res[0].ends_with("live=0"),
            "alloc failure at call #{n} leaked: {}",
            res[0]
        );
        if res[0].starts_with("null=true") {
            nnull += 1;
        }
    }
    assert!(
        nnull > probes.len() / 2,
        "only {nnull}/{} failing allocations produced a NULL state",
        probes.len()
    );

    // Beyond the total number of allocations the state must come up fine.
    for n in [total + 1, total + 5, u32::MAX as u64] {
        for l in [&p.c, &p.rs] {
            let mut cx = Actx::new(n);
            let ap = &mut *cx as *mut Actx as *mut c_void;
            unsafe {
                let j = l.js_newstate(Some(tracking_alloc), ap, 0);
                assert!(!j.is_null(), "{}: fail_at={n} should not fail", l.name);
                l.js_freestate(j);
            }
            assert_eq!(cx.live, 0);
        }
    }
}

/* ====================================================================== */
/*  Row 5: js_setcontext / js_getcontext                                   */
/* ====================================================================== */

#[test]
fn t_setcontext_getcontext() {
    let p = libs();
    let mut rng = Rng::new(0xC0FFEE);
    let mut vals: Vec<usize> = vec![0, 1, usize::MAX, 8, 0x1000];
    for _ in 0..64 {
        vals.push(rng.next_u64() as usize);
    }
    for l in [&p.c, &p.rs] {
        unsafe {
            set_cur(l);
            let j = new_state(l, 0);
            // fresh state: uctx is NULL (memset)
            assert!(l.js_getcontext(j).is_null(), "{}: initial uctx", l.name);
            for v in &vals {
                let ptr = *v as *mut c_void;
                l.js_setcontext(j, ptr);
                assert_eq!(l.js_getcontext(j), ptr, "{}: uctx {v:#x}", l.name);
            }
            // setting it back to NULL works too
            l.js_setcontext(j, null_mut());
            assert!(l.js_getcontext(j).is_null());
            l.js_freestate(j);
        }
    }
}

/* ====================================================================== */
/*  Rows 6, 7: js_setreport with default / custom / NULL                   */
/* ====================================================================== */

#[test]
fn t_setreport() {
    let p = libs();
    let mut rng = Rng::new(0x5EE7);

    // (a) the js_newstate default hook writes "<message>\n" to stderr
    let mut msgs: Vec<String> = vec![
        String::new(),
        "x".into(),
        "hello report".into(),
        "line1\nline2".into(),
    ];
    for _ in 0..20 {
        msgs.push(rng.ascii_string(40));
    }
    for m in &msgs {
        let mut got = vec![];
        for l in [&p.c, &p.rs] {
            let cs = cstr(m);
            let text = capture_stderr(|| unsafe {
                set_cur(l);
                let j = l.js_newstate(None, null_mut(), 0);
                assert!(!j.is_null());
                l.js_report(j, cs.as_ptr());
                l.js_freestate(j);
            });
            got.push(text);
        }
        assert_eq!(got[0], got[1], "default report divergence for {m:?}");
        assert_eq!(got[0], format!("{}\n", m.replace('\0', "")));
    }

    // (b) custom hook receives the message verbatim
    for m in &msgs {
        diff2(&format!("custom report {m:?}"), |l| unsafe {
            let j = new_state(l, 0);
            let cs = cstr(m);
            l.js_report(j, cs.as_ptr());
            l.js_report(j, cs.as_ptr());
            let s = snap(l, j);
            l.js_freestate(j);
            s
        });
    }

    // (c) js_setreport(J, NULL): every js_report becomes a silent no-op, and
    //     js_gc(J, 1) formats its summary and drops it (row 7 / row 191)
    for m in &msgs {
        let mut got = vec![];
        for l in [&p.c, &p.rs] {
            let cs = cstr(m);
            out_clear();
            let text = capture_stderr(|| unsafe {
                set_cur(l);
                let j = new_state(l, 0);
                l.js_setreport(j, None);
                l.js_report(j, cs.as_ptr());
                l.js_gc(j, 1);
                l.js_gc(j, 0);
                l.js_report(j, cs.as_ptr());
                let d = cstr("var a = {}; a.b = [1,2,3]; throw new Error('e')");
                let rc = l.js_dostring(j, d.as_ptr());
                out_push(format!("dostring rc={rc}\n").as_bytes());
                l.js_freestate(j);
            });
            got.push(format!("stderr={text:?} buf={:?}", out_take()));
        }
        assert_eq!(got[0], got[1], "NULL report divergence for {m:?}");
        assert!(
            got[0].starts_with("stderr=\"\""),
            "NULL report hook still wrote to stderr: {}",
            got[0]
        );
    }

    // (d) report never called at all
    diff2("report never called", |l| unsafe {
        let j = new_state(l, 0);
        let s = cstr("print('ok'); 1+1");
        let rc = l.js_dostring(j, s.as_ptr());
        let r = format!("rc={rc} {}", snap(l, j));
        l.js_freestate(j);
        r
    });
}

/* ====================================================================== */
/*  Rows 8, 9: js_atpanic                                                  */
/* ====================================================================== */

#[test]
fn t_atpanic_roundtrip() {
    let p = libs();
    for l in [&p.c, &p.rs] {
        unsafe {
            set_cur(l);
            let j = new_state(l, 0);
            // the default handler installed by js_newstate must be non-NULL
            let default = l.js_atpanic(j, Some(custom_panic));
            assert!(
                default.is_some(),
                "{}: js_atpanic returned NULL for the default handler",
                l.name
            );
            // installing again returns exactly what we put in
            let prev = l.js_atpanic(j, None);
            assert_eq!(
                prev.map(|f| f as *const () as usize),
                Some(custom_panic as *const () as usize),
                "{}: js_atpanic did not return the previous handler",
                l.name
            );
            // and NULL round-trips
            let prev2 = l.js_atpanic(j, default);
            assert!(prev2.is_none(), "{}: expected NULL back", l.name);
            let prev3 = l.js_atpanic(j, Some(custom_panic));
            assert_eq!(
                prev3.map(|f| f as *const () as usize),
                default.map(|f| f as *const () as usize),
                "{}: default handler not restored",
                l.name
            );
            l.js_atpanic(j, default);
            l.js_freestate(j);
        }
    }
}

/// Row 8/9: the panic hook only runs for a `js_throw` with `trytop == 0`, which
/// ends in `abort()`.  Drive it in a forked copy of this binary.
#[test]
fn t_panic_child() {
    let Ok(spec) = std::env::var("MUJS_PANIC_CHILD") else {
        return;
    };
    let (which, mode) = spec.split_once(':').expect("spec");
    let p = libs();
    let l = if which == "c" { &p.c } else { &p.rs };
    unsafe {
        set_cur(l);
        let j = l.js_newstate(None, null_mut(), 0);
        assert!(!j.is_null());
        l.js_setreport(j, Some(stderr_report));
        if mode == "custom" {
            let old = l.js_atpanic(j, Some(custom_panic));
            assert!(old.is_some());
        }
        l.js_pushstring(j, cn!("thrown-value"));
        l.js_throw(j); // trytop == 0 -> panic hook -> abort()
    }
    unreachable!("js_throw at trytop==0 must not return");
}

#[test]
fn t_panic_default_vs_custom() {
    if std::env::var_os("MUJS_PANIC_CHILD").is_some() {
        return;
    }
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    let exe = std::env::current_exe().expect("current_exe");
    for mode in ["default", "custom"] {
        let mut res = vec![];
        for which in ["c", "rs"] {
            let mut cmd = std::process::Command::new(&exe);
            cmd.args(["t_panic_child", "--exact", "--nocapture", "--test-threads=1"])
                .env("MUJS_PANIC_CHILD", format!("{which}:{mode}"))
                .env("RUST_BACKTRACE", "0");
            unsafe {
                cmd.pre_exec(|| {
                    // no multi-megabyte core dumps for the deliberate abort()
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
            let marks: Vec<&str> = err
                .lines()
                .filter(|l| l.starts_with("REPORT:") || l.starts_with("CUSTOMPANIC"))
                .collect();
            res.push(format!("signal={:?} marks={:?}", out.status.signal(), marks));
        }
        assert_eq!(res[0], res[1], "panic ({mode}) divergence");
        assert!(
            res[0].contains("signal=Some(6)"),
            "expected SIGABRT for panic ({mode}): {}",
            res[0]
        );
        if mode == "default" {
            assert!(
                res[0].contains("REPORT:uncaught exception"),
                "default panic must report: {}",
                res[0]
            );
        } else {
            assert!(
                res[0].contains("CUSTOMPANIC"),
                "custom panic must run: {}",
                res[0]
            );
        }
    }
}

/* ====================================================================== */
/*  Rows 10-13: js_setlimit                                                */
/* ====================================================================== */

fn limited(l: &Lib, flags: c_int, runlimit: c_int, memlimit: c_int, src: &str) -> String {
    unsafe {
        let j = new_state(l, flags);
        l.js_setlimit(j, runlimit, memlimit);
        let cs = cstr(src);
        let rc = l.js_dostring(j, cs.as_ptr());
        let top = l.js_gettop(j);
        l.js_freestate(j);
        format!("rc={rc} top={top}")
    }
}

#[test]
fn t_setlimit_offswitch() {
    // Row 10: both checks are `> 0`, so 0 and every negative value is unlimited
    let mut rng = Rng::new(0x1111_2222);
    let mut pairs: Vec<(c_int, c_int)> = vec![
        (0, 0),
        (0, -1),
        (-1, 0),
        (-1, -1),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        (0, i32::MAX),
        (i32::MAX, 0),
    ];
    for _ in 0..24 {
        pairs.push((
            -(rng.below(1 << 20) as c_int),
            -(rng.below(1 << 20) as c_int),
        ));
    }
    let srcs = [
        "var s=0; for (var i=0;i<500;++i) s+=i; print(s)",
        "print([1,2,3].concat([4,5]).join(','))",
        "print('x'.repeat ? 'has' : 'none')",
        "var o={}; for (var i=0;i<100;++i) o['k'+i]=i; print(Object.keys(o).length)",
    ];
    for (r, m) in pairs {
        for src in srcs {
            diff2(&format!("setlimit({r},{m}) {src}"), |l| {
                limited(l, 0, r, m, src)
            });
        }
    }
}

#[test]
fn t_setlimit_runlimit() {
    // Row 11: one decrement per VM instruction, thrown as "script ran too long"
    let mut rng = Rng::new(0x3333_4444);
    let mut lims: Vec<c_int> = (1..=140).collect();
    lims.extend([200, 400, 1000, 5000, 20000, 100000]);
    for _ in 0..120 {
        lims.push(1 + rng.below(6000) as c_int);
    }
    let srcs = [
        "1+1",
        "print('hi')",
        "var s=0; for (var i=0;i<200;++i) s+=i; print(s)",
        "function f(n){ return n<=1?1:n*f(n-1) } print(f(10))",
        "try { var s=0; for(;;) ++s } catch (e) { print('caught', e) }",
    ];
    for lim in &lims {
        for src in srcs {
            diff2(&format!("runlimit({lim}) {src}"), |l| {
                limited(l, 0, *lim, 0, src)
            });
        }
    }

    // the message must actually be "script ran too long", and pure C-API
    // sequences must never consume any budget
    diff2("runlimit=1 message", |l| unsafe {
        let j = new_state(l, 0);
        l.js_setlimit(j, 1, 0);
        // no VM instruction runs here, so nothing is charged
        for i in 0..50 {
            l.js_pushnumber(j, i as f64);
            l.js_newobject(j);
            l.js_newarray(j);
            l.js_pushstring(j, cn!("a fairly long string value here"));
            l.js_pop(j, 4);
        }
        let mid = l.js_gettop(j);
        let cs = cstr("1+1");
        let rc = l.js_dostring(j, cs.as_ptr());
        let r = format!("mid={mid} rc={rc} top={}", l.js_gettop(j));
        l.js_freestate(j);
        r
    });
    let (rc, out) = {
        let p = libs();
        let a = unsafe {
            out_clear();
            let j = new_state(&p.c, 0);
            p.c.js_setlimit(j, 1, 0);
            let cs = cstr("1+1");
            let rc = p.c.js_dostring(j, cs.as_ptr());
            p.c.js_freestate(j);
            (rc, out_take())
        };
        a
    };
    assert_eq!(rc, 1);
    assert_eq!(out, "[report] script ran too long\n");
}

#[test]
fn t_setlimit_memlimit() {
    // Row 12: every js_malloc/js_realloc subtracts `size`; js_free credits
    // nothing back, so the budget only shrinks.
    let mut rng = Rng::new(0x5555_6666);
    let mut lims: Vec<c_int> = (1..=96).collect();
    lims.extend([128, 192, 256, 384, 512, 1024, 2048, 4096, 1 << 14, 1 << 16, 1 << 18]);
    for _ in 0..140 {
        lims.push(1 + rng.below(1 << 17) as c_int);
    }
    let srcs = [
        "1+1",
        "print('hello world')",
        "var a=[]; for (var i=0;i<300;++i) a.push('str'+i); print(a.length)",
        "print(JSON.stringify({a:1,b:[1,2,3]}))",
        "var s=''; for (var i=0;i<200;++i) s+='abcdefgh'; print(s.length)",
        "print(/(a+)(b+)c?/.exec('xxaaabbbz'))",
        "print(new Date(86400000).toUTCString())",
        "print(JSON.parse('{\"k\":[1,2,{\"m\":\"v\"}]}').k.length)",
        "print(encodeURIComponent('a b/c?d=e&f'))",
        "print([5,3,1,4,2].sort().join('-'))",
        "function f(n){ return n<2?n:f(n-1)+f(n-2) } print(f(12))",
        "print('abcdef'.replace(/b(c)d/, '[$1]'))",
        "try { throw new TypeError('t') } catch (e) { print(e.name, e.message) }",
    ];
    for lim in &lims {
        for src in srcs {
            diff2(&format!("memlimit({lim}) {src}"), |l| {
                limited(l, 0, 0, *lim, src)
            });
        }
    }

    // small memlimit + the C API: js_pushstring of a >15 byte string allocates
    for lim in [1, 2, 8, 17, 32] {
        diff2(&format!("memlimit({lim}) api"), |l| unsafe {
            let j = new_state(l, 0);
            l.js_setlimit(j, 0, lim);
            // js_dostring is protected, so the out-of-memory throw is caught
            let cs = cstr("var q = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaa'; print(q.length)");
            let rc = l.js_dostring(j, cs.as_ptr());
            let r = format!("rc={rc} top={}", l.js_gettop(j));
            l.js_freestate(j);
            r
        });
    }

    let mut got = None;
    for lim in [1, 2, 3] {
        let p = libs();
        out_clear();
        let a = unsafe {
            let j = new_state(&p.c, 0);
            p.c.js_setlimit(j, 0, lim);
            let cs = cstr("1+1");
            let rc = p.c.js_dostring(j, cs.as_ptr());
            p.c.js_freestate(j);
            (rc, out_take())
        };
        if a.0 == 1 {
            got = Some(a);
            break;
        }
    }
    let (rc, out) = got.expect("memlimit never triggered");
    assert_eq!(rc, 1);
    assert_eq!(out, "[report] out of memory\n");
}

/// After a memlimit-driven "out of memory" throw the budget stays exhausted, so
/// every later allocation throws too. The state must still be collectable and
/// freeable, and both libraries must agree on the exact gc summary.
#[test]
fn t_setlimit_memlimit_then_gc() {
    let mut rng = Rng::new(0xAA55_AA55);
    let mut lims: Vec<c_int> = (1..=48).collect();
    lims.extend([64, 128, 512, 4096, 1 << 15]);
    for _ in 0..60 {
        lims.push(1 + rng.below(1 << 16) as c_int);
    }
    for lim in lims {
        diff2(&format!("memlimit then gc {lim}"), move |l| unsafe {
            let j = new_state(l, 0);
            let pre = cstr("var live = {a:1,b:'a string longer than fifteen bytes'}; 0");
            let rc0 = l.js_dostring(j, pre.as_ptr());
            l.js_setlimit(j, 0, lim);
            let cs = cstr("var big=[]; for (var i=0;i<200;++i) big.push('item'+i); big.length");
            let rc = l.js_dostring(j, cs.as_ptr());
            // js_free credits nothing back, so the budget is still gone
            let rc2 = l.js_dostring(j, cs.as_ptr());
            let mut r = format!("rc0={rc0} rc={rc} rc2={rc2} top={}", l.js_gettop(j));
            // js_gc only frees, so it can always run
            l.js_gc(j, 1);
            l.js_gc(j, 1);
            let rc3 = l.js_dostring(j, cs.as_ptr());
            r.push_str(&format!(" rc3={rc3} "));
            // lifting the limit again makes the state usable
            l.js_setlimit(j, 0, 0);
            let rc4 = l.js_dostring(j, cs.as_ptr());
            r.push_str(&format!("rc4={rc4} top={}", l.js_gettop(j)));
            l.js_gc(j, 1);
            l.js_freestate(j);
            r
        });
    }
}

#[test]
fn t_setlimit_both() {
    // Row 13: two independent counters
    let mut rng = Rng::new(0x7777_8888);
    let mut pairs: Vec<(c_int, c_int)> = vec![
        (1, 1),
        (1, 1 << 20),
        (1 << 20, 1),
        (10, 100),
        (100, 10),
        (5000, 1 << 16),
        (1 << 16, 5000),
    ];
    for _ in 0..48 {
        pairs.push((
            1 + rng.below(3000) as c_int,
            1 + rng.below(1 << 16) as c_int,
        ));
    }
    let srcs = [
        "var s=0; for (var i=0;i<100;++i) s+=('x'+i).length; print(s)",
        "print([1,2,3,4,5].map ? 'map' : 'nomap')",
        "var o={}; for (var i=0;i<50;++i) o['key'+i]={v:i}; print(Object.keys(o).length)",
    ];
    for (r, m) in pairs {
        for src in srcs {
            diff2(&format!("setlimit({r},{m}) {src}"), |l| {
                limited(l, 0, r, m, src)
            });
        }
    }
}

/* ====================================================================== */
/*  Rows 14-22: loadstring / loadeval / ploadstring / dostring / eval       */
/* ====================================================================== */

const LOAD_SRCS: &[&str] = &[
    "1+1",
    "var x = 1; x",
    "'use strict'; var x = 1; x",
    "\"use strict\"\nzz = 1; zz",
    "'use strict'; function f(a,a){return a} f(1,2)",
    "function g(){ return typeof this } g()",
    "yy = 5; yy",
    "var a = 1; delete a",
    "010",
    "'use strict'; 010",
    "print('side effect'); 7",
    "syntax ((( error",
    "",
    "   ",
    "// only a comment",
    "throw new Error('boom')",
    "eval('var e1 = 1'); typeof e1",
    "'use strict'; eval('var e2 = 1'); typeof e2",
    "function h(){ eval('var e3 = 1'); return typeof e3 } h() + ':' + typeof e3",
    "function h(){ 'use strict'; eval('var e4 = 1'); return typeof e4 } h() + ':' + typeof e4",
    "var ev = eval; ev('1')",
    "(0, eval)('1')",
    "this.eval",
];

#[test]
fn t_loadstring_and_ploadstring() {
    // Rows 14, 15, 16, 19, 21
    for flags in [0, JS_STRICT] {
        for src in LOAD_SRCS {
            let s = src.to_string();
            diff2(&format!("ploadstring flags={flags} {src:?}"), move |l| unsafe {
                let j = new_state(l, flags);
                let cs = cstr(&s);
                let load_rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
                let mut r = format!("load={load_rc} {}", snap(l, j));
                if load_rc == 0 {
                    // the script object is on the stack (row 19)
                    r.push_str(&format!(
                        " isscript={} callable={}",
                        l.js_type(j, -1),
                        l.pred("js_iscallable", j, -1)
                    ));
                    l.js_pushundefined(j);
                    let call_rc = l.js_pcall(j, 0);
                    r.push_str(&format!(" call={call_rc} {}", snap(l, j)));
                }
                l.js_pop(j, 1);
                r.push_str(&format!(" end={}", l.js_gettop(j)));
                l.js_freestate(j);
                r
            });

            // Row 21: js_dostring
            diff_dostring(flags, src);
        }
    }
}

#[test]
fn t_loadeval() {
    // Rows 17, 18, 34: iseval=1 with J->strict 0 (scope NULL) and 1 (scope E)
    for flags in [0, JS_STRICT] {
        for src in LOAD_SRCS {
            let s = src.to_string();
            diff2(&format!("loadeval flags={flags} {src:?}"), move |l| unsafe {
                let j = new_state(l, flags);
                let cs = cstr(&s);
                EVAL_SRC.with(|c| c.set(cs.as_ptr()));
                l.js_newcfunction(j, Some(cf_loadeval), cn!("le"), 0);
                l.js_pushundefined(j);
                let rc = l.js_pcall(j, 0);
                let mut r = format!("rc={rc} {}", snap(l, j));
                l.js_pop(j, 1);
                // and again from inside a strict JS function, so J->strict==1
                l.js_newcfunction(j, Some(cf_loadeval), cn!("le"), 0);
                l.js_setglobal(j, cn!("le"));
                let d = cstr("function w(){ 'use strict'; return le() } print(w())");
                let rc2 = l.js_dostring(j, d.as_ptr());
                r.push_str(&format!(" strictrc={rc2} top={}", l.js_gettop(j)));
                l.js_freestate(j);
                EVAL_SRC.with(|c| c.set(std::ptr::null()));
                r
            });
        }
    }
}

#[test]
fn t_js_eval() {
    // Row 22: a string on top vs a non-string on top (early return)
    let mut rng = Rng::new(0x9999_AAAA);
    let mut cases: Vec<String> = LOAD_SRCS.iter().map(|s| s.to_string()).collect();
    for _ in 0..24 {
        cases.push(rng.ascii_string(24));
    }
    for flags in [0, JS_STRICT] {
        for src in &cases {
            let s = src.clone();
            diff2(&format!("js_eval flags={flags} {s:?}"), move |l| unsafe {
                let j = new_state(l, flags);
                l.js_newcfunction(j, Some(cf_eval), cn!("ev"), 1);
                l.js_pushundefined(j);
                let cs = cstr(&s);
                l.js_pushstring(j, cs.as_ptr());
                let rc = l.js_pcall(j, 1);
                let r = format!("str rc={rc} {}", snap(l, j));
                l.js_pop(j, 1);
                l.js_freestate(j);
                r
            });
        }
        // non-string tops: js_eval returns immediately, stack untouched
        diff2(&format!("js_eval nonstring flags={flags}"), |l| unsafe {
            let j = new_state(l, flags);
            let mut r = String::new();
            for k in 0..6 {
                l.js_newcfunction(j, Some(cf_eval), cn!("ev"), 1);
                l.js_pushundefined(j);
                match k {
                    0 => l.js_pushundefined(j),
                    1 => l.js_pushnull(j),
                    2 => l.js_pushboolean(j, 1),
                    3 => l.js_pushnumber(j, 42.5),
                    4 => l.js_newobject(j),
                    _ => l.js_newarray(j),
                }
                let rc = l.js_pcall(j, 1);
                r.push_str(&format!("k={k} rc={rc} {} | ", snap(l, j)));
                l.js_pop(j, 1);
            }
            l.js_freestate(j);
            r
        });
    }
}

/// The `iseval` axis in detail: `eval("var x=1")` picks `J->strict` and captures
/// `J->E` (or NULL when non-strict), while a plain script picks
/// `J->default_strict` and always captures `J->GE`.  Indirect uses of `eval` are
/// rejected by the compiler with js_evalerror "invalid use of 'eval'".
#[test]
fn t_eval_axis() {
    let direct = [
        // eval declaring a var, at top level and inside a function
        "eval('var x = 1'); print(typeof x, x)",
        "function f(){ eval('var x = 1'); return typeof x } print(f(), typeof x)",
        "function f(){ 'use strict'; eval('var x = 1'); return typeof x } print(f(), typeof x)",
        "'use strict'; eval('var x = 1'); print(typeof x)",
        // the same thing as a plain script (var goes to the global env)
        "var x = 1; print(typeof x, x)",
        "function f(){ var x = 1; return typeof x } print(f(), typeof x)",
        // eval seeing and mutating the caller's scope
        "function f(){ var y = 2; eval('y = 3'); return y } print(f())",
        "function f(){ 'use strict'; var y = 2; eval('y = 3'); return y } print(f())",
        // eval inheriting / not inheriting strictness
        "print(eval('typeof this'))",
        "'use strict'; print(eval('typeof this'))",
        "function f(){ return eval('zzz = 1') } try { print(f()) } catch (e) { print('E', e) }",
        "function f(){ 'use strict'; return eval('zzz = 1') } try { print(f()) } catch (e) { print('E', e) }",
        "print(eval('010'))",
        "'use strict'; try { print(eval('010')) } catch (e) { print('E', e) }",
        // eval returning the value of its last expression
        "print(eval('1+1'), eval('\'s\''), eval(''), eval('var q'), typeof eval(1))",
        // nested eval
        "print(eval('eval(\'1+2\')'))",
        "function f(){ return eval('eval(\'typeof this\')') } print(f())",
    ];
    let indirect = [
        "var ev = eval; print(ev('1'))",
        "(0, eval)('1')",
        "print(this.eval)",
        "var o = {eval: 1}; print(o.eval)",
        "function eval(){} ",
        "'use strict'; var ev = eval;",
        "try { eval = 1 } catch (e) { print('E', e) }",
        "print(typeof eval)",
    ];
    let mut sawevalerror = 0;
    for flags in [0, JS_STRICT] {
        for src in direct.iter().chain(indirect.iter()) {
            diff_dostring(flags, src);
            diff_eval(flags, src);
            let t = diff2(&format!("eval axis flags={flags} {src:?}"), move |l| unsafe {
                let j = new_state(l, flags);
                let cs = cstr(src);
                let rc = l.js_dostring(j, cs.as_ptr());
                let r = format!("rc={rc} top={}", l.js_gettop(j));
                l.js_freestate(j);
                r
            });
            if t.contains("invalid use of 'eval'") {
                sawevalerror += 1;
            }
        }
    }
    assert!(
        sawevalerror >= 4,
        "the indirect-eval compile error never fired ({sawevalerror})"
    );
}

/* ====================================================================== */
/*  Rows 23-40: js_call / js_pcall / js_construct / js_pconstruct           */
/* ====================================================================== */

/// Sources whose *value* is the callee.  `None` means "built natively".
const JS_CALLEES: &[&str] = &[
    "(function(){ return 1 })",                                       // 0 params
    "(function(a,b){ return String(a)+'/'+String(b) })",              // lightweight, 2 params
    "(function(a){ return arguments.length + ':' + String(a) })",     // arguments, non-strict
    "(function(a){ 'use strict'; return arguments.length+':'+String(a) })",
    "(function(a){ var h = function(){ return a }; return h() })",    // closure -> heavyweight
    "(function(){ throw new Error('boom') })",                        // throwing callee
    "(function(a,b,c,d,e){ return [a,b,c,d,e].join(',') })",          // arity 5
    "(function(){ return this === undefined ? 'undef' : typeof this })",
    "(function(){ 'use strict'; return this === undefined ? 'undef' : typeof this })",
    "(function(){ return {tag:'obj'} })",                             // ctor returning object
    "(function(){ return 5 })",                                       // ctor returning primitive
    "(function(){ this.k = 1 })",                                     // ctor mutating this
    "(function(){})",                                                 // ctor returning nothing
    "42",
    "'not callable'",
    "({})",
    "undefined",
    "null",
    "[1,2,3]",
    "Object",
    "Array",
    "Error",
    "(function F(){ this.p = 1 })",
];

/// Pushes callee number `k` onto the stack.  `k >= JS_CALLEES.len()` selects a
/// natively built callee.
unsafe fn push_callee(l: &Lib, j: JS, k: usize) -> String {
    if k < JS_CALLEES.len() {
        let rc = push_expr(l, j, JS_CALLEES[k]);
        return format!("callee{k}=js({rc})");
    }
    match k - JS_CALLEES.len() {
        0 => {
            l.js_newcfunction(j, Some(cf_probe), N_PROBE, 0);
            "callee=cf0".into()
        }
        1 => {
            l.js_newcfunction(j, Some(cf_probe), N_PROBE, 2);
            "callee=cf2".into()
        }
        2 => {
            l.js_newcfunction(j, Some(cf_probe), N_PROBE, 5);
            "callee=cf5".into()
        }
        3 => {
            l.js_newcfunction(j, Some(cf_void), N_VOIDF, 0);
            "callee=cfvoid".into()
        }
        4 => {
            l.js_newcfunction(j, Some(cf_three), N_THREE, 1);
            "callee=cfthree".into()
        }
        5 => {
            l.js_newobject(j);
            l.js_newcconstructor(j, Some(cf_probe), Some(cc_probe), N_CTOR, 1);
            "callee=cctor".into()
        }
        6 => {
            // js_newcfunction: u.c.constructor == NULL (row 37)
            l.js_newcfunction(j, Some(cf_probe), N_PROBE, 1);
            "callee=cfnoctor".into()
        }
        7 => {
            // a JS function whose 'prototype' is not an object (row 38)
            let rc = push_expr(l, j, "(function(){ var f=function(){}; f.prototype=5; return f })()");
            format!("callee=protonum({rc})")
        }
        8 => {
            // script object, scope == J->GE (row 33)
            let cs = cstr("var sv = 1; 'script-ran'");
            let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
            format!("callee=script({rc})")
        }
        9 => {
            l.js_newcfunctionx(j, Some(cf_data), N_DATAF, 1, PAYLOAD as *mut c_void, Some(fin_cb));
            "callee=cfdata".into()
        }
        _ => {
            l.js_newobject(j);
            l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, Some(fin_cb));
            "callee=userdata".into()
        }
    }
}

const NCALLEES: usize = JS_CALLEES.len() + 11;

/// Runs one (entry point, callee, argc) configuration and returns a transcript.
/// Pushes argument `kind` (a randomised mix of value tags).
unsafe fn push_arg(l: &Lib, j: JS, kind: u32, x: f64) {
    match kind % 8 {
        0 => l.js_pushnumber(j, x),
        1 => l.js_pushundefined(j),
        2 => l.js_pushnull(j),
        3 => l.js_pushboolean(j, (x != 0.0) as c_int),
        4 => l.js_pushstring(j, cn!("short")),
        5 => l.js_pushstring(j, cn!("a string well over fifteen bytes long")),
        6 => l.js_newobject(j),
        _ => l.js_newarray(j),
    }
}

fn call_case(l: &Lib, flags: c_int, ep: u8, k: usize, n: c_int, args: &[(u32, f64)]) -> String {
    unsafe {
        let j = new_state(l, flags);
        // sentinel: keeps js_pconstruct's savetop = TOP-n-2 inside the stack
        l.js_pushnumber(j, -777.0);
        let mut r = String::new();
        INNER_MODE.with(|m| m.set(if ep == 2 { 1 } else { 0 }));
        match ep {
            0 | 2 => {
                l.js_newcfunction(j, Some(cf_inner), N_INNER, 1);
                l.js_pushundefined(j);
                r.push_str(&push_callee(l, j, k));
                for i in 0..n {
                    let (kd, x) = args[i as usize % args.len()];
                    push_arg(l, j, kd, x);
                }
                let rc = l.js_pcall(j, n + 1);
                r.push_str(&format!(" rc={rc} {}", snap(l, j)));
            }
            1 => {
                r.push_str(&push_callee(l, j, k));
                l.js_pushundefined(j);
                for i in 0..n {
                    let (kd, x) = args[i as usize % args.len()];
                    push_arg(l, j, kd, x);
                }
                let rc = l.js_pcall(j, n);
                r.push_str(&format!(" rc={rc} {}", snap(l, j)));
            }
            _ => {
                r.push_str(&push_callee(l, j, k));
                for i in 0..n {
                    let (kd, x) = args[i as usize % args.len()];
                    push_arg(l, j, kd, x);
                }
                let rc = l.js_pconstruct(j, n);
                r.push_str(&format!(" rc={rc} {}", snap(l, j)));
            }
        }
        // fully drain and check the sentinel survived where expected
        let top = l.js_gettop(j);
        r.push_str(&format!(" finaltop={top} bottom={}", from_c(l.js_tryrepr(j, 0, ERRSTR))));
        l.js_pop(j, top);
        r.push_str(&format!(" drained={}", l.js_gettop(j)));
        l.js_freestate(j);
        r
    }
}

#[test]
fn t_call_matrix() {
    with_big_stack(body_t_call_matrix);
}

fn body_t_call_matrix() {
    let mut rng = Rng::new(0x0BAD_F00D);
    let argpool = [0.0, 1.0, -1.5, 2.0, 1e21, f64::NAN, 1234.5, -0.0];
    let mut seen_ok = 0usize;
    let mut seen_err = 0usize;
    let mut marks = std::collections::BTreeSet::new();
    for flags in [0, JS_STRICT] {
        for ep in 0u8..4 {
            for k in 0..NCALLEES {
                for n in [0, 1, 2, 3, 5, 7, 12, 33] {
                    let mut args = vec![];
                    for _ in 0..8 {
                        args.push((
                            rng.next_u32(),
                            argpool[rng.below(argpool.len() as u32) as usize],
                        ));
                    }
                    let a = args.clone();
                    let t = diff2(
                        &format!("call ep={ep} callee={k} n={n} flags={flags}"),
                        move |l| call_case(l, flags, ep, k, n, &a),
                    );
                    if t.contains(" rc=0 ") {
                        seen_ok += 1;
                    }
                    if t.contains(" rc=1 ") {
                        seen_err += 1;
                    }
                    for m in [
                        "[probe ", "[cc ", "[void ", "[three ", "[data=", "is not callable",
                        "probe-result", "script-ran",
                    ] {
                        if t.contains(m) {
                            marks.insert(m);
                        }
                    }
                }
            }
        }
    }
    // make sure the matrix really reached all the interesting shapes
    assert!(seen_ok > 100, "only {seen_ok} successful calls");
    assert!(seen_err > 100, "only {seen_err} throwing calls");
    assert_eq!(
        marks.len(),
        8,
        "the call matrix never reached: {:?}",
        [
            "[probe ", "[cc ", "[void ", "[three ", "[data=", "is not callable", "probe-result",
            "script-ran"
        ]
        .iter()
        .filter(|m| !marks.contains(*m))
        .collect::<Vec<_>>()
    );
}

/// `js_call(J, n)` with n < 0 raises js_rangeerror "number of arguments cannot
/// be negative", which is only observable through js_pcall.
///
/// UNDEFINED BEHAVIOUR, deliberately not tested: `js_pcall` (jsrun.c:1416) and
/// `js_pconstruct` (jsrun.c:1402) compute `savetop = TOP - n - 2` *before* `n`
/// is validated, and their error handlers then write `STACK[savetop]`. For
/// n <= -2 that slot is above TOP, so the handler publishes uninitialised
/// js_Values and TOP jumps by |n|+1; for a large positive n with a nearly empty
/// stack `savetop` goes negative and the handler writes *before* `J->stack`.
/// Both are out-of-bounds accesses in the C, so only n == -1 (savetop == TOP-1,
/// exactly the slot the error legitimately lands in) is driven here, and every
/// other test in this file keeps `savetop` inside the frame.
#[test]
fn t_call_negative_and_big_n() {
    for n in [-1] {
        diff2(&format!("pcall n={n}"), move |l| unsafe {
            let j = new_state(l, 0);
            for i in 0..8 {
                l.js_pushnumber(j, i as f64);
            }
            let rc = push_expr(l, j, "(function(){ return 'x' })");
            l.js_pushundefined(j);
            let rc2 = l.js_pcall(j, n);
            let r = format!("pushrc={rc} rc={rc2} {}", snap(l, j));
            let t = l.js_gettop(j);
            l.js_pop(j, t);
            l.js_freestate(j);
            r
        });
    }
    // n larger than the number of pushed arguments, but still with savetop
    // inside the frame: stackidx() clamps to the shared static `undefined`, so
    // `js_iscallable(J, -n-2)` is false and the typeerror path runs.
    for n in [1, 2, 3, 8, 20] {
        diff2(&format!("pcall short n={n}"), move |l| unsafe {
            let j = new_state(l, 0);
            // enough padding that savetop = TOP - n - 2 >= 0
            for i in 0..40 {
                l.js_pushnumber(j, i as f64);
            }
            let rc = l.js_pcall(j, n);
            let r = format!("rc={rc} {}", snap(l, j));
            let t = l.js_gettop(j);
            l.js_pop(j, t);
            l.js_freestate(j);
            r
        });
    }
    // and the same for js_pconstruct
    for n in [0, 1, 2, 3, 8, 20] {
        diff2(&format!("pconstruct short n={n}"), move |l| unsafe {
            let j = new_state(l, 0);
            for i in 0..40 {
                l.js_pushnumber(j, i as f64);
            }
            let rc = l.js_pconstruct(j, n);
            let r = format!("rc={rc} {}", snap(l, j));
            let t = l.js_gettop(j);
            l.js_pop(j, t);
            l.js_freestate(j);
            r
        });
    }
}

/* ====================================================================== */
/*  Rows 56, 79: js_currentfunction / js_currentfunctiondata               */
/* ====================================================================== */

#[test]
fn t_currentfunction() {
    diff2("currentfunction at top level", |l| unsafe {
        let j = new_state(l, 0);
        // BOT == 0: undefined is pushed, and the data pointer is NULL
        let d = l.js_currentfunctiondata(j);
        l.js_currentfunction(j);
        let r = format!(
            "data_null={} {} ",
            d.is_null(),
            snap(l, j)
        );
        l.js_pop(j, 1);
        l.js_freestate(j);
        r
    });

    for withfin in [false, true] {
        let t = diff2(&format!("cfunctionx data fin={withfin}"), move |l| unsafe {
            let j = new_state(l, 0);
            l.js_newcfunctionx(
                j,
                Some(cf_data),
                N_DATAF,
                1,
                PAYLOAD as *mut c_void,
                if withfin { Some(fin_cb) } else { None },
            );
            l.js_setglobal(j, N_DATAF);
            let cs = cstr("print(dataf(), dataf(1,2,3), dataf.length, typeof dataf.prototype)");
            let rc = l.js_dostring(j, cs.as_ptr());
            let r = format!("rc={rc} top={}", l.js_gettop(j));
            // js_gc must not free the still-reachable function
            l.js_gc(j, 0);
            l.js_delglobal(j, N_DATAF);
            l.js_gc(j, 0); // now unreachable -> finalizer runs (row 193)
            l.js_freestate(j); // and again for anything left (row 195)
            r
        });
        assert!(
            t.contains("[data=payload-A fn=function callable=1 same=1]"),
            "js_currentfunctiondata did not surface the data pointer: {t}"
        );
        assert_eq!(
            t.matches("[finalize payload-A]").count(),
            if withfin { 1 } else { 0 },
            "wrong number of finalizer runs (fin={withfin}): {t}"
        );
    }

    // js_newcfunctionx with data but no finalizer, called through js_construct
    diff2("cfunctionx construct", |l| unsafe {
        let j = new_state(l, 0);
        l.js_pushnumber(j, -1.0); // sentinel under the callee
        l.js_newcfunctionx(j, Some(cf_data), N_DATAF, 1, PAYLOAD as *mut c_void, None);
        let rc = l.js_pconstruct(j, 0);
        let r = format!("rc={rc} {}", snap(l, j));
        let t = l.js_gettop(j);
        l.js_pop(j, t);
        l.js_freestate(j);
        r
    });
}

/* ====================================================================== */
/*  Rows 81-84: js_newuserdata / js_newuserdatax hooks                     */
/* ====================================================================== */

#[test]
fn t_userdata_hooks() {
    let probe = "\
print(u.magic1, u.magic2, u.plain);\n\
u.rox = 1; u.keep = 2;\n\
print(u.keep, u.rox);\n\
print(delete u.delz, delete u.keep, delete u.other);\n\
print('magic3' in u, 'nope' in u);\n\
print(typeof u, u.magicX);\n";

    // (a) js_newuserdata: has/put/delete all NULL -> generic property tree
    for protoobj in [false, true] {
        diff2(&format!("newuserdata proto={protoobj}"), move |l| unsafe {
            let j = new_state(l, 0);
            if protoobj {
                l.js_newobject(j);
                l.js_pushnumber(j, 9.0);
                l.js_setproperty(j, -2, cn!("inherited"));
            } else {
                l.js_pushnumber(j, 7.0); // non-object -> prototype NULL
            }
            l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, Some(fin_cb));
            let isud = l.js_isuserdata(j, -1, N_TAG);
            let isother = l.js_isuserdata(j, -1, cn!("othertag"));
            l.js_setglobal(j, N_UD);
            let cs = cstr("print(u.magic1, u.plain, u.inherited); u.k=1; print(u.k, delete u.k)");
            let rc = l.js_dostring(j, cs.as_ptr());
            let r = format!("isud={isud} isother={isother} rc={rc} top={}", l.js_gettop(j));
            l.js_freestate(j);
            r
        });
    }

    // (b) js_newuserdatax with all four hooks
    let mut hookmarks: std::collections::BTreeSet<&str> = Default::default();
    for mask in 0..16u32 {
        let src = probe.to_string();
        let t = diff2(&format!("newuserdatax mask={mask}"), move |l| unsafe {
            let j = new_state(l, 0);
            l.js_newobject(j);
            l.js_newuserdatax(
                j,
                N_TAG,
                PAYLOAD as *mut c_void,
                if mask & 1 != 0 { Some(ud_has) } else { None },
                if mask & 2 != 0 { Some(ud_put) } else { None },
                if mask & 4 != 0 { Some(ud_del) } else { None },
                if mask & 8 != 0 { Some(fin_cb) } else { None },
            );
            l.js_setglobal(j, N_UD);
            let cs = cstr(&src);
            let rc = l.js_dostring(j, cs.as_ptr());
            let mut r = format!("rc={rc} top={}", l.js_gettop(j));
            // the C API side of the same hooks
            r.push_str(&format!(
                " api_has={} ",
                {
                    l.js_pushglobal(j);
                    let h = l.js_hasproperty(j, -1, cn!("magicAPI"));
                    let s = format!("{h}/{}", from_c(l.js_tryrepr(j, -1, ERRSTR)));
                    if h != 0 {
                        l.js_pop(j, 1);
                    }
                    l.js_pop(j, 1);
                    s
                }
            ));
            l.js_getglobal(j, N_UD);
            let h2 = l.js_hasproperty(j, -1, cn!("magicAPI"));
            if h2 != 0 {
                r.push_str(&format!(" hasval={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                l.js_pop(j, 1);
            }
            l.js_pushnumber(j, 3.0);
            l.js_setproperty(j, -2, cn!("roAPI"));
            l.js_delproperty(j, -1, cn!("delAPI"));
            l.js_pop(j, 1);
            r.push_str(&format!(" end={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
        for m in ["[has ", "[put ", "[del ", "[finalize "] {
            if t.contains(m) {
                hookmarks.insert(m);
            }
        }
        // a hook that was not installed must never appear
        for (bit, m) in [(1u32, "[has "), (2, "[put "), (4, "[del "), (8, "[finalize ")] {
            if mask & bit == 0 {
                assert!(!t.contains(m), "mask={mask} fired {m} anyway: {t}");
            } else {
                assert!(t.contains(m), "mask={mask} never fired {m}: {t}");
            }
        }
    }
    assert_eq!(hookmarks.len(), 4, "not every userdata hook fired: {hookmarks:?}");

    // (c) strict mode changes what a rejected write does
    for flags in [0, JS_STRICT] {
        let src = probe.to_string();
        diff2(&format!("userdatax strict flags={flags}"), move |l| unsafe {
            let j = new_state(l, flags);
            l.js_newobject(j);
            l.js_newuserdatax(
                j,
                N_TAG,
                PAYLOAD as *mut c_void,
                Some(ud_has),
                Some(ud_put),
                Some(ud_del),
                Some(fin_cb),
            );
            l.js_setglobal(j, N_UD);
            let cs = cstr(&src);
            let rc = l.js_dostring(j, cs.as_ptr());
            let r = format!("rc={rc} top={}", l.js_gettop(j));
            l.js_freestate(j);
            r
        });
    }
}

/// Row 79: the `js_newcfunctionx` / `js_newuserdatax` finalizer must run before
/// the exception is rethrown when construction itself throws.
#[test]
fn t_finalizer_on_construction_failure() {
    let mut rng = Rng::new(0x0F1E_2D3C);
    let mut lims: Vec<c_int> = (1..=80).collect();
    lims.extend([100, 160, 256, 1024, 1 << 16, 0]);
    for _ in 0..40 {
        lims.push(rng.below(4096) as c_int);
    }
    let mut nfin = 0usize;
    let mut nok = 0usize;
    for lim in lims {
        for which in 0..2 {
            let t = diff2(
                &format!("oom finalizer lim={lim} which={which}"),
                move |l| unsafe {
                    let j = new_state(l, 0);
                    FIN_LIMIT.with(|c| c.set(lim));
                    l.js_newcfunction(
                        j,
                        Some(if which == 0 {
                            cf_oom_cfunctionx
                        } else {
                            cf_oom_userdatax
                        }),
                        cn!("oom"),
                        0,
                    );
                    l.js_pushundefined(j);
                    let rc = l.js_pcall(j, 0);
                    // lift the limit again so the teardown is unconstrained
                    l.js_setlimit(j, 0, 0);
                    let r = format!("rc={rc} {}", safeview(l, j));
                    drain_to(l, j, 0);
                    l.js_freestate(j);
                    r
                },
            );
            if t.contains("rc=1") && t.contains("[finalize payload-A]") {
                nfin += 1;
            }
            if t.contains("rc=0") {
                nok += 1;
            }
        }
    }
    assert!(
        nfin > 10,
        "the construction-failure finalizer path never ran ({nfin})"
    );
    assert!(nok > 0, "no successful construction at all");
}

/* ====================================================================== */
/*  Rows 183-187: js_ref / js_unref / registry                             */
/* ====================================================================== */

#[test]
fn t_ref_fixed_names() {
    // Row 183: undefined / null / true / false get fixed names
    diff2("ref fixed names", |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for k in 0..6 {
            match k {
                0 => l.js_pushundefined(j),
                1 => l.js_pushnull(j),
                2 => l.js_pushboolean(j, 1),
                3 => l.js_pushboolean(j, 0),
                4 => l.js_pushboolean(j, 42), // normalised to !!v
                _ => l.js_pushboolean(j, -7),
            }
            let name = from_c(l.js_ref(j));
            // js_ref popped the value via js_setregistry
            l.js_getregistry(j, cstr(&name).as_ptr());
            r.push_str(&format!(
                "k={k} ref={name} back={} top={} | ",
                from_c(l.js_tryrepr(j, -1, ERRSTR)),
                l.js_gettop(j)
            ));
            l.js_pop(j, 1);
        }
        l.js_freestate(j);
        r
    });
}

#[test]
fn t_ref_objects() {
    // Row 184: "%p" of the object pointer, interned.  The pointer value itself
    // is not comparable between the two libraries, so compare the *structure*:
    // shape of the string, stability per object, and uniqueness across objects.
    let p = libs();
    for l in [&p.c, &p.rs] {
        unsafe {
            set_cur(l);
            let j = new_state(l, 0);
            let mut names = vec![];
            for i in 0..24 {
                if i % 3 == 0 {
                    l.js_newobject(j);
                } else if i % 3 == 1 {
                    l.js_newarray(j);
                } else {
                    l.js_newcfunction(j, Some(cf_void), N_VOIDF, 0);
                }
                l.nullary("js_dup", j);
                let a = from_c(l.js_ref(j));
                let b = from_c(l.js_ref(j)); // same object -> same ref
                assert_eq!(a, b, "{}: js_ref not stable for one object", l.name);
                assert!(
                    a.starts_with("0x") || a.starts_with("(nil)"),
                    "{}: js_ref(object) = {a:?} is not a %p rendering",
                    l.name
                );
                names.push(a);
            }
            let uniq: std::collections::BTreeSet<_> = names.iter().collect();
            assert_eq!(
                uniq.len(),
                names.len(),
                "{}: distinct objects shared a ref name",
                l.name
            );
            // and each one still resolves to an object in the registry
            for n in &names {
                let cs = cstr(n);
                l.js_getregistry(j, cs.as_ptr());
                assert_eq!(l.pred("js_isobject", j, -1), 1, "{}: {n} lost", l.name);
                l.js_pop(j, 1);
            }
            l.js_freestate(j);
        }
    }
}

#[test]
fn t_ref_sequential() {
    // Row 185: numbers and strings get J->nextref, so equal values still get
    // fresh (and byte-identical between the two libraries) names.
    let mut rng = Rng::new(0xBEEF_1234);
    let mut vals: Vec<String> = vec![];
    for _ in 0..40 {
        vals.push(rng.ascii_string(20));
    }
    diff2("ref sequential", move |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for i in 0..12 {
            l.js_pushnumber(j, 7.0); // same value every time
            r.push_str(&from_c(l.js_ref(j)));
            r.push(',');
            let _ = i;
        }
        for v in &vals {
            let cs = cstr(v);
            l.js_pushstring(j, cs.as_ptr());
            let n = from_c(l.js_ref(j));
            let cn = cstr(&n);
            l.js_getregistry(j, cn.as_ptr());
            r.push_str(&format!(
                "{n}={} ",
                from_c(l.js_tryrepr(j, -1, ERRSTR))
            ));
            l.js_pop(j, 1);
        }
        r.push_str(&format!(" top={}", l.js_gettop(j)));
        l.js_freestate(j);
        r
    });
}

/// `J->nextref` is a plain `int` formatted with `"%d"` and then interned, so a
/// long run of `js_ref` calls also exercises the intern table's growth.
#[test]
fn t_ref_many() {
    for n in [64usize, 1000, 5000] {
        diff2(&format!("ref many n={n}"), move |l| unsafe {
            let j = new_state(l, 0);
            let mut first = String::new();
            let mut last = String::new();
            let mut lastraw = String::new();
            for i in 0..n {
                if i % 5 == 4 {
                    l.js_newobject(j);
                } else {
                    l.js_pushnumber(j, (i % 7) as f64);
                }
                let name = from_c(l.js_ref(j));
                if i == 0 {
                    first = norm_ref(&name);
                }
                lastraw = name.clone();
                last = norm_ref(&name);
            }
            let mut r = format!("first={first} last={last} top={}", l.js_gettop(j));
            l.js_gc(j, 1);
            // every ref is still reachable from J->R
            l.js_getregistry(j, cstr(&lastraw).as_ptr());
            r.push_str(&format!(" back={}", from_c(l.js_typeof(j, -1))));
            l.js_pop(j, 1);
            l.js_freestate(j);
            r
        });
    }
}

#[test]
fn t_registry_roundtrip() {
    // Rows 186, 187
    let mut rng = Rng::new(0x1357_9BDF);
    let mut names: Vec<String> = vec![
        "a".into(),
        "".into(),
        "_Undefined".into(),
        "0".into(),
        "a very long registry key name indeed".into(),
    ];
    for _ in 0..40 {
        let s = rng.ascii_string(18);
        if !s.is_empty() {
            names.push(s);
        }
    }
    for _ in 0..40 {
        let s = rng.unicode_string(10);
        if !s.is_empty() {
            names.push(s);
        }
    }
    names.push("x".repeat(300));
    names.push("\u{10FFFF}\u{7f}\u{80}\u{7ff}\u{800}".to_string());
    diff2("registry roundtrip", move |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for (i, n) in names.iter().enumerate() {
            let cs = cstr(n);
            // get before set -> undefined
            l.js_getregistry(j, cs.as_ptr());
            r.push_str(&format!("pre={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
            // delregistry on a name never set is a no-op
            l.js_delregistry(j, cs.as_ptr());
            // setregistry consumes its value
            match i % 4 {
                0 => l.js_pushnumber(j, i as f64),
                1 => l.js_pushstring(j, cs.as_ptr()),
                2 => l.js_newobject(j),
                _ => l.js_pushboolean(j, (i % 8 > 3) as c_int),
            }
            let before = l.js_gettop(j);
            l.js_setregistry(j, cs.as_ptr());
            r.push_str(&format!("popped={} ", before - l.js_gettop(j)));
            l.js_getregistry(j, cs.as_ptr());
            r.push_str(&format!("v={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
            // js_unref delegates to js_delregistry
            l.js_unref(j, cs.as_ptr());
            l.js_getregistry(j, cs.as_ptr());
            r.push_str(&format!("post={} | ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
        }
        r.push_str(&format!("top={}", l.js_gettop(j)));
        l.js_freestate(j);
        r
    });

    // js_ref -> js_unref -> js_getregistry round-trip through J->R
    diff2("ref/unref roundtrip", |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for k in 0..8 {
            match k % 4 {
                0 => l.js_pushnumber(j, k as f64),
                1 => l.js_pushstring(j, cn!("value")),
                2 => l.js_pushundefined(j),
                _ => l.js_newobject(j),
            }
            let name = from_c(l.js_ref(j));
            let cs = cstr(&name);
            l.js_getregistry(j, cs.as_ptr());
            let a = from_c(l.js_typeof(j, -1));
            l.js_pop(j, 1);
            l.js_unref(j, cs.as_ptr());
            l.js_getregistry(j, cs.as_ptr());
            let b = from_c(l.js_tryrepr(j, -1, ERRSTR));
            l.js_pop(j, 1);
            r.push_str(&format!("k={k} kind={a} after_unref={b} | "));
        }
        r.push_str(&format!("top={}", l.js_gettop(j)));
        l.js_freestate(j);
        r
    });
}

/* ====================================================================== */
/*  Rows 188, 189: globals                                                 */
/* ====================================================================== */

#[test]
fn t_globals() {
    let mut rng = Rng::new(0x2468_ACE0);
    let mut names: Vec<String> = vec![
        "gv".into(),
        "NaN".into(),
        "Infinity".into(),
        "undefined".into(),
        "parseInt".into(),
        "Object".into(),
        "".into(),
        "never-set-name".into(),
    ];
    for _ in 0..30 {
        names.push(format!("g{}", rng.below(1_000_000)));
    }
    // index-like names: J->G is a JS_COBJECT so there is no array fast path,
    // but js_isarrayindex still runs over them inside jsV_setproperty
    for nm in ["0", "1", "00", "01", "10", "4294967295", "4294967296", "-1", "1e3", "0.5"] {
        names.push(nm.to_string());
    }
    for _ in 0..30 {
        let s = rng.unicode_string(8);
        if !s.is_empty() {
            names.push(s);
        }
    }
    for _ in 0..30 {
        let s = rng.ascii_string(20);
        if !s.is_empty() {
            names.push(s);
        }
    }
    names.push("g".repeat(400));
    diff2("get/set/delglobal", move |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();
        for (i, n) in names.iter().enumerate() {
            let cs = cstr(n);
            l.js_getglobal(j, cs.as_ptr());
            r.push_str(&format!(
                "{n}: pre={} ",
                from_c(l.js_tryrepr(j, -1, ERRSTR))
            ));
            l.js_pop(j, 1);
            match i % 3 {
                0 => l.js_pushnumber(j, i as f64),
                1 => l.js_pushstring(j, cs.as_ptr()),
                _ => l.js_newarray(j),
            }
            let before = l.js_gettop(j);
            l.js_setglobal(j, cs.as_ptr());
            r.push_str(&format!("popped={} ", before - l.js_gettop(j)));
            l.js_getglobal(j, cs.as_ptr());
            r.push_str(&format!("v={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
            l.js_delglobal(j, cs.as_ptr());
            l.js_getglobal(j, cs.as_ptr());
            r.push_str(&format!("post={} | ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            l.js_pop(j, 1);
        }
        r.push_str(&format!("top={}", l.js_gettop(j)));
        l.js_freestate(j);
        r
    });
}

#[test]
fn t_defglobal_attributes() {
    // Row 189: every combination of JS_READONLY | JS_DONTENUM | JS_DONTCONF
    let observe = "\
var found = false;\n\
for (var k in this) if (k === 'gv') found = true;\n\
print('enum', found);\n\
print('value', gv, typeof gv);\n\
gv = 99;\n\
print('afterwrite', gv);\n\
print('delete', delete gv);\n\
print('afterdelete', typeof gv, gv);\n\
print('own', Object.getOwnPropertyDescriptor ? 'yes' : 'no');\n";
    for atts in 0..8 {
        for flags in [0, JS_STRICT] {
            let obs = observe.to_string();
            diff2(&format!("defglobal atts={atts} flags={flags}"), move |l| unsafe {
                let j = new_state(l, flags);
                l.js_pushnumber(j, 7.0);
                let before = l.js_gettop(j);
                l.js_defglobal(j, N_GV, atts);
                let popped = before - l.js_gettop(j);
                let cs = cstr(&obs);
                let rc = l.js_dostring(j, cs.as_ptr());
                let mut r = format!("popped={popped} rc={rc} top={}", l.js_gettop(j));
                // Re-defining with different attributes. jsR_defproperty raises
                // a typeerror for a JS_DONTCONF property, so this has to run
                // inside a protected frame.
                REDEF_ATTS.with(|c| c.set(atts ^ 7));
                l.js_newcfunction(j, Some(cf_redef), cn!("redef"), 0);
                l.js_pushundefined(j);
                let rc2 = l.js_pcall(j, 0);
                r.push_str(&format!(
                    " redef_rc={rc2} redef={}",
                    from_c(l.js_tryrepr(j, -1, ERRSTR))
                ));
                l.js_pop(j, 1);
                l.js_getglobal(j, N_GV);
                r.push_str(&format!(" after={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                l.js_pop(j, 1);
                // the C-API write path: js_setglobal on a JS_READONLY global
                // throws in strict mode, so go through a protected frame again
                l.js_newcfunction(j, Some(cf_setgv), cn!("setgv"), 0);
                l.js_pushundefined(j);
                let rc3 = l.js_pcall(j, 0);
                r.push_str(&format!(
                    " set_rc={rc3} set={}",
                    from_c(l.js_tryrepr(j, -1, ERRSTR))
                ));
                l.js_pop(j, 1);
                // and js_delglobal, which also throws in strict mode
                l.js_newcfunction(j, Some(cf_delgv), cn!("delgv"), 0);
                l.js_pushundefined(j);
                let rc4 = l.js_pcall(j, 0);
                r.push_str(&format!(
                    " del_rc={rc4} del={}",
                    from_c(l.js_tryrepr(j, -1, ERRSTR))
                ));
                l.js_pop(j, 1);
                let t = l.js_gettop(j);
                r.push_str(&format!(" finaltop={t}"));
                l.js_pop(j, t);
                l.js_freestate(j);
                r
            });
        }
    }
}

/* ====================================================================== */
/*  Rows 190-195: js_gc                                                    */
/* ====================================================================== */

#[test]
fn t_gc_report() {
    // Rows 190, 191, 192: report 0 vs 1, repeated collections, live vs dead
    let scripts = [
        "1+1",
        "var a = {}; a.b = {c:{d:1}}; 1",
        "var arr = []; for (var i=0;i<100;++i) arr.push({i:i}); arr.length",
        "(function(){ var s=''; for (var i=0;i<50;++i) s+='xyzzy'+i; return s.length })()",
        "var f = function(){ return function(){ return 1 } }; f()()",
        "for (var i=0;i<50;++i) ({dead:i}); 0",
        "var re = /a(b)c/g; re.exec('abcabc'); 0",
        "var d = new Date(0); d.getTime()",
        "JSON.parse('{\"a\":[1,2,3]}').a.length",
    ];
    let mut saw_report = 0usize;
    for src in scripts {
        let t = diff2(&format!("gc report {src:?}"), move |l| unsafe {
            let j = new_state(l, 0);
            let mut r = String::new();
            // silent collection first (row 190)
            l.js_gc(j, 0);
            l.js_gc(j, 0);
            let cs = cstr(src);
            let rc = l.js_dostring(j, cs.as_ptr());
            r.push_str(&format!("rc={rc} "));
            // reporting collections; gcmark alternates 1 -> 2 -> 1 (row 192)
            for _ in 0..5 {
                l.js_gc(j, 1);
            }
            l.js_gc(j, 0);
            l.js_gc(j, 1);
            // a second script run reuses the grown gcthresh
            let rc2 = l.js_dostring(j, cs.as_ptr());
            r.push_str(&format!("rc2={rc2} "));
            l.js_gc(j, 1);
            r.push_str(&format!("top={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
        saw_report += t.matches("[report] garbage collected (").count();
        assert!(
            t.contains("[report] garbage collected ("),
            "js_gc(J,1) produced no summary for {src:?}: {t}"
        );
        assert!(
            t.contains("envs,") && t.contains("funs,") && t.contains("objs,")
                && t.contains("props,") && t.contains("strs"),
            "unexpected gc summary shape for {src:?}: {t}"
        );
    }
    assert!(saw_report >= 7 * scripts.len(), "only {saw_report} summaries");
}

#[test]
fn t_gc_many_objects() {
    // gcthresh growth: create far more objects than the initial threshold
    for n in [10, 100, 1000, 5000] {
        diff2(&format!("gc growth n={n}"), move |l| unsafe {
            let j = new_state(l, 0);
            let src = format!(
                "var keep=[]; for (var i=0;i<{n};++i) {{ keep.push({{i:i,s:'value'+i}}); ({{dead:i}}); }} keep.length"
            );
            let cs = cstr(&src);
            let rc = l.js_dostring(j, cs.as_ptr());
            let mut r = format!("rc={rc} ");
            l.js_gc(j, 1);
            l.js_gc(j, 1);
            let cs2 = cstr("keep = null; 0");
            let rc2 = l.js_dostring(j, cs2.as_ptr());
            r.push_str(&format!("rc2={rc2} "));
            l.js_gc(j, 1);
            l.js_gc(j, 1);
            r.push_str(&format!("top={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
    }
}

#[test]
fn t_gc_finalizers() {
    // Row 193: unreachable JS_CUSERDATA / js_newcfunctionx run their finalizers
    // from jsG_freeobject during the sweep; row 195: js_freestate runs the rest.
    for keep in [false, true] {
        diff2(&format!("gc finalizers keep={keep}"), move |l| unsafe {
            let j = new_state(l, 0);
            let mut r = String::new();
            for i in 0..6 {
                l.js_newobject(j);
                l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, Some(fin_cb));
                if keep && i % 2 == 0 {
                    l.js_setglobal(j, N_UD);
                } else {
                    l.js_pop(j, 1);
                }
                l.js_newcfunctionx(
                    j,
                    Some(cf_void),
                    N_VOIDF,
                    0,
                    PAYLOAD as *mut c_void,
                    Some(fin_cb),
                );
                if keep && i % 2 == 1 {
                    l.js_setglobal(j, N_VOIDF);
                } else {
                    l.js_pop(j, 1);
                }
            }
            out_push(b"--gc1--\n");
            l.js_gc(j, 1);
            out_push(b"--gc2--\n");
            l.js_gc(j, 1);
            out_push(b"--free--\n");
            r.push_str(&format!("top={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
    }
}

#[test]
fn t_gc_roots() {
    // Row 194: values reachable only from the value stack, only from the
    // environments, or only from J->R / J->G
    diff2("gc roots", |l| unsafe {
        let j = new_state(l, 0);
        let mut r = String::new();

        // (a) only from the value stack
        l.js_newobject(j);
        l.js_pushstring(j, cn!("stack-only-string-value"));
        l.js_setproperty(j, -2, N_X);
        l.js_gc(j, 1);
        l.js_getproperty(j, -1, N_X);
        r.push_str(&format!("stack={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
        l.js_pop(j, 1);

        // (b) only from J->R
        l.js_newobject(j);
        l.js_pushstring(j, cn!("registry-only-string-value"));
        l.js_setproperty(j, -2, N_X);
        l.js_setregistry(j, cn!("keeper"));
        l.js_gc(j, 1);
        l.js_getregistry(j, cn!("keeper"));
        l.js_getproperty(j, -1, N_X);
        r.push_str(&format!("registry={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
        l.js_pop(j, 2);

        // (c) only from J->G
        l.js_newobject(j);
        l.js_pushstring(j, cn!("global-only-string-value"));
        l.js_setproperty(j, -2, N_X);
        l.js_setglobal(j, cn!("keeper2"));
        l.js_gc(j, 1);
        l.js_getglobal(j, cn!("keeper2"));
        l.js_getproperty(j, -1, N_X);
        r.push_str(&format!("global={} ", from_c(l.js_tryrepr(j, -1, ERRSTR))));
        l.js_pop(j, 2);

        // (d) only from J->E / GE (a var binding), collected mid-run
        let cs = cstr(
            "var envkeep = {s:'env-only-string-value'};\n\
             (function(){ var inner = {t:'closure-only'}; return function(){ return inner.t } })();\n\
             envkeep.s",
        );
        let rc = l.js_dostring(j, cs.as_ptr());
        l.js_gc(j, 1);
        l.js_gc(j, 1);
        let cs2 = cstr("print(envkeep.s)");
        let rc2 = l.js_dostring(j, cs2.as_ptr());
        r.push_str(&format!("rc={rc} rc2={rc2} top={} ", l.js_gettop(j)));

        // (e) after dropping every root the objects go away
        l.js_delregistry(j, cn!("keeper"));
        l.js_delglobal(j, cn!("keeper2"));
        l.js_pop(j, l.js_gettop(j));
        l.js_gc(j, 1);
        l.js_gc(j, 1);
        r.push_str(&format!("end={}", l.js_gettop(j)));
        l.js_freestate(j);
        r
    });
}

/// A randomised walk over the operations that move objects in and out of the
/// GC roots, comparing the exact `js_gc(J, 1)` summary (env / fun / obj / prop /
/// str counts and the percentage) after every step.  This is the sharpest check
/// on the GC bookkeeping: any difference in how many objects, properties or
/// interned strings the two implementations create shows up immediately.
#[test]
fn t_gc_random_walk() {
    const WNAMES: [*const c_char; 8] = [
        cn!("w0"),
        cn!("w1"),
        cn!("w2"),
        cn!("w3"),
        cn!("w4"),
        cn!("w5"),
        cn!("w6"),
        cn!("w7"),
    ];
    const WSRC: [&str; 8] = [
        "var q = {a:1,b:[1,2,3]}; 0",
        "(function(){ var c = {x:'y'}; return function(){ return c.x } })()",
        "var s=''; for (var i=0;i<20;++i) s += 'chunk'+i; s.length",
        "var re = /a(b+)c/g; re.exec('abbbc'); 0",
        "for (var i=0;i<30;++i) ({dead:i}); 0",
        "q = null; 0",
        "JSON.stringify({z:[1,2,3,'four']})",
        "new Date(0).getTime()",
    ];
    let mut rng = Rng::new(0xFEED_FACE);
    let mut nreports = 0usize;
    for round in 0..24 {
        let ops: Vec<u32> = (0..48).map(|_| rng.next_u32()).collect();
        let t = diff2(&format!("gc walk {round}"), move |l| unsafe {
            let j = new_state(l, 0);
            let mut nstack = 0i32;
            let mut r = String::new();
            for (i, op) in ops.iter().enumerate() {
                let name = WNAMES[i % WNAMES.len()];
                match op % 14 {
                    0 => {
                        l.js_newobject(j);
                        nstack += 1;
                    }
                    1 => {
                        l.js_newarray(j);
                        nstack += 1;
                    }
                    2 => {
                        l.js_pushstring(j, cn!("a heap string longer than fifteen bytes"));
                        nstack += 1;
                    }
                    3 => {
                        l.js_pushnumber(j, i as f64);
                        nstack += 1;
                    }
                    4 => {
                        if nstack > 0 {
                            l.js_pop(j, 1);
                            nstack -= 1;
                        }
                    }
                    5 => l.js_gc(j, 1),
                    6 => l.js_gc(j, 0),
                    7 => {
                        let cs = cstr(WSRC[(op / 14) as usize % WSRC.len()]);
                        let rc = l.js_dostring(j, cs.as_ptr());
                        r.push_str(&format!("do{i}={rc} "));
                    }
                    8 => {
                        if nstack > 0 {
                            l.js_setglobal(j, name);
                            nstack -= 1;
                        }
                    }
                    9 => l.js_delglobal(j, name),
                    10 => {
                        if nstack > 0 {
                            l.js_setregistry(j, name);
                            nstack -= 1;
                        }
                    }
                    11 => l.js_delregistry(j, name),
                    12 => {
                        l.js_newobject(j);
                        l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, Some(fin_cb));
                        nstack += 1;
                    }
                    _ => {
                        l.js_newcfunctionx(
                            j,
                            Some(cf_void),
                            N_VOIDF,
                            0,
                            PAYLOAD as *mut c_void,
                            Some(fin_cb),
                        );
                        nstack += 1;
                    }
                }
                r.push_str(&format!("{}:{} ", i, l.js_gettop(j)));
            }
            drain_to(l, j, 0);
            l.js_gc(j, 1);
            l.js_gc(j, 1);
            r.push_str(&format!("end={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
        nreports += t.matches("garbage collected (").count();
    }
    assert!(nreports > 100, "only {nreports} gc summaries in the walk");
}

/// The report hook must receive the message pointer verbatim, whatever bytes it
/// holds (`js_defaultreport` only does `fputs`).
#[test]
fn t_report_raw_bytes() {
    let p = libs();
    let mut rng = Rng::new(0x00FF_1234);
    for _ in 0..60 {
        let mut msg = rng.raw_bytes(60);
        msg.push(0);
        let mut got = vec![];
        for l in [&p.c, &p.rs] {
            out_clear();
            unsafe {
                set_cur(l);
                let j = new_state(l, 0);
                l.js_report(j, msg.as_ptr() as *const c_char);
                l.js_setreport(j, None);
                l.js_report(j, msg.as_ptr() as *const c_char);
                l.js_freestate(j);
            }
            got.push(out_take());
        }
        assert_eq!(got[0], got[1], "raw report divergence for {msg:?}");
        // exactly one delivery: the second js_report is a no-op
        assert_eq!(got[0].matches("[report] ").count(), 1, "{:?}", got[0]);
    }
}

/* ====================================================================== */
/*  Rows 20, 196, 197: the try stack                                       */
/* ====================================================================== */

#[test]
fn t_trylimit_savetry() {
    // Row 196: js_savetry at depth 0..JS_TRYLIMIT-1 is fine.  We never push a
    // 65th frame (that would throw into a jmp_buf we never setjmp'd), we go to
    // exactly 64 and then use the protected entry points, whose js_ptry guard
    // returns without throwing (rows 20 and the js_dostring/js_trystring twins).
    for depth in [0, 1, 2, 32, 62, 63, 64] {
        diff2(&format!("savetry depth={depth}"), move |l| unsafe {
            let j = new_state(l, 0);
            for _ in 0..depth {
                let b = l.js_savetry(j);
                assert!(!b.is_null(), "{}: js_savetry returned NULL", l.name);
            }
            let mut r = String::new();
            let good = cstr("1+2*3");
            let bad = cstr("this is ((( not js");

            let base = l.js_gettop(j);
            let rc = l.js_ploadstring(j, FILENAME, good.as_ptr());
            r.push_str(&format!("load_good={rc} {} ", safeview(l, j)));
            drain_to(l, j, base);

            let rc = l.js_ploadstring(j, FILENAME, bad.as_ptr());
            r.push_str(&format!("load_bad={rc} {} ", safeview(l, j)));
            drain_to(l, j, base);

            let rc = l.js_dostring(j, good.as_ptr());
            r.push_str(&format!("do_good={rc} top={} ", l.js_gettop(j)));
            drain_to(l, j, base);
            let rc = l.js_dostring(j, bad.as_ptr());
            r.push_str(&format!("do_bad={rc} top={} ", l.js_gettop(j)));
            drain_to(l, j, base);

            // js_trystring / js_trynumber / js_tryinteger / js_tryboolean all
            // carry the js_ptry guard, and that guard *pops* one value before
            // returning the caller's default.
            l.js_newobject(j);
            let s = from_c(l.js_trystring(j, -1, cn!("DEFAULT")));
            r.push_str(&format!("trystring={s} d={} ", l.js_gettop(j) - base));
            drain_to(l, j, base);

            l.js_newobject(j);
            let n = l.js_trynumber(j, -1, -1.0);
            r.push_str(&format!(
                "trynumber={} d={} ",
                fbits(n),
                l.js_gettop(j) - base
            ));
            drain_to(l, j, base);

            l.js_newobject(j);
            let i = l.js_tryinteger(j, -1, -1);
            r.push_str(&format!("tryint={i} d={} ", l.js_gettop(j) - base));
            drain_to(l, j, base);

            l.js_newobject(j);
            let b = l.js_tryboolean(j, -1, -1);
            r.push_str(&format!("trybool={b} d={} ", l.js_gettop(j) - base));
            drain_to(l, j, base);

            r.push_str(&format!("top={} ", l.js_gettop(j)));
            for _ in 0..depth {
                l.js_endtry(j);
            }
            // and everything works again once the frames are gone
            let rc = l.js_dostring(j, good.as_ptr());
            r.push_str(&format!("after={rc} top={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
    }
}

#[test]
fn t_trylimit_nested_js() {
    with_big_stack(body_t_trylimit_nested_js);
}

fn body_t_trylimit_nested_js() {
    // Row 196 second half: js_savetrypc at trytop == JS_TRYLIMIT calls
    // js_trystackoverflow, which pushes the bare literal
    // "exception stack overflow" and throws into the enclosing frame.
    for n in [1usize, 2, 10, 32, 60, 62, 63, 64, 65, 66, 70, 80, 120, 200] {
        let t = diff2(&format!("nested try n={n}"), move |l| unsafe {
            let j = new_state(l, 0);
            let mut src = String::new();
            for _ in 0..n {
                src.push_str("try { ");
            }
            src.push_str("print('inner reached')");
            for i in 0..n {
                src.push_str(&format!(" }} catch (e{i}) {{ print('catch {i}', e{i}) }}"));
            }
            let cs = cstr(&src);
            let rc = l.js_dostring(j, cs.as_ptr());
            let r = format!("rc={rc} top={}", l.js_gettop(j));
            l.js_freestate(j);
            r
        });
        // js_dostring already holds one try frame, so the script itself only
        // has JS_TRYLIMIT-1 = 63 left before js_savetrypc hits the limit.
        if n <= 63 {
            assert!(
                t.contains("inner reached") && !t.contains("exception stack overflow"),
                "n={n} should stay under the try limit: {t}"
            );
        } else {
            assert!(
                t.contains("exception stack overflow"),
                "n={n} should have overflowed the try stack: {t}"
            );
        }
    }
}

#[test]
fn t_trylimit_nested_pcall() {
    with_big_stack(body_t_trylimit_nested_pcall);
}

fn body_t_trylimit_nested_pcall() {
    // Each js_pcall level adds one try frame, so at level 65 js_savetry hits the
    // limit and throws into level 64.  Reaching exactly 64 lets js_ploadstring /
    // js_dostring see trytop == JS_TRYLIMIT (rows 20, 196).
    for target in [1i32, 2, 32, 63, 64, 65, 70] {
        let t = diff2(&format!("nested pcall target={target}"), move |l| unsafe {
            let j = new_state(l, 0);
            REC_TARGET.with(|c| c.set(target));
            REC_DEPTH.with(|c| c.set(0));
            l.js_newcfunction(j, Some(cf_rec), N_REC, 0);
            l.js_setglobal(j, N_REC);
            l.js_getglobal(j, N_REC);
            l.js_pushundefined(j);
            let rc = l.js_pcall(j, 0);
            let r = format!(
                "rc={rc} {} depth_left={}",
                snap(l, j),
                REC_DEPTH.with(|c| c.get())
            );
            l.js_pop(j, 1);
            l.js_freestate(j);
            r
        });
        if target <= 64 {
            assert!(
                t.contains(&format!("at depth {target}: ploadstring")),
                "target={target} never reached the innermost frame: {t}"
            );
        }
        if target == 64 {
            // js_ptry fires: rc=1 and the bare literal is pushed
            assert!(
                t.contains("ploadstring rc=1") && t.contains("exception stack overflow"),
                "target=64 should hit the js_ptry guard: {t}"
            );
        }
        if target == 65 || target == 70 {
            assert!(
                t.contains("exception stack overflow"),
                "target={target} should overflow the try stack: {t}"
            );
        }
    }
}

/// A randomised walk over `js_savetry` / `js_endtry` interleaved with the
/// protected entry points.  The walk never asks for a 65th frame (no `setjmp`
/// stands behind these frames) and never calls `js_endtry` at depth 0 (that
/// raises js_error "endtry: exception stack underflow" with `trytop == 0`, which
/// would panic and abort).
#[test]
fn t_savetry_random_walk() {
    let mut rng = Rng::new(0x7175_5259);
    for round in 0..40 {
        let ops: Vec<u32> = (0..90).map(|_| rng.next_u32()).collect();
        diff2(&format!("savetry walk {round}"), move |l| unsafe {
            let j = new_state(l, 0);
            let good = cstr("var wv = 1; wv + 2");
            let bad = cstr("nope ((( nope");
            let mut depth = 0i32;
            let mut r = String::new();
            for (i, op) in ops.iter().enumerate() {
                let base = l.js_gettop(j);
                match op % 10 {
                    0 => {
                        if depth < 64 {
                            let b = l.js_savetry(j);
                            assert!(!b.is_null());
                            depth += 1;
                        }
                    }
                    1 => {
                        if depth > 0 {
                            l.js_endtry(j);
                            depth -= 1;
                        }
                    }
                    2 => {
                        let rc = l.js_ploadstring(j, FILENAME, good.as_ptr());
                        r.push_str(&format!("{i}:lg={rc},{} ", safeview(l, j)));
                        drain_to(l, j, base);
                    }
                    3 => {
                        let rc = l.js_ploadstring(j, FILENAME, bad.as_ptr());
                        r.push_str(&format!("{i}:lb={rc},{} ", safeview(l, j)));
                        drain_to(l, j, base);
                    }
                    4 => {
                        let rc = l.js_dostring(j, good.as_ptr());
                        r.push_str(&format!("{i}:dg={rc},{} ", l.js_gettop(j) - base));
                        drain_to(l, j, base);
                    }
                    5 => {
                        let rc = l.js_dostring(j, bad.as_ptr());
                        r.push_str(&format!("{i}:db={rc},{} ", l.js_gettop(j) - base));
                        drain_to(l, j, base);
                    }
                    6 => {
                        l.js_newobject(j);
                        let v = from_c(l.js_trystring(j, -1, cn!("DEF")));
                        r.push_str(&format!("{i}:ts={v},{} ", l.js_gettop(j) - base));
                        drain_to(l, j, base);
                    }
                    7 => {
                        l.js_newarray(j);
                        let v = l.js_tryinteger(j, -1, -9);
                        r.push_str(&format!("{i}:ti={v},{} ", l.js_gettop(j) - base));
                        drain_to(l, j, base);
                    }
                    8 => l.js_gc(j, 1),
                    _ => {
                        l.js_pushstring(j, cn!("a value that lives on the stack"));
                        drain_to(l, j, base);
                    }
                }
                r.push_str(&format!("d{depth} "));
            }
            for _ in 0..depth {
                l.js_endtry(j);
            }
            drain_to(l, j, 0);
            let rc = l.js_dostring(j, good.as_ptr());
            r.push_str(&format!("final={rc} top={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
    }
}

/// Deep native recursion: the trace stack (`JS_ENVLIMIT` = 1024) and the try
/// stack (`JS_TRYLIMIT` = 64) run out at very different depths, and both errors
/// have to be reported identically.
#[test]
fn t_deep_recursion() {
    with_big_stack(body_t_deep_recursion);
}

fn body_t_deep_recursion() {
    let srcs = [
        // bounded recursion, well under every limit
        "function r(n){ return n<=0 ? 0 : r(n-1)+1 } print(r(100))",
        "function r(n){ return n<=0 ? 0 : r(n-1)+1 } print(r(500))",
        // unbounded recursion -> "call stack overflow" from jsR_pushtrace
        "function r(n){ return r(n+1) } try { r(0) } catch (e) { print('caught', e) }",
        // one try frame per level -> "exception stack overflow" first
        "function r(n){ try { return r(n+1) } catch (e) { return 'c'+n+':'+e } } print(r(0))",
        "function r(n){ try { return r(n+1) } finally { } } try { print(r(0)) } catch (e) { print('caught', e) }",
        // and mutual recursion through a cfunction boundary
        "function r(n){ return n<=0 ? [] : r(n-1).concat([n]) } print(r(60).length)",
        // deep expression nesting inside a try
        "try { print(eval('1' + '+1'.repeat(200))) } catch (e) { print('caught', e) }",
    ];
    for flags in [0, JS_STRICT] {
        for src in srcs {
            diff_dostring(flags, src);
            diff_eval(flags, src);
        }
    }
    // the same, but with the recursion happening under N pre-existing try frames
    for depth in [0, 30, 60, 63, 64] {
        diff2(&format!("deep under {depth} frames"), move |l| unsafe {
            let j = new_state(l, 0);
            for _ in 0..depth {
                l.js_savetry(j);
            }
            let cs = cstr("function r(n){ try { return r(n+1) } catch (e) { return 'c'+n } } r(0)");
            let rc = l.js_dostring(j, cs.as_ptr());
            let mut r = format!("rc={rc} top={}", l.js_gettop(j));
            drain_to(l, j, 0);
            for _ in 0..depth {
                l.js_endtry(j);
            }
            let rc2 = l.js_dostring(j, cs.as_ptr());
            r.push_str(&format!(" rc2={rc2} top={}", l.js_gettop(j)));
            l.js_freestate(j);
            r
        });
    }
}

#[test]
fn t_throw_restores_strict() {
    // Row 197: js_throw restores strict / top / bot / E / envtop / tracetop from
    // the frame, so a strict-mode switch made by the aborted call is rolled back
    let srcs = [
        "function f(){ 'use strict'; throw new Error('x') }\n\
         try { f() } catch (e) { print('caught', e) }\n\
         zz = 1; print('nonstrict still works', zz)",
        "function f(){ 'use strict'; undeclared_in_strict = 1 }\n\
         try { f() } catch (e) { print('caught', e) }\n\
         zz2 = 2; print(zz2)",
        "function g(){ 'use strict'; return (function(){ throw 'deep' })() }\n\
         try { g() } catch (e) { print('caught', e) }\n\
         print(typeof this, (function(){ return typeof this })())",
        "try { (function(){ 'use strict'; (function(){ 'use strict'; throw 1 })() })() }\n\
         catch (e) { print('c', e) }\n\
         zz3 = 3; print(zz3)",
        "var log=[];\n\
         function h(n){ log.push(n); if (n>3) { 'use strict'; throw new Error('deep'+n) } return h(n+1) }\n\
         try { h(0) } catch (e) { print('caught', e) }\n\
         print(log.join(','));\n\
         zz4 = 4; print(zz4)",
    ];
    for flags in [0, JS_STRICT] {
        for src in srcs {
            diff_dostring(flags, src);
            diff_eval(flags, src);
        }
    }
}

/* ====================================================================== */
/*  Row 195: js_freestate                                                  */
/* ====================================================================== */

#[test]
fn t_freestate_runs_everything() {
    let p = libs();
    let scripts = [
        "1+1",
        "var a=[]; for (var i=0;i<500;++i) a.push({i:i,s:'value'+i}); a.length",
        "var re=/(a+)(b+)/g; re.exec('aaabbb'); 0",
        "var s = new String('a string that is definitely longer than fifteen bytes'); s.length",
        "var it = {a:1,b:2}; var n=0; for (var k in it) ++n; n",
        "new Date(12345).toString().length > 0",
        "JSON.stringify({a:[1,2,{b:'c'}]})",
    ];
    for src in scripts {
        for l in [&p.c, &p.rs] {
            let mut cx = Actx::new(0);
            let ap = &mut *cx as *mut Actx as *mut c_void;
            unsafe {
                out_clear();
                set_cur(l);
                let j = l.js_newstate(Some(tracking_alloc), ap, 0);
                assert!(!j.is_null());
                l.js_setreport(j, Some(report_cb));
                // a userdata and a cfunctionx whose finalizers must run
                l.js_newobject(j);
                l.js_newuserdata(j, N_TAG, PAYLOAD as *mut c_void, Some(fin_cb));
                l.js_setglobal(j, N_UD);
                l.js_newcfunctionx(
                    j,
                    Some(cf_void),
                    N_VOIDF,
                    0,
                    PAYLOAD as *mut c_void,
                    Some(fin_cb),
                );
                l.js_setglobal(j, N_VOIDF);
                let cs = cstr(src);
                let _ = l.js_dostring(j, cs.as_ptr());
                // leave values on the value stack too
                for i in 0..10 {
                    l.js_pushnumber(j, i as f64);
                    l.js_newobject(j);
                }
                l.js_freestate(j);
                let text = out_take();
                assert_eq!(
                    text.matches("[finalize payload-A]").count(),
                    2,
                    "{}: finalizers did not all run for {src:?}: {text:?}",
                    l.name
                );
            }
            assert_eq!(cx.live, 0, "{}: leak after freestate for {src:?}", l.name);
            assert_eq!(cx.bad_actx, 0);
        }
    }
    // js_freestate(NULL) is a documented no-op
    for l in [&p.c, &p.rs] {
        unsafe { l.js_freestate(null_mut()) };
    }
}
