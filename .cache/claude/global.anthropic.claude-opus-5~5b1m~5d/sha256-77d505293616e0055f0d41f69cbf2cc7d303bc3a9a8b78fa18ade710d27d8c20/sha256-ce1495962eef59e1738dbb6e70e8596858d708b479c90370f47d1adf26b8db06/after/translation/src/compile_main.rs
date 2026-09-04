//! Translated from pcre2_compile.c, lines 10280-11350 (pcre2_compile).
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use crate::compile_tables::*;
use crate::compile::*;
use crate::compile_parse::*;
use crate::compile_branch::*;
use crate::compile_aux::*;
use core::ffi::{c_char, c_void};

/* #define GROUPINFO_DEFAULT_SIZE 256 (from the head of pcre2_compile.c) */
const GROUPINFO_DEFAULT_SIZE: usize = 256;

/* #define RSCAN_CACHE_SIZE 8 */
const RSCAN_CACHE_SIZE: usize = 8;

/* #define IS_DIGIT(x) ((x) >= CHAR_0 && (x) <= CHAR_9) */
macro_rules! IS_DIGIT {
    ($x:expr) => {
        (($x as u32) >= 0x30 && ($x as u32) <= 0x39)
    };
}

/* State labels used to emulate the C gotos at the end of pcre2_compile(). */
const L_MAIN: u32 = 0;
const L_HAD_CB_ERROR: u32 = 1;
const L_HAD_EARLY_ERROR: u32 = 2;
const L_HAD_ERROR: u32 = 3;
const L_EXIT: u32 = 4;

/*************************************************
*        Compile a Regular Expression            *
*************************************************/

/* This function comprises the memory management for compiling a pattern. It
calls other functions to do the compilation.

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
pub unsafe extern "C" fn pcre2_compile_8(mut pattern: PCRE2_SPTR, mut patlen: PCRE2_SIZE, mut options: u32, errorptr: *mut i32, erroroffset: *mut PCRE2_SIZE, mut ccontext: *mut pcre2_real_compile_context) -> *mut pcre2_real_code {
let mut utf: BOOL = FALSE;                  /* Set TRUE for UTF mode */
let mut ucp: BOOL = FALSE;                  /* Set TRUE for UCP mode */
let mut has_lookbehind: BOOL = FALSE;       /* Set TRUE if a lookbehind is found */
let mut zero_terminated: BOOL = FALSE;      /* Set TRUE for zero-terminated pattern */
let mut re: *mut pcre2_real_code = core::ptr::null_mut();   /* What we will return */
let mut cb: compile_block = core::mem::zeroed();            /* "Static" compile-time data */
let cbp: *mut compile_block = core::ptr::addr_of_mut!(cb);
let mut tables: *const u8 = core::ptr::null();              /* Char tables base pointer */

let mut null_str: [PCRE2_UCHAR; 1] = [0xcd];  /* Dummy for handling null inputs */
let mut code: *mut PCRE2_UCHAR = core::ptr::null_mut();      /* Current pointer in compiled code */
let mut codestart: *mut PCRE2_UCHAR = core::ptr::null_mut(); /* Start of compiled code */
let mut ptr: PCRE2_SPTR = core::ptr::null();                 /* Current pointer in pattern */
let mut pptr: *mut u32 = core::ptr::null_mut();              /* Current pointer in parsed pattern */

let mut length: PCRE2_SIZE = 1;             /* Allow for final END opcode */
let mut usedlength: PCRE2_SIZE;             /* Actual length used */
let mut re_blocksize: PCRE2_SIZE;           /* Size of memory block */
let mut parsed_size_needed: PCRE2_SIZE;     /* Needed for parsed pattern */

let mut firstcuflags: u32 = 0;
let mut reqcuflags: u32 = 0;                /* Type of first/req code unit */
let mut firstcu: u32 = 0;
let mut reqcu: u32 = 0;                     /* Value of first/req code unit */
let mut setflags: u32 = 0;                  /* NL and BSR set flags */
let mut xoptions: u32;                      /* Flags from context, modified */

let mut skipatstart: u32;                   /* When checking (*UTF) etc */
let mut limit_heap: u32 = u32::MAX;
let mut limit_match: u32 = u32::MAX;        /* Unset match limits */
let mut limit_depth: u32 = u32::MAX;

let mut newline: i32 = 0;                   /* Unset; can be set by the pattern */
let mut bsr: i32 = 0;                       /* Unset; can be set by the pattern */
let mut errorcode: i32 = 0;                 /* Initialize to avoid compiler warn */
let mut regexrc: i32;                       /* Return from compile */

let mut i: u32;                             /* Local loop counter */

/* Enable all optimizations by default. */
let mut optim_flags: u32 = if !ccontext.is_null() { (*ccontext).optimization_flags }
                           else { PCRE2_OPTIMIZATION_ALL };

/* Comments at the head of this file explain about these variables. */

let mut stack_groupinfo: [u32; GROUPINFO_DEFAULT_SIZE] = [0; GROUPINFO_DEFAULT_SIZE];
let mut stack_parsed_pattern: [u32; PARSED_PATTERN_DEFAULT_SIZE] = [0; PARSED_PATTERN_DEFAULT_SIZE];
let mut named_groups: [named_group; NAMED_GROUP_LIST_SIZE] = core::mem::zeroed();

/* The workspace is used in different ways in the different compiling phases.
It needs to be 16-bit aligned for the preliminary parsing scan. */

let mut c16workspace: [u32; C16_WORK_SIZE] = [0; C16_WORK_SIZE];
let cworkspace: *mut PCRE2_UCHAR = c16workspace.as_mut_ptr() as *mut PCRE2_UCHAR;

let mut state: u32 = L_MAIN;

'sm: loop { match state {

L_MAIN => {

/* -------------- Check arguments and set up the pattern ----------------- */

/* There must be error code and offset pointers. */

if errorptr.is_null()
  {
  if !erroroffset.is_null() { *erroroffset = 0; }
  return core::ptr::null_mut();
  }
if erroroffset.is_null()
  {
  if !errorptr.is_null() { *errorptr = ERR120; }
  return core::ptr::null_mut();
  }
*errorptr = ERR0;
*erroroffset = 0;

/* There must be a pattern, but NULL is allowed with zero length. */

if pattern.is_null()
  {
  if patlen == 0
    { pattern = null_str.as_mut_ptr() as PCRE2_SPTR; }
  else
    {
    *errorptr = ERR16;
    return core::ptr::null_mut();
    }
  }

/* A NULL compile context means "use a default context" */

if ccontext.is_null()
  { ccontext = core::ptr::addr_of_mut!(crate::context::_pcre2_default_compile_context_8); }

/* PCRE2_MATCH_INVALID_UTF implies UTF */

if (options & PCRE2_MATCH_INVALID_UTF) != 0 { options |= PCRE2_UTF; }

/* Check that all undefined public option bits are zero. */

if (options & !PUBLIC_COMPILE_OPTIONS) != 0 ||
    ((*ccontext).extra_options & !PUBLIC_COMPILE_EXTRA_OPTIONS) != 0
  {
  *errorptr = ERR17;
  return core::ptr::null_mut();
  }

if (options & PCRE2_LITERAL) != 0 &&
    ((options & !PUBLIC_LITERAL_COMPILE_OPTIONS) != 0 ||
     ((*ccontext).extra_options & !PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS) != 0)
  {
  *errorptr = ERR92;
  return core::ptr::null_mut();
  }

/* A zero-terminated pattern is indicated by the special length value
PCRE2_ZERO_TERMINATED. Check for an overlong pattern. */

zero_terminated = (patlen == PCRE2_ZERO_TERMINATED) as BOOL;
if zero_terminated != 0
  { patlen = crate::string_utils::_pcre2_strlen_8(pattern); }
let _ = zero_terminated; /* Silence compiler; only used if Valgrind enabled */

if patlen > (*ccontext).max_pattern_length
  {
  *errorptr = ERR88;
  return core::ptr::null_mut();
  }

/* Optimization flags in 'options' can override those in the compile context.
This is because some options to disable optimizations were added before the
optimization flags word existed, and we need to continue supporting them
for backwards compatibility. */

if (options & PCRE2_NO_AUTO_POSSESS) != 0
  { optim_flags &= !PCRE2_OPTIM_AUTO_POSSESS; }
if (options & PCRE2_NO_DOTSTAR_ANCHOR) != 0
  { optim_flags &= !PCRE2_OPTIM_DOTSTAR_ANCHOR; }
if (options & PCRE2_NO_START_OPTIMIZE) != 0
  { optim_flags &= !PCRE2_OPTIM_START_OPTIMIZE; }

/* From here on, all returns from this function should end up going via the
EXIT label. */


/* ------------ Initialize the "static" compile data -------------- */

tables = if !(*ccontext).tables.is_null() { (*ccontext).tables }
         else { crate::chartables::_pcre2_default_tables_8.as_ptr() };

cb.lcc = tables.add(lcc_offset);          /* Individual */
cb.fcc = tables.add(fcc_offset);          /*   character */
cb.cbits = tables.add(cbits_offset);      /*      tables */
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
cb.max_lookbehind = 0;                                  /* Max encountered */
cb.max_varlookbehind = (*ccontext).max_varlookbehind;   /* Limit */
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
cb.workspace_size = COMPILE_WORK_SIZE;
cb.first_data = core::ptr::null_mut();
cb.last_data = core::ptr::null_mut();
/* #ifdef SUPPORT_WIDE_CHARS */
cb.char_lists_size = 0;

/* Maximum back reference and backref bitmap. The bitmap records up to 31 back
references to help in deciding whether (.*) can be treated as anchored or not.
*/

cb.top_backref = 0;
cb.backref_map = 0;

/* Escape sequences \1 to \9 are always back references, but as they are only
two characters long, only two elements can be used in the parsed_pattern
vector. The first contains the reference, and we'd like to use the second to
record the offset in the pattern, so that forward references to non-existent
groups can be diagnosed later with an offset. However, on 64-bit systems,
PCRE2_SIZE won't fit. Instead, we have a vector of offsets for the first
occurrence of \1 to \9, indexed by the second parsed_pattern value. All other
references have enough space for the offset to be put into the parsed pattern.
*/

i = 0;
while i < 10 { cb.small_ref_offset[i as usize] = PCRE2_UNSET; i += 1; }


/* --------------- Start looking at the pattern --------------- */

/* Unless PCRE2_LITERAL is set, check for global one-time option settings at
the start of the pattern, and remember the offset to the actual regex. */

xoptions = (*ccontext).extra_options;
ptr = pattern;
skipatstart = 0;

if (options & PCRE2_LITERAL) == 0
  {
  while patlen - skipatstart as usize >= 2 &&
        *ptr.add(skipatstart as usize) == b'(' /* CHAR_LEFT_PARENTHESIS */ &&
        *ptr.add(skipatstart as usize + 1) == b'*' /* CHAR_ASTERISK */
    {
    i = 0;
    while (i as usize) < PSO_LIST_COUNT
      {
      let p: *const pso = pso_list.as_ptr().add(i as usize);

      if patlen - skipatstart as usize - 2 >= (*p).length as usize &&
          crate::string_utils::_pcre2_strncmp_c8_8(ptr.add(skipatstart as usize + 2),
            (*p).name as *const c_char, (*p).length as usize) == 0
        {
        let mut c: u32;
        let mut pp: u32;

        skipatstart += (*p).length as u32 + 2;
        match (*p).type_ as u32
          {
          PSO_OPT =>
          {
          cb.external_options |= (*p).value;
          }

          PSO_XOPT =>
          {
          xoptions |= (*p).value;
          }

          PSO_FLG =>
          {
          setflags |= (*p).value;
          }

          PSO_NL =>
          {
          newline = (*p).value as i32;
          setflags |= PCRE2_NL_SET;
          }

          PSO_BSR =>
          {
          bsr = (*p).value as i32;
          setflags |= PCRE2_BSR_SET;
          }

          PSO_LIMM | PSO_LIMD | PSO_LIMH =>
          {
          c = 0;
          pp = skipatstart;
          while (pp as usize) < patlen && IS_DIGIT!(*ptr.add(pp as usize))
            {
            if c > u32::MAX / 10 - 1 { break; }   /* Integer overflow */
            c = c*10 + (*ptr.add(pp as usize) as u32 - 0x30 /* CHAR_0 */); pp += 1;
            }
          if pp as usize >= patlen || pp == skipatstart ||
             *ptr.add(pp as usize) != b')' /* CHAR_RIGHT_PARENTHESIS */
            {
            errorcode = ERR60;
            ptr = ptr.add(pp as usize);
            utf = FALSE;  /* Used by HAD_EARLY_ERROR */
            /* goto HAD_EARLY_ERROR */
            state = L_HAD_EARLY_ERROR; continue 'sm;
            }
          if (*p).type_ as u32 == PSO_LIMH { limit_heap = c; }
            else if (*p).type_ as u32 == PSO_LIMM { limit_match = c; }
            else { limit_depth = c; }
          pp += 1; skipatstart = pp;
          }

          PSO_OPTMZ =>
          {
          optim_flags &= !((*p).value);

          /* For backward compatibility the three original VERBs to disable
          optimizations need to also update the corresponding bit in the
          external options. */

          match (*p).value
            {
            PCRE2_OPTIM_AUTO_POSSESS =>
            {
            cb.external_options |= PCRE2_NO_AUTO_POSSESS;
            }

            PCRE2_OPTIM_DOTSTAR_ANCHOR =>
            {
            cb.external_options |= PCRE2_NO_DOTSTAR_ANCHOR;
            }

            PCRE2_OPTIM_START_OPTIMIZE =>
            {
            cb.external_options |= PCRE2_NO_START_OPTIMIZE;
            }

            _ => {}
            }
          }

          /* LCOV_EXCL_START */
          _ =>
          {
          /* All values in the enum need an explicit entry for this switch
          but until a better way to prevent coding mistakes is invented keep
          a catch all that triggers a debug build assert as a failsafe */
          /* PCRE2_DEBUG_UNREACHABLE() */
          }
          /* LCOV_EXCL_STOP */
          }
        break;   /* Out of the table scan loop */
        }
      i += 1;
      }
    if i as usize >= PSO_LIST_COUNT { break; }   /* Out of pso loop */
    }
    /* PCRE2_ASSERT(skipatstart <= patlen); */
  }

/* End of pattern-start options; advance to start of real regex. */

ptr = ptr.add(skipatstart as usize);

/* Can't support UTF or UCP if PCRE2 was built without Unicode support.
(#ifndef SUPPORT_UNICODE - not compiled) */

/* Check UTF. We have the original options in 'options', with that value as
modified by (*UTF) etc in cb->external_options. */

utf = ((cb.external_options & PCRE2_UTF) != 0) as BOOL;
if utf != 0
  {
  if (options & PCRE2_NEVER_UTF) != 0
    {
    errorcode = ERR74;
    /* goto HAD_EARLY_ERROR */
    state = L_HAD_EARLY_ERROR; continue 'sm;
    }
  if (options & PCRE2_NO_UTF_CHECK) == 0 &&
       { errorcode = crate::valid_utf::_pcre2_valid_utf_8(pattern, patlen, erroroffset); errorcode != 0 }
    {
    /* goto HAD_ERROR - offset was set by valid_utf() */
    state = L_HAD_ERROR; continue 'sm;
    }

  /* #if PCRE2_CODE_UNIT_WIDTH == 16 surrogate-escape check: not compiled */
  }

/* Check UCP lockout. */

ucp = ((cb.external_options & PCRE2_UCP) != 0) as BOOL;
if ucp != 0 && (cb.external_options & PCRE2_NEVER_UCP) != 0
  {
  errorcode = ERR75;
  /* goto HAD_EARLY_ERROR */
  state = L_HAD_EARLY_ERROR; continue 'sm;
  }

/* PCRE2_EXTRA_TURKISH_CASING checks */

if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0
  {
  if utf == 0 && ucp == 0
    {
    errorcode = ERR104;
    /* goto HAD_EARLY_ERROR */
    state = L_HAD_EARLY_ERROR; continue 'sm;
    }

  /* #if PCRE2_CODE_UNIT_WIDTH == 8 */
  if utf == 0
    {
    errorcode = ERR105;
    /* goto HAD_EARLY_ERROR */
    state = L_HAD_EARLY_ERROR; continue 'sm;
    }

  if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
    {
    errorcode = ERR106;
    /* goto HAD_EARLY_ERROR */
    state = L_HAD_EARLY_ERROR; continue 'sm;
    }
  }

/* Process the BSR setting. */

if bsr == 0 { bsr = (*ccontext).bsr_convention as i32; }

/* Process the newline setting. */

if newline == 0 { newline = (*ccontext).newline_convention as i32; }
cb.nltype = NLTYPE_FIXED;
match newline as u32
  {
  PCRE2_NEWLINE_CR =>
  {
  cb.nllen = 1;
  cb.nl[0] = 0x0d;  /* CHAR_CR */
  }

  PCRE2_NEWLINE_LF =>
  {
  cb.nllen = 1;
  cb.nl[0] = 0x0a;  /* CHAR_NL */
  }

  PCRE2_NEWLINE_NUL =>
  {
  cb.nllen = 1;
  cb.nl[0] = 0x00;  /* CHAR_NUL */
  }

  PCRE2_NEWLINE_CRLF =>
  {
  cb.nllen = 2;
  cb.nl[0] = 0x0d;  /* CHAR_CR */
  cb.nl[1] = 0x0a;  /* CHAR_NL */
  }

  PCRE2_NEWLINE_ANY =>
  {
  cb.nltype = NLTYPE_ANY;
  }

  PCRE2_NEWLINE_ANYCRLF =>
  {
  cb.nltype = NLTYPE_ANYCRLF;
  }

  /* LCOV_EXCL_START */
  _ =>
  {
  /* PCRE2_DEBUG_UNREACHABLE() */
  errorcode = ERR56;
  /* goto HAD_EARLY_ERROR */
  state = L_HAD_EARLY_ERROR; continue 'sm;
  }
  /* LCOV_EXCL_STOP */
  }

/* Pre-scan the pattern to do two things: (1) Discover the named groups and
their numerical equivalents, so that this information is always available for
the remaining processing. (2) At the same time, parse the pattern and put a
processed version into the parsed_pattern vector. This has escapes interpreted
and comments removed (amongst other things). */

/* Ensure that the parsed pattern buffer is big enough. For many smaller
patterns the vector on the stack (which was set up above) can be used. */

parsed_size_needed = max_parsed_pattern(ptr, cb.end_pattern, utf, options) as PCRE2_SIZE;

/* Allow for 2x uint32_t at the start and 2 at the end, for
PCRE2_EXTRA_MATCH_WORD or PCRE2_EXTRA_MATCH_LINE (which are exclusive). */

if ((*ccontext).extra_options &
     (PCRE2_EXTRA_MATCH_WORD|PCRE2_EXTRA_MATCH_LINE)) != 0
  { parsed_size_needed += 4; }

/* When PCRE2_AUTO_CALLOUT is set we allow for one callout at the end. */

if (options & PCRE2_AUTO_CALLOUT) != 0
  { parsed_size_needed += 4; }

parsed_size_needed += 1;  /* For the final META_END */

if parsed_size_needed > PARSED_PATTERN_DEFAULT_SIZE
  {
  let heap_parsed_pattern: *mut u32 = ((*ccontext).memctl.malloc.unwrap())(
    parsed_size_needed * core::mem::size_of::<u32>(), (*ccontext).memctl.memory_data) as *mut u32;
  if heap_parsed_pattern.is_null()
    {
    *errorptr = ERR21;
    /* goto EXIT */
    state = L_EXIT; continue 'sm;
    }
  cb.parsed_pattern = heap_parsed_pattern;
  }
cb.parsed_pattern_end = cb.parsed_pattern.add(parsed_size_needed);

/* Do the parsing scan. */

errorcode = parse_regex(ptr, cb.external_options, xoptions,
  core::ptr::addr_of_mut!(has_lookbehind), cbp);
if errorcode != 0 { /* goto HAD_CB_ERROR */ state = L_HAD_CB_ERROR; continue 'sm; }

/* If there are any lookbehinds, scan the parsed pattern to figure out their
lengths. */

if has_lookbehind != 0
  {
  let mut loopcount: i32 = 0;
  if cb.bracount as usize >= GROUPINFO_DEFAULT_SIZE/2
    {
    cb.groupinfo = ((*ccontext).memctl.malloc.unwrap())(
      (2 * (cb.bracount + 1)) as usize * core::mem::size_of::<u32>(),
      (*ccontext).memctl.memory_data) as *mut u32;
    if cb.groupinfo.is_null()
      {
      errorcode = ERR21;
      cb.erroroffset = 0;
      /* goto HAD_CB_ERROR */
      state = L_HAD_CB_ERROR; continue 'sm;
      }
    }
  core::ptr::write_bytes(cb.groupinfo as *mut u8, 0,
    (2 * cb.bracount + 1) as usize * core::mem::size_of::<u32>());
  errorcode = check_lookbehinds(cb.parsed_pattern, core::ptr::null_mut(),
    core::ptr::null_mut(), cbp, core::ptr::addr_of_mut!(loopcount));
  if errorcode != 0 { /* goto HAD_CB_ERROR */ state = L_HAD_CB_ERROR; continue 'sm; }
  }

/* #ifdef DEBUG_SHOW_PARSED / DEBUG_SHOW_CAPTURES: not compiled */

/* Pretend to compile the pattern while actually just accumulating the amount
of memory required in the 'length' variable. */

cb.erroroffset = patlen;   /* For any subsequent errors that do not set it */
pptr = cb.parsed_pattern;
code = cworkspace;
*code = OP_BRA as u8;

let _ = compile_regex(cb.external_options, xoptions,
   core::ptr::addr_of_mut!(code), core::ptr::addr_of_mut!(pptr),
   core::ptr::addr_of_mut!(errorcode), 0,
   core::ptr::addr_of_mut!(firstcu), core::ptr::addr_of_mut!(firstcuflags),
   core::ptr::addr_of_mut!(reqcu), core::ptr::addr_of_mut!(reqcuflags),
   core::ptr::null_mut(), core::ptr::null_mut(),
   cbp, core::ptr::addr_of_mut!(length));

if errorcode != 0 { /* goto HAD_CB_ERROR - offset is in cb.erroroffset */
  state = L_HAD_CB_ERROR; continue 'sm; }

/* This should be caught in compile_regex(), but just in case... */

/* PCRE2_ASSERT((cb.char_lists_size & 0x3) == 0); */
if length > MAX_PATTERN_SIZE ||
    MAX_PATTERN_SIZE - length < (cb.char_lists_size / core::mem::size_of::<PCRE2_UCHAR>())
  {
  errorcode = ERR20;
  cb.erroroffset = 0;
  /* goto HAD_CB_ERROR */
  state = L_HAD_CB_ERROR; continue 'sm;
  }

/* Compute the size of, then, if not too large, get and initialize the data
block for storing the compiled pattern and names table. */

re_blocksize =
  CU2BYTES!(cb.names_found as PCRE2_SIZE * cb.name_entry_size as PCRE2_SIZE);

/* #if defined SUPPORT_WIDE_CHARS */
if cb.char_lists_size != 0
  {
  /* #if PCRE2_CODE_UNIT_WIDTH != 32
  Align to 32 bit first. This ensures the
  allocated area will also be 32 bit aligned. */
  re_blocksize = CLIST_ALIGN_TO!(re_blocksize, core::mem::size_of::<u32>()) as PCRE2_SIZE;
  re_blocksize += cb.char_lists_size;
  }

re_blocksize += CU2BYTES!(length);

if re_blocksize > (*ccontext).max_pattern_compiled_length
  {
  errorcode = ERR101;
  cb.erroroffset = 0;
  /* goto HAD_CB_ERROR */
  state = L_HAD_CB_ERROR; continue 'sm;
  }

re_blocksize += core::mem::size_of::<pcre2_real_code>();
re = ((*ccontext).memctl.malloc.unwrap())(re_blocksize, (*ccontext).memctl.memory_data)
       as *mut pcre2_real_code;
if re.is_null()
  {
  errorcode = ERR21;
  cb.erroroffset = 0;
  /* goto HAD_CB_ERROR */
  state = L_HAD_CB_ERROR; continue 'sm;
  }

/* The compiler may put padding at the end of the pcre2_real_code structure in
order to round it up to a multiple of 4 or 8 bytes. This means that when a
compiled pattern is copied (for example, when serialized) undefined bytes are
read, and this annoys debuggers such as valgrind. To avoid this, we explicitly
write to the last 8 bytes of the structure before setting the fields. */

core::ptr::write_bytes((re as *mut u8).add(core::mem::size_of::<pcre2_real_code>() - 8), 0, 8);
(*re).memctl = (*ccontext).memctl;
(*re).tables = tables;
(*re).executable_jit = core::ptr::null_mut();
core::ptr::write_bytes(core::ptr::addr_of_mut!((*re).start_bitmap) as *mut u8, 0,
  32 * core::mem::size_of::<u8>());
(*re).blocksize = re_blocksize;
(*re).code_start = re_blocksize - CU2BYTES!(length);
(*re).magic_number = MAGIC_NUMBER;
(*re).compile_options = options;
(*re).overall_options = cb.external_options;
(*re).extra_options = xoptions;
(*re).flags = (8/8) as u32 | cb.external_flags | setflags;  /* PCRE2_CODE_UNIT_WIDTH/8 */
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

/* The basic block is immediately followed by the name table, and the compiled
code follows after that. */

codestart = (re as *mut u8).add((*re).code_start) as *mut PCRE2_UCHAR;

/* Update the compile data block for the actual compile. */

cb.parens_depth = 0;
cb.assert_depth = 0;
cb.lastcapture = 0;
cb.name_table = (re as *mut u8).add(core::mem::size_of::<pcre2_real_code>()) as *mut PCRE2_UCHAR;
cb.start_code = codestart;
cb.req_varyopt = 0;
cb.had_accept = FALSE;
cb.had_pruneorskip = FALSE;
/* #ifdef SUPPORT_WIDE_CHARS */
cb.char_lists_size = 0;


/* If any named groups were found, create the name/number table from the list
created in the pre-pass. */

if cb.names_found > 0
  {
  let mut ng: *mut named_group = cb.named_groups;
  let mut tablecount: u32 = 0;

  /* Length 0 represents duplicates, and they have already been handled. */
  i = 0;
  while i < cb.names_found as u32
    {
    if (*ng).length > 0
      { tablecount = crate::compile_cgroup::_pcre2_compile_add_name_to_table8(cbp, ng, tablecount); }
    i += 1; ng = ng.add(1);
    }

  /* PCRE2_ASSERT(tablecount == cb.names_found); */
  }

/* Set up a starting, non-extracting bracket, then compile the expression. */

pptr = cb.parsed_pattern;
code = codestart;
*code = OP_BRA as u8;
regexrc = compile_regex((*re).overall_options, (*re).extra_options,
  core::ptr::addr_of_mut!(code), core::ptr::addr_of_mut!(pptr),
  core::ptr::addr_of_mut!(errorcode), 0,
  core::ptr::addr_of_mut!(firstcu), core::ptr::addr_of_mut!(firstcuflags),
  core::ptr::addr_of_mut!(reqcu), core::ptr::addr_of_mut!(reqcuflags),
  core::ptr::null_mut(), core::ptr::null_mut(), cbp, core::ptr::null_mut());
if regexrc < 0 { (*re).flags |= PCRE2_MATCH_EMPTY; }
(*re).top_bracket = cb.bracount as u16;
(*re).top_backref = cb.top_backref as u16;
(*re).max_lookbehind = cb.max_lookbehind as u16;

if cb.had_accept != 0
  {
  reqcu = 0;                        /* Must disable after (*ACCEPT) */
  reqcuflags = REQ_NONE;
  (*re).flags |= PCRE2_HASACCEPT;   /* Disables minimum length */
  }

/* Fill in the final opcode and check for disastrous overflow. */

*code = OP_END as u8; code = code.add(1);
usedlength = code.offset_from(codestart) as PCRE2_SIZE;
/* LCOV_EXCL_START */
if usedlength > length
  {
  /* PCRE2_DEBUG_UNREACHABLE() */
  errorcode = ERR23;  /* Overflow of code block - internal error */
  cb.erroroffset = 0;
  /* goto HAD_CB_ERROR */
  state = L_HAD_CB_ERROR; continue 'sm;
  }
/* LCOV_EXCL_STOP */

(*re).blocksize -= CU2BYTES!(length - usedlength);
/* #ifdef SUPPORT_VALGRIND: not compiled */

/* Scan the pattern for recursion/subroutine calls and convert the group
numbers into offsets. Maintain a small cache so that repeated groups containing
recursions are efficiently handled. */

if errorcode == 0 && cb.had_recurse != 0
  {
  let mut rcode: *mut PCRE2_UCHAR;
  let mut rgroup: PCRE2_SPTR;
  let mut ccount: u32 = 0;
  let mut start: i32 = RSCAN_CACHE_SIZE as i32;
  let mut rc: [recurse_cache; RSCAN_CACHE_SIZE] = core::mem::zeroed();
  let rcp: *mut recurse_cache = rc.as_mut_ptr();

  rcode = find_recurse(codestart, utf);
  while !rcode.is_null()
    {
    let mut p: i32;
    let groupnumber: i32;

    groupnumber = GET!(rcode, 1) as i32;
    if groupnumber == 0 { rgroup = codestart as PCRE2_SPTR; } else
      {
      let mut search_from: PCRE2_SPTR = codestart as PCRE2_SPTR;
      rgroup = core::ptr::null();
      i = 0; p = start;
      while i < ccount
        {
        if groupnumber == (*rcp.add(p as usize)).groupnumber
          {
          rgroup = (*rcp.add(p as usize)).group;
          break;
          }

        /* Group n+1 must always start to the right of group n, so we can save
        search time below when the new group number is greater than any of the
        previously found groups. */

        if groupnumber > (*rcp.add(p as usize)).groupnumber
          { search_from = (*rcp.add(p as usize)).group; }
        i += 1; p = (p + 1) & 7;
        }

      if rgroup.is_null()
        {
        rgroup = crate::find_bracket::_pcre2_find_bracket_8(search_from, utf, groupnumber);
        /* LCOV_EXCL_START */
        if rgroup.is_null()
          {
          /* PCRE2_DEBUG_UNREACHABLE() */
          errorcode = ERR53;
          break;
          }
        /* LCOV_EXCL_STOP */

        start -= 1; if start < 0 { start = RSCAN_CACHE_SIZE as i32 - 1; }
        (*rcp.add(start as usize)).groupnumber = groupnumber;
        (*rcp.add(start as usize)).group = rgroup;
        if (ccount as usize) < RSCAN_CACHE_SIZE { ccount += 1; }
        }
      }

    PUT!(rcode, 1, rgroup.offset_from(codestart as PCRE2_SPTR) as u32);
    rcode = find_recurse(rcode.add(1 + LINK_SIZE), utf);
    }
  }

/* #ifdef DEBUG_CALL_PRINTINT: not compiled */

/* Unless disabled, check whether any single character iterators can be
auto-possessified. */

if errorcode == 0 && (optim_flags & PCRE2_OPTIM_AUTO_POSSESS) != 0
  {
  let temp: *mut PCRE2_UCHAR = codestart;
  let possessify_rc: i32 = crate::auto_possess::_pcre2_auto_possessify_8(temp, cbp);
  /* LCOV_EXCL_START */
  if possessify_rc != 0
    {
    /* PCRE2_DEBUG_UNREACHABLE() */
    errorcode = ERR80;
    cb.erroroffset = 0;
    }
  /* LCOV_EXCL_STOP */
  }

/* Failed to compile, or error while post-processing. */

if errorcode != 0 { /* goto HAD_CB_ERROR */ state = L_HAD_CB_ERROR; continue 'sm; }

/* Successful compile. If the anchored option was not passed, set it if
we can determine that the pattern is anchored. */

if ((*re).overall_options & PCRE2_ANCHORED) == 0
  {
  let dotstar_anchor: BOOL = ((optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0) as BOOL;
  if is_anchored(codestart as PCRE2_SPTR, 0, cbp, 0, FALSE, dotstar_anchor) != 0
    { (*re).overall_options |= PCRE2_ANCHORED; }
  }

/* Set up the first code unit or startline flag, the required code unit, and
then study the pattern. */

if (optim_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0
  {
  let mut minminlength: i32 = 0;  /* For minimal minlength from first/required CU */
  let study_rc: i32;

  /* If we do not have a first code unit, see if there is one that is asserted
  (these are not saved during the compile because they can cause conflicts with
  actual literals that follow). */

  if firstcuflags >= REQ_NONE {
    let mut assertedcuflags: u32 = 0;
    let assertedcu: u32 = find_firstassertedcu(codestart as PCRE2_SPTR,
      core::ptr::addr_of_mut!(assertedcuflags), 0);
    /* It would be wrong to use the asserted first code unit as `firstcu` for
     * regexes which are able to match a 1-character string (e.g. /(?=a)b?a/) */
    if assertedcuflags < REQ_NONE && assertedcu != reqcu {
      firstcu = assertedcu;
      firstcuflags = assertedcuflags;
    }
  }

  /* Save the data for a first code unit. The existence of one means the
  minimum length must be at least 1. */

  if firstcuflags < REQ_NONE
    {
    (*re).first_codeunit = firstcu;
    (*re).flags |= PCRE2_FIRSTSET;
    minminlength += 1;

    /* Handle caseless first code units. */

    if (firstcuflags & REQ_CASELESS) != 0
      {
      if firstcu < 128 || (utf == 0 && ucp == 0 && firstcu < 255)
        {
        if *cb.fcc.add(firstcu as usize) as u32 != firstcu { (*re).flags |= PCRE2_FIRSTCASELESS; }
        }

      /* The first code unit is > 128 in UTF or UCP mode, or > 255 otherwise.
      In 8-bit UTF mode, code units in the range 128-255 are introductory code
      units and cannot have another case, but if UCP is set they may do. */

      /* #ifdef SUPPORT_UNICODE / #if PCRE2_CODE_UNIT_WIDTH == 8 */
      else if ucp != 0 && utf == 0 && UCD_OTHERCASE!(firstcu) != firstcu
        { (*re).flags |= PCRE2_FIRSTCASELESS; }
      }
    }

  /* When there is no first code unit, for non-anchored patterns, see if we can
  set the PCRE2_STARTLINE flag. */

  else if ((*re).overall_options & PCRE2_ANCHORED) == 0
    {
    let dotstar_anchor: BOOL = ((optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0) as BOOL;
    if is_startline(codestart as PCRE2_SPTR, 0, cbp, 0, FALSE, dotstar_anchor) != 0
      { (*re).flags |= PCRE2_STARTLINE; }
    }

  /* Handle the "required code unit", if one is set. */

  if reqcuflags < REQ_NONE
    {
    /* #elif PCRE2_CODE_UNIT_WIDTH == 8 */
    if ((*re).overall_options & PCRE2_UTF) == 0 ||   /* Not UTF */
        firstcuflags >= REQ_NONE ||                 /* First not set */
        (firstcu & 0x80) == 0 ||                    /* First is ASCII */
        (reqcu & 0x80) == 0                         /* Req is ASCII */
      {
      minminlength += 1;
      }

    /* In the case of an anchored pattern, set up the value only if it follows
    a variable length item in the pattern. */

    if ((*re).overall_options & PCRE2_ANCHORED) == 0 ||
        (reqcuflags & REQ_VARY) != 0
      {
      (*re).last_codeunit = reqcu;
      (*re).flags |= PCRE2_LASTSET;

      /* Handle caseless required code units as for first code units (above). */

      if (reqcuflags & REQ_CASELESS) != 0
        {
        if reqcu < 128 || (utf == 0 && ucp == 0 && reqcu < 255)
          {
          if *cb.fcc.add(reqcu as usize) as u32 != reqcu { (*re).flags |= PCRE2_LASTCASELESS; }
          }
      /* #ifdef SUPPORT_UNICODE / #if PCRE2_CODE_UNIT_WIDTH == 8 */
      else if ucp != 0 && utf == 0 && UCD_OTHERCASE!(reqcu) != reqcu
        { (*re).flags |= PCRE2_LASTCASELESS; }
        }
      }
    }

  /* Study the compiled pattern to set up information such as a bitmap of
  starting code units and a minimum matching length. */

  study_rc = crate::study::_pcre2_study_8(re);
  /* LCOV_EXCL_START */
  if study_rc != 0
    {
    /* PCRE2_DEBUG_UNREACHABLE() */
    errorcode = ERR31;
    cb.erroroffset = 0;
    /* goto HAD_CB_ERROR */
    state = L_HAD_CB_ERROR; continue 'sm;
    }
  /* LCOV_EXCL_STOP */

  /* If study() set a bitmap of starting code units, it implies a minimum
  length of at least one. */

  if ((*re).flags & PCRE2_FIRSTMAPSET) != 0 && minminlength == 0
    { minminlength = 1; }

  /* If the minimum length set (or not set) by study() is less than the minimum
  implied by required code units, override it. */

  if ((*re).minlength as i32) < minminlength { (*re).minlength = minminlength as u16; }
  }   /* End of start-of-match optimizations. */

/* Control ends up here in all cases. */

/* #ifdef SUPPORT_UNICODE - PCRE2_ASSERT(cb.first_data == NULL); */

/* Fall through to EXIT */
state = L_EXIT; continue 'sm;

}   /* End of L_MAIN */

L_EXIT => {
/* #ifdef SUPPORT_VALGRIND: not compiled */
if cb.parsed_pattern != stack_parsed_pattern.as_mut_ptr()
  { ((*ccontext).memctl.free.unwrap())(cb.parsed_pattern as *mut c_void,
      (*ccontext).memctl.memory_data); }
if cb.named_group_list_size > NAMED_GROUP_LIST_SIZE as u32
  { ((*ccontext).memctl.free.unwrap())(cb.named_groups as *mut c_void,
      (*ccontext).memctl.memory_data); }
if cb.groupinfo != stack_groupinfo.as_mut_ptr()
  { ((*ccontext).memctl.free.unwrap())(cb.groupinfo as *mut c_void,
      (*ccontext).memctl.memory_data); }

return re;    /* Will be NULL after an error */
}

/* Errors discovered in parse_regex() set the offset value in the compile
block. Errors discovered before it is called must compute it from the ptr
value. After parse_regex() is called, the offset in the compile block is set to
the end of the pattern, but certain errors in compile_regex() may reset it if
an offset is available in the parsed pattern. */

L_HAD_CB_ERROR => {
ptr = pattern.add(cb.erroroffset);
/* Fall through to HAD_EARLY_ERROR */
state = L_HAD_EARLY_ERROR; continue 'sm;
}

L_HAD_EARLY_ERROR => {
/* Ensure we don't return out-of-range erroroffset. */
/* PCRE2_ASSERT(ptr >= pattern); PCRE2_ASSERT(ptr <= (pattern + patlen)); */
/* #if defined PCRE2_DEBUG && defined SUPPORT_UNICODE: not compiled */
*erroroffset = ptr.offset_from(pattern) as PCRE2_SIZE;
/* Fall through to HAD_ERROR */
state = L_HAD_ERROR; continue 'sm;
}

L_HAD_ERROR => {
*errorptr = errorcode;
crate::compile::pcre2_code_free_8(re);
re = core::ptr::null_mut();

if !cb.first_data.is_null()
  {
  let mut current_data: *mut compile_data = cb.first_data;
  loop
    {
    let next_data: *mut compile_data = (*current_data).next;
    ((*cb.cx).memctl.free.unwrap())(current_data as *mut c_void, (*cb.cx).memctl.memory_data);
    current_data = next_data;
    if !(!current_data.is_null()) { break; }
    }
  }

/* goto EXIT */
state = L_EXIT; continue 'sm;
}

_ => {}
} break; }

/* Not reached: every state either returns or jumps. */
re
}

/* These #undefs are here to enable unity builds with CMake. */

/* #undef NLBLOCK / PSSTART / PSEND */

/* End of pcre2_compile.c */
