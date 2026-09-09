//! Differential tests for CONFIGS.md rows C056-C077:
//!   * C056-C059 — `_pcre2_study_8` (through `pcre2_compile` and called directly)
//!   * C060-C064 — `_pcre2_auto_possessify_8` (through `pcre2_compile` and directly)
//!   * C065-C068 — the context create/copy/free family
//!   * C069      — `pcre2_maketables` / `pcre2_maketables_free` / `pcre2_set_character_tables`
//!   * C070-C074 — the validating setters
//!   * C075-C076 — `pcre2_config`
//!   * C077      — `pcre2_get_error_message`
//!
//! Everything goes through the two loaded `.so`s (see `tests/common/mod.rs`).

mod common;
use common::*;

use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

// ===================================================================== constants
// pcre2.h:486,494 — keys missing from common/mod.rs
const PCRE2_CONFIG_STACKRECURSE: u32 = 8;
const PCRE2_CONFIG_EFFECTIVE_LINKSIZE: u32 = 16;

// pcre2.h:499-507 — pcre2_set_optimize() directives
const PCRE2_OPTIMIZATION_NONE: u32 = 0;
const PCRE2_OPTIMIZATION_FULL: u32 = 1;
const PCRE2_AUTO_POSSESS: u32 = 64;
const PCRE2_AUTO_POSSESS_OFF: u32 = 65;
const PCRE2_DOTSTAR_ANCHOR: u32 = 66;
const PCRE2_DOTSTAR_ANCHOR_OFF: u32 = 67;
const PCRE2_START_OPTIMIZE: u32 = 68;
const PCRE2_START_OPTIMIZE_OFF: u32 = 69;

// pcre2_internal.h:579,594-598
const LCC_OFFSET: usize = 0;
const FCC_OFFSET: usize = 256;
const CBITS_OFFSET: usize = 512;
const CBIT_LENGTH: usize = 320;
const CTYPES_OFFSET: usize = CBITS_OFFSET + CBIT_LENGTH;
const TABLES_LENGTH: usize = CTYPES_OFFSET + 256; // 1088

// offsetof(pcre2_real_code, ...) for the LP64 layout in pcre2_intmodedep.h:660
//   memctl 0..24, tables 24, executable_jit 32, start_bitmap 40..72,
//   blocksize 72, code_start 80, magic_number 88, ...
const RC_TABLES: usize = 24;
const RC_BLOCKSIZE: usize = 72;
const RC_CODESTART: usize = 80;

// offsetof(pcre2_real_compile_context, ...) — pcre2_intmodedep.h:601
const CC_TABLES: usize = 40;
const CC_BSR: usize = 64;
const CC_NEWLINE: usize = 66;
const CC_OPTIMIZATION_FLAGS: usize = 80;
const CC_SIZE: usize = 88;

// offsetof(pcre2_real_match_context, ...) — pcre2_intmodedep.h:618 (no SUPPORT_JIT)
const MC_OFFSET_LIMIT: usize = 72;
const MC_HEAP_LIMIT: usize = 80;
const MC_MATCH_LIMIT: usize = 84;
const MC_DEPTH_LIMIT: usize = 88;
const MC_SIZE: usize = 96;

// offsetof(pcre2_real_convert_context, ...) — pcre2_intmodedep.h:640
const CVC_GLOB_SEPARATOR: usize = 24;
const CVC_GLOB_ESCAPE: usize = 28;
const CVC_SIZE: usize = 32;

const GC_SIZE: usize = 24; // pcre2_real_general_context == { pcre2_memctl }

// pcre2_internal.h:2296 / :2315 — the two PRIV entry points called directly
type StudyFn = unsafe extern "C" fn(Ptr) -> i32;
type PossessifyFn = unsafe extern "C" fn(*mut u8, *const CompileBlock) -> i32;

// ================================================================ small helpers

fn show(p: &[u8]) -> String {
    let mut s = String::new();
    for &b in p {
        if b >= 0x20 && b < 0x7f {
            s.push(b as char)
        } else {
            s.push_str(&format!("\\x{:02x}", b))
        }
    }
    s
}

fn diff(a: &CompileOut, b: &CompileOut) -> String {
    let mut s = String::new();
    if a.ok != b.ok {
        s += &format!(" ok C={} R={};", a.ok, b.ok);
    }
    if a.errorcode != b.errorcode {
        s += &format!(" errorcode C={} R={};", a.errorcode, b.errorcode);
    }
    if a.erroroffset != b.erroroffset {
        s += &format!(" erroroffset C={} R={};", a.erroroffset, b.erroroffset);
    }
    for (x, y) in a.info.iter().zip(b.info.iter()) {
        if x != y {
            s += &format!(
                " info[key {}] C=(rc {}, v {}) R=(rc {}, v {});",
                x.0, x.1, x.2, y.1, y.2
            );
        }
    }
    if a.image != b.image {
        let n = a.image.iter().zip(b.image.iter()).position(|(p, q)| p != q);
        s += &format!(
            " image len C={} R={} first-diff {:?};",
            a.image.len(),
            b.image.len(),
            n
        );
        if let Some(i) = n {
            let lo = i.saturating_sub(8);
            let hi = (i + 8).min(a.image.len().min(b.image.len()));
            s += &format!(" C{:02x?} R{:02x?}", &a.image[lo..hi], &b.image[lo..hi]);
        }
    }
    if s.is_empty() {
        s += " (identical?!)";
    }
    s
}

/// Compile the same pattern in both libraries with no compile context and compare
/// everything observable.
unsafe fn cmp_probe(id: &str, tag: &str, pat: &[u8], opts: u32) -> bool {
    let (c, r) = both();
    let a = c.compile_probe(pat, pat.len(), opts, ptr::null_mut());
    let b = r.compile_probe(pat, pat.len(), opts, ptr::null_mut());
    if a != b {
        panic!(
            "{} {}: pattern \"{}\" options {:#x}:{}",
            id,
            tag,
            show(pat),
            opts,
            diff(&a, &b)
        );
    }
    a.ok
}

/// Same, but a fresh compile context is created in each library and handed to
/// `setup` first (so both libraries get exactly the same configuration).
unsafe fn cmp_probe_ctx(
    id: &str,
    tag: &str,
    pat: &[u8],
    opts: u32,
    setup: &dyn Fn(&Api, Ptr),
) -> bool {
    let (c, r) = both();
    let cc = (c.pcre2_compile_context_create_8)(ptr::null_mut());
    let rr = (r.pcre2_compile_context_create_8)(ptr::null_mut());
    assert!(!cc.is_null() && !rr.is_null(), "{} {}: ctx create", id, tag);
    setup(c, cc);
    setup(r, rr);
    let a = c.compile_probe(pat, pat.len(), opts, cc);
    let b = r.compile_probe(pat, pat.len(), opts, rr);
    (c.pcre2_compile_context_free_8)(cc);
    (r.pcre2_compile_context_free_8)(rr);
    if a != b {
        panic!(
            "{} {}: pattern \"{}\" options {:#x}:{}",
            id,
            tag,
            show(pat),
            opts,
            diff(&a, &b)
        );
    }
    a.ok
}

/// Compile with a configured compile context in both libraries and, if the compile
/// succeeded, run `pcre2_match` over every subject comparing the full match output.
unsafe fn cmp_probe_ctx_match(
    id: &str,
    tag: &str,
    pat: &[u8],
    opts: u32,
    setup: &dyn Fn(&Api, Ptr),
    subjects: &[&[u8]],
    mopts: u32,
) {
    let (c, r) = both();
    let cc = (c.pcre2_compile_context_create_8)(ptr::null_mut());
    let rr = (r.pcre2_compile_context_create_8)(ptr::null_mut());
    assert!(!cc.is_null() && !rr.is_null(), "{} {}: ctx create", id, tag);
    setup(c, cc);
    setup(r, rr);
    let a = c.compile_probe(pat, pat.len(), opts, cc);
    let b = r.compile_probe(pat, pat.len(), opts, rr);
    if a != b {
        panic!(
            "{} {}: pattern \"{}\" options {:#x}:{}",
            id,
            tag,
            show(pat),
            opts,
            diff(&a, &b)
        );
    }
    if a.ok {
        let mut ec = 0i32;
        let mut eo = 0usize;
        let code_c = (c.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, cc);
        let code_r = (r.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, rr);
        assert!(!code_c.is_null() && !code_r.is_null());
        let md_c = (c.pcre2_match_data_create_from_pattern_8)(code_c, ptr::null_mut());
        let md_r = (r.pcre2_match_data_create_from_pattern_8)(code_r, ptr::null_mut());
        for s in subjects {
            let rc_c = (c.pcre2_match_8)(
                code_c,
                s.as_ptr(),
                s.len(),
                0,
                mopts,
                md_c,
                ptr::null_mut(),
            );
            let rc_r = (r.pcre2_match_8)(
                code_r,
                s.as_ptr(),
                s.len(),
                0,
                mopts,
                md_r,
                ptr::null_mut(),
            );
            let mc = read_match(c, md_c, rc_c);
            let mr = read_match(r, md_r, rc_r);
            cmp_match_out(
                id,
                &format!(
                    "{}: pattern \"{}\" options {:#x} mopts {:#x} subject \"{}\"",
                    tag,
                    show(pat),
                    opts,
                    mopts,
                    show(s)
                ),
                &mc,
                &mr,
            );
        }
        (c.pcre2_match_data_free_8)(md_c);
        (r.pcre2_match_data_free_8)(md_r);
        (c.pcre2_code_free_8)(code_c);
        (r.pcre2_code_free_8)(code_r);
    }
    (c.pcre2_compile_context_free_8)(cc);
    (r.pcre2_compile_context_free_8)(rr);
}

/// Compare two match results.  After a failed match (other than a partial match)
/// the ovector and startchar are documented as undefined — the two libraries use
/// the ovector as scratch during matching and leave different residue there — so
/// only the return code and the mark are compared in that case.
fn cmp_match_out(id: &str, what: &str, a: &MatchOut, b: &MatchOut) {
    assert_eq!(a.rc, b.rc, "{} {}: rc (C {:?} Rust {:?})", id, what, a, b);
    if a.rc >= 0 || a.rc == PCRE2_ERROR_PARTIAL {
        assert_eq!(a, b, "{} {}", id, what);
    } else {
        assert_eq!(a.mark, b.mark, "{} {}: mark", id, what);
    }
}

/// A heap block aligned like a `pcre2_real_code`, used to call the PRIV functions
/// directly on an exact copy of a compiled block.
struct Aligned {
    p: *mut u8,
    n: usize,
}
impl Aligned {
    unsafe fn dup(src: *const u8, n: usize) -> Aligned {
        assert!(n > 0);
        let p = std::alloc::alloc(std::alloc::Layout::from_size_align(n, 16).unwrap());
        assert!(!p.is_null());
        ptr::copy_nonoverlapping(src, p, n);
        Aligned { p, n }
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.p, self.n) }
    }
}
impl Drop for Aligned {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(
                self.p,
                std::alloc::Layout::from_size_align(self.n, 16).unwrap(),
            )
        }
    }
}

unsafe fn rd_u16(p: Ptr, off: usize) -> u16 {
    ptr::read_unaligned((p as *const u8).add(off) as *const u16)
}
unsafe fn rd_u32(p: Ptr, off: usize) -> u32 {
    ptr::read_unaligned((p as *const u8).add(off) as *const u32)
}
unsafe fn rd_usize(p: Ptr, off: usize) -> usize {
    ptr::read_unaligned((p as *const u8).add(off) as *const usize)
}
unsafe fn rd_ptr(p: Ptr, off: usize) -> *const u8 {
    ptr::read_unaligned((p as *const u8).add(off) as *const *const u8)
}
unsafe fn bytes_of(p: Ptr, n: usize) -> Vec<u8> {
    std::slice::from_raw_parts(p as *const u8, n).to_vec()
}

/// Compare two structs byte-for-byte, ignoring the given (offset,len) ranges
/// (used to skip fields that legitimately hold different pointers in the two
/// shared objects, e.g. `memctl.malloc`).
fn cmp_mem(id: &str, what: &str, a: &[u8], b: &[u8], skip: &[(usize, usize)]) {
    let mut x = a.to_vec();
    let mut y = b.to_vec();
    for &(o, l) in skip {
        for i in o..o + l {
            x[i] = 0;
            y[i] = 0;
        }
    }
    if x != y {
        let i = x.iter().zip(y.iter()).position(|(p, q)| p != q).unwrap();
        panic!(
            "{} {}: struct bytes differ at offset {}\n  C={:02x?}\n  R={:02x?}",
            id, what, i, x, y
        );
    }
}

// ------------------------------------------------------------ test allocator
#[repr(C)]
struct Stats {
    allocs: AtomicUsize,
    frees: AtomicUsize,
    bytes: AtomicUsize,
    fail: AtomicUsize,
}
impl Stats {
    const fn new() -> Stats {
        Stats {
            allocs: AtomicUsize::new(0),
            frees: AtomicUsize::new(0),
            bytes: AtomicUsize::new(0),
            fail: AtomicUsize::new(0),
        }
    }
    fn snap(&self) -> (usize, usize, usize) {
        (
            self.allocs.load(Ordering::SeqCst),
            self.frees.load(Ordering::SeqCst),
            self.bytes.load(Ordering::SeqCst),
        )
    }
}

/// libc `malloc`/`free`, reached through the C shared object's dependency chain, so
/// that the counting allocators below are interchangeable with the libraries' own
/// `default_malloc`/`default_free` (which are plain malloc/free).
type LibcMalloc = unsafe extern "C" fn(usize) -> *mut u8;
type LibcFree = unsafe extern "C" fn(*mut u8);
fn libc_fns() -> &'static (LibcMalloc, LibcFree) {
    use std::sync::OnceLock;
    static F: OnceLock<(LibcMalloc, LibcFree)> = OnceLock::new();
    F.get_or_init(|| {
        let (c, _) = both();
        unsafe { (c.raw::<LibcMalloc>("malloc"), c.raw::<LibcFree>("free")) }
    })
}

unsafe extern "C" fn t_malloc(size: usize, data: Ptr) -> Ptr {
    let st = &*(data as *const Stats);
    if st.fail.load(Ordering::SeqCst) != 0 {
        return ptr::null_mut();
    }
    let p = (libc_fns().0)(size);
    if p.is_null() {
        return ptr::null_mut();
    }
    st.allocs.fetch_add(1, Ordering::SeqCst);
    st.bytes.fetch_add(size, Ordering::SeqCst);
    p as Ptr
}

unsafe extern "C" fn t_free(block: Ptr, data: Ptr) {
    if block.is_null() {
        return;
    }
    let st = &*(data as *const Stats);
    st.frees.fetch_add(1, Ordering::SeqCst);
    (libc_fns().1)(block as *mut u8);
}

unsafe extern "C" fn null_malloc(_size: usize, _data: Ptr) -> Ptr {
    ptr::null_mut()
}
unsafe extern "C" fn noop_free(_block: Ptr, _data: Ptr) {}

// dummy callbacks used to fill in context function-pointer fields identically in
// both libraries
unsafe extern "C" fn dummy_callout(_b: Ptr, _d: Ptr) -> i32 {
    0
}
unsafe extern "C" fn dummy_sub_callout(_b: Ptr, _d: Ptr) -> i32 {
    0
}
unsafe extern "C" fn dummy_case_callout(
    _i: Sptr,
    _il: usize,
    _o: *mut u8,
    _ol: usize,
    _t: i32,
    _d: Ptr,
) -> usize {
    0
}
unsafe extern "C" fn dummy_guard(_depth: u32, _d: Ptr) -> i32 {
    0
}

// =========================================================== option matrices

/// Compile options that gate the study / auto-possessify passes.
const OPTS: &[u32] = &[
    0,
    PCRE2_NO_START_OPTIMIZE,
    PCRE2_NO_AUTO_POSSESS,
    PCRE2_NO_DOTSTAR_ANCHOR,
    PCRE2_NO_START_OPTIMIZE | PCRE2_NO_AUTO_POSSESS,
    PCRE2_UTF,
    PCRE2_UTF | PCRE2_UCP,
    PCRE2_UCP,
    PCRE2_CASELESS,
    PCRE2_CASELESS | PCRE2_UTF | PCRE2_UCP,
    PCRE2_MULTILINE,
    PCRE2_DOTALL,
    PCRE2_ANCHORED,
];

/// `pcre2_set_optimize` directive sequences applied to a compile context.
const OPT_SETS: &[&[u32]] = &[
    &[],
    &[PCRE2_OPTIMIZATION_NONE],
    &[PCRE2_OPTIMIZATION_FULL],
    &[PCRE2_AUTO_POSSESS_OFF],
    &[PCRE2_START_OPTIMIZE_OFF],
    &[PCRE2_DOTSTAR_ANCHOR_OFF],
    &[PCRE2_OPTIMIZATION_NONE, PCRE2_AUTO_POSSESS],
    &[PCRE2_OPTIMIZATION_NONE, PCRE2_START_OPTIMIZE],
    &[PCRE2_OPTIMIZATION_NONE, PCRE2_DOTSTAR_ANCHOR],
    &[
        PCRE2_OPTIMIZATION_FULL,
        PCRE2_AUTO_POSSESS_OFF,
        PCRE2_START_OPTIMIZE_OFF,
    ],
];

fn set_optimize_all(api: &Api, ctx: Ptr, dirs: &[u32]) {
    for &d in dirs {
        let rc = unsafe { (api.pcre2_set_optimize_8)(ctx, d) };
        assert_eq!(rc, 0, "set_optimize({}) on {}", d, api.tag);
    }
}

/// Drive one pattern through every option in `OPTS` and every directive set.
unsafe fn sweep(id: &str, pats: &[&str], opts: &[u32], optsets: &[&[u32]]) {
    for p in pats {
        let pat = p.as_bytes();
        for &o in opts {
            cmp_probe(id, "plain", pat, o);
            for ds in optsets {
                let d: Vec<u32> = ds.to_vec();
                cmp_probe_ctx(id, "optimize", pat, o, &move |api, ctx| {
                    set_optimize_all(api, ctx, &d)
                });
            }
        }
    }
}

// ------------------------------------------------------- random pattern maker
const ATOMS: &[&str] = &[
    "a", "b", "x", "\\d", "\\D", "\\w", "\\W", "\\s", "\\S", "\\h", "\\H", "\\v", "\\V", "\\R",
    "\\X", ".", "[ab]", "[^ab]", "[a-z]", "[abc]", "[Ww]", "[[:alnum:]]", "[[:space:]]",
    "[[:word:]]", "[[:punct:]]", "\\p{L}", "\\P{L}", "\\p{Lu}", "\\p{N}", "\\p{Nd}", "\\p{Z}",
    "\\p{Cc}", "\\p{Han}", "\\p{Latin}", "\\x{e9}", "\\x{100}", "[\\x{100}a]", "[\\p{L}]",
    "[^\\p{L}]", "(a)", "(?:ab)", "(?i)a", "\\1", "\\C", "\\Z", "\\z", "$", "^", "\\A", "\\G",
    "\\b", "\\K", "(?=a)", "(?!a)", "(?<=ab)", "(?<!ab)", "(a|bb)", "k", "\\x{17f}",
];
const QUANTS: &[&str] = &[
    "", "", "", "*", "*?", "*+", "+", "+?", "++", "?", "??", "?+", "{0,3}", "{0,3}?", "{2,4}",
    "{3}", "{1,}",
];

fn rand_pattern(rng: &mut Rng) -> Vec<u8> {
    let n = 1 + rng.below(4) as usize;
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(rng.pick(ATOMS));
        s.push_str(rng.pick(QUANTS));
    }
    s.into_bytes()
}

/// `n` random patterns compared under a random option combination.
unsafe fn random_sweep(id: &str, seed: u64, n: usize) {
    let mut rng = Rng::new(seed);
    for _ in 0..n {
        let pat = rand_pattern(&mut rng);
        let o = *rng.pick(OPTS);
        cmp_probe(id, "random", &pat, o);
        let ds: Vec<u32> = rng.pick(OPT_SETS).to_vec();
        cmp_probe_ctx(id, "random+optimize", &pat, o, &move |api, ctx| {
            set_optimize_all(api, ctx, &ds)
        });
    }
}

// ======================================================= C056 - C059  study

#[test]
fn c056_study_skipped_when_first_or_startline_set() {
    println!("C056 _pcre2_study_8: set_start_bits skipped (FIRSTSET|STARTLINE)");
    let pats = &["abc", "^abc", "\\Aabc", "abc|abd", "^a", "(?m)^abc", "a"];
    unsafe {
        sweep("C056", pats, OPTS, OPT_SETS);
        // explicit shapes named by the row
        cmp_probe("C056", "abc", b"abc", 0);
        cmp_probe("C056", "^abc multiline", b"^abc", PCRE2_MULTILINE);
        random_sweep("C056", 0x5601, 200);
    }
}

#[test]
fn c057_study_first_code_unit_collapse() {
    println!("C057 _pcre2_study_8: bitmap collapse to first code unit / LASTSET clearing");
    let pats = &[
        "[Ww]ord",
        "(word|WORD)",
        "[ab]c",
        "[abc]x",
        "[\\x{e9}\\x{ff}]x",
        "a*a",
        "a*ab",
        "[Aa]",
        "(?i)k",
        "(?i)\\x{17f}",
        "(?i)s",
        "[Kk]x",
        "[\\x{212a}Kk]x",
        "(?:a|A)b",
        "[qQ]",
        "[0-9]x",
    ];
    unsafe {
        sweep("C057", pats, OPTS, OPT_SETS);
        random_sweep("C057", 0x5701, 200);
    }
}

#[test]
fn c058_study_start_bits_give_up() {
    println!("C058 _pcre2_study_8: SSB_FAIL / SSB_CONTINUE shapes");
    let pats = &[
        "(a)\\1x",
        "(?i)(a)\\1x",
        "(?<=ab)x",
        "(?<=ab|abc)x",
        "(?(R)a)x",
        "(x)(?(1)a?)*x",
        "a\\Kx",
        "a(*SKIP)x",
        "a(*SKIP:n)x",
        "\\Ax",
        "\\Gx",
        "a(*THEN)x",
        "a(*THEN:n)x",
        "\\p{L}x",
        "\\p{Lu}?x",
        "[[a-z]&&[^q]]x",
        "[\\p{L}]x",
        "[^\\p{L}]x",
        "\\p{Any}x",
        ".*x",
        "\\C x",
        "(?i)[\\x{100}-\\x{200}]x",
        "\\R x",
        "\\X x",
    ];
    unsafe {
        sweep("C058", pats, OPTS, OPT_SETS);
        // the extended-class shapes need PCRE2_ALT_EXTENDED_CLASS to compile
        for p in &["[[a-z]&&[^q]]x", "[[a-z]&&[^q]]*x"] {
            for &o in &[
                PCRE2_ALT_EXTENDED_CLASS,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_UCP,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_CASELESS,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_NO_START_OPTIMIZE,
            ] {
                cmp_probe("C058", "eclass", p.as_bytes(), o);
            }
        }
        random_sweep("C058", 0x5801, 200);
    }
}

#[test]
fn c059_study_minlength() {
    println!("C059 _pcre2_study_8: find_minlength shapes");
    // >128 back references disables the minlength phase
    let mut many_refs = String::new();
    for _ in 0..130 {
        many_refs.push_str("(a)");
    }
    for i in 1..=130 {
        many_refs.push_str(&format!("\\{}", i));
    }
    // >1000 find_minlength() calls trips the complexity counter
    let mut many_groups = String::new();
    for _ in 0..1100 {
        many_groups.push_str("(a)");
    }
    let owned = vec![
        "abc".to_string(),
        "a{2,5}".to_string(),
        "abc|de".to_string(),
        "(ab)\\1".to_string(),
        "(?:a{60000}){2}".to_string(),
        "(?:a{60000}){2}b".to_string(),
        "a*".to_string(),
        "a(*ACCEPT)b".to_string(),
        "a(*ACCEPT)".to_string(),
        "a\\Cb".to_string(),
        "(a)(b)\\1\\2".to_string(),
        many_refs,
        many_groups,
    ];
    let pats: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    unsafe {
        sweep("C059", &pats, OPTS, &OPT_SETS[..4]);
        random_sweep("C059", 0x5901, 200);
    }
}

#[test]
fn c056_c059_study_direct_ffi() {
    println!("C056-C059 _pcre2_study_8 called DIRECTLY on a compiled block");
    let (c, r) = both();
    unsafe {
        let study_c: StudyFn = c.raw("_pcre2_study_8");
        let study_r: StudyFn = r.raw("_pcre2_study_8");

        let mut fixed: Vec<String> = vec![
            "abc", "^abc", "[Ww]ord", "(word|WORD)", "[ab]c", "[abc]x", "a*a", "(a)\\1x",
            "(?<=ab)x", "\\p{L}x", "[\\p{L}]x", "a\\Kx", "a(*SKIP)x", "\\Ax", "\\Gx", "abc|de",
            "(ab)\\1", "a{2,5}", "a*", "a(*ACCEPT)b", "[\\x{e9}\\x{ff}]x", ".*x", "\\R", "\\X",
            "(?i)k", "[^ab]c", "(a|bb)c", "a?b", "\\d+\\D",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
        let mut rng = Rng::new(0xBEEF);
        for _ in 0..120 {
            fixed.push(String::from_utf8_lossy(&rand_pattern(&mut rng)).into_owned());
        }

        let mut done = 0usize;
        let mut changed = 0usize;
        for p in &fixed {
            let pat = p.as_bytes();
            for &o in &[
                PCRE2_NO_START_OPTIMIZE,
                PCRE2_NO_START_OPTIMIZE | PCRE2_UTF,
                PCRE2_NO_START_OPTIMIZE | PCRE2_UCP,
                PCRE2_NO_START_OPTIMIZE | PCRE2_CASELESS,
                PCRE2_NO_START_OPTIMIZE | PCRE2_MULTILINE,
            ] {
                let mut ec = 0i32;
                let mut eo = 0usize;
                // Compile in the C library with the start optimizations disabled, so
                // PRIV(study) has NOT run on this block yet.
                let code = (c.pcre2_compile_8)(
                    pat.as_ptr(),
                    pat.len(),
                    o,
                    &mut ec,
                    &mut eo,
                    ptr::null_mut(),
                );
                if code.is_null() {
                    continue;
                }
                let mut blocksize: usize = 0;
                assert_eq!(
                    (c.pcre2_pattern_info_8)(
                        code,
                        PCRE2_INFO_SIZE,
                        &mut blocksize as *mut usize as Ptr
                    ),
                    0
                );
                assert_eq!(rd_usize(code, RC_BLOCKSIZE), blocksize, "blocksize offset");
                let code_start = rd_usize(code, RC_CODESTART);
                assert!(code_start < blocksize, "code_start sanity");

                let ca = Aligned::dup(code as *const u8, blocksize);
                let ra = Aligned::dup(code as *const u8, blocksize);
                let before = ca.bytes().to_vec();
                let rc_c = study_c(ca.p as Ptr);
                let rc_r = study_r(ra.p as Ptr);
                assert_eq!(
                    rc_c,
                    rc_r,
                    "C056-C059 direct study rc: pattern \"{}\" options {:#x}",
                    show(pat),
                    o
                );
                if ca.bytes() != ra.bytes() {
                    let i = ca
                        .bytes()
                        .iter()
                        .zip(ra.bytes())
                        .position(|(x, y)| x != y)
                        .unwrap();
                    let lo = i.saturating_sub(8);
                    let hi = (i + 8).min(blocksize);
                    panic!(
                        "C056-C059 direct study: pattern \"{}\" options {:#x}: block differs at \
                         byte {} (code_start {}): C{:02x?} R{:02x?}",
                        show(pat),
                        o,
                        i,
                        code_start,
                        &ca.bytes()[lo..hi],
                        &ra.bytes()[lo..hi]
                    );
                }
                if before != ca.bytes() {
                    changed += 1;
                }
                (c.pcre2_code_free_8)(code);
                done += 1;
            }
        }
        println!(
            "  C056-C059 direct study: {} block comparisons, {} blocks modified by study",
            done, changed
        );
        assert!(done > 100);
        assert!(changed > 50, "direct study never modified a block");
    }
}

// ============================================== C060 - C064  auto-possessify

/// `compile_block` — pcre2_intmodedep.h:802.  Only `fcc`, `cbits`, `ctypes`,
/// `start_code`, `external_options` and `had_recurse` are read by
/// `PRIV(auto_possessify)`; everything else is zero.
#[repr(C)]
#[derive(Clone, Copy)]
struct CompileBlock {
    cx: *mut u8,
    lcc: *const u8,
    fcc: *const u8,
    cbits: *const u8,
    ctypes: *const u8,
    start_workspace: *mut u8,
    start_code: *mut u8,
    start_pattern: *const u8,
    end_pattern: *const u8,
    name_table: *mut u8,
    workspace_size: usize,
    small_ref_offset: [usize; 10],
    erroroffset: usize,
    classbits: [u32; 8],
    names_found: u16,
    name_entry_size: u16,
    parens_depth: u16,
    assert_depth: u16,
    named_groups: *mut u8,
    named_group_list_size: u32,
    external_options: u32,
    external_flags: u32,
    bracount: u32,
    lastcapture: u32,
    parsed_pattern: *mut u32,
    parsed_pattern_end: *mut u32,
    groupinfo: *mut u32,
    top_backref: u32,
    backref_map: u32,
    nltype: u32,
    nllen: u32,
    nl: [u8; 4],
    class_op_used: [u8; 15],
    req_varyopt: u32,
    max_varlookbehind: u32,
    max_lookbehind: i32,
    had_accept: i32,
    had_pruneorskip: i32,
    had_recurse: i32,
    dupnames: i32,
    first_data: *mut u8,
    last_data: *mut u8,
    char_lists_size: usize,
}

impl CompileBlock {
    fn zeroed() -> CompileBlock {
        unsafe { std::mem::zeroed() }
    }
}

const AP_PATS_060: &[&str] = &[
    "a*b", "a*?b", "a+b", "a+?b", "a?b", "a??b", "a{0,3}b", "a{0,3}?b", "a{2,5}b", "a{2,5}?b",
    "\\d*\\D", "\\d+?\\D", "\\w?\\W", "\\s{0,3}\\S",
];
const AP_PATS_061: &[&str] = &[
    "[ab]*c",
    "[ab]*?c",
    "[ab]+c",
    "[ab]+?c",
    "[ab]?c",
    "[ab]??c",
    "[ab]{0,3}c",
    "[ab]{0,3}?c",
    "[^ab]*c",
    "[^ab]+c",
    "[^ab]?c",
    "[^ab]{0,3}c",
    "[\\x{100}a]*c",
    "[\\x{100}a]+c",
    "[\\x{100}a]?c",
    "[\\x{100}a]{0,3}c",
    // the same eight repeat forms where the follower is NOT in the class, so the
    // rewrite actually happens for a negated class and for an XCLASS
    "[^ab]*a",
    "[^ab]*?a",
    "[^ab]+a",
    "[^ab]+?a",
    "[^ab]?a",
    "[^ab]??a",
    "[^ab]{0,3}a",
    "[^ab]{0,3}?a",
    "[\\x{100}a]*b",
    "[\\x{100}a]*?b",
    "[\\x{100}a]+b",
    "[\\x{100}a]+?b",
    "[\\x{100}a]?b",
    "[\\x{100}a]??b",
    "[\\x{100}a]{0,3}b",
    "[\\x{100}a]{0,3}?b",
];
const AP_PATS_062: &[&str] = &[
    "\\p{L}+\\P{L}",
    "\\p{Han}+\\p{Latin}",
    "\\p{L}+\\p{N}",
    "\\p{L}+\\p{Nd}",
    "\\p{Nd}+\\p{L}",
    "[[:alnum:]]+\\p{Z}",
    "[[:space:]]+\\p{Cc}",
    "[[:word:]]+\\p{L}",
    "\\p{Bidi_Control}+\\p{L}",
    "\\p{bc=AL}+\\p{L}",
    "(?i)k+\\p{L}",
    "\\p{Lu}+\\p{Ll}",
    "\\p{Greek}+\\p{Latin}",
    "\\p{Xan}+\\p{Xsp}",
    "\\p{L&}+\\p{N}",
];
const AP_PATS_063: &[&str] = &[
    "\\d+\\d",
    "\\w+\\w",
    ".+.",
    "\\C+x",
    "a{3}b",
    "a++b",
    "\\d++\\D",
    "[a-z]+[c-x]",
    "a*a",
    "\\s+\\s",
    "[ab]*+c",
    "\\p{L}+\\p{L}",
    "\\X+\\X",
];
/// C064 — one pattern per autoposstab cell that yields 1
const AP_PATS_064: &[&str] = &[
    "\\d+\\D", "\\d+\\s", "\\d+\\W", "\\d+\\R", "\\d+\\h", "\\d+\\v", "\\d+\\Z", "\\d+\\z",
    "\\d+$", "\\D+\\d", "\\S+\\s", "\\s+\\d", "\\s+\\S", "\\s+\\w", "\\W+\\d", "\\W+\\w",
    "\\w+\\s", "\\w+\\W", "\\w+\\R", ".+\\R", "\\R+\\d", "\\R+\\s", "\\R+\\w", "\\R+.", "\\R+\\h",
    "\\H+\\h", "\\h+\\d", "\\h+\\S", "\\h+\\w", "\\h+\\R", "\\h+\\H", "\\h+\\v", "\\V+\\R",
    "\\V+\\v", "\\v+\\d", "\\v+\\S", "\\v+\\w", "\\v+\\h", "\\v+\\V", "\\X+\\z",
];

/// The compiled opcode stream only (`re + re->code_start` .. `re->blocksize`), so
/// that switching an option bit — which lands in the header — does not by itself
/// show up as a difference.
unsafe fn code_bytes(api: &Api, pat: &[u8], opts: u32) -> Option<Vec<u8>> {
    let mut ec = 0i32;
    let mut eo = 0usize;
    let code = (api.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, ptr::null_mut());
    if code.is_null() {
        return None;
    }
    let mut blocksize: usize = 0;
    (api.pcre2_pattern_info_8)(code, PCRE2_INFO_SIZE, &mut blocksize as *mut usize as Ptr);
    let cs = rd_usize(code, RC_CODESTART);
    let v = std::slice::from_raw_parts((code as *const u8).add(cs), blocksize - cs).to_vec();
    (api.pcre2_code_free_8)(code);
    Some(v)
}

/// Count how many of `pats` have a different opcode stream when auto-possessification
/// is switched off — proves the pass is actually being exercised.
unsafe fn count_changed(api: &Api, pats: &[&str], opts: u32) -> usize {
    let mut n = 0;
    for p in pats {
        let pat = p.as_bytes();
        let on = code_bytes(api, pat, opts);
        let off = code_bytes(api, pat, opts | PCRE2_NO_AUTO_POSSESS);
        if let (Some(a), Some(b)) = (on, off) {
            if a != b {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn c060_autopossess_single_char_repeats() {
    println!("C060 _pcre2_auto_possessify_8: OP_STAR..OP_MINUPTO rewrites");
    unsafe {
        sweep("C060", AP_PATS_060, OPTS, OPT_SETS);
        // (*NO_AUTO_POSSESS) in the pattern must have the same effect
        for p in AP_PATS_060 {
            let s = format!("(*NO_AUTO_POSSESS){}", p);
            cmp_probe("C060", "verb", s.as_bytes(), 0);
            cmp_probe("C060", "verb+utf", s.as_bytes(), PCRE2_UTF | PCRE2_UCP);
        }
        let (c, r) = both();
        let nc = count_changed(c, AP_PATS_060, 0);
        let nr = count_changed(r, AP_PATS_060, 0);
        println!("  possessified (image changed) C={} R={}", nc, nr);
        assert_eq!(nc, nr, "C060: number of patterns rewritten differs");
        assert!(nc > 0, "C060: auto-possessification never fired in C");
        random_sweep("C060", 0x6001, 200);
    }
}

#[test]
fn c061_autopossess_class_repeats() {
    println!("C061 _pcre2_auto_possessify_8: OP_CRSTAR..OP_CRMINRANGE rewrites");
    unsafe {
        sweep("C061", AP_PATS_061, OPTS, OPT_SETS);
        // ECLASS shapes need PCRE2_ALT_EXTENDED_CLASS
        for p in &[
            "[[a-z]&&[^q]]*c",
            "[[a-z]&&[^q]]+c",
            "[[a-z]&&[^q]]?c",
            "[[a-z]&&[^q]]{0,3}c",
            "[[a-z]&&[^q]]*?c",
            "[[a-z]&&[^q]]*Q",
            "[[a-z]&&[^q]]*?Q",
            "[[a-z]&&[^q]]+Q",
            "[[a-z]&&[^q]]+?Q",
            "[[a-z]&&[^q]]?Q",
            "[[a-z]&&[^q]]??Q",
            "[[a-z]&&[^q]]{0,3}Q",
            "[[a-z]&&[^q]]{0,3}?Q",
        ] {
            for &o in &[
                PCRE2_ALT_EXTENDED_CLASS,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_UCP,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_NO_AUTO_POSSESS,
                PCRE2_ALT_EXTENDED_CLASS | PCRE2_CASELESS,
            ] {
                cmp_probe("C061", "eclass", p.as_bytes(), o);
            }
        }
        let (c, r) = both();
        let nc = count_changed(c, AP_PATS_061, 0);
        let nr = count_changed(r, AP_PATS_061, 0);
        println!("  possessified (image changed) C={} R={}", nc, nr);
        assert_eq!(nc, nr, "C061: number of patterns rewritten differs");
        assert!(nc >= 16, "C061: only {} class repeats were rewritten", nc);
        random_sweep("C061", 0x6101, 200);
    }
}

#[test]
fn c062_autopossess_property_pairs() {
    println!("C062 _pcre2_auto_possessify_8: propposstab / catposstab / posspropstab");
    unsafe {
        sweep("C062", AP_PATS_062, OPTS, OPT_SETS);
        let (c, r) = both();
        for &o in &[0, PCRE2_UTF, PCRE2_UCP, PCRE2_UTF | PCRE2_UCP] {
            let nc = count_changed(c, AP_PATS_062, o);
            let nr = count_changed(r, AP_PATS_062, o);
            println!("  options {:#x}: possessified C={} R={}", o, nc, nr);
            assert_eq!(nc, nr, "C062: rewrite count differs for options {:#x}", o);
        }
        random_sweep("C062", 0x6201, 200);
    }
}

#[test]
fn c063_autopossess_declined() {
    println!("C063 _pcre2_auto_possessify_8: shapes that must NOT be possessified");
    let (c, r) = both();
    unsafe {
        sweep("C063", AP_PATS_063, OPTS, OPT_SETS);
        // the row's claim: switching the pass off changes nothing for these
        for p in AP_PATS_063 {
            let pat = p.as_bytes();
            for &o in &[0u32, PCRE2_UTF, PCRE2_UCP, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS] {
                for api in [c, r] {
                    // compare the opcode stream only: the option bit itself lands in
                    // the header, so the full image always differs
                    let on = code_bytes(api, pat, o);
                    let off = code_bytes(api, pat, o | PCRE2_NO_AUTO_POSSESS);
                    if on.is_none() {
                        continue;
                    }
                    assert_eq!(
                        on,
                        off,
                        "C063 [{}]: pattern \"{}\" options {:#x} WAS possessified",
                        api.tag,
                        show(pat),
                        o
                    );
                }
            }
        }
        random_sweep("C063", 0x6301, 200);
    }
}

#[test]
fn c064_autoposstab_rows() {
    println!("C064 _pcre2_auto_possessify_8: the 41 autoposstab cells");
    unsafe {
        sweep("C064", AP_PATS_064, OPTS, OPT_SETS);
        // the $ / $M cells
        for p in &["\\d+$", "\\w+$", "\\s+$", "\\R+$", "\\h+$", "\\v+$", "\\X+$"] {
            for &o in &[
                0,
                PCRE2_MULTILINE,
                PCRE2_MULTILINE | PCRE2_UTF,
                PCRE2_DOLLAR_ENDONLY,
                PCRE2_MULTILINE | PCRE2_DOLLAR_ENDONLY,
                PCRE2_NO_AUTO_POSSESS,
                PCRE2_MULTILINE | PCRE2_NO_AUTO_POSSESS,
            ] {
                cmp_probe("C064", "dollar", p.as_bytes(), o);
            }
        }
        let (c, r) = both();
        let nc = count_changed(c, AP_PATS_064, 0);
        let nr = count_changed(r, AP_PATS_064, 0);
        println!("  possessified (image changed) C={}/{} R={}", nc, AP_PATS_064.len(), nr);
        assert_eq!(nc, nr, "C064: rewrite count differs");
        assert!(nc >= 35, "C064: expected nearly every cell to fire, got {}", nc);
        random_sweep("C064", 0x6401, 200);
    }
}

#[test]
fn c060_c064_autopossess_direct_ffi() {
    println!("C060-C064 _pcre2_auto_possessify_8 called DIRECTLY on a compiled block");
    let (c, r) = both();
    unsafe {
        let poss_c: PossessifyFn = c.raw("_pcre2_auto_possessify_8");
        let poss_r: PossessifyFn = r.raw("_pcre2_auto_possessify_8");

        let mut pats: Vec<String> = vec![];
        for src in [AP_PATS_060, AP_PATS_061, AP_PATS_062, AP_PATS_063, AP_PATS_064] {
            for p in src {
                pats.push(p.to_string());
            }
        }
        let mut rng = Rng::new(0xC0DE);
        for _ in 0..120 {
            pats.push(String::from_utf8_lossy(&rand_pattern(&mut rng)).into_owned());
        }

        let mut done = 0usize;
        let mut changed = 0usize;
        for p in &pats {
            let pat = p.as_bytes();
            for &o in &[
                PCRE2_NO_AUTO_POSSESS,
                PCRE2_NO_AUTO_POSSESS | PCRE2_UTF,
                PCRE2_NO_AUTO_POSSESS | PCRE2_UCP,
                PCRE2_NO_AUTO_POSSESS | PCRE2_UTF | PCRE2_UCP,
                PCRE2_NO_AUTO_POSSESS | PCRE2_CASELESS,
            ] {
                let mut ec = 0i32;
                let mut eo = 0usize;
                // Compile in C with the pass disabled, so the block is un-possessified.
                let code = (c.pcre2_compile_8)(
                    pat.as_ptr(),
                    pat.len(),
                    o,
                    &mut ec,
                    &mut eo,
                    ptr::null_mut(),
                );
                if code.is_null() {
                    continue;
                }
                let mut blocksize: usize = 0;
                (c.pcre2_pattern_info_8)(code, PCRE2_INFO_SIZE, &mut blocksize as *mut usize as Ptr);
                let code_start = rd_usize(code, RC_CODESTART);
                let tables = rd_ptr(code, RC_TABLES);
                assert!(!tables.is_null(), "re->tables");

                let ca = Aligned::dup(code as *const u8, blocksize);
                let ra = Aligned::dup(code as *const u8, blocksize);
                let before = ca.bytes().to_vec();

                let mut cb = CompileBlock::zeroed();
                cb.lcc = tables.add(LCC_OFFSET);
                cb.fcc = tables.add(FCC_OFFSET);
                cb.cbits = tables.add(CBITS_OFFSET);
                cb.ctypes = tables.add(CTYPES_OFFSET);
                // external_options is what compile passed in; the pass only reads the
                // UTF and UCP bits out of it.
                cb.external_options = o;
                cb.had_recurse = 0;

                let mut cb_c = cb;
                cb_c.start_code = ca.p.add(code_start);
                let mut cb_r = cb;
                cb_r.start_code = ra.p.add(code_start);

                let rc_c = poss_c(ca.p.add(code_start), &cb_c);
                let rc_r = poss_r(ra.p.add(code_start), &cb_r);
                assert_eq!(
                    rc_c,
                    rc_r,
                    "C060-C064 direct possessify rc: pattern \"{}\" options {:#x}",
                    show(pat),
                    o
                );
                if ca.bytes() != ra.bytes() {
                    let i = ca
                        .bytes()
                        .iter()
                        .zip(ra.bytes())
                        .position(|(x, y)| x != y)
                        .unwrap();
                    let lo = i.saturating_sub(8);
                    let hi = (i + 8).min(blocksize);
                    panic!(
                        "C060-C064 direct possessify: pattern \"{}\" options {:#x}: block differs \
                         at byte {} (code_start {}): C{:02x?} R{:02x?}",
                        show(pat),
                        o,
                        i,
                        code_start,
                        &ca.bytes()[lo..hi],
                        &ra.bytes()[lo..hi]
                    );
                }
                if before != ca.bytes() {
                    changed += 1;
                }
                (c.pcre2_code_free_8)(code);
                done += 1;
            }
        }
        println!(
            "  C060-C064 direct possessify: {} comparisons, {} blocks rewritten",
            done, changed
        );
        assert!(done > 200);
        assert!(changed > 20, "direct possessify never rewrote anything");
    }
}

#[test]
fn c058_c063_deep_nesting() {
    println!("C058/C063 depth limits: 1001 nested groups (SSB_TOODEEP, rec_limit)");
    // run on a big stack: 1001 nested groups recurse deeply in both libraries
    let h = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| unsafe {
            let mut deep = String::new();
            for _ in 0..1001 {
                deep.push('(');
            }
            deep.push('a');
            for _ in 0..1001 {
                deep.push(')');
            }
            let deep_q = format!("{}*b", deep);
            let raise = |api: &Api, ctx: Ptr| {
                assert_eq!((api.pcre2_set_parens_nest_limit_8)(ctx, 4000), 0);
            };
            for p in [deep.as_str(), deep_q.as_str()] {
                for &o in &[
                    0u32,
                    PCRE2_NO_START_OPTIMIZE,
                    PCRE2_NO_AUTO_POSSESS,
                    PCRE2_UTF,
                ] {
                    let ok = cmp_probe_ctx("C058/C063", "deep", p.as_bytes(), o, &raise);
                    println!("  1001 nested groups, options {:#x}: compiled = {}", o, ok);
                    assert!(ok, "C058/C063: the deep pattern must compile (parens_nest_limit raised)");
                }
            }
            // and below the limit, for contrast
            let mut shallow = String::new();
            for _ in 0..200 {
                shallow.push('(');
            }
            shallow.push('a');
            for _ in 0..200 {
                shallow.push(')');
            }
            cmp_probe_ctx("C058/C063", "shallow", shallow.as_bytes(), 0, &raise);
        })
        .unwrap();
    h.join().unwrap();
}

// ================================================== C065 - C068  contexts

#[test]
fn c065_general_context() {
    println!("C065 pcre2_general_context_create/copy/free");
    let (c, r) = both();
    static ST_C: Stats = Stats::new();
    static ST_R: Stats = Stats::new();
    unsafe {
        // (NULL,NULL,NULL) — defaults installed
        let gc = (c.pcre2_general_context_create_8)(None, None, ptr::null_mut());
        let gr = (r.pcre2_general_context_create_8)(None, None, ptr::null_mut());
        assert!(!gc.is_null() && !gr.is_null(), "C065: default create");
        // the two malloc/free pointers must be that library's own defaults, and
        // memory_data must be NULL in both
        assert!(!rd_ptr(gc, 0).is_null() && !rd_ptr(gc, 8).is_null());
        assert!(!rd_ptr(gr, 0).is_null() && !rd_ptr(gr, 8).is_null());
        assert!(rd_ptr(gc, 16).is_null(), "C065: C memory_data");
        assert!(rd_ptr(gr, 16).is_null(), "C065: Rust memory_data");
        // copy and free
        let gc2 = (c.pcre2_general_context_copy_8)(gc);
        let gr2 = (r.pcre2_general_context_copy_8)(gr);
        assert!(!gc2.is_null() && !gr2.is_null());
        cmp_mem(
            "C065",
            "copy(default) == original [C]",
            &bytes_of(gc, GC_SIZE),
            &bytes_of(gc2, GC_SIZE),
            &[],
        );
        cmp_mem(
            "C065",
            "copy(default) == original [Rust]",
            &bytes_of(gr, GC_SIZE),
            &bytes_of(gr2, GC_SIZE),
            &[],
        );
        (c.pcre2_general_context_free_8)(gc2);
        (r.pcre2_general_context_free_8)(gr2);
        (c.pcre2_general_context_free_8)(gc);
        (r.pcre2_general_context_free_8)(gr);

        // NULL is a no-op for free
        (c.pcre2_general_context_free_8)(ptr::null_mut());
        (r.pcre2_general_context_free_8)(ptr::null_mut());

        // one NULL function pointer at a time (defaults substituted)
        for (m, f) in [
            (None, Some(t_free as unsafe extern "C" fn(Ptr, Ptr))),
            (Some(t_malloc as unsafe extern "C" fn(usize, Ptr) -> Ptr), None),
        ] {
            let a = (c.pcre2_general_context_create_8)(m, f, &ST_C as *const Stats as Ptr);
            let b = (r.pcre2_general_context_create_8)(m, f, &ST_R as *const Stats as Ptr);
            assert!(!a.is_null() && !b.is_null(), "C065: half-custom create");
            // the supplied pointer must have been stored verbatim; the missing one
            // replaced by that library's default (non-NULL, and equal in the two
            // libraries only when it is our own function)
            if m.is_some() {
                assert_eq!(rd_ptr(a, 0), t_malloc as *const u8);
                assert_eq!(rd_ptr(b, 0), t_malloc as *const u8);
            } else {
                assert_eq!(rd_ptr(a, 8), t_free as *const u8);
                assert_eq!(rd_ptr(b, 8), t_free as *const u8);
            }
            assert_eq!(rd_ptr(a, 16), &ST_C as *const Stats as *const u8);
            assert_eq!(rd_ptr(b, 16), &ST_R as *const Stats as *const u8);
            (c.pcre2_general_context_free_8)(a);
            (r.pcre2_general_context_free_8)(b);
        }

        // full custom pair: identical byte images (both hold OUR pointers)
        let gc = (c.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_C as *const Stats as Ptr,
        );
        let gr = (r.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_R as *const Stats as Ptr,
        );
        assert!(!gc.is_null() && !gr.is_null());
        cmp_mem(
            "C065",
            "custom context image",
            &bytes_of(gc, GC_SIZE),
            &bytes_of(gr, GC_SIZE),
            &[(16, 8)], // memory_data intentionally differs (per-library Stats)
        );
        assert_eq!(rd_ptr(gc, 0), t_malloc as *const u8);
        assert_eq!(rd_ptr(gr, 0), t_malloc as *const u8);
        assert_eq!(rd_ptr(gc, 8), t_free as *const u8);
        assert_eq!(rd_ptr(gr, 8), t_free as *const u8);

        // copy through the custom allocator
        let gc2 = (c.pcre2_general_context_copy_8)(gc);
        let gr2 = (r.pcre2_general_context_copy_8)(gr);
        assert!(!gc2.is_null() && !gr2.is_null());
        cmp_mem(
            "C065",
            "custom copy",
            &bytes_of(gc2, GC_SIZE),
            &bytes_of(gr2, GC_SIZE),
            &[(16, 8)],
        );
        (c.pcre2_general_context_free_8)(gc2);
        (r.pcre2_general_context_free_8)(gr2);

        // use the custom context for real allocations
        let mdc = (c.pcre2_match_data_create_8)(4, gc);
        let mdr = (r.pcre2_match_data_create_8)(4, gr);
        assert!(!mdc.is_null() && !mdr.is_null());
        assert_eq!(
            (c.pcre2_get_ovector_count_8)(mdc),
            (r.pcre2_get_ovector_count_8)(mdr)
        );
        assert_eq!(
            (c.pcre2_get_match_data_size_8)(mdc),
            (r.pcre2_get_match_data_size_8)(mdr)
        );
        (c.pcre2_match_data_free_8)(mdc);
        (r.pcre2_match_data_free_8)(mdr);

        let ccc = (c.pcre2_compile_context_create_8)(gc);
        let ccr = (r.pcre2_compile_context_create_8)(gr);
        assert!(!ccc.is_null() && !ccr.is_null());
        // the memctl was overridden from the gcontext...
        assert_eq!(rd_ptr(ccc, 0), t_malloc as *const u8);
        assert_eq!(rd_ptr(ccr, 0), t_malloc as *const u8);
        // ...and everything after it is still the default
        let a = c.compile_probe(b"a(b)c\\1", 6, 0, ccc);
        let b = r.compile_probe(b"a(b)c\\1", 6, 0, ccr);
        assert_eq!(a, b, "C065: compile with custom-allocator ccontext:{}", diff(&a, &b));

        // a malloc that fails: every allocation through this context must fail
        ST_C.fail.store(1, Ordering::SeqCst);
        ST_R.fail.store(1, Ordering::SeqCst);
        let a = c.compile_probe(b"abc", 3, 0, ccc);
        let b = r.compile_probe(b"abc", 3, 0, ccr);
        assert_eq!(a, b, "C065: compile with failing malloc:{}", diff(&a, &b));
        assert!(!a.ok, "C065: compile should fail when malloc returns NULL");
        assert!(
            (c.pcre2_compile_context_create_8)(gc).is_null(),
            "C065: C compile_context_create with failing malloc"
        );
        assert!(
            (r.pcre2_compile_context_create_8)(gr).is_null(),
            "C065: Rust compile_context_create with failing malloc"
        );
        assert!((c.pcre2_general_context_copy_8)(gc).is_null());
        assert!((r.pcre2_general_context_copy_8)(gr).is_null());
        assert!((c.pcre2_maketables_8)(gc).is_null());
        assert!((r.pcre2_maketables_8)(gr).is_null());
        assert!((c.pcre2_match_data_create_8)(4, gc).is_null());
        assert!((r.pcre2_match_data_create_8)(4, gr).is_null());
        ST_C.fail.store(0, Ordering::SeqCst);
        ST_R.fail.store(0, Ordering::SeqCst);

        (c.pcre2_compile_context_free_8)(ccc);
        (r.pcre2_compile_context_free_8)(ccr);
        (c.pcre2_general_context_free_8)(gc);
        (r.pcre2_general_context_free_8)(gr);

        // create with a malloc that always returns NULL -> NULL context
        let a = (c.pcre2_general_context_create_8)(
            Some(null_malloc),
            Some(noop_free),
            ptr::null_mut(),
        );
        let b = (r.pcre2_general_context_create_8)(
            Some(null_malloc),
            Some(noop_free),
            ptr::null_mut(),
        );
        assert!(a.is_null() && b.is_null(), "C065: create with failing malloc");

        // allocation accounting must match (both libraries did the same work)
        let sc = ST_C.snap();
        let sr = ST_R.snap();
        println!("  allocator: C {:?} Rust {:?} (allocs, frees, bytes)", sc, sr);
        assert_eq!(sc.0, sr.0, "C065: malloc call count differs");
        assert_eq!(sc.1, sr.1, "C065: free call count differs");
        assert_eq!(sc.2, sr.2, "C065: total bytes requested differs");
        assert_eq!(sc.0, sc.1, "C065: C leaked (allocs != frees)");
        assert_eq!(sr.0, sr.1, "C065: Rust leaked (allocs != frees)");
    }
}

#[test]
fn c066_compile_context() {
    println!("C066 pcre2_compile_context_create/copy/free");
    let (c, r) = both();
    static ST_C: Stats = Stats::new();
    static ST_R: Stats = Stats::new();
    unsafe {
        // create(NULL) must equal that library's _pcre2_default_compile_context
        let cc = (c.pcre2_compile_context_create_8)(ptr::null_mut());
        let rr = (r.pcre2_compile_context_create_8)(ptr::null_mut());
        assert!(!cc.is_null() && !rr.is_null());
        let dc = c.data("_pcre2_default_compile_context_8") as Ptr;
        let dr = r.data("_pcre2_default_compile_context_8") as Ptr;
        cmp_mem(
            "C066",
            "create(NULL) == default [C]",
            &bytes_of(cc, CC_SIZE),
            &bytes_of(dc, CC_SIZE),
            &[],
        );
        cmp_mem(
            "C066",
            "create(NULL) == default [Rust]",
            &bytes_of(rr, CC_SIZE),
            &bytes_of(dr, CC_SIZE),
            &[],
        );
        // cross-library: everything but the three memctl pointers and `tables`
        cmp_mem(
            "C066",
            "create(NULL) C vs Rust",
            &bytes_of(cc, CC_SIZE),
            &bytes_of(rr, CC_SIZE),
            &[(0, 24), (CC_TABLES, 8)],
        );
        assert_eq!(
            rd_ptr(cc, CC_TABLES),
            c.data("_pcre2_default_tables_8"),
            "C066: C tables field"
        );
        assert_eq!(
            rd_ptr(rr, CC_TABLES),
            r.data("_pcre2_default_tables_8"),
            "C066: Rust tables field"
        );
        (c.pcre2_compile_context_free_8)(cc);
        (r.pcre2_compile_context_free_8)(rr);
        (c.pcre2_compile_context_free_8)(ptr::null_mut());
        (r.pcre2_compile_context_free_8)(ptr::null_mut());

        // create with a gcontext: memctl overridden, everything else defaulted
        let gc = (c.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_C as *const Stats as Ptr,
        );
        let gr = (r.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_R as *const Stats as Ptr,
        );
        let cc = (c.pcre2_compile_context_create_8)(gc);
        let rr = (r.pcre2_compile_context_create_8)(gr);
        assert!(!cc.is_null() && !rr.is_null());
        cmp_mem(
            "C066",
            "create(gcontext) C vs Rust",
            &bytes_of(cc, CC_SIZE),
            &bytes_of(rr, CC_SIZE),
            &[(16, 8), (CC_TABLES, 8)],
        );
        assert_eq!(rd_ptr(cc, 0), t_malloc as *const u8);
        assert_eq!(rd_ptr(rr, 0), t_malloc as *const u8);

        // mutate every setter, then copy and compare all fields
        let tables_c = (c.pcre2_maketables_8)(ptr::null_mut());
        let tables_r = (r.pcre2_maketables_8)(ptr::null_mut());
        assert!(!tables_c.is_null() && !tables_r.is_null());
        let setup = |api: &Api, ctx: Ptr, tables: *const u8| {
            assert_eq!((api.pcre2_set_character_tables_8)(ctx, tables), 0);
            assert_eq!((api.pcre2_set_bsr_8)(ctx, PCRE2_BSR_ANYCRLF), 0);
            assert_eq!((api.pcre2_set_newline_8)(ctx, PCRE2_NEWLINE_ANYCRLF), 0);
            assert_eq!((api.pcre2_set_max_pattern_length_8)(ctx, 12345), 0);
            assert_eq!((api.pcre2_set_max_pattern_compiled_length_8)(ctx, 999999), 0);
            assert_eq!((api.pcre2_set_max_varlookbehind_8)(ctx, 77), 0);
            assert_eq!((api.pcre2_set_parens_nest_limit_8)(ctx, 133), 0);
            assert_eq!(
                (api.pcre2_set_compile_extra_options_8)(ctx, PCRE2_EXTRA_MATCH_WORD),
                0
            );
            assert_eq!(
                (api.pcre2_set_compile_recursion_guard_8)(
                    ctx,
                    Some(dummy_guard),
                    0x1234 as Ptr
                ),
                0
            );
            assert_eq!((api.pcre2_set_optimize_8)(ctx, PCRE2_AUTO_POSSESS_OFF), 0);
        };
        setup(c, cc, tables_c);
        setup(r, rr, tables_r);
        cmp_mem(
            "C066",
            "all setters applied",
            &bytes_of(cc, CC_SIZE),
            &bytes_of(rr, CC_SIZE),
            &[(16, 8), (CC_TABLES, 8)],
        );
        let cc2 = (c.pcre2_compile_context_copy_8)(cc);
        let rr2 = (r.pcre2_compile_context_copy_8)(rr);
        assert!(!cc2.is_null() && !rr2.is_null());
        cmp_mem(
            "C066",
            "copy == original [C]",
            &bytes_of(cc, CC_SIZE),
            &bytes_of(cc2, CC_SIZE),
            &[],
        );
        cmp_mem(
            "C066",
            "copy == original [Rust]",
            &bytes_of(rr, CC_SIZE),
            &bytes_of(rr2, CC_SIZE),
            &[],
        );

        // compile with the original and with the copy: identical results
        for pat in [
            &b"abc"[..],
            &b"[[:alpha:]]+\\R"[..],
            &b"(?i)a\\d(x|y)"[..],
            &b"a$"[..],
        ] {
            let a1 = c.compile_probe(pat, pat.len(), 0, cc);
            let a2 = c.compile_probe(pat, pat.len(), 0, cc2);
            let b1 = r.compile_probe(pat, pat.len(), 0, rr);
            let b2 = r.compile_probe(pat, pat.len(), 0, rr2);
            assert_eq!(a1, a2, "C066: C original vs copy:{}", diff(&a1, &a2));
            assert_eq!(b1, b2, "C066: Rust original vs copy:{}", diff(&b1, &b2));
            assert_eq!(a1, b1, "C066: C vs Rust with mutated ctx:{}", diff(&a1, &b1));
        }
        (c.pcre2_compile_context_free_8)(cc2);
        (r.pcre2_compile_context_free_8)(rr2);
        (c.pcre2_compile_context_free_8)(cc);
        (r.pcre2_compile_context_free_8)(rr);
        (c.pcre2_maketables_free_8)(ptr::null_mut(), tables_c);
        (r.pcre2_maketables_free_8)(ptr::null_mut(), tables_r);
        (c.pcre2_general_context_free_8)(gc);
        (r.pcre2_general_context_free_8)(gr);
        let sc = ST_C.snap();
        let sr = ST_R.snap();
        println!("  allocator: C {:?} Rust {:?}", sc, sr);
        assert_eq!(sc, sr, "C066: allocation accounting differs");
        assert_eq!(sc.0, sc.1, "C066: leak");
    }
}

#[test]
fn c067_match_context() {
    println!("C067 pcre2_match_context_create/copy/free");
    let (c, r) = both();
    static ST_C: Stats = Stats::new();
    static ST_R: Stats = Stats::new();
    unsafe {
        let mc = (c.pcre2_match_context_create_8)(ptr::null_mut());
        let mr = (r.pcre2_match_context_create_8)(ptr::null_mut());
        assert!(!mc.is_null() && !mr.is_null());
        let dc = c.data("_pcre2_default_match_context_8") as Ptr;
        let dr = r.data("_pcre2_default_match_context_8") as Ptr;
        cmp_mem(
            "C067",
            "create(NULL) == default [C]",
            &bytes_of(mc, MC_SIZE),
            &bytes_of(dc, MC_SIZE),
            &[],
        );
        cmp_mem(
            "C067",
            "create(NULL) == default [Rust]",
            &bytes_of(mr, MC_SIZE),
            &bytes_of(dr, MC_SIZE),
            &[],
        );
        cmp_mem(
            "C067",
            "create(NULL) C vs Rust",
            &bytes_of(mc, MC_SIZE),
            &bytes_of(mr, MC_SIZE),
            &[(0, 24)],
        );
        assert_eq!(rd_usize(mc, MC_OFFSET_LIMIT), PCRE2_UNSET);
        assert_eq!(rd_usize(mr, MC_OFFSET_LIMIT), PCRE2_UNSET);
        (c.pcre2_match_context_free_8)(mc);
        (r.pcre2_match_context_free_8)(mr);
        (c.pcre2_match_context_free_8)(ptr::null_mut());
        (r.pcre2_match_context_free_8)(ptr::null_mut());

        let gc = (c.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_C as *const Stats as Ptr,
        );
        let gr = (r.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_R as *const Stats as Ptr,
        );
        let mc = (c.pcre2_match_context_create_8)(gc);
        let mr = (r.pcre2_match_context_create_8)(gr);
        assert!(!mc.is_null() && !mr.is_null());
        assert_eq!(rd_ptr(mc, 0), t_malloc as *const u8);
        assert_eq!(rd_ptr(mr, 0), t_malloc as *const u8);
        cmp_mem(
            "C067",
            "create(gcontext) C vs Rust",
            &bytes_of(mc, MC_SIZE),
            &bytes_of(mr, MC_SIZE),
            &[(16, 8)],
        );

        // all three callouts and all four limits
        let setup = |api: &Api, ctx: Ptr| {
            assert_eq!(
                (api.pcre2_set_callout_8)(ctx, Some(dummy_callout), 0x11 as Ptr),
                0
            );
            assert_eq!(
                (api.pcre2_set_substitute_callout_8)(ctx, Some(dummy_sub_callout), 0x22 as Ptr),
                0
            );
            assert_eq!(
                (api.pcre2_set_substitute_case_callout_8)(
                    ctx,
                    Some(dummy_case_callout),
                    0x33 as Ptr
                ),
                0
            );
            assert_eq!((api.pcre2_set_offset_limit_8)(ctx, 4242), 0);
            assert_eq!((api.pcre2_set_heap_limit_8)(ctx, 4243), 0);
            assert_eq!((api.pcre2_set_match_limit_8)(ctx, 4244), 0);
            assert_eq!((api.pcre2_set_depth_limit_8)(ctx, 4245), 0);
        };
        setup(c, mc);
        setup(r, mr);
        cmp_mem(
            "C067",
            "all setters applied",
            &bytes_of(mc, MC_SIZE),
            &bytes_of(mr, MC_SIZE),
            &[(16, 8)],
        );
        assert_eq!(rd_usize(mc, MC_OFFSET_LIMIT), 4242);
        assert_eq!(rd_u32(mc, MC_HEAP_LIMIT), 4243);
        assert_eq!(rd_u32(mc, MC_MATCH_LIMIT), 4244);
        assert_eq!(rd_u32(mc, MC_DEPTH_LIMIT), 4245);

        let mc2 = (c.pcre2_match_context_copy_8)(mc);
        let mr2 = (r.pcre2_match_context_copy_8)(mr);
        assert!(!mc2.is_null() && !mr2.is_null());
        cmp_mem(
            "C067",
            "copy == original [C]",
            &bytes_of(mc, MC_SIZE),
            &bytes_of(mc2, MC_SIZE),
            &[],
        );
        cmp_mem(
            "C067",
            "copy == original [Rust]",
            &bytes_of(mr, MC_SIZE),
            &bytes_of(mr2, MC_SIZE),
            &[],
        );

        // use both for a real match with limits that bite
        let pat = b"(a+)*b";
        let subj = b"aaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut ec = 0i32;
        let mut eo = 0usize;
        let code_c =
            (c.pcre2_compile_8)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, ptr::null_mut());
        let code_r =
            (r.pcre2_compile_8)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, ptr::null_mut());
        let mdc = (c.pcre2_match_data_create_from_pattern_8)(code_c, ptr::null_mut());
        let mdr = (r.pcre2_match_data_create_from_pattern_8)(code_r, ptr::null_mut());
        // An offset limit without PCRE2_USE_OFFSET_LIMIT is rejected before the
        // ovector is initialized, so only the return code is defined here.
        for (nm, ctx_c, ctx_r) in [("orig", mc, mr), ("copy", mc2, mr2)] {
            let rc_c = (c.pcre2_match_8)(code_c, subj.as_ptr(), subj.len(), 0, 0, mdc, ctx_c);
            let rc_r = (r.pcre2_match_8)(code_r, subj.as_ptr(), subj.len(), 0, 0, mdr, ctx_r);
            assert_eq!(rc_c, rc_r, "C067 {}: offset limit without USE_OFFSET_LIMIT", nm);
            assert_eq!(rc_c, PCRE2_ERROR_BADOFFSETLIMIT, "C067 {}", nm);
            // now with the option set the limit is honoured
            let mut e2 = 0i32;
            let mut o2 = 0usize;
            let cc2 = (c.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                PCRE2_USE_OFFSET_LIMIT,
                &mut e2,
                &mut o2,
                ptr::null_mut(),
            );
            let rr2c = (r.pcre2_compile_8)(
                pat.as_ptr(),
                pat.len(),
                PCRE2_USE_OFFSET_LIMIT,
                &mut e2,
                &mut o2,
                ptr::null_mut(),
            );
            for &ol in &[0usize, 1, 3, 10, 4242, PCRE2_UNSET] {
                assert_eq!((c.pcre2_set_offset_limit_8)(ctx_c, ol), 0);
                assert_eq!((r.pcre2_set_offset_limit_8)(ctx_r, ol), 0);
                let rc_c =
                    (c.pcre2_match_8)(cc2, subj.as_ptr(), subj.len(), 0, 0, mdc, ctx_c);
                let rc_r =
                    (r.pcre2_match_8)(rr2c, subj.as_ptr(), subj.len(), 0, 0, mdr, ctx_r);
                cmp_match_out(
                    "C067",
                    &format!("{}: offset limit {}", nm, ol),
                    &read_match(c, mdc, rc_c),
                    &read_match(r, mdr, rc_r),
                );
            }
            (c.pcre2_code_free_8)(cc2);
            (r.pcre2_code_free_8)(rr2c);
            // leave the limit unset for the loop below
            assert_eq!((c.pcre2_set_offset_limit_8)(ctx_c, PCRE2_UNSET), 0);
            assert_eq!((r.pcre2_set_offset_limit_8)(ctx_r, PCRE2_UNSET), 0);
        }
        for &lim in &[0u32, 1, 5, 20, 100, 1000, u32::MAX] {
            for (nm, ctx_c, ctx_r) in [("orig", mc, mr), ("copy", mc2, mr2)] {
                assert_eq!((c.pcre2_set_match_limit_8)(ctx_c, lim), 0);
                assert_eq!((r.pcre2_set_match_limit_8)(ctx_r, lim), 0);
                assert_eq!((c.pcre2_set_depth_limit_8)(ctx_c, lim), 0);
                assert_eq!((r.pcre2_set_depth_limit_8)(ctx_r, lim), 0);
                let rc_c = (c.pcre2_match_8)(
                    code_c,
                    subj.as_ptr(),
                    subj.len(),
                    0,
                    0,
                    mdc,
                    ctx_c,
                );
                let rc_r = (r.pcre2_match_8)(
                    code_r,
                    subj.as_ptr(),
                    subj.len(),
                    0,
                    0,
                    mdr,
                    ctx_r,
                );
                let a = read_match(c, mdc, rc_c);
                let b = read_match(r, mdr, rc_r);
                cmp_match_out("C067", &format!("{}: limit {}", nm, lim), &a, &b);
            }
        }
        (c.pcre2_match_data_free_8)(mdc);
        (r.pcre2_match_data_free_8)(mdr);
        (c.pcre2_code_free_8)(code_c);
        (r.pcre2_code_free_8)(code_r);
        (c.pcre2_match_context_free_8)(mc2);
        (r.pcre2_match_context_free_8)(mr2);
        (c.pcre2_match_context_free_8)(mc);
        (r.pcre2_match_context_free_8)(mr);
        (c.pcre2_general_context_free_8)(gc);
        (r.pcre2_general_context_free_8)(gr);
        let sc = ST_C.snap();
        let sr = ST_R.snap();
        println!("  allocator: C {:?} Rust {:?}", sc, sr);
        assert_eq!(sc, sr, "C067: allocation accounting differs");
        assert_eq!(sc.0, sc.1, "C067: leak");
    }
}

#[test]
fn c068_convert_context() {
    println!("C068 pcre2_convert_context_create/copy/free");
    let (c, r) = both();
    static ST_C: Stats = Stats::new();
    static ST_R: Stats = Stats::new();
    unsafe {
        let cc = (c.pcre2_convert_context_create_8)(ptr::null_mut());
        let rr = (r.pcre2_convert_context_create_8)(ptr::null_mut());
        assert!(!cc.is_null() && !rr.is_null());
        let dc = c.data("_pcre2_default_convert_context_8") as Ptr;
        let dr = r.data("_pcre2_default_convert_context_8") as Ptr;
        cmp_mem(
            "C068",
            "create(NULL) == default [C]",
            &bytes_of(cc, CVC_SIZE),
            &bytes_of(dc, CVC_SIZE),
            &[],
        );
        cmp_mem(
            "C068",
            "create(NULL) == default [Rust]",
            &bytes_of(rr, CVC_SIZE),
            &bytes_of(dr, CVC_SIZE),
            &[],
        );
        cmp_mem(
            "C068",
            "create(NULL) C vs Rust",
            &bytes_of(cc, CVC_SIZE),
            &bytes_of(rr, CVC_SIZE),
            &[(0, 24)],
        );
        // non-Windows defaults
        assert_eq!(rd_u32(cc, CVC_GLOB_SEPARATOR), b'/' as u32);
        assert_eq!(rd_u32(rr, CVC_GLOB_SEPARATOR), b'/' as u32);
        assert_eq!(rd_u32(cc, CVC_GLOB_ESCAPE), b'\\' as u32);
        assert_eq!(rd_u32(rr, CVC_GLOB_ESCAPE), b'\\' as u32);
        (c.pcre2_convert_context_free_8)(cc);
        (r.pcre2_convert_context_free_8)(rr);
        (c.pcre2_convert_context_free_8)(ptr::null_mut());
        (r.pcre2_convert_context_free_8)(ptr::null_mut());

        let gc = (c.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_C as *const Stats as Ptr,
        );
        let gr = (r.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_R as *const Stats as Ptr,
        );
        let cc = (c.pcre2_convert_context_create_8)(gc);
        let rr = (r.pcre2_convert_context_create_8)(gr);
        assert!(!cc.is_null() && !rr.is_null());
        assert_eq!(rd_ptr(cc, 0), t_malloc as *const u8);
        assert_eq!(rd_ptr(rr, 0), t_malloc as *const u8);
        assert_eq!((c.pcre2_set_glob_separator_8)(cc, b'.' as u32), 0);
        assert_eq!((r.pcre2_set_glob_separator_8)(rr, b'.' as u32), 0);
        assert_eq!((c.pcre2_set_glob_escape_8)(cc, b'`' as u32), 0);
        assert_eq!((r.pcre2_set_glob_escape_8)(rr, b'`' as u32), 0);
        cmp_mem(
            "C068",
            "setters applied",
            &bytes_of(cc, CVC_SIZE),
            &bytes_of(rr, CVC_SIZE),
            &[(16, 8)],
        );
        let cc2 = (c.pcre2_convert_context_copy_8)(cc);
        let rr2 = (r.pcre2_convert_context_copy_8)(rr);
        assert!(!cc2.is_null() && !rr2.is_null());
        cmp_mem(
            "C068",
            "copy == original [C]",
            &bytes_of(cc, CVC_SIZE),
            &bytes_of(cc2, CVC_SIZE),
            &[],
        );
        cmp_mem(
            "C068",
            "copy == original [Rust]",
            &bytes_of(rr, CVC_SIZE),
            &bytes_of(rr2, CVC_SIZE),
            &[],
        );
        // convert with original and copy
        for g in ["a/b*", "**/x", "[!ab]c", "a`*b", "x.y"] {
            for (ctx_c, ctx_r) in [(cc, rr), (cc2, rr2)] {
                cmp_convert("C068", g.as_bytes(), PCRE2_CONVERT_GLOB, ctx_c, ctx_r);
            }
        }
        (c.pcre2_convert_context_free_8)(cc2);
        (r.pcre2_convert_context_free_8)(rr2);
        (c.pcre2_convert_context_free_8)(cc);
        (r.pcre2_convert_context_free_8)(rr);
        (c.pcre2_general_context_free_8)(gc);
        (r.pcre2_general_context_free_8)(gr);
        let sc = ST_C.snap();
        let sr = ST_R.snap();
        println!("  allocator: C {:?} Rust {:?}", sc, sr);
        assert_eq!(sc, sr, "C068: allocation accounting differs");
        assert_eq!(sc.0, sc.1, "C068: leak");
    }
}

/// Run `pcre2_pattern_convert` in both libraries with the given contexts and
/// compare the return code and every byte of the converted pattern.
unsafe fn cmp_convert(id: &str, pat: &[u8], opts: u32, ctx_c: Ptr, ctx_r: Ptr) {
    let (c, r) = both();
    let mut bc: *mut u8 = ptr::null_mut();
    let mut lc: usize = 0;
    let mut br: *mut u8 = ptr::null_mut();
    let mut lr: usize = 0;
    let rc = (c.pcre2_pattern_convert_8)(pat.as_ptr(), pat.len(), opts, &mut bc, &mut lc, ctx_c);
    let rr = (r.pcre2_pattern_convert_8)(pat.as_ptr(), pat.len(), opts, &mut br, &mut lr, ctx_r);
    assert_eq!(
        rc,
        rr,
        "{}: convert rc for \"{}\" options {:#x}",
        id,
        show(pat),
        opts
    );
    if rc == 0 {
        assert_eq!(lc, lr, "{}: convert length for \"{}\"", id, show(pat));
        assert!(!bc.is_null() && !br.is_null());
        let a = std::slice::from_raw_parts(bc, lc + 1);
        let b = std::slice::from_raw_parts(br, lr + 1);
        assert_eq!(
            a,
            b,
            "{}: converted \"{}\" options {:#x}: C \"{}\" Rust \"{}\"",
            id,
            show(pat),
            opts,
            show(a),
            show(b)
        );
    }
    if !bc.is_null() {
        (c.pcre2_converted_pattern_free_8)(bc);
    }
    if !br.is_null() {
        (r.pcre2_converted_pattern_free_8)(br);
    }
}

// ============================================================ C069 maketables

#[test]
fn c069_maketables() {
    println!("C069 pcre2_maketables / pcre2_maketables_free / pcre2_set_character_tables");
    let (c, r) = both();
    static ST_C: Stats = Stats::new();
    static ST_R: Stats = Stats::new();
    unsafe {
        // NULL gcontext
        let tc = (c.pcre2_maketables_8)(ptr::null_mut());
        let tr = (r.pcre2_maketables_8)(ptr::null_mut());
        assert!(!tc.is_null() && !tr.is_null(), "C069: maketables(NULL)");
        let bc = std::slice::from_raw_parts(tc, TABLES_LENGTH).to_vec();
        let br = std::slice::from_raw_parts(tr, TABLES_LENGTH).to_vec();
        if bc != br {
            let i = bc.iter().zip(br.iter()).position(|(x, y)| x != y).unwrap();
            panic!(
                "C069: maketables byte {} differs: C={:#04x} Rust={:#04x}",
                i, bc[i], br[i]
            );
        }
        // ...and both must equal the static default tables in the C locale
        let dc = std::slice::from_raw_parts(c.data("_pcre2_default_tables_8"), TABLES_LENGTH);
        let dr = std::slice::from_raw_parts(r.data("_pcre2_default_tables_8"), TABLES_LENGTH);
        assert_eq!(&bc[..], dc, "C069: C maketables != C default_tables");
        assert_eq!(&br[..], dr, "C069: Rust maketables != Rust default_tables");

        // custom-allocator gcontext
        let gc = (c.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_C as *const Stats as Ptr,
        );
        let gr = (r.pcre2_general_context_create_8)(
            Some(t_malloc),
            Some(t_free),
            &ST_R as *const Stats as Ptr,
        );
        let before_c = ST_C.snap();
        let before_r = ST_R.snap();
        let tc2 = (c.pcre2_maketables_8)(gc);
        let tr2 = (r.pcre2_maketables_8)(gr);
        assert!(!tc2.is_null() && !tr2.is_null());
        assert_eq!(
            std::slice::from_raw_parts(tc2, TABLES_LENGTH),
            &bc[..],
            "C069: C maketables(gcontext) differs from maketables(NULL)"
        );
        assert_eq!(
            std::slice::from_raw_parts(tr2, TABLES_LENGTH),
            &br[..],
            "C069: Rust maketables(gcontext) differs from maketables(NULL)"
        );
        assert_eq!(
            ST_C.snap().0 - before_c.0,
            1,
            "C069: C maketables did not use the gcontext malloc exactly once"
        );
        assert_eq!(
            ST_R.snap().0 - before_r.0,
            1,
            "C069: Rust maketables did not use the gcontext malloc exactly once"
        );
        assert_eq!(
            ST_C.snap().2 - before_c.2,
            TABLES_LENGTH,
            "C069: C requested TABLES_LENGTH"
        );
        assert_eq!(
            ST_R.snap().2 - before_r.2,
            TABLES_LENGTH,
            "C069: Rust requested TABLES_LENGTH"
        );

        // install the produced tables and compile with them
        let cc = (c.pcre2_compile_context_create_8)(ptr::null_mut());
        let rr = (r.pcre2_compile_context_create_8)(ptr::null_mut());
        assert_eq!((c.pcre2_set_character_tables_8)(cc, tc), 0);
        assert_eq!((r.pcre2_set_character_tables_8)(rr, tr), 0);
        let pats: &[&[u8]] = &[
            b"[[:alpha:]]",
            b"\\w",
            b"(?i)a",
            b"[a-z]",
            b"\\d+",
            b"[[:punct:]]",
            b"[[:space:]]\\W",
            b"(?i)[k-m]",
            b"[[:xdigit:]]",
            b"[[:cntrl:]]",
            b"[[:graph:]][[:print:]]",
            b"(?i)\\xe9",
        ];
        for pat in pats {
            // default tables
            let a0 = c.compile_probe(pat, pat.len(), 0, ptr::null_mut());
            let b0 = r.compile_probe(pat, pat.len(), 0, ptr::null_mut());
            assert_eq!(a0, b0, "C069 default tables \"{}\":{}", show(pat), diff(&a0, &b0));
            // maketables tables
            let a1 = c.compile_probe(pat, pat.len(), 0, cc);
            let b1 = r.compile_probe(pat, pat.len(), 0, rr);
            assert_eq!(a1, b1, "C069 maketables \"{}\":{}", show(pat), diff(&a1, &b1));
            // and in the C locale the two must agree with each other
            assert_eq!(
                a0, a1,
                "C069: C compile differs between default and maketables tables for \"{}\"",
                show(pat)
            );
            // caseless matching through the installed tables
            for &co in &[0u32, PCRE2_CASELESS, PCRE2_CASELESS | PCRE2_UCP] {
                let a = c.compile_probe(pat, pat.len(), co, cc);
                let b = r.compile_probe(pat, pat.len(), co, rr);
                assert_eq!(a, b, "C069 caseless \"{}\" {:#x}:{}", show(pat), co, diff(&a, &b));
            }
        }
        // a hand-modified table set: swap the case-flip entries for 'a'/'A' so the
        // tables are demonstrably in use
        let mut custom = bc.clone();
        custom[FCC_OFFSET + b'a' as usize] = b'a';
        custom[FCC_OFFSET + b'A' as usize] = b'A';
        custom[LCC_OFFSET + b'A' as usize] = b'A';
        assert_eq!((c.pcre2_set_character_tables_8)(cc, custom.as_ptr()), 0);
        assert_eq!((r.pcre2_set_character_tables_8)(rr, custom.as_ptr()), 0);
        for pat in [&b"(?i)a"[..], &b"(?i)[a-c]"[..], &b"(?i)A"[..]] {
            let a = c.compile_probe(pat, pat.len(), 0, cc);
            let b = r.compile_probe(pat, pat.len(), 0, rr);
            assert_eq!(a, b, "C069 custom tables \"{}\":{}", show(pat), diff(&a, &b));
            let a0 = c.compile_probe(pat, pat.len(), 0, ptr::null_mut());
            assert_ne!(
                a.image, a0.image,
                "C069: custom tables had no effect on \"{}\"",
                show(pat)
            );
        }
        (c.pcre2_compile_context_free_8)(cc);
        (r.pcre2_compile_context_free_8)(rr);

        // free with the same library's free function
        (c.pcre2_maketables_free_8)(gc, tc2);
        (r.pcre2_maketables_free_8)(gr, tr2);
        (c.pcre2_maketables_free_8)(ptr::null_mut(), tc);
        (r.pcre2_maketables_free_8)(ptr::null_mut(), tr);
        (c.pcre2_general_context_free_8)(gc);
        (r.pcre2_general_context_free_8)(gr);
        let sc = ST_C.snap();
        let sr = ST_R.snap();
        println!("  allocator: C {:?} Rust {:?}", sc, sr);
        assert_eq!(sc, sr, "C069: allocation accounting differs");
        assert_eq!(sc.0, sc.1, "C069: leak");
    }
}

// ================================================== C070 - C074  the setters

/// Every value a validating setter might see.
fn setter_values() -> Vec<u32> {
    let mut v: Vec<u32> = (0u32..=300).collect();
    v.push(u32::MAX);
    v.push(u32::MAX - 1);
    v.push(0x8000_0000);
    v.push(0x10000);
    let mut rng = Rng::new(0x7070);
    for _ in 0..64 {
        v.push(rng.next_u64() as u32);
    }
    v
}

#[test]
fn c070_set_bsr() {
    println!("C070 pcre2_set_bsr");
    let (c, r) = both();
    let subjects: &[&[u8]] = &[
        b"\n",
        b"\r",
        b"\r\n",
        b"\x0b",
        b"\x0c",
        b"\x85",
        b"a\nb",
        b"\xc2\x85",
        b"\xe2\x80\xa8",
        b"\xe2\x80\xa9",
        b"",
    ];
    unsafe {
        for v in setter_values() {
            let cc = (c.pcre2_compile_context_create_8)(ptr::null_mut());
            let rr = (r.pcre2_compile_context_create_8)(ptr::null_mut());
            let a = (c.pcre2_set_bsr_8)(cc, v);
            let b = (r.pcre2_set_bsr_8)(rr, v);
            assert_eq!(a, b, "C070: set_bsr({}) rc", v);
            if v == PCRE2_BSR_UNICODE || v == PCRE2_BSR_ANYCRLF {
                assert_eq!(a, 0, "C070: set_bsr({}) should be accepted", v);
            } else {
                assert_eq!(a, PCRE2_ERROR_BADDATA, "C070: set_bsr({}) should be rejected", v);
            }
            // the stored field (accepted-but-ignored would show up here)
            assert_eq!(
                rd_u16(cc, CC_BSR),
                rd_u16(rr, CC_BSR),
                "C070: bsr_convention after set_bsr({})",
                v
            );
            (c.pcre2_compile_context_free_8)(cc);
            (r.pcre2_compile_context_free_8)(rr);
        }
        // behaviour under each value: compile and match \R (and friends)
        for v in 0u32..=6 {
            let set = move |api: &Api, ctx: Ptr| {
                let _ = (api.pcre2_set_bsr_8)(ctx, v);
            };
            for pat in [&b"\\R"[..], &b"\\R+"[..], &b"a\\Rb"[..], &b"\\R{2}"[..], &b"[\\R]"[..]] {
                for &o in &[0u32, PCRE2_UTF, PCRE2_MULTILINE, PCRE2_UTF | PCRE2_UCP] {
                    cmp_probe_ctx_match("C070", "bsr", pat, o, &set, subjects, 0);
                }
            }
        }
        // (*BSR_UNICODE) / (*BSR_ANYCRLF) in the pattern must interact identically
        for pat in [&b"(*BSR_UNICODE)\\R"[..], &b"(*BSR_ANYCRLF)\\R"[..]] {
            for v in 0u32..=3 {
                let set = move |api: &Api, ctx: Ptr| {
                    let _ = (api.pcre2_set_bsr_8)(ctx, v);
                };
                cmp_probe_ctx_match("C070", "bsr-verb", pat, 0, &set, subjects, 0);
            }
        }
    }
}

#[test]
fn c071_set_newline() {
    println!("C071 pcre2_set_newline");
    let (c, r) = both();
    let subjects: &[&[u8]] = &[
        b"a\n",
        b"a\r",
        b"a\r\n",
        b"a\x00b",
        b"a\x0b",
        b"a\x0c",
        b"a\x85",
        b"a\xc2\x85b",
        b"a\xe2\x80\xa8b",
        b"a\xe2\x80\xa9b",
        b"\na\n",
        b"a",
        b"",
        b"\r\r\n\n",
    ];
    unsafe {
        for v in setter_values() {
            let cc = (c.pcre2_compile_context_create_8)(ptr::null_mut());
            let rr = (r.pcre2_compile_context_create_8)(ptr::null_mut());
            let a = (c.pcre2_set_newline_8)(cc, v);
            let b = (r.pcre2_set_newline_8)(rr, v);
            assert_eq!(a, b, "C071: set_newline({}) rc", v);
            if (1..=6).contains(&v) {
                assert_eq!(a, 0, "C071: set_newline({}) should be accepted", v);
            } else {
                assert_eq!(
                    a, PCRE2_ERROR_BADDATA,
                    "C071: set_newline({}) should be rejected",
                    v
                );
            }
            assert_eq!(
                rd_u16(cc, CC_NEWLINE),
                rd_u16(rr, CC_NEWLINE),
                "C071: newline_convention after set_newline({})",
                v
            );
            (c.pcre2_compile_context_free_8)(cc);
            (r.pcre2_compile_context_free_8)(rr);
        }
        for v in 0u32..=7 {
            let set = move |api: &Api, ctx: Ptr| {
                let _ = (api.pcre2_set_newline_8)(ctx, v);
            };
            for pat in [
                &b"^a$"[..],
                &b"a$"[..],
                &b"a\\Z"[..],
                &b"a\\z"[..],
                &b"."[..],
                &b"\\R"[..],
                &b"a.b"[..],
                &b"^"[..],
                &b"$"[..],
                &b"a\\N"[..],
            ] {
                for &o in &[
                    0u32,
                    PCRE2_MULTILINE,
                    PCRE2_UTF,
                    PCRE2_MULTILINE | PCRE2_UTF,
                    PCRE2_DOTALL,
                    PCRE2_FIRSTLINE,
                    PCRE2_ALT_CIRCUMFLEX | PCRE2_MULTILINE,
                    PCRE2_DOLLAR_ENDONLY,
                ] {
                    cmp_probe_ctx_match("C071", "newline", pat, o, &set, subjects, 0);
                }
            }
            // partial matching hits the CRLF special cases
            for pat in [&b"a$"[..], &b"a\\Z"[..], &b"\\R"[..]] {
                cmp_probe_ctx_match(
                    "C071",
                    "newline-partial",
                    pat,
                    0,
                    &set,
                    subjects,
                    PCRE2_PARTIAL_HARD,
                );
            }
        }
        // (*CR) etc. in the pattern
        for pat in [
            &b"(*CR)^a$"[..],
            &b"(*LF)^a$"[..],
            &b"(*CRLF)^a$"[..],
            &b"(*ANY)^a$"[..],
            &b"(*ANYCRLF)^a$"[..],
            &b"(*NUL)^a$"[..],
        ] {
            for v in 0u32..=6 {
                let set = move |api: &Api, ctx: Ptr| {
                    let _ = (api.pcre2_set_newline_8)(ctx, v);
                };
                cmp_probe_ctx_match("C071", "newline-verb", pat, PCRE2_MULTILINE, &set, subjects, 0);
            }
        }
    }
}

#[test]
fn c072_set_optimize() {
    println!("C072 pcre2_set_optimize");
    let (c, r) = both();
    unsafe {
        // NULL context
        assert_eq!(
            (c.pcre2_set_optimize_8)(ptr::null_mut(), PCRE2_OPTIMIZATION_FULL),
            PCRE2_ERROR_NULL
        );
        assert_eq!(
            (r.pcre2_set_optimize_8)(ptr::null_mut(), PCRE2_OPTIMIZATION_FULL),
            PCRE2_ERROR_NULL
        );
        for v in setter_values() {
            let cc = (c.pcre2_compile_context_create_8)(ptr::null_mut());
            let rr = (r.pcre2_compile_context_create_8)(ptr::null_mut());
            let a = (c.pcre2_set_optimize_8)(cc, v);
            let b = (r.pcre2_set_optimize_8)(rr, v);
            assert_eq!(a, b, "C072: set_optimize({}) rc", v);
            let expect_ok = v == PCRE2_OPTIMIZATION_NONE
                || v == PCRE2_OPTIMIZATION_FULL
                || (PCRE2_AUTO_POSSESS..=PCRE2_START_OPTIMIZE_OFF).contains(&v);
            assert_eq!(
                a,
                if expect_ok { 0 } else { PCRE2_ERROR_BADOPTION },
                "C072: set_optimize({}) rc value",
                v
            );
            assert_eq!(
                rd_u32(cc, CC_OPTIMIZATION_FLAGS),
                rd_u32(rr, CC_OPTIMIZATION_FLAGS),
                "C072: optimization_flags after set_optimize({})",
                v
            );
            (c.pcre2_compile_context_free_8)(cc);
            (r.pcre2_compile_context_free_8)(rr);
        }
        // every single directive, and a few sequences, driven through a compile
        let pats: &[&[u8]] = &[
            b"a+b",
            b".*abc",
            b"abc",
            b"[ab]c",
            b"\\d+\\D",
            b"(?s).*abc",
            b"^abc",
            b"[Ww]ord",
            b"a*a",
            b"(*NO_AUTO_POSSESS)a+b",
            b"(*NO_DOTSTAR_ANCHOR).*abc",
            b"(*NO_START_OPT)abc",
        ];
        let subjects: &[&[u8]] = &[b"aaab", b"xxxabc", b"abc", b"word", b"WORD", b"", b"aaa\nabc"];
        let dirs = [
            PCRE2_OPTIMIZATION_NONE,
            PCRE2_OPTIMIZATION_FULL,
            PCRE2_AUTO_POSSESS,
            PCRE2_AUTO_POSSESS_OFF,
            PCRE2_DOTSTAR_ANCHOR,
            PCRE2_DOTSTAR_ANCHOR_OFF,
            PCRE2_START_OPTIMIZE,
            PCRE2_START_OPTIMIZE_OFF,
            63,
            70,
            71,
            2,
        ];
        for pat in pats {
            for &d in &dirs {
                let set = move |api: &Api, ctx: Ptr| {
                    let _ = (api.pcre2_set_optimize_8)(ctx, d);
                };
                for &o in &[0u32, PCRE2_DOTALL, PCRE2_MULTILINE, PCRE2_UTF] {
                    cmp_probe_ctx_match("C072", "directive", pat, o, &set, subjects, 0);
                }
            }
            // sequences, including the legacy option bits alongside
            for ds in OPT_SETS {
                let d: Vec<u32> = ds.to_vec();
                let set = move |api: &Api, ctx: Ptr| set_optimize_all(api, ctx, &d);
                for &o in &[
                    0u32,
                    PCRE2_NO_AUTO_POSSESS,
                    PCRE2_NO_DOTSTAR_ANCHOR,
                    PCRE2_NO_START_OPTIMIZE,
                    PCRE2_DOTALL,
                ] {
                    cmp_probe_ctx_match("C072", "sequence", pat, o, &set, subjects, 0);
                }
            }
        }
    }
}

#[test]
fn c073_set_glob_separator_and_escape() {
    println!("C073 pcre2_set_glob_separator / pcre2_set_glob_escape");
    let (c, r) = both();
    // pcre2_context.c:539-546 — the escape must be 0 or a member of globpunct
    let globpunct: &[u8] = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
    unsafe {
        for v in setter_values() {
            let cc = (c.pcre2_convert_context_create_8)(ptr::null_mut());
            let rr = (r.pcre2_convert_context_create_8)(ptr::null_mut());
            let a = (c.pcre2_set_glob_separator_8)(cc, v);
            let b = (r.pcre2_set_glob_separator_8)(rr, v);
            assert_eq!(a, b, "C073: set_glob_separator({}) rc", v);
            let sep_ok = v == b'/' as u32 || v == b'\\' as u32 || v == b'.' as u32;
            assert_eq!(
                a,
                if sep_ok { 0 } else { PCRE2_ERROR_BADDATA },
                "C073: set_glob_separator({}) rc value",
                v
            );
            let a = (c.pcre2_set_glob_escape_8)(cc, v);
            let b = (r.pcre2_set_glob_escape_8)(rr, v);
            assert_eq!(a, b, "C073: set_glob_escape({}) rc", v);
            let esc_ok = v == 0 || (v < 256 && globpunct.contains(&(v as u8)));
            assert_eq!(
                a,
                if esc_ok { 0 } else { PCRE2_ERROR_BADDATA },
                "C073: set_glob_escape({}) rc value",
                v
            );
            cmp_mem(
                "C073",
                &format!("convert context after value {}", v),
                &bytes_of(cc, CVC_SIZE),
                &bytes_of(rr, CVC_SIZE),
                &[(0, 24)],
            );
            (c.pcre2_convert_context_free_8)(cc);
            (r.pcre2_convert_context_free_8)(rr);
        }

        // every accepted combination, driven through the glob converter
        let globs: &[&[u8]] = &[
            b"",
            b"a",
            b"*",
            b"**",
            b"***",
            b"?",
            b"a?b",
            b"*a",
            b"a*",
            b"a**b",
            b"**/x",
            b"/**",
            b"a/**/b",
            b"a/**\\/b",
            b"x/**",
            b"x*",
            b"\\*",
            b"a\\",
            b"a`",
            b"[abc]",
            b"[!abc]",
            b"[^abc]",
            b"[]abc]",
            b"[a-z]",
            b"[z-a]",
            b"[9-0]",
            b"[a-[:digit:]]",
            b"[[:digit:]]",
            b"[[:punct:]]",
            b"[[:nosuch:]]",
            b"[a",
            b"[!",
            b"[",
            b"[-a]",
            b"[a-]",
            b"[/]",
            b"[.]",
            b"[\\\\]",
            b"[a\\]b]",
            b"a.b",
            b"a\\b",
            b".*",
            b"**.x",
        ];
        let seps = [b'/' as u32, b'\\' as u32, b'.' as u32];
        let escs = [0u32, b'\\' as u32, b'`' as u32, b'!' as u32, b'*' as u32, b'[' as u32];
        let opts = [
            PCRE2_CONVERT_GLOB,
            PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
            PCRE2_CONVERT_GLOB_NO_STARSTAR,
            PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR | PCRE2_CONVERT_GLOB_NO_STARSTAR,
            PCRE2_CONVERT_GLOB | PCRE2_CONVERT_UTF,
        ];
        for &sep in &seps {
            for &esc in &escs {
                let cc = (c.pcre2_convert_context_create_8)(ptr::null_mut());
                let rr = (r.pcre2_convert_context_create_8)(ptr::null_mut());
                assert_eq!((c.pcre2_set_glob_separator_8)(cc, sep), 0);
                assert_eq!((r.pcre2_set_glob_separator_8)(rr, sep), 0);
                assert_eq!((c.pcre2_set_glob_escape_8)(cc, esc), 0);
                assert_eq!((r.pcre2_set_glob_escape_8)(rr, esc), 0);
                for g in globs {
                    for &o in &opts {
                        cmp_convert("C073", g, o, cc, rr);
                    }
                }
                (c.pcre2_convert_context_free_8)(cc);
                (r.pcre2_convert_context_free_8)(rr);
            }
        }
        // random globs
        let mut rng = Rng::new(0x7373);
        let alpha: &[u8] = b"ab*?[]!/\\.`^-:digt";
        for _ in 0..200 {
            let n = 1 + rng.below(10) as usize;
            let g: Vec<u8> = (0..n).map(|_| *rng.pick(alpha)).collect();
            let sep = seps[rng.below(3) as usize];
            let esc = escs[rng.below(escs.len() as u32) as usize];
            let cc = (c.pcre2_convert_context_create_8)(ptr::null_mut());
            let rr = (r.pcre2_convert_context_create_8)(ptr::null_mut());
            assert_eq!((c.pcre2_set_glob_separator_8)(cc, sep), 0);
            assert_eq!((r.pcre2_set_glob_separator_8)(rr, sep), 0);
            assert_eq!((c.pcre2_set_glob_escape_8)(cc, esc), 0);
            assert_eq!((r.pcre2_set_glob_escape_8)(rr, esc), 0);
            let o = opts[rng.below(opts.len() as u32) as usize];
            cmp_convert("C073", &g, o, cc, rr);
            (c.pcre2_convert_context_free_8)(cc);
            (r.pcre2_convert_context_free_8)(rr);
        }
    }
}

#[test]
fn c074_recursion_setters() {
    println!("C074 pcre2_set_recursion_limit / pcre2_set_recursion_memory_management");
    let (c, r) = both();
    static ST_C: Stats = Stats::new();
    static ST_R: Stats = Stats::new();
    unsafe {
        // set_recursion_limit(n) == set_depth_limit(n)
        for &n in &[0u32, 1, 2, 17, 1000, 10_000_000, u32::MAX] {
            for api in [c, r] {
                let a = (api.pcre2_match_context_create_8)(ptr::null_mut());
                let b = (api.pcre2_match_context_create_8)(ptr::null_mut());
                assert_eq!((api.pcre2_set_recursion_limit_8)(a, n), 0);
                assert_eq!((api.pcre2_set_depth_limit_8)(b, n), 0);
                cmp_mem(
                    "C074",
                    &format!("[{}] recursion_limit({}) == depth_limit", api.tag, n),
                    &bytes_of(a, MC_SIZE),
                    &bytes_of(b, MC_SIZE),
                    &[],
                );
                assert_eq!(rd_u32(a, MC_DEPTH_LIMIT), n);
                (api.pcre2_match_context_free_8)(a);
                (api.pcre2_match_context_free_8)(b);
            }
            // and across the two libraries
            let a = (c.pcre2_match_context_create_8)(ptr::null_mut());
            let b = (r.pcre2_match_context_create_8)(ptr::null_mut());
            assert_eq!((c.pcre2_set_recursion_limit_8)(a, n), 0);
            assert_eq!((r.pcre2_set_recursion_limit_8)(b, n), 0);
            cmp_mem(
                "C074",
                &format!("recursion_limit({}) C vs Rust", n),
                &bytes_of(a, MC_SIZE),
                &bytes_of(b, MC_SIZE),
                &[(0, 24)],
            );
            (c.pcre2_match_context_free_8)(a);
            (r.pcre2_match_context_free_8)(b);
        }

        // set_recursion_memory_management is a no-op
        let combos: [(
            Option<unsafe extern "C" fn(usize, Ptr) -> Ptr>,
            Option<unsafe extern "C" fn(Ptr, Ptr)>,
            Ptr,
        ); 8] = [
            (None, None, ptr::null_mut()),
            (None, None, 0x99 as Ptr),
            (Some(t_malloc), None, ptr::null_mut()),
            (None, Some(t_free), ptr::null_mut()),
            (Some(t_malloc), Some(t_free), ptr::null_mut()),
            (Some(t_malloc), Some(t_free), &ST_C as *const Stats as Ptr),
            (Some(null_malloc), Some(noop_free), 0x1 as Ptr),
            (Some(t_malloc), Some(noop_free), 0x2 as Ptr),
        ];
        for (m, f, d) in combos {
            let a = (c.pcre2_match_context_create_8)(ptr::null_mut());
            let b = (r.pcre2_match_context_create_8)(ptr::null_mut());
            let before_a = bytes_of(a, MC_SIZE);
            let before_b = bytes_of(b, MC_SIZE);
            let rc_a = (c.pcre2_set_recursion_memory_management_8)(a, m, f, d);
            let rc_b = (r.pcre2_set_recursion_memory_management_8)(b, m, f, d);
            assert_eq!(rc_a, 0, "C074: C set_recursion_memory_management rc");
            assert_eq!(rc_b, 0, "C074: Rust set_recursion_memory_management rc");
            cmp_mem("C074", "context unchanged [C]", &before_a, &bytes_of(a, MC_SIZE), &[]);
            cmp_mem("C074", "context unchanged [Rust]", &before_b, &bytes_of(b, MC_SIZE), &[]);
            (c.pcre2_match_context_free_8)(a);
            (r.pcre2_match_context_free_8)(b);
        }
        // ...and with a NULL match context (all arguments are discarded)
        assert_eq!(
            (c.pcre2_set_recursion_memory_management_8)(
                ptr::null_mut(),
                Some(t_malloc),
                Some(t_free),
                ptr::null_mut()
            ),
            0
        );
        assert_eq!(
            (r.pcre2_set_recursion_memory_management_8)(
                ptr::null_mut(),
                Some(t_malloc),
                Some(t_free),
                ptr::null_mut()
            ),
            0
        );
        assert_eq!(ST_C.snap().0, 0, "C074: nothing may be allocated");
        assert_eq!(ST_R.snap().0, 0, "C074: nothing may be allocated");

        // behaviour: recursion_limit must throttle matching exactly like depth_limit
        let pat = b"(a+)*b";
        let subj = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut ec = 0i32;
        let mut eo = 0usize;
        let code_c =
            (c.pcre2_compile_8)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, ptr::null_mut());
        let code_r =
            (r.pcre2_compile_8)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, ptr::null_mut());
        let mdc = (c.pcre2_match_data_create_from_pattern_8)(code_c, ptr::null_mut());
        let mdr = (r.pcre2_match_data_create_from_pattern_8)(code_r, ptr::null_mut());
        for &n in &[0u32, 1, 2, 5, 10, 50, 500, 100_000] {
            let a = (c.pcre2_match_context_create_8)(ptr::null_mut());
            let b = (r.pcre2_match_context_create_8)(ptr::null_mut());
            assert_eq!((c.pcre2_set_recursion_limit_8)(a, n), 0);
            assert_eq!((r.pcre2_set_recursion_limit_8)(b, n), 0);
            let rc_c = (c.pcre2_match_8)(code_c, subj.as_ptr(), subj.len(), 0, 0, mdc, a);
            let rc_r = (r.pcre2_match_8)(code_r, subj.as_ptr(), subj.len(), 0, 0, mdr, b);
            cmp_match_out(
                "C074",
                &format!("match with recursion_limit {}", n),
                &read_match(c, mdc, rc_c),
                &read_match(r, mdr, rc_r),
            );
            (c.pcre2_match_context_free_8)(a);
            (r.pcre2_match_context_free_8)(b);
        }
        (c.pcre2_match_data_free_8)(mdc);
        (r.pcre2_match_data_free_8)(mdr);
        (c.pcre2_code_free_8)(code_c);
        (r.pcre2_code_free_8)(code_r);
    }
}

// ================================================== C075 - C076  pcre2_config

const NUMERIC_KEYS: &[u32] = &[
    PCRE2_CONFIG_BSR,
    PCRE2_CONFIG_JIT,
    PCRE2_CONFIG_LINKSIZE,
    PCRE2_CONFIG_MATCHLIMIT,
    PCRE2_CONFIG_NEWLINE,
    PCRE2_CONFIG_PARENSLIMIT,
    PCRE2_CONFIG_DEPTHLIMIT,
    PCRE2_CONFIG_STACKRECURSE,
    PCRE2_CONFIG_UNICODE,
    PCRE2_CONFIG_HEAPLIMIT,
    PCRE2_CONFIG_NEVER_BACKSLASH_C,
    PCRE2_CONFIG_COMPILED_WIDTHS,
    PCRE2_CONFIG_TABLES_LENGTH,
    PCRE2_CONFIG_EFFECTIVE_LINKSIZE,
];

#[test]
fn c075_config_with_buffer() {
    println!("C075 pcre2_config with a real `where` buffer");
    let (c, r) = both();
    unsafe {
        // every key 0..=20 plus out-of-range ones
        let mut keys: Vec<u32> = (0u32..=20).collect();
        keys.extend_from_slice(&[100, 1000, u32::MAX, u32::MAX - 1, 0x8000_0000]);
        for k in keys {
            let mut ba = [0xAAu8; 512];
            let mut bb = [0xAAu8; 512];
            let ra = (c.pcre2_config_8)(k, ba.as_mut_ptr() as Ptr);
            let rb = (r.pcre2_config_8)(k, bb.as_mut_ptr() as Ptr);
            assert_eq!(ra, rb, "C075: config({}) rc", k);
            if ba != bb {
                let i = ba.iter().zip(bb.iter()).position(|(x, y)| x != y).unwrap();
                panic!(
                    "C075: config({}) wrote different bytes at {}: C={:02x?} R={:02x?}",
                    i,
                    k,
                    &ba[..32],
                    &bb[..32]
                );
            }
            if NUMERIC_KEYS.contains(&k) {
                assert_eq!(ra, 0, "C075: numeric key {} should return 0", k);
                // exactly 4 bytes must have been written
                assert!(
                    ba[4..].iter().all(|&b| b == 0xAA),
                    "C075: key {} wrote past 4 bytes",
                    k
                );
                println!("  key {:2} = {}", k, u32::from_ne_bytes([ba[0], ba[1], ba[2], ba[3]]));
            } else if k == PCRE2_CONFIG_UNICODE_VERSION || k == PCRE2_CONFIG_VERSION {
                assert!(ra > 0, "C075: string key {} rc {}", k, ra);
                println!(
                    "  key {:2} = \"{}\" (rc {})",
                    k,
                    show(&ba[..(ra as usize - 1)]),
                    ra
                );
            } else {
                assert_eq!(
                    ra, PCRE2_ERROR_BADOPTION,
                    "C075: key {} should be rejected",
                    k
                );
                assert!(ba.iter().all(|&b| b == 0xAA), "C075: key {} wrote to buffer", k);
            }
        }
        // the documented values for this build (row C075)
        let get = |api: &Api, k: u32| -> (i32, u32) {
            let mut v: u32 = 0xDEAD_BEEF;
            let rc = (api.pcre2_config_8)(k, &mut v as *mut u32 as Ptr);
            (rc, v)
        };
        for &(k, name) in &[
            (PCRE2_CONFIG_BSR, "BSR"),
            (PCRE2_CONFIG_JIT, "JIT"),
            (PCRE2_CONFIG_LINKSIZE, "LINKSIZE"),
            (PCRE2_CONFIG_EFFECTIVE_LINKSIZE, "EFFECTIVE_LINKSIZE"),
            (PCRE2_CONFIG_MATCHLIMIT, "MATCHLIMIT"),
            (PCRE2_CONFIG_NEWLINE, "NEWLINE"),
            (PCRE2_CONFIG_PARENSLIMIT, "PARENSLIMIT"),
            (PCRE2_CONFIG_DEPTHLIMIT, "DEPTHLIMIT"),
            (PCRE2_CONFIG_STACKRECURSE, "STACKRECURSE"),
            (PCRE2_CONFIG_UNICODE, "UNICODE"),
            (PCRE2_CONFIG_HEAPLIMIT, "HEAPLIMIT"),
            (PCRE2_CONFIG_NEVER_BACKSLASH_C, "NEVER_BACKSLASH_C"),
            (PCRE2_CONFIG_COMPILED_WIDTHS, "COMPILED_WIDTHS"),
            (PCRE2_CONFIG_TABLES_LENGTH, "TABLES_LENGTH"),
        ] {
            let a = get(c, k);
            let b = get(r, k);
            assert_eq!(a, b, "C075: {} (key {})", name, k);
        }
        // TABLES_LENGTH must agree with the value the tests use
        let (_, tl) = get(c, PCRE2_CONFIG_TABLES_LENGTH);
        assert_eq!(tl as usize, TABLES_LENGTH, "C075: TABLES_LENGTH");
    }
}

#[test]
fn c076_config_null_where_and_strings() {
    println!("C076 pcre2_config with where == NULL, and the string keys");
    let (c, r) = both();
    unsafe {
        let mut keys: Vec<u32> = (0u32..=20).collect();
        keys.extend_from_slice(&[100, 1000, u32::MAX]);
        for k in keys {
            let ra = (c.pcre2_config_8)(k, ptr::null_mut());
            let rb = (r.pcre2_config_8)(k, ptr::null_mut());
            assert_eq!(ra, rb, "C076: config({}, NULL) rc", k);
            if NUMERIC_KEYS.contains(&k) {
                assert_eq!(ra, 4, "C076: numeric key {} length query", k);
            } else if k == PCRE2_CONFIG_UNICODE_VERSION || k == PCRE2_CONFIG_VERSION {
                assert!(ra > 1, "C076: string key {} length query = {}", k, ra);
            } else {
                assert_eq!(ra, PCRE2_ERROR_BADOPTION, "C076: key {} length query", k);
            }
        }
        // JITTARGET is unavailable in a no-JIT build, both with and without a buffer
        let mut buf = [0xAAu8; 256];
        assert_eq!(
            (c.pcre2_config_8)(PCRE2_CONFIG_JITTARGET, ptr::null_mut()),
            (r.pcre2_config_8)(PCRE2_CONFIG_JITTARGET, ptr::null_mut())
        );
        assert_eq!(
            (c.pcre2_config_8)(PCRE2_CONFIG_JITTARGET, buf.as_mut_ptr() as Ptr),
            (r.pcre2_config_8)(PCRE2_CONFIG_JITTARGET, buf.as_mut_ptr() as Ptr)
        );
        assert!(buf.iter().all(|&b| b == 0xAA), "C076: JITTARGET wrote to the buffer");

        // the two string keys: NULL query, then a buffer, comparing every byte
        for k in [PCRE2_CONFIG_VERSION, PCRE2_CONFIG_UNICODE_VERSION] {
            let na = (c.pcre2_config_8)(k, ptr::null_mut());
            let nb = (r.pcre2_config_8)(k, ptr::null_mut());
            assert_eq!(na, nb, "C076: key {} NULL query", k);
            let mut ba = [0xAAu8; 256];
            let mut bb = [0xAAu8; 256];
            let ra = (c.pcre2_config_8)(k, ba.as_mut_ptr() as Ptr);
            let rb = (r.pcre2_config_8)(k, bb.as_mut_ptr() as Ptr);
            assert_eq!(ra, rb, "C076: key {} with buffer", k);
            assert_eq!(ra, na, "C076: key {} buffer rc != NULL query", k);
            assert_eq!(&ba[..], &bb[..], "C076: key {} bytes differ", k);
            // exactly `rc` bytes written (rc-1 characters plus the terminator)
            let n = ra as usize;
            assert_eq!(ba[n - 1], 0, "C076: key {} not NUL terminated", k);
            assert!(
                ba[n..].iter().all(|&b| b == 0xAA),
                "C076: key {} wrote past {} bytes",
                k,
                n
            );
            println!("  key {} = \"{}\" ({} bytes)", k, show(&ba[..n - 1]), n);
        }
    }
}

// ============================================ C077 pcre2_get_error_message

#[test]
fn c077_get_error_message() {
    println!("C077 pcre2_get_error_message");
    let (c, r) = both();
    unsafe {
        let mut codes: Vec<i32> = vec![];
        codes.extend(-80..=0); // runtime errors -1..-76 plus 0 and out of range
        codes.extend(95..=225); // compile errors 100..220 plus below/above
        codes.extend_from_slice(&[1000, -1000, i32::MAX, i32::MIN + 1, 99, 100, 220, 221]);
        let mut checked = 0usize;
        let mut valid = 0usize;
        for &e in &codes {
            // a big buffer first
            let mut ba = [0xAAu8; 512];
            let mut bb = [0xAAu8; 512];
            let ra = (c.pcre2_get_error_message_8)(e, ba.as_mut_ptr(), ba.len());
            let rb = (r.pcre2_get_error_message_8)(e, bb.as_mut_ptr(), bb.len());
            assert_eq!(ra, rb, "C077: get_error_message({}) rc", e);
            assert_eq!(&ba[..], &bb[..], "C077: get_error_message({}) buffer", e);
            checked += 1;
            if ra >= 0 {
                valid += 1;
                let n = ra as usize;
                assert_eq!(ba[n], 0, "C077: code {} not terminated at {}", e, n);
                assert!(
                    ba[n + 1..].iter().all(|&b| b == 0xAA),
                    "C077: code {} wrote past the message",
                    e
                );
                // now every interesting buffer size
                for size in [
                    0usize,
                    1,
                    2,
                    n.saturating_sub(1),
                    n,
                    n + 1,
                    n + 2,
                    n + 50,
                    400,
                ] {
                    let mut xa = [0xAAu8; 512];
                    let mut xb = [0xAAu8; 512];
                    let sa = (c.pcre2_get_error_message_8)(e, xa.as_mut_ptr(), size);
                    let sb = (r.pcre2_get_error_message_8)(e, xb.as_mut_ptr(), size);
                    assert_eq!(sa, sb, "C077: code {} size {} rc", e, size);
                    assert_eq!(
                        &xa[..],
                        &xb[..],
                        "C077: code {} size {} buffer:\n C={:?}\n R={:?}",
                        e,
                        size,
                        show(&xa[..64]),
                        show(&xb[..64])
                    );
                    if size == 0 {
                        assert_eq!(sa, PCRE2_ERROR_NOMEMORY);
                        assert!(xa.iter().all(|&b| b == 0xAA), "C077: size 0 touched buffer");
                    } else if size <= n {
                        assert_eq!(
                            sa, PCRE2_ERROR_NOMEMORY,
                            "C077: code {} size {} should not fit ({} chars)",
                            e, size, n
                        );
                        assert_eq!(xa[size - 1], 0, "C077: truncated message not terminated");
                        assert!(
                            xa[size..].iter().all(|&b| b == 0xAA),
                            "C077: code {} size {} overran",
                            e,
                            size
                        );
                    } else {
                        assert_eq!(sa, n as i32, "C077: code {} size {} rc", e, size);
                    }
                }
            } else {
                assert_eq!(
                    ra, PCRE2_ERROR_BADDATA,
                    "C077: code {} should be PCRE2_ERROR_BADDATA",
                    e
                );
            }
        }
        println!("  {} codes checked, {} with a message", checked, valid);
        assert!(valid > 180, "C077: only {} valid codes found", valid);
        // random codes
        let mut rng = Rng::new(0x7777);
        for _ in 0..300 {
            let e = (rng.next_u64() % 4000) as i32 - 2000;
            let size = (rng.below(80) as usize) + 0;
            let mut ba = [0xAAu8; 128];
            let mut bb = [0xAAu8; 128];
            let ra = (c.pcre2_get_error_message_8)(e, ba.as_mut_ptr(), size);
            let rb = (r.pcre2_get_error_message_8)(e, bb.as_mut_ptr(), size);
            assert_eq!(ra, rb, "C077 random: code {} size {}", e, size);
            assert_eq!(&ba[..], &bb[..], "C077 random: code {} size {}", e, size);
        }
    }
}
