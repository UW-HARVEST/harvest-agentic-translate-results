use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CnRnd {
    state: [u64; 2],
}

type NextDouble = unsafe extern "C" fn(*mut CnRnd) -> f64;

const NULL_CHILD_ENV: &str = "NEXT_DOUBLE_NULL_CHILD_LIBRARY";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn target_dir() -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR") {
        Some(path) if Path::new(&path).is_absolute() => PathBuf::from(path),
        Some(path) => manifest_dir().join(path),
        None => manifest_dir().join("target"),
    }
}

fn rust_library_path() -> PathBuf {
    target_dir()
        .join("debug")
        .join("deps")
        .join("libnext_double_lib.so")
}

unsafe fn load_next_double(library: &Library) -> NextDouble {
    let symbol: Symbol<'_, NextDouble> = unsafe {
        library
            .get(b"next_double\0")
            .expect("shared library must export next_double")
    };
    *symbol
}

fn with_both_apis(test: impl FnOnce(NextDouble, NextDouble)) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared library {}; build c_src first",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library {}",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_next_double = load_next_double(&c_library);
        let rust_next_double = load_next_double(&rust_library);
        test(c_next_double, rust_next_double);
    }
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn compare_sequence(
    c_next_double: NextDouble,
    rust_next_double: NextDouble,
    initial_state: [u64; 2],
    calls: usize,
) {
    let mut c_state = CnRnd {
        state: initial_state,
    };
    let mut rust_state = c_state;

    for call in 0..calls {
        let c_result = unsafe { c_next_double(&mut c_state) };
        let rust_result = unsafe { rust_next_double(&mut rust_state) };
        assert_eq!(
            rust_result.to_bits(),
            c_result.to_bits(),
            "return mismatch for initial state {initial_state:016x?}, call {call}"
        );
        assert_eq!(
            rust_state, c_state,
            "state mismatch for initial state {initial_state:016x?}, call {call}"
        );
    }
}

#[test]
fn v1_arbitrary_state_matches_byte_for_byte() {
    with_both_apis(|c_next_double, rust_next_double| {
        let boundary_states = [
            [0, 0],
            [0, 1],
            [1, 0],
            [1, 1],
            [0, u64::MAX],
            [u64::MAX, 0],
            [u64::MAX, u64::MAX],
            [1 << 22, 1 << 25],
            [1 << 23, 1 << 26],
            [1 << 63, 1 << 63],
        ];

        for state in boundary_states {
            compare_sequence(c_next_double, rust_next_double, state, 257);
        }

        let mut seed = 0x4d59_5df4_d0f3_3173;
        for _ in 0..4096 {
            let state = [splitmix64(&mut seed), splitmix64(&mut seed)];
            let calls = 1 + (splitmix64(&mut seed) as usize & 63);
            compare_sequence(c_next_double, rust_next_double, state, calls);
        }
    });
}

fn run_null_child(library_path: &Path) -> ExitStatus {
    Command::new(std::env::current_exe().expect("find current test executable"))
        .arg("--exact")
        .arg("null_pointer_child")
        .arg("--nocapture")
        .env(NULL_CHILD_ENV, library_path)
        .status()
        .expect("run null-pointer child process")
}

#[test]
fn null_pointer_child() {
    let Some(library_path) = std::env::var_os(NULL_CHILD_ENV) else {
        return;
    };

    unsafe {
        let library = Library::new(library_path).expect("load child shared library");
        let next_double = load_next_double(&library);
        let _ = next_double(std::ptr::null_mut());
    }

    panic!("next_double unexpectedly returned for a null pointer");
}

#[test]
#[cfg(unix)]
fn g1_null_pointer_boundary_has_same_process_result() {
    let c_status = run_null_child(&c_library_path());
    let rust_status = run_null_child(&rust_library_path());

    assert!(
        !c_status.success(),
        "C null-pointer call unexpectedly succeeded"
    );
    assert!(
        !rust_status.success(),
        "Rust null-pointer call unexpectedly succeeded"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "null-pointer calls terminated with different signals"
    );
}
