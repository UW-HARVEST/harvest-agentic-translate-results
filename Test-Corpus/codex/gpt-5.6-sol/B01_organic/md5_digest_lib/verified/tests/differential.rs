use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::OnceLock;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct TflacMd5 {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

type Md5Digest = unsafe extern "C" fn(*const TflacMd5, *mut u8);

const C_LIBRARY: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIBRARY_NAME: &str = "libmd5_digest_lib.so";
const NULL_PROBE_MODE: &str = "MD5_NULL_PROBE_LIBRARY";
const NULL_PROBE_POINTER: &str = "MD5_NULL_PROBE_POINTER";
static RUST_LIBRARY: OnceLock<PathBuf> = OnceLock::new();

fn manifest_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn rust_library_path() -> PathBuf {
    RUST_LIBRARY
        .get_or_init(|| {
            let test_exe =
                std::env::current_exe().expect("current integration-test executable");
            let deps_dir = test_exe.parent().expect("target profile deps directory");
            let output_path = deps_dir.join(RUST_LIBRARY_NAME);
            let source_path = manifest_path("src/lib.rs");
            let output = Command::new("rustc")
                .args([
                    "--edition=2024",
                    "--crate-type=cdylib",
                    "--crate-name=md5_digest_lib",
                ])
                .arg(source_path)
                .arg("-o")
                .arg(&output_path)
                .output()
                .expect("invoke rustc for the differential-test cdylib");

            assert!(
                output.status.success(),
                "rustc failed to build {}:\n{}",
                output_path.display(),
                String::from_utf8_lossy(&output.stderr)
            );
            output_path
        })
        .clone()
}

fn library_paths() -> (PathBuf, PathBuf) {
    let c_path = manifest_path(C_LIBRARY);
    assert!(
        c_path.is_file(),
        "C shared library is missing at {}; build it with CMake first",
        c_path.display()
    );
    (c_path, rust_library_path())
}

unsafe fn load_digest(library: &Library) -> Symbol<'_, Md5Digest> {
    unsafe {
        library
            .get::<Md5Digest>(b"md5_digest\0")
            .expect("load md5_digest")
    }
}

fn next_u32(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

#[test]
fn md5_digest_matches_for_boundaries_and_random_values() {
    let (c_path, rust_path) = library_paths();

    unsafe {
        let c_library = Library::new(c_path).expect("load C shared library");
        let rust_library = Library::new(rust_path).expect("load Rust shared library");
        let c_digest = load_digest(&c_library);
        let rust_digest = load_digest(&rust_library);

        let fixed = [
            TflacMd5 {
                a: 0,
                b: 0,
                c: 0,
                d: 0,
            },
            TflacMd5 {
                a: u32::MAX,
                b: u32::MAX,
                c: u32::MAX,
                d: u32::MAX,
            },
            TflacMd5 {
                a: 0x0123_4567,
                b: 0x89ab_cdef,
                c: 0x55aa_00ff,
                d: 0x8000_0001,
            },
            TflacMd5 {
                a: 1,
                b: 0x100,
                c: 0x1_0000,
                d: 0x100_0000,
            },
        ];

        let compare = |input: TflacMd5| {
            let mut c_output = [0xa5; 16];
            let mut rust_output = [0xa5; 16];
            c_digest(&input, c_output.as_mut_ptr());
            rust_digest(&input, rust_output.as_mut_ptr());
            assert_eq!(rust_output, c_output, "input: {input:?}");
        };

        for input in fixed {
            compare(input);
        }

        let mut state = 0x4d44_355f_4449_4646_u64;
        for _ in 0..10_000 {
            compare(TflacMd5 {
                a: next_u32(&mut state),
                b: next_u32(&mut state),
                c: next_u32(&mut state),
                d: next_u32(&mut state),
            });
        }
    }
}

#[test]
fn md5_digest_matches_for_overlapping_input_and_output() {
    let (c_path, rust_path) = library_paths();

    unsafe {
        let c_library = Library::new(c_path).expect("load C shared library");
        let rust_library = Library::new(rust_path).expect("load Rust shared library");
        let c_digest = load_digest(&c_library);
        let rust_digest = load_digest(&rust_library);
        let mut state = 0x414c_4941_5345_5321_u64;

        for _ in 0..256 {
            let mut initial = [0_u32; 12];
            for word in &mut initial {
                *word = next_u32(&mut state);
            }

            for relative_output_offset in -16_isize..=16 {
                let mut c_storage = initial;
                let mut rust_storage = initial;
                let c_base = c_storage.as_mut_ptr().cast::<u8>();
                let rust_base = rust_storage.as_mut_ptr().cast::<u8>();
                let c_input = c_base.add(16).cast::<TflacMd5>();
                let rust_input = rust_base.add(16).cast::<TflacMd5>();
                let c_output = c_base.offset(16 + relative_output_offset);
                let rust_output = rust_base.offset(16 + relative_output_offset);

                c_digest(c_input, c_output);
                rust_digest(rust_input, rust_output);

                assert_eq!(
                    rust_storage, c_storage,
                    "output offset relative to input: {relative_output_offset}"
                );
            }
        }
    }
}

fn run_null_probe(library_path: &Path, pointer: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current integration-test executable"))
        .args(["--exact", "null_probe_child", "--test-threads=1"])
        .env(NULL_PROBE_MODE, library_path)
        .env(NULL_PROBE_POINTER, pointer)
        .status()
        .expect("run isolated null-pointer probe")
}

#[test]
#[cfg(unix)]
fn null_pointer_boundaries_are_not_accepted() {
    let (c_path, rust_path) = library_paths();

    for pointer in ["input", "output"] {
        let c_status = run_null_probe(&c_path, pointer);
        let rust_status = run_null_probe(&rust_path, pointer);

        assert!(
            c_status.signal().is_some(),
            "C {pointer}-null probe unexpectedly returned: {c_status}"
        );
        assert!(
            rust_status.signal().is_some(),
            "Rust {pointer}-null probe unexpectedly returned: {rust_status}"
        );
    }
}

#[test]
fn null_probe_child() {
    let Some(library_path) = std::env::var_os(NULL_PROBE_MODE) else {
        return;
    };
    let pointer = std::env::var(NULL_PROBE_POINTER).expect("null probe pointer kind");
    let input = TflacMd5 {
        a: 0x0123_4567,
        b: 0x89ab_cdef,
        c: 0x55aa_00ff,
        d: 0x8000_0001,
    };
    let mut output = [0_u8; 16];

    unsafe {
        let library = Library::new(library_path).expect("load null-probe shared library");
        let digest = load_digest(&library);
        match pointer.as_str() {
            "input" => digest(std::ptr::null(), output.as_mut_ptr()),
            "output" => digest(&input, std::ptr::null_mut()),
            _ => panic!("unknown null probe pointer: {pointer}"),
        }
    }

    panic!("md5_digest unexpectedly returned after receiving a null {pointer} pointer");
}
