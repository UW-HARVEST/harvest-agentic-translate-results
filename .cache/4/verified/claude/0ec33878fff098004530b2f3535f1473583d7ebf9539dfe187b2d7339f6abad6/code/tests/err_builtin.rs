//! Differential tests for the ERROR SURFACE of the builtin object library.
//! Covers ERRORS.md rows 477-730:
//!   jsarray.c 477-539, jsobject.c 540-589, jsstring.c 590-679,
//!   jsnumber.c 680-711, jsboolean.c 712-715, jsbuiltin.c 716-730.
//!
//! Every call goes through the two `.so` exports via `tests/common/mod.rs`.
//! Nothing that can throw is ever called with `trytop == 0`: the JS-level
//! probes run inside `js_ploadstring` + `js_pcall` (and each individual probe
//! additionally inside a JS `try`/`catch`), and the FFI-level probes run inside
//! a cfunction invoked with `js_pcall` (the `probe()` pattern of
//! `tests/err_core.rs`).
//!
//! Where a row raises an error, the test asserts the EXACT error constructor
//! name and the EXACT message text (`expect_*` helpers), not merely that both
//! libraries failed.  Where a row is a silent behaviour fork, the test asserts
//! that the two libraries produce byte-identical transcripts.
//!
//! Set `MUJS_DUMP=1` to print every transcript.
//!
//! ==========================================================================
//! ROWS DELIBERATELY NOT DRIVEN, and why (each verified against the C source)
//! ==========================================================================
//!
//! * row 484 -- jsarray.c:76.  `Ap_join_cycle` bails out when the trace frame's
//!   callee slot `&J->stack[stk-1]` is not `JS_TOBJECT`.  UNREACHABLE: every
//!   trace frame with index > 0 was pushed by `jsR_pushtrace` from `js_call`
//!   (jsrun.c:1315/1322/1326) or `js_construct` (jsrun.c:1350), both of which
//!   set `BOT = TOP - n - 1` only *after* `js_iscallable(J, -n-2)` /
//!   `js_iscallable(J, -n-1)` accepted the value in that very slot, so
//!   `stack[BOT-1]` is always a `JS_TOBJECT`.  `jsR_callcfunction` only
//!   overwrites it (`TOP = --BOT`, jsrun.c:1270/1273) after `F` has returned,
//!   i.e. never while a nested `Ap_join_cycle` can observe the frame.
//!   `trace[0]` is never examined (`while (top > 0)`).
//!
//! * row 486 -- jsarray.c:81.  A matched `Ap_join` frame whose `this` slot
//!   `&J->stack[stk]` is not `JS_TOBJECT`.  UNREACHABLE: `fun == Ap_join` means
//!   that frame's own `Ap_join_cycle` already ran `js_toobject(J, 0)`
//!   (jsarray.c:71) on exactly that slot, and `jsV_toobject` (jsvalue.c:409-411)
//!   rewrites the slot in place to `JS_TOBJECT` for every coercible primitive
//!   and throws for `undefined`/`null`.  So a live `Ap_join` frame always has an
//!   object in its `this` slot.
//!
//! * row 500 -- jsarray.c:279 `Ap_sort_cmp` reads `js_tovalue(J,0)->u.object`
//!   with no type check, and then `obj->u.a.simple` / `obj->u.a.flat_length`
//!   with no class check.  UNDEFINED BEHAVIOUR for a non-object `this`
//!   (`u.object` read out of a `JS_TSHRSTR`/`JS_TNUMBER` value) and for an
//!   object of a class that stores a different union member (`JS_CSTRING`
//!   stores `u.s.string`, whose *heap address* would be reinterpreted as
//!   `u.a.simple` / `u.a.flat_length`, so the outcome differs from run to run).
//!   Only `this` values that are plain `JS_COBJECT`s are driven here: those are
//!   `memset` to 0 by `jsV_newobject` (jsproperty.c:168), so `u.a.simple == 0`
//!   deterministically selects the generic path of rows 501/511.
//!
//! * row 511's sibling write path and row 500 share that caveat; see
//!   `t_array_sort_generic_path`.
//!
//! * row 545 -- jsobject.c:27.  `Op_toString`'s `switch (self->type)` has no
//!   `default`, but `enum js_Class` (jsi.h:313-330) has exactly the 16 members
//!   the switch enumerates, so no reachable object can fall through.
//!   `t_object_tostring_all_classes` instead drives all 16 classes -- including
//!   `JS_CSCRIPT` (via `js_loadstring`), `JS_CITERATOR` (via `js_pushiterator`)
//!   and `JS_CUSERDATA` (via `js_newuserdata`), which no JS expression can
//!   produce -- and shows both libraries push a string for every one of them.
//!
//! * row 614 -- jsstring.c:253-258.  `Sp_substring_imp`'s `js_try` handler runs
//!   `js_free(J, p)` on `p`, which is declared uninitialised at line 228 and is
//!   only assigned by the `js_malloc` at line 258.  The handler is therefore
//!   reachable with an INDETERMINATE pointer (allocation failure inside that
//!   very `js_malloc`), which is undefined behaviour -- a free() of a garbage
//!   stack value.  Not driven.  Every non-OOM path through `Sp_substring_imp`
//!   (rows 613, 615, 616) is driven in `t_string_substring_surrogates`.
//!
//! * row 637 -- jsstring.c:455.  `js_malloc(J, (top-1) * UTFmax + 1)` has no
//!   `JS_STRLIMIT` check and the product overflows `int` for
//!   `top - 1 > (INT_MAX-1)/4`, i.e. more than ~536 million arguments.  Signed
//!   integer overflow is undefined behaviour, and reaching it needs >536M stack
//!   slots (>8 GiB) which `JS_STACKSIZE == 4096` forbids anyway; the *reachable*
//!   part of the row (no RangeError however many arguments are passed) is driven
//!   in `t_string_fromcharcode_range`.
//!
//! * rows 650 / 664 -- jsstring.c:552 / jsstring.c:726 `js_toregexp(J, 1)`
//!   inside `Sp_replace_regexp` / `Sp_split_regexp`.  Both functions are
//!   `static` and are only ever entered from `Sp_replace` (jsstring.c:710) /
//!   `Sp_split` (jsstring.c:824) behind an `js_isregexp(J, 1)` test, so the
//!   TypeError cannot be raised through any exported entry point.  The exported
//!   `js_toregexp` itself IS driven with every non-regexp shape in
//!   `t_ffi_toregexp_not_a_regexp`, which asserts the same
//!   TypeError `"not a regexp"` from both libraries.
//!
//! * jsstring.c:573 -- `Sp_replace_regexp`'s FUNCTION-replacement arm walks
//!   `for (x = 0; m.sub[x].sp; ++x)`.  `regexec` (regexp.c:1235-1237) NULLs all
//!   `REG_MAXSUB == 16` entries, so a pattern with the maximum 15 capture groups
//!   whose groups all participate leaves `sub[0..15]` non-NULL and the loop
//!   reads `m.sub[16]`, one past the end of `Resub`.  Both `libmujs.so` builds
//!   segfault on it.  `t_string_replace_function_captures` caps the group count
//!   at 13.

#![allow(unused_unsafe, clippy::too_many_arguments)]

mod common;
use common::*;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void};
use std::rc::Rc;

/* ----------------------------------------------------------- name literals */

macro_rules! cn {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

const N_JOB: *const c_char = cn!("job");

/// `(expression, expected)` list whose expectations may mix `&str` literals and
/// `ok_str(...)`-built `String`s.
macro_rules! cases {
    ($(($e:expr, $w:expr $(,)?)),* $(,)?) => {
        &[$(($e, String::from($w))),*] as &[(&str, String)]
    };
}

/// `1 << 26`, the `JS_ARRAYLIMIT` of jsi.h.
const JS_ARRAYLIMIT: i64 = 1 << 26;
/// `1 << 28`, the `JS_STRLIMIT` of jsi.h.
const JS_STRLIMIT: i64 = 1 << 28;

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

/* ==========================================================================
 * The JS-level probe driver.
 *
 * Every probe body runs inside `E(function(){ ... })`, which renders either
 * `OK:<value>` or `THREW:<Name>: <message>`.  A whole batch of probes goes into
 * one script, so a single `js_ploadstring` + `js_pcall` covers dozens of rows.
 * ========================================================================== */

const PRELUDE: &str = r#"
function X(e) {
	if (e === null) return 'null';
	if (e === undefined) return 'undefined';
	if (typeof e !== 'object') return 'raw<' + (typeof e) + '>' + String(e);
	return String(e.name) + ': ' + String(e.message);
}
function Q(s) {
	var o = '"', i, c;
	for (i = 0; i < s.length; ++i) {
		c = s.charCodeAt(i);
		if (c === 34) o += '\\"';
		else if (c === 92) o += '\\\\';
		else if (c < 32 || c > 126) o += '\\u' + c;
		else o += s.charAt(i);
	}
	return o + '"';
}
function V(v) {
	var t = typeof v;
	if (v === null) return 'null';
	if (t === 'string') return 'str#' + v.length + Q(v);
	if (t === 'number') {
		if (v === 0 && 1 / v === -Infinity) return 'num(-0)';
		return 'num(' + String(v) + ')';
	}
	if (t === 'boolean' || t === 'undefined') return t + '(' + String(v) + ')';
	if (t === 'function') return 'fn';
	var cls = Object.prototype.toString.call(v);
	if (cls === '[object Array]') return 'arr' + A(v);
	return 'obj' + cls + '{' + K(v) + '}';
}
function A(a) {
	var s = '[', i;
	for (i = 0; i < a.length; ++i) {
		if (i) s += ',';
		if (!(i in a)) s += '~';
		else s += V(a[i]);
	}
	return s + ']#' + a.length;
}
function K(o) {
	var s = '', k;
	for (k in o) { if (s) s += ','; s += k + '=' + T(o[k]) }
	return s;
}
function T(v) {
	var t = typeof v;
	if (v === null) return 'null';
	if (t === 'object' || t === 'function') return Object.prototype.toString.call(v);
	if (t === 'string') return Q(v);
	return String(v);
}
function E(f) { try { return 'OK:' + V(f()) } catch (e) { return 'THREW:' + X(e) } }
function S(f) { try { return 'OK:' + String(f()) } catch (e) { return 'THREW:' + X(e) } }
"#;

struct Run {
    load_rc: c_int,
    call_rc: c_int,
    err: String,
    out: String,
}

impl PartialEq for Run {
    fn eq(&self, o: &Run) -> bool {
        self.load_rc == o.load_rc
            && self.call_rc == o.call_rc
            && self.err == o.err
            && self.out == o.out
    }
}

fn run(l: &Lib, src: &str) -> Run {
    unsafe {
        out_clear();
        let j = new_state(l, 0);
        let cs = cstr(src);
        let load_rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
        let mut call_rc = -999;
        let err;
        if load_rc == 0 {
            l.js_pushundefined(j);
            call_rc = l.js_pcall(j, 0);
            err = from_c(l.js_tryrepr(j, -1, ERRSTR));
            l.js_pop(j, 1);
        } else {
            err = from_c(l.js_tryrepr(j, -1, ERRSTR));
            l.js_pop(j, 1);
        }
        l.js_freestate(j);
        Run {
            load_rc,
            call_rc,
            err,
            out: out_take(),
        }
    }
}

/// Drive a batch of probe *function bodies* through both libraries, assert the
/// transcripts are identical, and return the per-body result strings.
///
/// The script must parse and must not throw at top level, otherwise the batch
/// would pass vacuously without ever exercising anything.
fn drive(bodies: &[String]) -> Vec<String> {
    assert!(!bodies.is_empty(), "empty batch");
    let p = libs();
    let mut res: Vec<String> = Vec::with_capacity(bodies.len());
    for chunk in bodies.chunks(50) {
        let mut src = String::from(PRELUDE);
        for (i, b) in chunk.iter().enumerate() {
            src.push_str(&format!("print('#{i}|' + E(function(){{ {b} }}));\n"));
        }
        if std::env::var_os("MUJS_DUMP").is_some() {
            eprintln!("=== src ===\n{src}");
        }
        let a = run(&p.c, &src);
        let b = run(&p.rs, &src);
        if a != b {
            panic!(
                "divergence\n--- src ---\n{src}\n--- C ---\nload={} call={} err={}\n{}\
                 \n--- RS ---\nload={} call={} err={}\n{}",
                a.load_rc, a.call_rc, a.err, a.out, b.load_rc, b.call_rc, b.err, b.out
            );
        }
        assert_eq!(a.load_rc, 0, "batch did not parse:\n{src}\nerr={}", a.err);
        assert_eq!(
            a.call_rc, 0,
            "batch threw at top level:\n{src}\nerr={}\nout={}",
            a.err, a.out
        );
        if std::env::var_os("MUJS_DUMP").is_some() {
            println!("=== batch ===\n{}", a.out);
        }
        let mut got: Vec<Option<String>> = vec![None; chunk.len()];
        for line in a.out.lines() {
            let rest = line
                .strip_prefix('#')
                .unwrap_or_else(|| panic!("unexpected output line {line:?}\nout:\n{}", a.out));
            let (idx, val) = rest
                .split_once('|')
                .unwrap_or_else(|| panic!("unexpected output line {line:?}"));
            let i: usize = idx.parse().unwrap_or_else(|_| panic!("bad index {idx:?}"));
            got[i] = Some(val.to_string());
        }
        for (i, g) in got.into_iter().enumerate() {
            res.push(g.unwrap_or_else(|| {
                panic!(
                    "probe {i} ({}) produced no output line\nout:\n{}",
                    chunk[i], a.out
                )
            }));
        }
    }
    res
}

fn body(expr: &str) -> String {
    format!("return ({expr});")
}

/// Diff-only: assert the two libraries agree on every expression's outcome.
fn diff_exprs(exprs: &[String]) {
    let bodies: Vec<String> = exprs.iter().map(|e| body(e)).collect();
    drive(&bodies);
}

/// Diff-only over full probe bodies.
fn diff_bodies(bodies: &[String]) {
    drive(bodies);
}

/// Diff-and-assert: `(expression, exact expected transcript)`.
fn expect_exprs<W: AsRef<str>>(cases: &[(&str, W)]) {
    let bodies: Vec<String> = cases.iter().map(|(e, _)| body(e)).collect();
    let got = drive(&bodies);
    for (i, (e, want)) in cases.iter().enumerate() {
        assert_eq!(got[i].as_str(), want.as_ref(), "probe expression: {e}");
    }
}

/// The transcript `V()` renders for an ASCII string result, so an expectation
/// never has to hand-count `String.prototype.length`.
fn ok_str(v: &str) -> String {
    assert!(v.is_ascii(), "ok_str is ASCII-only: {v:?}");
    let mut q = String::from("\"");
    for c in v.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                q.push_str(&format!("\\u{}", c as u32))
            }
            c => q.push(c),
        }
    }
    q.push('"');
    format!("OK:str#{}{}", v.chars().count(), q)
}

/// Diff-and-assert over *bodies* (multi-statement probes).
fn expect_bodies(cases: &[(&str, &str)]) {
    let bodies: Vec<String> = cases.iter().map(|(b, _)| b.to_string()).collect();
    let got = drive(&bodies);
    for (i, (b, want)) in cases.iter().enumerate() {
        assert_eq!(&got[i].as_str(), want, "probe body: {b}");
    }
}

/// Every `f.call(x)` / `f.apply(x)` spelling of "invoke with this == x".
/// `Fp_call` pads its argument list to `min == 1` and `Fp_apply` to `min == 2`
/// (jsR_callcfunction, jsrun.c:1259-1260), so `f.call()` really does reach the
/// callee with `this == undefined` rather than tripping `js_call`'s negative-n
/// RangeError.
fn call_spellings(recv: &str, method: &str, args: &[&str]) -> Vec<String> {
    let all = if args.is_empty() {
        String::new()
    } else {
        format!(", {}", args.join(", "))
    };
    let arr = format!("[{}]", args.join(", "));
    vec![
        format!("{method}.call({recv}{all})"),
        format!("{method}.apply({recv}, {arr})"),
    ]
}

/* ------------------------------------------------------- the generic probe */

thread_local! {
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

unsafe fn drain_to(l: &Lib, j: JS, base: c_int) {
    let t = l.js_gettop(j);
    if t > base {
        l.js_pop(j, t - base);
    }
}

/// Run `f` inside `js_pcall` and return a transcript of `(rc, thrown-or-result,
/// stack delta)`.  NOTHING that can throw may be called outside this.
unsafe fn probe(l: &Lib, j: JS, f: Rc<dyn Fn(&Lib, JS) -> String>) -> String {
    JOB.with(|b| *b.borrow_mut() = Some(f));
    let base = l.js_gettop(j);
    l.js_newcfunction(j, Some(cf_job), N_JOB, 0);
    l.js_pushundefined(j);
    let rc = l.js_pcall(j, 0);
    // `js_torepr` (jsrepr.c:271) and `js_tostring` (jsvalue.c:344/360) REWRITE
    // the slot they are handed, so always stringify a COPY and leave the thrown
    // value itself intact for the predicates.
    let ty = from_c(l.js_typeof(j, -1));
    let iserr = l.pred("js_iserror", j, -1);
    let mut name = String::from("-");
    let mut msg = String::from("-");
    if iserr != 0 {
        l.js_copy(j, -1);
        if l.js_hasproperty(j, -1, cn!("name")) != 0 {
            l.js_copy(j, -1);
            name = from_c(l.js_trystring(j, -1, cn!("<nostr>")));
            l.js_pop(j, 2);
        }
        if l.js_hasproperty(j, -1, cn!("message")) != 0 {
            l.js_copy(j, -1);
            msg = from_c(l.js_trystring(j, -1, cn!("<nostr>")));
            l.js_pop(j, 2);
        }
        l.js_pop(j, 1);
    }
    l.js_copy(j, -1);
    let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
    l.js_pop(j, 2);
    let after = l.js_gettop(j);
    let r = format!("[rc={rc} ty={ty} v={v} err={iserr} {name}: {msg} top {base}->{after}]");
    drain_to(l, j, base);
    r
}

macro_rules! job {
    (|$l:ident, $j:ident| $body:block) => {
        std::rc::Rc::new(move |$l: &Lib, $j: JS| -> String { unsafe { $body } })
            as std::rc::Rc<dyn Fn(&Lib, JS) -> String>
    };
}

/// One-shot: fresh state, run `f` protected, free the state.
fn probe_state(tag: &str, flags: c_int, f: impl Fn() -> Rc<dyn Fn(&Lib, JS) -> String>) -> String {
    diff2(tag, move |l| unsafe {
        let j = new_state(l, flags);
        let r = probe(l, j, f());
        let t = l.js_gettop(j);
        l.js_freestate(j);
        format!("{r} endtop={t}")
    })
}

/// Evaluate `src` as a script inside the current (protected) frame and leave its
/// value on the stack.  Returns the load/call rc pair.
unsafe fn push_expr(l: &Lib, j: JS, src: &str) -> c_int {
    let cs = cstr(src);
    let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
    if rc != 0 {
        return 100 + rc;
    }
    l.js_pushundefined(j);
    l.js_pcall(j, 0)
}

/// stdout serialisation for anything that writes to the real stdout
/// (`js_gc(J, 1)`, `jsS_dumpstrings`).  Nothing in this file needs it yet, but
/// keeping the lock here means a future test cannot silently make the suite
/// flaky.
#[allow(dead_code)]
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[allow(dead_code)]
fn stdout_guard() -> std::sync::MutexGuard<'static, ()> {
    STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/* =========================================================================
 *  SMOKE: the probe plumbing itself.
 * ========================================================================= */

#[test]
fn t_smoke_plumbing() {
    expect_exprs(&[
        ("1+1", "OK:num(2)"),
        ("'ab'", "OK:str#2\"ab\""),
        ("[1,,3]", "OK:arr[num(1),~,num(3)]#3"),
        ("null", "OK:null"),
        ("undefined", "OK:undefined(undefined)"),
        ("(function(){ throw new TypeError('boom') })()", "THREW:TypeError: boom"),
        ("(function(){ throw 'str' })()", "THREW:raw<string>str"),
        ("(-0)", "OK:num(-0)"),
    ]);
}

/* =========================================================================
 *  jsarray.c
 * ========================================================================= */

/// Row 477 -- jsarray.c:11 `js_getlength`.  `js_tointeger` funnels through
/// `jsV_numbertointeger` (jsvalue.c:41-48): NaN -> 0, `n < INT_MIN` -> INT_MIN,
/// `n > INT_MAX` -> INT_MAX, everything else truncated toward zero.  No throw.
#[test]
fn t_array_getlength_coercion() {
    let mut vals: Vec<String> = vec![
        "0".into(),
        "1".into(),
        "3".into(),
        "-1".into(),
        "-3".into(),
        "1.5".into(),
        "-1.5".into(),
        "0.9".into(),
        "-0.9".into(),
        "-0".into(),
        "NaN".into(),
        "Infinity".into(),
        "-Infinity".into(),
        "1e30".into(),
        "-1e30".into(),
        "2147483647".into(),
        "2147483648".into(),
        "-2147483648".into(),
        "-2147483649".into(),
        "4294967296".into(),
        "'7'".into(),
        "'abc'".into(),
        "''".into(),
        "'  12  '".into(),
        "null".into(),
        "undefined".into(),
        "true".into(),
        "false".into(),
        "({})".into(),
        "[]".into(),
        "[5]".into(),
        "[1,2]".into(),
        "(function(){})".into(),
        "new Number(9)".into(),
        "new String('4')".into(),
    ];
    let mut rng = Rng::new(0x4771_0001);
    for _ in 0..24 {
        vals.push(format!("{}", rng.range(-2147483650, 2147483650)));
    }
    for _ in 0..12 {
        vals.push(format!("{:?}", rng.f64_sane()).replace("inf", "Infinity"));
    }
    // through the exported js_getlength, which IS jsarray.c:7-14
    for v in vals.clone() {
        let expr = format!("({{length: {v}}})");
        probe_state(&format!("js_getlength length={v}"), 0, move || {
            let e = expr.clone();
            job!(|l, j| {
                let rc = push_expr(l, j, &e);
                if rc != 0 {
                    return format!("push rc={rc}");
                }
                let n = l.js_getlength(j, -1);
                format!("rc={rc} len={n}")
            })
        });
    }
    // no `length` property at all, and a throwing / mutating getter
    for setup in [
        "({})",
        "[]",
        "[1,2,3]",
        "(function(){ var o={}; Object.defineProperty(o,'length',\
          {get:function(){ throw new RangeError('len!') }}); return o })()",
        "(function(){ var o={}; Object.defineProperty(o,'length',\
          {get:function(){ return '5' }}); return o })()",
        "new String('abcd')",
        "Object.create(null)",
    ] {
        probe_state(&format!("js_getlength on {setup}"), 0, move || {
            job!(|l, j| {
                let rc = push_expr(l, j, setup);
                if rc != 0 {
                    return format!("push rc={rc}");
                }
                let n = l.js_getlength(j, -1);
                format!("rc={rc} len={n}")
            })
        });
    }
    // and through a JS entry point that reads length without looping over it
    let mut exprs: Vec<String> = Vec::new();
    for v in &vals {
        exprs.push(format!("Array.prototype.pop.call({{length: {v}}})"));
        exprs.push(format!(
            "(function(){{ var o = {{length: {v}}}; Array.prototype.pop.call(o); \
             return String(o.length) }})()"
        ));
    }
    diff_exprs(&exprs);
}

/// Rows 478 / 479 -- jsarray.c:19 `js_setlength` propagating jsrun.c:707
/// RangeError `"invalid array length"` and jsrun.c:709 RangeError
/// `"array too large"`.  `js_setlength` takes an `int`, so the
/// `newlen != rawlen` half of the jsrun.c:707 test can never fire through it;
/// only `newlen < 0` can.
#[test]
fn t_array_setlength_rangeerrors() {
    let mut lens: Vec<i64> = vec![
        0,
        1,
        3,
        -1,
        -2,
        -3,
        JS_ARRAYLIMIT - 1,
        JS_ARRAYLIMIT,
        JS_ARRAYLIMIT + 1,
        JS_ARRAYLIMIT + 2,
        2147483647,
        -2147483648,
    ];
    let mut rng = Rng::new(0x4780_0002);
    for _ in 0..16 {
        lens.push(JS_ARRAYLIMIT + rng.range(-3, 4));
    }
    for _ in 0..16 {
        lens.push(rng.range(-40, 40));
    }
    for n in lens {
        for target in ["array", "object"] {
            let t = probe_state(
                &format!("js_setlength {target} n={n}"),
                0,
                move || {
                    job!(|l, j| {
                        if target == "array" {
                            l.js_newarray(j);
                            for i in 0..3 {
                                l.js_pushnumber(j, i as f64);
                                l.js_setindex(j, -2, i);
                            }
                        } else {
                            l.js_newobject(j);
                        }
                        l.js_setlength(j, -1, n as c_int);
                        format!("len={}", l.js_getlength(j, -1))
                    })
                },
            );
            if target == "array" {
                if n < 0 {
                    assert!(
                        t.contains("RangeError: invalid array length"),
                        "js_setlength({n}) on an array: {t}"
                    );
                } else if n > JS_ARRAYLIMIT {
                    assert!(
                        t.contains("RangeError: array too large"),
                        "js_setlength({n}) on an array: {t}"
                    );
                } else {
                    assert!(t.contains("rc=0"), "js_setlength({n}) on an array: {t}");
                }
            } else {
                assert!(t.contains("rc=0"), "js_setlength({n}) on an object: {t}");
            }
        }
    }
    // row 478 reached from JS: Ap_map's `js_setlength(J, -1, len)` (jsarray.c:709)
    // with a negative `this.length`.
    expect_exprs(&[
        (
            "Array.prototype.map.call({length:-5}, function(){})",
            "THREW:RangeError: invalid array length",
        ),
        (
            "Array.prototype.map.call({length:-1}, function(){})",
            "THREW:RangeError: invalid array length",
        ),
        (
            "Array.prototype.map.call({length:'-2'}, function(){})",
            "THREW:RangeError: invalid array length",
        ),
        (
            "Array.prototype.map.call({length:-1.5}, function(){})",
            "THREW:RangeError: invalid array length",
        ),
        ("Array.prototype.map.call({length:0}, function(){})", "OK:arr[]#0"),
        ("Array.prototype.map.call({length:NaN}, function(){})", "OK:arr[]#0"),
        // row 479 reached from JS: Ap_push's `js_setlength(J, 0, n)`
        // (jsarray.c:197) once `n` steps past JS_ARRAYLIMIT.
        (
            "(function(){ var a=[]; a.length=67108864; return a.push(1) })()",
            "THREW:RangeError: array too large",
        ),
        (
            "(function(){ var a=[]; a.length=67108863; return a.push(1) })()",
            "OK:num(67108864)",
        ),
        (
            "(function(){ var a=[]; a.length=67108864; a.push(1); return 'no throw' })()",
            "THREW:RangeError: array too large",
        ),
    ]);
}

/// Rows 480 / 481 / 482 -- jsarray.c:28-35 `jsB_new_Array`'s single-argument
/// fork.  `top == 2 && js_isnumber(J,1)` sets `length` (and inherits both
/// jsrun.c:707/709 RangeErrors); anything else stores the value at index 0.
#[test]
fn t_array_constructor_single_arg() {
    let mut cases: Vec<(String, Option<&'static str>)> = vec![
        ("-1".into(), Some("THREW:RangeError: invalid array length")),
        ("-2".into(), Some("THREW:RangeError: invalid array length")),
        ("1.5".into(), Some("THREW:RangeError: invalid array length")),
        ("-1.5".into(), Some("THREW:RangeError: invalid array length")),
        ("0.5".into(), Some("THREW:RangeError: invalid array length")),
        ("NaN".into(), Some("THREW:RangeError: invalid array length")),
        ("Infinity".into(), Some("THREW:RangeError: invalid array length")),
        (
            "-Infinity".into(),
            Some("THREW:RangeError: invalid array length"),
        ),
        ("1e30".into(), Some("THREW:RangeError: invalid array length")),
        (
            "2147483648".into(),
            Some("THREW:RangeError: invalid array length"),
        ),
        ("67108865".into(), Some("THREW:RangeError: array too large")),
        ("67108866".into(), Some("THREW:RangeError: array too large")),
        (
            "1000000000".into(),
            Some("THREW:RangeError: array too large"),
        ),
        ("67108864".into(), None),
        ("67108863".into(), None),
        ("0".into(), None),
        ("-0".into(), None),
        ("3".into(), None),
        // row 482: a non-number single argument never touches `length`
        ("'3'".into(), None),
        ("'x'".into(), None),
        ("undefined".into(), None),
        ("null".into(), None),
        ("true".into(), None),
        ("({})".into(), None),
        ("[]".into(), None),
        ("[1,2]".into(), None),
        ("new Number(3)".into(), None),
        ("new Number(-1)".into(), None),
    ];
    let mut rng = Rng::new(0x4800_0003);
    for _ in 0..16 {
        let n = JS_ARRAYLIMIT + rng.range(-3, 4);
        cases.push((
            format!("{n}"),
            if n > JS_ARRAYLIMIT {
                Some("THREW:RangeError: array too large")
            } else {
                None
            },
        ));
    }
    let mut exprs: Vec<(String, Option<&'static str>)> = Vec::new();
    for (v, want) in cases {
        // the object's shape for the accepted cases: `length` only, or a
        // one-element array.
        exprs.push((
            format!("(function(){{ var a = new Array({v}); return a.length + '/' + (0 in a) + '/' + String(a[0]) }})()"),
            want,
        ));
        exprs.push((format!("(function(){{ var a = Array({v}); return a.length + '/' + (0 in a) + '/' + String(a[0]) }})()"), want));
    }
    let bodies: Vec<String> = exprs.iter().map(|(e, _)| body(e)).collect();
    let got = drive(&bodies);
    for (i, (e, want)) in exprs.iter().enumerate() {
        if let Some(w) = want {
            assert_eq!(&got[i].as_str(), w, "new Array: {e}");
        } else {
            assert!(got[i].starts_with("OK:"), "new Array: {e} -> {}", got[i]);
        }
    }
    // multi-argument and zero-argument forms take the jsarray.c:37-40 branch
    diff_bodies(&[
        body("(function(){ var a=new Array(); return a.length })()"),
        body("(function(){ var a=new Array(1,2,3); return a.length+'/'+a.join('|') })()"),
        body("(function(){ var a=new Array(-1,-2); return a.length+'/'+a.join('|') })()"),
        body("(function(){ var a=new Array(1.5,2.5); return a.join('|') })()"),
        body("(function(){ var a=new Array(undefined,undefined); return a.length+'/'+(0 in a) })()"),
    ]);
}

/// Rows 483 / 489 / 490 / 492 -- `Ap_join`: the `js_toobject(J,0)` TypeError,
/// the cycle guard, the `len <= 0` early return and the non-coercible element
/// fork.
#[test]
fn t_array_join_paths() {
    expect_exprs(&[
        // row 483: js_toobject(J, 0) on a non-coercible `this` (jsvalue.c:401/402)
        (
            "Array.prototype.join.call(undefined)",
            "THREW:TypeError: cannot convert undefined to object",
        ),
        (
            "Array.prototype.join.call(null)",
            "THREW:TypeError: cannot convert null to object",
        ),
        (
            "Array.prototype.join.apply(undefined, [])",
            "THREW:TypeError: cannot convert undefined to object",
        ),
        (
            "Array.prototype.join.apply(null, ['-'])",
            "THREW:TypeError: cannot convert null to object",
        ),
        (
            "Array.prototype.join.call(undefined, ',')",
            "THREW:TypeError: cannot convert undefined to object",
        ),
        // row 489: self-referential array -> the guard pushes ""
        ("(function(){ var a=[]; a[0]=a; return a.join() })()", "OK:str#0\"\""),
        (
            "(function(){ var a=[1,2]; a[2]=a; return a.join('-') })()",
            "OK:str#4\"1-2-\"",
        ),
        (
            "(function(){ var a=[]; a[0]=a; a[1]='x'; return a.join('+') })()",
            "OK:str#2\"+x\"",
        ),
        // row 490: len <= 0
        ("[].join()", "OK:str#0\"\""),
        ("Array.prototype.join.call({length:0})", "OK:str#0\"\""),
        ("Array.prototype.join.call({length:-5, 0:'a'})", "OK:str#0\"\""),
        ("Array.prototype.join.call({length:NaN, 0:'a'})", "OK:str#0\"\""),
        ("Array.prototype.join.call({}, '-')", "OK:str#0\"\""),
        // row 492: undefined / null elements contribute nothing
        ("[1,null,undefined,2].join('-')", "OK:str#5\"1---2\""),
        ("[undefined,undefined].join('-')", "OK:str#1\"-\""),
        ("[null].join()", "OK:str#0\"\""),
        ("[undefined].join()", "OK:str#0\"\""),
        ("[,,].join('-')", "OK:str#1\"-\""),
        ("[1,undefined,null].join('')", "OK:str#1\"1\""),
        // separator coercion: js_isdefined(J,1) is the fork at jsarray.c:113
        ("[1,2].join(undefined)", "OK:str#3\"1,2\""),
        ("[1,2].join(null)", "OK:str#6\"1null2\""),
        ("[1,2].join('')", "OK:str#2\"12\""),
        ("[1,2].join(0)", "OK:str#3\"102\""),
        ("[1,2].join({})", "OK:str#17\"1[object Object]2\""),
    ]);
    // row 491: any throw inside the join loop re-raises through the js_try
    // handler at jsarray.c:126-129 after freeing `out`.
    expect_exprs(&[
        (
            "[1, {toString: function(){ throw new RangeError('elem') }}].join('-')",
            "THREW:RangeError: elem",
        ),
        (
            "[{toString: function(){ throw new RangeError('first') }}, 1].join('-')",
            "THREW:RangeError: first",
        ),
        (
            "[1,2,3].join({toString: function(){ throw new TypeError('sep') }})",
            "THREW:TypeError: sep",
        ),
        (
            "(function(){ var n=0; return [1,2,3,4].join({toString:function(){ \
             if (++n===3) throw new Error('sep3'); return '-' }}) })()",
            "OK:str#7\"1-2-3-4\"",
        ),
    ]);
    // and the same throw seen from a deeper nesting, so the freed `out` cannot
    // be reused
    diff_bodies(&[
        body(
            "(function(){ var t=0; var o={toString:function(){ if(++t>2) throw new Error('x'); \
             return 'v'+t }}; return [o,o,o,o].join('|') })()",
        ),
        body("[1,[2,[3,{toString:function(){throw 'deep'}}]]].join('-')"),
    ]);
}

/// Rows 484-488 -- `Ap_join_cycle`'s five `return 0` exits.
///
/// `trace[top].stack` is `J->bot` (jsrun.c:1292), so `stack[stk-1]` is the
/// callee and `stack[stk]` is `this` for that frame.  Rows 484 and 486 are
/// unreachable (see the file header).  Row 485 fires for *every* top-level
/// `a.join()`, because the walk immediately meets the enclosing `JS_CSCRIPT`
/// frame; row 487 needs a `JS_CCFUNCTION` frame that is neither `Ap_join` nor
/// `Ap_toString` below the join; row 488 needs the walk to reach `top == 0`,
/// i.e. `Ap_join` invoked directly by the host with no script frame at all.
#[test]
fn t_array_join_cycle_bailouts() {
    // row 485: the frame below the join is a JS_CSCRIPT / JS_CFUNCTION object.
    // With no real cycle this is invisible; with a cycle routed through a
    // *script* function the guard is defeated and the recursion runs until
    // jsrun.c:1289 "call stack overflow".
    with_big_stack(|| {
        expect_exprs(&[
            // Ap_join installs a `js_try` (jsarray.c:126) on every invocation
            // once `len > 0`, so the runaway recursion exhausts JS_TRYLIMIT
            // (64) and raises the BARE STRING "exception stack overflow"
            // (jsrun.c:1435) well before jsR_pushtrace's JS_ENVLIMIT.
            (
                "(function(){ var a=[]; a[0]={toString:function(){ return a.join() }}; \
                 return a.join() })()",
                "THREW:raw<string>exception stack overflow",
            ),
            (
                "(function(){ var a=[]; a[0]={toString:function(){ return a.toString() }}; \
                 return a.join('-') })()",
                "THREW:raw<string>exception stack overflow",
            ),
        ]);
        // the same guard, but the cycle closes through Ap_toString only, so it
        // IS detected
        expect_exprs(&[
            ("(function(){ var a=[]; a[0]=a; return String(a) })()", "OK:str#0\"\""),
            (
                "(function(){ var a=[], b=[a]; a[0]=b; return String(a) })()",
                "OK:str#0\"\"",
            ),
            (
                "(function(){ var a=[], b=[a]; a[0]=b; return a.join('-') })()",
                "OK:str#0\"\"",
            ),
        ]);
    });
    // row 487: a CCFUNCTION frame that is neither Ap_join nor Ap_toString
    // (jsB_String / jsB_encodeURI / jsB_parseInt / Ap_map ...) sits below the
    // first Ap_join, so the walk gives up there.
    diff_bodies(&[
        body("(function(){ var a=[]; a[0]=a; return String(a) })()"),
        body("(function(){ var a=[]; a[0]=a; return encodeURI(a) })()"),
        body("(function(){ var a=[1]; a[1]=a; return String(parseInt(a)) })()"),
        body("(function(){ var a=[]; a[0]=a; return [a].join('*') })()"),
        body("(function(){ var a=[]; a[0]=a; return [a,a].map(String).join('*') })()"),
        body("(function(){ var a=[]; a[0]=a; return JSON.stringify([String(a)]) })()"),
        body("(function(){ var a=[]; a[0]=a; return a.concat([a]).join('/') })()"),
        body("(function(){ var a=[], b=[a], c=[b]; a[0]=c; return String(a)+'|'+String(b) })()"),
    ]);
    // row 488: Ap_join called straight from the host through js_pcall, so
    // `J->tracetop == 1` and the `while (top > 0)` loop never runs (or walks
    // down to 0 through join/toString frames only).
    for setup in [
        "[1,2,3]",
        "[]",
        "(function(){ var a=[]; a[0]=a; return a })()",
        "(function(){ var a=[[1],[2]]; return a })()",
        "(function(){ var a=[], b=[a]; a[0]=b; return a })()",
        "({length:2, 0:'a', 1:'b'})",
    ] {
        for sep in ["none", "dash"] {
            probe_state(
                &format!("pcall Ap_join {setup} sep={sep}"),
                0,
                move || {
                    job!(|l, j| {
                        let rc = push_expr(l, j, setup);
                        if rc != 0 {
                            return format!("push rc={rc}");
                        }
                        l.js_getglobal(j, cn!("Array"));
                        l.js_getproperty(j, -1, cn!("prototype"));
                        l.js_getproperty(j, -1, cn!("join"));
                        // stack: value, Array, prototype, join
                        l.js_copy(j, -4); // this
                        let n = if sep == "none" {
                            0
                        } else {
                            l.js_pushstring(j, cn!("-"));
                            1
                        };
                        let rc2 = l.js_pcall(j, n);
                        let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
                        format!("rc={rc} rc2={rc2} v={v}")
                    })
                },
            );
        }
    }
}

/// Rows 494 / 495 -- `Ap_pop`'s `n > 0` fork (jsarray.c:175) and `Ap_shift`'s
/// `len == 0` fork (jsarray.c:236).  Note `Ap_shift` tests `== 0`, not `<= 0`,
/// so a NEGATIVE length takes the shifting path with a negative `len - 1`.
#[test]
fn t_array_pop_shift_empty() {
    let mut shapes: Vec<String> = vec![
        "[]".into(),
        "[1]".into(),
        "[1,2,3]".into(),
        "({length:0})".into(),
        "({length:-1, 0:'a'})".into(),
        "({length:-3, 0:'a'})".into(),
        "({length:NaN, 0:'a'})".into(),
        "({length:'0'})".into(),
        "({})".into(),
        "({length:1, 0:'a'})".into(),
        "({length:2, 0:'a'})".into(),
        "({length:1.9, 0:'a', 1:'b'})".into(),
        "(function(){ var a=[1,2,3]; a[7]=8; return a })()".into(),
        "new Array(3)".into(),
    ];
    let mut rng = Rng::new(0x4940_0004);
    for _ in 0..10 {
        shapes.push(format!("({{length:{}, 0:'a', 1:'b'}})", rng.range(-5, 6)));
    }
    let mut bodies: Vec<String> = Vec::new();
    for s in &shapes {
        for m in ["pop", "shift"] {
            bodies.push(format!(
                "var o = {s}; var r = Array.prototype.{m}.call(o); \
                 return String(r) + '/len=' + String(o.length) + '/0=' + (0 in o) + \
                 '/' + String(o[0]) + '/' + String(o[1]);"
            ));
            bodies.push(format!(
                "var o = {s}; return String(Array.prototype.{m}.apply(o, []));"
            ));
        }
    }
    diff_bodies(&bodies);
}

/// Rows 496-499 -- `Ap_slice`'s four index adjustments (jsarray.c:266-270).
#[test]
fn t_array_slice_clamping() {
    let idx: &[&str] = &[
        "undefined",
        "0",
        "1",
        "2",
        "3",
        "4",
        "-1",
        "-2",
        "-3",
        "-4",
        "-5",
        "100",
        "-100",
        "1.5",
        "-1.5",
        "NaN",
        "Infinity",
        "-Infinity",
        "'2'",
        "'-2'",
        "'x'",
        "null",
        "true",
        "2147483648",
        "-2147483649",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for a in idx {
        for b in idx {
            bodies.push(format!(
                "return A([1,2,3].slice({a}, {b})) + '|' + \
                 A(Array.prototype.slice.call({{length:3,0:'x',2:'z'}}, {a}, {b}));"
            ));
        }
    }
    // and negative / clamped ranges against a length that itself coerces oddly
    for a in ["-1", "0", "2", "-100"] {
        for l in ["-3", "0", "NaN", "1.5", "'2'"] {
            bodies.push(format!(
                "return A(Array.prototype.slice.call({{length:{l},0:'a',1:'b'}}, {a}));"
            ));
        }
    }
    diff_bodies(&bodies);
}

/// Rows 501-511 -- `Ap_sort_cmp` / `Ap_sort_swap`.
///
/// The flat fast path (jsarray.c:280) is taken only for `u.a.simple` arrays with
/// `idx_b < flat_length`; everything else falls into the generic
/// has/get/set/del path, which is also the only path that can see HOLES.
/// `this` is restricted to real arrays and plain `JS_COBJECT`s -- see the file
/// header for why any other class would be undefined behaviour at jsarray.c:279.
#[test]
fn t_array_sort_generic_path() {
    let shapes: &[&str] = &[
        // flat, simple arrays: rows 502/503/504
        "[3,1,2]",
        "[2,1]",
        "[1,undefined,2]",
        "[undefined,1]",
        "[1,undefined]",
        "[undefined,undefined]",
        "[undefined,undefined,undefined]",
        "['b','a','c']",
        "[10,9,1,2]",
        "[1,1,1]",
        "[null,undefined,0,'']",
        // unflattened arrays with holes: rows 501/505/506/507/508/509/510/511
        "(function(){ var a=[3,1,2]; a[8]=0; return a })()",
        "(function(){ var a=[3,1,2]; a[8]=0; delete a[1]; return a })()",
        "(function(){ var a=[3,1,2,4]; delete a[1]; delete a[2]; return a })()",
        "(function(){ var a=new Array(4); a[0]=2; a[3]=1; return a })()",
        "new Array(3)",
        "new Array(4)",
        "(function(){ var a=[undefined,1]; a[5]=0; delete a[3]; return a })()",
        "(function(){ var a=[1,undefined]; a[4]=2; return a })()",
        // plain objects: u is memset to 0 by jsV_newobject, so u.a.simple == 0
        // deterministically selects the generic path
        "({length:3, 0:'b', 1:'a', 2:'c'})",
        "({length:3, 0:'b', 2:'a'})",
        "({length:3})",
        "({length:2, 0:undefined, 1:1})",
        "({length:2, 1:1})",
        "({length:4, 0:4, 1:3, 2:2, 3:1})",
    ];
    let cmps: &[&str] = &[
        "undefined",
        "function(a,b){ return a-b }",
        "function(a,b){ return b-a }",
        "function(a,b){ return 0 }",
        "function(a,b){ return 0/0 }",
        "function(a,b){ return NaN }",
        "function(a,b){ return String(a)<String(b) ? -1 : 1 }",
        "function(a,b){ return -0 }",
        "function(a,b){ return a===undefined ? -1 : 1 }",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in shapes {
        for c in cmps {
            bodies.push(format!(
                "var o = {s}; var r = Array.prototype.sort.call(o, {c}); \
                 var t = 'len=' + String(o.length); var i; \
                 for (i = 0; i < 9; ++i) t += '/' + (i in o ? T(o[i]) : '~'); \
                 return t + '|same=' + (r === o);"
            ));
        }
    }
    diff_bodies(&bodies);
    // randomised shapes so the heapsort walks many different comparison orders
    let mut rng = Rng::new(0x5010_0005);
    let mut rnd: Vec<String> = Vec::new();
    for _ in 0..60 {
        let n = rng.below(9) as usize;
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..n {
            parts.push(match rng.below(6) {
                0 => "undefined".into(),
                1 => "null".into(),
                2 => format!("{}", rng.range(-5, 6)),
                3 => format!("'{}'", (b'a' + rng.below(5) as u8) as char),
                4 => "NaN".into(),
                _ => format!("{}", rng.range(0, 3)),
            });
        }
        let arr = format!("[{}]", parts.join(","));
        let extra = if rng.below(2) == 0 {
            format!("a[{}]=7;", 9 + rng.below(3))
        } else {
            String::new()
        };
        let del = if n > 1 && rng.below(2) == 0 {
            format!("delete a[{}];", rng.below(n as u32))
        } else {
            String::new()
        };
        let c = cmps[rng.below(cmps.len() as u32) as usize];
        rnd.push(format!(
            "var a = {arr}; {extra} {del} Array.prototype.sort.call(a, {c}); \
             var t = 'len=' + String(a.length); var i; \
             for (i = 0; i < 12; ++i) t += '/' + (i in a ? T(a[i]) : '~'); return t;"
        ));
    }
    diff_bodies(&rnd);
}

/// Rows 512 / 513 / 514 -- `Ap_sort`'s three guards.
#[test]
fn t_array_sort_guards() {
    expect_exprs(&[
        // row 512: len <= 1 returns `this` untouched, and the row-513 guard is
        // never reached
        (
            "(function(){ var a=[]; return (a.sort()===a) + '/' + a.length })()",
            "OK:str#6\"true/0\"",
        ),
        (
            "(function(){ var a=[5]; return (a.sort()===a) + '/' + a[0] })()",
            "OK:str#6\"true/5\"",
        ),
        (
            "(function(){ var a=[5]; return (a.sort(1)===a) + '/' + a[0] })()",
            "OK:str#6\"true/5\"",
        ),
        (
            "(function(){ var o={length:1, 0:'z'}; \
             return (Array.prototype.sort.call(o,'nope')===o) + '/' + o[0] })()",
            "OK:str#6\"true/z\"",
        ),
        (
            "String(Array.prototype.sort.call({length:-4}, 'nope').length)",
            "OK:str#2\"-4\"",
        ),
        // row 513: a comparator that is neither callable nor undefined
        (
            "[3,1].sort(1)",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
        (
            "[3,1].sort('x')",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
        (
            "[3,1].sort(null)",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
        (
            "[3,1].sort({})",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
        (
            "[3,1].sort([])",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
        (
            "[3,1].sort(true)",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
        (
            "[3,1].sort(0)",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
        (
            "[3,1].sort(/re/)",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
        (
            "[3,1].sort(new Number(1))",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
        ("[3,1].sort(undefined).join('|')", "OK:str#3\"1|3\""),
        // row 514: len >= INT_MAX, reached because jsV_numbertointeger clamps
        (
            "Array.prototype.sort.call({length:1e30})",
            "THREW:RangeError: array is too large to sort",
        ),
        (
            "Array.prototype.sort.call({length:2147483647})",
            "THREW:RangeError: array is too large to sort",
        ),
        (
            "Array.prototype.sort.call({length:Infinity})",
            "THREW:RangeError: array is too large to sort",
        ),
        (
            "Array.prototype.sort.call({length:1e30}, function(a,b){return 0})",
            "THREW:RangeError: array is too large to sort",
        ),
        // ... and the guard order: the callable check (row 513) runs first
        (
            "Array.prototype.sort.call({length:1e30}, 7)",
            "THREW:TypeError: comparison function must be a function or undefined",
        ),
    ]);
    // `Ap_sort` never calls `js_toobject` itself, so a non-coercible `this`
    // fails one level down, inside `js_getlength` -> `js_getproperty` ->
    // `js_toobject` (jsvalue.c:401/402).
    expect_exprs(&[
        (
            "Array.prototype.sort.call(undefined)",
            "THREW:TypeError: cannot convert undefined to object",
        ),
        (
            "Array.prototype.sort.call(null)",
            "THREW:TypeError: cannot convert null to object",
        ),
    ]);
}

/// Rows 515-518 -- `Ap_splice`'s start / deleteCount clamping.
#[test]
fn t_array_splice_clamping() {
    let starts: &[&str] = &[
        "undefined", "0", "1", "2", "3", "5", "-1", "-2", "-5", "-100", "100", "1.5", "-1.5", "NaN",
        "Infinity", "-Infinity", "'2'", "'-1'", "null", "true",
    ];
    let dels: &[&str] = &[
        "undefined",
        "0",
        "1",
        "2",
        "5",
        "-1",
        "-100",
        "100",
        "1.5",
        "NaN",
        "Infinity",
        "-Infinity",
        "'1'",
        "null",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in starts {
        for d in dels {
            bodies.push(format!(
                "var a=[1,2,3,4]; var r=a.splice({s}, {d}); \
                 return 'del=' + A(r) + ' left=' + A(a);"
            ));
        }
    }
    for s in starts {
        bodies.push(format!(
            "var a=[1,2,3]; var r=a.splice({s}, 1, 'X', 'Y'); \
             return 'del=' + A(r) + ' left=' + A(a);"
        ));
        bodies.push(format!(
            "var o={{length:3,0:'a',2:'c'}}; \
             var r=Array.prototype.splice.call(o, {s}, 2, 'X'); \
             var t='del=' + A(r) + ' len=' + String(o.length); var i; \
             for (i=0;i<5;++i) t += '/' + (i in o ? T(o[i]) : '~'); return t;"
        ));
    }
    diff_bodies(&bodies);
}

/// Rows 519 / 520 -- `Ap_toString`'s coercibility guard and its
/// Object.prototype.toString substitution when `this.join` is not callable.
#[test]
fn t_array_tostring() {
    expect_exprs(&[
        (
            "Array.prototype.toString.call(undefined)",
            "THREW:TypeError: 'this' is not an object",
        ),
        (
            "Array.prototype.toString.call(null)",
            "THREW:TypeError: 'this' is not an object",
        ),
        (
            "Array.prototype.toString.apply(undefined, [])",
            "THREW:TypeError: 'this' is not an object",
        ),
        // row 520: join absent / not callable -> Object.prototype.toString
        ("Array.prototype.toString.call({join:1})", "OK:str#15\"[object Object]\""),
        ("Array.prototype.toString.call({})", "OK:str#15\"[object Object]\""),
        ("Array.prototype.toString.call({join:null})", "OK:str#15\"[object Object]\""),
        ("Array.prototype.toString.call(5)", "OK:str#15\"[object Number]\""),
        ("Array.prototype.toString.call('s')", "OK:str#15\"[object String]\""),
        ("Array.prototype.toString.call(true)", "OK:str#16\"[object Boolean]\""),
        ("Array.prototype.toString.call(/re/)", "OK:str#15\"[object RegExp]\""),
        ("Array.prototype.toString.call(Math)", "OK:str#13\"[object Math]\""),
        // ... and a callable join really is used
        (
            "Array.prototype.toString.call({join:function(){ return 'J' }})",
            "OK:str#1\"J\"",
        ),
        (
            "Array.prototype.toString.call({join:function(){ throw new Error('J!') }})",
            "THREW:Error: J!",
        ),
        ("[1,2].toString()", "OK:str#3\"1,2\""),
        (
            "Array.prototype.toString.call({length:2,0:'a',1:'b'})",
            "OK:str#15\"[object Object]\"",
        ),
    ]);
}

/// Rows 521-526 -- `Ap_indexOf` / `Ap_lastIndexOf` fromIndex handling and the
/// `-1` not-found sentinels.  Note jsarray.c:581/582 clamp to `len - 1` BEFORE
/// rebasing a negative argument, so a negative `fromIndex` can stay negative.
#[test]
fn t_array_indexof_bounds() {
    let froms: &[&str] = &[
        "undefined",
        "0",
        "1",
        "2",
        "3",
        "4",
        "10",
        "-1",
        "-2",
        "-3",
        "-4",
        "-10",
        "1.5",
        "-1.5",
        "NaN",
        "Infinity",
        "-Infinity",
        "'1'",
        "'-1'",
        "null",
        "true",
        "2147483648",
        "-2147483649",
    ];
    let needles: &[&str] = &["1", "2", "3", "'2'", "undefined", "null", "NaN", "0"];
    // `length >= 0` shapes: every `fromIndex` above is safe, because
    // `len + from` (jsarray.c:558/582) cannot overflow `int` for `len >= 0` and
    // `from >= INT_MIN`.
    let shapes: &[&str] = &[
        "[1,2,3]",
        "[1,2,3,2]",
        "[]",
        "[undefined]",
        "(function(){ var a=[1,2,3]; delete a[1]; return a })()",
        "({length:3, 0:1, 2:3})",
        "({length:NaN, 0:1})",
        "({length:0})",
        "new Array(3)",
    ];
    // NEGATIVE `length` shapes.  `-Infinity` / `-2147483649` coerce to INT_MIN
    // (jsvalue.c:45) and `len + from` at jsarray.c:558 / :582 then overflows
    // `int`: UNDEFINED BEHAVIOUR, and with gcc's wraparound `Ap_lastIndexOf`
    // ends up looping from +2147483646 down to 0.  Those two arguments are
    // therefore excluded for negative lengths only.
    let froms_safe: Vec<&str> = froms
        .iter()
        .copied()
        .filter(|f| *f != "-Infinity" && *f != "-2147483649")
        .collect();
    let neg_shapes: &[&str] = &["({length:-2, 0:1})", "({length:-1, 0:1})"];
    let mut bodies: Vec<String> = Vec::new();
    for s in shapes {
        for n in needles {
            for f in froms {
                bodies.push(format!(
                    "return String(Array.prototype.indexOf.call({s}, {n}, {f})) + '/' + \
                     String(Array.prototype.lastIndexOf.call({s}, {n}, {f}));"
                ));
            }
        }
    }
    for s in neg_shapes {
        for n in needles {
            for f in &froms_safe {
                bodies.push(format!(
                    "return String(Array.prototype.indexOf.call({s}, {n}, {f})) + '/' + \
                     String(Array.prototype.lastIndexOf.call({s}, {n}, {f}));"
                ));
            }
        }
    }
    diff_bodies(&bodies);
    expect_exprs(&[
        ("[1,2,3].indexOf(9)", "OK:num(-1)"),
        ("[1,2,3].lastIndexOf(9)", "OK:num(-1)"),
        ("[].indexOf(undefined)", "OK:num(-1)"),
        ("[].lastIndexOf(undefined)", "OK:num(-1)"),
        ("[NaN].indexOf(NaN)", "OK:num(-1)"),
        ("[NaN].lastIndexOf(NaN)", "OK:num(-1)"),
    ]);
}

/// Rows 527-537 -- the six iteration methods' "callback is not a function"
/// TypeError, plus `Ap_reduce` / `Ap_reduceRight`'s two "no initial value"
/// TypeErrors (the `len == 0` one and the all-holes-scan one).
#[test]
fn t_array_callback_typeerrors() {
    let methods = [
        "every",
        "some",
        "forEach",
        "map",
        "filter",
        "reduce",
        "reduceRight",
    ];
    let bad = [
        "", "1", "0", "'x'", "''", "null", "undefined", "{}", "[]", "true", "false", "/re/",
        "new Number(1)", "Math", "NaN",
    ];
    let mut cases: Vec<(String, String)> = Vec::new();
    for m in methods {
        for b in bad {
            cases.push((
                format!("[1,2,3].{m}({b})"),
                "THREW:TypeError: callback is not a function".to_string(),
            ));
            let tail = if b.is_empty() {
                String::new()
            } else {
                format!(", {b}")
            };
            cases.push((
                format!("Array.prototype.{m}.call([1,2]{tail})"),
                "THREW:TypeError: callback is not a function".to_string(),
            ));
            // the guard runs before js_getlength, so even a hostile `this`
            // cannot pre-empt it
            cases.push((
                format!("Array.prototype.{m}.call(undefined{tail})"),
                "THREW:TypeError: callback is not a function".to_string(),
            ));
        }
    }
    // rows 533 / 536: `len == 0 && !hasinitial` (jsarray.c:756 / :797).  The
    // test is `len == 0`, NOT `len <= 0`.
    for m in ["reduce", "reduceRight"] {
        for s in ["[]", "({length:0})", "({length:NaN})", "({})", "({length:'0'})"] {
            cases.push((
                format!("Array.prototype.{m}.call({s}, function(a,b){{return a}})"),
                "THREW:TypeError: no initial value".to_string(),
            ));
        }
        // rows 534 / 537: no initialValue and every index is a hole
        for s in [
            "new Array(5)",
            "new Array(1)",
            "({length:3})",
            "(function(){ var a=[1,2]; delete a[0]; delete a[1]; return a })()",
        ] {
            cases.push((
                format!("Array.prototype.{m}.call({s}, function(a,b){{return a}})"),
                "THREW:TypeError: no initial value".to_string(),
            ));
        }
        // ... and with an initialValue the same shapes succeed
        cases.push((
            format!("[].{m}(function(a,b){{return a}}, 'seed')"),
            "OK:str#4\"seed\"".to_string(),
        ));
        cases.push((
            format!("new Array(5).{m}(function(a,b){{return a}}, 'seed')"),
            "OK:str#4\"seed\"".to_string(),
        ));
        // undefined counts as an initialValue: hasinitial is `js_gettop(J) >= 3`
        cases.push((
            format!("String([].{m}(function(a,b){{return a}}, undefined))"),
            "OK:str#9\"undefined\"".to_string(),
        ));
    }
    // A NEGATIVE length splits the two: `Ap_reduce` starts at `k = 0`, so
    // neither jsarray.c:756 (`len == 0`) nor jsarray.c:766 (`k == len`) fires and
    // the function pushes nothing at all (jsrun.c:1273 substitutes `undefined`);
    // `Ap_reduceRight` starts at `k = len - 1 < 0` and therefore always reaches
    // the jsarray.c:807 `k < 0` throw (row 537).
    // NOT `{length:-1e30}`: `js_getlength` clamps that to INT_MIN (jsvalue.c:44)
    // and `Ap_reduceRight`'s `k = len - 1` (jsarray.c:795) then overflows `int`
    // -- UNDEFINED BEHAVIOUR, and with gcc's wraparound the hole scan runs from
    // +2147483647 down to 0.
    for s in ["({length:-1})", "({length:-3})", "({length:-2})"] {
        cases.push((
            format!("Array.prototype.reduce.call({s}, function(a,b){{return a}})"),
            "OK:undefined(undefined)".to_string(),
        ));
        cases.push((
            format!("Array.prototype.reduceRight.call({s}, function(a,b){{return a}})"),
            "THREW:TypeError: no initial value".to_string(),
        ));
    }
    let refs: Vec<(&str, &str)> = cases
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    expect_exprs(&refs);
    // and the callable case, so the guard is not merely always-throwing
    diff_bodies(&[
        body("[1,2,3].every(function(v){ return v < 3 })"),
        body("[1,2,3].some(function(v){ return v === 2 })"),
        body("(function(){ var s=''; [1,2,3].forEach(function(v){ s+=v }); return s })()"),
        body("A([1,2,3].map(function(v){ return v*2 }))"),
        body("A([1,2,3].filter(function(v){ return v!==2 }))"),
        body("[1,2,3].reduce(function(a,b){ return a+b })"),
        body("[1,2,3].reduceRight(function(a,b){ return a+'/'+b })"),
        body("new Array(4).reduce(function(a,b){return a}, 1)"),
    ]);
}

/// Rows 538 / 539 -- `A_isArray`'s two `false` exits.
#[test]
fn t_array_isarray() {
    let vals: &[&str] = &[
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
        "'[]'",
        "({})",
        "[]",
        "[1,2]",
        "new Array(3)",
        "(function(){})",
        "Array",
        "Array.prototype",
        "/re/g",
        "new String('s')",
        "new Number(2)",
        "new Boolean(true)",
        "new Date(0)",
        "new Error('e')",
        "Math",
        "JSON",
        "this",
        "Object.create([])",
        "Object.create(Array.prototype)",
        "arguments",
        "(function(){ return arguments })(1,2)",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for v in vals {
        bodies.push(format!("return String(Array.isArray({v}));"));
        bodies.push(format!("return String(Array.isArray.call(null, {v}));"));
    }
    bodies.push("return String(Array.isArray());".into());
    diff_bodies(&bodies);
}

/* =========================================================================
 *  jsobject.c
 * ========================================================================= */

/// Rows 540 / 541 / 542 -- `jsB_new_Object` / `jsB_Object`.  `undefined` and
/// `null` do NOT reach `js_toobject`, so they produce a fresh empty object
/// instead of the jsvalue.c:401/402 TypeError.
#[test]
fn t_object_constructor() {
    let vals: &[&str] = &[
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
        "Object.create(null)",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for v in vals {
        for form in ["new Object", "Object"] {
            bodies.push(format!(
                "var a = {v}; var o = {form}({v}); \
                 return Object.prototype.toString.call(o) + '/same=' + (o === a) + \
                 '/proto=' + (Object.getPrototypeOf(o) === Object.prototype) + \
                 '/val=' + T(o.valueOf());"
            ));
        }
    }
    bodies.push("return Object.prototype.toString.call(new Object());".into());
    bodies.push("return Object.prototype.toString.call(Object());".into());
    bodies.push("return Object.prototype.toString.call(new Object(1,2));".into());
    diff_bodies(&bodies);
}

/// Rows 543 / 544 / 545 -- `Op_toString`.  The two guards for `undefined` /
/// `null` come BEFORE `js_toobject`, and the `switch (self->type)` that follows
/// has no `default` -- but `enum js_Class` has exactly the 16 members the switch
/// enumerates, so nothing can fall through.  All 16 are driven here, including
/// the three (`JS_CSCRIPT`, `JS_CITERATOR`, `JS_CUSERDATA`) that no JS
/// expression can produce.
#[test]
fn t_object_tostring_all_classes() {
    // JS-reachable classes, plus rows 543 / 544
    expect_exprs(&[
        (
            "Object.prototype.toString.call(undefined)",
            "OK:str#18\"[object Undefined]\"",
        ),
        ("Object.prototype.toString.call(null)", "OK:str#13\"[object Null]\""),
        ("Object.prototype.toString.apply(undefined, [])", "OK:str#18\"[object Undefined]\""),
        ("Object.prototype.toString.apply(null, [])", "OK:str#13\"[object Null]\""),
        ("Object.prototype.toString.call()", "OK:str#18\"[object Undefined]\""),
        ("Object.prototype.toString.call({})", "OK:str#15\"[object Object]\""),
        ("Object.prototype.toString.call([])", "OK:str#14\"[object Array]\""),
        (
            "Object.prototype.toString.call(function(){})",
            "OK:str#17\"[object Function]\"",
        ),
        ("Object.prototype.toString.call(print)", "OK:str#17\"[object Function]\""),
        ("Object.prototype.toString.call(new Error('e'))", "OK:str#14\"[object Error]\""),
        (
            "Object.prototype.toString.call(new Boolean(true))",
            "OK:str#16\"[object Boolean]\"",
        ),
        ("Object.prototype.toString.call(new Number(1))", "OK:str#15\"[object Number]\""),
        ("Object.prototype.toString.call(new String('s'))", "OK:str#15\"[object String]\""),
        ("Object.prototype.toString.call(/re/g)", "OK:str#15\"[object RegExp]\""),
        ("Object.prototype.toString.call(new Date(0))", "OK:str#13\"[object Date]\""),
        ("Object.prototype.toString.call(Math)", "OK:str#13\"[object Math]\""),
        ("Object.prototype.toString.call(JSON)", "OK:str#13\"[object JSON]\""),
        (
            "(function(){ return Object.prototype.toString.call(arguments) })(1,2)",
            "OK:str#18\"[object Arguments]\"",
        ),
        // primitives are boxed by js_toobject first (jsvalue.c:404-408)
        ("Object.prototype.toString.call(1)", "OK:str#15\"[object Number]\""),
        ("Object.prototype.toString.call('s')", "OK:str#15\"[object String]\""),
        ("Object.prototype.toString.call(true)", "OK:str#16\"[object Boolean]\""),
    ]);
    // the three classes that need the C API: JS_CSCRIPT (js_loadstring),
    // JS_CITERATOR (js_pushiterator), JS_CUSERDATA (js_newuserdata)
    for kind in ["script", "iterator", "userdata", "ccfunction", "cconstructor"] {
        probe_state(&format!("Op_toString on {kind}"), 0, move || {
            job!(|l, j| {
                l.js_getglobal(j, cn!("Object"));
                l.js_getproperty(j, -1, cn!("prototype"));
                l.js_getproperty(j, -1, cn!("toString"));
                match kind {
                    "script" => l.js_loadstring(j, FILENAME, cn!("1+1")),
                    "iterator" => {
                        let rc = push_expr(l, j, "({a:1,b:2})");
                        assert_eq!(rc, 0);
                        l.js_pushiterator(j, -1, 1);
                        l.js_remove(j, -2);
                    }
                    "userdata" => {
                        l.js_pushundefined(j); // no prototype
                        l.js_newuserdata(j, cn!("udtag"), std::ptr::null_mut(), None);
                    }
                    "ccfunction" => l.js_getglobal(j, cn!("print")),
                    _ => l.js_getglobal(j, cn!("Object")),
                }
                let ty = from_c(l.js_typeof(j, -1));
                let rc = l.js_pcall(j, 0);
                let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
                format!("recv_ty={ty} rc={rc} v={v}")
            })
        });
    }
}

/// Rows 546-554 -- `Op_hasOwnProperty` / `Op_isPrototypeOf` /
/// `Op_propertyIsEnumerable`: the shared `js_toobject(J, 0)` TypeError, the
/// `JS_CSTRING` and simple-`JS_CARRAY` index bounds checks, and every `false`
/// exit.
#[test]
fn t_object_own_property_predicates() {
    // row 546 / 550 / 553: js_toobject(J, 0) on a non-coercible `this`
    let mut cases: Vec<(String, String)> = Vec::new();
    for m in ["hasOwnProperty", "isPrototypeOf", "propertyIsEnumerable"] {
        for (recv, want) in [
            ("undefined", "cannot convert undefined to object"),
            ("null", "cannot convert null to object"),
        ] {
            for form in call_spellings(recv, &format!("Object.prototype.{m}"), &["'x'"]) {
                cases.push((form, format!("THREW:TypeError: {want}")));
            }
            cases.push((
                format!("Object.prototype.{m}.call({recv})"),
                format!("THREW:TypeError: {want}"),
            ));
        }
    }
    let refs: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    expect_exprs(&refs);

    // rows 547 / 548 / 549: the two index fast paths and their bounds checks
    let names: &[&str] = &[
        "'0'", "'1'", "'2'", "'3'", "'4'", "'-1'", "'-0'", "'1.5'", "'01'", "''", "'length'",
        "'x'", "'toString'", "0", "1", "3", "-1", "1.5", "'4294967295'", "'4294967296'",
        "'2147483647'", "'2147483648'", "'99999999999999999999'", "'+1'", "' 1'", "'1 '",
    ];
    let recvs: &[&str] = &[
        "new String('abc')",
        "new String('')",
        "new String('\\u00e9x')",
        "[1,2,3]",
        "[]",
        "(function(){ var a=[1,2,3]; a[9]=1; return a })()",
        "(function(){ var a=[1,2,3]; delete a[1]; return a })()",
        "new Array(4)",
        "({})",
        "({0:'a', length:1})",
        "'abc'",
        "5",
        "true",
        "(function(){})",
        "/re/",
        "Object.create(null)",
        "Object.create({inherited:1})",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for r in recvs {
        for n in names {
            bodies.push(format!(
                "return String(Object.prototype.hasOwnProperty.call({r}, {n})) + '/' + \
                 String(Object.prototype.propertyIsEnumerable.call({r}, {n}));"
            ));
        }
    }
    diff_bodies(&bodies);

    // row 554: present but JS_DONTENUM
    expect_exprs(&[
        ("({a:1}).propertyIsEnumerable('a')", "OK:boolean(true)"),
        ("({}).propertyIsEnumerable('a')", "OK:boolean(false)"),
        ("Object.prototype.propertyIsEnumerable('toString')", "OK:boolean(false)"),
        ("Array.prototype.propertyIsEnumerable('join')", "OK:boolean(false)"),
        // The global object is created with NO prototype (jsstate.c:230), so it
        // inherits nothing from Object.prototype.
        (
            "this.propertyIsEnumerable('Array')",
            "THREW:TypeError: undefined is not callable",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o,'a',{value:1,enumerable:false}); \
             return o.propertyIsEnumerable('a') })()",
            "OK:boolean(false)",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o,'a',{value:1,enumerable:true}); \
             return o.propertyIsEnumerable('a') })()",
            "OK:boolean(true)",
        ),
        // rows 551 / 552: non-object argument, and a chain walked to NULL
        ("({}).isPrototypeOf(1)", "OK:boolean(false)"),
        ("({}).isPrototypeOf('x')", "OK:boolean(false)"),
        ("({}).isPrototypeOf(null)", "OK:boolean(false)"),
        ("({}).isPrototypeOf(undefined)", "OK:boolean(false)"),
        ("({}).isPrototypeOf(true)", "OK:boolean(false)"),
        ("({}).isPrototypeOf({})", "OK:boolean(false)"),
        ("(function(){ var o={}; return o.isPrototypeOf(o) })()", "OK:boolean(false)"),
        ("Object.prototype.isPrototypeOf({})", "OK:boolean(true)"),
        ("Object.prototype.isPrototypeOf([])", "OK:boolean(true)"),
        ("Array.prototype.isPrototypeOf([])", "OK:boolean(true)"),
        ("Array.prototype.isPrototypeOf({})", "OK:boolean(false)"),
        (
            "Object.prototype.isPrototypeOf(Object.create(null))",
            "OK:boolean(false)",
        ),
        (
            "(function(){ var a={}, b=Object.create(a), c=Object.create(b); \
             return a.isPrototypeOf(c) })()",
            "OK:boolean(true)",
        ),
        ("({}).isPrototypeOf()", "OK:boolean(false)"),
    ]);
}

/// Rows 555-560 / 578-589 -- every `js_isobject(J, 1)` gate in jsobject.c and
/// the `false`/`undefined`/`null` exits behind them.
#[test]
fn t_object_static_typeerrors() {
    let statics = [
        ("getPrototypeOf", 1),
        ("getOwnPropertyDescriptor", 2),
        ("getOwnPropertyNames", 1),
        ("defineProperty", 3),
        ("defineProperties", 2),
        ("seal", 1),
        ("freeze", 1),
        ("preventExtensions", 1),
        ("isSealed", 1),
        ("isFrozen", 1),
        ("isExtensible", 1),
        ("keys", 1),
    ];
    let nonobjects = [
        "undefined",
        "null",
        "0",
        "-1",
        "NaN",
        "''",
        "'x'",
        "true",
        "false",
    ];
    let mut cases: Vec<(String, String)> = Vec::new();
    for (m, argc) in statics {
        for v in nonobjects {
            let extra = match argc {
                1 => String::new(),
                2 => ", {}".to_string(),
                _ => ", 'x', {}".to_string(),
            };
            cases.push((
                format!("Object.{m}({v}{extra})"),
                "THREW:TypeError: not an object".to_string(),
            ));
            cases.push((
                format!("Object.{m}.call(null, {v}{extra})"),
                "THREW:TypeError: not an object".to_string(),
            ));
        }
        // no arguments at all -> the padded `undefined`
        cases.push((
            format!("Object.{m}()"),
            "THREW:TypeError: not an object".to_string(),
        ));
    }
    // row 575: Object.create wants an object OR null, with its own message
    for v in ["undefined", "0", "-1", "NaN", "''", "'x'", "true", "false"] {
        cases.push((
            format!("Object.create({v})"),
            "THREW:TypeError: not an object or null".to_string(),
        ));
    }
    cases.push((
        "Object.create()".to_string(),
        "THREW:TypeError: not an object or null".to_string(),
    ));
    let refs: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    expect_exprs(&refs);

    expect_exprs(&[
        // row 556: obj->prototype == NULL
        ("Object.getPrototypeOf(Object.create(null))", "OK:null"),
        (
            "Object.getPrototypeOf({}) === Object.prototype",
            "OK:boolean(true)",
        ),
        ("Object.getPrototypeOf([]) === Array.prototype", "OK:boolean(true)"),
        // row 558: jsV_getproperty returned NULL
        ("Object.getOwnPropertyDescriptor({}, 'x')", "OK:undefined(undefined)"),
        (
            "Object.getOwnPropertyDescriptor({a:1}, 'b')",
            "OK:undefined(undefined)",
        ),
        // the TODO at jsobject.c:129: built-in string / array properties are
        // NOT reported even though hasOwnProperty says they exist
        (
            "Object.getOwnPropertyDescriptor(new String('ab'), '0')",
            "OK:undefined(undefined)",
        ),
        (
            "Object.getOwnPropertyDescriptor(new String('ab'), 'length')",
            "OK:undefined(undefined)",
        ),
        ("Object.getOwnPropertyDescriptor([1,2], '0')", "OK:undefined(undefined)"),
        ("Object.getOwnPropertyDescriptor([1,2], 'length')", "OK:undefined(undefined)"),
        ("Object.getOwnPropertyDescriptor(/re/, 'source')", "OK:undefined(undefined)"),
        // row 560: obj->properties->level == 0
        ("A(Object.getOwnPropertyNames({}))", "OK:str#4\"[]#0\""),
        ("A(Object.getOwnPropertyNames(Object.create(null)))", "OK:str#4\"[]#0\""),
        ("A(Object.keys({}))", "OK:str#4\"[]#0\""),
        ("A(Object.keys(Object.create(null)))", "OK:str#4\"[]#0\""),
        (
            "Object.getOwnPropertyNames(function(){}).sort().join('|')",
            "OK:str#16\"length|prototype\"",
        ),
    ]);
    // and the class-specific synthetic names / keys
    diff_bodies(&[
        body("Object.getOwnPropertyNames([]).join('|')"),
        body("Object.getOwnPropertyNames([1,2,3]).join('|')"),
        body("Object.getOwnPropertyNames(new Array(3)).join('|')"),
        body("Object.getOwnPropertyNames((function(){var a=[1];a[5]=1;return a})()).join('|')"),
        body("Object.getOwnPropertyNames(new String('abc')).join('|')"),
        body("Object.getOwnPropertyNames(new String('')).join('|')"),
        body("Object.getOwnPropertyNames(/re/g).join('|')"),
        body("Object.getOwnPropertyNames({a:1,b:2,c:3}).sort().join('|')"),
        body("Object.keys([1,2,3]).join('|')"),
        body("Object.keys(new String('abc')).join('|')"),
        body("Object.keys(new Array(3)).join('|')"),
        body("Object.keys({a:1,b:2}).sort().join('|')"),
        body("Object.keys(/re/).join('|')"),
        body("Object.keys(Object.defineProperty({},'h',{value:1,enumerable:false})).join('|')"),
    ]);
}

/// Rows 561-567 / 568 / 569 -- `ToPropertyDescriptor` and `O_defineProperty`.
/// An ABSENT `writable` / `enumerable` / `configurable` means read-only /
/// non-enumerable / non-configurable (jsobject.c:252-254), and `get`/`set`
/// combined with `writable`/`value` is a TypeError.
#[test]
fn t_object_defineproperty_descriptor() {
    expect_exprs(&[
        // rows 561-563: every attribute defaults to "off"
        (
            "(function(){ var o={}; Object.defineProperty(o,'x',{value:1}); \
             var d=Object.getOwnPropertyDescriptor(o,'x'); \
             return d.value+'/'+d.writable+'/'+d.enumerable+'/'+d.configurable })()",
            "OK:str#19\"1/false/false/false\"",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o,'x',\
             {value:1,writable:true,enumerable:true,configurable:true}); \
             var d=Object.getOwnPropertyDescriptor(o,'x'); \
             return d.value+'/'+d.writable+'/'+d.enumerable+'/'+d.configurable })()",
            "OK:str#16\"1/true/true/true\"",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o,'x',{writable:true}); \
             var d=Object.getOwnPropertyDescriptor(o,'x'); \
             return T(d.value)+'/'+d.writable+'/'+d.enumerable+'/'+d.configurable })()",
            "OK:str#26\"undefined/true/false/false\"",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o,'x',{enumerable:true}); \
             var d=Object.getOwnPropertyDescriptor(o,'x'); \
             return d.writable+'/'+d.enumerable+'/'+d.configurable })()",
            "OK:str#16\"false/true/false\"",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o,'x',{configurable:true}); \
             var d=Object.getOwnPropertyDescriptor(o,'x'); \
             return d.writable+'/'+d.enumerable+'/'+d.configurable })()",
            "OK:str#16\"false/false/true\"",
        ),
        // `writable` is read with js_toboolean, so any falsy value counts as
        // "present but false"
        (
            "(function(){ var o={}; Object.defineProperty(o,'x',{value:1,writable:0}); \
             return Object.getOwnPropertyDescriptor(o,'x').writable })()",
            "OK:boolean(false)",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o,'x',{value:1,writable:'no'}); \
             return Object.getOwnPropertyDescriptor(o,'x').writable })()",
            "OK:boolean(true)",
        ),
        // row 564: `get` together with `writable` / `value`
        (
            "Object.defineProperty({},'x',{get:function(){},value:1})",
            "THREW:TypeError: value/writable and get/set attributes are exclusive",
        ),
        (
            "Object.defineProperty({},'x',{get:function(){},writable:true})",
            "THREW:TypeError: value/writable and get/set attributes are exclusive",
        ),
        (
            "Object.defineProperty({},'x',{get:function(){},writable:false})",
            "THREW:TypeError: value/writable and get/set attributes are exclusive",
        ),
        (
            "Object.defineProperty({},'x',{get:undefined,value:1})",
            "THREW:TypeError: value/writable and get/set attributes are exclusive",
        ),
        (
            "Object.defineProperty({},'x',{get:1,value:1})",
            "THREW:TypeError: value/writable and get/set attributes are exclusive",
        ),
        (
            "Object.defineProperty({},'x',{value:undefined,get:function(){}})",
            "THREW:TypeError: value/writable and get/set attributes are exclusive",
        ),
        // row 566: `set` together with `writable` / `value`
        (
            "Object.defineProperty({},'x',{set:function(v){},value:1})",
            "THREW:TypeError: value/writable and get/set attributes are exclusive",
        ),
        (
            "Object.defineProperty({},'x',{set:function(v){},writable:true})",
            "THREW:TypeError: value/writable and get/set attributes are exclusive",
        ),
        (
            "Object.defineProperty({},'x',{set:undefined,writable:false})",
            "THREW:TypeError: value/writable and get/set attributes are exclusive",
        ),
        // rows 565 / 567: only one accessor supplied
        (
            "(function(){ var o={}; Object.defineProperty(o,'x',\
             {get:function(){ return 7 }}); return o.x + '/' + (o.x = 9) + '/' + o.x })()",
            "OK:str#5\"7/9/7\"",
        ),
        (
            "(function(){ var log=''; var o={}; Object.defineProperty(o,'x',\
             {set:function(v){ log += v }}); o.x=1; o.x=2; return T(o.x)+'/'+log })()",
            "OK:str#12\"undefined/12\"",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o,'x',{}); \
             var d=Object.getOwnPropertyDescriptor(o,'x'); \
             return T(d.value)+'/'+d.writable+'/'+d.enumerable+'/'+d.configurable })()",
            "OK:str#27\"undefined/false/false/false\"",
        ),
        // rows 568 / 569
        (
            "Object.defineProperty(1,'x',{value:1})",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.defineProperty({},'x',1)",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.defineProperty({},'x','str')",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.defineProperty({},'x',null)",
            "THREW:TypeError: not an object",
        ),
        ("Object.defineProperty({},'x')", "THREW:TypeError: not an object"),
    ]);
    // property-name coercion happens through js_tostring(J, 2), so a throwing
    // toString propagates out of O_defineProperty
    expect_exprs(&[
        (
            "Object.defineProperty({}, {toString:function(){ throw new Error('name!') }}, {value:1})",
            "THREW:Error: name!",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o, 5, {value:'v'}); return o[5] })()",
            "OK:str#1\"v\"",
        ),
        (
            "(function(){ var o={}; Object.defineProperty(o, undefined, {value:'v'}); \
             return o['undefined'] })()",
            "OK:str#1\"v\"",
        ),
    ]);
}

/// Rows 570-574 / 576 / 577 -- `O_defineProperties_walk`,
/// `O_defineProperties_imp`, `O_defineProperties` and `O_create`.
#[test]
fn t_object_defineproperties() {
    expect_exprs(&[
        // row 570: an enumerable own property of `props` whose value is not an
        // object
        (
            "Object.defineProperties({}, {x: 1})",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.defineProperties({}, {x: 'str'})",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.defineProperties({}, {x: null})",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.defineProperties({}, {x: undefined})",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.defineProperties({}, {a: {value:1}, b: 2})",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.create({}, {x: 1})",
            "THREW:TypeError: not an object",
        ),
        // a NON-enumerable property of `props` is skipped, so its non-object
        // value never trips the walk
        (
            "(function(){ var p={}; Object.defineProperty(p,'x',{value:1,enumerable:false}); \
             var o=Object.defineProperties({}, p); return 'x' in o })()",
            "OK:boolean(false)",
        ),
        // row 571: the properties bag itself is not an object
        (
            "Object.defineProperties({}, 1)",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.defineProperties({}, 'x')",
            "THREW:TypeError: not an object",
        ),
        (
            "Object.defineProperties({}, null)",
            "THREW:TypeError: not an object",
        ),
        ("Object.defineProperties({})", "THREW:TypeError: not an object"),
        ("Object.create({}, 1)", "THREW:TypeError: not an object"),
        ("Object.create(null, 'x')", "THREW:TypeError: not an object"),
        // row 572: props has no own properties -> silently does nothing
        (
            "(function(){ var o=Object.defineProperties({a:1}, {}); \
             return Object.keys(o).join('|') })()",
            "OK:str#1\"a\"",
        ),
        (
            "(function(){ var o=Object.defineProperties({}, Object.create({x:{value:1}})); \
             return 'x' in o })()",
            "OK:boolean(false)",
        ),
        // row 574: the target is not an object
        (
            "Object.defineProperties(1, {x:{value:1}})",
            "THREW:TypeError: not an object",
        ),
        // row 576: Object.create(null)
        (
            "(function(){ var o=Object.create(null); \
             return String(Object.getPrototypeOf(o)) + '/' + ('toString' in o) })()",
            "OK:str#10\"null/false\"",
        ),
        // row 577: an undefined second argument skips O_defineProperties_imp
        (
            "(function(){ var o=Object.create({p:1}, undefined); \
             return o.p + '/' + Object.keys(o).length })()",
            "OK:str#3\"1/0\"",
        ),
        (
            "(function(){ var o=Object.create(null, undefined); \
             return String(Object.getPrototypeOf(o)) })()",
            "OK:str#4\"null\"",
        ),
        (
            "(function(){ var o=Object.create({}, {x:{value:7,enumerable:true}}); \
             return o.x + '/' + Object.keys(o).join('|') })()",
            "OK:str#3\"7/x\"",
        ),
    ]);
    // row 573: `js_hasproperty(J, 2, name)` false for a name the walk had
    // already collected.  The names are collected in tree (alphabetical) order,
    // so a `writable` getter on descriptor "a" can delete `props.b` before the
    // loop reaches it.
    expect_exprs(&[
        (
            "(function(){ \
               var props = {}; \
               var descA = {value:1}; \
               Object.defineProperty(descA, 'writable', \
                 {get: function(){ delete props.b; return true }, enumerable:true}); \
               props.a = descA; \
               props.b = {value:2}; \
               var o = Object.defineProperties({}, props); \
               return ('a' in o) + '/' + ('b' in o) + '/' + ('b' in props) })()",
            "OK:str#16\"true/false/false\"",
        ),
        (
            "(function(){ \
               var props = {}; \
               var descA = {value:1}; \
               Object.defineProperty(descA, 'writable', \
                 {get: function(){ delete props.c; delete props.d; return true }, \
                  enumerable:true}); \
               props.a = descA; props.b = {value:2}; props.c = {value:3}; \
               props.d = {value:4}; \
               var o = Object.defineProperties({}, props); \
               return ('a' in o)+'/'+('b' in o)+'/'+('c' in o)+'/'+('d' in o) })()",
            "OK:str#21\"true/true/false/false\"",
        ),
    ]);
}

/// Rows 579-589 -- `O_preventExtensions` / `O_isExtensible` / `O_seal` /
/// `O_isSealed` / `O_freeze` / `O_isFrozen` and the two recursive walks'
/// `return 0` exits.
#[test]
fn t_object_seal_freeze_walks() {
    let shapes: &[&str] = &[
        "({})",
        "({a:1})",
        "({a:1,b:2})",
        "({a:1,b:2,c:3,d:4,e:5})",
        "[]",
        "[1,2,3]",
        "(function(){})",
        "new String('ab')",
        "new Number(1)",
        "/re/",
        "Object.create(null)",
        // one property READONLY but configurable -> row 587 only
        "Object.defineProperty({}, 'a', {value:1, writable:false, configurable:true})",
        // one property DONTCONF but writable -> row 586 only
        "Object.defineProperty({}, 'a', {value:1, writable:true, configurable:false})",
        // mixed, so the walk has to recurse into both subtrees
        "(function(){ var o={a:1,b:2,c:3}; \
          Object.defineProperty(o,'b',{value:2,writable:false,configurable:false}); \
          return o })()",
    ];
    let ops: &[&str] = &[
        "",
        "Object.preventExtensions(o);",
        "Object.seal(o);",
        "Object.freeze(o);",
        "Object.seal(o); Object.preventExtensions(o);",
        "Object.freeze(o); Object.seal(o);",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in shapes {
        for op in ops {
            bodies.push(format!(
                "var o = {s}; {op} \
                 var r = Object.isExtensible(o) + '/' + Object.isSealed(o) + '/' + \
                         Object.isFrozen(o); \
                 var t = ''; try {{ o.zz = 1; t = 'added=' + ('zz' in o) }} \
                 catch (e) {{ t = 'add:' + X(e) }} \
                 var u = ''; try {{ o.a = 99; u = 'a=' + T(o.a) }} \
                 catch (e) {{ u = 'set:' + X(e) }} \
                 var v = ''; try {{ v = 'del=' + String(delete o.a) }} \
                 catch (e) {{ v = 'del:' + X(e) }} \
                 return r + ' ' + t + ' ' + u + ' ' + v;"
            ));
            bodies.push(format!(
                "var o = {s}; {op} return (Object.seal(o) === o) + '/' + \
                 (Object.freeze(o) === o) + '/' + (Object.preventExtensions(o) === o);"
            ));
        }
    }
    diff_bodies(&bodies);
    expect_exprs(&[
        // row 584: still extensible -> isSealed is false without walking
        ("Object.isSealed({})", "OK:boolean(false)"),
        ("Object.isSealed({a:1})", "OK:boolean(false)"),
        ("Object.isSealed(Object.preventExtensions({}))", "OK:boolean(true)"),
        // row 582: a property without JS_DONTCONF
        (
            "Object.isSealed(Object.preventExtensions({a:1}))",
            "OK:boolean(false)",
        ),
        ("Object.isSealed(Object.seal({a:1}))", "OK:boolean(true)"),
        // rows 586 / 587 / 589
        ("Object.isFrozen({})", "OK:boolean(false)"),
        ("Object.isFrozen(Object.preventExtensions({}))", "OK:boolean(true)"),
        ("Object.isFrozen(Object.seal({a:1}))", "OK:boolean(false)"),
        ("Object.isFrozen(Object.freeze({a:1}))", "OK:boolean(true)"),
        (
            "Object.isFrozen(Object.preventExtensions(\
             Object.defineProperty({},'a',{value:1,writable:false,configurable:true})))",
            "OK:boolean(false)",
        ),
        (
            "Object.isFrozen(Object.preventExtensions(\
             Object.defineProperty({},'a',{value:1,writable:true,configurable:false})))",
            "OK:boolean(false)",
        ),
        ("Object.isExtensible({})", "OK:boolean(true)"),
        ("Object.isExtensible(Object.preventExtensions({}))", "OK:boolean(false)"),
    ]);
}

/* =========================================================================
 *  jsstring.c
 * ========================================================================= */

/// The 17 `checkstring` (jsstring.c:13-18) call sites, with a representative
/// argument list for each.  `Sp_concat` MUST be given at least one argument,
/// because jsstring.c:151 returns before `checkstring` when `top == 1`, and
/// `String.prototype.split` MUST be given a defined separator, because
/// jsstring.c:820 handles `undefined` before either split helper runs.
const CHECKSTRING_SITES: &[(&str, &str, &str)] = &[
    ("jsstring.c:122 Sp_charAt", "charAt", "0"),
    ("jsstring.c:135 Sp_charCodeAt", "charCodeAt", "0"),
    ("jsstring.c:154 Sp_concat", "concat", "'a'"),
    ("jsstring.c:183 Sp_indexOf", "indexOf", "'a'"),
    ("jsstring.c:202 Sp_lastIndexOf", "lastIndexOf", "'a'"),
    ("jsstring.c:219 Sp_localeCompare", "localeCompare", "'a'"),
    ("jsstring.c:283 Sp_slice", "slice", "0, 1"),
    ("jsstring.c:304 Sp_substring", "substring", "0, 1"),
    ("jsstring.c:322 Sp_toLowerCase", "toLowerCase", ""),
    ("jsstring.c:322 Sp_toLowerCase/locale", "toLocaleLowerCase", ""),
    ("jsstring.c:372 Sp_toUpperCase", "toUpperCase", ""),
    ("jsstring.c:372 Sp_toUpperCase/locale", "toLocaleUpperCase", ""),
    ("jsstring.c:434 Sp_trim", "trim", ""),
    ("jsstring.c:477 Sp_match", "match", "/a/"),
    ("jsstring.c:526 Sp_search", "search", "/a/"),
    ("jsstring.c:551 Sp_replace_regexp", "replace", "/a/, 'b'"),
    ("jsstring.c:652 Sp_replace_string", "replace", "'a', 'b'"),
    ("jsstring.c:725 Sp_split_regexp", "split", "/a/"),
    ("jsstring.c:780 Sp_split_string", "split", "'a'"),
];

/// Rows 591 / 597 / 599 / 602 / 606 / 609 / 612 / 617 / 623 / 627 / 630 / 633 /
/// 639 / 645 / 649 / 658 / 663 / 672 -- every `checkstring` call site, driven
/// with `this` = `null` and `this` = `undefined` through `.call` and `.apply`.
#[test]
fn t_string_checkstring_all_sites() {
    let mut cases: Vec<(String, String)> = Vec::new();
    for (_site, m, args) in CHECKSTRING_SITES {
        for recv in ["undefined", "null"] {
            let arglist: Vec<&str> = if args.is_empty() {
                vec![]
            } else {
                args.split(", ").collect()
            };
            for form in call_spellings(recv, &format!("String.prototype.{m}"), &arglist) {
                cases.push((
                    form,
                    "THREW:TypeError: string function called on null or undefined".to_string(),
                ));
            }
            // no arguments at all: Fp_call/Fp_apply pad to their declared
            // minimum, so `this` still arrives as `undefined`
            cases.push((
                format!("String.prototype.{m}.call({recv})"),
                "THREW:TypeError: string function called on null or undefined".to_string(),
            ));
        }
        // and the zero-argument .call(), whose padded `this` is undefined
        cases.push((
            format!("String.prototype.{m}.call()"),
            "THREW:TypeError: string function called on null or undefined".to_string(),
        ));
    }
    // ... except split/concat, whose early-outs run BEFORE checkstring
    let refs: Vec<(&str, &str)> = cases
        .iter()
        .filter(|(e, _)| {
            !(e.contains("split.call()")
                || e.contains("concat.call()")
                || e.contains("split.call(undefined)")
                || e.contains("split.call(null)")
                || e.contains("concat.call(undefined)")
                || e.contains("concat.call(null)"))
        })
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    expect_exprs(&refs);
    // the two early-outs themselves: row 601 (Sp_concat top == 1) and row 677
    // (Sp_split with an undefined separator).
    expect_exprs(cases![
        ("String.prototype.concat.call()", "OK:undefined(undefined)"),
        ("String.prototype.concat.call(undefined)", "OK:undefined(undefined)"),
        ("String.prototype.concat.call(null)", "OK:undefined(undefined)"),
        ("'abc'.concat()", "OK:undefined(undefined)"),
        ("A(String.prototype.split.call(undefined))", ok_str("[str#9\"undefined\"]#1")),
        ("A(String.prototype.split.call(null))", ok_str("[str#4\"null\"]#1")),
        ("A(String.prototype.split.call())", ok_str("[str#9\"undefined\"]#1")),
        ("A('a,b'.split())", ok_str("[str#3\"a,b\"]#1")),
        ("A('a,b'.split(undefined, 0))", ok_str("[str#3\"a,b\"]#1")),
    ]);
    // every method with a NON-string but coercible `this`: checkstring falls
    // through to js_tostring, so the receiver is stringified first.
    let recvs: &[&str] = &[
        "0",
        "-0",
        "1.5",
        "NaN",
        "Infinity",
        "true",
        "false",
        "({})",
        "([1,2])",
        "(function(){})",
        "/ab/g",
        "new String('aXa')",
        "new Number(1)",
        "new Boolean(false)",
        "Math",
        "({toString: function(){ return 'aXa' }})",
        "({valueOf: function(){ return 'aVa' }})",
        "({toString: function(){ return 5 }})",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for (_site, m, args) in CHECKSTRING_SITES {
        for r in recvs {
            let all = if args.is_empty() {
                String::new()
            } else {
                format!(", {args}")
            };
            bodies.push(format!(
                "return T(String.prototype.{m}.call({r}{all}));"
            ));
        }
    }
    diff_bodies(&bodies);
    // ... and a receiver whose toString throws, so checkstring's js_tostring
    // propagates
    let mut cases2: Vec<(String, String)> = Vec::new();
    for (_site, m, args) in CHECKSTRING_SITES {
        let all = if args.is_empty() {
            String::new()
        } else {
            format!(", {args}")
        };
        cases2.push((
            format!(
                "String.prototype.{m}.call(\
                 {{toString: function(){{ throw new RangeError('recv!') }}}}{all})"
            ),
            "THREW:RangeError: recv!".to_string(),
        ));
    }
    let refs2: Vec<(&str, &str)> = cases2.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    expect_exprs(&refs2);
}

/// Rows 593-596 -- `Sp_toString` / `Sp_valueOf`.  These two do NOT use
/// `checkstring`: they call `js_toobject(J, 0)` (so the message is the
/// jsvalue.c:401/402 one, NOT checkstring's) and then require `JS_CSTRING`.
#[test]
fn t_string_tostring_valueof() {
    let mut cases: Vec<(String, String)> = Vec::new();
    for m in ["toString", "valueOf"] {
        for (recv, want) in [
            ("undefined", "cannot convert undefined to object"),
            ("null", "cannot convert null to object"),
        ] {
            for form in call_spellings(recv, &format!("String.prototype.{m}"), &[]) {
                cases.push((form, format!("THREW:TypeError: {want}")));
            }
        }
        // rows 594 / 596: coercible but not a JS_CSTRING
        for recv in [
            "1",
            "0",
            "true",
            "({})",
            "[]",
            "(function(){})",
            "/re/",
            "new Number(1)",
            "new Boolean(true)",
            "new Date(0)",
            "new Error('e')",
            "Math",
            "JSON",
        ] {
            cases.push((
                format!("String.prototype.{m}.call({recv})"),
                "THREW:TypeError: not a string".to_string(),
            ));
        }
        // ... and the accepting cases: a primitive string is boxed by
        // js_toobject into a fresh JS_CSTRING
        cases.push((format!("String.prototype.{m}.call('ab')"), ok_str("ab")));
        cases.push((
            format!("String.prototype.{m}.call(new String('ab'))"),
            ok_str("ab"),
        ));
        cases.push((format!("String.prototype.{m}.call('')"), ok_str("")));
        cases.push((
            format!("String.prototype.{m}.call(String.prototype)"),
            ok_str(""),
        ));
    }
    let refs: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    expect_exprs(&refs);
}

/// Rows 592 / 598 / 600 -- `js_runeat` returning `EOF` and the two callers'
/// `""` / `NaN` substitutions.
#[test]
fn t_string_charat_bounds() {
    let strs: &[&str] = &[
        "''",
        "'a'",
        "'abc'",
        "'\\u00e9'",
        "'a\\u00e9b'",
        "'\\u20ac'",
        "String.fromCharCode(65535)",
        "String.fromCharCode(65536)",
        "String.fromCharCode(65537)",
        "('a' + String.fromCharCode(65536) + 'b')",
        "String.fromCharCode(1114111)",
        "String.fromCharCode(128512)",
        "('a' + String.fromCharCode(128512) + 'b')",
        "(String.fromCharCode(128512) + String.fromCharCode(128512))",
    ];
    let idx: &[&str] = &[
        "undefined",
        "0",
        "1",
        "2",
        "3",
        "4",
        "5",
        "-1",
        "-2",
        "-100",
        "1.5",
        "-1.5",
        "NaN",
        "Infinity",
        "-Infinity",
        "'1'",
        "'x'",
        "null",
        "true",
        "2147483647",
        "2147483648",
        "-2147483649",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in strs {
        for i in idx {
            bodies.push(format!(
                "var s = {s}; return 'L' + s.length + '/' + Q(s.charAt({i})) + '/' + \
                 String(s.charCodeAt({i}));"
            ));
        }
    }
    diff_bodies(&bodies);
    // and js_runeat straight through the exported symbol, including the
    // negative-index case where the `while (i >= 0)` body never runs and `rune`
    // keeps its EOF initialiser
    let mut rng = Rng::new(0x5920_0006);
    let mut ns: Vec<c_int> = vec![-3, -2, -1, 0, 1, 2, 3, 4, 8, 100, i32::MAX, i32::MIN];
    for _ in 0..12 {
        ns.push(rng.range(-6, 10) as c_int);
    }
    for raw in [
        "", "a", "abc", "\u{e9}", "a\u{e9}b", "\u{ffff}", "\u{10000}", "\u{10001}",
        "a\u{10000}b", "\u{1f600}", "a\u{1f600}b", "\u{10ffff}",
    ] {
        for n in ns.clone() {
            let s = raw.to_string();
            probe_state(&format!("js_runeat {s:?} i={n}"), 0, move || {
                let s2 = s.clone();
                job!(|l, j| {
                    let cs = cstr(&s2);
                    let r = l.js_runeat(j, cs.as_ptr(), n);
                    format!("rune={r} utflen={}", l.js_utflen(cs.as_ptr()))
                })
            });
        }
    }
}

/// Rows 601 / 603 -- `Sp_concat`'s `top == 1` early return and the `js_try`
/// handler that frees `out` and rethrows.
#[test]
fn t_string_concat_paths() {
    expect_exprs(cases![
        // row 601: nothing is pushed, so jsrun.c:1273 substitutes `undefined`
        ("'abc'.concat()", "OK:undefined(undefined)"),
        ("''.concat()", "OK:undefined(undefined)"),
        ("String.prototype.concat.call(5)", "OK:undefined(undefined)"),
        ("'abc'.concat('')", ok_str("abc")),
        ("'abc'.concat('d','e')", ok_str("abcde")),
        ("''.concat('')", ok_str("")),
        ("'a'.concat(1, true, null, undefined, {})", ok_str("a1truenullundefined[object Object]")),
        // row 603: any throw inside the loop rethrows through the handler
        (
            "'a'.concat({toString: function(){ throw new RangeError('arg!') }})",
            "THREW:RangeError: arg!",
        ),
        (
            "'a'.concat('b', {toString: function(){ throw new TypeError('arg2!') }}, 'c')",
            "THREW:TypeError: arg2!",
        ),
        (
            "String.prototype.concat.call(\
             {toString: function(){ throw new Error('recv!') }}, 'x')",
            "THREW:Error: recv!",
        ),
    ]);
    // and a throw from a LATER argument, after `out` has already been grown a
    // few times by js_realloc
    diff_bodies(&[
        body(
            "(function(){ var n=0; \
             var o={toString:function(){ if (++n===4) throw new Error('n4'); return 'x'+n }}; \
             return 'A'.concat(o,o,o,o,o) })()",
        ),
        body("'a'.concat.apply('base', ['1','2','3','4','5','6','7','8'])"),
    ]);
}

/// Rows 607 / 608 / 610 / 611 / 612 -- `Sp_indexOf` / `Sp_lastIndexOf` /
/// `Sp_localeCompare`.  Note `Sp_lastIndexOf`'s default `pos` is
/// `strlen(haystack)` in BYTES while `k` counts RUNES, and `Sp_indexOf` returns
/// -1 for `"".indexOf("")` because the loop never runs.
#[test]
fn t_string_indexof_paths() {
    let hays: &[&str] = &[
        "''",
        "'a'",
        "'abc'",
        "'abcabc'",
        "'aaa'",
        "'\\u00e9\\u00e9'",
        "('x' + String.fromCharCode(128512) + 'x')",
        "'a\\u20acb'",
    ];
    let needles: &[&str] = &[
        "''",
        "'a'",
        "'b'",
        "'abc'",
        "'abcd'",
        "'\\u00e9'",
        "String.fromCharCode(128512)",
        "1",
        "undefined",
        "null",
        "({})",
    ];
    let poss: &[&str] = &[
        "undefined",
        "0",
        "1",
        "2",
        "3",
        "6",
        "100",
        "-1",
        "-100",
        "1.5",
        "NaN",
        "Infinity",
        "-Infinity",
        "'1'",
        "null",
        "2147483647",
        "-2147483648",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for h in hays {
        for n in needles {
            for p in poss {
                bodies.push(format!(
                    "return String({h}.indexOf({n}, {p})) + '/' + \
                     String({h}.lastIndexOf({n}, {p}));"
                ));
            }
            bodies.push(format!(
                "return String({h}.localeCompare({n})) + '/' + \
                 String(String.prototype.localeCompare.call({n}, {h}));"
            ));
        }
    }
    diff_bodies(&bodies);
    expect_exprs(&[
        // row 608: never matched
        ("'abc'.indexOf('z')", "OK:num(-1)"),
        ("''.indexOf('')", "OK:num(-1)"),
        ("''.indexOf('a')", "OK:num(-1)"),
        // row 607: a match strictly before fromIndex is rejected
        ("'abcabc'.indexOf('b', 2)", "OK:num(4)"),
        ("'abcabc'.indexOf('b', 5)", "OK:num(-1)"),
        ("'abc'.indexOf('a', 1)", "OK:num(-1)"),
        // rows 610 / 611
        ("'abc'.lastIndexOf('z')", "OK:num(-1)"),
        ("''.lastIndexOf('')", "OK:num(-1)"),
        ("'abcabc'.lastIndexOf('b')", "OK:num(4)"),
        ("'abcabc'.lastIndexOf('b', 2)", "OK:num(1)"),
        ("'abcabc'.lastIndexOf('b', 0)", "OK:num(-1)"),
        // row 612
        ("'a'.localeCompare('a')", "OK:num(0)"),
        ("'a'.localeCompare('b')", "OK:num(-1)"),
        ("'b'.localeCompare('a')", "OK:num(1)"),
    ]);
}

/// Rows 678 / 679 -- `jsB_new_String` / `jsB_String` with no argument.
#[test]
fn t_string_constructor() {
    diff_bodies(&[
        body("Q(String())"),
        body("Q(new String().valueOf()) + '/' + new String().length"),
        body("Q(String(undefined))"),
        body("Q(String(null))"),
        body("Q(new String(undefined).valueOf())"),
        body("Q(String(1,2))"),
        body("Q(new String(1,2).valueOf())"),
        body("Q(String({toString:function(){return 'T'}}))"),
        body("Object.prototype.toString.call(String())"),
        body("Object.prototype.toString.call(new String())"),
        body("String.prototype.length + '/' + Q(String.prototype.valueOf())"),
    ]);
}

/// Rows 613 / 615 / 616 -- `Sp_substring_imp`'s three exits: the
/// no-surrogate-split fast path, the synthesized LOW surrogate prefix (`i > a`)
/// and the synthesized HIGH surrogate suffix (`k > n`).  Reached by slicing
/// through an astral rune, which `js_utflen` counts as TWO positions.
#[test]
fn t_string_substring_surrogates() {
    let strs: &[&str] = &[
        "String.fromCharCode(128512)",
        "('a' + String.fromCharCode(128512) + 'b')",
        "(String.fromCharCode(128512) + String.fromCharCode(128513))",
        "('a' + String.fromCharCode(65536) + String.fromCharCode(1114111) + 'z')",
        "('\\u00e9' + String.fromCharCode(128512) + '\\u20ac')",
        "'plain'",
        "''",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in strs {
        for a in -2i32..8 {
            for b in -2i32..8 {
                bodies.push(format!(
                    "var s = {s}; return 'L' + s.length + ' sl' + Q(s.slice({a}, {b})) + \
                     ' su' + Q(s.substring({a}, {b}));"
                ));
            }
        }
        bodies.push(format!(
            "var s = {s}; return 'L' + s.length + ' sl' + Q(s.slice(1)) + \
             ' su' + Q(s.substring(1)) + ' sl0' + Q(s.slice(0)) + ' suU' + Q(s.substring());"
        ));
    }
    diff_bodies(&bodies);
}

/// Rows 618-622 / 624-626 -- `Sp_slice` (which rebases negatives against `len`)
/// and `Sp_substring` (which does NOT rebase, only clamps), plus their shared
/// `s == e` empty-range exit.
#[test]
fn t_string_slice_substring_clamping() {
    let idx: &[&str] = &[
        "undefined",
        "0",
        "1",
        "2",
        "3",
        "4",
        "5",
        "-1",
        "-2",
        "-3",
        "-4",
        "-5",
        "-100",
        "100",
        "1.5",
        "-1.5",
        "NaN",
        "Infinity",
        "-Infinity",
        "'2'",
        "'-2'",
        "'x'",
        "null",
        "true",
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for a in idx {
        for b in idx {
            bodies.push(format!(
                "return Q('abcd'.slice({a}, {b})) + '/' + Q('abcd'.substring({a}, {b})) + \
                 '/' + Q(''.slice({a}, {b})) + '/' + Q(''.substring({a}, {b}));"
            ));
        }
    }
    diff_bodies(&bodies);
    expect_exprs(cases![
        // row 622 / 626: s == e
        ("Q('abcd'.slice(2,2))", ok_str("\"\"")),
        ("Q('abcd'.substring(2,2))", ok_str("\"\"")),
        ("Q(''.slice(0,0))", ok_str("\"\"")),
        // rows 618/619: slice rebases negatives
        ("'abcd'.slice(-2)", ok_str("cd")),
        ("'abcd'.slice(-2,-1)", ok_str("c")),
        ("'abcd'.slice(1,-1)", ok_str("bc")),
        // rows 620/621: clamped
        ("'abcd'.slice(-100,100)", ok_str("abcd")),
        ("'abcd'.slice(3,1)", ok_str("bc")),
        // rows 624/625: substring clamps instead, and swaps
        ("'abcd'.substring(-2)", ok_str("abcd")),
        ("'abcd'.substring(-2,-1)", ok_str("")),
        ("'abcd'.substring(3,1)", ok_str("bc")),
        ("'abcd'.substring(100,1)", ok_str("bcd")),
    ]);
}

/// Rows 629 / 631 -- `tolowerrune_full` / `toupperrune_full` returning NULL
/// (single-rune fallback) versus a real multi-rune mapping.
#[test]
fn t_string_case_mapping() {
    let strs: &[&str] = &[
        "''",
        "'abcXYZ'",
        "'123 !@#'",
        "'\\u00df'",
        "'\\u0130'",
        "'\\u0131'",
        "'\\u0149'",
        "'\\ufb00'",
        "'\\ufb01'",
        "'\\ufb02'",
        "'\\ufb03'",
        "'\\ufb04'",
        "'\\u0390'",
        "'\\u1e96'",
        "'\\u1f50'",
        "'\\ufb17'",
        "'\\u01f0'",
        "'\\u1e9a'",
        "'\\u0587'",
        "'\\u00e9\\u00c9'",
        "'\\u03c3\\u03a3\\u03c2'",
        "'\\u0410\\u0430'",
        "String.fromCharCode(128512)",
        "String.fromCharCode(66600)",
        "'a\\u00dfb\\u0130c'",
        "'\\ufb00\\ufb01\\ufb02\\u00df\\u0149'",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in strs {
        bodies.push(format!(
            "var s = {s}; return 'L' + s.length + \
             ' lo' + Q(s.toLowerCase()) + '#' + s.toLowerCase().length + \
             ' up' + Q(s.toUpperCase()) + '#' + s.toUpperCase().length + \
             ' llo' + Q(s.toLocaleLowerCase()) + ' lup' + Q(s.toLocaleUpperCase());"
        ));
        bodies.push(format!(
            "var s = {s}; return Q(s.toUpperCase().toLowerCase()) + '/' + \
             Q(s.toLowerCase().toUpperCase());"
        ));
    }
    // randomised runes so both full-mapping tables are walked broadly
    let mut rng = Rng::new(0x6290_0007);
    for _ in 0..40 {
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..(1 + rng.below(6)) {
            let c = match rng.below(4) {
                0 => rng.below(0x80),
                1 => 0x80 + rng.below(0x780),
                2 => 0x800 + rng.below(0xf800),
                _ => 0xfb00 + rng.below(0x60),
            };
            parts.push(format!("String.fromCharCode({c})"));
        }
        bodies.push(format!(
            "var s = {}; return Q(s.toLowerCase()) + '/' + Q(s.toUpperCase());",
            parts.join(" + ")
        ));
    }
    diff_bodies(&bodies);
}

/// Rows 634 / 635 -- `Sp_trim`.  `istrim` (jsstring.c:425-429) is applied to a
/// raw `char`, so the four multi-byte code points it lists (U+00A0, U+FEFF,
/// U+2028, U+2029) can never match a single UTF-8 byte and are NOT trimmed.
#[test]
fn t_string_trim() {
    let strs: &[&str] = &[
        "''",
        "' '",
        "'   '",
        "'\\t\\n\\r\\v\\f '",
        "'  a  '",
        "'a'",
        "'\\ta\\t'",
        "'\\u00a0'",
        "'\\u00a0a\\u00a0'",
        "'\\ufeff'",
        "'\\ufeffa\\ufeff'",
        "'\\u2028'",
        "'\\u2029a'",
        "' \\u00a0 a \\u00a0 '",
        "'\\u0000'",
        "'a\\u0000b'",
        "String.fromCharCode(160)",
        "String.fromCharCode(9,11,12,32,10,13)",
        "(String.fromCharCode(9) + 'x' + String.fromCharCode(13))",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in strs {
        bodies.push(format!(
            "var s = {s}; var t = s.trim(); return 'L' + s.length + '->' + t.length + \
             ' ' + Q(t);"
        ));
    }
    let mut rng = Rng::new(0x6340_0008);
    for _ in 0..40 {
        let n = 1 + rng.below(6);
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..n {
            let c = match rng.below(5) {
                0 => 0x20,
                1 => 9 + rng.below(5),
                2 => 0x61 + rng.below(26),
                3 => 0xa0,
                _ => [0xfeff, 0x2028, 0x2029, 0x3000][rng.below(4) as usize],
            };
            parts.push(format!("String.fromCharCode({c})"));
        }
        bodies.push(format!(
            "var s = {}; return 'L' + s.length + '->' + s.trim().length + ' ' + Q(s.trim());",
            parts.join(" + ")
        ));
    }
    diff_bodies(&bodies);
}

/// Row 638 -- `S_fromCharCode`'s `js_touint32` wrap.  No range check at all, so
/// negative and >0x10FFFF arguments are silently wrapped modulo 2^32 and handed
/// to `runetochar`, which substitutes `Runeerror` for anything above `Runemax`
/// (utf.c:167-168).  Also the reachable half of row 637: no RangeError however
/// many arguments are passed.
#[test]
fn t_string_fromcharcode_range() {
    let mut vals: Vec<String> = vec![
        "0".into(),
        "-1".into(),
        "-2".into(),
        "1".into(),
        "65".into(),
        "127".into(),
        "128".into(),
        "0x7ff".into(),
        "0x800".into(),
        "0xd7ff".into(),
        "0xd800".into(),
        "0xdbff".into(),
        "0xdc00".into(),
        "0xdfff".into(),
        "0xe000".into(),
        "0xfffd".into(),
        "0xffff".into(),
        "0x10000".into(),
        "0x10ffff".into(),
        "0x110000".into(),
        "0x110001".into(),
        "0x1fffff".into(),
        "0x200000".into(),
        "0x7fffffff".into(),
        "0x80000000".into(),
        "0xffffffff".into(),
        "4294967296".into(),
        "4294967297".into(),
        "-4294967295".into(),
        "1.5".into(),
        "-1.5".into(),
        "NaN".into(),
        "Infinity".into(),
        "-Infinity".into(),
        "'65'".into(),
        "'x'".into(),
        "null".into(),
        "undefined".into(),
        "true".into(),
        "({})".into(),
    ];
    let mut rng = Rng::new(0x6380_0009);
    for _ in 0..30 {
        vals.push(format!("{}", rng.range(-0x11_0010, 0x11_0010)));
    }
    let mut bodies: Vec<String> = Vec::new();
    for v in &vals {
        bodies.push(format!(
            "var s = String.fromCharCode({v}); return 'L' + s.length + ' ' + Q(s);"
        ));
    }
    // several arguments at once, and a LOT of arguments (row 637's reachable
    // half: `(top-1) * UTFmax + 1` is never checked against JS_STRLIMIT, and no
    // RangeError is raised)
    bodies.push("return Q(String.fromCharCode());".into());
    bodies.push("return Q(String.fromCharCode(72,73));".into());
    bodies.push("return Q(String.fromCharCode(-1,-1,-1));".into());
    for n in [1usize, 2, 100, 1000, 2000, 3000, 4000, 4080, 4090] {
        bodies.push(format!(
            "var a = new Array({n}); var i; for (i=0;i<{n};++i) a[i]=65+(i%26); \
             var s = String.fromCharCode.apply(null, a); return 'n={n} L' + s.length + \
             ' first' + s.charCodeAt(0) + ' last' + s.charCodeAt(s.length-1);"
        ));
    }
    diff_bodies(&bodies);
}

/// Rows 640-644 -- `Sp_match`.
#[test]
fn t_string_match_paths() {
    expect_exprs(cases![
        // row 640: an undefined argument becomes the empty regexp
        ("A('abc'.match(undefined))", ok_str("[str#0\"\"]#1")),
        ("A('abc'.match())", ok_str("[str#0\"\"]#1")),
        ("A(''.match(undefined))", ok_str("[str#0\"\"]#1")),
        // row 641: a non-regexp argument whose string form is not a valid pattern
        (
            "'x'.match('[')",
            "THREW:SyntaxError: regular expression: unterminated character class",
        ),
        (
            "'x'.match('(')",
            "THREW:SyntaxError: regular expression: unmatched '('",
        ),
        (
            "'x'.match('a{2,1}')",
            "THREW:SyntaxError: regular expression: invalid quantifier",
        ),
        // row 642: no `g` flag delegates to js_RegExp_prototype_exec
        ("A('abcabc'.match(/b/))", ok_str("[str#1\"b\"]#1")),
        ("A('abcabc'.match('b'))", ok_str("[str#1\"b\"]#1")),
        ("'abc'.match(/z/)", "OK:null"),
        // row 644: a global match with zero hits
        ("'abc'.match(/z/g)", "OK:null"),
        ("''.match(/z/g)", "OK:null"),
        // row 643: the scan pointer walks one rune past `e`
        ("A('abc'.match(/x*/g))", ok_str("[str#0\"\",str#0\"\",str#0\"\",str#0\"\"]#4")),
        ("A(''.match(/x*/g))", ok_str("[str#0\"\"]#1")),
        ("A('abc'.match(/b*/g))", ok_str("[str#0\"\",str#1\"b\",str#0\"\",str#0\"\"]#4")),
    ]);
    let subjects: &[&str] = &[
        "''",
        "'a'",
        "'abcabc'",
        "'aaa'",
        "'\\u00e9\\u00e9'",
        "('x' + String.fromCharCode(128512) + 'x')",
        "'a\\nb\\nc'",
    ];
    let pats: &[&str] = &[
        "/a/",
        "/a/g",
        "/a/gi",
        "/a*/g",
        "/x*/g",
        "/(a)(b)/g",
        "/^a/gm",
        "/$/g",
        "/\\b/g",
        "undefined",
        "''",
        "'a'",
        "1",
        "null",
        "({})",
        "/(?:)/g",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in subjects {
        for p in pats {
            bodies.push(format!(
                "var r = {s}.match({p}); return r === null ? 'null' : A(r);"
            ));
            bodies.push(format!(
                "var re = {p}; var a = {s}.match(re); var b = {s}.match(re); \
                 return (a===null?'null':A(a)) + '|' + (b===null?'null':A(b)) + \
                 '|last=' + (re && re.lastIndex !== undefined ? re.lastIndex : 'n/a');"
            ));
        }
    }
    diff_bodies(&bodies);
}

/// Rows 646 / 647 / 648 -- `Sp_search`.
#[test]
fn t_string_search_paths() {
    expect_exprs(cases![
        ("'abc'.search(undefined)", "OK:num(0)"),
        ("'abc'.search()", "OK:num(0)"),
        ("''.search(undefined)", "OK:num(0)"),
        (
            "'x'.search('[')",
            "THREW:SyntaxError: regular expression: unterminated character class",
        ),
        ("'abc'.search(/z/)", "OK:num(-1)"),
        ("'abc'.search(/b/)", "OK:num(1)"),
        ("'abc'.search('b')", "OK:num(1)"),
        ("''.search(/z/)", "OK:num(-1)"),
    ]);
    let mut bodies: Vec<String> = Vec::new();
    for s in [
        "''",
        "'abc'",
        "('x' + String.fromCharCode(128512) + 'yz')",
        "'\\u00e9\\u20acz'",
        "'a\\nb'",
    ] {
        for p in [
            "/z/", "/b/", "/b/g", "/^b/m", "/$/", "/\\u00e9/", "/z*/", "undefined", "''", "'b'",
            "1",
        ] {
            bodies.push(format!("return String({s}.search({p}));"));
        }
    }
    diff_bodies(&bodies);
}

/// Rows 651-657 -- `Sp_replace_regexp`: the no-match early return, the `js_try`
/// handler, and every arm of the `$` substitution switch.
///
/// Row 655 is a genuine off-by-one in the C: the test at jsstring.c:605 is
/// `x > 10`, not `x >= 10`, so an out-of-range `$10` takes the single-digit
/// branch and emits `'0' + 10`, i.e. `':'`.  Reproduced, NOT fixed.
#[test]
fn t_string_replace_regexp() {
    expect_exprs(cases![
        // row 651: the first js_doregexec found nothing
        ("'abc'.replace(/z/, 'X')", ok_str("abc")),
        ("'abc'.replace(/z/g, 'X')", ok_str("abc")),
        ("''.replace(/z/, 'X')", ok_str("")),
        // row 653: a lone trailing '$'
        ("'abc'.replace(/b/, 'X$')", ok_str("aX$c")),
        ("'abc'.replace(/b/, '$')", ok_str("a$c")),
        ("'abc'.replace(/b/, '$$')", ok_str("a$c")),
        // row 654: $N with x == 0 or x >= m.nsub
        ("'abc'.replace(/b/, '[$0]')", ok_str("a[$0]c")),
        ("'abc'.replace(/b/, '[$1]')", ok_str("a[$1]c")),
        ("'abc'.replace(/b/, '[$9]')", ok_str("a[$9]c")),
        ("'abc'.replace(/(b)/, '[$1]')", ok_str("a[b]c")),
        ("'abc'.replace(/(b)/, '[$2]')", ok_str("a[$2]c")),
        // row 655: `$10` exactly -> '0' + 10 == ':'
        ("'abc'.replace(/b/, '[$10]')", ok_str("a[$:]c")),
        ("'abc'.replace(/(b)/, '[$10]')", ok_str("a[$:]c")),
        ("'abc'.replace(/b/, '[$11]')", ok_str("a[$11]c")),
        ("'abc'.replace(/b/, '[$12]')", ok_str("a[$12]c")),
        ("'abc'.replace(/b/, '[$99]')", ok_str("a[$99]c")),
        ("'abc'.replace(/b/, '[$00]')", ok_str("a[$0]c")),
        ("'abc'.replace(/b/, '[$01]')", ok_str("a[$1]c")),
        ("'abc'.replace(/b/, '[$09]')", ok_str("a[$9]c")),
        // row 656: '$' followed by anything else
        ("'abc'.replace(/b/, '[$x]')", ok_str("a[$x]c")),
        ("'abc'.replace(/b/, '[$ ]')", ok_str("a[$ ]c")),
        ("'abc'.replace(/b/, '[$-]')", ok_str("a[$-]c")),
        // the supported non-digit escapes
        ("'abc'.replace(/b/, '[$&]')", ok_str("a[b]c")),
        ("'abc'.replace(/b/, '[$`]')", ok_str("a[a]c")),
        ("'abc'.replace(/b/, \"[$']\")", ok_str("a[c]c")),
        // row 657: a global replace whose match is empty at end of input
        ("'abc'.replace(/x*/g, '-')", ok_str("-a-b-c-")),
        ("''.replace(/x*/g, '-')", ok_str("-")),
        ("'aaa'.replace(/a*/g, '-')", ok_str("--")),
        ("'abc'.replace(/b*/g, '-')", ok_str("-a--c-")),
    ]);
    // row 652: any throw after the js_try re-raises through the handler
    expect_exprs(&[
        (
            "'abc'.replace(/b/, function(){ throw new RangeError('fn!') })",
            "THREW:RangeError: fn!",
        ),
        (
            "'abc'.replace(/b/g, function(){ throw new TypeError('fn2!') })",
            "THREW:TypeError: fn2!",
        ),
        (
            "'abc'.replace(/b/, {toString: function(){ throw new Error('r!') }})",
            "THREW:Error: r!",
        ),
        // NOTE the offset argument is `s - source` (jsstring.c:575), relative to
        // the CURRENT scan position rather than the original string, so it is 0
        // on every global iteration; count invocations instead.
        (
            "(function(){ var n=0; return 'aaa'.replace(/a/g, function(){ \
             if (++n===3) throw new Error('third'); return 'x' }) })()",
            "THREW:Error: third",
        ),
    ]);
    // the function-replacement arm, capped at 13 capture groups.
    //
    // WITH ALL 15 GROUPS PARTICIPATING this is UNDEFINED BEHAVIOUR: jsstring.c:573
    // walks `for (x = 0; m.sub[x].sp; ++x)` and `regexec` (regexp.c:1235) NULLs
    // exactly REG_MAXSUB == 16 entries, so `m.sub[16]` is read one past the end
    // of `Resub` and both libraries segfault.
    let mut bodies: Vec<String> = Vec::new();
    for g in 0..=13usize {
        let letters: String = (0..g)
            .map(|i| format!("({})", (b'a' + i as u8) as char))
            .collect();
        let subject: String = (0..g.max(1))
            .map(|i| (b'a' + i as u8) as char)
            .collect();
        let pat = if g == 0 {
            "/a/".to_string()
        } else {
            format!("/{letters}/")
        };
        bodies.push(format!(
            "return Q('{subject}z'.replace({pat}, function(){{ \
             return '<' + arguments.length + ':' + \
             Array.prototype.join.call(arguments, ',') + '>' }}));"
        ));
    }
    // and the string-replacement arm over every subject / pattern / template mix
    let subjects: &[&str] = &[
        "''",
        "'abc'",
        "'aaa'",
        "'abcabc'",
        "'\\u00e9x\\u00e9'",
        "('q' + String.fromCharCode(128512) + 'q')",
        "'a\\nb'",
    ];
    let pats: &[&str] = &[
        "/b/", "/b/g", "/a*/g", "/(a)(b)/", "/(a)(b)/g", "/^a/gm", "/$/g", "/z/", "/(?:)/g",
        "/(a)|(z)/g",
    ];
    let reps: &[&str] = &[
        "'X'", "''", "'$&$&'", "'$`|$\\''", "'$1-$2'", "'$3'", "'$0'", "'$10'", "'$'", "'$$'",
        "'$q'", "1", "null", "undefined",
    ];
    for s in subjects {
        for p in pats {
            for r in reps {
                bodies.push(format!("return Q({s}.replace({p}, {r}));"));
            }
            bodies.push(format!(
                "return Q({s}.replace({p}, function(m){{ return '<'+m+'>' }}));"
            ));
            bodies.push(format!(
                "return Q({s}.replace({p}, function(){{ \
                 return arguments.length + ':' + arguments[arguments.length-2] }}));"
            ));
        }
    }
    diff_bodies(&bodies);
}

/// Rows 659-662 -- `Sp_replace_string`.  `$N` group references are NOT supported
/// on this path (jsstring.c:692's `default`), so `$1` is emitted literally.
#[test]
fn t_string_replace_string() {
    expect_exprs(cases![
        // row 659: strstr found nothing
        ("'abc'.replace('z', 'X')", ok_str("abc")),
        ("''.replace('z', 'X')", ok_str("")),
        ("'abc'.replace('abcd', 'X')", ok_str("abc")),
        // only the FIRST occurrence is replaced; there is no global string path
        ("'aaa'.replace('a', 'X')", ok_str("Xaa")),
        ("'abc'.replace('', 'X')", ok_str("Xabc")),
        ("''.replace('', 'X')", ok_str("X")),
        // row 661: a lone trailing '$'
        ("'abc'.replace('b', 'X$')", ok_str("aX$c")),
        ("'abc'.replace('b', '$')", ok_str("a$c")),
        ("'abc'.replace('b', '$$')", ok_str("a$c")),
        // row 662: '$N' is NOT a group reference here
        ("'abc'.replace('b', '[$1]')", ok_str("a[$1]c")),
        ("'abc'.replace('b', '[$0]')", ok_str("a[$0]c")),
        ("'abc'.replace('b', '[$10]')", ok_str("a[$10]c")),
        ("'abc'.replace('b', '[$x]')", ok_str("a[$x]c")),
        // the four supported escapes
        ("'abc'.replace('b', '[$&]')", ok_str("a[b]c")),
        ("'abc'.replace('b', '[$`]')", ok_str("a[a]c")),
        ("'abc'.replace('b', \"[$']\")", ok_str("a[c]c")),
    ]);
    // row 660: any throw after the js_try
    expect_exprs(&[
        (
            "'abc'.replace('b', function(){ throw new RangeError('sfn!') })",
            "THREW:RangeError: sfn!",
        ),
        (
            "'abc'.replace('b', {toString: function(){ throw new Error('sr!') }})",
            "THREW:Error: sr!",
        ),
        (
            "'abc'.replace({toString: function(){ throw new Error('needle!') }}, 'x')",
            "THREW:Error: needle!",
        ),
    ]);
    let mut bodies: Vec<String> = Vec::new();
    for s in [
        "''",
        "'abc'",
        "'aaa'",
        "'\\u00e9x\\u00e9'",
        "('q' + String.fromCharCode(128512) + 'q')",
    ] {
        for n in ["''", "'a'", "'b'", "'abc'", "'z'", "1", "null", "undefined", "({})"] {
            for r in ["'X'", "''", "'$&'", "'$`'", "'$\\''", "'$1'", "'$'", "1", "null"] {
                bodies.push(format!("return Q({s}.replace({n}, {r}));"));
            }
            bodies.push(format!(
                "return Q({s}.replace({n}, function(m,o,t){{ \
                 return '<'+m+'@'+o+'#'+t.length+'>' }}));"
            ));
        }
    }
    diff_bodies(&bodies);
}

/// Rows 665-671 -- `Sp_split_regexp`: the `1 << 30` default limit, the
/// `limit == 0` early return, the empty-input special case, the empty-match
/// guard and the three `len == limit` truncations.
#[test]
fn t_string_split_regexp() {
    expect_exprs(cases![
        // row 665 / 666: the limit default and the `limit == 0` early return.
        // NOTE the test is `limit == 0`, so a NEGATIVE limit is NOT an early
        // return -- it simply never equals `len`.
        ("A('a1b2c'.split(/\\d/))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        ("A('a1b2c'.split(/\\d/, undefined))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        ("A('a1b2c'.split(/\\d/, 0))", ok_str("[]#0")),
        ("A('a1b2c'.split(/\\d/, NaN))", ok_str("[]#0")),
        ("A('a1b2c'.split(/\\d/, 'x'))", ok_str("[]#0")),
        ("A('a1b2c'.split(/\\d/, null))", ok_str("[]#0")),
        ("A('a1b2c'.split(/\\d/, 0.5))", ok_str("[]#0")),
        ("A('a1b2c'.split(/\\d/, -1))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        // rows 669 / 671: len == limit truncations
        ("A('a1b2c'.split(/\\d/, 1))", ok_str("[str#1\"a\"]#1")),
        ("A('a1b2c'.split(/\\d/, 2))", ok_str("[str#1\"a\",str#1\"b\"]#2")),
        ("A('a1b2c'.split(/\\d/, 3))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        ("A('a1b2c'.split(/\\d/, 4))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        // row 670: truncation in the middle of the capture groups
        (
            "A('a1b'.split(/(\\d)(x?)/, 2))",
            ok_str("[str#1\"a\",str#1\"1\"]#2"),
        ),
        (
            "A('a1b'.split(/(\\d)(x?)/, 3))",
            ok_str("[str#1\"a\",str#1\"1\",str#0\"\"]#3"),
        ),
        (
            "A('a1b'.split(/(\\d)(x?)/))",
            ok_str("[str#1\"a\",str#1\"1\",str#0\"\",str#1\"b\"]#4"),
        ),
        // row 667: the empty input string
        ("A(''.split(/z/))", ok_str("[str#0\"\"]#1")),
        ("A(''.split(/z*/))", ok_str("[]#0")),
        ("A(''.split(/(?:)/))", ok_str("[]#0")),
        ("A(''.split(/z/, 0))", ok_str("[]#0")),
        // row 668: an empty match at the end of the previous match is rejected
        ("A('abc'.split(/x*/))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        ("A('abc'.split(/(?:)/))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        ("A('aaa'.split(/a*/))", ok_str("[str#0\"\",str#0\"\"]#2")),
    ]);
    let subjects: &[&str] = &[
        "''",
        "'a'",
        "'a1b2c'",
        "'1a1'",
        "'aaa'",
        "'\\u00e9\\u00e9'",
        "('q' + String.fromCharCode(128512) + 'q')",
        "'a\\nb\\nc'",
    ];
    let pats: &[&str] = &[
        "/\\d/", "/a/", "/a/g", "/x*/", "/(?:)/", "/(a)/", "/(a)(b)?/", "/^/m", "/$/m", "/z/",
        "/\\b/",
    ];
    let limits: &[&str] = &[
        "undefined",
        "0",
        "1",
        "2",
        "3",
        "10",
        "-1",
        "-100",
        "1.9",
        "NaN",
        "Infinity",
        "-Infinity",
        "'2'",
        "null",
        "true",
        "1073741824",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in subjects {
        for p in pats {
            for l in limits {
                bodies.push(format!("return A({s}.split({p}, {l}));"));
            }
        }
    }
    diff_bodies(&bodies);
}

/// Rows 673-677 -- `Sp_split_string` and `Sp_split`'s undefined-separator arm.
#[test]
fn t_string_split_string() {
    expect_exprs(cases![
        // row 673: the default limit
        ("A('a,b,c'.split(','))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        ("A('a,b,c'.split(',', undefined))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        // row 674: limit == 0
        ("A('a,b,c'.split(',', 0))", ok_str("[]#0")),
        ("A('a,b,c'.split(',', NaN))", ok_str("[]#0")),
        ("A('a,b,c'.split(',', 'x'))", ok_str("[]#0")),
        // row 675: an empty separator splits into runes
        ("A('abc'.split(''))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        ("A('abc'.split('', 2))", ok_str("[str#1\"a\",str#1\"b\"]#2")),
        ("A(''.split(''))", ok_str("[]#0")),
        ("A(''.split('x'))", ok_str("[str#0\"\"]#1")),
        // row 676: truncation at limit / the final unmatched piece
        ("A('a,b,c'.split(',', 1))", ok_str("[str#1\"a\"]#1")),
        ("A('a,b,c'.split(',', 2))", ok_str("[str#1\"a\",str#1\"b\"]#2")),
        ("A('a,b,c'.split(',', 99))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        ("A('a,b,c'.split(',', -1))", ok_str("[]#0")),
        ("A(',a,'.split(','))", ok_str("[str#0\"\",str#1\"a\",str#0\"\"]#3")),
        // row 677: an undefined separator never consults `limit`
        ("A('a,b'.split(undefined))", ok_str("[str#3\"a,b\"]#1")),
        ("A('a,b'.split(undefined, 0))", ok_str("[str#3\"a,b\"]#1")),
        ("A('a,b'.split())", ok_str("[str#3\"a,b\"]#1")),
        ("A(''.split(undefined))", ok_str("[str#0\"\"]#1")),
        ("A(String.prototype.split.call(5, undefined))", ok_str("[str#1\"5\"]#1")),
    ]);
    let subjects: &[&str] = &[
        "''",
        "'a'",
        "'a,b,c'",
        "',,'",
        "'aaa'",
        "'\\u00e9\\u00e9'",
        "('q' + String.fromCharCode(128512) + 'q')",
    ];
    let seps: &[&str] = &["''", "','", "'a'", "'aa'", "'z'", "1", "null", "({})", "'\\u00e9'"];
    let limits: &[&str] = &[
        "undefined",
        "0",
        "1",
        "2",
        "3",
        "-1",
        "1.9",
        "NaN",
        "Infinity",
        "-Infinity",
        "'2'",
        "null",
        "true",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in subjects {
        for sep in seps {
            for l in limits {
                bodies.push(format!("return A({s}.split({sep}, {l}));"));
            }
        }
    }
    diff_bodies(&bodies);
}

/// Row 590 -- `js_doregexec` (jsstring.c:5-11) raising plain
/// `Error "regexec failed"` when `js_regexec` returns a negative value.  Reached
/// through the `REG_MAXREC == 4096` recursion limit of `match()`
/// (regexp.c:1075-1076) using a LINEAR repetition, so no catastrophic
/// backtracking is needed.  All four jsstring.c call sites are driven.
#[test]
fn t_string_regexec_failed() {
    with_big_stack(body_t_string_regexec_failed);
}

fn body_t_string_regexec_failed() {
    // Which `n` first trips REG_MAXREC depends on how many `match()` frames the
    // compiled program uses per repetition, so the exact threshold is not
    // asserted; what IS asserted is that both libraries agree on every `n` AND
    // that the "regexec failed" Error really is reached by at least one case at
    // each of the four jsstring.c call sites.
    let mut bodies: Vec<String> = Vec::new();
    let mut which: Vec<&'static str> = Vec::new();
    for n in [100usize, 1000, 4000, 4090, 4095, 4096, 4097, 4100, 5000, 6000, 9000] {
        let mk = format!("var s = new Array({}).join('a'); ", n + 1);
        // Sp_match's GLOBAL arm (jsstring.c:500) -- the non-global arm delegates
        // to js_RegExp_prototype_exec, whose js_doregexec is jsregexp.c:77.
        bodies.push(format!("{mk}return String(s.match(/a*b/g));"));
        which.push("match");
        // Sp_search (jsstring.c:537)
        bodies.push(format!("{mk}return String(s.search(/a*b/));"));
        which.push("search");
        // Sp_replace_regexp's first exec (jsstring.c:554)
        bodies.push(format!("{mk}return String(s.replace(/a*b/, 'X').length);"));
        which.push("replace");
        // Sp_split_regexp's loop exec (jsstring.c:748)
        bodies.push(format!("{mk}return String(s.split(/a*b/).length);"));
        which.push("split");
        bodies.push(format!("{mk}return String(s.split(/a+b/, 3).length);"));
        which.push("split");
    }
    let got = drive(&bodies);
    for site in ["match", "search", "replace", "split"] {
        let hit = got
            .iter()
            .zip(which.iter())
            .any(|(g, w)| *w == site && g == "THREW:Error: regexec failed");
        assert!(
            hit,
            "row 590 never reached through {site}: {got:?}"
        );
    }
    // and the sub-limit cases really do succeed, so the throw is input-driven
    assert_eq!(got[0], ok_str("null"), "n=100 match should succeed: {}", got[0]);
    assert_eq!(got[1], ok_str("-1"), "n=100 search should succeed: {}", got[1]);
}

/* =========================================================================
 *  jsnumber.c
 * ========================================================================= */

/// Rows 680 / 681 -- `jsB_new_Number` / `jsB_Number` with no argument.
#[test]
fn t_number_constructor() {
    diff_bodies(&[
        body("String(Number())"),
        body("String(new Number().valueOf())"),
        body("String(Number(undefined))"),
        body("String(new Number(undefined).valueOf())"),
        body("String(Number(null))"),
        body("String(Number(1,2))"),
        body("String(new Number(1,2).valueOf())"),
        body("String(Number('  12  '))"),
        body("String(Number({}))"),
        body("Object.prototype.toString.call(Number())"),
        body("Object.prototype.toString.call(new Number())"),
        body("String(Number.prototype.valueOf.call(Number.prototype))"),
        body("String(1/Number(-0))"),
    ]);
}

/// Rows 682-688 / 690-692 -- `Np_valueOf` and `Np_toString`: the shared
/// `js_toobject(J, 0)` TypeError, the `JS_CNUMBER` class check, the radix
/// default, the `radix == 10` fast path, the `invalid radix` RangeError and the
/// three non-finite short circuits.
#[test]
fn t_number_valueof_tostring() {
    let mut cases: Vec<(String, String)> = Vec::new();
    for m in ["valueOf", "toString", "toLocaleString"] {
        for (recv, want) in [
            ("undefined", "cannot convert undefined to object"),
            ("null", "cannot convert null to object"),
        ] {
            for form in call_spellings(recv, &format!("Number.prototype.{m}"), &[]) {
                cases.push((form, format!("THREW:TypeError: {want}")));
            }
            cases.push((
                format!("Number.prototype.{m}.call({recv}, 16)"),
                format!("THREW:TypeError: {want}"),
            ));
        }
        // rows 683 / 686: coercible but not JS_CNUMBER
        for recv in [
            "'x'",
            "'1'",
            "true",
            "({})",
            "[]",
            "[1]",
            "(function(){})",
            "/re/",
            "new String('1')",
            "new Boolean(true)",
            "new Date(0)",
            "Math",
        ] {
            cases.push((
                format!("Number.prototype.{m}.call({recv})"),
                "THREW:TypeError: not a number".to_string(),
            ));
        }
        let seven = if m == "valueOf" {
            "OK:num(7)".to_string()
        } else {
            ok_str("7")
        };
        cases.push((
            format!("Number.prototype.{m}.call(new Number(7))"),
            seven.clone(),
        ));
        cases.push((format!("Number.prototype.{m}.call(7)"), seven));
    }
    // row 685: the radix argument is coerced BEFORE the class check, so a
    // throwing valueOf pre-empts "not a number"
    cases.push((
        "Number.prototype.toString.call('x', \
         {valueOf: function(){ throw new RangeError('radix!') }})"
            .to_string(),
        "THREW:RangeError: radix!".to_string(),
    ));
    cases.push((
        "Number.prototype.toString.call(1, \
         {valueOf: function(){ throw new RangeError('radix2!') }})"
            .to_string(),
        "THREW:RangeError: radix2!".to_string(),
    ));
    // row 688: radix out of [2, 36]
    for r in [
        "0", "1", "-1", "-2", "37", "38", "100", "0.5", "1.5", "-0.5", "1e30", "-1e30",
        "Infinity", "-Infinity", "'1'", "'37'", "false", "null",
    ] {
        let w = match r {
            "0" | "0.5" | "-0.5" | "false" | "null" => 0i64,
            "1" | "1.5" | "'1'" => 1,
            "-1" => -1,
            "-2" => -2,
            "37" | "'37'" => 37,
            "38" => 38,
            "100" => 100,
            "1e30" | "Infinity" => 2147483647,
            _ => -2147483648,
        };
        // `radix == 10` is checked first, and 0/1/-1/... are never 10
        let _ = w;
        cases.push((
            format!("(5).toString({r})"),
            "THREW:RangeError: invalid radix".to_string(),
        ));
        cases.push((
            format!("(5).toLocaleString({r})"),
            "THREW:RangeError: invalid radix".to_string(),
        ));
    }
    // rows 687 / 690 / 691 / 692
    cases.push(("(5).toString(10)".to_string(), ok_str("5")));
    cases.push(("(5).toString(undefined)".to_string(), ok_str("5")));
    cases.push(("(5).toString()".to_string(), ok_str("5")));
    cases.push(("(5).toLocaleString()".to_string(), ok_str("5")));
    cases.push(("(0/0).toString(10)".to_string(), ok_str("NaN")));
    cases.push(("(0).toString(2)".to_string(), ok_str("0")));
    cases.push(("(-0).toString(2)".to_string(), ok_str("0")));
    cases.push(("(0/0).toString(2)".to_string(), ok_str("NaN")));
    cases.push(("(0/0).toString(36)".to_string(), ok_str("NaN")));
    cases.push((
        "(1/0).toString(2)".to_string(),
        ok_str("Infinity"),
    ));
    cases.push((
        "(-1/0).toString(2)".to_string(),
        ok_str("-Infinity"),
    ));
    cases.push((
        "(-1/0).toString(36)".to_string(),
        ok_str("-Infinity"),
    ));
    let refs: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    expect_exprs(&refs);
}

/// Rows 689 / 693 / 694 -- the radix-conversion body of `Np_toString`: the
/// `1 << 52` mantissa cap, the trailing-zero trim and the unbounded
/// `char buf[100]` digit buffer.  Swept over every radix 2..36 and a wide range
/// of magnitudes.
#[test]
fn t_number_radix_digits() {
    let mut vals: Vec<String> = vec![
        "0".into(),
        "1".into(),
        "-1".into(),
        "2".into(),
        "35".into(),
        "36".into(),
        "255".into(),
        "65535".into(),
        "4294967295".into(),
        "4294967296".into(),
        "9007199254740991".into(),
        "9007199254740992".into(),
        "9007199254740993".into(),
        "0.5".into(),
        "0.25".into(),
        "0.1".into(),
        "-0.1".into(),
        "1/3".into(),
        "1e-10".into(),
        "1e10".into(),
        "1e20".into(),
        "1e100".into(),
        "1e-100".into(),
        "1e300".into(),
        "5e-324".into(),
        "1.7976931348623157e308".into(),
        "-1.7976931348623157e308".into(),
        "1e-323".into(),
        "123.456".into(),
        "-123.456".into(),
        "1024".into(),
        "1000000".into(),
    ];
    let mut rng = Rng::new(0x6890_000a);
    for _ in 0..24 {
        vals.push(format!("{}", rng.range(-1_000_000, 1_000_000)));
    }
    for _ in 0..24 {
        let m = rng.range(1, 1 << 20);
        let e = rng.range(-30, 30);
        vals.push(format!("{m} * Math.pow(2, {e})"));
    }
    let mut bodies: Vec<String> = Vec::new();
    for v in &vals {
        let mut parts: Vec<String> = Vec::new();
        for r in 2..=36 {
            parts.push(format!("({v}).toString({r})"));
        }
        bodies.push(format!("return [{}].join(' ');", parts.join(", ")));
    }
    diff_bodies(&bodies);
}

/// Rows 697-711 -- `Np_toFixed` / `Np_toExponential` / `Np_toPrecision`:
/// `js_toobject`, the `JS_CNUMBER` check, the documented digit ranges swept one
/// step past each bound in BOTH directions, and the non-finite fallbacks.
/// Row 696 (`numtostr`'s `strchr(buf, 'e') == NULL`) comes along for free.
#[test]
fn t_number_precision_ranges() {
    let mut cases: Vec<(String, String)> = Vec::new();
    // rows 697 / 702 / 707: js_toobject on a non-coercible `this`
    for m in ["toFixed", "toExponential", "toPrecision"] {
        for (recv, want) in [
            ("undefined", "cannot convert undefined to object"),
            ("null", "cannot convert null to object"),
        ] {
            for form in call_spellings(recv, &format!("Number.prototype.{m}"), &["2"]) {
                cases.push((form, format!("THREW:TypeError: {want}")));
            }
            cases.push((
                format!("Number.prototype.{m}.call({recv})"),
                format!("THREW:TypeError: {want}"),
            ));
        }
        // rows 698 / 703 / 708: coercible but not JS_CNUMBER.  The class check
        // runs AFTER js_tointeger on the width, but BEFORE the range check.
        for recv in ["'x'", "'1'", "true", "({})", "[]", "/re/", "new String('1')", "Math"] {
            cases.push((
                format!("Number.prototype.{m}.call({recv}, 2)"),
                "THREW:TypeError: not a number".to_string(),
            ));
            cases.push((
                format!("Number.prototype.{m}.call({recv}, -1)"),
                "THREW:TypeError: not a number".to_string(),
            ));
            cases.push((
                format!("Number.prototype.{m}.call({recv}, 99)"),
                "THREW:TypeError: not a number".to_string(),
            ));
        }
        cases.push((
            format!(
                "Number.prototype.{m}.call('x', \
                 {{valueOf: function(){{ throw new Error('w!') }}}})"
            ),
            "THREW:Error: w!".to_string(),
        ));
    }
    // rows 699 / 700 / 704 / 705 / 709 / 710: one step past each bound, both ways
    for (m, lo, hi) in [
        ("toFixed", 0i64, 20i64),
        ("toExponential", 0, 20),
        ("toPrecision", 1, 21),
    ] {
        for w in [lo - 3, lo - 2, lo - 1, lo, lo + 1, hi - 1, hi, hi + 1, hi + 2, hi + 3] {
            let expr = format!("(1).{m}({w})");
            if w < lo || w > hi {
                cases.push((
                    expr,
                    format!("THREW:RangeError: precision {w} out of range"),
                ));
            } else {
                // the accepting cases are diffed below, not hard-coded here
                cases.push((format!("typeof {expr}"), ok_str("string")));
            }
        }
        // non-integral / non-numeric widths funnel through jsV_numbertointeger
        for (arg, w) in [
            ("-0.5", 0i64),
            ("0.5", 0),
            ("-1.5", -1),
            ("20.5", 20),
            ("21.5", 21),
            ("22.5", 22),
            ("NaN", 0),
            ("Infinity", 2147483647),
            ("-Infinity", -2147483648),
            ("1e30", 2147483647),
            ("-1e30", -2147483648),
            ("'3'", 3),
            ("'x'", 0),
            ("null", 0),
            ("true", 1),
            ("false", 0),
            ("undefined", 0),
            ("({})", 0),
            ("[]", 0),
            ("[5]", 5),
        ] {
            let expr = format!("(1).{m}({arg})");
            if w < lo || w > hi {
                cases.push((
                    expr,
                    format!("THREW:RangeError: precision {w} out of range"),
                ));
            } else {
                cases.push((format!("typeof {expr}"), ok_str("string")));
            }
        }
        cases.push((
            format!("(1).{m}()"),
            match m {
                "toPrecision" => "THREW:RangeError: precision 0 out of range".to_string(),
                "toExponential" => ok_str("1e+0"),
                _ => ok_str("1"),
            },
        ));
    }
    let refs: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    expect_exprs(&refs);

    // rows 701 / 706 / 711 plus row 696: the exact formatted output for every
    // in-range width over the whole interesting value space.
    let mut vals: Vec<String> = vec![
        "0".into(),
        "-0".into(),
        "1".into(),
        "-1".into(),
        "0.5".into(),
        "1.5".into(),
        "2.5".into(),
        "-1.5".into(),
        "1/3".into(),
        "0.1".into(),
        "1e-7".into(),
        "1e-21".into(),
        "1e20".into(),
        "1e21".into(),
        "-1e21".into(),
        "1e21-1".into(),
        "999999999999999999999".into(),
        "1e-323".into(),
        "5e-324".into(),
        "1.7976931348623157e308".into(),
        "0/0".into(),
        "1/0".into(),
        "-1/0".into(),
        "123.456".into(),
        "-123.456".into(),
        "9007199254740991".into(),
    ];
    let mut rng = Rng::new(0x7010_000b);
    for _ in 0..16 {
        vals.push(format!("{}", rng.range(-100000, 100000)));
    }
    for _ in 0..16 {
        let m = rng.range(1, 1 << 30);
        let e = rng.range(-25, 25);
        vals.push(format!("{m} * Math.pow(10, {e})"));
    }
    let mut bodies: Vec<String> = Vec::new();
    for v in &vals {
        let mut parts: Vec<String> = Vec::new();
        for w in 0..=20 {
            parts.push(format!("({v}).toFixed({w})"));
        }
        bodies.push(format!("return [{}].join(' ');", parts.join(", ")));
        let mut parts: Vec<String> = Vec::new();
        for w in 0..=20 {
            parts.push(format!("({v}).toExponential({w})"));
        }
        bodies.push(format!("return [{}].join(' ');", parts.join(", ")));
        let mut parts: Vec<String> = Vec::new();
        for w in 1..=21 {
            parts.push(format!("({v}).toPrecision({w})"));
        }
        bodies.push(format!("return [{}].join(' ');", parts.join(", ")));
        bodies.push(format!(
            "return ({v}).toFixed() + '|' + ({v}).toExponential() + '|' + \
             ({v}).toFixed(undefined) + '|' + ({v}).toExponential(undefined);"
        ));
    }
    diff_bodies(&bodies);
}

/* =========================================================================
 *  jsboolean.c
 * ========================================================================= */

/// Rows 712-715 -- `Bp_toString` / `Bp_valueOf`: `js_toobject(J, 0)` and the
/// `JS_CBOOLEAN` class check.
#[test]
fn t_boolean_prototype() {
    let mut cases: Vec<(String, String)> = Vec::new();
    for m in ["toString", "valueOf"] {
        for (recv, want) in [
            ("undefined", "cannot convert undefined to object"),
            ("null", "cannot convert null to object"),
        ] {
            for form in call_spellings(recv, &format!("Boolean.prototype.{m}"), &[]) {
                cases.push((form, format!("THREW:TypeError: {want}")));
            }
        }
        for recv in [
            "1",
            "0",
            "''",
            "'true'",
            "({})",
            "[]",
            "(function(){})",
            "/re/",
            "new Number(1)",
            "new String('true')",
            "new Date(0)",
            "Math",
            "JSON",
        ] {
            cases.push((
                format!("Boolean.prototype.{m}.call({recv})"),
                "THREW:TypeError: not a boolean".to_string(),
            ));
        }
    }
    // the accepting cases: a primitive boolean is boxed by js_toobject
    cases.push(("Boolean.prototype.toString.call(true)".into(), ok_str("true")));
    cases.push(("Boolean.prototype.toString.call(false)".into(), ok_str("false")));
    cases.push((
        "Boolean.prototype.toString.call(new Boolean(true))".into(),
        ok_str("true"),
    ));
    cases.push((
        "Boolean.prototype.toString.call(Boolean.prototype)".into(),
        ok_str("false"),
    ));
    cases.push(("Boolean.prototype.valueOf.call(true)".into(), "OK:boolean(true)".into()));
    cases.push((
        "Boolean.prototype.valueOf.call(new Boolean(false))".into(),
        "OK:boolean(false)".into(),
    ));
    cases.push((
        "Boolean.prototype.valueOf.call(Boolean.prototype)".into(),
        "OK:boolean(false)".into(),
    ));
    let refs: Vec<(&str, &str)> = cases.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    expect_exprs(&refs);
    diff_bodies(&[
        body("String(Boolean())"),
        body("String(Boolean(undefined))"),
        body("String(new Boolean().valueOf())"),
        body("Object.prototype.toString.call(new Boolean())"),
        body("String(new Boolean(0).valueOf()) + String(new Boolean('0').valueOf())"),
    ]);
}

/* =========================================================================
 *  jsbuiltin.c
 * ========================================================================= */

unsafe extern "C" fn cf_noop(j: JS) {
    cur().js_pushundefined(j);
}

/// Row 716 -- `jsB_propf`'s `strrchr(name, '.') == NULL` fork (jsbuiltin.c:12).
/// Every `jsB_propf` call inside the library passes a dotted name, so the fork
/// is only reachable through the exported symbol.  The property KEY is the text
/// after the last dot (or the whole name), while `u.c.name` -- what
/// `Function.prototype.toString` prints -- is always the FULL name.
///
/// A NULL `name` is NOT driven: `strrchr(NULL, '.')` is an unconditional NULL
/// dereference in the C, i.e. undefined behaviour rather than a rejection.
#[test]
fn t_builtin_propf_names() {
    for name in [
        "nodot",
        "",
        ".",
        "..",
        "a.b",
        "a.b.c",
        "trailing.",
        ".leading",
        "A.prototype.m",
        "x.y.",
        "with space",
        "with.space here",
        "0",
        "a..b",
    ] {
        for n in [-1i32, 0, 1, 7, i32::MAX, i32::MIN] {
            probe_state(&format!("jsB_propf {name:?} n={n}"), 0, move || {
                let nm = cstr(name);
                job!(|l, j| {
                    let propf: unsafe extern "C" fn(JS, *const c_char, js_CFunction, c_int) =
                        l.raw2("jsB_propf");
                    l.js_newobject(j);
                    propf(j, nm.as_ptr(), Some(cf_noop), n);
                    // enumerate the keys the object ended up with, and read the
                    // function object back out through the computed key
                    let mut keys: Vec<String> = Vec::new();
                    l.js_pushiterator(j, -1, 1);
                    loop {
                        let k = l.js_nextiterator(j, -1);
                        if k.is_null() {
                            break;
                        }
                        keys.push(from_c(k));
                    }
                    l.js_pop(j, 1);
                    // JS_DONTENUM hides it from the iterator, so ask directly
                    let want = match name.rfind('.') {
                        Some(i) => &name[i + 1..],
                        None => name,
                    };
                    let wk = cstr(want);
                    let has = l.js_hasproperty(j, -1, wk.as_ptr());
                    let mut shape = String::new();
                    if has != 0 {
                        shape = format!(
                            " ty={} len={}",
                            from_c(l.js_typeof(j, -1)),
                            {
                                l.js_getproperty(j, -1, cn!("length"));
                                let v = l.js_tonumber(j, -1);
                                l.js_pop(j, 1);
                                v
                            }
                        );
                        // Function.prototype.toString prints u.c.name, the FULL name
                        l.js_getglobal(j, cn!("Function"));
                        l.js_getproperty(j, -1, cn!("prototype"));
                        l.js_getproperty(j, -1, cn!("toString"));
                        l.js_copy(j, -4);
                        let rc = l.js_pcall(j, 0);
                        shape.push_str(&format!(
                            " fts_rc={rc} fts={}",
                            from_c(l.js_tryrepr(j, -1, ERRSTR))
                        ));
                        l.js_pop(j, 3);
                        l.js_pop(j, 1);
                    }
                    format!("enum={keys:?} key={want:?} has={has}{shape}")
                })
            });
        }
    }
}

/// Rows 717-721 -- `jsB_parseInt` / `jsB_parseFloat`.
#[test]
fn t_builtin_parseint_parsefloat() {
    let strs: &[&str] = &[
        "''",
        "'0'",
        "'10'",
        "'zzz'",
        "'0x'",
        "'0x10'",
        "'0X10'",
        "'0xzz'",
        "'  0x10'",
        "'-0x10'",
        "'+0x10'",
        "'-10'",
        "'+10'",
        "'--10'",
        "' \\t\\n\\r10 '",
        "'10abc'",
        "'abc10'",
        "'1.9'",
        "'.5'",
        "'1e3'",
        "'Infinity'",
        "'+Infinity'",
        "'-Infinity'",
        "'InfinityX'",
        "'infinity'",
        "'NaN'",
        "'0b11'",
        "'0o17'",
        "'z'",
        "'Z'",
        "'zz'",
        "'99999999999999999999999999'",
        "'-'",
        "'+'",
        "'.'",
        "'e5'",
        "1",
        "1.9",
        "-1.9",
        "0",
        "null",
        "undefined",
        "true",
        "({})",
        "[10]",
        "[]",
        "1e21",
        "1e-7",
    ];
    let radices: &[&str] = &[
        "undefined",
        "0",
        "1",
        "2",
        "8",
        "10",
        "16",
        "36",
        "37",
        "38",
        "-1",
        "-16",
        "1.9",
        "2.9",
        "16.9",
        "NaN",
        "Infinity",
        "-Infinity",
        "'16'",
        "'x'",
        "null",
        "true",
        "false",
        "({})",
        "2147483648",
        "-2147483649",
    ];
    let mut bodies: Vec<String> = Vec::new();
    for s in strs {
        for r in radices {
            bodies.push(format!("return String(parseInt({s}, {r}));"));
        }
        bodies.push(format!(
            "return String(parseInt({s})) + '/' + String(parseFloat({s}));"
        ));
    }
    bodies.push("return String(parseInt()) + '/' + String(parseFloat());".into());
    diff_bodies(&bodies);
    expect_exprs(&[
        // row 717 / 718: an absent radix auto-detects the 0x prefix
        ("parseInt('0x1f')", "OK:num(31)"),
        ("parseInt('0X1f')", "OK:num(31)"),
        // an EXPLICIT radix never skips the 0x prefix (jsbuiltin.c:46 only runs
        // when radix == 0), so js_strtol stops at the 'x' after reading "0"
        ("parseInt('0x1f', 16)", "OK:num(0)"),
        ("parseInt('0x1f', 10)", "OK:num(0)"),
        ("parseInt('-0x10')", "OK:num(-16)"),
        // row 719: an explicit out-of-range radix is NaN, not an error
        ("parseInt('10', 1)", "OK:num(NaN)"),
        ("parseInt('10', 0)", "OK:num(10)"),
        ("parseInt('10', 37)", "OK:num(NaN)"),
        ("parseInt('10', -1)", "OK:num(NaN)"),
        // row 720: js_strtol consumed nothing
        ("parseInt('zzz')", "OK:num(NaN)"),
        ("parseInt('')", "OK:num(NaN)"),
        ("parseInt('-')", "OK:num(NaN)"),
        ("parseInt('0x')", "OK:num(NaN)"),
        // row 721: js_stringtofloat consumed nothing
        ("parseFloat('abc')", "OK:num(NaN)"),
        ("parseFloat('')", "OK:num(NaN)"),
        ("parseFloat('.')", "OK:num(NaN)"),
        ("parseFloat('e5')", "OK:num(NaN)"),
    ]);
}

/// Rows 723 / 724 / 726-729 -- `Encode` and `Decode`.
///
/// The ONLY two URIErrors the pair can raise are the truncated / invalid escape
/// sequences of jsbuiltin.c:145 / :149.  `Encode` walks RAW BYTES and never
/// validates UTF-8, and `Decode` never validates the byte stream it produces,
/// so all of ES5's lone-surrogate URIErrors are ABSENT -- this test asserts that
/// explicitly by feeding lone surrogates and malformed sequences through both.
#[test]
fn t_builtin_uri_errors() {
    expect_exprs(cases![
        // row 726: '%' with fewer than two bytes left
        ("decodeURI('%')", "THREW:URIError: truncated escape sequence"),
        ("decodeURI('%A')", "THREW:URIError: truncated escape sequence"),
        ("decodeURI('%4')", "THREW:URIError: truncated escape sequence"),
        ("decodeURI('abc%')", "THREW:URIError: truncated escape sequence"),
        ("decodeURI('abc%4')", "THREW:URIError: truncated escape sequence"),
        ("decodeURI('%41%')", "THREW:URIError: truncated escape sequence"),
        ("decodeURIComponent('%')", "THREW:URIError: truncated escape sequence"),
        ("decodeURIComponent('%A')", "THREW:URIError: truncated escape sequence"),
        // row 727: a non-hex digit in either position
        ("decodeURI('%zz')", "THREW:URIError: invalid escape sequence"),
        ("decodeURI('%4z')", "THREW:URIError: invalid escape sequence"),
        ("decodeURI('%z4')", "THREW:URIError: invalid escape sequence"),
        ("decodeURI('%%41')", "THREW:URIError: invalid escape sequence"),
        ("decodeURI('%  ')", "THREW:URIError: invalid escape sequence"),
        ("decodeURI('% 1')", "THREW:URIError: invalid escape sequence"),
        ("decodeURI('%-1')", "THREW:URIError: invalid escape sequence"),
        ("decodeURI('%1g')", "THREW:URIError: invalid escape sequence"),
        ("decodeURI('%G1')", "THREW:URIError: invalid escape sequence"),
        ("decodeURIComponent('%zz')", "THREW:URIError: invalid escape sequence"),
        // hex digits ARE case-insensitive
        ("decodeURI('%4a')", ok_str("J")),
        ("decodeURI('%4A')", ok_str("J")),
        // row 724 / 729: empty input pushes "" through `sb ? sb->s : ""`
        ("Q(encodeURI(''))", ok_str("\"\"")),
        ("Q(encodeURIComponent(''))", ok_str("\"\"")),
        ("Q(decodeURI(''))", ok_str("\"\"")),
        ("Q(decodeURIComponent(''))", ok_str("\"\"")),
        // row 728: a decoded byte in the reserved set is re-emitted as an escape
        ("decodeURI('%3B%2F%3F%3A%40%26%3D%2B%24%2C%23')", ok_str("%3B%2F%3F%3A%40%26%3D%2B%24%2C%23")),
        ("decodeURIComponent('%3B%2F%3F%3A%40%26%3D%2B%24%2C%23')", ok_str(";/?:@&=+$,#")),
        ("decodeURI('%3b')", ok_str("%3b")),
        ("decodeURI('%41')", ok_str("A")),
        // row 723: the unescaped sets differ between encodeURI and
        // encodeURIComponent
        ("encodeURI(';/?:@&=+$,#')", ok_str(";/?:@&=+$,#")),
        ("encodeURIComponent(';/?:@&=+$,#')", ok_str("%3B%2F%3F%3A%40%26%3D%2B%24%2C%23")),
        ("encodeURI(' ')", ok_str("%20")),
        ("encodeURI('-_.!~*\\'()')", ok_str("-_.!~*'()")),
        // NO URIError for lone surrogates: runetochar encodes 0xD800 as the
        // three bytes ED A0 80 and Encode just percent-escapes them
        ("encodeURIComponent(String.fromCharCode(0xd800))", ok_str("%ED%A0%80")),
        ("encodeURIComponent(String.fromCharCode(0xdc00))", ok_str("%ED%B0%80")),
        ("encodeURIComponent(String.fromCharCode(0xdfff))", ok_str("%ED%BF%BF")),
        ("encodeURI(String.fromCharCode(0xd800))", ok_str("%ED%A0%80")),
        (
            "encodeURIComponent(String.fromCharCode(0xdc00) + String.fromCharCode(0xd800))",
            ok_str("%ED%B0%80%ED%A0%80"),
        ),
        // ... and no URIError when Decode PRODUCES a malformed byte stream
        ("decodeURIComponent('%80').length", "OK:num(1)"),
        ("decodeURIComponent('%FF').length", "OK:num(1)"),
        ("decodeURIComponent('%C0%80').length", "OK:num(1)"),
        ("decodeURIComponent('%ED%A0%80').length", "OK:num(1)"),
        ("decodeURIComponent('%E0').length", "OK:num(1)"),
        ("decodeURIComponent('%F4%90%80%80').length", "OK:num(4)"),
    ]);
    // round trips over a broad byte / rune space
    let mut bodies: Vec<String> = Vec::new();
    for s in [
        "''",
        "'abc'",
        "'a b'",
        "';/?:@&=+$,#'",
        "'-_.!~*\\'()'",
        "'%'",
        "'\\u00e9'",
        "'\\u20ac'",
        "String.fromCharCode(128512)",
        "String.fromCharCode(0xd800)",
        "String.fromCharCode(0xdfff)",
        "(String.fromCharCode(0xd83d) + String.fromCharCode(0xde00))",
        "'\\u0000x'",
        "'ABCabc012'",
    ] {
        bodies.push(format!(
            "var s = {s}; \
             var eu = 'X', euc = 'X', du = 'X', duc = 'X'; \
             try {{ eu = encodeURI(s) }} catch (e) {{ eu = 'E:' + X(e) }} \
             try {{ euc = encodeURIComponent(s) }} catch (e) {{ euc = 'E:' + X(e) }} \
             try {{ du = decodeURI(s) }} catch (e) {{ du = 'E:' + X(e) }} \
             try {{ duc = decodeURIComponent(s) }} catch (e) {{ duc = 'E:' + X(e) }} \
             return Q(eu) + '|' + Q(euc) + '|' + Q(du) + '|' + Q(duc);"
        ));
        bodies.push(format!(
            "var s = {s}; \
             try {{ return Q(decodeURI(encodeURI(s))) + '/' + \
               Q(decodeURIComponent(encodeURIComponent(s))) }} \
             catch (e) {{ return 'E:' + X(e) }}"
        ));
    }
    // randomised percent sequences, valid and not
    let mut rng = Rng::new(0x7260_000c);
    for _ in 0..120 {
        let n = 1 + rng.below(6);
        let mut s = String::new();
        for _ in 0..n {
            match rng.below(6) {
                0 => s.push('%'),
                1 => s.push_str(&format!("%{:02X}", rng.below(256))),
                2 => s.push_str(&format!("%{:02x}", rng.below(256))),
                3 => s.push_str("%zz"),
                4 => s.push_str(&format!("%{}", (b'a' + rng.below(26) as u8) as char)),
                _ => s.push((0x21 + rng.below(0x5e) as u8) as char),
            }
        }
        let q = s.replace('\\', "\\\\").replace('\'', "\\'");
        bodies.push(format!(
            "var s = '{q}'; \
             var a, b; \
             try {{ a = Q(decodeURI(s)) }} catch (e) {{ a = 'E:' + X(e) }} \
             try {{ b = Q(decodeURIComponent(s)) }} catch (e) {{ b = 'E:' + X(e) }} \
             return a + '|' + b + '|' + Q(encodeURI(s)) + '|' + Q(encodeURIComponent(s));"
        ));
    }
    diff_bodies(&bodies);
}

/// Rows 723 / 727 / 728 through the FFI: raw, deliberately malformed UTF-8 byte
/// strings pushed with `js_pushlstring`, so `Encode` / `Decode` see byte
/// sequences no JS source literal can produce.  Neither validates UTF-8, so the
/// only errors possible remain the two escape-sequence URIErrors.
#[test]
fn t_builtin_uri_raw_bytes() {
    let mut inputs: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xbf],
        vec![0xc0, 0x80],
        vec![0xc2],
        vec![0xe0, 0x80],
        vec![0xe0, 0x80, 0x80],
        vec![0xed, 0xa0, 0x80],
        vec![0xf0, 0x80, 0x80, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![0xf8, 0x88, 0x80, 0x80, 0x80],
        vec![0xfe],
        vec![0xff, 0xfe],
        vec![b'a', 0xff, b'b'],
        vec![b'%', 0xff, 0xff],
        vec![b'%', b'4', 0xff],
        vec![b'%', 0x80],
        vec![b'%', b'2', b'0', 0xff],
    ];
    let mut rng = Rng::new(0x7230_000d);
    for _ in 0..40 {
        inputs.push(rng.raw_bytes(10));
    }
    for bytes in inputs {
        for f in [
            "encodeURI",
            "encodeURIComponent",
            "decodeURI",
            "decodeURIComponent",
        ] {
            let b = bytes.clone();
            probe_state(&format!("{f} raw {b:02x?}"), 0, move || {
                let b2 = b.clone();
                job!(|l, j| {
                    let name = cstr(f);
                    l.js_getglobal(j, name.as_ptr());
                    l.js_pushundefined(j);
                    l.js_pushlstring(j, b2.as_ptr() as *const c_char, b2.len() as c_int);
                    let rc = l.js_pcall(j, 1);
                    let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
                    let ty = from_c(l.js_typeof(j, -1));
                    format!("rc={rc} ty={ty} v={v}")
                })
            });
        }
    }
}

/// Row 730 -- `jsB_init`'s `js_regcompx(J->alloc, J->actx, "(?:)", 0, NULL)`.
/// The error-out parameter is NULL, so `regcompx`'s `if (errorp) *errorp = ...`
/// (regexp.c:904) drops the diagnostic and the caller only sees a NULL `Reprog`,
/// which `jsB_init` never checks.  The built-in pattern `"(?:)"` always compiles,
/// so `RegExp.prototype->u.r.prog` is in fact never NULL -- shown here by
/// actually executing it -- and the NULL-errorp behaviour itself is driven
/// through the exported `js_regcompx`.
#[test]
fn t_builtin_regexp_prototype_prog() {
    expect_exprs(cases![
        ("RegExp.prototype.source", ok_str("(?:)")),
        ("String(RegExp.prototype.global)", ok_str("false")),
        ("String(RegExp.prototype.lastIndex)", ok_str("0")),
        ("A(RegExp.prototype.exec(''))", ok_str("[str#0\"\"]#1")),
        ("A(RegExp.prototype.exec('abc'))", ok_str("[str#0\"\"]#1")),
        ("String(RegExp.prototype.test('abc'))", ok_str("true")),
        ("A('abc'.match(RegExp.prototype))", ok_str("[str#0\"\"]#1")),
        ("String('abc'.search(RegExp.prototype))", ok_str("0")),
        ("Q('abc'.replace(RegExp.prototype, '-'))", ok_str("\"-abc\"")),
        ("A('abc'.split(RegExp.prototype))", ok_str("[str#1\"a\",str#1\"b\",str#1\"c\"]#3")),
        ("Object.prototype.toString.call(RegExp.prototype)", ok_str("[object RegExp]")),
    ]);
    // and js_regcompx with errorp == NULL, over good and bad patterns
    for pat in [
        "(?:)",
        "",
        "a",
        "(",
        "[",
        "a{2,1}",
        "*",
        "\\",
        "(?<x>a)",
        "a**",
        "(((((((((((((((((a)))))))))))))))))",
    ] {
        for null_errorp in [false, true] {
            probe_state(
                &format!("js_regcompx {pat:?} nullerr={null_errorp}"),
                0,
                move || {
                    let p = cstr(pat);
                    job!(|l, j| {
                        let _ = j;
                        let mut err: *const c_char = cn!("<untouched>");
                        let ep = if null_errorp {
                            std::ptr::null_mut()
                        } else {
                            &mut err as *mut *const c_char
                        };
                        let prog = l.js_regcompx(Some(test_alloc), std::ptr::null_mut(), p.as_ptr(), 0, ep);
                        let r = format!("prog_null={} err={}", prog.is_null(), from_c(err));
                        if !prog.is_null() {
                            l.js_regfreex(Some(test_alloc), std::ptr::null_mut(), prog);
                        }
                        r
                    })
                },
            );
        }
    }
}

/// The `regcompx` allocator contract of regexp.c:998-1005: `n == 0` frees and
/// returns NULL, otherwise `realloc`.
unsafe extern "C" fn test_alloc(_ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    extern "C" {
        fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
        fn free(p: *mut c_void);
    }
    if n == 0 {
        if !p.is_null() {
            free(p);
        }
        std::ptr::null_mut()
    } else {
        realloc(p, n as usize)
    }
}

/* =========================================================================
 *  The `js_try` cleanup handlers, reached by exhausting J->memlimit
 * ========================================================================= */

/// Rows 491 / 603 / 628 / 632 / 636 / 652 / 660 / 695 / 722 / 725 -- the ten
/// `js_try` handlers in these six files that free a scratch buffer and rethrow.
/// Several of them (`Sp_toLowerCase`, `Sp_toUpperCase`, `S_fromCharCode`,
/// `Np_toString`, `Encode`, `Decode`) can only be entered through an allocation
/// failure, so the test exhausts `J->memlimit`, which jsrun.c:53-63 implements as
/// a strictly decreasing byte budget -- deterministic, and therefore identical in
/// both libraries at every limit.
///
/// The script is COMPILED before the limit is installed, so the failure always
/// lands inside the builtin under test rather than in the parser.
#[test]
fn t_builtin_oom_try_handlers() {
    let snippets: &[&str] = &[
        // jsstring.c:344 Sp_toLowerCase / jsstring.c:394 Sp_toUpperCase
        "'aBcDeF\\u00df\\u0130\\ufb00'.toLowerCase()",
        "'aBcDeF\\u00df\\u0130\\ufb00'.toUpperCase()",
        "'a'.toLowerCase()",
        // jsstring.c:450 S_fromCharCode
        "String.fromCharCode(65,66,67,68,69,70,71,72,73,74)",
        "String.fromCharCode(128512,128513,128514)",
        // jsnumber.c:81 Np_toString's radix buffer
        "(255).toString(2)",
        "(1e100).toString(36)",
        "(0.1).toString(3)",
        // jsbuiltin.c:105 Encode / jsbuiltin.c:134 Decode
        "encodeURI('a b c \\u00e9 \\u20ac')",
        "encodeURIComponent(';/?:@&=+$,# \\u00e9')",
        "decodeURI('%41%42%43%20%E2%82%AC')",
        "decodeURIComponent('%3B%2F%3F%20%41')",
        // jsstring.c:561 Sp_replace_regexp / jsstring.c:662 Sp_replace_string
        "'aaa'.replace(/a/g,'bbbb')",
        "'aaa'.replace('a','bbbb')",
        "'abcabc'.replace(/(b)/g, '[$1]')",
        // jsarray.c:126 Ap_join
        "[1,2,3,4,5].join('--')",
        "[[1,2],[3,4]].join('|')",
        // jsstring.c:157 Sp_concat
        "'abc'.concat('d','e','f','g')",
        // jsstring.c:253 Sp_substring_imp's fast path (no js_malloc, so the
        // handler is never armed)
        "'abcdef'.substring(1,4)",
    ];
    for s in snippets {
        for lim in [
            1i32, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 1024, 4096,
        ] {
            probe_state(&format!("oom lim={lim} {s}"), 0, move || {
                job!(|l, j| {
                    let src = format!("({s})");
                    let cs = cstr(&src);
                    let rc0 = l.js_ploadstring(j, FILENAME, cs.as_ptr());
                    if rc0 != 0 {
                        return format!("load rc={rc0}");
                    }
                    l.js_pushundefined(j);
                    l.js_setlimit(j, 0, lim);
                    let rc = l.js_pcall(j, 0);
                    l.js_setlimit(j, 0, 0);
                    let ty = from_c(l.js_typeof(j, -1));
                    let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
                    l.js_pop(j, 1);
                    format!("rc={rc} ty={ty} v={v}")
                })
            });
        }
    }
}

/* =========================================================================
 *  The JS_STRLIMIT rows that need a real 256 MiB string
 * ========================================================================= */

/// Rows 493 / 604 / 605 -- the three `JS_STRLIMIT` (1<<28) range checks in
/// `Ap_join` (jsarray.c:148) and `Sp_concat` (jsstring.c:162 / :170).
///
/// All three need a genuine `1 << 28`-byte NUL-terminated buffer, because the
/// checks are on `strlen()` of a real string.  ONE buffer is built and reused for
/// every case and for both libraries, and each case is arranged so the RangeError
/// fires BEFORE the `js_malloc`/`js_realloc` that would double the footprint.
#[test]
fn t_builtin_string_limit_rows() {
    // `js_pushstring` accepts strlen == JS_STRLIMIT and rejects strlen >
    // JS_STRLIMIT (jsrun.c:147-149), so this is the largest string that can
    // exist.
    let big: Vec<u8> = {
        let mut v = vec![b'x'; JS_STRLIMIT as usize];
        v.push(0);
        v
    };
    let bp = big.as_ptr() as usize;
    // one byte shorter, so `1 + strlen` is exactly JS_STRLIMIT and row 604's
    // check does NOT fire
    let bp1 = bp + 1;

    // row 604: `n = 1 + strlen(this) > JS_STRLIMIT`
    let t = probe_state("Sp_concat receiver at JS_STRLIMIT", 0, move || {
        job!(|l, j| {
            l.js_getglobal(j, cn!("String"));
            l.js_getproperty(j, -1, cn!("prototype"));
            l.js_getproperty(j, -1, cn!("concat"));
            l.js_pushstring(j, bp as *const c_char);
            l.js_pushstring(j, cn!("y"));
            let rc = l.js_pcall(j, 1);
            let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
            l.js_pop(j, 1);
            format!("rc={rc} v={v}")
        })
    });
    // the job itself completed (so js_pushstring accepted the buffer) and the
    // protected call inside it is what raised the RangeError
    assert!(
        t.contains("rc=1") && t.contains("RangeError") && t.contains("invalid string length"),
        "row 604: {t}"
    );

    // row 605: the receiver fits, the accumulated total after argument 1 does not
    let t = probe_state("Sp_concat argument pushes past JS_STRLIMIT", 0, move || {
        job!(|l, j| {
            l.js_getglobal(j, cn!("String"));
            l.js_getproperty(j, -1, cn!("prototype"));
            l.js_getproperty(j, -1, cn!("concat"));
            l.js_pushstring(j, cn!("xy"));
            l.js_pushstring(j, bp1 as *const c_char);
            let rc = l.js_pcall(j, 1);
            let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
            l.js_pop(j, 1);
            format!("rc={rc} v={v}")
        })
    });
    // the job itself completed (so js_pushstring accepted the buffer) and the
    // protected call inside it is what raised the RangeError
    assert!(
        t.contains("rc=1") && t.contains("RangeError") && t.contains("invalid string length"),
        "row 605: {t}"
    );

    // row 493: `n + seplen + rlen > JS_STRLIMIT` inside Ap_join's loop, reached
    // with a JS_STRLIMIT-1 byte SEPARATOR and a two-element array
    let t = probe_state("Ap_join separator at JS_STRLIMIT", 0, move || {
        job!(|l, j| {
            l.js_getglobal(j, cn!("Array"));
            l.js_getproperty(j, -1, cn!("prototype"));
            l.js_getproperty(j, -1, cn!("join"));
            let rc0 = push_expr(l, j, "[1,2]");
            assert_eq!(rc0, 0);
            l.js_pushstring(j, bp1 as *const c_char);
            let rc = l.js_pcall(j, 1);
            let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
            l.js_pop(j, 1);
            format!("rc={rc} v={v}")
        })
    });
    // the job itself completed (so js_pushstring accepted the buffer) and the
    // protected call inside it is what raised the RangeError
    assert!(
        t.contains("rc=1") && t.contains("RangeError") && t.contains("invalid string length"),
        "row 493: {t}"
    );

    // a single-element array never reaches the check (k == 0 takes the js_malloc
    // branch at jsarray.c:142 instead), so the row really is input-driven
    let t = probe_state("Ap_join separator unused for one element", 0, move || {
        job!(|l, j| {
            l.js_getglobal(j, cn!("Array"));
            l.js_getproperty(j, -1, cn!("prototype"));
            l.js_getproperty(j, -1, cn!("join"));
            let rc0 = push_expr(l, j, "[1]");
            assert_eq!(rc0, 0);
            l.js_pushstring(j, bp1 as *const c_char);
            let rc = l.js_pcall(j, 1);
            let ty = from_c(l.js_typeof(j, -1));
            let n = l.js_getlength(j, -1);
            l.js_pop(j, 1);
            format!("rc={rc} ty={ty} len={n}")
        })
    });
    assert!(t.contains("rc=0"), "one-element join should succeed: {t}");

    // and js_pushstring itself, one step past the limit in both directions
    for off in [0usize, 1] {
        let p = bp + off;
        probe_state(&format!("js_pushstring strlen=JS_STRLIMIT-{off}"), 0, move || {
            job!(|l, j| {
                l.js_pushstring(j, p as *const c_char);
                let n = l.js_gettop(j);
                let ty = from_c(l.js_typeof(j, -1));
                l.js_getproperty(j, -1, cn!("length"));
                let len = l.js_tonumber(j, -1);
                l.js_pop(j, 1);
                format!("top={n} ty={ty} len={len}")
            })
        });
    }
    drop(big);
}

/* =========================================================================
 *  Generic FFI boundary abuse for the entry points these six files export
 * ========================================================================= */

/// `js_utflen` (jsstring.c:49) and `js_utfptrtoidx` (jsstring.c:73) across the
/// FFI: empty strings, deliberately malformed UTF-8, astral runes, and every
/// interior pointer including one BEFORE the start of the string.
///
/// A NULL `const char *` is NOT driven for either: both dereference `*s`
/// unconditionally on entry, so `js_utflen(NULL)` / `js_utfptrtoidx(NULL, p)` is
/// an unconditional NULL dereference (undefined behaviour), not a rejection.
/// `js_utfptrtoidx` is also only driven with `p` in `[s, s + strlen(s)]`: a `p`
/// beyond the terminator makes `while (s < p)` read past the end of the
/// allocation.
#[test]
fn t_ffi_utflen_utfptrtoidx() {
    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"abc".to_vec(),
        "\u{e9}".as_bytes().to_vec(),
        "a\u{e9}b".as_bytes().to_vec(),
        "\u{20ac}".as_bytes().to_vec(),
        // the `rune >= 0x10000` boundary of js_utflen / js_runeat / js_utfptrtoidx
        "\u{ffff}".as_bytes().to_vec(),
        "\u{10000}".as_bytes().to_vec(),
        "\u{10001}".as_bytes().to_vec(),
        "a\u{10000}b".as_bytes().to_vec(),
        "\u{1f600}".as_bytes().to_vec(),
        "a\u{1f600}b".as_bytes().to_vec(),
        "\u{10ffff}".as_bytes().to_vec(),
        vec![0x80],
        vec![0xbf, 0xbf],
        vec![0xc0, 0x80],
        vec![0xc2],
        vec![0xe0, 0x80, 0x80],
        vec![0xed, 0xa0, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![0xf8, 0x88, 0x80, 0x80, 0x80],
        vec![0xfe, 0xff],
        vec![b'a', 0xff, b'b'],
    ];
    let mut rng = Rng::new(0x7360_000e);
    for _ in 0..30 {
        inputs.push(rng.raw_bytes(12));
    }
    for bytes in inputs {
        let b = bytes.clone();
        probe_state(&format!("utflen {b:02x?}"), 0, move || {
            let b2 = b.clone();
            job!(|l, j| {
                let _ = j;
                // a two-byte prefix so `p < s` can be formed without leaving the
                // allocation
                let mut buf: Vec<u8> = vec![b'#', b'#'];
                buf.extend_from_slice(&b2);
                buf.push(0);
                let s = unsafe { buf.as_ptr().add(2) } as *const c_char;
                let mut r = format!("utflen={}", l.js_utflen(s));
                for k in 0..=(b2.len() + 1) {
                    let p = unsafe { s.add(k) };
                    r.push_str(&format!(" i[{k}]={}", l.js_utfptrtoidx(s, p)));
                }
                // p strictly BEFORE s: `while (s < p)` never runs
                for back in 1..=2usize {
                    let p = unsafe { s.sub(back) };
                    r.push_str(&format!(" i[-{back}]={}", l.js_utfptrtoidx(s, p)));
                }
                r
            })
        });
    }
}

/// `jsB_propn` (jsbuiltin.c:18) and `jsB_props` (jsbuiltin.c:24) across the FFI:
/// unlike `jsB_propf` these use `name` verbatim as the property key, with no
/// `strrchr`.  Driven with dotted / empty / index-shaped keys, out-of-range
/// doubles, and with a NON-OBJECT at `-2` so `js_defproperty`'s `js_toobject`
/// (jsrun.c) sees a transient receiver.
#[test]
fn t_ffi_propn_props() {
    let names: &[&str] = &["x", "", "a.b", "0", "1", "length", "toString", "-1", "1.5"];
    let recvs: &[&str] = &["object", "array", "number", "string", "undefined", "null"];
    for name in names.iter().copied() {
        for recv in recvs.iter().copied() {
            for which in ["propn", "props"] {
                probe_state(
                    &format!("jsB_{which} {name:?} on {recv}"),
                    0,
                    move || {
                        let nm = cstr(name);
                        job!(|l, j| {
                            match recv {
                                "object" => l.js_newobject(j),
                                "array" => l.js_newarray(j),
                                "number" => l.js_pushnumber(j, 7.0),
                                "string" => l.js_pushstring(j, cn!("recv")),
                                "undefined" => l.js_pushundefined(j),
                                _ => l.js_pushnull(j),
                            }
                            if which == "propn" {
                                let f: unsafe extern "C" fn(JS, *const c_char, f64) =
                                    l.raw2("jsB_propn");
                                f(j, nm.as_ptr(), -1.5);
                            } else {
                                let f: unsafe extern "C" fn(JS, *const c_char, *const c_char) =
                                    l.raw2("jsB_props");
                                f(j, nm.as_ptr(), cn!("VAL"));
                            }
                            let has = l.js_hasproperty(j, -1, nm.as_ptr());
                            let mut r = format!(
                                "top={} ty={} has={has}",
                                l.js_gettop(j),
                                from_c(l.js_typeof(j, -1))
                            );
                            if has != 0 {
                                r.push_str(&format!(
                                    " v={}",
                                    from_c(l.js_tryrepr(j, -1, ERRSTR))
                                ));
                                l.js_pop(j, 1);
                            }
                            r
                        })
                    },
                );
            }
        }
    }
}

/// `jsB_propf` with out-of-range `int` lengths and a NULL `js_CFunction`, and
/// the five `jsB_init*` entry points invoked a SECOND time on a live state.
///
/// `jsB_init` itself is NOT re-invoked: it replaces every `J->*_prototype`
/// pointer with a fresh object (jsbuiltin.c:196-215) while the previous
/// prototypes are still the `prototype` of every object the state already
/// created, so a second call leaves a state that no longer satisfies its own
/// invariants -- a deliberate corruption rather than an input rejection.
#[test]
fn t_ffi_reinit_builtins() {
    for n in [i32::MIN, -1, 0, 1, 255, 65536, i32::MAX] {
        probe_state(&format!("jsB_propf NULL cfun len={n}"), 0, move || {
            job!(|l, j| {
                let propf: unsafe extern "C" fn(JS, *const c_char, js_CFunction, c_int) =
                    l.raw2("jsB_propf");
                l.js_newobject(j);
                propf(j, cn!("z.m"), None, n);
                let has = l.js_hasproperty(j, -1, cn!("m"));
                let mut r = format!("has={has} top={}", l.js_gettop(j));
                if has != 0 {
                    r.push_str(&format!(" ty={}", from_c(l.js_typeof(j, -1))));
                    l.js_getproperty(j, -1, cn!("length"));
                    r.push_str(&format!(" len={}", l.js_tonumber(j, -1)));
                    l.js_pop(j, 2);
                }
                r
            })
        });
    }
    for init in [
        "jsB_initarray",
        "jsB_initobject",
        "jsB_initstring",
        "jsB_initnumber",
        "jsB_initboolean",
    ] {
        for times in [1, 2] {
            probe_state(&format!("{init} x{times}"), 0, move || {
                job!(|l, j| {
                    let base = l.js_gettop(j);
                    for _ in 0..times {
                        l.nullary(init, j);
                    }
                    let after = l.js_gettop(j);
                    // and the builtins still work afterwards
                    let rc = push_expr(
                        l,
                        j,
                        "[3,1,2].sort().join('|') + '/' + 'aB'.toLowerCase() + '/' + \
                         (255).toString(16) + '/' + String(new Boolean(1)) + '/' + \
                         Object.keys({a:1}).join('') + '/' + Array.isArray([])",
                    );
                    let v = from_c(l.js_tryrepr(j, -1, ERRSTR));
                    format!("top {base}->{after} rc={rc} v={v}")
                })
            });
        }
    }
}

/// Rows 650 / 664 -- the `js_toregexp(J, 1)` TypeError inside
/// `Sp_replace_regexp` (jsstring.c:552) and `Sp_split_regexp` (jsstring.c:726).
///
/// Both call sites are DEAD: the two functions are `static` and are only entered
/// from `Sp_replace` (jsstring.c:710) and `Sp_split` (jsstring.c:824), each
/// behind an `js_isregexp(J, 1)` test, and `js_isregexp` (jsrun.c:123) accepts
/// exactly what `js_toregexp` accepts.  What IS driveable is the exported
/// `js_toregexp` itself, whose TypeError `"not a regexp"` (jsrun.c:373) is the
/// error those two rows would raise -- so the message and constructor are pinned
/// here, and the guard that makes the call sites unreachable is pinned by showing
/// that `replace` / `split` route every non-regexp argument to the string helper
/// instead.
#[test]
fn t_ffi_toregexp_not_a_regexp() {
    let shapes: &[&str] = &[
        "undefined",
        "null",
        "true",
        "0",
        "1.5",
        "''",
        "'/re/'",
        "({})",
        "[]",
        "(function(){})",
        "new String('re')",
        "new Number(1)",
        "new Date(0)",
        "new Error('e')",
        "Math",
        "JSON",
        "RegExp",
        "Object.create(RegExp.prototype)",
        "/re/",
        "/re/gim",
        "RegExp.prototype",
        "new RegExp('a')",
    ];
    for src in shapes {
        probe_state(&format!("js_toregexp {src}"), 0, move || {
            job!(|l, j| {
                let rc = push_expr(l, j, src);
                if rc != 0 {
                    return format!("push rc={rc}");
                }
                let isre = l.pred("js_isregexp", j, -1);
                let base = l.js_gettop(j);
                let f: unsafe extern "C" fn(JS, c_int) -> *mut c_void = l.raw2("js_toregexp");
                let p = f(j, -1);
                format!("isregexp={isre} null={} top={base}", p.is_null())
            })
        });
    }
    // and the guard: every non-regexp argument reaches the STRING helper, which
    // never calls js_toregexp at all
    let mut bodies: Vec<String> = Vec::new();
    for src in shapes {
        bodies.push(format!("return Q('a/re/b'.replace({src}, 'X'));"));
        bodies.push(format!("return A('a/re/b'.split({src}));"));
        bodies.push(format!(
            "return String('a/re/b'.replace({src}, function(m){{ return '<'+m+'>' }}));"
        ));
    }
    diff_bodies(&bodies);
    // ... and js_toregexp's message itself, raised from inside a cfunction so it
    // is observable as a real Error object
    let t = probe_state("js_toregexp message", 0, || {
        job!(|l, j| {
            l.js_pushnumber(j, 1.0);
            let f: unsafe extern "C" fn(JS, c_int) -> *mut c_void = l.raw2("js_toregexp");
            let p = f(j, -1);
            format!("unreachable null={}", p.is_null())
        })
    });
    assert!(
        t.contains("TypeError: not a regexp"),
        "js_toregexp on a number: {t}"
    );
}

/// `js_getlength` (jsarray.c:7) and `js_setlength` (jsarray.c:16) with
/// OUT-OF-RANGE stack indices, and `js_setlength`'s `idx < 0 ? idx - 1 : idx`
/// adjustment (jsarray.c:19) at every stack depth.
///
/// `stackidx` (jsrun.c) substitutes a static `undefined` for any index outside
/// `[0, TOP)`, so an out-of-range index is a well-defined
/// TypeError `"cannot convert undefined to object"` from `js_toobject` rather
/// than a memory error.
///
/// NULL `const char *` arguments are NOT driven for the four string entry points
/// these files export (`js_runeat`, `js_utflen`, `js_utfptrtoidx`, and the
/// `name` of `jsB_propf` / `jsB_propn` / `jsB_props`): every one of them
/// dereferences the pointer unconditionally on entry (`*(unsigned char*)s`,
/// `strrchr(name, '.')`, `strcmp(name, ...)`), so a NULL is an unconditional NULL
/// dereference -- undefined behaviour, not a rejection.  The one NULL pointer the
/// C really does test for is `regcompx`'s `errorp` (regexp.c:904), driven in
/// `t_builtin_regexp_prototype_prog`.
#[test]
fn t_ffi_length_index_abuse() {
    let idxs: Vec<c_int> = vec![
        i32::MIN,
        -1000,
        -5,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        1000,
        i32::MAX,
    ];
    for depth in 0..4i32 {
        for idx in idxs.clone() {
            probe_state(&format!("js_getlength depth={depth} idx={idx}"), 0, move || {
                job!(|l, j| {
                    for k in 0..depth {
                        if k == 0 {
                            l.js_newarray(j);
                            l.js_pushnumber(j, 9.0);
                            l.js_setindex(j, -2, 0);
                        } else {
                            l.js_newobject(j);
                            l.js_pushnumber(j, (k + 1) as f64);
                            l.js_setproperty(j, -2, cn!("length"));
                        }
                    }
                    let base = l.js_gettop(j);
                    let n = l.js_getlength(j, idx);
                    format!("base={base} len={n} top={}", l.js_gettop(j))
                })
            });
            for len in [-1i32, 0, 1, 3, (JS_ARRAYLIMIT + 1) as c_int] {
                probe_state(
                    &format!("js_setlength depth={depth} idx={idx} len={len}"),
                    0,
                    move || {
                        job!(|l, j| {
                            for k in 0..depth {
                                if k == 0 {
                                    l.js_newarray(j);
                                } else {
                                    l.js_newobject(j);
                                }
                            }
                            let base = l.js_gettop(j);
                            l.js_setlength(j, idx, len);
                            let after = l.js_gettop(j);
                            let mut r = format!("base={base} top={after}");
                            if after > 0 {
                                r.push_str(&format!(
                                    " ty={} len={}",
                                    from_c(l.js_typeof(j, -1)),
                                    l.js_getlength(j, -1)
                                ));
                            }
                            r
                        })
                    },
                );
            }
        }
    }
}
