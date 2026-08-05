// Common FFI loader for differential testing of C vs Rust PCRE2 .so files.
// We NEVER call the Rust functions directly — both libraries are loaded via
// libloading and their `_8` exports are invoked through raw symbol pointers,
// exactly as an external C consumer would.
#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};

pub type PcreSize = usize; // PCRE2_SIZE (size_t on 64-bit)

// ---- constants (from pcre2.h) ----
pub const PCRE2_ZERO_TERMINATED: usize = !0usize;
pub const PCRE2_UNSET: usize = !0usize;

pub const PCRE2_ANCHORED: u32 = 0x8000_0000;
pub const PCRE2_ENDANCHORED: u32 = 0x2000_0000;
pub const PCRE2_NO_UTF_CHECK: u32 = 0x4000_0000;
pub const PCRE2_CASELESS: u32 = 0x0000_0008;
pub const PCRE2_DOTALL: u32 = 0x0000_0020;
pub const PCRE2_EXTENDED: u32 = 0x0000_0080;
pub const PCRE2_MULTILINE: u32 = 0x0000_0400;
pub const PCRE2_UCP: u32 = 0x0002_0000;
pub const PCRE2_UTF: u32 = 0x0008_0000;

pub const PCRE2_NOTBOL: u32 = 0x0000_0001;
pub const PCRE2_NOTEOL: u32 = 0x0000_0002;
pub const PCRE2_NOTEMPTY: u32 = 0x0000_0004;
pub const PCRE2_NOTEMPTY_ATSTART: u32 = 0x0000_0008;
pub const PCRE2_DFA_SHORTEST: u32 = 0x0000_0080;

pub const PCRE2_SUBSTITUTE_GLOBAL: u32 = 0x0000_0100;
pub const PCRE2_SUBSTITUTE_EXTENDED: u32 = 0x0000_0200;
pub const PCRE2_SUBSTITUTE_UNSET_EMPTY: u32 = 0x0000_0400;
pub const PCRE2_SUBSTITUTE_UNKNOWN_UNSET: u32 = 0x0000_0800;
pub const PCRE2_SUBSTITUTE_OVERFLOW_LENGTH: u32 = 0x0000_1000;
pub const PCRE2_SUBSTITUTE_LITERAL: u32 = 0x0000_8000;
pub const PCRE2_SUBSTITUTE_REPLACEMENT_ONLY: u32 = 0x0002_0000;

pub const PCRE2_ERROR_NOMATCH: c_int = -1;
pub const PCRE2_ERROR_PARTIAL: c_int = -2;
pub const PCRE2_ERROR_BADOFFSET: c_int = -33;
pub const PCRE2_ERROR_BADOPTION: c_int = -34;
pub const PCRE2_ERROR_BADREPLACEMENT: c_int = -35;
pub const PCRE2_ERROR_DFA_WSSIZE: c_int = -43;
pub const PCRE2_ERROR_NOMEMORY: c_int = -48;
pub const PCRE2_ERROR_NOSUBSTRING: c_int = -49;
pub const PCRE2_ERROR_NULL: c_int = -51;
pub const PCRE2_ERROR_UNAVAILABLE: c_int = -54;
pub const PCRE2_ERROR_UNSET: c_int = -55;
pub const PCRE2_ERROR_REPMISSINGBRACE: c_int = -58;

// INFO selectors
pub const PCRE2_INFO_ALLOPTIONS: u32 = 0;
pub const PCRE2_INFO_ARGOPTIONS: u32 = 1;
pub const PCRE2_INFO_BACKREFMAX: u32 = 2;
pub const PCRE2_INFO_BSR: u32 = 3;
pub const PCRE2_INFO_CAPTURECOUNT: u32 = 4;
pub const PCRE2_INFO_HASCRORLF: u32 = 8;
pub const PCRE2_INFO_JCHANGED: u32 = 9;
pub const PCRE2_INFO_MATCHEMPTY: u32 = 13;
pub const PCRE2_INFO_MAXLOOKBEHIND: u32 = 15;
pub const PCRE2_INFO_MINLENGTH: u32 = 16;
pub const PCRE2_INFO_NAMECOUNT: u32 = 17;
pub const PCRE2_INFO_NAMEENTRYSIZE: u32 = 18;
pub const PCRE2_INFO_NEWLINE: u32 = 20;
pub const PCRE2_INFO_SIZE: u32 = 22;

// CONFIG selectors
pub const PCRE2_CONFIG_BSR: u32 = 0;
pub const PCRE2_CONFIG_JIT: u32 = 1;
pub const PCRE2_CONFIG_LINKSIZE: u32 = 3;
pub const PCRE2_CONFIG_MATCHLIMIT: u32 = 4;
pub const PCRE2_CONFIG_NEWLINE: u32 = 5;
pub const PCRE2_CONFIG_PARENSLIMIT: u32 = 6;
pub const PCRE2_CONFIG_UNICODE: u32 = 9;
pub const PCRE2_CONFIG_DEPTHLIMIT: u32 = 8;
pub const PCRE2_CONFIG_HEAPLIMIT: u32 = 12;

pub fn c_lib_path() -> String {
    "c_src/build/libpcre2.so".to_string()
}

pub fn rust_lib_path() -> String {
    // cargo test builds the debug cdylib
    let candidates = ["target/debug/libpcre2.so", "target/release/libpcre2.so"];
    for c in candidates {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }
    "target/debug/libpcre2.so".to_string()
}

// Function pointer type aliases matching the C signatures.
pub type FnCompile = unsafe extern "C" fn(
    *const u8, usize, u32, *mut c_int, *mut usize, *mut c_void,
) -> *mut c_void;
pub type FnCodeFree = unsafe extern "C" fn(*mut c_void);
pub type FnMatchDataFromPattern =
    unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
pub type FnMatchDataCreate =
    unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
pub type FnMatchDataFree = unsafe extern "C" fn(*mut c_void);
pub type FnMatch = unsafe extern "C" fn(
    *const c_void, *const u8, usize, usize, u32, *mut c_void, *mut c_void,
) -> c_int;
pub type FnDfaMatch = unsafe extern "C" fn(
    *const c_void, *const u8, usize, usize, u32, *mut c_void, *mut c_void,
    *mut c_int, usize,
) -> c_int;
pub type FnGetOvectorCount = unsafe extern "C" fn(*mut c_void) -> u32;
pub type FnGetOvectorPointer = unsafe extern "C" fn(*mut c_void) -> *mut usize;
pub type FnGetStartchar = unsafe extern "C" fn(*mut c_void) -> usize;
pub type FnGetMark = unsafe extern "C" fn(*mut c_void) -> *const u8;
pub type FnPatternInfo =
    unsafe extern "C" fn(*const c_void, u32, *mut c_void) -> c_int;
pub type FnConfig = unsafe extern "C" fn(u32, *mut c_void) -> c_int;
pub type FnGetErrMsg =
    unsafe extern "C" fn(c_int, *mut u8, usize) -> c_int;
pub type FnSubstitute = unsafe extern "C" fn(
    *const c_void, *const u8, usize, usize, u32, *mut c_void, *mut c_void,
    *const u8, usize, *mut u8, *mut usize,
) -> c_int;
pub type FnSubstrLenByNum =
    unsafe extern "C" fn(*mut c_void, u32, *mut usize) -> c_int;
pub type FnSubstrCopyByNum =
    unsafe extern "C" fn(*mut c_void, u32, *mut u8, *mut usize) -> c_int;
pub type FnSubstrGetByNum =
    unsafe extern "C" fn(*mut c_void, u32, *mut *mut u8, *mut usize) -> c_int;
pub type FnSubstrFree = unsafe extern "C" fn(*mut u8);
pub type FnSubstrNumFromName =
    unsafe extern "C" fn(*const c_void, *const u8) -> c_int;
pub type FnSubstrNametableScan = unsafe extern "C" fn(
    *const c_void, *const u8, *mut *const u8, *mut *const u8,
) -> c_int;
pub type FnMaketables = unsafe extern "C" fn(*mut c_void) -> *const u8;
pub type FnMaketablesFree =
    unsafe extern "C" fn(*mut c_void, *const u8);
pub type FnSetNewline = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
pub type FnSetBsr = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
pub type FnCCtxCreate = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
pub type FnCCtxFree = unsafe extern "C" fn(*mut c_void);
pub type FnSetCharTables =
    unsafe extern "C" fn(*mut c_void, *const u8) -> c_int;
pub type FnSetNewlineCtx = unsafe extern "C" fn(*mut c_void, u32) -> c_int;

// Serialize
pub type FnSerializeEncode = unsafe extern "C" fn(
    *const *const c_void, i32, *mut *mut u8, *mut usize, *mut c_void,
) -> i32;
pub type FnSerializeDecode = unsafe extern "C" fn(
    *mut *mut c_void, i32, *const u8, *mut c_void,
) -> i32;
pub type FnSerializeFree = unsafe extern "C" fn(*mut u8);
pub type FnSerializeGetNum = unsafe extern "C" fn(*const u8) -> i32;

/// A loaded PCRE2 library with the symbols we test bound as function pointers.
pub struct Pcre2Lib {
    _lib: Library,
    pub compile: FnCompile,
    pub code_free: FnCodeFree,
    pub md_from_pattern: FnMatchDataFromPattern,
    pub md_create: FnMatchDataCreate,
    pub md_free: FnMatchDataFree,
    pub r#match: FnMatch,
    pub dfa_match: FnDfaMatch,
    pub ovector_count: FnGetOvectorCount,
    pub ovector_ptr: FnGetOvectorPointer,
    pub startchar: FnGetStartchar,
    pub get_mark: FnGetMark,
    pub pattern_info: FnPatternInfo,
    pub config: FnConfig,
    pub get_err_msg: FnGetErrMsg,
    pub substitute: FnSubstitute,
    pub substr_len_bynum: FnSubstrLenByNum,
    pub substr_copy_bynum: FnSubstrCopyByNum,
    pub substr_get_bynum: FnSubstrGetByNum,
    pub substr_free: FnSubstrFree,
    pub substr_num_from_name: FnSubstrNumFromName,
    pub substr_nametable_scan: FnSubstrNametableScan,
    pub maketables: FnMaketables,
    pub maketables_free: FnMaketablesFree,
    pub cctx_create: FnCCtxCreate,
    pub cctx_free: FnCCtxFree,
    pub set_char_tables: FnSetCharTables,
    pub set_newline_ctx: FnSetNewlineCtx,
    pub serialize_encode: FnSerializeEncode,
    pub serialize_decode: FnSerializeDecode,
    pub serialize_free: FnSerializeFree,
    pub serialize_get_num: FnSerializeGetNum,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $name:expr) => {{
        let s: Symbol<$t> = $lib.get($name).expect("symbol not found");
        *s.into_raw()
    }};
}

impl Pcre2Lib {
    pub unsafe fn load(path: &str) -> Pcre2Lib {
        let lib = Library::new(path).expect("load .so");
        let out = Pcre2Lib {
            compile: sym!(lib, FnCompile, b"pcre2_compile_8"),
            code_free: sym!(lib, FnCodeFree, b"pcre2_code_free_8"),
            md_from_pattern: sym!(
                lib, FnMatchDataFromPattern,
                b"pcre2_match_data_create_from_pattern_8"
            ),
            md_create: sym!(lib, FnMatchDataCreate, b"pcre2_match_data_create_8"),
            md_free: sym!(lib, FnMatchDataFree, b"pcre2_match_data_free_8"),
            r#match: sym!(lib, FnMatch, b"pcre2_match_8"),
            dfa_match: sym!(lib, FnDfaMatch, b"pcre2_dfa_match_8"),
            ovector_count: sym!(lib, FnGetOvectorCount, b"pcre2_get_ovector_count_8"),
            ovector_ptr: sym!(lib, FnGetOvectorPointer, b"pcre2_get_ovector_pointer_8"),
            startchar: sym!(lib, FnGetStartchar, b"pcre2_get_startchar_8"),
            get_mark: sym!(lib, FnGetMark, b"pcre2_get_mark_8"),
            pattern_info: sym!(lib, FnPatternInfo, b"pcre2_pattern_info_8"),
            config: sym!(lib, FnConfig, b"pcre2_config_8"),
            get_err_msg: sym!(lib, FnGetErrMsg, b"pcre2_get_error_message_8"),
            substitute: sym!(lib, FnSubstitute, b"pcre2_substitute_8"),
            substr_len_bynum: sym!(lib, FnSubstrLenByNum, b"pcre2_substring_length_bynumber_8"),
            substr_copy_bynum: sym!(lib, FnSubstrCopyByNum, b"pcre2_substring_copy_bynumber_8"),
            substr_get_bynum: sym!(lib, FnSubstrGetByNum, b"pcre2_substring_get_bynumber_8"),
            substr_free: sym!(lib, FnSubstrFree, b"pcre2_substring_free_8"),
            substr_num_from_name: sym!(lib, FnSubstrNumFromName, b"pcre2_substring_number_from_name_8"),
            substr_nametable_scan: sym!(lib, FnSubstrNametableScan, b"pcre2_substring_nametable_scan_8"),
            maketables: sym!(lib, FnMaketables, b"pcre2_maketables_8"),
            maketables_free: sym!(lib, FnMaketablesFree, b"pcre2_maketables_free_8"),
            cctx_create: sym!(lib, FnCCtxCreate, b"pcre2_compile_context_create_8"),
            cctx_free: sym!(lib, FnCCtxFree, b"pcre2_compile_context_free_8"),
            set_char_tables: sym!(lib, FnSetCharTables, b"pcre2_set_character_tables_8"),
            set_newline_ctx: sym!(lib, FnSetNewlineCtx, b"pcre2_set_newline_8"),
            serialize_encode: sym!(lib, FnSerializeEncode, b"pcre2_serialize_encode_8"),
            serialize_decode: sym!(lib, FnSerializeDecode, b"pcre2_serialize_decode_8"),
            serialize_free: sym!(lib, FnSerializeFree, b"pcre2_serialize_free_8"),
            serialize_get_num: sym!(lib, FnSerializeGetNum, b"pcre2_serialize_get_number_of_codes_8"),
            _lib: lib,
        };
        out
    }
}

pub fn both() -> (Pcre2Lib, Pcre2Lib) {
    unsafe { (Pcre2Lib::load(&c_lib_path()), Pcre2Lib::load(&rust_lib_path())) }
}

/// Result of a full compile+match run on one library.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MatchOutcome {
    pub compile_ok: bool,
    pub compile_errcode: c_int,
    pub compile_erroffset: usize,
    pub rc: c_int,
    pub ovector: Vec<usize>,
    pub startchar: usize,
}

impl Pcre2Lib {
    /// Compile+match a pattern/subject on THIS library, returning the outcome.
    pub unsafe fn run_match(
        &self,
        pattern: &[u8],
        options: u32,
        subject: &[u8],
        subject_len: usize,
        start_offset: usize,
        match_options: u32,
        ovecsize: u32,
    ) -> MatchOutcome {
        let mut errcode: c_int = 0;
        let mut erroffset: usize = 0;
        let code = (self.compile)(
            pattern.as_ptr(),
            pattern.len(),
            options,
            &mut errcode,
            &mut erroffset,
            std::ptr::null_mut(),
        );
        if code.is_null() {
            return MatchOutcome {
                compile_ok: false,
                compile_errcode: errcode,
                compile_erroffset: erroffset,
                rc: 0,
                ovector: vec![],
                startchar: 0,
            };
        }
        let md = (self.md_create)(ovecsize, std::ptr::null_mut());
        let rc = (self.r#match)(
            code,
            subject.as_ptr(),
            subject_len,
            start_offset,
            match_options,
            md,
            std::ptr::null_mut(),
        );
        let ocount = (self.ovector_count)(md) as usize;
        let optr = (self.ovector_ptr)(md);
        let mut ovector = Vec::with_capacity(ocount * 2);
        for i in 0..ocount * 2 {
            ovector.push(*optr.add(i));
        }
        let sc = (self.startchar)(md);
        (self.md_free)(md);
        (self.code_free)(code);
        MatchOutcome {
            compile_ok: true,
            compile_errcode: 0,
            compile_erroffset: 0,
            rc,
            ovector,
            startchar: sc,
        }
    }
}
