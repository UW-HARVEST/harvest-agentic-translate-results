use libloading::Library;
use std::ffi::{c_double, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ResultValue {
    value: c_int,
    scaled: c_double,
    rank: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ResultArray {
    data: [ResultValue; 10],
    count: c_int,
}

type Operation = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type SafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type ComputeScaledValue = unsafe extern "C" fn(c_int, c_double) -> c_int;
type CompareResults = unsafe extern "C" fn(*mut ResultArray, c_int, c_int) -> c_int;
type InitResultArray = unsafe extern "C" fn(*mut ResultArray, *mut c_int, c_int);
type ProcessWithForeach = unsafe extern "C" fn(*mut ResultArray, *const c_void) -> c_int;
type ComputeWeightedSum = unsafe extern "C" fn(*mut ResultArray) -> c_int;
type ArrayFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    add: Operation,
    multiply: Operation,
    subtract: Operation,
    modulo: Operation,
    safe_double_to_int: SafeDoubleToInt,
    compute_scaled_value: ComputeScaledValue,
    compare_results: CompareResults,
    init_result_array: InitResultArray,
    process_with_foreach: ProcessWithForeach,
    compute_weighted_sum: ComputeWeightedSum,
    arrayfunc: ArrayFunc,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        unsafe {
            Self {
                add: *library.get(b"add_operation\0").unwrap(),
                multiply: *library.get(b"multiply_operation\0").unwrap(),
                subtract: *library.get(b"subtract_operation\0").unwrap(),
                modulo: *library.get(b"modulo_operation\0").unwrap(),
                safe_double_to_int: *library.get(b"safe_double_to_int\0").unwrap(),
                compute_scaled_value: *library.get(b"compute_scaled_value\0").unwrap(),
                compare_results: *library.get(b"compare_results_in_array\0").unwrap(),
                init_result_array: *library.get(b"init_result_array\0").unwrap(),
                process_with_foreach: *library.get(b"process_with_foreach\0").unwrap(),
                compute_weighted_sum: *library.get(b"compute_weighted_sum\0").unwrap(),
                arrayfunc: *library.get(b"arrayfunc\0").unwrap(),
                _library: library,
            }
        }
    }

    fn operation(&self, kind: OperationKind) -> Operation {
        match kind {
            OperationKind::Add => self.add,
            OperationKind::Multiply => self.multiply,
            OperationKind::Subtract => self.subtract,
            OperationKind::Modulo => self.modulo,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum OperationKind {
    Add,
    Multiply,
    Subtract,
    Modulo,
}

#[derive(Clone, Copy)]
struct Lcg(u64);

impl Lcg {
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

    fn i32_inclusive(&mut self, min: i32, max: i32) -> i32 {
        let width = (i64::from(max) - i64::from(min) + 1) as u64;
        (i64::from(min) + i64::try_from(u64::from(self.next_u32()) % width).unwrap()) as i32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-giENQi.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libarrayfunc_lib.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}; run cargo build --release first",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn zeroed_array() -> ResultArray {
    unsafe { zeroed() }
}

fn seeded_array(count: i32, seed: u64, limit: i32) -> ResultArray {
    let mut rng = Lcg::new(seed);
    let mut array = zeroed_array();
    array.count = count;
    for (index, item) in array.data.iter_mut().enumerate() {
        item.value = rng.i32_inclusive(-limit, limit);
        item.scaled = f64::from(rng.i32_inclusive(-limit, limit)) / 7.0;
        item.rank = index as i32;
    }
    array
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((value as *const T).cast::<u8>(), size_of::<T>()) }
}

fn assert_arrays_equal(c_array: &ResultArray, rust_array: &ResultArray, context: &str) {
    assert_eq!(
        bytes(c_array),
        bytes(rust_array),
        "mutated ResultArray bytes differ for {context}\nC: {c_array:?}\nRust: {rust_array:?}"
    );
}

#[test]
fn operation_exports_match_randomized_inputs() {
    let (c, rust) = load_apis();
    let mut rng = Lcg::new(0x51a7_10a5_0dd5_eed);

    // CONFIGS C1-C3.
    for iteration in 0..1_000 {
        let a = rng.i32_inclusive(-30_000, 30_000);
        let b = rng.i32_inclusive(-30_000, 30_000);
        let unused1 = rng.i32_inclusive(i32::MIN, i32::MAX);
        let unused2 = rng.i32_inclusive(i32::MIN, i32::MAX);
        for kind in [
            OperationKind::Add,
            OperationKind::Multiply,
            OperationKind::Subtract,
        ] {
            let c_result = unsafe { c.operation(kind)(a, b, unused1, unused2) };
            let rust_result = unsafe { rust.operation(kind)(a, b, unused1, unused2) };
            assert_eq!(c_result, rust_result, "{kind:?}, iteration {iteration}");
        }
    }

    // CONFIGS C4-C5 and ERRORS E1.
    for iteration in 0..1_000 {
        let a = rng.i32_inclusive(i32::MIN + 1, i32::MAX);
        for b in [
            rng.i32_inclusive(1, i32::MAX),
            rng.i32_inclusive(i32::MIN, -1),
            0,
        ] {
            let c_result = unsafe { (c.modulo)(a, b, iteration, -iteration) };
            let rust_result = unsafe { (rust.modulo)(a, b, iteration, -iteration) };
            assert_eq!(c_result, rust_result, "modulo iteration {iteration}, b={b}");
            if b == 0 {
                assert_eq!(c_result, 0);
            }
        }
    }
}

#[test]
fn floating_conversion_exports_match_all_classes() {
    let (c, rust) = load_apis();
    let mut rng = Lcg::new(0xd0ab_1e5_f10a_7);
    let fixed_values = [
        f64::NEG_INFINITY,
        f64::from(i32::MIN) - 65_536.0,
        f64::from(i32::MIN),
        f64::from(i32::MIN) + 0.5,
        -1.75,
        -0.0,
        0.0,
        1.75,
        f64::from(i32::MAX) - 0.5,
        f64::from(i32::MAX),
        f64::from(i32::MAX) + 65_536.0,
        f64::INFINITY,
        f64::NAN,
    ];

    // CONFIGS C6-C9 and ERRORS E2-E4.
    for (index, value) in fixed_values.into_iter().enumerate() {
        let c_result = unsafe { (c.safe_double_to_int)(value) };
        let rust_result = unsafe { (rust.safe_double_to_int)(value) };
        assert_eq!(c_result, rust_result, "fixed conversion {index}: {value:?}");
    }
    for iteration in 0..2_000 {
        let value = f64::from(rng.i32_inclusive(i32::MIN + 1, i32::MAX)) / 1.25;
        let c_result = unsafe { (c.safe_double_to_int)(value) };
        let rust_result = unsafe { (rust.safe_double_to_int)(value) };
        assert_eq!(c_result, rust_result, "random conversion {iteration}");
    }
    for iteration in 0..500 {
        let offset = f64::from(rng.i32_inclusive(0, 1_000_000));
        let upper = f64::from(i32::MAX) + offset;
        let lower = f64::from(i32::MIN) - offset;
        let nan_bits = 0x7ff8_0000_0000_0000_u64 | (u64::from(rng.next_u32()) << 1) | 1;
        for (label, value, expected) in [
            ("upper", upper, i32::MAX),
            ("lower", lower, i32::MIN),
            ("nan", f64::from_bits(nan_bits), 0),
        ] {
            let c_result = unsafe { (c.safe_double_to_int)(value) };
            let rust_result = unsafe { (rust.safe_double_to_int)(value) };
            assert_eq!(
                c_result, rust_result,
                "{label} conversion iteration {iteration}"
            );
            assert_eq!(c_result, expected, "{label} conversion result");
        }
    }

    // CONFIGS C10-C13.
    let fixed_scaled = [
        (0, f64::NAN),
        (1, f64::INFINITY),
        (1, f64::NEG_INFINITY),
        (-1, f64::INFINITY),
        (-1, f64::NEG_INFINITY),
        (i32::MAX, 1.0),
        (i32::MIN, 1.0),
        (123, -0.75),
    ];
    for (base, scale) in fixed_scaled {
        let c_result = unsafe { (c.compute_scaled_value)(base, scale) };
        let rust_result = unsafe { (rust.compute_scaled_value)(base, scale) };
        assert_eq!(c_result, rust_result, "scaled base={base}, scale={scale:?}");
    }
    for iteration in 0..2_000 {
        let base = rng.i32_inclusive(-1_000_000, 1_000_000);
        let scale = f64::from(rng.i32_inclusive(-2_000, 2_000)) / 37.0;
        let c_result = unsafe { (c.compute_scaled_value)(base, scale) };
        let rust_result = unsafe { (rust.compute_scaled_value)(base, scale) };
        assert_eq!(c_result, rust_result, "random scaled value {iteration}");
    }
    for iteration in 0..500 {
        let magnitude = rng.i32_inclusive(1_000_000, 2_000_000);
        let scale = f64::from(rng.i32_inclusive(3_000, 5_000));
        let nan_bits = 0xfff8_0000_0000_0000_u64 | (u64::from(rng.next_u32()) << 1) | 1;
        for (label, base, factor, expected) in [
            ("upper", magnitude, scale, i32::MAX),
            ("lower", -magnitude, scale, i32::MIN),
            ("nan", magnitude, f64::from_bits(nan_bits), 0),
        ] {
            let c_result = unsafe { (c.compute_scaled_value)(base, factor) };
            let rust_result = unsafe { (rust.compute_scaled_value)(base, factor) };
            assert_eq!(
                c_result, rust_result,
                "{label} scaled iteration {iteration}"
            );
            assert_eq!(c_result, expected, "{label} scaled result");
        }
    }
}

#[test]
fn compare_results_matches_index_relations_and_rejections() {
    let (c, rust) = load_apis();
    let mut rng = Lcg::new(0xc0de_c0de_1234_5678);

    // CONFIGS C14-C18 and ERRORS E5-E6.
    for iteration in 0..500 {
        let count = rng.i32_inclusive(1, 10);
        let base = seeded_array(count, u64::from(rng.next_u32()), 1_000_000);
        for (idx1, idx2) in [
            (0, count - 1),
            (count - 1, count - 1),
            (count - 1, 0),
            (count, 0),
            (0, count),
            (count + 1, count + 2),
        ] {
            let mut c_array = base;
            let mut rust_array = base;
            let c_result = unsafe { (c.compare_results)(&mut c_array, idx1, idx2) };
            let rust_result = unsafe { (rust.compare_results)(&mut rust_array, idx1, idx2) };
            assert_eq!(
                c_result, rust_result,
                "iteration {iteration}, count={count}, indices=({idx1}, {idx2})"
            );
            assert_arrays_equal(&c_array, &rust_array, "compare_results");
        }
    }
}

#[test]
fn init_result_array_matches_every_count_shape() {
    let (c, rust) = load_apis();
    let mut rng = Lcg::new(0x1a17_a22a_900d);

    // CONFIGS C19-C24 and ERROR E7.
    for iteration in 0..300 {
        for count in [-7, 0, 1, 2, 5, 9, 10, 11, 100] {
            let mut values = [0_i32; 100];
            for value in &mut values {
                *value = rng.i32_inclusive(i32::MIN, i32::MAX);
            }
            let mut c_values = values;
            let mut rust_values = values;
            let mut c_array = zeroed_array();
            let mut rust_array = zeroed_array();
            let c_values_ptr = if count <= 0 {
                std::ptr::null_mut()
            } else {
                c_values.as_mut_ptr()
            };
            let rust_values_ptr = if count <= 0 {
                std::ptr::null_mut()
            } else {
                rust_values.as_mut_ptr()
            };

            unsafe {
                (c.init_result_array)(&mut c_array, c_values_ptr, count);
                (rust.init_result_array)(&mut rust_array, rust_values_ptr, count);
            }
            assert_arrays_equal(
                &c_array,
                &rust_array,
                &format!("init iteration {iteration}, count={count}"),
            );
            assert_eq!(c_array.count, count.min(10));
        }
    }
}

#[test]
fn process_with_foreach_matches_callbacks_and_mutated_bytes() {
    let (c, rust) = load_apis();
    let mut rng = Lcg::new(0xf0ae_ac11_5eed);

    // CONFIGS C25-C36.
    for iteration in 0..300 {
        for kind in [
            OperationKind::Add,
            OperationKind::Multiply,
            OperationKind::Subtract,
            OperationKind::Modulo,
        ] {
            for count in [0, 1, 2, 7, 10] {
                let base = seeded_array(count, u64::from(rng.next_u32()), 100_000);
                let mut c_array = base;
                let mut rust_array = base;
                let c_operation = c.operation(kind);
                let rust_operation = rust.operation(kind);
                let c_result = unsafe {
                    (c.process_with_foreach)(
                        &mut c_array,
                        c_operation as *const () as *const c_void,
                    )
                };
                let rust_result = unsafe {
                    (rust.process_with_foreach)(
                        &mut rust_array,
                        rust_operation as *const () as *const c_void,
                    )
                };
                assert_eq!(
                    c_result, rust_result,
                    "{kind:?}, iteration {iteration}, count={count}"
                );
                assert_arrays_equal(
                    &c_array,
                    &rust_array,
                    &format!("{kind:?}, iteration {iteration}, count={count}"),
                );
            }
        }
    }
}

#[test]
fn weighted_sum_matches_count_weight_and_saturation_shapes() {
    let (c, rust) = load_apis();
    let mut rng = Lcg::new(0xae16_47ed_5eed);

    // CONFIGS C37-C40.
    for iteration in 0..500 {
        for count in [-5, 0, 1, 2, 6, 10] {
            let base = seeded_array(count, u64::from(rng.next_u32()), 100_000);
            let mut c_array = base;
            let mut rust_array = base;
            let c_result = unsafe { (c.compute_weighted_sum)(&mut c_array) };
            let rust_result = unsafe { (rust.compute_weighted_sum)(&mut rust_array) };
            assert_eq!(
                c_result, rust_result,
                "weighted iteration {iteration}, count={count}"
            );
            assert_arrays_equal(&c_array, &rust_array, "compute_weighted_sum");
        }
    }

    // CONFIG C41. Index nine supplies a large weight, and all prior terms are zero,
    // so each randomized case saturates exactly once without overflowing the sum.
    for iteration in 0..500 {
        let positive = rng.i32_inclusive(300_000_000, i32::MAX);
        let negative = rng.i32_inclusive(i32::MIN, -300_000_000);
        for (value, expected) in [(positive, i32::MAX), (negative, i32::MIN)] {
            let mut base = zeroed_array();
            base.count = 10;
            base.data[9].value = value;
            let mut c_array = base;
            let mut rust_array = base;
            let c_result = unsafe { (c.compute_weighted_sum)(&mut c_array) };
            let rust_result = unsafe { (rust.compute_weighted_sum)(&mut rust_array) };
            assert_eq!(
                c_result, rust_result,
                "weighted saturation iteration={iteration}, value={value}"
            );
            assert_eq!(c_result, expected);
            assert_arrays_equal(&c_array, &rust_array, "weighted saturation");
        }
    }
}

#[test]
fn arrayfunc_matches_randomized_end_to_end_inputs() {
    let (c, rust) = load_apis();
    let mut rng = Lcg::new(0xa22a_f00c_5eed);

    // CONFIG C42.
    for iteration in 0..5_000 {
        let args = [
            rng.i32_inclusive(-100_000, 100_000),
            rng.i32_inclusive(-100_000, 100_000),
            rng.i32_inclusive(-100_000, 100_000),
            rng.i32_inclusive(-100_000, 100_000),
        ];
        let c_result = unsafe { (c.arrayfunc)(args[0], args[1], args[2], args[3]) };
        let rust_result = unsafe { (rust.arrayfunc)(args[0], args[1], args[2], args[3]) };
        assert_eq!(
            c_result, rust_result,
            "iteration {iteration}, args={args:?}"
        );
    }

    // CONFIG C43.
    let mixed_sign = [
        [0, 0, 0, 0],
        [1, -1, 1, -1],
        [-1, 1, -1, 1],
        [10, -20, 30, -40],
        [-10, 20, -30, 40],
        [5, 5, 5, 5],
    ];
    for args in mixed_sign {
        let c_result = unsafe { (c.arrayfunc)(args[0], args[1], args[2], args[3]) };
        let rust_result = unsafe { (rust.arrayfunc)(args[0], args[1], args[2], args[3]) };
        assert_eq!(c_result, rust_result, "mixed-sign args={args:?}");
    }
    for iteration in 0..1_000 {
        let args = [
            0,
            rng.i32_inclusive(1, 100_000),
            rng.i32_inclusive(-100_000, -1),
            rng.i32_inclusive(-100_000, 100_000),
        ];
        let c_result = unsafe { (c.arrayfunc)(args[0], args[1], args[2], args[3]) };
        let rust_result = unsafe { (rust.arrayfunc)(args[0], args[1], args[2], args[3]) };
        assert_eq!(
            c_result, rust_result,
            "random mixed-sign iteration {iteration}, args={args:?}"
        );
    }

    // CONFIG C44: large magnitudes chosen to keep every C arithmetic operation defined.
    let large_defined = [
        [10_000_000, -10_000_000, 0, 10_000_000],
        [-10_000_000, 10_000_000, 0, -10_000_000],
        [20_000_000, 0, 0, 0],
        [-20_000_000, 0, 0, 0],
    ];
    for args in large_defined {
        let c_result = unsafe { (c.arrayfunc)(args[0], args[1], args[2], args[3]) };
        let rust_result = unsafe { (rust.arrayfunc)(args[0], args[1], args[2], args[3]) };
        assert_eq!(c_result, rust_result, "large args={args:?}");
    }
    for iteration in 0..2_000 {
        let mut large = || {
            let magnitude = rng.i32_inclusive(1_000_000, 5_000_000);
            if rng.next_u32() & 1 == 0 {
                magnitude
            } else {
                -magnitude
            }
        };
        let args = [large(), large(), large(), large()];
        let c_result = unsafe { (c.arrayfunc)(args[0], args[1], args[2], args[3]) };
        let rust_result = unsafe { (rust.arrayfunc)(args[0], args[1], args[2], args[3]) };
        assert_eq!(
            c_result, rust_result,
            "random large iteration {iteration}, args={args:?}"
        );
    }
}

#[test]
fn ffi_crash_probe() {
    let Some(probe) = std::env::var_os("ARRAYFUNC_CRASH_PROBE") else {
        return;
    };
    let library = std::env::var_os("ARRAYFUNC_PROBE_LIBRARY").unwrap();
    let api = unsafe { Api::load(Path::new(&library)) };
    let mut array = zeroed_array();

    unsafe {
        match probe.to_str().unwrap() {
            "compare_null_array" => {
                (api.compare_results)(std::ptr::null_mut(), 0, 0);
            }
            "init_null_array" => {
                (api.init_result_array)(std::ptr::null_mut(), std::ptr::null_mut(), 0);
            }
            "init_null_values" => {
                (api.init_result_array)(&mut array, std::ptr::null_mut(), 1);
            }
            "process_null_array" => {
                (api.process_with_foreach)(
                    std::ptr::null_mut(),
                    api.add as *const () as *const c_void,
                );
            }
            "process_null_callback" => {
                array.count = 1;
                (api.process_with_foreach)(&mut array, std::ptr::null());
            }
            "weighted_null_array" => {
                (api.compute_weighted_sum)(std::ptr::null_mut());
            }
            unknown => panic!("unknown crash probe {unknown}"),
        }
    }

    panic!("unchecked invalid call unexpectedly returned");
}

#[test]
fn unchecked_null_pointer_behavior_is_isolated_and_equivalent() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().unwrap();
    for probe in [
        "compare_null_array",
        "init_null_array",
        "init_null_values",
        "process_null_array",
        "process_null_callback",
        "weighted_null_array",
    ] {
        let run = |library: &Path| {
            Command::new(&executable)
                .args(["--exact", "ffi_crash_probe", "--nocapture"])
                .env("ARRAYFUNC_CRASH_PROBE", probe)
                .env("ARRAYFUNC_PROBE_LIBRARY", library)
                .status()
                .unwrap()
        };
        let c_status = run(&c_library_path());
        let rust_status = run(&rust_library_path());
        assert!(!c_status.success(), "C {probe} unexpectedly succeeded");
        assert!(
            !rust_status.success(),
            "Rust {probe} unexpectedly succeeded"
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "different terminating signal for {probe}: C={c_status}, Rust={rust_status}"
        );
    }
}
