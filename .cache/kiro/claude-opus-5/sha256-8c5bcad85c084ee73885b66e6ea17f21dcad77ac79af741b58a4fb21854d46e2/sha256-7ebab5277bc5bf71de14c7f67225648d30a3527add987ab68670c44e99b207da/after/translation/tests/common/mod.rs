//! Shared harness: loads both the C and the Rust `libpcre2.so` via `libloading`
//! and exposes helpers for symbol lookup so that every comparison goes through
//! the real dynamic-linker FFI boundary.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type PCRE2_SIZE = usize;
pub type PCRE2_SPTR = *const u8;
pub type PCRE2_UCHAR = u8;

pub const PCRE2_UNSET: PCRE2_SIZE = PCRE2_SIZE::MAX;
pub const PCRE2_ZERO_TERMINATED: PCRE2_SIZE = PCRE2_SIZE::MAX;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("PCRE2_C_SO") {
        return PathBuf::from(p);
    }
    workspace_root().join("c_src/build/libpcre2.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("PCRE2_RUST_SO") {
        return PathBuf::from(p);
    }
    // tests/<name> binaries live in target/<profile>/deps, so walk up from the
    // current exe to find the sibling cdylib produced by the same build.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            let cand = deps.join("libpcre2.so");
            if cand.exists() {
                return cand;
            }
            if let Some(profile) = deps.parent() {
                let cand = profile.join("libpcre2.so");
                if cand.exists() {
                    return cand;
                }
            }
        }
    }
    workspace_root().join("translation/target/release/libpcre2.so")
}

pub struct Libs {
    pub c: Library,
    pub r: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        unsafe {
            let c = Library::new(&cp).unwrap_or_else(|e| panic!("load C so {cp:?}: {e}"));
            let r = Library::new(&rp).unwrap_or_else(|e| panic!("load Rust so {rp:?}: {e}"));
            Libs { c, r }
        }
    })
}

/// Fetch the same symbol from both libraries as a function pointer type `T`.
pub fn both<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let l = libs();
    let mut b = name.as_bytes().to_vec();
    b.push(0);
    unsafe {
        let cs: Symbol<'static, T> = l
            .c
            .get(&b)
            .unwrap_or_else(|e| panic!("C symbol {name}: {e}"));
        let rs: Symbol<'static, T> = l
            .r
            .get(&b)
            .unwrap_or_else(|e| panic!("Rust symbol {name}: {e}"));
        (cs, rs)
    }
}

/// Fetch a data symbol address from both libraries.
pub fn both_data(name: &str) -> (*const u8, *const u8) {
    let l = libs();
    let mut b = name.as_bytes().to_vec();
    b.push(0);
    unsafe {
        let cs: Symbol<'static, *const u8> = l
            .c
            .get(&b)
            .unwrap_or_else(|e| panic!("C data symbol {name}: {e}"));
        let rs: Symbol<'static, *const u8> = l
            .r
            .get(&b)
            .unwrap_or_else(|e| panic!("Rust data symbol {name}: {e}"));
        // `Symbol::into_raw` would give the address of the pointer's *value*;
        // we want the address of the object itself.
        (
            cs.into_raw().into_raw() as *const u8,
            rs.into_raw().into_raw() as *const u8,
        )
    }
}

pub unsafe fn slice_at<'a>(p: *const u8, len: usize) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(p, len) }
}

pub fn hexdiff(a: &[u8], b: &[u8]) -> String {
    if a.len() != b.len() {
        return format!("length {} vs {}", a.len(), b.len());
    }
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            let lo = i.saturating_sub(8);
            let hi = (i + 8).min(a.len());
            return format!(
                "first difference at byte {i}: {x:#04x} vs {y:#04x}\n  C: {:02x?}\n  R: {:02x?}",
                &a[lo..hi],
                &b[lo..hi]
            );
        }
    }
    "identical".to_string()
}

pub fn assert_bytes_eq(ctx: &str, a: &[u8], b: &[u8]) {
    if a != b {
        panic!("{ctx}: {}", hexdiff(a, b));
    }
}

/* ------------------------------------------------------------------ */
/* Structure layouts mirrored from pcre2_intmodedep.h                  */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealCodeTail {
    pub blocksize: PCRE2_SIZE,
    pub code_start: PCRE2_SIZE,
    pub magic_number: u32,
    pub compile_options: u32,
    pub overall_options: u32,
    pub extra_options: u32,
    pub flags: u32,
    pub limit_heap: u32,
    pub limit_match: u32,
    pub limit_depth: u32,
    pub first_codeunit: u32,
    pub last_codeunit: u32,
    pub bsr_convention: u16,
    pub newline_convention: u16,
    pub max_lookbehind: u16,
    pub minlength: u16,
    pub top_bracket: u16,
    pub top_backref: u16,
    pub name_entry_size: u16,
    pub name_count: u16,
    pub optimization_flags: u32,
}

/// Offsets inside `pcre2_real_code` (LP64, no SUPPORT_JIT in the match context).
pub const CODE_OFF_MEMCTL: usize = 0;
pub const CODE_OFF_TABLES: usize = 24;
pub const CODE_OFF_EXECUTABLE_JIT: usize = 32;
pub const CODE_OFF_START_BITMAP: usize = 40;
pub const CODE_OFF_TAIL: usize = 72;
pub const CODE_SIZE: usize = 72 + std::mem::size_of::<RealCodeTail>();

/// Read the comparable (address-independent) parts of a compiled `pcre2_code`.
pub unsafe fn code_snapshot(code: *const c_void) -> (Vec<u8>, RealCodeTail, Vec<u8>) {
    unsafe {
        let base = code as *const u8;
        let bitmap = slice_at(base.add(CODE_OFF_START_BITMAP), 32).to_vec();
        let tail = std::ptr::read_unaligned(base.add(CODE_OFF_TAIL) as *const RealCodeTail);
        // Everything from code_start to blocksize is the byte code + name table.
        let body = slice_at(base.add(CODE_OFF_TAIL), tail.blocksize - CODE_OFF_TAIL).to_vec();
        (bitmap, tail, body)
    }
}

/// Offsets inside `pcre2_real_match_data`.
pub const MD_OFF_CODE: usize = 24;
pub const MD_OFF_SUBJECT: usize = 32;
pub const MD_OFF_MARK: usize = 40;
pub const MD_OFF_HEAPFRAMES: usize = 48;
pub const MD_OFF_HEAPFRAMES_SIZE: usize = 56;
pub const MD_OFF_SUBJECT_LENGTH: usize = 64;
pub const MD_OFF_START_OFFSET: usize = 72;
pub const MD_OFF_LEFTCHAR: usize = 80;
pub const MD_OFF_RIGHTCHAR: usize = 88;
pub const MD_OFF_STARTCHAR: usize = 96;
pub const MD_OFF_MATCHEDBY: usize = 104;
pub const MD_OFF_FLAGS: usize = 105;
pub const MD_OFF_OVECCOUNT: usize = 106;
pub const MD_OFF_OPTIONS: usize = 108;
pub const MD_OFF_RC: usize = 112;
pub const MD_OFF_OVECTOR: usize = 120;

#[derive(Debug, PartialEq, Eq)]
pub enum PtrField {
    /// The library never wrote this field (it still holds the poison pattern).
    Untouched,
    Null,
    /// Offset relative to the subject buffer.
    Offset(isize),
    /// Written, but not pointing into the subject buffer (e.g. a copied subject).
    Foreign,
}

/// Byte written across the parts of a `pcre2_match_data` that the library is
/// expected to fill in, so that fields neither library writes still compare
/// equal instead of exposing malloc garbage.
pub const MD_POISON: u8 = 0x5A;
pub const MD_POISON_WORD: usize = usize::from_ne_bytes([MD_POISON; 8]);

/// Ranges of `pcre2_match_data` that are safe to poison. `heapframes`,
/// `heapframes_size`, `flags` and `oveccount` are owned by
/// `pcre2_match_data_create` and must survive.
pub unsafe fn md_poison(md: *mut c_void, oveccount: u16) {
    unsafe {
        let base = md as *mut u8;
        std::ptr::write_bytes(base.add(MD_OFF_CODE), MD_POISON, 48 - MD_OFF_CODE);
        std::ptr::write_bytes(base.add(MD_OFF_SUBJECT_LENGTH), MD_POISON, 105 - 64);
        std::ptr::write_bytes(
            base.add(MD_OFF_OPTIONS),
            MD_POISON,
            (MD_OFF_OVECTOR - MD_OFF_OPTIONS) + 2 * oveccount as usize * 8,
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MatchDataSnapshot {
    pub code: PtrField,
    pub subject: PtrField,
    pub mark: Option<Vec<u8>>,
    pub mark_field: PtrField,
    pub subject_length: PCRE2_SIZE,
    pub start_offset: PCRE2_SIZE,
    pub leftchar: PCRE2_SIZE,
    pub rightchar: PCRE2_SIZE,
    pub startchar: PCRE2_SIZE,
    pub matchedby: u8,
    pub flags: u8,
    pub oveccount: u16,
    pub options: u32,
    pub rc: i32,
    pub ovector: Vec<PCRE2_SIZE>,
}

fn classify(raw: usize, subject: *const u8, len: usize) -> PtrField {
    if raw == MD_POISON_WORD {
        PtrField::Untouched
    } else if raw == 0 {
        PtrField::Null
    } else {
        let base = subject as usize;
        if raw >= base && raw <= base + len {
            PtrField::Offset((raw - base) as isize)
        } else {
            PtrField::Foreign
        }
    }
}

pub unsafe fn md_snapshot(
    md: *const c_void,
    subject: *const u8,
    subject_len: usize,
) -> MatchDataSnapshot {
    unsafe {
        let base = md as *const u8;
        let rd8 = |off: usize| std::ptr::read_unaligned(base.add(off) as *const usize);
        let rd4 = |off: usize| std::ptr::read_unaligned(base.add(off) as *const u32);
        let markraw = rd8(MD_OFF_MARK);
        let oveccount = std::ptr::read_unaligned(base.add(MD_OFF_OVECCOUNT) as *const u16);
        let mark_field = classify(markraw, subject, subject_len);
        // The mark points into the compiled pattern, so read the string itself.
        let mark = if markraw == 0 || markraw == MD_POISON_WORD {
            None
        } else {
            let mut v = Vec::new();
            let mut p = markraw as *const u8;
            while *p != 0 {
                v.push(*p);
                p = p.add(1);
            }
            Some(v)
        };
        let ovector = (0..(oveccount as usize) * 2)
            .map(|i| std::ptr::read_unaligned((base.add(MD_OFF_OVECTOR) as *const usize).add(i)))
            .collect();
        MatchDataSnapshot {
            code: classify(rd8(MD_OFF_CODE), subject, subject_len),
            subject: classify(rd8(MD_OFF_SUBJECT), subject, subject_len),
            mark,
            mark_field,
            subject_length: rd8(MD_OFF_SUBJECT_LENGTH),
            start_offset: rd8(MD_OFF_START_OFFSET),
            leftchar: rd8(MD_OFF_LEFTCHAR),
            rightchar: rd8(MD_OFF_RIGHTCHAR),
            startchar: rd8(MD_OFF_STARTCHAR),
            matchedby: *base.add(MD_OFF_MATCHEDBY),
            flags: *base.add(MD_OFF_FLAGS),
            oveccount,
            options: rd4(MD_OFF_OPTIONS),
            rc: rd4(MD_OFF_RC) as i32,
            ovector,
        }
    }
}

/* ------------------------------------------------------------------ */
/* Convenience bindings for the API used by many test files            */
/* ------------------------------------------------------------------ */

pub type CompileFn = unsafe extern "C" fn(
    PCRE2_SPTR,
    PCRE2_SIZE,
    u32,
    *mut i32,
    *mut PCRE2_SIZE,
    *mut c_void,
) -> *mut c_void;
pub type CodeFreeFn = unsafe extern "C" fn(*mut c_void);
pub type MatchFn = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    PCRE2_SIZE,
    u32,
    *mut c_void,
    *mut c_void,
) -> i32;
pub type MdCreateFn = unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
pub type MdCreateFromPatFn = unsafe extern "C" fn(*const c_void, *mut c_void) -> *mut c_void;
pub type MdFreeFn = unsafe extern "C" fn(*mut c_void);
pub type PatternInfoFn = unsafe extern "C" fn(*const c_void, u32, *mut c_void) -> i32;

/// A pair of compiled patterns (C-side and Rust-side) plus the loaded symbols
/// needed to free them.
pub struct CodePair {
    pub c: *mut c_void,
    pub r: *mut c_void,
    c_free: Symbol<'static, CodeFreeFn>,
    r_free: Symbol<'static, CodeFreeFn>,
}

impl Drop for CodePair {
    fn drop(&mut self) {
        unsafe {
            if !self.c.is_null() {
                (self.c_free)(self.c);
            }
            if !self.r.is_null() {
                (self.r_free)(self.r);
            }
        }
    }
}

/// Compile `pattern` with both libraries, asserting that the error code, error
/// offset, and (on success) the whole compiled block agree.
pub fn compile_both(pattern: &[u8], options: u32) -> Option<CodePair> {
    compile_both_ctx(pattern, options, std::ptr::null_mut(), std::ptr::null_mut())
}

pub fn compile_both_ctx(
    pattern: &[u8],
    options: u32,
    cctx_c: *mut c_void,
    cctx_r: *mut c_void,
) -> Option<CodePair> {
    let (cc, rc) = both::<CompileFn>("pcre2_compile_8");
    let (cf, rf) = both::<CodeFreeFn>("pcre2_code_free_8");

    let mut ec_c: i32 = -999;
    let mut eo_c: PCRE2_SIZE = usize::MAX;
    let mut ec_r: i32 = -999;
    let mut eo_r: PCRE2_SIZE = usize::MAX;

    unsafe {
        let code_c = cc(
            pattern.as_ptr(),
            pattern.len(),
            options,
            &mut ec_c,
            &mut eo_c,
            cctx_c,
        );
        let code_r = rc(
            pattern.as_ptr(),
            pattern.len(),
            options,
            &mut ec_r,
            &mut eo_r,
            cctx_r,
        );

        let show = String::from_utf8_lossy(pattern).to_string();
        assert_eq!(
            ec_c, ec_r,
            "compile errorcode mismatch for {show:?} options={options:#x}"
        );
        assert_eq!(
            eo_c, eo_r,
            "compile erroroffset mismatch for {show:?} options={options:#x} (rc {ec_c})"
        );
        assert_eq!(
            code_c.is_null(),
            code_r.is_null(),
            "compile null-ness mismatch for {show:?}"
        );

        if code_c.is_null() {
            return None;
        }

        let (bm_c, tail_c, body_c) = code_snapshot(code_c);
        let (bm_r, tail_r, body_r) = code_snapshot(code_r);
        assert_bytes_eq(&format!("start_bitmap for {show:?}"), &bm_c, &bm_r);
        assert_eq!(tail_c, tail_r, "code header mismatch for {show:?}");
        assert_bytes_eq(&format!("code body for {show:?}"), &body_c, &body_r);

        Some(CodePair {
            c: code_c,
            r: code_r,
            c_free: cf,
            r_free: rf,
        })
    }
}

/* ------------------------------------------------------------------ */
/* Option constants                                                    */
/* ------------------------------------------------------------------ */

pub const PCRE2_ANCHORED: u32 = 0x8000_0000;
pub const PCRE2_NO_UTF_CHECK: u32 = 0x4000_0000;
pub const PCRE2_ENDANCHORED: u32 = 0x2000_0000;
pub const PCRE2_ALLOW_EMPTY_CLASS: u32 = 0x0000_0001;
pub const PCRE2_ALT_BSUX: u32 = 0x0000_0002;
pub const PCRE2_AUTO_CALLOUT: u32 = 0x0000_0004;
pub const PCRE2_CASELESS: u32 = 0x0000_0008;
pub const PCRE2_DOLLAR_ENDONLY: u32 = 0x0000_0010;
pub const PCRE2_DOTALL: u32 = 0x0000_0020;
pub const PCRE2_DUPNAMES: u32 = 0x0000_0040;
pub const PCRE2_EXTENDED: u32 = 0x0000_0080;
pub const PCRE2_FIRSTLINE: u32 = 0x0000_0100;
pub const PCRE2_MATCH_UNSET_BACKREF: u32 = 0x0000_0200;
pub const PCRE2_MULTILINE: u32 = 0x0000_0400;
pub const PCRE2_NEVER_UCP: u32 = 0x0000_0800;
pub const PCRE2_NEVER_UTF: u32 = 0x0000_1000;
pub const PCRE2_NO_AUTO_CAPTURE: u32 = 0x0000_2000;
pub const PCRE2_NO_AUTO_POSSESS: u32 = 0x0000_4000;
pub const PCRE2_NO_DOTSTAR_ANCHOR: u32 = 0x0000_8000;
pub const PCRE2_NO_START_OPTIMIZE: u32 = 0x0001_0000;
pub const PCRE2_UCP: u32 = 0x0002_0000;
pub const PCRE2_UNGREEDY: u32 = 0x0004_0000;
pub const PCRE2_UTF: u32 = 0x0008_0000;
pub const PCRE2_NEVER_BACKSLASH_C: u32 = 0x0010_0000;
pub const PCRE2_ALT_CIRCUMFLEX: u32 = 0x0020_0000;
pub const PCRE2_ALT_VERBNAMES: u32 = 0x0040_0000;
pub const PCRE2_USE_OFFSET_LIMIT: u32 = 0x0080_0000;
pub const PCRE2_EXTENDED_MORE: u32 = 0x0100_0000;
pub const PCRE2_LITERAL: u32 = 0x0200_0000;
pub const PCRE2_MATCH_INVALID_UTF: u32 = 0x0400_0000;
pub const PCRE2_ALT_EXTENDED_CLASS: u32 = 0x0800_0000;

pub const PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES: u32 = 0x0000_0001;
pub const PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL: u32 = 0x0000_0002;
pub const PCRE2_EXTRA_MATCH_WORD: u32 = 0x0000_0004;
pub const PCRE2_EXTRA_MATCH_LINE: u32 = 0x0000_0008;
pub const PCRE2_EXTRA_ESCAPED_CR_IS_LF: u32 = 0x0000_0010;
pub const PCRE2_EXTRA_ALT_BSUX: u32 = 0x0000_0020;
pub const PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK: u32 = 0x0000_0040;
pub const PCRE2_EXTRA_CASELESS_RESTRICT: u32 = 0x0000_0080;
pub const PCRE2_EXTRA_ASCII_BSD: u32 = 0x0000_0100;
pub const PCRE2_EXTRA_ASCII_BSS: u32 = 0x0000_0200;
pub const PCRE2_EXTRA_ASCII_BSW: u32 = 0x0000_0400;
pub const PCRE2_EXTRA_ASCII_POSIX: u32 = 0x0000_0800;
pub const PCRE2_EXTRA_ASCII_DIGIT: u32 = 0x0000_1000;
pub const PCRE2_EXTRA_PYTHON_OCTAL: u32 = 0x0000_2000;
pub const PCRE2_EXTRA_NO_BS0: u32 = 0x0000_4000;
pub const PCRE2_EXTRA_NEVER_CALLOUT: u32 = 0x0000_8000;
pub const PCRE2_EXTRA_TURKISH_CASING: u32 = 0x0001_0000;

pub const PCRE2_NOTBOL: u32 = 0x0000_0001;
pub const PCRE2_NOTEOL: u32 = 0x0000_0002;
pub const PCRE2_NOTEMPTY: u32 = 0x0000_0004;
pub const PCRE2_NOTEMPTY_ATSTART: u32 = 0x0000_0008;
pub const PCRE2_PARTIAL_SOFT: u32 = 0x0000_0010;
pub const PCRE2_PARTIAL_HARD: u32 = 0x0000_0020;
pub const PCRE2_DFA_RESTART: u32 = 0x0000_0040;
pub const PCRE2_DFA_SHORTEST: u32 = 0x0000_0080;
pub const PCRE2_SUBSTITUTE_GLOBAL: u32 = 0x0000_0100;
pub const PCRE2_SUBSTITUTE_EXTENDED: u32 = 0x0000_0200;
pub const PCRE2_SUBSTITUTE_UNSET_EMPTY: u32 = 0x0000_0400;
pub const PCRE2_SUBSTITUTE_UNKNOWN_UNSET: u32 = 0x0000_0800;
pub const PCRE2_SUBSTITUTE_OVERFLOW_LENGTH: u32 = 0x0000_1000;
pub const PCRE2_NO_JIT: u32 = 0x0000_2000;
pub const PCRE2_COPY_MATCHED_SUBJECT: u32 = 0x0000_4000;
pub const PCRE2_SUBSTITUTE_LITERAL: u32 = 0x0000_8000;
pub const PCRE2_SUBSTITUTE_MATCHED: u32 = 0x0001_0000;
pub const PCRE2_SUBSTITUTE_REPLACEMENT_ONLY: u32 = 0x0002_0000;
pub const PCRE2_DISABLE_RECURSELOOP_CHECK: u32 = 0x0004_0000;

/* ------------------------------------------------------------------ */
/* Pattern corpus                                                      */
/* ------------------------------------------------------------------ */

/// Patterns exercising as much of the compiler as possible: literals, escapes,
/// classes, extended classes, quantifiers, groups, back references, assertions,
/// conditionals, recursion, verbs, callouts, inline options and Unicode
/// properties. Many are deliberately invalid so that error codes and error
/// offsets get compared too.
pub fn patterns() -> Vec<&'static [u8]> {
    vec![
        /* --- trivial / literal --- */
        b"",
        b"a",
        b"abc",
        b"abcdefghijklmnopqrstuvwxyz",
        b"\x00",
        b"a\x00b",
        b"\xff",
        b"\x80\x81\x82",
        /* --- alternation --- */
        b"a|b",
        b"a|b|c|d",
        b"|",
        b"||",
        b"abc|abd|abe",
        b"(?:foo|foobar|foobaz)",
        /* --- quantifiers --- */
        b"a*",
        b"a+",
        b"a?",
        b"a*?",
        b"a+?",
        b"a??",
        b"a*+",
        b"a++",
        b"a?+",
        b"a{3}",
        b"a{3,}",
        b"a{3,5}",
        b"a{0,1}",
        b"a{0,0}",
        b"a{,5}",
        b"a{1,65535}",
        b"a{65535}",
        b"a{65536}",
        b"a{2,1}",
        b"(ab){2,4}",
        b"(?:ab)*",
        b"(a|b)+",
        b".*",
        b".+",
        b"[ab]{2,3}",
        b"\\d{1,3}",
        b"a{3,5}+",
        b"a{3,5}?",
        /* --- dot, anchors --- */
        b".",
        b"^abc$",
        b"^",
        b"$",
        b"\\A\\Z\\z",
        b"\\G",
        b"\\b\\B",
        b"\\Kabc",
        b"a\\Kb",
        b"^.*$",
        /* --- escapes --- */
        b"\\d\\D\\s\\S\\w\\W",
        b"\\h\\H\\v\\V",
        b"\\R",
        b"\\N",
        b"\\C",
        b"\\X",
        b"\\n\\r\\t\\f\\a\\e",
        b"\\0",
        b"\\00",
        b"\\000",
        b"\\07",
        b"\\o{101}",
        b"\\o{0}",
        b"\\o{}",
        b"\\x41",
        b"\\x{41}",
        b"\\x{10ffff}",
        b"\\x{110000}",
        b"\\x{}",
        b"\\x",
        b"\\cA",
        b"\\c[",
        b"\\c",
        b"\\Qa.b*c\\E",
        b"\\Qabc",
        b"\\E",
        b"\\Q\\E",
        b"a\\Q\\Eb",
        b"\\1",
        b"\\8",
        b"\\9",
        b"(a)\\1",
        b"(a)(b)\\2\\1",
        b"\\g1",
        b"\\g{1}",
        b"\\g{-1}",
        b"(a)\\g{-1}",
        b"\\g<1>",
        b"\\g'1'",
        b"\\k<n>",
        b"(?<n>a)\\k<n>",
        b"(?<n>a)\\k'n'",
        b"(?<n>a)\\k{n}",
        b"(?<n>a)(?P=n)",
        b"\\z\\Z",
        b"\\p{L}",
        b"\\p{Lu}",
        b"\\P{L}",
        b"\\pL",
        b"\\p{Greek}",
        b"\\p{Han}",
        b"\\p{Any}",
        b"\\p{Xan}",
        b"\\p{Xps}",
        b"\\p{Xsp}",
        b"\\p{Xuc}",
        b"\\p{Xwd}",
        b"\\p{Bidi_Control}",
        b"\\p{scx:Latin}",
        b"\\p{sc=Greek}",
        b"\\p{Nonsense}",
        b"\\p{",
        b"\\p",
        b"\\p{^L}",
        /* --- character classes --- */
        b"[a]",
        b"[abc]",
        b"[^abc]",
        b"[a-z]",
        b"[a-]",
        b"[-a]",
        b"[]]",
        b"[^]]",
        b"[]",
        b"[^]",
        b"[a-z0-9_]",
        b"[\\d]",
        b"[\\D]",
        b"[\\w\\s]",
        b"[^\\w]",
        b"[\\x00-\\xff]",
        b"[\\x{100}-\\x{200}]",
        b"[[:alpha:]]",
        b"[[:^alpha:]]",
        b"[[:digit:][:space:]]",
        b"[[:alnum:][:punct:][:xdigit:]]",
        b"[[:graph:][:print:]]",
        b"[[:blank:][:cntrl:][:lower:][:upper:][:word:]]",
        b"[[:bogus:]]",
        b"[a-\\d]",
        b"[\\p{L}]",
        b"[\\P{Nd}]",
        b"[^\\p{Greek}]",
        b"[z-a]",
        b"[\\Qab\\E]",
        b"[\\b]",
        b"[\\B]",
        b"[a[b]c]",
        b"[\\]]",
        b"[^\\x00-\\x{10ffff}]",
        b"[\\s\\S]",
        b"[0-9a-fA-F]",
        b"[\xc3\xa9]",
        /* --- extended (set-operation) classes --- */
        b"[[a-z]&&[^aeiou]]",
        b"[[a-z]--[aeiou]]",
        b"[[a-z]||[0-9]]",
        b"[[a-z]~~[a-c]]",
        b"[!\\p{L}]",
        b"[[:alpha:]&&[:^lower:]]",
        b"[[a-c][d-f]]",
        b"[[a-z]&&]",
        b"[\\p{L}&&\\p{Greek}]",
        b"[\\p{L}--\\p{Greek}]",
        b"[\\p{L}||\\p{Nd}]",
        b"[\\p{L}~~\\p{Greek}]",
        b"[[\\x{100}-\\x{200}]&&[\\x{150}-\\x{250}]]",
        b"[!\\p{L}&&\\p{Greek}]",
        b"[\\p{L}&&[^\\x{100}]]",
        b"[\\p{Greek}||[\\x{100}-\\x{110}]]",
        b"[\\p{L}&&\\p{Greek}&&\\p{Nd}]",
        b"[[\\x{100}-\\x{200}]||[a-z]]",
        b"[\\x{100}-\\x{200}]",
        b"[^\\x{100}]",
        b"[\\x{10000}-\\x{10ffff}]",
        /* --- groups --- */
        b"(a)",
        b"(?:a)",
        b"(?<name>a)",
        b"(?'name'a)",
        b"(?P<name>a)",
        b"(?<a>x)(?<b>y)",
        b"(?<n>a)|(?<n>b)",
        b"(?|(a)|(b))",
        b"(?>a*)b",
        b"(?i)abc",
        b"(?-i)abc",
        b"(?i:abc)",
        b"(?im-sx:a)",
        b"(?x) a b c # comment\n d",
        b"(?xx) a b [c d]",
        b"(?s).",
        b"(?m)^a$",
        b"(?J)(?<n>a)(?<n>b)",
        b"(?U)a*",
        b"(?n)(a)(b)",
        b"(?aD)\\d",
        b"(?aS)\\s",
        b"(?aW)\\w",
        b"(?aP)[[:alpha:]]",
        b"(?aT)\\d",
        b"(?r)abc",
        b"(?#comment)abc",
        b"(?#unterminated",
        b"(?C)a",
        b"(?C1)a",
        b"(?C255)a",
        b"(?C256)a",
        b"(?C{text})a",
        b"(?C\"text\")a",
        b"(?C'text')a",
        b"(?C`text`)a",
        b"(?C^text^)a",
        b"(?C%text%)a",
        b"(?C#text#)a",
        b"(?C$text$)a",
        b"(?C{unterminated",
        /* --- assertions --- */
        b"(?=a)",
        b"(?!a)",
        b"(?<=a)",
        b"(?<!a)",
        b"(?<=ab|cde)",
        b"(?<=a{2,4})",
        b"(?<=a*)",
        b"(?*a)",
        b"(?<*a)",
        b"(*positive_lookahead:a)",
        b"(*pla:a)",
        b"(*negative_lookahead:a)",
        b"(*nla:a)",
        b"(*positive_lookbehind:a)",
        b"(*plb:a)",
        b"(*negative_lookbehind:a)",
        b"(*nlb:a)",
        b"(*non_atomic_positive_lookahead:a)",
        b"(*napla:a)",
        b"(*non_atomic_positive_lookbehind:a)",
        b"(*naplb:a)",
        b"(*atomic:a)",
        b"(*script_run:abc)",
        b"(*sr:abc)",
        b"(*atomic_script_run:abc)",
        b"(*asr:abc)",
        b"(a)(*scan_substring:(1)x)",
        b"(a)(*scs:(1)x)",
        b"(a)(*scs:(1)b)",
        b"(?<n>a)(*scs:(<n>)b)",
        b"(a)(b)(*scs:(1,2)x)",
        /* --- conditionals --- */
        b"(a)(?(1)b|c)",
        b"(a)(?(1)b)",
        b"(?(?=a)b|c)",
        b"(?(?!a)b|c)",
        b"(?(DEFINE)(?<x>a))(?&x)",
        b"(?<n>a)(?(<n>)b|c)",
        b"(?<n>a)(?('n')b|c)",
        b"(?(R)a|b)",
        b"(?(R1)a|b)",
        b"(?<n>a)(?(R&n)a|b)",
        b"(?(VERSION>=10.0)a|b)",
        b"(?(VERSION=10.48)a|b)",
        b"(?(VERSION>=99.0)a|b)",
        b"(?(1)a|b)",
        b"(?(0)a|b)",
        b"(?(+1)a|b)(x)",
        b"(?(-1)a|b)",
        b"(x)(?(-1)a|b)",
        b"(?(1)a|b|c)",
        /* --- recursion / subroutines --- */
        b"(?R)",
        b"a(?R)?b",
        b"(a)(?1)",
        b"(a)(?-1)",
        b"(a)(?+1)(b)",
        b"(?<n>a)(?&n)",
        b"(?<n>a)(?P>n)",
        b"\\((?>[^()]|(?R))*\\)",
        b"(?(DEFINE)(?<word>\\w+))(?&word)\\s(?&word)",
        /* --- verbs --- */
        b"(*FAIL)",
        b"(*F)",
        b"(*ACCEPT)",
        b"(*COMMIT)",
        b"(*PRUNE)",
        b"(*SKIP)",
        b"(*THEN)",
        b"(*MARK:x)",
        b"(*:x)",
        b"(*COMMIT:x)",
        b"(*PRUNE:x)",
        b"(*SKIP:x)",
        b"(*THEN:x)",
        b"a(*THEN)b|c",
        b"(*ACCEPT:x)",
        b"(*UNKNOWNVERB)",
        b"(*LIMIT_MATCH=100)a",
        b"(*LIMIT_DEPTH=100)a",
        b"(*LIMIT_HEAP=100)a",
        b"(*CR)a",
        b"(*LF)a",
        b"(*CRLF)a",
        b"(*ANY)a",
        b"(*ANYCRLF)a",
        b"(*NUL)a",
        b"(*BSR_ANYCRLF)\\R",
        b"(*BSR_UNICODE)\\R",
        b"(*UTF)a",
        b"(*UCP)a",
        b"(*NOTEMPTY)a",
        b"(*NOTEMPTY_ATSTART)a",
        b"(*NO_AUTO_POSSESS)a*",
        b"(*NO_DOTSTAR_ANCHOR).*",
        b"(*NO_START_OPT)a",
        b"(*NO_JIT)a",
        b"(*CRLF)(*UTF)\xc3\xa9",
        /* --- auto-possessify triggers --- */
        b"\\d+abc",
        b"\\d+\\D",
        b"[a-z]+[0-9]",
        b"a*b",
        b"\\w+\\s",
        b"\\s*\\S",
        b"a++b",
        b".*\\d",
        b"\\p{L}+\\d",
        b"[^a]*a",
        b"\\D+\\d",
        b"\\H+\\h",
        b"\\V+\\v",
        b"x?\\d",
        b"\\d?+x",
        /* --- negated-class repeats (caseful and caseless OP_NOT* opcodes) --- */
        b"[^a]*a",
        b"[^a]+b",
        b"[^a]?b",
        b"[^a]{2}b",
        b"[^a]{2,4}b",
        b"[^a]*+b",
        b"[^a]++b",
        b"[^a]?+b",
        b"[^a]{2,4}+b",
        b"[^a]*?b",
        b"[^a]{2,4}?b",
        b"(?i)[^a]*a",
        b"(?i)[^a]+b",
        b"(?i)[^a]?b",
        b"(?i)[^a]{2}b",
        b"(?i)[^a]{2,4}b",
        b"(?i)[^a]*+b",
        b"(?i)[^K]+x",
        b"[^\\x{100}]*a",
        b"(?i)[^\\x{100}]{1,3}a",
        /* --- start optimisation / firstline / study --- */
        b"abc.*def",
        b"^(?:abc|abd)",
        b"[ab]cd",
        b"\\bword\\b",
        b"(?=abc)abcdef",
        b"a{10}b{10}",
        b"(a|bb|ccc)ddd",
        /* --- nesting / deep structures --- */
        b"((((((((((a))))))))))",
        b"(?:(?:(?:(?:(?:a)))))",
        b"(a(b(c(d(e)))))",
        b"[a](?:[b]|[c])+",
        /* --- error cases --- */
        b"(",
        b")",
        b"(?",
        b"(?<",
        b"(?<>a)",
        b"(?<1a>x)",
        b"(?<name)",
        b"[",
        b"[a",
        b"[a-",
        b"a{",
        b"a{1",
        b"*",
        b"+",
        b"?",
        b"{1}",
        b"a**",
        b"a*{2}",
        b"(?<n>a)(?<n>b)",
        b"\\",
        b"(?P",
        b"(?P<>a)",
        b"(?Z)",
        b"(?i-i)a",
        b"(?<=a+)",
        b"(?<=(?<=a))",
        b"(?(1)a)",
        b"(?(?<name>a)b)",
        b"\\g",
        b"\\g{",
        b"\\g{}",
        b"\\g<>",
        b"\\k",
        b"\\k<>",
        b"(?&)",
        b"(?&nonexistent)",
        b"a\\x{d800}",
        b"[\\x{d800}]",
        b"\\N{U+41}",
        b"\\N{name}",
        b"(?=a)*",
        b"(?<=a)*",
        b"()*",
        b"(){0}",
        b"(*MARK)",
        b"(*SKIP:)",
        b"(?J:(?<a>1)(?<a>2))",
        b"[[:alpha:",
        b"(?i)(?-i)(?i)a",
        b"a{1,2}{3,4}",
        /* --- longer patterns --- */
        b"^(?:[a-zA-Z0-9_.+-]+)@(?:[a-zA-Z0-9-]+\\.)+[a-zA-Z]{2,}$",
        b"(?<year>\\d{4})-(?<month>\\d{2})-(?<day>\\d{2})",
        b"\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b",
        b"(?x)\n  \\d+  # digits\n  \\s*  # space\n  \\w+  # word\n",
        b"(?:a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z)+",
        /* --- UTF-8 bytes in the pattern --- */
        b"\xc3\xa9",
        b"\xe6\x97\xa5\xe6\x9c\xac",
        b"[\xc3\xa9\xc3\xa8]",
        b"\xf0\x9f\x98\x80",
        b"\xc3",
        b"\xe6\x97",
        b"a\xffb",
        b"[\xc3\xa0-\xc3\xbf]",
        /* --- \R / newline handling --- */
        b"\\R+",
        b"\\R{2,3}",
        b"a\\Rb",
        /* --- misc --- */
        b"(?:)",
        b"(?:)*",
        b"(?:|)",
        b"(|a)",
        b"(a|)",
        b"[^\\n]",
        b"\\d++",
        b"(?>\\d+)",
        b"(?i)[a-z]",
        b"(?i)\xc3\xa9",
        b"(?i)[\xc3\xa9]",
        b"(?i)k",
        b"(?i)s",
        b"(?i)\\x{130}",
        b"(?i)\\x{131}",
        b"(?i)i",
        b"(?i)I",
    ]
}

/// The compile-option sets to try for every pattern.
pub fn compile_option_sets() -> Vec<u32> {
    vec![
        0,
        PCRE2_CASELESS,
        PCRE2_MULTILINE,
        PCRE2_DOTALL,
        PCRE2_EXTENDED,
        PCRE2_EXTENDED_MORE,
        PCRE2_UNGREEDY,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_ANCHORED | PCRE2_ENDANCHORED,
        PCRE2_NO_AUTO_CAPTURE,
        PCRE2_NO_AUTO_POSSESS,
        PCRE2_NO_START_OPTIMIZE,
        PCRE2_NO_DOTSTAR_ANCHOR,
        PCRE2_DUPNAMES,
        PCRE2_AUTO_CALLOUT,
        PCRE2_ALT_BSUX,
        PCRE2_ALT_CIRCUMFLEX,
        PCRE2_ALT_VERBNAMES,
        PCRE2_ALT_EXTENDED_CLASS,
        PCRE2_ALLOW_EMPTY_CLASS,
        PCRE2_MATCH_UNSET_BACKREF,
        PCRE2_DOLLAR_ENDONLY,
        PCRE2_FIRSTLINE,
        PCRE2_LITERAL,
        PCRE2_NEVER_UTF,
        PCRE2_NEVER_UCP,
        PCRE2_NEVER_BACKSLASH_C,
        PCRE2_UTF,
        PCRE2_UTF | PCRE2_NO_UTF_CHECK,
        PCRE2_UCP,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_UTF | PCRE2_MATCH_INVALID_UTF,
        PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS,
        PCRE2_UTF | PCRE2_CASELESS | PCRE2_MULTILINE | PCRE2_DOTALL,
        PCRE2_CASELESS | PCRE2_MULTILINE | PCRE2_DOTALL | PCRE2_EXTENDED,
        PCRE2_UCP | PCRE2_CASELESS,
        PCRE2_USE_OFFSET_LIMIT,
    ]
}

/// The extra-option sets to try.
pub fn extra_option_sets() -> Vec<u32> {
    vec![
        0,
        PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
        PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
        PCRE2_EXTRA_MATCH_WORD,
        PCRE2_EXTRA_MATCH_LINE,
        PCRE2_EXTRA_MATCH_WORD | PCRE2_EXTRA_MATCH_LINE,
        PCRE2_EXTRA_ESCAPED_CR_IS_LF,
        PCRE2_EXTRA_ALT_BSUX,
        PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        PCRE2_EXTRA_CASELESS_RESTRICT,
        PCRE2_EXTRA_ASCII_BSD,
        PCRE2_EXTRA_ASCII_BSS,
        PCRE2_EXTRA_ASCII_BSW,
        PCRE2_EXTRA_ASCII_POSIX,
        PCRE2_EXTRA_ASCII_DIGIT,
        PCRE2_EXTRA_ASCII_BSD
            | PCRE2_EXTRA_ASCII_BSS
            | PCRE2_EXTRA_ASCII_BSW
            | PCRE2_EXTRA_ASCII_POSIX
            | PCRE2_EXTRA_ASCII_DIGIT,
        PCRE2_EXTRA_PYTHON_OCTAL,
        PCRE2_EXTRA_NO_BS0,
        PCRE2_EXTRA_NEVER_CALLOUT,
        PCRE2_EXTRA_TURKISH_CASING,
    ]
}

/// Subjects used for match / dfa / substitute comparisons.
pub fn subjects() -> Vec<&'static [u8]> {
    vec![
        b"",
        b"a",
        b"A",
        b"abc",
        b"ABC",
        b"abcabc",
        b"aaa",
        b"aaaaaaaaaaaaaaaaaaaa",
        b"xyz",
        b"ab",
        b"abd",
        b"a\nb",
        b"a\r\nb",
        b"a\rb",
        b"\n",
        b"\r\n",
        b"\x0b\x0c\x85",
        b"123",
        b"a1b2c3",
        b"  spaced  out  ",
        b"the quick brown fox",
        b"foo@bar.example.com",
        b"2024-01-31",
        b"192.168.0.1",
        b"(nested (parens) here)",
        b"((()))",
        b"word another",
        b"\xc3\xa9",
        b"caf\xc3\xa9",
        b"\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e",
        b"\xf0\x9f\x98\x80!",
        b"e\xcc\x81",
        b"\xff\xfe",
        b"\xc3",
        b"a\xffb",
        b"\x00\x01\x02",
        b"a\x00b",
        b"MARKED",
        b"aeiou",
        b"AEIOU",
        b"0123456789abcdefABCDEF",
        b"\t\x0b\x0c \xc2\xa0",
        b"\xe2\x80\xa8\xe2\x80\xa9",
        b"stra\xc3\x9fe",
        b"\xc4\xb0\xc4\xb1ii",
        b"kelvin \xe2\x84\xaa",
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
    ]
}
