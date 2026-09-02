//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and exposes every exported PCRE2 symbol as a raw function/data pointer.
//!
//! Nothing in here calls a Rust function directly — every call goes through the
//! dynamic-symbol table of a shared object, exactly like an external C consumer,
//! so the `#[no_mangle]` / `extern "C"` wrappers are under test too.
#![allow(dead_code, non_camel_case_types, non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;

pub type Ctx = *mut c_void;
pub type Code = *mut c_void;
pub type MData = *mut c_void;
pub type Sz = usize;

pub const PCRE2_ZERO_TERMINATED: Sz = !0usize;
pub const PCRE2_UNSET: Sz = !0usize;

unsafe fn addr(lib: &Library, name: &[u8]) -> *mut c_void {
    let s: Symbol<*mut c_void> = unsafe { lib.get(name) }
        .unwrap_or_else(|e| panic!("symbol {:?}: {}", String::from_utf8_lossy(name), e));
    *s
}

macro_rules! def_api {
    ($( $f:ident : $n:expr => $t:ty ; )*) => {
        pub struct Api {
            pub tag: &'static str,
            _lib: Library,
            $( pub $f : $t , )*
        }
        impl Api {
            pub fn load(tag: &'static str, path: &std::path::Path) -> Api {
                let lib = unsafe { Library::new(path) }
                    .unwrap_or_else(|e| panic!("dlopen {}: {}", path.display(), e));
                $( let $f : $t = unsafe {
                    std::mem::transmute(addr(&lib, concat!($n, "\0").as_bytes()))
                }; )*
                Api { tag, _lib: lib, $( $f , )* }
            }
        }
    }
}

def_api! {
    // ---- general information ------------------------------------------------
    config: "pcre2_config_8" => unsafe extern "C" fn(u32, *mut c_void) -> i32;
    get_error_message: "pcre2_get_error_message_8" => unsafe extern "C" fn(i32, *mut u8, Sz) -> i32;
    maketables: "pcre2_maketables_8" => unsafe extern "C" fn(Ctx) -> *const u8;
    maketables_free: "pcre2_maketables_free_8" => unsafe extern "C" fn(Ctx, *const u8);

    // ---- contexts ----------------------------------------------------------
    general_context_create: "pcre2_general_context_create_8" => unsafe extern "C" fn(
        Option<unsafe extern "C" fn(Sz, *mut c_void) -> *mut c_void>,
        Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
        *mut c_void) -> Ctx;
    general_context_copy: "pcre2_general_context_copy_8" => unsafe extern "C" fn(Ctx) -> Ctx;
    general_context_free: "pcre2_general_context_free_8" => unsafe extern "C" fn(Ctx);

    compile_context_create: "pcre2_compile_context_create_8" => unsafe extern "C" fn(Ctx) -> Ctx;
    compile_context_copy: "pcre2_compile_context_copy_8" => unsafe extern "C" fn(Ctx) -> Ctx;
    compile_context_free: "pcre2_compile_context_free_8" => unsafe extern "C" fn(Ctx);
    set_bsr: "pcre2_set_bsr_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_character_tables: "pcre2_set_character_tables_8" => unsafe extern "C" fn(Ctx, *const u8) -> i32;
    set_compile_extra_options: "pcre2_set_compile_extra_options_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_max_pattern_length: "pcre2_set_max_pattern_length_8" => unsafe extern "C" fn(Ctx, Sz) -> i32;
    set_max_pattern_compiled_length: "pcre2_set_max_pattern_compiled_length_8" => unsafe extern "C" fn(Ctx, Sz) -> i32;
    set_max_varlookbehind: "pcre2_set_max_varlookbehind_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_newline: "pcre2_set_newline_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_parens_nest_limit: "pcre2_set_parens_nest_limit_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_compile_recursion_guard: "pcre2_set_compile_recursion_guard_8" => unsafe extern "C" fn(
        Ctx, Option<unsafe extern "C" fn(u32, *mut c_void) -> i32>, *mut c_void) -> i32;
    set_optimize: "pcre2_set_optimize_8" => unsafe extern "C" fn(Ctx, u32) -> i32;

    match_context_create: "pcre2_match_context_create_8" => unsafe extern "C" fn(Ctx) -> Ctx;
    match_context_copy: "pcre2_match_context_copy_8" => unsafe extern "C" fn(Ctx) -> Ctx;
    match_context_free: "pcre2_match_context_free_8" => unsafe extern "C" fn(Ctx);
    set_callout: "pcre2_set_callout_8" => unsafe extern "C" fn(
        Ctx, Option<unsafe extern "C" fn(*mut CalloutBlock, *mut c_void) -> i32>, *mut c_void) -> i32;
    set_substitute_callout: "pcre2_set_substitute_callout_8" => unsafe extern "C" fn(
        Ctx, Option<unsafe extern "C" fn(*mut SubstCalloutBlock, *mut c_void) -> i32>, *mut c_void) -> i32;
    set_substitute_case_callout: "pcre2_set_substitute_case_callout_8" => unsafe extern "C" fn(
        Ctx, Option<unsafe extern "C" fn(*const u8, Sz, *mut u8, Sz, i32, *mut c_void) -> Sz>, *mut c_void) -> i32;
    set_depth_limit: "pcre2_set_depth_limit_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_heap_limit: "pcre2_set_heap_limit_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_match_limit: "pcre2_set_match_limit_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_offset_limit: "pcre2_set_offset_limit_8" => unsafe extern "C" fn(Ctx, Sz) -> i32;
    set_recursion_limit: "pcre2_set_recursion_limit_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_recursion_memory_management: "pcre2_set_recursion_memory_management_8" => unsafe extern "C" fn(
        Ctx, Option<unsafe extern "C" fn(Sz, *mut c_void) -> *mut c_void>,
        Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>, *mut c_void) -> i32;

    convert_context_create: "pcre2_convert_context_create_8" => unsafe extern "C" fn(Ctx) -> Ctx;
    convert_context_copy: "pcre2_convert_context_copy_8" => unsafe extern "C" fn(Ctx) -> Ctx;
    convert_context_free: "pcre2_convert_context_free_8" => unsafe extern "C" fn(Ctx);
    set_glob_escape: "pcre2_set_glob_escape_8" => unsafe extern "C" fn(Ctx, u32) -> i32;
    set_glob_separator: "pcre2_set_glob_separator_8" => unsafe extern "C" fn(Ctx, u32) -> i32;

    // ---- compile -----------------------------------------------------------
    compile: "pcre2_compile_8" => unsafe extern "C" fn(*const u8, Sz, u32, *mut i32, *mut Sz, Ctx) -> Code;
    code_free: "pcre2_code_free_8" => unsafe extern "C" fn(Code);
    code_copy: "pcre2_code_copy_8" => unsafe extern "C" fn(Code) -> Code;
    code_copy_with_tables: "pcre2_code_copy_with_tables_8" => unsafe extern "C" fn(Code) -> Code;

    // ---- pattern info ------------------------------------------------------
    pattern_info: "pcre2_pattern_info_8" => unsafe extern "C" fn(Code, u32, *mut c_void) -> i32;
    callout_enumerate: "pcre2_callout_enumerate_8" => unsafe extern "C" fn(
        Code, Option<unsafe extern "C" fn(*mut CalloutEnumBlock, *mut c_void) -> i32>, *mut c_void) -> i32;

    // ---- match -------------------------------------------------------------
    match_data_create: "pcre2_match_data_create_8" => unsafe extern "C" fn(u32, Ctx) -> MData;
    match_data_create_from_pattern: "pcre2_match_data_create_from_pattern_8" => unsafe extern "C" fn(Code, Ctx) -> MData;
    match_data_free: "pcre2_match_data_free_8" => unsafe extern "C" fn(MData);
    pcre2_match: "pcre2_match_8" => unsafe extern "C" fn(Code, *const u8, Sz, Sz, u32, MData, Ctx) -> i32;
    dfa_match: "pcre2_dfa_match_8" => unsafe extern "C" fn(Code, *const u8, Sz, Sz, u32, MData, Ctx, *mut i32, Sz) -> i32;
    get_mark: "pcre2_get_mark_8" => unsafe extern "C" fn(MData) -> *const u8;
    get_match_data_size: "pcre2_get_match_data_size_8" => unsafe extern "C" fn(MData) -> Sz;
    get_match_data_heapframes_size: "pcre2_get_match_data_heapframes_size_8" => unsafe extern "C" fn(MData) -> Sz;
    get_ovector_count: "pcre2_get_ovector_count_8" => unsafe extern "C" fn(MData) -> u32;
    get_ovector_pointer: "pcre2_get_ovector_pointer_8" => unsafe extern "C" fn(MData) -> *mut Sz;
    get_startchar: "pcre2_get_startchar_8" => unsafe extern "C" fn(MData) -> Sz;
    next_match: "pcre2_next_match_8" => unsafe extern "C" fn(MData, *mut Sz, *mut u32) -> i32;

    // ---- substrings --------------------------------------------------------
    substring_copy_byname: "pcre2_substring_copy_byname_8" => unsafe extern "C" fn(MData, *const u8, *mut u8, *mut Sz) -> i32;
    substring_copy_bynumber: "pcre2_substring_copy_bynumber_8" => unsafe extern "C" fn(MData, u32, *mut u8, *mut Sz) -> i32;
    substring_free: "pcre2_substring_free_8" => unsafe extern "C" fn(*mut u8);
    substring_get_byname: "pcre2_substring_get_byname_8" => unsafe extern "C" fn(MData, *const u8, *mut *mut u8, *mut Sz) -> i32;
    substring_get_bynumber: "pcre2_substring_get_bynumber_8" => unsafe extern "C" fn(MData, u32, *mut *mut u8, *mut Sz) -> i32;
    substring_length_byname: "pcre2_substring_length_byname_8" => unsafe extern "C" fn(MData, *const u8, *mut Sz) -> i32;
    substring_length_bynumber: "pcre2_substring_length_bynumber_8" => unsafe extern "C" fn(MData, u32, *mut Sz) -> i32;
    substring_nametable_scan: "pcre2_substring_nametable_scan_8" => unsafe extern "C" fn(Code, *const u8, *mut *const u8, *mut *const u8) -> i32;
    substring_number_from_name: "pcre2_substring_number_from_name_8" => unsafe extern "C" fn(Code, *const u8) -> i32;
    substring_list_free: "pcre2_substring_list_free_8" => unsafe extern "C" fn(*mut *mut u8);
    substring_list_get: "pcre2_substring_list_get_8" => unsafe extern "C" fn(MData, *mut *mut *mut u8, *mut *mut Sz) -> i32;

    // ---- serialize ---------------------------------------------------------
    serialize_encode: "pcre2_serialize_encode_8" => unsafe extern "C" fn(*const Code, i32, *mut *mut u8, *mut Sz, Ctx) -> i32;
    serialize_decode: "pcre2_serialize_decode_8" => unsafe extern "C" fn(*mut Code, i32, *const u8, Ctx) -> i32;
    serialize_get_number_of_codes: "pcre2_serialize_get_number_of_codes_8" => unsafe extern "C" fn(*const u8) -> i32;
    serialize_free: "pcre2_serialize_free_8" => unsafe extern "C" fn(*mut u8);

    // ---- substitute --------------------------------------------------------
    substitute: "pcre2_substitute_8" => unsafe extern "C" fn(
        Code, *const u8, Sz, Sz, u32, MData, Ctx, *const u8, Sz, *mut u8, *mut Sz) -> i32;

    // ---- convert -----------------------------------------------------------
    pattern_convert: "pcre2_pattern_convert_8" => unsafe extern "C" fn(*const u8, Sz, u32, *mut *mut u8, *mut Sz, Ctx) -> i32;
    converted_pattern_free: "pcre2_converted_pattern_free_8" => unsafe extern "C" fn(*mut u8);

    // ---- JIT (stubs in this build) -----------------------------------------
    jit_compile: "pcre2_jit_compile_8" => unsafe extern "C" fn(Code, u32) -> i32;
    jit_match: "pcre2_jit_match_8" => unsafe extern "C" fn(Code, *const u8, Sz, Sz, u32, MData, Ctx) -> i32;
    jit_free_unused_memory: "pcre2_jit_free_unused_memory_8" => unsafe extern "C" fn(Ctx);
    jit_stack_create: "pcre2_jit_stack_create_8" => unsafe extern "C" fn(Sz, Sz, Ctx) -> *mut c_void;
    jit_stack_assign: "pcre2_jit_stack_assign_8" => unsafe extern "C" fn(Ctx, Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>, *mut c_void);
    jit_stack_free: "pcre2_jit_stack_free_8" => unsafe extern "C" fn(*mut c_void);
    priv_jit_get_size: "_pcre2_jit_get_size_8" => unsafe extern "C" fn(*mut c_void) -> Sz;
    priv_jit_get_target: "_pcre2_jit_get_target_8" => unsafe extern "C" fn() -> *const i8;
    priv_jit_free: "_pcre2_jit_free_8" => unsafe extern "C" fn(*mut c_void, *mut c_void);
    priv_jit_free_rodata: "_pcre2_jit_free_rodata_8" => unsafe extern "C" fn(*mut c_void, *mut c_void);

    // ---- private (non-API) functions ---------------------------------------
    priv_strlen: "_pcre2_strlen_8" => unsafe extern "C" fn(*const u8) -> Sz;
    priv_strcmp: "_pcre2_strcmp_8" => unsafe extern "C" fn(*const u8, *const u8) -> i32;
    priv_strcmp_c8: "_pcre2_strcmp_c8_8" => unsafe extern "C" fn(*const u8, *const i8) -> i32;
    priv_strncmp: "_pcre2_strncmp_8" => unsafe extern "C" fn(*const u8, *const u8, Sz) -> i32;
    priv_strncmp_c8: "_pcre2_strncmp_c8_8" => unsafe extern "C" fn(*const u8, *const i8, Sz) -> i32;
    priv_strcpy_c8: "_pcre2_strcpy_c8_8" => unsafe extern "C" fn(*mut u8, *const i8) -> Sz;
    priv_ord2utf: "_pcre2_ord2utf_8" => unsafe extern "C" fn(u32, *mut u8) -> u32;
    priv_valid_utf: "_pcre2_valid_utf_8" => unsafe extern "C" fn(*const u8, Sz, *mut Sz) -> i32;
    priv_is_newline: "_pcre2_is_newline_8" => unsafe extern "C" fn(*const u8, u32, *const u8, *mut u32, i32) -> i32;
    priv_was_newline: "_pcre2_was_newline_8" => unsafe extern "C" fn(*const u8, u32, *const u8, *mut u32, i32) -> i32;
    priv_extuni: "_pcre2_extuni_8" => unsafe extern "C" fn(u32, *const u8, *const u8, *const u8, i32, *mut i32) -> *const u8;
    priv_script_run: "_pcre2_script_run_8" => unsafe extern "C" fn(*const u8, *const u8, i32) -> i32;
    priv_find_bracket: "_pcre2_find_bracket_8" => unsafe extern "C" fn(*const u8, i32, i32) -> *const u8;
    priv_xclass: "_pcre2_xclass_8" => unsafe extern "C" fn(u32, *const u8, *const u8, i32) -> i32;
    priv_eclass: "_pcre2_eclass_8" => unsafe extern "C" fn(u32, *const u8, *const u8, *const u8, i32) -> i32;
    priv_ckd_smul: "_pcre2_ckd_smul_8" => unsafe extern "C" fn(*mut Sz, i32, i32) -> i32;
    priv_study: "_pcre2_study_8" => unsafe extern "C" fn(Code) -> i32;
    priv_auto_possessify: "_pcre2_auto_possessify_8" => unsafe extern "C" fn(*mut u8, *const c_void) -> i32;
    priv_memctl_malloc: "_pcre2_memctl_malloc_8" => unsafe extern "C" fn(Sz, *mut c_void) -> *mut c_void;
    priv_update_classbits: "_pcre2_update_classbits_8" => unsafe extern "C" fn(u32, u32, i32, *mut u8);
    priv_get_hash_from_name: "_pcre2_compile_get_hash_from_name8" => unsafe extern "C" fn(*const u8, u32) -> u16;

    // ---- exported data tables ---------------------------------------------
    d_OP_lengths: "_pcre2_OP_lengths_8" => *const u8;
    d_callout_end_delims: "_pcre2_callout_end_delims_8" => *const u32;
    d_callout_start_delims: "_pcre2_callout_start_delims_8" => *const u32;
    d_default_tables: "_pcre2_default_tables_8" => *const u8;
    d_hspace_list: "_pcre2_hspace_list_8" => *const u32;
    d_vspace_list: "_pcre2_vspace_list_8" => *const u32;
    d_posix_class_maps: "_pcre2_posix_class_maps8" => *const i32;
    d_ucd_boolprop_sets: "_pcre2_ucd_boolprop_sets_8" => *const u32;
    d_ucd_caseless_sets: "_pcre2_ucd_caseless_sets_8" => *const u32;
    d_ucd_digit_sets: "_pcre2_ucd_digit_sets_8" => *const u32;
    d_ucd_nocase_ranges: "_pcre2_ucd_nocase_ranges_8" => *const u32;
    d_ucd_nocase_ranges_size: "_pcre2_ucd_nocase_ranges_size_8" => *const u32;
    d_ucd_records: "_pcre2_ucd_records_8" => *const u8;
    d_ucd_script_sets: "_pcre2_ucd_script_sets_8" => *const u32;
    d_ucd_stage1: "_pcre2_ucd_stage1_8" => *const u16;
    d_ucd_stage2: "_pcre2_ucd_stage2_8" => *const u16;
    d_ucd_turkish_dotted_i_caseset: "_pcre2_ucd_turkish_dotted_i_caseset_8" => *const u32;
    d_ucp_gbtable: "_pcre2_ucp_gbtable_8" => *const u32;
    d_ucp_gentype: "_pcre2_ucp_gentype_8" => *const u32;
    d_unicode_version: "_pcre2_unicode_version_8" => *const *const i8;
    d_utf8_table1: "_pcre2_utf8_table1" => *const i32;
    d_utf8_table1_size: "_pcre2_utf8_table1_size" => *const u32;
    d_utf8_table2: "_pcre2_utf8_table2" => *const i32;
    d_utf8_table3: "_pcre2_utf8_table3" => *const i32;
    d_utf8_table4: "_pcre2_utf8_table4" => *const u8;
    d_utt: "_pcre2_utt_8" => *const u8;
    d_utt_names: "_pcre2_utt_names_8" => *const u8;
    d_utt_size: "_pcre2_utt_size_8" => *const usize;
    d_default_compile_context: "_pcre2_default_compile_context_8" => *const u8;
    d_default_match_context: "_pcre2_default_match_context_8" => *const u8;
    d_default_convert_context: "_pcre2_default_convert_context_8" => *const u8;
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CalloutBlock {
    pub version: u32,
    pub callout_number: u32,
    pub capture_top: u32,
    pub capture_last: u32,
    pub offset_vector: *mut Sz,
    pub mark: *const u8,
    pub subject: *const u8,
    pub subject_length: Sz,
    pub start_match: Sz,
    pub current_position: Sz,
    pub pattern_position: Sz,
    pub next_item_length: Sz,
    pub callout_string_offset: Sz,
    pub callout_string_length: Sz,
    pub callout_string: *const u8,
    pub callout_flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CalloutEnumBlock {
    pub version: u32,
    pub pattern_position: Sz,
    pub next_item_length: Sz,
    pub callout_number: u32,
    pub callout_string_offset: Sz,
    pub callout_string_length: Sz,
    pub callout_string: *const u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SubstCalloutBlock {
    pub version: u32,
    pub input: *const u8,
    pub output: *const u8,
    pub output_offsets: [Sz; 2],
    pub ovector: *mut Sz,
    pub oveccount: u32,
    pub subscount: u32,
}

// ---------------------------------------------------------------------------
// Loading both libraries once per test binary.
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

// The two `Api` structs hold only immutable code/data addresses into two dlopen'd
// shared objects; sharing them across threads is sound (PCRE2 itself is
// thread-safe for compile/match on distinct objects).
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let root = workspace_root();
        let c_so = root.join("c_src/build/libpcre2.so");
        let r_so = root.join("translation/target/release/libpcre2.so");
        assert!(c_so.exists(), "missing {} — build the C library first", c_so.display());
        assert!(r_so.exists(), "missing {} — run `cargo build --release` first", r_so.display());
        Pair {
            c: Api::load("C", &c_so),
            r: Api::load("RUST", &r_so),
        }
    })
}

// ---------------------------------------------------------------------------
// Option constants (mirrors of pcre2.h)
// ---------------------------------------------------------------------------

pub mod o {
    pub const ANCHORED: u32 = 0x8000_0000;
    pub const NO_UTF_CHECK: u32 = 0x4000_0000;
    pub const ENDANCHORED: u32 = 0x2000_0000;

    pub const ALLOW_EMPTY_CLASS: u32 = 0x0000_0001;
    pub const ALT_BSUX: u32 = 0x0000_0002;
    pub const AUTO_CALLOUT: u32 = 0x0000_0004;
    pub const CASELESS: u32 = 0x0000_0008;
    pub const DOLLAR_ENDONLY: u32 = 0x0000_0010;
    pub const DOTALL: u32 = 0x0000_0020;
    pub const DUPNAMES: u32 = 0x0000_0040;
    pub const EXTENDED: u32 = 0x0000_0080;
    pub const FIRSTLINE: u32 = 0x0000_0100;
    pub const MATCH_UNSET_BACKREF: u32 = 0x0000_0200;
    pub const MULTILINE: u32 = 0x0000_0400;
    pub const NEVER_UCP: u32 = 0x0000_0800;
    pub const NEVER_UTF: u32 = 0x0000_1000;
    pub const NO_AUTO_CAPTURE: u32 = 0x0000_2000;
    pub const NO_AUTO_POSSESS: u32 = 0x0000_4000;
    pub const NO_DOTSTAR_ANCHOR: u32 = 0x0000_8000;
    pub const NO_START_OPTIMIZE: u32 = 0x0001_0000;
    pub const UCP: u32 = 0x0002_0000;
    pub const UNGREEDY: u32 = 0x0004_0000;
    pub const UTF: u32 = 0x0008_0000;
    pub const NEVER_BACKSLASH_C: u32 = 0x0010_0000;
    pub const ALT_CIRCUMFLEX: u32 = 0x0020_0000;
    pub const ALT_VERBNAMES: u32 = 0x0040_0000;
    pub const USE_OFFSET_LIMIT: u32 = 0x0080_0000;
    pub const EXTENDED_MORE: u32 = 0x0100_0000;
    pub const LITERAL: u32 = 0x0200_0000;
    pub const MATCH_INVALID_UTF: u32 = 0x0400_0000;
    pub const ALT_EXTENDED_CLASS: u32 = 0x0800_0000;

    pub const X_ALLOW_SURROGATE_ESCAPES: u32 = 0x0000_0001;
    pub const X_BAD_ESCAPE_IS_LITERAL: u32 = 0x0000_0002;
    pub const X_MATCH_WORD: u32 = 0x0000_0004;
    pub const X_MATCH_LINE: u32 = 0x0000_0008;
    pub const X_ESCAPED_CR_IS_LF: u32 = 0x0000_0010;
    pub const X_ALT_BSUX: u32 = 0x0000_0020;
    pub const X_ALLOW_LOOKAROUND_BSK: u32 = 0x0000_0040;
    pub const X_CASELESS_RESTRICT: u32 = 0x0000_0080;
    pub const X_ASCII_BSD: u32 = 0x0000_0100;
    pub const X_ASCII_BSS: u32 = 0x0000_0200;
    pub const X_ASCII_BSW: u32 = 0x0000_0400;
    pub const X_ASCII_POSIX: u32 = 0x0000_0800;
    pub const X_ASCII_DIGIT: u32 = 0x0000_1000;
    pub const X_PYTHON_OCTAL: u32 = 0x0000_2000;
    pub const X_NO_BS0: u32 = 0x0000_4000;
    pub const X_NEVER_CALLOUT: u32 = 0x0000_8000;
    pub const X_TURKISH_CASING: u32 = 0x0001_0000;

    pub const NOTBOL: u32 = 0x0000_0001;
    pub const NOTEOL: u32 = 0x0000_0002;
    pub const NOTEMPTY: u32 = 0x0000_0004;
    pub const NOTEMPTY_ATSTART: u32 = 0x0000_0008;
    pub const PARTIAL_SOFT: u32 = 0x0000_0010;
    pub const PARTIAL_HARD: u32 = 0x0000_0020;
    pub const DFA_RESTART: u32 = 0x0000_0040;
    pub const DFA_SHORTEST: u32 = 0x0000_0080;
    pub const SUBSTITUTE_GLOBAL: u32 = 0x0000_0100;
    pub const SUBSTITUTE_EXTENDED: u32 = 0x0000_0200;
    pub const SUBSTITUTE_UNSET_EMPTY: u32 = 0x0000_0400;
    pub const SUBSTITUTE_UNKNOWN_UNSET: u32 = 0x0000_0800;
    pub const SUBSTITUTE_OVERFLOW_LENGTH: u32 = 0x0000_1000;
    pub const NO_JIT: u32 = 0x0000_2000;
    pub const COPY_MATCHED_SUBJECT: u32 = 0x0000_4000;
    pub const SUBSTITUTE_LITERAL: u32 = 0x0000_8000;
    pub const SUBSTITUTE_MATCHED: u32 = 0x0001_0000;
    pub const SUBSTITUTE_REPLACEMENT_ONLY: u32 = 0x0002_0000;
    pub const DISABLE_RECURSELOOP_CHECK: u32 = 0x0004_0000;

    pub const CONVERT_UTF: u32 = 0x0000_0001;
    pub const CONVERT_NO_UTF_CHECK: u32 = 0x0000_0002;
    pub const CONVERT_POSIX_BASIC: u32 = 0x0000_0004;
    pub const CONVERT_POSIX_EXTENDED: u32 = 0x0000_0008;
    pub const CONVERT_GLOB: u32 = 0x0000_0010;
    pub const CONVERT_GLOB_NO_WILD_SEPARATOR: u32 = 0x0000_0030;
    pub const CONVERT_GLOB_NO_STARSTAR: u32 = 0x0000_0050;

    pub const NEWLINE_CR: u32 = 1;
    pub const NEWLINE_LF: u32 = 2;
    pub const NEWLINE_CRLF: u32 = 3;
    pub const NEWLINE_ANY: u32 = 4;
    pub const NEWLINE_ANYCRLF: u32 = 5;
    pub const NEWLINE_NUL: u32 = 6;

    pub const BSR_UNICODE: u32 = 1;
    pub const BSR_ANYCRLF: u32 = 2;

    pub const OPTIMIZATION_NONE: u32 = 0;
    pub const OPTIMIZATION_FULL: u32 = 1;
    pub const AUTO_POSSESS: u32 = 64;
    pub const AUTO_POSSESS_OFF: u32 = 65;
    pub const DOTSTAR_ANCHOR: u32 = 66;
    pub const DOTSTAR_ANCHOR_OFF: u32 = 67;
    pub const START_OPTIMIZE: u32 = 68;
    pub const START_OPTIMIZE_OFF: u32 = 69;

    pub const JIT_COMPLETE: u32 = 0x0000_0001;
    pub const JIT_PARTIAL_SOFT: u32 = 0x0000_0002;
    pub const JIT_PARTIAL_HARD: u32 = 0x0000_0004;
    pub const JIT_INVALID_UTF: u32 = 0x0000_0100;
    pub const JIT_TEST_ALLOC: u32 = 0x0000_0200;
}

/// pcre2_pattern_info request codes.
pub mod info {
    pub const ALLOPTIONS: u32 = 0;
    pub const ARGOPTIONS: u32 = 1;
    pub const BACKREFMAX: u32 = 2;
    pub const BSR: u32 = 3;
    pub const CAPTURECOUNT: u32 = 4;
    pub const FIRSTCODEUNIT: u32 = 5;
    pub const FIRSTCODETYPE: u32 = 6;
    pub const FIRSTBITMAP: u32 = 7;
    pub const HASCRORLF: u32 = 8;
    pub const JCHANGED: u32 = 9;
    pub const JITSIZE: u32 = 10;
    pub const LASTCODEUNIT: u32 = 11;
    pub const LASTCODETYPE: u32 = 12;
    pub const MATCHEMPTY: u32 = 13;
    pub const MATCHLIMIT: u32 = 14;
    pub const MAXLOOKBEHIND: u32 = 15;
    pub const MINLENGTH: u32 = 16;
    pub const NAMECOUNT: u32 = 17;
    pub const NAMEENTRYSIZE: u32 = 18;
    pub const NAMETABLE: u32 = 19;
    pub const NEWLINE: u32 = 20;
    pub const DEPTHLIMIT: u32 = 21;
    pub const SIZE: u32 = 22;
    pub const HASBACKSLASHC: u32 = 23;
    pub const FRAMESIZE: u32 = 24;
    pub const HEAPLIMIT: u32 = 25;
    pub const EXTRAOPTIONS: u32 = 26;
}

/// pcre2_config request codes.
pub mod cfg {
    pub const BSR: u32 = 0;
    pub const JIT: u32 = 1;
    pub const JITTARGET: u32 = 2;
    pub const LINKSIZE: u32 = 3;
    pub const MATCHLIMIT: u32 = 4;
    pub const NEWLINE: u32 = 5;
    pub const PARENSLIMIT: u32 = 6;
    pub const DEPTHLIMIT: u32 = 7;
    pub const STACKRECURSE: u32 = 8;
    pub const UNICODE: u32 = 9;
    pub const UNICODE_VERSION: u32 = 10;
    pub const VERSION: u32 = 11;
    pub const HEAPLIMIT: u32 = 12;
    pub const NEVER_BACKSLASH_C: u32 = 13;
    pub const COMPILED_WIDTHS: u32 = 14;
    pub const TABLES_LENGTH: u32 = 15;
    pub const EFFECTIVE_LINKSIZE: u32 = 16;
}

pub mod err {
    pub const NOMATCH: i32 = -1;
    pub const PARTIAL: i32 = -2;
    pub const BADDATA: i32 = -29;
    pub const MIXEDTABLES: i32 = -30;
    pub const BADMAGIC: i32 = -31;
    pub const BADMODE: i32 = -32;
    pub const BADOFFSET: i32 = -33;
    pub const BADOPTION: i32 = -34;
    pub const BADREPLACEMENT: i32 = -35;
    pub const BADUTFOFFSET: i32 = -36;
    pub const DFA_BADRESTART: i32 = -38;
    pub const DFA_UFUNC: i32 = -41;
    pub const DFA_WSSIZE: i32 = -43;
    pub const INTERNAL: i32 = -44;
    pub const JIT_BADOPTION: i32 = -45;
    pub const MATCHLIMIT: i32 = -47;
    pub const NOMEMORY: i32 = -48;
    pub const NOSUBSTRING: i32 = -49;
    pub const NOUNIQUESUBSTRING: i32 = -50;
    pub const NULL: i32 = -51;
    pub const DEPTHLIMIT: i32 = -53;
    pub const UNAVAILABLE: i32 = -54;
    pub const UNSET: i32 = -55;
    pub const BADOFFSETLIMIT: i32 = -56;
    pub const BADREPESCAPE: i32 = -57;
    pub const REPMISSINGBRACE: i32 = -58;
    pub const BADSUBSTITUTION: i32 = -59;
    pub const BADSUBSPATTERN: i32 = -60;
    pub const TOOMANYREPLACE: i32 = -61;
    pub const BADSERIALIZEDDATA: i32 = -62;
    pub const HEAPLIMIT: i32 = -63;
    pub const CONVERT_SYNTAX: i32 = -64;
    pub const DFA_UINVALID_UTF: i32 = -66;
    pub const INVALIDOFFSET: i32 = -67;
    pub const JIT_UNSUPPORTED: i32 = -68;
    pub const REPLACECASE: i32 = -69;
    pub const TOOLARGEREPLACE: i32 = -70;
    pub const DIFFSUBSPATTERN: i32 = -71;
    pub const DIFFSUBSSUBJECT: i32 = -72;
    pub const DIFFSUBSOFFSET: i32 = -73;
    pub const DIFFSUBSOPTIONS: i32 = -74;
    pub const BAD_BACKSLASH_K: i32 = -75;
    pub const PARTIALSUBS: i32 = -76;
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every "randomized" test is reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Convenience assertion helpers
// ---------------------------------------------------------------------------

/// A compiled-pattern pair (same pattern compiled by both libraries).
pub struct CodePair {
    pub c: Code,
    pub r: Code,
}

impl CodePair {
    pub fn is_null(&self) -> bool {
        self.c.is_null()
    }
}

/// Compile the same pattern in both libraries and assert error code / offset agree.
/// Returns `Ok(CodePair)` when both compiled, `Err((code, offset))` when both failed.
pub fn compile_both(
    p: &Pair,
    pattern: &[u8],
    plen: Sz,
    options: u32,
    cctx_c: Ctx,
    cctx_r: Ctx,
    ctx_label: &str,
) -> Result<CodePair, (i32, Sz)> {
    let mut ec_c: i32 = 12345;
    let mut eo_c: Sz = 0xdead;
    let mut ec_r: i32 = 12345;
    let mut eo_r: Sz = 0xdead;
    let pp = if pattern.is_empty() { std::ptr::null() } else { pattern.as_ptr() };
    let code_c = unsafe { (p.c.compile)(pp, plen, options, &mut ec_c, &mut eo_c, cctx_c) };
    let code_r = unsafe { (p.r.compile)(pp, plen, options, &mut ec_r, &mut eo_r, cctx_r) };
    assert_eq!(
        code_c.is_null(),
        code_r.is_null(),
        "compile null-ness differs for {:?} opts={:#x} [{}]: C ec={} eo={}, RUST ec={} eo={}",
        String::from_utf8_lossy(pattern),
        options,
        ctx_label,
        ec_c,
        eo_c,
        ec_r,
        eo_r
    );
    assert_eq!(
        (ec_c, eo_c),
        (ec_r, eo_r),
        "compile (errorcode, erroroffset) differ for {:?} opts={:#x} [{}]",
        String::from_utf8_lossy(pattern),
        options,
        ctx_label
    );
    if code_c.is_null() {
        Err((ec_c, eo_c))
    } else {
        Ok(CodePair { c: code_c, r: code_r })
    }
}

pub fn free_code_pair(p: &Pair, cp: CodePair) {
    unsafe {
        (p.c.code_free)(cp.c);
        (p.r.code_free)(cp.r);
    }
}

/// Read a `uint32_t` pattern-info item from both and compare.
pub fn cmp_info_u32(p: &Pair, cp: &CodePair, what: u32, label: &str) {
    let mut vc: u32 = 0xAAAA_AAAA;
    let mut vr: u32 = 0x5555_5555;
    let rc = unsafe { (p.c.pattern_info)(cp.c, what, &mut vc as *mut u32 as *mut c_void) };
    let rr = unsafe { (p.r.pattern_info)(cp.r, what, &mut vr as *mut u32 as *mut c_void) };
    assert_eq!(rc, rr, "pattern_info({}) rc differs [{}]", what, label);
    if rc == 0 {
        assert_eq!(vc, vr, "pattern_info({}) value differs [{}]", what, label);
    }
}

pub fn cmp_info_usize(p: &Pair, cp: &CodePair, what: u32, label: &str) {
    let mut vc: Sz = 0xAAAA;
    let mut vr: Sz = 0x5555;
    let rc = unsafe { (p.c.pattern_info)(cp.c, what, &mut vc as *mut Sz as *mut c_void) };
    let rr = unsafe { (p.r.pattern_info)(cp.r, what, &mut vr as *mut Sz as *mut c_void) };
    assert_eq!(rc, rr, "pattern_info({}) rc differs [{}]", what, label);
    if rc == 0 {
        assert_eq!(vc, vr, "pattern_info({}) value differs [{}]", what, label);
    }
}

/// Compare every scalar/bitmap/nametable item reported by `pcre2_pattern_info`.
pub fn cmp_all_pattern_info(p: &Pair, cp: &CodePair, label: &str) {
    for w in [
        info::ALLOPTIONS,
        info::ARGOPTIONS,
        info::BACKREFMAX,
        info::BSR,
        info::CAPTURECOUNT,
        info::FIRSTCODEUNIT,
        info::FIRSTCODETYPE,
        info::HASCRORLF,
        info::JCHANGED,
        info::LASTCODEUNIT,
        info::LASTCODETYPE,
        info::MATCHEMPTY,
        info::MAXLOOKBEHIND,
        info::NAMECOUNT,
        info::NAMEENTRYSIZE,
        info::NEWLINE,
        info::HASBACKSLASHC,
        info::EXTRAOPTIONS,
    ] {
        cmp_info_u32(p, cp, w, label);
    }
    for w in [info::SIZE, info::FRAMESIZE, info::MINLENGTH, info::JITSIZE] {
        cmp_info_usize(p, cp, w, label);
    }
    // MATCHLIMIT / DEPTHLIMIT / HEAPLIMIT return PCRE2_ERROR_UNSET unless set.
    for w in [info::MATCHLIMIT, info::DEPTHLIMIT, info::HEAPLIMIT] {
        cmp_info_u32(p, cp, w, label);
    }
    // FIRSTBITMAP: pointer to a 32-byte table (or NULL).
    unsafe {
        let mut bc: *const u8 = std::ptr::null();
        let mut br: *const u8 = std::ptr::null();
        let rc = (p.c.pattern_info)(cp.c, info::FIRSTBITMAP, &mut bc as *mut _ as *mut c_void);
        let rr = (p.r.pattern_info)(cp.r, info::FIRSTBITMAP, &mut br as *mut _ as *mut c_void);
        assert_eq!(rc, rr, "FIRSTBITMAP rc differs [{}]", label);
        assert_eq!(bc.is_null(), br.is_null(), "FIRSTBITMAP null-ness differs [{}]", label);
        if !bc.is_null() {
            let sc = std::slice::from_raw_parts(bc, 32);
            let sr = std::slice::from_raw_parts(br, 32);
            assert_eq!(sc, sr, "FIRSTBITMAP contents differ [{}]", label);
        }
    }
    // NAMETABLE: namecount * nameentrysize code units.
    unsafe {
        let mut nc: u32 = 0;
        let mut nes: u32 = 0;
        (p.c.pattern_info)(cp.c, info::NAMECOUNT, &mut nc as *mut _ as *mut c_void);
        (p.c.pattern_info)(cp.c, info::NAMEENTRYSIZE, &mut nes as *mut _ as *mut c_void);
        let mut tc: *const u8 = std::ptr::null();
        let mut tr: *const u8 = std::ptr::null();
        let rc = (p.c.pattern_info)(cp.c, info::NAMETABLE, &mut tc as *mut _ as *mut c_void);
        let rr = (p.r.pattern_info)(cp.r, info::NAMETABLE, &mut tr as *mut _ as *mut c_void);
        assert_eq!(rc, rr, "NAMETABLE rc differs [{}]", label);
        if nc > 0 && !tc.is_null() && !tr.is_null() {
            let n = (nc * nes) as usize;
            let sc = std::slice::from_raw_parts(tc, n);
            let sr = std::slice::from_raw_parts(tr, n);
            assert_eq!(sc, sr, "NAMETABLE contents differ [{}]", label);
        }
    }
}

/// Byte offset of `magic_number` inside a compiled code block, and the value of
/// `code_start` for a pattern with no name table and no char lists — i.e.
/// `sizeof(pcre2_real_code)`.
fn code_header_size(p: &Pair) -> usize {
    use std::sync::OnceLock;
    static H: OnceLock<usize> = OnceLock::new();
    *H.get_or_init(|| unsafe {
        let mut ec = 0i32;
        let mut eo = 0usize;
        let code = (p.c.compile)(b"a".as_ptr(), 1, 0, &mut ec, &mut eo, std::ptr::null_mut());
        assert!(!code.is_null());
        let base = code as *const u8;
        let mut cs = usize::MAX;
        for off in (0..512usize).step_by(4) {
            if *(base.add(off) as *const u32) == 0x5043_5245 {
                cs = *(base.add(off - std::mem::size_of::<usize>()) as *const usize);
                break;
            }
        }
        (p.c.code_free)(code);
        assert_ne!(cs, usize::MAX, "could not locate MAGIC_NUMBER / code_start");
        cs
    })
}

/// Zero the bytes of a serialized code block that the C library never writes.
///
/// `pcre2_compile` lays the block out as
///   `sizeof(pcre2_real_code)` | name table | \[align to 4\] char lists | byte code
/// and `re_blocksize` is only rounded up to a 4-byte boundary *when char lists are
/// present* (`CLIST_ALIGN_TO` in pcre2_compile.c:10865). Those alignment bytes are
/// never assigned, so they hold whatever `malloc` returned. Mask them out.
fn mask_undefined_block_padding(
    p: &Pair,
    code: Code,
    api: &Api,
    blob: &mut [u8],
    block_off: usize,
) {
    unsafe {
        let header = code_header_size(p);
        let mut nc: u32 = 0;
        let mut nes: u32 = 0;
        (api.pattern_info)(code, info::NAMECOUNT, &mut nc as *mut _ as *mut c_void);
        (api.pattern_info)(code, info::NAMEENTRYSIZE, &mut nes as *mut _ as *mut c_void);
        let names_size = nc as usize * nes as usize;
        let base = code as *const u8;
        let mut code_start = usize::MAX;
        for off in (0..512usize).step_by(4) {
            if *(base.add(off) as *const u32) == 0x5043_5245 {
                code_start = *(base.add(off - std::mem::size_of::<usize>()) as *const usize);
                break;
            }
        }
        assert_ne!(code_start, usize::MAX);
        if code_start == header + names_size {
            return; // no char lists => no alignment padding
        }
        let pad_start = header + names_size;
        let pad_end = header + ((names_size + 3) & !3usize);
        for i in pad_start..pad_end.min(code_start) {
            let idx = block_off + i;
            if idx < blob.len() {
                blob[idx] = 0;
            }
        }
    }
}

/// Is `subj[..len]` valid UTF-8 according to the C library?
pub fn is_valid_utf(p: &Pair, subj: &[u8], len: usize) -> bool {
    if len == 0 {
        return true;
    }
    let mut off: Sz = 0;
    unsafe { (p.c.priv_valid_utf)(subj.as_ptr(), len.min(subj.len()), &mut off) == 0 }
}

/// Inputs that make the **C library itself** crash, so no differential
/// comparison is possible.
///
/// Both reproducers below segfault `c_src/build/libpcre2.so` (verified by calling
/// only the C library, with the Rust one untouched), i.e. they are upstream PCRE2
/// defects rather than translation differences. The common ingredient is a
/// pattern that walks **backwards** over subject code units that PCRE2 never
/// validated: `pcre2_match` / `pcre2_dfa_match` only validate the subject from
/// `start_offset - re->max_lookbehind` onwards (`pcre2_match.c:7335`), and with
/// `PCRE2_MATCH_INVALID_UTF` they deliberately continue past bad code units.
///
/// ```text
/// (1) pattern  (?<*\p{Xwd}{0,3})
///     options  PCRE2_UTF|PCRE2_MATCH_INVALID_UTF
///     subject  61 ff 61        start offset 0        -> pcre2_match SIGSEGV
///
/// (2) pattern  (*positive_lookbehind:0|((?U:\Z)))
///     options  PCRE2_UTF|PCRE2_UCP|PCRE2_CASELESS|PCRE2_MULTILINE
///     subject  9f 62 c3 43 31 78 39 00 82 2d 82 43 00 ff 61 20 31 79
///     start offset 15, PCRE2_PARTIAL_HARD        -> pcre2_dfa_match SIGSEGV
/// ```
///
/// The exclusion rule is mechanical:
///
/// > in UTF mode, if the subject is not valid UTF-8, skip when either
/// > `start_offset > 0` (the prefix `[0, start_offset - max_lookbehind)` is never
/// > validated, so anything that steps backwards reads unchecked code units) or
/// > `PCRE2_MATCH_INVALID_UTF` is set (the C then deliberately continues past bad
/// > code units and a later backward step reads them).
///
/// A third reproducer of the same class, where `max_lookbehind` is 0 because the
/// lookbehind body is empty:
///
/// ```text
/// (3) pattern  (?<!(?= ))
///     options  PCRE2_UTF|PCRE2_UCP|PCRE2_CASELESS|PCRE2_MULTILINE
///     subject  0d 98 ff        start offset 3, NOTBOL|NOTEOL|NOTEMPTY
///                                            -> pcre2_dfa_match SIGSEGV
/// ```
///
/// This still leaves the important case covered: `PCRE2_UTF` with an invalid
/// subject at `start_offset == 0` and without `PCRE2_MATCH_INVALID_UTF`, where the
/// whole subject *is* validated and the library must report the right
/// `PCRE2_ERROR_UTF8_ERRn` (see `match_utf_subject_errors`).
pub fn c_crashes_on_invalid_utf(
    p: &Pair,
    cp: &CodePair,
    subj: &[u8],
    len: usize,
    start: Sz,
) -> bool {
    unsafe {
        let mut allopts: u32 = 0;
        (p.c.pattern_info)(cp.c, info::ALLOPTIONS, &mut allopts as *mut _ as *mut c_void);
        if allopts & o::UTF == 0 {
            return false;
        }
        if start == 0 && allopts & o::MATCH_INVALID_UTF == 0 {
            return false;
        }
        !is_valid_utf(p, subj, len)
    }
}

/// Byte-compare the whole serialized form of two compiled patterns.
/// This is the strongest possible equality check on the compiled bytecode:
/// `pcre2_serialize_encode` dumps the entire `pcre2_real_code` block plus the
/// character tables.
pub fn cmp_compiled_bytes(p: &Pair, cp: &CodePair, label: &str) {
    unsafe {
        let mut bc: *mut u8 = std::ptr::null_mut();
        let mut br: *mut u8 = std::ptr::null_mut();
        let mut lc: Sz = 0;
        let mut lr: Sz = 0;
        let codes_c = [cp.c];
        let codes_r = [cp.r];
        let rc = (p.c.serialize_encode)(codes_c.as_ptr(), 1, &mut bc, &mut lc, std::ptr::null_mut());
        let rr = (p.r.serialize_encode)(codes_r.as_ptr(), 1, &mut br, &mut lr, std::ptr::null_mut());
        assert_eq!(rc, rr, "serialize_encode rc differs [{}]", label);
        if rc < 0 {
            return;
        }
        assert_eq!(lc, lr, "serialized length differs [{}]", label);
        let mut sc = std::slice::from_raw_parts(bc, lc).to_vec();
        let mut sr = std::slice::from_raw_parts(br, lr).to_vec();
        (p.c.serialize_free)(bc);
        (p.r.serialize_free)(br);
        // The single code block sits at the end of the blob.
        let mut bs: Sz = 0;
        (p.c.pattern_info)(cp.c, info::SIZE, &mut bs as *mut _ as *mut c_void);
        let block_off = lc - bs;
        mask_undefined_block_padding(p, cp.c, &p.c, &mut sc, block_off);
        mask_undefined_block_padding(p, cp.r, &p.r, &mut sr, block_off);
        if sc != sr {
            let first = sc.iter().zip(&sr).position(|(a, b)| a != b).unwrap();
            panic!(
                "serialized compiled pattern differs at byte {} (len {}, block at {}) [{}]\n C: {:02x?}\n R: {:02x?}",
                first,
                lc,
                block_off,
                label,
                &sc[first.saturating_sub(8)..(first + 24).min(lc)],
                &sr[first.saturating_sub(8)..(first + 24).min(lr)]
            );
        }
    }
}
