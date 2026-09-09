//! ERRORS.md Part 1 (public-API rejections) and Part 3 (generic FFI boundaries).
//!
//! Every row constructs the exact invalid input/condition, calls BOTH the C and
//! the Rust `.so`, and asserts the SAME error code / sentinel is returned.
mod common;
use common::corpus::PATTERNS;
use common::*;
use std::ffi::{c_int, c_void};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn zp(p: &str) -> Vec<u8> {
    let mut v = p.as_bytes().to_vec();
    v.push(0);
    v
}
fn pad(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.extend_from_slice(&[0u8; 16]);
    v
}

/// Compile a pattern in both libraries, asserting they agree.
fn compile2(p: &str, opts: u32, ctx: (*mut Ctx, *mut Ctx)) -> Option<(*mut Code, *mut Code)> {
    let (c, r) = pair();
    let v = zp(p);
    let mut ce: c_int = 0;
    let mut co: Sz = 0;
    let mut re: c_int = 0;
    let mut ro: Sz = 0;
    let a = unsafe { (c.compile)(v.as_ptr(), p.len(), opts, &mut ce, &mut co, ctx.0) };
    let b = unsafe { (r.compile)(v.as_ptr(), p.len(), opts, &mut re, &mut ro, ctx.1) };
    assert_eq!((ce, co, a.is_null()), (re, ro, b.is_null()), "compile {p:?} 0x{opts:08x}");
    if a.is_null() { None } else { Some((a, b)) }
}

/// Offsets inside `pcre2_real_code` (`pcre2_intmodedep.h`):
///   memctl 24 | tables 8 | executable_jit 8 | start_bitmap[32] = 72
///   blocksize (PCRE2_SIZE) 72 | code_start 80 | magic_number 88
///   compile_options 92 | overall_options 96 | extra_options 100 | flags 104
/// Corrupting `magic_number` / `flags` is how ERRORS rows 31/32 build a
/// "bad magic" / "bad mode" code block. `row31_32_offsets_are_right` proves the
/// offsets by checking that the expected error actually appears.
const MAGIC_OFF: usize = 88;
const FLAGS_OFF: usize = 104;

unsafe fn corrupt_magic(code: *mut Code) -> u32 {
    let p = unsafe { (code as *mut u8).add(MAGIC_OFF) as *mut u32 };
    let old = unsafe { *p };
    unsafe { *p = 0xDEADBEEF };
    old
}
unsafe fn restore_u32(code: *mut Code, off: usize, v: u32) {
    unsafe { *((code as *mut u8).add(off) as *mut u32) = v };
}
unsafe fn corrupt_mode(code: *mut Code) -> u32 {
    let p = unsafe { (code as *mut u8).add(FLAGS_OFF) as *mut u32 };
    let old = unsafe { *p };
    // clear the code-unit-width bit (PCRE2_CODE_UNIT_WIDTH/8 == 1)
    unsafe { *p = old & !1u32 };
    old
}

/// Sanity: the offsets above really do address `magic_number` / `flags`.
#[test]
fn row31_32_offsets_are_right() {
    let (c, r) = pair();
    for a in [c, r] {
        let (code, _) = {
            let v = zp("abc");
            let mut e: c_int = 0;
            let mut o: Sz = 0;
            let x = unsafe {
                (a.compile)(v.as_ptr(), 3, 0, &mut e, &mut o, std::ptr::null_mut())
            };
            (x, e)
        };
        assert!(!code.is_null());
        // A healthy code block answers pattern_info fine.
        let mut n: u32 = 0;
        assert_eq!(
            unsafe { (a.pattern_info)(code, 4, &mut n as *mut u32 as *mut c_void) },
            0
        );
        let old = unsafe { corrupt_magic(code) };
        assert_eq!(
            unsafe { (a.pattern_info)(code, 4, &mut n as *mut u32 as *mut c_void) },
            PCRE2_ERROR_BADMAGIC,
            "{}: MAGIC_OFF is wrong",
            a.name
        );
        unsafe { restore_u32(code, MAGIC_OFF, old) };
        let old = unsafe { corrupt_mode(code) };
        assert_eq!(
            unsafe { (a.pattern_info)(code, 4, &mut n as *mut u32 as *mut c_void) },
            PCRE2_ERROR_BADMODE,
            "{}: FLAGS_OFF is wrong",
            a.name
        );
        unsafe { restore_u32(code, FLAGS_OFF, old) };
        unsafe { (a.code_free)(code) };
    }
}

// ---------------------------------------------- rows 1-17: compile arguments --

#[test]
fn rows1_17_compile_arguments() {
    let (c, r) = pair();
    let p = zp("abc");
    // row 1: errorptr == NULL
    let mut co: Sz = 0xDEAD;
    let mut ro: Sz = 0xDEAD;
    let a = unsafe { (c.compile)(p.as_ptr(), 3, 0, std::ptr::null_mut(), &mut co, std::ptr::null_mut()) };
    let b = unsafe { (r.compile)(p.as_ptr(), 3, 0, std::ptr::null_mut(), &mut ro, std::ptr::null_mut()) };
    assert_eq!((a.is_null(), co), (b.is_null(), ro), "row1 errorptr NULL");
    assert!(a.is_null());
    // row 2: erroroffset == NULL
    let mut ce: c_int = 0xDEAD;
    let mut re: c_int = 0xDEAD;
    let a = unsafe { (c.compile)(p.as_ptr(), 3, 0, &mut ce, std::ptr::null_mut(), std::ptr::null_mut()) };
    let b = unsafe { (r.compile)(p.as_ptr(), 3, 0, &mut re, std::ptr::null_mut(), std::ptr::null_mut()) };
    assert_eq!((a.is_null(), ce), (b.is_null(), re), "row2 erroroffset NULL");
    assert!(a.is_null());
    // row 3: both NULL
    let a = unsafe {
        (c.compile)(p.as_ptr(), 3, 0, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
    };
    let b = unsafe {
        (r.compile)(p.as_ptr(), 3, 0, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
    };
    assert_eq!(a.is_null(), b.is_null(), "row3 both NULL");
    // rows 4/5: NULL pattern
    for len in [0usize, 1, 3, PCRE2_ZERO_TERMINATED, Sz::MAX - 1] {
        let mut ce: c_int = 0;
        let mut co: Sz = 0;
        let mut re: c_int = 0;
        let mut ro: Sz = 0;
        let a = unsafe { (c.compile)(std::ptr::null(), len, 0, &mut ce, &mut co, std::ptr::null_mut()) };
        let b = unsafe { (r.compile)(std::ptr::null(), len, 0, &mut re, &mut ro, std::ptr::null_mut()) };
        assert_eq!((ce, co, a.is_null()), (re, ro, b.is_null()), "row4/5 NULL pattern len={len}");
        if !a.is_null() {
            assert_eq!(
                unsafe { serialize_bytes(c, a) },
                unsafe { serialize_bytes(r, b) },
                "NULL pattern len={len} serialized"
            );
            unsafe { (c.code_free)(a) };
            unsafe { (r.code_free)(b) };
        }
    }
    // rows 6/7/8/9: option-bit rejections (B11/B12 sweeps)
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    for bit in 0..32u32 {
        let opt = 1u32 << bit;
        for base in [0u32, PCRE2_LITERAL] {
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let a = unsafe { (c.compile)(p.as_ptr(), 3, base | opt, &mut ce, &mut co, cc) };
            let b = unsafe { (r.compile)(p.as_ptr(), 3, base | opt, &mut re, &mut ro, rc) };
            assert_eq!(
                (ce, co, a.is_null()),
                (re, ro, b.is_null()),
                "row6/8 option bit 0x{opt:08x} base=0x{base:08x}"
            );
            if !a.is_null() {
                unsafe { (c.code_free)(a) };
                unsafe { (r.code_free)(b) };
            }
        }
        // extra options sweep
        unsafe { (c.set_compile_extra_options)(cc, opt) };
        unsafe { (r.set_compile_extra_options)(rc, opt) };
        for base in [0u32, PCRE2_LITERAL, PCRE2_UTF] {
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let a = unsafe { (c.compile)(p.as_ptr(), 3, base, &mut ce, &mut co, cc) };
            let b = unsafe { (r.compile)(p.as_ptr(), 3, base, &mut re, &mut ro, rc) };
            assert_eq!(
                (ce, co, a.is_null()),
                (re, ro, b.is_null()),
                "row7/9 extra bit 0x{opt:08x} base=0x{base:08x}"
            );
            if !a.is_null() {
                unsafe { (c.code_free)(a) };
                unsafe { (r.code_free)(b) };
            }
        }
        unsafe { (c.set_compile_extra_options)(cc, 0) };
        unsafe { (r.set_compile_extra_options)(rc, 0) };
    }
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
    // rows 15/16/17: code_copy / code_free with NULL
    assert!(unsafe { (c.code_copy)(std::ptr::null()) }.is_null());
    assert!(unsafe { (r.code_copy)(std::ptr::null()) }.is_null());
    assert!(unsafe { (c.code_copy_with_tables)(std::ptr::null()) }.is_null());
    assert!(unsafe { (r.code_copy_with_tables)(std::ptr::null()) }.is_null());
    unsafe { (c.code_free)(std::ptr::null_mut()) };
    unsafe { (r.code_free)(std::ptr::null_mut()) };
}

// ------------------------------------------ rows 18-26, B7-B9, B23: setters ---

#[test]
fn rows18_26_context_setter_rejections() {
    let (c, r) = pair();
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    let vc = unsafe { (c.convert_context_create)(std::ptr::null_mut()) };
    let vr = unsafe { (r.convert_context_create)(std::ptr::null_mut()) };
    // row 18 / B8: set_bsr
    for v in [0u32, 1, 2, 3, 4, 100, u32::MAX, u32::MAX - 1] {
        assert_eq!(
            unsafe { (c.set_bsr)(cc, v) },
            unsafe { (r.set_bsr)(rc, v) },
            "set_bsr({v})"
        );
    }
    // row 19 / B7: set_newline
    for v in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 100, u32::MAX] {
        assert_eq!(
            unsafe { (c.set_newline)(cc, v) },
            unsafe { (r.set_newline)(rc, v) },
            "set_newline({v})"
        );
    }
    // row 20: set_optimize(NULL)
    assert_eq!(
        unsafe { (c.set_optimize)(std::ptr::null_mut(), 0) },
        unsafe { (r.set_optimize)(std::ptr::null_mut(), 0) },
        "set_optimize(NULL)"
    );
    // rows 21/22 / B9: set_optimize sweep
    for v in (0u32..=80).chain([100, 1000, u32::MAX, u32::MAX - 1]) {
        assert_eq!(
            unsafe { (c.set_optimize)(cc, v) },
            unsafe { (r.set_optimize)(rc, v) },
            "set_optimize({v})"
        );
    }
    // rows 23-26 / B23: glob separator & escape full sweep
    for v in (0u32..=0x200).chain([0x1000, 0xFFFF, 0x10000, 0x10FFFF, 0x110000, u32::MAX]) {
        assert_eq!(
            unsafe { (c.set_glob_separator)(vc, v) },
            unsafe { (r.set_glob_separator)(vr, v) },
            "set_glob_separator({v})"
        );
        assert_eq!(
            unsafe { (c.set_glob_escape)(vc, v) },
            unsafe { (r.set_glob_escape)(vr, v) },
            "set_glob_escape({v})"
        );
    }
    // setters with no validation must still agree
    for v in [0u32, 1, u32::MAX] {
        assert_eq!(
            unsafe { (c.set_max_varlookbehind)(cc, v) },
            unsafe { (r.set_max_varlookbehind)(rc, v) }
        );
        assert_eq!(
            unsafe { (c.set_parens_nest_limit)(cc, v) },
            unsafe { (r.set_parens_nest_limit)(rc, v) }
        );
        assert_eq!(
            unsafe { (c.set_compile_extra_options)(cc, v) },
            unsafe { (r.set_compile_extra_options)(rc, v) }
        );
    }
    for v in [0usize, 1, Sz::MAX] {
        assert_eq!(
            unsafe { (c.set_max_pattern_length)(cc, v) },
            unsafe { (r.set_max_pattern_length)(rc, v) }
        );
        assert_eq!(
            unsafe { (c.set_max_pattern_compiled_length)(cc, v) },
            unsafe { (r.set_max_pattern_compiled_length)(rc, v) }
        );
    }
    assert_eq!(
        unsafe { (c.set_character_tables)(cc, std::ptr::null()) },
        unsafe { (r.set_character_tables)(rc, std::ptr::null()) }
    );
    // match-context setters
    let mc = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let mr = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    for v in [0u32, 1, u32::MAX] {
        assert_eq!(unsafe { (c.set_heap_limit)(mc, v) }, unsafe { (r.set_heap_limit)(mr, v) });
        assert_eq!(unsafe { (c.set_match_limit)(mc, v) }, unsafe { (r.set_match_limit)(mr, v) });
        assert_eq!(unsafe { (c.set_depth_limit)(mc, v) }, unsafe { (r.set_depth_limit)(mr, v) });
        assert_eq!(
            unsafe { (c.set_recursion_limit)(mc, v) },
            unsafe { (r.set_recursion_limit)(mr, v) }
        );
    }
    for v in [0usize, 1, Sz::MAX] {
        assert_eq!(
            unsafe { (c.set_offset_limit)(mc, v) },
            unsafe { (r.set_offset_limit)(mr, v) }
        );
    }
    assert_eq!(
        unsafe {
            (c.set_recursion_memory_management)(mc, None, None, std::ptr::null_mut())
        },
        unsafe {
            (r.set_recursion_memory_management)(mr, None, None, std::ptr::null_mut())
        }
    );
    assert_eq!(
        unsafe { (c.set_callout)(mc, None, std::ptr::null_mut()) },
        unsafe { (r.set_callout)(mr, None, std::ptr::null_mut()) }
    );
    assert_eq!(
        unsafe { (c.set_substitute_callout)(mc, None, std::ptr::null_mut()) },
        unsafe { (r.set_substitute_callout)(mr, None, std::ptr::null_mut()) }
    );
    assert_eq!(
        unsafe { (c.set_substitute_case_callout)(mc, None, std::ptr::null_mut()) },
        unsafe { (r.set_substitute_case_callout)(mr, None, std::ptr::null_mut()) }
    );
    unsafe { (c.match_context_free)(mc) };
    unsafe { (r.match_context_free)(mr) };
    unsafe { (c.convert_context_free)(vc) };
    unsafe { (r.convert_context_free)(vr) };
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
}

// ------------------------------------------- rows 30-41, B6: pattern_info -----

static ENUM_RET: Mutex<c_int> = Mutex::new(0);
static ENUM_COUNT: AtomicUsize = AtomicUsize::new(0);
extern "C" fn enum_cb(_b: *mut c_void, _u: *mut c_void) -> c_int {
    ENUM_COUNT.fetch_add(1, Ordering::Relaxed);
    *ENUM_RET.lock().unwrap()
}

#[test]
fn rows30_41_pattern_info_and_callout_enumerate() {
    let (c, r) = pair();
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    // rows 30/38: NULL code
    for what in [0u32, 4, 26, 27, 1000, u32::MAX] {
        let mut buf = [0u8; 64];
        assert_eq!(
            unsafe { (c.pattern_info)(std::ptr::null(), what, buf.as_mut_ptr() as *mut c_void) },
            unsafe { (r.pattern_info)(std::ptr::null(), what, buf.as_mut_ptr() as *mut c_void) },
            "pattern_info(NULL, {what})"
        );
    }
    assert_eq!(
        unsafe { (c.callout_enumerate)(std::ptr::null(), Some(enum_cb), std::ptr::null_mut()) },
        unsafe { (r.callout_enumerate)(std::ptr::null(), Some(enum_cb), std::ptr::null_mut()) },
        "callout_enumerate(NULL)"
    );
    for p in PATTERNS.iter() {
        let Some((cc, rr)) = compile2(p, 0, null_ctx) else { continue };
        // rows 33-36 / B6: every info code, in range and out
        for what in (0u32..=30).chain([100, 1000, u32::MAX, u32::MAX - 1]) {
            let mut cb = [0xAAu8; 64];
            let mut rb = [0xAAu8; 64];
            let ci = unsafe { (c.pattern_info)(cc, what, cb.as_mut_ptr() as *mut c_void) };
            let ri = unsafe { (r.pattern_info)(rr, what, rb.as_mut_ptr() as *mut c_void) };
            assert_eq!(ci, ri, "pattern_info({what}) rc for {p:?}");
            // row 37: NULL `where`
            assert_eq!(
                unsafe { (c.pattern_info)(cc, what, std::ptr::null_mut()) },
                unsafe { (r.pattern_info)(rr, what, std::ptr::null_mut()) },
                "pattern_info({what}) NULL where for {p:?}"
            );
        }
        // rows 31/32/39/40: corrupted magic / mode
        let cm = unsafe { corrupt_magic(cc) };
        let rm = unsafe { corrupt_magic(rr) };
        for what in [0u32, 4, 22, 27] {
            let mut buf = [0u8; 64];
            assert_eq!(
                unsafe { (c.pattern_info)(cc, what, buf.as_mut_ptr() as *mut c_void) },
                unsafe { (r.pattern_info)(rr, what, buf.as_mut_ptr() as *mut c_void) },
                "bad magic pattern_info({what}) {p:?}"
            );
        }
        assert_eq!(
            unsafe { (c.callout_enumerate)(cc, Some(enum_cb), std::ptr::null_mut()) },
            unsafe { (r.callout_enumerate)(rr, Some(enum_cb), std::ptr::null_mut()) },
            "bad magic callout_enumerate {p:?}"
        );
        // serialize with bad magic (rows 99/104)
        let codes = [cc as *const Code];
        let rcodes = [rr as *const Code];
        let mut bp: *mut u8 = std::ptr::null_mut();
        let mut bs: Sz = 0;
        let mut rp: *mut u8 = std::ptr::null_mut();
        let mut rs: Sz = 0;
        assert_eq!(
            unsafe { (c.serialize_encode)(codes.as_ptr(), 1, &mut bp, &mut bs, std::ptr::null_mut()) },
            unsafe { (r.serialize_encode)(rcodes.as_ptr(), 1, &mut rp, &mut rs, std::ptr::null_mut()) },
            "bad magic serialize_encode {p:?}"
        );
        unsafe { restore_u32(cc, MAGIC_OFF, cm) };
        unsafe { restore_u32(rr, MAGIC_OFF, rm) };
        let cf = unsafe { corrupt_mode(cc) };
        let rf = unsafe { corrupt_mode(rr) };
        for what in [0u32, 4, 22] {
            let mut buf = [0u8; 64];
            assert_eq!(
                unsafe { (c.pattern_info)(cc, what, buf.as_mut_ptr() as *mut c_void) },
                unsafe { (r.pattern_info)(rr, what, buf.as_mut_ptr() as *mut c_void) },
                "bad mode pattern_info({what}) {p:?}"
            );
        }
        assert_eq!(
            unsafe { (c.callout_enumerate)(cc, Some(enum_cb), std::ptr::null_mut()) },
            unsafe { (r.callout_enumerate)(rr, Some(enum_cb), std::ptr::null_mut()) },
            "bad mode callout_enumerate {p:?}"
        );
        // row 44-50: match with bad mode
        let cmd = unsafe { (c.match_data_create)(4, std::ptr::null_mut()) };
        let rmd = unsafe { (r.match_data_create)(4, std::ptr::null_mut()) };
        let s = pad(b"abc");
        assert_eq!(
            unsafe { (c.pcre2_match)(cc, s.as_ptr(), 3, 0, 0, cmd, std::ptr::null_mut()) },
            unsafe { (r.pcre2_match)(rr, s.as_ptr(), 3, 0, 0, rmd, std::ptr::null_mut()) },
            "bad mode match {p:?}"
        );
        let mut cws = [0 as c_int; 64];
        let mut rws = [0 as c_int; 64];
        assert_eq!(
            unsafe {
                (c.dfa_match)(cc, s.as_ptr(), 3, 0, 0, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 64)
            },
            unsafe {
                (r.dfa_match)(rr, s.as_ptr(), 3, 0, 0, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 64)
            },
            "bad mode dfa_match {p:?}"
        );
        unsafe { (c.match_data_free)(cmd) };
        unsafe { (r.match_data_free)(rmd) };
        unsafe { restore_u32(cc, FLAGS_OFF, cf) };
        unsafe { restore_u32(rr, FLAGS_OFF, rf) };
        // row 41: callback return values propagate
        for ret in [0 as c_int, 1, -1, 99, -99] {
            *ENUM_RET.lock().unwrap() = ret;
            assert_eq!(
                unsafe { (c.callout_enumerate)(cc, Some(enum_cb), std::ptr::null_mut()) },
                unsafe { (r.callout_enumerate)(rr, Some(enum_cb), std::ptr::null_mut()) },
                "callout_enumerate ret={ret} {p:?}"
            );
        }
        *ENUM_RET.lock().unwrap() = 0;
        // row 42: match_data_create_from_pattern(NULL)
        unsafe { (c.code_free)(cc) };
        unsafe { (r.code_free)(rr) };
    }
    assert!(
        unsafe { (c.match_data_create_from_pattern)(std::ptr::null(), std::ptr::null_mut()) }
            .is_null()
    );
    assert!(
        unsafe { (r.match_data_create_from_pattern)(std::ptr::null(), std::ptr::null_mut()) }
            .is_null()
    );
}

// ----------------------------------- rows 43-76, B13/B14/B18/B19/B20: match ---

#[test]
fn rows43_76_match_rejections() {
    let (c, r) = pair();
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    // row 43 / B19: oveccount 0 is promoted to 1
    for n in [0u32, 1, 2, 0xFFFF, u32::MAX, u32::MAX - 1] {
        let cmd = unsafe { (c.match_data_create)(n, std::ptr::null_mut()) };
        let rmd = unsafe { (r.match_data_create)(n, std::ptr::null_mut()) };
        assert_eq!(cmd.is_null(), rmd.is_null(), "match_data_create({n})");
        if !cmd.is_null() {
            assert_eq!(
                unsafe { (c.get_ovector_count)(cmd) },
                unsafe { (r.get_ovector_count)(rmd) },
                "ovector_count({n})"
            );
            unsafe { (c.match_data_free)(cmd) };
            unsafe { (r.match_data_free)(rmd) };
        }
    }
    let s = pad(b"abcdef");
    let Some((cc, rr)) = compile2("(a)(b)", 0, null_ctx) else { panic!() };
    // row 44: match_data == NULL
    assert_eq!(
        unsafe { (c.pcre2_match)(cc, s.as_ptr(), 6, 0, 0, std::ptr::null_mut(), std::ptr::null_mut()) },
        unsafe { (r.pcre2_match)(rr, s.as_ptr(), 6, 0, 0, std::ptr::null_mut(), std::ptr::null_mut()) },
        "row44 match_data NULL"
    );
    assert_eq!(
        unsafe {
            (c.dfa_match)(cc, s.as_ptr(), 6, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 64)
        },
        unsafe {
            (r.dfa_match)(rr, s.as_ptr(), 6, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 64)
        },
        "row62 dfa match_data NULL"
    );
    let cmd = unsafe { (c.match_data_create)(8, std::ptr::null_mut()) };
    let rmd = unsafe { (r.match_data_create)(8, std::ptr::null_mut()) };
    let mut cws = [0 as c_int; 2000];
    let mut rws = [0 as c_int; 2000];
    // rows 45/46/63: NULL code / NULL subject
    for (len, tag) in [(0usize, "len0"), (1, "len1"), (6, "len6"), (PCRE2_ZERO_TERMINATED, "zt")] {
        let a = unsafe { (c.pcre2_match)(std::ptr::null(), s.as_ptr(), len, 0, 0, cmd, std::ptr::null_mut()) };
        let b = unsafe { (r.pcre2_match)(std::ptr::null(), s.as_ptr(), len, 0, 0, rmd, std::ptr::null_mut()) };
        assert_eq!(a, b, "row45 NULL code {tag}");
        let a = unsafe { (c.pcre2_match)(cc, std::ptr::null(), len, 0, 0, cmd, std::ptr::null_mut()) };
        let b = unsafe { (r.pcre2_match)(rr, std::ptr::null(), len, 0, 0, rmd, std::ptr::null_mut()) };
        assert_eq!(a, b, "row46 NULL subject {tag}");
        let a = unsafe {
            (c.dfa_match)(std::ptr::null(), s.as_ptr(), len, 0, 0, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 64)
        };
        let b = unsafe {
            (r.dfa_match)(std::ptr::null(), s.as_ptr(), len, 0, 0, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 64)
        };
        assert_eq!(a, b, "row63 dfa NULL code {tag}");
        let a = unsafe {
            (c.dfa_match)(cc, std::ptr::null(), len, 0, 0, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 64)
        };
        let b = unsafe {
            (r.dfa_match)(rr, std::ptr::null(), len, 0, 0, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 64)
        };
        assert_eq!(a, b, "row63 dfa NULL subject {tag}");
    }
    // rows 47/64 + B13/B14: every single option bit
    for bit in 0..32u32 {
        let opt = 1u32 << bit;
        let a = unsafe { (c.pcre2_match)(cc, s.as_ptr(), 6, 0, opt, cmd, std::ptr::null_mut()) };
        let b = unsafe { (r.pcre2_match)(rr, s.as_ptr(), 6, 0, opt, rmd, std::ptr::null_mut()) };
        assert_eq!(
            unsafe { md_dump(c, cmd, a) },
            unsafe { md_dump(r, rmd, b) },
            "row47 match option bit 0x{opt:08x}"
        );
        let a = unsafe {
            (c.dfa_match)(cc, s.as_ptr(), 6, 0, opt, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 2000)
        };
        let b = unsafe {
            (r.dfa_match)(rr, s.as_ptr(), 6, 0, opt, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 2000)
        };
        assert_eq!(
            unsafe { md_dump(c, cmd, a) },
            unsafe { md_dump(r, rmd, b) },
            "row64 dfa option bit 0x{opt:08x}"
        );
    }
    // rows 48/66 + B18: start offsets past the end
    for so in [0usize, 6, 7, 8, 1000, Sz::MAX - 1, Sz::MAX] {
        let a = unsafe { (c.pcre2_match)(cc, s.as_ptr(), 6, so, 0, cmd, std::ptr::null_mut()) };
        let b = unsafe { (r.pcre2_match)(rr, s.as_ptr(), 6, so, 0, rmd, std::ptr::null_mut()) };
        assert_eq!(a, b, "row48 start_offset={so}");
        let a = unsafe {
            (c.dfa_match)(cc, s.as_ptr(), 6, so, 0, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 2000)
        };
        let b = unsafe {
            (r.dfa_match)(rr, s.as_ptr(), 6, so, 0, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 2000)
        };
        assert_eq!(a, b, "row66 dfa start_offset={so}");
    }
    // row 65 / B20: wscount below the minimum
    for ws in [0usize, 1, 19, 20, 21, 100] {
        let a = unsafe {
            (c.dfa_match)(cc, s.as_ptr(), 6, 0, 0, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), ws)
        };
        let b = unsafe {
            (r.dfa_match)(rr, s.as_ptr(), 6, 0, 0, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), ws)
        };
        assert_eq!(a, b, "row65 wscount={ws}");
    }
    // row 67: DFA on a MATCH_INVALID_UTF pattern
    if let Some((cu, ru)) = compile2("a", PCRE2_MATCH_INVALID_UTF, null_ctx) {
        let a = unsafe {
            (c.dfa_match)(cu, s.as_ptr(), 6, 0, 0, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 2000)
        };
        let b = unsafe {
            (r.dfa_match)(ru, s.as_ptr(), 6, 0, 0, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 2000)
        };
        assert_eq!(a, b, "row67 dfa MATCH_INVALID_UTF");
        unsafe { (c.code_free)(cu) };
        unsafe { (r.code_free)(ru) };
    }
    // row 69: DFA_RESTART with a garbage workspace
    for fill in [0 as c_int, 1, -1, 0x7FFFFFFF, 12345] {
        cws.fill(fill);
        rws.fill(fill);
        let a = unsafe {
            (c.dfa_match)(cc, s.as_ptr(), 6, 0, PCRE2_DFA_RESTART, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 2000)
        };
        let b = unsafe {
            (r.dfa_match)(rr, s.as_ptr(), 6, 0, PCRE2_DFA_RESTART, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 2000)
        };
        assert_eq!(a, b, "row69 DFA_RESTART garbage workspace fill={fill}");
    }
    cws.fill(0);
    rws.fill(0);
    // rows 71/72/73: DFA-unsupported items
    for p in [
        "\\Ca", "(*UTF)\\C", "(?(R1)a|b)", "(?(R)a|b)", "(?(1)a|b)((x))", "(a)(?(1)b|c)",
        "(?R)?a", "(a)(?1)", "\\X", "(?=a)\\K", "(?C1)a", "(*ACCEPT)",
    ] {
        for copts in [0u32, PCRE2_UTF] {
            let Some((cu, ru)) = compile2(p, copts, null_ctx) else { continue };
            let a = unsafe {
                (c.dfa_match)(cu, s.as_ptr(), 6, 0, 0, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 2000)
            };
            let b = unsafe {
                (r.dfa_match)(ru, s.as_ptr(), 6, 0, 0, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 2000)
            };
            assert_eq!(a, b, "row71-74 dfa {p:?} copts=0x{copts:x}");
            unsafe { (c.code_free)(cu) };
            unsafe { (r.code_free)(ru) };
        }
    }
    // rows 53/54/76: invalid UTF subjects (checked path)
    let bad: &[&[u8]] = &[b"\xff", b"\x80", b"\xc2", b"a\xc2b", b"\xed\xa0\x80", b"\xf4\x90\x80\x80"];
    if let Some((cu, ru)) = compile2("a", PCRE2_UTF, null_ctx) {
        for sb in bad {
            let sp = pad(sb);
            for so in 0..=sb.len() {
                let a = unsafe { (c.pcre2_match)(cu, sp.as_ptr(), sb.len(), so, 0, cmd, std::ptr::null_mut()) };
                let b = unsafe { (r.pcre2_match)(ru, sp.as_ptr(), sb.len(), so, 0, rmd, std::ptr::null_mut()) };
                assert_eq!(a, b, "row53/54 invalid utf {sb:02x?} so={so}");
                assert_eq!(
                    unsafe { (c.get_startchar)(cmd) },
                    unsafe { (r.get_startchar)(rmd) },
                    "row53 startchar {sb:02x?} so={so}"
                );
                let a = unsafe {
                    (c.dfa_match)(cu, sp.as_ptr(), sb.len(), so, 0, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 2000)
                };
                let b = unsafe {
                    (r.dfa_match)(ru, sp.as_ptr(), sb.len(), so, 0, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 2000)
                };
                assert_eq!(a, b, "row76 dfa invalid utf {sb:02x?} so={so}");
            }
        }
        unsafe { (c.code_free)(cu) };
        unsafe { (r.code_free)(ru) };
    }
    // rows 55-59: limits and \K / recursion errors
    let mc = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let mr = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    for p in [
        "(a+)+b", "(?R)?a", "(a)(?1)*b", "(?<=(?=x)\\Ka)b", "(?=a\\K)b",
        "(?:a{0,100}){0,100}b", "\\b(\\w+)\\s+\\1",
    ] {
        for xopt in [0u32, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK] {
            let cx = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
            let rx = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
            unsafe { (c.set_compile_extra_options)(cx, xopt) };
            unsafe { (r.set_compile_extra_options)(rx, xopt) };
            if let Some((cu, ru)) = compile2(p, 0, (cx, rx)) {
                for lim in [0u32, 1, 2, 10] {
                    for which in 0..3 {
                        unsafe {
                            (c.set_match_limit)(mc, u32::MAX);
                            (r.set_match_limit)(mr, u32::MAX);
                            (c.set_depth_limit)(mc, u32::MAX);
                            (r.set_depth_limit)(mr, u32::MAX);
                            (c.set_heap_limit)(mc, u32::MAX);
                            (r.set_heap_limit)(mr, u32::MAX);
                        }
                        match which {
                            0 => unsafe {
                                (c.set_match_limit)(mc, lim);
                                (r.set_match_limit)(mr, lim);
                            },
                            1 => unsafe {
                                (c.set_depth_limit)(mc, lim);
                                (r.set_depth_limit)(mr, lim);
                            },
                            _ => unsafe {
                                (c.set_heap_limit)(mc, lim);
                                (r.set_heap_limit)(mr, lim);
                            },
                        }
                        let a = unsafe { (c.pcre2_match)(cu, s.as_ptr(), 6, 0, 0, cmd, mc) };
                        let b = unsafe { (r.pcre2_match)(ru, s.as_ptr(), 6, 0, 0, rmd, mr) };
                        assert_eq!(a, b, "rows55-57 {p:?} which={which} lim={lim}");
                        let a = unsafe {
                            (c.dfa_match)(cu, s.as_ptr(), 6, 0, 0, cmd, mc, cws.as_mut_ptr(), 2000)
                        };
                        let b = unsafe {
                            (r.dfa_match)(ru, s.as_ptr(), 6, 0, 0, rmd, mr, rws.as_mut_ptr(), 2000)
                        };
                        assert_eq!(a, b, "row75 dfa {p:?} which={which} lim={lim}");
                    }
                }
                unsafe { (c.code_free)(cu) };
                unsafe { (r.code_free)(ru) };
            }
            unsafe { (c.compile_context_free)(cx) };
            unsafe { (r.compile_context_free)(rx) };
        }
    }
    // rows 52/70: offset-limit conflict
    unsafe {
        (c.set_match_limit)(mc, u32::MAX);
        (r.set_match_limit)(mr, u32::MAX);
        (c.set_depth_limit)(mc, u32::MAX);
        (r.set_depth_limit)(mr, u32::MAX);
        (c.set_heap_limit)(mc, u32::MAX);
        (r.set_heap_limit)(mr, u32::MAX);
    }
    for copts in [0u32, PCRE2_USE_OFFSET_LIMIT] {
        let Some((cu, ru)) = compile2("b", copts, null_ctx) else { continue };
        for lim in [0usize, 1, 3, 6, 7, PCRE2_UNSET] {
            unsafe { (c.set_offset_limit)(mc, lim) };
            unsafe { (r.set_offset_limit)(mr, lim) };
            for mopts in [0u32, PCRE2_USE_OFFSET_LIMIT] {
                let a = unsafe { (c.pcre2_match)(cu, s.as_ptr(), 6, 0, mopts, cmd, mc) };
                let b = unsafe { (r.pcre2_match)(ru, s.as_ptr(), 6, 0, mopts, rmd, mr) };
                assert_eq!(a, b, "row52 offset limit copts=0x{copts:x} lim={lim} mopts=0x{mopts:x}");
                let a = unsafe {
                    (c.dfa_match)(cu, s.as_ptr(), 6, 0, mopts, cmd, mc, cws.as_mut_ptr(), 2000)
                };
                let b = unsafe {
                    (r.dfa_match)(ru, s.as_ptr(), 6, 0, mopts, rmd, mr, rws.as_mut_ptr(), 2000)
                };
                assert_eq!(a, b, "row70 dfa offset limit lim={lim}");
            }
        }
        unsafe { (c.code_free)(cu) };
        unsafe { (r.code_free)(ru) };
    }
    // rows 77/78: next_match / get_mark on an unmatched match_data
    let mut cs: Sz = 0;
    let mut ce2: u32 = 0;
    let mut rs2: Sz = 0;
    let mut re2: u32 = 0;
    let _ = unsafe { (c.pcre2_match)(cc, s.as_ptr(), 6, 0, PCRE2_ANCHORED, cmd, std::ptr::null_mut()) };
    let _ = unsafe { (r.pcre2_match)(rr, s.as_ptr(), 6, 0, PCRE2_ANCHORED, rmd, std::ptr::null_mut()) };
    assert_eq!(
        unsafe { (c.next_match)(cmd, &mut cs, &mut ce2) },
        unsafe { (r.next_match)(rmd, &mut rs2, &mut re2) },
        "row77 next_match after failure"
    );
    assert_eq!(
        unsafe { (c.get_mark)(cmd) }.is_null(),
        unsafe { (r.get_mark)(rmd) }.is_null(),
        "row78 get_mark"
    );
    unsafe { (c.match_context_free)(mc) };
    unsafe { (r.match_context_free)(mr) };
    unsafe { (c.match_data_free)(cmd) };
    unsafe { (r.match_data_free)(rmd) };
    unsafe { (c.code_free)(cc) };
    unsafe { (r.code_free)(rr) };
}

// -------------------------------- rows 79-93, B21: substring rejections -------

#[test]
fn rows79_93_substring_rejections() {
    let (c, r) = pair();
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    let s = pad(b"abcdef");
    let mut cws = [0 as c_int; 2000];
    let mut rws = [0 as c_int; 2000];
    for p in ["(a)(b)", "(?<n>a)(b)?", "(?J)(?<n>a)|(?<n>b)", "a", "(a)?(b)?(c)?"] {
        let Some((cc, rr)) = compile2(p, 0, null_ctx) else { continue };
        for ovec in [1u32, 2, 3, 8] {
            let cmd = unsafe { (c.match_data_create)(ovec, std::ptr::null_mut()) };
            let rmd = unsafe { (r.match_data_create)(ovec, std::ptr::null_mut()) };
            for (mopts, tag, dfa) in [
                (0u32, "match", false),
                (PCRE2_PARTIAL_HARD, "partial", false),
                (PCRE2_ANCHORED, "anchored", false),
                (0u32, "dfa", true),
            ] {
                let (a, b) = if dfa {
                    (
                        unsafe {
                            (c.dfa_match)(cc, s.as_ptr(), 6, 0, mopts, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 2000)
                        },
                        unsafe {
                            (r.dfa_match)(rr, s.as_ptr(), 6, 0, mopts, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 2000)
                        },
                    )
                } else {
                    (
                        unsafe { (c.pcre2_match)(cc, s.as_ptr(), 6, 0, mopts, cmd, std::ptr::null_mut()) },
                        unsafe { (r.pcre2_match)(rr, s.as_ptr(), 6, 0, mopts, rmd, std::ptr::null_mut()) },
                    )
                };
                assert_eq!(a, b, "{tag} rc {p:?} ovec={ovec}");
                // rows 79-84 / B21
                for num in [0u32, 1, 2, 3, 4, 100, u32::MAX] {
                    let mut cl: Sz = 0;
                    let mut rl: Sz = 0;
                    assert_eq!(
                        unsafe { (c.substring_length_bynumber)(cmd, num, &mut cl) },
                        unsafe { (r.substring_length_bynumber)(rmd, num, &mut rl) },
                        "row79-84 length_bynumber({num}) {tag} {p:?} ovec={ovec}"
                    );
                    // row 85: buffer too small
                    for bs in [0usize, 1, 2] {
                        let mut cb = vec![0xAAu8; bs + 8];
                        let mut rb = vec![0xAAu8; bs + 8];
                        let mut csz: Sz = bs;
                        let mut rsz: Sz = bs;
                        assert_eq!(
                            unsafe { (c.substring_copy_bynumber)(cmd, num, cb.as_mut_ptr(), &mut csz) },
                            unsafe { (r.substring_copy_bynumber)(rmd, num, rb.as_mut_ptr(), &mut rsz) },
                            "row85 copy_bynumber({num},{bs}) {tag} {p:?}"
                        );
                        assert_eq!(csz, rsz, "row85 size");
                        assert_eq!(cb, rb, "row85 buffer");
                    }
                    let mut cbp: *mut u8 = std::ptr::null_mut();
                    let mut rbp: *mut u8 = std::ptr::null_mut();
                    let mut csz: Sz = 0;
                    let mut rsz: Sz = 0;
                    let ci = unsafe { (c.substring_get_bynumber)(cmd, num, &mut cbp, &mut csz) };
                    let ri = unsafe { (r.substring_get_bynumber)(rmd, num, &mut rbp, &mut rsz) };
                    assert_eq!(ci, ri, "row87 get_bynumber({num}) {tag} {p:?}");
                    if ci == 0 {
                        unsafe { (c.substring_free)(cbp) };
                        unsafe { (r.substring_free)(rbp) };
                    }
                }
                // rows 86/88/89/90/91/92: names
                for name in ["n", "zzz", "", "N", "a"] {
                    let nz = zp(name);
                    let mut cl: Sz = 0;
                    let mut rl: Sz = 0;
                    assert_eq!(
                        unsafe { (c.substring_length_byname)(cmd, nz.as_ptr(), &mut cl) },
                        unsafe { (r.substring_length_byname)(rmd, nz.as_ptr(), &mut rl) },
                        "row89 length_byname({name:?}) {tag} {p:?}"
                    );
                    let mut cb = [0xAAu8; 16];
                    let mut rb = [0xAAu8; 16];
                    let mut csz: Sz = 4;
                    let mut rsz: Sz = 4;
                    assert_eq!(
                        unsafe { (c.substring_copy_byname)(cmd, nz.as_ptr(), cb.as_mut_ptr(), &mut csz) },
                        unsafe { (r.substring_copy_byname)(rmd, nz.as_ptr(), rb.as_mut_ptr(), &mut rsz) },
                        "row86 copy_byname({name:?}) {tag} {p:?}"
                    );
                    assert_eq!((csz, cb), (rsz, rb), "row86 copy_byname out");
                    let mut cbp: *mut u8 = std::ptr::null_mut();
                    let mut rbp: *mut u8 = std::ptr::null_mut();
                    let mut csz: Sz = 0;
                    let mut rsz: Sz = 0;
                    let ci = unsafe { (c.substring_get_byname)(cmd, nz.as_ptr(), &mut cbp, &mut csz) };
                    let ri = unsafe { (r.substring_get_byname)(rmd, nz.as_ptr(), &mut rbp, &mut rsz) };
                    assert_eq!(ci, ri, "row88 get_byname({name:?}) {tag} {p:?}");
                    if ci == 0 {
                        unsafe { (c.substring_free)(cbp) };
                        unsafe { (r.substring_free)(rbp) };
                    }
                    assert_eq!(
                        unsafe { (c.substring_number_from_name)(cc, nz.as_ptr()) },
                        unsafe { (r.substring_number_from_name)(rr, nz.as_ptr()) },
                        "row90/91 number_from_name({name:?}) {p:?}"
                    );
                    let mut cf: *const u8 = std::ptr::null();
                    let mut clst: *const u8 = std::ptr::null();
                    let mut rf: *const u8 = std::ptr::null();
                    let mut rlst: *const u8 = std::ptr::null();
                    assert_eq!(
                        unsafe { (c.substring_nametable_scan)(cc, nz.as_ptr(), &mut cf, &mut clst) },
                        unsafe { (r.substring_nametable_scan)(rr, nz.as_ptr(), &mut rf, &mut rlst) },
                        "row92 nametable_scan({name:?}) {p:?}"
                    );
                }
                // row 93: substring_list_get
                let mut cl: *mut *mut u8 = std::ptr::null_mut();
                let mut rl: *mut *mut u8 = std::ptr::null_mut();
                let mut clens: *mut Sz = std::ptr::null_mut();
                let mut rlens: *mut Sz = std::ptr::null_mut();
                let ci = unsafe { (c.substring_list_get)(cmd, &mut cl, &mut clens) };
                let ri = unsafe { (r.substring_list_get)(rmd, &mut rl, &mut rlens) };
                assert_eq!(ci, ri, "row93 substring_list_get {tag} {p:?}");
                if ci == 0 {
                    unsafe { (c.substring_list_free)(cl) };
                    unsafe { (r.substring_list_free)(rl) };
                }
            }
            unsafe { (c.match_data_free)(cmd) };
            unsafe { (r.match_data_free)(rmd) };
        }
        unsafe { (c.code_free)(cc) };
        unsafe { (r.code_free)(rr) };
    }
    // NOTE: a NULL `name` is undefined behaviour in the C — every
    // `*_byname` / `nametable_scan` entry point feeds it straight to
    // `PRIV(strcmp)` / `PRIV(strlen)` with no NULL check — so it is not a
    // comparable row. NULL `firstptr`/`lastptr` ARE handled and are covered by
    // `tests/e_substring.rs`.
    let Some((cc, rr)) = compile2("(?<n>a)", 0, null_ctx) else { panic!() };
    let empty = zp("");
    assert_eq!(
        unsafe { (c.substring_nametable_scan)(cc, empty.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut()) },
        unsafe { (r.substring_nametable_scan)(rr, empty.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut()) },
        "nametable_scan empty name, NULL out ptrs"
    );
    unsafe { (c.code_free)(cc) };
    unsafe { (r.code_free)(rr) };
}

// --------------------------------- rows 94-108, B22: serialize rejections -----

#[test]
fn rows94_108_serialize_rejections() {
    let (c, r) = pair();
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    let Some((cc, rr)) = compile2("abc", 0, null_ctx) else { panic!() };
    let ccodes = [cc as *const Code];
    let rcodes = [rr as *const Code];
    let mut bp: *mut u8 = std::ptr::null_mut();
    let mut bs: Sz = 0;
    let mut rp: *mut u8 = std::ptr::null_mut();
    let mut rs: Sz = 0;
    // rows 94/95/96
    assert_eq!(
        unsafe { (c.serialize_encode)(std::ptr::null(), 1, &mut bp, &mut bs, std::ptr::null_mut()) },
        unsafe { (r.serialize_encode)(std::ptr::null(), 1, &mut rp, &mut rs, std::ptr::null_mut()) },
        "row94 codes NULL"
    );
    assert_eq!(
        unsafe { (c.serialize_encode)(ccodes.as_ptr(), 1, std::ptr::null_mut(), &mut bs, std::ptr::null_mut()) },
        unsafe { (r.serialize_encode)(rcodes.as_ptr(), 1, std::ptr::null_mut(), &mut rs, std::ptr::null_mut()) },
        "row95 bytes NULL"
    );
    assert_eq!(
        unsafe { (c.serialize_encode)(ccodes.as_ptr(), 1, &mut bp, std::ptr::null_mut(), std::ptr::null_mut()) },
        unsafe { (r.serialize_encode)(rcodes.as_ptr(), 1, &mut rp, std::ptr::null_mut(), std::ptr::null_mut()) },
        "row96 size NULL"
    );
    // rows 97 / B22. `number_of_codes` must not exceed the array length: the C
    // walks `codes[0..number_of_codes]`, so a larger count is an out-of-bounds
    // read in the caller's array, not a comparable row.
    for n in [i32::MIN, -2, -1, 0, 1] {
        assert_eq!(
            unsafe { (c.serialize_encode)(ccodes.as_ptr(), n, &mut bp, &mut bs, std::ptr::null_mut()) },
            unsafe { (r.serialize_encode)(rcodes.as_ptr(), n, &mut rp, &mut rs, std::ptr::null_mut()) },
            "row97 number_of_codes={n}"
        );
        if n == 1 {
            assert_eq!(bs, rs);
            unsafe { (c.serialize_free)(bp) };
            unsafe { (r.serialize_free)(rp) };
        }
    }
    // row 98: codes[i] == NULL
    let cnull = [std::ptr::null::<Code>()];
    assert_eq!(
        unsafe { (c.serialize_encode)(cnull.as_ptr(), 1, &mut bp, &mut bs, std::ptr::null_mut()) },
        unsafe { (r.serialize_encode)(cnull.as_ptr(), 1, &mut rp, &mut rs, std::ptr::null_mut()) },
        "row98 codes[0] NULL"
    );
    // row 100: mixed tables
    let ct = unsafe { (c.maketables)(std::ptr::null_mut()) };
    let rt = unsafe { (r.maketables)(std::ptr::null_mut()) };
    let cx = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rx = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    unsafe { (c.set_character_tables)(cx, ct) };
    unsafe { (r.set_character_tables)(rx, rt) };
    let Some((cc2, rr2)) = compile2("xyz", 0, (cx, rx)) else { panic!() };
    let cpair = [cc as *const Code, cc2 as *const Code];
    let rpair = [rr as *const Code, rr2 as *const Code];
    assert_eq!(
        unsafe { (c.serialize_encode)(cpair.as_ptr(), 2, &mut bp, &mut bs, std::ptr::null_mut()) },
        unsafe { (r.serialize_encode)(rpair.as_ptr(), 2, &mut rp, &mut rs, std::ptr::null_mut()) },
        "row100 mixed tables"
    );
    unsafe { (c.code_free)(cc2) };
    unsafe { (r.code_free)(rr2) };
    unsafe { (c.compile_context_free)(cx) };
    unsafe { (r.compile_context_free)(rx) };
    unsafe { (c.maketables_free)(std::ptr::null_mut(), ct) };
    unsafe { (r.maketables_free)(std::ptr::null_mut(), rt) };
    // a good stream, then corrupt each header field (rows 101-108)
    let crc = unsafe { (c.serialize_encode)(ccodes.as_ptr(), 1, &mut bp, &mut bs, std::ptr::null_mut()) };
    let rrc = unsafe { (r.serialize_encode)(rcodes.as_ptr(), 1, &mut rp, &mut rs, std::ptr::null_mut()) };
    assert_eq!((crc, bs), (rrc, rs));
    let good = unsafe { std::slice::from_raw_parts(bp, bs) }.to_vec();
    unsafe { (c.serialize_free)(bp) };
    unsafe { (r.serialize_free)(rp) };
    // `pcre2_serialized_data` = { magic:u32, version:u32, config:u32, number_of_codes:i32 }
    let mut out_c = [std::ptr::null_mut::<Code>(); 2];
    let mut out_r = [std::ptr::null_mut::<Code>(); 2];
    for (off, name) in [(0usize, "magic"), (4, "version"), (8, "config"), (12, "count")] {
        for bad in [0u32, 1, 0xFFFF_FFFF, 0x1234_5678] {
            let mut v = good.clone();
            v[off..off + 4].copy_from_slice(&bad.to_ne_bytes());
            for n in [-1i32, 0, 1, 2] {
                assert_eq!(
                    unsafe { (c.serialize_decode)(out_c.as_mut_ptr(), n, v.as_ptr(), std::ptr::null_mut()) },
                    unsafe { (r.serialize_decode)(out_r.as_mut_ptr(), n, v.as_ptr(), std::ptr::null_mut()) },
                    "rows101-106 decode {name}=0x{bad:x} n={n}"
                );
            }
            assert_eq!(
                unsafe { (c.serialize_get_number_of_codes)(v.as_ptr()) },
                unsafe { (r.serialize_get_number_of_codes)(v.as_ptr()) },
                "row108 get_number_of_codes {name}=0x{bad:x}"
            );
        }
    }
    // rows 101/107: NULL arguments
    assert_eq!(
        unsafe { (c.serialize_decode)(out_c.as_mut_ptr(), 1, std::ptr::null(), std::ptr::null_mut()) },
        unsafe { (r.serialize_decode)(out_r.as_mut_ptr(), 1, std::ptr::null(), std::ptr::null_mut()) },
        "row101 decode NULL bytes"
    );
    assert_eq!(
        unsafe { (c.serialize_decode)(std::ptr::null_mut(), 1, good.as_ptr(), std::ptr::null_mut()) },
        unsafe { (r.serialize_decode)(std::ptr::null_mut(), 1, good.as_ptr(), std::ptr::null_mut()) },
        "row101 decode NULL codes"
    );
    assert_eq!(
        unsafe { (c.serialize_get_number_of_codes)(std::ptr::null()) },
        unsafe { (r.serialize_get_number_of_codes)(std::ptr::null()) },
        "row107 get_number_of_codes NULL"
    );
    unsafe { (c.serialize_free)(std::ptr::null_mut()) };
    unsafe { (r.serialize_free)(std::ptr::null_mut()) };
    unsafe { (c.code_free)(cc) };
    unsafe { (r.code_free)(rr) };
}

// ---------------------------- rows 109-130, B15: substitute rejections -------

#[test]
fn rows109_130_substitute_rejections() {
    let (c, r) = pair();
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    let s = pad(b"abcdef");
    let repl = pad(b"X$1");
    let Some((cc, rr)) = compile2("(a)(b)", 0, null_ctx) else { panic!() };
    let Some((co2, ro2)) = compile2("(c)", 0, null_ctx) else { panic!() };
    let mut cbuf = [0xAAu8; 64];
    let mut rbuf = [0xAAu8; 64];
    // row 109 / B15: every single option bit
    for bit in 0..32u32 {
        let opt = 1u32 << bit;
        let mut cl: Sz = 32;
        let mut rl: Sz = 32;
        let a = unsafe {
            (c.substitute)(cc, s.as_ptr(), 6, 0, opt, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, cbuf.as_mut_ptr(), &mut cl)
        };
        let b = unsafe {
            (r.substitute)(rr, s.as_ptr(), 6, 0, opt, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, rbuf.as_mut_ptr(), &mut rl)
        };
        assert_eq!((a, cl), (b, rl), "row109 substitute option bit 0x{opt:08x}");
    }
    // rows 110/111/112: NULL replacement / subject / match_data
    for (rl_, tag) in [(0usize, "rlen0"), (1, "rlen1"), (3, "rlen3")] {
        let mut cl: Sz = 32;
        let mut rl: Sz = 32;
        let a = unsafe {
            (c.substitute)(cc, s.as_ptr(), 6, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), rl_, cbuf.as_mut_ptr(), &mut cl)
        };
        let b = unsafe {
            (r.substitute)(rr, s.as_ptr(), 6, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(), rl_, rbuf.as_mut_ptr(), &mut rl)
        };
        assert_eq!((a, cl), (b, rl), "row110 NULL replacement {tag}");
        let mut cl: Sz = 32;
        let mut rl: Sz = 32;
        let a = unsafe {
            (c.substitute)(cc, std::ptr::null(), rl_, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, cbuf.as_mut_ptr(), &mut cl)
        };
        let b = unsafe {
            (r.substitute)(rr, std::ptr::null(), rl_, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, rbuf.as_mut_ptr(), &mut rl)
        };
        assert_eq!((a, cl), (b, rl), "row111 NULL subject {tag}");
    }
    let mut cl: Sz = 32;
    let mut rl: Sz = 32;
    let a = unsafe {
        (c.substitute)(cc, s.as_ptr(), 6, 0, PCRE2_SUBSTITUTE_MATCHED, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, cbuf.as_mut_ptr(), &mut cl)
    };
    let b = unsafe {
        (r.substitute)(rr, s.as_ptr(), 6, 0, PCRE2_SUBSTITUTE_MATCHED, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, rbuf.as_mut_ptr(), &mut rl)
    };
    assert_eq!((a, cl), (b, rl), "row112 SUBSTITUTE_MATCHED with NULL match_data");
    // rows 113-119: SUBSTITUTE_MATCHED consistency checks
    let cmd = unsafe { (c.match_data_create)(8, std::ptr::null_mut()) };
    let rmd = unsafe { (r.match_data_create)(8, std::ptr::null_mut()) };
    let s2 = pad(b"abcdef");
    let mut cws = [0 as c_int; 2000];
    let mut rws = [0 as c_int; 2000];
    // (a) DFA-produced match_data -> DFA_UFUNC
    let _ = unsafe {
        (c.dfa_match)(cc, s.as_ptr(), 6, 0, 0, cmd, std::ptr::null_mut(), cws.as_mut_ptr(), 2000)
    };
    let _ = unsafe {
        (r.dfa_match)(rr, s.as_ptr(), 6, 0, 0, rmd, std::ptr::null_mut(), rws.as_mut_ptr(), 2000)
    };
    let mut cl: Sz = 32;
    let mut rl: Sz = 32;
    let a = unsafe {
        (c.substitute)(cc, s.as_ptr(), 6, 0, PCRE2_SUBSTITUTE_MATCHED, cmd, std::ptr::null_mut(), repl.as_ptr(), 3, cbuf.as_mut_ptr(), &mut cl)
    };
    let b = unsafe {
        (r.substitute)(rr, s.as_ptr(), 6, 0, PCRE2_SUBSTITUTE_MATCHED, rmd, std::ptr::null_mut(), repl.as_ptr(), 3, rbuf.as_mut_ptr(), &mut rl)
    };
    assert_eq!((a, cl), (b, rl), "row113 DFA match_data");
    // (b) match_data from another pattern / subject / offset / options
    let _ = unsafe { (c.pcre2_match)(co2, s.as_ptr(), 6, 0, 0, cmd, std::ptr::null_mut()) };
    let _ = unsafe { (r.pcre2_match)(ro2, s.as_ptr(), 6, 0, 0, rmd, std::ptr::null_mut()) };
    for (subj, len, so, mo, tag) in [
        (s.as_ptr(), 6usize, 0usize, 0u32, "row114 other pattern"),
        (s2.as_ptr(), 6, 0, 0, "row115 other subject"),
        (s.as_ptr(), 6, 1, 0, "row116 other offset"),
        (s.as_ptr(), 6, 0, PCRE2_NOTBOL, "row117 other options"),
    ] {
        let mut cl: Sz = 32;
        let mut rl: Sz = 32;
        let a = unsafe {
            (c.substitute)(cc, subj, len, so, PCRE2_SUBSTITUTE_MATCHED | mo, cmd, std::ptr::null_mut(), repl.as_ptr(), 3, cbuf.as_mut_ptr(), &mut cl)
        };
        let b = unsafe {
            (r.substitute)(rr, subj, len, so, PCRE2_SUBSTITUTE_MATCHED | mo, rmd, std::ptr::null_mut(), repl.as_ptr(), 3, rbuf.as_mut_ptr(), &mut rl)
        };
        assert_eq!((a, cl), (b, rl), "{tag}");
    }
    // row 118 / B18: start_offset > length
    for so in [0usize, 6, 7, 100, Sz::MAX - 1, Sz::MAX] {
        let mut cl: Sz = 32;
        let mut rl: Sz = 32;
        let a = unsafe {
            (c.substitute)(cc, s.as_ptr(), 6, so, 0, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, cbuf.as_mut_ptr(), &mut cl)
        };
        let b = unsafe {
            (r.substitute)(rr, s.as_ptr(), 6, so, 0, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, rbuf.as_mut_ptr(), &mut rl)
        };
        assert_eq!((a, cl), (b, rl), "row118 start_offset={so}");
    }
    // rows 120-127: replacement-string errors, buffer overflow
    let bad_repls: &[&str] = &[
        "$*", "$", "${1", "${", "${zz}", "$99", "${99}", "\\q", "\\", "${1:", "${1:x}",
        "${1:-", "${1:+", "${1:+a", "$1$2$3$4", "\\U", "\\L", "\\u", "\\l", "\\E",
        "${name}", "$name", "\\x{110000}", "\\o{999}", "\\c",
    ];
    for rp in bad_repls {
        let rz = pad(rp.as_bytes());
        for opts in [
            0u32,
            PCRE2_SUBSTITUTE_EXTENDED,
            PCRE2_SUBSTITUTE_UNSET_EMPTY,
            PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
            PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
            PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY,
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
            PCRE2_SUBSTITUTE_LITERAL,
        ] {
            for cap in [0usize, 1, 2, 8, 32] {
                let mut cl: Sz = cap;
                let mut rl: Sz = cap;
                let a = unsafe {
                    (c.substitute)(cc, s.as_ptr(), 6, 0, opts, std::ptr::null_mut(), std::ptr::null_mut(), rz.as_ptr(), rp.len(), cbuf.as_mut_ptr(), &mut cl)
                };
                let b = unsafe {
                    (r.substitute)(rr, s.as_ptr(), 6, 0, opts, std::ptr::null_mut(), std::ptr::null_mut(), rz.as_ptr(), rp.len(), rbuf.as_mut_ptr(), &mut rl)
                };
                assert_eq!(
                    (a, cl),
                    (b, rl),
                    "rows120-127 repl={rp:?} opts=0x{opts:08x} cap={cap}"
                );
                if a >= 0 {
                    assert_eq!(
                        &cbuf[..cl.min(64)],
                        &rbuf[..rl.min(64)],
                        "rows120-127 output repl={rp:?} opts=0x{opts:08x} cap={cap}"
                    );
                }
            }
        }
    }
    // row 128: partial match during substitute
    for opts in [PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
        let Some((cp, rp2)) = compile2("abcdefgh", 0, null_ctx) else { continue };
        let mut cl: Sz = 32;
        let mut rl: Sz = 32;
        let a = unsafe {
            (c.substitute)(cp, s.as_ptr(), 6, 0, opts, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, cbuf.as_mut_ptr(), &mut cl)
        };
        let b = unsafe {
            (r.substitute)(rp2, s.as_ptr(), 6, 0, opts, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, rbuf.as_mut_ptr(), &mut rl)
        };
        assert_eq!((a, cl), (b, rl), "row128 partial substitute opts=0x{opts:x}");
        unsafe { (c.code_free)(cp) };
        unsafe { (r.code_free)(rp2) };
    }
    // row 129: pathological global empty-match loop
    for p in ["", "a*", "(?=a)", "\\b", "$", "^"] {
        let Some((cp, rp2)) = compile2(p, 0, null_ctx) else { continue };
        for cap in [0usize, 4, 64] {
            let mut cl: Sz = cap;
            let mut rl: Sz = cap;
            let a = unsafe {
                (c.substitute)(cp, s.as_ptr(), 6, 0, PCRE2_SUBSTITUTE_GLOBAL, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, cbuf.as_mut_ptr(), &mut cl)
            };
            let b = unsafe {
                (r.substitute)(rp2, s.as_ptr(), 6, 0, PCRE2_SUBSTITUTE_GLOBAL, std::ptr::null_mut(), std::ptr::null_mut(), repl.as_ptr(), 3, rbuf.as_mut_ptr(), &mut rl)
            };
            assert_eq!((a, cl), (b, rl), "row129 global {p:?} cap={cap}");
        }
        unsafe { (c.code_free)(cp) };
        unsafe { (r.code_free)(rp2) };
    }
    unsafe { (c.match_data_free)(cmd) };
    unsafe { (r.match_data_free)(rmd) };
    unsafe { (c.code_free)(co2) };
    unsafe { (r.code_free)(ro2) };
    unsafe { (c.code_free)(cc) };
    unsafe { (r.code_free)(rr) };
}

// ---------------------------- rows 131-142, B16: convert & error message -----

#[test]
fn rows131_142_convert_and_message_rejections() {
    let (c, r) = pair();
    let p = zp("a*");
    let mut cbp: *mut u8 = std::ptr::null_mut();
    let mut rbp: *mut u8 = std::ptr::null_mut();
    let mut cl: Sz = 0;
    let mut rl: Sz = 0;
    // row 131: NULL arguments
    assert_eq!(
        unsafe { (c.pattern_convert)(std::ptr::null(), 2, PCRE2_CONVERT_GLOB, &mut cbp, &mut cl, std::ptr::null_mut()) },
        unsafe { (r.pattern_convert)(std::ptr::null(), 2, PCRE2_CONVERT_GLOB, &mut rbp, &mut rl, std::ptr::null_mut()) },
        "row131 NULL pattern"
    );
    // `buffptr == NULL` is a legal "just tell me the length" call
    // (pcre2_convert.c:1209), not an error.
    let mut cl2: Sz = 0;
    let mut rl2: Sz = 0;
    assert_eq!(
        unsafe { (c.pattern_convert)(p.as_ptr(), 2, PCRE2_CONVERT_GLOB, std::ptr::null_mut(), &mut cl2, std::ptr::null_mut()) },
        unsafe { (r.pattern_convert)(p.as_ptr(), 2, PCRE2_CONVERT_GLOB, std::ptr::null_mut(), &mut rl2, std::ptr::null_mut()) },
        "row131 NULL buffptr (length probe)"
    );
    assert_eq!(cl2, rl2, "row131 NULL buffptr length");
    assert_eq!(
        unsafe { (c.pattern_convert)(p.as_ptr(), 2, PCRE2_CONVERT_GLOB, &mut cbp, std::ptr::null_mut(), std::ptr::null_mut()) },
        unsafe { (r.pattern_convert)(p.as_ptr(), 2, PCRE2_CONVERT_GLOB, &mut rbp, std::ptr::null_mut(), std::ptr::null_mut()) },
        "row131 NULL blength"
    );
    // rows 132/133 / B16: every single option bit, and multi-type combinations
    for bit in 0..32u32 {
        let opt = 1u32 << bit;
        let mut cbp: *mut u8 = std::ptr::null_mut();
        let mut rbp: *mut u8 = std::ptr::null_mut();
        let mut cl: Sz = 0;
        let mut rl: Sz = 0;
        let a = unsafe { (c.pattern_convert)(p.as_ptr(), 2, opt, &mut cbp, &mut cl, std::ptr::null_mut()) };
        let b = unsafe { (r.pattern_convert)(p.as_ptr(), 2, opt, &mut rbp, &mut rl, std::ptr::null_mut()) };
        assert_eq!((a, cl), (b, rl), "row132 convert option bit 0x{opt:08x}");
        if a == 0 {
            assert_eq!(
                unsafe { std::slice::from_raw_parts(cbp, cl + 1) },
                unsafe { std::slice::from_raw_parts(rbp, rl + 1) }
            );
            unsafe { (c.converted_pattern_free)(cbp) };
            unsafe { (r.converted_pattern_free)(rbp) };
        }
    }
    for combo in [
        PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED,
        PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_GLOB,
        PCRE2_CONVERT_POSIX_EXTENDED | PCRE2_CONVERT_GLOB,
        PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR | PCRE2_CONVERT_GLOB_NO_STARSTAR,
        0,
        0xFFFF_FFFF,
    ] {
        let mut cbp: *mut u8 = std::ptr::null_mut();
        let mut rbp: *mut u8 = std::ptr::null_mut();
        let mut cl: Sz = 0;
        let mut rl: Sz = 0;
        let a = unsafe { (c.pattern_convert)(p.as_ptr(), 2, combo, &mut cbp, &mut cl, std::ptr::null_mut()) };
        let b = unsafe { (r.pattern_convert)(p.as_ptr(), 2, combo, &mut rbp, &mut rl, std::ptr::null_mut()) };
        assert_eq!((a, cl), (b, rl), "row132 convert combo 0x{combo:08x}");
        if a == 0 {
            unsafe { (c.converted_pattern_free)(cbp) };
            unsafe { (r.converted_pattern_free)(rbp) };
        }
    }
    // rows 134-138: syntax errors and a too-small caller buffer
    let bad: &[&str] = &[
        "[", "[a", "a\\", "\\", "a{", "a{2", "a{2,", "a{,2}", "(", "()", "a\\{2",
        "**", "a**", "[[:foo:]]", "[[.a.]]", "[[=a=]]", "\\(", "\\)", "*", "^*",
    ];
    for pat in bad {
        let pz = zp(pat);
        for ty in [
            PCRE2_CONVERT_POSIX_BASIC,
            PCRE2_CONVERT_POSIX_EXTENDED,
            PCRE2_CONVERT_GLOB,
            PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
            PCRE2_CONVERT_GLOB_NO_STARSTAR,
        ] {
            let mut cbp: *mut u8 = std::ptr::null_mut();
            let mut rbp: *mut u8 = std::ptr::null_mut();
            let mut cl: Sz = 0;
            let mut rl: Sz = 0;
            let a = unsafe { (c.pattern_convert)(pz.as_ptr(), pat.len(), ty, &mut cbp, &mut cl, std::ptr::null_mut()) };
            let b = unsafe { (r.pattern_convert)(pz.as_ptr(), pat.len(), ty, &mut rbp, &mut rl, std::ptr::null_mut()) };
            assert_eq!((a, cl), (b, rl), "rows134-137 {pat:?} ty=0x{ty:x}");
            if a == 0 {
                unsafe { (c.converted_pattern_free)(cbp) };
                unsafe { (r.converted_pattern_free)(rbp) };
            }
            // row 138: caller buffer too small
            for cap in [0usize, 1, 2, 3] {
                let mut cbuf = vec![0xAAu8; cap + 8];
                let mut rbuf = vec![0xAAu8; cap + 8];
                let mut cbp = cbuf.as_mut_ptr();
                let mut rbp = rbuf.as_mut_ptr();
                let mut cl: Sz = cap;
                let mut rl: Sz = cap;
                let a = unsafe { (c.pattern_convert)(pz.as_ptr(), pat.len(), ty, &mut cbp, &mut cl, std::ptr::null_mut()) };
                let b = unsafe { (r.pattern_convert)(pz.as_ptr(), pat.len(), ty, &mut rbp, &mut rl, std::ptr::null_mut()) };
                assert_eq!((a, cl), (b, rl), "row138 {pat:?} ty=0x{ty:x} cap={cap}");
                assert_eq!(cbuf, rbuf, "row138 buffer {pat:?} cap={cap}");
            }
        }
    }
    // rows 139-142 / B24: get_error_message
    for code in [0i32, 1, 2, -1, -100, -1000, -100000, 100000, i32::MIN, i32::MAX] {
        for bufsz in [0usize, 1, 2, 8, 256] {
            let mut cb = vec![0xAAu8; bufsz + 8];
            let mut rb = vec![0xAAu8; bufsz + 8];
            assert_eq!(
                unsafe { (c.get_error_message)(code, cb.as_mut_ptr(), bufsz) },
                unsafe { (r.get_error_message)(code, rb.as_mut_ptr(), bufsz) },
                "rows139-141 get_error_message({code},{bufsz})"
            );
            assert_eq!(cb, rb, "rows139-141 buffer({code},{bufsz})");
        }
        // row 142: `pcre2_get_error_message` has NO NULL check on `buffer`
        // (pcre2_error.c: it does `buffer[i] = 0` unconditionally), so a NULL
        // buffer is only safe together with size == 0, which short-circuits to
        // PCRE2_ERROR_NOMEMORY before any store. That is the comparable row.
        assert_eq!(
            unsafe { (c.get_error_message)(code, std::ptr::null_mut(), 0) },
            unsafe { (r.get_error_message)(code, std::ptr::null_mut(), 0) },
            "row142 get_error_message(NULL, 0) ({code})"
        );
    }
}

// ------------------------------------- rows 143-155, B17: JIT & allocators ----

#[test]
fn rows143_155_jit_and_allocator_rejections() {
    let (c, r) = pair();
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    // rows 143-146 / B17
    assert_eq!(
        unsafe { (c.jit_compile)(std::ptr::null_mut(), 0) },
        unsafe { (r.jit_compile)(std::ptr::null_mut(), 0) },
        "row143 jit_compile NULL"
    );
    let Some((cc, rr)) = compile2("abc", 0, null_ctx) else { panic!() };
    for bit in 0..32u32 {
        let opt = 1u32 << bit;
        assert_eq!(
            unsafe { (c.jit_compile)(cc, opt) },
            unsafe { (r.jit_compile)(rr, opt) },
            "row144 jit_compile bit 0x{opt:08x}"
        );
    }
    for opt in [0u32, 1, 2, 4, 6, 7, 8, 0xFFFF_FFFF] {
        assert_eq!(
            unsafe { (c.jit_compile)(cc, opt) },
            unsafe { (r.jit_compile)(rr, opt) },
            "row145 jit_compile 0x{opt:08x}"
        );
    }
    let cmd = unsafe { (c.match_data_create)(4, std::ptr::null_mut()) };
    let rmd = unsafe { (r.match_data_create)(4, std::ptr::null_mut()) };
    let s = pad(b"abc");
    for mopts in [0u32, PCRE2_NO_JIT, PCRE2_ANCHORED, 0xFFFF_FFFF] {
        assert_eq!(
            unsafe { (c.jit_match)(cc, s.as_ptr(), 3, 0, mopts, cmd, std::ptr::null_mut()) },
            unsafe { (r.jit_match)(rr, s.as_ptr(), 3, 0, mopts, rmd, std::ptr::null_mut()) },
            "row146 jit_match 0x{mopts:08x}"
        );
    }
    assert_eq!(
        unsafe { (c.jit_match)(std::ptr::null(), s.as_ptr(), 3, 0, 0, cmd, std::ptr::null_mut()) },
        unsafe { (r.jit_match)(std::ptr::null(), s.as_ptr(), 3, 0, 0, rmd, std::ptr::null_mut()) },
        "row146 jit_match NULL code"
    );
    unsafe { (c.match_data_free)(cmd) };
    unsafe { (r.match_data_free)(rmd) };
    unsafe { (c.code_free)(cc) };
    unsafe { (r.code_free)(rr) };
    // row 147: jit_stack_create with JIT absent
    for (a, b) in [(0usize, 0usize), (1, 1), (1, 0), (0, 1), (1 << 20, 1 << 30), (usize::MAX, usize::MAX)] {
        let cs = unsafe { (c.jit_stack_create)(a, b, std::ptr::null_mut()) };
        let rs = unsafe { (r.jit_stack_create)(a, b, std::ptr::null_mut()) };
        assert_eq!(cs.is_null(), rs.is_null(), "row147 jit_stack_create({a},{b})");
        unsafe { (c.jit_stack_free)(cs) };
        unsafe { (r.jit_stack_free)(rs) };
    }
    // row 148: general_context_create with a half-supplied allocator pair
    for (m, f) in [
        (None, None),
        (Some(ok_malloc as MallocFn), None),
        (None, Some(ok_free as FreeFn)),
        (Some(ok_malloc as MallocFn), Some(ok_free as FreeFn)),
    ] {
        let a = unsafe { (c.general_context_create)(m, f, std::ptr::null_mut()) };
        let b = unsafe { (r.general_context_create)(m, f, std::ptr::null_mut()) };
        assert_eq!(a.is_null(), b.is_null(), "row148 general_context_create pair");
        unsafe { (c.general_context_free)(a) };
        unsafe { (r.general_context_free)(b) };
    }
    // rows 149-155: allocator that always fails
    for x in [c, r] {
        let g = unsafe {
            (x.general_context_create)(Some(fail_malloc), Some(ok_free), std::ptr::null_mut())
        };
        assert!(g.is_null(), "{}: row149 gcontext with failing malloc", x.name);
    }
    // an allocator that lets the gcontext through but then fails
    for budget in [1usize, 2, 3] {
        let mut cres: Vec<(usize, bool, bool, bool, bool, bool)> = Vec::new();
        let mut rres: Vec<(usize, bool, bool, bool, bool, bool)> = Vec::new();
        for (x, out) in [(c, &mut cres), (r, &mut rres)] {
            BUDGET.store(usize::MAX, Ordering::Relaxed);
            let g = unsafe {
                (x.general_context_create)(Some(budget_malloc), Some(ok_free), std::ptr::null_mut())
            };
            assert!(!g.is_null());
            BUDGET.store(budget, Ordering::Relaxed);
            let cc2 = unsafe { (x.compile_context_create)(g) };
            let mc2 = unsafe { (x.match_context_create)(g) };
            let vc2 = unsafe { (x.convert_context_create)(g) };
            let md2 = unsafe { (x.match_data_create)(8, g) };
            let tb = unsafe { (x.maketables)(g) };
            out.push((
                budget,
                cc2.is_null(),
                mc2.is_null(),
                vc2.is_null(),
                md2.is_null(),
                tb.is_null(),
            ));
            BUDGET.store(usize::MAX, Ordering::Relaxed);
            unsafe { (x.compile_context_free)(cc2) };
            unsafe { (x.match_context_free)(mc2) };
            unsafe { (x.convert_context_free)(vc2) };
            unsafe { (x.match_data_free)(md2) };
            if !tb.is_null() {
                unsafe { (x.maketables_free)(g, tb) };
            }
            unsafe { (x.general_context_free)(g) };
        }
        assert_eq!(cres, rres, "rows150-155 allocator exhaustion budget={budget}");
    }
    // row 153: the C `pcre2_*_context_copy` functions have NO NULL check —
    // they dereference `ccontext->memctl.malloc` straight away
    // (pcre2_context.c:229-276) — so `copy(NULL)` is undefined behaviour, not a
    // comparable row. What IS comparable is copying a context whose allocator
    // then fails, which must yield NULL in both.
    for budget in [0usize, 1] {
        let mut cres: Vec<bool> = Vec::new();
        let mut rres: Vec<bool> = Vec::new();
        for (x, out) in [(c, &mut cres), (r, &mut rres)] {
            BUDGET.store(usize::MAX, Ordering::Relaxed);
            let g = unsafe {
                (x.general_context_create)(Some(budget_malloc), Some(ok_free), std::ptr::null_mut())
            };
            let cc2 = unsafe { (x.compile_context_create)(g) };
            let mc2 = unsafe { (x.match_context_create)(g) };
            let vc2 = unsafe { (x.convert_context_create)(g) };
            BUDGET.store(budget, Ordering::Relaxed);
            let g2 = unsafe { (x.general_context_copy)(g) };
            let c2 = unsafe { (x.compile_context_copy)(cc2) };
            let m2 = unsafe { (x.match_context_copy)(mc2) };
            let v2 = unsafe { (x.convert_context_copy)(vc2) };
            out.extend([g2.is_null(), c2.is_null(), m2.is_null(), v2.is_null()]);
            BUDGET.store(usize::MAX, Ordering::Relaxed);
            unsafe { (x.general_context_free)(g2) };
            unsafe { (x.compile_context_free)(c2) };
            unsafe { (x.match_context_free)(m2) };
            unsafe { (x.convert_context_free)(v2) };
            unsafe { (x.compile_context_free)(cc2) };
            unsafe { (x.match_context_free)(mc2) };
            unsafe { (x.convert_context_free)(vc2) };
            unsafe { (x.general_context_free)(g) };
        }
        assert_eq!(cres, rres, "row153 context_copy under allocator failure budget={budget}");
    }
}

extern "C" fn ok_malloc(n: usize, _d: *mut c_void) -> *mut c_void {
    unsafe { libc_malloc(n) }
}
extern "C" fn ok_free(p: *mut c_void, _d: *mut c_void) {
    unsafe { libc_free(p) }
}
extern "C" fn fail_malloc(_n: usize, _d: *mut c_void) -> *mut c_void {
    std::ptr::null_mut()
}
static BUDGET: AtomicUsize = AtomicUsize::new(usize::MAX);
extern "C" fn budget_malloc(n: usize, _d: *mut c_void) -> *mut c_void {
    loop {
        let cur = BUDGET.load(Ordering::Relaxed);
        if cur == 0 {
            return std::ptr::null_mut();
        }
        let next = if cur == usize::MAX { usize::MAX } else { cur - 1 };
        if BUDGET
            .compare_exchange(cur, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            break;
        }
    }
    unsafe { libc_malloc(n) }
}
unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

// -------------------------------- rows 156-158: private helper error paths ----

#[test]
fn rows156_158_private_helper_errors() {
    let (c, r) = pair();
    // row 156: every _pcre2_valid_utf_8 error shape, offset included
    let cases: &[&[u8]] = &[
        &[0xc2], &[0xe2, 0x82], &[0xf0, 0x90, 0x8d], &[0xf8, 0x88, 0x80, 0x80],
        &[0xfc, 0x84, 0x80, 0x80, 0x80], &[0xc2, 0x41], &[0xe2, 0x82, 0x41],
        &[0xf0, 0x90, 0x8d, 0x41], &[0xf8, 0x88, 0x80, 0x80, 0x41],
        &[0xfc, 0x84, 0x80, 0x80, 0x80, 0x41], &[0xfe], &[0xff], &[0xed, 0xa0, 0x80],
        &[0xc0, 0x80], &[0xe0, 0x80, 0x80], &[0xf0, 0x80, 0x80, 0x80],
        &[0xf8, 0x80, 0x80, 0x80, 0x80], &[0xfc, 0x80, 0x80, 0x80, 0x80, 0x80],
        &[0xf4, 0x90, 0x80, 0x80], &[0xf7, 0xbf, 0xbf, 0xbf], &[0x80], &[0xbf],
    ];
    for s in cases {
        let sp = pad(s);
        for len in [0usize, 1, s.len(), s.len() + 1] {
            let mut co: Sz = 0xDEAD;
            let mut ro: Sz = 0xDEAD;
            let a = unsafe { (c.priv_valid_utf)(sp.as_ptr(), len, &mut co) };
            let b = unsafe { (r.priv_valid_utf)(sp.as_ptr(), len, &mut ro) };
            assert_eq!((a, co), (b, ro), "row156 valid_utf {s:02x?} len={len}");
        }
    }
    // row 157: ckd_smul overflow
    for (a, b) in [
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MIN, -1),
        (65536, 65536),
        (46341, 46341),
        (0, i32::MAX),
        (-1, i32::MIN),
    ] {
        let mut co: Sz = 0xDEAD;
        let mut ro: Sz = 0xDEAD;
        let x = unsafe { (c.priv_ckd_smul)(&mut co, a, b) };
        let y = unsafe { (r.priv_ckd_smul)(&mut ro, a, b) };
        assert_eq!((x, co), (y, ro), "row157 ckd_smul({a},{b})");
    }
    // row 158: ord2utf boundaries
    for cp in [
        0u32, 0x7f, 0x80, 0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff, 0xd800, 0xdfff,
        0x110000, 0x7fffffff,
    ] {
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        let x = unsafe { (c.priv_ord2utf)(cp, cb.as_mut_ptr()) };
        let y = unsafe { (r.priv_ord2utf)(cp, rb.as_mut_ptr()) };
        assert_eq!((x, cb), (y, rb), "row158 ord2utf(0x{cp:x})");
    }
}
