use libloading::{Library, Symbol};
use std::ffi::{c_int, c_long, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fileno(stream: *mut c_void) -> c_int;
    fn fread(buffer: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fseek(stream: *mut c_void, offset: c_long, origin: c_int) -> c_int;
    fn ftell(stream: *mut c_void) -> c_long;
    fn rewind(stream: *mut c_void);
    fn tmpfile() -> *mut c_void;
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("resolve integration-test executable");
    test_executable
        .parent()
        .and_then(Path::parent)
        .expect("integration test must run from Cargo's target directory")
        .join("libdriver.so")
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

unsafe fn load_driver(library: &Library) -> Driver {
    let symbol: Symbol<'_, Driver> = unsafe {
        library
            .get(b"driver\0")
            .expect("shared object must export driver")
    };
    *symbol
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const SEEK_END: c_int = 2;
    const STDOUT_FILENO: c_int = 1;

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);

    let capture_file = unsafe { tmpfile() };
    assert!(!capture_file.is_null());
    let capture_fd = unsafe { fileno(capture_file) };
    assert!(capture_fd >= 0);
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(capture_fd, STDOUT_FILENO) }, STDOUT_FILENO);

    call();

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    assert_eq!(unsafe { fseek(capture_file, 0, SEEK_END) }, 0);
    let output_len = unsafe { ftell(capture_file) };
    assert!(output_len >= 0);
    unsafe { rewind(capture_file) };

    let mut output = vec![0_u8; output_len as usize];
    let bytes_read = unsafe {
        fread(
            output.as_mut_ptr().cast::<c_void>(),
            1,
            output.len(),
            capture_file,
        )
    };
    assert_eq!(bytes_read, output.len());
    assert_eq!(unsafe { fclose(capture_file) }, 0);
    output
}

fn test_inputs() -> Vec<c_int> {
    let mut inputs = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -1_073_741_825,
        -1_073_741_824,
        -151,
        -150,
        -149,
        -1,
        0,
        1,
        1_073_741_673,
        1_073_741_674,
        1_073_741_823,
        1_073_741_824,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    // Fixed-seed xorshift32 spans the complete c_int bit pattern.
    let mut state = 0x6d2b_79f5_u32;
    for _ in 0..4096 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        inputs.push(state as c_int);
    }
    inputs
}

#[test]
fn driver_matches_for_full_int_surface() {
    let _stdout_guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path).expect("load C shared object") };
    let rust_library = unsafe { Library::new(&rust_path).expect("load Rust shared object") };
    let c_driver = unsafe { load_driver(&c_library) };
    let rust_driver = unsafe { load_driver(&rust_library) };
    let inputs = test_inputs();

    let c_output = unsafe {
        capture_stdout(|| {
            for &input in &inputs {
                c_driver(input);
            }
        })
    };
    let rust_output = unsafe {
        capture_stdout(|| {
            for &input in &inputs {
                rust_driver(input);
            }
        })
    };

    assert_eq!(rust_output, c_output);
}
