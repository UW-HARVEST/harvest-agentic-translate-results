//! Phase C, part 1: one differential test case per row of `ERRORS.md` part 1
//! (the 121 compile-time error codes ERR0..ERR120).
//!
//! The row data lives in `_v/err_compile.tsv` (7 tab-separated fields:
//! errname, numeric code, c_file:line, description, pattern, options, extra options).
//! For every row we
//!   * build the exact invalid input (plus the compile-context setting named by a
//!     `REQUIRES-CONTEXT:` description),
//!   * call `pcre2_compile_8` in BOTH the C and the Rust `.so`,
//!   * assert both fail, with the SAME `errorcode` AND the same `erroroffset`,
//!   * assert that error code is the one the table says the C produces,
//!   * and cross-check `pcre2_get_error_message_8` for that code in both libraries.
mod common;
use common::*;
use std::ffi::c_void;
use std::ptr;

fn tsv() -> String {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("_v/err_compile.tsv");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {:?}: {}", p, e))
}

fn opt(name: &str) -> u32 {
    match name.trim() {
        "0" => 0,
        "PCRE2_ANCHORED" => PCRE2_ANCHORED,
        "PCRE2_NO_UTF_CHECK" => PCRE2_NO_UTF_CHECK,
        "PCRE2_ENDANCHORED" => PCRE2_ENDANCHORED,
        "PCRE2_ALLOW_EMPTY_CLASS" => PCRE2_ALLOW_EMPTY_CLASS,
        "PCRE2_ALT_BSUX" => PCRE2_ALT_BSUX,
        "PCRE2_AUTO_CALLOUT" => PCRE2_AUTO_CALLOUT,
        "PCRE2_CASELESS" => PCRE2_CASELESS,
        "PCRE2_DOLLAR_ENDONLY" => PCRE2_DOLLAR_ENDONLY,
        "PCRE2_DOTALL" => PCRE2_DOTALL,
        "PCRE2_DUPNAMES" => PCRE2_DUPNAMES,
        "PCRE2_EXTENDED" => PCRE2_EXTENDED,
        "PCRE2_FIRSTLINE" => PCRE2_FIRSTLINE,
        "PCRE2_MATCH_UNSET_BACKREF" => PCRE2_MATCH_UNSET_BACKREF,
        "PCRE2_MULTILINE" => PCRE2_MULTILINE,
        "PCRE2_NEVER_UCP" => PCRE2_NEVER_UCP,
        "PCRE2_NEVER_UTF" => PCRE2_NEVER_UTF,
        "PCRE2_NO_AUTO_CAPTURE" => PCRE2_NO_AUTO_CAPTURE,
        "PCRE2_NO_AUTO_POSSESS" => PCRE2_NO_AUTO_POSSESS,
        "PCRE2_NO_DOTSTAR_ANCHOR" => PCRE2_NO_DOTSTAR_ANCHOR,
        "PCRE2_NO_START_OPTIMIZE" => PCRE2_NO_START_OPTIMIZE,
        "PCRE2_UCP" => PCRE2_UCP,
        "PCRE2_UNGREEDY" => PCRE2_UNGREEDY,
        "PCRE2_UTF" => PCRE2_UTF,
        "PCRE2_NEVER_BACKSLASH_C" => PCRE2_NEVER_BACKSLASH_C,
        "PCRE2_ALT_CIRCUMFLEX" => PCRE2_ALT_CIRCUMFLEX,
        "PCRE2_ALT_VERBNAMES" => PCRE2_ALT_VERBNAMES,
        "PCRE2_USE_OFFSET_LIMIT" => PCRE2_USE_OFFSET_LIMIT,
        "PCRE2_EXTENDED_MORE" => PCRE2_EXTENDED_MORE,
        "PCRE2_LITERAL" => PCRE2_LITERAL,
        "PCRE2_MATCH_INVALID_UTF" => PCRE2_MATCH_INVALID_UTF,
        "PCRE2_ALT_EXTENDED_CLASS" => PCRE2_ALT_EXTENDED_CLASS,
        "PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES" => PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
        "PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL" => PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
        "PCRE2_EXTRA_MATCH_WORD" => PCRE2_EXTRA_MATCH_WORD,
        "PCRE2_EXTRA_MATCH_LINE" => PCRE2_EXTRA_MATCH_LINE,
        "PCRE2_EXTRA_ESCAPED_CR_IS_LF" => PCRE2_EXTRA_ESCAPED_CR_IS_LF,
        "PCRE2_EXTRA_ALT_BSUX" => PCRE2_EXTRA_ALT_BSUX,
        "PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK" => PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        "PCRE2_EXTRA_CASELESS_RESTRICT" => PCRE2_EXTRA_CASELESS_RESTRICT,
        "PCRE2_EXTRA_ASCII_BSD" => PCRE2_EXTRA_ASCII_BSD,
        "PCRE2_EXTRA_ASCII_BSS" => PCRE2_EXTRA_ASCII_BSS,
        "PCRE2_EXTRA_ASCII_BSW" => PCRE2_EXTRA_ASCII_BSW,
        "PCRE2_EXTRA_ASCII_POSIX" => PCRE2_EXTRA_ASCII_POSIX,
        "PCRE2_EXTRA_ASCII_DIGIT" => PCRE2_EXTRA_ASCII_DIGIT,
        "PCRE2_EXTRA_PYTHON_OCTAL" => PCRE2_EXTRA_PYTHON_OCTAL,
        "PCRE2_EXTRA_NO_BS0" => PCRE2_EXTRA_NO_BS0,
        "PCRE2_EXTRA_NEVER_CALLOUT" => PCRE2_EXTRA_NEVER_CALLOUT,
        "PCRE2_EXTRA_TURKISH_CASING" => PCRE2_EXTRA_TURKISH_CASING,
        other => {
            if let Some(h) = other.strip_prefix("0x") {
                u32::from_str_radix(h, 16).unwrap_or_else(|_| panic!("bad option {:?}", other))
            } else {
                other
                    .parse()
                    .unwrap_or_else(|_| panic!("unknown option {:?}", other))
            }
        }
    }
}

fn opts(expr: &str) -> u32 {
    expr.split('|').filter(|s| !s.trim().is_empty()).map(opt).fold(0, |a, b| a | b)
}

/// Decode the body of a Rust byte-string literal.
fn unescape(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            match b[i + 1] {
                b'x' => {
                    let h = std::str::from_utf8(&b[i + 2..i + 4]).unwrap();
                    out.push(u8::from_str_radix(h, 16).unwrap());
                    i += 4;
                }
                b'n' => {
                    out.push(b'\n');
                    i += 2;
                }
                b'r' => {
                    out.push(b'\r');
                    i += 2;
                }
                b't' => {
                    out.push(b'\t');
                    i += 2;
                }
                b'0' => {
                    out.push(0);
                    i += 2;
                }
                c => {
                    out.push(c);
                    i += 2;
                }
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

// -------- allocator that starts failing after a budget, for the ERR21 row
static mut BUDGET: i64 = 0;
unsafe extern "C" fn counting_malloc(size: usize, _d: Ptr) -> Ptr {
    let b = BUDGET;
    BUDGET = b - 1;
    if b <= 0 {
        ptr::null_mut()
    } else {
        // std allocator via libc malloc semantics
        let layout = std::alloc::Layout::from_size_align(size.max(1) + 16, 16).unwrap();
        let p = std::alloc::alloc(layout) as *mut usize;
        if p.is_null() {
            return ptr::null_mut();
        }
        *p = size.max(1) + 16;
        (p as *mut u8).add(16) as Ptr
    }
}
unsafe extern "C" fn counting_free(p: Ptr, _d: Ptr) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(16);
    let size = *(base as *mut usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(size, 16).unwrap());
}
unsafe extern "C" fn always_guard(_depth: u32, _d: Ptr) -> i32 {
    1
}

struct Row {
    name: String,
    code: i32,
    site: String,
    desc: String,
    pat: Vec<u8>,
    unreachable: bool,
    options: u32,
    xoptions: u32,
}

fn rows() -> Vec<Row> {
    tsv()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert_eq!(f.len(), 7, "bad row {:?}", &l[..40.min(l.len())]);
            Row {
                name: f[0].to_string(),
                code: f[1].parse().unwrap(),
                site: f[2].to_string(),
                desc: f[3].to_string(),
                unreachable: f[4] == "UNREACHABLE",
                pat: unescape(f[4]),
                options: opts(f[5]),
                xoptions: opts(f[6]),
            }
        })
        .collect()
}

/// Run one row against one library, returning (compiled_ok, errorcode, erroroffset).
unsafe fn run(api: &Api, r: &Row) -> (bool, i32, usize) {
    let mut gcontext: Ptr = ptr::null_mut();
    if r.name == "ERR21" {
        BUDGET = 2;
        gcontext = (api.pcre2_general_context_create_8)(
            Some(counting_malloc),
            Some(counting_free),
            ptr::null_mut(),
        );
    }
    let cc = (api.pcre2_compile_context_create_8)(gcontext);
    assert!(!cc.is_null(), "{}: compile context create failed", api.tag);
    if r.xoptions != 0 {
        (api.pcre2_set_compile_extra_options_8)(cc, r.xoptions);
    }
    let mut patptr: Sptr = r.pat.as_ptr();
    let mut patlen = r.pat.len();
    match r.name.as_str() {
        "ERR16" => {
            patptr = ptr::null();
            patlen = 5;
        }
        "ERR33" => {
            (api.pcre2_set_compile_recursion_guard_8)(cc, Some(always_guard), ptr::null_mut());
        }
        "ERR84" | "ERR86" => {
            (api.pcre2_set_parens_nest_limit_8)(cc, 5000);
        }
        "ERR88" => {
            (api.pcre2_set_max_pattern_length_8)(cc, 4);
        }
        "ERR101" => {
            (api.pcre2_set_max_pattern_compiled_length_8)(cc, 8);
        }
        _ => {}
    }
    let mut ec: i32 = -12345;
    let mut eo: usize = usize::MAX;
    let eo_ptr: *mut usize = if r.name == "ERR120" {
        ptr::null_mut()
    } else {
        &mut eo
    };
    let code = (api.pcre2_compile_8)(patptr, patlen, r.options, &mut ec, eo_ptr, cc);
    let ok = !code.is_null();
    if ok {
        (api.pcre2_code_free_8)(code);
    }
    (api.pcre2_compile_context_free_8)(cc);
    if !gcontext.is_null() {
        (api.pcre2_general_context_free_8)(gcontext);
    }
    (ok, ec, eo)
}

#[test]
fn every_compile_error_row_matches() {
    let (c, r) = both();
    let all = rows();
    assert_eq!(all.len(), 121, "expected ERR0..ERR120");
    let mut tested = 0;
    let mut skipped = 0;
    for row in &all {
        if row.unreachable {
            skipped += 1;
            continue;
        }
        let cres = unsafe { run(c, row) };
        let rres = unsafe { run(r, row) };
        assert!(
            !cres.0,
            "{} ({}): C compiled the pattern successfully — row is stale",
            row.name, row.site
        );
        assert_eq!(
            cres.1, row.code,
            "{} ({}): C returned {} not the tabulated {} — {}",
            row.name, row.site, cres.1, row.code, row.desc
        );
        assert_eq!(
            cres, rres,
            "\nROW {} ({}) {}\n  pattern  = {:?}\n  options  = {:#x} extra = {:#x}\n  C    (ok,code,offset) = {:?}\n  RUST (ok,code,offset) = {:?}",
            row.name,
            row.site,
            row.desc,
            String::from_utf8_lossy(&row.pat[..row.pat.len().min(120)]),
            row.options,
            row.xoptions,
            cres,
            rres
        );
        tested += 1;
    }
    println!("compile-error rows: {} tested, {} unreachable", tested, skipped);
    assert_eq!(tested + skipped, 121);
}

#[test]
fn error_messages_match_for_every_code() {
    // pcre2_get_error_message for every compile code and every runtime code,
    // plus out-of-range codes on both sides of the valid ranges.
    let (c, r) = both();
    unsafe {
        for code in -200i32..=350 {
            let mut b1 = [0u8; 256];
            let mut b2 = [0u8; 256];
            let n1 = (c.pcre2_get_error_message_8)(code, b1.as_mut_ptr(), b1.len());
            let n2 = (r.pcre2_get_error_message_8)(code, b2.as_mut_ptr(), b2.len());
            assert_eq!((n1, b1), (n2, b2), "get_error_message({})", code);
        }
        // buffer-too-small behaviour for every size from 0 to 40
        for size in 0..40usize {
            let mut b1 = [0xAAu8; 64];
            let mut b2 = [0xAAu8; 64];
            let n1 = (c.pcre2_get_error_message_8)(-47, b1.as_mut_ptr(), size);
            let n2 = (r.pcre2_get_error_message_8)(-47, b2.as_mut_ptr(), size);
            assert_eq!((n1, b1), (n2, b2), "get_error_message(-47, size={})", size);
        }
    }
}

#[test]
fn compile_rejects_out_of_range_and_null_arguments_identically() {
    let (c, r) = both();
    unsafe {
        // Every single undefined option bit, one at a time, plus the all-ones word.
        for bit in 0..32u32 {
            let o = 1u32 << bit;
            let a = c.compile_probe(b"a", 1, o, ptr::null_mut());
            let b = r.compile_probe(b"a", 1, o, ptr::null_mut());
            assert_eq!(a, b, "compile option bit {:#x}", o);
        }
        let a = c.compile_probe(b"a", 1, u32::MAX, ptr::null_mut());
        let b = r.compile_probe(b"a", 1, u32::MAX, ptr::null_mut());
        assert_eq!(a, b, "compile options = 0xffffffff");

        // NULL pattern with a zero length, and PCRE2_ZERO_TERMINATED on an empty string.
        for (p, l) in [
            (ptr::null::<u8>(), 0usize),
            (ptr::null::<u8>(), PCRE2_ZERO_TERMINATED),
            (b"\0".as_ptr(), PCRE2_ZERO_TERMINATED),
        ] {
            let mut e1 = 0;
            let mut o1 = 0;
            let mut e2 = 0;
            let mut o2 = 0;
            let c1 = (c.pcre2_compile_8)(p, l, 0, &mut e1, &mut o1, ptr::null_mut());
            let c2 = (r.pcre2_compile_8)(p, l, 0, &mut e2, &mut o2, ptr::null_mut());
            assert_eq!(
                (c1.is_null(), e1, o1),
                (c2.is_null(), e2, o2),
                "compile(ptr={:?}, len={})",
                p,
                l as i64
            );
            if !c1.is_null() {
                (c.pcre2_code_free_8)(c1);
            }
            if !c2.is_null() {
                (r.pcre2_code_free_8)(c2);
            }
        }
        // NULL errorcode pointer is legal (documented); check both survive it.
        let c1 = (c.pcre2_compile_8)(
            b"a".as_ptr(),
            1,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        let c2 = (r.pcre2_compile_8)(
            b"a".as_ptr(),
            1,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        assert_eq!(c1.is_null(), c2.is_null());
        if !c1.is_null() {
            (c.pcre2_code_free_8)(c1);
        }
        if !c2.is_null() {
            (r.pcre2_code_free_8)(c2);
        }
        let _ = std::hint::black_box(0 as *mut c_void);
    }
}
