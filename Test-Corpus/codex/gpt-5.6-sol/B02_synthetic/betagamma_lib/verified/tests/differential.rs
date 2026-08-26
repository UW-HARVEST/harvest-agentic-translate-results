use libloading::Library;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, mem, ptr, slice};

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

#[derive(Clone, Copy)]
struct Api {
    create_block: CreateBlock,
    allocate_block: AllocateBlock,
    free_block: FreeBlock,
    compute_hash: ComputeHash,
    betagamma: Betagamma,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn pipe(fds: *mut c_int) -> c_int;
    fn fork() -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn write(fd: c_int, buffer: *const c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
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

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn below(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = root.join("c_src/build/libtranslated_rust.so");
    let profile_dir = env::current_exe()
        .expect("test executable path")
        .parent()
        .expect("deps directory")
        .parent()
        .expect("Cargo profile directory")
        .to_owned();
    let rust_library = profile_dir.join("libbetagamma_lib.so");
    if !rust_library.is_file() {
        let mut build = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
        build
            .args(["build", "--no-default-features"])
            .current_dir(&root)
            .env(
                "CARGO_TARGET_DIR",
                profile_dir.parent().expect("Cargo target directory"),
            );
        if profile_dir.file_name().and_then(|name| name.to_str()) == Some("release") {
            build.arg("--release");
        }
        let status = build.status().expect("build Rust cdylib for FFI tests");
        assert!(status.success(), "Rust cdylib build failed");
    }
    assert!(
        c_library.is_file(),
        "missing C shared library: {}",
        c_library.display()
    );
    assert!(
        rust_library.is_file(),
        "missing Rust shared library: {}",
        rust_library.display()
    );
    (c_library, rust_library)
}

unsafe fn load_api(library: &Library) -> Api {
    unsafe {
        Api {
            create_block: *library.get(b"create_block\0").unwrap(),
            allocate_block: *library.get(b"allocate_block\0").unwrap(),
            free_block: *library.get(b"free_block\0").unwrap(),
            compute_hash: *library.get(b"compute_hash\0").unwrap(),
            betagamma: *library.get(b"betagamma\0").unwrap(),
        }
    }
}

fn with_apis(test: impl FnOnce(Api, Api)) {
    let (c_path, rust_path) = library_paths();
    unsafe {
        let c_library = Library::new(c_path).expect("load C shared library");
        let rust_library = Library::new(rust_path).expect("load Rust shared library");
        test(load_api(&c_library), load_api(&rust_library));
    }
}

fn assert_blocks_equal(c_block: DataBlock, rust_block: DataBlock, initialized_name: usize) {
    assert_eq!(c_block.id, rust_block.id);
    assert_eq!(c_block.flags, rust_block.flags);
    assert_eq!(
        &c_block.name[..initialized_name],
        &rust_block.name[..initialized_name]
    );
}

unsafe fn allocation_snapshot(
    api: Api,
    count: usize,
    init_value: c_int,
) -> Option<(usize, bool, Vec<c_int>)> {
    let block = unsafe { (api.allocate_block)(count, init_value) };
    if block.is_null() {
        return None;
    }
    let size = unsafe { (*block).size };
    let data_is_null = unsafe { (*block).data.is_null() };
    let values = if data_is_null {
        Vec::new()
    } else {
        unsafe { slice::from_raw_parts((*block).data, size) }.to_vec()
    };
    unsafe {
        (api.free_block)(block);
    }
    Some((size, data_is_null, values))
}

fn test_create_block(c: Api, rust: Api, rng: &mut Rng) {
    for flags in 0..=u8::MAX {
        let name = [0 as c_char];
        let id = rng.next_i32();
        unsafe {
            let c_block = (c.create_block)(id, name.as_ptr(), flags);
            let rust_block = (rust.create_block)(id, name.as_ptr(), flags);
            assert_blocks_equal(c_block, rust_block, 1);
        }
    }

    for _ in 0..128 {
        let length = 1 + rng.below(30);
        let mut name = vec![0 as c_char; length + 1];
        for byte in &mut name[..length] {
            *byte = (1 + rng.below(127)) as c_char;
        }
        let id = rng.next_i32();
        let flags = rng.next_u64() as u8;
        unsafe {
            let c_block = (c.create_block)(id, name.as_ptr(), flags);
            let rust_block = (rust.create_block)(id, name.as_ptr(), flags);
            assert_blocks_equal(c_block, rust_block, length + 1);
        }
    }

    for _ in 0..128 {
        let mut name = [0 as c_char; 32];
        for byte in &mut name[..31] {
            *byte = (1 + rng.below(127)) as c_char;
        }
        let id = rng.next_i32();
        let flags = rng.next_u64() as u8;
        unsafe {
            let c_block = (c.create_block)(id, name.as_ptr(), flags);
            let rust_block = (rust.create_block)(id, name.as_ptr(), flags);
            assert_blocks_equal(c_block, rust_block, 32);
        }
    }
}

fn test_allocate_and_free(c: Api, rust: Api, rng: &mut Rng) {
    for &count in &[0, 1] {
        for _ in 0..128 {
            let init_value = rng.next_i32();
            unsafe {
                assert_eq!(
                    allocation_snapshot(c, count, init_value),
                    allocation_snapshot(rust, count, init_value)
                );
            }
        }
    }

    for _ in 0..256 {
        let count = 2 + rng.below(127);
        let init_value = rng.next_i32();
        unsafe {
            assert_eq!(
                allocation_snapshot(c, count, init_value),
                allocation_snapshot(rust, count, init_value)
            );
        }
    }

    unsafe {
        (c.free_block)(ptr::null_mut());
        (rust.free_block)(ptr::null_mut());

        for api in [c, rust] {
            let block = malloc(mem::size_of::<MemoryBlock>()).cast::<MemoryBlock>();
            assert!(!block.is_null());
            block.write(MemoryBlock {
                data: ptr::null_mut(),
                size: 123,
            });
            (api.free_block)(block);
        }
    }
}

fn test_compute_hash(c: Api, rust: Api, rng: &mut Rng) {
    for block_order in [-1, 1] {
        for data_order in [-1, 0, 1] {
            for iteration in 0..128 {
                let mut data = [0_i32; 16];
                let low = rng.below(8);
                let high = 8 + rng.below(8);
                let (data1, data2) = match data_order {
                    -1 => (unsafe { data.as_mut_ptr().add(low) }, unsafe {
                        data.as_mut_ptr().add(high)
                    }),
                    0 if iteration % 2 == 0 => (ptr::null_mut(), ptr::null_mut()),
                    0 => {
                        let same = unsafe { data.as_mut_ptr().add(rng.below(data.len())) };
                        (same, same)
                    }
                    1 => (unsafe { data.as_mut_ptr().add(high) }, unsafe {
                        data.as_mut_ptr().add(low)
                    }),
                    _ => unreachable!(),
                };
                let mut blocks = [
                    MemoryBlock {
                        data: ptr::null_mut(),
                        size: 0,
                    },
                    MemoryBlock {
                        data: ptr::null_mut(),
                        size: 0,
                    },
                ];
                let (first, second) = if block_order < 0 {
                    let (left, right) = blocks.split_at_mut(1);
                    (&mut left[0], &mut right[0])
                } else {
                    let (left, right) = blocks.split_at_mut(1);
                    (&mut right[0], &mut left[0])
                };
                first.data = data1;
                second.data = data2;
                let expected = match (block_order, data_order) {
                    (-1, -1) => 110,
                    (-1, 0) => 10,
                    (-1, 1) => 210,
                    (1, -1) => 120,
                    (1, 0) => 20,
                    (1, 1) => 220,
                    _ => unreachable!(),
                };
                unsafe {
                    assert_eq!((c.compute_hash)(first, second), expected);
                    assert_eq!((rust.compute_hash)(first, second), expected);
                }
            }
        }
    }

    for iteration in 0..128 {
        let mut value = 0;
        let mut block = MemoryBlock {
            data: if iteration % 2 == 0 {
                ptr::null_mut()
            } else {
                &mut value
            },
            size: 1,
        };
        unsafe {
            assert_eq!((c.compute_hash)(&mut block, &mut block), 0);
            assert_eq!((rust.compute_hash)(&mut block, &mut block), 0);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ChildOutcome {
    status: c_int,
    value: Option<c_int>,
}

fn fork_call(call: impl FnOnce() -> c_int) -> ChildOutcome {
    unsafe {
        let mut fds = [0; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let pid = fork();
        assert!(pid >= 0);
        if pid == 0 {
            close(fds[0]);
            let value = call();
            let bytes = write(
                fds[1],
                (&value as *const c_int).cast::<c_void>(),
                mem::size_of::<c_int>(),
            );
            _exit(if bytes == mem::size_of::<c_int>() as isize {
                0
            } else {
                120
            });
        }

        close(fds[1]);
        let mut value = 0;
        let bytes = read(
            fds[0],
            (&mut value as *mut c_int).cast::<c_void>(),
            mem::size_of::<c_int>(),
        );
        close(fds[0]);
        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid);
        ChildOutcome {
            status,
            value: (bytes == mem::size_of::<c_int>() as isize).then_some(value),
        }
    }
}

fn assert_forked_equal(c_call: impl FnOnce() -> c_int, rust_call: impl FnOnce() -> c_int) {
    let c_outcome = fork_call(c_call);
    let rust_outcome = fork_call(rust_call);
    assert_eq!(c_outcome, rust_outcome);
}

fn test_betagamma(c: Api, rust: Api, rng: &mut Rng) {
    for remainder in [-5_i32, -4] {
        for _ in 0..128 {
            let multiplier = rng.below(200_000_000) as i32;
            let param1 = remainder - 10 * multiplier;
            let args = (param1, rng.next_i32(), rng.next_i32(), rng.next_i32());
            assert_forked_equal(
                || unsafe { (c.betagamma)(args.0, args.1, args.2, args.3) },
                || unsafe { (rust.betagamma)(args.0, args.1, args.2, args.3) },
            );
        }
    }

    let boundary_values = [i32::MIN + 5, -3, -2, -1, 0, 1, 9, 10, i32::MAX];
    for iteration in 0..256 {
        let param1 = if iteration < boundary_values.len() {
            boundary_values[iteration]
        } else {
            loop {
                let candidate = rng.next_i32();
                if candidate % 10 >= -3 {
                    break candidate;
                }
            }
        };
        let args = (param1, rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_forked_equal(
            || unsafe { (c.betagamma)(args.0, args.1, args.2, args.3) },
            || unsafe { (rust.betagamma)(args.0, args.1, args.2, args.3) },
        );
    }
}

fn test_natural_errors(c: Api, rust: Api, rng: &mut Rng) {
    unsafe {
        assert!(
            (c.allocate_block)(usize::MAX, rng.next_i32()).is_null(),
            "C oversized allocation must fail"
        );
        assert!(
            (rust.allocate_block)(usize::MAX, rng.next_i32()).is_null(),
            "Rust oversized allocation must fail"
        );
    }

    for _ in 0..128 {
        let remainder = -9 + rng.below(4) as i32;
        let multiplier = rng.below(200_000_000) as i32;
        let param1 = remainder - 10 * multiplier;
        let args = (param1, rng.next_i32(), rng.next_i32(), rng.next_i32());
        unsafe {
            assert_eq!((c.betagamma)(args.0, args.1, args.2, args.3), -1);
            assert_eq!((rust.betagamma)(args.0, args.1, args.2, args.3), -1);
        }
    }
}

fn test_generic_boundaries(c: Api, rust: Api) {
    assert_forked_equal(
        || unsafe {
            black_box((c.create_block)(7, ptr::null(), 3));
            0
        },
        || unsafe {
            black_box((rust.create_block)(7, ptr::null(), 3));
            0
        },
    );

    let mut overlong = [0 as c_char; 33];
    overlong[..32].fill(b'X' as c_char);
    unsafe {
        let c_block = (c.create_block)(i32::MIN, overlong.as_ptr(), u8::MAX);
        let rust_block = (rust.create_block)(i32::MIN, overlong.as_ptr(), u8::MAX);
        assert_blocks_equal(c_block, rust_block, 32);
    }

    assert_forked_equal(
        || unsafe {
            black_box((c.compute_hash)(ptr::null_mut(), ptr::null_mut()));
            0
        },
        || unsafe {
            black_box((rust.compute_hash)(ptr::null_mut(), ptr::null_mut()));
            0
        },
    );

    assert_forked_equal(
        || {
            let mut block = MemoryBlock {
                data: ptr::null_mut(),
                size: 0,
            };
            unsafe {
                black_box((c.compute_hash)(&mut block, ptr::null_mut()));
            }
            0
        },
        || {
            let mut block = MemoryBlock {
                data: ptr::null_mut(),
                size: 0,
            };
            unsafe {
                black_box((rust.compute_hash)(&mut block, ptr::null_mut()));
            }
            0
        },
    );
}

fn fail_alloc_library(root: &Path) -> PathBuf {
    root.join("target/test-support/libfail_alloc.so")
}

fn run_preloaded_allocator_test() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_dir = root.join("target/test-support");
    std::fs::create_dir_all(&output_dir).expect("create test support output");
    let helper = fail_alloc_library(&root);
    let compile = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(root.join("tests/support/fail_alloc.c"))
        .args(["-o"])
        .arg(&helper)
        .status()
        .expect("run cc for allocation failure helper");
    assert!(
        compile.success(),
        "allocation failure helper compilation failed"
    );

    let status = Command::new(env::current_exe().expect("test executable"))
        .args(["--exact", "preloaded_allocator_failures", "--nocapture"])
        .env("LD_PRELOAD", &helper)
        .env("BETAGAMMA_PRELOADED", "1")
        .status()
        .expect("run preloaded allocator failure test");
    assert!(status.success(), "preloaded allocator failure test failed");
}

#[test]
fn differential_surface() {
    if env::var_os("BETAGAMMA_PRELOADED").is_some() {
        return;
    }

    with_apis(|c, rust| {
        let mut rng = Rng::new(0x8f3c_2a19_d407_6be5);
        test_create_block(c, rust, &mut rng);
        test_allocate_and_free(c, rust, &mut rng);
        test_compute_hash(c, rust, &mut rng);
        test_betagamma(c, rust, &mut rng);
        test_natural_errors(c, rust, &mut rng);
        test_generic_boundaries(c, rust);
    });
    run_preloaded_allocator_test();
}

#[test]
fn preloaded_allocator_failures() {
    if env::var_os("BETAGAMMA_PRELOADED").is_none() {
        return;
    }

    type FailMalloc = unsafe extern "C" fn(usize, c_uint);
    type FailCalloc = unsafe extern "C" fn(c_uint);

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    unsafe {
        let helper = Library::new(fail_alloc_library(&root)).expect("open preload helper");
        let fail_malloc: FailMalloc = *helper.get(b"fail_malloc_on_nth\0").unwrap();
        let fail_calloc: FailCalloc = *helper.get(b"fail_calloc_on_nth\0").unwrap();

        with_apis(|c, rust| {
            for api in [c, rust] {
                fail_malloc(mem::size_of::<MemoryBlock>(), 1);
                assert!((api.allocate_block)(8, 1).is_null());

                fail_calloc(1);
                assert!((api.allocate_block)(8, 1).is_null());

                fail_calloc(1);
                assert_eq!((api.betagamma)(1, 2, 3, 4), -1);

                fail_calloc(2);
                assert_eq!((api.betagamma)(1, 2, 3, 4), -1);
            }
        });
    }
}
