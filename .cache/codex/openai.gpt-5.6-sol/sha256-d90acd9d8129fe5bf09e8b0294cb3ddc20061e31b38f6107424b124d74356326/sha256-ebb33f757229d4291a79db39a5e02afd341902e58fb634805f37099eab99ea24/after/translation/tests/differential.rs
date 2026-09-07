use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type StaticSum = unsafe extern "C" fn(c_int) -> c_int;
type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const RANDOM_CASES: usize = 64;
const SEQUENCE_LENGTH: usize = 128;

static NEXT_COPY_ID: AtomicU64 = AtomicU64::new(0);
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct LoadedPair {
    c_library: Option<Library>,
    rust_library: Option<Library>,
    c_static_sum: StaticSum,
    rust_static_sum: StaticSum,
    c_driver: Driver,
    rust_driver: Driver,
    directory: PathBuf,
}

impl LoadedPair {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_source = manifest.join("../c_src/build/libStaticLoop.so");
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let rust_source = manifest
            .join("target")
            .join(profile)
            .join("libStaticLoop.so");

        assert!(
            c_source.is_file(),
            "missing C shared library at {}; build c_src first",
            c_source.display()
        );
        assert!(
            rust_source.is_file(),
            "missing Rust shared library at {}; build the crate first",
            rust_source.display()
        );

        let copy_id = NEXT_COPY_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "staticloop-differential-{}-{copy_id}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let c_copy = directory.join("libStaticLoop_c.so");
        let rust_copy = directory.join("libStaticLoop_rust.so");
        fs::copy(&c_source, &c_copy).unwrap();
        fs::copy(&rust_source, &rust_copy).unwrap();

        let c_library = unsafe { Library::new(&c_copy) }.unwrap();
        let rust_library = unsafe { Library::new(&rust_copy) }.unwrap();

        let c_static_sum = unsafe { *c_library.get::<StaticSum>(b"static_sum\0").unwrap() };
        let rust_static_sum = unsafe { *rust_library.get::<StaticSum>(b"static_sum\0").unwrap() };
        let c_driver = unsafe { *c_library.get::<Driver>(b"driver\0").unwrap() };
        let rust_driver = unsafe { *rust_library.get::<Driver>(b"driver\0").unwrap() };

        Self {
            c_library: Some(c_library),
            rust_library: Some(rust_library),
            c_static_sum,
            rust_static_sum,
            c_driver,
            rust_driver,
            directory,
        }
    }

    fn compare_sum(&self, update: i32) -> i32 {
        let c_result = unsafe { (self.c_static_sum)(update) };
        let rust_result = unsafe { (self.rust_static_sum)(update) };
        assert_eq!(
            rust_result, c_result,
            "static_sum diverged for update {update}"
        );
        c_result
    }

    fn compare_driver(&self, stride: i32) {
        let c_output = capture_stdout(&self.directory, "c-output", || unsafe {
            (self.c_driver)(stride)
        });
        let rust_output = capture_stdout(&self.directory, "rust-output", || unsafe {
            (self.rust_driver)(stride)
        });
        assert_eq!(
            rust_output, c_output,
            "driver stdout diverged for stride {stride}"
        );
    }
}

impl Drop for LoadedPair {
    fn drop(&mut self) {
        self.c_library.take();
        self.rust_library.take();
        fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn capture_stdout(directory: &Path, name: &str, operation: impl FnOnce()) -> Vec<u8> {
    let path = directory.join(name);
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(path)
        .unwrap();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(
        unsafe { dup2(output.as_raw_fd(), STDOUT_FILENO) },
        STDOUT_FILENO
    );

    operation();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    output.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).unwrap();
    bytes
}

#[derive(Clone)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        assert_ne!(seed, 0);
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    fn next_nonzero_i32(&mut self) -> i32 {
        loop {
            let value = self.next_i32();
            if value != 0 {
                return value;
            }
        }
    }
}

fn boundary_values() -> [i32; 9] {
    [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
    ]
}

// CONFIGS.md row 1.
#[test]
fn fresh_static_sum_matches_for_full_integer_shapes() {
    for update in boundary_values() {
        LoadedPair::load().compare_sum(update);
    }

    let mut rng = XorShift64::new(0x5a17_1c0d_74e2_39b1);
    for _ in 0..RANDOM_CASES {
        LoadedPair::load().compare_sum(rng.next_i32());
    }
}

// CONFIGS.md row 2.
#[test]
fn static_sum_stateful_sequences_match() {
    let mut rng = XorShift64::new(0xc4a7_09e1_5d38_62bf);
    for case in 0..RANDOM_CASES {
        let pair = LoadedPair::load();
        pair.compare_sum(rng.next_nonzero_i32());

        for index in 0..SEQUENCE_LENGTH {
            let update = if index < boundary_values().len() {
                boundary_values()[index]
            } else if index % 17 == 0 {
                0
            } else {
                rng.next_i32()
            };
            pair.compare_sum(update);
        }

        assert!(case < RANDOM_CASES);
    }
}

// CONFIGS.md row 3. Zero is a singleton input configuration.
#[test]
fn fresh_driver_zero_stride_matches() {
    let _stdout_guard = STDOUT_LOCK.lock().unwrap();
    LoadedPair::load().compare_driver(0);
}

// CONFIGS.md row 4.
#[test]
fn fresh_driver_randomized_nonzero_strides_match() {
    let _stdout_guard = STDOUT_LOCK.lock().unwrap();
    for stride in boundary_values().into_iter().filter(|value| *value != 0) {
        LoadedPair::load().compare_driver(stride);
    }

    let mut rng = XorShift64::new(0x8f63_d2a9_47bc_105e);
    for _ in 0..RANDOM_CASES {
        LoadedPair::load().compare_driver(rng.next_nonzero_i32());
    }
}

// CONFIGS.md row 5.
#[test]
fn preloaded_driver_zero_stride_matches() {
    let _stdout_guard = STDOUT_LOCK.lock().unwrap();
    let mut rng = XorShift64::new(0x31d8_b7f4_a20e_695c);
    for _ in 0..RANDOM_CASES {
        let pair = LoadedPair::load();
        pair.compare_sum(rng.next_nonzero_i32());
        pair.compare_driver(0);
    }
}

// CONFIGS.md row 6.
#[test]
fn preloaded_driver_randomized_nonzero_strides_match() {
    let _stdout_guard = STDOUT_LOCK.lock().unwrap();
    let mut rng = XorShift64::new(0xe7c1_4a90_2db5_683f);
    for _ in 0..RANDOM_CASES {
        let pair = LoadedPair::load();
        pair.compare_sum(rng.next_nonzero_i32());
        pair.compare_driver(rng.next_nonzero_i32());
    }
}

// CONFIGS.md row 7.
#[test]
fn repeated_driver_calls_preserve_matching_state() {
    let _stdout_guard = STDOUT_LOCK.lock().unwrap();
    let mut rng = XorShift64::new(0x9b42_f03d_76a1_c85e);
    for _ in 0..RANDOM_CASES {
        let pair = LoadedPair::load();
        for index in 0..32 {
            let stride = if index % 8 == 0 {
                0
            } else if index < boundary_values().len() {
                boundary_values()[index]
            } else {
                rng.next_i32()
            };
            pair.compare_driver(stride);
        }
    }
}
