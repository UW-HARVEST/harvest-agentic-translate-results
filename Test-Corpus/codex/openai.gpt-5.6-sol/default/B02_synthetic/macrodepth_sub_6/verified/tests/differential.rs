use libloading::{Library, Symbol};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::ptr;

type BinaryOp = unsafe extern "C" fn(c_int, c_int) -> c_int;
type UnaryOp = unsafe extern "C" fn(c_int) -> c_int;

unsafe extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Inputs(u64);

impl Inputs {
    fn new() -> Self {
        Self(0x6d64_2d64_6966_6621)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn bounded_i32(&mut self, bound: i32) -> i32 {
        let width = i64::from(bound) * 2 + 1;
        (self.next() as i64).rem_euclid(width) as i32 - bound
    }
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} must point to the artifact under differential test"))
}

fn selected_op() -> &'static str {
    if cfg!(feature = "mul") {
        "mul"
    } else if cfg!(feature = "sub") {
        "sub"
    } else {
        "add"
    }
}

fn selected_repeat() -> i32 {
    if cfg!(feature = "7") {
        7
    } else if cfg!(feature = "6") {
        6
    } else if cfg!(feature = "5") {
        5
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

unsafe fn function<T: Copy>(library: &Library, name: &[u8]) -> T {
    let symbol: Symbol<'_, T> =
        unsafe { library.get(name) }.unwrap_or_else(|error| panic!("{name:?}: {error}"));
    *symbol
}

unsafe fn global_op(library: &Library) -> BinaryOp {
    let symbol: Symbol<'_, *const BinaryOp> =
        unsafe { library.get(b"G_OP\0") }.expect("G_OP export");
    unsafe { **symbol }
}

unsafe fn global_name(library: &Library) -> Vec<u8> {
    let symbol: Symbol<'_, *const *const c_char> =
        unsafe { library.get(b"G_OP_NAME\0") }.expect("G_OP_NAME export");
    let pointer = unsafe { **symbol };
    assert!(!pointer.is_null(), "G_OP_NAME must not be null");
    unsafe { CStr::from_ptr(pointer) }.to_bytes().to_vec()
}

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let mut descriptors = [-1, -1];
    assert_eq!(unsafe { pipe(descriptors.as_mut_ptr()) }, 0, "pipe");
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup stdout");

    unsafe {
        fflush(ptr::null_mut());
    }
    assert_eq!(unsafe { dup2(descriptors[1], 1) }, 1, "redirect stdout");
    assert_eq!(unsafe { close(descriptors[1]) }, 0, "close pipe writer");

    let result = call();

    unsafe {
        fflush(ptr::null_mut());
    }
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "restore stdout");
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close saved stdout");

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(descriptors[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    (result, output)
}

fn run_driver(path: &PathBuf, arguments: &[String]) -> Output {
    let mut command = Command::new(path);
    command.arg0("driver");
    command.args(arguments);
    command.output().expect("run driver")
}

fn compare_driver(c_path: &PathBuf, rust_path: &PathBuf, arguments: &[String]) {
    let c = run_driver(c_path, arguments);
    let rust = run_driver(rust_path, arguments);
    assert_eq!(
        rust.status.code(),
        c.status.code(),
        "status for {arguments:?}"
    );
    assert_eq!(rust.stdout, c.stdout, "stdout for {arguments:?}");
    assert_eq!(rust.stderr, c.stderr, "stderr for {arguments:?}");
}

#[test]
fn all_valid_and_error_surfaces_match_through_dynamic_ffi() {
    let c_so_path = required_path("C_REFERENCE_SO");
    let rust_so_path = required_path("RUST_TRANSLATION_SO");
    let c_driver_path = required_path("C_REFERENCE_BIN");
    let rust_driver_path = required_path("RUST_TRANSLATION_BIN");

    let c_library = unsafe { Library::new(&c_so_path) }.expect("load C shared object");
    let rust_library = unsafe { Library::new(&rust_so_path) }.expect("load Rust shared object");

    let c_add: BinaryOp = unsafe { function(&c_library, b"op_add\0") };
    let r_add: BinaryOp = unsafe { function(&rust_library, b"op_add\0") };
    let c_sub: BinaryOp = unsafe { function(&c_library, b"op_sub\0") };
    let r_sub: BinaryOp = unsafe { function(&rust_library, b"op_sub\0") };
    let c_mul: BinaryOp = unsafe { function(&c_library, b"op_mul\0") };
    let r_mul: BinaryOp = unsafe { function(&rust_library, b"op_mul\0") };
    let c_helper_call: BinaryOp = unsafe { function(&c_library, b"helper_call\0") };
    let r_helper_call: BinaryOp = unsafe { function(&rust_library, b"helper_call\0") };
    let c_helper_ptr: BinaryOp = unsafe { function(&c_library, b"helper_ptr\0") };
    let r_helper_ptr: BinaryOp = unsafe { function(&rust_library, b"helper_ptr\0") };
    let c_generated: UnaryOp = unsafe { function(&c_library, b"use_generated\0") };
    let r_generated: UnaryOp = unsafe { function(&rust_library, b"use_generated\0") };
    let c_global_op = unsafe { global_op(&c_library) };
    let r_global_op = unsafe { global_op(&rust_library) };

    assert_eq!(
        unsafe { global_name(&rust_library) },
        selected_op().as_bytes()
    );
    assert_eq!(unsafe { global_name(&rust_library) }, unsafe {
        global_name(&c_library)
    });

    let mut inputs = Inputs::new();
    let operation_boundaries = [
        (i32::MIN, 0),
        (i32::MAX, 0),
        (i32::MIN, 1),
        (i32::MAX, -1),
        (0, i32::MIN),
        (0, i32::MAX),
    ];
    for (a, b) in operation_boundaries {
        assert_eq!(unsafe { r_add(a, b) }, unsafe { c_add(a, b) });
    }
    let subtraction_boundaries = [
        (i32::MIN, 0),
        (i32::MAX, 0),
        (i32::MIN, -1),
        (i32::MAX, 1),
        (0, i32::MIN + 1),
        (0, i32::MAX),
    ];
    for (a, b) in subtraction_boundaries {
        assert_eq!(unsafe { r_sub(a, b) }, unsafe { c_sub(a, b) });
    }
    let multiplication_boundaries = [
        (i32::MIN, 0),
        (i32::MAX, 0),
        (i32::MIN, 1),
        (i32::MAX, 1),
        (1, i32::MIN),
        (1, i32::MAX),
    ];
    for (a, b) in multiplication_boundaries {
        assert_eq!(unsafe { r_mul(a, b) }, unsafe { c_mul(a, b) });
    }

    for _ in 0..256 {
        let a = inputs.bounded_i32(30_000);
        let b = inputs.bounded_i32(30_000);
        assert_eq!(unsafe { r_add(a, b) }, unsafe { c_add(a, b) });
        assert_eq!(unsafe { r_sub(a, b) }, unsafe { c_sub(a, b) });
        assert_eq!(unsafe { r_mul(a, b) }, unsafe { c_mul(a, b) });
        assert_eq!(unsafe { r_global_op(a, b) }, unsafe { c_global_op(a, b) });

        let c = capture_stdout(|| unsafe { c_helper_ptr(a, b) });
        let rust = capture_stdout(|| unsafe { r_helper_ptr(a, b) });
        assert_eq!(rust, c, "helper_ptr({a}, {b})");

        let c = capture_stdout(|| unsafe { c_helper_call(a, b) });
        let rust = capture_stdout(|| unsafe { r_helper_call(a, b) });
        assert_eq!(
            rust,
            c,
            "helper_call({}, {}, OP={}, REPEAT={})",
            a,
            b,
            selected_op(),
            selected_repeat()
        );
    }

    for n in 0..=6 {
        let c = capture_stdout(|| unsafe { c_generated(n) });
        let rust = capture_stdout(|| unsafe { r_generated(n) });
        assert_eq!(rust, c, "use_generated({n}), OP={}", selected_op());
    }

    let mut default_values = vec![i32::MIN, -1, 7, 8, i32::MAX];
    for _ in 0..256 {
        let candidate = inputs.next() as i32;
        if !(0..=6).contains(&candidate) {
            default_values.push(candidate);
        }
    }
    for n in default_values {
        let c = capture_stdout(|| unsafe { c_generated(n) });
        let rust = capture_stdout(|| unsafe { r_generated(n) });
        assert_eq!(rust, c, "use_generated({n}), OP={}", selected_op());
    }

    let fixed_driver_inputs = [
        ("0", "0"),
        ("-1", "1"),
        ("+17", "-23"),
        ("12junk", "7"),
        ("junk", "9"),
        (" 42", " -6"),
        ("30000", "-30000"),
    ];
    for (a, b) in fixed_driver_inputs {
        compare_driver(
            &c_driver_path,
            &rust_driver_path,
            &[a.to_owned(), b.to_owned()],
        );
    }
    for _ in 0..32 {
        let bound = if selected_op() == "mul" {
            30_000
        } else {
            1_000_000
        };
        let a = inputs.bounded_i32(bound).to_string();
        let b = inputs.bounded_i32(bound).to_string();
        compare_driver(&c_driver_path, &rust_driver_path, &[a, b]);
    }

    compare_driver(&c_driver_path, &rust_driver_path, &[]);
    compare_driver(&c_driver_path, &rust_driver_path, &["123".to_owned()]);
}
