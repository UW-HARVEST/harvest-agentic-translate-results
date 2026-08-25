use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(f32);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("c_src/build/libdriver.so"),
        root.join("target/debug/libdriver.so"),
    )
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;

    let _guard = STDOUT_LOCK.lock().expect("stdout capture mutex poisoned");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush before capture");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup stdout");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close duplicated pipe writer");

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush captured output");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    let mut output = Vec::new();
    unsafe {
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
    }
    output
}

fn next_random(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

#[test]
fn driver_matches_for_all_float_classes_and_random_bit_patterns() {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_driver: Symbol<'_, Driver> = c_library.get(b"driver").expect("load C driver");
        let rust_driver: Symbol<'_, Driver> =
            rust_library.get(b"driver").expect("load Rust driver");

        let edge_patterns = [
            0x0000_0000,
            0x8000_0000,
            0x0000_0001,
            0x007f_ffff,
            0x0080_0000,
            0x3f80_0000,
            0x7f7f_ffff,
            0xff7f_ffff,
            0x7f80_0000,
            0xff80_0000,
            0x7f80_0001,
            0x7fc0_0000,
            0x7fff_ffff,
            0xff80_0001,
            0xffff_ffff,
        ];

        let compare = |bits: u32| {
            let value = f32::from_bits(bits);
            let c_output = capture_stdout(|| c_driver(value));
            let rust_output = capture_stdout(|| rust_driver(value));
            assert_eq!(
                rust_output, c_output,
                "output differs for float bit pattern 0x{bits:08x}"
            );
        };

        for bits in edge_patterns {
            compare(bits);
        }

        let mut state = 0x5eed_c0de_d15c_a11e;
        for _ in 0..4096 {
            compare(next_random(&mut state));
        }
    }
}
