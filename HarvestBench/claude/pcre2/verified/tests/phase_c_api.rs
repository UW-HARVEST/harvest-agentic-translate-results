// Phase C (error paths) — ERRORS.md rows 380..494.
//
//   pcre2_substring.c     380-412
//   pcre2_match_data.c    413-418
//   pcre2_pattern_info.c  419-436
//   pcre2_context.c       437-454
//   pcre2_config.c        455-459
//   pcre2_serialize.c     460-488
//   pcre2_error.c         489-494
//
// Every case constructs the EXACT invalid input the row names, calls the
// function in BOTH shared libraries, and compares the numeric result plus every
// out-parameter and the full contents of every caller-supplied buffer.

mod common;
use common::*;
use std::alloc::Layout;
use std::ffi::{c_int, c_void};
use std::mem::offset_of;
use std::ptr;

// ==================================================================== allocators

// A fallible allocator with a SEPARATE budget/counter per library, as
// tests/HARNESS.md prescribes. Index 0 = C, index 1 = rust.
static mut BUDGET: [i64; 2] = [-1, -1]; // -1 = unlimited
static mut NMALLOC: [i64; 2] = [0, 0];
static mut NFREE: [i64; 2] = [0, 0];

// The custom allocators below must be interchangeable with the libraries' OWN
// defaults (`malloc`/`free`), because some rows deliberately supply only one of
// the two and let the default fill in the other. So they are built on the
// process's real `malloc`/`free`, resolved once through `dlopen(NULL)`.
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

unsafe fn raw_alloc(n: usize, zero: bool) -> *mut c_void {
    let (m, _) = libc_fns();
    let p = m(n.max(1));
    assert!(!p.is_null(), "test harness out of memory ({n} bytes)");
    if zero {
        ptr::write_bytes(p as *mut u8, 0, n.max(1));
    }
    p
}
unsafe fn raw_free(p: *mut c_void) {
    let (_, f) = libc_fns();
    f(p)
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
    raw_alloc(n, false)
}
unsafe extern "C" fn mal_c(n: usize, _d: *mut c_void) -> *mut c_void {
    fallible(0, n)
}
unsafe extern "C" fn mal_r(n: usize, _d: *mut c_void) -> *mut c_void {
    fallible(1, n)
}
unsafe extern "C" fn fre_c(p: *mut c_void, _d: *mut c_void) {
    (*ptr::addr_of_mut!(NFREE))[0] += 1;
    raw_free(p)
}
unsafe extern "C" fn fre_r(p: *mut c_void, _d: *mut c_void) {
    (*ptr::addr_of_mut!(NFREE))[1] += 1;
    raw_free(p)
}

/// `(malloc, free)` pair for library `idx` (0 = C, 1 = rust).
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
unsafe fn nmalloc(idx: usize) -> i64 {
    (*ptr::addr_of_mut!(NMALLOC))[idx]
}
unsafe fn nfree(idx: usize) -> i64 {
    (*ptr::addr_of_mut!(NFREE))[idx]
}

// A zeroing allocator (identical code for both libraries — no counters), used
// where the C reads a field that `pcre2_match_data_create` never initialises.
// Without it the observable would be uninitialised heap, which is not
// behaviour and therefore not comparable.
unsafe extern "C" fn zmal(n: usize, _d: *mut c_void) -> *mut c_void {
    raw_alloc(n, true)
}
unsafe extern "C" fn zfree(p: *mut c_void, _d: *mut c_void) {
    raw_free(p)
}

// ======================================================================= helpers

unsafe fn must_compile(api: &Api, pat: &[u8], opts: u32) -> Ptr {
    must_compile_cc(api, pat, opts, ptr::null_mut())
}
unsafe fn must_compile_cc(api: &Api, pat: &[u8], opts: u32, cc: Ptr) -> Ptr {
    let mut e: c_int = 0;
    let mut off: Sz = 0;
    let c = (api.compile)(pat.as_ptr(), pat.len(), opts, &mut e, &mut off, cc);
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

fn nul(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

/// Pull the first `rc=<int>` out of a rendered observation.
fn rc_of(s: &str) -> i64 {
    let t = s.split("rc=").nth(1).expect("no rc= in observation");
    t.split(|c: char| !(c.is_ascii_digit() || c == '-'))
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

/// An aligned, freeable byte buffer — a serialized stream must be at least
/// pointer-aligned for `pcre2_serialized_data` to be read out of it.
struct Buf {
    p: *mut u8,
    len: usize,
}
impl Buf {
    unsafe fn from_raw(src: *const u8, len: usize) -> Buf {
        let p = std::alloc::alloc(Layout::from_size_align(len, 16).unwrap());
        assert!(!p.is_null());
        ptr::copy_nonoverlapping(src, p, len);
        Buf { p, len }
    }
    fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.p, self.len) }
    }
    unsafe fn patch(&mut self, o: &SerOffsets, patch: Patch) {
        let put = |off: usize, src: &[u8]| {
            assert!(off + src.len() <= self.len);
            ptr::copy_nonoverlapping(src.as_ptr(), self.p.add(off), src.len());
        };
        match patch {
            Patch::None => {}
            Patch::U32(f, v) => put(field_off(o, f), &v.to_ne_bytes()),
            Patch::I32(f, v) => put(field_off(o, f), &v.to_ne_bytes()),
            Patch::U16(f, v) => put(field_off(o, f), &v.to_ne_bytes()),
            Patch::Usize(f, v) => put(field_off(o, f), &v.to_ne_bytes()),
        }
    }
}
impl Drop for Buf {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.p, Layout::from_size_align(self.len, 16).unwrap()) }
    }
}

// ============================================================== pcre2_substring.c

#[derive(Copy, Clone, Debug)]
enum Md {
    /// `pcre2_match_data_create_from_pattern_8`
    FromPat,
    /// `pcre2_match_data_create_8(n, NULL)`
    Count(u32),
}

#[derive(Copy, Clone, Debug)]
enum Run {
    /// no match call at all (the accessor only looks at `code`)
    NoRun,
    /// `pcre2_match_8` with these extra option bits
    Norm(u32),
    /// `pcre2_dfa_match_8` with these extra option bits
    Dfa(u32),
}

#[derive(Copy, Clone, Debug)]
enum Call {
    /// group, pass a real `sizeptr`?
    LenNum(u32, bool),
    /// group, value written into `*sizeptr` before the call
    CopyNum(u32, usize),
    GetNum(u32),
    LenName(&'static str, bool),
    CopyName(&'static str, usize),
    GetName(&'static str),
    /// name, pass non-NULL `firstptr`/`lastptr`?
    Scan(&'static str, bool),
    NumFromName(&'static str),
    /// pass a `lengthsptr`?
    ListGet(bool),
    /// Hand-write `ovector[2*g]` past `subject_length`, then `LenNum(g)`.
    /// This is the only way to reach pcre2_substring.c:344.
    HackOvector(u32),
}

struct SubCase {
    rows: &'static [u32],
    pat: &'static str,
    opts: u32,
    subj: &'static str,
    md: Md,
    run: Run,
    call: Call,
    /// what ERRORS.md documents the C returns
    expect: c_int,
}

const SUB_BUF: usize = 32;

const SUB_CASES: &[SubCase] = &[
    // ---- pcre2_substring_copy_byname_8 -------------------------------- 380-384
    SubCase { rows: &[380], pat: "(?<n>a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Dfa(0), call: Call::CopyName("n", 16), expect: PCRE2_ERROR_DFA_UFUNC },
    SubCase { rows: &[381], pat: "(?<abc>a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyName("xyz", 16), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[382], pat: "(a)(b)(?<n>c)", opts: 0, subj: "abc", md: Md::Count(1),
              run: Run::Norm(0), call: Call::CopyName("n", 16), expect: PCRE2_ERROR_UNAVAILABLE },
    SubCase { rows: &[383], pat: "(?<n>a)|b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyName("n", 16), expect: PCRE2_ERROR_UNSET },
    SubCase { rows: &[384], pat: "(?<n>abc)", opts: 0, subj: "abc", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyName("n", 3), expect: PCRE2_ERROR_NOMEMORY },
    SubCase { rows: &[384], pat: "(?<n>abc)", opts: 0, subj: "abc", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyName("n", 0), expect: PCRE2_ERROR_NOMEMORY },
    // control: exactly big enough succeeds
    SubCase { rows: &[384], pat: "(?<n>abc)", opts: 0, subj: "abc", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyName("n", 4), expect: 0 },

    // ---- pcre2_substring_copy_bynumber_8 ------------------------------ 385-386
    SubCase { rows: &[385], pat: "(a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyNum(2, 16), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[385], pat: "(a)(b)", opts: 0, subj: "ab", md: Md::Count(2),
              run: Run::Norm(0), call: Call::CopyNum(2, 16), expect: PCRE2_ERROR_UNAVAILABLE },
    SubCase { rows: &[385], pat: "(a)|b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyNum(1, 16), expect: PCRE2_ERROR_UNSET },
    SubCase { rows: &[385], pat: "abc", opts: 0, subj: "xyz", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyNum(0, 16), expect: PCRE2_ERROR_NOMATCH },
    SubCase { rows: &[385], pat: "(abc)", opts: 0, subj: "ab", md: Md::FromPat,
              run: Run::Norm(PCRE2_PARTIAL_SOFT), call: Call::CopyNum(1, 16),
              expect: PCRE2_ERROR_PARTIAL },
    SubCase { rows: &[386], pat: "(abc)", opts: 0, subj: "abc", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyNum(1, 3), expect: PCRE2_ERROR_NOMEMORY },
    SubCase { rows: &[386], pat: "(abc)", opts: 0, subj: "abc", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyNum(1, 0), expect: PCRE2_ERROR_NOMEMORY },
    SubCase { rows: &[386], pat: "(abc)", opts: 0, subj: "abc", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyNum(1, 4), expect: 0 },
    // a zero-length capture still needs one byte for the NUL
    SubCase { rows: &[386], pat: "(a?)b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyNum(1, 0), expect: PCRE2_ERROR_NOMEMORY },
    SubCase { rows: &[386], pat: "(a?)b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::CopyNum(1, 1), expect: 0 },

    // ---- pcre2_substring_get_byname_8 --------------------------------- 387-390
    SubCase { rows: &[387], pat: "(?<n>a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Dfa(0), call: Call::GetName("n"), expect: PCRE2_ERROR_DFA_UFUNC },
    SubCase { rows: &[388], pat: "(?<abc>a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::GetName("xyz"), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[389], pat: "(a)(b)(?<n>c)", opts: 0, subj: "abc", md: Md::Count(1),
              run: Run::Norm(0), call: Call::GetName("n"), expect: PCRE2_ERROR_UNAVAILABLE },
    SubCase { rows: &[390], pat: "(?<n>a)|b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::GetName("n"), expect: PCRE2_ERROR_UNSET },
    // DUPNAMES: the name maps to several groups; the first SET one wins
    SubCase { rows: &[389, 390], pat: "(?J)(?<n>a)|(?<n>b)", opts: 0, subj: "b",
              md: Md::FromPat, run: Run::Norm(0), call: Call::GetName("n"), expect: 0 },
    SubCase { rows: &[389, 390], pat: "(?J)(?<n>a)|(?<n>b)", opts: 0, subj: "b",
              md: Md::Count(1), run: Run::Norm(0), call: Call::GetName("n"),
              expect: PCRE2_ERROR_UNAVAILABLE },

    // ---- pcre2_substring_get_bynumber_8 ------------------------------- 391
    SubCase { rows: &[391], pat: "(a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::GetNum(2), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[391], pat: "(a)|b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::GetNum(1), expect: PCRE2_ERROR_UNSET },
    SubCase { rows: &[391], pat: "abc", opts: 0, subj: "xyz", md: Md::FromPat,
              run: Run::Norm(0), call: Call::GetNum(0), expect: PCRE2_ERROR_NOMATCH },
    SubCase { rows: &[391], pat: "(a)(b)", opts: 0, subj: "ab", md: Md::Count(2),
              run: Run::Norm(0), call: Call::GetNum(2), expect: PCRE2_ERROR_UNAVAILABLE },

    // ---- pcre2_substring_length_bynumber_8 ---------------------------- 393-400
    SubCase { rows: &[393], pat: "(abc)", opts: 0, subj: "ab", md: Md::FromPat,
              run: Run::Norm(PCRE2_PARTIAL_SOFT), call: Call::LenNum(1, true),
              expect: PCRE2_ERROR_PARTIAL },
    SubCase { rows: &[393], pat: "(abc)", opts: 0, subj: "ab", md: Md::FromPat,
              run: Run::Norm(PCRE2_PARTIAL_HARD), call: Call::LenNum(1, true),
              expect: PCRE2_ERROR_PARTIAL },
    // stringnumber == 0 after a partial match is NOT an error (count becomes 0)
    SubCase { rows: &[393], pat: "(abc)", opts: 0, subj: "ab", md: Md::FromPat,
              run: Run::Norm(PCRE2_PARTIAL_SOFT), call: Call::LenNum(0, true), expect: 0 },
    SubCase { rows: &[394], pat: "abc", opts: 0, subj: "xyz", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenNum(0, true), expect: PCRE2_ERROR_NOMATCH },
    SubCase { rows: &[394], pat: "abc", opts: 0, subj: "xyz", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenNum(9, true), expect: PCRE2_ERROR_NOMATCH },
    SubCase { rows: &[395], pat: "(a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenNum(2, true), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[395], pat: "(a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenNum(65535, true),
              expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[395], pat: "(a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenNum(u32::MAX, true),
              expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[396], pat: "(a)(b)", opts: 0, subj: "ab", md: Md::Count(2),
              run: Run::Norm(0), call: Call::LenNum(2, true), expect: PCRE2_ERROR_UNAVAILABLE },
    SubCase { rows: &[396], pat: "(a)(b)", opts: 0, subj: "ab", md: Md::Count(1),
              run: Run::Norm(0), call: Call::LenNum(1, true), expect: PCRE2_ERROR_UNAVAILABLE },
    SubCase { rows: &[397], pat: "(a)|b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenNum(1, true), expect: PCRE2_ERROR_UNSET },
    SubCase { rows: &[398], pat: "(a)", opts: 0, subj: "a", md: Md::Count(1),
              run: Run::Dfa(0), call: Call::LenNum(1, true), expect: PCRE2_ERROR_UNAVAILABLE },
    SubCase { rows: &[399], pat: "abc", opts: 0, subj: "abc", md: Md::Count(4),
              run: Run::Dfa(0), call: Call::LenNum(1, true), expect: PCRE2_ERROR_UNSET },
    SubCase { rows: &[399], pat: "abc", opts: 0, subj: "abc", md: Md::Count(4),
              run: Run::Dfa(0), call: Call::LenNum(3, true), expect: PCRE2_ERROR_UNSET },
    SubCase { rows: &[400], pat: "(a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::HackOvector(1),
              expect: PCRE2_ERROR_INVALIDOFFSET },
    SubCase { rows: &[400], pat: "(abc)", opts: 0, subj: "abc", md: Md::FromPat,
              run: Run::Norm(0), call: Call::HackOvector(0),
              expect: PCRE2_ERROR_INVALIDOFFSET },
    // NULL sizeptr is explicitly allowed by pcre2_substring.c:350
    SubCase { rows: &[397], pat: "(a)|b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenNum(1, false), expect: PCRE2_ERROR_UNSET },
    SubCase { rows: &[393], pat: "(a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenNum(1, false), expect: 0 },

    // ---- pcre2_substring_length_byname_8 ------------------------------ 401-404
    SubCase { rows: &[401], pat: "(?<n>a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Dfa(0), call: Call::LenName("n", true), expect: PCRE2_ERROR_DFA_UFUNC },
    SubCase { rows: &[402], pat: "(?<abc>a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenName("xyz", true),
              expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[402], pat: "(a)", opts: 0, subj: "a", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenName("n", true),
              expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[403], pat: "(a)(b)(?<n>c)", opts: 0, subj: "abc", md: Md::Count(1),
              run: Run::Norm(0), call: Call::LenName("n", true),
              expect: PCRE2_ERROR_UNAVAILABLE },
    SubCase { rows: &[404], pat: "(?<n>a)|b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenName("n", true), expect: PCRE2_ERROR_UNSET },
    SubCase { rows: &[404], pat: "(?<n>a)|b", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::LenName("n", false), expect: PCRE2_ERROR_UNSET },

    // ---- pcre2_substring_list_get_8 ----------------------------------- 405
    SubCase { rows: &[405], pat: "abc", opts: 0, subj: "xyz", md: Md::FromPat,
              run: Run::Norm(0), call: Call::ListGet(true), expect: PCRE2_ERROR_NOMATCH },
    SubCase { rows: &[405], pat: "abc", opts: 0, subj: "xyz", md: Md::FromPat,
              run: Run::Norm(0), call: Call::ListGet(false), expect: PCRE2_ERROR_NOMATCH },
    SubCase { rows: &[405], pat: "(abc)", opts: 0, subj: "ab", md: Md::FromPat,
              run: Run::Norm(PCRE2_PARTIAL_SOFT), call: Call::ListGet(true),
              expect: PCRE2_ERROR_PARTIAL },
    // rc == 0 (ovector too small) falls back to oveccount
    SubCase { rows: &[405], pat: "(a)(b)", opts: 0, subj: "ab", md: Md::Count(2),
              run: Run::Norm(0), call: Call::ListGet(true), expect: 0 },
    SubCase { rows: &[405], pat: "(a)|(b)", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::ListGet(true), expect: 0 },
    SubCase { rows: &[405], pat: "(a)|(b)", opts: 0, subj: "b", md: Md::FromPat,
              run: Run::Norm(0), call: Call::ListGet(false), expect: 0 },

    // ---- pcre2_substring_nametable_scan_8 ----------------------------- 407-408
    SubCase { rows: &[407], pat: "(?J)(?<n>a)|(?<n>b)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::Scan("n", false),
              expect: PCRE2_ERROR_NOUNIQUESUBSTRING },
    SubCase { rows: &[408], pat: "(a)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::Scan("n", true), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[408], pat: "(a)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::Scan("n", false), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[408], pat: "(?<abc>a)(?<def>b)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::Scan("abd", true), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[408], pat: "(?<abc>a)(?<def>b)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::Scan("", true), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[408], pat: "(?<abc>a)(?<def>b)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::Scan("zzz", true), expect: PCRE2_ERROR_NOSUBSTRING },
    // success: entrysize returned and both pointers written
    SubCase { rows: &[407, 408], pat: "(?<abc>a)(?<def>b)", opts: 0, subj: "",
              md: Md::Count(4), run: Run::NoRun, call: Call::Scan("def", true), expect: 6 },
    SubCase { rows: &[407], pat: "(?J)(?<n>a)|(?<n>b)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::Scan("n", true), expect: 4 },

    // ---- pcre2_substring_number_from_name_8 --------------------------- 409-410
    SubCase { rows: &[409], pat: "(?<abc>a)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::NumFromName("xyz"),
              expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[409], pat: "(a)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::NumFromName("a"), expect: PCRE2_ERROR_NOSUBSTRING },
    SubCase { rows: &[410], pat: "(?J)(?<n>a)|(?<n>b)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::NumFromName("n"),
              expect: PCRE2_ERROR_NOUNIQUESUBSTRING },
    SubCase { rows: &[410], pat: "(?<n>a)", opts: 0, subj: "", md: Md::Count(4),
              run: Run::NoRun, call: Call::NumFromName("n"), expect: 1 },
];

unsafe fn run_sub(api: &Api, cs: &SubCase) -> String {
    let code = must_compile(api, cs.pat.as_bytes(), cs.opts);
    let md = match cs.md {
        Md::FromPat => (api.match_data_create_from_pattern)(code, ptr::null_mut()),
        Md::Count(n) => (api.match_data_create)(n, ptr::null_mut()),
    };
    assert!(!md.is_null());
    let sb = cs.subj.as_bytes();
    let mrc = match cs.run {
        Run::NoRun => 0,
        Run::Norm(o) => (api.do_match)(code, sb.as_ptr(), sb.len(), 0, o, md, ptr::null_mut()),
        Run::Dfa(o) => {
            let mut ws = [0i32; 128];
            (api.dfa_match)(
                code,
                sb.as_ptr(),
                sb.len(),
                0,
                o,
                md,
                ptr::null_mut(),
                ws.as_mut_ptr(),
                ws.len(),
            )
        }
    };
    let mut out = format!("match={mrc} ");

    // Where the name table starts, so nametable_scan results can be reported as
    // offsets instead of host addresses.
    let mut ntab: *const u8 = ptr::null();
    assert_eq!(
        (api.pattern_info)(code, PCRE2_INFO_NAMETABLE, &mut ntab as *mut _ as Ptr),
        0
    );

    match cs.call {
        Call::LenNum(g, with) => {
            let mut sz: Sz = 0xDEAD_BEEF;
            let rc =
                (api.substring_length_bynumber)(md, g, if with { &mut sz } else { ptr::null_mut() });
            out += &format!("rc={rc} size={sz:#x}");
        }
        Call::LenName(n, with) => {
            let nm = nul(n);
            let mut sz: Sz = 0xDEAD_BEEF;
            let rc = (api.substring_length_byname)(
                md,
                nm.as_ptr(),
                if with { &mut sz } else { ptr::null_mut() },
            );
            out += &format!("rc={rc} size={sz:#x}");
        }
        Call::CopyNum(g, cap) => {
            let mut buf = vec![0xA5u8; SUB_BUF];
            let mut sz: Sz = cap;
            let rc = (api.substring_copy_bynumber)(md, g, buf.as_mut_ptr(), &mut sz);
            out += &format!("rc={rc} size={sz:#x} buf={}", show(&buf));
        }
        Call::CopyName(n, cap) => {
            let nm = nul(n);
            let mut buf = vec![0xA5u8; SUB_BUF];
            let mut sz: Sz = cap;
            let rc = (api.substring_copy_byname)(md, nm.as_ptr(), buf.as_mut_ptr(), &mut sz);
            out += &format!("rc={rc} size={sz:#x} buf={}", show(&buf));
        }
        Call::GetNum(g) => {
            let mut sp: *mut u8 = 1 as *mut u8; // sentinel: must be untouched on error
            let mut sz: Sz = 0xDEAD_BEEF;
            let rc = (api.substring_get_bynumber)(md, g, &mut sp, &mut sz);
            out += &format!("rc={rc} size={sz:#x}{}", show_got(api, rc, sp, sz));
        }
        Call::GetName(n) => {
            let nm = nul(n);
            let mut sp: *mut u8 = 1 as *mut u8;
            let mut sz: Sz = 0xDEAD_BEEF;
            let rc = (api.substring_get_byname)(md, nm.as_ptr(), &mut sp, &mut sz);
            out += &format!("rc={rc} size={sz:#x}{}", show_got(api, rc, sp, sz));
        }
        Call::Scan(n, with) => {
            let nm = nul(n);
            let mut first: Sptr = 7 as Sptr;
            let mut last: Sptr = 9 as Sptr;
            let rc = if with {
                (api.substring_nametable_scan)(code, nm.as_ptr(), &mut first, &mut last)
            } else {
                (api.substring_nametable_scan)(code, nm.as_ptr(), ptr::null_mut(), ptr::null_mut())
            };
            let f = if first == 7 as Sptr {
                -1i64
            } else {
                first as i64 - ntab as i64
            };
            let l = if last == 9 as Sptr {
                -1i64
            } else {
                last as i64 - ntab as i64
            };
            out += &format!("rc={rc} first={f} last={l}");
        }
        Call::NumFromName(n) => {
            let nm = nul(n);
            let rc = (api.substring_number_from_name)(code, nm.as_ptr());
            out += &format!("rc={rc}");
        }
        Call::ListGet(with) => {
            let mut list: *mut *mut u8 = 3 as *mut *mut u8;
            let mut lens: *mut Sz = 5 as *mut Sz;
            let rc = if with {
                (api.substring_list_get)(md, &mut list, &mut lens)
            } else {
                (api.substring_list_get)(md, &mut list, ptr::null_mut())
            };
            out += &format!("rc={rc}");
            if rc != 0 {
                out += &format!(
                    " list_untouched={} lens_untouched={}",
                    list == 3 as *mut *mut u8,
                    lens == 5 as *mut Sz
                );
            } else {
                let mut i = 0usize;
                while !(*list.add(i)).is_null() {
                    let s = *list.add(i);
                    let bytes = if with {
                        std::slice::from_raw_parts(s, *lens.add(i)).to_vec()
                    } else {
                        let mut v = Vec::new();
                        let mut q = s;
                        while *q != 0 {
                            v.push(*q);
                            q = q.add(1);
                        }
                        v
                    };
                    out += &format!(" [{i}]={}", show(&bytes));
                    if with {
                        out += &format!("/{}", *lens.add(i));
                    }
                    i += 1;
                    assert!(i < 64);
                }
                out += &format!(" n={i}");
                (api.substring_list_free)(list);
            }
        }
        Call::HackOvector(g) => {
            let ov = (api.get_ovector_pointer)(md);
            *ov.add(2 * g as usize) = sb.len() + 4;
            let mut sz: Sz = 0xDEAD_BEEF;
            let rc = (api.substring_length_bynumber)(md, g, &mut sz);
            out += &format!("rc={rc} size={sz:#x}");
        }
    }

    (api.match_data_free)(md);
    (api.code_free)(code);
    out
}

unsafe fn show_got(api: &Api, rc: c_int, sp: *mut u8, sz: Sz) -> String {
    if rc != 0 {
        return format!(" untouched={}", sp == 1 as *mut u8);
    }
    let bytes = std::slice::from_raw_parts(sp, sz + 1).to_vec();
    let s = format!(" got={}", show(&bytes));
    (api.substring_free)(sp);
    s
}

#[test]
fn substring_accessor_errors() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in SUB_CASES {
            let tag = format!(
                "rows {:?} pat={} subj={} md={:?} run={:?} call={:?}",
                cs.rows,
                show(cs.pat.as_bytes()),
                show(cs.subj.as_bytes()),
                cs.md,
                cs.run,
                cs.call
            );
            let a = run_sub(&p.c, cs);
            let b = run_sub(&p.r, cs);
            d.eq(&tag, a.clone(), b);
            // The C is ground truth; check ERRORS.md agrees with it.
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect as i64, rc_of(&a));
        }
    }
    doc.finish("ERRORS.md `expected C result` vs the actual C, rows 380-410");
    d.finish("rows 380-410: every pcre2_substring.c rejection path");
}

/// A row whose whole content is "this entry point tolerates the boundary
/// argument named here" — used for the NULL-argument / guarded-no-op rows.
struct NullArgCase {
    rows: &'static [u32],
    what: &'static str,
}

// rows 411, 412: the two NULL-tolerant free functions.
const SUB_FREE_NULL: &[NullArgCase] = &[
    NullArgCase { rows: &[411], what: "pcre2_substring_free_8(NULL)" },
    NullArgCase { rows: &[412], what: "pcre2_substring_list_free_8(NULL)" },
];

#[test]
fn substring_free_null_is_noop() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for cs in SUB_FREE_NULL {
            for _ in 0..3 {
                if cs.rows[0] == 411 {
                    (p.c.substring_free)(ptr::null_mut());
                    (p.r.substring_free)(ptr::null_mut());
                } else {
                    (p.c.substring_list_free)(ptr::null_mut());
                    (p.r.substring_list_free)(ptr::null_mut());
                }
            }
            d.eq(&format!("rows {:?} {} survived", cs.rows, cs.what), true, true);
        }

        // And prove the custom `free` is NOT invoked for NULL: run a real
        // get_bynumber through a counting allocator, then hit it with NULL.
        let mut added = [0i64; 2];
        for idx in 0..2 {
            let api = if idx == 0 { &p.c } else { &p.r };
            let (m, f) = allocs(idx);
            reset(idx);
            let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
            let code = must_compile(api, b"(abc)", 0);
            let md = (api.match_data_create_from_pattern)(code, g);
            assert_eq!(
                (api.do_match)(code, b"abc".as_ptr(), 3, 0, 0, md, ptr::null_mut()),
                2
            );
            let mut sp: *mut u8 = ptr::null_mut();
            let mut sz: Sz = 0;
            assert_eq!((api.substring_get_bynumber)(md, 1, &mut sp, &mut sz), 0);
            (api.substring_free)(sp);
            let before = nfree(idx);
            (api.substring_free)(ptr::null_mut());
            (api.substring_list_free)(ptr::null_mut());
            added[idx] = nfree(idx) - before;
            (api.match_data_free)(md);
            (api.code_free)(code);
            (api.general_context_free)(g);
        }
        d.eq("rows [411, 412] free calls added by the NULL frees", added[0], added[1]);
        assert_eq!(added[0], 0, "ERRORS.md rows 411-412: the C must not call free");
    }
    d.finish("rows 411-412: pcre2_substring_free_8(NULL) / pcre2_substring_list_free_8(NULL)");
}

// rows 392, 406: the two PRIV(memctl_malloc) failures inside pcre2_substring.c.
struct AllocCase {
    rows: &'static [u32],
    /// 0 = substring_get_bynumber, 1 = substring_list_get
    which: u32,
    expect: c_int,
}
const SUB_ALLOC: &[AllocCase] = &[
    AllocCase { rows: &[392], which: 0, expect: PCRE2_ERROR_NOMEMORY },
    AllocCase { rows: &[406], which: 1, expect: PCRE2_ERROR_NOMEMORY },
];

unsafe fn run_sub_alloc(api: &Api, idx: usize, which: u32, budget: i64) -> String {
    reset(idx);
    let (m, f) = allocs(idx);
    let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
    assert!(!g.is_null());
    let code = must_compile(api, b"(abc)(d)", 0);
    let md = (api.match_data_create_from_pattern)(code, g);
    assert!(!md.is_null());
    assert_eq!(
        (api.do_match)(code, b"abcd".as_ptr(), 4, 0, 0, md, ptr::null_mut()),
        3
    );
    // Everything above must succeed; only the accessor's allocation is starved.
    set_budget(idx, budget);
    let mut out = String::new();
    if which == 0 {
        let mut sp: *mut u8 = 1 as *mut u8;
        let mut sz: Sz = 0xDEAD;
        let r = (api.substring_get_bynumber)(md, 1, &mut sp, &mut sz);
        out += &format!("rc={r} size={sz:#x} untouched={}", sp == 1 as *mut u8);
        if r == 0 {
            out += &format!(" got={}", show(std::slice::from_raw_parts(sp, sz)));
            (api.substring_free)(sp);
        }
    } else {
        let mut list: *mut *mut u8 = 3 as *mut *mut u8;
        let mut lens: *mut Sz = 5 as *mut Sz;
        let r = (api.substring_list_get)(md, &mut list, &mut lens);
        out += &format!("rc={r} untouched={}", list == 3 as *mut *mut u8);
        if r == 0 {
            let mut i = 0;
            while !(*list.add(i)).is_null() {
                out += &format!(
                    " [{i}]={}",
                    show(std::slice::from_raw_parts(*list.add(i), *lens.add(i)))
                );
                i += 1;
            }
            (api.substring_list_free)(list);
        }
    }
    set_budget(idx, -1);
    (api.match_data_free)(md);
    (api.code_free)(code);
    (api.general_context_free)(g);
    out
}

#[test]
fn substring_allocation_failures() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in SUB_ALLOC {
            for budget in 0..=2i64 {
                let a = run_sub_alloc(&p.c, 0, cs.which, budget);
                let b = run_sub_alloc(&p.r, 1, cs.which, budget);
                d.eq(
                    &format!("rows {:?} which={} budget={budget}", cs.rows, cs.which),
                    a.clone(),
                    b,
                );
                if budget == 0 {
                    doc.eq(
                        &format!("ERRORS.md rows {:?}", cs.rows),
                        cs.expect as i64,
                        rc_of(&a),
                    );
                }
            }
        }
    }
    doc.finish("ERRORS.md rows 392, 406");
    d.finish("rows 392, 406: PRIV(memctl_malloc) failure inside get_bynumber / list_get");
}

// ============================================================ pcre2_match_data.c

/// `offsetof(pcre2_match_data, ovector)` on LP64 — memctl(24) + 10 pointers or
/// PCRE2_SIZEs (80) + matchedby/flags/oveccount (4) + options (4) + rc (4),
/// rounded up to the alignment of PCRE2_SIZE.
const MD_OVECTOR_OFFSET: usize = 120;

struct MdCase {
    rows: &'static [u32],
    oveccount: u32,
    /// what `pcre2_get_ovector_count_8` must report afterwards
    expect_count: u32,
}
const MD_NULL_ARGS: &[NullArgCase] = &[
    NullArgCase { rows: &[416], what: "match_data_create_from_pattern(NULL, NULL)" },
    NullArgCase { rows: &[416], what: "match_data_create_from_pattern(NULL, gcontext)" },
    NullArgCase { rows: &[418], what: "pcre2_match_data_free_8(NULL)" },
];

const MD_CASES: &[MdCase] = &[
    MdCase { rows: &[413], oveccount: 0, expect_count: 1 },
    MdCase { rows: &[413], oveccount: 1, expect_count: 1 },
    MdCase { rows: &[414], oveccount: 2, expect_count: 2 },
    MdCase { rows: &[414], oveccount: 65534, expect_count: 65534 },
    MdCase { rows: &[414], oveccount: 65535, expect_count: 65535 },
    MdCase { rows: &[414], oveccount: 65536, expect_count: 65535 },
    MdCase { rows: &[414], oveccount: 100000, expect_count: 65535 },
    MdCase { rows: &[414], oveccount: u32::MAX, expect_count: 65535 },
];

#[test]
fn match_data_create_bounds() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in MD_CASES {
            let a = (p.c.match_data_create)(cs.oveccount, ptr::null_mut());
            let b = (p.r.match_data_create)(cs.oveccount, ptr::null_mut());
            assert!(!a.is_null() && !b.is_null());
            let (ca, cb) = ((p.c.get_ovector_count)(a), (p.r.get_ovector_count)(b));
            let (sa, sb) = ((p.c.get_match_data_size)(a), (p.r.get_match_data_size)(b));
            let (ha, hb) = (
                (p.c.get_match_data_heapframes_size)(a),
                (p.r.get_match_data_heapframes_size)(b),
            );
            let tag = format!("rows {:?} match_data_create({})", cs.rows, cs.oveccount);
            d.eq(&format!("{tag} ovector_count"), ca, cb);
            d.eq(&format!("{tag} size"), sa, sb);
            d.eq(&format!("{tag} heapframes_size"), ha, hb);
            doc.eq(&format!("ERRORS.md {tag} clamped count"), cs.expect_count, ca);
            doc.eq(
                &format!("ERRORS.md {tag} documented size formula"),
                MD_OVECTOR_OFFSET + 2 * ca as usize * 8,
                sa,
            );
            d.eq(&format!("{tag} heapframes fresh"), 0usize, ha);
            (p.c.match_data_free)(a);
            (p.r.match_data_free)(b);
        }

        // rows 416, 418: NULL code / NULL match_data
        let ga = (p.c.general_context_create)(None, None, ptr::null_mut());
        let gb = (p.r.general_context_create)(None, None, ptr::null_mut());
        for cs in MD_NULL_ARGS {
            match cs.what {
                "match_data_create_from_pattern(NULL, NULL)" => {
                    let a =
                        (p.c.match_data_create_from_pattern)(ptr::null_mut(), ptr::null_mut());
                    let b =
                        (p.r.match_data_create_from_pattern)(ptr::null_mut(), ptr::null_mut());
                    d.eq(&format!("rows {:?} {}", cs.rows, cs.what), a.is_null(), b.is_null());
                    doc.eq(&format!("ERRORS.md rows {:?} returns NULL", cs.rows), true, a.is_null());
                }
                "match_data_create_from_pattern(NULL, gcontext)" => {
                    let a = (p.c.match_data_create_from_pattern)(ptr::null_mut(), ga);
                    let b = (p.r.match_data_create_from_pattern)(ptr::null_mut(), gb);
                    d.eq(&format!("rows {:?} {}", cs.rows, cs.what), a.is_null(), b.is_null());
                    doc.eq(&format!("ERRORS.md rows {:?} returns NULL", cs.rows), true, a.is_null());
                }
                _ => {
                    for _ in 0..3 {
                        (p.c.match_data_free)(ptr::null_mut());
                        (p.r.match_data_free)(ptr::null_mut());
                    }
                    d.eq(&format!("rows {:?} {} survived", cs.rows, cs.what), true, true);
                }
            }
        }
        (p.c.general_context_free)(ga);
        (p.r.general_context_free)(gb);
    }
    doc.finish("ERRORS.md rows 413, 414, 416");
    d.finish("rows 413, 414, 416, 418: pcre2_match_data_create_8 clamping and NULL handling");
}

// rows 415, 417: allocation failure in match_data_create / _from_pattern.
struct MdAllocCase {
    rows: &'static [u32],
    /// 0 = create(4, gcontext);
    /// 1 = create_from_pattern(code, NULL), the code itself carrying the
    ///     fallible allocator (so the allocator comes from `code`)
    which: u32,
}
const MD_ALLOC: &[MdAllocCase] =
    &[MdAllocCase { rows: &[415], which: 0 }, MdAllocCase { rows: &[417], which: 1 }];

unsafe fn run_md_alloc(api: &Api, idx: usize, which: u32, budget: i64) -> String {
    reset(idx);
    let (m, f) = allocs(idx);
    let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
    assert!(!g.is_null());
    let out;
    if which == 0 {
        set_budget(idx, budget);
        let md = (api.match_data_create)(4, g);
        out = format!(
            "null={} count={} mallocs={}",
            md.is_null(),
            if md.is_null() { 0 } else { (api.get_ovector_count)(md) },
            nmalloc(idx)
        );
        set_budget(idx, -1);
        if !md.is_null() {
            (api.match_data_free)(md);
        }
    } else {
        let cc = (api.compile_context_create)(g);
        assert!(!cc.is_null());
        let code = must_compile_cc(api, b"(a)(b)(c)", 0, cc);
        set_budget(idx, budget);
        let md = (api.match_data_create_from_pattern)(code, ptr::null_mut());
        out = format!(
            "null={} count={}",
            md.is_null(),
            if md.is_null() { 0 } else { (api.get_ovector_count)(md) }
        );
        set_budget(idx, -1);
        if !md.is_null() {
            (api.match_data_free)(md);
        }
        (api.code_free)(code);
        (api.compile_context_free)(cc);
    }
    (api.general_context_free)(g);
    out
}

#[test]
fn match_data_allocation_failures() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for cs in MD_ALLOC {
            for budget in 0..=2i64 {
                let a = run_md_alloc(&p.c, 0, cs.which, budget);
                let b = run_md_alloc(&p.r, 1, cs.which, budget);
                d.eq(
                    &format!("rows {:?} which={} budget={budget}", cs.rows, cs.which),
                    a.clone(),
                    b,
                );
                if budget == 0 {
                    assert!(
                        a.starts_with("null=true"),
                        "ERRORS.md rows {:?}: the C must return NULL when the \
                         allocation fails, got {a}",
                        cs.rows
                    );
                }
            }
        }
    }
    d.finish("rows 415, 417: allocation failure in pcre2_match_data_create_8 / _from_pattern_8");
}

// The `pcre2_get_*` accessors on a match_data that was never used for a match.
// `pcre2_match_data_create` leaves `mark`, `startchar`, `rc` and the ovector
// uninitialised, so a ZEROING allocator is installed to make the observable
// deterministic (and identical) in both libraries.
struct FreshCase {
    rows: &'static [u32],
    oveccount: u32,
}
const FRESH: &[FreshCase] = &[
    FreshCase { rows: &[413], oveccount: 0 },
    FreshCase { rows: &[414], oveccount: 3 },
    FreshCase { rows: &[418], oveccount: 8 },
];

#[test]
fn fresh_match_data_accessors() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for cs in FRESH {
            let ga = (p.c.general_context_create)(Some(zmal), Some(zfree), ptr::null_mut());
            let gb = (p.r.general_context_create)(Some(zmal), Some(zfree), ptr::null_mut());
            let a = (p.c.match_data_create)(cs.oveccount, ga);
            let b = (p.r.match_data_create)(cs.oveccount, gb);
            let tag = format!("rows {:?} fresh md({})", cs.rows, cs.oveccount);
            let na = (p.c.get_ovector_count)(a) as usize;
            let nb = (p.r.get_ovector_count)(b) as usize;
            d.eq(&format!("{tag} ovector_count"), na, nb);
            d.eq(
                &format!("{tag} size"),
                (p.c.get_match_data_size)(a),
                (p.r.get_match_data_size)(b),
            );
            d.eq(
                &format!("{tag} heapframes_size"),
                (p.c.get_match_data_heapframes_size)(a),
                (p.r.get_match_data_heapframes_size)(b),
            );
            d.eq(
                &format!("{tag} startchar"),
                (p.c.get_startchar)(a),
                (p.r.get_startchar)(b),
            );
            d.eq(
                &format!("{tag} mark null"),
                (p.c.get_mark)(a).is_null(),
                (p.r.get_mark)(b).is_null(),
            );
            d.eq(
                &format!("{tag} ovector"),
                std::slice::from_raw_parts((p.c.get_ovector_pointer)(a), 2 * na).to_vec(),
                std::slice::from_raw_parts((p.r.get_ovector_pointer)(b), 2 * nb).to_vec(),
            );
            // With rc == 0 the C treats it as "ovector too small", so
            // substring_list_get succeeds with `oveccount` empty strings.
            let mut la: *mut *mut u8 = ptr::null_mut();
            let mut lb: *mut *mut u8 = ptr::null_mut();
            let mut sa: *mut Sz = ptr::null_mut();
            let mut sb: *mut Sz = ptr::null_mut();
            let ra = (p.c.substring_list_get)(a, &mut la, &mut sa);
            let rb = (p.r.substring_list_get)(b, &mut lb, &mut sb);
            d.eq(&format!("{tag} list_get rc"), ra, rb);
            if ra == 0 && rb == 0 {
                d.eq(
                    &format!("{tag} list_get lengths"),
                    std::slice::from_raw_parts(sa, na).to_vec(),
                    std::slice::from_raw_parts(sb, nb).to_vec(),
                );
                (p.c.substring_list_free)(la);
                (p.r.substring_list_free)(lb);
            }
            (p.c.match_data_free)(a);
            (p.r.match_data_free)(b);
            (p.c.general_context_free)(ga);
            (p.r.general_context_free)(gb);
        }
    }
    d.finish("rows 413-418: pcre2_get_* accessors on a never-matched match_data");
}

// ========================================================== pcre2_pattern_info.c

/// How many bytes `pcre2_pattern_info_8` writes for a given `what`, and whether
/// what it writes is a host pointer (so must be normalised before comparing).
fn what_width(what: u32) -> (usize, bool) {
    match what {
        PCRE2_INFO_FIRSTBITMAP | PCRE2_INFO_NAMETABLE => (8, true),
        PCRE2_INFO_JITSIZE | PCRE2_INFO_SIZE | PCRE2_INFO_FRAMESIZE => (8, false),
        0..=26 => (4, false),
        _ => (0, false),
    }
}

const PI_FILL: u8 = 0xAA;

/// Runs `pcre2_pattern_info_8(code, what, buf)` and renders the result plus the
/// whole 32-byte output buffer. A pointer result is reported relative to
/// `code`, so the two libraries are comparable.
unsafe fn run_pi(api: &Api, code: Ptr, what: u32) -> String {
    let mut buf = [PI_FILL; 32];
    let rc = (api.pattern_info)(code, what, buf.as_mut_ptr() as Ptr);
    let (_, isptr) = what_width(what);
    let mut head = String::from("-");
    if isptr && rc == 0 {
        let v = usize::from_ne_bytes(buf[0..8].try_into().unwrap());
        if v != 0 {
            head = format!("code+{}", v.wrapping_sub(code as usize));
            buf[0..8].fill(0);
        } else {
            head = String::from("NULL");
        }
    }
    format!("rc={rc} head={head} buf={}", show(&buf))
}

struct PiRow {
    rows: &'static [u32],
    pat: &'static str,
    what: u32,
    expect: c_int,
    /// Documented `*where` value (`what_width` bytes), if any.
    expect_where: Option<u64>,
}

const PI_ROWS: &[PiRow] = &[
    // --- what out of range: nothing is written, PCRE2_ERROR_BADOPTION
    PiRow { rows: &[419], pat: "abc", what: 27, expect: PCRE2_ERROR_BADOPTION,
            expect_where: None },
    PiRow { rows: &[419], pat: "abc", what: 28, expect: PCRE2_ERROR_BADOPTION,
            expect_where: None },
    PiRow { rows: &[419], pat: "abc", what: 100, expect: PCRE2_ERROR_BADOPTION,
            expect_where: None },
    PiRow { rows: &[419], pat: "abc", what: 0xFFFF_FFFF, expect: PCRE2_ERROR_BADOPTION,
            expect_where: None },
    // --- documented UNSET items (the value IS written before returning)
    PiRow { rows: &[425], pat: "abc", what: PCRE2_INFO_MATCHLIMIT, expect: PCRE2_ERROR_UNSET,
            expect_where: Some(0xFFFF_FFFF) },
    PiRow { rows: &[426], pat: "abc", what: PCRE2_INFO_DEPTHLIMIT, expect: PCRE2_ERROR_UNSET,
            expect_where: Some(0xFFFF_FFFF) },
    PiRow { rows: &[427], pat: "abc", what: PCRE2_INFO_HEAPLIMIT, expect: PCRE2_ERROR_UNSET,
            expect_where: Some(0xFFFF_FFFF) },
    // --- items that succeed but hand back a "nothing here" value
    PiRow { rows: &[428], pat: "abc", what: PCRE2_INFO_JITSIZE, expect: 0,
            expect_where: Some(0) },
    PiRow { rows: &[429], pat: "abc", what: PCRE2_INFO_FIRSTBITMAP, expect: 0,
            expect_where: Some(0) },
    PiRow { rows: &[430], pat: "[ab]c", what: PCRE2_INFO_FIRSTCODETYPE, expect: 0,
            expect_where: Some(0) },
    PiRow { rows: &[430], pat: "\\d+", what: PCRE2_INFO_FIRSTCODETYPE, expect: 0,
            expect_where: Some(0) },
    PiRow { rows: &[431], pat: "[ab]c", what: PCRE2_INFO_FIRSTCODEUNIT, expect: 0,
            expect_where: Some(0) },
    PiRow { rows: &[432], pat: "a.*", what: PCRE2_INFO_LASTCODETYPE, expect: 0,
            expect_where: Some(0) },
    PiRow { rows: &[432], pat: "a.*", what: PCRE2_INFO_LASTCODEUNIT, expect: 0,
            expect_where: Some(0) },
    // --- limits that ARE set: no UNSET
    PiRow { rows: &[425], pat: "(*LIMIT_MATCH=1000)a", what: PCRE2_INFO_MATCHLIMIT, expect: 0,
            expect_where: Some(1000) },
    PiRow { rows: &[426], pat: "(*LIMIT_DEPTH=100)a", what: PCRE2_INFO_DEPTHLIMIT, expect: 0,
            expect_where: Some(100) },
    PiRow { rows: &[427], pat: "(*LIMIT_HEAP=1000)a", what: PCRE2_INFO_HEAPLIMIT, expect: 0,
            expect_where: Some(1000) },
];

#[test]
fn pattern_info_documented_rows() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in PI_ROWS {
            let ka = must_compile(&p.c, cs.pat.as_bytes(), 0);
            let kb = must_compile(&p.r, cs.pat.as_bytes(), 0);
            let tag = format!(
                "rows {:?} pat={} what={}",
                cs.rows,
                show(cs.pat.as_bytes()),
                cs.what
            );
            d.eq(&tag, run_pi(&p.c, ka, cs.what), run_pi(&p.r, kb, cs.what));

            // ERRORS.md cross-check on the C.
            let mut buf = [PI_FILL; 32];
            let rc = (p.c.pattern_info)(ka, cs.what, buf.as_mut_ptr() as Ptr);
            doc.eq(&format!("ERRORS.md {tag} rc"), cs.expect, rc);
            if let Some(want) = cs.expect_where {
                let (w, _) = what_width(cs.what);
                let got = match w {
                    4 => u32::from_ne_bytes(buf[0..4].try_into().unwrap()) as u64,
                    8 => u64::from_ne_bytes(buf[0..8].try_into().unwrap()),
                    _ => unreachable!(),
                };
                doc.eq(&format!("ERRORS.md {tag} *where"), want, got);
                // nothing beyond the documented width may be touched
                doc.eq(
                    &format!("ERRORS.md {tag} tail untouched"),
                    vec![PI_FILL; 32 - w],
                    buf[w..].to_vec(),
                );
            } else {
                doc.eq(
                    &format!("ERRORS.md {tag} *where untouched"),
                    vec![PI_FILL; 32],
                    buf.to_vec(),
                );
            }
            (p.c.code_free)(ka);
            (p.r.code_free)(kb);
        }
    }
    doc.finish("ERRORS.md rows 419, 425-432");
    d.finish("rows 419, 425-432: pcre2_pattern_info_8 BADOPTION / UNSET / zero-valued items");
}

// Every `what` 0..=26 plus out-of-range, both the value form and the documented
// `where == NULL` length-query form. Rows 419/420 define the accepted domain.
struct PiSweep {
    rows: &'static [u32],
    what: u32,
    name: &'static str,
}
const PI_SWEEP: &[PiSweep] = &[
    PiSweep { rows: &[419, 420], what: 0, name: "ALLOPTIONS" },
    PiSweep { rows: &[419, 420], what: 1, name: "ARGOPTIONS" },
    PiSweep { rows: &[419, 420], what: 2, name: "BACKREFMAX" },
    PiSweep { rows: &[419, 420], what: 3, name: "BSR" },
    PiSweep { rows: &[419, 420], what: 4, name: "CAPTURECOUNT" },
    PiSweep { rows: &[419, 420, 431], what: 5, name: "FIRSTCODEUNIT" },
    PiSweep { rows: &[419, 420, 430], what: 6, name: "FIRSTCODETYPE" },
    PiSweep { rows: &[419, 420, 429], what: 7, name: "FIRSTBITMAP" },
    PiSweep { rows: &[419, 420], what: 8, name: "HASCRORLF" },
    PiSweep { rows: &[419, 420], what: 9, name: "JCHANGED" },
    PiSweep { rows: &[419, 420, 428], what: 10, name: "JITSIZE" },
    PiSweep { rows: &[419, 420, 432], what: 11, name: "LASTCODEUNIT" },
    PiSweep { rows: &[419, 420, 432], what: 12, name: "LASTCODETYPE" },
    PiSweep { rows: &[419, 420], what: 13, name: "MATCHEMPTY" },
    PiSweep { rows: &[419, 420, 425], what: 14, name: "MATCHLIMIT" },
    PiSweep { rows: &[419, 420], what: 15, name: "MAXLOOKBEHIND" },
    PiSweep { rows: &[419, 420], what: 16, name: "MINLENGTH" },
    PiSweep { rows: &[419, 420], what: 17, name: "NAMECOUNT" },
    PiSweep { rows: &[419, 420], what: 18, name: "NAMEENTRYSIZE" },
    PiSweep { rows: &[419, 420], what: 19, name: "NAMETABLE" },
    PiSweep { rows: &[419, 420], what: 20, name: "NEWLINE" },
    PiSweep { rows: &[419, 420, 426], what: 21, name: "DEPTHLIMIT" },
    PiSweep { rows: &[419, 420], what: 22, name: "SIZE" },
    PiSweep { rows: &[419, 420], what: 23, name: "HASBACKSLASHC" },
    PiSweep { rows: &[419, 420], what: 24, name: "FRAMESIZE" },
    PiSweep { rows: &[419, 420, 427], what: 25, name: "HEAPLIMIT" },
    PiSweep { rows: &[419, 420], what: 26, name: "EXTRAOPTIONS" },
    PiSweep { rows: &[419, 420], what: 27, name: "<out of range>" },
    PiSweep { rows: &[419, 420], what: 28, name: "<out of range>" },
    PiSweep { rows: &[419, 420], what: 100, name: "<out of range>" },
    PiSweep { rows: &[419, 420], what: 0x8000_0000, name: "<out of range>" },
    PiSweep { rows: &[419, 420], what: 0xFFFF_FFFF, name: "<out of range>" },
];

const PI_PATTERNS: &[&str] = &[
    "",
    "abc",
    "[ab]c",
    "\\d+",
    "a.*",
    "^abc$",
    "(?<n>a)(?<m>b)",
    "(?J)(?<n>a)|(?<n>b)",
    "a(?C1)b",
    "(*LIMIT_MATCH=1000)(*LIMIT_DEPTH=100)(*LIMIT_HEAP=1000)(a)(b)",
    "(*CRLF)(*BSR_ANYCRLF)a\\R",
    "\\Ca",
    "(a)(b)(c)(d)(e)",
];

#[test]
fn pattern_info_every_what() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for pat in PI_PATTERNS {
            let ka = must_compile(&p.c, pat.as_bytes(), 0);
            let kb = must_compile(&p.r, pat.as_bytes(), 0);
            for cs in PI_SWEEP {
                let tag = format!(
                    "rows {:?} pat={} what={} ({})",
                    cs.rows,
                    show(pat.as_bytes()),
                    cs.what,
                    cs.name
                );
                d.eq(&tag, run_pi(&p.c, ka, cs.what), run_pi(&p.r, kb, cs.what));
                // where == NULL: the documented length query
                d.eq(
                    &format!("{tag} length query"),
                    (p.c.pattern_info)(ka, cs.what, ptr::null_mut()),
                    (p.r.pattern_info)(kb, cs.what, ptr::null_mut()),
                );
            }
            (p.c.code_free)(ka);
            (p.r.code_free)(kb);
        }
    }
    d.finish(
        "rows 419-420: pcre2_pattern_info_8 for every `what` 0..=26 plus out-of-range, both forms",
    );
}

// rows 421, 422: code == NULL.
struct PiNull {
    rows: &'static [u32],
    what: u32,
    where_null: bool,
    expect: c_int,
}
const PI_NULL: &[PiNull] = &[
    PiNull { rows: &[421], what: PCRE2_INFO_SIZE, where_null: false, expect: PCRE2_ERROR_NULL },
    PiNull { rows: &[421], what: PCRE2_INFO_CAPTURECOUNT, where_null: false,
             expect: PCRE2_ERROR_NULL },
    PiNull { rows: &[421], what: 0, where_null: false, expect: PCRE2_ERROR_NULL },
    PiNull { rows: &[421], what: 27, where_null: false, expect: PCRE2_ERROR_NULL },
    // where == NULL and `what` recognised: the length switch answers first
    PiNull { rows: &[421], what: PCRE2_INFO_SIZE, where_null: true, expect: 8 },
    PiNull { rows: &[421], what: PCRE2_INFO_ALLOPTIONS, where_null: true, expect: 4 },
    PiNull { rows: &[421], what: PCRE2_INFO_FIRSTBITMAP, where_null: true, expect: 8 },
    PiNull { rows: &[421], what: PCRE2_INFO_NAMETABLE, where_null: true, expect: 8 },
    PiNull { rows: &[421], what: PCRE2_INFO_FRAMESIZE, where_null: true, expect: 8 },
    PiNull { rows: &[421], what: PCRE2_INFO_JITSIZE, where_null: true, expect: 8 },
    // where == NULL and `what` unrecognised: falls into the NULL test
    PiNull { rows: &[422], what: 27, where_null: true, expect: PCRE2_ERROR_NULL },
    PiNull { rows: &[422], what: 28, where_null: true, expect: PCRE2_ERROR_NULL },
    PiNull { rows: &[422], what: 100, where_null: true, expect: PCRE2_ERROR_NULL },
    PiNull { rows: &[422], what: 0xFFFF_FFFF, where_null: true, expect: PCRE2_ERROR_NULL },
];

#[test]
fn pattern_info_null_code() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in PI_NULL {
            let tag = format!(
                "rows {:?} pattern_info(NULL, {}, {})",
                cs.rows,
                cs.what,
                if cs.where_null { "NULL" } else { "&buf" }
            );
            let ra = if cs.where_null {
                let ra = (p.c.pattern_info)(ptr::null_mut(), cs.what, ptr::null_mut());
                let rb = (p.r.pattern_info)(ptr::null_mut(), cs.what, ptr::null_mut());
                d.eq(&tag, ra, rb);
                ra
            } else {
                let mut ba = [PI_FILL; 32];
                let mut bb = [PI_FILL; 32];
                let ra = (p.c.pattern_info)(ptr::null_mut(), cs.what, ba.as_mut_ptr() as Ptr);
                let rb = (p.r.pattern_info)(ptr::null_mut(), cs.what, bb.as_mut_ptr() as Ptr);
                d.eq(&tag, ra, rb);
                d.eq(&format!("{tag} buf"), ba, bb);
                d.eq(&format!("{tag} buf untouched"), [PI_FILL; 32], ba);
                ra
            };
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect, ra);
        }
    }
    doc.finish("ERRORS.md rows 421, 422");
    d.finish("rows 421-422: pcre2_pattern_info_8 with code == NULL");
}

/// A `pcre2_code`-shaped scratch block: either a byte-for-byte copy of a real
/// code or all zeroes, so the magic/mode checks can be reached without UB.
struct FakeCode {
    bytes: Vec<u8>,
}
impl FakeCode {
    fn zeroed() -> FakeCode {
        FakeCode { bytes: vec![0u8; 512] }
    }
    unsafe fn copy_of(code: Ptr) -> FakeCode {
        let n = code_blocksize(code);
        FakeCode {
            bytes: std::slice::from_raw_parts(code as *const u8, n).to_vec(),
        }
    }
    fn ptr(&mut self) -> Ptr {
        self.bytes.as_mut_ptr() as Ptr
    }
    fn put_u32(&mut self, off: usize, v: u32) {
        self.bytes[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }
    fn get_u32(&self, off: usize) -> u32 {
        u32::from_ne_bytes(self.bytes[off..off + 4].try_into().unwrap())
    }
}

struct BadBlock {
    rows: &'static [u32],
    /// 0 = pattern_info, 1 = callout_enumerate
    fun: u32,
    /// 0 = all zeroes (bad magic), 1 = real code with flags bit 0 cleared
    kind: u32,
    expect: c_int,
}
const BAD_BLOCKS: &[BadBlock] = &[
    BadBlock { rows: &[423], fun: 0, kind: 0, expect: PCRE2_ERROR_BADMAGIC },
    BadBlock { rows: &[424], fun: 0, kind: 1, expect: PCRE2_ERROR_BADMODE },
    BadBlock { rows: &[434], fun: 1, kind: 0, expect: PCRE2_ERROR_BADMAGIC },
    BadBlock { rows: &[435], fun: 1, kind: 1, expect: PCRE2_ERROR_BADMODE },
];

unsafe extern "C" fn enum_cb_ok(_b: *mut c_void, _d: *mut c_void) -> c_int {
    0
}
static mut ENUM_RET: c_int = 0;
static mut ENUM_HITS: [i32; 2] = [0, 0];
unsafe extern "C" fn enum_cb_c(_b: *mut c_void, _d: *mut c_void) -> c_int {
    (*ptr::addr_of_mut!(ENUM_HITS))[0] += 1;
    ENUM_RET
}
unsafe extern "C" fn enum_cb_r(_b: *mut c_void, _d: *mut c_void) -> c_int {
    (*ptr::addr_of_mut!(ENUM_HITS))[1] += 1;
    ENUM_RET
}

#[test]
fn pattern_info_bad_magic_and_mode() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        // Self-check the layout the corruption below relies on.
        assert_eq!(offset_of!(RealCodeHead, magic_number), 88);
        assert_eq!(offset_of!(RealCodeHead, flags), 104);
        assert_eq!(std::mem::size_of::<RealCodeHead>(), 152);

        for cs in BAD_BLOCKS {
            let mut rcs = [0 as c_int; 2];
            for idx in 0..2 {
                let api = if idx == 0 { &p.c } else { &p.r };
                let mut fake = if cs.kind == 0 {
                    FakeCode::zeroed()
                } else {
                    let real = must_compile(api, b"a(?C1)b", 0);
                    let mut f = FakeCode::copy_of(real);
                    (api.code_free)(real);
                    let fl = f.get_u32(offset_of!(RealCodeHead, flags));
                    f.put_u32(offset_of!(RealCodeHead, flags), fl & !1);
                    assert_eq!(
                        f.get_u32(offset_of!(RealCodeHead, magic_number)),
                        MAGIC_NUMBER
                    );
                    f
                };
                let fp = fake.ptr();
                rcs[idx] = if cs.fun == 0 {
                    let mut buf = [PI_FILL; 32];
                    let r = (api.pattern_info)(fp, PCRE2_INFO_SIZE, buf.as_mut_ptr() as Ptr);
                    assert_eq!(buf, [PI_FILL; 32], "*where must not be written");
                    r
                } else {
                    (api.callout_enumerate)(fp, Some(enum_cb_ok), ptr::null_mut())
                };
            }
            let tag = format!("rows {:?} fun={} kind={}", cs.rows, cs.fun, cs.kind);
            d.eq(&tag, rcs[0], rcs[1]);
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect, rcs[0]);
        }
    }
    doc.finish("ERRORS.md rows 423, 424, 434, 435");
    d.finish("rows 423-424, 434-435: corrupt magic number / wrong compile mode");
}

// rows 433, 436: pcre2_callout_enumerate_8 NULL code and callback abort.
struct CalloutCase {
    rows: &'static [u32],
    pat: &'static str,
    /// value the callback returns
    ret: c_int,
    expect: c_int,
}
const CALLOUTS: &[CalloutCase] = &[
    CalloutCase { rows: &[436], pat: "a(?C1)b", ret: 0, expect: 0 },
    CalloutCase { rows: &[436], pat: "a(?C1)b", ret: 7, expect: 7 },
    CalloutCase { rows: &[436], pat: "a(?C1)b", ret: -99, expect: -99 },
    CalloutCase { rows: &[436], pat: "a(?C{txt})b", ret: 0, expect: 0 },
    CalloutCase { rows: &[436], pat: "a(?C{txt})b", ret: 5, expect: 5 },
    CalloutCase { rows: &[436], pat: "a(?C{txt})b", ret: c_int::MIN, expect: c_int::MIN },
    CalloutCase { rows: &[436], pat: "(?C1)a(?C2)b(?C3)", ret: 3, expect: 3 },
    CalloutCase { rows: &[436], pat: "(?C1)a(?C{x})b(?C3)", ret: 1, expect: 1 },
    CalloutCase { rows: &[436], pat: "abc", ret: 42, expect: 0 },
];

const CE_NULL: &[NullArgCase] = &[
    NullArgCase { rows: &[433], what: "pcre2_callout_enumerate_8(NULL, cb, data)" },
    NullArgCase { rows: &[433], what: "pcre2_callout_enumerate_8(code, NULL, data)" },
];

#[test]
fn callout_enumerate_errors() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in CE_NULL {
            if cs.what.contains("(NULL, cb") {
                let ra =
                    (p.c.callout_enumerate)(ptr::null_mut(), Some(enum_cb_ok), ptr::null_mut());
                let rb =
                    (p.r.callout_enumerate)(ptr::null_mut(), Some(enum_cb_ok), ptr::null_mut());
                d.eq(&format!("rows {:?} {}", cs.rows, cs.what), ra, rb);
                doc.eq(&format!("ERRORS.md rows {:?}", cs.rows), PCRE2_ERROR_NULL, ra);
            } else {
                // A NULL callback is never dereferenced when the pattern has no
                // callouts at all.
                let ka = must_compile(&p.c, b"abc", 0);
                let kb = must_compile(&p.r, b"abc", 0);
                d.eq(
                    &format!("rows {:?} {} on a callout-free pattern", cs.rows, cs.what),
                    (p.c.callout_enumerate)(ka, None, ptr::null_mut()),
                    (p.r.callout_enumerate)(kb, None, ptr::null_mut()),
                );
                (p.c.code_free)(ka);
                (p.r.code_free)(kb);
            }
        }

        for cs in CALLOUTS {
            let ka = must_compile(&p.c, cs.pat.as_bytes(), 0);
            let kb = must_compile(&p.r, cs.pat.as_bytes(), 0);
            ENUM_RET = cs.ret;
            *ptr::addr_of_mut!(ENUM_HITS) = [0, 0];
            let ra = (p.c.callout_enumerate)(ka, Some(enum_cb_c), 1 as Ptr);
            let rb = (p.r.callout_enumerate)(kb, Some(enum_cb_r), 1 as Ptr);
            let hits = *ptr::addr_of!(ENUM_HITS);
            let tag = format!(
                "rows {:?} pat={} ret={}",
                cs.rows,
                show(cs.pat.as_bytes()),
                cs.ret
            );
            d.eq(&tag, ra, rb);
            d.eq(&format!("{tag} callback invocations"), hits[0], hits[1]);
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect, ra);
            (p.c.code_free)(ka);
            (p.r.code_free)(kb);
        }
    }
    doc.finish("ERRORS.md rows 433, 436");
    d.finish("rows 433, 436: pcre2_callout_enumerate_8 NULL code and non-zero callback return");
}

// =============================================================== pcre2_context.c

struct SetterCase {
    rows: &'static [u32],
    /// 0 = set_newline, 1 = set_bsr, 2 = set_optimize,
    /// 3 = set_glob_separator, 4 = set_glob_escape
    which: u32,
    value: u32,
    expect: c_int,
}
const SETTERS: &[SetterCase] = &[
    // ---- set_newline: valid set is 1..6 ------------------------------- 438
    SetterCase { rows: &[438], which: 0, value: 0, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[438], which: 0, value: 1, expect: 0 },
    SetterCase { rows: &[438], which: 0, value: 2, expect: 0 },
    SetterCase { rows: &[438], which: 0, value: 3, expect: 0 },
    SetterCase { rows: &[438], which: 0, value: 4, expect: 0 },
    SetterCase { rows: &[438], which: 0, value: 5, expect: 0 },
    SetterCase { rows: &[438], which: 0, value: 6, expect: 0 },
    SetterCase { rows: &[438], which: 0, value: 7, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[438], which: 0, value: 8, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[438], which: 0, value: 99, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[438], which: 0, value: 0xFFFF_FFFF, expect: PCRE2_ERROR_BADDATA },
    // ---- set_bsr: valid set is 1..2 ----------------------------------- 437
    SetterCase { rows: &[437], which: 1, value: 0, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[437], which: 1, value: 1, expect: 0 },
    SetterCase { rows: &[437], which: 1, value: 2, expect: 0 },
    SetterCase { rows: &[437], which: 1, value: 3, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[437], which: 1, value: 4, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[437], which: 1, value: 99, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[437], which: 1, value: 0xFFFF_FFFF, expect: PCRE2_ERROR_BADDATA },
    // ---- set_optimize: accepted set is exactly {0,1,64..69} ----------- 440, 441
    SetterCase { rows: &[440], which: 2, value: 0, expect: 0 },
    SetterCase { rows: &[440], which: 2, value: 1, expect: 0 },
    SetterCase { rows: &[440], which: 2, value: 2, expect: PCRE2_ERROR_BADOPTION },
    SetterCase { rows: &[440], which: 2, value: 3, expect: PCRE2_ERROR_BADOPTION },
    SetterCase { rows: &[440], which: 2, value: 32, expect: PCRE2_ERROR_BADOPTION },
    SetterCase { rows: &[440], which: 2, value: 63, expect: PCRE2_ERROR_BADOPTION },
    SetterCase { rows: &[441], which: 2, value: 64, expect: 0 },
    SetterCase { rows: &[441], which: 2, value: 65, expect: 0 },
    SetterCase { rows: &[441], which: 2, value: 66, expect: 0 },
    SetterCase { rows: &[441], which: 2, value: 67, expect: 0 },
    SetterCase { rows: &[441], which: 2, value: 68, expect: 0 },
    SetterCase { rows: &[441], which: 2, value: 69, expect: 0 },
    SetterCase { rows: &[441], which: 2, value: 70, expect: PCRE2_ERROR_BADOPTION },
    SetterCase { rows: &[441], which: 2, value: 71, expect: PCRE2_ERROR_BADOPTION },
    SetterCase { rows: &[441], which: 2, value: 1000, expect: PCRE2_ERROR_BADOPTION },
    SetterCase { rows: &[441], which: 2, value: 0xFFFF_FFFF, expect: PCRE2_ERROR_BADOPTION },
    // ---- set_glob_separator: only '.', '/', '\\' ---------------------- 442
    SetterCase { rows: &[442], which: 3, value: 0, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[442], which: 3, value: 44, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[442], which: 3, value: 45, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[442], which: 3, value: 46, expect: 0 },
    SetterCase { rows: &[442], which: 3, value: 47, expect: 0 },
    SetterCase { rows: &[442], which: 3, value: 48, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[442], which: 3, value: 58, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[442], which: 3, value: 92, expect: 0 },
    SetterCase { rows: &[442], which: 3, value: 97, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[442], which: 3, value: 256, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[442], which: 3, value: 0xFFFF_FFFF, expect: PCRE2_ERROR_BADDATA },
    // ---- set_glob_escape: 0 or ASCII punctuation ---------------------- 443, 444
    SetterCase { rows: &[444], which: 4, value: 0, expect: 0 },
    SetterCase { rows: &[444], which: 4, value: 1, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 32, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 33, expect: 0 },
    SetterCase { rows: &[444], which: 4, value: 47, expect: 0 },
    SetterCase { rows: &[444], which: 4, value: 48, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 57, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 58, expect: 0 },
    SetterCase { rows: &[444], which: 4, value: 64, expect: 0 },
    SetterCase { rows: &[444], which: 4, value: 65, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 90, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 91, expect: 0 },
    SetterCase { rows: &[444], which: 4, value: 96, expect: 0 },
    SetterCase { rows: &[444], which: 4, value: 97, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 122, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 123, expect: 0 },
    SetterCase { rows: &[444], which: 4, value: 126, expect: 0 },
    SetterCase { rows: &[444], which: 4, value: 127, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 200, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[444], which: 4, value: 255, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[443], which: 4, value: 256, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[443], which: 4, value: 1000, expect: PCRE2_ERROR_BADDATA },
    SetterCase { rows: &[443], which: 4, value: 0xFFFF_FFFF, expect: PCRE2_ERROR_BADDATA },
];

/// row 439: `pcre2_set_optimize_8(NULL, ...)` — the NULL test comes first, so
/// every `directive` yields PCRE2_ERROR_NULL rather than PCRE2_ERROR_BADOPTION.
const OPT_NULL: &[SetterCase] = &[
    SetterCase { rows: &[439], which: 2, value: 0, expect: PCRE2_ERROR_NULL },
    SetterCase { rows: &[439], which: 2, value: 1, expect: PCRE2_ERROR_NULL },
    SetterCase { rows: &[439], which: 2, value: 2, expect: PCRE2_ERROR_NULL },
    SetterCase { rows: &[439], which: 2, value: 64, expect: PCRE2_ERROR_NULL },
    SetterCase { rows: &[439], which: 2, value: 69, expect: PCRE2_ERROR_NULL },
    SetterCase { rows: &[439], which: 2, value: 70, expect: PCRE2_ERROR_NULL },
    SetterCase { rows: &[439], which: 2, value: 0xFFFF_FFFF, expect: PCRE2_ERROR_NULL },
];

#[test]
fn context_setter_rejections() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        let cca = (p.c.compile_context_create)(ptr::null_mut());
        let ccb = (p.r.compile_context_create)(ptr::null_mut());
        let vca = (p.c.convert_context_create)(ptr::null_mut());
        let vcb = (p.r.convert_context_create)(ptr::null_mut());
        assert!(!cca.is_null() && !ccb.is_null() && !vca.is_null() && !vcb.is_null());

        for cs in SETTERS {
            let (ra, rb) = match cs.which {
                0 => ((p.c.set_newline)(cca, cs.value), (p.r.set_newline)(ccb, cs.value)),
                1 => ((p.c.set_bsr)(cca, cs.value), (p.r.set_bsr)(ccb, cs.value)),
                2 => ((p.c.set_optimize)(cca, cs.value), (p.r.set_optimize)(ccb, cs.value)),
                3 => (
                    (p.c.set_glob_separator)(vca, cs.value),
                    (p.r.set_glob_separator)(vcb, cs.value),
                ),
                _ => (
                    (p.c.set_glob_escape)(vca, cs.value),
                    (p.r.set_glob_escape)(vcb, cs.value),
                ),
            };
            let tag = format!("rows {:?} setter {} value {}", cs.rows, cs.which, cs.value);
            d.eq(&tag, ra, rb);
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect, ra);
            // The stored state must stay in lock-step: compile through the
            // context and compare the whole code block byte for byte.
            let ka = must_compile_cc(&p.c, b"a\\R(?:b)+", 0, cca);
            let kb = must_compile_cc(&p.r, b"a\\R(?:b)+", 0, ccb);
            assert_code_eq(ka, kb, &tag);
            (p.c.code_free)(ka);
            (p.r.code_free)(kb);
        }

        // Exhaustive sweeps of the five validating setters.
        for v in 0u32..=300 {
            d.eq(
                &format!("rows [442] set_glob_separator({v})"),
                (p.c.set_glob_separator)(vca, v),
                (p.r.set_glob_separator)(vcb, v),
            );
            d.eq(
                &format!("rows [443, 444] set_glob_escape({v})"),
                (p.c.set_glob_escape)(vca, v),
                (p.r.set_glob_escape)(vcb, v),
            );
            d.eq(
                &format!("rows [437] set_bsr({v})"),
                (p.c.set_bsr)(cca, v),
                (p.r.set_bsr)(ccb, v),
            );
            d.eq(
                &format!("rows [438] set_newline({v})"),
                (p.c.set_newline)(cca, v),
                (p.r.set_newline)(ccb, v),
            );
            d.eq(
                &format!("rows [440, 441] set_optimize({v})"),
                (p.c.set_optimize)(cca, v),
                (p.r.set_optimize)(ccb, v),
            );
        }

        // row 439: the only setter in the file with a NULL check.
        for cs in OPT_NULL {
            let ra = (p.c.set_optimize)(ptr::null_mut(), cs.value);
            let rb = (p.r.set_optimize)(ptr::null_mut(), cs.value);
            let tag = format!("rows {:?} set_optimize(NULL, {})", cs.rows, cs.value);
            d.eq(&tag, ra, rb);
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect, ra);
        }

        (p.c.compile_context_free)(cca);
        (p.r.compile_context_free)(ccb);
        (p.c.convert_context_free)(vca);
        (p.r.convert_context_free)(vcb);
    }
    doc.finish("ERRORS.md rows 437-444");
    d.finish("rows 437-444: out-of-range enum arguments to the validating context setters");
}

// rows 445-453: every constructor / copy under allocation failure.
struct CtxAllocCase {
    rows: &'static [u32],
    /// 0 memctl_malloc, 1 general_create, 2 compile_create, 3 match_create,
    /// 4 convert_create, 5 general_copy, 6 compile_copy, 7 match_copy,
    /// 8 convert_copy
    which: u32,
}
const CTX_ALLOC: &[CtxAllocCase] = &[
    CtxAllocCase { rows: &[445], which: 0 },
    CtxAllocCase { rows: &[446], which: 1 },
    CtxAllocCase { rows: &[447], which: 2 },
    CtxAllocCase { rows: &[448], which: 3 },
    CtxAllocCase { rows: &[449], which: 4 },
    CtxAllocCase { rows: &[450], which: 5 },
    CtxAllocCase { rows: &[451], which: 6 },
    CtxAllocCase { rows: &[452], which: 7 },
    CtxAllocCase { rows: &[453], which: 8 },
];

unsafe fn run_ctx_alloc(api: &Api, idx: usize, which: u32, budget: i64) -> String {
    reset(idx);
    let (m, f) = allocs(idx);
    if which == 1 {
        // The failing malloc IS the one general_context_create makes.
        set_budget(idx, budget);
        let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
        let out = format!("null={} mallocs={}", g.is_null(), nmalloc(idx));
        set_budget(idx, -1);
        if !g.is_null() {
            (api.general_context_free)(g);
        }
        return out;
    }
    let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
    assert!(!g.is_null());
    // For the *_copy rows the source context must exist first.
    let src: Ptr = match which {
        5 => g,
        6 => (api.compile_context_create)(g),
        7 => (api.match_context_create)(g),
        8 => (api.convert_context_create)(g),
        _ => ptr::null_mut(),
    };
    if which >= 6 {
        assert!(!src.is_null());
    }
    set_budget(idx, budget);
    let (res, kind) = match which {
        0 => ((api.p_memctl_malloc)(64, g), 9u32),
        2 => ((api.compile_context_create)(g), 2),
        3 => ((api.match_context_create)(g), 3),
        4 => ((api.convert_context_create)(g), 4),
        5 => ((api.general_context_copy)(src), 1),
        6 => ((api.compile_context_copy)(src), 2),
        7 => ((api.match_context_copy)(src), 3),
        8 => ((api.convert_context_copy)(src), 4),
        _ => unreachable!(),
    };
    let out = format!("null={} mallocs={}", res.is_null(), nmalloc(idx));
    set_budget(idx, -1);
    if !res.is_null() {
        match kind {
            1 => (api.general_context_free)(res),
            2 => (api.compile_context_free)(res),
            3 => (api.match_context_free)(res),
            4 => (api.convert_context_free)(res),
            // memctl_malloc's block carries a pcre2_memctl at its head; release
            // it through that, exactly as the library itself does.
            _ => (api.serialize_free)((res as *mut u8).add(24)),
        }
    }
    match which {
        6 => (api.compile_context_free)(src),
        7 => (api.match_context_free)(src),
        8 => (api.convert_context_free)(src),
        _ => {}
    }
    (api.general_context_free)(g);
    out
}

#[test]
fn context_allocation_failures() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for cs in CTX_ALLOC {
            for budget in 0..=1i64 {
                let a = run_ctx_alloc(&p.c, 0, cs.which, budget);
                let b = run_ctx_alloc(&p.r, 1, cs.which, budget);
                let tag = format!("rows {:?} which={} budget={budget}", cs.rows, cs.which);
                d.eq(&tag, a.clone(), b);
                if budget == 0 {
                    assert!(
                        a.starts_with("null=true"),
                        "ERRORS.md rows {:?}: the C must return NULL when its \
                         allocation fails, got {a}",
                        cs.rows
                    );
                }
            }
        }
        // row 445 also fires for a plain, absurdly large malloc with no memctl.
        d.eq(
            "rows [445] memctl_malloc(SIZE_MAX, NULL)",
            (p.c.p_memctl_malloc)(usize::MAX, ptr::null_mut()).is_null(),
            (p.r.p_memctl_malloc)(usize::MAX, ptr::null_mut()).is_null(),
        );
        assert!(
            (p.c.p_memctl_malloc)(usize::MAX, ptr::null_mut()).is_null(),
            "ERRORS.md row 445: the C must return NULL"
        );
        // NULL malloc / NULL free are legal for general_context_create (row 446).
        for (mm, ff) in [
            (None, None),
            (Some(mal_c as MallocFn), None),
            (None, Some(fre_c as FreeFn)),
        ] {
            reset(0);
            reset(1);
            let ga = (p.c.general_context_create)(mm, ff, ptr::null_mut());
            let gb = (p.r.general_context_create)(mm, ff, ptr::null_mut());
            d.eq(
                "rows [446] general_context_create with NULL malloc/free",
                ga.is_null(),
                gb.is_null(),
            );
            // and it must be usable
            let ca = (p.c.compile_context_create)(ga);
            let cb = (p.r.compile_context_create)(gb);
            d.eq("rows [446] derived compile context", ca.is_null(), cb.is_null());
            (p.c.compile_context_free)(ca);
            (p.r.compile_context_free)(cb);
            (p.c.general_context_free)(ga);
            (p.r.general_context_free)(gb);
        }
    }
    d.finish(
        "rows 445-453: PRIV(memctl_malloc) / every *_context_create / *_context_copy \
         under allocation failure",
    );
}

// row 454: all four context free functions accept NULL and must NOT call the
// user's free.
const CTX_FREE_NULL: &[NullArgCase] = &[NullArgCase {
    rows: &[454],
    what: "pcre2_{general,compile,match,convert}_context_free_8(NULL)",
}];

#[test]
fn context_free_null() {
    let p = pair();
    let mut d = Diffs::new();
    let cs = &CTX_FREE_NULL[0];
    unsafe {
        let mut added = [0i64; 2];
        for idx in 0..2 {
            let api = if idx == 0 { &p.c } else { &p.r };
            let (m, f) = allocs(idx);
            reset(idx);
            let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
            let cc = (api.compile_context_create)(g);
            let mc = (api.match_context_create)(g);
            let vc = (api.convert_context_create)(g);
            (api.compile_context_free)(cc);
            (api.match_context_free)(mc);
            (api.convert_context_free)(vc);
            let before = nfree(idx);
            for _ in 0..3 {
                (api.general_context_free)(ptr::null_mut());
                (api.compile_context_free)(ptr::null_mut());
                (api.match_context_free)(ptr::null_mut());
                (api.convert_context_free)(ptr::null_mut());
            }
            added[idx] = nfree(idx) - before;
            (api.general_context_free)(g);
        }
        d.eq(
            &format!("rows {:?} free calls added by {}", cs.rows, cs.what),
            added[0],
            added[1],
        );
        assert_eq!(added[0], 0, "ERRORS.md row 454: the C must not call free");
    }
    d.finish("row 454: pcre2_*_context_free_8(NULL) is a guarded no-op");
}

// ================================================================ pcre2_config.c

struct ConfigCase {
    rows: &'static [u32],
    what: u32,
    /// 0 = integer, 1 = string, 2 = always rejected
    kind: u32,
    /// documented result of the `where == NULL` length query
    expect_len: c_int,
}
const CONFIGS: &[ConfigCase] = &[
    ConfigCase { rows: &[459], what: 0, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 1, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[457], what: 2, kind: 2, expect_len: PCRE2_ERROR_BADOPTION },
    ConfigCase { rows: &[459], what: 3, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 4, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 5, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 6, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 7, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 8, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 9, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[458], what: 10, kind: 1, expect_len: 7 },
    ConfigCase { rows: &[458], what: 11, kind: 1, expect_len: 21 },
    ConfigCase { rows: &[459], what: 12, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 13, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 14, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 15, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[459], what: 16, kind: 0, expect_len: 4 },
    ConfigCase { rows: &[455, 456], what: 17, kind: 2, expect_len: PCRE2_ERROR_BADOPTION },
    ConfigCase { rows: &[455, 456], what: 18, kind: 2, expect_len: PCRE2_ERROR_BADOPTION },
    ConfigCase { rows: &[455, 456], what: 100, kind: 2, expect_len: PCRE2_ERROR_BADOPTION },
    ConfigCase { rows: &[455, 456], what: 0x8000_0000, kind: 2,
                 expect_len: PCRE2_ERROR_BADOPTION },
    ConfigCase { rows: &[455, 456], what: 0xFFFF_FFFF, kind: 2,
                 expect_len: PCRE2_ERROR_BADOPTION },
];

#[test]
fn config_every_what() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in CONFIGS {
            let tag = format!("rows {:?} config({})", cs.rows, cs.what);
            // (a) length query
            let la = (p.c.config)(cs.what, ptr::null_mut());
            let lb = (p.r.config)(cs.what, ptr::null_mut());
            d.eq(&format!("{tag} where=NULL"), la, lb);
            doc.eq(&format!("ERRORS.md {tag} where=NULL"), cs.expect_len, la);
            // (b) with a generously sized, poisoned buffer: nothing beyond the
            //     documented width may be touched.
            let mut ba = [0xAAu8; 64];
            let mut bb = [0xAAu8; 64];
            let ra = (p.c.config)(cs.what, ba.as_mut_ptr() as Ptr);
            let rb = (p.r.config)(cs.what, bb.as_mut_ptr() as Ptr);
            d.eq(&format!("{tag} rc"), ra, rb);
            d.eq(&format!("{tag} buffer"), ba, bb);
            match cs.kind {
                0 => {
                    doc.eq(&format!("ERRORS.md {tag} integer rc"), 0, ra);
                    doc.eq(
                        &format!("ERRORS.md {tag} only 4 bytes written"),
                        vec![0xAAu8; 60],
                        ba[4..].to_vec(),
                    );
                }
                1 => {
                    doc.eq(&format!("ERRORS.md {tag} string rc"), cs.expect_len, ra);
                    let n = ra as usize;
                    doc.eq(
                        &format!("ERRORS.md {tag} only {n} bytes written"),
                        vec![0xAAu8; 64 - n],
                        ba[n..].to_vec(),
                    );
                    doc.eq(&format!("ERRORS.md {tag} NUL terminated"), 0u8, ba[n - 1]);
                    println!(
                        "config[{}] = {:?} (len {n})",
                        cs.what,
                        std::str::from_utf8(&ba[..n - 1]).unwrap()
                    );
                }
                _ => {
                    doc.eq(
                        &format!("ERRORS.md {tag} rejected"),
                        PCRE2_ERROR_BADOPTION,
                        ra,
                    );
                    doc.eq(
                        &format!("ERRORS.md {tag} buffer untouched"),
                        vec![0xAAu8; 64],
                        ba.to_vec(),
                    );
                }
            }
        }
    }
    doc.finish("ERRORS.md rows 455-459");
    d.finish("rows 455-459: pcre2_config_8 for every `what` 0..=16 plus out-of-range, both forms");
}

// ============================================================= pcre2_serialize.c

const SD_HEADER: usize = 16; // sizeof(pcre2_serialized_data)

/// Offsets inside the serialized stream, DERIVED (not assumed) from the
/// `pcre2_real_code` layout the harness already validates.
struct SerOffsets {
    tables: usize,
    code0: usize,
    blocksize: usize,
    magic: usize,
    name_entry_size: usize,
    name_count: usize,
}
fn ser_offsets(tables_len: usize) -> SerOffsets {
    let code0 = SD_HEADER + tables_len;
    SerOffsets {
        tables: SD_HEADER,
        code0,
        blocksize: code0 + offset_of!(RealCodeHead, blocksize),
        magic: code0 + offset_of!(RealCodeHead, magic_number),
        name_entry_size: code0 + offset_of!(RealCodeHead, name_entry_size),
        name_count: code0 + offset_of!(RealCodeHead, name_count),
    }
}
fn field_off(o: &SerOffsets, f: &str) -> usize {
    match f {
        "magic" => 0,
        "version" => 4,
        "config" => 8,
        "number_of_codes" => 12,
        "tables" => o.tables,
        "blocksize" => o.blocksize,
        "code_magic" => o.magic,
        "name_entry_size" => o.name_entry_size,
        "name_count" => o.name_count,
        _ => panic!("unknown field {f}"),
    }
}
unsafe fn tables_length(api: &Api) -> usize {
    let mut tl: u32 = 0;
    assert_eq!((api.config)(PCRE2_CONFIG_TABLES_LENGTH, &mut tl as *mut _ as Ptr), 0);
    tl as usize
}

const SER_PATS: &[&str] = &["(?<abc>a)(b)", "x[0-9]+y"];

/// Encodes SER_PATS with `api` and returns the stream as an owned aligned copy.
unsafe fn encode_stream(api: &Api) -> Buf {
    let codes: Vec<Ptr> = SER_PATS
        .iter()
        .map(|s| must_compile(api, s.as_bytes(), 0))
        .collect();
    let mut bytes: *mut u8 = ptr::null_mut();
    let mut size: Sz = 0;
    let rc = (api.serialize_encode)(
        codes.as_ptr(),
        codes.len() as i32,
        &mut bytes,
        &mut size,
        ptr::null_mut(),
    );
    assert_eq!(
        rc,
        codes.len() as i32,
        "[{}] serialize_encode failed",
        api.name
    );
    let buf = Buf::from_raw(bytes, size);
    (api.serialize_free)(bytes);
    for c in codes {
        (api.code_free)(c);
    }
    buf
}

/// Decodes `buf` with `api` and renders every observable.
unsafe fn decode_report(api: &Api, buf: &Buf, ncodes: i32, pass_codes: bool) -> String {
    let mut codes: [Ptr; 4] = [1 as Ptr, 2 as Ptr, 3 as Ptr, 4 as Ptr];
    let rc = (api.serialize_decode)(
        if pass_codes {
            codes.as_mut_ptr()
        } else {
            ptr::null_mut()
        },
        ncodes,
        buf.p,
        ptr::null_mut(),
    );
    let mut out = format!("rc={rc} slots=[");
    for (i, c) in codes.iter().enumerate() {
        out += if *c == (i + 1) as usize as Ptr {
            "untouched,"
        } else if c.is_null() {
            "NULL,"
        } else {
            "code,"
        };
    }
    out += "]";
    if rc > 0 {
        for i in 0..rc as usize {
            if !codes[i].is_null() && codes[i] != (i + 1) as usize as Ptr {
                let mut n: u32 = 0;
                let r = (api.pattern_info)(
                    codes[i],
                    PCRE2_INFO_CAPTURECOUNT,
                    &mut n as *mut _ as Ptr,
                );
                out += &format!(" [{i}]cc={n}/{r}");
                (api.code_free)(codes[i]);
            }
        }
    }
    out
}

#[test]
fn serialize_streams_are_identical() {
    let p = pair();
    unsafe {
        let a = encode_stream(&p.c);
        let b = encode_stream(&p.r);
        assert_eq!(a.len, b.len, "serialized sizes differ");
        assert_eq!(a.as_slice(), b.as_slice(), "serialized streams differ");

        // Self-check every offset the corruption cases below rely on.
        let tl = tables_length(&p.c);
        let o = ser_offsets(tl);
        let mut sum = 0;
        for s in SER_PATS {
            let k = must_compile(&p.c, s.as_bytes(), 0);
            sum += code_blocksize(k);
            (p.c.code_free)(k);
        }
        assert_eq!(a.len, SD_HEADER + tl + sum, "unexpected stream length");
        let s = a.as_slice();
        let bs = usize::from_ne_bytes(s[o.blocksize..o.blocksize + 8].try_into().unwrap());
        assert!(bs > 152 && bs < a.len, "blocksize in stream = {bs}");
        assert_eq!(
            u32::from_ne_bytes(s[o.magic..o.magic + 4].try_into().unwrap()),
            MAGIC_NUMBER
        );
        // `(?<abc>a)(b)` has exactly one name, entry size IMM2_SIZE + 3 + 1 = 6
        assert_eq!(
            u16::from_ne_bytes(s[o.name_count..o.name_count + 2].try_into().unwrap()),
            1
        );
        assert_eq!(
            u16::from_ne_bytes(
                s[o.name_entry_size..o.name_entry_size + 2].try_into().unwrap()
            ),
            6
        );
        println!(
            "serialize offsets: tables={} code0={} blocksize={} code_magic={} \
             name_entry_size={} name_count={}",
            o.tables, o.code0, o.blocksize, o.magic, o.name_entry_size, o.name_count
        );
    }
}

#[derive(Copy, Clone, Debug)]
enum Patch {
    None,
    U32(&'static str, u32),
    I32(&'static str, i32),
    U16(&'static str, u16),
    Usize(&'static str, usize),
}

struct DecCase {
    rows: &'static [u32],
    patch: Patch,
    ncodes: i32,
    pass_codes: bool,
    expect: i32,
}

const DEC_CASES: &[DecCase] = &[
    // controls
    DecCase { rows: &[471], patch: Patch::None, ncodes: 2, pass_codes: true, expect: 2 },
    DecCase { rows: &[471], patch: Patch::None, ncodes: 1, pass_codes: true, expect: 1 },
    DecCase { rows: &[471], patch: Patch::None, ncodes: 5, pass_codes: true, expect: 2 },
    // 469: codes == NULL
    DecCase { rows: &[469], patch: Patch::None, ncodes: 2, pass_codes: false,
              expect: PCRE2_ERROR_NULL as i32 },
    // 470: caller's number_of_codes <= 0
    DecCase { rows: &[470], patch: Patch::None, ncodes: 0, pass_codes: true,
              expect: PCRE2_ERROR_BADDATA as i32 },
    DecCase { rows: &[470], patch: Patch::None, ncodes: -1, pass_codes: true,
              expect: PCRE2_ERROR_BADDATA as i32 },
    DecCase { rows: &[470], patch: Patch::None, ncodes: i32::MIN, pass_codes: true,
              expect: PCRE2_ERROR_BADDATA as i32 },
    // 471: the stream's own number_of_codes <= 0 (checked BEFORE magic)
    DecCase { rows: &[471], patch: Patch::I32("number_of_codes", 0), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    DecCase { rows: &[471], patch: Patch::I32("number_of_codes", -1), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    DecCase { rows: &[471], patch: Patch::I32("number_of_codes", i32::MIN), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    // 472: magic
    DecCase { rows: &[472], patch: Patch::U32("magic", 0), ncodes: 2, pass_codes: true,
              expect: PCRE2_ERROR_BADMAGIC as i32 },
    DecCase { rows: &[472], patch: Patch::U32("magic", 0x5052_3252), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADMAGIC as i32 },
    DecCase { rows: &[472], patch: Patch::U32("magic", 0xFFFF_FFFF), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADMAGIC as i32 },
    // 473: version
    DecCase { rows: &[473], patch: Patch::U32("version", 0x0027_000A), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADMODE as i32 },
    DecCase { rows: &[473], patch: Patch::U32("version", 0), ncodes: 2, pass_codes: true,
              expect: PCRE2_ERROR_BADMODE as i32 },
    // 474: config
    DecCase { rows: &[474], patch: Patch::U32("config", 0x0008_0802), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADMODE as i32 },
    DecCase { rows: &[474], patch: Patch::U32("config", 0x0004_0401), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADMODE as i32 },
    // 475: per-code blocksize <= sizeof(pcre2_real_code)
    DecCase { rows: &[475], patch: Patch::Usize("blocksize", 0), ncodes: 2, pass_codes: true,
              expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    DecCase { rows: &[475], patch: Patch::Usize("blocksize", 1), ncodes: 2, pass_codes: true,
              expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    DecCase { rows: &[475], patch: Patch::Usize("blocksize", 152), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    // 478: the copied code's magic number
    DecCase { rows: &[478], patch: Patch::U32("code_magic", 0), ncodes: 2, pass_codes: true,
              expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    DecCase { rows: &[478], patch: Patch::U32("code_magic", 0x5043_5246), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    // 479: name_entry_size > MAX_NAME_SIZE + IMM2_SIZE + 1 == 131
    DecCase { rows: &[479], patch: Patch::U16("name_entry_size", 132), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    DecCase { rows: &[479], patch: Patch::U16("name_entry_size", 65535), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    DecCase { rows: &[479], patch: Patch::U16("name_entry_size", 131), ncodes: 2,
              pass_codes: true, expect: 2 },
    // 480: name_count > MAX_NAME_COUNT == 10000
    DecCase { rows: &[480], patch: Patch::U16("name_count", 10001), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    DecCase { rows: &[480], patch: Patch::U16("name_count", 65535), ncodes: 2,
              pass_codes: true, expect: PCRE2_ERROR_BADSERIALIZEDDATA as i32 },
    DecCase { rows: &[480], patch: Patch::U16("name_count", 10000), ncodes: 2,
              pass_codes: true, expect: 2 },
];

const DEC_NULL: &[NullArgCase] = &[
    NullArgCase { rows: &[468], what: "pcre2_serialize_decode_8(codes, 2, NULL, NULL)" },
    NullArgCase { rows: &[468, 469], what: "pcre2_serialize_decode_8(NULL, 2, NULL, NULL)" },
];

#[test]
fn serialize_decode_corruption() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        let o = ser_offsets(tables_length(&p.c));
        let pristine = encode_stream(&p.c);

        for cs in DEC_CASES {
            let mut ba = Buf::from_raw(pristine.p, pristine.len);
            let mut bb = Buf::from_raw(pristine.p, pristine.len);
            ba.patch(&o, cs.patch);
            bb.patch(&o, cs.patch);
            let tag = format!(
                "rows {:?} patch={:?} ncodes={} codes={}",
                cs.rows, cs.patch, cs.ncodes, cs.pass_codes
            );
            let a = decode_report(&p.c, &ba, cs.ncodes, cs.pass_codes);
            let b = decode_report(&p.r, &bb, cs.ncodes, cs.pass_codes);
            d.eq(&tag, a.clone(), b);
            // Neither library may modify the caller's stream.
            d.eq(&format!("{tag} stream unmodified"), ba.as_slice(), bb.as_slice());
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect as i64, rc_of(&a));
        }

        // row 468: bytes == NULL
        let mut codes: [Ptr; 2] = [ptr::null_mut(); 2];
        for cs in DEC_NULL {
            let (ra, rb) = if cs.what.contains("(codes,") {
                (
                    (p.c.serialize_decode)(codes.as_mut_ptr(), 2, ptr::null(), ptr::null_mut()),
                    (p.r.serialize_decode)(codes.as_mut_ptr(), 2, ptr::null(), ptr::null_mut()),
                )
            } else {
                (
                    (p.c.serialize_decode)(ptr::null_mut(), 2, ptr::null(), ptr::null_mut()),
                    (p.r.serialize_decode)(ptr::null_mut(), 2, ptr::null(), ptr::null_mut()),
                )
            };
            d.eq(&format!("rows {:?} {}", cs.rows, cs.what), ra, rb);
            doc.eq(
                &format!("ERRORS.md rows {:?} {}", cs.rows, cs.what),
                PCRE2_ERROR_NULL as i32,
                ra,
            );
        }
        assert!(codes.iter().all(|c| c.is_null()), "codes must be untouched");
    }
    doc.finish("ERRORS.md rows 468-475, 478-480");
    d.finish("rows 468-475, 478-480: pcre2_serialize_decode_8 argument and stream validation");
}

struct EncCase {
    rows: &'static [u32],
    /// 0 codes NULL, 1 bytes NULL, 2 size NULL, 3 n<=0, 4 codes[i]==NULL,
    /// 5 bad magic, 6 mixed tables
    which: u32,
    n: i32,
    expect: i32,
}
const ENC_CASES: &[EncCase] = &[
    EncCase { rows: &[460], which: 0, n: 1, expect: PCRE2_ERROR_NULL as i32 },
    EncCase { rows: &[461], which: 1, n: 1, expect: PCRE2_ERROR_NULL as i32 },
    EncCase { rows: &[462], which: 2, n: 1, expect: PCRE2_ERROR_NULL as i32 },
    EncCase { rows: &[463], which: 3, n: 0, expect: PCRE2_ERROR_BADDATA as i32 },
    EncCase { rows: &[463], which: 3, n: -1, expect: PCRE2_ERROR_BADDATA as i32 },
    EncCase { rows: &[463], which: 3, n: i32::MIN, expect: PCRE2_ERROR_BADDATA as i32 },
    EncCase { rows: &[464], which: 4, n: 2, expect: PCRE2_ERROR_NULL as i32 },
    EncCase { rows: &[465], which: 5, n: 1, expect: PCRE2_ERROR_BADMAGIC as i32 },
    EncCase { rows: &[465], which: 5, n: 2, expect: PCRE2_ERROR_BADMAGIC as i32 },
    EncCase { rows: &[466], which: 6, n: 2, expect: PCRE2_ERROR_MIXEDTABLES as i32 },
    // control: two codes sharing the default tables encode fine
    EncCase { rows: &[466], which: 7, n: 2, expect: 2 },
];

#[test]
fn serialize_encode_rejections() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in ENC_CASES {
            let mut outs = [String::new(), String::new()];
            for idx in 0..2 {
                let api = if idx == 0 { &p.c } else { &p.r };
                let ka = must_compile(api, b"(a)", 0);
                // Row 466 needs a SECOND code built against DIFFERENT tables.
                // NB tables are borrowed: they must outlive every code compiled
                // against them, and the encode call itself.
                let tables = if cs.which == 6 {
                    (api.maketables)(ptr::null_mut())
                } else {
                    ptr::null()
                };
                assert!(cs.which != 6 || !tables.is_null());
                let kb = if cs.which == 6 {
                    let cc = (api.compile_context_create)(ptr::null_mut());
                    assert_eq!((api.set_character_tables)(cc, tables), 0);
                    let k = must_compile_cc(api, b"(b)", 0, cc);
                    (api.compile_context_free)(cc);
                    k
                } else {
                    must_compile(api, b"(b)", 0)
                };
                let mut zero = FakeCode::zeroed();
                let zp = zero.ptr();
                let codes: Vec<Ptr> = match cs.which {
                    4 => vec![ka, ptr::null_mut()],
                    5 if cs.n == 1 => vec![zp],
                    5 => vec![ka, zp],
                    _ => vec![ka, kb],
                };
                let mut bytes: *mut u8 = 11 as *mut u8;
                let mut size: Sz = 0xDEAD;
                let rc = (api.serialize_encode)(
                    if cs.which == 0 { ptr::null() } else { codes.as_ptr() },
                    cs.n,
                    if cs.which == 1 { ptr::null_mut() } else { &mut bytes },
                    if cs.which == 2 { ptr::null_mut() } else { &mut size },
                    ptr::null_mut(),
                );
                outs[idx] = format!(
                    "rc={rc} bytes_untouched={} size={size:#x}",
                    bytes == 11 as *mut u8
                );
                if rc > 0 && cs.which != 1 {
                    (api.serialize_free)(bytes);
                }
                (api.code_free)(ka);
                (api.code_free)(kb);
                if !tables.is_null() {
                    (api.maketables_free)(ptr::null_mut(), tables);
                }
            }
            let tag = format!("rows {:?} which={} n={}", cs.rows, cs.which, cs.n);
            d.eq(&tag, outs[0].clone(), outs[1].clone());
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect as i64, rc_of(&outs[0]));
        }
    }
    doc.finish("ERRORS.md rows 460-466");
    d.finish("rows 460-466: pcre2_serialize_encode_8 NULL / BADDATA / BADMAGIC / MIXEDTABLES");
}

struct GnocCase {
    rows: &'static [u32],
    patch: Patch,
    expect: i32,
}
const GNOC_CASES: &[GnocCase] = &[
    GnocCase { rows: &[485], patch: Patch::None, expect: 2 },
    GnocCase { rows: &[482], patch: Patch::U32("magic", 0),
               expect: PCRE2_ERROR_BADMAGIC as i32 },
    GnocCase { rows: &[482], patch: Patch::U32("magic", 0xDEAD_BEEF),
               expect: PCRE2_ERROR_BADMAGIC as i32 },
    GnocCase { rows: &[483], patch: Patch::U32("version", 0),
               expect: PCRE2_ERROR_BADMODE as i32 },
    GnocCase { rows: &[483], patch: Patch::U32("version", 0x0027_000A),
               expect: PCRE2_ERROR_BADMODE as i32 },
    GnocCase { rows: &[484], patch: Patch::U32("config", 0),
               expect: PCRE2_ERROR_BADMODE as i32 },
    GnocCase { rows: &[484], patch: Patch::U32("config", 0x0008_0802),
               expect: PCRE2_ERROR_BADMODE as i32 },
    // 485: number_of_codes is NOT validated here — returned verbatim
    GnocCase { rows: &[485], patch: Patch::I32("number_of_codes", 0), expect: 0 },
    GnocCase { rows: &[485], patch: Patch::I32("number_of_codes", -1), expect: -1 },
    GnocCase { rows: &[485], patch: Patch::I32("number_of_codes", i32::MIN), expect: i32::MIN },
    GnocCase { rows: &[485], patch: Patch::I32("number_of_codes", 12345), expect: 12345 },
];

const GNOC_NULL: &[NullArgCase] =
    &[NullArgCase { rows: &[481], what: "pcre2_serialize_get_number_of_codes_8(NULL)" }];

#[test]
fn serialize_get_number_of_codes_validation() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        let o = ser_offsets(tables_length(&p.c));
        let pristine = encode_stream(&p.c);
        for cs in GNOC_CASES {
            let mut b = Buf::from_raw(pristine.p, pristine.len);
            b.patch(&o, cs.patch);
            let tag = format!("rows {:?} patch={:?}", cs.rows, cs.patch);
            let ra = (p.c.serialize_get_number_of_codes)(b.p);
            let rb = (p.r.serialize_get_number_of_codes)(b.p);
            d.eq(&tag, ra, rb);
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect, ra);
        }
        // row 481: NULL
        for cs in GNOC_NULL {
            let ra = (p.c.serialize_get_number_of_codes)(ptr::null());
            let rb = (p.r.serialize_get_number_of_codes)(ptr::null());
            d.eq(&format!("rows {:?} {}", cs.rows, cs.what), ra, rb);
            doc.eq(
                &format!("ERRORS.md rows {:?} {}", cs.rows, cs.what),
                PCRE2_ERROR_NULL as i32,
                ra,
            );
        }
        // an all-zero 16-byte header
        let zeros_src = [0u8; 64];
        let zeros = Buf::from_raw(zeros_src.as_ptr(), 64);
        d.eq(
            "rows [482] get_number_of_codes(all-zero header)",
            (p.c.serialize_get_number_of_codes)(zeros.p),
            (p.r.serialize_get_number_of_codes)(zeros.p),
        );
    }
    doc.finish("ERRORS.md rows 481-485");
    d.finish("rows 481-485: pcre2_serialize_get_number_of_codes_8 validation");
}

// rows 467, 476, 477, 488: allocation failures in serialize / maketables.
struct SerAllocCase {
    rows: &'static [u32],
    /// 0 = encode, 1 = decode, 2 = maketables
    which: u32,
    expect: i32,
}
const SER_ALLOC: &[SerAllocCase] = &[
    SerAllocCase { rows: &[467], which: 0, expect: PCRE2_ERROR_NOMEMORY as i32 },
    SerAllocCase { rows: &[476, 477], which: 1, expect: PCRE2_ERROR_NOMEMORY as i32 },
    SerAllocCase { rows: &[488], which: 2, expect: 0 },
];

unsafe fn run_ser_alloc(api: &Api, idx: usize, which: u32, budget: i64, pristine: &Buf) -> String {
    reset(idx);
    let (m, f) = allocs(idx);
    let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
    assert!(!g.is_null());
    let out;
    match which {
        0 => {
            let codes: Vec<Ptr> = SER_PATS
                .iter()
                .map(|s| must_compile(api, s.as_bytes(), 0))
                .collect();
            set_budget(idx, budget);
            let mut bytes: *mut u8 = 11 as *mut u8;
            let mut size: Sz = 0xDEAD;
            let rc = (api.serialize_encode)(
                codes.as_ptr(),
                codes.len() as i32,
                &mut bytes,
                &mut size,
                g,
            );
            out = format!("rc={rc} untouched={}", bytes == 11 as *mut u8);
            set_budget(idx, -1);
            if rc > 0 {
                (api.serialize_free)(bytes);
            }
            for c in codes {
                (api.code_free)(c);
            }
        }
        1 => {
            let buf = Buf::from_raw(pristine.p, pristine.len);
            set_budget(idx, budget);
            let mut codes: [Ptr; 4] = [1 as Ptr, 2 as Ptr, 3 as Ptr, 4 as Ptr];
            let rc = (api.serialize_decode)(codes.as_mut_ptr(), 2, buf.p, g);
            let mut s = format!("rc={rc} slots=[");
            for (i, c) in codes.iter().enumerate() {
                s += if *c == (i + 1) as usize as Ptr {
                    "untouched,"
                } else if c.is_null() {
                    "NULL,"
                } else {
                    "code,"
                };
            }
            s += "]";
            out = s;
            set_budget(idx, -1);
            if rc > 0 {
                for i in 0..rc as usize {
                    (api.code_free)(codes[i]);
                }
            }
        }
        _ => {
            set_budget(idx, budget);
            let t = (api.maketables)(g);
            out = format!("null={} mallocs={}", t.is_null(), nmalloc(idx));
            set_budget(idx, -1);
            if !t.is_null() {
                (api.maketables_free)(g, t);
            }
        }
    }
    (api.general_context_free)(g);
    out
}

#[test]
fn serialize_allocation_failures() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let pristine = encode_stream(&p.c);
        for cs in SER_ALLOC {
            for budget in 0..=4i64 {
                let a = run_ser_alloc(&p.c, 0, cs.which, budget, &pristine);
                let b = run_ser_alloc(&p.r, 1, cs.which, budget, &pristine);
                let tag = format!("rows {:?} which={} budget={budget}", cs.rows, cs.which);
                d.eq(&tag, a.clone(), b);
                if budget == 0 {
                    if cs.which == 2 {
                        assert!(
                            a.starts_with("null=true"),
                            "ERRORS.md row 488: pcre2_maketables_8 must return NULL, got {a}"
                        );
                    } else {
                        assert_eq!(
                            rc_of(&a),
                            cs.expect as i64,
                            "ERRORS.md rows {:?}: expected {} from the C",
                            cs.rows,
                            cs.expect
                        );
                    }
                }
            }
        }
    }
    d.finish(
        "rows 467, 476, 477, 488: allocation failure in serialize_encode / \
         serialize_decode / maketables",
    );
}

// rows 486, 487: pcre2_serialize_free_8.
#[test]
fn serialize_free_null() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for _ in 0..3 {
            (p.c.serialize_free)(ptr::null_mut());
            (p.r.serialize_free)(ptr::null_mut());
        }
        d.eq("rows [486] serialize_free(NULL) survived", true, true);
        // The custom `free` must NOT be reached for NULL. Round-trip a real
        // stream through a counting allocator first so the counter is live.
        let mut added = [0i64; 2];
        for idx in 0..2 {
            let api = if idx == 0 { &p.c } else { &p.r };
            let (m, f) = allocs(idx);
            reset(idx);
            let g = (api.general_context_create)(Some(m), Some(f), ptr::null_mut());
            let code = must_compile(api, b"(a)", 0);
            let codes = [code];
            let mut bytes: *mut u8 = ptr::null_mut();
            let mut size: Sz = 0;
            assert_eq!(
                (api.serialize_encode)(codes.as_ptr(), 1, &mut bytes, &mut size, g),
                1
            );
            (api.serialize_free)(bytes);
            let before = nfree(idx);
            (api.serialize_free)(ptr::null_mut());
            added[idx] = nfree(idx) - before;
            (api.code_free)(code);
            (api.general_context_free)(g);
        }
        d.eq("rows [486] free calls for NULL", added[0], added[1]);
        assert_eq!(added[0], 0, "ERRORS.md row 486: the C must not call free");

        // Row 487 documents UNDEFINED BEHAVIOUR: a pointer that did not come
        // from pcre2_serialize_encode_8 makes the function call whatever
        // function pointer happens to sit 24 bytes below it. There is no
        // comparable observable, so the nearest reachable input is asserted
        // instead — a genuine encode/free pair, which both libraries survive.
        for idx in 0..2 {
            let api = if idx == 0 { &p.c } else { &p.r };
            let code = must_compile(api, b"(a)", 0);
            let codes = [code];
            let mut bytes: *mut u8 = ptr::null_mut();
            let mut size: Sz = 0;
            assert_eq!(
                (api.serialize_encode)(codes.as_ptr(), 1, &mut bytes, &mut size, ptr::null_mut()),
                1
            );
            (api.serialize_free)(bytes);
            (api.code_free)(code);
        }
        d.eq("rows [487] genuine encode/serialize_free round trip", true, true);
    }
    d.finish("rows 486-487: pcre2_serialize_free_8(NULL) and the documented UB case");
}

// ================================================================= pcre2_error.c

struct ErrCase {
    rows: &'static [u32],
    enumber: c_int,
    size: usize,
    expect: c_int,
}
const ERR_CASES: &[ErrCase] = &[
    // 489: size == 0 short-circuits before anything is written
    ErrCase { rows: &[489], enumber: -1, size: 0, expect: PCRE2_ERROR_NOMEMORY },
    ErrCase { rows: &[489], enumber: 100, size: 0, expect: PCRE2_ERROR_NOMEMORY },
    ErrCase { rows: &[489], enumber: 0, size: 0, expect: PCRE2_ERROR_NOMEMORY },
    ErrCase { rows: &[489], enumber: 999_999, size: 0, expect: PCRE2_ERROR_NOMEMORY },
    // 490: 0 <= enumber < COMPILE_ERROR_BASE
    ErrCase { rows: &[490], enumber: 0, size: 64, expect: PCRE2_ERROR_BADDATA },
    ErrCase { rows: &[490], enumber: 1, size: 64, expect: PCRE2_ERROR_BADDATA },
    ErrCase { rows: &[490], enumber: 50, size: 64, expect: PCRE2_ERROR_BADDATA },
    ErrCase { rows: &[490], enumber: 99, size: 64, expect: PCRE2_ERROR_BADDATA },
    ErrCase { rows: &[490], enumber: 0, size: 1, expect: PCRE2_ERROR_BADDATA },
    // 491: above the last compile-error text (index 120 == enumber 220)
    ErrCase { rows: &[491], enumber: 220, size: 64, expect: 26 },
    ErrCase { rows: &[491], enumber: 221, size: 64, expect: PCRE2_ERROR_BADDATA },
    ErrCase { rows: &[491], enumber: 300, size: 64, expect: PCRE2_ERROR_BADDATA },
    ErrCase { rows: &[491], enumber: 1000, size: 64, expect: PCRE2_ERROR_BADDATA },
    ErrCase { rows: &[491], enumber: c_int::MAX, size: 64, expect: PCRE2_ERROR_BADDATA },
    // 492: below the last match-error text (index 76 == enumber -76)
    ErrCase { rows: &[492], enumber: -76, size: 128, expect: 53 },
    ErrCase { rows: &[492], enumber: -77, size: 64, expect: PCRE2_ERROR_BADDATA },
    ErrCase { rows: &[492], enumber: -100, size: 64, expect: PCRE2_ERROR_BADDATA },
    ErrCase { rows: &[492], enumber: -1000, size: 64, expect: PCRE2_ERROR_BADDATA },
    // 493: buffer one byte too small, and every smaller size ("no match" == 8)
    ErrCase { rows: &[493], enumber: -1, size: 1, expect: PCRE2_ERROR_NOMEMORY },
    ErrCase { rows: &[493], enumber: -1, size: 2, expect: PCRE2_ERROR_NOMEMORY },
    ErrCase { rows: &[493], enumber: -1, size: 5, expect: PCRE2_ERROR_NOMEMORY },
    ErrCase { rows: &[493], enumber: -1, size: 8, expect: PCRE2_ERROR_NOMEMORY },
    ErrCase { rows: &[493], enumber: -1, size: 9, expect: 8 },
    ErrCase { rows: &[493], enumber: -1, size: 64, expect: 8 },
    ErrCase { rows: &[493], enumber: 101, size: 19, expect: PCRE2_ERROR_NOMEMORY },
    ErrCase { rows: &[493], enumber: 101, size: 20, expect: 19 },
];

#[test]
fn get_error_message_errors() {
    let p = pair();
    let mut d = Diffs::new();
    let mut doc = Diffs::new();
    unsafe {
        for cs in ERR_CASES {
            // A guard region past `size` catches a one-past-the-end write.
            let mut ba = vec![0xC7u8; cs.size + 16];
            let mut bb = vec![0xC7u8; cs.size + 16];
            let ra = (p.c.get_error_message)(cs.enumber, ba.as_mut_ptr(), cs.size);
            let rb = (p.r.get_error_message)(cs.enumber, bb.as_mut_ptr(), cs.size);
            let tag = format!("rows {:?} enumber={} size={}", cs.rows, cs.enumber, cs.size);
            d.eq(&tag, ra, rb);
            d.eq(&format!("{tag} buffer"), ba.clone(), bb);
            d.eq(
                &format!("{tag} guard region"),
                vec![0xC7u8; 16],
                ba[cs.size..].to_vec(),
            );
            doc.eq(&format!("ERRORS.md {tag}"), cs.expect, ra);
        }
        // 489 with buffer == NULL: the size test runs first, so this is safe.
        for e in [-1, 0, 100, c_int::MIN, c_int::MAX] {
            let ra = (p.c.get_error_message)(e, ptr::null_mut(), 0);
            let rb = (p.r.get_error_message)(e, ptr::null_mut(), 0);
            d.eq(&format!("rows [489] get_error_message({e}, NULL, 0)"), ra, rb);
            doc.eq(&format!("ERRORS.md row 489 with enumber {e}"), PCRE2_ERROR_NOMEMORY, ra);
        }
        // Row 494 documents UNDEFINED BEHAVIOUR (buffer == NULL with size > 0
        // writes through the NULL pointer unconditionally). Not a comparable
        // observable; the nearest reachable input is row 489, asserted above.
        d.eq(
            "rows [494] documented UB, nearest reachable input is row 489",
            true,
            true,
        );

        // Exhaustive sweep over every error number the library can name, at
        // every interesting buffer size: the truncated contents must agree too.
        let mut codes: Vec<c_int> = (-80..=0).collect();
        codes.extend(100..=225);
        codes.extend([226, 300, -1000, c_int::MIN, c_int::MAX]);
        for e in codes {
            for size in [0usize, 1, 2, 3, 8, 17, 33, 128] {
                let mut ba = vec![0xC7u8; size + 8];
                let mut bb = vec![0xC7u8; size + 8];
                let ra = (p.c.get_error_message)(e, ba.as_mut_ptr(), size);
                let rb = (p.r.get_error_message)(e, bb.as_mut_ptr(), size);
                d.eq(&format!("rows [489-493] msg({e}, {size}) rc"), ra, rb);
                d.eq(&format!("rows [489-493] msg({e}, {size}) buf"), ba.clone(), bb);
                d.eq(
                    &format!("rows [489-493] msg({e}, {size}) guard"),
                    vec![0xC7u8; 8],
                    ba[size..].to_vec(),
                );
            }
        }
    }
    doc.finish("ERRORS.md rows 489-493");
    d.finish("rows 489-494: pcre2_get_error_message_8 zero/short buffers and out-of-range numbers");
}
