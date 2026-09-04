use libloading::Library;
use std::env;
use std::ffi::{CStr, c_char, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::OnceLock;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;
type FilterFn = unsafe extern "C" fn(*const c_char, bool) -> *mut c_char;
type ArmFailureFn = unsafe extern "C" fn(i32);

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

struct Api {
    _library: Library,
    drop_fn: DropFn,
    filter_fn: FilterFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let drop_fn = unsafe {
            *library
                .get::<DropFn>(b"w_utf8_drop\0")
                .expect("missing w_utf8_drop")
        };
        let filter_fn = unsafe {
            *library
                .get::<FilterFn>(b"w_utf8_filter\0")
                .expect("missing w_utf8_filter")
        };
        Self {
            _library: library,
            drop_fn,
            filter_fn,
        }
    }
}

#[derive(Clone, Copy)]
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

    fn range(&mut self, start: usize, end_exclusive: usize) -> usize {
        assert!(start < end_exclusive);
        start + (self.next_u64() as usize % (end_exclusive - start))
    }

    fn byte(&mut self, start: u8, end_inclusive: u8) -> u8 {
        start + (self.next_u64() % u64::from(end_inclusive - start + 1)) as u8
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("crate must have a parent")
        .join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let executable = env::current_exe().expect("current test executable path");
    executable
        .parent()
        .expect("test executable directory")
        .parent()
        .expect("Cargo profile directory")
        .join("libdriver.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn padded_input(bytes: &[u8]) -> Vec<u8> {
    assert!(
        !bytes.contains(&0),
        "test inputs must not contain interior NUL"
    );
    let mut padded = Vec::with_capacity(bytes.len() + 4);
    padded.extend_from_slice(bytes);
    padded.extend_from_slice(&[0; 4]);
    padded
}

fn drop_offset(api: &Api, bytes: &[u8]) -> usize {
    let input = padded_input(bytes);
    let start = input.as_ptr().cast::<c_char>();
    let result = unsafe { (api.drop_fn)(start) };
    assert!(!result.is_null(), "w_utf8_drop returned NULL");
    unsafe { result.offset_from(start) as usize }
}

fn filter_output(api: &Api, bytes: &[u8], replacement: bool) -> Vec<u8> {
    let input = padded_input(bytes);
    let result = unsafe { (api.filter_fn)(input.as_ptr().cast::<c_char>(), replacement) };
    assert!(!result.is_null(), "w_utf8_filter returned NULL");
    let output = unsafe { CStr::from_ptr(result).to_bytes_with_nul().to_vec() };
    unsafe { free(result.cast::<c_void>()) };
    output
}

fn verify_drop_generated<F>(
    row: usize,
    count: usize,
    rng: &mut Rng,
    c_api: &Api,
    rust_api: &Api,
    mut make_input: F,
) where
    F: FnMut(&mut Rng) -> Vec<u8>,
{
    for sample in 0..count {
        let input = make_input(rng);
        let c_offset = drop_offset(c_api, &input);
        let rust_offset = drop_offset(rust_api, &input);
        assert_eq!(
            rust_offset, c_offset,
            "CONFIGS.md row {row}, sample {sample}, input {input:02x?}"
        );
    }
}

fn verify_filter_generated<F>(
    row: usize,
    count: usize,
    replacement: bool,
    rng: &mut Rng,
    c_api: &Api,
    rust_api: &Api,
    mut make_input: F,
) where
    F: FnMut(&mut Rng) -> Vec<u8>,
{
    for sample in 0..count {
        let input = make_input(rng);
        let c_output = filter_output(c_api, &input, replacement);
        let rust_output = filter_output(rust_api, &input, replacement);
        assert_eq!(
            rust_output, c_output,
            "CONFIGS.md row {row}, replacement={replacement}, sample {sample}, input {input:02x?}"
        );
    }
}

fn ascii_byte(rng: &mut Rng) -> u8 {
    rng.byte(1, 0x7f)
}

fn continuation(rng: &mut Rng) -> u8 {
    rng.byte(0x80, 0xbf)
}

fn valid_2(rng: &mut Rng) -> Vec<u8> {
    vec![rng.byte(0xc2, 0xdf), continuation(rng)]
}

fn valid_3_e0(rng: &mut Rng) -> Vec<u8> {
    vec![0xe0, rng.byte(0xa0, 0xbf), continuation(rng)]
}

fn valid_3_ed(rng: &mut Rng) -> Vec<u8> {
    vec![0xed, rng.byte(0x80, 0x9f), continuation(rng)]
}

fn valid_3_generic(rng: &mut Rng) -> Vec<u8> {
    let lead = if rng.next_u64() & 1 == 0 {
        rng.byte(0xe1, 0xec)
    } else {
        rng.byte(0xee, 0xef)
    };
    vec![lead, continuation(rng), continuation(rng)]
}

fn valid_4_f0(rng: &mut Rng) -> Vec<u8> {
    vec![
        0xf0,
        rng.byte(0x90, 0xbf),
        continuation(rng),
        continuation(rng),
    ]
}

fn valid_4_f4(rng: &mut Rng) -> Vec<u8> {
    vec![
        0xf4,
        rng.byte(0x80, 0x8f),
        continuation(rng),
        continuation(rng),
    ]
}

fn valid_4_generic(rng: &mut Rng) -> Vec<u8> {
    vec![
        rng.byte(0xf1, 0xf3),
        continuation(rng),
        continuation(rng),
        continuation(rng),
    ]
}

fn random_valid_token(rng: &mut Rng) -> Vec<u8> {
    match rng.range(0, 8) {
        0 => vec![ascii_byte(rng)],
        1 => valid_2(rng),
        2 => valid_3_e0(rng),
        3 => valid_3_ed(rng),
        4 => valid_3_generic(rng),
        5 => valid_4_f0(rng),
        6 => valid_4_f4(rng),
        _ => valid_4_generic(rng),
    }
}

fn mixed_valid(rng: &mut Rng, minimum_tokens: usize) -> Vec<u8> {
    let token_count = rng.range(minimum_tokens, minimum_tokens + 24);
    let mut bytes = Vec::new();
    for _ in 0..token_count {
        bytes.extend(random_valid_token(rng));
    }
    bytes
}

fn bad_noncontinuation(rng: &mut Rng) -> u8 {
    if rng.next_u64() & 1 == 0 {
        ascii_byte(rng)
    } else {
        rng.byte(0xc0, 0xff)
    }
}

fn malformed_3(rng: &mut Rng) -> Vec<u8> {
    if rng.next_u64() & 1 == 0 {
        vec![rng.byte(0xe1, 0xec), bad_noncontinuation(rng), continuation(rng)]
    } else {
        vec![rng.byte(0xe1, 0xec), continuation(rng), bad_noncontinuation(rng)]
    }
}

fn malformed_4(rng: &mut Rng) -> Vec<u8> {
    let mut bytes = vec![
        rng.byte(0xf1, 0xf3),
        continuation(rng),
        continuation(rng),
        continuation(rng),
    ];
    let malformed_index = rng.range(1, 4);
    bytes[malformed_index] = bad_noncontinuation(rng);
    bytes
}

fn malformed_class(rng: &mut Rng) -> Vec<u8> {
    match rng.range(0, 10) {
        0 => vec![continuation(rng)],
        1 => vec![rng.byte(0xc0, 0xc1), continuation(rng)],
        2 => vec![rng.byte(0xc2, 0xdf), bad_noncontinuation(rng)],
        3 => malformed_3(rng),
        4 => vec![0xe0, rng.byte(0x80, 0x9f), continuation(rng)],
        5 => vec![0xed, rng.byte(0xa0, 0xbf), continuation(rng)],
        6 => malformed_4(rng),
        7 => vec![
            0xf0,
            rng.byte(0x80, 0x8f),
            continuation(rng),
            continuation(rng),
        ],
        8 => vec![
            0xf4,
            rng.byte(0x90, 0xbf),
            continuation(rng),
            continuation(rng),
        ],
        _ => vec![rng.byte(0xf5, 0xff)],
    }
}

fn random_mixed_invalid(rng: &mut Rng) -> Vec<u8> {
    let count = rng.range(2, 50);
    let mut bytes = Vec::new();
    bytes.extend(random_valid_token(rng));
    bytes.push(continuation(rng));
    for _ in 2..count {
        if rng.range(0, 4) == 0 {
            bytes.push(continuation(rng));
        } else {
            bytes.extend(random_valid_token(rng));
        }
    }
    bytes
}

#[test]
fn configs_drop_rows_1_to_22() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x49f6_0c7d_abc1_2501);

    verify_drop_generated(1, 1, &mut rng, &c_api, &rust_api, |_| vec![]);
    verify_drop_generated(2, 128, &mut rng, &c_api, &rust_api, |rng| {
        vec![ascii_byte(rng)]
    });
    verify_drop_generated(3, 128, &mut rng, &c_api, &rust_api, |rng| {
        let length = rng.range(2, 257);
        (0..length).map(|_| ascii_byte(rng)).collect()
    });
    verify_drop_generated(4, 128, &mut rng, &c_api, &rust_api, valid_2);
    verify_drop_generated(5, 128, &mut rng, &c_api, &rust_api, valid_3_e0);
    verify_drop_generated(6, 128, &mut rng, &c_api, &rust_api, valid_3_ed);
    verify_drop_generated(7, 128, &mut rng, &c_api, &rust_api, valid_3_generic);
    verify_drop_generated(8, 128, &mut rng, &c_api, &rust_api, valid_4_f0);
    verify_drop_generated(9, 128, &mut rng, &c_api, &rust_api, valid_4_f4);
    verify_drop_generated(10, 128, &mut rng, &c_api, &rust_api, valid_4_generic);
    verify_drop_generated(11, 128, &mut rng, &c_api, &rust_api, |rng| {
        mixed_valid(rng, 4)
    });
    verify_drop_generated(12, 128, &mut rng, &c_api, &rust_api, |rng| {
        vec![continuation(rng)]
    });
    verify_drop_generated(13, 128, &mut rng, &c_api, &rust_api, |rng| {
        vec![rng.byte(0xc0, 0xc1), continuation(rng)]
    });
    verify_drop_generated(14, 128, &mut rng, &c_api, &rust_api, |rng| {
        vec![rng.byte(0xc2, 0xdf), bad_noncontinuation(rng)]
    });
    verify_drop_generated(15, 128, &mut rng, &c_api, &rust_api, malformed_3);
    verify_drop_generated(16, 128, &mut rng, &c_api, &rust_api, |rng| {
        vec![0xe0, rng.byte(0x80, 0x9f), continuation(rng)]
    });
    verify_drop_generated(17, 128, &mut rng, &c_api, &rust_api, |rng| {
        vec![0xed, rng.byte(0xa0, 0xbf), continuation(rng)]
    });
    verify_drop_generated(18, 128, &mut rng, &c_api, &rust_api, malformed_4);
    verify_drop_generated(19, 128, &mut rng, &c_api, &rust_api, |rng| {
        vec![
            0xf0,
            rng.byte(0x80, 0x8f),
            continuation(rng),
            continuation(rng),
        ]
    });
    verify_drop_generated(20, 128, &mut rng, &c_api, &rust_api, |rng| {
        vec![
            0xf4,
            rng.byte(0x90, 0xbf),
            continuation(rng),
            continuation(rng),
        ]
    });
    verify_drop_generated(21, 128, &mut rng, &c_api, &rust_api, |rng| {
        vec![rng.byte(0xf5, 0xff)]
    });
    verify_drop_generated(22, 128, &mut rng, &c_api, &rust_api, |rng| {
        let mut bytes = mixed_valid(rng, 1);
        bytes.extend(malformed_class(rng));
        bytes
    });
}

#[test]
fn configs_filter_valid_rows_23_to_28() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x8642_468a_dda0_0017);

    for replacement in [false, true] {
        verify_filter_generated(23, 1, replacement, &mut rng, &c_api, &rust_api, |_| {
            vec![]
        });
        verify_filter_generated(
            24,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| {
                let length = rng.range(1, 257);
                (0..length).map(|_| ascii_byte(rng)).collect()
            },
        );
        verify_filter_generated(
            25,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| {
                let count = rng.range(1, 33);
                let mut bytes = Vec::new();
                for _ in 0..count {
                    bytes.extend(valid_2(rng));
                }
                bytes
            },
        );
        verify_filter_generated(
            26,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| {
                let count = rng.range(1, 33);
                let mut bytes = Vec::new();
                for _ in 0..count {
                    bytes.extend(match rng.range(0, 3) {
                        0 => valid_3_e0(rng),
                        1 => valid_3_ed(rng),
                        _ => valid_3_generic(rng),
                    });
                }
                bytes
            },
        );
        verify_filter_generated(
            27,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| {
                let count = rng.range(1, 33);
                let mut bytes = Vec::new();
                for _ in 0..count {
                    bytes.extend(match rng.range(0, 3) {
                        0 => valid_4_f0(rng),
                        1 => valid_4_f4(rng),
                        _ => valid_4_generic(rng),
                    });
                }
                bytes
            },
        );
        verify_filter_generated(
            28,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| mixed_valid(rng, 4),
        );
    }
}

#[test]
fn configs_filter_structured_rows_29_to_44() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x2cb4_9327_1234_fedc);

    verify_filter_generated(29, 128, false, &mut rng, &c_api, &rust_api, |rng| {
        vec![continuation(rng)]
    });
    verify_filter_generated(30, 128, true, &mut rng, &c_api, &rust_api, |rng| {
        vec![continuation(rng)]
    });
    verify_filter_generated(31, 128, false, &mut rng, &c_api, &rust_api, |rng| {
        let mut bytes = mixed_valid(rng, 1);
        bytes.push(continuation(rng));
        bytes
    });
    verify_filter_generated(32, 128, true, &mut rng, &c_api, &rust_api, |rng| {
        let mut bytes = mixed_valid(rng, 1);
        bytes.push(continuation(rng));
        bytes
    });

    for replacement in [false, true] {
        verify_filter_generated(
            33,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| vec![continuation(rng), ascii_byte(rng)],
        );
        verify_filter_generated(
            34,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| {
                let mut bytes = vec![continuation(rng)];
                bytes.extend(valid_2(rng));
                bytes
            },
        );
        verify_filter_generated(
            35,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| {
                let mut bytes = vec![continuation(rng)];
                bytes.extend(match rng.range(0, 3) {
                    0 => valid_3_e0(rng),
                    1 => valid_3_ed(rng),
                    _ => valid_3_generic(rng),
                });
                bytes
            },
        );
        verify_filter_generated(
            36,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| {
                let mut bytes = vec![continuation(rng)];
                bytes.extend(match rng.range(0, 3) {
                    0 => valid_4_f0(rng),
                    1 => valid_4_f4(rng),
                    _ => valid_4_generic(rng),
                });
                bytes
            },
        );
        verify_filter_generated(
            37,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| {
                if rng.next_u64() & 1 == 0 {
                    vec![rng.byte(0xc0, 0xc1), continuation(rng)]
                } else {
                    vec![rng.byte(0xc2, 0xdf), bad_noncontinuation(rng)]
                }
            },
        );
        verify_filter_generated(
            38,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            malformed_3,
        );
        verify_filter_generated(
            39,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| {
                if rng.next_u64() & 1 == 0 {
                    vec![0xe0, rng.byte(0x80, 0x9f), continuation(rng)]
                } else {
                    vec![0xed, rng.byte(0xa0, 0xbf), continuation(rng)]
                }
            },
        );
        verify_filter_generated(
            40,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            malformed_4,
        );
        verify_filter_generated(
            41,
            128,
            replacement,
            &mut rng,
            &c_api,
            &rust_api,
            |rng| match rng.range(0, 3) {
                0 => vec![
                    0xf0,
                    rng.byte(0x80, 0x8f),
                    continuation(rng),
                    continuation(rng),
                ],
                1 => vec![
                    0xf4,
                    rng.byte(0x90, 0xbf),
                    continuation(rng),
                    continuation(rng),
                ],
                _ => vec![rng.byte(0xf5, 0xff)],
            },
        );
    }

    verify_filter_generated(42, 64, false, &mut rng, &c_api, &rust_api, |rng| {
        vec![0x80; rng.range(2, 2049)]
    });
    verify_filter_generated(43, 64, true, &mut rng, &c_api, &rust_api, |rng| {
        vec![0x80; rng.range(1, 1366)]
    });
    verify_filter_generated(44, 24, true, &mut rng, &c_api, &rust_api, |rng| {
        vec![0x80; rng.range(1366, 1700)]
    });
}

#[test]
fn configs_filter_random_rows_45_to_46() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0xd00d_f00d_9876_5432);

    verify_filter_generated(
        45,
        512,
        false,
        &mut rng,
        &c_api,
        &rust_api,
        random_mixed_invalid,
    );
    verify_filter_generated(
        46,
        512,
        true,
        &mut rng,
        &c_api,
        &rust_api,
        random_mixed_invalid,
    );
}

fn child_status(test_name: &str, variables: &[(&str, &str)]) -> ExitStatus {
    let mut command = Command::new(env::current_exe().expect("current test executable"));
    command.args(["--exact", test_name, "--nocapture"]);
    for (name, value) in variables {
        command.env(name, value);
    }
    command.status().expect("run child test")
}

#[test]
fn errors_null_rows_1_and_2() {
    for symbol in ["drop", "filter"] {
        let c_status = child_status(
            "null_pointer_child",
            &[("DIFF_NULL_CHILD", symbol), ("DIFF_LIBRARY", "c")],
        );
        let rust_status = child_status(
            "null_pointer_child",
            &[("DIFF_NULL_CHILD", symbol), ("DIFF_LIBRARY", "rust")],
        );

        #[cfg(unix)]
        {
            assert_eq!(
                c_status.signal(),
                Some(6),
                "C {symbol} null call did not terminate with SIGABRT: {c_status}"
            );
            assert_eq!(
                rust_status.signal(),
                c_status.signal(),
                "Rust {symbol} null behavior differs from C"
            );
        }
    }
}

#[test]
fn null_pointer_child() {
    let Ok(symbol) = env::var("DIFF_NULL_CHILD") else {
        return;
    };
    let library_kind = env::var("DIFF_LIBRARY").expect("DIFF_LIBRARY");
    let path = if library_kind == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&path) };

    unsafe {
        match symbol.as_str() {
            "drop" => {
                (api.drop_fn)(std::ptr::null());
            }
            "filter" => {
                (api.filter_fn)(std::ptr::null(), false);
            }
            _ => panic!("unknown null child symbol"),
        }
    }
    panic!("null pointer call unexpectedly returned");
}

fn allocator_shim_path() -> &'static Path {
    static SHIM: OnceLock<PathBuf> = OnceLock::new();
    SHIM.get_or_init(|| {
        let output_dir = manifest_dir().join("target/differential-tests");
        fs::create_dir_all(&output_dir).expect("create allocator shim output directory");
        let output = output_dir.join("libfail_alloc.so");
        let source = manifest_dir().join("tests/fail_alloc.c");
        let status = Command::new("cc")
            .args(["-shared", "-fPIC", "-std=c11", "-O2"])
            .arg(&source)
            .arg("-o")
            .arg(&output)
            .status()
            .expect("compile allocator failure shim");
        assert!(status.success(), "allocator failure shim compilation failed");
        output
    })
}

#[test]
fn errors_allocator_rows_3_to_5() {
    let shim = allocator_shim_path();
    let shim_text = shim.to_str().expect("UTF-8 shim path");

    for mode in ["strdup", "malloc", "realloc"] {
        for library in ["c", "rust"] {
            let status = child_status(
                "allocator_failure_child",
                &[
                    ("DIFF_ALLOC_CHILD", mode),
                    ("DIFF_LIBRARY", library),
                    ("FAIL_ALLOC_SHIM", shim_text),
                    ("LD_PRELOAD", shim_text),
                ],
            );
            assert!(
                status.success(),
                "{library} allocator-failure child failed for {mode}: {status}"
            );
        }
    }
}

#[test]
fn allocator_failure_child() {
    let Ok(mode) = env::var("DIFF_ALLOC_CHILD") else {
        return;
    };
    let library_kind = env::var("DIFF_LIBRARY").expect("DIFF_LIBRARY");
    let target_path = if library_kind == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let shim_path = PathBuf::from(env::var("FAIL_ALLOC_SHIM").expect("FAIL_ALLOC_SHIM"));

    let shim = unsafe { Library::new(&shim_path) }.expect("load allocator failure shim");
    let arm = unsafe {
        *shim
            .get::<ArmFailureFn>(b"fail_alloc_arm\0")
            .expect("load fail_alloc_arm")
    };
    let api = unsafe { Api::load(&target_path) };

    let (input, replacement, failure_kind) = match mode.as_str() {
        "strdup" => (b"valid".as_slice(), false, 1),
        "malloc" => (b"\x80".as_slice(), false, 2),
        "realloc" => (b"\x80".as_slice(), true, 3),
        _ => panic!("unknown allocation failure mode"),
    };

    let _warmup = filter_output(&api, input, replacement);

    let padded = padded_input(input);
    unsafe { arm(failure_kind) };
    let result =
        unsafe { (api.filter_fn)(padded.as_ptr().cast::<c_char>(), replacement) };
    assert!(
        result.is_null(),
        "{library_kind} {mode} failure did not return NULL"
    );
}
