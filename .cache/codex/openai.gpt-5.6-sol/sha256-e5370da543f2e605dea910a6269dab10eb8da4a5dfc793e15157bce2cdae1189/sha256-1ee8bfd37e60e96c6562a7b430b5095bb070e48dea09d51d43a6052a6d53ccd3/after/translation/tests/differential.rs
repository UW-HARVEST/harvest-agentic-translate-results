use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CnRnd {
    state: [u64; 2],
}

type NextDouble = unsafe extern "C" fn(*mut CnRnd) -> f64;

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-ktJGSr.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libnext_double_lib.so")
}

fn assert_libraries_exist() {
    for path in [c_library_path(), rust_library_path()] {
        assert!(
            path.is_file(),
            "missing shared library {}; build both libraries before testing",
            path.display()
        );
    }
}

fn compare_sequence(
    c_next: &Symbol<'_, NextDouble>,
    rust_next: &Symbol<'_, NextDouble>,
    initial_state: [u64; 2],
    calls: usize,
) {
    let mut c_state = CnRnd {
        state: initial_state,
    };
    let mut rust_state = c_state;

    for call in 0..calls {
        // SAFETY: Both symbols have the public C signature and receive valid,
        // writable pointers to identically laid-out state objects.
        let c_result = unsafe { c_next(&mut c_state) };
        let rust_result = unsafe { rust_next(&mut rust_state) };

        assert_eq!(
            rust_result.to_bits(),
            c_result.to_bits(),
            "return mismatch for initial state {initial_state:016x?} at call {call}"
        );
        assert_eq!(
            rust_state, c_state,
            "state mismatch for initial state {initial_state:016x?} at call {call}"
        );
    }
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[test]
fn configuration_1_boundary_and_randomized_states_match() {
    assert_libraries_exist();

    // SAFETY: Paths refer to shared libraries built from this workspace.
    let c_library = unsafe { Library::new(c_library_path()) }.unwrap();
    // SAFETY: Paths refer to shared libraries built from this workspace.
    let rust_library = unsafe { Library::new(rust_library_path()) }.unwrap();
    // SAFETY: Phase A established that both libraries export next_double with
    // the signature declared by the public C header.
    let c_next: Symbol<'_, NextDouble> = unsafe { c_library.get(b"next_double") }.unwrap();
    // SAFETY: The Rust export is required to implement the same C signature.
    let rust_next: Symbol<'_, NextDouble> = unsafe { rust_library.get(b"next_double") }.unwrap();

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
        [0x000f_ffff_ffff_ffff, 0xfff0_0000_0000_0000],
    ];
    for state in boundary_states {
        compare_sequence(&c_next, &rust_next, state, 1_024);
    }

    let mut seed = 0x6a09_e667_f3bc_c909;
    for _ in 0..20_000 {
        let state = [splitmix64(&mut seed), splitmix64(&mut seed)];
        let calls = 1 + (splitmix64(&mut seed) as usize % 64);
        compare_sequence(&c_next, &rust_next, state, calls);
    }
}

#[test]
fn null_pointer_probe() {
    let Some(path) = std::env::var_os("NULL_PROBE_LIBRARY") else {
        return;
    };

    // SAFETY: The parent test supplies a path to one of the two built
    // libraries and intentionally isolates this undefined-behavior probe.
    let library = unsafe { Library::new(path) }.unwrap();
    // SAFETY: Both shared libraries export this public C symbol.
    let next: Symbol<'_, NextDouble> = unsafe { library.get(b"next_double") }.unwrap();
    // SAFETY: Intentional generic FFI-boundary probe. The C API has no null
    // check, so this call is isolated in a subprocess.
    let _ = unsafe { next(std::ptr::null_mut()) };
}

#[cfg(unix)]
#[test]
fn generic_null_pointer_behavior_matches() {
    use std::os::unix::process::ExitStatusExt;

    assert_libraries_exist();
    let current_test_binary = std::env::current_exe().unwrap();

    let run_probe = |library: PathBuf| {
        Command::new(&current_test_binary)
            .arg("--exact")
            .arg("null_pointer_probe")
            .arg("--nocapture")
            .env("NULL_PROBE_LIBRARY", library)
            .status()
            .unwrap()
    };

    let c_status = run_probe(c_library_path());
    let rust_status = run_probe(rust_library_path());

    assert!(
        !c_status.success(),
        "the C null-pointer probe unexpectedly succeeded"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "null-pointer subprocesses terminated differently: C={c_status:?}, Rust={rust_status:?}"
    );
}
