use libloading::{Library, Symbol};
use std::collections::BTreeSet;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};

type BinaryFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type GeneratedFn = unsafe extern "C" fn(c_int) -> c_int;

unsafe extern "C" {
    fn pipe(file_descriptors: *mut c_int) -> c_int;
    fn dup(file_descriptor: c_int) -> c_int;
    fn dup2(old_file_descriptor: c_int, new_file_descriptor: c_int) -> c_int;
    fn close(file_descriptor: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const RANDOM_PAIRS: usize = 96;
const RANDOM_DEFAULT_VALUES: usize = 48;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Artifacts {
    c_library: PathBuf,
    c_driver: PathBuf,
    rust_library: PathBuf,
    rust_driver: PathBuf,
}

struct Api {
    _library: Library,
    op_add: BinaryFn,
    op_sub: BinaryFn,
    op_mul: BinaryFn,
    helper_call: BinaryFn,
    helper_ptr: BinaryFn,
    use_generated: GeneratedFn,
    global_op: BinaryFn,
    global_op_name: String,
}

#[derive(Clone, Copy)]
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
        (self.0 >> 32) as u32 as i32
    }
}

fn selected_op() -> &'static str {
    #[cfg(feature = "mul")]
    {
        return "mul";
    }
    #[cfg(all(not(feature = "mul"), feature = "sub"))]
    {
        return "sub";
    }
    #[cfg(all(not(feature = "mul"), not(feature = "sub")))]
    {
        "add"
    }
}

fn selected_repeat() -> i32 {
    #[cfg(feature = "7")]
    {
        return 7;
    }
    #[cfg(all(not(feature = "7"), feature = "6"))]
    {
        return 6;
    }
    #[cfg(all(not(feature = "7"), not(feature = "6"), feature = "4"))]
    {
        return 4;
    }
    #[cfg(all(
        not(feature = "7"),
        not(feature = "6"),
        not(feature = "4"),
        feature = "3"
    ))]
    {
        return 3;
    }
    #[cfg(all(
        not(feature = "7"),
        not(feature = "6"),
        not(feature = "4"),
        not(feature = "3"),
        feature = "2"
    ))]
    {
        return 2;
    }
    #[cfg(all(
        not(feature = "7"),
        not(feature = "6"),
        not(feature = "4"),
        not(feature = "3"),
        not(feature = "2"),
        feature = "1"
    ))]
    {
        return 1;
    }
    #[cfg(all(
        not(feature = "7"),
        not(feature = "6"),
        not(feature = "4"),
        not(feature = "3"),
        not(feature = "2"),
        not(feature = "1"),
        feature = "0"
    ))]
    {
        return 0;
    }
    #[cfg(not(any(
        feature = "7",
        feature = "6",
        feature = "4",
        feature = "3",
        feature = "2",
        feature = "1",
        feature = "0"
    )))]
    {
        5
    }
}

fn artifacts() -> &'static Artifacts {
    static ARTIFACTS: OnceLock<Artifacts> = OnceLock::new();
    ARTIFACTS.get_or_init(build_c_artifacts)
}

fn rust_profile_directory() -> PathBuf {
    let test_executable = std::env::current_exe().expect("current test executable path");
    test_executable
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory")
        .to_path_buf()
}

fn build_c_artifacts() -> Artifacts {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_source = manifest.join("../c_src/src");
    let profile = rust_profile_directory();
    let fixture =
        profile
            .join("differential-c")
            .join(format!("{}-{}", selected_op(), selected_repeat()));
    fs::create_dir_all(&fixture).expect("create C fixture directory");

    let c_library = fixture.join("libdriver.so");
    let c_driver = fixture.join("driver");
    let op_define = format!("-DOP={}", selected_op());
    let repeat_define = format!("-DREPEAT={}", selected_repeat());
    let include = format!("-I{}", c_source.display());

    run_checked(
        Command::new("cc")
            .args([
                "-shared",
                "-fPIC",
                &op_define,
                &repeat_define,
                &include,
                "-o",
            ])
            .arg(&c_library)
            .arg(c_source.join("mdcore.c")),
        "compile C shared object",
    );
    run_checked(
        Command::new("cc")
            .args([&op_define, &repeat_define, &include, "-o"])
            .arg(&c_driver)
            .arg(c_source.join("mdcore.c"))
            .arg(c_source.join("mdmain.c")),
        "compile C driver",
    );

    let rust_library = profile.join(format!(
        "{}md_driver{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ));
    let rust_driver = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
    assert!(c_library.is_file(), "missing {}", c_library.display());
    assert!(c_driver.is_file(), "missing {}", c_driver.display());
    assert!(rust_library.is_file(), "missing {}", rust_library.display());
    assert!(rust_driver.is_file(), "missing {}", rust_driver.display());

    Artifacts {
        c_library,
        c_driver,
        rust_library,
        rust_driver,
    }
}

fn run_checked(command: &mut Command, description: &str) {
    let output = command.output().unwrap_or_else(|error| {
        panic!("{description}: failed to start command: {error}");
    });
    assert!(
        output.status.success(),
        "{description} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

unsafe fn load_api(path: &Path) -> Api {
    // SAFETY: The test owns the loaded library for the full lifetime of every
    // copied function pointer and validates the exact C ABI symbol types.
    let library = unsafe { Library::new(path) }.expect("load shared object");
    let op_add = unsafe { *library.get::<BinaryFn>(b"op_add\0").expect("op_add") };
    let op_sub = unsafe { *library.get::<BinaryFn>(b"op_sub\0").expect("op_sub") };
    let op_mul = unsafe { *library.get::<BinaryFn>(b"op_mul\0").expect("op_mul") };
    let helper_call = unsafe {
        *library
            .get::<BinaryFn>(b"helper_call\0")
            .expect("helper_call")
    };
    let helper_ptr = unsafe {
        *library
            .get::<BinaryFn>(b"helper_ptr\0")
            .expect("helper_ptr")
    };
    let use_generated = unsafe {
        *library
            .get::<GeneratedFn>(b"use_generated\0")
            .expect("use_generated")
    };

    let global_op_symbol: Symbol<'_, *mut BinaryFn> =
        unsafe { library.get(b"G_OP\0") }.expect("G_OP");
    let global_op = unsafe { **global_op_symbol };
    let global_name_symbol: Symbol<'_, *mut *const c_char> =
        unsafe { library.get(b"G_OP_NAME\0") }.expect("G_OP_NAME");
    let global_name_pointer = unsafe { **global_name_symbol };
    let global_op_name = unsafe { CStr::from_ptr(global_name_pointer) }
        .to_str()
        .expect("ASCII operation name")
        .to_owned();

    Api {
        _library: library,
        op_add,
        op_sub,
        op_mul,
        helper_call,
        helper_ptr,
        use_generated,
        global_op,
        global_op_name,
    }
}

fn load_apis() -> (Api, Api) {
    let paths = artifacts();
    // SAFETY: load_api checks the expected exported C ABI surface.
    unsafe { (load_api(&paths.c_library), load_api(&paths.rust_library)) }
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _lock = STDOUT_LOCK.lock().expect("stdout capture lock");
    std::io::stdout().flush().expect("flush Rust stdout");
    // SAFETY: All file descriptors are checked, stdout is restored before
    // returning, and the read descriptor is transferred exactly once to File.
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush C stdout");
        let mut descriptors = [-1, -1];
        assert_eq!(pipe(descriptors.as_mut_ptr()), 0, "create stdout pipe");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(descriptors[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );
        assert_eq!(close(descriptors[1]), 0, "close duplicate pipe writer");

        let result = operation();

        std::io::stdout()
            .flush()
            .expect("flush redirected Rust stdout");
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush redirected C stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        File::from_raw_fd(descriptors[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
        (result, output)
    }
}

fn call_with_output(function: BinaryFn, a: i32, b: i32) -> (i32, Vec<u8>) {
    capture_stdout(|| {
        // SAFETY: The symbol has the validated BinaryFn C ABI.
        unsafe { function(a, b) }
    })
}

fn generated_with_output(function: GeneratedFn, n: i32) -> (i32, Vec<u8>) {
    capture_stdout(|| {
        // SAFETY: The symbol has the validated GeneratedFn C ABI.
        unsafe { function(n) }
    })
}

fn run_driver(path: &Path, arguments: &[String]) -> Output {
    let mut command = Command::new(path);
    command.arg0("driver").args(arguments);
    command.output().expect("run driver")
}

fn assert_same_process_output(c_output: &Output, rust_output: &Output) {
    assert_eq!(
        rust_output.status.code(),
        c_output.status.code(),
        "exit status differs"
    );
    assert_eq!(rust_output.stdout, c_output.stdout, "stdout differs");
    assert_eq!(rust_output.stderr, c_output.stderr, "stderr differs");
}

#[test]
fn low_level_operation_exports_match_for_randomized_full_width_inputs() {
    let (c_api, rust_api) = load_apis();
    let function_pairs = [
        ("op_add", c_api.op_add, rust_api.op_add),
        ("op_sub", c_api.op_sub, rust_api.op_sub),
        ("op_mul", c_api.op_mul, rust_api.op_mul),
    ];
    let boundaries = [
        (i32::MIN, i32::MIN),
        (i32::MIN, -1),
        (i32::MIN, 0),
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (0, 0),
        (-1, 1),
    ];
    let mut generator = Lcg::new(0x4d44_5f4f_5053_5f31);

    for (name, c_function, rust_function) in function_pairs {
        for (a, b) in boundaries
            .into_iter()
            .chain((0..RANDOM_PAIRS).map(|_| (generator.next_i32(), generator.next_i32())))
        {
            // SAFETY: Both pointers came from symbols with the BinaryFn ABI.
            let c_result = unsafe { c_function(a, b) };
            let rust_result = unsafe { rust_function(a, b) };
            assert_eq!(
                rust_result,
                c_result,
                "{name} differs for a={a}, b={b}, op={}, repeat={}",
                selected_op(),
                selected_repeat()
            );
        }
    }
}

#[test]
fn selected_helpers_and_globals_match_for_randomized_inputs() {
    let (c_api, rust_api) = load_apis();
    assert_eq!(rust_api.global_op_name, c_api.global_op_name);
    assert_eq!(rust_api.global_op_name, selected_op());
    let mut generator = Lcg::new(0x4d44_5f48_454c_5052);

    for _ in 0..RANDOM_PAIRS {
        let a = generator.next_i32();
        let b = generator.next_i32();

        let c_call = call_with_output(c_api.helper_call, a, b);
        let rust_call = call_with_output(rust_api.helper_call, a, b);
        assert_eq!(rust_call, c_call, "helper_call differs for a={a}, b={b}");

        let c_ptr = call_with_output(c_api.helper_ptr, a, b);
        let rust_ptr = call_with_output(rust_api.helper_ptr, a, b);
        assert_eq!(rust_ptr, c_ptr, "helper_ptr differs for a={a}, b={b}");

        // SAFETY: Both global values were read as BinaryFn pointers.
        let c_global = unsafe { (c_api.global_op)(a, b) };
        let rust_global = unsafe { (rust_api.global_op)(a, b) };
        assert_eq!(rust_global, c_global, "G_OP differs for a={a}, b={b}");
    }
}

#[test]
fn generated_accumulator_matches_every_switch_shape() {
    let (c_api, rust_api) = load_apis();
    let mut values = vec![
        i32::MIN,
        -1_000_000,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        1_000_000,
        i32::MAX,
    ];
    let mut generator = Lcg::new(0x4d44_5f47_454e_5f31);
    for _ in 0..RANDOM_DEFAULT_VALUES {
        let value = generator.next_i32();
        if !(0..=6).contains(&value) {
            values.push(value);
        }
    }

    for n in values {
        let c_result = generated_with_output(c_api.use_generated, n);
        let rust_result = generated_with_output(rust_api.use_generated, n);
        assert_eq!(rust_result, c_result, "use_generated differs for n={n}");
    }
}

#[test]
fn valid_driver_stdout_is_byte_identical() {
    let paths = artifacts();
    let mut generator = Lcg::new(0x4d44_5f44_5256_5f31);
    let boundary_pairs = [
        (0, 0),
        (1, -1),
        (i32::MIN, 0),
        (i32::MAX, 0),
        (46_340, 46_340),
        (-46_340, 46_340),
    ];

    for (a, b) in boundary_pairs
        .into_iter()
        .chain((0..RANDOM_PAIRS).map(|_| (generator.next_i32(), generator.next_i32())))
    {
        let arguments = [a.to_string(), b.to_string()];
        let c_output = run_driver(&paths.c_driver, &arguments);
        let rust_output = run_driver(&paths.rust_driver, &arguments);
        assert_same_process_output(&c_output, &rust_output);
    }
}

#[test]
fn insufficient_arguments_match_exactly() {
    let paths = artifacts();
    for arguments in [vec![], vec!["123".to_owned()]] {
        let c_output = run_driver(&paths.c_driver, &arguments);
        let rust_output = run_driver(&paths.rust_driver, &arguments);
        assert_same_process_output(&c_output, &rust_output);
        assert_eq!(c_output.status.code(), Some(2));
        assert_eq!(c_output.stdout, b"");
        assert_eq!(c_output.stderr, b"usage: driver A B\n");
    }
}

fn defined_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(output.status.success(), "nm failed");
    String::from_utf8(output.stdout)
        .expect("nm UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

#[test]
fn dynamic_symbol_surface_is_complete() {
    let paths = artifacts();
    let c_symbols = defined_symbols(&paths.c_library);
    let rust_symbols = defined_symbols(&paths.rust_library);
    let missing: Vec<_> = c_symbols.difference(&rust_symbols).cloned().collect();
    assert!(missing.is_empty(), "Rust is missing C symbols: {missing:?}");

    let output = Command::new("ldd")
        .arg("-r")
        .arg(&paths.rust_library)
        .output()
        .expect("run ldd -r");
    assert!(output.status.success(), "ldd -r failed");
    let mut diagnostics = output.stdout;
    diagnostics.extend_from_slice(&output.stderr);
    assert!(
        !String::from_utf8_lossy(&diagnostics).contains("undefined symbol"),
        "unresolved Rust symbols:\n{}",
        String::from_utf8_lossy(&diagnostics)
    );
}
