// Level 3: regexp.c -- standalone regex engine (js_regcomp/js_regexec/js_regfree).
mod common;

use common::both;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

const REG_MAXSUB: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
struct ResubEntry {
    sp: *const c_char,
    ep: *const c_char,
}

#[repr(C)]
struct Resub {
    nsub: c_int,
    sub: [ResubEntry; REG_MAXSUB],
}

impl Resub {
    fn new() -> Self {
        Resub {
            nsub: -999,
            sub: [ResubEntry {
                sp: std::ptr::null(),
                ep: std::ptr::null(),
            }; REG_MAXSUB],
        }
    }
}

type RegcompFn = unsafe extern "C-unwind" fn(*const c_char, c_int, *mut *const c_char) -> *mut c_void;
type RegexecFn = unsafe extern "C-unwind" fn(*mut c_void, *const c_char, *mut Resub, c_int) -> c_int;
type RegfreeFn = unsafe extern "C-unwind" fn(*mut c_void);

fn patterns() -> Vec<&'static str> {
    vec![
        "",
        "a",
        "abc",
        "a*",
        "a+",
        "a?",
        "a{2}",
        "a{2,}",
        "a{2,4}",
        "a{0,0}",
        "(a)",
        "(a)(b)",
        "(a|b)",
        "(?:ab)",
        "(?=a)",
        "(?!a)",
        "a|b|c",
        "^abc$",
        "^a",
        "a$",
        ".",
        ".*",
        "[abc]",
        "[^abc]",
        "[a-z]",
        "[a-zA-Z0-9_]",
        "[]a]",
        "[^]a]",
        "[-a]",
        "[a-]",
        "\\d",
        "\\D",
        "\\w",
        "\\W",
        "\\s",
        "\\S",
        "\\b",
        "\\B",
        "\\n",
        "\\t",
        "\\r",
        "\\f",
        "\\v",
        "\\0",
        "\\x41",
        "\\u0041",
        "\\cA",
        "\\.",
        "\\\\",
        "\\/",
        "(a)\\1",
        "(a)(b)\\2\\1",
        "a(?:b|c)d",
        "(a+)+",
        "(a*)*b",
        "[\\d-a]",
        "[\\w]",
        "[\\b]",
        "a{,3}",
        "a{",
        "a}",
        "*a",
        "+a",
        "?a",
        "(",
        ")",
        "[",
        "[]",
        "[^]",
        "\\",
        "a**",
        "a*?",
        "a+?",
        "a??",
        "a{1,2}?",
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)",
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)(q)",
        "(?:)",
        "^",
        "$",
        "^$",
        "x(?=y)z",
        "[a-c]|[e-g]",
        "(\\w+)\\s(\\w+)",
        "colou?r",
        "([0-9]{4})-([0-9]{2})-([0-9]{2})",
        "\\u00e9",
        "é",
        "[é-ü]",
        "\\uD83D\\uDE00",
        "(?<a>x)",
        "a|",
        "|a",
        "||",
        "[z-a]",
        "\\p{L}",
        "\\Q",
        "(?i)a",
        "[[:alpha:]]",
        "a\\",
        "\\1",
        "(a)|\\1",
        ".{1,}",
        "[^\\d]",
        "[\\s\\S]",
        "\\$\\^\\*\\+\\?\\(\\)\\[\\]\\{\\}\\|",
    ]
}

fn subjects() -> Vec<&'static str> {
    vec![
        "",
        "a",
        "b",
        "aa",
        "ab",
        "abc",
        "abcabc",
        "aaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaa",
        "xyz",
        "\n",
        "a\nb",
        "\r\n",
        "hello world",
        "2024-01-31",
        "Colour and color",
        "  spaces  ",
        "\t\ttabs",
        "AaBbCc",
        "0123456789",
        "_under_score",
        "é",
        "éü",
        "日本語",
        "\u{1F600}",
        "aab",
        "xyz-abc",
        "$^*+?()[]{}|",
        "\\",
        "xz",
        "xyz",
        "aXbXc",
        "The quick brown fox",
        "-",
        "]",
        "^",
        "abcdefghijklmnopqrstuvwxyz",
        "z",
    ]
}

#[test]
fn regcomp_error_messages_match() {
    let (cc, rc) = unsafe { both::<RegcompFn>("js_regcomp") };
    let (cf, rf) = unsafe { both::<RegfreeFn>("js_regfree") };
    for cflags in [0i32, 1, 2, 3, 4, 7] {
        for p in patterns() {
            let cp = CString::new(p).unwrap();
            let mut ec: *const c_char = std::ptr::null();
            let mut er: *const c_char = std::ptr::null();
            let pc = unsafe { cc(cp.as_ptr(), cflags, &mut ec) };
            let pr = unsafe { rc(cp.as_ptr(), cflags, &mut er) };
            let msgc = unsafe { common::cstr_to_string(ec) };
            let msgr = unsafe { common::cstr_to_string(er) };
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "regcomp({:?},{}) success differs: C err={:?} Rust err={:?}",
                p,
                cflags,
                msgc,
                msgr
            );
            assert_eq!(
                msgc, msgr,
                "regcomp({:?},{}) error message differs",
                p, cflags
            );
            if !pc.is_null() {
                unsafe { cf(pc) };
            }
            if !pr.is_null() {
                unsafe { rf(pr) };
            }
        }
    }
}

#[test]
fn regexec_matches() {
    let (cc, rc) = unsafe { both::<RegcompFn>("js_regcomp") };
    let (ce, re) = unsafe { both::<RegexecFn>("js_regexec") };
    let (cf, rf) = unsafe { both::<RegfreeFn>("js_regfree") };

    let subs = subjects();
    for cflags in [0i32, 1, 2, 3] {
        for p in patterns() {
            let cp = CString::new(p).unwrap();
            let mut ec: *const c_char = std::ptr::null();
            let mut er: *const c_char = std::ptr::null();
            let pc = unsafe { cc(cp.as_ptr(), cflags, &mut ec) };
            let pr = unsafe { rc(cp.as_ptr(), cflags, &mut er) };
            if pc.is_null() || pr.is_null() {
                if !pc.is_null() {
                    unsafe { cf(pc) };
                }
                if !pr.is_null() {
                    unsafe { rf(pr) };
                }
                continue;
            }
            for s in subs.iter() {
                let cs = CString::new(*s).unwrap();
                for eflags in [0i32, 4] {
                    let mut mc = Resub::new();
                    let mut mr = Resub::new();
                    let okc = unsafe { ce(pc, cs.as_ptr(), &mut mc, eflags) };
                    let okr = unsafe { re(pr, cs.as_ptr(), &mut mr, eflags) };
                    assert_eq!(
                        okc, okr,
                        "regexec result differs: pattern={:?} cflags={} subject={:?} eflags={}",
                        p, cflags, s, eflags
                    );
                    if okc == 0 {
                        assert_eq!(
                            mc.nsub, mr.nsub,
                            "nsub differs: pattern={:?} subject={:?}",
                            p, s
                        );
                        let base_c = cs.as_ptr() as usize;
                        for i in 0..(mc.nsub.max(0) as usize).min(REG_MAXSUB) {
                            let a = (
                                if mc.sub[i].sp.is_null() {
                                    None
                                } else {
                                    Some(mc.sub[i].sp as usize - base_c)
                                },
                                if mc.sub[i].ep.is_null() {
                                    None
                                } else {
                                    Some(mc.sub[i].ep as usize - base_c)
                                },
                            );
                            let b = (
                                if mr.sub[i].sp.is_null() {
                                    None
                                } else {
                                    Some(mr.sub[i].sp as usize - base_c)
                                },
                                if mr.sub[i].ep.is_null() {
                                    None
                                } else {
                                    Some(mr.sub[i].ep as usize - base_c)
                                },
                            );
                            assert_eq!(
                                a, b,
                                "sub[{}] differs: pattern={:?} cflags={} subject={:?} eflags={}",
                                i, p, cflags, s, eflags
                            );
                        }
                    }
                    // also exercise the NULL-Resub path
                    let okc2 = unsafe { ce(pc, cs.as_ptr(), std::ptr::null_mut(), eflags) };
                    let okr2 = unsafe { re(pr, cs.as_ptr(), std::ptr::null_mut(), eflags) };
                    assert_eq!(
                        okc2, okr2,
                        "regexec(NULL sub) differs: pattern={:?} subject={:?}",
                        p, s
                    );
                }
            }
            unsafe { cf(pc) };
            unsafe { rf(pr) };
        }
    }
}

/// js_regcompx / js_regfreex with a custom allocator.
#[test]
fn regcompx_matches() {
    type AllocFn = unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;
    type RegcompxFn = unsafe extern "C-unwind" fn(
        AllocFn,
        *mut c_void,
        *const c_char,
        c_int,
        *mut *const c_char,
    ) -> *mut c_void;
    type RegfreexFn = unsafe extern "C-unwind" fn(AllocFn, *mut c_void, *mut c_void);

    unsafe extern "C-unwind" fn myalloc(
        _ctx: *mut c_void,
        ptr: *mut c_void,
        n: c_int,
    ) -> *mut c_void {
        unsafe {
            if n == 0 {
                libc_free(ptr);
                return std::ptr::null_mut();
            }
            libc_realloc(ptr, n as usize)
        }
    }

    let (cc, rc) = unsafe { both::<RegcompxFn>("js_regcompx") };
    let (cf, rf) = unsafe { both::<RegfreexFn>("js_regfreex") };
    let (ce, re) = unsafe { both::<RegexecFn>("js_regexec") };

    for p in patterns() {
        let cp = CString::new(p).unwrap();
        let mut ec: *const c_char = std::ptr::null();
        let mut er: *const c_char = std::ptr::null();
        let pc = unsafe { cc(myalloc, std::ptr::null_mut(), cp.as_ptr(), 0, &mut ec) };
        let pr = unsafe { rc(myalloc, std::ptr::null_mut(), cp.as_ptr(), 0, &mut er) };
        assert_eq!(pc.is_null(), pr.is_null(), "regcompx({:?})", p);
        assert_eq!(
            unsafe { common::cstr_to_string(ec) },
            unsafe { common::cstr_to_string(er) },
            "regcompx({:?}) error",
            p
        );
        if !pc.is_null() {
            let s = CString::new("abcabc123 xyz").unwrap();
            let mut mc = Resub::new();
            let mut mr = Resub::new();
            let okc = unsafe { ce(pc, s.as_ptr(), &mut mc, 0) };
            let okr = unsafe { re(pr, s.as_ptr(), &mut mr, 0) };
            assert_eq!(okc, okr, "regexec after regcompx({:?})", p);
            unsafe { cf(myalloc, std::ptr::null_mut(), pc) };
            unsafe { rf(myalloc, std::ptr::null_mut(), pr) };
        }
    }
}

unsafe extern "C" {
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}
