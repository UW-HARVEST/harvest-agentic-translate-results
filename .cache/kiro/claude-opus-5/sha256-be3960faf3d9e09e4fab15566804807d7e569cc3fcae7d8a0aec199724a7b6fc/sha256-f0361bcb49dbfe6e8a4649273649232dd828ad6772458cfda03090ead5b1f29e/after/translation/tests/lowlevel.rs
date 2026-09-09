//! CONFIGS.md rows 60, 62-73, 84 — the lowest-level exported entry points:
//! builtin table installers, the lexer, the parser, the compiler, the raw
//! `jsV_*` value/property API, the interning table, the allocator wrappers,
//! the GC and the stdout-writing helpers.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

/* ------------------------------------------------------------------ */
/* row 60 — jsB_init* and jsB_propf / jsB_propn / jsB_props            */
/* ------------------------------------------------------------------ */
unsafe extern "C-unwind" fn cf_id(J: JS) {
    let api = active_api();
    unsafe { (api.js_pushnumber)(J, (api.js_gettop)(J) as f64) }
}

fn task_builtins(api: &Api, J: JS) {
    unsafe {
        // Every builtin table is already installed by js_newstate; re-running an
        // installer must be idempotent-or-identical on both sides.
        let inits: [(&str, unsafe extern "C-unwind" fn(JS)); 13] = [
            ("jsB_initobject", api.jsB_initobject),
            ("jsB_initarray", api.jsB_initarray),
            ("jsB_initfunction", api.jsB_initfunction),
            ("jsB_initboolean", api.jsB_initboolean),
            ("jsB_initnumber", api.jsB_initnumber),
            ("jsB_initstring", api.jsB_initstring),
            ("jsB_initregexp", api.jsB_initregexp),
            ("jsB_initerror", api.jsB_initerror),
            ("jsB_initmath", api.jsB_initmath),
            ("jsB_initjson", api.jsB_initjson),
            ("jsB_initdate", api.jsB_initdate),
            ("jsB_init", api.jsB_init),
            ("jsB_init(again)", api.jsB_init),
        ];
        for (name, f) in inits {
            f(J);
            emit(format!("{name} installed; top={}", (api.js_gettop)(J)));
            eval(
                api,
                J,
                name,
                "[typeof Object,typeof Array,typeof Function,typeof Boolean,typeof Number,\
                 typeof String,typeof RegExp,typeof Error,typeof Math,typeof JSON,typeof Date,\
                 typeof parseInt,typeof escape].join(',')",
            );
        }
        // jsB_propf / jsB_propn / jsB_props operate on the object at -1
        (api.js_newobject)(J);
        (api.jsB_propf)(J, sstr("myfun"), cf_id, 2);
        (api.jsB_propn)(J, cs("mynum").as_ptr(), 1.5);
        (api.jsB_props)(J, sstr("mystr"), sstr("hello"));
        (api.jsB_propn)(J, cs("nan").as_ptr(), f64::NAN);
        (api.jsB_props)(J, sstr("empty"), sstr(""));
        (api.jsB_propf)(J, sstr("zero"), cf_id, 0);
        for k in ["myfun", "mynum", "mystr", "nan", "empty", "zero"] {
            (api.js_getproperty)(J, -1, cs(k).as_ptr());
            emit(format!("prop {k}: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
        }
        (api.js_pushiterator)(J, -1, 1);
        loop {
            match opt_str((api.js_nextiterator)(J, -1)) {
                None => break,
                Some(b) => emit(format!("  enum: {:?}", String::from_utf8_lossy(&b))),
            }
        }
        (api.js_pop)(J, 1);
        (api.js_setglobal)(J, cs("bag").as_ptr());
        eval(api, J, "bag", "bag.myfun(1,2,3)+','+bag.myfun.length+','+bag.myfun.name");
        eval(api, J, "bag keys", "Object.keys(bag).sort().join()");
    }
}

#[test]
fn row60_builtin_installers() {
    diff_all_flags("builtin installers", task_builtins);
}

/* ------------------------------------------------------------------ */
/* row 62 — the lexer, driven directly                                 */
/* ------------------------------------------------------------------ */
fn lex_corpus() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "".into(),
        " ".into(),
        "\n".into(),
        "\r\n".into(),
        "\u{2028}\u{2029}".into(),
        "\u{feff}x".into(),
        "// line comment".into(),
        "/* block */".into(),
        "/* unterminated".into(),
        "/**/x".into(),
        "a".into(),
        "abc def".into(),
        "$_a1".into(),
        "\\u0041".into(),
        "\u{e9}".into(),
        "0".into(),
        "0.5".into(),
        ".5".into(),
        "5.".into(),
        "1e10".into(),
        "1e+10".into(),
        "1e-10".into(),
        "1e".into(),
        "0x1f".into(),
        "0X1F".into(),
        "0x".into(),
        "08".into(),
        "017".into(),
        "0b1".into(),
        "1_000".into(),
        "1abc".into(),
        "\"\"".into(),
        "\"a\"".into(),
        "'a'".into(),
        "\"unterminated".into(),
        "'unterminated".into(),
        "\"a\nb\"".into(),
        "\"\\n\\t\\r\\b\\f\\v\\0\\\\\\\"\\'\"".into(),
        "\"\\x41\"".into(),
        "\"\\x4\"".into(),
        "\"\\xZZ\"".into(),
        "\"\\u0041\"".into(),
        "\"\\u00\"".into(),
        "\"\\uZZZZ\"".into(),
        "\"\\q\"".into(),
        "\"\\\n\"".into(),
        "\"\\8\"".into(),
        "\"\\1\"".into(),
        "+ - * / % ++ -- << >> >>> < > <= >= == != === !== & | ^ ~ ! && || ? : = += -= *= /= %= \
         <<= >>= >>>= &= |= ^= , ; ( ) [ ] { } .".into(),
        "@".into(),
        "#".into(),
        "`".into(),
        "\\".into(),
        "\u{0}".into(),
        "break case catch continue debugger default delete do else false finally for function if \
         in instanceof new null return switch this throw true try typeof var void while with"
            .into(),
        "class const enum export extends import super implements interface let package private \
         protected public static yield"
            .into(),
        "a/b".into(),
        "/re/".into(),
        "/re/gim".into(),
        "/[/]/".into(),
        "/\\//".into(),
        "/unterminated".into(),
        "/re\n/".into(),
        "x = /a/g".into(),
        "1 / 2 / 3".into(),
        "function f(){ return /a/ }".into(),
        "var x = 1; if (x) { x++ } else { --x }".into(),
    ];
    let mut rng = Rng::new(0xC0DE_0062);
    const FRAG: &[&str] = &[
        "a", "1", "0x", ".", "'", "\"", "/", "//", "/*", "*/", "\\", "\n", " ", "+", "-", "=",
        "==", "===", ">>>", "{", "}", "(", ")", "[", "]", ";", ",", "e", "e+", "0.5", "\\u0041",
        "\u{e9}", "\u{20ac}", "if", "var", "function", "return", "in", "/a/g", "?", ":", "!", "~",
        "@", "#", "\u{0}",
    ];
    for _ in 0..4000 {
        let n = rng.below(9) as usize;
        v.push((0..n).map(|_| *rng.pick(FRAG)).collect::<Vec<_>>().join(""));
    }
    v
}

fn task_lexer(api: &Api, J: JS) {
    unsafe {
        let f = cs("lex.js");
        for src in lex_corpus() {
            let c = match std::ffi::CString::new(src.clone()) {
                Ok(c) => c,
                Err(_) => continue,
            };
            // jsY_lex throws a SyntaxError on bad input, so run each source in
            // its own protected sub-call via js_ploadstring's machinery: here we
            // instead let the outer pcall catch it, so lex one source per
            // ploadstring-protected pass.
            let base = (api.js_gettop)(J);
            (api.jsY_initlex)(J, f.as_ptr(), c.as_ptr());
            let mut toks = Vec::new();
            for _ in 0..4096 {
                let t = (api.jsY_lex)(J);
                toks.push(format!("{t}:{}", s((api.jsY_tokenstring)(J_tok(t))).replace('\n', "\\n")));
                if t == 0 {
                    break;
                }
            }
            emit(format!("lex {src:?} -> {}", toks.join(" ")));
            restore_top(api, J, base);
        }
    }
}

// jsY_tokenstring takes the token as its only argument.
fn J_tok(t: c_int) -> c_int {
    t
}

#[test]
fn row62_lexer() {
    // Each source is lexed inside its own protected call so a SyntaxError from
    // one input does not abandon the rest of the corpus.
    diff_all_flags("lexer (whole corpus, first error stops)", task_lexer);
    diff_all_flags("lexer (per-source protected)", task_lexer_isolated);
}

unsafe extern "C-unwind" fn lex_one(J: JS) {
    let api = active_api();
    unsafe {
        let f = cs("lex.js");
        let src = (api.js_tostring)(J, 1);
        (api.jsY_initlex)(J, f.as_ptr(), src);
        let mut toks = Vec::new();
        for _ in 0..4096 {
            let t = (api.jsY_lex)(J);
            toks.push(format!("{t}", ));
            if t == 0 {
                break;
            }
        }
        emit(format!("  tokens: {}", toks.join(" ")));
        (api.js_pushnumber)(J, toks.len() as f64);
    }
}

unsafe extern "C-unwind" fn lexjson_one(J: JS) {
    let api = active_api();
    unsafe {
        let f = cs("lex.json");
        let src = (api.js_tostring)(J, 1);
        (api.jsY_initlex)(J, f.as_ptr(), src);
        let mut toks = Vec::new();
        for _ in 0..4096 {
            let t = (api.jsY_lexjson)(J);
            toks.push(format!("{t}"));
            if t == 0 {
                break;
            }
        }
        emit(format!("  json tokens: {}", toks.join(" ")));
        (api.js_pushnumber)(J, toks.len() as f64);
    }
}

fn lex_isolated(api: &Api, J: JS, cb: unsafe extern "C-unwind" fn(JS), corpus: Vec<String>) {
    unsafe {
        for src in corpus {
            let c = match std::ffi::CString::new(src.clone()) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let base = (api.js_gettop)(J);
            (api.js_newcfunction)(J, cb, sstr("lex"), 1);
            (api.js_pushnull)(J);
            (api.js_pushstring)(J, c.as_ptr());
            let rc = (api.js_pcall)(J, 1);
            let sentinel = cs("<throw>");
            emit(format!(
                "lex {src:?} rc={rc} -> {:?}",
                s((api.js_trystring)(J, -1, sentinel.as_ptr()))
            ));
            restore_top(api, J, base);
        }
    }
}

fn task_lexer_isolated(api: &Api, J: JS) {
    lex_isolated(api, J, lex_one, lex_corpus())
}

/* ------------------------------------------------------------------ */
/* row 63 — jsY_lexjson                                                */
/* ------------------------------------------------------------------ */
fn json_corpus() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "".into(), " ".into(), "null".into(), "true".into(), "false".into(), "0".into(),
        "-0".into(), "1".into(), "-1".into(), "1.5".into(), "1e5".into(), "1e+5".into(),
        "1e-5".into(), "01".into(), "+1".into(), ".5".into(), "5.".into(), "1.".into(),
        "\"\"".into(), "\"a\"".into(), "\"\\n\"".into(), "\"\\u0041\"".into(),
        "\"\\uD83D\\uDE00\"".into(), "\"\\x41\"".into(), "\"\\q\"".into(), "\"unterminated".into(),
        "'single'".into(), "[]".into(), "{}".into(), "[1,2]".into(), "{\"a\":1}".into(),
        "[1,]".into(), "{,}".into(), "{a:1}".into(), "nul".into(), "NaN".into(),
        "Infinity".into(), "undefined".into(), "[".into(), "]".into(), "{".into(), "}".into(),
        ":".into(), ",".into(), "@".into(), "\t\n\r ".into(), "\"a\tb\"".into(),
        "\"\\\u{0}\"".into(), "1 2 3".into(),
    ];
    let mut rng = Rng::new(0xC0DE_0063);
    const FRAG: &[&str] = &[
        "[", "]", "{", "}", ":", ",", "1", "0", "-", "+", ".", "e", "\"", "\"a\"", "null", "true",
        "false", " ", "\n", "\\", "\\n", "\\u0041", "\\u00", "'", "@", "NaN",
    ];
    for _ in 0..4000 {
        let n = rng.below(9) as usize;
        v.push((0..n).map(|_| *rng.pick(FRAG)).collect::<Vec<_>>().join(""));
    }
    v
}

fn task_lexjson(api: &Api, J: JS) {
    lex_isolated(api, J, lexjson_one, json_corpus())
}

#[test]
fn row63_json_lexer() {
    diff_all_flags("json lexer", task_lexjson);
}

/* char-class helpers (part of row 62) */
#[test]
fn row62b_lexer_char_classes() {
    let p = both();
    for c in -300i32..0x11_0000 {
        unsafe {
            assert_eq!((p.c.jsY_iswhite)(c), (p.r.jsY_iswhite)(c), "jsY_iswhite({c})");
            assert_eq!(
                (p.c.jsY_isnewline)(c),
                (p.r.jsY_isnewline)(c),
                "jsY_isnewline({c})"
            );
            assert_eq!((p.c.jsY_ishex)(c), (p.r.jsY_ishex)(c), "jsY_ishex({c})");
            assert_eq!((p.c.jsY_tohex)(c), (p.r.jsY_tohex)(c), "jsY_tohex({c})");
        }
    }
    // jsY_tokenstring over the whole token range, including out-of-range ints
    for t in -10i32..400 {
        unsafe {
            assert_eq!(
                s((p.c.jsY_tokenstring)(t)),
                s((p.r.jsY_tokenstring)(t)),
                "jsY_tokenstring({t})"
            );
        }
    }
    // jsY_findword: binary search over a sorted word list
    let words: Vec<&str> = vec!["alpha", "beta", "delta", "gamma", "omega", "zeta"];
    let cwords: Vec<std::ffi::CString> = words.iter().map(|w| cs(w)).collect();
    let ptrs: Vec<*const c_char> = cwords.iter().map(|c| c.as_ptr()).collect();
    let probes = [
        "alpha", "beta", "delta", "gamma", "omega", "zeta", "aardvark", "zzz", "", "alph",
        "alphab", "epsilon", "Beta", "ALPHA",
    ];
    for probe in probes {
        let pc = cs(probe);
        for n in [0usize, 1, 3, 6] {
            unsafe {
                assert_eq!(
                    (p.c.jsY_findword)(pc.as_ptr(), ptrs.as_ptr(), n as c_int),
                    (p.r.jsY_findword)(pc.as_ptr(), ptrs.as_ptr(), n as c_int),
                    "jsY_findword({probe:?}, n={n})"
                );
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* rows 64-66 — parser, compiler, function/script/env construction      */
/* ------------------------------------------------------------------ */
unsafe extern "C-unwind" fn parse_compile_one(J: JS) {
    let api = active_api();
    unsafe {
        let f = cs("p.js");
        let src = (api.js_tostring)(J, 1);
        let strict = (api.js_tryinteger)(J, 2, 0);
        let ast = (api.jsP_parse)(J, f.as_ptr(), src);
        emit(format!("  parse -> {}", if ast.is_null() { "NULL" } else { "ast" }));
        if !ast.is_null() {
            let fun = (api.jsC_compilescript)(J, ast, strict);
            emit(format!(
                "  compilescript(strict={strict}) -> {}",
                if fun.is_null() { "NULL" } else { "fn" }
            ));
            (api.jsP_freeparse)(J);
            if !fun.is_null() {
                // js_newscript wraps the compiled function; then call it
                (api.js_newscript)(J, fun, std::ptr::null_mut());
                (api.js_pushundefined)(J);
                (api.js_call)(J, 0);
                let sentinel = cs("<throw>");
                emit(format!(
                    "  run -> {:?}",
                    s((api.js_tryrepr)(J, -1, sentinel.as_ptr()))
                ));
                return;
            }
        } else {
            (api.jsP_freeparse)(J);
        }
        (api.js_pushundefined)(J);
    }
}

unsafe extern "C-unwind" fn parse_function_one(J: JS) {
    let api = active_api();
    unsafe {
        let f = cs("pf.js");
        let params = (api.js_tostring)(J, 1);
        let body = (api.js_tostring)(J, 2);
        let ast = (api.jsP_parsefunction)(J, f.as_ptr(), params, body);
        emit(format!(
            "  parsefunction -> {}",
            if ast.is_null() { "NULL" } else { "ast" }
        ));
        if ast.is_null() {
            (api.jsP_freeparse)(J);
            (api.js_pushundefined)(J);
            return;
        }
        let fun = (api.jsC_compilefunction)(J, ast);
        emit(format!(
            "  compilefunction -> {}",
            if fun.is_null() { "NULL" } else { "fn" }
        ));
        (api.jsP_freeparse)(J);
        if fun.is_null() {
            (api.js_pushundefined)(J);
            return;
        }
        // build an environment for it and call it
        (api.js_newobject)(J);
        let vars = (api.js_toobject)(J, -1);
        (api.js_pop)(J, 1);
        let env = (api.jsR_newenvironment)(J, vars, std::ptr::null_mut());
        (api.js_newfunction)(J, fun, env);
        (api.js_pushundefined)(J);
        (api.js_pushnumber)(J, 21.0);
        (api.js_pushnumber)(J, 2.0);
        (api.js_call)(J, 2);
        let sentinel = cs("<throw>");
        emit(format!(
            "  call -> {:?}",
            s((api.js_tryrepr)(J, -1, sentinel.as_ptr()))
        ));
    }
}

fn program_corpus() -> Vec<String> {
    let mut v: Vec<String> = js_snippets().iter().map(|s| s.to_string()).collect();
    v.extend(
        [
            "", " ", ";", "{}", "{;}", "(", ")", "var", "var x", "var x=", "function", "function(){}",
            "function f(){}", "function f(a,a){}", "function f(eval){}", "function f(arguments){}",
            "return", "break", "continue", "if", "if(1)", "if(1);else;", "for(;;)break;",
            "for(var i in {});", "for(1 in {});", "while(0);", "do;while(0)", "with({});",
            "try{}catch(e){}", "try{}finally{}", "try{}", "switch(1){}", "switch(1){default:}",
            "switch(1){case 1:case 1:}", "label:;", "label:break label;", "break nolabel;",
            "a:b:;", "delete x", "typeof x", "void 0", "1++", "++1", "x=y=z=1",
            "0.1.toString()", "1..toString()", "({a:1,a:2})", "({get a(){},set a(v){}})",
            "({get a(){},get a(){}})", "[,,]", "[1,,2]", "new new Date()", "a?b:c",
            "'use strict'; x=1", "'use strict'; delete x", "'use strict'; with({});",
            "'use strict'; var eval", "'use strict'; function f(a,a){}", "'use strict'; 017",
            "'use strict'; arguments=1", "'use strict'; eval=1",
            "function f(){'use strict'; return this}", "(function(){return typeof this})()",
            "class C{}", "let x=1", "const y=1", "x=>x", "`t`", "x**2", "a?.b", "a??b",
            "/*", "*/", "\"", "'", "\\", "@", "#",
            "function f(){return arguments}", "eval('1')", "(0,eval)('1')",
            "var a=1;function a(){}", "function a(){};var a=1;",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    // deeply nested expressions (JS_ASTLIMIT = 400)
    for d in [1usize, 10, 100, 398, 399, 400, 401, 500, 1000] {
        v.push(format!("{}1{}", "(".repeat(d), ")".repeat(d)));
        v.push(format!("{}1{}", "-".repeat(d), ""));
        v.push(format!("{}{}", "[".repeat(d), "]".repeat(d)));
        v.push(format!("{}x{}", "{a:".repeat(d), "}".repeat(d)));
        v.push(format!("{}1", "1+".repeat(d)));
    }
    let mut rng = Rng::new(0xC0DE_0064);
    const FRAG: &[&str] = &[
        "var ", "x", "y", "1", "+", "-", "*", "/", "=", "==", "(", ")", "{", "}", "[", "]", ";",
        ",", "if", "else", "for", "while", "function", "return", "'s'", "\"d\"", "/re/", ".", "?",
        ":", "new ", "typeof ", "delete ", "in", "instanceof", "this", "null", "true", "&&", "||",
        "!", "~", "++", "--", "try", "catch", "finally", "throw", "with", "switch", "case",
        "default", "break", "continue", "\n",
    ];
    for _ in 0..3000 {
        let n = rng.below(10) as usize;
        v.push((0..n).map(|_| *rng.pick(FRAG)).collect::<Vec<_>>().join(""));
    }
    v
}

fn task_parse_compile(api: &Api, J: JS) {
    unsafe {
        for strict in [0, 1] {
            for src in program_corpus() {
                let c = match std::ffi::CString::new(src.clone()) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let base = (api.js_gettop)(J);
                (api.js_newcfunction)(J, parse_compile_one, sstr("pc"), 2);
                (api.js_pushnull)(J);
                (api.js_pushstring)(J, c.as_ptr());
                (api.js_pushnumber)(J, strict as f64);
                let rc = (api.js_pcall)(J, 2);
                let sentinel = cs("<throw>");
                emit(format!(
                    "parse+compile strict={strict} {src:?} rc={rc} -> {:?}",
                    s((api.js_trystring)(J, -1, sentinel.as_ptr()))
                ));
                restore_top(api, J, base);
            }
        }
    }
}

#[test]
fn row64_65_parser_and_compiler() {
    diff_all_flags("parser + compiler", task_parse_compile);
}

fn task_parse_function(api: &Api, J: JS) {
    unsafe {
        let cases: [(&str, &str); 22] = [
            ("", "return 1"),
            ("a", "return a"),
            ("a,b", "return a*b"),
            ("a, b , c", "return a+b+c"),
            ("", ""),
            ("", "return arguments.length"),
            ("", "return this"),
            ("a", "'use strict'; return a"),
            ("a,a", "return a"),
            ("eval", "return 1"),
            ("arguments", "return 1"),
            ("a=1", "return a"),
            ("...a", "return a"),
            ("(", "return 1"),
            ("a", "return"),
            ("a", "}"),
            ("a", "{"),
            ("a", "return a; garbage garbage"),
            ("a", "function inner(){return a} return inner()"),
            ("a", "var a = 2; return a"),
            ("a", "throw new Error('x')"),
            ("a", "return a * 2"),
        ];
        for (params, body) in cases {
            let base = (api.js_gettop)(J);
            (api.js_newcfunction)(J, parse_function_one, sstr("pf"), 2);
            (api.js_pushnull)(J);
            (api.js_pushstring)(J, cs(params).as_ptr());
            (api.js_pushstring)(J, cs(body).as_ptr());
            let rc = (api.js_pcall)(J, 2);
            let sentinel = cs("<throw>");
            emit(format!(
                "parsefunction({params:?},{body:?}) rc={rc} -> {:?}",
                s((api.js_trystring)(J, -1, sentinel.as_ptr()))
            ));
            restore_top(api, J, base);
        }
    }
}

#[test]
fn row64b_66_parsefunction_and_env() {
    diff_all_flags("jsP_parsefunction + jsR_newenvironment", task_parse_function);
}

/* js_newarguments (row 66) */
unsafe extern "C-unwind" fn cf_arguments(J: JS) {
    let api = active_api();
    unsafe {
        (api.js_newarguments)(J);
        emit(format!("newarguments: {}", dump_slot(api, J, -1)));
        (api.js_getproperty)(J, -1, cs("length").as_ptr());
        emit(format!("  length -> {}", dump_slot(api, J, -1)));
        (api.js_pop)(J, 1);
        for i in 0..4 {
            (api.js_getindex)(J, -1, i);
            emit(format!("  [{i}] -> {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
        }
    }
}

fn task_newarguments(api: &Api, J: JS) {
    unsafe {
        for argc in 0..4 {
            let base = (api.js_gettop)(J);
            (api.js_newcfunction)(J, cf_arguments, sstr("args"), 0);
            (api.js_pushnull)(J);
            for i in 0..argc {
                (api.js_pushnumber)(J, i as f64);
            }
            let rc = (api.js_pcall)(J, argc);
            emit(format!("newarguments argc={argc} rc={rc}"));
            restore_top(api, J, base);
        }
        eval(api, J, "arguments", "(function(){return arguments.length+','+arguments[0]})(7,8)");
        eval(api, J, "arguments callee", "(function(){return typeof arguments.callee})()");
        eval(api, J, "arguments write", "(function(a){arguments[0]=9; return a})(1)");
    }
}

#[test]
fn row66b_newarguments() {
    diff_all_flags("js_newarguments", task_newarguments);
}

/* ------------------------------------------------------------------ */
/* rows 67-68 — the raw jsV_* object/property/conversion API            */
/* ------------------------------------------------------------------ */
fn task_jsv_api(api: &Api, J: JS) {
    unsafe {
        // jsV_newobject only zeroes the common header; the per-class union must
        // be filled in by that class' own constructor. Passing a class whose
        // union holds pointers (JS_CSTRING, JS_CARRAY, JS_CREGEXP, JS_CFUNCTION,
        // JS_C*FUNCTION, JS_CUSERDATA, JS_CITERATOR, JS_CDATE) and then touching
        // a property dereferences uninitialised memory in the *C* original, so
        // only the classes with no union payload are exercised here. The other
        // classes are covered through their real constructors below.
        for class in [
            0,  /* JS_COBJECT */
            5,  /* JS_CERROR */
            11, /* JS_CMATH */
            12, /* JS_CJSON */
        ] {
            (api.js_newobject)(J);
            let proto = (api.js_toobject)(J, -1);
            (api.js_pop)(J, 1);
            let o = (api.jsV_newobject)(J, class, proto);
            (api.js_pushobject)(J, o);
            emit(format!("jsV_newobject(class={class}): {}", dump_slot(api, J, -1)));
            // property get/set/del through the low-level API
            for name in ["a", "", "0", "length", "toString"] {
                let n = cs(name);
                let p = (api.jsV_setproperty)(J, o, n.as_ptr());
                emit(format!(
                    "  setproperty {name:?} -> {}",
                    if p.is_null() { "NULL" } else { "prop" }
                ));
                let p = (api.jsV_getownproperty)(J, o, n.as_ptr());
                emit(format!(
                    "  getownproperty {name:?} -> {}",
                    if p.is_null() { "NULL" } else { "prop" }
                ));
                let p = (api.jsV_getproperty)(J, o, n.as_ptr());
                emit(format!(
                    "  getproperty {name:?} -> {}",
                    if p.is_null() { "NULL" } else { "prop" }
                ));
                let mut own: c_int = -1;
                let p = (api.jsV_getpropertyx)(J, o, n.as_ptr(), &mut own);
                emit(format!(
                    "  getpropertyx {name:?} -> {} own={own}",
                    if p.is_null() { "NULL" } else { "prop" }
                ));
                (api.jsV_delproperty)(J, o, n.as_ptr());
                let p = (api.jsV_getownproperty)(J, o, n.as_ptr());
                emit(format!(
                    "  after del {name:?} -> {}",
                    if p.is_null() { "NULL" } else { "prop" }
                ));
            }
            (api.js_pop)(J, 1);
        }
        // every properly-constructed object class, through the real constructors
        let ctors: [(&str, fn(&Api, JS)); 9] = [
            ("object", |a, j| unsafe { (a.js_newobject)(j) }),
            ("array", |a, j| unsafe { (a.js_newarray)(j) }),
            ("boolean", |a, j| unsafe { (a.js_newboolean)(j, 1) }),
            ("number", |a, j| unsafe { (a.js_newnumber)(j, 2.5) }),
            ("string", |a, j| unsafe {
                (a.js_newstring)(j, cs("h\u{e9}llo").as_ptr())
            }),
            ("regexp", |a, j| unsafe {
                (a.js_newregexp)(j, cs("a+").as_ptr(), JS_REGEXP_G)
            }),
            ("error", |a, j| unsafe { (a.js_newerror)(j, cs("m").as_ptr()) }),
            ("cfunction", |a, j| unsafe {
                (a.js_newcfunction)(j, cf_id, sstr("cf"), 1)
            }),
            ("userdata", |a, j| unsafe {
                (a.js_newuserdata)(j, sstr("T"), std::ptr::null_mut(), None)
            }),
        ];
        for (name, f) in ctors {
            f(api, J);
            let o = (api.js_toobject)(J, -1);
            emit(format!("class {name}: {}", dump_slot(api, J, -1)));
            for pname in ["a", "", "0", "1", "length", "toString", "source", "message"] {
                let n = cs(pname);
                let p = (api.jsV_getownproperty)(J, o, n.as_ptr());
                let mut own: c_int = -1;
                let px = (api.jsV_getpropertyx)(J, o, n.as_ptr(), &mut own);
                let pg = (api.jsV_getproperty)(J, o, n.as_ptr());
                emit(format!(
                    "  {pname:?}: own={} x={}/own={own} get={}",
                    !p.is_null(),
                    !px.is_null(),
                    !pg.is_null()
                ));
                let ps = (api.jsV_setproperty)(J, o, n.as_ptr());
                emit(format!("  set {pname:?} -> {}", !ps.is_null()));
                (api.jsV_delproperty)(J, o, n.as_ptr());
                let p = (api.jsV_getownproperty)(J, o, n.as_ptr());
                emit(format!("  after del {pname:?} -> {}", !p.is_null()));
            }
            (api.js_pop)(J, 1);
        }
        // jsV_* conversions on every primitive shape
        for k in 0..16 {
            match k {
                0 => (api.js_pushundefined)(J),
                1 => (api.js_pushnull)(J),
                2 => (api.js_pushboolean)(J, 0),
                3 => (api.js_pushboolean)(J, 1),
                4 => (api.js_pushnumber)(J, 0.0),
                5 => (api.js_pushnumber)(J, -0.0),
                6 => (api.js_pushnumber)(J, 1.5),
                7 => (api.js_pushnumber)(J, f64::NAN),
                8 => (api.js_pushnumber)(J, f64::INFINITY),
                9 => (api.js_pushstring)(J, cs("").as_ptr()),
                10 => (api.js_pushstring)(J, cs("12").as_ptr()),
                11 => (api.js_pushstring)(J, cs("abc").as_ptr()),
                12 => (api.js_pushliteral)(J, sstr("lit")),
                13 => (api.js_newobject)(J),
                14 => (api.js_newarray)(J),
                _ => (api.js_newnumber)(J, 3.0),
            }
            let vp = (api.js_tovalue)(J, -1);
            emit(format!(
                "jsV shape {k}: bool={} num={:#x} int={:#x}",
                (api.jsV_toboolean)(J, vp),
                (api.jsV_tonumber)(J, vp).to_bits(),
                (api.jsV_tointeger)(J, vp).to_bits()
            ));
            let vp = (api.js_tovalue)(J, -1);
            emit(format!("  tostring={:?}", s((api.jsV_tostring)(J, vp))));
            let vp = (api.js_tovalue)(J, -1);
            let o = (api.jsV_toobject)(J, vp);
            (api.js_pushobject)(J, o);
            emit(format!("  toobject: {}", dump_slot(api, J, -1)));
            (api.js_pop)(J, 1);
            for hint in [0, 1, 2, 7, -3] {
                (api.js_copy)(J, -1);
                let vp = (api.js_tovalue)(J, -1);
                (api.jsV_toprimitive)(J, vp, hint);
                emit(format!("  toprimitive({hint}): {}", dump_slot(api, J, -1)));
                (api.js_pop)(J, 1);
            }
            (api.js_pop)(J, 1);
        }
        // jsV_newmemstring
        for n in [0usize, 1, 7, 8, 64] {
            let bytes: Vec<u8> = (0..n).map(|i| b'a' + (i % 26) as u8).collect();
            let ms = (api.jsV_newmemstring)(J, bytes.as_ptr() as *const c_char, n as c_int);
            emit(format!("jsV_newmemstring({n}) -> {}", !ms.is_null()));
        }
        (api.js_gc)(J, 0);
    }
}

#[test]
fn row67_68_jsv_api() {
    diff_all_flags("jsV_* low-level API", task_jsv_api);
}

/* ------------------------------------------------------------------ */
/* rows 70-71 — interning table and allocator wrappers                 */
/* ------------------------------------------------------------------ */
fn task_intern_and_alloc(api: &Api, J: JS) {
    unsafe {
        let mut rng = Rng::new(0xC0DE_0070);
        let mut words: Vec<String> = vec![
            "".into(), "a".into(), "b".into(), "aa".into(), "ab".into(), "ba".into(),
            "length".into(), "toString".into(), "0".into(), "1".into(), "\u{e9}".into(),
            "a longer interned string".into(),
        ];
        for _ in 0..400 {
            let n = rng.below(8) as usize;
            words.push((0..n).map(|_| *rng.pick(&['a', 'b', 'c', '0', '1', 'Z'])).collect());
        }
        // interning is idempotent: the same bytes must map to the same pointer
        let mut first: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for w in &words {
            let c = cs(w);
            let p1 = (api.js_intern)(J, c.as_ptr());
            let p2 = (api.js_intern)(J, c.as_ptr());
            let stable = p1 == p2;
            let prev = first.insert(w.clone(), p1 as usize);
            let same_as_before = prev.map(|q| q == p1 as usize).unwrap_or(true);
            emit(format!(
                "intern {w:?}: stable={stable} same_as_before={same_as_before} text={:?}",
                s(p1)
            ));
        }
        // js_strdup / js_malloc / js_realloc / js_free
        for w in ["", "a", "abcdefghij", "\u{e9}\u{20ac}"] {
            let c = cs(w);
            let d = (api.js_strdup)(J, c.as_ptr());
            emit(format!("strdup {w:?} -> {:?}", s(d)));
            (api.js_free)(J, d as *mut c_void);
        }
        for n in [0, 1, 8, 64, 4096] {
            let p = (api.js_malloc)(J, n);
            emit(format!("malloc({n}) -> nonnull={}", !p.is_null()));
            let q = (api.js_realloc)(J, p, n * 2 + 1);
            emit(format!("realloc({}) -> nonnull={}", n * 2 + 1, !q.is_null()));
            (api.js_free)(J, q);
        }
        (api.js_free)(J, std::ptr::null_mut());
        emit("free(NULL) ok".into());
    }
}

#[test]
fn row70_71_intern_and_alloc() {
    diff_all_flags("intern + allocator wrappers", task_intern_and_alloc);
}

/* jsS_dumpstrings / jsS_freestrings and js_trap print to stdout (row 84) */

/// Replace `0x…` hex addresses with a placeholder.
fn mask_addrs(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'0' && i + 1 < b.len() && (b[i + 1] == b'x' || b[i + 1] == b'X') {
            let mut j = i + 2;
            while j < b.len() && (b[j] as char).is_ascii_hexdigit() {
                j += 1;
            }
            if j > i + 2 {
                out.push_str("0x<addr>");
                i = j;
                continue;
            }
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

#[test]
fn row84_stdout_writers() {
    let p = both();
    for (tag, api) in [("C", &p.c), ("RUST", &p.r)] {
        let _ = tag;
        let _ = api;
    }
    // Compare stdout from both libraries for the printing entry points.
    let run = |api: &Api| -> String {
        capture_stdout(|| unsafe {
            let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            for w in ["zeta", "alpha", "mid", "beta", "omega", "", "a"] {
                let c = cs(w);
                let _ = (api.js_intern)(J, c.as_ptr());
            }
            (api.jsS_dumpstrings)(J);
            (api.js_pushnumber)(J, 1.0);
            (api.js_pushstring)(J, cs("s").as_ptr());
            (api.js_newobject)(J);
            (api.js_trap)(J, 0);
            (api.js_trap)(J, -1);
            (api.js_trap)(J, 12345);
            (api.js_gc)(J, 1);
            (api.js_freestate)(J);
        })
    };
    let a = mask_addrs(&run(&p.c));
    let b = mask_addrs(&run(&p.r));
    // `js_dumpvalue` prints objects as `[Object %p]` (jsrun.c), so raw addresses
    // are masked; everything else (the interning tree, the stack layout, the
    // stack trace) is compared verbatim.
    assert_eq!(a, b, "stdout from jsS_dumpstrings / js_trap / js_gc(1) differs");
    assert!(a.contains("interned strings"), "capture failed: {a:?}");

    // js_putc / js_puts / js_putm build a js_Buffer; compare its bytes.
    #[repr(C)]
    struct Buf {
        n: c_int,
        m: c_int,
        s: [u8; 64],
    }
    let run_buf = |api: &Api| -> String {
        unsafe {
            let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            let mut out = String::new();
            let mut sb: *mut c_void = std::ptr::null_mut();
            for c in 0..=255u32 {
                (api.js_putc)(J, &mut sb, c as c_int);
            }
            for t in ["", "a", "hello", "\u{e9}\u{20ac}", "0123456789"] {
                let ct = cs(t);
                (api.js_puts)(J, &mut sb, ct.as_ptr());
            }
            let long = cs("the quick brown fox jumps over the lazy dog 0123456789");
            let base = long.as_ptr();
            for (a, b) in [(0usize, 0usize), (0, 5), (5, 10), (10, 10), (0, 54)] {
                (api.js_putm)(J, &mut sb, base.add(a), base.add(b));
            }
            (api.js_putc)(J, &mut sb, 0);
            let bf = &*(sb as *const Buf);
            out.push_str(&format!("n={} ", bf.n));
            let bytes = std::slice::from_raw_parts(bf.s.as_ptr(), bf.n.max(0) as usize);
            out.push_str(&format!("{:02x?}", bytes));
            (api.js_free)(J, sb);
            (api.js_freestate)(J);
            out
        }
    };
    let a = run_buf(&p.c);
    let b = run_buf(&p.r);
    assert_eq!(a, b, "js_putc/js_puts/js_putm buffer differs");
}

/* ------------------------------------------------------------------ */
/* row 72 — the GC in both report modes, with real garbage              */
/* ------------------------------------------------------------------ */
fn task_gc(api: &Api, J: JS) {
    unsafe {
        for report in [0, 1, 2, -1] {
            eval(
                api,
                J,
                "make garbage",
                "var keep=[]; for (var i=0;i<200;i++) { var o={a:[1,2,3],s:'x'+i}; \
                 if (i%10==0) keep.push(o) } keep.length",
            );
            (api.js_gc)(J, report);
            emit(format!("gc({report}) done; top={}", (api.js_gettop)(J)));
            eval(api, J, "after gc", "keep.length + ',' + keep[0].s");
        }
        // cyclic garbage
        eval(
            api,
            J,
            "cycles",
            "var a={},b={}; a.b=b; b.a=a; a=null; b=null; 'ok'",
        );
        (api.js_gc)(J, 0);
        (api.js_gc)(J, 1);
        // strings and functions
        eval(
            api,
            J,
            "closures",
            "var fs=[]; for (var i=0;i<50;i++) fs.push((function(k){return function(){return k}})(i)); \
             fs[10]()",
        );
        (api.js_gc)(J, 1);
        eval(api, J, "closures live", "fs[49]()");
    }
}

#[test]
fn row72_gc() {
    diff_all_flags("gc", task_gc);
}

/* ------------------------------------------------------------------ */
/* row 73 — report / panic / context plumbing                          */
/* ------------------------------------------------------------------ */
unsafe extern "C-unwind" fn report_two(_J: JS, msg: *const c_char) {
    emit(format!("REPORT2 {}", unsafe { s(msg) }));
}
unsafe extern "C-unwind" fn panic_noop(_J: JS) {
    emit("PANIC handler".into());
}

fn task_callbacks(api: &Api, J: JS) {
    unsafe {
        // swapping the report callback is observable through js_report and
        // js_dostring
        (api.js_report)(J, cs("first").as_ptr());
        (api.js_setreport)(J, Some(report_two));
        (api.js_report)(J, cs("second").as_ptr());
        let bad = cs("this is (not valid javascript");
        emit(format!("dostring rc={}", (api.js_dostring)(J, bad.as_ptr())));
        (api.js_setreport)(J, None);
        (api.js_report)(J, cs("third (default handler)").as_ptr());
        emit(format!("dostring rc={}", (api.js_dostring)(J, bad.as_ptr())));

        // js_atpanic returns the previous handler
        let p0 = (api.js_atpanic)(J, Some(panic_noop));
        emit(format!("atpanic p0_some={}", p0.is_some()));
        let p1 = (api.js_atpanic)(J, None);
        emit(format!("atpanic p1_is_ours={}", p1 == Some(panic_noop)));
        let p2 = (api.js_atpanic)(J, p0);
        emit(format!("atpanic p2_is_none={}", p2.is_none()));

        // context pointer
        for v in [0usize, 1, 0xdeadbeef, usize::MAX] {
            (api.js_setcontext)(J, v as *mut c_void);
            emit(format!("context {v:#x} -> {:#x}", (api.js_getcontext)(J) as usize));
        }
    }
}

#[test]
fn row73_callbacks() {
    // with_report=false so the driver does not install its own report callback
    let cfg = Cfg {
        with_report: false,
        ..Cfg::default()
    };
    diff("report/panic/context plumbing", cfg, task_callbacks);
    diff(
        "report/panic/context plumbing [strict]",
        Cfg {
            with_report: false,
            flags: JS_STRICT,
            ..Cfg::default()
        },
        task_callbacks,
    );
}

/* ------------------------------------------------------------------ */
/* row 69 — flat -> hashed array transitions through the low-level API  */
/* ------------------------------------------------------------------ */
fn task_array_transitions(api: &Api, J: JS) {
    unsafe {
        for n in [0usize, 1, 2, 7, 8, 9, 16, 17, 64, 100] {
            (api.js_newarray)(J);
            for i in 0..n {
                (api.js_pushnumber)(J, i as f64);
                (api.js_setindex)(J, -2, i as c_int);
            }
            let o = (api.js_toobject)(J, -1);
            emit(format!("array n={n} before: len={}", (api.js_getlength)(J, -1)));
            let it = (api.jsV_newiterator)(J, o, 1);
            let mut keys = Vec::new();
            loop {
                match opt_str((api.jsV_nextiterator)(J, it)) {
                    None => break,
                    Some(b) => {
                        keys.push(String::from_utf8_lossy(&b).into_owned());
                        if keys.len() > 300 {
                            break;
                        }
                    }
                }
            }
            emit(format!("  flat iter keys={keys:?}"));
            (api.jsR_unflattenarray)(J, o);
            emit(format!("  after unflatten len={}", (api.js_getlength)(J, -1)));
            let it = (api.jsV_newiterator)(J, o, 1);
            let mut keys = Vec::new();
            loop {
                match opt_str((api.jsV_nextiterator)(J, it)) {
                    None => break,
                    Some(b) => {
                        keys.push(String::from_utf8_lossy(&b).into_owned());
                        if keys.len() > 300 {
                            break;
                        }
                    }
                }
            }
            emit(format!("  hashed iter keys={keys:?}"));
            for newlen in [0usize, 1, n, n + 1, n * 2] {
                (api.jsV_resizearray)(J, o, newlen as c_int);
                emit(format!(
                    "  resize({newlen}) -> len={} repr={:?}",
                    (api.js_getlength)(J, -1),
                    {
                        let sentinel = cs("<throw>");
                        s((api.js_tryrepr)(J, -1, sentinel.as_ptr()))
                    }
                ));
            }
            // double unflatten
            (api.jsR_unflattenarray)(J, o);
            emit(format!("  double unflatten len={}", (api.js_getlength)(J, -1)));
            (api.js_pop)(J, 1);
        }
    }
}

#[test]
fn row69_array_transitions() {
    diff_all_flags("array flat/hashed transitions", task_array_transitions);
}
