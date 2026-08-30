use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("lock stdout capture");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(
            fflush(std::ptr::null_mut()),
            0,
            "flush stdout before capture"
        );
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout capture pipe");

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(dup2(pipe_fds[1], 1), 1, "redirect stdout");
        assert_eq!(close(pipe_fds[1]), 0, "close extra pipe writer");

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 128];
        loop {
            let bytes_read = read(
                pipe_fds[0],
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len(),
            );
            assert!(bytes_read >= 0, "read captured stdout");
            if bytes_read == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..bytes_read as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close pipe reader");
        output
    }
}

fn call_driver(driver: &Symbol<'_, Driver>, floors: c_int) -> Vec<u8> {
    capture_stdout(|| unsafe {
        driver(floors);
    })
}

#[test]
fn driver_matches_for_boundaries_and_randomized_ints() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
    let c_driver: Symbol<'_, Driver> =
        unsafe { c_library.get(b"driver\0") }.expect("load C driver export");
    let rust_driver: Symbol<'_, Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("load Rust driver export");

    let mut inputs = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    let mut state = 0x5eed_c0de_u32;
    for _ in 0..4096 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        inputs.push(state as c_int);
    }

    for floors in inputs {
        let c_output = call_driver(&c_driver, floors);
        let rust_output = call_driver(&rust_driver, floors);
        assert_eq!(
            rust_output,
            c_output,
            "stdout differs for driver({floors}); C={}, Rust={}",
            String::from_utf8_lossy(&c_output),
            String::from_utf8_lossy(&rust_output)
        );
        assert_eq!(c_output.len(), 33, "unexpected C output width for {floors}");
        assert_eq!(
            c_output.last(),
            Some(&b'\n'),
            "missing C newline for {floors}"
        );
    }
}
