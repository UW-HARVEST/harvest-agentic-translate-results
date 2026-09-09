#!/usr/bin/env python3
"""Create lib.rs plus stub module files with the exact exported signatures."""
import os

OUT = os.path.join(os.path.dirname(__file__), '..', 'translation', 'src')

# module -> list of (linker_symbol, rust_signature_without_name, body_default)
FUNCS = {
 'auto_possess': [
   ('_pcre2_auto_possessify_8', '(code: *mut PCRE2_UCHAR, cb: *const compile_block) -> i32', '0'),
 ],
 'chkdint': [
   ('_pcre2_ckd_smul_8', '(r: *mut PCRE2_SIZE, a: i32, b: i32) -> BOOL', '0'),
 ],
 'compile': [
   ('pcre2_compile_8', '(pattern: PCRE2_SPTR, patlen: PCRE2_SIZE, options: u32, errorptr: *mut i32, erroroffset: *mut PCRE2_SIZE, ccontext: *mut pcre2_real_compile_context) -> *mut pcre2_real_code', 'core::ptr::null_mut()'),
   ('pcre2_code_free_8', '(code: *mut pcre2_real_code)', '()'),
   ('pcre2_code_copy_8', '(code: *const pcre2_real_code) -> *mut pcre2_real_code', 'core::ptr::null_mut()'),
   ('pcre2_code_copy_with_tables_8', '(code: *const pcre2_real_code) -> *mut pcre2_real_code', 'core::ptr::null_mut()'),
   ('_pcre2_check_escape_8', '(ptrptr: *mut PCRE2_SPTR, ptrend: PCRE2_SPTR, chptr: *mut u32, errorcodeptr: *mut i32, options: u32, xoptions: u32, bracount: u32, isclass: BOOL, cb: *mut compile_block) -> i32', '0'),
 ],
 'compile_cgroup': [
   ('_pcre2_compile_get_hash_from_name8', '(name: PCRE2_SPTR, length: u32) -> u16', '0'),
   ('_pcre2_compile_find_named_group8', '(name: PCRE2_SPTR, length: u32, cb: *mut compile_block) -> *mut named_group', 'core::ptr::null_mut()'),
   ('_pcre2_compile_add_name_to_table8', '(cb: *mut compile_block, ng: *mut named_group, tablecount: u32) -> u32', '0'),
   ('_pcre2_compile_find_dupname_details8', '(name: PCRE2_SPTR, length: u32, indexptr: *mut i32, countptr: *mut i32, errorcodeptr: *mut i32, cb: *mut compile_block) -> BOOL', '0'),
   ('_pcre2_compile_parse_scan_substr_args8', '(pptr: *mut u32, errorcodeptr: *mut i32, cb: *mut compile_block, lengthptr: *mut PCRE2_SIZE) -> *mut u32', 'core::ptr::null_mut()'),
   ('_pcre2_compile_parse_recurse_args8', '(pptr_start: *mut u32, offset: PCRE2_SIZE, errorcodeptr: *mut i32, cb: *mut compile_block) -> BOOL', '0'),
 ],
 'compile_class': [
   ('_pcre2_update_classbits_8', '(ptype: u32, pdata: u32, negated: BOOL, classbits: *mut u8)', '()'),
   ('_pcre2_compile_class_not_nested_8', '(options: u32, xoptions: u32, start_ptr: *mut u32, pcode: *mut *mut PCRE2_UCHAR, negate_class: BOOL, has_bitmap: *mut BOOL, errorcodeptr: *mut i32, cb: *mut compile_block, lengthptr: *mut PCRE2_SIZE) -> *mut u32', 'core::ptr::null_mut()'),
   ('_pcre2_compile_class_nested_8', '(options: u32, xoptions: u32, pptr: *mut *mut u32, pcode: *mut *mut PCRE2_UCHAR, errorcodeptr: *mut i32, cb: *mut compile_block, lengthptr: *mut PCRE2_SIZE) -> BOOL', '0'),
 ],
 'config': [
   ('pcre2_config_8', '(what: u32, where_: *mut c_void) -> i32', '0'),
 ],
 'context': [
   ('_pcre2_memctl_malloc_8', '(size: usize, memctl: *mut pcre2_memctl) -> *mut c_void', 'core::ptr::null_mut()'),
   ('pcre2_general_context_create_8', '(private_malloc: MallocFn, private_free: FreeFn, memory_data: *mut c_void) -> *mut pcre2_real_general_context', 'core::ptr::null_mut()'),
   ('pcre2_general_context_copy_8', '(gcontext: *mut pcre2_real_general_context) -> *mut pcre2_real_general_context', 'core::ptr::null_mut()'),
   ('pcre2_general_context_free_8', '(gcontext: *mut pcre2_real_general_context)', '()'),
   ('pcre2_compile_context_create_8', '(gcontext: *mut pcre2_real_general_context) -> *mut pcre2_real_compile_context', 'core::ptr::null_mut()'),
   ('pcre2_compile_context_copy_8', '(ccontext: *mut pcre2_real_compile_context) -> *mut pcre2_real_compile_context', 'core::ptr::null_mut()'),
   ('pcre2_compile_context_free_8', '(ccontext: *mut pcre2_real_compile_context)', '()'),
   ('pcre2_match_context_create_8', '(gcontext: *mut pcre2_real_general_context) -> *mut pcre2_real_match_context', 'core::ptr::null_mut()'),
   ('pcre2_match_context_copy_8', '(mcontext: *mut pcre2_real_match_context) -> *mut pcre2_real_match_context', 'core::ptr::null_mut()'),
   ('pcre2_match_context_free_8', '(mcontext: *mut pcre2_real_match_context)', '()'),
   ('pcre2_convert_context_create_8', '(gcontext: *mut pcre2_real_general_context) -> *mut pcre2_real_convert_context', 'core::ptr::null_mut()'),
   ('pcre2_convert_context_copy_8', '(ccontext: *mut pcre2_real_convert_context) -> *mut pcre2_real_convert_context', 'core::ptr::null_mut()'),
   ('pcre2_convert_context_free_8', '(ccontext: *mut pcre2_real_convert_context)', '()'),
   ('pcre2_set_character_tables_8', '(ccontext: *mut pcre2_real_compile_context, tables: *const u8) -> i32', '0'),
   ('pcre2_set_bsr_8', '(ccontext: *mut pcre2_real_compile_context, value: u32) -> i32', '0'),
   ('pcre2_set_max_pattern_length_8', '(ccontext: *mut pcre2_real_compile_context, length: PCRE2_SIZE) -> i32', '0'),
   ('pcre2_set_max_pattern_compiled_length_8', '(ccontext: *mut pcre2_real_compile_context, length: PCRE2_SIZE) -> i32', '0'),
   ('pcre2_set_max_varlookbehind_8', '(ccontext: *mut pcre2_real_compile_context, limit: u32) -> i32', '0'),
   ('pcre2_set_newline_8', '(ccontext: *mut pcre2_real_compile_context, newline: u32) -> i32', '0'),
   ('pcre2_set_parens_nest_limit_8', '(ccontext: *mut pcre2_real_compile_context, limit: u32) -> i32', '0'),
   ('pcre2_set_compile_extra_options_8', '(ccontext: *mut pcre2_real_compile_context, options: u32) -> i32', '0'),
   ('pcre2_set_compile_recursion_guard_8', '(ccontext: *mut pcre2_real_compile_context, guard: StackGuardFn, user_data: *mut c_void) -> i32', '0'),
   ('pcre2_set_optimize_8', '(ccontext: *mut pcre2_real_compile_context, directive: u32) -> i32', '0'),
   ('pcre2_set_callout_8', '(mcontext: *mut pcre2_real_match_context, callout: CalloutFn, callout_data: *mut c_void) -> i32', '0'),
   ('pcre2_set_substitute_callout_8', '(mcontext: *mut pcre2_real_match_context, callout: SubstituteCalloutFn, callout_data: *mut c_void) -> i32', '0'),
   ('pcre2_set_substitute_case_callout_8', '(mcontext: *mut pcre2_real_match_context, callout: SubstituteCaseCalloutFn, callout_data: *mut c_void) -> i32', '0'),
   ('pcre2_set_heap_limit_8', '(mcontext: *mut pcre2_real_match_context, limit: u32) -> i32', '0'),
   ('pcre2_set_match_limit_8', '(mcontext: *mut pcre2_real_match_context, limit: u32) -> i32', '0'),
   ('pcre2_set_depth_limit_8', '(mcontext: *mut pcre2_real_match_context, limit: u32) -> i32', '0'),
   ('pcre2_set_offset_limit_8', '(mcontext: *mut pcre2_real_match_context, limit: PCRE2_SIZE) -> i32', '0'),
   ('pcre2_set_recursion_limit_8', '(mcontext: *mut pcre2_real_match_context, limit: u32) -> i32', '0'),
   ('pcre2_set_recursion_memory_management_8', '(_mcontext: *mut pcre2_real_match_context, _mymalloc: MallocFn, _myfree: FreeFn, _mydata: *mut c_void) -> i32', '0'),
   ('pcre2_set_glob_separator_8', '(ccontext: *mut pcre2_real_convert_context, separator: u32) -> i32', '0'),
   ('pcre2_set_glob_escape_8', '(ccontext: *mut pcre2_real_convert_context, escape: u32) -> i32', '0'),
 ],
 'convert': [
   ('pcre2_pattern_convert_8', '(pattern: PCRE2_SPTR, plength: PCRE2_SIZE, options: u32, buffptr: *mut *mut PCRE2_UCHAR, bufflenptr: *mut PCRE2_SIZE, ccontext: *mut pcre2_real_convert_context) -> i32', '0'),
   ('pcre2_converted_pattern_free_8', '(converted: *mut PCRE2_UCHAR)', '()'),
 ],
 'dfa_match': [
   ('pcre2_dfa_match_8', '(code: *const pcre2_real_code, subject: PCRE2_SPTR, length: PCRE2_SIZE, start_offset: PCRE2_SIZE, options: u32, match_data: *mut pcre2_real_match_data, mcontext: *mut pcre2_real_match_context, workspace: *mut i32, wscount: PCRE2_SIZE) -> i32', '0'),
 ],
 'error': [
   ('pcre2_get_error_message_8', '(enumber: i32, buffer: *mut PCRE2_UCHAR, size: PCRE2_SIZE) -> i32', '0'),
 ],
 'extuni': [
   ('_pcre2_extuni_8', '(c: u32, eptr: PCRE2_SPTR, start_subject: PCRE2_SPTR, end_subject: PCRE2_SPTR, utf: BOOL, xcount: *mut i32) -> PCRE2_SPTR', 'core::ptr::null()'),
 ],
 'find_bracket': [
   ('_pcre2_find_bracket_8', '(code: PCRE2_SPTR, utf: BOOL, number: i32) -> PCRE2_SPTR', 'core::ptr::null()'),
 ],
 'jit': [
   ('pcre2_jit_compile_8', '(code: *mut pcre2_real_code, options: u32) -> i32', '0'),
   ('pcre2_jit_match_8', '(code: *const pcre2_real_code, subject: PCRE2_SPTR, length: PCRE2_SIZE, start_offset: PCRE2_SIZE, options: u32, match_data: *mut pcre2_real_match_data, mcontext: *mut pcre2_real_match_context) -> i32', '0'),
   ('pcre2_jit_free_unused_memory_8', '(gcontext: *mut pcre2_real_general_context)', '()'),
   ('pcre2_jit_stack_create_8', '(startsize: usize, maxsize: usize, gcontext: *mut pcre2_real_general_context) -> *mut pcre2_real_jit_stack', 'core::ptr::null_mut()'),
   ('pcre2_jit_stack_assign_8', '(mcontext: *mut pcre2_real_match_context, callback: JitCallbackFn, callback_data: *mut c_void)', '()'),
   ('pcre2_jit_stack_free_8', '(jit_stack: *mut pcre2_real_jit_stack)', '()'),
   ('_pcre2_jit_free_8', '(executable_jit: *mut c_void, memctl: *mut pcre2_memctl)', '()'),
   ('_pcre2_jit_free_rodata_8', '(current: *mut c_void, next: *mut c_void)', '()'),
   ('_pcre2_jit_get_size_8', '(executable_jit: *mut c_void) -> usize', '0'),
   ('_pcre2_jit_get_target_8', '() -> *const c_char', 'core::ptr::null()'),
 ],
 'maketables': [
   ('pcre2_maketables_8', '(gcontext: *mut pcre2_real_general_context) -> *const u8', 'core::ptr::null()'),
   ('pcre2_maketables_free_8', '(gcontext: *mut pcre2_real_general_context, tables: *const u8)', '()'),
 ],
 'matcher': [
   ('pcre2_match_8', '(code: *const pcre2_real_code, subject: PCRE2_SPTR, length: PCRE2_SIZE, start_offset: PCRE2_SIZE, options: u32, match_data: *mut pcre2_real_match_data, mcontext: *mut pcre2_real_match_context) -> i32', '0'),
 ],
 'match_data': [
   ('pcre2_match_data_create_8', '(oveccount: u32, gcontext: *mut pcre2_real_general_context) -> *mut pcre2_real_match_data', 'core::ptr::null_mut()'),
   ('pcre2_match_data_create_from_pattern_8', '(code: *const pcre2_real_code, gcontext: *mut pcre2_real_general_context) -> *mut pcre2_real_match_data', 'core::ptr::null_mut()'),
   ('pcre2_match_data_free_8', '(match_data: *mut pcre2_real_match_data)', '()'),
   ('pcre2_get_mark_8', '(match_data: *mut pcre2_real_match_data) -> PCRE2_SPTR', 'core::ptr::null()'),
   ('pcre2_get_ovector_pointer_8', '(match_data: *mut pcre2_real_match_data) -> *mut PCRE2_SIZE', 'core::ptr::null_mut()'),
   ('pcre2_get_ovector_count_8', '(match_data: *mut pcre2_real_match_data) -> u32', '0'),
   ('pcre2_get_startchar_8', '(match_data: *mut pcre2_real_match_data) -> PCRE2_SIZE', '0'),
   ('pcre2_get_match_data_size_8', '(match_data: *mut pcre2_real_match_data) -> PCRE2_SIZE', '0'),
   ('pcre2_get_match_data_heapframes_size_8', '(match_data: *mut pcre2_real_match_data) -> PCRE2_SIZE', '0'),
 ],
 'match_next': [
   ('pcre2_next_match_8', '(match_data: *mut pcre2_real_match_data, lengthptr: *mut PCRE2_SIZE, optionsptr: *mut u32) -> i32', '0'),
 ],
 'newline': [
   ('_pcre2_is_newline_8', '(ptr: PCRE2_SPTR, type_: u32, endptr: PCRE2_SPTR, lenptr: *mut u32, utf: BOOL) -> BOOL', '0'),
   ('_pcre2_was_newline_8', '(ptr: PCRE2_SPTR, type_: u32, startptr: PCRE2_SPTR, lenptr: *mut u32, utf: BOOL) -> BOOL', '0'),
 ],
 'ord2utf': [
   ('_pcre2_ord2utf_8', '(cvalue: u32, buffer: *mut PCRE2_UCHAR) -> u32', '0'),
 ],
 'pattern_info': [
   ('pcre2_pattern_info_8', '(code: *const pcre2_real_code, what: u32, where_: *mut c_void) -> i32', '0'),
   ('pcre2_callout_enumerate_8', '(code: *const pcre2_real_code, callback: CalloutEnumerateFn, callout_data: *mut c_void) -> i32', '0'),
 ],
 'script_run': [
   ('_pcre2_script_run_8', '(ptr: PCRE2_SPTR, endptr: PCRE2_SPTR, utf: BOOL) -> BOOL', '0'),
 ],
 'serialize': [
   ('pcre2_serialize_encode_8', '(codes: *const *const pcre2_real_code, number_of_codes: i32, serialized_bytes: *mut *mut u8, serialized_size: *mut PCRE2_SIZE, gcontext: *mut pcre2_real_general_context) -> i32', '0'),
   ('pcre2_serialize_decode_8', '(codes: *mut *mut pcre2_real_code, number_of_codes: i32, bytes: *const u8, gcontext: *mut pcre2_real_general_context) -> i32', '0'),
   ('pcre2_serialize_get_number_of_codes_8', '(bytes: *const u8) -> i32', '0'),
   ('pcre2_serialize_free_8', '(data: *mut u8)', '()'),
 ],
 'string_utils': [
   ('_pcre2_strcmp_8', '(str1: PCRE2_SPTR, str2: PCRE2_SPTR) -> i32', '0'),
   ('_pcre2_strcmp_c8_8', '(str1: PCRE2_SPTR, str2: *const c_char) -> i32', '0'),
   ('_pcre2_strncmp_8', '(str1: PCRE2_SPTR, str2: PCRE2_SPTR, len: usize) -> i32', '0'),
   ('_pcre2_strncmp_c8_8', '(str1: PCRE2_SPTR, str2: *const c_char, len: usize) -> i32', '0'),
   ('_pcre2_strlen_8', '(str: PCRE2_SPTR) -> PCRE2_SIZE', '0'),
   ('_pcre2_strcpy_c8_8', '(buffer: *mut PCRE2_UCHAR, str: *const c_char) -> PCRE2_SIZE', '0'),
 ],
 'study': [
   ('_pcre2_study_8', '(re: *mut pcre2_real_code) -> i32', '0'),
 ],
 'substitute': [
   ('pcre2_substitute_8', '(code: *const pcre2_real_code, subject: PCRE2_SPTR, length: PCRE2_SIZE, start_offset: PCRE2_SIZE, options: u32, match_data: *mut pcre2_real_match_data, mcontext: *mut pcre2_real_match_context, replacement: PCRE2_SPTR, rlength: PCRE2_SIZE, buffer: *mut PCRE2_UCHAR, blength: *mut PCRE2_SIZE) -> i32', '0'),
 ],
 'substring': [
   ('pcre2_substring_copy_byname_8', '(match_data: *mut pcre2_real_match_data, stringname: PCRE2_SPTR, buffer: *mut PCRE2_UCHAR, sizeptr: *mut PCRE2_SIZE) -> i32', '0'),
   ('pcre2_substring_copy_bynumber_8', '(match_data: *mut pcre2_real_match_data, stringnumber: u32, buffer: *mut PCRE2_UCHAR, sizeptr: *mut PCRE2_SIZE) -> i32', '0'),
   ('pcre2_substring_free_8', '(string: *mut PCRE2_UCHAR)', '()'),
   ('pcre2_substring_get_byname_8', '(match_data: *mut pcre2_real_match_data, stringname: PCRE2_SPTR, stringptr: *mut *mut PCRE2_UCHAR, sizeptr: *mut PCRE2_SIZE) -> i32', '0'),
   ('pcre2_substring_get_bynumber_8', '(match_data: *mut pcre2_real_match_data, stringnumber: u32, stringptr: *mut *mut PCRE2_UCHAR, sizeptr: *mut PCRE2_SIZE) -> i32', '0'),
   ('pcre2_substring_length_byname_8', '(match_data: *mut pcre2_real_match_data, stringname: PCRE2_SPTR, lengthptr: *mut PCRE2_SIZE) -> i32', '0'),
   ('pcre2_substring_length_bynumber_8', '(match_data: *mut pcre2_real_match_data, stringnumber: u32, lengthptr: *mut PCRE2_SIZE) -> i32', '0'),
   ('pcre2_substring_nametable_scan_8', '(code: *const pcre2_real_code, stringname: PCRE2_SPTR, firstptr: *mut PCRE2_SPTR, lastptr: *mut PCRE2_SPTR) -> i32', '0'),
   ('pcre2_substring_number_from_name_8', '(code: *const pcre2_real_code, stringname: PCRE2_SPTR) -> i32', '0'),
   ('pcre2_substring_list_free_8', '(list: *mut *mut PCRE2_UCHAR)', '()'),
   ('pcre2_substring_list_get_8', '(match_data: *mut pcre2_real_match_data, listptr: *mut *mut *mut PCRE2_UCHAR, lengthsptr: *mut *mut PCRE2_SIZE) -> i32', '0'),
 ],
 'valid_utf': [
   ('_pcre2_valid_utf_8', '(string: PCRE2_SPTR, length: PCRE2_SIZE, erroroffset: *mut PCRE2_SIZE) -> i32', '0'),
 ],
 'xclass': [
   ('_pcre2_xclass_8', '(c: u32, data: PCRE2_SPTR, char_lists_end: *const u8, utf: BOOL) -> BOOL', '0'),
   ('_pcre2_eclass_8', '(c: u32, data_start: PCRE2_SPTR, data_end: PCRE2_SPTR, char_lists_end: *const u8, utf: BOOL) -> BOOL', '0'),
 ],
}

DATA_MODULES = ['chartables', 'tables', 'ucd']
SUPPORT_MODULES = ['consts', 'types', 'macros']

header = '''//! Translated from %s.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};
'''

CFILE = {
 'auto_possess': 'pcre2_auto_possess.c', 'chkdint': 'pcre2_chkdint.c',
 'compile': 'pcre2_compile.c', 'compile_cgroup': 'pcre2_compile_cgroup.c',
 'compile_class': 'pcre2_compile_class.c', 'config': 'pcre2_config.c',
 'context': 'pcre2_context.c', 'convert': 'pcre2_convert.c',
 'dfa_match': 'pcre2_dfa_match.c', 'error': 'pcre2_error.c',
 'extuni': 'pcre2_extuni.c', 'find_bracket': 'pcre2_find_bracket.c',
 'jit': 'pcre2_jit_compile.c (no JIT support)', 'maketables': 'pcre2_maketables.c',
 'matcher': 'pcre2_match.c', 'match_data': 'pcre2_match_data.c',
 'match_next': 'pcre2_match_next.c', 'newline': 'pcre2_newline.c',
 'ord2utf': 'pcre2_ord2utf.c', 'pattern_info': 'pcre2_pattern_info.c',
 'script_run': 'pcre2_script_run.c', 'serialize': 'pcre2_serialize.c',
 'string_utils': 'pcre2_string_utils.c', 'study': 'pcre2_study.c',
 'substitute': 'pcre2_substitute.c', 'substring': 'pcre2_substring.c',
 'valid_utf': 'pcre2_valid_utf.c', 'xclass': 'pcre2_xclass.c',
}

for mod, funcs in FUNCS.items():
    path = os.path.join(OUT, mod + '.rs')
    if os.path.exists(path):
        continue
    with open(path, 'w') as f:
        f.write(header % CFILE[mod])
        f.write('\n// TODO: translate %s\n\n' % CFILE[mod])
        for sym, sig, body in funcs:
            f.write('#[unsafe(no_mangle)]\npub unsafe extern "C" fn %s%s {\n    %s\n}\n\n'
                    % (sym, sig, body))

# lib.rs
mods = sorted(set(list(FUNCS.keys()) + DATA_MODULES))
with open(os.path.join(OUT, 'lib.rs'), 'w') as f:
    f.write('''//! A translation of the PCRE2 library (10.48-DEV, 8-bit mode, LINK_SIZE 2,
//! SUPPORT_UNICODE, no JIT) from C to Rust.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(unused_unsafe, unused_assignments, unused_labels)]

#[macro_use]
pub mod macros;
pub mod consts;
pub mod types;

''')
    for m in mods:
        f.write('pub mod %s;\n' % m)
    f.write('\n')
print('skeleton written')
