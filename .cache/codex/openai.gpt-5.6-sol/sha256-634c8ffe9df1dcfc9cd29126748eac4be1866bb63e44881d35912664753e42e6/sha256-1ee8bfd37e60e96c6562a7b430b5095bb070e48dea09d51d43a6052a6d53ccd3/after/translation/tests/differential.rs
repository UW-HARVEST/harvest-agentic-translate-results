use libloading::{Library, Symbol};
use std::ffi::{c_double, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

type SafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type ProcessWithFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
type CopyDataBlock = unsafe extern "C" fn(*mut c_void, *const c_void);
type HandlePointerOperations = unsafe extern "C" fn(c_int) -> c_int;
type Overunder = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fd: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const RANDOM_CASES: usize = 2_048;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

#[repr(C, align(8))]
#[derive(Clone, Copy)]
struct RawDataBlock([u8; 40]);

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
}

fn c_library_path() -> PathBuf {
    let build = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut libraries: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build.display()))
        .map(|entry| entry.expect("invalid C build entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared object in {}",
        build.display()
    );
    libraries.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/liboverunder_lib.so")
}

unsafe fn libraries() -> (Library, Library) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        rust_path.is_file(),
        "missing {}; run `cargo build --release` first",
        rust_path.display()
    );
    unsafe {
        (
            Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display())),
            Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display())),
        )
    }
}

unsafe fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = STDOUT_LOCK.lock().expect("stdout mutex poisoned");
    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    let result = call();

    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    (result, output)
}

fn assert_safe_case(
    c_fn: &Symbol<'_, SafeDoubleToInt>,
    rust_fn: &Symbol<'_, SafeDoubleToInt>,
    value: f64,
) {
    let c_result = unsafe { c_fn(value) };
    let rust_result = unsafe { rust_fn(value) };
    assert_eq!(
        rust_result,
        c_result,
        "safe_double_to_int diverged for value {value:?} ({:#018x})",
        value.to_bits()
    );
}

#[test]
fn config_01_safe_double_to_int_in_range() {
    unsafe {
        let (c, rust) = libraries();
        let c_fn: Symbol<SafeDoubleToInt> = c.get(b"safe_double_to_int").unwrap();
        let rust_fn: Symbol<SafeDoubleToInt> = rust.get(b"safe_double_to_int").unwrap();

        let fixed = [
            i32::MIN as f64,
            -2_147_483_647.999,
            -12345.875,
            -1.999,
            -f64::MIN_POSITIVE,
            -0.0,
            0.0,
            f64::from_bits(1),
            0.999,
            1.999,
            12345.875,
            i32::MAX as f64,
        ];
        for value in fixed {
            assert_safe_case(&c_fn, &rust_fn, value);
        }

        let mut rng = Rng::new(0x7b90_a4e1_2c65_d83f);
        for _ in 0..RANDOM_CASES {
            let integer = rng.next_i32();
            let fraction = (rng.next_u64() >> 12) as f64 / (1_u64 << 52) as f64;
            let value = if integer == i32::MAX {
                integer as f64
            } else {
                integer as f64 + fraction
            };
            assert_safe_case(&c_fn, &rust_fn, value);
        }
    }
}

fn check_process_code(code: i32, seed: u64) {
    unsafe {
        let (c, rust) = libraries();
        let c_fn: Symbol<ProcessWithFallthrough> = c.get(b"process_with_fallthrough").unwrap();
        let rust_fn: Symbol<ProcessWithFallthrough> =
            rust.get(b"process_with_fallthrough").unwrap();
        let fixed = [i32::MIN, -151, -1, 0, 1, 151, i32::MAX];
        for base in fixed {
            assert_eq!(
                rust_fn(code, base),
                c_fn(code, base),
                "code={code}, base={base}"
            );
        }
        let mut rng = Rng::new(seed);
        for _ in 0..RANDOM_CASES {
            let base = rng.next_i32();
            assert_eq!(
                rust_fn(code, base),
                c_fn(code, base),
                "code={code}, base={base}"
            );
        }
    }
}

#[test]
fn config_02_process_code_0() {
    check_process_code(0, 0x02);
}

#[test]
fn config_03_process_code_1() {
    check_process_code(1, 0x03);
}

#[test]
fn config_04_process_code_2() {
    check_process_code(2, 0x04);
}

#[test]
fn config_05_process_code_3() {
    check_process_code(3, 0x05);
}

#[test]
fn config_06_process_code_4() {
    check_process_code(4, 0x06);
}

#[test]
fn config_07_process_code_5() {
    check_process_code(5, 0x07);
}

#[test]
fn config_08_copy_data_block_all_bytes() {
    unsafe {
        let (c, rust) = libraries();
        let c_fn: Symbol<CopyDataBlock> = c.get(b"copy_data_block").unwrap();
        let rust_fn: Symbol<CopyDataBlock> = rust.get(b"copy_data_block").unwrap();
        let mut rng = Rng::new(0x0818_2838_4858_6878);

        for case in 0..RANDOM_CASES {
            let mut source = RawDataBlock([0; 40]);
            for chunk in source.0.chunks_mut(8) {
                chunk.copy_from_slice(&rng.next_u64().to_ne_bytes()[..chunk.len()]);
            }
            let mut c_dest = RawDataBlock([0xa5; 40]);
            let mut rust_dest = RawDataBlock([0x5a; 40]);
            c_fn(
                (&mut c_dest as *mut RawDataBlock).cast(),
                (&source as *const RawDataBlock).cast(),
            );
            rust_fn(
                (&mut rust_dest as *mut RawDataBlock).cast(),
                (&source as *const RawDataBlock).cast(),
            );
            assert_eq!(
                rust_dest.0, c_dest.0,
                "copy_data_block diverged in randomized case {case}"
            );
            assert_eq!(
                c_dest.0, source.0,
                "C did not copy all bytes in case {case}"
            );
        }
    }
}

#[test]
fn config_09_handle_pointer_operations_full_domain() {
    unsafe {
        let (c, rust) = libraries();
        let c_fn: Symbol<HandlePointerOperations> = c.get(b"handle_pointer_operations").unwrap();
        let rust_fn: Symbol<HandlePointerOperations> =
            rust.get(b"handle_pointer_operations").unwrap();
        let fixed = [
            i32::MIN,
            ((i32::MIN as i64 - 100) / 2) as i32,
            -51,
            -50,
            -1,
            0,
            1,
            (i32::MAX - 100) / 2,
            i32::MAX,
        ];
        for value in fixed {
            assert_eq!(rust_fn(value), c_fn(value), "value={value}");
        }
        let mut rng = Rng::new(0x0919_2939_4959_6979);
        for _ in 0..RANDOM_CASES {
            let value = rng.next_i32();
            assert_eq!(rust_fn(value), c_fn(value), "value={value}");
        }
    }
}

fn a_switch_category(a: i32) -> usize {
    let remainder = a % 6;
    if remainder < 0 { 6 } else { remainder as usize }
}

fn conversion_category(value: f64) -> usize {
    if value < i32::MIN as f64 {
        0
    } else if value > i32::MAX as f64 {
        2
    } else {
        1
    }
}

fn sqrt_category(a: i32, d: i32) -> usize {
    let sum = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
    usize::from(sum < 0)
}

#[test]
fn config_10_overunder_cross_product_and_output() {
    unsafe {
        let (c, rust) = libraries();
        let c_fn: Symbol<Overunder> = c.get(b"overunder").unwrap();
        let rust_fn: Symbol<Overunder> = rust.get(b"overunder").unwrap();
        let mut representatives = [[[None; 2]; 3]; 7];
        let mut rng = Rng::new(0x10a0_b0c0_d0e0_f001);

        let boundary_a = [
            i32::MIN,
            -1_431_655_767,
            -1_431_655_766,
            -6,
            -5,
            -1,
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            1_431_655_764,
            1_431_655_765,
            i32::MAX,
        ];
        let boundary_d = [
            i32::MIN,
            -46_341,
            -46_340,
            -1,
            0,
            1,
            46_340,
            46_341,
            i32::MAX,
        ];
        for a in boundary_a {
            for d in boundary_d {
                let switch = a_switch_category(a);
                let conversion = conversion_category(a as f64 * 1.5);
                let sqrt = sqrt_category(a, d);
                representatives[switch][conversion][sqrt].get_or_insert((a, d));
            }
        }
        for _ in 0..1_000_000 {
            let a = rng.next_i32();
            let d = rng.next_i32();
            let switch = a_switch_category(a);
            let conversion = conversion_category(a as f64 * 1.5);
            let sqrt = sqrt_category(a, d);
            representatives[switch][conversion][sqrt].get_or_insert((a, d));
        }

        let b_values = [i32::MIN, 0, i32::MAX];
        let mut tested_cells = 0;
        for (switch, conversions) in representatives.iter().enumerate() {
            for (a_conversion, sqrt_values) in conversions.iter().enumerate() {
                for (sqrt, representative) in sqrt_values.iter().enumerate() {
                    let Some((a, d)) = representative else {
                        continue;
                    };
                    for (b_conversion, b) in b_values.into_iter().enumerate() {
                        assert_eq!(
                            conversion_category(b as f64 * 2.7),
                            b_conversion,
                            "bad b representative"
                        );
                        for sample in 0..8 {
                            let c_value = if sample < 4 {
                                [i32::MIN, -1, 0, i32::MAX][sample]
                            } else {
                                rng.next_i32()
                            };
                            let (c_result, c_output) = capture_stdout(|| c_fn(*a, b, c_value, *d));
                            let (rust_result, rust_output) =
                                capture_stdout(|| rust_fn(*a, b, c_value, *d));
                            assert_eq!(
                                rust_result, c_result,
                                "return mismatch: switch={switch}, a_conversion={a_conversion}, \
                                 b_conversion={b_conversion}, sqrt={sqrt}, \
                                 input=({a}, {b}, {c_value}, {d})"
                            );
                            assert_eq!(
                                rust_output, c_output,
                                "stdout mismatch: switch={switch}, a_conversion={a_conversion}, \
                                 b_conversion={b_conversion}, sqrt={sqrt}, \
                                 input=({a}, {b}, {c_value}, {d})"
                            );
                        }
                        tested_cells += 1;
                    }
                }
            }
        }
        assert_eq!(
            tested_cells, 90,
            "cross-product search did not cover all feasible cells"
        );

        for case in 0..512 {
            let input = (
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            );
            let (c_result, c_output) = capture_stdout(|| c_fn(input.0, input.1, input.2, input.3));
            let (rust_result, rust_output) =
                capture_stdout(|| rust_fn(input.0, input.1, input.2, input.3));
            assert_eq!(
                rust_result, c_result,
                "randomized return mismatch in case {case}: input={input:?}"
            );
            assert_eq!(
                rust_output, c_output,
                "randomized stdout mismatch in case {case}: input={input:?}"
            );
        }
    }
}

#[test]
fn error_01_safe_above_int_max() {
    unsafe {
        let (c, rust) = libraries();
        let c_fn: Symbol<SafeDoubleToInt> = c.get(b"safe_double_to_int").unwrap();
        let rust_fn: Symbol<SafeDoubleToInt> = rust.get(b"safe_double_to_int").unwrap();
        let mut rng = Rng::new(0xe101);
        let fixed = [
            i32::MAX as f64 + 1.0,
            f64::from_bits((i32::MAX as f64).to_bits() + 1),
            1e15,
            f64::MAX,
            f64::INFINITY,
        ];
        for value in fixed {
            assert_eq!(c_fn(value), i32::MAX);
            assert_safe_case(&c_fn, &rust_fn, value);
        }
        for _ in 0..RANDOM_CASES {
            let value = i32::MAX as f64 + 1.0 + (rng.next_u64() as f64);
            assert_eq!(c_fn(value), i32::MAX);
            assert_safe_case(&c_fn, &rust_fn, value);
        }
    }
}

#[test]
fn error_02_safe_below_int_min() {
    unsafe {
        let (c, rust) = libraries();
        let c_fn: Symbol<SafeDoubleToInt> = c.get(b"safe_double_to_int").unwrap();
        let rust_fn: Symbol<SafeDoubleToInt> = rust.get(b"safe_double_to_int").unwrap();
        let mut rng = Rng::new(0xe202);
        let fixed = [
            i32::MIN as f64 - 1.0,
            f64::from_bits((i32::MIN as f64).to_bits() + 1),
            -1e15,
            -f64::MAX,
            f64::NEG_INFINITY,
        ];
        for value in fixed {
            assert_eq!(c_fn(value), i32::MIN);
            assert_safe_case(&c_fn, &rust_fn, value);
        }
        for _ in 0..RANDOM_CASES {
            let value = i32::MIN as f64 - 1.0 - (rng.next_u64() as f64);
            assert_eq!(c_fn(value), i32::MIN);
            assert_safe_case(&c_fn, &rust_fn, value);
        }
    }
}

#[test]
fn error_03_safe_nan() {
    unsafe {
        let (c, rust) = libraries();
        let c_fn: Symbol<SafeDoubleToInt> = c.get(b"safe_double_to_int").unwrap();
        let rust_fn: Symbol<SafeDoubleToInt> = rust.get(b"safe_double_to_int").unwrap();
        let mut rng = Rng::new(0xe303);
        for fixed in [
            f64::NAN,
            f64::from_bits(0x7ff0_0000_0000_0001),
            f64::from_bits(0xfff8_0000_0000_0000),
        ] {
            assert_eq!(c_fn(fixed), 0);
            assert_safe_case(&c_fn, &rust_fn, fixed);
        }
        for _ in 0..RANDOM_CASES {
            let sign_and_payload = rng.next_u64() & 0x800f_ffff_ffff_ffff;
            let value = f64::from_bits(0x7ff0_0000_0000_0000 | sign_and_payload | 1);
            assert!(value.is_nan());
            assert_eq!(c_fn(value), 0);
            assert_safe_case(&c_fn, &rust_fn, value);
        }
    }
}

#[test]
fn error_04_process_default() {
    unsafe {
        let (c, rust) = libraries();
        let c_fn: Symbol<ProcessWithFallthrough> = c.get(b"process_with_fallthrough").unwrap();
        let rust_fn: Symbol<ProcessWithFallthrough> =
            rust.get(b"process_with_fallthrough").unwrap();
        let mut rng = Rng::new(0xe404);
        for code in [i32::MIN, -1, 6, 7, i32::MAX] {
            for base in [i32::MIN, -1, 0, 1, i32::MAX] {
                assert_eq!(c_fn(code, base), -1);
                assert_eq!(rust_fn(code, base), c_fn(code, base));
            }
        }
        for _ in 0..RANDOM_CASES {
            let mut code = rng.next_i32();
            if (0..=5).contains(&code) {
                code = 6 + code;
            }
            let base = rng.next_i32();
            assert_eq!(c_fn(code, base), -1);
            assert_eq!(rust_fn(code, base), c_fn(code, base));
        }
    }
}

fn run_null_child(library: &str, null_side: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("DIFFERENTIAL_NULL_LIBRARY", library)
        .env("DIFFERENTIAL_NULL_SIDE", null_side)
        .status()
        .expect("run null-pointer child")
}

#[test]
fn generic_boundary_copy_data_block_null_pointers() {
    for null_side in ["source", "destination"] {
        let c_status = run_null_child("c", null_side);
        let rust_status = run_null_child("rust", null_side);
        assert!(
            !c_status.success(),
            "C unexpectedly accepted null {null_side}"
        );
        assert!(
            !rust_status.success(),
            "Rust unexpectedly accepted null {null_side}"
        );
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "termination signal differs for null {null_side}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

#[test]
fn null_pointer_child() {
    let Ok(library) = std::env::var("DIFFERENTIAL_NULL_LIBRARY") else {
        return;
    };
    let null_side = std::env::var("DIFFERENTIAL_NULL_SIDE").expect("null side");
    unsafe {
        let selected = if library == "c" {
            Library::new(c_library_path()).unwrap()
        } else {
            Library::new(rust_library_path()).unwrap()
        };
        let copy: Symbol<CopyDataBlock> = selected.get(b"copy_data_block").unwrap();
        let mut valid = RawDataBlock([0; 40]);
        if null_side == "source" {
            copy((&mut valid as *mut RawDataBlock).cast(), std::ptr::null());
        } else {
            copy(std::ptr::null_mut(), (&valid as *const RawDataBlock).cast());
        }
    }
}
