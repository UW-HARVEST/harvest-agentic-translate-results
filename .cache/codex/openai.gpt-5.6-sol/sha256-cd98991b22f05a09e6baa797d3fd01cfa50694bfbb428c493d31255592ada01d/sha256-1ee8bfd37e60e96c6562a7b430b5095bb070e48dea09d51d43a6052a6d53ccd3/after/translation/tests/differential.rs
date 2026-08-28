use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_double, c_int};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type SafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type ProcessArrayReverse = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type SwitchCalculator = unsafe extern "C" fn(c_int, c_int) -> c_int;
type AllocateAndCompute = unsafe extern "C" fn(c_int, c_double) -> c_int;
type ForeachSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type Fallcalc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        Self {
            // SAFETY: Both paths name libraries built from this workspace.
            c: unsafe { Library::new(&c_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display())),
            // SAFETY: Both paths name libraries built from this workspace.
            rust: unsafe { Library::new(&rust_path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display())),
        }
    }
}

macro_rules! assert_same {
    ($libraries:expr, $function_type:ty, $symbol:literal $(, $argument:expr)*) => {{
        // SAFETY: The symbol table and function signatures come from the C API.
        let c_function: Symbol<$function_type> =
            unsafe { $libraries.c.get(concat!($symbol, "\0").as_bytes()) }.unwrap();
        // SAFETY: The Rust cdylib must expose the identical C ABI.
        let rust_function: Symbol<$function_type> =
            unsafe { $libraries.rust.get(concat!($symbol, "\0").as_bytes()) }.unwrap();
        let c_result = unsafe { c_function($($argument),*) };
        let rust_result = unsafe { rust_function($($argument),*) };
        assert_eq!(
            rust_result, c_result,
            "{} returned different values",
            $symbol
        );
        c_result
    }};
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 16) as i32
    }

    fn below(&mut self, limit: u32) -> u32 {
        (self.next_u64() % u64::from(limit)) as u32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("libharvest-work-"))
        })
        .unwrap_or_else(|| panic!("no reference shared library in {}", build_dir.display()))
}

fn rust_library_path() -> PathBuf {
    let test_executable = env::current_exe().expect("test executable path");
    let profile_dir = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("Cargo profile directory");
    profile_dir.join(format!(
        "{}fallcalc_lib{}",
        env::consts::DLL_PREFIX,
        env::consts::DLL_SUFFIX
    ))
}

#[test]
fn safe_double_to_int_configurations_match() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x45d9_f3b1_3311_73ab);

    for _ in 0..256 {
        let payload = (rng.next_u64() & 0x000f_ffff_ffff_ffff).max(1);
        let sign = rng.next_u64() & (1_u64 << 63);
        let nan = f64::from_bits(sign | 0x7ff0_0000_0000_0000 | payload);
        assert_same!(libraries, SafeDoubleToInt, "safe_double_to_int", nan);
    }

    for value in [f64::INFINITY, f64::NEG_INFINITY] {
        assert_same!(libraries, SafeDoubleToInt, "safe_double_to_int", value);
    }

    for _ in 0..256 {
        let above = i32::MAX as f64 + f64::from(rng.below(1_000_000));
        let below = i32::MIN as f64 - f64::from(rng.below(1_000_000));
        assert_same!(libraries, SafeDoubleToInt, "safe_double_to_int", above);
        assert_same!(libraries, SafeDoubleToInt, "safe_double_to_int", below);

        let integral = rng.next_i32() as f64;
        assert_same!(libraries, SafeDoubleToInt, "safe_double_to_int", integral);

        let base = rng.next_i32().clamp(i32::MIN + 2, i32::MAX - 2) as f64;
        let fraction = if rng.next_u64() & 1 == 0 { 0.25 } else { -0.75 };
        assert_same!(
            libraries,
            SafeDoubleToInt,
            "safe_double_to_int",
            base + fraction
        );
    }
}

#[test]
fn process_array_reverse_configurations_match() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa076_1d64_78bd_642f);

    assert_eq!(
        assert_same!(
            libraries,
            ProcessArrayReverse,
            "process_array_reverse",
            std::ptr::null_mut(),
            -1
        ),
        0
    );
    assert_eq!(
        assert_same!(
            libraries,
            ProcessArrayReverse,
            "process_array_reverse",
            std::ptr::null_mut(),
            0
        ),
        0
    );

    for _ in 0..256 {
        let len = 1 + rng.below(128) as usize;
        let mut values: Vec<i32> = (0..len)
            .map(|_| rng.next_i32().rem_euclid(2_000_001) - 1_000_000)
            .collect();
        let end = unsafe { values.as_mut_ptr().add(len - 1) };
        assert_same!(
            libraries,
            ProcessArrayReverse,
            "process_array_reverse",
            end,
            len as i32
        );
    }
}

#[test]
fn switch_configurations_match() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xe703_7ed1_a0b4_28db);

    for operation in [0, 1, 2, 3, 4, -1, 5, i32::MIN, i32::MAX] {
        for _ in 0..512 {
            let value = rng.next_i32();
            assert_same!(
                libraries,
                SwitchCalculator,
                "switch_fallthrough_calculator",
                value,
                operation
            );
        }
    }
}

#[test]
fn allocate_and_compute_configurations_match() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x8ebc_6af0_9c88_c6e3);

    for size in [-1, -2, i32::MIN] {
        assert_eq!(
            assert_same!(
                libraries,
                AllocateAndCompute,
                "allocate_and_compute",
                size,
                1.5
            ),
            -1
        );
    }

    for size in [0, 1] {
        for multiplier in [0.0, 1.5, -2.25, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_same!(
                libraries,
                AllocateAndCompute,
                "allocate_and_compute",
                size,
                multiplier
            );
        }
    }

    for _ in 0..256 {
        let size = 2 + rng.below(63) as i32;
        let positive = 0.001 + f64::from(rng.below(1_000_000)) / 100.0;
        for multiplier in [
            0.0,
            positive,
            -positive,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            1.0e15,
            -1.0e15,
        ] {
            assert_same!(
                libraries,
                AllocateAndCompute,
                "allocate_and_compute",
                size,
                multiplier
            );
        }
    }
}

#[test]
fn foreach_sum_configurations_match() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x5899_65cc_7537_4cc3);

    for count in [-1, i32::MIN, 0] {
        assert_eq!(
            assert_same!(
                libraries,
                ForeachSum,
                "foreach_sum",
                std::ptr::null_mut(),
                count
            ),
            0
        );
    }

    for _ in 0..256 {
        let len = 1 + rng.below(128) as usize;
        let mut values: Vec<i32> = (0..len)
            .map(|_| rng.next_i32().rem_euclid(2_000_001) - 1_000_000)
            .collect();
        assert_same!(
            libraries,
            ForeachSum,
            "foreach_sum",
            values.as_mut_ptr(),
            len as i32
        );
    }
}

fn param3_for_class(class: usize, rng: &mut Rng) -> i32 {
    match class {
        0 => -(5 * (rng.below(20_000) as i32 + 1) + rng.below(4) as i32 + 1),
        1 => 5 * (rng.below(1_026) as i32 - 1_000),
        2..=5 => {
            let operation = class as i32 - 1;
            let max_quotient = (128 - operation) / 5;
            5 * rng.below((max_quotient + 1) as u32) as i32 + operation
        }
        6..=10 => {
            let operation = class as i32 - 6;
            let min_quotient = (129 - operation + 4) / 5;
            5 * (min_quotient + rng.below(20_000) as i32) + operation
        }
        _ => unreachable!(),
    }
}

fn param4_for_class(class: usize, rng: &mut Rng) -> i32 {
    let quotient = rng.below(20_000) as i32;
    match class {
        0 => -(10 * quotient + rng.below(8) as i32 + 2),
        1 => -(10 * quotient + 1),
        2 => {
            let sign = if rng.next_u64() & 1 == 0 { 1 } else { -1 };
            sign * 10 * quotient
        }
        3 => 10 * quotient + rng.below(9) as i32 + 1,
        _ => unreachable!(),
    }
}

#[test]
fn fallcalc_configuration_cross_product_matches() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xd1b5_4a32_d192_ed03);

    for param3_class in 0..11 {
        for param4_class in 0..4 {
            for _ in 0..128 {
                let param1 = rng.next_i32();
                let param2 = rng.next_i32();
                let param3 = param3_for_class(param3_class, &mut rng);
                let param4 = param4_for_class(param4_class, &mut rng);
                assert_same!(
                    libraries, Fallcalc, "fallcalc", param1, param2, param3, param4
                );
            }
        }
    }
}

#[test]
fn explicit_error_surface_matches() {
    let libraries = Libraries::load();

    for (input, expected) in [
        (f64::NAN, 0),
        (f64::INFINITY, i32::MAX),
        (f64::NEG_INFINITY, i32::MIN),
        (i32::MAX as f64, i32::MAX),
        (i32::MAX as f64 + 1.0, i32::MAX),
        (i32::MIN as f64, i32::MIN),
        (i32::MIN as f64 - 1.0, i32::MIN),
    ] {
        assert_eq!(
            assert_same!(libraries, SafeDoubleToInt, "safe_double_to_int", input),
            expected
        );
    }

    assert_eq!(
        assert_same!(
            libraries,
            AllocateAndCompute,
            "allocate_and_compute",
            -1,
            1.0
        ),
        -1
    );
}

#[test]
fn generic_ffi_boundaries_match() {
    let libraries = Libraries::load();
    let mut values = vec![1_i32; 4_096];

    for count in [i32::MIN, -1, 0] {
        assert_same!(
            libraries,
            ForeachSum,
            "foreach_sum",
            std::ptr::null_mut(),
            count
        );
        assert_same!(
            libraries,
            ProcessArrayReverse,
            "process_array_reverse",
            std::ptr::null_mut(),
            count
        );
    }

    assert_same!(
        libraries,
        ForeachSum,
        "foreach_sum",
        values.as_mut_ptr(),
        values.len() as i32
    );
    let end = unsafe { values.as_mut_ptr().add(values.len() - 1) };
    assert_same!(
        libraries,
        ProcessArrayReverse,
        "process_array_reverse",
        end,
        values.len() as i32
    );

    for mut edge_values in [
        vec![i32::MAX, 1],
        vec![i32::MIN, -1],
        vec![i32::MAX, i32::MIN, i32::MAX, i32::MIN],
    ] {
        let len = edge_values.len();
        assert_same!(
            libraries,
            ForeachSum,
            "foreach_sum",
            edge_values.as_mut_ptr(),
            len as i32
        );
        let end = unsafe { edge_values.as_mut_ptr().add(len - 1) };
        assert_same!(
            libraries,
            ProcessArrayReverse,
            "process_array_reverse",
            end,
            len as i32
        );
    }

    for invalid_operation in [-1, 5, i32::MIN, i32::MAX] {
        assert_eq!(
            assert_same!(
                libraries,
                SwitchCalculator,
                "switch_fallthrough_calculator",
                123,
                invalid_operation
            ),
            0
        );
    }

    for parameters in [
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (0, 0, 128, -9),
        (0, 0, 129, -10),
        (-1, 1, -1, -11),
    ] {
        assert_same!(
            libraries,
            Fallcalc,
            "fallcalc",
            parameters.0,
            parameters.1,
            parameters.2,
            parameters.3
        );
    }
}

fn allocator_interposer_path() -> PathBuf {
    manifest_dir()
        .join("target")
        .join("malloc-fault")
        .join("libmalloc_fail.so")
}

fn compile_allocator_interposer(output: &Path) {
    fs::create_dir_all(output.parent().unwrap()).expect("create interposer output directory");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(manifest_dir().join("tests/malloc_fail.c"))
        .args(["-ldl", "-o"])
        .arg(output)
        .status()
        .expect("run C compiler for allocator interposer");
    assert!(status.success(), "allocator interposer compilation failed");
}

fn run_allocator_failure_child(interposer_path: &Path) {
    let libraries = Libraries::load();
    // SAFETY: This is the helper built from tests/malloc_fail.c and preloaded
    // into this process.
    let interposer = unsafe { Library::new(interposer_path) }.expect("load allocator interposer");

    type ArmFailure = unsafe extern "C" fn(usize);
    let arm: Symbol<ArmFailure> =
        unsafe { interposer.get(b"fail_next_allocation_of_size\0") }.unwrap();
    let c_allocate: Symbol<AllocateAndCompute> =
        unsafe { libraries.c.get(b"allocate_and_compute\0") }.unwrap();
    let rust_allocate: Symbol<AllocateAndCompute> =
        unsafe { libraries.rust.get(b"allocate_and_compute\0") }.unwrap();
    let c_fallcalc: Symbol<Fallcalc> = unsafe { libraries.c.get(b"fallcalc\0") }.unwrap();
    let rust_fallcalc: Symbol<Fallcalc> = unsafe { libraries.rust.get(b"fallcalc\0") }.unwrap();

    unsafe { arm(3 * 16) };
    let c_allocate_result = unsafe { c_allocate(3, 1.5) };
    unsafe { arm(3 * 16) };
    let rust_allocate_result = unsafe { rust_allocate(3, 1.5) };
    assert_eq!(c_allocate_result, -1);
    assert_eq!(rust_allocate_result, c_allocate_result);

    unsafe { arm(5 * size_of::<i32>()) };
    let c_fallcalc_result = unsafe { c_fallcalc(1, 2, 3, 4) };
    unsafe { arm(5 * size_of::<i32>()) };
    let rust_fallcalc_result = unsafe { rust_fallcalc(1, 2, 3, 4) };
    assert_eq!(c_fallcalc_result, -1);
    assert_eq!(rust_fallcalc_result, c_fallcalc_result);
}

#[test]
fn malloc_failure_paths_match() {
    let interposer_path = allocator_interposer_path();

    if env::var_os("FALLCALC_FAULT_CHILD").is_some() {
        run_allocator_failure_child(&interposer_path);
        return;
    }

    compile_allocator_interposer(&interposer_path);
    let output = Command::new(env::current_exe().expect("test executable path"))
        .args(["--exact", "malloc_failure_paths_match", "--nocapture"])
        .env("FALLCALC_FAULT_CHILD", "1")
        .env("LD_PRELOAD", &interposer_path)
        .output()
        .expect("run allocator-failure child");
    assert!(
        output.status.success(),
        "allocator-failure child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
