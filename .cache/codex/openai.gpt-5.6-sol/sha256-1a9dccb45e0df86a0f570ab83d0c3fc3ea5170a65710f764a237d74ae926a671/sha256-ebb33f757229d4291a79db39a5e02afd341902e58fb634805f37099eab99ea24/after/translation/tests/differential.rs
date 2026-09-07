use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

type DriverFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const OUTPUT_MODE: c_int = 0o600;

static OUTPUT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
enum Call {
    Run(i32),
    Driver(i32),
}

impl Call {
    fn encode(self) -> String {
        match self {
            Self::Run(value) => format!("r:{value}"),
            Self::Driver(value) => format!("d:{value}"),
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = manifest
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library was not built");
    let rust_library = manifest
        .join("target/release/libdriver.so")
        .canonicalize()
        .expect("Rust release shared library was not built");
    (c_library, rust_library)
}

fn output_path(label: &str) -> PathBuf {
    let id = OUTPUT_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "driver-differential-{label}-{}-{id}.out",
        std::process::id()
    ))
}

fn run_child(library: &Path, calls: &[Call], label: &str) -> Vec<u8> {
    let output_path = output_path(label);
    let encoded = calls
        .iter()
        .copied()
        .map(Call::encode)
        .collect::<Vec<_>>()
        .join(",");

    let child = Command::new(env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_differential_child")
        .arg("--nocapture")
        .env("DRIVER_DIFF_LIBRARY", library)
        .env("DRIVER_DIFF_CALLS", encoded)
        .env("DRIVER_DIFF_OUTPUT", &output_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn differential child");

    assert!(
        child.status.success(),
        "child failed for {}: {}",
        library.display(),
        String::from_utf8_lossy(&child.stderr)
    );

    let bytes = fs::read(&output_path).expect("read redirected native stdout");
    fs::remove_file(&output_path).expect("remove differential output");
    bytes
}

fn assert_same(calls: &[Call], label: &str) {
    let (c_library, rust_library) = library_paths();
    let c_output = run_child(&c_library, calls, &format!("c-{label}"));
    let rust_output = run_child(&rust_library, calls, &format!("rust-{label}"));
    assert_eq!(
        c_output, rust_output,
        "native stdout differs for calls {calls:?}"
    );
}

fn parse_calls(encoded: &str) -> Vec<Call> {
    if encoded.is_empty() {
        return Vec::new();
    }

    encoded
        .split(',')
        .map(|item| {
            let (kind, value) = item.split_once(':').expect("encoded call");
            let value = value.parse::<i32>().expect("encoded i32");
            match kind {
                "r" => Call::Run(value),
                "d" => Call::Driver(value),
                _ => panic!("unknown call kind {kind}"),
            }
        })
        .collect()
}

unsafe fn redirect_stdout(path: &Path) -> (c_int, c_int) {
    let path = CString::new(path.as_os_str().as_encoded_bytes()).expect("output path");
    let output_fd = unsafe { open(path.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, OUTPUT_MODE) };
    assert!(output_fd >= 0, "open output file");

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(
        unsafe { dup2(output_fd, STDOUT_FILENO) },
        STDOUT_FILENO,
        "redirect stdout"
    );
    (output_fd, saved_stdout)
}

unsafe fn restore_stdout(output_fd: c_int, saved_stdout: c_int) {
    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0, "flush C stdout");
    assert_eq!(
        unsafe { dup2(saved_stdout, STDOUT_FILENO) },
        STDOUT_FILENO,
        "restore stdout"
    );
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close saved stdout");
    assert_eq!(unsafe { close(output_fd) }, 0, "close output");
}

#[test]
fn ffi_differential_child() {
    let Ok(library_path) = env::var("DRIVER_DIFF_LIBRARY") else {
        return;
    };
    let calls = parse_calls(&env::var("DRIVER_DIFF_CALLS").expect("child calls"));
    let output_path = PathBuf::from(env::var_os("DRIVER_DIFF_OUTPUT").expect("child output"));

    unsafe {
        let library = Library::new(&library_path).expect("load requested shared library");
        let run: Symbol<'_, DriverFn> = library.get(b"run\0").expect("load run export");
        let driver: Symbol<'_, DriverFn> = library.get(b"driver\0").expect("load driver export");
        let (output_fd, saved_stdout) = redirect_stdout(&output_path);

        for call in calls {
            match call {
                Call::Run(value) => run(value),
                Call::Driver(value) => driver(value),
            }
        }

        restore_stdout(output_fd, saved_stdout);
    }
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_i32(&mut self) -> i32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as i32
    }
}

fn randomized_values(seed: u64, count: usize) -> Vec<i32> {
    let mut rng = FixedRng::new(seed);
    let mut values = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    values.extend((0..count).map(|_| rng.next_i32()));
    values
}

#[test]
fn config_1_run_fresh_randomized() {
    for (index, value) in randomized_values(0x4a17_9d23_641b_ef05, 32)
        .into_iter()
        .enumerate()
    {
        assert_same(&[Call::Run(value)], &format!("config-1-{index}"));
    }
}

#[test]
fn config_2_run_stateful_randomized() {
    let calls = randomized_values(0xa761_bac4_10f3_82de, 128)
        .into_iter()
        .map(Call::Run)
        .collect::<Vec<_>>();
    assert_same(&calls, "config-2");
}

#[test]
fn config_3_driver_fresh_randomized() {
    for (index, value) in randomized_values(0xf39d_2115_8ab4_670c, 32)
        .into_iter()
        .enumerate()
    {
        assert_same(&[Call::Driver(value)], &format!("config-3-{index}"));
    }
}

#[test]
fn config_4_driver_stateful_randomized() {
    let calls = randomized_values(0x9b03_5e7c_218a_d4f6, 128)
        .into_iter()
        .map(Call::Driver)
        .collect::<Vec<_>>();
    assert_same(&calls, "config-4");
}

#[test]
fn config_5_mixed_entry_points_randomized() {
    let values = randomized_values(0xc582_74a1_3fd9_06be, 128);
    let calls = values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            if index % 4 == 0 || index % 4 == 3 {
                Call::Driver(value)
            } else {
                Call::Run(value)
            }
        })
        .collect::<Vec<_>>();
    assert_same(&calls, "config-5-forward");

    let reverse_calls = calls.into_iter().rev().collect::<Vec<_>>();
    assert_same(&reverse_calls, "config-5-reverse");
}

#[test]
fn phase_c_generic_integer_boundaries() {
    for call in [
        Call::Run(i32::MIN),
        Call::Run(i32::MAX),
        Call::Driver(i32::MIN),
        Call::Driver(i32::MAX),
    ] {
        assert_same(&[call], "phase-c-int-boundary");
    }
}
