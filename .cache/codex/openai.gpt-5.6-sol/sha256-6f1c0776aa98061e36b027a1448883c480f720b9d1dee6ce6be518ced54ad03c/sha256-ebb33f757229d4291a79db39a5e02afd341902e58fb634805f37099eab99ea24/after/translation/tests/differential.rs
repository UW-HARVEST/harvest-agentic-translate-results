use libloading::Library;
use std::collections::BTreeSet;
use std::env;
use std::ffi::{c_char, c_int};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;

type ShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
type ProcessString = unsafe extern "C" fn(*const c_char) -> c_int;
type ApplyBitmask = unsafe extern "C" fn(c_int, c_int) -> c_int;
type InitMatrix = unsafe extern "C" fn(*mut [c_int; 4]);
type CompareAllocations = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Arity2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Arity3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type Arity4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type Arity = unsafe extern "C" fn(c_int, *mut c_int) -> c_int;

struct Api {
    _library: Library,
    shift_array: ShiftArray,
    process_string: ProcessString,
    apply_bitmask: ApplyBitmask,
    init_matrix: InitMatrix,
    compare_allocations: CompareAllocations,
    arity2: Arity2,
    arity3: Arity3,
    arity4: Arity4,
    arity: Arity,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
            *unsafe { library.get::<T>(name) }
                .unwrap_or_else(|error| panic!("failed to load {:?}: {error}", name))
        }

        Self {
            shift_array: unsafe { symbol(&library, b"shift_array\0") },
            process_string: unsafe { symbol(&library, b"process_string\0") },
            apply_bitmask: unsafe { symbol(&library, b"apply_bitmask\0") },
            init_matrix: unsafe { symbol(&library, b"init_matrix\0") },
            compare_allocations: unsafe { symbol(&library, b"compare_allocations\0") },
            arity2: unsafe { symbol(&library, b"arity2\0") },
            arity3: unsafe { symbol(&library, b"arity3\0") },
            arity4: unsafe { symbol(&library, b"arity4\0") },
            arity: unsafe { symbol(&library, b"arity\0") },
            _library: library,
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build = crate_root().join("../c_src/build");
    let mut libraries: Vec<_> = fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    libraries.sort();
    assert_eq!(libraries.len(), 1, "expected exactly one C shared library");
    libraries.remove(0)
}

fn rust_library_path() -> PathBuf {
    let release = crate_root().join("target/release/libarity_lib.so");
    assert!(
        release.exists(),
        "release cdylib is missing; run cargo build --release first"
    );
    release
}

fn load_apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn nonzero_i32(&mut self) -> i32 {
        loop {
            let value = self.next_i32();
            if value != 0 {
                return value;
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum P1Class {
    P0,
    P1,
    P2,
    P3,
    Z,
    N0,
    ND,
}

const P1_CLASSES: [P1Class; 7] = [
    P1Class::P0,
    P1Class::P1,
    P1Class::P2,
    P1Class::P3,
    P1Class::Z,
    P1Class::N0,
    P1Class::ND,
];

fn param1_for(class: P1Class, rng: &mut Rng) -> i32 {
    match class {
        P1Class::P0 => ((rng.next_u32() % 536_870_911 + 1) * 4) as i32,
        P1Class::P1 => ((rng.next_u32() % 536_870_912) * 4 + 1) as i32,
        P1Class::P2 => ((rng.next_u32() % 536_870_912) * 4 + 2) as i32,
        P1Class::P3 => ((rng.next_u32() % 536_870_912) * 4 + 3) as i32,
        P1Class::Z => 0,
        P1Class::N0 => {
            let magnitude = (rng.next_u32() % 536_870_912 + 1).wrapping_mul(4);
            (magnitude as i32).wrapping_neg()
        }
        P1Class::ND => {
            let mut value = (rng.next_u32() | 0x8000_0000) as i32;
            if value % 4 == 0 {
                value = value.wrapping_add(1);
            }
            value
        }
    }
}

fn len_with_low_byte(low: u8, canonical: bool, rng: &mut Rng) -> i32 {
    if canonical {
        i32::from(low)
    } else {
        (rng.next_i32() & !0xff) | i32::from(low)
    }
}

unsafe fn stable_compare_allocations(
    c_api: &Api,
    rust_api: &Api,
    val1: i32,
    val2: i32,
) -> (i32, i32) {
    let expected = unsafe { (c_api.compare_allocations)(val1, val2) };
    let _restore_allocator_order = unsafe { (c_api.compare_allocations)(val1, val2) };
    let actual = unsafe { (rust_api.compare_allocations)(val1, val2) };
    (expected, actual)
}

unsafe fn stable_arity2(c_function: Arity2, rust_function: Arity2, p1: i32, p2: i32) -> (i32, i32) {
    let expected = unsafe { c_function(p1, p2) };
    let _restore_allocator_order = unsafe { c_function(p1, p2) };
    let actual = unsafe { rust_function(p1, p2) };
    (expected, actual)
}

unsafe fn stable_arity3(
    c_function: Arity3,
    rust_function: Arity3,
    p1: i32,
    p2: i32,
    p3: i32,
) -> (i32, i32) {
    let expected = unsafe { c_function(p1, p2, p3) };
    let _restore_allocator_order = unsafe { c_function(p1, p2, p3) };
    let actual = unsafe { rust_function(p1, p2, p3) };
    (expected, actual)
}

unsafe fn stable_arity4(
    c_function: Arity4,
    rust_function: Arity4,
    p1: i32,
    p2: i32,
    p3: i32,
    p4: i32,
) -> (i32, i32) {
    let expected = unsafe { c_function(p1, p2, p3, p4) };
    let _restore_allocator_order = unsafe { c_function(p1, p2, p3, p4) };
    let actual = unsafe { rust_function(p1, p2, p3, p4) };
    (expected, actual)
}

unsafe fn stable_arity(
    c_function: Arity,
    rust_function: Arity,
    len: i32,
    params: &mut [i32; 4],
) -> (i32, i32) {
    let expected = unsafe { c_function(len, params.as_mut_ptr()) };
    let _restore_allocator_order = unsafe { c_function(len, params.as_mut_ptr()) };
    let actual = unsafe { rust_function(len, params.as_mut_ptr()) };
    (expected, actual)
}

#[test]
fn valid_low_level_configurations_c01_to_c16() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x9f37_79b9_7f4a_7c15);

    for iteration in 0..128 {
        let original: Vec<i32> = (0..16).map(|_| rng.next_i32()).collect();

        let mut c_values = original.clone();
        let mut rust_values = original.clone();
        let size = [0, -1, i32::MIN][iteration % 3];
        let positions = rng.next_i32();
        unsafe {
            (c_api.shift_array)(c_values.as_mut_ptr(), size, positions);
            (rust_api.shift_array)(rust_values.as_mut_ptr(), size, positions);
        }
        assert_eq!(c_values, rust_values, "C01 iteration {iteration}");

        for (row, positions) in [("C02", -1), ("C02", 0), ("C03", 1), ("C03", i32::MAX)] {
            let mut c_values = original[..1].to_vec();
            let mut rust_values = c_values.clone();
            unsafe {
                (c_api.shift_array)(c_values.as_mut_ptr(), 1, positions);
                (rust_api.shift_array)(rust_values.as_mut_ptr(), 1, positions);
            }
            assert_eq!(c_values, rust_values, "{row} iteration {iteration}");
        }

        let size = 2 + (rng.next_u32() % 15) as i32;
        for (row, positions) in [
            ("C04", 0),
            ("C04", -1),
            ("C05", 1 + (rng.next_u32() % (size as u32 - 1)) as i32),
            ("C06", size),
            ("C06", i32::MAX),
        ] {
            let mut c_values = original.clone();
            let mut rust_values = original.clone();
            unsafe {
                (c_api.shift_array)(c_values.as_mut_ptr(), size, positions);
                (rust_api.shift_array)(rust_values.as_mut_ptr(), size, positions);
            }
            assert_eq!(c_values, rust_values, "{row} iteration {iteration}");
        }
    }

    let empty = [0_u8];
    let c_empty = unsafe { (c_api.process_string)(empty.as_ptr().cast()) };
    let rust_empty = unsafe { (rust_api.process_string)(empty.as_ptr().cast()) };
    assert_eq!(c_empty, rust_empty, "C07");

    for iteration in 0..128 {
        let length = 1 + (rng.next_u32() % 1024) as usize;
        let mut string: Vec<u8> = (0..length)
            .map(|_| (rng.next_u32() % 255 + 1) as u8)
            .collect();
        string.push(0);
        let expected = unsafe { (c_api.process_string)(string.as_ptr().cast()) };
        let actual = unsafe { (rust_api.process_string)(string.as_ptr().cast()) };
        assert_eq!(expected, actual, "C08 iteration {iteration}");
    }

    for (row, operation) in [
        ("C09", 0),
        ("C10", 1),
        ("C11", 2),
        ("C12", 3),
        ("C13", -1),
        ("C13", 4),
        ("C13", i32::MIN),
        ("C13", i32::MAX),
    ] {
        for iteration in 0..128 {
            let value = rng.next_i32();
            let expected = unsafe { (c_api.apply_bitmask)(value, operation) };
            let actual = unsafe { (rust_api.apply_bitmask)(value, operation) };
            assert_eq!(expected, actual, "{row} iteration {iteration}");
        }
    }

    for iteration in 0..128 {
        let mut c_matrix = [[rng.next_i32(); 4]; 3];
        let mut rust_matrix = c_matrix;
        unsafe {
            (c_api.init_matrix)(c_matrix.as_mut_ptr());
            (rust_api.init_matrix)(rust_matrix.as_mut_ptr());
        }
        assert_eq!(c_matrix, rust_matrix, "C14 iteration {iteration}");
    }

    for (row, positive) in [("C15", false), ("C16", true)] {
        for iteration in 0..128 {
            let val1 = if positive {
                (rng.next_u32() & 0x7fff_ffff).max(1) as i32
            } else {
                (rng.next_u32() | 0x8000_0000) as i32
            };
            let val2 = rng.next_i32();
            let (expected, actual) =
                unsafe { stable_compare_allocations(&c_api, &rust_api, val1, val2) };
            assert_eq!(expected, actual, "{row} iteration {iteration}");
        }
    }
}

#[test]
fn valid_arity2_configurations_c17_to_c23() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x17c2_23a2_5eed_0001);

    for (class_index, class) in P1_CLASSES.into_iter().enumerate() {
        let row = 17 + class_index;
        for iteration in 0..128 {
            let p1 = param1_for(class, &mut rng);
            let p2 = rng.next_i32();
            let (expected, actual) =
                unsafe { stable_arity2(c_api.arity2, rust_api.arity2, p1, p2) };
            assert_eq!(expected, actual, "C{row:02} arity2 iteration {iteration}");

            let len = len_with_low_byte(2, iteration % 2 == 0, &mut rng);
            let mut params = [p1, p2, rng.next_i32(), rng.next_i32()];
            let (expected, actual) =
                unsafe { stable_arity(c_api.arity, rust_api.arity, len, &mut params) };
            assert_eq!(expected, actual, "C{row:02} arity iteration {iteration}");
        }
    }
}

#[test]
fn valid_arity3_configurations_c24_to_c37() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x24c3_37a3_5eed_0002);

    for nonzero_p3 in [false, true] {
        for (class_index, class) in P1_CLASSES.into_iter().enumerate() {
            let row = 24 + usize::from(nonzero_p3) * 7 + class_index;
            for iteration in 0..128 {
                let p1 = param1_for(class, &mut rng);
                let p2 = rng.next_i32();
                let p3 = if nonzero_p3 { rng.nonzero_i32() } else { 0 };
                let (expected, actual) =
                    unsafe { stable_arity3(c_api.arity3, rust_api.arity3, p1, p2, p3) };
                assert_eq!(expected, actual, "C{row:02} arity3 iteration {iteration}");

                let len = len_with_low_byte(3, iteration % 2 == 0, &mut rng);
                let mut params = [p1, p2, p3, rng.next_i32()];
                let (expected, actual) =
                    unsafe { stable_arity(c_api.arity, rust_api.arity, len, &mut params) };
                assert_eq!(expected, actual, "C{row:02} arity iteration {iteration}");
            }
        }
    }
}

#[test]
fn valid_arity4_configurations_c38_to_c65() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x38c4_65a4_5eed_0003);

    for (nonzero_p3, nonzero_p4) in [(false, false), (true, false), (false, true), (true, true)] {
        let quadrant = match (nonzero_p3, nonzero_p4) {
            (false, false) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (true, true) => 3,
        };
        for (class_index, class) in P1_CLASSES.into_iter().enumerate() {
            let row = 38 + quadrant * 7 + class_index;
            for iteration in 0..128 {
                let p1 = param1_for(class, &mut rng);
                let p2 = rng.next_i32();
                let p3 = if nonzero_p3 { rng.nonzero_i32() } else { 0 };
                let p4 = if nonzero_p4 { rng.nonzero_i32() } else { 0 };
                let (expected, actual) =
                    unsafe { stable_arity4(c_api.arity4, rust_api.arity4, p1, p2, p3, p4) };
                assert_eq!(expected, actual, "C{row:02} arity4 iteration {iteration}");

                let low = match iteration % 4 {
                    0 => 4,
                    1 => 5,
                    2 => 255,
                    _ => 4 + (rng.next_u32() % 252) as u8,
                };
                let len = len_with_low_byte(low, iteration % 2 == 0, &mut rng);
                let mut params = [p1, p2, p3, p4];
                let (expected, actual) =
                    unsafe { stable_arity(c_api.arity, rust_api.arity, len, &mut params) };
                assert_eq!(expected, actual, "C{row:02} arity iteration {iteration}");
            }
        }
    }
}

#[test]
fn error_surface_e01_e02_e06_to_e09_and_boundaries() {
    let (c_api, rust_api) = load_apis();
    let original = [11, 22, 33, 44];

    for (row, size, positions) in [
        ("E01", 4, 0),
        ("E01", 4, -1),
        ("E01", 0, i32::MIN),
        ("E02", 4, 4),
        ("E02", 4, 5),
        ("E02", 1, i32::MAX),
        ("E02", i32::MAX, i32::MAX),
    ] {
        let mut c_values = original;
        let mut rust_values = original;
        unsafe {
            (c_api.shift_array)(c_values.as_mut_ptr(), size, positions);
            (rust_api.shift_array)(rust_values.as_mut_ptr(), size, positions);
        }
        assert_eq!(c_values, original, "{row} C rejection");
        assert_eq!(rust_values, original, "{row} Rust rejection");
    }

    for (row, operation) in [
        ("E06", -1),
        ("E06", i32::MIN),
        ("E07", 4),
        ("E07", i32::MAX),
    ] {
        for value in [i32::MIN, -1, 0, 1, i32::MAX] {
            let expected = unsafe { (c_api.apply_bitmask)(value, operation) };
            let actual = unsafe { (rust_api.apply_bitmask)(value, operation) };
            assert_eq!(expected, value, "{row} C identity");
            assert_eq!(actual, expected, "{row} Rust identity");
        }
    }

    for (row, low) in [("E08", 0_u8), ("E09", 1_u8)] {
        for high in [0, 0x100, -0x100, i32::MAX & !0xff, i32::MIN] {
            let len = high | i32::from(low);
            let expected = unsafe { (c_api.arity)(len, ptr::null_mut()) };
            let actual = unsafe { (rust_api.arity)(len, ptr::null_mut()) };
            assert_eq!(expected, -1, "{row} C result for len {len}");
            assert_eq!(actual, expected, "{row} Rust result for len {len}");
        }
    }

    unsafe {
        (c_api.shift_array)(ptr::null_mut(), 0, 0);
        (rust_api.shift_array)(ptr::null_mut(), 0, 0);
    }

    let mut params = [7, 8, 9, 10];
    for len in [5, 255, i32::MAX, -1] {
        let (expected, actual) =
            unsafe { stable_arity(c_api.arity, rust_api.arity, len, &mut params) };
        assert_eq!(expected, actual, "oversized/effective arity len {len}");
    }
}

fn compile_malloc_interposer() -> PathBuf {
    let output = crate_root().join("target/fail_malloc.so");
    let source = crate_root().join("tests/support/fail_malloc.c");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&output)
        .arg(&source)
        .status()
        .expect("failed to run cc for malloc interposer");
    assert!(status.success(), "failed to compile malloc interposer");
    output
}

#[test]
fn allocator_failure_paths_e03_to_e05() {
    if env::var_os("ALLOCATOR_FAILURE_CHILD").is_some() {
        return;
    }

    let interposer = compile_malloc_interposer();
    let status = Command::new(env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "allocator_failure_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("LD_PRELOAD", &interposer)
        .env("ALLOCATOR_FAILURE_CHILD", "1")
        .status()
        .expect("failed to run allocator failure child");
    assert!(status.success(), "allocator failure child failed: {status}");
}

#[test]
fn allocator_failure_child() {
    if env::var_os("ALLOCATOR_FAILURE_CHILD").is_none() {
        return;
    }

    type SetMode = unsafe extern "C" fn(c_int);
    let interposer_path = crate_root().join("target/fail_malloc.so");
    let interposer = unsafe { Library::new(&interposer_path) }.expect("load malloc interposer");
    let set_mode: SetMode =
        *unsafe { interposer.get::<SetMode>(b"fail_malloc_set\0") }.expect("load fail_malloc_set");
    let (c_api, rust_api) = load_apis();

    for (row, mode) in [("E03", 1), ("E04", 2), ("E05", 3)] {
        unsafe { set_mode(mode) };
        let expected = unsafe { (c_api.compare_allocations)(7, 9) };
        unsafe { set_mode(0) };

        unsafe { set_mode(mode) };
        let actual = unsafe { (rust_api.compare_allocations)(7, 9) };
        unsafe { set_mode(0) };

        assert_eq!(expected, -1, "{row} C allocation failure");
        assert_eq!(actual, expected, "{row} Rust allocation failure");
    }
}

fn run_crash_child(case: &str, implementation: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "null_pointer_crash_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NULL_CRASH_CASE", case)
        .env("NULL_CRASH_IMPLEMENTATION", implementation)
        .status()
        .unwrap_or_else(|error| {
            panic!("failed to run crash child {case}/{implementation}: {error}")
        })
}

#[test]
fn null_pointer_crashes_match_exactly() {
    if env::var_os("NULL_CRASH_CASE").is_some() {
        return;
    }

    for case in ["process_string", "shift_array", "init_matrix", "arity"] {
        let c_status = run_crash_child(case, "c");
        let rust_status = run_crash_child(case, "rust");
        assert!(!c_status.success(), "{case}: C unexpectedly succeeded");
        assert!(
            !rust_status.success(),
            "{case}: Rust unexpectedly succeeded"
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "{case}: termination signal differs (C={c_status}, Rust={rust_status})"
        );
        assert_eq!(
            c_status.code(),
            rust_status.code(),
            "{case}: exit code differs (C={c_status}, Rust={rust_status})"
        );
    }
}

#[test]
fn null_pointer_crash_child() {
    let Some(case) = env::var_os("NULL_CRASH_CASE") else {
        return;
    };
    let implementation = env::var("NULL_CRASH_IMPLEMENTATION").expect("implementation selector");
    let (c_api, rust_api) = load_apis();
    let api = if implementation == "c" {
        &c_api
    } else {
        &rust_api
    };

    unsafe {
        match case.to_str().expect("UTF-8 crash case") {
            "process_string" => {
                (api.process_string)(ptr::null());
            }
            "shift_array" => {
                (api.shift_array)(ptr::null_mut(), 2, 1);
            }
            "init_matrix" => {
                (api.init_matrix)(ptr::null_mut());
            }
            "arity" => {
                (api.arity)(2, ptr::null_mut());
            }
            other => panic!("unknown crash case {other}"),
        }
    }
}

fn defined_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run nm on {}: {error}", path.display()));
    assert!(output.status.success(), "nm failed on {}", path.display());
    String::from_utf8(output.stdout)
        .expect("nm output is UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

#[test]
fn phase_d_defined_symbol_parity_is_exact() {
    let c_symbols = defined_symbols(&c_library_path());
    let rust_symbols = defined_symbols(&rust_library_path());
    assert_eq!(c_symbols, rust_symbols);
}
