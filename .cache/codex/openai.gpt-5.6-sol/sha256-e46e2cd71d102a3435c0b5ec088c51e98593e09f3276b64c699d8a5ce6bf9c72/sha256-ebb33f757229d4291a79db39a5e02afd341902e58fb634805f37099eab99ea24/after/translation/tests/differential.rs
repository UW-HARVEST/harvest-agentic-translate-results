use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Copy)]
struct DataBlock {
    id: c_int,
    name: [c_char; 32],
    flags: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryBlock {
    data: *mut c_int,
    size: usize,
}

type CreateBlock = unsafe extern "C" fn(c_int, *const c_char, u8) -> DataBlock;
type AllocateBlock = unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock;
type FreeBlock = unsafe extern "C" fn(*mut MemoryBlock);
type ComputeHash = unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int;
type Betagamma = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn i32_between(&mut self, low: i32, high: i32) -> i32 {
        let width = (i64::from(high) - i64::from(low) + 1) as u32;
        low + (self.next_u32() % width) as i32
    }

    fn nonzero_byte(&mut self) -> u8 {
        (self.next_u32() % 255 + 1) as u8
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<_> = fs::read_dir(&build)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "so")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected one C shared library in {build:?}"
    );
    candidates.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libbetagamma_lib.so")
}

fn library_paths() -> [PathBuf; 2] {
    let paths = [c_library_path(), rust_library_path()];
    for path in &paths {
        assert!(path.is_file(), "missing shared library: {}", path.display());
    }
    paths
}

unsafe fn get<'a, T>(library: &'a Library, symbol: &[u8]) -> Symbol<'a, T> {
    unsafe { library.get(symbol).unwrap() }
}

fn initialized_name_bytes(block: &DataBlock, length: usize) -> &[u8] {
    unsafe { std::slice::from_raw_parts(block.name.as_ptr().cast::<u8>(), length + 1) }
}

fn compare_create_case(name: &[u8], id: i32, flags: u8) {
    assert_eq!(name.last(), Some(&0));
    unsafe {
        let c = Library::new(c_library_path()).unwrap();
        let rust = Library::new(rust_library_path()).unwrap();
        let c_create: Symbol<CreateBlock> = get(&c, b"create_block\0");
        let rust_create: Symbol<CreateBlock> = get(&rust, b"create_block\0");
        let c_result = c_create(id, name.as_ptr().cast(), flags);
        let rust_result = rust_create(id, name.as_ptr().cast(), flags);

        assert_eq!(c_result.id, rust_result.id);
        assert_eq!(c_result.flags, rust_result.flags);
        assert_eq!(
            initialized_name_bytes(&c_result, name.len() - 1),
            initialized_name_bytes(&rust_result, name.len() - 1)
        );
    }
}

#[test]
fn configs_c1_create_empty_name() {
    let mut rng = FixedRng::new(0xC1);
    for _ in 0..256 {
        compare_create_case(&[0], rng.next_u32() as i32, rng.next_u32() as u8);
    }
}

#[test]
fn configs_c2_create_variable_names() {
    let mut rng = FixedRng::new(0xC2);
    for _ in 0..1_024 {
        let length = (rng.next_u32() % 30 + 1) as usize;
        let mut name = Vec::with_capacity(length + 1);
        name.extend((0..length).map(|_| rng.nonzero_byte()));
        name.push(0);
        compare_create_case(&name, rng.next_u32() as i32, rng.next_u32() as u8);
    }
}

#[test]
fn configs_c3_create_full_name() {
    let mut rng = FixedRng::new(0xC3);
    for _ in 0..256 {
        let mut name: Vec<u8> = (0..31).map(|_| rng.nonzero_byte()).collect();
        name.push(0);
        compare_create_case(&name, rng.next_u32() as i32, rng.next_u32() as u8);
    }
}

fn compare_allocation(count: usize, init: i32) {
    unsafe {
        let c = Library::new(c_library_path()).unwrap();
        let rust = Library::new(rust_library_path()).unwrap();
        let c_allocate: Symbol<AllocateBlock> = get(&c, b"allocate_block\0");
        let rust_allocate: Symbol<AllocateBlock> = get(&rust, b"allocate_block\0");
        let c_free: Symbol<FreeBlock> = get(&c, b"free_block\0");
        let rust_free: Symbol<FreeBlock> = get(&rust, b"free_block\0");
        let c_block = c_allocate(count, init);
        let rust_block = rust_allocate(count, init);

        assert_eq!(c_block.is_null(), rust_block.is_null());
        if !c_block.is_null() {
            assert_eq!((*c_block).size, (*rust_block).size);
            assert_eq!((*c_block).data.is_null(), (*rust_block).data.is_null());
            if !(*c_block).data.is_null() {
                let c_data = std::slice::from_raw_parts((*c_block).data, count);
                let rust_data = std::slice::from_raw_parts((*rust_block).data, count);
                assert_eq!(c_data, rust_data);
            }
        }

        c_free(c_block);
        rust_free(rust_block);
    }
}

#[test]
fn configs_c4_allocate_zero() {
    let mut rng = FixedRng::new(0xC4);
    for _ in 0..256 {
        compare_allocation(0, rng.i32_between(-1_000_000, 1_000_000));
    }
}

#[test]
fn configs_c5_allocate_one() {
    let mut rng = FixedRng::new(0xC5);
    for _ in 0..256 {
        compare_allocation(1, rng.i32_between(-1_000_000, 1_000_000));
    }
}

#[test]
fn configs_c6_allocate_many() {
    let mut rng = FixedRng::new(0xC6);
    for _ in 0..1_024 {
        let count = (rng.next_u32() % 255 + 2) as usize;
        let init = rng.i32_between(-1_000_000, 1_000_000);
        compare_allocation(count, init);
    }
}

#[test]
fn configs_c7_free_null() {
    unsafe {
        for path in library_paths() {
            let library = Library::new(path).unwrap();
            let free_block: Symbol<FreeBlock> = get(&library, b"free_block\0");
            free_block(std::ptr::null_mut());
        }
    }
}

#[test]
fn configs_c8_free_block_with_null_data() {
    unsafe {
        for path in library_paths() {
            let library = Library::new(path).unwrap();
            let allocate: Symbol<AllocateBlock> = get(&library, b"allocate_block\0");
            let free_block: Symbol<FreeBlock> = get(&library, b"free_block\0");
            for _ in 0..64 {
                let block = allocate(8, 17);
                assert!(!block.is_null());
                free((*block).data.cast());
                (*block).data = std::ptr::null_mut();
                free_block(block);
            }
        }
    }
}

fn compare_hash_case(
    first_block: usize,
    second_block: usize,
    first_data: usize,
    second_data: usize,
    expected: i32,
) {
    let mut data = [11_i32, 22_i32];
    let mut blocks = [
        MemoryBlock {
            data: unsafe { data.as_mut_ptr().add(first_data) },
            size: 1,
        },
        MemoryBlock {
            data: unsafe { data.as_mut_ptr().add(second_data) },
            size: 1,
        },
    ];
    let first = &mut blocks[first_block] as *mut MemoryBlock;
    let second = &mut blocks[second_block] as *mut MemoryBlock;

    unsafe {
        let c = Library::new(c_library_path()).unwrap();
        let rust = Library::new(rust_library_path()).unwrap();
        let c_hash: Symbol<ComputeHash> = get(&c, b"compute_hash\0");
        let rust_hash: Symbol<ComputeHash> = get(&rust, b"compute_hash\0");
        assert_eq!(c_hash(first, second), expected);
        assert_eq!(rust_hash(first, second), expected);
    }
}

#[test]
fn configs_c9_through_c15_compute_hash_orders() {
    for _ in 0..512 {
        compare_hash_case(0, 1, 0, 1, 110);
        compare_hash_case(1, 0, 1, 0, 120);
        compare_hash_case(0, 1, 1, 0, 210);
        compare_hash_case(1, 0, 0, 1, 220);
        compare_hash_case(0, 1, 0, 0, 10);
        compare_hash_case(1, 0, 0, 0, 20);
        compare_hash_case(0, 0, 0, 0, 0);
    }
}

fn compare_betagamma(params: [i32; 4]) {
    let interposer_path = PathBuf::from(
        std::env::var("DIFF_INTERPOSER")
            .expect("betagamma comparisons must run in the deterministic child"),
    );
    unsafe {
        let c = Library::new(c_library_path()).unwrap();
        let rust = Library::new(rust_library_path()).unwrap();
        let interposer = Library::new(interposer_path).unwrap();
        let arm: Symbol<unsafe extern "C" fn()> =
            get(&interposer, b"arm_deterministic_allocations\0");
        let c_betagamma: Symbol<Betagamma> = get(&c, b"betagamma\0");
        let rust_betagamma: Symbol<Betagamma> = get(&rust, b"betagamma\0");
        arm();
        let c_result = c_betagamma(params[0], params[1], params[2], params[3]);
        arm();
        let rust_result = rust_betagamma(params[0], params[1], params[2], params[3]);
        assert_eq!(c_result, rust_result, "parameters: {params:?}");
    }
}

fn enter_deterministic_child(test_name: &str) -> bool {
    if std::env::var_os("DIFF_DETERMINISTIC_CHILD").is_some() {
        return true;
    }
    let interposer = build_interposer();
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env("LD_PRELOAD", &interposer)
        .env("DIFF_INTERPOSER", &interposer)
        .env("DIFF_DETERMINISTIC_CHILD", "1")
        .status()
        .unwrap();
    assert!(status.success(), "deterministic child failed: {status:?}");
    false
}

#[test]
fn configs_c16_betagamma_zero_count() {
    if !enter_deterministic_child("configs_c16_betagamma_zero_count") {
        return;
    }
    let mut rng = FixedRng::new(0xC16);
    for _ in 0..1_024 {
        let quotient = rng.i32_between(0, 10_000);
        compare_betagamma([
            -(quotient * 10 + 5),
            rng.i32_between(-10_000, 10_000),
            rng.i32_between(-10_000, 10_000),
            rng.i32_between(-10_000, 10_000),
        ]);
    }
}

#[test]
fn configs_c17_betagamma_one_count() {
    if !enter_deterministic_child("configs_c17_betagamma_one_count") {
        return;
    }
    let mut rng = FixedRng::new(0xC17);
    for _ in 0..1_024 {
        let quotient = rng.i32_between(0, 10_000);
        compare_betagamma([
            -(quotient * 10 + 4),
            rng.i32_between(-10_000, 10_000),
            rng.i32_between(-10_000, 10_000),
            rng.i32_between(-10_000, 10_000),
        ]);
    }
}

#[test]
fn configs_c18_betagamma_many_counts() {
    if !enter_deterministic_child("configs_c18_betagamma_many_counts") {
        return;
    }
    let mut rng = FixedRng::new(0xC18);
    for remainder in -3_i32..=9 {
        for _ in 0..256 {
            let quotient = rng.i32_between(0, 1_000);
            let param1 = if remainder < 0 {
                -(quotient * 10 + -remainder)
            } else {
                quotient * 10 + remainder
            };
            compare_betagamma([
                param1,
                rng.i32_between(-10_000, 10_000),
                rng.i32_between(-10_000, 10_000),
                rng.i32_between(-10_000, 10_000),
            ]);
        }
    }
}

#[test]
fn configs_c19_betagamma_defined_arithmetic_boundaries() {
    if !enter_deterministic_child("configs_c19_betagamma_defined_arithmetic_boundaries") {
        return;
    }
    let limit = i32::MAX;
    let cases = [
        [limit / 8, 0, 0, 0],
        [-(limit / 8), 0, 0, 0],
        [0, limit / 12, 0, 0],
        [0, -(limit / 12), 0, 0],
        [0, 0, limit / 12, 0],
        [0, 0, -(limit / 12), 0],
        [0, 0, 0, limit / 10],
        [0, 0, 0, -(limit / 10)],
    ];
    for params in cases {
        compare_betagamma(params);
    }
}

#[test]
fn errors_e3_betagamma_propagates_allocation_failure() {
    for param1 in [-6, -7, -8, -9, -16, -17, -18, -19] {
        unsafe {
            let c = Library::new(c_library_path()).unwrap();
            let rust = Library::new(rust_library_path()).unwrap();
            let c_betagamma: Symbol<Betagamma> = get(&c, b"betagamma\0");
            let rust_betagamma: Symbol<Betagamma> = get(&rust, b"betagamma\0");
            assert_eq!(c_betagamma(param1, 2, 3, 4), -1);
            assert_eq!(rust_betagamma(param1, 2, 3, 4), -1);
        }
    }
}

#[test]
fn generic_g4_allocate_size_max() {
    unsafe {
        let c = Library::new(c_library_path()).unwrap();
        let rust = Library::new(rust_library_path()).unwrap();
        let c_allocate: Symbol<AllocateBlock> = get(&c, b"allocate_block\0");
        let rust_allocate: Symbol<AllocateBlock> = get(&rust, b"allocate_block\0");
        assert!(c_allocate(usize::MAX, 0).is_null());
        assert!(rust_allocate(usize::MAX, 0).is_null());
    }
}

#[test]
fn ffi_fault_child() {
    let Ok(which_library) = std::env::var("DIFF_FAULT_LIBRARY") else {
        return;
    };
    let fault_case = std::env::var("DIFF_FAULT_CASE").unwrap();
    let path = if which_library == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    unsafe {
        let library = Library::new(path).unwrap();
        match fault_case.as_str() {
            "create_null" => {
                let function: Symbol<CreateBlock> = get(&library, b"create_block\0");
                let _ = function(1, std::ptr::null(), 0);
            }
            "hash_null_first" => {
                let function: Symbol<ComputeHash> = get(&library, b"compute_hash\0");
                let mut value = 0;
                let mut valid = MemoryBlock {
                    data: &mut value,
                    size: 1,
                };
                let _ = function(std::ptr::null_mut(), &mut valid);
            }
            "hash_null_second" => {
                let function: Symbol<ComputeHash> = get(&library, b"compute_hash\0");
                let mut value = 0;
                let mut valid = MemoryBlock {
                    data: &mut value,
                    size: 1,
                };
                let _ = function(&mut valid, std::ptr::null_mut());
            }
            other => panic!("unknown fault case {other}"),
        }
    }
    panic!("faulting call unexpectedly returned");
}

fn run_fault_child(which_library: &str, fault_case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_fault_child")
        .arg("--nocapture")
        .env("DIFF_FAULT_LIBRARY", which_library)
        .env("DIFF_FAULT_CASE", fault_case)
        .status()
        .unwrap()
}

#[test]
#[cfg(unix)]
fn generic_g1_g6_g7_null_pointer_faults_match() {
    for fault_case in ["create_null", "hash_null_first", "hash_null_second"] {
        let c_status = run_fault_child("c", fault_case);
        let rust_status = run_fault_child("rust", fault_case);
        assert!(!c_status.success());
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "different process signal for {fault_case}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

fn build_interposer() -> PathBuf {
    let output_dir = manifest_dir()
        .join("target")
        .join(format!("alloc-fail-{}", std::process::id()));
    fs::create_dir_all(&output_dir).unwrap();
    let output = output_dir.join("liballoc_fail.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(manifest_dir().join("tests/alloc_fail.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(output.is_file());
    output
}

#[test]
fn allocation_failure_child() {
    let Ok(which_library) = std::env::var("DIFF_ALLOC_LIBRARY") else {
        return;
    };
    let failure_case = std::env::var("DIFF_ALLOC_CASE").unwrap();
    let interposer_path = PathBuf::from(std::env::var("DIFF_INTERPOSER").unwrap());
    let target_path = if which_library == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };

    unsafe {
        let target = Library::new(target_path).unwrap();
        let interposer = Library::new(interposer_path).unwrap();
        let arm_malloc: Symbol<unsafe extern "C" fn(i64)> =
            get(&interposer, b"arm_malloc_failure\0");
        let arm_calloc: Symbol<unsafe extern "C" fn(i64)> =
            get(&interposer, b"arm_calloc_failure\0");

        match failure_case.as_str() {
            "allocate_malloc" => {
                let function: Symbol<AllocateBlock> = get(&target, b"allocate_block\0");
                arm_malloc(0);
                assert!(function(4, 7).is_null());
            }
            "allocate_calloc" => {
                let function: Symbol<AllocateBlock> = get(&target, b"allocate_block\0");
                arm_calloc(0);
                assert!(function(4, 7).is_null());
            }
            "betagamma_malloc" => {
                let function: Symbol<Betagamma> = get(&target, b"betagamma\0");
                arm_malloc(0);
                assert_eq!(function(1, 2, 3, 4), -1);
            }
            "betagamma_calloc" => {
                let function: Symbol<Betagamma> = get(&target, b"betagamma\0");
                arm_calloc(0);
                assert_eq!(function(1, 2, 3, 4), -1);
            }
            other => panic!("unknown allocation failure case {other}"),
        }
    }
}

fn run_allocation_child(interposer: &Path, which_library: &str, failure_case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("allocation_failure_child")
        .arg("--nocapture")
        .env("LD_PRELOAD", interposer)
        .env("DIFF_INTERPOSER", interposer)
        .env("DIFF_ALLOC_LIBRARY", which_library)
        .env("DIFF_ALLOC_CASE", failure_case)
        .status()
        .unwrap()
}

#[test]
fn errors_e1_e2_and_e3_allocator_injection() {
    let interposer = build_interposer();
    for failure_case in [
        "allocate_malloc",
        "allocate_calloc",
        "betagamma_malloc",
        "betagamma_calloc",
    ] {
        for which_library in ["c", "rust"] {
            let status = run_allocation_child(&interposer, which_library, failure_case);
            assert!(
                status.success(),
                "{which_library} failed injected case {failure_case}: {status:?}"
            );
        }
    }
}
