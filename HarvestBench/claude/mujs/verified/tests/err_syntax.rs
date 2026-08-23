//! Phase C: the SYNTAX error surface — ERRORS.md rows 259-476.
//!
//!   * jslex.c    rows 259-322  (`jsY_error`, every number/string/regexp/comment
//!                               rejection, `jsY_unescape`, `lexescape`, the
//!                               JSON lexer)
//!   * jsparse.c  rows 323-407  (`jsP_error`, `jsP_warning`, the 17 `INCREC`
//!                               sites, the ~44 `jsP_expect` sites)
//!   * jscompile.c rows 408-460 (`jsC_error`, `js_syntaxerror`, `js_evalerror`)
//!   * jsdtoa.c   rows 461-476  (`js_strtod` clamping, `js_fmtexp`)
//!
//! All three error helpers build their message as `"%s:%d: " + msg` from
//! `J->filename` and the CURRENT line (`J->lexline` for the lexer/parser,
//! `node->line` for the compiler), so the FILENAME and the LINE NUMBER are part
//! of the observable result and are compared too.  Every source is therefore run
//! through several drivers, each of which supplies a different filename:
//!
//!   `js_ploadstring(filename, src)`  -> caller-chosen filename
//!   `js_dostring(src)`               -> "[string]"
//!   `js_loadeval(filename, src)`     -> caller-chosen filename, `iseval` = 1
//!                                       (picks `J->strict` and a NULL scope)
//!   `JSON.parse(text)`               -> "JSON"
//!
//! and in both non-strict and `JS_STRICT` mode.
//!
//! Every call goes through the two `.so` exports; nothing throwing is ever
//! invoked outside a protected frame (`js_ploadstring` / `js_dostring` /
//! `js_pcall`).
#![allow(clippy::needless_range_loop)]

mod common;
use common::*;
use std::cell::Cell;
use std::ffi::{c_char, c_int, CString};

macro_rules! cn {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

/* ===================================================================== */
/*  Drivers                                                              */
/* ===================================================================== */

/// Everything observable about one load attempt: the protected return code, the
/// type of whatever was left on the stack, the `toString()` of the thrown error
/// (which is `"<Name>: <filename>:<line>: <msg>"` for all three helpers), the
/// resulting stack depth and everything the report hook printed.
#[derive(Debug, PartialEq, Eq, Clone)]
struct Load {
    rc: c_int,
    ty: String,
    msg: String,
    top: c_int,
    out: String,
}

fn pload(l: &Lib, flags: c_int, filename: &str, src: &str) -> Load {
    unsafe {
        out_clear();
        let j = new_state(l, flags);
        let cf = cstr(filename);
        let cs = cstr(src);
        let rc = l.js_ploadstring(j, cf.as_ptr(), cs.as_ptr());
        let ty = from_c(l.js_typeof(j, -1));
        let msg = if rc != 0 {
            from_c(l.js_trystring(j, -1, ERRSTR))
        } else {
            String::new()
        };
        l.js_pop(j, 1);
        let top = l.js_gettop(j);
        l.js_freestate(j);
        Load {
            rc,
            ty,
            msg,
            top,
            out: out_take(),
        }
    }
}

/// Same, but with a `js_setlimit` memory budget so that the `js_malloc` /
/// `js_realloc` failure paths inside the lexer, parser and compiler are taken.
fn pload_mem(l: &Lib, flags: c_int, lim: c_int, src: &str) -> Load {
    unsafe {
        out_clear();
        let j = new_state(l, flags);
        l.js_setlimit(j, 0, lim);
        let cs = cstr(src);
        let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
        let ty = from_c(l.js_typeof(j, -1));
        let msg = if rc != 0 {
            from_c(l.js_trystring(j, -1, ERRSTR))
        } else {
            String::new()
        };
        l.js_pop(j, 1);
        let top = l.js_gettop(j);
        l.js_freestate(j);
        Load {
            rc,
            ty,
            msg,
            top,
            out: out_take(),
        }
    }
}

/// Instruction budget for the drivers that RUN the compiled script.  Several of
/// the shapes needed to reach a given error site are legal infinite loops once
/// they compile (`for(;;);`, `while(1){continue}`, ...), so every executing
/// driver charges a `js_setlimit` run limit.  One decrement per VM instruction
/// is charged identically by both libraries (api_state.rs `t_setlimit_runlimit`
/// pins that), so the cut-off point is itself a compared observable.
const RUNLIMIT: c_int = 300_000;

/// `js_dostring`, which uses the fixed filename `"[string]"` (jsstate.c:151) and
/// reports the error through the report hook instead of leaving it on the stack.
fn dorun(l: &Lib, flags: c_int, src: &str) -> Load {
    unsafe {
        out_clear();
        let j = new_state(l, flags);
        l.js_setlimit(j, RUNLIMIT, 0);
        let cs = cstr(src);
        let rc = l.js_dostring(j, cs.as_ptr());
        let top = l.js_gettop(j);
        l.js_freestate(j);
        Load {
            rc,
            ty: String::new(),
            msg: String::new(),
            top,
            out: out_take(),
        }
    }
}

/// `js_ploadstring` + `js_pushundefined` + `js_pcall`: compile AND run.
fn prun(l: &Lib, flags: c_int, filename: &str, src: &str) -> Load {
    unsafe {
        out_clear();
        let j = new_state(l, flags);
        l.js_setlimit(j, RUNLIMIT, 0);
        let cf = cstr(filename);
        let cs = cstr(src);
        let load_rc = l.js_ploadstring(j, cf.as_ptr(), cs.as_ptr());
        let rc;
        let msg;
        if load_rc == 0 {
            l.js_pushundefined(j);
            let call_rc = l.js_pcall(j, 0);
            rc = 10 + call_rc;
            msg = from_c(l.js_tryrepr(j, -1, ERRSTR));
        } else {
            rc = load_rc;
            msg = from_c(l.js_trystring(j, -1, ERRSTR));
        }
        let ty = from_c(l.js_typeof(j, -1));
        l.js_pop(j, 1);
        let top = l.js_gettop(j);
        l.js_freestate(j);
        Load {
            rc,
            ty,
            msg,
            top,
            out: out_take(),
        }
    }
}

thread_local! {
    static LE_FILE: Cell<*const c_char> = const { Cell::new(std::ptr::null()) };
    static LE_SRC: Cell<*const c_char> = const { Cell::new(std::ptr::null()) };
}

/// `js_loadeval` is unprotected, so it is driven from inside a cfunction that is
/// itself entered through `js_pcall`.
unsafe extern "C" fn cf_loadeval(j: JS) {
    let l = cur();
    let f = LE_FILE.with(|c| c.get());
    let s = LE_SRC.with(|c| c.get());
    l.js_loadeval(j, f, s);
    out_push(format!("[script={}]\n", from_c(l.js_typeof(j, -1))).as_bytes());
    l.js_pushundefined(j);
    l.js_call(j, 0);
}

const N_LE: *const c_char = cn!("le");

fn leval(l: &Lib, flags: c_int, filename: &str, src: &str) -> Load {
    unsafe {
        out_clear();
        let j = new_state(l, flags);
        l.js_setlimit(j, RUNLIMIT, 0);
        let cf = cstr(filename);
        let cs = cstr(src);
        LE_FILE.with(|c| c.set(cf.as_ptr()));
        LE_SRC.with(|c| c.set(cs.as_ptr()));
        l.js_newcfunction(j, Some(cf_loadeval), N_LE, 0);
        l.js_setglobal(j, N_LE);
        l.js_getglobal(j, N_LE);
        l.js_pushundefined(j);
        let rc = l.js_pcall(j, 0);
        let ty = from_c(l.js_typeof(j, -1));
        let msg = from_c(l.js_trystring(j, -1, ERRSTR));
        l.js_pop(j, 1);
        let top = l.js_gettop(j);
        l.js_freestate(j);
        LE_FILE.with(|c| c.set(std::ptr::null()));
        LE_SRC.with(|c| c.set(std::ptr::null()));
        Load {
            rc,
            ty,
            msg,
            top,
            out: out_take(),
        }
    }
}

fn diff_pload(flags: c_int, filename: &str, src: &str) {
    let p = libs();
    let a = pload(&p.c, flags, filename, src);
    let b = pload(&p.rs, flags, filename, src);
    assert_eq!(
        a, b,
        "js_ploadstring divergence (flags={flags} file={filename:?})\nsrc: {src:?}"
    );
}

fn diff_pload_mem(flags: c_int, lim: c_int, src: &str) {
    let p = libs();
    let a = pload_mem(&p.c, flags, lim, src);
    let b = pload_mem(&p.rs, flags, lim, src);
    assert_eq!(
        a, b,
        "js_ploadstring/memlimit divergence (flags={flags} lim={lim})\nsrc: {src:?}"
    );
}

fn diff_leval(flags: c_int, filename: &str, src: &str) {
    let p = libs();
    let a = leval(&p.c, flags, filename, src);
    let b = leval(&p.rs, flags, filename, src);
    assert_eq!(
        a, b,
        "js_loadeval divergence (flags={flags} file={filename:?})\nsrc: {src:?}"
    );
}

fn diff_dorun(flags: c_int, src: &str) {
    let p = libs();
    let a = dorun(&p.c, flags, src);
    let b = dorun(&p.rs, flags, src);
    assert_eq!(a, b, "js_dostring divergence (flags={flags})\nsrc: {src:?}");
}

fn diff_prun(flags: c_int, filename: &str, src: &str) {
    let p = libs();
    let a = prun(&p.c, flags, filename, src);
    let b = prun(&p.rs, flags, filename, src);
    assert_eq!(
        a, b,
        "js_ploadstring+js_pcall divergence (flags={flags} file={filename:?})\nsrc: {src:?}"
    );
}

/// Two drivers x two strictness settings: the cheap sweep used for the large
/// randomised corpora.  `js_ploadstring` never runs the script, so a source that
/// would loop forever is still safe here.
fn diff_quick(src: &str) {
    for flags in [0, JS_STRICT] {
        diff_pload(flags, "test.js", src);
    }
}

/// The full matrix: every driver (and therefore every filename) x both
/// strictness settings.  `js_ploadstring` only compiles; `js_dostring`,
/// `js_ploadstring`+`js_pcall` and `js_loadeval`+`js_call` also run the result
/// under `RUNLIMIT`.
fn diff_all(src: &str) {
    for flags in [0, JS_STRICT] {
        for f in ["test.js", "", "deep/dir/name.js"] {
            diff_pload(flags, f, src);
        }
        diff_dorun(flags, src);
        diff_prun(flags, "test.js", src);
        diff_leval(flags, "(le)", src);
    }
}

/// Like `diff_all` but compiles only (no execution).
fn diff_compileonly(src: &str) {
    for flags in [0, JS_STRICT] {
        for f in ["test.js", "", "deep/dir/name.js"] {
            diff_pload(flags, f, src);
        }
    }
}

/* ===================================================================== */
/*  jslex.c rows 259-264: the shared helper and the character helpers    */
/* ===================================================================== */

/// Rows 259 / 323 / 408: `jsY_error`, `jsP_error` and `jsC_error` all build
/// `snprintf(buf, 256, "%s:%d: ", J->filename, line)` + `strcat(buf, msgbuf)`.
/// The filename, the line and the 256-byte truncation of the prefix are all
/// observable, so sweep filenames (including one longer than the prefix
/// buffer), every line terminator form and a range of line numbers.
#[test]
fn t_error_prefix_filename_line_and_truncation() {
    let long_name = "N".repeat(300);
    let filenames: Vec<&str> = vec![
        "test.js",
        "",
        "a",
        "deep/dir/name.js",
        "with space.js",
        "pct%s%d.js",
        &long_name,
    ];
    // one lexical (jsY_error), one parse (jsP_error) and three compile
    // (jsC_error) rejections
    let bodies = [
        "@",       // jslex.c:728  unexpected character: '@'
        "1e",      // jslex.c:377  missing exponent
        "var 1",   // jsparse.c:166 unexpected token: (number) (expected identifier)
        "return",  // jscompile.c:1251 return not in function
        "break",   // jscompile.c:1221 unlabelled break ...
        "1 = 2",   // jscompile.c:400  invalid l-value in assignment
        "delete 1",// jscompile.c:524  invalid l-value in delete expression
    ];
    for f in &filenames {
        for term in ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
            for nl in [0usize, 1, 2, 5, 17] {
                for b in bodies {
                    let src = format!("{}{}", term.repeat(nl), b);
                    diff_pload(0, f, &src);
                    diff_pload(JS_STRICT, f, &src);
                }
            }
        }
    }
    // mixed terminators, so that the line counter has to agree exactly
    for b in bodies {
        let src = format!("\n\r\n\r\u{2028}\u{2029}\n\n\r\r\n{b}");
        diff_all(&src);
    }
    // `\r\n` is consumed as ONE line terminator (jslex.c:166-167)
    for n in 1..12 {
        let a = format!("{}@", "\r\n".repeat(n));
        let b = format!("{}@", "\n".repeat(n));
        diff_pload(0, "t.js", &a);
        diff_pload(0, "t.js", &b);
        // and they must land on the same line
        let p = libs();
        assert_eq!(
            pload(&p.c, 0, "t.js", &a).msg,
            pload(&p.c, 0, "t.js", &b).msg,
            "CR LF must count as one line terminator"
        );
    }
}

/// Rows 259 / 323 / 408, truncation edges.  All three helpers format the prefix
/// with `snprintf(buf, 256, ...)` into a `char buf[512]` and then `strcat` a
/// message that was itself built with `vsnprintf(msgbuf, 256, ...)`, so:
///
///   * a filename around 250 characters long makes the prefix truncate part way
///     through `":<line>: "`, and the truncation point moves with the number of
///     digits in the line number;
///   * a `%s` argument longer than ~240 characters makes the MESSAGE truncate.
///
/// Both truncations are observable in the thrown message.
#[test]
fn t_error_message_truncation_edges() {
    let p = libs();
    // prefix truncation: filename length x line-number digit count
    for len in 230..=270usize {
        let f = "F".repeat(len);
        for nl in [0usize, 8, 9, 98, 99, 998, 999] {
            let src = format!("{}@", "\n".repeat(nl));
            let a = pload(&p.c, 0, &f, &src);
            let b = pload(&p.rs, 0, &f, &src);
            assert_eq!(a, b, "prefix truncation (filename len {len}, line {})", nl + 1);
            assert!(a.rc != 0);
        }
    }
    // message truncation: a long %s argument
    for n in [1usize, 10, 200, 230, 235, 240, 241, 242, 243, 250, 300, 600] {
        let long = "L".repeat(n);
        for (src, flags) in [
            // "break label '%s' not found"
            (format!("break {long};"), 0),
            // "continue label '%s' not found"
            (format!("continue {long};"), 0),
            // "duplicate formal parameter '%s'"
            (format!("function f({long},{long}){{}}"), JS_STRICT),
            // "duplicate property '%s' in object literal"
            (format!("({{'{long}':1,'{long}':2}})"), JS_STRICT),
            // "'%s' is a future reserved word" cannot be long, but the
            // by-name identifier paths still carry the name through emitstring
            (format!("var {long} = 1; print({long})"), 0),
        ] {
            let a = pload(&p.c, flags, "t.js", &src);
            let b = pload(&p.rs, flags, "t.js", &src);
            assert_eq!(a, b, "message truncation (n={n})");
            // both truncations at once
            let f = "F".repeat(250);
            assert_eq!(
                pload(&p.c, flags, &f, &src),
                pload(&p.rs, flags, &f, &src),
                "prefix+message truncation (n={n})"
            );
        }
    }
    // a `%`-bearing filename and a `%`-bearing `%s` argument must be passed as
    // ARGUMENTS, never re-interpreted as a format
    for f in ["%s%d%n.js", "%%%%", "%s", "%1000000d"] {
        for src in [
            "@",
            "break %s%d;",
            "({'%s%d%n':1,'%s%d%n':2})",
            "function g(%s){}",
        ] {
            for flags in [0, JS_STRICT] {
                diff_pload(flags, f, src);
            }
        }
    }
    for key in ["%s", "%d", "%n", "%%", "%s%s%s%s%s%s%s%s%s%s"] {
        let src = format!("({{'{key}':1,'{key}':2}})");
        diff_pload(JS_STRICT, "t.js", &src);
        diff_pload(0, "t.js", &src);
    }
}

/// Row 292's message is built with `"%c"` and `J->lexchar` is a `Rune`, so a
/// non-ASCII flag character is TRUNCATED to its low byte by the `int -> unsigned
/// char` conversion inside `vsnprintf`.  When that low byte is 0 the message is
/// cut short at the embedded NUL.  Same story for row 265's `"expected '%c'"`.
#[test]
fn t_regexp_flag_char_is_truncated_to_one_byte() {
    let p = libs();
    let runes = [
        // low byte != 0
        '\u{e9}', '\u{391}', '\u{4e2d}', '\u{ff47}', '\u{ff49}', '\u{ff4d}',
        '\u{2160}', '\u{a1}', '\u{101}', '\u{1e9}',
        // low byte == 0 -> the %c writes a NUL and cuts the message
        '\u{100}', '\u{200}', '\u{300}', '\u{400}', '\u{500}', '\u{1000}',
        '\u{4e00}', '\u{ff00}', '\u{10000}',
    ];
    for r in runes {
        for src in [
            format!("/a/{r}"),
            format!("/a/g{r}"),
            format!("var re=/abc/{r}"),
            format!("/a/{r}g"),
        ] {
            for flags in [0, JS_STRICT] {
                diff_pload(flags, "t.js", &src);
            }
        }
    }
    // pin the truncation for one rune of each kind
    let a = pload(&p.c, 0, "t.js", "/a/\u{ff47}");
    assert_eq!(
        a.msg, "SyntaxError: t.js:1: illegal flag in regular expression: G",
        "U+FF47 must be truncated to its low byte 0x47 = 'G'"
    );
    assert_eq!(a, pload(&p.rs, 0, "t.js", "/a/\u{ff47}"));
    let b = pload(&p.c, 0, "t.js", "/a/\u{100}");
    assert_eq!(
        b.msg, "SyntaxError: t.js:1: illegal flag in regular expression: ",
        "U+0100 has low byte 0, so the %c writes a NUL and cuts the message"
    );
    assert_eq!(b, pload(&p.rs, 0, "t.js", "/a/\u{100}"));
}

/// Rows 326-343 through the OTHER parser entry point: `jsP_parsefunction`
/// (jsparse.c:1054) resets `J->astdepth` for the parameter list and then again
/// inside `jsP_parse` for the body, and uses the filename `"[string]"`
/// (jsfunction.c:31).  Reached from JS with `new Function(params, body)`.
fn body_t_function_constructor_parser() {
    let mut srcs: Vec<String> = vec![];
    for n in [0usize, 1, 5, 190, 195, 198, 199, 200, 201, 202, 205, 398, 399, 400, 401, 405] {
        let deep_expr = format!("{}1{}", "(".repeat(n), ")".repeat(n));
        let chain = format!("1{}", "+1".repeat(n));
        srcs.push(format!("new Function('return {deep_expr}')"));
        srcs.push(format!("new Function('return {chain}')"));
        srcs.push(format!("new Function('{}', 'return 1')", "!".repeat(0)));
        srcs.push(format!("new Function('return {}1', '')", "!".repeat(n)));
        srcs.push(format!(
            "new Function('{}', 'return 1')",
            (0..n.min(60)).map(|i| format!("p{i}")).collect::<Vec<_>>().join(",")
        ));
    }
    // parse / compile errors reported with the "[string]" filename
    for s in [
        "new Function('@')",
        "new Function('1e')",
        "new Function('var 1')",
        "new Function('return')",
        "new Function('break')",
        "new Function('1=2')",
        "new Function('a', 'b', '@')",
        "new Function('1', 'return 1')",
        "new Function('a,', 'return 1')",
        "new Function('a b', 'return 1')",
        "new Function('eval', 'return 1')",
        "new Function('arguments', 'return 1')",
        "new Function('class', 'return 1')",
        "new Function('a', 'a', 'return 1')",
        "new Function('return \"use strict\"')",
        "new Function()",
        "new Function('')",
        "Function('return 1')()",
    ] {
        srcs.push(s.to_string());
    }
    for s in &srcs {
        for flags in [0, JS_STRICT] {
            diff_dorun(flags, s);
            diff_prun(flags, "test.js", s);
        }
    }
}

#[test]
fn t_function_constructor_parser() {
    with_big_stack(body_t_function_constructor_parser);
}

/// Rows 260 / 261: `jsY_tokenstring` out of range, and the NULL filler slots
/// (0x80..0xFF) both return the literal `"<unknown>"`.
#[test]
fn t_tokenstring_unknown() {
    let p = libs();
    unsafe {
        let mut vals: Vec<c_int> = (-40..400).collect();
        vals.extend([i32::MIN, i32::MIN + 1, -1, 0, 156, 157, 312, 313, 1 << 20, i32::MAX]);
        for t in vals {
            assert_eq!(
                from_c(p.c.jsY_tokenstring(t)),
                from_c(p.rs.jsY_tokenstring(t)),
                "jsY_tokenstring({t})"
            );
        }
        // every filler slot in 128..=255 must be "<unknown>"
        for t in 128..=255 {
            assert_eq!(from_c(p.c.jsY_tokenstring(t)), "<unknown>");
            assert_eq!(from_c(p.rs.jsY_tokenstring(t)), "<unknown>");
        }
    }
}

/// Row 262: `jsY_findword` returns -1 when the binary search finds no exact
/// match.  Driven directly against the two real tables (`keywords` and the two
/// `futurewords` lists are the only callers) and through `checkfutureword`.
#[test]
fn t_findword_no_match() {
    let p = libs();
    unsafe {
        let future = [
            "class", "const", "enum", "export", "extends", "import", "super",
        ];
        let strictfuture = [
            "implements", "interface", "let", "package", "private", "protected",
            "public", "static", "yield",
        ];
        let keywords = [
            "break", "case", "catch", "continue", "debugger", "default", "delete",
            "do", "else", "false", "finally", "for", "function", "if", "in",
            "instanceof", "new", "null", "return", "switch", "this", "throw",
            "true", "try", "typeof", "var", "void", "while", "with",
        ];
        for list in [&future[..], &strictfuture[..], &keywords[..]] {
            let cs: Vec<CString> = list.iter().map(|w| cstr(w)).collect();
            let ptrs: Vec<*const c_char> = cs.iter().map(|c| c.as_ptr()).collect();
            let mut probes: Vec<String> = list.iter().map(|s| s.to_string()).collect();
            probes.extend(
                [
                    "", "a", "zzz", "Class", "CLASS", "clas", "classs", "let ", " let",
                    "publi", "publics", "aaaa", "~", "0",
                ]
                .iter()
                .map(|s| s.to_string()),
            );
            let mut rng = Rng::new(0x5EED_0001);
            for _ in 0..2000 {
                probes.push(rng.ascii_string(9));
            }
            for probe in &probes {
                let ps = cstr(probe);
                for n in 0..=ptrs.len() {
                    assert_eq!(
                        p.c.jsY_findword(ps.as_ptr(), ptrs.as_ptr(), n as c_int),
                        p.rs.jsY_findword(ps.as_ptr(), ptrs.as_ptr(), n as c_int),
                        "jsY_findword({probe:?}, n={n})"
                    );
                }
            }
        }
    }
}

/// Row 263: `jsY_tohex` returns 0 for anything that is not `0-9a-fA-F`.  Only
/// reachable directly (every in-tree caller pre-checks with `jsY_ishex`).
#[test]
fn t_tohex_non_hex() {
    let p = libs();
    unsafe {
        let mut vals: Vec<c_int> = (-300..600).collect();
        vals.extend([i32::MIN, i32::MIN + 1, -1, 0x10ffff, i32::MAX, i32::MAX - 1]);
        let mut rng = Rng::new(0x5EED_0002);
        for _ in 0..3000 {
            vals.push(rng.next_u32() as c_int);
        }
        for c in vals {
            assert_eq!(
                p.c.int_pred("jsY_tohex", c),
                p.rs.int_pred("jsY_tohex", c),
                "jsY_tohex({c})"
            );
            assert_eq!(
                p.c.int_pred("jsY_ishex", c),
                p.rs.int_pred("jsY_ishex", c),
                "jsY_ishex({c})"
            );
        }
    }
}

/// Row 264: `jsY_next` turns the NUL that terminates the source buffer into
/// `EOF` without advancing `J->source`.  Reached by every construct that runs
/// off the end of the input, and by row 295 (`jsY_lexx` returning token 0).
#[test]
fn t_eof_at_end_of_source() {
    for src in [
        "", " ", "\t", "\n", "\r", "\r\n", "\u{2028}", "\u{2029}", "\u{a0}",
        "\u{feff}", "\u{b}", "\u{c}", "//", "// x", "/**/", ";", "1", "1;",
        "var", "var a", "var a=", "a.", "a[", "a(", "'a", "\"a", "'a\\", "\"a\\",
        "/a", "/a\\", "/*", "/* x", "{", "}", "(", ")", "1+", "1+1", "function",
        "function f", "function f(", "function f()", "function f(){",
        "\\", "\\u", "\\u0", "\\u00", "\\u004", "\\u0041",
    ] {
        diff_quick(src);
        diff_dostring(0, src);
        diff_dostring(JS_STRICT, src);
    }
}

/* ===================================================================== */
/*  jslex.c rows 265-297: identifier escapes, numbers, strings, regexps  */
/* ===================================================================== */

/// Rows 266-270: `jsY_unescape`.  A `\` in identifier position must be followed
/// by `u` and four hex digits; each of the five failure points reports the same
/// `"unexpected escape sequence"`.
#[test]
fn t_unescape_identifier_errors() {
    let mut srcs: Vec<String> = vec![];
    // row 266: `\` not followed by `u`
    for c in [
        "", "b", "x", "U", "0", "\\", "'", " ", "\n", "\t", "-", "$", "_", "n",
    ] {
        srcs.push(format!("\\{c}"));
        srcs.push(format!("a\\{c}"));
        srcs.push(format!("ab\\{c}c"));
        srcs.push(format!("var \\{c}"));
    }
    // rows 267-270: each of the four hex digits in turn
    for (i, pat) in [
        "\\uZ123", "\\u1Z23", "\\u12Z3", "\\u123Z", "\\uZZZZ",
        "\\u", "\\u0", "\\u00", "\\u000",
        "\\u-123", "\\u1 23", "\\u12'3", "\\u123 ",
    ]
    .iter()
    .enumerate()
    {
        srcs.push(pat.to_string());
        srcs.push(format!("a{pat}"));
        srcs.push(format!("a{pat}b"));
        srcs.push(format!("var {pat} = {i}"));
        srcs.push(format!("x.{pat}"));
        srcs.push(format!("({{ {pat}: 1 }})"));
    }
    // valid escapes must still work, including ones that decode to a
    // non-identifier rune (row 297) and to keywords
    for pat in [
        "\\u0041", "\\u0061", "\\u0024", "\\u005F", "\\u00e9", "\\u4e2d",
        "\\u0000", "\\u0020", "\\u0009", "\\u000A", "\\u0030", "\\u007F",
        "\\u2028", "\\u2029", "\\uFFFF", "\\u0076\\u0061\\u0072",
    ] {
        srcs.push(pat.to_string());
        srcs.push(format!("var {pat}"));
        srcs.push(format!("a{pat}"));
        srcs.push(format!("{pat}a"));
    }
    // randomised: 4 characters drawn from hex digits + near-misses
    let alphabet: Vec<char> = "0123456789abcdefABCDEFgGzZ_$ '\"-+.\\\n\t".chars().collect();
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..700 {
        let s: String = (0..4)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
            .collect();
        srcs.push(format!("\\u{s}"));
        srcs.push(format!("a\\u{s}b"));
    }
    for s in &srcs {
        diff_quick(s);
    }
    for s in srcs.iter().take(120) {
        diff_all(s);
    }
}

/// Rows 271 / 272: `textinit` allocates the 4096-byte lexer buffer and
/// `textpush` doubles it; both go through `js_malloc` / `js_realloc`, so a
/// `js_setlimit` memory budget makes them throw the raw literal
/// `"out of memory"`.  Sweep the budget so the failure point is compared at
/// every possible position.
#[test]
fn t_lexbuf_allocation_out_of_memory() {
    let srcs = [
        // needs textinit but no growth
        "var s = 'short'".to_string(),
        // token longer than the initial 4096 cap -> textpush doubles
        format!("var s = '{}'", "a".repeat(5000)),
        format!("var s = '{}'", "a".repeat(20000)),
        // long identifier (textinit + textpush in the identifier path)
        format!("var {} = 1", "i".repeat(9000)),
        // long regexp body
        format!("var r = /{}/", "a".repeat(6000)),
        // many separate tokens, so textinit is hit repeatedly
        (0..400)
            .map(|i| format!("var s{i} = 'value{i}';"))
            .collect::<Vec<_>>()
            .join(""),
    ];
    let mut lims: Vec<c_int> = (1..=64).collect();
    lims.extend([
        96, 128, 192, 256, 384, 512, 1024, 2048, 4000, 4095, 4096, 4097, 5000,
        8191, 8192, 8193, 1 << 14, 1 << 15, 1 << 16, 1 << 17, 1 << 18, 1 << 20,
    ]);
    let mut rng = Rng::new(0x5EED_0004);
    for _ in 0..60 {
        lims.push(1 + rng.below(1 << 16) as c_int);
    }
    for src in &srcs {
        for lim in &lims {
            diff_pload_mem(0, *lim, src);
        }
    }
    // and the thrown value really is the raw C literal `"out of memory"`
    // (a JS_TLITSTR pushed by js_outofmemory, jsrun.c:30-36), NOT an Error
    // object -- so there is no `"Error: "` prefix and no filename prefix.
    let p = libs();
    for l in [&p.c, &p.rs] {
        let mut seen = false;
        for lim in 1..40 {
            let a = pload_mem(l, 0, lim, "var s = 'x'");
            if a.rc != 0 {
                assert_eq!(a.msg, "out of memory", "{}: lim={lim}", l.name);
                assert_eq!(a.ty, "string", "{}: lim={lim}", l.name);
                seen = true;
                break;
            }
        }
        assert!(seen, "{}: memlimit never triggered in the lexer", l.name);
    }
}

/// Rows 273 / 294: an unterminated `/*` block comment. `lexcomment` returns -1
/// at EOF and `jsY_lexx` turns that into `"multi-line comment not terminated"`.
#[test]
fn t_block_comment_not_terminated() {
    for src in [
        "/*", "/**", "/***", "/* x", "/* x *", "/* x **", "/*\n", "/*\n\n\n",
        "1 /*", "1 /* x", "/*/", "/**/", "/***/", "/* * /", "/* / */", "/*/*/",
        "/*\r\n\r\n*", "var a = 1; /* unterminated", "/* a */ /* b",
        "/*\u{2028}\u{2029}", "/*\u{4e2d}",
    ] {
        diff_all(src);
    }
    // the reported line is the line the token STARTED on (J->lexline)
    for n in 0..6 {
        let src = format!("{}/* x", "\n".repeat(n));
        diff_compileonly(&src);
    }
}

/// Rows 274-278: every `lexnumber` rejection, plus row 276 (a `.` that is not
/// followed by a digit falls back to the `'.'` punctuation token).
#[test]
fn t_number_literal_errors() {
    let mut srcs: Vec<String> = vec![];
    for s in [
        // row 274: malformed hexadecimal number
        "0x", "0X", "0x;", "0xg", "0Xg", "0x.", "0x ", "0x\n", "0x+1", "0x-",
        "-0x", "0x0", "0xF", "0xdeadBEEF", "0x1g", "0xg1",
        // row 275: number with leading zero
        "01", "00", "08", "09", "0123", "0.5", "00.5", "01.5", "0e1", "0E1",
        "-01", "0_", "007",
        // row 276: '.' that is not a number
        ".", ".a", ". ", ".;", ".)", ".e5", "..", "...", "a.b", ".5", ".5e3",
        "1..2", "1.2.3", "(1).x",
        // row 277: missing exponent
        "1e", "1E", "1e+", "1e-", "1E+", "1E-", "1e;", "1e ", "1e\n", "1e+;",
        ".5e", "0.5e+", "1.e", "5.e-", "0e", "0e+", "12345e-",
        // row 278: number with letter suffix
        "1x", "3abc", "1$", "1_", "1e5x", "1.5f", "0.1L", "1\u{e9}", "1\u{4e2d}",
        "123abc456", "1n", "0b11", "0o17", "1if", "1in", "3 in [3]",
        // accepted forms for contrast
        "0", "1", "1.5", "1e5", "1E5", "1e+5", "1e-5", "5.", "5.5", "1e308",
        "1e309", "1e-400", "9007199254740993", "0.30000000000000004",
    ] {
        srcs.push(s.to_string());
        srcs.push(format!("print({s})"));
        srcs.push(format!("var v = {s}"));
        srcs.push(format!("x[{s}]"));
    }
    // randomised numeric-looking token soup
    let alphabet: Vec<char> = "0123456789.eE+-xXabcdefABCDEF_$ ".chars().collect();
    let mut rng = Rng::new(0x5EED_0005);
    for _ in 0..2500 {
        let n = 1 + rng.below(9) as usize;
        let s: String = (0..n)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
            .collect();
        srcs.push(s);
    }
    for s in &srcs {
        diff_quick(s);
        diff_dostring(0, s);
    }
    for s in srcs.iter().take(140) {
        diff_all(s);
    }
}

/// Rows 279-287: `lexescape` and `lexstring`.  Rows 280-285 return 1, which
/// `lexstring` turns into `"malformed escape sequence"`; row 279 errors inside
/// `lexescape` itself with `"unterminated escape sequence"`; row 286 is the
/// unterminated string literal.
#[test]
fn t_string_literal_errors() {
    let mut srcs: Vec<String> = vec![];
    for q in ['\'', '"'] {
        let o = if q == '\'' { '"' } else { '\'' };
        for body in [
            // row 279: EOF straight after the backslash (source ends)
            "a\\", "\\",
            // rows 280-283: `\uXXXX` with a bad digit in each position
            "\\uZ123", "\\u1Z23", "\\u12Z3", "\\u123Z", "\\uZZZZ", "\\u",
            "\\u0", "\\u00", "\\u000", "\\u 000", "\\u0 00",
            // rows 284-285: `\xXX`
            "\\x", "\\xZ", "\\x1", "\\x1Z", "\\xZ1", "\\x0", "\\x 1",
            // row 287 (contrast): valid escapes
            "\\u0041", "\\x41", "\\0", "\\\\", "\\n", "\\r", "\\t", "\\b",
            "\\f", "\\v", "\\'", "\\\"", "\\a", "\\1", "\\8", "\\/", "\\\n",
            // row 286: unterminated
            "a", "", "a\nb", "a\rb", "a\r\nb", "a\u{2028}b", "a\u{2029}b",
        ] {
            let full = format!("{q}{body}{q}");
            srcs.push(full.clone());
            srcs.push(format!("print({full})"));
            // unterminated variants (no closing quote)
            srcs.push(format!("{q}{body}"));
            srcs.push(format!("var s = {q}{body}"));
            // and with the other quote inside
            srcs.push(format!("{q}{body}{o}{q}"));
        }
    }
    // randomised escape soup inside a string literal
    let esc: Vec<&str> = vec![
        "\\u", "\\x", "\\n", "\\\\", "\\'", "\\0", "\\uZ", "\\x1", "\\u12",
        "\\u123", "\\uABCD", "\\x4Z", "a", "Z", "1", " ", "\\", "\\u0041",
    ];
    let mut rng = Rng::new(0x5EED_0006);
    for _ in 0..2500 {
        let n = 1 + rng.below(6) as usize;
        let body: String = (0..n)
            .map(|_| esc[rng.below(esc.len() as u32) as usize])
            .collect();
        srcs.push(format!("'{body}'"));
        srcs.push(format!("'{body}"));
    }
    for s in &srcs {
        diff_quick(s);
    }
    for s in srcs.iter().take(160) {
        diff_all(s);
    }
}

/// Row 288 is UNREACHABLE: `lexstring`'s `jsY_expect(J, q)` (jslex.c:449) can
/// only run after the `while (J->lexchar != q)` loop has exited, which happens
/// exactly when `J->lexchar == q`, so the expect always succeeds.  Row 291
/// (jslex.c:510) and row 309 (jslex.c:830) are unreachable for the same reason.
/// This test documents the invariant: every string / regexp / JSON string that
/// reaches the closing delimiter is accepted, and every one that does not
/// reports the *unterminated* diagnostic instead of `"expected '%c'"`.
#[test]
fn t_unreachable_closing_delimiter_expects() {
    let p = libs();
    for src in [
        "'a", "\"a", "'a\\", "/a", "/a\\", "/[a", "'a\nb'", "/a\nb/",
    ] {
        let a = pload(&p.c, 0, "t.js", src);
        assert!(a.rc != 0, "{src:?} should not compile");
        assert!(
            !a.msg.contains("expected '\''")
                && !a.msg.contains("expected '\"'")
                && !a.msg.contains("expected '/'"),
            "{src:?}: unexpectedly reached the unreachable jsY_expect: {}",
            a.msg
        );
        diff_all(src);
    }
}

/// Rows 289-293: `lexregexp`.
#[test]
fn t_regexp_literal_errors() {
    let mut srcs: Vec<String> = vec![];
    for body in [
        // rows 289 / 290: not terminated
        "a", "", "a\\", "\\", "[a", "[a\\", "a\nb", "a\\\nb", "[\n]",
        "a\rb", "a\r\nb", "a\u{2028}b",
        // accepted bodies
        "a/", "\\/", "[/]", "[abc]", "a\\\\", "(a)(b)",
    ] {
        srcs.push(format!("/{body}"));
        srcs.push(format!("/{body}/"));
        srcs.push(format!("var r = /{body}/"));
    }
    for flags in [
        // row 292: illegal flag
        "x", "X", "G", "I", "M", "1", "$", "_", "gx", "igm1", "y", "s", "u",
        // row 293: duplicated flag
        "gg", "ii", "mm", "gig", "gimg", "ggg", "gimgim", "mim",
        // accepted
        "", "g", "i", "m", "gi", "gm", "im", "gim", "mig", "mgi",
    ] {
        srcs.push(format!("/a/{flags}"));
        srcs.push(format!("var r = /abc/{flags}"));
        srcs.push(format!("print(/a/{flags}.source)"));
    }
    // division vs regexp context (isregexpcontext)
    for s in [
        "1/2", "a/b/c", "(1)/2/3", "x = y / z / w", "]/a/", ")/a/", "}/a/",
        "1/a/", "'s'/a/", "true/a/", "null/a/", "this/a/", "/a/g/b/",
        "var re = /a/; 1/2", "typeof/a/", "return/a/", "case/a/:",
    ] {
        srcs.push(s.to_string());
    }
    // randomised regexp-ish token soup
    let parts: Vec<&str> = vec![
        "a", "[", "]", "\\", "/", "(", ")", "\n", "*", "+", "?", "{", "}", "|",
        "^", "$", ".",
    ];
    let mut rng = Rng::new(0x5EED_0007);
    for _ in 0..2000 {
        let n = 1 + rng.below(7) as usize;
        let body: String = (0..n)
            .map(|_| parts[rng.below(parts.len() as u32) as usize])
            .collect();
        let fl: String = (0..rng.below(4))
            .map(|_| "gim xGZ".chars().nth(rng.below(7) as usize).unwrap())
            .collect();
        srcs.push(format!("/{body}/{fl}"));
    }
    for s in &srcs {
        diff_quick(s);
    }
    for s in srcs.iter().take(160) {
        diff_all(s);
    }
}

/// Rows 295-297: the end-of-file token, and the two "unexpected character"
/// diagnostics (`'%c'` for printable ASCII, `\\u%04X` otherwise).
#[test]
fn t_unexpected_character() {
    // every byte 0x01..0x7F on its own and after a token
    for b in 1u8..=0x7f {
        let c = b as char;
        for src in [
            format!("{c}"),
            format!("1{c}"),
            format!("{c}1"),
            format!("a {c} b"),
            format!("var x = {c}"),
        ] {
            diff_quick(&src);
        }
    }
    // non-ASCII runes: identifier parts vs. rejected ones
    for r in [
        '\u{80}', '\u{a0}', '\u{a1}', '\u{ab}', '\u{b5}', '\u{bf}', '\u{c0}',
        '\u{e9}', '\u{2022}', '\u{2028}', '\u{2029}', '\u{20ac}', '\u{3000}',
        '\u{4e2d}', '\u{feff}', '\u{fffd}', '\u{ffff}', '\u{10000}',
        '\u{1d400}', '\u{10ffff}',
    ] {
        for src in [
            format!("{r}"),
            format!("1{r}"),
            format!("a{r}"),
            format!("{r}a"),
            format!("var {r} = 1"),
            format!("'{r}'"),
            format!("/{r}/"),
        ] {
            diff_quick(&src);
            diff_dostring(0, &src);
        }
    }
    // rune decoded by jsY_unescape into a rejected character (row 297)
    for e in [
        "\\u0000", "\\u0001", "\\u0007", "\\u000B", "\\u001F", "\\u0020",
        "\\u007F", "\\u0080", "\\u00A0", "\\u2022", "\\u2028", "\\uFEFF",
        "\\uFFFF",
    ] {
        diff_quick(e);
        diff_quick(&format!("a{e}"));
        diff_quick(&format!("var {e}"));
    }
    // token 0 (row 295) is not an error
    for src in ["", " \t\n", "//x", "/*x*/", "\u{feff}", "\u{a0}"] {
        diff_all(src);
    }
}

/* ===================================================================== */
/*  jslex.c rows 298-322: the JSON lexer                                 */
/* ===================================================================== */

const N_JSON: *const c_char = cn!("JSON");
const N_PARSE: *const c_char = cn!("parse");

/// `JSON.parse(text)` through `js_pcall`.  `jsY_initlex` is called with the
/// literal filename `"JSON"` (json.c:162), so every `jsY_error` raised by
/// `jsY_lexjson` is prefixed `"JSON:<line>: "`.
fn json_parse(l: &Lib, flags: c_int, text: &str) -> Load {
    unsafe {
        out_clear();
        let j = new_state(l, flags);
        let cs = cstr(text);
        l.js_getglobal(j, N_JSON);
        l.js_getproperty(j, -1, N_PARSE);
        l.js_copy(j, -2); // this = JSON
        l.js_pushstring(j, cs.as_ptr());
        let rc = l.js_pcall(j, 1);
        let ty = from_c(l.js_typeof(j, -1));
        let msg = from_c(l.js_trystring(j, -1, ERRSTR));
        l.js_pop(j, 1);
        let top = l.js_gettop(j);
        l.js_freestate(j);
        Load {
            rc,
            ty,
            msg,
            top,
            out: out_take(),
        }
    }
}

fn diff_json(text: &str) {
    let p = libs();
    for flags in [0, JS_STRICT] {
        let a = json_parse(&p.c, flags, text);
        let b = json_parse(&p.rs, flags, text);
        assert_eq!(a, b, "JSON.parse divergence (flags={flags})\ntext: {text:?}");
    }
}

/// Rows 298-300: `lexjsonnumber`.
#[test]
fn t_json_number_errors() {
    let mut v: Vec<String> = vec![];
    for s in [
        // row 298: neither `0` nor `1`-`9` after the optional `-`
        "-", "-x", "-e", "-.5", "- 1", "--1", "-+1", "-\n", "-\"", "-]",
        // row 299: `.` not followed by a digit
        "1.", "0.", "-1.", "1.e5", "1.x", "1..", "12.", "1.]",
        // row 300: `e` not followed by a digit
        "1e", "1E", "1e+", "1e-", "1E+", "1E-", "0e", "1e.", "1ex", "1e]",
        "1.5e", "1.5e+", "-1.5E-",
        // accepted
        "0", "-0", "1", "-1", "1.5", "-1.5e3", "1e5", "1E5", "1e+5", "1e-5",
        "123456789", "0.0001", "1e308", "1e309", "1e-400",
        // leading zero is a JSON-specific accept-then-stop
        "01", "00", "-01", "0x10", "007",
    ] {
        v.push(s.to_string());
        v.push(format!("[{s}]"));
        v.push(format!("{{\"k\":{s}}}"));
        v.push(format!("[1,{s}]"));
        v.push(format!(" \n {s}"));
    }
    let alphabet: Vec<char> = "0123456789.eE+-x ".chars().collect();
    let mut rng = Rng::new(0x5EED_0008);
    for _ in 0..1800 {
        let n = 1 + rng.below(7) as usize;
        v.push(
            (0..n)
                .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
                .collect(),
        );
    }
    for s in &v {
        diff_json(s);
    }
}

/// Row 301: `lexjsonescape` rejects any escape character outside
/// `u " \ / b f n r t` with `"invalid escape sequence"`.
#[test]
fn t_json_invalid_escape_sequence() {
    let mut v: Vec<String> = vec![];
    for b in 1u8..=0x7f {
        let c = b as char;
        v.push(format!("\"\\{c}\""));
        v.push(format!("\"a\\{c}b\""));
    }
    for e in [
        "\\q", "\\x41", "\\v", "\\0", "\\'", "\\ ", "\\\n", "\\\t", "\\U0041",
        "\\\u{4e2d}", "\\",
    ] {
        v.push(format!("\"{e}\""));
        v.push(format!("[\"{e}\"]"));
    }
    // accepted escapes
    for e in [
        "\\\"", "\\\\", "\\/", "\\b", "\\f", "\\n", "\\r", "\\t", "\\u0041",
        "\\u00e9", "\\u4e2d", "\\uFFFF", "\\u0000",
    ] {
        v.push(format!("\"{e}\""));
    }
    for s in &v {
        diff_json(s);
    }
}

/// Rows 302-305 and 308: THE ASYMMETRY.  `lexjsonstring` (jslex.c:823-824)
/// calls `lexjsonescape(J)` and DISCARDS the return value, so a malformed JSON
/// `\uXXXX` raises NO error at all and lexing simply continues with the
/// characters that were not consumed.  The JS string path (jslex.c:442-443)
/// checks the identical return value and reports
/// `"malformed escape sequence"`.  Both halves are asserted, and the exact
/// resulting JSON string values are pinned so the asymmetry cannot silently
/// disappear.
#[test]
fn t_json_malformed_unicode_escape_is_silently_accepted() {
    let p = libs();
    // (json text, the string value JSON.parse must produce)
    let pinned: &[(&str, &str)] = &[
        (r#""\uZZZZ""#, "ZZZZ"),
        (r#""\u0ZZZ""#, "ZZZ"),
        (r#""\u00ZZ""#, "ZZ"),
        (r#""\u000Z""#, "Z"),
        (r#""\u""#, ""),
        (r#""\u0""#, ""),
        (r#""\u00""#, ""),
        (r#""\u000""#, ""),
        (r#""a\uZb""#, "aZb"),
        (r#""\u123g4""#, "g4"),
    ];
    for (text, want) in pinned {
        for flags in [0, JS_STRICT] {
            let a = json_parse(&p.c, flags, text);
            let b = json_parse(&p.rs, flags, text);
            assert_eq!(a, b, "JSON.parse divergence for {text:?}");
            assert_eq!(a.rc, 0, "JSON.parse({text:?}) must NOT raise: {}", a.msg);
            assert_eq!(
                a.msg, *want,
                "JSON.parse({text:?}) value (return value of lexjsonescape is discarded)"
            );
        }
    }
    // ... while the JS string lexer DOES report the same malformed escapes
    for js in [
        "'\\uZZZZ'", "'\\u0ZZZ'", "'\\u00ZZ'", "'\\u000Z'", "'\\u'", "'\\u0'",
        "'\\u00'", "'\\u000'", "'a\\uZb'",
    ] {
        let a = pload(&p.c, 0, "test.js", js);
        assert_eq!(a.rc, 1, "{js:?} must be rejected");
        assert_eq!(
            a.msg, "SyntaxError: test.js:1: malformed escape sequence",
            "{js:?}"
        );
        diff_all(js);
    }
    // randomised: 0..4 characters after `\u`
    let alphabet: Vec<char> = "0123456789abcdefZzg _-".chars().collect();
    let mut rng = Rng::new(0x5EED_0009);
    for _ in 0..900 {
        let n = rng.below(5) as usize;
        let tail: String = (0..n)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
            .collect();
        diff_json(&format!("\"\\u{tail}\""));
        diff_json(&format!("[\"x\\u{tail}y\"]"));
        diff_quick(&format!("'\\u{tail}'"));
    }
}

/// Rows 306 / 307 / 309: `lexjsonstring`.  Note that `jsY_next` folds every
/// line terminator to `'\n'` and bumps `J->line`, so a raw newline inside a
/// JSON string is reported as an invalid control character on the FOLLOWING
/// line.
#[test]
fn t_json_string_errors() {
    let mut v: Vec<String> = vec![];
    // row 306: EOF before the closing quote
    for s in [
        "\"", "\"a", "\"abc", "\"\\", "\"\\u0041", "[\"a", "{\"a", "{\"a\":\"b",
    ] {
        v.push(s.to_string());
    }
    // row 307: raw control character
    for b in 1u8..32 {
        v.push(format!("\"a{}b\"", b as char));
        v.push(format!("[\"{}\"]", b as char));
    }
    v.push("\"a\nb\"".into());
    v.push("\"a\rb\"".into());
    v.push("\"a\r\nb\"".into());
    v.push("\n\n\"a\nb\"".into());
    v.push("\"a\u{2028}b\"".into());
    v.push("\"a\u{2029}b\"".into());
    v.push("\"a\tb\"".into());
    // accepted
    for s in [
        "\"\"", "\"a\"", "\"\u{e9}\"", "\"\u{4e2d}\"", "\" \"", "\"\u{7f}\"",
        "\"\u{a0}\"",
    ] {
        v.push(s.to_string());
    }
    for s in &v {
        diff_json(s);
    }
}

/// Rows 310-319 (and therefore row 265, `jsY_expect`): the per-character
/// `true` / `false` / `null` keyword checks, each naming a different expected
/// character in `"expected '%c'"`.
#[test]
fn t_json_keyword_expects() {
    let mut v: Vec<String> = vec![];
    for kw in ["false", "null", "true"] {
        for cut in 1..=kw.len() {
            let prefix = &kw[..cut];
            v.push(prefix.to_string()); // EOF right there
            for bad in ["x", "0", " ", "\n", "\"", ",", "]", "\\", "\u{4e2d}"] {
                v.push(format!("{prefix}{bad}"));
            }
            // wrong character in the middle
            if cut < kw.len() {
                let mut s = kw.to_string();
                s.replace_range(cut..cut + 1, "Q");
                v.push(s);
            }
        }
        v.push(kw.to_string());
        v.push(format!("[{kw}]"));
        v.push(format!("{{\"k\":{kw}}}"));
        v.push(format!("{kw}{kw}"));
        v.push(format!("{}", kw.to_uppercase()));
    }
    // truncated keywords inside containers, so the error line/column vary
    for s in ["[fals]", "[nul]", "[tru]", "[f]", "[n]", "[t]", "\n\n[nul]"] {
        v.push(s.to_string());
    }
    for s in &v {
        diff_json(s);
    }
}

/// Rows 320-322: the JSON end-of-file token and the two JSON
/// "unexpected character" diagnostics.
#[test]
fn t_json_unexpected_character_and_eof() {
    let mut v: Vec<String> = vec![];
    // row 320: EOF -> token 0 -> json.c reports "JSON: unexpected token: (end-of-file)"
    for s in ["", " ", "\t", "\n", "\r", "\r\n", "\u{a0}", "\u{feff}", "  \n  "] {
        v.push(s.to_string());
    }
    // row 321: printable ASCII with no JSON rule
    for b in 1u8..=0x7f {
        v.push(format!("{}", b as char));
        v.push(format!("[{}]", b as char));
        v.push(format!("1 {}", b as char));
    }
    // row 322: outside 0x20..0x7E
    for r in [
        '\u{80}', '\u{a1}', '\u{e9}', '\u{2022}', '\u{2028}', '\u{2029}',
        '\u{4e2d}', '\u{ffff}', '\u{10ffff}', '\u{7f}',
    ] {
        v.push(format!("{r}"));
        v.push(format!("[{r}]"));
    }
    // structural errors that go through jsonexpect / jsonvalue instead
    for s in [
        "[", "]", "{", "}", "[1", "{\"a\"", "{\"a\":", "{\"a\":1", "[1,]",
        "{\"a\":1,}", "{a:1}", "[1 2]", "1 2", "'a'", "+1", "NaN", "Infinity",
        "undefined", "[,]", "{,}", "{:1}", "{1:2}",
    ] {
        v.push(s.to_string());
    }
    let mut rng = Rng::new(0x5EED_000A);
    let toks = [
        "[", "]", "{", "}", ",", ":", "1", "\"a\"", "true", "false", "null",
        "-", ".", "e", " ", "\n", "x", "'", "+",
    ];
    for _ in 0..2000 {
        let n = 1 + rng.below(6) as usize;
        v.push(
            (0..n)
                .map(|_| toks[rng.below(toks.len() as u32) as usize])
                .collect::<Vec<_>>()
                .join(""),
        );
    }
    for s in &v {
        diff_json(s);
    }
}

/// The `#if 0` block at jslex.c:263-337 (`lexinteger`, `lexfraction`,
/// `lexexponent` and the first `lexnumber`) is NOT compiled in and therefore
/// NOT reachable; its `"malformed number"` diagnostic can never be produced.
/// This test pins that: the live `lexnumber` (jslex.c:341) reports
/// `"number with leading zero"` / `"missing exponent"` /
/// `"number with letter suffix"` and never `"malformed number"`.
#[test]
fn t_dead_lexnumber_block_is_unreachable() {
    let p = libs();
    for src in [
        "01", "1e", "1x", "0x", ".", "1..2", "0.5e", "08", "1e+", "0xg",
    ] {
        for l in [&p.c, &p.rs] {
            let r = pload(l, 0, "t.js", src);
            assert!(
                !r.msg.contains("malformed number"),
                "{}: {src:?} produced the dead #if 0 diagnostic: {}",
                l.name,
                r.msg
            );
        }
    }
}

/* ===================================================================== */
/*  jsparse.c rows 326-343: the 17 INCREC sites (JS_ASTLIMIT = 400)      */
/* ===================================================================== */

/// Depths swept for the recursion-limit tests.  Different constructs hold a
/// different number of `INCREC`s per nesting level (1 for the left-recursive
/// binary chains, 2 for parenthesised expressions, 3 for `f(...)` / `a[...]`),
/// so the sweep has to cover the 400 / 2 and 400 / 3 transition zones too.
fn rec_depths() -> Vec<usize> {
    let mut v: Vec<usize> = vec![0, 1, 2, 3, 4, 5, 10, 50, 90];
    v.extend(126..=142);
    v.extend(190..=206);
    v.extend(390..=406);
    v
}

/// `(name, generator)` for every distinct nesting construct.  Between them they
/// reach all 17 `INCREC` call sites in jsparse.c.
#[allow(clippy::type_complexity)]
fn rec_constructs() -> Vec<(&'static str, fn(usize) -> String)> {
    fn parens(n: usize) -> String {
        format!("{}1{}", "(".repeat(n), ")".repeat(n))
    }
    fn array(n: usize) -> String {
        format!("{}1{}", "[".repeat(n), "]".repeat(n))
    }
    fn object(n: usize) -> String {
        format!("({}1{})", "{a:".repeat(n), "}".repeat(n))
    }
    fn call(n: usize) -> String {
        format!("f{}1{}", "(".repeat(n), ")".repeat(n))
    }
    fn member(n: usize) -> String {
        format!("a{}", ".b".repeat(n))
    }
    fn newmember(n: usize) -> String {
        format!("new a{}", ".b".repeat(n))
    }
    fn index(n: usize) -> String {
        format!("a{}", "[1]".repeat(n))
    }
    fn index_nest(n: usize) -> String {
        format!("a{}1{}", "[".repeat(n), "]".repeat(n))
    }
    fn callchain(n: usize) -> String {
        format!("f{}", "()".repeat(n))
    }
    fn unary_not(n: usize) -> String {
        format!("{}1", "!".repeat(n))
    }
    fn unary_mixed(n: usize) -> String {
        let ops = ["!", "~", "-", "+", "void ", "typeof "];
        let mut s = String::new();
        for i in 0..n {
            s.push_str(ops[i % ops.len()]);
        }
        s.push('1');
        s
    }
    fn mul(n: usize) -> String {
        format!("1{}", "*1".repeat(n))
    }
    fn add(n: usize) -> String {
        format!("1{}", "+1".repeat(n))
    }
    fn shift(n: usize) -> String {
        format!("1{}", "<<1".repeat(n))
    }
    fn rel(n: usize) -> String {
        format!("1{}", "<1".repeat(n))
    }
    fn eq(n: usize) -> String {
        format!("1{}", "==1".repeat(n))
    }
    fn bitand(n: usize) -> String {
        format!("1{}", "&1".repeat(n))
    }
    fn bitxor(n: usize) -> String {
        format!("1{}", "^1".repeat(n))
    }
    fn bitor(n: usize) -> String {
        format!("1{}", "|1".repeat(n))
    }
    fn logand(n: usize) -> String {
        format!("1{}", "&&1".repeat(n))
    }
    fn logor(n: usize) -> String {
        format!("1{}", "||1".repeat(n))
    }
    fn cond(n: usize) -> String {
        format!("{}1", "1?1:".repeat(n))
    }
    fn assign(n: usize) -> String {
        format!("{}1", "a=".repeat(n))
    }
    fn comma(n: usize) -> String {
        format!("1{}", ",1".repeat(n))
    }
    fn block(n: usize) -> String {
        format!("{}{}", "{".repeat(n), "}".repeat(n))
    }
    fn ifstm(n: usize) -> String {
        format!("{};", "if(1)".repeat(n))
    }
    fn whilestm(n: usize) -> String {
        format!("{};", "while(0)".repeat(n))
    }
    fn forstm(n: usize) -> String {
        format!("{};", "for(;0;)".repeat(n))
    }
    fn forin(n: usize) -> String {
        format!("{};", "for(k in o)".repeat(n))
    }
    fn dostm(n: usize) -> String {
        format!("{};{}", "do ".repeat(n), " while(0)".repeat(n))
    }
    fn label(n: usize) -> String {
        format!("{};", (0..n).map(|i| format!("L{i}:")).collect::<String>())
    }
    fn trystm(n: usize) -> String {
        format!("{};{}", "try{".repeat(n), "}catch(e){}".repeat(n))
    }
    fn withstm(n: usize) -> String {
        format!("{};", "with(o)".repeat(n))
    }
    fn switchstm(n: usize) -> String {
        format!("{};{}", "switch(1){case 1:".repeat(n), "}".repeat(n))
    }
    fn funbody(n: usize) -> String {
        format!("x={}1{}", "function(){return ".repeat(n), "}".repeat(n))
    }
    vec![
        ("parens", parens),
        ("array", array),
        ("object", object),
        ("call", call),
        ("member", member),
        ("newmember", newmember),
        ("index", index),
        ("index_nest", index_nest),
        ("callchain", callchain),
        ("unary_not", unary_not),
        ("unary_mixed", unary_mixed),
        ("mul", mul),
        ("add", add),
        ("shift", shift),
        ("rel", rel),
        ("eq", eq),
        ("bitand", bitand),
        ("bitxor", bitxor),
        ("bitor", bitor),
        ("logand", logand),
        ("logor", logor),
        ("cond", cond),
        ("assign", assign),
        ("comma", comma),
        ("block", block),
        ("if", ifstm),
        ("while", whilestm),
        ("for", forstm),
        ("forin", forin),
        ("do", dostm),
        ("label", label),
        ("try", trystm),
        ("with", withstm),
        ("switch", switchstm),
        ("funbody", funbody),
    ]
}

/// Assert C == RUST at every swept depth, and that the construct really does
/// cross `JS_ASTLIMIT` somewhere inside the sweep (otherwise the test would be
/// vacuous).  Also pins the diagnostic text of the crossing.
fn run_rec_group(from: usize, to: usize) {
    let p = libs();
    let all = rec_constructs();
    for (name, gen) in &all[from..to] {
        let mut ok = 0usize;
        let mut over = 0usize;
        let mut first_over: Option<(usize, String)> = None;
        for d in rec_depths() {
            let src = gen(d);
            for flags in [0, JS_STRICT] {
                let a = pload(&p.c, flags, "test.js", &src);
                let b = pload(&p.rs, flags, "test.js", &src);
                assert_eq!(
                    a, b,
                    "recursion-limit divergence: {name} depth={d} flags={flags}"
                );
            }
            let a = pload(&p.c, 0, "test.js", &src);
            if a.msg.contains("too much recursion") {
                over += 1;
                if first_over.is_none() {
                    first_over = Some((d, a.msg.clone()));
                }
            } else if a.rc == 0 {
                ok += 1;
            }
        }
        assert!(
            ok > 0,
            "{name}: never parsed successfully at any swept depth"
        );
        assert!(
            over > 0,
            "{name}: never hit JS_ASTLIMIT (\"too much recursion\") in {:?}",
            rec_depths()
        );
        let (_d, msg) = first_over.unwrap();
        assert_eq!(
            msg, "SyntaxError: test.js:1: too much recursion",
            "{name}: wrong recursion diagnostic"
        );
    }
}

fn body_t_recursion_limit_a() {
    run_rec_group(0, 12);
}
fn body_t_recursion_limit_b() {
    run_rec_group(12, 24);
}
fn body_t_recursion_limit_c() {
    run_rec_group(24, 35);
}

/// Rows 326-343: `INCREC` / `JS_ASTLIMIT` (400).  Needs a big native stack
/// because both parsers recurse once per nesting level.
#[test]
fn t_recursion_limit_expressions() {
    with_big_stack(body_t_recursion_limit_a);
}

#[test]
fn t_recursion_limit_operators() {
    with_big_stack(body_t_recursion_limit_b);
}

#[test]
fn t_recursion_limit_statements() {
    with_big_stack(body_t_recursion_limit_c);
}

/// The exact depth at which each construct starts failing must be identical in
/// the two libraries; assert it as a single number rather than only pointwise.
fn body_t_recursion_limit_threshold() {
    let p = libs();
    for (name, gen) in rec_constructs() {
        for flags in [0, JS_STRICT] {
            let mut ca: Option<usize> = None;
            let mut cb: Option<usize> = None;
            for d in 0..=420usize {
                if ca.is_some() && cb.is_some() {
                    break;
                }
                let src = gen(d);
                if ca.is_none()
                    && pload(&p.c, flags, "t.js", &src)
                        .msg
                        .contains("too much recursion")
                {
                    ca = Some(d);
                }
                if cb.is_none()
                    && pload(&p.rs, flags, "t.js", &src)
                        .msg
                        .contains("too much recursion")
                {
                    cb = Some(d);
                }
            }
            assert_eq!(ca, cb, "{name}: JS_ASTLIMIT threshold (flags={flags})");
            assert!(ca.is_some(), "{name}: no threshold found below depth 420");
        }
    }
}

#[test]
fn t_recursion_limit_threshold() {
    with_big_stack(body_t_recursion_limit_threshold);
}

/* ===================================================================== */
/*  jsparse.c rows 344-406: jsP_expect and the other parse rejections    */
/* ===================================================================== */

/// Rows 344-387 and 389-404: one input per `jsP_expect` call site (each names a
/// different expected token in `"unexpected token: %s (expected %s)"`) plus the
/// hand-written parse diagnostics.
#[test]
fn t_parse_expect_sites() {
    let cases: &[(&str, &str)] = &[
        /* row 345 jsparse.c:232 getter shorthand, expected '(' */
        ("({get x 1})", "'('"),
        ("({get x})", "'('"),
        ("({get 1 2})", "'('"),
        /* row 346 jsparse.c:233 getter param list not empty, expected ')' */
        ("({get x(a){}})", "')'"),
        ("({get x(,){}})", "')'"),
        /* row 347 jsparse.c:239 setter shorthand, expected '(' */
        ("({set x 1})", "'('"),
        ("({set x})", "'('"),
        /* row 348 jsparse.c:241 setter param list, expected ')' */
        ("({set x(a,b){}})", "')'"),
        ("({set x(a b){}})", "')'"),
        /* row 349 jsparse.c:247 property name not followed by ':' */
        ("({a 1})", "':'"),
        ("({a})", "':'"),
        ("({1 2})", "':'"),
        ("({'a' 1})", "':'"),
        ("({if 1})", "':'"),
        /* row 350 jsparse.c:284 fundec name, expected '(' */
        ("function f 1{}", "'('"),
        ("function f{}", "'('"),
        /* row 351 jsparse.c:286 fundec parameters, expected ')' */
        ("function f(a 1){}", "')'"),
        ("function f(a,b 1){}", "')'"),
        /* row 352 jsparse.c:295 funstm name, expected '(' */
        ("if(1) function f 1{}", "'('"),
        ("if(1) function f{}", "'('"),
        /* row 353 jsparse.c:297 funstm parameters, expected ')' */
        ("if(1) function f(a 1){}", "')'"),
        /* row 354 jsparse.c:307 funexp, expected '(' */
        ("(function 1(){})", "'('"),
        ("(function{})", "'('"),
        ("(function f 1(){})", "'('"),
        /* row 355 jsparse.c:309 funexp parameters, expected ')' */
        ("(function(a 1){})", "')'"),
        ("(function f(a,b 1){})", "')'"),
        /* row 356 jsparse.c:349 object literal, expected '}' */
        ("({a:1 2})", "'}'"),
        ("({a:1,b:2 3})", "'}'"),
        /* row 357 jsparse.c:354 array literal, expected ']' */
        ("[1 2]", "']'"),
        ("[1,2 3]", "']'"),
        /* row 358 jsparse.c:359 parenthesised expression, expected ')' */
        ("(1 2)", "')'"),
        ("(1;)", "')'"),
        /* row 359 jsparse.c:387 new arguments, expected ')' */
        ("new a(1 2)", "')'"),
        ("new a(1,2 3)", "')'"),
        /* row 360 jsparse.c:408 memberexp index, expected ']' */
        ("new a[1 2]", "']'"),
        ("new a[1;]", "']'"),
        /* row 361 jsparse.c:422 callexp index, expected ']' */
        ("a[1 2]", "']'"),
        ("a[1][2 3]", "']'"),
        /* row 362 jsparse.c:423 callexp arguments, expected ')' */
        ("a(1 2)", "')'"),
        ("a()(1 2)", "')'"),
        /* row 363 jsparse.c:608 conditional, expected ':' */
        ("1?2 3", "':'"),
        ("1?2;3", "':'"),
        /* row 364 jsparse.c:689 case clause, expected ':' */
        ("switch(1){case 1 2:}", "':'"),
        ("switch(1){case 1}", "':'"),
        /* row 365 jsparse.c:695 default clause, expected ':' */
        ("switch(1){default 1:}", "':'"),
        ("switch(1){default}", "':'"),
        /* row 366 jsparse.c:718 block, expected '{' */
        ("try 1 {}", "'{'"),
        ("try{}catch(e) 1", "'{'"),
        ("try{}finally 1", "'{'"),
        /* row 367 jsparse.c:720 block, expected '}' */
        ("{case 1:}", "'}'"),
        ("{default:}", "'}'"),
        ("{1;case 2:}", "'}'"),
        ("while(1){case 1:}", "'}'"),
        /* row 368 jsparse.c:729 for-header clause */
        ("for(;1 2;);", "';'"),
        ("for(;;1 2);", "')'"),
        ("for(var i;1 2;);", "';'"),
        ("for(1;2;3 4);", "')'"),
        /* row 369 jsparse.c:736 for, expected '(' */
        ("for 1(;;);", "'('"),
        ("for;;", "'('"),
        /* row 370 jsparse.c:747 for-var-in, expected ')' */
        ("for(var x in y 1);", "')'"),
        ("for(var x in y;);", "')'"),
        /* row 371 jsparse.c:766 for-in, expected ')' */
        ("for(x in y 1);", "')'"),
        ("for(x in y;);", "')'"),
        /* row 372 jsparse.c:797 if, expected '(' */
        ("if 1(2);", "'('"),
        ("if;", "'('"),
        /* row 373 jsparse.c:799 if condition, expected ')' */
        ("if(1 2);", "')'"),
        ("if(1;);", "')'"),
        /* row 374 jsparse.c:810 do, expected 'while' */
        ("do; 1", "'while'"),
        ("do;", "'while'"),
        ("do{}else{}", "'while'"),
        /* row 375 jsparse.c:811 do-while, expected '(' */
        ("do; while 1(0)", "'('"),
        ("do; while", "'('"),
        /* row 376 jsparse.c:813 do-while condition, expected ')' */
        ("do; while(0 1)", "')'"),
        ("do; while(0;", "')'"),
        /* row 377 jsparse.c:819 while, expected '(' */
        ("while 1(0);", "'('"),
        ("while;", "'('"),
        /* row 378 jsparse.c:821 while condition, expected ')' */
        ("while(0 1);", "')'"),
        ("while(0;);", "')'"),
        /* row 379 jsparse.c:852 with, expected '(' */
        ("with 1({});", "'('"),
        ("with;", "'('"),
        /* row 380 jsparse.c:854 with object, expected ')' */
        ("with({} 1);", "')'"),
        ("with({};);", "')'"),
        /* row 381 jsparse.c:860 switch, expected '(' */
        ("switch 1(1){}", "'('"),
        ("switch{}", "'('"),
        /* row 382 jsparse.c:862 switch discriminant, expected ')' */
        ("switch(1 2){}", "')'"),
        ("switch(1;){}", "')'"),
        /* row 383 jsparse.c:863 switch head, expected '{' */
        ("switch(1) 2{}", "'{'"),
        ("switch(1);", "'{'"),
        /* row 385 jsparse.c:879 catch, expected '(' */
        ("try{}catch 1(e){}", "'('"),
        ("try{}catch{}", "'('"),
        /* row 386 jsparse.c:881 catch parameter, expected ')' */
        ("try{}catch(e 1){}", "')'"),
        ("try{}catch(e,f){}", "')'"),
        /* row 387 jsparse.c:949 function body, expected '{' */
        ("function f() 1{}", "'{'"),
        ("function f();", "'{'"),
        ("(function() 1{})", "'{'"),
        ("({get x() 1{}})", "'{'"),
        ("({set x(a) 1{}})", "'{'"),
    ];
    let p = libs();
    for (src, expected) in cases {
        // the exact "expected <tok>" text is part of the observable message
        let a = pload(&p.c, 0, "test.js", src);
        assert_eq!(a.rc, 1, "{src:?} should be a syntax error");
        assert!(
            a.msg.starts_with("SyntaxError: test.js:1: unexpected token: ")
                && a.msg.ends_with(&format!("(expected {expected})")),
            "{src:?}: message {:?} does not name (expected {expected})",
            a.msg
        );
        diff_all(src);
    }
}

/// Rows 384 and 388 are UNREACHABLE.
///
///   * jsparse.c:865 (`switch` body `jsP_expect(J, '}')`) runs after
///     `caselist`, whose only exits are `J->lookahead == '}'`, so the expect
///     always succeeds.
///   * jsparse.c:951 (`funbody`'s `jsP_expect(J, '}')`) runs after
///     `script(J, '}')`, whose only exit is `J->lookahead == '}'`.
///
/// Every input that runs off the end of a switch body / function body therefore
/// reports a diagnostic from *inside* the body instead.  Pinned here so a
/// future change that makes the expects reachable is noticed.
#[test]
fn t_unreachable_switch_and_funbody_expects() {
    let p = libs();
    for src in [
        "switch(1){case 1:", "switch(1){", "switch(1){default:",
        "switch(1){case 1:1", "function f(){", "function f(){1",
        "function f(){case 1:", "(function(){", "(function(){var a",
        "function f(){function g(){", "switch(1){case 1:switch(2){",
    ] {
        for l in [&p.c, &p.rs] {
            let r = pload(l, 0, "t.js", src);
            assert_eq!(r.rc, 1, "{}: {src:?}", l.name);
            assert!(
                !r.msg.ends_with("(expected '}')"),
                "{}: {src:?} reached an unreachable jsP_expect: {}",
                l.name,
                r.msg
            );
        }
        diff_all(src);
    }
}

/// Row 389 (`semicolon`), row 390 (`identifier`), row 391 (`identifieropt`
/// returning NULL) and row 392 (`identifiername`).
#[test]
fn t_parse_identifier_and_semicolon() {
    let mut srcs: Vec<String> = vec![];
    // row 389: no ';', no newline, lookahead is neither '}' nor EOF
    for s in [
        "1 2", "var a b", "a=1 b=2", "return 1 2", "throw 1 2", "break 1",
        "continue 1", "debugger 1", "do; while(0) 1", "var a=1 var b=2",
        // accepted by ASI
        "1\n2", "a=1\nb=2", "{1 }", "1", "var a=1\nvar b=2", "1;2",
        "return\n1", "throw\n1", "a\n++b",
    ] {
        srcs.push(s.to_string());
    }
    // row 390: binding identifier required
    for s in [
        "var 1", "var 'a'", "var =1", "var", "var ;", "function 1(){}",
        "function (){}", "try{}catch(1){}", "try{}catch(){}",
        "({set x(1){}})", "({set x(){}})", "for(var 1 in x);",
        "function f(1){}", "function f(a,1){}", "var a,1",
    ] {
        srcs.push(s.to_string());
    }
    // row 391: optional identifier absent (no error)
    for s in [
        "(function(){})", "while(1){break}", "while(1){continue}",
        "while(1)break;", "for(;;){continue;}", "(function f(){})",
        "while(1){break\n}", "L:while(1){break L}",
    ] {
        srcs.push(s.to_string());
    }
    // row 392: property name must be an identifier or a keyword
    for s in [
        "a.1", "a.'x'", "a.+", "a.", "a.(", "a.)", "a.;", "a..b", "a.[0]",
        // keywords ARE accepted after '.'
        "a.if", "a.var", "a.function", "a.true", "a.null", "a.in", "a.new",
        "a.delete", "a.typeof", "a.class", "({if:1})", "({var:1})",
        "({true:1,null:2})",
    ] {
        srcs.push(s.to_string());
    }
    for s in &srcs {
        diff_all(s);
    }
}

/// Rows 393-401 and 406: the non-error "empty" paths of the parser
/// (`arrayliteral`, `objectliteral`, `parameters`, `arguments`,
/// `statementlist`, `caselist`, `forexpression`, `script`), plus rows 396, 399,
/// 402, 403 and 404 (the hand-written `jsP_error` diagnostics).
#[test]
fn t_parse_empty_paths_and_diagnostics() {
    let mut srcs: Vec<String> = vec![];
    // 393/394/395/397/398/400/401/406
    for s in [
        "[]", "[,]", "[1,]", "[,1]", "[1,,2]", "({})", "({a:1,})",
        "function f(){}", "(function(){})", "f()", "new f()", "new f",
        "{}", "{;}", "switch(1){}", "switch(1){case 1:}",
        "switch(1){default:}", "switch(1){case 1:default:}",
        "for(;;)break;", "for(;;);", "for(1;;)break;", "for(;1;)break;",
        "for(;;1)break;", "for(var i=0;;)break;", "", " ", ";",
        "function f(){ }", "(function(){ })", "({get a(){}})",
        "({set a(v){}})",
    ] {
        srcs.push(s.to_string());
    }
    // row 396: cannot start a primary expression
    for s in [
        ")", "]", "}", ",", "*", "/=", "%", "&", "|", "^", "<", ">", "=", ":",
        "?", "1+", "1*", "a=", "a&&", "a||", "a?", "(", "[", "a[", "f(",
        "in", "instanceof", "else", "case", "default", "catch", "finally",
        "1+*2", "var a=,", "[,,",
    ] {
        srcs.push(s.to_string());
    }
    // row 399: neither 'case' nor 'default' inside a switch body
    for s in [
        "switch(1){1:}", "switch(1){x}", "switch(1){;}", "switch(1){var a;}",
        "switch(1){case 1:break;1:}", "switch(1){else:}",
    ] {
        srcs.push(s.to_string());
    }
    // row 402: for-var-statement
    for s in [
        "for(var i)", "for(var i=1)", "for(var i,j)", "for(var i 1;;);",
        "for(var i:);", "for(var i of x);",
    ] {
        srcs.push(s.to_string());
    }
    // row 403: for-statement
    for s in [
        "for(i)", "for(i=1)", "for(1)", "for(i 1;;);", "for(i:);",
        "for(i of x);", "for(a,b)",
    ] {
        srcs.push(s.to_string());
    }
    // row 404: try with neither catch nor finally
    for s in [
        "try{}", "try{}1", "try{};", "try{}else{}", "try{}\n1",
        "try{}catch(e){}", "try{}finally{}", "try{}catch(e){}finally{}",
    ] {
        srcs.push(s.to_string());
    }
    for s in &srcs {
        diff_all(s);
    }
}

/// Rows 324 / 405: `jsP_warning`.  A `function` in statement position emits
/// `"%s:%d: warning: function statements are not standard"` through
/// `js_report` and then keeps parsing (rewritten as `var X = function X(){}`).
/// The filename and the line of the warning are compared, so this also covers
/// the `jsP_warning` prefix separately from `jsP_error`.
#[test]
fn t_function_statement_warning() {
    let p = libs();
    let srcs = [
        "if(1) function f(){}",
        "if(0) function f(){} else function g(){}",
        "while(0) function f(){}",
        "{ function f(){} }",
        "L: function f(){}",
        "for(;0;) function f(){}",
        "do function f(){} while(0)",
        "try{ function f(){} }catch(e){}",
        "switch(1){case 1: function f(){}}",
        "if(1) function f(){}\nif(1) function g(){}",
        "\n\n\nif(1) function f(){}",
        // NOT a statement: these are declarations / expressions, no warning
        "function f(){}",
        "(function f(){})",
        "var f = function(){}",
        "function f(){ function g(){} }",
    ];
    for src in srcs {
        diff_all(src);
    }
    // the warning text, filename and line must be exact
    let a = pload(&p.c, 0, "warn.js", "\n\nif(1) function f(){}");
    assert_eq!(a.rc, 0);
    assert_eq!(
        a.out, "[report] warn.js:3: warning: function statements are not standard\n",
        "jsP_warning prefix"
    );
    let b = pload(&p.rs, 0, "warn.js", "\n\nif(1) function f(){}");
    assert_eq!(a, b);
    // through js_dostring the filename is "[string]"
    let (rc, out) = dostring(&p.c, 0, "if(1) function f(){}");
    assert_eq!(rc, 0);
    assert!(
        out.contains("[string]:1: warning: function statements are not standard"),
        "{out:?}"
    );
}

/// Row 325: `jsP_newnode` allocates every AST node with `js_malloc`, so a
/// `js_setlimit` budget makes the parser throw the raw literal
/// `"out of memory"`.  Swept so the failure lands at many different nodes.
#[test]
fn t_astnode_allocation_out_of_memory() {
    let srcs = [
        "1".to_string(),
        "1+2*3-4/5".to_string(),
        "function f(a,b){ return a+b } f(1,2)".to_string(),
        "var o={a:1,b:[1,2,3],c:{d:4}}".to_string(),
        format!("1{}", "+1".repeat(200)),
        format!("{}1{}", "(".repeat(80), ")".repeat(80)),
        (0..120).map(|i| format!("var v{i}={i};")).collect::<Vec<_>>().join(""),
        "switch(1){case 1:break;case 2:break;default:}".to_string(),
        "try{throw 1}catch(e){}finally{}".to_string(),
        "for(var i=0;i<10;++i){ if(i) continue; else break; }".to_string(),
    ];
    let mut lims: Vec<c_int> = (1..=48).collect();
    lims.extend([
        64, 96, 128, 256, 512, 1024, 2048, 4096, 8192, 1 << 14, 1 << 16, 1 << 18,
    ]);
    let mut rng = Rng::new(0x5EED_000B);
    for _ in 0..80 {
        lims.push(1 + rng.below(1 << 14) as c_int);
    }
    for src in &srcs {
        for lim in &lims {
            diff_pload_mem(0, *lim, src);
            diff_pload_mem(JS_STRICT, *lim, src);
        }
    }
}

/// Row 407: `toint32` during constant folding returns 0 for NaN, +-Inf and 0
/// (including -0.0).  Only observable through the folded value.
#[test]
fn t_constfold_toint32() {
    let mut srcs: Vec<String> = vec![];
    let operands = [
        "0", "-0", "0.0", "-0.0", "1/0", "-1/0", "0/0", "1e400", "-1e400",
        "1", "-1", "2147483647", "2147483648", "-2147483648", "-2147483649",
        "4294967295", "4294967296", "1.9", "-1.9", "1e21", "1e-21",
    ];
    for a in operands {
        srcs.push(format!("print(~({a}))"));
        srcs.push(format!("print(-({a}))"));
        srcs.push(format!("print(+({a}))"));
        for b in ["0", "1", "31", "32", "0/0", "1/0", "-0"] {
            srcs.push(format!("print(({a})<<({b}))"));
            srcs.push(format!("print(({a})>>({b}))"));
            srcs.push(format!("print(({a})>>>({b}))"));
            srcs.push(format!("print(({a})&({b}))"));
            srcs.push(format!("print(({a})^({b}))"));
            srcs.push(format!("print(({a})|({b}))"));
        }
        // the other folded operators
        for op in ["*", "/", "%", "+", "-"] {
            srcs.push(format!("print(({a}){op}(3))"));
        }
    }
    for s in &srcs {
        diff_dostring(0, s);
        diff_dostring(JS_STRICT, s);
    }
    for s in srcs.iter().take(120) {
        diff_all(s);
    }
}

/* ===================================================================== */
/*  jscompile.c rows 408-460                                             */
/* ===================================================================== */

/// Rows 409 / 410 / 453 / 454 / 456 / 457 / 458 / 460: `checkfutureword` at
/// every call site, and the strict-mode-only second list.
#[test]
fn t_future_reserved_words() {
    let future = [
        "class", "const", "enum", "export", "extends", "import", "super",
    ];
    let strictfuture = [
        "implements", "interface", "let", "package", "private", "protected",
        "public", "static", "yield",
    ];
    let ordinary = ["ok", "await", "clas", "classs", "lets", "publicx", "Class"];
    let forms = [
        // jscompile.c:1328 cparams
        "function f({n}){}",
        "(function({n}){})",
        "({set p({n}){}})",
        "new Function('{n}','return 1')",
        // jscompile.c:1348 cvardecs
        "var {n}",
        "var {n}=1",
        "var a,{n}",
        "function f(){{ var {n}; }}",
        "for(var {n} in o);",
        "for(var {n}=0;0;);",
        // jscompile.c:1397 cfunbody (function name)
        "function {n}(){{}}",
        "(function {n}(){{}})",
        "if(1) function {n}(){{}}",
        // jscompile.c:201 emitlocal
        "{n}",
        "{n}=1",
        "print({n})",
        "++{n}",
        "{n}+1",
        "typeof {n}",
        "delete {n}",
        // jscompile.c:1214 / 1230 break / continue label
        "while(1){{break {n};}}",
        "while(1){{continue {n};}}",
        "{n}: while(1){{break {n};}}",
        // jscompile.c:958 ctrycatch
        "try{{}}catch({n}){{}}",
        // jscompile.c:991 ctrycatchfinally
        "try{{}}catch({n}){{}}finally{{}}",
        // property names are NOT identifiers -> never checked
        "x.{n}",
        "({{{n}:1}})",
        "({{get {n}(){{}}}})",
    ];
    for n in future.iter().chain(strictfuture.iter()).chain(ordinary.iter()) {
        for f in forms {
            let src = f.replace("{n}", n);
            diff_all(&src);
        }
    }
    // the exact diagnostics
    let p = libs();
    for n in future {
        let src = format!("var {n}");
        let a = pload(&p.c, 0, "test.js", &src);
        assert_eq!(
            a.msg,
            format!("SyntaxError: test.js:1: '{n}' is a future reserved word")
        );
    }
    for n in strictfuture {
        let src = format!("var {n}");
        assert_eq!(pload(&p.c, 0, "test.js", &src).rc, 0, "{src} non-strict");
        let a = pload(&p.c, JS_STRICT, "test.js", &src);
        assert_eq!(
            a.msg,
            format!("SyntaxError: test.js:1: '{n}' is a strict mode future reserved word")
        );
    }
}

/// Row 455 vs row 454: `ctrycatchfinally` (jscompile.c:990-995) calls
/// `checkfutureword` ONLY when `F->strict`, while `ctrycatch`
/// (jscompile.c:958-963) always calls it.  So `try{}catch(class){}finally{}` is
/// ACCEPTED in sloppy mode while `try{}catch(class){}` is REJECTED.  Pinned
/// explicitly in both libraries.
#[test]
fn t_trycatchfinally_futureword_asymmetry() {
    let p = libs();
    for l in [&p.c, &p.rs] {
        for n in ["class", "const", "enum", "export", "extends", "import", "super"] {
            let with_finally = format!("try{{}}catch({n}){{}}finally{{}}");
            let without = format!("try{{}}catch({n}){{}}");

            let a = pload(l, 0, "test.js", &with_finally);
            assert_eq!(
                a.rc, 0,
                "{}: sloppy try/catch({n})/finally must be ACCEPTED, got {}",
                l.name, a.msg
            );

            let b = pload(l, 0, "test.js", &without);
            assert_eq!(
                b.msg,
                format!("SyntaxError: test.js:1: '{n}' is a future reserved word"),
                "{}: sloppy try/catch({n}) must be REJECTED",
                l.name
            );

            // in strict mode BOTH are rejected
            let c = pload(l, JS_STRICT, "test.js", &with_finally);
            assert_eq!(
                c.msg,
                format!("SyntaxError: test.js:1: '{n}' is a future reserved word"),
                "{}: strict try/catch({n})/finally must be REJECTED",
                l.name
            );
        }
        // 'arguments' / 'eval' as the catch variable: rejected in strict mode by
        // both shapes, accepted in sloppy mode by both
        for n in ["arguments", "eval"] {
            for src in [
                format!("try{{}}catch({n}){{}}"),
                format!("try{{}}catch({n}){{}}finally{{}}"),
            ] {
                assert_eq!(
                    pload(l, 0, "test.js", &src).rc,
                    0,
                    "{}: sloppy {src} must be accepted",
                    l.name
                );
                assert_eq!(
                    pload(l, JS_STRICT, "test.js", &src).msg,
                    format!(
                        "SyntaxError: test.js:1: redefining '{n}' is not allowed in strict mode"
                    ),
                    "{}: strict {src}",
                    l.name
                );
            }
        }
    }
    for n in ["class", "arguments", "eval", "let", "ok"] {
        diff_all(&format!("try{{}}catch({n}){{}}finally{{}}"));
        diff_all(&format!("try{{}}catch({n}){{}}"));
    }
}

/// Row 411: `emitraw` refuses any value that does not round-trip through
/// `js_Instruction` (`unsigned short`).  `emit()` writes `F->lastline` as a raw
/// instruction word first, so a statement on a line past 65535 cannot be
/// encoded.  The message has NO filename prefix (bare `js_syntaxerror`).
#[test]
fn t_instruction_coding_overflow_via_line_number() {
    let p = libs();
    let mut boundary: Option<usize> = None;
    for nl in 65530..=65540usize {
        let src = format!("{}x=1", "\n".repeat(nl));
        let a = pload(&p.c, 0, "t.js", &src);
        let b = pload(&p.rs, 0, "t.js", &src);
        assert_eq!(a, b, "instruction overflow divergence at {} lines", nl + 1);
        if a.rc != 0 && boundary.is_none() {
            boundary = Some(nl + 1);
            assert_eq!(
                a.msg, "SyntaxError: integer overflow in instruction coding",
                "line {} (note: NO filename prefix)",
                nl + 1
            );
        }
    }
    assert_eq!(
        boundary,
        Some(65536),
        "the js_Instruction (unsigned short) boundary must be line 65536"
    );
    // and a few other line numbers on both sides, through every driver
    for nl in [0usize, 1, 100, 65534, 65535, 65536, 70000] {
        let src = format!("{}x=1", "\n".repeat(nl));
        for flags in [0, JS_STRICT] {
            diff_pload(flags, "t.js", &src);
            diff_pload(flags, "", &src);
            diff_leval(flags, "(le)", &src);
        }
        diff_dostring(0, &src);
    }
    // an overflowing line inside a function body, and for a var initialiser
    for src in [
        format!("function f(){{{}return 1}}", "\n".repeat(65540)),
        format!("var q ={} 1", "\n".repeat(65540)),
        format!("{}function f(){{}}", "\n".repeat(65540)),
    ] {
        diff_pload(0, "t.js", &src);
        diff_pload(JS_STRICT, "t.js", &src);
    }
}

/// Row 411 again, through the OTHER kind of oversized instruction argument: the
/// argument COUNT of a call / `new`, which `ccall` / `cexp` pass straight to
/// `emitarg`.  65535 arguments still encode; 65536 does not.
fn body_t_instruction_coding_overflow_via_argument_count() {
    let p = libs();
    let args = |n: usize| {
        let mut s = String::with_capacity(2 * n);
        for i in 0..n {
            if i > 0 {
                s.push(',');
            }
            s.push('1');
        }
        s
    };
    for (n, want_ok) in [
        (0usize, true),
        (1, true),
        (100, true),
        (65534, true),
        (65535, true),
        (65536, false),
        (65537, false),
    ] {
        let a = args(n);
        for src in [format!("f({a})"), format!("new f({a})")] {
            let ca = pload(&p.c, 0, "t.js", &src);
            let cb = pload(&p.rs, 0, "t.js", &src);
            assert_eq!(ca, cb, "argument-count overflow divergence (n={n})");
            if want_ok {
                assert_eq!(ca.rc, 0, "n={n} should compile: {}", ca.msg);
            } else {
                assert_eq!(
                    ca.msg, "SyntaxError: integer overflow in instruction coding",
                    "n={n} (NO filename prefix)"
                );
            }
            diff_pload(JS_STRICT, "t.js", &src);
        }
    }
}

#[test]
fn t_instruction_coding_overflow_via_argument_count() {
    with_big_stack(body_t_instruction_coding_overflow_via_argument_count);
}

/// Rows 427 / 428: `emitjumpto` and `labelto` reject a jump address that does
/// not fit in `js_Instruction`.  Reached by making the code buffer longer than
/// 65535 instructions before the jump is emitted / patched.  Both report
/// `"jump address integer overflow"` with NO filename prefix.
fn body_t_jump_address_overflow() {
    let p = libs();
    // ~11 instructions per `x=1;` statement in script context
    let filler = |n: usize| "x=1;".repeat(n);

    // row 427: emitjumpto(OP_JUMP, loop) where `loop` is past 65535
    let over = format!("{}while(0){{}}", filler(9000));
    let a = pload(&p.c, 0, "t.js", &over);
    let b = pload(&p.rs, 0, "t.js", &over);
    assert_eq!(a, b, "emitjumpto overflow divergence");
    assert_eq!(
        a.msg, "SyntaxError: jump address integer overflow",
        "emitjumpto (jscompile.c:238)"
    );

    // row 428: labelto(inst, addr) where `addr` (F->codelen) is past 65535
    let over2 = format!("if(0){{{}}}", filler(9000));
    let a2 = pload(&p.c, 0, "t.js", &over2);
    let b2 = pload(&p.rs, 0, "t.js", &over2);
    assert_eq!(a2, b2, "labelto overflow divergence");
    assert_eq!(
        a2.msg, "SyntaxError: jump address integer overflow",
        "labelto (jscompile.c:245)"
    );

    // the same shapes just under the limit must compile
    for n in [1usize, 10, 100, 1000, 4000, 5000, 5800] {
        for src in [
            format!("{}while(0){{}}", filler(n)),
            format!("if(0){{{}}}", filler(n)),
            format!("{}do{{}}while(0)", filler(n)),
            format!("{}for(;0;){{}}", filler(n)),
            format!("{}1?2:3", filler(n)),
            format!("{}1&&2", filler(n)),
            format!("{}1||2", filler(n)),
            format!("{}switch(1){{case 1:break}}", filler(n)),
        ] {
            diff_pload(0, "t.js", &src);
            diff_pload(JS_STRICT, "t.js", &src);
        }
    }
    // sweep across the boundary for each jump-emitting construct
    for n in [5900usize, 5950, 5960, 5970, 6000, 6100, 7000, 9000] {
        for src in [
            format!("{}while(0){{}}", filler(n)),
            format!("if(0){{{}}}", filler(n)),
            format!("{}do{{}}while(0)", filler(n)),
            format!("{}for(;0;){{}}", filler(n)),
            format!("{}1?2:3", filler(n)),
            format!("{}1&&2", filler(n)),
            format!("{}switch(1){{case 1:break}}", filler(n)),
            format!("while(0){{{}}}", filler(n)),
            format!("{}L:while(0){{break L}}", filler(n)),
        ] {
            diff_pload(0, "t.js", &src);
        }
    }
}

#[test]
fn t_jump_address_overflow() {
    with_big_stack(body_t_jump_address_overflow);
}

/// Rows 412 / 413 / 419: `emitraw` grows `F->code`, `addfunction` grows
/// `F->funtab` and `addlocal` grows `F->vartab`, all through `js_realloc`.
/// Exercised at their doubling boundaries (64/128/..., 16/32/..., 16/32/...)
/// and under a `js_setlimit` budget so the failure path is compared too.
#[test]
fn t_compile_table_growth_and_out_of_memory() {
    let mut srcs: Vec<String> = vec![];
    // row 412: code buffer doubling
    for n in [0usize, 1, 2, 5, 6, 7, 8, 15, 16, 31, 32, 63, 64, 100, 500, 2000] {
        srcs.push("x=1;".repeat(n));
    }
    // row 413: nested-function table doubling (0 -> 16 -> 32 -> ...)
    for n in [0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100] {
        srcs.push(format!(
            "var a=[{}]",
            (0..n).map(|_| "function(){}").collect::<Vec<_>>().join(",")
        ));
        srcs.push(format!(
            "function f(){{{}}}",
            (0..n)
                .map(|i| format!("function g{i}(){{}}"))
                .collect::<Vec<_>>()
                .join("")
        ));
    }
    // row 419: local variable table doubling
    for n in [0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 200] {
        srcs.push(format!(
            "function f(){{var {};}}",
            (0..n).map(|i| format!("v{i}")).collect::<Vec<_>>().join(",")
        ));
        if n > 0 {
            srcs.push(format!(
                "function f({}){{}}",
                (0..n).map(|i| format!("p{i}")).collect::<Vec<_>>().join(",")
            ));
        }
    }
    for s in &srcs {
        diff_pload(0, "t.js", s);
        diff_pload(JS_STRICT, "t.js", s);
    }
    // under memory pressure
    let mut lims: Vec<c_int> = (1..=32).collect();
    lims.extend([64, 128, 256, 512, 1024, 4096, 1 << 14, 1 << 16, 1 << 18]);
    let mut rng = Rng::new(0x5EED_000C);
    for _ in 0..40 {
        lims.push(1 + rng.below(1 << 15) as c_int);
    }
    let stress = [
        "x=1;".repeat(300),
        format!(
            "var a=[{}]",
            (0..40).map(|_| "function(){}").collect::<Vec<_>>().join(",")
        ),
        format!(
            "function f(){{var {};}}",
            (0..60).map(|i| format!("v{i}")).collect::<Vec<_>>().join(",")
        ),
    ];
    for s in &stress {
        for lim in &lims {
            diff_pload_mem(0, *lim, s);
        }
    }
}

/// Rows 414-418, 420, 423-426: `addlocal`, `findlocal` and `emitlocal` -- the
/// strict-mode rejections of `arguments` / `eval`, the non-strict `EvalError`
/// paths (which build their own `"%s:%d: invalid use of 'eval'"` prefix, so the
/// FILENAME and LINE are part of the message), the `reuse` slot hit, the
/// duplicate formal parameter check, and the by-name fallback.
#[test]
fn t_addlocal_and_emitlocal() {
    let mut srcs: Vec<String> = vec![];
    for n in ["arguments", "eval"] {
        for f in [
            // rows 414/415/416 addlocal
            "var {n}",
            "var {n}=1",
            "var a,{n}",
            "function f({n}){{}}",
            "(function({n}){{}})",
            "function {n}(){{}}",
            "(function {n}(){{}})",
            "for(var {n} in o);",
            "function f(){{var {n};}}",
            "({{set p({n}){{}}}})",
            // rows 423/424/425 emitlocal
            "{n}=1",
            "{n}",
            "print({n})",
            "++{n}",
            "{n}++",
            "{n}+=1",
            "typeof {n}",
            "delete {n}",
            "x.{n}",
            "({{{n}:1}})",
            "function f(){{ {n}=1 }}",
            "function f(){{ return {n} }}",
            "for({n} in o);",
            // eval called as a function is special-cased by ccall
            "{n}('1')",
            "{n}(1,2)",
            "{n}()",
        ] {
            srcs.push(f.replace("{n}", n));
        }
    }
    // row 417: reuse -> existing slot
    for s in [
        "var a; var a;", "var a=1; var a=2;", "function f(){var a;var a;}",
        "function f(a){var a;}", "var a; function a(){}",
        "function a(){} var a;", "function f(){var a,a;}",
    ] {
        srcs.push(s.to_string());
    }
    // row 418: duplicate formal parameter (strict only)
    for s in [
        "function f(a,a){}", "function f(a,b,a){}", "(function(a,a){})",
        "function f(a,a,a){}", "new Function('a','a','return 1')",
        "function f(a,b){}",
    ] {
        srcs.push(s.to_string());
    }
    // rows 420/426: findlocal returns -1 -> by-name opcode
    for s in [
        "notdeclared", "notdeclared=1", "print(typeof notdeclared)",
        "function f(){ return notdeclared }", "delete notdeclared",
        "notdeclared.x", "notdeclared()",
    ] {
        srcs.push(s.to_string());
    }
    for s in &srcs {
        diff_all(s);
    }
    // the EvalError message carries the filename and the line of the identifier
    let p = libs();
    for (src, line) in [("var eval", 1), ("\n\nvar eval", 3), ("\n\neval=1", 3)] {
        let a = pload(&p.c, 0, "ev.js", src);
        assert_eq!(
            a.msg,
            format!("EvalError: ev.js:{line}: invalid use of 'eval'"),
            "{src:?}"
        );
        assert_eq!(a, pload(&p.rs, 0, "ev.js", src));
    }
    // strict mode reports the read-only / redefining SyntaxErrors instead
    for (src, want) in [
        ("var eval", "SyntaxError: s.js:1: redefining 'eval' is not allowed in strict mode"),
        ("var arguments", "SyntaxError: s.js:1: redefining 'arguments' is not allowed in strict mode"),
        ("eval=1", "SyntaxError: s.js:1: 'eval' is read-only in strict mode"),
        ("arguments=1", "SyntaxError: s.js:1: 'arguments' is read-only in strict mode"),
        ("function f(a,a){}", "SyntaxError: s.js:1: duplicate formal parameter 'a'"),
    ] {
        let a = pload(&p.c, JS_STRICT, "s.js", src);
        assert_eq!(a.msg, want, "{src:?}");
        assert_eq!(a, pload(&p.rs, JS_STRICT, "s.js", src));
    }
    // strict-mode `eval` READ still takes the EvalError path (oploc is
    // OP_GETLOCAL, so the read-only check is skipped)
    let a = pload(&p.c, JS_STRICT, "s.js", "print(eval)");
    assert_eq!(a.msg, "EvalError: s.js:1: invalid use of 'eval'");
    assert_eq!(a, pload(&p.rs, JS_STRICT, "s.js", "print(eval)"));
}

/// Rows 421 / 422: `emitnumber`'s three shapes -- the `num == 0` bias
/// (plus the extra `OP_NEG` for `-0.0`), the `SHRT_MIN..SHRT_MAX` integer
/// shape, and the raw-double fallback.
#[test]
fn t_emitnumber_shapes() {
    let mut srcs: Vec<String> = vec![];
    let vals = [
        "0", "-0", "0.0", "-0.0", "0e0", "-0e0", "1", "-1", "32766", "32767",
        "32768", "32769", "-32767", "-32768", "-32769", "-32770", "1.5", "-1.5",
        "0.5", "1e21", "1e-21", "1e308", "1e309", "-1e309", "4294967296",
        "9007199254740992", "9007199254740993", "0.1", "1/0", "-1/0", "0/0",
    ];
    for v in vals {
        srcs.push(format!("print({v})"));
        srcs.push(format!("dump({v})"));
        srcs.push(format!("print(1/({v}))"));
        srcs.push(format!("var x={v}; print(x, 1/x)"));
        srcs.push(format!("print([{v}][0])"));
        srcs.push(format!("print(({{k:{v}}}).k)"));
        srcs.push(format!("print(({v})===({v}))"));
    }
    let mut rng = Rng::new(0x5EED_000D);
    for _ in 0..600 {
        let n = rng.range(-40000, 40000);
        srcs.push(format!("print({n})"));
        srcs.push(format!("print({n}.5)"));
    }
    for s in &srcs {
        diff_dostring(0, s);
        diff_dostring(JS_STRICT, s);
    }
    for s in srcs.iter().take(120) {
        diff_all(s);
    }
}

/// Row 429: `checkdup` -- strict-mode duplicate object-literal keys.  Numeric
/// keys are compared after `jsV_numbertostring` into a 32-byte buffer, so a
/// number and the string that spells it collide.
#[test]
fn t_object_literal_duplicate_property() {
    let mut srcs: Vec<String> = vec![];
    for s in [
        "({a:1,a:2})", "({a:1,b:2,a:3})", "({a:1,a:2,a:3})",
        "({1:1,1:2})", "({1:1,'1':2})", "({'1':1,1:2})",
        "({1.0:1,1:2})", "({1.5:1,1.5:2})", "({0:1,'0':2})",
        "({1e21:1,1e21:2})", "({1e21:1,'1e+21':2})",
        "({1e-21:1,'1e-21':2})", "({0.1:1,'0.1':2})",
        "({4294967296:1,'4294967296':2})",
        "({12345678901234567890:1,'12345678901234567000':2})",
        "({get a(){},get a(){}})", "({set a(v){},set a(v){}})",
        "({get a(){},set a(v){}})", "({a:1,get a(){}})",
        "({get a(){},a:1})", "({a:1,b:2})", "({})", "({a:1})",
        "({if:1,if:2})", "({'a':1,'a':2})", "({a:1,'a':2})",
        "({'\\u0041':1,A:2})", "({0:1,'-0':2})", "({'-0':1,'-0':2})",
        "({NaN:1,'NaN':2})",
    ] {
        srcs.push(s.to_string());
        srcs.push(format!("function f(){{'use strict'; return {s} }} f()"));
    }
    for s in &srcs {
        diff_all(s);
    }
    let p = libs();
    // `jsC_error(J, list, ...)` blames the AST_LIST node, whose line is the
    // hard-coded 0 from `LIST(h)` (jsparse.c:3) -- not the source line.
    for l in [&p.c, &p.rs] {
        assert_eq!(
            pload(l, JS_STRICT, "d.js", "({a:1,a:2})").msg,
            "SyntaxError: d.js:0: duplicate property 'a' in object literal",
            "{}",
            l.name
        );
        assert_eq!(
            pload(l, JS_STRICT, "d.js", "\n\n({a:1,\na:2})").msg,
            "SyntaxError: d.js:0: duplicate property 'a' in object literal",
            "{}: the AST_LIST line is always 0",
            l.name
        );
        // ... and it is a STRICT-mode-only check
        assert_eq!(pload(l, 0, "d.js", "({a:1,a:2})").rc, 0, "{}", l.name);
    }
}

/// Rows 430 / 431 are UNREACHABLE.
///
///   * jscompile.c:329-336 (`"invalid property name in object initializer"`):
///     `cobject`'s key node always comes from `propname` (jsparse.c:207), which
///     can only produce `EXP_NUMBER`, `EXP_STRING` or `AST_IDENTIFIER`, and
///     constant folding never rewrites a property-name node.
///   * jscompile.c:342-343 (the `default: /* impossible */ break;` of the
///     `kv->type` switch): `propassign` only ever builds `EXP_PROP_VAL`,
///     `EXP_PROP_GET` or `EXP_PROP_SET`.
///
/// This test enumerates every key shape the grammar admits and asserts that
/// none of them reaches either site.
#[test]
fn t_unreachable_object_key_paths() {
    let p = libs();
    let keys = [
        "a", "_a", "$a", "A1", "if", "var", "function", "true", "false", "null",
        "in", "class", "0", "1", "1.5", "1e21", "0x10", "'s'", "\"s\"", "''",
        "'\\u0041'", "\\u0041",
    ];
    for k in keys {
        for form in [
            format!("({{{k}:1}})"),
            format!("({{get {k}(){{}}}})"),
            format!("({{set {k}(v){{}}}})"),
            format!("({{{k}:1,b:2}})"),
        ] {
            for l in [&p.c, &p.rs] {
                let r = pload(l, 0, "t.js", &form);
                assert!(
                    !r.msg.contains("invalid property name in object initializer"),
                    "{}: {form:?} reached the unreachable cobject error: {}",
                    l.name,
                    r.msg
                );
            }
            diff_all(&form);
        }
    }
}

/// Rows 432 / 434 / 435 / 437 / 438: every `"invalid l-value"` /
/// `"delete on an unqualified name"` rejection in `jscompile.c`.
#[test]
fn t_invalid_lvalues() {
    let mut srcs: Vec<String> = vec![];
    let bad = [
        "1", "1.5", "'s'", "true", "false", "null", "this", "f()", "(1,2)",
        "[1]", "({})", "-1", "!1", "1+1", "typeof x", "void 0", "new f",
        "(function(){})", "/re/", "delete x", "(1?2:3)", "1&&2", "x++",
    ];
    for b in bad {
        // row 432 cassign
        srcs.push(format!("{b}=1"));
        // row 435 cassignop1
        srcs.push(format!("{b}+=1"));
        srcs.push(format!("{b}-=1"));
        srcs.push(format!("{b}*=1"));
        srcs.push(format!("++{b}"));
        srcs.push(format!("--{b}"));
        srcs.push(format!("{b}++"));
        srcs.push(format!("{b}--"));
        // row 434 cassignforin
        srcs.push(format!("for({b} in o);"));
        // row 438 cdelete
        srcs.push(format!("delete {b}"));
    }
    // valid l-values for contrast
    for g in ["x", "x.y", "x[0]", "x.y.z", "x[0][1]", "x[y]"] {
        srcs.push(format!("{g}=1"));
        srcs.push(format!("{g}+=1"));
        srcs.push(format!("++{g}"));
        srcs.push(format!("{g}++"));
        srcs.push(format!("for({g} in o);"));
        srcs.push(format!("delete {g}"));
    }
    // row 437: `delete <bare identifier>` in strict mode
    for s in [
        "delete x", "delete undeclared", "function f(){delete x}",
        "delete x.y", "delete x[0]", "var a; delete a",
    ] {
        srcs.push(s.to_string());
    }
    for s in &srcs {
        diff_all(s);
    }
    let p = libs();
    for (src, want) in [
        ("1=2", "SyntaxError: t.js:1: invalid l-value in assignment"),
        ("1+=2", "SyntaxError: t.js:1: invalid l-value in assignment"),
        ("++1", "SyntaxError: t.js:1: invalid l-value in assignment"),
        ("delete 1", "SyntaxError: t.js:1: invalid l-value in delete expression"),
        ("for(1 in o);", "SyntaxError: t.js:1: invalid l-value in for-in loop assignment"),
    ] {
        assert_eq!(pload(&p.c, 0, "t.js", src).msg, want, "{src:?}");
        assert_eq!(pload(&p.rs, 0, "t.js", src).msg, want, "{src:?}");
    }
    assert_eq!(
        pload(&p.c, JS_STRICT, "t.js", "delete x").msg,
        "SyntaxError: t.js:1: delete on an unqualified name is not allowed in strict mode"
    );
}

/// Row 436 is UNREACHABLE: `cassignop2` (jscompile.c:486-487) has exactly the
/// same `EXP_IDENTIFIER` / `EXP_INDEX` / `EXP_MEMBER` switch as `cassignop1`
/// (jscompile.c:463-464) and is only ever reached after `cassignop1` has
/// already accepted the same node, so its `default:` can never be taken.
/// Row 439 (`cexp`'s `default: "unknown expression type"`) is unreachable for a
/// related reason: every node type the parser can place in expression position
/// has an explicit `case`.
#[test]
fn t_unreachable_compile_defaults() {
    let p = libs();
    let exprs = [
        "1", "'s'", "/re/", "null", "true", "false", "this", "[1,,2]", "({a:1})",
        "(function(){})", "x", "x[0]", "x.y", "f()", "new f", "new f()",
        "delete x.y", "void 0", "typeof x", "++x", "--x", "x++", "x--", "+x",
        "-x", "~x", "!x", "x*1", "x/1", "x%1", "x+1", "x-1", "x<<1", "x>>1",
        "x>>>1", "x<1", "x>1", "x<=1", "x>=1", "x instanceof f", "'a' in x",
        "x==1", "x!=1", "x===1", "x!==1", "x&1", "x^1", "x|1", "x&&1", "x||1",
        "x?1:2", "x=1", "x*=1", "x/=1", "x%=1", "x+=1", "x-=1", "x<<=1",
        "x>>=1", "x>>>=1", "x&=1", "x^=1", "x|=1", "(x,1)", "eval2(1)",
    ];
    for e in exprs {
        for form in [
            format!("print({e})"),
            format!("var v = {e}"),
            format!("{e};"),
            format!("if({e});"),
            format!("return2({e})"),
        ] {
            for l in [&p.c, &p.rs] {
                let r = pload(l, 0, "t.js", &form);
                assert!(
                    !r.msg.contains("unknown expression type"),
                    "{}: {form:?} reached cexp's unreachable default: {}",
                    l.name,
                    r.msg
                );
            }
        }
        diff_all(&format!("print({e})"));
    }
    // and the "invalid l-value" rejections always come from cassignop1, never
    // from cassignop2: the diagnostic is emitted exactly once
    for src in ["1+=2", "++1", "1++", "f()+=1", "f()++"] {
        for l in [&p.c, &p.rs] {
            let r = pload(l, 0, "t.js", src);
            assert_eq!(
                r.msg, "SyntaxError: t.js:1: invalid l-value in assignment",
                "{}: {src:?}",
                l.name
            );
        }
    }
}

/// Row 433: `for (var a, b in x)` -- more than one loop variable.
#[test]
fn t_for_in_var_multiple_variables() {
    for s in [
        "for(var a,b in x);", "for(var a,b,c in x);", "for(var a=1,b in x);",
        "for(var a in x);", "for(var a=1 in x);", "for(var a,b;;);",
        "for(var a,b=2;0;);",
    ] {
        diff_all(s);
    }
    let p = libs();
    // NOTE the line number 0: `jsC_error(J, lhs->b, ...)` blames the AST_LIST
    // node that holds the second declarator, and `LIST(h)` (jsparse.c:3) builds
    // its nodes with a hard-coded line of 0.
    for l in [&p.c, &p.rs] {
        assert_eq!(
            pload(l, 0, "t.js", "for(var a,b in x);").msg,
            "SyntaxError: t.js:0: more than one loop variable in for-in statement",
            "{}",
            l.name
        );
        assert_eq!(
            pload(l, 0, "t.js", "\n\n\nfor(var a,b in x);").msg,
            "SyntaxError: t.js:0: more than one loop variable in for-in statement",
            "{}: the AST_LIST line is always 0, not the source line",
            l.name
        );
    }
}

/// Rows 440-443, 445-449, 451, 452: `breaktarget` / `continuetarget` /
/// `returntarget` failing, `cexit`'s `default: /* impossible */` frames on the
/// unwind path, `ctryfinally` for a `try` without a `catch`, and plain
/// expression statements.
#[test]
fn t_break_continue_return_targets() {
    let mut srcs: Vec<String> = vec![];
    for s in [
        // rows 445/446 break
        "break;", "break x;", "{break;}", "if(1)break;", "L:{break L;}",
        "L:{break M;}", "while(1){break;}", "while(1){break x;}",
        "L:while(1){break L;}", "L:while(1){break M;}",
        "switch(1){case 1:break;}", "switch(1){case 1:break L;}",
        "L:switch(1){case 1:break L;}",
        "while(1){function g(){break;}}",
        "L:while(1){function g(){break L;}}",
        // rows 447/448 continue
        "continue;", "continue x;", "{continue;}", "while(1){continue;}",
        "while(1){continue x;}", "L:while(1){continue L;}",
        "L:while(1){continue M;}", "L:{continue L;}",
        "switch(1){case 1:continue;}",
        "L:switch(1){case 1:continue L;}",
        "while(1){function g(){continue;}}",
        "do{continue}while(0)", "for(;;){continue}", "for(k in o){continue}",
        "for(var k in o){continue}",
        // rows 442/449 return
        "return;", "return 1;", "{return 1;}", "if(1)return;",
        "function f(){return}", "function f(){return 1}",
        "(function(){return 1})()", "({get a(){return 1}})",
        "({set a(v){return}})",
        // row 443: unwind frames of every kind
        "while(1){{break;}}", "while(1){if(1)break;}",
        "while(1){L:{break;}}", "while(1){with(o){break}}",
        "while(1){try{break}catch(e){}}",
        "while(1){try{break}finally{}}",
        "while(1){try{}catch(e){break}}",
        "while(1){try{}catch(e){break}finally{}}",
        "for(k in o){break}", "for(k in o){if(1)break}",
        "for(k in o){continue}", "for(k in o){with(p){break}}",
        "function f(){for(k in o){return 1}}",
        "function f(){while(1){try{return 1}finally{}}}",
        "function f(){with(o){return 1}}",
        "L:for(k in o){for(j in p){continue L}}",
        "L:for(k in o){for(j in p){break L}}",
        // row 451: try without catch -> ctryfinally
        "try{}finally{}", "try{1}finally{2}", "try{throw 1}finally{}",
        "try{}catch(e){}", "try{}catch(e){}finally{}",
        // row 452: expression statements
        "1;", "1", "x;", "f();", "x=1;", "(1,2);", "typeof x;", "'s';",
        "function f(){1;}", "function f(){x=1}",
    ] {
        srcs.push(s.to_string());
    }
    for s in &srcs {
        diff_all(s);
    }
    let p = libs();
    for (src, want) in [
        ("break;", "SyntaxError: t.js:1: unlabelled break must be inside loop or switch"),
        ("break foo;", "SyntaxError: t.js:1: break label 'foo' not found"),
        ("continue;", "SyntaxError: t.js:1: continue must be inside loop"),
        ("continue foo;", "SyntaxError: t.js:1: continue label 'foo' not found"),
        ("return;", "SyntaxError: t.js:1: return not in function"),
    ] {
        assert_eq!(pload(&p.c, 0, "t.js", src).msg, want, "{src:?}");
        assert_eq!(pload(&p.rs, 0, "t.js", src).msg, want, "{src:?}");
    }
}

/// Row 444: a second `default:` clause in a switch.
#[test]
fn t_switch_multiple_defaults() {
    for s in [
        "switch(1){default:;default:;}", "switch(1){default:default:}",
        "switch(1){case 1:default:case 2:default:}",
        "switch(1){default:;default:;default:;}",
        "switch(1){default:}", "switch(1){case 1:default:}",
        "switch(1){}", "switch(1){case 1:case 2:}",
        "switch(1){default:switch(2){default:}}",
    ] {
        diff_all(s);
    }
    let p = libs();
    assert_eq!(
        pload(&p.c, 0, "t.js", "switch(1){default:;default:;}").msg,
        "SyntaxError: t.js:1: more than one default label in switch"
    );
}

/// Row 450: `with` in strict mode.  The line number comes from `stm->a` (the
/// object expression), NOT from the `with` keyword, so put them on different
/// lines.
#[test]
fn t_with_in_strict_mode() {
    let srcs = [
        "with(o){}",
        "with(o);",
        "with({a:1}){print(a)}",
        "function f(){with(o){}}",
        "function f(){'use strict';with(o){}}",
        "with(o){with(p){}}",
        "\n\nwith(o){}",
        "with\n(\no\n){}",
        "with(\n\n\no){}",
    ];
    for s in srcs {
        diff_all(s);
    }
    let p = libs();
    // stm->a is the object expression: `o` is on line 4 here
    let src = "with\n(\n\no\n){}";
    let a = pload(&p.c, JS_STRICT, "w.js", src);
    assert_eq!(
        a.msg, "SyntaxError: w.js:4: 'with' statements are not allowed in strict mode",
        "the line must come from stm->a"
    );
    assert_eq!(a, pload(&p.rs, JS_STRICT, "w.js", src));
    assert_eq!(pload(&p.c, 0, "w.js", src).rc, 0);
}

/// Row 459: a leading `"use strict"` string literal statement flips
/// `F->strict`, enabling every strict-mode rejection for that function only.
#[test]
fn t_use_strict_directive() {
    let mut srcs: Vec<String> = vec![];
    for prologue in [
        "'use strict';", "\"use strict\";", "'use strict'\n", "'use  strict';",
        "'Use strict';", "'use stricts';", "1;'use strict';", ";'use strict';",
        "", "'\\u0075se strict';",
    ] {
        for body in [
            "with(o){}", "var eval=1", "delete x", "arguments=1",
            "function g(a,a){}", "({a:1,a:2})", "var let", "octal=1",
        ] {
            srcs.push(format!("{prologue}{body}"));
            srcs.push(format!("function f(){{{prologue}{body}}}"));
            srcs.push(format!("(function(){{{prologue}{body}}})"));
            srcs.push(format!("({{get p(){{{prologue}{body}}}}})"));
        }
    }
    // strictness does NOT leak out of the function that declares it
    for s in [
        "function f(){'use strict'} with(o){}",
        "function f(){'use strict'; function g(){ with(o){} } }",
        "'use strict'; function g(){ with(o){} }",
    ] {
        srcs.push(s.to_string());
    }
    for s in &srcs {
        diff_all(s);
    }
}

/// Rows 456-458 and 460 in their own right: `cparams`, `cvardecs`, `cfundecs`
/// and `cfunbody`'s self-binding for a named function expression.
#[test]
fn t_params_vardecs_fundecs_and_selfbinding() {
    let mut srcs: Vec<String> = vec![];
    for n in ["ok", "class", "let", "eval", "arguments", "f", "g"] {
        for f in [
            // cparams
            "function f({n}){{ return {n} }}",
            "function f({n},{n}){{}}",
            "(function({n}){{}})",
            "({{set p({n}){{}}}})",
            // cvardecs (reuse = 1)
            "var {n}",
            "function f(){{ var {n}; var {n}; }}",
            "function f({n}){{ var {n}; }}",
            "if(1){{ var {n}; }}",
            "for(var {n}=0;0;);",
            "try{{}}catch(e){{ var {n}; }}",
            "while(0){{ var {n}; }}",
            "switch(1){{case 1: var {n};}}",
            // cfundecs (hoisted declaration, reuse = 1)
            "function {n}(){{}}",
            "function {n}(){{}} function {n}(){{}}",
            "var {n}; function {n}(){{}}",
            "function f(){{ function {n}(){{}} }}",
            // cfunbody self-binding for a named function EXPRESSION
            "(function {n}(){{ return {n} }})",
            "var h = function {n}(){{ return typeof {n} }}; print(h())",
            "(function {n}({n}){{ return {n} }})",
            "(function {n}(){{ var {n}; return {n} }})",
        ] {
            srcs.push(f.replace("{n}", n));
        }
    }
    for s in &srcs {
        diff_all(s);
    }
}

/* ===================================================================== */
/*  jsdtoa.c rows 461-476                                                */
/* ===================================================================== */

/// Rows 461 / 462 are UNCOVERABLE by a differential test.  `minus()`
/// (jsdtoa.c:386-387) asserts `x.e == y.e` and `x.f >= y.f`; the C library is
/// built without `NDEBUG` (see c_src/CMakeLists.txt: no `-DNDEBUG`, no
/// `CMAKE_BUILD_TYPE`), so violating either precondition calls `abort()` and
/// TERMINATES THE PROCESS instead of raising a JS exception.  `minus` is
/// `static` and only reachable through `js_grisu2`; the only inputs that break
/// the preconditions are the ones `js_grisu2`'s documented contract already
/// excludes -- `0.0`, a negative value and a non-finite value (the real caller,
/// `jsV_numbertostring`, special-cases all three and passes `fabs(v)`).
/// Calling `js_grisu2(-1.0, ...)` aborts under both libraries identically, but
/// an aborting process cannot be compared, so those inputs are excluded here
/// exactly as they are in tests/ll_num.rs `t_grisu2`.
///
/// Rows 463 / 464 / 465 / 466 are UNDEFINED BEHAVIOUR in the C and are
/// deliberately NOT tested:
///
///   * jsdtoa.c:370-377 `cached_power`: `powers_ten[343 + k]` has no bounds
///     check, so a `k` outside the table reads out of bounds.
///   * jsdtoa.c:480 `digit_gen`: `((uint64_t)1) << -Mp.e` is UB when
///     `Mp.e >= 0`.
///   * jsdtoa.c:486,495 `digit_gen`: `buffer[(*len)++]` has no bounds check.
///   * jsdtoa.c:36-43 `js_fmtexp`: a 10-digit exponent writes `se[9]`, one past
///     the array (already documented in tests/ll_num.rs `t_fmtexp`).
///
/// What IS testable is that inside the contract every positive finite double
/// converts identically -- which pins the fact that neither library ever
/// reaches those paths.
#[test]
fn t_grisu_invariants_hold_inside_the_contract() {
    let p = libs();
    unsafe {
        let mut vals: Vec<f64> = vec![
            1.0, f64::MIN_POSITIVE, 5e-324, f64::MAX, 1e308, 1e-308, 0.1, 0.5,
            1.5, 1e21, 1e-21, 9007199254740992.0, 9007199254740993.0,
            2.2250738585072011e-308, 4.9406564584124654e-324,
            1.7976931348623157e308,
        ];
        let mut rng = Rng::new(0x5EED_000E);
        for _ in 0..4000 {
            vals.push(rng.f64_any());
            vals.push(rng.f64_sane());
        }
        for x in vals {
            // js_grisu2's precondition: positive and finite (jsV_numbertostring
            // handles 0 / -0 / inf / nan and the sign itself)
            if x == 0.0 || !x.is_finite() {
                continue;
            }
            let v = x.abs();
            let mut ba = [0i8; 64];
            let mut bb = [0i8; 64];
            let mut ka: c_int = -12345;
            let mut kb: c_int = -12345;
            let na = p.c.js_grisu2(v, ba.as_mut_ptr(), &mut ka);
            let nb = p.rs.js_grisu2(v, bb.as_mut_ptr(), &mut kb);
            assert_eq!((na, ka), (nb, kb), "js_grisu2({v:e}) len/k");
            assert_eq!(&ba[..], &bb[..], "js_grisu2({v:e}) digits");
        }
        // and through the real caller, which is total over the doubles
        let jc = new_state(&p.c, 0);
        set_cur(&p.rs);
        let jr = new_state(&p.rs, 0);
        let mut rng = Rng::new(0x5EED_001E);
        let mut vals: Vec<f64> = vec![0.0, -0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
        for _ in 0..4000 {
            vals.push(rng.f64_any());
            vals.push(rng.f64_sane());
        }
        for v in vals {
            let mut ba = [0i8; 64];
            let mut bb = [0i8; 64];
            set_cur(&p.c);
            let sa = from_c(p.c.jsV_numbertostring(jc, ba.as_mut_ptr(), v));
            set_cur(&p.rs);
            let sb = from_c(p.rs.jsV_numbertostring(jr, bb.as_mut_ptr(), v));
            assert_eq!(sa, sb, "jsV_numbertostring({:#x})", v.to_bits());
        }
        set_cur(&p.c);
        p.c.js_freestate(jc);
        set_cur(&p.rs);
        p.rs.js_freestate(jr);
    }
}

/// Row 467: `js_fmtexp` with `e == 0` pads a single `'0'`, producing `"e+0"`.
#[test]
fn t_fmtexp_zero_exponent() {
    let p = libs();
    unsafe {
        for e in [0, 1, -1, 9, -9, 10, -10, 100, -100, 999, -999] {
            let mut ba = [0i8; 32];
            let mut bb = [0i8; 32];
            p.c.js_fmtexp(ba.as_mut_ptr(), e);
            p.rs.js_fmtexp(bb.as_mut_ptr(), e);
            assert_eq!(ba, bb, "js_fmtexp({e})");
        }
        let mut b = [0i8; 32];
        p.c.js_fmtexp(b.as_mut_ptr(), 0);
        assert_eq!(from_c(b.as_ptr()), "e+0", "row 467");
        let mut b2 = [0i8; 32];
        p.rs.js_fmtexp(b2.as_mut_ptr(), 0);
        assert_eq!(from_c(b2.as_ptr()), "e+0", "row 467 (RUST)");
    }
}

/// Compare `js_strtod` with and without an `endPtr` (row 476: `endPtr == NULL`
/// is the calling convention used by `lexnumber` and `lexjsonnumber`, so the
/// caller cannot detect a partial conversion).
fn diff_strtod(s: &str) {
    let p = libs();
    unsafe {
        let cs = cstr(s);
        let mut ea: *mut c_char = std::ptr::null_mut();
        let mut eb: *mut c_char = std::ptr::null_mut();
        let a = p.c.js_strtod(cs.as_ptr(), &mut ea);
        let b = p.rs.js_strtod(cs.as_ptr(), &mut eb);
        assert_eq!(a.to_bits(), b.to_bits(), "js_strtod({s:?}) value");
        let oa = ea.offset_from(cs.as_ptr() as *mut c_char);
        let ob = eb.offset_from(cs.as_ptr() as *mut c_char);
        assert_eq!(oa, ob, "js_strtod({s:?}) endptr");
        assert!(
            oa >= 0 && oa as usize <= cs.as_bytes().len(),
            "js_strtod({s:?}) endptr {oa} out of the buffer"
        );
        // row 476: NULL endPtr must not change the value
        let a2 = p.c.js_strtod(cs.as_ptr(), std::ptr::null_mut());
        let b2 = p.rs.js_strtod(cs.as_ptr(), std::ptr::null_mut());
        assert_eq!(a2.to_bits(), a.to_bits(), "C js_strtod({s:?}) NULL endPtr");
        assert_eq!(b2.to_bits(), b.to_bits(), "RUST js_strtod({s:?}) NULL endPtr");
    }
}

/// Rows 468 / 469 / 471 / 472 / 476: `js_strtod`'s scanning rules -- leading
/// whitespace, the mantissa scan stopping at a non-digit or a second `.`, the
/// `mantSize == 0` "no conversion" result (which resets `*endPtr` to the
/// original string), and an `E` that is not followed by any digit (whose
/// characters are still consumed).
#[test]
fn t_strtod_scanning() {
    let mut v: Vec<String> = vec![];
    for s in [
        // row 468: leading ' ', '\t', '\n', '\r' are skipped (and ONLY those)
        " 1", "\t1", "\n1", "\r1", " \t\n\r 1", "  -1", "\t+1", "\n\r-2.5e3",
        "\u{b}1", "\u{c}1", "\u{a0}1", " ", "\t", "\n", "\r", "   ",
        // row 469: mantissa scan stops at a non-digit or a second '.'
        "1a", "1 2", "1..2", "1.2.3", "..1", ".1.", "1,2", "12x34", "1-2",
        "1+2", "1e5e5", "0x10", "1_000", "1'0",
        // row 471: mantSize == 0 -> 0.0 (or -0.0) and *endPtr = string
        "", "-", "+", ".", "-.", "+.", "..", "abc", "-abc", "+abc", "e5",
        "E5", "-e5", "x", "-x", " -", " .", "-.e5", "inf", "Infinity", "nan",
        "NaN", "-Infinity", "null", "true",
        // row 472: E present but no digits -> exp stays 0, chars consumed
        "1e", "1E", "1e+", "1e-", "1E+", "1E-", "1e+x", "1e-x", "1ex",
        "1.5e", "1.5e-", "0e", "0e+", "-1e", ".5e", "1e+ 5", "1e--5",
        // and the plain accepted forms
        "0", "1", "-1", "1.5", "-1.5", "1e5", "1e-5", "1E+5", ".5", "5.",
        "0.0", "-0.0", "-0",
    ] {
        v.push(s.to_string());
    }
    for s in &v {
        diff_strtod(s);
    }
    // pin row 471: "no conversion performed" leaves *endPtr == string
    let p = libs();
    unsafe {
        for s in ["", "-", "+", ".", "abc", "e5", " -x"] {
            let cs = cstr(s);
            for l in [&p.c, &p.rs] {
                let mut e: *mut c_char = std::ptr::null_mut();
                let val = l.js_strtod(cs.as_ptr(), &mut e);
                assert_eq!(
                    e.offset_from(cs.as_ptr() as *mut c_char),
                    0,
                    "{}: js_strtod({s:?}) must reset endPtr to `string`",
                    l.name
                );
                let want = if s.starts_with('-') || s.trim_start().starts_with('-') {
                    (-0.0f64).to_bits()
                } else {
                    0.0f64.to_bits()
                };
                assert_eq!(
                    val.to_bits(),
                    want,
                    "{}: js_strtod({s:?}) must be +-0.0",
                    l.name
                );
            }
        }
        // pin row 472: the bogus exponent characters ARE consumed (so *endPtr
        // moves past them) and `exp` stays 0, so the mantissa is returned as is
        for (s, want_off, want_val) in [
            ("1e", 2isize, 1.0f64),
            ("1E", 2, 1.0),
            ("1e+", 3, 1.0),
            ("1e-", 3, 1.0),
            ("1.5E-", 5, 1.5),
            ("1.5e+", 5, 1.5),
            ("-2.5e", 5, -2.5),
            ("0e", 2, 0.0),
            (".5e-", 4, 0.5),
        ] {
            let cs = cstr(s);
            for l in [&p.c, &p.rs] {
                let mut e: *mut c_char = std::ptr::null_mut();
                let val = l.js_strtod(cs.as_ptr(), &mut e);
                assert_eq!(
                    e.offset_from(cs.as_ptr() as *mut c_char),
                    want_off,
                    "{}: js_strtod({s:?}) endPtr past the bogus exponent",
                    l.name
                );
                assert_eq!(
                    val.to_bits(),
                    want_val.to_bits(),
                    "{}: js_strtod({s:?}) value",
                    l.name
                );
            }
        }
    }
}

/// Row 470: a mantissa with more than 18 significant digits is clamped to 18
/// (`fracExp = decPt - 18`), so the extra digits are silently dropped.
#[test]
fn t_strtod_18_digit_mantissa_clamp() {
    let mut v: Vec<String> = vec![];
    for n in 1..=40usize {
        v.push("1".repeat(n));
        v.push("9".repeat(n));
        v.push(format!("-{}", "7".repeat(n)));
        v.push(format!("0.{}", "1".repeat(n)));
        v.push(format!("{}.{}", "1".repeat(n), "2".repeat(n)));
        v.push(format!("{}e10", "1".repeat(n)));
        v.push(format!("{}e-10", "1".repeat(n)));
        v.push(format!("{}.5", "3".repeat(n)));
        v.push(format!(".{}5", "0".repeat(n)));
        v.push(format!("{}{}", "1".repeat(n), "0".repeat(n)));
    }
    // exactly at the 18/19 digit boundary
    for s in [
        "123456789012345678", "1234567890123456789", "12345678901234567890",
        "123456789012345678.9", "1.23456789012345678", "1.234567890123456789",
        "0.000000000000000000123456789012345678901234",
        "999999999999999999", "9999999999999999999",
    ] {
        v.push(s.to_string());
    }
    let mut rng = Rng::new(0x5EED_000F);
    for _ in 0..2500 {
        let n = 1 + rng.below(30) as usize;
        let digits: String = (0..n)
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        let dot = rng.below(n as u32 + 1) as usize;
        let (a, b) = digits.split_at(dot);
        v.push(format!("{a}.{b}"));
        v.push(digits.clone());
        v.push(format!("{digits}e{}", rng.range(-40, 40)));
    }
    for s in &v {
        diff_strtod(s);
        // ... and through a JS number literal
        if s.chars().all(|c| c.is_ascii_digit() || c == '.') && !s.starts_with('0') {
            diff_dostring(0, &format!("print({s})"));
        }
    }
}

/// Rows 473 / 474 / 475: the exponent guards.  `exp` stops accumulating at
/// `INT_MAX/100` and the remaining digits are skipped; a combined exponent
/// below `-maxExponent` (511) or above it is clamped to 511 with
/// `errno = ERANGE`, giving 0 / +-inf.
#[test]
fn t_strtod_exponent_clamping() {
    let mut v: Vec<String> = vec![];
    // rows 474 / 475: the +-511 boundary, exactly
    for e in [
        -520i64, -519, -518, -517, -516, -515, -514, -513, -512, -511, -510,
        -509, -400, -324, -323, -308, -1, 0, 1, 308, 309, 323, 324, 400, 509,
        510, 511, 512, 513, 514, 515, 516, 517, 518, 519, 520,
    ] {
        for m in ["1", "9", "1.5", "0.1", "12345", "-1", "-9.9", "0"] {
            v.push(format!("{m}e{e}"));
            v.push(format!("{m}E{e}"));
            if e >= 0 {
                v.push(format!("{m}e+{e}"));
            }
        }
    }
    // row 473: exponent accumulation stops at INT_MAX/100 = 21474836
    for e in [
        "21474835", "21474836", "21474837", "99999999", "999999999",
        "2147483647", "2147483648", "4294967296", "99999999999999999999",
        "1".to_string().repeat(30).as_str(),
    ] {
        for m in ["1", "-1", "0", "9.5"] {
            v.push(format!("{m}e{e}"));
            v.push(format!("{m}e-{e}"));
            v.push(format!("{m}e+{e}"));
        }
    }
    // leading zeros in the exponent do not count toward the guard
    for e in ["000000000000000000001", "0000000000512", "00000000000000000000"] {
        v.push(format!("1e{e}"));
        v.push(format!("1e-{e}"));
    }
    // the fractional part contributes to the combined exponent too
    v.push(format!("0.{}1e520", "0".repeat(30)));
    v.push(format!("{}e-520", "1".repeat(30)));
    v.push(format!("0.{}1e-520", "0".repeat(30)));
    for s in &v {
        diff_strtod(s);
    }
    // pin the clamped results.  `exp` is forced to 511 with `errno = ERANGE`,
    // and 10^511 itself already overflows a double, so `dblExp` is +inf: the
    // overflow branch multiplies by inf and the underflow branch divides by it.
    let p = libs();
    unsafe {
        for (s, want) in [
            ("1e512", f64::INFINITY),
            ("1e99999", f64::INFINITY),
            ("-1e512", f64::NEG_INFINITY),
            ("9.9e520", f64::INFINITY),
            ("1e-512", 0.0),
            ("1e-99999", 0.0),
            ("-1e-512", -0.0),
            ("-9.9e-520", -0.0),
        ] {
            let cs = cstr(s);
            for l in [&p.c, &p.rs] {
                let got = l.js_strtod(cs.as_ptr(), std::ptr::null_mut());
                assert_eq!(
                    got.to_bits(),
                    want.to_bits(),
                    "{}: js_strtod({s:?}) exponent clamp",
                    l.name
                );
            }
        }
        // a ZERO mantissa with a clamped positive exponent becomes 0 * inf = NaN
        // (jsdtoa.c:737): the clamp is applied before the mantissa is looked at
        for s in ["0e512", "0e99999", "-0e512", "0.0e600"] {
            let cs = cstr(s);
            let a = p.c.js_strtod(cs.as_ptr(), std::ptr::null_mut());
            let b = p.rs.js_strtod(cs.as_ptr(), std::ptr::null_mut());
            assert!(a.is_nan(), "C: js_strtod({s:?}) must be NaN, got {a}");
            assert_eq!(a.to_bits(), b.to_bits(), "js_strtod({s:?})");
        }
        // ... while a zero mantissa with a clamped NEGATIVE exponent is 0 / inf = 0
        for (s, want) in [("0e-512", 0.0f64), ("-0e-512", -0.0f64)] {
            let cs = cstr(s);
            for l in [&p.c, &p.rs] {
                let got = l.js_strtod(cs.as_ptr(), std::ptr::null_mut());
                assert_eq!(
                    got.to_bits(),
                    want.to_bits(),
                    "{}: js_strtod({s:?})",
                    l.name
                );
            }
        }
    }
    // and through JS number literals (lexnumber calls js_strtod(s, NULL))
    for s in [
        "1e511", "1e512", "1e513", "1e-511", "1e-512", "1e-513", "1e99999",
        "1e-99999", "1e21474836", "1e21474837", "1e2147483647",
        "0.00001e520", "12345678901234567890e-520",
    ] {
        diff_dostring(0, &format!("print({s})"));
        diff_dostring(JS_STRICT, &format!("print({s})"));
        diff_all(&format!("print({s})"));
        diff_json(s);
    }
}

/// Row 476 in its own right, plus the two in-tree `endPtr == NULL` callers
/// (`lexnumber`, jslex.c:383, and `lexjsonnumber`, jslex.c:780): a partial
/// conversion is invisible to them, so trailing garbage that the *lexer*
/// already rejected can never reach `js_strtod`, while trailing characters the
/// lexer accepted are simply ignored by the conversion.
#[test]
fn t_strtod_null_endptr_callers() {
    let mut v: Vec<String> = vec![];
    for s in [
        "1", "1.5", "1e5", ".5", "5.", "0", "0.0", "1e308", "1e309", "1e-400",
        "9007199254740993", "0.1", "1e511", "1e512",
    ] {
        v.push(s.to_string());
    }
    let mut rng = Rng::new(0x5EED_0010);
    for _ in 0..1500 {
        let int: String = (0..1 + rng.below(18))
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        let frac: String = (0..rng.below(18))
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        let e = rng.range(-600, 600);
        v.push(format!("{int}.{frac}e{e}"));
        v.push(format!("{int}.{frac}"));
        v.push(int.clone());
    }
    for s in &v {
        diff_strtod(s);
        let lit = s.trim_start_matches('0');
        let lit = if lit.starts_with('.') || lit.is_empty() {
            format!("0{lit}")
        } else {
            lit.to_string()
        };
        diff_dostring(0, &format!("print({lit})"));
        diff_json(s);
    }
}

/* ===================================================================== */
/*  Cross-cutting: the iseval axis and randomised syntax fuzzing          */
/* ===================================================================== */

/// `js_loadstringx`'s `iseval` flag (jsstate.c:111-127) selects
/// `J->strict` instead of `J->default_strict` and a different scope, so the
/// same source can compile differently through `js_loadeval` than through
/// `js_loadstring`.  Run the whole strict-mode error surface through both.
#[test]
fn t_iseval_axis() {
    let srcs = [
        "with(o){}", "var eval=1", "var arguments=1", "delete x",
        "function f(a,a){}", "({a:1,a:2})", "var let", "eval=1",
        "arguments=1", "'use strict'; with(o){}",
        "'use strict'; var eval=1", "eval('1')", "eval('with(o){}')",
        "eval('var eval=1')", "function f(){ return eval('1') } f()",
        "(function(){ 'use strict'; return eval('var x=1') })()",
        "1", "1+1", "@", "1e", "var 1", "return", "break", "1=2",
    ];
    for src in srcs {
        for flags in [0, JS_STRICT] {
            diff_leval(flags, "(le)", src);
            diff_leval(flags, "", src);
            diff_leval(flags, "deep/name.js", src);
            diff_pload(flags, "(le)", src);
            diff_dorun(flags, src);
            diff_prun(flags, "(pl)", src);
        }
    }
}

/// Rows 417 / 420 / 426 in detail: the local slot that `addlocal(reuse=1)`
/// returns, the backwards search in `findlocal` (so a later duplicate shadows an
/// earlier one) and the by-name fallback.  The slot numbers themselves are only
/// observable through the values the compiled code reads back, so run the
/// scripts and compare the results.
#[test]
fn t_local_slot_allocation() {
    let mut srcs: Vec<String> = vec![];
    for s in [
        "function f(a,b,c){var c,b,a; return [a,b,c].join(',')} print(f(1,2,3))",
        "function f(a){var a; return a} print(f(7))",
        "function f(a){var a=9; return a} print(f(7))",
        "function f(){var a=1; var a=2; return a} print(f())",
        "function f(){var a=1; function a(){}; return typeof a} print(f())",
        "function f(){function a(){}; var a=1; return typeof a} print(f())",
        "function f(a,b){function a(){}; return typeof a} print(f(1,2))",
        "function f(){var a; return a} print(f())",
        "var a=1; function g(){return a} print(g())",
        "function f(){return notalocal} print(typeof f)",
        "function f(){notalocal=1; return notalocal} print(f())",
        "function f(x){ if(x) { var y=1 } return y } print(f(1), f(0))",
        "function f(){ for(var i=0;i<3;++i){} return i } print(f())",
        "function f(){ for(var k in {a:1}){} return k } print(f())",
        "function f(){ try{throw 1}catch(e){var e2=e} return e2 } print(f())",
        "function f(){ with({a:5}){ return a } } print(f())",
        "print((function q(){ return typeof q })())",
        "print((function q(q){ return typeof q })(1))",
        "print((function q(){ var q=2; return q })())",
        "print(function q(){}.name)",
    ] {
        srcs.push(s.to_string());
    }
    // many locals, so the slot indices cross the 16/32/64 doubling boundaries
    for n in [15usize, 16, 17, 32, 33, 64, 65, 100] {
        let decls = (0..n)
            .map(|i| format!("v{i}={i}"))
            .collect::<Vec<_>>()
            .join(",");
        srcs.push(format!(
            "function f(){{var {decls}; return v0+v{}}} print(f())",
            n - 1
        ));
        let params = (0..n).map(|i| format!("p{i}")).collect::<Vec<_>>().join(",");
        let args = (0..n).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        srcs.push(format!(
            "function f({params}){{return p0+p{}}} print(f({args}))",
            n - 1
        ));
    }
    for s in &srcs {
        diff_all(s);
    }
}

/// Row 429 at scale, and row 452's `F->script`-dependent `OP_POP` ordering: the
/// completion value of a script is observable through `js_pcall`, so compare it
/// for every statement shape.
#[test]
fn t_checkdup_at_scale_and_completion_values() {
    let mut srcs: Vec<String> = vec![];
    // checkdup walks the whole prefix for every property
    for n in [1usize, 2, 8, 32, 64] {
        let uniq = (0..n)
            .map(|i| format!("k{i}:{i}"))
            .collect::<Vec<_>>()
            .join(",");
        srcs.push(format!("({{{uniq}}})"));
        srcs.push(format!("({{{uniq},k0:99}})"));
        let nums = (0..n)
            .map(|i| format!("{i}:{i}"))
            .collect::<Vec<_>>()
            .join(",");
        srcs.push(format!("({{{nums}}})"));
        srcs.push(format!("({{{nums},'{}':9}})", n - 1));
        let mixed = (0..n)
            .map(|i| {
                if i % 2 == 0 {
                    format!("{i}:{i}")
                } else {
                    format!("'{i}':{i}")
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        srcs.push(format!("({{{mixed}}})"));
    }
    // completion values (row 452)
    for s in [
        "1", "1;2;3", "1;;", ";", "{}", "{1}", "{1;2}", "var x=1", "var x=1;2",
        "if(1)1", "if(0)1", "if(1)1;else 2", "while(0)1", "do 1;while(0)",
        "for(var i=0;i<2;++i)i", "for(var k in {a:1,b:2})k",
        "for(k in {a:1})k", "switch(1){case 1:5}", "switch(1){default:5}",
        "switch(2){case 1:5}", "try{1}catch(e){2}", "try{throw 1}catch(e){2}",
        "try{1}finally{2}", "try{throw 1}catch(e){3}finally{4}",
        "with({a:1})a", "L:{1}", "L:while(1){break L}",
        "function f(){} 1", "function f(){}", "'s'", "/re/", "[1,2]",
        "({a:1})", "typeof x", "(1,2)", "1&&2", "0||3", "1?2:3", "-1", "!0",
        "x=5", "x=5;x", "new Object", "(function(){return 9})()",
        "debugger", "throw 1", "1;throw 2",
    ] {
        srcs.push(s.to_string());
    }
    for s in &srcs {
        diff_all(s);
    }
}

/// Row 297 with raw, possibly-invalid UTF-8 input.  `jsY_next` decodes with
/// `chartorune`, which maps an ill-formed sequence to `Runeerror` (U+FFFD) and
/// consumes one byte, so the resulting `"unexpected character: \\u%04X"` and the
/// byte position both have to agree.
#[test]
fn t_raw_byte_sources() {
    let p = libs();
    fn cbytes(b: &[u8]) -> CString {
        CString::new(b.iter().copied().filter(|c| *c != 0).collect::<Vec<u8>>()).unwrap()
    }
    let mut srcs: Vec<Vec<u8>> = vec![];
    // every single high byte on its own, and after a token
    for b in 0x80u8..=0xff {
        srcs.push(vec![b]);
        srcs.push(vec![b'1', b]);
        srcs.push(vec![b'a', b]);
        srcs.push(vec![b'\'', b, b'\'']);
        srcs.push(vec![b'/', b, b'/']);
        srcs.push(vec![b'v', b'a', b'r', b' ', b]);
    }
    // truncated / overlong / surrogate sequences
    for seq in [
        &[0xc2u8][..], &[0xc2, 0x41], &[0xe4, 0xb8][..], &[0xe4, 0xb8, 0x41],
        &[0xf0, 0x9f][..], &[0xf0, 0x9f, 0x98], &[0xc0, 0x80], &[0xe0, 0x80, 0x80],
        &[0xed, 0xa0, 0x80], &[0xed, 0xbf, 0xbf], &[0xf4, 0x90, 0x80, 0x80],
        &[0xf8, 0x88, 0x80, 0x80, 0x80], &[0xfe], &[0xff], &[0x80], &[0xbf],
    ] {
        srcs.push(seq.to_vec());
        let mut v = b"'".to_vec();
        v.extend_from_slice(seq);
        v.push(b'\'');
        srcs.push(v);
        let mut v = b"JSONISH".to_vec();
        v.extend_from_slice(seq);
        srcs.push(v);
    }
    let mut rng = Rng::new(0x5EED_0011);
    for _ in 0..2500 {
        srcs.push(rng.raw_bytes(9));
    }
    unsafe {
        for bytes in &srcs {
            let cs = cbytes(bytes);
            for flags in [0, JS_STRICT] {
                let mut res: Vec<Load> = vec![];
                for l in [&p.c, &p.rs] {
                    out_clear();
                    let j = new_state(l, flags);
                    let rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
                    let ty = from_c(l.js_typeof(j, -1));
                    let msg = if rc != 0 {
                        from_c(l.js_trystring(j, -1, ERRSTR))
                    } else {
                        String::new()
                    };
                    l.js_pop(j, 1);
                    let top = l.js_gettop(j);
                    l.js_freestate(j);
                    res.push(Load {
                        rc,
                        ty,
                        msg,
                        top,
                        out: out_take(),
                    });
                }
                assert_eq!(
                    res[0], res[1],
                    "raw-byte source divergence (flags={flags}) bytes={bytes:02x?}"
                );
            }
            // and through the JSON lexer
            for flags in [0, JS_STRICT] {
                let mut got: Vec<Load> = vec![];
                for l in [&p.c, &p.rs] {
                    out_clear();
                    let j = new_state(l, flags);
                    l.js_getglobal(j, N_JSON);
                    l.js_getproperty(j, -1, N_PARSE);
                    l.js_copy(j, -2);
                    l.js_pushstring(j, cs.as_ptr());
                    let rc = l.js_pcall(j, 1);
                    let ty = from_c(l.js_typeof(j, -1));
                    let msg = from_c(l.js_trystring(j, -1, ERRSTR));
                    l.js_pop(j, 1);
                    let top = l.js_gettop(j);
                    l.js_freestate(j);
                    got.push(Load {
                        rc,
                        ty,
                        msg,
                        top,
                        out: out_take(),
                    });
                }
                assert_eq!(
                    got[0], got[1],
                    "raw-byte JSON divergence (flags={flags}) bytes={bytes:02x?}"
                );
            }
        }
    }
}

/// `lexescape`'s `\0`, `\x00` and `\u0000` all `textpush` a NUL byte, which
/// terminates the interned token text early -- an observable side effect of the
/// same code path that rows 280-285 fail on.
#[test]
fn t_nul_producing_escapes() {
    let mut srcs: Vec<String> = vec![];
    for e in ["\\0", "\\x00", "\\u0000"] {
        for body in [
            format!("{e}"),
            format!("a{e}"),
            format!("{e}b"),
            format!("a{e}b"),
            format!("{e}{e}"),
        ] {
            srcs.push(format!("print('{body}'.length)"));
            srcs.push(format!("print('{body}')"));
            srcs.push(format!("print(JSON.stringify('{body}'))"));
            srcs.push(format!("var o={{}}; o['{body}']=1; print(Object.keys(o).length)"));
            srcs.push(format!("print(('{body}').charCodeAt(0))"));
        }
    }
    // and the JSON side (\u0000 is a legal JSON escape)
    for s in &srcs {
        diff_all(s);
    }
    for t in ["\"\\u0000\"", "\"a\\u0000b\"", "\"\\u0000\\u0000\""] {
        diff_json(t);
    }
}

/// Line accounting for tokens that themselves span several lines: a block
/// comment, a `\<newline>` string continuation, and the virtual `';'` that ASI
/// inserts (whose `J->lexline` is the line BEFORE the newline, because
/// `jsY_lexx` saves `lexline` at the top of the loop and only then consumes the
/// terminator).
#[test]
fn t_lexline_for_multiline_tokens() {
    let p = libs();
    let cases: &[(&str, &str)] = &[
        // block comment spanning 3 lines, then the offending character
        ("/*\n\n*/@", "SyntaxError: t.js:3: unexpected character: '@'"),
        ("/*\r\n\r\n*/@", "SyntaxError: t.js:3: unexpected character: '@'"),
        ("/*\u{2028}\u{2029}*/@", "SyntaxError: t.js:3: unexpected character: '@'"),
        // string with a line continuation
        ("'a\\\nb'\n@", "SyntaxError: t.js:3: unexpected character: '@'"),
        ("'a\\\nb\\\nc'@", "SyntaxError: t.js:3: unexpected character: '@'"),
        // the unterminated-comment error itself is reported at the START line
        ("\n\n/*abc", "SyntaxError: t.js:3: multi-line comment not terminated"),
        ("\n\n/*a\nb\nc", "SyntaxError: t.js:3: multi-line comment not terminated"),
        // ASI: the virtual ';' carries the line AFTER the terminator that
        // triggered it, because `jsY_next` had already read (and counted) that
        // terminator while finishing the `throw` token, so
        // `J->lexline = J->line` at the top of `jsY_lexx` already sees the
        // incremented value
        ("throw\n1", "SyntaxError: t.js:2: unexpected token in expression: ';'"),
        ("throw\n\n\n1", "SyntaxError: t.js:2: unexpected token in expression: ';'"),
        ("\n\nthrow\n1", "SyntaxError: t.js:4: unexpected token in expression: ';'"),
        ("throw\r\n1", "SyntaxError: t.js:2: unexpected token in expression: ';'"),
        ("throw\u{2028}1", "SyntaxError: t.js:2: unexpected token in expression: ';'"),
        // a multi-line string literal error is reported where the token started
        ("\n'a\nb'", "SyntaxError: t.js:2: string not terminated"),
        ("\n/a\nb/", "SyntaxError: t.js:2: regular expression not terminated"),
    ];
    for (src, want) in cases {
        for l in [&p.c, &p.rs] {
            assert_eq!(pload(l, 0, "t.js", src).msg, *want, "{}: {src:?}", l.name);
        }
        diff_all(src);
    }
    // exhaustive: each terminator form, each count, for the multi-line comment
    for term in ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
        for n in 0..8 {
            for src in [
                format!("/*{}*/@", term.repeat(n)),
                format!("{}/*abc", term.repeat(n)),
                format!("'a\\{}b'@", term),
                format!("//x{}@", term),
            ] {
                diff_pload(0, "t.js", &src);
                diff_pload(JS_STRICT, "t.js", &src);
            }
        }
    }
}

/// Randomised mixed nesting right at `JS_ASTLIMIT`, so the interaction between
/// the 17 `INCREC` sites (and the `SAVEREC` / `POPREC` restores) is compared as
/// well as each site on its own.
fn body_t_mixed_nesting_fuzz() {
    let wrappers: &[(&str, &str)] = &[
        ("(", ")"),
        ("[", "]"),
        ("!", ""),
        ("~", ""),
        ("-", ""),
        ("typeof ", ""),
        ("f(", ")"),
        ("a[", "]"),
        ("1+", ""),
        ("1*", ""),
        ("1&&", ""),
        ("1||", ""),
        ("1?", ":1"),
        ("x=", ""),
        ("{a:", "}"),
        ("new f(", ")"),
    ];
    let mut rng = Rng::new(0x5EED_0012);
    for _ in 0..500 {
        let depth = rng.range(180, 260) as usize;
        let mut pre = String::new();
        let mut post = String::new();
        for _ in 0..depth {
            let (a, b) = wrappers[rng.below(wrappers.len() as u32) as usize];
            pre.push_str(a);
            post.insert_str(0, b);
        }
        let src = format!("({pre}1{post})");
        for flags in [0, JS_STRICT] {
            diff_pload(flags, "test.js", &src);
        }
    }
    // statement nesting mixed with expression nesting
    let stmt: &[(&str, &str)] = &[
        ("{", "}"),
        ("if(1)", ""),
        ("while(0)", ""),
        ("for(;0;)", ""),
        ("with(o)", ""),
        ("L:", ""),
        ("try{", "}catch(e){}"),
        ("switch(1){case 1:", "}"),
        ("do ", " while(0)"),
    ];
    for _ in 0..500 {
        let depth = rng.range(180, 260) as usize;
        let mut pre = String::new();
        let mut post = String::new();
        for _ in 0..depth {
            let (a, b) = stmt[rng.below(stmt.len() as u32) as usize];
            pre.push_str(a);
            post.insert_str(0, b);
        }
        let src = format!("{pre};{post}");
        for flags in [0, JS_STRICT] {
            diff_pload(flags, "test.js", &src);
        }
    }
}

#[test]
fn t_mixed_nesting_fuzz() {
    with_big_stack(body_t_mixed_nesting_fuzz);
}

/// Randomised syntax fuzzing over a token alphabet, with a fixed seed.  Only
/// `js_ploadstring` is used, so a source that would loop forever is harmless.
#[test]
fn t_syntax_fuzz() {
    let toks = [
        "1", "0x1f", "01", "1e", ".5", "'s'", "\"s\"", "'\\u", "a", "eval",
        "arguments", "class", "let", "+", "-", "*", "/", "%", "(", ")", "{",
        "}", "[", "]", ";", ",", ".", "?", ":", "=", "==", "===", "!", "<",
        ">", "<=", ">=", "&&", "||", "++", "--", "~", "&", "|", "^", "<<",
        ">>", ">>>", "+=", "var", "function", "if", "else", "for", "while",
        "do", "return", "break", "continue", "new", "typeof", "delete",
        "void", "in", "instanceof", "this", "null", "true", "false", "try",
        "catch", "finally", "throw", "switch", "case", "default", "with",
        "debugger", "/re/", "/re/gg", "//c\n", "/*c*/", "/*c", "\n", "\r\n",
        " ", "\t", "\\u0041", "\\uZZZZ", "$", "_", "@", "#", "`",
        "\u{2028}", "\u{e9}",
    ];
    let mut rng = Rng::new(0x5EED_C0DE);
    for _ in 0..7000 {
        let n = 1 + rng.below(12) as usize;
        let src: String = (0..n)
            .map(|_| toks[rng.below(toks.len() as u32) as usize])
            .collect::<Vec<_>>()
            .join("");
        for flags in [0, JS_STRICT] {
            diff_pload(flags, "test.js", &src);
        }
        if rng.below(4) == 0 {
            diff_pload(0, "", &src);
            diff_leval(0, "(le)", &src);
            diff_json(&src);
        }
    }
}
