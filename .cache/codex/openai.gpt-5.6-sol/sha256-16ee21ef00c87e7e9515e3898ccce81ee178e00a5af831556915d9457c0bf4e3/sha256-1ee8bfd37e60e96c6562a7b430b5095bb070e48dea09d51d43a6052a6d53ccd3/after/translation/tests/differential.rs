use libloading::Library;
use std::ffi::c_uint;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type HdrBitrate = unsafe extern "C" fn(*const u8) -> c_uint;

struct Api {
    _library: Library,
    hdr_bitrate: HdrBitrate,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let hdr_bitrate = unsafe {
            *library
                .get::<HdrBitrate>(b"hdr_bitrate\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load hdr_bitrate from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            hdr_bitrate,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-tgoXJn.so")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("HDR_BITRATE_RUST_LIBRARY") {
        return PathBuf::from(path);
    }

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let target = manifest_dir().join("target").join(profile);
    let candidates = [
        target.join("libhdr_bitrate_lib.so"),
        target.join("deps/libhdr_bitrate_lib.so"),
        manifest_dir().join("target/release/libhdr_bitrate_lib.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("Rust shared library not found under {}", target.display()))
}

fn next_random(state: &mut u64) -> u8 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 24) as u8
}

#[test]
fn all_defined_configurations_match() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "C shared library missing: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "Rust shared library missing: {}",
        rust_path.display()
    );

    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };
    let mut random = 0x6a09_e667_f3bc_c909_u64;

    for version in 0_u8..=1 {
        for layer in 1_u8..=3 {
            for bitrate_index in 0_u8..=14 {
                for sample in 0..1024 {
                    let h0 = next_random(&mut random);
                    let ignored_h1 = next_random(&mut random) & 0xf1;
                    let ignored_h2 = next_random(&mut random) & 0x0f;
                    let header = [
                        h0,
                        ignored_h1 | (version << 3) | (layer << 1),
                        (bitrate_index << 4) | ignored_h2,
                    ];

                    let c_result = unsafe { (c.hdr_bitrate)(header.as_ptr()) };
                    let rust_result = unsafe { (rust.hdr_bitrate)(header.as_ptr()) };
                    assert_eq!(
                        rust_result, c_result,
                        "version={version}, layer={layer}, bitrate_index={bitrate_index}, \
                         sample={sample}, header={header:02x?}"
                    );
                }
            }
        }
    }
}

fn run_null_probe(library: &Path) -> ExitStatus {
    Command::new(std::env::current_exe().expect("integration-test executable path"))
        .args(["--exact", "null_pointer_probe", "--ignored", "--nocapture"])
        .env("HDR_BITRATE_NULL_PROBE_LIBRARY", library)
        .status()
        .unwrap_or_else(|error| {
            panic!(
                "failed to run null-pointer probe for {}: {error}",
                library.display()
            )
        })
}

#[test]
fn null_pointer_boundary_matches_process_termination() {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_null_probe(&c_library_path());
    let rust_status = run_null_probe(&rust_library_path());

    assert!(
        !c_status.success() && !rust_status.success(),
        "null calls unexpectedly returned: C={c_status:?}, Rust={rust_status:?}"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "null calls terminated differently: C={c_status:?}, Rust={rust_status:?}"
    );
    assert!(
        c_status.signal().is_some(),
        "C null call did not terminate by signal: {c_status:?}"
    );
}

#[test]
#[ignore = "run only as an isolated subprocess by null_pointer_boundary_matches_process_termination"]
fn null_pointer_probe() {
    let Some(path) = std::env::var_os("HDR_BITRATE_NULL_PROBE_LIBRARY") else {
        return;
    };
    let api = unsafe { Api::load(Path::new(&path)) };
    let _ = unsafe { (api.hdr_bitrate)(std::ptr::null()) };
}
