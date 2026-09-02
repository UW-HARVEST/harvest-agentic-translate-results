//! Phase B rows 27-34: the standalone regexp engine (`js_regcomp*`,
//! `js_regexec`, `js_regfree*`) driven directly through the `.so` exports.
mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

const REG_ICASE: c_int = 1;
const REG_NEWLINE: c_int = 2;
const REG_NOTBOL: c_int = 4;

pub fn patterns() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "".into(),
        "a".into(),
        "abc".into(),
        "a|b".into(),
        "a|".into(),
        "|a".into(),
        "a*".into(),
        "a+".into(),
        "a?".into(),
        "a*?".into(),
        "a+?".into(),
        "a??".into(),
        "a{2}".into(),
        "a{2,}".into(),
        "a{2,4}".into(),
        "a{0,0}".into(),
        "a{0}".into(),
        "a{255}".into(),
        "a{2,4}?".into(),
        "(a)".into(),
        "(a)(b)".into(),
        "(?:a)".into(),
        "(?=a)".into(),
        "(?!a)".into(),
        "(a)\\1".into(),
        "(a)\\2".into(),
        "\\1".into(),
        "[abc]".into(),
        "[^abc]".into(),
        "[a-z]".into(),
        "[z-a]".into(),
        "[]".into(),
        "[^]".into(),
        "[-a]".into(),
        "[a-]".into(),
        "[\\d]".into(),
        "[\\D\\w\\W\\s\\S]".into(),
        "[\\b]".into(),
        "\\d".into(),
        "\\D".into(),
        "\\w".into(),
        "\\W".into(),
        "\\s".into(),
        "\\S".into(),
        "\\b".into(),
        "\\B".into(),
        ".".into(),
        "^a".into(),
        "a$".into(),
        "^$".into(),
        "^a$".into(),
        "\\n".into(),
        "\\r".into(),
        "\\t".into(),
        "\\f".into(),
        "\\v".into(),
        "\\0".into(),
        "\\x41".into(),
        "\\u0041".into(),
        "\\cA".into(),
        "\\.".into(),
        "\\\\".into(),
        "a.c".into(),
        "(a+)+b".into(),
        "(a|b)*c".into(),
        "((a)(b))".into(),
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)".into(),
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)".into(),
        "\u{e9}".into(),
        "[\u{e9}-\u{ff}]".into(),
        "\u{1f600}".into(),
        "(?:a|b|c|d)+".into(),
        "x(?=y)z".into(),
        "[a-c]{1,3}".into(),
        "(|a)*".into(),
        "()".into(),
        "()*".into(),
        "(a*)*".into(),
        "^(a|ab)+$".into(),
        "\\B\\w+\\B".into(),
        "a{1,2}{3}".into(),
        "[\\-]".into(),
        "[a\\]b]".into(),
        "[^\\n]".into(),
        "\\$".into(),
        "$a".into(),
        "a**".into(),
        "*a".into(),
        "+a".into(),
        "?a".into(),
        "{2}".into(),
        "(".into(),
        ")".into(),
        "[".into(),
        "a)".into(),
        "\\".into(),
        "\\x".into(),
        "\\x4".into(),
        "\\u00".into(),
        "\\c".into(),
        "a{2,1}".into(),
        "a{4294967296}".into(),
        "a{,2}".into(),
    ];
    let mut rng = Rng::new(0x2727);
    let alphabet: &[u8] = b"ab()[]{}|*+?.^$\\-,0123456789dDwWsSnb:=!";
    for _ in 0..4000 {
        let n = 1 + rng.below(12) as usize;
        v.push(
            (0..n)
                .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize] as char)
                .collect(),
        );
    }
    v
}

fn subjects() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "".into(),
        "a".into(),
        "b".into(),
        "aa".into(),
        "ab".into(),
        "abc".into(),
        "abcabc".into(),
        "aaaaa".into(),
        "xay".into(),
        "\n".into(),
        "a\nb".into(),
        "a\nb\nc".into(),
        "\na".into(),
        "a\n".into(),
        "ABC".into(),
        "AbC".into(),
        " a ".into(),
        "0123".into(),
        "a1b2".into(),
        "_a-b".into(),
        "\u{e9}".into(),
        "h\u{e9}llo".into(),
        "\u{1f600}a".into(),
        "\u{ff}\u{100}".into(),
        "aaaaaaaaaaaaaaaaaaaab".into(),
        "\t\r\u{b}\u{c} ".into(),
        "\u{0}".into(),
    ];
    let mut rng = Rng::new(0x2828);
    let alphabet: &[u8] = b"abcABC01 \n\t-_.";
    for _ in 0..600 {
        let n = rng.below(10) as usize;
        v.push(
            (0..n)
                .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize] as char)
                .collect(),
        );
    }
    v
}

struct Compiled {
    prog: *mut c_void,
    err: String,
}

unsafe fn compile(api: &Api, pat: &str, cflags: c_int) -> Compiled {
    unsafe {
        let cp = cbuf(pat.as_bytes());
        let mut ep: *const c_char = std::ptr::null();
        let prog = (api.js_regcomp)(cp.as_ptr(), cflags, &mut ep);
        Compiled {
            prog,
            err: if prog.is_null() {
                rstr(ep)
            } else {
                String::new()
            },
        }
    }
}

/// Compare `regexec` results, translating the `sub` pointers into offsets so
/// they are comparable across libraries.
fn exec_result(api: &Api, prog: *mut c_void, subject: &[c_char], eflags: c_int) -> (c_int, Vec<(isize, isize)>) {
    let mut sub = Resub::default();
    let rc = unsafe { (api.js_regexec)(prog, subject.as_ptr(), &mut sub, eflags) };
    let mut caps = Vec::new();
    if rc == 0 {
        let base = subject.as_ptr() as isize;
        for i in 0..(sub.nsub.clamp(0, REG_MAXSUB as c_int) as usize) {
            let sp = sub.sub[i].sp;
            let ep = sub.sub[i].ep;
            caps.push((
                if sp.is_null() { -1 } else { sp as isize - base },
                if ep.is_null() { -1 } else { ep as isize - base },
            ));
        }
        caps.insert(0, (sub.nsub as isize, -2));
    }
    (rc, caps)
}

fn run_flag_combo(cflags: c_int, eflags_list: &[c_int], label: &str) {
    let p = pair();
    let subs: Vec<Vec<c_char>> = subjects().iter().map(|s| cbuf(s.as_bytes())).collect();
    for pat in patterns() {
        let a = unsafe { compile(&p.c, &pat, cflags) };
        let b = unsafe { compile(&p.r, &pat, cflags) };
        assert_eq!(
            a.prog.is_null(),
            b.prog.is_null(),
            "{label}: compile success mismatch for {pat:?} (C err {:?}, RUST err {:?})",
            a.err,
            b.err
        );
        assert_eq!(a.err, b.err, "{label}: compile error text for {pat:?}");
        if a.prog.is_null() {
            continue;
        }
        for s in &subs {
            for &ef in eflags_list {
                let ra = exec_result(&p.c, a.prog, s, ef);
                let rb = exec_result(&p.r, b.prog, s, ef);
                assert_eq!(
                    ra, rb,
                    "{label}: exec mismatch pattern={pat:?} eflags={ef} subject={:?}",
                    String::from_utf8_lossy(
                        &s[..s.len() - 1].iter().map(|&c| c as u8).collect::<Vec<_>>()
                    )
                );
            }
        }
        unsafe { (p.c.js_regfree)(a.prog) };
        unsafe { (p.r.js_regfree)(b.prog) };
    }
}

#[test]
fn row27_regcomp_flags_none() {
    run_flag_combo(0, &[0], "cflags=0");
}

#[test]
fn row28_regcomp_icase() {
    run_flag_combo(REG_ICASE, &[0], "cflags=REG_ICASE");
}

#[test]
fn row29_regcomp_newline() {
    run_flag_combo(REG_NEWLINE, &[0], "cflags=REG_NEWLINE");
}

#[test]
fn row30_regcomp_icase_newline() {
    run_flag_combo(REG_ICASE | REG_NEWLINE, &[0], "cflags=ICASE|NEWLINE");
}

#[test]
fn row31_regexec_notbol() {
    run_flag_combo(0, &[REG_NOTBOL], "eflags=NOTBOL");
    run_flag_combo(REG_NEWLINE, &[0, REG_NOTBOL], "NEWLINE + NOTBOL");
    // undocumented / out-of-range eflag bits are accepted as plain ints by C
    run_flag_combo(0, &[8, 16, -1, 0x7fff_ffff], "eflags out of range");
}

#[test]
fn row32_regexec_null_sub() {
    let p = pair();
    let subs: Vec<Vec<c_char>> = subjects().iter().map(|s| cbuf(s.as_bytes())).collect();
    for pat in patterns() {
        let a = unsafe { compile(&p.c, &pat, 0) };
        let b = unsafe { compile(&p.r, &pat, 0) };
        assert_eq!(a.prog.is_null(), b.prog.is_null(), "compile {pat:?}");
        if a.prog.is_null() {
            continue;
        }
        for s in &subs {
            for ef in [0, REG_NOTBOL] {
                let ra =
                    unsafe { (p.c.js_regexec)(a.prog, s.as_ptr(), std::ptr::null_mut(), ef) };
                let rb =
                    unsafe { (p.r.js_regexec)(b.prog, s.as_ptr(), std::ptr::null_mut(), ef) };
                assert_eq!(ra, rb, "regexec(sub=NULL) {pat:?} eflags={ef}");
            }
        }
        unsafe { (p.c.js_regfree)(a.prog) };
        unsafe { (p.r.js_regfree)(b.prog) };
    }
}

/* --- custom allocator, exercised through regcompx / regfreex --- */

static mut ALLOC_CALLS_C: i64 = 0;
static mut ALLOC_CALLS_R: i64 = 0;

unsafe extern "C-unwind" fn alloc_c(_ctx: *mut c_void, ptr: *mut c_void, n: c_int) -> *mut c_void {
    unsafe {
        ALLOC_CALLS_C += 1;
        realloc_like(ptr, n)
    }
}

unsafe extern "C-unwind" fn alloc_r(_ctx: *mut c_void, ptr: *mut c_void, n: c_int) -> *mut c_void {
    unsafe {
        ALLOC_CALLS_R += 1;
        realloc_like(ptr, n)
    }
}

unsafe fn realloc_like(ptr: *mut c_void, n: c_int) -> *mut c_void {
    unsafe {
        unsafe extern "C" {
            fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
            fn free(p: *mut c_void);
        }
        if n == 0 {
            free(ptr);
            std::ptr::null_mut()
        } else {
            realloc(ptr, n as usize)
        }
    }
}

#[test]
fn row33_regcompx_custom_allocator() {
    let p = pair();
    let subs: Vec<Vec<c_char>> = subjects().iter().take(60).map(|s| cbuf(s.as_bytes())).collect();
    let mut ok = 0;
    for pat in patterns() {
        let cp = cbuf(pat.as_bytes());
        let mut ea: *const c_char = std::ptr::null();
        let mut eb: *const c_char = std::ptr::null();
        let pa = unsafe {
            (p.c.js_regcompx)(
                Some(alloc_c),
                std::ptr::null_mut(),
                cp.as_ptr(),
                0,
                &mut ea,
            )
        };
        let pb = unsafe {
            (p.r.js_regcompx)(
                Some(alloc_r),
                std::ptr::null_mut(),
                cp.as_ptr(),
                0,
                &mut eb,
            )
        };
        assert_eq!(pa.is_null(), pb.is_null(), "regcompx compile {pat:?}");
        if pa.is_null() {
            assert_eq!(
                unsafe { rstr(ea) },
                unsafe { rstr(eb) },
                "regcompx error text {pat:?}"
            );
            continue;
        }
        ok += 1;
        for s in &subs {
            let ra = exec_result(&p.c, pa, s, 0);
            let rb = exec_result(&p.r, pb, s, 0);
            assert_eq!(ra, rb, "regcompx exec {pat:?}");
        }
        unsafe { (p.c.js_regfreex)(Some(alloc_c), std::ptr::null_mut(), pa) };
        unsafe { (p.r.js_regfreex)(Some(alloc_r), std::ptr::null_mut(), pb) };
    }
    assert!(ok > 50, "expected many patterns to compile, got {ok}");
    assert!(unsafe { ALLOC_CALLS_C } > 0 && unsafe { ALLOC_CALLS_R } > 0);
}

#[test]
fn row34_regexec_randomized_subjects() {
    let p = pair();
    let mut rng = Rng::new(0x3434);
    let core_patterns = [
        "a", "a*", "(a|b)+", "^a.*z$", "[a-z]+", "\\d{2,3}", "(\\w)\\1", "x(?=y)", "\\bword\\b",
        ".", "(a)(b)?(c)?", "[^a]*", "a{0,}", "(?:ab)+c",
    ];
    for pat in core_patterns {
        for cflags in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
            let a = unsafe { compile(&p.c, pat, cflags) };
            let b = unsafe { compile(&p.r, pat, cflags) };
            assert!(!a.prog.is_null() && !b.prog.is_null(), "compile {pat:?}");
            for _ in 0..500 {
                let n = rng.below(24) as usize;
                let bytes: Vec<u8> = (0..n)
                    .map(|_| b"abczXYZ019 \n\t.-_"[rng.below(16) as usize])
                    .collect();
                let s = cbuf(&bytes);
                for ef in [0, REG_NOTBOL] {
                    let ra = exec_result(&p.c, a.prog, &s, ef);
                    let rb = exec_result(&p.r, b.prog, &s, ef);
                    assert_eq!(
                        ra,
                        rb,
                        "exec {pat:?} cflags={cflags} eflags={ef} subj={:?}",
                        String::from_utf8_lossy(&bytes)
                    );
                }
            }
            unsafe { (p.c.js_regfree)(a.prog) };
            unsafe { (p.r.js_regfree)(b.prog) };
        }
    }
}

#[test]
fn regfree_null_is_safe() {
    let p = pair();
    unsafe { (p.c.js_regfree)(std::ptr::null_mut()) };
    unsafe { (p.r.js_regfree)(std::ptr::null_mut()) };
    unsafe { (p.c.js_regfreex)(Some(alloc_c), std::ptr::null_mut(), std::ptr::null_mut()) };
    unsafe { (p.r.js_regfreex)(Some(alloc_r), std::ptr::null_mut(), std::ptr::null_mut()) };
}
