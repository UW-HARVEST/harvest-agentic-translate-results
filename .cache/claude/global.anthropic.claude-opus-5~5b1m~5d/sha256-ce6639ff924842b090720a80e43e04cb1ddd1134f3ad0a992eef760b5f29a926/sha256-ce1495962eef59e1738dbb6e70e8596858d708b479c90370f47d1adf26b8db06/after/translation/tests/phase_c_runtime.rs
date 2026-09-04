//! Phase C (runtime) — differential ERROR-PATH tests for ERRORS.md rows 170..338:
//! `jsrun.c`, `jsstate.c`, `jsvalue.c`, `jsproperty.c`, `jsintern.c`, `jserror.c`,
//! `jsgc.c`.
//!
//! Every case is driven either through `diff_eval` (JS source, errors caught by
//! the harness `js_pcall`) or `diff_native` (raw C API inside a `js_pcall`, so a
//! throw is caught and rendered instead of reaching `js_throw`'s `abort()`).
//! Nothing here lets an exception escape to the panic handler.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void, CString};

const SEED: u64 = 0x5EED_C0DE_1701;

/* flag words handed to js_newstate, including out-of-range values */
const FLAGSETS: [c_int; 5] = [0, JS_STRICT, 2, -1, 0x7fffffff];

/* -------------------------------------------------------------------------- */
/* the seven variadic throwers of jserror.c are exported by both libraries but  */
/* are not part of the harness `Api` (their signature is variadic).  They are   */
/* fetched from the very same two `.so` files (same paths as `common::libs()`,  */
/* so `dlopen` returns the identical handle and the identical symbols).         */
/* -------------------------------------------------------------------------- */

type Vfn = unsafe extern "C" fn(JS, *const c_char, ...);

struct Throwers {
    error: Vfn,
    evalerror: Vfn,
    rangeerror: Vfn,
    referenceerror: Vfn,
    syntaxerror: Vfn,
    typeerror: Vfn,
    urierror: Vfn,
}

fn load_throwers(p: &std::path::Path) -> Throwers {
    unsafe {
        let lib = libloading::Library::new(p)
            .unwrap_or_else(|e| panic!("dlopen {}: {}", p.display(), e));
        let get = |n: &[u8]| -> Vfn {
            let s: libloading::Symbol<Vfn> = lib
                .get(n)
                .unwrap_or_else(|e| panic!("{}: missing {:?}: {}", p.display(), n, e));
            *s
        };
        let t = Throwers {
            error: get(b"js_error\0"),
            evalerror: get(b"js_evalerror\0"),
            rangeerror: get(b"js_rangeerror\0"),
            referenceerror: get(b"js_referenceerror\0"),
            syntaxerror: get(b"js_syntaxerror\0"),
            typeerror: get(b"js_typeerror\0"),
            urierror: get(b"js_urierror\0"),
        };
        std::mem::forget(lib);
        t
    }
}

fn throwers(tag: &str) -> &'static Throwers {
    use std::sync::OnceLock;
    static T: OnceLock<(Throwers, Throwers)> = OnceLock::new();
    let t = T.get_or_init(|| (load_throwers(&c_so()), load_throwers(&rust_so())));
    if tag == "C" {
        &t.0
    } else {
        &t.1
    }
}

/* -------------------------------------------------------------------------- */
/* small helpers                                                              */
/* -------------------------------------------------------------------------- */

fn dump(a: &Api, J: JS) {
    unsafe {
        let n = (a.js_gettop)(J);
        emit(&format!("top={}", n));
        for i in 0..n {
            emit(&format!("[{}]={}", i, repr_at(a, J, i)));
        }
    }
}

/// Load + call `src` in the *current* state, recording rc and the result.
fn run_src(a: &Api, J: JS, src: &CString, label: &str) {
    unsafe {
        let nm = cs("sub.js");
        let rc = (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr());
        if rc != 0 {
            emit(&format!("{} load={} {:?}", label, rc, str_at(a, J, -1)));
            (a.js_pop)(J, 1);
            return;
        }
        (a.js_pushundefined)(J);
        let rc = (a.js_pcall)(J, 0);
        emit(&format!("{} call={} {:?}", label, rc, str_at(a, J, -1)));
        (a.js_pop)(J, 1);
    }
}

fn rep300() -> String {
    "p".repeat(300)
}

/// Deeply-recursive JS needs more C stack than libtest's default 2 MiB thread.
fn on_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(192 << 20)
        .spawn(f)
        .expect("spawn big-stack thread")
        .join()
        .expect("big-stack thread panicked");
}

/* ========================================================================== */
/* rows 197-204: stack underflow / stack error / rot counts                    */
/* ========================================================================== */

/* row 197 js_pop */
#[test]
fn r197_js_pop_underflow() {
    for npush in [0i64, 1, 2, 5] {
        for n in [0i64, 1, 2, 3, 6, 100] {
            set_pi(0, npush);
            set_pi(1, n);
            fn act(a: &Api, J: JS) {
                unsafe {
                    for k in 0..pi(0) {
                        (a.js_pushnumber)(J, k as f64 + 0.5);
                    }
                    emit(&format!("before={}", (a.js_gettop)(J)));
                    (a.js_pop)(J, pic(1));
                    emit(&format!("after={}", (a.js_gettop)(J)));
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            let c = p.c.run_native(act, 0);
            let r = p.r.run_native(act, 0);
            same(&format!("js_pop npush={} n={}", npush, n), &c, &r);
            if n > npush + 1 {
                assert!(c.contains("stack underflow!"), "npush={} n={}: {}", npush, n, c);
            }
        }
    }
}

/* rows 198 + 200 js_remove / js_replace "stack error!" */
#[test]
fn r198_r200_remove_replace_stack_error() {
    for op in [0i64, 1] {
        for npush in [0i64, 1, 3] {
            for idx in [-100i64, -5, -1, 0, 1, 3, 50, 100] {
                set_pi(0, op);
                set_pi(1, npush);
                set_pi(2, idx);
                fn act(a: &Api, J: JS) {
                    unsafe {
                        for k in 0..pi(1) {
                            (a.js_pushnumber)(J, k as f64 + 0.5);
                        }
                        emit(&format!("before={}", (a.js_gettop)(J)));
                        if pi(0) == 0 {
                            (a.js_remove)(J, pic(2));
                        } else {
                            (a.js_replace)(J, pic(2));
                        }
                        dump(a, J);
                        (a.js_pushnumber)(J, 0.0);
                    }
                }
                let p = libs();
                let c = p.c.run_native(act, 0);
                let r = p.r.run_native(act, 0);
                let label = format!(
                    "{} npush={} idx={}",
                    if op == 0 { "remove" } else { "replace" },
                    npush,
                    idx
                );
                same(&label, &c, &r);
                let depth = npush + 1;
                if idx >= depth || idx < -depth {
                    assert!(c.contains("stack error!"), "{}: {}", label, c);
                }
            }
        }
    }
}

/* row 199 js_insert is unconditionally "not implemented yet" */
#[test]
fn r199_js_insert_not_implemented() {
    for idx in [-1i64, 0, 1, 99] {
        set_pi(0, idx);
        fn act(a: &Api, J: JS) {
            unsafe {
                (a.js_pushnumber)(J, 1.0);
                (a.js_insert)(J, pic(0));
                emit("insert-returned");
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        same(&format!("js_insert({})", idx), &c, &p.r.run_native(act, 0));
        assert!(c.contains("not implemented yet"), "{}", c);
        for f in [JS_STRICT, 2] {
            diff_native(&format!("js_insert({})", idx), act, f);
        }
    }
}

/* row 204 js_rot / js_rotN — counts that stay inside the frame */
#[test]
fn r204_rot_counts_in_frame() {
    for npush in [0i64, 1, 2, 3, 4, 6] {
        let depth = npush + 1; /* `this` is index 0 */
        for n in 0..=depth {
            set_pi(0, npush);
            set_pi(1, n);
            fn act(a: &Api, J: JS) {
                unsafe {
                    for k in 0..pi(0) {
                        (a.js_pushnumber)(J, k as f64 + 0.5);
                    }
                    (a.js_rot)(J, pic(1));
                    dump(a, J);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            same(
                &format!("js_rot npush={} n={}", npush, n),
                &p.c.run_native(act, 0),
                &p.r.run_native(act, 0),
            );
        }
    }
    /* rot2/rot3/rot4/rot2pop1/rot3pop2 at exactly their minimum depth */
    for op in 0i64..5 {
        set_pi(0, op);
        fn act(a: &Api, J: JS) {
            unsafe {
                let need = match pi(0) {
                    0 => 2,
                    1 => 3,
                    2 => 4,
                    3 => 2,
                    _ => 3,
                };
                /* frame already holds `this`, so push need-1 more */
                for k in 0..(need - 1) {
                    (a.js_pushnumber)(J, k as f64 + 0.5);
                }
                match pi(0) {
                    0 => (a.js_rot2)(J),
                    1 => (a.js_rot3)(J),
                    2 => (a.js_rot4)(J),
                    3 => (a.js_rot2pop1)(J),
                    _ => (a.js_rot3pop2)(J),
                }
                dump(a, J);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        diff_native(&format!("rotN op={}", op), act, 0);
    }
}

/* rows 192 + 298: stackidx() masks every out-of-range index as `undefined` */
#[test]
fn r192_r298_stackidx_out_of_range() {
    fn act(a: &Api, J: JS) {
        unsafe {
            (a.js_pushnumber)(J, 7.0);
            (a.js_pushstring)(J, cs("s").as_ptr());
            let e = cs("<E>");
            for idx in [-1000i64, -100, -4, -3, -2, -1, 0, 1, 2, 3, 50, 99, 4095, 100000] {
                let i = idx as c_int;
                emit(&format!(
                    "idx={} type={} typeof={} undef={} def={} tryi={} tryn={:#x} trys={:?} tryr={:?} tob={}",
                    idx,
                    (a.js_type)(J, i),
                    rs((a.js_typeof)(J, i)),
                    (a.js_isundefined)(J, i),
                    (a.js_isdefined)(J, i),
                    (a.js_tryinteger)(J, i, -7),
                    (a.js_trynumber)(J, i, -7.5).to_bits(),
                    rs((a.js_trystring)(J, i, e.as_ptr())),
                    rs((a.js_tryrepr)(J, i, e.as_ptr())),
                    (a.js_toboolean)(J, i),
                ));
                emit(&format!("tonum={:#x}", (a.js_tonumber)(J, i).to_bits()));
                /* js_copy of an out-of-range slot yields the shared undefined */
                (a.js_copy)(J, i);
                emit(&format!("copy={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }
            dump(a, J);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("stackidx-oob", act, f);
    }
}

/* ========================================================================== */
/* rows 171, 179-189, 201-203: CHECKSTACK -> "stack overflow"                  */
/* ========================================================================== */

static LIT: &[u8] = b"literal\0";

#[test]
fn r171_checkstack_overflow_every_push() {
    /* 0..=15 selects which pushing primitive is hammered until CHECKSTACK fires */
    for op in 0i64..16 {
        set_pi(0, op);
        fn act(a: &Api, J: JS) {
            unsafe {
                let long = cs("0123456789ABCDEF-a-memstr-not-a-shrstr");
                let short = cs("s");
                for k in 0..6000 {
                    if k % 1024 == 0 {
                        emit(&format!("k={} top={}", k, (a.js_gettop)(J)));
                    }
                    match pi(0) {
                        0 => (a.js_pushundefined)(J),
                        1 => (a.js_pushnull)(J),
                        2 => (a.js_pushboolean)(J, 1),
                        3 => (a.js_pushnumber)(J, k as f64),
                        4 => (a.js_pushstring)(J, short.as_ptr()),
                        5 => (a.js_pushstring)(J, long.as_ptr()),
                        6 => (a.js_pushliteral)(J, LIT.as_ptr() as *const c_char),
                        7 => (a.js_pushlstring)(J, long.as_ptr(), 4),
                        8 => (a.js_newobject)(J),
                        9 => (a.js_newarray)(J),
                        10 => (a.js_copy)(J, 0),
                        11 => (a.js_dup)(J),
                        12 => (a.js_dup2)(J),
                        13 => (a.js_currentfunction)(J),
                        14 => (a.js_pushglobal)(J),
                        _ => (a.js_newnumber)(J, k as f64),
                    }
                }
                emit("no-overflow");
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        let r = p.r.run_native(act, 0);
        same(&format!("checkstack op={}", op), &c, &r);
        assert!(c.contains("stack overflow"), "op={}: {}", op, c);
    }
}

/* rows 178 + 180: JS_STRLIMIT on js_pushstring / js_pushlstring.
 * js_pushlstring checks `n` BEFORE touching the buffer, so the limit is
 * reachable with a tiny buffer and a huge length. */
#[test]
fn r180_pushlstring_invalid_string_length() {
    /* Only two classes of `n` are legal to pass:
     *   - n <= the real buffer length (the copy stays in bounds), and
     *   - n > JS_STRLIMIT, which is rejected *before* the buffer is touched.
     * Anything in between (e.g. n == 1<<28, or a negative n) makes the C
     * memcpy/`while (n--)` loop run off the end of the buffer — a genuine
     * out-of-bounds read in both libraries, so it is not exercised here. */
    for n in [0i64, 1, 4, (1 << 28) + 1, (1 << 29) + 7, i32::MAX as i64] {
        set_pi(0, n);
        fn act(a: &Api, J: JS) {
            unsafe {
                let buf = cs("abcd");
                (a.js_pushlstring)(J, buf.as_ptr(), pic(0));
                emit(&format!(
                    "pushed len={} repr={}",
                    (a.js_getlength)(J, -1),
                    repr_at(a, J, -1)
                ));
                (a.js_pop)(J, 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        let r = p.r.run_native(act, 0);
        same(&format!("js_pushlstring n={}", n), &c, &r);
        if n > (1 << 28) {
            assert!(
                c.contains("invalid string length"),
                "expected RangeError for n={}, got {}",
                n,
                c
            );
        }
    }
}

/* rows 178 + 320: a real >2^28 byte string for js_pushstring / js_intern.
 * Allocates ~256 MiB once and shares it between both libraries. */
fn huge_string() -> &'static CString {
    use std::sync::OnceLock;
    static S: OnceLock<CString> = OnceLock::new();
    S.get_or_init(|| {
        let n = (1usize << 28) + 8;
        CString::new(vec![b'a'; n]).unwrap()
    })
}

#[test]
fn r178_r320_strlimit_pushstring_and_intern() {
    fn act(a: &Api, J: JS) {
        unsafe {
            let s = huge_string();
            match pi(0) {
                0 => {
                    (a.js_pushstring)(J, s.as_ptr());
                    emit("pushstring-ok");
                    (a.js_pop)(J, 1);
                }
                _ => {
                    let p = (a.js_intern)(J, s.as_ptr());
                    emit(&format!("intern-ok null={}", p.is_null()));
                }
            }
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for op in 0i64..2 {
        set_pi(0, op);
        let p = libs();
        let c = p.c.run_native(act, 0);
        let r = p.r.run_native(act, 0);
        same(&format!("strlimit op={}", op), &c, &r);
        assert!(c.contains("invalid string length"), "op={}: {}", op, c);
    }
}

/* ========================================================================== */
/* rows 245, 246: JS_ENVLIMIT — deep JS recursion                              */
/* ========================================================================== */

#[test]
fn r245_r246_deep_recursion_limits() {
    on_big_stack(|| {
        let srcs = [
            "function f(){ return f() } try { f() } catch (e) { String(e) }",
            "function f(){ f() } try { f() } catch (e) { e.name + '/' + e.message }",
            "var g = function(){ return 1 + g() }; try { g() } catch (e) { String(e) }",
            "function f(n){ return n <= 0 ? 0 : 1 + f(n-1) } try { f(5000) } catch (e) { String(e) }",
            "function f(n){ return n <= 0 ? 0 : 1 + f(n-1) } f(100)",
            "function f(){ return arguments.length ? f() : f(1) } try { f() } catch (e) { String(e) }",
            "try { (function r(){ return [r()] })() } catch (e) { String(e) }",
            "function f(){ try { f() } catch (e) { throw e } } try { f() } catch (e) { String(e) }",
        ];
        for s in srcs {
            for f in [0, JS_STRICT] {
                diff_eval("deep recursion", s, f);
            }
        }
        /* row 261: a *lightweight* recursive function keeps its locals on the
         * value stack, so CHECKSTACK in OP_GETLOCAL / jsR_calllwfunction fires
         * long before the JS_ENVLIMIT trace guard. `r` only touches its own
         * parameters and locals, which is what makes it lightweight. */
        let mut locals = String::new();
        for i in 0..40 {
            locals.push_str(&format!("var L{} = {};", i, i));
        }
        let lw = format!(
            "function r(fn, n){{ {} if (n <= 0) return L0; return fn(fn, n-1) }}              try {{ r(r, 5000) }} catch (e) {{ String(e) }}",
            locals
        );
        for f in [0, JS_STRICT] {
            diff_eval("lightweight recursion", &lw, f);
        }

        /* ground truth: jsR_pushtrace's JS_ENVLIMIT guard, and the value-stack
         * guard for the lightweight variant */
        let p = libs();
        let c = p.c.eval(srcs[0], 0);
        assert!(c.contains("call stack overflow"), "{}", c);
        let c = p.c.eval(&lw, 0);
        assert!(c.contains("stack overflow"), "{}", c);
    });
}

/* ========================================================================== */
/* rows 170, 256: JS_TRYLIMIT reached by nested JS try blocks                  */
/* ========================================================================== */

#[test]
fn r170_r256_trylimit_nested_js_try() {
    on_big_stack(|| {
        for n in 1..=70usize {
            if !(n <= 2 || (58..=70).contains(&n)) {
                continue;
            }
            let mut src = String::from("var r='none';");
            for _ in 0..n {
                src.push_str("try{");
            }
            src.push_str("r='deep';");
            for _ in 0..n {
                src.push_str("}catch(e){r=String(e)}");
            }
            src.push_str("r");
            for f in [0, JS_STRICT] {
                diff_eval(&format!("nested try n={}", n), &src, f);
            }
            let p = libs();
            let c = p.c.eval(&src, 0);
            /* JS_TRYLIMIT is 64 and the harness `js_pcall` already holds one
             * frame, so the 64th script-level `try` is the one that overflows. */
            if n >= 64 {
                assert!(c.contains("exception stack overflow"), "n={}: {}", n, c);
            } else {
                assert!(c.contains("deep"), "n={}: {}", n, c);
            }
        }
    });
}

/* ========================================================================== */
/* rows 269-278: js_ptry — every js_p... / js_try... entry point, full try stack */
/* ========================================================================== */

unsafe extern "C" fn rep_emit(_J: JS, msg: *const c_char) {
    emit(&format!("report:{:?}", unsafe { rs(msg) }));
}

/* Called from the innermost of N nested JS try blocks, so J->trytop is exactly
 * 2 + N.  Only entry points that go through js_ptry (which never longjmps) are
 * used here. */
unsafe extern "C" fn cf_probe_ptry(J: JS) {
    let a = cur();
    unsafe {
        (a.js_setreport)(J, Some(rep_emit));
        let nm = cs("probe.js");
        let ok = cs("1+1");
        let bad = cs("var 1 = ;");

        /* rows 270/271: js_ploadstring */
        let rc = (a.js_ploadstring)(J, nm.as_ptr(), ok.as_ptr());
        emit(&format!("plo_ok={} v={:?}", rc, str_at(a, J, -1)));
        (a.js_pop)(J, 1);
        let rc = (a.js_ploadstring)(J, nm.as_ptr(), bad.as_ptr());
        emit(&format!("plo_bad={} v={:?}", rc, str_at(a, J, -1)));
        (a.js_pop)(J, 1);

        /* rows 272-276: js_trystring / js_trynumber / js_tryinteger / js_tryboolean
         * on a value whose conversion throws */
        let src = cs("({toString:function(){throw new Error('ts')},valueOf:function(){throw new Error('vo')}})");
        let rc = (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr());
        if rc == 0 {
            (a.js_pushundefined)(J);
            if (a.js_pcall)(J, 0) == 0 {
                let e = cs("<FALLBACK>");
                emit(&format!("trystring={:?}", rs((a.js_trystring)(J, -1, e.as_ptr()))));
                emit(&format!("trynumber={:#x}", (a.js_trynumber)(J, -1, -3.5).to_bits()));
                emit(&format!("tryinteger={}", (a.js_tryinteger)(J, -1, -3)));
                emit(&format!("tryboolean={}", (a.js_tryboolean)(J, -1, -3)));
                emit(&format!("tryrepr={:?}", rs((a.js_tryrepr)(J, -1, e.as_ptr()))));
            } else {
                emit("build-throwing-object-failed");
            }
        }
        (a.js_pop)(J, 1);

        /* rows 278/279: js_dostring reports through J->report */
        emit(&format!("dostring_ok={}", (a.js_dostring)(J, cs("1+1").as_ptr())));
        emit(&format!("dostring_bad={}", (a.js_dostring)(J, cs("var 1 = ;").as_ptr())));
        emit(&format!(
            "dostring_throw={}",
            (a.js_dostring)(J, cs("throw new Error('boom')").as_ptr())
        ));
        (a.js_pushnumber)(J, 0.0);
    }
}

#[test]
fn r269_r278_ptry_with_full_try_stack() {
    on_big_stack(|| {
        let mut saw_ptry_refusal = false;
        for n in 55..=66usize {
            set_pi(0, n as i64);
            let mut src = String::from("var r='none';");
            for _ in 0..n {
                src.push_str("try{");
            }
            src.push_str("r=String(PROBE());");
            for _ in 0..n {
                src.push_str("}catch(e){r='CAUGHT:'+String(e)}");
            }
            src.push_str("r");
            set_ps(0, &src);
            fn act(a: &Api, J: JS) {
                unsafe {
                    (a.js_newcfunction)(J, Some(cf_probe_ptry), b"PROBE\0".as_ptr() as *const c_char, 0);
                    (a.js_setglobal)(J, cs("PROBE").as_ptr());
                    let src = ps(0);
                    run_src(a, J, &src, "nested");
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            let c = p.c.run_native(act, 0);
            let r = p.r.run_native(act, 0);
            same(&format!("ptry full stack n={}", n), &c, &r);
            /* The 64 JS_TRYLIMIT slots are shared with the harness pcall, the
             * sub-script pcall and the try frames js_ploadstring/js_loadstringx
             * install themselves; beyond some `n` the nested `try` statements
             * overflow before PROBE is even reached.  What is pinned down is
             * that the small end still calls js_ploadstring successfully and
             * that somewhere in the sweep js_ptry does refuse. */
            if n <= 55 {
                assert!(c.contains("plo_ok=0"), "n={}: {}", n, c);
            }
            if c.contains("plo_ok=1") && c.contains("exception stack overflow") {
                saw_ptry_refusal = true;
            }
        }
        assert!(saw_ptry_refusal, "js_ptry never hit its JS_TRYLIMIT guard");
    });
}

/* row 257: js_savetry with a full try stack — js_trystackoverflow longjmps to
 * the innermost *real* OP_TRY setjmp, so the JS catch handles it. */
#[test]
fn r257_savetry_overflow_caught_by_js_catch() {
    on_big_stack(|| {
        for n in 55..=66usize {
            let mut src = String::from("var r='none';");
            for _ in 0..n {
                src.push_str("try{");
            }
            src.push_str("r=String(PROBE());");
            for _ in 0..n {
                src.push_str("}catch(e){r='CAUGHT:'+String(e)}");
            }
            src.push_str("r");
            set_ps(0, &src);
            fn act(a: &Api, J: JS) {
                unsafe {
                    (a.js_newcfunction)(J, Some(cf_probe_savetry), b"PROBE\0".as_ptr() as *const c_char, 0);
                    (a.js_setglobal)(J, cs("PROBE").as_ptr());
                    let src = ps(0);
                    run_src(a, J, &src, "savetry");
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            let c = p.c.run_native(act, 0);
            let r = p.r.run_native(act, 0);
            same(&format!("savetry overflow n={}", n), &c, &r);
            if n >= 62 {
                assert!(c.contains("CAUGHT:exception stack overflow"), "n={}: {}", n, c);
            } else {
                assert!(c.contains("pcall=0"), "n={}: {}", n, c);
            }
        }
    });
}

unsafe extern "C" fn cf_noop(J: JS) {
    let a = cur();
    unsafe { (a.js_pushnumber)(J, 42.0) };
}

unsafe extern "C" fn cf_probe_savetry(J: JS) {
    let a = cur();
    unsafe {
        /* js_pcall installs a try frame: with a full try stack js_savetry throws */
        (a.js_newcfunction)(J, Some(cf_noop), b"noop\0".as_ptr() as *const c_char, 0);
        (a.js_pushundefined)(J);
        let rc = (a.js_pcall)(J, 0);
        emit(&format!("pcall={} v={:?}", rc, str_at(a, J, -1)));
        (a.js_pop)(J, 1);
        (a.js_pushnumber)(J, 1.0);
    }
}

/* ========================================================================== */
/* rows 219-230, 315, 316: jsR_setproperty readonly / transient / extensible   */
/* ========================================================================== */

const SETPROP_SRCS: &[&str] = &[
    /* array length (rows 217, 218) */
    "var a=[]; a.length=1.5; a.length",
    "var a=[]; a.length=-1; a.length",
    "var a=[]; a.length='x'; a.length",
    "var a=[]; a.length=NaN; a.length",
    "var a=[]; a.length=Infinity; a.length",
    "var a=[1,2,3]; a.length=1; String(a)",
    "var a=[]; a.length=67108864; a.length",
    "var a=[]; a.length=67108865; a.length",
    "var a=[]; a.length=4294967296; a.length",
    "var a=[1,2,3]; a[1000]=1; a.length",
    /* String object length + indices (rows 219, 220) */
    "var s=new String('ab'); s.length=5; s.length",
    "var s=new String('ab'); s[0]='z'; s[0]",
    "var s=new String('ab'); s[5]='z'; s[5]",
    "'abc'.length=9",
    /* RegExp own slots (rows 221-224 + lastIndex) */
    "var r=/a/; r.source='b'; r.source",
    "var r=/a/; r.global=true; r.global",
    "var r=/a/; r.ignoreCase=true; r.ignoreCase",
    "var r=/a/; r.multiline=true; r.multiline",
    "var r=/a/g; r.lastIndex=3; r.lastIndex",
    "var r=/a/g; r.lastIndex='7'; r.lastIndex",
    /* getter-only accessor (row 225) */
    "var o={get x(){return 1}}; o.x=2; o.x",
    "var o=Object.create({get x(){return 1}}); o.x=2; o.x",
    /* writable:false (rows 226, 229) */
    "var o={}; Object.defineProperty(o,'x',{value:1,writable:false}); o.x=2; o.x",
    "var o=Object.create(Object.defineProperty({},'x',{value:1,writable:false})); o.x=2; o.x",
    /* transient receivers (rows 227, 228) */
    "'abc'.foo = 1",
    "(5).bar = 1",
    "true.baz = 1",
    "var s='abc'; s.foo=1; s.foo",
    /* non-extensible / sealed / frozen (rows 230, 315, 316) */
    "var o=Object.preventExtensions({}); o.x=1; o.x",
    "var o=Object.preventExtensions({y:1}); o.y=2; o.y",
    "var o=Object.seal({y:1}); o.y=2; o.x=3; [o.y,o.x]",
    "var o=Object.freeze({y:1}); o.y=2; o.y",
    "var o=Object.freeze({y:1}); o.z=2; o.z",
    "var o=Object.freeze([1,2]); o[0]=9; String(o)",
    /* setter present -> no error */
    "var o={set x(v){this.v=v}}; o.x=5; o.v",
];

#[test]
fn r219_r230_setproperty_readonly_js() {
    for s in SETPROP_SRCS {
        for f in [0, JS_STRICT] {
            diff_eval("setproperty", s, f);
            diff_eval("setproperty/usestrict", &format!("'use strict'; {}", s), f);
        }
    }
}

/* the same readonly slots, but reached through the raw C API */
#[test]
fn r219_r230_setproperty_readonly_native() {
    /* 0 String length, 1 String index, 2 regexp source, 3 regexp global,
     * 4 regexp ignoreCase, 5 regexp multiline, 6 regexp lastIndex,
     * 7 array length invalid, 8 array length too large, 9 readonly own prop,
     * 10 transient string, 11 transient number, 12 non-extensible,
     * 13 getter-only, 14 readonly with 300-char name (message truncation) */
    for mode in 0i64..15 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                let long = cs(&rep300());
                match pi(0) {
                    0 => {
                        (a.js_newstring)(J, cs("ab").as_ptr());
                        (a.js_pushnumber)(J, 5.0);
                        (a.js_setproperty)(J, -2, cs("length").as_ptr());
                    }
                    1 => {
                        (a.js_newstring)(J, cs("ab").as_ptr());
                        (a.js_pushstring)(J, cs("z").as_ptr());
                        (a.js_setproperty)(J, -2, cs("0").as_ptr());
                    }
                    2..=6 => {
                        let name = ["source", "global", "ignoreCase", "multiline", "lastIndex"]
                            [(pi(0) - 2) as usize];
                        (a.js_newregexp)(J, cs("a").as_ptr(), JS_REGEXP_G);
                        (a.js_pushnumber)(J, 3.0);
                        (a.js_setproperty)(J, -2, cs(name).as_ptr());
                    }
                    7 => {
                        (a.js_newarray)(J);
                        (a.js_pushnumber)(J, 1.5);
                        (a.js_setproperty)(J, -2, cs("length").as_ptr());
                    }
                    8 => {
                        (a.js_newarray)(J);
                        (a.js_pushnumber)(J, (1u32 << 26) as f64 + 1.0);
                        (a.js_setproperty)(J, -2, cs("length").as_ptr());
                    }
                    9 => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs("x").as_ptr(), JS_READONLY);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_setproperty)(J, -2, cs("x").as_ptr());
                    }
                    10 => {
                        (a.js_pushstring)(J, cs("abc").as_ptr());
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setproperty)(J, -2, cs("foo").as_ptr());
                    }
                    11 => {
                        (a.js_pushnumber)(J, 5.0);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setproperty)(J, -2, cs("bar").as_ptr());
                    }
                    12 => {
                        /* rows 230/315/316: jsV_setproperty on a non-extensible
                         * object returns NULL (non-strict) or throws (strict) */
                        let src = cs("Object.preventExtensions({keep:1})");
                        let nm = cs("ne.js");
                        if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) == 0 {
                            (a.js_pushundefined)(J);
                            if (a.js_pcall)(J, 0) == 0 {
                                (a.js_pushnumber)(J, 2.0);
                                (a.js_setproperty)(J, -2, cs("keep").as_ptr());
                                emit("existing-name-ok");
                                (a.js_pushnumber)(J, 1.0);
                                (a.js_setproperty)(J, -2, cs("fresh").as_ptr());
                                emit("fresh-name-ok");
                            }
                        }
                    }
                    13 => {
                        let src = cs("({get x(){return 1}})");
                        let nm = cs("g.js");
                        if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) == 0 {
                            (a.js_pushundefined)(J);
                            if (a.js_pcall)(J, 0) == 0 {
                                (a.js_pushnumber)(J, 2.0);
                                (a.js_setproperty)(J, -2, cs("x").as_ptr());
                            }
                        }
                    }
                    _ => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, long.as_ptr(), JS_READONLY);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_setproperty)(J, -2, long.as_ptr());
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                dump(a, J);
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("native setproperty mode={}", mode), act, f);
        }
    }
}

/* ========================================================================== */
/* rows 231-237: jsR_defproperty readonly / non-configurable                   */
/* ========================================================================== */

#[test]
fn r231_r237_defproperty_readonly_native() {
    /* js_defproperty/js_defaccessor pass throw=1, so the readonly label throws
     * even in non-strict mode; js_defglobal passes throw=0. */
    for mode in 0i64..12 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                let long = cs(&rep300());
                match pi(0) {
                    0 => {
                        (a.js_newarray)(J);
                        (a.js_pushnumber)(J, 3.0);
                        (a.js_defproperty)(J, -2, cs("length").as_ptr(), 0);
                    }
                    1 => {
                        (a.js_newstring)(J, cs("ab").as_ptr());
                        (a.js_pushnumber)(J, 3.0);
                        (a.js_defproperty)(J, -2, cs("length").as_ptr(), 0);
                    }
                    2 => {
                        (a.js_newstring)(J, cs("ab").as_ptr());
                        (a.js_pushstring)(J, cs("z").as_ptr());
                        (a.js_defproperty)(J, -2, cs("0").as_ptr(), 0);
                    }
                    3..=7 => {
                        let name = ["source", "global", "ignoreCase", "multiline", "lastIndex"]
                            [(pi(0) - 3) as usize];
                        (a.js_newregexp)(J, cs("a").as_ptr(), 0);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs(name).as_ptr(), 0);
                    }
                    8 => {
                        /* row 235: redefine the value of a JS_READONLY property */
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs("x").as_ptr(), JS_READONLY);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_defproperty)(J, -2, cs("x").as_ptr(), JS_READONLY);
                    }
                    9 => {
                        /* row 236: install a getter over a JS_DONTCONF property */
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs("x").as_ptr(), JS_DONTCONF);
                        (a.js_newcfunction)(J, Some(cf_noop), b"g\0".as_ptr() as *const c_char, 0);
                        (a.js_pushundefined)(J);
                        (a.js_defaccessor)(J, -3, cs("x").as_ptr(), 0);
                    }
                    10 => {
                        /* row 237: install a setter over a JS_DONTCONF property */
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs("x").as_ptr(), JS_DONTCONF);
                        (a.js_pushundefined)(J);
                        (a.js_newcfunction)(J, Some(cf_noop), b"s\0".as_ptr() as *const c_char, 0);
                        (a.js_defaccessor)(J, -3, cs("x").as_ptr(), 0);
                    }
                    _ => {
                        /* 300-char name: vsnprintf truncation in js_typeerror */
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, long.as_ptr(), JS_READONLY);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_defproperty)(J, -2, long.as_ptr(), JS_READONLY);
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                dump(a, J);
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("native defproperty mode={}", mode), act, f);
        }
    }
}

#[test]
fn r231_r237_defproperty_readonly_js() {
    let srcs = [
        "Object.defineProperty([], 'length', {value:3})",
        "Object.defineProperty([1,2], 'length', {value:0})",
        "Object.defineProperty(new String('ab'), 'length', {value:3})",
        "Object.defineProperty(new String('ab'), '0', {value:'z'})",
        "Object.defineProperty(new String('ab'), '5', {value:'z'})",
        "Object.defineProperty(/a/, 'source', {value:'b'})",
        "Object.defineProperty(/a/, 'global', {value:true})",
        "Object.defineProperty(/a/, 'ignoreCase', {value:true})",
        "Object.defineProperty(/a/, 'multiline', {value:true})",
        "Object.defineProperty(/a/, 'lastIndex', {value:1})",
        "var o={}; Object.defineProperty(o,'x',{value:1,writable:false}); Object.defineProperty(o,'x',{value:2}); o.x",
        "var o={}; Object.defineProperty(o,'x',{value:1,configurable:false}); Object.defineProperty(o,'x',{get:function(){return 9}}); o.x",
        "var o={}; Object.defineProperty(o,'x',{value:1,configurable:false}); Object.defineProperty(o,'x',{set:function(v){}}); o.x",
        "Object.defineProperty(function(){}, 'length', {get:function(){return 1}})",
        "var o=Object.preventExtensions({}); Object.defineProperty(o,'x',{value:1}); o.x",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("defproperty", s, f);
            diff_eval("defproperty/usestrict", &format!("'use strict'; {}", s), f);
        }
    }
}

/* ========================================================================== */
/* rows 238-241, 244: jsR_delproperty / js_delvar "non-configurable"           */
/* ========================================================================== */

#[test]
fn r238_r244_delproperty_dontconf() {
    let srcs = [
        "delete [].length",
        "var a=[1,2,3]; [delete a[2], a.length, String(a)]",
        "var a=[1,2,3]; [delete a[0], a.length, String(a)]",
        "var s=new String('ab'); delete s.length",
        "var s=new String('ab'); delete s[0]",
        "var s=new String('ab'); delete s[9]",
        "delete /a/.source",
        "delete /a/.global",
        "delete /a/.ignoreCase",
        "delete /a/.multiline",
        "delete /a/.lastIndex",
        "delete Math",
        "delete JSON",
        "delete undefined",
        "delete Object",
        "var o={}; Object.defineProperty(o,'x',{value:1,configurable:false}); [delete o.x, o.x]",
        "var o={x:1}; [delete o.x, o.x]",
        "delete nosuchglobalatall",
        "function f(){ var x=1; return eval('delete x') } f()",
        "function f(){ return eval('delete arguments') } f()",
        "var o=Object.seal({a:1}); [delete o.a, o.a]",
        "var o=Object.freeze({a:1}); [delete o.a, o.a]",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("delproperty", s, f);
            diff_eval("delproperty/usestrict", &format!("'use strict'; {}", s), f);
        }
    }
    /* same through the C API, plus the 300-char-name truncation */
    for mode in 0i64..7 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                let long = cs(&rep300());
                match pi(0) {
                    0 => {
                        (a.js_newarray)(J);
                        (a.js_delproperty)(J, -1, cs("length").as_ptr());
                    }
                    1 => {
                        (a.js_newstring)(J, cs("ab").as_ptr());
                        (a.js_delproperty)(J, -1, cs("length").as_ptr());
                    }
                    2 => {
                        (a.js_newstring)(J, cs("ab").as_ptr());
                        (a.js_delproperty)(J, -1, cs("0").as_ptr());
                    }
                    3 => {
                        (a.js_newregexp)(J, cs("a").as_ptr(), 0);
                        (a.js_delproperty)(J, -1, cs("source").as_ptr());
                    }
                    4 => {
                        (a.js_newregexp)(J, cs("a").as_ptr(), 0);
                        (a.js_delproperty)(J, -1, cs("lastIndex").as_ptr());
                    }
                    5 => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs("x").as_ptr(), JS_DONTCONF);
                        (a.js_delproperty)(J, -1, cs("x").as_ptr());
                    }
                    _ => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, long.as_ptr(), JS_DONTCONF);
                        (a.js_delproperty)(J, -1, long.as_ptr());
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                dump(a, J);
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("native delproperty mode={}", mode), act, f);
        }
    }
}

/* ========================================================================== */
/* rows 215-218: array length / index limits                                   */
/* ========================================================================== */

#[test]
fn r215_r218_array_length_limits() {
    for len in [
        -1.0f64,
        -0.5,
        0.0,
        1.5,
        3.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        ((1u64 << 26) - 1) as f64,
        (1u64 << 26) as f64,
        ((1u64 << 26) + 1) as f64,
        4294967295.0,
        4294967296.0,
        1e300,
    ] {
        set_pf(0, len);
        fn act(a: &Api, J: JS) {
            unsafe {
                (a.js_newarray)(J);
                (a.js_pushnumber)(J, pf(0));
                (a.js_setproperty)(J, -2, cs("length").as_ptr());
                emit(&format!("len={}", (a.js_getlength)(J, -1)));
                /* now poke an index on the (still flat) array */
                (a.js_pushnumber)(J, 1.0);
                (a.js_setindex)(J, -2, 0);
                emit(&format!("len2={}", (a.js_getlength)(J, -1)));
                (a.js_pop)(J, 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("array length {:e}", len), act, f);
        }
    }
    /* js_setlength / js_setindex with extreme integer indices */
    for k in [-1i64, 0, 1, (1 << 26) - 1, 1 << 26, (1 << 26) + 1, i32::MAX as i64] {
        set_pi(0, k);
        fn act(a: &Api, J: JS) {
            unsafe {
                (a.js_newarray)(J);
                (a.js_pushnumber)(J, 1.0);
                (a.js_setindex)(J, -2, pic(0));
                emit(&format!("len={} repr_ok", (a.js_getlength)(J, -1)));
                (a.js_setlength)(J, -1, pic(0));
                emit(&format!("len2={}", (a.js_getlength)(J, -1)));
                (a.js_pop)(J, 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("array index {}", k), act, f);
        }
    }
    let srcs = [
        "var a=[]; a[67108863]=1; a.length",
        "var a=[]; a[67108864]=1; a.length",
        "var a=[]; a[67108865]=1; a.length",
        "var a=[]; a[-1]=1; [a.length, a[-1]]",
        "var a=[]; a[1.5]=1; [a.length, a[1.5]]",
        "var a=[]; a['1']=1; a.length",
        "var a=[]; a['']=1; [a.length, a['']]",
        "var a=[]; a['01']=1; [a.length, a['01']]",
        "var a=[]; a['1a']=1; a.length",
        "var a=[]; a['4294967296']=1; a.length",
        "var a=[1,2]; a[7]",
        "var a=[1,2]; 7 in a",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("array index js", s, f);
        }
    }
}

/* ========================================================================== */
/* rows 242, 243, 262, 263, 264: js_setvar / OP_GETVAR / OP_HASVAR             */
/* ========================================================================== */

#[test]
fn r242_r243_setvar_readonly_and_undeclared() {
    /* row 242: a JS_READONLY global binding assigned from strict code */
    fn act(a: &Api, J: JS) {
        unsafe {
            (a.js_pushnumber)(J, 1.0);
            (a.js_defglobal)(J, cs("ro").as_ptr(), JS_READONLY);
            (a.js_pushnumber)(J, 2.0);
            (a.js_defglobal)(J, cs("rodc").as_ptr(), JS_READONLY | JS_DONTCONF);
            for s in [
                "ro = 2; ro",
                "'use strict'; ro = 2; ro",
                "function f(){ ro = 3; return ro } f()",
                "function f(){ 'use strict'; ro = 3; return ro } f()",
                "delete ro",
                "'use strict'; delete rodc",
                "rodc = 9; rodc",
            ] {
                let src = cs(s);
                run_src(a, J, &src, s);
            }
            (a.js_pushnumber)(J, 0.0);
        }
    }
    let p = libs();
    let c = p.c.run_native(act, 0);
    let r = p.r.run_native(act, 0);
    same("setvar readonly", &c, &r);
    assert!(c.contains("'ro' is read-only"), "{}", c);
    for f in [JS_STRICT, 2] {
        diff_native("setvar readonly", act, f);
    }
}

#[test]
fn r243_r263_referenceerror_paths() {
    let long = rep300();
    let srcs: Vec<String> = vec![
        "nosuchvariable".into(),
        "typeof nosuchvariable".into(),
        "nosuchvariable = 1".into(),
        "'use strict'; undeclaredvar = 1".into(),
        "function f(){ 'use strict'; undeclaredvar2 = 1 } f()".into(),
        "delete nosuchvariable".into(),
        "'use strict'; typeof nosuchvariable".into(),
        "function f(){ eval('delete x'); return x } var x=1; f()".into(),
        "function f(){ var x=1; eval('delete x'); return x } f()".into(),
        "with({}){ nosuchvariable }".into(),
        /* 300-char names exercise the vsnprintf truncation of the message */
        long.clone(),
        format!("'use strict'; {} = 1", long),
        format!("{} = 1; {}", long, long),
        format!("typeof {}", long),
    ];
    for s in &srcs {
        for f in [0, JS_STRICT] {
            diff_eval("referenceerror", s, f);
        }
    }
}

/* ========================================================================== */
/* rows 299, 300: jsV_toobject on undefined / null                            */
/* ========================================================================== */

#[test]
fn r299_r300_toobject_undefined_null() {
    for mode in 0i64..10 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                match pi(0) {
                    0 => {
                        (a.js_pushundefined)(J);
                        emit(&format!("toobject={:?}", (a.js_toobject)(J, -1)));
                    }
                    1 => {
                        (a.js_pushnull)(J);
                        emit(&format!("toobject={:?}", (a.js_toobject)(J, -1)));
                    }
                    2 => {
                        (a.js_pushundefined)(J);
                        let v = (a.js_tovalue)(J, -1);
                        emit(&format!("jsV_toobject={:?}", (a.jsV_toobject)(J, v)));
                    }
                    3 => {
                        (a.js_pushnull)(J);
                        let v = (a.js_tovalue)(J, -1);
                        emit(&format!("jsV_toobject={:?}", (a.jsV_toobject)(J, v)));
                    }
                    4 => {
                        (a.js_pushundefined)(J);
                        (a.js_getproperty)(J, -1, cs("x").as_ptr());
                    }
                    5 => {
                        (a.js_pushnull)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setproperty)(J, -2, cs("x").as_ptr());
                    }
                    6 => {
                        (a.js_pushundefined)(J);
                        emit(&format!("has={}", (a.js_hasproperty)(J, -1, cs("x").as_ptr())));
                    }
                    7 => {
                        (a.js_pushnull)(J);
                        emit(&format!("len={}", (a.js_getlength)(J, -1)));
                    }
                    8 => {
                        (a.js_pushundefined)(J);
                        (a.js_pushiterator)(J, -1, 1);
                    }
                    _ => {
                        /* out-of-range index yields the static undefined */
                        emit(&format!("toobject_oob={:?}", (a.js_toobject)(J, 99)));
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("toobject mode={}", mode), act, f);
        }
    }
    let srcs = [
        "undefined.x",
        "null.x",
        "undefined.x = 1",
        "null.x = 1",
        "undefined[0]",
        "null['a']",
        "var u; u.foo()",
        "for (var k in undefined) k",
        "for (var k in null) k",
        "for (var k in 1) k",
        "for (var k in 'ab') k",
        "'x' in 1",
        "'x' in 'abc'",
        "'x' in null",
        "delete undefined.x",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("toobject js", s, f);
        }
    }
}

/* ========================================================================== */
/* rows 193, 194, 195, 196: js_toregexp / js_touserdata / jsR_tofunction       */
/* ========================================================================== */

unsafe extern "C" fn ud_fin(_J: JS, _p: *mut c_void) {
    emit("finalize");
}

#[test]
fn r193_r196_toregexp_touserdata_tofunction() {
    for mode in 0i64..14 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                let tag = TAG_FOO.as_ptr() as *const c_char;
                let other = TAG_BAR.as_ptr() as *const c_char;
                match pi(0) {
                    /* row 193 js_toregexp */
                    0 => {
                        (a.js_pushnumber)(J, 1.0);
                        emit(&format!("re={:?}", (a.js_toregexp)(J, -1)));
                    }
                    1 => {
                        (a.js_newobject)(J);
                        emit(&format!("re={:?}", (a.js_toregexp)(J, -1)));
                    }
                    2 => {
                        (a.js_pushundefined)(J);
                        emit(&format!("re={:?}", (a.js_toregexp)(J, -1)));
                    }
                    3 => {
                        (a.js_newregexp)(J, cs("a").as_ptr(), 0);
                        emit(&format!("re_nonnull={}", !(a.js_toregexp)(J, -1).is_null()));
                    }
                    4 => {
                        emit(&format!("re_oob={:?}", (a.js_toregexp)(J, 99)));
                    }
                    /* row 194 js_touserdata */
                    5 => {
                        (a.js_newobject)(J);
                        (a.js_newuserdata)(J, tag, 0x1234 as *mut c_void, Some(ud_fin));
                        emit(&format!("ud={:?}", (a.js_touserdata)(J, -1, other)));
                    }
                    6 => {
                        (a.js_newobject)(J);
                        (a.js_newuserdata)(J, tag, 0x1234 as *mut c_void, Some(ud_fin));
                        emit(&format!("ud={:?}", (a.js_touserdata)(J, -1, tag)));
                    }
                    7 => {
                        (a.js_pushnumber)(J, 1.0);
                        emit(&format!("ud={:?}", (a.js_touserdata)(J, -1, tag)));
                    }
                    8 => {
                        emit(&format!("ud_oob={:?}", (a.js_touserdata)(J, 99, tag)));
                    }
                    /* rows 195/196 jsR_tofunction through js_defaccessor */
                    9 => {
                        (a.js_newobject)(J);
                        (a.js_pushundefined)(J);
                        (a.js_pushundefined)(J);
                        (a.js_defaccessor)(J, -3, cs("p").as_ptr(), 0);
                        emit(&format!("has_p={}", (a.js_hasproperty)(J, -1, cs("p").as_ptr())));
                    }
                    10 => {
                        (a.js_newobject)(J);
                        (a.js_pushnull)(J);
                        (a.js_pushnull)(J);
                        (a.js_defaccessor)(J, -3, cs("p").as_ptr(), 0);
                        emit(&format!("has_p={}", (a.js_hasproperty)(J, -1, cs("p").as_ptr())));
                    }
                    11 => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_defaccessor)(J, -3, cs("p").as_ptr(), 0);
                    }
                    12 => {
                        (a.js_newobject)(J);
                        (a.js_newcfunction)(J, Some(cf_noop), b"g\0".as_ptr() as *const c_char, 0);
                        (a.js_pushstring)(J, cs("nope").as_ptr());
                        (a.js_defaccessor)(J, -3, cs("p").as_ptr(), 0);
                    }
                    _ => {
                        (a.js_newobject)(J);
                        (a.js_newarray)(J);
                        (a.js_pushundefined)(J);
                        (a.js_defaccessor)(J, -3, cs("p").as_ptr(), 0);
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("toregexp/touserdata mode={}", mode), act, f);
        }
    }
    let srcs = [
        "RegExp.prototype.exec.call({}, 'x')",
        "RegExp.prototype.test.call(1, 'x')",
        "({get x(){}}).x",
        "Object.defineProperty({}, 'p', {get: 1})",
        "Object.defineProperty({}, 'p', {set: 'no'})",
        "Object.defineProperty({}, 'p', {get: undefined, set: undefined})",
        "({}) instanceof Object",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("tofunction js", s, f);
        }
    }
}

/* ========================================================================== */
/* rows 247-255: js_call / js_construct / js_eval / js_pcall / js_pconstruct   */
/* ========================================================================== */

unsafe extern "C" fn cf_throw(J: JS) {
    let a = cur();
    unsafe {
        (a.js_newtypeerror)(J, cs("thrown by cf_throw").as_ptr());
        (a.js_throw)(J);
    }
}

#[test]
fn r247_r255_call_construct_errors() {
    for mode in 0i64..14 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                match pi(0) {
                    /* row 247: negative argument count */
                    0 => {
                        (a.js_newcfunction)(J, Some(cf_noop), b"f\0".as_ptr() as *const c_char, 0);
                        (a.js_pushundefined)(J);
                        (a.js_call)(J, -1);
                    }
                    1 => {
                        (a.js_newcfunction)(J, Some(cf_noop), b"f\0".as_ptr() as *const c_char, 0);
                        (a.js_pushundefined)(J);
                        emit(&format!("pcall={}", (a.js_pcall)(J, -1)));
                    }
                    /* row 248: callee not callable */
                    2 => {
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_pushundefined)(J);
                        (a.js_call)(J, 0);
                    }
                    3 => {
                        (a.js_pushundefined)(J);
                        (a.js_pushundefined)(J);
                        (a.js_call)(J, 0);
                    }
                    4 => {
                        (a.js_newobject)(J);
                        (a.js_pushundefined)(J);
                        (a.js_call)(J, 0);
                    }
                    5 => {
                        (a.js_pushstring)(J, cs("s").as_ptr());
                        (a.js_pushundefined)(J);
                        emit(&format!("pcall={} v={:?}", (a.js_pcall)(J, 0), str_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    /* row 250: construct a non-callable */
                    6 => {
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_construct)(J, 0);
                    }
                    7 => {
                        (a.js_newobject)(J);
                        emit(&format!(
                            "pconstruct={} v={:?}",
                            (a.js_pconstruct)(J, 0),
                            str_at(a, J, -1)
                        ));
                        (a.js_pop)(J, 1);
                    }
                    /* rows 254/255: pcall / pconstruct of a throwing callee */
                    8 => {
                        (a.js_newcfunction)(J, Some(cf_throw), b"t\0".as_ptr() as *const c_char, 0);
                        (a.js_pushundefined)(J);
                        emit(&format!("pcall={} v={:?}", (a.js_pcall)(J, 0), str_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    9 => {
                        (a.js_newcfunction)(J, Some(cf_throw), b"t\0".as_ptr() as *const c_char, 0);
                        emit(&format!(
                            "pconstruct={} v={:?}",
                            (a.js_pconstruct)(J, 0),
                            str_at(a, J, -1)
                        ));
                        (a.js_pop)(J, 1);
                    }
                    10 => {
                        /* pcall with extra args on the stack: only the error remains */
                        (a.js_newcfunction)(J, Some(cf_throw), b"t\0".as_ptr() as *const c_char, 0);
                        (a.js_pushundefined)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_pushnumber)(J, 2.0);
                        emit(&format!("pcall3={}", (a.js_pcall)(J, 2)));
                        dump(a, J);
                        (a.js_pop)(J, 1);
                    }
                    /* row 253: js_eval with a non-string on top */
                    11 => {
                        (a.js_pushnumber)(J, 5.0);
                        (a.js_eval)(J);
                        emit(&format!("eval_num={}", repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    12 => {
                        (a.js_newobject)(J);
                        (a.js_eval)(J);
                        emit(&format!("eval_obj={}", repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    _ => {
                        (a.js_pushundefined)(J);
                        (a.js_eval)(J);
                        emit(&format!("eval_undef={}", repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("call/construct mode={}", mode), act, f);
        }
    }
    let srcs = [
        "var x = 1; x()",
        "undefined()",
        "null()",
        "({})()",
        "'abc'()",
        "true()",
        "[]()",
        "/re/()",
        "new 1",
        "new ({})",
        "new undefined",
        "new 'x'",
        "function f(){}; f.prototype=1; String(new f())",
        "function f(){ return 1 }; String(new f())",
        "function f(){ return {a:1} }; new f().a",
        "eval(1)",
        "eval({})",
        "eval()",
        "var o={}; o.nosuchmethod()",
        "Math.nosuch()",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("call js", s, f);
        }
    }
}

/* ========================================================================== */
/* rows 303-306: js_instanceof                                                 */
/* ========================================================================== */

#[test]
fn r303_r306_instanceof_operands() {
    for a_kind in 0i64..7 {
        for b_kind in 0i64..7 {
            set_pi(0, a_kind);
            set_pi(1, b_kind);
            fn push(a: &Api, J: JS, k: i64) {
                unsafe {
                    match k {
                        0 => (a.js_pushundefined)(J),
                        1 => (a.js_pushnull)(J),
                        2 => (a.js_pushnumber)(J, 1.0),
                        3 => (a.js_pushstring)(J, cs("s").as_ptr()),
                        4 => (a.js_newobject)(J),
                        5 => (a.js_newarray)(J),
                        _ => (a.js_newcfunction)(J, Some(cf_noop), b"f\0".as_ptr() as *const c_char, 0),
                    }
                }
            }
            fn act(a: &Api, J: JS) {
                unsafe {
                    push(a, J, pi(0));
                    push(a, J, pi(1));
                    emit(&format!("instanceof={}", (a.js_instanceof)(J)));
                    dump(a, J);
                    (a.js_pop)(J, (a.js_gettop)(J) - 1);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            same(
                &format!("instanceof {} {}", a_kind, b_kind),
                &p.c.run_native(act, 0),
                &p.r.run_native(act, 0),
            );
        }
    }
    let srcs = [
        "({}) instanceof 1",
        "({}) instanceof ({})",
        "({}) instanceof undefined",
        "({}) instanceof null",
        "({}) instanceof 'Object'",
        "1 instanceof Object",
        "'x' instanceof String",
        "null instanceof Object",
        "undefined instanceof Object",
        "function f(){}; f.prototype=1; ({}) instanceof f",
        "function f(){}; f.prototype=null; ({}) instanceof f",
        "function f(){}; f.prototype='x'; ({}) instanceof f",
        "({}) instanceof Array",
        "[] instanceof Array",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("instanceof js", s, f);
        }
    }
}

/* ========================================================================== */
/* rows 322-333: jserror.c — every thrower and constructor                     */
/* ========================================================================== */

#[test]
fn r332_new_error_constructors() {
    /* the js_new*error family builds the object but does not throw */
    for msg_kind in 0i64..4 {
        set_pi(0, msg_kind);
        fn act(a: &Api, J: JS) {
            unsafe {
                let msg = match pi(0) {
                    0 => cs(""),
                    1 => cs("plain message"),
                    2 => cs(&rep300()),
                    _ => cs("with % and %s and \\ and \"quotes\""),
                };
                let ctors: [unsafe extern "C" fn(JS, *const c_char); 7] = [
                    a.js_newerror,
                    a.js_newevalerror,
                    a.js_newrangeerror,
                    a.js_newreferenceerror,
                    a.js_newsyntaxerror,
                    a.js_newtypeerror,
                    a.js_newurierror,
                ];
                for (i, c) in ctors.iter().enumerate() {
                    c(J, msg.as_ptr());
                    emit(&format!("ctor{} repr={}", i, repr_at(a, J, -1)));
                    emit(&format!("ctor{} str={:?}", i, str_at(a, J, -1)));
                    emit(&format!("ctor{} iserror={}", i, (a.js_iserror)(J, -1)));
                    (a.js_getproperty)(J, -1, cs("name").as_ptr());
                    emit(&format!("ctor{} name={:?}", i, str_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                    (a.js_getproperty)(J, -1, cs("message").as_ptr());
                    emit(&format!("ctor{} message={:?}", i, str_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                    emit(&format!(
                        "ctor{} has_stackTrace={}",
                        i,
                        (a.js_hasproperty)(J, -1, cs("stackTrace").as_ptr())
                    ));
                    (a.js_pop)(J, (a.js_gettop)(J) - 2);
                    (a.js_pop)(J, 1);
                }
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("new*error msg={}", msg_kind), act, f);
        }
    }
}

#[test]
fn r332_new_error_then_throw() {
    for which in 0i64..7 {
        set_pi(0, which);
        fn act(a: &Api, J: JS) {
            unsafe {
                let ctors: [unsafe extern "C" fn(JS, *const c_char); 7] = [
                    a.js_newerror,
                    a.js_newevalerror,
                    a.js_newrangeerror,
                    a.js_newreferenceerror,
                    a.js_newsyntaxerror,
                    a.js_newtypeerror,
                    a.js_newurierror,
                ];
                let m = cs("thrown from C");
                ctors[pi(0) as usize](J, m.as_ptr());
                (a.js_throw)(J);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("new*error+throw {}", which), act, f);
        }
    }
}

#[test]
fn r325_r331_variadic_throwers() {
    /* js_error / js_evalerror / js_rangeerror / js_referenceerror /
     * js_syntaxerror / js_typeerror / js_urierror, including the 256-byte
     * vsnprintf truncation of the message buffer. */
    for which in 0i64..7 {
        for fmt_kind in 0i64..5 {
            set_pi(0, which);
            set_pi(1, fmt_kind);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let t = throwers(a.tag);
                    let f: Vfn = match pi(0) {
                        0 => t.error,
                        1 => t.evalerror,
                        2 => t.rangeerror,
                        3 => t.referenceerror,
                        4 => t.syntaxerror,
                        5 => t.typeerror,
                        _ => t.urierror,
                    };
                    emit(&format!("thrower={} fmt={}", pi(0), pi(1)));
                    match pi(1) {
                        0 => {
                            let m = cs("simple");
                            f(J, m.as_ptr());
                        }
                        1 => {
                            let m = cs("");
                            f(J, m.as_ptr());
                        }
                        2 => {
                            let m = cs("num=%d str=%s");
                            let s = cs("arg");
                            f(J, m.as_ptr(), -17 as c_int, s.as_ptr());
                        }
                        3 => {
                            /* 300+ char argument: truncated at 255 chars */
                            let m = cs("'%s' is read-only");
                            let s = cs(&rep300());
                            f(J, m.as_ptr(), s.as_ptr());
                        }
                        _ => {
                            /* a 300+ char literal format with no arguments */
                            let m = cs(&("x".repeat(300)));
                            f(J, m.as_ptr());
                        }
                    }
                    emit("thrower-returned");
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            let c = p.c.run_native(act, 0);
            let r = p.r.run_native(act, 0);
            let label = format!("thrower {} fmt {}", which, fmt_kind);
            same(&label, &c, &r);
            if fmt_kind == 3 {
                /* char buf[256] => the formatted message is cut at 255 bytes:
                 * "'" followed by 254 'p's. */
                assert!(c.contains(&format!("'{}", "p".repeat(254))), "{}: {}", label, c);
                assert!(!c.contains(&"p".repeat(255)), "{}: {}", label, c);
            }
            if fmt_kind == 4 {
                assert!(c.contains(&"x".repeat(255)), "{}: {}", label, c);
                assert!(!c.contains(&"x".repeat(256)), "{}: {}", label, c);
            }
            for fl in [JS_STRICT, 2] {
                diff_native(&label, act, fl);
            }
        }
    }
}

#[test]
fn r322_r324_r333_error_object_shapes() {
    let long = rep300();
    let srcs: Vec<String> = vec![
        /* row 333: new Error() with no argument */
        "var e=new Error(); [('message' in e), e.message, e.hasOwnProperty('message')]".into(),
        "var e=new Error(undefined); [e.message, e.hasOwnProperty('message')]".into(),
        "var e=new Error('m'); [e.message, e.hasOwnProperty('message')]".into(),
        "var e=new TypeError(); String(e)".into(),
        "var e=new RangeError('r'); String(e)".into(),
        "var e=new EvalError('r'); e.name".into(),
        "var e=new ReferenceError('r'); e.name".into(),
        "var e=new SyntaxError('r'); e.name".into(),
        "var e=new URIError('r'); e.name".into(),
        /* row 322: no frames -> no stackTrace */
        "var e=new Error('x'); e.hasOwnProperty('stackTrace')".into(),
        "function g(){ return new Error('x').hasOwnProperty('stackTrace') } g()".into(),
        "function g(){ return new Error('x').stackTrace } g()".into(),
        /* row 324: Error.prototype.toString with a non-object `this` */
        "Error.prototype.toString.call(1)".into(),
        "Error.prototype.toString.call(undefined)".into(),
        "Error.prototype.toString.call(null)".into(),
        "Error.prototype.toString.call('s')".into(),
        "Error.prototype.toString.call({})".into(),
        "Error.prototype.toString.call({name:'',message:''})".into(),
        "Error.prototype.toString.call({name:'N',message:''})".into(),
        "Error.prototype.toString.call({name:'',message:'M'})".into(),
        "var e=new Error('x'); e.stack".into(),
        /* row 323: a >255-byte stack-trace frame line is truncated */
        format!("eval('function {}(){{ return new Error(\"x\").stackTrace }}'); {}()", long, long),
        format!("eval('function {}(){{ throw new Error(\"x\") }}'); try {{ {}() }} catch(e) {{ e.stackTrace }}", long, long),
    ];
    for s in &srcs {
        for f in [0, JS_STRICT] {
            diff_eval("error shapes", s, f);
        }
    }
}

/* ========================================================================== */
/* rows 286-297: jsvalue.c numeric + string conversion sentinels               */
/* ========================================================================== */

#[test]
fn r286_r289_numbertointeger_family() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 7);
    let mut vals: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1e300,
        1e300,
        i32::MIN as f64,
        i32::MAX as f64,
        i32::MIN as f64 - 1.0,
        i32::MAX as f64 + 1.0,
        4294967295.0,
        4294967296.0,
        -0.5,
        0.5,
        65535.5,
        -65536.5,
    ];
    for _ in 0..400 {
        vals.push(rng.f64());
    }
    let mut c = String::new();
    let mut r = String::new();
    for v in vals {
        unsafe {
            c.push_str(&format!(
                "{} {} {} {} {}\n",
                (p.c.jsV_numbertointeger)(v),
                (p.c.jsV_numbertoint32)(v),
                (p.c.jsV_numbertouint32)(v),
                (p.c.jsV_numbertoint16)(v),
                (p.c.jsV_numbertouint16)(v)
            ));
            r.push_str(&format!(
                "{} {} {} {} {}\n",
                (p.r.jsV_numbertointeger)(v),
                (p.r.jsV_numbertoint32)(v),
                (p.r.jsV_numbertouint32)(v),
                (p.r.jsV_numbertoint16)(v),
                (p.r.jsV_numbertouint16)(v)
            ));
        }
    }
    same("jsV_numberto*", &c, &r);
}

#[test]
fn r296_r297_stringtonumber_sentinels() {
    let probes = [
        "", " ", "\t\n", "0", "0x10", "0X10", "0b1", "0o7", "010", ".e5", ".5", "5.", "+", "-",
        "--1", "12abc", "1 2", "1e", "1e+", "Infinity", "-Infinity", "+Infinity", "infinity",
        "NaN", "nan", "  12  ", "  12x", "1.5e3", "0.0", "-0", "1e400", "1e-400",
        "4294967296", "9007199254740993", "0xZZ", "0x", "  0x10  ", "\u{a0}1",
    ];
    for s in probes {
        set_ps(0, s);
        fn act(a: &Api, J: JS) {
            unsafe {
                let s = ps(0);
                emit(&format!(
                    "stringtonumber={:#x}",
                    (a.jsV_stringtonumber)(J, s.as_ptr()).to_bits()
                ));
                let mut ep: *mut c_char = std::ptr::null_mut();
                let v = (a.js_stringtofloat)(s.as_ptr(), &mut ep);
                let consumed = if ep.is_null() {
                    -1
                } else {
                    ep as isize - s.as_ptr() as isize
                };
                emit(&format!("stringtofloat={:#x} consumed={}", v.to_bits(), consumed));
                (a.js_pushstring)(J, s.as_ptr());
                emit(&format!("tonumber={:#x}", (a.js_tonumber)(J, -1).to_bits()));
                emit(&format!("tointeger={}", (a.js_tointeger)(J, -1)));
                (a.js_pop)(J, 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("stringtonumber {:?}", s),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

#[test]
fn r290_r295_toprimitive_refusals() {
    let srcs = [
        "Object.create(null) + 1",
        "String(Object.create(null))",
        "Number(Object.create(null))",
        "'' + Object.create(null)",
        "({toString:function(){return {}}}) + ''",
        "+({valueOf:function(){return {}}})",
        "({valueOf:function(){return {}}, toString:function(){return {}}}) + ''",
        "({toString:null, valueOf:null}) + ''",
        "({toString:1, valueOf:2}) + ''",
        "var o=Object.create(null); o.toString=function(){return 'T'}; o + ''",
        "var o=Object.create(null); o.valueOf=function(){return 7}; o + 1",
        "({toString:function(){throw new Error('ts')}}) + ''",
        "({valueOf:function(){throw new Error('vo')}}) + 1",
        "Object.create(null) < 1",
        "Object.create(null) == 1",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("toprimitive", s, f);
            diff_eval("toprimitive/usestrict", &format!("'use strict'; {}", s), f);
        }
    }
    /* js_toprimitive with in-range and out-of-range hints */
    for hint in [-1i64, 0, 1, 2, 3, 4, 5, 8, 99, -1000, i32::MAX as i64] {
        set_pi(0, hint);
        fn act(a: &Api, J: JS) {
            unsafe {
                for k in 0..8 {
                    match k {
                        0 => (a.js_newobject)(J),
                        1 => (a.js_newarray)(J),
                        2 => (a.js_newnumber)(J, 5.0),
                        3 => (a.js_newstring)(J, cs("s").as_ptr()),
                        4 => (a.js_newboolean)(J, 1),
                        5 => (a.js_newregexp)(J, cs("r").as_ptr(), 0),
                        6 => (a.js_pushnumber)(J, 3.5),
                        _ => (a.js_pushundefined)(J),
                    }
                    (a.js_toprimitive)(J, -1, pic(0));
                    emit(&format!("[{}]={}", k, repr_at(a, J, -1)));
                    /* the value-level entry point too */
                    let v = (a.js_tovalue)(J, -1);
                    (a.jsV_toprimitive)(J, v, pic(0));
                    emit(&format!("v[{}]={}", k, repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                }
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("toprimitive hint={}", hint), act, f);
        }
    }
}

#[test]
fn r308_r310_compare_equal_strictequal() {
    let srcs = [
        "NaN < 1", "NaN > 1", "NaN <= 1", "NaN >= 1",
        "undefined > 0", "undefined < 0", "'x' <= 1", "'x' >= 1",
        "null == 0", "null == undefined", "null === undefined",
        "undefined == 0", "({}) == null", "({}) == undefined",
        "1 === '1'", "1 == '1'", "0 == ''", "0 == '0'", "'' == '0'",
        "[] == false", "[0] == false", "[1] == true",
        "null == false", "undefined == false",
        "NaN == NaN", "NaN === NaN",
        "-0 === 0", "-0 == 0",
        "({}) == ({})", "var o={}; o == o",
        "'a' < 'b'", "'b' < 'a'", "'a' < 'a'",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("compare/equal", s, f);
        }
    }
}

/* ========================================================================== */
/* rows 311-318: jsproperty.c lookups, non-extensible, iterators               */
/* ========================================================================== */

#[test]
fn r311_r318_property_lookups_and_iterators() {
    for class in [0i64, 1, 5, 6, 7, 10, 11, 12, 13] {
        set_pi(0, class);
        fn act(a: &Api, J: JS) {
            unsafe {
                let obj = (a.jsV_newobject)(J, pic(0), std::ptr::null_mut());
                (a.js_pushobject)(J, obj);
                for name in ["nosuch", "", "length", "0", "toString"] {
                    let n = cs(name);
                    let mut own: c_int = -1;
                    emit(&format!(
                        "{:?} own={} get={} x={}/{} enum_via_iter",
                        name,
                        (a.jsV_getownproperty)(J, obj, n.as_ptr()).is_null(),
                        (a.jsV_getproperty)(J, obj, n.as_ptr()).is_null(),
                        (a.jsV_getpropertyx)(J, obj, n.as_ptr(), &mut own).is_null(),
                        own
                    ));
                }
                /* row 317: js_nextiterator on a non-iterator object */
                emit("nextiterator-on-plain-object");
                let s = (a.js_nextiterator)(J, -1);
                emit(&format!("unexpected={:?}", rs(s)));
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("property lookup class={}", class), act, f);
        }
    }

    /* row 318: iterator exhaustion, including names deleted from the target */
    fn act2(a: &Api, J: JS) {
        unsafe {
            (a.js_newobject)(J);
            for n in ["a", "b", "c"] {
                (a.js_pushnumber)(J, 1.0);
                (a.js_setproperty)(J, -2, cs(n).as_ptr());
            }
            (a.js_pushiterator)(J, -2, 1);
            /* delete every property before draining the iterator */
            for n in ["a", "b", "c"] {
                (a.js_delproperty)(J, -2, cs(n).as_ptr());
            }
            let mut k = 0;
            loop {
                let nm = (a.js_nextiterator)(J, -1);
                if nm.is_null() {
                    emit(&format!("exhausted after {}", k));
                    break;
                }
                emit(&format!("it={:?}", rs(nm)));
                k += 1;
                if k > 32 {
                    break;
                }
            }
            /* calling again after exhaustion */
            emit(&format!("again_null={}", (a.js_nextiterator)(J, -1).is_null()));
            (a.js_pop)(J, 2);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("iterator exhaustion", act2, f);
    }

    /* rows 315/316: jsV_setproperty on a non-extensible object */
    fn act3(a: &Api, J: JS) {
        unsafe {
            let src = cs("Object.preventExtensions({keep:1})");
            let nm = cs("ne.js");
            if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) == 0 {
                (a.js_pushundefined)(J);
                if (a.js_pcall)(J, 0) == 0 {
                    let obj = (a.js_toobject)(J, -1);
                    emit(&format!(
                        "keep_null={}",
                        (a.jsV_setproperty)(J, obj, cs("keep").as_ptr()).is_null()
                    ));
                    emit("about-to-add-new-name");
                    emit(&format!(
                        "new_null={}",
                        (a.jsV_setproperty)(J, obj, cs("fresh").as_ptr()).is_null()
                    ));
                }
            }
            emit(&format!("survived top={}", (a.js_gettop)(J)));
            (a.js_pop)(J, (a.js_gettop)(J) - 1);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("non-extensible jsV_setproperty", act3, f);
    }

    let srcs = [
        "var o=Object.preventExtensions({}); o.x=1; o.x",
        "var o=Object.preventExtensions({}); Object.isExtensible(o)",
        "var it=[]; for (var k in Object.create(null)) it.push(k); it.length",
        "var o={a:1}; var s=''; for (var k in o) { delete o.a; s+=k } s",
        "var o=Object.create({p:1}); var s=''; for (var k in o) s+=k; s",
        "var a=[1,2]; var s=''; for (var k in a) s+=k; s",
        "var s=''; for (var k in 'ab') s+=k; s",
        "var s=''; for (var k in new String('ab')) s+=k; s",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("iterator js", s, f);
        }
    }
}

/* ========================================================================== */
/* rows 205-212, 266-268: index/name classification + hasproperty misses       */
/* ========================================================================== */

#[test]
fn r205_r212_r266_r268_index_classification() {
    let srcs = [
        "var a={}; a['']=1; a['']",
        "var a={}; a['01']=1; [a['01'], a[1]]",
        "var a={}; a['0x']=1; a['0x']",
        "var a={}; a['1a']=1; a['1a']",
        "var a={}; a['-1']=1; a['-1']",
        "var a={}; a['1.5']=1; a['1.5']",
        "var a={}; a['4294967296']=1; a['4294967296']",
        "var a={}; a['99999999999']=1; a['99999999999']",
        "var a=[1,2]; [a[7], a[-1], a[1.5], a['1']]",
        "({}).nosuch",
        "({}).nosuch === undefined",
        "'abc'[10]",
        "'abc'[-1]",
        "new String('abc')[10]",
        "new String('abc')[1]",
        "var s=''; for (var k in 1) s+=k; s",
        "var s=''; for (var k in true) s+=k; s",
        "var s=''; for (var k in undefined) s+='X'; s",
        "var s=''; for (var k in null) s+='X'; s",
        "var a=[1,2]; a.length=5; [a.length, a[4], 4 in a]",
    ];
    for s in srcs {
        for f in [0, JS_STRICT] {
            diff_eval("index classification", s, f);
        }
    }
    /* js_isarrayindex directly on the boundary values */
    for s in ["", "0", "01", "0x", "1a", "-1", "1.5", "214748364", "2147483647",
              "2147483648", "4294967296", "99999999999", "999999999999999999999"] {
        set_ps(0, s);
        fn act(a: &Api, J: JS) {
            unsafe {
                let s = ps(0);
                let mut idx: c_int = -12345;
                emit(&format!(
                    "{} idx={}",
                    (a.js_isarrayindex)(J, s.as_ptr(), &mut idx),
                    idx
                ));
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        same(
            &format!("isarrayindex {:?}", s),
            &p.c.run_native(act, 0),
            &p.r.run_native(act, 0),
        );
    }
}

/* ========================================================================== */
/* rows 172-177, 260, 301, 302, 307, 321, 338: js_setlimit                     */
/* ========================================================================== */

#[test]
fn r172_r177_r260_setlimit_run_and_mem() {
    let limits: [(i64, i64); 14] = [
        (0, 0),
        (-1, -1),
        (1, 0),
        (2, 0),
        (3, 0),
        (50, 0),
        (0, 1),
        (0, 8),
        (0, 64),
        (0, 1024),
        (0, 1 << 16),
        (-1, 64),
        (1, 64),
        (i32::MAX as i64, i32::MAX as i64),
    ];
    for (rl, ml) in limits {
        for op in 0i64..7 {
            set_pi(0, rl);
            set_pi(1, ml);
            set_pi(2, op);
            fn act(a: &Api, J: JS) {
                unsafe {
                    (a.js_setlimit)(J, pic(0), pic(1));
                    match pi(2) {
                        0 => {
                            let src = cs("var s=0; for (var i=0;i<500;i++) s+=i; s");
                            run_src(a, J, &src, "loop");
                        }
                        1 => {
                            let src = cs("var s='x'; for (var i=0;i<8;i++) s=s+s; s.length");
                            run_src(a, J, &src, "concat");
                        }
                        2 => {
                            let src = cs("var a=[]; for(var i=0;i<300;i++) a[i]=i; a.length");
                            run_src(a, J, &src, "array");
                        }
                        3 => {
                            /* row 307: js_concat's own cleanup handler */
                            (a.js_pushstring)(J, cs("aaaaaaaaaaaaaaaaaaaa").as_ptr());
                            (a.js_pushstring)(J, cs("bbbbbbbbbbbbbbbbbbbb").as_ptr());
                            (a.js_concat)(J);
                            emit(&format!("concat={}", repr_at(a, J, -1)));
                            (a.js_pop)(J, 1);
                        }
                        4 => {
                            /* rows 172/174: js_malloc / js_realloc directly */
                            let p1 = (a.js_malloc)(J, 4096);
                            emit(&format!("malloc={}", !p1.is_null()));
                            let p2 = (a.js_realloc)(J, p1, 65536);
                            emit(&format!("realloc={}", !p2.is_null()));
                            (a.js_free)(J, p2);
                        }
                        5 => {
                            /* row 321: js_putc buffer growth through js_intern */
                            for k in 0..40 {
                                let s = cs(&format!("interned-name-number-{}", k));
                                let _ = (a.js_intern)(J, s.as_ptr());
                            }
                            emit("interned");
                        }
                        _ => {
                            /* rows 301/302: js_newcfunctionx / js_newuserdatax with a
                             * finalize callback that must run on allocation failure */
                            (a.js_newcfunctionx)(
                                J,
                                Some(cf_noop),
                                b"cfx\0".as_ptr() as *const c_char,
                                0,
                                0xD00D as *mut c_void,
                                Some(ud_fin),
                            );
                            emit("cfunctionx-ok");
                            (a.js_newobject)(J);
                            (a.js_newuserdatax)(
                                J,
                                TAG_FOO.as_ptr() as *const c_char,
                                0xBEEF as *mut c_void,
                                None,
                                None,
                                None,
                                Some(ud_fin),
                            );
                            emit("userdatax-ok");
                            (a.js_pop)(J, 2);
                        }
                    }
                    emit(&format!("survived top={}", (a.js_gettop)(J)));
                    (a.js_pop)(J, (a.js_gettop)(J) - 1);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            same(
                &format!("setlimit({},{}) op={}", rl, ml, op),
                &p.c.run_native(act, 0),
                &p.r.run_native(act, 0),
            );
        }
    }
}

/* ========================================================================== */
/* out-of-range enum values across the FFI boundary                            */
/* ========================================================================== */

#[test]
fn ffi_newstate_flags_out_of_range() {
    let srcs = [
        "x = 1; x",
        "var o=Object.freeze({a:1}); o.a=2; o.a",
        "nosuch = 1",
        "(function(){ return typeof this })()",
        "var s=new String('ab'); s.length=3; s.length",
        "delete [].length",
    ];
    for f in FLAGSETS {
        for s in srcs {
            diff_eval("newstate flags", s, f);
        }
    }
    /* the flag word also reaches jsC_compilescript via default_strict */
    for f in FLAGSETS {
        set_pi(0, f as i64);
        fn act(a: &Api, J: JS) {
            unsafe {
                emit(&format!("flags={}", pi(0)));
                let src = cs("nosuchvar = 1");
                run_src(a, J, &src, "assign");
                (a.js_pushnumber)(J, 0.0);
            }
        }
        diff_native(&format!("flags={}", f), act, f);
    }
}

#[test]
fn ffi_attribute_values_out_of_range() {
    for atts in [8i64, 9, 15, 16, 255, 256, -1, -8, i32::MAX as i64, i32::MIN as i64] {
        set_pi(0, atts);
        fn act(a: &Api, J: JS) {
            unsafe {
                let atts = pic(0);
                (a.js_newobject)(J);
                (a.js_pushnumber)(J, 1.0);
                (a.js_defproperty)(J, -2, cs("x").as_ptr(), atts);
                emit(&format!("has_x={}", (a.js_hasproperty)(J, -1, cs("x").as_ptr())));
                (a.js_pop)(J, (a.js_gettop)(J) - 2);
                (a.js_pushnumber)(J, 2.0);
                (a.js_setproperty)(J, -2, cs("x").as_ptr());
                (a.js_getproperty)(J, -1, cs("x").as_ptr());
                emit(&format!("x={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                emit(&format!("del={}", {
                    (a.js_delproperty)(J, -1, cs("x").as_ptr());
                    (a.js_hasproperty)(J, -1, cs("x").as_ptr())
                }));
                (a.js_pop)(J, (a.js_gettop)(J) - 2);
                /* enumerability */
                (a.js_pushiterator)(J, -1, 1);
                let mut k = 0;
                loop {
                    let nm = (a.js_nextiterator)(J, -1);
                    if nm.is_null() {
                        break;
                    }
                    emit(&format!("it={:?}", rs(nm)));
                    k += 1;
                    if k > 8 {
                        break;
                    }
                }
                (a.js_pop)(J, 1);
                /* accessors + globals with the same attribute word */
                (a.js_newcfunction)(J, Some(cf_noop), b"g\0".as_ptr() as *const c_char, 0);
                (a.js_pushundefined)(J);
                (a.js_defaccessor)(J, -3, cs("acc").as_ptr(), atts);
                (a.js_pop)(J, (a.js_gettop)(J) - 2);
                (a.js_pushnumber)(J, 3.0);
                (a.js_defglobal)(J, cs("gv").as_ptr(), atts);
                (a.js_getglobal)(J, cs("gv").as_ptr());
                emit(&format!("gv={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("atts={}", atts), act, f);
        }
    }
}

#[test]
fn ffi_regexp_flags_out_of_range() {
    for flags in [0i64, 7, 8, 9, 255, 256, -1, i32::MAX as i64, i32::MIN as i64] {
        set_pi(0, flags);
        fn act(a: &Api, J: JS) {
            unsafe {
                (a.js_newregexp)(J, cs("a(b)c").as_ptr(), pic(0));
                emit(&format!("re={}", repr_at(a, J, -1)));
                emit(&format!("isregexp={}", (a.js_isregexp)(J, -1)));
                for k in ["source", "global", "ignoreCase", "multiline", "lastIndex"] {
                    (a.js_getproperty)(J, -1, cs(k).as_ptr());
                    emit(&format!("{}={}", k, repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                }
                (a.js_copy)(J, -1);
                (a.js_setglobal)(J, cs("RE").as_ptr());
                let src = cs("[String(RE.exec('xxabcabc')), RE.lastIndex, RE.test('abc'), RE.lastIndex, 'AbC'.replace(RE,'!')]");
                run_src(a, J, &src, "drive");
                (a.js_pop)(J, 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("regexp flags={}", flags), act, f);
        }
    }
}

#[test]
fn ffi_jsV_newobject_class_out_of_range() {
    for class in [16i64, 17, 99, 255, -1, -99, i32::MAX as i64, i32::MIN as i64] {
        set_pi(0, class);
        fn act(a: &Api, J: JS) {
            unsafe {
                let obj = (a.jsV_newobject)(J, pic(0), std::ptr::null_mut());
                emit(&format!("nonnull={}", !obj.is_null()));
                (a.js_pushobject)(J, obj);
                emit(&format!(
                    "type={} typeof={} isobject={} isarray={} isregexp={} iscallable={} iserror={}",
                    (a.js_type)(J, -1),
                    rs((a.js_typeof)(J, -1)),
                    (a.js_isobject)(J, -1),
                    (a.js_isarray)(J, -1),
                    (a.js_isregexp)(J, -1),
                    (a.js_iscallable)(J, -1),
                    (a.js_iserror)(J, -1),
                ));
                /* the repr switch falls into its `default:` branch for unknown classes */
                emit(&format!("repr={}", repr_at(a, J, -1)));
                emit(&format!("str={:?}", str_at(a, J, -1)));
                (a.js_pushnumber)(J, 1.0);
                (a.js_setproperty)(J, -2, cs("p").as_ptr());
                (a.js_getproperty)(J, -1, cs("p").as_ptr());
                emit(&format!("p={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                (a.js_delproperty)(J, -1, cs("p").as_ptr());
                emit(&format!("len={}", (a.js_getlength)(J, -1)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_gc)(J, 0);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("jsV_newobject class={}", class), act, f);
        }
    }
}

/* ========================================================================== */
/* NULL pointer arguments the C accepts                                        */
/* ========================================================================== */

#[test]
fn null_pointer_arguments() {
    fn act(a: &Api, J: JS) {
        unsafe {
            /* js_setreport(J, NULL): js_report becomes a no-op */
            (a.js_setreport)(J, None);
            (a.js_report)(J, cs("dropped").as_ptr());
            emit("report-with-null-handler-ok");

            /* js_atpanic(J, NULL): installed but never triggered here */
            let old = (a.js_atpanic)(J, None);
            emit(&format!("old_panic_none={}", old.is_none()));
            let back = (a.js_atpanic)(J, old);
            emit(&format!("back_none={}", back.is_none()));

            /* js_newcfunction with a NULL function pointer, never called */
            (a.js_newcfunction)(J, None, b"nullfn\0".as_ptr() as *const c_char, 0);
            emit(&format!(
                "nullfn callable={} repr={}",
                (a.js_iscallable)(J, -1),
                repr_at(a, J, -1)
            ));
            (a.js_pop)(J, 1);

            /* js_newuserdatax with all-NULL callbacks */
            (a.js_newobject)(J);
            (a.js_newuserdatax)(
                J,
                TAG_NULL.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                None,
                None,
                None,
                None,
            );
            emit(&format!(
                "ud data={:?} isud={} has={}",
                (a.js_touserdata)(J, -1, TAG_NULL.as_ptr() as *const c_char),
                (a.js_isuserdata)(J, -1, TAG_NULL.as_ptr() as *const c_char),
                (a.js_hasproperty)(J, -1, cs("zap").as_ptr())
            ));
            (a.js_pop)(J, (a.js_gettop)(J) - 1);
            (a.js_pushnumber)(J, 1.0);
            (a.js_setproperty)(J, -2, cs("zap").as_ptr());
            (a.js_delproperty)(J, -1, cs("zap").as_ptr());
            (a.js_pop)(J, 1);

            /* js_regcomp / js_regexec with a NULL Resub */
            let mut err: *const c_char = std::ptr::null();
            let pat = cs("a(b)c");
            let prog = (a.js_regcomp)(pat.as_ptr(), 0, &mut err);
            emit(&format!("regcomp nonnull={} err={:?}", !prog.is_null(), rs(err)));
            if !prog.is_null() {
                let sub = cs("xxabcxx");
                emit(&format!(
                    "regexec_null_sub={}",
                    (a.js_regexec)(prog, sub.as_ptr(), std::ptr::null_mut(), 0)
                ));
                let nomatch = cs("zzz");
                emit(&format!(
                    "regexec_null_sub_nomatch={}",
                    (a.js_regexec)(prog, nomatch.as_ptr(), std::ptr::null_mut(), 0)
                ));
                let mut m = Resub::default();
                emit(&format!(
                    "regexec_sub={} nsub={}",
                    (a.js_regexec)(prog, sub.as_ptr(), &mut m, 0),
                    m.nsub
                ));
                (a.js_regfree)(prog);
            }
            /* js_regcomp of an invalid pattern returns NULL + an error string */
            let bad = cs("a(");
            let prog2 = (a.js_regcomp)(bad.as_ptr(), 0, &mut err);
            emit(&format!("regcomp_bad null={} err={:?}", prog2.is_null(), rs(err)));

            /* row 337: js_freestate(NULL) is a documented no-op */
            (a.js_freestate)(std::ptr::null_mut());
            emit("freestate-null-ok");
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("null pointer args", act, f);
    }
}

/* rows 190, 191: js_currentfunction / js_currentfunctiondata with BOT == 0.
 * A brand-new state has no active call frame, so this is only reachable on a
 * secondary state created inside the action. */
#[test]
fn r190_r191_currentfunction_without_frame() {
    for sub_flags in FLAGSETS {
        set_pi(0, sub_flags as i64);
        fn act(a: &Api, J: JS) {
            unsafe {
                let J2 = (a.js_newstate)(None, std::ptr::null_mut(), pic(0));
                emit(&format!("J2_nonnull={}", !J2.is_null()));
                if !J2.is_null() {
                    emit(&format!("top0={}", (a.js_gettop)(J2)));
                    emit(&format!("data={:?}", (a.js_currentfunctiondata)(J2)));
                    (a.js_currentfunction)(J2);
                    emit(&format!("top1={}", (a.js_gettop)(J2)));
                    emit(&format!("cur={}", repr_at(a, J2, -1)));
                    emit(&format!("isundefined={}", (a.js_isundefined)(J2, -1)));
                    (a.js_pop)(J2, 1);
                    /* js_gc on a state that has only the bootstrap objects */
                    (a.js_gc)(J2, 0);
                    (a.js_freestate)(J2);
                }
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        let r = p.r.run_native(act, 0);
        same(&format!("currentfunction BOT=0 flags={}", sub_flags), &c, &r);
        /* row 190: with BOT == 0 js_currentfunction pushes undefined;
         * row 191: js_currentfunctiondata returns NULL */
        assert!(c.contains("cur=undefined"), "{}", c);
        assert!(c.contains("data=0x0"), "{}", c);
    }
}

/* row 338: js_setlimit never validates anything */
#[test]
fn r338_setlimit_accepts_anything() {
    for (rl, ml) in [
        (i32::MIN as i64, i32::MIN as i64),
        (-1, 0),
        (0, -1),
        (i32::MAX as i64, 0),
        (0, i32::MAX as i64),
    ] {
        set_pi(0, rl);
        set_pi(1, ml);
        fn act(a: &Api, J: JS) {
            unsafe {
                (a.js_setlimit)(J, pic(0), pic(1));
                emit("setlimit-returned");
                (a.js_setlimit)(J, 0, 0);
                let src = cs("var s=0; for(var i=0;i<20;i++) s+=i; s");
                run_src(a, J, &src, "after");
                (a.js_pushnumber)(J, 0.0);
            }
        }
        diff_native(&format!("setlimit({},{})", rl, ml), act, 0);
    }
}

/* rows 334-336: js_gc report + gc triggered from the interpreter loop */
#[test]
fn r335_r336_gc_report_and_trigger() {
    let p = libs();
    for report in [0i64, 1, 2, -1] {
        set_pi(0, report);
        fn act(a: &Api, J: JS) {
            unsafe {
                /* js_gc's report goes through J->report, whose default writes to
                 * stderr; route it into the diffed transcript instead. */
                (a.js_setreport)(J, Some(rep_emit));
                let src = cs("var a=[]; for(var i=0;i<400;i++) a.push({x:i,s:'s'+i}); a.length");
                run_src(a, J, &src, "alloc");
                (a.js_gc)(J, pic(0));
                (a.js_gc)(J, pic(0));
                emit("gc-done");
                (a.js_pushnumber)(J, 0.0);
            }
        }
        /* The gc report prints allocator-dependent byte counts, so only the
         * shape (non-digit characters) is compared. libtest's own progress
         * lines can land in the captured fd when tests run in parallel; they
         * are not produced by the library and are filtered out. */
        let strip = |s: &str| {
            s.lines()
                .filter(|l| !l.starts_with("test ") && !l.contains("test result"))
                .map(|l| l.chars().filter(|c| !c.is_ascii_digit()).collect::<String>())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let mut oc = String::new();
        let mut or_ = String::new();
        let c = capture_stdout(|| oc = p.c.run_native(act, 0));
        let r = capture_stdout(|| or_ = p.r.run_native(act, 0));
        /* nothing of this reaches fd 1 in either library */
        same(&format!("js_gc({}) stdout", report), &strip(&c), &strip(&r));
        same(&format!("js_gc({}) result", report), &oc, &or_);
        if report != 0 {
            assert!(oc.contains("garbage collected"), "report={}: {}", report, oc);
        } else {
            assert!(!oc.contains("garbage collected"), "report={}: {}", report, oc);
        }
    }
}

/* ========================================================================== */
/* rows 175, 177, 283, 284, 285: allocator failures                            */
/* ========================================================================== */

use std::sync::atomic::{AtomicI64, Ordering};

static BUDGET: AtomicI64 = AtomicI64::new(0);

extern "C" {
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// Fails every allocation (row 283).
unsafe extern "C" fn null_alloc(_ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    if n == 0 {
        free(p);
    }
    std::ptr::null_mut()
}

/// Succeeds for the first `BUDGET` requests, then fails (rows 175/177/284/285).
unsafe extern "C" fn budget_alloc(_ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    if n == 0 {
        free(p);
        return std::ptr::null_mut();
    }
    if BUDGET.fetch_sub(1, Ordering::SeqCst) <= 0 {
        return std::ptr::null_mut();
    }
    realloc(p, n as usize)
}

#[test]
fn r283_r285_newstate_allocator_failures() {
    /* row 283: the very first allocation fails */
    fn act_null(a: &Api, J: JS) {
        unsafe {
            let J2 = (a.js_newstate)(Some(null_alloc), std::ptr::null_mut(), 0);
            emit(&format!("null_alloc_state_null={}", J2.is_null()));
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("newstate null alloc", act_null, f);
    }

    /* rows 284/285: the allocation budget runs out at some point during the
     * bootstrap; both libraries must fail (or succeed) at the same budget. */
    for budget in [
        0i64, 1, 2, 3, 4, 5, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192,
    ] {
        set_pi(0, budget);
        fn act(a: &Api, J: JS) {
            unsafe {
                BUDGET.store(pi(0), Ordering::SeqCst);
                let J2 = (a.js_newstate)(Some(budget_alloc), std::ptr::null_mut(), 0);
                BUDGET.store(i64::MAX / 2, Ordering::SeqCst);
                emit(&format!("budget={} null={}", pi(0), J2.is_null()));
                if !J2.is_null() {
                    /* rows 175/177: js_malloc / js_realloc get NULL back on an
                     * already-bootstrapped state; the throw is caught by pcall */
                    BUDGET.store(20, Ordering::SeqCst);
                    let nm = cs("oom.js");
                    let src = cs("var a=[]; for (var i=0;i<300;i++) a.push({k:i,s:'v'+i}); a.length");
                    let rc = (a.js_ploadstring)(J2, nm.as_ptr(), src.as_ptr());
                    emit(&format!("oom_load={}", rc));
                    if rc == 0 {
                        (a.js_pushundefined)(J2);
                        let rc = (a.js_pcall)(J2, 0);
                        BUDGET.store(i64::MAX / 2, Ordering::SeqCst);
                        emit(&format!("oom_call={} v={:?}", rc, str_at(a, J2, -1)));
                    } else {
                        BUDGET.store(i64::MAX / 2, Ordering::SeqCst);
                        emit(&format!("oom_loaderr={:?}", str_at(a, J2, -1)));
                    }
                    (a.js_pop)(J2, 1);
                    (a.js_freestate)(J2);
                }
                BUDGET.store(i64::MAX / 2, Ordering::SeqCst);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        let r = p.r.run_native(act, 0);
        same(&format!("newstate budget={}", budget), &c, &r);
        if budget == 0 {
            assert!(c.contains("null=true"), "budget 0: {}", c);
        }
    }
}

/* ========================================================================== */
/* ground truth: the exact C message text of every reachable thrower           */
/* ========================================================================== */

#[test]
fn ground_truth_messages_js() {
    /* (source, flags, expected substring of the rendered C result) */
    let cases: &[(&str, c_int, &str)] = &[
        ("var a=[]; a.length=1.5", 0, "invalid array length"),
        ("var a=[]; a.length=-1", 0, "invalid array length"),
        ("var a=[]; a.length=67108865", 0, "array too large"),
        ("'use strict'; var s=new String('ab'); s.length=5", 0, "'length' is read-only"),
        ("'use strict'; var s=new String('ab'); s[0]='z'", 0, "'0' is read-only"),
        ("'use strict'; /a/.source='b'", 0, "'source' is read-only"),
        ("'use strict'; /a/.global=true", 0, "'global' is read-only"),
        ("'use strict'; /a/.ignoreCase=true", 0, "'ignoreCase' is read-only"),
        ("'use strict'; /a/.multiline=true", 0, "'multiline' is read-only"),
        (
            "'use strict'; var o={get x(){return 1}}; o.x=2",
            0,
            "setting property 'x' that only has a getter",
        ),
        (
            "'use strict'; var o={}; Object.defineProperty(o,'x',{value:1,writable:false}); o.x=2",
            0,
            "'x' is read-only",
        ),
        /* NOTE: the `transient` TypeError of jsR_setproperty (rows 227/228) is
         * NOT reachable from JS source: OP_SETPROP does
         *     obj = js_toobject(J, -3); transient = !js_isobject(J, -3);
         * and js_toobject rewrites the stack slot into an object *in place*, so
         * the following js_isobject is always true and transient is always 0.
         * The assignment therefore silently targets the temporary wrapper. */
        ("'use strict'; 'abc'.foo=1", 0, "ok 1"),
        ("'use strict'; 'abc'.foo=1; 'abc'.foo", 0, "ok undefined"),
        (
            "'use strict'; var o=Object.preventExtensions({}); o.x=1",
            0,
            "object is non-extensible",
        ),
        (
            "Object.defineProperty([], 'length', {value:3})",
            0,
            "'length' is read-only or non-configurable",
        ),
        (
            "Object.defineProperty(new String('ab'), '0', {value:'z'})",
            0,
            "'0' is read-only or non-configurable",
        ),
        (
            "Object.defineProperty(/a/, 'lastIndex', {value:1})",
            0,
            "'lastIndex' is read-only or non-configurable",
        ),
        ("'use strict'; delete [].length", 0, "'length' is non-configurable"),
        (
            "'use strict'; var s=new String('ab'); delete s[0]",
            0,
            "'0' is non-configurable",
        ),
        ("'use strict'; delete /a/.source", 0, "'source' is non-configurable"),
        /* `delete <identifier>` is a strict-mode SyntaxError, so the global
         * JS_DONTCONF branch of jsR_delproperty is only observable non-strict
         * from JS (false) — the TypeError form is checked natively below. */
        ("delete Math", 0, "ok true"),
        ("delete NaN", 0, "ok false"),
        ("delete Infinity", 0, "ok false"),
        ("'use strict'; delete NaN", 0, "delete on an unqualified name"),
        (
            "'use strict'; undeclaredzz = 1",
            0,
            "assignment to undeclared variable 'undeclaredzz'",
        ),
        ("nosuchvariable", 0, "'nosuchvariable' is not defined"),
        ("'x' in 1", 0, "operand to 'in' is not an object"),
        ("undefined.x", 0, "cannot convert undefined to object"),
        ("null.x", 0, "cannot convert null to object"),
        ("var x=1; x()", 0, "number is not callable"),
        ("undefined()", 0, "undefined is not callable"),
        ("({})()", 0, "object is not callable"),
        ("new 1", 0, "number is not callable"),
        ("({}) instanceof 1", 0, "instanceof: invalid operand"),
        (
            "function f(){}; f.prototype=1; ({}) instanceof f",
            0,
            "instanceof: 'prototype' property is not an object",
        ),
        ("Error.prototype.toString.call(1)", 0, "not an object"),
        (
            "'use strict'; var o=Object.create(null); o+1",
            0,
            "cannot convert object to primitive",
        ),
        ("String(Object.create(null))", 0, "[object]"),
        ("function f(){ return f() } f()", 0, "call stack overflow"),
    ];
    on_big_stack(move || {
        let p = libs();
        for (src, f, want) in cases {
            let c = p.c.eval(src, *f);
            let r = p.r.eval(src, *f);
            same(&format!("ground truth {:?}", src), &c, &r);
            assert!(
                c.contains(want),
                "src={:?} expected {:?} in C output {:?}",
                src,
                want,
                c
            );
        }
    });
}

/* (expected message, js_newstate flags) */
const NATIVE_MSGS: &[(&str, c_int)] = &[
    ("not a regexp", 0),
    ("not a Bar", 0),
    ("not a function", 0),
    ("number of arguments cannot be negative", 0),
    ("not an iterator", 0),
    ("script ran too long", 0),
    ("out of memory", 0),
    ("stack underflow!", 0),
    ("stack error!", 0),
    ("not implemented yet", 0),
    ("stack overflow", 0),
    ("invalid string length", 0),
    /* rows 227/228 — only reachable through the C API, where the `transient`
     * flag is computed from the untouched stack slot */
    ("cannot create property 'foo' on transient object", JS_STRICT),
    ("'length' is read-only", JS_STRICT),
    ("'length' is read-only or non-configurable", 0),
    ("'x' is non-configurable", JS_STRICT),
    ("cannot convert undefined to object", 0),
    ("cannot convert null to object", 0),
    ("exception stack overflow", 0),
    ("'NaN' is non-configurable", JS_STRICT),
];

/* js_newuserdata stores the tag POINTER (it does not copy the bytes), so the
 * tag must outlive the state: only static strings are safe. */
static TAG_FOO: &[u8] = b"Foo\0";
static TAG_BAR: &[u8] = b"Bar\0";
static TAG_NULL: &[u8] = b"NullTag\0";

#[test]
fn ground_truth_messages_native() {
    for (mode, (want, flags)) in NATIVE_MSGS.iter().enumerate() {
        set_pi(0, mode as i64);
        fn act(a: &Api, J: JS) {
            unsafe {
                match pi(0) {
                    0 => {
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_toregexp)(J, -1);
                    }
                    1 => {
                        (a.js_newobject)(J);
                        (a.js_newuserdata)(
                            J,
                            TAG_FOO.as_ptr() as *const c_char,
                            0x1 as *mut c_void,
                            None,
                        );
                        (a.js_touserdata)(J, -1, TAG_BAR.as_ptr() as *const c_char);
                    }
                    2 => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_defaccessor)(J, -3, cs("p").as_ptr(), 0);
                    }
                    3 => {
                        (a.js_newcfunction)(J, Some(cf_noop), b"f\0".as_ptr() as *const c_char, 0);
                        (a.js_pushundefined)(J);
                        (a.js_call)(J, -1);
                    }
                    4 => {
                        (a.js_newobject)(J);
                        (a.js_nextiterator)(J, -1);
                    }
                    5 => {
                        (a.js_setlimit)(J, 1, 0);
                        let src = cs("var s=0; for(var i=0;i<100;i++) s+=i; s");
                        run_src(a, J, &src, "runlimit");
                    }
                    6 => {
                        (a.js_setlimit)(J, 0, 8);
                        let q = (a.js_malloc)(J, 4096);
                        emit(&format!("malloc={}", !q.is_null()));
                    }
                    7 => (a.js_pop)(J, 99),
                    8 => (a.js_remove)(J, 99),
                    9 => (a.js_insert)(J, 0),
                    10 => {
                        for _ in 0..6000 {
                            (a.js_pushundefined)(J);
                        }
                    }
                    11 => {
                        (a.js_pushlstring)(J, cs("abcd").as_ptr(), (1 << 28) + 1);
                    }
                    12 => {
                        (a.js_pushstring)(J, cs("abc").as_ptr());
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setproperty)(J, -2, cs("foo").as_ptr());
                    }
                    13 => {
                        (a.js_newstring)(J, cs("ab").as_ptr());
                        (a.js_pushnumber)(J, 5.0);
                        (a.js_setproperty)(J, -2, cs("length").as_ptr());
                    }
                    14 => {
                        (a.js_newarray)(J);
                        (a.js_pushnumber)(J, 5.0);
                        (a.js_defproperty)(J, -2, cs("length").as_ptr(), 0);
                    }
                    15 => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs("x").as_ptr(), JS_DONTCONF);
                        (a.js_delproperty)(J, -1, cs("x").as_ptr());
                    }
                    16 => {
                        (a.js_pushundefined)(J);
                        (a.js_toobject)(J, -1);
                    }
                    17 => {
                        (a.js_pushnull)(J);
                        (a.js_toobject)(J, -1);
                    }
                    19 => {
                        (a.js_pushglobal)(J);
                        (a.js_delproperty)(J, -1, cs("NaN").as_ptr());
                    }
                    _ => {
                        /* JS_TRYLIMIT via nested JS try blocks */
                        let mut s = String::from("var r='none';");
                        for _ in 0..70 {
                            s.push_str("try{");
                        }
                        s.push_str("r='deep';");
                        for _ in 0..70 {
                            s.push_str("}catch(e){r=String(e)}");
                        }
                        s.push_str("r");
                        let src = cs(&s);
                        run_src(a, J, &src, "trylimit");
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        let c = p.c.run_native(act, *flags);
        let r = p.r.run_native(act, *flags);
        same(&format!("native ground truth mode={}", mode), &c, &r);
        assert!(c.contains(want), "mode={} expected {:?} in {:?}", mode, want, c);
    }
}

/* ========================================================================== */
/* randomized coverage over the property / index API (fixed seed)              */
/* ========================================================================== */

#[test]
fn randomized_property_and_index_ops() {
    let mut rng = Rng::new(SEED ^ 0xABCD);
    let names = [
        "", "a", "length", "0", "1", "01", "-1", "1.5", "4294967296", "source", "global",
        "lastIndex", "toString", "prototype", "constructor", "x", "99999999999",
    ];
    for iter in 0..600 {
        let recv = rng.range_i64(0, 9);
        let op = rng.range_i64(0, 11);
        let name = names[rng.below(names.len() as u64) as usize];
        let idx = rng.range_i64(-4, 4);
        let atts = rng.range_i64(-1, 8);
        let val = rng.range_i64(0, 6);
        set_pi(0, recv);
        set_pi(1, op);
        set_pi(2, atts * 100 + val); /* packed */
        set_pi(3, idx);
        set_ps(0, name);
        fn push_recv(a: &Api, J: JS, k: i64) {
            unsafe {
                match k {
                    0 => (a.js_newobject)(J),
                    1 => (a.js_newarray)(J),
                    2 => (a.js_newstring)(J, cs("abc").as_ptr()),
                    3 => (a.js_newregexp)(J, cs("a").as_ptr(), JS_REGEXP_G),
                    4 => (a.js_pushstring)(J, cs("abc").as_ptr()),
                    5 => (a.js_pushnumber)(J, 5.0),
                    6 => (a.js_pushglobal)(J),
                    7 => (a.js_newcfunction)(J, Some(cf_noop), b"f\0".as_ptr() as *const c_char, 0),
                    8 => (a.js_newnumber)(J, 3.0),
                    _ => (a.js_newarguments)(J),
                }
            }
        }
        fn push_val(a: &Api, J: JS, k: i64) {
            unsafe {
                match k {
                    0 => (a.js_pushnumber)(J, 7.0),
                    1 => (a.js_pushstring)(J, cs("v").as_ptr()),
                    2 => (a.js_pushundefined)(J),
                    3 => (a.js_pushnull)(J),
                    4 => (a.js_pushboolean)(J, 1),
                    5 => (a.js_newobject)(J),
                    _ => (a.js_pushnumber)(J, -1.5),
                }
            }
        }
        fn act(a: &Api, J: JS) {
            unsafe {
                let packed = pi(2);
                let atts = (packed / 100) as c_int;
                let val = packed % 100;
                let name = ps(0);
                let idx = pic(3);
                push_recv(a, J, pi(0));
                match pi(1) {
                    0 => {
                        (a.js_getproperty)(J, -1, name.as_ptr());
                        emit(&format!("get={}", repr_at(a, J, -1)));
                    }
                    1 => {
                        push_val(a, J, val);
                        (a.js_setproperty)(J, -2, name.as_ptr());
                        emit("set-ok");
                    }
                    2 => {
                        push_val(a, J, val);
                        (a.js_defproperty)(J, -2, name.as_ptr(), atts);
                        emit("def-ok");
                    }
                    3 => {
                        (a.js_delproperty)(J, -1, name.as_ptr());
                        emit("del-ok");
                    }
                    4 => {
                        emit(&format!("has={}", (a.js_hasproperty)(J, -1, name.as_ptr())));
                    }
                    5 => {
                        (a.js_getindex)(J, -1, idx);
                        emit(&format!("getindex={}", repr_at(a, J, -1)));
                    }
                    6 => {
                        push_val(a, J, val);
                        (a.js_setindex)(J, -2, idx);
                        emit("setindex-ok");
                    }
                    7 => {
                        (a.js_delindex)(J, -1, idx);
                        emit("delindex-ok");
                    }
                    8 => {
                        emit(&format!("hasindex={}", (a.js_hasindex)(J, -1, idx)));
                    }
                    9 => {
                        emit(&format!("getlength={}", (a.js_getlength)(J, -1)));
                        (a.js_setlength)(J, -1, idx);
                        emit(&format!("getlength2={}", (a.js_getlength)(J, -1)));
                    }
                    10 => {
                        (a.js_newcfunction)(J, Some(cf_noop), b"g\0".as_ptr() as *const c_char, 0);
                        (a.js_pushundefined)(J);
                        (a.js_defaccessor)(J, -3, name.as_ptr(), atts);
                        emit("defaccessor-ok");
                    }
                    _ => {
                        (a.js_pushiterator)(J, -1, (val % 2) as c_int);
                        let mut k = 0;
                        loop {
                            let nm = (a.js_nextiterator)(J, -1);
                            if nm.is_null() {
                                break;
                            }
                            emit(&format!("it={:?}", rs(nm)));
                            k += 1;
                            if k > 12 {
                                break;
                            }
                        }
                    }
                }
                emit(&format!("top={}", (a.js_gettop)(J)));
                dump(a, J);
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        for f in [0, JS_STRICT] {
            same(
                &format!(
                    "rnd iter={} recv={} op={} name={:?} idx={} atts/val={} flags={}",
                    iter, recv, op, name, idx, pi(2), f
                ),
                &p.c.run_native(act, f),
                &p.r.run_native(act, f),
            );
        }
    }
}
