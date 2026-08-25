mod common;

use common::*;
use libloading::Library;
use std::ffi::c_void;
use std::fs;
use std::ptr;

#[repr(align(8))]
#[derive(PartialEq, Eq, Debug)]
struct AlignedBytes<const N: usize>([u8; N]);

#[test]
fn every_c_dynamic_symbol_is_loadable_from_both_libraries() {
    unsafe {
        let libraries = Libraries::open();
        let symbols = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/SYMBOLS.md"))
            .expect("read SYMBOLS.md");
        let mut checked = 0;
        for line in symbols
            .lines()
            .filter(|line| line.starts_with("| ") && line.contains("`"))
        {
            let fields: Vec<_> = line.split('|').collect();
            if fields.len() < 5 || fields[1].trim().parse::<usize>().is_err() {
                continue;
            }
            let name = fields[3].trim().trim_matches('`');
            let mut nul_name = name.as_bytes().to_vec();
            nul_name.push(0);
            let _: *mut c_void = sym(&libraries.c, &nul_name);
            let _: *mut c_void = sym(&libraries.rust, &nul_name);
            checked += 1;
        }
        assert_eq!(checked, 143);
    }
}

#[test]
fn configuration_selectors_match_for_values_lengths_and_errors() {
    unsafe {
        let libraries = Libraries::open();
        let c: ConfigFn = sym(&libraries.c, b"pcre2_config_8\0");
        let rust: ConfigFn = sym(&libraries.rust, b"pcre2_config_8\0");

        for selector in 0..=16 {
            let c_length = c(selector, ptr::null_mut());
            let rust_length = rust(selector, ptr::null_mut());
            assert_eq!(rust_length, c_length, "length selector {selector}");

            let mut c_output = AlignedBytes([0xa5_u8; 128]);
            let mut rust_output = AlignedBytes([0xa5_u8; 128]);
            let c_rc = c(selector, c_output.0.as_mut_ptr().cast());
            let rust_rc = rust(selector, rust_output.0.as_mut_ptr().cast());
            assert_eq!(rust_rc, c_rc, "return selector {selector}");
            assert_eq!(rust_output, c_output, "output selector {selector}");
        }

        for selector in [17, u32::MAX, 0xdead_beef] {
            assert_eq!(
                rust(selector, ptr::null_mut()),
                c(selector, ptr::null_mut()),
                "invalid selector {selector:#x}"
            );
            let mut c_output = AlignedBytes([0xa5_u8; 16]);
            let mut rust_output = AlignedBytes([0xa5_u8; 16]);
            let c_rc = c(selector, c_output.0.as_mut_ptr().cast());
            let rust_rc = rust(selector, rust_output.0.as_mut_ptr().cast());
            assert_eq!(rust_rc, c_rc);
            assert_eq!(rust_output, c_output);
        }
    }
}

#[test]
fn all_public_error_messages_match_at_buffer_boundaries() {
    unsafe {
        let libraries = Libraries::open();
        let c: GetErrorMessageFn = sym(&libraries.c, b"pcre2_get_error_message_8\0");
        let rust: GetErrorMessageFn = sym(&libraries.rust, b"pcre2_get_error_message_8\0");

        for error in -100..=130 {
            for capacity in [0, 1, 2, 7, 32, 128, 256] {
                let mut c_buffer = vec![0xa5_u8; capacity.max(1)];
                let mut rust_buffer = vec![0xa5_u8; capacity.max(1)];
                let c_pointer = if capacity == 0 {
                    ptr::null_mut()
                } else {
                    c_buffer.as_mut_ptr()
                };
                let rust_pointer = if capacity == 0 {
                    ptr::null_mut()
                } else {
                    rust_buffer.as_mut_ptr()
                };
                let c_rc = c(error, c_pointer, capacity);
                let rust_rc = rust(error, rust_pointer, capacity);
                assert_eq!(rust_rc, c_rc, "error {error}, capacity {capacity}");
                assert_eq!(
                    rust_buffer, c_buffer,
                    "message bytes for error {error}, capacity {capacity}"
                );
            }
        }
    }
}

type GeneralCreateFn = unsafe extern "C" fn(
    Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    *mut c_void,
) -> *mut GeneralContext;
type ContextCreateFn = unsafe extern "C" fn(*mut GeneralContext) -> *mut c_void;
type ContextCopyFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type ContextFreeFn = unsafe extern "C" fn(*mut c_void);
type SetU32Fn = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type SetUsizeFn = unsafe extern "C" fn(*mut c_void, usize) -> i32;
type SetPointerFn = unsafe extern "C" fn(*mut c_void, *const u8) -> i32;
type SetCallbackFn =
    unsafe extern "C" fn(*mut c_void, Option<unsafe extern "C" fn()>, *mut c_void) -> i32;
type SetRecursionMemoryFn = unsafe extern "C" fn(
    *mut MatchContext,
    Option<unsafe extern "C" fn()>,
    Option<unsafe extern "C" fn()>,
    *mut c_void,
) -> i32;
type MakeTablesFn = unsafe extern "C" fn(*mut GeneralContext) -> *const u8;
type MakeTablesFreeFn = unsafe extern "C" fn(*mut GeneralContext, *const u8);
type JitStackCreateFn = unsafe extern "C" fn(usize, usize, *mut GeneralContext) -> *mut c_void;
type JitStackAssignFn =
    unsafe extern "C" fn(*mut MatchContext, Option<unsafe extern "C" fn()>, *mut c_void);

unsafe fn exercise_contexts(library: &Library) -> Vec<i64> {
    let general_create: GeneralCreateFn = sym(library, b"pcre2_general_context_create_8\0");
    let general_copy: ContextCopyFn = sym(library, b"pcre2_general_context_copy_8\0");
    let general_free: ContextFreeFn = sym(library, b"pcre2_general_context_free_8\0");
    let compile_create: ContextCreateFn = sym(library, b"pcre2_compile_context_create_8\0");
    let compile_copy: ContextCopyFn = sym(library, b"pcre2_compile_context_copy_8\0");
    let compile_free: ContextFreeFn = sym(library, b"pcre2_compile_context_free_8\0");
    let match_create: ContextCreateFn = sym(library, b"pcre2_match_context_create_8\0");
    let match_copy: ContextCopyFn = sym(library, b"pcre2_match_context_copy_8\0");
    let match_free: ContextFreeFn = sym(library, b"pcre2_match_context_free_8\0");
    let convert_create: ContextCreateFn = sym(library, b"pcre2_convert_context_create_8\0");
    let convert_copy: ContextCopyFn = sym(library, b"pcre2_convert_context_copy_8\0");
    let convert_free: ContextFreeFn = sym(library, b"pcre2_convert_context_free_8\0");

    let general = general_create(None, None, ptr::null_mut());
    assert!(!general.is_null());
    let general_clone = general_copy(general.cast());
    assert!(!general_clone.is_null());
    let compile = compile_create(general).cast::<CompileContext>();
    let match_context = match_create(general).cast::<MatchContext>();
    let convert = convert_create(general).cast::<ConvertContext>();
    assert!(!compile.is_null() && !match_context.is_null() && !convert.is_null());
    let compile_clone = compile_copy(compile.cast());
    let match_clone = match_copy(match_context.cast());
    let convert_clone = convert_copy(convert.cast());
    assert!(!compile_clone.is_null() && !match_clone.is_null() && !convert_clone.is_null());

    let mut results = Vec::new();
    let set_bsr: SetU32Fn = sym(library, b"pcre2_set_bsr_8\0");
    for value in [0, 1, 2, 3, u32::MAX] {
        results.push(set_bsr(compile.cast(), value) as i64);
    }
    let set_newline: SetU32Fn = sym(library, b"pcre2_set_newline_8\0");
    for value in [0, 1, 2, 3, 4, 5, 6, 7, u32::MAX] {
        results.push(set_newline(compile.cast(), value) as i64);
    }
    let set_optimize: SetU32Fn = sym(library, b"pcre2_set_optimize_8\0");
    for value in [0, 1, 2, 63, 64, 65, 66, 67, 68, 69, 70, u32::MAX] {
        results.push(set_optimize(compile.cast(), value) as i64);
    }

    for name in [
        b"pcre2_set_compile_extra_options_8\0".as_slice(),
        b"pcre2_set_max_varlookbehind_8\0",
        b"pcre2_set_parens_nest_limit_8\0",
    ] {
        let setter: SetU32Fn = sym(library, name);
        for value in [0, 1, 255, u32::MAX] {
            results.push(setter(compile.cast(), value) as i64);
        }
    }
    for name in [
        b"pcre2_set_max_pattern_length_8\0".as_slice(),
        b"pcre2_set_max_pattern_compiled_length_8\0",
    ] {
        let setter: SetUsizeFn = sym(library, name);
        for value in [0, 1, 255, usize::MAX] {
            results.push(setter(compile.cast(), value) as i64);
        }
    }

    let tables: MakeTablesFn = sym(library, b"pcre2_maketables_8\0");
    let tables_free: MakeTablesFreeFn = sym(library, b"pcre2_maketables_free_8\0");
    let table_pointer = tables(general);
    assert!(!table_pointer.is_null());
    results.extend_from_slice(
        std::slice::from_raw_parts(table_pointer, 1088)
            .iter()
            .map(|byte| *byte as i64)
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let set_tables: SetPointerFn = sym(library, b"pcre2_set_character_tables_8\0");
    results.push(set_tables(compile.cast(), table_pointer) as i64);

    for name in [
        b"pcre2_set_heap_limit_8\0".as_slice(),
        b"pcre2_set_match_limit_8\0",
        b"pcre2_set_depth_limit_8\0",
        b"pcre2_set_recursion_limit_8\0",
    ] {
        let setter: SetU32Fn = sym(library, name);
        for value in [0, 1, 100, u32::MAX] {
            results.push(setter(match_context.cast(), value) as i64);
        }
    }
    let offset_setter: SetUsizeFn = sym(library, b"pcre2_set_offset_limit_8\0");
    for value in [0, 1, usize::MAX] {
        results.push(offset_setter(match_context.cast(), value) as i64);
    }

    for name in [
        b"pcre2_set_callout_8\0".as_slice(),
        b"pcre2_set_substitute_callout_8\0",
        b"pcre2_set_substitute_case_callout_8\0",
        b"pcre2_set_compile_recursion_guard_8\0",
    ] {
        let setter: SetCallbackFn = sym(library, name);
        let target = if name.windows(7).any(|window| window == b"compile") {
            compile.cast()
        } else {
            match_context.cast()
        };
        results.push(setter(target, None, ptr::null_mut()) as i64);
    }
    let set_recursion_memory: SetRecursionMemoryFn =
        sym(library, b"pcre2_set_recursion_memory_management_8\0");
    results.push(set_recursion_memory(match_context, None, None, ptr::null_mut()) as i64);

    let set_separator: SetU32Fn = sym(library, b"pcre2_set_glob_separator_8\0");
    for value in [b'/' as u32, b'\\' as u32, b'.' as u32, 0, 256, u32::MAX] {
        results.push(set_separator(convert.cast(), value) as i64);
    }
    let set_escape: SetU32Fn = sym(library, b"pcre2_set_glob_escape_8\0");
    for value in [0, b'!' as u32, b'\\' as u32, b'a' as u32, 256, u32::MAX] {
        results.push(set_escape(convert.cast(), value) as i64);
    }

    let jit_create: JitStackCreateFn = sym(library, b"pcre2_jit_stack_create_8\0");
    let jit_assign: JitStackAssignFn = sym(library, b"pcre2_jit_stack_assign_8\0");
    let jit_free: ContextFreeFn = sym(library, b"pcre2_jit_stack_free_8\0");
    let jit_unused: ContextFreeFn = sym(library, b"pcre2_jit_free_unused_memory_8\0");
    let stack = jit_create(1, 32 * 1024, general);
    results.push((!stack.is_null()) as i64);
    jit_assign(match_context, None, ptr::null_mut());
    jit_free(stack);
    jit_unused(general.cast());

    tables_free(general, table_pointer);
    compile_free(compile_clone);
    match_free(match_clone);
    convert_free(convert_clone);
    compile_free(compile.cast());
    match_free(match_context.cast());
    convert_free(convert.cast());
    general_free(general_clone);
    general_free(general.cast());
    results
}

#[test]
fn contexts_setters_tables_and_no_jit_stubs_match() {
    unsafe {
        let libraries = Libraries::open();
        assert_eq!(
            exercise_contexts(&libraries.rust),
            exercise_contexts(&libraries.c)
        );
    }
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

unsafe extern "C" fn tracked_malloc(size: usize, data: *mut c_void) -> *mut c_void {
    if !data.is_null() {
        unsafe { *data.cast::<usize>() += 1 };
    }
    unsafe { malloc(size) }
}

unsafe extern "C" fn tracked_free(pointer: *mut c_void, data: *mut c_void) {
    if !data.is_null() {
        unsafe { *data.cast::<usize>() += 1 };
    }
    unsafe { free(pointer) };
}

unsafe fn custom_allocator_snapshot(library: &Library) -> (usize, Vec<u32>) {
    let create: GeneralCreateFn = sym(library, b"pcre2_general_context_create_8\0");
    let free_context: ContextFreeFn = sym(library, b"pcre2_general_context_free_8\0");
    let create_data: MatchDataCreateFn = sym(library, b"pcre2_match_data_create_8\0");
    let free_data: MatchDataFreeFn = sym(library, b"pcre2_match_data_free_8\0");
    let count: GetOvectorCountFn = sym(library, b"pcre2_get_ovector_count_8\0");
    let mut callback_count = 0_usize;
    let context = create(
        Some(tracked_malloc),
        Some(tracked_free),
        (&mut callback_count as *mut usize).cast(),
    );
    assert!(!context.is_null());
    let mut counts = Vec::new();
    for requested in [0, 1, 17, 65536] {
        let data = create_data(requested, context);
        assert!(!data.is_null());
        counts.push(count(data));
        free_data(data);
    }
    free_context(context.cast());
    (callback_count, counts)
}

#[test]
fn custom_general_context_allocator_behavior_matches() {
    unsafe {
        let libraries = Libraries::open();
        assert_eq!(
            custom_allocator_snapshot(&libraries.rust),
            custom_allocator_snapshot(&libraries.c)
        );
    }
}

unsafe fn contextual_compile_match(
    library: &Library,
    setter_name: &[u8],
    value: u32,
    pattern: &[u8],
    compile_options: u32,
    subject: &[u8],
) -> (i32, CompileSnapshot, Option<MatchSnapshot>) {
    let create: ContextCreateFn = sym(library, b"pcre2_compile_context_create_8\0");
    let free_context: ContextFreeFn = sym(library, b"pcre2_compile_context_free_8\0");
    let setter: SetU32Fn = sym(library, setter_name);
    let context = create(ptr::null_mut()).cast::<CompileContext>();
    let setter_rc = setter(context.cast(), value);
    let (compile, code) =
        compile_snapshot(library, pattern, pattern.len(), compile_options, context);
    let matched = if code.is_null() {
        None
    } else {
        Some(match_snapshot(
            library,
            code,
            subject,
            subject.len(),
            0,
            0,
            ptr::null_mut(),
        ))
    };
    free_code(library, code);
    free_context(context.cast());
    (setter_rc, compile, matched)
}

#[test]
fn compile_context_options_newlines_and_bsr_match_end_to_end() {
    unsafe {
        let libraries = Libraries::open();
        let extra_patterns: &[(u32, &[u8], u32, &[u8])] = &[
            (0x0000_0001, b"\\x{d800}", 0x0008_0000, b""),
            (0x0000_0002, b"\\q", 0, b"q"),
            (0x0000_0004, b"word", 0, b"word"),
            (0x0000_0008, b"line", 0, b"line"),
            (0x0000_0010, b"\\r", 0, b"\n"),
            (0x0000_0020, b"\\u0061", 0, b"a"),
            (0x0000_0040, b"(?=a\\K)a", 0, b"a"),
            (0x0000_0080, b"(?i)a", 0, b"A"),
            (0x0000_0100, b"\\d+", 0, b"123"),
            (0x0000_0200, b"\\s+", 0, b" "),
            (0x0000_0400, b"\\w+", 0, b"word"),
            (0x0000_0800, b"[[:alpha:]]+", 0, b"abc"),
            (0x0000_1000, b"\\d+", 0, b"123"),
            (0x0000_2000, b"\\012", 0, b"\n"),
            (0x0000_4000, b"\\0", 0, b"\0"),
            (0x0000_8000, b"(?C1)a", 0, b"a"),
            (0x0001_0000, b"(?i)i", 0x0008_0000, b"i"),
        ];
        for &(value, pattern, options, subject) in extra_patterns {
            assert_eq!(
                contextual_compile_match(
                    &libraries.rust,
                    b"pcre2_set_compile_extra_options_8\0",
                    value,
                    pattern,
                    options,
                    subject,
                ),
                contextual_compile_match(
                    &libraries.c,
                    b"pcre2_set_compile_extra_options_8\0",
                    value,
                    pattern,
                    options,
                    subject,
                ),
                "extra option {value:#x}"
            );
        }
        for newline in 1..=6 {
            assert_eq!(
                contextual_compile_match(
                    &libraries.rust,
                    b"pcre2_set_newline_8\0",
                    newline,
                    b"^a$",
                    0x0000_0400,
                    b"a\r\n",
                ),
                contextual_compile_match(
                    &libraries.c,
                    b"pcre2_set_newline_8\0",
                    newline,
                    b"^a$",
                    0x0000_0400,
                    b"a\r\n",
                ),
                "newline {newline}"
            );
        }
        for bsr in 1..=2 {
            assert_eq!(
                contextual_compile_match(
                    &libraries.rust,
                    b"pcre2_set_bsr_8\0",
                    bsr,
                    b"\\R",
                    0x0008_0000,
                    "\u{0085}".as_bytes(),
                ),
                contextual_compile_match(
                    &libraries.c,
                    b"pcre2_set_bsr_8\0",
                    bsr,
                    b"\\R",
                    0x0008_0000,
                    "\u{0085}".as_bytes(),
                ),
                "bsr {bsr}"
            );
        }
    }
}

const COMPILE_OPTIONS: &[u32] = &[
    0,
    0x8000_0000,
    0x4000_0000,
    0x2000_0000,
    0x0000_0001,
    0x0000_0002,
    0x0000_0004,
    0x0000_0008,
    0x0000_0010,
    0x0000_0020,
    0x0000_0040,
    0x0000_0080,
    0x0000_0100,
    0x0000_0200,
    0x0000_0400,
    0x0000_0800,
    0x0000_1000,
    0x0000_2000,
    0x0000_4000,
    0x0000_8000,
    0x0001_0000,
    0x0002_0000,
    0x0004_0000,
    0x0008_0000,
    0x0010_0000,
    0x0020_0000,
    0x0040_0000,
    0x0080_0000,
    0x0100_0000,
    0x0200_0000,
    0x0400_0000,
    0x0800_0000,
];

unsafe fn free_code(library: &Library, code: *mut Code) {
    let free: CodeFreeFn = sym(library, b"pcre2_code_free_8\0");
    free(code);
}

#[test]
fn randomized_compile_options_errors_and_pattern_info_match() {
    unsafe {
        let libraries = Libraries::open();
        let patterns: &[&[u8]] = &[
            b"",
            b"a",
            b"abc",
            b"a|bc",
            b"(a+)(?<word>b*)\\1",
            b"[A-Za-z0-9_]+",
            b"^.*$",
            b"(?<=ab)c",
            b"\\p{L}+",
            b"(*MARK:tag)a",
            b"a{0,3}?",
            b"(?C1)a(?C\"callout\")",
        ];
        for &options in COMPILE_OPTIONS {
            for &pattern in patterns {
                let (c_snapshot, c_code) = compile_snapshot(
                    &libraries.c,
                    pattern,
                    pattern.len(),
                    options,
                    ptr::null_mut(),
                );
                let (rust_snapshot, rust_code) = compile_snapshot(
                    &libraries.rust,
                    pattern,
                    pattern.len(),
                    options,
                    ptr::null_mut(),
                );
                assert_eq!(
                    rust_snapshot,
                    c_snapshot,
                    "compile option {options:#010x}, pattern {:?}",
                    String::from_utf8_lossy(pattern)
                );
                assert_eq!(rust_code.is_null(), c_code.is_null());
                free_code(&libraries.c, c_code);
                free_code(&libraries.rust, rust_code);
            }
        }

        let mut seed = 0x4d59_5df4_d0f3_3173_u64;
        for case in 0..1500 {
            let length = (xorshift64(&mut seed) % 48) as usize;
            let mut pattern = vec![0_u8; length.max(1)];
            for byte in &mut pattern[..length] {
                *byte = xorshift64(&mut seed) as u8;
            }
            let explicit_length = if case % 11 == 0 {
                pattern[length.saturating_sub(1)] = 0;
                ZERO_TERMINATED
            } else {
                length
            };
            let options = if case % 7 == 0 { 0x0008_0000 } else { 0 };
            let (c_snapshot, c_code) = compile_snapshot(
                &libraries.c,
                &pattern,
                explicit_length,
                options,
                ptr::null_mut(),
            );
            let (rust_snapshot, rust_code) = compile_snapshot(
                &libraries.rust,
                &pattern,
                explicit_length,
                options,
                ptr::null_mut(),
            );
            assert_eq!(rust_snapshot, c_snapshot, "random compile case {case}");
            assert_eq!(rust_code.is_null(), c_code.is_null());
            free_code(&libraries.c, c_code);
            free_code(&libraries.rust, rust_code);
        }
    }
}

unsafe fn compile_pair(
    libraries: &Libraries,
    pattern: &[u8],
    options: u32,
) -> (*mut Code, *mut Code) {
    let (c_snapshot, c_code) = compile_snapshot(
        &libraries.c,
        pattern,
        pattern.len(),
        options,
        ptr::null_mut(),
    );
    let (rust_snapshot, rust_code) = compile_snapshot(
        &libraries.rust,
        pattern,
        pattern.len(),
        options,
        ptr::null_mut(),
    );
    assert_eq!(rust_snapshot, c_snapshot);
    assert!(!c_code.is_null() && !rust_code.is_null(), "{c_snapshot:?}");
    (c_code, rust_code)
}

#[test]
fn randomized_interpreter_and_copied_code_matches_are_byte_identical() {
    unsafe {
        let libraries = Libraries::open();
        let cases: &[(&[u8], u32, &[&[u8]])] = &[
            (b"", 0, &[b"", b"a"]),
            (b"a", 0, &[b"", b"a", b"ba", b"ab"]),
            (b"(a+)(b*)", 0, &[b"a", b"aaabbb", b"baaa", b""]),
            (b"(?<word>[A-Za-z]+)", 0, &[b"abc", b"123abc", b""]),
            (b"^.*$", 0x0000_0020, &[b"", b"abc", b"a\nb"]),
            (b"(*MARK:seen)a+", 0, &[b"a", b"baaa", b"bbb"]),
            (
                b"\\p{L}+",
                0x0008_0000 | 0x0002_0000,
                &[b"letters", "\u{00e9}lan".as_bytes(), b"123"],
            ),
        ];
        let match_options = [
            0,
            0x8000_0000,
            0x2000_0000,
            0x0000_0001,
            0x0000_0002,
            0x0000_0004,
            0x0000_0008,
            0x0000_0010,
            0x0000_0020,
            0x0000_2000,
            0x0000_4000,
        ];

        for &(pattern, compile_options, subjects) in cases {
            let (c_code, rust_code) = compile_pair(&libraries, pattern, compile_options);
            let c_copy: CodeCopyFn = sym(&libraries.c, b"pcre2_code_copy_8\0");
            let rust_copy: CodeCopyFn = sym(&libraries.rust, b"pcre2_code_copy_8\0");
            let c_copy_tables: CodeCopyFn = sym(&libraries.c, b"pcre2_code_copy_with_tables_8\0");
            let rust_copy_tables: CodeCopyFn =
                sym(&libraries.rust, b"pcre2_code_copy_with_tables_8\0");
            let c_copies = [c_copy(c_code), c_copy_tables(c_code)];
            let rust_copies = [rust_copy(rust_code), rust_copy_tables(rust_code)];
            assert!(c_copies.iter().all(|code| !code.is_null()));
            assert!(rust_copies.iter().all(|code| !code.is_null()));

            for (subject, options) in subjects.iter().flat_map(|subject| {
                match_options
                    .iter()
                    .map(move |options| (*subject, *options))
            }) {
                for start in [0, subject.len() / 2, subject.len()] {
                    let c_match = match_snapshot(
                        &libraries.c,
                        c_code,
                        subject,
                        subject.len(),
                        start,
                        options,
                        ptr::null_mut(),
                    );
                    let rust_match = match_snapshot(
                        &libraries.rust,
                        rust_code,
                        subject,
                        subject.len(),
                        start,
                        options,
                        ptr::null_mut(),
                    );
                    assert_eq!(
                        rust_match,
                        c_match,
                        "pattern {:?}, subject {:?}, start {start}, options {options:#x}",
                        String::from_utf8_lossy(pattern),
                        String::from_utf8_lossy(subject)
                    );
                    for index in 0..2 {
                        let c_copy_match = match_snapshot(
                            &libraries.c,
                            c_copies[index],
                            subject,
                            subject.len(),
                            start,
                            options,
                            ptr::null_mut(),
                        );
                        let rust_copy_match = match_snapshot(
                            &libraries.rust,
                            rust_copies[index],
                            subject,
                            subject.len(),
                            start,
                            options,
                            ptr::null_mut(),
                        );
                        assert_eq!(rust_copy_match, c_copy_match);
                    }
                }
            }
            for code in c_copies {
                free_code(&libraries.c, code);
            }
            for code in rust_copies {
                free_code(&libraries.rust, code);
            }
            free_code(&libraries.c, c_code);
            free_code(&libraries.rust, rust_code);
        }
    }
}

unsafe fn dfa_snapshot(
    library: &Library,
    code: *const Code,
    subject: &[u8],
    options: u32,
    workspace_size: usize,
) -> (i32, Vec<usize>, Vec<i32>) {
    let create: MatchDataFromPatternFn = sym(library, b"pcre2_match_data_create_from_pattern_8\0");
    let free: MatchDataFreeFn = sym(library, b"pcre2_match_data_free_8\0");
    let run: DfaMatchFn = sym(library, b"pcre2_dfa_match_8\0");
    let vector_fn: GetOvectorPointerFn = sym(library, b"pcre2_get_ovector_pointer_8\0");
    let count_fn: GetOvectorCountFn = sym(library, b"pcre2_get_ovector_count_8\0");
    let data = create(code, ptr::null_mut());
    let mut workspace = vec![0x5a5a_5a5a_i32; workspace_size.max(1)];
    let rc = run(
        code,
        subject.as_ptr(),
        subject.len(),
        0,
        options,
        data,
        ptr::null_mut(),
        workspace.as_mut_ptr(),
        workspace_size,
    );
    let count = count_fn(data) as usize;
    let used = if rc > 0 { (rc as usize).min(count) } else { 1 };
    let ovector = std::slice::from_raw_parts(vector_fn(data), used * 2).to_vec();
    free(data);
    (rc, ovector, workspace)
}

#[test]
fn dfa_modes_workspace_boundaries_and_jit_stubs_match() {
    unsafe {
        let libraries = Libraries::open();
        let (c_code, rust_code) = compile_pair(&libraries, b"(a|ab)+", 0);
        for subject in [b"".as_slice(), b"a", b"abab", b"xabab"] {
            for options in [0, 0x8000_0000, 0x10, 0x20, 0x80] {
                for workspace_size in [0, 1, 10, 20, 100] {
                    assert_eq!(
                        dfa_snapshot(&libraries.rust, rust_code, subject, options, workspace_size,),
                        dfa_snapshot(&libraries.c, c_code, subject, options, workspace_size,),
                        "subject {:?}, options {options:#x}, workspace {workspace_size}",
                        String::from_utf8_lossy(subject)
                    );
                }
            }
        }

        type JitCompileFn = unsafe extern "C" fn(*mut Code, u32) -> i32;
        let c_jit: JitCompileFn = sym(&libraries.c, b"pcre2_jit_compile_8\0");
        let rust_jit: JitCompileFn = sym(&libraries.rust, b"pcre2_jit_compile_8\0");
        for options in [0, 1, 2, 4, 0x100, u32::MAX] {
            assert_eq!(rust_jit(rust_code, options), c_jit(c_code, options));
        }
        free_code(&libraries.c, c_code);
        free_code(&libraries.rust, rust_code);
    }
}

#[test]
fn public_null_invalid_option_offset_and_utf_rejections_match() {
    unsafe {
        let libraries = Libraries::open();
        for library in [&libraries.c, &libraries.rust] {
            let compile: CompileFn = sym(library, b"pcre2_compile_8\0");
            let mut error = 0;
            let mut offset = 0;
            let null_pattern = compile(ptr::null(), 1, 0, &mut error, &mut offset, ptr::null_mut());
            assert!(null_pattern.is_null());
        }

        let invalid_patterns: &[(&[u8], u32)] = &[
            (b"\\", 0),
            (b"[", 0),
            (b"(", 0),
            (b"a{2,1}", 0),
            (b"(?<1bad>a)", 0),
            (b"\\x{110000}", 0x0008_0000),
            (&[0xff], 0x0008_0000),
        ];
        for &(pattern, options) in invalid_patterns {
            let (c_snapshot, c_code) = compile_snapshot(
                &libraries.c,
                pattern,
                pattern.len(),
                options,
                ptr::null_mut(),
            );
            let (rust_snapshot, rust_code) = compile_snapshot(
                &libraries.rust,
                pattern,
                pattern.len(),
                options,
                ptr::null_mut(),
            );
            assert_eq!(rust_snapshot, c_snapshot);
            assert!(c_code.is_null() && rust_code.is_null());
        }

        let (c_code, rust_code) = compile_pair(&libraries, b"a", 0);
        let mut rejection_sets = Vec::new();
        for (library, code) in [(&libraries.c, c_code), (&libraries.rust, rust_code)] {
            let create: MatchDataFromPatternFn =
                sym(library, b"pcre2_match_data_create_from_pattern_8\0");
            let free: MatchDataFreeFn = sym(library, b"pcre2_match_data_free_8\0");
            let run: MatchFn = sym(library, b"pcre2_match_8\0");
            let data = create(code, ptr::null_mut());
            assert!(!data.is_null());
            let null_code_rc = run(ptr::null(), b"a".as_ptr(), 1, 0, 0, data, ptr::null_mut());
            let null_subject_rc = run(code, ptr::null(), 1, 0, 0, data, ptr::null_mut());
            let bad_offset_rc = run(code, b"a".as_ptr(), 1, 2, 0, data, ptr::null_mut());
            let bad_option_rc = run(
                code,
                b"a".as_ptr(),
                1,
                0,
                0x1000_0000,
                data,
                ptr::null_mut(),
            );
            let null_data_rc = run(
                code,
                b"a".as_ptr(),
                1,
                0,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            rejection_sets.push([
                null_code_rc,
                null_subject_rc,
                bad_offset_rc,
                bad_option_rc,
                null_data_rc,
            ]);
            free(data);
        }
        assert_eq!(rejection_sets[1], rejection_sets[0]);
        free_code(&libraries.c, c_code);
        free_code(&libraries.rust, rust_code);
    }
}

type SubstringCopyNumberFn = unsafe extern "C" fn(*mut MatchData, u32, *mut u8, *mut usize) -> i32;
type SubstringCopyNameFn =
    unsafe extern "C" fn(*mut MatchData, *const u8, *mut u8, *mut usize) -> i32;
type SubstringLengthNumberFn = unsafe extern "C" fn(*mut MatchData, u32, *mut usize) -> i32;
type SubstringLengthNameFn = unsafe extern "C" fn(*mut MatchData, *const u8, *mut usize) -> i32;
type SubstringGetNumberFn =
    unsafe extern "C" fn(*mut MatchData, u32, *mut *mut u8, *mut usize) -> i32;
type SubstringGetNameFn =
    unsafe extern "C" fn(*mut MatchData, *const u8, *mut *mut u8, *mut usize) -> i32;
type SubstringFreeFn = unsafe extern "C" fn(*mut u8);
type SubstringListGetFn =
    unsafe extern "C" fn(*mut MatchData, *mut *mut *mut u8, *mut *mut usize) -> i32;
type SubstringListFreeFn = unsafe extern "C" fn(*mut *mut u8);
type NameNumberFn = unsafe extern "C" fn(*const Code, *const u8) -> i32;
type NameScanFn =
    unsafe extern "C" fn(*const Code, *const u8, *mut *const u8, *mut *const u8) -> i32;

#[derive(Debug, PartialEq, Eq)]
struct SubstringSnapshot {
    copy_results: Vec<(i32, usize, Vec<u8>)>,
    length_results: Vec<(i32, usize)>,
    get_results: Vec<(i32, usize, Vec<u8>)>,
    name_number: i32,
    name_scan: i32,
    list_result: i32,
    list: Vec<Vec<u8>>,
    lengths: Vec<usize>,
    next: (i32, usize, u32),
}

unsafe fn substring_snapshot(
    library: &Library,
    code: *mut Code,
    subject: &[u8],
) -> SubstringSnapshot {
    let create: MatchDataFromPatternFn = sym(library, b"pcre2_match_data_create_from_pattern_8\0");
    let data_free: MatchDataFreeFn = sym(library, b"pcre2_match_data_free_8\0");
    let run: MatchFn = sym(library, b"pcre2_match_8\0");
    let copy_number: SubstringCopyNumberFn = sym(library, b"pcre2_substring_copy_bynumber_8\0");
    let copy_name: SubstringCopyNameFn = sym(library, b"pcre2_substring_copy_byname_8\0");
    let length_number: SubstringLengthNumberFn =
        sym(library, b"pcre2_substring_length_bynumber_8\0");
    let length_name: SubstringLengthNameFn = sym(library, b"pcre2_substring_length_byname_8\0");
    let get_number: SubstringGetNumberFn = sym(library, b"pcre2_substring_get_bynumber_8\0");
    let get_name: SubstringGetNameFn = sym(library, b"pcre2_substring_get_byname_8\0");
    let string_free: SubstringFreeFn = sym(library, b"pcre2_substring_free_8\0");
    let list_get: SubstringListGetFn = sym(library, b"pcre2_substring_list_get_8\0");
    let list_free: SubstringListFreeFn = sym(library, b"pcre2_substring_list_free_8\0");
    let number_from_name: NameNumberFn = sym(library, b"pcre2_substring_number_from_name_8\0");
    let scan: NameScanFn = sym(library, b"pcre2_substring_nametable_scan_8\0");
    let next_fn: NextMatchFn = sym(library, b"pcre2_next_match_8\0");

    let data = create(code, ptr::null_mut());
    assert!(!data.is_null());
    assert!(
        run(
            code,
            subject.as_ptr(),
            subject.len(),
            0,
            0,
            data,
            ptr::null_mut(),
        ) > 0
    );

    let mut copy_results = Vec::new();
    for number in [0, 1, 2, 3, 99] {
        for capacity in [0, 1, 4, 32] {
            let mut output = vec![0xa5_u8; capacity.max(1)];
            let mut length = capacity;
            let rc = copy_number(data, number, output.as_mut_ptr(), &mut length);
            copy_results.push((rc, length, output));
        }
    }
    for name in [b"word\0".as_slice(), b"digits\0", b"missing\0"] {
        let mut output = vec![0xa5_u8; 32];
        let mut length = output.len();
        let rc = copy_name(data, name.as_ptr(), output.as_mut_ptr(), &mut length);
        copy_results.push((rc, length, output));
    }

    let mut length_results = Vec::new();
    for number in [0, 1, 2, 3, 99] {
        let mut length = usize::MAX;
        let rc = length_number(data, number, &mut length);
        length_results.push((rc, length));
    }
    for name in [b"word\0".as_slice(), b"digits\0", b"missing\0"] {
        let mut length = usize::MAX;
        let rc = length_name(data, name.as_ptr(), &mut length);
        length_results.push((rc, length));
    }

    let mut get_results = Vec::new();
    for number in [0, 1, 2, 3, 99] {
        let mut output = ptr::null_mut();
        let mut length = usize::MAX;
        let rc = get_number(data, number, &mut output, &mut length);
        let bytes = if rc >= 0 {
            std::slice::from_raw_parts(output, length + 1).to_vec()
        } else {
            Vec::new()
        };
        get_results.push((rc, length, bytes));
        string_free(output);
    }
    for name in [b"word\0".as_slice(), b"digits\0", b"missing\0"] {
        let mut output = ptr::null_mut();
        let mut length = usize::MAX;
        let rc = get_name(data, name.as_ptr(), &mut output, &mut length);
        let bytes = if rc >= 0 {
            std::slice::from_raw_parts(output, length + 1).to_vec()
        } else {
            Vec::new()
        };
        get_results.push((rc, length, bytes));
        string_free(output);
    }

    let name_number = number_from_name(code, b"word\0".as_ptr());
    let mut first = ptr::null();
    let mut last = ptr::null();
    let name_scan = scan(code, b"word\0".as_ptr(), &mut first, &mut last);

    let mut list_pointer = ptr::null_mut();
    let mut lengths_pointer = ptr::null_mut();
    let list_result = list_get(data, &mut list_pointer, &mut lengths_pointer);
    let mut list = Vec::new();
    let mut lengths = Vec::new();
    if list_result >= 0 {
        let mut index = 0;
        while !(*list_pointer.add(index)).is_null() {
            let length = *lengths_pointer.add(index);
            lengths.push(length);
            list.push(std::slice::from_raw_parts(*list_pointer.add(index), length + 1).to_vec());
            index += 1;
        }
    }
    list_free(list_pointer);

    let mut start = usize::MAX;
    let mut options = u32::MAX;
    let next_rc = next_fn(data, &mut start, &mut options);
    data_free(data);

    SubstringSnapshot {
        copy_results,
        length_results,
        get_results,
        name_number,
        name_scan,
        list_result,
        list,
        lengths,
        next: (next_rc, start, options),
    }
}

#[test]
fn substring_lists_names_and_next_match_state_match() {
    unsafe {
        let libraries = Libraries::open();
        let pattern = b"(?<word>[A-Za-z]+)-(?<digits>[0-9]+)";
        let subject = b"prefix abc-123 suffix";
        let (c_code, rust_code) = compile_pair(&libraries, pattern, 0);
        assert_eq!(
            substring_snapshot(&libraries.rust, rust_code, subject),
            substring_snapshot(&libraries.c, c_code, subject)
        );
        free_code(&libraries.c, c_code);
        free_code(&libraries.rust, rust_code);
    }
}

type EnumerateCallback = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
type EnumerateFn = unsafe extern "C" fn(*const Code, Option<EnumerateCallback>, *mut c_void) -> i32;

unsafe extern "C" fn count_callout(_: *mut c_void, data: *mut c_void) -> i32 {
    let count = data.cast::<u32>();
    unsafe { *count += 1 };
    0
}

unsafe fn enumerate_snapshot(library: &Library, code: *const Code) -> (i32, u32) {
    let enumerate: EnumerateFn = sym(library, b"pcre2_callout_enumerate_8\0");
    let mut count = 0_u32;
    let rc = enumerate(
        code,
        Some(count_callout),
        (&mut count as *mut u32).cast::<c_void>(),
    );
    (rc, count)
}

#[test]
fn callout_enumeration_matches() {
    unsafe {
        let libraries = Libraries::open();
        let (c_code, rust_code) = compile_pair(&libraries, b"(?C1)a(?C\"named\")b(?C255)", 0);
        assert_eq!(
            enumerate_snapshot(&libraries.rust, rust_code),
            enumerate_snapshot(&libraries.c, c_code)
        );
        free_code(&libraries.c, c_code);
        free_code(&libraries.rust, rust_code);
    }
}

type SerializeEncodeFn = unsafe extern "C" fn(
    *const *const Code,
    i32,
    *mut *mut u8,
    *mut usize,
    *mut GeneralContext,
) -> i32;
type SerializeDecodeFn =
    unsafe extern "C" fn(*mut *mut Code, i32, *const u8, *mut GeneralContext) -> i32;
type SerializeCountFn = unsafe extern "C" fn(*const u8) -> i32;
type SerializeFreeFn = unsafe extern "C" fn(*mut u8);

unsafe fn serialize_snapshot(
    library: &Library,
    codes: &[*mut Code],
) -> (i32, Vec<u8>, i32, Vec<MatchSnapshot>) {
    let encode: SerializeEncodeFn = sym(library, b"pcre2_serialize_encode_8\0");
    let decode: SerializeDecodeFn = sym(library, b"pcre2_serialize_decode_8\0");
    let count: SerializeCountFn = sym(library, b"pcre2_serialize_get_number_of_codes_8\0");
    let serialized_free: SerializeFreeFn = sym(library, b"pcre2_serialize_free_8\0");
    let mut bytes_pointer = ptr::null_mut();
    let mut size = 0;
    let pointers: Vec<*const Code> = codes.iter().map(|code| *code as *const Code).collect();
    let encoded = encode(
        pointers.as_ptr(),
        pointers.len() as i32,
        &mut bytes_pointer,
        &mut size,
        ptr::null_mut(),
    );
    let bytes = if encoded > 0 {
        std::slice::from_raw_parts(bytes_pointer, size).to_vec()
    } else {
        Vec::new()
    };
    let number = if bytes.is_empty() {
        count(ptr::null())
    } else {
        count(bytes.as_ptr())
    };
    let mut decoded = vec![ptr::null_mut(); codes.len()];
    let decoded_count = if bytes.is_empty() {
        0
    } else {
        decode(
            decoded.as_mut_ptr(),
            decoded.len() as i32,
            bytes.as_ptr(),
            ptr::null_mut(),
        )
    };
    let mut matches = Vec::new();
    if decoded_count > 0 {
        for code in decoded {
            matches.push(match_snapshot(
                library,
                code,
                b"aaabbb",
                6,
                0,
                0,
                ptr::null_mut(),
            ));
            free_code(library, code);
        }
    }
    serialized_free(bytes_pointer);
    (encoded, bytes, number, matches)
}

#[test]
fn serialization_bytes_counts_and_decoded_behavior_match() {
    unsafe {
        let libraries = Libraries::open();
        let (c_one, rust_one) = compile_pair(&libraries, b"a+", 0);
        let (c_two, rust_two) = compile_pair(&libraries, b"(a+)(b*)", 0);
        let c_snapshot = serialize_snapshot(&libraries.c, &[c_one, c_two]);
        let rust_snapshot = serialize_snapshot(&libraries.rust, &[rust_one, rust_two]);
        assert_eq!(rust_snapshot, c_snapshot);
        free_code(&libraries.c, c_one);
        free_code(&libraries.c, c_two);
        free_code(&libraries.rust, rust_one);
        free_code(&libraries.rust, rust_two);
    }
}

type SubstituteFn = unsafe extern "C" fn(
    *const Code,
    *const u8,
    usize,
    usize,
    u32,
    *mut MatchData,
    *mut MatchContext,
    *const u8,
    usize,
    *mut u8,
    *mut usize,
) -> i32;

unsafe fn substitute_snapshot(
    library: &Library,
    code: *const Code,
    subject: &[u8],
    replacement: &[u8],
    options: u32,
    capacity: usize,
) -> (i32, usize, Vec<u8>) {
    let substitute: SubstituteFn = sym(library, b"pcre2_substitute_8\0");
    let mut output = vec![0xa5_u8; capacity.max(1)];
    let mut length = capacity;
    let rc = substitute(
        code,
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
    (rc, length, output)
}

#[test]
fn substitution_modes_replacements_and_capacities_match() {
    unsafe {
        let libraries = Libraries::open();
        let (c_code, rust_code) = compile_pair(&libraries, b"(?<word>[A-Za-z]+)-([0-9]+)", 0);
        let subject = b"abc-12 def-34";
        for replacement in [
            b"".as_slice(),
            b"literal",
            b"${word}",
            b"$1:$2",
            b"\\U$1\\E",
            b"$99",
        ] {
            for options in [
                0,
                0x0000_0100,
                0x0000_0200,
                0x0000_0400,
                0x0000_0800,
                0x0000_1000,
                0x0000_8000,
                0x0001_0000,
                0x0002_0000,
            ] {
                for capacity in [0, 1, 8, 32, 128] {
                    assert_eq!(
                        substitute_snapshot(
                            &libraries.rust,
                            rust_code,
                            subject,
                            replacement,
                            options,
                            capacity,
                        ),
                        substitute_snapshot(
                            &libraries.c,
                            c_code,
                            subject,
                            replacement,
                            options,
                            capacity,
                        ),
                        "replacement {:?}, options {options:#x}, capacity {capacity}",
                        String::from_utf8_lossy(replacement)
                    );
                }
            }
        }
        free_code(&libraries.c, c_code);
        free_code(&libraries.rust, rust_code);
    }
}

type ConvertFn = unsafe extern "C" fn(
    *const u8,
    usize,
    u32,
    *mut *mut u8,
    *mut usize,
    *mut ConvertContext,
) -> i32;
type ConvertedFreeFn = unsafe extern "C" fn(*mut u8);

unsafe fn convert_snapshot(
    library: &Library,
    pattern: &[u8],
    options: u32,
) -> (i32, usize, Vec<u8>) {
    let convert: ConvertFn = sym(library, b"pcre2_pattern_convert_8\0");
    let free: ConvertedFreeFn = sym(library, b"pcre2_converted_pattern_free_8\0");
    let mut output = ptr::null_mut();
    let mut length = usize::MAX;
    let rc = convert(
        pattern.as_ptr(),
        pattern.len(),
        options,
        &mut output,
        &mut length,
        ptr::null_mut(),
    );
    let bytes = if rc == 0 {
        std::slice::from_raw_parts(output, length + 1).to_vec()
    } else {
        Vec::new()
    };
    free(output);
    (rc, length, bytes)
}

#[test]
fn pattern_conversion_modes_shapes_and_errors_match() {
    unsafe {
        let libraries = Libraries::open();
        for pattern in [
            b"".as_slice(),
            b"abc",
            b"a*b?.[0-9]",
            b"foo/**/bar",
            b"\\(a\\)\\{1,3\\}",
            b"[\xff]",
        ] {
            for options in [1, 3, 4, 8, 0x10, 0x30, 0x50, 0, u32::MAX] {
                assert_eq!(
                    convert_snapshot(&libraries.rust, pattern, options),
                    convert_snapshot(&libraries.c, pattern, options),
                    "pattern {:?}, options {options:#x}",
                    String::from_utf8_lossy(pattern)
                );
            }
        }
    }
}

#[test]
fn direct_match_data_sizes_and_jit_match_stub_match() {
    unsafe {
        let libraries = Libraries::open();
        let mut snapshots = Vec::new();
        for library in [&libraries.c, &libraries.rust] {
            let create: MatchDataCreateFn = sym(library, b"pcre2_match_data_create_8\0");
            let free: MatchDataFreeFn = sym(library, b"pcre2_match_data_free_8\0");
            let count: GetOvectorCountFn = sym(library, b"pcre2_get_ovector_count_8\0");
            let size: GetSizeFn = sym(library, b"pcre2_get_match_data_size_8\0");
            let mut values = Vec::new();
            for requested in [0, 1, 2, 255, 65535, 65536, u32::MAX] {
                let data = create(requested, ptr::null_mut());
                assert!(!data.is_null());
                values.push((count(data), size(data)));
                free(data);
            }
            snapshots.push(values);
        }
        assert_eq!(snapshots[1], snapshots[0]);

        let (c_code, rust_code) = compile_pair(&libraries, b"a+", 0);
        type JitMatchFn = unsafe extern "C" fn(
            *const Code,
            *const u8,
            usize,
            usize,
            u32,
            *mut MatchData,
            *mut MatchContext,
        ) -> i32;
        let mut jit_results = Vec::new();
        for (library, code) in [(&libraries.c, c_code), (&libraries.rust, rust_code)] {
            let create: MatchDataFromPatternFn =
                sym(library, b"pcre2_match_data_create_from_pattern_8\0");
            let free: MatchDataFreeFn = sym(library, b"pcre2_match_data_free_8\0");
            let jit: JitMatchFn = sym(library, b"pcre2_jit_match_8\0");
            let data = create(code, ptr::null_mut());
            jit_results.push(jit(code, b"aaa".as_ptr(), 3, 0, 0, data, ptr::null_mut()));
            free(data);
        }
        assert_eq!(jit_results[1], jit_results[0]);
        free_code(&libraries.c, c_code);
        free_code(&libraries.rust, rust_code);
    }
}

unsafe extern "C" fn failing_malloc(_: usize, _: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}

unsafe fn boundary_error_snapshot(library: &Library) -> Vec<i64> {
    let mut results = Vec::new();

    let general_create: GeneralCreateFn = sym(library, b"pcre2_general_context_create_8\0");
    results.push(
        general_create(Some(failing_malloc), Some(tracked_free), ptr::null_mut()).is_null() as i64,
    );

    let (invalid_compile, invalid_code) =
        compile_snapshot(library, b"a", 1, u32::MAX, ptr::null_mut());
    results.push(invalid_compile.error as i64);
    results.push(invalid_compile.offset as i64);
    results.push(invalid_code.is_null() as i64);
    free_code(library, invalid_code);

    let (valid_compile, code) = compile_snapshot(library, b"a", 1, 0, ptr::null_mut());
    assert!(!code.is_null(), "{valid_compile:?}");
    let info: PatternInfoFn = sym(library, b"pcre2_pattern_info_8\0");
    let mut aligned = [0_usize; 2];
    results.push(info(ptr::null(), 0, aligned.as_mut_ptr().cast()) as i64);
    results.push(info(code, u32::MAX, aligned.as_mut_ptr().cast()) as i64);
    results.push(info(ptr::null(), u32::MAX, ptr::null_mut()) as i64);

    let create_data: MatchDataFromPatternFn =
        sym(library, b"pcre2_match_data_create_from_pattern_8\0");
    let free_data: MatchDataFreeFn = sym(library, b"pcre2_match_data_free_8\0");
    let dfa: DfaMatchFn = sym(library, b"pcre2_dfa_match_8\0");
    let data = create_data(code, ptr::null_mut());
    let mut workspace = [0_i32; 32];
    results.push(dfa(
        code,
        b"a".as_ptr(),
        1,
        0,
        0,
        data,
        ptr::null_mut(),
        ptr::null_mut(),
        32,
    ) as i64);
    results.push(dfa(
        code,
        b"a".as_ptr(),
        1,
        0,
        u32::MAX,
        data,
        ptr::null_mut(),
        workspace.as_mut_ptr(),
        workspace.len(),
    ) as i64);
    results.push(dfa(
        code,
        b"a".as_ptr(),
        1,
        0,
        0,
        ptr::null_mut(),
        ptr::null_mut(),
        workspace.as_mut_ptr(),
        workspace.len(),
    ) as i64);
    free_data(data);

    let encode: SerializeEncodeFn = sym(library, b"pcre2_serialize_encode_8\0");
    let decode: SerializeDecodeFn = sym(library, b"pcre2_serialize_decode_8\0");
    let count: SerializeCountFn = sym(library, b"pcre2_serialize_get_number_of_codes_8\0");
    let code_pointer = code as *const Code;
    let mut bytes = ptr::null_mut();
    let mut size = 0_usize;
    results.push(encode(ptr::null(), 1, &mut bytes, &mut size, ptr::null_mut()) as i64);
    results.push(encode(&code_pointer, 0, &mut bytes, &mut size, ptr::null_mut()) as i64);
    results.push(count(ptr::null()) as i64);
    let malformed = [0_u32; 4];
    results.push(count(malformed.as_ptr().cast::<u8>()) as i64);
    let mut decoded = ptr::null_mut();
    results.push(decode(
        &mut decoded,
        1,
        malformed.as_ptr().cast::<u8>(),
        ptr::null_mut(),
    ) as i64);
    results.push(decode(&mut decoded, 0, ptr::null(), ptr::null_mut()) as i64);

    free_code(library, code);
    results
}

#[test]
fn generic_public_error_boundaries_return_identical_sentinels() {
    unsafe {
        let libraries = Libraries::open();
        assert_eq!(
            boundary_error_snapshot(&libraries.rust),
            boundary_error_snapshot(&libraries.c)
        );
    }
}

fn parse_upstream_pattern(line: &str) -> Option<(Vec<u8>, usize, u32)> {
    let bytes = line.as_bytes();
    if bytes.first() != Some(&b'/') {
        return None;
    }
    let mut escaped = false;
    let mut closing = None;
    for (index, byte) in bytes.iter().enumerate().skip(1) {
        if *byte == b'/' && !escaped {
            closing = Some(index);
            break;
        }
        if *byte == b'\\' {
            escaped = !escaped;
        } else {
            escaped = false;
        }
    }
    let closing = closing?;
    let modifier_text = &line[closing + 1..];
    let modifiers: Vec<_> = modifier_text
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    let mut pattern = if modifiers.contains(&"hex") {
        let text = std::str::from_utf8(&bytes[1..closing]).ok()?;
        let mut decoded = Vec::new();
        for token in text.split_ascii_whitespace() {
            if token.len() != 2 {
                return None;
            }
            decoded.push(u8::from_str_radix(token, 16).ok()?);
        }
        decoded
    } else {
        bytes[1..closing].to_vec()
    };
    if pattern.len() > 16 * 1024 {
        return None;
    }

    let mut options = 0_u32;
    for modifier in &modifiers {
        options |= match *modifier {
            "i" | "caseless" => 0x0000_0008,
            "s" | "dotall" => 0x0000_0020,
            "dupnames" => 0x0000_0040,
            "x" | "extended" => 0x0000_0080,
            "firstline" => 0x0000_0100,
            "match_unset_backref" => 0x0000_0200,
            "m" | "multiline" => 0x0000_0400,
            "never_ucp" => 0x0000_0800,
            "never_utf" => 0x0000_1000,
            "no_auto_capture" => 0x0000_2000,
            "no_auto_possess" => 0x0000_4000,
            "no_dotstar_anchor" => 0x0000_8000,
            "no_start_optimize" => 0x0001_0000,
            "ucp" => 0x0002_0000,
            "ungreedy" => 0x0004_0000,
            "utf" => 0x0008_0000,
            "never_backslash_c" => 0x0010_0000,
            "alt_circumflex" => 0x0020_0000,
            "alt_verbnames" => 0x0040_0000,
            "use_offset_limit" => 0x0080_0000,
            "extended_more" => 0x0100_0000,
            "literal" => 0x0200_0000,
            "match_invalid_utf" => 0x0400_0000,
            "alt_extended_class" => 0x0800_0000,
            "anchored" => 0x8000_0000,
            _ => 0,
        };
    }
    let length = if modifiers.contains(&"zero_terminate") {
        pattern.push(0);
        ZERO_TERMINATED
    } else {
        pattern.len()
    };
    Some((pattern, length, options))
}

unsafe fn corpus_compile_result(
    library: &Library,
    pattern: &[u8],
    length: usize,
    options: u32,
) -> (i32, usize, bool) {
    let compile: CompileFn = sym(library, b"pcre2_compile_8\0");
    let mut error = i32::MIN;
    let mut offset = usize::MAX;
    let code = compile(
        pattern.as_ptr(),
        length,
        options,
        &mut error,
        &mut offset,
        ptr::null_mut(),
    );
    let failed = code.is_null();
    free_code(library, code);
    (error, offset, failed)
}

#[test]
fn upstream_pcre2_pattern_corpus_compiles_identically() {
    std::thread::Builder::new()
        .name("pcre2-upstream-corpus".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| unsafe {
            let libraries = Libraries::open();
            let directory =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/upstream");
            let mut paths: Vec<_> = fs::read_dir(directory)
                .expect("read upstream test data")
                .map(|entry| entry.expect("test data entry").path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("testinput"))
                })
                .collect();
            paths.sort();

            let mut cases = 0_usize;
            let mut compile_errors = std::collections::BTreeSet::new();
            for path in paths {
                let raw_contents = fs::read(&path).expect("read upstream test input");
                let contents = String::from_utf8_lossy(&raw_contents);
                for (line_number, line) in contents.lines().enumerate() {
                    let Some((pattern, length, options)) = parse_upstream_pattern(line) else {
                        continue;
                    };
                    let c = corpus_compile_result(&libraries.c, &pattern, length, options);
                    let rust = corpus_compile_result(&libraries.rust, &pattern, length, options);
                    assert_eq!(
                        rust,
                        c,
                        "{}:{} options={options:#x}",
                        path.display(),
                        line_number + 1
                    );
                    if c.2 && c.0 >= 100 {
                        compile_errors.insert(c.0);
                    }
                    cases += 1;
                }
            }
            assert!(cases >= 8_000, "only parsed {cases} upstream patterns");
            assert!(
                compile_errors.len() >= 80,
                "only covered {} distinct compile errors: {compile_errors:?}",
                compile_errors.len()
            );
        })
        .expect("spawn corpus thread")
        .join()
        .expect("corpus thread");
}
