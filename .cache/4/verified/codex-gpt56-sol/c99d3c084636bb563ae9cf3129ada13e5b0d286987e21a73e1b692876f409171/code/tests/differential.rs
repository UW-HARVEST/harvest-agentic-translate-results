use libloading::Library;
use std::ffi::{c_double, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;

type SafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type ProcessArrayReverse = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type SwitchCalculator = unsafe extern "C" fn(c_int, c_int) -> c_int;
type AllocateAndCompute = unsafe extern "C" fn(c_int, c_double) -> c_int;
type ForeachSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type Fallcalc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

const RANDOM_CASES: usize = 256;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        unsafe {
            Self {
                c: Library::new(c_library_path()).expect("load C shared library"),
                rust: Library::new(rust_library_path()).expect("load Rust shared library"),
            }
        }
    }

    unsafe fn functions<T: Copy>(&self, name: &[u8]) -> (T, T) {
        unsafe {
            (
                *self.c.get::<T>(name).expect("load C symbol"),
                *self.rust.get::<T>(name).expect("load Rust symbol"),
            )
        }
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn i32(&mut self) -> i32 {
        self.u64() as i32
    }

    fn usize(&mut self, low: usize, high_exclusive: usize) -> usize {
        low + self.u64() as usize % (high_exclusive - low)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("FALLCALC_RUST_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target/release/libfallcalc_lib.so"))
}

fn assert_same<T: std::fmt::Debug + PartialEq>(c: T, rust: T, context: &str) {
    assert_eq!(c, rust, "{context}");
}

#[test]
fn configs_1_through_6_safe_double_to_int() {
    let libraries = Libraries::load();
    let (c, rust) = unsafe { libraries.functions::<SafeDoubleToInt>(b"safe_double_to_int\0") };
    let mut rng = Rng::new(0x11a2_7c45_d903_e6bf);

    let fixed_interior = [
        0.0,
        -0.0,
        0.5,
        -0.5,
        1.999,
        -1.999,
        c_int::MAX as f64 - 1.0,
        c_int::MIN as f64 + 1.0,
    ];
    for value in fixed_interior {
        assert_same(
            unsafe { c(value) },
            unsafe { rust(value) },
            "finite interior",
        );
    }
    for _ in 0..RANDOM_CASES {
        let value = rng.i32() as f64 / 2.0;
        assert_same(
            unsafe { c(value) },
            unsafe { rust(value) },
            "random interior",
        );
    }

    for _ in 0..RANDOM_CASES {
        let payload = rng.u64() & ((1_u64 << 52) - 1) | 1;
        let sign = (rng.u64() & 1) << 63;
        let value = f64::from_bits(sign | (0x7ff_u64 << 52) | payload);
        assert!(value.is_nan());
        assert_same(unsafe { c(value) }, unsafe { rust(value) }, "NaN");
    }

    for value in [f64::INFINITY, f64::NEG_INFINITY] {
        for _ in 0..RANDOM_CASES {
            assert_same(unsafe { c(value) }, unsafe { rust(value) }, "infinity");
        }
    }

    for _ in 0..RANDOM_CASES {
        let delta = (rng.u64() % 10_000_000) as f64;
        let high = c_int::MAX as f64 + delta;
        let low = c_int::MIN as f64 - delta;
        assert_same(unsafe { c(high) }, unsafe { rust(high) }, "upper clamp");
        assert_same(unsafe { c(low) }, unsafe { rust(low) }, "lower clamp");
    }
}

#[test]
fn configs_7_through_9_process_array_reverse() {
    let libraries = Libraries::load();
    let (c, rust) =
        unsafe { libraries.functions::<ProcessArrayReverse>(b"process_array_reverse\0") };
    let mut rng = Rng::new(0x829e_58dc_f015_4a73);

    for count in [0, -1, -2, c_int::MIN] {
        assert_same(
            unsafe { c(std::ptr::null_mut(), count) },
            unsafe { rust(std::ptr::null_mut(), count) },
            "empty reverse",
        );
    }
    for _ in 0..RANDOM_CASES {
        let count = -1 - (rng.u64() % c_int::MAX as u64) as i32;
        assert_same(
            unsafe { c(std::ptr::null_mut(), count) },
            unsafe { rust(std::ptr::null_mut(), count) },
            "random empty reverse",
        );
    }

    for _ in 0..RANDOM_CASES {
        let mut one = [rng.i32()];
        assert_same(
            unsafe { c(one.as_mut_ptr(), 1) },
            unsafe { rust(one.as_mut_ptr(), 1) },
            "one reverse element",
        );

        let len = rng.usize(2, 129);
        let mut values: Vec<i32> = (0..len).map(|_| rng.i32()).collect();
        let end = unsafe { values.as_mut_ptr().add(len - 1) };
        assert_same(
            unsafe { c(end, len as i32) },
            unsafe { rust(end, len as i32) },
            "many reverse elements",
        );
    }
}

#[test]
fn configs_10_through_15_switch_fallthrough() {
    let libraries = Libraries::load();
    let (c, rust) =
        unsafe { libraries.functions::<SwitchCalculator>(b"switch_fallthrough_calculator\0") };
    let mut rng = Rng::new(0x649c_03ab_28f7_d15e);

    for operation in 0..=4 {
        for _ in 0..RANDOM_CASES {
            let value = match operation {
                0 => rng.i32() % 200_000_000,
                3 => rng.i32() % 500_000_000,
                _ => rng.i32(),
            };
            assert_same(
                unsafe { c(value, operation) },
                unsafe { rust(value, operation) },
                "switch operation",
            );
        }
    }

    for operation in [-1, 5, c_int::MIN, c_int::MAX] {
        for _ in 0..RANDOM_CASES {
            let value = rng.i32();
            assert_same(
                unsafe { c(value, operation) },
                unsafe { rust(value, operation) },
                "switch default",
            );
        }
    }
    for _ in 0..RANDOM_CASES {
        let operation = loop {
            let candidate = rng.i32();
            if !(0..=4).contains(&candidate) {
                break candidate;
            }
        };
        let value = rng.i32();
        assert_same(
            unsafe { c(value, operation) },
            unsafe { rust(value, operation) },
            "random switch default",
        );
    }
}

#[test]
fn configs_16_through_23_allocate_and_compute() {
    let libraries = Libraries::load();
    let (c, rust) = unsafe { libraries.functions::<AllocateAndCompute>(b"allocate_and_compute\0") };
    let mut rng = Rng::new(0x7fd4_c219_a60b_3e85);

    for _ in 0..RANDOM_CASES {
        let multiplier = f64::from_bits(rng.u64());
        assert_same(
            unsafe { c(0, multiplier) },
            unsafe { rust(0, multiplier) },
            "size zero",
        );
        assert_same(
            unsafe { c(1, multiplier) },
            unsafe { rust(1, multiplier) },
            "size one",
        );

        let size = rng.usize(2, 65) as i32;
        let finite = (rng.i32() % 10_000) as f64 / 128.0;
        assert_same(
            unsafe { c(size, finite) },
            unsafe { rust(size, finite) },
            "finite allocation",
        );
        let payload = rng.u64() & ((1_u64 << 52) - 1) | 1;
        let nan = f64::from_bits((rng.u64() & (1 << 63)) | (0x7ff_u64 << 52) | payload);
        assert_same(unsafe { c(size, nan) }, unsafe { rust(size, nan) }, "NaN");
        assert_same(
            unsafe { c(size, f64::INFINITY) },
            unsafe { rust(size, f64::INFINITY) },
            "positive infinity",
        );
        assert_same(
            unsafe { c(size, f64::NEG_INFINITY) },
            unsafe { rust(size, f64::NEG_INFINITY) },
            "negative infinity",
        );

        let n = size as f64;
        let sum_of_squares = (n - 1.0) * n * (2.0 * n - 1.0) / 6.0;
        let factor = 8.0 * sum_of_squares;
        let target = c_int::MAX as f64 * (2.0 + (rng.u64() % 100) as f64);
        let saturating_multiplier = target / factor;
        assert_same(
            unsafe { c(size, saturating_multiplier) },
            unsafe { rust(size, saturating_multiplier) },
            "positive saturation",
        );
        assert_same(
            unsafe { c(size, -saturating_multiplier) },
            unsafe { rust(size, -saturating_multiplier) },
            "negative saturation",
        );
    }
}

#[test]
fn configs_24_through_26_foreach_sum() {
    let libraries = Libraries::load();
    let (c, rust) = unsafe { libraries.functions::<ForeachSum>(b"foreach_sum\0") };
    let mut rng = Rng::new(0xb31f_9408_6de2_57ca);

    for count in [0, -1, -2, c_int::MIN] {
        assert_same(
            unsafe { c(std::ptr::null_mut(), count) },
            unsafe { rust(std::ptr::null_mut(), count) },
            "empty foreach",
        );
    }
    for _ in 0..RANDOM_CASES {
        let count = -1 - (rng.u64() % c_int::MAX as u64) as i32;
        assert_same(
            unsafe { c(std::ptr::null_mut(), count) },
            unsafe { rust(std::ptr::null_mut(), count) },
            "random empty foreach",
        );
    }

    for _ in 0..RANDOM_CASES {
        let mut one = [rng.i32()];
        assert_same(
            unsafe { c(one.as_mut_ptr(), 1) },
            unsafe { rust(one.as_mut_ptr(), 1) },
            "one foreach element",
        );

        let len = rng.usize(2, 129);
        let mut values: Vec<i32> = (0..len).map(|_| rng.i32()).collect();
        assert_same(
            unsafe { c(values.as_mut_ptr(), len as i32) },
            unsafe { rust(values.as_mut_ptr(), len as i32) },
            "many foreach elements",
        );
    }
}

fn low_param3(rng: &mut Rng, remainder: i32) -> i32 {
    let max_quotient = (128 - remainder) / 5;
    (rng.u64() % (max_quotient as u64 + 1)) as i32 * 5 + remainder
}

fn high_param3(rng: &mut Rng, remainder: i32) -> i32 {
    let min_quotient = (129 + 4 - remainder) / 5;
    let quotient = min_quotient + (rng.u64() % 100_000) as i32;
    quotient * 5 + remainder
}

fn negative_default_param3(rng: &mut Rng) -> i32 {
    let remainder = 1 + (rng.u64() % 4) as i32;
    -((rng.u64() % 100_000) as i32 * 5 + remainder)
}

fn shaped_param4(rng: &mut Rng, iteration: usize) -> i32 {
    let quotient = (rng.u64() % 100_000) as i32;
    match iteration % 4 {
        0 => -(quotient * 10 + 2 + (rng.u64() % 8) as i32),
        1 => -(quotient * 10 + 1),
        2 => quotient * 10,
        _ => quotient * 10 + 1 + (rng.u64() % 9) as i32,
    }
}

fn compare_fallcalc_mode(
    c: Fallcalc,
    rust: Fallcalc,
    rng: &mut Rng,
    make_param3: impl Fn(&mut Rng) -> i32,
    context: &str,
) {
    for iteration in 0..RANDOM_CASES {
        let param1 = rng.i32();
        let param2 = rng.i32();
        let param3 = make_param3(rng);
        let param4 = shaped_param4(rng, iteration);
        assert_same(
            unsafe { c(param1, param2, param3, param4) },
            unsafe { rust(param1, param2, param3, param4) },
            context,
        );
    }
}

#[test]
fn configs_27_through_37_fallcalc_composed_pipeline() {
    let libraries = Libraries::load();
    let (c, rust) = unsafe { libraries.functions::<Fallcalc>(b"fallcalc\0") };
    let mut rng = Rng::new(0xce81_5a37_209d_f46b);

    for remainder in 0..=4 {
        compare_fallcalc_mode(
            c,
            rust,
            &mut rng,
            |rng| low_param3(rng, remainder),
            "fallcalc low param3",
        );
    }
    compare_fallcalc_mode(
        c,
        rust,
        &mut rng,
        negative_default_param3,
        "fallcalc default operation",
    );
    for remainder in 0..=4 {
        compare_fallcalc_mode(
            c,
            rust,
            &mut rng,
            |rng| high_param3(rng, remainder),
            "fallcalc high param3",
        );
    }
}

#[test]
fn errors_1_through_5_special_double_results() {
    let libraries = Libraries::load();
    let (c, rust) = unsafe { libraries.functions::<SafeDoubleToInt>(b"safe_double_to_int\0") };
    let cases = [
        (f64::NAN, 0),
        (f64::INFINITY, c_int::MAX),
        (f64::NEG_INFINITY, c_int::MIN),
        (c_int::MAX as f64, c_int::MAX),
        (c_int::MAX as f64 + 1.0, c_int::MAX),
        (c_int::MIN as f64, c_int::MIN),
        (c_int::MIN as f64 - 1.0, c_int::MIN),
    ];

    for (value, expected) in cases {
        assert_eq!(unsafe { c(value) }, expected, "C special double result");
        assert_eq!(
            unsafe { rust(value) },
            expected,
            "Rust special double result"
        );
    }
}

#[test]
fn error_6_allocation_failure() {
    let libraries = Libraries::load();
    let (c, rust) = unsafe { libraries.functions::<AllocateAndCompute>(b"allocate_and_compute\0") };

    assert_eq!(unsafe { c(-1, 1.0) }, -1);
    assert_eq!(unsafe { rust(-1, 1.0) }, -1);
}

fn interposer_path() -> PathBuf {
    manifest_dir().join("target/test-support/libfail_malloc.so")
}

fn build_interposer() -> PathBuf {
    let output = interposer_path();
    std::fs::create_dir_all(output.parent().expect("interposer parent"))
        .expect("create interposer directory");
    let source = manifest_dir().join("tests/fail_malloc.c");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&output)
        .arg(source)
        .status()
        .expect("run C compiler for malloc interposer");
    assert!(status.success(), "compile malloc interposer");
    output
}

fn run_preloaded_child(library: &Path) -> std::process::Output {
    let interposer = build_interposer();
    Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "child_error_7_fallcalc_allocation_failure",
            "--ignored",
            "--nocapture",
        ])
        .env("LD_PRELOAD", &interposer)
        .env("FALLCALC_CHILD_LIBRARY", library)
        .env("FALLCALC_INTERPOSER", interposer)
        .output()
        .expect("run allocation-failure child")
}

#[test]
fn error_7_fallcalc_allocation_failure() {
    let c = run_preloaded_child(&c_library_path());
    let rust = run_preloaded_child(&rust_library_path());
    assert!(
        c.status.success(),
        "C allocation-failure child failed:\n{}",
        String::from_utf8_lossy(&c.stdout)
    );
    assert!(
        rust.status.success(),
        "Rust allocation-failure child failed:\n{}",
        String::from_utf8_lossy(&rust.stdout)
    );
}

#[test]
#[ignore = "spawned with LD_PRELOAD by error_7_fallcalc_allocation_failure"]
fn child_error_7_fallcalc_allocation_failure() {
    let Some(library_path) = std::env::var_os("FALLCALC_CHILD_LIBRARY") else {
        return;
    };
    let interposer_path =
        PathBuf::from(std::env::var_os("FALLCALC_INTERPOSER").expect("interposer path"));
    unsafe {
        let library = Library::new(library_path).expect("load child library");
        let interposer = Library::new(interposer_path).expect("open preloaded interposer");
        let reject = interposer
            .get::<unsafe extern "C" fn(usize)>(b"fail_malloc_of_size\0")
            .expect("load malloc failure control");
        let fallcalc = library
            .get::<Fallcalc>(b"fallcalc\0")
            .expect("load fallcalc");
        reject(5 * std::mem::size_of::<c_int>());
        assert_eq!(fallcalc(1, 2, 3, 4), -1);
    }
}

#[test]
fn generic_boundaries_non_crashing() {
    let libraries = Libraries::load();
    let (c_reverse, rust_reverse) =
        unsafe { libraries.functions::<ProcessArrayReverse>(b"process_array_reverse\0") };
    let (c_foreach, rust_foreach) = unsafe { libraries.functions::<ForeachSum>(b"foreach_sum\0") };
    let (c_switch, rust_switch) =
        unsafe { libraries.functions::<SwitchCalculator>(b"switch_fallthrough_calculator\0") };
    let (c_allocate, rust_allocate) =
        unsafe { libraries.functions::<AllocateAndCompute>(b"allocate_and_compute\0") };

    for count in [0, -1, c_int::MIN] {
        assert_eq!(unsafe { c_reverse(std::ptr::null_mut(), count) }, 0);
        assert_eq!(unsafe { rust_reverse(std::ptr::null_mut(), count) }, 0);
        assert_eq!(unsafe { c_foreach(std::ptr::null_mut(), count) }, 0);
        assert_eq!(unsafe { rust_foreach(std::ptr::null_mut(), count) }, 0);
    }

    let mut rng = Rng::new(0x45da_970c_31ef_b826);
    let mut values: Vec<i32> = (0..4096).map(|_| rng.i32()).collect();
    let end = unsafe { values.as_mut_ptr().add(values.len() - 1) };
    assert_same(
        unsafe { c_reverse(end, values.len() as i32) },
        unsafe { rust_reverse(end, values.len() as i32) },
        "oversized reverse buffer",
    );
    assert_same(
        unsafe { c_foreach(values.as_mut_ptr(), values.len() as i32) },
        unsafe { rust_foreach(values.as_mut_ptr(), values.len() as i32) },
        "oversized foreach buffer",
    );

    for operation in [-1, 5] {
        assert_eq!(unsafe { c_switch(123, operation) }, 0);
        assert_eq!(unsafe { rust_switch(123, operation) }, 0);
    }
    assert_eq!(unsafe { c_allocate(0, 1.0) }, 0);
    assert_eq!(unsafe { rust_allocate(0, 1.0) }, 0);
    assert_eq!(unsafe { c_allocate(-1, 1.0) }, -1);
    assert_eq!(unsafe { rust_allocate(-1, 1.0) }, -1);
}

fn run_null_child(library: &Path, function: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "child_null_positive_count",
            "--ignored",
            "--nocapture",
        ])
        .env("FALLCALC_CHILD_LIBRARY", library)
        .env("FALLCALC_CHILD_FUNCTION", function)
        .status()
        .expect("run null-pointer child")
}

#[test]
#[cfg(unix)]
fn generic_null_positive_count_terminates_equally() {
    use std::os::unix::process::ExitStatusExt;

    for function in ["process_array_reverse", "foreach_sum"] {
        let c = run_null_child(&c_library_path(), function);
        let rust = run_null_child(&rust_library_path(), function);
        assert_eq!(c.signal(), rust.signal(), "{function} termination signal");
        assert!(c.signal().is_some(), "{function} must terminate by signal");
    }
}

#[test]
#[ignore = "spawned by generic_null_positive_count_terminates_equally"]
fn child_null_positive_count() {
    let Some(library_path) = std::env::var_os("FALLCALC_CHILD_LIBRARY") else {
        return;
    };
    let function = std::env::var("FALLCALC_CHILD_FUNCTION").expect("child function");
    unsafe {
        let library = Library::new(library_path).expect("load child library");
        match function.as_str() {
            "process_array_reverse" => {
                let call = library
                    .get::<ProcessArrayReverse>(b"process_array_reverse\0")
                    .expect("load process_array_reverse");
                std::hint::black_box(call(std::ptr::null_mut(), 1));
            }
            "foreach_sum" => {
                let call = library
                    .get::<ForeachSum>(b"foreach_sum\0")
                    .expect("load foreach_sum");
                std::hint::black_box(call(std::ptr::null_mut(), 1));
            }
            _ => panic!("unknown child function"),
        }
    }
}
