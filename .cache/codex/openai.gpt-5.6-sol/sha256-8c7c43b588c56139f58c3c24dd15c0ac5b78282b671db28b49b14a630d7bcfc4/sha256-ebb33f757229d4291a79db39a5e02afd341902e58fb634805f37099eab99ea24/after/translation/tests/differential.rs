use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;

type Driver = unsafe extern "C" fn(f64);

unsafe extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = crate_dir
        .parent()
        .expect("translation crate has no parent directory")
        .join("c_src/build/libdriver.so");
    let rust_library = crate_dir.join("target/release/libdriver.so");
    (c_library, rust_library)
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");
        assert_eq!(fflush(std::ptr::null_mut()), 0, "initial fflush failed");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirecting stdout failed"
        );
        assert_eq!(
            close(pipe_fds[1]),
            0,
            "closing duplicate pipe writer failed"
        );
        let read_fd = pipe_fds[0];
        let reader_thread = thread::spawn(move || {
            let mut output = Vec::new();
            let mut reader = File::from_raw_fd(read_fd);
            reader
                .read_to_end(&mut output)
                .expect("reading captured stdout failed");
            output
        });

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "final fflush failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restoring stdout failed"
        );
        assert_eq!(close(saved_stdout), 0, "closing saved stdout failed");
        reader_thread.join().expect("stdout reader thread panicked")
    }
}

fn test_values() -> Vec<u64> {
    let mut values = vec![
        0x0000_0000_0000_0000, // +0
        0x8000_0000_0000_0000, // -0
        0x0000_0000_0000_0001, // smallest positive subnormal
        0x8000_0000_0000_0001, // smallest negative subnormal
        0x000f_ffff_ffff_ffff, // largest positive subnormal
        0x800f_ffff_ffff_ffff, // largest negative subnormal
        0x0010_0000_0000_0000, // smallest positive normal
        0x8010_0000_0000_0000, // smallest negative normal
        0x3ff0_0000_0000_0000, // +1
        0xbff0_0000_0000_0000, // -1
        0x7fef_ffff_ffff_ffff, // largest finite
        0xffef_ffff_ffff_ffff, // most negative finite
        0x7ff0_0000_0000_0000, // +infinity
        0xfff0_0000_0000_0000, // -infinity
        0x7ff8_0000_0000_0000, // canonical quiet NaN
        0xfff8_0000_0000_0000, // negative quiet NaN
        0x7ff0_0000_0000_0001, // signaling NaN with payload
        0x7fff_ffff_ffff_ffff, // quiet NaN with maximal payload
        0x3f1a_36e2_eb1c_432d, // value near four-decimal formatting boundaries
        0x3ff3_c083_126e_978d, // non-terminating decimal fraction
    ];

    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    for _ in 0..32_768 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        values.push(state.wrapping_mul(0x2545_f491_4f6c_dd1d));
    }
    values
}

#[test]
fn driver_matches_c_for_all_binary64_shapes() {
    let (c_path, rust_path) = library_paths();
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

    let c_library = unsafe { Library::new(&c_path) }.expect("loading C library failed");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("loading Rust library failed");
    let c_driver: Symbol<Driver> =
        unsafe { c_library.get(b"driver\0") }.expect("C driver symbol missing");
    let rust_driver: Symbol<Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("Rust driver symbol missing");
    let bits = test_values();

    let c_output = capture_stdout(|| {
        for &value in &bits {
            unsafe { c_driver(f64::from_bits(value)) };
        }
    });
    let rust_output = capture_stdout(|| {
        for &value in &bits {
            unsafe { rust_driver(f64::from_bits(value)) };
        }
    });

    if c_output != rust_output {
        let c_lines: Vec<&[u8]> = c_output.split_inclusive(|byte| *byte == b'\n').collect();
        let rust_lines: Vec<&[u8]> = rust_output.split_inclusive(|byte| *byte == b'\n').collect();
        let mismatch = c_lines
            .iter()
            .zip(&rust_lines)
            .position(|(c_line, rust_line)| c_line != rust_line)
            .unwrap_or_else(|| c_lines.len().min(rust_lines.len()));
        let input = bits.get(mismatch).copied();
        panic!(
            "stdout differs at call {mismatch}, input bits={input:#018x?}\nC: {:?}\nRust: {:?}\nC bytes: {}, Rust bytes: {}",
            c_lines
                .get(mismatch)
                .map(|line| String::from_utf8_lossy(line)),
            rust_lines
                .get(mismatch)
                .map(|line| String::from_utf8_lossy(line)),
            c_output.len(),
            rust_output.len()
        );
    }
}
