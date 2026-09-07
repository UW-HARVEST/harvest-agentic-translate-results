use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

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

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-eBuD7L.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libmd5_digest_lib.so")
}

unsafe fn load_library(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared library does not exist: {}",
        path.display()
    );
    unsafe { Library::new(path) }.unwrap_or_else(|error| {
        panic!("failed to load {}: {error}", path.display());
    })
}

unsafe fn digest_with_canaries(library: &Library, state: &TflacMd5) -> [u8; 18] {
    let digest: Symbol<Md5Digest> =
        unsafe { library.get(b"md5_digest\0") }.expect("shared object must export md5_digest");
    let mut output = [0xa5; 18];
    unsafe { digest(state, output.as_mut_ptr().add(1)) };
    output
}

unsafe fn digest_in_place(
    library: &Library,
    storage: &mut [u32; 16],
    state_offset: usize,
    output_offset: usize,
) {
    let digest: Symbol<Md5Digest> =
        unsafe { library.get(b"md5_digest\0") }.expect("shared object must export md5_digest");
    let bytes = storage.as_mut_ptr().cast::<u8>();
    let state = unsafe { bytes.add(state_offset).cast::<TflacMd5>() };
    let output = unsafe { bytes.add(output_offset) };
    unsafe { digest(state, output) };
}

fn boundary_states() -> Vec<TflacMd5> {
    let mut states = vec![
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
            c: 0xfedc_ba98,
            d: 0x7654_3210,
        },
        TflacMd5 {
            a: 0x0000_00ff,
            b: 0x0000_ff00,
            c: 0x00ff_0000,
            d: 0xff00_0000,
        },
        TflacMd5 {
            a: 0xaaaa_aaaa,
            b: 0x5555_5555,
            c: 0x8000_0000,
            d: 0x0000_0001,
        },
    ];

    for bit in 0..32 {
        let one = 1_u32 << bit;
        states.push(TflacMd5 {
            a: one,
            b: !one,
            c: one.rotate_left(8),
            d: one.rotate_right(8),
        });
    }
    states
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn random_state(seed: &mut u64) -> TflacMd5 {
    TflacMd5 {
        a: splitmix64(seed) as u32,
        b: splitmix64(seed) as u32,
        c: splitmix64(seed) as u32,
        d: splitmix64(seed) as u32,
    }
}

#[test]
fn md5_digest_matches_for_configuration_row_1() {
    unsafe {
        let c_library = load_library(&c_library_path());
        let rust_library = load_library(&rust_library_path());

        for state in boundary_states() {
            let c_output = digest_with_canaries(&c_library, &state);
            let rust_output = digest_with_canaries(&rust_library, &state);
            assert_eq!(rust_output, c_output, "boundary state: {state:?}");
            assert_eq!(c_output[0], 0xa5, "C wrote before the 16-byte output");
            assert_eq!(c_output[17], 0xa5, "C wrote after the 16-byte output");
        }

        let mut seed = 0xd1ff_3a71_5eed_c0de;
        for case in 0..50_000 {
            let state = random_state(&mut seed);
            let c_output = digest_with_canaries(&c_library, &state);
            let rust_output = digest_with_canaries(&rust_library, &state);
            assert_eq!(
                rust_output, c_output,
                "fixed-seed randomized case {case}, state: {state:?}"
            );
        }
    }
}

#[test]
fn md5_digest_matches_for_overlapping_state_and_output() {
    const STATE_OFFSET: usize = 24;

    unsafe {
        let c_library = load_library(&c_library_path());
        let rust_library = load_library(&rust_library_path());
        let mut seed = 0xa11a_51a5_0dd5_eed5;

        for relative_output_offset in -15_isize..=15 {
            let output_offset = (STATE_OFFSET as isize + relative_output_offset) as usize;

            for case in 0..256 {
                let mut initial = [0_u32; 16];
                for word in &mut initial {
                    *word = splitmix64(&mut seed) as u32;
                }
                let mut c_storage = initial;
                let mut rust_storage = initial;

                digest_in_place(&c_library, &mut c_storage, STATE_OFFSET, output_offset);
                digest_in_place(
                    &rust_library,
                    &mut rust_storage,
                    STATE_OFFSET,
                    output_offset,
                );

                assert_eq!(
                    rust_storage, c_storage,
                    "overlap offset {relative_output_offset}, case {case}"
                );
            }
        }
    }
}

fn run_null_probe(library_kind: &str, null_argument: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("integration test executable path"))
        .args(["--exact", "null_pointer_probe", "--ignored", "--nocapture"])
        .env("DIFFERENTIAL_LIBRARY", library_kind)
        .env("DIFFERENTIAL_NULL_ARGUMENT", null_argument)
        .status()
        .expect("null-pointer probe process must start")
}

#[test]
fn null_pointer_boundary_behavior_matches() {
    for null_argument in ["state", "output"] {
        let c_status = run_null_probe("c", null_argument);
        let rust_status = run_null_probe("rust", null_argument);

        assert!(
            !c_status.success(),
            "C unexpectedly accepted null {null_argument}"
        );
        assert!(
            !rust_status.success(),
            "Rust unexpectedly accepted null {null_argument}"
        );

        #[cfg(unix)]
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "different terminating signal for null {null_argument}: \
             C={c_status:?}, Rust={rust_status:?}"
        );

        #[cfg(not(unix))]
        assert_eq!(
            rust_status.code(),
            c_status.code(),
            "different termination status for null {null_argument}: \
             C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

#[test]
#[ignore = "isolated child-process probe invoked by null_pointer_boundary_behavior_matches"]
fn null_pointer_probe() {
    let library_kind =
        std::env::var("DIFFERENTIAL_LIBRARY").expect("probe library kind must be set");
    let null_argument =
        std::env::var("DIFFERENTIAL_NULL_ARGUMENT").expect("probe null argument must be set");
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown probe library kind: {other}"),
    };

    unsafe {
        let library = load_library(&path);
        let digest: Symbol<Md5Digest> = library
            .get(b"md5_digest\0")
            .expect("shared object must export md5_digest");
        let state = TflacMd5 {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
        };
        let mut output = [0_u8; 16];

        match null_argument.as_str() {
            "state" => digest(std::ptr::null(), output.as_mut_ptr()),
            "output" => digest(&state, std::ptr::null_mut()),
            other => panic!("unknown null argument: {other}"),
        }
    }
}
