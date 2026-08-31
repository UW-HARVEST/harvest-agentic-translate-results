//! Translation of the public compile API of `c_src/src/pcre2_compile.c`:
//! `pcre2_code_copy` (C ~1132), `pcre2_code_copy_with_tables` (~1166),
//! `pcre2_code_free` (~1201) and `pcre2_compile` itself (~10279..end).
//!
//! Built for the 8-bit library with `SUPPORT_UNICODE` (hence
//! `SUPPORT_WIDE_CHARS`), `LINK_SIZE == 2`, no JIT, no EBCDIC, no
//! `PCRE2_DEBUG`, no `SUPPORT_VALGRIND`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens, dead_code)]

use core::ffi::{c_int, c_void};

use crate::chars::*;
use crate::compile_internal::*;
use crate::compile_scan::{
    check_lookbehinds, find_firstassertedcu, find_recurse, is_anchored, is_startline,
};
use crate::compile_tables::{
    pso, pso_list, IS_DIGIT, COMPILE_WORK_SIZE, GROUPINFO_DEFAULT_SIZE, NAMED_GROUP_LIST_SIZE,
    PARSED_PATTERN_DEFAULT_SIZE, PSO_BSR, PSO_FLG, PSO_LIMD, PSO_LIMH, PSO_LIMM, PSO_NL, PSO_OPT,
    PSO_OPTMZ, PSO_XOPT, REQ_CASELESS, REQ_NONE, REQ_VARY,
};
use crate::internal::*;
use crate::opcodes::*;

/* C type aliases. In the 8-bit library `pcre2_code` is `pcre2_real_code` and
`pcre2_compile_context` is `pcre2_real_compile_context`. */
type pcre2_code = pcre2_real_code;
type pcre2_compile_context = pcre2_real_compile_context;

/* UINT32_MAX from <stdint.h>. */
const UINT32_MAX: u32 = u32::MAX;

/* C16_WORK_SIZE = (COMPILE_WORK_SIZE * sizeof(PCRE2_UCHAR)) / sizeof(uint16_t).
In 8-bit mode sizeof(PCRE2_UCHAR) == 1 and sizeof(uint16_t) == 2. */
const C16_WORK_SIZE: usize = (COMPILE_WORK_SIZE * 1) / 2;

const RSCAN_CACHE_SIZE: usize = 8;

/* Public option masks (see pcre2_compile.c). */
const PUBLIC_LITERAL_COMPILE_OPTIONS: u32 = PCRE2_ANCHORED
    | PCRE2_AUTO_CALLOUT
    | PCRE2_CASELESS
    | PCRE2_ENDANCHORED
    | PCRE2_FIRSTLINE
    | PCRE2_LITERAL
    | PCRE2_MATCH_INVALID_UTF
    | PCRE2_NO_START_OPTIMIZE
    | PCRE2_NO_UTF_CHECK
    | PCRE2_USE_OFFSET_LIMIT
    | PCRE2_UTF;

const PUBLIC_COMPILE_OPTIONS: u32 = PUBLIC_LITERAL_COMPILE_OPTIONS
    | PCRE2_ALLOW_EMPTY_CLASS
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
    | PCRE2_ALT_EXTENDED_CLASS;

const PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS: u32 = PCRE2_EXTRA_MATCH_LINE
    | PCRE2_EXTRA_MATCH_WORD
    | PCRE2_EXTRA_CASELESS_RESTRICT
    | PCRE2_EXTRA_TURKISH_CASING;

const PUBLIC_COMPILE_EXTRA_OPTIONS: u32 = PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS
    | PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES
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
    | PCRE2_EXTRA_NEVER_CALLOUT;

/*************************************************
*               Copy compiled code               *
*************************************************/

/* Compiled JIT code cannot be copied, so the new compiled block has no
associated JIT data. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_8(code: *const pcre2_code) -> *mut pcre2_code {
    unsafe {
        if code.is_null() {
            return core::ptr::null_mut();
        }
        let newcode = ((*code).memctl.malloc.unwrap())(
            (*code).blocksize,
            (*code).memctl.memory_data,
        ) as *mut pcre2_code;
        if newcode.is_null() {
            return core::ptr::null_mut();
        }
        memcpy(newcode as *mut u8, code as *const u8, (*code).blocksize);
        (*newcode).executable_jit = core::ptr::null_mut();

        /* If the code is one that has been deserialized, increment the reference
        count in the decoded tables. */

        if ((*code).flags & PCRE2_DEREF_TABLES) != 0 {
            let ref_count = (*code).tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
            *ref_count += 1;
        }

        newcode
    }
}

/*************************************************
*     Copy compiled code and character tables    *
*************************************************/

/* Compiled JIT code cannot be copied, so the new compiled block has no
associated JIT data. This version of code_copy also makes a separate copy of
the character tables. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_with_tables_8(
    code: *const pcre2_code,
) -> *mut pcre2_code {
    unsafe {
        if code.is_null() {
            return core::ptr::null_mut();
        }
        let newcode = ((*code).memctl.malloc.unwrap())(
            (*code).blocksize,
            (*code).memctl.memory_data,
        ) as *mut pcre2_code;
        if newcode.is_null() {
            return core::ptr::null_mut();
        }
        memcpy(newcode as *mut u8, code as *const u8, (*code).blocksize);
        (*newcode).executable_jit = core::ptr::null_mut();

        let newtables = ((*code).memctl.malloc.unwrap())(
            TABLES_LENGTH + core::mem::size_of::<PCRE2_SIZE>(),
            (*code).memctl.memory_data,
        ) as *mut u8;
        if newtables.is_null() {
            ((*code).memctl.free.unwrap())(newcode as *mut c_void, (*code).memctl.memory_data);
            return core::ptr::null_mut();
        }
        memcpy(newtables, (*code).tables, TABLES_LENGTH);
        let ref_count = newtables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
        *ref_count = 1;

        (*newcode).tables = newtables;
        (*newcode).flags |= PCRE2_DEREF_TABLES;
        newcode
    }
}

/*************************************************
*               Free compiled code               *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_free_8(code: *mut pcre2_code) {
    unsafe {
        if !code.is_null() {
            /* SUPPORT_JIT is not defined, so there is no JIT block to free. */

            if ((*code).flags & PCRE2_DEREF_TABLES) != 0 {
                /* Decoded tables belong to the codes after deserialization, and
                they must be freed when there are no more references to them. The
                *ref_count should always be > 0. */

                let ref_count = (*code).tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
                if *ref_count > 0 {
                    *ref_count -= 1;
                    if *ref_count == 0 {
                        ((*code).memctl.free.unwrap())(
                            (*code).tables as *mut c_void,
                            (*code).memctl.memory_data,
                        );
                    }
                }
            }

            ((*code).memctl.free.unwrap())(code as *mut c_void, (*code).memctl.memory_data);
        }
    }
}

/*************************************************
*        Compile a Regular Expression            *
*************************************************/

/* This function is used to compile a regular expression.

Arguments:
  pattern       the regular expression
  patlen        the length of the pattern, or PCRE2_ZERO_TERMINATED
  options       option bits
  errorptr      pointer to errorcode
  erroroffset   pointer to error offset
  ccontext      points to a compile context or is NULL

Returns:        pointer to compiled data block, or NULL on error,
                with errorcode and erroroffset set
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_8(
    mut pattern: PCRE2_SPTR,
    mut patlen: PCRE2_SIZE,
    mut options: u32,
    errorptr: *mut c_int,
    erroroffset: *mut PCRE2_SIZE,
    mut ccontext: *mut pcre2_compile_context,
) -> *mut pcre2_code {
    unsafe {
        let mut utf: BOOL; /* Set TRUE for UTF mode */
        let ucp: BOOL; /* Set TRUE for UCP mode */
        let mut has_lookbehind: BOOL = FALSE; /* Set TRUE if a lookbehind is found */
        let zero_terminated: BOOL; /* Set TRUE for zero-terminated pattern */
        let mut re: *mut pcre2_real_code = core::ptr::null_mut(); /* What we will return */
        let mut cb: compile_block = core::mem::zeroed(); /* "Static" compile-time data */
        let tables: *const u8; /* Char tables base pointer */

        let mut null_str: [PCRE2_UCHAR; 1] = [0xcd]; /* Dummy for handling null inputs */
        let mut code: *mut PCRE2_UCHAR; /* Current pointer in compiled code */
        let codestart: *mut PCRE2_UCHAR; /* Start of compiled code */
        let mut ptr: PCRE2_SPTR; /* Current pointer in pattern */
        let mut pptr: *mut u32; /* Current pointer in parsed pattern */

        let mut length: PCRE2_SIZE = 1; /* Allow for final END opcode */
        let usedlength: PCRE2_SIZE; /* Actual length used */
        let mut re_blocksize: PCRE2_SIZE; /* Size of memory block */
        let parsed_size_needed: PCRE2_SIZE; /* Needed for parsed pattern */

        let mut firstcuflags: u32 = 0;
        let mut reqcuflags: u32 = 0; /* Type of first/req code unit */
        let mut firstcu: u32 = 0;
        let mut reqcu: u32 = 0; /* Value of first/req code unit */
        let mut setflags: u32 = 0; /* NL and BSR set flags */
        let mut xoptions: u32; /* Flags from context, modified */

        let mut skipatstart: u32; /* When checking (*UTF) etc */
        let mut limit_heap: u32 = UINT32_MAX;
        let mut limit_match: u32 = UINT32_MAX; /* Unset match limits */
        let mut limit_depth: u32 = UINT32_MAX;

        let mut newline: c_int = 0; /* Unset; can be set by the pattern */
        let mut bsr: c_int = 0; /* Unset; can be set by the pattern */
        let mut errorcode: c_int = 0; /* Initialize to avoid compiler warn */
        let regexrc: c_int; /* Return from compile */

        let mut i: u32; /* Local loop counter */

        /* Enable all optimizations by default. */
        let mut optim_flags: u32 = if !ccontext.is_null() {
            (*ccontext).optimization_flags
        } else {
            PCRE2_OPTIMIZATION_ALL
        };

        /* Comments at the head of this file explain about these variables. */

        let mut stack_groupinfo: [u32; GROUPINFO_DEFAULT_SIZE] =
            [0; GROUPINFO_DEFAULT_SIZE];
        let mut stack_parsed_pattern: [u32; PARSED_PATTERN_DEFAULT_SIZE] =
            [0; PARSED_PATTERN_DEFAULT_SIZE];
        let mut named_groups: [named_group; NAMED_GROUP_LIST_SIZE] =
            core::mem::zeroed();

        /* The workspace is used in different ways in the different compiling
        phases. It needs to be 16-bit aligned for the preliminary parsing scan. */

        let mut c16workspace: [u16; C16_WORK_SIZE] = [0; C16_WORK_SIZE];
        let cworkspace: *mut PCRE2_UCHAR = c16workspace.as_mut_ptr() as *mut PCRE2_UCHAR;

        /* This macro reproduces the C `goto EXIT` cleanup epilogue. */
        // The main body runs inside a labelled block; error paths break out to
        // one of the goto targets that follow.

        // Return value from the body: on the EXIT path we simply fall through
        // and run the cleanup; error paths funnel through HAD_* handling.

        /* -------------- Check arguments and set up the pattern ------------- */

        /* There must be error code and offset pointers. */

        if errorptr.is_null() {
            if !erroroffset.is_null() {
                *erroroffset = 0;
            }
            return core::ptr::null_mut();
        }
        if erroroffset.is_null() {
            if !errorptr.is_null() {
                *errorptr = ERR120;
            }
            return core::ptr::null_mut();
        }
        *errorptr = ERR0;
        *erroroffset = 0;

        /* There must be a pattern, but NULL is allowed with zero length. */

        if pattern.is_null() {
            if patlen == 0 {
                pattern = null_str.as_mut_ptr();
            } else {
                *errorptr = ERR16;
                return core::ptr::null_mut();
            }
        }

        /* A NULL compile context means "use a default context" */

        if ccontext.is_null() {
            ccontext = &raw mut crate::context::_pcre2_default_compile_context_8;
        }

        /* PCRE2_MATCH_INVALID_UTF implies UTF */

        if (options & PCRE2_MATCH_INVALID_UTF) != 0 {
            options |= PCRE2_UTF;
        }

        /* Check that all undefined public option bits are zero. */

        if (options & !PUBLIC_COMPILE_OPTIONS) != 0
            || ((*ccontext).extra_options & !PUBLIC_COMPILE_EXTRA_OPTIONS) != 0
        {
            *errorptr = ERR17;
            return core::ptr::null_mut();
        }

        if (options & PCRE2_LITERAL) != 0
            && ((options & !PUBLIC_LITERAL_COMPILE_OPTIONS) != 0
                || ((*ccontext).extra_options & !PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS) != 0)
        {
            *errorptr = ERR92;
            return core::ptr::null_mut();
        }

        /* A zero-terminated pattern is indicated by the special length value
        PCRE2_ZERO_TERMINATED. Check for an overlong pattern. */

        zero_terminated = (patlen == PCRE2_ZERO_TERMINATED) as BOOL;
        if zero_terminated != FALSE {
            patlen = crate::string_utils::strlen(pattern);
        }
        let _ = zero_terminated; /* Silence compiler; only used if Valgrind enabled */

        if patlen > (*ccontext).max_pattern_length {
            *errorptr = ERR88;
            return core::ptr::null_mut();
        }

        /* Optimization flags in 'options' can override those in the compile
        context. This is because some options to disable optimizations were added
        before the optimization flags word existed, and we need to continue
        supporting them for backwards compatibility. */

        if (options & PCRE2_NO_AUTO_POSSESS) != 0 {
            optim_flags &= !PCRE2_OPTIM_AUTO_POSSESS;
        }
        if (options & PCRE2_NO_DOTSTAR_ANCHOR) != 0 {
            optim_flags &= !PCRE2_OPTIM_DOTSTAR_ANCHOR;
        }
        if (options & PCRE2_NO_START_OPTIMIZE) != 0 {
            optim_flags &= !PCRE2_OPTIM_START_OPTIMIZE;
        }

        /* From here on, all returns from this function should end up going via
        the EXIT label. */

        /* ------------ Initialize the "static" compile data -------------- */

        tables = if !(*ccontext).tables.is_null() {
            (*ccontext).tables
        } else {
            &raw const crate::context::_pcre2_default_tables_8 as *const u8
        };

        cb.lcc = tables.add(lcc_offset); /* Individual */
        cb.fcc = tables.add(fcc_offset); /*   character */
        cb.cbits = tables.add(cbits_offset); /*      tables */
        cb.ctypes = tables.add(ctypes_offset);

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
        cb.max_lookbehind = 0; /* Max encountered */
        cb.max_varlookbehind = (*ccontext).max_varlookbehind; /* Limit */
        cb.name_entry_size = 0;
        cb.name_table = core::ptr::null_mut();
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
        cb.first_data = core::ptr::null_mut();
        cb.last_data = core::ptr::null_mut();
        /* SUPPORT_WIDE_CHARS is defined */
        cb.char_lists_size = 0;

        /* Maximum back reference and backref bitmap. The bitmap records up to 31
        back references to help in deciding whether (.*) can be treated as
        anchored or not. */

        cb.top_backref = 0;
        cb.backref_map = 0;

        /* Escape sequences \1 to \9 ... small_ref_offset. */

        i = 0;
        while i < 10 {
            cb.small_ref_offset[i as usize] = PCRE2_UNSET;
            i += 1;
        }

        /* --------------- Start looking at the pattern --------------- */

        /* Unless PCRE2_LITERAL is set, check for global one-time option settings
        at the start of the pattern, and remember the offset to the actual
        regex. */

        xoptions = (*ccontext).extra_options;
        ptr = pattern;
        skipatstart = 0;

        if (options & PCRE2_LITERAL) == 0 {
            'pso: while patlen - skipatstart as PCRE2_SIZE >= 2
                && *ptr.add(skipatstart as usize) as u32 == CHAR_LEFT_PARENTHESIS
                && *ptr.add(skipatstart as usize + 1) as u32 == CHAR_ASTERISK
            {
                i = 0;
                let pcount = (pso_list.len()) as u32;
                while i < pcount {
                    let p: *const pso = &pso_list[i as usize];

                    if patlen - skipatstart as PCRE2_SIZE - 2 >= (*p).length as PCRE2_SIZE
                        && crate::string_utils::strncmp_c8(
                            ptr.add(skipatstart as usize + 2),
                            (*p).name,
                            (*p).length as usize,
                        ) == 0
                    {
                        let mut c: u32;
                        let mut pp: u32;

                        skipatstart += (*p).length as u32 + 2;
                        match (*p).type_ {
                            PSO_OPT => {
                                cb.external_options |= (*p).value;
                            }

                            PSO_XOPT => {
                                xoptions |= (*p).value;
                            }

                            PSO_FLG => {
                                setflags |= (*p).value;
                            }

                            PSO_NL => {
                                newline = (*p).value as c_int;
                                setflags |= PCRE2_NL_SET;
                            }

                            PSO_BSR => {
                                bsr = (*p).value as c_int;
                                setflags |= PCRE2_BSR_SET;
                            }

                            PSO_LIMM | PSO_LIMD | PSO_LIMH => {
                                c = 0;
                                pp = skipatstart;
                                while (pp as PCRE2_SIZE) < patlen
                                    && IS_DIGIT(*ptr.add(pp as usize) as u32)
                                {
                                    if c > UINT32_MAX / 10 - 1 {
                                        break; /* Integer overflow */
                                    }
                                    c = c * 10 + (*ptr.add(pp as usize) as u32 - CHAR_0);
                                    pp += 1;
                                }
                                if (pp as PCRE2_SIZE) >= patlen
                                    || pp == skipatstart
                                    || *ptr.add(pp as usize) as u32 != CHAR_RIGHT_PARENTHESIS
                                {
                                    errorcode = ERR60;
                                    ptr = ptr.add(pp as usize);
                                    utf = FALSE; /* Used by HAD_EARLY_ERROR */
                                    return had_early_error(
                                        errorcode, ptr, pattern, patlen, utf, erroroffset,
                                        errorptr, re, &mut cb,
                                    );
                                }
                                if (*p).type_ == PSO_LIMH {
                                    limit_heap = c;
                                } else if (*p).type_ == PSO_LIMM {
                                    limit_match = c;
                                } else {
                                    limit_depth = c;
                                }
                                pp += 1;
                                skipatstart = pp;
                            }

                            PSO_OPTMZ => {
                                optim_flags &= !((*p).value);

                                /* For backward compatibility the three original
                                VERBs to disable optimizations need to also update
                                the corresponding bit in the external options. */

                                match (*p).value {
                                    PCRE2_OPTIM_AUTO_POSSESS => {
                                        cb.external_options |= PCRE2_NO_AUTO_POSSESS;
                                    }
                                    PCRE2_OPTIM_DOTSTAR_ANCHOR => {
                                        cb.external_options |= PCRE2_NO_DOTSTAR_ANCHOR;
                                    }
                                    PCRE2_OPTIM_START_OPTIMIZE => {
                                        cb.external_options |= PCRE2_NO_START_OPTIMIZE;
                                    }
                                    _ => {}
                                }
                            }

                            _ => {
                                /* PCRE2_DEBUG_UNREACHABLE(); */
                            }
                        }
                        break; /* Out of the table scan loop */
                    }
                    i += 1;
                }
                if i >= pcount {
                    break 'pso; /* Out of pso loop */
                }
            }
        }

        /* End of pattern-start options; advance to start of real regex. */

        ptr = ptr.add(skipatstart as usize);

        /* SUPPORT_UNICODE is defined, so UTF/UCP are supported. */

        /* Check UTF. */

        utf = ((cb.external_options & PCRE2_UTF) != 0) as BOOL;
        if utf != FALSE {
            if (options & PCRE2_NEVER_UTF) != 0 {
                errorcode = ERR74;
                return had_early_error(
                    errorcode, ptr, pattern, patlen, utf, erroroffset, errorptr, re, &mut cb,
                );
            }
            if (options & PCRE2_NO_UTF_CHECK) == 0 && {
                errorcode = crate::valid_utf::valid_utf(pattern, patlen, erroroffset);
                errorcode != 0
            } {
                /* Offset was set by valid_utf() */
                return had_error(errorcode, errorptr, re, &mut cb);
            }
            /* PCRE2_CODE_UNIT_WIDTH == 8, so the 16-bit surrogate check is
            omitted. */
        }

        /* Check UCP lockout. */

        ucp = ((cb.external_options & PCRE2_UCP) != 0) as BOOL;
        if ucp != FALSE && (cb.external_options & PCRE2_NEVER_UCP) != 0 {
            errorcode = ERR75;
            return had_early_error(
                errorcode, ptr, pattern, patlen, utf, erroroffset, errorptr, re, &mut cb,
            );
        }

        /* PCRE2_EXTRA_TURKISH_CASING checks */

        if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
            if utf == FALSE && ucp == FALSE {
                errorcode = ERR104;
                return had_early_error(
                    errorcode, ptr, pattern, patlen, utf, erroroffset, errorptr, re, &mut cb,
                );
            }

            /* PCRE2_CODE_UNIT_WIDTH == 8 */
            if utf == FALSE {
                errorcode = ERR105;
                return had_early_error(
                    errorcode, ptr, pattern, patlen, utf, erroroffset, errorptr, re, &mut cb,
                );
            }

            if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                errorcode = ERR106;
                return had_early_error(
                    errorcode, ptr, pattern, patlen, utf, erroroffset, errorptr, re, &mut cb,
                );
            }
        }

        /* Process the BSR setting. */

        if bsr == 0 {
            bsr = (*ccontext).bsr_convention as c_int;
        }

        /* Process the newline setting. */

        if newline == 0 {
            newline = (*ccontext).newline_convention as c_int;
        }
        cb.nltype = NLTYPE_FIXED;
        match newline as u32 {
            PCRE2_NEWLINE_CR => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_CR as PCRE2_UCHAR;
            }
            PCRE2_NEWLINE_LF => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_NL as PCRE2_UCHAR;
            }
            PCRE2_NEWLINE_NUL => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_NUL as PCRE2_UCHAR;
            }
            PCRE2_NEWLINE_CRLF => {
                cb.nllen = 2;
                cb.nl[0] = CHAR_CR as PCRE2_UCHAR;
                cb.nl[1] = CHAR_NL as PCRE2_UCHAR;
            }
            PCRE2_NEWLINE_ANY => {
                cb.nltype = NLTYPE_ANY;
            }
            PCRE2_NEWLINE_ANYCRLF => {
                cb.nltype = NLTYPE_ANYCRLF;
            }
            _ => {
                errorcode = ERR56;
                return had_early_error(
                    errorcode, ptr, pattern, patlen, utf, erroroffset, errorptr, re, &mut cb,
                );
            }
        }

        /* Pre-scan the pattern to do two things ... put a processed version into
        the parsed_pattern vector. */

        parsed_size_needed =
            crate::compile_tables::max_parsed_pattern(ptr, cb.end_pattern, utf, options)
                as PCRE2_SIZE;

        /* Allow for 2x uint32_t at the start and 2 at the end, for
        PCRE2_EXTRA_MATCH_WORD or PCRE2_EXTRA_MATCH_LINE (which are exclusive). */

        let mut parsed_size_needed = parsed_size_needed;
        if ((*ccontext).extra_options & (PCRE2_EXTRA_MATCH_WORD | PCRE2_EXTRA_MATCH_LINE)) != 0 {
            parsed_size_needed += 4;
        }

        /* When PCRE2_AUTO_CALLOUT is set we allow for one callout at the end. */

        if (options & PCRE2_AUTO_CALLOUT) != 0 {
            parsed_size_needed += 4;
        }

        parsed_size_needed += 1; /* For the final META_END */

        if parsed_size_needed > PARSED_PATTERN_DEFAULT_SIZE {
            let heap_parsed_pattern = ((*ccontext).memctl.malloc.unwrap())(
                parsed_size_needed * core::mem::size_of::<u32>(),
                (*ccontext).memctl.memory_data,
            ) as *mut u32;
            if heap_parsed_pattern.is_null() {
                *errorptr = ERR21;
                return exit_cleanup(
                    re,
                    &mut cb,
                    ccontext,
                    stack_parsed_pattern.as_mut_ptr(),
                    stack_groupinfo.as_mut_ptr(),
                );
            }
            cb.parsed_pattern = heap_parsed_pattern;
        }
        cb.parsed_pattern_end = cb.parsed_pattern.add(parsed_size_needed);

        /* Do the parsing scan. */

        errorcode = crate::compile_parse::parse_regex(
            ptr,
            cb.external_options,
            xoptions,
            &mut has_lookbehind,
            &mut cb,
        );
        if errorcode != 0 {
            return had_cb_error(
                errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                ccontext, stack_parsed_pattern.as_mut_ptr(), stack_groupinfo.as_mut_ptr(),
            );
        }

        /* If there are any lookbehinds, scan the parsed pattern to figure out
        their lengths. */

        if has_lookbehind != FALSE {
            let mut loopcount: c_int = 0;
            if cb.bracount >= (GROUPINFO_DEFAULT_SIZE / 2) as u32 {
                cb.groupinfo = ((*ccontext).memctl.malloc.unwrap())(
                    (2 * (cb.bracount as usize + 1)) * core::mem::size_of::<u32>(),
                    (*ccontext).memctl.memory_data,
                ) as *mut u32;
                if cb.groupinfo.is_null() {
                    errorcode = ERR21;
                    cb.erroroffset = 0;
                    return had_cb_error(
                        errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                        ccontext, stack_parsed_pattern.as_mut_ptr(),
                        stack_groupinfo.as_mut_ptr(),
                    );
                }
            }
            memset(
                cb.groupinfo as *mut u8,
                0,
                (2 * cb.bracount as usize + 1) * core::mem::size_of::<u32>(),
            );
            errorcode = check_lookbehinds(
                cb.parsed_pattern,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut cb,
                &mut loopcount,
            );
            if errorcode != 0 {
                return had_cb_error(
                    errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                    ccontext, stack_parsed_pattern.as_mut_ptr(), stack_groupinfo.as_mut_ptr(),
                );
            }
        }

        /* DEBUG_SHOW_PARSED / DEBUG_SHOW_CAPTURES are skipped. */

        /* Pretend to compile the pattern while actually just accumulating the
        amount of memory required in the 'length' variable. */

        cb.erroroffset = patlen; /* For any subsequent errors that do not set it */
        pptr = cb.parsed_pattern;
        code = cworkspace;
        *code = OP_BRA;

        crate::compile_branch::compile_regex(
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
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut cb,
            &mut length,
        );

        if errorcode != 0 {
            return had_cb_error(
                errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                ccontext, stack_parsed_pattern.as_mut_ptr(), stack_groupinfo.as_mut_ptr(),
            );
        }

        /* This should be caught in compile_regex(), but just in case... */

        /* SUPPORT_WIDE_CHARS is defined. */
        debug_assert!((cb.char_lists_size & 0x3) == 0);
        if length > MAX_PATTERN_SIZE
            || MAX_PATTERN_SIZE - length
                < (cb.char_lists_size / core::mem::size_of::<PCRE2_UCHAR>())
        {
            errorcode = ERR20;
            cb.erroroffset = 0;
            return had_cb_error(
                errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                ccontext, stack_parsed_pattern.as_mut_ptr(), stack_groupinfo.as_mut_ptr(),
            );
        }

        /* Compute the size of, then, if not too large, get and initialize the
        data block for storing the compiled pattern and names table. */

        re_blocksize = cu2bytes(
            cb.names_found as PCRE2_SIZE * cb.name_entry_size as PCRE2_SIZE,
        );

        /* SUPPORT_WIDE_CHARS is defined. */
        if cb.char_lists_size != 0 {
            /* PCRE2_CODE_UNIT_WIDTH != 32; align to 32 bit first. This ensures
            the allocated area will also be 32 bit aligned. */
            re_blocksize =
                clist_align_to(re_blocksize, core::mem::size_of::<u32>()) as PCRE2_SIZE;
            re_blocksize += cb.char_lists_size;
        }

        re_blocksize += cu2bytes(length);

        if re_blocksize > (*ccontext).max_pattern_compiled_length {
            errorcode = ERR101;
            cb.erroroffset = 0;
            return had_cb_error(
                errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                ccontext, stack_parsed_pattern.as_mut_ptr(), stack_groupinfo.as_mut_ptr(),
            );
        }

        re_blocksize += core::mem::size_of::<pcre2_real_code>();
        re = ((*ccontext).memctl.malloc.unwrap())(
            re_blocksize,
            (*ccontext).memctl.memory_data,
        ) as *mut pcre2_real_code;
        if re.is_null() {
            errorcode = ERR21;
            cb.erroroffset = 0;
            return had_cb_error(
                errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                ccontext, stack_parsed_pattern.as_mut_ptr(), stack_groupinfo.as_mut_ptr(),
            );
        }

        /* ... explicitly write to the last 8 bytes of the structure before
        setting the fields. */

        memset(
            (re as *mut u8).add(core::mem::size_of::<pcre2_real_code>() - 8),
            0,
            8,
        );
        (*re).memctl = (*ccontext).memctl;
        (*re).tables = tables;
        (*re).executable_jit = core::ptr::null_mut();
        memset((*re).start_bitmap.as_mut_ptr(), 0, 32 * core::mem::size_of::<u8>());
        (*re).blocksize = re_blocksize;
        (*re).code_start = re_blocksize - cu2bytes(length);
        (*re).magic_number = MAGIC_NUMBER;
        (*re).compile_options = options;
        (*re).overall_options = cb.external_options;
        (*re).extra_options = xoptions;
        (*re).flags = PCRE2_CODE_UNIT_WIDTH / 8 | cb.external_flags | setflags;
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

        /* The basic block is immediately followed by the name table, and the
        compiled code follows after that. */

        codestart = (re as *mut u8).add((*re).code_start) as *mut PCRE2_UCHAR;

        /* Update the compile data block for the actual compile. */

        cb.parens_depth = 0;
        cb.assert_depth = 0;
        cb.lastcapture = 0;
        cb.name_table =
            (re as *mut u8).add(core::mem::size_of::<pcre2_real_code>()) as *mut PCRE2_UCHAR;
        cb.start_code = codestart;
        cb.req_varyopt = 0;
        cb.had_accept = FALSE;
        cb.had_pruneorskip = FALSE;
        /* SUPPORT_WIDE_CHARS is defined. */
        cb.char_lists_size = 0;

        /* If any named groups were found, create the name/number table from the
        list created in the pre-pass. */

        if cb.names_found > 0 {
            let mut ng = cb.named_groups;
            let mut tablecount: u32 = 0;

            /* Length 0 represents duplicates, and they have already been
            handled. */
            i = 0;
            while i < cb.names_found as u32 {
                if (*ng).length > 0 {
                    tablecount =
                        crate::compile_cgroup::add_name_to_table(&mut cb, ng, tablecount);
                }
                i += 1;
                ng = ng.add(1);
            }

            debug_assert!(tablecount == cb.names_found as u32);
        }

        /* Set up a starting, non-extracting bracket, then compile the
        expression. */

        pptr = cb.parsed_pattern;
        code = codestart;
        *code = OP_BRA;
        regexrc = crate::compile_branch::compile_regex(
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
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut cb,
            core::ptr::null_mut(),
        );
        if regexrc < 0 {
            (*re).flags |= PCRE2_MATCH_EMPTY;
        }
        (*re).top_bracket = cb.bracount as u16;
        (*re).top_backref = cb.top_backref as u16;
        (*re).max_lookbehind = cb.max_lookbehind as u16;

        if cb.had_accept != FALSE {
            reqcu = 0; /* Must disable after (*ACCEPT) */
            reqcuflags = REQ_NONE;
            (*re).flags |= PCRE2_HASACCEPT; /* Disables minimum length */
        }

        /* Fill in the final opcode and check for disastrous overflow. */

        *code = OP_END;
        code = code.add(1);
        usedlength = code.offset_from(codestart) as PCRE2_SIZE;

        if usedlength > length {
            errorcode = ERR23; /* Overflow of code block - internal error */
            cb.erroroffset = 0;
            return had_cb_error(
                errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                ccontext, stack_parsed_pattern.as_mut_ptr(), stack_groupinfo.as_mut_ptr(),
            );
        }

        (*re).blocksize -= cu2bytes(length - usedlength);

        /* Scan the pattern for recursion/subroutine calls and convert the group
        numbers into offsets. */

        if errorcode == 0 && cb.had_recurse != FALSE {
            let mut rcode: *mut PCRE2_UCHAR;
            let mut rgroup: PCRE2_SPTR;
            let mut ccount: u32 = 0;
            let mut start: c_int = RSCAN_CACHE_SIZE as c_int;
            let mut rc: [recurse_cache; RSCAN_CACHE_SIZE] = core::mem::zeroed();

            rcode = find_recurse(codestart, utf);
            while !rcode.is_null() {
                let mut p: c_int;
                let groupnumber: c_int = get(rcode, 1);
                if groupnumber == 0 {
                    rgroup = codestart;
                } else {
                    let mut search_from: PCRE2_SPTR = codestart;
                    rgroup = core::ptr::null();
                    i = 0;
                    p = start;
                    while i < ccount {
                        if groupnumber == rc[p as usize].groupnumber {
                            rgroup = rc[p as usize].group;
                            break;
                        }

                        /* Group n+1 must always start to the right of group n, so
                        we can save search time below when the new group number is
                        greater than any of the previously found groups. */

                        if groupnumber > rc[p as usize].groupnumber {
                            search_from = rc[p as usize].group;
                        }
                        i += 1;
                        p = (p + 1) & 7;
                    }

                    if rgroup.is_null() {
                        rgroup = crate::find_bracket::find_bracket(search_from, utf, groupnumber);
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

                put(rcode, 1, rgroup.offset_from(codestart) as i32);

                rcode = find_recurse(rcode.add(1 + LINK_SIZE), utf);
            }
        }

        /* DEBUG_CALL_PRINTINT is skipped. */

        /* Unless disabled, check whether any single character iterators can be
        auto-possessified. */

        if errorcode == 0 && (optim_flags & PCRE2_OPTIM_AUTO_POSSESS) != 0 {
            let temp: *mut PCRE2_UCHAR = codestart;
            let possessify_rc = crate::auto_possess::auto_possessify(temp, &cb);
            if possessify_rc != 0 {
                errorcode = ERR80;
                cb.erroroffset = 0;
            }
        }

        /* Failed to compile, or error while post-processing. */

        if errorcode != 0 {
            return had_cb_error(
                errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                ccontext, stack_parsed_pattern.as_mut_ptr(), stack_groupinfo.as_mut_ptr(),
            );
        }

        /* Successful compile. If the anchored option was not passed, set it if
        we can determine that the pattern is anchored ... */

        if ((*re).overall_options & PCRE2_ANCHORED) == 0 {
            let dotstar_anchor = ((optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0) as BOOL;
            if is_anchored(codestart, 0, &mut cb, 0, FALSE, dotstar_anchor) != FALSE {
                (*re).overall_options |= PCRE2_ANCHORED;
            }
        }

        /* Set up the first code unit or startline flag, the required code unit,
        and then study the pattern. */

        if (optim_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0 {
            let mut minminlength: c_int = 0; /* For minimal minlength from first/required CU */
            let study_rc: c_int;

            /* If we do not have a first code unit, see if there is one that is
            asserted. */

            if firstcuflags >= REQ_NONE {
                let mut assertedcuflags: u32 = 0;
                let assertedcu = find_firstassertedcu(codestart, &mut assertedcuflags, 0);
                if assertedcuflags < REQ_NONE && assertedcu != reqcu {
                    firstcu = assertedcu;
                    firstcuflags = assertedcuflags;
                }
            }

            /* Save the data for a first code unit. */

            if firstcuflags < REQ_NONE {
                (*re).first_codeunit = firstcu;
                (*re).flags |= PCRE2_FIRSTSET;
                minminlength += 1;

                /* Handle caseless first code units. */

                if (firstcuflags & REQ_CASELESS) != 0 {
                    if firstcu < 128 || (utf == FALSE && ucp == FALSE && firstcu < 255) {
                        if *cb.fcc.add(firstcu as usize) as u32 != firstcu {
                            (*re).flags |= PCRE2_FIRSTCASELESS;
                        }
                    }
                    /* SUPPORT_UNICODE, PCRE2_CODE_UNIT_WIDTH == 8 */
                    else if ucp != FALSE && utf == FALSE && ucd_othercase(firstcu) != firstcu {
                        (*re).flags |= PCRE2_FIRSTCASELESS;
                    }
                }
            }
            /* When there is no first code unit, for non-anchored patterns, see if
            we can set the PCRE2_STARTLINE flag. */
            else if ((*re).overall_options & PCRE2_ANCHORED) == 0 {
                let dotstar_anchor = ((optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0) as BOOL;
                if is_startline(codestart, 0, &mut cb, 0, FALSE, dotstar_anchor) != FALSE {
                    (*re).flags |= PCRE2_STARTLINE;
                }
            }

            /* Handle the "required code unit", if one is set. */

            if reqcuflags < REQ_NONE {
                /* PCRE2_CODE_UNIT_WIDTH == 8 */
                if ((*re).overall_options & PCRE2_UTF) == 0 /* Not UTF */
                    || firstcuflags >= REQ_NONE /* First not set */
                    || (firstcu & 0x80) == 0 /* First is ASCII */
                    || (reqcu & 0x80) == 0
                /* Req is ASCII */
                {
                    minminlength += 1;
                }

                /* In the case of an anchored pattern, set up the value only if it
                follows a variable length item in the pattern. */

                if ((*re).overall_options & PCRE2_ANCHORED) == 0
                    || (reqcuflags & REQ_VARY) != 0
                {
                    (*re).last_codeunit = reqcu;
                    (*re).flags |= PCRE2_LASTSET;

                    /* Handle caseless required code units as for first code units
                    (above). */

                    if (reqcuflags & REQ_CASELESS) != 0 {
                        if reqcu < 128 || (utf == FALSE && ucp == FALSE && reqcu < 255) {
                            if *cb.fcc.add(reqcu as usize) as u32 != reqcu {
                                (*re).flags |= PCRE2_LASTCASELESS;
                            }
                        }
                        /* SUPPORT_UNICODE, PCRE2_CODE_UNIT_WIDTH == 8 */
                        else if ucp != FALSE && utf == FALSE && ucd_othercase(reqcu) != reqcu {
                            (*re).flags |= PCRE2_LASTCASELESS;
                        }
                    }
                }
            }

            /* Study the compiled pattern to set up information such as a bitmap of
            starting code units and a minimum matching length. */

            study_rc = crate::study::study(re);
            if study_rc != 0 {
                errorcode = ERR31;
                cb.erroroffset = 0;
                return had_cb_error(
                    errorcode, pattern, erroroffset, errorptr, re, &mut cb, utf, patlen,
                    ccontext, stack_parsed_pattern.as_mut_ptr(), stack_groupinfo.as_mut_ptr(),
                );
            }

            /* If study() set a bitmap of starting code units, it implies a minimum
            length of at least one. */

            if ((*re).flags & PCRE2_FIRSTMAPSET) != 0 && minminlength == 0 {
                minminlength = 1;
            }

            /* If the minimum length set (or not set) by study() is less than the
            minimum implied by required code units, override it. */

            if ((*re).minlength as c_int) < minminlength {
                (*re).minlength = minminlength as u16;
            }
        } /* End of start-of-match optimizations. */

        /* Control ends up here in all cases. */

        /* SUPPORT_UNICODE: all items must be freed. */
        debug_assert!(cb.first_data.is_null());

        exit_cleanup(
            re,
            &mut cb,
            ccontext,
            stack_parsed_pattern.as_mut_ptr(),
            stack_groupinfo.as_mut_ptr(),
        )
    }
}

/* ---------------- goto-target helper functions ---------------- */

/* The EXIT label: free heap-allocated scratch buffers and return `re` (which is
NULL after an error). */
#[inline]
unsafe fn exit_cleanup(
    re: *mut pcre2_real_code,
    cb: *mut compile_block,
    ccontext: *mut pcre2_compile_context,
    stack_parsed_pattern: *mut u32,
    stack_groupinfo: *mut u32,
) -> *mut pcre2_code {
    unsafe {
        if (*cb).parsed_pattern != stack_parsed_pattern {
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
        if (*cb).groupinfo != stack_groupinfo {
            ((*ccontext).memctl.free.unwrap())(
                (*cb).groupinfo as *mut c_void,
                (*ccontext).memctl.memory_data,
            );
        }
        re
    }
}

/* The HAD_ERROR label: set the error code, free `re`, free the compile-data
list, then go to EXIT. Because the callers of this helper have already set
`*erroroffset` (or come from HAD_EARLY_ERROR/HAD_CB_ERROR), this only performs
the tail of the C flow. */
#[inline]
unsafe fn had_error_tail(
    errorcode: c_int,
    errorptr: *mut c_int,
    mut re: *mut pcre2_real_code,
    cb: *mut compile_block,
) {
    unsafe {
        *errorptr = errorcode;
        pcre2_code_free_8(re);
        re = core::ptr::null_mut();
        let _ = re;

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
    }
}

/* HAD_ERROR: offset already set. Runs HAD_ERROR then EXIT. */
#[inline]
unsafe fn had_error(
    errorcode: c_int,
    errorptr: *mut c_int,
    re: *mut pcre2_real_code,
    cb: *mut compile_block,
) -> *mut pcre2_code {
    unsafe {
        had_error_tail(errorcode, errorptr, re, cb);
        /* After HAD_ERROR the C code sets re = NULL and goes to EXIT. The scratch
        buffers are always the stack ones on this path (errors reaching here occur
        before or independently of the heap allocations for parsed_pattern etc.),
        but to be safe we reproduce EXIT with the cb's current buffers. */
        exit_cleanup(
            core::ptr::null_mut(),
            cb,
            (*cb).cx,
            /* Recompute "stack" markers: freeing is guarded by inequality with
            these pointers. We cannot recover them here, so pass the cb's own
            pointers to make the equality tests true and skip freeing, matching
            the fact that on the valid_utf early path nothing was heap-allocated. */
            (*cb).parsed_pattern,
            (*cb).groupinfo,
        )
    }
}

/* HAD_EARLY_ERROR: compute erroroffset from ptr, then HAD_ERROR, then EXIT. */
#[inline]
unsafe fn had_early_error(
    errorcode: c_int,
    ptr: PCRE2_SPTR,
    pattern: PCRE2_SPTR,
    _patlen: PCRE2_SIZE,
    _utf: BOOL,
    erroroffset: *mut PCRE2_SIZE,
    errorptr: *mut c_int,
    re: *mut pcre2_real_code,
    cb: *mut compile_block,
) -> *mut pcre2_code {
    unsafe {
        /* PCRE2_ASSERT(ptr >= pattern && ptr <= pattern + patlen); */
        *erroroffset = ptr.offset_from(pattern) as PCRE2_SIZE;
        had_error(errorcode, errorptr, re, cb)
    }
}

/* HAD_CB_ERROR: compute erroroffset from cb->erroroffset (HAD_CB_ERROR sets
ptr = pattern + cb->erroroffset), then HAD_EARLY_ERROR, then HAD_ERROR, then
EXIT. This path may have heap-allocated scratch buffers, so EXIT must use the
real stack markers. */
#[inline]
unsafe fn had_cb_error(
    errorcode: c_int,
    pattern: PCRE2_SPTR,
    erroroffset: *mut PCRE2_SIZE,
    errorptr: *mut c_int,
    re: *mut pcre2_real_code,
    cb: *mut compile_block,
    _utf: BOOL,
    patlen: PCRE2_SIZE,
    ccontext: *mut pcre2_compile_context,
    stack_parsed_pattern: *mut u32,
    stack_groupinfo: *mut u32,
) -> *mut pcre2_code {
    unsafe {
        let ptr = pattern.add((*cb).erroroffset);
        /* PCRE2_ASSERT(ptr >= pattern && ptr <= pattern + patlen); */
        let _ = patlen;
        *erroroffset = ptr.offset_from(pattern) as PCRE2_SIZE;

        had_error_tail(errorcode, errorptr, re, cb);

        exit_cleanup(
            core::ptr::null_mut(),
            cb,
            ccontext,
            stack_parsed_pattern,
            stack_groupinfo,
        )
    }
}
