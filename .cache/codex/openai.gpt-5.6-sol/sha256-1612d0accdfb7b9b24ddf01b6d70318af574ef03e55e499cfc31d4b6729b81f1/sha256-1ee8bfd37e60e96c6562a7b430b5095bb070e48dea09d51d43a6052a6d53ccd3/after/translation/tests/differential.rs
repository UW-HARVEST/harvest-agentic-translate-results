use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, ptr};

#[repr(C)]
#[derive(Clone, Copy)]
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

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct Api {
    library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        Self {
            library: unsafe { Library::new(path) }
                .unwrap_or_else(|error| panic!("load {}: {error}", path.display())),
        }
    }

    unsafe fn function<T: Copy>(&self, name: &[u8]) -> T {
        unsafe {
            *self.library.get::<T>(name).unwrap_or_else(|error| {
                panic!("load {:?}: {error}", CStr::from_bytes_with_nul(name))
            })
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    unsafe fn load() -> Self {
        Self {
            c: unsafe { Api::load(&c_library_path()) },
            rust: unsafe { Api::load(&rust_library_path()) },
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum OperationClass {
    Add,
    Subtract,
    Multiply,
    Divide,
    DivideByZero,
    Unknown,
}

impl OperationClass {
    fn always_zero(self) -> bool {
        matches!(self, Self::DivideByZero | Self::Unknown)
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn range(&mut self, start: i32, end_inclusive: i32) -> i32 {
        start + (self.next_u32() % (end_inclusive - start + 1) as u32) as i32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-s5WMbT.so")
}

fn rust_library_path() -> PathBuf {
    let profile_dir = env::current_exe()
        .expect("current test executable")
        .parent()
        .expect("deps directory")
        .parent()
        .expect("target profile directory")
        .to_path_buf();
    [
        profile_dir.join("libbuffapp_lib.so"),
        manifest_dir().join("target/release/libbuffapp_lib.so"),
        manifest_dir().join("target/debug/libbuffapp_lib.so"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .unwrap_or_else(|| profile_dir.join("libbuffapp_lib.so"))
}

unsafe fn snapshot(buffer: *mut StringBuffer) -> (i32, i32, Vec<u8>) {
    assert!(!buffer.is_null());
    let buffer = unsafe { &*buffer };
    let bytes = unsafe { CStr::from_ptr(buffer.data) }
        .to_bytes_with_nul()
        .to_vec();
    (buffer.capacity, buffer.length, bytes)
}

unsafe fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut fds = [-1, -1];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);

        let result = call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut chunk = [0_u8; 512];
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

unsafe fn compare_append_sequence(pair: &Pair, capacity: i32, strings: &[CString]) {
    let c_create: CreateBuffer = unsafe { pair.c.function(b"create_buffer\0") };
    let r_create: CreateBuffer = unsafe { pair.rust.function(b"create_buffer\0") };
    let c_append: AppendToBuffer = unsafe { pair.c.function(b"append_to_buffer\0") };
    let r_append: AppendToBuffer = unsafe { pair.rust.function(b"append_to_buffer\0") };
    let c_destroy: DestroyBuffer = unsafe { pair.c.function(b"destroy_buffer\0") };
    let r_destroy: DestroyBuffer = unsafe { pair.rust.function(b"destroy_buffer\0") };

    let c_buffer = unsafe { c_create(capacity) };
    let r_buffer = unsafe { r_create(capacity) };
    assert_eq!(c_buffer.is_null(), r_buffer.is_null());
    assert!(!c_buffer.is_null());

    for string in strings {
        let c_result = unsafe { c_append(c_buffer, string.as_ptr()) };
        let r_result = unsafe { r_append(r_buffer, string.as_ptr()) };
        assert_eq!(c_result, r_result);
        assert_eq!(unsafe { snapshot(c_buffer) }, unsafe { snapshot(r_buffer) });
    }

    unsafe {
        c_destroy(c_buffer);
        r_destroy(r_buffer);
    }
}

fn random_ascii(rng: &mut Rng, length: usize) -> CString {
    let bytes = (0..length)
        .map(|_| b'a' + (rng.next_u32() % 26) as u8)
        .collect::<Vec<_>>();
    CString::new(bytes).unwrap()
}

fn pair_for_class(class: OperationClass, zero: bool, rng: &mut Rng) -> (i32, i32) {
    let scale = rng.range(1, 6);
    match class {
        OperationClass::Add => {
            let first = 4 * scale;
            let second = if zero { -first } else { rng.range(1, 12) };
            (first, second)
        }
        OperationClass::Subtract => {
            let first = 4 * scale + 1;
            let second = if zero { first } else { -rng.range(1, 12) };
            (first, second)
        }
        OperationClass::Multiply => {
            let first = 4 * scale + 2;
            let second = if zero { 0 } else { rng.range(1, 12) };
            (first, second)
        }
        OperationClass::Divide => {
            let first = 4 * scale + 3;
            let second = if zero {
                first + rng.range(1, 12)
            } else {
                rng.range(1, first)
            };
            (first, second)
        }
        OperationClass::DivideByZero => (4 * scale + 3, 0),
        OperationClass::Unknown => {
            const UNKNOWN_CODES: [i32; 6] = [-1, -2, -3, -5, -6, -7];
            (
                UNKNOWN_CODES[(rng.next_u32() as usize) % UNKNOWN_CODES.len()],
                rng.range(-12, 12),
            )
        }
    }
}

fn intermediate(class: OperationClass, first: i32, second: i32) -> i32 {
    match class {
        OperationClass::Add => first + second,
        OperationClass::Subtract => first - second,
        OperationClass::Multiply => first * second,
        OperationClass::Divide if second != 0 => first / second,
        OperationClass::Divide | OperationClass::DivideByZero | OperationClass::Unknown => 0,
    }
}

unsafe fn valid_surface(pair: &Pair) {
    let c_create: CreateBuffer = unsafe { pair.c.function(b"create_buffer\0") };
    let r_create: CreateBuffer = unsafe { pair.rust.function(b"create_buffer\0") };
    let c_destroy: DestroyBuffer = unsafe { pair.c.function(b"destroy_buffer\0") };
    let r_destroy: DestroyBuffer = unsafe { pair.rust.function(b"destroy_buffer\0") };
    let c_name: GetOperationName = unsafe { pair.c.function(b"get_operation_name\0") };
    let r_name: GetOperationName = unsafe { pair.rust.function(b"get_operation_name\0") };
    let c_perform: PerformOperation = unsafe { pair.c.function(b"perform_operation\0") };
    let r_perform: PerformOperation = unsafe { pair.rust.function(b"perform_operation\0") };
    let c_buffapp: Buffapp = unsafe { pair.c.function(b"buffapp\0") };
    let r_buffapp: Buffapp = unsafe { pair.rust.function(b"buffapp\0") };
    let mut rng = Rng::new(0x5eed_d1ff_2026_0827);

    for capacity in [0, 1, 2, 7, 32, 257] {
        let c_buffer = unsafe { c_create(capacity) };
        let r_buffer = unsafe { r_create(capacity) };
        assert_eq!(c_buffer.is_null(), r_buffer.is_null());
        assert!(!c_buffer.is_null());
        assert_eq!(unsafe { snapshot(c_buffer) }, unsafe { snapshot(r_buffer) });
        unsafe {
            c_destroy(c_buffer);
            r_destroy(r_buffer);
        }
    }

    unsafe {
        c_destroy(ptr::null_mut());
        r_destroy(ptr::null_mut());
    }
    for destroy in [c_destroy, r_destroy] {
        let raw = unsafe { malloc(std::mem::size_of::<StringBuffer>()) }.cast::<StringBuffer>();
        assert!(!raw.is_null());
        unsafe {
            raw.write(StringBuffer {
                data: ptr::null_mut(),
                capacity: 0,
                length: 0,
            });
            destroy(raw);
        }
    }

    for _ in 0..64 {
        let exact_length = rng.range(1, 24) as usize;
        unsafe {
            compare_append_sequence(pair, 8, &[CString::new("").unwrap()]);
            compare_append_sequence(
                pair,
                exact_length as i32 + 1,
                &[random_ascii(&mut rng, exact_length)],
            );
            compare_append_sequence(
                pair,
                exact_length as i32 + 8,
                &[random_ascii(&mut rng, exact_length)],
            );
            compare_append_sequence(pair, 1, &[random_ascii(&mut rng, exact_length)]);
            compare_append_sequence(
                pair,
                4,
                &[
                    random_ascii(&mut rng, 2),
                    random_ascii(&mut rng, exact_length),
                ],
            );
            compare_append_sequence(
                pair,
                1,
                &(0..12)
                    .map(|_| {
                        let length = rng.range(0, 12) as usize;
                        random_ascii(&mut rng, length)
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }

    let expected_names = ["add", "subtract", "multiply", "divide", "unknown"];
    for (code, expected) in [0, 1, 2, 3, -1].into_iter().zip(expected_names).chain(
        [4, i32::MIN, i32::MAX]
            .into_iter()
            .map(|code| (code, "unknown")),
    ) {
        let c_result = unsafe { CStr::from_ptr(c_name(code)) };
        let r_result = unsafe { CStr::from_ptr(r_name(code)) };
        assert_eq!(
            c_result.to_bytes_with_nul(),
            expected
                .as_bytes()
                .iter()
                .copied()
                .chain([0])
                .collect::<Vec<_>>()
        );
        assert_eq!(c_result, r_result);
    }

    let operations = [
        CString::new("add").unwrap(),
        CString::new("subtract").unwrap(),
        CString::new("multiply").unwrap(),
        CString::new("divide").unwrap(),
        CString::new("unknown").unwrap(),
        CString::new("").unwrap(),
    ];
    for operation in &operations {
        for _ in 0..256 {
            let first = rng.range(-1_000, 1_000);
            let mut second = rng.range(-1_000, 1_000);
            if operation.as_bytes() == b"divide" && second == 0 {
                second = 1;
            }
            let c_result = unsafe { c_perform(first, second, operation.as_ptr()) };
            let r_result = unsafe { r_perform(first, second, operation.as_ptr()) };
            assert_eq!(c_result, r_result, "operation={operation:?}");
        }
    }
    let divide = CString::new("divide").unwrap();
    for first in [-1_000, -1, 0, 1, 1_000] {
        assert_eq!(unsafe { c_perform(first, 0, divide.as_ptr()) }, unsafe {
            r_perform(first, 0, divide.as_ptr())
        });
    }

    const ALL_CLASSES: [OperationClass; 6] = [
        OperationClass::Add,
        OperationClass::Subtract,
        OperationClass::Multiply,
        OperationClass::Divide,
        OperationClass::DivideByZero,
        OperationClass::Unknown,
    ];
    const NONZERO_CLASSES: [OperationClass; 4] = [
        OperationClass::Add,
        OperationClass::Subtract,
        OperationClass::Multiply,
        OperationClass::Divide,
    ];

    for &left in &NONZERO_CLASSES {
        for &right in &NONZERO_CLASSES {
            for _ in 0..24 {
                let (a, b) = pair_for_class(left, false, &mut rng);
                let (c, d) = pair_for_class(right, false, &mut rng);
                assert_ne!(intermediate(left, a, b), 0);
                assert_ne!(intermediate(right, c, d), 0);
                let c_call = unsafe { capture_stdout(|| c_buffapp(a, b, c, d)) };
                let r_call = unsafe { capture_stdout(|| r_buffapp(a, b, c, d)) };
                assert_eq!(c_call, r_call, "{left:?} x {right:?}, NZ");
            }
        }
    }

    for &left in &ALL_CLASSES {
        for &right in &ALL_CLASSES {
            for iteration in 0..24 {
                let force_left_zero =
                    left.always_zero() || (!right.always_zero() && iteration % 2 == 0);
                let force_right_zero = right.always_zero() || !force_left_zero;
                let (a, b) = pair_for_class(left, force_left_zero, &mut rng);
                let (c, d) = pair_for_class(right, force_right_zero, &mut rng);
                assert_eq!(intermediate(left, a, b) * intermediate(right, c, d), 0);
                let c_call = unsafe { capture_stdout(|| c_buffapp(a, b, c, d)) };
                let r_call = unsafe { capture_stdout(|| r_buffapp(a, b, c, d)) };
                assert_eq!(c_call, r_call, "{left:?} x {right:?}, Z");
            }
        }
    }
}

fn run_child(
    test_name: &str,
    variables: &[(&str, &str)],
    preload: bool,
) -> std::process::ExitStatus {
    let mut command = Command::new(env::current_exe().unwrap());
    command
        .args(["--exact", test_name, "--nocapture"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (key, value) in variables {
        command.env(key, value);
    }
    if preload {
        command.env("LD_PRELOAD", env!("ALLOC_FAIL_SHIM"));
    }
    command.status().expect("run isolated test child")
}

unsafe fn error_surface(pair: &Pair) {
    let c_create: CreateBuffer = unsafe { pair.c.function(b"create_buffer\0") };
    let r_create: CreateBuffer = unsafe { pair.rust.function(b"create_buffer\0") };
    let c_perform: PerformOperation = unsafe { pair.c.function(b"perform_operation\0") };
    let r_perform: PerformOperation = unsafe { pair.rust.function(b"perform_operation\0") };
    let c_name: GetOperationName = unsafe { pair.c.function(b"get_operation_name\0") };
    let r_name: GetOperationName = unsafe { pair.rust.function(b"get_operation_name\0") };
    let divide = CString::new("divide").unwrap();
    let invalid = CString::new("not-an-operation").unwrap();

    for oversized_capacity in [-1, i32::MIN] {
        assert!(unsafe { c_create(oversized_capacity) }.is_null());
        assert!(unsafe { r_create(oversized_capacity) }.is_null());
    }
    for value in [-32, -1, 0, 1, 32] {
        assert_eq!(unsafe { c_perform(value, 0, divide.as_ptr()) }, 0);
        assert_eq!(unsafe { r_perform(value, 0, divide.as_ptr()) }, 0);
        assert_eq!(unsafe { c_perform(value, 7, invalid.as_ptr()) }, 0);
        assert_eq!(unsafe { r_perform(value, 7, invalid.as_ptr()) }, 0);
    }
    for code in [-1, 4, i32::MIN, i32::MAX] {
        let c_result = unsafe { CStr::from_ptr(c_name(code)) };
        let r_result = unsafe { CStr::from_ptr(r_name(code)) };
        assert_eq!(c_result.to_bytes_with_nul(), b"unknown\0");
        assert_eq!(c_result, r_result);
    }

    for case in ["first_malloc", "second_malloc", "realloc"] {
        for kind in ["c", "rust"] {
            let status = run_child(
                "allocation_failure_child",
                &[("DIFF_ALLOC_CASE", case), ("DIFF_LIBRARY", kind)],
                true,
            );
            assert!(status.success(), "{kind} allocation case {case}: {status}");
        }
    }

    for case in ["null_buffer", "null_string", "null_operation"] {
        let c_status = run_child(
            "boundary_crash_child",
            &[("DIFF_CRASH_CASE", case), ("DIFF_LIBRARY", "c")],
            false,
        );
        let rust_status = run_child(
            "boundary_crash_child",
            &[("DIFF_CRASH_CASE", case), ("DIFF_LIBRARY", "rust")],
            false,
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "crash signal mismatch for {case}: C={c_status}, Rust={rust_status}"
        );
        assert!(c_status.signal().is_some(), "{case} unexpectedly survived");
    }
}

#[test]
fn differential_surface() {
    assert!(c_library_path().is_file(), "C library was not built");
    assert!(rust_library_path().is_file(), "Rust cdylib was not built");
    let pair = unsafe { Pair::load() };
    unsafe {
        valid_surface(&pair);
        error_surface(&pair);
    }
}

#[test]
fn allocation_failure_child() {
    let Ok(case) = env::var("DIFF_ALLOC_CASE") else {
        return;
    };
    let kind = env::var("DIFF_LIBRARY").unwrap();
    let path = if kind == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&path) };
    let create: CreateBuffer = unsafe { api.function(b"create_buffer\0") };
    let append: AppendToBuffer = unsafe { api.function(b"append_to_buffer\0") };
    let destroy: DestroyBuffer = unsafe { api.function(b"destroy_buffer\0") };
    let shim = unsafe { Library::new(env!("ALLOC_FAIL_SHIM")) }.unwrap();
    let fail_malloc: unsafe extern "C" fn(c_int) =
        unsafe { *shim.get(b"fail_malloc_after\0").unwrap() };
    let fail_realloc: unsafe extern "C" fn() =
        unsafe { *shim.get(b"fail_next_realloc\0").unwrap() };

    match case.as_str() {
        "first_malloc" => {
            unsafe { fail_malloc(0) };
            assert!(unsafe { create(32) }.is_null());
        }
        "second_malloc" => {
            unsafe { fail_malloc(1) };
            assert!(unsafe { create(32) }.is_null());
        }
        "realloc" => {
            let buffer = unsafe { create(1) };
            assert!(!buffer.is_null());
            let before = unsafe { snapshot(buffer) };
            unsafe { fail_realloc() };
            assert_eq!(
                unsafe { append(buffer, CString::new("x").unwrap().as_ptr()) },
                -1
            );
            assert_eq!(unsafe { snapshot(buffer) }, before);
            unsafe { destroy(buffer) };
        }
        _ => panic!("unknown allocation case {case}"),
    }
}

#[test]
fn boundary_crash_child() {
    let Ok(case) = env::var("DIFF_CRASH_CASE") else {
        return;
    };
    let kind = env::var("DIFF_LIBRARY").unwrap();
    let path = if kind == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&path) };

    match case.as_str() {
        "null_buffer" => {
            let append: AppendToBuffer = unsafe { api.function(b"append_to_buffer\0") };
            let value = CString::new("x").unwrap();
            unsafe { append(ptr::null_mut(), value.as_ptr()) };
        }
        "null_string" => {
            let create: CreateBuffer = unsafe { api.function(b"create_buffer\0") };
            let append: AppendToBuffer = unsafe { api.function(b"append_to_buffer\0") };
            let buffer = unsafe { create(8) };
            unsafe { append(buffer, ptr::null()) };
        }
        "null_operation" => {
            let perform: PerformOperation = unsafe { api.function(b"perform_operation\0") };
            unsafe { perform(1, 2, ptr::null()) };
        }
        _ => panic!("unknown crash case {case}"),
    }
}
