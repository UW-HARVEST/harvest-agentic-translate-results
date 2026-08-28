use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

type Operation = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;
type Gotomach = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const u8, flags: c_int, ...) -> c_int;
}

struct SilencedStdout {
    saved_fd: c_int,
}

impl SilencedStdout {
    fn new() -> Self {
        unsafe {
            fflush(std::ptr::null_mut());
            let saved_fd = dup(1);
            assert!(saved_fd >= 0);
            let null_fd = open(c"/dev/null".as_ptr().cast(), 1);
            assert!(null_fd >= 0);
            assert_eq!(dup2(null_fd, 1), 1);
            close(null_fd);
            Self { saved_fd }
        }
    }
}

impl Drop for SilencedStdout {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            assert_eq!(dup2(self.saved_fd, 1), 1);
            close(self.saved_fd);
        }
    }
}

struct Api {
    _library: Library,
    process_value: Operation,
    double_value: Operation,
    triple_value: Operation,
    gotomach: Gotomach,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let process_value = unsafe { *library.get(b"process_value\0").unwrap() };
        let double_value = unsafe { *library.get(b"double_value\0").unwrap() };
        let triple_value = unsafe { *library.get(b"triple_value\0").unwrap() };
        let gotomach = unsafe { *library.get(b"gotomach\0").unwrap() };
        Self {
            _library: library,
            process_value,
            double_value,
            triple_value,
            gotomach,
        }
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(row: u64) -> Self {
        Self(0x9e37_79b9_7f4a_7c15 ^ row.wrapping_mul(0xd1b5_4a32_d192_ed03))
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn int(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    fn range(&mut self, low: i32, high: i32) -> i32 {
        assert!(low <= high);
        let width = (high as i64 - low as i64 + 1) as u64;
        (low as i64 + (self.next_u64() % width) as i64) as i32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("../c_src/build")
        .join("libharvest-work-wommeA.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir()
        .join("target")
        .join("release")
        .join("libgotomach_lib.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn compare_operation(
    row: usize,
    c_function: Operation,
    rust_function: Operation,
    value: i32,
    unused: i32,
    context: *mut c_void,
) {
    let c_result = unsafe { c_function(value, unused, context) };
    let rust_result = unsafe { rust_function(value, unused, context) };
    assert_eq!(
        rust_result, c_result,
        "CONFIGS.md row {row}: value={value}, unused={unused}, context={context:p}"
    );
}

fn compare_gotomach(
    row: usize,
    c_function: Gotomach,
    rust_function: Gotomach,
    iterations: i32,
    seed: i32,
    mode: i32,
    threshold: i32,
) {
    let c_result = unsafe { c_function(iterations, seed, mode, threshold) };
    let rust_result = unsafe { rust_function(iterations, seed, mode, threshold) };
    assert_eq!(
        rust_result, c_result,
        "CONFIGS.md row {row}: iterations={iterations}, seed={seed}, \
         mode={mode}, threshold={threshold}"
    );
}

fn invalid_mode(rng: &mut Rng) -> i32 {
    loop {
        let mode = rng.int();
        if !(0..=2).contains(&mode) {
            return mode;
        }
    }
}

fn mode_for(class: i32, rng: &mut Rng) -> i32 {
    if class == 3 { invalid_mode(rng) } else { class }
}

fn generated_values(iterations: i32, seed: i32, mode: i32) -> Vec<i32> {
    let mut values = Vec::with_capacity(iterations as usize);
    let mut current = seed;
    for _ in 0..iterations {
        current = match mode {
            1 => current * 2,
            2 => current * 3,
            _ => current + 10,
        };
        values.push(current);
        current %= 1000;
    }
    values
}

#[test]
fn configuration_surface_rows_1_through_35() {
    let _stdout_lock = STDOUT_LOCK.lock().unwrap();
    let _silenced_stdout = SilencedStdout::new();
    let (c, rust) = load_apis();
    let mut context_byte = 0_u8;
    let non_null_context = (&mut context_byte as *mut u8).cast::<c_void>();

    // CONFIGS.md rows 1-3: all low-level public entry points.
    let operation_rows = [
        (
            1,
            c.process_value,
            rust.process_value,
            i32::MIN,
            i32::MAX - 10,
        ),
        (
            2,
            c.double_value,
            rust.double_value,
            i32::MIN / 2,
            i32::MAX / 2,
        ),
        (
            3,
            c.triple_value,
            rust.triple_value,
            i32::MIN / 3,
            i32::MAX / 3,
        ),
    ];
    for (row, c_function, rust_function, low, high) in operation_rows {
        let mut rng = Rng::new(row);
        for (index, value) in [low, -1, 0, 1, high]
            .into_iter()
            .chain((0..128).map(|_| rng.range(low, high)))
            .enumerate()
        {
            let unused = match index % 3 {
                0 => i32::MIN,
                1 => 0,
                _ => i32::MAX,
            };
            let context = if index % 2 == 0 {
                std::ptr::null_mut()
            } else {
                non_null_context
            };
            compare_operation(
                row as usize,
                c_function,
                rust_function,
                value,
                unused,
                context,
            );
        }
    }

    // CONFIGS.md rows 4-7: zero iterations for each mode class.
    for class in 0..=3 {
        let row = 4 + class as usize;
        let mut rng = Rng::new(row as u64);
        let seeds = [0, 1, u16::MAX as i32];
        for index in 0..67 {
            let seed = if index < seeds.len() {
                seeds[index]
            } else {
                rng.range(0, u16::MAX as i32)
            };
            let mode = mode_for(class, &mut rng);
            compare_gotomach(row, c.gotomach, rust.gotomach, 0, seed, mode, rng.int());
        }
    }

    // CONFIGS.md rows 8-15: one iteration, rejected then accepted.
    for class in 0..=3 {
        for accepted in [false, true] {
            let row = 8 + class as usize * 2 + usize::from(accepted);
            let mut rng = Rng::new(row as u64);
            for index in 0..64 {
                let seed = rng.range(0, u16::MAX as i32);
                let mode = mode_for(class, &mut rng);
                let generated = generated_values(1, seed, mode)[0];
                let threshold = if accepted {
                    generated + 1
                } else if index % 2 == 0 {
                    generated
                } else {
                    generated - rng.range(0, 1000)
                };
                compare_gotomach(row, c.gotomach, rust.gotomach, 1, seed, mode, threshold);
            }
        }
    }

    // CONFIGS.md rows 16-27: many iterations with none/mixed/all accepted.
    for class in 0..=3 {
        for threshold_class in 0..3 {
            let row = 16 + class as usize * 3 + threshold_class;
            let mut rng = Rng::new(row as u64);
            for _ in 0..48 {
                let iterations = rng.range(3, 96);
                let (seed, mode, values) = loop {
                    let seed = rng.range(0, u16::MAX as i32);
                    let mode = mode_for(class, &mut rng);
                    let values = generated_values(iterations, seed, mode);
                    if threshold_class != 1
                        || values.iter().min().unwrap() != values.iter().max().unwrap()
                    {
                        break (seed, mode, values);
                    }
                };
                let threshold = match threshold_class {
                    0 => i32::MIN,
                    1 => values.iter().min().unwrap() + 1,
                    _ => i32::MAX,
                };
                compare_gotomach(
                    row,
                    c.gotomach,
                    rust.gotomach,
                    iterations,
                    seed,
                    mode,
                    threshold,
                );
            }
        }
    }

    // CONFIGS.md rows 28-35: maximum count, without and with saturation.
    for class in 0..=3 {
        for all_accepted in [false, true] {
            let row = 28 + class as usize * 2 + usize::from(all_accepted);
            let mut rng = Rng::new(row as u64);
            for index in 0..12 {
                let seed = if index == 0 {
                    0
                } else if index == 1 {
                    u16::MAX as i32
                } else {
                    rng.range(0, u16::MAX as i32)
                };
                let mode = mode_for(class, &mut rng);
                let threshold = if all_accepted { i32::MAX } else { i32::MIN };
                compare_gotomach(
                    row,
                    c.gotomach,
                    rust.gotomach,
                    u16::MAX as i32,
                    seed,
                    mode,
                    threshold,
                );
            }
        }
    }
}

fn compile_fault_interposer() -> PathBuf {
    let output = manifest_dir()
        .join("target")
        .join("fault-injection")
        .join("libfault_alloc.so");
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&output)
        .arg(manifest_dir().join("tests/fault_alloc.c"))
        .status()
        .expect("failed to run cc for allocation fault interposer");
    assert!(status.success(), "failed to compile fault interposer");
    output
}

fn fault_result(target: &str, mode: i32, nth: usize) -> i32 {
    let interposer = compile_fault_interposer();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "fault_child_probe", "--nocapture"])
        .env("LD_PRELOAD", &interposer)
        .env("FAULT_CHILD", "1")
        .env("FAULT_TARGET", target)
        .env("FAULT_MODE", mode.to_string())
        .env("FAULT_NTH", nth.to_string())
        .output()
        .expect("failed to launch allocation fault child");
    assert!(
        output.status.success(),
        "fault child failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("FAULT_RESULT="))
        .unwrap_or_else(|| panic!("fault child emitted no result:\n{stdout}"))
        .parse()
        .unwrap()
}

fn compare_fault(row: usize, mode: i32, nth: usize, expected: i32) {
    let c_result = fault_result("c", mode, nth);
    let rust_result = fault_result("rust", mode, nth);
    assert_eq!(c_result, expected, "ERRORS.md row {row}: C result");
    assert_eq!(rust_result, c_result, "ERRORS.md row {row}: Rust result");
}

#[test]
fn error_surface_rows_1_through_10_and_generic_boundaries() {
    if std::env::var_os("FAULT_CHILD").is_some() {
        return;
    }

    let _stdout_lock = STDOUT_LOCK.lock().unwrap();
    let _silenced_stdout = SilencedStdout::new();
    let (c, rust) = load_apis();
    let compare_error = |row, iterations, seed, mode, threshold, expected| {
        let c_result = unsafe { (c.gotomach)(iterations, seed, mode, threshold) };
        let rust_result = unsafe { (rust.gotomach)(iterations, seed, mode, threshold) };
        assert_eq!(c_result, expected, "ERRORS.md row {row}: C result");
        assert_eq!(rust_result, c_result, "ERRORS.md row {row}: Rust result");
    };

    // Exact lower/upper failures plus generic one-step and integer boundaries.
    for iterations in [-1, i32::MIN] {
        compare_error(3, iterations, 0, 0, 0, -1);
    }
    for iterations in [u16::MAX as i32 + 1, i32::MAX] {
        compare_error(4, iterations, 0, 0, 0, -1);
    }
    for seed in [-1, i32::MIN] {
        compare_error(5, 1, seed, 0, 0, -2);
    }
    for seed in [u16::MAX as i32 + 1, i32::MAX] {
        compare_error(6, 1, seed, 0, 0, -2);
    }

    // Rows 1, 2, 7, and 8 force each allocation site to return NULL.
    compare_fault(1, 1, 1, -3);
    compare_fault(2, 1, 2, -3);
    compare_fault(7, 1, 1, -3);
    compare_fault(8, 1, 3, -4);

    // Rows 9 and 10 corrupt only the private state condition under test.
    compare_fault(9, 2, 0, -5);
    compare_fault(10, 3, 0, -6);
}

#[test]
fn fault_child_probe() {
    if std::env::var_os("FAULT_CHILD").is_none() {
        return;
    }

    type Configure = unsafe extern "C" fn(c_int, usize);
    type Disable = unsafe extern "C" fn();

    let target = std::env::var("FAULT_TARGET").unwrap();
    let path = if target == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&path) };

    // Initialize libc's stdout buffer before allocation counting starts.
    let _ = unsafe { (api.gotomach)(-1, 0, 0, 0) };

    let interposer = unsafe {
        Library::new(std::env::var_os("LD_PRELOAD").unwrap())
            .expect("failed to open preloaded fault interposer")
    };
    let configure: Configure = unsafe { *interposer.get(b"fault_configure\0").unwrap() };
    let disable: Disable = unsafe { *interposer.get(b"fault_disable\0").unwrap() };
    let mode = std::env::var("FAULT_MODE").unwrap().parse().unwrap();
    let nth = std::env::var("FAULT_NTH").unwrap().parse().unwrap();

    unsafe { configure(mode, nth) };
    let result = unsafe { (api.gotomach)(2, 246, 0, 257) };
    unsafe { disable() };
    println!("FAULT_RESULT={result}");
}
