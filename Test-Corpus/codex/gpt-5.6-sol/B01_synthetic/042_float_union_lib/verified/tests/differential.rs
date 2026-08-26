use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;

type Driver = unsafe extern "C" fn(f64);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("find integration test executable");
    test_executable
        .parent()
        .and_then(Path::parent)
        .expect("integration test executable under target profile directory")
        .join("libdriver.so")
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn read_all(fd: c_int) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let count = unsafe { read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        assert!(count >= 0, "read redirected stdout");
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }

    assert_eq!(unsafe { close(fd) }, 0, "close redirected stdout reader");
    output
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("lock process stdout");
    let mut pipe_fds = [-1; 2];

    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0, "flush stdout");
    assert_eq!(
        unsafe { pipe(pipe_fds.as_mut_ptr()) },
        0,
        "create stdout pipe"
    );

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1, "redirect stdout");
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0, "close pipe writer");

    let reader = thread::spawn(move || read_all(pipe_fds[0]));
    call();

    assert_eq!(
        unsafe { fflush(std::ptr::null_mut()) },
        0,
        "flush redirected stdout"
    );
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "restore stdout");
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close saved stdout");

    reader.join().expect("join stdout reader")
}

fn inputs() -> Vec<u64> {
    let mut bits = vec![
        0x0000_0000_0000_0000, // positive zero
        0x8000_0000_0000_0000, // negative zero
        0x0000_0000_0000_0001, // minimum positive subnormal
        0x000f_ffff_ffff_ffff, // maximum positive subnormal
        0x0010_0000_0000_0000, // minimum positive normal
        0x3ff0_0000_0000_0000, // one
        0xbff0_0000_0000_0000, // negative one
        0x7fef_ffff_ffff_ffff, // maximum finite
        0xffef_ffff_ffff_ffff, // minimum finite
        0x7ff0_0000_0000_0000, // positive infinity
        0xfff0_0000_0000_0000, // negative infinity
        0x7ff8_0000_0000_0000, // positive quiet NaN
        0xfff8_0000_0000_0000, // negative quiet NaN
        0x7ff0_0000_0000_0001, // positive signaling NaN
        0xfff0_0000_0000_0001, // negative signaling NaN
        0x7fff_ffff_ffff_ffff, // maximum positive NaN payload
        0xffff_ffff_ffff_ffff, // maximum negative NaN payload
    ];

    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..8192 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bits.push(state);
    }
    bits
}

fn invoke_all(library_path: &Path, inputs: &[u64]) -> Vec<u8> {
    let library = unsafe { Library::new(library_path) }
        .unwrap_or_else(|error| panic!("load {}: {error}", library_path.display()));
    let driver: Symbol<'_, Driver> =
        unsafe { library.get(b"driver\0") }.expect("load exported driver symbol");

    capture_stdout(|| {
        for &bits in inputs {
            unsafe { driver(f64::from_bits(bits)) };
        }
    })
}

#[test]
fn driver_matches_c_for_all_runtime_configurations() {
    let inputs = inputs();
    let c_output = invoke_all(&c_library_path(), &inputs);
    let rust_output = invoke_all(&rust_library_path(), &inputs);

    let c_lines: Vec<_> = c_output.split_inclusive(|byte| *byte == b'\n').collect();
    let rust_lines: Vec<_> = rust_output.split_inclusive(|byte| *byte == b'\n').collect();
    assert_eq!(c_lines.len(), inputs.len(), "C emitted one line per call");
    assert_eq!(
        rust_lines.len(),
        inputs.len(),
        "Rust emitted one line per call"
    );

    for (index, ((c_line, rust_line), bits)) in
        c_lines.iter().zip(&rust_lines).zip(&inputs).enumerate()
    {
        assert_eq!(
            c_line, rust_line,
            "output mismatch at input {index}, bits 0x{bits:016x}"
        );
    }
    assert_eq!(
        c_output, rust_output,
        "complete output must be byte-identical"
    );
}
