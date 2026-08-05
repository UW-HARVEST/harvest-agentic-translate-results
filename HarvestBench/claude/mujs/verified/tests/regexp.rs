//! Phase B/C — differential tests for regexp.c.
//! CONFIGS rows 9-14, ERRORS rows 1-14.
mod common;
use common::{Libs, Rng};
use std::os::raw::{c_char, c_int, c_void};

const REG_MAXSUB: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
struct ResubItem {
    sp: *const c_char,
    ep: *const c_char,
}
#[repr(C)]
struct Resub {
    nsub: c_int,
    sub: [ResubItem; REG_MAXSUB],
}
impl Resub {
    fn zeroed() -> Resub {
        Resub {
            nsub: 0,
            sub: [ResubItem { sp: std::ptr::null(), ep: std::ptr::null() }; REG_MAXSUB],
        }
    }
}

// Reprog* regcomp(const char *pattern, int cflags, const char **errorp)
type RegcompFn = unsafe extern "C" fn(*const c_char, c_int, *mut *const c_char) -> *mut c_void;
// int regexec(Reprog *prog, const char *string, Resub *sub, int eflags)
type RegexecFn = unsafe extern "C" fn(*mut c_void, *const c_char, *mut Resub, c_int) -> c_int;
type RegfreeFn = unsafe extern "C" fn(*mut c_void);

const REG_ICASE: c_int = 1;
const REG_NEWLINE: c_int = 2;
const REG_NOTBOL: c_int = 4;

fn cstr(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

unsafe fn errstr(p: *const c_char) -> Option<String> {
    if p.is_null() { return None; }
    let mut s = String::new();
    let mut i = 0;
    loop {
        let c = *p.add(i);
        if c == 0 { break; }
        s.push(c as u8 as char);
        i += 1;
    }
    Some(s)
}

struct Regexp<'a> {
    libs: &'a Libs,
    comp_c: libloading::Symbol<'a, RegcompFn>,
    comp_r: libloading::Symbol<'a, RegcompFn>,
    exec_c: libloading::Symbol<'a, RegexecFn>,
    exec_r: libloading::Symbol<'a, RegexecFn>,
    free_c: libloading::Symbol<'a, RegfreeFn>,
    free_r: libloading::Symbol<'a, RegfreeFn>,
}

impl<'a> Regexp<'a> {
    unsafe fn new(libs: &'a Libs) -> Regexp<'a> {
        Regexp {
            comp_c: libs.c_sym(b"js_regcomp"),
            comp_r: libs.rust_sym(b"js_regcomp"),
            exec_c: libs.c_sym(b"js_regexec"),
            exec_r: libs.rust_sym(b"js_regexec"),
            free_c: libs.c_sym(b"js_regfree"),
            free_r: libs.rust_sym(b"js_regfree"),
            libs,
        }
    }

    /// Compile in both; assert error-parity. Returns compiled progs if both ok.
    unsafe fn compile(&self, pat: &str, cflags: c_int) -> Option<(*mut c_void, *mut c_void)> {
        let _ = self.libs;
        let cp = cstr(pat);
        let rp = cstr(pat);
        let mut cerr: *const c_char = std::ptr::null();
        let mut rerr: *const c_char = std::ptr::null();
        let cprog = (self.comp_c)(cp.as_ptr(), cflags, &mut cerr);
        let rprog = (self.comp_r)(rp.as_ptr(), cflags, &mut rerr);
        assert_eq!(cprog.is_null(), rprog.is_null(),
            "regcomp null-parity pat={:?} cflags={} cerr={:?} rerr={:?}",
            pat, cflags, errstr(cerr), errstr(rerr));
        if cprog.is_null() {
            assert_eq!(errstr(cerr), errstr(rerr),
                "regcomp error string pat={:?} cflags={}", pat, cflags);
            return None;
        }
        Some((cprog, rprog))
    }

    unsafe fn exec_and_compare(&self, cprog: *mut c_void, rprog: *mut c_void, s: &str, eflags: c_int) {
        let cs = cstr(s);
        let rs = cstr(s);
        let mut csub = Resub::zeroed();
        let mut rsub = Resub::zeroed();
        let cr = (self.exec_c)(cprog, cs.as_ptr(), &mut csub, eflags);
        let rr = (self.exec_r)(rprog, rs.as_ptr(), &mut rsub, eflags);
        assert_eq!(cr, rr, "regexec return s={:?} eflags={}", s, eflags);
        if cr == 0 {
            assert_eq!(csub.nsub, rsub.nsub, "nsub s={:?}", s);
            for i in 0..(csub.nsub as usize).min(REG_MAXSUB) {
                // compare offsets relative to their own base pointers
                let c_off_sp = if csub.sub[i].sp.is_null() { -1isize }
                    else { csub.sub[i].sp as isize - cs.as_ptr() as isize };
                let c_off_ep = if csub.sub[i].ep.is_null() { -1isize }
                    else { csub.sub[i].ep as isize - cs.as_ptr() as isize };
                let r_off_sp = if rsub.sub[i].sp.is_null() { -1isize }
                    else { rsub.sub[i].sp as isize - rs.as_ptr() as isize };
                let r_off_ep = if rsub.sub[i].ep.is_null() { -1isize }
                    else { rsub.sub[i].ep as isize - rs.as_ptr() as isize };
                assert_eq!((c_off_sp, c_off_ep), (r_off_sp, r_off_ep),
                    "capture[{}] offsets s={:?} eflags={}", i, s, eflags);
            }
        }
    }

    unsafe fn free(&self, cprog: *mut c_void, rprog: *mut c_void) {
        (self.free_c)(cprog);
        (self.free_r)(rprog);
    }
}

#[test]
fn regcomp_error_parity() {
    let libs = Libs::load();
    unsafe {
        let re = Regexp::new(&libs);
        // ERRORS rows 1-12: every distinct rejection path
        let bad = [
            "\\",           // unterminated escape
            "a\\",          // unterminated escape
            "*",            // invalid quantifier (nothing to repeat)
            "+",
            "?",
            "(",            // unmatched (
            "(a",           // unmatched (
            ")",            // unmatched )
            "a)",
            "[a-",          // unterminated character class
            "[",
            "[z-a]",        // reversed range
            "[b-a]",
            "\\9",          // invalid back-reference
            "\\8",
            "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)(q)", // too many captures (>15)
            "\\x",          // truncated hex escape
            "\\u",
            "\\uG",
            "a{2,1}",       // invalid quantifier order? (regexp.c treats as range)
            "a{",
        ];
        for pat in bad {
            // most should error; a few may compile — either way parity is asserted
            if let Some((c, r)) = re.compile(pat, 0) {
                re.free(c, r);
            }
        }
        // program-too-large: a very long pattern (len*2 > 32768)
        let huge = "a".repeat(20000);
        if let Some((c, r)) = re.compile(&huge, 0) {
            re.free(c, r);
        }
    }
}

#[test]
fn regexp_match_valid_paths() {
    let libs = Libs::load();
    unsafe {
        let re = Regexp::new(&libs);
        // (pattern, cflags) matrix
        let patterns: &[(&str, c_int)] = &[
            ("abc", 0),
            ("a.c", 0),
            ("a*b", 0),
            ("a+b?", 0),
            ("(a|b)c", 0),
            ("(foo)(bar)", 0),
            ("^start", 0),
            ("end$", 0),
            ("[a-z]+", 0),
            ("[^0-9]+", 0),
            ("a{2,4}", 0),
            ("(\\w+)@(\\w+)", 0),
            ("\\d{3}-\\d{4}", 0),
            ("ABC", REG_ICASE),
            ("[a-z]+", REG_ICASE),
            ("^line$", REG_NEWLINE),
            ("^.$", REG_NEWLINE),
            ("(a(b(c)))", 0),
            ("colou?r", 0),
            ("(cat|dog|bird)s?", 0),
            ("\\bword\\b", 0),
            ("a.*z", 0),
        ];
        let subjects = [
            "abc", "aXc", "aaab", "ab", "bc", "foobar", "start here", "the end",
            "hello", "12345", "aaaa", "user@host", "555-1234", "abcABC",
            "line1\nline2\nline3", "x\ny", "abc", "color colour", "cats dogs birds",
            "a word here", "azaz", "", "no match at all", "AAAA",
            "a\nb", "MixedCase",
        ];
        for &(pat, cflags) in patterns {
            if let Some((c, r)) = re.compile(pat, cflags) {
                for s in subjects {
                    for ef in [0, REG_NOTBOL] {
                        re.exec_and_compare(c, r, s, ef);
                    }
                }
                re.free(c, r);
            }
        }
    }
}

#[test]
fn regexp_out_of_range_flags() {
    // Phase C generic boundary: C enums accept any int. Feed cflags/eflags
    // values with bits outside the documented REG_* set and assert parity.
    let libs = Libs::load();
    unsafe {
        let re = Regexp::new(&libs);
        let weird_flags = [
            0, 1, 2, 4, 7, 8, 16, 0x7F, -1, i32::MIN, i32::MAX, 0x1000_0000, 255,
        ];
        for cf in weird_flags {
            if let Some((c, r)) = re.compile("a(b|c)d?", cf) {
                for ef in weird_flags {
                    for s in ["abd", "acd", "ab", "xyz", "abdabd", ""] {
                        re.exec_and_compare(c, r, s, ef);
                    }
                }
                re.free(c, r);
            }
        }
    }
}

#[test]
fn regexp_random_fuzz() {
    let libs = Libs::load();
    unsafe {
        let re = Regexp::new(&libs);
        let mut rng = Rng::new(777);
        // build random small patterns from a metachar alphabet, assert parity
        let alpha = b"abc().*+?|[]^$\\dws-{}0123";
        for _ in 0..8000 {
            let n = 1 + rng.below(10) as usize;
            let mut pat = String::new();
            for _ in 0..n {
                pat.push(alpha[rng.below(alpha.len() as u32) as usize] as char);
            }
            let cflags = (rng.below(4) as c_int) & (REG_ICASE | REG_NEWLINE);
            if let Some((c, r)) = re.compile(&pat, cflags) {
                // random subjects
                for _ in 0..8 {
                    let sn = rng.below(12) as usize;
                    let mut s = String::new();
                    let set = b"abcABC123 \n@-";
                    for _ in 0..sn {
                        s.push(set[rng.below(set.len() as u32) as usize] as char);
                    }
                    let ef = if rng.below(2) == 0 { 0 } else { REG_NOTBOL };
                    re.exec_and_compare(c, r, &s, ef);
                }
                re.free(c, r);
            }
        }
    }
}
