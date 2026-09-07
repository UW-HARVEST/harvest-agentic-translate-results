use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;

type HdrCompare = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

struct Api {
    _library: Library,
    hdr_compare: HdrCompare,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let hdr_compare = unsafe {
            *library
                .get::<HdrCompare>(b"hdr_compare\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load hdr_compare from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            hdr_compare,
        }
    }

    unsafe fn compare(&self, h1: *const u8, h2: *const u8) -> c_int {
        unsafe { (self.hdr_compare)(h1, h2) }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .map(|entry| entry.expect("invalid C build directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        build_dir.display()
    );
    libraries.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libhdr_compare_lib.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        rust_path.is_file(),
        "missing Rust release library {}; run cargo build --release first",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
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

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }

    fn below(&mut self, upper: u8) -> u8 {
        (self.next_u64() % u64::from(upper)) as u8
    }
}

#[derive(Clone, Copy)]
enum HeaderPattern {
    HighF,
    E2,
}

#[derive(Clone, Copy)]
enum UpperNibble {
    Zero,
    NonZero,
}

#[derive(Clone, Copy)]
enum Outcome {
    Byte1Mismatch,
    ModeMismatch,
    ClassMismatch,
    Match,
}

fn valid_h2(rng: &mut Rng, pattern: HeaderPattern, upper: UpperNibble) -> [u8; 3] {
    let byte1 = match pattern {
        HeaderPattern::HighF => {
            let layer = 1 + rng.below(3);
            0xf0 | (rng.byte() & 0x09) | (layer << 1)
        }
        HeaderPattern::E2 => 0xe2 | (rng.byte() & 1),
    };
    let high = match upper {
        UpperNibble::Zero => 0,
        UpperNibble::NonZero => 1 + rng.below(14),
    };
    let mode = rng.below(3);
    [0xff, byte1, (high << 4) | (mode << 2) | (rng.byte() & 3)]
}

fn headers_for_outcome(
    rng: &mut Rng,
    pattern: HeaderPattern,
    upper: UpperNibble,
    outcome: Outcome,
) -> ([u8; 3], [u8; 3]) {
    let h2 = valid_h2(rng, pattern, upper);
    let mut h1 = [rng.byte(), h2[1] ^ (rng.byte() & 1), rng.byte()];
    let h2_mode = (h2[2] >> 2) & 3;
    let h2_is_zero = (h2[2] & 0xf0) == 0;

    match outcome {
        Outcome::Byte1Mismatch => {
            h1[1] ^= 1 << (1 + rng.below(7));
        }
        Outcome::ModeMismatch => {
            let offset = 1 + rng.below(3);
            let h1_mode = (h2_mode + offset) & 3;
            h1[2] = (rng.byte() & 0xf3) | (h1_mode << 2);
        }
        Outcome::ClassMismatch => {
            let high = if h2_is_zero { 1 + rng.below(15) } else { 0 };
            h1[2] = (high << 4) | (h2_mode << 2) | (rng.byte() & 3);
        }
        Outcome::Match => {
            let high = if h2_is_zero { 0 } else { 1 + rng.below(15) };
            h1[2] = (high << 4) | (h2_mode << 2) | (rng.byte() & 3);
        }
    }
    (h1, h2)
}

fn assert_differential(c: &Api, rust: &Api, h1: &[u8; 3], h2: &[u8; 3], context: &str) -> c_int {
    let c_result = unsafe { c.compare(h1.as_ptr(), h2.as_ptr()) };
    let rust_result = unsafe { rust.compare(h1.as_ptr(), h2.as_ptr()) };
    assert_eq!(
        rust_result, c_result,
        "{context}: h1={h1:02x?}, h2={h2:02x?}"
    );
    c_result
}

fn run_config_row(row: u8, pattern: HeaderPattern, upper: UpperNibble, outcome: Outcome) {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909 ^ u64::from(row));
    for iteration in 0..2_048 {
        let (h1, h2) = headers_for_outcome(&mut rng, pattern, upper, outcome);
        let result = assert_differential(
            &c,
            &rust,
            &h1,
            &h2,
            &format!("CONFIGS.md row {row}, iteration {iteration}"),
        );
        let expected = matches!(outcome, Outcome::Match) as c_int;
        assert_eq!(
            result, expected,
            "bad generated case for CONFIGS.md row {row}"
        );
    }
}

macro_rules! config_test {
    ($name:ident, $row:literal, $pattern:expr, $upper:expr, $outcome:expr) => {
        #[test]
        fn $name() {
            run_config_row($row, $pattern, $upper, $outcome);
        }
    };
}

config_test!(
    config_01_high_f_zero_byte1_mismatch,
    1,
    HeaderPattern::HighF,
    UpperNibble::Zero,
    Outcome::Byte1Mismatch
);
config_test!(
    config_02_high_f_nonzero_byte1_mismatch,
    2,
    HeaderPattern::HighF,
    UpperNibble::NonZero,
    Outcome::Byte1Mismatch
);
config_test!(
    config_03_e2_zero_byte1_mismatch,
    3,
    HeaderPattern::E2,
    UpperNibble::Zero,
    Outcome::Byte1Mismatch
);
config_test!(
    config_04_e2_nonzero_byte1_mismatch,
    4,
    HeaderPattern::E2,
    UpperNibble::NonZero,
    Outcome::Byte1Mismatch
);
config_test!(
    config_05_high_f_zero_mode_mismatch,
    5,
    HeaderPattern::HighF,
    UpperNibble::Zero,
    Outcome::ModeMismatch
);
config_test!(
    config_06_high_f_nonzero_mode_mismatch,
    6,
    HeaderPattern::HighF,
    UpperNibble::NonZero,
    Outcome::ModeMismatch
);
config_test!(
    config_07_e2_zero_mode_mismatch,
    7,
    HeaderPattern::E2,
    UpperNibble::Zero,
    Outcome::ModeMismatch
);
config_test!(
    config_08_e2_nonzero_mode_mismatch,
    8,
    HeaderPattern::E2,
    UpperNibble::NonZero,
    Outcome::ModeMismatch
);
config_test!(
    config_09_high_f_zero_class_mismatch,
    9,
    HeaderPattern::HighF,
    UpperNibble::Zero,
    Outcome::ClassMismatch
);
config_test!(
    config_10_high_f_nonzero_class_mismatch,
    10,
    HeaderPattern::HighF,
    UpperNibble::NonZero,
    Outcome::ClassMismatch
);
config_test!(
    config_11_e2_zero_class_mismatch,
    11,
    HeaderPattern::E2,
    UpperNibble::Zero,
    Outcome::ClassMismatch
);
config_test!(
    config_12_e2_nonzero_class_mismatch,
    12,
    HeaderPattern::E2,
    UpperNibble::NonZero,
    Outcome::ClassMismatch
);
config_test!(
    config_13_high_f_zero_match,
    13,
    HeaderPattern::HighF,
    UpperNibble::Zero,
    Outcome::Match
);
config_test!(
    config_14_high_f_nonzero_match,
    14,
    HeaderPattern::HighF,
    UpperNibble::NonZero,
    Outcome::Match
);
config_test!(
    config_15_e2_zero_match,
    15,
    HeaderPattern::E2,
    UpperNibble::Zero,
    Outcome::Match
);
config_test!(
    config_16_e2_nonzero_match,
    16,
    HeaderPattern::E2,
    UpperNibble::NonZero,
    Outcome::Match
);

fn run_error_row(row: u8, generate: impl Fn(&mut Rng) -> ([u8; 3], [u8; 3])) {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b ^ u64::from(row));
    for iteration in 0..2_048 {
        let (h1, h2) = generate(&mut rng);
        let result = assert_differential(
            &c,
            &rust,
            &h1,
            &h2,
            &format!("ERRORS.md row {row}, iteration {iteration}"),
        );
        assert_eq!(result, 0, "bad generated case for ERRORS.md row {row}");
    }
}

#[test]
fn error_01_bad_sync_byte() {
    run_error_row(1, |rng| {
        let mut h2 = valid_h2(rng, HeaderPattern::HighF, UpperNibble::NonZero);
        h2[0] = loop {
            let value = rng.byte();
            if value != 0xff {
                break value;
            }
        };
        ([rng.byte(), rng.byte(), rng.byte()], h2)
    });
}

#[test]
fn error_02_bad_byte1_pattern() {
    run_error_row(2, |rng| {
        let byte1 = loop {
            let value = rng.byte();
            if (value & 0xf0) != 0xf0 && (value & 0xfe) != 0xe2 {
                break value;
            }
        };
        (
            [rng.byte(), rng.byte(), rng.byte()],
            [0xff, byte1, rng.byte()],
        )
    });
}

#[test]
fn error_03_zero_layer_field() {
    run_error_row(3, |rng| {
        let h2 = [0xff, 0xf0 | (rng.byte() & 0x09), rng.byte()];
        ([rng.byte(), rng.byte(), rng.byte()], h2)
    });
}

#[test]
fn error_04_forbidden_upper_nibble() {
    run_error_row(4, |rng| {
        let mut h2 = valid_h2(rng, HeaderPattern::HighF, UpperNibble::NonZero);
        h2[2] = 0xf0 | (rng.below(3) << 2) | (rng.byte() & 3);
        ([rng.byte(), rng.byte(), rng.byte()], h2)
    });
}

#[test]
fn error_05_forbidden_mode_field() {
    run_error_row(5, |rng| {
        let mut h2 = valid_h2(rng, HeaderPattern::HighF, UpperNibble::NonZero);
        h2[2] = (h2[2] & 0xf3) | 0x0c;
        ([rng.byte(), rng.byte(), rng.byte()], h2)
    });
}

#[test]
fn error_06_masked_byte1_mismatch() {
    run_error_row(6, |rng| {
        headers_for_outcome(
            rng,
            HeaderPattern::HighF,
            UpperNibble::NonZero,
            Outcome::Byte1Mismatch,
        )
    });
}

#[test]
fn error_07_mode_field_mismatch() {
    run_error_row(7, |rng| {
        headers_for_outcome(
            rng,
            HeaderPattern::HighF,
            UpperNibble::NonZero,
            Outcome::ModeMismatch,
        )
    });
}

#[test]
fn error_08_zero_class_mismatch() {
    run_error_row(8, |rng| {
        headers_for_outcome(
            rng,
            HeaderPattern::HighF,
            UpperNibble::NonZero,
            Outcome::ClassMismatch,
        )
    });
}

#[test]
fn generic_g3_null_h1_short_circuited_by_invalid_h2() {
    let (c, rust) = load_apis();
    let h2 = [0, 0, 0];
    let c_result = unsafe { c.compare(std::ptr::null(), h2.as_ptr()) };
    let rust_result = unsafe { rust.compare(std::ptr::null(), h2.as_ptr()) };
    assert_eq!(c_result, 0);
    assert_eq!(rust_result, c_result);
}

fn run_crashing_probe(library: &Path, kind: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("failed to resolve current test executable"))
        .arg("--exact")
        .arg("child_pointer_probe")
        .arg("--nocapture")
        .env("HDR_COMPARE_PROBE_LIBRARY", library)
        .env("HDR_COMPARE_PROBE_KIND", kind)
        .status()
        .unwrap_or_else(|error| panic!("failed to launch pointer probe: {error}"))
}

#[cfg(unix)]
#[test]
fn generic_g1_g2_null_pointer_process_behavior() {
    use std::os::unix::process::ExitStatusExt;

    for kind in ["null_h2", "null_h1"] {
        let c_status = run_crashing_probe(&c_library_path(), kind);
        let rust_status = run_crashing_probe(&rust_library_path(), kind);
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "{kind}: C status {c_status:?}, Rust status {rust_status:?}"
        );
        assert_eq!(
            c_status.signal(),
            Some(11),
            "{kind}: expected C to receive SIGSEGV, got {c_status:?}"
        );
    }
}

#[test]
fn child_pointer_probe() {
    let Ok(library_path) = std::env::var("HDR_COMPARE_PROBE_LIBRARY") else {
        return;
    };
    let kind = std::env::var("HDR_COMPARE_PROBE_KIND").expect("missing pointer probe kind");
    let api = unsafe { Api::load(Path::new(&library_path)) };
    let valid = [0xff, 0xfa, 0x10];
    let result = match kind.as_str() {
        "null_h2" => unsafe { api.compare(valid.as_ptr(), std::ptr::null()) },
        "null_h1" => unsafe { api.compare(std::ptr::null(), valid.as_ptr()) },
        other => panic!("unknown pointer probe kind: {other}"),
    };
    panic!("pointer probe unexpectedly returned {result}");
}

#[test]
fn exported_symbol_loads_from_both_shared_libraries() {
    let (_c, _rust) = load_apis();
}
