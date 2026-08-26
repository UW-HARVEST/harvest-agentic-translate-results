use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_long, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;

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

struct Api {
    create_buffer: CreateBuffer,
    append_to_buffer: AppendToBuffer,
    destroy_buffer: DestroyBuffer,
    get_operation_name: GetOperationName,
    perform_operation: PerformOperation,
    buffapp: Buffapp,
    _library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        unsafe {
            let library = Library::new(path)
                .unwrap_or_else(|error| panic!("load {}: {error}", path.display()));
            let create_buffer = *library.get(b"create_buffer\0").unwrap();
            let append_to_buffer = *library.get(b"append_to_buffer\0").unwrap();
            let destroy_buffer = *library.get(b"destroy_buffer\0").unwrap();
            let get_operation_name = *library.get(b"get_operation_name\0").unwrap();
            let perform_operation = *library.get(b"perform_operation\0").unwrap();
            let buffapp = *library.get(b"buffapp\0").unwrap();
            Self {
                create_buffer,
                append_to_buffer,
                destroy_buffer,
                get_operation_name,
                perform_operation,
                buffapp,
                _library: library,
            }
        }
    }
}

type FaultAfter = unsafe extern "C" fn(c_long);
type FaultReset = unsafe extern "C" fn();
type FaultFreeCalls = unsafe extern "C" fn() -> c_long;

struct FaultApi {
    malloc_after: FaultAfter,
    realloc_after: FaultAfter,
    reset: FaultReset,
    free_calls: FaultFreeCalls,
    _library: libloading::os::unix::Library,
}

impl FaultApi {
    unsafe fn load() -> Self {
        unsafe {
            let library = libloading::os::unix::Library::this();
            let malloc_after = *library.get(b"fault_malloc_after\0").unwrap();
            let realloc_after = *library.get(b"fault_realloc_after\0").unwrap();
            let reset = *library.get(b"fault_reset\0").unwrap();
            let free_calls = *library.get(b"fault_free_calls\0").unwrap();
            Self {
                malloc_after,
                realloc_after,
                reset,
                free_calls,
                _library: library,
            }
        }
    }
}

unsafe extern "C" {
    fn _exit(status: c_int) -> !;
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn free(pointer: *mut c_void);
    fn pipe(fds: *mut c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn profile_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_library_path() -> PathBuf {
    let profile = profile_dir();
    let direct = profile.join("libbuffapp_lib.so");
    if direct.exists() {
        direct
    } else {
        profile.join("deps/libbuffapp_lib.so")
    }
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);

        let result = call();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
        let mut output = Vec::new();
        File::from_raw_fd(fds[0]).read_to_end(&mut output).unwrap();
        (result, output)
    }
}

fn child_status(call: impl FnOnce()) -> c_int {
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        let pid = fork();
        assert!(pid >= 0);
        if pid == 0 {
            call();
            _exit(0);
        }
        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid);
        status
    }
}

fn terminating_signal(status: c_int) -> Option<c_int> {
    let signal = status & 0x7f;
    (signal != 0 && signal != 0x7f).then_some(signal)
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        (value.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn inclusive(&mut self, minimum: i32, maximum: i32) -> i32 {
        let width = (maximum as i64 - minimum as i64 + 1) as u64;
        (minimum as i64 + (self.next_u32() as u64 % width) as i64) as i32
    }
}

#[derive(Clone, Copy, Debug)]
enum OperationState {
    AddZero,
    AddNonzero,
    SubtractZero,
    SubtractNonzero,
    MultiplyZero,
    MultiplyNonzero,
    DivideByZero,
    DivideQuotientZero,
    DivideNonzero,
    Unknown,
}

const OPERATION_STATES: [OperationState; 10] = [
    OperationState::AddZero,
    OperationState::AddNonzero,
    OperationState::SubtractZero,
    OperationState::SubtractNonzero,
    OperationState::MultiplyZero,
    OperationState::MultiplyNonzero,
    OperationState::DivideByZero,
    OperationState::DivideQuotientZero,
    OperationState::DivideNonzero,
    OperationState::Unknown,
];

impl OperationState {
    fn code(self) -> &'static str {
        match self {
            Self::AddZero => "AZ",
            Self::AddNonzero => "AN",
            Self::SubtractZero => "SZ",
            Self::SubtractNonzero => "SN",
            Self::MultiplyZero => "MZ",
            Self::MultiplyNonzero => "MN",
            Self::DivideByZero => "DZ",
            Self::DivideQuotientZero => "DQ",
            Self::DivideNonzero => "DN",
            Self::Unknown => "UZ",
        }
    }

    fn arguments(self, rng: &mut Rng) -> (i32, i32) {
        let k = rng.inclusive(1, 12);
        match self {
            Self::AddZero => {
                let parameter = 4 * k;
                (parameter, -parameter)
            }
            Self::AddNonzero => {
                let parameter = 4 * k;
                let mut operand = rng.inclusive(-40, 40);
                if operand == -parameter {
                    operand += 1;
                }
                (parameter, operand)
            }
            Self::SubtractZero => {
                let parameter = 4 * k + 1;
                (parameter, parameter)
            }
            Self::SubtractNonzero => {
                let parameter = 4 * k + 1;
                let mut operand = rng.inclusive(-40, 40);
                if operand == parameter {
                    operand -= 1;
                }
                (parameter, operand)
            }
            Self::MultiplyZero => (4 * k + 2, 0),
            Self::MultiplyNonzero => {
                let parameter = 4 * k + 2;
                let mut operand = rng.inclusive(-12, 12);
                if operand == 0 {
                    operand = 1;
                }
                (parameter, operand)
            }
            Self::DivideByZero => (4 * k + 3, 0),
            Self::DivideQuotientZero => {
                let parameter = 4 * k + 3;
                (parameter, parameter + rng.inclusive(1, 20))
            }
            Self::DivideNonzero => {
                let parameter = 4 * k + 3;
                let magnitude = rng.inclusive(1, parameter);
                let operand = if rng.next_u32() & 1 == 0 {
                    magnitude
                } else {
                    -magnitude
                };
                (parameter, operand)
            }
            Self::Unknown => {
                let remainder = rng.inclusive(1, 3);
                (-(4 * k + remainder), rng.inclusive(-40, 40))
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct BufferSnapshot {
    capacity: i32,
    length: i32,
    bytes: Vec<u8>,
}

unsafe fn snapshot(buffer: *mut StringBuffer) -> BufferSnapshot {
    unsafe {
        let length = (*buffer).length;
        BufferSnapshot {
            capacity: (*buffer).capacity,
            length,
            bytes: std::slice::from_raw_parts((*buffer).data.cast::<u8>(), length as usize + 1)
                .to_vec(),
        }
    }
}

unsafe fn append_sequence(
    api: &Api,
    capacity: i32,
    strings: &[Vec<u8>],
) -> Vec<(i32, BufferSnapshot)> {
    unsafe {
        let buffer = (api.create_buffer)(capacity);
        assert!(!buffer.is_null());
        let mut results = Vec::with_capacity(strings.len());
        for bytes in strings {
            let string = CString::new(bytes.clone()).unwrap();
            let result = (api.append_to_buffer)(buffer, string.as_ptr());
            results.push((result, snapshot(buffer)));
        }
        (api.destroy_buffer)(buffer);
        results
    }
}

fn compare_append_case(
    c_api: &Api,
    rust_api: &Api,
    row: usize,
    capacity: i32,
    strings: &[Vec<u8>],
) {
    unsafe {
        let c_result = append_sequence(c_api, capacity, strings);
        let rust_result = append_sequence(rust_api, capacity, strings);
        assert_eq!(c_result, rust_result, "CONFIGS.md row {row}");
    }
}

fn verify_create_buffer(c_api: &Api, rust_api: &Api) {
    let mut rng = Rng::new(0x9d2c_5680_1d13_59ab);
    for _ in 0..128 {
        let capacity = rng.inclusive(1, 256);
        unsafe {
            let c_buffer = (c_api.create_buffer)(capacity);
            let rust_buffer = (rust_api.create_buffer)(capacity);
            assert!(!c_buffer.is_null());
            assert!(!rust_buffer.is_null());
            assert_eq!(
                snapshot(c_buffer),
                snapshot(rust_buffer),
                "CONFIGS.md row 1"
            );
            (c_api.destroy_buffer)(c_buffer);
            (rust_api.destroy_buffer)(rust_buffer);
        }
    }
}

fn verify_append_paths(c_api: &Api, rust_api: &Api) {
    let mut rng = Rng::new(0x4f1b_bcdd_8142_8841);
    for _ in 0..64 {
        compare_append_case(c_api, rust_api, 2, rng.inclusive(1, 64), &[vec![]]);

        let length = rng.inclusive(1, 40) as usize;
        let ascii = (0..length)
            .map(|_| b'a' + (rng.next_u32() % 26) as u8)
            .collect::<Vec<_>>();
        compare_append_case(
            c_api,
            rust_api,
            3,
            length as i32 + rng.inclusive(2, 20),
            std::slice::from_ref(&ascii),
        );
        compare_append_case(
            c_api,
            rust_api,
            4,
            length as i32 + 1,
            std::slice::from_ref(&ascii),
        );
        compare_append_case(
            c_api,
            rust_api,
            5,
            rng.inclusive(1, length as i32),
            std::slice::from_ref(&ascii),
        );

        let first = vec![b'x'; rng.inclusive(1, 5) as usize];
        let second = vec![b'y'; rng.inclusive(20, 50) as usize];
        compare_append_case(c_api, rust_api, 6, 16, &[first, second]);

        let bytes = (0..length)
            .map(|_| {
                let value = (rng.next_u32() % 255 + 1) as u8;
                if value < 0x80 { value | 0x80 } else { value }
            })
            .collect::<Vec<_>>();
        compare_append_case(c_api, rust_api, 7, rng.inclusive(1, 48), &[bytes]);
    }
}

fn verify_destroy_normal(c_api: &Api, rust_api: &Api, faults: &FaultApi) {
    unsafe {
        let c_buffer = (c_api.create_buffer)(32);
        (faults.reset)();
        (c_api.destroy_buffer)(c_buffer);
        let c_frees = (faults.free_calls)();

        let rust_buffer = (rust_api.create_buffer)(32);
        (faults.reset)();
        (rust_api.destroy_buffer)(rust_buffer);
        let rust_frees = (faults.free_calls)();

        assert_eq!(c_frees, 2, "C normal destroy");
        assert_eq!(rust_frees, c_frees, "CONFIGS.md row 8");
        (faults.reset)();
    }
}

fn operation_name(api: &Api, code: i32) -> Vec<u8> {
    unsafe {
        CStr::from_ptr((api.get_operation_name)(code))
            .to_bytes()
            .to_vec()
    }
}

fn verify_operation_names(c_api: &Api, rust_api: &Api) {
    let expected: [&[u8]; 4] = [b"add", b"subtract", b"multiply", b"divide"];
    for (code, expected_name) in expected.into_iter().enumerate() {
        for _ in 0..64 {
            let c_name = operation_name(c_api, code as i32);
            let rust_name = operation_name(rust_api, code as i32);
            assert_eq!(c_name, expected_name, "C operation code {code}");
            assert_eq!(rust_name, c_name, "CONFIGS.md row {}", code + 9);
        }
    }
}

fn perform(api: &Api, a: i32, b: i32, operation: &CStr) -> i32 {
    unsafe { (api.perform_operation)(a, b, operation.as_ptr()) }
}

fn compare_perform(c_api: &Api, rust_api: &Api, row: usize, operation: &CStr, a: i32, b: i32) {
    assert_eq!(
        perform(rust_api, a, b, operation),
        perform(c_api, a, b, operation),
        "CONFIGS.md row {row}: ({a}, {b})"
    );
}

fn verify_perform_operations(c_api: &Api, rust_api: &Api) {
    let add = c"add";
    let subtract = c"subtract";
    let multiply = c"multiply";
    let divide = c"divide";
    let mut rng = Rng::new(0xd1b5_4a32_d192_ed03);

    for _ in 0..256 {
        compare_perform(
            c_api,
            rust_api,
            13,
            add,
            rng.inclusive(-1_000_000, 1_000_000),
            rng.inclusive(-1_000_000, 1_000_000),
        );
        compare_perform(
            c_api,
            rust_api,
            14,
            subtract,
            rng.inclusive(-1_000_000, 1_000_000),
            rng.inclusive(-1_000_000, 1_000_000),
        );
        compare_perform(
            c_api,
            rust_api,
            15,
            multiply,
            rng.inclusive(-30_000, 30_000),
            rng.inclusive(-30_000, 30_000),
        );

        let positive_a = rng.inclusive(0, 1_000_000);
        let positive_b = rng.inclusive(1, 10_000);
        compare_perform(c_api, rust_api, 16, divide, positive_a, positive_b);

        let magnitude_a = rng.inclusive(0, 1_000_000);
        let magnitude_b = rng.inclusive(1, 10_000);
        for (a, b) in [
            (-magnitude_a, magnitude_b),
            (magnitude_a, -magnitude_b),
            (-magnitude_a, -magnitude_b),
        ] {
            compare_perform(c_api, rust_api, 17, divide, a, b);
        }
    }

    for (row, operation, a, b) in [
        (13, add, i32::MAX, 1),
        (13, add, i32::MIN, -1),
        (14, subtract, i32::MIN, 1),
        (14, subtract, i32::MAX, -1),
        (15, multiply, i32::MAX, 2),
        (15, multiply, i32::MIN, -1),
    ] {
        compare_perform(c_api, rust_api, row, operation, a, b);
    }
}

fn buffapp_batch(api: &Api, cases: &[(i32, i32, i32, i32)]) -> (Vec<i32>, Vec<u8>) {
    capture_stdout(|| unsafe {
        cases
            .iter()
            .map(|&(a, b, c, d)| (api.buffapp)(a, b, c, d))
            .collect()
    })
}

fn verify_buffapp_cross_product(c_api: &Api, rust_api: &Api) {
    for (left_index, left) in OPERATION_STATES.into_iter().enumerate() {
        for (right_index, right) in OPERATION_STATES.into_iter().enumerate() {
            let row = 18 + left_index * OPERATION_STATES.len() + right_index;
            let mut rng =
                Rng::new(0x6a09_e667_f3bc_c909 ^ ((left_index as u64) << 32) ^ right_index as u64);
            let cases = (0..32)
                .map(|_| {
                    let (a, b) = left.arguments(&mut rng);
                    let (c, d) = right.arguments(&mut rng);
                    (a, b, c, d)
                })
                .collect::<Vec<_>>();

            let (c_results, c_output) = buffapp_batch(c_api, &cases);
            let (rust_results, rust_output) = buffapp_batch(rust_api, &cases);
            assert_eq!(
                rust_results,
                c_results,
                "CONFIGS.md row {row}: {} x {} return values",
                left.code(),
                right.code()
            );
            assert_eq!(
                rust_output,
                c_output,
                "CONFIGS.md row {row}: {} x {} output bytes",
                left.code(),
                right.code()
            );
        }
    }
}

fn create_with_malloc_failure(api: &Api, faults: &FaultApi, fail_after: i64) -> (bool, i64) {
    unsafe {
        (faults.reset)();
        (faults.malloc_after)(fail_after);
        let buffer = (api.create_buffer)(32);
        let free_calls = (faults.free_calls)();
        (faults.reset)();
        if !buffer.is_null() {
            (api.destroy_buffer)(buffer);
        }
        (buffer.is_null(), free_calls)
    }
}

fn append_with_realloc_failure(
    api: &Api,
    faults: &FaultApi,
) -> (i32, BufferSnapshot, BufferSnapshot) {
    unsafe {
        let buffer = (api.create_buffer)(2);
        assert!(!buffer.is_null());
        let before = snapshot(buffer);
        let string = c"reallocation required";
        (faults.reset)();
        (faults.realloc_after)(0);
        let result = (api.append_to_buffer)(buffer, string.as_ptr());
        (faults.reset)();
        let after = snapshot(buffer);
        (api.destroy_buffer)(buffer);
        (result, before, after)
    }
}

fn verify_allocation_failures(c_api: &Api, rust_api: &Api, faults: &FaultApi) {
    let c_first = create_with_malloc_failure(c_api, faults, 0);
    let rust_first = create_with_malloc_failure(rust_api, faults, 0);
    assert_eq!(c_first, (true, 0), "ERRORS.md row 1 C result");
    assert_eq!(rust_first, c_first, "ERRORS.md row 1");

    let c_second = create_with_malloc_failure(c_api, faults, 1);
    let rust_second = create_with_malloc_failure(rust_api, faults, 1);
    assert_eq!(c_second, (true, 1), "ERRORS.md row 2 C result");
    assert_eq!(rust_second, c_second, "ERRORS.md row 2");

    let c_append = append_with_realloc_failure(c_api, faults);
    let rust_append = append_with_realloc_failure(rust_api, faults);
    assert_eq!(c_append.0, -1, "ERRORS.md row 3 C result");
    assert_eq!(c_append.1, c_append.2, "C realloc failure mutated buffer");
    assert_eq!(rust_append, c_append, "ERRORS.md row 3");
}

fn destroy_with_null_data(api: &Api, faults: &FaultApi) -> i64 {
    unsafe {
        let buffer = (api.create_buffer)(32);
        assert!(!buffer.is_null());
        free((*buffer).data.cast::<c_void>());
        (*buffer).data = std::ptr::null_mut();
        (faults.reset)();
        (api.destroy_buffer)(buffer);
        let calls = (faults.free_calls)();
        (faults.reset)();
        calls
    }
}

fn verify_destroy_null_checks(c_api: &Api, rust_api: &Api, faults: &FaultApi) {
    unsafe {
        (faults.reset)();
        (c_api.destroy_buffer)(std::ptr::null_mut());
        let c_null_frees = (faults.free_calls)();
        (faults.reset)();
        (rust_api.destroy_buffer)(std::ptr::null_mut());
        let rust_null_frees = (faults.free_calls)();
        assert_eq!(c_null_frees, 0, "ERRORS.md row 4 C result");
        assert_eq!(rust_null_frees, c_null_frees, "ERRORS.md row 4");

        let c_data_null_frees = destroy_with_null_data(c_api, faults);
        let rust_data_null_frees = destroy_with_null_data(rust_api, faults);
        assert_eq!(c_data_null_frees, 1, "ERRORS.md row 5 C result");
        assert_eq!(rust_data_null_frees, c_data_null_frees, "ERRORS.md row 5");
    }
}

fn verify_rejected_operations(c_api: &Api, rust_api: &Api) {
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);
    for _ in 0..256 {
        let code = if rng.next_u32() & 1 == 0 {
            rng.inclusive(i32::MIN, -1)
        } else {
            rng.inclusive(4, i32::MAX)
        };
        assert_eq!(operation_name(c_api, code), b"unknown", "C default arm");
        assert_eq!(
            operation_name(rust_api, code),
            operation_name(c_api, code),
            "ERRORS.md row 6"
        );

        let dividend = rng.inclusive(i32::MIN, i32::MAX);
        assert_eq!(perform(c_api, dividend, 0, c"divide"), 0);
        assert_eq!(
            perform(rust_api, dividend, 0, c"divide"),
            perform(c_api, dividend, 0, c"divide"),
            "ERRORS.md row 7"
        );

        let length = rng.inclusive(0, 24) as usize;
        let mut bytes = (0..length)
            .map(|_| b'a' + (rng.next_u32() % 26) as u8)
            .collect::<Vec<_>>();
        if matches!(
            bytes.as_slice(),
            b"add" | b"subtract" | b"multiply" | b"divide"
        ) {
            bytes.push(b'x');
        }
        let unknown = CString::new(bytes).unwrap();
        let a = rng.inclusive(-1_000_000, 1_000_000);
        let b = rng.inclusive(-1_000_000, 1_000_000);
        assert_eq!(perform(c_api, a, b, &unknown), 0);
        assert_eq!(
            perform(rust_api, a, b, &unknown),
            perform(c_api, a, b, &unknown),
            "ERRORS.md row 8"
        );
    }
}

fn assert_same_signal(label: &str, c_call: impl FnOnce(), rust_call: impl FnOnce()) {
    let c_status = child_status(c_call);
    let rust_status = child_status(rust_call);
    let c_signal = terminating_signal(c_status);
    assert!(c_signal.is_some(), "{label}: C did not terminate by signal");
    assert_eq!(terminating_signal(rust_status), c_signal, "{label}");
}

fn verify_generic_boundaries(c_api: &Api, rust_api: &Api) {
    assert_same_signal(
        "ERRORS.md row G1",
        || unsafe {
            (c_api.append_to_buffer)(std::ptr::null_mut(), c"x".as_ptr());
        },
        || unsafe {
            (rust_api.append_to_buffer)(std::ptr::null_mut(), c"x".as_ptr());
        },
    );

    assert_same_signal(
        "ERRORS.md row G2",
        || unsafe {
            let buffer = (c_api.create_buffer)(16);
            (c_api.append_to_buffer)(buffer, std::ptr::null());
        },
        || unsafe {
            let buffer = (rust_api.create_buffer)(16);
            (rust_api.append_to_buffer)(buffer, std::ptr::null());
        },
    );

    assert_same_signal(
        "ERRORS.md row G3",
        || unsafe {
            (c_api.perform_operation)(1, 2, std::ptr::null());
        },
        || unsafe {
            (rust_api.perform_operation)(1, 2, std::ptr::null());
        },
    );

    unsafe {
        let c_zero = (c_api.create_buffer)(0);
        let rust_zero = (rust_api.create_buffer)(0);
        assert_eq!(rust_zero.is_null(), c_zero.is_null(), "ERRORS.md row G4");
        if !c_zero.is_null() {
            assert_eq!(snapshot(rust_zero), snapshot(c_zero), "zero capacity");
            (c_api.destroy_buffer)(c_zero);
            (rust_api.destroy_buffer)(rust_zero);
        }

        for capacity in [i32::MIN, -1, -4096] {
            let c_buffer = (c_api.create_buffer)(capacity);
            let rust_buffer = (rust_api.create_buffer)(capacity);
            assert!(c_buffer.is_null(), "C oversized capacity {capacity}");
            assert_eq!(
                rust_buffer.is_null(),
                c_buffer.is_null(),
                "ERRORS.md row G5: {capacity}"
            );
        }
    }

    assert_eq!(operation_name(c_api, 4), b"unknown");
    assert_eq!(
        operation_name(rust_api, 4),
        operation_name(c_api, 4),
        "ERRORS.md row G6"
    );

    assert_same_signal(
        "ERRORS.md row G7",
        || unsafe {
            (c_api.perform_operation)(i32::MIN, -1, c"divide".as_ptr());
        },
        || unsafe {
            (rust_api.perform_operation)(i32::MIN, -1, c"divide".as_ptr());
        },
    );
}

fn run_suite() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.exists(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.exists(),
        "missing Rust library: {}",
        rust_path.display()
    );

    unsafe {
        let faults = FaultApi::load();
        let c_api = Api::load(&c_path);
        let rust_api = Api::load(&rust_path);

        verify_create_buffer(&c_api, &rust_api);
        verify_append_paths(&c_api, &rust_api);
        verify_destroy_normal(&c_api, &rust_api, &faults);
        verify_operation_names(&c_api, &rust_api);
        verify_perform_operations(&c_api, &rust_api);
        verify_buffapp_cross_product(&c_api, &rust_api);

        verify_allocation_failures(&c_api, &rust_api, &faults);
        verify_destroy_null_checks(&c_api, &rust_api, &faults);
        verify_rejected_operations(&c_api, &rust_api);
        verify_generic_boundaries(&c_api, &rust_api);
    }
}

fn compile_fault_allocator() -> PathBuf {
    let output = profile_dir().join("libfault_alloc.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11", "-O2"])
        .arg(manifest_dir().join("tests/fault_alloc.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .expect("run C compiler for fault allocator");
    assert!(status.success(), "fault allocator compilation failed");
    output
}

fn compile_rust_library() {
    let status = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["build", "--no-default-features", "--features", ""])
        .current_dir(manifest_dir())
        .status()
        .expect("build Rust cdylib for external loading");
    assert!(status.success(), "Rust cdylib compilation failed");
}

#[test]
fn differential_ffi_matches_c() {
    if std::env::var_os("BUFFAPP_PRELOAD_WORKER").is_some() {
        run_suite();
        return;
    }

    compile_rust_library();
    let shim = compile_fault_allocator();
    let mut preload = shim.into_os_string();
    if let Some(existing) = std::env::var_os("LD_PRELOAD") {
        preload.push(":");
        preload.push(existing);
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "differential_ffi_matches_c", "--nocapture"])
        .env("BUFFAPP_PRELOAD_WORKER", "1")
        .env("LD_PRELOAD", preload)
        .output()
        .expect("launch fault-injection test worker");

    assert!(
        output.status.success(),
        "worker failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
