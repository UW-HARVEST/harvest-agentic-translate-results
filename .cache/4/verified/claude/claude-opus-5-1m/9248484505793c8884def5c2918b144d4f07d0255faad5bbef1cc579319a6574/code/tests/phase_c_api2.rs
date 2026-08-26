// Phase C (error paths) — ERRORS.md rows 495..542.
//
//   pcre2_convert.c                    495-516
//   pcre2_valid_utf.c                  517-537
//   Other files — no rejection paths   538-542
//
// Every case constructs the EXACT invalid input the row names, calls the
// function in BOTH shared libraries, and compares the numeric result plus every
// out-parameter and the full contents of every caller-supplied buffer.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ==================================================================== allocators

// A fallible allocator with a SEPARATE budget/counter per library. It is built
// on the process's real malloc/free so that it stays interchangeable with the
// libraries' own defaults.
static mut BUDGET: [i64; 2] = [-1, -1];
static mut NMALLOC: [i64; 2] = [0, 0];
static mut NFREE: [i64; 2] = [0, 0];

type LibcMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
type LibcFree = unsafe extern "C" fn(*mut c_void);
static mut LIBC: Option<(LibcMalloc, LibcFree)> = None;
static LIBC_ONCE: std::sync::Once = std::sync::Once::new();

unsafe fn libc_fns() -> (LibcMalloc, LibcFree) {
    LIBC_ONCE.call_once(|| {
        use libloading::os::unix::{Library, Symbol, RTLD_NOW};
        let me: &'static Library =
            Box::leak(Box::new(Library::open(None::<&std::path::Path>, RTLD_NOW).unwrap()));
        let m: Symbol<LibcMalloc> = me.get(b"malloc\0").unwrap();
        let f: Symbol<LibcFree> = me.get(b"free\0").unwrap();
        LIBC = Some((*m, *f));
    });
    (*ptr::addr_of!(LIBC)).unwrap()
}

unsafe fn fallible(idx: usize, n: usize) -> *mut c_void {
    (*ptr::addr_of_mut!(NMALLOC))[idx] += 1;
    let b = &mut (*ptr::addr_of_mut!(BUDGET))[idx];
    if *b == 0 {
        return ptr::null_mut();
    }
    if *b > 0 {
        *b -= 1;
    }
    libc_fns().0(n.max(1))
}
unsafe extern "C" fn mal_c(n: usize, _d: *mut c_void) -> *mut c_void {
    fallible(0, n)
}
unsafe extern "C" fn mal_r(n: usize, _d: *mut c_void) -> *mut c_void {
    fallible(1, n)
}
unsafe extern "C" fn fre_c(p: *mut c_void, _d: *mut c_void) {
    (*ptr::addr_of_mut!(NFREE))[0] += 1;
    libc_fns().1(p)
}
unsafe extern "C" fn fre_r(p: *mut c_void, _d: *mut c_void) {
    (*ptr::addr_of_mut!(NFREE))[1] += 1;
    libc_fns().1(p)
}
fn allocs(idx: usize) -> (MallocFn, FreeFn) {
    if idx == 0 {
        (mal_c, fre_c)
    } else {
        (mal_r, fre_r)
    }
}
unsafe fn reset(idx: usize) {
    (*ptr::addr_of_mut!(BUDGET))[idx] = -1;
    (*ptr::addr_of_mut!(NMALLOC))[idx] = 0;
    (*ptr::addr_of_mut!(NFREE))[idx] = 0;
}
unsafe fn set_budget(idx: usize, n: i64) {
    (*ptr::addr_of_mut!(BUDGET))[idx] = n;
}
unsafe fn nfree(idx: usize) -> i64 {
    (*ptr::addr_of_mut!(NFREE))[idx]
}

// ======================================================================= helpers

unsafe fn must_compile(api: &Api, pat: &[u8], opts: u32) -> Ptr {
    let mut e: c_int = 0;
    let mut off: Sz = 0;
    let c = (api.compile)(pat.as_ptr(), pat.len(), opts, &mut e, &mut off, ptr::null_mut());
    assert!(
        !c.is_null(),
        "[{}] compile {} failed: err {} at offset {}",
        api.name,
        show(pat),
        e,
        off
    );
    c
}

fn rc_of(s: &str) -> i64 {
    let t = s.split("rc=").nth(1).expect("no rc= in observation");
    t.split(|c: char| !(c.is_ascii_digit() || c == '-'))
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

// =============================================================== pcre2_convert.c

#[derive(Copy, Clone, Debug)]
enum PLen {
    /// the true length of the pattern bytes
    Actual,
    /// PCRE2_ZERO_TERMINATED (the pattern literal must end with a NUL)
    Zeroterm,
    /// a caller-declared length, which may be shorter than the bytes given
    Exact(usize),
}

#[derive(Copy, Clone, Debug)]
enum CMode {
    /// `buffptr == NULL`: only the required length is computed
    LenOnly,
    /// `buffptr != NULL`, `*buffptr == NULL`: two passes, output allocated
    Alloc,
    /// `buffptr != NULL`, `*buffptr` = a caller buffer, `*bufflenptr` = n
    Given(usize),
    /// `bufflenptr == NULL`
    NoBufflen,
}

struct ConvCase {
    rows: &'static [u32],
    /// `None` means a NULL `pattern` argument
    pat: Option<&'static [u8]>,
    plen: PLen,
    opts: u32,
    mode: CMode,
    sep: Option<u32>,
    esc: Option<u32>,
    /// what ERRORS.md documents the C returns
    expect: c_int,
    /// what ERRORS.md documents `*bufflenptr` becomes (None = compare C/rust only)
    expect_len: Option<Sz>,
}

const CONV_BUF: usize = 96;
/// Bytes appended after every pattern so that a converter which reads past the
/// declared `plength` (legal for it to do with PCRE2_CONVERT_NO_UTF_CHECK) stays
/// inside memory the test owns, and reads the SAME bytes in both libraries.
const CONV_PAD: [u8; 8] = [0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41];

const GLOB: u32 = PCRE2_CONVERT_GLOB;
const PBAS: u32 = PCRE2_CONVERT_POSIX_BASIC;
const PEXT: u32 = PCRE2_CONVERT_POSIX_EXTENDED;
const CUTF: u32 = PCRE2_CONVERT_UTF;
const CNOC: u32 = PCRE2_CONVERT_NO_UTF_CHECK;

const CONV_CASES: &[ConvCase] = &[
    // ---- 495: pattern == NULL with a non-zero length -------------------------
    ConvCase { rows: &[495], pat: None, plen: PLen::Exact(1), opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_NULL,
               expect_len: Some(0) },
    ConvCase { rows: &[495], pat: None, plen: PLen::Zeroterm, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_NULL,
               expect_len: Some(0) },
    ConvCase { rows: &[495], pat: None, plen: PLen::Exact(99), opts: PBAS,
               mode: CMode::LenOnly, sep: None, esc: None, expect: PCRE2_ERROR_NULL,
               expect_len: Some(0) },
    // pattern == NULL with plength == 0 is LEGAL (an internal 1-byte stand-in)
    ConvCase { rows: &[495], pat: None, plen: PLen::Exact(0), opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: None },
    ConvCase { rows: &[495], pat: None, plen: PLen::Exact(0), opts: PBAS,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: None },

    // ---- 496: bufflenptr == NULL --------------------------------------------
    ConvCase { rows: &[496], pat: Some(b"a*"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::NoBufflen, sep: None, esc: None, expect: PCRE2_ERROR_NULL,
               expect_len: None },
    ConvCase { rows: &[496], pat: None, plen: PLen::Exact(0), opts: GLOB,
               mode: CMode::NoBufflen, sep: None, esc: None, expect: PCRE2_ERROR_NULL,
               expect_len: None },

    // ---- 497: a bit outside ALL_OPTIONS (0x7F) ------------------------------
    ConvCase { rows: &[497], pat: Some(b"a"), plen: PLen::Actual, opts: GLOB | 0x80,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[497], pat: Some(b"a"), plen: PLen::Actual, opts: GLOB | 0x8000_0000,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[497], pat: Some(b"a"), plen: PLen::Actual, opts: 0xFFFF_FFFF,
               mode: CMode::LenOnly, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[497], pat: Some(b"a"), plen: PLen::Actual, opts: PBAS | 0x80,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    // 0x40 IS inside ALL_OPTIONS (it is part of PCRE2_CONVERT_GLOB_NO_STARSTAR
    // == 0x50), so PBAS|0x40 is accepted — the undefined-bit test is `& ~0x7F`.
    ConvCase { rows: &[497], pat: Some(b"a"), plen: PLen::Actual, opts: PBAS | 0x40,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: None },

    // ---- 498: more than one type bit in TYPE_OPTIONS (0x1C) -----------------
    ConvCase { rows: &[498], pat: Some(b"a"), plen: PLen::Actual, opts: PBAS | PEXT,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[498], pat: Some(b"a"), plen: PLen::Actual, opts: GLOB | PBAS,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[498], pat: Some(b"a"), plen: PLen::Actual, opts: GLOB | PEXT,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[498], pat: Some(b"a"), plen: PLen::Actual,
               opts: GLOB | PBAS | PEXT, mode: CMode::LenOnly, sep: None, esc: None,
               expect: PCRE2_ERROR_BADOPTION, expect_len: Some(0) },

    // ---- 499: no type bit at all -------------------------------------------
    ConvCase { rows: &[499], pat: Some(b"a"), plen: PLen::Actual, opts: 0,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[499], pat: Some(b"a"), plen: PLen::Actual, opts: CUTF,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[499], pat: Some(b"a"), plen: PLen::Actual, opts: CNOC,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[499], pat: Some(b"a"), plen: PLen::Actual, opts: CUTF | CNOC,
               mode: CMode::LenOnly, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },
    ConvCase { rows: &[499], pat: Some(b"a"), plen: PLen::Actual, opts: 0x20,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_BADOPTION,
               expect_len: Some(0) },

    // ---- 500: PCRE2_CONVERT_UTF and an invalid UTF-8 pattern ----------------
    ConvCase { rows: &[500], pat: Some(b"\xFF"), plen: PLen::Actual, opts: GLOB | CUTF,
               mode: CMode::Alloc, sep: None, esc: None, expect: -23, expect_len: Some(0) },
    ConvCase { rows: &[500], pat: Some(b"a\xC2"), plen: PLen::Actual, opts: GLOB | CUTF,
               mode: CMode::Alloc, sep: None, esc: None, expect: -3, expect_len: Some(1) },
    ConvCase { rows: &[500], pat: Some(b"ab\x80"), plen: PLen::Actual, opts: GLOB | CUTF,
               mode: CMode::LenOnly, sep: None, esc: None, expect: -22, expect_len: Some(2) },
    ConvCase { rows: &[500], pat: Some(b"a\xED\xA0\x80"), plen: PLen::Actual,
               opts: PBAS | CUTF, mode: CMode::Alloc, sep: None, esc: None, expect: -16,
               expect_len: Some(1) },
    // ... and the same inputs WITH PCRE2_CONVERT_NO_UTF_CHECK: the check is
    // skipped entirely, so the bytes are converted verbatim. (The glob main loop
    // walks byte by byte, so this reads nothing beyond the pattern.)
    ConvCase { rows: &[500], pat: Some(b"\xFF"), plen: PLen::Actual,
               opts: GLOB | CUTF | CNOC, mode: CMode::Alloc, sep: None, esc: None,
               expect: 0, expect_len: None },
    ConvCase { rows: &[500], pat: Some(b"a\xC2"), plen: PLen::Actual,
               opts: GLOB | CUTF | CNOC, mode: CMode::Alloc, sep: None, esc: None,
               expect: 0, expect_len: None },
    ConvCase { rows: &[500], pat: Some(b"ab\x80"), plen: PLen::Actual,
               opts: GLOB | CUTF | CNOC, mode: CMode::LenOnly, sep: None, esc: None,
               expect: 0, expect_len: None },
    // valid UTF-8 with PCRE2_CONVERT_UTF passes the check
    ConvCase { rows: &[500], pat: Some("a\u{e9}\u{1f600}".as_bytes()), plen: PLen::Actual,
               opts: GLOB | CUTF, mode: CMode::Alloc, sep: None, esc: None, expect: 0,
               expect_len: None },

    // ---- 503: POSIX pattern ending in a lone backslash ---------------------
    ConvCase { rows: &[503], pat: Some(b"a\\"), plen: PLen::Actual, opts: PBAS,
               mode: CMode::Alloc, sep: None, esc: None, expect: 101, expect_len: Some(2) },
    ConvCase { rows: &[503], pat: Some(b"a\\"), plen: PLen::Actual, opts: PEXT,
               mode: CMode::Alloc, sep: None, esc: None, expect: 101, expect_len: Some(2) },
    ConvCase { rows: &[503], pat: Some(b"\\"), plen: PLen::Actual, opts: PBAS,
               mode: CMode::LenOnly, sep: None, esc: None, expect: 101, expect_len: Some(1) },
    ConvCase { rows: &[503], pat: Some(b"abc\\"), plen: PLen::Actual, opts: PEXT,
               mode: CMode::Given(64), sep: None, esc: None, expect: 101, expect_len: Some(4) },

    // ---- 504: POSIX unterminated character class ---------------------------
    ConvCase { rows: &[504], pat: Some(b"[abc"), plen: PLen::Actual, opts: PBAS,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(4) },
    ConvCase { rows: &[504], pat: Some(b"["), plen: PLen::Actual, opts: PBAS,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(1) },
    ConvCase { rows: &[504], pat: Some(b"[[:alpha:"), plen: PLen::Actual, opts: PEXT,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(9) },
    ConvCase { rows: &[504], pat: Some(b"[^"), plen: PLen::Actual, opts: PEXT,
               mode: CMode::LenOnly, sep: None, esc: None, expect: 106, expect_len: Some(2) },

    // ---- 505: POSIX with a caller buffer that is too small -----------------
    ConvCase { rows: &[505], pat: Some(b"abc"), plen: PLen::Actual, opts: PBAS,
               mode: CMode::Given(1), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(3) },
    ConvCase { rows: &[505], pat: Some(b"abc"), plen: PLen::Actual, opts: PBAS,
               mode: CMode::Given(5), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(3) },
    ConvCase { rows: &[505], pat: Some(b"abc"), plen: PLen::Actual, opts: PBAS,
               mode: CMode::Given(9), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(3) },
    ConvCase { rows: &[505], pat: Some(b"abc"), plen: PLen::Actual, opts: PBAS,
               mode: CMode::Given(10), sep: None, esc: None, expect: 0, expect_len: Some(9) },
    ConvCase { rows: &[505], pat: Some(b"a.c*[x]$"), plen: PLen::Actual, opts: PEXT,
               mode: CMode::Given(1), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(8) },
    ConvCase { rows: &[505], pat: Some(b"a.c*[x]$"), plen: PLen::Actual, opts: PEXT,
               mode: CMode::Given(12), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(8) },
    ConvCase { rows: &[505], pat: Some(b"a.c*[x]$"), plen: PLen::Actual, opts: PEXT,
               mode: CMode::Given(64), sep: None, esc: None, expect: 0, expect_len: None },
    ConvCase { rows: &[505], pat: Some(b"a\\(b\\)c"), plen: PLen::Actual, opts: PBAS,
               mode: CMode::Given(3), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(7) },

    // ---- 506: GLOB with '[' as the final character -------------------------
    ConvCase { rows: &[506], pat: Some(b"["), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(1) },
    ConvCase { rows: &[506], pat: Some(b"a["), plen: PLen::Actual, opts: GLOB,
               mode: CMode::LenOnly, sep: None, esc: None, expect: 106, expect_len: Some(2) },

    // ---- 507: GLOB ending right after a class negator ----------------------
    ConvCase { rows: &[507], pat: Some(b"[!"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(2) },
    ConvCase { rows: &[507], pat: Some(b"[^"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(2) },

    // ---- 508: GLOB unterminated bracket expression -------------------------
    ConvCase { rows: &[508], pat: Some(b"[abc"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(4) },
    ConvCase { rows: &[508], pat: Some(b"[a-"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(3) },
    ConvCase { rows: &[508], pat: Some(b"[a\\"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(3) },
    ConvCase { rows: &[508], pat: Some(b"[!abc"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::LenOnly, sep: None, esc: None, expect: 106, expect_len: Some(5) },
    ConvCase { rows: &[508], pat: Some(b"[]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(2) },

    // ---- 509: GLOB POSIX class as the upper end of a range ------------------
    ConvCase { rows: &[509], pat: Some(b"[a-[:digit:]]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_CONVERT_SYNTAX,
               expect_len: Some(4) },
    ConvCase { rows: &[509], pat: Some(b"[x-[:alpha:]]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::LenOnly, sep: None, esc: None, expect: PCRE2_ERROR_CONVERT_SYNTAX,
               expect_len: Some(4) },

    // ---- 510: GLOB out-of-order range --------------------------------------
    ConvCase { rows: &[510], pat: Some(b"[z-a]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_CONVERT_SYNTAX,
               expect_len: Some(4) },
    ConvCase { rows: &[510], pat: Some(b"[9-0]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::LenOnly, sep: None, esc: None, expect: PCRE2_ERROR_CONVERT_SYNTAX,
               expect_len: Some(4) },

    // ---- 511: GLOB ending in the escape character --------------------------
    ConvCase { rows: &[511], pat: Some(b"a\\"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_CONVERT_SYNTAX,
               expect_len: Some(2) },
    ConvCase { rows: &[511], pat: Some(b"\\"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: PCRE2_ERROR_CONVERT_SYNTAX,
               expect_len: Some(1) },
    // with escaping disabled (escape == 0) a trailing backslash is a literal
    ConvCase { rows: &[511], pat: Some(b"a\\"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: Some(0), expect: 0, expect_len: None },
    // with a different escape character it is the new one that must not dangle
    ConvCase { rows: &[511], pat: Some(b"a!"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: Some(33),
               expect: PCRE2_ERROR_CONVERT_SYNTAX, expect_len: Some(2) },

    // ---- 512: GLOB with a caller buffer that is too small ------------------
    ConvCase { rows: &[512], pat: Some(b"a"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Given(1), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(1) },
    ConvCase { rows: &[512], pat: Some(b"a"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Given(8), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(1) },
    // "(?s)\Aa\z" is 8 code units, and one more is needed for the terminating
    // NUL that convert_glob writes, so 9 still fails and 10 is the first size
    // that works. Note *bufflenptr reports 9 = the length WITHOUT the NUL.
    ConvCase { rows: &[512], pat: Some(b"a"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Given(9), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(1) },
    ConvCase { rows: &[512], pat: Some(b"a"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Given(10), sep: None, esc: None, expect: 0, expect_len: Some(9) },
    ConvCase { rows: &[512], pat: Some(b"*/a?b[c-e]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Given(4), sep: None, esc: None, expect: PCRE2_ERROR_NOMEMORY,
               expect_len: Some(10) },
    ConvCase { rows: &[512], pat: Some(b"*/a?b[c-e]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Given(CONV_BUF), sep: None, esc: None, expect: 0,
               expect_len: None },

    // ---- 513: separator/escape >= 128 is UNREACHABLE through the public API,
    //           because pcre2_set_glob_separator_8/_escape_8 reject those (rows
    //           442-444). The nearest reachable inputs are the accepted ASCII
    //           separators/escapes with PCRE2_CONVERT_UTF, asserted here.
    ConvCase { rows: &[513], pat: Some("a/\u{e9}/b".as_bytes()), plen: PLen::Actual,
               opts: GLOB | CUTF, mode: CMode::Alloc, sep: Some(47), esc: Some(92),
               expect: 0, expect_len: None },
    ConvCase { rows: &[513], pat: Some("a.\u{e9}.b".as_bytes()), plen: PLen::Actual,
               opts: GLOB | CUTF, mode: CMode::Alloc, sep: Some(46), esc: Some(96),
               expect: 0, expect_len: None },
    ConvCase { rows: &[513], pat: Some("x\\\u{2028}y".as_bytes()), plen: PLen::Actual,
               opts: GLOB | CUTF, mode: CMode::Alloc, sep: Some(92), esc: Some(126),
               expect: 0, expect_len: None },

    // ---- 514: GLOB malformed POSIX class inside a bracket expression -------
    // `[[:alph` really does end up unterminated -> 106.
    ConvCase { rows: &[514], pat: Some(b"[[:alph"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(7) },
    ConvCase { rows: &[514], pat: Some(b"[[:alpha:"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 106, expect_len: Some(9) },
    // `[[:alpha]]` and `[[:bogus:]]`, on the other hand, SUCCEED: after
    // convert_glob_parse_class returns 0 the `[` is emitted as a literal and the
    // FIRST `]` still closes the bracket expression, leaving the second `]` as
    // an ordinary escaped literal in the main loop.
    ConvCase { rows: &[514], pat: Some(b"[[:alpha]]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: Some(20) },
    ConvCase { rows: &[514], pat: Some(b"[[:bogus:]]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: Some(21) },
    ConvCase { rows: &[514], pat: Some(b"[[:alpha:]]"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: None },

    // ---- 515: PCRE2_ERROR_UNICODE_NOT_SUPPORTED (132) is UNREACHABLE here,
    //           SUPPORT_UNICODE being defined. The nearest reachable input is
    //           PCRE2_CONVERT_UTF over valid UTF-8, asserted here.
    ConvCase { rows: &[515], pat: Some("\u{e9}\u{4e00}\u{1f600}".as_bytes()),
               plen: PLen::Actual, opts: GLOB | CUTF, mode: CMode::Alloc, sep: None,
               esc: None, expect: 0, expect_len: None },
    ConvCase { rows: &[515], pat: Some("[\u{e9}-\u{4e00}]".as_bytes()), plen: PLen::Actual,
               opts: GLOB | CUTF, mode: CMode::Alloc, sep: None, esc: None, expect: 0,
               expect_len: None },
    ConvCase { rows: &[515], pat: Some("\u{e9}.*".as_bytes()), plen: PLen::Actual,
               opts: PEXT | CUTF, mode: CMode::Alloc, sep: None, esc: None, expect: 0,
               expect_len: None },

    // ---- 502: the `switch(pattype)` default and the fall-out past the
    //           two-iteration loop are DEAD (pattype was validated at :1138).
    //           The nearest reachable inputs are the four legal pattypes.
    ConvCase { rows: &[502], pat: Some(b"a*b?c"), plen: PLen::Actual, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: None },
    ConvCase { rows: &[502], pat: Some(b"a*b?c"), plen: PLen::Actual,
               opts: PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR, mode: CMode::Alloc, sep: None,
               esc: None, expect: 0, expect_len: None },
    ConvCase { rows: &[502], pat: Some(b"a*b?c"), plen: PLen::Actual,
               opts: PCRE2_CONVERT_GLOB_NO_STARSTAR, mode: CMode::Alloc, sep: None,
               esc: None, expect: 0, expect_len: None },
    ConvCase { rows: &[502], pat: Some(b"a.c*"), plen: PLen::Actual, opts: PBAS,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: None },
    ConvCase { rows: &[502], pat: Some(b"a.c*"), plen: PLen::Actual, opts: PEXT,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: None },

    // a truncated-but-in-bounds declared length: the checker sees a lone lead
    // byte, but with NO_UTF_CHECK the converter reads the (present) trailer
    ConvCase { rows: &[500], pat: Some("\u{e9}".as_bytes()), plen: PLen::Exact(1),
               opts: GLOB | CUTF, mode: CMode::Alloc, sep: None, esc: None, expect: -3,
               expect_len: Some(0) },

    // zero-terminated length form
    ConvCase { rows: &[495], pat: Some(b"a*b\0"), plen: PLen::Zeroterm, opts: GLOB,
               mode: CMode::Alloc, sep: None, esc: None, expect: 0, expect_len: None },
];

unsafe fn run_conv(api: &Api, cs: &ConvCase) -> String {
    let cc = if cs.sep.is_some() || cs.esc.is_some() {
        let c = (api.convert_context_create)(ptr::null_mut());
        assert!(!c.is_null());
        if let Some(s) = cs.sep {
            assert_eq!((api.set_glob_separator)(c, s), 0, "separator {s} rejected");
        }
        if let Some(e) = cs.esc {
            assert_eq!((api.set_glob_escape)(c, e), 0, "escape {e} rejected");
        }
        c
    } else {
        ptr::null_mut()
    };

    // Owned pattern buffer with padding (see CONV_PAD).
    let owned: Option<Vec<u8>> = cs.pat.map(|b| {
        let mut v = b.to_vec();
        v.extend_from_slice(&CONV_PAD);
        v
    });
    let pptr: Sptr = match &owned {
        None => ptr::null(),
        Some(v) => v.as_ptr(),
    };
    let plen: Sz = match cs.plen {
        PLen::Actual => cs.pat.map(|b| b.len()).unwrap_or(0),
        PLen::Zeroterm => PCRE2_ZERO_TERMINATED,
        PLen::Exact(n) => n,
    };

    let mut buf = vec![0xA5u8; CONV_BUF];
    let mut bufflen: Sz = 0xDEAD_BEEF;
    let mut buffptr: *mut u8 = ptr::null_mut();
    let (bp, blp): (*mut *mut u8, *mut Sz) = match cs.mode {
        CMode::LenOnly => (ptr::null_mut(), &mut bufflen),
        CMode::Alloc => (&mut buffptr, &mut bufflen),
        CMode::Given(n) => {
            buffptr = buf.as_mut_ptr();
            bufflen = n;
            (&mut buffptr, &mut bufflen)
        }
        CMode::NoBufflen => (&mut buffptr, ptr::null_mut()),
    };

    let rc = (api.pattern_convert)(pptr, plen, cs.opts, bp, blp, cc);
    let mut out = format!("rc={rc}");
    if !matches!(cs.mode, CMode::NoBufflen) {
        out += &format!(" len={bufflen:#x}");
    }
    match cs.mode {
        CMode::Given(_) => {
            out += &format!(" buf={}", show(&buf));
        }
        CMode::Alloc | CMode::NoBufflen => {
            if buffptr.is_null() {
                out += " out=NULL";
            } else {
                let n = if matches!(cs.mode, CMode::NoBufflen) {
                    0
                } else {
                    bufflen
                };
                let bytes = std::slice::from_raw_parts(buffptr, n + 1);
                out += &format!(" out={}", show(bytes));
                (api.converted_pattern_free)(buffptr);
            }
        }
        CMode::LenOnly => {}
    }
    if !cc.is_null() {
        (api.convert_context_free)(cc);
    }
    out
}

#[test]
fn pattern_convert_rejections() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in CONV_CASES {
            let tag = format!(
                "rows {:?} pat={} plen={:?} opts={:#x} mode={:?} sep={:?} esc={:?}",
                cs.rows,
                cs.pat.map(show).unwrap_or_else(|| "NULL".into()),
                cs.plen,
                cs.opts,
                cs.mode,
                cs.sep,
                cs.esc
            );
            let a = run_conv(&p.c, cs);
            let b = run_conv(&p.r, cs);
            d.eq(&tag, a.clone(), b);
            doc.eq(&format!("ERRORS.md {tag} rc"), cs.expect as i64, rc_of(&a));
            if let Some(want) = cs.expect_len {
                let got: u64 = u64::from_str_radix(
                    a.split(" len=0x").nth(1).unwrap().split(' ').next().unwrap(),
                    16,
                )
                .unwrap();
                doc.eq(&format!("ERRORS.md {tag} *bufflenptr"), want as u64, got);
            }
        }
    }
    doc.finish("ERRORS.md rows 495-515");
    d.finish("rows 495-515: pcre2_pattern_convert_8 NULL / BADOPTION / UTF / syntax / buffer errors");
}

// row 501: the second-pass output-buffer allocation fails.
#[test]
fn pattern_convert_allocation_failure() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // rows: &[501]
        for budget in 0..=2i64 {
            let mut outs = [String::new(), String::new()];
            for idx in 0..2 {
                let api = if idx == 0 { &p.c } else { &p.r };
                reset(idx);
                let (m, f) = allocs(idx);
                let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
                let cc = (api.convert_context_create)(g);
                assert!(!cc.is_null());
                set_budget(idx, budget);
                let mut buffptr: *mut u8 = ptr::null_mut();
                let mut bufflen: Sz = 0xDEAD_BEEF;
                let pat = b"a*b?c[d-f]";
                let rc = (api.pattern_convert)(
                    pat.as_ptr(),
                    pat.len(),
                    PCRE2_CONVERT_GLOB,
                    &mut buffptr,
                    &mut bufflen,
                    cc,
                );
                outs[idx] = format!("rc={rc} len={bufflen:#x} null={}", buffptr.is_null());
                set_budget(idx, -1);
                if !buffptr.is_null() {
                    outs[idx] += &format!(
                        " out={}",
                        show(std::slice::from_raw_parts(buffptr, bufflen + 1))
                    );
                    (api.converted_pattern_free)(buffptr);
                }
                (api.convert_context_free)(cc);
                (api.general_context_free)(g);
            }
            d.eq(
                &format!("rows [501] convert allocation budget={budget}"),
                outs[0].clone(),
                outs[1].clone(),
            );
            if budget == 0 {
                assert_eq!(
                    rc_of(&outs[0]),
                    PCRE2_ERROR_NOMEMORY as i64,
                    "ERRORS.md row 501: expected -48 from the C, got {}",
                    outs[0]
                );
                assert!(
                    outs[0].contains("len=0x0 "),
                    "ERRORS.md row 501: *bufflenptr must be 0, got {}",
                    outs[0]
                );
            }
        }
    }
    d.finish("row 501: pcre2_pattern_convert_8 second-pass allocation failure");
}

// row 516: pcre2_converted_pattern_free_8(NULL).
#[test]
fn converted_pattern_free_null() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for _ in 0..3 {
            (p.c.converted_pattern_free)(ptr::null_mut());
            (p.r.converted_pattern_free)(ptr::null_mut());
        }
        d.eq("rows [516] converted_pattern_free(NULL) survived", true, true);
        // The custom `free` must NOT be reached for NULL.
        let mut added = [0i64; 2];
        for idx in 0..2 {
            let api = if idx == 0 { &p.c } else { &p.r };
            reset(idx);
            let (m, f) = allocs(idx);
            let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
            let cc = (api.convert_context_create)(g);
            let mut buffptr: *mut u8 = ptr::null_mut();
            let mut bufflen: Sz = 0;
            let pat = b"a*";
            assert_eq!(
                (api.pattern_convert)(
                    pat.as_ptr(),
                    pat.len(),
                    PCRE2_CONVERT_GLOB,
                    &mut buffptr,
                    &mut bufflen,
                    cc
                ),
                0
            );
            (api.converted_pattern_free)(buffptr);
            let before = nfree(idx);
            for _ in 0..3 {
                (api.converted_pattern_free)(ptr::null_mut());
            }
            added[idx] = nfree(idx) - before;
            (api.convert_context_free)(cc);
            (api.general_context_free)(g);
        }
        d.eq("rows [516] free calls added by the NULL frees", added[0], added[1]);
        assert_eq!(added[0], 0, "ERRORS.md row 516: the C must not call free");
    }
    d.finish("row 516: pcre2_converted_pattern_free_8(NULL) is a guarded no-op");
}

// ============================================================= pcre2_valid_utf.c

struct UtfCase {
    rows: &'static [u32],
    bytes: &'static [u8],
    expect: c_int,
    /// `*erroroffset` when the bad sequence starts at offset 0
    expect_off: Sz,
}

const UTF_CASES: &[UtfCase] = &[
    // 517: 2-byte lead byte as the last byte
    UtfCase { rows: &[517], bytes: &[0xC2], expect: -3, expect_off: 0 },
    UtfCase { rows: &[517], bytes: &[0xDF], expect: -3, expect_off: 0 },
    // 518: 3-byte lead byte as the last byte
    UtfCase { rows: &[518], bytes: &[0xE1], expect: -4, expect_off: 0 },
    UtfCase { rows: &[518], bytes: &[0xEF], expect: -4, expect_off: 0 },
    // 519: three bytes missing
    UtfCase { rows: &[519], bytes: &[0xF0], expect: -5, expect_off: 0 },
    UtfCase { rows: &[519], bytes: &[0xF8, 0x80], expect: -5, expect_off: 0 },
    // 520: four bytes missing
    UtfCase { rows: &[520], bytes: &[0xF8], expect: -6, expect_off: 0 },
    UtfCase { rows: &[520], bytes: &[0xFC, 0x80], expect: -6, expect_off: 0 },
    // 521: five bytes missing
    UtfCase { rows: &[521], bytes: &[0xFC], expect: -7, expect_off: 0 },
    // 522: 2nd byte not 10xxxxxx
    UtfCase { rows: &[522], bytes: &[0xC2, 0x41], expect: -8, expect_off: 0 },
    UtfCase { rows: &[522], bytes: &[0xE1, 0x41, 0x80], expect: -8, expect_off: 0 },
    UtfCase { rows: &[522], bytes: &[0xF0, 0xC0, 0x80, 0x80], expect: -8, expect_off: 0 },
    // 523: 3rd byte not 10xxxxxx
    UtfCase { rows: &[523], bytes: &[0xE1, 0x80, 0x41], expect: -9, expect_off: 0 },
    UtfCase { rows: &[523], bytes: &[0xF0, 0x90, 0x41, 0x80], expect: -9, expect_off: 0 },
    UtfCase { rows: &[523], bytes: &[0xF8, 0x88, 0x41, 0x80, 0x80], expect: -9,
              expect_off: 0 },
    UtfCase { rows: &[523], bytes: &[0xFC, 0x84, 0x41, 0x80, 0x80, 0x80], expect: -9,
              expect_off: 0 },
    // 524: 4th byte not 10xxxxxx
    UtfCase { rows: &[524], bytes: &[0xF0, 0x90, 0x80, 0x41], expect: -10, expect_off: 0 },
    UtfCase { rows: &[524], bytes: &[0xF8, 0x88, 0x80, 0x41, 0x80], expect: -10,
              expect_off: 0 },
    UtfCase { rows: &[524], bytes: &[0xFC, 0x84, 0x80, 0x41, 0x80, 0x80], expect: -10,
              expect_off: 0 },
    // 525: 5th byte not 10xxxxxx
    UtfCase { rows: &[525], bytes: &[0xF8, 0x88, 0x80, 0x80, 0x41], expect: -11,
              expect_off: 0 },
    UtfCase { rows: &[525], bytes: &[0xFC, 0x84, 0x80, 0x80, 0x41, 0x80], expect: -11,
              expect_off: 0 },
    // 526: 6th byte not 10xxxxxx
    UtfCase { rows: &[526], bytes: &[0xFC, 0x84, 0x80, 0x80, 0x80, 0x41], expect: -12,
              expect_off: 0 },
    // 527: well-formed but forbidden 5-byte character
    UtfCase { rows: &[527], bytes: &[0xF8, 0x88, 0x80, 0x80, 0x80], expect: -13,
              expect_off: 0 },
    // 528: well-formed but forbidden 6-byte character
    UtfCase { rows: &[528], bytes: &[0xFC, 0x84, 0x80, 0x80, 0x80, 0x80], expect: -14,
              expect_off: 0 },
    // 529: 4-byte character above U+10FFFF
    UtfCase { rows: &[529], bytes: &[0xF5, 0x80, 0x80, 0x80], expect: -15, expect_off: 0 },
    UtfCase { rows: &[529], bytes: &[0xF4, 0x90, 0x80, 0x80], expect: -15, expect_off: 0 },
    // 530: 3-byte encoding of a surrogate
    UtfCase { rows: &[530], bytes: &[0xED, 0xA0, 0x80], expect: -16, expect_off: 0 },
    UtfCase { rows: &[530], bytes: &[0xED, 0xBF, 0xBF], expect: -16, expect_off: 0 },
    // 531: overlong 2-byte sequence
    UtfCase { rows: &[531], bytes: &[0xC0, 0x80], expect: -17, expect_off: 0 },
    UtfCase { rows: &[531], bytes: &[0xC1, 0xBF], expect: -17, expect_off: 0 },
    // 532: overlong 3-byte sequence
    UtfCase { rows: &[532], bytes: &[0xE0, 0x80, 0x80], expect: -18, expect_off: 0 },
    UtfCase { rows: &[532], bytes: &[0xE0, 0x9F, 0xBF], expect: -18, expect_off: 0 },
    // 533: overlong 4-byte sequence
    UtfCase { rows: &[533], bytes: &[0xF0, 0x80, 0x80, 0x80], expect: -19, expect_off: 0 },
    UtfCase { rows: &[533], bytes: &[0xF0, 0x8F, 0xBF, 0xBF], expect: -19, expect_off: 0 },
    // 534: overlong 5-byte sequence
    UtfCase { rows: &[534], bytes: &[0xF8, 0x80, 0x80, 0x80, 0x80], expect: -20,
              expect_off: 0 },
    UtfCase { rows: &[534], bytes: &[0xF8, 0x87, 0xBF, 0xBF, 0xBF], expect: -20,
              expect_off: 0 },
    // 535: overlong 6-byte sequence
    UtfCase { rows: &[535], bytes: &[0xFC, 0x80, 0x80, 0x80, 0x80, 0x80], expect: -21,
              expect_off: 0 },
    UtfCase { rows: &[535], bytes: &[0xFC, 0x83, 0xBF, 0xBF, 0xBF, 0xBF], expect: -21,
              expect_off: 0 },
    // 536: isolated continuation byte
    UtfCase { rows: &[536], bytes: &[0x80], expect: -22, expect_off: 0 },
    UtfCase { rows: &[536], bytes: &[0xBF], expect: -22, expect_off: 0 },
    UtfCase { rows: &[536], bytes: &[0xA0, 0x41], expect: -22, expect_off: 0 },
    // 537: the illegal bytes 0xFE / 0xFF
    UtfCase { rows: &[537], bytes: &[0xFF], expect: -23, expect_off: 0 },
    UtfCase { rows: &[537], bytes: &[0xFE], expect: -23, expect_off: 0 },
    UtfCase { rows: &[537], bytes: &[0xFE, 0x80, 0x80], expect: -23, expect_off: 0 },
];

/// Valid-UTF-8 prefixes prepended to each bad sequence, so the reported
/// `*erroroffset` / `startchar` has to move with them.
const UTF_PREFIXES: &[&[u8]] = &[b"", b"ab", "x\u{e9}\u{1f600}".as_bytes()];

#[test]
fn valid_utf_direct() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in UTF_CASES {
            for pre in UTF_PREFIXES {
                let mut s = pre.to_vec();
                s.extend_from_slice(cs.bytes);
                let want_off = cs.expect_off + pre.len();
                let mut oa: Sz = 0xDEAD_BEEF;
                let mut ob: Sz = 0xDEAD_BEEF;
                let ra = (p.c.p_valid_utf)(s.as_ptr(), s.len(), &mut oa);
                let rb = (p.r.p_valid_utf)(s.as_ptr(), s.len(), &mut ob);
                let tag = format!("rows {:?} valid_utf({})", cs.rows, show(&s));
                d.eq(&tag, (ra, oa), (rb, ob));
                doc.eq(&format!("ERRORS.md {tag} rc"), cs.expect, ra);
                doc.eq(&format!("ERRORS.md {tag} erroroffset"), want_off, oa);
            }
            // a valid prefix followed by the bad sequence followed by more text
            let mut s = b"ab".to_vec();
            s.extend_from_slice(cs.bytes);
            s.extend_from_slice(b"cd");
            let mut oa: Sz = 0xDEAD_BEEF;
            let mut ob: Sz = 0xDEAD_BEEF;
            let ra = (p.c.p_valid_utf)(s.as_ptr(), s.len(), &mut oa);
            let rb = (p.r.p_valid_utf)(s.as_ptr(), s.len(), &mut ob);
            d.eq(
                &format!("rows {:?} valid_utf({}) trailing", cs.rows, show(&s)),
                (ra, oa),
                (rb, ob),
            );
        }
        // and a control: valid UTF-8 leaves *erroroffset alone and returns 0
        for s in ["", "a", "abc", "\u{e9}", "\u{2028}", "\u{10ffff}", "a\u{1f600}b"] {
            let b = s.as_bytes();
            let mut oa: Sz = 0xDEAD_BEEF;
            let mut ob: Sz = 0xDEAD_BEEF;
            let ra = (p.c.p_valid_utf)(b.as_ptr(), b.len(), &mut oa);
            let rb = (p.r.p_valid_utf)(b.as_ptr(), b.len(), &mut ob);
            d.eq(
                &format!("rows [517-537] valid_utf({}) valid", show(b)),
                (ra, oa),
                (rb, ob),
            );
            assert_eq!(ra, 0);
        }
    }
    doc.finish("ERRORS.md rows 517-537 (direct _pcre2_valid_utf_8 calls)");
    d.finish("rows 517-537: every PCRE2_ERROR_UTF8_ERR1..ERR21 through _pcre2_valid_utf_8");
}

#[test]
fn valid_utf_through_compile() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in UTF_CASES {
            for pre in UTF_PREFIXES {
                let mut s = pre.to_vec();
                s.extend_from_slice(cs.bytes);
                let want_off = cs.expect_off + pre.len();
                let mut ea: c_int = 0;
                let mut eb: c_int = 0;
                let mut oa: Sz = 0xDEAD_BEEF;
                let mut ob: Sz = 0xDEAD_BEEF;
                let ka = (p.c.compile)(s.as_ptr(), s.len(), PCRE2_UTF, &mut ea, &mut oa,
                                       ptr::null_mut());
                let kb = (p.r.compile)(s.as_ptr(), s.len(), PCRE2_UTF, &mut eb, &mut ob,
                                       ptr::null_mut());
                let tag = format!("rows {:?} compile(UTF, {})", cs.rows, show(&s));
                d.eq(&tag, (ka.is_null(), ea, oa), (kb.is_null(), eb, ob));
                doc.eq(&format!("ERRORS.md {tag} errorcode"), cs.expect, ea);
                doc.eq(&format!("ERRORS.md {tag} erroroffset"), want_off, oa);
                assert!(ka.is_null() && kb.is_null());
                // With PCRE2_NO_UTF_CHECK the same pattern skips the check, so
                // it either compiles or fails for an unrelated syntax reason —
                // whichever it is, both libraries must agree.
                let mut ea2: c_int = 0;
                let mut eb2: c_int = 0;
                let mut oa2: Sz = 0xDEAD_BEEF;
                let mut ob2: Sz = 0xDEAD_BEEF;
                let ka2 = (p.c.compile)(s.as_ptr(), s.len(), PCRE2_UTF | PCRE2_NO_UTF_CHECK,
                                        &mut ea2, &mut oa2, ptr::null_mut());
                let kb2 = (p.r.compile)(s.as_ptr(), s.len(), PCRE2_UTF | PCRE2_NO_UTF_CHECK,
                                        &mut eb2, &mut ob2, ptr::null_mut());
                d.eq(
                    &format!("{tag} with NO_UTF_CHECK"),
                    (ka2.is_null(), ea2, oa2),
                    (kb2.is_null(), eb2, ob2),
                );
                if !ka2.is_null() && !kb2.is_null() {
                    assert_code_eq(ka2, kb2, &tag);
                }
                if !ka2.is_null() {
                    (p.c.code_free)(ka2);
                }
                if !kb2.is_null() {
                    (p.r.code_free)(kb2);
                }
            }
        }
    }
    doc.finish("ERRORS.md rows 517-537 (reached through pcre2_compile_8)");
    d.finish("rows 517-537: every UTF-8 error reached through pcre2_compile_8 with PCRE2_UTF");
}

#[test]
fn valid_utf_through_match() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        let ka = must_compile(&p.c, b"a", PCRE2_UTF);
        let kb = must_compile(&p.r, b"a", PCRE2_UTF);
        let mda = (p.c.match_data_create)(4, ptr::null_mut());
        let mdb = (p.r.match_data_create)(4, ptr::null_mut());
        for cs in UTF_CASES {
            for pre in UTF_PREFIXES {
                let mut s = pre.to_vec();
                s.extend_from_slice(cs.bytes);
                let want_off = cs.expect_off + pre.len();
                let tag = format!("rows {:?} match(UTF, {})", cs.rows, show(&s));

                let ra = (p.c.do_match)(ka, s.as_ptr(), s.len(), 0, 0, mda, ptr::null_mut());
                let rb = (p.r.do_match)(kb, s.as_ptr(), s.len(), 0, 0, mdb, ptr::null_mut());
                d.eq(
                    &tag,
                    read_match_out(&p.c, mda, ra),
                    read_match_out(&p.r, mdb, rb),
                );
                doc.eq(&format!("ERRORS.md {tag} rc"), cs.expect, ra);
                doc.eq(
                    &format!("ERRORS.md {tag} startchar"),
                    want_off,
                    (p.c.get_startchar)(mda),
                );

                // pcre2_dfa_match_8 reaches the same check.
                let mut ws = [0i32; 128];
                let da = (p.c.dfa_match)(ka, s.as_ptr(), s.len(), 0, 0, mda, ptr::null_mut(),
                                         ws.as_mut_ptr(), ws.len());
                let db = (p.r.dfa_match)(kb, s.as_ptr(), s.len(), 0, 0, mdb, ptr::null_mut(),
                                         ws.as_mut_ptr(), ws.len());
                d.eq(
                    &format!("{tag} dfa"),
                    read_match_out_of(&p.c, mda, da, Engine::Dfa),
                    read_match_out_of(&p.r, mdb, db, Engine::Dfa),
                );
                doc.eq(&format!("ERRORS.md {tag} dfa rc"), cs.expect, da);
                doc.eq(
                    &format!("ERRORS.md {tag} dfa startchar"),
                    want_off,
                    (p.c.get_startchar)(mda),
                );

                // With PCRE2_NO_UTF_CHECK the check is skipped, so the matcher
                // walks the raw bytes; both libraries must still agree.
                let na = (p.c.do_match)(ka, s.as_ptr(), s.len(), 0, PCRE2_NO_UTF_CHECK, mda,
                                        ptr::null_mut());
                let nb = (p.r.do_match)(kb, s.as_ptr(), s.len(), 0, PCRE2_NO_UTF_CHECK, mdb,
                                        ptr::null_mut());
                d.eq(
                    &format!("{tag} with NO_UTF_CHECK"),
                    read_match_out(&p.c, mda, na),
                    read_match_out(&p.r, mdb, nb),
                );
            }
        }
        (p.c.match_data_free)(mda);
        (p.r.match_data_free)(mdb);
        (p.c.code_free)(ka);
        (p.r.code_free)(kb);
    }
    doc.finish("ERRORS.md rows 517-537 (reached through pcre2_match_8 / pcre2_dfa_match_8)");
    d.finish("rows 517-537: every UTF-8 error reached through the matchers on an invalid subject");
}

// ======================================== Other files — no rejection paths (538-542)

struct OrdCase {
    rows: &'static [u32],
    cvalue: u32,
    /// documented number of code units written
    expect: u32,
}
const ORD_CASES: &[OrdCase] = &[
    OrdCase { rows: &[538], cvalue: 0, expect: 1 },
    OrdCase { rows: &[538], cvalue: 1, expect: 1 },
    OrdCase { rows: &[538], cvalue: 0x7f, expect: 1 },
    OrdCase { rows: &[538], cvalue: 0x80, expect: 2 },
    OrdCase { rows: &[538], cvalue: 0x7ff, expect: 2 },
    OrdCase { rows: &[538], cvalue: 0x800, expect: 3 },
    // surrogates are happily encoded — no validation at all
    OrdCase { rows: &[538], cvalue: 0xd800, expect: 3 },
    OrdCase { rows: &[538], cvalue: 0xdfff, expect: 3 },
    OrdCase { rows: &[538], cvalue: 0xffff, expect: 3 },
    OrdCase { rows: &[538], cvalue: 0x1_0000, expect: 4 },
    OrdCase { rows: &[538], cvalue: 0x10_ffff, expect: 4 },
    // above U+10FFFF, likewise unvalidated
    OrdCase { rows: &[538], cvalue: 0x11_0000, expect: 4 },
    OrdCase { rows: &[538], cvalue: 0x1f_ffff, expect: 4 },
    OrdCase { rows: &[538], cvalue: 0x20_0000, expect: 5 },
    OrdCase { rows: &[538], cvalue: 0x3ff_ffff, expect: 5 },
    OrdCase { rows: &[538], cvalue: 0x400_0000, expect: 6 },
    OrdCase { rows: &[538], cvalue: 0x7fff_ffff, expect: 6 },
    // (int)cvalue < 0: the utf8_table1 scan breaks at i == 0
    OrdCase { rows: &[538], cvalue: 0x8000_0000, expect: 1 },
    OrdCase { rows: &[538], cvalue: 0xdead_beef, expect: 1 },
    OrdCase { rows: &[538], cvalue: 0xffff_ffff, expect: 1 },
];

#[test]
fn ord2utf_has_no_rejection_path() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in ORD_CASES {
            let mut ba = [0xAAu8; 16];
            let mut bb = [0xAAu8; 16];
            let ra = (p.c.p_ord2utf)(cs.cvalue, ba.as_mut_ptr());
            let rb = (p.r.p_ord2utf)(cs.cvalue, bb.as_mut_ptr());
            let tag = format!("rows {:?} ord2utf({:#x})", cs.rows, cs.cvalue);
            d.eq(&tag, ra, rb);
            d.eq(&format!("{tag} buffer"), ba, bb);
            doc.eq(&format!("ERRORS.md {tag} code units"), cs.expect, ra as u32);
            doc.eq(
                &format!("ERRORS.md {tag} nothing beyond {ra} bytes written"),
                vec![0xAAu8; 16 - ra as usize],
                ba[ra as usize..].to_vec(),
            );
        }
    }
    doc.finish("ERRORS.md row 538");
    d.finish("row 538: _pcre2_ord2utf_8 accepts every uint32 without a rejection path");
}

const NL_BUF: &[u8] = &[
    b'a', b'\r', b'\n', b'b', 0xC2, 0x85, 0xE2, 0x80, 0xA8, b'z', b'\n', b'\r',
];

struct NlCase {
    rows: &'static [u32],
    /// NLTYPE_ANY = 1, NLTYPE_ANYCRLF = 2 (NLTYPE_FIXED is handled inline by the
    /// callers and never reaches these functions)
    nltype: u32,
    utf: c_int,
}
const NL_CASES: &[NlCase] = &[
    NlCase { rows: &[539], nltype: 1, utf: 0 },
    NlCase { rows: &[539], nltype: 1, utf: 1 },
    NlCase { rows: &[539], nltype: 2, utf: 0 },
    NlCase { rows: &[539], nltype: 2, utf: 1 },
    // an unrecognised type falls into the NLTYPE_ANY arm; still no error path
    NlCase { rows: &[539], nltype: 0, utf: 0 },
    NlCase { rows: &[539], nltype: 99, utf: 1 },
];

#[test]
fn newline_helpers_have_no_rejection_path() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let s = NL_BUF;
        let start = s.as_ptr();
        let end = start.add(s.len());
        for cs in NL_CASES {
            for i in 0..s.len() {
                let mut la: u32 = 0xDEAD_BEEF;
                let mut lb: u32 = 0xDEAD_BEEF;
                let ra = (p.c.p_is_newline)(start.add(i), cs.nltype, end, &mut la, cs.utf);
                let rb = (p.r.p_is_newline)(start.add(i), cs.nltype, end, &mut lb, cs.utf);
                d.eq(
                    &format!(
                        "rows {:?} is_newline(off {i}, type {}, utf {})",
                        cs.rows, cs.nltype, cs.utf
                    ),
                    (ra, la),
                    (rb, lb),
                );
            }
            // was_newline requires ptr > startptr
            for i in 1..=s.len() {
                let mut la: u32 = 0xDEAD_BEEF;
                let mut lb: u32 = 0xDEAD_BEEF;
                let ra = (p.c.p_was_newline)(start.add(i), cs.nltype, start, &mut la, cs.utf);
                let rb = (p.r.p_was_newline)(start.add(i), cs.nltype, start, &mut lb, cs.utf);
                d.eq(
                    &format!(
                        "rows {:?} was_newline(off {i}, type {}, utf {})",
                        cs.rows, cs.nltype, cs.utf
                    ),
                    (ra, la),
                    (rb, lb),
                );
            }
        }
        // FALSE is the ordinary "not a newline" answer, and it leaves *lenptr
        // untouched — assert that on a non-newline byte.
        let mut la: u32 = 0x1234;
        let r = (p.c.p_is_newline)(start, 1, end, &mut la, 0);
        assert_eq!((r, la), (0, 0x1234), "ERRORS.md row 539: 'a' is not a newline");
    }
    d.finish("row 539: _pcre2_is_newline_8 / _pcre2_was_newline_8 have no error path");
}

const STR_CORPUS: &[&[u8]] = &[
    b"\0",
    b"a\0",
    b"ab\0",
    b"abc\0",
    b"abd\0",
    b"abcd\0",
    b"b\0",
    b"A\0",
    b"zzzzz\0",
    b"\x7f\0",
    b"\xff\0",
    b"\x80\x81\0",
];

struct StrCase {
    rows: &'static [u32],
    name: &'static str,
}
const STR_CASES: &[StrCase] = &[
    StrCase { rows: &[540], name: "strlen" },
    StrCase { rows: &[540], name: "strcmp" },
    StrCase { rows: &[540], name: "strcmp_c8" },
    StrCase { rows: &[540], name: "strncmp" },
    StrCase { rows: &[540], name: "strncmp_c8" },
    StrCase { rows: &[540], name: "strcpy_c8" },
];

#[test]
fn string_utils_have_no_rejection_path() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for cs in STR_CASES {
            for a in STR_CORPUS {
                match cs.name {
                    "strlen" => {
                        d.eq(
                            &format!("rows {:?} strlen({})", cs.rows, show(a)),
                            (p.c.p_strlen)(a.as_ptr()),
                            (p.r.p_strlen)(a.as_ptr()),
                        );
                    }
                    "strcpy_c8" => {
                        let mut ba = [0xA5u8; 32];
                        let mut bb = [0xA5u8; 32];
                        let ra = (p.c.p_strcpy_c8)(ba.as_mut_ptr(), a.as_ptr() as *const c_char);
                        let rb = (p.r.p_strcpy_c8)(bb.as_mut_ptr(), a.as_ptr() as *const c_char);
                        d.eq(
                            &format!("rows {:?} strcpy_c8({})", cs.rows, show(a)),
                            (ra, ba),
                            (rb, bb),
                        );
                    }
                    _ => {
                        for b in STR_CORPUS {
                            let tag = format!(
                                "rows {:?} {}({}, {})",
                                cs.rows,
                                cs.name,
                                show(a),
                                show(b)
                            );
                            match cs.name {
                                "strcmp" => d.eq(
                                    &tag,
                                    (p.c.p_strcmp)(a.as_ptr(), b.as_ptr()),
                                    (p.r.p_strcmp)(a.as_ptr(), b.as_ptr()),
                                ),
                                "strcmp_c8" => d.eq(
                                    &tag,
                                    (p.c.p_strcmp_c8)(a.as_ptr(), b.as_ptr() as *const c_char),
                                    (p.r.p_strcmp_c8)(a.as_ptr(), b.as_ptr() as *const c_char),
                                ),
                                "strncmp" => {
                                    for n in [0usize, 1, 2, 3, 6] {
                                        d.eq(
                                            &format!("{tag} n={n}"),
                                            (p.c.p_strncmp)(a.as_ptr(), b.as_ptr(), n),
                                            (p.r.p_strncmp)(a.as_ptr(), b.as_ptr(), n),
                                        );
                                    }
                                }
                                _ => {
                                    for n in [0usize, 1, 2, 3, 6] {
                                        d.eq(
                                            &format!("{tag} n={n}"),
                                            (p.c.p_strncmp_c8)(
                                                a.as_ptr(),
                                                b.as_ptr() as *const c_char,
                                                n,
                                            ),
                                            (p.r.p_strncmp_c8)(
                                                a.as_ptr(),
                                                b.as_ptr() as *const c_char,
                                                n,
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    d.finish("row 540: the pcre2_string_utils.c helpers have no error paths");
}

struct MulCase {
    rows: &'static [u32],
    a: c_int,
    b: c_int,
    /// ERRORS.md: never TRUE on a 64-bit host
    expect: Bool,
}
const MUL_CASES: &[MulCase] = &[
    MulCase { rows: &[541], a: 0, b: 0, expect: 0 },
    MulCase { rows: &[541], a: 1, b: 1, expect: 0 },
    MulCase { rows: &[541], a: 100_000, b: 100_000, expect: 0 },
    MulCase { rows: &[541], a: 65535, b: 65536, expect: 0 },
    MulCase { rows: &[541], a: c_int::MAX, b: 1, expect: 0 },
    MulCase { rows: &[541], a: c_int::MAX, b: 2, expect: 0 },
    MulCase { rows: &[541], a: c_int::MAX, b: c_int::MAX, expect: 0 },
    MulCase { rows: &[541], a: 0, b: c_int::MAX, expect: 0 },
    // the PCRE2_ASSERT(a >= 0 && b >= 0) is a no-op in this build
    MulCase { rows: &[541], a: -1, b: -1, expect: 0 },
    MulCase { rows: &[541], a: -1, b: 2, expect: 0 },
    MulCase { rows: &[541], a: c_int::MIN, b: c_int::MIN, expect: 0 },
    MulCase { rows: &[541], a: c_int::MIN, b: 3, expect: 0 },
];

#[test]
fn ckd_smul_never_reports_overflow() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in MUL_CASES {
            let mut ra: Sz = 0xDEAD_BEEF;
            let mut rb: Sz = 0xDEAD_BEEF;
            let ba = (p.c.p_ckd_smul)(&mut ra, cs.a, cs.b);
            let bb = (p.r.p_ckd_smul)(&mut rb, cs.a, cs.b);
            let tag = format!("rows {:?} ckd_smul({}, {})", cs.rows, cs.a, cs.b);
            d.eq(&tag, (ba, ra), (bb, rb));
            doc.eq(&format!("ERRORS.md {tag} never TRUE"), cs.expect, ba);
        }
    }
    doc.finish("ERRORS.md row 541");
    d.finish("row 541: _pcre2_ckd_smul_8 never reports overflow on a 64-bit host");
}

// row 542: pcre2_maketables_free_8 has NO NULL guard.
#[test]
fn maketables_free_null_tables() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // gcontext == NULL: plain free(NULL), harmless.
        for _ in 0..3 {
            (p.c.maketables_free)(ptr::null_mut(), ptr::null());
            (p.r.maketables_free)(ptr::null_mut(), ptr::null());
        }
        d.eq("rows [542] maketables_free(NULL, NULL) survived", true, true);

        // gcontext != NULL: the user's free IS called, with a NULL block.
        let mut calls = [0i64; 2];
        for idx in 0..2 {
            let api = if idx == 0 { &p.c } else { &p.r };
            reset(idx);
            let (m, f) = allocs(idx);
            let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
            // a real round trip first, so the counter is known to be live
            let t = (api.maketables)(g);
            assert!(!t.is_null());
            (api.maketables_free)(g, t);
            let before = nfree(idx);
            for _ in 0..3 {
                (api.maketables_free)(g, ptr::null());
            }
            calls[idx] = nfree(idx) - before;
            (api.general_context_free)(g);
        }
        d.eq("rows [542] free calls for a NULL tables pointer", calls[0], calls[1]);
        assert_eq!(
            calls[0], 3,
            "ERRORS.md row 542: with a non-NULL gcontext the user's free must be \
             called even for a NULL block"
        );
    }
    d.finish("row 542: pcre2_maketables_free_8 has no guard for tables == NULL");
}
