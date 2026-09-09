// Translated from the type definitions in pcre2.h, pcre2_internal.h,
// pcre2_intmodedep.h and pcre2_compile.h (8-bit mode, LINK_SIZE 2,
// SUPPORT_UNICODE, no JIT).
#![allow(dead_code, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use core::ffi::c_void;

pub type BOOL = i32;
pub type PCRE2_UCHAR = u8;
pub type PCRE2_SPTR = *const u8;
pub type PCRE2_SIZE = usize;

/* ---------------- Memory control ---------------- */

pub type MallocFn = Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>;
pub type FreeFn = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_memctl {
    pub malloc: MallocFn,
    pub free: FreeFn,
    pub memory_data: *mut c_void,
}

/* ---------------- Callout blocks (public API) ---------------- */

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

pub type CalloutFn = Option<unsafe extern "C" fn(*mut pcre2_callout_block, *mut c_void) -> i32>;
pub type SubstituteCalloutFn =
    Option<unsafe extern "C" fn(*mut pcre2_substitute_callout_block, *mut c_void) -> i32>;
pub type SubstituteCaseCalloutFn = Option<
    unsafe extern "C" fn(PCRE2_SPTR, PCRE2_SIZE, *mut PCRE2_UCHAR, PCRE2_SIZE, i32, *mut c_void)
        -> PCRE2_SIZE,
>;
pub type StackGuardFn = Option<unsafe extern "C" fn(u32, *mut c_void) -> i32>;
pub type CalloutEnumerateFn =
    Option<unsafe extern "C" fn(*mut pcre2_callout_enumerate_block, *mut c_void) -> i32>;
pub type JitCallbackFn = Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>;

/* ---------------- Contexts ---------------- */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_real_general_context {
    pub memctl: pcre2_memctl,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
pub struct pcre2_real_match_context {
    pub memctl: pcre2_memctl,
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
#[derive(Copy, Clone)]
pub struct pcre2_real_convert_context {
    pub memctl: pcre2_memctl,
    pub glob_separator: u32,
    pub glob_escape: u32,
}

/* ---------------- The compiled pattern ---------------- */

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

/* ---------------- Match data ---------------- */

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
    pub rc: i32,
    pub ovector: [PCRE2_SIZE; 131072],
}

#[repr(C)]
pub struct pcre2_real_jit_stack {
    pub memctl: pcre2_memctl,
    pub stack: *mut c_void,
}

/* ---------------- Miscellaneous mode-independent structures ---------------- */

#[repr(C)]
pub struct open_capitem {
    pub next: *mut open_capitem,
    pub number: u16,
    pub assert_depth: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ucp_type_table {
    pub name_offset: u16,
    pub type_: u16,
    pub value: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ucd_record {
    pub script: u8,
    pub chartype: u8,
    pub gbprop: u8,
    pub caseset: u8,
    pub other_case: i32,
    pub scriptx_bidiclass: u16,
    pub bprops: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_serialized_data {
    pub magic: u32,
    pub version: u32,
    pub config: u32,
    pub number_of_codes: i32,
}

/* ---------------- Private compile-time structures ---------------- */

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
#[derive(Copy, Clone)]
pub struct recurse_cache {
    pub group: PCRE2_SPTR,
    pub groupnumber: i32,
}

#[repr(C)]
pub struct branch_chain {
    pub outer: *mut branch_chain,
    pub current_branch: *mut PCRE2_UCHAR,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct named_group {
    pub name: PCRE2_SPTR,
    pub number: u32,
    pub length: u16,
    pub hash_dup: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
    /* Followed by the list of ranges (start/end pairs) */
}

#[repr(C)]
pub struct recurse_arguments {
    pub header: compile_data,
    pub size: usize,
    pub skip_size: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
    pub class_op_used: [u8; ECLASS_NEST_LIMIT],
    pub req_varyopt: u32,
    pub max_varlookbehind: u32,
    pub max_lookbehind: i32,
    pub had_accept: BOOL,
    pub had_pruneorskip: BOOL,
    pub had_recurse: BOOL,
    pub dupnames: BOOL,
    pub first_data: *mut compile_data,
    pub last_data: *mut compile_data,
    pub char_lists_size: usize,
}

#[repr(C)]
pub struct eclass_op_info {
    pub code_start: *mut PCRE2_UCHAR,
    pub length: PCRE2_SIZE,
    pub op_single_type: u8,
    pub bits: class_bits_storage,
}

/* ---------------- Match-time structures ---------------- */

#[repr(C)]
pub struct dfa_recursion_info {
    pub prevrec: *mut dfa_recursion_info,
    pub subject_position: PCRE2_SPTR,
    pub last_used_ptr: PCRE2_SPTR,
    pub group_num: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union heapframe_oc {
    pub oc: u32,
    pub occu: [PCRE2_UCHAR; 4],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_char_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub charptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    pub oc: heapframe_oc,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_charnot_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    pub oc: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_class_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub byte_map_address: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_xclass_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub xclass_data: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_eclass_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub eclass_data: PCRE2_SPTR,
    pub eclass_len: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_type_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub ctype: u32,
    pub propvalue: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_ref_repeat {
    pub start: PCRE2_SPTR,
    pub offset: PCRE2_SIZE,
    pub length: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_bra {
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_brapos {
    pub start_eptr: PCRE2_SPTR,
    pub start_group: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_recurse {
    pub start_branch: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_assert_scs {
    pub saved_end_subject: PCRE2_SPTR,
    pub saved_eptr: PCRE2_SPTR,
    pub true_end_extra: PCRE2_SIZE,
    pub saved_moptions: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_cond {
    pub start_branch: PCRE2_SPTR,
    pub length: PCRE2_SIZE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_vreverse {
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union heapframe_fields {
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
    pub ecode: PCRE2_SPTR,
    pub back_frame: PCRE2_SIZE,
    pub rdepth: u32,
    pub group_frame_type: u32,
    pub return_id: u8,
    pub op: u8,
    pub byte1: u8,
    pub byte2: u8,

    pub fields: heapframe_fields,

    pub eptr: PCRE2_SPTR,
    pub start_match: PCRE2_SPTR,
    pub mark: PCRE2_SPTR,
    pub recurse_last_used: PCRE2_SPTR,
    pub current_recurse: u32,
    pub capture_last: u32,
    pub last_group_offset: PCRE2_SIZE,
    pub offset_top: PCRE2_SIZE,
    pub ovector: [PCRE2_SIZE; 131072],
}

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

/* Offsets and sizes that the C code obtains with offsetof()/sizeof(). */

pub const OVECTOR_OFFSET_IN_MATCH_DATA: usize =
    core::mem::offset_of!(pcre2_real_match_data, ovector);
pub const OVECTOR_OFFSET_IN_HEAPFRAME: usize = core::mem::offset_of!(heapframe, ovector);
pub const HEAPFRAME_ALIGNMENT: usize = core::mem::align_of::<heapframe>();

pub const EPTR_OFFSET_IN_HEAPFRAME: usize = core::mem::offset_of!(heapframe, eptr);
