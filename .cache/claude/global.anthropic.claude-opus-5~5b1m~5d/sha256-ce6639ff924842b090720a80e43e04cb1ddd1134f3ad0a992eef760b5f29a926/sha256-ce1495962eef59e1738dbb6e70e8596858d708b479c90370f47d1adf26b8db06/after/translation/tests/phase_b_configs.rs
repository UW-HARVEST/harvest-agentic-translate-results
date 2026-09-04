//! Phase B — the rows of `CONFIGS.md` that the other Phase B/C suites do not
//! reach: state creation/flags/context/report, the allocator `actx`, strict
//! compile-time rejections, `js_setlimit` in every shape, attribute words and
//! their accumulation, userdata / cfunction callback matrices, `js_ref` naming,
//! `js_torepr`/`js_tryrepr` index semantics, the `js_p*`/`js_try*` entry points,
//! the predicate family, `js_newobjectx` prototypes, `js_freestate` teardown and
//! the two timezone-dependent Date rows.
//!
//! Everything goes through the two `.so` exports only (`libloading` via
//! `common::libs()`); no Rust function of the crate is ever called directly.
//! Every error path is driven inside a `js_pcall` (`diff_native` / `diff_eval`),
//! never through `diff_isolated`.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::cell::Cell;

/* Callback bookkeeping. These counters are THREAD-LOCAL on purpose: every C
 * callback runs on the thread that drives the action, so two tests running in
 * parallel can never see each other's counts. */
thread_local! {
    static UD_FIN: Cell<i64> = const { Cell::new(0) };
    static CF_FIN: Cell<i64> = const { Cell::new(0) };
    static ACTX_EXPECT: Cell<i64> = const { Cell::new(0) };
    static ACTX_BAD: Cell<i64> = const { Cell::new(0) };
    static ALLOC_BUDGET: Cell<i64> = const { Cell::new(0) };
}

fn ud_fin_set(v: i64) { UD_FIN.with(|c| c.set(v)) }
fn ud_fin_get() -> i64 { UD_FIN.with(|c| c.get()) }
fn ud_fin_inc() { UD_FIN.with(|c| c.set(c.get() + 1)) }
fn cf_fin_set(v: i64) { CF_FIN.with(|c| c.set(v)) }
fn cf_fin_get() -> i64 { CF_FIN.with(|c| c.get()) }
fn cf_fin_inc() { CF_FIN.with(|c| c.set(c.get() + 1)) }
fn actx_expect_set(v: i64) { ACTX_EXPECT.with(|c| c.set(v)) }
fn actx_expect_get() -> i64 { ACTX_EXPECT.with(|c| c.get()) }
fn actx_bad_set(v: i64) { ACTX_BAD.with(|c| c.set(v)) }
fn actx_bad_get() -> i64 { ACTX_BAD.with(|c| c.get()) }
fn actx_bad_inc() { ACTX_BAD.with(|c| c.set(c.get() + 1)) }
fn budget_set(v: i64) { ALLOC_BUDGET.with(|c| c.set(v)) }
/// Consume one unit of the allocation budget; false once it is exhausted.
fn budget_take() -> bool {
    ALLOC_BUDGET.with(|c| {
        let v = c.get();
        c.set(v - 1);
        v > 0
    })
}

const SEED: u64 = 0xC0F1_6534_9ABC_DEF1;

/* Every state-flag word the C can be handed (rows 1, 4, 5). Only bit 0
 * (JS_STRICT) is ever read, so 0/2/-2/0x80000000/0x40000000 must behave exactly
 * like `flags = 0` and 1/3/-1/0x7fffffff exactly like `flags = JS_STRICT`. */
const FLAGSETS: [c_int; 9] = [
    0,
    JS_STRICT,   /* 1        */
    2,           /* bit 1 only          -> non-strict */
    3,           /* 1|2                 -> strict     */
    -1,          /* 0xFFFFFFFF          -> strict     */
    0x7fff_ffff, /* bit 0 set           -> strict     */
    -2,          /* 0xFFFFFFFE          -> non-strict */
    i32::MIN,    /* 0x80000000          -> non-strict */
    0x4000_0000, /* -> non-strict */
];

extern "C" {
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/* ---------------------------------------------------------------- helpers */

/// `js_pushliteral`, cfunction/userdata names and userdata tags are stored as
/// the RAW pointer (never copied), so they must be static.
fn cptr(b: &'static [u8]) -> *const c_char {
    b.as_ptr() as *const c_char
}

static N_PROBE: &[u8] = b"PROBE\0";
static N_FN: &[u8] = b"fn\0";
static N_CTOR: &[u8] = b"Ctor\0";
static N_GET: &[u8] = b"get\0";
static TAG_A: &[u8] = b"tagA\0";
static TAG_B: &[u8] = b"tagB\0";
static LIT_EMPTY: &[u8] = b"\0";
static LIT_SHORT: &[u8] = b"lit\0";
static LIT_15: &[u8] = b"123456789012345\0";
static LIT_16: &[u8] = b"1234567890123456\0";

fn dump(a: &Api, J: JS) {
    unsafe {
        let n = (a.js_gettop)(J);
        emit(&format!("top={}", n));
        for i in 0..n {
            emit(&format!("[{}]={}", i, repr_at(a, J, i)));
        }
    }
}

/// Compile + run `src` in the current state, recording rc and the result.
/// Never lets an exception escape (`js_ploadstring` + `js_pcall`).
fn run_expr(a: &Api, J: JS, src: &str, label: &str) {
    unsafe {
        let nm = cs("cfg.js");
        let s = cs(src);
        let rc = (a.js_ploadstring)(J, nm.as_ptr(), s.as_ptr());
        if rc != 0 {
            emit(&format!("{} load={} {:?}", label, rc, str_at(a, J, -1)));
            (a.js_pop)(J, 1);
            return;
        }
        (a.js_pushundefined)(J);
        let rc = (a.js_pcall)(J, 0);
        emit(&format!("{} rc={} {}", label, rc, repr_at(a, J, -1)));
        (a.js_pop)(J, 1);
    }
}

/// `js_hasproperty` pushes the value when it finds one: observe and clean up.
fn probe_has(a: &Api, J: JS, idx: c_int, name: &str) {
    unsafe {
        let n = cs(name);
        let h = (a.js_hasproperty)(J, idx, n.as_ptr());
        if h != 0 {
            emit(&format!("has {:?}=1 v={}", name, repr_at(a, J, -1)));
            (a.js_pop)(J, 1);
        } else {
            emit(&format!("has {:?}=0", name));
        }
    }
}

fn probe_get(a: &Api, J: JS, idx: c_int, name: &str) {
    unsafe {
        let n = cs(name);
        (a.js_getproperty)(J, idx, n.as_ptr());
        emit(&format!("get {:?}={}", name, repr_at(a, J, -1)));
        (a.js_pop)(J, 1);
    }
}

unsafe extern "C" fn rep_emit(_J: JS, msg: *const c_char) {
    emit(&format!("report:{:?}", unsafe { rs(msg) }));
}

unsafe extern "C" fn panic_emit(_J: JS) {
    emit("panic-handler");
}

/* ========================================================================== */
/* rows 173 + 175: Date under a fixed timezone.                                */
/*                                                                            */
/* `LocalTZA()` caches its result in a function-local `static` (jsdate.c:28),   */
/* so ONE process can only ever observe ONE timezone per library.  Row 173     */
/* (TZ=UTC, LocalTZA()==0) therefore runs FIRST in this process — the test     */
/* name sorts before every other test in this file and libtest runs the tests  */
/* in sorted order — and row 175 (TZ=Asia/Kolkata, +05:30, the half-hour       */
/* offset that exercises the msPerMinute division of Dp_getTimezoneOffset and   */
/* the fmtdatetime offset rendering) runs in a re-exec of this very test binary.*/
/* Row 174 (America/New_York) is covered by phase_b_script::date_getters_*.     */
/* ========================================================================== */

const TZ_ENV: &str = "MUJS_CONFIGS_TZ";

fn date_tz_body(zone: &str, want_offset: &str) {
    let p = libs();

    /* fixed + randomized epochs (fixed seed) */
    let mut rng = Rng::new(SEED ^ 0x7A);
    let mut ts: Vec<String> = vec![
        "0".into(),
        "1".into(),
        "-1".into(),
        "1000".into(),
        "1500000000000".into(),   /* summer 2017 */
        "1483228800000".into(),   /* winter 2017 */
        "-86400000".into(),
        "8.64e15".into(),
        "-8.64e15".into(),
        "NaN".into(),
    ];
    for _ in 0..40 {
        ts.push(format!("{}", rng.range_i64(-2_000_000_000_000, 2_000_000_000_000)));
    }

    let getters = [
        "getTimezoneOffset",
        "getHours",
        "getUTCHours",
        "getDate",
        "getUTCDate",
        "getDay",
        "getUTCDay",
        "getMonth",
        "getUTCMonth",
        "getFullYear",
        "getUTCFullYear",
        "getMinutes",
        "getUTCMinutes",
        "toString",
        "toDateString",
        "toTimeString",
        "toLocaleString",
        "toLocaleDateString",
        "toLocaleTimeString",
        "toUTCString",
        "toISOString",
    ];

    let mut src = String::from("var T=[");
    for (i, t) in ts.iter().enumerate() {
        if i > 0 {
            src.push(',');
        }
        src.push_str(t);
    }
    src.push_str("];var G=[");
    for (i, g) in getters.iter().enumerate() {
        if i > 0 {
            src.push(',');
        }
        src.push_str(&format!("'{}'", g));
    }
    src.push_str("];var o=[];for(var i=0;i<T.length;i++){var d=new Date(T[i]);");
    src.push_str("for(var j=0;j<G.length;j++){try{o.push(String(d[G[j]]()))}catch(e){o.push('!'+String(e))}}");
    /* local-string round trip and the zone-less parse (TZ dependent) */
    src.push_str("try{o.push(String(Date.parse(d.toString())))}catch(e){o.push('!p')}");
    src.push_str("try{o.push(String(Date.parse('2017-07-14T02:40:00')))}catch(e){o.push('!q')}");
    src.push_str("try{o.push(String(new Date(2017,6,14,2,40,0,0).getTime()))}catch(e){o.push('!r')}");
    src.push_str("try{o.push(String(Date.UTC(2017,6,14,2,40,0,0)))}catch(e){o.push('!s')}");
    src.push_str("}o.join('~')");

    let c = p.c.eval(&src, 0);
    let r = p.r.eval(&src, 0);
    same(&format!("date TZ={} (rows 173/175)", zone), &c, &r);
    assert!(c.starts_with("ok "), "TZ={} script failed: {}", zone, c);

    /* prove the zone really took effect (and let a parent process see it) */
    let off = p.c.eval("new Date(0).getTimezoneOffset()", 0);
    let offr = p.r.eval("new Date(0).getTimezoneOffset()", 0);
    same(&format!("date TZ={} offset", zone), &off, &offr);
    println!("TZ[{}] offset={}", zone, off);
    assert_eq!(off, want_offset, "TZ={} did not take effect", zone);
}

/// Row 173 — `TZ=UTC`, `LocalTZA() == 0`: every local getter equals its UTC
/// twin and `getTimezoneOffset()` is 0.  MUST run before anything else in this
/// process touches a Date (see the comment above).
#[test]
fn a1_date_timezone_utc_first() {
    if std::env::var(TZ_ENV).is_ok() {
        return; /* we are the row-175 child */
    }
    std::env::set_var("TZ", "UTC");
    date_tz_body("UTC", "ok 0");
}

/// Row 175 — `TZ=Asia/Kolkata` (+05:30, no DST). Runs in a re-exec of this test
/// binary because `LocalTZA()` can only be initialised once per process.
#[test]
fn a2_date_timezone_half_hour_offset_child() {
    if std::env::var(TZ_ENV).is_ok() {
        return;
    }
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .arg("zz_date_timezone_from_env")
        .arg("--exact")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("TZ", "Asia/Kolkata")
        .env(TZ_ENV, "Asia/Kolkata")
        .output()
        .expect("spawn TZ child");
    let so = String::from_utf8_lossy(&out.stdout);
    let se = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "row 175 child failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        so,
        se
    );
    assert!(
        so.contains("TZ[Asia/Kolkata] offset=ok -330"),
        "row 175 child did not pick up TZ:\nstdout:\n{}\nstderr:\n{}",
        so,
        se
    );
}

/// The row-175 body; only active in the child process.
#[test]
fn zz_date_timezone_from_env() {
    let zone = match std::env::var(TZ_ENV) {
        Ok(z) => z,
        Err(_) => return,
    };
    date_tz_body(&zone, "ok -330");
}

/* ========================================================================== */
/* rows 1-5: js_newstate flag words + js_dostring strict/non-strict            */
/* ========================================================================== */

/// Rows 2, 3, 4, 5 — `js_dostring` assignment to an undeclared variable under
/// every flag word, plus a strict-only compile rejection to pin the state's
/// strictness, plus the report text `js_dostring` produces.
#[test]
fn state_flag_words_and_dostring_strictness() {
    fn act(a: &Api, J: JS) {
        unsafe {
            (a.js_setreport)(J, Some(rep_emit));
            for s in [
                "x = 1;",
                "x = 1; x",
                "y = 1",
                "var z = 1; z",
                "'use strict'; w = 1",
                "(",
            ] {
                let src = cs(s);
                emit(&format!("dostring({:?})={}", s, (a.js_dostring)(J, src.as_ptr())));
            }
            /* observable strictness of the state itself */
            run_expr(a, J, "(function(){ return typeof this })()", "this");
            run_expr(a, J, "with({a:1}){a}", "with");
            run_expr(a, J, "var o={}; delete o.a", "delete-prop");
            run_expr(a, J, "var o=Object.freeze({a:1}); o.a=2; o.a", "frozen");
            (a.js_pushnumber)(J, 0.0);
        }
    }
    let p = libs();
    for f in FLAGSETS {
        diff_native("newstate flag word", act, f);
        /* ground truth: only bit 0 selects strict mode */
        let c = p.c.run_native(act, f);
        if f & JS_STRICT != 0 {
            assert!(
                c.contains("dostring(\"x = 1;\")=1")
                    && c.contains("assignment to undeclared variable 'x'")
                    && c.contains("with load=1"),
                "flags={} should be strict: {}",
                f,
                c
            );
        } else {
            assert!(
                c.contains("dostring(\"x = 1;\")=0") && c.contains("with rc=0"),
                "flags={} should be non-strict: {}",
                f,
                c
            );
        }
    }
}

/// Rows 9, 10, 11 — `js_setcontext`/`js_getcontext`, `js_setreport` (callback /
/// NULL) and `js_atpanic`'s previous-handler protocol.  The *aborting* half of
/// row 11 (`js_throw` with `trytop == 0` calling `J->panic` and then `abort`)
/// and row 10(a) (the default report writing to stderr) are covered by
/// `phase_c_isolated::all_isolated_cases_match` (cases `throw_no_try` /
/// `report_default`), which compares the child's exit signal and stderr.
#[test]
fn state_context_report_and_panic_handlers() {
    let mut rng = Rng::new(SEED ^ 0x9);
    let mut msgs: Vec<String> = vec![
        "".into(),
        "msg".into(),
        "with % and %s".into(),
        "line\nbreak".into(),
        "\u{4e2d}\u{6587}".into(),
    ];
    for _ in 0..40 {
        msgs.push(rng.string(24));
    }
    for m in &msgs {
        set_ps(0, m);
        for ctx in [0i64, 1, 0x1234_5678] {
            set_pi(0, ctx);
            fn act(a: &Api, J: JS) {
                unsafe {
                    /* row 9 */
                    emit(&format!("ctx0_null={}", (a.js_getcontext)(J).is_null()));
                    let want = pi(0) as *mut c_void;
                    (a.js_setcontext)(J, want);
                    emit(&format!("ctx1_match={}", (a.js_getcontext)(J) == want));
                    (a.js_setcontext)(J, std::ptr::null_mut());
                    emit(&format!("ctx2_null={}", (a.js_getcontext)(J).is_null()));
                    (a.js_setcontext)(J, want);

                    /* row 10 (b) + (c) */
                    let m = ps(0);
                    (a.js_setreport)(J, Some(rep_emit));
                    (a.js_report)(J, m.as_ptr());
                    (a.js_setreport)(J, None);
                    (a.js_report)(J, m.as_ptr());
                    emit("after-null-report");
                    (a.js_setreport)(J, Some(rep_emit));
                    (a.js_report)(J, cs("back").as_ptr());

                    /* row 11: the handler protocol (previous handler returned) */
                    let old = (a.js_atpanic)(J, Some(panic_emit));
                    emit(&format!("old_is_none={}", old.is_none()));
                    let back = (a.js_atpanic)(J, None);
                    /* compare handler identity as addresses (the two libraries
                     * hand back the very pointer we installed) */
                    let addr = |h: Option<Panic>| h.map(|f| f as usize).unwrap_or(0);
                    emit(&format!(
                        "back_is_ours={}",
                        addr(back) == addr(Some(panic_emit))
                    ));
                    let back2 = (a.js_atpanic)(J, old);
                    emit(&format!("back2_is_none={}", back2.is_none()));
                    let back3 = (a.js_atpanic)(J, old);
                    emit(&format!(
                        "back3_is_default={}",
                        back3.is_some() && addr(back3) == addr(old)
                    ));

                    /* a caught throw must not reach the panic handler at all */
                    run_expr(a, J, "throw new Error('caught')", "throw");
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            diff_native("context/report/panic", act, 0);
        }
    }
    /* rows 9/10/11 ground truth: the context round trip, the report callback
     * seeing exactly the message, `js_setreport(NULL)` dropping it, and
     * `js_atpanic` returning the PREVIOUS handler (js_defaultpanic first). */
    set_ps(0, "exact message");
    set_pi(0, 0x1234_5678);
    fn act2(a: &Api, J: JS) {
        unsafe {
            emit(&format!("ctx0_null={}", (a.js_getcontext)(J).is_null()));
            let want = pi(0) as *mut c_void;
            (a.js_setcontext)(J, want);
            emit(&format!("ctx1_match={}", (a.js_getcontext)(J) == want));
            (a.js_setcontext)(J, std::ptr::null_mut());
            emit(&format!("ctx2_null={}", (a.js_getcontext)(J).is_null()));
            (a.js_setreport)(J, Some(rep_emit));
            (a.js_report)(J, ps(0).as_ptr());
            (a.js_setreport)(J, None);
            (a.js_report)(J, ps(0).as_ptr());
            emit("end");
            let old = (a.js_atpanic)(J, Some(panic_emit));
            emit(&format!("old_is_none={}", old.is_none()));
            (a.js_pushnumber)(J, 0.0);
        }
    }
    let p = libs();
    let c = p.c.run_native(act2, 0);
    same("context/report/panic ground truth", &c, &p.r.run_native(act2, 0));
    assert!(c.contains("ctx0_null=true|ctx1_match=true|ctx2_null=true"), "row 9: {}", c);
    assert_eq!(
        c.matches("report:\"exact message\"").count(),
        1,
        "row 10: the NULL handler must drop the second report: {}",
        c
    );
    assert!(c.contains("old_is_none=false"), "row 11 (js_defaultpanic): {}", c);
}

/* ---------------------------------------------------------------- row 8 */

/// Allocator that (a) checks `actx` on EVERY call and (b) fails after a budget.
unsafe extern "C" fn ctx_alloc(ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    if ctx as usize as i64 != actx_expect_get() {
        actx_bad_inc();
    }
    if n == 0 {
        free(p);
        return std::ptr::null_mut();
    }
    if !budget_take() {
        return std::ptr::null_mut();
    }
    realloc(p, n as usize)
}

/// Row 8 — the allocation budget runs out somewhere inside `jsB_init`, so the
/// `js_try` of `js_newstate` fires, `js_freestate` runs and NULL is returned;
/// and `actx` is handed unchanged to every single allocator call (including the
/// frees performed by `js_freestate`).
#[test]
fn newstate_allocator_budget_and_actx() {
    for budget in [
        0i64, 1, 2, 3, 4, 6, 10, 20, 50, 100, 250, 600, 1500, 4000, 20000,
    ] {
        for ctx in [0x0A0Ci64, 1, 0x7fff_0000] {
            set_pi(0, budget);
            set_pi(1, ctx);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let ctx = pi(1) as *mut c_void;
                    actx_expect_set(pi(1));
                    actx_bad_set(0);
                    budget_set(pi(0));
                    let J2 = (a.js_newstate)(Some(ctx_alloc), ctx, 0);
                    budget_set(i64::MAX / 2);
                    emit(&format!(
                        "budget={} null={} badctx={}",
                        pi(0),
                        J2.is_null(),
                        actx_bad_get()
                    ));
                    if !J2.is_null() {
                        run_expr(a, J2, "var s=0; for(var i=0;i<20;i++) s+=i; s", "sub");
                        (a.js_freestate)(J2);
                    }
                    emit(&format!("badctx_final={}", actx_bad_get()));
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            let c = p.c.run_native(act, 0);
            let r = p.r.run_native(act, 0);
            same(&format!("newstate budget={} ctx={:#x}", budget, ctx), &c, &r);
            assert!(c.contains("badctx=0"), "actx not passed through: {}", c);
            assert!(c.contains("badctx_final=0"), "actx not passed through: {}", c);
            if budget == 0 {
                assert!(c.contains("null=true"), "budget 0 must fail: {}", c);
            }
        }
    }
}

/* ========================================================================== */
/* rows 12, 13: strict-mode COMPILE-TIME rejections                            */
/* ========================================================================== */

/// (source, does it compile in a NON-strict state?).  Everything here is
/// rejected once strict mode is in force — either by the state flag (row 12) or
/// by a `"use strict"` prologue.  The four `eval` bindings raise an EvalError
/// from `checkfutureword`/`addlocal` even non-strict, and mujs rejects
/// leading-zero numbers unconditionally, so those rows are marked `false`.
const STRICT_REJECTED: &[(&str, bool)] = &[
    ("with(o){}", true),
    ("with({}){}", true),
    ("function f(){ with({}){} }", true),
    ("delete x", true),
    ("var x; delete x", true),
    ("function f(){ var y; return delete y }", true),
    ("var implements", true),
    ("var interface", true),
    ("var package", true),
    ("var private", true),
    ("var protected", true),
    ("var public", true),
    ("var static", true),
    ("var yield", true),
    ("var let", true),
    ("var arguments", true),
    ("var eval", false),
    ("function f(arguments){}", true),
    ("function f(eval){}", false),
    ("arguments = 1", true),
    ("eval = 1", false),
    ("try{}catch(eval){}", true),
    ("try{}catch(arguments){}", true),
    ("try{}catch(eval){}finally{}", true),
    ("function arguments(){}", true),
    ("function eval(){}", false),
    ("function f(a,a){}", true),
    ("010", false),
];

/// Rows 12 + 13 — every strict-mode compile rejection through
/// `js_ploadstring`, in a `JS_STRICT` state (must return 1 with a SyntaxError)
/// and in a non-strict state (must return 0), plus the same source with a
/// `"use strict"` prologue.
#[test]
fn compile_strict_rejections_and_acceptance() {
    for (s, ok_nonstrict) in STRICT_REJECTED {
        for prologue in [0i64, 1] {
            set_ps(0, s);
            set_pi(0, prologue);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let base = ps(0);
                    let src = if pi(0) == 0 {
                        base
                    } else {
                        cs(&format!("'use strict'; {}", base.to_str().unwrap()))
                    };
                    let nm = cs("strict.js");
                    let rc = (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr());
                    emit(&format!("load={} {:?}", rc, str_at(a, J, -1)));
                    if rc == 0 {
                        emit(&format!("typeof={}", rs((a.js_typeof)(J, -1))));
                    }
                    (a.js_pop)(J, 1);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            let p = libs();
            for f in [0, JS_STRICT] {
                diff_native("strict compile", act, f);
                let c = p.c.run_native(act, f);
                if f == 0 && prologue == 0 {
                    /* row 13: the non-strict state compiles all but the
                     * unconditionally-rejected sources */
                    if *ok_nonstrict {
                        assert!(c.contains("load=0"), "src={:?} must compile: {}", s, c);
                    } else {
                        assert!(c.contains("load=1"), "src={:?} must be rejected: {}", s, c);
                    }
                } else {
                    /* row 12: rejected with a SyntaxError once strict is on */
                    assert!(
                        c.contains("load=1") && c.contains("SyntaxError"),
                        "src={:?} prologue={} flags={} must be rejected: {}",
                        s,
                        prologue,
                        f,
                        c
                    );
                }
            }
        }
    }
}

/* ========================================================================== */
/* rows 25-33: js_setlimit                                                     */
/* ========================================================================== */

/// Rows 25-33 — every `js_setlimit` shape: both limits off, `runlimit == 1`,
/// a runlimit that expires mid-loop, a runlimit that is never reached, negative
/// runlimits (no limiting), the `js_malloc` memlimit branch through
/// `jsV_newmemstring`, the `js_realloc` memlimit branch through
/// `jsR_setarrayindex`, an allocation-heavy script, negative memlimits and both
/// limits together / cleared mid-run.
#[test]
fn setlimit_every_shape() {
    for mode in 0i64..12 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                match pi(0) {
                    /* row 25: both disabled */
                    0 => {
                        (a.js_setlimit)(J, 0, 0);
                        run_expr(a, J, "var i=0;while(i<200000)i++;i", "loop");
                    }
                    /* row 26: runlimit == 1 fires on the first instruction */
                    1 => {
                        (a.js_setlimit)(J, 1, 0);
                        run_expr(a, J, "var i=0;while(i<10)i++;i", "loop");
                    }
                    /* row 27: expires mid-loop / never reached */
                    2 => {
                        (a.js_setlimit)(J, 50, 0);
                        run_expr(a, J, "var i=0;while(1)i++;", "spin");
                    }
                    3 => {
                        (a.js_setlimit)(J, 1000000, 0);
                        run_expr(a, J, "var i=0;while(i<10)i++;i", "short");
                        run_expr(a, J, "var i=0;while(i<10)i++;i", "short2");
                    }
                    /* row 28: negative runlimit disables the check */
                    4 => {
                        (a.js_setlimit)(J, -1, 0);
                        run_expr(a, J, "var i=0;while(i<20000)i++;i", "neg1");
                    }
                    5 => {
                        (a.js_setlimit)(J, i32::MIN, 0);
                        run_expr(a, J, "var i=0;while(i<20000)i++;i", "negmin");
                    }
                    /* row 29: js_malloc branch (jsV_newmemstring) */
                    6 => {
                        (a.js_setlimit)(J, 0, 8);
                        (a.js_pushstring)(J, cs("0123456789ABCDEFG").as_ptr());
                        emit(&format!("pushed={}", repr_at(a, J, -1)));
                    }
                    /* row 30: js_realloc branch (jsR_setarrayindex) */
                    7 => {
                        (a.js_newarray)(J);
                        (a.js_setlimit)(J, 0, 200);
                        for i in 0..24 {
                            (a.js_pushnumber)(J, i as f64);
                            (a.js_setindex)(J, -2, i);
                            emit(&format!("appended {} len={}", i, (a.js_getlength)(J, -1)));
                        }
                    }
                    /* row 31: allocation-heavy script; js_free does not credit back */
                    8 => {
                        (a.js_setlimit)(J, 0, 4096);
                        run_expr(a, J, "var a=[];for(var i=0;i<1000;i++)a.push({});a.length", "alloc");
                        run_expr(a, J, "1+1", "after");
                    }
                    /* row 32: negative memlimit disables the check */
                    9 => {
                        (a.js_setlimit)(J, 0, -1);
                        let q = (a.js_malloc)(J, 1 << 20);
                        emit(&format!("malloc_ok={}", !q.is_null()));
                        (a.js_free)(J, q);
                        run_expr(a, J, "var a=[];for(var i=0;i<200;i++)a.push('s'+i);a.length", "big");
                    }
                    10 => {
                        (a.js_setlimit)(J, 0, i32::MIN);
                        let q = (a.js_malloc)(J, 1 << 20);
                        emit(&format!("malloc_ok={}", !q.is_null()));
                        (a.js_free)(J, q);
                    }
                    /* row 33: both limits; then cleared mid-run */
                    _ => {
                        (a.js_setlimit)(J, 100, 100000);
                        run_expr(a, J, "var a=[];for(var i=0;i<1000;i++)a.push({});a.length", "both");
                        (a.js_setlimit)(J, 0, 0);
                        run_expr(a, J, "var a=[];for(var i=0;i<1000;i++)a.push({});a.length", "cleared");
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("setlimit mode={}", mode), act, f);
        }
        /* ground truth for the two literal messages of jsrun.c */
        let p = libs();
        let c = p.c.run_native(act, 0);
        if mode == 1 || mode == 2 {
            assert!(c.contains("script ran too long"), "mode={}: {}", mode, c);
        }
        if mode == 6 || mode == 7 {
            assert!(c.contains("out of memory"), "mode={}: {}", mode, c);
        }
        /* rows 25/28/32: no limiting at all */
        if mode == 0 {
            assert!(c.contains("loop rc=0"), "mode={}: {}", mode, c);
        }
        if mode == 4 || mode == 5 {
            assert!(c.contains("rc=0"), "mode={}: {}", mode, c);
        }
        if mode == 9 || mode == 10 {
            assert!(c.contains("malloc_ok=true"), "mode={}: {}", mode, c);
        }
    }

    /* --- property style: randomized (runlimit, memlimit) pairs x workloads -- */
    let mut rng = Rng::new(SEED ^ 0x25);
    for iter in 0..200 {
        let rl = match rng.below(5) {
            0 => 0,
            1 => rng.range_i64(1, 40),
            2 => -rng.range_i64(1, 1000),
            3 => rng.range_i64(1, 5000),
            _ => i32::MAX as i64,
        };
        let ml = match rng.below(5) {
            0 => 0,
            1 => rng.range_i64(1, 512),
            2 => -rng.range_i64(1, 1000),
            3 => rng.range_i64(512, 1 << 16),
            _ => i32::MAX as i64,
        };
        set_pi(0, rl);
        set_pi(1, ml);
        set_pi(2, rng.range_i64(0, 5));
        fn rnd(a: &Api, J: JS) {
            unsafe {
                (a.js_setlimit)(J, pic(0), pic(1));
                match pi(2) {
                    0 => run_expr(a, J, "var s=0;for(var i=0;i<400;i++)s+=i;s", "loop"),
                    1 => run_expr(a, J, "var s='x';for(var i=0;i<6;i++)s=s+s;s.length", "concat"),
                    2 => run_expr(a, J, "var a=[];for(var i=0;i<120;i++)a[i]={k:i};a.length", "array"),
                    3 => {
                        (a.js_pushstring)(J, cs("a string long enough to need a memstr").as_ptr());
                        emit(&format!("pushed={}", repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    4 => {
                        let q = (a.js_malloc)(J, 4096);
                        emit(&format!("malloc={}", !q.is_null()));
                        let q2 = (a.js_realloc)(J, q, 65536);
                        emit(&format!("realloc={}", !q2.is_null()));
                        (a.js_free)(J, q2);
                    }
                    _ => {
                        (a.js_newarray)(J);
                        for i in 0..20 {
                            (a.js_pushnumber)(J, i as f64);
                            (a.js_setindex)(J, -2, i);
                        }
                        emit(&format!("len={}", (a.js_getlength)(J, -1)));
                        (a.js_pop)(J, 1);
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        diff_native(
            &format!("setlimit rnd iter={} rl={} ml={} op={}", iter, rl, ml, pi(2)),
            rnd,
            if iter % 2 == 0 { 0 } else { JS_STRICT },
        );
    }
}

/* ========================================================================== */
/* rows 34, 38, 40, 42, 43: attribute words                                    */
/* ========================================================================== */

/// Rows 34, 38, 40, 42 (+43) — the attribute word of `js_defproperty` /
/// `js_defglobal` / `js_defaccessor` reflected back through
/// `Object.getOwnPropertyDescriptor`, `Object.keys`, `delete` and a write, for
/// every in-range value AND out-of-band bits, plus the `ref->atts |= atts`
/// accumulation (attributes can never be cleared through this API).
#[test]
fn attribute_words_and_accumulation() {
    for atts in [
        0i64, 1, 2, 3, 4, 5, 6, 7, /* out of band (row 43) */ 8, 9, 15, 16, 255, -1, -8,
        i32::MAX as i64, i32::MIN as i64,
    ] {
        set_pi(0, atts);
        fn act(a: &Api, J: JS) {
            unsafe {
                let atts = pic(0);
                /* --- js_defproperty on a plain object --- */
                (a.js_newobject)(J);
                (a.js_pushnumber)(J, 1.0);
                (a.js_defproperty)(J, -2, cs("a").as_ptr(), atts);
                /* accumulation: DONTENUM then READONLY must end up as 3 */
                (a.js_pushnumber)(J, 1.0);
                (a.js_defproperty)(J, -2, cs("b").as_ptr(), JS_DONTENUM);
                (a.js_pushnumber)(J, 2.0);
                (a.js_defproperty)(J, -2, cs("b").as_ptr(), JS_READONLY);
                /* and with the sweep value applied on top of a 0-atts property */
                (a.js_pushnumber)(J, 1.0);
                (a.js_defproperty)(J, -2, cs("c").as_ptr(), 0);
                (a.js_pushnumber)(J, 2.0);
                (a.js_defproperty)(J, -2, cs("c").as_ptr(), atts);
                (a.js_copy)(J, -1);
                (a.js_setglobal)(J, cs("O").as_ptr());
                run_expr(
                    a,
                    J,
                    "['a','b','c'].map(function(k){var d=Object.getOwnPropertyDescriptor(O,k);\
                     return k+':'+(d?[d.value,d.writable,d.enumerable,d.configurable,('get' in d)].join('/'):'none')}).join(' ')",
                    "desc",
                );
                run_expr(a, J, "Object.keys(O).join(',')", "keys");
                run_expr(a, J, "Object.getOwnPropertyNames(O).join(',')", "names");
                run_expr(a, J, "O.a=9; O.b=9; O.c=9; [O.a,O.b,O.c].join(',')", "write");
                run_expr(a, J, "[delete O.a, delete O.b, delete O.c].join(',')", "delete");
                run_expr(a, J, "Object.getOwnPropertyNames(O).join(',')", "names2");
                (a.js_pop)(J, 1);

                /* --- js_defglobal with the same word (row 34) --- */
                (a.js_pushnumber)(J, 5.0);
                (a.js_defglobal)(J, cs("gv").as_ptr(), atts);
                run_expr(
                    a,
                    J,
                    "var d=Object.getOwnPropertyDescriptor(this,'gv');\
                     d?[d.value,d.writable,d.enumerable,d.configurable].join('/'):'none'",
                    "gdesc",
                );
                (a.js_getglobal)(J, cs("gv").as_ptr());
                emit(&format!("gv={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                (a.js_pushnumber)(J, 6.0);
                (a.js_setglobal)(J, cs("gv").as_ptr());
                (a.js_getglobal)(J, cs("gv").as_ptr());
                emit(&format!("gv2={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                (a.js_delglobal)(J, cs("gv").as_ptr());
                (a.js_getglobal)(J, cs("gv").as_ptr());
                emit(&format!("gv3={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);

                /* --- js_defaccessor with the same word --- */
                (a.js_newobject)(J);
                (a.js_newcfunction)(J, Some(cf_const42), cptr(N_GET), 0);
                (a.js_pushundefined)(J);
                (a.js_defaccessor)(J, -3, cs("acc").as_ptr(), atts);
                (a.js_copy)(J, -1);
                (a.js_setglobal)(J, cs("A").as_ptr());
                run_expr(
                    a,
                    J,
                    "var d=Object.getOwnPropertyDescriptor(A,'acc');\
                     [A.acc,(typeof d.get),(typeof d.set),d.enumerable,d.configurable,('value' in d)].join('/')",
                    "adesc",
                );
                run_expr(a, J, "Object.keys(A).join(',')", "akeys");
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("atts={}", atts), act, f);
        }
        /* row 42: `ref->atts |= atts` — DONTENUM then READONLY ends up as 3 and
         * the value written by the second js_defproperty is kept. */
        let p = libs();
        let c = p.c.run_native(act, 0);
        assert!(
            c.contains("b:2/false/false/true"),
            "attribute accumulation (atts={}): {}",
            atts,
            c
        );
        /* row 34: atts == 0 is writable + enumerable + configurable */
        if atts == 0 {
            assert!(c.contains("a:1/true/true/true"), "atts=0: {}", c);
            assert!(c.contains("gdesc rc=0 \"5/true/true/true\""), "atts=0 global: {}", c);
        }
        /* rows 38/40 and the out-of-band words behave like their low 3 bits */
        if atts == 3 {
            assert!(c.contains("a:1/false/false/true"), "atts=3: {}", c);
        }
        if atts == 6 {
            assert!(c.contains("a:1/true/false/false"), "atts=6: {}", c);
        }
        if atts == 8 || atts == 16 || atts == i32::MIN as i64 {
            assert!(c.contains("a:1/true/true/true"), "atts={} (bits>2 ignored): {}", atts, c);
        }
    }

    /* --- property style: randomized attribute words, names and orders ------ */
    let mut rng = Rng::new(SEED ^ 0x34);
    for iter in 0..250 {
        set_pi(0, rng.range_i64(-8, 16));
        set_pi(1, rng.range_i64(-8, 16));
        set_pi(2, rng.range_i64(-8, 16));
        let name = match rng.below(5) {
            0 => "p".to_string(),
            1 => format!("k{}", rng.below(5)),
            2 => format!("{}", rng.below(4)),
            3 => "".to_string(),
            _ => rng.string(6),
        };
        set_ps(0, &name);
        fn rnd(a: &Api, J: JS) {
            unsafe {
                let name = ps(0);
                /* the name reaches the script as a global STRING, so even an
                 * empty or non-identifier name can be reflected */
                (a.js_pushstring)(J, name.as_ptr());
                (a.js_setglobal)(J, cs("NM").as_ptr());
                (a.js_newobject)(J);
                for k in 0..3 {
                    (a.js_pushnumber)(J, k as f64);
                    (a.js_defproperty)(J, -2, name.as_ptr(), pic(k as usize));
                    probe_get(a, J, -1, name.to_str().unwrap_or("<utf8>"));
                }
                (a.js_copy)(J, -1);
                (a.js_setglobal)(J, cs("O").as_ptr());
                run_expr(
                    a,
                    J,
                    "(function(){var d=Object.getOwnPropertyDescriptor(O,NM);\
                     return d?[d.value,d.writable,d.enumerable,d.configurable].join('/'):'none'})()",
                    "desc",
                );
                run_expr(a, J, "Object.keys(O).join(',')", "keys");
                run_expr(a, J, "Object.getOwnPropertyNames(O).join(',')", "names");
                /* write / delete / re-read through the raw API */
                (a.js_pushnumber)(J, 99.0);
                (a.js_setproperty)(J, -2, name.as_ptr());
                probe_get(a, J, -1, "after-write");
                (a.js_delproperty)(J, -1, name.as_ptr());
                probe_has(a, J, -1, "after-delete");
                /* the same word through js_defglobal (throw == 0) */
                (a.js_pushnumber)(J, 7.0);
                (a.js_defglobal)(J, name.as_ptr(), pic(0));
                run_expr(
                    a,
                    J,
                    "(function(){var d=Object.getOwnPropertyDescriptor(this,NM);\
                     return d?[d.value,d.writable,d.enumerable,d.configurable].join('/'):'none'})()",
                    "gdesc",
                );
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        diff_native(
            &format!(
                "atts rnd iter={} {}/{}/{} name={:?}",
                iter,
                pi(0),
                pi(1),
                pi(2),
                name
            ),
            rnd,
            if iter % 2 == 0 { 0 } else { JS_STRICT },
        );
    }
}

unsafe extern "C" fn cf_const42(J: JS) {
    let a = cur();
    unsafe {
        emit(&format!("cf_const42 top={}", (a.js_gettop)(J)));
        (a.js_pushnumber)(J, 42.0);
    }
}

/* ========================================================================== */
/* rows 19, 21: readonly writes and the `transient` receiver                   */
/* ========================================================================== */

/// Rows 19 + 21 — `jsR_setproperty`'s readonly branch (`js_defproperty` with
/// `JS_READONLY` then `js_setproperty`: silent when non-strict, TypeError under
/// `JS_STRICT`) and the `transient` branch reached by `js_setproperty` /
/// `js_setindex` on a NON-object receiver (the C computes
/// `transient = !js_isobject(idx)` from the still-unboxed slot).
#[test]
fn readonly_and_transient_writes() {
    for mode in 0i64..8 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                match pi(0) {
                    /* row 19 */
                    0 => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs("a").as_ptr(), JS_READONLY);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_setproperty)(J, -2, cs("a").as_ptr());
                        probe_get(a, J, -1, "a");
                    }
                    1 => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs("0").as_ptr(), JS_READONLY);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_setindex)(J, -2, 0);
                        emit(&format!("hasindex={}", (a.js_hasindex)(J, -1, 0)));
                        (a.js_pop)(J, (a.js_gettop)(J) - 2);
                    }
                    2 => {
                        /* readonly inherited from the prototype chain */
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defproperty)(J, -2, cs("a").as_ptr(), JS_READONLY);
                        (a.js_newobjectx)(J);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_setproperty)(J, -2, cs("a").as_ptr());
                        probe_get(a, J, -1, "a");
                    }
                    /* row 21: transient receivers */
                    3 => {
                        (a.js_pushstring)(J, cs("abc").as_ptr());
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setproperty)(J, -2, cs("foo").as_ptr());
                        emit(&format!("after={}", repr_at(a, J, -1)));
                    }
                    4 => {
                        (a.js_pushstring)(J, cs("abc").as_ptr());
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setindex)(J, -2, 7);
                        emit(&format!("after={}", repr_at(a, J, -1)));
                    }
                    5 => {
                        (a.js_pushnumber)(J, 5.0);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setproperty)(J, -2, cs("bar").as_ptr());
                        emit(&format!("after={}", repr_at(a, J, -1)));
                    }
                    6 => {
                        (a.js_pushboolean)(J, 1);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setindex)(J, -2, 0);
                        emit(&format!("after={}", repr_at(a, J, -1)));
                    }
                    _ => {
                        /* an OBJECT receiver is never transient */
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setproperty)(J, -2, cs("foo").as_ptr());
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_setindex)(J, -2, 3);
                        emit(&format!("after={}", repr_at(a, J, -1)));
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("readonly/transient mode={}", mode), act, f);
        }
    }
    /* ground truth: the transient message is only reachable through the C API */
    let p = libs();
    set_pi(0, 4);
    fn act4(a: &Api, J: JS) {
        unsafe {
            (a.js_pushstring)(J, cs("abc").as_ptr());
            (a.js_pushnumber)(J, 1.0);
            (a.js_setindex)(J, -2, 7);
            emit("no-throw");
            (a.js_pushnumber)(J, 0.0);
        }
    }
    let c = p.c.run_native(act4, JS_STRICT);
    let r = p.r.run_native(act4, JS_STRICT);
    same("setindex transient strict", &c, &r);
    assert!(
        c.contains("transient object"),
        "js_setindex should hit the transient branch: {}",
        c
    );

    /* --- property style: randomized receiver / name / index / value --------- */
    let mut rng = Rng::new(SEED ^ 0x19);
    for iter in 0..300 {
        set_pi(0, rng.range_i64(0, 6)); /* receiver kind */
        set_pi(1, rng.range_i64(0, 4)); /* value kind    */
        set_pi(2, rng.range_i64(-2, 9)); /* index        */
        set_pi(3, rng.range_i64(0, 1)); /* setproperty or setindex */
        let name = match rng.below(4) {
            0 => "a".to_string(),
            1 => format!("{}", rng.below(6)),
            2 => "length".to_string(),
            _ => rng.string(5),
        };
        set_ps(0, &name);
        fn rnd(a: &Api, J: JS) {
            unsafe {
                let name = ps(0);
                match pi(0) {
                    0 => (a.js_pushstring)(J, cs("abc").as_ptr()),
                    1 => (a.js_pushnumber)(J, 5.5),
                    2 => (a.js_pushboolean)(J, 1),
                    3 => (a.js_pushliteral)(J, cptr(LIT_SHORT)),
                    4 => (a.js_newobject)(J),
                    5 => (a.js_newstring)(J, cs("abc").as_ptr()),
                    _ => (a.js_newarray)(J),
                }
                /* make the target property readonly first (row 19) whenever the
                 * receiver is a real object */
                if (a.js_isobject)(J, -1) != 0 {
                    (a.js_pushnumber)(J, 1.0);
                    (a.js_defproperty)(J, -2, name.as_ptr(), JS_READONLY);
                    emit("predefined-readonly");
                }
                match pi(1) {
                    0 => (a.js_pushnumber)(J, 2.0),
                    1 => (a.js_pushstring)(J, cs("v").as_ptr()),
                    2 => (a.js_pushundefined)(J),
                    3 => (a.js_newobject)(J),
                    _ => (a.js_pushboolean)(J, 0),
                }
                if pi(3) == 0 {
                    (a.js_setproperty)(J, -2, name.as_ptr());
                } else {
                    (a.js_setindex)(J, -2, pic(2));
                }
                emit(&format!("after={}", repr_at(a, J, -1)));
                probe_has(a, J, -1, name.to_str().unwrap_or("<utf8>"));
                emit(&format!("hasindex={}", (a.js_hasindex)(J, -1, pic(2))));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(
                &format!(
                    "readonly/transient rnd iter={} recv={} val={} idx={} op={} name={:?}",
                    iter,
                    pi(0),
                    pi(1),
                    pi(2),
                    pi(3),
                    name
                ),
                rnd,
                f,
            );
        }
    }
}

/* ========================================================================== */
/* rows 24, 138: js_try/js_endtry/js_throw restore the whole frame             */
/* ========================================================================== */

unsafe extern "C" fn cf_push_and_throw(J: JS) {
    let a = cur();
    unsafe {
        for k in 0..5 {
            (a.js_pushnumber)(J, 100.0 + k as f64);
        }
        (a.js_newobject)(J);
        emit(&format!("inside top={}", (a.js_gettop)(J)));
        (a.js_newtypeerror)(J, cs("from the middle of a frame").as_ptr());
        (a.js_throw)(J);
    }
}

/// Rows 24 + 138 — the try frame saves and restores `E`, `envtop`, `tracetop`,
/// `top`, `bot` and `strict`.  A `"use strict"` script that throws sets
/// `J->strict = 1` inside `jsR_run`; after the longjmp the state must be
/// non-strict again (row 24), and a throw from a nested native frame with a
/// non-empty stack must leave the caller's stack exactly as it was (row 138).
#[test]
fn try_frame_restores_state() {
    fn act(a: &Api, J: JS) {
        unsafe {
            emit(&format!("top0={}", (a.js_gettop)(J)));
            /* strictness before / after a strict script that throws */
            run_expr(a, J, "u1 = 1; u1", "nonstrict-before");
            run_expr(a, J, "'use strict'; null.x", "strict-throw");
            run_expr(a, J, "u2 = 1; u2", "nonstrict-after");
            run_expr(a, J, "(function(){ return typeof this })()", "this-after");
            emit(&format!("top1={}", (a.js_gettop)(J)));

            /* a throw out of a nested native frame with junk on the stack */
            for k in 0..3 {
                (a.js_pushnumber)(J, k as f64);
            }
            (a.js_newcfunction)(J, Some(cf_push_and_throw), cptr(N_FN), 0);
            (a.js_pushundefined)(J);
            (a.js_pushnumber)(J, 7.0);
            let rc = (a.js_pcall)(J, 1);
            emit(&format!("pcall={} err={:?}", rc, str_at(a, J, -1)));
            (a.js_pop)(J, 1);
            dump(a, J);

            /* the environment survived: globals and new vars still work */
            run_expr(a, J, "var q=5; q", "env-after");
            run_expr(a, J, "u2", "global-after");
            /* and so did the trace stack */
            run_expr(a, J, "function g(){ return new Error('x').stackTrace } typeof g()", "trace");
            (a.js_pop)(J, (a.js_gettop)(J) - 1);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("try frame restore", act, f);
    }
    /* row 24: a strict script that throws must not leave J->strict set;
     * row 138: the caller's stack depth is restored exactly. */
    let p = libs();
    let c = p.c.run_native(act, 0);
    assert!(c.contains("strict-throw rc=1"), "row 24 setup: {}", c);
    assert!(
        c.contains("nonstrict-before rc=0") && c.contains("nonstrict-after rc=0"),
        "row 24 strict flag restored: {}",
        c
    );
    assert!(c.contains("this-after rc=0 \"object\""), "row 24 OP_THIS after: {}", c);
    assert!(c.contains("top0=1") && c.contains("top1=1"), "row 138 top: {}", c);
    assert!(c.contains("inside top=8") && c.contains("pcall=1"), "row 138 throw: {}", c);
    assert!(c.contains("env-after rc=0") && c.contains("global-after rc=0"), "row 138 env: {}", c);
    /* the strict state must stay strict (the same flag restored, not cleared) */
    let cs_ = p.c.run_native(act, JS_STRICT);
    assert!(
        cs_.contains("nonstrict-before rc=1") && cs_.contains("nonstrict-after rc=1"),
        "row 24 strict state: {}",
        cs_
    );
}

/* ========================================================================== */
/* rows 61-68: userdata                                                        */
/* ========================================================================== */

unsafe extern "C" fn u_has(J: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let a = cur();
    let nm = unsafe { rs(name) };
    emit(&format!("u_has({:?},data={:?})", nm, data));
    if nm == "virtual" {
        unsafe { (a.js_pushnumber)(J, 111.0) };
        return 1;
    }
    0
}

unsafe extern "C" fn u_put(_J: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let nm = unsafe { rs(name) };
    emit(&format!("u_put({:?},data={:?})", nm, data));
    if nm == "x" {
        1
    } else {
        0
    }
}

unsafe extern "C" fn u_del(_J: JS, data: *mut c_void, name: *const c_char) -> c_int {
    let nm = unsafe { rs(name) };
    emit(&format!("u_del({:?},data={:?})", nm, data));
    if nm == "d" {
        1
    } else {
        0
    }
}

unsafe extern "C" fn u_fin(_J: JS, data: *mut c_void) {
    ud_fin_inc();
    emit(&format!("u_fin(data={:?})", data));
}

/// Rows 61, 62, 64-68 (row 63 lives in `freestate_teardown_and_finalizers`) —
/// `js_newuserdata` / `js_newuserdatax` over the full callback
/// matrix (has/put/del/finalize present or absent) x prototype shape (object /
/// null / non-object), driving `js_isuserdata`, `js_touserdata`,
/// `js_hasproperty` (row 64), `js_setproperty` + `js_defproperty` (row 65),
/// `js_delproperty` (row 66) and the GC finalizer (row 62, called exactly once).
#[test]
fn userdata_callback_matrix() {
    for mask in 0i64..16 {
        for proto in 0i64..3 {
            set_pi(0, mask);
            set_pi(1, proto);
            fn act(a: &Api, J: JS) {
                unsafe {
                    ud_fin_set(0);
                    let m = pi(0);
                    let has: Option<HasProp> = if m & 1 != 0 { Some(u_has) } else { None };
                    let put: Option<PutProp> = if m & 2 != 0 { Some(u_put) } else { None };
                    let del: Option<DelProp> = if m & 4 != 0 { Some(u_del) } else { None };
                    let fin: Option<Finalize> = if m & 8 != 0 { Some(u_fin) } else { None };
                    let data = 0xBEEF as *mut c_void;
                    match pi(1) {
                        0 => (a.js_newobject)(J),
                        1 => (a.js_pushnull)(J),
                        _ => (a.js_pushnumber)(J, 5.0),
                    }
                    if m == 0 {
                        /* row 68 == plain js_newuserdata with a NULL finalizer */
                        (a.js_newuserdata)(J, cptr(TAG_A), data, None);
                    } else if m == 8 {
                        /* row 62/63 shape: plain js_newuserdata + finalizer */
                        (a.js_newuserdata)(J, cptr(TAG_A), data, Some(u_fin));
                    } else {
                        (a.js_newuserdatax)(J, cptr(TAG_A), data, has, put, del, fin);
                    }
                    emit(&format!("repr={}", repr_at(a, J, -1)));
                    emit(&format!(
                        "typeof={} type={} isud_a={} isud_b={} ud_a={:?}",
                        rs((a.js_typeof)(J, -1)),
                        (a.js_type)(J, -1),
                        (a.js_isuserdata)(J, -1, cptr(TAG_A)),
                        (a.js_isuserdata)(J, -1, cptr(TAG_B)),
                        (a.js_touserdata)(J, -1, cptr(TAG_A)),
                    ));
                    /* row 64 */
                    probe_has(a, J, -1, "virtual");
                    probe_has(a, J, -1, "other");
                    probe_has(a, J, -1, "toString");
                    /* row 65 */
                    (a.js_pushnumber)(J, 7.0);
                    (a.js_setproperty)(J, -2, cs("x").as_ptr());
                    probe_get(a, J, -1, "x");
                    (a.js_pushnumber)(J, 8.0);
                    (a.js_setproperty)(J, -2, cs("y").as_ptr());
                    probe_get(a, J, -1, "y");
                    (a.js_pushnumber)(J, 9.0);
                    (a.js_defproperty)(J, -2, cs("x").as_ptr(), 0);
                    probe_get(a, J, -1, "x");
                    (a.js_pushnumber)(J, 10.0);
                    (a.js_defproperty)(J, -2, cs("z").as_ptr(), JS_READONLY);
                    probe_get(a, J, -1, "z");
                    /* row 66 */
                    (a.js_pushnumber)(J, 1.0);
                    (a.js_setproperty)(J, -2, cs("d").as_ptr());
                    (a.js_delproperty)(J, -1, cs("d").as_ptr());
                    probe_has(a, J, -1, "d");
                    (a.js_delproperty)(J, -1, cs("y").as_ptr());
                    probe_has(a, J, -1, "y");
                    /* own-property enumeration is unaffected by the callbacks */
                    (a.js_pushiterator)(J, -1, 1);
                    let mut k = 0;
                    loop {
                        let nm = (a.js_nextiterator)(J, -1);
                        if nm.is_null() {
                            break;
                        }
                        emit(&format!("it={:?}", rs(nm)));
                        k += 1;
                        if k > 16 {
                            break;
                        }
                    }
                    (a.js_pop)(J, 1);
                    /* row 62: drop it and collect -> finalize exactly once */
                    (a.js_pop)(J, (a.js_gettop)(J) - 1);
                    (a.js_gc)(J, 0);
                    emit(&format!("fin1={}", ud_fin_get()));
                    (a.js_gc)(J, 0);
                    emit(&format!("fin2={}", ud_fin_get()));
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            for f in [0, JS_STRICT] {
                diff_native(&format!("userdata mask={} proto={}", mask, proto), act, f);
            }
            let p = libs();
            let c = p.c.run_native(act, 0);
            let lbl = format!("mask={} proto={}", mask, proto);
            assert!(c.contains("isud_a=1 isud_b=0"), "{}: {}", lbl, c);
            assert!(c.contains("[userdata tagA]"), "{}: {}", lbl, c);
            /* row 64: `has` short-circuits the property tree for "virtual" */
            if mask & 1 != 0 {
                assert!(c.contains("u_has(\"virtual\""), "{}: {}", lbl, c);
                assert!(c.contains("has \"virtual\"=1 v=111"), "{}: {}", lbl, c);
            } else {
                assert!(c.contains("has \"virtual\"=0"), "{}: {}", lbl, c);
            }
            /* row 65: `put` swallows the write of "x" (both set and def) */
            if mask & 2 != 0 {
                assert!(c.contains("u_put(\"x\""), "{}: {}", lbl, c);
                assert!(!c.contains("get \"x\"=9"), "{}: {}", lbl, c);
            } else {
                assert!(c.contains("get \"x\"=7"), "{}: {}", lbl, c);
                assert!(c.contains("get \"x\"=9"), "{}: {}", lbl, c);
            }
            /* row 66: `del` claims the delete of "d" */
            if mask & 4 != 0 {
                assert!(c.contains("u_del(\"d\""), "{}: {}", lbl, c);
            }
            /* row 62: the finalizer runs exactly once, on the first GC */
            if mask & 8 != 0 {
                assert!(c.contains("fin1=1") && c.contains("fin2=1"), "{}: {}", lbl, c);
            } else {
                assert!(c.contains("fin1=0") && c.contains("fin2=0"), "{}: {}", lbl, c);
            }
            /* row 61/170: a non-object prototype means no inherited toString */
            if proto == 0 {
                assert!(c.contains("has \"toString\"=1"), "{}: {}", lbl, c);
            } else {
                assert!(c.contains("has \"toString\"=0"), "{}: {}", lbl, c);
            }
        }
    }
}

/* ========================================================================== */
/* rows 70-72, 74: cfunctions and cconstructors                                */
/* ========================================================================== */

unsafe extern "C" fn cf_fin(_J: JS, data: *mut c_void) {
    cf_fin_inc();
    emit(&format!("cf_fin(data={:?})", data));
}

unsafe extern "C" fn cf_report(J: JS) {
    let a = cur();
    unsafe {
        let n = (a.js_gettop)(J);
        emit(&format!("cf top={}", n));
        for i in 0..n {
            emit(&format!("cf[{}]={}", i, repr_at(a, J, i)));
        }
        emit(&format!("cf data={:?}", (a.js_currentfunctiondata)(J)));
        (a.js_currentfunction)(J);
        emit(&format!("cf cur={}", repr_at(a, J, -1)));
        (a.js_pop)(J, 1);
        (a.js_pushnumber)(J, n as f64);
    }
}

unsafe extern "C" fn cf_ctor(J: JS) {
    let a = cur();
    unsafe {
        emit(&format!(
            "ctor top={} this_null={} this_undef={} data={:?}",
            (a.js_gettop)(J),
            (a.js_isnull)(J, 0),
            (a.js_isundefined)(J, 0),
            (a.js_currentfunctiondata)(J)
        ));
        (a.js_newobject)(J);
        (a.js_pushnumber)(J, 9.0);
        (a.js_setproperty)(J, -2, cs("made").as_ptr());
    }
}

/// Rows 70, 71, 72, 74 — `js_newcfunction` (no data / no finalizer),
/// `js_newcfunctionx` (data and/or finalizer, `js_currentfunctiondata`, the GC
/// finalizer) and `js_newcconstructor` (`u.c.function` for a plain call,
/// `u.c.constructor` for `js_construct` with `this == null`, and `ccon == NULL`
/// falling through to the generic construct path), each for `length` 0/1/3 and
/// call sites with fewer / exactly / more arguments than `length`
/// (`jsR_callcfunction` pads with `undefined` up to `min == u.c.length`).
#[test]
fn cfunction_shapes_lengths_and_finalizers() {
    for kind in 0i64..6 {
        for len in [0i64, 1, 3] {
            for nargs in [0i64, 1, 2, 4] {
                for op in 0i64..2 {
                    set_pi(0, kind);
                    set_pi(1, len);
                    set_pi(2, nargs);
                    set_pi(3, op);
                    fn build(a: &Api, J: JS) {
                        unsafe {
                            let len = pic(1);
                            match pi(0) {
                                0 => (a.js_newcfunction)(J, Some(cf_report), cptr(N_FN), len),
                                1 => (a.js_newcfunctionx)(
                                    J,
                                    Some(cf_report),
                                    cptr(N_FN),
                                    len,
                                    0xD00D as *mut c_void,
                                    Some(cf_fin),
                                ),
                                2 => (a.js_newcfunctionx)(
                                    J,
                                    Some(cf_report),
                                    cptr(N_FN),
                                    len,
                                    0xD00D as *mut c_void,
                                    None,
                                ),
                                3 => (a.js_newcfunctionx)(
                                    J,
                                    Some(cf_report),
                                    cptr(N_FN),
                                    len,
                                    std::ptr::null_mut(),
                                    Some(cf_fin),
                                ),
                                4 => {
                                    (a.js_newobject)(J); /* prototype for the ctor */
                                    (a.js_newcconstructor)(
                                        J,
                                        Some(cf_report),
                                        Some(cf_ctor),
                                        cptr(N_CTOR),
                                        len,
                                    )
                                }
                                _ => {
                                    (a.js_newobject)(J);
                                    (a.js_newcconstructor)(J, Some(cf_report), None, cptr(N_CTOR), len)
                                }
                            }
                        }
                    }
                    fn act(a: &Api, J: JS) {
                        unsafe {
                            cf_fin_set(0);
                            let nargs = pi(2);
                            build(a, J);
                            emit(&format!("fn={}", repr_at(a, J, -1)));
                            emit(&format!(
                                "callable={} typeof={} len={} data={:?}",
                                (a.js_iscallable)(J, -1),
                                rs((a.js_typeof)(J, -1)),
                                (a.js_getlength)(J, -1),
                                (a.js_currentfunctiondata)(J)
                            ));
                            /* row 70: the shape of `length` and `prototype` */
                            (a.js_copy)(J, -1);
                            (a.js_setglobal)(J, cs("F").as_ptr());
                            /* An IIFE, so the descriptor objects stay LOCAL: a
                             * script-level `var` would keep F alive through the
                             * `constructor` descriptor and defeat the row-72
                             * finalizer check below. */
                            run_expr(
                                a,
                                J,
                                "(function(){\
                                 var d=Object.getOwnPropertyDescriptor(F,'length');\
                                 var p=Object.getOwnPropertyDescriptor(F,'prototype');\
                                 var c=Object.getOwnPropertyDescriptor(F.prototype,'constructor');\
                                 return [d.value,d.writable,d.enumerable,d.configurable,\
                                  (typeof F.prototype),p.enumerable,p.configurable,p.writable,\
                                  (c.value===F),c.enumerable].join('/')})()",
                                "shape",
                            );
                            /* row 71 / 74: call or construct with nargs arguments */
                            if pi(3) == 0 {
                                (a.js_copy)(J, -1);
                                (a.js_pushundefined)(J);
                                for k in 0..nargs {
                                    (a.js_pushnumber)(J, 1.0 + k as f64);
                                }
                                let rc = (a.js_pcall)(J, nargs as c_int);
                                emit(&format!("pcall={} res={}", rc, repr_at(a, J, -1)));
                                (a.js_pop)(J, 1);
                            } else {
                                (a.js_pushnumber)(J, -999.0); /* pconstruct scratch slot */
                                (a.js_copy)(J, -2);
                                for k in 0..nargs {
                                    (a.js_pushnumber)(J, 1.0 + k as f64);
                                }
                                let rc = (a.js_pconstruct)(J, nargs as c_int);
                                emit(&format!("pconstruct={} res={}", rc, repr_at(a, J, -1)));
                                (a.js_pop)(J, (a.js_gettop)(J) - 2);
                            }
                            /* also drive it from JS */
                            run_expr(a, J, "[F(1,2,3), typeof F, F.length, F.name].join('/')", "js");
                            /* row 72: unreachable -> finalize */
                            (a.js_pop)(J, (a.js_gettop)(J) - 1);
                            run_expr(a, J, "delete this.F", "unref");
                            (a.js_gc)(J, 0);
                            emit(&format!("fin1={}", cf_fin_get()));
                            (a.js_gc)(J, 0);
                            emit(&format!("fin2={}", cf_fin_get()));
                            (a.js_pushnumber)(J, 0.0);
                        }
                    }
                    let lbl = format!("cfun kind={} len={} nargs={} op={}", kind, len, nargs, op);
                    diff_native(&lbl, act, 0);
                    let p = libs();
                    let c = p.c.run_native(act, 0);
                    /* row 70: `length` is READONLY|DONTENUM|DONTCONF and
                     * `prototype` is DONTENUM|DONTCONF with a DONTENUM
                     * `constructor` back-reference */
                    assert!(
                        c.contains(&format!(
                            "shape rc=0 \"{}/false/false/false/object/false/false/true/true/false\"",
                            len
                        )),
                        "{}: {}",
                        lbl,
                        c
                    );
                    /* row 71: jsR_callcfunction pads the arguments with
                     * undefined up to min == u.c.length (and never truncates) */
                    if op == 0 {
                        let want = 1 + std::cmp::max(len, nargs);
                        assert!(c.contains(&format!("cf top={}", want)), "{}: {}", lbl, c);
                    }
                    /* rows 70/72: js_currentfunctiondata */
                    if kind == 1 || kind == 2 {
                        assert!(c.contains("cf data=0xd00d"), "{}: {}", lbl, c);
                    } else {
                        assert!(c.contains("cf data=0x0"), "{}: {}", lbl, c);
                    }
                    /* row 72: the finalizer runs once the function is dropped */
                    if kind == 1 || kind == 3 {
                        assert!(c.contains("fin1=1") && c.contains("fin2=1"), "{}: {}", lbl, c);
                    } else {
                        assert!(c.contains("fin1=0"), "{}: {}", lbl, c);
                    }
                    /* row 74: js_construct takes u.c.constructor with this==null
                     * when one is installed, otherwise the generic path */
                    if op == 1 {
                        if kind == 4 {
                            assert!(c.contains("this_null=1"), "{}: {}", lbl, c);
                        } else {
                            assert!(!c.contains("this_null=1"), "{}: {}", lbl, c);
                        }
                    }
                }
            }
        }
    }
}

/* ========================================================================== */
/* row 97: js_pushliteral                                                      */
/* ========================================================================== */

/// Row 97 — `JS_TLITSTR`: the pointer is stored verbatim (no copy, not
/// GC-scanned), so `js_tostring` hands back the very same pointer; the value is
/// still a string for every predicate and compares equal to a copied twin.
#[test]
fn pushliteral_pointer_identity() {
    for which in 0i64..4 {
        set_pi(0, which);
        fn act(a: &Api, J: JS) {
            unsafe {
                let lit = match pi(0) {
                    0 => cptr(LIT_EMPTY),
                    1 => cptr(LIT_SHORT),
                    2 => cptr(LIT_15),
                    _ => cptr(LIT_16),
                };
                (a.js_pushliteral)(J, lit);
                let back = (a.js_tostring)(J, -1);
                emit(&format!(
                    "same_ptr={} isstring={} typeof={} type={} bool={} num={:#x} repr={}",
                    back == lit,
                    (a.js_isstring)(J, -1),
                    rs((a.js_typeof)(J, -1)),
                    (a.js_type)(J, -1),
                    (a.js_toboolean)(J, -1),
                    (a.js_tonumber)(J, -1).to_bits(),
                    repr_at(a, J, -1)
                ));
                /* js_getlength autoboxes the slot (js_getproperty -> js_toobject
                 * rewrites it in place), so measure a COPY and leave the literal
                 * value itself untouched. */
                (a.js_copy)(J, -1);
                emit(&format!("len={}", (a.js_getlength)(J, -1)));
                emit(&format!("boxed={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                /* a copied twin: equal by value, different pointer */
                (a.js_pushstring)(J, lit);
                let copy = (a.js_tostring)(J, -1);
                emit(&format!("copy_same_ptr={}", copy == lit));
                emit(&format!(
                    "equal={} strictequal={}",
                    (a.js_equal)(J),
                    (a.js_strictequal)(J)
                ));
                /* literals survive a GC (they are not GC objects at all) */
                (a.js_gc)(J, 0);
                emit(&format!("after_gc={}", repr_at(a, J, -2)));
                emit(&format!("still_same_ptr={}", (a.js_tostring)(J, -2) == lit));
                (a.js_pop)(J, 2);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("pushliteral {}", which), act, f);
        }
        /* row 97: the pointer is stored verbatim; js_pushstring copies */
        let p = libs();
        let c = p.c.run_native(act, 0);
        assert!(c.contains("same_ptr=true"), "row 97 (which={}): {}", which, c);
        assert!(c.contains("still_same_ptr=true"), "row 97 gc (which={}): {}", which, c);
        assert!(c.contains("isstring=1 typeof=string"), "row 97: {}", c);
        assert!(c.contains("equal=1 strictequal=1"), "row 97 equality: {}", c);
        if which == 0 {
            /* the empty literal */
            assert!(c.contains("len=0"), "row 97 empty: {}", c);
        } else {
            /* a copied twin is a *different* pointer (shrstr / memstr) */
            assert!(c.contains("copy_same_ptr=false"), "row 97 copy: {}", c);
        }
    }
}

/* ========================================================================== */
/* rows 116-118: js_gc                                                         */
/* ========================================================================== */

/// Rows 116, 117, 118 — `js_gc(J,0)` (no report) on a brand-new state and after
/// allocating + dropping objects, `js_gc(J,1)`'s report line (with a report
/// callback installed AND with `J->report == NULL`), and the implicit
/// `js_gc(J,0)` that `jsR_run` performs once `gccounter > gcthresh`
/// (`JS_GCFACTOR` stabilisation over several cycles).
#[test]
fn gc_report_and_implicit_collection() {
    for mode in 0i64..5 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                match pi(0) {
                    /* row 116: a fresh state, no report */
                    0 => {
                        let J2 = (a.js_newstate)(None, std::ptr::null_mut(), 0);
                        emit(&format!("J2={}", !J2.is_null()));
                        if !J2.is_null() {
                            (a.js_setreport)(J2, Some(rep_emit));
                            (a.js_gc)(J2, 0);
                            emit("gc0-done");
                            (a.js_gc)(J2, 1);
                            (a.js_gc)(J2, 1);
                            (a.js_freestate)(J2);
                        }
                    }
                    /* row 116/117: allocate and drop 100 objects */
                    1 => {
                        (a.js_setreport)(J, Some(rep_emit));
                        run_expr(
                            a,
                            J,
                            "var a=[];for(var i=0;i<100;i++)a.push({x:i});a=null;'built'",
                            "build",
                        );
                        (a.js_gc)(J, 1);
                        (a.js_gc)(J, 1);
                        (a.js_gc)(J, 1);
                    }
                    /* row 117: report path with J->report == NULL */
                    2 => {
                        (a.js_setreport)(J, None);
                        run_expr(a, J, "var a=[];for(var i=0;i<100;i++)a.push({x:i});a.length", "build");
                        (a.js_gc)(J, 1);
                        (a.js_gc)(J, 1);
                        emit("no-report-gc-done");
                    }
                    /* row 118: implicit collection from jsR_run */
                    3 => {
                        for round in 0..4 {
                            run_expr(
                                a,
                                J,
                                "var a=[];for(var i=0;i<300;i++)a.push({x:i,s:'s'+i});a.length",
                                "round",
                            );
                            emit(&format!("round={}", round));
                        }
                        (a.js_setreport)(J, Some(rep_emit));
                        (a.js_gc)(J, 1);
                        (a.js_gc)(J, 1);
                    }
                    /* strings and properties are reclaimed too */
                    _ => {
                        (a.js_setreport)(J, Some(rep_emit));
                        run_expr(
                            a,
                            J,
                            "var s='';for(var i=0;i<200;i++)s=('x'+i);var o={};for(var j=0;j<50;j++)o['k'+j]=j;o=null;s",
                            "strings",
                        );
                        (a.js_gc)(J, 1);
                        (a.js_gc)(J, 1);
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("gc mode={}", mode), act, f);
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        if mode == 1 {
            assert!(c.contains("garbage collected"), "mode={}: {}", mode, c);
        }
        if mode == 2 {
            assert!(!c.contains("garbage collected"), "mode={}: {}", mode, c);
        }
    }
}

/* ========================================================================== */
/* row 123: jsS_dumpstrings                                                    */
/* ========================================================================== */

/// Row 123 — `jsS_dumpstrings` on (a) a state whose intern tree has just been
/// created and (b) after several `js_ref`s have populated the AA tree, so the
/// recursive `dumpstringnode` walk runs. Both go to stdout.
#[test]
fn dumpstrings_fresh_and_populated() {
    let p = libs();
    fn act(a: &Api, J: JS) {
        unsafe {
            let J2 = (a.js_newstate)(None, std::ptr::null_mut(), 0);
            if J2.is_null() {
                emit("no-state");
                (a.js_pushnumber)(J, 0.0);
                return;
            }
            /* (a) as close to "fresh" as the API allows */
            (a.jsS_dumpstrings)(J2);
            /* (b) intern a deterministic set of names through js_ref + js_intern */
            for k in 0..12 {
                (a.js_pushnumber)(J2, (k * 7 % 12) as f64);
                let r = (a.js_ref)(J2);
                let _ = r;
            }
            for k in 0..24 {
                let s = cs(&format!("key{:02}", k * 11 % 24));
                let _ = (a.js_intern)(J2, s.as_ptr());
            }
            (a.jsS_dumpstrings)(J2);
            (a.js_freestate)(J2);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    let oc = capture_stdout(|| {
        let _ = p.c.run_native(act, 0);
    });
    let or_ = capture_stdout(|| {
        let _ = p.r.run_native(act, 0);
    });
    same("jsS_dumpstrings", &mask_ptrs(&oc), &mask_ptrs(&or_));
    assert!(oc.contains("interned strings {"), "no dump: {:?}", oc);
}

/* ========================================================================== */
/* rows 119-122: js_ref / js_unref / registry                                  */
/* ========================================================================== */

/// Rows 119, 120, 121, 122 — `js_ref`'s three naming branches (the fixed
/// `_Undefined`/`_Null`/`_True`/`_False` names, the interned `"%p"` name of an
/// object, and the `J->nextref` counter for everything else), `js_unref`, and
/// the `js_getregistry`/`js_setregistry`/`js_delregistry` round trip.
#[test]
fn refs_and_registry_names() {
    fn act(a: &Api, J: JS) {
        unsafe {
            /* row 122: plain names */
            (a.js_getregistry)(J, cs("missing").as_ptr());
            emit(&format!("missing={}", repr_at(a, J, -1)));
            (a.js_pop)(J, 1);
            (a.js_pushnumber)(J, 7.0);
            (a.js_setregistry)(J, cs("k").as_ptr());
            emit(&format!("after_set top={}", (a.js_gettop)(J)));
            (a.js_getregistry)(J, cs("k").as_ptr());
            emit(&format!("k={}", repr_at(a, J, -1)));
            (a.js_pop)(J, 1);
            (a.js_delregistry)(J, cs("k").as_ptr());
            (a.js_getregistry)(J, cs("k").as_ptr());
            emit(&format!("k_after_del={}", repr_at(a, J, -1)));
            (a.js_pop)(J, 1);

            /* row 119: the fixed names, twice each (same name both times) */
            for k in 0..4 {
                let mut names: [String; 2] = [String::new(), String::new()];
                for t in 0..2 {
                    match k {
                        0 => (a.js_pushundefined)(J),
                        1 => (a.js_pushnull)(J),
                        2 => (a.js_pushboolean)(J, 1),
                        _ => (a.js_pushboolean)(J, 0),
                    }
                    let r = (a.js_ref)(J);
                    names[t] = rs(r);
                    (a.js_getregistry)(J, r);
                    emit(&format!("fixed{}[{}]={:?} v={}", k, t, names[t], repr_at(a, J, -1)));
                    (a.js_pop)(J, 1);
                }
                emit(&format!("fixed{}_same={}", k, names[0] == names[1]));
                /* unref and look again */
                let n0 = cs(&names[0]);
                (a.js_unref)(J, n0.as_ptr());
                (a.js_getregistry)(J, n0.as_ptr());
                emit(&format!("fixed{}_after_unref={}", k, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }

            /* row 120: objects are named by address; the same object twice gives
             * the same name, two objects give different names, and the referenced
             * object survives a GC because it is reachable from J->R. */
            (a.js_newobject)(J);
            (a.js_pushnumber)(J, 5.0);
            (a.js_setproperty)(J, -2, cs("tag").as_ptr());
            (a.js_copy)(J, -1);
            let r1 = rs((a.js_ref)(J));
            (a.js_copy)(J, -1);
            let r2 = rs((a.js_ref)(J));
            emit(&format!("obj_same_name={}", r1 == r2));
            emit(&format!("obj_name={:?}", r1));
            (a.js_pop)(J, 1); /* drop our own reference */
            (a.js_gc)(J, 0);
            let c1 = cs(&r1);
            (a.js_getregistry)(J, c1.as_ptr());
            emit(&format!("obj_alive_after_gc={}", repr_at(a, J, -1)));
            (a.js_pop)(J, 1);
            (a.js_newobject)(J);
            let r3 = rs((a.js_ref)(J));
            emit(&format!("distinct_objects={}", r1 != r3));
            (a.js_unref)(J, c1.as_ptr());
            (a.js_getregistry)(J, c1.as_ptr());
            emit(&format!("obj_after_unref={}", repr_at(a, J, -1)));
            (a.js_pop)(J, 1);

            /* row 121: the counter branch — identical values get distinct refs */
            for k in 0..6 {
                match k % 3 {
                    0 => (a.js_pushnumber)(J, 42.0),
                    1 => (a.js_pushstring)(J, cs("s").as_ptr()),
                    _ => (a.js_pushliteral)(J, cptr(LIT_SHORT)),
                }
                let r = rs((a.js_ref)(J));
                emit(&format!("counter{}={:?}", k, r));
                let cr = cs(&r);
                (a.js_getregistry)(J, cr.as_ptr());
                emit(&format!("counter{}_v={}", k, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        let p = libs();
        /* object ref names are heap addresses: mask them */
        same(
            &format!("refs/registry flags={}", f),
            &mask_ptrs(&p.c.run_native(act, f)),
            &mask_ptrs(&p.r.run_native(act, f)),
        );
    }
    /* ground truth for every naming branch of js_ref */
    let p = libs();
    let c = mask_ptrs(&p.c.run_native(act, 0));
    for (k, name) in ["_Undefined", "_Null", "_True", "_False"].iter().enumerate() {
        assert!(
            c.contains(&format!("fixed{}[0]={:?}", k, name)) && c.contains(&format!("fixed{}_same=true", k)),
            "row 119 {}: {}",
            name,
            c
        );
        assert!(
            c.contains(&format!("fixed{}_after_unref=undefined", k)),
            "row 119 unref {}: {}",
            name,
            c
        );
    }
    assert!(c.contains("obj_same_name=true"), "row 120: {}", c);
    assert!(c.contains("distinct_objects=true"), "row 120: {}", c);
    assert!(c.contains("obj_name=\"0xPTR\""), "row 120 name shape: {}", c);
    assert!(c.contains("obj_alive_after_gc={tag: 5}"), "row 120 gc: {}", c);
    assert!(c.contains("obj_after_unref=undefined"), "row 120 unref: {}", c);
    for k in 0..6 {
        assert!(
            c.contains(&format!("counter{}={:?}", k, k.to_string())),
            "row 121 counter {}: {}",
            k,
            c
        );
    }
    assert!(c.contains("missing=undefined") && c.contains("k=7") && c.contains("k_after_del=undefined"),
        "row 122: {}", c);

    /* --- property style: randomized registry names and referenced values ---- */
    let mut rng = Rng::new(SEED ^ 0x77);
    for iter in 0..250 {
        let name = match rng.below(5) {
            0 => "".to_string(),
            1 => format!("n{}", rng.below(8)),
            2 => format!("{}", rng.below(8)),
            3 => "_Undefined".to_string(), /* collides with a fixed ref name */
            _ => rng.string(8),
        };
        set_ps(0, &name);
        set_pi(0, rng.range_i64(0, 8));
        set_pf(0, rng.f64());
        fn rnd(a: &Api, J: JS) {
            unsafe {
                let name = ps(0);
                /* a registry round trip under an arbitrary name */
                (a.js_getregistry)(J, name.as_ptr());
                emit(&format!("get0={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                match pi(0) {
                    0 => (a.js_pushundefined)(J),
                    1 => (a.js_pushnull)(J),
                    2 => (a.js_pushboolean)(J, 1),
                    3 => (a.js_pushboolean)(J, 0),
                    4 => (a.js_pushnumber)(J, pf(0)),
                    5 => (a.js_pushstring)(J, name.as_ptr()),
                    6 => (a.js_newobject)(J),
                    7 => (a.js_newarray)(J),
                    _ => (a.js_pushliteral)(J, cptr(LIT_SHORT)),
                }
                (a.js_copy)(J, -1);
                (a.js_setregistry)(J, name.as_ptr());
                emit(&format!("after_set top={}", (a.js_gettop)(J)));
                (a.js_getregistry)(J, name.as_ptr());
                emit(&format!("get1={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                /* js_ref names the SAME value and must round trip too */
                let r = (a.js_ref)(J);
                let rn = rs(r);
                emit(&format!("ref={:?}", rn));
                let rc = cs(&rn);
                (a.js_getregistry)(J, rc.as_ptr());
                emit(&format!("deref={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                (a.js_unref)(J, rc.as_ptr());
                (a.js_getregistry)(J, rc.as_ptr());
                emit(&format!("after_unref={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                (a.js_delregistry)(J, name.as_ptr());
                (a.js_getregistry)(J, name.as_ptr());
                emit(&format!("get2={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                (a.js_gc)(J, 0);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        let p = libs();
        let f = if iter % 2 == 0 { 0 } else { JS_STRICT };
        same(
            &format!("registry rnd iter={} name={:?} kind={}", iter, name, pi(0)),
            &mask_ptrs(&p.c.run_native(rnd, f)),
            &mask_ptrs(&p.r.run_native(rnd, f)),
        );
    }
}

/* ========================================================================== */
/* rows 127, 128: js_torepr / js_tryrepr                                       */
/* ========================================================================== */

/// Rows 127 + 128 — `js_torepr` replaces the value AT `idx` (`idx-1` for a
/// negative index, because `js_repr` has already pushed the string) leaving
/// `js_gettop` unchanged, and `js_tryrepr` returns its `error` default and pops
/// the exception when the repr throws, where `js_torepr` propagates.
#[test]
fn torepr_index_semantics_and_tryrepr() {
    for idx in [-1i64, -2, -3, 0, 1, 2] {
        set_pi(0, idx);
        fn act(a: &Api, J: JS) {
            unsafe {
                (a.js_pushnumber)(J, 1.5);
                (a.js_pushstring)(J, cs("two").as_ptr());
                (a.js_newobject)(J);
                (a.js_pushnumber)(J, 3.0);
                (a.js_setproperty)(J, -2, cs("k").as_ptr());
                emit(&format!("before top={}", (a.js_gettop)(J)));
                dump(a, J);
                let s = (a.js_torepr)(J, pic(0));
                emit(&format!("torepr({})={:?}", pi(0), rs(s)));
                emit(&format!("after top={}", (a.js_gettop)(J)));
                dump(a, J);
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("torepr idx={}", idx), act, f);
        }
        /* row 127: js_gettop is unchanged and the slot now holds the repr string */
        let p = libs();
        let c = p.c.run_native(act, 0);
        assert!(
            c.contains("before top=4") && c.contains("after top=4"),
            "row 127 idx={}: {}",
            idx,
            c
        );
        match idx {
            -1 | 3 => assert!(c.contains("torepr(-1)=\"{k: 3}\"") || c.contains("torepr(3)=\"{k: 3}\""), "row 127: {}", c),
            -2 | 2 => assert!(c.contains("=\"\\\"two\\\"\""), "row 127 idx={}: {}", idx, c),
            -3 | 1 => assert!(c.contains("=\"1.5\""), "row 127 idx={}: {}", idx, c),
            _ => assert!(c.contains("=\"undefined\""), "row 127 idx={}: {}", idx, c),
        }
    }

    /* row 128: a throwing getter */
    for mode in 0i64..3 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                let nm = cs("g.js");
                let src = cs("({get x(){ throw new Error('getter') }, y:1})");
                if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) != 0 {
                    emit("loadfail");
                    (a.js_pop)(J, 1);
                    (a.js_pushnumber)(J, 0.0);
                    return;
                }
                (a.js_pushundefined)(J);
                if (a.js_pcall)(J, 0) != 0 {
                    emit("callfail");
                    (a.js_pop)(J, 1);
                    (a.js_pushnumber)(J, 0.0);
                    return;
                }
                emit(&format!("built top={}", (a.js_gettop)(J)));
                match pi(0) {
                    0 => {
                        let e = cs("ERR");
                        let s = (a.js_tryrepr)(J, -1, e.as_ptr());
                        emit(&format!("tryrepr={:?} top={}", rs(s), (a.js_gettop)(J)));
                        emit(&format!("value_intact={}", (a.js_isobject)(J, -1)));
                    }
                    1 => {
                        let s = (a.js_tryrepr)(J, -1, std::ptr::null());
                        emit(&format!("tryrepr_null={:?} top={}", rs(s), (a.js_gettop)(J)));
                    }
                    _ => {
                        /* js_torepr propagates: the pcall of the harness catches it */
                        let s = (a.js_torepr)(J, -1);
                        emit(&format!("torepr={:?}", rs(s)));
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("tryrepr mode={}", mode), act, f);
        }
        /* row 128: js_tryrepr swallows the exception and returns `error`,
         * js_torepr propagates it */
        let p = libs();
        let c = p.c.run_native(act, 0);
        match mode {
            0 => assert!(
                c.contains("tryrepr=\"ERR\"") && c.contains("value_intact=1"),
                "row 128: {}",
                c
            ),
            1 => assert!(c.contains("tryrepr_null=\"<null>\""), "row 128 NULL error: {}", c),
            _ => assert!(
                c.contains("throw(1) Error: getter") && !c.contains("survived"),
                "row 128 js_torepr must propagate: {}",
                c
            ),
        }
    }
}

/* ========================================================================== */
/* rows 129, 130: js_ploadstring                                               */
/* ========================================================================== */

/// Rows 129 + 130 — `js_ploadstring` over valid sources (rc 0, a callable
/// `JS_CSCRIPT` object on the stack that runs) and every class of invalid source
/// (rc 1 with a SyntaxError object, the parse arena released so the next load
/// still works), including the `JS_ASTLIMIT` "too much recursion" guard.
///
/// `filename` variants: `"test.js"`, another name, and `""`.  A NULL `filename`
/// is NOT exercised: `jsC_compilescript` does `F->filename = js_intern(J,
/// J->filename)` and `jsS_insert` `strcmp`s it, so a NULL filename is a NULL
/// dereference in BOTH libraries (verified: identical SIGSEGV), exactly like the
/// out-of-range `js_pushlstring` length documented in `phase_c_runtime`.
#[test]
fn ploadstring_filename_and_error_shapes() {
    let deep_ok = format!("{}1{}", "(".repeat(390), ")".repeat(390));
    let deep_bad = format!("{}1{}", "(".repeat(410), ")".repeat(410));
    let srcs: Vec<String> = vec![
        "1+1".into(),
        "".into(),
        "   ".into(),
        "//just a comment".into(),
        "var x=1; x*2".into(),
        "function f(){return 3} f()".into(),
        "'\\u0041'".into(),
        /* invalid */
        "function{".into(),
        "var".into(),
        "'abc".into(),
        "/*unterminated".into(),
        "(".into(),
        ")".into(),
        "var 1".into(),
        "1 = 2".into(),
        "return 5".into(),
        "0x".into(),
        deep_ok,
        deep_bad,
    ];
    /* property style: random byte soup (mostly SyntaxErrors, occasionally not) */
    let mut rng = Rng::new(SEED ^ 0x81);
    let mut srcs = srcs;
    for _ in 0..80 {
        srcs.push(rng.string(18));
    }
    for _ in 0..40 {
        /* random token soup that the lexer accepts more often */
        let toks = [
            "var ", "x", "1", "+", "(", ")", "{", "}", ";", "function", "return", " ", "'s'",
            "[", "]", ",", "if", "else", "==", "=",
        ];
        let n = rng.range_i64(1, 12) as usize;
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(toks[rng.below(toks.len() as u64) as usize]);
        }
        srcs.push(s);
    }
    for s in &srcs {
        for named in [0i64, 1, 2] {
            set_ps(0, s);
            set_pi(0, named);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let src = ps(0);
                    let nm = match pi(0) {
                        0 => cs(""),
                        1 => cs("test.js"),
                        _ => cs("some/other/name.js"),
                    };
                    let fname = nm.as_ptr();
                    let rc = (a.js_ploadstring)(J, fname, src.as_ptr());
                    emit(&format!("rc={}", rc));
                    if rc == 0 {
                        emit(&format!(
                            "typeof={} type={} callable={} isobject={} repr={}",
                            rs((a.js_typeof)(J, -1)),
                            (a.js_type)(J, -1),
                            (a.js_iscallable)(J, -1),
                            (a.js_isobject)(J, -1),
                            repr_at(a, J, -1)
                        ));
                        (a.js_pushundefined)(J);
                        let rc = (a.js_pcall)(J, 0);
                        emit(&format!("call={} res={}", rc, repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    } else {
                        emit(&format!(
                            "err={:?} iserror={} name-and-message",
                            str_at(a, J, -1),
                            (a.js_iserror)(J, -1)
                        ));
                        probe_get(a, J, -1, "name");
                        probe_get(a, J, -1, "message");
                        (a.js_pop)(J, 1);
                    }
                    /* the parse arena was released: a second load still works */
                    let ok = cs("2+2");
                    let rc2 = (a.js_ploadstring)(J, fname, ok.as_ptr());
                    emit(&format!("rc2={}", rc2));
                    (a.js_pop)(J, 1);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            for f in [0, JS_STRICT] {
                diff_native("ploadstring", act, f);
            }
        }
    }
}

/* ========================================================================== */
/* rows 132, 134: js_pcall / js_pconstruct success paths                       */
/* ========================================================================== */

/// Rows 132 + 134 — `js_pcall` success for several argument counts (rc 0, the
/// result at the top, the stack trimmed to `savetop+1`) and `js_pconstruct`
/// success through the generic path (`prototype` read, fresh `jsV_newobject`,
/// the created object returned when the body returns a non-object) as well as
/// the `js_rot2pop1` path (the body returns an object).
#[test]
fn pcall_and_pconstruct_success() {
    for n in [0i64, 1, 2, 3] {
        for kind in 0i64..6 {
            set_pi(0, n);
            set_pi(1, kind);
            fn act(a: &Api, J: JS) {
                unsafe {
                    let n = pi(0) as c_int;
                    /* define the callees */
                    run_expr(
                        a,
                        J,
                        "function C(x,y){ this.x=x; this.y=y; this.sum=(x||0)+(y||0) }\
                         function D(){ return {made:1} }\
                         function E(){ }; E.prototype=1;\
                         function S(a,b,c){ return [arguments.length,a,b,c].join('/') }\
                         'defined'",
                        "define",
                    );
                    emit(&format!("base top={}", (a.js_gettop)(J)));
                    (a.js_pushnumber)(J, -999.0); /* scratch slot for js_pconstruct */
                    match pi(1) {
                        0 => {
                            (a.js_newcfunction)(J, Some(cf_report), cptr(N_FN), 0);
                            (a.js_pushundefined)(J);
                            for k in 0..n {
                                (a.js_pushnumber)(J, 10.0 + k as f64);
                            }
                            let rc = (a.js_pcall)(J, n);
                            emit(&format!("pcall={} res={} top={}", rc, repr_at(a, J, -1), (a.js_gettop)(J)));
                        }
                        1 => {
                            (a.js_getglobal)(J, cs("S").as_ptr());
                            (a.js_pushundefined)(J);
                            for k in 0..n {
                                (a.js_pushnumber)(J, 10.0 + k as f64);
                            }
                            let rc = (a.js_pcall)(J, n);
                            emit(&format!("pcall={} res={} top={}", rc, repr_at(a, J, -1), (a.js_gettop)(J)));
                        }
                        2 => {
                            /* a `this` that is not undefined */
                            (a.js_getglobal)(J, cs("S").as_ptr());
                            (a.js_newobject)(J);
                            for k in 0..n {
                                (a.js_pushnumber)(J, 10.0 + k as f64);
                            }
                            let rc = (a.js_pcall)(J, n);
                            emit(&format!("pcall={} res={} top={}", rc, repr_at(a, J, -1), (a.js_gettop)(J)));
                        }
                        3 => {
                            (a.js_getglobal)(J, cs("C").as_ptr());
                            for k in 0..n {
                                (a.js_pushnumber)(J, 10.0 + k as f64);
                            }
                            let rc = (a.js_pconstruct)(J, n);
                            emit(&format!("pconstruct={} res={} top={}", rc, repr_at(a, J, -1), (a.js_gettop)(J)));
                            run_expr(a, J, "1", "still-alive");
                        }
                        4 => {
                            (a.js_getglobal)(J, cs("D").as_ptr());
                            for k in 0..n {
                                (a.js_pushnumber)(J, 10.0 + k as f64);
                            }
                            let rc = (a.js_pconstruct)(J, n);
                            emit(&format!("pconstruct={} res={} top={}", rc, repr_at(a, J, -1), (a.js_gettop)(J)));
                        }
                        _ => {
                            (a.js_getglobal)(J, cs("E").as_ptr());
                            for k in 0..n {
                                (a.js_pushnumber)(J, 10.0 + k as f64);
                            }
                            let rc = (a.js_pconstruct)(J, n);
                            emit(&format!("pconstruct={} res={} top={}", rc, repr_at(a, J, -1), (a.js_gettop)(J)));
                            run_expr(a, J, "String(new E() instanceof Object)", "protochain");
                        }
                    }
                    dump(a, J);
                    (a.js_pop)(J, (a.js_gettop)(J) - 1);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            for f in [0, JS_STRICT] {
                diff_native(&format!("pcall/pconstruct n={} kind={}", n, kind), act, f);
            }
            let p = libs();
            let c = p.c.run_native(act, 0);
            let lbl = format!("n={} kind={}", n, kind);
            match kind {
                /* row 132: rc 0, the result at the top, the stack trimmed back
                 * to savetop+1 (base top was 1 + the scratch slot) */
                0 => assert!(
                    c.contains(&format!("pcall=0 res={} top=3", 1 + n)),
                    "row 132 {}: {}",
                    lbl,
                    c
                ),
                1 | 2 => assert!(c.contains("pcall=0 res=") && c.contains("top=3"), "row 132 {}: {}", lbl, c),
                /* row 134: the generic construct path returns the new object */
                3 => assert!(
                    c.contains("pconstruct=0 res={sum:") && c.contains("still-alive rc=0"),
                    "row 134 {}: {}",
                    lbl,
                    c
                ),
                /* the body returned an object -> js_rot2pop1 keeps it */
                4 => assert!(c.contains("pconstruct=0 res={made: 1}"), "row 134 {}: {}", lbl, c),
                /* prototype is not an object -> falls back to Object_prototype */
                _ => assert!(
                    c.contains("pconstruct=0 res={}") && c.contains("protochain rc=0 \"true\""),
                    "row 134 {}: {}",
                    lbl,
                    c
                ),
            }
        }
    }
}

/* ========================================================================== */
/* row 137: js_loadstring vs js_eval (iseval / scope / strict)                  */
/* ========================================================================== */

unsafe extern "C" fn cf_load_or_eval(J: JS) {
    let a = cur();
    unsafe {
        let src = ps(0);
        match pi(1) {
            /* js_loadstring: compiled with J->default_strict, scope J->GE */
            0 => {
                (a.js_loadstring)(J, cs("l.js").as_ptr(), src.as_ptr());
                emit(&format!("loaded={}", repr_at(a, J, -1)));
                (a.js_pushundefined)(J);
                let rc = (a.js_pcall)(J, 0);
                emit(&format!("call={} res={}", rc, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }
            /* js_loadeval: compiled with J->strict, scope J->strict ? J->E : NULL */
            1 => {
                (a.js_loadeval)(J, cs("e.js").as_ptr(), src.as_ptr());
                emit(&format!("loadeval={}", repr_at(a, J, -1)));
                (a.js_pushundefined)(J);
                let rc = (a.js_pcall)(J, 0);
                emit(&format!("call={} res={}", rc, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }
            /* js_eval: loadeval + call with the current `this` */
            2 => {
                (a.js_pushstring)(J, src.as_ptr());
                (a.js_eval)(J);
                emit(&format!("eval={}", repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }
            /* js_eval with a NON-string on top returns immediately */
            _ => {
                (a.js_pushnumber)(J, 5.0);
                (a.js_eval)(J);
                emit(&format!("eval_nonstring={} top={}", repr_at(a, J, -1), (a.js_gettop)(J)));
                (a.js_pop)(J, 1);
            }
        }
        (a.js_pushnumber)(J, 1.0);
    }
}

/// Row 137 — the non-`p` entry points: `js_loadstring` compiles with
/// `J->default_strict` and the global scope, `js_loadeval`/`js_eval` with the
/// *current* `J->strict` and `J->strict ? J->E : NULL`.  The probe runs from a
/// strict and from a non-strict caller, in a strict and in a non-strict state.
#[test]
fn loadstring_versus_eval_strictness_and_scope() {
    let srcs = [
        "undeclared_here = 1; typeof undeclared_here",
        "var local_in_eval = 7; local_in_eval",
        "typeof this",
        "(function(){ return typeof this })()",
        "with({a:1}){ a }",
        "delete undeclared_here",
        "var 1",
        "1+1",
    ];
    for s in srcs {
        for op in 0i64..4 {
            for caller in 0i64..2 {
                set_ps(0, s);
                set_pi(1, op);
                set_pi(2, caller);
                fn act(a: &Api, J: JS) {
                    unsafe {
                        (a.js_newcfunction)(J, Some(cf_load_or_eval), cptr(N_PROBE), 0);
                        (a.js_setglobal)(J, cs("PROBE").as_ptr());
                        if pi(2) == 0 {
                            run_expr(a, J, "PROBE()", "caller-nonstrict");
                        } else {
                            run_expr(a, J, "'use strict'; PROBE()", "caller-strict");
                        }
                        /* also from the top level of the native frame */
                        (a.js_getglobal)(J, cs("PROBE").as_ptr());
                        (a.js_pushundefined)(J);
                        let rc = (a.js_pcall)(J, 0);
                        emit(&format!("native rc={} res={}", rc, repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                        (a.js_pushnumber)(J, 0.0);
                    }
                }
                for f in [0, JS_STRICT] {
                    diff_native("loadstring vs eval", act, f);
                    /* row 137 ground truth on the one source that separates the
                     * two compilation modes: an assignment to an undeclared
                     * variable is a ReferenceError only when the code was
                     * COMPILED strict. */
                    if s == srcs[0] {
                        let p = libs();
                        let c = p.c.run_native(act, f);
                        let strict_compile = match op {
                            /* js_loadstring uses J->default_strict */
                            0 => f & JS_STRICT != 0,
                            /* js_loadeval / js_eval use the CURRENT J->strict */
                            1 | 2 => f & JS_STRICT != 0 || caller == 1,
                            _ => false,
                        };
                        let lbl = format!("row 137 op={} caller={} flags={}", op, caller, f);
                        match op {
                            0 | 1 => {
                                if strict_compile {
                                    assert!(
                                        c.contains("call=1")
                                            && c.contains("assignment to undeclared variable"),
                                        "{}: {}",
                                        lbl,
                                        c
                                    );
                                } else {
                                    assert!(c.contains("call=0 res=\"number\""), "{}: {}", lbl, c);
                                }
                            }
                            2 => {
                                if strict_compile {
                                    assert!(
                                        c.contains("assignment to undeclared variable"),
                                        "{}: {}",
                                        lbl,
                                        c
                                    );
                                } else {
                                    assert!(c.contains("eval=\"number\""), "{}: {}", lbl, c);
                                }
                            }
                            /* js_eval with a non-string on top returns at once */
                            _ => assert!(
                                c.contains("eval_nonstring=5"),
                                "{}: {}",
                                lbl,
                                c
                            ),
                        }
                    }
                }
            }
        }
    }
}

/* ========================================================================== */
/* rows 157, 158, 159: the predicate family                                    */
/* ========================================================================== */

/// Rows 157, 158, 159 — every predicate against every value shape (including
/// all three string representations, a `JS_CSCRIPT` object from
/// `js_ploadstring`, a cfunction, a userdata, an error object, an iterator, the
/// four wrapper classes and a Date) and against out-of-range indices, which
/// resolve to the shared `undefined` sentinel.
#[test]
fn predicates_over_every_value_shape() {
    fn act(a: &Api, J: JS) {
        unsafe {
            (a.js_pushundefined)(J);
            (a.js_pushnull)(J);
            (a.js_pushboolean)(J, 0);
            (a.js_pushboolean)(J, 1);
            (a.js_pushnumber)(J, 0.0);
            (a.js_pushnumber)(J, f64::NAN);
            (a.js_pushstring)(J, cs("shrstr").as_ptr()); /* JS_TSHRSTR */
            (a.js_pushstring)(J, cs("0123456789ABCDEF").as_ptr()); /* JS_TMEMSTR */
            (a.js_pushliteral)(J, cptr(LIT_SHORT)); /* JS_TLITSTR */
            (a.js_newobject)(J);
            (a.js_newarray)(J);
            (a.js_newboolean)(J, 1);
            (a.js_newnumber)(J, 3.5);
            (a.js_newstring)(J, cs("boxed").as_ptr());
            (a.js_newregexp)(J, cs("a+").as_ptr(), JS_REGEXP_G);
            (a.js_newerror)(J, cs("err").as_ptr());
            (a.js_newcfunction)(J, Some(cf_report), cptr(N_FN), 1);
            (a.js_pushglobal)(J);
            /* a JS_CSCRIPT object */
            let nm = cs("s.js");
            let src = cs("1+1");
            if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) != 0 {
                emit("script-load-failed");
            }
            /* a userdata */
            (a.js_newobject)(J);
            (a.js_newuserdata)(J, cptr(TAG_A), 0xBEEF as *mut c_void, None);
            /* an iterator */
            (a.js_newobject)(J);
            (a.js_pushnumber)(J, 1.0);
            (a.js_setproperty)(J, -2, cs("p").as_ptr());
            (a.js_pushiterator)(J, -1, 1);
            /* a Date and a script function */
            let src2 = cs("new Date(0)");
            if (a.js_ploadstring)(J, nm.as_ptr(), src2.as_ptr()) == 0 {
                (a.js_pushundefined)(J);
                if (a.js_pcall)(J, 0) != 0 {
                    emit("date-failed");
                }
            }
            let src3 = cs("(function(x){return x})");
            if (a.js_ploadstring)(J, nm.as_ptr(), src3.as_ptr()) == 0 {
                (a.js_pushundefined)(J);
                if (a.js_pcall)(J, 0) != 0 {
                    emit("fun-failed");
                }
            }

            let n = (a.js_gettop)(J);
            emit(&format!("count={}", n));
            let idxs: Vec<c_int> = (0..n).chain([-1000, -100, n, n + 1, 4095].into_iter()).collect();
            for i in idxs {
                emit(&format!(
                    "[{}] def={} undef={} null={} bool={} num={} str={} prim={} obj={} coerc={}",
                    i,
                    (a.js_isdefined)(J, i),
                    (a.js_isundefined)(J, i),
                    (a.js_isnull)(J, i),
                    (a.js_isboolean)(J, i),
                    (a.js_isnumber)(J, i),
                    (a.js_isstring)(J, i),
                    (a.js_isprimitive)(J, i),
                    (a.js_isobject)(J, i),
                    (a.js_iscoercible)(J, i),
                ));
                emit(&format!(
                    "[{}] callable={} array={} regexp={} error={} ud_a={} ud_b={} numobj={} strobj={} boolobj={} dateobj={}",
                    i,
                    (a.js_iscallable)(J, i),
                    (a.js_isarray)(J, i),
                    (a.js_isregexp)(J, i),
                    (a.js_iserror)(J, i),
                    (a.js_isuserdata)(J, i, cptr(TAG_A)),
                    (a.js_isuserdata)(J, i, cptr(TAG_B)),
                    (a.js_isnumberobject)(J, i),
                    (a.js_isstringobject)(J, i),
                    (a.js_isbooleanobject)(J, i),
                    (a.js_isdateobject)(J, i),
                ));
                emit(&format!(
                    "[{}] typeof={} type={} tob={}",
                    i,
                    rs((a.js_typeof)(J, i)),
                    (a.js_type)(J, i),
                    (a.js_toboolean)(J, i)
                ));
            }
            (a.js_pop)(J, (a.js_gettop)(J) - 1);
            (a.js_pushnumber)(J, 0.0);
        }
    }
    for f in [0, JS_STRICT] {
        diff_native("predicates", act, f);
    }
    /* ground truth: the interesting classes really are on the stack, and an
     * out-of-range index resolves to the shared `undefined` sentinel. */
    let p = libs();
    let c = p.c.run_native(act, 0);
    /* index 0 is the trampoline's `this`, so the zoo starts at index 1 */
    assert!(c.contains("count=25"), "row 157 zoo: {}", c);
    for probe in [
        /* undefined / null */
        "[1] def=0 undef=1 null=0 bool=0 num=0 str=0 prim=1 obj=0 coerc=0",
        "[2] def=1 undef=0 null=1 bool=0 num=0 str=0 prim=1 obj=0 coerc=0",
        /* the three string representations behave identically */
        "[7] def=1 undef=0 null=0 bool=0 num=0 str=1 prim=1 obj=0 coerc=1",
        "[8] def=1 undef=0 null=0 bool=0 num=0 str=1 prim=1 obj=0 coerc=1",
        "[9] def=1 undef=0 null=0 bool=0 num=0 str=1 prim=1 obj=0 coerc=1",
        /* out-of-range indices resolve to the undefined sentinel */
        "[-1000] def=0 undef=1 null=0 bool=0 num=0 str=0 prim=1 obj=0 coerc=0",
        "[4095] def=0 undef=1 null=0 bool=0 num=0 str=0 prim=1 obj=0 coerc=0",
        "[26] def=0 undef=1 null=0 bool=0 num=0 str=0 prim=1 obj=0 coerc=0",
    ] {
        assert!(c.contains(probe), "row 157 {:?}: {}", probe, c);
    }
    /* row 158: js_iscallable is true for CFUNCTION/CSCRIPT/CCFUNCTION only —
     * note the asymmetry: a JS_CSCRIPT object is callable but `typeof` object */
    assert!(c.contains("[11] callable=0 array=1"), "row 158 array: {}", c);
    assert!(c.contains("[17] callable=1"), "row 158 cfunction: {}", c);
    assert!(c.contains("[19] callable=1"), "row 158 script object: {}", c);
    assert!(c.contains("[19] typeof=object"), "row 158 script typeof: {}", c);
    assert!(c.contains("[20] callable=0 array=0 regexp=0 error=0 ud_a=1 ud_b=0"), "row 158 userdata: {}", c);
    assert!(c.contains("[16] callable=0 array=0 regexp=0 error=1"), "row 158 error: {}", c);
    assert!(c.contains("[15] callable=0 array=0 regexp=1"), "row 158 regexp: {}", c);
    /* row 159: the wrapper-object predicates */
    assert!(c.contains("numobj=1"), "row 159 number object: {}", c);
    assert!(c.contains("strobj=1"), "row 159 string object: {}", c);
    assert!(c.contains("boolobj=1"), "row 159 boolean object: {}", c);
    assert!(c.contains("dateobj=1"), "row 159 date object: {}", c);
}

/* ========================================================================== */
/* rows 162-165: js_trystring / js_trynumber / js_tryinteger / js_tryboolean    */
/* ========================================================================== */

/// Rows 162, 163, 164, 165 — the `js_try*` conversions on values that convert
/// cleanly, on objects whose `toString`/`valueOf` throw (the default is
/// returned and exactly one value is popped), with a `NULL` error string, and on
/// a value that needs `jsV_numbertointeger` clamping.  The `js_ptry`
/// (try-stack-exhausted) branch of the same functions is covered by
/// `phase_c_runtime::r269_r278_ptry_with_full_try_stack`.
#[test]
fn try_conversion_defaults() {
    let vals = [
        "1.5",
        "'abc'",
        "''",
        "undefined",
        "null",
        "true",
        "1e10",
        "({valueOf:function(){return 1e10}})",
        "({toString:function(){return 'ts'}})",
        "({toString:function(){throw new Error('ts!')}})",
        "({valueOf:function(){throw new Error('vo!')}, toString:function(){throw new Error('ts!')}})",
        "Object.create(null)",
        "({})",
        "[1,2]",
        "(function(){})",
    ];
    let mut rng = Rng::new(SEED ^ 0xAB);
    for v in vals {
        for _ in 0..3 {
            let di = rng.range_i64(-9, 9);
            let df = rng.f64();
            set_ps(0, v);
            set_pi(0, di);
            set_pf(0, df);
            let null_err = rng.range_i64(0, 1) == 1;
            set_pi(1, null_err as i64); /* NULL error string or not */
            fn act(a: &Api, J: JS) {
                unsafe {
                    let src = cs(&format!("({})", ps(0).to_str().unwrap()));
                    let nm = cs("v.js");
                    if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) != 0 {
                        emit("loadfail");
                        (a.js_pop)(J, 1);
                        (a.js_pushnumber)(J, 0.0);
                        return;
                    }
                    (a.js_pushundefined)(J);
                    if (a.js_pcall)(J, 0) != 0 {
                        emit("callfail");
                        (a.js_pop)(J, 1);
                        (a.js_pushnumber)(J, 0.0);
                        return;
                    }
                    let e = cs("<DEFAULT>");
                    let ep = if pi(1) == 0 { e.as_ptr() } else { std::ptr::null() };
                    let top0 = (a.js_gettop)(J);
                    /* js_trystring/js_tryrepr convert the slot IN PLACE (through
                     * js_tostring / js_torepr), so every conversion runs on a
                     * fresh copy of the pristine value. */
                    (a.js_copy)(J, -1);
                    emit(&format!("trys={:?}", rs((a.js_trystring)(J, -1, ep))));
                    (a.js_pop)(J, 1);
                    (a.js_copy)(J, -1);
                    emit(&format!("tryn={:#x}", (a.js_trynumber)(J, -1, pf(0)).to_bits()));
                    (a.js_pop)(J, 1);
                    (a.js_copy)(J, -1);
                    emit(&format!("tryi={}", (a.js_tryinteger)(J, -1, pic(0))));
                    (a.js_pop)(J, 1);
                    (a.js_copy)(J, -1);
                    emit(&format!("tryb={}", (a.js_tryboolean)(J, -1, pic(0))));
                    (a.js_pop)(J, 1);
                    (a.js_copy)(J, -1);
                    emit(&format!("tryr={:?}", rs((a.js_tryrepr)(J, -1, ep))));
                    (a.js_pop)(J, 1);
                    emit(&format!(
                        "top={} unchanged={}",
                        (a.js_gettop)(J),
                        (a.js_gettop)(J) == top0
                    ));
                    /* out-of-range index resolves to the undefined sentinel */
                    emit(&format!(
                        "oob trys={:?} tryn={:#x} tryi={} tryb={}",
                        rs((a.js_trystring)(J, 999, ep)),
                        (a.js_trynumber)(J, 999, pf(0)).to_bits(),
                        (a.js_tryinteger)(J, 999, pic(0)),
                        (a.js_tryboolean)(J, 999, pic(0)),
                    ));
                    (a.js_pop)(J, (a.js_gettop)(J) - 1);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            for f in [0, JS_STRICT] {
                diff_native("try conversions", act, f);
            }
            let p = libs();
            let c = p.c.run_native(act, 0);
            /* none of the js_try* functions may disturb the stack */
            assert!(c.contains("unchanged=true"), "rows 162-165 stack: {}", c);
            let want_default_str = if null_err {
                "trys=\"<null>\""
            } else {
                "trys=\"<DEFAULT>\""
            };
            if v.contains("throw") {
                /* rows 162/163/164: the throwing conversion returns `error` */
                assert!(c.contains(want_default_str), "row 162 default: {}", c);
                assert!(
                    c.contains(&format!("tryn={:#x}", df.to_bits())),
                    "row 163 default: {}",
                    c
                );
                assert!(c.contains(&format!("tryi={}", di)), "row 164 default: {}", c);
                /* row 165: js_toboolean itself never throws, so an object is
                 * always true and the default is unreachable for it */
                assert!(c.contains("tryb=1"), "row 165: {}", c);
            }
            if v == "1e10" || v.contains("return 1e10") {
                /* row 164: jsV_numbertointeger clamping on the success path */
                assert!(c.contains("tryi=2147483647"), "row 164 clamp: {}", c);
            }
            /* out-of-range index -> the undefined sentinel, never the default */
            assert!(c.contains("oob trys=\"undefined\""), "row 162 oob: {}", c);
        }
    }
}

/* ========================================================================== */
/* row 166: js_touserdata + js_toobject autoboxing                             */
/* ========================================================================== */

/// Row 166 — `js_touserdata` with the matching tag, with a wrong tag
/// (TypeError `"not a %s"`) and on a non-userdata object, plus `js_toobject`
/// on `undefined`/`null` (TypeError) and on string/number/boolean primitives,
/// where the autoboxed wrapper is written back INTO the stack slot.
#[test]
fn touserdata_and_toobject_autoboxing() {
    for mode in 0i64..12 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                match pi(0) {
                    0 | 1 | 2 => {
                        (a.js_newobject)(J);
                        (a.js_newuserdata)(J, cptr(TAG_A), 0xBEEF as *mut c_void, None);
                        let tag = if pi(0) == 0 { cptr(TAG_A) } else { cptr(TAG_B) };
                        if pi(0) == 2 {
                            emit(&format!("isud={}", (a.js_isuserdata)(J, -1, cptr(TAG_B))));
                            emit(&format!("ud={:?}", (a.js_touserdata)(J, -1, cptr(TAG_A))));
                        } else {
                            emit(&format!("ud={:?}", (a.js_touserdata)(J, -1, tag)));
                        }
                    }
                    3 => {
                        (a.js_newobject)(J);
                        emit(&format!("ud_on_plain={:?}", (a.js_touserdata)(J, -1, cptr(TAG_A))));
                    }
                    4 => {
                        (a.js_pushnumber)(J, 1.0);
                        emit(&format!("ud_on_number={:?}", (a.js_touserdata)(J, -1, cptr(TAG_A))));
                    }
                    /* js_toobject autoboxing writes the wrapper back into the slot */
                    5 | 6 | 7 | 8 => {
                        match pi(0) {
                            5 => (a.js_pushstring)(J, cs("abc").as_ptr()),
                            6 => (a.js_pushnumber)(J, 2.5),
                            7 => (a.js_pushboolean)(J, 1),
                            _ => (a.js_pushliteral)(J, cptr(LIT_SHORT)),
                        }
                        emit(&format!(
                            "before type={} typeof={} isobject={} repr={}",
                            (a.js_type)(J, -1),
                            rs((a.js_typeof)(J, -1)),
                            (a.js_isobject)(J, -1),
                            repr_at(a, J, -1)
                        ));
                        let o = (a.js_toobject)(J, -1);
                        emit(&format!("obj_nonnull={}", !o.is_null()));
                        emit(&format!(
                            "after type={} typeof={} isobject={} repr={} strobj={} numobj={} boolobj={}",
                            (a.js_type)(J, -1),
                            rs((a.js_typeof)(J, -1)),
                            (a.js_isobject)(J, -1),
                            repr_at(a, J, -1),
                            (a.js_isstringobject)(J, -1),
                            (a.js_isnumberobject)(J, -1),
                            (a.js_isbooleanobject)(J, -1),
                        ));
                        probe_get(a, J, -1, "constructor");
                        /* the second call must return the very same object */
                        let o2 = (a.js_toobject)(J, -1);
                        emit(&format!("same_object={}", o == o2));
                    }
                    9 => {
                        (a.js_pushundefined)(J);
                        emit(&format!("toobject={:?}", (a.js_toobject)(J, -1)));
                    }
                    10 => {
                        (a.js_pushnull)(J);
                        emit(&format!("toobject={:?}", (a.js_toobject)(J, -1)));
                    }
                    _ => {
                        (a.js_newobject)(J);
                        let o = (a.js_toobject)(J, -1);
                        emit(&format!("plain_object_identity={}", o == (a.js_toobject)(J, -1)));
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("touserdata/toobject mode={}", mode), act, f);
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        match mode {
            0 => assert!(c.contains("ud=0xbeef"), "row 166 matching tag: {}", c),
            1 => assert!(c.contains("not a tagB"), "row 166 wrong tag: {}", c),
            2 => assert!(c.contains("isud=0") && c.contains("ud=0xbeef"), "row 166: {}", c),
            3 | 4 => assert!(c.contains("not a tagA"), "row 166 non-userdata: {}", c),
            5 => assert!(
                c.contains("before type=4") && c.contains("after type=6") && c.contains("strobj=1")
                    && c.contains("same_object=true"),
                "row 166 string autobox: {}",
                c
            ),
            6 => assert!(c.contains("after type=6") && c.contains("numobj=1"), "row 166 number autobox: {}", c),
            7 => assert!(c.contains("after type=6") && c.contains("boolobj=1"), "row 166 boolean autobox: {}", c),
            8 => assert!(c.contains("after type=6") && c.contains("strobj=1"), "row 166 literal autobox: {}", c),
            9 => assert!(c.contains("cannot convert undefined to object"), "row 166: {}", c),
            10 => assert!(c.contains("cannot convert null to object"), "row 166: {}", c),
            _ => assert!(c.contains("plain_object_identity=true"), "row 166: {}", c),
        }
    }
}

/* ========================================================================== */
/* row 169: the `throw` parameter of jsR_defproperty + js_delglobal            */
/* ========================================================================== */

/// Row 169 — `js_defproperty` passes `throw = 1` to `jsR_defproperty`, so a
/// readonly/non-configurable slot raises a TypeError even in a NON-strict
/// state, whereas `js_defglobal` passes `throw = 0` and is silently ignored;
/// plus `js_delglobal` on the `JS_DONTCONF` globals (`undefined`, `NaN`,
/// `Infinity`) versus a configurable one.
#[test]
fn defproperty_throw_flag_and_delglobal() {
    for mode in 0i64..12 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                match pi(0) {
                    /* throw=1: js_defproperty on array length / string index */
                    0 => {
                        (a.js_newarray)(J);
                        (a.js_pushnumber)(J, 3.0);
                        (a.js_defproperty)(J, -2, cs("length").as_ptr(), 0);
                    }
                    1 => {
                        (a.js_newstring)(J, cs("ab").as_ptr());
                        (a.js_pushstring)(J, cs("z").as_ptr());
                        (a.js_defproperty)(J, -2, cs("0").as_ptr(), 0);
                    }
                    /* throw=0: js_defglobal over a readonly/dontconf global */
                    2 => {
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defglobal)(J, cs("undefined").as_ptr(), 0);
                        emit("defglobal-undefined-returned");
                        run_expr(a, J, "String(undefined)", "undef");
                    }
                    3 => {
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defglobal)(J, cs("NaN").as_ptr(), 0);
                        emit("defglobal-NaN-returned");
                        run_expr(a, J, "String(NaN)", "nan");
                    }
                    4 => {
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defglobal)(J, cs("Infinity").as_ptr(), JS_READONLY);
                        emit("defglobal-Infinity-returned");
                        run_expr(a, J, "String(Infinity)", "inf");
                    }
                    5 => {
                        /* a readonly global defined by us, redefined again */
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defglobal)(J, cs("ro").as_ptr(), JS_READONLY | JS_DONTCONF);
                        (a.js_pushnumber)(J, 2.0);
                        (a.js_defglobal)(J, cs("ro").as_ptr(), 0);
                        emit("redef-returned");
                        (a.js_getglobal)(J, cs("ro").as_ptr());
                        emit(&format!("ro={}", repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    /* js_delglobal */
                    6 => {
                        (a.js_delglobal)(J, cs("undefined").as_ptr());
                        emit("delglobal-undefined-returned");
                        (a.js_pushglobal)(J);
                        probe_has(a, J, -1, "undefined");
                    }
                    7 => {
                        (a.js_delglobal)(J, cs("NaN").as_ptr());
                        emit("delglobal-NaN-returned");
                        (a.js_pushglobal)(J);
                        probe_has(a, J, -1, "NaN");
                    }
                    8 => {
                        (a.js_delglobal)(J, cs("Math").as_ptr());
                        emit("delglobal-Math-returned");
                        (a.js_pushglobal)(J);
                        probe_has(a, J, -1, "Math");
                    }
                    9 => {
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_defglobal)(J, cs("dc").as_ptr(), JS_DONTCONF);
                        (a.js_delglobal)(J, cs("dc").as_ptr());
                        emit("delglobal-dc-returned");
                        (a.js_getglobal)(J, cs("dc").as_ptr());
                        emit(&format!("dc={}", repr_at(a, J, -1)));
                        (a.js_pop)(J, 1);
                    }
                    10 => {
                        (a.js_delglobal)(J, cs("nosuchglobal").as_ptr());
                        emit("delglobal-missing-returned");
                    }
                    _ => {
                        /* js_defaccessor also passes throw=1 */
                        (a.js_newarray)(J);
                        (a.js_newcfunction)(J, Some(cf_const42), cptr(N_GET), 0);
                        (a.js_pushundefined)(J);
                        (a.js_defaccessor)(J, -3, cs("length").as_ptr(), 0);
                    }
                }
                emit(&format!("survived top={}", (a.js_gettop)(J)));
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("defglobal/delglobal mode={}", mode), act, f);
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        match mode {
            /* throw=1: raises even in a NON-strict state */
            0 | 1 | 11 => assert!(
                c.contains("read-only or non-configurable") && !c.contains("survived"),
                "row 169 defproperty throw=1 (mode={}): {}",
                mode,
                c
            ),
            /* throw=0: js_defglobal is silently ignored */
            2 => assert!(
                c.contains("defglobal-undefined-returned") && c.contains("undef rc=0 \"undefined\""),
                "row 169 defglobal throw=0: {}",
                c
            ),
            3 => assert!(
                c.contains("defglobal-NaN-returned") && c.contains("nan rc=0 \"NaN\""),
                "row 169 defglobal throw=0: {}",
                c
            ),
            4 => assert!(
                c.contains("defglobal-Infinity-returned") && c.contains("inf rc=0 \"Infinity\""),
                "row 169 defglobal throw=0: {}",
                c
            ),
            5 => assert!(c.contains("redef-returned") && c.contains("ro=1"), "row 169 redef: {}", c),
            /* js_delglobal on the JS_DONTCONF globals refuses silently */
            6 => assert!(c.contains("has \"undefined\"=1"), "row 169 delglobal: {}", c),
            7 => assert!(c.contains("has \"NaN\"=1"), "row 169 delglobal: {}", c),
            8 => assert!(c.contains("has \"Math\"=0"), "row 169 delglobal configurable: {}", c),
            9 => assert!(c.contains("dc=1"), "row 169 delglobal DONTCONF: {}", c),
            _ => assert!(c.contains("survived"), "row 169 mode={}: {}", mode, c),
        }
        /* under JS_STRICT the same js_delglobal raises */
        if mode == 6 || mode == 7 || mode == 9 {
            let cs_ = p.c.run_native(act, JS_STRICT);
            assert!(
                cs_.contains("is non-configurable"),
                "row 169 strict delglobal (mode={}): {}",
                mode,
                cs_
            );
        }
    }
}

/* ========================================================================== */
/* row 170: js_newobjectx                                                      */
/* ========================================================================== */

/// Row 170 — `js_newobjectx` uses the top of the stack as the prototype when it
/// is an object (and always pops it); a non-object leaves `prototype == NULL`,
/// so the result has no `Object.prototype` at all (no `toString`, which makes
/// `jsV_toprimitive` fail).  Contrasted with plain `js_newobject`.
#[test]
fn newobjectx_prototype_variants() {
    for kind in 0i64..9 {
        set_pi(0, kind);
        fn act(a: &Api, J: JS) {
            unsafe {
                emit(&format!("top0={}", (a.js_gettop)(J)));
                match pi(0) {
                    0 => {
                        (a.js_newobject)(J);
                        (a.js_pushnumber)(J, 1.0);
                        (a.js_setproperty)(J, -2, cs("inherited").as_ptr());
                        (a.js_newobjectx)(J);
                    }
                    1 => {
                        (a.js_pushnull)(J);
                        (a.js_newobjectx)(J);
                    }
                    2 => {
                        (a.js_pushnumber)(J, 5.0);
                        (a.js_newobjectx)(J);
                    }
                    3 => {
                        (a.js_pushundefined)(J);
                        (a.js_newobjectx)(J);
                    }
                    4 => {
                        (a.js_pushstring)(J, cs("s").as_ptr());
                        (a.js_newobjectx)(J);
                    }
                    5 => {
                        (a.js_newarray)(J);
                        (a.js_newobjectx)(J);
                    }
                    6 => {
                        (a.js_newcfunction)(J, Some(cf_const42), cptr(N_FN), 0);
                        (a.js_newobjectx)(J);
                    }
                    7 => {
                        (a.js_pushglobal)(J);
                        (a.js_newobjectx)(J);
                    }
                    _ => {
                        (a.js_newobject)(J);
                    }
                }
                emit(&format!("top1={}", (a.js_gettop)(J)));
                emit(&format!(
                    "typeof={} type={} isobject={}",
                    rs((a.js_typeof)(J, -1)),
                    (a.js_type)(J, -1),
                    (a.js_isobject)(J, -1)
                ));
                emit(&format!("repr={}", repr_at(a, J, -1)));
                emit(&format!("str={:?}", str_at(a, J, -1)));
                probe_has(a, J, -1, "toString");
                probe_has(a, J, -1, "inherited");
                probe_has(a, J, -1, "hasOwnProperty");
                (a.js_pushnumber)(J, 2.0);
                (a.js_setproperty)(J, -2, cs("own").as_ptr());
                probe_get(a, J, -1, "own");
                (a.js_copy)(J, -1);
                (a.js_setglobal)(J, cs("X").as_ptr());
                run_expr(a, J, "Object.getPrototypeOf(X)===null", "protonull");
                run_expr(a, J, "String(Object.keys(X))", "keys");
                run_expr(a, J, "try{ String(X) }catch(e){ 'throws:'+e.name }", "tostring");
                run_expr(a, J, "try{ X+'' }catch(e){ 'throws:'+e.name }", "concat");
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("newobjectx kind={}", kind), act, f);
        }
        /* row 170: an object prototype is used (and popped); anything else
         * leaves prototype == NULL, so nothing at all is inherited. */
        let p = libs();
        let c = p.c.run_native(act, 0);
        assert!(c.contains("top0=1") && c.contains("top1=2"), "row 170 pop (kind={}): {}", kind, c);
        assert!(c.contains("get \"own\"=2"), "row 170 own prop (kind={}): {}", kind, c);
        match kind {
            /* the prototype is itself a js_newobject, so Object.prototype is
             * still two links up the chain */
            0 => assert!(
                c.contains("has \"inherited\"=1 v=1")
                    && c.contains("has \"toString\"=1")
                    && c.contains("protonull rc=0 false"),
                "row 170 object prototype: {}",
                c
            ),
            1 | 2 | 3 | 4 => assert!(
                c.contains("has \"toString\"=0")
                    && c.contains("has \"hasOwnProperty\"=0")
                    && c.contains("protonull rc=0 true"),
                "row 170 non-object prototype (kind={}): {}",
                kind,
                c
            ),
            8 => assert!(
                c.contains("has \"toString\"=1") && c.contains("protonull rc=0 false"),
                "row 170 js_newobject: {}",
                c
            ),
            /* the global object is a valid prototype, but its OWN prototype is
             * NULL, so `toString` is still not reachable */
            7 => assert!(
                c.contains("has \"toString\"=0") && c.contains("protonull rc=0 false"),
                "row 170 global prototype: {}",
                c
            ),
            /* array / cfunction prototypes inherit from Array.prototype /
             * Function.prototype and therefore do see toString */
            _ => assert!(
                c.contains("has \"toString\"=1") && c.contains("protonull rc=0 false"),
                "row 170 kind={}: {}",
                kind,
                c
            ),
        }
    }
}

/* ========================================================================== */
/* row 195: js_freestate                                                       */
/* ========================================================================== */

/// Row 195 — (a) `js_freestate(NULL)` is a no-op; (b) tearing down a state that
/// still holds live objects, a regexp (`js_regfreex`), a long (heap) string
/// object, a flat array, an iterator, a userdata finalizer and a cfunction
/// finalizer runs every `jsG_freeobject` sub-branch exactly once — including
/// row 63, where the userdata is only reachable from the registry.
///
/// The `J->trytop > 0` variant of (c) is not constructible from outside the
/// library (a state can only be freed from a frame that is not running it), so
/// it is left to `phase_c_isolated`'s child processes.
#[test]
fn freestate_teardown_and_finalizers() {
    for mode in 0i64..4 {
        set_pi(0, mode);
        fn act(a: &Api, J: JS) {
            unsafe {
                ud_fin_set(0);
                cf_fin_set(0);
                /* (a) */
                (a.js_freestate)(std::ptr::null_mut());
                emit("freestate-null-ok");

                let J2 = (a.js_newstate)(None, std::ptr::null_mut(), pic(0) & 1);
                emit(&format!("J2={}", !J2.is_null()));
                if J2.is_null() {
                    (a.js_pushnumber)(J, 0.0);
                    return;
                }
                /* live objects of every interesting class */
                (a.js_newregexp)(J2, cs("a(b)c+").as_ptr(), JS_REGEXP_G | JS_REGEXP_I);
                let _ = (a.js_ref)(J2);
                (a.js_newstring)(J2, cs("a string well over fifteen bytes long").as_ptr());
                let _ = (a.js_ref)(J2);
                (a.js_newarray)(J2);
                for i in 0..12 {
                    (a.js_pushnumber)(J2, i as f64);
                    (a.js_setindex)(J2, -2, i);
                }
                let _ = (a.js_ref)(J2);
                (a.js_newobject)(J2);
                (a.js_pushnumber)(J2, 1.0);
                (a.js_setproperty)(J2, -2, cs("p").as_ptr());
                (a.js_pushiterator)(J2, -1, 1);
                let _ = (a.js_ref)(J2);
                (a.js_pop)(J2, 1);
                /* row 63: userdata reachable only from the registry */
                (a.js_newobject)(J2);
                (a.js_newuserdatax)(
                    J2,
                    cptr(TAG_A),
                    0xBEEF as *mut c_void,
                    Some(u_has),
                    Some(u_put),
                    Some(u_del),
                    Some(u_fin),
                );
                let _ = (a.js_ref)(J2);
                (a.js_newcfunctionx)(
                    J2,
                    Some(cf_report),
                    cptr(N_FN),
                    1,
                    0xD00D as *mut c_void,
                    Some(cf_fin),
                );
                let _ = (a.js_ref)(J2);
                if pi(0) >= 2 {
                    /* also run a script so environments/functions/traces exist */
                    run_expr(
                        a,
                        J2,
                        "var a=[];for(var i=0;i<50;i++)a.push({x:i,s:'s'+i});\
                         function f(){return a.length} f()",
                        "sub",
                    );
                    run_expr(a, J2, "try{ null.x }catch(e){ String(e) }", "subthrow");
                }
                if pi(0) == 3 {
                    (a.js_gc)(J2, 0);
                    emit(&format!(
                        "before_free ud={} cf={}",
                        ud_fin_get(),
                        cf_fin_get()
                    ));
                }
                emit(&format!(
                    "pre ud={} cf={}",
                    ud_fin_get(),
                    cf_fin_get()
                ));
                (a.js_freestate)(J2);
                emit(&format!(
                    "post ud={} cf={}",
                    ud_fin_get(),
                    cf_fin_get()
                ));
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            let p = libs();
            same(
                &format!("freestate mode={} flags={}", mode, f),
                &mask_ptrs(&p.c.run_native(act, f)),
                &mask_ptrs(&p.r.run_native(act, f)),
            );
        }
        let p = libs();
        let c = mask_ptrs(&p.c.run_native(act, 0));
        assert!(c.contains("post ud=1 cf=1"), "teardown finalizers: {}", c);
    }
}

/* ========================================================================== */
/* row 151: js_gettop / js_pushglobal / js_currentfunction(data)               */
/* ========================================================================== */

unsafe extern "C" fn cf_frame_probe(J: JS) {
    let a = cur();
    unsafe {
        emit(&format!("frame top={}", (a.js_gettop)(J)));
        emit(&format!("frame data={:?}", (a.js_currentfunctiondata)(J)));
        (a.js_currentfunction)(J);
        emit(&format!("frame cur={}", repr_at(a, J, -1)));
        emit(&format!("frame cur_callable={}", (a.js_iscallable)(J, -1)));
        (a.js_pop)(J, 1);
        (a.js_pushglobal)(J);
        (a.js_pushglobal)(J);
        emit(&format!("global_identical={}", (a.js_strictequal)(J)));
        (a.js_pop)(J, 2);
        (a.js_pushglobal)(J);
        (a.js_getglobal)(J, cs("Math").as_ptr());
        emit(&format!("global_vs_math={}", (a.js_strictequal)(J)));
        (a.js_pop)(J, 2);
        (a.js_pushglobal)(J);
        (a.js_getproperty)(J, -1, cs("Math").as_ptr());
        (a.js_getglobal)(J, cs("Math").as_ptr());
        emit(&format!("math_identical={}", (a.js_strictequal)(J)));
        (a.js_pop)(J, 3);
        (a.js_pushnumber)(J, 1.0);
    }
}

/// Row 151 — `js_currentfunction` / `js_currentfunctiondata` at the top level
/// (`BOT == 0`: undefined / NULL) and inside a `js_newcfunctionx` frame (the
/// function object and its `data`), and `js_pushglobal`'s identity against
/// `js_getglobal`.
#[test]
fn currentfunction_and_pushglobal_identity() {
    for kind in 0i64..3 {
        set_pi(0, kind);
        fn act(a: &Api, J: JS) {
            unsafe {
                /* BOT == 0 on a state with no active call frame */
                let J2 = (a.js_newstate)(None, std::ptr::null_mut(), 0);
                if !J2.is_null() {
                    emit(&format!("J2 top={}", (a.js_gettop)(J2)));
                    emit(&format!("J2 data={:?}", (a.js_currentfunctiondata)(J2)));
                    (a.js_currentfunction)(J2);
                    emit(&format!("J2 cur={}", repr_at(a, J2, -1)));
                    emit(&format!("J2 undef={}", (a.js_isundefined)(J2, -1)));
                    (a.js_pop)(J2, 1);
                    (a.js_pushglobal)(J2);
                    emit(&format!("J2 global={}", (a.js_isobject)(J2, -1)));
                    (a.js_pop)(J2, 1);
                    (a.js_freestate)(J2);
                }
                /* inside a frame */
                match pi(0) {
                    0 => (a.js_newcfunction)(J, Some(cf_frame_probe), cptr(N_PROBE), 0),
                    1 => (a.js_newcfunctionx)(
                        J,
                        Some(cf_frame_probe),
                        cptr(N_PROBE),
                        2,
                        0xD00D as *mut c_void,
                        None,
                    ),
                    _ => (a.js_newcfunctionx)(
                        J,
                        Some(cf_frame_probe),
                        cptr(N_PROBE),
                        0,
                        std::ptr::null_mut(),
                        None,
                    ),
                }
                (a.js_copy)(J, -1);
                (a.js_setglobal)(J, cs("PROBE").as_ptr());
                (a.js_pushundefined)(J);
                let rc = (a.js_pcall)(J, 0);
                emit(&format!("pcall={} res={}", rc, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
                run_expr(a, J, "String(PROBE())", "from-js");
                (a.js_pop)(J, (a.js_gettop)(J) - 1);
                (a.js_pushnumber)(J, 0.0);
            }
        }
        for f in [0, JS_STRICT] {
            diff_native(&format!("currentfunction kind={}", kind), act, f);
        }
        let p = libs();
        let c = p.c.run_native(act, 0);
        /* row 151, BOT == 0 */
        assert!(
            c.contains("J2 top=0") && c.contains("J2 data=0x0") && c.contains("J2 cur=undefined")
                && c.contains("J2 undef=1"),
            "row 151 BOT=0 (kind={}): {}",
            kind,
            c
        );
        /* row 151, inside a frame */
        assert!(c.contains("frame cur_callable=1"), "row 151 frame: {}", c);
        if kind == 1 {
            assert!(c.contains("frame data=0xd00d"), "row 151 data: {}", c);
            assert!(c.contains("frame top=3"), "row 151 padding: {}", c);
        } else {
            assert!(c.contains("frame data=0x0"), "row 151 data: {}", c);
            assert!(c.contains("frame top=1"), "row 151 padding: {}", c);
        }
        /* js_pushglobal identity */
        assert!(
            c.contains("global_identical=1")
                && c.contains("global_vs_math=0")
                && c.contains("math_identical=1"),
            "row 151 pushglobal: {}",
            c
        );
    }
}


/* ========================================================================== */
/* rows 69, 73: a finalizer that must run when the constructor runs out of      */
/* memory inside its own js_try                                                */
/* ========================================================================== */

/// Rows 69 + 73 — `js_newuserdatax` (jsvalue.c:548-552) and `js_newcfunctionx`
/// (jsvalue.c:486) install a `js_try` whose handler calls `finalize(J, data)`
/// and rethrows.  Under a tiny `memlimit` the `jsV_newobject` inside that try
/// throws "out of memory", so the finalizer must be observed exactly once and
/// the exception must still propagate.  With a NULL finalizer nothing is called;
/// with a generous memlimit the object is built normally.
///
/// NOTE: each constructor is exercised ALONE here — `js_newcfunctionx` before a
/// `js_newuserdatax` in the same action would throw first and the userdata call
/// would never be reached.
#[test]
fn newuserdatax_out_of_memory_finalizer() {
    for mode in 0i64..4 {
        for ml in [1i64, 8, 64, 4096] {
            set_pi(0, mode);
            set_pi(1, ml);
            fn act(a: &Api, J: JS) {
                unsafe {
                    ud_fin_set(0);
                    cf_fin_set(0);
                    match pi(0) {
                        0 | 1 => {
                            /* a null prototype: js_pushnull never allocates */
                            (a.js_pushnull)(J);
                            (a.js_setlimit)(J, 0, pic(1));
                            let fin: Option<Finalize> = if pi(0) == 0 { Some(u_fin) } else { None };
                            (a.js_newuserdatax)(
                                J,
                                cptr(TAG_A),
                                0xBEEF as *mut c_void,
                                None,
                                None,
                                None,
                                fin,
                            );
                            emit(&format!("ud-ok repr={}", repr_at(a, J, -1)));
                        }
                        _ => {
                            (a.js_setlimit)(J, 0, pic(1));
                            let fin: Option<Finalize> = if pi(0) == 2 { Some(cf_fin) } else { None };
                            (a.js_newcfunctionx)(
                                J,
                                Some(cf_report),
                                cptr(N_FN),
                                1,
                                0xD00D as *mut c_void,
                                fin,
                            );
                            emit("cf-ok");
                        }
                    }
                    (a.js_setlimit)(J, 0, 0);
                    emit(&format!("ud={} cf={}", ud_fin_get(), cf_fin_get()));
                    (a.js_pop)(J, (a.js_gettop)(J) - 1);
                    (a.js_pushnumber)(J, 0.0);
                }
            }
            for f in [0, JS_STRICT] {
                diff_native(&format!("oom finalizer mode={} ml={}", mode, ml), act, f);
            }
            let p = libs();
            let c = p.c.run_native(act, 0);
            let lbl = format!("rows 69/73 mode={} ml={}", mode, ml);
            if ml <= 64 {
                assert!(c.contains("out of memory"), "{}: {}", lbl, c);
                match mode {
                    0 => assert!(
                        c.contains("u_fin(data=0xbeef)") && !c.contains("ud-ok"),
                        "{}: {}",
                        lbl,
                        c
                    ),
                    2 => assert!(
                        c.contains("cf_fin(data=0xd00d)") && !c.contains("cf-ok"),
                        "{}: {}",
                        lbl,
                        c
                    ),
                    _ => assert!(!c.contains("_fin("), "{}: {}", lbl, c),
                }
            } else {
                assert!(
                    c.contains("ud=0 cf=0") && !c.contains("out of memory"),
                    "{}: {}",
                    lbl,
                    c
                );
            }
        }
    }
}
