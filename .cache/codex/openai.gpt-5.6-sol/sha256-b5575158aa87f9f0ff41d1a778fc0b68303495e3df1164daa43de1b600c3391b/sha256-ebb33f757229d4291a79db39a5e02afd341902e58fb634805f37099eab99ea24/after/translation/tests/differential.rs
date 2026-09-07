use libloading::Library;
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::process::Command;

type ToolBasename = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

struct LoadedApi {
    _library: Library,
    tool_basename: ToolBasename,
}

impl LoadedApi {
    fn open(path: &Path) -> Self {
        assert!(
            path.is_file(),
            "shared library does not exist: {}",
            path.display()
        );
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let tool_basename = unsafe {
            *library
                .get::<ToolBasename>(b"tool_basename\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load tool_basename from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            tool_basename,
        }
    }
}

struct Differential {
    c: LoadedApi,
    rust: LoadedApi,
}

impl Differential {
    fn load() -> Self {
        Self {
            c: LoadedApi::open(&c_library_path()),
            rust: LoadedApi::open(&rust_library_path()),
        }
    }

    fn compare(&self, case: &str, input: &[u8]) {
        assert!(
            input.contains(&0),
            "{case}: backing allocation must contain a C-string terminator"
        );

        let mut c_input = input.to_vec();
        let mut rust_input = input.to_vec();
        let original = input.to_vec();

        let c_base = c_input.as_mut_ptr();
        let rust_base = rust_input.as_mut_ptr();
        let c_result = unsafe { (self.c.tool_basename)(c_base.cast::<c_char>()) }.cast::<u8>();
        let rust_result =
            unsafe { (self.rust.tool_basename)(rust_base.cast::<c_char>()) }.cast::<u8>();

        let c_offset = checked_offset(case, "C", c_base, c_input.len(), c_result);
        let rust_offset = checked_offset(case, "Rust", rust_base, rust_input.len(), rust_result);

        assert_eq!(
            c_input, original,
            "{case}: C unexpectedly modified the input allocation"
        );
        assert_eq!(
            rust_input, original,
            "{case}: Rust unexpectedly modified the input allocation"
        );
        assert_eq!(
            rust_offset, c_offset,
            "{case}: returned pointer offset differs"
        );
        assert_eq!(
            c_string_suffix(&c_input, c_offset),
            c_string_suffix(&rust_input, rust_offset),
            "{case}: returned basename bytes differ"
        );
    }
}

fn checked_offset(
    case: &str,
    implementation: &str,
    base: *const u8,
    len: usize,
    result: *const u8,
) -> usize {
    let base = base as usize;
    let result = result as usize;
    assert!(
        (base..base + len).contains(&result),
        "{case}: {implementation} returned a pointer outside its input allocation"
    );
    result - base
}

fn c_string_suffix(bytes: &[u8], offset: usize) -> &[u8] {
    let terminator = bytes[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .expect("returned pointer has no in-allocation C-string terminator");
    &bytes[offset..=offset + terminator]
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_root()
        .parent()
        .expect("translation crate must have a parent")
        .join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    crate_root().join("target/release/libdriver.so")
}

#[derive(Clone)]
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        assert!(upper_exclusive > 0);
        (self.next_u64() as usize) % upper_exclusive
    }

    fn bytes(&mut self, min_len: usize, max_len: usize, alphabet: &[u8]) -> Vec<u8> {
        assert!(min_len <= max_len);
        let len = min_len + self.usize(max_len - min_len + 1);
        (0..len)
            .map(|_| alphabet[self.usize(alphabet.len())])
            .collect()
    }
}

const PLAIN: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-+ =@#$%^&()[]{}";
const WITH_SEPARATORS: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-/\\";

fn terminate(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(0);
    bytes
}

fn one_separator_kind(rng: &mut Lcg, separator: u8) -> Vec<u8> {
    let separator_count = 1 + rng.usize(7);
    let mut bytes = rng.bytes(0, 24, PLAIN);
    for index in 0..separator_count {
        bytes.push(separator);
        bytes.extend(rng.bytes(usize::from(index + 1 == separator_count), 24, PLAIN));
    }
    terminate(bytes)
}

fn both_kinds_with_later(rng: &mut Lcg, later: u8) -> Vec<u8> {
    let earlier = if later == b'/' { b'\\' } else { b'/' };
    let mut bytes = rng.bytes(0, 16, PLAIN);

    for _ in 0..rng.usize(6) {
        bytes.push(if rng.usize(2) == 0 { b'/' } else { b'\\' });
        bytes.extend(rng.bytes(0, 12, PLAIN));
    }

    bytes.push(earlier);
    bytes.extend(rng.bytes(0, 16, PLAIN));
    bytes.push(later);
    bytes.extend(rng.bytes(1, 24, PLAIN));
    terminate(bytes)
}

#[test]
fn config_01_empty_c_string() {
    let differential = Differential::load();
    let mut rng = Lcg::new(0x01_5eed_f00d_cafe);

    for case_index in 0..256 {
        let mut backing = vec![0];
        backing.extend(rng.bytes(0, 128, PLAIN));
        differential.compare(&format!("config 1 case {case_index}"), &backing);
    }
}

#[test]
fn config_02_no_separators() {
    let differential = Differential::load();
    let mut rng = Lcg::new(0x02_5eed_f00d_cafe);

    for case_index in 0..512 {
        let input = terminate(rng.bytes(1, 256, PLAIN));
        differential.compare(&format!("config 2 case {case_index}"), &input);
    }
}

#[test]
fn config_03_only_forward_slashes() {
    let differential = Differential::load();
    let mut rng = Lcg::new(0x03_5eed_f00d_cafe);

    for case_index in 0..512 {
        let input = one_separator_kind(&mut rng, b'/');
        differential.compare(&format!("config 3 case {case_index}"), &input);
    }
}

#[test]
fn config_04_only_backslashes() {
    let differential = Differential::load();
    let mut rng = Lcg::new(0x04_5eed_f00d_cafe);

    for case_index in 0..512 {
        let input = one_separator_kind(&mut rng, b'\\');
        differential.compare(&format!("config 4 case {case_index}"), &input);
    }
}

#[test]
fn config_05_both_with_forward_slash_later() {
    let differential = Differential::load();
    let mut rng = Lcg::new(0x05_5eed_f00d_cafe);

    for case_index in 0..512 {
        let input = both_kinds_with_later(&mut rng, b'/');
        differential.compare(&format!("config 5 case {case_index}"), &input);
    }
}

#[test]
fn config_06_both_with_backslash_later() {
    let differential = Differential::load();
    let mut rng = Lcg::new(0x06_5eed_f00d_cafe);

    for case_index in 0..512 {
        let input = both_kinds_with_later(&mut rng, b'\\');
        differential.compare(&format!("config 6 case {case_index}"), &input);
    }
}

#[test]
fn config_07_trailing_forward_slash() {
    let differential = Differential::load();
    let mut rng = Lcg::new(0x07_5eed_f00d_cafe);

    for case_index in 0..512 {
        let mut input = rng.bytes(0, 256, WITH_SEPARATORS);
        input.push(b'/');
        let input = terminate(input);
        differential.compare(&format!("config 7 case {case_index}"), &input);
    }
}

#[test]
fn config_08_trailing_backslash() {
    let differential = Differential::load();
    let mut rng = Lcg::new(0x08_5eed_f00d_cafe);

    for case_index in 0..512 {
        let mut input = rng.bytes(0, 256, WITH_SEPARATORS);
        input.push(b'\\');
        let input = terminate(input);
        differential.compare(&format!("config 8 case {case_index}"), &input);
    }
}

#[test]
fn null_pointer_child() {
    let Some(library_path) = std::env::var_os("TOOL_BASENAME_NULL_LIBRARY") else {
        return;
    };
    let api = LoadedApi::open(Path::new(&library_path));
    unsafe {
        (api.tool_basename)(std::ptr::null_mut());
    }
    panic!("tool_basename unexpectedly returned after receiving a null pointer");
}

#[cfg(unix)]
#[test]
fn error_boundary_null_pointer_behavior_matches() {
    use std::os::unix::process::ExitStatusExt;

    fn child_status(library_path: &Path) -> std::process::ExitStatus {
        Command::new(std::env::current_exe().expect("current test executable"))
            .arg("--exact")
            .arg("null_pointer_child")
            .arg("--nocapture")
            .env("TOOL_BASENAME_NULL_LIBRARY", library_path)
            .status()
            .unwrap_or_else(|error| {
                panic!(
                    "failed to run null-pointer child for {}: {error}",
                    library_path.display()
                )
            })
    }

    let c_status = child_status(&c_library_path());
    let rust_status = child_status(&rust_library_path());

    assert!(
        !c_status.success(),
        "C unexpectedly accepted a null pointer"
    );
    assert!(
        !rust_status.success(),
        "Rust unexpectedly accepted a null pointer"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "C and Rust terminated with different signals for a null pointer"
    );
    assert_eq!(
        rust_status.code(),
        c_status.code(),
        "C and Rust returned different exit codes for a null pointer"
    );
}
