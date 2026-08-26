//! Coverage-gap closers found by mutation testing the Rust translation.
//!
//! Each test in this file corresponds to a mutation of `src/*.rs` that SURVIVED
//! the rest of the suite, i.e. a code path with no differential coverage. The
//! module named in each test's comment is the one whose mutation now dies.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

/* ------------------------------------------------- jsboolean.rs */

/// jsboolean.rs `Bp_toString` / `Bp_valueOf`: the `u.boolean` read and the
/// JS_CBOOLEAN class check.
#[test]
fn t_boolean_prototype() {
    let mut srcs: Vec<String> = vec![];
    for v in [
        "true", "false", "new Boolean(true)", "new Boolean(false)", "new Boolean()",
        "new Boolean(0)", "new Boolean(1)", "new Boolean('')", "new Boolean('x')",
        "new Boolean(null)", "new Boolean(undefined)", "new Boolean(NaN)",
        "Boolean(true)", "Boolean(false)", "Boolean()", "Boolean(0)", "Boolean([])",
        "Boolean({})", "Boolean(-0)", "Boolean('false')",
    ] {
        srcs.push(format!("print(({v}).toString())"));
        srcs.push(format!("print(({v}).valueOf())"));
        srcs.push(format!("print(String({v}))"));
        srcs.push(format!("print(({v}) ? 'T' : 'F')"));
        srcs.push(format!("print(typeof ({v}))"));
        srcs.push(format!("dump({v})"));
        srcs.push(format!("print(JSON.stringify({v}))"));
        srcs.push(format!("print(({v}) + '')"));
        srcs.push(format!("print(+({v}))"));
        srcs.push(format!("print(Boolean.prototype.toString.call({v}))"));
        srcs.push(format!("print(Boolean.prototype.valueOf.call({v}))"));
    }
    // wrong `this` -> TypeError from the class check
    for v in ["1", "'x'", "{}", "[]", "null", "undefined", "new Number(1)"] {
        srcs.push(format!("print(Boolean.prototype.toString.call({v}))"));
        srcs.push(format!("print(Boolean.prototype.valueOf.call({v}))"));
    }
    srcs.push("print(Boolean.prototype.toString.call())".into());
    srcs.push("print(Boolean.prototype.valueOf.call())".into());
    for s in &srcs {
        diff_dostring(0, s);
        diff_dostring(JS_STRICT, s);
        diff_eval(0, s);
    }
}

/* ------------------------------------------------- jsbuiltin.rs */

/// jsbuiltin.rs `jsB_parseInt` sign handling, `jsB_parseFloat`, and the
/// URI encode/decode state machines.
#[test]
fn t_builtin_global_functions() {
    let mut srcs: Vec<String> = vec![];
    let ints = [
        "'-5'", "'+5'", "'5'", "'-0'", "'+0'", "'  -12  '", "'--5'", "'+-5'", "'-'",
        "'+'", "''", "'-0x10'", "'+0x10'", "'0x10'", "'-1e3'", "'-abc'", "'-12abc'",
        "'\\t-7'", "'\\n+8'", "'-Infinity'", "'-.5'", "'.5'", "'-2147483648'",
        "'2147483648'", "'-99999999999999999999'", "'-0b11'", "'-017'",
    ];
    for v in ints {
        srcs.push(format!("print(parseInt({v}))"));
        srcs.push(format!("print(parseFloat({v}))"));
        for r in [
            "undefined", "0", "1", "2", "8", "10", "16", "36", "37", "-1", "1.5", "'16'",
            "NaN", "Infinity", "null", "{}",
        ] {
            srcs.push(format!("print(parseInt({v}, {r}))"));
        }
    }
    // URI encode/decode state machines
    let uris = [
        "''", "'abc'", "'a b'", "'a+b'", "'%41'", "'%4'", "'%'", "'%zz'", "'%41%42'",
        "'%E4%B8%AD'", "'%C2'", "'%FF'", "'%00'", "'a%20b'", "'/?:@&=+$,#'",
        "'-_.!~*\\'()'", "'\\u00e9'", "'\\u4e2d'", "'\\ud800'", "'\\udfff'",
        "'\\ud800\\udc00'", "'%%41'", "'%41%'", "'\\u0000'",
    ];
    for v in uris {
        for f in [
            "encodeURI", "encodeURIComponent", "decodeURI", "decodeURIComponent",
            "escape", "unescape",
        ] {
            srcs.push(format!("print({f}({v}))"));
        }
    }
    // isNaN / isFinite / eval
    for v in ["1", "'1'", "NaN", "Infinity", "'abc'", "null", "undefined", "{}", "[]"] {
        srcs.push(format!("print(isNaN({v}), isFinite({v}))"));
    }
    let mut rng = Rng::new(0xB017);
    for _ in 0..1500 {
        let s = rng.ascii_string(10).replace('\'', "").replace('\\', "");
        srcs.push(format!("print(parseInt('{s}'), parseFloat('{s}'))"));
        srcs.push(format!(
            "print(parseInt('{s}', {}))",
            2 + rng.below(35)
        ));
        srcs.push(format!("print(encodeURIComponent('{s}'))"));
        srcs.push(format!("print(escape('{s}'))"));
    }
    for s in &srcs {
        diff_dostring(0, s);
    }
}

/* ------------------------------------------------- jscompile.rs */

/// jscompile.rs `checkfutureword` / `addlocal` / `emitlocal`: the strict-mode
/// rejection of `arguments` and `eval` as bindings, and every future reserved
/// word.
#[test]
fn t_compile_strict_bindings() {
    let names = [
        "arguments", "eval", "class", "const", "enum", "export", "extends", "import",
        "super", "implements", "interface", "let", "package", "private", "protected",
        "public", "static", "yield", "ok", "await", "null", "true", "false", "if",
        "var", "function", "this", "with", "argument", "arguments2", "evals", "eva",
    ];
    let forms = [
        "var {n} = 1",
        "var x, {n}",
        "function f({n}) {{}}",
        "function {n}() {{}}",
        "{n} = 1",
        "try {{}} catch({n}) {{}}",
        "try {{}} catch({n}) {{}} finally {{}}",
        "function f() {{ var {n} = 1; }}",
        "function f() {{ {n} = 1; }}",
        "for (var {n} in {{}}) {{}}",
        "for ({n} in {{}}) {{}}",
        "({{ {n}: 1 }})",
        "({{ get {n}() {{ return 1 }} }})",
        "x.{n}",
        "++{n}",
        "delete {n}",
        "typeof {n}",
        "new Function('{n}', 'return 1')",
        "new Function('return typeof {n}')",
        "function f() {{ return {n}; }} f()",
    ];
    for n in names {
        for f in forms {
            let src = f.replace("{n}", n);
            diff_dostring(0, &src);
            diff_dostring(JS_STRICT, &src);
            let strictsrc = format!("'use strict'; {src}");
            diff_dostring(0, &strictsrc);
            diff_eval(0, &src);
            diff_eval(JS_STRICT, &src);
        }
    }
    // `with` is rejected in strict mode; duplicate params/properties; octal
    for src in [
        "with({}){}",
        "'use strict'; with({}){}",
        "function f(a,a){}",
        "'use strict'; function f(a,a){}",
        "({a:1,a:2})",
        "'use strict'; ({a:1,a:2})",
        "({get a(){},get a(){}})",
        "({a:1,get a(){}})",
        "0123",
        "'use strict'; 0123",
        "'\\012'",
        "'use strict'; '\\012'",
        "switch(1){default:;default:;}",
        "break",
        "continue",
        "return 1",
        "lbl: break other",
        "lbl: continue lbl",
        "1 = 2",
        "1++",
        "(a,b) = 1",
        "delete 1",
        "'use strict'; delete x",
        "eval = 1",
        "'use strict'; eval = 1",
        "arguments = 1",
        "'use strict'; arguments = 1",
        "'use strict'; function f(){ return arguments.callee }; f()",
        "function f(){ return arguments.callee }; f()",
    ] {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
        diff_eval(0, src);
        diff_eval(JS_STRICT, src);
    }
}

/* ------------------------------------------------- jsfunction.rs */

/// jsfunction.rs `jsB_Function` (the `Function(...)` constructor joins its
/// leading arguments into a parameter list) plus apply/call/bind.
#[test]
fn t_function_constructor_and_methods() {
    let mut srcs: Vec<String> = vec![];
    // Function() with 0..6 args -- exercises the parameter-joining loop
    for args in [
        "",
        "'return 1'",
        "'a', 'return a'",
        "'a', 'b', 'return a+b'",
        "'a', 'b', 'c', 'return a+b+c'",
        "'a', 'b', 'c', 'd', 'return a+b+c+d'",
        "'a,b', 'return a+b'",
        "'a', 'b', ''",
        "'', 'return 1'",
        "'', '', 'return 1'",
        "'a'",
        "'a', 'b'",
        "'a=1', 'return a'",
        "'a', 'return arguments.length'",
    ] {
        srcs.push(format!("var f = new Function({args}); print(f.length, f(1,2,3,4))"));
        srcs.push(format!("var f = Function({args}); print(f.length, f(1,2))"));
        srcs.push(format!("print(new Function({args}).toString())"));
    }
    // invalid bodies / parameter lists
    for args in [
        "'return'", "'a b', 'return 1'", "'1', 'return 1'", "'a', '{'",
        "'a', 'return a'", "')', 'return 1'", "'a', 'syntax error ('",
        "null, 'return 1'", "1, 'return 1'", "'a', null",
    ] {
        srcs.push(format!("print(new Function({args}))"));
    }
    // apply / call / bind
    let ths = ["null", "undefined", "1", "'s'", "{x:9}", "[1,2]"];
    let argl = [
        "[]", "[1]", "[1,2]", "[1,2,3]", "null", "undefined", "'ab'", "{length:2}",
        "{length:0}", "{length:-1}", "{length:'2',0:'a',1:'b'}", "1", "arguments",
    ];
    for t in ths {
        for a in argl {
            srcs.push(format!(
                "function f(){{ return [this===undefined?'u':String(this), arguments.length, [].slice.call(arguments).join(',')].join('|') }} print(f.apply({t}, {a}))"
            ));
        }
        srcs.push(format!(
            "function f(a,b){{ return String(this)+':'+a+','+b }} print(f.call({t}, 1, 2))"
        ));
        srcs.push(format!(
            "function f(a,b){{ return String(this)+':'+a+','+b }} var g=f.bind({t}, 1); print(g(2), g.length)"
        ));
    }
    srcs.push("print(Function.prototype.apply.call(print, null, ['x']))".into());
    srcs.push("print(Function.prototype.call.call(print, null, 'y'))".into());
    srcs.push("print(Function.prototype.toString.call(print))".into());
    srcs.push("print(Function.prototype.toString.call(1))".into());
    srcs.push("print(Function.prototype())".into());
    srcs.push("print((function(a,b,c){}).length)".into());
    for s in &srcs {
        diff_dostring(0, s);
        diff_dostring(JS_STRICT, s);
    }
}

/* ------------------------------------------------- jsobject.rs */

/// jsobject.rs `Op_getOwnPropertyDescriptor` / the JS_CSTRING index path:
/// `k >= 0 && k < self->u.s.length`.
#[test]
fn t_string_object_indices() {
    let mut srcs: Vec<String> = vec![];
    for s in ["''", "'a'", "'ab'", "'abc'", "'\\u00e9'", "'\\u4e2d\\u6587'", "'\\u{1f600}'"] {
        for i in [
            "-2", "-1", "0", "1", "2", "3", "4", "5", "100", "'0'", "'1'", "'-0'",
            "'01'", "'1.0'", "'length'", "'x'", "0.5", "NaN", "1e21",
        ] {
            srcs.push(format!("var o = new String({s}); print(o[{i}])"));
            srcs.push(format!("var o = new String({s}); print({i} in o)"));
            srcs.push(format!(
                "var o = new String({s}); print(o.hasOwnProperty({i}))"
            ));
            srcs.push(format!(
                "var o = new String({s}); print(JSON.stringify(Object.getOwnPropertyDescriptor(o, String({i}))))"
            ));
            srcs.push(format!("print(({s})[{i}])"));
            srcs.push(format!(
                "var o = new String({s}); o[{i}] = 'Z'; print(o[{i}], o.length, String(o))"
            ));
            srcs.push(format!(
                "var o = new String({s}); print(delete o[{i}], o[{i}])"
            ));
        }
        srcs.push(format!(
            "var o = new String({s}); var ks=[]; for (var k in o) ks.push(k); print(ks.join(','))"
        ));
        srcs.push(format!(
            "print(JSON.stringify(Object.keys(new String({s}))))"
        ));
        srcs.push(format!(
            "print(JSON.stringify(Object.getOwnPropertyNames(new String({s}))))"
        ));
    }
    // Object.* over every receiver shape
    for recv in [
        "{}", "{a:1}", "{a:1,b:2}", "[]", "[1,2]", "new String('ab')",
        "new Number(1)", "new Boolean(true)", "function(){}", "new Date(0)",
        "/re/g", "null", "undefined", "1", "'s'", "true",
    ] {
        for m in [
            "Object.keys", "Object.getOwnPropertyNames", "Object.getPrototypeOf",
            "Object.isExtensible", "Object.isSealed", "Object.isFrozen",
            "Object.seal", "Object.freeze", "Object.preventExtensions",
        ] {
            srcs.push(format!("print(JSON.stringify({m}({recv})))"));
        }
        srcs.push(format!("print(Object.prototype.toString.call({recv}))"));
        srcs.push(format!("print(Object.prototype.valueOf.call({recv}))"));
        srcs.push(format!(
            "print(Object.prototype.hasOwnProperty.call({recv}, 'a'))"
        ));
        srcs.push(format!(
            "print(Object.prototype.propertyIsEnumerable.call({recv}, 'a'))"
        ));
        srcs.push(format!(
            "print(Object.prototype.isPrototypeOf.call({recv}, {{}}))"
        ));
        srcs.push(format!("print(JSON.stringify(Object({recv})))"));
    }
    for s in &srcs {
        diff_dostring(0, s);
        diff_dostring(JS_STRICT, s);
    }
}

/* ------------------------------------------------- jsregexp.rs */

/// jsregexp.rs `escaperegexp`: a literal `/` inside a RegExp pattern must be
/// escaped when building `.source`.
#[test]
fn t_regexp_source_escaping() {
    let p = libs();
    let pats = [
        "/", "//", "a/b", "/a", "a/", "///", "a/b/c", "\\/", "\\//", "[/]", "(/)",
        "a\\/b", "", "a", "[a/b]", "/*", "*/", "\\\\/", "/\\\\", "a//b",
    ];
    // through js_newregexp (the low level entry point).
    // js_newregexp THROWS a SyntaxError on an uncompilable pattern
    // (jsregexp.c:37), so it must run inside a protected call -- calling it with
    // trytop == 0 aborts the process.
    unsafe {
        for pat in pats {
            for flags in [0, 1, 2, 3, 4, 5, 6, 7, -1, 8, 255, i32::MAX, i32::MIN] {
                let mut ra = String::new();
                let mut rb = String::new();
                for (l, outv) in [(&p.c, &mut ra), (&p.rs, &mut rb)] {
                    set_cur(l);
                    let j = new_state(l, 0);
                    let cp = cstr(pat);
                    NEWRE_PAT.with(|c| c.set(cp.as_ptr()));
                    NEWRE_FLAGS.with(|c| c.set(flags));
                    out_clear();
                    l.js_newcfunction(
                        j,
                        Some(newregexp_probe),
                        b"mk\0".as_ptr() as *const c_char,
                        0,
                    );
                    l.js_pushundefined(j);
                    let mkrc = l.js_pcall(j, 0);
                    let mkmsg = from_c(l.js_tryrepr(j, -1, ERRSTR));
                    let mut body = String::new();
                    if mkrc == 0 {
                        l.js_setglobal(j, b"re\0".as_ptr() as *const c_char);
                        let src = cstr(
                            "print(re.source, re.global, re.ignoreCase, re.multiline, \
                             re.lastIndex, String(re), re.toString(), re.exec('a/b'))",
                        );
                        let rc = l.js_dostring(j, src.as_ptr());
                        body = format!("rc={rc}");
                    } else {
                        l.js_pop(j, 1);
                    }
                    *outv = format!("mkrc={mkrc} mkmsg={mkmsg} {body} {}", out_take());
                    l.js_freestate(j);
                }
                assert_eq!(ra, rb, "js_newregexp source escaping pat={pat:?} flags={flags}");
            }
        }
    }
    // and through JS: new RegExp(...)
    for pat in pats {
        for f in ["undefined", "''", "'g'", "'i'", "'m'", "'gim'", "'x'", "'gg'"] {
            for src in [
                format!("var r = new RegExp('{}', {f}); print(r.source, String(r))",
                        pat.replace('\\', "\\\\")),
                format!("var r = new RegExp('{}', {f}); print(new RegExp(r).source)",
                        pat.replace('\\', "\\\\")),
                format!("var r = new RegExp('{}', {f}); print(r.exec('a/b'))",
                        pat.replace('\\', "\\\\")),
            ] {
                diff_dostring(0, &src);
            }
        }
    }
}

thread_local! {
    static NEWRE_PAT: std::cell::Cell<*const c_char> =
        const { std::cell::Cell::new(std::ptr::null()) };
    static NEWRE_FLAGS: std::cell::Cell<c_int> = const { std::cell::Cell::new(0) };
}

/// Calls `js_newregexp` inside a protected call so a bad pattern's SyntaxError
/// is caught by `js_pcall` instead of aborting.
unsafe extern "C" fn newregexp_probe(j: JS) {
    let l = cur();
    l.js_newregexp(j, NEWRE_PAT.with(|c| c.get()), NEWRE_FLAGS.with(|c| c.get()));
}

/* ------------------------------------------------- jsvalue.rs */

/// jsvalue.rs `js_strtol`: the `base == 10` fast path vs the table path. Only
/// sweeping EVERY radix 2..36 distinguishes them (radix 11 is the first radix
/// where the two differ on input like "1a").
#[test]
fn t_strtol_all_radices() {
    let p = libs();
    let mut inputs: Vec<String> = vec![
        "".into(), "0".into(), "1".into(), "9".into(), "a".into(), "A".into(),
        "z".into(), "Z".into(), "1a".into(), "1A".into(), "19".into(), "1z".into(),
        "ab".into(), "zz".into(), "10".into(), "11".into(), "0z".into(), "9a".into(),
        "aa".into(), "ff".into(), "FF".into(), "7f".into(), "gg".into(), "1g".into(),
        " 1".into(), "-1".into(), "+1".into(), "1 ".into(), "1.5".into(), "1e3".into(),
        "0x10".into(), ":".into(), "/".into(), "@".into(), "`".into(), "{".into(),
        "[".into(), "1:".into(), "1/".into(), "1@".into(), "1`".into(),
        "z".repeat(20), "9".repeat(20), "1".repeat(40),
        "0123456789abcdefghijklmnopqrstuvwxyz".into(),
        "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".into(),
    ];
    let mut rng = Rng::new(0x5701);
    for _ in 0..2000 {
        let n = 1 + rng.below(12) as usize;
        let s: String = (0..n)
            .map(|_| {
                let t = rng.below(4);
                match t {
                    0 => (b'0' + rng.below(10) as u8) as char,
                    1 => (b'a' + rng.below(26) as u8) as char,
                    2 => (b'A' + rng.below(26) as u8) as char,
                    _ => (0x21u8 + rng.below(0x5e) as u8) as char,
                }
            })
            .collect();
        inputs.push(s);
    }
    unsafe {
        for s in &inputs {
            let cs = cstr(s);
            // every well-defined base (see ll_num.rs: base > 80 is C UB)
            for radix in (-2..=80).chain([i32::MIN, -1000]) {
                let mut ea: *mut c_char = std::ptr::null_mut();
                let mut eb: *mut c_char = std::ptr::null_mut();
                let a = p.c.js_strtol(cs.as_ptr(), &mut ea, radix);
                let b = p.rs.js_strtol(cs.as_ptr(), &mut eb, radix);
                assert_eq!(
                    a.to_bits(),
                    b.to_bits(),
                    "js_strtol({s:?}, radix={radix}) value: C={a} RUST={b}"
                );
                assert_eq!(
                    ea.offset_from(cs.as_ptr() as *mut c_char),
                    eb.offset_from(cs.as_ptr() as *mut c_char),
                    "js_strtol({s:?}, radix={radix}) endptr"
                );
            }
        }
    }
    // and via Number.prototype.toString / parseInt over every radix
    for r in 2..=36 {
        for v in [
            "0", "1", "-1", "255", "-255", "1.5", "-1.5", "0.1", "1e21", "1e-7",
            "NaN", "Infinity", "-Infinity", "2147483647", "-2147483648", "4294967295",
            "1e300", "5e-324", "-0",
        ] {
            diff_dostring(0, &format!("print(({v}).toString({r}))"));
            diff_dostring(0, &format!("print(parseInt(({v}).toString({r}), {r}))"));
        }
    }
    for r in [-1, 0, 1, 37, 38, 100, 1000] {
        diff_dostring(0, &format!("print((255).toString({r}))"));
    }
}

/* ------------------------------------------------- jsrun.rs / jsi.rs limits */

/// jsi.rs `JS_ARRAYLIMIT` (1<<26): `RangeError "array too large"` from both
/// jsR_setproperty("length") and jsR_setarrayindex.
#[test]
fn t_array_limit() {
    let lim: i64 = 1 << 26;
    let mut srcs: Vec<String> = vec![];
    for n in [
        0i64, 1, 2, 8, 1000, lim - 2, lim - 1, lim, lim + 1, lim + 2, lim * 2,
        (1i64 << 31) - 1, 1i64 << 31, 4294967295, 4294967296,
    ] {
        // length assignment
        srcs.push(format!("var a = []; a.length = {n}; print(a.length)"));
        srcs.push(format!("var a = [1,2,3]; a.length = {n}; print(a.length)"));
        // Array(n) constructor
        srcs.push(format!("print(new Array({n}).length)"));
        srcs.push(format!("print(Array({n}).length)"));
        // sparse index write (goes through jsR_setarrayindex / unflatten)
        srcs.push(format!("var a = []; a[{n}] = 1; print(a.length, a[{n}])"));
        srcs.push(format!("var a = [1]; a[{n}] = 1; print(a.length)"));
    }
    // invalid array lengths
    for n in [
        "-1", "-2", "1.5", "0.5", "-0.5", "NaN", "Infinity", "-Infinity", "'abc'",
        "'10'", "'-1'", "null", "undefined", "{}", "[]", "[5]", "true", "false",
        "-0", "1e21", "4294967296.5",
    ] {
        srcs.push(format!("var a = []; a.length = {n}; print(a.length)"));
        srcs.push(format!("print(new Array({n}).length)"));
    }
    for s in &srcs {
        diff_dostring(0, s);
        diff_dostring(JS_STRICT, s);
    }
}

/// jsrun.rs `js_pushlstring` / `js_pushstring` `JS_STRLIMIT` (1<<28) check, and
/// the JS_TSHRSTR (n <= 15 inline) vs JS_TMEMSTR representation boundary.
#[test]
fn t_pushlstring_representation_and_limit() {
    let p = libs();
    unsafe {
        let mut rng = Rng::new(0x5715);
        // the shrstr/memstr boundary is soffsetof(js_Value, t.type) == 15
        let mut cases: Vec<Vec<u8>> = vec![];
        for n in 0..40usize {
            cases.push(vec![b'x'; n]);
            cases.push((0..n).map(|i| b'a' + (i % 26) as u8).collect());
        }
        // embedded NUL bytes: js_pushlstring keys off `n`, not strlen
        cases.push(b"ab\0cd".to_vec());
        cases.push(b"\0".to_vec());
        cases.push(b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec());
        cases.push(b"abcdefghijklmno\0extra".to_vec());
        for _ in 0..400 {
            cases.push(rng.raw_bytes(40));
        }
        for bytes in &cases {
            for n in [
                0i32,
                1,
                bytes.len() as i32,
                (bytes.len() as i32).min(15),
                (bytes.len() as i32).min(16),
            ] {
                if n < 0 || n as usize > bytes.len() {
                    continue;
                }
                let mut ra = String::new();
                let mut rb = String::new();
                for (l, outv) in [(&p.c, &mut ra), (&p.rs, &mut rb)] {
                    set_cur(l);
                    let j = new_state(l, 0);
                    l.js_pushlstring(j, bytes.as_ptr() as *const c_char, n);
                    let t = l.js_type(j, -1);
                    let tn = from_c(l.js_typeof(j, -1));
                    let s = from_c(l.js_tostring(j, -1));
                    let r = from_c(l.js_tryrepr(j, -1, ERRSTR));
                    let top = l.js_gettop(j);
                    // and round-trip through a property name
                    l.js_newobject(j);
                    l.js_copy(j, -2);
                    let key = cstr("k");
                    l.js_setproperty(j, -2, key.as_ptr());
                    l.js_getproperty(j, -1, key.as_ptr());
                    let back = from_c(l.js_tostring(j, -1));
                    *outv = format!("t={t} tn={tn} s={s:?} r={r} top={top} back={back:?}");
                    l.js_freestate(j);
                }
                assert_eq!(
                    ra, rb,
                    "js_pushlstring({:02x?}, n={n}) representation",
                    bytes
                );
            }
        }
        // JS_STRLIMIT rejection: n > 1<<28 -> RangeError "invalid string length".
        // The check happens BEFORE `v` is read, so a short buffer is safe.
        let small = cstr("x");
        for n in [
            (1i32 << 28) + 1,
            (1i32 << 28) + 2,
            i32::MAX,
            i32::MAX - 1,
            1 << 29,
            1 << 30,
        ] {
            let mut ra = String::new();
            let mut rb = String::new();
            for (l, outv) in [(&p.c, &mut ra), (&p.rs, &mut rb)] {
                set_cur(l);
                let j = new_state(l, 0);
                // must be inside a protected call, js_pushlstring throws
                PUSH_N.with(|c| c.set(n));
                PUSH_PTR.with(|c| c.set(small.as_ptr()));
                l.js_newcfunction(
                    j,
                    Some(pushlstring_probe),
                    b"probe\0".as_ptr() as *const c_char,
                    0,
                );
                l.js_pushundefined(j);
                let rc = l.js_pcall(j, 0);
                let msg = from_c(l.js_tryrepr(j, -1, ERRSTR));
                *outv = format!("rc={rc} msg={msg}");
                l.js_freestate(j);
            }
            assert_eq!(ra, rb, "js_pushlstring STRLIMIT n={n}");
        }
        // NOTE: negative `n` is UNDEFINED BEHAVIOUR in the C and therefore not
        // differentially testable. `js_pushlstring` (jsrun.c:163) does
        // `if (n > JS_STRLIMIT) ...` (false for n < 0), then
        // `if (n <= soffsetof(js_Value, t.type))` (true), then
        // `while (n--) *s++ = *v++;`. With n = -1 the post-decrement yields -1
        // (truthy), so the loop runs ~2^64 times writing past the 16-byte inline
        // shrstr buffer and straight off the value stack: an immediate SIGSEGV in
        // BOTH libraries. Only n >= 0 is exercised above.
    }
}

thread_local! {
    static PUSH_N: std::cell::Cell<c_int> = const { std::cell::Cell::new(0) };
    static PUSH_PTR: std::cell::Cell<*const c_char> =
        const { std::cell::Cell::new(std::ptr::null()) };
}

unsafe extern "C" fn pushlstring_probe(j: JS) {
    let l = cur();
    l.js_pushlstring(j, PUSH_PTR.with(|c| c.get()), PUSH_N.with(|c| c.get()));
}

/// Long strings that cross the JS_TMEMSTR threshold and get concatenated,
/// interned, used as property names and compared.
#[test]
fn t_long_string_paths() {
    let mut rng = Rng::new(0x1077);
    for _ in 0..200 {
        let n = rng.below(200) as usize;
        let s: String = (0..n).map(|_| (b'a' + rng.below(26) as u8) as char).collect();
        let src = format!(
            "var s = '{s}'; var o = {{}}; o[s] = 1; \
             print(s.length, s === '{s}', o[s], JSON.stringify(o).length, \
                   (s+s).length, s.charCodeAt(0), s.slice(1,-1).length)"
        );
        diff_dostring(0, &src);
    }
    for n in [0usize, 1, 14, 15, 16, 17, 31, 32, 33, 63, 64, 255, 256, 1000, 5000] {
        let s = "z".repeat(n);
        let src = format!(
            "var s = '{s}'; print(s.length, typeof s, s===''+s, ({{}})[s]===undefined, \
             JSON.stringify(s).length)"
        );
        diff_dostring(0, &src);
    }
}

/* ------------------------------------------------- js_trap (stdout dump) */

/// jsrun.rs `js_trap` writes the stack and environment to stdout via printf.
/// Compare by redirecting fd 1 to a temp file around each call.
#[test]
fn t_js_trap_stdout() {
    let p = libs();
    unsafe {
        let mut outs: Vec<String> = vec![];
        for l in [&p.c, &p.rs] {
            let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
            let path = format!("{dir}/trap_{}.txt", l.name);
            let text = capture_stdout(&path, || {
                set_cur(l);
                let j = new_state(l, 0);
                let src = cstr(
                    "var a = 1; var o = {x:1}; function f(){ debugger; return 1 } f(); \
                     debugger;",
                );
                l.js_dostring(j, src.as_ptr());
                l.js_freestate(j);
            });
            outs.push(text);
        }
        // The dump prints raw heap addresses (`[Function 0x7f..., f, ...]`), which
        // differ between the two libraries by construction. Normalise them.
        assert_eq!(
            norm_dump(&outs[0]),
            norm_dump(&outs[1]),
            "js_trap stdout dump"
        );
        assert!(
            outs[0].contains("stack"),
            "js_trap produced no dump: {:?}",
            outs[0]
        );
    }
}

/// Normalise a captured stdout dump for comparison:
///  * `0x<hex>` -> `0xPTR` (raw heap addresses can never match between two
///    independently loaded libraries), and
///  * drop libtest's own progress lines. libtest writes `test <name> ... ok`
///    straight to fd 1 from other threads, so while we have fd 1 redirected
///    those lines land in our capture file.
fn norm_dump(s: &str) -> String {
    s.lines()
        .filter(|l| {
            !(l.starts_with("test ") || l.starts_with("running ") || l.is_empty()
              || l.starts_with("failures") || l.starts_with("warning:"))
        })
        .map(|l| norm_ptrs(l) + "\n")
        .collect()
}

/// Replace every `0x<hex>` run with `0xPTR`.
fn norm_ptrs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '0' && i + 1 < b.len() && (b[i + 1] == 'x' || b[i + 1] == 'X') {
            let mut j = i + 2;
            while j < b.len() && b[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j > i + 2 {
                out.push_str("0xPTR");
                i = j;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// Serialises fd-1 redirection: `capture_stdout` rebinds the process-wide
/// stdout descriptor, so two tests doing it concurrently would steal each
/// other's output (libtest runs tests in parallel threads of ONE process).
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` with fd 1 redirected into `path`, then return the captured bytes.
fn capture_stdout<F: FnOnce()>(path: &str, f: F) -> String {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use std::os::unix::io::AsRawFd;
    extern "C" {
        fn dup(fd: c_int) -> c_int;
        fn dup2(old: c_int, new: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
        fn fflush(f: *mut std::ffi::c_void) -> c_int;
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

/* ------------------------------------------------- jsS_dumpstrings */

/// jsintern.rs `jsS_dumpstrings` also writes to stdout.
#[test]
fn t_dumpstrings_stdout() {
    let p = libs();
    unsafe {
        let mut outs: Vec<String> = vec![];
        for l in [&p.c, &p.rs] {
            let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
            let path = format!("{dir}/dumpstr_{}.txt", l.name);
            let text = capture_stdout(&path, || {
                set_cur(l);
                let j = new_state(l, 0);
                let src = cstr(
                    "var o = {}; for (var i = 0; i < 40; ++i) o['key'+i] = i; \
                     var s = 'interned'; o[s] = 1;",
                );
                l.js_dostring(j, src.as_ptr());
                l.jsS_dumpstrings(j);
                l.js_freestate(j);
            });
            outs.push(text);
        }
        assert_eq!(
            norm_dump(&outs[0]),
            norm_dump(&outs[1]),
            "jsS_dumpstrings stdout dump"
        );
        assert!(!outs[0].is_empty(), "jsS_dumpstrings produced nothing");
    }
}

/* ------------------------------------------------- misc exported symbols */

/// The remaining exported symbols that no other test drives directly:
/// jsB_propf / jsB_propn / jsB_props, jsV_newmemstring, jsR_newenvironment,
/// js_newarguments, js_toobject/js_toprimitive/js_tovalue/js_pushvalue/
/// js_pushobject, js_putc/js_puts/js_putm, jsV_getownproperty and friends.
#[test]
fn t_misc_exports_via_js() {
    // jsB_propf/propn/props are used by every jsB_init*; observing the shape of
    // the built-in objects exercises them.
    for src in [
        "var n=[]; for (var k in Math) n.push(k); print(n.sort().join(','))",
        "print(Object.getOwnPropertyNames(Math).sort().join(','))",
        "print(Object.getOwnPropertyNames(JSON).sort().join(','))",
        "print(Object.getOwnPropertyNames(String.prototype).sort().join(','))",
        "print(Object.getOwnPropertyNames(Array.prototype).sort().join(','))",
        "print(Object.getOwnPropertyNames(Number.prototype).sort().join(','))",
        "print(Object.getOwnPropertyNames(Date.prototype).sort().join(','))",
        "print(Object.getOwnPropertyNames(RegExp.prototype).sort().join(','))",
        "print(Object.getOwnPropertyNames(Object.prototype).sort().join(','))",
        "print(Object.getOwnPropertyNames(Function.prototype).sort().join(','))",
        "print(Object.getOwnPropertyNames(Error.prototype).sort().join(','))",
        "print(Object.getOwnPropertyNames(this).sort().join(','))",
        "print(Math.PI, Math.E, Math.LN2, Math.LN10, Math.LOG2E, Math.LOG10E, \
         Math.SQRT1_2, Math.SQRT2)",
        "print(Number.MAX_VALUE, Number.MIN_VALUE, Number.NaN, \
         Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY)",
        "print(Math.abs.length, Math.max.length, Math.min.length, Math.pow.length)",
        "print(String.prototype.replace.length, Array.prototype.slice.length)",
        // js_newarguments
        "function f(){ return arguments } var a=f(1,2,3); \
         print(a.length, a[0], a[1], a[2], a[3], Object.prototype.toString.call(a))",
        "function f(){ arguments[0]=9; return arguments[0] } print(f(1))",
        "function f(a){ arguments[0]=9; return a } print(f(1))",
        "function f(){ return arguments.length } print(f(), f(1), f(1,2))",
        "function f(){ var n=[]; for (var k in arguments) n.push(k); return n.join(',') } \
         print(f(1,2,3))",
        // jsR_newenvironment / closures
        "function o(){ var x=1; return function(){ return ++x } } var g=o(); \
         print(g(), g(), g())",
        "var fs=[]; for (var i=0;i<3;++i) fs.push(function(){return i}); \
         print(fs[0](), fs[1](), fs[2]())",
        "function o(){ var x=1; function i(){ x++ } i(); return x } print(o())",
        "with ({a:1}) { print(a) }",
        "var a = 5; with ({a:1}) { print(a) } print(a)",
        "with ({a:1}) { a = 2; print(a) }",
        // js_toprimitive hint paths
        "print(({valueOf:function(){return 1},toString:function(){return 's'}}) + '')",
        "print(+({valueOf:function(){return 1},toString:function(){return 's'}}))",
        "print(String({valueOf:function(){return 1},toString:function(){return 's'}}))",
        "print(({toString:null}) + '')",
        "print(({valueOf:null, toString:null}) + '')",
        "print([] + [], [] + {}, ({}) + [], 1 + {})",
        "print(({}) == '[object Object]')",
        "var d=new Date(0); print(d + '' === d.toString())",
        // js_putc/js_puts/js_putm buffers (used by JSON.stringify + js_repr)
        "print(JSON.stringify({a:'\\u0000\\u001f\"\\\\\\n\\t\\r\\b\\f'}))",
        "print(JSON.stringify('\\u00e9\\u4e2d\\u{1f600}'))",
        "print(JSON.stringify({a:[1,[2,[3,[4]]]]}, null, 4))",
    ] {
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
        diff_eval(0, src);
    }
}

/// Direct FFI drive of jsV_newmemstring + js_tovalue + js_pushvalue +
/// js_pushobject + js_toobject + js_toprimitive.
#[test]
fn t_value_level_exports() {
    let p = libs();
    unsafe {
        let mut rng = Rng::new(0x5A1E);
        let mut cases: Vec<Vec<u8>> = vec![b"".to_vec(), b"a".to_vec(), b"hello world".to_vec()];
        for _ in 0..300 {
            cases.push(rng.raw_bytes(30));
        }
        for bytes in &cases {
            let mut ra = String::new();
            let mut rb = String::new();
            for (l, outv) in [(&p.c, &mut ra), (&p.rs, &mut rb)] {
                set_cur(l);
                let j = new_state(l, 0);
                // js_pushlstring -> js_tovalue -> js_pushvalue round trip
                l.js_pushlstring(j, bytes.as_ptr() as *const c_char, bytes.len() as c_int);
                let v = sym_tovalue(l, j, -1);
                sym_pushvalue(l, j, v);
                let same = from_c(l.js_tostring(j, -1)) == from_c(l.js_tostring(j, -2));
                // js_toobject on the string, then js_pushobject it back
                let o = sym_toobject(l, j, -1);
                sym_pushobject(l, j, o);
                let cls = from_c(l.js_tostring(j, -1));
                let isobj = l.pred("js_isobject", j, -1);
                let isstrobj = l.pred("js_isstringobject", j, -1);
                // js_toprimitive with each hint
                let mut hints = String::new();
                for hint in [0, 1, 2, -1, 99] {
                    l.js_copy(j, -1);
                    sym_toprimitive(l, j, -1, hint);
                    hints.push_str(&format!(
                        "{}:{} ",
                        hint,
                        from_c(l.js_tryrepr(j, -1, ERRSTR))
                    ));
                    l.js_pop(j, 1);
                }
                *outv = format!(
                    "same={same} cls={cls:?} isobj={isobj} isstrobj={isstrobj} \
                     hints={hints} top={}",
                    l.js_gettop(j)
                );
                l.js_freestate(j);
            }
            assert_eq!(ra, rb, "value-level exports for {:02x?}", bytes);
        }
    }
}

// tiny local shims for the pointer-returning value APIs
unsafe fn sym_tovalue(l: &Lib, j: JS, idx: c_int) -> *mut std::ffi::c_void {
    l.raw2::<unsafe extern "C" fn(JS, c_int) -> *mut std::ffi::c_void>("js_tovalue")(j, idx)
}
unsafe fn sym_pushvalue(l: &Lib, j: JS, v: *mut std::ffi::c_void) {
    // js_pushvalue takes js_Value BY VALUE (16 bytes). Passing it through a
    // pointer would be wrong, so copy the 16 bytes into a repr(C) struct.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct V16 {
        a: u64,
        b: u64,
    }
    let val = *(v as *const V16);
    l.raw2::<unsafe extern "C" fn(JS, V16)>("js_pushvalue")(j, val)
}
unsafe fn sym_toobject(l: &Lib, j: JS, idx: c_int) -> *mut std::ffi::c_void {
    l.raw2::<unsafe extern "C" fn(JS, c_int) -> *mut std::ffi::c_void>("js_toobject")(j, idx)
}
unsafe fn sym_pushobject(l: &Lib, j: JS, o: *mut std::ffi::c_void) {
    l.raw2::<unsafe extern "C" fn(JS, *mut std::ffi::c_void)>("js_pushobject")(j, o)
}
unsafe fn sym_toprimitive(l: &Lib, j: JS, idx: c_int, hint: c_int) {
    l.raw2::<unsafe extern "C" fn(JS, c_int, c_int)>("js_toprimitive")(j, idx, hint)
}

/* ------------------------------------------------- jsrun.rs JS_STRLIMIT */

/// jsrun.rs `js_pushstring` (jsrun.c:148) and `js_pushlstring` (jsrun.c:165)
/// both reject `n > JS_STRLIMIT` (1<<28) with `RangeError "invalid string
/// length"`. Pinning down the EXACT threshold requires driving lengths on both
/// sides of it, and the check happens before `v` is read only on the rejecting
/// side -- an accepted length really copies `n` bytes. So this test needs a real
/// buffer of ~128 MiB. It is the only way to distinguish the true limit from,
/// say, JS_STRLIMIT/2.
#[test]
fn t_strlimit_boundary() {
    let p = libs();
    const LIM: usize = 1 << 28;
    // Just above half the limit: accepted by the real code, rejected by any
    // implementation whose threshold is JS_STRLIMIT/2.
    let n = (LIM / 2) + 1;
    let mut buf: Vec<u8> = vec![b'x'; n + 1];
    buf[n] = 0;
    unsafe {
        for &len in &[n as c_int, (n - 1) as c_int] {
            let mut ra = String::new();
            let mut rb = String::new();
            for (l, outv) in [(&p.c, &mut ra), (&p.rs, &mut rb)] {
                set_cur(l);
                let j = new_state(l, 0);
                PUSH_N.with(|c| c.set(len));
                PUSH_PTR.with(|c| c.set(buf.as_ptr() as *const c_char));
                l.js_newcfunction(
                    j,
                    Some(pushlstring_probe),
                    b"probe\0".as_ptr() as *const c_char,
                    0,
                );
                l.js_pushundefined(j);
                let rc = l.js_pcall(j, 0);
                let mut desc = format!("rc={rc}");
                if rc == 0 {
                    // measure without materialising a Rust copy of 128 MiB
                    let s = l.js_tostring(j, -1);
                    let strlen = if s.is_null() {
                        usize::MAX
                    } else {
                        CStrLen(s)
                    };
                    let t = l.js_type(j, -1);
                    desc.push_str(&format!(" strlen={strlen} type={t}"));
                } else {
                    desc.push_str(&format!(" msg={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
                }
                *outv = desc;
                l.js_pop(j, 1);
                l.js_freestate(j);
            }
            assert_eq!(ra, rb, "js_pushlstring at n={len} (JS_STRLIMIT={LIM})");
        }
        // js_pushstring takes strlen(v); same length, same expectation.
        let mut ra = String::new();
        let mut rb = String::new();
        for (l, outv) in [(&p.c, &mut ra), (&p.rs, &mut rb)] {
            set_cur(l);
            let j = new_state(l, 0);
            PUSH_PTR.with(|c| c.set(buf.as_ptr() as *const c_char));
            l.js_newcfunction(
                j,
                Some(pushstring_probe),
                b"probe\0".as_ptr() as *const c_char,
                0,
            );
            l.js_pushundefined(j);
            let rc = l.js_pcall(j, 0);
            let mut desc = format!("rc={rc}");
            if rc == 0 {
                let s = l.js_tostring(j, -1);
                desc.push_str(&format!(" strlen={}", CStrLen(s)));
            } else {
                desc.push_str(&format!(" msg={}", from_c(l.js_tryrepr(j, -1, ERRSTR))));
            }
            *outv = desc;
            l.js_pop(j, 1);
            l.js_freestate(j);
        }
        assert_eq!(ra, rb, "js_pushstring at strlen={n}");
    }
}

unsafe extern "C" fn pushstring_probe(j: JS) {
    let l = cur();
    l.js_pushstring(j, PUSH_PTR.with(|c| c.get()));
}

#[allow(non_snake_case)]
unsafe fn CStrLen(s: *const c_char) -> usize {
    extern "C" {
        fn strlen(s: *const c_char) -> usize;
    }
    strlen(s)
}
