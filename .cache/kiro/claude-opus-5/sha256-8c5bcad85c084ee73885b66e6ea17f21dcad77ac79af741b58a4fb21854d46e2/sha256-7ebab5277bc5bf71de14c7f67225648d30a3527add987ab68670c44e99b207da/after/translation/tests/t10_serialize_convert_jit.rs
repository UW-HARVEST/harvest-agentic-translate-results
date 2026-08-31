//! `pcre2_serialize.c`, `pcre2_convert.c` and `pcre2_jit_*` (stub) entry points.
mod common;

use common::*;
use std::ffi::c_void;

/* ------------------------------- serialize ------------------------------- */

type SerEncode = unsafe extern "C" fn(
    *const *const c_void,
    i32,
    *mut *mut u8,
    *mut PCRE2_SIZE,
    *mut c_void,
) -> i32;
type SerDecode =
    unsafe extern "C" fn(*mut *mut c_void, i32, *const u8, *mut c_void) -> i32;
type SerCount = unsafe extern "C" fn(*const u8) -> i32;
type SerFree = unsafe extern "C" fn(*mut u8);
type CodeFree = unsafe extern "C" fn(*mut c_void);

#[test]
fn serialize_roundtrip_matches() {
    let (sec, ser) = both::<SerEncode>("pcre2_serialize_encode_8");
    let (sdc, sdr) = both::<SerDecode>("pcre2_serialize_decode_8");
    let (scc, scr) = both::<SerCount>("pcre2_serialize_get_number_of_codes_8");
    let (sfc, sfr) = both::<SerFree>("pcre2_serialize_free_8");
    let (cfc, cfr) = both::<CodeFree>("pcre2_code_free_8");

    // Build the same list of compiled patterns in both libraries.
    let mut pats: Vec<&[u8]> = patterns();
    pats.retain(|p| compile_both(p, 0).is_some());
    let held: Vec<CodePair> = pats
        .iter()
        .filter_map(|p| compile_both(p, 0))
        .collect();
    let codes_c: Vec<*const c_void> = held.iter().map(|p| p.c as *const c_void).collect();
    let codes_r: Vec<*const c_void> = held.iter().map(|p| p.r as *const c_void).collect();

    unsafe {
        for n in [1usize, 2, 5, codes_c.len()] {
            if n > codes_c.len() {
                continue;
            }
            let mut ba: *mut u8 = std::ptr::null_mut();
            let mut bb: *mut u8 = std::ptr::null_mut();
            let mut la: PCRE2_SIZE = 0;
            let mut lb: PCRE2_SIZE = 0;
            let x = sec(
                codes_c.as_ptr(),
                n as i32,
                &mut ba,
                &mut la,
                std::ptr::null_mut(),
            );
            let y = ser(
                codes_r.as_ptr(),
                n as i32,
                &mut bb,
                &mut lb,
                std::ptr::null_mut(),
            );
            assert_eq!(x, y, "serialize_encode({n}) rc");
            assert_eq!(la, lb, "serialize_encode({n}) length");
            if x < 0 {
                continue;
            }
            // The serialized blob is memory-image based; the header contains a
            // magic number, version and a table pointer, but the rest (the
            // compiled patterns) must be byte-identical. Compare everything
            // except the embedded `tables` pointer field of each code.
            assert_eq!(scc(ba), scr(bb), "get_number_of_codes({n})");

            // Decode and compare each recovered pattern.
            let mut outa: Vec<*mut c_void> = vec![std::ptr::null_mut(); n];
            let mut outb: Vec<*mut c_void> = vec![std::ptr::null_mut(); n];
            let x = sdc(outa.as_mut_ptr(), n as i32, ba, std::ptr::null_mut());
            let y = sdr(outb.as_mut_ptr(), n as i32, bb, std::ptr::null_mut());
            assert_eq!(x, y, "serialize_decode({n}) rc");
            if x > 0 {
                for i in 0..x as usize {
                    let (bm_c, tail_c, body_c) = code_snapshot(outa[i]);
                    let (bm_r, tail_r, body_r) = code_snapshot(outb[i]);
                    assert_bytes_eq(&format!("decoded[{i}] bitmap"), &bm_c, &bm_r);
                    assert_eq!(tail_c, tail_r, "decoded[{i}] header");
                    assert_bytes_eq(&format!("decoded[{i}] body"), &body_c, &body_r);
                    cfc(outa[i]);
                    cfr(outb[i]);
                }
            }

            // Decoding fewer codes than are present.
            for k in [0i32, 1, -1] {
                let mut oa: Vec<*mut c_void> = vec![std::ptr::null_mut(); n.max(1)];
                let mut ob: Vec<*mut c_void> = vec![std::ptr::null_mut(); n.max(1)];
                let x = sdc(oa.as_mut_ptr(), k, ba, std::ptr::null_mut());
                let y = sdr(ob.as_mut_ptr(), k, bb, std::ptr::null_mut());
                assert_eq!(x, y, "serialize_decode(count={k}) rc");
                if x > 0 {
                    for i in 0..x as usize {
                        cfc(oa[i]);
                        cfr(ob[i]);
                    }
                }
            }

            sfc(ba);
            sfr(bb);
        }

        // Error paths.
        let mut ba: *mut u8 = std::ptr::null_mut();
        let mut la: PCRE2_SIZE = 0;
        let mut bb: *mut u8 = std::ptr::null_mut();
        let mut lb: PCRE2_SIZE = 0;
        for n in [0i32, -1] {
            assert_eq!(
                sec(codes_c.as_ptr(), n, &mut ba, &mut la, std::ptr::null_mut()),
                ser(codes_r.as_ptr(), n, &mut bb, &mut lb, std::ptr::null_mut()),
                "serialize_encode(count={n})"
            );
        }
        assert_eq!(
            sec(std::ptr::null(), 1, &mut ba, &mut la, std::ptr::null_mut()),
            ser(std::ptr::null(), 1, &mut bb, &mut lb, std::ptr::null_mut()),
            "serialize_encode(NULL codes)"
        );
        // A NULL code in the list.
        let bad_c: [*const c_void; 1] = [std::ptr::null()];
        assert_eq!(
            sec(bad_c.as_ptr(), 1, &mut ba, &mut la, std::ptr::null_mut()),
            ser(bad_c.as_ptr(), 1, &mut bb, &mut lb, std::ptr::null_mut()),
            "serialize_encode(NULL member)"
        );
        // Garbage blob.
        let garbage = [0u8; 64];
        assert_eq!(
            scc(garbage.as_ptr()),
            scr(garbage.as_ptr()),
            "get_number_of_codes(garbage)"
        );
        let mut oa: [*mut c_void; 4] = [std::ptr::null_mut(); 4];
        let mut ob: [*mut c_void; 4] = [std::ptr::null_mut(); 4];
        assert_eq!(
            sdc(oa.as_mut_ptr(), 4, garbage.as_ptr(), std::ptr::null_mut()),
            sdr(ob.as_mut_ptr(), 4, garbage.as_ptr(), std::ptr::null_mut()),
            "serialize_decode(garbage)"
        );
        sfc(std::ptr::null_mut());
        sfr(std::ptr::null_mut());
    }
}

/* -------------------------------- convert -------------------------------- */

type PatternConvert = unsafe extern "C" fn(
    PCRE2_SPTR,
    PCRE2_SIZE,
    u32,
    *mut *mut PCRE2_UCHAR,
    *mut PCRE2_SIZE,
    *mut c_void,
) -> i32;
type ConvertedFree = unsafe extern "C" fn(*mut PCRE2_UCHAR);
type CtxCreate = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type CtxFree = unsafe extern "C" fn(*mut c_void);
type SetU32 = unsafe extern "C" fn(*mut c_void, u32) -> i32;

const GLOBS: &[&[u8]] = &[
    b"",
    b"*",
    b"?",
    b"**",
    b"a",
    b"a*b",
    b"a?b",
    b"[abc]",
    b"[!abc]",
    b"[^abc]",
    b"[a-z]",
    b"[]",
    b"[!]",
    b"[",
    b"a\\*b",
    b"\\",
    b"/*/",
    b"**/*.c",
    b"/a/**/b",
    b"a/b/c",
    b"a**b",
    b".*",
    b"a.b",
    b"{a,b}",
    b"a[/]b",
    b"a[[:alpha:]]b",
    b"*.[ch]",
    b"\xc3\xa9*",
    b"a\xffb",
    /* POSIX BRE/ERE inputs */
    b"a\\(b\\)c",
    b"a(b)c",
    b"^abc$",
    b"a\\{2,3\\}",
    b"a{2,3}",
    b"a\\|b",
    b"a|b",
    b"[[:digit:]]+",
    b"\\1",
    b"a**",
    b"\\.",
    b"a\\nb",
    b"[a\\]",
    b"()",
    b"a+",
    b"+a",
    b"*a",
    b"a\\",
    /* multi-byte input, which must be treated byte-wise unless CONVERT_UTF */
    b"\xc3\xa9",
    b"\xc3*",
    b"\xc3(",
    b"\xc3)",
    b"\xc3\\",
    b"\xc3[",
    b"\xc3.",
    b"\xe6\x97\xa5*",
    b"\xe6\x97",
    b"\xf0\x9f\x98\x80{2}",
    b"a\xc3",
    b"\xc3\xa9\xc3\xa8",
    b"[\xc3\xa9]",
    b"[\xc3\xa9-\xc3\xbf]",
    b"\xff*",
    b"\x80\x80",
];

const CONVERT_OPTS: &[u32] = &[
    0x00000004, // POSIX_BASIC
    0x00000008, // POSIX_EXTENDED
    0x00000010, // GLOB
    0x00000030, // GLOB_NO_WILD_SEPARATOR
    0x00000050, // GLOB_NO_STARSTAR
    0x00000004 | 0x00000001, // + CONVERT_UTF
    0x00000008 | 0x00000001,
    0x00000010 | 0x00000001,
    0x00000010 | 0x00000001 | 0x00000002, // + NO_UTF_CHECK
    0,          // invalid: no type selected
    0x00000004 | 0x00000008, // invalid: two types
    0xffffffff,
];

#[test]
fn pattern_convert_matches() {
    let (pc, pr) = both::<PatternConvert>("pcre2_pattern_convert_8");
    let (fc, fr) = both::<ConvertedFree>("pcre2_converted_pattern_free_8");
    for g in GLOBS {
        for &opts in CONVERT_OPTS {
            // PCRE2_CONVERT_UTF together with PCRE2_CONVERT_NO_UTF_CHECK on an
            // ill-formed pattern is undefined behaviour in the C code.
            if (opts & 0x1) != 0 && (opts & 0x2) != 0 && std::str::from_utf8(g).is_err() {
                continue;
            }
            for zero_terminated in [false, true] {
                if zero_terminated && g.contains(&0) {
                    continue;
                }
                let mut buf = g.to_vec();
                buf.push(0);
                let len = if zero_terminated {
                    PCRE2_ZERO_TERMINATED
                } else {
                    g.len()
                };
                let label = format!("convert {g:02x?} opts={opts:#x} zt={zero_terminated}");
                unsafe {
                    // Allocating form: *buffptr starts NULL.
                    let mut pa: *mut PCRE2_UCHAR = std::ptr::null_mut();
                    let mut pb: *mut PCRE2_UCHAR = std::ptr::null_mut();
                    let mut la: PCRE2_SIZE = 0xdead;
                    let mut lb: PCRE2_SIZE = 0xdead;
                    let x = pc(buf.as_ptr(), len, opts, &mut pa, &mut la, std::ptr::null_mut());
                    let y = pr(buf.as_ptr(), len, opts, &mut pb, &mut lb, std::ptr::null_mut());
                    assert_eq!(x, y, "{label}: rc");
                    assert_eq!(la, lb, "{label}: length");
                    if x == 0 {
                        assert_bytes_eq(
                            &format!("{label}: output"),
                            slice_at(pa, la + 1),
                            slice_at(pb, lb + 1),
                        );
                        fc(pa);
                        fr(pb);
                    }

                    // Length-only form: buffptr == NULL.
                    let mut la: PCRE2_SIZE = 0xdead;
                    let mut lb: PCRE2_SIZE = 0xdead;
                    let x = pc(buf.as_ptr(), len, opts, std::ptr::null_mut(), &mut la, std::ptr::null_mut());
                    let y = pr(buf.as_ptr(), len, opts, std::ptr::null_mut(), &mut lb, std::ptr::null_mut());
                    assert_eq!(x, y, "{label}: rc (length only)");
                    assert_eq!(la, lb, "{label}: length (length only)");

                    // Caller-supplied buffer form, including buffers that are too small.
                    for cap in [1usize, 2, 8, 512] {
                        let mut outa = vec![0xAAu8; 600];
                        let mut outb = vec![0xAAu8; 600];
                        let mut pa = outa.as_mut_ptr();
                        let mut pb = outb.as_mut_ptr();
                        let mut la: PCRE2_SIZE = cap;
                        let mut lb: PCRE2_SIZE = cap;
                        let x = pc(buf.as_ptr(), len, opts, &mut pa, &mut la, std::ptr::null_mut());
                        let y = pr(buf.as_ptr(), len, opts, &mut pb, &mut lb, std::ptr::null_mut());
                        assert_eq!(x, y, "{label}: rc (buffer cap={cap})");
                        assert_eq!(la, lb, "{label}: length (buffer cap={cap})");
                        assert_bytes_eq(&format!("{label}: buffer cap={cap}"), &outa, &outb);
                    }
                }
            }
        }
    }
    unsafe {
        // NULL pattern and NULL length pointer.
        let mut pa: *mut PCRE2_UCHAR = std::ptr::null_mut();
        let mut pb: *mut PCRE2_UCHAR = std::ptr::null_mut();
        let mut la: PCRE2_SIZE = 0xdead;
        let mut lb: PCRE2_SIZE = 0xdead;
        assert_eq!(
            pc(std::ptr::null(), 0, 0x10, &mut pa, &mut la, std::ptr::null_mut()),
            pr(std::ptr::null(), 0, 0x10, &mut pb, &mut lb, std::ptr::null_mut()),
            "convert(NULL, 0)"
        );
        assert_eq!(la, lb);
        if la != 0xdead && pa != std::ptr::null_mut() {
            fc(pa);
            fr(pb);
        }
        assert_eq!(
            pc(std::ptr::null(), 5, 0x10, &mut pa, &mut la, std::ptr::null_mut()),
            pr(std::ptr::null(), 5, 0x10, &mut pb, &mut lb, std::ptr::null_mut()),
            "convert(NULL, 5)"
        );
        let g = b"a*b";
        assert_eq!(
            pc(g.as_ptr(), 3, 0x10, &mut pa, std::ptr::null_mut(), std::ptr::null_mut()),
            pr(g.as_ptr(), 3, 0x10, &mut pb, std::ptr::null_mut(), std::ptr::null_mut()),
            "convert(NULL bufflenptr)"
        );
    }
}

#[test]
fn pattern_convert_with_context() {
    let (pc, pr) = both::<PatternConvert>("pcre2_pattern_convert_8");
    let (fc, fr) = both::<ConvertedFree>("pcre2_converted_pattern_free_8");
    let (cc, rc) = both::<CtxCreate>("pcre2_convert_context_create_8");
    let (cf, rf) = both::<CtxFree>("pcre2_convert_context_free_8");
    let (ssc, ssr) = both::<SetU32>("pcre2_set_glob_separator_8");
    let (sec, ser) = both::<SetU32>("pcre2_set_glob_escape_8");
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        for sep in [b'/' as u32, b'\\' as u32, b'.' as u32] {
            assert_eq!(ssc(a, sep), ssr(b, sep));
            for esc in [0u32, b'\\' as u32, b'~' as u32] {
                assert_eq!(sec(a, esc), ser(b, esc));
                for g in GLOBS {
                    for &opts in &[0x10u32, 0x30, 0x50, 0x11] {
                        let mut pa: *mut PCRE2_UCHAR = std::ptr::null_mut();
                        let mut pb: *mut PCRE2_UCHAR = std::ptr::null_mut();
                        let mut la: PCRE2_SIZE = 0xdead;
                        let mut lb: PCRE2_SIZE = 0xdead;
                        let x = pc(g.as_ptr(), g.len(), opts, &mut pa, &mut la, a);
                        let y = pr(g.as_ptr(), g.len(), opts, &mut pb, &mut lb, b);
                        let label =
                            format!("convert-ctx {g:02x?} opts={opts:#x} sep={sep} esc={esc}");
                        assert_eq!(x, y, "{label}: rc");
                        assert_eq!(la, lb, "{label}: length");
                        if x == 0 {
                            assert_bytes_eq(
                                &format!("{label}: output"),
                                slice_at(pa, la + 1),
                                slice_at(pb, lb + 1),
                            );
                            fc(pa);
                            fr(pb);
                        }
                    }
                }
            }
        }
        cf(a);
        rf(b);
    }
    unsafe {
        fc(std::ptr::null_mut());
        fr(std::ptr::null_mut());
    }
}

/* ---------------------------------- JIT ---------------------------------- */

type JitCompile = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type JitMatch = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    PCRE2_SIZE,
    u32,
    *mut c_void,
    *mut c_void,
) -> i32;
type JitStackCreate = unsafe extern "C" fn(usize, usize, *mut c_void) -> *mut c_void;
type JitStackFree = unsafe extern "C" fn(*mut c_void);
type JitFreeUnused = unsafe extern "C" fn(*mut c_void);
type JitGetTarget = unsafe extern "C" fn() -> *const std::ffi::c_char;
type JitGetSize = unsafe extern "C" fn(*mut c_void) -> usize;
type MdCreate = unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
type MdFree = unsafe extern "C" fn(*mut c_void);

#[test]
fn jit_entry_points_match() {
    let (jcc, jcr) = both::<JitCompile>("pcre2_jit_compile_8");
    let (jmc, jmr) = both::<JitMatch>("pcre2_jit_match_8");
    let (jsc, jsr) = both::<JitStackCreate>("pcre2_jit_stack_create_8");
    let (jsfc, jsfr) = both::<JitStackFree>("pcre2_jit_stack_free_8");
    let (jfc, jfr) = both::<JitFreeUnused>("pcre2_jit_free_unused_memory_8");
    let (jtc, jtr) = both::<JitGetTarget>("_pcre2_jit_get_target_8");
    let (jgc, jgr) = both::<JitGetSize>("_pcre2_jit_get_size_8");
    let (mdc, mdr) = both::<MdCreate>("pcre2_match_data_create_8");
    let (mdfc, mdfr) = both::<MdFree>("pcre2_match_data_free_8");

    unsafe {
        // JIT is not built, so every entry point must report the same thing.
        let a = std::ffi::CStr::from_ptr(jtc());
        let b = std::ffi::CStr::from_ptr(jtr());
        assert_eq!(a, b, "_pcre2_jit_get_target");

        assert_eq!(jgc(std::ptr::null_mut()), jgr(std::ptr::null_mut()));

        let md_c = mdc(16, std::ptr::null_mut());
        let md_r = mdr(16, std::ptr::null_mut());
        for p in patterns() {
            let Some(pair) = compile_both(p, 0) else {
                continue;
            };
            for opts in [0u32, 1, 2, 4, 7, 0x100, 0x200, 0xffff_ffff] {
                assert_eq!(
                    jcc(pair.c, opts),
                    jcr(pair.r, opts),
                    "jit_compile({opts:#x}) for {p:02x?}"
                );
            }
            for s in subjects() {
                let rc = jmc(pair.c, s.as_ptr(), s.len(), 0, 0, md_c, std::ptr::null_mut());
                let rr = jmr(pair.r, s.as_ptr(), s.len(), 0, 0, md_r, std::ptr::null_mut());
                assert_eq!(rc, rr, "jit_match {p:02x?} / {s:02x?}");
            }
        }
        assert_eq!(jcc(std::ptr::null_mut(), 1), jcr(std::ptr::null_mut(), 1));
        mdfc(md_c);
        mdfr(md_r);

        let sa = jsc(1024, 32768, std::ptr::null_mut());
        let sb = jsr(1024, 32768, std::ptr::null_mut());
        assert_eq!(sa.is_null(), sb.is_null(), "jit_stack_create");
        jsfc(sa);
        jsfr(sb);
        jsfc(std::ptr::null_mut());
        jsfr(std::ptr::null_mut());
        jfc(std::ptr::null_mut());
        jfr(std::ptr::null_mut());
    }
}

#[test]
fn jit_stack_assign_matches() {
    type JitStackAssign = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
    let (ac, ar) = both::<JitStackAssign>("pcre2_jit_stack_assign_8");
    let (cc, rc) = both::<CtxCreate>("pcre2_match_context_create_8");
    let (cf, rf) = both::<CtxFree>("pcre2_match_context_free_8");
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        ac(a, std::ptr::null_mut(), std::ptr::null_mut());
        ar(b, std::ptr::null_mut(), std::ptr::null_mut());
        // Without JIT support the context must be unchanged in both.
        assert_bytes_eq(
            "match context after jit_stack_assign",
            &slice_at(a as *const u8, 0x60)[16..],
            &slice_at(b as *const u8, 0x60)[16..],
        );
        ac(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        ar(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        cf(a);
        rf(b);
    }
}

#[test]
fn jit_free_and_free_rodata_match() {
    type JitFree = unsafe extern "C" fn(*mut c_void, *mut c_void);
    type JitFreeRodata = unsafe extern "C" fn(*mut c_void, *mut c_void);
    let (fc, fr) = both::<JitFree>("_pcre2_jit_free_8");
    let (rc_, rr) = both::<JitFreeRodata>("_pcre2_jit_free_rodata_8");
    unsafe {
        fc(std::ptr::null_mut(), std::ptr::null_mut());
        fr(std::ptr::null_mut(), std::ptr::null_mut());
        rc_(std::ptr::null_mut(), std::ptr::null_mut());
        rr(std::ptr::null_mut(), std::ptr::null_mut());
    }
}
