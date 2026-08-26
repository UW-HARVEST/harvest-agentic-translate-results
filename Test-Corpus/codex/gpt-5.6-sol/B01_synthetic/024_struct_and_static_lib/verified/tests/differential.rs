use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
};

type EntryPoint = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fd: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

const STDOUT_FILENO: c_int = 1;
static COPY_ID: AtomicUsize = AtomicUsize::new(0);
static RUST_LIBRARY_PATH: OnceLock<PathBuf> = OnceLock::new();

struct LibraryPair {
    c_library: Library,
    rust_library: Library,
    c_run: EntryPoint,
    rust_run: EntryPoint,
    c_driver: EntryPoint,
    rust_driver: EntryPoint,
    copies: [PathBuf; 2],
}

impl LibraryPair {
    unsafe fn fresh() -> Self {
        let c_source = Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so");
        let rust_source = rust_library_path();
        let copy_id = COPY_ID.fetch_add(1, Ordering::Relaxed);
        let prefix = format!(
            "translated-driver-differential-{}-{copy_id}",
            std::process::id()
        );
        let copies = [
            std::env::temp_dir().join(format!("{prefix}-c.so")),
            std::env::temp_dir().join(format!("{prefix}-rust.so")),
        ];

        fs::copy(&c_source, &copies[0])
            .unwrap_or_else(|error| panic!("failed to copy {}: {error}", c_source.display()));
        fs::copy(&rust_source, &copies[1])
            .unwrap_or_else(|error| panic!("failed to copy {}: {error}", rust_source.display()));

        let c_library = unsafe { Library::new(&copies[0]) }.unwrap();
        let rust_library = unsafe { Library::new(&copies[1]) }.unwrap();
        let c_run = unsafe { *c_library.get::<EntryPoint>(b"run\0").unwrap() };
        let rust_run = unsafe { *rust_library.get::<EntryPoint>(b"run\0").unwrap() };
        let c_driver = unsafe { *c_library.get::<EntryPoint>(b"driver\0").unwrap() };
        let rust_driver = unsafe { *rust_library.get::<EntryPoint>(b"driver\0").unwrap() };

        Self {
            c_library,
            rust_library,
            c_run,
            rust_run,
            c_driver,
            rust_driver,
            copies,
        }
    }

    fn compare_run(&self, value: c_int, context: &str) {
        let c_output = capture_stdout(|| unsafe { (self.c_run)(value) });
        let rust_output = capture_stdout(|| unsafe { (self.rust_run)(value) });
        assert_eq!(
            c_output, rust_output,
            "{context}: run({value}) output differs"
        );
    }

    fn compare_driver(&self, value: c_int, context: &str) {
        let c_output = capture_stdout(|| unsafe { (self.c_driver)(value) });
        let rust_output = capture_stdout(|| unsafe { (self.rust_driver)(value) });
        assert_eq!(
            c_output, rust_output,
            "{context}: driver({value}) output differs"
        );
    }
}

impl Drop for LibraryPair {
    fn drop(&mut self) {
        // Keep the handles visibly live until after all function-pointer calls.
        let _ = (&self.c_library, &self.rust_library);
        for copy in &self.copies {
            let _ = fs::remove_file(copy);
        }
    }
}

fn rust_library_path() -> PathBuf {
    RUST_LIBRARY_PATH
        .get_or_init(|| {
            if let Some(path) = std::env::var_os("DRIVER_RUST_SO") {
                return path.into();
            }

            let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
            let target_dir = manifest_dir.join("target/differential-cdylib");
            let output = Command::new(env!("CARGO"))
                .arg("build")
                .arg("--manifest-path")
                .arg(manifest_dir.join("Cargo.toml"))
                .arg("--target-dir")
                .arg(&target_dir)
                .arg("--no-default-features")
                .output()
                .expect("failed to start cargo build for the Rust cdylib");
            assert!(
                output.status.success(),
                "Rust cdylib build failed:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );

            let library = target_dir.join("debug/libdriver.so");
            assert!(
                library.is_file(),
                "cargo build did not produce {}",
                library.display()
            );
            library
        })
        .clone()
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds = [0; 2];
    unsafe {
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");
        assert_eq!(fflush(std::ptr::null_mut()), 0, "initial fflush failed");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup failed");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "stdout redirect failed"
        );
        assert_eq!(close(pipe_fds[1]), 0, "closing pipe writer failed");

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "captured fflush failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "stdout restore failed"
        );
        assert_eq!(close(saved_stdout), 0, "closing saved stdout failed");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0, "read failed");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "closing pipe reader failed");
        output
    }
}

struct FixedRng(u64);

impl FixedRng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn bounded_i32(&mut self) -> i32 {
        (self.next_u32() % 20_001) as i32 - 10_000
    }
}

#[test]
fn all_configuration_rows_match_byte_for_byte() {
    unsafe {
        // CONFIGS.md row 1: independent one-shot run calls from fresh state.
        let mut rng = FixedRng(0x4f8c_2d19_a673_b501);
        let mut values = vec![i32::MIN, -10_000, -1, 0, 1, 10_000, i32::MAX - 5];
        values.extend((0..64).map(|_| rng.bounded_i32()));
        for (iteration, value) in values.into_iter().enumerate() {
            LibraryPair::fresh().compare_run(value, &format!("row 1 iteration {iteration}"));
        }

        // CONFIGS.md row 2: many run calls sharing persistent state.
        let pair = LibraryPair::fresh();
        let mut rng = FixedRng(0xf2a4_0ce7_931d_685b);
        for iteration in 0..256 {
            let value = match iteration {
                0 => -1,
                1 => 0,
                2 => 1,
                _ => rng.bounded_i32(),
            };
            pair.compare_run(value, &format!("row 2 iteration {iteration}"));
        }

        // CONFIGS.md row 3: independent composed driver calls from fresh state.
        let mut rng = FixedRng(0x91b6_5a02_d48e_37fc);
        let safe_positive_edge = (i32::MAX - 5) / 2;
        let mut values = vec![i32::MIN / 2, -10_000, -1, 0, 1, 10_000, safe_positive_edge];
        values.extend((0..64).map(|_| rng.bounded_i32()));
        for (iteration, value) in values.into_iter().enumerate() {
            LibraryPair::fresh().compare_driver(value, &format!("row 3 iteration {iteration}"));
        }

        // CONFIGS.md row 4: both entry points sharing one persistent state.
        let pair = LibraryPair::fresh();
        let mut rng = FixedRng(0x2d73_ea95_60c1_4bf8);
        for iteration in 0..256 {
            let value = match iteration {
                0 => -1,
                1 => 0,
                2 => 1,
                _ => rng.bounded_i32(),
            };
            if rng.next_u32() & 1 == 0 {
                pair.compare_run(value, &format!("row 4 iteration {iteration}"));
            } else {
                pair.compare_driver(value, &format!("row 4 iteration {iteration}"));
            }
        }
    }
}
