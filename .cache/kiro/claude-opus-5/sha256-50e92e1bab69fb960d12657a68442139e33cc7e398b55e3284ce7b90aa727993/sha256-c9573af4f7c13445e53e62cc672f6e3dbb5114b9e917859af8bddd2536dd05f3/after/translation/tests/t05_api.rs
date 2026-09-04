//! Phase B/C: serialization, code copying, contexts, tables, config, error
//! messages, match-data sizes and the JIT stubs.
//! CONFIGS.md rows 49, 77, 81-83, 85-90, 108; ERRORS.md rows 106-126, 144-159.
mod harness;
use harness::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------- rows 81, 82
const SER_PATS: &[&str] = &[
    "abc",
    "(a)(b)(?<n>c)",
    "\\d+[a-z]*",
    "(?i)HELLO",
    "(?<x>a)|(?<y>b)",
    "\\p{L}+",
    "^(?:foo|bar)$",
    "a{2,10}?b",
    "(?R)?x",
    "(*MARK:m)a",
];

fn serialize_roundtrip(api: &Api, pats: &[&str], with_tables: bool) -> (i32, Vec<u8>, i32, Vec<(c_int, Vec<Sz>)>) {
    unsafe {
        let cc = (api.compile_context_create)(std::ptr::null_mut());
        let mut tables = std::ptr::null();
        if with_tables {
            tables = (api.maketables)(std::ptr::null_mut());
            (api.set_character_tables)(cc, tables);
        }
        let mut codes: Vec<Code> = Vec::new();
        for p in pats {
            let mut err = 0;
            let mut off = 0;
            let b = p.as_bytes();
            let code = (api.compile)(b.as_ptr(), b.len(), 0, &mut err, &mut off, cc);
            assert!(!code.is_null(), "{p} err {err}");
            codes.push(code);
        }
        let mut bytes: *mut u8 = std::ptr::null_mut();
        let mut size: Sz = 0;
        let erc = (api.serialize_encode)(
            codes.as_ptr(),
            codes.len() as i32,
            &mut bytes,
            &mut size,
            std::ptr::null_mut(),
        );
        let blob = if erc > 0 && !bytes.is_null() {
            std::slice::from_raw_parts(bytes, size).to_vec()
        } else {
            Vec::new()
        };
        let ncodes = if blob.is_empty() {
            0
        } else {
            (api.serialize_get_number_of_codes)(blob.as_ptr())
        };
        // decode and match with each decoded code
        let mut results = Vec::new();
        if erc > 0 {
            let mut decoded: Vec<Code> = vec![std::ptr::null_mut(); codes.len()];
            let drc = (api.serialize_decode)(
                decoded.as_mut_ptr(),
                codes.len() as i32,
                bytes,
                std::ptr::null_mut(),
            );
            assert!(drc > 0, "decode failed: {drc}");
            for (i, dc) in decoded.iter().enumerate() {
                let md = (api.match_data_create_from_pattern)(*dc, std::ptr::null_mut());
                for subj in ["abc", "a1b", "HELLOx", "ax", "b", "\u{e9}", "foo", "aab", "xx"] {
                    let s = subj.as_bytes();
                    let rc = (api.do_match)(*dc, s.as_ptr(), s.len(), 0, 0, md,
                                            std::ptr::null_mut());
                    let n = (api.get_ovector_count)(md);
                    let ov = if rc > 0 {
                        std::slice::from_raw_parts(
                            (api.get_ovector_pointer)(md),
                            (rc as usize).min(n as usize) * 2,
                        )
                        .to_vec()
                    } else {
                        Vec::new()
                    };
                    results.push((rc, ov));
                }
                (api.match_data_free)(md);
                let _ = i;
            }
            for dc in decoded {
                (api.code_free)(dc);
            }
        }
        if !bytes.is_null() {
            (api.serialize_free)(bytes);
        }
        for code in codes {
            (api.code_free)(code);
        }
        if !tables.is_null() {
            (api.maketables_free)(std::ptr::null_mut(), tables);
        }
        (api.compile_context_free)(cc);
        (erc, blob, ncodes, results)
    }
}

#[test]
fn serialize_encode_decode() {
    for n in 1..=SER_PATS.len() {
        for with_tables in [false, true] {
            let subset = &SER_PATS[..n];
            let co = serialize_roundtrip(c(), subset, with_tables);
            let ro = serialize_roundtrip(r(), subset, with_tables);
            assert_eq!(co.0, ro.0, "encode rc differs (n={n}, tables={with_tables})");
            assert_eq!(
                co.1, ro.1,
                "serialized bytes differ (n={n}, tables={with_tables})"
            );
            assert_eq!(co.2, ro.2, "number_of_codes differs");
            assert_eq!(co.3, ro.3, "post-decode match results differ");
        }
    }
}

// ------------------------------------------------------- ERRORS.md rows 112-126
#[test]
fn serialize_error_paths() {
    unsafe {
        let mut all = Vec::new();
        for api in [c(), r()] {
            let mut v: Vec<i32> = Vec::new();
            let mut err = 0;
            let mut off = 0;
            let code = (api.compile)(b"abc".as_ptr(), 3, 0, &mut err, &mut off,
                                     std::ptr::null_mut());
            let codes = [code];
            let mut bytes: *mut u8 = std::ptr::null_mut();
            let mut size: Sz = 0;

            // row 112: NULL arguments
            v.push((api.serialize_encode)(std::ptr::null(), 1, &mut bytes, &mut size,
                                          std::ptr::null_mut()));
            v.push((api.serialize_encode)(codes.as_ptr(), 1, std::ptr::null_mut(), &mut size,
                                          std::ptr::null_mut()));
            v.push((api.serialize_encode)(codes.as_ptr(), 1, &mut bytes, std::ptr::null_mut(),
                                          std::ptr::null_mut()));
            // row 113: number_of_codes <= 0
            for n in [0i32, -1, i32::MIN] {
                v.push((api.serialize_encode)(codes.as_ptr(), n, &mut bytes, &mut size,
                                              std::ptr::null_mut()));
            }
            // row 114: codes[i] == NULL
            let nullcodes: [Code; 2] = [code, std::ptr::null_mut()];
            v.push((api.serialize_encode)(nullcodes.as_ptr(), 2, &mut bytes, &mut size,
                                          std::ptr::null_mut()));
            // row 115: bad magic
            let mut fake = vec![0u8; 4096];
            let fakecodes: [Code; 1] = [fake.as_mut_ptr() as Code];
            v.push((api.serialize_encode)(fakecodes.as_ptr(), 1, &mut bytes, &mut size,
                                          std::ptr::null_mut()));
            // row 116: mixed tables
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            let tables = (api.maketables)(std::ptr::null_mut());
            (api.set_character_tables)(cc, tables);
            let code2 = (api.compile)(b"xyz".as_ptr(), 3, 0, &mut err, &mut off, cc);
            let mixed: [Code; 2] = [code, code2];
            v.push((api.serialize_encode)(mixed.as_ptr(), 2, &mut bytes, &mut size,
                                          std::ptr::null_mut()));

            // a good stream, for decode-side corruption tests
            let good = {
                let mut b: *mut u8 = std::ptr::null_mut();
                let mut s: Sz = 0;
                let rc = (api.serialize_encode)(codes.as_ptr(), 1, &mut b, &mut s,
                                                std::ptr::null_mut());
                assert!(rc > 0);
                let blob = std::slice::from_raw_parts(b, s).to_vec();
                (api.serialize_free)(b);
                blob
            };
            let mut decoded: [Code; 1] = [std::ptr::null_mut()];
            // row 117: NULL args
            v.push((api.serialize_decode)(std::ptr::null_mut(), 1, good.as_ptr(),
                                          std::ptr::null_mut()));
            v.push((api.serialize_decode)(decoded.as_mut_ptr(), 1, std::ptr::null(),
                                          std::ptr::null_mut()));
            // row 118
            for n in [0i32, -1] {
                v.push((api.serialize_decode)(decoded.as_mut_ptr(), n, good.as_ptr(),
                                              std::ptr::null_mut()));
            }
            // rows 119-123: corrupt magic / version / config / count / body
            for (label, patch) in [
                ("magic", 0usize),
                ("version", 8),
                ("config", 16),
                ("count", 24),
                ("body", 40),
            ] {
                let _ = label;
                let mut bad = good.clone();
                if patch < bad.len() {
                    bad[patch] ^= 0xff;
                }
                v.push((api.serialize_decode)(decoded.as_mut_ptr(), 1, bad.as_ptr(),
                                              std::ptr::null_mut()));
                v.push((api.serialize_get_number_of_codes)(bad.as_ptr()));
            }
            // truncation
            for cut in [1usize, 8, 16, 24, 32, good.len() / 2] {
                let bad = &good[..cut.min(good.len())];
                v.push((api.serialize_get_number_of_codes)(bad.as_ptr()));
            }
            // row 124: NULL bytes
            v.push((api.serialize_get_number_of_codes)(std::ptr::null()));

            (api.code_free)(code);
            (api.code_free)(code2);
            (api.maketables_free)(std::ptr::null_mut(), tables);
            (api.compile_context_free)(cc);
            all.push(v);
        }
        assert_eq!(all[0], all[1], "serialize error codes differ");
        eprintln!("serialize error codes = {:?}", all[0]);
        assert_eq!(all[0][0], PCRE2_ERROR_NULL);
        assert_eq!(all[0][3], PCRE2_ERROR_BADDATA);
        assert_eq!(all[0][6], PCRE2_ERROR_NULL);
        assert_eq!(all[0][7], PCRE2_ERROR_BADMAGIC);
        assert_eq!(all[0][8], PCRE2_ERROR_MIXEDTABLES);
    }
}

// -------------------------------------------------------------------- row 83
#[test]
fn code_copy_variants() {
    for p in SER_PATS {
        for with_tables in [false, true] {
            let mut outs = Vec::new();
            for api in [c(), r()] {
                unsafe {
                    let cc = (api.compile_context_create)(std::ptr::null_mut());
                    let mut tables = std::ptr::null();
                    if with_tables {
                        tables = (api.maketables)(std::ptr::null_mut());
                        (api.set_character_tables)(cc, tables);
                    }
                    let mut err = 0;
                    let mut off = 0;
                    let b = p.as_bytes();
                    let code = (api.compile)(b.as_ptr(), b.len(), 0, &mut err, &mut off, cc);
                    assert!(!code.is_null());
                    let c1 = (api.code_copy)(code);
                    let c2 = (api.code_copy_with_tables)(code);
                    assert!(!c1.is_null() && !c2.is_null());
                    let mut v: Vec<(c_int, Vec<Sz>, InfoOut)> = Vec::new();
                    for cd in [code, c1, c2] {
                        let md = (api.match_data_create_from_pattern)(cd, std::ptr::null_mut());
                        for subj in ["abc", "a1b", "HELLO", "ax", "b", "foo", "aab"] {
                            let s = subj.as_bytes();
                            let rc = (api.do_match)(cd, s.as_ptr(), s.len(), 0, 0, md,
                                                    std::ptr::null_mut());
                            let ov = if rc > 0 {
                                let n = (api.get_ovector_count)(md);
                                std::slice::from_raw_parts(
                                    (api.get_ovector_pointer)(md),
                                    (rc as usize).min(n as usize) * 2,
                                )
                                .to_vec()
                            } else {
                                Vec::new()
                            };
                            v.push((rc, ov, api.info(cd)));
                        }
                        (api.match_data_free)(md);
                    }
                    // NULL inputs (ERRORS.md rows 156, 157)
                    let n1 = (api.code_copy)(std::ptr::null_mut());
                    let n2 = (api.code_copy_with_tables)(std::ptr::null_mut());
                    assert!(n1.is_null() && n2.is_null());
                    (api.code_free)(c1);
                    (api.code_free)(c2);
                    (api.code_free)(code);
                    if !tables.is_null() {
                        (api.maketables_free)(std::ptr::null_mut(), tables);
                    }
                    (api.compile_context_free)(cc);
                    outs.push(v);
                }
            }
            assert!(outs[0] == outs[1], "code_copy divergence for {p:?} tables={with_tables}");
        }
    }
    // pcre2_code_free(NULL) must be a no-op in both
    unsafe {
        (c().code_free)(std::ptr::null_mut());
        (r().code_free)(std::ptr::null_mut());
    }
}

// -------------------------------------------------------------------- row 90
#[test]
fn maketables_identical() {
    unsafe {
        let ct = (c().maketables)(std::ptr::null_mut());
        let rt = (r().maketables)(std::ptr::null_mut());
        assert!(!ct.is_null() && !rt.is_null());
        // tables length comes from PCRE2_CONFIG_TABLES_LENGTH
        let mut len: u32 = 0;
        (c().config)(15, &mut len as *mut u32 as *mut c_void);
        let mut len2: u32 = 0;
        (r().config)(15, &mut len2 as *mut u32 as *mut c_void);
        assert_eq!(len, len2, "TABLES_LENGTH differs");
        assert!(len > 0);
        let a = std::slice::from_raw_parts(ct, len as usize);
        let b = std::slice::from_raw_parts(rt, len as usize);
        assert_eq!(a, b, "maketables output differs");
        // and identical to the built-in default tables
        let da = std::slice::from_raw_parts(c().d_default_tables, len as usize);
        assert_eq!(a, da, "maketables differs from default tables (C)");
        (c().maketables_free)(std::ptr::null_mut(), ct);
        (r().maketables_free)(std::ptr::null_mut(), rt);
    }
}

// -------------------------------------------------------------------- row 77
#[test]
fn config_all_options() {
    unsafe {
        let mut all = Vec::new();
        for api in [c(), r()] {
            let mut v: Vec<(u32, c_int, Vec<u8>)> = Vec::new();
            for what in 0..20u32 {
                // size query
                let szrc = (api.config)(what, std::ptr::null_mut());
                let mut buf = [0u8; 64];
                let rc = (api.config)(what, buf.as_mut_ptr() as *mut c_void);
                v.push((what, szrc, if rc >= 0 { buf.to_vec() } else { Vec::new() }));
                v.push((what, rc, Vec::new()));
            }
            // out-of-range (ERRORS.md rows 106, 176)
            for what in [20u32, 99, 999, u32::MAX, u32::MAX - 1] {
                let mut buf = [0u8; 64];
                v.push((what, (api.config)(what, buf.as_mut_ptr() as *mut c_void), Vec::new()));
                v.push((what, (api.config)(what, std::ptr::null_mut()), Vec::new()));
            }
            all.push(v);
        }
        assert_eq!(all[0], all[1], "pcre2_config output differs");
        // JITTARGET must be BADOPTION in a non-JIT build; JIT must report 0.
        let jit = all[0].iter().find(|e| e.0 == 1).unwrap();
        assert_eq!(jit.1, 4, "CONFIG_JIT should return the uint32 size");
    }
}

// ---------------------------------------------------------- rows 85, err 109-111
#[test]
fn error_messages() {
    unsafe {
        for code in -200i32..=300 {
            for bufsize in [0usize, 1, 2, 8, 64, 256] {
                let mut cb = vec![0xaau8; bufsize.max(1)];
                let mut rb = vec![0xaau8; bufsize.max(1)];
                let crc = (c().get_error_message)(code, cb.as_mut_ptr(), bufsize);
                let rrc = (r().get_error_message)(code, rb.as_mut_ptr(), bufsize);
                assert_eq!(crc, rrc, "get_error_message({code},{bufsize}) rc");
                if crc >= 0 {
                    assert_eq!(
                        &cb[..(crc as usize + 1).min(cb.len())],
                        &rb[..(crc as usize + 1).min(rb.len())],
                        "get_error_message({code},{bufsize}) text"
                    );
                }
            }
        }
        // extreme codes
        for code in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, 1000, -1000] {
            let mut cb = [0xaau8; 128];
            let mut rb = [0xaau8; 128];
            let crc = (c().get_error_message)(code, cb.as_mut_ptr(), 128);
            let rrc = (r().get_error_message)(code, rb.as_mut_ptr(), 128);
            assert_eq!(crc, rrc, "get_error_message({code}) rc");
            if crc >= 0 {
                assert_eq!(cb, rb, "get_error_message({code}) text");
            }
        }
    }
}

// ----------------------------------------------------------------- rows 86, 87
#[test]
fn match_data_sizes_and_accessors() {
    unsafe {
        for ovec in 0..40u32 {
            let cs_ = (c().match_data_create)(ovec, std::ptr::null_mut());
            let rs_ = (r().match_data_create)(ovec, std::ptr::null_mut());
            assert_eq!(
                (c().get_match_data_size)(cs_),
                (r().get_match_data_size)(rs_),
                "match_data_size({ovec})"
            );
            assert_eq!(
                (c().get_match_data_heapframes_size)(cs_),
                (r().get_match_data_heapframes_size)(rs_),
                "heapframes_size({ovec}) before match"
            );
            assert_eq!(
                (c().get_ovector_count)(cs_),
                (r().get_ovector_count)(rs_),
                "ovector_count({ovec})"
            );
            (c().match_data_free)(cs_);
            (r().match_data_free)(rs_);
        }
        // after a match, heapframes have been allocated
        for p in ["(a)(b)(c)", "a", "((((a))))", "(?R)?a"] {
            let mut sizes = Vec::new();
            for api in [c(), r()] {
                let mut err = 0;
                let mut off = 0;
                let b = p.as_bytes();
                let code = (api.compile)(b.as_ptr(), b.len(), 0, &mut err, &mut off,
                                         std::ptr::null_mut());
                let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                let s = b"abcabc";
                let rc = (api.do_match)(code, s.as_ptr(), 6, 0, 0, md, std::ptr::null_mut());
                sizes.push((
                    rc,
                    (api.get_match_data_size)(md),
                    (api.get_match_data_heapframes_size)(md),
                    (api.get_ovector_count)(md),
                    (api.get_startchar)(md),
                ));
                (api.match_data_free)(md);
                (api.code_free)(code);
            }
            assert_eq!(sizes[0], sizes[1], "sizes after match differ for {p:?}");
        }
        // match_data_create_from_pattern(NULL) -> NULL (ERRORS.md row 155)
        assert!((c().match_data_create_from_pattern)(std::ptr::null_mut(), std::ptr::null_mut())
            .is_null());
        assert!((r().match_data_create_from_pattern)(std::ptr::null_mut(), std::ptr::null_mut())
            .is_null());
        // free(NULL) is a no-op
        (c().match_data_free)(std::ptr::null_mut());
        (r().match_data_free)(std::ptr::null_mut());
    }
}

// ------------------------------------------------------------------ rows 88, 89
static mut ALLOC_CALLS: usize = 0;

unsafe extern "C" fn my_malloc(size: Sz, data: *mut c_void) -> *mut c_void {
    unsafe {
        ALLOC_CALLS += 1;
        let _ = data;
        let layout = std::alloc::Layout::from_size_align(size.max(1) + 16, 16).unwrap();
        let p = std::alloc::alloc(layout);
        // store the size so free can reconstruct the layout
        (p as *mut Sz).write(size.max(1) + 16);
        p.add(16) as *mut c_void
    }
}
unsafe extern "C" fn my_free(p: *mut c_void, data: *mut c_void) {
    unsafe {
        let _ = data;
        if p.is_null() {
            return;
        }
        let base = (p as *mut u8).sub(16);
        let size = (base as *mut Sz).read();
        let layout = std::alloc::Layout::from_size_align(size, 16).unwrap();
        std::alloc::dealloc(base, layout);
    }
}

#[test]
fn custom_allocator_and_context_copies() {
    let mut outs = Vec::new();
    for api in [c(), r()] {
        unsafe {
            let gc = (api.general_context_create)(Some(my_malloc), Some(my_free),
                                                  std::ptr::null_mut());
            assert!(!gc.is_null());
            let gc2 = (api.general_context_copy)(gc);
            assert!(!gc2.is_null());
            let cc = (api.compile_context_create)(gc);
            let mc = (api.match_context_create)(gc);
            let vc = (api.convert_context_create)(gc);
            assert!(!cc.is_null() && !mc.is_null() && !vc.is_null());
            let mut rcs: Vec<c_int> = Vec::new();
            // exercise every setter, then copy the contexts and use the copies
            rcs.push((api.set_bsr)(cc, PCRE2_BSR_ANYCRLF));
            rcs.push((api.set_newline)(cc, PCRE2_NEWLINE_ANYCRLF));
            rcs.push((api.set_max_pattern_length)(cc, 1000));
            rcs.push((api.set_max_pattern_compiled_length)(cc, 100000));
            rcs.push((api.set_max_varlookbehind)(cc, 100));
            rcs.push((api.set_parens_nest_limit)(cc, 100));
            rcs.push((api.set_compile_extra_options)(cc, PCRE2_EXTRA_MATCH_WORD));
            rcs.push((api.set_optimize)(cc, PCRE2_OPTIMIZATION_FULL));
            rcs.push((api.set_compile_recursion_guard)(cc, None, std::ptr::null_mut()));
            rcs.push((api.set_character_tables)(cc, std::ptr::null()));
            rcs.push((api.set_match_limit)(mc, 5000));
            rcs.push((api.set_depth_limit)(mc, 5000));
            rcs.push((api.set_heap_limit)(mc, 5000));
            rcs.push((api.set_offset_limit)(mc, PCRE2_UNSET));
            rcs.push((api.set_recursion_limit)(mc, 4321));
            rcs.push((api.set_recursion_memory_management)(
                mc,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ));
            rcs.push((api.set_callout)(mc, None, std::ptr::null_mut()));
            rcs.push((api.set_substitute_callout)(mc, None, std::ptr::null_mut()));
            rcs.push((api.set_substitute_case_callout)(mc, None, std::ptr::null_mut()));
            rcs.push((api.set_glob_separator)(vc, b'/' as u32));
            rcs.push((api.set_glob_escape)(vc, b'\\' as u32));
            let cc2 = (api.compile_context_copy)(cc);
            let mc2 = (api.match_context_copy)(mc);
            let vc2 = (api.convert_context_copy)(vc);
            assert!(!cc2.is_null() && !mc2.is_null() && !vc2.is_null());
            // compile+match through the copied contexts
            let mut err = 0;
            let mut off = 0;
            let pat = b"(?<w>\\w+)";
            let code = (api.compile)(pat.as_ptr(), pat.len(), 0, &mut err, &mut off, cc2);
            assert!(!code.is_null(), "compile through copied ctx failed: {err}");
            let md = (api.match_data_create_from_pattern)(code, gc2);
            let s = b"  hello  ";
            let rc = (api.do_match)(code, s.as_ptr(), s.len(), 0, 0, md, mc2);
            let n = (api.get_ovector_count)(md);
            let ov = if rc > 0 {
                std::slice::from_raw_parts((api.get_ovector_pointer)(md),
                                           (rc as usize).min(n as usize) * 2).to_vec()
            } else {
                Vec::new()
            };
            // convert through the copied convert context
            let mut buf: *mut u8 = std::ptr::null_mut();
            let mut blen: Sz = 0;
            let crc = (api.pattern_convert)(b"a*b".as_ptr(), 3, PCRE2_CONVERT_GLOB, &mut buf,
                                            &mut blen, vc2);
            let conv = if crc == 0 && !buf.is_null() {
                let v = std::slice::from_raw_parts(buf, blen).to_vec();
                (api.converted_pattern_free)(buf);
                v
            } else {
                Vec::new()
            };
            outs.push((rcs, rc, ov, crc, conv));
            (api.match_data_free)(md);
            (api.code_free)(code);
            (api.compile_context_free)(cc);
            (api.compile_context_free)(cc2);
            (api.match_context_free)(mc);
            (api.match_context_free)(mc2);
            (api.convert_context_free)(vc);
            (api.convert_context_free)(vc2);
            (api.general_context_free)(gc);
            (api.general_context_free)(gc2);
            // NULL frees must be no-ops
            (api.compile_context_free)(std::ptr::null_mut());
            (api.match_context_free)(std::ptr::null_mut());
            (api.convert_context_free)(std::ptr::null_mut());
            (api.general_context_free)(std::ptr::null_mut());
        }
    }
    assert_eq!(outs[0], outs[1], "custom-allocator / context-copy path diverges");
}

// -------------------------------------------------- ERRORS.md rows 137-143, 176
#[test]
fn setter_validation() {
    let mut outs = Vec::new();
    for api in [c(), r()] {
        unsafe {
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            let vc = (api.convert_context_create)(std::ptr::null_mut());
            let mut v: Vec<(&str, u32, c_int)> = Vec::new();
            for val in [0u32, 1, 2, 3, 4, 100, u32::MAX, u32::MAX - 1] {
                v.push(("bsr", val, (api.set_bsr)(cc, val)));
            }
            for val in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 100, u32::MAX] {
                v.push(("newline", val, (api.set_newline)(cc, val)));
            }
            for val in [
                0u32, 1, 2, 3, 63, 64, 65, 66, 67, 68, 69, 70, 71, 128, u32::MAX,
            ] {
                v.push(("optimize", val, (api.set_optimize)(cc, val)));
            }
            v.push(("optimize-null", 0, (api.set_optimize)(std::ptr::null_mut(), 0)));
            for val in [
                0u32, b'/' as u32, b'\\' as u32, b'.' as u32, b'x' as u32, 255, 256, u32::MAX,
            ] {
                v.push(("glob_sep", val, (api.set_glob_separator)(vc, val)));
            }
            for val in [
                0u32, b'\\' as u32, b'!' as u32, b'~' as u32, b'a' as u32, b'0' as u32, 255,
                256, u32::MAX,
            ] {
                v.push(("glob_esc", val, (api.set_glob_escape)(vc, val)));
            }
            // setters that never validate
            v.push(("varlookbehind", 0, (api.set_max_varlookbehind)(cc, 0)));
            v.push(("parens", 0, (api.set_parens_nest_limit)(cc, 0)));
            v.push(("extra", 0xffff_ffff, (api.set_compile_extra_options)(cc, 0xffff_ffff)));
            v.push(("matchlimit", 0, (api.set_match_limit)(
                api.match_context_create(std::ptr::null_mut()), 0)));
            (api.compile_context_free)(cc);
            (api.convert_context_free)(vc);
            outs.push(v);
        }
    }
    assert_eq!(outs[0], outs[1], "setter validation differs");
    for (name, val, rc) in &outs[0] {
        match *name {
            "bsr" => assert_eq!(
                *rc,
                if *val == 1 || *val == 2 { 0 } else { PCRE2_ERROR_BADDATA },
                "set_bsr({val})"
            ),
            "newline" => assert_eq!(
                *rc,
                if (1..=6).contains(val) { 0 } else { PCRE2_ERROR_BADDATA },
                "set_newline({val})"
            ),
            "optimize-null" => assert_eq!(*rc, PCRE2_ERROR_NULL),
            _ => {}
        }
    }
}

impl Api {
    unsafe fn match_context_create(&self, gc: Ctx) -> Ctx {
        unsafe { (self.match_context_create)(gc) }
    }
}

// ------------------------------------------- row 108 / ERRORS.md rows 144-153
#[test]
fn jit_stubs() {
    let mut outs = Vec::new();
    for api in [c(), r()] {
        unsafe {
            let mut v: Vec<i64> = Vec::new();
            let mut err = 0;
            let mut off = 0;
            let code = (api.compile)(b"abc".as_ptr(), 3, 0, &mut err, &mut off,
                                     std::ptr::null_mut());
            // row 144/145: TEST_ALLOC alone and combined
            v.push((api.jit_compile)(code, PCRE2_JIT_TEST_ALLOC) as i64);
            v.push((api.jit_compile)(code, PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_COMPLETE) as i64);
            v.push((api.jit_compile)(std::ptr::null_mut(), PCRE2_JIT_TEST_ALLOC) as i64);
            // row 146: NULL code
            v.push((api.jit_compile)(std::ptr::null_mut(), PCRE2_JIT_COMPLETE) as i64);
            // row 147: bad option bits
            for o in [0x0000_0008u32, 0x0000_0010, 0x0000_0200 | 0x1, 0xffff_ffff, 0x8000_0000] {
                v.push((api.jit_compile)(code, o) as i64);
            }
            // row 148: valid options in a non-JIT build
            for o in [
                0u32,
                PCRE2_JIT_COMPLETE,
                PCRE2_JIT_PARTIAL_SOFT,
                PCRE2_JIT_PARTIAL_HARD,
                PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD,
            ] {
                v.push((api.jit_compile)(code, o) as i64);
            }
            // row 149: JIT_INVALID_UTF sets MATCH_INVALID_UTF on the code
            let mut before: u32 = 0;
            (api.pattern_info)(code, 0, &mut before as *mut u32 as *mut c_void);
            v.push((api.jit_compile)(code, PCRE2_JIT_INVALID_UTF) as i64);
            let mut after: u32 = 0;
            (api.pattern_info)(code, 0, &mut after as *mut u32 as *mut c_void);
            v.push(before as i64);
            v.push(after as i64);
            // row 150: jit_match
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            v.push((api.jit_match)(code, b"abc".as_ptr(), 3, 0, 0, md, std::ptr::null_mut())
                as i64);
            // rows 151-153
            let st = (api.jit_stack_create)(1024, 1024 * 1024, std::ptr::null_mut());
            v.push(st.is_null() as i64);
            (api.jit_stack_assign)(std::ptr::null_mut(), std::ptr::null_mut(),
                                   std::ptr::null_mut());
            (api.jit_stack_free)(st);
            (api.jit_stack_free)(std::ptr::null_mut());
            (api.jit_free_unused_memory)(std::ptr::null_mut());
            v.push((api.priv_jit_get_size)(std::ptr::null_mut()) as i64);
            (api.priv_jit_free)(std::ptr::null_mut(), std::ptr::null_mut());
            (api.priv_jit_free_rodata)(std::ptr::null_mut(), std::ptr::null_mut());
            let tgt = (api.priv_jit_get_target)();
            let tgt = cstr(tgt as *const u8);
            v.push(tgt.len() as i64);
            outs.push((v, tgt));
            (api.match_data_free)(md);
            (api.code_free)(code);
        }
    }
    assert_eq!(outs[0], outs[1], "JIT stub behaviour differs");
    eprintln!("jit codes = {:?}", outs[0]);
    assert_eq!(outs[0].0[0], PCRE2_ERROR_JIT_UNSUPPORTED as i64, "TEST_ALLOC");
    assert_eq!(outs[0].0[1], PCRE2_ERROR_JIT_BADOPTION as i64, "TEST_ALLOC|COMPLETE");
    assert_eq!(outs[0].0[3], PCRE2_ERROR_NULL as i64, "NULL code");
    assert_eq!(
        String::from_utf8_lossy(&outs[0].1),
        "JIT is not supported",
        "jit_get_target string"
    );
    let _: *const c_char = std::ptr::null();
}
