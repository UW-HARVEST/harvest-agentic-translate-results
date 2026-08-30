use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

type PrintIntPtrLine = unsafe extern "C" fn(*const c_int);
type VoidFn = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);

#[unsafe(naked)]
unsafe extern "C" fn call_bad_with_seed(_function: VoidFn, _seed: *const c_int) {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "mov qword ptr [rsp - 24], rsi",
        "call rdi",
        "leave",
        "ret",
    );
}

#[unsafe(naked)]
unsafe extern "C" fn call_driver_zero_with_seed(
    _function: Driver,
    _argument: c_int,
    _seed: *const c_int,
) {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "mov rax, rdi",
        "mov edi, esi",
        "mov qword ptr [rsp - 56], rdx",
        "call rax",
        "leave",
        "ret",
    );
}

unsafe extern "C" {
    fn close(fd: RawFd) -> c_int;
    fn dup(fd: RawFd) -> RawFd;
    fn dup2(old_fd: RawFd, new_fd: RawFd) -> RawFd;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut RawFd) -> c_int;
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library is missing; build c_src first")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("libdriver.so")
        .canonicalize()
        .expect("Rust cdylib is missing; run cargo build --release first")
}

fn both_library_paths() -> [PathBuf; 2] {
    [c_library_path(), rust_library_path()]
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds = [0; 2];
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut output = Vec::new();
    unsafe {
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
    }
    output
}

unsafe fn call_print(path: &Path, value: c_int) -> Vec<u8> {
    let library = unsafe { Library::new(path).expect("load shared library") };
    let function: Symbol<PrintIntPtrLine> = unsafe {
        library
            .get(b"printIntPtrLine")
            .expect("load printIntPtrLine")
    };
    capture_stdout(|| unsafe { function(&value) })
}

unsafe fn call_good(path: &Path) -> Vec<u8> {
    let library = unsafe { Library::new(path).expect("load shared library") };
    let function: Symbol<VoidFn> = unsafe { library.get(b"good").expect("load good") };
    capture_stdout(|| unsafe { function() })
}

unsafe fn call_driver(path: &Path, use_good: c_int) -> Vec<u8> {
    let library = unsafe { Library::new(path).expect("load shared library") };
    let function: Symbol<Driver> = unsafe { library.get(b"driver").expect("load driver") };
    capture_stdout(|| unsafe { function(use_good) })
}

fn random_ints() -> Vec<c_int> {
    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    let mut state = 0xd1ff_e2e5_c0de_5eed_u64;
    for _ in 0..512 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push(state as c_int);
    }
    values
}

fn run_isolated(path: &Path, symbol: &str, argument: Option<c_int>) -> ExitStatus {
    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command
        .arg("--exact")
        .arg("isolated_ffi_case")
        .arg("--nocapture")
        .env("DIFF_CHILD_LIBRARY", path)
        .env("DIFF_CHILD_SYMBOL", symbol)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(value) = argument {
        command.env("DIFF_CHILD_ARGUMENT", value.to_string());
    }
    command.status().expect("run isolated FFI case")
}

fn assert_same_signal(symbol: &str, argument: Option<c_int>) {
    let [c_path, rust_path] = both_library_paths();
    let c_status = run_isolated(&c_path, symbol, argument);
    let rust_status = run_isolated(&rust_path, symbol, argument);
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "{symbol} terminated differently: C={c_status:?}, Rust={rust_status:?}"
    );
    assert!(
        c_status.signal().is_some(),
        "{symbol} unexpectedly returned normally: C={c_status:?}, Rust={rust_status:?}"
    );
}

fn assert_isolated_pair_matches(symbol: &str, argument: Option<c_int>) {
    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command
        .arg("--exact")
        .arg("isolated_ffi_pair")
        .arg("--nocapture")
        .env("DIFF_PAIR_SYMBOL", symbol)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(value) = argument {
        command.env("DIFF_PAIR_ARGUMENT", value.to_string());
    }
    let status = command.status().expect("run isolated FFI pair");
    assert!(
        status.success(),
        "isolated {symbol} comparison failed: {status:?}"
    );
}

#[test]
fn configs_c1_print_int_ptr_line_randomized() {
    let [c_path, rust_path] = both_library_paths();
    for value in random_ints() {
        let c_output = unsafe { call_print(&c_path, value) };
        let rust_output = unsafe { call_print(&rust_path, value) };
        assert_eq!(c_output, rust_output, "output differs for value {value}");
    }
}

#[test]
fn config_c2_good() {
    let [c_path, rust_path] = both_library_paths();
    for _ in 0..64 {
        let c_output = unsafe { call_good(&c_path) };
        let rust_output = unsafe { call_good(&rust_path) };
        assert_eq!(c_output, rust_output);
    }
}

#[test]
fn config_c3_bad() {
    assert_isolated_pair_matches("bad", None);
}

#[test]
fn config_c4_driver_zero() {
    assert_isolated_pair_matches("driver", Some(0));
}

#[test]
fn config_c5_driver_nonzero_randomized() {
    let [c_path, rust_path] = both_library_paths();
    for value in random_ints().into_iter().filter(|value| *value != 0) {
        let c_output = unsafe { call_driver(&c_path, value) };
        let rust_output = unsafe { call_driver(&rust_path, value) };
        assert_eq!(c_output, rust_output, "output differs for useGood={value}");
    }
}

#[test]
fn error_g1_print_int_ptr_line_null() {
    assert_same_signal("printIntPtrLine", None);
}

#[test]
fn isolated_ffi_case() {
    let Ok(path) = std::env::var("DIFF_CHILD_LIBRARY") else {
        return;
    };
    let symbol = std::env::var("DIFF_CHILD_SYMBOL").expect("child symbol");
    let library = unsafe { Library::new(path).expect("load child shared library") };
    unsafe {
        match symbol.as_str() {
            "printIntPtrLine" => {
                let function: Symbol<PrintIntPtrLine> = library
                    .get(b"printIntPtrLine")
                    .expect("load printIntPtrLine");
                function(std::ptr::null());
            }
            "bad" => {
                let function: Symbol<VoidFn> = library.get(b"bad").expect("load bad");
                function();
            }
            "driver" => {
                let argument = std::env::var("DIFF_CHILD_ARGUMENT")
                    .expect("child argument")
                    .parse()
                    .expect("integer child argument");
                let function: Symbol<Driver> = library.get(b"driver").expect("load driver");
                function(argument);
            }
            _ => panic!("unknown child symbol: {symbol}"),
        }
    }
}

#[test]
fn isolated_ffi_pair() {
    let Ok(symbol) = std::env::var("DIFF_PAIR_SYMBOL") else {
        return;
    };
    let [c_path, rust_path] = both_library_paths();
    unsafe {
        let c_library = Library::new(c_path).expect("load C shared library");
        let rust_library = Library::new(rust_path).expect("load Rust shared library");
        match symbol.as_str() {
            "bad" => {
                let c_function: Symbol<VoidFn> = c_library.get(b"bad").expect("load C bad");
                let rust_function: Symbol<VoidFn> =
                    rust_library.get(b"bad").expect("load Rust bad");
                for seed in random_ints() {
                    let c_output = capture_stdout(|| call_bad_with_seed(*c_function, &seed));
                    let rust_output = capture_stdout(|| call_bad_with_seed(*rust_function, &seed));
                    assert_eq!(c_output, rust_output, "bad output differs for seed {seed}");
                }
            }
            "driver" => {
                let argument = std::env::var("DIFF_PAIR_ARGUMENT")
                    .expect("pair argument")
                    .parse()
                    .expect("integer pair argument");
                let c_function: Symbol<Driver> = c_library.get(b"driver").expect("load C driver");
                let rust_function: Symbol<Driver> =
                    rust_library.get(b"driver").expect("load Rust driver");
                let warmup_seed = 0;
                let _ = capture_stdout(|| {
                    call_driver_zero_with_seed(*c_function, argument, &warmup_seed)
                });
                let _ = capture_stdout(|| {
                    call_driver_zero_with_seed(*rust_function, argument, &warmup_seed)
                });
                for seed in random_ints() {
                    let c_output =
                        capture_stdout(|| call_driver_zero_with_seed(*c_function, argument, &seed));
                    let rust_output = capture_stdout(|| {
                        call_driver_zero_with_seed(*rust_function, argument, &seed)
                    });
                    assert_eq!(
                        c_output, rust_output,
                        "driver output differs for stack seed {seed}"
                    );
                }
            }
            _ => panic!("unknown pair symbol: {symbol}"),
        }
    }
}

#[test]
fn dynamic_export_sets_match() {
    fn exports(path: &Path) -> Vec<String> {
        let output = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("run nm");
        assert!(output.status.success(), "nm failed for {}", path.display());
        let mut names: Vec<_> = String::from_utf8(output.stdout)
            .expect("nm output is UTF-8")
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2))
            .map(str::to_owned)
            .collect();
        names.sort();
        names
    }

    let [c_path, rust_path] = both_library_paths();
    assert_eq!(exports(&c_path), exports(&rust_path));
}
