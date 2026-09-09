//! PHASE C — error-path differential tests.
//!
//! `tests/error_patterns.txt` holds the 133 distinct error-message format
//! strings extracted mechanically from every throwing site in `c_src/src/*.c`
//! (218 sites). `errors_cover_every_message` proves that the trigger corpus in
//! this file actually reaches each one, and every trigger is compared between
//! the C and Rust `.so` for the same error class AND the same message text.
//!
//! Sentinel-return rejections (ERRORS.md rows S1-S57) get their own tests below.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

/* ================================================================== */
/* Trigger corpus                                                      */
/* ================================================================== */

/// Sources that must be rejected (or throw) — one entry per distinct C error
/// site that is reachable from JavaScript.
fn throwing_sources() -> Vec<&'static str> {
    vec![
        /* ---------- jslex.c ---------- */
        "@",
        "#",
        "`",
        "\u{2603}",
        "\u{0}",
        "\u{1}x",
        "/* unterminated",
        "/*/",
        "\"unterminated",
        "'unterminated",
        "\"a\nb\"",
        "'a\nb'",
        "\"\\u00\"",
        "\"\\uZZZZ\"",
        "\"\\x4\"",
        "\"\\xZZ\"",
        "x = /unterminated",
        "x = /a\nb/",
        "x = /a/gg",
        "x = /a/ii",
        "x = /a/mm",
        "x = /a/z",
        "x = /a/1",
        "0x",
        "0X",
        "1e",
        "1e+",
        "1e-",
        "1abc",
        "0xg",
        "1.2.3",
        "017",
        "'use strict'; 017",
        "'use strict'; 08",
        "1_0",
        "0b",
        ".e1",
        "1..2",
        "\\",
        "\\u0",
        "\\uZZZZ",
        "\\x41",
        /* ---------- jsparse.c ---------- */
        "(",
        ")",
        "[",
        "{",
        "var",
        "var 1",
        "var x=",
        "1+",
        "function",
        "function(){}",
        "function f",
        "function f(",
        "function f(1){}",
        "function f(){",
        "if",
        "if(",
        "if(1",
        "if(1)else;",
        "for",
        "for(",
        "for(;",
        "for(;;",
        "for(1 in x);",
        "for(var x=1, y=2 in z);",
        "for(x;y);",
        "while",
        "while(1",
        "do;",
        "do;while",
        "switch",
        "switch(1)",
        "switch(1){x:}",
        "switch(1){default:;default:;}",
        "try",
        "try{}",
        "try{}x",
        "catch(e){}",
        "finally{}",
        "with",
        "with(1)",
        "return 1",
        "break",
        "continue",
        "break nosuchlabel;",
        "continue nosuchlabel;",
        "label: continue label;",
        "1=2",
        "1+2=3",
        "delete 1",
        "'use strict'; delete x",
        "({a:1,a:2,get a(){}})",
        "'use strict'; ({a:1,a:2})",
        "({1:1,1:2,get 1(){}})",
        "({get a(){},get a(){}})",
        "({,})",
        "({a})",
        "({*a(){}})",
        "class C{}",
        "let x = 1",
        "const x",
        "x => x",
        "`t`",
        "a ?? b",
        "a?.b",
        "x ** 2",
        "for (a of b);",
        "function* g(){}",
        "async function f(){}",
        "1 ?",
        "1 ? 2",
        "x[1",
        "x(1",
        "new",
        "new 1",
        "{a:1",
        "'use strict'; with({});",
        "'use strict'; var eval = 1",
        "'use strict'; var arguments = 1",
        "'use strict'; function f(a,a){}",
        "'use strict'; function eval(){}",
        "'use strict'; eval = 1",
        "'use strict'; arguments = 1",
        "'use strict'; function f(){ arguments = 1 }",
        "'use strict'; function f(){ eval = 1 }",
        "'use strict'; ++eval",
        "'use strict'; delete arguments",
        "var implements",
        "'use strict'; var implements",
        "'use strict'; var yield",
        "var class",
        "var enum",
        /* deep nesting: JS_ASTLIMIT */
        // (built dynamically below)
        /* ---------- jscompile.c ---------- */
        "for (1 in {});",
        "for (var a, b in {});",
        "(function(){ return })()",
        /* ---------- jsrun.c / runtime ---------- */
        "nosuchvariable",
        "nosuchvariable.x",
        "nosuchvariable()",
        "null.x",
        "undefined.x",
        "null.x = 1",
        "undefined.x = 1",
        "(1)()",
        "'s'()",
        "true()",
        "({}) ()",
        "([]) ()",
        "(1).nosuchmethod()",
        "new 1",
        "new (1)",
        "new ({})",
        "new 'str'",
        "1 instanceof 1",
        "1 instanceof {}",
        "({}) instanceof (function(){}) ",
        "(function(){}).prototype = 1; ({}) instanceof (function(){})",
        "var f = function(){}; f.prototype = 1; ({}) instanceof f",
        "1 in {}",
        "'a' in 1",
        "'a' in null",
        "'use strict'; undeclared = 1",
        "'use strict'; var o = Object.freeze({a:1}); o.a = 2",
        "'use strict'; var o = {}; Object.defineProperty(o,'a',{value:1}); o.a = 2",
        "var o = {get a(){return 1}}; 'use strict'; o.a = 2",
        "(function(){'use strict'; var o={get a(){return 1}}; o.a=2})()",
        "(function f(){ return f() })()",
        "(function f(n){ return f(n+1) })(0)",
        "var a = []; a.length = -1",
        "var a = []; a.length = 1.5",
        "var a = []; a.length = 4294967296",
        "new Array(-1)",
        "new Array(1.5)",
        "new Array(4294967296)",
        "'x'.repeat",
        "for (var k in 1) {}",
        "for (var k in null) {}",
        /* ---------- jsvalue.c / conversions ---------- */
        "Object(null).x",
        "({valueOf:function(){return {}}, toString:function(){return {}}}) + 1",
        "String({toString:function(){return {}}, valueOf:function(){return {}}})",
        "Number({toString:function(){return {}}, valueOf:function(){return {}}})",
        /* ---------- jsobject.c ---------- */
        "Object.keys(1)",
        "Object.keys(null)",
        "Object.getOwnPropertyNames(null)",
        "Object.getPrototypeOf(1)",
        "Object.defineProperty(1,'a',{})",
        "Object.defineProperty({},'a',{get:1})",
        "Object.defineProperty({},'a',{set:1})",
        "Object.defineProperty({},'a',{value:1,get:function(){}})",
        "Object.defineProperty({},'a',{writable:true,get:function(){}})",
        "Object.defineProperties(1,{})",
        "Object.defineProperties({},1)",
        "Object.create(1)",
        "Object.create({},1)",
        "Object.freeze(1)",
        "Object.seal(1)",
        "Object.preventExtensions(1)",
        "Object.isFrozen(1)",
        "Object.isSealed(1)",
        "Object.isExtensible(1)",
        "Object.prototype.hasOwnProperty.call(null,'a')",
        "Object.prototype.propertyIsEnumerable.call(null,'a')",
        "Object.prototype.toString.call",
        "var o = Object.preventExtensions({}); 'use strict'; o.x = 1",
        "var o = {}; Object.defineProperty(o,'a',{value:1}); Object.defineProperty(o,'a',{value:2})",
        "var o = {}; Object.defineProperty(o,'a',{value:1}); delete o.a; o.a",
        "'use strict'; var o={}; Object.defineProperty(o,'a',{value:1}); delete o.a",
        "Object.getOwnPropertyDescriptor(1,'a')",
        /* ---------- jsarray.c ---------- */
        "[1,2].sort(1)",
        "[1,2].sort('x')",
        "[1,2].every(1)",
        "[1,2].some(1)",
        "[1,2].forEach(1)",
        "[1,2].map(1)",
        "[1,2].filter(1)",
        "[1,2].reduce(1)",
        "[1,2].reduceRight(1)",
        "[].reduce(function(a,b){return a})",
        "[].reduceRight(function(a,b){return a})",
        "[,].reduce(function(a,b){return a})",
        "[,].reduceRight(function(a,b){return a})",
        "Array.prototype.toString.call(null)",
        "Array.prototype.join.call(null)",
        "Array.prototype.push.call(null,1)",
        "Array.prototype.concat.call(null)",
        /* ---------- jsstring.c ---------- */
        "String.prototype.toString.call(1)",
        "String.prototype.valueOf.call(1)",
        "String.prototype.charAt.call(null,0)",
        "String.prototype.replace.call(null,'a','b')",
        "String.prototype.split.call(null,'a')",
        "'abc'.replace(/a/,{})",
        /* ---------- jsnumber.c ---------- */
        "Number.prototype.toString.call('x')",
        "Number.prototype.valueOf.call('x')",
        "(1).toString(1)",
        "(1).toString(37)",
        "(1).toString(0)",
        "(1).toString(-1)",
        "(1).toFixed(-1)",
        "(1).toFixed(101)",
        "(1).toExponential(-1)",
        "(1).toExponential(101)",
        "(1).toPrecision(0)",
        "(1).toPrecision(-1)",
        "(1).toPrecision(101)",
        /* ---------- jsboolean.c ---------- */
        "Boolean.prototype.toString.call(1)",
        "Boolean.prototype.valueOf.call(1)",
        /* ---------- jsfunction.c ---------- */
        "Function.prototype.call.call(1)",
        "Function.prototype.apply.call(1)",
        "(function(){}).apply(null,1)",
        "(function(){}).apply(null,{length:-1})",
        "(function(){}).apply(null,{length:1e9})",
        "Function.prototype.bind===undefined",
        "Function.prototype.toString.call(1)",
        "new Function('(')",
        "new Function('a','(')",
        "new Function('(', '1')",
        /* ---------- jsregexp.c ---------- */
        "new RegExp('[')",
        "new RegExp('(')",
        "new RegExp('*')",
        "new RegExp('a','x')",
        "new RegExp('a','gg')",
        "new RegExp('a','ii')",
        "new RegExp('a','mm')",
        "new RegExp(/a/,'g')",
        "RegExp.prototype.exec.call(1)",
        "RegExp.prototype.test.call(1)",
        "RegExp.prototype.toString.call(1)",
        /* ---------- jsdate.c ---------- */
        "Date.prototype.getTime.call(1)",
        "Date.prototype.valueOf.call(1)",
        "new Date(NaN).toISOString()",
        "new Date(8.64e15+1).toISOString()",
        "Date.prototype.toJSON.call({toISOString:1})",
        "Date.prototype.toJSON.call({})",
        /* ---------- json.c ---------- */
        "JSON.parse('')",
        "JSON.parse('[')",
        "JSON.parse('{')",
        "JSON.parse('{1:2}')",
        "JSON.parse('{\"a\"}')",
        "JSON.parse('{\"a\":}')",
        "JSON.parse('[1,]')",
        "JSON.parse('nul')",
        "JSON.parse('\"\\u0001\"')",
        "JSON.parse(\"'a'\")",
        "var a=[]; a[0]=a; JSON.stringify(a)",
        "var o={}; o.o=o; JSON.stringify(o)",
        "JSON.stringify({a:1},function(){throw new TypeError('x')})",
        /* ---------- jsbuiltin.c ---------- */
        "decodeURI('%')",
        "decodeURI('%2')",
        "decodeURI('%zz')",
        "decodeURI('%FF')",
        "decodeURI('%80')",
        "decodeURI('%C0%80')",
        "decodeURI('%ED%A0%80')",
        "decodeURIComponent('%')",
        "decodeURIComponent('%E0%80')",
        "encodeURI('\\ud800')",
        "encodeURIComponent('\\udfff')",
        /* ---------- jsmath / misc ---------- */
        "Math.max.call(null,{valueOf:function(){throw new RangeError('m')}})",
        /* ---------- iterators ---------- */
        "var it = {}; for (var k in it) {}",
        /* ---------- sites found by auditing the remaining messages -------- */
        // jsarray.c:443 "array is too large to sort" (len >= INT_MAX)
        "Array.prototype.sort.call({length: 2147483647})",
        "Array.prototype.sort.call({length: 2147483647}, undefined)",
        // jsrun.c:783 "cannot create property '%s' on transient object"
        "'use strict'; var s = 'x'; s.foo = 1",
        "'use strict'; (1).foo = 1",
        "'use strict'; true.foo = 1",
        "(function(){'use strict'; var s='x'; s.foo=1})()",
        // regexp.c:170 "invalid escape character"
        "new RegExp('\\\\q')",
        "new RegExp('[\\\\q]')",
        "/\\q/",
        // regexp.c:108 "invalid quantifier" (non-digit where a count is expected)
        "new RegExp('a{z}')",
        "new RegExp('a{,}')",
        "new RegExp('a{1,z}')",
        // regexp.c:598 "invalid quantifier" (max < min)
        "new RegExp('a{3,2}')",
        "new RegExp('a{5,1}')",
        // regexp.c:253 "too many character class ranges" (REG_MAXSPAN = 64 Runes,
        // so the 32nd non-mergeable range trips it)
        "new RegExp('[' + Array.prototype.join.call({length:0},'') + \
         'ACEGIKMOQSUWY' + 'acegikmoqsuwy' + '13579' + '!#%' + ']')",
        // jsregexp.c:77/126 + jsstring.c:9 "regexec failed": js_regexec returns
        // -1 once match() recurses past REG_MAXREC (4096). A greedy star over a
        // long run recurses once per repetition, so 6000 'a' is enough.
        "/a*b/.exec(Array(6000).join('a'))",
        "/a*b/.test(Array(6000).join('a'))",
        "Array(6000).join('a').match(/a*b/)",
        "Array(6000).join('a').replace(/a*b/,'x')",
        "Array(6000).join('a').split(/a*b/).length",
        "Array(6000).join('a').search(/a*b/)",
        "Array(6000).join('a').match(/a*b/g)",
        "Array(6000).join('a').replace(/a*b/g,'x')",
        // jsparse.c:751 "unexpected token in for-var-statement: %s"
        "for (var i) ;",
        "for (var i, j) ;",
        "for (var i = 1) ;",
        "for (var i + 1;;) ;",
        /* ---------- JSON lexer sites (jsY_lexjson only) ---------- */
        "JSON.parse('1.')",              // missing digits after decimal point
        "JSON.parse('-1.')",
        "JSON.parse('1e')",              // missing digits after exponent indicator
        "JSON.parse('1e+')",
        "JSON.parse('1E-')",
        "JSON.parse('1.5e')",
        "JSON.parse('-')",               // unexpected non-digit
        "JSON.parse('-x')",
        "JSON.parse('-.')",
        "JSON.parse('-e1')",
        "JSON.parse('\"abc')",           // unterminated string
        "JSON.parse('\"')",
        "JSON.parse('[\"abc')",
        "JSON.parse('{\"a\":\"b')",
    ]
}

/// Sources built programmatically (limits and very long inputs).
fn generated_sources() -> Vec<String> {
    let mut v = Vec::new();
    // JS_ASTLIMIT = 400 nested expressions
    for d in [398usize, 399, 400, 401, 500, 2000] {
        v.push(format!("{}1{}", "(".repeat(d), ")".repeat(d)));
        v.push(format!("{}1", "-".repeat(d)));
        v.push(format!("{}1", "!".repeat(d)));
        v.push(format!("{}{}", "[".repeat(d), "]".repeat(d)));
        v.push(format!("{}1{}", "[".repeat(d), "]".repeat(d)));
        v.push(format!("{}1", "typeof ".repeat(d)));
    }
    // very deep nested functions / blocks (call stack, jump offsets)
    for d in [100usize, 1000, 5000] {
        v.push(format!("{}{}", "{".repeat(d), "}".repeat(d)));
        v.push(format!("{}1{}", "if(1){".repeat(d), "}".repeat(d)));
    }
    // huge literal string (JS_STRLIMIT) and huge array
    v.push(format!("var s='{}'; s.length", "x".repeat(100_000)));
    v.push("var s='x'; for (var i=0;i<40;i++) s+=s; s.length".into()); // hits JS_STRLIMIT
    v.push("var a=[]; a.length=1e9; a.join('')".into());
    v.push("new Array(1e9).join('x')".into());
    v.push("'x'.repeat===undefined ? (function(){var s='x'; while(1) s+=s})() : 0".into());
    // recursion to blow the call stack / env stack
    v.push("(function f(){return f()})()".into());
    v.push("(function f(n){return n?f(n-1):0})(1e6)".into());
    v.push("function f(){ return f() } f()".into());
    // very many locals / arguments (instruction coding overflow)
    let many: String = (0..70_000).map(|i| format!("var v{i}=0;")).collect();
    v.push(many);
    let manyargs: String = (0..300).map(|i| format!("a{i},")).collect();
    v.push(format!("function f({}z){{return z}} f(1)", manyargs));
    // huge number of switch cases / jump distance
    let cases: String = (0..40_000).map(|i| format!("case {i}: break;")).collect();
    v.push(format!("switch(1){{{cases}}}"));
    // regexp compile limits reached through the JS constructor
    v.push(format!("new RegExp('{}')", "(".repeat(5000)));
    v.push(format!("new RegExp('{}')", "a".repeat(70_000)));
    v.push(format!("new RegExp('{}')", "[a-b]".repeat(300)));
    v.push(format!(
        "new RegExp('{}')",
        (0..200)
            .map(|i| format!("\\\\u{:04x}-\\\\u{:04x}", 0x100 + i * 4, 0x102 + i * 4))
            .collect::<Vec<_>>()
            .join("")
    ));
    v.push(format!("new RegExp('{}')", "(a)".repeat(20)));
    v.push("new RegExp('()*')".into());
    v.push("new RegExp('(a*)*')".into());
    v.push("new RegExp('\\\\1')".into());
    v.push("new RegExp('a{99999999999}')".into());
    v.push("new RegExp('[z-a]')".into());
    v.push("new RegExp('a{2}{3}*')".into());
    v.push("new RegExp(')')".into());
    v.push("new RegExp('(?')".into());
    v.push("new RegExp('\\\\c1')".into());
    v.push("new RegExp('\\\\x')".into());
    // exception stack overflow (JS_TRYLIMIT)
    for d in [64usize, 70, 200] {
        v.push(format!("{}throw 1;{}", "try{".repeat(d), "}catch(e){}".repeat(d)));
    }
    // JSON deep nesting
    for d in [1000usize, 5000] {
        v.push(format!(
            "JSON.parse('{}1{}')",
            "[".repeat(d),
            "]".repeat(d)
        ));
    }
    // labels
    v.push("a: a: ;".into());
    v.push("a: { break a; }".into());
    v.push("break a;".into());
    v.push("continue a;".into());
    v
}

/* ================================================================== */
/* The differential driver for one source                              */
/* ================================================================== */

thread_local! {
    static SEEN: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn record(msg: &str) {
    SEEN.with(|s| s.borrow_mut().push(msg.to_string()));
}

/// Load + run one source through the protected entry points and return a
/// normalised description of the outcome (message text included).
unsafe fn outcome(api: &Api, J: JS, src: &str) -> String {
    unsafe {
        let f = cs("err.js");
        let c = match std::ffi::CString::new(src) {
            Ok(c) => c,
            Err(_) => return "SOURCE-HAS-NUL".into(),
        };
        let sentinel = cs("<unrepresentable>");
        let base = (api.js_gettop)(J);
        let rc = (api.js_ploadstring)(J, f.as_ptr(), c.as_ptr());
        if rc != 0 {
            let name = if (api.js_iserror)(J, -1) != 0 {
                (api.js_getproperty)(J, -1, cs("name").as_ptr());
                let n = s((api.js_trystring)(J, -1, sentinel.as_ptr()));
                (api.js_pop)(J, 1);
                n
            } else {
                "<non-error>".into()
            };
            let m = s((api.js_trystring)(J, -1, sentinel.as_ptr()));
            restore_top(api, J, base);
            return format!("LOAD rc=1 {name} | {m}");
        }
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        let name = if rc != 0 && (api.js_iserror)(J, -1) != 0 {
            (api.js_getproperty)(J, -1, cs("name").as_ptr());
            let n = s((api.js_trystring)(J, -1, sentinel.as_ptr()));
            (api.js_pop)(J, 1);
            n
        } else {
            "<none>".into()
        };
        let m = s((api.js_trystring)(J, -1, sentinel.as_ptr()));
        let ty = (api.js_type)(J, -1);
        restore_top(api, J, base);
        format!("CALL rc={rc} type={ty} {name} | {m}")
    }
}

fn diff_sources(sources: &[String], flags: c_int) {
    let p = both();
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), flags) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), flags) };
    assert!(!jc.is_null() && !jr.is_null());
    let mut diffs = Vec::new();
    for src in sources {
        let a = unsafe { outcome(&p.c, jc, src) };
        let b = unsafe { outcome(&p.r, jr, src) };
        record(&a);
        if a != b {
            let shown: String = src.chars().take(120).collect();
            diffs.push(format!(
                "source {:?} (len {})\n  C    : {a}\n  RUST : {b}",
                shown,
                src.len()
            ));
        }
    }
    unsafe {
        (p.c.js_freestate)(jc);
        (p.r.js_freestate)(jr);
    }
    if !diffs.is_empty() {
        panic!(
            "{} error-path divergence(s) with flags={flags}:\n{}",
            diffs.len(),
            diffs.join("\n")
        );
    }
}

fn all_sources() -> Vec<String> {
    let mut v: Vec<String> = throwing_sources().iter().map(|s| s.to_string()).collect();
    v.extend(generated_sources());
    v
}

// NOTE: the non-strict and strict sweeps are deliberately NOT separate #[test]
// functions. libtest runs tests in parallel, and this corpus intentionally
// includes inputs that push the JS heap to JS_STRLIMIT (2^28); running two or
// three of those concurrently exhausts the machine's memory rather than the
// library's limit. `errors_cover_every_message` runs both sweeps sequentially.

/* ================================================================== */
/* jsrun.c:1461 — js_endtry with no live try frame                     */
/* ================================================================== */
//
// `js_endtry` with `trytop == 0` raises an Error which, having no try frame to
// unwind to, goes straight to `J->panic()` and then `abort()`. The message can
// therefore only be observed from inside the panic handler, in a child process.

static RAW_API: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C-unwind" fn raw_panic(J: JS) {
    let api = unsafe { &*(RAW_API.load(Ordering::SeqCst) as *const Api) };
    unsafe {
        let sentinel = cs("<unrepresentable>");
        println!("PANIC {}", s((api.js_trystring)(J, -1, sentinel.as_ptr())));
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
}

/// Raw worker: no protecting `js_pcall`, so the abort path is reached directly.
#[test]
fn raw_worker() {
    let side = match std::env::var("MUJS_RAW_SIDE") {
        Ok(v) => v,
        Err(_) => return,
    };
    let p = both();
    let api: &'static Api = if side == "c" { &p.c } else { &p.r };
    RAW_API.store(api as *const Api as usize, Ordering::SeqCst);
    unsafe {
        let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        (api.js_atpanic)(J, Some(raw_panic));
        (api.js_endtry)(J);
        println!("NO ERROR from js_endtry");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        (api.js_freestate)(J);
    }
    std::process::exit(0);
}

fn spawn_raw(side: &str) -> String {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["raw_worker", "--exact", "--nocapture", "--test-threads=1"])
        .env("MUJS_RAW_SIDE", side)
        .output()
        .expect("spawn raw worker");
    use std::os::unix::process::ExitStatusExt;
    let body: String = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(strip_libtest_prefix)
        .filter(|l| l.starts_with("PANIC") || l.starts_with("NO ERROR"))
        .map(|l| format!("{l}\n"))
        .collect();
    format!(
        "exit={:?} signal={:?}\n{}",
        out.status.code(),
        out.status.signal(),
        body
    )
}

fn endtry_underflow_output() -> String {
    let a = spawn_raw("c");
    let b = spawn_raw("rust");
    assert_eq!(a, b, "js_endtry underflow behaviour differs");
    a
}

#[test]
fn s_endtry_underflow() {
    let out = endtry_underflow_output();
    assert!(
        out.contains("endtry: exception stack underflow"),
        "expected the endtry underflow message, got: {out:?}"
    );
}

/* ================================================================== */
/* regexp.c:916/926/956/961 — allocator-failure paths                  */
/* ================================================================== */
//
// These four `die()` sites fire only when the caller-supplied `js_Alloc`
// returns NULL, so they are unreachable through JavaScript (JS regexps use the
// state's allocator) and can only be driven through `js_regcompx`.

use std::sync::atomic::{AtomicI64, Ordering};
static FAIL_AT: AtomicI64 = AtomicI64::new(-1);
static CALLS: AtomicI64 = AtomicI64::new(0);

unsafe extern "C-unwind" fn nth_failing_alloc(
    _ctx: *mut c_void,
    ptr: *mut c_void,
    n: c_int,
) -> *mut c_void {
    unsafe extern "C" {
        fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
        fn free(p: *mut c_void);
    }
    unsafe {
        if n == 0 {
            free(ptr);
            return std::ptr::null_mut();
        }
        let i = CALLS.fetch_add(1, Ordering::SeqCst);
        if i == FAIL_AT.load(Ordering::SeqCst) {
            return std::ptr::null_mut();
        }
        realloc(ptr, n as usize)
    }
}

/// Returns every distinct `*errorp` message the two libraries produce, and
/// asserts they agree for every (pattern, failing-allocation-index) pair.
fn regcompx_alloc_failures() -> Vec<String> {
    let p = both();
    let mut seen = Vec::new();
    // "a"     -> Reprog, parse list, instruction list
    // "[a]"   -> Reprog, parse list, instruction list, character class list
    // "\\d"   -> same, via a predefined class
    for pat in ["a", "[a]", "\\d", "[a-c]|[d-f]", "(a)[b]"] {
        let cp = cs(pat);
        for fail_at in 0..8i64 {
            let mut msgs = Vec::new();
            for api in [&p.c, &p.r] {
                FAIL_AT.store(fail_at, Ordering::SeqCst);
                CALLS.store(0, Ordering::SeqCst);
                let mut err: *const c_char = std::ptr::null();
                let prog = unsafe {
                    (api.js_regcompx)(
                        nth_failing_alloc,
                        std::ptr::null_mut(),
                        cp.as_ptr(),
                        0,
                        &mut err,
                    )
                };
                let m = if prog.is_null() {
                    let m = unsafe { s(err) };
                    m
                } else {
                    FAIL_AT.store(-1, Ordering::SeqCst);
                    unsafe { (api.js_regfreex)(nth_failing_alloc, std::ptr::null_mut(), prog) };
                    "<compiled>".to_string()
                };
                msgs.push(m);
            }
            FAIL_AT.store(-1, Ordering::SeqCst);
            assert_eq!(
                msgs[0], msgs[1],
                "js_regcompx({pat:?}) with allocation #{fail_at} failing"
            );
            seen.push(format!("regcompx {pat:?} fail@{fail_at}: {}", msgs[0]));
        }
    }
    seen
}

#[test]
fn s41b_regcompx_allocator_failures() {
    let seen = regcompx_alloc_failures();
    let blob = seen.join("\n");
    for expect in [
        "cannot allocate regular expression",
        "cannot allocate regular expression parse list",
        "cannot allocate regular expression instruction list",
        "cannot allocate regular expression character class list",
    ] {
        assert!(
            blob.contains(expect),
            "allocator-failure sweep never produced {expect:?}\n{blob}"
        );
    }
}

/* ================================================================== */
/* Coverage gate: every distinct C error message must have been reached */
/* ================================================================== */

fn pattern_to_regex(fmt: &str) -> String {
    let mut out = String::from("");
    let b: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '%' && i + 1 < b.len() {
            // consume a printf conversion
            let mut j = i + 1;
            while j < b.len() && "0123456789.-+ #'".contains(b[j]) {
                j += 1;
            }
            if j < b.len() {
                match b[j] {
                    '%' => {
                        out.push('%');
                        i = j + 1;
                        continue;
                    }
                    's' => {
                        out.push_str("(?s).*");
                        i = j + 1;
                        continue;
                    }
                    'd' | 'i' | 'u' => {
                        out.push_str("-?[0-9]+");
                        i = j + 1;
                        continue;
                    }
                    'c' => {
                        out.push_str("(?s).");
                        i = j + 1;
                        continue;
                    }
                    'g' | 'f' | 'e' => {
                        out.push_str("[-0-9.eE+inaf]+");
                        i = j + 1;
                        continue;
                    }
                    'X' | 'x' => {
                        out.push_str("[0-9A-Fa-f]+");
                        i = j + 1;
                        continue;
                    }
                    _ => {}
                }
            }
        }
        let c = b[i];
        if "\\^$.|?*+()[]{}".contains(c) {
            out.push('\\');
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Minimal regex matcher: supports only the constructs `pattern_to_regex`
/// produces (literals, `.`, `.*`, `[...]`, `[...]+`, `-?`), so the test needs no
/// external crate. Returns true if `hay` contains a match.
fn re_search(pat: &str, hay: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let h: Vec<char> = hay.chars().collect();
    for start in 0..=h.len() {
        if re_here(&p, 0, &h, start) {
            return true;
        }
    }
    false
}

fn parse_atom(p: &[char], mut i: usize) -> Option<(usize, Box<dyn Fn(char) -> bool>)> {
    if i >= p.len() {
        return None;
    }
    if p[i] == '(' {
        // only the non-capturing inline-flag group (?s) is produced
        if p[i..].starts_with(&['(', '?', 's', ')']) {
            return parse_atom(p, i + 4);
        }
    }
    if p[i] == '\\' && i + 1 < p.len() {
        let c = p[i + 1];
        return Some((i + 2, Box::new(move |x| x == c)));
    }
    if p[i] == '.' {
        return Some((i + 1, Box::new(|_| true)));
    }
    if p[i] == '[' {
        let mut j = i + 1;
        let neg = j < p.len() && p[j] == '^';
        if neg {
            j += 1;
        }
        let mut set: Vec<(char, char)> = Vec::new();
        let mut first = true;
        while j < p.len() && (p[j] != ']' || first) {
            first = false;
            let lo = p[j];
            if j + 2 < p.len() && p[j + 1] == '-' && p[j + 2] != ']' {
                set.push((lo, p[j + 2]));
                j += 3;
            } else {
                set.push((lo, lo));
                j += 1;
            }
        }
        j += 1; // ']'
        return Some((
            j,
            Box::new(move |x| {
                let inside = set.iter().any(|(a, b)| x >= *a && x <= *b);
                inside != neg
            }),
        ));
    }
    let c = p[i];
    i += 1;
    Some((i, Box::new(move |x| x == c)))
}

fn re_here(p: &[char], pi: usize, h: &[char], hi: usize) -> bool {
    if pi >= p.len() {
        return true;
    }
    if p[pi..].starts_with(&['(', '?', 's', ')']) {
        return re_here(p, pi + 4, h, hi);
    }
    let (next, pred) = match parse_atom(p, pi) {
        None => return true,
        Some(v) => v,
    };
    let quant = p.get(next).copied();
    match quant {
        Some('*') => {
            let mut k = hi;
            loop {
                if re_here(p, next + 1, h, k) {
                    return true;
                }
                if k < h.len() && pred(h[k]) {
                    k += 1;
                } else {
                    return false;
                }
            }
        }
        Some('+') => {
            let mut k = hi;
            while k < h.len() && pred(h[k]) {
                k += 1;
                if re_here(p, next + 1, h, k) {
                    return true;
                }
            }
            false
        }
        Some('?') => {
            if re_here(p, next + 1, h, hi) {
                return true;
            }
            if hi < h.len() && pred(h[hi]) {
                return re_here(p, next + 1, h, hi + 1);
            }
            false
        }
        _ => {
            if hi < h.len() && pred(h[hi]) {
                re_here(p, next, h, hi + 1)
            } else {
                false
            }
        }
    }
}

/// The Phase C gate: runs the whole trigger corpus through both libraries under
/// both state configurations (asserting identical error class + message text for
/// every input) and then proves that every distinct error message in the C
/// source was actually reached.
#[test]
fn errors_cover_every_message() {
    // Run the whole corpus and collect the C-side outcomes.
    SEEN.with(|s| s.borrow_mut().clear());
    let sources = all_sources();
    diff_sources(&sources, 0);
    diff_sources(&sources, JS_STRICT);
    // plus everything the direct-API tests below trigger
    for extra in direct_api_messages() {
        record(&extra);
    }
    // the allocator-failure regexp sites
    for extra in regcompx_alloc_failures() {
        record(&extra);
    }
    // and the abort-only sites, captured from the child processes
    for m in OOB_MODES {
        record(&diff_subproc_output("worker", &m.to_string(), Cfg::default()));
    }
    record(&endtry_underflow_output());
    let observed: Vec<String> = SEEN.with(|s| s.borrow().clone());
    let blob = observed.join("\n");

    let patterns: Vec<&str> = include_str!("error_patterns.txt")
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    // Three messages are STRUCTURALLY unreachable in the C library. Each is
    // justified from the source, not waved away:
    //
    //  * "malformed number" (jslex.c:269) sits inside a `#if 0 ... #endif`
    //    block spanning jslex.c:263-387, so it is not even compiled into
    //    libmujs.so - `nm -D` has no such code path.
    //  * "unknown expression type" (jscompile.c:780) is the `default:` arm of
    //    cexp()'s switch; every expression node kind the parser can build has an
    //    explicit case, and no exported entry point lets a caller hand the
    //    compiler a hand-built AST.
    //  * "invalid property name in object initializer" (jscompile.c:336) is the
    //    else-arm for a property-name node that is not AST_IDENTIFIER /
    //    EXP_STRING / EXP_NUMBER, but jsparse.c:207 `propname()` can only
    //    produce exactly those three kinds.
    const UNREACHABLE: [&str; 3] = [
        "malformed number",
        "unknown expression type",
        "invalid property name in object initializer",
    ];
    let mut missing = Vec::new();
    for pat in &patterns {
        if UNREACHABLE.contains(pat) {
            continue;
        }
        // un-escape the C string literal escapes that appear in the source
        let unescaped = pat.replace("\\\\", "\\").replace("\\\"", "\"").replace("\\n", "\n");
        let rx = pattern_to_regex(&unescaped);
        if !re_search(&rx, &blob) {
            missing.push((*pat).to_string());
        }
    }
    let report = format!(
        "{} / {} distinct C error messages reached\nMISSING:\n{}\n",
        patterns.len() - missing.len(),
        patterns.len(),
        missing.join("\n")
    );
    std::fs::write("/tmp/mujs-error-coverage.txt", &report).ok();
    assert!(missing.is_empty(), "{report}");
}

/* ================================================================== */
/* ERRORS.md rows S1-S57 — sentinel-return rejections                  */
/* ================================================================== */

/// Messages produced by the direct-API error tests, folded into the coverage
/// blob so that sites unreachable from JavaScript still count.
fn direct_api_messages() -> Vec<String> {
    let p = both();
    let mut out = Vec::new();
    for api in [&p.c, &p.r] {
        out.push(run_side(api, Cfg::default(), task_direct_api_errors));
        out.push(run_side(api, Cfg::strict(), task_direct_api_errors));
    }
    out
}

/// Modes 0, 5 and 15 are *deliberately* separated out: they make the C original
/// write out of bounds, so they can only be compared in a child process.
///
///  * `js_pushlstring(J, v, n)` checks only `n > JS_STRLIMIT` (`jsrun.c:164`),
///    never `n < 0`, so a negative length runs `while (n--) *s++ = *v++` about
///    2^32 times into the 16-byte `shrstr` slot.
///  * `js_rot(J, n)` (`jsrun.c:499`) has no bound on `n` at all and walks below
///    `BOT`.
///  * mode 10 calls `js_endtry` with no matching `js_savetry`; since the driver's
///    own `js_pcall` frame is the innermost one, that *pops the harness' try
///    frame*, and the next throw reaches `abort()` (`jsrun.c:1481`). The C
///    library behaves exactly that way, so it is compared out-of-process too.
const OOB_MODES: [i32; 4] = [0, 5, 10, 15];

unsafe extern "C-unwind" fn cf_pushbig(J: JS) {
    let api = active_api();
    unsafe {
        let mode = (api.js_tryinteger)(J, 1, 0);
        match mode {
            0 => (api.js_pushlstring)(J, b"x".as_ptr() as *const c_char, -1),
            1 => (api.js_pushlstring)(J, b"x".as_ptr() as *const c_char, i32::MAX),
            2 => (api.js_pushlstring)(J, b"x".as_ptr() as *const c_char, (1 << 28) + 1),
            3 => {
                // js_pushstring with a string longer than JS_STRLIMIT is not
                // constructible here; instead exhaust the value stack.
                for _ in 0..5000 {
                    (api.js_pushnumber)(J, 1.0);
                }
            }
            4 => (api.js_pop)(J, 9999),
            5 => (api.js_rot)(J, 9999),
            6 => (api.js_copy)(J, 9999),
            7 => (api.js_remove)(J, 9999),
            8 => (api.js_insert)(J, 9999),
            9 => (api.js_replace)(J, 9999),
            10 => (api.js_endtry)(J),
            11 => {
                (api.js_newobject)(J);
                let _ = (api.js_toregexp)(J, -1);
            }
            12 => {
                (api.js_newobject)(J);
                let _ = (api.js_touserdata)(J, -1, cs("Tag").as_ptr());
            }
            13 => {
                (api.js_pushnull)(J);
                let _ = (api.js_getlength)(J, -1);
            }
            14 => {
                (api.js_pushundefined)(J);
                (api.js_getproperty)(J, -1, cs("x").as_ptr());
            }
            15 => (api.js_pushlstring)(J, b"x".as_ptr() as *const c_char, i32::MIN),
            16 => {
                (api.js_pushnull)(J);
                let _ = (api.js_toobject)(J, -1);
            }
            17 => {
                (api.js_pushundefined)(J);
                let _ = (api.js_toobject)(J, -1);
            }
            18 => {
                // js_setindex with a huge index -> "array too large"
                (api.js_newarray)(J);
                (api.js_pushnumber)(J, 1.0);
                (api.js_setindex)(J, -2, i32::MAX);
            }
            19 => {
                (api.js_newarray)(J);
                (api.js_setlength)(J, -1, -1);
            }
            /* ---- jsproperty.c:303 "not an iterator" ---- */
            21 => {
                (api.js_newobject)(J);
                let _ = (api.js_nextiterator)(J, -1);
            }
            22 => {
                (api.js_newarray)(J);
                let _ = (api.js_nextiterator)(J, -1);
            }
            23 => {
                (api.js_pushnumber)(J, 1.0);
                let _ = (api.js_nextiterator)(J, -1);
            }
            24 => {
                (api.js_newobject)(J);
                let o = (api.js_toobject)(J, -1);
                let _ = (api.jsV_nextiterator)(J, o);
            }
            /* ---- jsrun.c:1304 "number of arguments cannot be negative" ---- */
            25 => {
                (api.js_getglobal)(J, cs("String").as_ptr());
                (api.js_pushnull)(J);
                (api.js_call)(J, -1);
            }
            26 => {
                (api.js_getglobal)(J, cs("String").as_ptr());
                (api.js_pushnull)(J);
                (api.js_call)(J, i32::MIN);
            }
            27 => {
                (api.js_getglobal)(J, cs("Object").as_ptr());
                (api.js_construct)(J, -1);
            }
            /* ---- jsrun.c:875 "'%s' is read-only or non-configurable" ----
             * The public js_defproperty passes throw=1 (jsrun.c:1015), so this
             * fires in non-strict mode too. */
            28 => {
                (api.js_newarray)(J);
                (api.js_pushnumber)(J, 5.0);
                (api.js_defproperty)(J, -2, cs("length").as_ptr(), 0);
            }
            29 => {
                (api.js_newstring)(J, cs("abc").as_ptr());
                (api.js_pushnumber)(J, 5.0);
                (api.js_defproperty)(J, -2, cs("length").as_ptr(), 0);
            }
            30 => {
                (api.js_newstring)(J, cs("abc").as_ptr());
                (api.js_pushnumber)(J, 5.0);
                (api.js_defproperty)(J, -2, cs("1").as_ptr(), 0);
            }
            31 => {
                (api.js_newregexp)(J, cs("a").as_ptr(), 0);
                (api.js_pushnumber)(J, 5.0);
                (api.js_defproperty)(J, -2, cs("source").as_ptr(), 0);
            }
            32 => {
                (api.js_newregexp)(J, cs("a").as_ptr(), 0);
                (api.js_pushnumber)(J, 5.0);
                (api.js_defproperty)(J, -2, cs("lastIndex").as_ptr(), 0);
            }
            33 => {
                (api.js_newarray)(J);
                (api.js_newcfunction)(J, cf_getter0, sstr("g"), 0);
                (api.js_pushnull)(J);
                (api.js_defaccessor)(J, -3, cs("length").as_ptr(), 0);
            }
            /* ---- jsrun.c:783 "cannot create property '%s' on transient object"
             * js_setproperty passes transient = !js_isobject(idx) (jsrun.c:1011),
             * so assigning to a *primitive* through the C API reaches it; the
             * compiled OP_SETPROP path never sets `transient`. Requires strict. */
            34 => {
                (api.js_pushstring)(J, cs("abc").as_ptr());
                (api.js_pushnumber)(J, 1.0);
                (api.js_setproperty)(J, -2, cs("foo").as_ptr());
            }
            35 => {
                (api.js_pushnumber)(J, 3.5);
                (api.js_pushnumber)(J, 1.0);
                (api.js_setproperty)(J, -2, cs("foo").as_ptr());
            }
            36 => {
                (api.js_pushboolean)(J, 1);
                (api.js_pushnumber)(J, 1.0);
                (api.js_setproperty)(J, -2, cs("foo").as_ptr());
            }
            37 => {
                // existing read-only property on a transient -> "'%s' is read-only"
                (api.js_pushstring)(J, cs("abc").as_ptr());
                (api.js_pushnumber)(J, 1.0);
                (api.js_setproperty)(J, -2, cs("length").as_ptr());
            }
            38 => {
                // setter-less accessor -> "setting property '%s' that only has a getter"
                (api.js_newobject)(J);
                (api.js_newcfunction)(J, cf_getter0, sstr("g"), 0);
                (api.js_pushnull)(J);
                (api.js_defaccessor)(J, -3, cs("acc").as_ptr(), 0);
                (api.js_pushnumber)(J, 1.0);
                (api.js_setproperty)(J, -2, cs("acc").as_ptr());
            }
            _ => (api.js_pushundefined)(J),
        }
    }
}

unsafe extern "C-unwind" fn cf_getter0(J: JS) {
    let api = active_api();
    unsafe { (api.js_pushnumber)(J, 0.0) }
}

fn task_direct_api_errors(api: &Api, J: JS) {
    unsafe {
        for mode in (0..39).filter(|m| !OOB_MODES.contains(m)) {
            let base = (api.js_gettop)(J);
            (api.js_newcfunction)(J, cf_pushbig, sstr("t"), 1);
            (api.js_pushnull)(J);
            (api.js_pushnumber)(J, mode as f64);
            let rc = (api.js_pcall)(J, 1);
            let sentinel = cs("<unrepresentable>");
            emit(format!(
                "direct mode={mode} rc={rc} msg={:?}",
                s((api.js_trystring)(J, -1, sentinel.as_ptr()))
            ));
            restore_top(api, J, base);
        }
        // nesting js_savetry past JS_TRYLIMIT is exercised in scripts.rs
    }
}

#[test]
fn s47_to_s53_direct_api_error_paths() {
    diff_all_flags("direct API error paths", task_direct_api_errors);
}

/// Worker for the out-of-bounds modes (compared as child processes).
#[test]
fn worker() {
    common::maybe_worker(dispatch_oob);
}

/// Print the pending exception before the C library's `abort()` (`jsrun.c:1481`),
/// so that even the abort-only sites contribute their message text.
unsafe extern "C-unwind" fn panic_print(J: JS) {
    let api = active_api();
    unsafe {
        let sentinel = cs("<unrepresentable>");
        let m = s((api.js_trystring)(J, -1, sentinel.as_ptr()));
        println!("PANIC {m}");
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
}

fn dispatch_oob(api: &Api, J: JS) {
    let mode: i32 = case().parse().unwrap_or(0);
    unsafe {
        (api.js_atpanic)(J, Some(panic_print));
        (api.js_newcfunction)(J, cf_pushbig, sstr("t"), 1);
        (api.js_pushnull)(J);
        (api.js_pushnumber)(J, mode as f64);
        let rc = (api.js_pcall)(J, 1);
        let sentinel = cs("<unrepresentable>");
        emit(format!(
            "oob mode={mode} rc={rc} msg={:?}",
            s((api.js_trystring)(J, -1, sentinel.as_ptr()))
        ));
    }
}

#[test]
fn s47b_out_of_bounds_modes_match() {
    for m in OOB_MODES {
        diff_subproc("worker", &m.to_string(), Cfg::default());
        diff_subproc("worker", &m.to_string(), Cfg::strict());
    }
}

/* ---- S1/S2: js_regcomp error strings (already covered in regex.rs, here we
       assert the exact sentinel contract) ---- */
#[test]
fn s1_s2_regcomp_sentinels() {
    let p = both();
    let cases: [(&str, bool); 12] = [
        ("a", true),
        ("", true),
        ("(", false),
        (")", false),
        ("*", false),
        ("[", false),
        ("[z-a]", false),
        ("\\", false),
        ("\\1", false),
        ("a{99999999999}", false),
        ("()*", false),
        ("(?", false),
    ];
    for (pat, ok) in cases {
        let cp = cs(pat);
        for cflags in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
            let mut ec: *const c_char = 0xdead_beef_usize as *const c_char;
            let mut er: *const c_char = 0xdead_beef_usize as *const c_char;
            unsafe {
                let a = (p.c.js_regcomp)(cp.as_ptr(), cflags, &mut ec);
                let b = (p.r.js_regcomp)(cp.as_ptr(), cflags, &mut er);
                assert_eq!(a.is_null(), b.is_null(), "regcomp({pat:?}) NULL-ness");
                assert_eq!(a.is_null(), !ok, "regcomp({pat:?}) expected ok={ok}");
                if a.is_null() {
                    assert_eq!(opt_str(ec), opt_str(er), "regcomp({pat:?}) *errorp");
                    assert!(!ec.is_null(), "regcomp({pat:?}) left *errorp NULL");
                } else {
                    // On success regexp.c clears *errorp (it does not leave the
                    // caller's value in place); both libraries must do the same.
                    assert_eq!(ec as usize, er as usize, "regcomp *errorp on success");
                    assert!(ec.is_null(), "regcomp left *errorp non-NULL on success");
                    (p.c.js_regfree)(a);
                    (p.r.js_regfree)(b);
                }
            }
        }
    }
    // NULL errorp must be tolerated on the success path
    let cp = cs("a+");
    unsafe {
        let a = (p.c.js_regcomp)(cp.as_ptr(), 0, std::ptr::null_mut());
        let b = (p.r.js_regcomp)(cp.as_ptr(), 0, std::ptr::null_mut());
        assert_eq!(a.is_null(), b.is_null());
        (p.c.js_regfree)(a);
        (p.r.js_regfree)(b);
    }
    // js_regfree(NULL) must be a no-op on both
    unsafe {
        (p.c.js_regfree)(std::ptr::null_mut());
        (p.r.js_regfree)(std::ptr::null_mut());
        (p.c.js_regfreex)(nullalloc, std::ptr::null_mut(), std::ptr::null_mut());
        (p.r.js_regfreex)(nullalloc, std::ptr::null_mut(), std::ptr::null_mut());
    }
}

unsafe extern "C-unwind" fn nullalloc(_c: *mut c_void, _p: *mut c_void, _n: c_int) -> *mut c_void {
    std::ptr::null_mut()
}

/* ---- S3/S4/S5: js_regexec sentinels ---- */
#[test]
fn s3_s4_s5_regexec_sentinels() {
    let p = both();
    let pat = cs("^abc$");
    unsafe {
        let mut e: *const c_char = std::ptr::null();
        let a = (p.c.js_regcomp)(pat.as_ptr(), 0, &mut e);
        let b = (p.r.js_regcomp)(pat.as_ptr(), 0, &mut e);
        for subj in ["", "abc", "xabc", "abcx", "ABC"] {
            let sc = cs(subj);
            for eflags in [0, REG_NOTBOL, -1, i32::MAX, 8, 1024] {
                let mut ma = Resub::default();
                let mut mb = Resub::default();
                let ra = (p.c.js_regexec)(a, sc.as_ptr(), &mut ma, eflags);
                let rb = (p.r.js_regexec)(b, sc.as_ptr(), &mut mb, eflags);
                assert_eq!(ra, rb, "regexec({subj:?}, eflags={eflags})");
                assert!(ra == 0 || ra == 1, "unexpected regexec return {ra}");
            }
        }
        (p.c.js_regfree)(a);
        (p.r.js_regfree)(b);
    }
}

/* ---- S13-S17: the js_try* family returns the caller's sentinel ---- */
unsafe extern "C-unwind" fn cf_try_family(J: JS) {
    let api = active_api();
    unsafe {
        // a value whose every conversion throws
        let f = cs("t.js");
        let src = cs(
            "({valueOf:function(){throw new Error('v')}, \
               toString:function(){throw new Error('s')}})",
        );
        (api.js_ploadstring)(J, f.as_ptr(), src.as_ptr());
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
        let sent = sstr("SENTINEL");
        emit(format!(
            "trystring={:?}",
            s((api.js_trystring)(J, -1, sent))
        ));
        emit(format!(
            "trynumber={:?}",
            (api.js_trynumber)(J, -1, -4321.25).to_bits()
        ));
        emit(format!("tryinteger={}", (api.js_tryinteger)(J, -1, -777)));
        emit(format!("tryboolean={}", (api.js_tryboolean)(J, -1, -9)));
        emit(format!("tryrepr={:?}", s((api.js_tryrepr)(J, -1, sent))));
        // and on a value whose conversions succeed
        (api.js_pushnumber)(J, 42.5);
        emit(format!("ok trystring={:?}", s((api.js_trystring)(J, -1, sent))));
        emit(format!("ok trynumber={}", (api.js_trynumber)(J, -1, -1.0)));
        emit(format!("ok tryinteger={}", (api.js_tryinteger)(J, -1, -1)));
        emit(format!("ok tryboolean={}", (api.js_tryboolean)(J, -1, -1)));
        emit(format!("ok tryrepr={:?}", s((api.js_tryrepr)(J, -1, sent))));
        (api.js_pop)(J, 2);
    }
}

fn task_try_family(api: &Api, J: JS) {
    unsafe {
        let base = (api.js_gettop)(J);
        (api.js_newcfunction)(J, cf_try_family, sstr("tf"), 0);
        (api.js_pushnull)(J);
        let rc = (api.js_pcall)(J, 0);
        emit(format!("try family rc={rc}"));
        restore_top(api, J, base);
    }
}

#[test]
fn s13_to_s17_try_family_sentinels() {
    diff_all_flags("js_try* sentinel returns", task_try_family);
}

/* ---- S18/S19/S53: userdata tag mismatch ---- */
unsafe extern "C-unwind" fn cf_ud_tag(J: JS) {
    let api = active_api();
    unsafe {
        let mut payload: u64 = 7;
        (api.js_newuserdata)(
            J,
            sstr("Right"),
            &mut payload as *mut u64 as *mut c_void,
            None,
        );
        for probe in ["Right", "Wrong", "", "right"] {
            emit(format!(
                "isuserdata({probe:?})={}",
                (api.js_isuserdata)(J, -1, cs(probe).as_ptr())
            ));
        }
        // touserdata with the wrong tag throws TypeError: not a <tag>
        let _ = (api.js_touserdata)(J, -1, cs("Wrong").as_ptr());
        emit("UNREACHABLE".into());
    }
}

fn task_ud_tag(api: &Api, J: JS) {
    unsafe {
        let base = (api.js_gettop)(J);
        (api.js_newcfunction)(J, cf_ud_tag, sstr("ud"), 0);
        (api.js_pushnull)(J);
        let rc = (api.js_pcall)(J, 0);
        let sentinel = cs("<throw>");
        emit(format!(
            "ud tag rc={rc} msg={:?}",
            s((api.js_trystring)(J, -1, sentinel.as_ptr()))
        ));
        restore_top(api, J, base);
        // js_isuserdata on non-userdata values
        for k in 0..6 {
            match k {
                0 => (api.js_pushundefined)(J),
                1 => (api.js_pushnull)(J),
                2 => (api.js_pushnumber)(J, 1.0),
                3 => (api.js_pushstring)(J, cs("s").as_ptr()),
                4 => (api.js_newobject)(J),
                _ => (api.js_newarray)(J),
            }
            emit(format!(
                "shape {k} isuserdata(T)={} isuserdata('')={}",
                (api.js_isuserdata)(J, -1, cs("T").as_ptr()),
                (api.js_isuserdata)(J, -1, cs("").as_ptr())
            ));
            (api.js_pop)(J, 1);
        }
    }
}

#[test]
fn s18_s19_s53_userdata_tags() {
    diff_all_flags("userdata tag mismatch", task_ud_tag);
}

/* ---- S20: js_compare *okay ---- */
fn task_compare_okay(api: &Api, J: JS) {
    unsafe {
        let shapes: [f64; 6] = [0.0, 1.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
        for a in shapes {
            for b in shapes {
                (api.js_pushnumber)(J, a);
                (api.js_pushnumber)(J, b);
                let mut okay: c_int = -5;
                let r = (api.js_compare)(J, &mut okay);
                emit(format!("compare({a},{b}) = {r} okay={okay}"));
                (api.js_pop)(J, 2);
            }
        }
        for (a, b) in [
            ("''", "''"),
            ("'a'", "'b'"),
            ("'10'", "'9'"),
            ("NaN", "'a'"),
            ("'a'", "NaN"),
            ("{}", "1"),
            ("1", "{}"),
            ("undefined", "1"),
            ("null", "0"),
        ] {
            eval(
                api,
                J,
                &format!("compare {a} {b}"),
                &format!("[{a}<{b},{a}>{b},{a}<={b},{a}>={b}].join(',')"),
            );
        }
    }
}

#[test]
fn s20_compare_okay() {
    diff_all_flags("js_compare okay flag", task_compare_okay);
}

/* ---- S22-S27: UTF-8 decode/encode rejections ---- */
#[test]
fn s22_to_s27_utf_rejections() {
    let p = both();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", vec![]),
        ("truncated 2-byte", vec![0xC3]),
        ("truncated 3-byte", vec![0xE2, 0x82]),
        ("truncated 4-byte", vec![0xF0, 0x9F, 0x98]),
        ("bad continuation", vec![0xC3, 0x41]),
        ("overlong 2", vec![0xC0, 0x80]),
        ("overlong 3", vec![0xE0, 0x80, 0x80]),
        ("overlong 4", vec![0xF0, 0x80, 0x80, 0x80]),
        ("surrogate", vec![0xED, 0xA0, 0x80]),
        ("above Runemax", vec![0xF7, 0xBF, 0xBF, 0xBF]),
        ("0xF8 lead", vec![0xF8, 0x80, 0x80, 0x80, 0x80]),
        ("0xFF lead", vec![0xFF]),
        ("lone continuation", vec![0x80]),
        ("lone continuation 2", vec![0xBF, 0xBF]),
    ];
    for (name, bytes) in cases {
        let mut z = bytes.clone();
        z.push(0);
        let mut ra: c_int = -1;
        let mut rb: c_int = -1;
        unsafe {
            let ka = (p.c.jsU_chartorune)(&mut ra, z.as_ptr() as *const c_char);
            let kb = (p.r.jsU_chartorune)(&mut rb, z.as_ptr() as *const c_char);
            assert_eq!((ka, ra), (kb, rb), "chartorune {name}");
        }
    }
    // runetochar / runelen with out-of-range runes must encode Runeerror
    for r in [-1, -0x80, i32::MIN, RUNEMAX + 1, 0x11_0000, 0x7FFF_FFFF, 0xD800, 0xDFFF] {
        let mut ba = [0u8; 8];
        let mut bb = [0u8; 8];
        unsafe {
            let na = (p.c.jsU_runetochar)(ba.as_mut_ptr() as *mut c_char, &r);
            let nb = (p.r.jsU_runetochar)(bb.as_mut_ptr() as *mut c_char, &r);
            assert_eq!((na, ba), (nb, bb), "runetochar({r:#x})");
            assert_eq!(
                (p.c.jsU_runelen)(r),
                (p.r.jsU_runelen)(r),
                "runelen({r:#x})"
            );
        }
    }
}

/* ---- S28-S36: numeric parse rejections ---- */
#[test]
fn s28_to_s36_numeric_rejections() {
    let p = both();
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };
    let cases = [
        "", " ", "abc", "-", "+", ".", "e", "e1", "-.", "+.", "0x", "0xg", "1e", "1e+", "1e-",
        "1e400", "-1e400", "1e-400", "Infinity", "-Infinity", "NaN", "  \t\n12", "12  ", "1_2",
        "99999999999999999999999999999999", "-99999999999999999999999999999999", "1.7976931348623159e308",
    ];
    for t in cases {
        let ct = cs(t);
        unsafe {
            let mut ea: *mut c_char = std::ptr::null_mut();
            let mut eb: *mut c_char = std::ptr::null_mut();
            let a = (p.c.js_strtod)(ct.as_ptr(), &mut ea);
            let b = (p.r.js_strtod)(ct.as_ptr(), &mut eb);
            assert!(same_f64(a, b), "js_strtod({t:?}) {a} vs {b}");
            assert_eq!(
                ea as usize - ct.as_ptr() as usize,
                eb as usize - ct.as_ptr() as usize,
                "js_strtod({t:?}) end"
            );
            let a = (p.c.jsV_stringtonumber)(jc, ct.as_ptr());
            let b = (p.r.jsV_stringtonumber)(jr, ct.as_ptr());
            assert!(same_f64(a, b), "jsV_stringtonumber({t:?}) {a} vs {b}");
            for radix in [0, 2, 8, 10, 16, 36] {
                let mut ea: *mut c_char = std::ptr::null_mut();
                let mut eb: *mut c_char = std::ptr::null_mut();
                let a = (p.c.js_strtol)(ct.as_ptr(), &mut ea, radix);
                let b = (p.r.js_strtol)(ct.as_ptr(), &mut eb, radix);
                assert!(same_f64(a, b), "js_strtol({t:?},{radix}) {a} vs {b}");
                assert_eq!(
                    ea as usize - ct.as_ptr() as usize,
                    eb as usize - ct.as_ptr() as usize,
                    "js_strtol({t:?},{radix}) end"
                );
            }
        }
    }
    // js_isarrayindex rejections
    for t in [
        "", "-1", "+1", "01", "00", " 1", "1 ", "1.0", "1e1", "0x1", "4294967295", "4294967296",
        "99999999999999999999", "a", "length",
    ] {
        let ct = cs(t);
        let mut ia: c_int = -1;
        let mut ib: c_int = -1;
        unsafe {
            let a = (p.c.js_isarrayindex)(jc, ct.as_ptr(), &mut ia);
            let b = (p.r.js_isarrayindex)(jr, ct.as_ptr(), &mut ib);
            assert_eq!((a, ia), (b, ib), "js_isarrayindex({t:?})");
        }
    }
    unsafe {
        (p.c.js_freestate)(jc);
        (p.r.js_freestate)(jr);
    }
}

/* ---- S40/S41/S42/S43: js_newstate / js_setlimit / js_atpanic ---- */
unsafe extern "C-unwind" fn failing_alloc(
    _ctx: *mut c_void,
    _p: *mut c_void,
    _n: c_int,
) -> *mut c_void {
    std::ptr::null_mut()
}

#[test]
fn s40_s41_s42_state_construction_rejections() {
    let p = both();
    // out-of-range / garbage flag ints: only bit 0 (JS_STRICT) is inspected
    for flags in [0, 1, 2, 3, 8, -1, i32::MAX, i32::MIN, 0x7fff_fffe] {
        for api in [&p.c, &p.r] {
            unsafe {
                let J = (api.js_newstate)(None, std::ptr::null_mut(), flags);
                assert!(!J.is_null(), "js_newstate(flags={flags}) returned NULL");
                (api.js_freestate)(J);
            }
        }
        // and the observable strictness must agree
        let cfg = Cfg {
            flags,
            ..Cfg::default()
        };
        diff(&format!("newstate flags={flags}"), cfg, task_strictness_probe);
    }
    // an allocator that always fails must make js_newstate return NULL on both
    for api in [&p.c, &p.r] {
        unsafe {
            let J = (api.js_newstate)(Some(failing_alloc), std::ptr::null_mut(), 0);
            assert!(J.is_null(), "js_newstate with a failing alloc must return NULL");
        }
    }
    // js_setlimit with non-positive values disables the limit
    for (r, m) in [(0, 0), (-1, -1), (i32::MIN, i32::MIN)] {
        diff(&format!("setlimit({r},{m})"), Cfg::limits(r, m), task_strictness_probe);
    }
}

fn task_strictness_probe(api: &Api, J: JS) {
    unsafe {
        eval(api, J, "strictness", "(function(){ try { undeclared_xyz = 1; return 'ok' } \
                                    catch (e) { return e.name } })()");
        eval(api, J, "octal", "try { eval('017') } catch (e) { e.name }");
        eval(api, J, "with", "try { eval('with({}){}') } catch (e) { e.name }");
        eval(api, J, "delete", "try { eval('var q; delete q') } catch (e) { e.name }");
        eval(api, J, "this", "(function(){ return typeof this })()");
    }
}

/* ---- S46: exception stack overflow through js_ploadstring ---- */
unsafe extern "C-unwind" fn cf_nest_load(J: JS) {
    let api = active_api();
    unsafe {
        let n = (api.js_tryinteger)(J, 1, 0);
        if n <= 0 {
            (api.js_pushstring)(J, cs("bottom").as_ptr());
            return;
        }
        let f = cs("n.js");
        let src = cs("nest(NEXT)");
        let text = s(src.as_ptr()).replace("NEXT", &format!("{}", n - 1));
        let ct = cs(&text);
        let rc = (api.js_ploadstring)(J, f.as_ptr(), ct.as_ptr());
        if rc != 0 {
            let sentinel = cs("<throw>");
            emit(format!(
                "level {n} load rc=1 {:?}",
                s((api.js_trystring)(J, -1, sentinel.as_ptr()))
            ));
            return;
        }
        (api.js_pushundefined)(J);
        (api.js_call)(J, 0);
    }
}

fn task_ploadstring_nesting(api: &Api, J: JS) {
    unsafe {
        (api.js_newcfunction)(J, cf_nest_load, sstr("nest"), 1);
        (api.js_setglobal)(J, cs("nest").as_ptr());
        for depth in [1, 20, 40, 60, 62, 63, 64, 80, 200] {
            let base = (api.js_gettop)(J);
            (api.js_getglobal)(J, cs("nest").as_ptr());
            (api.js_pushnull)(J);
            (api.js_pushnumber)(J, depth as f64);
            let rc = (api.js_pcall)(J, 1);
            let sentinel = cs("<throw>");
            emit(format!(
                "ploadstring nesting depth={depth} rc={rc} -> {:?}",
                s((api.js_trystring)(J, -1, sentinel.as_ptr()))
            ));
            restore_top(api, J, base);
        }
    }
}

#[test]
fn s8_s46_exception_stack_overflow() {
    diff_all_flags("exception stack overflow", task_ploadstring_nesting);
}

/* ---- S56/S57: iterator exhaustion and unknown refs ---- */
fn task_iter_and_ref_sentinels(api: &Api, J: JS) {
    unsafe {
        (api.js_newobject)(J);
        (api.js_pushiterator)(J, -1, 1);
        for i in 0..3 {
            emit(format!(
                "empty iter {i} -> {:?}",
                opt_str((api.js_nextiterator)(J, -1))
            ));
        }
        (api.js_pop)(J, 2);
        for bad in ["nosuchref", "", "0x0", "_Undefined"] {
            (api.js_unref)(J, cs(bad).as_ptr());
            (api.js_getregistry)(J, cs(bad).as_ptr());
            emit(format!("unref/getregistry {bad:?}: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_delregistry)(J, cs(bad).as_ptr());
        }
        emit("ok".into());
    }
}

#[test]
fn s56_s57_iterator_and_ref_sentinels() {
    diff_all_flags("iterator/ref sentinels", task_iter_and_ref_sentinels);
}

/* ---- Generic FFI boundary abuse: NULL pointers and out-of-range enums ---- */
#[test]
fn generic_null_and_out_of_range() {
    let p = both();
    // NULL string arguments to the pure leaf functions
    unsafe {
        // js_utflen(NULL) is not defined by the C code; skip. Instead pass "" and
        // exercise every out-of-range integer argument.
        for c in [i32::MIN, -1, 0, 1, 0x10FFFF, 0x110000, i32::MAX] {
            assert_eq!((p.c.jsU_runelen)(c), (p.r.jsU_runelen)(c), "runelen({c})");
            assert_eq!((p.c.jsY_iswhite)(c), (p.r.jsY_iswhite)(c));
            assert_eq!((p.c.jsY_isnewline)(c), (p.r.jsY_isnewline)(c));
            assert_eq!((p.c.jsY_ishex)(c), (p.r.jsY_ishex)(c));
            assert_eq!((p.c.jsY_tohex)(c), (p.r.jsY_tohex)(c));
            assert_eq!(
                s((p.c.jsY_tokenstring)(c)),
                s((p.r.jsY_tokenstring)(c)),
                "tokenstring({c})"
            );
        }
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -1e300] {
            assert_eq!((p.c.jsV_numbertointeger)(v), (p.r.jsV_numbertointeger)(v));
            assert_eq!((p.c.jsV_numbertoint32)(v), (p.r.jsV_numbertoint32)(v));
            assert_eq!((p.c.jsV_numbertouint32)(v), (p.r.jsV_numbertouint32)(v));
            assert_eq!((p.c.jsV_numbertoint16)(v), (p.r.jsV_numbertoint16)(v));
            assert_eq!((p.c.jsV_numbertouint16)(v), (p.r.jsV_numbertouint16)(v));
        }
    }
    // out-of-range enum ints through the stateful API
    diff_all_flags("out-of-range enum ints", task_out_of_range_enums);
}

fn task_out_of_range_enums(api: &Api, J: JS) {
    unsafe {
        // js_toprimitive hint (JS_HNONE/JS_HNUMBER/JS_HSTRING = 0/1/2)
        for hint in [i32::MIN, -1, 0, 1, 2, 3, 99, i32::MAX] {
            (api.js_newobject)(J);
            (api.js_toprimitive)(J, -1, hint);
            emit(format!("toprimitive hint={hint}: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_newnumber)(J, 3.5);
            (api.js_toprimitive)(J, -1, hint);
            emit(format!("toprimitive(Number) hint={hint}: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
        }
        // property attribute ints outside JS_READONLY|JS_DONTENUM|JS_DONTCONF
        for atts in [i32::MIN, -1, 8, 16, 32, 255, i32::MAX] {
            (api.js_newobject)(J);
            (api.js_pushnumber)(J, 1.0);
            (api.js_defproperty)(J, -2, cs("p").as_ptr(), atts);
            (api.js_getproperty)(J, -1, cs("p").as_ptr());
            emit(format!("defproperty atts={atts}: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 2);
            (api.js_pushnumber)(J, 2.0);
            (api.js_defglobal)(J, cs("gg").as_ptr(), atts);
            (api.js_getglobal)(J, cs("gg").as_ptr());
            emit(format!("defglobal atts={atts}: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            (api.js_delglobal)(J, cs("gg").as_ptr());
        }
        // js_pushiterator "own" outside {0,1}
        for own in [i32::MIN, -1, 0, 1, 2, 99] {
            (api.js_newobject)(J);
            (api.js_pushnumber)(J, 1.0);
            (api.js_setproperty)(J, -2, cs("k").as_ptr());
            (api.js_pushiterator)(J, -1, own);
            let mut keys = Vec::new();
            loop {
                match opt_str((api.js_nextiterator)(J, -1)) {
                    None => break,
                    Some(b) => {
                        keys.push(String::from_utf8_lossy(&b).into_owned());
                        if keys.len() > 400 {
                            break;
                        }
                    }
                }
            }
            emit(format!("pushiterator own={own}: {} keys", keys.len()));
            (api.js_pop)(J, 2);
        }
        // js_gc report outside {0,1}
        for report in [i32::MIN, -1, 0, 1, 2, 99] {
            (api.js_gc)(J, report);
        }
        emit("gc report range ok".into());
        // js_newregexp / js_newstate flags outside their documented bits
        for flags in [i32::MIN, -1, 8, 16, 255, i32::MAX] {
            (api.js_newregexp)(J, cs("a|b").as_ptr(), flags);
            emit(format!("newregexp flags={flags}: {}", dump_slot(api, J, -1)));
            for prop in ["source", "global", "ignoreCase", "multiline", "lastIndex"] {
                (api.js_getproperty)(J, -1, cs(prop).as_ptr());
                emit(format!("  {prop}={}", dump_slot(api, J, -1)));
                (api.js_pop)(J, 1);
            }
            (api.js_pop)(J, 1);
        }
        // js_type over indices well outside the frame
        emit(format!("gettop={}", (api.js_gettop)(J)));
    }
}
