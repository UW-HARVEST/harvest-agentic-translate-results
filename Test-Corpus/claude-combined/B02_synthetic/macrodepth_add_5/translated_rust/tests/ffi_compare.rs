// FFI tests: load both the C .so and the Rust .so and compare their public
// exports for byte-identical behavior.
//
// The C .so is built by build.rs into target/c_so/libdriver_c.so using gcc
// with the same OP/REPEAT macro values that the active Cargo features
// represent. The Rust .so is the one produced by `cargo build`.

use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

type OpFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type UnaryFn = unsafe extern "C" fn(c_int) -> c_int;

fn rust_lib_path() -> PathBuf {
    // The cdylib produced by `cargo test` lands either next to the test
    // binary in target/<profile>/deps or one level up in target/<profile>.
    let mut start = std::env::current_exe().expect("current_exe");
    start.pop(); // -> deps/
    let candidates_names = ["libdriver.so", "libdriver.dylib", "driver.dll"];
    let search_dirs = [start.clone(), start.parent().unwrap().to_path_buf()];
    for dir in &search_dirs {
        for c in &candidates_names {
            let p = dir.join(c);
            if p.exists() {
                return p;
            }
        }
    }
    panic!(
        "Could not locate driver cdylib (searched {:?})",
        search_dirs
    );
}

fn c_lib_path() -> PathBuf {
    // Built by build.rs.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("target")
        .join("c_so")
        .join("libdriver_c.so")
}

// Minimal raw bindings to the libc functions we need for stdout capture.
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    // Flush any buffered output first.
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let original_stdout = unsafe { dup(1) };
    assert!(original_stdout >= 0, "dup failed");

    let mut pipe_fds = [0i32; 2];
    let r = unsafe { pipe(pipe_fds.as_mut_ptr()) };
    assert_eq!(r, 0, "pipe failed");
    let read_fd = pipe_fds[0];
    let write_fd = pipe_fds[1];

    unsafe {
        dup2(write_fd, 1);
        close(write_fd);
    }

    f();

    // Flush so all output reaches the pipe before we drain it.
    unsafe {
        fflush(std::ptr::null_mut());
        // Restore stdout.
        dup2(original_stdout, 1);
        close(original_stdout);
    }

    let mut buf = Vec::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    file.read_to_end(&mut buf).expect("read pipe");
    String::from_utf8_lossy(&buf).into_owned()
}

struct Loaded {
    lib: Library,
}

impl Loaded {
    unsafe fn new(path: PathBuf) -> Self {
        let lib = unsafe { Library::new(path) }.expect("failed to load library");
        Loaded { lib }
    }

    unsafe fn op_fn<'a>(&'a self, name: &str) -> Symbol<'a, OpFn> {
        unsafe { self.lib.get(name.as_bytes()) }.expect("missing op_fn symbol")
    }

    unsafe fn unary_fn<'a>(&'a self, name: &str) -> Symbol<'a, UnaryFn> {
        unsafe { self.lib.get(name.as_bytes()) }.expect("missing unary_fn symbol")
    }

    unsafe fn g_op(&self) -> OpFn {
        let s: Symbol<*mut Option<OpFn>> =
            unsafe { self.lib.get(b"G_OP") }.expect("missing G_OP symbol");
        // The C symbol is `int (*G_OP)(int,int)` — a non-nullable function pointer
        // stored as a pointer-sized word. The Rust symbol is `Option<extern "C" fn(...)>`,
        // which has the same niche-optimized layout (None == null pointer).
        // Reading either via the same memory works because both are a single
        // pointer-sized word and a non-null function pointer is a valid Some.
        unsafe { (**s).expect("G_OP not initialized") }
    }

    unsafe fn g_op_name(&self) -> *const c_char {
        let s: Symbol<*mut *const c_char> =
            unsafe { self.lib.get(b"G_OP_NAME") }.expect("missing G_OP_NAME symbol");
        unsafe { **s }
    }
}

#[test]
fn ffi_op_add_matches() {
    unsafe {
        let c = Loaded::new(c_lib_path());
        let r = Loaded::new(rust_lib_path());

        let cf = c.op_fn("op_add");
        let rf = r.op_fn("op_add");
        for (a, b) in [(0, 0), (3, 4), (-1, 1), (100, -50), (i32::MAX, 1)] {
            assert_eq!(cf(a, b), rf(a, b), "op_add({},{})", a, b);
        }
    }
}

#[test]
fn ffi_op_sub_matches() {
    unsafe {
        let c = Loaded::new(c_lib_path());
        let r = Loaded::new(rust_lib_path());

        let cf = c.op_fn("op_sub");
        let rf = r.op_fn("op_sub");
        for (a, b) in [(0, 0), (3, 4), (-1, 1), (100, -50), (i32::MIN, 1)] {
            assert_eq!(cf(a, b), rf(a, b), "op_sub({},{})", a, b);
        }
    }
}

#[test]
fn ffi_op_mul_matches() {
    unsafe {
        let c = Loaded::new(c_lib_path());
        let r = Loaded::new(rust_lib_path());

        let cf = c.op_fn("op_mul");
        let rf = r.op_fn("op_mul");
        for (a, b) in [(0, 0), (3, 4), (-1, 1), (100, -50), (7, 6)] {
            assert_eq!(cf(a, b), rf(a, b), "op_mul({},{})", a, b);
        }
    }
}

// Stdout-capturing tests cannot safely run concurrently with the rest of
// the harness (which prints "ok"/"FAILED" lines to stdout between tests).
// We collapse the three printf-using comparisons into a single test that
// captures stdout once and serializes the comparisons.
#[test]
fn ffi_printf_helpers_match() {
    unsafe {
        let c = Loaded::new(c_lib_path());
        let r = Loaded::new(rust_lib_path());

        let c_call = c.op_fn("helper_call");
        let r_call = r.op_fn("helper_call");
        let c_ptr = c.op_fn("helper_ptr");
        let r_ptr = r.op_fn("helper_ptr");
        let c_use = c.unary_fn("use_generated");
        let r_use = r.unary_fn("use_generated");

        for (a, b) in [(0, 0), (3, 4), (-2, 5), (10, 10)] {
            let mut cret = 0;
            let cout = capture_stdout(|| {
                cret = c_call(a, b);
            });
            let mut rret = 0;
            let rout = capture_stdout(|| {
                rret = r_call(a, b);
            });
            assert_eq!(cout, rout, "helper_call stdout differs for ({},{})", a, b);
            assert_eq!(cret, rret, "helper_call return differs for ({},{})", a, b);

            let mut cret = 0;
            let cout = capture_stdout(|| {
                cret = c_ptr(a, b);
            });
            let mut rret = 0;
            let rout = capture_stdout(|| {
                rret = r_ptr(a, b);
            });
            assert_eq!(cout, rout, "helper_ptr stdout differs for ({},{})", a, b);
            assert_eq!(cret, rret, "helper_ptr return differs for ({},{})", a, b);
        }

        for n in 0..=7i32 {
            let mut cret = 0;
            let cout = capture_stdout(|| {
                cret = c_use(n);
            });
            let mut rret = 0;
            let rout = capture_stdout(|| {
                rret = r_use(n);
            });
            assert_eq!(cout, rout, "use_generated stdout differs for n={}", n);
            assert_eq!(cret, rret, "use_generated return differs for n={}", n);
        }
    }
}

#[test]
fn ffi_g_op_call_matches() {
    unsafe {
        let c = Loaded::new(c_lib_path());
        let r = Loaded::new(rust_lib_path());

        let cf = c.g_op();
        let rf = r.g_op();
        for (a, b) in [(0, 0), (3, 4), (-1, 1), (100, -50), (7, 6)] {
            assert_eq!(cf(a, b), rf(a, b), "G_OP({},{})", a, b);
        }
    }
}

#[test]
fn ffi_g_op_name_matches() {
    unsafe {
        let c = Loaded::new(c_lib_path());
        let r = Loaded::new(rust_lib_path());

        let cn = c.g_op_name();
        let rn = r.g_op_name();
        let cs = CStr::from_ptr(cn).to_str().expect("c utf8");
        let rs = CStr::from_ptr(rn).to_str().expect("r utf8");
        assert_eq!(cs, rs, "G_OP_NAME differs");
    }
}
