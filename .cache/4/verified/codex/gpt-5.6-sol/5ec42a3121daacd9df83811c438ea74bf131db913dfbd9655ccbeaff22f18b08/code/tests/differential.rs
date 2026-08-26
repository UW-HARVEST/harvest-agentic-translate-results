use libloading::{Library, Symbol};
use std::collections::BTreeSet;
use std::ffi::c_uint;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

type HdrBitrate = unsafe extern "C" fn(*const u8) -> c_uint;

const CASES_PER_CONFIGURATION: usize = 256;
const NULL_CHILD_ENV: &str = "HDR_BITRATE_NULL_CHILD";

struct Generator(u64);

impl Generator {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u8(&mut self) -> u8 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u8
    }
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let deps_directory = std::env::current_exe()
        .expect("test executable path")
        .parent()
        .expect("test executable directory")
        .to_owned();
    let in_deps = deps_directory.join("libhdr_bitrate_lib.so");
    if in_deps.exists() {
        in_deps
    } else {
        deps_directory
            .parent()
            .expect("target profile directory")
            .join("libhdr_bitrate_lib.so")
    }
}

fn with_functions(test: impl FnOnce(HdrBitrate, HdrBitrate)) {
    let c_library =
        unsafe { Library::new(c_library_path()) }.expect("load C ground-truth shared object");
    let rust_library =
        unsafe { Library::new(rust_library_path()) }.expect("load Rust shared object");
    let c_function: Symbol<HdrBitrate> =
        unsafe { c_library.get(b"hdr_bitrate\0") }.expect("load C hdr_bitrate");
    let rust_function: Symbol<HdrBitrate> =
        unsafe { rust_library.get(b"hdr_bitrate\0") }.expect("load Rust hdr_bitrate");

    test(*c_function, *rust_function);
}

fn compare_call(
    c_function: HdrBitrate,
    rust_function: HdrBitrate,
    header: &[u8; 3],
    context: &str,
) {
    let c_result = unsafe { c_function(header.as_ptr()) };
    let rust_result = unsafe { rust_function(header.as_ptr()) };
    assert_eq!(
        c_result.to_ne_bytes(),
        rust_result.to_ne_bytes(),
        "{context}: header={header:02x?}, C={c_result}, Rust={rust_result}"
    );
}

fn defined_dynamic_symbols(path: PathBuf) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only", "--extern-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(output.status.success(), "nm failed: {output:?}");

    String::from_utf8(output.stdout)
        .expect("nm output is UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

#[test]
fn defined_dynamic_symbol_parity() {
    assert_eq!(
        defined_dynamic_symbols(c_library_path()),
        defined_dynamic_symbols(rust_library_path())
    );
}

#[test]
fn valid_configuration_surface() {
    with_functions(|c_function, rust_function| {
        let mut generator = Generator::new(0x5eed_d1ff_3a90_0042);

        for version in 0_u8..=1 {
            for layer_bits in 1_u8..=3 {
                for bitrate_index in 0_u8..=14 {
                    for case in 0..CASES_PER_CONFIGURATION {
                        let mut header = [
                            generator.next_u8(),
                            generator.next_u8(),
                            generator.next_u8(),
                        ];
                        header[1] = (header[1] & !0x0e) | (version << 3) | (layer_bits << 1);
                        header[2] = (bitrate_index << 4) | (header[2] & 0x0f);
                        compare_call(
                            c_function,
                            rust_function,
                            &header,
                            &format!(
                                "version={version}, layer={layer_bits}, \
                                 bitrate={bitrate_index}, case={case}"
                            ),
                        );
                    }
                }
            }
        }
    });
}

#[test]
fn invalid_layer_encoding_matches() {
    with_functions(|c_function, rust_function| {
        let mut generator = Generator::new(0xb0ad_1a7e_0000_0002);

        for version in 0_u8..=1 {
            for bitrate_index in 0_u8..=15 {
                for case in 0..CASES_PER_CONFIGURATION {
                    let mut header = [
                        generator.next_u8(),
                        generator.next_u8(),
                        generator.next_u8(),
                    ];
                    header[1] = (header[1] & !0x0e) | (version << 3);
                    header[2] = (bitrate_index << 4) | (header[2] & 0x0f);
                    compare_call(
                        c_function,
                        rust_function,
                        &header,
                        &format!(
                            "invalid layer: version={version}, \
                             bitrate={bitrate_index}, case={case}"
                        ),
                    );
                }
            }
        }
    });
}

#[test]
fn oversized_bitrate_index_matches() {
    with_functions(|c_function, rust_function| {
        let mut generator = Generator::new(0xb17a_7e15_0000_0003);

        for version in 0_u8..=1 {
            for layer_bits in 1_u8..=3 {
                for case in 0..CASES_PER_CONFIGURATION {
                    let mut header = [
                        generator.next_u8(),
                        generator.next_u8(),
                        generator.next_u8(),
                    ];
                    header[1] = (header[1] & !0x0e) | (version << 3) | (layer_bits << 1);
                    header[2] = 0xf0 | (header[2] & 0x0f);
                    compare_call(
                        c_function,
                        rust_function,
                        &header,
                        &format!(
                            "oversized bitrate: version={version}, \
                             layer={layer_bits}, case={case}"
                        ),
                    );
                }
            }
        }
    });
}

fn run_null_child(library: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("test executable path"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env(NULL_CHILD_ENV, library)
        .status()
        .expect("run null-pointer child process")
}

#[test]
fn null_pointer_termination_matches() {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_null_child("c");
    let rust_status = run_null_child("rust");
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "C status={c_status:?}, Rust status={rust_status:?}"
    );
    assert_eq!(
        c_status.signal(),
        Some(11),
        "C ground truth did not SIGSEGV"
    );
}

#[test]
fn null_pointer_child() {
    let Ok(library_name) = std::env::var(NULL_CHILD_ENV) else {
        return;
    };
    let path = match library_name.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown child library {other}"),
    };
    let library = unsafe { Library::new(path) }.expect("load child shared object");
    let function: Symbol<HdrBitrate> =
        unsafe { library.get(b"hdr_bitrate\0") }.expect("load child hdr_bitrate");

    unsafe { function(std::ptr::null()) };
    panic!("null-pointer call unexpectedly returned");
}
