#[path = "../../translation/src/consts.rs"] mod consts;
#[path = "../../translation/src/types.rs"] mod types;
use types::*;
fn main() {
    macro_rules! p { ($t:ty) => { println!("size {} = {} align {}", stringify!($t), core::mem::size_of::<$t>(), core::mem::align_of::<$t>()); } }
    p!(pcre2_memctl); p!(pcre2_real_general_context); p!(pcre2_real_compile_context);
    p!(pcre2_real_match_context); p!(pcre2_real_convert_context); p!(pcre2_real_code);
    p!(pcre2_callout_block); p!(pcre2_callout_enumerate_block); p!(pcre2_substitute_callout_block);
    p!(ucd_record); p!(ucp_type_table); p!(pcre2_serialized_data); p!(named_group);
    p!(compile_block); p!(match_block); p!(dfa_match_block); p!(class_ranges);
    p!(recurse_arguments); p!(eclass_op_info); p!(heapframe_fields); p!(pcre2_real_jit_stack);
    println!("offset match_data.ovector = {}", core::mem::offset_of!(pcre2_real_match_data, ovector));
    println!("offset heapframe.eptr = {}", core::mem::offset_of!(heapframe, eptr));
    println!("offset heapframe.ovector = {}", core::mem::offset_of!(heapframe, ovector));
    println!("offset heapframe.fields = {}", core::mem::offset_of!(heapframe, fields));
    println!("align heapframe = {}", core::mem::align_of::<heapframe>());
    println!("offset code.start_bitmap = {}", core::mem::offset_of!(pcre2_real_code, tables));
    println!("offset code.optimization_flags = {}", core::mem::offset_of!(pcre2_real_code, optimization_flags));
    println!("offset compile_block.classbits = {}", core::mem::offset_of!(compile_block, classbits));
    println!("offset compile_block.char_lists_size = {}", core::mem::offset_of!(compile_block, char_lists_size));
    println!("offset match_block.callout = {}", core::mem::offset_of!(match_block, callout));
    println!("offset dfa_match_block.recursive = {}", core::mem::offset_of!(dfa_match_block, recursive));
}
