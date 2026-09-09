//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries (the C build and the Rust cdylib) through
//! `libloading` and exposes every exported symbol as a cached function pointer.
//! No Rust function is ever called directly — everything goes through the
//! `#[no_mangle]` export wrappers, exactly like an external C caller.
#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};

pub mod corpus;

pub const C_SO: &str = "../c_src/build/libpcre2.so";
pub const R_SO: &str = "target/release/libpcre2.so";

pub type Code = c_void;
pub type MatchData = c_void;
pub type Ctx = c_void;
pub type Sz = usize;

pub type MallocFn = extern "C" fn(usize, *mut c_void) -> *mut c_void;
pub type FreeFn = extern "C" fn(*mut c_void, *mut c_void);
pub type CalloutFn = extern "C" fn(*mut c_void, *mut c_void) -> c_int;
pub type EnumFn = extern "C" fn(*mut c_void, *mut c_void) -> c_int;
pub type SubCalloutFn = extern "C" fn(*mut c_void, *mut c_void) -> c_int;
pub type CaseCalloutFn = extern "C" fn(*const u8, Sz, *mut u8, Sz, c_int, *mut c_void) -> Sz;
pub type GuardFn = extern "C" fn(u32, *mut c_void) -> c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemCtl {
    pub malloc: Option<MallocFn>,
    pub free: Option<FreeFn>,
    pub memory_data: *mut c_void,
}

macro_rules! api {
    ( $( $field:ident : $sym:literal : $ty:ty ; )* ) => {
        pub struct Api {
            pub name: &'static str,
            pub lib: &'static Library,
            $( pub $field : Symbol<'static, $ty>, )*
        }
        impl Api {
            pub fn load(name: &'static str, path: &str) -> Api {
                let lib: &'static Library =
                    Box::leak(Box::new(unsafe { Library::new(path) }
                        .unwrap_or_else(|e| panic!("dlopen {path}: {e}"))));
                unsafe {
                    Api {
                        name,
                        lib,
                        $( $field : lib.get::<$ty>(concat!($sym, "\0").as_bytes())
                             .unwrap_or_else(|e| panic!("{}: dlsym {}: {}", path, $sym, e)), )*
                    }
                }
            }
        }
    }
}

api! {
    // ---- general info / misc ----
    config: "pcre2_config_8": unsafe extern "C" fn(u32, *mut c_void) -> c_int;
    get_error_message: "pcre2_get_error_message_8": unsafe extern "C" fn(c_int, *mut u8, Sz) -> c_int;
    maketables: "pcre2_maketables_8": unsafe extern "C" fn(*mut Ctx) -> *const u8;
    maketables_free: "pcre2_maketables_free_8": unsafe extern "C" fn(*mut Ctx, *const u8);

    // ---- contexts ----
    general_context_create: "pcre2_general_context_create_8": unsafe extern "C" fn(Option<MallocFn>, Option<FreeFn>, *mut c_void) -> *mut Ctx;
    general_context_copy: "pcre2_general_context_copy_8": unsafe extern "C" fn(*mut Ctx) -> *mut Ctx;
    general_context_free: "pcre2_general_context_free_8": unsafe extern "C" fn(*mut Ctx);
    compile_context_create: "pcre2_compile_context_create_8": unsafe extern "C" fn(*mut Ctx) -> *mut Ctx;
    compile_context_copy: "pcre2_compile_context_copy_8": unsafe extern "C" fn(*mut Ctx) -> *mut Ctx;
    compile_context_free: "pcre2_compile_context_free_8": unsafe extern "C" fn(*mut Ctx);
    match_context_create: "pcre2_match_context_create_8": unsafe extern "C" fn(*mut Ctx) -> *mut Ctx;
    match_context_copy: "pcre2_match_context_copy_8": unsafe extern "C" fn(*mut Ctx) -> *mut Ctx;
    match_context_free: "pcre2_match_context_free_8": unsafe extern "C" fn(*mut Ctx);
    convert_context_create: "pcre2_convert_context_create_8": unsafe extern "C" fn(*mut Ctx) -> *mut Ctx;
    convert_context_copy: "pcre2_convert_context_copy_8": unsafe extern "C" fn(*mut Ctx) -> *mut Ctx;
    convert_context_free: "pcre2_convert_context_free_8": unsafe extern "C" fn(*mut Ctx);

    // ---- compile-context setters ----
    set_bsr: "pcre2_set_bsr_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_character_tables: "pcre2_set_character_tables_8": unsafe extern "C" fn(*mut Ctx, *const u8) -> c_int;
    set_compile_extra_options: "pcre2_set_compile_extra_options_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_max_pattern_length: "pcre2_set_max_pattern_length_8": unsafe extern "C" fn(*mut Ctx, Sz) -> c_int;
    set_max_pattern_compiled_length: "pcre2_set_max_pattern_compiled_length_8": unsafe extern "C" fn(*mut Ctx, Sz) -> c_int;
    set_max_varlookbehind: "pcre2_set_max_varlookbehind_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_newline: "pcre2_set_newline_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_parens_nest_limit: "pcre2_set_parens_nest_limit_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_compile_recursion_guard: "pcre2_set_compile_recursion_guard_8": unsafe extern "C" fn(*mut Ctx, Option<GuardFn>, *mut c_void) -> c_int;
    set_optimize: "pcre2_set_optimize_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;

    // ---- match-context setters ----
    set_callout: "pcre2_set_callout_8": unsafe extern "C" fn(*mut Ctx, Option<CalloutFn>, *mut c_void) -> c_int;
    set_substitute_callout: "pcre2_set_substitute_callout_8": unsafe extern "C" fn(*mut Ctx, Option<SubCalloutFn>, *mut c_void) -> c_int;
    set_substitute_case_callout: "pcre2_set_substitute_case_callout_8": unsafe extern "C" fn(*mut Ctx, Option<CaseCalloutFn>, *mut c_void) -> c_int;
    set_depth_limit: "pcre2_set_depth_limit_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_heap_limit: "pcre2_set_heap_limit_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_match_limit: "pcre2_set_match_limit_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_offset_limit: "pcre2_set_offset_limit_8": unsafe extern "C" fn(*mut Ctx, Sz) -> c_int;
    set_recursion_limit: "pcre2_set_recursion_limit_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_recursion_memory_management: "pcre2_set_recursion_memory_management_8": unsafe extern "C" fn(*mut Ctx, Option<MallocFn>, Option<FreeFn>, *mut c_void) -> c_int;

    // ---- convert-context setters ----
    set_glob_escape: "pcre2_set_glob_escape_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;
    set_glob_separator: "pcre2_set_glob_separator_8": unsafe extern "C" fn(*mut Ctx, u32) -> c_int;

    // ---- compile ----
    compile: "pcre2_compile_8": unsafe extern "C" fn(*const u8, Sz, u32, *mut c_int, *mut Sz, *mut Ctx) -> *mut Code;
    code_free: "pcre2_code_free_8": unsafe extern "C" fn(*mut Code);
    code_copy: "pcre2_code_copy_8": unsafe extern "C" fn(*const Code) -> *mut Code;
    code_copy_with_tables: "pcre2_code_copy_with_tables_8": unsafe extern "C" fn(*const Code) -> *mut Code;

    // ---- pattern info ----
    pattern_info: "pcre2_pattern_info_8": unsafe extern "C" fn(*const Code, u32, *mut c_void) -> c_int;
    callout_enumerate: "pcre2_callout_enumerate_8": unsafe extern "C" fn(*const Code, Option<EnumFn>, *mut c_void) -> c_int;

    // ---- match ----
    match_data_create: "pcre2_match_data_create_8": unsafe extern "C" fn(u32, *mut Ctx) -> *mut MatchData;
    match_data_create_from_pattern: "pcre2_match_data_create_from_pattern_8": unsafe extern "C" fn(*const Code, *mut Ctx) -> *mut MatchData;
    match_data_free: "pcre2_match_data_free_8": unsafe extern "C" fn(*mut MatchData);
    pcre2_match: "pcre2_match_8": unsafe extern "C" fn(*const Code, *const u8, Sz, Sz, u32, *mut MatchData, *mut Ctx) -> c_int;
    dfa_match: "pcre2_dfa_match_8": unsafe extern "C" fn(*const Code, *const u8, Sz, Sz, u32, *mut MatchData, *mut Ctx, *mut c_int, Sz) -> c_int;
    get_mark: "pcre2_get_mark_8": unsafe extern "C" fn(*mut MatchData) -> *const u8;
    get_match_data_size: "pcre2_get_match_data_size_8": unsafe extern "C" fn(*mut MatchData) -> Sz;
    get_match_data_heapframes_size: "pcre2_get_match_data_heapframes_size_8": unsafe extern "C" fn(*mut MatchData) -> Sz;
    get_ovector_count: "pcre2_get_ovector_count_8": unsafe extern "C" fn(*mut MatchData) -> u32;
    get_ovector_pointer: "pcre2_get_ovector_pointer_8": unsafe extern "C" fn(*mut MatchData) -> *mut Sz;
    get_startchar: "pcre2_get_startchar_8": unsafe extern "C" fn(*mut MatchData) -> Sz;
    next_match: "pcre2_next_match_8": unsafe extern "C" fn(*mut MatchData, *mut Sz, *mut u32) -> c_int;

    // ---- substrings ----
    substring_copy_byname: "pcre2_substring_copy_byname_8": unsafe extern "C" fn(*mut MatchData, *const u8, *mut u8, *mut Sz) -> c_int;
    substring_copy_bynumber: "pcre2_substring_copy_bynumber_8": unsafe extern "C" fn(*mut MatchData, u32, *mut u8, *mut Sz) -> c_int;
    substring_free: "pcre2_substring_free_8": unsafe extern "C" fn(*mut u8);
    substring_get_byname: "pcre2_substring_get_byname_8": unsafe extern "C" fn(*mut MatchData, *const u8, *mut *mut u8, *mut Sz) -> c_int;
    substring_get_bynumber: "pcre2_substring_get_bynumber_8": unsafe extern "C" fn(*mut MatchData, u32, *mut *mut u8, *mut Sz) -> c_int;
    substring_length_byname: "pcre2_substring_length_byname_8": unsafe extern "C" fn(*mut MatchData, *const u8, *mut Sz) -> c_int;
    substring_length_bynumber: "pcre2_substring_length_bynumber_8": unsafe extern "C" fn(*mut MatchData, u32, *mut Sz) -> c_int;
    substring_nametable_scan: "pcre2_substring_nametable_scan_8": unsafe extern "C" fn(*const Code, *const u8, *mut *const u8, *mut *const u8) -> c_int;
    substring_number_from_name: "pcre2_substring_number_from_name_8": unsafe extern "C" fn(*const Code, *const u8) -> c_int;
    substring_list_free: "pcre2_substring_list_free_8": unsafe extern "C" fn(*mut *mut u8);
    substring_list_get: "pcre2_substring_list_get_8": unsafe extern "C" fn(*mut MatchData, *mut *mut *mut u8, *mut *mut Sz) -> c_int;

    // ---- serialize ----
    serialize_encode: "pcre2_serialize_encode_8": unsafe extern "C" fn(*const *const Code, i32, *mut *mut u8, *mut Sz, *mut Ctx) -> i32;
    serialize_decode: "pcre2_serialize_decode_8": unsafe extern "C" fn(*mut *mut Code, i32, *const u8, *mut Ctx) -> i32;
    serialize_get_number_of_codes: "pcre2_serialize_get_number_of_codes_8": unsafe extern "C" fn(*const u8) -> i32;
    serialize_free: "pcre2_serialize_free_8": unsafe extern "C" fn(*mut u8);

    // ---- substitute ----
    substitute: "pcre2_substitute_8": unsafe extern "C" fn(*const Code, *const u8, Sz, Sz, u32, *mut MatchData, *mut Ctx, *const u8, Sz, *mut u8, *mut Sz) -> c_int;

    // ---- convert ----
    pattern_convert: "pcre2_pattern_convert_8": unsafe extern "C" fn(*const u8, Sz, u32, *mut *mut u8, *mut Sz, *mut Ctx) -> c_int;
    converted_pattern_free: "pcre2_converted_pattern_free_8": unsafe extern "C" fn(*mut u8);

    // ---- JIT (stubs in this build: no SUPPORT_JIT) ----
    jit_compile: "pcre2_jit_compile_8": unsafe extern "C" fn(*mut Code, u32) -> c_int;
    jit_match: "pcre2_jit_match_8": unsafe extern "C" fn(*const Code, *const u8, Sz, Sz, u32, *mut MatchData, *mut Ctx) -> c_int;
    jit_free_unused_memory: "pcre2_jit_free_unused_memory_8": unsafe extern "C" fn(*mut Ctx);
    jit_stack_create: "pcre2_jit_stack_create_8": unsafe extern "C" fn(usize, usize, *mut Ctx) -> *mut c_void;
    jit_stack_assign: "pcre2_jit_stack_assign_8": unsafe extern "C" fn(*mut Ctx, Option<extern "C" fn(*mut c_void) -> *mut c_void>, *mut c_void);
    jit_stack_free: "pcre2_jit_stack_free_8": unsafe extern "C" fn(*mut c_void);
    priv_jit_free: "_pcre2_jit_free_8": unsafe extern "C" fn(*mut c_void, *mut MemCtl);
    priv_jit_free_rodata: "_pcre2_jit_free_rodata_8": unsafe extern "C" fn(*mut c_void, *mut c_void);
    priv_jit_get_size: "_pcre2_jit_get_size_8": unsafe extern "C" fn(*mut c_void) -> usize;
    priv_jit_get_target: "_pcre2_jit_get_target_8": unsafe extern "C" fn() -> *const c_char;

    // ---- low-level private helpers ----
    priv_strlen: "_pcre2_strlen_8": unsafe extern "C" fn(*const u8) -> Sz;
    priv_strcmp: "_pcre2_strcmp_8": unsafe extern "C" fn(*const u8, *const u8) -> c_int;
    priv_strcmp_c8: "_pcre2_strcmp_c8_8": unsafe extern "C" fn(*const u8, *const c_char) -> c_int;
    priv_strncmp: "_pcre2_strncmp_8": unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int;
    priv_strncmp_c8: "_pcre2_strncmp_c8_8": unsafe extern "C" fn(*const u8, *const c_char, usize) -> c_int;
    priv_strcpy_c8: "_pcre2_strcpy_c8_8": unsafe extern "C" fn(*mut u8, *const c_char) -> Sz;
    priv_ord2utf: "_pcre2_ord2utf_8": unsafe extern "C" fn(u32, *mut u8) -> u32;
    priv_valid_utf: "_pcre2_valid_utf_8": unsafe extern "C" fn(*const u8, Sz, *mut Sz) -> c_int;
    priv_ckd_smul: "_pcre2_ckd_smul_8": unsafe extern "C" fn(*mut Sz, c_int, c_int) -> c_int;
    priv_is_newline: "_pcre2_is_newline_8": unsafe extern "C" fn(*const u8, u32, *const u8, *mut u32, c_int) -> c_int;
    priv_was_newline: "_pcre2_was_newline_8": unsafe extern "C" fn(*const u8, u32, *const u8, *mut u32, c_int) -> c_int;
    priv_extuni: "_pcre2_extuni_8": unsafe extern "C" fn(u32, *const u8, *const u8, *const u8, c_int, *mut c_int) -> *const u8;
    priv_script_run: "_pcre2_script_run_8": unsafe extern "C" fn(*const u8, *const u8, c_int) -> c_int;
    priv_find_bracket: "_pcre2_find_bracket_8": unsafe extern "C" fn(*const u8, c_int, c_int) -> *const u8;
    priv_memctl_malloc: "_pcre2_memctl_malloc_8": unsafe extern "C" fn(usize, *mut MemCtl) -> *mut c_void;
    priv_xclass: "_pcre2_xclass_8": unsafe extern "C" fn(u32, *const u8, *const u8, c_int) -> c_int;
    priv_study: "_pcre2_study_8": unsafe extern "C" fn(*mut Code) -> c_int;
}

impl Api {
    /// Address + declared size of an exported data symbol.
    pub fn data(&self, sym: &str) -> *const u8 {
        let mut name = sym.as_bytes().to_vec();
        name.push(0);
        unsafe {
            let s: Symbol<*const u8> = self
                .lib
                .get(&name)
                .unwrap_or_else(|e| panic!("{}: dlsym {}: {}", self.name, sym, e));
            // `get` on a data symbol yields a pointer to the symbol's storage.
            s.into_raw().into_raw() as *const u8
        }
    }
}

pub fn both() -> (Api, Api) {
    (Api::load("C", C_SO), Api::load("RUST", R_SO))
}

/// One shared pair, loaded once per test binary.
pub fn pair() -> &'static (Api, Api) {
    use std::sync::OnceLock;
    static P: OnceLock<(Api, Api)> = OnceLock::new();
    P.get_or_init(both)
}
unsafe impl Sync for Api {}
unsafe impl Send for Api {}

// ---------------------------------------------------------------- PRNG -----

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

pub const SEED: u64 = 0x2024_1105_ABCD_EF01;

/// Iteration count for property-style loops, scaled by `$SOAK` (default 1) so
/// the same tests can be run as a long soak without editing them.
pub fn iters(base: usize) -> usize {
    let m: usize = std::env::var("SOAK").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    base.saturating_mul(m.max(1))
}

// ------------------------------------------------------- error constants ---

pub const PCRE2_ERROR_NOMATCH: c_int = -1;
pub const PCRE2_ERROR_PARTIAL: c_int = -2;
pub const PCRE2_ERROR_BADDATA: c_int = -29;
pub const PCRE2_ERROR_MIXEDTABLES: c_int = -30;
pub const PCRE2_ERROR_BADMAGIC: c_int = -31;
pub const PCRE2_ERROR_BADMODE: c_int = -32;
pub const PCRE2_ERROR_BADOFFSET: c_int = -33;
pub const PCRE2_ERROR_BADOPTION: c_int = -34;
pub const PCRE2_ERROR_BADREPLACEMENT: c_int = -35;
pub const PCRE2_ERROR_BADUTFOFFSET: c_int = -36;
pub const PCRE2_ERROR_DFA_BADRESTART: c_int = -38;
pub const PCRE2_ERROR_DFA_UITEM: c_int = -41;
pub const PCRE2_ERROR_DFA_UCOND: c_int = -42;
pub const PCRE2_ERROR_DFA_UFUNC: c_int = -44;
pub const PCRE2_ERROR_DFA_WSSIZE: c_int = -46;
pub const PCRE2_ERROR_JIT_BADOPTION: c_int = -45;
pub const PCRE2_ERROR_MATCHLIMIT: c_int = -47;
pub const PCRE2_ERROR_NOMEMORY: c_int = -48;
pub const PCRE2_ERROR_NOSUBSTRING: c_int = -49;
pub const PCRE2_ERROR_NOUNIQUESUBSTRING: c_int = -50;
pub const PCRE2_ERROR_NULL: c_int = -51;
pub const PCRE2_ERROR_RECURSELOOP: c_int = -52;
pub const PCRE2_ERROR_DEPTHLIMIT: c_int = -53;
pub const PCRE2_ERROR_UNAVAILABLE: c_int = -54;
pub const PCRE2_ERROR_UNSET: c_int = -55;
pub const PCRE2_ERROR_BADOFFSETLIMIT: c_int = -56;
pub const PCRE2_ERROR_INVALIDOFFSET: c_int = -57;
pub const PCRE2_ERROR_BADREPESCAPE: c_int = -59;
pub const PCRE2_ERROR_REPMISSINGBRACE: c_int = -60;
pub const PCRE2_ERROR_BADSUBSTITUTION: c_int = -61;
pub const PCRE2_ERROR_BADSERIALIZEDDATA: c_int = -62;
pub const PCRE2_ERROR_HEAPLIMIT: c_int = -63;
pub const PCRE2_ERROR_CONVERT_SYNTAX: c_int = -64;
pub const PCRE2_ERROR_UNICODE_NOT_SUPPORTED: c_int = -70;

pub const PCRE2_ZERO_TERMINATED: Sz = Sz::MAX;
pub const PCRE2_UNSET: Sz = Sz::MAX;

// compile options
pub const PCRE2_ANCHORED: u32 = 0x8000_0000;
pub const PCRE2_NO_UTF_CHECK: u32 = 0x4000_0000;
pub const PCRE2_ENDANCHORED: u32 = 0x2000_0000;
pub const PCRE2_ALLOW_EMPTY_CLASS: u32 = 0x0000_0001;
pub const PCRE2_ALT_BSUX: u32 = 0x0000_0002;
pub const PCRE2_AUTO_CALLOUT: u32 = 0x0000_0004;
pub const PCRE2_CASELESS: u32 = 0x0000_0008;
pub const PCRE2_DOLLAR_ENDONLY: u32 = 0x0000_0010;
pub const PCRE2_DOTALL: u32 = 0x0000_0020;
pub const PCRE2_DUPNAMES: u32 = 0x0000_0040;
pub const PCRE2_EXTENDED: u32 = 0x0000_0080;
pub const PCRE2_FIRSTLINE: u32 = 0x0000_0100;
pub const PCRE2_MATCH_UNSET_BACKREF: u32 = 0x0000_0200;
pub const PCRE2_MULTILINE: u32 = 0x0000_0400;
pub const PCRE2_NEVER_UCP: u32 = 0x0000_0800;
pub const PCRE2_NEVER_UTF: u32 = 0x0000_1000;
pub const PCRE2_NO_AUTO_CAPTURE: u32 = 0x0000_2000;
pub const PCRE2_NO_AUTO_POSSESS: u32 = 0x0000_4000;
pub const PCRE2_NO_DOTSTAR_ANCHOR: u32 = 0x0000_8000;
pub const PCRE2_NO_START_OPTIMIZE: u32 = 0x0001_0000;
pub const PCRE2_UCP: u32 = 0x0002_0000;
pub const PCRE2_UNGREEDY: u32 = 0x0004_0000;
pub const PCRE2_UTF: u32 = 0x0008_0000;
pub const PCRE2_NEVER_BACKSLASH_C: u32 = 0x0010_0000;
pub const PCRE2_ALT_CIRCUMFLEX: u32 = 0x0020_0000;
pub const PCRE2_ALT_VERBNAMES: u32 = 0x0040_0000;
pub const PCRE2_USE_OFFSET_LIMIT: u32 = 0x0080_0000;
pub const PCRE2_EXTENDED_MORE: u32 = 0x0100_0000;
pub const PCRE2_LITERAL: u32 = 0x0200_0000;
pub const PCRE2_MATCH_INVALID_UTF: u32 = 0x0400_0000;
pub const PCRE2_ALT_EXTENDED_CLASS: u32 = 0x0800_0000;

// extra compile options
pub const PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES: u32 = 0x0000_0001;
pub const PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL: u32 = 0x0000_0002;
pub const PCRE2_EXTRA_MATCH_WORD: u32 = 0x0000_0004;
pub const PCRE2_EXTRA_MATCH_LINE: u32 = 0x0000_0008;
pub const PCRE2_EXTRA_ESCAPED_CR_IS_LF: u32 = 0x0000_0010;
pub const PCRE2_EXTRA_ALT_BSUX: u32 = 0x0000_0020;
pub const PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK: u32 = 0x0000_0040;
pub const PCRE2_EXTRA_CASELESS_RESTRICT: u32 = 0x0000_0080;
pub const PCRE2_EXTRA_ASCII_BSD: u32 = 0x0000_0100;
pub const PCRE2_EXTRA_ASCII_BSS: u32 = 0x0000_0200;
pub const PCRE2_EXTRA_ASCII_BSW: u32 = 0x0000_0400;
pub const PCRE2_EXTRA_ASCII_POSIX: u32 = 0x0000_0800;
pub const PCRE2_EXTRA_ASCII_DIGIT: u32 = 0x0000_1000;
pub const PCRE2_EXTRA_PYTHON_OCTAL: u32 = 0x0000_2000;
pub const PCRE2_EXTRA_NO_BS0: u32 = 0x0000_4000;
pub const PCRE2_EXTRA_NEVER_CALLOUT: u32 = 0x0000_8000;
pub const PCRE2_EXTRA_TURKISH_CASING: u32 = 0x0001_0000;

// match options
pub const PCRE2_NOTBOL: u32 = 0x0000_0001;
pub const PCRE2_NOTEOL: u32 = 0x0000_0002;
pub const PCRE2_NOTEMPTY: u32 = 0x0000_0004;
pub const PCRE2_NOTEMPTY_ATSTART: u32 = 0x0000_0008;
pub const PCRE2_PARTIAL_SOFT: u32 = 0x0000_0010;
pub const PCRE2_PARTIAL_HARD: u32 = 0x0000_0020;
pub const PCRE2_DFA_RESTART: u32 = 0x0000_0040;
pub const PCRE2_DFA_SHORTEST: u32 = 0x0000_0080;
pub const PCRE2_SUBSTITUTE_GLOBAL: u32 = 0x0000_0100;
pub const PCRE2_SUBSTITUTE_EXTENDED: u32 = 0x0000_0200;
pub const PCRE2_SUBSTITUTE_UNSET_EMPTY: u32 = 0x0000_0400;
pub const PCRE2_SUBSTITUTE_UNKNOWN_UNSET: u32 = 0x0000_0800;
pub const PCRE2_SUBSTITUTE_OVERFLOW_LENGTH: u32 = 0x0000_1000;
pub const PCRE2_NO_JIT: u32 = 0x0000_2000;
pub const PCRE2_COPY_MATCHED_SUBJECT: u32 = 0x0000_4000;
pub const PCRE2_SUBSTITUTE_LITERAL: u32 = 0x0000_8000;
pub const PCRE2_SUBSTITUTE_MATCHED: u32 = 0x0001_0000;
pub const PCRE2_SUBSTITUTE_REPLACEMENT_ONLY: u32 = 0x0002_0000;

// convert options
pub const PCRE2_CONVERT_UTF: u32 = 0x0000_0001;
pub const PCRE2_CONVERT_NO_UTF_CHECK: u32 = 0x0000_0002;
pub const PCRE2_CONVERT_POSIX_BASIC: u32 = 0x0000_0004;
pub const PCRE2_CONVERT_POSIX_EXTENDED: u32 = 0x0000_0008;
pub const PCRE2_CONVERT_GLOB: u32 = 0x0000_0010;
pub const PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR: u32 = 0x0000_0030;
pub const PCRE2_CONVERT_GLOB_NO_STARSTAR: u32 = 0x0000_0050;

pub const PCRE2_OPTIMIZATION_NONE: u32 = 0;
pub const PCRE2_OPTIMIZATION_FULL: u32 = 1;

// -------------------------------------------------------------- helpers ----

/// Compile the same pattern in both libraries; returns
/// `((c_code, c_err, c_off), (r_code, r_err, r_off))`.
pub struct Compiled {
    pub code: *mut Code,
    pub err: c_int,
    pub off: Sz,
}

pub unsafe fn compile_one(
    a: &Api,
    pat: &[u8],
    patlen: Sz,
    options: u32,
    cc: *mut Ctx,
) -> Compiled {
    let mut err: c_int = 12345;
    let mut off: Sz = 0xDEAD;
    let p = if pat.is_empty() { pat.as_ptr() } else { pat.as_ptr() };
    let code = unsafe { (a.compile)(p, patlen, options, &mut err, &mut off, cc) };
    Compiled { code, err, off }
}

/// `pcre2_serialize_encode` of a single code → owned byte vector.
/// Returns `Err(rc)` if encoding failed.
pub unsafe fn serialize_bytes(a: &Api, code: *const Code) -> Result<Vec<u8>, i32> {
    let codes = [code];
    let mut bytes: *mut u8 = std::ptr::null_mut();
    let mut size: Sz = 0;
    let rc = unsafe {
        (a.serialize_encode)(
            codes.as_ptr(),
            1,
            &mut bytes,
            &mut size,
            std::ptr::null_mut(),
        )
    };
    if rc < 0 {
        return Err(rc);
    }
    let v = unsafe { std::slice::from_raw_parts(bytes, size).to_vec() };
    unsafe { (a.serialize_free)(bytes) };
    Ok(v)
}

/// All 27 `pcre2_pattern_info` results, rendered as a comparable string.
pub unsafe fn info_dump(a: &Api, code: *const Code) -> String {
    let mut out = String::new();
    for what in 0u32..=26 {
        // Use a big buffer; the widest item is a pointer or PCRE2_SIZE.
        let mut buf = [0u8; 64];
        let rc = unsafe { (a.pattern_info)(code, what, buf.as_mut_ptr() as *mut c_void) };
        if rc != 0 {
            out.push_str(&format!("{what}:rc={rc};"));
            continue;
        }
        match what {
            // uint32_t results
            0 | 1 | 2 | 3 | 4 | 5 | 6 | 8 | 9 | 11 | 12 | 13 | 14 | 15 | 17 | 18 | 20 | 21
            | 23 | 25 | 26 => {
                let v = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
                out.push_str(&format!("{what}:{v};"));
            }
            // PCRE2_SIZE results
            16 | 22 | 24 => {
                let v = Sz::from_ne_bytes(buf[..8].try_into().unwrap());
                out.push_str(&format!("{what}:{v};"));
            }
            // JITSIZE (size_t)
            10 => {
                let v = Sz::from_ne_bytes(buf[..8].try_into().unwrap());
                out.push_str(&format!("{what}:{v};"));
            }
            // pointer results: FIRSTBITMAP (7), NAMETABLE (19) -> deref
            7 => {
                let p = Sz::from_ne_bytes(buf[..8].try_into().unwrap()) as *const u8;
                if p.is_null() {
                    out.push_str("7:null;");
                } else {
                    let s = unsafe { std::slice::from_raw_parts(p, 32) };
                    out.push_str(&format!("7:{s:02x?};"));
                }
            }
            19 => {
                let p = Sz::from_ne_bytes(buf[..8].try_into().unwrap()) as *const u8;
                let mut cnt = [0u8; 4];
                let mut esz = [0u8; 4];
                unsafe {
                    (a.pattern_info)(code, 17, cnt.as_mut_ptr() as *mut c_void);
                    (a.pattern_info)(code, 18, esz.as_mut_ptr() as *mut c_void);
                }
                let n = u32::from_ne_bytes(cnt) as usize;
                let e = u32::from_ne_bytes(esz) as usize;
                if p.is_null() || n == 0 {
                    out.push_str("19:null;");
                } else {
                    let s = unsafe { std::slice::from_raw_parts(p, n * e) };
                    out.push_str(&format!("19:{s:02x?};"));
                }
            }
            _ => unreachable!(),
        }
    }
    out
}

/// `PCRE2_INFO_ALLOPTIONS` for a compiled pattern.
pub unsafe fn all_options(a: &Api, code: *const Code) -> u32 {
    let mut v: u32 = 0;
    unsafe { (a.pattern_info)(code, 0, &mut v as *mut u32 as *mut c_void) };
    v
}

/// True when the subject is valid UTF-8 over `[0, len)`.
pub unsafe fn subject_is_valid_utf(a: &Api, s: &[u8]) -> bool {
    let mut off: Sz = 0;
    unsafe { (a.priv_valid_utf)(s.as_ptr(), s.len(), &mut off) == 0 }
}

/// PCRE2 documents `PCRE2_NO_UTF_CHECK` as "the subject MUST already be valid
/// UTF and `start_offset` MUST be at a character boundary"; violating that is
/// undefined behaviour in the C (8-bit `GET_UCD` has no range guard, so an
/// over-long sequence indexes past `_pcre2_ucd_stage1`).
///
/// `PCRE2_MATCH_INVALID_UTF` combined with a subject that is NOT valid UTF-8 is
/// also outside the comparable domain: the C reads past `end_subject` there.
/// Proven by placing the subject flush against an unmapped guard page —
/// `pcre2_match` on `(?<!\P{L}b?)` with the subject `f4 19`, compiled with
/// `PCRE2_MATCH_INVALID_UTF`, SIGSEGVs in the C library (while `a`, `\P{L}`,
/// `(?<!\P{L})`, `(?<!ab?)` and `(?<!.b?)` all return normally). Cause: for a
/// variable-length lookbehind, `case OP_VREVERSE` in `pcre2_match.c` limits the
/// backward move with `mb->start_subject`, not `mb->check_subject`, so `Feptr`
/// can land before the validated fragment and `GETCHARINCTEST` then decodes a
/// truncated sequence, yielding a code point above `0x10FFFF`.
///
/// Such calls cannot be compared (the C result depends on memory past the
/// object), so they are skipped.
pub unsafe fn subject_domain_is_defined(
    a: &Api,
    code: *const Code,
    subj: &[u8],
    start: Sz,
    mopts: u32,
) -> bool {
    let all = unsafe { all_options(a, code) };
    if all & PCRE2_UTF == 0 {
        return true;
    }
    let valid = unsafe { subject_is_valid_utf(a, subj) };
    if all & PCRE2_MATCH_INVALID_UTF != 0 {
        return valid;
    }
    if mopts & PCRE2_NO_UTF_CHECK == 0 {
        return true;
    }
    if !valid {
        return false;
    }
    // start_offset must be <= len and on a character boundary
    if start > subj.len() {
        return false;
    }
    if start < subj.len() && (subj[start] & 0xc0) == 0x80 {
        return false;
    }
    true
}

/// Full ovector + accessory state after a match call.
///
/// PCRE2 only fills a defined prefix of the ovector:
/// * `rc > 0`  → the first `rc` pairs are set (the rest keep old contents);
/// * `rc == 0` → the ovector was too small, so all `n` pairs are set;
/// * `PCRE2_ERROR_PARTIAL` → pair 0 delimits the partial match.
///
/// Everything past that is whatever happened to be in the (malloc'ed) block, so
/// comparing it would just compare uninitialised memory. `mark` is defined after
/// a match, a partial match, or `PCRE2_ERROR_NOMATCH`.
pub unsafe fn md_dump(a: &Api, md: *mut MatchData, rc: c_int) -> String {
    let n = unsafe { (a.get_ovector_count)(md) } as usize;
    let mut s = format!("rc={rc};n={n};");
    let pairs = if rc > 0 {
        (rc as usize).min(n)
    } else if rc == 0 {
        n
    } else if rc == PCRE2_ERROR_PARTIAL {
        1.min(n)
    } else {
        0
    };
    if pairs > 0 || rc >= 0 || rc == PCRE2_ERROR_PARTIAL {
        let ov = unsafe { (a.get_ovector_pointer)(md) };
        s.push_str(&format!("sc={};", unsafe { (a.get_startchar)(md) }));
        for i in 0..(pairs * 2) {
            s.push_str(&format!("{},", unsafe { *ov.add(i) }));
        }
    }
    if rc >= 0 || rc == PCRE2_ERROR_PARTIAL || rc == PCRE2_ERROR_NOMATCH {
        let mark = unsafe { (a.get_mark)(md) };
        if mark.is_null() {
            s.push_str("mark=null;");
        } else {
            let len = unsafe { (a.priv_strlen)(mark) };
            let b = unsafe { std::slice::from_raw_parts(mark, len) };
            s.push_str(&format!("mark={b:02x?};"));
        }
    }
    s.push_str(&format!(
        "mdsz={};hfsz={};",
        unsafe { (a.get_match_data_size)(md) },
        unsafe { (a.get_match_data_heapframes_size)(md) }
    ));
    s
}
