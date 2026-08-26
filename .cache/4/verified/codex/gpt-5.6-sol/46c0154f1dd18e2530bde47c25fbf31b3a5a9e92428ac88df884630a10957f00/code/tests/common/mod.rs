use libloading::Library;
use std::ffi::c_void;
use std::mem::size_of;
use std::path::{Path, PathBuf};

pub type Code = c_void;
pub type GeneralContext = c_void;
pub type CompileContext = c_void;
pub type MatchContext = c_void;
pub type ConvertContext = c_void;
pub type MatchData = c_void;

pub const ZERO_TERMINATED: usize = usize::MAX;

pub struct Libraries {
    pub c: Library,
    pub rust: Library,
}

impl Libraries {
    pub unsafe fn open() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libpcre2.so");
        let rust_path = rust_library_path(&root);
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        Self {
            c: unsafe { Library::new(c_path).expect("load C library") },
            rust: unsafe { Library::new(rust_path).expect("load Rust library") },
        }
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    if let Ok(path) = std::env::var("PCRE2_RUST_SO") {
        return PathBuf::from(path);
    }
    root.join("target/debug/libpcre2.so")
}

pub unsafe fn sym<T: Copy>(library: &Library, name: &[u8]) -> T {
    *unsafe { library.get::<T>(name) }
        .unwrap_or_else(|error| panic!("load {}: {error}", String::from_utf8_lossy(name)))
}

pub type ConfigFn = unsafe extern "C" fn(u32, *mut c_void) -> i32;
pub type GetErrorMessageFn = unsafe extern "C" fn(i32, *mut u8, usize) -> i32;
pub type CompileFn = unsafe extern "C" fn(
    *const u8,
    usize,
    u32,
    *mut i32,
    *mut usize,
    *mut CompileContext,
) -> *mut Code;
pub type CodeFreeFn = unsafe extern "C" fn(*mut Code);
pub type CodeCopyFn = unsafe extern "C" fn(*const Code) -> *mut Code;
pub type PatternInfoFn = unsafe extern "C" fn(*const Code, u32, *mut c_void) -> i32;
pub type MatchDataCreateFn = unsafe extern "C" fn(u32, *mut GeneralContext) -> *mut MatchData;
pub type MatchDataFromPatternFn =
    unsafe extern "C" fn(*const Code, *mut GeneralContext) -> *mut MatchData;
pub type MatchDataFreeFn = unsafe extern "C" fn(*mut MatchData);
pub type MatchFn = unsafe extern "C" fn(
    *const Code,
    *const u8,
    usize,
    usize,
    u32,
    *mut MatchData,
    *mut MatchContext,
) -> i32;
pub type DfaMatchFn = unsafe extern "C" fn(
    *const Code,
    *const u8,
    usize,
    usize,
    u32,
    *mut MatchData,
    *mut MatchContext,
    *mut i32,
    usize,
) -> i32;
pub type GetOvectorCountFn = unsafe extern "C" fn(*mut MatchData) -> u32;
pub type GetOvectorPointerFn = unsafe extern "C" fn(*mut MatchData) -> *mut usize;
pub type GetSizeFn = unsafe extern "C" fn(*mut MatchData) -> usize;
pub type GetMarkFn = unsafe extern "C" fn(*mut MatchData) -> *const u8;
pub type NextMatchFn = unsafe extern "C" fn(*mut MatchData, *mut usize, *mut u32) -> i32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileSnapshot {
    pub error: i32,
    pub offset: usize,
    pub info: Vec<(i32, [u8; 16])>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSnapshot {
    pub rc: i32,
    pub ovector_count: u32,
    pub ovector: Vec<usize>,
    pub startchar: usize,
    pub mark: Option<Vec<u8>>,
    pub data_size: usize,
    pub heapframes_size: usize,
}

pub unsafe fn compile_snapshot(
    library: &Library,
    pattern: &[u8],
    length: usize,
    options: u32,
    context: *mut CompileContext,
) -> (CompileSnapshot, *mut Code) {
    let compile: CompileFn = unsafe { sym(library, b"pcre2_compile_8\0") };
    let info_fn: PatternInfoFn = unsafe { sym(library, b"pcre2_pattern_info_8\0") };
    let mut error = i32::MIN;
    let mut offset = usize::MAX - 1;
    let code = unsafe {
        compile(
            pattern.as_ptr(),
            length,
            options,
            &mut error,
            &mut offset,
            context,
        )
    };
    let mut info = Vec::new();
    if !code.is_null() {
        for selector in 0..=26 {
            let mut aligned = [usize::from_ne_bytes([0xa5; size_of::<usize>()]); 2];
            let rc = unsafe { info_fn(code, selector, aligned.as_mut_ptr().cast::<c_void>()) };
            let mut bytes = [0_u8; 16];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    aligned.as_ptr().cast::<u8>(),
                    bytes.as_mut_ptr(),
                    bytes.len(),
                );
            }
            if selector == 7 || selector == 19 {
                bytes.fill(0);
            }
            info.push((rc, bytes));
        }
    }
    (
        CompileSnapshot {
            error,
            offset,
            info,
        },
        code,
    )
}

pub unsafe fn match_snapshot(
    library: &Library,
    code: *const Code,
    subject: &[u8],
    length: usize,
    start: usize,
    options: u32,
    context: *mut MatchContext,
) -> MatchSnapshot {
    let create: MatchDataFromPatternFn =
        unsafe { sym(library, b"pcre2_match_data_create_from_pattern_8\0") };
    let free: MatchDataFreeFn = unsafe { sym(library, b"pcre2_match_data_free_8\0") };
    let run: MatchFn = unsafe { sym(library, b"pcre2_match_8\0") };
    let count_fn: GetOvectorCountFn = unsafe { sym(library, b"pcre2_get_ovector_count_8\0") };
    let vector_fn: GetOvectorPointerFn = unsafe { sym(library, b"pcre2_get_ovector_pointer_8\0") };
    let start_fn: GetSizeFn = unsafe { sym(library, b"pcre2_get_startchar_8\0") };
    let size_fn: GetSizeFn = unsafe { sym(library, b"pcre2_get_match_data_size_8\0") };
    let heap_fn: GetSizeFn = unsafe { sym(library, b"pcre2_get_match_data_heapframes_size_8\0") };
    let mark_fn: GetMarkFn = unsafe { sym(library, b"pcre2_get_mark_8\0") };

    let data = unsafe { create(code, std::ptr::null_mut()) };
    assert!(!data.is_null());
    let subject_pointer = if subject.is_empty() {
        std::ptr::null()
    } else {
        subject.as_ptr()
    };
    let rc = unsafe { run(code, subject_pointer, length, start, options, data, context) };
    let ovector_count = unsafe { count_fn(data) };
    let vector_pointer = unsafe { vector_fn(data) };
    let populated = if rc > 0 { rc as usize } else { 1 };
    let pair_count = populated.min(ovector_count as usize);
    let ovector = unsafe { std::slice::from_raw_parts(vector_pointer, pair_count * 2) }.to_vec();
    let mark_pointer = unsafe { mark_fn(data) };
    let mark = if mark_pointer.is_null() {
        None
    } else {
        let mut length = 0;
        while unsafe { *mark_pointer.add(length) } != 0 {
            length += 1;
        }
        Some(unsafe { std::slice::from_raw_parts(mark_pointer, length) }.to_vec())
    };
    let snapshot = MatchSnapshot {
        rc,
        ovector_count,
        ovector,
        startchar: unsafe { start_fn(data) },
        mark,
        data_size: unsafe { size_fn(data) },
        heapframes_size: unsafe { heap_fn(data) },
    };
    unsafe { free(data) };
    snapshot
}

pub fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
