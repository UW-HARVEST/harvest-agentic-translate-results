//! Phase B — `CONFIGS.md` sections D (lexer helpers), E (stack / values /
//! conversions), F (objects / properties / arrays) and G (the compile+run
//! pipeline driven through its LOW-LEVEL entry points, not just `js_dostring`).
//!
//! Everything is called through the `.so` exports of both libraries.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

/* ================================================================== */
/*  helpers                                                            */
/* ================================================================== */

/// MuJS stores these pointers WITHOUT copying them
/// (`obj->u.user.tag = tag;` in js_newuserdatax, `obj->u.c.name = name;` in
/// js_newcfunctionx), so anything passed as a userdata tag, a cfunction name or
/// a js_pushliteral string must outlive the state. A `CString` temporary would
/// be freed immediately and the library would then read freed memory.
macro_rules! lit {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

/// Snapshot of one stack slot in an address-independent way.
unsafe fn slot(api: &Api, J: State, idx: c_int) -> String {
    let ty = (api.js_type)(J, idx);
    let tyof = cstr_string((api.js_typeof)(J, idx)).unwrap_or_default();
    let tag = (*(api.js_tovalue)(J, idx)).tag();
    // CAREFUL: js_torepr (and therefore js_tryrepr) does
    //     js_repr(J, idx); js_replace(J, idx<0 ? idx-1 : idx);
    // i.e. it REPLACES the value at `idx` with its string representation.
    // js_tostring can likewise rewrite the slot via jsV_toobject. So always
    // inspect a COPY, never the live slot. (Both paths of js_tryrepr /
    // js_trystring leave the stack at the height it had on entry, so a single
    // js_pop balances the js_copy.)
    (api.js_copy)(J, idx);
    let repr = cstr_string((api.js_tryrepr)(J, -1, cs("<throws>").as_ptr())).unwrap_or_default();
    (api.js_pop)(J, 1);
    (api.js_copy)(J, idx);
    let s = cstr_string((api.js_trystring)(J, -1, cs("<throws>").as_ptr())).unwrap_or_default();
    (api.js_pop)(J, 1);
    format!("t={} typeof={} tag={} repr={} str={}", ty, tyof, tag, repr, s)
}

/// `js_hasproperty` / `js_hasindex` PUSH the value when they return 1 and push
/// nothing when they return 0, so the pop must be conditional. Wrap them.
unsafe fn has_prop(api: &Api, J: State, idx: c_int, name: &str) -> String {
    let n = cs(name);
    let r = (api.js_hasproperty)(J, idx, n.as_ptr());
    if r != 0 {
        let v = slot(api, J, -1);
        (api.js_pop)(J, 1);
        format!("1({})", v)
    } else {
        "0".to_string()
    }
}

unsafe fn has_index(api: &Api, J: State, idx: c_int, i: c_int) -> String {
    let r = (api.js_hasindex)(J, idx, i);
    if r != 0 {
        let v = slot(api, J, -1);
        (api.js_pop)(J, 1);
        format!("1({})", v)
    } else {
        "0".to_string()
    }
}

/// Snapshot the whole stack.
unsafe fn stack_snapshot(api: &Api, J: State) -> Vec<String> {
    let top = (api.js_gettop)(J);
    (0..top).map(|i| slot(api, J, i)).collect()
}

/// A list of JS expressions covering every value type and object class.
const VALUES: &[&str] = &[
    "undefined",
    "null",
    "true",
    "false",
    "0",
    "-0",
    "1",
    "-1",
    "0.5",
    "1/3",
    "NaN",
    "Infinity",
    "-Infinity",
    "1e21",
    "1e-7",
    "2147483647",
    "2147483648",
    "-2147483648",
    "4294967295",
    "4294967296",
    "9007199254740993",
    "''",
    "'a'",
    "'0123456789abcde'",   // 15 bytes: last shrstr
    "'0123456789abcdef'",  // 16 bytes: first memstr
    "'0123456789abcdefg'",
    "'h\\u00e9llo'",
    "'\\u65e5\\u672c\\u8a9e'",
    "'\\ud83d\\ude00'",
    "'12'",
    "' 12 '",
    "'0x1f'",
    "'abc'",
    "({})",
    "[]",
    "[1,2,3]",
    "[1,,3]",
    "(function(){})",
    "(function f(a,b){return a+b})",
    "Math.max",
    "Object",
    "new Boolean(true)",
    "new Number(7)",
    "new String('s')",
    "new Date(1234567890123)",
    "/a(b)c/gi",
    "new Error('e')",
    "new TypeError('t')",
    "Math",
    "JSON",
    "Object.create(null)",
    "({valueOf:function(){return 42}})",
    "({toString:function(){return 'ts'}})",
    "({valueOf:function(){throw new Error('vo')}, toString:function(){throw new Error('ts')}})",
    "(function(){return arguments})(1,2)",
];

/// Push `expr` (evaluated by the library itself) and run `f` on the resulting
/// stack; returns whatever `f` produced plus the prep return code.
unsafe fn with_value<T>(api: &Api, expr: &str, f: impl FnOnce(&Api, State) -> T) -> (c_int, T) {
    let J = new_state(api, 0);
    let prep = format!("var V = ({});", expr);
    let rc = (api.js_dostring)(J, cs(&prep).as_ptr());
    (api.js_getglobal)(J, cs("V").as_ptr());
    let out = f(api, J);
    (api.js_freestate)(J);
    (rc, out)
}

#[track_caller]
fn diff<T: PartialEq + std::fmt::Debug>(label: &str, f: impl Fn(&Api) -> T) {
    let (c, r) = both(|api, _| f(api));
    assert_eq!(c, r, "DIVERGENCE in {}:\n  C   : {:?}\n  Rust: {:?}", label, c, r);
}


/* ------------------------------------------------------------------ */
/*  running an arbitrary C-API sequence inside a PROTECTED frame        */
/* ------------------------------------------------------------------ */
//
// Many exported entry points throw, and an unprotected `js_throw` calls
// `J->panic` and then `abort()` (jsrun.c:1465-...). An external caller cannot
// use the `js_try()` setjmp macro against the Rust cdylib (it models `longjmp`
// with a `panic`), so the only portable way to get a protected frame is to run
// the sequence inside a cfunction that JavaScript calls from a `try{}catch{}`.

type Job = Box<dyn Fn(&Api, State) -> String>;

thread_local! {
    static PROT_API: std::cell::Cell<*const Api> = std::cell::Cell::new(std::ptr::null());
    static PROT_JOB: std::cell::RefCell<Option<Job>> = std::cell::RefCell::new(None);
}

unsafe extern "C-unwind" fn prot_cb(J: State) {
    let api = &*PROT_API.with(|c| c.get());
    // Take the job out so the RefCell is not borrowed while the job runs (the
    // job may unwind straight through this frame).
    let job = PROT_JOB.with(|j| j.borrow_mut().take());
    let msg = match &job {
        Some(f) => f(api, J),
        None => "<no job>".to_string(),
    };
    PROT_JOB.with(|j| *j.borrow_mut() = job);
    let out = cs(&msg);
    (api.js_pushstring)(J, out.as_ptr());
}

/// Run `job` inside a protected frame; returns (js_dostring rc, output bytes).
/// The string `job` returns is printed, so it is part of the compared output.
fn run_protected(
    api: &Api,
    flags: c_int,
    job: impl Fn(&Api, State) -> String + 'static,
) -> (c_int, Vec<u8>) {
    unsafe {
        PROT_API.with(|c| c.set(api as *const Api));
        PROT_JOB.with(|j| *j.borrow_mut() = Some(Box::new(job)));
        out_clear();
        let J = new_state(api, flags);
        (api.js_newcfunction)(J, Some(prot_cb), lit!("prot"), 0);
        (api.js_setglobal)(J, lit!("prot"));
        let rc = (api.js_dostring)(
            J,
            cs("try { print('ok', prot()) } catch (e) { print('caught', (e && e.name) + ': ' + (e && e.message)) }")
                .as_ptr(),
        );
        (api.js_freestate)(J);
        PROT_JOB.with(|j| *j.borrow_mut() = None);
        PROT_API.with(|c| c.set(std::ptr::null()));
        (rc, out_take())
    }
}

/* ================================================================== */
/*  D3 — jsY_findword                                                  */
/* ================================================================== */

#[test]
fn d3_jsY_findword() {
    // jslex.c: binary search over a sorted NUL-terminated word list.
    const WORDS: &[&str] = &[
        "alpha", "beta", "delta", "epsilon", "gamma", "omega", "zeta",
    ];
    let probes: Vec<String> = WORDS
        .iter()
        .map(|s| s.to_string())
        .chain(
            [
                "", "a", "z", "aa", "alph", "alphaa", "ALPHA", "Beta", "zz", "zetaa", "gammaa",
                "omeg", "\u{e9}", "0", "~",
            ]
            .iter()
            .map(|s| s.to_string()),
        )
        .collect();
    diff("jsY_findword", |api| {
        // build the C array of pointers
        let owned: Vec<std::ffi::CString> = WORDS.iter().map(|s| cs(s)).collect();
        let ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        let mut out = Vec::new();
        for p in &probes {
            let needle = cs(p);
            for n in [0usize, 1, 3, WORDS.len()] {
                out.push(unsafe {
                    (api.jsY_findword)(needle.as_ptr(), ptrs.as_ptr(), n as c_int)
                });
            }
        }
        out
    });
}

/* ================================================================== */
/*  D5 — jsY_initlex + jsY_lex / jsY_lexjson token streams             */
/* ================================================================== */

/* The lexer THROWS on malformed input, and an unprotected throw aborts. So the
 * whole init+loop runs inside a cfunction that JS calls from a try/catch, which
 * gives both libraries an identical protected frame. The cfunction returns the
 * joined token stream, so a lexer error shows up as the caught exception. */
thread_local! {
    static LEX_API: std::cell::Cell<*const Api> = std::cell::Cell::new(std::ptr::null());
    static LEX_JOB: std::cell::RefCell<(String, String, bool, usize)> =
        std::cell::RefCell::new((String::new(), String::new(), false, 0));
}

unsafe extern "C-unwind" fn lex_cb(J: State) {
    let api = &*LEX_API.with(|c| c.get());
    let (fname, src, json, limit) = LEX_JOB.with(|j| j.borrow().clone());
    let mut out: Vec<String> = Vec::new();
    let f = cs(&fname);
    let source = cs(&src);
    (api.jsY_initlex)(J, f.as_ptr(), source.as_ptr());
    for _ in 0..limit {
        let t = if json {
            (api.jsY_lexjson)(J)
        } else {
            (api.jsY_lex)(J)
        };
        out.push(format!(
            "{}:{}",
            t,
            cstr_string((api.jsY_tokenstring)(t)).unwrap_or_default()
        ));
        if t == 0 {
            break;
        }
    }
    let joined = cs(&out.join(" "));
    (api.js_pushstring)(J, joined.as_ptr());
}

/// Returns (dostring rc, captured output) for lexing `src`.
fn lex_stream(api: &Api, filename: &str, src: &str, json: bool, limit: usize) -> (c_int, Vec<u8>) {
    unsafe {
        LEX_API.with(|c| c.set(api as *const Api));
        LEX_JOB.with(|j| {
            *j.borrow_mut() = (filename.to_string(), src.to_string(), json, limit)
        });
        out_clear();
        let J = new_state(api, 0);
        (api.js_newcfunction)(J, Some(lex_cb), lit!("lex"), 0);
        (api.js_setglobal)(J, lit!("lex"));
        let rc = (api.js_dostring)(
            J,
            cs("try { print(lex()) } catch (e) { print('caught', e.name + ': ' + e.message) }")
                .as_ptr(),
        );
        (api.js_freestate)(J);
        LEX_API.with(|c| c.set(std::ptr::null()));
        (rc, out_take())
    }
}

#[test]
fn d5_lexer_token_streams() {
    // Only clean-lexing sources here (error sources are covered by errors_js.rs
    // where the exception is properly protected).
    let sources = [
        "",
        " \t\n\r",
        "// line comment",
        "/* block\ncomment */",
        "1 2 3",
        "0 1 12 0x1f 0X1F 1.5 .5 5. 1e10 1E-10 1e+10",
        "'a' \"b\" 'a\\nb' \"\\x41\\u0042\"",
        "a b _c $d e1",
        "if else while for function return var new delete typeof instanceof in this null true false",
        "break case catch continue debugger default do finally switch throw try void with",
        "+ - * / % = == != === !== < > <= >= << >> >>> & | ^ ~ ! && || ++ --",
        "+= -= *= /= %= <<= >>= >>>= &= |= ^=",
        "( ) [ ] { } ; , . : ?",
        "a.b.c",
        "x = /ab+c/gi",
        "a / b / c",
        "\"\\u00e9\\u65e5\"",
        "\u{e9}dentifier",
        "1\n2\r3\u{2028}4\u{2029}5",
        "{a:1,b:[2,3]}",
    ];
    // Malformed sources are safe now: the cfunction runs in a protected frame.
    let error_sources = [
        "0x", "01", "1e", "1a", "\"abc", "\"\\x\"", "/abc", "/a/x", "/a/gg", "/*", "#",
        "\u{20ac}", "\\q", "\"\\", "'unterminated", "1..2", "0b1", "0o7", "@",
    ];
    for src in sources.iter().copied().chain(error_sources.iter().copied()) {
        let (c, r) = both(|api, _| lex_stream(api, "[t]", src, false, 400));
        assert_eq!(
            c,
            r,
            "DIVERGENCE jsY_lex stream for {:?}:\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
            src,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
    let json_sources = [
        "",
        "null",
        "true",
        "false",
        "0",
        "-0",
        "1.5",
        "1e10",
        "-1E-10",
        "\"str\"",
        "\"a\\u0041\\n\\t\\\\\\/\\b\\f\\r\"",
        "[]",
        "[1,2,3]",
        "{}",
        "{\"a\":1}",
        "{\"a\":[1,{\"b\":null}]}",
        " \t\n\r [ 1 , 2 ] ",
    ];
    let json_error_sources = [
        "x", "nul", "tru", "fals", "-", "1.", "1e", "\"\\q\"", "\"abc", "\"\x01\"",
        "\u{20ac}", "[1", "{\"a\"", "01", "+1", "'a'", "NaN",
    ];
    for src in json_sources
        .iter()
        .copied()
        .chain(json_error_sources.iter().copied())
    {
        let (c, r) = both(|api, _| lex_stream(api, "JSON", src, true, 400));
        assert_eq!(
            c,
            r,
            "DIVERGENCE jsY_lexjson stream for {:?}:\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
            src,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
}

/* ================================================================== */
/*  E1..E6 — state construction & context                              */
/* ================================================================== */

#[test]
fn e1_e3_newstate_flags() {
    for flags in [0, JS_STRICT, 2, 3, 4, 7, 8, -1, i32::MAX, i32::MIN] {
        let scripts = [
            "print(typeof this)",
            "x = 1; print(x)",
            "print((function(){ return this === undefined })())",
            "print(eval('1+1'))",
            "function f(){ return typeof this } print(f())",
            "print(delete Object.prototype)",
        ];
        for s in scripts {
            let (c, r) = both(|api, _| run_script(api, flags, s));
            assert_eq!(
                c,
                r,
                "DIVERGENCE js_newstate(flags={}) {:?}:\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
                flags,
                s,
                c.0,
                String::from_utf8_lossy(&c.1),
                r.0,
                String::from_utf8_lossy(&r.1)
            );
        }
    }
}

/* A counting allocator, per side. Two separate statics so the two libraries
 * never share a counter. */
static mut ALLOC_N: [i64; 2] = [0, 0];
static mut ALLOC_SIDE: usize = 0;

unsafe extern "C" fn counting_alloc(_ctx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    ALLOC_N[ALLOC_SIDE] += 1;
    if size == 0 {
        libc::free(ptr);
        std::ptr::null_mut()
    } else {
        libc::realloc(ptr, size as usize)
    }
}

#[test]
fn e4_custom_allocator_and_actx() {
    // Same number of allocator calls and the same actx round trip.
    let (c, r) = both(|api, side| unsafe {
        ALLOC_SIDE = if side == Side::C { 0 } else { 1 };
        ALLOC_N[ALLOC_SIDE] = 0;
        let actx = 0xABCD as *mut c_void;
        let J = (api.js_newstate)(Some(counting_alloc), actx, 0);
        assert!(!J.is_null());
        (api.js_setreport)(J, Some(report_cb));
        bind_callbacks(api);
        (api.js_newcfunction)(J, Some(print_cb), b"print\0".as_ptr() as *const c_char, 1);
        (api.js_setglobal)(J, cs("print").as_ptr());
        out_clear();
        let rc = (api.js_dostring)(
            J,
            cs("var a=[]; for (var i=0;i<50;++i) a.push('s'+i); print(a.length, a.join('').length)")
                .as_ptr(),
        );
        (api.js_gc)(J, 0);
        (api.js_freestate)(J);
        (rc, out_take(), ALLOC_N[ALLOC_SIDE])
    });
    assert_eq!(
        c.0, r.0,
        "custom-allocator dostring rc diverged: C={} Rust={}",
        c.0, r.0
    );
    assert_eq!(
        String::from_utf8_lossy(&c.1),
        String::from_utf8_lossy(&r.1),
        "custom-allocator output diverged"
    );
    assert_eq!(
        c.2, r.2,
        "ALLOCATOR CALL COUNT diverged: C={} Rust={}",
        c.2, r.2
    );
}

#[test]
fn e4b_allocator_failing_at_nth_call_returns_null() {
    // ERRORS.md L18/L19/L20: js_newstate must return NULL (and must not leak or
    // crash) when an allocation fails.
    static mut FAIL_AT: i64 = 0;
    static mut CALLS: i64 = 0;
    unsafe extern "C" fn failing_alloc(
        _ctx: *mut c_void,
        ptr: *mut c_void,
        size: c_int,
    ) -> *mut c_void {
        if size == 0 {
            libc::free(ptr);
            return std::ptr::null_mut();
        }
        CALLS += 1;
        if CALLS == FAIL_AT {
            return std::ptr::null_mut();
        }
        libc::realloc(ptr, size as usize)
    }
    for n in 1..=40i64 {
        let (c, r) = both(|api, _| unsafe {
            FAIL_AT = n;
            CALLS = 0;
            let J = (api.js_newstate)(Some(failing_alloc), std::ptr::null_mut(), 0);
            let isnull = J.is_null();
            if !isnull {
                // If it survived, it must still be a working state.
                FAIL_AT = 0;
                let rc = (api.js_dostring)(J, cs("1+1").as_ptr());
                (api.js_freestate)(J);
                (false, rc)
            } else {
                (true, -1)
            }
        });
        assert_eq!(
            c, r,
            "DIVERGENCE js_newstate with allocation #{} failing: C={:?} Rust={:?}",
            n, c, r
        );
    }
}

#[test]
fn e5_e6_context_and_panic_handler() {
    diff("js_setcontext/js_getcontext/js_atpanic", |api| unsafe {
        let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        let c0 = (api.js_getcontext)(J) as usize;
        (api.js_setcontext)(J, 1 as *mut c_void);
        let c1 = (api.js_getcontext)(J) as usize;
        (api.js_setcontext)(J, std::ptr::null_mut());
        let c2 = (api.js_getcontext)(J) as usize;
        // js_atpanic returns the previous handler; installing None then reading
        // it back must report "there was a handler" then "there was none".
        let had1 = (api.js_atpanic)(J, Some(panic_cb)).is_some();
        let had2 = (api.js_atpanic)(J, None).is_some();
        let had3 = (api.js_atpanic)(J, Some(panic_cb)).is_some();
        (api.js_freestate)(J);
        (c0, c1, c2, had1, had2, had3)
    });
}

/* ================================================================== */
/*  E7..E10 — push / pop / shuffle                                     */
/* ================================================================== */

#[test]
fn e7_e9_push_family_and_string_shapes() {
    diff("push family", |api| unsafe {
        let J = new_state(api, 0);
        (api.js_pushundefined)(J);
        (api.js_pushnull)(J);
        for b in [0, 1, 2, -1, i32::MAX, i32::MIN] {
            (api.js_pushboolean)(J, b);
        }
        for n in [0.0f64, -0.0, 1.0, -1.0, 0.5, f64::NAN, f64::INFINITY, 1e21, 1e-7] {
            (api.js_pushnumber)(J, n);
        }
        (api.js_pushglobal)(J);
        let snap = stack_snapshot(api, J);
        (api.js_freestate)(J);
        snap
    });

    // js_pushstring across the shrstr/memstr boundary
    diff("js_pushstring shapes", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        for len in [0usize, 1, 7, 14, 15, 16, 17, 31, 64, 1024] {
            let s = "x".repeat(len);
            let cstring = cs(&s);
            (api.js_pushstring)(J, cstring.as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
        }
        for s in ["h\u{e9}llo", "\u{65e5}\u{672c}\u{8a9e}", "\u{1F600}", "a\u{10FFFF}b"] {
            let cstring = cs(s);
            (api.js_pushstring)(J, cstring.as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
        }
        // js_pushliteral keeps the caller's pointer (JS_TLITSTR)
        (api.js_pushliteral)(J, b"literal-string\0".as_ptr() as *const c_char);
        out.push(slot(api, J, -1));
        (api.js_freestate)(J);
        out
    });

    // js_pushlstring: explicit lengths, including shorter than strlen and with
    // embedded NULs.
    diff("js_pushlstring shapes", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        let src: &[u8] = b"abc\0def-0123456789-0123456789\0";
        for n in [0usize, 1, 3, 4, 7, 14, 15, 16, 17, 29] {
            (api.js_pushlstring)(J, src.as_ptr() as *const c_char, n as c_int);
            let v = (api.js_tovalue)(J, -1);
            // Only the shrstr payload up to its terminating NUL is defined; the
            // C copies exactly `n` bytes + a NUL and leaves whatever was in the
            // slot before in the remaining bytes, and a memstr slot holds a raw
            // pointer. Comparing the whole union would compare stale/heap data.
            let defined: Vec<u8> = if (*v).tag() == JS_TSHRSTR {
                let b = (*v).bytes();
                let end = b.iter().position(|&c| c == 0).map(|i| i + 1).unwrap_or(16);
                b[..end].to_vec()
            } else {
                Vec::new()
            };
            out.push(format!("n={} {} shrstr_bytes={:?}", n, slot(api, J, -1), defined));
            (api.js_pop)(J, 1);
        }
        (api.js_freestate)(J);
        out
    });
}

#[test]
fn e10_stack_shuffling() {
    // All the shuffle primitives over a known stack, using only in-range
    // indices (the C does not bounds-check most of them).
    diff("stack shuffling", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        let mut rng = Rng::new(0x5748_1234);
        for round in 0..200 {
            // build a stack of 6 distinguishable values
            let base = (api.js_gettop)(J);
            for i in 0..6 {
                (api.js_pushnumber)(J, (round * 100 + i) as f64);
            }
            match rng.below(13) {
                0 => (api.js_dup)(J),
                1 => (api.js_dup2)(J),
                2 => (api.js_rot2)(J),
                3 => (api.js_rot3)(J),
                4 => (api.js_rot4)(J),
                5 => (api.js_rot2pop1)(J),
                6 => (api.js_rot3pop2)(J),
                7 => (api.js_copy)(J, base + rng.below(6) as c_int),
                8 => (api.js_copy)(J, -1 - rng.below(6) as c_int),
                9 => (api.js_remove)(J, base + rng.below(6) as c_int),
                10 => (api.js_replace)(J, base + rng.below(6) as c_int),
                11 => (api.js_rot)(J, 1 + rng.below(5) as c_int),
                _ => (api.js_pop)(J, rng.below(4) as c_int),
            }
            out.push(format!("{}: {:?}", round, stack_snapshot(api, J)));
            // reset
            let t = (api.js_gettop)(J);
            if t > 0 {
                (api.js_pop)(J, t);
            }
        }
        (api.js_freestate)(J);
        out
    });
}

/* ================================================================== */
/*  E11..E17 — predicates, conversions, comparison                     */
/* ================================================================== */

#[test]
fn e11_e13_predicates_and_types_over_every_value() {
    for expr in VALUES {
        let (c, r) = both(|api, _| unsafe {
            with_value(api, expr, |api, J| {
                let mut v = Vec::new();
                macro_rules! p {
                    ($f:ident) => {
                        v.push(format!("{}={}", stringify!($f), (api.$f)(J, -1)))
                    };
                }
                p!(js_isdefined);
                p!(js_isundefined);
                p!(js_isnull);
                p!(js_isboolean);
                p!(js_isnumber);
                p!(js_isstring);
                p!(js_isprimitive);
                p!(js_isobject);
                p!(js_isarray);
                p!(js_isregexp);
                p!(js_iscoercible);
                p!(js_iscallable);
                p!(js_iserror);
                p!(js_isnumberobject);
                p!(js_isstringobject);
                p!(js_isbooleanobject);
                p!(js_isdateobject);
                v.push(format!("js_type={}", (api.js_type)(J, -1)));
                v.push(format!(
                    "js_typeof={:?}",
                    cstr_string((api.js_typeof)(J, -1))
                ));
                for tag in ["Foo", "", "Bar"] {
                    v.push(format!(
                        "isuserdata({})={}",
                        tag,
                        (api.js_isuserdata)(J, -1, cs(tag).as_ptr())
                    ));
                }
                v
            })
        });
        assert_eq!(
            c, r,
            "DIVERGENCE predicates for {}:\n  C   : {:?}\n  Rust: {:?}",
            expr, c, r
        );
    }
}

#[test]
fn e12_conversions_over_every_value() {
    for expr in VALUES {
        let (c, r) = both(|api, _| unsafe {
            with_value(api, expr, |api, J| {
                let mut v = Vec::new();
                // Protected conversions first (they never throw).
                v.push(format!(
                    "trystring={:?}",
                    cstr_string((api.js_trystring)(J, -1, cs("<T>").as_ptr()))
                ));
                v.push(format!(
                    "trynumber={:#018x}",
                    (api.js_trynumber)(J, -1, -12345.0).to_bits()
                ));
                v.push(format!("tryinteger={}", (api.js_tryinteger)(J, -1, -999)));
                v.push(format!("tryboolean={}", (api.js_tryboolean)(J, -1, 2)));
                v.push(format!(
                    "tryrepr={:?}",
                    cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr()))
                ));
                // js_toboolean never throws.
                v.push(format!("toboolean={}", (api.js_toboolean)(J, -1)));
                v
            })
        });
        assert_eq!(
            c, r,
            "DIVERGENCE conversions for {}:\n  C   : {:?}\n  Rust: {:?}",
            expr, c, r
        );
    }
}

#[test]
fn e12b_throwing_conversions_inside_a_protected_frame() {
    // js_tonumber / js_tostring / js_tointeger / js_toint32 / js_touint32 /
    // js_toint16 / js_touint16 / js_toobject / js_toprimitive CAN throw, so they
    // are driven from JS where the interpreter provides the try frame.
    let scripts: Vec<String> = VALUES
        .iter()
        .map(|e| {
            format!(
                "var V = ({}); \
                 function t(f){{ try {{ return String(f()) }} catch(e) {{ return 'E:'+e.name }} }} \
                 print(t(function(){{return +V}}), \
                       t(function(){{return ''+V}}), \
                       t(function(){{return V|0}}), \
                       t(function(){{return V>>>0}}), \
                       t(function(){{return !!V}}), \
                       t(function(){{return Object(V)&&'obj'}}), \
                       t(function(){{return JSON.stringify(V)}}));",
                e
            )
        })
        .collect();
    let refs: Vec<&str> = scripts.iter().map(|s| s.as_str()).collect();
    diff_scripts(0, &refs);
    diff_scripts(JS_STRICT, &refs);
}

#[test]
fn e12c_integer_conversions_via_capi() {
    // js_tointeger/toint32/touint32/toint16/touint16 on values that cannot
    // throw (primitives only), driven directly through the C API.
    let safe: &[&str] = &[
        "undefined", "null", "true", "false", "0", "-0", "0.5", "-0.5", "1.9", "-1.9", "NaN",
        "Infinity", "-Infinity", "2147483647", "2147483648", "-2147483648", "-2147483649",
        "4294967295", "4294967296", "65535", "65536", "-65536", "32767", "32768", "-32768",
        "1e21", "1e-7", "9007199254740993", "'12'", "' 12 '", "'0x1f'", "'abc'", "''",
    ];
    for expr in safe {
        let (c, r) = both(|api, _| unsafe {
            with_value(api, expr, |api, J| {
                (
                    (api.js_tointeger)(J, -1),
                    (api.js_toint32)(J, -1),
                    (api.js_touint32)(J, -1),
                    (api.js_toint16)(J, -1),
                    (api.js_touint16)(J, -1),
                    (api.js_tonumber)(J, -1).to_bits(),
                    cstr_string((api.js_tostring)(J, -1)),
                )
            })
        });
        assert_eq!(
            c, r,
            "DIVERGENCE integer conversions for {}:\n  C   : {:?}\n  Rust: {:?}",
            expr, c, r
        );
    }
}

#[test]
fn e16_compare_equal_instanceof_cross_product() {
    // js_compare (with the `okay` out-parameter), js_equal, js_strictequal and
    // js_instanceof over a cross product. Each can throw, so drive them from a
    // cfunction-free path: only non-throwing operands are used here; the
    // throwing ones are covered by the JS-level corpus.
    let ops: &[&str] = &[
        "undefined", "null", "true", "false", "0", "-0", "1", "NaN", "Infinity", "'a'", "'b'",
        "''", "'0'", "'1'", "({})", "[]", "[1]", "(function(){})", "new Number(1)",
        "new String('a')", "new Boolean(false)",
    ];
    for a in ops {
        for b in ops {
            let (c, r) = both(|api, _| unsafe {
                let J = new_state(api, 0);
                let prep = format!("var A=({}); var B=({});", a, b);
                let rc = (api.js_dostring)(J, cs(&prep).as_ptr());
                let mut out = Vec::new();
                // js_compare
                (api.js_getglobal)(J, cs("A").as_ptr());
                (api.js_getglobal)(J, cs("B").as_ptr());
                let mut okay: c_int = -7;
                let cmp = (api.js_compare)(J, &mut okay);
                out.push(format!("compare={} okay={}", cmp, okay));
                (api.js_pop)(J, 2);
                // js_equal
                (api.js_getglobal)(J, cs("A").as_ptr());
                (api.js_getglobal)(J, cs("B").as_ptr());
                out.push(format!("equal={}", (api.js_equal)(J)));
                (api.js_pop)(J, 2);
                // js_strictequal
                (api.js_getglobal)(J, cs("A").as_ptr());
                (api.js_getglobal)(J, cs("B").as_ptr());
                out.push(format!("strictequal={}", (api.js_strictequal)(J)));
                (api.js_pop)(J, 2);
                // js_concat
                (api.js_getglobal)(J, cs("A").as_ptr());
                (api.js_getglobal)(J, cs("B").as_ptr());
                (api.js_concat)(J);
                out.push(format!("concat={}", slot(api, J, -1)));
                (api.js_pop)(J, 1);
                (api.js_freestate)(J);
                (rc, out)
            });
            assert_eq!(
                c, r,
                "DIVERGENCE compare/equal/concat for ({}, {}):\n  C   : {:?}\n  Rust: {:?}",
                a, b, c, r
            );
        }
    }
}

#[test]
fn e15_repr_over_every_value() {
    for expr in VALUES {
        let (c, r) = both(|api, _| unsafe {
            with_value(api, expr, |api, J| {
                let a = cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr()));
                // js_repr pushes the representation
                // js_repr can throw; use the protected variant again
                let b = cstr_string((api.js_tryrepr)(J, -1, cs("<R2>").as_ptr()));
                (a, b)
            })
        });
        assert_eq!(c, r, "DIVERGENCE repr for {}: C={:?} Rust={:?}", expr, c, r);
    }
    // nested / cyclic structures through the JS level (js_repr may throw)
    let scripts = [
        "var a=[1,[2,[3,[4]]]]; print(JSON.stringify(a));",
        "var o={a:{b:{c:{d:1}}}}; print(JSON.stringify(o));",
        "var c={}; c.self=c; try { print(JSON.stringify(c)) } catch(e) { print('caught', e.name) }",
        "print(JSON.stringify(['\\n','\\t','\\\\','\"',\"'\"]));",
        "print(JSON.stringify({ 'k\\ne\\ty': 'v\\u0001' }));",
    ];
    diff_scripts_both_modes(&scripts);
}

/* ================================================================== */
/*  F1..F12 — objects, properties, arrays                              */
/* ================================================================== */

#[test]
fn f1_f6_properties_and_attributes() {
    let attrs: Vec<c_int> = (0..8).chain([8, 15, -1, i32::MAX].into_iter()).collect();
    for &atts in &attrs {
        diff(&format!("properties atts={}", atts), |api| unsafe {
            let J = new_state(api, 0);
            let mut out = Vec::new();

            // js_newobject / js_newobjectx / js_newarray
            (api.js_newobject)(J);
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
            // js_newobjectx CONSUMES a stack slot: `js_isobject(J,-1) ?
            // prototype : NULL` then `js_pop(J, 1)`. Calling it on an empty
            // stack underflows. Exercise both an object prototype and a
            // non-object (which yields a NULL prototype).
            (api.js_newobject)(J);
            (api.js_pushnumber)(J, 3.0);
            (api.js_setproperty)(J, -2, cs("fromproto").as_ptr());
            (api.js_newobjectx)(J);
            out.push(slot(api, J, -1));
            (api.js_getproperty)(J, -1, cs("fromproto").as_ptr());
            out.push(format!("inherited={}", slot(api, J, -1)));
            (api.js_pop)(J, 2);
            (api.js_pushnumber)(J, 1.0); // not an object -> NULL prototype
            (api.js_newobjectx)(J);
            out.push(slot(api, J, -1));
            out.push(format!("no-proto has toString={}", has_prop(api, J, -1, "toString")));
            (api.js_pop)(J, 1);
            (api.js_pushnull)(J);
            (api.js_newobjectx)(J);
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
            (api.js_newarray)(J);
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);

            // defproperty with `atts`, then observe set/get/has/del/enumerate
            (api.js_newobject)(J);
            (api.js_pushnumber)(J, 1.0);
            (api.js_defproperty)(J, -2, cs("k").as_ptr(), atts);
            out.push(format!("has={}", has_prop(api, J, -1, "k")));
            (api.js_getproperty)(J, -1, cs("k").as_ptr());
            out.push(format!("get={}", slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_pushnumber)(J, 2.0);
            (api.js_setproperty)(J, -2, cs("k").as_ptr());
            (api.js_getproperty)(J, -1, cs("k").as_ptr());
            out.push(format!("after-set={}", slot(api, J, -1)));
            (api.js_pop)(J, 1);
            // enumerate (own only, and with the prototype chain)
            for own in [0, 1] {
                (api.js_copy)(J, -1);
                (api.js_pushiterator)(J, -1, own);
                let mut names = Vec::new();
                loop {
                    let p = (api.js_nextiterator)(J, -1);
                    if p.is_null() {
                        break;
                    }
                    names.push(cstr_string(p).unwrap_or_default());
                    if names.len() > 500 {
                        break;
                    }
                }
                (api.js_pop)(J, 2);
                out.push(format!("iter own={} {:?}", own, names));
            }
            (api.js_delproperty)(J, -1, cs("k").as_ptr());
            out.push(format!("after-del has={}", has_prop(api, J, -1, "k")));
            let t = (api.js_gettop)(J);
            (api.js_pop)(J, t);

            // js_defglobal with `atts`
            (api.js_pushnumber)(J, 5.0);
            (api.js_defglobal)(J, cs("gk").as_ptr(), atts);
            (api.js_getglobal)(J, cs("gk").as_ptr());
            out.push(format!("global={}", slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_pushnumber)(J, 6.0);
            (api.js_setglobal)(J, cs("gk").as_ptr());
            (api.js_getglobal)(J, cs("gk").as_ptr());
            out.push(format!("global-after-set={}", slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_delglobal)(J, cs("gk").as_ptr());
            (api.js_getglobal)(J, cs("gk").as_ptr());
            out.push(format!("global-after-del={}", slot(api, J, -1)));
            (api.js_pop)(J, 1);

            // js_defaccessor with `atts`: getter only / setter only / both
            for (g, s) in [(true, false), (false, true), (true, true)] {
                (api.js_newobject)(J);
                let prep = "var GET = function(){ return 'G' }; var SET = function(v){ this.seen = v };";
                (api.js_dostring)(J, cs(prep).as_ptr());
                if g {
                    (api.js_getglobal)(J, cs("GET").as_ptr());
                } else {
                    (api.js_pushnull)(J);
                }
                if s {
                    (api.js_getglobal)(J, cs("SET").as_ptr());
                } else {
                    (api.js_pushnull)(J);
                }
                (api.js_defaccessor)(J, -3, cs("acc").as_ptr(), atts);
                out.push(format!(
                    "accessor g={} s={} has={}",
                    g,
                    s,
                    has_prop(api, J, -1, "acc")
                ));
                let t = (api.js_gettop)(J);
                (api.js_pop)(J, t);
            }
            (api.js_freestate)(J);
            out
        });
    }
}

#[test]
fn f7_f8_registry_and_refs() {
    diff("registry + refs", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        // registry round trip
        for (k, v) in [("a", "1"), ("b", "'two'"), ("c", "({x:3})"), ("", "null")] {
            let prep = format!("var R = ({});", v);
            (api.js_dostring)(J, cs(&prep).as_ptr());
            (api.js_getglobal)(J, cs("R").as_ptr());
            (api.js_setregistry)(J, cs(k).as_ptr());
            (api.js_getregistry)(J, cs(k).as_ptr());
            out.push(format!("reg[{}]={}", k, slot(api, J, -1)));
            (api.js_pop)(J, 1);
        }
        (api.js_getregistry)(J, cs("missing").as_ptr());
        out.push(format!("reg[missing]={}", slot(api, J, -1)));
        (api.js_pop)(J, 1);
        (api.js_delregistry)(J, cs("a").as_ptr());
        (api.js_getregistry)(J, cs("a").as_ptr());
        out.push(format!("reg[a] after del={}", slot(api, J, -1)));
        (api.js_pop)(J, 1);

        // js_ref / js_unref: the returned id strings must match exactly
        let mut ids = Vec::new();
        for i in 0..40 {
            (api.js_pushnumber)(J, i as f64);
            let id = cstr_string((api.js_ref)(J)).unwrap_or_default();
            ids.push(id);
        }
        out.push(format!("refs={:?}", ids));
        for id in &ids {
            (api.js_getregistry)(J, cs(id).as_ptr());
            out.push(format!("deref {}={}", id, slot(api, J, -1)));
            (api.js_pop)(J, 1);
        }
        for id in &ids {
            (api.js_unref)(J, cs(id).as_ptr());
        }
        // reuse after unref must produce the same ids again
        let mut ids2 = Vec::new();
        for i in 0..40 {
            (api.js_pushnumber)(J, 100.0 + i as f64);
            ids2.push(cstr_string((api.js_ref)(J)).unwrap_or_default());
        }
        out.push(format!("refs2={:?}", ids2));
        (api.js_freestate)(J);
        out
    });
}

#[test]
fn f9_f12_indexes_lengths_and_array_shapes() {
    diff("index/length/array shapes", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        let mut rng = Rng::new(0xA55E_1234);

        // flat/simple array: sequential appends, then reads
        (api.js_newarray)(J);
        for i in 0..40 {
            (api.js_pushnumber)(J, (i * 3) as f64);
            (api.js_setindex)(J, -2, i);
        }
        out.push(format!("len={}", (api.js_getlength)(J, -1)));
        for i in -3..45 {
            out.push(format!("has[{}]={}", i, has_index(api, J, -1, i)));
            let t = (api.js_gettop)(J);
            (api.js_getindex)(J, -1, i);
            out.push(format!("get[{}]={}", i, slot(api, J, -1)));
            (api.js_pop)(J, (api.js_gettop)(J) - t);
        }
        // force the sparse/unflattened representation
        (api.js_pushnumber)(J, 1.0);
        (api.js_setindex)(J, -2, 100000);
        out.push(format!("after sparse len={}", (api.js_getlength)(J, -1)));
        out.push(format!("has[100000]={}", has_index(api, J, -1, 100000)));
        out.push(format!("has[50000]={}", has_index(api, J, -1, 50000)));
        // delete + read back
        (api.js_delindex)(J, -1, 5);
        out.push(format!("has[5] after del={}", has_index(api, J, -1, 5)));
        // setlength: grow / shrink / zero
        for l in [50, 10, 0, 5, 1000] {
            (api.js_setlength)(J, -1, l);
            out.push(format!("setlength({}) -> {}", l, (api.js_getlength)(J, -1)));
        }
        let t = (api.js_gettop)(J);
        (api.js_pop)(J, t);

        // getlength/setlength on a plain object
        (api.js_newobject)(J);
        out.push(format!("obj len={}", (api.js_getlength)(J, -1)));
        (api.js_setlength)(J, -1, 7);
        out.push(format!("obj len after set={}", (api.js_getlength)(J, -1)));
        (api.js_pop)(J, 1);

        // randomized index/length churn
        (api.js_newarray)(J);
        for _ in 0..600 {
            match rng.below(5) {
                0 => {
                    let i = rng.below(30) as c_int;
                    (api.js_pushnumber)(J, rng.below(1000) as f64);
                    (api.js_setindex)(J, -2, i);
                }
                1 => {
                    let i = rng.below(40) as c_int;
                    (api.js_delindex)(J, -1, i);
                }
                2 => {
                    let l = rng.below(50) as c_int;
                    (api.js_setlength)(J, -1, l);
                }
                3 => {
                    let i = rng.below(40) as c_int;
                    let t = (api.js_gettop)(J);
                    (api.js_getindex)(J, -1, i);
                    out.push(format!("r-get[{}]={}", i, slot(api, J, -1)));
                    (api.js_pop)(J, (api.js_gettop)(J) - t);
                }
                _ => {
                    let i = rng.below(40) as c_int;
                    out.push(format!("r-has[{}]={}", i, has_index(api, J, -1, i)));
                }
            }
            out.push(format!("len={}", (api.js_getlength)(J, -1)));
        }
        // enumerate the churned array both ways
        for own in [0, 1] {
            (api.js_copy)(J, -1);
            (api.js_pushiterator)(J, -1, own);
            let mut names = Vec::new();
            loop {
                let p = (api.js_nextiterator)(J, -1);
                if p.is_null() {
                    break;
                }
                names.push(cstr_string(p).unwrap_or_default());
                if names.len() > 2000 {
                    break;
                }
            }
            (api.js_pop)(J, 2);
            out.push(format!("final iter own={} {:?}", own, names));
        }
        (api.js_freestate)(J);
        out
    });
}

#[test]
fn f13_iterators_over_many_receivers() {
    let receivers: &[&str] = &[
        "({})",
        "({a:1,b:2,c:3})",
        "[]",
        "[1,2,3]",
        "[1,,3]",
        "new String('abc')",
        "'abc'",
        "Object.create({inherited:1})",
        "Object.create({inherited:1},{own:{value:2,enumerable:true}})",
        "(function(){ var o={}; Object.defineProperty(o,'hidden',{value:1,enumerable:false}); o.shown=2; return o })()",
        "Math",
        "JSON",
        "(function(){})",
        "new Date(0)",
        "/re/g",
        "new Error('e')",
        "Object.create(null)",
        "(function(){return arguments})(7,8,9)",
        "null",
        "undefined",
        "42",
    ];
    for expr in receivers {
        for own in [0, 1, 2, -1] {
            let (c, r) = both(|api, _| unsafe {
                with_value(api, expr, |api, J| {
                    // js_pushiterator throws for non-coercible receivers, so run
                    // it protected via js_dostring where possible; here we only
                    // call it for objects and record the error otherwise.
                    if (api.js_isobject)(J, -1) == 0 {
                        return vec![format!("not-an-object typeof={:?}",
                            cstr_string((api.js_typeof)(J, -1)))];
                    }
                    (api.js_pushiterator)(J, -1, own);
                    let mut names = Vec::new();
                    loop {
                        let p = (api.js_nextiterator)(J, -1);
                        if p.is_null() {
                            break;
                        }
                        names.push(cstr_string(p).unwrap_or_default());
                        if names.len() > 1000 {
                            break;
                        }
                    }
                    (api.js_pop)(J, 1);
                    names
                })
            });
            assert_eq!(
                c, r,
                "DIVERGENCE iterator({}, own={}):\n  C   : {:?}\n  Rust: {:?}",
                expr, own, c, r
            );
        }
    }
    // and the JS-visible for-in ordering for the same receivers
    let scripts: Vec<String> = receivers
        .iter()
        .map(|e| {
            format!(
                "try {{ var ks=[]; for (var k in ({})) ks.push(k); print(ks.join(',')) }} \
                 catch(e) {{ print('caught', e.name, e.message) }}",
                e
            )
        })
        .collect();
    let refs: Vec<&str> = scripts.iter().map(|s| s.as_str()).collect();
    diff_scripts_both_modes(&refs);
}

/* ---------------- F14/F15 userdata ---------------- */

static mut FINALIZED: [i64; 2] = [0, 0];
static mut UD_SIDE: usize = 0;
static mut HAS_RET: c_int = 0;
static mut PUT_RET: c_int = 0;
static mut DEL_RET: c_int = 0;
static mut CB_LOG: Option<Vec<String>> = None;

unsafe extern "C-unwind" fn ud_finalize(_J: State, _p: *mut c_void) {
    FINALIZED[UD_SIDE] += 1;
}
unsafe extern "C-unwind" fn ud_has(J: State, _p: *mut c_void, name: *const c_char) -> c_int {
    if let Some(l) = CB_LOG.as_mut() {
        l.push(format!("has({:?})", cstr_string(name)));
    }
    if HAS_RET != 0 {
        // must push the value it claims to have
        let api = CUR_API.with(|c| c.get());
        if !api.is_null() {
            ((*api).js_pushnumber)(J, 111.0);
        }
    }
    HAS_RET
}
unsafe extern "C-unwind" fn ud_put(_J: State, _p: *mut c_void, name: *const c_char) -> c_int {
    if let Some(l) = CB_LOG.as_mut() {
        l.push(format!("put({:?})", cstr_string(name)));
    }
    PUT_RET
}
unsafe extern "C-unwind" fn ud_del(_J: State, _p: *mut c_void, name: *const c_char) -> c_int {
    if let Some(l) = CB_LOG.as_mut() {
        l.push(format!("del({:?})", cstr_string(name)));
    }
    DEL_RET
}

thread_local! {
    static CUR_API: std::cell::Cell<*const Api> = std::cell::Cell::new(std::ptr::null());
}

#[test]
fn f14_f15_userdata() {
    for (h, p, d) in [
        (0, 0, 0),
        (1, 0, 0),
        (0, 1, 0),
        (0, 0, 1),
        (1, 1, 1),
    ] {
        let (c, r) = both(|api, side| unsafe {
            UD_SIDE = if side == Side::C { 0 } else { 1 };
            FINALIZED[UD_SIDE] = 0;
            HAS_RET = h;
            PUT_RET = p;
            DEL_RET = d;
            CB_LOG = Some(Vec::new());
            CUR_API.with(|c| c.set(api as *const Api));
            out_clear();
            let J = new_state(api, 0);
            let mut out = Vec::new();

            // plain js_newuserdata
            (api.js_newobject)(J); // prototype
            (api.js_newuserdata)(
                J,
                lit!("MyTag"),
                0x1234 as *mut c_void,
                Some(ud_finalize),
            );
            out.push(format!("ud={}", slot(api, J, -1)));
            out.push(format!(
                "isuserdata(MyTag)={} isuserdata(Other)={}",
                (api.js_isuserdata)(J, -1, lit!("MyTag")),
                (api.js_isuserdata)(J, -1, cs("Other").as_ptr())
            ));
            out.push(format!(
                "touserdata={:#x}",
                (api.js_touserdata)(J, -1, lit!("MyTag")) as usize
            ));
            (api.js_setglobal)(J, cs("UD").as_ptr());

            // js_newuserdatax with has/put/delete
            (api.js_newobject)(J);
            (api.js_newuserdatax)(
                J,
                lit!("XTag"),
                0x5678 as *mut c_void,
                Some(ud_has),
                Some(ud_put),
                Some(ud_del),
                Some(ud_finalize),
            );
            (api.js_setglobal)(J, cs("UDX").as_ptr());

            let rc = (api.js_dostring)(
                J,
                cs("print(typeof UD, typeof UDX); \
                    print(UDX.anything); \
                    UDX.other = 5; \
                    print(delete UDX.gone); \
                    print('own' in UDX);")
                    .as_ptr(),
            );
            out.push(format!("rc={}", rc));
            out.push(format!("log={:?}", CB_LOG.as_ref().unwrap()));

            // drop the references and collect
            (api.js_dostring)(J, cs("UD=null; UDX=null;").as_ptr());
            (api.js_gc)(J, 0);
            (api.js_gc)(J, 0);
            out.push(format!("finalized={}", FINALIZED[UD_SIDE]));
            (api.js_freestate)(J);
            out.push(format!("finalized after freestate={}", FINALIZED[UD_SIDE]));
            CUR_API.with(|c| c.set(std::ptr::null()));
            (out, out_take())
        });
        assert_eq!(
            c.0, r.0,
            "DIVERGENCE userdata(h={},p={},d={}):\n  C   : {:#?}\n  Rust: {:#?}",
            h, p, d, c.0, r.0
        );
        assert_eq!(
            String::from_utf8_lossy(&c.1),
            String::from_utf8_lossy(&r.1),
            "DIVERGENCE userdata output(h={},p={},d={})",
            h,
            p,
            d
        );
    }
}

/* ---------------- F16..F19 cfunctions & constructors ---------------- */

static mut CDATA_FINALIZED: [i64; 2] = [0, 0];
static mut CF_SIDE: usize = 0;

unsafe extern "C-unwind" fn cf_data_finalize(_J: State, _p: *mut c_void) {
    CDATA_FINALIZED[CF_SIDE] += 1;
}

/// A cfunction that reports its arguments and its function data.
unsafe extern "C-unwind" fn cf_probe(J: State) {
    let api = CUR_API.with(|c| c.get());
    if api.is_null() {
        return;
    }
    let api = &*api;
    let top = (api.js_gettop)(J);
    let data = (api.js_currentfunctiondata)(J);
    let mut s = format!("top={} data={:#x} args=[", top, data as usize);
    for i in 0..top {
        if i > 0 {
            s.push(',');
        }
        let p = (api.js_trystring)(J, i, cs("<T>").as_ptr());
        s.push_str(&cstr_string(p).unwrap_or_default());
    }
    s.push(']');
    (api.js_currentfunction)(J);
    let cf = cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr())).unwrap_or_default();
    (api.js_pop)(J, 1);
    s.push_str(&format!(" self={}", cf));
    let out = cs(&s);
    (api.js_pushstring)(J, out.as_ptr());
}

#[test]
fn f16_f19_cfunctions_and_constructors() {
    let (c, r) = both(|api, side| unsafe {
        CF_SIDE = if side == Side::C { 0 } else { 1 };
        CDATA_FINALIZED[CF_SIDE] = 0;
        CUR_API.with(|c| c.set(api as *const Api));
        out_clear();
        let J = new_state(api, 0);
        let mut out = Vec::new();

        for length in [0, 1, 2, 3, -1] {
            (api.js_newcfunction)(J, Some(cf_probe), lit!("probe"), length);
            (api.js_setglobal)(J, lit!("probe"));
            for call in [
                "probe()",
                "probe(1)",
                "probe(1,2)",
                "probe(1,2,3,4,5)",
                "probe.call(null)",
                "probe.apply(null,[9,8])",
            ] {
                let src = format!("print({});", call);
                let rc = (api.js_dostring)(J, cs(&src).as_ptr());
                out.push(format!("len={} {} rc={}", length, call, rc));
            }
        }

        // js_newcfunctionx: data + finalize
        (api.js_newcfunctionx)(
            J,
            Some(cf_probe),
            lit!("probex"),
            1,
            0xDEAD as *mut c_void,
            Some(cf_data_finalize),
        );
        (api.js_setglobal)(J, lit!("probex"));
        out.push(format!(
            "probex rc={}",
            (api.js_dostring)(J, cs("print(probex(1,2));").as_ptr())
        ));

        // js_newcconstructor: callable and constructible.
        // jsvalue.c:520 documents the stack effect as `/* prototype -- constructor */`
        // — the caller MUST push the prototype object first (the function does
        // `js_rot2` to get it below the new function object).
        (api.js_newobject)(J);
        (api.js_newcconstructor)(
            J,
            Some(cf_probe),
            Some(cf_probe),
            lit!("Ctor"),
            2,
        );
        (api.js_setglobal)(J, lit!("Ctor"));
        for call in ["Ctor()", "Ctor(1,2)", "new Ctor()", "new Ctor(1,2,3)"] {
            let src = format!("print(String({}));", call);
            out.push(format!(
                "{} rc={}",
                call,
                (api.js_dostring)(J, cs(&src).as_ptr())
            ));
        }

        // js_newboolean/newnumber/newstring and all seven js_new*error
        (api.js_newboolean)(J, 1);
        out.push(slot(api, J, -1));
        (api.js_pop)(J, 1);
        (api.js_newboolean)(J, 0);
        out.push(slot(api, J, -1));
        (api.js_pop)(J, 1);
        for n in [0.0, -0.0, 1.5, f64::NAN, f64::INFINITY] {
            (api.js_newnumber)(J, n);
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
        }
        for s in ["", "a", "0123456789abcdefg"] {
            (api.js_newstring)(J, cs(s).as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
        }
        for msg in ["", "boom", "a\nb"] {
            let m = cs(msg);
            (api.js_newerror)(J, m.as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
            (api.js_newevalerror)(J, m.as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
            (api.js_newrangeerror)(J, m.as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
            (api.js_newreferenceerror)(J, m.as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
            (api.js_newsyntaxerror)(J, m.as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
            (api.js_newtypeerror)(J, m.as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
            (api.js_newurierror)(J, m.as_ptr());
            out.push(slot(api, J, -1));
            (api.js_pop)(J, 1);
        }

        (api.js_dostring)(J, cs("probex=null;").as_ptr());
        (api.js_gc)(J, 0);
        (api.js_gc)(J, 0);
        (api.js_freestate)(J);
        out.push(format!("cdata finalized={}", CDATA_FINALIZED[CF_SIDE]));
        CUR_API.with(|c| c.set(std::ptr::null()));
        (out, out_take())
    });
    assert_eq!(
        c.0, r.0,
        "DIVERGENCE cfunctions:\n  C   : {:#?}\n  Rust: {:#?}",
        c.0, r.0
    );
    assert_eq!(
        String::from_utf8_lossy(&c.1),
        String::from_utf8_lossy(&r.1),
        "DIVERGENCE cfunction output"
    );
}

#[test]
fn f20_js_newregexp_all_flag_combinations() {
    let patterns = ["a", "a(b)c", "^x$", "[a-z]+", "(?:q)*", "\\d\\w\\s", "(", "[z-a]"];
    for p in patterns {
        for flags in 0..8 {
            let (c, r) = both(|api, _| unsafe {
                out_clear();
                let J = new_state(api, 0);
                (api.js_pushnumber)(J, 0.0); // spare slot
                let mut ok = 1;
                // js_newregexp throws for an invalid pattern; guard by using a
                // protected outer frame via js_dostring + a cfunction is heavy,
                // so pre-validate with js_regcomp and only then call it.
                let pat = cs(p);
                let mut err: *const c_char = std::ptr::null();
                let prog = (api.js_regcomp)(pat.as_ptr(), 0, &mut err);
                if prog.is_null() {
                    ok = 0;
                } else {
                    (api.js_regfree)(prog);
                }
                let mut out = Vec::new();
                out.push(format!("compiles={} err={:?}", ok, cstr_string(err)));
                if ok == 1 {
                    (api.js_newregexp)(J, pat.as_ptr(), flags);
                    out.push(slot(api, J, -1));
                    (api.js_setglobal)(J, cs("RE").as_ptr());
                    let rc = (api.js_dostring)(
                        J,
                        cs("print(RE.source, RE.global, RE.ignoreCase, RE.multiline, RE.lastIndex); \
                            print(RE.exec('xxabcAB'), RE.lastIndex); \
                            print(RE.test('QqABab'), RE.lastIndex); \
                            print(String(RE));")
                            .as_ptr(),
                    );
                    out.push(format!("rc={}", rc));
                }
                (api.js_freestate)(J);
                (out, out_take())
            });
            assert_eq!(
                c.0, r.0,
                "DIVERGENCE js_newregexp({:?}, flags={}):\n  C   : {:?}\n  Rust: {:?}",
                p, flags, c.0, r.0
            );
            assert_eq!(
                String::from_utf8_lossy(&c.1),
                String::from_utf8_lossy(&r.1),
                "DIVERGENCE js_newregexp output ({:?}, flags={})",
                p,
                flags
            );
        }
    }
}

/* ---------------- F21..F24 low-level jsV_* entry points ---------------- */

#[test]
fn f21_f22_low_level_property_api() {
    diff("jsV_* property API", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        // build an object with a prototype chain through the public API
        (api.js_dostring)(
            J,
            cs("var proto = {p:1, shared:'proto'}; \
                var obj = Object.create(proto); obj.own = 2; obj.shared = 'own'; \
                Object.defineProperty(obj, 'hidden', {value:3, enumerable:false}); \
                Object.defineProperty(obj, 'ro', {value:4, writable:false}); \
                Object.defineProperty(obj, 'acc', {get:function(){return 5}});")
                .as_ptr(),
        );
        (api.js_getglobal)(J, cs("obj").as_ptr());
        let o = (api.js_toobject)(J, -1);
        for name in ["own", "p", "shared", "hidden", "ro", "acc", "missing", ""] {
            let n = cs(name);
            let g_own = (api.jsV_getownproperty)(J, o, n.as_ptr());
            let g_any = (api.jsV_getproperty)(J, o, n.as_ptr());
            let mut own_flag: c_int = -7;
            let g_x = (api.jsV_getpropertyx)(J, o, n.as_ptr(), &mut own_flag);
            out.push(format!(
                "{}: own={} any={} x={} ownflag={}",
                name,
                !g_own.is_null(),
                !g_any.is_null(),
                !g_x.is_null(),
                own_flag
            ));
        }
        // jsV_setproperty creates a slot
        for name in ["fresh", "own", "ro"] {
            let n = cs(name);
            let p = (api.jsV_setproperty)(J, o, n.as_ptr());
            out.push(format!("set {} -> created={}", name, !p.is_null()));
        }
        // jsV_delproperty
        for name in ["own", "missing", "hidden"] {
            let n = cs(name);
            (api.jsV_delproperty)(J, o, n.as_ptr());
            out.push(format!(
                "after del {} own={}",
                name,
                !(api.jsV_getownproperty)(J, o, n.as_ptr()).is_null()
            ));
        }
        // jsV_newobject for every class value plus out-of-range
        for cl in [
            JS_COBJECT, JS_CARRAY, JS_CFUNCTION, JS_CSCRIPT, JS_CCFUNCTION, JS_CERROR,
            JS_CBOOLEAN, JS_CNUMBER, JS_CSTRING, JS_CREGEXP, JS_CDATE, JS_CMATH, JS_CJSON,
            JS_CARGUMENTS, JS_CITERATOR, JS_CUSERDATA, 16, 99, -1,
        ] {
            let no = (api.jsV_newobject)(J, cl, std::ptr::null_mut());
            (api.js_pushobject)(J, no);
            out.push(format!(
                "class {} typeof={:?} type={} isobject={} iscallable={}",
                cl,
                cstr_string((api.js_typeof)(J, -1)),
                (api.js_type)(J, -1),
                (api.js_isobject)(J, -1),
                (api.js_iscallable)(J, -1)
            ));
            (api.js_pop)(J, 1);
        }
        // jsV_newiterator / jsV_nextiterator directly
        for own in [0, 1] {
            let it = (api.jsV_newiterator)(J, o, own);
            let mut names = Vec::new();
            loop {
                let p = (api.jsV_nextiterator)(J, it);
                if p.is_null() {
                    break;
                }
                names.push(cstr_string(p).unwrap_or_default());
                if names.len() > 500 {
                    break;
                }
            }
            out.push(format!("jsV_newiterator own={} {:?}", own, names));
        }
        (api.js_freestate)(J);
        out
    });
}

#[test]
fn f24_low_level_value_conversions() {
    for expr in VALUES {
        let (c, r) = both(|api, _| unsafe {
            with_value(api, expr, |api, J| {
                let v = (api.js_tovalue)(J, -1);
                let mut out = Vec::new();
                out.push(format!("bytes_tag={}", (*v).tag()));
                out.push(format!("jsV_toboolean={}", (api.jsV_toboolean)(J, v)));
                // jsV_tonumber/tostring/tointeger/toobject can throw for the
                // exotic receivers, so only call them when the value is a
                // primitive (that is exactly what the C guarantees cannot throw).
                if (api.js_isprimitive)(J, -1) != 0 {
                    out.push(format!(
                        "jsV_tonumber={:#018x}",
                        (api.jsV_tonumber)(J, v).to_bits()
                    ));
                    out.push(format!(
                        "jsV_tointeger={:#018x}",
                        (api.jsV_tointeger)(J, v).to_bits()
                    ));
                    out.push(format!(
                        "jsV_tostring={:?}",
                        cstr_string((api.jsV_tostring)(J, v))
                    ));
                    // jsV_toobject THROWS for undefined/null (jsvalue.c:401/402),
                    // and an unprotected throw aborts, so only call it for
                    // coercible primitives. The undefined/null rejections
                    // themselves are ERRORS.md rows covered by errors_js.rs.
                    if (api.js_iscoercible)(J, -1) != 0 {
                        let ob = (api.jsV_toobject)(J, v);
                        (api.js_pushobject)(J, ob);
                        out.push(format!("jsV_toobject -> {}", slot(api, J, -1)));
                        (api.js_pop)(J, 1);
                    } else {
                        out.push("jsV_toobject -> throws (non-coercible)".to_string());
                    }
                }
                // js_pushvalue must round-trip the 16-byte union by value
                (api.js_pushvalue)(J, *v);
                out.push(format!("pushvalue -> {}", slot(api, J, -1)));
                out.push(format!(
                    "identical_bytes={}",
                    (*(api.js_tovalue)(J, -1)).bytes() == (*v).bytes()
                ));
                (api.js_pop)(J, 1);
                out
            })
        });
        assert_eq!(
            c, r,
            "DIVERGENCE low-level conversions for {}:\n  C   : {:?}\n  Rust: {:?}",
            expr, c, r
        );
    }
}

#[test]
fn e14_number_coercion_helpers_direct() {
    // jsV_numbertostring / jsV_stringtonumber / js_itoa etc. are covered in
    // tests/numbers.rs. Here: js_isarrayindex, which writes through `int *k`.
    diff("js_isarrayindex", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        let names = [
            "", "0", "1", "9", "10", "01", "1e2", "-1", "+1", "1.0", "0.5", " 1", "1 ",
            "4294967294", "4294967295", "4294967296", "2147483647", "2147483648",
            "99999999999999999999", "abc", "1a", "a1", "0x10", "NaN", "Infinity", "-0",
        ];
        for n in names {
            let cn = cs(n);
            let mut k: c_int = -777;
            let is = (api.js_isarrayindex)(J, cn.as_ptr(), &mut k);
            out.push(format!("{:?} -> is={} k={}", n, is, k));
        }
        (api.js_freestate)(J);
        out
    });
}

/* ================================================================== */
/*  G — the compile/run pipeline through its LOW-LEVEL entry points     */
/* ================================================================== */

const PROGRAMS: &[&str] = &[
    "",
    "1+1",
    "print(1+1)",
    "var x = 1; print(x)",
    "function f(a,b){ return a*b } print(f(6,7))",
    "var a=[]; for (var i=0;i<10;++i) a.push(i*i); print(a.join(','))",
    "try { null.x } catch(e) { print(e.name) }",
    "print((function(){ return typeof this })())",
    "'use strict'; var y=2; print(y)",
    "print([1,2,3].map(function(v){return v+1}).join('-'))",
    "switch(2){case 1:print('a');break;case 2:print('b');default:print('c')}",
    "var o={get g(){return 9}}; print(o.g)",
    "print(JSON.stringify({a:[1,'2',null,true]}))",
    "label: for (var i=0;i<3;++i) { for (var j=0;j<3;++j) { if (j==1) continue label; print(i,j) } }",
    "with({w:5}) print(w)",
    "print(eval('2*21'))",
    "var g=0; function inc(){ g++ } inc(); inc(); print(g)",
    "print(/a(b)c/.exec('xabcx'))",
    "print(new Date(0).toISOString())",
    "print(Math.floor(3.7), Math.max(1,2,3))",
];

#[test]
fn g1_g4_manual_parse_compile_run_pipeline() {
    // Mirror js_loadstringx by hand: jsP_parse -> jsC_compilescript ->
    // jsP_freeparse -> js_newscript -> js_call, for both compile-time strict
    // settings. Any of those steps can throw (e.g. `with` compiled with
    // strict=1), so the whole sequence runs in a protected frame. jsP_freeparse
    // is exercised on BOTH the success and the failure path, exactly like the
    // C's own `if (js_try(J)) { jsP_freeparse(J); js_throw(J); }`.
    let sources: Vec<String> = PROGRAMS
        .iter()
        .map(|s| s.to_string())
        .chain(
            [
                "(", "var 1", "with({}){}", "'use strict'; with({}){}", "break",
                "continue", "return", "1 2", "function f(a,a){}", "delete 1",
                "1 = 2", "({a:1,a:2})", "'use strict'; ({a:1,a:2})",
            ]
            .iter()
            .map(|s| s.to_string()),
        )
        .collect();
    for strict in [0, 1] {
        for src in &sources {
            let owned = src.clone();
            let (c, r) = both(|api, _| {
                let owned = owned.clone();
                run_protected(api, 0, move |api, J| unsafe {
                    let fname = cs("[pipeline]");
                    let source = cs(&owned);
                    let mut log = String::new();
                    let P = (api.jsP_parse)(J, fname.as_ptr(), source.as_ptr());
                    log.push_str(&format!("parsed={} ", !P.is_null()));
                    let F = (api.jsC_compilescript)(J, P, strict);
                    log.push_str(&format!("compiled={} ", !F.is_null()));
                    (api.jsP_freeparse)(J);
                    (api.js_newscript)(J, F, std::ptr::null_mut());
                    log.push_str(&format!(
                        "script={} ",
                        cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr()))
                            .unwrap_or_default()
                    ));
                    (api.js_pushundefined)(J);
                    (api.js_call)(J, 0);
                    log.push_str(&format!(
                        "result={}",
                        cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr()))
                            .unwrap_or_default()
                    ));
                    (api.js_pop)(J, 1);
                    log
                })
            });
            assert_eq!(
                c,
                r,
                "DIVERGENCE pipeline strict={} {:?}:\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
                strict,
                src,
                c.0,
                String::from_utf8_lossy(&c.1),
                r.0,
                String::from_utf8_lossy(&r.1)
            );
        }
    }
    // jsP_freeparse called twice / after a failed parse
    for src in ["1+1", "("] {
        let owned = src.to_string();
        let (c, r) = both(|api, _| {
            let owned = owned.clone();
            run_protected(api, 0, move |api, J| unsafe {
                let fname = cs("[fp]");
                let source = cs(&owned);
                let P = (api.jsP_parse)(J, fname.as_ptr(), source.as_ptr());
                (api.jsP_freeparse)(J);
                (api.jsP_freeparse)(J);
                format!("parsed={} freed twice", !P.is_null())
            })
        });
        assert_eq!(c, r, "DIVERGENCE jsP_freeparse twice for {:?}", src);
    }
}

#[test]
fn g3_parsefunction_compilefunction() {
    // jsP_parsefunction + jsC_compilefunction: the `new Function(...)` path.
    // Runs protected because a bad parameter list or body throws.
    let cases: &[(&str, &str)] = &[
        ("", ""),
        ("", "return 1"),
        ("a", "return a"),
        ("a,b", "return a+b"),
        ("a,b,c", "return a*b*c"),
        ("a", "var t = a*2; return t"),
        ("", "print('side effect')"),
        ("x", "if (x) return 'yes'; return 'no'"),
        ("", "for (var i=0;i<3;++i) print(i)"),
        ("a,a", "return a"),
        ("1", "return 1"),
        ("a", "return ("),
        ("", "'use strict'; return this === undefined"),
        ("a,b,c,d,e,f,g,h", "return h"),
    ];
    for (params, body) in cases {
        let (p, b) = (params.to_string(), body.to_string());
        let (c, r) = both(|api, _| {
            let (p, b) = (p.clone(), b.clone());
            run_protected(api, 0, move |api, J| unsafe {
                let fname = cs("[fn]");
                let cp = cs(&p);
                let cb = cs(&b);
                let mut log = String::new();
                let P = (api.jsP_parsefunction)(J, fname.as_ptr(), cp.as_ptr(), cb.as_ptr());
                log.push_str(&format!("parsed={} ", !P.is_null()));
                let F = (api.jsC_compilefunction)(J, P);
                log.push_str(&format!("compiled={} ", !F.is_null()));
                (api.jsP_freeparse)(J);
                (api.js_newfunction)(J, F, std::ptr::null_mut());
                log.push_str(&format!(
                    "fn={} ",
                    cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr())).unwrap_or_default()
                ));
                (api.js_pushundefined)(J);
                (api.js_pushnumber)(J, 3.0);
                (api.js_pushnumber)(J, 4.0);
                (api.js_pushnumber)(J, 5.0);
                (api.js_call)(J, 3);
                log.push_str(&format!(
                    "result={}",
                    cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr())).unwrap_or_default()
                ));
                (api.js_pop)(J, 1);
                log
            })
        });
        assert_eq!(
            c,
            r,
            "DIVERGENCE parse/compilefunction({:?}, {:?}):\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
            params,
            body,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
    // and the same shapes through the public `Function` constructor
    let scripts = [
        "var f = new Function('return 1'); print(f())",
        "var f = new Function('a','return a*2'); print(f(21))",
        "var f = new Function('a','b','return a+b'); print(f(1,2))",
        "var f = Function('return this===undefined'); print(f())",
        "try { new Function('return (') } catch(e) { print('caught', e.name) }",
        "try { new Function('a','a','return a') } catch(e) { print('caught', e.name, e.message) }",
        "print(new Function('a','b','c','return a+b+c').length)",
        "print(String(new Function('a','return a')))",
    ];
    diff_scripts_both_modes(&scripts);
}

#[test]
fn g5_g8_loadstring_loadeval_ploadstring() {
    // js_loadstring / js_loadeval throw on a parse error, so they run protected;
    // js_ploadstring is itself protected and its return code is compared.
    let sources: Vec<String> = PROGRAMS
        .iter()
        .map(|s| s.to_string())
        .chain(
            ["(", "var 1", "with({}){}", "1 2", "'use strict'; with({}){}"]
                .iter()
                .map(|s| s.to_string()),
        )
        .collect();
    for src in &sources {
        for which in 0..3 {
            for flags in [0, JS_STRICT] {
                let owned = src.clone();
                let (c, r) = both(|api, _| {
                    let owned = owned.clone();
                    run_protected(api, flags, move |api, J| unsafe {
                        let fname = cs("[load]");
                        let source = cs(&owned);
                        let mut log = String::new();
                        let rc = match which {
                            0 => {
                                (api.js_loadstring)(J, fname.as_ptr(), source.as_ptr());
                                0
                            }
                            1 => {
                                (api.js_loadeval)(J, fname.as_ptr(), source.as_ptr());
                                0
                            }
                            _ => (api.js_ploadstring)(J, fname.as_ptr(), source.as_ptr()),
                        };
                        log.push_str(&format!(
                            "rc={} loaded={} ",
                            rc,
                            cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr()))
                                .unwrap_or_default()
                        ));
                        if rc == 0 {
                            (api.js_pushundefined)(J);
                            let crc = (api.js_pcall)(J, 0);
                            log.push_str(&format!(
                                "crc={} result={}",
                                crc,
                                cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr()))
                                    .unwrap_or_default()
                            ));
                        }
                        (api.js_pop)(J, 1);
                        log
                    })
                });
                assert_eq!(
                    c,
                    r,
                    "DIVERGENCE load which={} flags={} {:?}:\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
                    which,
                    flags,
                    src,
                    c.0,
                    String::from_utf8_lossy(&c.1),
                    r.0,
                    String::from_utf8_lossy(&r.1)
                );
            }
        }
    }
}

#[test]
fn g13_js_eval_entry_point() {
    // js_eval() evaluates the string on top of the stack and CAN throw
    // (`with` in a strict state is a SyntaxError), so it runs in a protected
    // frame supplied by the interpreter.
    for src in PROGRAMS {
        for flags in [0, JS_STRICT] {
            let owned = src.to_string();
            let (c, r) = both(|api, _| {
                let owned = owned.clone();
                run_protected(api, flags, move |api, J| unsafe {
                    let s = cs(&owned);
                    (api.js_pushstring)(J, s.as_ptr());
                    (api.js_eval)(J);
                    let res = cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr()))
                        .unwrap_or_default();
                    (api.js_pop)(J, 1);
                    format!("eval -> {}", res)
                })
            });
            assert_eq!(
                c,
                r,
                "DIVERGENCE js_eval flags={} {:?}:\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
                flags,
                src,
                c.0,
                String::from_utf8_lossy(&c.1),
                r.0,
                String::from_utf8_lossy(&r.1)
            );
        }
    }
    // and a set of sources that DO throw, so the error path is compared too
    let bad = [
        "(", "var 1", "with({}){}", "null.x", "throw 1", "undefined()", "1 2",
        "function f(a,a){'use strict'}", "'use strict'; delete x", "eval('(')",
    ];
    for src in bad {
        for flags in [0, JS_STRICT] {
            let owned = src.to_string();
            let (c, r) = both(|api, _| {
                let owned = owned.clone();
                run_protected(api, flags, move |api, J| unsafe {
                    let s = cs(&owned);
                    (api.js_pushstring)(J, s.as_ptr());
                    (api.js_eval)(J);
                    let res = cstr_string((api.js_tryrepr)(J, -1, cs("<R>").as_ptr()))
                        .unwrap_or_default();
                    (api.js_pop)(J, 1);
                    format!("eval -> {}", res)
                })
            });
            assert_eq!(
                c,
                r,
                "DIVERGENCE js_eval (throwing) flags={} {:?}:\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
                flags,
                src,
                c.0,
                String::from_utf8_lossy(&c.1),
                r.0,
                String::from_utf8_lossy(&r.1)
            );
        }
    }
}

#[test]
fn g11_g12_pcall_pconstruct_and_direct_call() {
    let scripts = [
        "print((function(a,b){return a+b})(1,2))",
        "print(new (function(){this.v=7})().v)",
        "function C(a){ this.a=a } var o=new C(3); print(o.a, o instanceof C)",
        "function C(){ return {other:1} } print(JSON.stringify(new C()))",
        "print(Math.max.apply(null,[3,1,2]))",
        "print((function(){ return arguments.length })(1,2,3))",
        "try { (function(){ throw new RangeError('r') })() } catch(e) { print(e.name, e.message) }",
        "try { (1)() } catch(e) { print(e.name, e.message) }",
        "try { new 1 } catch(e) { print(e.name, e.message) }",
        "var f = function(){ return this }; print(typeof f.call(null), typeof f.call(1))",
    ];
    diff_scripts_both_modes(&scripts);
}

#[test]
fn g14_g15_runlimit_and_memlimit_matrix() {
    let programs = [
        "var s=0; for (var i=0;i<20000;++i) s+=i; print(s)",
        "var a=[]; for (var i=0;i<3000;++i) a.push('x'+i); print(a.length)",
        "function f(n){ return n<=0?0:1+f(n-1) } print(f(200))",
        "print('a'.split('').length)",
        "var o={}; for (var i=0;i<500;++i) o['k'+i]=i; print(Object.keys(o).length)",
    ];
    for src in programs {
        for &rl in &[0, 1, 2, 100, 5000, 50000, -1] {
            for &ml in &[0, 1, 512, 65536, 1 << 20, 1 << 24, -1] {
                let (c, r) = both(|api, _| unsafe {
                    out_clear();
                    let J = new_state(api, 0);
                    (api.js_setlimit)(J, rl, ml);
                    let rc = (api.js_dostring)(J, cs(src).as_ptr());
                    (api.js_freestate)(J);
                    (rc, out_take())
                });
                assert_eq!(
                    c,
                    r,
                    "DIVERGENCE setlimit(run={}, mem={}) {:?}:\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
                    rl,
                    ml,
                    src,
                    c.0,
                    String::from_utf8_lossy(&c.1),
                    r.0,
                    String::from_utf8_lossy(&r.1)
                );
            }
        }
    }
}

#[test]
fn g16_gc_and_freestate() {
    diff("js_gc + js_freestate", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        for round in 0..8 {
            let src = format!(
                "var a=[]; for (var i=0;i<{};++i) a.push({{k:i, s:'v'+i, r:/x{}/g}}); \
                 print(a.length); a=null;",
                50 * (round + 1),
                round
            );
            out.push(format!("rc={}", (api.js_dostring)(J, cs(&src).as_ptr())));
            (api.js_gc)(J, 0);
            out.push(format!(
                "post-gc rc={}",
                (api.js_dostring)(J, cs("print('alive')").as_ptr())
            ));
        }
        (api.js_freestate)(J);
        out
    });
    // Also make sure repeated create/destroy cycles agree.
    diff("state churn", |api| unsafe {
        let mut out = Vec::new();
        for i in 0..25 {
            let J = new_state(api, if i % 2 == 0 { 0 } else { JS_STRICT });
            out.push(format!(
                "{} rc={}",
                i,
                (api.js_dostring)(J, cs("var q=[1,2,3]; print(q.length)").as_ptr())
            ));
            (api.js_gc)(J, 0);
            (api.js_freestate)(J);
        }
        out
    });
}

#[test]
fn g17_individual_init_entry_points() {
    // Each jsB_init* on a bare state (js_newstate already ran jsB_init, so this
    // exercises re-initialisation, which is what the C allows).
    let inits: &[(&str, fn(&Api) -> unsafe extern "C-unwind" fn(State))] = &[
        ("jsB_init", |a| a.jsB_init),
        ("jsB_initobject", |a| a.jsB_initobject),
        ("jsB_initarray", |a| a.jsB_initarray),
        ("jsB_initboolean", |a| a.jsB_initboolean),
        ("jsB_initdate", |a| a.jsB_initdate),
        ("jsB_initerror", |a| a.jsB_initerror),
        ("jsB_initfunction", |a| a.jsB_initfunction),
        ("jsB_initjson", |a| a.jsB_initjson),
        ("jsB_initmath", |a| a.jsB_initmath),
        ("jsB_initnumber", |a| a.jsB_initnumber),
        ("jsB_initregexp", |a| a.jsB_initregexp),
        ("jsB_initstring", |a| a.jsB_initstring),
    ];
    for (name, get) in inits {
        let (c, r) = both(|api, _| unsafe {
            out_clear();
            let J = new_state(api, 0);
            (get(api))(J);
            let rc = (api.js_dostring)(
                J,
                cs("print(typeof Object, typeof Array, typeof Boolean, typeof Date, \
                    typeof Error, typeof Function, typeof JSON, typeof Math, \
                    typeof Number, typeof RegExp, typeof String); \
                    print([3,1,2].sort().join(','), 'ab'.toUpperCase(), Math.max(1,2), \
                    JSON.stringify({a:1}), new Date(0).getTime(), /a/.test('a'));")
                    .as_ptr(),
            );
            (api.js_freestate)(J);
            (rc, out_take())
        });
        assert_eq!(
            c,
            r,
            "DIVERGENCE {} :\n  C   : rc={} {:?}\n  Rust: rc={} {:?}",
            name,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
    // jsB_propf / jsB_propn / jsB_props on a fresh object
    diff("jsB_propf/propn/props", |api| unsafe {
        let J = new_state(api, 0);
        (api.js_newobject)(J);
        bind_callbacks(api);
        (api.jsB_propf)(J, lit!("T.f"), Some(print_cb), 1);
        (api.jsB_propn)(J, cs("n").as_ptr(), 1.5);
        // jsB_props does `js_pushliteral(J, v)`, which stores the pointer WITHOUT
        // copying (JS_TLITSTR), so the value string must be 'static.
        (api.jsB_props)(J, cs("s").as_ptr(), lit!("str"));
        (api.js_setglobal)(J, cs("T").as_ptr());
        out_clear();
        let rc = (api.js_dostring)(
            J,
            cs("print(typeof T.f, T.n, T.s); var ks=[]; for (var k in T) ks.push(k); print(ks.join(','));")
                .as_ptr(),
        );
        let o = out_take();
        (api.js_freestate)(J);
        (rc, String::from_utf8_lossy(&o).to_string())
    });
}

#[test]
fn g18_intern_and_freestrings() {
    // NOTE: `jsS_freestrings` (jsintern.c:124) does NOT reset `J->strings`, and
    // `js_freestate` calls it again (jsgc.c:279). So calling it explicitly and
    // then interning again -- or then calling js_freestate -- is a use-after-
    // free / double-free IN THE C. It is therefore exercised in two ways:
    //   (a) implicitly, via js_freestate, in the main body below;
    //   (b) explicitly, in a state that is then deliberately LEAKED.
    diff("js_intern", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        let words = [
            "", "a", "b", "aa", "ab", "zzz", "alpha", "beta", "gamma", "a", "alpha",
            "0123456789abcdefghij", "\u{e9}", "\u{65e5}\u{672c}", "A", "z", "aaa", "abc",
            "abd", "abb", "b", "ba", "bb", "m", "n", "o", "p", "q",
        ];
        let mut ptrs: Vec<usize> = Vec::new();
        for w in words {
            let cw = cs(w);
            let p = (api.js_intern)(J, cw.as_ptr());
            ptrs.push(p as usize);
            out.push(format!("{:?} -> {:?}", w, cstr_string(p)));
        }
        // interning the same string twice must return the SAME pointer
        out.push(format!("a==a: {}", ptrs[1] == ptrs[9]));
        out.push(format!("alpha==alpha: {}", ptrs[6] == ptrs[10]));
        out.push(format!("a!=b: {}", ptrs[1] != ptrs[2]));
        // interning strings that already exist as literals in the source
        let rc = (api.js_dostring)(
            J,
            cs("var alpha=1, beta=2; print(alpha+beta); print('zzz'.length);").as_ptr(),
        );
        out.push(format!("rc={}", rc));
        for w in ["alpha", "beta", "zzz"] {
            let cw = cs(w);
            out.push(format!(
                "re-intern {:?} -> {:?}",
                w,
                cstr_string((api.js_intern)(J, cw.as_ptr()))
            ));
        }
        // js_freestate calls jsS_freestrings internally (case (a))
        (api.js_freestate)(J);
        out
    });

    // Case (b): explicit jsS_freestrings, then LEAK the state (no js_freestate,
    // because the C would double-free).
    diff("explicit jsS_freestrings", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        for w in ["zeta", "eta", "theta", "iota", "kappa", "a", "zz"] {
            let cw = cs(w);
            out.push(format!(
                "{:?} -> {:?}",
                w,
                cstr_string((api.js_intern)(J, cw.as_ptr()))
            ));
        }
        (api.jsS_freestrings)(J);
        out.push("freed".to_string());
        // deliberately NOT calling js_freestate: see the comment above.
        let _leaked = J; // intentionally leaked: see the comment above
        out
    });
}

#[test]
fn g19_js_buffer_growth() {
    // js_putc / js_puts / js_putm growing a js_Buffer well past its 64-byte
    // inline capacity. (jsi.h: struct js_Buffer { int n, m; char s[64]; })
    #[repr(C)]
    struct Hdr {
        n: c_int,
        m: c_int,
    }
    diff("js_Buffer growth", |api| unsafe {
        let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        let mut out = Vec::new();
        let mut sb: *mut c_void = std::ptr::null_mut();
        let mut rng = Rng::new(0xB0FF_1234);
        for i in 0..1500 {
            match rng.below(3) {
                0 => (api.js_putc)(J, &mut sb, (b'A' + (i % 26) as u8) as c_int),
                1 => {
                    let s = cs(&"xy".repeat(1 + (i % 5)));
                    (api.js_puts)(J, &mut sb, s.as_ptr())
                }
                _ => {
                    let s = cs("0123456789");
                    let n = 1 + (i % 10);
                    (api.js_putm)(J, &mut sb, s.as_ptr(), s.as_ptr().add(n))
                }
            }
            if i % 100 == 0 {
                let h = sb as *const Hdr;
                out.push(format!("i={} n={} m={}", i, (*h).n, (*h).m));
            }
        }
        (api.js_putc)(J, &mut sb, 0);
        let h = sb as *const Hdr;
        let data = (sb as *const u8).add(8);
        let bytes = std::slice::from_raw_parts(data, (*h).n as usize).to_vec();
        out.push(format!("final n={} m={}", (*h).n, (*h).m));
        out.push(format!("sha-ish={}", bytes.iter().fold(0u64, |a, &b| a.wrapping_mul(31).wrapping_add(b as u64))));
        out.push(String::from_utf8_lossy(&bytes).to_string());
        (api.js_free)(J, sb);
        (api.js_freestate)(J);
        out
    });
}

#[test]
fn g_malloc_realloc_free_strdup() {
    diff("js_malloc/js_realloc/js_free/js_strdup", |api| unsafe {
        let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        let mut out = Vec::new();
        for n in [1, 8, 64, 1024, 65536] {
            let p = (api.js_malloc)(J, n);
            out.push(format!("malloc({}) null={}", n, p.is_null()));
            let q = (api.js_realloc)(J, p, n * 2);
            out.push(format!("realloc({}) null={}", n * 2, q.is_null()));
            (api.js_free)(J, q);
        }
        for s in ["", "a", "hello world", "0123456789abcdefghij"] {
            let cstring = cs(s);
            let d = (api.js_strdup)(J, cstring.as_ptr());
            out.push(format!("strdup({:?}) -> {:?}", s, cstr_string(d)));
            (api.js_free)(J, d as *mut c_void);
        }
        // jsV_newmemstring
        for (s, n) in [("abcdef", 6), ("abcdef", 3), ("", 0), ("with\0nul", 8)] {
            let cstring = std::ffi::CString::new(s.replace('\0', "\u{1}")).unwrap();
            let ms = (api.jsV_newmemstring)(J, cstring.as_ptr(), n);
            out.push(format!("newmemstring({:?},{}) null={}", s, n, ms.is_null()));
        }
        (api.js_freestate)(J);
        out
    });
}

#[test]
fn g_newenvironment_and_currentfunction() {
    diff("jsR_newenvironment", |api| unsafe {
        let J = new_state(api, 0);
        let mut out = Vec::new();
        (api.js_newobject)(J);
        let vars = (api.js_toobject)(J, -1);
        let e1 = (api.jsR_newenvironment)(J, vars, std::ptr::null_mut());
        let e2 = (api.jsR_newenvironment)(J, vars, e1);
        out.push(format!("e1 null={} e2 null={}", e1.is_null(), e2.is_null()));
        (api.js_pop)(J, 1);
        // js_currentfunction / js_currentfunctiondata outside a call
        (api.js_currentfunction)(J);
        out.push(format!("currentfunction={}", slot(api, J, -1)));
        (api.js_pop)(J, 1);
        out.push(format!(
            "currentfunctiondata null={}",
            (api.js_currentfunctiondata)(J).is_null()
        ));
        (api.js_gc)(J, 0);
        (api.js_freestate)(J);
        out
    });
}
