use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;

#[repr(C)]
#[derive(Debug)]
struct StringBuffer {
    data: *mut c_char,
    capacity: c_int,
    length: c_int,
}

type CreateBuffer = unsafe extern "C" fn(c_int) -> *mut StringBuffer;
type AppendToBuffer = unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int;
type DestroyBuffer = unsafe extern "C" fn(*mut StringBuffer);
type GetOperationName = unsafe extern "C" fn(c_int) -> *const c_char;
type PerformOperation = unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int;
type Buffapp = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type FailAllocArm = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-pddoiK.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libbuffapp_lib.so")
}

fn libraries() -> (Library, Library) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing release Rust shared library: {}",
        rust_path.display()
    );
    unsafe {
        (
            Library::new(c_path).expect("load C library"),
            Library::new(rust_path).expect("load Rust library"),
        )
    }
}

unsafe fn buffer_bytes(buffer: *mut StringBuffer) -> Vec<u8> {
    if buffer.is_null() || unsafe { (*buffer).data.is_null() } {
        return Vec::new();
    }
    unsafe { CStr::from_ptr((*buffer).data) }
        .to_bytes()
        .to_vec()
}

unsafe fn compare_buffers(c_buffer: *mut StringBuffer, rust_buffer: *mut StringBuffer) {
    assert_eq!(c_buffer.is_null(), rust_buffer.is_null());
    if c_buffer.is_null() {
        return;
    }
    unsafe {
        assert_eq!((*c_buffer).capacity, (*rust_buffer).capacity);
        assert_eq!((*c_buffer).length, (*rust_buffer).length);
        assert_eq!(buffer_bytes(c_buffer), buffer_bytes(rust_buffer));
    }
}

unsafe fn compare_append_sequence(capacity: c_int, strings: &[CString]) {
    let (c_library, rust_library) = libraries();
    let c_create = unsafe { c_library.get::<CreateBuffer>(b"create_buffer") }.unwrap();
    let r_create = unsafe { rust_library.get::<CreateBuffer>(b"create_buffer") }.unwrap();
    let c_append = unsafe { c_library.get::<AppendToBuffer>(b"append_to_buffer") }.unwrap();
    let r_append = unsafe { rust_library.get::<AppendToBuffer>(b"append_to_buffer") }.unwrap();
    let c_destroy = unsafe { c_library.get::<DestroyBuffer>(b"destroy_buffer") }.unwrap();
    let r_destroy = unsafe { rust_library.get::<DestroyBuffer>(b"destroy_buffer") }.unwrap();

    let c_buffer = unsafe { c_create(capacity) };
    let rust_buffer = unsafe { r_create(capacity) };
    unsafe { compare_buffers(c_buffer, rust_buffer) };

    for string in strings {
        let c_result = unsafe { c_append(c_buffer, string.as_ptr()) };
        let rust_result = unsafe { r_append(rust_buffer, string.as_ptr()) };
        assert_eq!(c_result, rust_result);
        unsafe { compare_buffers(c_buffer, rust_buffer) };
    }

    unsafe {
        c_destroy(c_buffer);
        r_destroy(rust_buffer);
    }
}

fn next_random(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

fn random_i32(state: &mut u64, minimum: i32, maximum: i32) -> i32 {
    let width = (maximum as i64 - minimum as i64 + 1) as u32;
    minimum + (next_random(state) % width) as i32
}

unsafe fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);

        let result = operation();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = read(fds[0], chunk.as_mut_ptr().cast(), chunk.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..count as usize]);
        }
        assert_eq!(close(fds[0]), 0);
        (result, output)
    }
}

#[derive(Clone, Copy)]
enum OperationClass {
    Unknown,
    Add,
    Subtract,
    Multiply,
    Divide,
}

const OPERATION_CLASSES: [OperationClass; 5] = [
    OperationClass::Unknown,
    OperationClass::Add,
    OperationClass::Subtract,
    OperationClass::Multiply,
    OperationClass::Divide,
];

fn parameter_for(class: OperationClass, state: &mut u64) -> i32 {
    match class {
        OperationClass::Unknown => {
            let quotient = random_i32(state, 1, 40);
            let remainder = random_i32(state, 1, 3);
            -(quotient * 4 + remainder)
        }
        OperationClass::Add => random_i32(state, -40, 40) * 4,
        OperationClass::Subtract => random_i32(state, 0, 40) * 4 + 1,
        OperationClass::Multiply => random_i32(state, 0, 40) * 4 + 2,
        OperationClass::Divide => random_i32(state, 0, 40) * 4 + 3,
    }
}

fn compile_fail_allocator() -> PathBuf {
    let output = manifest_dir().join("target/fail_alloc_test.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11", "-O2"])
        .arg(manifest_dir().join("tests/fail_alloc.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .expect("run C compiler for allocator shim");
    assert!(status.success(), "allocator shim compilation failed");
    assert!(output.is_file());
    output
}

fn run_child(target: &str, case: &str, preload: Option<&Path>) -> Output {
    let mut command = Command::new(env::current_exe().unwrap());
    command
        .args(["--exact", "isolated_ffi_child", "--nocapture"])
        .env("DIFF_CHILD_TARGET", target)
        .env("DIFF_CHILD_CASE", case);
    if let Some(preload) = preload {
        command.env("LD_PRELOAD", preload);
    }
    command.output().expect("run isolated FFI child")
}

unsafe fn child_library() -> Library {
    let path = match env::var("DIFF_CHILD_TARGET").unwrap().as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown child target {other}"),
    };
    unsafe { Library::new(path) }.unwrap()
}

#[test]
fn isolated_ffi_child() {
    let Ok(case) = env::var("DIFF_CHILD_CASE") else {
        return;
    };
    let library = unsafe { child_library() };

    unsafe {
        match case.as_str() {
            "null_append_buffer" => {
                let append = library.get::<AppendToBuffer>(b"append_to_buffer").unwrap();
                let text = CString::new("x").unwrap();
                append(ptr::null_mut(), text.as_ptr());
            }
            "null_append_string" => {
                let create = library.get::<CreateBuffer>(b"create_buffer").unwrap();
                let append = library.get::<AppendToBuffer>(b"append_to_buffer").unwrap();
                let buffer = create(8);
                append(buffer, ptr::null());
            }
            "null_operation" => {
                let perform = library
                    .get::<PerformOperation>(b"perform_operation")
                    .unwrap();
                perform(1, 2, ptr::null());
            }
            "divide_overflow" => {
                let perform = library
                    .get::<PerformOperation>(b"perform_operation")
                    .unwrap();
                let divide = CString::new("divide").unwrap();
                perform(c_int::MIN, -1, divide.as_ptr());
            }
            "buffapp_divide_overflow" => {
                let buffapp = library.get::<Buffapp>(b"buffapp").unwrap();
                buffapp(0, 1_073_741_825, 0, 1_073_741_823);
            }
            "fail_object_alloc" | "fail_data_alloc" | "fail_realloc" => {
                let shim_path = env::var_os("LD_PRELOAD").unwrap();
                let shim = Library::new(shim_path).unwrap();
                let arm = shim.get::<FailAllocArm>(b"fail_alloc_arm").unwrap();
                let create = library.get::<CreateBuffer>(b"create_buffer").unwrap();

                if case == "fail_object_alloc" {
                    arm(1, 1);
                    assert!(create(32).is_null());
                } else if case == "fail_data_alloc" {
                    arm(1, 2);
                    assert!(create(32).is_null());
                } else {
                    let append = library.get::<AppendToBuffer>(b"append_to_buffer").unwrap();
                    let destroy = library.get::<DestroyBuffer>(b"destroy_buffer").unwrap();
                    let initial = CString::new("a").unwrap();
                    let growth = CString::new("bc").unwrap();
                    let buffer = create(2);
                    assert!(!buffer.is_null());
                    assert_eq!(append(buffer, initial.as_ptr()), 0);
                    let old_data = (*buffer).data;
                    let old_capacity = (*buffer).capacity;
                    let old_length = (*buffer).length;
                    let old_bytes = buffer_bytes(buffer);
                    arm(2, 1);
                    assert_eq!(append(buffer, growth.as_ptr()), -1);
                    assert_eq!((*buffer).data, old_data);
                    assert_eq!((*buffer).capacity, old_capacity);
                    assert_eq!((*buffer).length, old_length);
                    assert_eq!(buffer_bytes(buffer), old_bytes);
                    destroy(buffer);
                }
            }
            other => panic!("unknown child case {other}"),
        }
    }
}

#[test]
fn differential_all_configurations_and_errors() {
    let (c_library, rust_library) = libraries();

    unsafe {
        let c_create = c_library.get::<CreateBuffer>(b"create_buffer").unwrap();
        let r_create = rust_library.get::<CreateBuffer>(b"create_buffer").unwrap();
        let c_destroy = c_library.get::<DestroyBuffer>(b"destroy_buffer").unwrap();
        let r_destroy = rust_library
            .get::<DestroyBuffer>(b"destroy_buffer")
            .unwrap();

        for capacity in [0, 1, 2, 7, 32, 4096, c_int::MAX, -1] {
            let c_buffer = c_create(capacity);
            let rust_buffer = r_create(capacity);
            compare_buffers(c_buffer, rust_buffer);
            c_destroy(c_buffer);
            r_destroy(rust_buffer);
        }

        c_destroy(ptr::null_mut());
        r_destroy(ptr::null_mut());

        let c_null_data = malloc(std::mem::size_of::<StringBuffer>()).cast::<StringBuffer>();
        let r_null_data = malloc(std::mem::size_of::<StringBuffer>()).cast::<StringBuffer>();
        assert!(!c_null_data.is_null() && !r_null_data.is_null());
        ptr::write(
            c_null_data,
            StringBuffer {
                data: ptr::null_mut(),
                capacity: 0,
                length: 0,
            },
        );
        ptr::write(
            r_null_data,
            StringBuffer {
                data: ptr::null_mut(),
                capacity: 0,
                length: 0,
            },
        );
        c_destroy(c_null_data);
        r_destroy(r_null_data);
    }

    unsafe {
        compare_append_sequence(1, &[CString::new("").unwrap()]);
        for length in 1..=32 {
            let exact = CString::new("x".repeat(length)).unwrap();
            compare_append_sequence((length + 1) as c_int, &[exact]);

            let growth = CString::new("y".repeat(length)).unwrap();
            compare_append_sequence(length as c_int, &[growth]);
        }

        let mut seed = 0x71d4_62a9_05ce_b833;
        for _ in 0..64 {
            let capacity = random_i32(&mut seed, 2, 48);
            let first_length = random_i32(&mut seed, 1, (capacity - 1).max(1));
            let remaining = capacity - first_length - 1;
            let second_length = random_i32(&mut seed, 1, 32);
            let first = CString::new("a".repeat(first_length as usize)).unwrap();
            let second = CString::new("b".repeat(second_length as usize)).unwrap();
            compare_append_sequence(capacity, &[first, second]);
            if remaining > 0 {
                let no_growth = CString::new("c".repeat(remaining as usize)).unwrap();
                compare_append_sequence(
                    capacity,
                    &[
                        CString::new("a".repeat(first_length as usize)).unwrap(),
                        no_growth,
                    ],
                );
            }
        }

        let repeated: Vec<CString> = (0..40)
            .map(|index| CString::new("z".repeat(index % 11 + 1)).unwrap())
            .collect();
        compare_append_sequence(2, &repeated);
    }

    unsafe {
        let c_get = c_library
            .get::<GetOperationName>(b"get_operation_name")
            .unwrap();
        let r_get = rust_library
            .get::<GetOperationName>(b"get_operation_name")
            .unwrap();
        for code in [
            c_int::MIN,
            -1000,
            -4,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            1000,
            c_int::MAX,
        ] {
            let c_name = CStr::from_ptr(c_get(code)).to_bytes();
            let rust_name = CStr::from_ptr(r_get(code)).to_bytes();
            assert_eq!(c_name, rust_name, "operation code {code}");
        }
    }

    unsafe {
        let c_perform = c_library
            .get::<PerformOperation>(b"perform_operation")
            .unwrap();
        let r_perform = rust_library
            .get::<PerformOperation>(b"perform_operation")
            .unwrap();
        let operations = ["add", "subtract", "multiply", "divide", "unknown", ""];
        let mut seed = 0x5ef1_977b_249a_c308;
        for operation in operations {
            let operation = CString::new(operation).unwrap();
            for index in 0..512 {
                let first = random_i32(&mut seed, -10_000, 10_000);
                let mut second = random_i32(&mut seed, -10_000, 10_000);
                if operation.as_bytes() == b"divide" && index % 17 == 0 {
                    second = 0;
                }
                let c_result = c_perform(first, second, operation.as_ptr());
                let rust_result = r_perform(first, second, operation.as_ptr());
                assert_eq!(
                    c_result, rust_result,
                    "perform_operation({first}, {second}, {:?})",
                    operation
                );
            }
        }

        for (operation, first, second) in [
            ("add", c_int::MAX, 1),
            ("add", c_int::MIN, -1),
            ("subtract", c_int::MIN, 1),
            ("subtract", c_int::MAX, -1),
            ("multiply", c_int::MAX, 2),
            ("multiply", c_int::MIN, -1),
        ] {
            let operation = CString::new(operation).unwrap();
            assert_eq!(
                c_perform(first, second, operation.as_ptr()),
                r_perform(first, second, operation.as_ptr())
            );
        }
    }

    unsafe {
        let c_buffapp = c_library.get::<Buffapp>(b"buffapp").unwrap();
        let r_buffapp = rust_library.get::<Buffapp>(b"buffapp").unwrap();
        let mut seed = 0x96ae_07cd_318b_5f42;
        for first_class in OPERATION_CLASSES {
            for second_class in OPERATION_CLASSES {
                for index in 0..48 {
                    let parameter1 = parameter_for(first_class, &mut seed);
                    let parameter3 = parameter_for(second_class, &mut seed);
                    let parameter2 = if index % 12 == 0 {
                        0
                    } else {
                        random_i32(&mut seed, -50, 50)
                    };
                    let parameter4 = if index % 12 == 1 {
                        0
                    } else {
                        random_i32(&mut seed, -50, 50)
                    };

                    let (c_result, c_stdout) = capture_stdout(|| {
                        c_buffapp(parameter1, parameter2, parameter3, parameter4)
                    });
                    let (rust_result, rust_stdout) = capture_stdout(|| {
                        r_buffapp(parameter1, parameter2, parameter3, parameter4)
                    });
                    assert_eq!(
                        c_result, rust_result,
                        "buffapp({parameter1}, {parameter2}, {parameter3}, {parameter4})"
                    );
                    assert_eq!(
                        c_stdout, rust_stdout,
                        "stdout for buffapp({parameter1}, {parameter2}, {parameter3}, {parameter4})"
                    );
                }
            }
        }

        for _ in 0..256 {
            let parameter1 = next_random(&mut seed) as i32;
            let parameter2 = next_random(&mut seed) as i32;
            let parameter3 = next_random(&mut seed) as i32;
            let parameter4 = next_random(&mut seed) as i32;
            let (c_result, c_stdout) =
                capture_stdout(|| c_buffapp(parameter1, parameter2, parameter3, parameter4));
            let (rust_result, rust_stdout) =
                capture_stdout(|| r_buffapp(parameter1, parameter2, parameter3, parameter4));
            assert_eq!(c_result, rust_result);
            assert_eq!(c_stdout, rust_stdout);
        }
    }

    let allocator = compile_fail_allocator();
    for case in ["fail_object_alloc", "fail_data_alloc", "fail_realloc"] {
        for target in ["c", "rust"] {
            let output = run_child(target, case, Some(&allocator));
            assert!(
                output.status.success(),
                "{target} {case} failed: status={:?}\nstdout={}\nstderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    for case in [
        "null_append_buffer",
        "null_append_string",
        "null_operation",
        "divide_overflow",
        "buffapp_divide_overflow",
    ] {
        let c_output = run_child("c", case, None);
        let rust_output = run_child("rust", case, None);
        assert!(!c_output.status.success(), "C unexpectedly accepted {case}");
        assert!(
            !rust_output.status.success(),
            "Rust unexpectedly accepted {case}"
        );
        assert_eq!(
            c_output.status.signal(),
            rust_output.status.signal(),
            "different termination signal for {case}: C={:?}, Rust={:?}",
            c_output.status,
            rust_output.status
        );
    }

    fs::remove_file(allocator).unwrap();
}
