//! Core types, structures and macro equivalents for the PCRE2 8-bit library.
//!
//! This mirrors `pcre2_internal.h`, `pcre2_intmodedep.h` and the public
//! `pcre2.h` for `PCRE2_CODE_UNIT_WIDTH == 8` with `SUPPORT_UNICODE` enabled
//! and `SUPPORT_JIT` disabled.
#![allow(dead_code, non_camel_case_types, non_snake_case, non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_uint, c_void};

pub use crate::consts::*;

// ---------------------------------------------------------------------------
// Basic types (8-bit mode)
// ---------------------------------------------------------------------------

pub type PCRE2_UCHAR = u8;
pub type PCRE2_SPTR = *const u8;
pub type PCRE2_SIZE = usize;
pub type BOOL = c_int;

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

pub const PCRE2_UNSET: PCRE2_SIZE = usize::MAX;
pub const PCRE2_ZERO_TERMINATED: PCRE2_SIZE = usize::MAX;
pub const PCRE2_SIZE_MAX: PCRE2_SIZE = usize::MAX;

/// `LINK_SIZE` is 2 in this configuration.
pub const LINK_SIZE_U: usize = 2;
/// `IMM2_SIZE` is 2 in 8-bit mode.
pub const IMM2_SIZE_U: usize = 2;

pub const MAX_PATTERN_SIZE_U: usize = 1 << 16;
pub const MAX_MARK_U: u32 = (1u32 << 8) - 1;
pub const MAX_UTF_SINGLE_CU_U: u32 = 127;
pub const MAX_UTF_CODE_POINT_U: u32 = 0x10ffff;
pub const NOTACHAR_U: u32 = 0xffffffff;

// ---------------------------------------------------------------------------
// Memory control
// ---------------------------------------------------------------------------

pub type MallocFn = Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>;
pub type FreeFn = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct pcre2_memctl {
    pub malloc: MallocFn,
    pub free: FreeFn,
    pub memory_data: *mut c_void,
}

// ---------------------------------------------------------------------------
// Public callout blocks
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct pcre2_callout_block {
    pub version: u32,
    pub callout_number: u32,
    pub capture_top: u32,
    pub capture_last: u32,
    pub offset_vector: *mut PCRE2_SIZE,
    pub mark: PCRE2_SPTR,
    pub subject: PCRE2_SPTR,
    pub subject_length: PCRE2_SIZE,
    pub start_match: PCRE2_SIZE,
    pub current_position: PCRE2_SIZE,
    pub pattern_position: PCRE2_SIZE,
    pub next_item_length: PCRE2_SIZE,
    pub callout_string_offset: PCRE2_SIZE,
    pub callout_string_length: PCRE2_SIZE,
    pub callout_string: PCRE2_SPTR,
    pub callout_flags: u32,
}

#[repr(C)]
pub struct pcre2_callout_enumerate_block {
    pub version: u32,
    pub pattern_position: PCRE2_SIZE,
    pub next_item_length: PCRE2_SIZE,
    pub callout_number: u32,
    pub callout_string_offset: PCRE2_SIZE,
    pub callout_string_length: PCRE2_SIZE,
    pub callout_string: PCRE2_SPTR,
}

#[repr(C)]
pub struct pcre2_substitute_callout_block {
    pub version: u32,
    pub input: PCRE2_SPTR,
    pub output: PCRE2_SPTR,
    pub output_offsets: [PCRE2_SIZE; 2],
    pub ovector: *mut PCRE2_SIZE,
    pub oveccount: u32,
    pub subscount: u32,
}

pub type CalloutFn = Option<unsafe extern "C" fn(*mut pcre2_callout_block, *mut c_void) -> c_int>;
pub type SubstituteCalloutFn =
    Option<unsafe extern "C" fn(*mut pcre2_substitute_callout_block, *mut c_void) -> c_int>;
pub type SubstituteCaseCalloutFn = Option<
    unsafe extern "C" fn(
        PCRE2_SPTR,
        PCRE2_SIZE,
        *mut PCRE2_UCHAR,
        PCRE2_SIZE,
        c_int,
        *mut c_void,
    ) -> PCRE2_SIZE,
>;
pub type StackGuardFn = Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>;
pub type CalloutEnumerateFn =
    Option<unsafe extern "C" fn(*mut pcre2_callout_enumerate_block, *mut c_void) -> c_int>;

// ---------------------------------------------------------------------------
// Hidden structures
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct pcre2_real_general_context {
    pub memctl: pcre2_memctl,
}

#[repr(C)]
pub struct pcre2_real_compile_context {
    pub memctl: pcre2_memctl,
    pub stack_guard: StackGuardFn,
    pub stack_guard_data: *mut c_void,
    pub tables: *const u8,
    pub max_pattern_length: PCRE2_SIZE,
    pub max_pattern_compiled_length: PCRE2_SIZE,
    pub bsr_convention: u16,
    pub newline_convention: u16,
    pub parens_nest_limit: u32,
    pub extra_options: u32,
    pub max_varlookbehind: u32,
    pub optimization_flags: u32,
}

#[repr(C)]
pub struct pcre2_real_match_context {
    pub memctl: pcre2_memctl,
    // SUPPORT_JIT is not defined, so jit_callback / jit_callback_data are absent.
    pub callout: CalloutFn,
    pub callout_data: *mut c_void,
    pub substitute_callout: SubstituteCalloutFn,
    pub substitute_callout_data: *mut c_void,
    pub substitute_case_callout: SubstituteCaseCalloutFn,
    pub substitute_case_callout_data: *mut c_void,
    pub offset_limit: PCRE2_SIZE,
    pub heap_limit: u32,
    pub match_limit: u32,
    pub depth_limit: u32,
}

#[repr(C)]
pub struct pcre2_real_convert_context {
    pub memctl: pcre2_memctl,
    pub glob_separator: u32,
    pub glob_escape: u32,
}

pub const LOOKBEHIND_MAX: c_int = u16::MAX as c_int;

#[repr(C)]
pub struct pcre2_real_code {
    pub memctl: pcre2_memctl,
    pub tables: *const u8,
    pub executable_jit: *mut c_void,
    pub start_bitmap: [u8; 32],
    pub blocksize: PCRE2_SIZE,
    pub code_start: PCRE2_SIZE,
    pub magic_number: u32,
    pub compile_options: u32,
    pub overall_options: u32,
    pub extra_options: u32,
    pub flags: u32,
    pub limit_heap: u32,
    pub limit_match: u32,
    pub limit_depth: u32,
    pub first_codeunit: u32,
    pub last_codeunit: u32,
    pub bsr_convention: u16,
    pub newline_convention: u16,
    pub max_lookbehind: u16,
    pub minlength: u16,
    pub top_bracket: u16,
    pub top_backref: u16,
    pub name_entry_size: u16,
    pub name_count: u16,
    pub optimization_flags: u32,
}

/// `pcre2_real_match_data`. The C declaration ends with `PCRE2_SIZE
/// ovector[131072]`, but the allocation is sized to `offsetof(ovector) + 2 *
/// oveccount * sizeof(PCRE2_SIZE)`. We therefore declare a zero-length tail and
/// index it unsafely, exactly as C does.
#[repr(C)]
pub struct pcre2_real_match_data {
    pub memctl: pcre2_memctl,
    pub code: *const pcre2_real_code,
    pub subject: PCRE2_SPTR,
    pub mark: PCRE2_SPTR,
    pub heapframes: *mut heapframe,
    pub heapframes_size: PCRE2_SIZE,
    pub subject_length: PCRE2_SIZE,
    pub start_offset: PCRE2_SIZE,
    pub leftchar: PCRE2_SIZE,
    pub rightchar: PCRE2_SIZE,
    pub startchar: PCRE2_SIZE,
    pub matchedby: u8,
    pub flags: u8,
    pub oveccount: u16,
    pub options: u32,
    pub rc: c_int,
    pub ovector: [PCRE2_SIZE; 0],
}

impl pcre2_real_match_data {
    /// Byte offset of the `ovector` field, i.e. C's
    /// `offsetof(pcre2_real_match_data, ovector)`.
    pub const OVECTOR_OFFSET: usize = core::mem::size_of::<Self>();

    #[inline]
    pub unsafe fn ovec(&self) -> *mut PCRE2_SIZE {
        unsafe { self.ovector.as_ptr() as *mut PCRE2_SIZE }
    }
}

#[repr(C)]
pub struct pcre2_real_jit_stack {
    pub memctl: pcre2_memctl,
    pub stack: *mut c_void,
}

pub type pcre2_jit_callback = Option<unsafe extern "C" fn(*mut c_void) -> *mut pcre2_real_jit_stack>;

// Convenience aliases matching the public typedef names.
pub type pcre2_general_context = pcre2_real_general_context;
pub type pcre2_compile_context = pcre2_real_compile_context;
pub type pcre2_match_context = pcre2_real_match_context;
pub type pcre2_convert_context = pcre2_real_convert_context;
pub type pcre2_code = pcre2_real_code;
pub type pcre2_match_data = pcre2_real_match_data;
pub type pcre2_jit_stack = pcre2_real_jit_stack;

// ---------------------------------------------------------------------------
// Private structures
// ---------------------------------------------------------------------------

/// Structure for building a chain of open capturing subpatterns during
/// compiling, so that instructions to close them can be compiled when
/// `(*ACCEPT)` is encountered.
#[repr(C)]
pub struct open_capitem {
    pub next: *mut open_capitem,
    pub number: u16,
    pub assert_depth: u16,
}

#[repr(C)]
pub struct recurse_check {
    pub prev: *mut recurse_check,
    pub group: PCRE2_SPTR,
}

#[repr(C)]
pub struct parsed_recurse_check {
    pub prev: *mut parsed_recurse_check,
    pub groupptr: *mut u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct recurse_cache {
    pub group: PCRE2_SPTR,
    pub groupnumber: c_int,
}

#[repr(C)]
pub struct branch_chain {
    pub outer: *mut branch_chain,
    pub current_branch: *mut PCRE2_UCHAR,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct named_group {
    pub name: PCRE2_SPTR,
    pub number: u32,
    pub length: u16,
    pub hash_dup: u16,
}

/// `compile_data` — PCRE2_DEBUG is not defined, so only `next` is present.
#[repr(C)]
pub struct compile_data {
    pub next: *mut compile_data,
}

#[repr(C)]
pub struct class_ranges {
    pub header: compile_data,
    pub char_lists_size: usize,
    pub char_lists_start: usize,
    pub range_list_size: u16,
    pub char_lists_types: u16,
    // followed by the list of ranges (start/end pairs)
}

#[repr(C)]
pub struct recurse_arguments {
    pub header: compile_data,
    pub size: usize,
    pub skip_size: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union class_bits_storage {
    pub classbits: [u8; 32],
    pub classwords: [u32; 8],
}

#[repr(C)]
pub struct compile_block {
    pub cx: *mut pcre2_real_compile_context,
    pub lcc: *const u8,
    pub fcc: *const u8,
    pub cbits: *const u8,
    pub ctypes: *const u8,
    pub start_workspace: *mut PCRE2_UCHAR,
    pub start_code: *mut PCRE2_UCHAR,
    pub start_pattern: PCRE2_SPTR,
    pub end_pattern: PCRE2_SPTR,
    pub name_table: *mut PCRE2_UCHAR,
    pub workspace_size: PCRE2_SIZE,
    pub small_ref_offset: [PCRE2_SIZE; 10],
    pub erroroffset: PCRE2_SIZE,
    pub classbits: class_bits_storage,
    pub names_found: u16,
    pub name_entry_size: u16,
    pub parens_depth: u16,
    pub assert_depth: u16,
    pub named_groups: *mut named_group,
    pub named_group_list_size: u32,
    pub external_options: u32,
    pub external_flags: u32,
    pub bracount: u32,
    pub lastcapture: u32,
    pub parsed_pattern: *mut u32,
    pub parsed_pattern_end: *mut u32,
    pub groupinfo: *mut u32,
    pub top_backref: u32,
    pub backref_map: u32,
    pub nltype: u32,
    pub nllen: u32,
    pub nl: [PCRE2_UCHAR; 4],
    pub class_op_used: [u8; ECLASS_NEST_LIMIT as usize],
    pub req_varyopt: u32,
    pub max_varlookbehind: u32,
    pub max_lookbehind: c_int,
    pub had_accept: BOOL,
    pub had_pruneorskip: BOOL,
    pub had_recurse: BOOL,
    pub dupnames: BOOL,
    pub first_data: *mut compile_data,
    pub last_data: *mut compile_data,
    // SUPPORT_WIDE_CHARS is defined (SUPPORT_UNICODE in 8-bit mode).
    pub char_lists_size: usize,
}

#[repr(C)]
pub struct dfa_recursion_info {
    pub prevrec: *mut dfa_recursion_info,
    pub subject_position: PCRE2_SPTR,
    pub last_used_ptr: PCRE2_SPTR,
    pub group_num: u32,
}

// --- heapframe -------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_char_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub charptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    pub oc: hf_oc,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union hf_oc {
    pub oc: u32,
    /// 8-bit mode: buffer for other-case code units, 4 bytes.
    pub occu: [PCRE2_UCHAR; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_charnot_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    pub oc: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_class_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub byte_map_address: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_xclass_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub xclass_data: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_eclass_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub eclass_data: PCRE2_SPTR,
    pub eclass_len: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_type_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub ctype: u32,
    pub propvalue: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_ref_repeat {
    pub start: PCRE2_SPTR,
    pub offset: PCRE2_SIZE,
    pub length: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_bra {
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_brapos {
    pub start_eptr: PCRE2_SPTR,
    pub start_group: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_recurse {
    pub start_branch: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_assert_scs {
    pub saved_end_subject: PCRE2_SPTR,
    pub saved_eptr: PCRE2_SPTR,
    pub true_end_extra: PCRE2_SIZE,
    pub saved_moptions: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_cond {
    pub start_branch: PCRE2_SPTR,
    pub length: PCRE2_SIZE,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_vreverse {
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union hf_fields {
    pub char_repeat: hf_char_repeat,
    pub charnot_repeat: hf_charnot_repeat,
    pub class_repeat: hf_class_repeat,
    pub xclass_repeat: hf_xclass_repeat,
    pub eclass_repeat: hf_eclass_repeat,
    pub type_repeat: hf_type_repeat,
    pub ref_repeat: hf_ref_repeat,
    pub op_bra: hf_op_bra,
    pub op_brapos: hf_op_brapos,
    pub op_recurse: hf_op_recurse,
    pub op_assert_scs: hf_op_assert_scs,
    pub op_cond: hf_op_cond,
    pub op_vreverse: hf_op_vreverse,
}

#[repr(C)]
pub struct heapframe {
    // Preserved over RMATCH(), not copied to new frames.
    pub ecode: PCRE2_SPTR,
    pub back_frame: PCRE2_SIZE,
    pub rdepth: u32,
    pub group_frame_type: u32,
    pub return_id: u8,
    pub op: u8,
    pub byte1: u8,
    pub byte2: u8,

    pub fields: hf_fields,

    // Copied from the previous frame when a new frame becomes current.
    pub eptr: PCRE2_SPTR,
    pub start_match: PCRE2_SPTR,
    pub mark: PCRE2_SPTR,
    pub recurse_last_used: PCRE2_SPTR,
    pub current_recurse: u32,
    pub capture_last: u32,
    pub last_group_offset: PCRE2_SIZE,
    pub offset_top: PCRE2_SIZE,
    pub ovector: [PCRE2_SIZE; 0],
}

impl heapframe {
    /// C's `offsetof(heapframe, ovector)`.
    pub const OVECTOR_OFFSET: usize = core::mem::size_of::<heapframe>();
    /// C's `offsetof(heapframe, eptr)`.
    pub const EPTR_OFFSET: usize = core::mem::offset_of!(heapframe, eptr);
}

/// C's `HEAPFRAME_ALIGNMENT` = `offsetof(heapframe_align, frame)`, i.e. the
/// alignment requirement of `heapframe`.
pub const HEAPFRAME_ALIGNMENT: usize = core::mem::align_of::<heapframe>();

#[repr(C)]
pub struct match_block {
    pub memctl: pcre2_memctl,
    pub heap_limit: u32,
    pub match_limit: u32,
    pub match_limit_depth: u32,
    pub match_call_count: u32,
    pub hitend: BOOL,
    pub hasthen: BOOL,
    pub hasbsk: BOOL,
    pub allowemptypartial: BOOL,
    pub allowlookaroundbsk: BOOL,
    pub lcc: *const u8,
    pub fcc: *const u8,
    pub ctypes: *const u8,
    pub start_offset: PCRE2_SIZE,
    pub end_offset_top: PCRE2_SIZE,
    pub partial: u16,
    pub bsr_convention: u16,
    pub name_count: u16,
    pub name_entry_size: u16,
    pub name_table: PCRE2_SPTR,
    pub start_code: PCRE2_SPTR,
    pub start_subject: PCRE2_SPTR,
    pub check_subject: PCRE2_SPTR,
    pub end_subject: PCRE2_SPTR,
    pub true_end_subject: PCRE2_SPTR,
    pub end_match_ptr: PCRE2_SPTR,
    pub start_used_ptr: PCRE2_SPTR,
    pub last_used_ptr: PCRE2_SPTR,
    pub mark: PCRE2_SPTR,
    pub nomatch_mark: PCRE2_SPTR,
    pub verb_ecode_ptr: PCRE2_SPTR,
    pub verb_skip_ptr: PCRE2_SPTR,
    pub verb_current_recurse: u32,
    pub moptions: u32,
    pub poptions: u32,
    pub skip_arg_count: u32,
    pub ignore_skip_arg: u32,
    pub nltype: u32,
    pub nllen: u32,
    pub nl: [PCRE2_UCHAR; 4],
    pub cb: *mut pcre2_callout_block,
    pub callout_data: *mut c_void,
    pub callout: CalloutFn,
}

#[repr(C)]
pub struct dfa_match_block {
    pub memctl: pcre2_memctl,
    pub start_code: PCRE2_SPTR,
    pub start_subject: PCRE2_SPTR,
    pub end_subject: PCRE2_SPTR,
    pub start_used_ptr: PCRE2_SPTR,
    pub last_used_ptr: PCRE2_SPTR,
    pub tables: *const u8,
    pub start_offset: PCRE2_SIZE,
    pub heap_limit: u32,
    pub heap_used: PCRE2_SIZE,
    pub match_limit: u32,
    pub match_limit_depth: u32,
    pub match_call_count: u32,
    pub moptions: u32,
    pub poptions: u32,
    pub nltype: u32,
    pub nllen: u32,
    pub allowemptypartial: BOOL,
    pub nl: [PCRE2_UCHAR; 4],
    pub bsr_convention: u16,
    pub cb: *mut pcre2_callout_block,
    pub callout_data: *mut c_void,
    pub callout: CalloutFn,
    pub recursive: *mut dfa_recursion_info,
}

// ---------------------------------------------------------------------------
// Unicode character database
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UcdRecord {
    pub script: u8,
    pub chartype: u8,
    pub gbprop: u8,
    pub caseset: u8,
    pub other_case: i32,
    pub scriptx_bidiclass: u16,
    pub bprops: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UcpTypeTable {
    pub name_offset: u16,
    pub type_: u16,
    pub value: u16,
}

// ---------------------------------------------------------------------------
// Macro equivalents from pcre2_intmodedep.h / pcre2_internal.h (8-bit, LINK_SIZE 2)
// ---------------------------------------------------------------------------

/// `PUT(a, n, d)` for LINK_SIZE == 2.
#[inline(always)]
pub unsafe fn PUT(a: *mut PCRE2_UCHAR, n: usize, d: i32) {
    unsafe {
        *a.add(n) = ((d as u32) >> 8) as u8;
        *a.add(n + 1) = ((d as u32) & 255) as u8;
    }
}

/// `GET(a, n)` for LINK_SIZE == 2.
#[inline(always)]
pub unsafe fn GET(a: PCRE2_SPTR, n: usize) -> u32 {
    unsafe { ((*a.add(n) as u32) << 8) | (*a.add(n + 1) as u32) }
}

/// `GET2(a, n)` for IMM2_SIZE == 2.
#[inline(always)]
pub unsafe fn GET2(a: PCRE2_SPTR, n: usize) -> u32 {
    unsafe { ((*a.add(n) as u32) << 8) | (*a.add(n + 1) as u32) }
}

/// `PUT2(a, n, d)` for IMM2_SIZE == 2.
#[inline(always)]
pub unsafe fn PUT2(a: *mut PCRE2_UCHAR, n: usize, d: u32) {
    unsafe {
        *a.add(n) = (d >> 8) as u8;
        *a.add(n + 1) = (d & 255) as u8;
    }
}

/// `PUTINC(a, n, d)` — `PUT` then advance by LINK_SIZE.
#[inline(always)]
pub unsafe fn PUTINC(a: &mut *mut PCRE2_UCHAR, n: usize, d: i32) {
    unsafe {
        PUT(*a, n, d);
        *a = a.add(LINK_SIZE_U);
    }
}

/// `PUT2INC(a, n, d)` — `PUT2` then advance by IMM2_SIZE.
#[inline(always)]
pub unsafe fn PUT2INC(a: &mut *mut PCRE2_UCHAR, n: usize, d: u32) {
    unsafe {
        PUT2(*a, n, d);
        *a = a.add(IMM2_SIZE_U);
    }
}

/// `MAX_255(c)` — always TRUE in 8-bit mode.
#[inline(always)]
pub const fn MAX_255(_c: u32) -> bool {
    true
}

/// `CHMAX_255(c)` — with SUPPORT_UNICODE in 8-bit mode.
#[inline(always)]
pub const fn CHMAX_255(c: u32) -> bool {
    c <= 255
}

/// `TABLE_GET(c, table, default)` — in 8-bit mode simply `table[c]`.
#[inline(always)]
pub unsafe fn TABLE_GET(c: u32, table: *const u8, _default: u32) -> u32 {
    unsafe { *table.add(c as usize) as u32 }
}

/// `HASUTF8EXTRALEN(c)` / `HAS_EXTRALEN(c)`.
#[inline(always)]
pub const fn HASUTF8EXTRALEN(c: u32) -> bool {
    c >= 0xc0
}

/// `HAS_EXTRALEN(c)`.
#[inline(always)]
pub const fn HAS_EXTRALEN(c: u32) -> bool {
    c >= 0xc0
}

/// `GET_EXTRALEN(c)`.
#[inline(always)]
pub fn GET_EXTRALEN(c: u32) -> u32 {
    crate::tables::_pcre2_utf8_table4[(c & 0x3f) as usize] as u32
}

/// `NOT_FIRSTCU(c)`.
#[inline(always)]
pub const fn NOT_FIRSTCU(c: u32) -> bool {
    (c & 0xc0) == 0x80
}

/// `GETUTF8(c, eptr)` — decode the remaining bytes without advancing.
#[inline(always)]
pub unsafe fn GETUTF8(c: u32, eptr: PCRE2_SPTR) -> u32 {
    unsafe {
        if (c & 0x20) == 0 {
            ((c & 0x1f) << 6) | (*eptr.add(1) as u32 & 0x3f)
        } else if (c & 0x10) == 0 {
            ((c & 0x0f) << 12) | ((*eptr.add(1) as u32 & 0x3f) << 6) | (*eptr.add(2) as u32 & 0x3f)
        } else if (c & 0x08) == 0 {
            ((c & 0x07) << 18)
                | ((*eptr.add(1) as u32 & 0x3f) << 12)
                | ((*eptr.add(2) as u32 & 0x3f) << 6)
                | (*eptr.add(3) as u32 & 0x3f)
        } else if (c & 0x04) == 0 {
            ((c & 0x03) << 24)
                | ((*eptr.add(1) as u32 & 0x3f) << 18)
                | ((*eptr.add(2) as u32 & 0x3f) << 12)
                | ((*eptr.add(3) as u32 & 0x3f) << 6)
                | (*eptr.add(4) as u32 & 0x3f)
        } else {
            ((c & 0x01) << 30)
                | ((*eptr.add(1) as u32 & 0x3f) << 24)
                | ((*eptr.add(2) as u32 & 0x3f) << 18)
                | ((*eptr.add(3) as u32 & 0x3f) << 12)
                | ((*eptr.add(4) as u32 & 0x3f) << 6)
                | (*eptr.add(5) as u32 & 0x3f)
        }
    }
}

/// `GETUTF8INC(c, eptr)` — decode remaining bytes, advancing `eptr` past them.
#[inline(always)]
pub unsafe fn GETUTF8INC(c: u32, eptr: &mut PCRE2_SPTR) -> u32 {
    unsafe {
        let mut p = *eptr;
        let mut next = || {
            let v = *p as u32 & 0x3f;
            p = p.add(1);
            v
        };
        let r = if (c & 0x20) == 0 {
            ((c & 0x1f) << 6) | next()
        } else if (c & 0x10) == 0 {
            let a = next();
            let b = next();
            ((c & 0x0f) << 12) | (a << 6) | b
        } else if (c & 0x08) == 0 {
            let a = next();
            let b = next();
            let d = next();
            ((c & 0x07) << 18) | (a << 12) | (b << 6) | d
        } else if (c & 0x04) == 0 {
            let a = next();
            let b = next();
            let d = next();
            let e = next();
            ((c & 0x03) << 24) | (a << 18) | (b << 12) | (d << 6) | e
        } else {
            let a = next();
            let b = next();
            let d = next();
            let e = next();
            let f = next();
            ((c & 0x01) << 30) | (a << 24) | (b << 18) | (d << 12) | (e << 6) | f
        };
        *eptr = p;
        r
    }
}

/// `GETUTF8LEN(c, eptr, len)` — decode without advancing, adding the number of
/// extra code units to `len`.
#[inline(always)]
pub unsafe fn GETUTF8LEN(c: u32, eptr: PCRE2_SPTR, len: &mut u32) -> u32 {
    unsafe {
        if (c & 0x20) == 0 {
            *len += 1;
            ((c & 0x1f) << 6) | (*eptr.add(1) as u32 & 0x3f)
        } else if (c & 0x10) == 0 {
            *len += 2;
            ((c & 0x0f) << 12) | ((*eptr.add(1) as u32 & 0x3f) << 6) | (*eptr.add(2) as u32 & 0x3f)
        } else if (c & 0x08) == 0 {
            *len += 3;
            ((c & 0x07) << 18)
                | ((*eptr.add(1) as u32 & 0x3f) << 12)
                | ((*eptr.add(2) as u32 & 0x3f) << 6)
                | (*eptr.add(3) as u32 & 0x3f)
        } else if (c & 0x04) == 0 {
            *len += 4;
            ((c & 0x03) << 24)
                | ((*eptr.add(1) as u32 & 0x3f) << 18)
                | ((*eptr.add(2) as u32 & 0x3f) << 12)
                | ((*eptr.add(3) as u32 & 0x3f) << 6)
                | (*eptr.add(4) as u32 & 0x3f)
        } else {
            *len += 5;
            ((c & 0x01) << 30)
                | ((*eptr.add(1) as u32 & 0x3f) << 24)
                | ((*eptr.add(2) as u32 & 0x3f) << 18)
                | ((*eptr.add(3) as u32 & 0x3f) << 12)
                | ((*eptr.add(4) as u32 & 0x3f) << 6)
                | (*eptr.add(5) as u32 & 0x3f)
        }
    }
}

/// `GETCHAR(c, eptr)` — UTF mode known.
#[inline(always)]
pub unsafe fn GETCHAR(eptr: PCRE2_SPTR) -> u32 {
    unsafe {
        let c = *eptr as u32;
        if c >= 0xc0 { GETUTF8(c, eptr) } else { c }
    }
}

/// `GETCHARTEST(c, eptr)`.
#[inline(always)]
pub unsafe fn GETCHARTEST(eptr: PCRE2_SPTR, utf: bool) -> u32 {
    unsafe {
        let c = *eptr as u32;
        if utf && c >= 0xc0 { GETUTF8(c, eptr) } else { c }
    }
}

/// `GETCHARINC(c, eptr)`.
#[inline(always)]
pub unsafe fn GETCHARINC(eptr: &mut PCRE2_SPTR) -> u32 {
    unsafe {
        let c = **eptr as u32;
        *eptr = eptr.add(1);
        if c >= 0xc0 { GETUTF8INC(c, eptr) } else { c }
    }
}

/// `GETCHARINCTEST(c, eptr)`.
#[inline(always)]
pub unsafe fn GETCHARINCTEST(eptr: &mut PCRE2_SPTR, utf: bool) -> u32 {
    unsafe {
        let c = **eptr as u32;
        *eptr = eptr.add(1);
        if utf && c >= 0xc0 {
            GETUTF8INC(c, eptr)
        } else {
            c
        }
    }
}

/// `GETCHARLEN(c, eptr, len)`.
#[inline(always)]
pub unsafe fn GETCHARLEN(eptr: PCRE2_SPTR, len: &mut u32) -> u32 {
    unsafe {
        let c = *eptr as u32;
        if c >= 0xc0 { GETUTF8LEN(c, eptr, len) } else { c }
    }
}

/// `GETCHARLENTEST(c, eptr, len)`.
#[inline(always)]
pub unsafe fn GETCHARLENTEST(eptr: PCRE2_SPTR, len: &mut u32, utf: bool) -> u32 {
    unsafe {
        let c = *eptr as u32;
        if utf && c >= 0xc0 {
            GETUTF8LEN(c, eptr, len)
        } else {
            c
        }
    }
}

/// `BACKCHAR(eptr)`.
#[inline(always)]
pub unsafe fn BACKCHAR(eptr: &mut PCRE2_SPTR) {
    unsafe {
        while (**eptr & 0xc0) == 0x80 {
            *eptr = eptr.sub(1);
        }
    }
}

/// `FORWARDCHAR(eptr)`.
#[inline(always)]
pub unsafe fn FORWARDCHAR(eptr: &mut PCRE2_SPTR) {
    unsafe {
        while (**eptr & 0xc0) == 0x80 {
            *eptr = eptr.add(1);
        }
    }
}

/// `FORWARDCHARTEST(eptr, end)`.
#[inline(always)]
pub unsafe fn FORWARDCHARTEST(eptr: &mut PCRE2_SPTR, end: PCRE2_SPTR) {
    unsafe {
        while *eptr < end && (**eptr & 0xc0) == 0x80 {
            *eptr = eptr.add(1);
        }
    }
}

/// `PUTCHAR(c, p)` — returns the number of code units written.
#[inline(always)]
pub unsafe fn PUTCHAR(c: u32, p: *mut PCRE2_UCHAR, utf: bool) -> u32 {
    unsafe {
        if utf && c > MAX_UTF_SINGLE_CU_U {
            crate::ord2utf::_pcre2_ord2utf_8(c, p)
        } else {
            *p = c as u8;
            1
        }
    }
}

// ---------------------------------------------------------------------------
// UCD access macros (pcre2_internal.h, SUPPORT_UNICODE)
// ---------------------------------------------------------------------------

pub const UCD_BLOCK_SZ: usize = 128;

#[inline(always)]
pub fn GET_UCD(ch: u32) -> &'static UcdRecord {
    let s1 = crate::tables::_pcre2_ucd_stage1[(ch as usize) / UCD_BLOCK_SZ] as usize;
    let s2 = crate::tables::_pcre2_ucd_stage2[s1 * UCD_BLOCK_SZ + (ch as usize) % UCD_BLOCK_SZ]
        as usize;
    &crate::tables::_pcre2_ucd_records[s2]
}

#[inline(always)]
pub fn UCD_CHARTYPE(ch: u32) -> u32 {
    GET_UCD(ch).chartype as u32
}
#[inline(always)]
pub fn UCD_SCRIPT(ch: u32) -> u32 {
    GET_UCD(ch).script as u32
}
#[inline(always)]
pub fn UCD_CATEGORY(ch: u32) -> u32 {
    crate::tables::_pcre2_ucp_gentype[UCD_CHARTYPE(ch) as usize]
}
#[inline(always)]
pub fn UCD_GRAPHBREAK(ch: u32) -> u32 {
    GET_UCD(ch).gbprop as u32
}
#[inline(always)]
pub fn UCD_CASESET(ch: u32) -> u32 {
    GET_UCD(ch).caseset as u32
}
#[inline(always)]
pub fn UCD_OTHERCASE(ch: u32) -> u32 {
    (ch as i32 + GET_UCD(ch).other_case) as u32
}
#[inline(always)]
pub fn UCD_SCRIPTX_PROP(prop: &UcdRecord) -> u32 {
    (prop.scriptx_bidiclass as u32) & UCD_SCRIPTX_MASK as u32
}
#[inline(always)]
pub fn UCD_BIDICLASS_PROP(prop: &UcdRecord) -> u32 {
    (prop.scriptx_bidiclass as u32) >> UCD_BIDICLASS_SHIFT as u32
}
#[inline(always)]
pub fn UCD_BPROPS_PROP(prop: &UcdRecord) -> u32 {
    prop.bprops as u32
}
#[inline(always)]
pub fn UCD_SCRIPTX(ch: u32) -> u32 {
    UCD_SCRIPTX_PROP(GET_UCD(ch))
}
#[inline(always)]
pub fn UCD_BIDICLASS(ch: u32) -> u32 {
    UCD_BIDICLASS_PROP(GET_UCD(ch))
}
#[inline(always)]
pub fn UCD_BPROPS(ch: u32) -> u32 {
    UCD_BPROPS_PROP(GET_UCD(ch))
}

/// `MAPBIT(map, c)` — test bit `c` in a 32-bit-word bitmap.
#[inline(always)]
pub unsafe fn MAPBIT(map: *const u32, c: u32) -> u32 {
    unsafe { *map.add((c as usize) / 32) & (1u32 << ((c as usize) % 32)) }
}

// ---------------------------------------------------------------------------
// Misc helpers
// ---------------------------------------------------------------------------

/// Signed multiplication with overflow check, matching `PRIV(ckd_smul)`
/// semantics for `int` operands.
#[inline(always)]
pub fn ckd_smul_i32(r: &mut PCRE2_SIZE, a: c_int, b: c_int) -> bool {
    match (a as i64).checked_mul(b as i64) {
        Some(v) if v >= 0 && (v as u64) <= usize::MAX as u64 => {
            *r = v as PCRE2_SIZE;
            false
        }
        _ => true,
    }
}

/// `CU2BYTES(x)` — in 8-bit mode a no-op.
#[inline(always)]
pub const fn CU2BYTES(x: usize) -> usize {
    x
}

/// `BYTES2CU(x)` — in 8-bit mode a no-op.
#[inline(always)]
pub const fn BYTES2CU(x: usize) -> usize {
    x
}

/// Allocate `size` bytes using the memory control block that heads `memctl`,
/// copying the control block into the front of the new allocation. Mirrors
/// `PRIV(memctl_malloc)`.
#[inline]
pub unsafe fn memctl_alloc(size: usize, memctl: *mut pcre2_memctl) -> *mut c_void {
    unsafe { crate::context::_pcre2_memctl_malloc_8(size, memctl) }
}

#[inline(always)]
pub unsafe fn c_memcpy(dst: *mut c_void, src: *const c_void, n: usize) {
    if n != 0 {
        unsafe { core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, n) };
    }
}

#[inline(always)]
pub unsafe fn c_memmove(dst: *mut c_void, src: *const c_void, n: usize) {
    if n != 0 {
        unsafe { core::ptr::copy(src as *const u8, dst as *mut u8, n) };
    }
}

#[inline(always)]
pub unsafe fn c_memset(dst: *mut c_void, v: u8, n: usize) {
    if n != 0 {
        unsafe { core::ptr::write_bytes(dst as *mut u8, v, n) };
    }
}

#[inline(always)]
pub unsafe fn c_memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int {
    unsafe {
        let a = a as *const u8;
        let b = b as *const u8;
        for i in 0..n {
            let x = *a.add(i);
            let y = *b.add(i);
            if x != y {
                return x as c_int - y as c_int;
            }
        }
        0
    }
}

#[inline(always)]
pub unsafe fn c_strlen(s: *const c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        n
    }
}

unsafe extern "C" {
    /// The libc allocator, used by `pcre2_general_context_create(NULL, NULL, ...)`
    /// style default memory management.
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
}

/// Default malloc wrapper used when no custom allocator is supplied.
pub unsafe extern "C" fn default_malloc(size: usize, _data: *mut c_void) -> *mut c_void {
    unsafe { malloc(size) }
}

/// Default free wrapper used when no custom allocator is supplied.
pub unsafe extern "C" fn default_free(block: *mut c_void, _data: *mut c_void) {
    unsafe { free(block) }
}

/// Number of `c_uint` values, used by generated code.
pub type CUint = c_uint;
