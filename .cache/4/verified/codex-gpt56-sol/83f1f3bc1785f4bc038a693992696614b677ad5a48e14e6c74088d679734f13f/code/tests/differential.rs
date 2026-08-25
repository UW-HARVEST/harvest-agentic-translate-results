use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;

type StaticAlias = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
type Driver = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const SAMPLES_PER_ROW: u64 = 24;
const FIXED_SEED: u64 = 0x5a17_1ca5_d1ff_2026;

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

    fn inclusive(&mut self, low: c_int, high: c_int) -> c_int {
        assert!(low <= high);
        low + (self.next_u32() % ((high - low + 1) as u32)) as c_int
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libStaticAlias.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/libStaticAlias.so")
}

unsafe fn load_functions(library: &Library) -> (Symbol<'_, StaticAlias>, Symbol<'_, Driver>) {
    let static_alias =
        unsafe { library.get(b"static_alias\0") }.expect("missing static_alias export");
    let driver = unsafe { library.get(b"driver\0") }.expect("missing driver export");
    (static_alias, driver)
}

unsafe fn call_pair(
    c_function: StaticAlias,
    rust_function: StaticAlias,
    c_argument: *mut c_int,
    rust_argument: *mut c_int,
    c_caller_storage: *mut c_int,
    rust_caller_storage: *mut c_int,
) -> (*mut c_int, *mut c_int) {
    assert_eq!(unsafe { *c_argument }, unsafe { *rust_argument });

    let c_result = unsafe { c_function(c_argument) };
    let rust_result = unsafe { rust_function(rust_argument) };

    assert!(!c_result.is_null());
    assert!(!rust_result.is_null());
    assert_eq!(unsafe { *c_result }, unsafe { *rust_result });
    assert_eq!(
        c_result == c_caller_storage,
        rust_result == rust_caller_storage,
        "returned aliases differ"
    );
    assert_eq!(
        unsafe { *c_caller_storage },
        unsafe { *rust_caller_storage },
        "caller-owned values differ"
    );

    (c_result, rust_result)
}

unsafe fn prefix_inner(
    c_function: StaticAlias,
    rust_function: StaticAlias,
    increment: c_int,
) -> c_int {
    let mut c_value = increment;
    let mut rust_value = increment;
    let c_storage = ptr::addr_of_mut!(c_value);
    let rust_storage = ptr::addr_of_mut!(rust_value);
    let (c_result, rust_result) = unsafe {
        call_pair(
            c_function,
            rust_function,
            c_storage,
            rust_storage,
            c_storage,
            rust_storage,
        )
    };
    assert_ne!(c_result, c_storage);
    assert_ne!(rust_result, rust_storage);
    increment + 1
}

unsafe fn capture_driver(function: Driver, initial_value: c_int, iterations: c_int) -> Vec<u8> {
    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    unsafe { function(initial_value, iterations) };
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_end(&mut output).expect("read driver stdout");
    output
}

unsafe fn run_static_row(
    row: u32,
    rng: &mut Rng,
    c_function: StaticAlias,
    rust_function: StaticAlias,
) {
    match row {
        1..=3 => {
            let inner = unsafe { prefix_inner(c_function, rust_function, rng.inclusive(1, 1000)) };
            let offset = match row {
                1 => -rng.inclusive(1, 1000),
                2 => 0,
                3 => rng.inclusive(1, 1000),
                _ => unreachable!(),
            };
            let mut c_value = inner + offset;
            let mut rust_value = c_value;
            let c_storage = ptr::addr_of_mut!(c_value);
            let rust_storage = ptr::addr_of_mut!(rust_value);
            let (c_result, rust_result) = unsafe {
                call_pair(
                    c_function,
                    rust_function,
                    c_storage,
                    rust_storage,
                    c_storage,
                    rust_storage,
                )
            };
            assert_eq!(c_result == c_storage, row == 1);
            assert_eq!(rust_result == rust_storage, row == 1);
        }
        4 => {
            let calls = rng.inclusive(2, 8);
            let mut c_value = -rng.inclusive(calls + 2, calls + 100);
            let mut rust_value = c_value;
            let c_storage = ptr::addr_of_mut!(c_value);
            let rust_storage = ptr::addr_of_mut!(rust_value);
            let mut c_argument = c_storage;
            let mut rust_argument = rust_storage;
            for _ in 0..calls {
                (c_argument, rust_argument) = unsafe {
                    call_pair(
                        c_function,
                        rust_function,
                        c_argument,
                        rust_argument,
                        c_storage,
                        rust_storage,
                    )
                };
                assert_eq!(c_argument, c_storage);
                assert_eq!(rust_argument, rust_storage);
            }
        }
        5 => {
            let below_calls = rng.inclusive(1, 8);
            let mut c_value = 1 - below_calls;
            let mut rust_value = c_value;
            let c_storage = ptr::addr_of_mut!(c_value);
            let rust_storage = ptr::addr_of_mut!(rust_value);
            let mut c_argument = c_storage;
            let mut rust_argument = rust_storage;
            for _ in 0..below_calls {
                (c_argument, rust_argument) = unsafe {
                    call_pair(
                        c_function,
                        rust_function,
                        c_argument,
                        rust_argument,
                        c_storage,
                        rust_storage,
                    )
                };
                assert_eq!(c_argument, c_storage);
                assert_eq!(rust_argument, rust_storage);
            }
            (c_argument, rust_argument) = unsafe {
                call_pair(
                    c_function,
                    rust_function,
                    c_argument,
                    rust_argument,
                    c_storage,
                    rust_storage,
                )
            };
            assert_ne!(c_argument, c_storage);
            assert_ne!(rust_argument, rust_storage);
        }
        6 => {
            let mut c_value = rng.inclusive(1, 30);
            let mut rust_value = c_value;
            let c_storage = ptr::addr_of_mut!(c_value);
            let rust_storage = ptr::addr_of_mut!(rust_value);
            let (mut c_argument, mut rust_argument) = unsafe {
                call_pair(
                    c_function,
                    rust_function,
                    c_storage,
                    rust_storage,
                    c_storage,
                    rust_storage,
                )
            };
            assert_ne!(c_argument, c_storage);
            assert_ne!(rust_argument, rust_storage);
            for _ in 0..rng.inclusive(2, 5) {
                (c_argument, rust_argument) = unsafe {
                    call_pair(
                        c_function,
                        rust_function,
                        c_argument,
                        rust_argument,
                        c_storage,
                        rust_storage,
                    )
                };
                assert_ne!(c_argument, c_storage);
                assert_ne!(rust_argument, rust_storage);
            }
        }
        _ => panic!("invalid static_alias row {row}"),
    }
}

unsafe fn run_driver_row(
    row: u32,
    rng: &mut Rng,
    c_static_alias: StaticAlias,
    rust_static_alias: StaticAlias,
    c_driver: Driver,
    rust_driver: Driver,
) {
    let (initial_value, iterations) = match row {
        7 => (rng.next_u32() as c_int, -rng.inclusive(1, 1000)),
        8 => (rng.next_u32() as c_int, 0),
        9..=11 => {
            let inner =
                unsafe { prefix_inner(c_static_alias, rust_static_alias, rng.inclusive(1, 1000)) };
            let initial = match row {
                9 => inner - rng.inclusive(1, 1000),
                10 => inner,
                11 => inner + rng.inclusive(1, 1000),
                _ => unreachable!(),
            };
            (initial, 1)
        }
        12 => {
            let iterations = rng.inclusive(2, 10);
            let initial = -rng.inclusive(iterations, iterations + 100);
            (initial, iterations)
        }
        13 => {
            let initial = -rng.inclusive(0, 8);
            let iterations = 2 - initial + rng.inclusive(0, 3);
            (initial, iterations)
        }
        14 => (rng.inclusive(1, 30), rng.inclusive(2, 8)),
        _ => panic!("invalid driver row {row}"),
    };

    let c_output = unsafe { capture_driver(c_driver, initial_value, iterations) };
    let rust_output = unsafe { capture_driver(rust_driver, initial_value, iterations) };
    assert_eq!(
        c_output, rust_output,
        "driver bytes differ for ({initial_value}, {iterations})"
    );
}

unsafe fn run_child_case(row: u32, sample: u64) {
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C shared object");
    let rust_library =
        unsafe { Library::new(rust_library_path()) }.expect("load Rust shared object");
    let (c_static_alias, c_driver) = unsafe { load_functions(&c_library) };
    let (rust_static_alias, rust_driver) = unsafe { load_functions(&rust_library) };
    let seed = FIXED_SEED ^ (u64::from(row) << 32) ^ sample.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut rng = Rng::new(seed);

    if row <= 6 {
        unsafe { run_static_row(row, &mut rng, *c_static_alias, *rust_static_alias) };
    } else {
        unsafe {
            run_driver_row(
                row,
                &mut rng,
                *c_static_alias,
                *rust_static_alias,
                *c_driver,
                *rust_driver,
            )
        };
    }
}

fn spawn_self(environment: &[(&str, String)]) -> Output {
    let mut command = Command::new(env::current_exe().expect("current test executable"));
    command.args([
        "--exact",
        "differential_surface_and_boundaries",
        "--nocapture",
        "--test-threads=1",
    ]);
    for (name, value) in environment {
        command.env(name, value);
    }
    command.output().expect("spawn isolated differential case")
}

fn assert_child_passed(row: u32, sample: u64, output: Output) {
    assert!(
        output.status.success(),
        "CONFIGS.md row {row}, sample {sample} failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

unsafe fn run_null_child(which: &str) {
    let path = match which {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown null-test library {which}"),
    };
    let library = unsafe { Library::new(path) }.expect("load null-test shared object");
    let function: Symbol<'_, StaticAlias> =
        unsafe { library.get(b"static_alias\0") }.expect("load static_alias");
    let _ = unsafe { function(ptr::null_mut()) };
    panic!("null static_alias call unexpectedly returned");
}

fn assert_symbol_parity() {
    fn exports(path: &Path) -> Vec<String> {
        let output = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("run nm");
        assert!(output.status.success(), "nm failed for {}", path.display());
        let mut symbols: Vec<_> = String::from_utf8(output.stdout)
            .expect("nm output is UTF-8")
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2))
            .map(str::to_owned)
            .collect();
        symbols.sort();
        symbols
    }

    assert_eq!(exports(&c_library_path()), exports(&rust_library_path()));
}

#[test]
fn differential_surface_and_boundaries() {
    if let Ok(value) = env::var("DIFF_CHILD_CASE") {
        let mut fields = value.split(':');
        let row = fields.next().unwrap().parse().unwrap();
        let sample = fields.next().unwrap().parse().unwrap();
        assert!(fields.next().is_none());
        unsafe { run_child_case(row, sample) };
        return;
    }
    if let Ok(which) = env::var("DIFF_NULL_CHILD") {
        unsafe { run_null_child(&which) };
        return;
    }

    assert_symbol_parity();

    for row in 1..=14 {
        for sample in 0..SAMPLES_PER_ROW {
            let output = spawn_self(&[("DIFF_CHILD_CASE", format!("{row}:{sample}"))]);
            assert_child_passed(row, sample, output);
        }
    }

    // C has no rejection rows. Exercise the generic null boundary out of
    // process because dereferencing null has no defined in-process C result.
    let c_null = spawn_self(&[("DIFF_NULL_CHILD", "c".to_owned())]);
    let rust_null = spawn_self(&[("DIFF_NULL_CHILD", "rust".to_owned())]);
    assert!(!c_null.status.success());
    assert!(!rust_null.status.success());
    assert_eq!(c_null.status.signal(), rust_null.status.signal());
    assert_eq!(c_null.status.signal(), Some(11));

    // Extreme non-positive loop counts are defined and must remain no-ops.
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C shared object");
    let rust_library =
        unsafe { Library::new(rust_library_path()) }.expect("load Rust shared object");
    let (_, c_driver) = unsafe { load_functions(&c_library) };
    let (_, rust_driver) = unsafe { load_functions(&rust_library) };
    for iterations in [c_int::MIN, -1, 0] {
        let c_output = unsafe { capture_driver(*c_driver, c_int::MAX, iterations) };
        let rust_output = unsafe { capture_driver(*rust_driver, c_int::MAX, iterations) };
        assert_eq!(c_output, rust_output);
        assert!(c_output.is_empty());
    }

    let c_output = unsafe { capture_driver(*c_driver, c_int::MIN, 64) };
    let rust_output = unsafe { capture_driver(*rust_driver, c_int::MIN, 64) };
    assert_eq!(c_output, rust_output);
}
