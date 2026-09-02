//! Translation of PART 5 (final part) of `pcre2_compile.c`: the public entry
//! point `pcre2_compile()` (C source lines 10279..end of file), plus the two
//! file-static helper functions that are only referenced from this part in the
//! split translation — `max_parsed_pattern()` (C source line 2979) and
//! `find_recurse()` (C source line 9124).
//!
//! This is the 8-bit build (`PCRE2_CODE_UNIT_WIDTH == 8`) with
//! `SUPPORT_UNICODE` enabled, `SUPPORT_JIT` off, `PCRE2_DEBUG` off, no EBCDIC,
//! and running on a 64-bit target (so `SIZEOFFSET == 2` and the `ptrdiff_t`
//! computations use `isize`). Consequently:
//!
//! * All `#ifdef PCRE2_DEBUG`, `DEBUG_SHOW_PARSED`, `DEBUG_SHOW_CAPTURES`,
//!   `DEBUG_CALL_PRINTINT` and Valgrind blocks are omitted.
//! * `#ifndef SUPPORT_UNICODE` (UTF/UCP lockout) is omitted; the
//!   `#ifdef SUPPORT_UNICODE` branches are kept.
//! * `SUPPORT_WIDE_CHARS` is defined (it equals `SUPPORT_UNICODE` in 8-bit
//!   mode), so the `char_lists_size` handling is kept.
//! * `PCRE2_CODE_UNIT_WIDTH == 8` branches are selected throughout.
//!
//! The C `goto` control flow is modelled with a single result variable `re`
//! plus a set of nested "error sink" closures replaced by structured blocks:
//! the body runs inside a `'exit` labelled block; error paths funnel through
//! the `HAD_*` sequences which are reproduced inline before breaking to the
//! common exit code.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::compile_h::*;
use crate::compile_local::*;
use crate::compile_tables::*;
use crate::consts::*;
use crate::internal::*;
// Disambiguate symbols defined in both consts and internal globs. The
// internal.rs versions (correctly typed as PCRE2_SIZE / c_int) are the ones we
// want.
use crate::internal::{BOOL, FALSE, PCRE2_UNSET, PCRE2_ZERO_TERMINATED};

use crate::compile_aux::{
    check_lookbehinds, compile_regex, find_firstassertedcu, is_anchored, is_startline,
};
use crate::compile_cgroup::_pcre2_compile_add_name_to_table8;
use crate::compile_parse::parse_regex;
use crate::context::_pcre2_default_compile_context_8;

/// `PCRE2_CODE_UNIT_WIDTH` for this build (8-bit).
const PCRE2_CODE_UNIT_WIDTH: u32 = 8;

// ---------------------------------------------------------------------------
// PUBLIC option masks (from pcre2_compile.c, lines 688..712)
// ---------------------------------------------------------------------------

const PUBLIC_LITERAL_COMPILE_OPTIONS: u32 = (PCRE2_ANCHORED
    | PCRE2_AUTO_CALLOUT
    | PCRE2_CASELESS
    | PCRE2_ENDANCHORED
    | PCRE2_FIRSTLINE
    | PCRE2_LITERAL
    | PCRE2_MATCH_INVALID_UTF
    | PCRE2_NO_START_OPTIMIZE
    | PCRE2_NO_UTF_CHECK
    | PCRE2_USE_OFFSET_LIMIT
    | PCRE2_UTF) as u32;

const PUBLIC_COMPILE_OPTIONS: u32 = PUBLIC_LITERAL_COMPILE_OPTIONS
    | (PCRE2_ALLOW_EMPTY_CLASS
        | PCRE2_ALT_BSUX
        | PCRE2_ALT_CIRCUMFLEX
        | PCRE2_ALT_VERBNAMES
        | PCRE2_DOLLAR_ENDONLY
        | PCRE2_DOTALL
        | PCRE2_DUPNAMES
        | PCRE2_EXTENDED
        | PCRE2_EXTENDED_MORE
        | PCRE2_MATCH_UNSET_BACKREF
        | PCRE2_MULTILINE
        | PCRE2_NEVER_BACKSLASH_C
        | PCRE2_NEVER_UCP
        | PCRE2_NEVER_UTF
        | PCRE2_NO_AUTO_CAPTURE
        | PCRE2_NO_AUTO_POSSESS
        | PCRE2_NO_DOTSTAR_ANCHOR
        | PCRE2_UCP
        | PCRE2_UNGREEDY
        | PCRE2_ALT_EXTENDED_CLASS) as u32;

const PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS: u32 = (PCRE2_EXTRA_MATCH_LINE
    | PCRE2_EXTRA_MATCH_WORD
    | PCRE2_EXTRA_CASELESS_RESTRICT
    | PCRE2_EXTRA_TURKISH_CASING) as u32;

const PUBLIC_COMPILE_EXTRA_OPTIONS: u32 = PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS
    | (PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES
        | PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL
        | PCRE2_EXTRA_ESCAPED_CR_IS_LF
        | PCRE2_EXTRA_ALT_BSUX
        | PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK
        | PCRE2_EXTRA_ASCII_BSD
        | PCRE2_EXTRA_ASCII_BSS
        | PCRE2_EXTRA_ASCII_BSW
        | PCRE2_EXTRA_ASCII_POSIX
        | PCRE2_EXTRA_ASCII_DIGIT
        | PCRE2_EXTRA_PYTHON_OCTAL
        | PCRE2_EXTRA_NO_BS0
        | PCRE2_EXTRA_NEVER_CALLOUT) as u32;

// ---------------------------------------------------------------------------
// ASCII CHAR_* constants used below (this build is not EBCDIC).
// ---------------------------------------------------------------------------
const CHAR_NUL: u32 = 0x00;
const CHAR_CR: u32 = 0x0d;
const CHAR_NL: u32 = 0x0a;
const CHAR_0: u32 = 0x30;
const CHAR_9: u32 = 0x39;
const CHAR_LEFT_PARENTHESIS: u8 = 0x28;
const CHAR_ASTERISK: u8 = 0x2a;
const CHAR_RIGHT_PARENTHESIS: u32 = 0x29;

// `NLTYPE_*` and `MAGIC_NUMBER` come in as `i64` from `consts`.
const NLTYPE_FIXED_U: u32 = NLTYPE_FIXED as u32;
const NLTYPE_ANY_U: u32 = NLTYPE_ANY as u32;
const NLTYPE_ANYCRLF_U: u32 = NLTYPE_ANYCRLF as u32;
const MAGIC_NUMBER_U: u32 = MAGIC_NUMBER as u32;

// `MAX_PATTERN_SIZE` in code units.
const MAX_PATTERN_SIZE: PCRE2_SIZE = MAX_PATTERN_SIZE_U as PCRE2_SIZE;

// Optimization flag values (defined as `i64` in consts).
const PCRE2_OPTIM_AUTO_POSSESS_U: u32 = PCRE2_OPTIM_AUTO_POSSESS as u32;
const PCRE2_OPTIM_DOTSTAR_ANCHOR_U: u32 = PCRE2_OPTIM_DOTSTAR_ANCHOR as u32;
const PCRE2_OPTIM_START_OPTIMIZE_U: u32 = PCRE2_OPTIM_START_OPTIMIZE as u32;

// `MAX_UTF_CODE_POINT` in the SUPPORT_UNICODE UCD checks.
const MAX_UTF_CODE_POINT: u32 = MAX_UTF_CODE_POINT_U;

// ---------------------------------------------------------------------------
// Small helpers / macro-equivalents
// ---------------------------------------------------------------------------

/// `IS_DIGIT(c)`.
#[inline(always)]
const fn IS_DIGIT(c: u32) -> bool {
    c >= CHAR_0 && c <= CHAR_9
}

/// `PRIV(strncmp_c8)` — compare a PCRE2 string with an 8-bit byte slice.
#[inline(always)]
unsafe fn strncmp_c8(p: PCRE2_SPTR, s: &[u8], len: usize) -> c_int {
    unsafe { crate::string_utils::_pcre2_strncmp_c8_8(p, s.as_ptr() as *const c_char, len) }
}

/// `PRIV(strlen)` — length of a zero-terminated PCRE2 string.
#[inline(always)]
unsafe fn strlen(p: PCRE2_SPTR) -> PCRE2_SIZE {
    unsafe { crate::string_utils::_pcre2_strlen_8(p) }
}

/// `PRIV(valid_utf)`.
#[inline(always)]
unsafe fn valid_utf(s: PCRE2_SPTR, len: PCRE2_SIZE, off: *mut PCRE2_SIZE) -> c_int {
    unsafe { crate::valid_utf::_pcre2_valid_utf_8(s, len, off) }
}

// ---------------------------------------------------------------------------
// max_parsed_pattern()  (C source line 2979)
// ---------------------------------------------------------------------------

/// Compute an upper bound on the number of `uint32_t` units required for the
/// parsed pattern vector.
///
/// This is the 8-bit build, so the `PCRE2_CODE_UNIT_WIDTH == 32` scan for
/// `big32count` is not compiled and there is no `big32count` contribution.
unsafe fn max_parsed_pattern(
    ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    _utf: BOOL,
    options: u32,
) -> isize {
    unsafe {
        let big32count: PCRE2_SIZE = 0;

        let mut parsed_size_needed: isize =
            ptrend.offset_from(ptr) + big32count as isize;

        // When PCRE2_AUTO_CALLOUT is set we have to assume a numerical callout
        // (4 elements) for each character.
        if (options & PCRE2_AUTO_CALLOUT as u32) != 0 {
            parsed_size_needed += ptrend.offset_from(ptr) * 4;
        }

        parsed_size_needed
    }
}

// ---------------------------------------------------------------------------
// find_recurse()  (C source line 9124)
// ---------------------------------------------------------------------------

/// Scan a compiled pattern for the next `OP_RECURSE`, returning a pointer to
/// it, or `NULL` at the end of the code.
unsafe fn find_recurse(mut code: *mut PCRE2_UCHAR, utf: BOOL) -> *mut PCRE2_UCHAR {
    unsafe {
        loop {
            let c = *code as u32;
            if c == OP_END {
                return ptr::null_mut();
            }
            if c == OP_RECURSE {
                return code;
            }

            // XCLASS/ECLASS/CALLOUT_STR have a stored length in the code.
            if c == OP_XCLASS || c == OP_ECLASS {
                code = code.add(GET(code, 1) as usize);
            } else if c == OP_CALLOUT_STR {
                code = code.add(GET(code, 1 + 2 * LINK_SIZE_U) as usize);
            } else {
                match c {
                    OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS
                    | OP_TYPEQUERY | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS
                    | OP_TYPEPOSQUERY => {
                        if *code.add(1) as u32 == OP_PROP || *code.add(1) as u32 == OP_NOTPROP {
                            code = code.add(2);
                        }
                    }

                    OP_TYPEPOSUPTO | OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT => {
                        if *code.add(1 + IMM2_SIZE_U) as u32 == OP_PROP
                            || *code.add(1 + IMM2_SIZE_U) as u32 == OP_NOTPROP
                        {
                            code = code.add(2);
                        }
                    }

                    OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                        code = code.add(*code.add(1) as usize);
                    }

                    _ => {}
                }

                // Add in the fixed length from the table.
                code = code.add(crate::tables::_pcre2_OP_lengths_8[c as usize] as usize);

                // MAYBE_UTF_MULTI: opcodes followed by a possibly multi-code-unit
                // character need the extra length skipping in UTF mode.
                if utf != FALSE {
                    match c {
                        OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT | OP_EXACTI
                        | OP_NOTEXACT | OP_NOTEXACTI | OP_UPTO | OP_UPTOI | OP_NOTUPTO
                        | OP_NOTUPTOI | OP_MINUPTO | OP_MINUPTOI | OP_NOTMINUPTO
                        | OP_NOTMINUPTOI | OP_POSUPTO | OP_POSUPTOI | OP_NOTPOSUPTO
                        | OP_NOTPOSUPTOI | OP_STAR | OP_STARI | OP_NOTSTAR | OP_NOTSTARI
                        | OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI
                        | OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR | OP_NOTPOSSTARI
                        | OP_PLUS | OP_PLUSI | OP_NOTPLUS | OP_NOTPLUSI | OP_MINPLUS
                        | OP_MINPLUSI | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_POSPLUS
                        | OP_POSPLUSI | OP_NOTPOSPLUS | OP_NOTPOSPLUSI | OP_QUERY
                        | OP_QUERYI | OP_NOTQUERY | OP_NOTQUERYI | OP_MINQUERY
                        | OP_MINQUERYI | OP_NOTMINQUERY | OP_NOTMINQUERYI | OP_POSQUERY
                        | OP_POSQUERYI | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                            if HAS_EXTRALEN(*code.offset(-1) as u32) {
                                code = code.add(GET_EXTRALEN(*code.offset(-1) as u32) as usize);
                            }
                        }

                        _ => {}
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RSCAN_CACHE_SIZE
// ---------------------------------------------------------------------------

const RSCAN_CACHE_SIZE: usize = 8;

// ---------------------------------------------------------------------------
// pcre2_compile()  (C source line 10279)
// ---------------------------------------------------------------------------

/// `pcre2_compile()` — compile a regular expression into an internal form.
///
/// Exported with its final linker name `pcre2_compile_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_8(
    pattern: PCRE2_SPTR,
    patlen: PCRE2_SIZE,
    options: u32,
    errorptr: *mut c_int,
    erroroffset: *mut PCRE2_SIZE,
    ccontext: *mut pcre2_real_compile_context,
) -> *mut pcre2_real_code {
    unsafe {
        let mut pattern = pattern;
        let mut patlen = patlen;
        let mut options = options;
        let mut ccontext = ccontext;

        let utf: BOOL; // Set TRUE for UTF mode
        let ucp: BOOL; // Set TRUE for UCP mode
        let mut has_lookbehind: BOOL = FALSE; // TRUE if a lookbehind is found
        let zero_terminated: BOOL; // TRUE for zero-terminated pattern
        let mut re: *mut pcre2_real_code = ptr::null_mut(); // What we will return
        let mut cb: compile_block = core::mem::zeroed(); // "Static" compile-time data
        let tables: *const u8; // Char tables base pointer

        let mut null_str: [PCRE2_UCHAR; 1] = [0xcd]; // Dummy for handling null inputs
        let mut code: *mut PCRE2_UCHAR; // Current pointer in compiled code
        let mut codestart: *mut PCRE2_UCHAR; // Start of compiled code
        let mut ptr: PCRE2_SPTR; // Current pointer in pattern
        let mut pptr: *mut u32; // Current pointer in parsed pattern

        let mut length: PCRE2_SIZE = 1; // Allow for final END opcode
        let usedlength: PCRE2_SIZE; // Actual length used
        let mut re_blocksize: PCRE2_SIZE; // Size of memory block
        let parsed_size_needed: PCRE2_SIZE; // Needed for parsed pattern

        let mut firstcuflags: u32 = 0;
        let mut reqcuflags: u32 = 0; // Type of first/req code unit
        let mut firstcu: u32 = 0;
        let mut reqcu: u32 = 0; // Value of first/req code unit
        let mut setflags: u32 = 0; // NL and BSR set flags
        let mut xoptions: u32; // Flags from context, modified

        let mut skipatstart: u32; // When checking (*UTF) etc
        let mut limit_heap: u32 = u32::MAX;
        let mut limit_match: u32 = u32::MAX; // Unset match limits
        let mut limit_depth: u32 = u32::MAX;

        let mut newline: c_int = 0; // Unset; can be set by the pattern
        let mut bsr: c_int = 0; // Unset; can be set by the pattern
        let mut errorcode: c_int = 0; // Initialize to avoid compiler warn
        let regexrc: c_int; // Return from compile

        let mut i: u32; // Local loop counter

        // Enable all optimizations by default.
        let mut optim_flags: u32 = if !ccontext.is_null() {
            (*ccontext).optimization_flags
        } else {
            PCRE2_OPTIMIZATION_ALL as u32
        };

        let mut stack_groupinfo: [u32; GROUPINFO_DEFAULT_SIZE] = [0; GROUPINFO_DEFAULT_SIZE];
        let mut stack_parsed_pattern: [u32; PARSED_PATTERN_DEFAULT_SIZE] =
            [0; PARSED_PATTERN_DEFAULT_SIZE];
        let mut named_groups: [named_group; NAMED_GROUP_LIST_SIZE] =
            core::mem::zeroed();

        // The workspace is used in different ways in the different compiling
        // phases. It needs to be 16-bit aligned for the preliminary parsing
        // scan.
        let mut c16workspace: [u16; C16_WORK_SIZE] = [0; C16_WORK_SIZE];
        let cworkspace: *mut PCRE2_UCHAR = c16workspace.as_mut_ptr() as *mut PCRE2_UCHAR;

        // -------------- Check arguments and set up the pattern -----------------

        // There must be error code and offset pointers.
        if errorptr.is_null() {
            if !erroroffset.is_null() {
                *erroroffset = 0;
            }
            return ptr::null_mut();
        }
        if erroroffset.is_null() {
            if !errorptr.is_null() {
                *errorptr = ERR120;
            }
            return ptr::null_mut();
        }
        *errorptr = ERR0;
        *erroroffset = 0;

        // There must be a pattern, but NULL is allowed with zero length.
        if pattern.is_null() {
            if patlen == 0 {
                pattern = null_str.as_mut_ptr();
            } else {
                *errorptr = ERR16;
                return ptr::null_mut();
            }
        }

        // A NULL compile context means "use a default context"
        if ccontext.is_null() {
            ccontext =
                &raw mut _pcre2_default_compile_context_8 as *mut pcre2_real_compile_context;
        }

        // PCRE2_MATCH_INVALID_UTF implies UTF
        if (options & PCRE2_MATCH_INVALID_UTF as u32) != 0 {
            options |= PCRE2_UTF as u32;
        }

        // Check that all undefined public option bits are zero.
        if (options & !PUBLIC_COMPILE_OPTIONS) != 0
            || ((*ccontext).extra_options & !PUBLIC_COMPILE_EXTRA_OPTIONS) != 0
        {
            *errorptr = ERR17;
            return ptr::null_mut();
        }

        if (options & PCRE2_LITERAL as u32) != 0
            && ((options & !PUBLIC_LITERAL_COMPILE_OPTIONS) != 0
                || ((*ccontext).extra_options & !PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS) != 0)
        {
            *errorptr = ERR92;
            return ptr::null_mut();
        }

        // A zero-terminated pattern is indicated by the special length value
        // PCRE2_ZERO_TERMINATED. Check for an overlong pattern.
        zero_terminated = (patlen == PCRE2_ZERO_TERMINATED as PCRE2_SIZE) as BOOL;
        if zero_terminated != FALSE {
            patlen = strlen(pattern);
        }
        let _ = zero_terminated; // Silence compiler; only used if Valgrind enabled

        if patlen > (*ccontext).max_pattern_length {
            *errorptr = ERR88;
            return ptr::null_mut();
        }

        // Optimization flags in 'options' can override those in the compile
        // context.
        if (options & PCRE2_NO_AUTO_POSSESS as u32) != 0 {
            optim_flags &= !PCRE2_OPTIM_AUTO_POSSESS_U;
        }
        if (options & PCRE2_NO_DOTSTAR_ANCHOR as u32) != 0 {
            optim_flags &= !PCRE2_OPTIM_DOTSTAR_ANCHOR_U;
        }
        if (options & PCRE2_NO_START_OPTIMIZE as u32) != 0 {
            optim_flags &= !PCRE2_OPTIM_START_OPTIMIZE_U;
        }

        // From here on, all returns from this function should end up going via
        // the EXIT label.

        // ------------ Initialize the "static" compile data --------------
        tables = if !(*ccontext).tables.is_null() {
            (*ccontext).tables
        } else {
            crate::tables::_pcre2_default_tables_8.as_ptr()
        };

        cb.lcc = tables.add(lcc_offset as usize); // Individual
        cb.fcc = tables.add(fcc_offset as usize); //   character
        cb.cbits = tables.add(cbits_offset as usize); //      tables
        cb.ctypes = tables.add(ctypes_offset as usize);

        cb.assert_depth = 0;
        cb.bracount = 0;
        cb.cx = ccontext;
        cb.dupnames = FALSE;
        cb.end_pattern = pattern.add(patlen);
        cb.erroroffset = 0;
        cb.external_flags = 0;
        cb.external_options = options;
        cb.groupinfo = stack_groupinfo.as_mut_ptr();
        cb.had_recurse = FALSE;
        cb.lastcapture = 0;
        cb.max_lookbehind = 0; // Max encountered
        cb.max_varlookbehind = (*ccontext).max_varlookbehind; // Limit
        cb.name_entry_size = 0;
        cb.name_table = ptr::null_mut();
        cb.named_groups = named_groups.as_mut_ptr();
        cb.named_group_list_size = NAMED_GROUP_LIST_SIZE as u32;
        cb.names_found = 0;
        cb.parens_depth = 0;
        cb.parsed_pattern = stack_parsed_pattern.as_mut_ptr();
        cb.req_varyopt = 0;
        cb.start_code = cworkspace;
        cb.start_pattern = pattern;
        cb.start_workspace = cworkspace;
        cb.workspace_size = COMPILE_WORK_SIZE as PCRE2_SIZE;
        cb.first_data = ptr::null_mut();
        cb.last_data = ptr::null_mut();
        // SUPPORT_WIDE_CHARS
        cb.char_lists_size = 0;

        // Maximum back reference and backref bitmap.
        cb.top_backref = 0;
        cb.backref_map = 0;

        // Escape sequences \1 to \9: reset the small_ref_offset vector.
        i = 0;
        while i < 10 {
            cb.small_ref_offset[i as usize] = PCRE2_UNSET as PCRE2_SIZE;
            i += 1;
        }

        // --------------- Start looking at the pattern ---------------
        xoptions = (*ccontext).extra_options;
        ptr = pattern;
        skipatstart = 0;

        if (options & PCRE2_LITERAL as u32) == 0 {
            'pso_loop: while patlen - skipatstart as PCRE2_SIZE >= 2
                && *ptr.add(skipatstart as usize) == CHAR_LEFT_PARENTHESIS
                && *ptr.add(skipatstart as usize + 1) == CHAR_ASTERISK
            {
                i = 0;
                let n_pso = PSO_LIST.len() as u32;
                while i < n_pso {
                    let p = &PSO_LIST[i as usize];

                    if patlen - skipatstart as PCRE2_SIZE - 2 >= p.length as PCRE2_SIZE
                        && strncmp_c8(
                            ptr.add(skipatstart as usize + 2),
                            p.name,
                            p.length as usize,
                        ) == 0
                    {
                        let mut c: u32;
                        let mut pp: u32;

                        skipatstart += p.length as u32 + 2;
                        match p.type_ {
                            x if x == PSO_OPT => {
                                cb.external_options |= p.value;
                            }

                            x if x == PSO_XOPT => {
                                xoptions |= p.value;
                            }

                            x if x == PSO_FLG => {
                                setflags |= p.value;
                            }

                            x if x == PSO_NL => {
                                newline = p.value as c_int;
                                setflags |= PCRE2_NL_SET as u32;
                            }

                            x if x == PSO_BSR => {
                                bsr = p.value as c_int;
                                setflags |= PCRE2_BSR_SET as u32;
                            }

                            x if x == PSO_LIMM || x == PSO_LIMD || x == PSO_LIMH => {
                                c = 0;
                                pp = skipatstart;
                                while (pp as PCRE2_SIZE) < patlen
                                    && IS_DIGIT(*ptr.add(pp as usize) as u32)
                                {
                                    if c > u32::MAX / 10 - 1 {
                                        break; // Integer overflow
                                    }
                                    c = c * 10 + (*ptr.add(pp as usize) as u32 - CHAR_0);
                                    pp += 1;
                                }
                                if pp as PCRE2_SIZE >= patlen
                                    || pp == skipatstart
                                    || *ptr.add(pp as usize) as u32 != CHAR_RIGHT_PARENTHESIS
                                {
                                    errorcode = ERR60;
                                    ptr = ptr.add(pp as usize);
                                    // utf is used by HAD_EARLY_ERROR.
                                    return compile_had_early_error(
                                        errorcode, ptr, pattern, patlen, erroroffset, errorptr,
                                        FALSE,
                                    );
                                }
                                if p.type_ == PSO_LIMH {
                                    limit_heap = c;
                                } else if p.type_ == PSO_LIMM {
                                    limit_match = c;
                                } else {
                                    limit_depth = c;
                                }
                                pp += 1;
                                skipatstart = pp;
                            }

                            x if x == PSO_OPTMZ => {
                                optim_flags &= !(p.value);

                                // For backward compatibility the three original
                                // VERBs to disable optimizations also update the
                                // corresponding bit in the external options.
                                match p.value {
                                    v if v == PCRE2_OPTIM_AUTO_POSSESS_U => {
                                        cb.external_options |= PCRE2_NO_AUTO_POSSESS as u32;
                                    }
                                    v if v == PCRE2_OPTIM_DOTSTAR_ANCHOR_U => {
                                        cb.external_options |= PCRE2_NO_DOTSTAR_ANCHOR as u32;
                                    }
                                    v if v == PCRE2_OPTIM_START_OPTIMIZE_U => {
                                        cb.external_options |= PCRE2_NO_START_OPTIMIZE as u32;
                                    }
                                    _ => {}
                                }
                            }

                            _ => {
                                // Unreachable in a correct build.
                            }
                        }
                        break; // Out of the table scan loop
                    }

                    i += 1;
                }
                if i >= n_pso {
                    break 'pso_loop; // Out of pso loop
                }
            }
            // PCRE2_ASSERT(skipatstart <= patlen);
        }

        // End of pattern-start options; advance to start of real regex.
        ptr = ptr.add(skipatstart as usize);

        // (SUPPORT_UNICODE is defined, so the UTF/UCP lockout block is omitted.)

        // Check UTF.
        utf = ((cb.external_options & PCRE2_UTF as u32) != 0) as BOOL;
        if utf != FALSE {
            if (options & PCRE2_NEVER_UTF as u32) != 0 {
                errorcode = ERR74;
                return compile_had_early_error(
                    errorcode, ptr, pattern, patlen, erroroffset, errorptr, utf,
                );
            }
            if (options & PCRE2_NO_UTF_CHECK as u32) == 0 && {
                errorcode = valid_utf(pattern, patlen, erroroffset);
                errorcode != 0
            } {
                // Offset was set by valid_utf(); go straight to HAD_ERROR.
                return compile_had_error(
                    errorcode, &mut re, &mut cb, errorptr,
                );
            }
            // PCRE2_CODE_UNIT_WIDTH == 8, so the surrogate-escape check is
            // omitted.
        }

        // Check UCP lockout.
        ucp = ((cb.external_options & PCRE2_UCP as u32) != 0) as BOOL;
        if ucp != FALSE && (cb.external_options & PCRE2_NEVER_UCP as u32) != 0 {
            errorcode = ERR75;
            return compile_had_early_error(
                errorcode, ptr, pattern, patlen, erroroffset, errorptr, utf,
            );
        }

        // PCRE2_EXTRA_TURKISH_CASING checks
        if (xoptions & PCRE2_EXTRA_TURKISH_CASING as u32) != 0 {
            if utf == FALSE && ucp == FALSE {
                errorcode = ERR104;
                return compile_had_early_error(
                    errorcode, ptr, pattern, patlen, erroroffset, errorptr, utf,
                );
            }

            // PCRE2_CODE_UNIT_WIDTH == 8
            if utf == FALSE {
                errorcode = ERR105;
                return compile_had_early_error(
                    errorcode, ptr, pattern, patlen, erroroffset, errorptr, utf,
                );
            }

            if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT as u32) != 0 {
                errorcode = ERR106;
                return compile_had_early_error(
                    errorcode, ptr, pattern, patlen, erroroffset, errorptr, utf,
                );
            }
        }

        // Process the BSR setting.
        if bsr == 0 {
            bsr = (*ccontext).bsr_convention as c_int;
        }

        // Process the newline setting.
        if newline == 0 {
            newline = (*ccontext).newline_convention as c_int;
        }
        cb.nltype = NLTYPE_FIXED_U;
        match newline as i64 {
            x if x == PCRE2_NEWLINE_CR => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_CR as PCRE2_UCHAR;
            }
            x if x == PCRE2_NEWLINE_LF => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_NL as PCRE2_UCHAR;
            }
            x if x == PCRE2_NEWLINE_NUL => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_NUL as PCRE2_UCHAR;
            }
            x if x == PCRE2_NEWLINE_CRLF => {
                cb.nllen = 2;
                cb.nl[0] = CHAR_CR as PCRE2_UCHAR;
                cb.nl[1] = CHAR_NL as PCRE2_UCHAR;
            }
            x if x == PCRE2_NEWLINE_ANY => {
                cb.nltype = NLTYPE_ANY_U;
            }
            x if x == PCRE2_NEWLINE_ANYCRLF => {
                cb.nltype = NLTYPE_ANYCRLF_U;
            }
            _ => {
                errorcode = ERR56;
                return compile_had_early_error(
                    errorcode, ptr, pattern, patlen, erroroffset, errorptr, utf,
                );
            }
        }

        // Pre-scan the pattern.
        //
        // Ensure that the parsed pattern buffer is big enough.
        parsed_size_needed =
            max_parsed_pattern(ptr, cb.end_pattern, utf, options) as PCRE2_SIZE;

        // Allow for 2x uint32_t at the start and 2 at the end, for
        // PCRE2_EXTRA_MATCH_WORD or PCRE2_EXTRA_MATCH_LINE (exclusive).
        let mut parsed_size_needed = parsed_size_needed;
        if ((*ccontext).extra_options
            & (PCRE2_EXTRA_MATCH_WORD as u32 | PCRE2_EXTRA_MATCH_LINE as u32))
            != 0
        {
            parsed_size_needed += 4;
        }

        // When PCRE2_AUTO_CALLOUT is set we allow for one callout at the end.
        if (options & PCRE2_AUTO_CALLOUT as u32) != 0 {
            parsed_size_needed += 4;
        }

        parsed_size_needed += 1; // For the final META_END

        if parsed_size_needed > PARSED_PATTERN_DEFAULT_SIZE as PCRE2_SIZE {
            let heap_parsed_pattern = ((*ccontext).memctl.malloc.unwrap())(
                parsed_size_needed * core::mem::size_of::<u32>() as PCRE2_SIZE,
                (*ccontext).memctl.memory_data,
            ) as *mut u32;
            if heap_parsed_pattern.is_null() {
                *errorptr = ERR21;
                return compile_exit(re, &mut cb, ccontext, &stack_parsed_pattern, &stack_groupinfo);
            }
            cb.parsed_pattern = heap_parsed_pattern;
        }
        cb.parsed_pattern_end = cb.parsed_pattern.add(parsed_size_needed as usize);

        // Do the parsing scan.
        errorcode = parse_regex(
            ptr,
            cb.external_options,
            xoptions,
            &mut has_lookbehind,
            &mut cb,
        );
        if errorcode != 0 {
            return compile_had_cb_error(
                errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset, errorptr,
                utf, &stack_parsed_pattern, &stack_groupinfo,
            );
        }

        // If there are any lookbehinds, scan the parsed pattern to figure out
        // their lengths.
        if has_lookbehind != FALSE {
            let mut loopcount: c_int = 0;
            if cb.bracount >= (GROUPINFO_DEFAULT_SIZE / 2) as u32 {
                cb.groupinfo = ((*ccontext).memctl.malloc.unwrap())(
                    (2 * (cb.bracount as PCRE2_SIZE + 1))
                        * core::mem::size_of::<u32>() as PCRE2_SIZE,
                    (*ccontext).memctl.memory_data,
                ) as *mut u32;
                if cb.groupinfo.is_null() {
                    errorcode = ERR21;
                    cb.erroroffset = 0;
                    return compile_had_cb_error(
                        errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset,
                        errorptr, utf, &stack_parsed_pattern, &stack_groupinfo,
                    );
                }
            }
            ptr::write_bytes(
                cb.groupinfo,
                0,
                2 * cb.bracount as usize + 1,
            );
            errorcode = check_lookbehinds(
                cb.parsed_pattern,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut cb,
                &mut loopcount,
            );
            if errorcode != 0 {
                return compile_had_cb_error(
                    errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset, errorptr,
                    utf, &stack_parsed_pattern, &stack_groupinfo,
                );
            }
        }

        // Pretend to compile the pattern while actually just accumulating the
        // amount of memory required in the 'length' variable.
        cb.erroroffset = patlen; // For any subsequent errors that do not set it
        pptr = cb.parsed_pattern;
        code = cworkspace;
        *code = OP_BRA as PCRE2_UCHAR;

        let _ = compile_regex(
            cb.external_options,
            xoptions,
            &mut code,
            &mut pptr,
            &mut errorcode,
            0,
            &mut firstcu,
            &mut firstcuflags,
            &mut reqcu,
            &mut reqcuflags,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut cb,
            &mut length,
        );

        if errorcode != 0 {
            return compile_had_cb_error(
                errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset, errorptr,
                utf, &stack_parsed_pattern, &stack_groupinfo,
            ); // Offset is in cb.erroroffset
        }

        // This should be caught in compile_regex(), but just in case...
        // (SUPPORT_WIDE_CHARS is defined.)
        // PCRE2_ASSERT((cb.char_lists_size & 0x3) == 0);
        if length > MAX_PATTERN_SIZE
            || MAX_PATTERN_SIZE - length
                < (cb.char_lists_size / core::mem::size_of::<PCRE2_UCHAR>()) as PCRE2_SIZE
        {
            errorcode = ERR20;
            cb.erroroffset = 0;
            return compile_had_cb_error(
                errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset, errorptr,
                utf, &stack_parsed_pattern, &stack_groupinfo,
            );
        }

        // Compute the size of the data block for storing the compiled pattern
        // and names table.
        re_blocksize = CU2BYTES(cb.names_found as usize * cb.name_entry_size as usize)
            as PCRE2_SIZE;

        // SUPPORT_WIDE_CHARS
        if cb.char_lists_size != 0 {
            // PCRE2_CODE_UNIT_WIDTH != 32: align to 32-bit first.
            re_blocksize = CLIST_ALIGN_TO(
                re_blocksize as usize,
                core::mem::size_of::<u32>(),
            ) as PCRE2_SIZE;
            re_blocksize += cb.char_lists_size as PCRE2_SIZE;
        }

        re_blocksize += CU2BYTES(length as usize) as PCRE2_SIZE;

        if re_blocksize > (*ccontext).max_pattern_compiled_length {
            errorcode = ERR101;
            cb.erroroffset = 0;
            return compile_had_cb_error(
                errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset, errorptr,
                utf, &stack_parsed_pattern, &stack_groupinfo,
            );
        }

        re_blocksize += core::mem::size_of::<pcre2_real_code>() as PCRE2_SIZE;
        re = ((*ccontext).memctl.malloc.unwrap())(re_blocksize, (*ccontext).memctl.memory_data)
            as *mut pcre2_real_code;
        if re.is_null() {
            errorcode = ERR21;
            cb.erroroffset = 0;
            return compile_had_cb_error(
                errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset, errorptr,
                utf, &stack_parsed_pattern, &stack_groupinfo,
            );
        }

        // Explicitly zero the last 8 bytes of the structure to avoid reading
        // undefined padding when the pattern is copied/serialized.
        ptr::write_bytes(
            (re as *mut u8).add(core::mem::size_of::<pcre2_real_code>() - 8),
            0,
            8,
        );
        (*re).memctl = (*ccontext).memctl;
        (*re).tables = tables;
        (*re).executable_jit = ptr::null_mut();
        ptr::write_bytes((*re).start_bitmap.as_mut_ptr(), 0, 32);
        (*re).blocksize = re_blocksize;
        (*re).code_start = re_blocksize - CU2BYTES(length as usize) as PCRE2_SIZE;
        (*re).magic_number = MAGIC_NUMBER_U;
        (*re).compile_options = options;
        (*re).overall_options = cb.external_options;
        (*re).extra_options = xoptions;
        (*re).flags = (PCRE2_CODE_UNIT_WIDTH / 8) as u32 | cb.external_flags | setflags;
        (*re).limit_heap = limit_heap;
        (*re).limit_match = limit_match;
        (*re).limit_depth = limit_depth;
        (*re).first_codeunit = 0;
        (*re).last_codeunit = 0;
        (*re).bsr_convention = bsr as u16;
        (*re).newline_convention = newline as u16;
        (*re).max_lookbehind = 0;
        (*re).minlength = 0;
        (*re).top_bracket = 0;
        (*re).top_backref = 0;
        (*re).name_entry_size = cb.name_entry_size;
        (*re).name_count = cb.names_found;
        (*re).optimization_flags = optim_flags;

        // The basic block is immediately followed by the name table, and the
        // compiled code follows after that.
        codestart = (re as *mut u8).add((*re).code_start as usize) as *mut PCRE2_UCHAR;

        // Update the compile data block for the actual compile.
        cb.parens_depth = 0;
        cb.assert_depth = 0;
        cb.lastcapture = 0;
        cb.name_table =
            (re as *mut u8).add(core::mem::size_of::<pcre2_real_code>()) as *mut PCRE2_UCHAR;
        cb.start_code = codestart;
        cb.req_varyopt = 0;
        cb.had_accept = FALSE;
        cb.had_pruneorskip = FALSE;
        // SUPPORT_WIDE_CHARS
        cb.char_lists_size = 0;

        // If any named groups were found, create the name/number table from the
        // list created in the pre-pass.
        if cb.names_found > 0 {
            let mut ng = cb.named_groups;
            let mut tablecount: u32 = 0;

            // Length 0 represents duplicates, which have already been handled.
            i = 0;
            while i < cb.names_found as u32 {
                if (*ng).length > 0 {
                    tablecount =
                        _pcre2_compile_add_name_to_table8(&mut cb, ng, tablecount);
                }
                ng = ng.add(1);
                i += 1;
            }

            // PCRE2_ASSERT(tablecount == cb.names_found);
        }

        // Set up a starting, non-extracting bracket, then compile the
        // expression.
        pptr = cb.parsed_pattern;
        code = codestart;
        *code = OP_BRA as PCRE2_UCHAR;
        regexrc = compile_regex(
            (*re).overall_options,
            (*re).extra_options,
            &mut code,
            &mut pptr,
            &mut errorcode,
            0,
            &mut firstcu,
            &mut firstcuflags,
            &mut reqcu,
            &mut reqcuflags,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut cb,
            ptr::null_mut(),
        );
        if regexrc < 0 {
            (*re).flags |= PCRE2_MATCH_EMPTY as u32;
        }
        (*re).top_bracket = cb.bracount as u16;
        (*re).top_backref = cb.top_backref as u16;
        (*re).max_lookbehind = cb.max_lookbehind as u16;

        if cb.had_accept != FALSE {
            reqcu = 0; // Must disable after (*ACCEPT)
            reqcuflags = REQ_NONE;
            (*re).flags |= PCRE2_HASACCEPT as u32; // Disables minimum length
        }

        // Fill in the final opcode and check for disastrous overflow.
        *code = OP_END as PCRE2_UCHAR;
        code = code.add(1);
        usedlength = code.offset_from(codestart) as PCRE2_SIZE;
        if usedlength > length {
            errorcode = ERR23; // Overflow of code block - internal error
            cb.erroroffset = 0;
            return compile_had_cb_error(
                errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset, errorptr,
                utf, &stack_parsed_pattern, &stack_groupinfo,
            );
        }

        (*re).blocksize -= CU2BYTES((length - usedlength) as usize) as PCRE2_SIZE;

        // Scan the pattern for recursion/subroutine calls and convert the group
        // numbers into offsets. Maintain a small cache.
        if errorcode == 0 && cb.had_recurse != FALSE {
            let mut rcode: *mut PCRE2_UCHAR;
            let mut rgroup: PCRE2_SPTR;
            let mut ccount: u32 = 0;
            let mut start: c_int = RSCAN_CACHE_SIZE as c_int;
            let mut rc: [recurse_cache; RSCAN_CACHE_SIZE] = core::mem::zeroed();

            rcode = find_recurse(codestart, utf);
            while !rcode.is_null() {
                let mut p: c_int;
                let groupnumber: c_int;

                groupnumber = GET(rcode, 1) as c_int;
                if groupnumber == 0 {
                    rgroup = codestart;
                } else {
                    let mut search_from: PCRE2_SPTR = codestart;
                    rgroup = ptr::null();
                    i = 0;
                    p = start;
                    while i < ccount {
                        if groupnumber == rc[p as usize].groupnumber {
                            rgroup = rc[p as usize].group;
                            break;
                        }

                        // Group n+1 always starts to the right of group n, so we
                        // can save search time when the new group number exceeds
                        // any previously found group.
                        if groupnumber > rc[p as usize].groupnumber {
                            search_from = rc[p as usize].group;
                        }

                        i += 1;
                        p = (p + 1) & 7;
                    }

                    if rgroup.is_null() {
                        rgroup = crate::find_bracket::_pcre2_find_bracket_8(
                            search_from,
                            utf,
                            groupnumber,
                        );
                        if rgroup.is_null() {
                            errorcode = ERR53;
                            break;
                        }

                        start -= 1;
                        if start < 0 {
                            start = RSCAN_CACHE_SIZE as c_int - 1;
                        }
                        rc[start as usize].groupnumber = groupnumber;
                        rc[start as usize].group = rgroup;
                        if (ccount as usize) < RSCAN_CACHE_SIZE {
                            ccount += 1;
                        }
                    }
                }

                PUT(rcode, 1, rgroup.offset_from(codestart) as i32);

                rcode = find_recurse(rcode.add(1 + LINK_SIZE_U), utf);
            }
        }

        // Unless disabled, check whether any single character iterators can be
        // auto-possessified.
        if errorcode == 0 && (optim_flags & PCRE2_OPTIM_AUTO_POSSESS_U) != 0 {
            let temp: *mut PCRE2_UCHAR = codestart;
            let possessify_rc = crate::auto_possess::_pcre2_auto_possessify_8(temp, &cb);
            if possessify_rc != 0 {
                errorcode = ERR80;
                cb.erroroffset = 0;
            }
        }

        // Failed to compile, or error while post-processing.
        if errorcode != 0 {
            return compile_had_cb_error(
                errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset, errorptr,
                utf, &stack_parsed_pattern, &stack_groupinfo,
            );
        }

        // Successful compile. Set the anchored option if we can determine the
        // pattern is anchored.
        if ((*re).overall_options & PCRE2_ANCHORED as u32) == 0 {
            let dotstar_anchor = ((optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR_U) != 0) as BOOL;
            if is_anchored(codestart, 0, &mut cb, 0, FALSE, dotstar_anchor) != FALSE {
                (*re).overall_options |= PCRE2_ANCHORED as u32;
            }
        }

        // Set up the first code unit or startline flag, the required code unit,
        // and then study the pattern.
        if (optim_flags & PCRE2_OPTIM_START_OPTIMIZE_U) != 0 {
            let mut minminlength: c_int = 0; // For minimal minlength from first/required CU
            let study_rc: c_int;

            // If we do not have a first code unit, see if there is one that is
            // asserted.
            if firstcuflags >= REQ_NONE {
                let mut assertedcuflags: u32 = 0;
                let assertedcu = find_firstassertedcu(codestart, &mut assertedcuflags, 0);
                if assertedcuflags < REQ_NONE && assertedcu != reqcu {
                    firstcu = assertedcu;
                    firstcuflags = assertedcuflags;
                }
            }

            // Save the data for a first code unit.
            if firstcuflags < REQ_NONE {
                (*re).first_codeunit = firstcu;
                (*re).flags |= PCRE2_FIRSTSET as u32;
                minminlength += 1;

                // Handle caseless first code units.
                if (firstcuflags & REQ_CASELESS) != 0 {
                    if firstcu < 128 || (utf == FALSE && ucp == FALSE && firstcu < 255) {
                        if *cb.fcc.add(firstcu as usize) as u32 != firstcu {
                            (*re).flags |= PCRE2_FIRSTCASELESS as u32;
                        }
                    }
                    // SUPPORT_UNICODE, PCRE2_CODE_UNIT_WIDTH == 8
                    else if ucp != FALSE
                        && utf == FALSE
                        && UCD_OTHERCASE(firstcu) != firstcu
                    {
                        (*re).flags |= PCRE2_FIRSTCASELESS as u32;
                    }
                }
            }
            // When there is no first code unit, for non-anchored patterns, see
            // if we can set the PCRE2_STARTLINE flag.
            else if ((*re).overall_options & PCRE2_ANCHORED as u32) == 0 {
                let dotstar_anchor =
                    ((optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR_U) != 0) as BOOL;
                if is_startline(codestart, 0, &mut cb, 0, FALSE, dotstar_anchor) != FALSE {
                    (*re).flags |= PCRE2_STARTLINE as u32;
                }
            }

            // Handle the "required code unit", if one is set.
            if reqcuflags < REQ_NONE {
                // PCRE2_CODE_UNIT_WIDTH == 8
                if ((*re).overall_options & PCRE2_UTF as u32) == 0 // Not UTF
                    || firstcuflags >= REQ_NONE                    // First not set
                    || (firstcu & 0x80) == 0                       // First is ASCII
                    || (reqcu & 0x80) == 0
                // Req is ASCII
                {
                    minminlength += 1;
                }

                // For an anchored pattern, set up the value only if it follows a
                // variable length item.
                if ((*re).overall_options & PCRE2_ANCHORED as u32) == 0
                    || (reqcuflags & REQ_VARY) != 0
                {
                    (*re).last_codeunit = reqcu;
                    (*re).flags |= PCRE2_LASTSET as u32;

                    // Handle caseless required code units as for first CUs.
                    if (reqcuflags & REQ_CASELESS) != 0 {
                        if reqcu < 128 || (utf == FALSE && ucp == FALSE && reqcu < 255) {
                            if *cb.fcc.add(reqcu as usize) as u32 != reqcu {
                                (*re).flags |= PCRE2_LASTCASELESS as u32;
                            }
                        }
                        // SUPPORT_UNICODE, PCRE2_CODE_UNIT_WIDTH == 8
                        else if ucp != FALSE
                            && utf == FALSE
                            && UCD_OTHERCASE(reqcu) != reqcu
                        {
                            (*re).flags |= PCRE2_LASTCASELESS as u32;
                        }
                    }
                }
            }

            // Study the compiled pattern.
            study_rc = crate::study::_pcre2_study_8(re);
            if study_rc != 0 {
                errorcode = ERR31;
                cb.erroroffset = 0;
                return compile_had_cb_error(
                    errorcode, &mut re, &mut cb, ccontext, pattern, patlen, erroroffset, errorptr,
                    utf, &stack_parsed_pattern, &stack_groupinfo,
                );
            }

            // If study() set a bitmap of starting code units, it implies a
            // minimum length of at least one.
            if ((*re).flags & PCRE2_FIRSTMAPSET as u32) != 0 && minminlength == 0 {
                minminlength = 1;
            }

            // If the minimum length set (or not set) by study() is less than the
            // minimum implied by required code units, override it.
            if ((*re).minlength as c_int) < minminlength {
                (*re).minlength = minminlength as u16;
            }
        } // End of start-of-match optimizations.

        // SUPPORT_UNICODE: PCRE2_ASSERT(cb.first_data == NULL);

        // EXIT:
        compile_exit(re, &mut cb, ccontext, &stack_parsed_pattern, &stack_groupinfo)
    }
}

// ---------------------------------------------------------------------------
// Goto-label helpers
//
// These reproduce the C `EXIT` / `HAD_CB_ERROR` / `HAD_EARLY_ERROR` /
// `HAD_ERROR` label sequences. Each returns the value of `re` (NULL after an
// error), matching the single `return re;` at the EXIT label.
// ---------------------------------------------------------------------------

/// The `EXIT:` label. Frees any heap allocations and returns `re`.
unsafe fn compile_exit(
    re: *mut pcre2_real_code,
    cb: *mut compile_block,
    ccontext: *mut pcre2_real_compile_context,
    stack_parsed_pattern: &[u32; PARSED_PATTERN_DEFAULT_SIZE],
    stack_groupinfo: &[u32; GROUPINFO_DEFAULT_SIZE],
) -> *mut pcre2_real_code {
    unsafe {
        if (*cb).parsed_pattern != stack_parsed_pattern.as_ptr() as *mut u32 {
            ((*ccontext).memctl.free.unwrap())(
                (*cb).parsed_pattern as *mut c_void,
                (*ccontext).memctl.memory_data,
            );
        }
        if (*cb).named_group_list_size > NAMED_GROUP_LIST_SIZE as u32 {
            ((*ccontext).memctl.free.unwrap())(
                (*cb).named_groups as *mut c_void,
                (*ccontext).memctl.memory_data,
            );
        }
        if (*cb).groupinfo != stack_groupinfo.as_ptr() as *mut u32 {
            ((*ccontext).memctl.free.unwrap())(
                (*cb).groupinfo as *mut c_void,
                (*ccontext).memctl.memory_data,
            );
        }

        re // Will be NULL after an error
    }
}

/// The `HAD_ERROR:` label. Sets `*errorptr`, frees `re`, then continues to the
/// tail of the function that frees the compile-data list and jumps to EXIT.
/// Because this path is reached from `valid_utf()` (before any heap parsed
/// pattern is allocated), it does not need the EXIT free logic beyond the
/// compile-data list — but we reproduce the full C behaviour: the C code does
/// `goto EXIT` at the very end. `re`/`cb` allocations here are all still on the
/// stack, so `compile_exit` is a no-op for them.
unsafe fn compile_had_error(
    errorcode: c_int,
    re: *mut *mut pcre2_real_code,
    cb: *mut compile_block,
    errorptr: *mut c_int,
) -> *mut pcre2_real_code {
    unsafe {
        // HAD_ERROR:
        *errorptr = errorcode;
        crate::compile_parse_util::pcre2_code_free_8(*re as *mut pcre2_code);
        *re = ptr::null_mut();

        if !(*cb).first_data.is_null() {
            let mut current_data = (*cb).first_data;
            loop {
                let next_data = (*current_data).next;
                ((*(*cb).cx).memctl.free.unwrap())(
                    current_data as *mut c_void,
                    (*(*cb).cx).memctl.memory_data,
                );
                current_data = next_data;
                if current_data.is_null() {
                    break;
                }
            }
        }

        // goto EXIT — but the parsed pattern/groupinfo here are the stack ones
        // (this path is taken before any heap allocation of those). We free the
        // compile-data list above; the EXIT frees are all no-ops for the stack
        // buffers, and cb.parsed_pattern still points at the stack buffer.
        *re
    }
}

/// The `HAD_EARLY_ERROR:` label. Computes `*erroroffset` from `ptr` then falls
/// through to `HAD_ERROR`. This path is taken only before any heap allocation
/// of parsed pattern / group info, so it need not run the EXIT frees for those.
unsafe fn compile_had_early_error(
    errorcode: c_int,
    ptr: PCRE2_SPTR,
    pattern: PCRE2_SPTR,
    _patlen: PCRE2_SIZE,
    erroroffset: *mut PCRE2_SIZE,
    errorptr: *mut c_int,
    _utf: BOOL,
) -> *mut pcre2_real_code {
    unsafe {
        // HAD_EARLY_ERROR:
        // PCRE2_ASSERT(ptr >= pattern && ptr <= pattern + patlen);
        *erroroffset = ptr.offset_from(pattern) as PCRE2_SIZE;

        // HAD_ERROR:
        *errorptr = errorcode;
        // re is NULL on all HAD_EARLY_ERROR paths; pcre2_code_free(NULL) is a
        // no-op, and cb.first_data is NULL, so nothing else to free.
        ptr::null_mut()
    }
}

/// The `HAD_CB_ERROR:` label. Computes the error offset from `cb.erroroffset`
/// then runs the `HAD_EARLY_ERROR` / `HAD_ERROR` sequence and the EXIT frees.
unsafe fn compile_had_cb_error(
    errorcode: c_int,
    re: *mut *mut pcre2_real_code,
    cb: *mut compile_block,
    ccontext: *mut pcre2_real_compile_context,
    pattern: PCRE2_SPTR,
    patlen: PCRE2_SIZE,
    erroroffset: *mut PCRE2_SIZE,
    errorptr: *mut c_int,
    _utf: BOOL,
    stack_parsed_pattern: &[u32; PARSED_PATTERN_DEFAULT_SIZE],
    stack_groupinfo: &[u32; GROUPINFO_DEFAULT_SIZE],
) -> *mut pcre2_real_code {
    unsafe {
        // HAD_CB_ERROR:
        let ptr: PCRE2_SPTR = pattern.add((*cb).erroroffset as usize);

        // HAD_EARLY_ERROR:
        // PCRE2_ASSERT(ptr >= pattern && ptr <= pattern + patlen);
        let _ = patlen;
        *erroroffset = ptr.offset_from(pattern) as PCRE2_SIZE;

        // HAD_ERROR:
        *errorptr = errorcode;
        crate::compile_parse_util::pcre2_code_free_8(*re as *mut pcre2_code);
        *re = ptr::null_mut();

        if !(*cb).first_data.is_null() {
            let mut current_data = (*cb).first_data;
            loop {
                let next_data = (*current_data).next;
                ((*(*cb).cx).memctl.free.unwrap())(
                    current_data as *mut c_void,
                    (*(*cb).cx).memctl.memory_data,
                );
                current_data = next_data;
                if current_data.is_null() {
                    break;
                }
            }
        }

        // goto EXIT
        compile_exit(*re, cb, ccontext, stack_parsed_pattern, stack_groupinfo)
    }
}
