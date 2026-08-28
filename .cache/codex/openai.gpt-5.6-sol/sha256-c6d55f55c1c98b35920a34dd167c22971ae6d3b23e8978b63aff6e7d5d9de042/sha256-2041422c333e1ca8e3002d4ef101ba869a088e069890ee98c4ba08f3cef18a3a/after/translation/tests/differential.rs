use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

use libloading::{Library, Symbol};

type BinaryFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type UnaryFn = unsafe extern "C" fn(c_int) -> c_int;
type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn selected_operation() -> &'static str {
    if cfg!(feature = "mul") {
        "mul"
    } else if cfg!(feature = "sub") {
        "sub"
    } else {
        "add"
    }
}

fn selected_repeat() -> c_int {
    if cfg!(feature = "7") {
        7
    } else if cfg!(feature = "6") {
        6
    } else if cfg!(feature = "4") {
        4
    } else if cfg!(feature = "3") {
        3
    } else if cfg!(feature = "2") {
        2
    } else if cfg!(feature = "1") {
        1
    } else if cfg!(feature = "0") {
        0
    } else {
        5
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join(format!(
        "../c_src/build/shared/libmacrodepth_{}_{}.so",
        selected_operation(),
        selected_repeat()
    ))
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libmacrodepth_add_5.so")
}

fn load_pair() -> (Library, Library) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}; run cargo build --release first",
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

fn capture<R>(call: impl FnOnce() -> R) -> (R, Vec<u8>, Vec<u8>) {
    let _guard = CAPTURE_LOCK.lock().expect("capture lock poisoned");
    let mut stdout_pipe = [0; 2];
    let mut stderr_pipe = [0; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(stdout_pipe.as_mut_ptr()), 0);
        assert_eq!(pipe(stderr_pipe.as_mut_ptr()), 0);
    }

    let saved_stdout = unsafe { dup(1) };
    let saved_stderr = unsafe { dup(2) };
    assert!(saved_stdout >= 0 && saved_stderr >= 0);

    unsafe {
        assert_eq!(dup2(stdout_pipe[1], 1), 1);
        assert_eq!(dup2(stderr_pipe[1], 2), 2);
        close(stdout_pipe[1]);
        close(stderr_pipe[1]);
    }

    let result = call();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(dup2(saved_stderr, 2), 2);
        close(saved_stdout);
        close(saved_stderr);
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    unsafe {
        File::from_raw_fd(stdout_pipe[0])
            .read_to_end(&mut stdout)
            .expect("read captured stdout");
        File::from_raw_fd(stderr_pipe[0])
            .read_to_end(&mut stderr)
            .expect("read captured stderr");
    }
    (result, stdout, stderr)
}

fn compare_call<R>(c_call: impl FnOnce() -> R, rust_call: impl FnOnce() -> R)
where
    R: std::fmt::Debug + PartialEq,
{
    let c = capture(c_call);
    let rust = capture(rust_call);
    assert_eq!(c, rust);
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn bounded_i32(&mut self, magnitude: i32) -> i32 {
        (self.next_u32() % (2 * magnitude as u32 + 1)) as i32 - magnitude
    }
}

unsafe fn symbol<'library, T>(library: &'library Library, name: &[u8]) -> Symbol<'library, T> {
    unsafe {
        library.get(name).unwrap_or_else(|error| {
            panic!("missing {:?}: {error}", CStr::from_bytes_with_nul(name))
        })
    }
}

fn argv(values: &[String]) -> (Vec<CString>, Vec<*mut c_char>) {
    let strings: Vec<_> = values
        .iter()
        .map(|value| CString::new(value.as_str()).expect("argument contains NUL"))
        .collect();
    let mut pointers: Vec<_> = strings
        .iter()
        .map(|value| value.as_ptr().cast_mut())
        .collect();
    pointers.push(ptr::null_mut());
    (strings, pointers)
}

#[test]
fn dynamic_symbol_surface_is_loadable() {
    let (c, rust) = load_pair();
    for name in [
        b"G_OP\0".as_slice(),
        b"G_OP_NAME\0",
        b"helper_call\0",
        b"helper_ptr\0",
        b"main\0",
        b"op_add\0",
        b"op_mul\0",
        b"op_sub\0",
        b"use_generated\0",
    ] {
        unsafe {
            let _: Symbol<*mut c_void> = symbol(&c, name);
            let _: Symbol<*mut c_void> = symbol(&rust, name);
        }
    }
}

#[test]
fn operation_family_matches_randomized() {
    let (c, rust) = load_pair();
    let mut rng = FixedRng::new(0x6d64_6f70_735f_3031);

    for name in [b"op_add\0".as_slice(), b"op_sub\0", b"op_mul\0"] {
        let c_fn: Symbol<BinaryFn> = unsafe { symbol(&c, name) };
        let rust_fn: Symbol<BinaryFn> = unsafe { symbol(&rust, name) };

        for &(a, b) in &[
            (0, 0),
            (i32::MAX, 0),
            (i32::MIN, 0),
            (i32::MAX, -1),
            (i32::MIN, 1),
        ] {
            assert_eq!(unsafe { c_fn(a, b) }, unsafe { rust_fn(a, b) });
        }
        for _ in 0..512 {
            let a = rng.bounded_i32(20_000);
            let b = rng.bounded_i32(20_000);
            assert_eq!(unsafe { c_fn(a, b) }, unsafe { rust_fn(a, b) });
        }
    }
}

#[test]
fn selected_helpers_and_globals_match_randomized() {
    let (c, rust) = load_pair();
    let c_helper_call: Symbol<BinaryFn> = unsafe { symbol(&c, b"helper_call\0") };
    let rust_helper_call: Symbol<BinaryFn> = unsafe { symbol(&rust, b"helper_call\0") };
    let c_helper_ptr: Symbol<BinaryFn> = unsafe { symbol(&c, b"helper_ptr\0") };
    let rust_helper_ptr: Symbol<BinaryFn> = unsafe { symbol(&rust, b"helper_ptr\0") };
    let c_global: Symbol<*mut BinaryFn> = unsafe { symbol(&c, b"G_OP\0") };
    let rust_global: Symbol<*mut BinaryFn> = unsafe { symbol(&rust, b"G_OP\0") };
    let c_name: Symbol<*mut *const c_char> = unsafe { symbol(&c, b"G_OP_NAME\0") };
    let rust_name: Symbol<*mut *const c_char> = unsafe { symbol(&rust, b"G_OP_NAME\0") };

    let c_global_fn = unsafe { **c_global };
    let rust_global_fn = unsafe { **rust_global };
    let c_name = unsafe { CStr::from_ptr(**c_name) };
    let rust_name = unsafe { CStr::from_ptr(**rust_name) };
    assert_eq!(c_name.to_bytes_with_nul(), rust_name.to_bytes_with_nul());
    assert_eq!(c_name.to_bytes(), selected_operation().as_bytes());

    let mut rng = FixedRng::new(0x6865_6c70_6572_3032);
    for _ in 0..256 {
        let a = rng.bounded_i32(10_000);
        let b = rng.bounded_i32(10_000);
        compare_call(
            || unsafe { c_helper_call(a, b) },
            || unsafe { rust_helper_call(a, b) },
        );
        compare_call(
            || unsafe { c_helper_ptr(a, b) },
            || unsafe { rust_helper_ptr(a, b) },
        );
        assert_eq!(unsafe { c_global_fn(a, b) }, unsafe {
            rust_global_fn(a, b)
        });
    }
}

#[test]
fn generated_accumulator_branches_match() {
    let (c, rust) = load_pair();
    let c_fn: Symbol<UnaryFn> = unsafe { symbol(&c, b"use_generated\0") };
    let rust_fn: Symbol<UnaryFn> = unsafe { symbol(&rust, b"use_generated\0") };

    for n in 0..=6 {
        compare_call(|| unsafe { c_fn(n) }, || unsafe { rust_fn(n) });
    }

    let mut rng = FixedRng::new(0x6765_6e65_7261_3033);
    for n in [i32::MIN, -1, 7, 8, i32::MAX] {
        compare_call(|| unsafe { c_fn(n) }, || unsafe { rust_fn(n) });
    }
    for _ in 0..256 {
        let n = if rng.next_u32() & 1 == 0 {
            -1 - (rng.next_u32() % 1_000_000) as i32
        } else {
            7 + (rng.next_u32() % 1_000_000) as i32
        };
        compare_call(|| unsafe { c_fn(n) }, || unsafe { rust_fn(n) });
    }
}

#[test]
fn main_valid_paths_match_randomized() {
    let (c, rust) = load_pair();
    let c_main: Symbol<MainFn> = unsafe { symbol(&c, b"main\0") };
    let rust_main: Symbol<MainFn> = unsafe { symbol(&rust, b"main\0") };
    let mut rng = FixedRng::new(0x6d61_696e_5f76_3034);

    for index in 0..64 {
        let a = rng.bounded_i32(10_000);
        let b = rng.bounded_i32(10_000);
        let values = vec![
            "differential-driver".to_owned(),
            a.to_string(),
            b.to_string(),
            "ignored".to_owned(),
        ];
        let (_c_strings, mut c_argv) = argv(&values);
        let (_rust_strings, mut rust_argv) = argv(&values);
        let argc = if index & 1 == 0 { 3 } else { 4 };
        compare_call(
            || unsafe { c_main(argc, c_argv.as_mut_ptr()) },
            || unsafe { rust_main(argc, rust_argv.as_mut_ptr()) },
        );
    }
}

#[test]
fn main_rejections_match() {
    let (c, rust) = load_pair();
    let c_main: Symbol<MainFn> = unsafe { symbol(&c, b"main\0") };
    let rust_main: Symbol<MainFn> = unsafe { symbol(&rust, b"main\0") };

    for argc in [-10, -1, 0, 1, 2] {
        let values = vec!["differential-driver".to_owned()];
        let (_c_strings, mut c_argv) = argv(&values);
        let (_rust_strings, mut rust_argv) = argv(&values);
        compare_call(
            || unsafe { c_main(argc, c_argv.as_mut_ptr()) },
            || unsafe { rust_main(argc, rust_argv.as_mut_ptr()) },
        );
    }
}

#[test]
fn main_oversized_argc_matches() {
    let (c, rust) = load_pair();
    let c_main: Symbol<MainFn> = unsafe { symbol(&c, b"main\0") };
    let rust_main: Symbol<MainFn> = unsafe { symbol(&rust, b"main\0") };
    let values = vec!["differential-driver", "17", "-4"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let (_c_strings, mut c_argv) = argv(&values);
    let (_rust_strings, mut rust_argv) = argv(&values);

    compare_call(
        || unsafe { c_main(i32::MAX, c_argv.as_mut_ptr()) },
        || unsafe { rust_main(i32::MAX, rust_argv.as_mut_ptr()) },
    );
}

fn null_argv_status(library: &Path) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--ignored", "--exact", "null_argv_child", "--nocapture"])
        .env("MD_NULL_ARGV_LIBRARY", library)
        .status()
        .expect("run null argv child")
}

#[test]
fn main_null_argv_failure_matches() {
    let c_status = null_argv_status(&c_library_path());
    let rust_status = null_argv_status(&rust_library_path());
    assert_eq!(c_status.signal(), rust_status.signal());
    assert!(
        c_status.signal().is_some(),
        "C unexpectedly survived NULL argv"
    );
}

#[test]
#[ignore = "launched in a subprocess by main_null_argv_failure_matches"]
fn null_argv_child() {
    let Some(path) = std::env::var_os("MD_NULL_ARGV_LIBRARY") else {
        return;
    };
    let library = unsafe { Library::new(path).expect("load child library") };
    let main_fn: Symbol<MainFn> = unsafe { symbol(&library, b"main\0") };
    unsafe {
        main_fn(3, ptr::null_mut());
    }
    panic!("NULL argv call unexpectedly returned");
}
