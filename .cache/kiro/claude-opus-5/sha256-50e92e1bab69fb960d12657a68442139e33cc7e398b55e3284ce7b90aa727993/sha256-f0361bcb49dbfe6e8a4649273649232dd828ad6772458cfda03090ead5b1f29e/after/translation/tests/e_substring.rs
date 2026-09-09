//! CONFIGS.md rows 115–122, 147–148 — substring extraction, serialization and
//! `pcre2_get_error_message`.
mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_int;

fn zpat(p: &str) -> Vec<u8> {
    let mut v = p.as_bytes().to_vec();
    v.push(0);
    v
}

fn pad(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.extend_from_slice(&[0u8; 16]);
    v
}

const NAMED_PATTERNS: &[&str] = &[
    "(?<a>x)",
    "(?<a>x)(?<b>y)",
    "(?<a>x)(y)(?<b>z)",
    "(?J)(?<a>x)|(?<a>y)",
    "(?J)(?<a>x)(?<a>y)",
    "(?<longname>abc)",
    "(?<a>a)(?<ab>b)(?<abc>c)",
    "(?<a>a)?(?<b>b)?",
    "(?<n>a)(?<m>(?<o>b))",
    "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)",
    "(a)|(b)",
    "(?<a>a)|(b)",
    "(a(b(c)))",
    "(?<year>\\d{4})-(?<mon>\\d{2})",
    "(?:x)(y)",
    "(?<\u{00e9}>a)",
];

const NAMES: &[&str] = &[
    "a", "b", "c", "ab", "abc", "n", "m", "o", "longname", "year", "mon", "zzz", "",
    "\u{00e9}", "A",
];

// ------------------------------------- rows 115-118: bynumber / byname --------

#[test]
fn rows115_118_substring_by_number_and_name() {
    let (c, r) = pair();
    let mut subs: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    subs.extend(RAW_SUBJECTS.iter().map(|s| s.to_vec()));
    for p in NAMED_PATTERNS.iter().chain(PATTERNS.iter()) {
        for copts in [0u32, PCRE2_UTF, PCRE2_DUPNAMES] {
            let v = zpat(p);
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let cc = unsafe {
                (c.compile)(v.as_ptr(), p.len(), copts, &mut ce, &mut co, std::ptr::null_mut())
            };
            let rr = unsafe {
                (r.compile)(v.as_ptr(), p.len(), copts, &mut re, &mut ro, std::ptr::null_mut())
            };
            assert_eq!((ce, co, cc.is_null()), (re, ro, rr.is_null()), "{p:?}");
            if cc.is_null() {
                continue;
            }
            // number_from_name / nametable_scan don't need a match
            for n in NAMES {
                let nz = zpat(n);
                assert_eq!(
                    unsafe { (c.substring_number_from_name)(cc, nz.as_ptr()) },
                    unsafe { (r.substring_number_from_name)(rr, nz.as_ptr()) },
                    "number_from_name({n:?}) {p:?}"
                );
                let mut cf: *const u8 = std::ptr::null();
                let mut cl: *const u8 = std::ptr::null();
                let mut rf: *const u8 = std::ptr::null();
                let mut rl: *const u8 = std::ptr::null();
                let crc =
                    unsafe { (c.substring_nametable_scan)(cc, nz.as_ptr(), &mut cf, &mut cl) };
                let rrc =
                    unsafe { (r.substring_nametable_scan)(rr, nz.as_ptr(), &mut rf, &mut rl) };
                assert_eq!(crc, rrc, "nametable_scan({n:?}) rc {p:?}");
                if crc >= 0 {
                    // compare the entry ranges relative to the name table base
                    let mut cnt: u32 = 0;
                    let mut esz: u32 = 0;
                    let mut cbase: *const u8 = std::ptr::null();
                    let mut rbase: *const u8 = std::ptr::null();
                    unsafe {
                        (c.pattern_info)(cc, 17, &mut cnt as *mut u32 as *mut std::ffi::c_void);
                        (c.pattern_info)(cc, 18, &mut esz as *mut u32 as *mut std::ffi::c_void);
                        (c.pattern_info)(
                            cc,
                            19,
                            &mut cbase as *mut *const u8 as *mut std::ffi::c_void,
                        );
                        (r.pattern_info)(
                            rr,
                            19,
                            &mut rbase as *mut *const u8 as *mut std::ffi::c_void,
                        );
                    }
                    assert_eq!(
                        (cf as usize).wrapping_sub(cbase as usize),
                        (rf as usize).wrapping_sub(rbase as usize),
                        "nametable_scan first {n:?} {p:?}"
                    );
                    assert_eq!(
                        (cl as usize).wrapping_sub(cbase as usize),
                        (rl as usize).wrapping_sub(rbase as usize),
                        "nametable_scan last {n:?} {p:?}"
                    );
                }
                // NULL first/last pointers
                assert_eq!(
                    unsafe {
                        (c.substring_nametable_scan)(
                            cc,
                            nz.as_ptr(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    },
                    unsafe {
                        (r.substring_nametable_scan)(
                            rr,
                            nz.as_ptr(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    },
                    "nametable_scan NULL out {n:?} {p:?}"
                );
            }
            for s in &subs {
                if !unsafe { subject_domain_is_defined(c, cc, s, 0, 0) } {
                    continue;
                }
                let sp = pad(s);
                let cmd = unsafe { (c.match_data_create)(16, std::ptr::null_mut()) };
                let rmd = unsafe { (r.match_data_create)(16, std::ptr::null_mut()) };
                let crc = unsafe {
                    (c.pcre2_match)(cc, sp.as_ptr(), s.len(), 0, 0, cmd, std::ptr::null_mut())
                };
                let rrc = unsafe {
                    (r.pcre2_match)(rr, sp.as_ptr(), s.len(), 0, 0, rmd, std::ptr::null_mut())
                };
                assert_eq!(
                    unsafe { md_dump(c, cmd, crc) },
                    unsafe { md_dump(r, rmd, rrc) },
                    "match {p:?} {s:02x?}"
                );
                for num in [0u32, 1, 2, 3, 5, 11, 12, 100, u32::MAX] {
                    // length_bynumber
                    let mut clen: Sz = 0xDEAD;
                    let mut rlen: Sz = 0xDEAD;
                    let ci = unsafe { (c.substring_length_bynumber)(cmd, num, &mut clen) };
                    let ri = unsafe { (r.substring_length_bynumber)(rmd, num, &mut rlen) };
                    assert_eq!(ci, ri, "length_bynumber({num}) rc {p:?} {s:02x?}");
                    if ci == 0 {
                        assert_eq!(clen, rlen, "length_bynumber({num}) len");
                    }
                    // length with NULL sizeptr
                    assert_eq!(
                        unsafe {
                            (c.substring_length_bynumber)(cmd, num, std::ptr::null_mut())
                        },
                        unsafe {
                            (r.substring_length_bynumber)(rmd, num, std::ptr::null_mut())
                        },
                        "length_bynumber({num}) NULL"
                    );
                    // copy_bynumber over a range of buffer sizes
                    let need = if ci == 0 { clen } else { 0 };
                    for bs in [0usize, 1, need, need + 1, need + 2, 64] {
                        let mut cbuf = vec![0xAAu8; bs + 8];
                        let mut rbuf = vec![0xAAu8; bs + 8];
                        let mut csz: Sz = bs;
                        let mut rsz: Sz = bs;
                        let cj = unsafe {
                            (c.substring_copy_bynumber)(cmd, num, cbuf.as_mut_ptr(), &mut csz)
                        };
                        let rj = unsafe {
                            (r.substring_copy_bynumber)(rmd, num, rbuf.as_mut_ptr(), &mut rsz)
                        };
                        assert_eq!(cj, rj, "copy_bynumber({num},{bs}) rc {p:?} {s:02x?}");
                        assert_eq!(csz, rsz, "copy_bynumber({num},{bs}) size");
                        if cj == 0 {
                            assert_eq!(cbuf, rbuf, "copy_bynumber({num},{bs}) bytes");
                        }
                    }
                    // get_bynumber (library-allocated)
                    let mut cb: *mut u8 = std::ptr::null_mut();
                    let mut rb: *mut u8 = std::ptr::null_mut();
                    let mut cs2: Sz = 0xDEAD;
                    let mut rs2: Sz = 0xDEAD;
                    let ck = unsafe { (c.substring_get_bynumber)(cmd, num, &mut cb, &mut cs2) };
                    let rk = unsafe { (r.substring_get_bynumber)(rmd, num, &mut rb, &mut rs2) };
                    assert_eq!(ck, rk, "get_bynumber({num}) rc {p:?} {s:02x?}");
                    if ck == 0 {
                        assert_eq!(cs2, rs2, "get_bynumber({num}) size");
                        let a = unsafe { std::slice::from_raw_parts(cb, cs2 + 1) };
                        let b = unsafe { std::slice::from_raw_parts(rb, rs2 + 1) };
                        assert_eq!(a, b, "get_bynumber({num}) bytes");
                        unsafe { (c.substring_free)(cb) };
                        unsafe { (r.substring_free)(rb) };
                    }
                }
                for n in NAMES {
                    let nz = zpat(n);
                    let mut clen: Sz = 0xDEAD;
                    let mut rlen: Sz = 0xDEAD;
                    let ci = unsafe { (c.substring_length_byname)(cmd, nz.as_ptr(), &mut clen) };
                    let ri = unsafe { (r.substring_length_byname)(rmd, nz.as_ptr(), &mut rlen) };
                    assert_eq!(ci, ri, "length_byname({n:?}) rc {p:?} {s:02x?}");
                    if ci == 0 {
                        assert_eq!(clen, rlen, "length_byname({n:?}) len");
                    }
                    let need = if ci == 0 { clen } else { 0 };
                    for bs in [0usize, need, need + 1, 64] {
                        let mut cbuf = vec![0xAAu8; bs + 8];
                        let mut rbuf = vec![0xAAu8; bs + 8];
                        let mut csz: Sz = bs;
                        let mut rsz: Sz = bs;
                        let cj = unsafe {
                            (c.substring_copy_byname)(cmd, nz.as_ptr(), cbuf.as_mut_ptr(), &mut csz)
                        };
                        let rj = unsafe {
                            (r.substring_copy_byname)(rmd, nz.as_ptr(), rbuf.as_mut_ptr(), &mut rsz)
                        };
                        assert_eq!(cj, rj, "copy_byname({n:?},{bs}) rc");
                        assert_eq!(csz, rsz, "copy_byname({n:?},{bs}) size");
                        if cj == 0 {
                            assert_eq!(cbuf, rbuf, "copy_byname({n:?},{bs}) bytes");
                        }
                    }
                    let mut cb: *mut u8 = std::ptr::null_mut();
                    let mut rb: *mut u8 = std::ptr::null_mut();
                    let mut cs2: Sz = 0xDEAD;
                    let mut rs2: Sz = 0xDEAD;
                    let ck =
                        unsafe { (c.substring_get_byname)(cmd, nz.as_ptr(), &mut cb, &mut cs2) };
                    let rk =
                        unsafe { (r.substring_get_byname)(rmd, nz.as_ptr(), &mut rb, &mut rs2) };
                    assert_eq!(ck, rk, "get_byname({n:?}) rc");
                    if ck == 0 {
                        assert_eq!(cs2, rs2, "get_byname({n:?}) size");
                        assert_eq!(
                            unsafe { std::slice::from_raw_parts(cb, cs2 + 1) },
                            unsafe { std::slice::from_raw_parts(rb, rs2 + 1) },
                            "get_byname({n:?}) bytes"
                        );
                        unsafe { (c.substring_free)(cb) };
                        unsafe { (r.substring_free)(rb) };
                    }
                }
                // row 121: substring_list_get
                let mut cl: *mut *mut u8 = std::ptr::null_mut();
                let mut rl: *mut *mut u8 = std::ptr::null_mut();
                let mut clens: *mut Sz = std::ptr::null_mut();
                let mut rlens: *mut Sz = std::ptr::null_mut();
                let ci = unsafe { (c.substring_list_get)(cmd, &mut cl, &mut clens) };
                let ri = unsafe { (r.substring_list_get)(rmd, &mut rl, &mut rlens) };
                assert_eq!(ci, ri, "substring_list_get rc {p:?} {s:02x?}");
                if ci == 0 {
                    let n = if crc > 0 { crc as usize } else { 0 };
                    for i in 0..n {
                        let cln = unsafe { *clens.add(i) };
                        let rln = unsafe { *rlens.add(i) };
                        assert_eq!(cln, rln, "list len[{i}] {p:?}");
                        assert_eq!(
                            unsafe { std::slice::from_raw_parts(*cl.add(i), cln + 1) },
                            unsafe { std::slice::from_raw_parts(*rl.add(i), rln + 1) },
                            "list str[{i}] {p:?}"
                        );
                    }
                    assert!(unsafe { *cl.add(n) }.is_null());
                    assert!(unsafe { *rl.add(n) }.is_null());
                    unsafe { (c.substring_list_free)(cl) };
                    unsafe { (r.substring_list_free)(rl) };
                }
                // list_get with NULL lengthsptr
                let mut cl2: *mut *mut u8 = std::ptr::null_mut();
                let mut rl2: *mut *mut u8 = std::ptr::null_mut();
                let ci = unsafe { (c.substring_list_get)(cmd, &mut cl2, std::ptr::null_mut()) };
                let ri = unsafe { (r.substring_list_get)(rmd, &mut rl2, std::ptr::null_mut()) };
                assert_eq!(ci, ri, "substring_list_get NULL lens rc");
                if ci == 0 {
                    unsafe { (c.substring_list_free)(cl2) };
                    unsafe { (r.substring_list_free)(rl2) };
                }
                unsafe { (c.match_data_free)(cmd) };
                unsafe { (r.match_data_free)(rmd) };
            }
            unsafe { (c.code_free)(cc) };
            unsafe { (r.code_free)(rr) };
        }
    }
    // substring_free / list_free on NULL must be no-ops
    unsafe { (c.substring_free)(std::ptr::null_mut()) };
    unsafe { (r.substring_free)(std::ptr::null_mut()) };
    unsafe { (c.substring_list_free)(std::ptr::null_mut()) };
    unsafe { (r.substring_list_free)(std::ptr::null_mut()) };
}

// ------------------------------------------------ row 122: serialization -----

#[test]
fn row122_serialize_roundtrip() {
    let (c, r) = pair();
    let groups: Vec<Vec<&str>> = vec![
        vec!["abc"],
        vec!["abc", "d(e)f"],
        vec!["a", "b", "c", "d", "e", "f", "g", "h"],
        vec!["(?<n>x)+", "\\p{L}", "(*UTF)\\X", "a{2,4}?"],
        vec![""],
    ];
    for g in &groups {
        let mut ccodes: Vec<*const Code> = Vec::new();
        let mut rcodes: Vec<*const Code> = Vec::new();
        let mut ok = true;
        for p in g {
            let v = zpat(p);
            let mut e: c_int = 0;
            let mut o: Sz = 0;
            let cc = unsafe {
                (c.compile)(v.as_ptr(), p.len(), 0, &mut e, &mut o, std::ptr::null_mut())
            };
            let rr = unsafe {
                (r.compile)(v.as_ptr(), p.len(), 0, &mut e, &mut o, std::ptr::null_mut())
            };
            if cc.is_null() || rr.is_null() {
                ok = false;
                break;
            }
            ccodes.push(cc);
            rcodes.push(rr);
        }
        if !ok {
            continue;
        }
        let n = ccodes.len() as i32;
        let mut cb: *mut u8 = std::ptr::null_mut();
        let mut rb: *mut u8 = std::ptr::null_mut();
        let mut cs: Sz = 0;
        let mut rs: Sz = 0;
        let crc = unsafe {
            (c.serialize_encode)(ccodes.as_ptr(), n, &mut cb, &mut cs, std::ptr::null_mut())
        };
        let rrc = unsafe {
            (r.serialize_encode)(rcodes.as_ptr(), n, &mut rb, &mut rs, std::ptr::null_mut())
        };
        assert_eq!((crc, cs), (rrc, rs), "serialize_encode {g:?}");
        assert!(crc > 0);
        let cbytes = unsafe { std::slice::from_raw_parts(cb, cs) }.to_vec();
        let rbytes = unsafe { std::slice::from_raw_parts(rb, rs) }.to_vec();
        assert_eq!(cbytes, rbytes, "serialized bytes {g:?}");
        assert_eq!(
            unsafe { (c.serialize_get_number_of_codes)(cb) },
            unsafe { (r.serialize_get_number_of_codes)(rb) },
            "get_number_of_codes {g:?}"
        );
        // decode: each library decodes both its own and the other's stream
        for count in [n, 1, n.min(2)] {
            let mut cout = vec![std::ptr::null_mut::<Code>(); count as usize];
            let mut rout = vec![std::ptr::null_mut::<Code>(); count as usize];
            let cd = unsafe {
                (c.serialize_decode)(cout.as_mut_ptr(), count, cb, std::ptr::null_mut())
            };
            let rd = unsafe {
                (r.serialize_decode)(rout.as_mut_ptr(), count, rb, std::ptr::null_mut())
            };
            assert_eq!(cd, rd, "serialize_decode({count}) {g:?}");
            if cd > 0 {
                for i in 0..cd as usize {
                    assert_eq!(
                        unsafe { serialize_bytes(c, cout[i]) },
                        unsafe { serialize_bytes(r, rout[i]) },
                        "decoded[{i}] re-encode {g:?}"
                    );
                    assert_eq!(
                        unsafe { info_dump(c, cout[i]) },
                        unsafe { info_dump(r, rout[i]) },
                        "decoded[{i}] info {g:?}"
                    );
                    // and it must still match identically
                    let sp = pad(b"abcdef");
                    let cmd = unsafe { (c.match_data_create)(8, std::ptr::null_mut()) };
                    let rmd = unsafe { (r.match_data_create)(8, std::ptr::null_mut()) };
                    let x = unsafe {
                        (c.pcre2_match)(cout[i], sp.as_ptr(), 6, 0, 0, cmd, std::ptr::null_mut())
                    };
                    let y = unsafe {
                        (r.pcre2_match)(rout[i], sp.as_ptr(), 6, 0, 0, rmd, std::ptr::null_mut())
                    };
                    assert_eq!(
                        unsafe { md_dump(c, cmd, x) },
                        unsafe { md_dump(r, rmd, y) },
                        "decoded[{i}] match {g:?}"
                    );
                    unsafe { (c.match_data_free)(cmd) };
                    unsafe { (r.match_data_free)(rmd) };
                    unsafe { (c.code_free)(cout[i]) };
                    unsafe { (r.code_free)(rout[i]) };
                }
            }
        }
        // cross-decode: Rust must be able to read the C's stream and vice versa
        for (dec, bytes, tag) in [(c, &rbytes, "C<-RUST"), (r, &cbytes, "RUST<-C")] {
            let mut out = vec![std::ptr::null_mut::<Code>(); n as usize];
            let rc = unsafe {
                (dec.serialize_decode)(out.as_mut_ptr(), n, bytes.as_ptr(), std::ptr::null_mut())
            };
            assert_eq!(rc, n, "cross decode {tag} {g:?}");
            for i in 0..n as usize {
                unsafe { (dec.code_free)(out[i]) };
            }
        }
        unsafe { (c.serialize_free)(cb) };
        unsafe { (r.serialize_free)(rb) };
        for i in 0..ccodes.len() {
            unsafe { (c.code_free)(ccodes[i] as *mut Code) };
            unsafe { (r.code_free)(rcodes[i] as *mut Code) };
        }
    }
    unsafe { (c.serialize_free)(std::ptr::null_mut()) };
    unsafe { (r.serialize_free)(std::ptr::null_mut()) };
}

// -------------------------------------------- row 147: get_error_message -----

#[test]
fn row147_get_error_message() {
    let (c, r) = pair();
    for code in -400i32..=400 {
        for bufsz in [0usize, 1, 2, 5, 10, 20, 40, 120, 256] {
            let mut cb = vec![0xAAu8; bufsz + 8];
            let mut rb = vec![0xAAu8; bufsz + 8];
            let crc = unsafe { (c.get_error_message)(code, cb.as_mut_ptr(), bufsz) };
            let rrc = unsafe { (r.get_error_message)(code, rb.as_mut_ptr(), bufsz) };
            assert_eq!(crc, rrc, "get_error_message({code},{bufsz}) rc");
            assert_eq!(cb, rb, "get_error_message({code},{bufsz}) buffer");
        }
    }
    for code in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -100000, 100000] {
        let mut cb = vec![0xAAu8; 256];
        let mut rb = vec![0xAAu8; 256];
        let crc = unsafe { (c.get_error_message)(code, cb.as_mut_ptr(), 256) };
        let rrc = unsafe { (r.get_error_message)(code, rb.as_mut_ptr(), 256) };
        assert_eq!((crc, cb), (rrc, rb), "get_error_message({code})");
    }
}

// ----------------------------------------- rows 19-22: context create/copy ----

#[test]
fn rows19_22_context_lifecycle() {
    let (c, r) = pair();
    for a in [c, r] {
        // general
        let g = unsafe { (a.general_context_create)(None, None, std::ptr::null_mut()) };
        assert!(!g.is_null());
        let g2 = unsafe { (a.general_context_copy)(g) };
        assert!(!g2.is_null());
        unsafe { (a.general_context_free)(g2) };
        // compile
        for base in [std::ptr::null_mut(), g] {
            let cc = unsafe { (a.compile_context_create)(base) };
            assert!(!cc.is_null());
            let cc2 = unsafe { (a.compile_context_copy)(cc) };
            assert!(!cc2.is_null());
            unsafe { (a.compile_context_free)(cc2) };
            unsafe { (a.compile_context_free)(cc) };
            let mc = unsafe { (a.match_context_create)(base) };
            assert!(!mc.is_null());
            let mc2 = unsafe { (a.match_context_copy)(mc) };
            assert!(!mc2.is_null());
            unsafe { (a.match_context_free)(mc2) };
            unsafe { (a.match_context_free)(mc) };
            let vc = unsafe { (a.convert_context_create)(base) };
            assert!(!vc.is_null());
            let vc2 = unsafe { (a.convert_context_copy)(vc) };
            assert!(!vc2.is_null());
            unsafe { (a.convert_context_free)(vc2) };
            unsafe { (a.convert_context_free)(vc) };
        }
        unsafe { (a.general_context_free)(g) };
        // NULL frees are no-ops
        unsafe { (a.general_context_free)(std::ptr::null_mut()) };
        unsafe { (a.compile_context_free)(std::ptr::null_mut()) };
        unsafe { (a.match_context_free)(std::ptr::null_mut()) };
        unsafe { (a.convert_context_free)(std::ptr::null_mut()) };
        unsafe { (a.match_data_free)(std::ptr::null_mut()) };
        unsafe { (a.code_free)(std::ptr::null_mut()) };
    }
    // copies of a configured compile context must behave identically
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    unsafe {
        (c.set_newline)(cc, 5);
        (r.set_newline)(rc, 5);
        (c.set_bsr)(cc, 2);
        (r.set_bsr)(rc, 2);
        (c.set_parens_nest_limit)(cc, 7);
        (r.set_parens_nest_limit)(rc, 7);
        (c.set_max_varlookbehind)(cc, 3);
        (r.set_max_varlookbehind)(rc, 3);
        (c.set_compile_extra_options)(cc, PCRE2_EXTRA_MATCH_WORD);
        (r.set_compile_extra_options)(rc, PCRE2_EXTRA_MATCH_WORD);
    }
    let cc2 = unsafe { (c.compile_context_copy)(cc) };
    let rc2 = unsafe { (r.compile_context_copy)(rc) };
    for p in PATTERNS {
        let v = zpat(p);
        for (ctx_c, ctx_r) in [(cc, rc), (cc2, rc2)] {
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let a = unsafe { (c.compile)(v.as_ptr(), p.len(), 0, &mut ce, &mut co, ctx_c) };
            let b = unsafe { (r.compile)(v.as_ptr(), p.len(), 0, &mut re, &mut ro, ctx_r) };
            assert_eq!((ce, co, a.is_null()), (re, ro, b.is_null()), "ctx copy {p:?}");
            if !a.is_null() {
                assert_eq!(
                    unsafe { serialize_bytes(c, a) },
                    unsafe { serialize_bytes(r, b) },
                    "ctx copy serialized {p:?}"
                );
                unsafe { (c.code_free)(a) };
                unsafe { (r.code_free)(b) };
            }
        }
    }
    unsafe { (c.compile_context_free)(cc2) };
    unsafe { (r.compile_context_free)(rc2) };
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
}
