use libloading::{Library, Symbol};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::{File, OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

type BinaryFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type GeneratedFn = unsafe extern "C" fn(c_int) -> c_int;
type MainFn = unsafe extern "C" fn(c_int, *const *const c_char) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Add,
    Sub,
    Mul,
}

impl Operation {
    fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
        }
    }
}

fn configured_operation() -> Operation {
    if cfg!(feature = "sub") {
        Operation::Sub
    } else if cfg!(feature = "mul") {
        Operation::Mul
    } else {
        Operation::Add
    }
}

fn configured_repeat() -> c_int {
    if cfg!(feature = "0") {
        0
    } else if cfg!(feature = "1") {
        1
    } else if cfg!(feature = "2") {
        2
    } else if cfg!(feature = "3") {
        3
    } else if cfg!(feature = "4") {
        4
    } else if cfg!(feature = "6") {
        6
    } else if cfg!(feature = "7") {
        7
    } else {
        5
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_REFERENCE_LIB")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("c_src/build/libdriver_c.so"))
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_TRANSLATION_LIB")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("target/debug/libdriver.so"))
}

static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

fn capture_path(label: &str) -> PathBuf {
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-ffi-{}-{}-{}",
        std::process::id(),
        id,
        label
    ))
}

fn open_capture(path: &Path) -> File {
    OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path)
        .unwrap()
}

fn capture_stdio<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>, Vec<u8>) {
    let stdout_path = capture_path("stdout");
    let stderr_path = capture_path("stderr");
    let mut stdout_file = open_capture(&stdout_path);
    let mut stderr_file = open_capture(&stderr_path);

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    let saved_stdout = unsafe { dup(1) };
    let saved_stderr = unsafe { dup(2) };
    assert!(saved_stdout >= 0 && saved_stderr >= 0);
    assert_eq!(unsafe { dup2(stdout_file.as_raw_fd(), 1) }, 1);
    assert_eq!(unsafe { dup2(stderr_file.as_raw_fd(), 2) }, 2);

    let result = call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { dup2(saved_stderr, 2) }, 2);
    unsafe {
        close(saved_stdout);
        close(saved_stderr);
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    stdout_file.seek(SeekFrom::Start(0)).unwrap();
    stderr_file.seek(SeekFrom::Start(0)).unwrap();
    stdout_file.read_to_end(&mut stdout).unwrap();
    stderr_file.read_to_end(&mut stderr).unwrap();
    drop(stdout_file);
    drop(stderr_file);
    remove_file(stdout_path).unwrap();
    remove_file(stderr_path).unwrap();
    (result, stdout, stderr)
}

fn integer_pairs() -> Vec<(c_int, c_int)> {
    let mut pairs = vec![
        (0, 0),
        (c_int::MIN, 0),
        (c_int::MAX, 0),
        (c_int::MAX, 1),
        (c_int::MIN, -1),
        (c_int::MIN, c_int::MAX),
        (c_int::MAX, c_int::MAX),
        (-1, -1),
        (46_341, 46_341),
        (-46_341, 46_341),
    ];
    let mut state = 0x6a09_e667_f3bc_c909_u64;
    for _ in 0..256 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let a = state as u32 as c_int;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let b = state as u32 as c_int;
        pairs.push((a, b));
    }
    pairs
}

unsafe fn load_binary<'a>(library: &'a Library, name: &[u8]) -> Symbol<'a, BinaryFn> {
    unsafe { library.get(name).unwrap() }
}

fn compare_binary(
    label: &str,
    c_fn: BinaryFn,
    rust_fn: BinaryFn,
    cases: &[(c_int, c_int)],
    emits_output: bool,
) {
    let run = |function: BinaryFn| {
        capture_stdio(|| {
            cases
                .iter()
                .map(|&(a, b)| unsafe { function(a, b) })
                .collect::<Vec<_>>()
        })
    };
    let (c_results, c_stdout, c_stderr) = run(c_fn);
    let (rust_results, rust_stdout, rust_stderr) = run(rust_fn);
    assert_eq!(rust_results, c_results, "{label} return values");
    assert_eq!(rust_stdout, c_stdout, "{label} stdout");
    assert_eq!(rust_stderr, c_stderr, "{label} stderr");
    if emits_output {
        assert!(!c_stdout.is_empty(), "{label} should emit stdout");
    } else {
        assert!(c_stdout.is_empty(), "{label} unexpectedly emitted stdout");
    }
}

fn compare_generated(c_fn: GeneratedFn, rust_fn: GeneratedFn) {
    let mut cases = vec![c_int::MIN, -10_000, -1];
    cases.extend(0..=7);
    cases.extend([8, 10_000, c_int::MAX]);
    let run = |function: GeneratedFn| {
        capture_stdio(|| {
            cases
                .iter()
                .map(|&n| unsafe { function(n) })
                .collect::<Vec<_>>()
        })
    };
    let (c_results, c_stdout, c_stderr) = run(c_fn);
    let (rust_results, rust_stdout, rust_stderr) = run(rust_fn);
    assert_eq!(rust_results, c_results, "use_generated return values");
    assert_eq!(rust_stdout, c_stdout, "use_generated stdout");
    assert_eq!(rust_stderr, c_stderr, "use_generated stderr");
}

fn main_inputs() -> Vec<(CString, CString, bool)> {
    let mut inputs = vec![
        (
            CString::new("0").unwrap(),
            CString::new("0").unwrap(),
            false,
        ),
        (
            CString::new(" -17").unwrap(),
            CString::new("+42").unwrap(),
            true,
        ),
        (
            CString::new("123tail").unwrap(),
            CString::new("7x").unwrap(),
            false,
        ),
        (CString::new("").unwrap(), CString::new("-0").unwrap(), true),
    ];
    for (index, (a, b)) in integer_pairs().into_iter().take(128).enumerate() {
        inputs.push((
            CString::new(a.to_string()).unwrap(),
            CString::new(b.to_string()).unwrap(),
            index % 2 == 0,
        ));
    }
    inputs
}

fn run_mains(function: MainFn) -> (Vec<c_int>, Vec<u8>, Vec<u8>) {
    let program = CString::new("driver").unwrap();
    let extra = CString::new("ignored").unwrap();
    capture_stdio(|| {
        main_inputs()
            .iter()
            .map(|(a, b, has_extra)| {
                let mut argv = vec![program.as_ptr(), a.as_ptr(), b.as_ptr()];
                if *has_extra {
                    argv.push(extra.as_ptr());
                }
                argv.push(std::ptr::null());
                unsafe { function((argv.len() - 1) as c_int, argv.as_ptr()) }
            })
            .collect()
    })
}

fn compare_valid_main(c_main: MainFn, rust_main: MainFn) {
    let (c_results, c_stdout, c_stderr) = run_mains(c_main);
    let (rust_results, rust_stdout, rust_stderr) = run_mains(rust_main);
    assert_eq!(rust_results, c_results, "main return values");
    assert_eq!(rust_stdout, c_stdout, "main stdout");
    assert_eq!(rust_stderr, c_stderr, "main stderr");
    assert!(c_results.iter().all(|&result| result == 0));
}

fn compare_main_rejection(c_main: MainFn, rust_main: MainFn) {
    let program = CString::new("driver").unwrap();
    let argv = [program.as_ptr(), std::ptr::null()];
    let run = |function: MainFn| {
        capture_stdio(|| {
            [c_int::MIN, -1, 0, 1, 2]
                .into_iter()
                .map(|argc| unsafe { function(argc, argv.as_ptr()) })
                .collect::<Vec<_>>()
        })
    };
    let (c_results, c_stdout, c_stderr) = run(c_main);
    let (rust_results, rust_stdout, rust_stderr) = run(rust_main);
    assert_eq!(rust_results, c_results, "main rejection return values");
    assert_eq!(rust_stdout, c_stdout, "main rejection stdout");
    assert_eq!(rust_stderr, c_stderr, "main rejection stderr");
    assert!(c_results.iter().all(|&result| result == 2));
    assert_eq!(
        c_stderr,
        b"usage: driver A B\n".repeat(c_results.len()),
        "main rejection diagnostic"
    );
}

#[test]
fn differential_surface() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    unsafe {
        let c = Library::new(&c_path).unwrap();
        let rust = Library::new(&rust_path).unwrap();
        let cases = integer_pairs();

        for symbol in [b"op_add\0".as_slice(), b"op_sub\0", b"op_mul\0"] {
            let c_fn = load_binary(&c, symbol);
            let rust_fn = load_binary(&rust, symbol);
            compare_binary(
                CStr::from_bytes_with_nul(symbol).unwrap().to_str().unwrap(),
                *c_fn,
                *rust_fn,
                &cases,
                false,
            );
        }

        let c_name: Symbol<*mut *const c_char> = c.get(b"G_OP_NAME\0").unwrap();
        let rust_name: Symbol<*mut *const c_char> = rust.get(b"G_OP_NAME\0").unwrap();
        let c_name = CStr::from_ptr(**c_name).to_bytes();
        let rust_name = CStr::from_ptr(**rust_name).to_bytes();
        assert_eq!(rust_name, c_name, "G_OP_NAME bytes");
        assert_eq!(c_name, configured_operation().name().as_bytes());

        let c_global: Symbol<*mut BinaryFn> = c.get(b"G_OP\0").unwrap();
        let rust_global: Symbol<*mut BinaryFn> = rust.get(b"G_OP\0").unwrap();
        compare_binary("G_OP", **c_global, **rust_global, &cases, false);

        let c_helper = load_binary(&c, b"helper_call\0");
        let rust_helper = load_binary(&rust, b"helper_call\0");
        compare_binary(
            &format!(
                "helper_call OP={} REPEAT={}",
                configured_operation().name(),
                configured_repeat()
            ),
            *c_helper,
            *rust_helper,
            &cases,
            true,
        );

        let c_helper_ptr = load_binary(&c, b"helper_ptr\0");
        let rust_helper_ptr = load_binary(&rust, b"helper_ptr\0");
        compare_binary("helper_ptr", *c_helper_ptr, *rust_helper_ptr, &cases, true);

        let c_generated: Symbol<GeneratedFn> = c.get(b"use_generated\0").unwrap();
        let rust_generated: Symbol<GeneratedFn> = rust.get(b"use_generated\0").unwrap();
        compare_generated(*c_generated, *rust_generated);

        let c_main: Symbol<MainFn> = c.get(b"main\0").unwrap();
        let rust_main: Symbol<MainFn> = rust.get(b"main\0").unwrap();
        compare_valid_main(*c_main, *rust_main);
        compare_main_rejection(*c_main, *rust_main);
    }

    for crash_case in ["null_argv", "null_argv1"] {
        let c_status = run_crash_probe("c", crash_case);
        let rust_status = run_crash_probe("rust", crash_case);
        assert_eq!(c_status.signal(), Some(11), "C {crash_case}: {c_status}");
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "Rust {crash_case}: {rust_status}"
        );
    }
}

fn run_crash_probe(library: &str, crash_case: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_probe_worker", "--nocapture"])
        .env("CRASH_PROBE_LIBRARY", library)
        .env("CRASH_PROBE_CASE", crash_case)
        .env("C_REFERENCE_LIB", c_library_path())
        .env("RUST_TRANSLATION_LIB", rust_library_path())
        .status()
        .unwrap()
}

#[test]
fn crash_probe_worker() {
    let Some(library_name) = std::env::var_os("CRASH_PROBE_LIBRARY") else {
        return;
    };
    let path = if library_name == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let crash_case = std::env::var("CRASH_PROBE_CASE").unwrap();

    unsafe {
        let library = Library::new(path).unwrap();
        let main: Symbol<MainFn> = library.get(b"main\0").unwrap();
        if crash_case == "null_argv" {
            main(0, std::ptr::null());
        } else {
            let program = CString::new("driver").unwrap();
            let value = CString::new("1").unwrap();
            let argv = [program.as_ptr(), std::ptr::null(), value.as_ptr()];
            main(3, argv.as_ptr());
        }
    }
    panic!("crash probe unexpectedly returned");
}
