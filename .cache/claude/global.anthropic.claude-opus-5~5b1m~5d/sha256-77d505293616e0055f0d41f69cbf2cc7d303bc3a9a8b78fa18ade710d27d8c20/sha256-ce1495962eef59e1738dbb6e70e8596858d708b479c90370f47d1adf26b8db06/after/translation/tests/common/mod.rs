//! Shared differential-test harness.
//!
//! Loads BOTH the C `libpcre2.so` and the Rust `libpcre2.so` via `libloading`
//! and exposes every public PCRE2 8-bit entry point as a raw `extern "C"` fn
//! pointer. Tests NEVER call Rust functions directly — everything goes through
//! the `.so` exports, so the `#[no_mangle]` wrappers are exercised too.
#![allow(dead_code, non_snake_case, non_camel_case_types)]

pub mod diff;

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

pub type SIZE = usize;
pub type SPTR = *const u8;
pub type UCHAR = u8;

// ---------------------------------------------------------------- fn typedefs
pub type FnConfig = unsafe extern "C" fn(u32, *mut c_void) -> c_int;

pub type FnGenCtxCreate = unsafe extern "C" fn(
    Option<unsafe extern "C" fn(SIZE, *mut c_void) -> *mut c_void>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    *mut c_void,
) -> *mut c_void;
pub type FnCtxCopy = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
pub type FnCtxCreate = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
pub type FnCtxFree = unsafe extern "C" fn(*mut c_void);

pub type FnSetU32 = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
pub type FnSetSize = unsafe extern "C" fn(*mut c_void, SIZE) -> c_int;
pub type FnSetTables = unsafe extern "C" fn(*mut c_void, *const u8) -> c_int;
pub type FnSetPtrPair =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> c_int;
pub type FnSetRecMemMgmt = unsafe extern "C" fn(
    *mut c_void,
    Option<unsafe extern "C" fn(SIZE, *mut c_void) -> *mut c_void>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    *mut c_void,
) -> c_int;

pub type FnCompile = unsafe extern "C" fn(
    SPTR,
    SIZE,
    u32,
    *mut c_int,
    *mut SIZE,
    *mut c_void,
) -> *mut c_void;
pub type FnCodeFree = unsafe extern "C" fn(*mut c_void);
pub type FnCodeCopy = unsafe extern "C" fn(*const c_void) -> *mut c_void;

pub type FnPatternInfo =
    unsafe extern "C" fn(*const c_void, u32, *mut c_void) -> c_int;
pub type FnCalloutEnumerate =
    unsafe extern "C" fn(*const c_void, *mut c_void, *mut c_void) -> c_int;

pub type FnMatchDataCreate = unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
pub type FnMatchDataCreateFromPattern =
    unsafe extern "C" fn(*const c_void, *mut c_void) -> *mut c_void;
pub type FnMatchDataFree = unsafe extern "C" fn(*mut c_void);

pub type FnMatch = unsafe extern "C" fn(
    *const c_void,
    SPTR,
    SIZE,
    SIZE,
    u32,
    *mut c_void,
    *mut c_void,
) -> c_int;
pub type FnDfaMatch = unsafe extern "C" fn(
    *const c_void,
    SPTR,
    SIZE,
    SIZE,
    u32,
    *mut c_void,
    *mut c_void,
    *mut c_int,
    SIZE,
) -> c_int;

pub type FnGetMark = unsafe extern "C" fn(*mut c_void) -> SPTR;
pub type FnGetSize = unsafe extern "C" fn(*mut c_void) -> SIZE;
pub type FnGetU32 = unsafe extern "C" fn(*mut c_void) -> u32;
pub type FnGetOvecPtr = unsafe extern "C" fn(*mut c_void) -> *mut SIZE;
pub type FnNextMatch =
    unsafe extern "C" fn(*mut c_void, *mut SIZE, *mut u32) -> c_int;

pub type FnSubstringCopyByName =
    unsafe extern "C" fn(*mut c_void, SPTR, *mut UCHAR, *mut SIZE) -> c_int;
pub type FnSubstringCopyByNumber =
    unsafe extern "C" fn(*mut c_void, u32, *mut UCHAR, *mut SIZE) -> c_int;
pub type FnSubstringFree = unsafe extern "C" fn(*mut UCHAR);
pub type FnSubstringGetByName =
    unsafe extern "C" fn(*mut c_void, SPTR, *mut *mut UCHAR, *mut SIZE) -> c_int;
pub type FnSubstringGetByNumber =
    unsafe extern "C" fn(*mut c_void, u32, *mut *mut UCHAR, *mut SIZE) -> c_int;
pub type FnSubstringLengthByName =
    unsafe extern "C" fn(*mut c_void, SPTR, *mut SIZE) -> c_int;
pub type FnSubstringLengthByNumber =
    unsafe extern "C" fn(*mut c_void, u32, *mut SIZE) -> c_int;
pub type FnSubstringNametableScan =
    unsafe extern "C" fn(*const c_void, SPTR, *mut SPTR, *mut SPTR) -> c_int;
pub type FnSubstringNumberFromName =
    unsafe extern "C" fn(*const c_void, SPTR) -> c_int;
pub type FnSubstringListFree = unsafe extern "C" fn(*mut *mut UCHAR);
pub type FnSubstringListGet =
    unsafe extern "C" fn(*mut c_void, *mut *mut *mut UCHAR, *mut *mut SIZE) -> c_int;

pub type FnSerializeEncode = unsafe extern "C" fn(
    *const *const c_void,
    i32,
    *mut *mut u8,
    *mut SIZE,
    *mut c_void,
) -> i32;
pub type FnSerializeDecode = unsafe extern "C" fn(
    *mut *mut c_void,
    i32,
    *const u8,
    *mut c_void,
) -> i32;
pub type FnSerializeGetNumber = unsafe extern "C" fn(*const u8) -> i32;
pub type FnSerializeFree = unsafe extern "C" fn(*mut u8);

pub type FnSubstitute = unsafe extern "C" fn(
    *const c_void,
    SPTR,
    SIZE,
    SIZE,
    u32,
    *mut c_void,
    *mut c_void,
    SPTR,
    SIZE,
    *mut UCHAR,
    *mut SIZE,
) -> c_int;

pub type FnPatternConvert = unsafe extern "C" fn(
    SPTR,
    SIZE,
    u32,
    *mut *mut UCHAR,
    *mut SIZE,
    *mut c_void,
) -> c_int;
pub type FnConvertedPatternFree = unsafe extern "C" fn(*mut UCHAR);

pub type FnJitCompile = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
pub type FnJitMatch = FnMatch;
pub type FnJitFreeUnused = unsafe extern "C" fn(*mut c_void);
pub type FnJitStackCreate =
    unsafe extern "C" fn(SIZE, SIZE, *mut c_void) -> *mut c_void;
pub type FnJitStackAssign =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
pub type FnJitStackFree = unsafe extern "C" fn(*mut c_void);

pub type FnGetErrorMessage =
    unsafe extern "C" fn(c_int, *mut UCHAR, SIZE) -> c_int;
pub type FnMaketables = unsafe extern "C" fn(*mut c_void) -> *const u8;
pub type FnMaketablesFree = unsafe extern "C" fn(*mut c_void, *const u8);

// ------------------------------------------- low-level (exported `_pcre2_*_8`)
pub type FnValidUtf = unsafe extern "C" fn(SPTR, SIZE, *mut SIZE) -> c_int;
pub type FnOrd2Utf = unsafe extern "C" fn(u32, *mut UCHAR) -> u32;
pub type FnStrlen = unsafe extern "C" fn(SPTR) -> SIZE;
pub type FnStrcmp = unsafe extern "C" fn(SPTR, SPTR) -> c_int;
pub type FnStrcmpC8 = unsafe extern "C" fn(SPTR, *const c_char) -> c_int;
pub type FnStrncmp = unsafe extern "C" fn(SPTR, SPTR, SIZE) -> c_int;
pub type FnStrncmpC8 = unsafe extern "C" fn(SPTR, *const c_char, SIZE) -> c_int;
pub type FnStrcpyC8 = unsafe extern "C" fn(*mut UCHAR, *const c_char) -> SIZE;
pub type FnExtuni = unsafe extern "C" fn(
    u32,
    SPTR,
    SPTR,
    SPTR,
    c_int,
    *mut c_int,
) -> SPTR;
pub type FnXclass = unsafe extern "C" fn(u32, SPTR, *const u8, c_int) -> c_int;
pub type FnEclass =
    unsafe extern "C" fn(u32, SPTR, SPTR, *const u8, c_int) -> c_int;
pub type FnScriptRun = unsafe extern "C" fn(SPTR, SPTR, c_int) -> c_int;
pub type FnIsNewline = unsafe extern "C" fn(
    SPTR,
    u32,
    SPTR,
    *mut u32,
    c_int,
) -> c_int;
pub type FnWasNewline = FnIsNewline;
pub type FnFindBracket = unsafe extern "C" fn(SPTR, c_int, c_int) -> SPTR;
pub type FnCkdSmul = unsafe extern "C" fn(*mut SIZE, c_int, c_int) -> c_int;
pub type FnMemctlMalloc =
    unsafe extern "C" fn(SIZE, *mut c_void) -> *mut c_void;
pub type FnStudy = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type FnUpdateClassbits =
    unsafe extern "C" fn(u32, u32, c_int, *mut u8);
pub type FnGetHashFromName = unsafe extern "C" fn(SPTR, u32) -> u16;
pub type FnJitGetSize = unsafe extern "C" fn(*mut c_void) -> SIZE;
pub type FnJitGetTarget = unsafe extern "C" fn() -> *const c_char;
pub type FnJitFree = unsafe extern "C" fn(*mut c_void, *mut c_void);
pub type FnJitFreeRodata = unsafe extern "C" fn(*mut c_void, *mut c_void);

// --------------------------------------------------------------------- loader
pub struct Api {
    // keep the library alive for the whole process
    _lib: &'static libloading::Library,
    pub name: &'static str,

    pub config: FnConfig,

    pub general_context_create: FnGenCtxCreate,
    pub general_context_copy: FnCtxCopy,
    pub general_context_free: FnCtxFree,
    pub compile_context_create: FnCtxCreate,
    pub compile_context_copy: FnCtxCopy,
    pub compile_context_free: FnCtxFree,
    pub match_context_create: FnCtxCreate,
    pub match_context_copy: FnCtxCopy,
    pub match_context_free: FnCtxFree,
    pub convert_context_create: FnCtxCreate,
    pub convert_context_copy: FnCtxCopy,
    pub convert_context_free: FnCtxFree,

    pub set_bsr: FnSetU32,
    pub set_newline: FnSetU32,
    pub set_max_varlookbehind: FnSetU32,
    pub set_parens_nest_limit: FnSetU32,
    pub set_compile_extra_options: FnSetU32,
    pub set_optimize: FnSetU32,
    pub set_max_pattern_length: FnSetSize,
    pub set_max_pattern_compiled_length: FnSetSize,
    pub set_character_tables: FnSetTables,
    pub set_compile_recursion_guard: FnSetPtrPair,
    pub set_callout: FnSetPtrPair,
    pub set_substitute_callout: FnSetPtrPair,
    pub set_substitute_case_callout: FnSetPtrPair,
    pub set_depth_limit: FnSetU32,
    pub set_heap_limit: FnSetU32,
    pub set_match_limit: FnSetU32,
    pub set_recursion_limit: FnSetU32,
    pub set_offset_limit: FnSetSize,
    pub set_recursion_memory_management: FnSetRecMemMgmt,
    pub set_glob_escape: FnSetU32,
    pub set_glob_separator: FnSetU32,

    pub compile: FnCompile,
    pub code_free: FnCodeFree,
    pub code_copy: FnCodeCopy,
    pub code_copy_with_tables: FnCodeCopy,

    pub pattern_info: FnPatternInfo,
    pub callout_enumerate: FnCalloutEnumerate,

    pub match_data_create: FnMatchDataCreate,
    pub match_data_create_from_pattern: FnMatchDataCreateFromPattern,
    pub match_data_free: FnMatchDataFree,
    pub do_match: FnMatch,
    pub dfa_match: FnDfaMatch,
    pub get_mark: FnGetMark,
    pub get_match_data_size: FnGetSize,
    pub get_match_data_heapframes_size: FnGetSize,
    pub get_ovector_count: FnGetU32,
    pub get_ovector_pointer: FnGetOvecPtr,
    pub get_startchar: FnGetSize,
    pub next_match: FnNextMatch,

    pub substring_copy_byname: FnSubstringCopyByName,
    pub substring_copy_bynumber: FnSubstringCopyByNumber,
    pub substring_free: FnSubstringFree,
    pub substring_get_byname: FnSubstringGetByName,
    pub substring_get_bynumber: FnSubstringGetByNumber,
    pub substring_length_byname: FnSubstringLengthByName,
    pub substring_length_bynumber: FnSubstringLengthByNumber,
    pub substring_nametable_scan: FnSubstringNametableScan,
    pub substring_number_from_name: FnSubstringNumberFromName,
    pub substring_list_free: FnSubstringListFree,
    pub substring_list_get: FnSubstringListGet,

    pub serialize_encode: FnSerializeEncode,
    pub serialize_decode: FnSerializeDecode,
    pub serialize_get_number_of_codes: FnSerializeGetNumber,
    pub serialize_free: FnSerializeFree,

    pub substitute: FnSubstitute,

    pub pattern_convert: FnPatternConvert,
    pub converted_pattern_free: FnConvertedPatternFree,

    pub jit_compile: FnJitCompile,
    pub jit_match: FnJitMatch,
    pub jit_free_unused_memory: FnJitFreeUnused,
    pub jit_stack_create: FnJitStackCreate,
    pub jit_stack_assign: FnJitStackAssign,
    pub jit_stack_free: FnJitStackFree,

    pub get_error_message: FnGetErrorMessage,
    pub maketables: FnMaketables,
    pub maketables_free: FnMaketablesFree,

    // low-level exported internals
    pub valid_utf: FnValidUtf,
    pub ord2utf: FnOrd2Utf,
    pub strlen: FnStrlen,
    pub strcmp: FnStrcmp,
    pub strcmp_c8: FnStrcmpC8,
    pub strncmp: FnStrncmp,
    pub strncmp_c8: FnStrncmpC8,
    pub strcpy_c8: FnStrcpyC8,
    pub extuni: FnExtuni,
    pub xclass: FnXclass,
    pub eclass: FnEclass,
    pub script_run: FnScriptRun,
    pub is_newline: FnIsNewline,
    pub was_newline: FnWasNewline,
    pub find_bracket: FnFindBracket,
    pub ckd_smul: FnCkdSmul,
    pub memctl_malloc: FnMemctlMalloc,
    pub study: FnStudy,
    pub update_classbits: FnUpdateClassbits,
    pub get_hash_from_name: FnGetHashFromName,
    pub jit_get_size: FnJitGetSize,
    pub jit_get_target: FnJitGetTarget,
    pub jit_free: FnJitFree,
    pub jit_free_rodata: FnJitFreeRodata,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

pub fn c_so_path() -> PathBuf {
    repo_root().join("c_src/build/libpcre2.so")
}

pub fn rust_so_path() -> PathBuf {
    // Always the RELEASE cdylib: the crate's release profile disables
    // overflow-checks, matching C's wrapping arithmetic.
    repo_root().join("translation/target/release/libpcre2.so")
}

unsafe fn sym<T: Copy>(lib: &libloading::Library, name: &str) -> T {
    let s: libloading::Symbol<T> = lib
        .get(format!("{}\0", name).as_bytes())
        .unwrap_or_else(|e| panic!("symbol `{}` not found: {}", name, e));
    *s
}

impl Api {
    pub fn load(path: &std::path::Path, name: &'static str) -> Api {
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("cannot load {:?}: {}", path, e))
        }));
        unsafe {
            Api {
                _lib: lib,
                name,
                config: sym(lib, "pcre2_config_8"),

                general_context_create: sym(lib, "pcre2_general_context_create_8"),
                general_context_copy: sym(lib, "pcre2_general_context_copy_8"),
                general_context_free: sym(lib, "pcre2_general_context_free_8"),
                compile_context_create: sym(lib, "pcre2_compile_context_create_8"),
                compile_context_copy: sym(lib, "pcre2_compile_context_copy_8"),
                compile_context_free: sym(lib, "pcre2_compile_context_free_8"),
                match_context_create: sym(lib, "pcre2_match_context_create_8"),
                match_context_copy: sym(lib, "pcre2_match_context_copy_8"),
                match_context_free: sym(lib, "pcre2_match_context_free_8"),
                convert_context_create: sym(lib, "pcre2_convert_context_create_8"),
                convert_context_copy: sym(lib, "pcre2_convert_context_copy_8"),
                convert_context_free: sym(lib, "pcre2_convert_context_free_8"),

                set_bsr: sym(lib, "pcre2_set_bsr_8"),
                set_newline: sym(lib, "pcre2_set_newline_8"),
                set_max_varlookbehind: sym(lib, "pcre2_set_max_varlookbehind_8"),
                set_parens_nest_limit: sym(lib, "pcre2_set_parens_nest_limit_8"),
                set_compile_extra_options: sym(
                    lib,
                    "pcre2_set_compile_extra_options_8",
                ),
                set_optimize: sym(lib, "pcre2_set_optimize_8"),
                set_max_pattern_length: sym(lib, "pcre2_set_max_pattern_length_8"),
                set_max_pattern_compiled_length: sym(
                    lib,
                    "pcre2_set_max_pattern_compiled_length_8",
                ),
                set_character_tables: sym(lib, "pcre2_set_character_tables_8"),
                set_compile_recursion_guard: sym(
                    lib,
                    "pcre2_set_compile_recursion_guard_8",
                ),
                set_callout: sym(lib, "pcre2_set_callout_8"),
                set_substitute_callout: sym(lib, "pcre2_set_substitute_callout_8"),
                set_substitute_case_callout: sym(
                    lib,
                    "pcre2_set_substitute_case_callout_8",
                ),
                set_depth_limit: sym(lib, "pcre2_set_depth_limit_8"),
                set_heap_limit: sym(lib, "pcre2_set_heap_limit_8"),
                set_match_limit: sym(lib, "pcre2_set_match_limit_8"),
                set_recursion_limit: sym(lib, "pcre2_set_recursion_limit_8"),
                set_offset_limit: sym(lib, "pcre2_set_offset_limit_8"),
                set_recursion_memory_management: sym(
                    lib,
                    "pcre2_set_recursion_memory_management_8",
                ),
                set_glob_escape: sym(lib, "pcre2_set_glob_escape_8"),
                set_glob_separator: sym(lib, "pcre2_set_glob_separator_8"),

                compile: sym(lib, "pcre2_compile_8"),
                code_free: sym(lib, "pcre2_code_free_8"),
                code_copy: sym(lib, "pcre2_code_copy_8"),
                code_copy_with_tables: sym(lib, "pcre2_code_copy_with_tables_8"),

                pattern_info: sym(lib, "pcre2_pattern_info_8"),
                callout_enumerate: sym(lib, "pcre2_callout_enumerate_8"),

                match_data_create: sym(lib, "pcre2_match_data_create_8"),
                match_data_create_from_pattern: sym(
                    lib,
                    "pcre2_match_data_create_from_pattern_8",
                ),
                match_data_free: sym(lib, "pcre2_match_data_free_8"),
                do_match: sym(lib, "pcre2_match_8"),
                dfa_match: sym(lib, "pcre2_dfa_match_8"),
                get_mark: sym(lib, "pcre2_get_mark_8"),
                get_match_data_size: sym(lib, "pcre2_get_match_data_size_8"),
                get_match_data_heapframes_size: sym(
                    lib,
                    "pcre2_get_match_data_heapframes_size_8",
                ),
                get_ovector_count: sym(lib, "pcre2_get_ovector_count_8"),
                get_ovector_pointer: sym(lib, "pcre2_get_ovector_pointer_8"),
                get_startchar: sym(lib, "pcre2_get_startchar_8"),
                next_match: sym(lib, "pcre2_next_match_8"),

                substring_copy_byname: sym(lib, "pcre2_substring_copy_byname_8"),
                substring_copy_bynumber: sym(lib, "pcre2_substring_copy_bynumber_8"),
                substring_free: sym(lib, "pcre2_substring_free_8"),
                substring_get_byname: sym(lib, "pcre2_substring_get_byname_8"),
                substring_get_bynumber: sym(lib, "pcre2_substring_get_bynumber_8"),
                substring_length_byname: sym(lib, "pcre2_substring_length_byname_8"),
                substring_length_bynumber: sym(
                    lib,
                    "pcre2_substring_length_bynumber_8",
                ),
                substring_nametable_scan: sym(
                    lib,
                    "pcre2_substring_nametable_scan_8",
                ),
                substring_number_from_name: sym(
                    lib,
                    "pcre2_substring_number_from_name_8",
                ),
                substring_list_free: sym(lib, "pcre2_substring_list_free_8"),
                substring_list_get: sym(lib, "pcre2_substring_list_get_8"),

                serialize_encode: sym(lib, "pcre2_serialize_encode_8"),
                serialize_decode: sym(lib, "pcre2_serialize_decode_8"),
                serialize_get_number_of_codes: sym(
                    lib,
                    "pcre2_serialize_get_number_of_codes_8",
                ),
                serialize_free: sym(lib, "pcre2_serialize_free_8"),

                substitute: sym(lib, "pcre2_substitute_8"),

                pattern_convert: sym(lib, "pcre2_pattern_convert_8"),
                converted_pattern_free: sym(lib, "pcre2_converted_pattern_free_8"),

                jit_compile: sym(lib, "pcre2_jit_compile_8"),
                jit_match: sym(lib, "pcre2_jit_match_8"),
                jit_free_unused_memory: sym(lib, "pcre2_jit_free_unused_memory_8"),
                jit_stack_create: sym(lib, "pcre2_jit_stack_create_8"),
                jit_stack_assign: sym(lib, "pcre2_jit_stack_assign_8"),
                jit_stack_free: sym(lib, "pcre2_jit_stack_free_8"),

                get_error_message: sym(lib, "pcre2_get_error_message_8"),
                maketables: sym(lib, "pcre2_maketables_8"),
                maketables_free: sym(lib, "pcre2_maketables_free_8"),

                valid_utf: sym(lib, "_pcre2_valid_utf_8"),
                ord2utf: sym(lib, "_pcre2_ord2utf_8"),
                strlen: sym(lib, "_pcre2_strlen_8"),
                strcmp: sym(lib, "_pcre2_strcmp_8"),
                strcmp_c8: sym(lib, "_pcre2_strcmp_c8_8"),
                strncmp: sym(lib, "_pcre2_strncmp_8"),
                strncmp_c8: sym(lib, "_pcre2_strncmp_c8_8"),
                strcpy_c8: sym(lib, "_pcre2_strcpy_c8_8"),
                extuni: sym(lib, "_pcre2_extuni_8"),
                xclass: sym(lib, "_pcre2_xclass_8"),
                eclass: sym(lib, "_pcre2_eclass_8"),
                script_run: sym(lib, "_pcre2_script_run_8"),
                is_newline: sym(lib, "_pcre2_is_newline_8"),
                was_newline: sym(lib, "_pcre2_was_newline_8"),
                find_bracket: sym(lib, "_pcre2_find_bracket_8"),
                ckd_smul: sym(lib, "_pcre2_ckd_smul_8"),
                memctl_malloc: sym(lib, "_pcre2_memctl_malloc_8"),
                study: sym(lib, "_pcre2_study_8"),
                update_classbits: sym(lib, "_pcre2_update_classbits_8"),
                get_hash_from_name: sym(lib, "_pcre2_compile_get_hash_from_name8"),
                jit_get_size: sym(lib, "_pcre2_jit_get_size_8"),
                jit_get_target: sym(lib, "_pcre2_jit_get_target_8"),
                jit_free: sym(lib, "_pcre2_jit_free_8"),
                jit_free_rodata: sym(lib, "_pcre2_jit_free_rodata_8"),
            }
        }
    }

    /// Raw access for data symbols / rarely used exports.
    pub unsafe fn raw<T: Copy>(&self, name: &str) -> T {
        sym(self._lib, name)
    }

    pub unsafe fn data_ptr(&self, name: &str) -> *const u8 {
        let s: libloading::Symbol<*const u8> = self
            ._lib
            .get(format!("{}\0", name).as_bytes())
            .unwrap_or_else(|e| panic!("data symbol `{}` not found: {}", name, e));
        s.into_raw().into_raw() as *const u8
    }
}

// ------------------------------------------------------------- the two APIs
use std::sync::OnceLock;

static C_API: OnceLock<Api> = OnceLock::new();
static R_API: OnceLock<Api> = OnceLock::new();

pub fn c() -> &'static Api {
    C_API.get_or_init(|| Api::load(&c_so_path(), "C"))
}
pub fn r() -> &'static Api {
    R_API.get_or_init(|| Api::load(&rust_so_path(), "Rust"))
}

/// Both implementations, C first.
pub fn both() -> (&'static Api, &'static Api) {
    (c(), r())
}

// ------------------------------------------------------------------- PRNG
/// Deterministic xorshift64* PRNG so every run is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            lo
        } else {
            lo + self.below(hi - lo)
        }
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Random byte string of length `len` drawn from `alphabet`.
    pub fn bytes_from(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len).map(|_| *self.pick(alphabet)).collect()
    }
    /// Random arbitrary bytes (may be invalid UTF-8).
    pub fn raw_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u32() as u8).collect()
    }
}

// ------------------------------------------------- global-state serialisation
/// Some tests install a process-wide failure-injecting allocator or record
/// callback invocations into `static mut` state. Those tests must not run
/// concurrently with each other *within the same test binary*, so they all take
/// this lock. (Separate test binaries are separate processes and cannot
/// interfere.)
static GLOBAL_STATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn global_lock() -> std::sync::MutexGuard<'static, ()> {
    // A poisoned lock just means an earlier test panicked; the state is reset by
    // each test before use, so recovering is correct here.
    match GLOBAL_STATE.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}
