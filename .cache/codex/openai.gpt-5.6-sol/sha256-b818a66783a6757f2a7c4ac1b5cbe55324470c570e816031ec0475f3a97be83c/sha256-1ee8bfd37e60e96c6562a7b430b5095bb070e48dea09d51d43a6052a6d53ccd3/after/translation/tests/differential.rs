use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type HdrCompare = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

const RANDOM_CASES_PER_ROW: usize = 4096;

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-XreACI.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libhdr_compare_lib.so")
}

fn with_hdr_compare(test: impl FnOnce(HdrCompare, HdrCompare)) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
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
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_function: Symbol<HdrCompare> =
            c_library.get(b"hdr_compare\0").expect("load C hdr_compare");
        let rust_function: Symbol<HdrCompare> = rust_library
            .get(b"hdr_compare\0")
            .expect("load Rust hdr_compare");
        test(*c_function, *rust_function);
    }
}

fn assert_same(
    c_function: HdrCompare,
    rust_function: HdrCompare,
    h1: *const u8,
    h2: *const u8,
    context: &str,
) -> c_int {
    let c_result = unsafe { c_function(h1, h2) };
    let rust_result = unsafe { rust_function(h1, h2) };
    assert_eq!(
        rust_result, c_result,
        "{context}: C returned {c_result}, Rust returned {rust_result}"
    );
    c_result
}

#[derive(Clone, Copy, Debug)]
enum SyncForm {
    FPrefix,
    E2OrE3,
}

#[derive(Clone, Copy, Debug)]
enum HighNibble {
    Zero,
    Nonzero,
}

#[derive(Clone, Copy, Debug)]
enum ComparisonPath {
    Byte1Mismatch,
    ModeMismatch,
    HighClassMismatch,
    Match,
}

#[derive(Clone, Copy, Debug)]
struct Config {
    row: usize,
    sync: SyncForm,
    high: HighNibble,
    comparison: ComparisonPath,
}

const CONFIGS: [Config; 16] = [
    Config {
        row: 1,
        sync: SyncForm::FPrefix,
        high: HighNibble::Zero,
        comparison: ComparisonPath::Byte1Mismatch,
    },
    Config {
        row: 2,
        sync: SyncForm::FPrefix,
        high: HighNibble::Zero,
        comparison: ComparisonPath::ModeMismatch,
    },
    Config {
        row: 3,
        sync: SyncForm::FPrefix,
        high: HighNibble::Zero,
        comparison: ComparisonPath::HighClassMismatch,
    },
    Config {
        row: 4,
        sync: SyncForm::FPrefix,
        high: HighNibble::Zero,
        comparison: ComparisonPath::Match,
    },
    Config {
        row: 5,
        sync: SyncForm::FPrefix,
        high: HighNibble::Nonzero,
        comparison: ComparisonPath::Byte1Mismatch,
    },
    Config {
        row: 6,
        sync: SyncForm::FPrefix,
        high: HighNibble::Nonzero,
        comparison: ComparisonPath::ModeMismatch,
    },
    Config {
        row: 7,
        sync: SyncForm::FPrefix,
        high: HighNibble::Nonzero,
        comparison: ComparisonPath::HighClassMismatch,
    },
    Config {
        row: 8,
        sync: SyncForm::FPrefix,
        high: HighNibble::Nonzero,
        comparison: ComparisonPath::Match,
    },
    Config {
        row: 9,
        sync: SyncForm::E2OrE3,
        high: HighNibble::Zero,
        comparison: ComparisonPath::Byte1Mismatch,
    },
    Config {
        row: 10,
        sync: SyncForm::E2OrE3,
        high: HighNibble::Zero,
        comparison: ComparisonPath::ModeMismatch,
    },
    Config {
        row: 11,
        sync: SyncForm::E2OrE3,
        high: HighNibble::Zero,
        comparison: ComparisonPath::HighClassMismatch,
    },
    Config {
        row: 12,
        sync: SyncForm::E2OrE3,
        high: HighNibble::Zero,
        comparison: ComparisonPath::Match,
    },
    Config {
        row: 13,
        sync: SyncForm::E2OrE3,
        high: HighNibble::Nonzero,
        comparison: ComparisonPath::Byte1Mismatch,
    },
    Config {
        row: 14,
        sync: SyncForm::E2OrE3,
        high: HighNibble::Nonzero,
        comparison: ComparisonPath::ModeMismatch,
    },
    Config {
        row: 15,
        sync: SyncForm::E2OrE3,
        high: HighNibble::Nonzero,
        comparison: ComparisonPath::HighClassMismatch,
    },
    Config {
        row: 16,
        sync: SyncForm::E2OrE3,
        high: HighNibble::Nonzero,
        comparison: ComparisonPath::Match,
    },
];

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

    fn below(&mut self, limit: u8) -> u8 {
        self.byte() % limit
    }
}

fn valid_h2(rng: &mut Rng, sync: SyncForm, high: HighNibble) -> [u8; 3] {
    let byte1 = match sync {
        SyncForm::FPrefix => loop {
            let candidate = 0xf0 | (rng.byte() & 0x0f);
            if ((candidate >> 1) & 3) != 0 {
                break candidate;
            }
        },
        SyncForm::E2OrE3 => 0xe2 | (rng.byte() & 1),
    };
    let high_nibble = match high {
        HighNibble::Zero => 0,
        HighNibble::Nonzero => 1 + rng.below(14),
    };
    let mode = rng.below(3);
    [0xff, byte1, (high_nibble << 4) | (mode << 2) | rng.below(4)]
}

fn configured_h1(
    rng: &mut Rng,
    h2: &[u8; 3],
    high: HighNibble,
    comparison: ComparisonPath,
) -> [u8; 3] {
    let mut h1 = [rng.byte(), h2[1] & 0xfe | rng.below(2), rng.byte()];
    let h2_mode = (h2[2] >> 2) & 3;

    match comparison {
        ComparisonPath::Byte1Mismatch => {
            let differing_bit = 1 + rng.below(7);
            h1[1] ^= 1 << differing_bit;
        }
        ComparisonPath::ModeMismatch => {
            let offset = 1 + rng.below(3);
            let different_mode = (h2_mode + offset) & 3;
            h1[2] = (rng.byte() & 0xf3) | (different_mode << 2);
        }
        ComparisonPath::HighClassMismatch => {
            let h1_high = match high {
                HighNibble::Zero => 1 + rng.below(15),
                HighNibble::Nonzero => 0,
            };
            h1[2] = (h1_high << 4) | (h2_mode << 2) | rng.below(4);
        }
        ComparisonPath::Match => {
            let h1_high = match high {
                HighNibble::Zero => 0,
                HighNibble::Nonzero => 1 + rng.below(15),
            };
            h1[2] = (h1_high << 4) | (h2_mode << 2) | rng.below(4);
        }
    }
    h1
}

#[test]
fn all_valid_configuration_rows_match() {
    with_hdr_compare(|c_function, rust_function| {
        for config in CONFIGS {
            let mut rng = Rng::new(0x4d59_5df4_d0f3_3173 ^ config.row as u64);
            for case in 0..RANDOM_CASES_PER_ROW {
                let h2 = valid_h2(&mut rng, config.sync, config.high);
                let h1 = configured_h1(&mut rng, &h2, config.high, config.comparison);
                let context = format!(
                    "CONFIGS.md row {}, case {case}, h1={h1:02x?}, h2={h2:02x?}",
                    config.row
                );
                let result = assert_same(
                    c_function,
                    rust_function,
                    h1.as_ptr(),
                    h2.as_ptr(),
                    &context,
                );
                let expected = matches!(config.comparison, ComparisonPath::Match) as c_int;
                assert_eq!(result, expected, "{context}");
            }
        }
    });
}

#[test]
fn unrestricted_random_inputs_match() {
    with_hdr_compare(|c_function, rust_function| {
        let mut rng = Rng::new(0x94d0_49bb_1331_11eb);
        for case in 0..100_000 {
            let h1 = [rng.byte(), rng.byte(), rng.byte()];
            let h2 = [rng.byte(), rng.byte(), rng.byte()];
            let context = format!("unrestricted case {case}, h1={h1:02x?}, h2={h2:02x?}");
            assert_same(
                c_function,
                rust_function,
                h1.as_ptr(),
                h2.as_ptr(),
                &context,
            );
        }
    });
}

#[test]
fn all_defined_rejection_rows_match() {
    with_hdr_compare(|c_function, rust_function| {
        let mut rng = Rng::new(0xd1b5_4a32_d192_ed03);
        for case in 0..RANDOM_CASES_PER_ROW {
            let h1 = [rng.byte(), rng.byte(), rng.byte()];

            let row1_h2 = [rng.byte() & 0xfe, rng.byte(), rng.byte()];
            assert_eq!(
                assert_same(
                    c_function,
                    rust_function,
                    h1.as_ptr(),
                    row1_h2.as_ptr(),
                    &format!("ERRORS.md row 1, case {case}")
                ),
                0
            );

            let row2_byte1 = loop {
                let candidate = rng.byte();
                if (candidate & 0xf0) != 0xf0 && (candidate & 0xfe) != 0xe2 {
                    break candidate;
                }
            };
            let row2_h2 = [0xff, row2_byte1, rng.byte()];
            assert_eq!(
                assert_same(
                    c_function,
                    rust_function,
                    h1.as_ptr(),
                    row2_h2.as_ptr(),
                    &format!("ERRORS.md row 2, case {case}")
                ),
                0
            );

            let row3_h2 = [0xff, 0xf0 | rng.below(2), rng.byte()];
            assert_eq!(
                assert_same(
                    c_function,
                    rust_function,
                    h1.as_ptr(),
                    row3_h2.as_ptr(),
                    &format!("ERRORS.md row 3, case {case}")
                ),
                0
            );

            let row4_h2 = [
                0xff,
                if rng.below(2) == 0 {
                    0xf2 | (rng.byte() & 0x0d)
                } else {
                    0xe2 | rng.below(2)
                },
                0xf0 | rng.below(16),
            ];
            assert_eq!(
                assert_same(
                    c_function,
                    rust_function,
                    h1.as_ptr(),
                    row4_h2.as_ptr(),
                    &format!("ERRORS.md row 4, case {case}")
                ),
                0
            );

            let high = rng.below(15);
            let row5_h2 = [0xff, 0xe2 | rng.below(2), (high << 4) | 0x0c | rng.below(4)];
            assert_eq!(
                assert_same(
                    c_function,
                    rust_function,
                    h1.as_ptr(),
                    row5_h2.as_ptr(),
                    &format!("ERRORS.md row 5, case {case}")
                ),
                0
            );
        }
    });
}

#[test]
fn null_h1_is_not_evaluated_when_h2_fails_first_check() {
    with_hdr_compare(|c_function, rust_function| {
        let h2 = [0, 0, 0];
        assert_eq!(
            assert_same(
                c_function,
                rust_function,
                std::ptr::null(),
                h2.as_ptr(),
                "ERRORS.md row 8"
            ),
            0
        );
    });
}

fn run_null_child(library: &Path, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("null_pointer_child")
        .arg("--nocapture")
        .env("HDR_NULL_LIBRARY", library)
        .env("HDR_NULL_CASE", case)
        .status()
        .expect("run null-pointer child process")
}

#[test]
fn null_pointer_termination_matches() {
    use std::os::unix::process::ExitStatusExt;

    for case in ["h2", "h1"] {
        let c_status = run_null_child(&c_library_path(), case);
        let rust_status = run_null_child(&rust_library_path(), case);
        assert!(
            c_status.signal().is_some(),
            "C null-{case} child unexpectedly returned {c_status}"
        );
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "null-{case} termination differs: C={c_status}, Rust={rust_status}"
        );
    }
}

#[test]
fn null_pointer_child() {
    let Ok(library_path) = std::env::var("HDR_NULL_LIBRARY") else {
        return;
    };
    let case = std::env::var("HDR_NULL_CASE").expect("HDR_NULL_CASE");

    unsafe {
        let library = Library::new(&library_path).expect("load child shared library");
        let function: Symbol<HdrCompare> = library
            .get(b"hdr_compare\0")
            .expect("load child hdr_compare");
        let valid = [0xff, 0xe2, 0];
        match case.as_str() {
            "h2" => {
                function(valid.as_ptr(), std::ptr::null());
            }
            "h1" => {
                function(std::ptr::null(), valid.as_ptr());
            }
            _ => panic!("unknown null-pointer case {case}"),
        }
    }
}
