use libloading::Library;
use std::ffi::{c_char, c_void};
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr;
use std::slice;

type CreateLinePointers = unsafe extern "C" fn(*mut c_char, usize, usize) -> *mut *const c_char;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Harness {
    _c_library: Library,
    _rust_library: Library,
    c_create: CreateLinePointers,
    rust_create: CreateLinePointers,
}

impl Harness {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libdriver.so");
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let rust_path = manifest.join("target").join(profile).join("libdriver.so");

        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_create = *c_library
                .get::<CreateLinePointers>(b"UTIL_createLinePointers\0")
                .expect("C symbol UTIL_createLinePointers");
            let rust_create = *rust_library
                .get::<CreateLinePointers>(b"UTIL_createLinePointers\0")
                .expect("Rust symbol UTIL_createLinePointers");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_create,
                rust_create,
            }
        }
    }

    fn compare(&self, buffer: &mut [u8], num_lines: usize) {
        self.compare_ptr(
            buffer.as_mut_ptr().cast::<c_char>(),
            num_lines,
            buffer.len(),
        );
    }

    fn compare_ptr(&self, buffer: *mut c_char, num_lines: usize, buffer_size: usize) {
        let c_result = unsafe { capture(self.c_create, buffer, num_lines, buffer_size) };
        let rust_result = unsafe { capture(self.rust_create, buffer, num_lines, buffer_size) };
        assert_eq!(
            c_result, rust_result,
            "result mismatch for num_lines={num_lines}, buffer_size={buffer_size}"
        );
    }
}

unsafe fn capture(
    create: CreateLinePointers,
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> Option<Vec<u8>> {
    let result = unsafe { create(buffer, num_lines, buffer_size) };
    if result.is_null() {
        return None;
    }

    let output_size = num_lines
        .checked_mul(size_of::<*const c_char>())
        .expect("successful output pointer array size");
    let bytes = unsafe { slice::from_raw_parts(result.cast::<u8>(), output_size) }.to_vec();
    unsafe { free(result.cast::<c_void>()) };
    Some(bytes)
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

    fn range(&mut self, start: usize, end: usize) -> usize {
        assert!(start < end);
        start + (self.next_u64() as usize % (end - start))
    }

    fn nonzero_byte(&mut self) -> u8 {
        (self.next_u64() % 255 + 1) as u8
    }

    fn nonzero_bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.nonzero_byte()).collect()
    }
}

#[test]
fn config_01_zero_lines_empty_buffer() {
    let harness = Harness::load();
    for _ in 0..128 {
        let mut empty = Vec::new();
        harness.compare(&mut empty, 0);
        harness.compare_ptr(ptr::null_mut(), 0, 0);
    }
}

#[test]
fn config_02_zero_lines_nonempty_buffer_is_ignored() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x02c0_ffee_1234_5678);
    for _ in 0..256 {
        let length = rng.range(1, 257);
        let mut buffer: Vec<u8> = (0..length).map(|_| rng.next_u64() as u8).collect();
        harness.compare(&mut buffer, 0);
    }

    harness.compare_ptr(ptr::null_mut(), 0, usize::MAX);
}

#[test]
fn config_03_one_nonempty_line_terminated_at_end() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x03c0_ffee_1234_5678);
    for _ in 0..256 {
        let length = rng.range(1, 129);
        let mut buffer = rng.nonzero_bytes(length);
        buffer.push(0);
        harness.compare(&mut buffer, 1);
    }
}

#[test]
fn config_04_one_nonempty_unterminated_line() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x04c0_ffee_1234_5678);
    for _ in 0..256 {
        let length = rng.range(1, 129);
        let mut buffer = rng.nonzero_bytes(length);
        harness.compare(&mut buffer, 1);
    }
}

#[test]
fn config_05_one_empty_line() {
    let harness = Harness::load();
    for _ in 0..128 {
        let mut buffer = [0u8];
        harness.compare(&mut buffer, 1);
    }
}

#[test]
fn config_06_one_terminated_line_with_extra_bytes() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x06c0_ffee_1234_5678);
    for _ in 0..256 {
        let line_length = rng.range(1, 65);
        let extra_length = rng.range(1, 129);
        let mut buffer = rng.nonzero_bytes(line_length);
        buffer.push(0);
        buffer.extend((0..extra_length).map(|_| rng.next_u64() as u8));
        harness.compare(&mut buffer, 1);
    }
}

#[test]
fn config_07_many_nonempty_terminated_lines_exactly_consume_buffer() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x07c0_ffee_1234_5678);
    for _ in 0..256 {
        let num_lines = rng.range(2, 17);
        let mut buffer = Vec::new();
        for _ in 0..num_lines {
            let length = rng.range(1, 65);
            buffer.extend(rng.nonzero_bytes(length));
            buffer.push(0);
        }
        harness.compare(&mut buffer, num_lines);
    }
}

#[test]
fn config_08_many_lines_including_empty_lines() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x08c0_ffee_1234_5678);
    for iteration in 0..384 {
        let num_lines = rng.range(3, 17);
        let forced_empty = match iteration % 3 {
            0 => 0,
            1 => rng.range(1, num_lines - 1),
            _ => num_lines - 1,
        };
        let mut buffer = Vec::new();
        for line in 0..num_lines {
            if line != forced_empty {
                let length = rng.range(1, 33);
                buffer.extend(rng.nonzero_bytes(length));
            }
            buffer.push(0);
        }
        harness.compare(&mut buffer, num_lines);
    }
}

#[test]
fn config_09_many_lines_with_unterminated_final_line() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x09c0_ffee_1234_5678);
    for _ in 0..256 {
        let num_lines = rng.range(2, 17);
        let mut buffer = Vec::new();
        for _ in 0..num_lines - 1 {
            let length = rng.range(1, 33);
            buffer.extend(rng.nonzero_bytes(length));
            buffer.push(0);
        }
        let final_length = rng.range(1, 65);
        buffer.extend(rng.nonzero_bytes(final_length));
        harness.compare(&mut buffer, num_lines);
    }
}

#[test]
fn config_10_requested_prefix_with_unrequested_data() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x10c0_ffee_1234_5678);
    for _ in 0..256 {
        let num_lines = rng.range(2, 17);
        let mut buffer = Vec::new();
        for _ in 0..num_lines {
            let length = rng.range(0, 33);
            buffer.extend(rng.nonzero_bytes(length));
            buffer.push(0);
        }
        let extra_length = rng.range(1, 129);
        buffer.extend((0..extra_length).map(|_| rng.next_u64() as u8));
        harness.compare(&mut buffer, num_lines);
    }
}

#[test]
fn error_01_allocation_failure_returns_null() {
    let harness = Harness::load();
    let c_result = unsafe { (harness.c_create)(ptr::null_mut(), usize::MAX, 0) };
    let rust_result = unsafe { (harness.rust_create)(ptr::null_mut(), usize::MAX, 0) };

    assert!(
        c_result.is_null(),
        "C unexpectedly satisfied an oversized allocation"
    );
    assert_eq!(c_result.is_null(), rust_result.is_null());
    if !rust_result.is_null() {
        unsafe { free(rust_result.cast::<c_void>()) };
    }
}

#[test]
fn error_02_fewer_line_starts_than_requested_returns_null() {
    let harness = Harness::load();
    let mut rng = Rng::new(0xe220_ffee_1234_5678);

    harness.compare_ptr(ptr::null_mut(), 1, 0);
    for _ in 0..256 {
        let available_lines = rng.range(1, 17);
        let missing_lines = rng.range(1, 9);
        let mut buffer = Vec::new();
        for _ in 0..available_lines {
            let length = rng.range(0, 33);
            buffer.extend(rng.nonzero_bytes(length));
            buffer.push(0);
        }

        let requested_lines = available_lines + missing_lines;
        let c_result = unsafe {
            (harness.c_create)(
                buffer.as_mut_ptr().cast::<c_char>(),
                requested_lines,
                buffer.len(),
            )
        };
        let rust_result = unsafe {
            (harness.rust_create)(
                buffer.as_mut_ptr().cast::<c_char>(),
                requested_lines,
                buffer.len(),
            )
        };
        assert!(c_result.is_null(), "C accepted an insufficient buffer");
        assert_eq!(c_result.is_null(), rust_result.is_null());
        if !rust_result.is_null() {
            unsafe { free(rust_result.cast::<c_void>()) };
        }
    }
}
