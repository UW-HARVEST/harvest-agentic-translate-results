use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_int, c_void};
use std::fs::{self, File};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const HELPER_LIBRARY: &str = "DRIVER_FFI_HELPER_LIBRARY";
const HELPER_SYMBOL: &str = "DRIVER_FFI_HELPER_SYMBOL";
const HELPER_VALUES: &str = "DRIVER_FFI_HELPER_VALUES";
const HELPER_OUTPUT: &str = "DRIVER_FFI_HELPER_OUTPUT";
const HELPER_RESULT: &str = "DRIVER_FFI_HELPER_RESULT";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

#[derive(Debug, Eq, PartialEq)]
struct Invocation {
    stdout: Vec<u8>,
    result: c_int,
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join(format!(
            "{}driver_c{}",
            env::consts::DLL_PREFIX,
            env::consts::DLL_SUFFIX
        ))
}

fn rust_library_path() -> PathBuf {
    let test_executable = env::current_exe().expect("current test executable");
    let deps_dir = test_executable.parent().expect("test deps directory");
    let profile_dir = deps_dir.parent().expect("Cargo profile directory");
    let filename = format!(
        "{}driver{}",
        env::consts::DLL_PREFIX,
        env::consts::DLL_SUFFIX
    );

    [profile_dir.join(&filename), deps_dir.join(&filename)]
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("Rust shared library {filename} was not built"))
}

fn temp_path(label: &str) -> PathBuf {
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "driver-differential-{}-{sequence}-{label}",
        std::process::id()
    ))
}

fn invoke(library: &Path, symbol: &str, values: &[c_int], stdin_bytes: &[u8]) -> Invocation {
    let output_path = temp_path("stdout");
    let result_path = temp_path("result");
    let values = values
        .iter()
        .map(c_int::to_string)
        .collect::<Vec<_>>()
        .join(",");

    let mut child = Command::new(env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "ffi_subprocess_helper",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(HELPER_LIBRARY, library)
        .env(HELPER_SYMBOL, symbol)
        .env(HELPER_VALUES, values)
        .env(HELPER_OUTPUT, &output_path)
        .env(HELPER_RESULT, &result_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn FFI helper");

    child
        .stdin
        .take()
        .expect("helper stdin")
        .write_all(stdin_bytes)
        .expect("write helper stdin");

    let process_output = child.wait_with_output().expect("wait for FFI helper");
    assert!(
        process_output.status.success(),
        "FFI helper failed for {}::{symbol}\nstdout:\n{}\nstderr:\n{}",
        library.display(),
        String::from_utf8_lossy(&process_output.stdout),
        String::from_utf8_lossy(&process_output.stderr)
    );

    let stdout = fs::read(&output_path).expect("read captured FFI stdout");
    let result = fs::read_to_string(&result_path)
        .expect("read captured FFI result")
        .parse()
        .expect("parse captured FFI result");
    fs::remove_file(output_path).expect("remove captured FFI stdout");
    fs::remove_file(result_path).expect("remove captured FFI result");

    Invocation { stdout, result }
}

fn invoke_both(symbol: &str, values: &[c_int], stdin_bytes: &[u8]) {
    let c = invoke(&c_library_path(), symbol, values, stdin_bytes);
    let rust = invoke(&rust_library_path(), symbol, values, stdin_bytes);
    assert_eq!(rust, c, "differential mismatch for {symbol}");
}

fn random_i32s(count: usize) -> Vec<c_int> {
    let mut state = 0x6a09_e667_f3bc_c909_u64;
    (0..count)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as c_int
        })
        .collect()
}

#[test]
fn ffi_subprocess_helper() {
    let Some(library_path) = env::var_os(HELPER_LIBRARY) else {
        return;
    };

    let symbol = env::var(HELPER_SYMBOL).expect("helper symbol");
    let output_path = env::var_os(HELPER_OUTPUT).expect("helper output path");
    let result_path = env::var_os(HELPER_RESULT).expect("helper result path");
    let output_file = File::create(output_path).expect("create helper output");

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup stdout failed");
    assert_eq!(
        unsafe { dup2(output_file.as_raw_fd(), 1) },
        1,
        "redirect stdout failed"
    );

    let library = unsafe { Library::new(library_path) }.expect("load shared library");
    let result = unsafe {
        match symbol.as_str() {
            "driver" => {
                let driver: Symbol<'_, unsafe extern "C" fn(c_int)> =
                    library.get(b"driver\0").expect("load driver");
                let values = env::var(HELPER_VALUES).expect("helper driver values");
                for value in values.split(',').filter(|value| !value.is_empty()) {
                    driver(value.parse().expect("parse helper driver value"));
                }
                0
            }
            "main" => {
                let main: Symbol<'_, unsafe extern "C" fn() -> c_int> =
                    library.get(b"main\0").expect("load main");
                main()
            }
            other => panic!("unknown helper symbol {other}"),
        }
    };

    std::io::stdout().flush().expect("flush Rust stdout");
    assert_eq!(
        unsafe { fflush(std::ptr::null_mut()) },
        0,
        "flush C stdout failed"
    );
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "restore stdout failed");
    assert_eq!(
        unsafe { close(saved_stdout) },
        0,
        "close saved stdout failed"
    );
    fs::write(result_path, result.to_string()).expect("write helper result");
}

#[test]
fn dynamic_symbol_surface_matches() {
    for library_path in [c_library_path(), rust_library_path()] {
        let library = unsafe { Library::new(&library_path) }
            .unwrap_or_else(|error| panic!("load {}: {error}", library_path.display()));
        unsafe {
            let _: Symbol<'_, unsafe extern "C" fn(c_int)> =
                library.get(b"driver\0").expect("load driver symbol");
            let _: Symbol<'_, unsafe extern "C" fn() -> c_int> =
                library.get(b"main\0").expect("load main symbol");
        }
    }
}

#[test]
fn config_01_driver_full_int_domain() {
    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    values.extend(random_i32s(512));
    invoke_both("driver", &values, &[]);
}

#[test]
fn config_02_main_scanf_success() {
    let mut values = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    values.extend(random_i32s(96));

    for (index, value) in values.into_iter().enumerate() {
        let input = match index % 4 {
            0 => format!("{value}\n"),
            1 => format!(" \t{value} trailing"),
            2 if value >= 0 => format!("+{value}\n"),
            2 => format!("{value}\n"),
            _ if value < 0 => format!("-000{}\n", value.unsigned_abs()),
            _ => format!("000{value}\n"),
        };
        invoke_both("main", &[], input.as_bytes());
    }
}

#[test]
fn config_03_main_scanf_matching_failure() {
    let starters = b"abcdefghijklmnopqrstuvwxyz!@#$%^&*()[]{}";
    for (index, starter) in starters.iter().copied().cycle().take(64).enumerate() {
        let input = format!(
            "{}{:08x}\n",
            char::from(starter),
            random_i32s(index + 1)[index]
        );
        invoke_both("main", &[], input.as_bytes());
    }
}

#[test]
fn config_04_main_scanf_eof() {
    invoke_both("main", &[], &[]);
    let whitespace = [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];
    for length in 1..=64 {
        let input = (0..length)
            .map(|index| whitespace[(index * 5 + length) % whitespace.len()])
            .collect::<Vec<_>>();
        invoke_both("main", &[], &input);
    }
}
