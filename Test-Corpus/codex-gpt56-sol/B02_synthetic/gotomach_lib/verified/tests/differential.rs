use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::ptr;

type Operation = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;
type Gotomach = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type FaultConfigure = unsafe extern "C" fn(c_int, usize);
type FaultDisable = unsafe extern "C" fn();

const FAULT_RETURN_NULL: c_int = 1;
const FAULT_ZERO_STATUS: c_int = 2;
const FAULT_FILL_CAPACITY: c_int = 3;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        Self {
            c: unsafe { Library::new(c_library_path()) }.expect("load C shared library"),
            rust: unsafe { Library::new(rust_library_path()) }.expect("load Rust shared library"),
        }
    }

    unsafe fn operation(
        &self,
        symbol: &[u8],
        value: c_int,
        unused: c_int,
        context: *mut c_void,
    ) -> (c_int, c_int) {
        let c = unsafe { self.c.get::<Operation>(symbol) }.expect("load C operation");
        let rust = unsafe { self.rust.get::<Operation>(symbol) }.expect("load Rust operation");
        (unsafe { c(value, unused, context) }, unsafe {
            rust(value, unused, context)
        })
    }

    unsafe fn gotomach(
        &self,
        iterations: c_int,
        seed: c_int,
        mode: c_int,
        threshold: c_int,
    ) -> (c_int, c_int) {
        let c = unsafe { self.c.get::<Gotomach>(b"gotomach\0") }.expect("load C gotomach");
        let rust = unsafe { self.rust.get::<Gotomach>(b"gotomach\0") }.expect("load Rust gotomach");
        (unsafe { c(iterations, seed, mode, threshold) }, unsafe {
            rust(iterations, seed, mode, threshold)
        })
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn range(&mut self, start: i32, end: i32) -> i32 {
        start + (self.next_u32() % (end - start) as u32) as i32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/debug/libgotomach_lib.so")
}

fn build_fault_malloc() -> PathBuf {
    let output = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libfault_malloc_test.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(manifest_dir().join("tests/fault_malloc.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .expect("compile malloc fault-injection shim");
    assert!(
        status.success(),
        "failed to compile malloc fault-injection shim"
    );
    output
}

fn assert_pair(row: usize, input: impl std::fmt::Debug, pair: (c_int, c_int)) {
    assert_eq!(
        pair.0, pair.1,
        "surface row {row} diverged for input {input:?}: C={}, Rust={}",
        pair.0, pair.1
    );
}

fn transformed(seed: i32, mode: i32) -> i32 {
    match mode {
        1 => seed.wrapping_mul(2),
        2 => seed.wrapping_mul(3),
        _ => seed.wrapping_add(10),
    }
}

fn mode_for_row(mode_index: usize, rng: &mut Rng) -> i32 {
    match mode_index {
        0..=2 => mode_index as i32,
        _ => {
            const INVALID: [i32; 4] = [-1, 3, i32::MIN, i32::MAX];
            INVALID[(rng.next_u32() as usize) % INVALID.len()]
        }
    }
}

fn mixed_seed(mode_index: usize, rng: &mut Rng) -> i32 {
    match mode_index {
        1 => rng.range(500, 750),
        2 => rng.range(334, 400),
        _ => rng.range(0, 100),
    }
}

#[test]
fn valid_and_scalar_error_surface() {
    assert!(c_library_path().is_file(), "C shared library is not built");
    assert!(
        rust_library_path().is_file(),
        "Rust shared library is not built"
    );

    let libraries = unsafe { Libraries::load() };
    let mut rng = Rng::new(0x5eed_c0de_d15c_a11);

    let edges = [
        i32::MIN,
        i32::MIN + 1,
        -1_073_741_825,
        -715_827_883,
        -1,
        0,
        1,
        715_827_882,
        1_073_741_823,
        i32::MAX - 1,
        i32::MAX,
    ];
    for (row, symbol) in [
        (1, b"process_value\0".as_slice()),
        (2, b"double_value\0".as_slice()),
        (3, b"triple_value\0".as_slice()),
    ] {
        for &value in &edges {
            let unused = rng.next_i32();
            let context = (rng.next_u32() as usize | 1) as *mut c_void;
            let pair = unsafe { libraries.operation(symbol, value, unused, context) };
            assert_pair(row, (value, unused, context), pair);
        }
        for _ in 0..256 {
            let value = rng.next_i32();
            let unused = rng.next_i32();
            let context = if rng.next_u32() & 1 == 0 {
                ptr::null_mut()
            } else {
                (rng.next_u32() as usize | 1) as *mut c_void
            };
            let pair = unsafe { libraries.operation(symbol, value, unused, context) };
            assert_pair(row, (value, unused, context), pair);
        }
    }

    for mode_index in 0..4 {
        for _ in 0..24 {
            let mode = mode_for_row(mode_index, &mut rng);
            let seed = rng.range(0, 65_536);
            let threshold = rng.next_i32();
            let pair = unsafe { libraries.gotomach(0, seed, mode, threshold) };
            assert_pair(4 + mode_index, (0, seed, mode, threshold), pair);
        }

        for accepted in [false, true] {
            let row = if accepted {
                12 + mode_index
            } else {
                8 + mode_index
            };
            for _ in 0..24 {
                let mode = mode_for_row(mode_index, &mut rng);
                let seed = rng.range(0, 65_536);
                let value = transformed(seed, mode);
                let threshold = if accepted {
                    value.checked_add(1).unwrap()
                } else {
                    value
                };
                let pair = unsafe { libraries.gotomach(1, seed, mode, threshold) };
                assert_pair(row, (1, seed, mode, threshold), pair);
            }
        }

        for _ in 0..16 {
            let mode = mode_for_row(mode_index, &mut rng);
            let seed = rng.range(0, 65_536);
            let iterations = rng.range(2, 257);
            let pair = unsafe { libraries.gotomach(iterations, seed, mode, i32::MIN) };
            assert_pair(16 + mode_index, (iterations, seed, mode, i32::MIN), pair);
        }

        for _ in 0..16 {
            let mode = mode_for_row(mode_index, &mut rng);
            let seed = mixed_seed(mode_index, &mut rng);
            let threshold = if mode_index == 0 || mode_index == 3 {
                500
            } else {
                1000
            };
            let pair = unsafe { libraries.gotomach(128, seed, mode, threshold) };
            assert_pair(20 + mode_index, (128, seed, mode, threshold), pair);
        }

        for _ in 0..16 {
            let mode = mode_for_row(mode_index, &mut rng);
            let seed = rng.range(0, 65_536);
            let iterations = rng.range(2, 257);
            let pair = unsafe { libraries.gotomach(iterations, seed, mode, i32::MAX) };
            assert_pair(24 + mode_index, (iterations, seed, mode, i32::MAX), pair);
        }

        for _ in 0..8 {
            let mode = mode_for_row(mode_index, &mut rng);
            let seed = rng.range(0, 65_536);
            let pair = unsafe { libraries.gotomach(65_535, seed, mode, i32::MAX) };
            assert_pair(28 + mode_index, (65_535, seed, mode, i32::MAX), pair);
        }

        for _ in 0..8 {
            let mode = mode_for_row(mode_index, &mut rng);
            let seed = rng.range(0, 65_536);
            let pair = unsafe { libraries.gotomach(65_535, seed, mode, i32::MIN) };
            assert_pair(32 + mode_index, (65_535, seed, mode, i32::MIN), pair);
        }
    }

    for &iterations in &[i32::MIN, -65_536, -1] {
        let pair = unsafe { libraries.gotomach(iterations, 0, 0, 0) };
        assert_pair(5, iterations, pair);
        assert_eq!(pair.0, -1);
    }
    for &iterations in &[65_536, 65_537, i32::MAX] {
        let pair = unsafe { libraries.gotomach(iterations, 0, 0, 0) };
        assert_pair(6, iterations, pair);
        assert_eq!(pair.0, -1);
    }
    for &seed in &[i32::MIN, -65_536, -1] {
        let pair = unsafe { libraries.gotomach(0, seed, 0, 0) };
        assert_pair(7, seed, pair);
        assert_eq!(pair.0, -2);
    }
    for &seed in &[65_536, 65_537, i32::MAX] {
        let pair = unsafe { libraries.gotomach(0, seed, 0, 0) };
        assert_pair(8, seed, pair);
        assert_eq!(pair.0, -2);
    }
}

#[test]
fn fault_injected_error_surface() {
    if std::env::var_os("FAULT_MALLOC_PRELOADED").is_none() {
        let fault_malloc = build_fault_malloc();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "fault_injected_error_surface", "--nocapture"])
            .env("FAULT_MALLOC_PRELOADED", "1")
            .env("FAULT_MALLOC_SO", &fault_malloc)
            .env("LD_PRELOAD", &fault_malloc)
            .status()
            .expect("spawn fault-injected test process");
        assert!(status.success(), "fault-injected child test failed");
        return;
    }

    let libraries = unsafe { Libraries::load() };
    let fault_malloc = std::env::var_os("FAULT_MALLOC_SO").expect("malloc shim path");
    let shim = unsafe { Library::new(PathBuf::from(fault_malloc)) }.expect("load malloc shim");
    let configure = unsafe { shim.get::<FaultConfigure>(b"fault_malloc_configure\0") }
        .expect("load fault configure");
    let disable =
        unsafe { shim.get::<FaultDisable>(b"fault_malloc_disable\0") }.expect("load fault disable");

    // Prime stdio before allocation counting so printf/puts buffering is stable.
    let _ = unsafe { libraries.gotomach(-1, 0, 0, 0) };

    let cases = [
        (3, FAULT_RETURN_NULL, 1, -3),
        (4, FAULT_RETURN_NULL, 2, -3),
        (9, FAULT_RETURN_NULL, 3, -4),
        (1, FAULT_ZERO_STATUS, 3, -5),
        (10, FAULT_ZERO_STATUS, 3, -5),
        (2, FAULT_FILL_CAPACITY, 3, -6),
        (11, FAULT_FILL_CAPACITY, 3, -6),
    ];

    let c = unsafe { libraries.c.get::<Gotomach>(b"gotomach\0") }.unwrap();
    let rust = unsafe { libraries.rust.get::<Gotomach>(b"gotomach\0") }.unwrap();
    for (row, action, allocation, expected) in cases {
        unsafe { configure(action, allocation) };
        let c_result = unsafe { c(2, 7, 0, i32::MAX) };
        unsafe { disable() };

        unsafe { configure(action, allocation) };
        let rust_result = unsafe { rust(2, 7, 0, i32::MAX) };
        unsafe { disable() };

        assert_pair(row, (action, allocation), (c_result, rust_result));
        assert_eq!(c_result, expected, "C did not reach error row {row}");
    }
}
