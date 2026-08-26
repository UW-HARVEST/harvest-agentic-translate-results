use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

type DecodeBase64 = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type FailAllocArm = unsafe extern "C" fn(c_int, usize);
type FailAllocWasFreed = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libdriver.so");
        let rust_path = current_rust_library();
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
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    unsafe fn decoders(&self) -> (Symbol<'_, DecodeBase64>, Symbol<'_, DecodeBase64>) {
        unsafe {
            (
                self.c.get(b"decode_base64\0").expect("C decode_base64"),
                self.rust
                    .get(b"decode_base64\0")
                    .expect("Rust decode_base64"),
            )
        }
    }
}

fn current_rust_library() -> PathBuf {
    std::env::current_exe()
        .expect("current test executable")
        .parent()
        .expect("test executable directory")
        .join("libdriver.so")
}

fn decoded_len(input: &[u8]) -> usize {
    let filtered: Vec<u8> = input
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .filter(|byte| is_base64(*byte))
        .collect();

    filtered
        .chunks(4)
        .map(|chunk| {
            let c3 = chunk.get(2).copied().unwrap_or(b'A');
            let c4 = chunk.get(3).copied().unwrap_or(b'A');
            1 + usize::from(c3 != b'=') + usize::from(c4 != b'=')
        })
        .sum()
}

fn is_base64(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')
}

unsafe fn call_bytes(
    function: &DecodeBase64,
    input: *const c_char,
    length: usize,
) -> Option<Vec<u8>> {
    let output = unsafe { function(input) };
    if output.is_null() {
        return None;
    }

    let bytes = unsafe { std::slice::from_raw_parts(output.cast::<u8>(), length + 1) }.to_vec();
    unsafe { free(output.cast()) };
    Some(bytes)
}

fn compare_input(libraries: &Libraries, input: &[u8]) {
    let length = decoded_len(input);
    let (c_decode, rust_decode) = unsafe { libraries.decoders() };
    let c_output = unsafe { call_bytes(&c_decode, input.as_ptr().cast(), length) };
    let rust_output = unsafe { call_bytes(&rust_decode, input.as_ptr().cast(), length) };
    assert_eq!(c_output, rust_output, "input bytes: {input:?}");
    if let Some(output) = c_output {
        assert_eq!(output.last(), Some(&0), "C output is not NUL terminated");
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next() as usize) % upper
    }
}

const DECODE_BYTES: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
const NO_PADDING_BYTES: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const IGNORED_BYTES: &[u8] = b"!@#$%^&*()[]{} \t\r\n,.;:_-~\x80\xff";

fn retained_input(rng: &mut Rng, length: usize) -> Vec<u8> {
    let mut input: Vec<u8> = (0..length)
        .map(|_| DECODE_BYTES[rng.usize(DECODE_BYTES.len())])
        .collect();
    for group in input.chunks_mut(4) {
        if group.len() > 2 && group[2] == b'=' {
            group[2] = NO_PADDING_BYTES[rng.usize(NO_PADDING_BYTES.len())];
        }
        if group.len() > 3 && group[3] == b'=' {
            group[3] = NO_PADDING_BYTES[rng.usize(NO_PADDING_BYTES.len())];
        }
    }
    input.push(0);
    input
}

fn padding_input(rng: &mut Rng, c3_padding: bool, c4_padding: bool) -> Vec<u8> {
    let groups = 1 + rng.usize(9);
    let mut input = retained_input(rng, groups * 4);
    input.pop();
    let final_group = (groups - 1) * 4;
    input[final_group + 2] = if c3_padding {
        b'='
    } else {
        NO_PADDING_BYTES[rng.usize(NO_PADDING_BYTES.len())]
    };
    input[final_group + 3] = if c4_padding {
        b'='
    } else {
        NO_PADDING_BYTES[rng.usize(NO_PADDING_BYTES.len())]
    };
    input.push(0);
    input
}

fn intersperse_ignored(rng: &mut Rng, input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() * 2 + 2);
    output.push(IGNORED_BYTES[rng.usize(IGNORED_BYTES.len())]);
    for byte in input.iter().copied().take_while(|byte| *byte != 0) {
        if rng.usize(3) != 0 {
            output.push(IGNORED_BYTES[rng.usize(IGNORED_BYTES.len())]);
        }
        output.push(byte);
        if rng.usize(2) == 0 {
            output.push(IGNORED_BYTES[rng.usize(IGNORED_BYTES.len())]);
        }
    }
    output.push(0);
    output
}

#[test]
fn valid_configuration_matrix() {
    let libraries = Libraries::load();
    let mut rng = Rng(0x4d59_5df4_d0f3_3173);

    for residue in 0..4 {
        for _ in 0..128 {
            let groups = if residue == 0 {
                1 + rng.usize(9)
            } else {
                rng.usize(9)
            };
            let input = retained_input(&mut rng, groups * 4 + residue);
            compare_input(&libraries, &input);
        }
    }

    for (c3_padding, c4_padding) in [(true, false), (false, true), (true, true)] {
        for _ in 0..128 {
            let input = padding_input(&mut rng, c3_padding, c4_padding);
            compare_input(&libraries, &input);
        }
    }

    for residue in 0..4 {
        for _ in 0..128 {
            let groups = if residue == 0 {
                1 + rng.usize(9)
            } else {
                rng.usize(9)
            };
            let retained = retained_input(&mut rng, groups * 4 + residue);
            let input = intersperse_ignored(&mut rng, &retained);
            compare_input(&libraries, &input);
        }
    }

    for (c3_padding, c4_padding) in [(true, false), (false, true), (true, true)] {
        for _ in 0..128 {
            let retained = padding_input(&mut rng, c3_padding, c4_padding);
            let input = intersperse_ignored(&mut rng, &retained);
            compare_input(&libraries, &input);
        }
    }

    for _ in 0..128 {
        let length = 1 + rng.usize(128);
        let mut input: Vec<u8> = (0..length)
            .map(|_| IGNORED_BYTES[rng.usize(IGNORED_BYTES.len())])
            .collect();
        input.push(0);
        compare_input(&libraries, &input);
    }

    for _ in 0..128 {
        let prefix_len = 1 + rng.usize(64);
        let mut input = retained_input(&mut rng, prefix_len);
        input.extend((0..64).map(|_| DECODE_BYTES[rng.usize(DECODE_BYTES.len())]));
        compare_input(&libraries, &input);
    }

    for _ in 0..64 {
        let length = 4096 + rng.usize(61_441);
        let input = retained_input(&mut rng, length);
        compare_input(&libraries, &input);
    }
}

#[test]
fn null_and_empty_inputs() {
    let libraries = Libraries::load();
    let (c_decode, rust_decode) = unsafe { libraries.decoders() };

    assert!(unsafe { c_decode(std::ptr::null()) }.is_null());
    assert!(unsafe { rust_decode(std::ptr::null()) }.is_null());

    let empty = CString::new("").unwrap();
    assert!(unsafe { c_decode(empty.as_ptr()) }.is_null());
    assert!(unsafe { rust_decode(empty.as_ptr()) }.is_null());
}

#[test]
fn allocation_failures() {
    const CHILD_ENV: &str = "DRIVER_ALLOC_FAILURE_CHILD";

    if std::env::var_os(CHILD_ENV).is_some() {
        run_allocation_failure_child();
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("tests/support/fail_alloc.c");
    let output = std::env::temp_dir().join(format!(
        "driver-fail-alloc-{}-{}.so",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let status = Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(&source)
        .arg("-ldl")
        .arg("-o")
        .arg(&output)
        .status()
        .expect("compile allocation failure shim");
    assert!(status.success(), "failed to compile {}", source.display());

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "allocation_failures", "--nocapture"])
        .env(CHILD_ENV, "1")
        .env("LD_PRELOAD", &output)
        .status()
        .expect("run allocation failure child");
    let _ = std::fs::remove_file(&output);
    assert!(status.success(), "allocation failure child failed");
}

fn run_allocation_failure_child() {
    let libraries = Libraries::load();
    let shim_path = std::env::var_os("LD_PRELOAD").expect("LD_PRELOAD");
    let shim = unsafe { Library::new(shim_path).expect("open allocation failure shim") };
    let arm: Symbol<'_, FailAllocArm> =
        unsafe { shim.get(b"fail_alloc_arm\0").expect("fail_alloc_arm") };
    let was_freed: Symbol<'_, FailAllocWasFreed> = unsafe {
        shim.get(b"fail_alloc_was_freed\0")
            .expect("fail_alloc_was_freed")
    };
    let input = CString::new("YWJjZGVmZw==").unwrap();
    let source_len = input.as_bytes().len();
    let (c_decode, rust_decode) = unsafe { libraries.decoders() };

    for decode in [&*c_decode, &*rust_decode] {
        unsafe { arm(1, source_len + 14) };
        assert!(unsafe { decode(input.as_ptr()) }.is_null());

        unsafe { arm(2, source_len + 1) };
        assert!(unsafe { decode(input.as_ptr()) }.is_null());
        assert_eq!(unsafe { was_freed() }, 1);
    }
}
