#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_parens, unused_assignments, unused_mut, unused_variables, dead_code)]
#![allow(unused_imports, unused_unsafe, unreachable_code, unused_labels)]
#![allow(static_mut_refs)]

pub mod compile_h;
pub mod internal;
pub mod pcre2_pub;
pub mod tables;
pub mod ucd_data;
pub mod ucp;
pub mod compile_tables;
pub mod auto_possess;
pub mod chkdint;
pub mod compile;
pub mod compile_util;
pub mod compile_parse;
pub mod compile_branch;
pub mod compile_cgroup;
pub mod compile_class;
pub mod config;
pub mod context;
pub mod convert;
pub mod dfa_match;
pub mod error;
pub mod extuni;
pub mod find_bracket;
pub mod jit_compile;
pub mod maketables;
pub mod match_data;
pub mod match_next;
pub mod matcher;
pub mod matcher_core;
pub mod newline;
pub mod ord2utf;
pub mod pattern_info;
pub mod script_run;
pub mod serialize;
pub mod string_utils;
pub mod study;
pub mod substitute;
pub mod substring;
pub mod valid_utf;
pub mod xclass;

#[cfg(test)]
mod layout_tests {
    use crate::internal::*;
    use core::mem::{align_of, size_of};
    macro_rules! chk { ($t:ty, $s:expr, $a:expr) => {
        assert_eq!(size_of::<$t>(), $s, concat!("size of ", stringify!($t)));
        assert_eq!(align_of::<$t>(), $a, concat!("align of ", stringify!($t)));
    }}
    #[test]
    fn sizes() {
        chk!(pcre2_memctl, 24, 8);
        chk!(pcre2_real_general_context, 24, 8);
        chk!(pcre2_real_compile_context, 88, 8);
        chk!(pcre2_real_match_context, 96, 8);
        chk!(pcre2_real_convert_context, 32, 8);
        chk!(pcre2_real_code, 152, 8);
        chk!(pcre2_real_match_data, 1048696, 8);
        chk!(heapframe, 1048696, 8);
        chk!(match_block, 272, 8);
        chk!(dfa_match_block, 168, 8);
        chk!(compile_block, 360, 8);
        chk!(pcre2_callout_block, 112, 8);
        chk!(pcre2_callout_enumerate_block, 56, 8);
        chk!(pcre2_substitute_callout_block, 56, 8);
        chk!(ucd_record, 12, 4);
        chk!(ucp_type_table, 6, 2);
        chk!(named_group, 16, 8);
        chk!(open_capitem, 16, 8);
        chk!(class_ranges, 32, 8);
        chk!(recurse_arguments, 24, 8);
        chk!(compile_data, 8, 8);
        chk!(eclass_op_info, 56, 8);
        chk!(pcre2_serialized_data, 16, 4);
        chk!(dfa_recursion_info, 32, 8);
        chk!(pcre2_real_jit_stack, 32, 8);
        chk!(class_bits_storage, 32, 4);
        chk!(hf_fields, 32, 8);
    }
    #[test]
    fn offsets() {
        macro_rules! off { ($t:ty, $f:ident, $v:expr) => {
            assert_eq!(core::mem::offset_of!($t, $f), $v, concat!(stringify!($t), ".", stringify!($f)));
        }}
        off!(pcre2_real_compile_context, optimization_flags, 80);
        off!(pcre2_real_match_context, depth_limit, 88);
        off!(pcre2_real_code, optimization_flags, 144);
        off!(pcre2_real_code, start_bitmap, 40);
        off!(pcre2_real_code, blocksize, 72);
        off!(pcre2_real_match_data, ovector, 120);
        off!(pcre2_real_match_data, rc, 112);
        off!(heapframe, fields, 32);
        off!(heapframe, eptr, 64);
        off!(heapframe, ovector, 120);
        off!(match_block, lcc, 64);
        off!(match_block, partial, 104);
        off!(match_block, nl, 244);
        off!(match_block, cb, 248);
        off!(match_block, callout, 264);
        off!(compile_block, classbits, 176);
        off!(compile_block, names_found, 208);
        off!(compile_block, named_groups, 216);
        off!(compile_block, nl, 288);
        off!(compile_block, class_op_used, 292);
        off!(compile_block, req_varyopt, 308);
        off!(compile_block, max_lookbehind, 316);
        off!(compile_block, had_accept, 320);
        off!(compile_block, first_data, 336);
        off!(compile_block, char_lists_size, 352);
        off!(pcre2_callout_block, callout_flags, 104);
        off!(eclass_op_info, bits, 20);
        off!(ucd_record, other_case, 4);
    }
}
