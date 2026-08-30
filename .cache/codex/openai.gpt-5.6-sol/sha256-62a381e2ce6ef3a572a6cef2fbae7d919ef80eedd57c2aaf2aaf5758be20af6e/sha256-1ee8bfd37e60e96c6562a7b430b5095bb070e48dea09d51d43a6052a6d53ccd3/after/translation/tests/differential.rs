use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(c_int, c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

const STDOUT_FILENO: c_int = 1;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn rust_library_path() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let target_dir = manifest_dir.join("target").join(profile);
    let direct = target_dir.join("libdriver.so");
    if direct.is_file() {
        return direct;
    }

    let deps = target_dir.join("deps").join("libdriver.so");
    if deps.is_file() {
        return deps;
    }

    let release = manifest_dir.join("target/release/libdriver.so");
    assert!(
        release.is_file(),
        "Rust cdylib not found at {}, {}, or {}",
        direct.display(),
        deps.display(),
        release.display()
    );
    release
}

fn capture_stdout(driver: Driver, inputs: &[(c_int, c_int)]) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture lock poisoned");
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{id}.out",
        std::process::id()
    ));
    let mut output = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create stdout capture file");

    unsafe {
        assert_eq!(
            fflush(std::ptr::null_mut()),
            0,
            "flush stdout before capture"
        );
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(output.as_raw_fd(), STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );

        for &(x, y) in inputs {
            driver(x, y);
        }

        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    output
        .seek(SeekFrom::Start(0))
        .expect("rewind capture file");
    let mut bytes = Vec::new();
    output
        .read_to_end(&mut bytes)
        .expect("read captured stdout");
    drop(output);
    remove_file(path).expect("remove stdout capture file");
    bytes
}

fn input_corpus() -> Vec<(c_int, c_int)> {
    const EDGES: [c_int; 9] = [
        c_int::MIN,
        c_int::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    let mut inputs = Vec::with_capacity(EDGES.len() * EDGES.len() + 20_000);
    for x in EDGES {
        for y in EDGES {
            inputs.push((x, y));
        }
    }

    // Fixed-seed SplitMix64 supplies reproducible, full-width integer pairs.
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..20_000 {
        let x = splitmix64(&mut state) as u32 as c_int;
        let y = splitmix64(&mut state) as u32 as c_int;
        inputs.push((x, y));
    }
    inputs
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[test]
fn driver_matches_c_for_full_width_integer_inputs() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest_dir.join("../c_src/build/libdriver.so");
    let rust_path = rust_library_path();

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
    let c_driver: Symbol<Driver> =
        unsafe { c_library.get(b"driver\0") }.expect("load C driver export");
    let rust_driver: Symbol<Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("load Rust driver export");

    let inputs = input_corpus();
    let c_output = capture_stdout(*c_driver, &inputs);
    let rust_output = capture_stdout(*rust_driver, &inputs);

    assert_eq!(
        rust_output, c_output,
        "stdout differs for edge cases and fixed-seed randomized inputs"
    );
}
