use libloading::Library;
use std::ffi::{c_char, c_int, c_long, c_void};
use std::mem::size_of;
use std::os::fd::RawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::slice;

type MathOperation = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type IsValidOperation = unsafe extern "C" fn(c_char) -> bool;
type GetOperationPriority = unsafe extern "C" fn(c_int) -> c_int;
type BinaryOperation = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type SelectOperation = unsafe extern "C" fn(c_int) -> MathOperation;
type GetTimestamp = unsafe extern "C" fn() -> c_long;
type AllocateResults = unsafe extern "C" fn(c_int) -> *mut ComputationResult;
type PerformWithHistory =
    unsafe extern "C" fn(c_int, c_int, c_int, *mut *mut ComputationResult, *mut c_int) -> c_int;
type Mathop = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
struct ComputationResult {
    value: c_int,
    timestamp: c_long,
    status: c_int,
}

struct Api {
    _library: Library,
    is_valid_operation: IsValidOperation,
    get_operation_priority: GetOperationPriority,
    add_operation: BinaryOperation,
    multiply_operation: BinaryOperation,
    subtract_operation: BinaryOperation,
    divide_operation: BinaryOperation,
    modulo_operation: BinaryOperation,
    select_operation: SelectOperation,
    get_computation_timestamp: GetTimestamp,
    allocate_results: AllocateResults,
    perform_computation_with_history: PerformWithHistory,
    mathop: Mathop,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let loaded = unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| {
                        panic!("failed to load {} from {}: {error}", $name, path.display())
                    });
                *loaded
            }};
        }

        Self {
            is_valid_operation: symbol!("is_valid_operation", IsValidOperation),
            get_operation_priority: symbol!("get_operation_priority", GetOperationPriority),
            add_operation: symbol!("add_operation", BinaryOperation),
            multiply_operation: symbol!("multiply_operation", BinaryOperation),
            subtract_operation: symbol!("subtract_operation", BinaryOperation),
            divide_operation: symbol!("divide_operation", BinaryOperation),
            modulo_operation: symbol!("modulo_operation", BinaryOperation),
            select_operation: symbol!("select_operation", SelectOperation),
            get_computation_timestamp: symbol!("get_computation_timestamp", GetTimestamp),
            allocate_results: symbol!("allocate_results", AllocateResults),
            perform_computation_with_history: symbol!(
                "perform_computation_with_history",
                PerformWithHistory
            ),
            mathop: symbol!("mathop", Mathop),
            _library: library,
        }
    }
}

unsafe extern "C" {
    fn close(fd: RawFd) -> c_int;
    fn dup(fd: RawFd) -> RawFd;
    fn dup2(old_fd: RawFd, new_fd: RawFd) -> RawFd;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(pointer: *mut c_void);
    fn pipe(fds: *mut RawFd) -> c_int;
    fn read(fd: RawFd, buffer: *mut c_void, count: usize) -> isize;
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn moderate(&mut self) -> i32 {
        (self.next_u32() % 2_000_001) as i32 - 1_000_000
    }

    fn positive(&mut self) -> i32 {
        (self.next_u32() % 1_000_000 + 1) as i32
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        manifest.join("../c_src/build/libharvest-work-R81tG0.so"),
        manifest.join("target/release/libmathop_lib.so"),
    )
}

fn load_apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}; run cargo build --release first",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn result_bytes(results: *const ComputationResult, count: usize) -> Vec<u8> {
    unsafe {
        slice::from_raw_parts(results.cast::<u8>(), count * size_of::<ComputationResult>()).to_vec()
    }
}

unsafe fn release(pointer: *mut ComputationResult) {
    unsafe { free(pointer.cast()) };
}

fn assert_binary_matches(
    c_function: BinaryOperation,
    rust_function: BinaryOperation,
    inputs: impl IntoIterator<Item = (i32, i32)>,
) {
    for (a, b) in inputs {
        let c_result = unsafe { c_function(a, b, 0x1234_5678) };
        let rust_result = unsafe { rust_function(a, b, 0x1234_5678) };
        assert_eq!(c_result, rust_result, "a={a}, b={b}");
    }
}

fn random_pairs(seed: u64, count: usize) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed);
    (0..count).map(|_| (rng.i32(), rng.i32())).collect()
}

fn signed_division_pairs(
    seed: u64,
    count: usize,
    negative_a: bool,
    negative_b: bool,
) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed);
    (0..count)
        .map(|_| {
            let a = rng.positive();
            let b = rng.positive();
            (
                if negative_a { -a } else { a },
                if negative_b { -b } else { b },
            )
        })
        .collect()
}

fn exercise_scalar_and_selector_rows(c: &Api, rust: &Api) {
    for character in b'1'..=b'5' {
        assert_eq!(
            unsafe { (c.is_valid_operation)(character as c_char) },
            unsafe { (rust.is_valid_operation)(character as c_char) }
        );
    }

    let mut rng = Rng::new(0xC2C2_C2C2);
    for _ in 0..512 {
        let character = (b'1' + (rng.next_u32() % 5) as u8) as c_char;
        assert_eq!(unsafe { (c.is_valid_operation)(character) }, unsafe {
            (rust.is_valid_operation)(character)
        });
    }

    for operation in [1, 2, 3, 4, 5]
        .into_iter()
        .chain((0..512).map(|_| rng.i32()))
    {
        assert_eq!(
            unsafe { (c.get_operation_priority)(operation) },
            unsafe { (rust.get_operation_priority)(operation) },
            "operation={operation}"
        );
    }

    let boundaries = [
        (0, 0),
        (i32::MIN, 0),
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (-1, i32::MIN),
    ];
    for (c_function, rust_function, seed) in [
        (c.add_operation, rust.add_operation, 0xA001),
        (c.multiply_operation, rust.multiply_operation, 0xA002),
        (c.subtract_operation, rust.subtract_operation, 0xA003),
    ] {
        assert_binary_matches(c_function, rust_function, boundaries);
        assert_binary_matches(c_function, rust_function, random_pairs(seed, 512));
    }

    for (index, (negative_a, negative_b)) in
        [(false, false), (true, false), (false, true), (true, true)]
            .into_iter()
            .enumerate()
    {
        let pairs = signed_division_pairs(0xD100 + index as u64, 512, negative_a, negative_b);
        assert_binary_matches(c.divide_operation, rust.divide_operation, pairs.clone());
        assert_binary_matches(c.modulo_operation, rust.modulo_operation, pairs);
    }

    for operation in [1, 2, 3, 4, 5, -1, 0, 6, i32::MIN, i32::MAX] {
        let c_selected = unsafe { (c.select_operation)(operation) };
        let rust_selected = unsafe { (rust.select_operation)(operation) };
        let mut rng = Rng::new(0x5E1E_C700_u64.wrapping_add(operation as u64));
        for _ in 0..512 {
            let a = rng.moderate();
            let mut b = rng.moderate();
            if matches!(operation, 4 | 5) && b == 0 {
                b = 1;
            }
            let unused = rng.i32();
            assert_eq!(
                unsafe { c_selected(a, b, unused) },
                unsafe { rust_selected(a, b, unused) },
                "selected operation={operation}, a={a}, b={b}"
            );
        }
    }

    for _ in 0..512 {
        assert_eq!(unsafe { (c.get_computation_timestamp)() }, unsafe {
            (rust.get_computation_timestamp)()
        });
    }
}

fn exercise_allocation_rows(c: &Api, rust: &Api) {
    let mut rng = Rng::new(0xA110_CA7E);
    let counts = std::iter::repeat_n(0, 64)
        .chain(std::iter::repeat_n(1, 64))
        .chain((0..512).map(|_| (rng.next_u32() % 64 + 2) as i32));

    for count in counts {
        let c_results = unsafe { (c.allocate_results)(count) };
        let rust_results = unsafe { (rust.allocate_results)(count) };
        assert_eq!(c_results.is_null(), rust_results.is_null(), "count={count}");
        if count > 0 {
            assert!(
                !c_results.is_null(),
                "C allocation failed for count={count}"
            );
            assert_eq!(
                result_bytes(c_results, count as usize),
                result_bytes(rust_results, count as usize),
                "count={count}"
            );
        }
        unsafe {
            release(c_results);
            release(rust_results);
        }
    }
}

fn operation_inputs(operation: i32, rng: &mut Rng) -> (i32, i32) {
    let a = rng.moderate();
    let mut b = rng.moderate();
    if matches!(operation, 4 | 5) && b == 0 {
        b = 1;
    }
    (a, b)
}

fn initialize_history(pointer: *mut ComputationResult, count: usize, rng: &mut Rng) {
    for index in 0..count {
        unsafe {
            pointer.add(index).write(ComputationResult {
                value: rng.i32(),
                timestamp: rng.i32() as c_long,
                status: rng.i32(),
            });
        }
    }
}

fn compare_history_call(
    c: &Api,
    rust: &Api,
    operation: i32,
    state: usize,
    a: i32,
    b: i32,
    initial_count: i32,
    seed: u64,
) {
    let mut c_history = ptr::null_mut();
    let mut rust_history = ptr::null_mut();
    let mut c_count = initial_count;
    let mut rust_count = initial_count;

    if state != 0 {
        c_history = unsafe { (c.allocate_results)(10) };
        rust_history = unsafe { (rust.allocate_results)(10) };
        assert!(!c_history.is_null() && !rust_history.is_null());
        let mut c_rng = Rng::new(seed);
        let mut rust_rng = Rng::new(seed);
        initialize_history(c_history, 10, &mut c_rng);
        initialize_history(rust_history, 10, &mut rust_rng);
    }

    let c_result = unsafe {
        (c.perform_computation_with_history)(a, b, operation, &mut c_history, &mut c_count)
    };
    let rust_result = unsafe {
        (rust.perform_computation_with_history)(a, b, operation, &mut rust_history, &mut rust_count)
    };

    assert_eq!(
        c_result, rust_result,
        "operation={operation}, state={state}"
    );
    assert_eq!(c_count, rust_count, "operation={operation}, state={state}");
    assert_eq!(
        result_bytes(c_history, 10),
        result_bytes(rust_history, 10),
        "operation={operation}, state={state}, count={initial_count}"
    );

    unsafe {
        release(c_history);
        release(rust_history);
    }
}

fn exercise_history_rows(c: &Api, rust: &Api) {
    for operation in [1, 2, 3, 4, 5, -17] {
        for state in 0..3 {
            let mut rng = Rng::new(
                0xA157_0000_u64
                    .wrapping_add((operation as u64).wrapping_mul(31))
                    .wrapping_add(state as u64),
            );
            for case in 0..128 {
                let (a, b) = operation_inputs(operation, &mut rng);
                let initial_count = match state {
                    0 => rng.i32(),
                    1 => (rng.next_u32() % 10) as i32,
                    _ => 10 + (rng.next_u32() % 100) as i32,
                };
                compare_history_call(c, rust, operation, state, a, b, initial_count, rng.0 ^ case);
            }
        }
    }
}

fn capture_stdout(function: Mathop, arguments: [i32; 4]) -> (i32, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        let mut pipe_fds = [-1, -1];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        let result = function(arguments[0], arguments[1], arguments[2], arguments[3]);
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let bytes_read = read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(bytes_read >= 0);
            if bytes_read == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..bytes_read as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0);
        (result, output)
    }
}

fn value_with_remainder(remainder: i32, factor: i32) -> i32 {
    match remainder.cmp(&0) {
        std::cmp::Ordering::Less => remainder - 5 * factor,
        std::cmp::Ordering::Equal => 5 * factor,
        std::cmp::Ordering::Greater => remainder + 5 * factor,
    }
}

fn exercise_mathop_matrix(c: &Api, rust: &Api) {
    let selections = [-3, -2, -1, 0, 1, 2, 3, 4, 5];
    let mut sequence = 0_u64;

    for selected in selections {
        for second in selections {
            for valid_character in [false, true] {
                let mut rng = Rng::new(
                    0x4D41_5448_0000_0000_u64
                        ^ ((selected as u64) << 16)
                        ^ ((second as u64) << 8)
                        ^ valid_character as u64,
                );
                for _ in 0..64 {
                    let factor3 = (rng.next_u32() % 1000) as i32;
                    let factor4 = (rng.next_u32() % 1000) as i32;
                    let param3 = value_with_remainder(selected - 1, factor3);
                    let param4 = value_with_remainder(second - 1, factor4) - 1;
                    let param1 = if valid_character {
                        (b'1' + (rng.next_u32() % 5) as u8) as i32
                            + 128 * (rng.next_u32() % 10_000) as i32
                    } else {
                        (b'6' + (rng.next_u32() % 40) as u8) as i32
                            + 128 * (rng.next_u32() % 10_000) as i32
                    };
                    let mut param2 = rng.moderate();
                    if selected == 4 && param2 == 0 {
                        param2 = 1;
                    }

                    assert_eq!(param3 % 5 + 1, selected);
                    assert_eq!((param4 + 1) % 5 + 1, second);
                    assert_eq!(
                        (param1 % 128 >= b'1' as i32 && param1 % 128 <= b'5' as i32),
                        valid_character
                    );

                    let arguments = [param1, param2, param3, param4];
                    let c_observed = capture_stdout(c.mathop, arguments);
                    let rust_observed = capture_stdout(rust.mathop, arguments);
                    assert_eq!(
                        c_observed, rust_observed,
                        "S={selected}, T={second}, valid={valid_character}, sequence={sequence}, args={arguments:?}"
                    );
                    sequence += 1;
                }
            }
        }
    }
}

fn exercise_error_rows(c: &Api, rust: &Api) {
    let invalid_characters = [0_i8, 1, b'0' as i8, -1, i8::MIN, b'6' as i8, i8::MAX];
    for character in invalid_characters {
        assert_eq!(
            unsafe { (c.is_valid_operation)(character as c_char) },
            unsafe { (rust.is_valid_operation)(character as c_char) },
            "character={character}"
        );
        assert!(!unsafe { (c.is_valid_operation)(character as c_char) });
    }

    let mut rng = Rng::new(0xE440_0000);
    for _ in 0..512 {
        let a = rng.i32();
        assert_eq!(unsafe { (c.divide_operation)(a, 0, rng.i32()) }, 0);
        assert_eq!(unsafe { (rust.divide_operation)(a, 0, rng.i32()) }, 0);
        assert_eq!(unsafe { (c.modulo_operation)(a, 0, rng.i32()) }, 0);
        assert_eq!(unsafe { (rust.modulo_operation)(a, 0, rng.i32()) }, 0);
    }
}

fn exercise_generic_boundaries(c: &Api, rust: &Api) {
    for count in [0, -1, i32::MIN, i32::MAX] {
        let c_pointer = unsafe { (c.allocate_results)(count) };
        let rust_pointer = unsafe { (rust.allocate_results)(count) };
        assert_eq!(
            c_pointer.is_null(),
            rust_pointer.is_null(),
            "boundary allocation count={count}"
        );
        unsafe {
            release(c_pointer);
            release(rust_pointer);
        }
    }

    for operation in [i32::MIN, -1, 0, 6, i32::MAX] {
        let c_selected = unsafe { (c.select_operation)(operation) };
        let rust_selected = unsafe { (rust.select_operation)(operation) };
        assert_eq!(
            unsafe { c_selected(123, -45, 0) },
            unsafe { rust_selected(123, -45, 0) },
            "out-of-range enum={operation}"
        );
    }
}

#[test]
fn differential_surface() {
    let (c, rust) = load_apis();
    exercise_scalar_and_selector_rows(&c, &rust);
    exercise_allocation_rows(&c, &rust);
    exercise_history_rows(&c, &rust);
    exercise_mathop_matrix(&c, &rust);
    exercise_error_rows(&c, &rust);
    exercise_generic_boundaries(&c, &rust);
}

fn run_crash_child(library_kind: &str, null_case: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("null_pointer_child")
        .arg("--nocapture")
        .env("DIFFERENTIAL_CRASH_CHILD", "1")
        .env("DIFFERENTIAL_LIBRARY", library_kind)
        .env("DIFFERENTIAL_NULL_CASE", null_case)
        .status()
        .expect("run crash child")
}

#[test]
fn null_pointer_boundaries_match_process_behavior() {
    for null_case in ["history", "history_count"] {
        let c_status = run_crash_child("c", null_case);
        let rust_status = run_crash_child("rust", null_case);
        assert_eq!(
            (c_status.code(), c_status.signal()),
            (rust_status.code(), rust_status.signal()),
            "null case={null_case}, C={c_status:?}, Rust={rust_status:?}"
        );
        assert!(
            !c_status.success(),
            "C unexpectedly accepted null case={null_case}"
        );
    }
}

#[test]
fn null_pointer_child() {
    if std::env::var_os("DIFFERENTIAL_CRASH_CHILD").is_none() {
        return;
    }

    let (c, rust) = load_apis();
    let api = match std::env::var("DIFFERENTIAL_LIBRARY").as_deref() {
        Ok("c") => &c,
        Ok("rust") => &rust,
        value => panic!("unexpected library kind: {value:?}"),
    };
    let mut history = ptr::null_mut();
    let mut count = 0;

    unsafe {
        match std::env::var("DIFFERENTIAL_NULL_CASE").as_deref() {
            Ok("history") => {
                (api.perform_computation_with_history)(1, 2, 1, ptr::null_mut(), &mut count);
            }
            Ok("history_count") => {
                (api.perform_computation_with_history)(1, 2, 1, &mut history, ptr::null_mut());
            }
            value => panic!("unexpected null case: {value:?}"),
        }
    }
}
