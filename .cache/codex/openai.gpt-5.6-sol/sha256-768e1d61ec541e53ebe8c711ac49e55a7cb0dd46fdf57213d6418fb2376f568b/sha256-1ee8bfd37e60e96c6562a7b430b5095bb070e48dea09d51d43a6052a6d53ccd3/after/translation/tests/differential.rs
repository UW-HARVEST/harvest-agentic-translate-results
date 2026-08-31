use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

type Context = c_void;
type Code = c_void;
type MatchData = c_void;

const ZERO_TERMINATED: usize = usize::MAX;
const UNSET: usize = usize::MAX;

const ERROR_BADDATA: c_int = -29;
const ERROR_MIXEDTABLES: c_int = -30;
const ERROR_BADMAGIC: c_int = -31;
const ERROR_BADMODE: c_int = -32;
const ERROR_BADOFFSET: c_int = -33;
const ERROR_BADOPTION: c_int = -34;
const ERROR_BADUTFOFFSET: c_int = -36;
const ERROR_DFA_BADRESTART: c_int = -38;
const ERROR_DFA_UFUNC: c_int = -41;
const ERROR_DFA_WSSIZE: c_int = -43;
const ERROR_JIT_BADOPTION: c_int = -45;
const ERROR_NOMEMORY: c_int = -48;
const ERROR_NOSUBSTRING: c_int = -49;
const ERROR_NOUNIQUESUBSTRING: c_int = -50;
const ERROR_NULL: c_int = -51;
const ERROR_UNAVAILABLE: c_int = -54;
const ERROR_UNSET: c_int = -55;
const ERROR_BADOFFSETLIMIT: c_int = -56;
const ERROR_BADSERIALIZEDDATA: c_int = -62;
const ERROR_JIT_UNSUPPORTED: c_int = -68;

const UTF: u32 = 0x0008_0000;
const UCP: u32 = 0x0002_0000;
const LITERAL: u32 = 0x0200_0000;
const USE_OFFSET_LIMIT: u32 = 0x0080_0000;
const MATCH_INVALID_UTF: u32 = 0x0400_0000;
const CASELESS: u32 = 0x0000_0008;
const MULTILINE: u32 = 0x0000_0400;
const DOTALL: u32 = 0x0000_0020;
const DUPNAMES: u32 = 0x0000_0040;
const EXTENDED: u32 = 0x0000_0080;
const UNGREEDY: u32 = 0x0004_0000;
const NO_AUTO_CAPTURE: u32 = 0x0000_2000;
const ANCHORED: u32 = 0x8000_0000;
const ENDANCHORED: u32 = 0x2000_0000;
const NO_UTF_CHECK: u32 = 0x4000_0000;
const NOTBOL: u32 = 0x0000_0001;
const NOTEOL: u32 = 0x0000_0002;
const NOTEMPTY: u32 = 0x0000_0004;
const NOTEMPTY_ATSTART: u32 = 0x0000_0008;
const PARTIAL_SOFT: u32 = 0x0000_0010;
const PARTIAL_HARD: u32 = 0x0000_0020;
const DFA_RESTART: u32 = 0x0000_0040;
const DFA_SHORTEST: u32 = 0x0000_0080;
const SUBSTITUTE_GLOBAL: u32 = 0x0000_0100;
const SUBSTITUTE_EXTENDED: u32 = 0x0000_0200;
const SUBSTITUTE_UNSET_EMPTY: u32 = 0x0000_0400;
const SUBSTITUTE_UNKNOWN_UNSET: u32 = 0x0000_0800;
const SUBSTITUTE_OVERFLOW_LENGTH: u32 = 0x0000_1000;
const NO_JIT: u32 = 0x0000_2000;
const COPY_MATCHED_SUBJECT: u32 = 0x0000_4000;
const SUBSTITUTE_LITERAL: u32 = 0x0000_8000;
const SUBSTITUTE_MATCHED: u32 = 0x0001_0000;
const SUBSTITUTE_REPLACEMENT_ONLY: u32 = 0x0002_0000;

const CONVERT_UTF: u32 = 0x01;
const CONVERT_NO_UTF_CHECK: u32 = 0x02;
const CONVERT_POSIX_BASIC: u32 = 0x04;
const CONVERT_POSIX_EXTENDED: u32 = 0x08;
const CONVERT_GLOB: u32 = 0x10;

const JIT_COMPLETE: u32 = 0x01;
const JIT_PARTIAL_SOFT: u32 = 0x02;
const JIT_PARTIAL_HARD: u32 = 0x04;
const JIT_INVALID_UTF: u32 = 0x100;
const JIT_TEST_ALLOC: u32 = 0x200;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate must have parent")
        .to_path_buf()
}

fn libraries() -> (Library, Library) {
    let root = root();
    let c = root.join("c_src/build/libpcre2.so");
    let rust = root.join("translation/target/release/libpcre2.so");
    assert!(c.is_file(), "missing C library: {}", c.display());
    assert!(rust.is_file(), "missing Rust library: {}", rust.display());
    unsafe {
        (
            Library::new(c).expect("load C library"),
            Library::new(rust).expect("load Rust library"),
        )
    }
}

unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
    unsafe { *library.get::<T>(name).unwrap() }
}

type Compile =
    unsafe extern "C" fn(*const u8, usize, u32, *mut c_int, *mut usize, *mut Context) -> *mut Code;
type CodeFree = unsafe extern "C" fn(*mut Code);
type MatchDataCreate = unsafe extern "C" fn(u32, *mut Context) -> *mut MatchData;
type MatchDataFromPattern = unsafe extern "C" fn(*const Code, *mut Context) -> *mut MatchData;
type MatchDataFree = unsafe extern "C" fn(*mut MatchData);
type Match = unsafe extern "C" fn(
    *const Code,
    *const u8,
    usize,
    usize,
    u32,
    *mut MatchData,
    *mut Context,
) -> c_int;
type DfaMatch = unsafe extern "C" fn(
    *const Code,
    *const u8,
    usize,
    usize,
    u32,
    *mut MatchData,
    *mut Context,
    *mut c_int,
    usize,
) -> c_int;

struct Compiled<'a> {
    code: *mut Code,
    free: CodeFree,
    _library: &'a Library,
}

impl Drop for Compiled<'_> {
    fn drop(&mut self) {
        unsafe { (self.free)(self.code) };
    }
}

unsafe fn compile<'a>(
    library: &'a Library,
    pattern: &[u8],
    length: usize,
    options: u32,
    context: *mut Context,
) -> Result<Compiled<'a>, (c_int, usize)> {
    let compile: Compile = unsafe { symbol(library, b"pcre2_compile_8\0") };
    let free: CodeFree = unsafe { symbol(library, b"pcre2_code_free_8\0") };
    let mut error = 0;
    let mut offset = 0;
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
    if code.is_null() {
        Err((error, offset))
    } else {
        Ok(Compiled {
            code,
            free,
            _library: library,
        })
    }
}

unsafe fn run_match(
    library: &Library,
    pattern: &[u8],
    compile_options: u32,
    subject: &[u8],
    length: usize,
    start: usize,
    match_options: u32,
) -> (c_int, Vec<usize>, usize, usize) {
    let code = unsafe {
        compile(
            library,
            pattern,
            pattern.len(),
            compile_options,
            ptr::null_mut(),
        )
    }
    .unwrap();
    let create: MatchDataFromPattern =
        unsafe { symbol(library, b"pcre2_match_data_create_from_pattern_8\0") };
    let free: MatchDataFree = unsafe { symbol(library, b"pcre2_match_data_free_8\0") };
    let match_fn: Match = unsafe { symbol(library, b"pcre2_match_8\0") };
    let get_count: unsafe extern "C" fn(*mut MatchData) -> u32 =
        unsafe { symbol(library, b"pcre2_get_ovector_count_8\0") };
    let get_vector: unsafe extern "C" fn(*mut MatchData) -> *mut usize =
        unsafe { symbol(library, b"pcre2_get_ovector_pointer_8\0") };
    let get_start: unsafe extern "C" fn(*mut MatchData) -> usize =
        unsafe { symbol(library, b"pcre2_get_startchar_8\0") };
    let get_heap: unsafe extern "C" fn(*mut MatchData) -> usize =
        unsafe { symbol(library, b"pcre2_get_match_data_heapframes_size_8\0") };
    let data = unsafe { create(code.code, ptr::null_mut()) };
    assert!(!data.is_null());
    let subject_ptr = if subject.is_empty() {
        subject.as_ptr()
    } else {
        subject.as_ptr()
    };
    let rc = unsafe {
        match_fn(
            code.code,
            subject_ptr,
            length,
            start,
            match_options,
            data,
            ptr::null_mut(),
        )
    };
    let count = unsafe { get_count(data) } as usize;
    let vector = if rc >= 0 {
        unsafe { std::slice::from_raw_parts(get_vector(data), count * 2).to_vec() }
    } else {
        Vec::new()
    };
    let startchar = unsafe { get_start(data) };
    let heap = unsafe { get_heap(data) };
    unsafe { free(data) };
    (rc, vector, startchar, heap)
}

fn rng_next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn randomized_ascii(state: &mut u64, max: usize) -> Vec<u8> {
    let len = (rng_next(state) as usize % max) + 1;
    (0..len)
        .map(|_| b'a' + (rng_next(state) % 6) as u8)
        .collect()
}

#[test]
fn config_and_messages() {
    unsafe {
        let (c, rust) = libraries();
        type Config = unsafe extern "C" fn(u32, *mut c_void) -> c_int;
        type ErrorMessage = unsafe extern "C" fn(c_int, *mut u8, usize) -> c_int;
        for what in 0..=16 {
            let mut cv = [0u8; 128];
            let mut rv = [0u8; 128];
            let cf: Config = symbol(&c, b"pcre2_config_8\0");
            let rf: Config = symbol(&rust, b"pcre2_config_8\0");
            let crc = cf(what, cv.as_mut_ptr().cast());
            let rrc = rf(what, rv.as_mut_ptr().cast());
            assert_eq!((rrc, rv), (crc, cv), "config selector {what}");
            assert_eq!(
                rf(what, ptr::null_mut()),
                cf(what, ptr::null_mut()),
                "config length selector {what}"
            );
        }

        let cf: ErrorMessage = symbol(&c, b"pcre2_get_error_message_8\0");
        let rf: ErrorMessage = symbol(&rust, b"pcre2_get_error_message_8\0");
        for error in -100..=240 {
            for size in [0, 1, 4, 32, 256] {
                let mut cb = vec![0xa5; size.max(1)];
                let mut rb = vec![0xa5; size.max(1)];
                let crc = cf(error, cb.as_mut_ptr(), size);
                let rrc = rf(error, rb.as_mut_ptr(), size);
                assert_eq!((rrc, rb), (crc, cb), "error={error} size={size}");
            }
        }
    }
}

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

unsafe extern "C" fn tracking_malloc(size: usize, _data: *mut c_void) -> *mut c_void {
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    unsafe { malloc(size) }
}

unsafe extern "C" fn tracking_free(pointer: *mut c_void, _data: *mut c_void) {
    ALLOCATIONS.fetch_sub(1, Ordering::Relaxed);
    unsafe { free(pointer) };
}

unsafe extern "C" fn failing_malloc(_size: usize, _data: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn callback_zero(_block: *mut c_void, data: *mut c_void) -> c_int {
    if !data.is_null() {
        unsafe { *(data.cast::<usize>()) += 1 };
    }
    0
}

unsafe extern "C" fn recursion_guard(depth: u32, data: *mut c_void) -> c_int {
    (depth >= data as usize as u32) as c_int
}

#[test]
fn context_lifecycle_and_setters() {
    unsafe {
        let (c, rust) = libraries();
        for library in [&c, &rust] {
            type GeneralCreate = unsafe extern "C" fn(
                Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
                Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
                *mut c_void,
            ) -> *mut Context;
            type ContextCreate = unsafe extern "C" fn(*mut Context) -> *mut Context;
            type ContextCopy = unsafe extern "C" fn(*mut Context) -> *mut Context;
            type ContextFree = unsafe extern "C" fn(*mut Context);
            type SetU32 = unsafe extern "C" fn(*mut Context, u32) -> c_int;
            type SetUsize = unsafe extern "C" fn(*mut Context, usize) -> c_int;

            let gc: GeneralCreate = symbol(library, b"pcre2_general_context_create_8\0");
            let gf: ContextFree = symbol(library, b"pcre2_general_context_free_8\0");
            let gcopy: ContextCopy = symbol(library, b"pcre2_general_context_copy_8\0");
            let general = gc(
                Some(tracking_malloc),
                Some(tracking_free),
                0x1234usize as *mut c_void,
            );
            assert!(!general.is_null());
            let copied = gcopy(general);
            assert!(!copied.is_null());
            gf(copied);

            for (create_name, copy_name, free_name) in [
                (
                    b"pcre2_compile_context_create_8\0".as_slice(),
                    b"pcre2_compile_context_copy_8\0".as_slice(),
                    b"pcre2_compile_context_free_8\0".as_slice(),
                ),
                (
                    b"pcre2_match_context_create_8\0".as_slice(),
                    b"pcre2_match_context_copy_8\0".as_slice(),
                    b"pcre2_match_context_free_8\0".as_slice(),
                ),
                (
                    b"pcre2_convert_context_create_8\0".as_slice(),
                    b"pcre2_convert_context_copy_8\0".as_slice(),
                    b"pcre2_convert_context_free_8\0".as_slice(),
                ),
            ] {
                let create: ContextCreate = symbol(library, create_name);
                let copy: ContextCopy = symbol(library, copy_name);
                let context_free: ContextFree = symbol(library, free_name);
                let context = create(general);
                assert!(!context.is_null());
                let context_copy = copy(context);
                assert!(!context_copy.is_null());
                context_free(context_copy);
                context_free(context);
                context_free(ptr::null_mut());
            }

            let compile_create: ContextCreate =
                symbol(library, b"pcre2_compile_context_create_8\0");
            let compile_free: ContextFree = symbol(library, b"pcre2_compile_context_free_8\0");
            let cc = compile_create(general);
            for (name, values) in [
                (b"pcre2_set_bsr_8\0".as_slice(), vec![1, 2]),
                (b"pcre2_set_newline_8\0".as_slice(), vec![1, 2, 3, 4, 5, 6]),
                (
                    b"pcre2_set_optimize_8\0".as_slice(),
                    vec![0, 1, 64, 65, 66, 67, 68, 69],
                ),
            ] {
                let set: SetU32 = symbol(library, name);
                for value in values {
                    assert_eq!(set(cc, value), 0);
                }
            }
            for name in [
                b"pcre2_set_compile_extra_options_8\0".as_slice(),
                b"pcre2_set_max_varlookbehind_8\0".as_slice(),
                b"pcre2_set_parens_nest_limit_8\0".as_slice(),
            ] {
                let set: SetU32 = symbol(library, name);
                for value in [0, 1, 17, u32::MAX] {
                    assert_eq!(set(cc, value), 0);
                }
            }
            for name in [
                b"pcre2_set_max_pattern_length_8\0".as_slice(),
                b"pcre2_set_max_pattern_compiled_length_8\0".as_slice(),
            ] {
                let set: SetUsize = symbol(library, name);
                for value in [0, 1, 4096, usize::MAX] {
                    assert_eq!(set(cc, value), 0);
                }
            }
            type SetGuard = unsafe extern "C" fn(
                *mut Context,
                Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>,
                *mut c_void,
            ) -> c_int;
            let set_guard: SetGuard = symbol(library, b"pcre2_set_compile_recursion_guard_8\0");
            assert_eq!(set_guard(cc, None, ptr::null_mut()), 0);
            assert_eq!(
                set_guard(cc, Some(recursion_guard), 100usize as *mut c_void),
                0
            );
            compile_free(cc);

            let match_create: ContextCreate = symbol(library, b"pcre2_match_context_create_8\0");
            let match_free: ContextFree = symbol(library, b"pcre2_match_context_free_8\0");
            let mc = match_create(general);
            for name in [
                b"pcre2_set_depth_limit_8\0".as_slice(),
                b"pcre2_set_recursion_limit_8\0".as_slice(),
                b"pcre2_set_heap_limit_8\0".as_slice(),
                b"pcre2_set_match_limit_8\0".as_slice(),
            ] {
                let set: SetU32 = symbol(library, name);
                for value in [0, 1, 1000, u32::MAX] {
                    assert_eq!(set(mc, value), 0);
                }
            }
            let set_offset: SetUsize = symbol(library, b"pcre2_set_offset_limit_8\0");
            for value in [0, 1, 1000, usize::MAX] {
                assert_eq!(set_offset(mc, value), 0);
            }
            type SetCallout = unsafe extern "C" fn(
                *mut Context,
                Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int>,
                *mut c_void,
            ) -> c_int;
            for name in [
                b"pcre2_set_callout_8\0".as_slice(),
                b"pcre2_set_substitute_callout_8\0".as_slice(),
            ] {
                let set: SetCallout = symbol(library, name);
                assert_eq!(set(mc, None, ptr::null_mut()), 0);
                let mut calls = 0usize;
                assert_eq!(
                    set(mc, Some(callback_zero), (&mut calls as *mut usize).cast()),
                    0
                );
            }
            type SetCaseCallout = unsafe extern "C" fn(
                *mut Context,
                Option<
                    unsafe extern "C" fn(
                        *const u8,
                        usize,
                        *mut u8,
                        usize,
                        c_int,
                        *mut c_void,
                    ) -> usize,
                >,
                *mut c_void,
            ) -> c_int;
            let set_case: SetCaseCallout =
                symbol(library, b"pcre2_set_substitute_case_callout_8\0");
            assert_eq!(set_case(mc, None, ptr::null_mut()), 0);
            type SetRecursionMemory = unsafe extern "C" fn(
                *mut Context,
                Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
                Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
                *mut c_void,
            ) -> c_int;
            let set_recursion: SetRecursionMemory =
                symbol(library, b"pcre2_set_recursion_memory_management_8\0");
            assert_eq!(
                set_recursion(
                    mc,
                    Some(tracking_malloc),
                    Some(tracking_free),
                    ptr::null_mut()
                ),
                0
            );
            match_free(mc);

            let convert_create: ContextCreate =
                symbol(library, b"pcre2_convert_context_create_8\0");
            let convert_free: ContextFree = symbol(library, b"pcre2_convert_context_free_8\0");
            let xc = convert_create(general);
            let set_separator: SetU32 = symbol(library, b"pcre2_set_glob_separator_8\0");
            for value in [b'/' as u32, b'\\' as u32, b'.' as u32] {
                assert_eq!(set_separator(xc, value), 0);
            }
            let set_escape: SetU32 = symbol(library, b"pcre2_set_glob_escape_8\0");
            for value in [0, b'!' as u32, b'\\' as u32, b'~' as u32] {
                assert_eq!(set_escape(xc, value), 0);
            }
            convert_free(xc);
            gf(general);
        }
        assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn compile_info_copy_callouts() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0xd1ff_e2e0_1234_5678;
        let static_patterns: &[(&[u8], u32)] = &[
            (b"", 0),
            (b"a+", 0),
            (b"^a.*z$", MULTILINE | DOTALL),
            (b"(?<x>a)(?<x>b)", DUPNAMES),
            (b"a # comment\n b", EXTENDED),
            (b"a+?", UNGREEDY),
            (b"(a)", NO_AUTO_CAPTURE),
            (b".*meta[", LITERAL),
            ("\u{e9}\u{20ac}\u{1f642}".as_bytes(), UTF | UCP),
        ];
        for (pattern, options) in static_patterns {
            let cc = compile(&c, pattern, pattern.len(), *options, ptr::null_mut());
            let rr = compile(&rust, pattern, pattern.len(), *options, ptr::null_mut());
            assert_eq!(cc.is_ok(), rr.is_ok());
            assert_eq!(cc.err(), rr.err());
        }
        for _ in 0..128 {
            let pattern = randomized_ascii(&mut seed, 24);
            let cc = compile(&c, &pattern, pattern.len(), 0, ptr::null_mut()).unwrap();
            let rr = compile(&rust, &pattern, pattern.len(), 0, ptr::null_mut()).unwrap();
            type PatternInfo = unsafe extern "C" fn(*const Code, u32, *mut c_void) -> c_int;
            let ci: PatternInfo = symbol(&c, b"pcre2_pattern_info_8\0");
            let ri: PatternInfo = symbol(&rust, b"pcre2_pattern_info_8\0");
            for what in 0..=26 {
                let mut cv = [0u8; 64];
                let mut rv = [0u8; 64];
                let crc = ci(cc.code, what, cv.as_mut_ptr().cast());
                let rrc = ri(rr.code, what, rv.as_mut_ptr().cast());
                if what != 7 && what != 19 {
                    assert_eq!((rrc, rv), (crc, cv), "info selector {what}");
                } else {
                    assert_eq!(rrc, crc);
                    assert_eq!(rv.iter().all(|b| *b == 0), cv.iter().all(|b| *b == 0));
                }
                assert_eq!(
                    ri(rr.code, what, ptr::null_mut()),
                    ci(cc.code, what, ptr::null_mut())
                );
            }
        }

        type CodeCopy = unsafe extern "C" fn(*const Code) -> *mut Code;
        for library in [&c, &rust] {
            let code = compile(library, b"(?C1)a(?C'word')", 17, 0, ptr::null_mut()).unwrap();
            let free: CodeFree = symbol(library, b"pcre2_code_free_8\0");
            for name in [
                b"pcre2_code_copy_8\0".as_slice(),
                b"pcre2_code_copy_with_tables_8\0".as_slice(),
            ] {
                let copy: CodeCopy = symbol(library, name);
                let copied = copy(code.code);
                assert!(!copied.is_null());
                free(copied);
            }
            type Enumerate = unsafe extern "C" fn(
                *const Code,
                Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int>,
                *mut c_void,
            ) -> c_int;
            let enumerate: Enumerate = symbol(library, b"pcre2_callout_enumerate_8\0");
            let mut calls = 0usize;
            assert_eq!(
                enumerate(
                    code.code,
                    Some(callback_zero),
                    (&mut calls as *mut usize).cast()
                ),
                0
            );
            assert_eq!(calls, 2);
        }
    }
}

#[test]
fn match_data_and_getters() {
    unsafe {
        let (c, rust) = libraries();
        for requested in [0, 1, 2, 17, 65_535, 65_536, u32::MAX] {
            let mut results = Vec::new();
            for library in [&c, &rust] {
                let create: MatchDataCreate = symbol(library, b"pcre2_match_data_create_8\0");
                let free: MatchDataFree = symbol(library, b"pcre2_match_data_free_8\0");
                let count: unsafe extern "C" fn(*mut MatchData) -> u32 =
                    symbol(library, b"pcre2_get_ovector_count_8\0");
                let size: unsafe extern "C" fn(*mut MatchData) -> usize =
                    symbol(library, b"pcre2_get_match_data_size_8\0");
                let data = create(requested, ptr::null_mut());
                assert!(!data.is_null());
                results.push((count(data), size(data)));
                free(data);
                free(ptr::null_mut());
            }
            assert_eq!(results[0], results[1]);
        }
    }
}

#[test]
fn randomized_match() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0x5eed_1234_cafe_babe;
        for _ in 0..256 {
            let pattern = randomized_ascii(&mut seed, 12);
            let mut subject = randomized_ascii(&mut seed, 48);
            if rng_next(&mut seed) & 1 == 0 {
                let offset = rng_next(&mut seed) as usize % (subject.len() + 1);
                subject.splice(offset..offset, pattern.iter().copied());
            }
            let options = match rng_next(&mut seed) % 8 {
                0 => 0,
                1 => ANCHORED,
                2 => ENDANCHORED,
                3 => NOTBOL,
                4 => NOTEOL,
                5 => NOTEMPTY,
                6 => NOTEMPTY_ATSTART,
                _ => NO_JIT,
            };
            let start = rng_next(&mut seed) as usize % (subject.len() + 1);
            let cr = run_match(&c, &pattern, 0, &subject, subject.len(), start, options);
            let rr = run_match(&rust, &pattern, 0, &subject, subject.len(), start, options);
            assert_eq!(rr, cr);
        }
        for (pattern, subject, coptions, moptions) in [
            (b"a+".as_slice(), b"aa".as_slice(), 0, PARTIAL_SOFT),
            (b"a+".as_slice(), b"aa".as_slice(), 0, PARTIAL_HARD),
            ("\u{e9}+".as_bytes(), "\u{e9}\u{e9}".as_bytes(), UTF, 0),
            (b"a".as_slice(), b"a\0b".as_slice(), 0, 0),
            (b"".as_slice(), b"".as_slice(), 0, COPY_MATCHED_SUBJECT),
        ] {
            assert_eq!(
                run_match(
                    &rust,
                    pattern,
                    coptions,
                    subject,
                    subject.len(),
                    0,
                    moptions
                ),
                run_match(&c, pattern, coptions, subject, subject.len(), 0, moptions)
            );
        }
    }
}

#[test]
fn dfa_and_iteration() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0xa11c_e55d_0dd5_1234;
        for _ in 0..128 {
            let pattern = randomized_ascii(&mut seed, 8);
            let subject = randomized_ascii(&mut seed, 32);
            for options in [0, ANCHORED, DFA_SHORTEST, PARTIAL_SOFT, PARTIAL_HARD] {
                let mut normalized = Vec::new();
                for library in [&c, &rust] {
                    let code =
                        compile(library, &pattern, pattern.len(), 0, ptr::null_mut()).unwrap();
                    let create: MatchDataFromPattern =
                        symbol(library, b"pcre2_match_data_create_from_pattern_8\0");
                    let free: MatchDataFree = symbol(library, b"pcre2_match_data_free_8\0");
                    let dfa: DfaMatch = symbol(library, b"pcre2_dfa_match_8\0");
                    let vector: unsafe extern "C" fn(*mut MatchData) -> *mut usize =
                        symbol(library, b"pcre2_get_ovector_pointer_8\0");
                    let data = create(code.code, ptr::null_mut());
                    let mut workspace = [0; 256];
                    let rc = dfa(
                        code.code,
                        subject.as_ptr(),
                        subject.len(),
                        0,
                        options,
                        data,
                        ptr::null_mut(),
                        workspace.as_mut_ptr(),
                        workspace.len(),
                    );
                    let offsets = if rc > 0 {
                        std::slice::from_raw_parts(vector(data), rc as usize * 2).to_vec()
                    } else {
                        Vec::new()
                    };
                    normalized.push((rc, offsets));
                    free(data);
                }
                assert_eq!(normalized[0], normalized[1]);
            }
        }

        type NextMatch = unsafe extern "C" fn(*mut MatchData, *mut usize, *mut u32) -> c_int;
        for library in [&c, &rust] {
            let code = compile(library, b"a*", 2, 0, ptr::null_mut()).unwrap();
            let create: MatchDataFromPattern =
                symbol(library, b"pcre2_match_data_create_from_pattern_8\0");
            let free: MatchDataFree = symbol(library, b"pcre2_match_data_free_8\0");
            let match_fn: Match = symbol(library, b"pcre2_match_8\0");
            let next: NextMatch = symbol(library, b"pcre2_next_match_8\0");
            let data = create(code.code, ptr::null_mut());
            assert!(match_fn(code.code, b"baa".as_ptr(), 3, 0, 0, data, ptr::null_mut()) >= 0);
            let mut offset = 0;
            let mut options = 0;
            assert_eq!(next(data, &mut offset, &mut options), 1);
            free(data);
        }
    }
}

unsafe fn matched_data<'a>(
    library: &'a Library,
    pattern: &[u8],
    options: u32,
    subject: &[u8],
) -> (Compiled<'a>, *mut MatchData, MatchDataFree) {
    let code =
        unsafe { compile(library, pattern, pattern.len(), options, ptr::null_mut()) }.unwrap();
    let create: MatchDataFromPattern =
        unsafe { symbol(library, b"pcre2_match_data_create_from_pattern_8\0") };
    let free: MatchDataFree = unsafe { symbol(library, b"pcre2_match_data_free_8\0") };
    let match_fn: Match = unsafe { symbol(library, b"pcre2_match_8\0") };
    let data = unsafe { create(code.code, ptr::null_mut()) };
    let rc = unsafe {
        match_fn(
            code.code,
            subject.as_ptr(),
            subject.len(),
            0,
            0,
            data,
            ptr::null_mut(),
        )
    };
    assert!(rc >= 0, "match failed: {rc}");
    (code, data, free)
}

#[test]
fn substring_api() {
    unsafe {
        let (c, rust) = libraries();
        let pattern = b"(?<first>a)(?<optional>b)?(?<last>c)";
        let subject = b"ac";
        let mut all = Vec::new();
        for library in [&c, &rust] {
            let (_code, data, free_data) = matched_data(library, pattern, 0, subject);
            type Length = unsafe extern "C" fn(*mut MatchData, u32, *mut usize) -> c_int;
            type Copy = unsafe extern "C" fn(*mut MatchData, u32, *mut u8, *mut usize) -> c_int;
            type Get = unsafe extern "C" fn(*mut MatchData, u32, *mut *mut u8, *mut usize) -> c_int;
            let length: Length = symbol(library, b"pcre2_substring_length_bynumber_8\0");
            let copy: Copy = symbol(library, b"pcre2_substring_copy_bynumber_8\0");
            let get: Get = symbol(library, b"pcre2_substring_get_bynumber_8\0");
            let free_string: unsafe extern "C" fn(*mut u8) =
                symbol(library, b"pcre2_substring_free_8\0");
            let get_mark: unsafe extern "C" fn(*mut MatchData) -> *const u8 =
                symbol(library, b"pcre2_get_mark_8\0");
            assert!(get_mark(data).is_null());
            let mut normalized = Vec::new();
            for number in 0..=5 {
                let mut len = 777;
                let lrc = length(data, number, &mut len);
                let mut buffer = [0xa5; 32];
                let mut cap = buffer.len();
                let crc = copy(data, number, buffer.as_mut_ptr(), &mut cap);
                let mut allocated = ptr::null_mut();
                let mut allocated_len = 0;
                let grc = get(data, number, &mut allocated, &mut allocated_len);
                let bytes = if grc == 0 {
                    let value = std::slice::from_raw_parts(allocated, allocated_len).to_vec();
                    free_string(allocated);
                    value
                } else {
                    Vec::new()
                };
                normalized.push((lrc, len, crc, cap, buffer, grc, allocated_len, bytes));
            }
            type NameScan = unsafe extern "C" fn(
                *const Code,
                *const u8,
                *mut *const u8,
                *mut *const u8,
            ) -> c_int;
            let number_from_name: unsafe extern "C" fn(*const Code, *const u8) -> c_int =
                symbol(library, b"pcre2_substring_number_from_name_8\0");
            let scan: NameScan = symbol(library, b"pcre2_substring_nametable_scan_8\0");
            for name in [
                b"first\0".as_slice(),
                b"last\0".as_slice(),
                b"missing\0".as_slice(),
            ] {
                normalized.push((
                    number_from_name(_code.code, name.as_ptr()),
                    0,
                    scan(_code.code, name.as_ptr(), ptr::null_mut(), ptr::null_mut()),
                    0,
                    [0; 32],
                    0,
                    0,
                    Vec::new(),
                ));
            }

            type NamedLength = unsafe extern "C" fn(*mut MatchData, *const u8, *mut usize) -> c_int;
            type NamedCopy =
                unsafe extern "C" fn(*mut MatchData, *const u8, *mut u8, *mut usize) -> c_int;
            type NamedGet =
                unsafe extern "C" fn(*mut MatchData, *const u8, *mut *mut u8, *mut usize) -> c_int;
            let named_length: NamedLength = symbol(library, b"pcre2_substring_length_byname_8\0");
            let named_copy: NamedCopy = symbol(library, b"pcre2_substring_copy_byname_8\0");
            let named_get: NamedGet = symbol(library, b"pcre2_substring_get_byname_8\0");
            for name in [b"first\0".as_slice(), b"optional\0".as_slice()] {
                let mut len = 99;
                let lrc = named_length(data, name.as_ptr(), &mut len);
                let mut buffer = [0xa5; 32];
                let mut cap = buffer.len();
                let crc = named_copy(data, name.as_ptr(), buffer.as_mut_ptr(), &mut cap);
                let mut allocated = ptr::null_mut();
                let mut allocated_len = 0;
                let grc = named_get(data, name.as_ptr(), &mut allocated, &mut allocated_len);
                let bytes = if grc == 0 {
                    let value = std::slice::from_raw_parts(allocated, allocated_len).to_vec();
                    free_string(allocated);
                    value
                } else {
                    Vec::new()
                };
                normalized.push((lrc, len, crc, cap, buffer, grc, allocated_len, bytes));
            }

            type ListGet =
                unsafe extern "C" fn(*mut MatchData, *mut *mut *mut u8, *mut *mut usize) -> c_int;
            let list_get: ListGet = symbol(library, b"pcre2_substring_list_get_8\0");
            let list_free: unsafe extern "C" fn(*mut *mut u8) =
                symbol(library, b"pcre2_substring_list_free_8\0");
            let mut list = ptr::null_mut();
            let mut lengths = ptr::null_mut();
            assert_eq!(list_get(data, &mut list, &mut lengths), 0);
            let listed = (0..4)
                .map(|index| {
                    let len = *lengths.add(index);
                    std::slice::from_raw_parts(*list.add(index), len).to_vec()
                })
                .collect::<Vec<_>>();
            normalized.push((0, 0, 0, 0, [0; 32], 0, 0, listed.concat()));
            list_free(list);
            free_data(data);
            all.push(normalized);
        }
        assert_eq!(all[0], all[1]);
    }
}

#[test]
fn serialize_roundtrip() {
    unsafe {
        let (c, rust) = libraries();
        let mut results = Vec::new();
        for library in [&c, &rust] {
            let a = compile(library, b"a+", 2, 0, ptr::null_mut()).unwrap();
            let b = compile(library, b"(?<x>b)", 7, 0, ptr::null_mut()).unwrap();
            type Encode = unsafe extern "C" fn(
                *const *const Code,
                i32,
                *mut *mut u8,
                *mut usize,
                *mut Context,
            ) -> i32;
            type Decode = unsafe extern "C" fn(*mut *mut Code, i32, *const u8, *mut Context) -> i32;
            let encode: Encode = symbol(library, b"pcre2_serialize_encode_8\0");
            let decode: Decode = symbol(library, b"pcre2_serialize_decode_8\0");
            let count: unsafe extern "C" fn(*const u8) -> i32 =
                symbol(library, b"pcre2_serialize_get_number_of_codes_8\0");
            let serialized_free: unsafe extern "C" fn(*mut u8) =
                symbol(library, b"pcre2_serialize_free_8\0");
            let code_free: CodeFree = symbol(library, b"pcre2_code_free_8\0");
            let codes = [a.code as *const Code, b.code as *const Code];
            let mut bytes = ptr::null_mut();
            let mut size = 0;
            let rc = encode(
                codes.as_ptr(),
                codes.len() as i32,
                &mut bytes,
                &mut size,
                ptr::null_mut(),
            );
            assert_eq!(rc, 2);
            let encoded = std::slice::from_raw_parts(bytes, size).to_vec();
            let number = count(bytes);
            let mut decoded = [ptr::null_mut(); 2];
            let drc = decode(decoded.as_mut_ptr(), 2, bytes, ptr::null_mut());
            for code in decoded {
                code_free(code);
            }
            serialized_free(bytes);
            results.push((size, encoded, number, drc));
        }
        assert_eq!(results[0], results[1]);
    }
}

#[test]
fn substitute_modes() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0x55b5_71a7_e123_9876;
        for _ in 0..128 {
            let needle = randomized_ascii(&mut seed, 5);
            let replacement = randomized_ascii(&mut seed, 8);
            let mut subject = randomized_ascii(&mut seed, 24);
            let pos = rng_next(&mut seed) as usize % (subject.len() + 1);
            subject.splice(pos..pos, needle.iter().copied());
            for options in [0, SUBSTITUTE_GLOBAL, SUBSTITUTE_LITERAL] {
                let mut values = Vec::new();
                for library in [&c, &rust] {
                    let code = compile(library, &needle, needle.len(), 0, ptr::null_mut()).unwrap();
                    type Substitute = unsafe extern "C" fn(
                        *const Code,
                        *const u8,
                        usize,
                        usize,
                        u32,
                        *mut MatchData,
                        *mut Context,
                        *const u8,
                        usize,
                        *mut u8,
                        *mut usize,
                    ) -> c_int;
                    let substitute: Substitute = symbol(library, b"pcre2_substitute_8\0");
                    let mut output = [0u8; 256];
                    let mut length = output.len();
                    let rc = substitute(
                        code.code,
                        subject.as_ptr(),
                        subject.len(),
                        0,
                        options,
                        ptr::null_mut(),
                        ptr::null_mut(),
                        replacement.as_ptr(),
                        replacement.len(),
                        output.as_mut_ptr(),
                        &mut length,
                    );
                    values.push((rc, length, output[..length.min(output.len())].to_vec()));
                }
                assert_eq!(values[0], values[1]);
            }
        }
    }
}

#[test]
fn convert_modes() {
    unsafe {
        let (c, rust) = libraries();
        let cases: &[(&[u8], u32)] = &[
            (b"a*b?[0-9]", CONVERT_GLOB),
            (b"^a\\(b\\)\\{1,2\\}$", CONVERT_POSIX_BASIC),
            (b"^(a|b)+$", CONVERT_POSIX_EXTENDED),
            ("\u{e9}*".as_bytes(), CONVERT_GLOB | CONVERT_UTF),
        ];
        for (pattern, options) in cases {
            let mut values = Vec::new();
            for library in [&c, &rust] {
                type Convert = unsafe extern "C" fn(
                    *const u8,
                    usize,
                    u32,
                    *mut *mut u8,
                    *mut usize,
                    *mut Context,
                ) -> c_int;
                let convert: Convert = symbol(library, b"pcre2_pattern_convert_8\0");
                let converted_free: unsafe extern "C" fn(*mut u8) =
                    symbol(library, b"pcre2_converted_pattern_free_8\0");
                let mut output = ptr::null_mut();
                let mut length = 0;
                let rc = convert(
                    pattern.as_ptr(),
                    pattern.len(),
                    *options,
                    &mut output,
                    &mut length,
                    ptr::null_mut(),
                );
                let bytes = if rc == 0 {
                    let result = std::slice::from_raw_parts(output, length).to_vec();
                    converted_free(output);
                    result
                } else {
                    Vec::new()
                };
                values.push((rc, length, bytes));
            }
            assert_eq!(values[0], values[1]);
        }
    }
}

#[test]
fn jit_disabled() {
    unsafe {
        let (c, rust) = libraries();
        let mut values = Vec::new();
        for library in [&c, &rust] {
            let code = compile(library, b"a", 1, 0, ptr::null_mut()).unwrap();
            let jit_compile: unsafe extern "C" fn(*mut Code, u32) -> c_int =
                symbol(library, b"pcre2_jit_compile_8\0");
            let mut compile_results = Vec::new();
            for options in [
                0,
                JIT_COMPLETE,
                JIT_PARTIAL_SOFT,
                JIT_PARTIAL_HARD,
                JIT_INVALID_UTF,
                JIT_TEST_ALLOC,
                JIT_TEST_ALLOC | JIT_COMPLETE,
            ] {
                compile_results.push(jit_compile(code.code, options));
            }
            let stack_create: unsafe extern "C" fn(usize, usize, *mut Context) -> *mut c_void =
                symbol(library, b"pcre2_jit_stack_create_8\0");
            let stack_free: unsafe extern "C" fn(*mut c_void) =
                symbol(library, b"pcre2_jit_stack_free_8\0");
            let stack = stack_create(32 * 1024, 512 * 1024, ptr::null_mut());
            stack_free(stack);
            let free_unused: unsafe extern "C" fn(*mut Context) =
                symbol(library, b"pcre2_jit_free_unused_memory_8\0");
            free_unused(ptr::null_mut());
            type ContextCreate = unsafe extern "C" fn(*mut Context) -> *mut Context;
            type ContextFree = unsafe extern "C" fn(*mut Context);
            let create_context: ContextCreate = symbol(library, b"pcre2_match_context_create_8\0");
            let free_context: ContextFree = symbol(library, b"pcre2_match_context_free_8\0");
            let assign: unsafe extern "C" fn(
                *mut Context,
                Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
                *mut c_void,
            ) = symbol(library, b"pcre2_jit_stack_assign_8\0");
            let match_context = create_context(ptr::null_mut());
            assign(match_context, None, ptr::null_mut());

            let create_data: MatchDataFromPattern =
                symbol(library, b"pcre2_match_data_create_from_pattern_8\0");
            let free_data: MatchDataFree = symbol(library, b"pcre2_match_data_free_8\0");
            let jit_match: Match = symbol(library, b"pcre2_jit_match_8\0");
            let data = create_data(code.code, ptr::null_mut());
            let match_result = jit_match(code.code, b"a".as_ptr(), 1, 0, 0, data, match_context);
            free_data(data);
            free_context(match_context);
            values.push((compile_results, stack.is_null(), match_result));
        }
        assert_eq!(values[0], values[1]);
    }
}

#[test]
fn tables_and_allocators() {
    unsafe {
        let (c, rust) = libraries();
        let mut values = Vec::new();
        for library in [&c, &rust] {
            let make: unsafe extern "C" fn(*mut Context) -> *const u8 =
                symbol(library, b"pcre2_maketables_8\0");
            let free_tables: unsafe extern "C" fn(*mut Context, *const u8) =
                symbol(library, b"pcre2_maketables_free_8\0");
            let tables = make(ptr::null_mut());
            assert!(!tables.is_null());
            values.push(std::slice::from_raw_parts(tables, 1088).to_vec());
            type ContextCreate = unsafe extern "C" fn(*mut Context) -> *mut Context;
            type ContextFree = unsafe extern "C" fn(*mut Context);
            let create: ContextCreate = symbol(library, b"pcre2_compile_context_create_8\0");
            let context_free: ContextFree = symbol(library, b"pcre2_compile_context_free_8\0");
            let set_tables: unsafe extern "C" fn(*mut Context, *const u8) -> c_int =
                symbol(library, b"pcre2_set_character_tables_8\0");
            let context = create(ptr::null_mut());
            assert_eq!(set_tables(context, tables), 0);
            context_free(context);
            free_tables(ptr::null_mut(), tables);
        }
        assert_eq!(values[0], values[1]);
    }
}

#[test]
fn error_surface() {
    unsafe {
        let (c, rust) = libraries();
        type Config = unsafe extern "C" fn(u32, *mut c_void) -> c_int;
        for library in [&c, &rust] {
            let config: Config = symbol(library, b"pcre2_config_8\0");
            assert_eq!(config(u32::MAX, ptr::null_mut()), ERROR_BADOPTION);
            let mut value = 0u32;
            assert_eq!(
                config(u32::MAX, (&mut value as *mut u32).cast()),
                ERROR_BADOPTION
            );
            assert_eq!(config(2, ptr::null_mut()), ERROR_BADOPTION);

            type ContextCreate = unsafe extern "C" fn(*mut Context) -> *mut Context;
            type ContextFree = unsafe extern "C" fn(*mut Context);
            type SetU32 = unsafe extern "C" fn(*mut Context, u32) -> c_int;
            let cc_create: ContextCreate = symbol(library, b"pcre2_compile_context_create_8\0");
            let cc_free: ContextFree = symbol(library, b"pcre2_compile_context_free_8\0");
            let cc = cc_create(ptr::null_mut());
            let set_bsr: SetU32 = symbol(library, b"pcre2_set_bsr_8\0");
            let set_newline: SetU32 = symbol(library, b"pcre2_set_newline_8\0");
            let set_optimize: SetU32 = symbol(library, b"pcre2_set_optimize_8\0");
            for invalid in [0, 3, u32::MAX] {
                assert_eq!(set_bsr(cc, invalid), ERROR_BADDATA);
            }
            for invalid in [0, 7, u32::MAX] {
                assert_eq!(set_newline(cc, invalid), ERROR_BADDATA);
            }
            assert_eq!(set_optimize(ptr::null_mut(), 0), ERROR_NULL);
            for invalid in [2, 63, 70, u32::MAX] {
                assert_eq!(set_optimize(cc, invalid), ERROR_BADOPTION);
            }
            cc_free(cc);

            let xc_create: ContextCreate = symbol(library, b"pcre2_convert_context_create_8\0");
            let xc_free: ContextFree = symbol(library, b"pcre2_convert_context_free_8\0");
            let xc = xc_create(ptr::null_mut());
            let set_separator: SetU32 = symbol(library, b"pcre2_set_glob_separator_8\0");
            let set_escape: SetU32 = symbol(library, b"pcre2_set_glob_escape_8\0");
            assert_eq!(set_separator(xc, b'x' as u32), ERROR_BADDATA);
            assert_eq!(set_escape(xc, 256), ERROR_BADDATA);
            assert_eq!(set_escape(xc, b'A' as u32), ERROR_BADDATA);
            xc_free(xc);

            let compile_fn: Compile = symbol(library, b"pcre2_compile_8\0");
            let mut error = 999;
            let mut offset = 999;
            assert!(
                compile_fn(
                    b"a".as_ptr(),
                    1,
                    0,
                    ptr::null_mut(),
                    &mut offset,
                    ptr::null_mut()
                )
                .is_null()
            );
            assert_eq!(offset, 0);
            assert!(
                compile_fn(
                    b"a".as_ptr(),
                    1,
                    0,
                    &mut error,
                    ptr::null_mut(),
                    ptr::null_mut()
                )
                .is_null()
            );
            assert_eq!(error, 220);
            assert!(
                compile_fn(ptr::null(), 1, 0, &mut error, &mut offset, ptr::null_mut()).is_null()
            );
            assert_eq!(error, 116);
            assert!(
                compile_fn(
                    b"a".as_ptr(),
                    1,
                    0x1000_0000,
                    &mut error,
                    &mut offset,
                    ptr::null_mut()
                )
                .is_null()
            );
            assert_eq!(error, 117);

            for (pattern, expected) in [
                (b"\\".as_slice(), 101),
                (b"\\j".as_slice(), 103),
                (b"a{2,1}".as_slice(), 104),
                (b"[".as_slice(), 106),
                (b"[z-a]".as_slice(), 108),
                (b"(?z)".as_slice(), 111),
                (b"(".as_slice(), 114),
                (b")".as_slice(), 122),
                (b"(?<x>a)(?<x>b)".as_slice(), 143),
                (b"(?<1>a)".as_slice(), 144),
                (b"\\p{NotAProperty}".as_slice(), 147),
            ] {
                let result = compile(library, pattern, pattern.len(), 0, ptr::null_mut());
                match result {
                    Err((error, _)) => assert_eq!(error, expected, "{pattern:?}"),
                    Ok(_) => panic!("invalid pattern compiled: {pattern:?}"),
                }
            }

            type PatternInfo = unsafe extern "C" fn(*const Code, u32, *mut c_void) -> c_int;
            let info: PatternInfo = symbol(library, b"pcre2_pattern_info_8\0");
            let mut out = 0usize;
            assert_eq!(
                info(ptr::null(), 0, (&mut out as *mut usize).cast()),
                ERROR_NULL
            );
            let code = compile(library, b"a", 1, 0, ptr::null_mut()).unwrap();
            assert_eq!(
                info(code.code, u32::MAX, (&mut out as *mut usize).cast()),
                ERROR_BADOPTION
            );

            let create: MatchDataFromPattern =
                symbol(library, b"pcre2_match_data_create_from_pattern_8\0");
            let create_count: MatchDataCreate = symbol(library, b"pcre2_match_data_create_8\0");
            let free_data: MatchDataFree = symbol(library, b"pcre2_match_data_free_8\0");
            let match_fn: Match = symbol(library, b"pcre2_match_8\0");
            assert!(create(ptr::null(), ptr::null_mut()).is_null());
            let data = create(code.code, ptr::null_mut());
            assert_eq!(
                match_fn(ptr::null(), b"a".as_ptr(), 1, 0, 0, data, ptr::null_mut()),
                ERROR_NULL
            );
            assert_eq!(
                match_fn(code.code, ptr::null(), 1, 0, 0, data, ptr::null_mut()),
                ERROR_NULL
            );
            assert_eq!(
                match_fn(
                    code.code,
                    b"a".as_ptr(),
                    1,
                    0,
                    DFA_RESTART,
                    data,
                    ptr::null_mut()
                ),
                ERROR_BADOPTION
            );
            assert_eq!(
                match_fn(code.code, b"a".as_ptr(), 1, 2, 0, data, ptr::null_mut()),
                ERROR_BADOFFSET
            );
            assert_eq!(
                match_fn(
                    code.code,
                    b"a".as_ptr(),
                    1,
                    0,
                    PARTIAL_SOFT | ENDANCHORED,
                    data,
                    ptr::null_mut()
                ),
                ERROR_BADOPTION
            );

            let dfa: DfaMatch = symbol(library, b"pcre2_dfa_match_8\0");
            let mut workspace = [0; 64];
            assert_eq!(
                dfa(
                    code.code,
                    b"a".as_ptr(),
                    1,
                    0,
                    0,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    workspace.as_mut_ptr(),
                    workspace.len()
                ),
                ERROR_NULL
            );
            assert_eq!(
                dfa(
                    code.code,
                    b"a".as_ptr(),
                    1,
                    0,
                    0,
                    data,
                    ptr::null_mut(),
                    workspace.as_mut_ptr(),
                    19
                ),
                ERROR_DFA_WSSIZE
            );
            assert_eq!(
                dfa(
                    code.code,
                    b"a".as_ptr(),
                    1,
                    0,
                    DFA_RESTART,
                    data,
                    ptr::null_mut(),
                    workspace.as_mut_ptr(),
                    workspace.len()
                ),
                ERROR_DFA_BADRESTART
            );

            assert_eq!(
                match_fn(code.code, b"a".as_ptr(), 1, 0, 0, data, ptr::null_mut()),
                1
            );
            type Length = unsafe extern "C" fn(*mut MatchData, u32, *mut usize) -> c_int;
            type Copy = unsafe extern "C" fn(*mut MatchData, u32, *mut u8, *mut usize) -> c_int;
            let length: Length = symbol(library, b"pcre2_substring_length_bynumber_8\0");
            let copy: Copy = symbol(library, b"pcre2_substring_copy_bynumber_8\0");
            assert_eq!(length(data, 1, &mut out), ERROR_NOSUBSTRING);
            let tiny = create_count(1, ptr::null_mut());
            assert!(!tiny.is_null());
            let mut cap = 1;
            let mut byte = 0;
            assert_eq!(copy(data, 0, &mut byte, &mut cap), ERROR_NOMEMORY);
            free_data(tiny);
            free_data(data);

            type Encode = unsafe extern "C" fn(
                *const *const Code,
                i32,
                *mut *mut u8,
                *mut usize,
                *mut Context,
            ) -> i32;
            type Decode = unsafe extern "C" fn(*mut *mut Code, i32, *const u8, *mut Context) -> i32;
            let encode: Encode = symbol(library, b"pcre2_serialize_encode_8\0");
            let decode: Decode = symbol(library, b"pcre2_serialize_decode_8\0");
            let count: unsafe extern "C" fn(*const u8) -> i32 =
                symbol(library, b"pcre2_serialize_get_number_of_codes_8\0");
            let mut bytes = ptr::null_mut();
            let mut size = 0;
            assert_eq!(
                encode(ptr::null(), 1, &mut bytes, &mut size, ptr::null_mut()),
                ERROR_NULL
            );
            let codes = [code.code as *const Code];
            assert_eq!(
                encode(codes.as_ptr(), 0, &mut bytes, &mut size, ptr::null_mut()),
                ERROR_BADDATA
            );
            let mut decoded = ptr::null_mut();
            assert_eq!(
                decode(&mut decoded, 1, ptr::null(), ptr::null_mut()),
                ERROR_NULL
            );
            assert_eq!(count(ptr::null()), ERROR_NULL);
            let bad = [0u8; 64];
            assert_eq!(count(bad.as_ptr()), ERROR_BADMAGIC);

            type Convert = unsafe extern "C" fn(
                *const u8,
                usize,
                u32,
                *mut *mut u8,
                *mut usize,
                *mut Context,
            ) -> c_int;
            let convert: Convert = symbol(library, b"pcre2_pattern_convert_8\0");
            let mut converted = ptr::null_mut();
            assert_eq!(
                convert(
                    ptr::null(),
                    1,
                    CONVERT_GLOB,
                    &mut converted,
                    &mut size,
                    ptr::null_mut()
                ),
                ERROR_NULL
            );
            assert_eq!(
                convert(
                    b"a".as_ptr(),
                    1,
                    0,
                    &mut converted,
                    &mut size,
                    ptr::null_mut()
                ),
                ERROR_BADOPTION
            );
            assert_eq!(
                convert(
                    b"a".as_ptr(),
                    1,
                    CONVERT_GLOB | CONVERT_POSIX_BASIC,
                    &mut converted,
                    &mut size,
                    ptr::null_mut()
                ),
                ERROR_BADOPTION
            );

            let jit_compile: unsafe extern "C" fn(*mut Code, u32) -> c_int =
                symbol(library, b"pcre2_jit_compile_8\0");
            assert_eq!(
                jit_compile(code.code, JIT_TEST_ALLOC | JIT_COMPLETE),
                ERROR_JIT_BADOPTION
            );
            assert_eq!(
                jit_compile(code.code, JIT_TEST_ALLOC),
                ERROR_JIT_UNSUPPORTED
            );
            assert_eq!(jit_compile(ptr::null_mut(), 0), ERROR_NULL);
            assert_eq!(jit_compile(code.code, u32::MAX), ERROR_JIT_BADOPTION);
            assert_eq!(jit_compile(code.code, JIT_COMPLETE), ERROR_JIT_BADOPTION);
        }

        let _ = (
            ERROR_MIXEDTABLES,
            ERROR_BADMODE,
            ERROR_BADUTFOFFSET,
            ERROR_DFA_UFUNC,
            ERROR_NOUNIQUESUBSTRING,
            ERROR_UNAVAILABLE,
            ERROR_UNSET,
            ERROR_BADOFFSETLIMIT,
            ERROR_BADSERIALIZEDDATA,
            MATCH_INVALID_UTF,
            USE_OFFSET_LIMIT,
            CASELESS,
            NO_UTF_CHECK,
            UCP,
            ZERO_TERMINATED,
            UNSET,
            SUBSTITUTE_EXTENDED,
            SUBSTITUTE_UNSET_EMPTY,
            SUBSTITUTE_UNKNOWN_UNSET,
            SUBSTITUTE_OVERFLOW_LENGTH,
            SUBSTITUTE_MATCHED,
            SUBSTITUTE_REPLACEMENT_ONLY,
            CONVERT_NO_UTF_CHECK,
            failing_malloc as unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void,
        );
    }
}
