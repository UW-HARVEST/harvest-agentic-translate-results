//! Translation of `pcre2_substitute.c`.

use crate::internal::*;
use crate::match_data::{
    pcre2_get_mark_8, pcre2_get_ovector_count_8, pcre2_get_ovector_pointer_8,
    pcre2_match_data_create_8, pcre2_match_data_create_from_pattern_8, pcre2_match_data_free_8,
};
use crate::match_next::pcre2_next_match_8;
use crate::ord2utf::_pcre2_ord2utf_8;
use crate::string_utils::{_pcre2_strcmp_c8_8, _pcre2_strlen_8};
use crate::substring::{pcre2_substring_length_bynumber_8, pcre2_substring_nametable_scan_8};
use crate::tables::_pcre2_ucp_gentype;
use crate::valid_utf::_pcre2_valid_utf_8;
use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// External functions written by other agents.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    /// `PRIV(check_escape)` — declared in src/compile.rs.
    #[link_name = "_pcre2_check_escape_8"]
    fn PRIV_check_escape(
        ptrptr: *mut PCRE2_SPTR,
        ptrend: PCRE2_SPTR,
        chptr: *mut u32,
        errorcodeptr: *mut c_int,
        options: u32,
        xoptions: u32,
        bracount: u32,
        isclass: BOOL,
        cb: *mut compile_block,
    ) -> c_int;

    /// `pcre2_match()` — declared in src/pcre2_match.rs.
    #[link_name = "pcre2_match_8"]
    fn pcre2_match_8(
        code: *const pcre2_code,
        subject: PCRE2_SPTR,
        length: PCRE2_SIZE,
        start_offset: PCRE2_SIZE,
        options: u32,
        match_data: *mut pcre2_match_data,
        mcontext: *mut pcre2_match_context,
    ) -> c_int;
}

// ---------------------------------------------------------------------------
// Character constants (EBCDIC not configured; ASCII values).
// ---------------------------------------------------------------------------

const CHAR_BACKSLASH: u8 = b'\\';
const CHAR_E: u8 = b'E';
const CHAR_L: u8 = b'L';
const CHAR_l: u8 = b'l';
const CHAR_U: u8 = b'U';
const CHAR_u: u8 = b'u';
const CHAR_RIGHT_CURLY_BRACKET: u8 = b'}';
const CHAR_LEFT_CURLY_BRACKET: u8 = b'{';
const CHAR_COLON: u8 = b':';
const CHAR_DOLLAR_SIGN: u8 = b'$';
const CHAR_AMPERSAND: u8 = b'&';
const CHAR_GRAVE_ACCENT: u8 = b'`';
const CHAR_APOSTROPHE: u8 = b'\'';
const CHAR_UNDERSCORE: u8 = b'_';
const CHAR_PLUS: u8 = b'+';
const CHAR_MINUS: u8 = b'-';
const CHAR_LESS_THAN_SIGN: u8 = b'<';
const CHAR_GREATER_THAN_SIGN: u8 = b'>';
const CHAR_ASTERISK: u8 = b'*';
const CHAR_0: u8 = b'0';
const CHAR_9: u8 = b'9';
const CHAR_BS: u32 = 0x08; // backspace
const CHAR_VT: u32 = 0x0b; // vertical tab

// `STRING_MARK` — the C macro expands to the 8-bit string "MARK".
const STRING_MARK: &[u8; 5] = b"MARK\0";

// The subset of ucp_* general-category values used here.
const ucp_L: u32 = crate::consts::ucp_L;
const ucp_Nd: u32 = crate::consts::ucp_Nd;
const ucp_Lu: u32 = crate::consts::ucp_Lu;
const ucp_Ll: u32 = crate::consts::ucp_Ll;

const PTR_STACK_SIZE: usize = 20;

const SUBSTITUTE_OPTIONS: u32 = (PCRE2_SUBSTITUTE_EXTENDED
    | PCRE2_SUBSTITUTE_GLOBAL
    | PCRE2_SUBSTITUTE_LITERAL
    | PCRE2_SUBSTITUTE_MATCHED
    | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
    | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY
    | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
    | PCRE2_SUBSTITUTE_UNSET_EMPTY) as u32;

const PCRE2_MATCHEDBY_DFA_INTERPRETER: u8 = 1;

// `SIZE_MAX` / `~(PCRE2_SIZE)0`.
const SIZE_MAX: PCRE2_SIZE = usize::MAX;

// ---------------------------------------------------------------------------
// Find end of substitute text
// ---------------------------------------------------------------------------

unsafe fn find_text_end(
    code: *const pcre2_code,
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    last: BOOL,
) -> c_int {
    unsafe {
        let mut rc: c_int = 0;
        let mut nestlevel: u32 = 0;
        let mut literal: BOOL = FALSE;
        let mut ptr: PCRE2_SPTR = *ptrptr;

        'outer: while ptr < ptrend {
            if literal != FALSE {
                if *ptr.add(0) == CHAR_BACKSLASH && ptr < ptrend.sub(1) && *ptr.add(1) == CHAR_E {
                    literal = FALSE;
                    ptr = ptr.add(1);
                }
            } else if *ptr == CHAR_RIGHT_CURLY_BRACKET {
                if nestlevel == 0 {
                    break 'outer;
                }
                nestlevel -= 1;
            } else if *ptr == CHAR_COLON && last == FALSE && nestlevel == 0 {
                break 'outer;
            } else if *ptr == CHAR_DOLLAR_SIGN {
                if ptr < ptrend.sub(1) && *ptr.add(1) == CHAR_LEFT_CURLY_BRACKET {
                    nestlevel += 1;
                    ptr = ptr.add(1);
                }
            } else if *ptr == CHAR_BACKSLASH {
                let erc: c_int;
                let mut errorcode: c_int = 0;
                let mut ch: u32 = 0;
                let esc_end_ptr: PCRE2_SPTR;

                if ptr < ptrend.sub(1) {
                    match *ptr.add(1) {
                        c if c == CHAR_L || c == CHAR_l || c == CHAR_U || c == CHAR_u => {
                            ptr = ptr.add(1);
                            ptr = ptr.add(1);
                            continue 'outer;
                        }
                        _ => {}
                    }
                }

                ptr = ptr.add(1); // Must point after \
                let mut ptr_mut = ptr;
                erc = PRIV_check_escape(
                    &mut ptr_mut,
                    ptrend,
                    &mut ch,
                    &mut errorcode,
                    (*code).overall_options,
                    (*code).extra_options,
                    (*code).top_bracket as u32,
                    FALSE,
                    ptr::null_mut(),
                );
                ptr = ptr_mut;
                if errorcode != 0 {
                    rc = PCRE2_ERROR_BADREPESCAPE as c_int;
                    break 'outer;
                }

                esc_end_ptr = ptr;
                ptr = ptr.sub(1); // Rewind by one, for-loop will increment it

                if erc == 0
                    || erc == ESC_b as c_int
                    || erc == ESC_v as c_int
                    || erc == ESC_E as c_int
                {
                    // Data character / isolated \E ignored
                } else if erc == ESC_Q as c_int {
                    literal = TRUE;
                } else if erc == ESC_g as c_int {
                    // \g<name> form; be lenient.
                } else {
                    if erc < 0 {
                        // capture group reference
                    } else {
                        ptr = esc_end_ptr;
                        rc = PCRE2_ERROR_BADREPESCAPE as c_int;
                        break 'outer;
                    }
                }
            }

            ptr = ptr.add(1);
        }

        if ptr >= ptrend {
            rc = PCRE2_ERROR_REPMISSINGBRACE as c_int; // Terminator not found
        }

        *ptrptr = ptr;
        rc
    }
}

// ---------------------------------------------------------------------------
// Validate group name
// ---------------------------------------------------------------------------

unsafe fn read_name_subst(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    ctypes: *const u8,
) -> BOOL {
    unsafe {
        let mut ptr: PCRE2_SPTR = *ptrptr;
        let nameptr: PCRE2_SPTR = ptr;

        if ptr >= ptrend {
            *ptrptr = ptr;
            return FALSE;
        }

        if utf != FALSE {
            while ptr < ptrend {
                let c = GETCHAR(ptr);
                let ty = UCD_CHARTYPE(c);
                if ty != ucp_Nd
                    && _pcre2_ucp_gentype[ty as usize] != ucp_L
                    && c != CHAR_UNDERSCORE as u32
                {
                    break;
                }
                ptr = ptr.add(1);
                FORWARDCHARTEST(&mut ptr, ptrend);
            }
        } else {
            while ptr < ptrend
                && MAX_255(*ptr as u32)
                && (*ctypes.add(*ptr as usize) as i64 & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        if ptr.offset_from(nameptr) > MAX_NAME_SIZE as isize {
            *ptrptr = ptr;
            return FALSE;
        }

        if ptr == nameptr {
            *ptrptr = ptr;
            return FALSE;
        }

        *ptrptr = ptr;
        TRUE
    }
}

// ---------------------------------------------------------------------------
// Case transformations
// ---------------------------------------------------------------------------

const PCRE2_SUBSTITUTE_CASE_NONE: c_int = 0;
// 1, 2, 3 are LOWER, UPPER, TITLE_FIRST.
const PCRE2_SUBSTITUTE_CASE_LOWER: c_int = crate::consts::PCRE2_SUBSTITUTE_CASE_LOWER as c_int;
const PCRE2_SUBSTITUTE_CASE_UPPER: c_int = crate::consts::PCRE2_SUBSTITUTE_CASE_UPPER as c_int;
const PCRE2_SUBSTITUTE_CASE_TITLE_FIRST: c_int =
    crate::consts::PCRE2_SUBSTITUTE_CASE_TITLE_FIRST as c_int;
const PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST: c_int = 4;

#[derive(Clone, Copy)]
struct case_state {
    to_case: c_int,
    single_char: BOOL,
}

/// Helper to estimate case-transform size inflation.
fn pessimistic_case_inflation(len: PCRE2_SIZE) -> PCRE2_SIZE {
    (len >> 3) + 10
}

/// Case transformation behaviour if no callout is passed.
unsafe fn default_substitute_case_callout(
    input: PCRE2_SPTR,
    input_len: PCRE2_SIZE,
    output: *mut PCRE2_UCHAR,
    output_cap: PCRE2_SIZE,
    state: *mut case_state,
    code: *const pcre2_code,
) -> PCRE2_SIZE {
    unsafe {
        let mut input = input;
        let input_end: PCRE2_SPTR = input.add(input_len);
        let mut output = output;
        let mut output_cap = output_cap;
        let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
        let next_to_upper: BOOL;
        let rest_to_upper: BOOL;
        let mut next_to_upper_v: bool;
        let rest_to_upper_v: bool;
        let single_char: BOOL;
        let mut overflow: BOOL = FALSE;
        let mut written: PCRE2_SIZE = 0;

        let utf = ((*code).overall_options & PCRE2_UTF as u32) != 0;
        let ucp = ((*code).overall_options & PCRE2_UCP as u32) != 0;

        if input_len == 0 {
            return 0;
        }

        match (*state).to_case {
            PCRE2_SUBSTITUTE_CASE_LOWER | PCRE2_SUBSTITUTE_CASE_UPPER => {
                next_to_upper = if (*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER {
                    TRUE
                } else {
                    FALSE
                };
                rest_to_upper = next_to_upper;
            }
            PCRE2_SUBSTITUTE_CASE_TITLE_FIRST => {
                next_to_upper = TRUE;
                rest_to_upper = FALSE;
                (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
            }
            PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST => {
                next_to_upper = FALSE;
                rest_to_upper = TRUE;
                (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
            }
            _ => {
                return 0;
            }
        }

        next_to_upper_v = next_to_upper != FALSE;
        rest_to_upper_v = rest_to_upper != FALSE;

        single_char = (*state).single_char;
        if single_char != FALSE {
            (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
        }

        while input < input_end {
            let mut ch: u32 = GETCHARINCTEST(&mut input, utf);
            let chlen: u32;

            if (utf || ucp) && ch >= 128 {
                let ty = UCD_CHARTYPE(ch);
                if _pcre2_ucp_gentype[ty as usize] == ucp_L
                    && ty != (if next_to_upper_v { ucp_Lu } else { ucp_Ll })
                {
                    ch = UCD_OTHERCASE(ch);
                }
            } else if MAX_255(ch) {
                if (*((*code).tables.add(
                    (cbits_offset + (if next_to_upper_v { cbit_upper } else { cbit_lower })) as usize
                        + (ch as usize / 8),
                )) & (1u8 << (ch % 8)))
                    == 0
                {
                    ch = *((*code).tables.add(fcc_offset as usize + ch as usize)) as u32;
                }
            }

            if utf {
                chlen = _pcre2_ord2utf_8(ch, temp.as_mut_ptr());
            } else {
                temp[0] = ch as u8;
                chlen = 1;
            }

            if overflow == FALSE && (chlen as PCRE2_SIZE) <= output_cap {
                c_memcpy(
                    output as *mut c_void,
                    temp.as_ptr() as *const c_void,
                    CU2BYTES(chlen as usize),
                );
                output = output.add(chlen as usize);
                output_cap -= chlen as PCRE2_SIZE;
            } else {
                overflow = TRUE;
            }

            if chlen as PCRE2_SIZE > SIZE_MAX - written {
                return SIZE_MAX;
            }
            written += chlen as PCRE2_SIZE;

            next_to_upper_v = rest_to_upper_v;

            if single_char != FALSE {
                let rest_len: PCRE2_SIZE = input_end.offset_from(input) as PCRE2_SIZE;

                if overflow == FALSE && rest_len <= output_cap {
                    c_memcpy(
                        output as *mut c_void,
                        input as *const c_void,
                        CU2BYTES(rest_len),
                    );
                }

                if rest_len > SIZE_MAX - written {
                    return SIZE_MAX;
                }
                written += rest_len;

                return written;
            }
        }

        written
    }
}

/// Helper to perform the call to the substitute_case_callout.
unsafe fn do_case_copy(
    input_output: *mut PCRE2_UCHAR,
    input_len: PCRE2_SIZE,
    output_cap: PCRE2_SIZE,
    state: *mut case_state,
    utf: BOOL,
    substitute_case_callout: SubstituteCaseCalloutFn,
    substitute_case_callout_data: *mut c_void,
) -> PCRE2_SIZE {
    unsafe {
        let input: PCRE2_SPTR = input_output;
        let output: *mut PCRE2_UCHAR = input_output;
        let mut rc: PCRE2_SIZE;
        let rc2: PCRE2_SIZE;
        let ch1_to_case: c_int;
        let rest_to_case: c_int;
        let mut ch1: [PCRE2_UCHAR; 6] = [0; 6];
        let ch1_len: PCRE2_SIZE;
        let mut rest: PCRE2_SPTR;
        let rest_len: PCRE2_SIZE;
        let mut ch1_overflow: BOOL = FALSE;
        let mut rest_overflow: BOOL = FALSE;

        let callout = substitute_case_callout.unwrap();

        match (*state).to_case {
            PCRE2_SUBSTITUTE_CASE_LOWER
            | PCRE2_SUBSTITUTE_CASE_UPPER
            | PCRE2_SUBSTITUTE_CASE_TITLE_FIRST => {
                if (*state).single_char == FALSE {
                    rc = callout(
                        input,
                        input_len,
                        output,
                        output_cap,
                        (*state).to_case,
                        substitute_case_callout_data,
                    );

                    if (*state).to_case == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST {
                        (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                    }

                    return rc;
                }

                ch1_to_case = (*state).to_case;
                rest_to_case = PCRE2_SUBSTITUTE_CASE_NONE;
            }
            PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST => {
                ch1_to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                rest_to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
            }
            _ => {
                return 0;
            }
        }

        // Identify the leading character.
        {
            let mut ch_end: PCRE2_SPTR = input;
            let _ch: u32 = GETCHARINCTEST(&mut ch_end, utf != FALSE);
            ch1_len = ch_end.offset_from(input) as PCRE2_SIZE;
            c_memcpy(
                ch1.as_mut_ptr() as *mut c_void,
                input as *const c_void,
                CU2BYTES(ch1_len),
            );
        }

        rest = input.add(ch1_len);
        rest_len = input_len - ch1_len;

        // Transform just ch1.
        {
            let mut ch1_cap: PCRE2_SIZE = ch1_len; // First attempt uses space vacated by ch1.
            let max_ch1_cap: PCRE2_SIZE = output_cap - rest_len;

            loop {
                rc = callout(
                    ch1.as_ptr(),
                    ch1_len,
                    output,
                    ch1_cap,
                    ch1_to_case,
                    substitute_case_callout_data,
                );
                if rc == SIZE_MAX {
                    return rc;
                }

                if rc <= ch1_cap {
                    break;
                }

                if rc > max_ch1_cap {
                    ch1_overflow = TRUE;
                    break;
                }

                // Move the rest to the right, to make room for expanding ch1.
                c_memmove(
                    input_output.add(rc) as *mut c_void,
                    rest as *const c_void,
                    CU2BYTES(rest_len),
                );
                rest = input.add(rc);

                ch1_cap = rc;
            }
        }

        if rest_to_case == PCRE2_SUBSTITUTE_CASE_NONE {
            if ch1_overflow == FALSE {
                c_memmove(
                    output.add(rc) as *mut c_void,
                    rest as *const c_void,
                    CU2BYTES(rest_len),
                );
            }
            rc2 = rest_len;

            (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
        } else {
            let mut dummy: [PCRE2_UCHAR; 1] = [0; 1];

            let mut r2 = callout(
                rest,
                rest_len,
                if ch1_overflow != FALSE {
                    dummy.as_mut_ptr()
                } else {
                    output.add(rc)
                },
                if ch1_overflow != FALSE {
                    0
                } else {
                    output_cap - rc
                },
                rest_to_case,
                substitute_case_callout_data,
            );
            if r2 == SIZE_MAX {
                return r2;
            }

            if ch1_overflow == FALSE && r2 > output_cap - rc {
                rest_overflow = TRUE;
            }

            if ch1_overflow != FALSE && r2 < rest_len {
                r2 = rest_len;
            }

            rc2 = r2;
            (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
        }

        let _ = rest_overflow;

        if rc2 > SIZE_MAX - rc {
            return SIZE_MAX;
        }

        rc + rc2
    }
}

// ---------------------------------------------------------------------------
// Match and substitute
// ---------------------------------------------------------------------------

/// `pcre2_substitute()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substitute_8(
    code: *const pcre2_code,
    mut subject: PCRE2_SPTR,
    mut length: PCRE2_SIZE,
    mut start_offset: PCRE2_SIZE,
    mut options: u32,
    mut match_data: *mut pcre2_match_data,
    mcontext: *mut pcre2_match_context,
    mut replacement: PCRE2_SPTR,
    mut rlength: PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    blength: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut rc: c_int = 0;
        let mut subs: c_int;
        let ovector_count: u32;
        let mut goptions: u32 = 0;
        let mut suboptions: u32;
        let mut internal_match_data: *mut pcre2_match_data = ptr::null_mut();
        let mut escaped_literal: BOOL = FALSE;
        let mut overflowed: BOOL = FALSE;
        let mut use_existing_match: BOOL;
        let replacement_only: BOOL;
        let utf: bool = ((*code).overall_options & PCRE2_UTF as u32) != 0;
        let partial: bool =
            (options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT) as u32) != 0;
        let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
        let mut null_str: [PCRE2_UCHAR; 1] = [0xcd];
        let original_subject: PCRE2_SPTR = subject;
        let mut ptr: PCRE2_SPTR;
        let mut repend: PCRE2_SPTR = ptr::null();
        let mut extra_needed: PCRE2_SIZE = 0;
        let mut buff_offset: PCRE2_SIZE;
        let buff_length: PCRE2_SIZE;
        let mut lengthleft: PCRE2_SIZE;
        let mut fraglength: PCRE2_SIZE;
        let ovector: *mut PCRE2_SIZE;
        let mut ovecsave: [PCRE2_SIZE; 2] = [0, 0];
        let mut scb: pcre2_substitute_callout_block = core::mem::zeroed();
        let mut sub_start_extra_needed: PCRE2_SIZE;
        let mut substitute_case_callout: SubstituteCaseCalloutFn = None;
        let mut substitute_case_callout_data: *mut c_void = ptr::null_mut();

        // General initialization.
        buff_offset = 0;
        buff_length = *blength;
        lengthleft = buff_length;
        *blength = PCRE2_UNSET;

        if !mcontext.is_null() {
            substitute_case_callout = (*mcontext).substitute_case_callout;
            substitute_case_callout_data = (*mcontext).substitute_case_callout_data;
        }

        // ---- Local control-flow helpers replicating the C gotos. ----------
        //
        // The C function uses several labels; we translate each `goto LABEL`
        // to `{ current_label = Label::LABEL; break/continue }` targeting an
        // outer dispatch. Given the complexity, we instead use a closure-free
        // structure with explicit control via macros and a final `'exit`
        // section.

        // Helper macros. `overflowed`, `extra_needed`, `lengthleft`,
        // `buff_offset` are captured by reference through the enclosing scope.

        // Control-flow dispatch enum (declared before the macros that use it).
        #[derive(Clone, Copy, PartialEq)]
        enum Flow {
            Running,
            Exit,      // goto EXIT
            NoRoom,    // goto NOROOM
            CaseError, // goto CASEERROR
            TooLarge,  // goto TOOLARGEREPLACE
        }
        let mut current = Flow::Running;

        macro_rules! checkmemcpy {
            ($from:expr, $length:expr) => {{
                let chkmc_length: PCRE2_SIZE = $length;
                if overflowed != FALSE {
                    if chkmc_length > SIZE_MAX - extra_needed {
                        rc = PCRE2_ERROR_TOOLARGEREPLACE as c_int;
                        current = Flow::Exit;
                    } else {
                        extra_needed += chkmc_length;
                    }
                } else if lengthleft < chkmc_length {
                    if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as u32) == 0 {
                        rc = PCRE2_ERROR_NOMEMORY as c_int;
                        current = Flow::Exit;
                    } else {
                        overflowed = TRUE;
                        extra_needed = chkmc_length - lengthleft;
                    }
                } else {
                    c_memcpy(
                        buffer.add(buff_offset) as *mut c_void,
                        ($from) as *const c_void,
                        CU2BYTES(chkmc_length),
                    );
                    buff_offset += chkmc_length;
                    lengthleft -= chkmc_length;
                }
            }};
        }

        // The initial checks that return directly (no cleanup needed).

        if partial && (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY as u32) == 0 {
            return PCRE2_ERROR_BADOPTION as c_int;
        }

        if replacement.is_null() {
            if rlength != 0 {
                return PCRE2_ERROR_NULL as c_int;
            }
            replacement = null_str.as_ptr();
        }

        if rlength == PCRE2_ZERO_TERMINATED {
            rlength = _pcre2_strlen_8(replacement);
        }
        repend = replacement.add(rlength);

        if subject.is_null() {
            if length != 0 {
                return PCRE2_ERROR_NULL as c_int;
            }
            subject = null_str.as_ptr();
        }

        if length == PCRE2_ZERO_TERMINATED {
            length = _pcre2_strlen_8(subject);
        }

        use_existing_match = if (options & PCRE2_SUBSTITUTE_MATCHED as u32) != 0 {
            TRUE
        } else {
            FALSE
        };
        replacement_only = if (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY as u32) != 0 {
            TRUE
        } else {
            FALSE
        };

        if use_existing_match != FALSE && match_data.is_null() {
            return PCRE2_ERROR_NULL as c_int;
        }

        if use_existing_match != FALSE {
            if (*match_data).rc < 0 && (*match_data).rc as i64 != PCRE2_ERROR_NOMATCH {
                return (*match_data).rc;
            }

            if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
                return PCRE2_ERROR_DFA_UFUNC as c_int;
            }

            if code != (*match_data).code {
                return PCRE2_ERROR_DIFFSUBSPATTERN as c_int;
            }

            if length != (*match_data).subject_length
                || !(original_subject == (*match_data).subject
                    || (((*match_data).flags as i64 & PCRE2_MD_COPIED_SUBJECT) != 0
                        && (length == 0
                            || c_memcmp(
                                subject as *const c_void,
                                (*match_data).subject as *const c_void,
                                CU2BYTES(length),
                            ) == 0)))
            {
                return PCRE2_ERROR_DIFFSUBSSUBJECT as c_int;
            }

            if start_offset != (*match_data).start_offset {
                return PCRE2_ERROR_DIFFSUBSOFFSET as c_int;
            }

            if (options & !(SUBSTITUTE_OPTIONS | PCRE2_NO_UTF_CHECK as u32))
                != ((*match_data).options & !(PCRE2_NO_UTF_CHECK as u32))
            {
                return PCRE2_ERROR_DIFFSUBSOPTIONS as c_int;
            }
        }

        if match_data.is_null() {
            let mut gcontext: pcre2_general_context = core::mem::zeroed();
            gcontext.memctl = if mcontext.is_null() {
                (*code).memctl
            } else {
                (*(mcontext as *const pcre2_real_match_context)).memctl
            };
            internal_match_data = pcre2_match_data_create_from_pattern_8(
                code,
                &mut gcontext as *mut pcre2_general_context,
            );
            match_data = internal_match_data;
            if internal_match_data.is_null() {
                return PCRE2_ERROR_NOMEMORY as c_int;
            }
        } else if use_existing_match != FALSE {
            let pairs: c_int;
            let mut gcontext: pcre2_general_context = core::mem::zeroed();
            gcontext.memctl = if mcontext.is_null() {
                (*code).memctl
            } else {
                (*(mcontext as *const pcre2_real_match_context)).memctl
            };
            pairs = if ((*code).top_bracket as c_int + 1) < (*match_data).oveccount as c_int {
                (*code).top_bracket as c_int + 1
            } else {
                (*match_data).oveccount as c_int
            };
            internal_match_data = pcre2_match_data_create_8(
                (*match_data).oveccount as u32,
                &mut gcontext as *mut pcre2_general_context,
            );
            if internal_match_data.is_null() {
                return PCRE2_ERROR_NOMEMORY as c_int;
            }
            c_memcpy(
                internal_match_data as *mut c_void,
                match_data as *const c_void,
                core::mem::offset_of!(pcre2_real_match_data, ovector)
                    + 2 * pairs as usize * core::mem::size_of::<PCRE2_SIZE>(),
            );
            (*internal_match_data).heapframes = ptr::null_mut();
            (*internal_match_data).heapframes_size = 0;
            (*internal_match_data).flags &= !(PCRE2_MD_COPIED_SUBJECT as u8);
            match_data = internal_match_data;
        }

        if !internal_match_data.is_null() {
            options &= !(PCRE2_COPY_MATCHED_SUBJECT as u32);
        }

        ovector = pcre2_get_ovector_pointer_8(match_data);
        ovector_count = pcre2_get_ovector_count_8(match_data);

        scb.version = 0;
        scb.input = subject;
        scb.output = buffer as PCRE2_SPTR;
        scb.ovector = ovector;

        // From here on, all early exits must go through the EXIT cleanup, so
        // we use a dispatch loop. Encode the check-UTF and subsequent flow.

        // Control-flow dispatch enum declared earlier (before macros).

        // Check UTF replacement string if necessary.
        if utf && (options & PCRE2_NO_UTF_CHECK as u32) == 0 {
            rc = _pcre2_valid_utf_8(replacement, rlength, &mut (*match_data).startchar);
            if rc != 0 {
                (*match_data).leftchar = 0;
                current = Flow::Exit;
            }
        }

        // The main body runs only if we're still Running.
        if current == Flow::Running {
            suboptions = options & SUBSTITUTE_OPTIONS;
            options &= !SUBSTITUTE_OPTIONS;

            if start_offset > length {
                (*match_data).leftchar = 0;
                rc = PCRE2_ERROR_BADOFFSET as c_int;
                current = Flow::Exit;
            } else {
                // NOTE: the C source has no assignment here; `rc` is simply left
                // to be set by the first pcre2_match() call. Clearing it
                // unconditionally would wipe out the BADOFFSET set just above.
                rc = 0;
            }

            // Copy up to the start offset, unless only the replacement is required.
            if current == Flow::Running && replacement_only == FALSE {
                checkmemcpy!(subject, start_offset);
            }

            subs = 0;

            // ---------------------------------------------------------------
            // Global substitution loop.
            // ---------------------------------------------------------------
            'global: while current == Flow::Running {
                let mut ptrstack: [PCRE2_SPTR; PTR_STACK_SIZE] = [ptr::null(); PTR_STACK_SIZE];
                let mut ptrstackptr: u32 = 0;
                let mut forcecase = case_state {
                    to_case: PCRE2_SUBSTITUTE_CASE_NONE,
                    single_char: FALSE,
                };
                let mut casestart_offset: PCRE2_SIZE = 0;
                let mut casestart_extra_needed: PCRE2_SIZE = 0;

                if use_existing_match != FALSE {
                    rc = (*match_data).rc;
                    use_existing_match = FALSE;
                } else {
                    rc = pcre2_match_8(
                        code,
                        subject,
                        length,
                        start_offset,
                        options | goptions,
                        match_data,
                        mcontext,
                    );
                }

                if utf {
                    options |= PCRE2_NO_UTF_CHECK as u32;
                }

                if rc as i64 == PCRE2_ERROR_NOMATCH {
                    break 'global;
                }

                if rc < 0 {
                    current = Flow::Exit;
                    break 'global;
                }

                if *ovector.add(1) < *ovector.add(0) || *ovector.add(0) < start_offset {
                    rc = PCRE2_ERROR_BADSUBSPATTERN as c_int;
                    current = Flow::Exit;
                    break 'global;
                }

                if subs > 0
                    && !(*ovector.add(1) > ovecsave[1]
                        || (*ovector.add(1) == *ovector.add(0)
                            && ovecsave[1] > ovecsave[0]
                            && *ovector.add(1) == ovecsave[1]))
                {
                    rc = PCRE2_ERROR_INTERNAL_DUPMATCH as c_int;
                    current = Flow::Exit;
                    break 'global;
                }

                ovecsave[0] = *ovector.add(0);
                ovecsave[1] = *ovector.add(1);

                if subs == c_int::MAX {
                    rc = PCRE2_ERROR_TOOMANYREPLACE as c_int;
                    current = Flow::Exit;
                    break 'global;
                }
                subs += 1;

                if rc == 0 {
                    rc = ovector_count as c_int;
                }
                fraglength = *ovector.add(0) - start_offset;
                if replacement_only == FALSE {
                    checkmemcpy!(subject.add(start_offset), fraglength);
                    if current != Flow::Running {
                        break 'global;
                    }
                }
                scb.output_offsets[0] = buff_offset;
                scb.oveccount = rc as u32;
                sub_start_extra_needed = extra_needed;

                // Process the replacement string.
                ptr = replacement;
                if (suboptions & PCRE2_SUBSTITUTE_LITERAL as u32) != 0 {
                    checkmemcpy!(ptr, rlength);
                    if current != Flow::Running {
                        break 'global;
                    }
                } else {
                    // -------------------------------------------------------
                    // Non-literal replacement scan.
                    // -------------------------------------------------------
                    'reploop: loop {
                        let mut ch: u32 = 0;
                        let mut chlen: u32;
                        let mut group: c_int;
                        let mut special: u32;
                        let mut text1_start: PCRE2_SPTR;
                        let mut text1_end: PCRE2_SPTR;
                        let mut text2_start: PCRE2_SPTR;
                        let mut text2_end: PCRE2_SPTR;
                        let mut name: [PCRE2_UCHAR; MAX_NAME_SIZE as usize + 1] =
                            [0; MAX_NAME_SIZE as usize + 1];

                        if ptr >= repend {
                            if ptrstackptr == 0 {
                                break 'reploop; // End of replacement string
                            }
                            ptrstackptr -= 1;
                            repend = ptrstack[ptrstackptr as usize];
                            ptrstackptr -= 1;
                            ptr = ptrstack[ptrstackptr as usize];
                            continue 'reploop;
                        }

                        // Inner sub-goto dispatch for $/group substitution.
                        // We implement GROUP_SUBSTITUTE / LITERAL_SUBSTITUTE /
                        // SUBPTR_SUBSTITUTE / BAD / BADESCAPE / PTREXIT / LOADLITERAL
                        // via an inner state machine.
                        #[derive(Clone, Copy, PartialEq)]
                        enum Rep {
                            Dollar,
                            GroupSubstitute,
                            LiteralSubstitute,
                            SubptrSubstitute,
                            LoadLiteral,
                            Bad,
                            BadEscape,
                            PtrExit,
                            NextIter, // continue 'reploop
                            Done,     // finished this iteration normally (fell through)
                        }

                        // Working variables shared across the inner states.
                        let mut subptr: PCRE2_SPTR = ptr::null();
                        let mut subptrend: PCRE2_SPTR = ptr::null();
                        let mut sublength: PCRE2_SIZE = 0;
                        let mut ch_start: PCRE2_SPTR = ptr::null();

                        group = -1;
                        special = 0;
                        text1_start = ptr::null();
                        text1_end = ptr::null();
                        text2_start = ptr::null();
                        text2_end = ptr::null();

                        let mut state: Rep;

                        // Determine which branch to take.
                        if escaped_literal != FALSE {
                            if *ptr.add(0) == CHAR_BACKSLASH
                                && ptr < repend.sub(1)
                                && *ptr.add(1) == CHAR_E
                            {
                                escaped_literal = FALSE;
                                ptr = ptr.add(2);
                                continue 'reploop;
                            }
                            state = Rep::LoadLiteral;
                        } else if *ptr == CHAR_DOLLAR_SIGN {
                            state = Rep::Dollar;
                        } else if (suboptions & PCRE2_SUBSTITUTE_EXTENDED as u32) != 0
                            && *ptr == CHAR_BACKSLASH
                        {
                            // ---- backslash processing in extended mode ----
                            let mut errorcode: c_int = 0;
                            let mut new_forcecase = case_state {
                                to_case: PCRE2_SUBSTITUTE_CASE_NONE,
                                single_char: FALSE,
                            };

                            if ptr < repend.sub(1) {
                                match *ptr.add(1) {
                                    c if c == CHAR_L => {
                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                                        new_forcecase.single_char = FALSE;
                                        ptr = ptr.add(2);
                                    }
                                    c if c == CHAR_l => {
                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                                        new_forcecase.single_char = TRUE;
                                        ptr = ptr.add(2);
                                        if ptr.add(2) < repend
                                            && *ptr.add(0) == CHAR_BACKSLASH
                                            && *ptr.add(1) == CHAR_U
                                        {
                                            new_forcecase.to_case =
                                                PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST;
                                            new_forcecase.single_char = FALSE;
                                            ptr = ptr.add(2);
                                        }
                                    }
                                    c if c == CHAR_U => {
                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
                                        new_forcecase.single_char = FALSE;
                                        ptr = ptr.add(2);
                                    }
                                    c if c == CHAR_u => {
                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                        new_forcecase.single_char = TRUE;
                                        ptr = ptr.add(2);
                                        if ptr.add(2) < repend
                                            && *ptr.add(0) == CHAR_BACKSLASH
                                            && *ptr.add(1) == CHAR_L
                                        {
                                            new_forcecase.to_case =
                                                PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                            new_forcecase.single_char = FALSE;
                                            ptr = ptr.add(2);
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            // SETFORCECASE handling, possibly entered via ESC_E.
                            let mut do_setforcecase =
                                new_forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE;

                            if !do_setforcecase {
                                ptr = ptr.add(1); // Point after \
                                let mut ptr_mut = ptr;
                                rc = PRIV_check_escape(
                                    &mut ptr_mut,
                                    repend,
                                    &mut ch,
                                    &mut errorcode,
                                    (*code).overall_options,
                                    (*code).extra_options,
                                    (*code).top_bracket as u32,
                                    FALSE,
                                    ptr::null_mut(),
                                );
                                ptr = ptr_mut;
                                if errorcode != 0 {
                                    state = Rep::BadEscape;
                                } else if rc == ESC_E as c_int {
                                    do_setforcecase = true;
                                    state = Rep::Done; // placeholder; handled below
                                } else if rc == ESC_Q as c_int {
                                    escaped_literal = TRUE;
                                    continue 'reploop;
                                } else if rc == 0
                                    || rc == ESC_b as c_int
                                    || rc == ESC_v as c_int
                                {
                                    if rc == ESC_b as c_int {
                                        ch = CHAR_BS;
                                    }
                                    if rc == ESC_v as c_int {
                                        ch = CHAR_VT;
                                    }

                                    if utf {
                                        chlen = _pcre2_ord2utf_8(ch, temp.as_mut_ptr());
                                    } else {
                                        temp[0] = ch as u8;
                                        chlen = 1;
                                    }

                                    if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                        && substitute_case_callout.is_none()
                                    {
                                        // CHECKCASECPY_DEFAULT(temp, chlen)
                                        let chkcc_length = chlen as PCRE2_SIZE;
                                        let chkcc_rc = default_substitute_case_callout(
                                            temp.as_ptr(),
                                            chkcc_length,
                                            buffer.add(buff_offset),
                                            if overflowed != FALSE { 0 } else { lengthleft },
                                            &mut forcecase,
                                            code,
                                        );
                                        if overflowed != FALSE {
                                            if chkcc_rc > SIZE_MAX - extra_needed {
                                                current = Flow::TooLarge;
                                                break 'reploop;
                                            }
                                            extra_needed += chkcc_rc;
                                        } else if lengthleft < chkcc_rc {
                                            if (suboptions
                                                & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as u32)
                                                == 0
                                            {
                                                current = Flow::NoRoom;
                                                break 'reploop;
                                            }
                                            overflowed = TRUE;
                                            extra_needed = chkcc_rc - lengthleft;
                                        } else {
                                            buff_offset += chkcc_rc;
                                            lengthleft -= chkcc_rc;
                                        }
                                    } else {
                                        checkmemcpy!(temp.as_ptr(), chlen as PCRE2_SIZE);
                                        if current != Flow::Running {
                                            break 'reploop;
                                        }
                                    }
                                    continue 'reploop;
                                } else if rc == ESC_g as c_int {
                                    // \g<name>
                                    let name_len: PCRE2_SIZE;
                                    let name_start: PCRE2_SPTR;

                                    if ptr >= repend || *ptr != CHAR_LESS_THAN_SIGN {
                                        state = Rep::BadEscape;
                                    } else {
                                        ptr = ptr.add(1);
                                        name_start = ptr;
                                        let mut ptr_mut = ptr;
                                        if read_name_subst(
                                            &mut ptr_mut,
                                            repend,
                                            if utf { TRUE } else { FALSE },
                                            (*code).tables.add(ctypes_offset as usize),
                                        ) == FALSE
                                        {
                                            ptr = ptr_mut;
                                            state = Rep::BadEscape;
                                        } else {
                                            ptr = ptr_mut;
                                            name_len = ptr.offset_from(name_start) as PCRE2_SIZE;
                                            if ptr >= repend || *ptr != CHAR_GREATER_THAN_SIGN {
                                                state = Rep::BadEscape;
                                            } else {
                                                ptr = ptr.add(1);
                                                special = 0;
                                                group = -1;
                                                c_memcpy(
                                                    name.as_mut_ptr() as *mut c_void,
                                                    name_start as *const c_void,
                                                    CU2BYTES(name_len),
                                                );
                                                name[name_len] = 0;
                                                state = Rep::GroupSubstitute;
                                            }
                                        }
                                    }
                                } else if rc < 0 {
                                    special = 0;
                                    group = -rc - 1;
                                    state = Rep::GroupSubstitute;
                                } else {
                                    state = Rep::BadEscape;
                                }
                            } else {
                                state = Rep::Done; // will process SETFORCECASE below
                            }

                            // SETFORCECASE processing (reached from \L\l\U\u or ESC_E).
                            if do_setforcecase {
                                if substitute_case_callout.is_some()
                                    && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                {
                                    // DELAYEDFORCECASE()
                                    let chars_outstanding: PCRE2_SIZE = (buff_offset
                                        - casestart_offset)
                                        + (extra_needed - casestart_extra_needed);
                                    if chars_outstanding > 0 {
                                        if overflowed != FALSE {
                                            let guess =
                                                pessimistic_case_inflation(chars_outstanding);
                                            if guess > SIZE_MAX - extra_needed {
                                                current = Flow::TooLarge;
                                                break 'reploop;
                                            }
                                            extra_needed += guess;
                                        } else {
                                            lengthleft += buff_offset - casestart_offset;
                                            buff_offset = casestart_offset;
                                            // CHECKCASECPY_CALLOUT(chars_outstanding)
                                            let chkcc_length = chars_outstanding;
                                            let chkcc_rc = do_case_copy(
                                                buffer.add(buff_offset),
                                                chkcc_length,
                                                lengthleft,
                                                &mut forcecase,
                                                if utf { TRUE } else { FALSE },
                                                substitute_case_callout,
                                                substitute_case_callout_data,
                                            );
                                            if chkcc_rc == SIZE_MAX {
                                                current = Flow::CaseError;
                                                break 'reploop;
                                            }
                                            if lengthleft < chkcc_rc {
                                                if (suboptions
                                                    & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as u32)
                                                    == 0
                                                {
                                                    current = Flow::NoRoom;
                                                    break 'reploop;
                                                }
                                                overflowed = TRUE;
                                                extra_needed = chkcc_rc - lengthleft;
                                            } else {
                                                buff_offset += chkcc_rc;
                                                lengthleft -= chkcc_rc;
                                            }
                                        }
                                    }
                                }

                                forcecase = new_forcecase;
                                casestart_offset = buff_offset;
                                casestart_extra_needed = extra_needed;
                                continue 'reploop;
                            }

                            // If we didn't set state above (only when do_setforcecase
                            // path was taken but not continued), fall through.
                            // state is already assigned in all non-continue paths.
                        } else {
                            state = Rep::LoadLiteral;
                        }

                        // ------ Inner state machine execution ------
                        loop {
                            match state {
                                Rep::Dollar => {
                                    let mut inparens: BOOL = FALSE;
                                    let mut inangle: BOOL = FALSE;
                                    let mut star: BOOL = FALSE;
                                    let mut next: PCRE2_UCHAR;

                                    ptr = ptr.add(1);
                                    if ptr >= repend {
                                        state = Rep::Bad;
                                        continue;
                                    }
                                    next = *ptr;
                                    if next == CHAR_DOLLAR_SIGN {
                                        state = Rep::LoadLiteral;
                                        continue;
                                    }

                                    special = 0;
                                    group = -1;

                                    if next == CHAR_AMPERSAND {
                                        ptr = ptr.add(1);
                                        group = 0;
                                        state = Rep::GroupSubstitute;
                                        continue;
                                    }
                                    if next == CHAR_GRAVE_ACCENT || next == CHAR_APOSTROPHE {
                                        ptr = ptr.add(1);

                                        rc = pcre2_substring_length_bynumber_8(
                                            match_data,
                                            0,
                                            &mut sublength,
                                        );
                                        if rc < 0 {
                                            state = Rep::PtrExit;
                                            continue;
                                        }

                                        if next == CHAR_GRAVE_ACCENT {
                                            subptr = subject;
                                            subptrend = subject.add(*ovector.add(0));
                                        } else {
                                            if partial {
                                                rc = PCRE2_ERROR_PARTIALSUBS as c_int;
                                                state = Rep::PtrExit;
                                                continue;
                                            }
                                            subptr = subject.add(*ovector.add(1));
                                            subptrend = subject.add(length);
                                        }

                                        state = Rep::SubptrSubstitute;
                                        continue;
                                    }
                                    if next == CHAR_UNDERSCORE {
                                        ptr = ptr.add(1);
                                        if partial {
                                            rc = PCRE2_ERROR_PARTIALSUBS as c_int;
                                            state = Rep::PtrExit;
                                            continue;
                                        }
                                        subptr = subject;
                                        subptrend = subject.add(length);
                                        state = Rep::SubptrSubstitute;
                                        continue;
                                    }
                                    if next == CHAR_PLUS
                                        && !(ptr.add(1) < repend
                                            && *ptr.add(1) == CHAR_LEFT_CURLY_BRACKET)
                                    {
                                        ptr = ptr.add(1);
                                        if (*code).top_bracket == 0 {
                                            if (suboptions
                                                & PCRE2_SUBSTITUTE_UNKNOWN_UNSET as u32)
                                                == 0
                                            {
                                                rc = PCRE2_ERROR_NOSUBSTRING as c_int;
                                                state = Rep::PtrExit;
                                                continue;
                                            }
                                            group = 0;
                                        } else {
                                            if (*match_data).oveccount
                                                < (*code).top_bracket + 1
                                            {
                                                rc = PCRE2_ERROR_UNAVAILABLE as c_int;
                                                state = Rep::PtrExit;
                                                continue;
                                            }
                                            group = (*code).top_bracket as c_int;
                                            while group > 0 {
                                                if *ovector.add(2 * group as usize) != PCRE2_UNSET {
                                                    break;
                                                }
                                                group -= 1;
                                            }
                                        }
                                        if group == 0 {
                                            if (suboptions
                                                & PCRE2_SUBSTITUTE_UNSET_EMPTY as u32)
                                                != 0
                                            {
                                                state = Rep::NextIter;
                                                continue;
                                            }
                                            rc = PCRE2_ERROR_UNSET as c_int;
                                            state = Rep::PtrExit;
                                            continue;
                                        }
                                        state = Rep::GroupSubstitute;
                                        continue;
                                    }

                                    if next == CHAR_LEFT_CURLY_BRACKET {
                                        ptr = ptr.add(1);
                                        if ptr >= repend {
                                            state = Rep::Bad;
                                            continue;
                                        }
                                        next = *ptr;
                                        inparens = TRUE;
                                    } else if next == CHAR_LESS_THAN_SIGN {
                                        ptr = ptr.add(1);
                                        if ptr >= repend {
                                            state = Rep::Bad;
                                            continue;
                                        }
                                        next = *ptr;
                                        inangle = TRUE;
                                    }

                                    if inangle == FALSE && next == CHAR_ASTERISK {
                                        ptr = ptr.add(1);
                                        if ptr >= repend {
                                            state = Rep::Bad;
                                            continue;
                                        }
                                        next = *ptr;
                                        star = TRUE;
                                    }

                                    if star == FALSE
                                        && inangle == FALSE
                                        && next >= CHAR_0
                                        && next <= CHAR_9
                                    {
                                        group = (next - CHAR_0) as c_int;
                                        loop {
                                            ptr = ptr.add(1);
                                            if ptr >= repend {
                                                break;
                                            }
                                            next = *ptr;
                                            if next < CHAR_0 || next > CHAR_9 {
                                                break;
                                            }
                                            group = group * 10 + (next - CHAR_0) as c_int;

                                            if group > (*code).top_bracket as c_int {
                                                if (suboptions
                                                    & PCRE2_SUBSTITUTE_UNKNOWN_UNSET as u32)
                                                    != 0
                                                {
                                                    loop {
                                                        ptr = ptr.add(1);
                                                        if !(ptr < repend
                                                            && *ptr >= CHAR_0
                                                            && *ptr <= CHAR_9)
                                                        {
                                                            break;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    rc = PCRE2_ERROR_NOSUBSTRING as c_int;
                                                    state = Rep::PtrExit;
                                                    break;
                                                }
                                            }
                                        }
                                        if state == Rep::PtrExit {
                                            continue;
                                        }
                                    } else {
                                        let name_len: PCRE2_SIZE;
                                        let name_start: PCRE2_SPTR = ptr;
                                        let mut ptr_mut = ptr;
                                        if read_name_subst(
                                            &mut ptr_mut,
                                            repend,
                                            if utf { TRUE } else { FALSE },
                                            (*code).tables.add(ctypes_offset as usize),
                                        ) == FALSE
                                        {
                                            ptr = ptr_mut;
                                            state = Rep::Bad;
                                            continue;
                                        }
                                        ptr = ptr_mut;
                                        name_len = ptr.offset_from(name_start) as PCRE2_SIZE;
                                        c_memcpy(
                                            name.as_mut_ptr() as *mut c_void,
                                            name_start as *const c_void,
                                            CU2BYTES(name_len),
                                        );
                                        name[name_len] = 0;
                                    }

                                    // In extended mode, recognize ${name:+..} / ${name:-..}.
                                    if inparens != FALSE {
                                        if (suboptions & PCRE2_SUBSTITUTE_EXTENDED as u32) != 0
                                            && star == FALSE
                                            && ptr < repend.sub(2)
                                            && *ptr == CHAR_COLON
                                        {
                                            ptr = ptr.add(1);
                                            special = *ptr as u32;
                                            if special != CHAR_PLUS as u32
                                                && special != CHAR_MINUS as u32
                                            {
                                                rc = PCRE2_ERROR_BADSUBSTITUTION as c_int;
                                                state = Rep::PtrExit;
                                                continue;
                                            }

                                            ptr = ptr.add(1);
                                            text1_start = ptr;
                                            let mut ptr_mut = ptr;
                                            rc = find_text_end(
                                                code,
                                                &mut ptr_mut,
                                                repend,
                                                if special == CHAR_MINUS as u32 {
                                                    TRUE
                                                } else {
                                                    FALSE
                                                },
                                            );
                                            ptr = ptr_mut;
                                            if rc != 0 {
                                                state = Rep::PtrExit;
                                                continue;
                                            }
                                            text1_end = ptr;

                                            if special == CHAR_PLUS as u32
                                                && *ptr == CHAR_COLON
                                            {
                                                ptr = ptr.add(1);
                                                text2_start = ptr;
                                                let mut ptr_mut = ptr;
                                                rc = find_text_end(
                                                    code, &mut ptr_mut, repend, TRUE,
                                                );
                                                ptr = ptr_mut;
                                                if rc != 0 {
                                                    state = Rep::PtrExit;
                                                    continue;
                                                }
                                                text2_end = ptr;
                                            }
                                        } else {
                                            if ptr >= repend
                                                || *ptr != CHAR_RIGHT_CURLY_BRACKET
                                            {
                                                rc = PCRE2_ERROR_REPMISSINGBRACE as c_int;
                                                state = Rep::PtrExit;
                                                continue;
                                            }
                                        }

                                        ptr = ptr.add(1);
                                    }

                                    if inangle != FALSE {
                                        if ptr >= repend || *ptr != CHAR_GREATER_THAN_SIGN {
                                            state = Rep::Bad;
                                            continue;
                                        }
                                        ptr = ptr.add(1);
                                    }

                                    // Have a syntactically valid group / *name.
                                    if star != FALSE {
                                        if _pcre2_strcmp_c8_8(
                                            name.as_ptr(),
                                            STRING_MARK.as_ptr() as *const c_char,
                                        ) == 0
                                        {
                                            let mark = pcre2_get_mark_8(match_data);
                                            if !mark.is_null() {
                                                fraglength = *mark.sub(1) as PCRE2_SIZE;
                                                if forcecase.to_case
                                                    != PCRE2_SUBSTITUTE_CASE_NONE
                                                    && substitute_case_callout.is_none()
                                                {
                                                    // CHECKCASECPY_DEFAULT(mark, fraglength)
                                                    let chkcc_length = fraglength;
                                                    let chkcc_rc =
                                                        default_substitute_case_callout(
                                                            mark,
                                                            chkcc_length,
                                                            buffer.add(buff_offset),
                                                            if overflowed != FALSE {
                                                                0
                                                            } else {
                                                                lengthleft
                                                            },
                                                            &mut forcecase,
                                                            code,
                                                        );
                                                    if overflowed != FALSE {
                                                        if chkcc_rc > SIZE_MAX - extra_needed {
                                                            current = Flow::TooLarge;
                                                            break 'reploop;
                                                        }
                                                        extra_needed += chkcc_rc;
                                                    } else if lengthleft < chkcc_rc {
                                                        if (suboptions
                                                            & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
                                                                as u32)
                                                            == 0
                                                        {
                                                            current = Flow::NoRoom;
                                                            break 'reploop;
                                                        }
                                                        overflowed = TRUE;
                                                        extra_needed = chkcc_rc - lengthleft;
                                                    } else {
                                                        buff_offset += chkcc_rc;
                                                        lengthleft -= chkcc_rc;
                                                    }
                                                } else {
                                                    checkmemcpy!(mark, fraglength);
                                                    if current != Flow::Running {
                                                        break 'reploop;
                                                    }
                                                }
                                            }
                                        } else {
                                            state = Rep::Bad;
                                            continue;
                                        }
                                        state = Rep::NextIter;
                                        continue;
                                    } else {
                                        state = Rep::GroupSubstitute;
                                        continue;
                                    }
                                }

                                Rep::GroupSubstitute => {
                                    if group < 0 {
                                        let mut first: PCRE2_SPTR = ptr::null();
                                        let mut last: PCRE2_SPTR = ptr::null();
                                        let mut entry: PCRE2_SPTR;
                                        rc = pcre2_substring_nametable_scan_8(
                                            code,
                                            name.as_ptr(),
                                            &mut first,
                                            &mut last,
                                        );
                                        if rc as i64 == PCRE2_ERROR_NOSUBSTRING
                                            && (suboptions
                                                & PCRE2_SUBSTITUTE_UNKNOWN_UNSET as u32)
                                                != 0
                                        {
                                            group = (*code).top_bracket as c_int + 1;
                                        } else {
                                            if rc < 0 {
                                                state = Rep::PtrExit;
                                                continue;
                                            }
                                            entry = first;
                                            while entry <= last {
                                                let ng = GET2(entry, 0);
                                                if ng < ovector_count {
                                                    if group < 0 {
                                                        group = ng as c_int;
                                                    }
                                                    if *ovector.add(ng as usize * 2)
                                                        != PCRE2_UNSET
                                                    {
                                                        group = ng as c_int;
                                                        break;
                                                    }
                                                }
                                                entry = entry.add(rc as usize);
                                            }

                                            if group < 0 {
                                                group = GET2(first, 0) as c_int;
                                            }
                                        }
                                    }

                                    rc = pcre2_substring_length_bynumber_8(
                                        match_data,
                                        group as u32,
                                        &mut sublength,
                                    );
                                    if rc < 0 {
                                        if rc as i64 == PCRE2_ERROR_NOSUBSTRING
                                            && (suboptions
                                                & PCRE2_SUBSTITUTE_UNKNOWN_UNSET as u32)
                                                != 0
                                        {
                                            rc = PCRE2_ERROR_UNSET as c_int;
                                        }
                                        if rc as i64 != PCRE2_ERROR_UNSET {
                                            state = Rep::PtrExit;
                                            continue;
                                        }
                                        if special == 0 {
                                            if (suboptions
                                                & PCRE2_SUBSTITUTE_UNSET_EMPTY as u32)
                                                != 0
                                            {
                                                state = Rep::NextIter;
                                                continue;
                                            }
                                            state = Rep::PtrExit;
                                            continue;
                                        }
                                    }

                                    if special != 0 {
                                        if special == CHAR_MINUS as u32 {
                                            if rc == 0 {
                                                state = Rep::LiteralSubstitute;
                                                continue;
                                            }
                                            text2_start = text1_start;
                                            text2_end = text1_end;
                                        }

                                        if ptrstackptr as usize >= PTR_STACK_SIZE {
                                            state = Rep::Bad;
                                            continue;
                                        }
                                        ptrstack[ptrstackptr as usize] = ptr;
                                        ptrstackptr += 1;
                                        ptrstack[ptrstackptr as usize] = repend;
                                        ptrstackptr += 1;

                                        if rc == 0 {
                                            ptr = text1_start;
                                            repend = text1_end;
                                        } else {
                                            ptr = text2_start;
                                            repend = text2_end;
                                        }
                                        state = Rep::NextIter;
                                        continue;
                                    }

                                    state = Rep::LiteralSubstitute;
                                    continue;
                                }

                                Rep::LiteralSubstitute => {
                                    subptr = subject.add(*ovector.add(group as usize * 2));
                                    subptrend = subject.add(*ovector.add(group as usize * 2 + 1));
                                    state = Rep::SubptrSubstitute;
                                    continue;
                                }

                                Rep::SubptrSubstitute => {
                                    let seg = subptrend.offset_from(subptr) as PCRE2_SIZE;
                                    if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                        && substitute_case_callout.is_none()
                                    {
                                        // CHECKCASECPY_DEFAULT(subptr, subptrend - subptr)
                                        let chkcc_length = seg;
                                        let chkcc_rc = default_substitute_case_callout(
                                            subptr,
                                            chkcc_length,
                                            buffer.add(buff_offset),
                                            if overflowed != FALSE { 0 } else { lengthleft },
                                            &mut forcecase,
                                            code,
                                        );
                                        if overflowed != FALSE {
                                            if chkcc_rc > SIZE_MAX - extra_needed {
                                                current = Flow::TooLarge;
                                                break 'reploop;
                                            }
                                            extra_needed += chkcc_rc;
                                        } else if lengthleft < chkcc_rc {
                                            if (suboptions
                                                & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as u32)
                                                == 0
                                            {
                                                current = Flow::NoRoom;
                                                break 'reploop;
                                            }
                                            overflowed = TRUE;
                                            extra_needed = chkcc_rc - lengthleft;
                                        } else {
                                            buff_offset += chkcc_rc;
                                            lengthleft -= chkcc_rc;
                                        }
                                    } else {
                                        checkmemcpy!(subptr, seg);
                                        if current != Flow::Running {
                                            break 'reploop;
                                        }
                                    }
                                    state = Rep::NextIter;
                                    continue;
                                }

                                Rep::LoadLiteral => {
                                    ch_start = ptr;
                                    let mut ptr_mut = ptr;
                                    let _ = GETCHARINCTEST(&mut ptr_mut, utf);
                                    ptr = ptr_mut;

                                    let seg = ptr.offset_from(ch_start) as PCRE2_SIZE;
                                    if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                        && substitute_case_callout.is_none()
                                    {
                                        // CHECKCASECPY_DEFAULT(ch_start, ptr - ch_start)
                                        let chkcc_length = seg;
                                        let chkcc_rc = default_substitute_case_callout(
                                            ch_start,
                                            chkcc_length,
                                            buffer.add(buff_offset),
                                            if overflowed != FALSE { 0 } else { lengthleft },
                                            &mut forcecase,
                                            code,
                                        );
                                        if overflowed != FALSE {
                                            if chkcc_rc > SIZE_MAX - extra_needed {
                                                current = Flow::TooLarge;
                                                break 'reploop;
                                            }
                                            extra_needed += chkcc_rc;
                                        } else if lengthleft < chkcc_rc {
                                            if (suboptions
                                                & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as u32)
                                                == 0
                                            {
                                                current = Flow::NoRoom;
                                                break 'reploop;
                                            }
                                            overflowed = TRUE;
                                            extra_needed = chkcc_rc - lengthleft;
                                        } else {
                                            buff_offset += chkcc_rc;
                                            lengthleft -= chkcc_rc;
                                        }
                                    } else {
                                        checkmemcpy!(ch_start, seg);
                                        if current != Flow::Running {
                                            break 'reploop;
                                        }
                                    }
                                    state = Rep::NextIter;
                                    continue;
                                }

                                Rep::Bad => {
                                    rc = PCRE2_ERROR_BADREPLACEMENT as c_int;
                                    state = Rep::PtrExit;
                                    continue;
                                }

                                Rep::BadEscape => {
                                    rc = PCRE2_ERROR_BADREPESCAPE as c_int;
                                    state = Rep::PtrExit;
                                    continue;
                                }

                                Rep::PtrExit => {
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    current = Flow::Exit;
                                    break 'reploop;
                                }

                                Rep::NextIter => {
                                    break; // break inner state loop -> continue 'reploop
                                }

                                Rep::Done => {
                                    break;
                                }
                            }
                        }

                        // Reached end of inner state machine for this char.
                        if current != Flow::Running {
                            break 'reploop;
                        }
                        // else loop back to 'reploop naturally.
                    } // end 'reploop

                    if current != Flow::Running {
                        break 'global;
                    }
                }

                // DELAYEDFORCECASE for trailing section.
                if substitute_case_callout.is_some()
                    && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                {
                    let chars_outstanding: PCRE2_SIZE = (buff_offset - casestart_offset)
                        + (extra_needed - casestart_extra_needed);
                    if chars_outstanding > 0 {
                        if overflowed != FALSE {
                            let guess = pessimistic_case_inflation(chars_outstanding);
                            if guess > SIZE_MAX - extra_needed {
                                current = Flow::TooLarge;
                                break 'global;
                            }
                            extra_needed += guess;
                        } else {
                            lengthleft += buff_offset - casestart_offset;
                            buff_offset = casestart_offset;
                            let chkcc_rc = do_case_copy(
                                buffer.add(buff_offset),
                                chars_outstanding,
                                lengthleft,
                                &mut forcecase,
                                if utf { TRUE } else { FALSE },
                                substitute_case_callout,
                                substitute_case_callout_data,
                            );
                            if chkcc_rc == SIZE_MAX {
                                current = Flow::CaseError;
                                break 'global;
                            }
                            if lengthleft < chkcc_rc {
                                if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as u32) == 0 {
                                    current = Flow::NoRoom;
                                    break 'global;
                                }
                                overflowed = TRUE;
                                extra_needed = chkcc_rc - lengthleft;
                            } else {
                                buff_offset += chkcc_rc;
                                lengthleft -= chkcc_rc;
                            }
                        }
                    }
                }

                // Handle the substitute callout.
                if !mcontext.is_null() && (*mcontext).substitute_callout.is_some() {
                    if overflowed == FALSE {
                        scb.subscount = subs as u32;
                        scb.output_offsets[1] = buff_offset;
                        rc = ((*mcontext).substitute_callout.unwrap())(
                            &mut scb,
                            (*mcontext).substitute_callout_data,
                        );

                        if rc != 0 {
                            let newlength: PCRE2_SIZE =
                                scb.output_offsets[1] - scb.output_offsets[0];
                            let oldlength: PCRE2_SIZE = *ovector.add(1) - *ovector.add(0);

                            buff_offset -= newlength;
                            lengthleft += newlength;
                            if replacement_only == FALSE {
                                checkmemcpy!(subject.add(*ovector.add(0)), oldlength);
                                if current != Flow::Running {
                                    break 'global;
                                }
                            }

                            if rc < 0 {
                                suboptions &= !(PCRE2_SUBSTITUTE_GLOBAL as u32);
                            }
                        }
                    } else {
                        let newlength_buf: PCRE2_SIZE = buff_offset - scb.output_offsets[0];
                        let newlength_extra: PCRE2_SIZE = extra_needed - sub_start_extra_needed;
                        let newlength: PCRE2_SIZE = if newlength_extra > SIZE_MAX - newlength_buf {
                            SIZE_MAX
                        } else {
                            newlength_buf + newlength_extra
                        };
                        let oldlength: PCRE2_SIZE = *ovector.add(1) - *ovector.add(0);

                        if oldlength > newlength {
                            let additional: PCRE2_SIZE = oldlength - newlength;
                            if additional > SIZE_MAX - extra_needed {
                                current = Flow::TooLarge;
                                break 'global;
                            }
                            extra_needed += additional;
                        }
                    }
                }

                // Exit global loop if not global, or next_match says we're done.
                if (suboptions & PCRE2_SUBSTITUTE_GLOBAL as u32) == 0
                    || pcre2_next_match_8(match_data, &mut start_offset, &mut goptions) == FALSE
                {
                    start_offset = *ovector.add(1);
                    break 'global;
                }

                debug_assert!(start_offset == *ovector.add(1));
            } // end 'global

            // Copy the rest of the subject unless not required.
            if current == Flow::Running {
                if replacement_only == FALSE {
                    fraglength = length - start_offset;
                    checkmemcpy!(subject.add(start_offset), fraglength);
                }
            }

            if current == Flow::Running {
                temp[0] = 0;
                checkmemcpy!(temp.as_ptr(), 1);
            }

            if current == Flow::Running {
                if overflowed != FALSE {
                    rc = PCRE2_ERROR_NOMEMORY as c_int;

                    if extra_needed > SIZE_MAX - buff_length {
                        current = Flow::TooLarge;
                    } else {
                        *blength = buff_length + extra_needed;
                    }
                } else {
                    rc = subs;
                    *blength = buff_offset - 1;
                }
            }
        }
        // If current != Running here, we jumped to EXIT (e.g. from the UTF
        // pre-check); `rc` already holds the value to return.

        // Resolve non-EXIT dispositions into rc, all funnel to EXIT cleanup.
        match current {
            Flow::NoRoom => {
                rc = PCRE2_ERROR_NOMEMORY as c_int;
            }
            Flow::CaseError => {
                rc = PCRE2_ERROR_REPLACECASE as c_int;
            }
            Flow::TooLarge => {
                rc = PCRE2_ERROR_TOOLARGEREPLACE as c_int;
            }
            _ => {}
        }

        // EXIT:
        if !internal_match_data.is_null() {
            pcre2_match_data_free_8(internal_match_data);
        } else {
            (*match_data).rc = rc;
        }
        rc
    }
}
