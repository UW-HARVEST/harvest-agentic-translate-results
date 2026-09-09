//! The variadic error entry points (`js_error`, `js_typeerror`, ... and the
//! internal `jsC_error`) are the only exported symbols that no other test
//! calls directly. They format into a fixed 256-byte buffer with `snprintf`
//! and then throw, so both the formatting (including truncation) and the
//! throw must match the C library exactly.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::null_mut;

extern "C" {
    fn setvbuf(f: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    static mut stdout: *mut c_void;
}

static mut CUR: *const Api = std::ptr::null();
static mut WHICH: c_int = 0;

static NAME: &[u8] = b"thrower\0";
static LONGARG: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\0";

/// Throws through one of the 7 variadic error functions, selected by WHICH.
unsafe extern "C" fn cf_throw(J: JS) {
    let api = &*CUR;
    let fmt_s = cs("'%s' is bad: %d %c %x %5.2f [%s]");
    let a1 = cs("name");
    match WHICH {
        0 => (api.js_error)(J, fmt_s.as_ptr(), a1.as_ptr(), 42, 'q' as c_int, 255, 1.5, LONGARG.as_ptr()),
        1 => (api.js_evalerror)(J, fmt_s.as_ptr(), a1.as_ptr(), 42, 'q' as c_int, 255, 1.5, LONGARG.as_ptr()),
        2 => (api.js_rangeerror)(J, fmt_s.as_ptr(), a1.as_ptr(), 42, 'q' as c_int, 255, 1.5, LONGARG.as_ptr()),
        3 => (api.js_referenceerror)(J, fmt_s.as_ptr(), a1.as_ptr(), 42, 'q' as c_int, 255, 1.5, LONGARG.as_ptr()),
        4 => (api.js_syntaxerror)(J, fmt_s.as_ptr(), a1.as_ptr(), 42, 'q' as c_int, 255, 1.5, LONGARG.as_ptr()),
        5 => (api.js_typeerror)(J, fmt_s.as_ptr(), a1.as_ptr(), 42, 'q' as c_int, 255, 1.5, LONGARG.as_ptr()),
        6 => (api.js_urierror)(J, fmt_s.as_ptr(), a1.as_ptr(), 42, 'q' as c_int, 255, 1.5, LONGARG.as_ptr()),
        /* no-argument format strings */
        7 => (api.js_error)(J, cs("plain message").as_ptr()),
        8 => (api.js_typeerror)(J, cs("").as_ptr()),
        9 => (api.js_rangeerror)(J, cs("%%%s%%").as_ptr(), cs("mid").as_ptr()),
        /* a format whose expansion exceeds the 256-byte buffer */
        _ => (api.js_error)(J, cs("%s%s").as_ptr(), LONGARG.as_ptr(), LONGARG.as_ptr()),
    }
}

fn probe(which: c_int) {
    diff(&format!("variadic_{}", which), move |api| unsafe {
        setvbuf(stdout, null_mut(), 2, 0);
        CUR = api as *const Api;
        WHICH = which;
        let J = newstate(api, 0);
        (api.js_newcfunction)(J, Some(cf_throw), NAME.as_ptr() as *const c_char, 0);
        (api.js_pushundefined)(J);
        let rc = (api.js_pcall)(J, 0);
        p_int("which", which);
        p_int("rc", rc);
        p_str("err", (api.js_trystring)(J, -1, cs("?").as_ptr()));
        (api.js_getproperty)(J, -1, cs("name").as_ptr());
        p_str("name", (api.js_trystring)(J, -1, cs("?").as_ptr()));
        (api.js_pop)(J, 1);
        (api.js_getproperty)(J, -1, cs("message").as_ptr());
        p_str("message", (api.js_trystring)(J, -1, cs("?").as_ptr()));
        (api.js_pop)(J, 2);
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

#[test]
fn variadic_error_functions() {
    for w in 0..=10 {
        probe(w);
    }
}

/// `jsC_error(J, node, fmt, ...)` — the compiler's error helper. It reads
/// `node->line`, so it needs a real AST node; parse a script first.
#[test]
fn jsC_error_direct() {
    diff("jsC_error_direct", |api| unsafe {
        setvbuf(stdout, null_mut(), 2, 0);
        let J = newstate(api, 0);
        let src = cs("var a = 1;\nvar b = 2;\nvar c = 3;\n");
        let file = cs("cerr.js");
        let rc = (api.js_ploadstring)(J, file.as_ptr(), src.as_ptr());
        p_int("preload_rc", rc);
        (api.js_pop)(J, 1);
        let ast = (api.jsP_parse)(J, file.as_ptr(), src.as_ptr());
        p_ptr_nonnull("ast", ast);
        /* throws out of the library: no try frame -> panic -> abort, in both */
        (api.jsC_error)(J, ast, cs("bad thing %s %d").as_ptr(), cs("here").as_ptr(), 7);
        p_line("unreachable");
    });
}

/// Non-vacuity guard: the probes above must really produce the formatted
/// messages (a silently-empty child would make `diff` pass trivially).
#[test]
fn variadic_output_is_substantive() {
    let l = libs();
    for w in 0..=10 {
        let (out, st) = capture(&format!("nv_{}", w), || unsafe {
            setvbuf(stdout, null_mut(), 2, 0);
            CUR = &l.c as *const Api;
            WHICH = w;
            let J = newstate(&l.c, 0);
            (l.c.js_newcfunction)(J, Some(cf_throw), NAME.as_ptr() as *const c_char, 0);
            (l.c.js_pushundefined)(J);
            let rc = (l.c.js_pcall)(J, 0);
            p_int("rc", rc);
            p_str("err", (l.c.js_trystring)(J, -1, cs("?").as_ptr()));
            (l.c.js_freestate)(J);
        });
        let s = String::from_utf8_lossy(&out).into_owned();
        assert_eq!(decode_status(st), "exit 0", "probe {} died: {}", w, s);
        assert!(s.contains("rc=1"), "probe {} did not throw: {}", w, s);
        assert!(
            s.lines().any(|l| l.starts_with("err=") && l.len() > 8),
            "probe {} produced no error message: {}",
            w,
            s
        );
    }
}
