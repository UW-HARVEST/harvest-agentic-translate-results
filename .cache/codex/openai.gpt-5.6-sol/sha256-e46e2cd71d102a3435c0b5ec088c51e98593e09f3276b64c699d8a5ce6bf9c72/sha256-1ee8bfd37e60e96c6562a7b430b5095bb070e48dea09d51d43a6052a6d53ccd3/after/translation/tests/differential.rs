use libloading::Library;
use std::cmp::Ordering;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

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

struct Api {
    _library: Library,
    create_block: CreateBlock,
    allocate_block: AllocateBlock,
    free_block: FreeBlock,
    compute_hash: ComputeHash,
    betagamma: Betagamma,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        // SAFETY: Each copied function pointer is kept alive by the stored Library.
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let create_block = unsafe { *library.get(b"create_block\0").unwrap() };
        let allocate_block = unsafe { *library.get(b"allocate_block\0").unwrap() };
        let free_block = unsafe { *library.get(b"free_block\0").unwrap() };
        let compute_hash = unsafe { *library.get(b"compute_hash\0").unwrap() };
        let betagamma = unsafe { *library.get(b"betagamma\0").unwrap() };
        Self {
            _library: library,
            create_block,
            allocate_block,
            free_block,
            compute_hash,
            betagamma,
        }
    }
}

struct ApiPair {
    c: Api,
    rust: Api,
}

impl ApiPair {
    fn load() -> Self {
        // SAFETY: Paths identify the two libraries with the signatures above.
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-YACHJq.so")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = env::var_os("BETAGAMMA_RUST_LIB") {
        return path.into();
    }

    let test_exe = env::current_exe().expect("current test executable path");
    let deps = test_exe.parent().expect("test executable parent");
    let profile = deps.parent().expect("Cargo profile directory");
    for candidate in [
        deps.join("libbetagamma_lib.so"),
        profile.join("libbetagamma_lib.so"),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libbetagamma_lib.so"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "could not find the Rust cdylib beside {}",
        test_exe.display()
    );
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn bounded_i32(&mut self) -> i32 {
        (self.next_u64() % 2_000_001) as i32 - 1_000_000
    }
}

unsafe fn block_values(block: *mut MemoryBlock) -> Vec<c_int> {
    assert!(!block.is_null());
    // SAFETY: The allocation API records the allocated element count in size.
    unsafe { std::slice::from_raw_parts((*block).data, (*block).size).to_vec() }
}

fn assert_allocations_match(pair: &ApiPair, count: usize, init: c_int) {
    // SAFETY: Both returned blocks are inspected before being freed by their APIs.
    unsafe {
        let c_block = (pair.c.allocate_block)(count, init);
        let rust_block = (pair.rust.allocate_block)(count, init);
        assert_eq!(c_block.is_null(), rust_block.is_null());
        if !c_block.is_null() {
            assert_eq!((*c_block).size, (*rust_block).size);
            assert_eq!(block_values(c_block), block_values(rust_block));
        }
        (pair.c.free_block)(c_block);
        (pair.rust.free_block)(rust_block);
    }
}

#[test]
fn configuration_create_block_row_1() {
    let pair = ApiPair::load();
    let mut rng = Rng::new(0x8a5c_1f37_d249_b6e0);

    for case in 0..256 {
        let len = case % 32;
        let bytes: Vec<u8> = (0..len)
            .map(|_| b'a' + (rng.next_u64() % 26) as u8)
            .collect();
        let name = CString::new(bytes.clone()).unwrap();
        let id = rng.next_u64() as i32;
        let flags = rng.next_u64() as u8;

        // SAFETY: name is NUL-terminated and no longer than the C destination.
        let (c_block, rust_block) = unsafe {
            (
                (pair.c.create_block)(id, name.as_ptr(), flags),
                (pair.rust.create_block)(id, name.as_ptr(), flags),
            )
        };
        assert_eq!(c_block.id.to_ne_bytes(), rust_block.id.to_ne_bytes());
        assert_eq!(c_block.flags, rust_block.flags);
        for (index, expected) in bytes.iter().copied().chain([0]).enumerate() {
            assert_eq!(c_block.name[index] as u8, expected);
            assert_eq!(rust_block.name[index] as u8, expected);
        }
    }
}

#[test]
fn configuration_allocate_rows_2_to_4() {
    let pair = ApiPair::load();
    let mut rng = Rng::new(0xf0c3_97a2_631d_45be);

    for _ in 0..128 {
        assert_allocations_match(&pair, 0, rng.bounded_i32());
        assert_allocations_match(&pair, 1, rng.bounded_i32());
        assert_allocations_match(&pair, 2 + (rng.next_u64() % 63) as usize, rng.bounded_i32());
    }
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

unsafe fn raw_memory_block(data: *mut c_int) -> *mut MemoryBlock {
    // SAFETY: malloc storage is large and suitably aligned for MemoryBlock.
    let block = unsafe { malloc(std::mem::size_of::<MemoryBlock>()).cast::<MemoryBlock>() };
    assert!(!block.is_null());
    unsafe {
        block.write(MemoryBlock { data, size: 0 });
    }
    block
}

#[test]
fn configuration_free_rows_5_to_7() {
    let pair = ApiPair::load();

    // Row 5: NULL is an accepted no-op.
    unsafe {
        (pair.c.free_block)(std::ptr::null_mut());
        (pair.rust.free_block)(std::ptr::null_mut());
    }

    // Row 6: a nonnull block with null data frees only the outer block.
    unsafe {
        (pair.c.free_block)(raw_memory_block(std::ptr::null_mut()));
        (pair.rust.free_block)(raw_memory_block(std::ptr::null_mut()));
    }

    // Row 7: a nonnull block with nonnull data frees both allocations.
    unsafe {
        for api in [&pair.c, &pair.rust] {
            let data = malloc(std::mem::size_of::<c_int>()).cast::<c_int>();
            assert!(!data.is_null());
            data.write(123);
            (api.free_block)(raw_memory_block(data));
        }
    }
}

fn expected_hash(block_order: Ordering, data_order: Ordering) -> c_int {
    let data = match data_order {
        Ordering::Less => 100,
        Ordering::Equal => 0,
        Ordering::Greater => 200,
    };
    let block = match block_order {
        Ordering::Less => 10,
        Ordering::Equal => 0,
        Ordering::Greater => 20,
    };
    data + block
}

fn check_hash_case(pair: &ApiPair, block_order: Ordering, data_order: Ordering, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..128 {
        let mut data = [
            rng.bounded_i32(),
            rng.bounded_i32(),
            rng.bounded_i32(),
            rng.bounded_i32(),
        ];
        let mut blocks = [
            MemoryBlock {
                data: std::ptr::null_mut(),
                size: rng.next_u64() as usize,
            },
            MemoryBlock {
                data: std::ptr::null_mut(),
                size: rng.next_u64() as usize,
            },
        ];

        let (first, second) = match block_order {
            Ordering::Less => (0, 1),
            Ordering::Greater => (1, 0),
            Ordering::Equal => (0, 0),
        };
        match data_order {
            Ordering::Less => {
                blocks[first].data = data.as_mut_ptr();
                blocks[second].data = unsafe { data.as_mut_ptr().add(2) };
            }
            Ordering::Equal => {
                blocks[first].data = data.as_mut_ptr();
                blocks[second].data = data.as_mut_ptr();
            }
            Ordering::Greater => {
                blocks[first].data = unsafe { data.as_mut_ptr().add(2) };
                blocks[second].data = data.as_mut_ptr();
            }
        }

        let first_ptr = &mut blocks[first] as *mut MemoryBlock;
        let second_ptr = &mut blocks[second] as *mut MemoryBlock;
        // SAFETY: Both pointers and their data fields point into live arrays.
        let (c_result, rust_result) = unsafe {
            (
                (pair.c.compute_hash)(first_ptr, second_ptr),
                (pair.rust.compute_hash)(first_ptr, second_ptr),
            )
        };
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, expected_hash(block_order, data_order));
    }
}

#[test]
fn configuration_compute_hash_rows_8_to_14() {
    let pair = ApiPair::load();
    let mut seed = 0x4f82_159b_cea7_603d;
    for block_order in [Ordering::Less, Ordering::Greater] {
        for data_order in [Ordering::Less, Ordering::Equal, Ordering::Greater] {
            check_hash_case(&pair, block_order, data_order, seed);
            seed = seed.wrapping_add(0x1020_3040_5060_7080);
        }
    }
    check_hash_case(&pair, Ordering::Equal, Ordering::Equal, seed);
}

unsafe extern "C" {
    fn pipe(descriptors: *mut c_int) -> c_int;
    fn fork() -> c_int;
    fn read(descriptor: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn write(descriptor: c_int, buffer: *const c_void, count: usize) -> isize;
    fn close(descriptor: c_int) -> c_int;
    fn waitpid(process: c_int, status: *mut c_int, options: c_int) -> c_int;
}

fn forked_betagamma(api: &Api, args: [c_int; 4]) -> c_int {
    let mut descriptors = [-1; 2];
    // SAFETY: The child only calls the loaded function, writes four bytes, and exits.
    unsafe {
        assert_eq!(pipe(descriptors.as_mut_ptr()), 0);
        let process = fork();
        assert!(process >= 0);
        if process == 0 {
            close(descriptors[0]);
            let result = (api.betagamma)(args[0], args[1], args[2], args[3]);
            let written = write(
                descriptors[1],
                (&result as *const c_int).cast::<c_void>(),
                std::mem::size_of::<c_int>(),
            );
            _exit(if written == std::mem::size_of::<c_int>() as isize {
                0
            } else {
                6
            });
        }

        close(descriptors[1]);
        let mut result = 0;
        let received = read(
            descriptors[0],
            (&mut result as *mut c_int).cast::<c_void>(),
            std::mem::size_of::<c_int>(),
        );
        close(descriptors[0]);
        let mut status = 0;
        assert_eq!(waitpid(process, &mut status, 0), process);
        assert_eq!(status, 0);
        assert_eq!(received, std::mem::size_of::<c_int>() as isize);
        result
    }
}

fn assert_betagamma_matches(pair: &ApiPair, args: [c_int; 4]) {
    let c_result = forked_betagamma(&pair.c, args);
    let rust_result = forked_betagamma(&pair.rust, args);
    assert_eq!(c_result, rust_result, "arguments: {args:?}");
}

#[test]
fn configuration_betagamma_rows_15_to_17() {
    let pair = ApiPair::load();
    let mut rng = Rng::new(0x19e6_b3d8_704a_2fc5);

    for iteration in 0..256 {
        let params = [rng.bounded_i32(), rng.bounded_i32(), rng.bounded_i32()];

        // Row 15: C remainder -5 gives an empty allocation.
        let p1_empty = -5 - 10 * (iteration % 100) as i32;
        assert_betagamma_matches(&pair, [p1_empty, params[0], params[1], params[2]]);

        // Row 16: C remainder -4 gives one element.
        let p1_one = -4 - 10 * (iteration % 100) as i32;
        assert_betagamma_matches(&pair, [p1_one, params[0], params[1], params[2]]);

        // Row 17: cycle through every successful many-element size, 2..=14.
        let count = 2 + iteration % 13;
        let p1_many = if count < 5 {
            count as i32 - 5
        } else {
            10 * (iteration % 100) as i32 + count as i32 - 5
        };
        assert_betagamma_matches(&pair, [p1_many, params[0], params[1], params[2]]);
    }
}

#[test]
fn error_calloc_failure_row_2() {
    let pair = ApiPair::load();
    unsafe {
        let c_result = (pair.c.allocate_block)(usize::MAX, 7);
        let rust_result = (pair.rust.allocate_block)(usize::MAX, 7);
        assert!(c_result.is_null());
        assert!(rust_result.is_null());
    }
}

#[test]
fn error_betagamma_allocation_failure_row_3() {
    let pair = ApiPair::load();
    let mut rng = Rng::new(0xd46a_0b93_27fe_851c);
    for iteration in 0..128 {
        let remainder = 6 + iteration % 4;
        let p1 = -(remainder as i32) - 10 * (iteration % 100) as i32;
        let args = [p1, rng.bounded_i32(), rng.bounded_i32(), rng.bounded_i32()];
        unsafe {
            assert_eq!((pair.c.betagamma)(args[0], args[1], args[2], args[3]), -1);
            assert_eq!(
                (pair.rust.betagamma)(args[0], args[1], args[2], args[3]),
                -1
            );
        }
    }
}

#[repr(C)]
struct Rlimit {
    current: u64,
    maximum: u64,
}

unsafe extern "C" {
    fn setrlimit(resource: c_int, limit: *const Rlimit) -> c_int;
    fn sysconf(name: c_int) -> isize;
    fn _exit(status: c_int) -> !;
}

const RLIMIT_AS: c_int = 9;
const SC_PAGESIZE: c_int = 30;

fn child_status(test_name: &str, variables: &[(&str, &str)]) -> ExitStatus {
    let mut command = Command::new(env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .arg("--test-threads=1");
    for (name, value) in variables {
        command.env(name, value);
    }
    command.status().expect("run isolated test child")
}

#[test]
fn ffi_malloc_failure_child() {
    let Ok(kind) = env::var("BETAGAMMA_MALLOC_FAILURE_CHILD") else {
        return;
    };
    let path = if kind == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    // SAFETY: The child exits immediately after forcing the allocation call.
    unsafe {
        let api = Api::load(&path);

        // Resolve the allocation PLT entries before constraining address space.
        let warmup = (api.allocate_block)(usize::MAX, 0);
        assert!(warmup.is_null());

        let pages = std::fs::read_to_string("/proc/self/statm")
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        let page_size = sysconf(SC_PAGESIZE) as u64;
        let limit = Rlimit {
            current: pages * page_size,
            maximum: pages * page_size,
        };
        if setrlimit(RLIMIT_AS, &limit) != 0 {
            _exit(3);
        }

        let mut attempts = 0usize;
        while !malloc(16).is_null() {
            attempts += 1;
            if attempts == 10_000_000 {
                _exit(4);
            }
        }

        let result = (api.allocate_block)(1, 0);
        _exit(if result.is_null() { 0 } else { 5 });
    }
}

#[test]
fn error_malloc_failure_row_1() {
    for kind in ["c", "rust"] {
        let status = child_status(
            "ffi_malloc_failure_child",
            &[("BETAGAMMA_MALLOC_FAILURE_CHILD", kind)],
        );
        assert!(status.success(), "{kind} child status: {status}");
    }
}

#[test]
fn ffi_null_pointer_child() {
    let Ok(kind) = env::var("BETAGAMMA_NULL_CHILD_KIND") else {
        return;
    };
    let case = env::var("BETAGAMMA_NULL_CHILD_CASE").unwrap();
    let path = if kind == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    // SAFETY: These calls intentionally reproduce the C API's undefined inputs
    // in an isolated process so a fault cannot terminate the parent test.
    unsafe {
        let api = Api::load(&path);
        match case.as_str() {
            "create_name" => {
                (api.create_block)(1, std::ptr::null(), 0);
            }
            "hash_first" => {
                let mut valid = MemoryBlock {
                    data: std::ptr::dangling_mut(),
                    size: 0,
                };
                (api.compute_hash)(std::ptr::null_mut(), &mut valid);
            }
            "hash_second" => {
                let mut valid = MemoryBlock {
                    data: std::ptr::dangling_mut(),
                    size: 0,
                };
                (api.compute_hash)(&mut valid, std::ptr::null_mut());
            }
            "hash_both" => {
                (api.compute_hash)(std::ptr::null_mut(), std::ptr::null_mut());
            }
            _ => panic!("unknown null-pointer case"),
        }
    }
}

#[cfg(unix)]
fn terminating_signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[test]
fn generic_null_pointer_boundaries() {
    for case in ["create_name", "hash_first", "hash_second", "hash_both"] {
        let c_status = child_status(
            "ffi_null_pointer_child",
            &[
                ("BETAGAMMA_NULL_CHILD_KIND", "c"),
                ("BETAGAMMA_NULL_CHILD_CASE", case),
            ],
        );
        let rust_status = child_status(
            "ffi_null_pointer_child",
            &[
                ("BETAGAMMA_NULL_CHILD_KIND", "rust"),
                ("BETAGAMMA_NULL_CHILD_CASE", case),
            ],
        );
        assert!(!c_status.success(), "C unexpectedly accepted {case}");
        assert!(!rust_status.success(), "Rust unexpectedly accepted {case}");
        assert_eq!(
            terminating_signal(c_status),
            terminating_signal(rust_status),
            "different process-level result for {case}"
        );
    }
}

#[test]
fn generic_integer_size_and_length_boundaries() {
    let pair = ApiPair::load();

    for id in [c_int::MIN, c_int::MAX] {
        let name = CString::new(vec![b'x'; 32]).unwrap();
        // The C strcpy writes its terminator into flags, then flags is assigned.
        // Compare the complete defined field contents produced at this boundary.
        unsafe {
            let c_block = (pair.c.create_block)(id, name.as_ptr(), u8::MAX);
            let rust_block = (pair.rust.create_block)(id, name.as_ptr(), u8::MAX);
            assert_eq!(c_block.id.to_ne_bytes(), rust_block.id.to_ne_bytes());
            assert_eq!(c_block.name.map(|value| value as u8), [b'x'; 32]);
            assert_eq!(
                c_block.name.map(|value| value as u8),
                rust_block.name.map(|value| value as u8)
            );
            assert_eq!(c_block.flags, rust_block.flags);
        }
    }

    for init in [c_int::MIN, c_int::MAX - 1, c_int::MAX] {
        assert_allocations_match(&pair, 4, init);
    }

    let mut data = [0];
    for size in [0, usize::MAX] {
        let mut block = MemoryBlock {
            data: data.as_mut_ptr(),
            size,
        };
        unsafe {
            assert_eq!(
                (pair.c.compute_hash)(&mut block, &mut block),
                (pair.rust.compute_hash)(&mut block, &mut block)
            );
        }
    }

    for args in [
        [0, c_int::MIN, c_int::MAX, c_int::MIN],
        [c_int::MAX, c_int::MIN, c_int::MAX, c_int::MIN],
        [-5, c_int::MAX, c_int::MIN, c_int::MAX],
    ] {
        assert_betagamma_matches(&pair, args);
    }
    unsafe {
        assert_eq!(
            (pair.c.betagamma)(c_int::MIN, 0, 0, 0),
            (pair.rust.betagamma)(c_int::MIN, 0, 0, 0)
        );
        assert_eq!((pair.c.betagamma)(c_int::MIN, 0, 0, 0), -1);
    }
}
