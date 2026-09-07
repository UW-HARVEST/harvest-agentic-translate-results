use libloading::Library;
use std::collections::BTreeMap;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

type Operation = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;
type Gotomach = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type ShimArm = unsafe extern "C" fn(c_int, c_int);
type ShimDisarm = unsafe extern "C" fn();

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_process: Operation,
    rust_process: Operation,
    c_double: Operation,
    rust_double: Operation,
    c_triple: Operation,
    rust_triple: Operation,
    c_gotomach: Gotomach,
    rust_gotomach: Gotomach,
}

impl Apis {
    unsafe fn load() -> Self {
        let c_path = library_path(
            "C_SO_PATH",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-oIOgbn.so"),
        );
        let rust_path = library_path(
            "RUST_SO_PATH",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libgotomach_lib.so"),
        );

        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        let c_library = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
        let rust_library = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));

        unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
            unsafe {
                *library
                    .get::<T>(name)
                    .unwrap_or_else(|error| panic!("load symbol {:?}: {error}", name))
            }
        }

        let c_process = unsafe { symbol(&c_library, b"process_value\0") };
        let rust_process = unsafe { symbol(&rust_library, b"process_value\0") };
        let c_double = unsafe { symbol(&c_library, b"double_value\0") };
        let rust_double = unsafe { symbol(&rust_library, b"double_value\0") };
        let c_triple = unsafe { symbol(&c_library, b"triple_value\0") };
        let rust_triple = unsafe { symbol(&rust_library, b"triple_value\0") };
        let c_gotomach = unsafe { symbol(&c_library, b"gotomach\0") };
        let rust_gotomach = unsafe { symbol(&rust_library, b"gotomach\0") };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_process,
            rust_process,
            c_double,
            rust_double,
            c_triple,
            rust_triple,
            c_gotomach,
            rust_gotomach,
        }
    }
}

fn library_path(variable: &str, fallback: PathBuf) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or(fallback)
}

fn capture_stdout(call: impl FnOnce() -> c_int) -> (c_int, Vec<u8>) {
    capture_stdout_then(call, || {})
}

fn capture_stdout_then(
    call: impl FnOnce() -> c_int,
    after_call: impl FnOnce(),
) -> (c_int, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let mut pipe_fds = [-1, -1];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        let result = call();
        after_call();
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let count = read(
                pipe_fds[0],
                chunk.as_mut_ptr().cast::<c_void>(),
                chunk.len(),
            );
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0);
        (result, output)
    }
}

fn compare_operation(
    c_function: Operation,
    rust_function: Operation,
    value: c_int,
    unused: c_int,
    context: *mut c_void,
) {
    let c_result = unsafe { c_function(value, unused, context) };
    let rust_result = unsafe { rust_function(value, unused, context) };
    assert_eq!(
        rust_result, c_result,
        "operation mismatch for value={value}, unused={unused}, context={context:p}"
    );
}

fn compare_gotomach(apis: &Apis, args: (c_int, c_int, c_int, c_int)) {
    let (iterations, seed, mode, threshold) = args;
    let (c_result, c_stdout) =
        capture_stdout(|| unsafe { (apis.c_gotomach)(iterations, seed, mode, threshold) });
    let (rust_result, rust_stdout) =
        capture_stdout(|| unsafe { (apis.rust_gotomach)(iterations, seed, mode, threshold) });

    assert_eq!(rust_result, c_result, "return mismatch for args={args:?}");
    assert_eq!(
        rust_stdout,
        c_stdout,
        "stdout mismatch for args={args:?}\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_stdout),
        String::from_utf8_lossy(&rust_stdout)
    );
}

fn call_gotomach(function: Gotomach, args: (c_int, c_int, c_int, c_int)) -> (c_int, Vec<u8>) {
    let (iterations, seed, mode, threshold) = args;
    capture_stdout(|| unsafe { function(iterations, seed, mode, threshold) })
}

fn compare_gotomach_expected(apis: &Apis, args: (c_int, c_int, c_int, c_int), expected: c_int) {
    let c = call_gotomach(apis.c_gotomach, args);
    let rust = call_gotomach(apis.rust_gotomach, args);
    assert_eq!(c.0, expected, "unexpected C return for args={args:?}");
    assert_eq!(rust.0, expected, "unexpected Rust return for args={args:?}");
    assert_eq!(rust.1, c.1, "stdout mismatch for args={args:?}");
}

fn shim_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-support/libmalloc_shim.so")
}

fn build_malloc_shim() -> PathBuf {
    let output = shim_path();
    std::fs::create_dir_all(output.parent().unwrap()).expect("create test-support directory");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/malloc_shim.c");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .status()
        .expect("run C compiler for malloc shim");
    assert!(status.success(), "malloc shim compilation failed");
    assert!(output.is_file(), "malloc shim was not produced");
    output
}

fn injected_call(
    function: Gotomach,
    arm: ShimArm,
    disarm: ShimDisarm,
    mode: c_int,
    fail_at: c_int,
    args: (c_int, c_int, c_int, c_int),
) -> (c_int, Vec<u8>) {
    unsafe {
        arm(mode, fail_at);
    }
    let (iterations, seed, operation_mode, threshold) = args;
    capture_stdout_then(
        || unsafe { function(iterations, seed, operation_mode, threshold) },
        || unsafe { disarm() },
    )
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn range_i32(&mut self, minimum: i32, maximum: i32) -> i32 {
        let width = (maximum as i64 - minimum as i64 + 1) as u64;
        (minimum as i64 + (self.next_u64() % width) as i64) as i32
    }
}

fn selected_mode(mode: i32) -> i32 {
    match mode {
        1 => 1,
        2 => 2,
        _ => 0,
    }
}

fn generated_values(iterations: i32, seed: i32, mode: i32) -> Vec<i32> {
    let mut values = Vec::with_capacity(iterations as usize);
    let mut current = seed;
    for _ in 0..iterations {
        let value = match selected_mode(mode) {
            1 => current * 2,
            2 => current * 3,
            _ => current + 10,
        };
        values.push(value);
        current = value % 1000;
    }
    values
}

fn threshold_selecting_some(iterations: i32, seed: i32, mode: i32) -> Option<i32> {
    let mut values = generated_values(iterations, seed, mode);
    values.sort_unstable();
    values.dedup();
    if values.len() < 2 {
        return None;
    }
    Some(values[values.len() / 2])
}

fn invalid_mode(rng: &mut XorShift64, index: usize) -> i32 {
    const FIXED: [i32; 4] = [-1, 3, i32::MIN, i32::MAX];
    if index < FIXED.len() {
        return FIXED[index];
    }
    loop {
        let value = rng.next_i32();
        if !(0..=2).contains(&value) {
            return value;
        }
    }
}

fn mark(rows: &mut BTreeMap<&'static str, usize>, row: &'static str) {
    *rows.entry(row).or_default() += 1;
}

#[test]
fn phase_b_all_configuration_rows_match() {
    let apis = unsafe { Apis::load() };
    let mut rng = XorShift64::new(0x6d5a_56da_f00d_cafe);
    let mut rows = BTreeMap::new();

    let mut opaque = 0x5a_u8;
    for index in 0..256 {
        let context = if index % 2 == 0 {
            ptr::null_mut()
        } else {
            (&mut opaque as *mut u8).cast::<c_void>()
        };
        let unused = rng.next_i32();

        compare_operation(
            apis.c_process,
            apis.rust_process,
            rng.range_i32(i32::MIN, i32::MAX - 10),
            unused,
            context,
        );
        mark(&mut rows, "C01");

        compare_operation(
            apis.c_double,
            apis.rust_double,
            rng.range_i32(i32::MIN / 2, i32::MAX / 2),
            unused,
            context,
        );
        mark(&mut rows, "C02");

        compare_operation(
            apis.c_triple,
            apis.rust_triple,
            rng.range_i32(i32::MIN / 3, i32::MAX / 3),
            unused,
            context,
        );
        mark(&mut rows, "C03");
    }

    let modes = [0, 1, 2, 3];
    let zero_rows = ["C04", "C05", "C06", "C07"];
    for (mode_index, base_mode) in modes.into_iter().enumerate() {
        for case in 0..20 {
            let seed = match case {
                0 => 0,
                1 => u16::MAX as i32,
                _ => rng.range_i32(0, u16::MAX as i32),
            };
            let mode = if mode_index == 3 {
                invalid_mode(&mut rng, case)
            } else {
                base_mode
            };
            compare_gotomach(&apis, (0, seed, mode, rng.next_i32()));
            mark(&mut rows, zero_rows[mode_index]);
        }
    }

    let one_none_rows = ["C08", "C10", "C12", "C14"];
    let one_all_rows = ["C09", "C11", "C13", "C15"];
    for (mode_index, base_mode) in modes.into_iter().enumerate() {
        for case in 0..24 {
            let seed = match case {
                0 => 0,
                1 => u16::MAX as i32,
                _ => rng.range_i32(0, u16::MAX as i32),
            };
            let mode = if mode_index == 3 {
                invalid_mode(&mut rng, case)
            } else {
                base_mode
            };
            compare_gotomach(&apis, (1, seed, mode, i32::MIN));
            mark(&mut rows, one_none_rows[mode_index]);
            compare_gotomach(&apis, (1, seed, mode, i32::MAX));
            mark(&mut rows, one_all_rows[mode_index]);
        }
    }

    let many_rows = [
        ["C16", "C17", "C18"],
        ["C19", "C20", "C21"],
        ["C22", "C23", "C24"],
        ["C25", "C26", "C27"],
    ];
    for (mode_index, base_mode) in modes.into_iter().enumerate() {
        let mut mixed_cases = 0;
        for case in 0..32 {
            let iterations = rng.range_i32(2, 512);
            let seed = match case {
                0 => 0,
                1 => u16::MAX as i32,
                _ => rng.range_i32(0, u16::MAX as i32),
            };
            let mode = if mode_index == 3 {
                invalid_mode(&mut rng, case)
            } else {
                base_mode
            };

            compare_gotomach(&apis, (iterations, seed, mode, i32::MIN));
            mark(&mut rows, many_rows[mode_index][0]);
            compare_gotomach(&apis, (iterations, seed, mode, i32::MAX));
            mark(&mut rows, many_rows[mode_index][2]);

            if let Some(threshold) = threshold_selecting_some(iterations, seed, mode) {
                let values = generated_values(iterations, seed, mode);
                let selected = values.iter().filter(|&&value| value < threshold).count();
                assert!(selected > 0 && selected < values.len());
                compare_gotomach(&apis, (iterations, seed, mode, threshold));
                mark(&mut rows, many_rows[mode_index][1]);
                mixed_cases += 1;
            }
        }
        assert!(
            mixed_cases >= 16,
            "insufficient randomized mixed-threshold cases for mode index {mode_index}"
        );
    }

    let maximum_rows = ["C28", "C29", "C30", "C31"];
    for (mode_index, base_mode) in modes.into_iter().enumerate() {
        for case in 0..8 {
            let seed = match case {
                0 => 0,
                1 => u16::MAX as i32,
                _ => rng.range_i32(0, u16::MAX as i32),
            };
            let mode = if mode_index == 3 {
                invalid_mode(&mut rng, case)
            } else {
                base_mode
            };
            compare_gotomach(&apis, (u16::MAX as i32, seed, mode, i32::MAX));
            mark(&mut rows, maximum_rows[mode_index]);
        }
    }

    for number in 1..=31 {
        let row = format!("C{number:02}");
        assert!(
            rows.get(row.as_str()).copied().unwrap_or_default() >= 8,
            "{row} was not exercised with enough inputs; counts={rows:?}"
        );
    }
}

#[test]
fn phase_c_public_input_boundaries_match() {
    let apis = unsafe { Apis::load() };
    let iteration_errors = [-1, u16::MAX as i32 + 1, i32::MIN, i32::MAX];
    for iterations in iteration_errors {
        compare_gotomach_expected(&apis, (iterations, 0, 0, 0), -1);
    }

    let seed_errors = [-1, u16::MAX as i32 + 1, i32::MIN, i32::MAX];
    for seed in seed_errors {
        compare_gotomach_expected(&apis, (1, seed, 0, 0), -2);
    }

    for mode in [-1, 3, i32::MIN, i32::MAX] {
        compare_gotomach(&apis, (4, 7, mode, 100));
    }
}

#[test]
fn phase_c_fault_injection_parent() {
    if std::env::var_os("DIFFERENTIAL_FAULT_CHILD").is_some() {
        return;
    }

    let shim = build_malloc_shim();
    let current_exe = std::env::current_exe().expect("locate integration-test executable");
    let preload = match std::env::var_os("LD_PRELOAD") {
        Some(existing) if !existing.is_empty() => {
            let mut value = shim.as_os_str().to_os_string();
            value.push(":");
            value.push(existing);
            value
        }
        _ => shim.as_os_str().to_os_string(),
    };

    let output = Command::new(current_exe)
        .args([
            "--exact",
            "phase_c_fault_injection_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("LD_PRELOAD", preload)
        .env("DIFFERENTIAL_FAULT_CHILD", "1")
        .output()
        .expect("run fault-injection child");

    assert!(
        output.status.success(),
        "fault-injection child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn phase_c_fault_injection_child() {
    if std::env::var_os("DIFFERENTIAL_FAULT_CHILD").is_none() {
        return;
    }

    const FAIL_ALLOCATION: c_int = 1;
    const ZERO_STATUS_BEFORE_CHECK: c_int = 2;
    const ZERO_STATUS_DURING_LOOP: c_int = 3;
    const FILL_COUNT_DURING_LOOP: c_int = 4;

    let shim_library =
        unsafe { Library::new(shim_path()) }.expect("open preloaded malloc fault shim");
    let arm: ShimArm = unsafe { *shim_library.get(b"shim_arm\0").expect("load shim_arm") };
    let disarm: ShimDisarm = unsafe {
        *shim_library
            .get(b"shim_disarm\0")
            .expect("load shim_disarm")
    };
    let apis = unsafe { Apis::load() };

    // Warm stdio and lazy dynamic bindings before arming the allocation counter.
    compare_gotomach(&apis, (2, 7, 0, 100));

    let scenarios = [
        ("E03", FAIL_ALLOCATION, 1, (4, 7, 0, 100), -3),
        ("E04", FAIL_ALLOCATION, 2, (4, 7, 0, 100), -3),
        ("E05", FAIL_ALLOCATION, 3, (4, 7, 0, 100), -4),
        ("E06", ZERO_STATUS_BEFORE_CHECK, 0, (4, 7, 0, 100), -5),
        ("E07", ZERO_STATUS_DURING_LOOP, 0, (2, 7, 0, i32::MIN), -6),
        ("E08", FILL_COUNT_DURING_LOOP, 0, (2, 7, 0, i32::MIN), -6),
    ];

    for (row, mode, fail_at, args, expected) in scenarios {
        let c = injected_call(apis.c_gotomach, arm, disarm, mode, fail_at, args);
        let rust = injected_call(apis.rust_gotomach, arm, disarm, mode, fail_at, args);
        assert_eq!(c.0, expected, "{row}: unexpected C result");
        assert_eq!(rust.0, expected, "{row}: unexpected Rust result");
        assert_eq!(
            rust.1,
            c.1,
            "{row}: output mismatch\nC: {:?}\nRust: {:?}",
            String::from_utf8_lossy(&c.1),
            String::from_utf8_lossy(&rust.1)
        );
    }
}
