// Phase B (part 2) — valid-path differential tests for the higher-level API:
// pattern_info, substring extraction, substitute, serialize, config, maketables.
mod common;
use common::*;
use std::os::raw::c_void;

struct Rng(u64);
impl Rng {
    fn new(s: u64) -> Rng { Rng(s) }
    fn next(&mut self) -> u64 {
        let mut x = self.0; x ^= x << 13; x ^= x >> 7; x ^= x << 17; self.0 = x; x
    }
    fn range(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}

unsafe fn compile(lib: &Pcre2Lib, pat: &[u8], opts: u32) -> *mut c_void {
    let mut ec = 0;
    let mut eo = 0;
    let code = (lib.compile)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, std::ptr::null_mut());
    assert!(!code.is_null(), "compile failed pat={:?} ec={}", String::from_utf8_lossy(pat), ec);
    code
}

// Row 26: pattern_info across many selectors and patterns.
#[test]
fn row_pattern_info() {
    let (c, r) = both();
    let pats: &[(&[u8], u32)] = &[
        (b"abc", 0),
        (b"(a)(b)(c)", 0),
        (b"(?<x>a)(?<y>b)", 0),
        (b"a.*c", 0),
        (b"^abc$", PCRE2_MULTILINE),
        (b"(?i)abc", 0),
        (b"\\bword\\b", 0),
        (b"a{2,4}", 0),
        (b".", PCRE2_UTF),
        (b"(a)\\1", 0),
        (b"(?<dup>a)|(?<dup>b)", 0x00000040 /*DUPNAMES*/),
    ];
    let selectors = [
        PCRE2_INFO_ALLOPTIONS, PCRE2_INFO_ARGOPTIONS, PCRE2_INFO_BACKREFMAX,
        PCRE2_INFO_BSR, PCRE2_INFO_CAPTURECOUNT, PCRE2_INFO_HASCRORLF,
        PCRE2_INFO_JCHANGED, PCRE2_INFO_MATCHEMPTY, PCRE2_INFO_MAXLOOKBEHIND,
        PCRE2_INFO_MINLENGTH, PCRE2_INFO_NAMECOUNT, PCRE2_INFO_NAMEENTRYSIZE,
        PCRE2_INFO_NEWLINE, PCRE2_INFO_SIZE,
    ];
    unsafe {
        for (pat, opts) in pats {
            let cc = compile(&c, pat, *opts);
            let rc = compile(&r, pat, *opts);
            for &sel in &selectors {
                // Most selectors return a value into a u64/size_t buffer.
                let mut cv: u64 = 0;
                let mut rv: u64 = 0;
                let crc = (c.pattern_info)(cc, sel, &mut cv as *mut u64 as *mut c_void);
                let rrc = (r.pattern_info)(rc, sel, &mut rv as *mut u64 as *mut c_void);
                assert_eq!(crc, rrc, "info rc diff sel={} pat={:?}", sel, String::from_utf8_lossy(pat));
                if crc == 0 {
                    assert_eq!(cv, rv, "info value diff sel={} pat={:?}", sel, String::from_utf8_lossy(pat));
                }
            }
            (c.code_free)(cc);
            (r.code_free)(rc);
        }
    }
}

// Rows 27-29: substring extraction functions after a match.
#[test]
fn row_substring() {
    let (c, r) = both();
    let mut rng = Rng::new(0x5B57_1234);
    let cases: &[(&[u8], &[u8])] = &[
        (b"(a+)(b+)(c+)", b"aaabbcccc"),
        (b"(?<first>\\w+)@(?<second>\\w+)", b"user@host"),
        (b"(\\d+)-(\\d+)", b"12-345"),
        (b"(a)(b)?(c)", b"ac"),
    ];
    unsafe {
        for (pat, subj) in cases {
            for _ in 0..30 {
                let _ = rng.next();
                let cc = compile(&c, pat, 0);
                let rc = compile(&r, pat, 0);
                let cmd = (c.md_create)(20, std::ptr::null_mut());
                let rmd = (r.md_create)(20, std::ptr::null_mut());
                let crc = (c.r#match)(cc, subj.as_ptr(), subj.len(), 0, 0, cmd, std::ptr::null_mut());
                let rrc = (r.r#match)(rc, subj.as_ptr(), subj.len(), 0, 0, rmd, std::ptr::null_mut());
                assert_eq!(crc, rrc, "match rc diff");
                if crc > 0 {
                    for gi in 0..(crc as u32 + 2) {
                        // length_bynumber
                        let mut cl = 0usize;
                        let mut rl = 0usize;
                        let clr = (c.substr_len_bynum)(cmd, gi, &mut cl);
                        let rlr = (r.substr_len_bynum)(rmd, gi, &mut rl);
                        assert_eq!(clr, rlr, "substr_len rc diff gi={}", gi);
                        if clr == 0 {
                            assert_eq!(cl, rl, "substr_len diff gi={}", gi);
                        }
                        // copy_bynumber into a generous buffer
                        let mut cbuf = [0u8; 64];
                        let mut rbuf = [0u8; 64];
                        let mut cbl = cbuf.len();
                        let mut rbl = rbuf.len();
                        let ccr = (c.substr_copy_bynum)(cmd, gi, cbuf.as_mut_ptr(), &mut cbl);
                        let rcr = (r.substr_copy_bynum)(rmd, gi, rbuf.as_mut_ptr(), &mut rbl);
                        assert_eq!(ccr, rcr, "substr_copy rc diff gi={}", gi);
                        if ccr == 0 {
                            assert_eq!(cbl, rbl, "substr_copy len diff gi={}", gi);
                            assert_eq!(&cbuf[..cbl], &rbuf[..rbl], "substr_copy data diff gi={}", gi);
                        }
                        // get_bynumber (heap alloc)
                        let mut cp: *mut u8 = std::ptr::null_mut();
                        let mut rp: *mut u8 = std::ptr::null_mut();
                        let mut cgl = 0usize;
                        let mut rgl = 0usize;
                        let cgr = (c.substr_get_bynum)(cmd, gi, &mut cp, &mut cgl);
                        let rgr = (r.substr_get_bynum)(rmd, gi, &mut rp, &mut rgl);
                        assert_eq!(cgr, rgr, "substr_get rc diff gi={}", gi);
                        if cgr == 0 {
                            assert_eq!(cgl, rgl, "substr_get len diff gi={}", gi);
                            let cs = std::slice::from_raw_parts(cp, cgl);
                            let rs = std::slice::from_raw_parts(rp, rgl);
                            assert_eq!(cs, rs, "substr_get data diff gi={}", gi);
                            (c.substr_free)(cp);
                            (r.substr_free)(rp);
                        }
                    }
                }
                (c.md_free)(cmd);
                (r.md_free)(rmd);
                (c.code_free)(cc);
                (r.code_free)(rc);
            }
        }
    }
}

// Row 28 cont.: number_from_name and nametable_scan.
#[test]
fn row_substring_names() {
    let (c, r) = both();
    let pat: &[u8] = b"(?<year>\\d{4})-(?<month>\\d{2})-(?<day>\\d{2})";
    let names: &[&[u8]] = &[b"year\0", b"month\0", b"day\0", b"nope\0"];
    unsafe {
        let cc = compile(&c, pat, 0);
        let rc = compile(&r, pat, 0);
        for name in names {
            let cn = (c.substr_num_from_name)(cc, name.as_ptr());
            let rn = (r.substr_num_from_name)(rc, name.as_ptr());
            assert_eq!(cn, rn, "num_from_name diff name={:?}", String::from_utf8_lossy(name));
        }
        (c.code_free)(cc);
        (r.code_free)(rc);
    }
}

// Rows 30-36: substitute.
fn subst_case(
    c: &Pcre2Lib, r: &Pcre2Lib,
    pat: &[u8], opts: u32, subject: &[u8], repl: &[u8], subopts: u32, bufsize: usize, ctx: &str,
) {
    unsafe {
        let cc = compile(c, pat, opts);
        let rc = compile(r, pat, opts);
        let mut cbuf = vec![0u8; bufsize];
        let mut rbuf = vec![0u8; bufsize];
        let mut cbl = bufsize;
        let mut rbl = bufsize;
        let crc = (c.substitute)(
            cc, subject.as_ptr(), subject.len(), 0, subopts,
            std::ptr::null_mut(), std::ptr::null_mut(),
            repl.as_ptr(), repl.len(), cbuf.as_mut_ptr(), &mut cbl,
        );
        let rrc = (r.substitute)(
            rc, subject.as_ptr(), subject.len(), 0, subopts,
            std::ptr::null_mut(), std::ptr::null_mut(),
            repl.as_ptr(), repl.len(), rbuf.as_mut_ptr(), &mut rbl,
        );
        assert_eq!(crc, rrc, "subst rc diff [{}] pat={:?} subj={:?} repl={:?}",
            ctx, String::from_utf8_lossy(pat), String::from_utf8_lossy(subject), String::from_utf8_lossy(repl));
        assert_eq!(cbl, rbl, "subst outlen diff [{}]", ctx);
        if crc >= 0 {
            assert_eq!(&cbuf[..cbl.min(bufsize)], &rbuf[..rbl.min(bufsize)], "subst out diff [{}]", ctx);
        }
        (c.code_free)(cc);
        (r.code_free)(rc);
    }
}

#[test]
fn row_substitute() {
    let (c, r) = both();
    // row 30: plain group refs
    subst_case(&c, &r, b"(\\w+)@(\\w+)", 0, b"user@host", b"$2.$1", 0, 128, "plain");
    // row 31: global
    subst_case(&c, &r, b"a", 0, b"banana", b"X", PCRE2_SUBSTITUTE_GLOBAL, 128, "global");
    subst_case(&c, &r, b"\\d", 0, b"a1b2c3", b"[$0]", PCRE2_SUBSTITUTE_GLOBAL, 128, "global2");
    // row 32: literal
    subst_case(&c, &r, b"a", 0, b"banana", b"$1\\n", PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_LITERAL, 128, "literal");
    // row 33: extended case forcing
    subst_case(&c, &r, b"(\\w+)", 0, b"hello world", b"\\U$1\\E", PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED, 128, "upper");
    subst_case(&c, &r, b"(\\w+)", 0, b"HELLO", b"\\L$1", PCRE2_SUBSTITUTE_EXTENDED, 128, "lower");
    subst_case(&c, &r, b"(\\w+)", 0, b"hello", b"\\u$1", PCRE2_SUBSTITUTE_EXTENDED, 128, "titlefirst");
    // row 34: named and default forms
    subst_case(&c, &r, b"(?<w>\\w+)", 0, b"hi", b"${w}!", 0, 128, "named");
    subst_case(&c, &r, b"(a)(b)?", 0, b"a", b"${2:-none}", PCRE2_SUBSTITUTE_EXTENDED, 128, "default");
    subst_case(&c, &r, b"(a)(b)?", 0, b"ab", b"${2:+yes:no}", PCRE2_SUBSTITUTE_EXTENDED, 128, "setunset");
    // row 35: overflow length computation (tiny buffer)
    subst_case(&c, &r, b"a", 0, b"banana", b"XYZ", PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH, 4, "overflow");
    // row 36: replacement only
    subst_case(&c, &r, b"(\\w+)@(\\w+)", 0, b"user@host", b"$2", PCRE2_SUBSTITUTE_REPLACEMENT_ONLY, 128, "reponly");
}

// Row 37: serialize encode+decode+re-match.
#[test]
fn row_serialize() {
    let (c, r) = both();
    let pats: &[&[u8]] = &[b"abc", b"(a)(b)(c)", b"a.*c", b"(?<n>\\d+)"];
    unsafe {
        for pat in pats {
            let cc = compile(&c, pat, 0);
            let rc = compile(&r, pat, 0);
            let ccodes = [cc as *const c_void];
            let rcodes = [rc as *const c_void];
            let mut cbytes: *mut u8 = std::ptr::null_mut();
            let mut rbytes: *mut u8 = std::ptr::null_mut();
            let mut cn = 0usize;
            let mut rn = 0usize;
            let cenc = (c.serialize_encode)(ccodes.as_ptr(), 1, &mut cbytes, &mut cn, std::ptr::null_mut());
            let renc = (r.serialize_encode)(rcodes.as_ptr(), 1, &mut rbytes, &mut rn, std::ptr::null_mut());
            assert_eq!(cenc, renc, "encode rc diff");
            assert!(cenc >= 0);
            // number of codes in each blob
            let cgc = (c.serialize_get_num)(cbytes);
            let rgc = (r.serialize_get_num)(rbytes);
            assert_eq!(cgc, rgc, "get_number diff");
            // decode and re-match on each engine's own blob
            let mut cdec: *mut c_void = std::ptr::null_mut();
            let mut rdec: *mut c_void = std::ptr::null_mut();
            let cd = (c.serialize_decode)(&mut cdec, 1, cbytes, std::ptr::null_mut());
            let rd = (r.serialize_decode)(&mut rdec, 1, rbytes, std::ptr::null_mut());
            assert_eq!(cd, rd, "decode rc diff");
            // re-match a subject via decoded pattern
            let subj = b"abc";
            let cmd = (c.md_create)(10, std::ptr::null_mut());
            let rmd = (r.md_create)(10, std::ptr::null_mut());
            let crc = (c.r#match)(cdec, subj.as_ptr(), subj.len(), 0, 0, cmd, std::ptr::null_mut());
            let rrc = (r.r#match)(rdec, subj.as_ptr(), subj.len(), 0, 0, rmd, std::ptr::null_mut());
            assert_eq!(crc, rrc, "decoded match rc diff");
            (c.md_free)(cmd);
            (r.md_free)(rmd);
            (c.code_free)(cdec);
            (r.code_free)(rdec);
            (c.serialize_free)(cbytes);
            (r.serialize_free)(rbytes);
            (c.code_free)(cc);
            (r.code_free)(rc);
        }
    }
}

// Row 39: config selectors.
#[test]
fn row_config() {
    let (c, r) = both();
    let sels = [
        PCRE2_CONFIG_BSR, PCRE2_CONFIG_JIT, PCRE2_CONFIG_LINKSIZE,
        PCRE2_CONFIG_MATCHLIMIT, PCRE2_CONFIG_NEWLINE, PCRE2_CONFIG_PARENSLIMIT,
        PCRE2_CONFIG_UNICODE, PCRE2_CONFIG_DEPTHLIMIT, PCRE2_CONFIG_HEAPLIMIT,
    ];
    unsafe {
        for &sel in &sels {
            let mut cv: u64 = 0;
            let mut rv: u64 = 0;
            let cr = (c.config)(sel, &mut cv as *mut u64 as *mut c_void);
            let rr = (r.config)(sel, &mut rv as *mut u64 as *mut c_void);
            assert_eq!(cr, rr, "config rc diff sel={}", sel);
            if cr >= 0 {
                assert_eq!(cv, rv, "config value diff sel={}", sel);
            }
        }
    }
}

// Row 38: maketables + compile w/ custom tables + match.
#[test]
fn row_maketables() {
    let (c, r) = both();
    unsafe {
        let ctab = (c.maketables)(std::ptr::null_mut());
        let rtab = (r.maketables)(std::ptr::null_mut());
        assert!(!ctab.is_null() && !rtab.is_null());
        // The 1088-byte tables should be byte-identical.
        let cs = std::slice::from_raw_parts(ctab, 1088);
        let rs = std::slice::from_raw_parts(rtab, 1088);
        assert_eq!(cs, rs, "maketables bytes differ");

        // Compile with custom tables via a compile context, then match.
        let cctx = (c.cctx_create)(std::ptr::null_mut());
        let rctx = (r.cctx_create)(std::ptr::null_mut());
        (c.set_char_tables)(cctx, ctab);
        (r.set_char_tables)(rctx, rtab);
        let pat: &[u8] = b"[[:alpha:]]+";
        let mut ec = 0; let mut eo = 0;
        let cc = (c.compile)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, cctx);
        let rc = (r.compile)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, rctx);
        assert!(!cc.is_null() && !rc.is_null());
        let subj = b"abc123";
        let cmd = (c.md_create)(10, std::ptr::null_mut());
        let rmd = (r.md_create)(10, std::ptr::null_mut());
        let crc = (c.r#match)(cc, subj.as_ptr(), subj.len(), 0, 0, cmd, std::ptr::null_mut());
        let rrc = (r.r#match)(rc, subj.as_ptr(), subj.len(), 0, 0, rmd, std::ptr::null_mut());
        assert_eq!(crc, rrc, "custom-table match rc diff");
        let cop = (c.ovector_ptr)(cmd);
        let rop = (r.ovector_ptr)(rmd);
        assert_eq!(*cop.add(0), *rop.add(0));
        assert_eq!(*cop.add(1), *rop.add(1));
        (c.md_free)(cmd); (r.md_free)(rmd);
        (c.code_free)(cc); (r.code_free)(rc);
        (c.cctx_free)(cctx); (r.cctx_free)(rctx);
        (c.maketables_free)(std::ptr::null_mut(), ctab);
        (r.maketables_free)(std::ptr::null_mut(), rtab);
    }
}

// Row 40: get_error_message across common error codes.
#[test]
fn row_error_message() {
    let (c, r) = both();
    unsafe {
        for code in [-1i32, -2, -33, -34, -35, -48, -49, -51, -55, -58, 100, 101, 150, 200] {
            let mut cbuf = [0u8; 256];
            let mut rbuf = [0u8; 256];
            let cn = (c.get_err_msg)(code, cbuf.as_mut_ptr(), cbuf.len());
            let rn = (r.get_err_msg)(code, rbuf.as_mut_ptr(), rbuf.len());
            assert_eq!(cn, rn, "err_msg rc diff code={}", code);
            if cn > 0 {
                assert_eq!(&cbuf[..cn as usize], &rbuf[..rn as usize], "err_msg text diff code={}", code);
            }
        }
    }
}
