use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;

type CreateLinePointers = unsafe extern "C" fn(*mut c_char, usize, usize) -> *mut *const c_char;

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Output {
    Null,
    PointerBytes(Vec<u8>),
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

    fn usize(&mut self, start: usize, end: usize) -> usize {
        assert!(start < end);
        start + self.next_u64() as usize % (end - start)
    }

    fn nonzero_byte(&mut self) -> u8 {
        self.usize(1, 256) as u8
    }
}

fn shared_object(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

fn with_apis(test: impl FnOnce(CreateLinePointers, CreateLinePointers)) {
    let c_path = shared_object("c_src/build/libdriver.so");
    let rust_deps_path = shared_object("target/debug/deps/libdriver.so");
    let rust_path = if rust_deps_path.exists() {
        rust_deps_path
    } else {
        shared_object("target/debug/libdriver.so")
    };

    assert!(
        c_path.exists(),
        "missing C shared library {}; build it with CMake first",
        c_path.display()
    );
    assert!(
        rust_path.exists(),
        "missing Rust shared library {}; build it before running tests",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_symbol: Symbol<CreateLinePointers> = c_library
            .get(b"UTIL_createLinePointers\0")
            .expect("resolve C symbol");
        let rust_symbol: Symbol<CreateLinePointers> = rust_library
            .get(b"UTIL_createLinePointers\0")
            .expect("resolve Rust symbol");

        assert_ne!(
            *c_symbol as usize, *rust_symbol as usize,
            "both symbols unexpectedly resolved to the same implementation"
        );
        test(*c_symbol, *rust_symbol);
    }
}

unsafe fn call(
    function: CreateLinePointers,
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> Output {
    let result = unsafe { function(buffer, num_lines, buffer_size) };
    if result.is_null() {
        return Output::Null;
    }

    let byte_len = num_lines
        .checked_mul(size_of::<*const c_char>())
        .expect("test output length overflow");
    let bytes = unsafe { slice::from_raw_parts(result.cast::<u8>(), byte_len) }.to_vec();
    unsafe { free(result.cast::<c_void>()) };
    Output::PointerBytes(bytes)
}

fn compare_case(
    c_function: CreateLinePointers,
    rust_function: CreateLinePointers,
    buffer: &mut [u8],
    num_lines: usize,
    buffer_size: usize,
) -> Output {
    let buffer_pointer = if buffer.is_empty() {
        ptr::null_mut()
    } else {
        buffer.as_mut_ptr().cast::<c_char>()
    };

    unsafe {
        let c_output = call(c_function, buffer_pointer, num_lines, buffer_size);
        let rust_output = call(rust_function, buffer_pointer, num_lines, buffer_size);
        assert_eq!(
            c_output, rust_output,
            "mismatch for num_lines={num_lines}, buffer_size={buffer_size}, buffer={buffer:?}"
        );
        c_output
    }
}

fn push_random_nonzero(buffer: &mut Vec<u8>, rng: &mut Rng, length: usize) {
    for _ in 0..length {
        buffer.push(rng.nonzero_byte());
    }
}

#[test]
fn config_01_zero_lines_zero_size() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x0101_0101_0101_0101);
        for iteration in 0..256 {
            let mut buffer = if iteration % 2 == 0 {
                Vec::new()
            } else {
                let length = rng.usize(1, 65);
                (0..length).map(|_| rng.next_u64() as u8).collect()
            };
            let output = compare_case(c_function, rust_function, &mut buffer, 0, 0);
            assert!(matches!(output, Output::PointerBytes(_)));
        }
    });
}

#[test]
fn config_02_zero_lines_nonzero_size_input_is_ignored() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x0202_0202_0202_0202);
        for iteration in 0..256 {
            let declared_size = rng.usize(1, 129);
            let mut buffer = if iteration % 2 == 0 {
                Vec::new()
            } else {
                (0..declared_size).map(|_| rng.next_u64() as u8).collect()
            };
            let output = compare_case(c_function, rust_function, &mut buffer, 0, declared_size);
            assert!(matches!(output, Output::PointerBytes(_)));
        }
    });
}

#[test]
fn config_03_one_empty_line() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x0303_0303_0303_0303);
        for _ in 0..256 {
            let trailing_length = rng.usize(0, 65);
            let mut buffer = vec![0];
            buffer.extend((0..trailing_length).map(|_| rng.next_u64() as u8));
            let declared_size = rng.usize(1, buffer.len() + 1);
            assert!(matches!(
                compare_case(c_function, rust_function, &mut buffer, 1, declared_size),
                Output::PointerBytes(_)
            ));
        }
    });
}

#[test]
fn config_04_one_nonempty_terminated_line() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x0404_0404_0404_0404);
        for _ in 0..256 {
            let line_length = rng.usize(1, 65);
            let trailing_length = rng.usize(0, 33);
            let mut buffer = Vec::new();
            push_random_nonzero(&mut buffer, &mut rng, line_length);
            buffer.push(0);
            buffer.extend((0..trailing_length).map(|_| rng.next_u64() as u8));
            let declared_size = rng.usize(line_length + 1, buffer.len() + 1);
            assert!(matches!(
                compare_case(c_function, rust_function, &mut buffer, 1, declared_size),
                Output::PointerBytes(_)
            ));
        }
    });
}

#[test]
fn config_05_one_nonempty_unterminated_line() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x0505_0505_0505_0505);
        for _ in 0..256 {
            let line_length = rng.usize(1, 129);
            let mut buffer = Vec::new();
            push_random_nonzero(&mut buffer, &mut rng, line_length);
            assert!(matches!(
                compare_case(c_function, rust_function, &mut buffer, 1, line_length),
                Output::PointerBytes(_)
            ));
        }
    });
}

#[test]
fn config_06_multiple_nonempty_terminated_lines() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x0606_0606_0606_0606);
        for _ in 0..256 {
            let line_count = rng.usize(2, 17);
            let mut buffer = Vec::new();
            for _ in 0..line_count {
                let line_length = rng.usize(1, 33);
                push_random_nonzero(&mut buffer, &mut rng, line_length);
                buffer.push(0);
            }
            let buffer_size = buffer.len();
            assert!(matches!(
                compare_case(
                    c_function,
                    rust_function,
                    &mut buffer,
                    line_count,
                    buffer_size
                ),
                Output::PointerBytes(_)
            ));
        }
    });
}

#[test]
fn config_07_multiple_lines_with_empty_interior_lines() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x0707_0707_0707_0707);
        for _ in 0..256 {
            let line_count = rng.usize(3, 17);
            let empty_index = rng.usize(1, line_count - 1);
            let mut buffer = Vec::new();
            for line_index in 0..line_count {
                let line_length = if line_index == empty_index {
                    0
                } else {
                    rng.usize(1, 33)
                };
                push_random_nonzero(&mut buffer, &mut rng, line_length);
                buffer.push(0);
            }
            let buffer_size = buffer.len();
            assert!(matches!(
                compare_case(
                    c_function,
                    rust_function,
                    &mut buffer,
                    line_count,
                    buffer_size
                ),
                Output::PointerBytes(_)
            ));
        }
    });
}

#[test]
fn config_08_multiple_lines_with_unterminated_final_line() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x0808_0808_0808_0808);
        for _ in 0..256 {
            let line_count = rng.usize(2, 17);
            let mut buffer = Vec::new();
            for line_index in 0..line_count {
                let line_length = rng.usize(1, 33);
                push_random_nonzero(&mut buffer, &mut rng, line_length);
                if line_index + 1 != line_count {
                    buffer.push(0);
                }
            }
            let buffer_size = buffer.len();
            assert!(matches!(
                compare_case(
                    c_function,
                    rust_function,
                    &mut buffer,
                    line_count,
                    buffer_size
                ),
                Output::PointerBytes(_)
            ));
        }
    });
}

#[test]
fn config_09_requested_prefix_of_available_lines() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x0909_0909_0909_0909);
        for _ in 0..256 {
            let available_count = rng.usize(2, 17);
            let requested_count = rng.usize(1, available_count);
            let mut buffer = Vec::new();
            for _ in 0..available_count {
                let line_length = rng.usize(0, 33);
                push_random_nonzero(&mut buffer, &mut rng, line_length);
                buffer.push(0);
            }
            let buffer_size = buffer.len();
            assert!(matches!(
                compare_case(
                    c_function,
                    rust_function,
                    &mut buffer,
                    requested_count,
                    buffer_size
                ),
                Output::PointerBytes(_)
            ));
        }
    });
}

#[test]
fn config_10_declared_size_truncates_backing_buffer() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0x1010_1010_1010_1010);
        for _ in 0..256 {
            let declared_size = rng.usize(1, 129);
            let mut buffer = Vec::new();
            push_random_nonzero(&mut buffer, &mut rng, declared_size);
            buffer.push(0);
            buffer.extend((0..rng.usize(0, 33)).map(|_| rng.next_u64() as u8));
            assert!(matches!(
                compare_case(c_function, rust_function, &mut buffer, 1, declared_size),
                Output::PointerBytes(_)
            ));
        }
    });
}

#[test]
fn error_01_allocation_failure_returns_null() {
    with_apis(|c_function, rust_function| {
        let pointer_size = size_of::<*const c_char>();
        let num_lines = usize::MAX / pointer_size;
        let mut buffer = Vec::new();
        assert_eq!(
            compare_case(c_function, rust_function, &mut buffer, num_lines, 0),
            Output::Null
        );
    });
}

#[test]
fn error_02_insufficient_line_starts_returns_null() {
    with_apis(|c_function, rust_function| {
        let mut rng = Rng::new(0xeeee_0202_eeee_0202);

        let mut empty_buffer = Vec::new();
        for num_lines in 1..=64 {
            assert_eq!(
                compare_case(c_function, rust_function, &mut empty_buffer, num_lines, 0),
                Output::Null
            );
        }

        for _ in 0..256 {
            let available_count = rng.usize(1, 17);
            let requested_count = available_count + rng.usize(1, 9);
            let mut buffer = Vec::new();
            for _ in 0..available_count {
                let line_length = rng.usize(0, 33);
                push_random_nonzero(&mut buffer, &mut rng, line_length);
                buffer.push(0);
            }
            let buffer_size = buffer.len();
            assert_eq!(
                compare_case(
                    c_function,
                    rust_function,
                    &mut buffer,
                    requested_count,
                    buffer_size
                ),
                Output::Null
            );
        }
    });
}
