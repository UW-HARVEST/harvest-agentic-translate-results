use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type Bin2Hex = unsafe extern "C" fn(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char;

const C_LIBRARY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../c_src/build/libharvest-work-TP6lE1.so"
);
const RUST_LIBRARY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/target/release/libbin2hex_lib.so"
);

struct Libraries {
    _c: Library,
    _rust: Library,
    c_bin2hex: Bin2Hex,
    rust_bin2hex: Bin2Hex,
}

impl Libraries {
    fn load() -> Self {
        unsafe {
            let c = Library::new(C_LIBRARY).expect("load C shared library");
            let rust = Library::new(RUST_LIBRARY).expect("load Rust shared library");
            let c_bin2hex: Symbol<Bin2Hex> = c.get(b"bin2hex\0").expect("load C bin2hex");
            let rust_bin2hex: Symbol<Bin2Hex> = rust.get(b"bin2hex\0").expect("load Rust bin2hex");
            let c_bin2hex = *c_bin2hex;
            let rust_bin2hex = *rust_bin2hex;
            Self {
                _c: c,
                _rust: rust,
                c_bin2hex,
                rust_bin2hex,
            }
        }
    }

    fn assert_valid_case(&self, input: &[u8], capacity: usize, seed: u64) {
        assert!(capacity > input.len() * 2);

        let mut rng = Rng::new(seed);
        let mut initial = vec![0_u8; capacity + 16];
        rng.fill(&mut initial);
        let mut c_output = initial.clone();
        let mut rust_output = initial;

        let c_base = c_output.as_mut_ptr();
        let rust_base = rust_output.as_mut_ptr();
        let c_return =
            unsafe { (self.c_bin2hex)(c_base.cast(), capacity, input.as_ptr(), input.len()) };
        let rust_return =
            unsafe { (self.rust_bin2hex)(rust_base.cast(), capacity, input.as_ptr(), input.len()) };

        assert_eq!(
            c_return.cast::<u8>(),
            c_base,
            "C returned a different pointer"
        );
        assert_eq!(
            rust_return.cast::<u8>(),
            rust_base,
            "Rust returned a different pointer"
        );
        assert_eq!(
            rust_output,
            c_output,
            "output mismatch for input length {} and capacity {capacity}",
            input.len()
        );
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn range(&mut self, start: usize, end: usize) -> usize {
        start + (self.next_u64() as usize % (end - start))
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

#[test]
fn config_1_empty_minimum_capacity() {
    let libraries = Libraries::load();
    for seed in 1..=256 {
        libraries.assert_valid_case(&[], 1, seed);
    }
}

#[test]
fn config_2_empty_spare_capacity() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x8c3c_010c_b475_4c9d);
    for iteration in 0..256 {
        let capacity = rng.range(2, 258);
        libraries.assert_valid_case(&[], capacity, rng.next_u64() ^ iteration);
    }
}

#[test]
fn config_3_one_byte_minimum_capacity() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x243f_6a88_85a3_08d3);
    for iteration in 0..512 {
        let byte = if iteration == 0 {
            0x00
        } else if iteration == 1 {
            0xff
        } else {
            rng.next_u64() as u8
        };
        libraries.assert_valid_case(&[byte], 3, rng.next_u64());
    }
}

#[test]
fn config_4_one_byte_spare_capacity() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x1319_8a2e_0370_7344);
    for iteration in 0..512 {
        let byte = if iteration == 0 {
            0x00
        } else if iteration == 1 {
            0xff
        } else {
            rng.next_u64() as u8
        };
        let capacity = rng.range(4, 68);
        libraries.assert_valid_case(&[byte], capacity, rng.next_u64());
    }
}

#[test]
fn config_5_many_bytes_minimum_capacity() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa409_3822_299f_31d0);
    for iteration in 0..512 {
        let length = rng.range(2, 258);
        let mut input = vec![0_u8; length];
        rng.fill(&mut input);
        input[0] = 0x00;
        input[length - 1] = 0xff;
        libraries.assert_valid_case(&input, length * 2 + 1, rng.next_u64() ^ iteration);
    }
}

#[test]
fn config_6_many_bytes_spare_capacity() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x082e_fa98_ec4e_6c89);
    for iteration in 0..512 {
        let length = rng.range(2, 258);
        let mut input = vec![0_u8; length];
        rng.fill(&mut input);
        input[0] = 0x00;
        input[length - 1] = 0xff;
        let capacity = length * 2 + 1 + rng.range(1, 65);
        libraries.assert_valid_case(&input, capacity, rng.next_u64() ^ iteration);
    }
}

#[test]
fn error_1_oversized_length_aborts() {
    for length in [usize::MAX / 2, usize::MAX / 2 + 1, usize::MAX] {
        assert_matching_signal("oversized", Some(length), 6);
    }
}

#[test]
fn error_2_insufficient_capacity_aborts() {
    for (length, capacity) in [
        (0, 0),
        (1, 0),
        (1, 1),
        (1, 2),
        (17, 33),
        (17, 34),
        (1024, 2048),
    ] {
        assert_matching_signal("insufficient", Some(pack_lengths(length, capacity)), 6);
    }
}

#[test]
fn generic_1_null_output_pointer_matches() {
    assert_matching_signal("null_output", None, 11);
}

#[test]
fn generic_2_null_input_pointer_matches() {
    assert_matching_signal("null_input", None, 11);
}

#[test]
fn generic_3_null_input_with_zero_length_is_accepted() {
    unsafe {
        for path in [Path::new(C_LIBRARY), Path::new(RUST_LIBRARY)] {
            let library = Library::new(path).expect("load shared library");
            let bin2hex: Symbol<Bin2Hex> = library.get(b"bin2hex\0").expect("load bin2hex");
            let mut output = [0xa5_u8; 8];
            let base = output.as_mut_ptr();
            let returned = bin2hex(base.cast(), output.len(), std::ptr::null(), 0);
            assert_eq!(returned.cast::<u8>(), base);
            assert_eq!(output, [0, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5]);
        }
    }
}

fn pack_lengths(length: usize, capacity: usize) -> usize {
    assert!(length <= u32::MAX as usize);
    assert!(capacity <= u32::MAX as usize);
    (length << 32) | capacity
}

fn unpack_lengths(value: usize) -> (usize, usize) {
    (value >> 32, value & u32::MAX as usize)
}

fn assert_matching_signal(case: &str, value: Option<usize>, expected_signal: i32) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_child(C_LIBRARY, case, value);
    let rust_status = run_child(RUST_LIBRARY, case, value);
    assert_eq!(
        c_status.signal(),
        Some(expected_signal),
        "C status for {case}: {c_status:?}"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "Rust status for {case}: {rust_status:?}; C status: {c_status:?}"
    );
}

fn run_child(library: &str, case: &str, value: Option<usize>) -> ExitStatus {
    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command
        .arg("--exact")
        .arg("ffi_child")
        .arg("--nocapture")
        .env("BIN2HEX_CHILD_LIBRARY", library)
        .env("BIN2HEX_CHILD_CASE", case);
    if let Some(value) = value {
        command.env("BIN2HEX_CHILD_VALUE", value.to_string());
    }
    let output = command.output().expect("run isolated FFI child");
    output.status
}

#[test]
fn ffi_child() {
    let Ok(library_path) = std::env::var("BIN2HEX_CHILD_LIBRARY") else {
        return;
    };
    let case = std::env::var("BIN2HEX_CHILD_CASE").expect("child case");
    let value = std::env::var("BIN2HEX_CHILD_VALUE")
        .ok()
        .map(|value| value.parse::<usize>().expect("numeric child value"));
    invoke_child_case(PathBuf::from(library_path), &case, value);
}

fn invoke_child_case(library_path: PathBuf, case: &str, value: Option<usize>) {
    unsafe {
        let library = Library::new(library_path).expect("load child shared library");
        let bin2hex: Symbol<Bin2Hex> = library.get(b"bin2hex\0").expect("load child bin2hex");
        let dangling_input = std::ptr::NonNull::<u8>::dangling().as_ptr();
        let dangling_output = std::ptr::NonNull::<u8>::dangling().as_ptr();

        match case {
            "oversized" => {
                bin2hex(
                    dangling_output.cast(),
                    usize::MAX,
                    dangling_input,
                    value.expect("oversized length"),
                );
            }
            "insufficient" => {
                let (length, capacity) = unpack_lengths(value.expect("packed length and capacity"));
                bin2hex(dangling_output.cast(), capacity, dangling_input, length);
            }
            "null_output" => {
                bin2hex(std::ptr::null_mut(), 1, dangling_input, 0);
            }
            "null_input" => {
                let mut output = [0_u8; 3];
                bin2hex(
                    output.as_mut_ptr().cast(),
                    output.len(),
                    std::ptr::null(),
                    1,
                );
            }
            _ => panic!("unknown child case {case}"),
        }

        panic!("child FFI call unexpectedly returned");
    }
}
