use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_int, c_void};
use std::fs::{self, File};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

const CHILD_MARKER: &str = "DRIVER_FFI_CHILD";
const LIBRARY_PATH: &str = "DRIVER_FFI_LIBRARY";
const ACTION: &str = "DRIVER_FFI_ACTION";
const OUTPUT_PATH: &str = "DRIVER_FFI_OUTPUT";

static C_LIBRARY: OnceLock<PathBuf> = OnceLock::new();
static NEXT_OUTPUT: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> &'static Path {
    C_LIBRARY
        .get_or_init(|| {
            let output_dir = manifest_dir().join("target/c-reference");
            fs::create_dir_all(&output_dir).expect("create C reference directory");
            let output = output_dir.join("libdriver_c.so");
            let source = manifest_dir().join("c_src/src/main.c");
            let result = Command::new("cc")
                .args(["-shared", "-fPIC", "-O0", "-o"])
                .arg(&output)
                .arg(source)
                .output()
                .expect("run C compiler");
            assert!(
                result.status.success(),
                "C shared-library build failed:\n{}",
                String::from_utf8_lossy(&result.stderr)
            );
            output
        })
        .as_path()
}

fn rust_library() -> PathBuf {
    let test_executable = env::current_exe().expect("locate integration-test executable");
    let deps_dir = test_executable
        .parent()
        .expect("integration-test executable directory");
    let profile_dir = deps_dir.parent().expect("Cargo profile directory");
    let candidates = [
        profile_dir.join("libdriver.so"),
        deps_dir.join("libdriver.so"),
    ];

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib not found; expected {}",
                profile_dir.join("libdriver.so").display()
            )
        })
}

fn child_output_path() -> PathBuf {
    let sequence = NEXT_OUTPUT.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "driver-differential-{}-{sequence}.out",
        std::process::id()
    ))
}

fn invoke(library: &Path, action: &str, input: &[u8]) -> Vec<u8> {
    let output_path = child_output_path();
    let mut child = Command::new(env::current_exe().expect("locate test executable"))
        .args(["--exact", "ffi_child", "--nocapture"])
        .env(CHILD_MARKER, "1")
        .env(LIBRARY_PATH, library)
        .env(ACTION, action)
        .env(OUTPUT_PATH, &output_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn FFI child");

    child
        .stdin
        .take()
        .expect("open child stdin")
        .write_all(input)
        .expect("write child stdin");

    let result = child.wait_with_output().expect("wait for FFI child");
    assert!(
        result.status.success(),
        "FFI child failed for {} ({action}):\n{}",
        library.display(),
        String::from_utf8_lossy(&result.stderr)
    );

    let output = fs::read(&output_path).expect("read redirected library output");
    fs::remove_file(output_path).expect("remove redirected output");
    output
}

fn compare(action: &str, input: &[u8]) {
    let c_output = invoke(c_library(), action, input);
    let rust_path = rust_library();
    let rust_output = invoke(&rust_path, action, input);
    assert_eq!(
        c_output,
        rust_output,
        "output mismatch for action {action:?} and input {:?}",
        String::from_utf8_lossy(input)
    );
}

fn randomized_i32s(count: usize) -> Vec<i32> {
    let mut values = vec![
        i32::MIN,
        i32::MIN + 1,
        -1_000_000,
        -1,
        0,
        1,
        1_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut state = 0x6a09_e667_f3bc_c909_u64;

    while values.len() < count {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push(state as i32);
    }
    values
}

#[test]
fn ffi_child() {
    if env::var_os(CHILD_MARKER).is_none() {
        return;
    }

    let output_path = env::var_os(OUTPUT_PATH).expect("child output path");
    let output = File::create(output_path).expect("create child output");
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(unsafe { dup2(output.as_raw_fd(), 1) }, 1, "redirect stdout");

    let library_path = env::var_os(LIBRARY_PATH).expect("child library path");
    let action = env::var(ACTION).expect("child action");
    let library = unsafe { Library::new(library_path) }.expect("load shared library");

    unsafe {
        if let Some(arguments) = action.strip_prefix("run:") {
            let run: Symbol<unsafe extern "C" fn(c_int)> =
                library.get(b"run\0").expect("resolve run");
            for argument in arguments.split(',') {
                run(argument.parse().expect("parse run argument"));
            }
        } else {
            assert_eq!(action, "main");
            let main: Symbol<unsafe extern "C" fn() -> c_int> =
                library.get(b"main\0").expect("resolve main");
            assert_eq!(main(), 0);
        }

        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush library output");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }
}

#[test]
fn config_1_run_from_fresh_state() {
    for value in randomized_i32s(40) {
        compare(&format!("run:{value}"), b"");
    }
}

#[test]
fn config_2_run_accumulates_global_state() {
    let values = randomized_i32s(128);
    let arguments = values
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    compare(&format!("run:{arguments}"), b"");
}

#[test]
fn config_3_main_successful_decimal_conversion() {
    for (index, value) in randomized_i32s(40).into_iter().enumerate() {
        let input = match (index % 4, value >= 0) {
            (0, _) => format!("{value}\n"),
            (1, true) => format!(" \t+{value}\r\n"),
            (1, false) => format!(" \t{value}\r\n"),
            (2, _) => format!("\x0b\x0c{value} "),
            _ => format!("000{value}\n"),
        };
        compare("main", input.as_bytes());
    }
}

#[test]
fn config_4_main_accepts_decimal_prefix() {
    for (index, value) in randomized_i32s(40).into_iter().enumerate() {
        let input = format!(" \t{value}suffix_{index}\n");
        compare("main", input.as_bytes());
    }
}

#[test]
fn config_5_main_failed_conversion_retains_zero() {
    let mut inputs = vec![
        Vec::new(),
        b"not-a-number\n".to_vec(),
        b" \t\r\n".to_vec(),
        b"+".to_vec(),
        b"-".to_vec(),
    ];
    let mut state = 0xbb67_ae85_84ca_a73b_u64;

    while inputs.len() < 40 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(format!("q{state:x}\n").into_bytes());
    }

    for input in inputs {
        compare("main", &input);
    }
}
