use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;
type FilterFn = unsafe extern "C" fn(*const c_char, u8) -> *mut c_char;

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_drop: DropFn,
    rust_drop: DropFn,
    c_filter: FilterFn,
    rust_filter: FilterFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn apis() -> &'static Apis {
    static APIS: OnceLock<Apis> = OnceLock::new();

    APIS.get_or_init(|| unsafe {
        let c_path = manifest_dir().join("../c_src/build/libdriver.so");
        let rust_path = manifest_dir().join("target/release/libdriver.so");
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

        let c_library = Library::new(&c_path).unwrap();
        let rust_library = Library::new(&rust_path).unwrap();
        let c_drop = *c_library.get::<DropFn>(b"w_utf8_drop\0").unwrap();
        let rust_drop = *rust_library.get::<DropFn>(b"w_utf8_drop\0").unwrap();
        let c_filter = *c_library.get::<FilterFn>(b"w_utf8_filter\0").unwrap();
        let rust_filter = *rust_library.get::<FilterFn>(b"w_utf8_filter\0").unwrap();

        Apis {
            _c_library: c_library,
            _rust_library: rust_library,
            c_drop,
            rust_drop,
            c_filter,
            rust_filter,
        }
    })
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn below(&mut self, exclusive: u32) -> u8 {
        (self.next_u32() % exclusive) as u8
    }

    fn inclusive(&mut self, low: u8, high: u8) -> u8 {
        low + self.below(u32::from(high - low) + 1)
    }
}

fn push_ascii(bytes: &mut Vec<u8>, rng: &mut Rng) {
    bytes.push(rng.inclusive(1, 0x7f));
}

fn push_valid_2(bytes: &mut Vec<u8>, rng: &mut Rng) {
    bytes.push(rng.inclusive(0xc2, 0xdf));
    bytes.push(rng.inclusive(0x80, 0xbf));
}

fn push_valid_3_ordinary(bytes: &mut Vec<u8>, rng: &mut Rng) {
    let lead = loop {
        let value = rng.inclusive(0xe1, 0xef);
        if value != 0xed {
            break value;
        }
    };
    bytes.push(lead);
    bytes.push(rng.inclusive(0x80, 0xbf));
    bytes.push(rng.inclusive(0x80, 0xbf));
}

fn push_valid_3_e0(bytes: &mut Vec<u8>, rng: &mut Rng) {
    bytes.push(0xe0);
    bytes.push(rng.inclusive(0xa0, 0xbf));
    bytes.push(rng.inclusive(0x80, 0xbf));
}

fn push_valid_3_ed(bytes: &mut Vec<u8>, rng: &mut Rng) {
    bytes.push(0xed);
    bytes.push(rng.inclusive(0x80, 0x9f));
    bytes.push(rng.inclusive(0x80, 0xbf));
}

fn push_valid_4_ordinary(bytes: &mut Vec<u8>, rng: &mut Rng) {
    bytes.push(rng.inclusive(0xf1, 0xf3));
    for _ in 0..3 {
        bytes.push(rng.inclusive(0x80, 0xbf));
    }
}

fn push_valid_4_f0(bytes: &mut Vec<u8>, rng: &mut Rng) {
    bytes.push(0xf0);
    bytes.push(rng.inclusive(0x90, 0xbf));
    bytes.push(rng.inclusive(0x80, 0xbf));
    bytes.push(rng.inclusive(0x80, 0xbf));
}

fn push_valid_4_f4(bytes: &mut Vec<u8>, rng: &mut Rng) {
    bytes.push(0xf4);
    bytes.push(rng.inclusive(0x80, 0x8f));
    bytes.push(rng.inclusive(0x80, 0xbf));
    bytes.push(rng.inclusive(0x80, 0xbf));
}

fn push_valid_kind(bytes: &mut Vec<u8>, rng: &mut Rng, kind: u8) {
    match kind % 8 {
        0 => push_ascii(bytes, rng),
        1 => push_valid_2(bytes, rng),
        2 => push_valid_3_ordinary(bytes, rng),
        3 => push_valid_3_e0(bytes, rng),
        4 => push_valid_3_ed(bytes, rng),
        5 => push_valid_4_ordinary(bytes, rng),
        6 => push_valid_4_f0(bytes, rng),
        _ => push_valid_4_f4(bytes, rng),
    }
}

fn random_valid(rng: &mut Rng, characters: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    for _ in 0..characters {
        let kind = rng.below(8);
        push_valid_kind(&mut bytes, rng, kind);
    }
    bytes
}

fn invalid_patterns() -> &'static [&'static [u8]] {
    &[
        &[0x80],
        &[0xbf],
        &[0xc0, 0x80],
        &[0xc1, 0xbf],
        &[0xc2],
        &[0xdf, 0x7f],
        &[0xe0, 0x9f, 0xbf],
        &[0xed, 0xa0, 0x80],
        &[0xe1],
        &[0xe1, 0x80],
        &[0xe1, 0x7f, 0x80],
        &[0xe1, 0x80, 0x7f],
        &[0xf0, 0x8f, 0xbf, 0xbf],
        &[0xf4, 0x90, 0x80, 0x80],
        &[0xf5, 0x80, 0x80, 0x80],
        &[0xff],
        &[0xf1],
        &[0xf1, 0x80],
        &[0xf1, 0x80, 0x80],
        &[0xf1, 0x7f, 0x80, 0x80],
        &[0xf1, 0x80, 0x7f, 0x80],
        &[0xf1, 0x80, 0x80, 0x7f],
    ]
}

fn compare_drop(bytes: &[u8]) -> usize {
    let input = CString::new(bytes).unwrap();
    let base = input.as_ptr() as usize;
    let api = apis();
    let c_pointer = unsafe { (api.c_drop)(input.as_ptr()) } as usize;
    let rust_pointer = unsafe { (api.rust_drop)(input.as_ptr()) } as usize;
    assert!(c_pointer >= base);
    assert!(rust_pointer >= base);
    let c_offset = c_pointer - base;
    let rust_offset = rust_pointer - base;
    assert_eq!(c_offset, rust_offset, "input: {bytes:02x?}");
    c_offset
}

fn compare_filter(bytes: &[u8], replacement: u8) -> Vec<u8> {
    let input = CString::new(bytes).unwrap();
    let api = apis();
    let c_pointer = unsafe { (api.c_filter)(input.as_ptr(), replacement) };
    let rust_pointer = unsafe { (api.rust_filter)(input.as_ptr(), replacement) };
    assert!(!c_pointer.is_null(), "C unexpectedly returned NULL");
    assert!(!rust_pointer.is_null(), "Rust unexpectedly returned NULL");

    let c_output = unsafe { CStr::from_ptr(c_pointer) }.to_bytes().to_vec();
    let rust_output = unsafe { CStr::from_ptr(rust_pointer) }.to_bytes().to_vec();
    unsafe {
        free(c_pointer.cast::<c_void>());
        free(rust_pointer.cast::<c_void>());
    }
    assert_eq!(
        c_output, rust_output,
        "replacement={replacement}, input={bytes:02x?}"
    );
    c_output
}

fn generated_sequences(seed: u64, count: usize, mut push: impl FnMut(&mut Vec<u8>, &mut Rng)) {
    let mut rng = Rng::new(seed);
    for iteration in 0..128 {
        let mut bytes = Vec::new();
        let sequence_count = 1 + usize::from(rng.below(count as u32));
        for _ in 0..sequence_count {
            push(&mut bytes, &mut rng);
        }
        assert_eq!(compare_drop(&bytes), bytes.len(), "iteration {iteration}");
    }
}

#[test]
fn config_01_drop_empty() {
    assert_eq!(compare_drop(&[]), 0);
}

#[test]
fn config_02_drop_ascii() {
    generated_sequences(0x02a5_5eed, 64, push_ascii);
}

#[test]
fn config_03_drop_two_byte() {
    assert_eq!(compare_drop(&[0xc2, 0x80, 0xdf, 0xbf]), 4);
    generated_sequences(0x03a5_5eed, 48, push_valid_2);
}

#[test]
fn config_04_drop_three_byte_ordinary() {
    generated_sequences(0x04a5_5eed, 40, push_valid_3_ordinary);
}

#[test]
fn config_05_drop_three_byte_e0() {
    assert_eq!(compare_drop(&[0xe0, 0xa0, 0x80]), 3);
    generated_sequences(0x05a5_5eed, 40, push_valid_3_e0);
}

#[test]
fn config_06_drop_three_byte_ed() {
    assert_eq!(compare_drop(&[0xed, 0x9f, 0xbf]), 3);
    generated_sequences(0x06a5_5eed, 40, push_valid_3_ed);
}

#[test]
fn config_07_drop_four_byte_ordinary() {
    generated_sequences(0x07a5_5eed, 32, push_valid_4_ordinary);
}

#[test]
fn config_08_drop_four_byte_f0() {
    assert_eq!(compare_drop(&[0xf0, 0x90, 0x80, 0x80]), 4);
    generated_sequences(0x08a5_5eed, 32, push_valid_4_f0);
}

#[test]
fn config_09_drop_four_byte_f4() {
    assert_eq!(compare_drop(&[0xf4, 0x8f, 0xbf, 0xbf]), 4);
    generated_sequences(0x09a5_5eed, 32, push_valid_4_f4);
}

#[test]
fn config_10_drop_mixed_valid_widths() {
    let mut rng = Rng::new(0x10a5_5eed);
    for iteration in 0..128 {
        let characters = 1 + usize::from(rng.below(96));
        let bytes = random_valid(&mut rng, characters);
        assert_eq!(compare_drop(&bytes), bytes.len(), "iteration {iteration}");
    }
}

#[test]
fn config_11_drop_first_invalid() {
    let patterns = invalid_patterns();
    let mut rng = Rng::new(0x11a5_5eed);
    for iteration in 0..256 {
        let prefix_characters = usize::from(rng.below(48));
        let mut bytes = random_valid(&mut rng, prefix_characters);
        let expected = bytes.len();
        let pattern = patterns[usize::from(rng.below(patterns.len() as u32))];
        bytes.extend_from_slice(pattern);
        let suffix_characters = usize::from(rng.below(16));
        bytes.extend_from_slice(&random_valid(&mut rng, suffix_characters));
        assert_eq!(compare_drop(&bytes), expected, "iteration {iteration}");
    }
}

fn push_single_invalid(bytes: &mut Vec<u8>, rng: &mut Rng) {
    const ALWAYS_INVALID: &[u8] = &[
        0x80, 0x91, 0xbf, 0xc0, 0xc1, 0xf5, 0xf6, 0xf7, 0xf8, 0xfc, 0xfe, 0xff,
    ];
    bytes.push(ALWAYS_INVALID[usize::from(rng.below(ALWAYS_INVALID.len() as u32))]);
}

fn generated_filter_after_valid(
    seed: u64,
    replacement: u8,
    mut push: impl FnMut(&mut Vec<u8>, &mut Rng),
) {
    let mut rng = Rng::new(seed);
    for _ in 0..128 {
        let mut bytes = Vec::new();
        push_single_invalid(&mut bytes, &mut rng);
        for _ in 0..(1 + rng.below(48)) {
            push(&mut bytes, &mut rng);
        }
        compare_filter(&bytes, replacement);
    }
}

fn random_interleaved(rng: &mut Rng, chunks: usize) -> Vec<u8> {
    let patterns = invalid_patterns();
    let mut bytes = Vec::new();
    for _ in 0..chunks {
        if rng.below(3) == 0 {
            let pattern = patterns[usize::from(rng.below(patterns.len() as u32))];
            bytes.extend_from_slice(pattern);
        } else {
            let kind = rng.below(8);
            push_valid_kind(&mut bytes, rng, kind);
        }
    }
    bytes
}

fn random_invalid_bytes(rng: &mut Rng, count: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(count);
    for _ in 0..count {
        push_single_invalid(&mut bytes, rng);
    }
    bytes
}

#[test]
fn config_12_filter_all_valid_strdup() {
    assert!(compare_filter(&[], 0).is_empty());
    assert!(compare_filter(&[], 1).is_empty());

    let mut rng = Rng::new(0x12a5_5eed);
    for _ in 0..128 {
        let characters = usize::from(rng.below(128));
        let bytes = random_valid(&mut rng, characters);
        assert_eq!(compare_filter(&bytes, 0), bytes);
        assert_eq!(compare_filter(&bytes, 1), bytes);
    }
}

#[test]
fn config_13_filter_delete_then_ascii() {
    generated_filter_after_valid(0x13a5_5eed, 0, push_ascii);
}

#[test]
fn config_14_filter_delete_then_two_byte() {
    generated_filter_after_valid(0x14a5_5eed, 0, push_valid_2);
}

#[test]
fn config_15_filter_delete_then_three_byte() {
    let mut rng = Rng::new(0x15a5_5eed);
    generated_filter_after_valid(0x15a5_5eed, 0, |bytes, _| {
        let kind = rng.below(3);
        match kind {
            0 => push_valid_3_ordinary(bytes, &mut rng),
            1 => push_valid_3_e0(bytes, &mut rng),
            _ => push_valid_3_ed(bytes, &mut rng),
        }
    });
}

#[test]
fn config_16_filter_delete_then_four_byte() {
    let mut rng = Rng::new(0x16a5_5eed);
    generated_filter_after_valid(0x16a5_5eed, 0, |bytes, _| {
        let kind = rng.below(3);
        match kind {
            0 => push_valid_4_ordinary(bytes, &mut rng),
            1 => push_valid_4_f0(bytes, &mut rng),
            _ => push_valid_4_f4(bytes, &mut rng),
        }
    });
}

#[test]
fn config_17_filter_delete_interleaved() {
    let mut rng = Rng::new(0x17a5_5eed);
    for _ in 0..256 {
        let chunks = 2 + usize::from(rng.below(96));
        let bytes = random_interleaved(&mut rng, chunks);
        compare_filter(&bytes, 0);
    }
}

#[test]
fn config_18_filter_replace_then_ascii() {
    generated_filter_after_valid(0x18a5_5eed, 1, push_ascii);
}

#[test]
fn config_19_filter_replace_then_two_byte() {
    generated_filter_after_valid(0x19a5_5eed, 1, push_valid_2);
}

#[test]
fn config_20_filter_replace_then_three_byte() {
    let mut rng = Rng::new(0x20a5_5eed);
    generated_filter_after_valid(0x20a5_5eed, 1, |bytes, _| {
        let kind = rng.below(3);
        match kind {
            0 => push_valid_3_ordinary(bytes, &mut rng),
            1 => push_valid_3_e0(bytes, &mut rng),
            _ => push_valid_3_ed(bytes, &mut rng),
        }
    });
}

#[test]
fn config_21_filter_replace_then_four_byte() {
    let mut rng = Rng::new(0x21a5_5eed);
    generated_filter_after_valid(0x21a5_5eed, 1, |bytes, _| {
        let kind = rng.below(3);
        match kind {
            0 => push_valid_4_ordinary(bytes, &mut rng),
            1 => push_valid_4_f0(bytes, &mut rng),
            _ => push_valid_4_f4(bytes, &mut rng),
        }
    });
}

#[test]
fn config_22_filter_replace_interleaved() {
    let mut rng = Rng::new(0x22a5_5eed);
    for _ in 0..256 {
        let chunks = 2 + usize::from(rng.below(96));
        let bytes = random_interleaved(&mut rng, chunks);
        compare_filter(&bytes, 1);
    }
}

#[test]
fn config_23_filter_invalid_between_valid_regions() {
    let mut rng = Rng::new(0x23a5_5eed);
    for _ in 0..128 {
        let prefix_characters = 1 + usize::from(rng.below(48));
        let suffix_characters = 1 + usize::from(rng.below(48));
        let mut bytes = random_valid(&mut rng, prefix_characters);
        push_single_invalid(&mut bytes, &mut rng);
        bytes.extend_from_slice(&random_valid(&mut rng, suffix_characters));
        compare_filter(&bytes, 0);
        compare_filter(&bytes, 1);
    }
}

#[test]
fn config_24_filter_first_reallocation() {
    let mut rng = Rng::new(0x24a5_5eed);
    for _ in 0..128 {
        let bytes = random_invalid_bytes(&mut rng, 1);
        assert_eq!(compare_filter(&bytes, 1), [0xef, 0xbf, 0xbd]);
    }
}

#[test]
fn config_25_filter_replacement_reserve_boundary() {
    let mut rng = Rng::new(0x25a5_5eed);
    for _ in 0..32 {
        let bytes = random_invalid_bytes(&mut rng, 1365);
        assert_eq!(compare_filter(&bytes, 1).len(), 4095);
    }
}

#[test]
fn config_26_filter_second_reallocation() {
    let mut rng = Rng::new(0x26a5_5eed);
    for _ in 0..32 {
        let bytes = random_invalid_bytes(&mut rng, 1366);
        assert_eq!(compare_filter(&bytes, 1).len(), 4098);
    }
}

#[test]
fn config_27_filter_large_mixed_input() {
    let mut rng = Rng::new(0x27a5_5eed);
    for _ in 0..64 {
        let mut bytes = Vec::new();
        while bytes.len() <= 5000 {
            if rng.below(3) == 0 {
                push_single_invalid(&mut bytes, &mut rng);
            } else {
                let kind = rng.below(8);
                push_valid_kind(&mut bytes, &mut rng, kind);
            }
        }
        compare_filter(&bytes, 0);
        compare_filter(&bytes, 1);
    }
}

#[test]
fn ffi_boundary_noncanonical_bool_is_nonzero() {
    let input = [0xff];
    assert_eq!(compare_filter(&input, 2), [0xef, 0xbf, 0xbd]);
    assert_eq!(compare_filter(&input, u8::MAX), [0xef, 0xbf, 0xbd]);
}

fn assert_children_abort(test_name: &str, environment: &str) {
    for implementation in ["c", "rust"] {
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test_name, "--nocapture"])
            .env(environment, implementation)
            .status()
            .unwrap();
        assert_eq!(
            status.signal(),
            Some(6),
            "{implementation} status was {status}"
        );
    }
}

#[test]
fn error_01_drop_null_assertion() {
    if let Ok(implementation) = std::env::var("DIFF_NULL_DROP") {
        let api = apis();
        let function = match implementation.as_str() {
            "c" => api.c_drop,
            "rust" => api.rust_drop,
            _ => panic!("unknown implementation"),
        };
        unsafe {
            function(std::ptr::null());
        }
        panic!("null call unexpectedly returned");
    }

    assert_children_abort("error_01_drop_null_assertion", "DIFF_NULL_DROP");
}

#[test]
fn error_02_filter_null_assertion() {
    if let Ok(implementation) = std::env::var("DIFF_NULL_FILTER") {
        let api = apis();
        let function = match implementation.as_str() {
            "c" => api.c_filter,
            "rust" => api.rust_filter,
            _ => panic!("unknown implementation"),
        };
        unsafe {
            function(std::ptr::null(), 0);
        }
        panic!("null call unexpectedly returned");
    }

    assert_children_abort("error_02_filter_null_assertion", "DIFF_NULL_FILTER");
}

fn build_allocation_interposer() -> PathBuf {
    static INTERPOSER: OnceLock<PathBuf> = OnceLock::new();

    INTERPOSER
        .get_or_init(|| {
            let source = manifest_dir().join("tests/alloc_fail.c");
            let output = manifest_dir().join("target/alloc-fail/liballoc_fail.so");
            std::fs::create_dir_all(output.parent().unwrap()).unwrap();
            let status = Command::new("cc")
                .args(["-shared", "-fPIC", "-O2", "-o"])
                .arg(&output)
                .arg(&source)
                .status()
                .unwrap();
            assert!(status.success(), "failed to build allocation interposer");
            assert!(output.is_file());
            output
        })
        .clone()
}

fn run_allocation_failure_child(kind: i32, implementation: &str) {
    type ArmFn = unsafe extern "C" fn(i32);

    let interposer_path = PathBuf::from(std::env::var_os("DIFF_INTERPOSER").unwrap());
    let interposer = unsafe { Library::new(interposer_path) }.unwrap();
    let arm = unsafe {
        *interposer
            .get::<ArmFn>(b"alloc_fail_arm\0")
            .expect("missing alloc_fail_arm")
    };

    let api = apis();
    let function = match implementation {
        "c" => api.c_filter,
        "rust" => api.rust_filter,
        _ => panic!("unknown implementation"),
    };
    let input = if kind == 1 {
        CString::new("valid UTF-8").unwrap()
    } else {
        CString::new([0xff]).unwrap()
    };
    let replacement = u8::from(kind == 3);

    let result = unsafe {
        arm(kind);
        function(input.as_ptr(), replacement)
    };
    assert!(result.is_null(), "allocation failure did not return NULL");
}

fn assert_allocation_failure_for_both(test_name: &str, kind: i32) {
    let interposer = build_allocation_interposer();
    for implementation in ["c", "rust"] {
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test_name, "--nocapture"])
            .env("LD_PRELOAD", &interposer)
            .env("DIFF_INTERPOSER", &interposer)
            .env("DIFF_ALLOC_KIND", kind.to_string())
            .env("DIFF_ALLOC_IMPLEMENTATION", implementation)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "{implementation} allocation child failed with {status}"
        );
    }
}

fn allocation_child_for(kind: i32) -> Option<String> {
    let child_kind: i32 = std::env::var("DIFF_ALLOC_KIND").ok()?.parse().unwrap();
    if child_kind != kind {
        return None;
    }
    Some(std::env::var("DIFF_ALLOC_IMPLEMENTATION").unwrap())
}

#[test]
fn error_03_strdup_failure_returns_null() {
    if let Some(implementation) = allocation_child_for(1) {
        run_allocation_failure_child(1, &implementation);
        return;
    }
    assert_allocation_failure_for_both("error_03_strdup_failure_returns_null", 1);
}

#[test]
fn error_04_malloc_failure_returns_null() {
    if let Some(implementation) = allocation_child_for(2) {
        run_allocation_failure_child(2, &implementation);
        return;
    }
    assert_allocation_failure_for_both("error_04_malloc_failure_returns_null", 2);
}

#[test]
fn error_05_realloc_failure_returns_null() {
    if let Some(implementation) = allocation_child_for(3) {
        run_allocation_failure_child(3, &implementation);
        return;
    }
    assert_allocation_failure_for_both("error_05_realloc_failure_returns_null", 3);
}
