//! Phase B/C: substring extraction and the name table.
//! CONFIGS.md rows 78-80; ERRORS.md rows 79-97.
mod harness;
use harness::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const NAMED_PATS: &[&str] = &[
    "(a)",
    "(a)(b)",
    "(a)(b)(c)",
    "(?<one>a)",
    "(?<one>a)(?<two>b)",
    "(?<one>a)(b)(?<three>c)",
    "(?<z>a)(?<y>b)(?<x>c)",
    "(a)(?:b)(c)",
    "(a)|(b)",
    "(?<n>a)|(?<n>b)",
    "(?<n>a)|(?<n>b)|(?<n>c)",
    "(?<longname_aaaaaaaaaa>a)",
    "(?<n1>a)(?<n2>b)(?<n3>c)(?<n4>d)",
    "(a)(b)?(c)?",
    "(?<opt>x)?y",
    "(?<a>1)(?<ab>2)(?<abc>3)",
];
const SUBJ: &[&str] = &["", "a", "ab", "abc", "abcd", "b", "c", "y", "xy", "123", "d"];
const NAMES: &[&str] = &[
    "one", "two", "three", "n", "z", "y", "x", "nope", "", "longname_aaaaaaaaaa", "n1", "n4",
    "opt", "a", "ab", "abc", "A",
];

#[derive(Debug, PartialEq, Eq)]
struct SubsOut {
    rc: c_int,
    by_number: Vec<(u32, c_int, Sz, c_int, Option<Vec<u8>>, c_int, Vec<u8>, Sz)>,
    by_name: Vec<(&'static str, c_int, Sz, c_int, Option<Vec<u8>>, c_int, Vec<u8>, Sz)>,
    numbers: Vec<(&'static str, c_int)>,
    scan: Vec<(&'static str, c_int, Vec<u8>, Vec<u8>)>,
    list: (c_int, Vec<Vec<u8>>),
}

fn probe(api: &Api, pat: &str, subj: &str, copts: u32, ovec: Option<u32>, dfa: bool) -> SubsOut {
    unsafe {
        let mut err = 0;
        let mut off = 0;
        let p = pat.as_bytes();
        let code = (api.compile)(p.as_ptr(), p.len(), copts, &mut err, &mut off,
                                 std::ptr::null_mut());
        assert!(!code.is_null(), "{pat:?} failed to compile: {err}");
        let md = match ovec {
            Some(n) => (api.match_data_create)(n, std::ptr::null_mut()),
            None => (api.match_data_create_from_pattern)(code, std::ptr::null_mut()),
        };
        let s = subj.as_bytes();
        let rc = if dfa {
            let mut ws = [0i32; 128];
            (api.dfa_match)(code, s.as_ptr(), s.len(), 0, 0, md, std::ptr::null_mut(),
                            ws.as_mut_ptr(), ws.len())
        } else {
            (api.do_match)(code, s.as_ptr(), s.len(), 0, 0, md, std::ptr::null_mut())
        };
        let mut by_number = Vec::new();
        for g in 0..8u32 {
            let mut len: Sz = 0xdead;
            let lrc = (api.substring_length_bynumber)(md, g, &mut len);
            let mut gp: *mut u8 = std::ptr::null_mut();
            let mut glen: Sz = 0;
            let grc = (api.substring_get_bynumber)(md, g, &mut gp, &mut glen);
            let gval = if grc == 0 && !gp.is_null() {
                let v = std::slice::from_raw_parts(gp, glen).to_vec();
                (api.substring_free)(gp);
                Some(v)
            } else {
                None
            };
            // copy into a deliberately small then adequate buffer
            let mut cbuf = [0xaau8; 8];
            let mut csize: Sz = cbuf.len();
            let crc = (api.substring_copy_bynumber)(md, g, cbuf.as_mut_ptr(), &mut csize);
            let mut tiny = [0xaau8; 1];
            let mut tsize: Sz = 1;
            let trc = (api.substring_copy_bynumber)(md, g, tiny.as_mut_ptr(), &mut tsize);
            by_number.push((
                g,
                lrc,
                if lrc == 0 { len } else { 0 },
                grc,
                gval,
                crc,
                if crc == 0 { cbuf.to_vec() } else { Vec::new() },
                if trc == 0 { tsize } else { trc as Sz },
            ));
        }
        let mut by_name = Vec::new();
        for &nm in NAMES {
            let n = cs(nm);
            let mut len: Sz = 0xdead;
            let lrc = (api.substring_length_byname)(md, n.as_ptr(), &mut len);
            let mut gp: *mut u8 = std::ptr::null_mut();
            let mut glen: Sz = 0;
            let grc = (api.substring_get_byname)(md, n.as_ptr(), &mut gp, &mut glen);
            let gval = if grc == 0 && !gp.is_null() {
                let v = std::slice::from_raw_parts(gp, glen).to_vec();
                (api.substring_free)(gp);
                Some(v)
            } else {
                None
            };
            let mut cbuf = [0xaau8; 8];
            let mut csize: Sz = cbuf.len();
            let crc = (api.substring_copy_byname)(md, n.as_ptr(), cbuf.as_mut_ptr(), &mut csize);
            let mut tiny = [0xaau8; 1];
            let mut tsize: Sz = 1;
            let trc = (api.substring_copy_byname)(md, n.as_ptr(), tiny.as_mut_ptr(), &mut tsize);
            by_name.push((
                nm,
                lrc,
                if lrc == 0 { len } else { 0 },
                grc,
                gval,
                crc,
                if crc == 0 { cbuf.to_vec() } else { Vec::new() },
                if trc == 0 { tsize } else { trc as Sz },
            ));
        }
        let mut numbers = Vec::new();
        let mut scan = Vec::new();
        for &nm in NAMES {
            let n = cs(nm);
            numbers.push((nm, (api.substring_number_from_name)(code, n.as_ptr())));
            let mut first: *const u8 = std::ptr::null();
            let mut last: *const u8 = std::ptr::null();
            let src = (api.substring_nametable_scan)(code, n.as_ptr(), &mut first, &mut last);
            // Entry size lets us read the found entries verbatim.
            let mut nsize: u32 = 0;
            (api.pattern_info)(code, 18, &mut nsize as *mut u32 as *mut c_void);
            let (fb, lb) = if src >= 0 && !first.is_null() && !last.is_null() {
                (
                    std::slice::from_raw_parts(first, nsize as usize).to_vec(),
                    std::slice::from_raw_parts(last, nsize as usize).to_vec(),
                )
            } else {
                (Vec::new(), Vec::new())
            };
            scan.push((nm, src, fb, lb));
        }
        let list = {
            let mut lp: *mut *mut u8 = std::ptr::null_mut();
            let mut lens: *mut Sz = std::ptr::null_mut();
            let lrc = (api.substring_list_get)(md, &mut lp, &mut lens);
            let mut v = Vec::new();
            if lrc == 0 && !lp.is_null() {
                let mut count = if rc > 0 { rc as usize } else { 0 };
                if rc == 0 {
                    count = (api.get_ovector_count)(md) as usize;
                }
                for i in 0..count {
                    let e = *lp.add(i);
                    let l = *lens.add(i);
                    v.push(std::slice::from_raw_parts(e, l).to_vec());
                }
                (api.substring_list_free)(lp);
            }
            (lrc, v)
        };
        (api.match_data_free)(md);
        (api.code_free)(code);
        SubsOut { rc, by_number, by_name, numbers, scan, list }
    }
}

#[test]
fn substrings_by_number_and_name() {
    for p in NAMED_PATS {
        for s in SUBJ {
            for copts in [0, PCRE2_DUPNAMES, PCRE2_CASELESS, PCRE2_DUPNAMES | PCRE2_CASELESS] {
                for ovec in [None, Some(0u32), Some(1), Some(2), Some(3), Some(16)] {
                    for dfa in [false, true] {
                        // DUPNAMES is required for the duplicate-name patterns.
                        if p.contains("(?<n>a)|(?<n>b)") && copts & PCRE2_DUPNAMES == 0 {
                            continue;
                        }
                        let co = probe(c(), p, s, copts, ovec, dfa);
                        let ro = probe(r(), p, s, copts, ovec, dfa);
                        if co != ro {
                            let mut d = Vec::new();
                            if co.rc != ro.rc {
                                d.push(format!("rc {} vs {}", co.rc, ro.rc));
                            }
                            for (a, b) in co.by_number.iter().zip(&ro.by_number) {
                                if a != b {
                                    d.push(format!("bynumber {a:?} vs {b:?}"));
                                }
                            }
                            for (a, b) in co.by_name.iter().zip(&ro.by_name) {
                                if a != b {
                                    d.push(format!("byname {a:?} vs {b:?}"));
                                }
                            }
                            for (a, b) in co.numbers.iter().zip(&ro.numbers) {
                                if a != b {
                                    d.push(format!("number_from_name {a:?} vs {b:?}"));
                                }
                            }
                            for (a, b) in co.scan.iter().zip(&ro.scan) {
                                if a != b {
                                    d.push(format!("scan {a:?} vs {b:?}"));
                                }
                            }
                            if co.list != ro.list {
                                d.push(format!("list {:?} vs {:?}", co.list, ro.list));
                            }
                            panic!(
                                "SUBSTRING DIVERGENCE p={p:?} s={s:?} copts={copts:#x} ovec={ovec:?} dfa={dfa}\n   {}",
                                d.join("\n   ")
                            );
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------- ERRORS.md rows 82, 90
#[test]
fn substring_after_partial_and_dfa() {
    unsafe {
        let mut out = Vec::new();
        for api in [c(), r()] {
            let mut err = 0;
            let mut off = 0;
            let pat = b"(a)(b)(c)d";
            let code = (api.compile)(pat.as_ptr(), pat.len(), 0, &mut err, &mut off,
                                     std::ptr::null_mut());
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let s = b"abc";
            // partial match
            let prc = (api.do_match)(code, s.as_ptr(), 3, 0, PCRE2_PARTIAL_HARD, md,
                                     std::ptr::null_mut());
            let mut res = vec![prc];
            for g in 0..5u32 {
                let mut len: Sz = 0;
                res.push((api.substring_length_bynumber)(md, g, &mut len));
            }
            let n = cs("nope");
            let mut len: Sz = 0;
            res.push((api.substring_length_byname)(md, n.as_ptr(), &mut len));
            // DFA match then name lookups (DFA_UFUNC)
            let md2 = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let mut ws = [0i32; 128];
            let drc = (api.dfa_match)(code, b"abcd".as_ptr(), 4, 0, 0, md2,
                                      std::ptr::null_mut(), ws.as_mut_ptr(), ws.len());
            res.push(drc);
            let mut len: Sz = 0;
            res.push((api.substring_length_byname)(md2, n.as_ptr(), &mut len));
            let mut gp: *mut u8 = std::ptr::null_mut();
            let mut glen: Sz = 0;
            res.push((api.substring_get_byname)(md2, n.as_ptr(), &mut gp, &mut glen));
            let mut cbuf = [0u8; 8];
            let mut csize: Sz = 8;
            res.push((api.substring_copy_byname)(md2, n.as_ptr(), cbuf.as_mut_ptr(), &mut csize));
            for g in 0..5u32 {
                let mut len: Sz = 0;
                res.push((api.substring_length_bynumber)(md2, g, &mut len));
            }
            out.push(res);
            (api.match_data_free)(md);
            (api.match_data_free)(md2);
            (api.code_free)(code);
        }
        assert_eq!(out[0], out[1], "partial/DFA substring codes differ");
        eprintln!("codes = {:?}", out[0]);
        assert_eq!(out[0][0], PCRE2_ERROR_PARTIAL);
        assert_eq!(out[0][2], PCRE2_ERROR_PARTIAL, "group>0 after partial must be PARTIAL");
        assert_eq!(out[0][8], PCRE2_ERROR_DFA_UFUNC, "byname after DFA must be DFA_UFUNC");
    }
}

// ------------------------------------------------------ ERRORS.md rows 93, 94
#[test]
fn number_from_name_uniqueness() {
    for (pat, copts) in [
        ("(?<n>a)(?<m>b)", 0u32),
        ("(?<n>a)|(?<n>b)", PCRE2_DUPNAMES),
        ("(?<n>a)|(?<n>b)|(?<n>c)", PCRE2_DUPNAMES),
        ("(a)(b)", 0),
    ] {
        unsafe {
            let mut vals = Vec::new();
            for api in [c(), r()] {
                let mut err = 0;
                let mut off = 0;
                let p = pat.as_bytes();
                let code = (api.compile)(p.as_ptr(), p.len(), copts, &mut err, &mut off,
                                         std::ptr::null_mut());
                assert!(!code.is_null(), "{pat} err {err}");
                let mut v = Vec::new();
                for nm in ["n", "m", "zzz", ""] {
                    let n = cs(nm);
                    v.push((api.substring_number_from_name)(code, n.as_ptr()));
                }
                vals.push(v);
                (api.code_free)(code);
            }
            assert_eq!(vals[0], vals[1], "{pat}: number_from_name differs");
            if copts & PCRE2_DUPNAMES != 0 {
                assert_eq!(vals[0][0], PCRE2_ERROR_NOUNIQUESUBSTRING);
            }
            assert_eq!(vals[0][2], PCRE2_ERROR_NOSUBSTRING);
        }
    }
}

// ------------------------------------------------------------ randomized rows
#[test]
fn substrings_randomized() {
    let mut rng = Rng::new(0x5EED_0004);
    for i in 0..4000u32 {
        let p = if rng.bool() {
            (*rng.pick(NAMED_PATS)).to_string()
        } else {
            let d = rng.range(1, 2) as u32;
            format!("(?<g0>{})", random_pattern(&mut rng, d))
        };
        let s = if rng.bool() {
            (*rng.pick(SUBJ)).to_string()
        } else {
            String::from_utf8_lossy(&random_subject(&mut rng, false)).into_owned()
        };
        let copts = *rng.pick(&[0u32, PCRE2_DUPNAMES, PCRE2_CASELESS, PCRE2_UTF]);
        // Skip patterns the C library rejects: probe() asserts on compile success.
        let mut err = 0;
        let mut off = 0;
        let pb = p.as_bytes();
        let ok = unsafe {
            let code = (c().compile)(pb.as_ptr(), pb.len(), copts, &mut err, &mut off,
                                     std::ptr::null_mut());
            let ok = !code.is_null();
            if ok {
                (c().code_free)(code);
            }
            ok
        };
        if !ok {
            continue;
        }
        let ovec = if rng.bool() { Some(rng.below(5) as u32) } else { None };
        let dfa = rng.below(4) == 0;
        let co = probe(c(), &p, &s, copts, ovec, dfa);
        let ro = probe(r(), &p, &s, copts, ovec, dfa);
        assert!(
            co == ro,
            "SUBSTRING DIVERGENCE iter={i} p={p:?} s={s:?} copts={copts:#x} ovec={ovec:?} dfa={dfa}\n C   ={co:?}\n Rust={ro:?}"
        );
    }
}
