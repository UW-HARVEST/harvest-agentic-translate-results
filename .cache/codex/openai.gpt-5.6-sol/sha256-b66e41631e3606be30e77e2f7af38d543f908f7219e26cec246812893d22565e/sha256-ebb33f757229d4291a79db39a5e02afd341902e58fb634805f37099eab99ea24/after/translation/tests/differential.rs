use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

type HashFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type SiphashFn = unsafe extern "C" fn(c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

struct Api {
    _library: Library,
    hash: HashFn,
    siphash: SiphashFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let hash = unsafe {
            *library
                .get::<HashFn>(b"stbds_hash_bytes\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load stbds_hash_bytes from {}: {error}",
                        path.display()
                    )
                })
        };
        let siphash = unsafe {
            *library
                .get::<SiphashFn>(b"siphash\0")
                .unwrap_or_else(|error| {
                    panic!("failed to load siphash from {}: {error}", path.display())
                })
        };
        Self {
            _library: library,
            hash,
            siphash,
        }
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn usize(&mut self) -> usize {
        self.next_u64() as usize
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            let random = self.next_u64().to_ne_bytes();
            chunk.copy_from_slice(&random[..chunk.len()]);
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let libraries: Vec<_> = fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .map(|entry| entry.expect("failed to read C build entry").path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("libharvest-work-"))
        })
        .collect();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {} but found {libraries:?}",
        build_dir.display()
    );
    libraries[0].clone()
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libsiphash_lib.so")
}

unsafe fn load_apis() -> (Api, Api) {
    (unsafe { Api::load(&c_library_path()) }, unsafe {
        Api::load(&rust_library_path())
    })
}

fn seed_for(iteration: usize, rng: &mut Rng) -> usize {
    match iteration {
        0 => 0,
        1 => usize::MAX,
        2 => 1usize << (usize::BITS - 1),
        _ => rng.usize(),
    }
}

unsafe fn compare_hash_case(
    c: &Api,
    rust: &Api,
    row: usize,
    iteration: usize,
    bytes: &mut [u8],
    seed: usize,
) {
    let pointer = bytes.as_mut_ptr().cast();
    let c_result = unsafe { (c.hash)(pointer, bytes.len(), seed) };
    let rust_result = unsafe { (rust.hash)(pointer, bytes.len(), seed) };
    assert_eq!(
        rust_result,
        c_result,
        "CONFIGS.md row {row}, iteration {iteration}, len {}, seed {seed:#x}, input {:02x?}",
        bytes.len(),
        bytes
    );
}

#[test]
fn hash_configuration_rows_1_through_24_match() {
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Rng::new(0x7d1f_4a89_3c26_b507);

    for full_blocks in 0..=1 {
        for tail in 0..8 {
            let row = full_blocks * 8 + tail + 1;
            let len = full_blocks * size_of::<usize>() + tail;
            for iteration in 0..512 {
                let mut bytes = vec![0; len];
                rng.fill(&mut bytes);
                let seed = seed_for(iteration, &mut rng);
                unsafe {
                    compare_hash_case(&c, &rust, row, iteration, &mut bytes, seed);
                }
            }
        }
    }

    for tail in 0..8 {
        let row = 17 + tail;
        for iteration in 0..512 {
            let full_blocks = 2 + (rng.usize() % 31);
            let len = full_blocks * size_of::<usize>() + tail;
            let mut bytes = vec![0; len];
            rng.fill(&mut bytes);
            let seed = seed_for(iteration, &mut rng);
            unsafe {
                compare_hash_case(&c, &rust, row, iteration, &mut bytes, seed);
            }
        }
    }
}

unsafe fn capture_stdout(function: SiphashFn, init: c_int) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let mut pipe_fds = [-1; 2];

    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    unsafe { function(init) };
    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut reader = unsafe { fs::File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

fn compare_siphash_case(c: &Api, rust: &Api, row: usize, init: c_int) {
    let c_output = unsafe { capture_stdout(c.siphash, init) };
    let rust_output = unsafe { capture_stdout(rust.siphash, init) };
    assert_eq!(rust_output, c_output, "CONFIGS.md row {row}, init {init}");
}

#[test]
fn siphash_configuration_rows_25_through_27_match() {
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Rng::new(0xc4b3_9a72_1e6d_508f);

    for init in [0, 1, 127, 128, 191, 192] {
        compare_siphash_case(&c, &rust, 25, init);
    }
    for _ in 0..32 {
        compare_siphash_case(&c, &rust, 25, (rng.next_u64() % 193) as c_int);
    }

    for init in [193, 194, 254, 255] {
        compare_siphash_case(&c, &rust, 26, init);
    }
    for _ in 0..32 {
        compare_siphash_case(&c, &rust, 26, 193 + (rng.next_u64() % 63) as c_int);
    }

    for init in [-1, -2, -63, -64, -127, -128, -255, -256] {
        compare_siphash_case(&c, &rust, 27, init);
    }
    for _ in 0..32 {
        compare_siphash_case(&c, &rust, 27, -1 - (rng.next_u64() % 1_000_000) as c_int);
    }
}

#[test]
fn generic_zero_length_boundaries_g1_and_g2_match() {
    let (c, rust) = unsafe { load_apis() };

    let c_null = unsafe { (c.hash)(std::ptr::null_mut(), 0, 0) };
    let rust_null = unsafe { (rust.hash)(std::ptr::null_mut(), 0, 0) };
    assert_eq!(rust_null, c_null, "ERRORS.md generic row G1");

    let mut byte = 0xa5u8;
    for seed in [0, 1, usize::MAX, 1usize << (usize::BITS - 1)] {
        let pointer = (&mut byte as *mut u8).cast();
        let c_nonnull = unsafe { (c.hash)(pointer, 0, seed) };
        let rust_nonnull = unsafe { (rust.hash)(pointer, 0, seed) };
        assert_eq!(
            rust_nonnull, c_nonnull,
            "ERRORS.md generic row G2, seed {seed:#x}"
        );
    }
}

#[test]
fn ffi_crash_child() {
    let Ok(which_library) = std::env::var("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let length = match std::env::var("DIFF_CRASH_LENGTH")
        .expect("DIFF_CRASH_LENGTH")
        .as_str()
    {
        "one" => 1,
        "max" => usize::MAX,
        value => panic!("unknown DIFF_CRASH_LENGTH {value}"),
    };
    let path = match which_library.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        value => panic!("unknown DIFF_CRASH_LIBRARY {value}"),
    };
    let api = unsafe { Api::load(&path) };
    let _ = unsafe { (api.hash)(std::ptr::null_mut(), length, 0) };
    panic!("invalid pointer unexpectedly returned");
}

fn crash_status(library: &str, length: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("ffi_crash_child")
        .arg("--nocapture")
        .env("DIFF_CRASH_LIBRARY", library)
        .env("DIFF_CRASH_LENGTH", length)
        .status()
        .unwrap_or_else(|error| panic!("failed to run crash child: {error}"))
}

#[test]
fn generic_invalid_pointer_boundaries_g3_and_g4_match() {
    for (row, length) in [("G3", "one"), ("G4", "max")] {
        let c_status = crash_status("c", length);
        let rust_status = crash_status("rust", length);
        assert!(!c_status.success(), "C {row} unexpectedly succeeded");
        assert!(!rust_status.success(), "Rust {row} unexpectedly succeeded");
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "ERRORS.md generic row {row}: C={c_status:?}, Rust={rust_status:?}"
        );
        assert_eq!(
            rust_status.code(),
            c_status.code(),
            "ERRORS.md generic row {row}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}
