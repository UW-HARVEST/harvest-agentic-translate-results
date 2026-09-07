use libloading::{Library, Symbol};
use std::ffi::c_uint;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type HdrBitrate = unsafe extern "C" fn(*const u8) -> c_uint;

const CASES_PER_CONFIGURATION: usize = 512;
const FIXED_SEED: u64 = 0x6d70_3368_6472_6272;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("../c_src/build")
        .join("libharvest-work-2TNgXE.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir()
        .join("target/release")
        .join("libhdr_bitrate_lib.so")
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "shared library does not exist: {}; build both release libraries first",
        path.display()
    );
}

struct XorShift64(u64);

impl XorShift64 {
    fn next_u8(&mut self) -> u8 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u8
    }
}

#[test]
fn all_valid_configuration_rows_match_byte_for_byte() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert_library_exists(&c_path);
    assert_library_exists(&rust_path);

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_hdr_bitrate: Symbol<HdrBitrate> =
            c_library.get(b"hdr_bitrate\0").expect("load C hdr_bitrate");
        let rust_hdr_bitrate: Symbol<HdrBitrate> = rust_library
            .get(b"hdr_bitrate\0")
            .expect("load Rust hdr_bitrate");
        let mut random = XorShift64(FIXED_SEED);

        for version in 0_u8..=1 {
            for layer_selector in 1_u8..=3 {
                for bitrate_index in 0_u8..=14 {
                    for case_index in 0..CASES_PER_CONFIGURATION {
                        let ignored_h1 = random.next_u8() & 0xf1;
                        let ignored_h2 = random.next_u8() & 0x0f;
                        let trailing_length = usize::from(random.next_u8() & 0x0f);
                        let mut header = Vec::with_capacity(3 + trailing_length);
                        header.push(random.next_u8());
                        header.push(
                            ignored_h1 | (version << 3) | (layer_selector << 1),
                        );
                        header.push((bitrate_index << 4) | ignored_h2);
                        for _ in 0..trailing_length {
                            header.push(random.next_u8());
                        }

                        let c_result = c_hdr_bitrate(header.as_ptr());
                        let rust_result = rust_hdr_bitrate(header.as_ptr());
                        assert_eq!(
                            c_result.to_ne_bytes(),
                            rust_result.to_ne_bytes(),
                            "divergence at version={version}, layer_selector={layer_selector}, \
                             bitrate_index={bitrate_index}, case={case_index}, header={header:02x?}"
                        );
                    }
                }
            }
        }
    }
}

fn run_probe(library: &str, probe: &str) -> Output {
    Command::new(std::env::current_exe().expect("locate integration test executable"))
        .args(["--exact", "ffi_invalid_input_probe_child", "--nocapture"])
        .env("HDR_PROBE_LIBRARY", library)
        .env("HDR_PROBE_CASE", probe)
        .output()
        .expect("run isolated FFI probe")
}

fn termination_signature(output: &Output) -> (Option<i32>, Option<i32>) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        (output.status.code(), output.status.signal())
    }
    #[cfg(not(unix))]
    {
        (output.status.code(), None)
    }
}

#[test]
fn generic_invalid_inputs_match_observable_ffi_behavior() {
    for probe in ["null", "layer_zero", "bitrate_fifteen"] {
        let c_output = run_probe("c", probe);
        let rust_output = run_probe("rust", probe);
        assert_eq!(
            termination_signature(&c_output),
            termination_signature(&rust_output),
            "termination differs for {probe}\nC stderr: {}\nRust stderr: {}",
            String::from_utf8_lossy(&c_output.stderr),
            String::from_utf8_lossy(&rust_output.stderr),
        );
        assert_eq!(
            c_output.stdout,
            rust_output.stdout,
            "observable result differs for {probe}\nC stderr: {}\nRust stderr: {}",
            String::from_utf8_lossy(&c_output.stderr),
            String::from_utf8_lossy(&rust_output.stderr),
        );
    }
}

#[test]
fn ffi_invalid_input_probe_child() {
    let Ok(library_kind) = std::env::var("HDR_PROBE_LIBRARY") else {
        return;
    };
    let probe = std::env::var("HDR_PROBE_CASE").expect("probe case");
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown library kind: {other}"),
    };
    assert_library_exists(&path);

    unsafe {
        let library = Library::new(path).expect("load probed shared library");
        let hdr_bitrate: Symbol<HdrBitrate> =
            library.get(b"hdr_bitrate\0").expect("load hdr_bitrate");
        let result = match probe.as_str() {
            "null" => hdr_bitrate(std::ptr::null()),
            "layer_zero" => {
                let header = [0_u8, 0, 0];
                hdr_bitrate(header.as_ptr())
            }
            "bitrate_fifteen" => {
                let header = [0_u8, 2, 0xf0];
                hdr_bitrate(header.as_ptr())
            }
            other => panic!("unknown probe: {other}"),
        };
        println!("HDR_RESULT={result:08x}");
    }
}
