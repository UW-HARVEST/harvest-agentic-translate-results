use libloading::Library;
use std::env;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::OnceLock;

type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;
type FilterFn = unsafe extern "C" fn(*const c_char, bool) -> *mut c_char;

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

struct Api {
    _library: Library,
    drop_utf8: DropFn,
    filter_utf8: FilterFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let drop_utf8 = unsafe { *library.get::<DropFn>(b"w_utf8_drop\0").unwrap() };
        let filter_utf8 = unsafe { *library.get::<FilterFn>(b"w_utf8_filter\0").unwrap() };
        Self {
            _library: library,
            drop_utf8,
            filter_utf8,
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }
}

fn c_library_path() -> PathBuf {
    env::var_os("C_DRIVER_SO").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("c_src")
                .join("build")
                .join("libdriver.so")
        },
        PathBuf::from,
    )
}

fn rust_library_path() -> PathBuf {
    env::var_os("RUST_DRIVER_SO").map_or_else(
        || {
            env::current_exe()
                .expect("test executable path")
                .parent()
                .expect("deps directory")
                .join("libdriver.so")
        },
        PathBuf::from,
    )
}

fn c_string_storage(bytes: &[u8]) -> Vec<u8> {
    assert!(!bytes.contains(&0), "test input contains an embedded NUL");
    let mut storage = Vec::with_capacity(bytes.len() + 4);
    storage.extend_from_slice(bytes);
    storage.extend_from_slice(&[0; 4]);
    storage
}

fn drop_offset(api: &Api, bytes: &[u8]) -> usize {
    let storage = c_string_storage(bytes);
    let start = storage.as_ptr().cast::<c_char>();
    let result = unsafe { (api.drop_utf8)(start) };
    unsafe { result.offset_from(start) as usize }
}

fn filtered(api: &Api, bytes: &[u8], replacement: bool) -> Vec<u8> {
    let storage = c_string_storage(bytes);
    let result = unsafe { (api.filter_utf8)(storage.as_ptr().cast(), replacement) };
    assert!(!result.is_null(), "filter unexpectedly returned NULL");
    let output = unsafe { CStr::from_ptr(result).to_bytes().to_vec() };
    unsafe { free(result.cast()) };
    output
}

fn compare_drop(libraries: &Libraries, bytes: &[u8], expected: usize) {
    let c = drop_offset(&libraries.c, bytes);
    let rust = drop_offset(&libraries.rust, bytes);
    assert_eq!(c, expected, "C offset for input {bytes:02x?}");
    assert_eq!(rust, c, "Rust offset for input {bytes:02x?}");
}

fn compare_filter(libraries: &Libraries, bytes: &[u8], replacement: bool) -> Vec<u8> {
    let c = filtered(&libraries.c, bytes, replacement);
    let rust = filtered(&libraries.rust, bytes, replacement);
    assert_eq!(rust, c, "input {bytes:02x?}, replacement={replacement}");
    c
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

    fn usize(&mut self, start: usize, end_inclusive: usize) -> usize {
        start + self.next_u64() as usize % (end_inclusive - start + 1)
    }

    fn byte(&mut self, start: u8, end_inclusive: u8) -> u8 {
        self.usize(start as usize, end_inclusive as usize) as u8
    }
}

fn ascii(rng: &mut Rng) -> Vec<u8> {
    vec![rng.byte(1, 0x7f)]
}

fn two_byte(rng: &mut Rng) -> Vec<u8> {
    vec![rng.byte(0xc2, 0xdf), rng.byte(0x80, 0xbf)]
}

fn ordinary_three_byte(rng: &mut Rng) -> Vec<u8> {
    let starts = [
        0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xee,
    ];
    vec![
        starts[rng.usize(0, starts.len() - 1)],
        rng.byte(0x80, 0xbf),
        rng.byte(0x80, 0xbf),
    ]
}

fn ordinary_four_byte(rng: &mut Rng) -> Vec<u8> {
    vec![
        rng.byte(0xf1, 0xf3),
        rng.byte(0x80, 0xbf),
        rng.byte(0x80, 0xbf),
        rng.byte(0x80, 0xbf),
    ]
}

fn mixed_valid(rng: &mut Rng, count: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    for _ in 0..count {
        let sequence = match rng.usize(0, 9) {
            0 => ascii(rng),
            1 => two_byte(rng),
            2 => ordinary_three_byte(rng),
            3 => vec![0xe0, rng.byte(0xa0, 0xbf), rng.byte(0x80, 0xbf)],
            4 => vec![0xed, rng.byte(0x80, 0x9f), rng.byte(0x80, 0xbf)],
            5 => vec![0xef, rng.byte(0x80, 0xbf), rng.byte(0x80, 0xbf)],
            6 => ordinary_four_byte(rng),
            7 => vec![
                0xf0,
                rng.byte(0x90, 0xbf),
                rng.byte(0x80, 0xbf),
                rng.byte(0x80, 0xbf),
            ],
            _ => vec![
                0xf4,
                rng.byte(0x80, 0x8f),
                rng.byte(0x80, 0xbf),
                rng.byte(0x80, 0xbf),
            ],
        };
        bytes.extend(sequence);
    }
    bytes
}

fn exercise_valid_class(seed: u64, generator: fn(&mut Rng) -> Vec<u8>) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    for _ in 0..128 {
        let mut input = Vec::new();
        for _ in 0..rng.usize(1, 12) {
            input.extend(generator(&mut rng));
        }
        compare_drop(&libraries, &input, input.len());
    }
}

#[test]
fn c01_drop_empty() {
    compare_drop(&Libraries::load(), &[], 0);
}

#[test]
fn c02_drop_ascii() {
    exercise_valid_class(0x02a5_5eed, ascii);
    compare_drop(&Libraries::load(), &[0x7f], 1);
}

#[test]
fn c03_drop_two_byte() {
    exercise_valid_class(0x03a5_5eed, two_byte);
    let libraries = Libraries::load();
    compare_drop(&libraries, &[0xc2, 0x80], 2);
    compare_drop(&libraries, &[0xdf, 0xbf], 2);
}

#[test]
fn c04_drop_ordinary_three_byte() {
    exercise_valid_class(0x04a5_5eed, ordinary_three_byte);
}

#[test]
fn c05_drop_e0_boundaries() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x05a5_5eed);
    for index in 0..128 {
        let second = if index % 2 == 0 { 0xa0 } else { 0xbf };
        let input = [0xe0, second, rng.byte(0x80, 0xbf)];
        compare_drop(&libraries, &input, 3);
    }
}

#[test]
fn c06_drop_ed_boundaries() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x06a5_5eed);
    for index in 0..128 {
        let second = if index % 2 == 0 { 0x80 } else { 0x9f };
        let input = [0xed, second, rng.byte(0x80, 0xbf)];
        compare_drop(&libraries, &input, 3);
    }
}

#[test]
fn c07_drop_ef_branch() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x07a5_5eed);
    for _ in 0..128 {
        let input = [0xef, rng.byte(0x80, 0xbf), rng.byte(0x80, 0xbf)];
        compare_drop(&libraries, &input, 3);
    }
}

#[test]
fn c08_drop_ordinary_four_byte() {
    exercise_valid_class(0x08a5_5eed, ordinary_four_byte);
}

#[test]
fn c09_drop_f0_boundaries() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x09a5_5eed);
    for index in 0..128 {
        let input = [
            0xf0,
            if index % 2 == 0 { 0x90 } else { 0xbf },
            rng.byte(0x80, 0xbf),
            rng.byte(0x80, 0xbf),
        ];
        compare_drop(&libraries, &input, 4);
    }
}

#[test]
fn c10_drop_f4_boundaries() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x10a5_5eed);
    for index in 0..128 {
        let input = [
            0xf4,
            if index % 2 == 0 { 0x80 } else { 0x8f },
            rng.byte(0x80, 0xbf),
            rng.byte(0x80, 0xbf),
        ];
        compare_drop(&libraries, &input, 4);
    }
}

#[test]
fn c11_drop_invalid_leads() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x11a5_5eed);
    for _ in 0..256 {
        let lead = match rng.usize(0, 3) {
            0 => rng.byte(0x80, 0xbf),
            1 => rng.byte(0xc0, 0xc1),
            2 => rng.byte(0xf5, 0xf7),
            _ => rng.byte(0xf8, 0xff),
        };
        compare_drop(&libraries, &[lead], 0);
    }
}

#[test]
fn c12_drop_invalid_after_valid_prefix() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x12a5_5eed);
    for _ in 0..128 {
        let prefix_count = rng.usize(1, 12);
        let mut input = mixed_valid(&mut rng, prefix_count);
        let valid_length = input.len();
        input.push([0x80, 0xc0, 0xf5, 0xff][rng.usize(0, 3)]);
        let suffix_count = rng.usize(1, 4);
        input.extend(mixed_valid(&mut rng, suffix_count));
        compare_drop(&libraries, &input, valid_length);
    }
}

#[test]
fn c13_drop_malformed_sequences() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x13a5_5eed);
    for index in 0..256 {
        let input = match index % 7 {
            0 => vec![rng.byte(0xc2, 0xdf), rng.byte(1, 0x7f)],
            1 => vec![
                rng.byte(0xe1, 0xef),
                rng.byte(0x80, 0xbf),
                rng.byte(1, 0x7f),
            ],
            2 => vec![
                rng.byte(0xf1, 0xf3),
                rng.byte(0x80, 0xbf),
                rng.byte(0x80, 0xbf),
                rng.byte(1, 0x7f),
            ],
            3 => vec![0xe0, rng.byte(0x80, 0x9f), rng.byte(0x80, 0xbf)],
            4 => vec![0xed, rng.byte(0xa0, 0xbf), rng.byte(0x80, 0xbf)],
            5 => vec![
                0xf0,
                rng.byte(0x80, 0x8f),
                rng.byte(0x80, 0xbf),
                rng.byte(0x80, 0xbf),
            ],
            _ => vec![
                0xf4,
                rng.byte(0x90, 0xbf),
                rng.byte(0x80, 0xbf),
                rng.byte(0x80, 0xbf),
            ],
        };
        compare_drop(&libraries, &input, 0);
    }
}

#[test]
fn c14_filter_wholly_valid() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x14a5_5eed);
    for count in 0..128 {
        let input = mixed_valid(&mut rng, count % 16);
        for replacement in [false, true] {
            let output = compare_filter(&libraries, &input, replacement);
            assert_eq!(output, input);
        }
    }
}

#[test]
fn c15_filter_delete_invalid_bytes() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x15a5_5eed);
    for _ in 0..128 {
        let prefix_count = rng.usize(0, 8);
        let prefix = mixed_valid(&mut rng, prefix_count);
        let suffix_count = rng.usize(0, 8);
        let suffix = mixed_valid(&mut rng, suffix_count);
        let invalid_count = rng.usize(1, 32);
        let mut input = prefix.clone();
        for _ in 0..invalid_count {
            input.push([0x80, 0xc0, 0xc1, 0xf5, 0xff][rng.usize(0, 4)]);
        }
        input.extend(&suffix);
        let output = compare_filter(&libraries, &input, false);
        let expected: Vec<u8> = prefix.into_iter().chain(suffix).collect();
        assert_eq!(output, expected);
    }
}

#[test]
fn c16_filter_delete_then_copy_all_widths() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x16a5_5eed);
    for _ in 0..128 {
        let suffix_count = rng.usize(4, 20);
        let suffix = mixed_valid(&mut rng, suffix_count);
        let mut input = vec![0xff];
        input.extend(&suffix);
        assert_eq!(compare_filter(&libraries, &input, false), suffix);
    }
}

#[test]
fn c17_filter_replace_one_invalid_byte() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x17a5_5eed);
    for _ in 0..128 {
        let prefix_count = rng.usize(0, 8);
        let prefix = mixed_valid(&mut rng, prefix_count);
        let suffix_count = rng.usize(0, 8);
        let suffix = mixed_valid(&mut rng, suffix_count);
        let mut input = prefix.clone();
        input.push([0x80, 0xc0, 0xc1, 0xf5, 0xff][rng.usize(0, 4)]);
        input.extend(&suffix);
        let output = compare_filter(&libraries, &input, true);
        let expected: Vec<u8> = prefix
            .into_iter()
            .chain([0xef, 0xbf, 0xbd])
            .chain(suffix)
            .collect();
        assert_eq!(output, expected);
    }
}

#[test]
fn c18_filter_replace_then_copy_all_widths() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x18a5_5eed);
    for _ in 0..128 {
        let suffix_count = rng.usize(4, 20);
        let suffix = mixed_valid(&mut rng, suffix_count);
        let mut input = vec![0xff];
        input.extend(&suffix);
        let output = compare_filter(&libraries, &input, true);
        let expected: Vec<u8> = [0xef, 0xbf, 0xbd].into_iter().chain(suffix).collect();
        assert_eq!(output, expected);
    }
}

#[test]
fn c19_filter_replacements_within_first_reserve() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x19a5_5eed);
    for count in [1, 2, 10, 100, 1365] {
        let output = compare_filter(&libraries, &vec![0xff; count], true);
        assert_eq!(output.len(), count * 3);
    }
    for _ in 0..64 {
        let count = rng.usize(1, 1365);
        let output = compare_filter(&libraries, &vec![0x80; count], true);
        assert_eq!(output, [0xef, 0xbf, 0xbd].repeat(count));
    }
}

#[test]
fn c20_filter_replacements_across_reserves() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x20a5_5eed);
    for _ in 0..32 {
        let count = rng.usize(1366, 4096);
        let output = compare_filter(&libraries, &vec![0xff; count], true);
        assert_eq!(output, [0xef, 0xbf, 0xbd].repeat(count));
    }
}

fn run_probe(variable: &str, value: &str, preload: Option<&Path>) -> ExitStatus {
    let mut command = Command::new(env::current_exe().expect("test executable path"));
    command
        .arg("--exact")
        .arg(if variable == "DIFF_NULL_CHILD" {
            "null_pointer_probe"
        } else {
            "allocation_failure_probe"
        })
        .arg("--nocapture")
        .env(variable, value);
    if let Some(preload) = preload {
        command.env("LD_PRELOAD", preload);
    }
    command.status().expect("run isolated differential probe")
}

#[cfg(unix)]
fn signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[test]
fn e01_drop_null_assertion() {
    let c = run_probe("DIFF_NULL_CHILD", "c:drop", None);
    let rust = run_probe("DIFF_NULL_CHILD", "rust:drop", None);
    assert_eq!(signal(rust), signal(c));
    assert_eq!(signal(c), Some(6), "C did not terminate with SIGABRT");
}

#[test]
fn e02_filter_null_assertion() {
    let c = run_probe("DIFF_NULL_CHILD", "c:filter", None);
    let rust = run_probe("DIFF_NULL_CHILD", "rust:filter", None);
    assert_eq!(signal(rust), signal(c));
    assert_eq!(signal(c), Some(6), "C did not terminate with SIGABRT");
}

#[test]
fn null_pointer_probe() {
    let Ok(specification) = env::var("DIFF_NULL_CHILD") else {
        return;
    };
    let (implementation, function) = specification.split_once(':').unwrap();
    let path = if implementation == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        if function == "drop" {
            (api.drop_utf8)(std::ptr::null());
        } else {
            (api.filter_utf8)(std::ptr::null(), false);
        }
    }
    panic!("null-pointer call unexpectedly returned");
}

fn allocator_shim_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let output = env::temp_dir().join(format!(
            "driver-fail-alloc-{}-{}.so",
            std::process::id(),
            env!("CARGO_PKG_VERSION")
        ));
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("fail_alloc.c");
        let status = Command::new("cc")
            .args(["-shared", "-fPIC", "-o"])
            .arg(&output)
            .arg(&source)
            .status()
            .expect("compile allocation-failure shim");
        assert!(
            status.success(),
            "allocation-failure shim compilation failed"
        );
        output
    })
}

#[test]
fn e03_initial_malloc_failure() {
    let preload = allocator_shim_path();
    let c = run_probe("DIFF_ALLOC_CHILD", "c:malloc", Some(preload));
    let rust = run_probe("DIFF_ALLOC_CHILD", "rust:malloc", Some(preload));
    assert_eq!(rust.code(), c.code());
    assert!(c.success(), "C malloc-failure probe failed");
}

#[test]
fn e04_realloc_failure() {
    let preload = allocator_shim_path();
    let c = run_probe("DIFF_ALLOC_CHILD", "c:realloc", Some(preload));
    let rust = run_probe("DIFF_ALLOC_CHILD", "rust:realloc", Some(preload));
    assert_eq!(rust.code(), c.code());
    assert!(c.success(), "C realloc-failure probe failed");
}

#[test]
fn e05_strdup_failure() {
    let preload = allocator_shim_path();
    let c = run_probe("DIFF_ALLOC_CHILD", "c:strdup", Some(preload));
    let rust = run_probe("DIFF_ALLOC_CHILD", "rust:strdup", Some(preload));
    assert_eq!(rust.code(), c.code());
    assert!(c.success(), "C strdup-failure probe failed");
}

#[test]
fn allocation_failure_probe() {
    let Ok(specification) = env::var("DIFF_ALLOC_CHILD") else {
        return;
    };
    let (implementation, failure) = specification.split_once(':').unwrap();
    let path = if implementation == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&path) };
    let process = libloading::os::unix::Library::this();
    let configure = unsafe {
        *process
            .get::<unsafe extern "C" fn(c_int)>(b"fail_alloc_configure\0")
            .expect("preloaded fail_alloc_configure")
    };
    let storage = if failure == "strdup" {
        c_string_storage(b"valid")
    } else {
        c_string_storage(&[0xff])
    };
    let mode = match failure {
        "malloc" => 1,
        "realloc" => 2,
        "strdup" => 3,
        _ => panic!("unknown allocation failure mode"),
    };
    unsafe { configure(mode) };
    let result = unsafe { (api.filter_utf8)(storage.as_ptr().cast(), failure == "realloc") };
    unsafe { configure(0) };
    assert!(result.is_null(), "{failure} failure did not return NULL");
}
