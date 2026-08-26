use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const WIDTH: usize = 256;
static PROCESS_IO_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Buffer {
    data: [u8; WIDTH],
    length: usize,
    checksum: u32,
}

impl Buffer {
    fn empty_with(fill: u8) -> Self {
        Self {
            data: [fill; WIDTH],
            length: 0,
            checksum: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct BufferArray {
    buffers: *mut Buffer,
    count: c_int,
    capacity: c_int,
}

type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;
type ChecksumFn = unsafe extern "C" fn(*const u8, usize) -> u32;
type ValidateFn = unsafe extern "C" fn(*const Buffer) -> bool;
type InitFn = unsafe extern "C" fn(c_int) -> *mut BufferArray;
type FreeFn = unsafe extern "C" fn(*mut BufferArray);
type CopyFn = unsafe extern "C" fn(*const Buffer, *mut Buffer) -> c_int;
type ReverseFn = unsafe extern "C" fn(*mut Buffer) -> c_int;
type MergeFn = unsafe extern "C" fn(*const Buffer, *const Buffer, *mut Buffer) -> c_int;
type SplitFn = unsafe extern "C" fn(*const Buffer, usize, *mut Buffer, *mut Buffer) -> c_int;
type InterleaveFn = unsafe extern "C" fn(*const Buffer, *const Buffer, *mut Buffer) -> c_int;
type RotateFn = unsafe extern "C" fn(*mut Buffer, c_int) -> c_int;
type ConditionalFn = unsafe extern "C" fn(*const Buffer, *mut Buffer, u8, bool) -> c_int;
type StridedFn = unsafe extern "C" fn(*const Buffer, *mut Buffer, c_int) -> c_int;
type ProcessFn = unsafe extern "C" fn(*mut BufferArray, c_int, c_int) -> c_int;
type ReadFn = unsafe extern "C" fn(*mut Buffer) -> c_int;
type WriteFn = unsafe extern "C" fn(*const Buffer);

struct Api {
    _library: Library,
    main: MainFn,
    checksum: ChecksumFn,
    validate: ValidateFn,
    init: InitFn,
    free: FreeFn,
    copy: CopyFn,
    reverse: ReverseFn,
    merge: MergeFn,
    split: SplitFn,
    interleave: InterleaveFn,
    rotate: RotateFn,
    conditional: ConditionalFn,
    strided: StridedFn,
    process: ProcessFn,
    read: ReadFn,
    write: WriteFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! load {
            ($name:literal, $kind:ty) => {
                *unsafe { library.get::<$kind>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name))
            };
        }
        Self {
            main: load!("main", MainFn),
            checksum: load!("calculate_checksum", ChecksumFn),
            validate: load!("validate_buffer", ValidateFn),
            init: load!("init_buffer_array", InitFn),
            free: load!("free_buffer_array", FreeFn),
            copy: load!("buffer_copy", CopyFn),
            reverse: load!("buffer_reverse", ReverseFn),
            merge: load!("buffer_merge", MergeFn),
            split: load!("buffer_split", SplitFn),
            interleave: load!("buffer_interleave", InterleaveFn),
            rotate: load!("buffer_rotate", RotateFn),
            conditional: load!("buffer_conditional_copy", ConditionalFn),
            strided: load!("buffer_copy_strided", StridedFn),
            process: load!("process_buffer_array", ProcessFn),
            read: load!("read_buffer", ReadFn),
            write: load!("write_buffer", WriteFn),
            _library: library,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_apis() -> (Api, Api) {
    let root = manifest_dir();
    let c_path = root.join("c_src/build/libdriver_c.so");
    let release = root.join("target/release/libdriver.so");
    let debug = root.join("target/debug/libdriver.so");
    let rust_path = if release.exists() { release } else { debug };
    assert!(c_path.exists(), "C oracle is missing: {}", c_path.display());
    assert!(
        rust_path.exists(),
        "Rust cdylib is missing: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

#[derive(Clone)]
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

    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

fn local_checksum(data: &[u8]) -> u32 {
    data.iter()
        .fold(0_u32, |sum, byte| sum.wrapping_shl(3) ^ u32::from(*byte))
}

fn random_buffer(rng: &mut Rng, length: usize) -> Buffer {
    let mut buffer = Buffer::empty_with(0xA5);
    buffer.length = length;
    for byte in &mut buffer.data[..length] {
        *byte = rng.byte();
    }
    buffer.checksum = local_checksum(&buffer.data[..length]);
    buffer
}

fn assert_buffer_eq(context: &str, c: &Buffer, rust: &Buffer) {
    assert_eq!(c.length, rust.length, "{context}: length");
    assert_eq!(c.checksum, rust.checksum, "{context}: checksum");
    assert_eq!(c.data, rust.data, "{context}: all data bytes");
}

fn assert_logical_buffer_eq(context: &str, c: &Buffer, rust: &Buffer) {
    assert_eq!(c.length, rust.length, "{context}: length");
    assert_eq!(c.checksum, rust.checksum, "{context}: checksum");
    assert_eq!(
        &c.data[..c.length],
        &rust.data[..rust.length],
        "{context}: logical data bytes"
    );
}

fn io_guard() -> std::sync::MutexGuard<'static, ()> {
    PROCESS_IO_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn dynamic_symbol_surface_loads() {
    let _guard = io_guard();
    let _ = load_apis();
}

#[test]
fn low_level_valid_paths_match() {
    let _guard = io_guard();
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x5eed_c0de_1234_5678);

    for &length in &[0, 1, 2, 17, 63, 128, 255, 256] {
        for iteration in 0..32 {
            let buffer = random_buffer(&mut rng, length);
            let c_sum = unsafe { (c.checksum)(buffer.data.as_ptr(), length) };
            let rust_sum = unsafe { (rust.checksum)(buffer.data.as_ptr(), length) };
            assert_eq!(c_sum, rust_sum, "checksum length={length} iter={iteration}");
            assert_eq!(
                unsafe { (c.validate)(&buffer) },
                unsafe { (rust.validate)(&buffer) },
                "validate length={length} iter={iteration}"
            );

            let mut mismatched = buffer;
            mismatched.checksum ^= 1;
            assert_eq!(
                unsafe { (c.validate)(&mismatched) },
                unsafe { (rust.validate)(&mismatched) },
                "mismatched checksum length={length} iter={iteration}"
            );

            let mut c_copy = Buffer::empty_with(0xCC);
            let mut rust_copy = c_copy;
            let c_result = unsafe { (c.copy)(&buffer, &mut c_copy) };
            let rust_result = unsafe { (rust.copy)(&buffer, &mut rust_copy) };
            assert_eq!(c_result, rust_result);
            assert_buffer_eq("copy", &c_copy, &rust_copy);

            let mut c_reverse = buffer;
            let mut rust_reverse = buffer;
            let c_result = unsafe { (c.reverse)(&mut c_reverse) };
            let rust_result = unsafe { (rust.reverse)(&mut rust_reverse) };
            assert_eq!(c_result, rust_result);
            assert_buffer_eq("reverse", &c_reverse, &rust_reverse);
        }
    }

    let merge_shapes = [
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (2, 19),
        (19, 2),
        (128, 128),
        (1, 255),
        (255, 1),
    ];
    for &(left_length, right_length) in &merge_shapes {
        for _ in 0..32 {
            let left = random_buffer(&mut rng, left_length);
            let right = random_buffer(&mut rng, right_length);
            let mut c_output = Buffer::empty_with(0x3C);
            let mut rust_output = c_output;
            let c_result = unsafe { (c.merge)(&left, &right, &mut c_output) };
            let rust_result = unsafe { (rust.merge)(&left, &right, &mut rust_output) };
            assert_eq!(c_result, rust_result);
            assert_buffer_eq("merge", &c_output, &rust_output);

            let mut c_output = Buffer::empty_with(0x3D);
            let mut rust_output = c_output;
            let c_result = unsafe { (c.interleave)(&left, &right, &mut c_output) };
            let rust_result = unsafe { (rust.interleave)(&left, &right, &mut rust_output) };
            assert_eq!(c_result, rust_result);
            assert_buffer_eq("interleave", &c_output, &rust_output);
        }
    }

    for &length in &[0, 1, 2, 17, 255, 256] {
        for _ in 0..32 {
            let source = random_buffer(&mut rng, length);
            let positions: Vec<usize> = if length == 0 {
                vec![0]
            } else {
                vec![0, length / 2, length]
            };
            for split_position in positions {
                let mut c_left = Buffer::empty_with(0x11);
                let mut c_right = Buffer::empty_with(0x22);
                let mut rust_left = c_left;
                let mut rust_right = c_right;
                let c_result =
                    unsafe { (c.split)(&source, split_position, &mut c_left, &mut c_right) };
                let rust_result = unsafe {
                    (rust.split)(&source, split_position, &mut rust_left, &mut rust_right)
                };
                assert_eq!(c_result, rust_result);
                assert_buffer_eq("split left", &c_left, &rust_left);
                assert_buffer_eq("split right", &c_right, &rust_right);
            }

            let rotation_positions = if length == 0 {
                vec![0, 1, -1, c_int::MAX, c_int::MIN]
            } else {
                vec![
                    0,
                    1,
                    length as c_int,
                    length as c_int + 1,
                    -1,
                    -(length as c_int),
                    -(length as c_int) - 1,
                    c_int::MAX,
                    c_int::MIN,
                ]
            };
            for positions in rotation_positions {
                let mut c_output = source;
                let mut rust_output = source;
                let c_result = unsafe { (c.rotate)(&mut c_output, positions) };
                let rust_result = unsafe { (rust.rotate)(&mut rust_output, positions) };
                assert_eq!(c_result, rust_result);
                assert_buffer_eq("rotate", &c_output, &rust_output);
            }
        }
    }

    for &length in &[0, 1, 2, 31, 255, 256] {
        for _ in 0..32 {
            let mut source = random_buffer(&mut rng, length);
            let pattern = rng.byte();
            if length > 0 {
                source.data[0] = pattern;
                source.checksum = local_checksum(&source.data[..length]);
            }
            for copy_matching in [false, true] {
                let mut c_output = Buffer::empty_with(0x71);
                let mut rust_output = c_output;
                let c_result =
                    unsafe { (c.conditional)(&source, &mut c_output, pattern, copy_matching) };
                let rust_result = unsafe {
                    (rust.conditional)(&source, &mut rust_output, pattern, copy_matching)
                };
                assert_eq!(c_result, rust_result);
                assert_buffer_eq("conditional", &c_output, &rust_output);
            }

            for stride in [1, 2, 7, length.max(1) as c_int, 257] {
                let mut c_output = Buffer::empty_with(0x72);
                let mut rust_output = c_output;
                let c_result = unsafe { (c.strided)(&source, &mut c_output, stride) };
                let rust_result = unsafe { (rust.strided)(&source, &mut rust_output, stride) };
                assert_eq!(c_result, rust_result);
                assert_buffer_eq("strided", &c_output, &rust_output);
            }
        }
    }
}

fn run_process_pair(
    c: &Api,
    rust: &Api,
    buffers: &[Buffer],
    count: c_int,
    operation: c_int,
    parameter: c_int,
) {
    let mut c_buffers = buffers.to_vec();
    let mut rust_buffers = buffers.to_vec();
    let mut c_array = BufferArray {
        buffers: c_buffers.as_mut_ptr(),
        count,
        capacity: buffers.len() as c_int,
    };
    let mut rust_array = BufferArray {
        buffers: rust_buffers.as_mut_ptr(),
        count,
        capacity: buffers.len() as c_int,
    };
    let c_result = unsafe { (c.process)(&mut c_array, operation, parameter) };
    let rust_result = unsafe { (rust.process)(&mut rust_array, operation, parameter) };
    assert_eq!(
        c_result, rust_result,
        "process result op={operation} count={count} param={parameter}"
    );
    assert_eq!(c_array.count, rust_array.count);
    assert_eq!(c_array.capacity, rust_array.capacity);
    for (index, (c_buffer, rust_buffer)) in c_buffers.iter().zip(&rust_buffers).enumerate() {
        assert_logical_buffer_eq(
            &format!("process op={operation} buffer={index}"),
            c_buffer,
            rust_buffer,
        );
    }
}

#[test]
fn allocation_and_process_paths_match() {
    let _guard = io_guard();
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xa110_ca7e_55aa_0101);

    unsafe {
        (c.free)(ptr::null_mut());
        (rust.free)(ptr::null_mut());
    }
    for capacity in [1, 2, 7, 100] {
        let c_array = unsafe { (c.init)(capacity) };
        let rust_array = unsafe { (rust.init)(capacity) };
        assert_eq!(c_array.is_null(), rust_array.is_null());
        assert!(!c_array.is_null());
        unsafe {
            assert_eq!((*c_array).count, (*rust_array).count);
            assert_eq!((*c_array).capacity, (*rust_array).capacity);
            for index in 0..capacity as usize {
                let value = random_buffer(&mut rng, index % 17);
                ptr::write((*c_array).buffers.add(index), value);
                ptr::write((*rust_array).buffers.add(index), value);
            }
            (*c_array).count = capacity;
            (*rust_array).count = capacity;
            for index in 0..capacity as usize {
                assert_buffer_eq(
                    "allocated storage",
                    &*(*c_array).buffers.add(index),
                    &*(*rust_array).buffers.add(index),
                );
            }
            (c.free)(c_array);
            (rust.free)(rust_array);
        }
    }

    for _ in 0..64 {
        let count = 1 + rng.usize(8);
        let mut buffers = Vec::with_capacity(count);
        for _ in 0..count {
            let length = rng.usize(65);
            buffers.push(random_buffer(&mut rng, length));
        }
        run_process_pair(&c, &rust, &buffers, count as c_int, 0, 0);
        run_process_pair(&c, &rust, &buffers, count as c_int, 1, 0);
        run_process_pair(
            &c,
            &rust,
            &buffers,
            count as c_int,
            5,
            rng.next_u64() as c_int,
        );
        run_process_pair(&c, &rust, &buffers, count as c_int, 6, 0);

        if count >= 2 {
            let mut merge_buffers = Vec::with_capacity(count);
            for _ in 0..count {
                let length = rng.usize(129);
                merge_buffers.push(random_buffer(&mut rng, length));
            }
            run_process_pair(&c, &rust, &merge_buffers, count as c_int, 2, 0);
        }
    }

    let one = [random_buffer(&mut rng, 5)];
    run_process_pair(&c, &rust, &one, 1, 0, 0);
    run_process_pair(&c, &rust, &one, -1, 0, 0);
    run_process_pair(&c, &rust, &one, -1, 1, 0);
    run_process_pair(&c, &rust, &one, -1, 5, 3);
    run_process_pair(&c, &rust, &one, -1, 6, 0);
    run_process_pair(&c, &rust, &one, -1, 2, 0);

    let mut mismatched = random_buffer(&mut rng, 31);
    mismatched.checksum ^= 0x8000_0000;
    run_process_pair(&c, &rust, &[mismatched], 1, 6, 0);
}

fn assert_same_i32(context: &str, c_value: c_int, rust_value: c_int) {
    assert_eq!(c_value, rust_value, "{context}");
}

#[test]
fn low_level_error_paths_match() {
    let _guard = io_guard();
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xeeee_0000_1234_9876);
    let source = random_buffer(&mut rng, 8);
    let second = random_buffer(&mut rng, 9);
    assert_eq!(unsafe { (c.checksum)(ptr::null(), 0) }, unsafe {
        (rust.checksum)(ptr::null(), 0)
    });

    assert_eq!(unsafe { (c.validate)(ptr::null()) }, unsafe {
        (rust.validate)(ptr::null())
    });
    let mut oversized = source;
    oversized.length = 257;
    assert_eq!(unsafe { (c.validate)(&oversized) }, unsafe {
        (rust.validate)(&oversized)
    });

    for capacity in [0, -1, c_int::MIN] {
        let c_array = unsafe { (c.init)(capacity) };
        let rust_array = unsafe { (rust.init)(capacity) };
        assert_eq!(c_array.is_null(), rust_array.is_null());
    }
    let c_array = unsafe { (c.init)(c_int::MAX) };
    let rust_array = unsafe { (rust.init)(c_int::MAX) };
    assert_eq!(c_array.is_null(), rust_array.is_null());
    if !c_array.is_null() {
        unsafe { (c.free)(c_array) };
    }
    if !rust_array.is_null() {
        unsafe { (rust.free)(rust_array) };
    }

    let mut c_destination = Buffer::empty_with(0x61);
    let mut rust_destination = c_destination;
    let null_buffer = ptr::null();
    let null_buffer_mut = ptr::null_mut();
    assert_same_i32(
        "copy null source",
        unsafe { (c.copy)(null_buffer, &mut c_destination) },
        unsafe { (rust.copy)(null_buffer, &mut rust_destination) },
    );
    assert_same_i32(
        "copy null destination",
        unsafe { (c.copy)(&source, null_buffer_mut) },
        unsafe { (rust.copy)(&source, null_buffer_mut) },
    );
    assert_same_i32(
        "copy oversized",
        unsafe { (c.copy)(&oversized, &mut c_destination) },
        unsafe { (rust.copy)(&oversized, &mut rust_destination) },
    );
    assert_same_i32(
        "reverse null",
        unsafe { (c.reverse)(null_buffer_mut) },
        unsafe { (rust.reverse)(null_buffer_mut) },
    );

    for null_index in 0..3 {
        let c_result = unsafe {
            (c.merge)(
                if null_index == 0 {
                    null_buffer
                } else {
                    &source
                },
                if null_index == 1 {
                    null_buffer
                } else {
                    &second
                },
                if null_index == 2 {
                    null_buffer_mut
                } else {
                    &mut c_destination
                },
            )
        };
        let rust_result = unsafe {
            (rust.merge)(
                if null_index == 0 {
                    null_buffer
                } else {
                    &source
                },
                if null_index == 1 {
                    null_buffer
                } else {
                    &second
                },
                if null_index == 2 {
                    null_buffer_mut
                } else {
                    &mut rust_destination
                },
            )
        };
        assert_same_i32("merge null", c_result, rust_result);
    }
    let left_128 = random_buffer(&mut rng, 128);
    let right_129 = random_buffer(&mut rng, 129);
    assert_same_i32(
        "merge oversized",
        unsafe { (c.merge)(&left_128, &right_129, &mut c_destination) },
        unsafe { (rust.merge)(&left_128, &right_129, &mut rust_destination) },
    );

    for null_index in 0..3 {
        let mut c_second_destination = Buffer::empty_with(0x62);
        let mut rust_second_destination = c_second_destination;
        let c_result = unsafe {
            (c.split)(
                if null_index == 0 {
                    null_buffer
                } else {
                    &source
                },
                0,
                if null_index == 1 {
                    null_buffer_mut
                } else {
                    &mut c_destination
                },
                if null_index == 2 {
                    null_buffer_mut
                } else {
                    &mut c_second_destination
                },
            )
        };
        let rust_result = unsafe {
            (rust.split)(
                if null_index == 0 {
                    null_buffer
                } else {
                    &source
                },
                0,
                if null_index == 1 {
                    null_buffer_mut
                } else {
                    &mut rust_destination
                },
                if null_index == 2 {
                    null_buffer_mut
                } else {
                    &mut rust_second_destination
                },
            )
        };
        assert_same_i32("split null", c_result, rust_result);
    }
    let mut c_second_destination = Buffer::empty_with(0x63);
    let mut rust_second_destination = c_second_destination;
    assert_same_i32(
        "split past end",
        unsafe {
            (c.split)(
                &source,
                source.length + 1,
                &mut c_destination,
                &mut c_second_destination,
            )
        },
        unsafe {
            (rust.split)(
                &source,
                source.length + 1,
                &mut rust_destination,
                &mut rust_second_destination,
            )
        },
    );

    for null_index in 0..3 {
        let c_result = unsafe {
            (c.interleave)(
                if null_index == 0 {
                    null_buffer
                } else {
                    &source
                },
                if null_index == 1 {
                    null_buffer
                } else {
                    &second
                },
                if null_index == 2 {
                    null_buffer_mut
                } else {
                    &mut c_destination
                },
            )
        };
        let rust_result = unsafe {
            (rust.interleave)(
                if null_index == 0 {
                    null_buffer
                } else {
                    &source
                },
                if null_index == 1 {
                    null_buffer
                } else {
                    &second
                },
                if null_index == 2 {
                    null_buffer_mut
                } else {
                    &mut rust_destination
                },
            )
        };
        assert_same_i32("interleave null", c_result, rust_result);
    }
    assert_same_i32(
        "interleave oversized",
        unsafe { (c.interleave)(&left_128, &right_129, &mut c_destination) },
        unsafe { (rust.interleave)(&left_128, &right_129, &mut rust_destination) },
    );
    assert_same_i32(
        "rotate null",
        unsafe { (c.rotate)(null_buffer_mut, 1) },
        unsafe { (rust.rotate)(null_buffer_mut, 1) },
    );

    for copy_matching in [false, true] {
        assert_same_i32(
            "conditional null source",
            unsafe { (c.conditional)(null_buffer, &mut c_destination, 1, copy_matching) },
            unsafe { (rust.conditional)(null_buffer, &mut rust_destination, 1, copy_matching) },
        );
        assert_same_i32(
            "conditional null destination",
            unsafe { (c.conditional)(&source, null_buffer_mut, 1, copy_matching) },
            unsafe { (rust.conditional)(&source, null_buffer_mut, 1, copy_matching) },
        );
    }
    assert_same_i32(
        "strided null source",
        unsafe { (c.strided)(null_buffer, &mut c_destination, 1) },
        unsafe { (rust.strided)(null_buffer, &mut rust_destination, 1) },
    );
    assert_same_i32(
        "strided null destination",
        unsafe { (c.strided)(&source, null_buffer_mut, 1) },
        unsafe { (rust.strided)(&source, null_buffer_mut, 1) },
    );
    for stride in [0, -1, c_int::MIN] {
        assert_same_i32(
            "invalid stride",
            unsafe { (c.strided)(&source, &mut c_destination, stride) },
            unsafe { (rust.strided)(&source, &mut rust_destination, stride) },
        );
    }

    assert_same_i32(
        "process null",
        unsafe { (c.process)(ptr::null_mut(), 0, 0) },
        unsafe { (rust.process)(ptr::null_mut(), 0, 0) },
    );
    let mut c_storage = [source, second];
    let mut rust_storage = c_storage;
    let mut c_array = BufferArray {
        buffers: c_storage.as_mut_ptr(),
        count: 0,
        capacity: 2,
    };
    let mut rust_array = BufferArray {
        buffers: rust_storage.as_mut_ptr(),
        count: 0,
        capacity: 2,
    };
    assert_same_i32(
        "process zero count",
        unsafe { (c.process)(&mut c_array, 0, 0) },
        unsafe { (rust.process)(&mut rust_array, 0, 0) },
    );
    for operation in [3, 4, -1, 7, c_int::MAX, c_int::MIN] {
        c_array.count = 2;
        rust_array.count = 2;
        assert_same_i32(
            "process unknown operation",
            unsafe { (c.process)(&mut c_array, operation, 0) },
            unsafe { (rust.process)(&mut rust_array, operation, 0) },
        );
    }

    c_array.count = 1;
    rust_array.count = 1;
    assert_same_i32(
        "process merge count one",
        unsafe { (c.process)(&mut c_array, 2, 0) },
        unsafe { (rust.process)(&mut rust_array, 2, 0) },
    );
    unsafe {
        ptr::write(c_array.buffers, oversized);
        ptr::write(rust_array.buffers, oversized);
    }
    c_array.count = 2;
    rust_array.count = 2;
    assert_same_i32(
        "process copy invalid source",
        unsafe { (c.process)(&mut c_array, 0, 0) },
        unsafe { (rust.process)(&mut rust_array, 0, 0) },
    );
    c_array.count = 1;
    rust_array.count = 1;
    assert_same_i32(
        "process checksum invalid source",
        unsafe { (c.process)(&mut c_array, 6, 0) },
        unsafe { (rust.process)(&mut rust_array, 6, 0) },
    );
    c_storage = [left_128, right_129];
    rust_storage = c_storage;
    c_array.buffers = c_storage.as_mut_ptr();
    rust_array.buffers = rust_storage.as_mut_ptr();
    c_array.count = 2;
    rust_array.count = 2;
    assert_same_i32(
        "process merge oversized pair",
        unsafe { (c.process)(&mut c_array, 2, 0) },
        unsafe { (rust.process)(&mut rust_array, 2, 0) },
    );

    let mut c_null_storage = BufferArray {
        buffers: ptr::null_mut(),
        count: 1,
        capacity: 1,
    };
    let mut rust_null_storage = BufferArray {
        buffers: ptr::null_mut(),
        count: 1,
        capacity: 1,
    };
    for operation in [1, 5, 6] {
        assert_same_i32(
            "process nested null storage",
            unsafe { (c.process)(&mut c_null_storage, operation, 1) },
            unsafe { (rust.process)(&mut rust_null_storage, operation, 1) },
        );
    }
    c_null_storage.count = 2;
    rust_null_storage.count = 2;
    for operation in [0, 2] {
        assert_same_i32(
            "process nested null storage",
            unsafe { (c.process)(&mut c_null_storage, operation, 0) },
            unsafe { (rust.process)(&mut rust_null_storage, operation, 0) },
        );
    }
}

#[test]
fn allocation_failure_paths_match() {
    const CHILD_VARIABLE: &str = "DRIVER_MALLOC_FAIL_CHILD";
    if std::env::var_os(CHILD_VARIABLE).is_some() {
        let _guard = io_guard();
        let (c, rust) = load_apis();
        let process = libloading::os::unix::Library::this();
        let set_failure = unsafe {
            *process
                .get::<unsafe extern "C" fn(usize)>(b"set_fail_malloc_size\0")
                .expect("malloc fault-injection control symbol")
        };

        for size in [size_of::<BufferArray>(), size_of::<Buffer>() * 3] {
            let capacity = if size == size_of::<BufferArray>() {
                1
            } else {
                3
            };
            unsafe { set_failure(size) };
            let c_result = unsafe { (c.init)(capacity) };
            unsafe { set_failure(size) };
            let rust_result = unsafe { (rust.init)(capacity) };
            assert_eq!(c_result.is_null(), rust_result.is_null());
            assert!(c_result.is_null(), "C allocation fault was not injected");
            assert!(
                rust_result.is_null(),
                "Rust allocation fault was not injected"
            );
        }

        for size in [size_of::<BufferArray>(), size_of::<Buffer>()] {
            let c_result = capture_call(b"1 1 0\n", || {
                unsafe { set_failure(size) };
                unsafe { (c.main)(0, ptr::null_mut()) }
            });
            let rust_result = capture_call(b"1 1 0\n", || {
                unsafe { set_failure(size) };
                unsafe { (rust.main)(0, ptr::null_mut()) }
            });
            assert_eq!(c_result, rust_result, "main allocation failure size={size}");
            assert_eq!(c_result.0, 1);
        }
        return;
    }

    let root = manifest_dir();
    let shim = root.join("target/libdriver_malloc_fail.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&shim)
        .arg(root.join("tests/malloc_fail.c"))
        .status()
        .expect("run cc for malloc fault-injection shim");
    assert!(status.success());
    assert!(shim.exists());

    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "allocation_failure_paths_match",
            "--test-threads=1",
        ])
        .env(CHILD_VARIABLE, "1")
        .env("LD_PRELOAD", &shim)
        .status()
        .expect("run allocation-fault child test");
    assert!(status.success());
}

unsafe extern "C" {
    static mut stdin: *mut c_void;
    static mut stdout: *mut c_void;
    static mut stderr: *mut c_void;

    fn freopen(path: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
    fn fflush(stream: *mut c_void) -> c_int;
    fn clearerr(stream: *mut c_void);
    fn dup(file_descriptor: c_int) -> c_int;
    fn dup2(old_file_descriptor: c_int, new_file_descriptor: c_int) -> c_int;
    fn close(file_descriptor: c_int) -> c_int;
}

fn capture_call<T>(input: &[u8], call: impl FnOnce() -> T) -> (T, Vec<u8>, Vec<u8>) {
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let prefix = format!("driver-ffi-{}-{id}", std::process::id());
    let input_path = std::env::temp_dir().join(format!("{prefix}.in"));
    let output_path = std::env::temp_dir().join(format!("{prefix}.out"));
    let error_path = std::env::temp_dir().join(format!("{prefix}.err"));
    fs::write(&input_path, input).unwrap();
    fs::write(&output_path, []).unwrap();
    fs::write(&error_path, []).unwrap();

    let input_path_c = CString::new(input_path.as_os_str().as_encoded_bytes()).unwrap();
    let output_path_c = CString::new(output_path.as_os_str().as_encoded_bytes()).unwrap();
    let error_path_c = CString::new(error_path.as_os_str().as_encoded_bytes()).unwrap();
    let read_mode = c"r";
    let write_mode = c"w";

    let saved_stdin;
    let saved_stdout;
    let saved_stderr;
    unsafe {
        fflush(ptr::null_mut());
        saved_stdin = dup(0);
        saved_stdout = dup(1);
        saved_stderr = dup(2);
        assert!(saved_stdin >= 0 && saved_stdout >= 0 && saved_stderr >= 0);
        assert!(!freopen(input_path_c.as_ptr(), read_mode.as_ptr(), stdin).is_null());
        assert!(!freopen(output_path_c.as_ptr(), write_mode.as_ptr(), stdout).is_null());
        assert!(!freopen(error_path_c.as_ptr(), write_mode.as_ptr(), stderr).is_null());
        clearerr(stdin);
        clearerr(stdout);
        clearerr(stderr);
    }

    let value = call();

    unsafe {
        fflush(ptr::null_mut());
        assert_eq!(dup2(saved_stdin, 0), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(dup2(saved_stderr, 2), 2);
        close(saved_stdin);
        close(saved_stdout);
        close(saved_stderr);
        clearerr(stdin);
        clearerr(stdout);
        clearerr(stderr);
    }

    let output = fs::read(&output_path).unwrap();
    let error = fs::read(&error_path).unwrap();
    fs::remove_file(input_path).unwrap();
    fs::remove_file(output_path).unwrap();
    fs::remove_file(error_path).unwrap();
    (value, output, error)
}

fn run_main(api: &Api, input: &[u8]) -> (c_int, Vec<u8>, Vec<u8>) {
    capture_call(input, || unsafe { (api.main)(0, ptr::null_mut()) })
}

fn assert_main_pair(c: &Api, rust: &Api, context: &str, input: &[u8]) {
    let c_result = run_main(c, input);
    let rust_result = run_main(rust, input);
    assert_eq!(c_result.0, rust_result.0, "{context}: return code");
    assert_eq!(c_result.1, rust_result.1, "{context}: stdout");
    assert_eq!(c_result.2, rust_result.2, "{context}: stderr");
}

fn main_input(operation: c_int, buffers: &[Buffer], parameter: Option<c_int>) -> Vec<u8> {
    let mut text = format!("{operation} {}", buffers.len());
    for buffer in buffers {
        text.push_str(&format!(" {}", buffer.length));
        for &byte in &buffer.data[..buffer.length] {
            text.push_str(&format!(" {byte}"));
        }
    }
    if let Some(parameter) = parameter {
        text.push_str(&format!(" {parameter}"));
    }
    text.push('\n');
    text.into_bytes()
}

#[test]
fn stdio_and_main_valid_paths_match() {
    let _guard = io_guard();
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x10ff_1ced_cafe_babe);

    for iteration in 0..16 {
        let length = 1 + rng.usize(WIDTH);
        let mut mismatched = random_buffer(&mut rng, length);
        mismatched.checksum ^= 1;
        let c_result = capture_call(b"", || unsafe { (c.validate)(&mismatched) });
        let rust_result = capture_call(b"", || unsafe { (rust.validate)(&mismatched) });
        assert_eq!(
            c_result, rust_result,
            "validate warning output iter={iteration}"
        );

        let mut c_storage = [mismatched];
        let mut rust_storage = [mismatched];
        let mut c_array = BufferArray {
            buffers: c_storage.as_mut_ptr(),
            count: 1,
            capacity: 1,
        };
        let mut rust_array = BufferArray {
            buffers: rust_storage.as_mut_ptr(),
            count: 1,
            capacity: 1,
        };
        let c_result = capture_call(b"", || unsafe { (c.process)(&mut c_array, 6, 0) });
        let rust_result = capture_call(b"", || unsafe { (rust.process)(&mut rust_array, 6, 0) });
        assert_eq!(
            c_result, rust_result,
            "process checksum warning output iter={iteration}"
        );
    }

    for &length in &[0, 1, 2, 31, 255, 256] {
        for iteration in 0..16 {
            let mut input = format!("{length}");
            let mut expected_data = Vec::new();
            for _ in 0..length {
                let value = rng.next_u64() as c_int;
                expected_data.push(value as u8);
                input.push_str(&format!(" {value}"));
            }
            input.push('\n');

            let mut c_buffer = Buffer::empty_with(0x81);
            let mut rust_buffer = c_buffer;
            let c_result = capture_call(input.as_bytes(), || unsafe { (c.read)(&mut c_buffer) });
            let rust_result = capture_call(input.as_bytes(), || unsafe {
                (rust.read)(&mut rust_buffer)
            });
            assert_eq!(
                c_result, rust_result,
                "read length={length} iter={iteration}"
            );
            assert_buffer_eq("read buffer", &c_buffer, &rust_buffer);
            assert_eq!(&c_buffer.data[..length], expected_data);

            let c_written = capture_call(b"", || unsafe { (c.write)(&c_buffer) });
            let rust_written = capture_call(b"", || unsafe { (rust.write)(&rust_buffer) });
            assert_eq!(
                c_written, rust_written,
                "write length={length} iter={iteration}"
            );
        }
    }

    let shape_lengths = [0, 1, 2, 17, 127, 128, 255, 256];
    for operation in 0..=6 {
        for iteration in 0..24 {
            let (buffers, parameter) = match operation {
                0 => {
                    let length = shape_lengths[iteration % shape_lengths.len()];
                    let second_length = rng.usize(33);
                    (
                        vec![
                            random_buffer(&mut rng, length),
                            random_buffer(&mut rng, second_length),
                        ],
                        None,
                    )
                }
                1 | 6 => {
                    let count = 1 + rng.usize(4);
                    let mut buffers = Vec::new();
                    for index in 0..count {
                        let length = shape_lengths[(iteration + index) % shape_lengths.len()];
                        buffers.push(random_buffer(&mut rng, length));
                    }
                    (buffers, None)
                }
                2 | 4 => {
                    let left = iteration * 11 % 257;
                    let right = 256 - left;
                    (
                        vec![
                            random_buffer(&mut rng, left),
                            random_buffer(&mut rng, right),
                        ],
                        None,
                    )
                }
                3 => {
                    let length = shape_lengths[iteration % shape_lengths.len()];
                    let split = match iteration % 3 {
                        0 => 0,
                        1 => length / 2,
                        _ => length,
                    };
                    (vec![random_buffer(&mut rng, length)], Some(split as c_int))
                }
                5 => {
                    let count = 1 + rng.usize(4);
                    let mut buffers = Vec::new();
                    for index in 0..count {
                        let length = shape_lengths[(iteration + index) % shape_lengths.len()];
                        buffers.push(random_buffer(&mut rng, length));
                    }
                    let positions = match iteration % 5 {
                        0 => 0,
                        1 => 1,
                        2 => -1,
                        3 => c_int::MAX,
                        _ => c_int::MIN,
                    };
                    (buffers, Some(positions))
                }
                _ => unreachable!(),
            };
            let input = main_input(operation, &buffers, parameter);
            assert_main_pair(
                &c,
                &rust,
                &format!("main op={operation} iter={iteration}"),
                &input,
            );
        }
    }
}

#[test]
fn stdio_and_main_error_paths_match() {
    let _guard = io_guard();
    let (c, rust) = load_apis();

    let read_cases: &[(&str, &[u8])] = &[
        ("missing length", b""),
        ("nonnumeric length", b"x"),
        ("negative length", b"-1"),
        ("oversized length", b"257"),
        ("missing first byte", b"1"),
        ("missing later byte", b"3 1 2 x"),
    ];
    for &(context, input) in read_cases {
        let mut c_buffer = Buffer::empty_with(0x91);
        let mut rust_buffer = c_buffer;
        let c_result = capture_call(input, || unsafe { (c.read)(&mut c_buffer) });
        let rust_result = capture_call(input, || unsafe { (rust.read)(&mut rust_buffer) });
        assert_eq!(c_result, rust_result, "read error: {context}");
        assert_buffer_eq(context, &c_buffer, &rust_buffer);
    }

    let c_null_read = capture_call(b"0", || unsafe { (c.read)(ptr::null_mut()) });
    let rust_null_read = capture_call(b"0", || unsafe { (rust.read)(ptr::null_mut()) });
    assert_eq!(c_null_read, rust_null_read);
    let c_null_write = capture_call(b"", || unsafe { (c.write)(ptr::null()) });
    let rust_null_write = capture_call(b"", || unsafe { (rust.write)(ptr::null()) });
    assert_eq!(c_null_write, rust_null_write);

    let main_errors: &[(&str, &[u8])] = &[
        ("missing operation", b""),
        ("invalid operation token", b"x"),
        ("missing count", b"0"),
        ("invalid count token", b"0 x"),
        ("zero count", b"0 0"),
        ("negative count", b"0 -1"),
        ("oversized count", b"0 101"),
        ("buffer missing length", b"1 1"),
        ("buffer negative length", b"1 1 -1"),
        ("buffer oversized length", b"1 1 257"),
        ("buffer missing byte", b"1 1 2 7 x"),
        ("copy count one", b"0 1 0"),
        ("merge count one", b"2 1 0"),
        ("merge oversized", b"2 2 128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 129"),
        ("split missing position", b"3 1 0"),
        ("split negative position", b"3 1 0 -1"),
        ("split past end", b"3 1 1 9 2"),
        ("interleave count one", b"4 1 0"),
        ("rotate missing amount", b"5 1 0"),
        ("unknown negative operation", b"-1 1 0"),
        ("unknown positive operation", b"7 1 0"),
        ("out of range enum", b"2147483647 1 0"),
    ];
    for &(context, input) in main_errors {
        assert_main_pair(&c, &rust, context, input);
    }

    let mut oversized_merge = String::from("2 2 128");
    for _ in 0..128 {
        oversized_merge.push_str(" 1");
    }
    oversized_merge.push_str(" 129");
    for _ in 0..129 {
        oversized_merge.push_str(" 2");
    }
    assert_main_pair(
        &c,
        &rust,
        "main merge combined length 257",
        oversized_merge.as_bytes(),
    );

    let mut oversized_interleave = oversized_merge.into_bytes();
    oversized_interleave[0] = b'4';
    assert_main_pair(
        &c,
        &rust,
        "main interleave combined length 257",
        &oversized_interleave,
    );
}
