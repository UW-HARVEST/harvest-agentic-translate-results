use libloading::Library;
use std::collections::BTreeSet;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;
use std::sync::Mutex;

type BinaryFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type UnaryFn = unsafe extern "C" fn(c_int) -> c_int;
type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

static PROCESS_IO_LOCK: Mutex<()> = Mutex::new(());

#[cfg(feature = "mul")]
const OP: &str = "mul";
#[cfg(all(not(feature = "mul"), feature = "sub"))]
const OP: &str = "sub";
#[cfg(all(not(feature = "mul"), not(feature = "sub"), feature = "add"))]
const OP: &str = "add";

#[cfg(feature = "7")]
const REPEAT: &str = "7";
#[cfg(all(not(feature = "7"), feature = "6"))]
const REPEAT: &str = "6";
#[cfg(all(not(feature = "7"), not(feature = "6"), feature = "5"))]
const REPEAT: &str = "5";
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    feature = "4"
))]
const REPEAT: &str = "4";
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    feature = "3"
))]
const REPEAT: &str = "3";
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    feature = "2"
))]
const REPEAT: &str = "2";
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2"),
    feature = "1"
))]
const REPEAT: &str = "1";
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2"),
    not(feature = "1"),
    feature = "0"
))]
const REPEAT: &str = "0";

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
}

struct Api {
    _library: Library,
    op_add: BinaryFn,
    op_sub: BinaryFn,
    op_mul: BinaryFn,
    helper_call: BinaryFn,
    helper_ptr: BinaryFn,
    use_generated: UnaryFn,
    driver_main: MainFn,
    global_op: BinaryFn,
    global_name: *const c_char,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        // SAFETY: The test controls the library paths and copies every symbol
        // value while retaining the owning Library for the lifetime of Api.
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        // SAFETY: All symbol types are derived directly from the C declarations.
        let op_add = unsafe { *library.get::<BinaryFn>(b"op_add\0").unwrap() };
        let op_sub = unsafe { *library.get::<BinaryFn>(b"op_sub\0").unwrap() };
        let op_mul = unsafe { *library.get::<BinaryFn>(b"op_mul\0").unwrap() };
        let helper_call = unsafe { *library.get::<BinaryFn>(b"helper_call\0").unwrap() };
        let helper_ptr = unsafe { *library.get::<BinaryFn>(b"helper_ptr\0").unwrap() };
        let use_generated = unsafe { *library.get::<UnaryFn>(b"use_generated\0").unwrap() };
        let driver_main = unsafe { *library.get::<MainFn>(b"main\0").unwrap() };

        // Data symbols returned by dlsym point to the exported storage. One
        // extra dereference reads the function/string pointer stored there.
        let global_op = unsafe { **library.get::<*mut BinaryFn>(b"G_OP\0").unwrap() };
        let global_name = unsafe { **library.get::<*mut *const c_char>(b"G_OP_NAME\0").unwrap() };

        Self {
            _library: library,
            op_add,
            op_sub,
            op_mul,
            helper_call,
            helper_ptr,
            use_generated,
            driver_main,
            global_op,
            global_name,
        }
    }
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_i32(&mut self) -> i32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as i32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    PathBuf::from(env!("MD_RUST_DYLIB"))
}

fn c_library_path() -> PathBuf {
    let output_dir = manifest_dir()
        .join("target")
        .join("c-reference")
        .join(format!("{OP}-{REPEAT}"));
    fs::create_dir_all(&output_dir).expect("create C reference output directory");
    let library = output_dir.join("libdriver.so");
    let c_root = manifest_dir().parent().unwrap().join("c_src");

    let output = Command::new("timeout")
        .arg("600")
        .arg("cc")
        .arg("-shared")
        .arg("-fPIC")
        .arg(format!("-DOP={OP}"))
        .arg(format!("-DREPEAT={REPEAT}"))
        .arg(c_root.join("src/mdcore.c"))
        .arg(c_root.join("src/mdmain.c"))
        .arg("-o")
        .arg(&library)
        .output()
        .expect("run C compiler");
    assert_command_success("compile C reference", &output);
    library
}

fn assert_command_success(action: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{action} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn randomized_pairs() -> Vec<(i32, i32)> {
    let mut pairs = vec![
        (0, 0),
        (1, -1),
        (-1, 1),
        (i32::MIN, 0),
        (i32::MAX, 0),
        (0, i32::MIN),
        (0, i32::MAX),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (i32::MIN, -1),
        (i32::MAX, 2),
    ];
    let mut rng = Lcg::new(0x5eed_d1ff_e12a_2025);
    for _ in 0..256 {
        pairs.push((rng.next_i32(), rng.next_i32()));
    }
    pairs
}

fn generated_inputs() -> Vec<i32> {
    let mut values = Vec::new();
    for n in 0..=6 {
        values.extend(std::iter::repeat_n(n, 32));
    }
    values.extend([i32::MIN, -1, 7, 8, i32::MAX]);

    let mut rng = Lcg::new(0xd15f_a7c4_600d_0001);
    while values.len() < 512 {
        let value = rng.next_i32();
        if !(0..=6).contains(&value) {
            values.push(value);
        }
    }
    values
}

fn capture_stdio<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>, Vec<u8>) {
    // SAFETY: File descriptors are validated after every syscall, restored
    // before reading, and each read end is owned by exactly one File.
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let mut stdout_pipe = [0; 2];
        let mut stderr_pipe = [0; 2];
        assert_eq!(pipe(stdout_pipe.as_mut_ptr()), 0);
        assert_eq!(pipe(stderr_pipe.as_mut_ptr()), 0);

        let saved_stdout = dup(1);
        let saved_stderr = dup(2);
        assert!(saved_stdout >= 0);
        assert!(saved_stderr >= 0);
        assert_eq!(dup2(stdout_pipe[1], 1), 1);
        assert_eq!(dup2(stderr_pipe[1], 2), 2);
        assert_eq!(close(stdout_pipe[1]), 0);
        assert_eq!(close(stderr_pipe[1]), 0);

        let result = call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(dup2(saved_stderr, 2), 2);
        assert_eq!(close(saved_stdout), 0);
        assert_eq!(close(saved_stderr), 0);

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        File::from_raw_fd(stdout_pipe[0])
            .read_to_end(&mut stdout)
            .unwrap();
        File::from_raw_fd(stderr_pipe[0])
            .read_to_end(&mut stderr)
            .unwrap();
        (result, stdout, stderr)
    }
}

fn call_binary_batch(function: BinaryFn, pairs: &[(i32, i32)]) -> (Vec<i32>, Vec<u8>) {
    let (returns, stdout, stderr) = capture_stdio(|| {
        pairs
            .iter()
            .map(|&(a, b)| {
                // SAFETY: Both arguments are C ints and the loaded symbol has
                // the matching C ABI.
                unsafe { function(a, b) }
            })
            .collect()
    });
    assert!(stderr.is_empty(), "unexpected stderr: {stderr:?}");
    (returns, stdout)
}

fn call_unary_batch(function: UnaryFn, values: &[i32]) -> (Vec<i32>, Vec<u8>) {
    let (returns, stdout, stderr) = capture_stdio(|| {
        values
            .iter()
            .map(|&value| {
                // SAFETY: The argument is a C int and the loaded symbol has
                // the matching C ABI.
                unsafe { function(value) }
            })
            .collect()
    });
    assert!(stderr.is_empty(), "unexpected stderr: {stderr:?}");
    (returns, stdout)
}

fn call_main_numeric_batch(function: MainFn, pairs: &[(i32, i32)]) -> (Vec<i32>, Vec<u8>, Vec<u8>) {
    capture_stdio(|| {
        pairs
            .iter()
            .enumerate()
            .map(|(index, &(a, b))| {
                let program = CString::new("driver").unwrap();
                let a = CString::new(a.to_string()).unwrap();
                let b = CString::new(b.to_string()).unwrap();
                let extra = CString::new("ignored").unwrap();
                let mut argv = vec![
                    program.as_ptr() as *mut c_char,
                    a.as_ptr() as *mut c_char,
                    b.as_ptr() as *mut c_char,
                ];
                let argc = if index % 2 == 0 {
                    argv.push(extra.as_ptr() as *mut c_char);
                    4
                } else {
                    3
                };
                argv.push(ptr::null_mut());

                // SAFETY: Every consumed entry points to a live CString.
                unsafe { function(argc, argv.as_mut_ptr()) }
            })
            .collect()
    })
}

fn call_main_error(function: MainFn, argc: i32, argv0: Option<&str>) -> (i32, Vec<u8>, Vec<u8>) {
    let program = argv0.map(|value| CString::new(value).unwrap());
    let mut argv = vec![
        program
            .as_ref()
            .map_or(ptr::null_mut(), |value| value.as_ptr() as *mut c_char),
        ptr::null_mut(),
        ptr::null_mut(),
    ];
    capture_stdio(|| {
        // SAFETY: argv has at least one readable entry, matching the only
        // pointer access in the argc < 3 branch.
        unsafe { function(argc, argv.as_mut_ptr()) }
    })
}

fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("timeout")
        .arg("600")
        .arg("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert_command_success("inspect dynamic symbols", &output);
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2))
        .map(str::to_owned)
        .collect()
}

#[test]
fn valid_surface_matches_through_both_shared_libraries() {
    let _io_guard = PROCESS_IO_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(rust_path.is_file(), "missing {}", rust_path.display());

    // SAFETY: Both paths are freshly built shared objects with the audited API.
    let (c, rust) = unsafe { (Api::load(&c_path), Api::load(&rust_path)) };
    let pairs = randomized_pairs();

    for (name, c_function, rust_function) in [
        ("op_add", c.op_add, rust.op_add),
        ("op_sub", c.op_sub, rust.op_sub),
        ("op_mul", c.op_mul, rust.op_mul),
    ] {
        for &(a, b) in &pairs {
            // SAFETY: Loaded signatures match the C declarations.
            let (c_result, rust_result) = unsafe { (c_function(a, b), rust_function(a, b)) };
            assert_eq!(
                c_result, rust_result,
                "{name} diverged for ({a}, {b}) under {OP},{REPEAT}"
            );
        }
    }

    let c_name = unsafe { CStr::from_ptr(c.global_name) }.to_bytes();
    let rust_name = unsafe { CStr::from_ptr(rust.global_name) }.to_bytes();
    assert_eq!(c_name, rust_name);
    assert_eq!(c_name, OP.as_bytes());
    for &(a, b) in &pairs {
        let (c_result, rust_result) = unsafe { ((c.global_op)(a, b), (rust.global_op)(a, b)) };
        assert_eq!(c_result, rust_result, "G_OP diverged for ({a}, {b})");
    }

    let c_helper_ptr = call_binary_batch(c.helper_ptr, &pairs);
    let rust_helper_ptr = call_binary_batch(rust.helper_ptr, &pairs);
    assert_eq!(c_helper_ptr, rust_helper_ptr, "helper_ptr diverged");

    let c_helper_call = call_binary_batch(c.helper_call, &pairs);
    let rust_helper_call = call_binary_batch(rust.helper_call, &pairs);
    assert_eq!(c_helper_call, rust_helper_call, "helper_call diverged");

    let generated_inputs = generated_inputs();
    let c_generated = call_unary_batch(c.use_generated, &generated_inputs);
    let rust_generated = call_unary_batch(rust.use_generated, &generated_inputs);
    assert_eq!(c_generated, rust_generated, "use_generated diverged");

    let main_pairs = &pairs[..96];
    let c_main = call_main_numeric_batch(c.driver_main, main_pairs);
    let rust_main = call_main_numeric_batch(rust.driver_main, main_pairs);
    assert_eq!(c_main, rust_main, "main diverged");

    // argc is only tested for < 3; extra/oversized counts are ignored after
    // argv[1] and argv[2], so this is a valid generic boundary.
    let program = CString::new("driver").unwrap();
    let one = CString::new("1").unwrap();
    let two = CString::new("2").unwrap();
    let mut argv = vec![
        program.as_ptr() as *mut c_char,
        one.as_ptr() as *mut c_char,
        two.as_ptr() as *mut c_char,
    ];
    let c_oversized = capture_stdio(|| unsafe { (c.driver_main)(i32::MAX, argv.as_mut_ptr()) });
    let rust_oversized =
        capture_stdio(|| unsafe { (rust.driver_main)(i32::MAX, argv.as_mut_ptr()) });
    assert_eq!(c_oversized, rust_oversized, "oversized argc diverged");
}

#[test]
fn explicit_and_generic_error_surface_matches() {
    let _io_guard = PROCESS_IO_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let c_path = c_library_path();
    let rust_path = rust_library_path();

    // SAFETY: Both paths are freshly built shared objects with the audited API.
    let (c, rust) = unsafe { (Api::load(&c_path), Api::load(&rust_path)) };

    for argc in [0, 1, 2] {
        let c_result = call_main_error(c.driver_main, argc, Some("driver"));
        let rust_result = call_main_error(rust.driver_main, argc, Some("driver"));
        assert_eq!(c_result, rust_result, "argc={argc} rejection diverged");
        assert_eq!(c_result.0, 2);
        assert_eq!(c_result.2, b"usage: driver A B\n");
    }

    // glibc accepts a null `%s` argument and renders it as `(null)`. This is
    // not a C source rejection, but it is a generic null-entry FFI boundary.
    let c_null_name = call_main_error(c.driver_main, 2, None);
    let rust_null_name = call_main_error(rust.driver_main, 2, None);
    assert_eq!(c_null_name, rust_null_name, "null argv[0] diverged");

    for scenario in ["argv-null", "arg1-null", "arg2-null"] {
        let c_status = run_null_pointer_probe(&c_path, scenario);
        let rust_status = run_null_pointer_probe(&rust_path, scenario);
        assert_eq!(
            (c_status.status.code(), c_status.status.signal()),
            (rust_status.status.code(), rust_status.status.signal()),
            "{scenario} termination diverged\nC stderr:\n{}\nRust stderr:\n{}",
            String::from_utf8_lossy(&c_status.stderr),
            String::from_utf8_lossy(&rust_status.stderr)
        );
        assert!(
            !c_status.status.success(),
            "{scenario} unexpectedly returned successfully"
        );
    }
}

#[test]
fn dynamic_symbol_surface_matches() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    let c_symbols = defined_dynamic_symbols(&c_path);
    let rust_symbols = defined_dynamic_symbols(&rust_path);
    let missing: Vec<_> = c_symbols.difference(&rust_symbols).cloned().collect();
    assert!(missing.is_empty(), "Rust is missing C symbols: {missing:?}");

    let expected: BTreeSet<_> = [
        "G_OP",
        "G_OP_NAME",
        "helper_call",
        "helper_ptr",
        "main",
        "op_add",
        "op_mul",
        "op_sub",
        "use_generated",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(c_symbols, expected, "C symbol artifact is stale");
}

fn run_null_pointer_probe(library: &Path, scenario: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("null_pointer_probe")
        .arg("--nocapture")
        .env("MD_NULL_LIBRARY", library)
        .env("MD_NULL_SCENARIO", scenario)
        .output()
        .expect("run null-pointer child")
}

#[test]
fn null_pointer_probe() {
    let Some(library) = std::env::var_os("MD_NULL_LIBRARY") else {
        return;
    };
    let scenario = std::env::var("MD_NULL_SCENARIO").unwrap();

    // SAFETY: This test intentionally exercises invalid C pointer inputs in an
    // isolated process so the parent can compare the exact termination mode.
    let api = unsafe { Api::load(Path::new(&library)) };
    let program = CString::new("driver").unwrap();
    let one = CString::new("1").unwrap();
    let two = CString::new("2").unwrap();

    unsafe {
        match scenario.as_str() {
            "argv-null" => {
                (api.driver_main)(0, ptr::null_mut());
            }
            "arg1-null" => {
                let mut argv = [
                    program.as_ptr() as *mut c_char,
                    ptr::null_mut(),
                    two.as_ptr() as *mut c_char,
                ];
                (api.driver_main)(3, argv.as_mut_ptr());
            }
            "arg2-null" => {
                let mut argv = [
                    program.as_ptr() as *mut c_char,
                    one.as_ptr() as *mut c_char,
                    ptr::null_mut(),
                ];
                (api.driver_main)(3, argv.as_mut_ptr());
            }
            _ => panic!("unknown scenario {scenario}"),
        }
    }

    std::process::exit(99);
}
