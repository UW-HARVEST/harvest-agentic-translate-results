use libloading::Library;
use std::ffi::{c_char, c_void};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

type CreateLinePointers = unsafe extern "C" fn(*mut c_char, usize, usize) -> *mut *const c_char;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Api {
    _library: Library,
    create_line_pointers: CreateLinePointers,
}

impl Api {
    fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let create_line_pointers = unsafe {
            *library
                .get::<CreateLinePointers>(b"UTIL_createLinePointers\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load UTIL_createLinePointers from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            create_line_pointers,
        }
    }

    fn offsets(
        &self,
        buffer: Option<&mut [u8]>,
        num_lines: usize,
        buffer_size: usize,
    ) -> Option<Vec<usize>> {
        let buffer_ptr = buffer
            .map(|bytes| bytes.as_mut_ptr().cast::<c_char>())
            .unwrap_or(ptr::null_mut());
        let result = unsafe { (self.create_line_pointers)(buffer_ptr, num_lines, buffer_size) };
        if result.is_null() {
            return None;
        }

        let mut offsets = Vec::with_capacity(num_lines);
        for index in 0..num_lines {
            let line_ptr = unsafe { *result.add(index) };
            assert!(
                !buffer_ptr.is_null(),
                "a non-empty output requires a non-null input buffer"
            );
            let line_address = line_ptr as usize;
            let buffer_address = buffer_ptr as usize;
            let offset = line_address
                .checked_sub(buffer_address)
                .expect("returned line pointer precedes the input buffer");
            assert!(
                offset < buffer_size,
                "returned line pointer is outside the advertised input buffer"
            );
            offsets.push(offset);
        }

        unsafe { free(result.cast::<c_void>()) };
        Some(offsets)
    }

    fn returns_null(&self, buffer: *mut c_char, num_lines: usize, buffer_size: usize) -> bool {
        let result = unsafe { (self.create_line_pointers)(buffer, num_lines, buffer_size) };
        if result.is_null() {
            true
        } else {
            unsafe { free(result.cast::<c_void>()) };
            false
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libdriver.so");
        let rust_path = manifest.join("target/release/libdriver.so");
        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}; run cargo build --release first",
            rust_path.display()
        );
        Self {
            c: Api::load(&c_path),
            rust: Api::load(&rust_path),
        }
    }

    fn compare(&self, input: &[u8], num_lines: usize, expected: &[usize]) {
        let mut c_input = input.to_vec();
        let mut rust_input = input.to_vec();
        let c_output = self.c.offsets(Some(&mut c_input), num_lines, input.len());
        let rust_output = self
            .rust
            .offsets(Some(&mut rust_input), num_lines, input.len());
        assert_eq!(
            c_output.as_deref(),
            Some(expected),
            "C result for {input:?}"
        );
        assert_eq!(rust_output, c_output, "Rust diverged for {input:?}");
        assert_eq!(c_input, input, "C modified the input buffer");
        assert_eq!(rust_input, input, "Rust modified the input buffer");
    }
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

    fn usize(&mut self, minimum: usize, maximum_inclusive: usize) -> usize {
        assert!(minimum <= maximum_inclusive);
        minimum + (self.next_u64() as usize % (maximum_inclusive - minimum + 1))
    }

    fn nonzero_byte(&mut self) -> u8 {
        (self.usize(1, u8::MAX as usize)) as u8
    }

    fn any_byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

fn append_nonempty_line(buffer: &mut Vec<u8>, rng: &mut Rng, length: usize) {
    buffer.extend((0..length).map(|_| rng.nonzero_byte()));
}

#[test]
fn config_01_zero_lines_zero_buffer() {
    let pair = Pair::load();
    for _ in 0..256 {
        let c_output = pair.c.offsets(None, 0, 0);
        let rust_output = pair.rust.offsets(None, 0, 0);
        assert_eq!(rust_output, c_output);
    }
}

#[test]
fn config_02_zero_lines_nonempty_buffer_is_ignored() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x02_6f4b_5b17_f45d);
    for _ in 0..256 {
        let length = rng.usize(1, 256);
        let input: Vec<u8> = (0..length).map(|_| rng.any_byte()).collect();
        pair.compare(&input, 0, &[]);

        let c_null = pair.c.offsets(None, 0, length);
        let rust_null = pair.rust.offsets(None, 0, length);
        assert_eq!(rust_null, c_null, "null ignored buffer, size {length}");
    }
}

#[test]
fn config_03_one_nonterminated_line_reaches_buffer_boundary() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x03_35ab_90c1_7e21);
    for _ in 0..512 {
        let length = rng.usize(1, 256);
        let input: Vec<u8> = (0..length).map(|_| rng.nonzero_byte()).collect();
        pair.compare(&input, 1, &[0]);
    }
}

#[test]
fn config_04_one_empty_line() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x04_d3ee_130c_a181);
    for _ in 0..512 {
        let trailing_length = rng.usize(0, 128);
        let mut input = vec![0];
        input.extend((0..trailing_length).map(|_| rng.any_byte()));
        pair.compare(&input, 1, &[0]);
    }
}

#[test]
fn config_05_one_terminated_line_ignores_remaining_bytes() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x05_b110_1c3d_0f97);
    for _ in 0..512 {
        let line_length = rng.usize(1, 64);
        let trailing_length = rng.usize(1, 128);
        let mut input = Vec::new();
        append_nonempty_line(&mut input, &mut rng, line_length);
        input.push(0);
        input.extend((0..trailing_length).map(|_| rng.any_byte()));
        pair.compare(&input, 1, &[0]);
    }
}

#[test]
fn config_06_multiple_nonempty_lines_final_line_is_not_terminated() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x06_150d_29f3_7c55);
    for _ in 0..512 {
        let line_count = rng.usize(2, 24);
        let mut input = Vec::new();
        let mut expected = Vec::new();
        for line in 0..line_count {
            expected.push(input.len());
            let line_length = rng.usize(1, 32);
            append_nonempty_line(&mut input, &mut rng, line_length);
            if line + 1 != line_count {
                input.push(0);
            }
        }
        pair.compare(&input, line_count, &expected);
    }
}

#[test]
fn config_07_multiple_lines_include_leading_and_consecutive_nulls() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x07_8d8c_e207_3b91);
    for _ in 0..512 {
        let line_count = rng.usize(3, 24);
        let mut input = Vec::new();
        let mut expected = Vec::new();
        for line in 0..line_count {
            expected.push(input.len());
            let line_length = if line < 2 { 0 } else { rng.usize(0, 24) };
            append_nonempty_line(&mut input, &mut rng, line_length);
            if line + 1 != line_count || line_length == 0 {
                input.push(0);
            }
        }
        pair.compare(&input, line_count, &expected);
    }
}

#[test]
fn config_08_final_line_has_a_terminator_at_buffer_end() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x08_18fe_771a_15c3);
    for _ in 0..512 {
        let line_count = rng.usize(2, 24);
        let mut input = Vec::new();
        let mut expected = Vec::new();
        for _ in 0..line_count {
            expected.push(input.len());
            let line_length = rng.usize(1, 32);
            append_nonempty_line(&mut input, &mut rng, line_length);
            input.push(0);
        }
        pair.compare(&input, line_count, &expected);
    }
}

#[test]
fn config_09_requested_count_smaller_than_available_lines() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x09_498d_641f_b13b);
    for _ in 0..512 {
        let requested = rng.usize(1, 12);
        let total = requested + rng.usize(1, 12);
        let mut input = Vec::new();
        let mut offsets = Vec::new();
        for line in 0..total {
            offsets.push(input.len());
            let line_length = rng.usize(0, 24);
            append_nonempty_line(&mut input, &mut rng, line_length);
            if line + 1 != total || line_length == 0 {
                input.push(0);
            }
        }
        pair.compare(&input, requested, &offsets[..requested]);
    }
}

#[test]
fn error_01_allocation_failure_returns_null() {
    let pair = Pair::load();
    for num_lines in [usize::MAX, usize::MAX - 1, usize::MAX / 2] {
        let c_null = pair.c.returns_null(ptr::null_mut(), num_lines, 0);
        let rust_null = pair.rust.returns_null(ptr::null_mut(), num_lines, 0);
        assert!(
            c_null,
            "C allocation unexpectedly succeeded for {num_lines}"
        );
        assert_eq!(rust_null, c_null, "allocation failure for {num_lines}");
    }
}

#[test]
fn error_02_fewer_line_starts_than_requested_returns_null() {
    let pair = Pair::load();

    for num_lines in 1..=128 {
        let c_null = pair.c.returns_null(ptr::null_mut(), num_lines, 0);
        let rust_null = pair.rust.returns_null(ptr::null_mut(), num_lines, 0);
        assert!(c_null);
        assert_eq!(rust_null, c_null, "zero-sized buffer, {num_lines} lines");
    }

    let mut rng = Rng::new(0xe2_47c6_89d1_572f);
    for _ in 0..512 {
        let available = rng.usize(1, 24);
        let requested = available + rng.usize(1, 24);
        let mut c_input = Vec::new();
        for line in 0..available {
            let line_length = rng.usize(1, 24);
            append_nonempty_line(&mut c_input, &mut rng, line_length);
            if line + 1 != available {
                c_input.push(0);
            }
        }
        let mut rust_input = c_input.clone();
        let c_null = pair.c.returns_null(
            c_input.as_mut_ptr().cast::<c_char>(),
            requested,
            c_input.len(),
        );
        let rust_null = pair.rust.returns_null(
            rust_input.as_mut_ptr().cast::<c_char>(),
            requested,
            rust_input.len(),
        );
        assert!(c_null, "C accepted {requested} lines in {available} starts");
        assert_eq!(rust_null, c_null);
    }
}

#[test]
fn generic_boundaries_match() {
    let pair = Pair::load();

    // A null buffer is defined here because numLines == 0 prevents all access,
    // even when the advertised size is maximal.
    for buffer_size in [0, 1, usize::MAX / 2, usize::MAX] {
        let c_output = pair.c.offsets(None, 0, buffer_size);
        let rust_output = pair.rust.offsets(None, 0, buffer_size);
        assert_eq!(rust_output, c_output, "null buffer, size {buffer_size}");
    }

    // One line is the largest successful request for a non-null-terminated
    // one-byte buffer; one step beyond it must use the C rejection sentinel.
    let mut c_input = [b'x'];
    let mut rust_input = c_input;
    assert_eq!(
        pair.c.offsets(Some(&mut c_input), 1, 1),
        pair.rust.offsets(Some(&mut rust_input), 1, 1)
    );
    let c_null = pair
        .c
        .returns_null(c_input.as_mut_ptr().cast::<c_char>(), 2, 1);
    let rust_null = pair
        .rust
        .returns_null(rust_input.as_mut_ptr().cast::<c_char>(), 2, 1);
    assert!(c_null);
    assert_eq!(rust_null, c_null);
}

#[test]
fn null_dereference_subprocess_probe() {
    let Ok(library_path) = std::env::var("DRIVER_NULL_PROBE_LIBRARY") else {
        return;
    };
    let api = Api::load(Path::new(&library_path));
    let _ = api.returns_null(ptr::null_mut(), 1, 1);
}

#[test]
fn generic_null_pointer_dereference_outcome_matches() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let current_test = std::env::current_exe().expect("current integration-test executable");
    let libraries = [
        manifest.join("../c_src/build/libdriver.so"),
        manifest.join("target/release/libdriver.so"),
    ];

    let outcomes: Vec<_> = libraries
        .iter()
        .map(|library| {
            let output = Command::new(&current_test)
                .args([
                    "--exact",
                    "null_dereference_subprocess_probe",
                    "--nocapture",
                ])
                .env("DRIVER_NULL_PROBE_LIBRARY", library)
                .output()
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to run null probe for {}: {error}",
                        library.display()
                    )
                });
            (
                output.status.success(),
                output.status.code(),
                output.status.signal(),
            )
        })
        .collect();

    assert_eq!(outcomes[1], outcomes[0], "null-pointer process outcome");
}
