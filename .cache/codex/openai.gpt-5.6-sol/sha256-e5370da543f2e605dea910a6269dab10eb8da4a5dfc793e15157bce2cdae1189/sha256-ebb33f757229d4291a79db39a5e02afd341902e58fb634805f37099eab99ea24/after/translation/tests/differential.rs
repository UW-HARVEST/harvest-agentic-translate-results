use libloading::Library;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CnRnd {
    state: [u64; 2],
}

type NextDouble = unsafe extern "C" fn(*mut CnRnd) -> f64;

struct Api {
    _library: Library,
    next_double: NextDouble,
}

impl Api {
    fn load(path: &Path) -> Self {
        // SAFETY: The test controls both library paths and keeps the library
        // alive for at least as long as the copied function pointer.
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        // SAFETY: Both libraries are required to export this exact C ABI.
        let next_double = unsafe {
            *library
                .get::<NextDouble>(b"next_double\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load next_double from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            next_double,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}, found {candidates:?}",
        build_dir.display()
    );
    candidates.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libnext_double_lib.so")
}

fn next_splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn compare_sequence(c: &Api, rust: &Api, initial: CnRnd, calls: usize) {
    let mut c_state = initial;
    let mut rust_state = initial;

    for call in 0..calls {
        // SAFETY: Both pointers reference writable repr(C) state objects.
        let c_result = unsafe { (c.next_double)(&mut c_state) };
        // SAFETY: Both pointers reference writable repr(C) state objects.
        let rust_result = unsafe { (rust.next_double)(&mut rust_state) };

        assert_eq!(
            c_result.to_bits(),
            rust_result.to_bits(),
            "return mismatch for initial state {initial:?} at call {call}"
        );
        assert_eq!(
            c_state, rust_state,
            "state mismatch for initial state {initial:?} at call {call}"
        );
    }
}

#[test]
fn config_1_next_double_matches_for_boundary_and_randomized_sequences() {
    let c = Api::load(&c_library_path());
    let rust = Api::load(&rust_library_path());

    let boundary_states = [
        CnRnd { state: [0, 0] },
        CnRnd { state: [0, 1] },
        CnRnd { state: [1, 0] },
        CnRnd { state: [1, 1] },
        CnRnd {
            state: [u64::MAX, 0],
        },
        CnRnd {
            state: [0, u64::MAX],
        },
        CnRnd {
            state: [u64::MAX, u64::MAX],
        },
        CnRnd {
            state: [1_u64 << 63, 1_u64 << 63],
        },
        CnRnd {
            state: [0x5555_5555_5555_5555, 0xaaaa_aaaa_aaaa_aaaa],
        },
    ];

    for initial in boundary_states {
        compare_sequence(&c, &rust, initial, 512);
    }

    let mut seed = 0x8f3c_2a17_d4e5_b609;
    for _case in 0..10_000 {
        let initial = CnRnd {
            state: [next_splitmix64(&mut seed), next_splitmix64(&mut seed)],
        };
        let calls = 1 + (next_splitmix64(&mut seed) as usize % 128);
        compare_sequence(&c, &rust, initial, calls);
    }
}

fn run_null_child(library: &Path) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("null_pointer_child")
        .arg("--nocapture")
        .env("NEXT_DOUBLE_NULL_LIBRARY", library)
        .status()
        .unwrap_or_else(|error| panic!("failed to run null-pointer child: {error}"))
}

#[test]
fn null_pointer_child() {
    let Some(path) = std::env::var_os("NEXT_DOUBLE_NULL_LIBRARY") else {
        return;
    };
    let api = Api::load(Path::new(&path));

    // SAFETY: Deliberately exercises the invalid pointer boundary in an
    // isolated subprocess so its process-level outcome can be compared.
    unsafe {
        (api.next_double)(std::ptr::null_mut());
    }
    panic!("next_double unexpectedly returned for a null pointer");
}

#[test]
fn generic_boundary_g1_null_pointer_process_outcome_matches() {
    let c_status = run_null_child(&c_library_path());
    let rust_status = run_null_child(&rust_library_path());

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "different terminating signals: C={c_status:?}, Rust={rust_status:?}"
        );
        assert!(
            c_status.signal().is_some(),
            "C unexpectedly survived the invalid null pointer: {c_status:?}"
        );
    }

    #[cfg(not(unix))]
    assert_eq!(
        c_status.code(),
        rust_status.code(),
        "different null-pointer process outcomes: C={c_status:?}, Rust={rust_status:?}"
    );
}
