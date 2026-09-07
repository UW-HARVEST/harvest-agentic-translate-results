use libloading::Library;
use std::env;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type SafeDoubleToInt = unsafe extern "C" fn(f64) -> i32;
type ProcessArrayReverse = unsafe extern "C" fn(*mut i32, i32) -> i32;
type SwitchCalculator = unsafe extern "C" fn(i32, i32) -> i32;
type AllocateAndCompute = unsafe extern "C" fn(i32, f64) -> i32;
type ForeachSum = unsafe extern "C" fn(*mut i32, i32) -> i32;
type Fallcalc = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

struct Api {
    _library: Library,
    safe_double_to_int: SafeDoubleToInt,
    process_array_reverse: ProcessArrayReverse,
    switch_calculator: SwitchCalculator,
    allocate_and_compute: AllocateAndCompute,
    foreach_sum: ForeachSum,
    fallcalc: Fallcalc,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let safe_double_to_int =
            *unsafe { library.get::<SafeDoubleToInt>(b"safe_double_to_int\0") }.unwrap();
        let process_array_reverse =
            *unsafe { library.get::<ProcessArrayReverse>(b"process_array_reverse\0") }.unwrap();
        let switch_calculator =
            *unsafe { library.get::<SwitchCalculator>(b"switch_fallthrough_calculator\0") }
                .unwrap();
        let allocate_and_compute =
            *unsafe { library.get::<AllocateAndCompute>(b"allocate_and_compute\0") }.unwrap();
        let foreach_sum = *unsafe { library.get::<ForeachSum>(b"foreach_sum\0") }.unwrap();
        let fallcalc = *unsafe { library.get::<Fallcalc>(b"fallcalc\0") }.unwrap();
        Self {
            _library: library,
            safe_double_to_int,
            process_array_reverse,
            switch_calculator,
            allocate_and_compute,
            foreach_sum,
            fallcalc,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    unsafe fn load() -> Self {
        Self {
            c: unsafe { Api::load(&c_library_path()) },
            rust: unsafe { Api::load(&rust_library_path()) },
        }
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn i32_in(&mut self, low: i32, high: i32) -> i32 {
        let width = (high as i64 - low as i64 + 1) as u64;
        (low as i64 + (self.next_u64() % width) as i64) as i32
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_root()
        .join("../c_src/build/libharvest-work-cp21cR.so")
        .canonicalize()
        .expect("C shared library must be built before cargo test")
}

fn rust_library_path() -> PathBuf {
    crate_root()
        .join("target/release/libfallcalc_lib.so")
        .canonicalize()
        .expect("Rust release cdylib must be built before cargo test")
}

fn assert_same<T: std::fmt::Debug + PartialEq>(row: &str, input: impl std::fmt::Debug, c: T, r: T) {
    assert_eq!(c, r, "{row}: input {input:?}");
}

#[test]
fn config_rows_1_through_30_low_level() {
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);

    for payload in 1..=128_u64 {
        let d = f64::from_bits(0x7ff8_0000_0000_0000 | payload);
        let c = unsafe { (pair.c.safe_double_to_int)(d) };
        let r = unsafe { (pair.rust.safe_double_to_int)(d) };
        assert_same("CONFIGS row 1", d.to_bits(), c, r);
    }
    for &(row, d) in &[
        ("CONFIGS row 2", f64::INFINITY),
        ("CONFIGS row 3", f64::NEG_INFINITY),
    ] {
        for _ in 0..64 {
            let c = unsafe { (pair.c.safe_double_to_int)(d) };
            let r = unsafe { (pair.rust.safe_double_to_int)(d) };
            assert_same(row, d, c, r);
        }
    }
    for _ in 0..128 {
        let offset = (rng.next_u64() % 1_000_000) as f64;
        for &(row, d) in &[
            ("CONFIGS row 4", i32::MAX as f64 + offset),
            ("CONFIGS row 5", i32::MIN as f64 - offset),
        ] {
            let c = unsafe { (pair.c.safe_double_to_int)(d) };
            let r = unsafe { (pair.rust.safe_double_to_int)(d) };
            assert_same(row, d, c, r);
        }
        let d = rng.i32_in(-2_000_000_000, 2_000_000_000) as f64
            + (rng.next_u64() % 10_000) as f64 / 10_001.0;
        let c = unsafe { (pair.c.safe_double_to_int)(d) };
        let r = unsafe { (pair.rust.safe_double_to_int)(d) };
        assert_same("CONFIGS row 6", d, c, r);
    }

    for count in [-100, -1] {
        let c = unsafe { (pair.c.process_array_reverse)(std::ptr::null_mut(), count) };
        let r = unsafe { (pair.rust.process_array_reverse)(std::ptr::null_mut(), count) };
        assert_same("CONFIGS row 7", count, c, r);
    }
    for _ in 0..64 {
        let c = unsafe { (pair.c.process_array_reverse)(std::ptr::null_mut(), 0) };
        let r = unsafe { (pair.rust.process_array_reverse)(std::ptr::null_mut(), 0) };
        assert_same("CONFIGS row 8", 0, c, r);

        let mut one = [rng.i32_in(-1_000_000, 1_000_000)];
        let c = unsafe { (pair.c.process_array_reverse)(one.as_mut_ptr(), 1) };
        let r = unsafe { (pair.rust.process_array_reverse)(one.as_mut_ptr(), 1) };
        assert_same("CONFIGS row 9", one, c, r);

        let len = rng.i32_in(2, 32);
        let mut values: Vec<i32> = (0..len)
            .map(|_| rng.i32_in(-1_000_000, 1_000_000))
            .collect();
        let end = unsafe { values.as_mut_ptr().add(values.len() - 1) };
        let c = unsafe { (pair.c.process_array_reverse)(end, len) };
        let r = unsafe { (pair.rust.process_array_reverse)(end, len) };
        assert_same("CONFIGS row 10", values, c, r);
    }

    for operation in 0..=4 {
        for _ in 0..128 {
            let value = rng.i32_in(-100_000_000, 100_000_000);
            let c = unsafe { (pair.c.switch_calculator)(value, operation) };
            let r = unsafe { (pair.rust.switch_calculator)(value, operation) };
            assert_same(
                &format!("CONFIGS row {}", 11 + operation),
                (value, operation),
                c,
                r,
            );
        }
    }
    for _ in 0..128 {
        let operation = if rng.next_u64() & 1 == 0 {
            rng.i32_in(i32::MIN, -1)
        } else {
            rng.i32_in(5, i32::MAX)
        };
        let value = rng.i32_in(-100_000_000, 100_000_000);
        let c = unsafe { (pair.c.switch_calculator)(value, operation) };
        let r = unsafe { (pair.rust.switch_calculator)(value, operation) };
        assert_same("CONFIGS row 16", (value, operation), c, r);
    }

    for _ in 0..64 {
        for &(row, size, multiplier) in &[("CONFIGS row 17", 0, 17.0), ("CONFIGS row 18", 1, -23.0)]
        {
            let c = unsafe { (pair.c.allocate_and_compute)(size, multiplier) };
            let r = unsafe { (pair.rust.allocate_and_compute)(size, multiplier) };
            assert_same(row, (size, multiplier), c, r);
        }

        let size = rng.i32_in(2, 32);
        for &(row, multiplier) in &[
            ("CONFIGS row 19", 0.001),
            ("CONFIGS row 20", 0.0),
            ("CONFIGS row 21", -0.001),
            ("CONFIGS row 22", f64::NAN),
            ("CONFIGS row 23", f64::INFINITY),
            ("CONFIGS row 24", f64::NEG_INFINITY),
            ("CONFIGS row 25", 1.0e20),
            ("CONFIGS row 26", -1.0e20),
        ] {
            let c = unsafe { (pair.c.allocate_and_compute)(size, multiplier) };
            let r = unsafe { (pair.rust.allocate_and_compute)(size, multiplier) };
            assert_same(row, (size, multiplier.to_bits()), c, r);
        }
    }

    for count in [-100, -1] {
        let c = unsafe { (pair.c.foreach_sum)(std::ptr::null_mut(), count) };
        let r = unsafe { (pair.rust.foreach_sum)(std::ptr::null_mut(), count) };
        assert_same("CONFIGS row 27", count, c, r);
    }
    for _ in 0..64 {
        let c = unsafe { (pair.c.foreach_sum)(std::ptr::null_mut(), 0) };
        let r = unsafe { (pair.rust.foreach_sum)(std::ptr::null_mut(), 0) };
        assert_same("CONFIGS row 28", 0, c, r);

        let mut one = [rng.i32_in(-1_000_000, 1_000_000)];
        let c = unsafe { (pair.c.foreach_sum)(one.as_mut_ptr(), 1) };
        let r = unsafe { (pair.rust.foreach_sum)(one.as_mut_ptr(), 1) };
        assert_same("CONFIGS row 29", one, c, r);

        let len = rng.i32_in(2, 32);
        let mut values: Vec<i32> = (0..len)
            .map(|_| rng.i32_in(-1_000_000, 1_000_000))
            .collect();
        let c = unsafe { (pair.c.foreach_sum)(values.as_mut_ptr(), len) };
        let r = unsafe { (pair.rust.foreach_sum)(values.as_mut_ptr(), len) };
        assert_same("CONFIGS row 30", values, c, r);
    }
}

#[derive(Clone, Copy, Debug)]
enum OperationClass {
    False(i32),
    True(i32),
    DefaultFalse,
}

#[derive(Clone, Copy, Debug)]
enum ConversionClass {
    Low,
    Interior,
    High,
}

#[derive(Clone, Copy, Debug)]
enum AllocationClass {
    Negative,
    Zero,
    One,
    Many,
}

fn param3_for(class: OperationClass, rng: &mut Rng) -> i32 {
    match class {
        OperationClass::False(residue) => residue + 5 * rng.i32_in(0, 20),
        OperationClass::True(residue) => {
            let first = if residue == 4 { 129 } else { 130 + residue };
            first + 5 * rng.i32_in(0, 2000)
        }
        OperationClass::DefaultFalse => -rng.i32_in(1, 2000) * 5 - rng.i32_in(1, 4),
    }
}

fn params12_for(class: ConversionClass, rng: &mut Rng) -> (i32, i32) {
    match class {
        ConversionClass::Low => (rng.i32_in(-1000, 1000), -1_000_000_000),
        ConversionClass::Interior => (rng.i32_in(-100_000, 100_000), rng.i32_in(-100_000, 100_000)),
        ConversionClass::High => (rng.i32_in(-1000, 1000), 1_000_000_000),
    }
}

fn param4_for(class: AllocationClass, rng: &mut Rng) -> i32 {
    match class {
        AllocationClass::Negative => -10 * rng.i32_in(0, 1000) - rng.i32_in(2, 9),
        AllocationClass::Zero => -10 * rng.i32_in(0, 1000) - 1,
        AllocationClass::One => 10 * rng.i32_in(-1000, 1000),
        AllocationClass::Many => {
            let residue = rng.i32_in(1, 9);
            10 * rng.i32_in(-1000, 1000) + residue
        }
    }
}

#[test]
fn config_rows_31_through_162_fallcalc_matrix() {
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0xa409_3822_299f_31d0);
    let operations = [
        OperationClass::False(0),
        OperationClass::False(1),
        OperationClass::False(2),
        OperationClass::False(3),
        OperationClass::False(4),
        OperationClass::True(0),
        OperationClass::True(1),
        OperationClass::True(2),
        OperationClass::True(3),
        OperationClass::True(4),
        OperationClass::DefaultFalse,
    ];
    let conversions = [
        ConversionClass::Low,
        ConversionClass::Interior,
        ConversionClass::High,
    ];
    let allocations = [
        AllocationClass::Negative,
        AllocationClass::Zero,
        AllocationClass::One,
        AllocationClass::Many,
    ];

    let mut row = 31;
    for operation in operations {
        for conversion in conversions {
            for allocation in allocations {
                for _ in 0..64 {
                    let (param1, param2) = params12_for(conversion, &mut rng);
                    let param3 = param3_for(operation, &mut rng);
                    let param4 = param4_for(allocation, &mut rng);
                    let input = (param1, param2, param3, param4);
                    let c = unsafe { (pair.c.fallcalc)(param1, param2, param3, param4) };
                    let r = unsafe { (pair.rust.fallcalc)(param1, param2, param3, param4) };
                    assert_same(&format!("CONFIGS row {row}"), input, c, r);
                }
                row += 1;
            }
        }
    }
    assert_eq!(row, 163);
}

#[test]
fn error_rows_without_process_isolation() {
    let pair = unsafe { Pair::load() };
    let exceptional = [
        (1, f64::NAN),
        (2, f64::INFINITY),
        (3, f64::NEG_INFINITY),
        (4, i32::MAX as f64),
        (5, i32::MIN as f64),
    ];
    for (row, d) in exceptional {
        let c = unsafe { (pair.c.safe_double_to_int)(d) };
        let r = unsafe { (pair.rust.safe_double_to_int)(d) };
        assert_same(&format!("ERRORS row {row}"), d.to_bits(), c, r);
    }

    let c = unsafe { (pair.c.allocate_and_compute)(-1, 1.5) };
    let r = unsafe { (pair.rust.allocate_and_compute)(-1, 1.5) };
    assert_same("ERRORS row 6", -1, c, r);
    assert_eq!(c, -1);

    for &(row, operation) in &[(8, -1), (9, 5)] {
        let c = unsafe { (pair.c.switch_calculator)(123, operation) };
        let r = unsafe { (pair.rust.switch_calculator)(123, operation) };
        assert_same(&format!("ERRORS row {row}"), operation, c, r);
        assert_eq!(c, 0);
    }

    for &(row, count) in &[(10, 0), (11, -1)] {
        let c = unsafe { (pair.c.process_array_reverse)(std::ptr::null_mut(), count) };
        let r = unsafe { (pair.rust.process_array_reverse)(std::ptr::null_mut(), count) };
        assert_same(&format!("ERRORS row {row}"), count, c, r);
        assert_eq!(c, 0);
    }
    for &(row, count) in &[(13, 0), (14, -1)] {
        let c = unsafe { (pair.c.foreach_sum)(std::ptr::null_mut(), count) };
        let r = unsafe { (pair.rust.foreach_sum)(std::ptr::null_mut(), count) };
        assert_same(&format!("ERRORS row {row}"), count, c, r);
        assert_eq!(c, 0);
    }
}

fn run_crash_child(library: &str, function: &str) -> ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("crash_child_dispatch")
        .arg("--nocapture")
        .env("FALLCALC_CRASH_CHILD", "1")
        .env("FALLCALC_CRASH_LIBRARY", library)
        .env("FALLCALC_CRASH_FUNCTION", function)
        .status()
        .unwrap()
}

#[test]
fn error_rows_12_15_16_17_invalid_span_termination() {
    use std::os::unix::process::ExitStatusExt;

    for &(row, function) in &[
        (12, "reverse_null"),
        (15, "foreach_null"),
        (16, "reverse_oversized"),
        (17, "foreach_oversized"),
    ] {
        let c = run_crash_child("c", function);
        let rust = run_crash_child("rust", function);
        assert!(!c.success(), "ERRORS row {row}: C unexpectedly succeeded");
        assert!(
            !rust.success(),
            "ERRORS row {row}: Rust unexpectedly succeeded"
        );
        assert_eq!(
            c.signal(),
            rust.signal(),
            "ERRORS row {row}: different terminating signals"
        );
    }
}

unsafe fn guarded_pointer(reverse: bool) -> *mut i32 {
    unsafe extern "C" {
        fn getpagesize() -> i32;
        fn mmap(
            address: *mut c_void,
            length: usize,
            protection: i32,
            flags: i32,
            fd: i32,
            offset: isize,
        ) -> *mut c_void;
        fn mprotect(address: *mut c_void, length: usize, protection: i32) -> i32;
    }

    const PROT_NONE: i32 = 0;
    const PROT_READ: i32 = 1;
    const PROT_WRITE: i32 = 2;
    const MAP_PRIVATE: i32 = 2;
    const MAP_ANONYMOUS: i32 = 0x20;

    let page = unsafe { getpagesize() } as usize;
    let mapping = unsafe {
        mmap(
            std::ptr::null_mut(),
            page * 2,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert_ne!(mapping as isize, -1, "mmap failed");

    let pointer = if reverse {
        assert_eq!(unsafe { mprotect(mapping, page, PROT_NONE) }, 0);
        unsafe { mapping.byte_add(page).cast::<i32>() }
    } else {
        assert_eq!(
            unsafe { mprotect(mapping.byte_add(page), page, PROT_NONE) },
            0
        );
        unsafe { mapping.byte_add(page - size_of::<i32>()).cast::<i32>() }
    };
    unsafe { pointer.write(7) };
    pointer
}

#[test]
fn crash_child_dispatch() {
    if env::var_os("FALLCALC_CRASH_CHILD").is_none() {
        return;
    }
    let path = match env::var("FALLCALC_CRASH_LIBRARY").unwrap().as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown child library {other}"),
    };
    let api = unsafe { Api::load(&path) };
    match env::var("FALLCALC_CRASH_FUNCTION").unwrap().as_str() {
        "reverse_null" => unsafe {
            (api.process_array_reverse)(std::ptr::null_mut(), 1);
        },
        "foreach_null" => unsafe {
            (api.foreach_sum)(std::ptr::null_mut(), 1);
        },
        "reverse_oversized" => unsafe {
            (api.process_array_reverse)(guarded_pointer(true), 2);
        },
        "foreach_oversized" => unsafe {
            (api.foreach_sum)(guarded_pointer(false), 2);
        },
        other => panic!("unknown child function {other}"),
    }
}

fn failmalloc_path() -> PathBuf {
    env::temp_dir().join(format!("fallcalc-failmalloc-{}.so", std::process::id()))
}

fn build_failmalloc() -> PathBuf {
    let output = failmalloc_path();
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&output)
        .arg(crate_root().join("tests/malloc_fail.c"))
        .status()
        .expect("failed to execute cc for malloc interposer");
    assert!(status.success(), "failed to build malloc interposer");
    output
}

#[test]
fn error_row_7_fallcalc_allocation_failure() {
    if env::var_os("FALLCALC_MALLOC_CHILD").is_some() {
        return;
    }
    let preload = build_failmalloc();
    let status = Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("malloc_failure_child")
        .arg("--nocapture")
        .env("FALLCALC_MALLOC_CHILD", "1")
        .env("LD_PRELOAD", &preload)
        .env("FALLCALC_FAILMALLOC_SO", &preload)
        .status()
        .unwrap();
    assert!(status.success(), "ERRORS row 7 child failed: {status}");
    std::fs::remove_file(preload).ok();
}

#[test]
fn malloc_failure_child() {
    if env::var_os("FALLCALC_MALLOC_CHILD").is_none() {
        return;
    }
    type Arm = unsafe extern "C" fn(usize);
    let interposer =
        unsafe { Library::new(env::var_os("FALLCALC_FAILMALLOC_SO").unwrap()).unwrap() };
    let arm = *unsafe { interposer.get::<Arm>(b"fail_next_malloc_of_size\0") }.unwrap();
    let pair = unsafe { Pair::load() };

    unsafe { arm(5 * size_of::<i32>()) };
    let c = unsafe { (pair.c.fallcalc)(1, 2, 3, 4) };
    unsafe { arm(5 * size_of::<i32>()) };
    let rust = unsafe { (pair.rust.fallcalc)(1, 2, 3, 4) };
    assert_same("ERRORS row 7", 20, c, rust);
    assert_eq!(c, -1);
}

#[test]
fn exported_symbols_are_callable_only_through_loaded_libraries() {
    let pair = unsafe { Pair::load() };
    let c = unsafe { (pair.c.fallcalc)(1, 2, 3, 4) };
    let rust = unsafe { (pair.rust.fallcalc)(1, 2, 3, 4) };
    assert_eq!(c, rust);
    let _: *const c_void = (&pair.c._library as *const Library).cast();
}
