use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

type Code = c_void;
type Context = c_void;
type MatchData = c_void;
type JitStack = c_void;

type Config = unsafe extern "C" fn(u32, *mut c_void) -> c_int;
type ContextCreate = unsafe extern "C" fn(*mut Context) -> *mut Context;
type ContextCopy = unsafe extern "C" fn(*mut Context) -> *mut Context;
type ContextFree = unsafe extern "C" fn(*mut Context);
type GeneralCreate = unsafe extern "C" fn(
    Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    *mut c_void,
) -> *mut Context;
type GeneralCopy = unsafe extern "C" fn(*mut Context) -> *mut Context;
type GeneralFree = unsafe extern "C" fn(*mut Context);
type SetterU32 = unsafe extern "C" fn(*mut Context, u32) -> c_int;
type SetterUsize = unsafe extern "C" fn(*mut Context, usize) -> c_int;
type SetterPtr = unsafe extern "C" fn(*mut Context, *const u8) -> c_int;
type SetterCallout = unsafe extern "C" fn(
    *mut Context,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int>,
    *mut c_void,
) -> c_int;
type SetterSubCase = unsafe extern "C" fn(
    *mut Context,
    Option<unsafe extern "C" fn(*const u8, usize, *mut u8, usize, c_int, *mut c_void) -> usize>,
    *mut c_void,
) -> c_int;
type SetterRecursionMemory = unsafe extern "C" fn(
    *mut Context,
    Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    *mut c_void,
) -> c_int;
type SetterGuard = unsafe extern "C" fn(
    *mut Context,
    Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>,
    *mut c_void,
) -> c_int;

type Compile =
    unsafe extern "C" fn(*const u8, usize, u32, *mut c_int, *mut usize, *mut Context) -> *mut Code;
type CodeFree = unsafe extern "C" fn(*mut Code);
type CodeCopy = unsafe extern "C" fn(*const Code) -> *mut Code;
type PatternInfo = unsafe extern "C" fn(*const Code, u32, *mut c_void) -> c_int;
type CalloutEnumerate = unsafe extern "C" fn(
    *const Code,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int>,
    *mut c_void,
) -> c_int;

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
type GetConstU8 = unsafe extern "C" fn(*mut MatchData) -> *const u8;
type GetUsize = unsafe extern "C" fn(*mut MatchData) -> usize;
type GetU32 = unsafe extern "C" fn(*mut MatchData) -> u32;
type GetOvector = unsafe extern "C" fn(*mut MatchData) -> *mut usize;
type NextMatch = unsafe extern "C" fn(*mut MatchData, *mut usize, *mut u32) -> c_int;

type SubstringCopyName =
    unsafe extern "C" fn(*mut MatchData, *const u8, *mut u8, *mut usize) -> c_int;
type SubstringCopyNumber = unsafe extern "C" fn(*mut MatchData, u32, *mut u8, *mut usize) -> c_int;
type SubstringGetName =
    unsafe extern "C" fn(*mut MatchData, *const u8, *mut *mut u8, *mut usize) -> c_int;
type SubstringGetNumber =
    unsafe extern "C" fn(*mut MatchData, u32, *mut *mut u8, *mut usize) -> c_int;
type SubstringLengthName = unsafe extern "C" fn(*mut MatchData, *const u8, *mut usize) -> c_int;
type SubstringLengthNumber = unsafe extern "C" fn(*mut MatchData, u32, *mut usize) -> c_int;
type SubstringFree = unsafe extern "C" fn(*mut u8);
type SubstringNumberFromName = unsafe extern "C" fn(*const Code, *const u8) -> c_int;
type SubstringNametableScan =
    unsafe extern "C" fn(*const Code, *const u8, *mut *const u8, *mut *const u8) -> c_int;
type SubstringListGet =
    unsafe extern "C" fn(*mut MatchData, *mut *mut *mut u8, *mut *mut usize) -> c_int;
type SubstringListFree = unsafe extern "C" fn(*mut *mut u8);

type SerializeEncode =
    unsafe extern "C" fn(*const *const Code, i32, *mut *mut u8, *mut usize, *mut Context) -> i32;
type SerializeDecode = unsafe extern "C" fn(*mut *mut Code, i32, *const u8, *mut Context) -> i32;
type SerializeCount = unsafe extern "C" fn(*const u8) -> i32;
type SerializeFree = unsafe extern "C" fn(*mut u8);

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
type PatternConvert =
    unsafe extern "C" fn(*const u8, usize, u32, *mut *mut u8, *mut usize, *mut Context) -> c_int;
type ConvertedFree = unsafe extern "C" fn(*mut u8);
type ErrorMessage = unsafe extern "C" fn(c_int, *mut u8, usize) -> c_int;
type MakeTables = unsafe extern "C" fn(*mut Context) -> *const u8;
type MakeTablesFree = unsafe extern "C" fn(*mut Context, *const u8);

type JitCompile = unsafe extern "C" fn(*mut Code, u32) -> c_int;
type JitMatch = Match;
type JitFreeUnused = unsafe extern "C" fn(*mut Context);
type JitStackCreate = unsafe extern "C" fn(usize, usize, *mut Context) -> *mut JitStack;
type JitStackFree = unsafe extern "C" fn(*mut JitStack);
type JitStackAssign = unsafe extern "C" fn(
    *mut Context,
    Option<unsafe extern "C" fn(*mut c_void) -> *mut JitStack>,
    *mut c_void,
);

struct Api {
    library: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        Self {
            library: unsafe { Library::new(path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        unsafe {
            *self
                .library
                .get::<T>(name)
                .unwrap_or_else(|error| panic!("missing {:?}: {error}", name))
        }
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn apis() -> (Api, Api) {
    let root = root();
    let c_path = root.join("c_src/build/libpcre2.so");
    let rust_debug = root.join("translation/target/debug/libpcre2.so");
    let rust_release = root.join("translation/target/release/libpcre2.so");
    let rust_path = if rust_debug.exists() {
        rust_debug
    } else {
        rust_release
    };
    assert!(c_path.exists(), "C shared library not built");
    assert!(rust_path.exists(), "Rust shared library not built");
    unsafe { (Api::open(&c_path), Api::open(&rust_path)) }
}

fn bytes_ptr(bytes: &[u8]) -> *const u8 {
    if bytes.is_empty() {
        ptr::null()
    } else {
        bytes.as_ptr()
    }
}

unsafe fn compile_one(
    api: &Api,
    pattern: &[u8],
    options: u32,
    context: *mut Context,
) -> (*mut Code, c_int, usize) {
    let compile: Compile = unsafe { api.symbol(b"pcre2_compile_8\0") };
    let mut error = 0;
    let mut offset = usize::MAX;
    let code = unsafe {
        compile(
            bytes_ptr(pattern),
            pattern.len(),
            options,
            &mut error,
            &mut offset,
            context,
        )
    };
    (code, error, offset)
}

unsafe fn free_code(api: &Api, code: *mut Code) {
    let free: CodeFree = unsafe { api.symbol(b"pcre2_code_free_8\0") };
    unsafe { free(code) };
}

unsafe fn compare_compile(
    c: &Api,
    rust: &Api,
    pattern: &[u8],
    options: u32,
) -> Option<(*mut Code, *mut Code)> {
    let (c_code, c_error, c_offset) = unsafe { compile_one(c, pattern, options, ptr::null_mut()) };
    let (r_code, r_error, r_offset) =
        unsafe { compile_one(rust, pattern, options, ptr::null_mut()) };
    assert_eq!(
        (c_code.is_null(), c_error, c_offset),
        (r_code.is_null(), r_error, r_offset),
        "compile divergence for pattern {:?}, options {options:#x}",
        String::from_utf8_lossy(pattern)
    );
    if c_code.is_null() {
        None
    } else {
        Some((c_code, r_code))
    }
}

unsafe fn ovector(api: &Api, data: *mut MatchData, rc: c_int) -> Vec<usize> {
    if rc <= 0 {
        return Vec::new();
    }
    let get: GetOvector = unsafe { api.symbol(b"pcre2_get_ovector_pointer_8\0") };
    unsafe { std::slice::from_raw_parts(get(data), rc as usize * 2).to_vec() }
}

fn dynamic_symbols(path: &Path) -> Vec<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(output.status.success());
    let mut symbols: Vec<_> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().last().map(str::to_owned))
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols
}

#[test]
fn exported_symbol_surface_is_identical() {
    let root = root();
    let c = dynamic_symbols(&root.join("c_src/build/libpcre2.so"));
    let rust_path = if root.join("translation/target/debug/libpcre2.so").exists() {
        root.join("translation/target/debug/libpcre2.so")
    } else {
        root.join("translation/target/release/libpcre2.so")
    };
    let rust = dynamic_symbols(&rust_path);
    assert_eq!(c, rust);
}

#[test]
fn config_context_and_allocator_surfaces_match() {
    let (c, rust) = apis();
    unsafe {
        let c_config: Config = c.symbol(b"pcre2_config_8\0");
        let r_config: Config = rust.symbol(b"pcre2_config_8\0");
        for what in 0..=18u32 {
            let mut cb = [0xa5u8; 256];
            let mut rb = [0xa5u8; 256];
            let cr = c_config(what, cb.as_mut_ptr().cast());
            let rr = r_config(what, rb.as_mut_ptr().cast());
            assert_eq!(cr, rr, "config selector {what}");
            assert_eq!(cb, rb, "config bytes selector {what}");
        }
        let mut cb = [0u8; 32];
        let mut rb = [0u8; 32];
        assert_eq!(
            c_config(u32::MAX, cb.as_mut_ptr().cast()),
            r_config(u32::MAX, rb.as_mut_ptr().cast())
        );

        let c_general_create: GeneralCreate = c.symbol(b"pcre2_general_context_create_8\0");
        let r_general_create: GeneralCreate = rust.symbol(b"pcre2_general_context_create_8\0");
        let c_general_copy: GeneralCopy = c.symbol(b"pcre2_general_context_copy_8\0");
        let r_general_copy: GeneralCopy = rust.symbol(b"pcre2_general_context_copy_8\0");
        let c_general_free: GeneralFree = c.symbol(b"pcre2_general_context_free_8\0");
        let r_general_free: GeneralFree = rust.symbol(b"pcre2_general_context_free_8\0");
        let cg = c_general_create(None, None, ptr::null_mut());
        let rg = r_general_create(None, None, ptr::null_mut());
        assert_eq!(cg.is_null(), rg.is_null());
        let cgc = c_general_copy(cg);
        let rgc = r_general_copy(rg);
        assert_eq!(cgc.is_null(), rgc.is_null());

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
            let cc: ContextCreate = c.symbol(create_name);
            let rc: ContextCreate = rust.symbol(create_name);
            let cp: ContextCopy = c.symbol(copy_name);
            let rp: ContextCopy = rust.symbol(copy_name);
            let cf: ContextFree = c.symbol(free_name);
            let rf: ContextFree = rust.symbol(free_name);
            let cctx = cc(cg);
            let rctx = rc(rg);
            assert_eq!(cctx.is_null(), rctx.is_null());
            let ccopy = cp(cctx);
            let rcopy = rp(rctx);
            assert_eq!(ccopy.is_null(), rcopy.is_null());
            cf(ccopy);
            rf(rcopy);
            cf(cctx);
            rf(rctx);
        }

        let c_compile_create: ContextCreate = c.symbol(b"pcre2_compile_context_create_8\0");
        let r_compile_create: ContextCreate = rust.symbol(b"pcre2_compile_context_create_8\0");
        let c_compile_free: ContextFree = c.symbol(b"pcre2_compile_context_free_8\0");
        let r_compile_free: ContextFree = rust.symbol(b"pcre2_compile_context_free_8\0");
        let cc = c_compile_create(cg);
        let rc = r_compile_create(rg);

        for name in [
            b"pcre2_set_bsr_8\0".as_slice(),
            b"pcre2_set_compile_extra_options_8\0".as_slice(),
            b"pcre2_set_max_varlookbehind_8\0".as_slice(),
            b"pcre2_set_newline_8\0".as_slice(),
            b"pcre2_set_parens_nest_limit_8\0".as_slice(),
            b"pcre2_set_optimize_8\0".as_slice(),
        ] {
            let cf: SetterU32 = c.symbol(name);
            let rf: SetterU32 = rust.symbol(name);
            for value in [0, 1, 2, 3, 4, 5, 6, 7, 0x10000, u32::MAX] {
                assert_eq!(cf(cc, value), rf(rc, value), "{name:?} value {value}");
            }
        }
        for name in [
            b"pcre2_set_max_pattern_length_8\0".as_slice(),
            b"pcre2_set_max_pattern_compiled_length_8\0".as_slice(),
        ] {
            let cf: SetterUsize = c.symbol(name);
            let rf: SetterUsize = rust.symbol(name);
            for value in [0, 1, 32, 1 << 16, usize::MAX] {
                assert_eq!(cf(cc, value), rf(rc, value), "{name:?} value {value}");
            }
        }
        let c_tables: MakeTables = c.symbol(b"pcre2_maketables_8\0");
        let r_tables: MakeTables = rust.symbol(b"pcre2_maketables_8\0");
        let c_tables_free: MakeTablesFree = c.symbol(b"pcre2_maketables_free_8\0");
        let r_tables_free: MakeTablesFree = rust.symbol(b"pcre2_maketables_free_8\0");
        let ct = c_tables(cg);
        let rt = r_tables(rg);
        assert_eq!(ct.is_null(), rt.is_null());
        if !ct.is_null() {
            assert_eq!(
                std::slice::from_raw_parts(ct, 1088),
                std::slice::from_raw_parts(rt, 1088)
            );
            let cs: SetterPtr = c.symbol(b"pcre2_set_character_tables_8\0");
            let rs: SetterPtr = rust.symbol(b"pcre2_set_character_tables_8\0");
            assert_eq!(cs(cc, ct), rs(rc, rt));
            c_tables_free(cg, ct);
            r_tables_free(rg, rt);
        }
        let cg_set: SetterGuard = c.symbol(b"pcre2_set_compile_recursion_guard_8\0");
        let rg_set: SetterGuard = rust.symbol(b"pcre2_set_compile_recursion_guard_8\0");
        assert_eq!(
            cg_set(cc, None, ptr::null_mut()),
            rg_set(rc, None, ptr::null_mut())
        );
        c_compile_free(cc);
        r_compile_free(rc);

        let c_match_create: ContextCreate = c.symbol(b"pcre2_match_context_create_8\0");
        let r_match_create: ContextCreate = rust.symbol(b"pcre2_match_context_create_8\0");
        let c_match_free: ContextFree = c.symbol(b"pcre2_match_context_free_8\0");
        let r_match_free: ContextFree = rust.symbol(b"pcre2_match_context_free_8\0");
        let cm = c_match_create(cg);
        let rm = r_match_create(rg);
        for name in [
            b"pcre2_set_depth_limit_8\0".as_slice(),
            b"pcre2_set_heap_limit_8\0".as_slice(),
            b"pcre2_set_match_limit_8\0".as_slice(),
            b"pcre2_set_recursion_limit_8\0".as_slice(),
        ] {
            let cf: SetterU32 = c.symbol(name);
            let rf: SetterU32 = rust.symbol(name);
            for value in [0, 1, 100, u32::MAX] {
                assert_eq!(cf(cm, value), rf(rm, value));
            }
        }
        let co: SetterUsize = c.symbol(b"pcre2_set_offset_limit_8\0");
        let ro: SetterUsize = rust.symbol(b"pcre2_set_offset_limit_8\0");
        for value in [0, 1, 100, usize::MAX] {
            assert_eq!(co(cm, value), ro(rm, value));
        }
        for name in [
            b"pcre2_set_callout_8\0".as_slice(),
            b"pcre2_set_substitute_callout_8\0".as_slice(),
        ] {
            let cf: SetterCallout = c.symbol(name);
            let rf: SetterCallout = rust.symbol(name);
            assert_eq!(cf(cm, None, ptr::null_mut()), rf(rm, None, ptr::null_mut()));
        }
        let csc: SetterSubCase = c.symbol(b"pcre2_set_substitute_case_callout_8\0");
        let rsc: SetterSubCase = rust.symbol(b"pcre2_set_substitute_case_callout_8\0");
        assert_eq!(
            csc(cm, None, ptr::null_mut()),
            rsc(rm, None, ptr::null_mut())
        );
        let crm: SetterRecursionMemory = c.symbol(b"pcre2_set_recursion_memory_management_8\0");
        let rrm: SetterRecursionMemory = rust.symbol(b"pcre2_set_recursion_memory_management_8\0");
        assert_eq!(
            crm(cm, None, None, ptr::null_mut()),
            rrm(rm, None, None, ptr::null_mut())
        );
        c_match_free(cm);
        r_match_free(rm);

        let c_convert_create: ContextCreate = c.symbol(b"pcre2_convert_context_create_8\0");
        let r_convert_create: ContextCreate = rust.symbol(b"pcre2_convert_context_create_8\0");
        let c_convert_free: ContextFree = c.symbol(b"pcre2_convert_context_free_8\0");
        let r_convert_free: ContextFree = rust.symbol(b"pcre2_convert_context_free_8\0");
        let cv = c_convert_create(cg);
        let rv = r_convert_create(rg);
        for name in [
            b"pcre2_set_glob_escape_8\0".as_slice(),
            b"pcre2_set_glob_separator_8\0".as_slice(),
        ] {
            let cf: SetterU32 = c.symbol(name);
            let rf: SetterU32 = rust.symbol(name);
            for value in [0, b'\\' as u32, b'/' as u32, 0x100, u32::MAX] {
                assert_eq!(cf(cv, value), rf(rv, value));
            }
        }
        c_convert_free(cv);
        r_convert_free(rv);

        let c_stack_create: JitStackCreate = c.symbol(b"pcre2_jit_stack_create_8\0");
        let r_stack_create: JitStackCreate = rust.symbol(b"pcre2_jit_stack_create_8\0");
        let c_stack_free: JitStackFree = c.symbol(b"pcre2_jit_stack_free_8\0");
        let r_stack_free: JitStackFree = rust.symbol(b"pcre2_jit_stack_free_8\0");
        for (start, max) in [(1, 1), (32 * 1024, 512 * 1024), (1024, 512)] {
            let cs = c_stack_create(start, max, cg);
            let rs = r_stack_create(start, max, rg);
            assert_eq!(cs.is_null(), rs.is_null());
            c_stack_free(cs);
            r_stack_free(rs);
        }
        let c_jit_unused: JitFreeUnused = c.symbol(b"pcre2_jit_free_unused_memory_8\0");
        let r_jit_unused: JitFreeUnused = rust.symbol(b"pcre2_jit_free_unused_memory_8\0");
        let c_jit_assign: JitStackAssign = c.symbol(b"pcre2_jit_stack_assign_8\0");
        let r_jit_assign: JitStackAssign = rust.symbol(b"pcre2_jit_stack_assign_8\0");
        let cm = c_match_create(cg);
        let rm = r_match_create(rg);
        c_jit_assign(cm, None, ptr::null_mut());
        r_jit_assign(rm, None, ptr::null_mut());
        c_match_free(cm);
        r_match_free(rm);
        c_jit_unused(cg);
        r_jit_unused(rg);
        c_general_free(cgc);
        r_general_free(rgc);
        c_general_free(cg);
        r_general_free(rg);
    }
}

const COMPILE_OPTIONS: &[u32] = &[
    0, 0x00000001, 0x00000002, 0x00000004, 0x00000008, 0x00000010, 0x00000020, 0x00000040,
    0x00000080, 0x00000100, 0x00000200, 0x00000400, 0x00000800, 0x00001000, 0x00002000, 0x00004000,
    0x00008000, 0x00010000, 0x00020000, 0x00040000, 0x00080000, 0x00100000, 0x00200000, 0x00400000,
    0x00800000, 0x01000000, 0x02000000, 0x08000000, 0x20000000, 0x80000000,
];

const PATTERNS: &[&[u8]] = &[
    b"",
    b"a",
    b"abc",
    b"a*",
    b"a+?",
    b"^a.$",
    b"(a)(b)?",
    b"(?<word>[a-z]+)-(\\d+)",
    b"(?<x>a)|(?<x>b)",
    b"[a-z&&[^aeiou]]+",
    b"\\R",
    b"\\X",
    b"\\p{L}+",
    b"(?<=ab)c",
    b"(?>a|ab)c",
    b"(a|b)++",
    b"(?C1)a(?C\"tag\")",
    b"\\A(?:foo|bar)\\z",
    b"(?i:Stra\xc3\x9fe)",
    b".*",
];

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn randomized_compile_match_dfa_jit_and_info_match() {
    let (c, rust) = apis();
    unsafe {
        let c_info: PatternInfo = c.symbol(b"pcre2_pattern_info_8\0");
        let r_info: PatternInfo = rust.symbol(b"pcre2_pattern_info_8\0");
        let c_copy: CodeCopy = c.symbol(b"pcre2_code_copy_8\0");
        let r_copy: CodeCopy = rust.symbol(b"pcre2_code_copy_8\0");
        let c_copy_tables: CodeCopy = c.symbol(b"pcre2_code_copy_with_tables_8\0");
        let r_copy_tables: CodeCopy = rust.symbol(b"pcre2_code_copy_with_tables_8\0");
        let c_md_create: MatchDataFromPattern =
            c.symbol(b"pcre2_match_data_create_from_pattern_8\0");
        let r_md_create: MatchDataFromPattern =
            rust.symbol(b"pcre2_match_data_create_from_pattern_8\0");
        let c_md_explicit: MatchDataCreate = c.symbol(b"pcre2_match_data_create_8\0");
        let r_md_explicit: MatchDataCreate = rust.symbol(b"pcre2_match_data_create_8\0");
        let c_md_free: MatchDataFree = c.symbol(b"pcre2_match_data_free_8\0");
        let r_md_free: MatchDataFree = rust.symbol(b"pcre2_match_data_free_8\0");
        let c_match: Match = c.symbol(b"pcre2_match_8\0");
        let r_match: Match = rust.symbol(b"pcre2_match_8\0");
        let c_dfa: DfaMatch = c.symbol(b"pcre2_dfa_match_8\0");
        let r_dfa: DfaMatch = rust.symbol(b"pcre2_dfa_match_8\0");
        let c_jit_compile: JitCompile = c.symbol(b"pcre2_jit_compile_8\0");
        let r_jit_compile: JitCompile = rust.symbol(b"pcre2_jit_compile_8\0");
        let c_jit_match: JitMatch = c.symbol(b"pcre2_jit_match_8\0");
        let r_jit_match: JitMatch = rust.symbol(b"pcre2_jit_match_8\0");
        let c_count: GetU32 = c.symbol(b"pcre2_get_ovector_count_8\0");
        let r_count: GetU32 = rust.symbol(b"pcre2_get_ovector_count_8\0");
        let c_size: GetUsize = c.symbol(b"pcre2_get_match_data_size_8\0");
        let r_size: GetUsize = rust.symbol(b"pcre2_get_match_data_size_8\0");
        let c_heap: GetUsize = c.symbol(b"pcre2_get_match_data_heapframes_size_8\0");
        let r_heap: GetUsize = rust.symbol(b"pcre2_get_match_data_heapframes_size_8\0");
        let c_start: GetUsize = c.symbol(b"pcre2_get_startchar_8\0");
        let r_start: GetUsize = rust.symbol(b"pcre2_get_startchar_8\0");
        let c_mark: GetConstU8 = c.symbol(b"pcre2_get_mark_8\0");
        let r_mark: GetConstU8 = rust.symbol(b"pcre2_get_mark_8\0");
        let c_next: NextMatch = c.symbol(b"pcre2_next_match_8\0");
        let r_next: NextMatch = rust.symbol(b"pcre2_next_match_8\0");

        for &pattern in PATTERNS {
            for &option in COMPILE_OPTIONS {
                let effective = if option == 0x00020000 {
                    option | 0x00080000
                } else {
                    option
                };
                let Some((ccode, rcode)) = compare_compile(&c, &rust, pattern, effective) else {
                    continue;
                };
                for what in 0..=27u32 {
                    let mut cb = [0x5au8; 32];
                    let mut rb = [0x5au8; 32];
                    let cr = c_info(ccode, what, cb.as_mut_ptr().cast());
                    let rr = r_info(rcode, what, rb.as_mut_ptr().cast());
                    assert_eq!(cr, rr, "pattern info {what}");
                    if !matches!(what, 7 | 19) {
                        assert_eq!(cb, rb, "pattern info bytes {what}");
                    }
                }
                let ccopy = c_copy(ccode);
                let rcopy = r_copy(rcode);
                assert_eq!(ccopy.is_null(), rcopy.is_null());
                let ccopyt = c_copy_tables(ccode);
                let rcopyt = r_copy_tables(rcode);
                assert_eq!(ccopyt.is_null(), rcopyt.is_null());
                free_code(&c, ccopy);
                free_code(&rust, rcopy);
                free_code(&c, ccopyt);
                free_code(&rust, rcopyt);

                let cmd = c_md_create(ccode, ptr::null_mut());
                let rmd = r_md_create(rcode, ptr::null_mut());
                assert_eq!(cmd.is_null(), rmd.is_null());
                assert_eq!(c_count(cmd), r_count(rmd));
                assert_eq!(c_size(cmd), r_size(rmd));
                let cexplicit = c_md_explicit(8, ptr::null_mut());
                let rexplicit = r_md_explicit(8, ptr::null_mut());
                assert_eq!(cexplicit.is_null(), rexplicit.is_null());
                c_md_free(cexplicit);
                r_md_free(rexplicit);

                let mut seed = 0x4d595df4d0f33173u64 ^ pattern.len() as u64 ^ effective as u64;
                for case in 0..12 {
                    let length = (next_random(&mut seed) % 48) as usize;
                    let mut subject = vec![0u8; length];
                    for byte in &mut subject {
                        const ALPHABET: &[u8] = b"abcxyz012-\n\r \xc3\x9f";
                        *byte = ALPHABET[(next_random(&mut seed) as usize) % ALPHABET.len()];
                    }
                    let start = if length == 0 {
                        0
                    } else {
                        (next_random(&mut seed) as usize) % (length + 1)
                    };
                    for match_options in [0, 1, 2, 4, 8, 0x10, 0x20, 0x2000, 0x4000] {
                        let cr = c_match(
                            ccode,
                            bytes_ptr(&subject),
                            subject.len(),
                            start,
                            match_options,
                            cmd,
                            ptr::null_mut(),
                        );
                        let rr = r_match(
                            rcode,
                            bytes_ptr(&subject),
                            subject.len(),
                            start,
                            match_options,
                            rmd,
                            ptr::null_mut(),
                        );
                        assert_eq!(cr, rr, "match case {case}, options {match_options:#x}");
                        assert_eq!(ovector(&c, cmd, cr), ovector(&rust, rmd, rr));
                        if cr >= 0 {
                            assert_eq!(c_start(cmd), r_start(rmd));
                            assert_eq!(c_heap(cmd), r_heap(rmd));
                            assert_eq!(c_mark(cmd).is_null(), r_mark(rmd).is_null());
                            let mut co = start;
                            let mut ro = start;
                            let mut copts = match_options;
                            let mut ropts = match_options;
                            assert_eq!(
                                c_next(cmd, &mut co, &mut copts),
                                r_next(rmd, &mut ro, &mut ropts)
                            );
                            assert_eq!((co, copts), (ro, ropts));
                        }
                    }

                    let mut cw = [0i32; 128];
                    let mut rw = [0i32; 128];
                    let cr = c_dfa(
                        ccode,
                        bytes_ptr(&subject),
                        subject.len(),
                        start,
                        0,
                        cmd,
                        ptr::null_mut(),
                        cw.as_mut_ptr(),
                        cw.len(),
                    );
                    let rr = r_dfa(
                        rcode,
                        bytes_ptr(&subject),
                        subject.len(),
                        start,
                        0,
                        rmd,
                        ptr::null_mut(),
                        rw.as_mut_ptr(),
                        rw.len(),
                    );
                    assert_eq!(cr, rr);
                    assert_eq!(ovector(&c, cmd, cr), ovector(&rust, rmd, rr));
                }

                for jit_option in [1, 2, 4, 0x100, 0x200, u32::MAX] {
                    assert_eq!(
                        c_jit_compile(ccode, jit_option),
                        r_jit_compile(rcode, jit_option)
                    );
                }
                let subject = b"abc-123";
                let cr = c_jit_match(
                    ccode,
                    subject.as_ptr(),
                    subject.len(),
                    0,
                    0,
                    cmd,
                    ptr::null_mut(),
                );
                let rr = r_jit_match(
                    rcode,
                    subject.as_ptr(),
                    subject.len(),
                    0,
                    0,
                    rmd,
                    ptr::null_mut(),
                );
                assert_eq!(cr, rr);
                assert_eq!(ovector(&c, cmd, cr), ovector(&rust, rmd, rr));

                c_md_free(cmd);
                r_md_free(rmd);
                free_code(&c, ccode);
                free_code(&rust, rcode);
            }
        }
    }
}

#[repr(C)]
struct EnumerateBlock {
    version: u32,
    pattern_position: usize,
    next_item_length: usize,
    callout_number: u32,
    callout_string_offset: usize,
    callout_string_length: usize,
    callout_string: *const u8,
}

unsafe extern "C" fn enumerate_callback(block: *mut c_void, data: *mut c_void) -> c_int {
    let block = unsafe { &*(block.cast::<EnumerateBlock>()) };
    let values = unsafe { &mut *(data.cast::<Vec<(usize, usize, u32, Vec<u8>)>>()) };
    let string = if block.callout_string.is_null() {
        Vec::new()
    } else {
        unsafe {
            std::slice::from_raw_parts(block.callout_string, block.callout_string_length).to_vec()
        }
    };
    values.push((
        block.pattern_position,
        block.next_item_length,
        block.callout_number,
        string,
    ));
    0
}

#[test]
fn callout_substring_serialization_substitution_and_conversion_match() {
    let (c, rust) = apis();
    unsafe {
        let (ccode, rcode) =
            compare_compile(&c, &rust, b"(?<word>a+)(?<tail>b?)(?C1)(?C\"tag\")", 0)
                .expect("pattern compiles");
        let c_enum: CalloutEnumerate = c.symbol(b"pcre2_callout_enumerate_8\0");
        let r_enum: CalloutEnumerate = rust.symbol(b"pcre2_callout_enumerate_8\0");
        let mut cevents: Vec<(usize, usize, u32, Vec<u8>)> = Vec::new();
        let mut revents: Vec<(usize, usize, u32, Vec<u8>)> = Vec::new();
        assert_eq!(
            c_enum(
                ccode,
                Some(enumerate_callback),
                (&mut cevents as *mut Vec<_>).cast()
            ),
            r_enum(
                rcode,
                Some(enumerate_callback),
                (&mut revents as *mut Vec<_>).cast()
            )
        );
        assert_eq!(cevents, revents);

        let c_md_create: MatchDataFromPattern =
            c.symbol(b"pcre2_match_data_create_from_pattern_8\0");
        let r_md_create: MatchDataFromPattern =
            rust.symbol(b"pcre2_match_data_create_from_pattern_8\0");
        let c_md_free: MatchDataFree = c.symbol(b"pcre2_match_data_free_8\0");
        let r_md_free: MatchDataFree = rust.symbol(b"pcre2_match_data_free_8\0");
        let c_match: Match = c.symbol(b"pcre2_match_8\0");
        let r_match: Match = rust.symbol(b"pcre2_match_8\0");
        let cmd = c_md_create(ccode, ptr::null_mut());
        let rmd = r_md_create(rcode, ptr::null_mut());
        let subject = b"aaab";
        assert_eq!(
            c_match(
                ccode,
                subject.as_ptr(),
                subject.len(),
                0,
                0,
                cmd,
                ptr::null_mut()
            ),
            r_match(
                rcode,
                subject.as_ptr(),
                subject.len(),
                0,
                0,
                rmd,
                ptr::null_mut()
            )
        );

        let c_copy_num: SubstringCopyNumber = c.symbol(b"pcre2_substring_copy_bynumber_8\0");
        let r_copy_num: SubstringCopyNumber = rust.symbol(b"pcre2_substring_copy_bynumber_8\0");
        let c_copy_name: SubstringCopyName = c.symbol(b"pcre2_substring_copy_byname_8\0");
        let r_copy_name: SubstringCopyName = rust.symbol(b"pcre2_substring_copy_byname_8\0");
        for number in 0..=6 {
            let mut cb = [0x55u8; 64];
            let mut rb = [0x55u8; 64];
            let mut cl = cb.len();
            let mut rl = rb.len();
            assert_eq!(
                c_copy_num(cmd, number, cb.as_mut_ptr(), &mut cl),
                r_copy_num(rmd, number, rb.as_mut_ptr(), &mut rl)
            );
            assert_eq!((cl, cb), (rl, rb));
        }
        for name in [b"word\0".as_slice(), b"tail\0", b"missing\0"] {
            let mut cb = [0x66u8; 64];
            let mut rb = [0x66u8; 64];
            let mut cl = cb.len();
            let mut rl = rb.len();
            assert_eq!(
                c_copy_name(cmd, name.as_ptr(), cb.as_mut_ptr(), &mut cl),
                r_copy_name(rmd, name.as_ptr(), rb.as_mut_ptr(), &mut rl)
            );
            assert_eq!((cl, cb), (rl, rb));
        }

        let c_len_num: SubstringLengthNumber = c.symbol(b"pcre2_substring_length_bynumber_8\0");
        let r_len_num: SubstringLengthNumber = rust.symbol(b"pcre2_substring_length_bynumber_8\0");
        let c_len_name: SubstringLengthName = c.symbol(b"pcre2_substring_length_byname_8\0");
        let r_len_name: SubstringLengthName = rust.symbol(b"pcre2_substring_length_byname_8\0");
        for number in 0..=6 {
            let mut cl = usize::MAX;
            let mut rl = usize::MAX;
            assert_eq!(
                c_len_num(cmd, number, &mut cl),
                r_len_num(rmd, number, &mut rl)
            );
            assert_eq!(cl, rl);
        }
        for name in [b"word\0".as_slice(), b"tail\0", b"missing\0"] {
            let mut cl = usize::MAX;
            let mut rl = usize::MAX;
            assert_eq!(
                c_len_name(cmd, name.as_ptr(), &mut cl),
                r_len_name(rmd, name.as_ptr(), &mut rl)
            );
            assert_eq!(cl, rl);
        }

        let c_get_num: SubstringGetNumber = c.symbol(b"pcre2_substring_get_bynumber_8\0");
        let r_get_num: SubstringGetNumber = rust.symbol(b"pcre2_substring_get_bynumber_8\0");
        let c_get_name: SubstringGetName = c.symbol(b"pcre2_substring_get_byname_8\0");
        let r_get_name: SubstringGetName = rust.symbol(b"pcre2_substring_get_byname_8\0");
        let c_sub_free: SubstringFree = c.symbol(b"pcre2_substring_free_8\0");
        let r_sub_free: SubstringFree = rust.symbol(b"pcre2_substring_free_8\0");
        for number in 0..=3 {
            let mut cp = ptr::null_mut();
            let mut rp = ptr::null_mut();
            let mut cl = 0;
            let mut rl = 0;
            let cr = c_get_num(cmd, number, &mut cp, &mut cl);
            let rr = r_get_num(rmd, number, &mut rp, &mut rl);
            assert_eq!((cr, cl), (rr, rl));
            if cr >= 0 {
                assert_eq!(
                    std::slice::from_raw_parts(cp, cl),
                    std::slice::from_raw_parts(rp, rl)
                );
            }
            c_sub_free(cp);
            r_sub_free(rp);
        }
        for name in [b"word\0".as_slice(), b"missing\0"] {
            let mut cp = ptr::null_mut();
            let mut rp = ptr::null_mut();
            let mut cl = 0;
            let mut rl = 0;
            let cr = c_get_name(cmd, name.as_ptr(), &mut cp, &mut cl);
            let rr = r_get_name(rmd, name.as_ptr(), &mut rp, &mut rl);
            assert_eq!((cr, cl), (rr, rl));
            if cr >= 0 {
                assert_eq!(
                    std::slice::from_raw_parts(cp, cl),
                    std::slice::from_raw_parts(rp, rl)
                );
            }
            c_sub_free(cp);
            r_sub_free(rp);
        }

        let c_num_name: SubstringNumberFromName = c.symbol(b"pcre2_substring_number_from_name_8\0");
        let r_num_name: SubstringNumberFromName =
            rust.symbol(b"pcre2_substring_number_from_name_8\0");
        let c_scan: SubstringNametableScan = c.symbol(b"pcre2_substring_nametable_scan_8\0");
        let r_scan: SubstringNametableScan = rust.symbol(b"pcre2_substring_nametable_scan_8\0");
        for name in [b"word\0".as_slice(), b"tail\0", b"missing\0"] {
            assert_eq!(
                c_num_name(ccode, name.as_ptr()),
                r_num_name(rcode, name.as_ptr())
            );
            let mut cf = ptr::null();
            let mut cl = ptr::null();
            let mut rf = ptr::null();
            let mut rl = ptr::null();
            assert_eq!(
                c_scan(ccode, name.as_ptr(), &mut cf, &mut cl),
                r_scan(rcode, name.as_ptr(), &mut rf, &mut rl)
            );
            assert_eq!((cf.is_null(), cl.is_null()), (rf.is_null(), rl.is_null()));
        }

        let c_list_get: SubstringListGet = c.symbol(b"pcre2_substring_list_get_8\0");
        let r_list_get: SubstringListGet = rust.symbol(b"pcre2_substring_list_get_8\0");
        let c_list_free: SubstringListFree = c.symbol(b"pcre2_substring_list_free_8\0");
        let r_list_free: SubstringListFree = rust.symbol(b"pcre2_substring_list_free_8\0");
        let mut clist = ptr::null_mut();
        let mut rlist = ptr::null_mut();
        let mut clens = ptr::null_mut();
        let mut rlens = ptr::null_mut();
        let cr = c_list_get(cmd, &mut clist, &mut clens);
        let rr = r_list_get(rmd, &mut rlist, &mut rlens);
        assert_eq!(cr, rr);
        if cr >= 0 {
            for index in 0..3 {
                let clen = *clens.add(index);
                let rlen = *rlens.add(index);
                assert_eq!(clen, rlen);
                assert_eq!(
                    std::slice::from_raw_parts(*clist.add(index), clen),
                    std::slice::from_raw_parts(*rlist.add(index), rlen)
                );
            }
        }
        c_list_free(clist);
        r_list_free(rlist);

        let c_encode: SerializeEncode = c.symbol(b"pcre2_serialize_encode_8\0");
        let r_encode: SerializeEncode = rust.symbol(b"pcre2_serialize_encode_8\0");
        let c_decode: SerializeDecode = c.symbol(b"pcre2_serialize_decode_8\0");
        let r_decode: SerializeDecode = rust.symbol(b"pcre2_serialize_decode_8\0");
        let c_serial_count: SerializeCount = c.symbol(b"pcre2_serialize_get_number_of_codes_8\0");
        let r_serial_count: SerializeCount =
            rust.symbol(b"pcre2_serialize_get_number_of_codes_8\0");
        let c_serial_free: SerializeFree = c.symbol(b"pcre2_serialize_free_8\0");
        let r_serial_free: SerializeFree = rust.symbol(b"pcre2_serialize_free_8\0");
        let ccodes = [ccode.cast_const()];
        let rcodes = [rcode.cast_const()];
        let mut cbytes = ptr::null_mut();
        let mut rbytes = ptr::null_mut();
        let mut csize = 0;
        let mut rsize = 0;
        assert_eq!(
            c_encode(ccodes.as_ptr(), 1, &mut cbytes, &mut csize, ptr::null_mut()),
            r_encode(rcodes.as_ptr(), 1, &mut rbytes, &mut rsize, ptr::null_mut())
        );
        assert_eq!(csize, rsize);
        assert_eq!(
            std::slice::from_raw_parts(cbytes, csize),
            std::slice::from_raw_parts(rbytes, rsize)
        );
        assert_eq!(c_serial_count(cbytes), r_serial_count(rbytes));
        let mut cdecoded = ptr::null_mut();
        let mut rdecoded = ptr::null_mut();
        assert_eq!(
            c_decode(&mut cdecoded, 1, cbytes, ptr::null_mut()),
            r_decode(&mut rdecoded, 1, rbytes, ptr::null_mut())
        );
        free_code(&c, cdecoded);
        free_code(&rust, rdecoded);
        c_serial_free(cbytes);
        r_serial_free(rbytes);

        let c_sub: Substitute = c.symbol(b"pcre2_substitute_8\0");
        let r_sub: Substitute = rust.symbol(b"pcre2_substitute_8\0");
        for replacement in [b"X".as_slice(), b"$1", b"${word}", b""] {
            for options in [0, 0x100, 0x200, 0x1000, 0x8000] {
                let mut cb = [0x77u8; 128];
                let mut rb = [0x77u8; 128];
                let mut cl = cb.len();
                let mut rl = rb.len();
                let cr = c_sub(
                    ccode,
                    subject.as_ptr(),
                    subject.len(),
                    0,
                    options,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    bytes_ptr(replacement),
                    replacement.len(),
                    cb.as_mut_ptr(),
                    &mut cl,
                );
                let rr = r_sub(
                    rcode,
                    subject.as_ptr(),
                    subject.len(),
                    0,
                    options,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    bytes_ptr(replacement),
                    replacement.len(),
                    rb.as_mut_ptr(),
                    &mut rl,
                );
                assert_eq!((cr, cl), (rr, rl));
                assert_eq!(cb, rb);
            }
        }

        let c_convert: PatternConvert = c.symbol(b"pcre2_pattern_convert_8\0");
        let r_convert: PatternConvert = rust.symbol(b"pcre2_pattern_convert_8\0");
        let c_convert_free: ConvertedFree = c.symbol(b"pcre2_converted_pattern_free_8\0");
        let r_convert_free: ConvertedFree = rust.symbol(b"pcre2_converted_pattern_free_8\0");
        for input in [
            b"a*b?[0-9]".as_slice(),
            b"^(ab|cd)+$",
            b"file/**/x?.txt",
            b"",
        ] {
            for options in [1, 2, 4, 8, 0x10, 0x30, 0x50, u32::MAX] {
                let mut cp = ptr::null_mut();
                let mut rp = ptr::null_mut();
                let mut cl = 0;
                let mut rl = 0;
                let cr = c_convert(
                    bytes_ptr(input),
                    input.len(),
                    options,
                    &mut cp,
                    &mut cl,
                    ptr::null_mut(),
                );
                let rr = r_convert(
                    bytes_ptr(input),
                    input.len(),
                    options,
                    &mut rp,
                    &mut rl,
                    ptr::null_mut(),
                );
                assert_eq!((cr, cl), (rr, rl));
                if cr >= 0 {
                    assert_eq!(
                        std::slice::from_raw_parts(cp, cl),
                        std::slice::from_raw_parts(rp, rl)
                    );
                }
                c_convert_free(cp);
                r_convert_free(rp);
            }
        }

        c_md_free(cmd);
        r_md_free(rmd);
        free_code(&c, ccode);
        free_code(&rust, rcode);
    }
}

const INVALID_PATTERNS: &[&[u8]] = &[
    b"(",
    b")",
    b"[",
    b"[]",
    b"*",
    b"+",
    b"?",
    b"{1,0}",
    b"a{999999}",
    b"\\",
    b"\\x",
    b"\\x{110000}",
    b"\\p",
    b"\\p{NoSuchProperty}",
    b"(?",
    b"(?<>)",
    b"(?<a>a)(?<a>b)",
    b"(?P<1>a)",
    b"(?<=a*)b",
    b"(?R",
    b"(?(999)a|b)",
    b"(?C999)",
    b"(?C\"unterminated)",
    b"(*UNKNOWN)",
    b"(?i",
    b"(?-)",
    b"(?|(",
    b"[z-a]",
    b"\\g{999999}",
    b"(?<averyveryveryveryveryveryveryveryveryveryverylongname>a)",
    b"\xff",
    b"\xc0\x80",
    b"\xe0\x80\x80",
    b"\xf5\x80\x80\x80",
];

#[test]
fn compile_and_public_error_paths_match_exactly() {
    let (c, rust) = apis();
    unsafe {
        for &pattern in INVALID_PATTERNS {
            for options in [0, 0x00080000, u32::MAX] {
                let _ = compare_compile(&c, &rust, pattern, options);
            }
        }

        let c_compile: Compile = c.symbol(b"pcre2_compile_8\0");
        let r_compile: Compile = rust.symbol(b"pcre2_compile_8\0");
        let mut ce = 0;
        let mut re = 0;
        let mut co = 0;
        let mut ro = 0;
        assert_eq!(
            c_compile(ptr::null(), 1, 0, &mut ce, &mut co, ptr::null_mut()).is_null(),
            r_compile(ptr::null(), 1, 0, &mut re, &mut ro, ptr::null_mut()).is_null()
        );
        assert_eq!((ce, co), (re, ro));
        assert_eq!(
            c_compile(
                b"a".as_ptr(),
                1,
                0,
                ptr::null_mut(),
                &mut co,
                ptr::null_mut()
            )
            .is_null(),
            r_compile(
                b"a".as_ptr(),
                1,
                0,
                ptr::null_mut(),
                &mut ro,
                ptr::null_mut()
            )
            .is_null()
        );
        assert_eq!(co, ro);
        assert_eq!(
            c_compile(
                b"a".as_ptr(),
                1,
                0,
                &mut ce,
                ptr::null_mut(),
                ptr::null_mut()
            )
            .is_null(),
            r_compile(
                b"a".as_ptr(),
                1,
                0,
                &mut re,
                ptr::null_mut(),
                ptr::null_mut()
            )
            .is_null()
        );
        assert_eq!(ce, re);

        let c_error: ErrorMessage = c.symbol(b"pcre2_get_error_message_8\0");
        let r_error: ErrorMessage = rust.symbol(b"pcre2_get_error_message_8\0");
        for code in -400..=300 {
            for size in [0, 1, 2, 8, 64, 256] {
                let mut cb = [0x88u8; 256];
                let mut rb = [0x88u8; 256];
                let cr = c_error(code, cb.as_mut_ptr(), size);
                let rr = r_error(code, rb.as_mut_ptr(), size);
                assert_eq!(cr, rr, "error code {code}, size {size}");
                assert_eq!(cb, rb);
            }
        }

        let c_info: PatternInfo = c.symbol(b"pcre2_pattern_info_8\0");
        let r_info: PatternInfo = rust.symbol(b"pcre2_pattern_info_8\0");
        let mut cb = [0u8; 16];
        let mut rb = [0u8; 16];
        for what in [0, 27, u32::MAX] {
            assert_eq!(
                c_info(ptr::null(), what, cb.as_mut_ptr().cast()),
                r_info(ptr::null(), what, rb.as_mut_ptr().cast())
            );
        }

        let c_match: Match = c.symbol(b"pcre2_match_8\0");
        let r_match: Match = rust.symbol(b"pcre2_match_8\0");
        let c_md_create: MatchDataCreate = c.symbol(b"pcre2_match_data_create_8\0");
        let r_md_create: MatchDataCreate = rust.symbol(b"pcre2_match_data_create_8\0");
        let c_md_free: MatchDataFree = c.symbol(b"pcre2_match_data_free_8\0");
        let r_md_free: MatchDataFree = rust.symbol(b"pcre2_match_data_free_8\0");
        let cmd = c_md_create(4, ptr::null_mut());
        let rmd = r_md_create(4, ptr::null_mut());
        for (subject, length, offset, options) in [
            (ptr::null(), 0, 0, 0),
            (ptr::null(), 1, 0, 0),
            (b"a".as_ptr(), 1, 2, 0),
            (b"a".as_ptr(), 1, 0, u32::MAX),
        ] {
            assert_eq!(
                c_match(
                    ptr::null(),
                    subject,
                    length,
                    offset,
                    options,
                    cmd,
                    ptr::null_mut()
                ),
                r_match(
                    ptr::null(),
                    subject,
                    length,
                    offset,
                    options,
                    rmd,
                    ptr::null_mut()
                )
            );
        }
        c_md_free(cmd);
        r_md_free(rmd);

        let c_encode: SerializeEncode = c.symbol(b"pcre2_serialize_encode_8\0");
        let r_encode: SerializeEncode = rust.symbol(b"pcre2_serialize_encode_8\0");
        let mut cp = ptr::null_mut();
        let mut rp = ptr::null_mut();
        let mut cs = 0;
        let mut rs = 0;
        for count in [-1, 0, 1] {
            assert_eq!(
                c_encode(ptr::null(), count, &mut cp, &mut cs, ptr::null_mut()),
                r_encode(ptr::null(), count, &mut rp, &mut rs, ptr::null_mut())
            );
            assert_eq!(cs, rs);
        }
        let c_count: SerializeCount = c.symbol(b"pcre2_serialize_get_number_of_codes_8\0");
        let r_count: SerializeCount = rust.symbol(b"pcre2_serialize_get_number_of_codes_8\0");
        assert_eq!(c_count(ptr::null()), r_count(ptr::null()));
        for bytes in [[0u8; 32], [0xffu8; 32], {
            let mut value = [0u8; 32];
            value[0] = 1;
            value
        }] {
            assert_eq!(c_count(bytes.as_ptr()), r_count(bytes.as_ptr()));
        }

        let c_convert: PatternConvert = c.symbol(b"pcre2_pattern_convert_8\0");
        let r_convert: PatternConvert = rust.symbol(b"pcre2_pattern_convert_8\0");
        for (pattern, length, options) in [
            (ptr::null(), 0, 4),
            (ptr::null(), 1, 4),
            (b"a".as_ptr(), 1, 0),
            (b"a".as_ptr(), 1, u32::MAX),
        ] {
            let mut cconverted = ptr::null_mut();
            let mut rconverted = ptr::null_mut();
            let mut clen = 0;
            let mut rlen = 0;
            assert_eq!(
                c_convert(
                    pattern,
                    length,
                    options,
                    &mut cconverted,
                    &mut clen,
                    ptr::null_mut()
                ),
                r_convert(
                    pattern,
                    length,
                    options,
                    &mut rconverted,
                    &mut rlen,
                    ptr::null_mut()
                )
            );
            assert_eq!(clen, rlen);
        }
    }
}
