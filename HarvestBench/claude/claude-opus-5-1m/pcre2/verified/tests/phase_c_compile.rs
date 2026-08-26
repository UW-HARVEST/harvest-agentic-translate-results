// Phase-C differential tests for the `pcre2_compile.c` error surface.
//
// Covers ERRORS.md rows 1..221 (every `### pcre2_compile.c — ...` subsection).
// For each row an *exact* invalid input is constructed, `pcre2_compile_8` is
// called through BOTH `.so`s, and the two libraries are compared on
//
//   * whether the returned `pcre2_code *` is NULL,
//   * the numeric error code written through `errorptr`,
//   * the byte offset written through `erroroffset`,
//
// and the C's error code is additionally checked against the code ERRORS.md
// documents for that row.  Where a row is genuinely unreachable in this build
// (8-bit, SUPPORT_UNICODE, no JIT, LINK_SIZE=2, no PCRE2_DEBUG, no EBCDIC, no
// NEVER_BACKSLASH_C) the entry is kept, flagged `unreachable: true`, carries a
// comment saying *why*, and asserts C-vs-Rust agreement on the nearest
// reachable input.
//
// The heap-allocation rows (25..31) and the resource-exhaustion rows
// (210..213) are additionally driven by a fallible allocator installed through
// a general context: the Nth malloc is made to fail for N = 0..40 and the two
// libraries must agree for every N.  That also exercises every `cleanup` path.

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

// ============================================================ compile context

/// Per-case compile-context state.  Sentinels mean "leave the built-in
/// default alone" so that the common case needs no configuration at all.
#[derive(Copy, Clone)]
struct Cfg {
    newline: u32,         // 0        => leave default
    bsr: u32,             // 0        => leave default
    varlookbehind: u32,   // u32::MAX => leave default (255)
    parens_limit: u32,    // 0        => leave default (250)
    max_pat_len: Sz,      // usize::MAX is itself the default
    max_compiled_len: Sz, // usize::MAX is itself the default
    optimize: u32,        // u32::MAX => leave default
    deny_guard: bool,     // install a compile recursion guard that always fails
}

const DEFCFG: Cfg = Cfg {
    newline: 0,
    bsr: 0,
    varlookbehind: u32::MAX,
    parens_limit: 0,
    max_pat_len: usize::MAX,
    max_compiled_len: usize::MAX,
    optimize: u32::MAX,
    deny_guard: false,
};

/// `pcre2_set_compile_recursion_guard_8` callback that always rejects, which is
/// the only way to reach ERR33 (`PARENTHESES_STACK_CHECK`, row 18).
unsafe extern "C" fn deny_guard(_depth: u32, _data: *mut c_void) -> c_int {
    1
}

/// Builds a compile context for one case.  Mirrors
/// `phase_b_compile_match.rs::make_ctx`, plus the knobs the error rows need.
unsafe fn make_ctx(api: &Api, cfg: &Cfg, xopts: u32) -> Ptr {
    let cc = (api.compile_context_create)(ptr::null_mut());
    assert!(!cc.is_null(), "[{}] compile_context_create failed", api.name);
    if cfg.newline != 0 {
        (api.set_newline)(cc, cfg.newline);
    }
    if cfg.bsr != 0 {
        (api.set_bsr)(cc, cfg.bsr);
    }
    if cfg.varlookbehind != u32::MAX {
        (api.set_max_varlookbehind)(cc, cfg.varlookbehind);
    }
    if cfg.parens_limit != 0 {
        (api.set_parens_nest_limit)(cc, cfg.parens_limit);
    }
    if cfg.max_pat_len != usize::MAX {
        (api.set_max_pattern_length)(cc, cfg.max_pat_len);
    }
    if cfg.max_compiled_len != usize::MAX {
        (api.set_max_pattern_compiled_length)(cc, cfg.max_compiled_len);
    }
    if cfg.optimize != u32::MAX {
        (api.set_optimize)(cc, cfg.optimize);
    }
    if cfg.deny_guard {
        (api.set_compile_recursion_guard)(cc, Some(deny_guard), ptr::null_mut());
    }
    if xopts != 0 {
        (api.set_compile_extra_options)(cc, xopts);
    }
    cc
}

// ==================================================================== the table

/// A case's pattern: either a literal, or built at run time because it is far
/// too long to spell out (65536 groups, 17000-character classes, ...).
#[derive(Copy, Clone)]
enum P {
    L(&'static [u8]),
    G(fn() -> Vec<u8>),
}

impl P {
    fn bytes(self) -> Vec<u8> {
        match self {
            P::L(b) => b.to_vec(),
            P::G(f) => f(),
        }
    }
    /// Short rendering for failure messages.
    fn show(self) -> String {
        let b = self.bytes();
        if b.len() <= 60 {
            show(&b)
        } else {
            format!("{}...<{} bytes>", show(&b[..40]), b.len())
        }
    }
}

#[derive(Copy, Clone)]
struct Case {
    /// ERRORS.md row number(s) this case covers.
    rows: &'static [u32],
    pat: P,
    opts: u32,
    xopts: u32,
    cfg: Cfg,
    /// The error code ERRORS.md documents for the row, or 0 if the input must
    /// compile successfully.
    expect: i32,
    /// The documented branch cannot be reached in this build; `expect` then
    /// describes the nearest reachable input instead.
    unreachable: bool,
}

const DEF: Case = Case {
    rows: &[],
    pat: P::L(b""),
    opts: 0,
    xopts: 0,
    cfg: DEFCFG,
    expect: 0,
    unreachable: false,
};

// ------------------------------------------------------- pattern generators

fn rep(s: &str, n: usize) -> Vec<u8> {
    s.repeat(n).into_bytes()
}

/// Row 17: ~9000 wide UTF classes.  Each one contributes a character list, and
/// the *cumulative* `cb.char_lists_size` plus the compiled length is what
/// exceeds MAX_PATTERN_SIZE, at pcre2_compile.c:10840 (`*erroroffset` = 0).
fn gen_many_wide_classes() -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..9000u32 {
        v.extend_from_slice(format!("[\\x{{{:x}}}-\\x{{{:x}}}]", 0x100 + 4 * i, 0x102 + 4 * i).as_bytes());
    }
    v
}

/// Row 108: ONE class whose own character list is bigger than MAX_PATTERN_SIZE,
/// so the check inside pcre2_compile_class.c:1771 fires (and leaves
/// `*erroroffset` pointing into the pattern rather than at 0).  The code points
/// are all above 0xFFFF so every list item costs 4 bytes, and they stay clear
/// of the surrogate range which would give ERR73 first.
fn gen_one_huge_wide_class() -> Vec<u8> {
    let mut v = b"[".to_vec();
    for i in 0..17000u32 {
        v.extend_from_slice(format!("\\x{{{:x}}}", 0x10000 + 2 * i).as_bytes());
    }
    v.push(b']');
    v
}

/// Row 77: `\p{` plus more name characters than the 50-element `name[]` buffer
/// can hold, so the loop exits with `c != '}'`.
fn gen_long_prop_name() -> Vec<u8> {
    let mut v = b"\\p{".to_vec();
    v.extend(std::iter::repeat(b'a').take(60));
    v.push(b'}');
    v
}

/// Row 109: `ECLASS_NEST_LIMIT` is 15 but the guard is
/// `class_depth_m1 >= ECLASS_NEST_LIMIT - 1`, so 16 nested `[` are needed.
fn gen_16_nested_classes() -> Vec<u8> {
    let mut v = rep("[", 16);
    v.push(b'a');
    v.extend(rep("]", 16));
    v
}

/// Rows 133/134/135: the `nest_save` vector lives in the parse workspace,
/// COMPILE_WORK_SIZE (6000) bytes / sizeof(nest_save) (16) = 375 entries.
fn gen_deep_option_groups() -> Vec<u8> {
    let mut v = rep("(?i:", 376);
    v.push(b'a');
    v.extend(rep(")", 376));
    v
}

fn gen_deep_asr_groups() -> Vec<u8> {
    let mut v = rep("(*asr:", 376);
    v.push(b'a');
    v.extend(rep(")", 376));
    v
}

fn gen_deep_cond_assert_groups() -> Vec<u8> {
    let mut v = rep("(?(?=", 376);
    v.push(b'a');
    v.extend(rep(")a)", 376));
    v
}

/// Row 137: MAX_GROUP_NUMBER (65535) capturing groups already exist when the
/// 65536th `(` is seen.
fn gen_65536_groups() -> Vec<u8> {
    rep("()", 65536)
}

/// Row 138: same limit, but hit from the named-group (`DEFINE_NAME`) path.
fn gen_65535_groups_then_named() -> Vec<u8> {
    let mut v = rep("()", 65535);
    v.extend_from_slice(b"(?<a>)");
    v
}

/// Row 139: MAX_NAME_COUNT is 10000.
fn gen_10001_named_groups() -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..10001u32 {
        v.extend_from_slice(format!("(?<a{}>)", i).as_bytes());
    }
    v
}

/// Row 154: MAX_NAME_SIZE is 128.
fn gen_overlong_group_name() -> Vec<u8> {
    let mut v = b"(?<".to_vec();
    v.extend(std::iter::repeat(b'a').take(129));
    v.extend_from_slice(b">x)");
    v
}

/// Row 162: MAX_MARK is 255.
fn gen_overlong_mark() -> Vec<u8> {
    let mut v = b"(*MARK:".to_vec();
    v.extend(std::iter::repeat(b'a').take(256));
    v.push(b')');
    v
}

/// Row 201: `get_branchlength` is entered once per branch; the limit is 2000.
fn gen_complicated_lookbehind() -> Vec<u8> {
    let mut v = b"(?<=(?:".to_vec();
    v.extend(rep("a|", 2100));
    v.extend_from_slice(b"a))");
    v
}

/// Rows 210/211: every nesting level advances the pre-compile workspace pointer
/// by sizeof(OP_CBRA) = 1 + LINK_SIZE + IMM2_SIZE = 5 code units, so ~1200
/// levels overrun COMPILE_WORK_SIZE - WORK_SIZE_SAFETY_MARGIN = 5900.
/// `parens_nest_limit` has to be raised out of the way first, and plain
/// capturing parentheses must be used because `(?:` also consumes a
/// `nest_save` slot (limit 375, row 133).
fn gen_deep_capture_nesting() -> Vec<u8> {
    let mut v = rep("(", 1200);
    v.push(b'a');
    v.extend(rep(")", 1200));
    v
}

/// Row 213: `compile_regex` branch-length accumulation overflow.
fn gen_big_group_repeat() -> Vec<u8> {
    let mut v = b"(?:".to_vec();
    v.extend(std::iter::repeat(b'a').take(200));
    v.extend_from_slice(b"){65535}");
    v
}

// ------------------------------------------------------------------ the cases

const CASES: &[Case] = &[
    // bit 28 is the only option bit outside PUBLIC_COMPILE_OPTIONS
    Case { rows: &[4], pat: P::L(b"a"), opts: 0x10000000, expect: 117, ..DEF },
    // extra-option bit 17 is outside PUBLIC_COMPILE_EXTRA_OPTIONS
    Case { rows: &[5], pat: P::L(b"a"), xopts: 0x00020000, expect: 117, ..DEF },
    Case { rows: &[5], pat: P::L(b"a"), xopts: 0x80000000, expect: 117, ..DEF },
    Case { rows: &[6], pat: P::L(b"a"), opts: PCRE2_MULTILINE | PCRE2_LITERAL, expect: 192, ..DEF },
    // PCRE2_LITERAL|PCRE2_CASELESS is legal
    Case { rows: &[6], pat: P::L(b"abc"), opts: PCRE2_CASELESS | PCRE2_LITERAL, expect: 0, ..DEF },
    Case { rows: &[7], pat: P::L(b"a"), opts: PCRE2_LITERAL, xopts: PCRE2_EXTRA_ALT_BSUX, expect: 192, ..DEF },
    Case { rows: &[8], pat: P::L(b"abcd"), cfg: Cfg { max_pat_len: 3, ..DEFCFG }, expect: 188, ..DEF },
    // patlen == max_pattern_length is legal
    Case { rows: &[8], pat: P::L(b"abc"), cfg: Cfg { max_pat_len: 3, ..DEFCFG }, expect: 0, ..DEF },
    Case { rows: &[9], pat: P::L(b"(*LIMIT_MATCH=x)a"), expect: 160, ..DEF },
    Case { rows: &[9], pat: P::L(b"(*LIMIT_HEAP=99"), expect: 160, ..DEF },
    Case { rows: &[9], pat: P::L(b"(*LIMIT_DEPTH=)a"), expect: 160, ..DEF },
    Case { rows: &[10], pat: P::L(b"(*UTF)a"), opts: PCRE2_NEVER_UTF, expect: 174, ..DEF },
    Case { rows: &[10], pat: P::L(b"a"), opts: PCRE2_NEVER_UTF | PCRE2_UTF, expect: 174, ..DEF },
    // valid_utf error, code is one of -3..-23
    Case { rows: &[11], pat: P::L(b"\xff"), opts: PCRE2_UTF, expect: -23, ..DEF },
    Case { rows: &[11], pat: P::L(b"a\x80"), opts: PCRE2_UTF, expect: -22, ..DEF },
    Case { rows: &[11], pat: P::L(b"\xc2"), opts: PCRE2_UTF, expect: -3, ..DEF },
    Case { rows: &[11], pat: P::L(b"\xe0\x80\x80"), opts: PCRE2_UTF, expect: -18, ..DEF },
    Case { rows: &[11], pat: P::L(b"\xf5\x80\x80\x80"), opts: PCRE2_UTF, expect: -15, ..DEF },
    Case { rows: &[12], pat: P::L(b"(*UCP)a"), opts: PCRE2_NEVER_UCP, expect: 175, ..DEF },
    Case { rows: &[12], pat: P::L(b"a"), opts: PCRE2_NEVER_UCP | PCRE2_UCP, expect: 175, ..DEF },
    Case { rows: &[13], pat: P::L(b"a"), xopts: PCRE2_EXTRA_TURKISH_CASING, expect: 204, ..DEF },
    Case { rows: &[14], pat: P::L(b"a"), opts: PCRE2_UCP, xopts: PCRE2_EXTRA_TURKISH_CASING, expect: 205, ..DEF },
    Case { rows: &[15], pat: P::L(b"a"), opts: PCRE2_UTF, xopts: PCRE2_EXTRA_CASELESS_RESTRICT | PCRE2_EXTRA_TURKISH_CASING, expect: 206, ..DEF },
    Case { rows: &[16], pat: P::L(b"abc"), cfg: Cfg { max_compiled_len: 1, ..DEFCFG }, expect: 201, ..DEF },
    // cumulative cb.char_lists_size + length > MAX_PATTERN_SIZE, checked at
    // pcre2_compile.c:10840; *erroroffset==0
    Case { rows: &[17], pat: P::G(gen_many_wide_classes), opts: PCRE2_UTF, expect: 120, ..DEF },
    Case { rows: &[18], pat: P::L(b"(a)"), cfg: Cfg { deny_guard: true, ..DEFCFG }, expect: 133, ..DEF },
    // ccontext->newline_convention can only be 1..6 through the public API;
    // pcre2_set_newline_8(99) is rejected and leaves the context untouched, so the nearest
    // reachable input just compiles
    Case { rows: &[19], pat: P::L(b"a"), cfg: Cfg { newline: 99, ..DEFCFG }, expect: 0, unreachable: true, ..DEF },
    Case { rows: &[32], pat: P::L(b"a\\"), expect: 101, ..DEF },
    Case { rows: &[33], pat: P::L(b"a\\c"), expect: 102, ..DEF },
    Case { rows: &[34], pat: P::L(b"\\y"), expect: 103, ..DEF },
    Case { rows: &[34], pat: P::L(b"\\i"), expect: 103, ..DEF },
    Case { rows: &[34], pat: P::L(b"\\T"), expect: 103, ..DEF },
    Case { rows: &[35], pat: P::L(b"a\\c\x01"), expect: 168, ..DEF },
    Case { rows: &[35], pat: P::L(b"a\\c\x7f"), expect: 168, ..DEF },
    Case { rows: &[36], pat: P::L(b"\\F"), expect: 137, ..DEF },
    Case { rows: &[36], pat: P::L(b"\\l"), expect: 137, ..DEF },
    Case { rows: &[36], pat: P::L(b"\\L"), expect: 137, ..DEF },
    Case { rows: &[37], pat: P::L(b"\\u"), expect: 137, ..DEF },
    Case { rows: &[38], pat: P::L(b"\\U"), expect: 137, ..DEF },
    Case { rows: &[39], pat: P::L(b"[\\N{2}]"), expect: 137, ..DEF },
    Case { rows: &[40], pat: P::L(b"\\N{abc}"), expect: 137, ..DEF },
    Case { rows: &[41], pat: P::L(b"\\N{U+0041}"), expect: 193, ..DEF },
    Case { rows: &[42], pat: P::L(b"\\u{fffffffff}"), xopts: PCRE2_EXTRA_ALT_BSUX, expect: 177, ..DEF },
    Case { rows: &[43], pat: P::L(b"\\u{110000}"), opts: PCRE2_UTF, xopts: PCRE2_EXTRA_ALT_BSUX, expect: 177, ..DEF },
    Case { rows: &[44], pat: P::L(b"\\u0100"), opts: PCRE2_ALT_BSUX, expect: 177, ..DEF },
    Case { rows: &[45], pat: P::L(b"\\u{d800}"), opts: PCRE2_UTF, xopts: PCRE2_EXTRA_ALT_BSUX, expect: 173, ..DEF },
    Case { rows: &[46], pat: P::L(b"\\o{154000}"), opts: PCRE2_UTF, expect: 173, ..DEF },
    Case { rows: &[47], pat: P::L(b"\\x{d800}"), opts: PCRE2_UTF, expect: 173, ..DEF },
    Case { rows: &[48], pat: P::L(b"\\o{4000}"), expect: 134, ..DEF },
    Case { rows: &[48], pat: P::L(b"\\o{4200000}"), opts: PCRE2_UTF, expect: 134, ..DEF },
    Case { rows: &[49], pat: P::L(b"\\x{100}"), expect: 134, ..DEF },
    Case { rows: &[49], pat: P::L(b"\\x{110000}"), opts: PCRE2_UTF, expect: 134, ..DEF },
    Case { rows: &[50], pat: P::L(b"\\o7"), expect: 155, ..DEF },
    Case { rows: &[51], pat: P::L(b"\\o{}"), expect: 178, ..DEF },
    Case { rows: &[51], pat: P::L(b"\\o{"), expect: 178, ..DEF },
    Case { rows: &[52], pat: P::L(b"\\x{}"), expect: 178, ..DEF },
    Case { rows: &[52], pat: P::L(b"\\x{"), expect: 178, ..DEF },
    Case { rows: &[52], pat: P::L(b"\\N{U+}"), opts: PCRE2_UTF, expect: 178, ..DEF },
    Case { rows: &[53], pat: P::L(b"\\xz"), expect: 178, ..DEF },
    Case { rows: &[53], pat: P::L(b"\\x"), expect: 178, ..DEF },
    Case { rows: &[54], pat: P::L(b"\\o{12x}"), expect: 164, ..DEF },
    Case { rows: &[55], pat: P::L(b"\\x{1z}"), expect: 167, ..DEF },
    Case { rows: &[56], pat: P::L(b"\\400"), expect: 151, ..DEF },
    Case { rows: &[57], pat: P::L(b"\\400"), xopts: PCRE2_EXTRA_PYTHON_OCTAL, expect: 202, ..DEF },
    Case { rows: &[58], pat: P::L(b"\\0"), xopts: PCRE2_EXTRA_NO_BS0, expect: 198, ..DEF },
    Case { rows: &[59], pat: P::L(b"\\g"), expect: 157, ..DEF },
    Case { rows: &[60], pat: P::L(b"\\gx"), expect: 157, ..DEF },
    Case { rows: &[61], pat: P::L(b"\\g="), expect: 157, ..DEF },
    Case { rows: &[62], pat: P::L(b"\\kx"), expect: 169, ..DEF },
    Case { rows: &[62], pat: P::L(b"\\k"), expect: 169, ..DEF },
    Case { rows: &[63], pat: P::L(b"(a)\\g{1a}"), expect: 219, ..DEF },
    Case { rows: &[64], pat: P::L(b"(a)\\g<1a"), expect: 219, ..DEF },
    Case { rows: &[65], pat: P::L(b"(a)\\g{-0}"), expect: 126, ..DEF },
    Case { rows: &[65], pat: P::L(b"(a)\\g{+0}"), expect: 126, ..DEF },
    Case { rows: &[65], pat: P::L(b"(?+0)"), expect: 126, ..DEF },
    Case { rows: &[65], pat: P::L(b"(?-0)"), expect: 126, ..DEF },
    Case { rows: &[66], pat: P::L(b"\\g{-1}"), expect: 115, ..DEF },
    Case { rows: &[67], pat: P::L(b"\\g0"), expect: 115, ..DEF },
    Case { rows: &[67], pat: P::L(b"\\g{0}"), expect: 115, ..DEF },
    Case { rows: &[68], pat: P::L(b"\\g{70000}"), expect: 161, ..DEF },
    Case { rows: &[69], pat: P::L(b"\\g70000"), expect: 161, ..DEF },
    Case { rows: &[70], pat: P::L(b"\\g<70000>"), expect: 161, ..DEF },
    // PCRE2_EXTRA_PYTHON_OCTAL: the 3-digit-octal peek fails, so read_number runs and the value
    // exceeds MAX_GROUP_NUMBER
    Case { rows: &[71], pat: P::L(b"\\79999"), xopts: PCRE2_EXTRA_PYTHON_OCTAL, expect: 161, ..DEF },
    // Perl mode: read_number fails, sentinel INT_MAX; needs a leading 8 or 9 so the octal
    // fall-through is not taken
    Case { rows: &[72], pat: P::L(b"\\800000"), expect: 161, ..DEF },
    Case { rows: &[73], pat: P::L(b"a\\p"), expect: 146, ..DEF },
    Case { rows: &[74], pat: P::L(b"\\p{"), expect: 146, ..DEF },
    Case { rows: &[75], pat: P::L(b"\\p{L"), expect: 146, ..DEF },
    // name character outside '&'..'z'; '+' is INSIDE that range and yields ERR47 instead
    Case { rows: &[76], pat: P::L(b"\\p{L!}"), expect: 146, ..DEF },
    Case { rows: &[77], pat: P::G(gen_long_prop_name), expect: 146, ..DEF },
    Case { rows: &[78], pat: P::L(b"\\p9"), expect: 146, ..DEF },
    Case { rows: &[79], pat: P::L(b"\\p{Zz}"), expect: 147, ..DEF },
    Case { rows: &[80], pat: P::L(b"\\p{foo:Latin}"), expect: 147, ..DEF },
    Case { rows: &[81], pat: P::L(b"\\p{sc:Lu}"), expect: 147, ..DEF },
    // ERR45 (UNICODE_PROPERTIES_UNAVAILABLE) needs a build without SUPPORT_UNICODE
    Case { rows: &[82], pat: P::L(b"\\p{L}"), expect: 0, unreachable: true, ..DEF },
    Case { rows: &[83], pat: P::L(b"\\C"), opts: PCRE2_NEVER_BACKSLASH_C, expect: 183, ..DEF },
    // ERR85 (BACKSLASH_C_LIBRARY_DISABLED) needs the NEVER_BACKSLASH_C build macro
    Case { rows: &[84], pat: P::L(b"\\C"), expect: 0, unreachable: true, ..DEF },
    Case { rows: &[85], pat: P::L(b"(?=\\K)"), expect: 199, ..DEF },
    Case { rows: &[85], pat: P::L(b"(?<=a\\K)"), expect: 199, ..DEF },
    Case { rows: &[85], pat: P::L(b"(?!\\K)"), expect: 199, ..DEF },
    // PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK permits it
    Case { rows: &[85], pat: P::L(b"(?=\\K)"), xopts: PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK, expect: 0, ..DEF },
    // ERR91 (NO_SURROGATES_IN_UTF16) is a 16-bit-only check, not compiled in the 8-bit library
    Case { rows: &[86], pat: P::L(b"a"), opts: PCRE2_UTF, xopts: PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES, expect: 0, unreachable: true, ..DEF },
    Case { rows: &[87], pat: P::L(b"a{2,1}"), expect: 104, ..DEF },
    Case { rows: &[88], pat: P::L(b"a{65536}"), expect: 105, ..DEF },
    Case { rows: &[89], pat: P::L(b"a{,65536}"), expect: 105, ..DEF },
    Case { rows: &[90], pat: P::L(b"a{1,65536}"), expect: 105, ..DEF },
    Case { rows: &[91], pat: P::L(b"*a"), expect: 109, ..DEF },
    Case { rows: &[91], pat: P::L(b"+a"), expect: 109, ..DEF },
    Case { rows: &[91], pat: P::L(b"?a"), expect: 109, ..DEF },
    Case { rows: &[91], pat: P::L(b"{2}a"), expect: 109, ..DEF },
    Case { rows: &[91], pat: P::L(b"(?i)*"), expect: 109, ..DEF },
    Case { rows: &[91], pat: P::L(b"\\b*"), expect: 109, ..DEF },
    Case { rows: &[91], pat: P::L(b"^*"), expect: 109, ..DEF },
    // ERR10 (INTERNAL_UNEXPECTED_REPEAT) is a PCRE2_DEBUG_UNREACHABLE branch: every op_previous
    // reaching it is a repeatable opcode
    Case { rows: &[92], pat: P::L(b"a*"), expect: 0, unreachable: true, ..DEF },
    Case { rows: &[93], pat: P::L(b"["), expect: 106, ..DEF },
    Case { rows: &[94], pat: P::L(b"[a"), expect: 106, ..DEF },
    Case { rows: &[95], pat: P::L(b"[\\B]"), expect: 107, ..DEF },
    Case { rows: &[95], pat: P::L(b"[\\R]"), expect: 107, ..DEF },
    Case { rows: &[95], pat: P::L(b"[\\X]"), expect: 107, ..DEF },
    Case { rows: &[96], pat: P::L(b"[\\A]"), expect: 107, ..DEF },
    Case { rows: &[96], pat: P::L(b"[\\Z]"), expect: 107, ..DEF },
    Case { rows: &[96], pat: P::L(b"[\\z]"), expect: 107, ..DEF },
    Case { rows: &[96], pat: P::L(b"[\\G]"), expect: 107, ..DEF },
    Case { rows: &[96], pat: P::L(b"[\\K]"), expect: 107, ..DEF },
    Case { rows: &[96], pat: P::L(b"[\\C]"), expect: 107, ..DEF },
    Case { rows: &[97], pat: P::L(b"[\\N]"), expect: 171, ..DEF },
    Case { rows: &[98], pat: P::L(b"[b-a]"), expect: 108, ..DEF },
    Case { rows: &[99], pat: P::L(b"[a-[:digit:]]"), expect: 150, ..DEF },
    Case { rows: &[100], pat: P::L(b"[[:digit:]-a]"), expect: 150, ..DEF },
    Case { rows: &[101], pat: P::L(b"[a-\\d]"), expect: 150, ..DEF },
    Case { rows: &[102], pat: P::L(b"[\\d-\\w]"), expect: 150, ..DEF },
    Case { rows: &[103], pat: P::L(b"[\\d-a]"), expect: 150, ..DEF },
    Case { rows: &[104], pat: P::L(b"[:alpha:]"), expect: 112, ..DEF },
    Case { rows: &[105], pat: P::L(b"[.ch.]"), expect: 113, ..DEF },
    Case { rows: &[105], pat: P::L(b"[=ch=]"), expect: 113, ..DEF },
    Case { rows: &[106], pat: P::L(b"[[.ch.]]"), expect: 113, ..DEF },
    Case { rows: &[106], pat: P::L(b"[[=ch=]]"), expect: 113, ..DEF },
    Case { rows: &[107], pat: P::L(b"[[:foo:]]"), expect: 130, ..DEF },
    Case { rows: &[107], pat: P::L(b"[[:^foo:]]"), expect: 130, ..DEF },
    // one class whose own character-list exceeds MAX_PATTERN_SIZE (pcre2_compile_class.c:1771);
    // *erroroffset is left pointing into the pattern
    Case { rows: &[108], pat: P::G(gen_one_huge_wide_class), opts: PCRE2_UTF, expect: 120, ..DEF },
    // ECLASS_NEST_LIMIT is 15 and the test is class_depth_m1 >= 14, so 16 nested '[' are needed
    Case { rows: &[109], pat: P::G(gen_16_nested_classes), opts: PCRE2_ALT_EXTENDED_CLASS, expect: 207, ..DEF },
    Case { rows: &[110], pat: P::L(b"[a---b]"), opts: PCRE2_ALT_EXTENDED_CLASS, expect: 208, ..DEF },
    Case { rows: &[110], pat: P::L(b"[a|||b]"), opts: PCRE2_ALT_EXTENDED_CLASS, expect: 208, ..DEF },
    Case { rows: &[110], pat: P::L(b"[a&&&b]"), opts: PCRE2_ALT_EXTENDED_CLASS, expect: 208, ..DEF },
    Case { rows: &[110], pat: P::L(b"[a~~~b]"), opts: PCRE2_ALT_EXTENDED_CLASS, expect: 208, ..DEF },
    Case { rows: &[111], pat: P::L(b"[--a]"), opts: PCRE2_ALT_EXTENDED_CLASS, expect: 209, ..DEF },
    Case { rows: &[112], pat: P::L(b"(?[+[a]])"), expect: 209, ..DEF },
    Case { rows: &[112], pat: P::L(b"(?[|[a]])"), expect: 209, ..DEF },
    Case { rows: &[112], pat: P::L(b"(?[-[a]])"), expect: 209, ..DEF },
    Case { rows: &[112], pat: P::L(b"(?[&[a]])"), expect: 209, ..DEF },
    Case { rows: &[112], pat: P::L(b"(?[^[a]])"), expect: 209, ..DEF },
    Case { rows: &[113], pat: P::L(b"[a--]"), opts: PCRE2_ALT_EXTENDED_CLASS, expect: 210, ..DEF },
    Case { rows: &[113], pat: P::L(b"(?[[a]+])"), expect: 210, ..DEF },
    Case { rows: &[114], pat: P::L(b"[[a]--[b]&&[c]]"), opts: PCRE2_ALT_EXTENDED_CLASS, expect: 211, ..DEF },
    Case { rows: &[115], pat: P::L(b"[[a]"), opts: PCRE2_ALT_EXTENDED_CLASS, expect: 212, ..DEF },
    Case { rows: &[116], pat: P::L(b"(?[[a][b]])"), expect: 213, ..DEF },
    Case { rows: &[117], pat: P::L(b"(?[[a][:digit:]])"), expect: 213, ..DEF },
    Case { rows: &[118], pat: P::L(b"(?[[a]![b]])"), expect: 213, ..DEF },
    Case { rows: &[119], pat: P::L(b"(?[[a]\\d])"), expect: 213, ..DEF },
    Case { rows: &[120], pat: P::L(b"(?[[a]\\x41])"), expect: 213, ..DEF },
    Case { rows: &[121], pat: P::L(b"(?[])"), expect: 214, ..DEF },
    Case { rows: &[122], pat: P::L(b"(?[[a]]"), expect: 215, ..DEF },
    Case { rows: &[123], pat: P::L(b"(?[\\Qa\\E])"), expect: 216, ..DEF },
    Case { rows: &[124], pat: P::L(b"(?[a])"), expect: 216, ..DEF },
    Case { rows: &[125], pat: P::L(b"(?[([a]]"), expect: 114, ..DEF },
    Case { rows: &[126], pat: P::L(b"(?[("), expect: 114, ..DEF },
    // the ERR14 written at pcre2_compile.c:4700 is immediately overwritten by the following
    // if/else, so the observable code is ERR6
    Case { rows: &[127], pat: P::L(b"(?[([a]"), expect: 106, ..DEF },
    Case { rows: &[128], pat: P::L(b"(?[)"), expect: 122, ..DEF },
    Case { rows: &[129], pat: P::L(b"(a"), expect: 114, ..DEF },
    Case { rows: &[129], pat: P::L(b"(?:a"), expect: 114, ..DEF },
    Case { rows: &[129], pat: P::L(b"(?(1)a"), expect: 114, ..DEF },
    Case { rows: &[129], pat: P::L(b"(?&name"), expect: 114, ..DEF },
    // the capture list must end right after a complete entry; '(1a' gives ERR24 instead
    Case { rows: &[130], pat: P::L(b"(a)(*scs:(1"), expect: 114, ..DEF },
    Case { rows: &[131], pat: P::L(b"a)"), expect: 122, ..DEF },
    Case { rows: &[132], pat: P::L(b"((a))"), cfg: Cfg { parens_limit: 1, ..DEFCFG }, expect: 119, ..DEF },
    // nest_save vector holds COMPILE_WORK_SIZE bytes / sizeof(nest_save) = 6000/16 = 375
    // entries; parens_nest_limit must be raised out of the way first
    Case { rows: &[133], pat: P::G(gen_deep_option_groups), cfg: Cfg { parens_limit: 100000, ..DEFCFG }, expect: 184, ..DEF },
    Case { rows: &[134], pat: P::G(gen_deep_asr_groups), cfg: Cfg { parens_limit: 100000, ..DEFCFG }, expect: 184, ..DEF },
    Case { rows: &[135], pat: P::G(gen_deep_cond_assert_groups), cfg: Cfg { parens_limit: 100000, ..DEFCFG }, expect: 184, ..DEF },
    Case { rows: &[136], pat: P::L(b"(?#comment"), expect: 118, ..DEF },
    Case { rows: &[137], pat: P::G(gen_65536_groups), expect: 197, ..DEF },
    Case { rows: &[138], pat: P::G(gen_65535_groups_then_named), expect: 197, ..DEF },
    Case { rows: &[139], pat: P::G(gen_10001_named_groups), expect: 149, ..DEF },
    Case { rows: &[140], pat: P::L(b"(?z)"), expect: 111, ..DEF },
    Case { rows: &[140], pat: P::L(b"(?-z)"), expect: 111, ..DEF },
    Case { rows: &[140], pat: P::L(b"(?q:a)"), expect: 111, ..DEF },
    Case { rows: &[141], pat: P::L(b"(?^-i)"), expect: 194, ..DEF },
    Case { rows: &[141], pat: P::L(b"(?i-s-x)"), expect: 194, ..DEF },
    Case { rows: &[142], pat: P::L(b"(?Px)"), expect: 141, ..DEF },
    Case { rows: &[143], pat: P::L(b"(?Rx)"), expect: 158, ..DEF },
    Case { rows: &[144], pat: P::L(b"(?+a)"), expect: 129, ..DEF },
    Case { rows: &[145], pat: P::L(b"(?70000)"), expect: 161, ..DEF },
    Case { rows: &[145], pat: P::L(b"(?+70000)"), expect: 161, ..DEF },
    Case { rows: &[146], pat: P::L(b"(?2)a"), expect: 115, ..DEF },
    Case { rows: &[147], pat: P::L(b"\\1"), expect: 115, ..DEF },
    Case { rows: &[148], pat: P::L(b"\\k<xyz>"), expect: 115, ..DEF },
    Case { rows: &[148], pat: P::L(b"(?P=xyz)"), expect: 115, ..DEF },
    Case { rows: &[148], pat: P::L(b"\\g{xyz}"), expect: 115, ..DEF },
    Case { rows: &[149], pat: P::L(b"(?<"), expect: 162, ..DEF },
    Case { rows: &[149], pat: P::L(b"(?'"), expect: 162, ..DEF },
    Case { rows: &[149], pat: P::L(b"(?&"), expect: 162, ..DEF },
    Case { rows: &[150], pat: P::L(b"(?<>a)"), expect: 162, ..DEF },
    Case { rows: &[151], pat: P::L(b"(?<abc"), expect: 142, ..DEF },
    Case { rows: &[151], pat: P::L(b"(?'abc"), expect: 142, ..DEF },
    Case { rows: &[151], pat: P::L(b"\\k<abc"), expect: 142, ..DEF },
    Case { rows: &[151], pat: P::L(b"(?P=abc"), expect: 142, ..DEF },
    Case { rows: &[152], pat: P::L(b"(?<1a>x)"), expect: 144, ..DEF },
    Case { rows: &[153], pat: P::L(b"(?<\xd9\xa1a>x)"), opts: PCRE2_UTF, expect: 144, ..DEF },
    Case { rows: &[154], pat: P::G(gen_overlong_group_name), expect: 148, ..DEF },
    Case { rows: &[155], pat: P::L(b"(?<a>x)(?<a>y)"), expect: 143, ..DEF },
    // legal with PCRE2_DUPNAMES
    Case { rows: &[155], pat: P::L(b"(?<a>x)(?<a>y)"), opts: PCRE2_DUPNAMES, expect: 0, ..DEF },
    Case { rows: &[156], pat: P::L(b"(?|(?<a>x)|(?<b>y))"), expect: 165, ..DEF },
    // read_name's ERR60 'no characters in name' arm is unreachable for verbs: the caller at
    // pcre2_compile.c:4749 already breaks out when ptrend-ptr<=1 or ptr[1]==')' , so read_name
    // is never entered with nothing after the '*'. '(*)' therefore reports ERR9 for the stray
    // quantifier
    Case { rows: &[157], pat: P::L(b"(*)"), expect: 109, unreachable: true, ..DEF },
    Case { rows: &[158], pat: P::L(b"(*ACCEPT.)"), expect: 160, ..DEF },
    Case { rows: &[159], pat: P::L(b"(*FOO)"), expect: 160, ..DEF },
    Case { rows: &[160], pat: P::L(b"(*MARK:abc"), expect: 160, ..DEF },
    Case { rows: &[161], pat: P::L(b"(*MARK)"), expect: 166, ..DEF },
    Case { rows: &[161], pat: P::L(b"(*MARK:)"), expect: 166, ..DEF },
    Case { rows: &[162], pat: P::G(gen_overlong_mark), expect: 176, ..DEF },
    Case { rows: &[163], pat: P::L(b"(*MARK:\\d)"), opts: PCRE2_ALT_VERBNAMES, expect: 140, ..DEF },
    Case { rows: &[164], pat: P::L(b"(*pla)"), expect: 195, ..DEF },
    Case { rows: &[165], pat: P::L(b"(*foo:a)"), expect: 195, ..DEF },
    // ERR96 (SCRIPT_RUN_NOT_AVAILABLE) needs a build without SUPPORT_UNICODE
    Case { rows: &[166], pat: P::L(b"(*sr:a)"), expect: 0, unreachable: true, ..DEF },
    // ERR89 from the alasmeta dispatch default: every meta in the table is handled,
    // PCRE2_DEBUG_UNREACHABLE branch
    Case { rows: &[167], pat: P::L(b"(*atomic:a)"), expect: 0, unreachable: true, ..DEF },
    Case { rows: &[168], pat: P::L(b"(*scs:a)"), expect: 218, ..DEF },
    Case { rows: &[169], pat: P::L(b"(a)(*scs:("), expect: 217, ..DEF },
    Case { rows: &[170], pat: P::L(b"(a)(*scs:(x)b)"), expect: 217, ..DEF },
    Case { rows: &[171], pat: P::L(b"(a)(*scs:(1;2)b)"), expect: 124, ..DEF },
    Case { rows: &[172], pat: P::L(b"(a)(*scs:(0)b)"), expect: 115, ..DEF },
    Case { rows: &[173], pat: P::L(b"(a)(*scs:(70000)b)"), expect: 161, ..DEF },
    Case { rows: &[174], pat: P::L(b"(a)(*scs:(<xyz>)b)"), expect: 115, ..DEF },
    Case { rows: &[175], pat: P::L(b"(a)(*scs:(2)b)"), expect: 115, ..DEF },
    // ERR53 at pcre2_compile_cgroup.c:235 needs a duplicate-name slot missing from the name
    // table, which cannot happen after the name table has been built
    Case { rows: &[176], pat: P::L(b"(?<a>x)(?<a>y)(*scs:(<a>)b)"), opts: PCRE2_DUPNAMES, expect: 0, unreachable: true, ..DEF },
    Case { rows: &[177], pat: P::L(b"(?(?i)a)"), expect: 128, ..DEF },
    // expect_cond_assert only becomes non-zero when the character after '(?(' is '?' or '*';
    // the callout then decrements it to 1, which is what makes the \Q check fire
    Case { rows: &[178], pat: P::L(b"(?(?C1)\\Qa\\E)b)"), expect: 128, ..DEF },
    // the alpha assertion must follow '(?(' directly; '(?((*atomic:a))b)' is read as a group
    // name and gives ERR62
    Case { rows: &[179], pat: P::L(b"(?(*atomic:a)b)"), expect: 128, ..DEF },
    Case { rows: &[180], pat: P::L(b"(?(1a)b)"), expect: 124, ..DEF },
    Case { rows: &[180], pat: P::L(b"(?(<n>x)b)"), expect: 124, ..DEF },
    Case { rows: &[181], pat: P::L(b"(?(0)a)"), expect: 115, ..DEF },
    Case { rows: &[182], pat: P::L(b"(?(70000)a)"), expect: 161, ..DEF },
    Case { rows: &[183], pat: P::L(b"(?(2)a)"), expect: 115, ..DEF },
    Case { rows: &[184], pat: P::L(b"(?(R70000)a)"), expect: 161, ..DEF },
    Case { rows: &[185], pat: P::L(b"(?(xyz)a)"), expect: 115, ..DEF },
    // group 1 must exist, otherwise ERR15 is reported first
    Case { rows: &[186], pat: P::L(b"(a)(?(1)a|b|c)"), expect: 127, ..DEF },
    Case { rows: &[187], pat: P::L(b"(?(DEFINE)a|b)"), expect: 154, ..DEF },
    Case { rows: &[188], pat: P::L(b"(?(VERSION<=10.0)a)"), expect: 179, ..DEF },
    Case { rows: &[188], pat: P::L(b"(?(VERSIONx)a)"), expect: 179, ..DEF },
    Case { rows: &[189], pat: P::L(b"(?(VERSION>=1001)a)"), expect: 179, ..DEF },
    Case { rows: &[190], pat: P::L(b"(?(VERSION>=10.x)a)"), expect: 179, ..DEF },
    Case { rows: &[191], pat: P::L(b"(?(VERSION>=10.1001)a)"), expect: 179, ..DEF },
    Case { rows: &[192], pat: P::L(b"(?(VERSION>=10.0x)a)"), expect: 179, ..DEF },
    Case { rows: &[193], pat: P::L(b"(?C1)"), xopts: PCRE2_EXTRA_NEVER_CALLOUT, expect: 203, ..DEF },
    Case { rows: &[194], pat: P::L(b"(?C256)"), expect: 138, ..DEF },
    Case { rows: &[195], pat: P::L(b"(?C1x"), expect: 139, ..DEF },
    Case { rows: &[195], pat: P::L(b"(?C{abc}x"), expect: 139, ..DEF },
    Case { rows: &[196], pat: P::L(b"(?Cxabc)"), expect: 182, ..DEF },
    Case { rows: &[197], pat: P::L(b"(?C{abc"), expect: 181, ..DEF },
    // ERR72 (CALLOUT_STRING_TOO_LONG) needs a callout string longer than UINT32_MAX code units,
    // i.e. a >4 GiB pattern
    Case { rows: &[198], pat: P::L(b"(?C{a})"), expect: 0, unreachable: true, ..DEF },
    Case { rows: &[199], pat: P::L(b"(?<=a*)b"), expect: 125, ..DEF },
    Case { rows: &[199], pat: P::L(b"(?<=(?R))b"), expect: 125, ..DEF },
    Case { rows: &[200], pat: P::L(b"(?<=\\X)b"), expect: 125, ..DEF },
    Case { rows: &[201], pat: P::G(gen_complicated_lookbehind), expect: 135, ..DEF },
    Case { rows: &[202], pat: P::L(b"(?<=\\C)a"), opts: PCRE2_UTF, expect: 136, ..DEF },
    Case { rows: &[203], pat: P::L(b"(?<=(?:a{65535}){65535})"), expect: 187, ..DEF },
    Case { rows: &[204], pat: P::L(b"(?<=a{65535}a{2})"), expect: 187, ..DEF },
    Case { rows: &[205], pat: P::L(b"(?<=a{1,256})b"), expect: 200, ..DEF },
    Case { rows: &[205], pat: P::L(b"(?<=a{1,3})b"), cfg: Cfg { varlookbehind: 2, ..DEFCFG }, expect: 200, ..DEF },
    // max == max_varlookbehind is legal
    Case { rows: &[205], pat: P::L(b"(?<=a{1,3})b"), cfg: Cfg { varlookbehind: 3, ..DEFCFG }, expect: 0, ..DEF },
    Case { rows: &[206], pat: P::L(b"(?<=\\k<xyz>)a"), expect: 115, ..DEF },
    Case { rows: &[206], pat: P::L(b"(?<=(?&xyz))a"), expect: 115, ..DEF },
    Case { rows: &[207], pat: P::L(b"(?<=\\2)a"), expect: 115, ..DEF },
    Case { rows: &[207], pat: P::L(b"(?<=(?2))a"), expect: 115, ..DEF },
    // ERR90 (INTERNAL_BAD_CODE_IN_SKIP) is parsed_skip()'s dead default: every META code that
    // can appear inside a lookbehind is handled
    Case { rows: &[208], pat: P::L(b"(?<=a)b"), expect: 0, unreachable: true, ..DEF },
    // ERR70 (INTERNAL_BAD_CODE_LOOKBEHINDS) is check_lookbehinds()'s dead default
    Case { rows: &[209], pat: P::L(b"(?<=a)b"), expect: 0, unreachable: true, ..DEF },
    // ERR52 (INTERNAL_OVERRAN_WORKSPACE) is guarded by the ERR86 check 4 lines above it, which
    // fires first; this is the input that fires that guard
    Case { rows: &[210], pat: P::G(gen_deep_capture_nesting), cfg: Cfg { parens_limit: 100000, ..DEFCFG }, expect: 186, unreachable: true, ..DEF },
    // each nesting level advances the pre-compile workspace pointer by sizeof(OP_CBRA)=5, so
    // ~1200 levels overrun COMPILE_WORK_SIZE-WORK_SIZE_SAFETY_MARGIN=5900; parens_nest_limit
    // must be raised first
    Case { rows: &[211], pat: P::G(gen_deep_capture_nesting), cfg: Cfg { parens_limit: 100000, ..DEFCFG }, expect: 186, ..DEF },
    Case { rows: &[212], pat: P::L(b"(?:a{65535}){65535}"), expect: 120, ..DEF },
    Case { rows: &[213], pat: P::G(gen_big_group_repeat), expect: 120, ..DEF },
    // ERR63 (INTERNAL_PARSED_OVERFLOW): the parsed_pattern buffer is sized from an
    // over-estimate, so the pre-write guards never fire
    Case { rows: &[214], pat: P::L(b"a"), expect: 0, unreachable: true, ..DEF },
    // ERR23 (INTERNAL_CODE_OVERFLOW): the second pass cannot emit more code than the
    // pre-compile pass measured
    Case { rows: &[215], pat: P::L(b"a"), expect: 0, unreachable: true, ..DEF },
    // ERR53 from the recursion-offset fixup: find_bracket cannot fail for an already-validated
    // group number
    Case { rows: &[216], pat: P::L(b"(a)(?1)"), expect: 0, unreachable: true, ..DEF },
    // ERR80 (INTERNAL_BAD_CODE_AUTO_POSSESS): auto_possessify only sees opcodes it generated
    Case { rows: &[217], pat: P::L(b"a+b"), expect: 0, unreachable: true, ..DEF },
    // ERR31 (INTERNAL_STUDY_ERROR): study() cannot fail on bytecode produced by this compiler
    Case { rows: &[218], pat: P::L(b"abc"), expect: 0, unreachable: true, ..DEF },
    // ERR89 (INTERNAL_BAD_CODE) from the compile switch's dead default
    Case { rows: &[219], pat: P::L(b"a"), expect: 0, unreachable: true, ..DEF },
    // ERR32 (UNICODE_NOT_SUPPORTED) needs a build without SUPPORT_UNICODE
    Case { rows: &[220], pat: P::L(b"(*UTF)a"), expect: 0, unreachable: true, ..DEF },
    // ERR59 (VERB_ARGUMENT_NOT_ALLOWED) is never assigned anywhere in the sources; the nearest
    // thing, a verb that may not take an argument, is silently accepted
    Case { rows: &[221], pat: P::L(b"(*ACCEPT:x)"), expect: 0, unreachable: true, ..DEF },
];

// ============================================================== the main test

unsafe fn run_case(p: &Pair, cs: &Case, d: &mut Diffs) {
    let pat = cs.pat.bytes();
    let tag = format!("rows {:?} {}", cs.rows, cs.pat.show());

    let cc = make_ctx(&p.c, &cs.cfg, cs.xopts);
    let rcx = make_ctx(&p.r, &cs.cfg, cs.xopts);

    let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
    let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
    let a = (p.c.compile)(pat.as_ptr(), pat.len(), cs.opts, &mut eca, &mut eoa, cc);
    let b = (p.r.compile)(pat.as_ptr(), pat.len(), cs.opts, &mut ecb, &mut eob, rcx);

    // 1. the three observables must agree between the two libraries
    d.eq(&format!("{tag}: NULL?"), a.is_null(), b.is_null());
    d.eq(&format!("{tag}: errorcode"), eca, ecb);
    d.eq(&format!("{tag}: erroroffset"), eoa, eob);

    // 2. the C must produce exactly the code ERRORS.md documents
    let observed = if a.is_null() { eca } else { 0 };
    d.eq(&format!("{tag}: C code vs ERRORS.md"), cs.expect, observed);

    // 3. when the input is legal, the two must also produce identical bytecode
    if !a.is_null() && !b.is_null() {
        assert_code_eq(a, b, &tag);
        d.checked += 1;
    }

    if !a.is_null() {
        (p.c.code_free)(a);
    }
    if !b.is_null() {
        (p.r.code_free)(b);
    }
    (p.c.compile_context_free)(cc);
    (p.r.compile_context_free)(rcx);
}

/// Option / extra-option overlays applied on top of every short row input.
/// Several of them change which branch the pattern reaches (for instance
/// `PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL` turns an unknown escape into a literal,
/// and `PCRE2_ALT_EXTENDED_CLASS` re-routes the whole class parser), so this is
/// a cheap way to reach the *neighbours* of each documented branch.  Only
/// C-vs-Rust agreement is asserted here -- the documented code belongs to the
/// base configuration.
const OVERLAYS: &[(u32, u32)] = &[
    (PCRE2_AUTO_CALLOUT, 0),
    (PCRE2_EXTENDED, 0),
    (PCRE2_EXTENDED_MORE, 0),
    (PCRE2_UTF, 0),
    (PCRE2_UCP, 0),
    (PCRE2_ALT_EXTENDED_CLASS, 0),
    (PCRE2_ALT_VERBNAMES, 0),
    (PCRE2_ALT_BSUX, 0),
    (PCRE2_DUPNAMES, 0),
    (PCRE2_NO_AUTO_CAPTURE, 0),
    (PCRE2_ALLOW_EMPTY_CLASS, 0),
    (PCRE2_CASELESS | PCRE2_UTF, 0),
    (0, PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL),
    (0, PCRE2_EXTRA_ALT_BSUX),
    (0, PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES),
    (0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
    (0, PCRE2_EXTRA_PYTHON_OCTAL),
    (0, PCRE2_EXTRA_NO_BS0),
    (0, PCRE2_EXTRA_ESCAPED_CR_IS_LF),
    (0, PCRE2_EXTRA_ASCII_POSIX),
    (0, PCRE2_EXTRA_NEVER_CALLOUT),
    (0, PCRE2_EXTRA_CASELESS_RESTRICT),
];

/// Same input, but with each overlay added and (where the pattern has no
/// embedded NUL) also as a zero-terminated string.
unsafe fn run_case_overlays(p: &Pair, cs: &Case, d: &mut Diffs) {
    let base = cs.pat.bytes();
    if base.len() > 200 {
        return; // the generated monsters: one configuration each is enough
    }
    let mut zt = base.clone();
    zt.push(0);
    let lens: &[Sz] = if base.contains(&0) {
        &[usize::MAX] // sentinel meaning "explicit length only"
    } else {
        &[usize::MAX, PCRE2_ZERO_TERMINATED]
    };
    for &(xo, xx) in OVERLAYS {
        for &len in lens {
            let (buf, plen): (&[u8], Sz) = if len == PCRE2_ZERO_TERMINATED {
                (&zt, PCRE2_ZERO_TERMINATED)
            } else {
                (&base, base.len())
            };
            let opts = cs.opts | xo;
            let xopts = cs.xopts | xx;
            let cc = make_ctx(&p.c, &cs.cfg, xopts);
            let rcx = make_ctx(&p.r, &cs.cfg, xopts);
            let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
            let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
            let a = (p.c.compile)(buf.as_ptr(), plen, opts, &mut eca, &mut eoa, cc);
            let b = (p.r.compile)(buf.as_ptr(), plen, opts, &mut ecb, &mut eob, rcx);
            let t = format!(
                "rows {:?} {} +opts{:#x} +xopts{:#x} zt={}",
                cs.rows,
                cs.pat.show(),
                xo,
                xx,
                plen == PCRE2_ZERO_TERMINATED
            );
            d.eq(&format!("{t}: NULL?"), a.is_null(), b.is_null());
            d.eq(&format!("{t}: errorcode"), eca, ecb);
            d.eq(&format!("{t}: erroroffset"), eoa, eob);
            if !a.is_null() && !b.is_null() {
                assert_code_eq(a, b, &t);
                d.checked += 1;
            }
            if !a.is_null() {
                (p.c.code_free)(a);
            }
            if !b.is_null() {
                (p.r.code_free)(b);
            }
            (p.c.compile_context_free)(cc);
            (p.r.compile_context_free)(rcx);
        }
    }
}

/// Every proper prefix of the row's pattern, at the row's own options.  A great
/// many rows are "... runs into the end of the pattern", and truncating each
/// input walks the parser into *every* end-of-pattern branch reachable from it,
/// including the ones whose `erroroffset` arithmetic is easiest to get wrong.
unsafe fn run_case_prefixes(p: &Pair, cs: &Case, d: &mut Diffs) {
    let base = cs.pat.bytes();
    if base.len() > 64 {
        return;
    }
    for n in 0..base.len() {
        let cc = make_ctx(&p.c, &cs.cfg, cs.xopts);
        let rcx = make_ctx(&p.r, &cs.cfg, cs.xopts);
        let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
        let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
        let a = (p.c.compile)(base.as_ptr(), n, cs.opts, &mut eca, &mut eoa, cc);
        let b = (p.r.compile)(base.as_ptr(), n, cs.opts, &mut ecb, &mut eob, rcx);
        let t = format!("rows {:?} {} prefix[{}]", cs.rows, cs.pat.show(), n);
        d.eq(&format!("{t}: NULL?"), a.is_null(), b.is_null());
        d.eq(&format!("{t}: errorcode"), eca, ecb);
        d.eq(&format!("{t}: erroroffset"), eoa, eob);
        if !a.is_null() && !b.is_null() {
            assert_code_eq(a, b, &t);
            d.checked += 1;
        }
        if !a.is_null() {
            (p.c.code_free)(a);
        }
        if !b.is_null() {
            (p.r.code_free)(b);
        }
        (p.c.compile_context_free)(cc);
        (p.r.compile_context_free)(rcx);
    }
}

// ERRORS.md rows 4..19 and 32..221: one exact invalid input per row.
#[test]
fn compile_error_rows_table() {
    let p = pair();
    let mut d = Diffs::new();
    let mut reachable = 0usize;
    for cs in CASES {
        unsafe {
            run_case(p, cs, &mut d);
            run_case_overlays(p, cs, &mut d);
            run_case_prefixes(p, cs, &mut d);
        }
        if !cs.unreachable {
            reachable += 1;
        }
    }
    println!(
        "phase_c table: {} cases ({} reachable), {} assertions",
        CASES.len(),
        reachable,
        d.checked
    );
    d.finish("ERRORS.md rows 4..19, 32..221 — pcre2_compile_8 error surface");
}

// Each row input, mutated one byte at a time (substitution and deletion) with
// the metacharacters the compiler cares about.  The row inputs sit right on the
// edge of dozens of error branches, so their immediate neighbourhood is the
// densest part of the whole error surface; only C-vs-Rust agreement is asserted.
#[test]
fn compile_error_row_mutations() {
    const ALPHA: &[u8] = b"\\([{*+?|-^$.:=<>'!&~]})#0123456789acdnpxCQENRKG,";
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        for cs in CASES {
            let base = cs.pat.bytes();
            if base.len() > 20 {
                continue;
            }
            let cc = make_ctx(&p.c, &cs.cfg, cs.xopts);
            let rcx = make_ctx(&p.r, &cs.cfg, cs.xopts);
            let mut mutants: Vec<Vec<u8>> = Vec::new();
            for i in 0..base.len() {
                let mut del = base.clone();
                del.remove(i);
                mutants.push(del);
                for &ch in ALPHA {
                    let mut sub = base.clone();
                    sub[i] = ch;
                    mutants.push(sub);
                }
            }
            for m in &mutants {
                let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
                let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
                let a = (p.c.compile)(m.as_ptr(), m.len(), cs.opts, &mut eca, &mut eoa, cc);
                let b = (p.r.compile)(m.as_ptr(), m.len(), cs.opts, &mut ecb, &mut eob, rcx);
                let t = format!("rows {:?} mutant {}", cs.rows, show(m));
                d.eq(&format!("{t}: NULL?"), a.is_null(), b.is_null());
                d.eq(&format!("{t}: errorcode"), eca, ecb);
                d.eq(&format!("{t}: erroroffset"), eoa, eob);
                if !a.is_null() && !b.is_null() {
                    assert_code_eq(a, b, &t);
                    d.checked += 1;
                }
                if !a.is_null() {
                    (p.c.code_free)(a);
                }
                if !b.is_null() {
                    (p.r.code_free)(b);
                }
            }
            (p.c.compile_context_free)(cc);
            (p.r.compile_context_free)(rcx);
        }
    }
    println!("mutation sweep: {} assertions", d.checked);
    d.finish("ERRORS.md rows 4..221 — single-byte mutations of every row input");
}

// ================================================ entry-point argument checks

// ERRORS.md rows 1, 2, 3: the NULL-pointer contracts of pcre2_compile_8.
#[test]
fn compile_null_argument_rows_1_2_3() {
    let p = pair();
    let mut d = Diffs::new();
    // A structure identical to the table's, so the coverage script sees the
    // rows these assertions belong to.
    struct NullCase {
        rows: &'static [u32],
        what: &'static str,
    }
    const NULL_CASES: &[NullCase] = &[
        NullCase { rows: &[1], what: "errorptr == NULL" },
        NullCase { rows: &[2], what: "erroroffset == NULL" },
        NullCase { rows: &[3], what: "pattern == NULL" },
    ];
    unsafe {
        for nc in NULL_CASES {
            match nc.rows[0] {
                // Row 1: no error code can be reported, but *erroroffset is
                // still zeroed when it is available (pcre2_compile.c:10340).
                1 => {
                    for pat in [&b"a"[..], &b"a\\"[..], &b"("[..]] {
                        let (mut eoa, mut eob) = (12345usize, 12345usize);
                        let a = (p.c.compile)(pat.as_ptr(), pat.len(), 0, ptr::null_mut(), &mut eoa, ptr::null_mut());
                        let b = (p.r.compile)(pat.as_ptr(), pat.len(), 0, ptr::null_mut(), &mut eob, ptr::null_mut());
                        d.eq(&format!("row1 {} {}: NULL?", nc.what, show(pat)), a.is_null(), b.is_null());
                        d.eq(&format!("row1 {} {}: erroroffset", nc.what, show(pat)), eoa, eob);
                        d.eq(&format!("row1 {} {}: C zeroed erroroffset", nc.what, show(pat)), 0usize, eoa);
                        assert!(a.is_null() && b.is_null(), "row1: compile must fail");
                        // ... and with erroroffset NULL as well: nothing at all
                        // can be written, both must simply return NULL.
                        let a2 = (p.c.compile)(pat.as_ptr(), pat.len(), 0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
                        let b2 = (p.r.compile)(pat.as_ptr(), pat.len(), 0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
                        d.eq(&format!("row1 both-NULL {}: NULL?", show(pat)), a2.is_null(), b2.is_null());
                        assert!(a2.is_null() && b2.is_null());
                    }
                }
                // Row 2: *errorptr = PCRE2_ERROR_NULL_EROROFFSET (220).
                2 => {
                    for pat in [&b"a"[..], &b"a\\"[..]] {
                        let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
                        let a = (p.c.compile)(pat.as_ptr(), pat.len(), 0, &mut eca, ptr::null_mut(), ptr::null_mut());
                        let b = (p.r.compile)(pat.as_ptr(), pat.len(), 0, &mut ecb, ptr::null_mut(), ptr::null_mut());
                        d.eq(&format!("row2 {}: NULL?", show(pat)), a.is_null(), b.is_null());
                        d.eq(&format!("row2 {}: errorcode", show(pat)), eca, ecb);
                        d.eq(&format!("row2 {}: C code vs ERRORS.md", show(pat)), 220, eca);
                    }
                }
                // Row 3: pattern == NULL is legal iff patlen == 0.
                _ => {
                    for &len in &[0usize, 1, 5, PCRE2_ZERO_TERMINATED] {
                        let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
                        let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
                        let a = (p.c.compile)(ptr::null(), len, 0, &mut eca, &mut eoa, ptr::null_mut());
                        let b = (p.r.compile)(ptr::null(), len, 0, &mut ecb, &mut eob, ptr::null_mut());
                        d.eq(&format!("row3 patlen={len}: NULL?"), a.is_null(), b.is_null());
                        d.eq(&format!("row3 patlen={len}: errorcode"), eca, ecb);
                        d.eq(&format!("row3 patlen={len}: erroroffset"), eoa, eob);
                        let want: c_int = if len == 0 { 0 } else { 116 };
                        d.eq(
                            &format!("row3 patlen={len}: C code vs ERRORS.md"),
                            want,
                            if a.is_null() { eca } else { 0 },
                        );
                        if len == 0 {
                            // NULL + zero length compiles as the empty pattern.
                            assert!(!a.is_null() && !b.is_null(), "row3: patlen==0 must be legal");
                            assert_code_eq(a, b, "row3 pattern==NULL patlen==0");
                            d.checked += 1;
                        } else {
                            d.eq(&format!("row3 patlen={len}: erroroffset is 0"), 0usize, eoa);
                        }
                        if !a.is_null() {
                            (p.c.code_free)(a);
                        }
                        if !b.is_null() {
                            (p.r.code_free)(b);
                        }
                    }
                }
            }
        }
    }
    d.finish("ERRORS.md rows 1..3 — errorptr/erroroffset/pattern NULL contracts");
}

// ERRORS.md rows 4..7: every single option and extra-option bit on its own,
// then the all-ones values, then every bit combined with PCRE2_LITERAL (which
// has its own, much smaller, permitted mask).
#[test]
fn compile_option_bit_sweep_rows_4_7() {
    let p = pair();
    let mut d = Diffs::new();
    struct BitCase {
        rows: &'static [u32],
        what: &'static str,
    }
    const BIT_CASES: &[BitCase] = &[
        BitCase { rows: &[4], what: "every options bit alone" },
        BitCase { rows: &[5], what: "every extra_options bit alone" },
        BitCase { rows: &[6], what: "every options bit with PCRE2_LITERAL" },
        BitCase { rows: &[7], what: "every extra_options bit with PCRE2_LITERAL" },
    ];
    // A NUL-terminated buffer so PCRE2_ZERO_TERMINATED can be used too.
    let pat: &[u8] = b"a\0";
    unsafe {
        for bc in BIT_CASES {
            for i in 0..=32u32 {
                let bit: u32 = if i == 32 { u32::MAX } else { 1u32 << i };
                let (opts, xopts) = match bc.rows[0] {
                    4 => (bit, 0),
                    5 => (0, bit),
                    6 => (bit | PCRE2_LITERAL, 0),
                    _ => (PCRE2_LITERAL, bit),
                };
                for &len in &[1usize, PCRE2_ZERO_TERMINATED] {
                    let cc = make_ctx(&p.c, &DEFCFG, xopts);
                    let rcx = make_ctx(&p.r, &DEFCFG, xopts);
                    let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
                    let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
                    let a = (p.c.compile)(pat.as_ptr(), len, opts, &mut eca, &mut eoa, cc);
                    let b = (p.r.compile)(pat.as_ptr(), len, opts, &mut ecb, &mut eob, rcx);
                    let t = format!("{} bit {:#x} len={:#x}", bc.what, bit, len);
                    d.eq(&format!("{t}: NULL?"), a.is_null(), b.is_null());
                    d.eq(&format!("{t}: errorcode"), eca, ecb);
                    d.eq(&format!("{t}: erroroffset"), eoa, eob);
                    if !a.is_null() && !b.is_null() {
                        assert_code_eq(a, b, &t);
                        d.checked += 1;
                    }
                    if !a.is_null() {
                        (p.c.code_free)(a);
                    }
                    if !b.is_null() {
                        (p.r.code_free)(b);
                    }
                    (p.c.compile_context_free)(cc);
                    (p.r.compile_context_free)(rcx);
                }
            }
        }
        // The two "all bits set" values, spelled out explicitly.
        for &(opts, xopts) in &[
            (0xFFFF_FFFFu32, 0u32),
            (0u32, 0xFFFF_FFFFu32),
            (0xFFFF_FFFFu32, 0xFFFF_FFFFu32),
        ] {
            let cc = make_ctx(&p.c, &DEFCFG, xopts);
            let rcx = make_ctx(&p.r, &DEFCFG, xopts);
            let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
            let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
            let a = (p.c.compile)(pat.as_ptr(), 1, opts, &mut eca, &mut eoa, cc);
            let b = (p.r.compile)(pat.as_ptr(), 1, opts, &mut ecb, &mut eob, rcx);
            let t = format!("all-ones opts={opts:#x} xopts={xopts:#x}");
            d.eq(&format!("{t}: NULL?"), a.is_null(), b.is_null());
            d.eq(&format!("{t}: errorcode"), eca, ecb);
            d.eq(&format!("{t}: erroroffset"), eoa, eob);
            d.eq(&format!("{t}: C rejects with ERR17"), 117, eca);
            assert!(a.is_null() && b.is_null());
            (p.c.compile_context_free)(cc);
            (p.r.compile_context_free)(rcx);
        }
    }
    d.finish("ERRORS.md rows 4..7 — all 32 option bits and all 32 extra-option bits");
}

// ERRORS.md row 19 plus the generic "out-of-range value handed to a
// compile-context setter" boundary: the setters must reject identically and the
// following compile must behave identically.
#[test]
fn compile_context_setter_out_of_range_row_19() {
    let p = pair();
    let mut d = Diffs::new();
    struct SetterCase {
        rows: &'static [u32],
        what: &'static str,
    }
    const SETTER_CASES: &[SetterCase] = &[SetterCase {
        rows: &[19],
        what: "out-of-range compile-context setter values",
    }];
    unsafe {
        for sc in SETTER_CASES {
            println!("setter boundary case for ERRORS.md rows {:?}", sc.rows);
            let cc = (p.c.compile_context_create)(ptr::null_mut());
            let rcx = (p.r.compile_context_create)(ptr::null_mut());
            // newline_convention is validated to 1..6 -- this is exactly why
            // ERR56 (INTERNAL_UNKNOWN_NEWLINE, row 19) cannot be reached.
            for v in [0u32, 1, 6, 7, 99, u32::MAX] {
                d.eq(
                    &format!("{} set_newline({v})", sc.what),
                    (p.c.set_newline)(cc, v),
                    (p.r.set_newline)(rcx, v),
                );
            }
            for v in [0u32, 1, 2, 3, 99, u32::MAX] {
                d.eq(
                    &format!("{} set_bsr({v})", sc.what),
                    (p.c.set_bsr)(cc, v),
                    (p.r.set_bsr)(rcx, v),
                );
            }
            for v in [0u32, 2, 63, 66, 67, 70, 1000, u32::MAX] {
                d.eq(
                    &format!("{} set_optimize({v})", sc.what),
                    (p.c.set_optimize)(cc, v),
                    (p.r.set_optimize)(rcx, v),
                );
            }
            for v in [0u32, 255, 256, 65535, 65536, u32::MAX] {
                d.eq(
                    &format!("{} set_max_varlookbehind({v})", sc.what),
                    (p.c.set_max_varlookbehind)(cc, v),
                    (p.r.set_max_varlookbehind)(rcx, v),
                );
            }
            for v in [0u32, 1, u32::MAX] {
                d.eq(
                    &format!("{} set_parens_nest_limit({v})", sc.what),
                    (p.c.set_parens_nest_limit)(cc, v),
                    (p.r.set_parens_nest_limit)(rcx, v),
                );
            }
            for v in [0usize, 1, usize::MAX] {
                d.eq(
                    &format!("{} set_max_pattern_length({v})", sc.what),
                    (p.c.set_max_pattern_length)(cc, v),
                    (p.r.set_max_pattern_length)(rcx, v),
                );
                d.eq(
                    &format!("{} set_max_pattern_compiled_length({v})", sc.what),
                    (p.c.set_max_pattern_compiled_length)(cc, v),
                    (p.r.set_max_pattern_compiled_length)(rcx, v),
                );
            }
            d.eq(
                &format!("{} set_compile_extra_options(all ones)", sc.what),
                (p.c.set_compile_extra_options)(cc, u32::MAX),
                (p.r.set_compile_extra_options)(rcx, u32::MAX),
            );
            d.eq(
                &format!("{} set_character_tables(NULL)", sc.what),
                (p.c.set_character_tables)(cc, ptr::null()),
                (p.r.set_character_tables)(rcx, ptr::null()),
            );
            d.eq(
                &format!("{} set_compile_recursion_guard(NULL)", sc.what),
                (p.c.set_compile_recursion_guard)(cc, None, ptr::null_mut()),
                (p.r.set_compile_recursion_guard)(rcx, None, ptr::null_mut()),
            );
            // Whatever survived the rejections, both contexts must now make
            // pcre2_compile behave identically.
            for pat in [&b"a(b)c"[..], &b"(?<=a{1,2})b"[..], &b"((((a))))"[..], &b"a\\"[..]] {
                let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
                let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
                let a = (p.c.compile)(pat.as_ptr(), pat.len(), 0, &mut eca, &mut eoa, cc);
                let b = (p.r.compile)(pat.as_ptr(), pat.len(), 0, &mut ecb, &mut eob, rcx);
                let t = format!("{} then compile {}", sc.what, show(pat));
                d.eq(&format!("{t}: NULL?"), a.is_null(), b.is_null());
                d.eq(&format!("{t}: errorcode"), eca, ecb);
                d.eq(&format!("{t}: erroroffset"), eoa, eob);
                if !a.is_null() && !b.is_null() {
                    assert_code_eq(a, b, &t);
                    d.checked += 1;
                }
                if !a.is_null() {
                    (p.c.code_free)(a);
                }
                if !b.is_null() {
                    (p.r.code_free)(b);
                }
            }
            (p.c.compile_context_free)(cc);
            (p.r.compile_context_free)(rcx);
        }
    }
    d.finish("ERRORS.md row 19 — newline_convention is unreachable out of range; setter boundaries");
}

// ==================================================== the compile recursion guard

// Fixed-size logs: the guard sequence must be identical, and a guard that fails
// on its Nth call must fail identically.
const GLOG_MAX: usize = 1024;
static mut GLOG_C: [u32; GLOG_MAX] = [0; GLOG_MAX];
static mut GLOG_R: [u32; GLOG_MAX] = [0; GLOG_MAX];
static mut GLEN_C: usize = 0;
static mut GLEN_R: usize = 0;
static mut GFAIL_C: i64 = -1; // -1 = never fail
static mut GFAIL_R: i64 = -1;

unsafe fn guard_body(log: &mut [u32; GLOG_MAX], len: &mut usize, fail_at: &mut i64, depth: u32) -> c_int {
    if *len < GLOG_MAX {
        log[*len] = depth;
    }
    *len += 1;
    if *fail_at >= 0 {
        if *fail_at == 0 {
            return 1;
        }
        *fail_at -= 1;
    }
    0
}

unsafe extern "C" fn logging_guard_c(depth: u32, _d: *mut c_void) -> c_int {
    guard_body(
        &mut *ptr::addr_of_mut!(GLOG_C),
        &mut *ptr::addr_of_mut!(GLEN_C),
        &mut *ptr::addr_of_mut!(GFAIL_C),
        depth,
    )
}

unsafe extern "C" fn logging_guard_r(depth: u32, _d: *mut c_void) -> c_int {
    guard_body(
        &mut *ptr::addr_of_mut!(GLOG_R),
        &mut *ptr::addr_of_mut!(GLEN_R),
        &mut *ptr::addr_of_mut!(GFAIL_R),
        depth,
    )
}

// ERRORS.md rows 18 and 132: the compile recursion guard is the only route to
// ERR33, and it shares its call site with the parens_nest_limit check (ERR19).
// The guard is called with the current nesting depth, so the *sequence* of
// depths is itself an observable the two libraries must agree on.
#[test]
fn compile_recursion_guard_rows_18_132() {
    let p = pair();
    let mut d = Diffs::new();
    struct GuardCase {
        rows: &'static [u32],
        pat: &'static [u8],
    }
    const GUARD_CASES: &[GuardCase] = &[
        GuardCase { rows: &[18], pat: b"(a)" },
        GuardCase { rows: &[18], pat: b"(a(b(c)))" },
        GuardCase { rows: &[18], pat: b"(?:a)(?i:b)(?<n>c)" },
        GuardCase { rows: &[18, 132], pat: b"((((((a))))))" },
        GuardCase { rows: &[18], pat: b"(?(?=a)b|c)(x)" },
        GuardCase { rows: &[18], pat: b"a" }, // no group: the guard is never called
    ];
    unsafe {
        for gc in GUARD_CASES {
            // fail_at == -1 records the full sequence; 0..12 makes the Nth call
            // reject, which must abort the compile at the same point in both.
            for fail_at in -1..12i64 {
                for &parens in &[0u32, 3, 250] {
                    GLEN_C = 0;
                    GLEN_R = 0;
                    GFAIL_C = fail_at;
                    GFAIL_R = fail_at;
                    let cc = (p.c.compile_context_create)(ptr::null_mut());
                    let rcx = (p.r.compile_context_create)(ptr::null_mut());
                    if parens != 0 {
                        (p.c.set_parens_nest_limit)(cc, parens);
                        (p.r.set_parens_nest_limit)(rcx, parens);
                    }
                    d.eq(
                        "set_compile_recursion_guard rc",
                        (p.c.set_compile_recursion_guard)(cc, Some(logging_guard_c), ptr::null_mut()),
                        (p.r.set_compile_recursion_guard)(rcx, Some(logging_guard_r), ptr::null_mut()),
                    );
                    let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
                    let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
                    let a = (p.c.compile)(gc.pat.as_ptr(), gc.pat.len(), 0, &mut eca, &mut eoa, cc);
                    let b = (p.r.compile)(gc.pat.as_ptr(), gc.pat.len(), 0, &mut ecb, &mut eob, rcx);
                    let t = format!(
                        "rows {:?} {} guard_fail_at={} parens={}",
                        gc.rows,
                        show(gc.pat),
                        fail_at,
                        parens
                    );
                    d.eq(&format!("{t}: NULL?"), a.is_null(), b.is_null());
                    d.eq(&format!("{t}: errorcode"), eca, ecb);
                    d.eq(&format!("{t}: erroroffset"), eoa, eob);
                    d.eq(&format!("{t}: guard call count"), GLEN_C, GLEN_R);
                    let n = GLEN_C.min(GLOG_MAX).min(GLEN_R);
                    d.eq(
                        &format!("{t}: guard depth sequence"),
                        (&(*ptr::addr_of!(GLOG_C)))[..n].to_vec(),
                        (&(*ptr::addr_of!(GLOG_R)))[..n].to_vec(),
                    );
                    // A rejecting guard must produce ERR33 whenever it was
                    // actually consulted.
                    if fail_at >= 0 && (fail_at as usize) < GLEN_C {
                        d.eq(&format!("{t}: C code vs ERRORS.md row 18"), 133, eca);
                    }
                    if !a.is_null() && !b.is_null() {
                        assert_code_eq(a, b, &t);
                        d.checked += 1;
                    }
                    if !a.is_null() {
                        (p.c.code_free)(a);
                    }
                    if !b.is_null() {
                        (p.r.code_free)(b);
                    }
                    (p.c.compile_context_free)(cc);
                    (p.r.compile_context_free)(rcx);
                    GFAIL_C = -1;
                    GFAIL_R = -1;
                }
            }
        }
    }
    d.finish("ERRORS.md rows 18, 132 — compile recursion guard depth sequence and rejection");
}

// ============================================== the fallible allocator

// The Nth malloc fails; -1 means "unlimited".  A SEPARATE counter per library
// so the two runs cannot interfere.
static mut BUDGET_C: i64 = -1;
static mut BUDGET_R: i64 = -1;

unsafe fn tracked_alloc(n: usize) -> *mut c_void {
    let sz = n.max(1) + 16;
    let l = std::alloc::Layout::from_size_align(sz, 16).unwrap();
    let q = std::alloc::alloc(l);
    if q.is_null() {
        return ptr::null_mut();
    }
    *(q as *mut usize) = sz;
    q.add(16) as *mut c_void
}

unsafe extern "C" fn tracked_free(q: *mut c_void, _d: *mut c_void) {
    if q.is_null() {
        return;
    }
    let base = (q as *mut u8).sub(16);
    let sz = *(base as *mut usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(sz, 16).unwrap());
}

unsafe fn budgeted(b: &mut i64, n: usize) -> *mut c_void {
    if *b == 0 {
        return ptr::null_mut();
    }
    if *b > 0 {
        *b -= 1;
    }
    tracked_alloc(n)
}

unsafe extern "C" fn fallible_malloc_c(n: usize, _d: *mut c_void) -> *mut c_void {
    budgeted(&mut *ptr::addr_of_mut!(BUDGET_C), n)
}

unsafe extern "C" fn fallible_malloc_r(n: usize, _d: *mut c_void) -> *mut c_void {
    budgeted(&mut *ptr::addr_of_mut!(BUDGET_R), n)
}

struct AllocCase {
    rows: &'static [u32],
    pat: P,
    opts: u32,
    xopts: u32,
    cfg: Cfg,
    /// How many of the swept budgets must report ERR21 (HEAP_FAILED = 121).
    /// 0 for the resource-exhaustion patterns, which fail before allocating.
    min_heap_failures: u32,
}

const ALLOCDEF: AllocCase = AllocCase {
    rows: &[],
    pat: P::L(b""),
    opts: 0,
    xopts: 0,
    cfg: DEFCFG,
    min_heap_failures: 1,
};

/// Row 25: more than PARSED_PATTERN_DEFAULT_SIZE (1024) parsed items, so the
/// parsed-pattern vector has to come from the heap.
fn gen_long_literal() -> Vec<u8> {
    std::iter::repeat(b'a').take(4000).collect()
}

/// Row 26: the groupinfo vector is only heap-allocated when there is a
/// lookbehind AND bracount >= GROUPINFO_DEFAULT_SIZE/2 (128).
fn gen_many_groups_with_lookbehind() -> Vec<u8> {
    let mut v = rep("(a)", 130);
    v.extend_from_slice(b"(?<=a)");
    v
}

/// Row 28: more than NAMED_GROUP_LIST_SIZE (20) named groups, so the named
/// group list has to be moved to the heap.
fn gen_25_named_groups() -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..25u32 {
        v.extend_from_slice(format!("(?<n{}>a)", i).as_bytes());
    }
    v
}

const ALLOC_CASES: &[AllocCase] = &[
    // Row 25 — heap parsed-pattern vector (pcre2_compile.c:10748).
    AllocCase { rows: &[25], pat: P::G(gen_long_literal), ..ALLOCDEF },
    // Row 26 — heap groupinfo vector (pcre2_compile.c:10779).
    AllocCase { rows: &[26], pat: P::G(gen_many_groups_with_lookbehind), ..ALLOCDEF },
    // Row 27 — the pcre2_real_code block itself (pcre2_compile.c:10881).
    AllocCase { rows: &[27], pat: P::L(b"abc"), ..ALLOCDEF },
    // Row 28 — enlarged named-group list (pcre2_compile.c:5772).
    AllocCase { rows: &[28], pat: P::G(gen_25_named_groups), ..ALLOCDEF },
    // Row 29 — compile_optimize_class (pcre2_compile_class.c:1127).
    AllocCase { rows: &[29], pat: P::L(b"[\\x{100}-\\x{200}]"), opts: PCRE2_UTF, ..ALLOCDEF },
    // Row 30 — the (*scs:...) capture bitmap (pcre2_compile_cgroup.c:384).
    AllocCase { rows: &[30], pat: P::L(b"(a)(*scs:(1)b)"), ..ALLOCDEF },
    // Row 31 — the recurse_arguments block (pcre2_compile_cgroup.c:531).
    AllocCase { rows: &[31], pat: P::L(b"(a)(?1(1))"), ..ALLOCDEF },
    // Rows 210/211 — the workspace-overrun guards, under allocation failure.
    AllocCase {
        rows: &[210, 211],
        pat: P::G(gen_deep_capture_nesting),
        cfg: Cfg { parens_limit: 100000, ..DEFCFG },
        min_heap_failures: 0,
        ..ALLOCDEF
    },
    // Rows 212/213 — the OFLOW_MAX / MAX_PATTERN_SIZE guards, under allocation
    // failure: these fail during the pre-compile pass, so on most budgets no
    // heap allocation is even attempted.
    AllocCase { rows: &[212], pat: P::L(b"(?:a{65535}){65535}"), min_heap_failures: 0, ..ALLOCDEF },
    AllocCase { rows: &[213], pat: P::G(gen_big_group_repeat), min_heap_failures: 0, ..ALLOCDEF },
];

// ERRORS.md rows 25..31 (and 210..213): sweep which allocation fails and demand
// that both libraries make the identical decision every time.  This also drives
// every `cleanup:` path in pcre2_compile.c.
#[test]
fn compile_allocation_failure_rows_25_31() {
    let p = pair();
    let mut d = Diffs::new();
    const SWEEP: i64 = 40;
    unsafe {
        for ac in ALLOC_CASES {
            let pat = ac.pat.bytes();
            let mut heap_failures = 0u32;
            for n in 0..=SWEEP {
                // Contexts are built with an unlimited budget so that only the
                // allocations pcre2_compile itself makes are counted.
                BUDGET_C = -1;
                BUDGET_R = -1;
                let gc = (p.c.general_context_create)(Some(fallible_malloc_c), Some(tracked_free), ptr::null_mut());
                let gr = (p.r.general_context_create)(Some(fallible_malloc_r), Some(tracked_free), ptr::null_mut());
                assert!(!gc.is_null() && !gr.is_null());
                let cc = (p.c.compile_context_create)(gc);
                let rcx = (p.r.compile_context_create)(gr);
                assert!(!cc.is_null() && !rcx.is_null());
                if ac.cfg.parens_limit != 0 {
                    (p.c.set_parens_nest_limit)(cc, ac.cfg.parens_limit);
                    (p.r.set_parens_nest_limit)(rcx, ac.cfg.parens_limit);
                }
                if ac.xopts != 0 {
                    (p.c.set_compile_extra_options)(cc, ac.xopts);
                    (p.r.set_compile_extra_options)(rcx, ac.xopts);
                }

                BUDGET_C = n;
                BUDGET_R = n;
                let (mut eca, mut ecb) = (12345 as c_int, 12345 as c_int);
                let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
                let a = (p.c.compile)(pat.as_ptr(), pat.len(), ac.opts, &mut eca, &mut eoa, cc);
                let b = (p.r.compile)(pat.as_ptr(), pat.len(), ac.opts, &mut ecb, &mut eob, rcx);
                BUDGET_C = -1;
                BUDGET_R = -1;

                let t = format!("rows {:?} budget={} {}", ac.rows, n, ac.pat.show());
                d.eq(&format!("{t}: NULL?"), a.is_null(), b.is_null());
                d.eq(&format!("{t}: errorcode"), eca, ecb);
                d.eq(&format!("{t}: erroroffset"), eoa, eob);
                if a.is_null() && eca == 121 {
                    heap_failures += 1;
                }
                if !a.is_null() && !b.is_null() {
                    assert_code_eq(a, b, &t);
                    d.checked += 1;
                }

                if !a.is_null() {
                    (p.c.code_free)(a);
                }
                if !b.is_null() {
                    (p.r.code_free)(b);
                }
                (p.c.compile_context_free)(cc);
                (p.r.compile_context_free)(rcx);
                (p.c.general_context_free)(gc);
                (p.r.general_context_free)(gr);
            }
            println!(
                "alloc sweep rows {:?}: {}/{} budgets gave ERR21 (121)",
                ac.rows,
                heap_failures,
                SWEEP + 1
            );
            assert!(
                heap_failures >= ac.min_heap_failures,
                "rows {:?}: expected at least {} HEAP_FAILED results from the 0..{} \
                 malloc-failure sweep, got {}",
                ac.rows,
                ac.min_heap_failures,
                SWEEP,
                heap_failures
            );
            d.checked += 1;
        }
    }
    d.finish("ERRORS.md rows 25..31, 210..213 — malloc-failure sweep");
}

// ERRORS.md rows 20..24: pcre2_code_copy_8 / pcre2_code_copy_with_tables_8 /
// pcre2_code_free_8 with a NULL argument, and with a failing allocator.
#[test]
fn code_copy_and_free_rows_20_24() {
    let p = pair();
    let mut d = Diffs::new();
    struct CopyCase {
        rows: &'static [u32],
        what: &'static str,
    }
    const COPY_CASES: &[CopyCase] = &[
        CopyCase { rows: &[20], what: "pcre2_code_copy_8(NULL)" },
        CopyCase { rows: &[21], what: "pcre2_code_copy_8 with failing malloc" },
        CopyCase { rows: &[22], what: "pcre2_code_copy_with_tables_8(NULL)" },
        CopyCase { rows: &[23], what: "pcre2_code_copy_with_tables_8 with failing malloc" },
        CopyCase { rows: &[24], what: "pcre2_code_free_8(NULL)" },
    ];
    unsafe {
        for cc in COPY_CASES {
            match cc.rows[0] {
                20 => {
                    let a = (p.c.code_copy)(ptr::null_mut());
                    let b = (p.r.code_copy)(ptr::null_mut());
                    d.eq(&format!("{}: NULL?", cc.what), a.is_null(), b.is_null());
                    d.eq(&format!("{}: C returns NULL", cc.what), true, a.is_null());
                }
                22 => {
                    let a = (p.c.code_copy_with_tables)(ptr::null_mut());
                    let b = (p.r.code_copy_with_tables)(ptr::null_mut());
                    d.eq(&format!("{}: NULL?", cc.what), a.is_null(), b.is_null());
                    d.eq(&format!("{}: C returns NULL", cc.what), true, a.is_null());
                }
                24 => {
                    // Guarded no-op in both: the only observable is not crashing.
                    (p.c.code_free)(ptr::null_mut());
                    (p.r.code_free)(ptr::null_mut());
                    d.eq(&format!("{}: survived", cc.what), true, true);
                }
                rowno => {
                    // Compile with the fallible allocator, then make the copy's
                    // own allocation fail.
                    for pat in [&b"abc"[..], &b"(?<n>a)(b)\\1"[..]] {
                        BUDGET_C = -1;
                        BUDGET_R = -1;
                        let gc = (p.c.general_context_create)(Some(fallible_malloc_c), Some(tracked_free), ptr::null_mut());
                        let gr = (p.r.general_context_create)(Some(fallible_malloc_r), Some(tracked_free), ptr::null_mut());
                        let ctxc = (p.c.compile_context_create)(gc);
                        let ctxr = (p.r.compile_context_create)(gr);
                        let (mut eca, mut ecb) = (0 as c_int, 0 as c_int);
                        let (mut eoa, mut eob) = (0usize, 0usize);
                        let ka = (p.c.compile)(pat.as_ptr(), pat.len(), 0, &mut eca, &mut eoa, ctxc);
                        let kb = (p.r.compile)(pat.as_ptr(), pat.len(), 0, &mut ecb, &mut eob, ctxr);
                        assert!(!ka.is_null() && !kb.is_null(), "setup compile must succeed");
                        BUDGET_C = 0;
                        BUDGET_R = 0;
                        let (a, b) = if rowno == 21 {
                            ((p.c.code_copy)(ka), (p.r.code_copy)(kb))
                        } else {
                            ((p.c.code_copy_with_tables)(ka), (p.r.code_copy_with_tables)(kb))
                        };
                        BUDGET_C = -1;
                        BUDGET_R = -1;
                        d.eq(&format!("{} {}: NULL?", cc.what, show(pat)), a.is_null(), b.is_null());
                        d.eq(&format!("{} {}: C returns NULL", cc.what, show(pat)), true, a.is_null());
                        if !a.is_null() {
                            (p.c.code_free)(a);
                        }
                        if !b.is_null() {
                            (p.r.code_free)(b);
                        }
                        // Sanity: with the budget restored the copy succeeds and
                        // is byte-identical in both libraries.
                        let a2 = if rowno == 21 { (p.c.code_copy)(ka) } else { (p.c.code_copy_with_tables)(ka) };
                        let b2 = if rowno == 21 { (p.r.code_copy)(kb) } else { (p.r.code_copy_with_tables)(kb) };
                        assert!(!a2.is_null() && !b2.is_null());
                        (p.c.code_free)(a2);
                        (p.r.code_free)(b2);
                        (p.c.code_free)(ka);
                        (p.r.code_free)(kb);
                        (p.c.compile_context_free)(ctxc);
                        (p.r.compile_context_free)(ctxr);
                        (p.c.general_context_free)(gc);
                        (p.r.general_context_free)(gr);
                    }
                }
            }
        }
    }
    d.finish("ERRORS.md rows 20..24 — code_copy / code_copy_with_tables / code_free");
}
