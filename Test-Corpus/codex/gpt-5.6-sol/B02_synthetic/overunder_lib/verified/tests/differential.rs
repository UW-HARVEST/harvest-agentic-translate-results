use libloading::Library;
use std::ffi::{c_char, c_double, c_int, c_void};
use std::io::Read;
use std::mem::{MaybeUninit, size_of};
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

#[repr(C)]
struct DataBlock {
    id: c_int,
    value: c_double,
    label: [c_char; 20],
}

type SafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type ProcessWithFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
type CopyDataBlock = unsafe extern "C" fn(*mut DataBlock, *const DataBlock);
type HandlePointerOperations = unsafe extern "C" fn(c_int) -> c_int;
type Overunder = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    safe_double_to_int: SafeDoubleToInt,
    process_with_fallthrough: ProcessWithFallthrough,
    copy_data_block: CopyDataBlock,
    handle_pointer_operations: HandlePointerOperations,
    overunder: Overunder,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let safe_double_to_int = unsafe {
            *library
                .get::<SafeDoubleToInt>(b"safe_double_to_int\0")
                .unwrap()
        };
        let process_with_fallthrough = unsafe {
            *library
                .get::<ProcessWithFallthrough>(b"process_with_fallthrough\0")
                .unwrap()
        };
        let copy_data_block =
            unsafe { *library.get::<CopyDataBlock>(b"copy_data_block\0").unwrap() };
        let handle_pointer_operations = unsafe {
            *library
                .get::<HandlePointerOperations>(b"handle_pointer_operations\0")
                .unwrap()
        };
        let overunder = unsafe { *library.get::<Overunder>(b"overunder\0").unwrap() };

        Self {
            _library: library,
            safe_double_to_int,
            process_with_fallthrough,
            copy_data_block,
            handle_pointer_operations,
            overunder,
        }
    }
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConversionClass {
    Low,
    InRange,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SumSign {
    Negative,
    Nonnegative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RemainderPath {
    Exact(i32),
    Default,
}

struct Lcg(u64);

impl Lcg {
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
        (self.next_u64() >> 32) as u32 as i32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/debug/liboverunder_lib.so")
}

fn conversion_class(value: f64) -> ConversionClass {
    if value > i32::MAX as f64 {
        ConversionClass::High
    } else if value < i32::MIN as f64 {
        ConversionClass::Low
    } else {
        ConversionClass::InRange
    }
}

fn t1_class(a: i32) -> ConversionClass {
    conversion_class(a as f64 * 1.5)
}

fn t2_class(b: i32) -> ConversionClass {
    conversion_class(b as f64 * 2.7)
}

fn remainder_matches(a: i32, path: RemainderPath) -> bool {
    match path {
        RemainderPath::Exact(value) => a % 6 == value,
        RemainderPath::Default => matches!(a % 6, -5..=-1),
    }
}

fn sum_sign(a: i32, d: i32) -> SumSign {
    let sum = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
    if sum < 0 {
        SumSign::Negative
    } else {
        SumSign::Nonnegative
    }
}

fn capture_stdout<R>(call: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let mut pipe_fds = [-1; 2];
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
    }
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    let result = call();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut bytes = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_end(&mut bytes).unwrap();
    (result, bytes)
}

fn compare_overunder(c_api: &Api, rust_api: &Api, args: [i32; 4], row: usize) {
    let (c_result, c_stdout) =
        capture_stdout(|| unsafe { (c_api.overunder)(args[0], args[1], args[2], args[3]) });
    let (rust_result, rust_stdout) =
        capture_stdout(|| unsafe { (rust_api.overunder)(args[0], args[1], args[2], args[3]) });
    assert_eq!(
        rust_result, c_result,
        "return mismatch for CONFIGS.md row {row}, args={args:?}"
    );
    assert_eq!(
        rust_stdout, c_stdout,
        "stdout mismatch for CONFIGS.md row {row}, args={args:?}"
    );
}

fn find_i32(rng: &mut Lcg, mut predicate: impl FnMut(i32) -> bool) -> i32 {
    for _ in 0..1_000_000 {
        let candidate = rng.next_i32();
        if predicate(candidate) {
            return candidate;
        }
    }
    panic!("failed to generate an input for a required configuration");
}

fn find_a_and_d(
    rng: &mut Lcg,
    path: RemainderPath,
    class: ConversionClass,
    sign: SumSign,
) -> (i32, i32) {
    for _ in 0..1_000_000 {
        let a = rng.next_i32();
        let d = rng.next_i32();
        if remainder_matches(a, path) && t1_class(a) == class && sum_sign(a, d) == sign {
            return (a, d);
        }
    }
    panic!("failed to generate a/d for {path:?}, {class:?}, {sign:?}");
}

fn run_null_probe(implementation: &str, null_arg: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_null_pointer_probe")
        .arg("--nocapture")
        .env("FFI_NULL_PROBE_IMPL", implementation)
        .env("FFI_NULL_PROBE_ARG", null_arg)
        .status()
        .unwrap()
}

#[test]
fn differential_valid_and_error_surface() {
    let _stdout_guard = STDOUT_LOCK.lock().unwrap();
    assert_eq!(size_of::<DataBlock>(), 40);
    assert!(c_library_path().is_file(), "C shared object was not built");
    assert!(
        rust_library_path().is_file(),
        "Rust shared object was not built"
    );

    let c_api = unsafe { Api::load(&c_library_path()) };
    let rust_api = unsafe { Api::load(&rust_library_path()) };
    let mut rng = Lcg::new(0x5eed_d1ff_2025_0825);

    // CONFIGS.md row 1: finite, in-range conversions.
    let mut valid_doubles = vec![i32::MIN as f64, -1.75, -0.0, 0.0, 1.75, i32::MAX as f64];
    for _ in 0..512 {
        let integer = rng.next_i32();
        let fraction = (rng.next_u64() & 0xffff) as f64 / 65_536.0;
        let value = if integer == i32::MAX {
            integer as f64
        } else if integer >= 0 {
            integer as f64 + fraction
        } else if integer == i32::MIN {
            integer as f64
        } else {
            integer as f64 - fraction
        };
        valid_doubles.push(value);
    }
    for value in valid_doubles {
        let c_result = unsafe { (c_api.safe_double_to_int)(value) };
        let rust_result = unsafe { (rust_api.safe_double_to_int)(value) };
        assert_eq!(rust_result, c_result, "in-range conversion for {value:?}");
    }

    // CONFIGS.md rows 2-7: every switch arm with randomized bases.
    for code in 0..=5 {
        for _ in 0..512 {
            let base = rng.next_i32();
            let c_result = unsafe { (c_api.process_with_fallthrough)(code, base) };
            let rust_result = unsafe { (rust_api.process_with_fallthrough)(code, base) };
            assert_eq!(rust_result, c_result, "switch code={code}, base={base}");
        }
    }

    // CONFIGS.md row 8: memcpy must preserve fields and padding byte-for-byte.
    for _ in 0..512 {
        let mut source = MaybeUninit::<DataBlock>::uninit();
        let mut c_dest = MaybeUninit::<DataBlock>::uninit();
        let mut rust_dest = MaybeUninit::<DataBlock>::uninit();
        unsafe {
            let source_bytes = std::slice::from_raw_parts_mut(
                source.as_mut_ptr().cast::<u8>(),
                size_of::<DataBlock>(),
            );
            for byte in source_bytes {
                *byte = rng.next_u64() as u8;
            }
            ptr::write_bytes(
                c_dest.as_mut_ptr().cast::<u8>(),
                0xa5,
                size_of::<DataBlock>(),
            );
            ptr::write_bytes(
                rust_dest.as_mut_ptr().cast::<u8>(),
                0x5a,
                size_of::<DataBlock>(),
            );
            (c_api.copy_data_block)(c_dest.as_mut_ptr(), source.as_ptr());
            (rust_api.copy_data_block)(rust_dest.as_mut_ptr(), source.as_ptr());
            let c_bytes =
                std::slice::from_raw_parts(c_dest.as_ptr().cast::<u8>(), size_of::<DataBlock>());
            let rust_bytes =
                std::slice::from_raw_parts(rust_dest.as_ptr().cast::<u8>(), size_of::<DataBlock>());
            assert_eq!(rust_bytes, c_bytes);
        }
    }

    // CONFIGS.md row 9: the full input bit-pattern domain, sampled randomly.
    for value in [i32::MIN, -100, -1, 0, 1, i32::MAX] {
        let c_result = unsafe { (c_api.handle_pointer_operations)(value) };
        let rust_result = unsafe { (rust_api.handle_pointer_operations)(value) };
        assert_eq!(rust_result, c_result, "pointer operation for {value}");
    }
    for _ in 0..2_048 {
        let value = rng.next_i32();
        let c_result = unsafe { (c_api.handle_pointer_operations)(value) };
        let rust_result = unsafe { (rust_api.handle_pointer_operations)(value) };
        assert_eq!(rust_result, c_result, "pointer operation for {value}");
    }

    // CONFIGS.md rows 10-99: the pruned cross-product in table order.
    let path_classes: &[(RemainderPath, &[ConversionClass])] = &[
        (
            RemainderPath::Exact(0),
            &[
                ConversionClass::Low,
                ConversionClass::InRange,
                ConversionClass::High,
            ],
        ),
        (
            RemainderPath::Exact(1),
            &[ConversionClass::InRange, ConversionClass::High],
        ),
        (
            RemainderPath::Exact(2),
            &[ConversionClass::InRange, ConversionClass::High],
        ),
        (
            RemainderPath::Exact(3),
            &[ConversionClass::InRange, ConversionClass::High],
        ),
        (
            RemainderPath::Exact(4),
            &[ConversionClass::InRange, ConversionClass::High],
        ),
        (
            RemainderPath::Exact(5),
            &[ConversionClass::InRange, ConversionClass::High],
        ),
        (
            RemainderPath::Default,
            &[ConversionClass::Low, ConversionClass::InRange],
        ),
    ];
    let b_classes = [
        ConversionClass::Low,
        ConversionClass::InRange,
        ConversionClass::High,
    ];
    let signs = [SumSign::Negative, SumSign::Nonnegative];
    let mut config_row = 10;
    for &(path, a_classes) in path_classes {
        for &a_class in a_classes {
            for &b_class in &b_classes {
                for &sign in &signs {
                    for _ in 0..16 {
                        let (a, d) = find_a_and_d(&mut rng, path, a_class, sign);
                        let b = find_i32(&mut rng, |candidate| t2_class(candidate) == b_class);
                        let c = rng.next_i32();
                        compare_overunder(&c_api, &rust_api, [a, b, c, d], config_row);
                    }
                    config_row += 1;
                }
            }
        }
    }
    assert_eq!(config_row, 100);

    // ERRORS.md rows 1-3: both one-past boundaries, infinities, and NaN payloads.
    let mut high_values = vec![i32::MAX as f64 + 1.0, f64::INFINITY];
    let mut low_values = vec![i32::MIN as f64 - 1.0, f64::NEG_INFINITY];
    let mut nan_values = vec![f64::NAN];
    for _ in 0..256 {
        high_values.push(i32::MAX as f64 + 1.0 + (rng.next_u64() & 0xffff_ffff) as f64);
        low_values.push(i32::MIN as f64 - 1.0 - (rng.next_u64() & 0xffff_ffff) as f64);
        let payload = rng.next_u64() & 0x0007_ffff_ffff_ffff;
        nan_values.push(f64::from_bits(0x7ff8_0000_0000_0000 | payload));
    }
    for (values, expected) in [
        (high_values, i32::MAX),
        (low_values, i32::MIN),
        (nan_values, 0),
    ] {
        for value in values {
            let c_result = unsafe { (c_api.safe_double_to_int)(value) };
            let rust_result = unsafe { (rust_api.safe_double_to_int)(value) };
            assert_eq!(c_result, expected, "unexpected C result for {value:?}");
            assert_eq!(rust_result, c_result, "error conversion for {value:?}");
        }
    }

    // ERRORS.md row 4: all out-of-range switch values use the exact -1 sentinel.
    for code in [i32::MIN, -100, -1, 6, 7, 100, i32::MAX] {
        for _ in 0..64 {
            let base = rng.next_i32();
            let c_result = unsafe { (c_api.process_with_fallthrough)(code, base) };
            let rust_result = unsafe { (rust_api.process_with_fallthrough)(code, base) };
            assert_eq!(c_result, -1, "unexpected C sentinel for code={code}");
            assert_eq!(rust_result, c_result, "default switch code={code}");
        }
    }

    // Generic pointer boundary: compare the observed fault for each null argument.
    for null_arg in ["dest", "src"] {
        let c_status = run_null_probe("c", null_arg);
        let rust_status = run_null_probe("rust", null_arg);
        assert!(!c_status.success(), "C null-{null_arg} probe did not fail");
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "null-{null_arg} termination signal differs"
        );
    }
}

#[test]
fn ffi_null_pointer_probe() {
    let Ok(implementation) = std::env::var("FFI_NULL_PROBE_IMPL") else {
        return;
    };
    let null_arg = std::env::var("FFI_NULL_PROBE_ARG").unwrap();
    let path = match implementation.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown probe implementation {other}"),
    };
    let api = unsafe { Api::load(&path) };
    let mut block = MaybeUninit::<DataBlock>::zeroed();
    unsafe {
        match null_arg.as_str() {
            "dest" => (api.copy_data_block)(ptr::null_mut(), block.as_ptr()),
            "src" => (api.copy_data_block)(block.as_mut_ptr(), ptr::null()),
            other => panic!("unknown null argument {other}"),
        }
    }
    panic!("null pointer probe unexpectedly returned");
}
