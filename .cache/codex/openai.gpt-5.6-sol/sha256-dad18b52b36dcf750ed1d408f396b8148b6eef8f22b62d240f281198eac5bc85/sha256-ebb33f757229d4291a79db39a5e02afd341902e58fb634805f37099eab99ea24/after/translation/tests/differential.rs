use libloading::Library;
use std::ffi::{c_char, c_int, c_long, c_void};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type MathOperation = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type IsValidOperation = unsafe extern "C" fn(c_char) -> bool;
type GetOperationPriority = unsafe extern "C" fn(c_int) -> c_int;
type ArithmeticOperation = MathOperation;
type SelectOperation = unsafe extern "C" fn(c_int) -> MathOperation;
type GetComputationTimestamp = unsafe extern "C" fn() -> c_long;
type AllocateResults = unsafe extern "C" fn(c_int) -> *mut ComputationResult;
type PerformComputationWithHistory =
    unsafe extern "C" fn(c_int, c_int, c_int, *mut *mut ComputationResult, *mut c_int) -> c_int;
type Mathop = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ComputationResult {
    value: c_int,
    timestamp: c_long,
    status: c_int,
}

struct Api {
    _library: Library,
    is_valid_operation: IsValidOperation,
    get_operation_priority: GetOperationPriority,
    add_operation: ArithmeticOperation,
    multiply_operation: ArithmeticOperation,
    subtract_operation: ArithmeticOperation,
    divide_operation: ArithmeticOperation,
    modulo_operation: ArithmeticOperation,
    select_operation: SelectOperation,
    get_computation_timestamp: GetComputationTimestamp,
    allocate_results: AllocateResults,
    perform_computation_with_history: PerformComputationWithHistory,
    mathop: Mathop,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("failed to load {}: {error}", $name))
            };
        }
        Self {
            is_valid_operation: symbol!("is_valid_operation", IsValidOperation),
            get_operation_priority: symbol!("get_operation_priority", GetOperationPriority),
            add_operation: symbol!("add_operation", ArithmeticOperation),
            multiply_operation: symbol!("multiply_operation", ArithmeticOperation),
            subtract_operation: symbol!("subtract_operation", ArithmeticOperation),
            divide_operation: symbol!("divide_operation", ArithmeticOperation),
            modulo_operation: symbol!("modulo_operation", ArithmeticOperation),
            select_operation: symbol!("select_operation", SelectOperation),
            get_computation_timestamp: symbol!(
                "get_computation_timestamp",
                GetComputationTimestamp
            ),
            allocate_results: symbol!("allocate_results", AllocateResults),
            perform_computation_with_history: symbol!(
                "perform_computation_with_history",
                PerformComputationWithHistory
            ),
            mathop: symbol!("mathop", Mathop),
            _library: library,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

struct FreshPair {
    pair: Pair,
    c_path: PathBuf,
    rust_path: PathBuf,
}

impl Drop for FreshPair {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.c_path);
        let _ = fs::remove_file(&self.rust_path);
    }
}

unsafe extern "C" {
    fn free(pointer: *mut c_void);
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

static SERIAL: Mutex<()> = Mutex::new(());
static UNIQUE_ID: AtomicU64 = AtomicU64::new(1);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-phwjKh.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libmathop_lib.so")
}

fn assert_library_files_exist() {
    for path in [c_library_path(), rust_library_path()] {
        assert!(
            path.is_file(),
            "required shared library does not exist: {}",
            path.display()
        );
    }
}

fn load_pair() -> Pair {
    assert_library_files_exist();
    unsafe {
        Pair {
            c: Api::load(&c_library_path()),
            rust: Api::load(&rust_library_path()),
        }
    }
}

fn load_fresh_pair() -> FreshPair {
    assert_library_files_exist();
    let id = UNIQUE_ID.fetch_add(1, Ordering::Relaxed);
    let prefix = format!("mathop-differential-{}-{id}", std::process::id());
    let c_path = std::env::temp_dir().join(format!("{prefix}-c.so"));
    let rust_path = std::env::temp_dir().join(format!("{prefix}-rust.so"));
    fs::copy(c_library_path(), &c_path).expect("copy C shared library");
    fs::copy(rust_library_path(), &rust_path).expect("copy Rust shared library");
    let pair = unsafe {
        Pair {
            c: Api::load(&c_path),
            rust: Api::load(&rust_path),
        }
    };
    FreshPair {
        pair,
        c_path,
        rust_path,
    }
}

#[derive(Clone)]
struct FixedRng(u64);

impl FixedRng {
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

    fn inclusive(&mut self, low: i32, high: i32) -> i32 {
        debug_assert!(low <= high);
        let width = (high as i64 - low as i64 + 1) as u64;
        if width == 1_u64 << 32 {
            self.next_u32() as i32
        } else {
            (low as i64 + (self.next_u32() as u64 % width) as i64) as i32
        }
    }

    fn nonzero(&mut self, low: i32, high: i32) -> i32 {
        loop {
            let value = self.inclusive(low, high);
            if value != 0 {
                return value;
            }
        }
    }
}

unsafe fn result_bytes(pointer: *const ComputationResult, count: usize) -> Vec<u8> {
    let byte_count = count * std::mem::size_of::<ComputationResult>();
    unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), byte_count) }.to_vec()
}

unsafe fn initialize_history(pointer: *mut ComputationResult, count: usize) {
    for index in 0..count {
        unsafe {
            (*pointer.add(index)).value = 1000 + index as i32;
            (*pointer.add(index)).timestamp = 2000 + index as c_long;
            (*pointer.add(index)).status = -7;
        }
    }
}

unsafe fn release(pointer: *mut ComputationResult) {
    unsafe { free(pointer.cast()) };
}

fn capture_stdout(call: impl FnOnce() -> i32) -> (i32, Vec<u8>) {
    let id = UNIQUE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("mathop-stdout-{}-{id}.txt", std::process::id()));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create stdout capture file");

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(file.as_raw_fd(), 1), 1);
        let result = call();
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        file.seek(SeekFrom::Start(0)).expect("rewind capture");
        let mut output = Vec::new();
        file.read_to_end(&mut output).expect("read capture");
        drop(file);
        fs::remove_file(path).expect("remove capture");
        (result, output)
    }
}

fn child_termination_signal(call: impl FnOnce()) -> i32 {
    unsafe {
        let child = fork();
        assert!(child >= 0, "fork failed");
        if child == 0 {
            call();
            _exit(0);
        }
        let mut status = 0;
        assert_eq!(waitpid(child, &mut status, 0), child);
        let signal = status & 0x7f;
        assert_ne!(signal, 0, "invalid call unexpectedly returned normally");
        signal
    }
}

#[test]
fn valid_low_level_configuration_matrix_matches() {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let pair = load_pair();
    let mut rng = FixedRng::new(0x26dc_631e_a532_b183);

    for raw in i8::MIN..=i8::MAX {
        let c = unsafe { (pair.c.is_valid_operation)(raw) };
        let rust = unsafe { (pair.rust.is_valid_operation)(raw) };
        assert_eq!(rust, c, "is_valid_operation({raw})");
    }

    for operation in 1..=5 {
        for _ in 0..128 {
            let c = unsafe { (pair.c.get_operation_priority)(operation) };
            let rust = unsafe { (pair.rust.get_operation_priority)(operation) };
            assert_eq!(rust, c, "get_operation_priority({operation})");
        }
    }

    for _ in 0..512 {
        let a = rng.inclusive(-30_000, 30_000);
        let b = rng.inclusive(-30_000, 30_000);
        let unused = rng.inclusive(i32::MIN, i32::MAX);
        for (name, c_function, rust_function) in [
            ("add", pair.c.add_operation, pair.rust.add_operation),
            (
                "subtract",
                pair.c.subtract_operation,
                pair.rust.subtract_operation,
            ),
        ] {
            let c = unsafe { c_function(a, b, unused) };
            let rust = unsafe { rust_function(a, b, unused) };
            assert_eq!(rust, c, "{name}({a}, {b}, {unused})");
        }

        let mul_a = rng.inclusive(-30_000, 30_000);
        let mul_b = rng.inclusive(-30_000, 30_000);
        let c = unsafe { (pair.c.multiply_operation)(mul_a, mul_b, unused) };
        let rust = unsafe { (pair.rust.multiply_operation)(mul_a, mul_b, unused) };
        assert_eq!(rust, c, "multiply({mul_a}, {mul_b}, {unused})");

        let divisor = rng.nonzero(-30_000, 30_000);
        let c_div = unsafe { (pair.c.divide_operation)(a, divisor, unused) };
        let rust_div = unsafe { (pair.rust.divide_operation)(a, divisor, unused) };
        assert_eq!(rust_div, c_div, "divide({a}, {divisor}, {unused})");

        let c_mod = unsafe { (pair.c.modulo_operation)(a, divisor, unused) };
        let rust_mod = unsafe { (pair.rust.modulo_operation)(a, divisor, unused) };
        assert_eq!(rust_mod, c_mod, "modulo({a}, {divisor}, {unused})");
    }

    for operation in [1, 2, 3, 4, 5, -99, 0, 6, 99] {
        let c_callback = unsafe { (pair.c.select_operation)(operation) };
        let rust_callback = unsafe { (pair.rust.select_operation)(operation) };
        for _ in 0..128 {
            let a = rng.inclusive(-20_000, 20_000);
            let mut b = rng.inclusive(-20_000, 20_000);
            if matches!(operation, 4 | 5) && b == 0 {
                b = 1;
            }
            let unused = rng.inclusive(-100, 100);
            let c = unsafe { c_callback(a, b, unused) };
            let rust = unsafe { rust_callback(a, b, unused) };
            assert_eq!(rust, c, "selected operation {operation}");
        }
    }

    for _ in 0..64 {
        let c = unsafe { (pair.c.get_computation_timestamp)() };
        let rust = unsafe { (pair.rust.get_computation_timestamp)() };
        assert_eq!(rust, c, "get_computation_timestamp");
    }

    for count in std::iter::once(1).chain(2..=64) {
        let c_pointer = unsafe { (pair.c.allocate_results)(count) };
        let rust_pointer = unsafe { (pair.rust.allocate_results)(count) };
        assert_eq!(
            rust_pointer.is_null(),
            c_pointer.is_null(),
            "allocate_results({count}) nullness"
        );
        assert!(
            !c_pointer.is_null(),
            "small C allocation unexpectedly failed"
        );
        let c_bytes = unsafe { result_bytes(c_pointer, count as usize) };
        let rust_bytes = unsafe { result_bytes(rust_pointer, count as usize) };
        assert_eq!(rust_bytes, c_bytes, "allocate_results({count}) bytes");
        assert!(c_bytes.iter().all(|byte| *byte == 0));
        unsafe {
            release(c_pointer);
            release(rust_pointer);
        }
    }
}

#[test]
fn valid_history_configuration_matrix_matches() {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let pair = load_pair();
    let mut rng = FixedRng::new(0x9942_8b62_1ef0_75d5);

    for operation in [1, 2, 3, 4, 5, 77] {
        for shape in 0..4 {
            for _ in 0..64 {
                let a = rng.inclusive(-20_000, 20_000);
                let mut b = rng.inclusive(-20_000, 20_000);
                if matches!(operation, 4 | 5) && b == 0 {
                    b = -1;
                }

                let (mut c_history, mut rust_history, initial_count) = unsafe {
                    if shape == 0 {
                        (ptr::null_mut(), ptr::null_mut(), rng.inclusive(-20, 20))
                    } else {
                        let c_pointer = (pair.c.allocate_results)(10);
                        let rust_pointer = (pair.rust.allocate_results)(10);
                        assert!(!c_pointer.is_null() && !rust_pointer.is_null());
                        initialize_history(c_pointer, 10);
                        initialize_history(rust_pointer, 10);
                        let count = match shape {
                            1 => 0,
                            2 => 9,
                            3 => rng.inclusive(10, 20),
                            _ => unreachable!(),
                        };
                        (c_pointer, rust_pointer, count)
                    }
                };
                let mut c_count = initial_count;
                let mut rust_count = initial_count;

                let c_result = unsafe {
                    (pair.c.perform_computation_with_history)(
                        a,
                        b,
                        operation,
                        &mut c_history,
                        &mut c_count,
                    )
                };
                let rust_result = unsafe {
                    (pair.rust.perform_computation_with_history)(
                        a,
                        b,
                        operation,
                        &mut rust_history,
                        &mut rust_count,
                    )
                };

                assert_eq!(
                    rust_result, c_result,
                    "operation {operation}, shape {shape}"
                );
                assert_eq!(rust_count, c_count, "history count");
                assert_eq!(
                    rust_history.is_null(),
                    c_history.is_null(),
                    "history nullness"
                );
                assert!(!c_history.is_null());
                let c_bytes = unsafe { result_bytes(c_history, 10) };
                let rust_bytes = unsafe { result_bytes(rust_history, 10) };
                assert_eq!(
                    rust_bytes, c_bytes,
                    "history bytes for operation {operation}, shape {shape}"
                );

                unsafe {
                    release(c_history);
                    release(rust_history);
                }
            }
        }
    }
}

fn first_parameter(valid_validation: bool, iteration: i32, rng: &mut FixedRng) -> i32 {
    if valid_validation {
        49 + iteration.rem_euclid(5) + 128 * rng.inclusive(0, 1)
    } else {
        loop {
            let candidate = rng.inclusive(-200, 200);
            let remainder = candidate % 128;
            if !(49..=53).contains(&remainder) {
                return candidate;
            }
        }
    }
}

fn parameter_for_first_operation(class: i32, iteration: i32, rng: &mut FixedRng) -> i32 {
    if class == 0 {
        let selected = [-3, -2, -1, 0][iteration.rem_euclid(4) as usize];
        let remainder = selected - 1;
        remainder - 5 * rng.inclusive(0, 10)
    } else {
        (class - 1) + 5 * rng.inclusive(0, 10)
    }
}

fn parameter_for_second_operation(class: i32, iteration: i32, rng: &mut FixedRng) -> i32 {
    let shifted = if class == 0 {
        let selected = [-3, -2, -1, 0][iteration.rem_euclid(4) as usize];
        let remainder = selected - 1;
        remainder - 5 * rng.inclusive(0, 10)
    } else {
        (class - 1) + 5 * rng.inclusive(0, 10)
    };
    shifted - 1
}

#[test]
fn valid_mathop_cross_product_and_stdout_match() {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    for first_class in 0..=5 {
        for second_class in 0..=5 {
            let fresh = load_fresh_pair();
            let pair = &fresh.pair;
            let seed = 0xd6e8_feb8_6659_fd93 ^ ((first_class as u64) << 32) ^ second_class as u64;
            let mut rng = FixedRng::new(seed);

            for iteration in 0..16 {
                let valid_validation = iteration % 2 == 0;
                let param1 = first_parameter(valid_validation, iteration, &mut rng);
                let mut param2 = rng.inclusive(-40, 40);
                if matches!(first_class, 4 | 5) && param2 == 0 {
                    param2 = 1;
                }
                let param3 = parameter_for_first_operation(first_class, iteration, &mut rng);
                let param4 = parameter_for_second_operation(second_class, iteration, &mut rng);

                let (c_result, c_stdout) =
                    capture_stdout(|| unsafe { (pair.c.mathop)(param1, param2, param3, param4) });
                let (rust_result, rust_stdout) = capture_stdout(|| unsafe {
                    (pair.rust.mathop)(param1, param2, param3, param4)
                });
                assert_eq!(
                    rust_result, c_result,
                    "mathop({param1}, {param2}, {param3}, {param4})"
                );
                assert_eq!(
                    rust_stdout, c_stdout,
                    "mathop stdout for ({param1}, {param2}, {param3}, {param4})"
                );
            }
        }
    }
}

#[test]
fn explicit_error_and_generic_boundary_matrix_matches() {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let pair = load_pair();
    let mut rng = FixedRng::new(0x2d35_8dcc_aa6c_78a5);

    for _ in 0..512 {
        let a = rng.inclusive(i32::MIN, i32::MAX);
        let unused = rng.inclusive(i32::MIN, i32::MAX);
        let c_divide = unsafe { (pair.c.divide_operation)(a, 0, unused) };
        let rust_divide = unsafe { (pair.rust.divide_operation)(a, 0, unused) };
        assert_eq!(rust_divide, c_divide);
        assert_eq!(c_divide, 0);

        let c_modulo = unsafe { (pair.c.modulo_operation)(a, 0, unused) };
        let rust_modulo = unsafe { (pair.rust.modulo_operation)(a, 0, unused) };
        assert_eq!(rust_modulo, c_modulo);
        assert_eq!(c_modulo, 0);
    }

    for invalid_operation in [i32::MIN, -100, -1, 0, 6, 100, i32::MAX] {
        let c_callback = unsafe { (pair.c.select_operation)(invalid_operation) };
        let rust_callback = unsafe { (pair.rust.select_operation)(invalid_operation) };
        for _ in 0..64 {
            let a = rng.inclusive(-10_000, 10_000);
            let b = rng.inclusive(-10_000, 10_000);
            let c = unsafe { c_callback(a, b, 123) };
            let rust = unsafe { rust_callback(a, b, 123) };
            assert_eq!(rust, c, "invalid operation {invalid_operation}");
            assert_eq!(c, a + b);
        }

        let mut c_history = ptr::null_mut();
        let mut rust_history = ptr::null_mut();
        let mut c_count = 999;
        let mut rust_count = 999;
        let c = unsafe {
            (pair.c.perform_computation_with_history)(
                123,
                -45,
                invalid_operation,
                &mut c_history,
                &mut c_count,
            )
        };
        let rust = unsafe {
            (pair.rust.perform_computation_with_history)(
                123,
                -45,
                invalid_operation,
                &mut rust_history,
                &mut rust_count,
            )
        };
        assert_eq!(rust, c);
        assert_eq!(c, 78);
        assert_eq!(rust_count, c_count);
        assert_eq!(unsafe { result_bytes(rust_history, 10) }, unsafe {
            result_bytes(c_history, 10)
        });
        unsafe {
            release(c_history);
            release(rust_history);
        }
    }

    for count in [0, i32::MAX, -1] {
        let c_pointer = unsafe { (pair.c.allocate_results)(count) };
        let rust_pointer = unsafe { (pair.rust.allocate_results)(count) };
        assert_eq!(
            rust_pointer.is_null(),
            c_pointer.is_null(),
            "allocate_results({count})"
        );
        unsafe {
            release(c_pointer);
            release(rust_pointer);
        }
    }

    let c_null_history_signal = child_termination_signal(|| unsafe {
        let mut count = 0;
        (pair.c.perform_computation_with_history)(1, 2, 1, ptr::null_mut(), &mut count);
    });
    let rust_null_history_signal = child_termination_signal(|| unsafe {
        let mut count = 0;
        (pair.rust.perform_computation_with_history)(1, 2, 1, ptr::null_mut(), &mut count);
    });
    assert_eq!(rust_null_history_signal, c_null_history_signal);

    let c_null_count_signal = child_termination_signal(|| unsafe {
        let mut history = ptr::null_mut();
        (pair.c.perform_computation_with_history)(1, 2, 1, &mut history, ptr::null_mut());
    });
    let rust_null_count_signal = child_termination_signal(|| unsafe {
        let mut history = ptr::null_mut();
        (pair.rust.perform_computation_with_history)(1, 2, 1, &mut history, ptr::null_mut());
    });
    assert_eq!(rust_null_count_signal, c_null_count_signal);
}
