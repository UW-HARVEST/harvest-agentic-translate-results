// Phase C — error-path differential tests. Each test constructs the exact
// invalid input/condition from ERRORS.md, calls BOTH C and Rust via the .so,
// and asserts the SAME error code / sentinel.
mod common;
use common::*;
use std::os::raw::c_void;
use std::ptr;

unsafe fn compile_ok(lib: &Pcre2Lib, pat: &[u8], opts: u32) -> *mut c_void {
    let mut ec = 0;
    let mut eo = 0;
    let code = (lib.compile)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, ptr::null_mut());
    assert!(!code.is_null());
    code
}

// Row 1: config unknown selector.
#[test]
fn err_config_badoption() {
    let (c, r) = both();
    unsafe {
        let mut cv: u64 = 0;
        let mut rv: u64 = 0;
        for sel in [9999u32, 100, 0xFFFF_FFFF] {
            let cr = (c.config)(sel, &mut cv as *mut u64 as *mut c_void);
            let rr = (r.config)(sel, &mut rv as *mut u64 as *mut c_void);
            assert_eq!(cr, rr, "config badoption diff sel={}", sel);
            assert_eq!(cr, PCRE2_ERROR_BADOPTION, "expected BADOPTION sel={}", sel);
        }
    }
}

// Rows 4-11: compile error cases (bad options + bad patterns), compare errcode+offset.
#[test]
fn err_compile() {
    let (c, r) = both();
    // (pattern, options)
    let cases: &[(&[u8], u32)] = &[
        (b"(", 0),          // ERR14 missing )
        (b")", 0),          // ERR22 unmatched )
        (b"a{2,1}", 0),     // ERR4 quantifier out of order
        (b"*a", 0),         // ERR9 nothing to repeat
        (b"[a", 0),         // ERR6 missing terminating ]
        (b"(?P<>)", 0),     // bad group name
        (b"\\", 0),         // trailing backslash
        (b"(?<n>a)(?<n>b)", 0), // duplicate name without DUPNAMES
        (b"a", 0x1000_0000), // invalid option bit (reserved, not a PUBLIC_COMPILE_OPTION)
        (b"(?#unterminated", 0),
    ];
    unsafe {
        for (pat, opts) in cases {
            let mut cec = 0; let mut ceo = 0;
            let mut rec = 0; let mut reo = 0;
            let cc = (c.compile)(pat.as_ptr(), pat.len(), *opts, &mut cec, &mut ceo, ptr::null_mut());
            let rc = (r.compile)(pat.as_ptr(), pat.len(), *opts, &mut rec, &mut reo, ptr::null_mut());
            assert!(cc.is_null(), "C should reject pat={:?}", String::from_utf8_lossy(pat));
            assert!(rc.is_null(), "Rust should reject pat={:?}", String::from_utf8_lossy(pat));
            assert_eq!(cec, rec, "errcode diff pat={:?}", String::from_utf8_lossy(pat));
            assert_eq!(ceo, reo, "erroffset diff pat={:?}", String::from_utf8_lossy(pat));
        }
    }
}

// Rows 12-15: match top-level errors.
#[test]
fn err_match_toplevel() {
    let (c, r) = both();
    unsafe {
        let cc = compile_ok(&c, b"abc", 0);
        let rc = compile_ok(&r, b"abc", 0);
        let cmd = (c.md_create)(10, ptr::null_mut());
        let rmd = (r.md_create)(10, ptr::null_mut());
        let subj = b"abc";

        // Row 12: NULL match_data
        let cr = (c.r#match)(cc, subj.as_ptr(), 3, 0, 0, ptr::null_mut(), ptr::null_mut());
        let rr = (r.r#match)(rc, subj.as_ptr(), 3, 0, 0, ptr::null_mut(), ptr::null_mut());
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NULL);

        // Row 13: NULL code
        let cr = (c.r#match)(ptr::null(), subj.as_ptr(), 3, 0, 0, cmd, ptr::null_mut());
        let rr = (r.r#match)(ptr::null(), subj.as_ptr(), 3, 0, 0, rmd, ptr::null_mut());
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NULL);
        // Row 13b: NULL subject
        let cr = (c.r#match)(cc, ptr::null(), 3, 0, 0, cmd, ptr::null_mut());
        let rr = (r.r#match)(rc, ptr::null(), 3, 0, 0, rmd, ptr::null_mut());
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NULL);

        // Row 14: invalid option bits
        let bad_opt = 0x0000_0001; // not a PUBLIC_MATCH_OPTION at that bit? use a reserved bit
        let cr = (c.r#match)(cc, subj.as_ptr(), 3, 0, bad_opt, cmd, ptr::null_mut());
        let rr = (r.r#match)(rc, subj.as_ptr(), 3, 0, bad_opt, rmd, ptr::null_mut());
        assert_eq!(cr, rr, "match bad option diff");

        // Row 15: start_offset > length
        let cr = (c.r#match)(cc, subj.as_ptr(), 3, 5, 0, cmd, ptr::null_mut());
        let rr = (r.r#match)(rc, subj.as_ptr(), 3, 5, 0, rmd, ptr::null_mut());
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_BADOFFSET);

        // Row 16: no match
        let cr = (c.r#match)(cc, b"xyz".as_ptr(), 3, 0, 0, cmd, ptr::null_mut());
        let rr = (r.r#match)(rc, b"xyz".as_ptr(), 3, 0, 0, rmd, ptr::null_mut());
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NOMATCH);

        (c.md_free)(cmd); (r.md_free)(rmd);
        (c.code_free)(cc); (r.code_free)(rc);
    }
}

// Row 17: invalid UTF subject.
#[test]
fn err_match_bad_utf() {
    let (c, r) = both();
    unsafe {
        let cc = compile_ok(&c, b".", PCRE2_UTF);
        let rc = compile_ok(&r, b".", PCRE2_UTF);
        let cmd = (c.md_create)(10, ptr::null_mut());
        let rmd = (r.md_create)(10, ptr::null_mut());
        // invalid UTF-8: lone continuation byte
        let bad = [0x80u8, 0x80, 0x80];
        let cr = (c.r#match)(cc, bad.as_ptr(), 3, 0, 0, cmd, ptr::null_mut());
        let rr = (r.r#match)(rc, bad.as_ptr(), 3, 0, 0, rmd, ptr::null_mut());
        assert_eq!(cr, rr, "bad utf rc diff");
        assert!(cr < PCRE2_ERROR_NOMATCH, "expected UTF error, got {}", cr);
        (c.md_free)(cmd); (r.md_free)(rmd);
        (c.code_free)(cc); (r.code_free)(rc);
    }
}

// Rows 18-22: dfa_match top-level errors.
#[test]
fn err_dfa_toplevel() {
    let (c, r) = both();
    unsafe {
        let cc = compile_ok(&c, b"abc", 0);
        let rc = compile_ok(&r, b"abc", 0);
        let cmd = (c.md_create)(10, ptr::null_mut());
        let rmd = (r.md_create)(10, ptr::null_mut());
        let subj = b"abc";
        let mut cws = [0i32; 40];
        let mut rws = [0i32; 40];

        // Row 18: NULL match_data
        let cr = (c.dfa_match)(cc, subj.as_ptr(), 3, 0, 0, ptr::null_mut(), ptr::null_mut(), cws.as_mut_ptr(), 40);
        let rr = (r.dfa_match)(rc, subj.as_ptr(), 3, 0, 0, ptr::null_mut(), ptr::null_mut(), rws.as_mut_ptr(), 40);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NULL);

        // Row 19: NULL workspace
        let cr = (c.dfa_match)(cc, subj.as_ptr(), 3, 0, 0, cmd, ptr::null_mut(), ptr::null_mut(), 40);
        let rr = (r.dfa_match)(rc, subj.as_ptr(), 3, 0, 0, rmd, ptr::null_mut(), ptr::null_mut(), 40);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NULL);
        // Row 19b: NULL subject
        let cr = (c.dfa_match)(cc, ptr::null(), 3, 0, 0, cmd, ptr::null_mut(), cws.as_mut_ptr(), 40);
        let rr = (r.dfa_match)(rc, ptr::null(), 3, 0, 0, rmd, ptr::null_mut(), rws.as_mut_ptr(), 40);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NULL);

        // Row 20: invalid option bits
        let cr = (c.dfa_match)(cc, subj.as_ptr(), 3, 0, 0x0000_0001, cmd, ptr::null_mut(), cws.as_mut_ptr(), 40);
        let rr = (r.dfa_match)(rc, subj.as_ptr(), 3, 0, 0x0000_0001, rmd, ptr::null_mut(), rws.as_mut_ptr(), 40);
        assert_eq!(cr, rr, "dfa bad option diff");

        // Row 21: wscount < 20
        let cr = (c.dfa_match)(cc, subj.as_ptr(), 3, 0, 0, cmd, ptr::null_mut(), cws.as_mut_ptr(), 10);
        let rr = (r.dfa_match)(rc, subj.as_ptr(), 3, 0, 0, rmd, ptr::null_mut(), rws.as_mut_ptr(), 10);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_DFA_WSSIZE);

        // Row 22: start_offset > length
        let cr = (c.dfa_match)(cc, subj.as_ptr(), 3, 5, 0, cmd, ptr::null_mut(), cws.as_mut_ptr(), 40);
        let rr = (r.dfa_match)(rc, subj.as_ptr(), 3, 5, 0, rmd, ptr::null_mut(), rws.as_mut_ptr(), 40);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_BADOFFSET);

        // Row 23: no match
        let cr = (c.dfa_match)(cc, b"xyz".as_ptr(), 3, 0, 0, cmd, ptr::null_mut(), cws.as_mut_ptr(), 40);
        let rr = (r.dfa_match)(rc, b"xyz".as_ptr(), 3, 0, 0, rmd, ptr::null_mut(), rws.as_mut_ptr(), 40);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NOMATCH);

        (c.md_free)(cmd); (r.md_free)(rmd);
        (c.code_free)(cc); (r.code_free)(rc);
    }
}

// Rows 24-26: pattern_info errors.
#[test]
fn err_pattern_info() {
    let (c, r) = both();
    unsafe {
        // Row 24: NULL code
        let mut cv: u64 = 0; let mut rv: u64 = 0;
        let cr = (c.pattern_info)(ptr::null(), PCRE2_INFO_SIZE, &mut cv as *mut u64 as *mut c_void);
        let rr = (r.pattern_info)(ptr::null(), PCRE2_INFO_SIZE, &mut rv as *mut u64 as *mut c_void);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NULL);

        let cc = compile_ok(&c, b"abc", 0);
        let rc = compile_ok(&r, b"abc", 0);
        // Row 25: unknown selector
        let cr = (c.pattern_info)(cc, 9999, &mut cv as *mut u64 as *mut c_void);
        let rr = (r.pattern_info)(rc, 9999, &mut rv as *mut u64 as *mut c_void);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_BADOPTION);

        // Row 26: limit selectors when unset (depth/heap/match limits)
        for sel in [14u32 /*MATCHLIMIT*/, 21 /*DEPTHLIMIT*/, 25 /*HEAPLIMIT*/] {
            let cr = (c.pattern_info)(cc, sel, &mut cv as *mut u64 as *mut c_void);
            let rr = (r.pattern_info)(rc, sel, &mut rv as *mut u64 as *mut c_void);
            assert_eq!(cr, rr, "info unset limit diff sel={}", sel);
            assert_eq!(cr, PCRE2_ERROR_UNSET, "expected UNSET sel={}", sel);
        }
        (c.code_free)(cc); (r.code_free)(rc);
    }
}

// Rows 27-29, 31-32: substring errors.
#[test]
fn err_substring() {
    let (c, r) = both();
    unsafe {
        let cc = compile_ok(&c, b"(a)(b)?", 0);
        let rc = compile_ok(&r, b"(a)(b)?", 0);
        let cmd = (c.md_create)(10, ptr::null_mut());
        let rmd = (r.md_create)(10, ptr::null_mut());
        let subj = b"a"; // group 2 will be unset
        (c.r#match)(cc, subj.as_ptr(), 1, 0, 0, cmd, ptr::null_mut());
        (r.r#match)(rc, subj.as_ptr(), 1, 0, 0, rmd, ptr::null_mut());

        // Row 27: number past top capture -> NOSUBSTRING
        let mut cl = 0usize; let mut rl = 0usize;
        let cr = (c.substr_len_bynum)(cmd, 99, &mut cl);
        let rr = (r.substr_len_bynum)(rmd, 99, &mut rl);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NOSUBSTRING);

        // Row 29: valid group index that is unset -> UNSET
        let cr = (c.substr_len_bynum)(cmd, 2, &mut cl);
        let rr = (r.substr_len_bynum)(rmd, 2, &mut rl);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_UNSET);

        // Row 30: copy into too-small buffer -> NOMEMORY
        let mut cbuf = [0u8; 1];
        let mut rbuf = [0u8; 1];
        let mut cbl = 0usize; // zero-size
        let mut rbl = 0usize;
        let cr = (c.substr_copy_bynum)(cmd, 1, cbuf.as_mut_ptr(), &mut cbl);
        let rr = (r.substr_copy_bynum)(rmd, 1, rbuf.as_mut_ptr(), &mut rbl);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NOMEMORY);

        // Row 31/32: name not present
        let bad = b"missing\0";
        let cn = (c.substr_num_from_name)(cc, bad.as_ptr());
        let rn = (r.substr_num_from_name)(rc, bad.as_ptr());
        assert_eq!(cn, rn); assert_eq!(cn, PCRE2_ERROR_NOSUBSTRING);
        let mut cf: *const u8 = ptr::null();
        let mut cll: *const u8 = ptr::null();
        let mut rf: *const u8 = ptr::null();
        let mut rll: *const u8 = ptr::null();
        let cs = (c.substr_nametable_scan)(cc, bad.as_ptr(), &mut cf, &mut cll);
        let rs = (r.substr_nametable_scan)(rc, bad.as_ptr(), &mut rf, &mut rll);
        assert_eq!(cs, rs); assert_eq!(cs, PCRE2_ERROR_NOSUBSTRING);

        (c.md_free)(cmd); (r.md_free)(rmd);
        (c.code_free)(cc); (r.code_free)(rc);
    }
}

// Rows 33-38: substitute errors.
#[test]
fn err_substitute() {
    let (c, r) = both();
    unsafe {
        // helper
        let run = |lib: &Pcre2Lib, pat: &[u8], subj: &[u8], repl: &[u8], subopts: u32, mopts: u32, bufsize: usize| -> (i32, usize) {
            let mut ec = 0; let mut eo = 0;
            let code = (lib.compile)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, ptr::null_mut());
            assert!(!code.is_null());
            let mut buf = vec![0u8; bufsize];
            let mut bl = bufsize;
            let rc = (lib.substitute)(code, subj.as_ptr(), subj.len(), 0, subopts | mopts, ptr::null_mut(), ptr::null_mut(), repl.as_ptr(), repl.len(), buf.as_mut_ptr(), &mut bl);
            (lib.code_free)(code);
            (rc, bl)
        };

        // Row 33: partial option without REPLACEMENT_ONLY -> BADOPTION
        let (cr, _) = run(&c, b"a", b"a", b"X", 0, 0x0000_0010 /*PARTIAL_SOFT*/, 32);
        let (rr, _) = run(&r, b"a", b"a", b"X", 0, 0x0000_0010, 32);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_BADOPTION);

        // Row 35: buffer too small (no OVERFLOW_LENGTH) -> NOMEMORY
        let (cr, _) = run(&c, b"a", b"aaaa", b"XXXX", PCRE2_SUBSTITUTE_GLOBAL, 0, 3);
        let (rr, _) = run(&r, b"a", b"aaaa", b"XXXX", PCRE2_SUBSTITUTE_GLOBAL, 0, 3);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NOMEMORY);

        // Row 36: bad replacement lone $ -> BADREPLACEMENT
        let (cr, _) = run(&c, b"a", b"a", b"$", 0, 0, 32);
        let (rr, _) = run(&r, b"a", b"a", b"$", 0, 0, 32);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_BADREPLACEMENT);

        // Row 37: ${ with no closing brace -> REPMISSINGBRACE
        let (cr, _) = run(&c, b"(a)", b"a", b"${1", 0, 0, 32);
        let (rr, _) = run(&r, b"(a)", b"a", b"${1", 0, 0, 32);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_REPMISSINGBRACE);

        // Row 38: $99 unknown group -> NOSUBSTRING
        let (cr, _) = run(&c, b"(a)", b"a", b"$99", 0, 0, 32);
        let (rr, _) = run(&r, b"(a)", b"a", b"$99", 0, 0, 32);
        assert_eq!(cr, rr); assert_eq!(cr, PCRE2_ERROR_NOSUBSTRING);
    }
}

// Rows 39-40: get_error_message edge cases.
#[test]
fn err_get_error_message() {
    let (c, r) = both();
    unsafe {
        // Row 39: unknown error number
        let mut cbuf = [0u8; 64];
        let mut rbuf = [0u8; 64];
        let cn = (c.get_err_msg)(999999, cbuf.as_mut_ptr(), cbuf.len());
        let rn = (r.get_err_msg)(999999, rbuf.as_mut_ptr(), rbuf.len());
        assert_eq!(cn, rn, "err_msg unknown code diff");

        // Row 40: zero-length buffer -> NOMEMORY
        let cn = (c.get_err_msg)(-1, cbuf.as_mut_ptr(), 0);
        let rn = (r.get_err_msg)(-1, rbuf.as_mut_ptr(), 0);
        assert_eq!(cn, rn, "err_msg zero buffer diff");
        assert_eq!(cn, PCRE2_ERROR_NOMEMORY);
    }
}

// Rows 41-42: serialize errors.
#[test]
fn err_serialize() {
    let (c, r) = both();
    unsafe {
        // Row 42: encode with zero count / NULL codes -> BADDATA
        let mut bytes: *mut u8 = ptr::null_mut();
        let mut n = 0usize;
        let cr = (c.serialize_encode)(ptr::null(), 0, &mut bytes, &mut n, ptr::null_mut());
        let rr = (r.serialize_encode)(ptr::null(), 0, &mut bytes, &mut n, ptr::null_mut());
        assert_eq!(cr, rr, "serialize_encode bad data diff");

        // Row 41: decode NULL/garbage -> error
        let mut dec: *mut c_void = ptr::null_mut();
        let cr = (c.serialize_decode)(&mut dec, 1, ptr::null(), ptr::null_mut());
        let rr = (r.serialize_decode)(&mut dec, 1, ptr::null(), ptr::null_mut());
        assert_eq!(cr, rr, "serialize_decode null diff");

        // garbage bytes
        let garbage = [0u8; 64];
        let mut dec2: *mut c_void = ptr::null_mut();
        let cr = (c.serialize_decode)(&mut dec2, 1, garbage.as_ptr(), ptr::null_mut());
        let rr = (r.serialize_decode)(&mut dec2, 1, garbage.as_ptr(), ptr::null_mut());
        assert_eq!(cr, rr, "serialize_decode garbage diff");
    }
}

// Generic boundary: out-of-range option values / enum values across FFI.
#[test]
fn err_generic_boundaries() {
    let (c, r) = both();
    unsafe {
        // pattern_info with huge selector values (enum out of range)
        let cc = compile_ok(&c, b"a", 0);
        let rc = compile_ok(&r, b"a", 0);
        let mut cv: u64 = 0; let mut rv: u64 = 0;
        for sel in [27u32, 28, 100, 0x7FFF_FFFF, 0xFFFF_FFFF] {
            let cr = (c.pattern_info)(cc, sel, &mut cv as *mut u64 as *mut c_void);
            let rr = (r.pattern_info)(rc, sel, &mut rv as *mut u64 as *mut c_void);
            assert_eq!(cr, rr, "pattern_info oob sel={} diff", sel);
        }
        (c.code_free)(cc); (r.code_free)(rc);

        // config with out-of-range selectors
        for sel in [100u32, 0x7FFF_FFFF, 0xFFFF_FFFF] {
            let cr = (c.config)(sel, &mut cv as *mut u64 as *mut c_void);
            let rr = (r.config)(sel, &mut rv as *mut u64 as *mut c_void);
            assert_eq!(cr, rr, "config oob sel={} diff", sel);
        }
    }
}
