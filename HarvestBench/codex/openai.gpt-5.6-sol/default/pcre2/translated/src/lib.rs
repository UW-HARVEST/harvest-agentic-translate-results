//! PCRE2 10.48-DEV 8-bit ABI.

use std::ffi::{c_int, c_void};

pub type GeneralContext = c_void;
pub type CompileContext = c_void;
pub type MatchContext = c_void;
pub type ConvertContext = c_void;
pub type Code = c_void;
pub type MatchData = c_void;
pub type JitStack = c_void;

type MallocCallback =
    Option<unsafe extern "C" fn(size: usize, memory_data: *mut c_void) -> *mut c_void>;
type FreeCallback =
    Option<unsafe extern "C" fn(pointer: *mut c_void, memory_data: *mut c_void)>;
type CompileGuardCallback =
    Option<unsafe extern "C" fn(depth: u32, user_data: *mut c_void) -> c_int>;
type CalloutCallback =
    Option<unsafe extern "C" fn(block: *mut c_void, user_data: *mut c_void) -> c_int>;
type SubstituteCaseCallback = Option<
    unsafe extern "C" fn(
        input: *const u8,
        input_length: usize,
        output: *mut u8,
        output_length: usize,
        case_type: c_int,
        user_data: *mut c_void,
    ) -> usize,
>;
type JitCallback =
    Option<unsafe extern "C" fn(user_data: *mut c_void) -> *mut JitStack>;

macro_rules! forward {
    (
        $(
            fn $public:ident => $internal:ident (
                $( $argument:ident : $argument_type:ty ),* $(,)?
            ) -> $return_type:ty;
        )*
    ) => {
        unsafe extern "C" {
            $(
                fn $internal(
                    $( $argument: $argument_type ),*
                ) -> $return_type;
            )*
        }

        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $public(
                $( $argument: $argument_type ),*
            ) -> $return_type {
                unsafe { $internal($( $argument ),*) }
            }
        )*
    };
}

forward! {
    fn pcre2_config_8 => rust_internal_pcre2_config_8(
        what: u32,
        where_: *mut c_void,
    ) -> c_int;

    fn pcre2_general_context_copy_8 => rust_internal_pcre2_general_context_copy_8(
        context: *mut GeneralContext,
    ) -> *mut GeneralContext;
    fn pcre2_general_context_create_8 => rust_internal_pcre2_general_context_create_8(
        malloc_callback: MallocCallback,
        free_callback: FreeCallback,
        memory_data: *mut c_void,
    ) -> *mut GeneralContext;
    fn pcre2_general_context_free_8 => rust_internal_pcre2_general_context_free_8(
        context: *mut GeneralContext,
    ) -> ();

    fn pcre2_compile_context_copy_8 => rust_internal_pcre2_compile_context_copy_8(
        context: *mut CompileContext,
    ) -> *mut CompileContext;
    fn pcre2_compile_context_create_8 => rust_internal_pcre2_compile_context_create_8(
        general_context: *mut GeneralContext,
    ) -> *mut CompileContext;
    fn pcre2_compile_context_free_8 => rust_internal_pcre2_compile_context_free_8(
        context: *mut CompileContext,
    ) -> ();
    fn pcre2_set_bsr_8 => rust_internal_pcre2_set_bsr_8(
        context: *mut CompileContext,
        value: u32,
    ) -> c_int;
    fn pcre2_set_character_tables_8 => rust_internal_pcre2_set_character_tables_8(
        context: *mut CompileContext,
        tables: *const u8,
    ) -> c_int;
    fn pcre2_set_compile_extra_options_8 => rust_internal_pcre2_set_compile_extra_options_8(
        context: *mut CompileContext,
        options: u32,
    ) -> c_int;
    fn pcre2_set_max_pattern_length_8 => rust_internal_pcre2_set_max_pattern_length_8(
        context: *mut CompileContext,
        length: usize,
    ) -> c_int;
    fn pcre2_set_max_pattern_compiled_length_8 =>
        rust_internal_pcre2_set_max_pattern_compiled_length_8(
            context: *mut CompileContext,
            length: usize,
        ) -> c_int;
    fn pcre2_set_max_varlookbehind_8 => rust_internal_pcre2_set_max_varlookbehind_8(
        context: *mut CompileContext,
        length: u32,
    ) -> c_int;
    fn pcre2_set_newline_8 => rust_internal_pcre2_set_newline_8(
        context: *mut CompileContext,
        newline: u32,
    ) -> c_int;
    fn pcre2_set_parens_nest_limit_8 => rust_internal_pcre2_set_parens_nest_limit_8(
        context: *mut CompileContext,
        limit: u32,
    ) -> c_int;
    fn pcre2_set_compile_recursion_guard_8 =>
        rust_internal_pcre2_set_compile_recursion_guard_8(
            context: *mut CompileContext,
            callback: CompileGuardCallback,
            user_data: *mut c_void,
        ) -> c_int;
    fn pcre2_set_optimize_8 => rust_internal_pcre2_set_optimize_8(
        context: *mut CompileContext,
        directive: u32,
    ) -> c_int;

    fn pcre2_match_context_copy_8 => rust_internal_pcre2_match_context_copy_8(
        context: *mut MatchContext,
    ) -> *mut MatchContext;
    fn pcre2_match_context_create_8 => rust_internal_pcre2_match_context_create_8(
        general_context: *mut GeneralContext,
    ) -> *mut MatchContext;
    fn pcre2_match_context_free_8 => rust_internal_pcre2_match_context_free_8(
        context: *mut MatchContext,
    ) -> ();
    fn pcre2_set_callout_8 => rust_internal_pcre2_set_callout_8(
        context: *mut MatchContext,
        callback: CalloutCallback,
        user_data: *mut c_void,
    ) -> c_int;
    fn pcre2_set_substitute_callout_8 => rust_internal_pcre2_set_substitute_callout_8(
        context: *mut MatchContext,
        callback: CalloutCallback,
        user_data: *mut c_void,
    ) -> c_int;
    fn pcre2_set_substitute_case_callout_8 =>
        rust_internal_pcre2_set_substitute_case_callout_8(
            context: *mut MatchContext,
            callback: SubstituteCaseCallback,
            user_data: *mut c_void,
        ) -> c_int;
    fn pcre2_set_depth_limit_8 => rust_internal_pcre2_set_depth_limit_8(
        context: *mut MatchContext,
        limit: u32,
    ) -> c_int;
    fn pcre2_set_heap_limit_8 => rust_internal_pcre2_set_heap_limit_8(
        context: *mut MatchContext,
        limit: u32,
    ) -> c_int;
    fn pcre2_set_match_limit_8 => rust_internal_pcre2_set_match_limit_8(
        context: *mut MatchContext,
        limit: u32,
    ) -> c_int;
    fn pcre2_set_offset_limit_8 => rust_internal_pcre2_set_offset_limit_8(
        context: *mut MatchContext,
        limit: usize,
    ) -> c_int;
    fn pcre2_set_recursion_limit_8 => rust_internal_pcre2_set_recursion_limit_8(
        context: *mut MatchContext,
        limit: u32,
    ) -> c_int;
    fn pcre2_set_recursion_memory_management_8 =>
        rust_internal_pcre2_set_recursion_memory_management_8(
            context: *mut MatchContext,
            malloc_callback: MallocCallback,
            free_callback: FreeCallback,
            memory_data: *mut c_void,
        ) -> c_int;

    fn pcre2_convert_context_copy_8 => rust_internal_pcre2_convert_context_copy_8(
        context: *mut ConvertContext,
    ) -> *mut ConvertContext;
    fn pcre2_convert_context_create_8 => rust_internal_pcre2_convert_context_create_8(
        general_context: *mut GeneralContext,
    ) -> *mut ConvertContext;
    fn pcre2_convert_context_free_8 => rust_internal_pcre2_convert_context_free_8(
        context: *mut ConvertContext,
    ) -> ();
    fn pcre2_set_glob_escape_8 => rust_internal_pcre2_set_glob_escape_8(
        context: *mut ConvertContext,
        escape: u32,
    ) -> c_int;
    fn pcre2_set_glob_separator_8 => rust_internal_pcre2_set_glob_separator_8(
        context: *mut ConvertContext,
        separator: u32,
    ) -> c_int;

    fn pcre2_compile_8 => rust_internal_pcre2_compile_8(
        pattern: *const u8,
        length: usize,
        options: u32,
        error_code: *mut c_int,
        error_offset: *mut usize,
        context: *mut CompileContext,
    ) -> *mut Code;
    fn pcre2_code_free_8 => rust_internal_pcre2_code_free_8(
        code: *mut Code,
    ) -> ();
    fn pcre2_code_copy_8 => rust_internal_pcre2_code_copy_8(
        code: *const Code,
    ) -> *mut Code;
    fn pcre2_code_copy_with_tables_8 => rust_internal_pcre2_code_copy_with_tables_8(
        code: *const Code,
    ) -> *mut Code;

    fn pcre2_pattern_info_8 => rust_internal_pcre2_pattern_info_8(
        code: *const Code,
        what: u32,
        where_: *mut c_void,
    ) -> c_int;
    fn pcre2_callout_enumerate_8 => rust_internal_pcre2_callout_enumerate_8(
        code: *const Code,
        callback: CalloutCallback,
        user_data: *mut c_void,
    ) -> c_int;

    fn pcre2_match_data_create_8 => rust_internal_pcre2_match_data_create_8(
        ovector_count: u32,
        general_context: *mut GeneralContext,
    ) -> *mut MatchData;
    fn pcre2_match_data_create_from_pattern_8 =>
        rust_internal_pcre2_match_data_create_from_pattern_8(
            code: *const Code,
            general_context: *mut GeneralContext,
        ) -> *mut MatchData;
    fn pcre2_match_data_free_8 => rust_internal_pcre2_match_data_free_8(
        match_data: *mut MatchData,
    ) -> ();
    fn pcre2_dfa_match_8 => rust_internal_pcre2_dfa_match_8(
        code: *const Code,
        subject: *const u8,
        length: usize,
        start_offset: usize,
        options: u32,
        match_data: *mut MatchData,
        match_context: *mut MatchContext,
        workspace: *mut c_int,
        workspace_count: usize,
    ) -> c_int;
    fn pcre2_match_8 => rust_internal_pcre2_match_8(
        code: *const Code,
        subject: *const u8,
        length: usize,
        start_offset: usize,
        options: u32,
        match_data: *mut MatchData,
        match_context: *mut MatchContext,
    ) -> c_int;
    fn pcre2_get_mark_8 => rust_internal_pcre2_get_mark_8(
        match_data: *mut MatchData,
    ) -> *const u8;
    fn pcre2_get_match_data_size_8 => rust_internal_pcre2_get_match_data_size_8(
        match_data: *mut MatchData,
    ) -> usize;
    fn pcre2_get_match_data_heapframes_size_8 =>
        rust_internal_pcre2_get_match_data_heapframes_size_8(
            match_data: *mut MatchData,
        ) -> usize;
    fn pcre2_get_ovector_count_8 => rust_internal_pcre2_get_ovector_count_8(
        match_data: *mut MatchData,
    ) -> u32;
    fn pcre2_get_ovector_pointer_8 => rust_internal_pcre2_get_ovector_pointer_8(
        match_data: *mut MatchData,
    ) -> *mut usize;
    fn pcre2_get_startchar_8 => rust_internal_pcre2_get_startchar_8(
        match_data: *mut MatchData,
    ) -> usize;
    fn pcre2_next_match_8 => rust_internal_pcre2_next_match_8(
        match_data: *mut MatchData,
        start_offset: *mut usize,
        options: *mut u32,
    ) -> c_int;

    fn pcre2_substring_copy_byname_8 => rust_internal_pcre2_substring_copy_byname_8(
        match_data: *mut MatchData,
        name: *const u8,
        buffer: *mut u8,
        buffer_size: *mut usize,
    ) -> c_int;
    fn pcre2_substring_copy_bynumber_8 => rust_internal_pcre2_substring_copy_bynumber_8(
        match_data: *mut MatchData,
        number: u32,
        buffer: *mut u8,
        buffer_size: *mut usize,
    ) -> c_int;
    fn pcre2_substring_free_8 => rust_internal_pcre2_substring_free_8(
        substring: *mut u8,
    ) -> ();
    fn pcre2_substring_get_byname_8 => rust_internal_pcre2_substring_get_byname_8(
        match_data: *mut MatchData,
        name: *const u8,
        substring: *mut *mut u8,
        length: *mut usize,
    ) -> c_int;
    fn pcre2_substring_get_bynumber_8 => rust_internal_pcre2_substring_get_bynumber_8(
        match_data: *mut MatchData,
        number: u32,
        substring: *mut *mut u8,
        length: *mut usize,
    ) -> c_int;
    fn pcre2_substring_length_byname_8 => rust_internal_pcre2_substring_length_byname_8(
        match_data: *mut MatchData,
        name: *const u8,
        length: *mut usize,
    ) -> c_int;
    fn pcre2_substring_length_bynumber_8 => rust_internal_pcre2_substring_length_bynumber_8(
        match_data: *mut MatchData,
        number: u32,
        length: *mut usize,
    ) -> c_int;
    fn pcre2_substring_nametable_scan_8 => rust_internal_pcre2_substring_nametable_scan_8(
        code: *const Code,
        name: *const u8,
        first: *mut *const u8,
        last: *mut *const u8,
    ) -> c_int;
    fn pcre2_substring_number_from_name_8 =>
        rust_internal_pcre2_substring_number_from_name_8(
            code: *const Code,
            name: *const u8,
        ) -> c_int;
    fn pcre2_substring_list_free_8 => rust_internal_pcre2_substring_list_free_8(
        list: *mut *mut u8,
    ) -> ();
    fn pcre2_substring_list_get_8 => rust_internal_pcre2_substring_list_get_8(
        match_data: *mut MatchData,
        list: *mut *mut *mut u8,
        lengths: *mut *mut usize,
    ) -> c_int;

    fn pcre2_serialize_encode_8 => rust_internal_pcre2_serialize_encode_8(
        codes: *const *const Code,
        code_count: i32,
        serialized_bytes: *mut *mut u8,
        serialized_size: *mut usize,
        general_context: *mut GeneralContext,
    ) -> i32;
    fn pcre2_serialize_decode_8 => rust_internal_pcre2_serialize_decode_8(
        codes: *mut *mut Code,
        code_count: i32,
        serialized_bytes: *const u8,
        general_context: *mut GeneralContext,
    ) -> i32;
    fn pcre2_serialize_get_number_of_codes_8 =>
        rust_internal_pcre2_serialize_get_number_of_codes_8(
            serialized_bytes: *const u8,
        ) -> i32;
    fn pcre2_serialize_free_8 => rust_internal_pcre2_serialize_free_8(
        serialized_bytes: *mut u8,
    ) -> ();

    fn pcre2_substitute_8 => rust_internal_pcre2_substitute_8(
        code: *const Code,
        subject: *const u8,
        length: usize,
        start_offset: usize,
        options: u32,
        match_data: *mut MatchData,
        match_context: *mut MatchContext,
        replacement: *const u8,
        replacement_length: usize,
        output_buffer: *mut u8,
        output_length: *mut usize,
    ) -> c_int;

    fn pcre2_pattern_convert_8 => rust_internal_pcre2_pattern_convert_8(
        pattern: *const u8,
        length: usize,
        options: u32,
        converted_pattern: *mut *mut u8,
        converted_length: *mut usize,
        convert_context: *mut ConvertContext,
    ) -> c_int;
    fn pcre2_converted_pattern_free_8 => rust_internal_pcre2_converted_pattern_free_8(
        converted_pattern: *mut u8,
    ) -> ();

    fn pcre2_jit_compile_8 => rust_internal_pcre2_jit_compile_8(
        code: *mut Code,
        options: u32,
    ) -> c_int;
    fn pcre2_jit_match_8 => rust_internal_pcre2_jit_match_8(
        code: *const Code,
        subject: *const u8,
        length: usize,
        start_offset: usize,
        options: u32,
        match_data: *mut MatchData,
        match_context: *mut MatchContext,
    ) -> c_int;
    fn pcre2_jit_free_unused_memory_8 => rust_internal_pcre2_jit_free_unused_memory_8(
        general_context: *mut GeneralContext,
    ) -> ();
    fn pcre2_jit_stack_create_8 => rust_internal_pcre2_jit_stack_create_8(
        start_size: usize,
        max_size: usize,
        general_context: *mut GeneralContext,
    ) -> *mut JitStack;
    fn pcre2_jit_stack_assign_8 => rust_internal_pcre2_jit_stack_assign_8(
        match_context: *mut MatchContext,
        callback: JitCallback,
        user_data: *mut c_void,
    ) -> ();
    fn pcre2_jit_stack_free_8 => rust_internal_pcre2_jit_stack_free_8(
        jit_stack: *mut JitStack,
    ) -> ();

    fn pcre2_get_error_message_8 => rust_internal_pcre2_get_error_message_8(
        error_code: c_int,
        buffer: *mut u8,
        buffer_size: usize,
    ) -> c_int;
    fn pcre2_maketables_8 => rust_internal_pcre2_maketables_8(
        general_context: *mut GeneralContext,
    ) -> *const u8;
    fn pcre2_maketables_free_8 => rust_internal_pcre2_maketables_free_8(
        general_context: *mut GeneralContext,
        tables: *const u8,
    ) -> ();
}
