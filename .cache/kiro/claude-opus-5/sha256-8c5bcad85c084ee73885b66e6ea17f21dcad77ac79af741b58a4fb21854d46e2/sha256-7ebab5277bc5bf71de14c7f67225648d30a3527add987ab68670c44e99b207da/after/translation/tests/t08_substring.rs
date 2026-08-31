//! `pcre2_substring.c`.
mod common;

use common::*;
use std::ffi::c_void;

type Match = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    PCRE2_SIZE,
    u32,
    *mut c_void,
    *mut c_void,
) -> i32;
type MdCreate = unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
type MdFree = unsafe extern "C" fn(*mut c_void);
type CopyByNumber =
    unsafe extern "C" fn(*mut c_void, u32, *mut PCRE2_UCHAR, *mut PCRE2_SIZE) -> i32;
type CopyByName =
    unsafe extern "C" fn(*mut c_void, PCRE2_SPTR, *mut PCRE2_UCHAR, *mut PCRE2_SIZE) -> i32;
type GetByNumber =
    unsafe extern "C" fn(*mut c_void, u32, *mut *mut PCRE2_UCHAR, *mut PCRE2_SIZE) -> i32;
type GetByName =
    unsafe extern "C" fn(*mut c_void, PCRE2_SPTR, *mut *mut PCRE2_UCHAR, *mut PCRE2_SIZE) -> i32;
type LenByNumber = unsafe extern "C" fn(*mut c_void, u32, *mut PCRE2_SIZE) -> i32;
type LenByName = unsafe extern "C" fn(*mut c_void, PCRE2_SPTR, *mut PCRE2_SIZE) -> i32;
type SubstringFree = unsafe extern "C" fn(*mut PCRE2_UCHAR);
type ListGet =
    unsafe extern "C" fn(*mut c_void, *mut *mut *mut PCRE2_UCHAR, *mut *mut PCRE2_SIZE) -> i32;
type ListFree = unsafe extern "C" fn(*mut *mut PCRE2_UCHAR);

/// Patterns with captures (named and numbered) plus the subjects they suit.
const CASES: &[(&[u8], &[&[u8]])] = &[
    (b"(a)(b)(c)", &[b"abc", b"xabc", b"ab", b""]),
    (b"(?<one>a)(?<two>b)", &[b"ab", b"a", b"", b"zzab"]),
    (b"(a)|(b)", &[b"a", b"b", b"c", b""]),
    (b"(a)(?:(b))?(c)", &[b"ac", b"abc", b"a"]),
    (b"(?<n>a)|(?<n>b)", &[b"a", b"b"]),
    (b"(\\d+)-(\\d+)", &[b"12-34", b"1-2", b"nope"]),
    (b"(.*)", &[b"", b"abc", b"a\nb"]),
    (b"()()()()()()()()()()", &[b"", b"x"]),
    (b"(?<long_name_here>x+)", &[b"xxx", b"y"]),
    (b"a(b)?c", &[b"ac", b"abc"]),
];

const NAMES: &[&[u8]] = &[
    b"one\0",
    b"two\0",
    b"n\0",
    b"long_name_here\0",
    b"missing\0",
    b"\0",
];

struct Pair {
    c: *mut c_void,
    r: *mut c_void,
    fc: libloading::Symbol<'static, MdFree>,
    fr: libloading::Symbol<'static, MdFree>,
}

impl Drop for Pair {
    fn drop(&mut self) {
        unsafe {
            (self.fc)(self.c);
            (self.fr)(self.r);
        }
    }
}

fn md_pair(n: u32) -> Pair {
    let (cc, rc) = both::<MdCreate>("pcre2_match_data_create_8");
    let (cf, rf) = both::<MdFree>("pcre2_match_data_free_8");
    unsafe {
        Pair {
            c: cc(n, std::ptr::null_mut()),
            r: rc(n, std::ptr::null_mut()),
            fc: cf,
            fr: rf,
        }
    }
}

#[test]
fn substring_length_and_copy_and_get() {
    let (mc, mr) = both::<Match>("pcre2_match_8");
    let (lnc, lnr) = both::<LenByNumber>("pcre2_substring_length_bynumber_8");
    let (lmc, lmr) = both::<LenByName>("pcre2_substring_length_byname_8");
    let (cnc, cnr) = both::<CopyByNumber>("pcre2_substring_copy_bynumber_8");
    let (cmc, cmr) = both::<CopyByName>("pcre2_substring_copy_byname_8");
    let (gnc, gnr) = both::<GetByNumber>("pcre2_substring_get_bynumber_8");
    let (gmc, gmr) = both::<GetByName>("pcre2_substring_get_byname_8");
    let (sfc, sfr) = both::<SubstringFree>("pcre2_substring_free_8");

    for &(pat, subs) in CASES {
        for &opts in &[0u32, PCRE2_DUPNAMES, PCRE2_MATCH_UNSET_BACKREF] {
            let Some(pair) = compile_both(pat, opts) else {
                continue;
            };
            for md_size in [0u32, 1, 2, 16] {
                let md = md_pair(md_size);
                for s in subs {
                    unsafe {
                        let rc = mc(pair.c, s.as_ptr(), s.len(), 0, 0, md.c, std::ptr::null_mut());
                        let rr = mr(pair.r, s.as_ptr(), s.len(), 0, 0, md.r, std::ptr::null_mut());
                        assert_eq!(rc, rr, "match {pat:02x?} / {s:02x?}");

                        for n in 0u32..12 {
                            // length by number
                            let mut a: PCRE2_SIZE = 0xdead;
                            let mut b: PCRE2_SIZE = 0xdead;
                            let x = lnc(md.c, n, &mut a);
                            let y = lnr(md.r, n, &mut b);
                            assert_eq!(x, y, "length_bynumber({n}) rc {pat:02x?}/{s:02x?}");
                            assert_eq!(a, b, "length_bynumber({n}) out {pat:02x?}/{s:02x?}");
                            // NULL sizeptr form
                            assert_eq!(
                                lnc(md.c, n, std::ptr::null_mut()),
                                lnr(md.r, n, std::ptr::null_mut()),
                                "length_bynumber({n}) NULL"
                            );

                            // copy by number, with several buffer sizes
                            for bufsz in [0usize, 1, 2, 4, 64] {
                                let mut bc = vec![0xAAu8; 64];
                                let mut br = vec![0xAAu8; 64];
                                let mut sa = bufsz;
                                let mut sb = bufsz;
                                let x = cnc(md.c, n, bc.as_mut_ptr(), &mut sa);
                                let y = cnr(md.r, n, br.as_mut_ptr(), &mut sb);
                                assert_eq!(
                                    x, y,
                                    "copy_bynumber({n}, {bufsz}) rc {pat:02x?}/{s:02x?}"
                                );
                                assert_eq!(sa, sb, "copy_bynumber({n}, {bufsz}) size");
                                assert_bytes_eq(
                                    &format!("copy_bynumber({n}, {bufsz}) buf"),
                                    &bc,
                                    &br,
                                );
                            }

                            // get by number
                            let mut pa: *mut PCRE2_UCHAR = std::ptr::null_mut();
                            let mut pb: *mut PCRE2_UCHAR = std::ptr::null_mut();
                            let mut sa: PCRE2_SIZE = 0xdead;
                            let mut sb: PCRE2_SIZE = 0xdead;
                            let x = gnc(md.c, n, &mut pa, &mut sa);
                            let y = gnr(md.r, n, &mut pb, &mut sb);
                            assert_eq!(x, y, "get_bynumber({n}) rc {pat:02x?}/{s:02x?}");
                            assert_eq!(sa, sb, "get_bynumber({n}) size");
                            assert_eq!(pa.is_null(), pb.is_null(), "get_bynumber({n}) nullness");
                            if x == 0 && !pa.is_null() {
                                assert_bytes_eq(
                                    &format!("get_bynumber({n}) data"),
                                    slice_at(pa, sa + 1),
                                    slice_at(pb, sb + 1),
                                );
                                sfc(pa);
                                sfr(pb);
                            }
                        }

                        for nm in NAMES {
                            let mut a: PCRE2_SIZE = 0xdead;
                            let mut b: PCRE2_SIZE = 0xdead;
                            let x = lmc(md.c, nm.as_ptr(), &mut a);
                            let y = lmr(md.r, nm.as_ptr(), &mut b);
                            assert_eq!(x, y, "length_byname({nm:02x?}) rc {pat:02x?}/{s:02x?}");
                            assert_eq!(a, b, "length_byname({nm:02x?}) out");

                            for bufsz in [0usize, 1, 4, 64] {
                                let mut bc = vec![0xAAu8; 64];
                                let mut br = vec![0xAAu8; 64];
                                let mut sa = bufsz;
                                let mut sb = bufsz;
                                let x = cmc(md.c, nm.as_ptr(), bc.as_mut_ptr(), &mut sa);
                                let y = cmr(md.r, nm.as_ptr(), br.as_mut_ptr(), &mut sb);
                                assert_eq!(x, y, "copy_byname({nm:02x?}, {bufsz}) rc");
                                assert_eq!(sa, sb, "copy_byname({nm:02x?}, {bufsz}) size");
                                assert_bytes_eq(
                                    &format!("copy_byname({nm:02x?}, {bufsz}) buf"),
                                    &bc,
                                    &br,
                                );
                            }

                            let mut pa: *mut PCRE2_UCHAR = std::ptr::null_mut();
                            let mut pb: *mut PCRE2_UCHAR = std::ptr::null_mut();
                            let mut sa: PCRE2_SIZE = 0xdead;
                            let mut sb: PCRE2_SIZE = 0xdead;
                            let x = gmc(md.c, nm.as_ptr(), &mut pa, &mut sa);
                            let y = gmr(md.r, nm.as_ptr(), &mut pb, &mut sb);
                            assert_eq!(x, y, "get_byname({nm:02x?}) rc");
                            assert_eq!(sa, sb, "get_byname({nm:02x?}) size");
                            assert_eq!(pa.is_null(), pb.is_null());
                            if x == 0 && !pa.is_null() {
                                assert_bytes_eq(
                                    &format!("get_byname({nm:02x?}) data"),
                                    slice_at(pa, sa + 1),
                                    slice_at(pb, sb + 1),
                                );
                                sfc(pa);
                                sfr(pb);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn substring_list_get_matches() {
    let (mc, mr) = both::<Match>("pcre2_match_8");
    let (lgc, lgr) = both::<ListGet>("pcre2_substring_list_get_8");
    let (lfc, lfr) = both::<ListFree>("pcre2_substring_list_free_8");
    let md = md_pair(16);
    for &(pat, subs) in CASES {
        let Some(pair) = compile_both(pat, 0) else {
            continue;
        };
        for s in subs {
            unsafe {
                let rc = mc(pair.c, s.as_ptr(), s.len(), 0, 0, md.c, std::ptr::null_mut());
                let rr = mr(pair.r, s.as_ptr(), s.len(), 0, 0, md.r, std::ptr::null_mut());
                assert_eq!(rc, rr);

                for want_lengths in [false, true] {
                    let mut la: *mut *mut PCRE2_UCHAR = std::ptr::null_mut();
                    let mut lb: *mut *mut PCRE2_UCHAR = std::ptr::null_mut();
                    let mut na: *mut PCRE2_SIZE = std::ptr::null_mut();
                    let mut nb: *mut PCRE2_SIZE = std::ptr::null_mut();
                    let x = lgc(
                        md.c,
                        &mut la,
                        if want_lengths { &mut na } else { std::ptr::null_mut() },
                    );
                    let y = lgr(
                        md.r,
                        &mut lb,
                        if want_lengths { &mut nb } else { std::ptr::null_mut() },
                    );
                    assert_eq!(x, y, "list_get rc {pat:02x?}/{s:02x?}");
                    if x != 0 {
                        continue;
                    }
                    // Walk both NULL-terminated lists of strings.
                    let mut i = 0usize;
                    loop {
                        let pa = *la.add(i);
                        let pb = *lb.add(i);
                        assert_eq!(pa.is_null(), pb.is_null(), "list entry {i} nullness");
                        if pa.is_null() {
                            break;
                        }
                        if want_lengths {
                            let ka = *na.add(i);
                            let kb = *nb.add(i);
                            assert_eq!(ka, kb, "list length {i}");
                            assert_bytes_eq(
                                &format!("list entry {i}"),
                                slice_at(pa, ka + 1),
                                slice_at(pb, kb + 1),
                            );
                        } else {
                            let a = std::ffi::CStr::from_ptr(pa as *const std::ffi::c_char);
                            let b = std::ffi::CStr::from_ptr(pb as *const std::ffi::c_char);
                            assert_eq!(a, b, "list entry {i}");
                        }
                        i += 1;
                    }
                    lfc(la);
                    lfr(lb);
                }
            }
        }
    }
}

#[test]
fn substring_free_null() {
    let (sfc, sfr) = both::<SubstringFree>("pcre2_substring_free_8");
    let (lfc, lfr) = both::<ListFree>("pcre2_substring_list_free_8");
    unsafe {
        sfc(std::ptr::null_mut());
        sfr(std::ptr::null_mut());
        lfc(std::ptr::null_mut());
        lfr(std::ptr::null_mut());
    }
}
