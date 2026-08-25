use libloading::Library;
use std::ffi::{c_char, c_int, c_long, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const OP_ADD: c_uint = 1;
const OP_MULTIPLY: c_uint = 2;
const OP_SUBTRACT: c_uint = 3;
const OP_DIVIDE: c_uint = 4;
const OP_MODULO: c_uint = 5;

#[repr(C)]
#[derive(Clone, Copy)]
struct ComputationResult {
    value: c_int,
    timestamp: c_long,
    status: c_int,
}

type IsValidFn = unsafe extern "C" fn(c_char) -> bool;
type PriorityFn = unsafe extern "C" fn(c_uint) -> c_int;
type MathFn = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type SelectFn = unsafe extern "C" fn(c_uint) -> MathFn;
type TimestampFn = unsafe extern "C" fn() -> c_long;
type AllocateFn = unsafe extern "C" fn(c_int) -> *mut ComputationResult;
type PerformFn =
    unsafe extern "C" fn(c_int, c_int, c_uint, *mut *mut ComputationResult, *mut c_int) -> c_int;
type MathopFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    is_valid_operation: IsValidFn,
    get_operation_priority: PriorityFn,
    add_operation: MathFn,
    multiply_operation: MathFn,
    subtract_operation: MathFn,
    divide_operation: MathFn,
    modulo_operation: MathFn,
    select_operation: SelectFn,
    get_computation_timestamp: TimestampFn,
    allocate_results: AllocateFn,
    perform_computation_with_history: PerformFn,
    mathop: MathopFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! load {
            ($name:literal, $ty:ty) => {{
                let symbol = unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("failed to load {}: {error}", $name));
                *symbol
            }};
        }

        Self {
            is_valid_operation: load!("is_valid_operation", IsValidFn),
            get_operation_priority: load!("get_operation_priority", PriorityFn),
            add_operation: load!("add_operation", MathFn),
            multiply_operation: load!("multiply_operation", MathFn),
            subtract_operation: load!("subtract_operation", MathFn),
            divide_operation: load!("divide_operation", MathFn),
            modulo_operation: load!("modulo_operation", MathFn),
            select_operation: load!("select_operation", SelectFn),
            get_computation_timestamp: load!("get_computation_timestamp", TimestampFn),
            allocate_results: load!("allocate_results", AllocateFn),
            perform_computation_with_history: load!("perform_computation_with_history", PerformFn),
            mathop: load!("mathop", MathopFn),
            _library: library,
        }
    }
}

unsafe extern "C" {
    fn free(pointer: *mut c_void);
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

fn call_mathop_captured(
    function: MathopFn,
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> (c_int, Vec<u8>) {
    unsafe {
        let mut pipe_fds = [0; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        fflush(std::ptr::null_mut());
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        close(pipe_fds[1]);

        let result = function(param1, param2, param3, param4);

        fflush(std::ptr::null_mut());
        assert_eq!(dup2(saved_stdout, 1), 1);
        close(saved_stdout);

        let mut output = Vec::new();
        loop {
            let mut buffer = [0_u8; 256];
            let count = read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        close(pipe_fds[0]);
        (result, output)
    }
}

fn mathop_output_record(output: &[u8]) -> &[u8] {
    const PREFIX: &[u8] = b"Computation performed at timestamp: ";
    let start = output
        .windows(PREFIX.len())
        .position(|window| window == PREFIX)
        .unwrap_or_else(|| panic!("missing mathop output in {output:?}"));
    let mut newlines = 0;
    for (offset, byte) in output[start..].iter().enumerate() {
        if *byte == b'\n' {
            newlines += 1;
            if newlines == 4 {
                return &output[start..=start + offset];
            }
        }
    }
    panic!("incomplete mathop output in {output:?}");
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

    fn bounded_i32(&mut self, magnitude: i32) -> i32 {
        (self.next_u32() % (2 * magnitude as u32 + 1)) as i32 - magnitude
    }

    fn nonzero_i32(&mut self, magnitude: i32) -> i32 {
        loop {
            let value = self.bounded_i32(magnitude);
            if value != 0 {
                return value;
            }
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let direct = root.join(profile).join("libmathop_lib.so");
    if direct.exists() {
        return direct;
    }

    let deps = root.join(profile).join("deps");
    let mut candidates: Vec<_> = std::fs::read_dir(&deps)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", deps.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libmathop_lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no Rust cdylib found under {}", root.display()))
}

fn load_pair() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

unsafe fn allocation_bytes(pointer: *const ComputationResult, count: usize) -> &'static [u8] {
    unsafe {
        std::slice::from_raw_parts(
            pointer.cast::<u8>(),
            count * std::mem::size_of::<ComputationResult>(),
        )
    }
}

unsafe fn release(pointer: *mut ComputationResult) {
    unsafe {
        free(pointer.cast());
    }
}

#[test]
fn valid_low_level_configurations_match() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x5eed_fade_cafe_beef);

    unsafe {
        for value in b'1'..=b'5' {
            assert_eq!(
                (c.is_valid_operation)(value as c_char),
                (rust.is_valid_operation)(value as c_char)
            );
            assert!((c.is_valid_operation)(value as c_char));
        }

        let priority_values = [
            0,
            OP_ADD,
            OP_MULTIPLY,
            OP_SUBTRACT,
            OP_DIVIDE,
            OP_MODULO,
            6,
            c_uint::MAX,
        ];
        for op in priority_values
            .into_iter()
            .chain((0..128).map(|_| rng.next_u32()))
        {
            assert_eq!(
                (c.get_operation_priority)(op),
                (rust.get_operation_priority)(op),
                "priority mismatch for raw operation {op}"
            );
        }

        for _ in 0..256 {
            let a = rng.bounded_i32(30_000);
            let b = rng.bounded_i32(30_000);
            let ignored = rng.next_u32() as i32;
            assert_eq!(
                (c.add_operation)(a, b, ignored),
                (rust.add_operation)(a, b, ignored)
            );
            assert_eq!(
                (c.multiply_operation)(a, b, ignored),
                (rust.multiply_operation)(a, b, ignored)
            );
            assert_eq!(
                (c.subtract_operation)(a, b, ignored),
                (rust.subtract_operation)(a, b, ignored)
            );

            let divisor = rng.nonzero_i32(30_000);
            assert_eq!(
                (c.divide_operation)(a, divisor, ignored),
                (rust.divide_operation)(a, divisor, ignored)
            );
            assert_eq!(
                (c.modulo_operation)(a, divisor, ignored),
                (rust.modulo_operation)(a, divisor, ignored)
            );
        }

        for (a, b) in [(i32::MAX, -1), (i32::MIN, 1), (i32::MAX, 0), (i32::MIN, 0)] {
            assert_eq!((c.add_operation)(a, b, 7), (rust.add_operation)(a, b, 7));
            assert_eq!(
                (c.subtract_operation)(a, b, 7),
                (rust.subtract_operation)(a, b, 7)
            );
        }
        for (a, b) in [(i32::MAX, 1), (i32::MIN, 1), (i32::MAX, 0)] {
            assert_eq!(
                (c.multiply_operation)(a, b, -9),
                (rust.multiply_operation)(a, b, -9)
            );
        }

        for op in [1, 2, 3, 4, 5, 0, 6, c_uint::MAX] {
            let c_selected = (c.select_operation)(op);
            let rust_selected = (rust.select_operation)(op);
            for _ in 0..64 {
                let a = rng.bounded_i32(20_000);
                let b = if matches!(op, OP_DIVIDE | OP_MODULO) {
                    rng.nonzero_i32(20_000)
                } else {
                    rng.bounded_i32(20_000)
                };
                assert_eq!(
                    c_selected(a, b, 123),
                    rust_selected(a, b, 123),
                    "selected operation mismatch for raw operation {op}"
                );
            }
        }

        assert_eq!(
            (c.get_computation_timestamp)(),
            (rust.get_computation_timestamp)()
        );

        for count in [0, 1, 2, 3, 8, 16, 64, -1, i32::MAX] {
            let c_pointer = (c.allocate_results)(count);
            let rust_pointer = (rust.allocate_results)(count);
            assert_eq!(
                c_pointer.is_null(),
                rust_pointer.is_null(),
                "allocation nullness mismatch for count {count}"
            );
            if !c_pointer.is_null() && (0..=64).contains(&count) {
                assert_eq!(
                    allocation_bytes(c_pointer, count as usize),
                    allocation_bytes(rust_pointer, count as usize),
                    "allocation bytes mismatch for count {count}"
                );
                assert!(
                    allocation_bytes(c_pointer, count as usize)
                        .iter()
                        .all(|byte| *byte == 0)
                );
            }
            release(c_pointer);
            release(rust_pointer);
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum HistoryShape {
    Null,
    Empty,
    Middle,
    Last,
    Saturated,
}

unsafe fn initialized_history(api: &Api) -> *mut ComputationResult {
    let pointer = unsafe { (api.allocate_results)(10) };
    assert!(!pointer.is_null());
    for index in 0..10 {
        unsafe {
            pointer.add(index).write(ComputationResult {
                value: 10_000 + index as i32,
                timestamp: -50_000 + index as c_long,
                status: -7,
            });
        }
    }
    pointer
}

#[test]
fn history_configurations_match() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x1234_9876_dead_beef);
    let shapes = [
        HistoryShape::Null,
        HistoryShape::Empty,
        HistoryShape::Middle,
        HistoryShape::Last,
        HistoryShape::Saturated,
    ];

    unsafe {
        for op in [1, 2, 3, 4, 5, 6] {
            for shape in shapes {
                for _ in 0..24 {
                    let mut c_history = if matches!(shape, HistoryShape::Null) {
                        std::ptr::null_mut()
                    } else {
                        initialized_history(&c)
                    };
                    let mut rust_history = if matches!(shape, HistoryShape::Null) {
                        std::ptr::null_mut()
                    } else {
                        initialized_history(&rust)
                    };
                    let initial_count = match shape {
                        HistoryShape::Null => 100 + rng.bounded_i32(20),
                        HistoryShape::Empty => 0,
                        HistoryShape::Middle => (rng.next_u32() % 8 + 1) as i32,
                        HistoryShape::Last => 9,
                        HistoryShape::Saturated => (rng.next_u32() % 4 + 10) as i32,
                    };
                    let mut c_count = initial_count;
                    let mut rust_count = initial_count;
                    let a = rng.bounded_i32(20_000);
                    let b = if matches!(op, OP_DIVIDE | OP_MODULO) {
                        rng.nonzero_i32(20_000)
                    } else {
                        rng.bounded_i32(20_000)
                    };

                    let c_result = (c.perform_computation_with_history)(
                        a,
                        b,
                        op,
                        &mut c_history,
                        &mut c_count,
                    );
                    let rust_result = (rust.perform_computation_with_history)(
                        a,
                        b,
                        op,
                        &mut rust_history,
                        &mut rust_count,
                    );
                    assert_eq!(c_result, rust_result, "result mismatch for {op} {shape:?}");
                    assert_eq!(c_count, rust_count, "count mismatch for {op} {shape:?}");
                    assert!(!c_history.is_null());
                    assert!(!rust_history.is_null());
                    assert_eq!(
                        allocation_bytes(c_history, 10),
                        allocation_bytes(rust_history, 10),
                        "history bytes mismatch for {op} {shape:?}"
                    );

                    release(c_history);
                    release(rust_history);
                }
            }
        }
    }
}

fn first_selector_parameter(op: c_uint, variant: i32) -> i32 {
    if (1..=5).contains(&op) {
        (op as i32 - 1) + 5 * variant
    } else {
        -2 - 5 * variant
    }
}

fn second_selector_parameter(op: c_uint, variant: i32) -> i32 {
    let base = match op {
        OP_ADD => 4,
        OP_MULTIPLY => 5,
        OP_SUBTRACT => 1,
        OP_DIVIDE => 2,
        OP_MODULO => 3,
        _ => -3,
    };
    if op <= OP_MODULO {
        base + 5 * variant
    } else {
        base - 5 * variant
    }
}

fn validation_parameter(valid: bool, variant: u32) -> i32 {
    let residues = if valid {
        [b'1', b'2', b'3', b'4', b'5']
    } else {
        [0, 1, b'0', b'6', 127]
    };
    residues[variant as usize % residues.len()] as i32 + 128 * (variant % 5) as i32
}

#[test]
fn mathop_configuration_cross_product_matches() {
    let mut rng = Rng::new(0xa11c_e55e_1357_2468);

    for valid in [true, false] {
        for first_op in [1, 2, 3, 4, 5, 6] {
            for second_op in [1, 2, 3, 4, 5, 6] {
                let (c, rust) = load_pair();
                for trial in 0..16 {
                    let variant = (rng.next_u32() % 5) as i32;
                    let param1 = validation_parameter(valid, rng.next_u32());
                    let param2 = if matches!(first_op, OP_DIVIDE | OP_MODULO) {
                        rng.nonzero_i32(500)
                    } else {
                        rng.bounded_i32(500)
                    };
                    let param3 = first_selector_parameter(first_op, variant);
                    let param4 = second_selector_parameter(second_op, variant);

                    let (c_result, c_output) =
                        call_mathop_captured(c.mathop, param1, param2, param3, param4);
                    let (rust_result, rust_output) =
                        call_mathop_captured(rust.mathop, param1, param2, param3, param4);
                    assert_eq!(
                        c_result, rust_result,
                        "mathop mismatch: valid={valid}, first={first_op}, \
                         second={second_op}, trial={trial}, \
                         args=({param1}, {param2}, {param3}, {param4})"
                    );
                    assert_eq!(
                        mathop_output_record(&c_output),
                        mathop_output_record(&rust_output),
                        "mathop stdout mismatch: valid={valid}, first={first_op}, \
                         second={second_op}, trial={trial}"
                    );
                }
            }
        }
    }
}

#[test]
fn explicit_error_and_boundary_results_match() {
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0xbad5_eed0_1020_3040);

    unsafe {
        for raw in i8::MIN..=i8::MAX {
            let expected_valid = (b'1' as i8..=b'5' as i8).contains(&raw);
            let c_result = (c.is_valid_operation)(raw as c_char);
            let rust_result = (rust.is_valid_operation)(raw as c_char);
            assert_eq!(c_result, rust_result, "validation mismatch for {raw}");
            assert_eq!(c_result, expected_valid, "unexpected C result for {raw}");
        }

        for _ in 0..256 {
            let a = rng.next_u32() as i32;
            let ignored = rng.next_u32() as i32;
            assert_eq!(
                (c.divide_operation)(a, 0, ignored),
                (rust.divide_operation)(a, 0, ignored)
            );
            assert_eq!((c.divide_operation)(a, 0, ignored), 0);
            assert_eq!(
                (c.modulo_operation)(a, 0, ignored),
                (rust.modulo_operation)(a, 0, ignored)
            );
            assert_eq!((c.modulo_operation)(a, 0, ignored), 0);
        }

        let c_zero = (c.allocate_results)(0);
        let rust_zero = (rust.allocate_results)(0);
        assert_eq!(c_zero.is_null(), rust_zero.is_null());
        release(c_zero);
        release(rust_zero);

        let c_oversized = (c.allocate_results)(-1);
        let rust_oversized = (rust.allocate_results)(-1);
        assert!(c_oversized.is_null());
        assert_eq!(c_oversized.is_null(), rust_oversized.is_null());
        release(c_oversized);
        release(rust_oversized);

        for op in [0, 6, c_uint::MAX] {
            assert_eq!(
                (c.get_operation_priority)(op),
                (rust.get_operation_priority)(op)
            );
            let c_selected = (c.select_operation)(op);
            let rust_selected = (rust.select_operation)(op);
            assert_eq!(c_selected(91, -17, 0), rust_selected(91, -17, 0));

            let mut c_history = std::ptr::null_mut();
            let mut rust_history = std::ptr::null_mut();
            let mut c_count = 77;
            let mut rust_count = 77;
            assert_eq!(
                (c.perform_computation_with_history)(91, -17, op, &mut c_history, &mut c_count),
                (rust.perform_computation_with_history)(
                    91,
                    -17,
                    op,
                    &mut rust_history,
                    &mut rust_count
                )
            );
            assert_eq!(c_count, rust_count);
            assert_eq!(
                allocation_bytes(c_history, 10),
                allocation_bytes(rust_history, 10)
            );
            release(c_history);
            release(rust_history);
        }
    }
}

fn run_crash_probe(library: &Path, probe: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("crash_probe")
        .arg("--nocapture")
        .env("DIFFERENTIAL_CRASH_LIBRARY", library)
        .env("DIFFERENTIAL_CRASH_PROBE", probe)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
}

#[cfg(unix)]
#[test]
fn null_pointer_termination_matches() {
    use std::os::unix::process::ExitStatusExt;

    for probe in ["null_history", "null_count"] {
        let c_status = run_crash_probe(&c_library_path(), probe);
        let rust_status = run_crash_probe(&rust_library_path(), probe);
        assert!(!c_status.success(), "C unexpectedly survived {probe}");
        assert!(!rust_status.success(), "Rust unexpectedly survived {probe}");
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "termination signal mismatch for {probe}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

#[test]
fn crash_probe() {
    let Some(library) = std::env::var_os("DIFFERENTIAL_CRASH_LIBRARY") else {
        return;
    };
    let probe = std::env::var("DIFFERENTIAL_CRASH_PROBE").unwrap();
    let api = unsafe { Api::load(Path::new(&library)) };

    unsafe {
        match probe.as_str() {
            "null_history" => {
                let mut count = 0;
                (api.perform_computation_with_history)(
                    1,
                    2,
                    OP_ADD,
                    std::ptr::null_mut(),
                    &mut count,
                );
            }
            "null_count" => {
                let mut entry = ComputationResult {
                    value: 0,
                    timestamp: 0,
                    status: 0,
                };
                let mut history = &mut entry as *mut ComputationResult;
                (api.perform_computation_with_history)(
                    1,
                    2,
                    OP_ADD,
                    &mut history,
                    std::ptr::null_mut(),
                );
            }
            _ => panic!("unknown crash probe {probe}"),
        }
    }
}
