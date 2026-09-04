//! Phase B: full end-to-end pipeline (CONFIGS.md row 109) and the in-pattern
//! startup directives (row 42) driven with subjects that exercise them.
mod harness;
use harness::*;
use std::ffi::c_void;
use std::os::raw::c_int;

#[derive(Debug, PartialEq, Eq)]
struct PipeOut {
    compile_err: c_int,
    compile_off: Sz,
    info: Option<InfoOut>,
    match_rc: c_int,
    ovector: Vec<Sz>,
    startchar: Sz,
    mark: Option<Vec<u8>>,
    substrings: Vec<(c_int, Option<Vec<u8>>)>,
    substitute: (c_int, Sz, Vec<u8>),
    serialized: Vec<u8>,
    decoded_match: (c_int, Vec<Sz>),
    copy_match: (c_int, Vec<Sz>),
    converted: (c_int, Vec<u8>),
}

/// compile -> info -> match -> substrings -> substitute -> serialize -> decode
/// -> match again -> code_copy -> match again, plus a glob conversion, all with
/// one configuration.
fn pipeline(api: &Api, pat: &str, subject: &[u8], copts: u32, xopts: u32, nl: u32, bsr: u32,
            mopts: u32) -> PipeOut {
    unsafe {
        let cc = (api.compile_context_create)(std::ptr::null_mut());
        (api.set_compile_extra_options)(cc, xopts);
        (api.set_newline)(cc, nl);
        (api.set_bsr)(cc, bsr);
        let mut err: c_int = 0;
        let mut off: Sz = 0;
        let pb = pat.as_bytes();
        let code = (api.compile)(pb.as_ptr(), pb.len(), copts, &mut err, &mut off, cc);
        if code.is_null() {
            (api.compile_context_free)(cc);
            return PipeOut {
                compile_err: err,
                compile_off: off,
                info: None,
                match_rc: 0,
                ovector: Vec::new(),
                startchar: 0,
                mark: None,
                substrings: Vec::new(),
                substitute: (0, 0, Vec::new()),
                serialized: Vec::new(),
                decoded_match: (0, Vec::new()),
                copy_match: (0, Vec::new()),
                converted: (0, Vec::new()),
            };
        }
        let info = api.info(code);
        let mut capcount: u32 = 0;
        (api.pattern_info)(code, 4, &mut capcount as *mut u32 as *mut c_void);

        let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
        let rc = (api.do_match)(code, subject.as_ptr(), subject.len(), 0, mopts, md,
                                std::ptr::null_mut());
        let m = api.read_match(md, rc, false, capcount);

        let mut substrings = Vec::new();
        for g in 0..(capcount + 2) {
            let mut p: *mut u8 = std::ptr::null_mut();
            let mut len: Sz = 0;
            let grc = (api.substring_get_bynumber)(md, g, &mut p, &mut len);
            let v = if grc == 0 && !p.is_null() {
                let v = std::slice::from_raw_parts(p, len).to_vec();
                (api.substring_free)(p);
                Some(v)
            } else {
                None
            };
            substrings.push((grc, v));
        }

        let mut obuf = vec![0xaau8; 256];
        let mut olen: Sz = obuf.len();
        let repl = b"<$0|$1>";
        let src = (api.substitute)(
            code,
            subject.as_ptr(),
            subject.len(),
            0,
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_EXTENDED,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            repl.as_ptr(),
            repl.len(),
            obuf.as_mut_ptr(),
            &mut olen,
        );
        let sout = if src >= 0 { obuf[..olen.min(obuf.len())].to_vec() } else { Vec::new() };

        // serialize -> decode -> match
        let codes = [code];
        let mut bytes: *mut u8 = std::ptr::null_mut();
        let mut size: Sz = 0;
        let erc = (api.serialize_encode)(codes.as_ptr(), 1, &mut bytes, &mut size,
                                         std::ptr::null_mut());
        let mut serialized = Vec::new();
        let mut decoded_match = (0, Vec::new());
        if erc > 0 {
            serialized = std::slice::from_raw_parts(bytes, size).to_vec();
            let mut dec: [Code; 1] = [std::ptr::null_mut()];
            let drc = (api.serialize_decode)(dec.as_mut_ptr(), 1, bytes, std::ptr::null_mut());
            if drc > 0 {
                let md2 = (api.match_data_create_from_pattern)(dec[0], std::ptr::null_mut());
                let rc2 = (api.do_match)(dec[0], subject.as_ptr(), subject.len(), 0, mopts, md2,
                                         std::ptr::null_mut());
                let m2 = api.read_match(md2, rc2, false, capcount);
                decoded_match = (rc2, m2.ovector);
                (api.match_data_free)(md2);
                (api.code_free)(dec[0]);
            }
            (api.serialize_free)(bytes);
        }

        // code_copy_with_tables -> match
        let cp = (api.code_copy_with_tables)(code);
        let copy_match = if cp.is_null() {
            (0, Vec::new())
        } else {
            let md3 = (api.match_data_create_from_pattern)(cp, std::ptr::null_mut());
            let rc3 = (api.do_match)(cp, subject.as_ptr(), subject.len(), 0, mopts, md3,
                                     std::ptr::null_mut());
            let m3 = api.read_match(md3, rc3, false, capcount);
            (api.match_data_free)(md3);
            (api.code_free)(cp);
            (rc3, m3.ovector)
        };

        // glob conversion of the same text
        let vc = (api.convert_context_create)(std::ptr::null_mut());
        let mut cbuf: *mut u8 = std::ptr::null_mut();
        let mut clen: Sz = 0;
        let crc = (api.pattern_convert)(pb.as_ptr(), pb.len(), PCRE2_CONVERT_GLOB, &mut cbuf,
                                        &mut clen, vc);
        let conv = if crc == 0 && !cbuf.is_null() {
            let v = std::slice::from_raw_parts(cbuf, clen).to_vec();
            (api.converted_pattern_free)(cbuf);
            v
        } else {
            Vec::new()
        };
        (api.convert_context_free)(vc);

        (api.match_data_free)(md);
        (api.code_free)(code);
        (api.compile_context_free)(cc);
        PipeOut {
            compile_err: err,
            compile_off: off,
            info: Some(info),
            match_rc: rc,
            ovector: m.ovector,
            startchar: m.startchar,
            mark: m.mark,
            substrings,
            substitute: (src, olen, sout),
            serialized,
            decoded_match,
            copy_match,
            converted: (crc, conv),
        }
    }
}

#[test]
fn end_to_end_pipeline() {
    let mut rng = Rng::new(0x5EED_E2E0);
    let copt_pool: &[u32] = &[
        0,
        PCRE2_CASELESS,
        PCRE2_MULTILINE,
        PCRE2_DOTALL,
        PCRE2_UTF,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_UCP,
        PCRE2_DUPNAMES,
        PCRE2_EXTENDED,
        PCRE2_ALT_EXTENDED_CLASS,
        PCRE2_NO_AUTO_CAPTURE,
        PCRE2_MULTILINE | PCRE2_CASELESS | PCRE2_DOTALL,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_USE_OFFSET_LIMIT,
        PCRE2_LITERAL,
    ];
    let xopt_pool: &[u32] = &[
        0,
        PCRE2_EXTRA_CASELESS_RESTRICT,
        PCRE2_EXTRA_MATCH_WORD,
        PCRE2_EXTRA_MATCH_LINE,
        PCRE2_EXTRA_ASCII_BSW | PCRE2_EXTRA_ASCII_BSD,
        PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
    ];
    let mopt_pool: &[u32] = &[
        0,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_NOTEMPTY,
        PCRE2_NOTEMPTY_ATSTART,
        PCRE2_ANCHORED,
        PCRE2_NO_UTF_CHECK,
        PCRE2_COPY_MATCHED_SUBJECT,
    ];
    let mut done = 0usize;
    for i in 0..12000u32 {
        let pat = if i % 3 == 0 {
            (*rng.pick(&curated_patterns())).to_string()
        } else {
            let d = rng.range(1, 3) as u32;
            random_pattern(&mut rng, d)
        };
        let copts = *rng.pick(copt_pool);
        let subject = if rng.bool() {
            (*rng.pick(&curated_subjects())).as_bytes().to_vec()
        } else {
            random_subject(&mut rng, copts & PCRE2_UTF != 0)
        };
        let xopts = *rng.pick(xopt_pool);
        let nl = rng.range(1, 6) as u32;
        let bsr = rng.range(1, 2) as u32;
        let mopts = *rng.pick(mopt_pool);
        let co = pipeline(c(), &pat, &subject, copts, xopts, nl, bsr, mopts);
        let ro = pipeline(r(), &pat, &subject, copts, xopts, nl, bsr, mopts);
        if co != ro {
            panic!(
                "PIPELINE DIVERGENCE iter={i}\n pat={pat:?} copts={copts:#x} xopts={xopts:#x} nl={nl} bsr={bsr} mopts={mopts:#x}\n subject={:?}\n C   ={co:#?}\n Rust={ro:#?}",
                String::from_utf8_lossy(&subject)
            );
        }
        done += 1;
    }
    eprintln!("pipeline configurations compared: {done}");
}

/// Row 42: in-pattern startup directives, with subjects chosen to make each one
/// observable.
#[test]
fn in_pattern_directives() {
    let directives: &[(&str, &[&str])] = &[
        ("(*UTF)\\x{e9}", &["é", "\u{e9}x", "e"]),
        ("(*UCP)\\w+", &["abcé", "日本", "___"]),
        ("(*UTF)(*UCP)\\w+", &["abcé", "日本語", "1２3"]),
        ("(*CR)a$", &["a\r", "a\n", "a", "a\r\n"]),
        ("(*LF)a$", &["a\r", "a\n", "a", "a\r\n"]),
        ("(*CRLF)a$", &["a\r", "a\n", "a", "a\r\n"]),
        ("(*ANY)a$", &["a\r", "a\n", "a\u{85}", "a\u{2028}"]),
        ("(*ANYCRLF)a$", &["a\r", "a\n", "a\r\n", "a\u{85}"]),
        ("(*NUL)a$", &["a\0", "a\n", "a"]),
        ("(*BSR_ANYCRLF)\\R", &["\r", "\n", "\r\n", "\u{85}", "\u{b}"]),
        ("(*BSR_UNICODE)\\R", &["\r", "\n", "\r\n", "\u{85}", "\u{b}"]),
        ("(*LIMIT_MATCH=1)(a+)+b", &["aaaaaaaaaaaaaaaac", "ab"]),
        ("(*LIMIT_DEPTH=1)(a+)+b", &["aaaaaaaaaaaaaaaac", "ab"]),
        ("(*LIMIT_HEAP=1)(a+)+b", &["aaaaaaaaaaaaaaaac", "ab"]),
        ("(*LIMIT_MATCH=100000)a", &["a"]),
        ("(*NOTEMPTY)a*", &["", "b", "aa"]),
        ("(*NOTEMPTY_ATSTART)a*", &["", "b", "aa", "ba"]),
        ("(*NO_AUTO_POSSESS)a*+b", &["aaab", "aaa"]),
        ("(*NO_DOTSTAR_ANCHOR).*b", &["xxb", "b", ""]),
        ("(*NO_START_OPT)abc", &["xxabc", "abc", "ab"]),
        ("(*NO_JIT)abc", &["abc", "xabc"]),
        ("(*CR)(*BSR_ANYCRLF)(*NO_START_OPT)a$", &["a\r", "a\n"]),
        ("(*UTF)(*CRLF)(*LIMIT_DEPTH=1000).$", &["é\r\n", "é"]),
    ];
    for (pat, subs) in directives {
        for s in *subs {
            for nl in 1..=6u32 {
                for bsr in 1..=2u32 {
                    for mopts in [0, PCRE2_NOTBOL, PCRE2_NOTEOL, PCRE2_ANCHORED] {
                        let co = pipeline(c(), pat, s.as_bytes(), 0, 0, nl, bsr, mopts);
                        let ro = pipeline(r(), pat, s.as_bytes(), 0, 0, nl, bsr, mopts);
                        assert!(
                            co == ro,
                            "DIRECTIVE DIVERGENCE pat={pat:?} subj={s:?} nl={nl} bsr={bsr} mopts={mopts:#x}\n C   ={co:#?}\n Rust={ro:#?}"
                        );
                    }
                }
            }
        }
    }
}
