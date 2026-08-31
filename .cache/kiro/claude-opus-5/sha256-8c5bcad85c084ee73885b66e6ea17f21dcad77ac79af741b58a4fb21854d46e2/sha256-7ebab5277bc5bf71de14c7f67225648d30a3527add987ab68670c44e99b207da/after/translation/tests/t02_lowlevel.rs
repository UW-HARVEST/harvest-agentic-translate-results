//! `pcre2_string_utils.c`, `pcre2_chkdint.c`, `pcre2_ord2utf.c`,
//! `pcre2_valid_utf.c`, `pcre2_newline.c`, `pcre2_extuni.c`,
//! `pcre2_script_run.c` and `pcre2_maketables.c`.
mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

type StrLenFn = unsafe extern "C" fn(PCRE2_SPTR) -> PCRE2_SIZE;
type StrCmpFn = unsafe extern "C" fn(PCRE2_SPTR, PCRE2_SPTR) -> c_int;
type StrCmpC8Fn = unsafe extern "C" fn(PCRE2_SPTR, *const c_char) -> c_int;
type StrCpyC8Fn = unsafe extern "C" fn(*mut PCRE2_UCHAR, *const c_char) -> PCRE2_SIZE;
type StrNCmpFn = unsafe extern "C" fn(PCRE2_SPTR, PCRE2_SPTR, usize) -> c_int;
type StrNCmpC8Fn = unsafe extern "C" fn(PCRE2_SPTR, *const c_char, usize) -> c_int;
type CkdSMulFn = unsafe extern "C" fn(*mut PCRE2_SIZE, c_int, c_int) -> c_int;
type Ord2UtfFn = unsafe extern "C" fn(u32, *mut PCRE2_UCHAR) -> u32;
type ValidUtfFn = unsafe extern "C" fn(PCRE2_SPTR, PCRE2_SIZE, *mut PCRE2_SIZE) -> c_int;
type NewlineFn = unsafe extern "C" fn(PCRE2_SPTR, u32, PCRE2_SPTR, *mut u32, c_int) -> c_int;
type ExtUniFn =
    unsafe extern "C" fn(u32, PCRE2_SPTR, PCRE2_SPTR, PCRE2_SPTR, c_int, *mut c_int) -> PCRE2_SPTR;
type ScriptRunFn = unsafe extern "C" fn(PCRE2_SPTR, PCRE2_SPTR, c_int) -> c_int;
type MakeTablesFn = unsafe extern "C" fn(*mut c_void) -> *const u8;
type MakeTablesFreeFn = unsafe extern "C" fn(*mut c_void, *const u8);
type HashFromNameFn = unsafe extern "C" fn(PCRE2_SPTR, u32) -> u16;
type UpdateClassbitsFn = unsafe extern "C" fn(u32, u32, c_int, *mut u8);

/// A varied corpus of NUL-terminated code-unit strings.
fn strings() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"a\0".to_vec(),
        b"A\0".to_vec(),
        b"abc\0".to_vec(),
        b"abd\0".to_vec(),
        b"ab\0".to_vec(),
        b"abcd\0".to_vec(),
        b"ABC\0".to_vec(),
        b"name_1\0".to_vec(),
        b"name_2\0".to_vec(),
        b"\x7f\0".to_vec(),
        b"\x80\0".to_vec(),
        b"\xff\xfe\0".to_vec(),
        b"\xff\xff\0".to_vec(),
        b"0123456789\0".to_vec(),
        b"the quick brown fox\0".to_vec(),
        b"the quick brown fox jumped\0".to_vec(),
    ];
    // Every single code unit on its own.
    for c in 0u32..=255 {
        if c != 0 {
            v.push(vec![c as u8, 0]);
        }
    }
    v
}

#[test]
fn strlen_matches() {
    let (c, r) = both::<StrLenFn>("_pcre2_strlen_8");
    for s in strings() {
        unsafe {
            assert_eq!(c(s.as_ptr()), r(s.as_ptr()), "strlen {s:02x?}");
        }
    }
}

#[test]
fn strcmp_matches() {
    let (c, r) = both::<StrCmpFn>("_pcre2_strcmp_8");
    let ss = strings();
    for a in &ss {
        for b in &ss {
            unsafe {
                assert_eq!(
                    c(a.as_ptr(), b.as_ptr()),
                    r(a.as_ptr(), b.as_ptr()),
                    "strcmp {a:02x?} {b:02x?}"
                );
            }
        }
    }
}

#[test]
fn strcmp_c8_matches() {
    let (c, r) = both::<StrCmpC8Fn>("_pcre2_strcmp_c8_8");
    let ss = strings();
    for a in &ss {
        for b in &ss {
            unsafe {
                assert_eq!(
                    c(a.as_ptr(), b.as_ptr() as *const c_char),
                    r(a.as_ptr(), b.as_ptr() as *const c_char),
                    "strcmp_c8 {a:02x?} {b:02x?}"
                );
            }
        }
    }
}

#[test]
fn strncmp_matches() {
    let (c, r) = both::<StrNCmpFn>("_pcre2_strncmp_8");
    let ss = strings();
    for a in &ss {
        for b in &ss {
            for n in 0..6usize {
                unsafe {
                    assert_eq!(
                        c(a.as_ptr(), b.as_ptr(), n),
                        r(a.as_ptr(), b.as_ptr(), n),
                        "strncmp {a:02x?} {b:02x?} {n}"
                    );
                }
            }
        }
    }
}

#[test]
fn strncmp_c8_matches() {
    let (c, r) = both::<StrNCmpC8Fn>("_pcre2_strncmp_c8_8");
    let ss = strings();
    for a in &ss {
        for b in &ss {
            for n in 0..6usize {
                unsafe {
                    assert_eq!(
                        c(a.as_ptr(), b.as_ptr() as *const c_char, n),
                        r(a.as_ptr(), b.as_ptr() as *const c_char, n),
                        "strncmp_c8 {a:02x?} {b:02x?} {n}"
                    );
                }
            }
        }
    }
}

#[test]
fn strcpy_c8_matches() {
    let (c, r) = both::<StrCpyC8Fn>("_pcre2_strcpy_c8_8");
    for s in strings() {
        let mut bc = [0xAAu8; 64];
        let mut br = [0xAAu8; 64];
        unsafe {
            let nc = c(bc.as_mut_ptr(), s.as_ptr() as *const c_char);
            let nr = r(br.as_mut_ptr(), s.as_ptr() as *const c_char);
            assert_eq!(nc, nr, "strcpy_c8 return {s:02x?}");
            assert_bytes_eq(&format!("strcpy_c8 buffer {s:02x?}"), &bc, &br);
        }
    }
}

#[test]
fn ckd_smul_matches() {
    let (c, r) = both::<CkdSMulFn>("_pcre2_ckd_smul_8");
    let vals: [c_int; 15] = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        100,
        -100,
        65535,
        65536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 16,
    ];
    for &a in &vals {
        for &b in &vals {
            let mut oc: PCRE2_SIZE = 0xdead_beef;
            let mut orr: PCRE2_SIZE = 0xdead_beef;
            unsafe {
                let rc = c(&mut oc, a, b);
                let rr = r(&mut orr, a, b);
                assert_eq!(rc != 0, rr != 0, "ckd_smul rc {a} * {b}");
                assert_eq!(oc, orr, "ckd_smul out {a} * {b}");
            }
        }
    }
}

#[test]
fn ord2utf_matches() {
    let (c, r) = both::<Ord2UtfFn>("_pcre2_ord2utf_8");
    let mut cps: Vec<u32> = (0u32..=0x2000).collect();
    cps.extend([
        0x7f, 0x80, 0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff, 0x110000, 0x1fffff, 0x200000,
        0x3ffffff, 0x4000000, 0x7fffffff,
    ]);
    for cp in cps {
        let mut bc = [0xAAu8; 16];
        let mut br = [0xAAu8; 16];
        unsafe {
            let nc = c(cp, bc.as_mut_ptr());
            let nr = r(cp, br.as_mut_ptr());
            assert_eq!(nc, nr, "ord2utf length {cp:#x}");
            assert_bytes_eq(&format!("ord2utf bytes {cp:#x}"), &bc, &br);
        }
    }
}

fn utf_subjects() -> Vec<Vec<u8>> {
    vec![
        b"".to_vec(),
        b"abc".to_vec(),
        "héllo wörld".as_bytes().to_vec(),
        "日本語テキスト".as_bytes().to_vec(),
        "𝒜𝒷𝒸 emoji 😀🎉".as_bytes().to_vec(),
        vec![0x80],                   // isolated continuation
        vec![0xc0, 0x80],            // overlong
        vec![0xc1, 0xbf],            // overlong
        vec![0xc2],                  // truncated 2-byte
        vec![0xc2, 0x41],            // bad continuation
        vec![0xe0, 0x80, 0x80],      // overlong 3-byte
        vec![0xe0, 0xa0],            // truncated 3-byte
        vec![0xed, 0xa0, 0x80],      // surrogate
        vec![0xef, 0xbf, 0xbd],      // U+FFFD, valid
        vec![0xf0, 0x80, 0x80, 0x80],
        vec![0xf0, 0x90, 0x80, 0x80],       // valid U+10000
        vec![0xf4, 0x90, 0x80, 0x80],       // > U+10FFFF
        vec![0xf5, 0x80, 0x80, 0x80],
        vec![0xf8, 0x88, 0x80, 0x80, 0x80], // 5-byte
        vec![0xfc, 0x84, 0x80, 0x80, 0x80, 0x80],
        vec![0xfe],
        vec![0xff],
        b"ok then \xf0\x9f\x98\x80 and \xc3".to_vec(),
        b"a\xffb".to_vec(),
        vec![0xe2, 0x80], // truncated at end
    ]
}

#[test]
fn valid_utf_matches() {
    let (c, r) = both::<ValidUtfFn>("_pcre2_valid_utf_8");
    for s in utf_subjects() {
        for len in 0..=s.len() {
            let mut oc: PCRE2_SIZE = 0xdead;
            let mut orr: PCRE2_SIZE = 0xdead;
            unsafe {
                let rc = c(s.as_ptr(), len, &mut oc);
                let rr = r(s.as_ptr(), len, &mut orr);
                assert_eq!(rc, rr, "valid_utf rc {s:02x?} len={len}");
                assert_eq!(oc, orr, "valid_utf offset {s:02x?} len={len}");
            }
        }
    }
    // Also exercise every single byte and every 2-byte sequence.
    for a in 0u32..=255 {
        let s = [a as u8];
        let mut oc = 0;
        let mut orr = 0;
        unsafe {
            assert_eq!(c(s.as_ptr(), 1, &mut oc), r(s.as_ptr(), 1, &mut orr));
            assert_eq!(oc, orr);
        }
        for b in 0u32..=255 {
            let s = [a as u8, b as u8];
            let mut oc = 0;
            let mut orr = 0;
            unsafe {
                let rc = c(s.as_ptr(), 2, &mut oc);
                let rr = r(s.as_ptr(), 2, &mut orr);
                assert_eq!(rc, rr, "valid_utf rc {a:#x} {b:#x}");
                assert_eq!(oc, orr, "valid_utf off {a:#x} {b:#x}");
            }
        }
    }
}

const NEWLINE_TYPES: [u32; 6] = [1, 2, 3, 4, 5, 6]; // CR, LF, CRLF, ANY, ANYCRLF, NUL

#[test]
fn is_newline_matches() {
    let (c, r) = both::<NewlineFn>("_pcre2_is_newline_8");
    let subjects: Vec<Vec<u8>> = vec![
        b"\r\n".to_vec(),
        b"\n\r".to_vec(),
        b"\r".to_vec(),
        b"\n".to_vec(),
        b"\x0b".to_vec(),
        b"\x0c".to_vec(),
        b"\x85".to_vec(),
        b"\xc2\x85".to_vec(),
        b"\xe2\x80\xa8".to_vec(),
        b"\xe2\x80\xa9".to_vec(),
        b"\0".to_vec(),
        b"a".to_vec(),
        b"\r\n\r\n".to_vec(),
        b"\rx".to_vec(),
    ];
    // The C code dereferences `ptr` unconditionally (and may read one past it),
    // so place every subject inside a zero-padded buffer that both libraries see
    // identically.
    for s in &subjects {
        let mut buf = [0u8; 64];
        buf[..s.len()].copy_from_slice(s);
        for &nl in &NEWLINE_TYPES {
            for &utf in &[0, 1] {
                let start = buf.as_ptr();
                let end = unsafe { start.add(s.len()) };
                let mut lc: u32 = 0xdead;
                let mut lr: u32 = 0xdead;
                unsafe {
                    let rc = c(start, nl, end, &mut lc, utf);
                    let rr = r(start, nl, end, &mut lr, utf);
                    assert_eq!(rc != 0, rr != 0, "is_newline rc {s:02x?} nl={nl} utf={utf}");
                    if rc != 0 {
                        assert_eq!(lc, lr, "is_newline len {s:02x?} nl={nl} utf={utf}");
                    }
                }
            }
        }
    }
    // Exhaustive single code units in non-UTF mode.
    for b in 0u32..=255 {
        let buf = [b as u8, 0, 0, 0];
        for &nl in &NEWLINE_TYPES {
            let mut lc: u32 = 0xdead;
            let mut lr: u32 = 0xdead;
            unsafe {
                let rc = c(buf.as_ptr(), nl, buf.as_ptr().add(1), &mut lc, 0);
                let rr = r(buf.as_ptr(), nl, buf.as_ptr().add(1), &mut lr, 0);
                assert_eq!(rc != 0, rr != 0, "is_newline rc byte={b:#x} nl={nl}");
                if rc != 0 {
                    assert_eq!(lc, lr, "is_newline len byte={b:#x} nl={nl}");
                }
            }
        }
    }
}

#[test]
fn was_newline_matches() {
    let (c, r) = both::<NewlineFn>("_pcre2_was_newline_8");
    let subjects: Vec<Vec<u8>> = vec![
        b"x\r\n".to_vec(),
        b"x\n".to_vec(),
        b"x\r".to_vec(),
        b"x\x0b".to_vec(),
        b"x\x0c".to_vec(),
        b"x\x85".to_vec(),
        b"x\xc2\x85".to_vec(),
        b"x\xe2\x80\xa8".to_vec(),
        b"x\xe2\x80\xa9".to_vec(),
        b"x\0".to_vec(),
        b"xa".to_vec(),
        b"\n".to_vec(),
        b"\r".to_vec(),
        b"\xc2\x85".to_vec(),
        b"\xe2\x80\xa8".to_vec(),
        b"\r\n\r\n".to_vec(),
    ];
    for s in &subjects {
        let mut buf = [0u8; 64];
        buf[..s.len()].copy_from_slice(s);
        for &nl in &NEWLINE_TYPES {
            for &utf in &[0, 1] {
                // `ptr` is one past the newline; `startptr` is the subject start.
                let start = buf.as_ptr();
                let ptr = unsafe { start.add(s.len()) };
                let mut lc: u32 = 0xdead;
                let mut lr: u32 = 0xdead;
                unsafe {
                    let rc = c(ptr, nl, start, &mut lc, utf);
                    let rr = r(ptr, nl, start, &mut lr, utf);
                    assert_eq!(rc != 0, rr != 0, "was_newline rc {s:02x?} nl={nl} utf={utf}");
                    if rc != 0 {
                        assert_eq!(lc, lr, "was_newline len {s:02x?} nl={nl} utf={utf}");
                    }
                }
            }
        }
    }
    // Exhaustive single trailing code units in non-UTF mode.
    for b in 0u32..=255 {
        let buf = [b as u8, 0, 0, 0];
        for &nl in &NEWLINE_TYPES {
            let mut lc: u32 = 0xdead;
            let mut lr: u32 = 0xdead;
            unsafe {
                let rc = c(buf.as_ptr().add(1), nl, buf.as_ptr(), &mut lc, 0);
                let rr = r(buf.as_ptr().add(1), nl, buf.as_ptr(), &mut lr, 0);
                assert_eq!(rc != 0, rr != 0, "was_newline rc byte={b:#x} nl={nl}");
                if rc != 0 {
                    assert_eq!(lc, lr, "was_newline len byte={b:#x} nl={nl}");
                }
            }
        }
    }
}

#[test]
fn extuni_matches() {
    let (c, r) = both::<ExtUniFn>("_pcre2_extuni_8");
    let subjects: Vec<Vec<u8>> = vec![
        "e\u{301}\u{302}xyz".as_bytes().to_vec(),
        "\u{1100}\u{1161}\u{11a8}rest".as_bytes().to_vec(),
        "\u{1F1E6}\u{1F1E7}\u{1F1E8}".as_bytes().to_vec(),
        "a\u{200D}b".as_bytes().to_vec(),
        "\u{0BCA}\u{0BBE}x".as_bytes().to_vec(),
        "abc".as_bytes().to_vec(),
        "\u{1F600}\u{FE0F}\u{200D}\u{1F5E8}".as_bytes().to_vec(),
        "\r\na".as_bytes().to_vec(),
        "\u{0903}\u{0904}".as_bytes().to_vec(),
    ];
    for s in &subjects {
        // Decode the first character to pass as `c`.
        let text = String::from_utf8_lossy(s).to_string();
        let mut chars = text.char_indices();
        let (_, first) = match chars.next() {
            Some(x) => x,
            None => continue,
        };
        let first_len = first.len_utf8();
        for &utf in &[0, 1] {
            let end = unsafe { s.as_ptr().add(s.len()) };
            let start = unsafe { s.as_ptr().add(first_len) };
            let mut xc: c_int = 0;
            let mut xr: c_int = 0;
            unsafe {
                let pc = c(first as u32, start, s.as_ptr(), end, utf, &mut xc);
                let pr = r(first as u32, start, s.as_ptr(), end, utf, &mut xr);
                assert_eq!(
                    pc.offset_from(s.as_ptr()),
                    pr.offset_from(s.as_ptr()),
                    "extuni ptr {s:02x?} utf={utf}"
                );
                assert_eq!(xc, xr, "extuni xcount {s:02x?} utf={utf}");
            }
        }
    }
}

#[test]
fn script_run_matches() {
    let (c, r) = both::<ScriptRunFn>("_pcre2_script_run_8");
    let subjects: Vec<Vec<u8>> = vec![
        b"abcdef".to_vec(),
        b"abc123".to_vec(),
        "abcабв".as_bytes().to_vec(),
        "аbс".as_bytes().to_vec(),
        "日本語".as_bytes().to_vec(),
        "日本語abc".as_bytes().to_vec(),
        "ひらがなカタカナ漢字".as_bytes().to_vec(),
        "٠١٢٣".as_bytes().to_vec(),
        "٠1٢".as_bytes().to_vec(),
        "\u{0300}abc".as_bytes().to_vec(),
        "abc\u{0300}".as_bytes().to_vec(),
        b"".to_vec(),
        b"a".to_vec(),
        "\u{3131}\u{1100}".as_bytes().to_vec(),
        "Ωμέγα".as_bytes().to_vec(),
    ];
    for s in &subjects {
        for &utf in &[0, 1] {
            let end = unsafe { s.as_ptr().add(s.len()) };
            unsafe {
                let rc = c(s.as_ptr(), end, utf);
                let rr = r(s.as_ptr(), end, utf);
                assert_eq!(rc != 0, rr != 0, "script_run {s:02x?} utf={utf}");
            }
        }
    }
}

#[test]
fn maketables_matches() {
    let (c, r) = both::<MakeTablesFn>("pcre2_maketables_8");
    let (cf, rf) = both::<MakeTablesFreeFn>("pcre2_maketables_free_8");
    unsafe {
        let tc = c(std::ptr::null_mut());
        let tr = r(std::ptr::null_mut());
        assert!(!tc.is_null() && !tr.is_null());
        assert_bytes_eq("maketables", slice_at(tc, 1088), slice_at(tr, 1088));
        cf(std::ptr::null_mut(), tc);
        rf(std::ptr::null_mut(), tr);
    }
}

#[test]
fn hash_from_name_matches() {
    let (c, r) = both::<HashFromNameFn>("_pcre2_compile_get_hash_from_name8");
    let names: Vec<&[u8]> = vec![
        b"a", b"b", b"ab", b"ba", b"name", b"NAME", b"n1", b"n2", b"group_one", b"group_two",
        b"xyzzy", b"\xff\x01\xa5", b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ];
    for n in &names {
        // The C code asserts length > 0 and reads name[length - 1].
        for len in 1..=n.len() as u32 {
            unsafe {
                assert_eq!(
                    c(n.as_ptr(), len),
                    r(n.as_ptr(), len),
                    "hash {n:02x?} len={len}"
                );
            }
        }
    }
}

#[test]
fn update_classbits_matches() {
    let (c, r) = both::<UpdateClassbitsFn>("_pcre2_update_classbits_8");
    // ptype values are PT_* (0..=PT_TABULATED_LAST); pdata varies per type.
    for ptype in 0u32..=24 {
        for pdata in 0u32..=40 {
            for negated in [0, 1] {
                let mut bc = [0u8; 32];
                let mut br = [0u8; 32];
                unsafe {
                    c(ptype, pdata, negated, bc.as_mut_ptr());
                    r(ptype, pdata, negated, br.as_mut_ptr());
                }
                assert_bytes_eq(
                    &format!("update_classbits ptype={ptype} pdata={pdata} neg={negated}"),
                    &bc,
                    &br,
                );
            }
        }
    }
}
