use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type HelloWorld = unsafe extern "C" fn() -> c_int;

const STDOUT_FILENO: c_int = 1;
const MESSAGE: &[u8] = b"Hello World!\n";

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct StdoutRestore {
    saved_fd: c_int,
}

impl Drop for StdoutRestore {
    fn drop(&mut self) {
        unsafe {
            let _ = dup2(self.saved_fd, STDOUT_FILENO);
            let _ = close(self.saved_fd);
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libhello.so"),
        crate_root.join("target/release/libhello.so"),
    )
}

fn capture_calls(function: HelloWorld, call_count: usize) -> (Vec<c_int>, Vec<u8>) {
    let _lock = STDOUT_LOCK.lock().expect("stdout capture lock poisoned");
    let capture_id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let capture_path = std::env::temp_dir().join(format!(
        "hello-differential-{}-{capture_id}.out",
        std::process::id()
    ));
    let mut capture = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&capture_path)
        .expect("create stdout capture file");

    let returns = unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before capture");
        let saved_fd = dup(STDOUT_FILENO);
        assert!(saved_fd >= 0, "duplicate stdout");
        let restore = StdoutRestore { saved_fd };
        assert_eq!(
            dup2(capture.as_raw_fd(), STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );

        let values = (0..call_count).map(|_| function()).collect();
        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stdout");
        drop(restore);
        values
    };

    capture.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut bytes = Vec::new();
    capture.read_to_end(&mut bytes).expect("read capture");
    drop(capture);
    fs::remove_file(&capture_path).expect("remove capture file");
    (returns, bytes)
}

#[test]
fn helloworld_matches_through_both_dynamic_libraries() {
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

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_helloworld: HelloWorld = *c_library
            .get::<HelloWorld>(b"helloworld\0")
            .expect("load C helloworld symbol");
        let rust_helloworld: HelloWorld = *rust_library
            .get::<HelloWorld>(b"helloworld\0")
            .expect("load Rust helloworld symbol");

        // There are no function inputs. Use a fixed-seed randomized series of
        // invocation counts to compare repeated calls and all observable bytes.
        let mut seed = 0x4d59_5df4_d0f3_3173_u64;
        for trial in 0..128 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let call_count = ((seed >> 32) as usize % 32) + 1;

            let (c_returns, c_stdout) = capture_calls(c_helloworld, call_count);
            let (rust_returns, rust_stdout) = capture_calls(rust_helloworld, call_count);

            assert_eq!(rust_returns, c_returns, "return mismatch in trial {trial}");
            assert!(
                c_returns.iter().all(|&value| value == 0),
                "C returned a nonzero value in trial {trial}"
            );
            assert_eq!(rust_stdout, c_stdout, "stdout mismatch in trial {trial}");
            assert_eq!(
                c_stdout,
                MESSAGE.repeat(call_count),
                "unexpected C stdout in trial {trial}"
            );
        }
    }
}
