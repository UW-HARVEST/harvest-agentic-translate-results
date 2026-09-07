use libloading::Library;
use std::collections::BTreeSet;
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;
use std::sync::Mutex;

type CreateResultString = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
type CheckPermissions = unsafe extern "C" fn(c_int, c_int) -> c_int;
type SafeAdd = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type MultiplyWithLog = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
type CopyAndSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type CompareOperations = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type ComplexMode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
const EXEC_PERM: c_int = 0o100;
const ITERATIONS: usize = 128;

static TEST_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(ptr: *mut c_void);
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

struct Api {
    _library: Library,
    create_result_string: CreateResultString,
    check_permissions: CheckPermissions,
    safe_add: SafeAdd,
    multiply_with_log: MultiplyWithLog,
    copy_and_sum: CopyAndSum,
    compare_operations: CompareOperations,
    complexmode: ComplexMode,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let create_result_string = unsafe {
            *library
                .get::<CreateResultString>(b"create_result_string\0")
                .unwrap()
        };
        let check_permissions = unsafe {
            *library
                .get::<CheckPermissions>(b"check_permissions\0")
                .unwrap()
        };
        let safe_add = unsafe { *library.get::<SafeAdd>(b"safe_add\0").unwrap() };
        let multiply_with_log = unsafe {
            *library
                .get::<MultiplyWithLog>(b"multiply_with_log\0")
                .unwrap()
        };
        let copy_and_sum = unsafe { *library.get::<CopyAndSum>(b"copy_and_sum\0").unwrap() };
        let compare_operations = unsafe {
            *library
                .get::<CompareOperations>(b"compare_operations\0")
                .unwrap()
        };
        let complexmode = unsafe { *library.get::<ComplexMode>(b"complexmode\0").unwrap() };
        Self {
            _library: library,
            create_result_string,
            check_permissions,
            safe_add,
            multiply_with_log,
            copy_and_sum,
            compare_operations,
            complexmode,
        }
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

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn small_i32(&mut self) -> i32 {
        (self.next_u32() % 2_000_001) as i32 - 1_000_000
    }

    fn ascii_string(&mut self, min_len: usize, max_len: usize) -> CString {
        let len = min_len + self.next_u32() as usize % (max_len - min_len + 1);
        let bytes = (0..len)
            .map(|_| b'a' + (self.next_u32() % 26) as u8)
            .collect::<Vec<_>>();
        CString::new(bytes).unwrap()
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let directory = manifest_dir().parent().unwrap().join("c_src/build");
    let libraries = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect::<Vec<_>>();
    assert_eq!(libraries.len(), 1, "expected one C shared library");
    libraries[0].clone()
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libcomplexmode_lib.so")
}

unsafe fn load_apis() -> (Api, Api) {
    (unsafe { Api::load(&c_library_path()) }, unsafe {
        Api::load(&rust_library_path())
    })
}

fn capture_stdout<R>(call: impl FnOnce() -> R) -> (R, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut descriptors = [0; 2];
        assert_eq!(pipe(descriptors.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(descriptors[1], 1), 1);
        assert_eq!(close(descriptors[1]), 0);

        let result = call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = read(descriptors[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(descriptors[0]), 0);
        (result, output)
    }
}

unsafe fn owned_c_string(pointer: *mut c_char) -> Option<Vec<u8>> {
    if pointer.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(pointer) }
        .to_bytes_with_nul()
        .to_vec();
    unsafe { free(pointer.cast()) };
    Some(bytes)
}

fn assert_simple_call_matches(c_call: impl FnOnce() -> c_int, rust_call: impl FnOnce() -> c_int) {
    let (c_result, c_output) = capture_stdout(c_call);
    let (rust_result, rust_output) = capture_stdout(rust_call);
    assert_eq!(rust_result, c_result);
    assert_eq!(rust_output, c_output);
}

unsafe fn assert_create_matches(c: &Api, rust: &Api, operation: *const c_char, value: c_int) {
    let c_result = unsafe { owned_c_string((c.create_result_string)(operation, value)) };
    let rust_result = unsafe { owned_c_string((rust.create_result_string)(operation, value)) };
    assert_eq!(rust_result, c_result);
}

unsafe fn assert_multiply_matches(c: &Api, rust: &Api, left: c_int, right: c_int) {
    let mut c_log = ptr::null_mut();
    let mut rust_log = ptr::null_mut();
    let (c_result, c_output) =
        capture_stdout(|| unsafe { (c.multiply_with_log)(left, right, &mut c_log) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.multiply_with_log)(left, right, &mut rust_log) });
    assert_eq!(rust_result, c_result);
    assert_eq!(rust_output, c_output);
    assert_eq!(unsafe { owned_c_string(rust_log) }, unsafe {
        owned_c_string(c_log)
    });
}

unsafe fn assert_copy_matches(c: &Api, rust: &Api, values: &[c_int], count: c_int) {
    let c_pointer = values.as_ptr().cast_mut();
    let rust_pointer = values.as_ptr().cast_mut();
    assert_simple_call_matches(
        || unsafe { (c.copy_and_sum)(c_pointer, count) },
        || unsafe { (rust.copy_and_sum)(rust_pointer, count) },
    );
}

unsafe fn assert_compare_matches(c: &Api, rust: &Api, left: &CStr, right: &CStr) {
    assert_simple_call_matches(
        || unsafe { (c.compare_operations)(left.as_ptr(), right.as_ptr()) },
        || unsafe { (rust.compare_operations)(left.as_ptr(), right.as_ptr()) },
    );
}

unsafe fn assert_complex_matches(
    c: &Api,
    rust: &Api,
    mode: c_int,
    first: c_int,
    second: c_int,
    third: c_int,
) {
    assert_simple_call_matches(
        || unsafe { (c.complexmode)(mode, first, second, third) },
        || unsafe { (rust.complexmode)(mode, first, second, third) },
    );
}

#[test]
fn valid_configuration_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);

    // CONFIGS.md rows 1-4: formatting shapes and the platform NULL-%s case.
    for _ in 0..ITERATIONS {
        let operation = rng.ascii_string(1, 20);
        unsafe {
            assert_create_matches(&c, &rust, operation.as_ptr(), rng.small_i32().abs());
        }
    }
    let empty = CString::new("").unwrap();
    for _ in 0..ITERATIONS {
        unsafe {
            assert_create_matches(&c, &rust, empty.as_ptr(), -rng.small_i32().abs());
        }
    }
    for _ in 0..ITERATIONS {
        let operation = rng.ascii_string(44, 160);
        unsafe {
            assert_create_matches(&c, &rust, operation.as_ptr(), rng.next_i32());
        }
    }
    for _ in 0..ITERATIONS {
        unsafe { assert_create_matches(&c, &rust, ptr::null(), rng.next_i32()) };
    }

    // Rows 5-10: every permission-mask branch and mask shape.
    for _ in 0..ITERATIONS {
        let permissions = rng.next_i32();
        unsafe {
            assert_eq!(
                (rust.check_permissions)(permissions, 0),
                (c.check_permissions)(permissions, 0)
            );
        }
    }
    for _ in 0..ITERATIONS {
        let bit = 1_i32 << (rng.next_u32() % 31);
        let extras = rng.next_i32() & !bit;
        let partial = if rng.next_u32() & 1 == 0 {
            READ_PERM
        } else {
            WRITE_PERM
        };
        unsafe {
            assert_eq!(
                (rust.check_permissions)(bit | extras, bit),
                (c.check_permissions)(bit | extras, bit)
            );
            assert_eq!(
                (rust.check_permissions)(extras, bit),
                (c.check_permissions)(extras, bit)
            );
            assert_eq!(
                (rust.check_permissions)(READ_PERM | WRITE_PERM | extras, READ_PERM | WRITE_PERM,),
                (c.check_permissions)(READ_PERM | WRITE_PERM | extras, READ_PERM | WRITE_PERM,)
            );
            assert_eq!(
                (rust.check_permissions)(
                    partial | (extras & !(READ_PERM | WRITE_PERM)),
                    READ_PERM | WRITE_PERM,
                ),
                (c.check_permissions)(
                    partial | (extras & !(READ_PERM | WRITE_PERM)),
                    READ_PERM | WRITE_PERM,
                )
            );
            assert_eq!(
                (rust.check_permissions)(extras | i32::MIN, i32::MIN),
                (c.check_permissions)(extras | i32::MIN, i32::MIN)
            );
            assert_eq!(
                (rust.check_permissions)(extras & !i32::MIN, i32::MIN),
                (c.check_permissions)(extras & !i32::MIN, i32::MIN)
            );
        }
    }

    // Rows 11-15: permission branches and ordinary/overflowing additions.
    for _ in 0..ITERATIONS {
        let left = rng.small_i32();
        let right = rng.small_i32();
        assert_simple_call_matches(
            || unsafe { (c.safe_add)(left, right, READ_PERM | WRITE_PERM) },
            || unsafe { (rust.safe_add)(left, right, READ_PERM | WRITE_PERM) },
        );
        let extra = READ_PERM | WRITE_PERM | (rng.next_i32() & !0o777);
        assert_simple_call_matches(
            || unsafe { (c.safe_add)(left, right, extra) },
            || unsafe { (rust.safe_add)(left, right, extra) },
        );
        for permissions in [0, READ_PERM, WRITE_PERM] {
            assert_simple_call_matches(
                || unsafe { (c.safe_add)(left, right, permissions) },
                || unsafe { (rust.safe_add)(left, right, permissions) },
            );
        }
    }
    for _ in 0..ITERATIONS {
        let offset = (rng.next_u32() % 1_000_000) as i32;
        let positive = (rng.next_u32() % 1_000_000 + 1) as i32;
        let (left, right) = if rng.next_u32() & 1 == 0 {
            (i32::MAX - offset, offset + positive)
        } else {
            (i32::MIN + offset, -(offset + positive))
        };
        assert_simple_call_matches(
            || unsafe { (c.safe_add)(left, right, READ_PERM | WRITE_PERM) },
            || unsafe { (rust.safe_add)(left, right, READ_PERM | WRITE_PERM) },
        );
    }

    // Rows 16-18: multiplication, log out-parameter, and arithmetic boundaries.
    for _ in 0..ITERATIONS {
        unsafe { assert_multiply_matches(&c, &rust, rng.small_i32(), rng.small_i32()) };
        unsafe { assert_multiply_matches(&c, &rust, 0, -rng.small_i32().abs()) };
    }
    for _ in 0..ITERATIONS {
        let left = 50_000 + (rng.next_u32() % 50_000) as i32;
        let mut right = 50_000 + (rng.next_u32() % 50_000) as i32;
        if rng.next_u32() & 1 != 0 {
            right = -right;
        }
        unsafe { assert_multiply_matches(&c, &rust, left, right) };
    }

    // Rows 19-22: zero, one, many, mixed-sign, and wrapping sums.
    for _ in 0..ITERATIONS {
        let dummy = [rng.next_i32()];
        unsafe { assert_copy_matches(&c, &rust, &dummy, 0) };

        let one = [rng.next_i32()];
        unsafe { assert_copy_matches(&c, &rust, &one, 1) };

        let len = 2 + rng.next_u32() as usize % 31;
        let values = (0..len).map(|_| rng.small_i32()).collect::<Vec<_>>();
        unsafe { assert_copy_matches(&c, &rust, &values, len as c_int) };
    }
    for _ in 0..ITERATIONS {
        let len = 2 + rng.next_u32() as usize % 31;
        let mut values = (0..len).map(|_| rng.small_i32()).collect::<Vec<_>>();
        values[0] = if rng.next_u32() & 1 == 0 {
            i32::MAX
        } else {
            i32::MIN
        };
        values[1] = if values[0] == i32::MAX {
            (rng.next_u32() % 1_000_000 + 1) as i32
        } else {
            -((rng.next_u32() % 1_000_000 + 1) as i32)
        };
        unsafe { assert_copy_matches(&c, &rust, &values, values.len() as c_int) };
    }

    // Rows 23-26: equal, empty, ordered, and prefix string comparisons.
    for _ in 0..ITERATIONS {
        let value = rng.ascii_string(1, 40);
        unsafe { assert_compare_matches(&c, &rust, &value, &value) };
    }
    unsafe { assert_compare_matches(&c, &rust, &empty, &empty) };
    for _ in 0..ITERATIONS {
        let suffix = rng.ascii_string(1, 30);
        let mut lower = vec![b'a'];
        lower.extend_from_slice(suffix.to_bytes());
        let mut higher = vec![b'z'];
        higher.extend_from_slice(suffix.to_bytes());
        let lower = CString::new(lower).unwrap();
        let higher = CString::new(higher).unwrap();
        unsafe {
            assert_compare_matches(&c, &rust, &lower, &higher);
            assert_compare_matches(&c, &rust, &higher, &lower);
        }

        let prefix = rng.ascii_string(1, 30);
        let mut longer = prefix.to_bytes().to_vec();
        longer.push(b'-');
        longer.extend_from_slice(rng.ascii_string(1, 20).to_bytes());
        let longer = CString::new(longer).unwrap();
        unsafe {
            assert_compare_matches(&c, &rust, &prefix, &longer);
            assert_compare_matches(&c, &rust, &longer, &prefix);
        }
    }

    // Rows 27-30: every valid complexmode switch arm, including boundaries.
    for mode in 1..=4 {
        for _ in 0..ITERATIONS {
            unsafe {
                assert_complex_matches(
                    &c,
                    &rust,
                    mode,
                    rng.small_i32(),
                    rng.small_i32(),
                    rng.small_i32(),
                );
            }
        }
    }
    for &(mode, first, second, third) in &[
        (1, i32::MAX, 1, 0),
        (1, i32::MIN, -1, 0),
        (2, i32::MAX, 2, 0),
        (2, i32::MIN, -1, 0),
        (3, i32::MAX, 1, 1),
        (3, i32::MIN, -1, -1),
        (4, i32::MAX, 1, 1),
        (4, i32::MIN, -1, -1),
    ] {
        unsafe { assert_complex_matches(&c, &rust, mode, first, second, third) };
    }
}

#[test]
fn explicit_and_generic_error_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { load_apis() };

    // ERRORS.md row 2.
    for permissions in [0, READ_PERM, WRITE_PERM, EXEC_PERM] {
        assert_simple_call_matches(
            || unsafe { (c.safe_add)(7, 11, permissions) },
            || unsafe { (rust.safe_add)(7, 11, permissions) },
        );
    }

    // Rows 4 and 11.
    for count in [0, 1, i32::MAX] {
        assert_simple_call_matches(
            || unsafe { (c.copy_and_sum)(ptr::null_mut(), count) },
            || unsafe { (rust.copy_and_sum)(ptr::null_mut(), count) },
        );
    }

    // Row 6: each side of the OR and both NULL.
    let operation = CString::new("operation").unwrap();
    for (left, right) in [
        (ptr::null(), operation.as_ptr()),
        (operation.as_ptr(), ptr::null()),
        (ptr::null(), ptr::null()),
    ] {
        assert_simple_call_matches(
            || unsafe { (c.compare_operations)(left, right) },
            || unsafe { (rust.compare_operations)(left, right) },
        );
    }

    // Row 9 and out-of-range enum/mode-style integer boundaries.
    for mode in [i32::MIN, -1, 0, 5, 6, i32::MAX] {
        unsafe { assert_complex_matches(&c, &rust, mode, 10, 20, 30) };
    }

    // Row 12: negative count becomes an oversized allocation request.
    let value = [1];
    for count in [-1, i32::MIN] {
        assert_simple_call_matches(
            || unsafe { (c.copy_and_sum)(value.as_ptr().cast_mut(), count) },
            || unsafe { (rust.copy_and_sum)(value.as_ptr().cast_mut(), count) },
        );
    }
}

fn compile_malloc_interposer() -> PathBuf {
    let output_directory = manifest_dir().join("target/differential-support");
    fs::create_dir_all(&output_directory).unwrap();
    let output = output_directory.join("libfail_malloc.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11", "-O2"])
        .arg(manifest_dir().join("tests/fail_malloc.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .expect("failed to execute cc for malloc interposer");
    assert!(status.success());
    assert!(output.is_file());
    output
}

fn run_fault_child(interposer: &Path, side: &str, fault_case: &str) -> Output {
    Command::new(env::current_exe().unwrap())
        .args(["--exact", "fault_child", "--nocapture", "--test-threads=1"])
        .env("LD_PRELOAD", interposer)
        .env("DIFF_SIDE", side)
        .env("DIFF_FAULT_CASE", fault_case)
        .output()
        .unwrap()
}

#[test]
fn allocation_and_crash_error_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let interposer = compile_malloc_interposer();
    for fault_case in [
        "create_alloc",
        "multiply_alloc",
        "copy_alloc",
        "tracker_alloc",
        "mode2_log_alloc",
    ] {
        let c = run_fault_child(&interposer, "c", fault_case);
        let rust = run_fault_child(&interposer, "rust", fault_case);
        assert!(c.status.success(), "C child failed for {fault_case}: {c:?}");
        assert!(
            rust.status.success(),
            "Rust child failed for {fault_case}: {rust:?}"
        );
        assert_eq!(rust.stdout, c.stdout, "stdout mismatch for {fault_case}");
        assert_eq!(rust.stderr, c.stderr, "stderr mismatch for {fault_case}");
    }

    // ERRORS.md row 10: both unchecked NULL out-pointers must fault identically.
    let c = run_fault_child(&interposer, "c", "null_log_slot");
    let rust = run_fault_child(&interposer, "rust", "null_log_slot");
    assert_eq!(c.status.signal(), Some(11), "unexpected C status: {c:?}");
    assert_eq!(
        rust.status.signal(),
        c.status.signal(),
        "Rust crash status differs: {rust:?}"
    );
}

#[test]
fn fault_child() {
    let Ok(fault_case) = env::var("DIFF_FAULT_CASE") else {
        return;
    };
    let side = env::var("DIFF_SIDE").unwrap();
    let path = if side == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&path) };
    let process = libloading::os::unix::Library::this();
    let arm = unsafe {
        *process
            .get::<unsafe extern "C" fn(usize)>(b"fail_malloc_once\0")
            .unwrap()
    };

    match fault_case.as_str() {
        "create_alloc" => unsafe {
            arm(64);
            let pointer = (api.create_result_string)(c"x".as_ptr(), 1);
            println!("pointer_null={}", pointer.is_null());
        },
        "multiply_alloc" => unsafe {
            let mut log = 1_usize as *mut c_char;
            arm(64);
            let result = (api.multiply_with_log)(3, 7, &mut log);
            println!("result={result};pointer_null={}", log.is_null());
        },
        "copy_alloc" => unsafe {
            let mut values = [1, 2, 3, 4];
            arm(16);
            let result = (api.copy_and_sum)(values.as_mut_ptr(), 4);
            println!("result={result}");
        },
        "tracker_alloc" => unsafe {
            arm(40);
            let result = (api.complexmode)(1, 3, 7, 11);
            println!("result={result}");
        },
        "mode2_log_alloc" => unsafe {
            arm(64);
            let result = (api.complexmode)(2, 3, 7, 11);
            println!("result={result}");
        },
        "null_log_slot" => unsafe {
            (api.multiply_with_log)(3, 7, ptr::null_mut());
        },
        _ => panic!("unknown fault case {fault_case}"),
    }
}

fn defined_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2))
        .map(ToOwned::to_owned)
        .collect()
}

#[test]
fn dynamic_symbol_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let c_symbols = defined_symbols(&c_library_path());
    let rust_symbols = defined_symbols(&rust_library_path());
    let missing = c_symbols.difference(&rust_symbols).collect::<Vec<_>>();
    assert!(missing.is_empty(), "Rust is missing C symbols: {missing:?}");
    assert_eq!(
        c_symbols,
        BTreeSet::from([
            "check_permissions".to_owned(),
            "compare_operations".to_owned(),
            "complexmode".to_owned(),
            "copy_and_sum".to_owned(),
            "create_result_string".to_owned(),
            "multiply_with_log".to_owned(),
            "safe_add".to_owned(),
        ])
    );
}
