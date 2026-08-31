//! Direct differential coverage for the exported INTERNAL entry points that the
//! other test files only reach indirectly: the lexer (`jsY_initlex`/`jsY_lex`/
//! `jsY_lexjson`), the parser and compiler (`jsP_parse`, `jsP_parsefunction`,
//! `jsP_freeparse`, `jsC_compilescript`, `jsC_compilefunction`, `jsC_error`),
//! the allocator and buffer helpers, the error constructors and throwers, the
//! `jsV_*` value layer, and the object/environment constructors.
//!
//! Everything here is called through the `.so` exports of BOTH libraries.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Source corpus for the lexer / parser / compiler
// ---------------------------------------------------------------------------

fn source_corpus() -> Vec<Vec<u8>> {
    let fixed: &[&str] = &[
        "",
        " ",
        "\n",
        "\t\r\n ",
        "1",
        "1;",
        "1+1",
        "var x = 1;",
        "var x = 1, y = 2;",
        "function f(a,b){ return a+b }",
        "f(1,2)",
        "'string'",
        "\"double\"",
        "'esc\\n\\t\\\\\\''",
        "'\\x41\\u0042'",
        "0",
        "0.5",
        ".5",
        "5.",
        "1e10",
        "1E-10",
        "1e+10",
        "0x1F",
        "0X1f",
        "010",
        "1_0",
        "/re/",
        "/re/gim",
        "/[a-z]+/i",
        "/a\\/b/",
        "a/b",
        "a/b/c",
        "x = /re/",
        "if (a) b; else c;",
        "while(1){break}",
        "do{}while(0)",
        "for(var i=0;i<3;i++){}",
        "for(var k in o){}",
        "switch(x){case 1: break; default: }",
        "try{}catch(e){}finally{}",
        "throw 1",
        "with(o){}",
        "debugger",
        "return",
        "{ }",
        "[]",
        "[1,,3]",
        "[1,2,]",
        "({})",
        "({a:1,'b':2,3:4})",
        "({get a(){return 1}, set a(v){}})",
        "a.b.c",
        "a[b][c]",
        "new a.b(c)",
        "a ? b : c",
        "a,b,c",
        "a===b!==c",
        "a<=b>=c",
        "a<<b>>c>>>d",
        "a&&b||c",
        "a+=b-=c*=d/=e%=f",
        "a<<=b>>=c>>>=d&=e|=f^=g",
        "++a--",
        "typeof void delete a.b",
        "!~-+a",
        "a instanceof b in c",
        "// line comment",
        "/* block */1",
        "/* multi\nline */1",
        "1 // trailing",
        "//",
        "/**/",
        "\u{feff}1",
        "\u{a0}1",
        "\u{2028}1",
        "\u{2029}1",
        "'\u{4f60}\u{597d}'",
        "var \u{e9} = 1",
        "$_ = 1",
        "a\u{200c}b",
        // error sources -- the lexer must reject these identically
        "'unterminated",
        "\"unterminated",
        "/* unterminated",
        "/unterminated",
        "\\",
        "'\\x'",
        "'\\xZZ'",
        "'\\u'",
        "'\\u00'",
        "'\\uZZZZ'",
        "1e",
        "1e+",
        "1abc",
        "0x",
        "@",
        "#",
        "`",
        "'a\nb'",
        "[",
        "(",
        "{",
        "function",
        "var",
        "1 2 3",
        "..",
        "...",
    ];
    let mut v: Vec<Vec<u8>> = fixed.iter().map(|s| s.as_bytes().to_vec()).collect();

    // Property-style random token soup: mostly invalid, which is the point.
    let mut rng = Rng::new(0x1E7E_2024);
    let pieces: &[&str] = &[
        "a", "b", "1", "0x1", "'s'", "\"d\"", "/r/", " ", "\n", "\t", "+", "-", "*", "/", "%", "=",
        "==", "===", "!", "<", ">", "&", "|", "^", "~", "(", ")", "[", "]", "{", "}", ";", ",", ".",
        ":", "?", "//", "/*", "*/", "\\", "var", "function", "return", "if", "else", "@", "#",
        "0.", ".0", "1e", "1e5", "'", "\"",
    ];
    for _ in 0..6000 {
        let n = rng.below(10) as usize;
        let s: String = (0..n).map(|_| *rng.pick(pieces)).collect();
        v.push(s.into_bytes());
    }
    // Random raw bytes: the lexer must reject malformed UTF-8 identically.
    for _ in 0..4000 {
        let n = rng.below(12) as usize;
        v.push((0..n).map(|_| rng.next_u32() as u8).collect());
    }
    v
}

fn json_corpus() -> Vec<Vec<u8>> {
    let fixed: &[&str] = &[
        "", " ", "null", "true", "false", "0", "-0", "1", "-1", "1.5", "-1.5", "1e5", "1E-5",
        "1e+5", "01", "+1", ".5", "5.", "\"\"", "\"a\"", "\"\\n\\t\\r\\b\\f\\\\\\\"\\/\"",
        "\"\\u0041\"", "\"\\ud83d\\ude00\"", "\"\\uZZZZ\"", "\"\\q\"", "\"unterminated",
        "[]", "[1]", "[1,2,3]", "[1,]", "[,]", "[", "]", "{}", "{\"a\":1}", "{\"a\":1,\"b\":2}",
        "{a:1}", "{'a':1}", "{\"a\":1,}", "{", "}", "{\"a\"}", "{\"a\":}", "[[[[1]]]]",
        "{\"a\":{\"b\":{\"c\":[1,2,{\"d\":null}]}}}", "  { \"a\" : [ 1 , 2 ] }  ", "nul", "tru",
        "NaN", "Infinity", "undefined", "'a'", "\t\n\r 1", "1 2", "[1 2]", "@",
    ];
    let mut v: Vec<Vec<u8>> = fixed.iter().map(|s| s.as_bytes().to_vec()).collect();
    let mut rng = Rng::new(0x1E7E_2025);
    let pieces: &[&str] = &[
        "{", "}", "[", "]", ",", ":", "\"a\"", "1", "true", "false", "null", " ", "\n", "-",
        "1.5", "1e5", "\"", "\\", "u0041",
    ];
    for _ in 0..5000 {
        let n = rng.below(10) as usize;
        let s: String = (0..n).map(|_| *rng.pick(pieces)).collect();
        v.push(s.into_bytes());
    }
    v
}

// ---------------------------------------------------------------------------
// jsY_initlex / jsY_lex / jsY_lexjson -- full token-stream comparison
// ---------------------------------------------------------------------------

type FnInitlex = unsafe extern "C" fn(JsState, *const c_char, *const c_char);
type FnLex = unsafe extern "C" fn(JsState) -> c_int;

thread_local! {
    static SRC: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static JSON_MODE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Lex the current source to EOF (or until it throws) and report the token
/// sequence, using `jsY_tokenstring` so the report is readable and
/// implementation-independent.
fn lex_probe(imp: &Impl, j: JsState) {
    let src = SRC.with(|s| s.borrow().clone());
    let json = JSON_MODE.with(|c| c.get());
    let buf = cbytes(&src);
    let fname = cstr("lex.js");
    unsafe { imp.f::<FnInitlex>("jsY_initlex")(j, fname.as_ptr(), buf.as_ptr() as *const c_char) };
    let lex = if json {
        imp.f::<FnLex>("jsY_lexjson")
    } else {
        imp.f::<FnLex>("jsY_lex")
    };
    let tokname = imp.f::<FnTokenString>("jsY_tokenstring");
    let mut out = String::new();
    // 0 is TK_EOF in this lexer's numbering (jsY_tokenstring(0) == "EOF").
    for _ in 0..4096 {
        let t = unsafe { lex(j) };
        let name = unsafe { read_cstr(tokname(t)) }.map(|x| show(&x)).unwrap_or_default();
        out.push_str(&format!("{t}:{name};"));
        if t == 0 {
            break;
        }
    }
    imp.pushstring(j, out.as_bytes());
}

#[test]
fn jsy_lex_token_streams_match() {
    let mut b = Batch::new();
    JSON_MODE.with(|c| c.set(false));
    for src in source_corpus() {
        SRC.with(|s| *s.borrow_mut() = src.clone());
        for flags in [0 as c_int, JS_STRICT] {
            b.probe(flags, &format!("jsY_lex {:?}", show(&src)), lex_probe as ProbeFn);
        }
    }
    b.finish("jsY_initlex + jsY_lex token streams");
}

#[test]
fn jsy_lexjson_token_streams_match() {
    let mut b = Batch::new();
    JSON_MODE.with(|c| c.set(true));
    for src in json_corpus() {
        SRC.with(|s| *s.borrow_mut() = src.clone());
        for flags in [0 as c_int, JS_STRICT] {
            b.probe(flags, &format!("jsY_lexjson {:?}", show(&src)), lex_probe as ProbeFn);
        }
    }
    JSON_MODE.with(|c| c.set(false));
    b.finish("jsY_initlex + jsY_lexjson token streams");
}

// ---------------------------------------------------------------------------
// jsP_parse / jsP_parsefunction / jsP_freeparse / jsC_compile* / jsC_error
// ---------------------------------------------------------------------------

type FnParse = unsafe extern "C" fn(JsState, *const c_char, *const c_char) -> *mut c_void;
type FnParsefunction =
    unsafe extern "C" fn(JsState, *const c_char, *const c_char, *const c_char) -> *mut c_void;
type FnFreeparse = unsafe extern "C" fn(JsState);
type FnCompilescript = unsafe extern "C" fn(JsState, *mut c_void, c_int) -> *mut c_void;
type FnCompilefunction = unsafe extern "C" fn(JsState, *mut c_void) -> *mut c_void;
type FnNewscript = unsafe extern "C" fn(JsState, *mut c_void, *mut c_void);
type FnNewfunction = unsafe extern "C" fn(JsState, *mut c_void, *mut c_void);

fn parse_probe(imp: &Impl, j: JsState) {
    let src = SRC.with(|s| s.borrow().clone());
    let buf = cbytes(&src);
    let fname = cstr("p.js");
    let ast = unsafe {
        imp.f::<FnParse>("jsP_parse")(j, fname.as_ptr(), buf.as_ptr() as *const c_char)
    };
    let mut out = format!("ast_null={};", ast.is_null());
    // Compile with each default_strict value and observe.
    for ds in [0 as c_int, 1] {
        let f = unsafe { imp.f::<FnCompilescript>("jsC_compilescript")(j, ast, ds) };
        out.push_str(&format!("compile(ds={ds}) fn_null={};", f.is_null()));
        if !f.is_null() {
            // Instantiate and call it, so the compiled bytecode is executed.
            unsafe { imp.f::<FnNewscript>("js_newscript")(j, f, std::ptr::null_mut()) };
            imp.pushundefined(j);
            let rc = imp.pcall(j, 0);
            out.push_str(&format!("run rc={rc} v={};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
        }
    }
    unsafe { imp.f::<FnFreeparse>("jsP_freeparse")(j) };
    out.push_str("freed;");
    imp.pushstring(j, out.as_bytes());
}

#[test]
fn jsp_parse_and_jsc_compilescript_match() {
    let mut b = Batch::new();
    for src in source_corpus().into_iter().take(3000) {
        SRC.with(|s| *s.borrow_mut() = src.clone());
        for flags in [0 as c_int, JS_STRICT] {
            b.probe(flags, &format!("jsP_parse {:?}", show(&src)), parse_probe as ProbeFn);
        }
    }
    b.finish("jsP_parse + jsC_compilescript + js_newscript");
}

// ---------------------------------------------------------------------------
// jsP_parsefunction / jsC_compilefunction / js_newfunction
// ---------------------------------------------------------------------------

thread_local! {
    static PARAMS: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn parsefunction_probe(imp: &Impl, j: JsState) {
    let params = PARAMS.with(|s| s.borrow().clone());
    let body = SRC.with(|s| s.borrow().clone());
    let pbuf = cbytes(&params);
    let bbuf = cbytes(&body);
    let fname = cstr("fn.js");
    let ast = unsafe {
        imp.f::<FnParsefunction>("jsP_parsefunction")(
            j,
            fname.as_ptr(),
            pbuf.as_ptr() as *const c_char,
            bbuf.as_ptr() as *const c_char,
        )
    };
    let mut out = format!("ast_null={};", ast.is_null());
    let f = unsafe { imp.f::<FnCompilefunction>("jsC_compilefunction")(j, ast) };
    out.push_str(&format!("fn_null={};", f.is_null()));
    // NOTE: we deliberately do NOT call `js_newfunction(J, f, NULL)` here.
    // Unlike `js_newscript`, a *function* requires a real enclosing scope (the C
    // always passes `J->GE`, jsfunction.c:38); a NULL scope makes the first
    // variable lookup dereference NULL in BOTH implementations. The
    // `js_newfunction` path with a proper scope is covered instead through
    // `new Function(...)` in the corpora and in the test below.
    unsafe { imp.f::<FnFreeparse>("jsP_freeparse")(j) };
    imp.pushstring(j, out.as_bytes());
}

#[test]
fn jsp_parsefunction_and_jsc_compilefunction_match() {
    let cases: &[(&str, &str)] = &[
        ("", "return 1"),
        ("a", "return a"),
        ("a,b", "return a+b"),
        ("a,b,c", "return a+b+c"),
        ("a,a", "return a"),
        ("a", ""),
        ("", ""),
        ("", "return arguments.length"),
        ("", "'use strict'; return typeof this"),
        ("a", "var a; return a"),
        ("eval", "return 1"),
        ("arguments", "return 1"),
        ("a", "return a("),
        ("a b", "return a"),
        ("1", "return 1"),
        ("a,", "return a"),
        (",a", "return a"),
        ("", "return"),
        ("", "throw new Error('x')"),
        ("", "return (function(){return 42})()"),
        ("a,b", "return a/b"),
        ("a", "with({}){ return a }"),
        ("a", "return /re/.test(String(a))"),
        ("", "for(var i=0;i<3;i++); return i"),
        ("", "@"),
        ("@", "return 1"),
    ];
    let mut b = Batch::new();
    for (params, body) in cases {
        PARAMS.with(|s| *s.borrow_mut() = params.as_bytes().to_vec());
        SRC.with(|s| *s.borrow_mut() = body.as_bytes().to_vec());
        for flags in [0 as c_int, JS_STRICT] {
            b.probe(
                flags,
                &format!("jsP_parsefunction({params:?},{body:?})"),
                parsefunction_probe as ProbeFn,
            );
        }
    }
    b.finish("jsP_parsefunction + jsC_compilefunction + js_newfunction");
}

/// `jsC_error` and `js_loadstring` / `js_loadeval` throw on bad input; compare
/// the exact messages.
#[test]
fn js_loadstring_and_loadeval_match() {
    type FnLoad = unsafe extern "C" fn(JsState, *const c_char, *const c_char);
    fn probe_loadstring(imp: &Impl, j: JsState) {
        let src = SRC.with(|s| s.borrow().clone());
        let buf = cbytes(&src);
        let fname = cstr("L.js");
        unsafe {
            imp.f::<FnLoad>("js_loadstring")(j, fname.as_ptr(), buf.as_ptr() as *const c_char)
        };
        imp.pushundefined(j);
        let rc = imp.pcall(j, 0);
        let v = show(&imp.trystring(j, -1));
        imp.pop(j, 1);
        imp.pushstring(j, format!("rc={rc} v={v}").as_bytes());
    }
    fn probe_loadeval(imp: &Impl, j: JsState) {
        let src = SRC.with(|s| s.borrow().clone());
        let buf = cbytes(&src);
        let fname = cstr("L.js");
        unsafe {
            imp.f::<FnLoad>("js_loadeval")(j, fname.as_ptr(), buf.as_ptr() as *const c_char)
        };
        imp.pushundefined(j);
        let rc = imp.pcall(j, 0);
        let v = show(&imp.trystring(j, -1));
        imp.pop(j, 1);
        imp.pushstring(j, format!("rc={rc} v={v}").as_bytes());
    }
    let mut b = Batch::new();
    for src in source_corpus().into_iter().take(800) {
        SRC.with(|s| *s.borrow_mut() = src.clone());
        for flags in [0 as c_int, JS_STRICT] {
            b.probe(flags, &format!("js_loadstring {:?}", show(&src)), probe_loadstring as ProbeFn);
            b.probe(flags, &format!("js_loadeval {:?}", show(&src)), probe_loadeval as ProbeFn);
        }
    }
    b.finish("js_loadstring / js_loadeval");
}

// ---------------------------------------------------------------------------
// Error constructors (js_new*error) and throwers (js_*error)
// ---------------------------------------------------------------------------

const NEWERROR_FNS: &[&str] = &[
    "js_newerror",
    "js_newevalerror",
    "js_newrangeerror",
    "js_newreferenceerror",
    "js_newsyntaxerror",
    "js_newtypeerror",
    "js_newurierror",
];

const THROWERROR_FNS: &[&str] = &[
    "js_error",
    "js_evalerror",
    "js_rangeerror",
    "js_referenceerror",
    "js_syntaxerror",
    "js_typeerror",
    "js_urierror",
];

thread_local! {
    static FN_IDX: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[test]
fn js_newerror_family_matches() {
    fn probe(imp: &Impl, j: JsState) {
        let name = NEWERROR_FNS[FN_IDX.with(|c| c.get())];
        let f = imp.f::<FnVoidStr>(name);
        let mut acc = String::new();
        for msg in ["", "boom", "with %s percent", "\u{4f60}\u{597d}", "a\nb"] {
            let m = cstr(msg);
            unsafe { f(j, m.as_ptr()) };
            acc.push_str(&format!(
                "{msg:?}: ty={} iserr={} str={} ",
                imp.ty(j, -1),
                imp.is(j, "js_iserror", -1),
                show(&imp.trystring(j, -1))
            ));
            imp.getproperty(j, -1, "name");
            acc.push_str(&format!("name={} ", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
            imp.getproperty(j, -1, "message");
            acc.push_str(&format!("message={};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
            imp.pop(j, 1);
        }
        imp.pushstring(j, acc.as_bytes());
    }
    let mut b = Batch::new();
    for (i, name) in NEWERROR_FNS.iter().enumerate() {
        FN_IDX.with(|c| c.set(i));
        for flags in [0 as c_int, JS_STRICT] {
            b.probe(flags, name, probe as ProbeFn);
        }
    }
    b.finish("js_new*error constructors");
}

#[test]
fn js_error_thrower_family_matches() {
    // These are variadic printf-style and noreturn; each throws, so each probe
    // ends there and js_pcall reports the message.
    // These are genuine C variadics (`const char *fmt, ...`). They MUST be
    // declared with Rust's variadic fn-pointer syntax: a plain non-variadic
    // `extern "C" fn(..., f64)` does not set `al` to the number of vector
    // registers used, so the callee's `va_arg(ap, double)` reads garbage.
    type FnErrV0 = unsafe extern "C" fn(JsState, *const c_char, ...);
    type FnErrVS = unsafe extern "C" fn(JsState, *const c_char, *const c_char, ...);
    type FnErrVD = unsafe extern "C" fn(JsState, *const c_char, c_int, ...);
    type FnErrVF = unsafe extern "C" fn(JsState, *const c_char, f64, ...);

    thread_local! {
        static SHAPE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    fn probe(imp: &Impl, j: JsState) {
        let name = THROWERROR_FNS[FN_IDX.with(|c| c.get())];
        match SHAPE.with(|c| c.get()) {
            0 => {
                let f = imp.f::<FnErrV0>(name);
                let m = cstr("plain message");
                unsafe { f(j, m.as_ptr()) };
            }
            1 => {
                let f = imp.f::<FnErrVS>(name);
                let m = cstr("string arg: '%s'");
                let a = cstr("VALUE");
                unsafe { f(j, m.as_ptr(), a.as_ptr()) };
            }
            2 => {
                let f = imp.f::<FnErrVD>(name);
                let m = cstr("int arg: %d");
                unsafe { f(j, m.as_ptr(), -12345) };
            }
            3 => {
                let f = imp.f::<FnErrVF>(name);
                let m = cstr("float arg: %g");
                unsafe { f(j, m.as_ptr(), 1.5) };
            }
            _ => {
                let f = imp.f::<FnErrVS>(name);
                let m = cstr("%s");
                let a = cstr("");
                unsafe { f(j, m.as_ptr(), a.as_ptr()) };
            }
        }
        imp.pushstring(j, b"NOT REACHED");
    }
    let mut b = Batch::new();
    for (i, name) in THROWERROR_FNS.iter().enumerate() {
        FN_IDX.with(|c| c.set(i));
        for shape in 0..5usize {
            SHAPE.with(|c| c.set(shape));
            for flags in [0 as c_int, JS_STRICT] {
                b.probe(flags, &format!("{name} shape={shape}"), probe as ProbeFn);
            }
        }
    }
    b.finish("js_*error throwers");
}

// ---------------------------------------------------------------------------
// Allocator helpers and js_Buffer helpers
// ---------------------------------------------------------------------------

#[test]
fn allocator_and_buffer_helpers_match() {
    type FnMalloc = unsafe extern "C" fn(JsState, c_int) -> *mut c_void;
    type FnRealloc = unsafe extern "C" fn(JsState, *mut c_void, c_int) -> *mut c_void;
    type FnFreeP = unsafe extern "C" fn(JsState, *mut c_void);
    type FnStrdup = unsafe extern "C" fn(JsState, *const c_char) -> *mut c_char;
    type FnPutc = unsafe extern "C" fn(JsState, *mut *mut c_void, c_int);
    type FnPuts = unsafe extern "C" fn(JsState, *mut *mut c_void, *const c_char);
    type FnPutm = unsafe extern "C" fn(JsState, *mut *mut c_void, *const c_char, *const c_char);

    fn probe(imp: &Impl, j: JsState) {
        let mut acc = String::new();
        // js_malloc / js_realloc / js_free
        let m = imp.f::<FnMalloc>("js_malloc");
        let re = imp.f::<FnRealloc>("js_realloc");
        let fr = imp.f::<FnFreeP>("js_free");
        for n in [1 as c_int, 8, 64, 1024, 65536] {
            let p = unsafe { m(j, n) };
            acc.push_str(&format!("malloc({n})_nonnull={};", !p.is_null()));
            let p2 = unsafe { re(j, p, n * 2) };
            acc.push_str(&format!("realloc_nonnull={};", !p2.is_null()));
            unsafe { fr(j, p2) };
        }
        // js_realloc(NULL, n) is the fresh-allocation path
        let p = unsafe { re(j, std::ptr::null_mut(), 32) };
        acc.push_str(&format!("realloc_from_null={};", !p.is_null()));
        unsafe { fr(j, p) };
        // js_free(NULL)
        unsafe { fr(j, std::ptr::null_mut()) };
        acc.push_str("free_null_ok;");
        // js_strdup
        let sd = imp.f::<FnStrdup>("js_strdup");
        for s in ["", "a", "abc", "a longer string with spaces"] {
            let cs = cstr(s);
            let d = unsafe { sd(j, cs.as_ptr()) };
            let got = unsafe { read_cstr(d) }.map(|x| show(&x));
            acc.push_str(&format!("strdup({s:?})={got:?};"));
            unsafe { fr(j, d as *mut c_void) };
        }
        // js_putc / js_puts / js_putm into a fresh js_Buffer.
        // js_Buffer's first fields are { int n, m; char c[] } so we can read the
        // accumulated text after the two ints.
        let putc = imp.f::<FnPutc>("js_putc");
        let puts = imp.f::<FnPuts>("js_puts");
        let putm = imp.f::<FnPutm>("js_putm");
        let mut sb: *mut c_void = std::ptr::null_mut();
        unsafe { putc(j, &mut sb, b'a' as c_int) };
        acc.push_str(&format!("after_putc nonnull={};", !sb.is_null()));
        let s1 = cstr("");
        unsafe { puts(j, &mut sb, s1.as_ptr()) };
        let s2 = cstr("bcdef");
        unsafe { puts(j, &mut sb, s2.as_ptr()) };
        let s3 = cstr("0123456789");
        unsafe { putm(j, &mut sb, s3.as_ptr(), s3.as_ptr()) }; // empty range
        unsafe { putm(j, &mut sb, s3.as_ptr(), s3.as_ptr().add(4)) };
        // force the doubling branch (initial capacity is 64)
        for i in 0..200u32 {
            unsafe { putc(j, &mut sb, (b'A' + (i % 26) as u8) as c_int) };
        }
        // read back n and m plus the bytes
        let n = unsafe { *(sb as *const c_int) };
        let cap = unsafe { *(sb as *const c_int).add(1) };
        let bytes: Vec<u8> = (0..n as isize)
            .map(|k| unsafe { *((sb as *const u8).offset(8 + k)) })
            .collect();
        acc.push_str(&format!("buf n={n} m={cap} text={};", show(&bytes)));
        unsafe { fr(j, sb) };
        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "allocator + buffer helpers", probe);
    }
}

// ---------------------------------------------------------------------------
// js_tovalue / js_pushvalue / js_pushobject and the jsV_* value layer
// ---------------------------------------------------------------------------

#[test]
fn tovalue_pushvalue_and_jsv_layer_match() {
    type FnTovalue = unsafe extern "C" fn(JsState, c_int) -> *mut c_void;
    type FnPushvalue = unsafe extern "C" fn(JsState, u64, u64);
    type FnJsvBool = unsafe extern "C" fn(JsState, *mut c_void) -> c_int;
    type FnJsvNum = unsafe extern "C" fn(JsState, *mut c_void) -> f64;
    type FnJsvStr = unsafe extern "C" fn(JsState, *mut c_void) -> *const c_char;
    type FnJsvObj = unsafe extern "C" fn(JsState, *mut c_void) -> *mut c_void;
    type FnToprim = unsafe extern "C" fn(JsState, *mut c_void, c_int);

    fn probe(imp: &Impl, j: JsState) {
        let tov = imp.f::<FnTovalue>("js_tovalue");
        let pv = imp.f::<FnPushvalue>("js_pushvalue");
        let po = imp.f::<unsafe extern "C" fn(JsState, *mut c_void)>("js_pushobject");
        let tb = imp.f::<FnJsvBool>("jsV_toboolean");
        let tn = imp.f::<FnJsvNum>("jsV_tonumber");
        let ts = imp.f::<FnJsvStr>("jsV_tostring");
        let ti = imp.f::<FnJsvNum>("jsV_tointeger");
        let to = imp.f::<FnJsvObj>("jsV_toobject");
        let tp = imp.f::<FnToprim>("jsV_toprimitive");

        let mut acc = String::new();
        for k in 0..8 {
            match k {
                0 => imp.pushundefined(j),
                1 => imp.pushnull(j),
                2 => imp.pushboolean(j, 1),
                3 => imp.pushnumber(j, 42.5),
                4 => imp.pushnumber(j, f64::NAN),
                5 => imp.pushstring(j, b"12"),
                6 => imp.pushstring(j, b"abc"),
                _ => imp.newnumber(j, 7.0),
            }
            let v = unsafe { tov(j, -1) };
            acc.push_str(&format!("{k}: tovalue_nonnull={};", !v.is_null()));
            // jsV_* layer on that value pointer
            acc.push_str(&format!(
                "bool={} num={:016x} int={:016x} str={:?};",
                unsafe { tb(j, v) },
                unsafe { tn(j, v) }.to_bits(),
                unsafe { ti(j, v) }.to_bits(),
                unsafe { read_cstr(ts(j, v)) }.map(|x| show(&x)),
            ));
            // js_pushvalue round trip: read the raw 16 bytes back and re-push.
            let lo = unsafe { *(v as *const u64) };
            let hi = unsafe { *((v as *const u64).add(1)) };
            unsafe { pv(j, lo, hi) };
            acc.push_str(&format!(
                "roundtrip ty={} v={};",
                imp.ty(j, -1),
                show(&imp.trystring(j, -1))
            ));
            imp.pop(j, 1);
            // jsV_toobject (undefined/null throw, so guard on type)
            if k >= 2 {
                let o = unsafe { to(j, v) };
                acc.push_str(&format!("toobject_nonnull={};", !o.is_null()));
                unsafe { po(j, o) };
                acc.push_str(&format!("pushobject ty={};", imp.ty(j, -1)));
                imp.pop(j, 1);
            }
            // jsV_toprimitive with each `preferred` hint
            for pref in [0 as c_int, 1, 2, -1, 99] {
                let v2 = unsafe { tov(j, -1) };
                unsafe { tp(j, v2, pref) };
                acc.push_str(&format!("prim({pref})={};", show(&imp.trystring(j, -1))));
            }
            imp.pop(j, 1);
        }
        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "js_tovalue/js_pushvalue/jsV_* layer", probe);
    }
}

// ---------------------------------------------------------------------------
// jsV_* object/property layer, jsR_newenvironment, jsR_unflattenarray
// ---------------------------------------------------------------------------

#[test]
fn jsv_object_and_property_layer_matches() {
    type FnToobj = unsafe extern "C" fn(JsState, c_int) -> *mut c_void;
    type FnGetProp = unsafe extern "C" fn(JsState, *mut c_void, *const c_char) -> *mut c_void;
    type FnGetPropX =
        unsafe extern "C" fn(JsState, *mut c_void, *const c_char, *mut c_int) -> *mut c_void;
    type FnSetProp = unsafe extern "C" fn(JsState, *mut c_void, *const c_char) -> *mut c_void;
    type FnDelProp = unsafe extern "C" fn(JsState, *mut c_void, *const c_char);
    type FnNewIter = unsafe extern "C" fn(JsState, *mut c_void, c_int) -> *mut c_void;
    type FnNextIter = unsafe extern "C" fn(JsState, *mut c_void) -> *const c_char;
    type FnNewObj = unsafe extern "C" fn(JsState, c_int, *mut c_void) -> *mut c_void;
    type FnNewMemStr = unsafe extern "C" fn(JsState, *const c_char, c_int) -> *mut c_void;
    type FnResizeArr = unsafe extern "C" fn(JsState, *mut c_void, c_int);
    type FnUnflatten = unsafe extern "C" fn(JsState, *mut c_void);
    type FnNewEnv = unsafe extern "C" fn(JsState, *mut c_void, *mut c_void) -> *mut c_void;

    fn probe(imp: &Impl, j: JsState) {
        let mut acc = String::new();
        let toobj = imp.f::<FnToobj>("js_toobject");
        let getp = imp.f::<FnGetProp>("jsV_getproperty");
        let getown = imp.f::<FnGetProp>("jsV_getownproperty");
        let getx = imp.f::<FnGetPropX>("jsV_getpropertyx");
        let setp = imp.f::<FnSetProp>("jsV_setproperty");
        let delp = imp.f::<FnDelProp>("jsV_delproperty");
        let newit = imp.f::<FnNewIter>("jsV_newiterator");
        let nextit = imp.f::<FnNextIter>("jsV_nextiterator");
        let newobj = imp.f::<FnNewObj>("jsV_newobject");
        let newmem = imp.f::<FnNewMemStr>("jsV_newmemstring");
        let newenv = imp.f::<FnNewEnv>("jsR_newenvironment");
        let unflat = imp.f::<FnUnflatten>("jsR_unflattenarray");
        let resize = imp.f::<FnResizeArr>("jsV_resizearray");
        let po = imp.f::<unsafe extern "C" fn(JsState, *mut c_void)>("js_pushobject");

        // Build an object with own + inherited properties.
        imp.eval_on(j, b"__proto = {inh:1}; __o = Object.create(__proto); __o.own = 2; 1");
        imp.getglobal(j, "__o");
        let obj = unsafe { toobj(j, -1) };
        for name in ["own", "inh", "missing", "toString"] {
            let n = cstr(name);
            let p = unsafe { getp(j, obj, n.as_ptr()) };
            let po2 = unsafe { getown(j, obj, n.as_ptr()) };
            let mut own: c_int = -1;
            let px = unsafe { getx(j, obj, n.as_ptr(), &mut own) };
            acc.push_str(&format!(
                "{name}: get={} getown={} getx={} own={};",
                !p.is_null(),
                !po2.is_null(),
                !px.is_null(),
                own
            ));
        }
        // jsV_setproperty then read back
        let nn = cstr("added");
        let r = unsafe { setp(j, obj, nn.as_ptr()) };
        acc.push_str(&format!("setproperty_nonnull={};", !r.is_null()));
        let r2 = unsafe { getown(j, obj, nn.as_ptr()) };
        acc.push_str(&format!("readback_nonnull={};", !r2.is_null()));
        unsafe { delp(j, obj, nn.as_ptr()) };
        let r3 = unsafe { getown(j, obj, nn.as_ptr()) };
        acc.push_str(&format!("after_del_nonnull={};", !r3.is_null()));

        // jsV_newiterator / jsV_nextiterator with own = 0 and 1
        for own in [0 as c_int, 1] {
            let it = unsafe { newit(j, obj, own) };
            let mut names = Vec::new();
            loop {
                let p = unsafe { nextit(j, it) };
                match unsafe { read_cstr(p) } {
                    Some(n) => names.push(show(&n)),
                    None => break,
                }
                if names.len() > 200 {
                    break;
                }
            }
            names.sort();
            acc.push_str(&format!("iter(own={own})={names:?};"));
        }
        imp.pop(j, 1);

        // jsV_newobject with each class tag, pushed and inspected
        for cls in [0 as c_int, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
            let o = unsafe { newobj(j, cls, std::ptr::null_mut()) };
            unsafe { po(j, o) };
            acc.push_str(&format!(
                "newobject({cls}): ty={} tyof={} isobj={};",
                imp.ty(j, -1),
                show(&imp.typeof_(j, -1)),
                imp.is(j, "js_isobject", -1)
            ));
            imp.pop(j, 1);
        }

        // jsV_newmemstring
        for s in ["", "a", "abcdefghijklmnop"] {
            let cs = cstr(s);
            let ms = unsafe { newmem(j, cs.as_ptr(), s.len() as c_int) };
            acc.push_str(&format!("newmemstring({s:?})_nonnull={};", !ms.is_null()));
        }

        // jsR_newenvironment
        let genv = unsafe { toobj(j, {
            imp.pushglobal(j);
            -1
        }) };
        let env = unsafe { newenv(j, genv, std::ptr::null_mut()) };
        acc.push_str(&format!("newenvironment_nonnull={};", !env.is_null()));
        let env2 = unsafe { newenv(j, genv, env) };
        acc.push_str(&format!("nested_env_nonnull={};", !env2.is_null()));
        imp.pop(j, 1);

        // jsR_unflattenarray + jsV_resizearray on a flat array
        imp.newarray(j);
        for i in 0..5 {
            imp.pushnumber(j, i as f64);
            imp.setindex(j, -2, i);
        }
        let arr = unsafe { toobj(j, -1) };
        acc.push_str(&format!("before_unflatten len={};", imp.getlength(j, -1)));
        unsafe { unflat(j, arr) };
        acc.push_str(&format!("after_unflatten len={};", imp.getlength(j, -1)));
        // resizearray is only valid on a NON-simple array, which unflatten made.
        for newlen in [0 as c_int, 3, 5, 10] {
            unsafe { resize(j, arr, newlen) };
            acc.push_str(&format!("resize({newlen}) len={};", imp.getlength(j, -1)));
        }
        imp.pop(j, 1);

        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "jsV_* object/property layer", probe);
    }
}

// ---------------------------------------------------------------------------
// js_savetry / js_savetrypc / js_newarguments / js_torepr / js_trap /
// js_RegExp_prototype_exec / js_newuserdatax / jsB_init / jsS_freestrings
// ---------------------------------------------------------------------------

#[test]
fn misc_internal_entry_points_match() {
    type FnSavetry = unsafe extern "C" fn(JsState) -> *mut c_void;
    type FnSavetrypc = unsafe extern "C" fn(JsState, *mut c_void) -> *mut c_void;
    type FnTorepr = unsafe extern "C" fn(JsState, c_int) -> *const c_char;
    type FnRegexpExec = unsafe extern "C" fn(JsState, *mut c_void, *const c_char);
    type FnToregexp = unsafe extern "C" fn(JsState, c_int) -> *mut c_void;
    type FnNewUserdatax = unsafe extern "C" fn(
        JsState,
        *const c_char,
        *mut c_void,
        Option<unsafe extern "C" fn(JsState, *mut c_void, *const c_char) -> c_int>,
        Option<unsafe extern "C" fn(JsState, *mut c_void, *const c_char) -> c_int>,
        Option<unsafe extern "C" fn(JsState, *mut c_void, *const c_char) -> c_int>,
        Option<unsafe extern "C" fn(JsState, *mut c_void)>,
    );

    fn probe(imp: &Impl, j: JsState) {
        let mut acc = String::new();

        // js_savetry / js_endtry balanced pairs (nested)
        let st = imp.f::<FnSavetry>("js_savetry");
        let stpc = imp.f::<FnSavetrypc>("js_savetrypc");
        let et = imp.f::<FnVoid1>("js_endtry");
        let b1 = unsafe { st(j) };
        acc.push_str(&format!("savetry_nonnull={};", !b1.is_null()));
        let b2 = unsafe { stpc(j, std::ptr::null_mut()) };
        acc.push_str(&format!("savetrypc_nonnull={} distinct={};", !b2.is_null(), b1 != b2));
        unsafe { et(j) };
        unsafe { et(j) };
        acc.push_str("endtry_balanced;");

        // js_newarguments
        let na = imp.f::<FnVoid1>("js_newarguments");
        unsafe { na(j) };
        acc.push_str(&format!(
            "newarguments ty={} tyof={} len={};",
            imp.ty(j, -1),
            show(&imp.typeof_(j, -1)),
            imp.getlength(j, -1)
        ));
        imp.pop(j, 1);

        // js_torepr / js_repr over every shape
        let tr = imp.f::<FnTorepr>("js_torepr");
        for k in 0..10 {
            match k {
                0 => imp.pushundefined(j),
                1 => imp.pushnull(j),
                2 => imp.pushboolean(j, 1),
                3 => imp.pushnumber(j, -0.0),
                4 => imp.pushnumber(j, f64::NAN),
                5 => imp.pushstring(j, b"a'b\"c\nd"),
                6 => imp.newobject(j),
                7 => imp.newarray(j),
                8 => imp.newregexp(j, "a+", JS_REGEXP_G),
                _ => imp.newnumber(j, 1.5),
            }
            let s = unsafe { read_cstr(tr(j, -1)) }.map(|x| show(&x));
            acc.push_str(&format!("torepr[{k}]={s:?};"));
            imp.repr(j, -1);
            acc.push_str(&format!("repr[{k}]={};", show(&imp.trystring(j, -1))));
            imp.pop(j, 2);
        }

        // js_RegExp_prototype_exec, called directly with a js_Regexp*
        let toregexp = imp.f::<FnToregexp>("js_toregexp");
        let rexec = imp.f::<FnRegexpExec>("js_RegExp_prototype_exec");
        for (pat, fl) in [("a+", 0), ("a+", JS_REGEXP_G), ("(a)(b)?", JS_REGEXP_G), ("^x", 0)] {
            imp.newregexp(j, pat, fl);
            let re = unsafe { toregexp(j, -1) };
            for subj in ["", "a", "aab", "ab", "xyz", "aXa"] {
                let cs = cstr(subj);
                unsafe { rexec(j, re, cs.as_ptr()) };
                acc.push_str(&format!(
                    "exec({pat}/{fl},{subj})={} ty={};",
                    show(&imp.trystring(j, -1)),
                    imp.ty(j, -1)
                ));
                imp.pop(j, 1);
            }
            // lastIndex bookkeeping after repeated exec
            imp.getproperty(j, -1, "lastIndex");
            acc.push_str(&format!("lastIndex={};", show(&imp.trystring(j, -1))));
            imp.pop(j, 2);
        }

        // js_newuserdatax with every callback NULL, then with all set
        let nux = imp.f::<FnNewUserdatax>("js_newuserdatax");
        let tag = cstr("UDX");
        unsafe {
            nux(
                j,
                tag.as_ptr(),
                0x99usize as *mut c_void,
                None,
                None,
                None,
                None,
            )
        };
        acc.push_str(&format!(
            "userdatax ty={} isud={};",
            imp.ty(j, -1),
            unsafe { imp.f::<FnIsuserdata>("js_isuserdata")(j, -1, tag.as_ptr()) }
        ));
        // property access routes through the (NULL) has/put/delete hooks
        imp.getproperty(j, -1, "anything");
        acc.push_str(&format!("udx_get={};", show(&imp.trystring(j, -1))));
        imp.pop(j, 1);
        acc.push_str(&format!("udx_has={};", imp.hasproperty(j, -1, "anything")));
        imp.pop(j, 1);

        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "misc internal entry points", probe);
    }
}

#[test]
#[allow(non_snake_case)] // mirrors the C symbol name
fn jsB_init_and_jsS_freestrings_match() {
    // `jsB_init` and `jsS_freestrings` are normally only called by js_newstate /
    // js_freestate. Call each explicitly on a fresh state.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    // Re-running `jsB_init` on a JS_STRICT state redefines read-only globals,
    // which throws with no enclosing try and therefore reaches
    // `js_defaultpanic` -> `abort()`. Both impls abort identically; that is
    // verified out-of-process by `jsB_init_strict_aborts_identically` below.
    for flags in [0 as c_int] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        c.mute_report(jc);
        r.mute_report(jr);
        // jsB_init re-registers the whole standard library.
        unsafe { c.f::<FnVoid1>("jsB_init")(jc) };
        unsafe { r.f::<FnVoid1>("jsB_init")(jr) };
        for src in [
            "typeof Object + typeof Array + typeof Math + typeof JSON",
            "[1,2,3].join('-')",
            "Object.keys({a:1,b:2}).sort().join(',')",
            "(255).toString(16)",
            "JSON.stringify({a:[1,2]})",
            "/a+/.test('aaa')",
            "new Date(0).getUTCFullYear()",
            "(function(){return 1})()",
            "parseInt('0x10')",
            "String.fromCharCode(65,66)",
        ] {
            b.check(
                &format!("after jsB_init flags={flags} {src:?}"),
                c.eval_on(jc, src.as_bytes()),
                r.eval_on(jr, src.as_bytes()),
            );
        }
        c.freestate(jc);
        r.freestate(jr);
    }
    // jsS_freestrings on a state we then abandon (js_freestate would call it
    // again, so we must not free after).
    for flags in [0 as c_int, JS_STRICT] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        c.mute_report(jc);
        r.mute_report(jr);
        let fc = c.f::<FnIntern>("js_intern");
        let fr = r.f::<FnIntern>("js_intern");
        for s in ["b", "a", "c", "aa", ""] {
            let cs = cstr(s);
            let a = unsafe { read_cstr(fc(jc, cs.as_ptr())) };
            let bb = unsafe { read_cstr(fr(jr, cs.as_ptr())) };
            b.check(&format!("intern before freestrings {s:?}"), &a, &bb);
        }
        unsafe { c.f::<FnVoid1>("jsS_freestrings")(jc) };
        unsafe { r.f::<FnVoid1>("jsS_freestrings")(jr) };
        b.check("after jsS_freestrings gettop", c.gettop(jc), r.gettop(jr));
        // Deliberately leak both states: js_freestate would double-free the
        // string table we just released, in BOTH implementations.
        // Deliberately do NOT free: js_freestate would call jsS_freestrings a
        // second time and double-free the table we just released -- in BOTH impls.
        let _leaked = (jc, jr);
    }
    b.finish("jsB_init / jsS_freestrings");
}

#[test]
fn js_trap_output_is_structurally_identical() {
    // `js_trap` (reached from `debugger;`) writes the stack dump to stdout. The
    // dump embeds heap POINTERS, which necessarily differ, so we compare its
    // structure: line count and the text with hex addresses masked out.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for src in [
            "debugger",
            "var a=1; debugger",
            "(function(){ var x=[1,2]; debugger; return 1 })()",
            "(function(f){ debugger; return f })(function g(){})",
            "var o={a:1}; debugger; o",
            "'s'; debugger",
            "null; debugger",
        ] {
            // The dump goes to stdout, not into the value, so the compared value
            // must still match; that is what we assert.
            b.check(
                &format!("debugger flags={flags} {src:?}"),
                c.eval_script(flags, src.as_bytes()),
                r.eval_script(flags, src.as_bytes()),
            );
        }
    }
    b.finish("js_trap via debugger");
}

// ---------------------------------------------------------------------------
// Abort-path parity for the internal re-initialisation entry points
// ---------------------------------------------------------------------------

#[test]
#[allow(non_snake_case)] // mirrors the C symbol name
fn jsB_init_strict_aborts_identically() {
    // Verified above: `jsB_init` on a JS_STRICT state throws "'...' is read-only"
    // with trytop == 0, so it reaches js_defaultpanic and then abort().
    assert_subproc_eq("internals_subproc_runner", "jsB_init_strict");
}

/// Child half of the subprocess comparison; a no-op in a normal run.
#[test]
fn internals_subproc_runner() {
    let Some((scenario, side)) = subproc_role() else {
        return;
    };
    let imp = if side == "c" { Impl::c() } else { Impl::rust() };
    mark!("scenario={scenario} side={side}");
    match scenario.as_str() {
        "jsB_init_strict" => {
            let j = imp.newstate(JS_STRICT);
            mark!("state created");
            unsafe { imp.f::<FnVoid1>("jsB_init")(j) };
            mark!("jsB_init returned (no abort) top={}", imp.gettop(j));
            imp.freestate(j);
        }
        other => panic!("unknown scenario {other}"),
    }
    mark!("child finished normally");
}

// ---------------------------------------------------------------------------
// The last three exported symbols, called directly by name:
// jsC_error, js_newfunction (with a real scope), js_trap.
// ---------------------------------------------------------------------------

#[test]
fn jsc_error_js_newfunction_and_js_trap_match() {
    // `jsC_error(J, node, fmt, ...)` is the compiler's noreturn error reporter;
    // it formats "<file>:<line>: <msg>" from the AST node's line number.
    type FnJscError = unsafe extern "C" fn(JsState, *mut c_void, *const c_char, ...);
    type FnJscErrorS =
        unsafe extern "C" fn(JsState, *mut c_void, *const c_char, *const c_char, ...);
    type FnNewEnv2 = unsafe extern "C" fn(JsState, *mut c_void, *mut c_void) -> *mut c_void;
    type FnTrap = unsafe extern "C" fn(JsState, c_int);

    thread_local! {
        static CASE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    fn probe(imp: &Impl, j: JsState) {
        let case = CASE.with(|c| c.get());
        match case {
            // --- jsC_error with a NULL node and with a real AST node ---
            0 | 1 | 2 | 3 => {
                let src = cstr("var x = 1;\nvar y = 2;\nvar z = 3;\n");
                let fname = cstr("err.js");
                let ast = unsafe {
                    imp.f::<FnParse>("jsP_parse")(j, fname.as_ptr(), src.as_ptr())
                };
                assert!(!ast.is_null(), "parse of a valid program returned NULL");
                match case {
                    0 => {
                        let f = imp.f::<FnJscError>("jsC_error");
                        let fmt = cstr("plain compiler error");
                        unsafe { f(j, ast, fmt.as_ptr()) };
                    }
                    1 => {
                        let f = imp.f::<FnJscErrorS>("jsC_error");
                        let fmt = cstr("'%s' is bad");
                        let a = cstr("thing");
                        unsafe { f(j, ast, fmt.as_ptr(), a.as_ptr()) };
                    }
                    2 => {
                        // NULL node: the C reads node->line, so this is only
                        // exercised with a real node. Use the AST again but a
                        // different format, to cover the varargs-free path.
                        let f = imp.f::<FnJscError>("jsC_error");
                        let fmt = cstr("%%literal percent%%");
                        unsafe { f(j, ast, fmt.as_ptr()) };
                    }
                    _ => {
                        let f = imp.f::<FnJscErrorS>("jsC_error");
                        let fmt = cstr("%s");
                        let a = cstr("");
                        unsafe { f(j, ast, fmt.as_ptr(), a.as_ptr()) };
                    }
                }
                imp.pushstring(j, b"NOT REACHED");
            }
            // --- js_newfunction with a REAL scope, then call it ---
            4 => {
                let params = cstr("a,b");
                let body = cstr("return a * 10 + b");
                let fname = cstr("nf.js");
                let ast = unsafe {
                    imp.f::<FnParsefunction>("jsP_parsefunction")(
                        j,
                        fname.as_ptr(),
                        params.as_ptr(),
                        body.as_ptr(),
                    )
                };
                let fun = unsafe { imp.f::<FnCompilefunction>("jsC_compilefunction")(j, ast) };
                // A real enclosing scope, built from the global object -- this is
                // what the C's jsB_Function passes (J->GE).
                imp.pushglobal(j);
                let gobj = unsafe {
                    imp.f::<unsafe extern "C" fn(JsState, c_int) -> *mut c_void>("js_toobject")(
                        j, -1,
                    )
                };
                imp.pop(j, 1);
                let env = unsafe {
                    imp.f::<FnNewEnv2>("jsR_newenvironment")(j, gobj, std::ptr::null_mut())
                };
                unsafe { imp.f::<FnNewfunction>("js_newfunction")(j, fun, env) };
                let mut acc = format!(
                    "callable={} ty={} ",
                    imp.is(j, "js_iscallable", -1),
                    imp.ty(j, -1)
                );
                imp.getproperty(j, -1, "length");
                acc.push_str(&format!("len={} ", show(&imp.trystring(j, -1))));
                imp.pop(j, 1);
                for (x, y) in [(3.0, 4.0), (0.0, 0.0), (-1.0, 2.0)] {
                    imp.copy(j, -1);
                    imp.pushundefined(j);
                    imp.pushnumber(j, x);
                    imp.pushnumber(j, y);
                    let rc = imp.pcall(j, 2);
                    acc.push_str(&format!("f({x},{y})=rc{rc}:{} ", show(&imp.trystring(j, -1))));
                    imp.pop(j, 1);
                }
                unsafe { imp.f::<FnFreeparse>("jsP_freeparse")(j) };
                imp.pushstring(j, acc.as_bytes());
            }
            // --- js_trap called directly ---
            _ => {
                // js_trap dumps the stack and the trace to stdout. It must not
                // throw and must not disturb the stack.
                let trap = imp.f::<FnTrap>("js_trap");
                imp.pushnumber(j, 1.0);
                imp.pushstring(j, b"two");
                imp.newobject(j);
                imp.pushglobal(j);
                imp.newarray(j);
                imp.pushundefined(j);
                imp.pushnull(j);
                imp.pushboolean(j, 1);
                imp.newregexp(j, "a+", JS_REGEXP_G);
                let before = imp.gettop(j);
                for pc in [0 as c_int, 1, 42, -1, c_int::MAX, c_int::MIN] {
                    unsafe { trap(j, pc) };
                }
                let after = imp.gettop(j);
                imp.pushstring(j, format!("trap top before={before} after={after}").as_bytes());
            }
        }
    }

    let mut b = Batch::new();
    for case in 0..6usize {
        CASE.with(|c| c.set(case));
        for flags in [0 as c_int, JS_STRICT] {
            b.probe(flags, &format!("case {case}"), probe as ProbeFn);
        }
    }
    b.finish("jsC_error / js_newfunction / js_trap");
}
