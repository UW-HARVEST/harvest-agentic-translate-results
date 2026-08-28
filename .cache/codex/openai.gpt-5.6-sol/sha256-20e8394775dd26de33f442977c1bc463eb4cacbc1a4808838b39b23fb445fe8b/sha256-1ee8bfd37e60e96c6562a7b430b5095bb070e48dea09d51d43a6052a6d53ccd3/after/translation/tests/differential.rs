use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

const C_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../c_src/build/libharvest-work-l4N9HM.so"
);
const RUST_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/target/release/libcomplexmode_lib.so"
);

static PROCESS_STATE: Mutex<()> = Mutex::new(());

type CreateResultString = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
type CheckPermissions = unsafe extern "C" fn(c_int, c_int) -> c_int;
type SafeAdd = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type MultiplyWithLog = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
type CopyAndSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type CompareOperations = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type ComplexMode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(ptr: *mut c_void);
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct Api {
    library: Library,
}

impl Api {
    unsafe fn load(path: impl AsRef<Path>) -> Self {
        Self {
            library: unsafe { Library::new(path.as_ref()) }.unwrap(),
        }
    }

    unsafe fn symbol<T>(&self, name: &[u8]) -> T
    where
        T: Copy,
    {
        let symbol: Symbol<'_, T> = unsafe { self.library.get(name) }.unwrap();
        *symbol
    }

    unsafe fn create_result_string(&self, op: *const c_char, value: c_int) -> *mut c_char {
        let function: CreateResultString = unsafe { self.symbol(b"create_result_string\0") };
        unsafe { function(op, value) }
    }

    unsafe fn check_permissions(&self, perms: c_int, required: c_int) -> c_int {
        let function: CheckPermissions = unsafe { self.symbol(b"check_permissions\0") };
        unsafe { function(perms, required) }
    }

    unsafe fn safe_add(&self, a: c_int, b: c_int, perms: c_int) -> c_int {
        let function: SafeAdd = unsafe { self.symbol(b"safe_add\0") };
        unsafe { function(a, b, perms) }
    }

    unsafe fn multiply_with_log(&self, a: c_int, b: c_int, log: *mut *mut c_char) -> c_int {
        let function: MultiplyWithLog = unsafe { self.symbol(b"multiply_with_log\0") };
        unsafe { function(a, b, log) }
    }

    unsafe fn copy_and_sum(&self, src: *mut c_int, count: c_int) -> c_int {
        let function: CopyAndSum = unsafe { self.symbol(b"copy_and_sum\0") };
        unsafe { function(src, count) }
    }

    unsafe fn compare_operations(&self, op1: *const c_char, op2: *const c_char) -> c_int {
        let function: CompareOperations = unsafe { self.symbol(b"compare_operations\0") };
        unsafe { function(op1, op2) }
    }

    unsafe fn complexmode(&self, mode: c_int, a: c_int, b: c_int, c: c_int) -> c_int {
        let function: ComplexMode = unsafe { self.symbol(b"complexmode\0") };
        unsafe { function(mode, a, b, c) }
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

    fn string(&mut self, length: usize) -> CString {
        let bytes = (0..length)
            .map(|_| b'a' + (self.next_u32() % 26) as u8)
            .collect::<Vec<_>>();
        CString::new(bytes).unwrap()
    }
}

unsafe fn capture_stdout<T>(function: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);

        let result = function();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let count = read(fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(fds[0]), 0);
        (result, output)
    }
}

unsafe fn returned_string(api: &Api, op: *const c_char, value: c_int) -> Option<Vec<u8>> {
    let pointer = unsafe { api.create_result_string(op, value) };
    if pointer.is_null() {
        return None;
    }
    let value = unsafe { CStr::from_ptr(pointer) }
        .to_bytes_with_nul()
        .to_vec();
    unsafe { free(pointer.cast()) };
    Some(value)
}

unsafe fn multiplied(api: &Api, a: c_int, b: c_int) -> (c_int, Option<Vec<u8>>) {
    let mut pointer = ptr::null_mut();
    let result = unsafe { api.multiply_with_log(a, b, &mut pointer) };
    let log = if pointer.is_null() {
        None
    } else {
        let bytes = unsafe { CStr::from_ptr(pointer) }
            .to_bytes_with_nul()
            .to_vec();
        unsafe { free(pointer.cast()) };
        Some(bytes)
    };
    (result, log)
}

fn load_pair() -> (Api, Api) {
    assert!(Path::new(C_SO).is_file(), "missing C shared object: {C_SO}");
    assert!(
        Path::new(RUST_SO).is_file(),
        "missing release Rust shared object: {RUST_SO}"
    );
    unsafe { (Api::load(C_SO), Api::load(RUST_SO)) }
}

#[test]
fn valid_configuration_surface_matches() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);

    unsafe {
        // CONFIGS rows 1-3: empty, fitting, and truncating strings.
        let empty = CString::new("").unwrap();
        for _ in 0..128 {
            let value = rng.next_i32();
            assert_eq!(
                returned_string(&c, empty.as_ptr(), value),
                returned_string(&rust, empty.as_ptr(), value)
            );
        }
        for _ in 0..256 {
            let length = 1 + (rng.next_u32() % 30) as usize;
            let op = rng.string(length);
            let value = rng.next_i32();
            assert_eq!(
                returned_string(&c, op.as_ptr(), value),
                returned_string(&rust, op.as_ptr(), value)
            );
        }
        for _ in 0..256 {
            let length = 40 + (rng.next_u32() % 80) as usize;
            let op = rng.string(length);
            let value = rng.next_i32();
            assert_eq!(
                returned_string(&c, op.as_ptr(), value),
                returned_string(&rust, op.as_ptr(), value)
            );
        }

        // CONFIGS rows 4-7: each result of the permission subset comparison.
        for _ in 0..256 {
            let perms = rng.next_i32();
            assert_eq!(
                c.check_permissions(perms, 0),
                rust.check_permissions(perms, 0)
            );

            let required = rng.next_i32() | 1;
            assert_eq!(
                c.check_permissions(required, required),
                rust.check_permissions(required, required)
            );

            let extra = required | rng.next_i32();
            assert_eq!(
                c.check_permissions(extra, required),
                rust.check_permissions(extra, required)
            );

            let missing = required & !1;
            assert_eq!(
                c.check_permissions(missing, required),
                rust.check_permissions(missing, required)
            );
        }

        // CONFIGS row 8: authorized addition.
        for _ in 0..512 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let perms = 0o600 | (rng.next_i32() & !0o600);
            let c_result = capture_stdout(|| c.safe_add(a, b, perms));
            let rust_result = capture_stdout(|| rust.safe_add(a, b, perms));
            assert_eq!(c_result, rust_result);
        }

        // CONFIGS row 9: multiplication and its caller-owned log allocation.
        for _ in 0..512 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            assert_eq!(multiplied(&c, a, b), multiplied(&rust, a, b));
        }

        // CONFIGS rows 10-12: zero, one, and many elements.
        let mut empty_values: [c_int; 0] = [];
        assert_eq!(
            c.copy_and_sum(empty_values.as_mut_ptr(), 0),
            rust.copy_and_sum(empty_values.as_mut_ptr(), 0)
        );
        for _ in 0..256 {
            let mut one = [rng.next_i32()];
            assert_eq!(
                c.copy_and_sum(one.as_mut_ptr(), 1),
                rust.copy_and_sum(one.as_mut_ptr(), 1)
            );
        }
        for _ in 0..256 {
            let count = 2 + (rng.next_u32() % 127) as usize;
            let mut values = (0..count).map(|_| rng.next_i32()).collect::<Vec<_>>();
            assert_eq!(
                c.copy_and_sum(values.as_mut_ptr(), count as c_int),
                rust.copy_and_sum(values.as_mut_ptr(), count as c_int)
            );
        }

        // CONFIGS rows 13-15: equal, less-than, and greater-than strings.
        for _ in 0..256 {
            let value_length = (rng.next_u32() % 64) as usize;
            let value = rng.string(value_length);
            assert_eq!(
                c.compare_operations(value.as_ptr(), value.as_ptr()),
                rust.compare_operations(value.as_ptr(), value.as_ptr())
            );

            let prefix_length = (rng.next_u32() % 32) as usize;
            let prefix = rng.string(prefix_length);
            let mut low = prefix.as_bytes().to_vec();
            let mut high = prefix.as_bytes().to_vec();
            low.push(b'a');
            high.push(b'z');
            let low = CString::new(low).unwrap();
            let high = CString::new(high).unwrap();
            assert_eq!(
                c.compare_operations(low.as_ptr(), high.as_ptr()),
                rust.compare_operations(low.as_ptr(), high.as_ptr())
            );
            assert_eq!(
                c.compare_operations(high.as_ptr(), low.as_ptr()),
                rust.compare_operations(high.as_ptr(), low.as_ptr())
            );
        }

        // CONFIGS rows 16-19: every valid complexmode switch arm.
        for mode in 1..=4 {
            for _ in 0..256 {
                let values = [rng.next_i32(), rng.next_i32(), rng.next_i32()];
                let c_result =
                    capture_stdout(|| c.complexmode(mode, values[0], values[1], values[2]));
                let rust_result =
                    capture_stdout(|| rust.complexmode(mode, values[0], values[1], values[2]));
                assert_eq!(c_result, rust_result, "complexmode mode {mode}");
            }
        }
    }
}

#[test]
fn explicit_error_surface_and_boundaries_match() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x94d0_49bb_1331_11eb);

    unsafe {
        // ERRORS row 2: every mask lacking read, write, or both is rejected.
        let mut denied_masks = vec![0, 0o400, 0o200, 0o100, i32::MIN, i32::MAX & !0o200];
        denied_masks.extend((0..256).map(|_| rng.next_i32() & !0o400));
        for perms in denied_masks {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let c_result = capture_stdout(|| c.safe_add(a, b, perms));
            let rust_result = capture_stdout(|| rust.safe_add(a, b, perms));
            assert_eq!(c_result, rust_result);
            assert_eq!(c_result.0, 0);
            assert_eq!(c_result.1, b"Insufficient permissions for addition\n");
        }

        // ERRORS row 4: null source pointer.
        let c_result = capture_stdout(|| c.copy_and_sum(ptr::null_mut(), 3));
        let rust_result = capture_stdout(|| rust.copy_and_sum(ptr::null_mut(), 3));
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, (-1, b"Source pointer is NULL\n".to_vec()));

        // ERRORS row 6: either or both operation pointers are null.
        let valid = CString::new("addition").unwrap();
        for (op1, op2) in [
            (ptr::null(), valid.as_ptr()),
            (valid.as_ptr(), ptr::null()),
            (ptr::null(), ptr::null()),
        ] {
            let c_result = capture_stdout(|| c.compare_operations(op1, op2));
            let rust_result = capture_stdout(|| rust.compare_operations(op1, op2));
            assert_eq!(c_result, rust_result);
            assert_eq!(
                c_result,
                (-1, b"One or both operation strings are NULL\n".to_vec())
            );
        }

        // ERRORS row 8 and enum-like FFI boundaries around the switch range.
        let mut invalid_modes = vec![0, 5, -1, i32::MIN, i32::MAX, 1000];
        invalid_modes.extend((0..256).map(|_| {
            let value = rng.next_i32();
            if (1..=4).contains(&value) { 0 } else { value }
        }));
        for mode in invalid_modes {
            let values = [rng.next_i32(), rng.next_i32(), rng.next_i32()];
            let c_result = capture_stdout(|| c.complexmode(mode, values[0], values[1], values[2]));
            let rust_result =
                capture_stdout(|| rust.complexmode(mode, values[0], values[1], values[2]));
            assert_eq!(c_result, rust_result);
            assert_eq!(c_result, (-1, b"Invalid mode\n".to_vec()));
        }

        // Generic null boundary: glibc formats a null %s pointer as "(null)".
        for value in [i32::MIN, -1, 0, 1, i32::MAX] {
            assert_eq!(
                returned_string(&c, ptr::null(), value),
                returned_string(&rust, ptr::null(), value)
            );
        }

        // Generic oversized boundary: negative count wraps to a huge allocation.
        let mut value = [7];
        for count in [-1, i32::MIN] {
            let c_result = capture_stdout(|| c.copy_and_sum(value.as_mut_ptr(), count));
            let rust_result = capture_stdout(|| rust.copy_and_sum(value.as_mut_ptr(), count));
            assert_eq!(c_result, rust_result);
            assert_eq!(c_result, (-1, b"Memory allocation failed\n".to_vec()));
        }
    }
}

fn failmalloc_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-support/libfailmalloc.so")
}

fn build_failmalloc() -> PathBuf {
    let output = failmalloc_path();
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/failmalloc.c");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(source)
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success(), "failed to build malloc fault injector");
    output
}

fn run_ignored_child(test_name: &str, environment: &[(&str, &str)], preload: Option<&Path>) {
    let mut command = Command::new(env::current_exe().unwrap());
    command.args(["--ignored", "--exact", test_name]);
    for (name, value) in environment {
        command.env(name, value);
    }
    if let Some(path) = preload {
        command.env("LD_PRELOAD", path);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "child {test_name} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn allocation_error_surface_matches() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let preload = build_failmalloc();
    for case in ["create", "multiply", "copy", "tracker", "mode2_log"] {
        run_ignored_child(
            "allocator_fault_child",
            &[("FAULT_CASE", case)],
            Some(&preload),
        );
    }
}

#[test]
#[ignore = "subprocess helper"]
fn allocator_fault_child() {
    let (c, rust) = load_pair();
    let shim = unsafe { Library::new(failmalloc_path()) }.unwrap();
    let arm: Symbol<'_, unsafe extern "C" fn(usize)> =
        unsafe { shim.get(b"fail_malloc_once\0") }.unwrap();

    unsafe {
        match env::var("FAULT_CASE").unwrap().as_str() {
            // ERRORS row 1.
            "create" => {
                let op = CString::new("addition").unwrap();
                arm(64);
                let c_pointer = c.create_result_string(op.as_ptr(), 7);
                arm(64);
                let rust_pointer = rust.create_result_string(op.as_ptr(), 7);
                assert!(c_pointer.is_null());
                assert_eq!(c_pointer.is_null(), rust_pointer.is_null());
            }
            // ERRORS row 3.
            "multiply" => {
                let mut c_log = 1_usize as *mut c_char;
                arm(64);
                let c_result = c.multiply_with_log(11, 13, &mut c_log);
                let mut rust_log = 1_usize as *mut c_char;
                arm(64);
                let rust_result = rust.multiply_with_log(11, 13, &mut rust_log);
                assert_eq!(c_result, 0);
                assert_eq!(c_result, rust_result);
                assert!(c_log.is_null());
                assert_eq!(c_log.is_null(), rust_log.is_null());
            }
            // ERRORS row 5.
            "copy" => {
                let mut values = [3, 5, 7];
                let c_result = capture_stdout(|| {
                    arm(12);
                    c.copy_and_sum(values.as_mut_ptr(), 3)
                });
                let rust_result = capture_stdout(|| {
                    arm(12);
                    rust.copy_and_sum(values.as_mut_ptr(), 3)
                });
                assert_eq!(c_result, rust_result);
                assert_eq!(c_result, (-1, b"Memory allocation failed\n".to_vec()));
            }
            // ERRORS row 7.
            "tracker" => {
                let c_result = capture_stdout(|| {
                    arm(40);
                    c.complexmode(1, 2, 3, 4)
                });
                let rust_result = capture_stdout(|| {
                    arm(40);
                    rust.complexmode(1, 2, 3, 4)
                });
                assert_eq!(c_result, rust_result);
                assert_eq!(
                    c_result,
                    (-1, b"Failed to allocate result tracker\n".to_vec())
                );
            }
            // ERRORS row 9.
            "mode2_log" => {
                let c_result = capture_stdout(|| {
                    arm(64);
                    c.complexmode(2, 2, 3, 4)
                });
                let rust_result = capture_stdout(|| {
                    arm(64);
                    rust.complexmode(2, 2, 3, 4)
                });
                assert_eq!(c_result, rust_result);
                assert_eq!(
                    c_result,
                    (
                        0,
                        concat!(
                            "Log message creation failed\n",
                            "Operation performed: multiplication\n"
                        )
                        .as_bytes()
                        .to_vec()
                    )
                );
            }
            case => panic!("unknown fault case {case}"),
        }
    }
}

#[test]
fn unchecked_null_output_pointer_behavior_matches() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let mut signals = Vec::new();
    for library in ["c", "rust"] {
        let output = Command::new(env::current_exe().unwrap())
            .args(["--ignored", "--exact", "null_log_output_pointer_child"])
            .env("CRASH_LIBRARY", library)
            .output()
            .unwrap();
        signals.push(output.status.signal());
    }
    assert_eq!(signals[0], signals[1]);
    assert_eq!(signals[0], Some(11));
}

#[test]
#[ignore = "subprocess helper expected to receive SIGSEGV"]
fn null_log_output_pointer_child() {
    let library = env::var("CRASH_LIBRARY").unwrap();
    let api = unsafe {
        if library == "c" {
            Api::load(C_SO)
        } else {
            Api::load(RUST_SO)
        }
    };
    unsafe {
        api.multiply_with_log(2, 3, ptr::null_mut());
    }
}
