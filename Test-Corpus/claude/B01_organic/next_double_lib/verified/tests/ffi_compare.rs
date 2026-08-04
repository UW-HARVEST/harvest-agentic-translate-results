use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy)]
struct CnRnd {
    state: [u64; 2],
}

type NextDoubleFn = unsafe extern "C" fn(*mut CnRnd) -> f64;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Try release first, fall back to debug
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let release = manifest.join("target/release/libnext_double_lib.so");
    if release.exists() {
        return release;
    }
    manifest.join("target/debug/libnext_double_lib.so")
}

unsafe fn load(lib_path: &PathBuf) -> (Library, Symbol<'static, NextDoubleFn>) {
    let lib = Library::new(lib_path).expect("failed to load library");
    // SAFETY: we leak the lifetime by transmuting; we keep `lib` alive in the tuple.
    let sym: Symbol<NextDoubleFn> = lib.get(b"next_double\0").expect("symbol not found");
    let sym_static: Symbol<'static, NextDoubleFn> = std::mem::transmute(sym);
    (lib, sym_static)
}

fn run_seeds() -> Vec<[u64; 2]> {
    vec![
        [1, 2],
        [0, 0],
        [u64::MAX, u64::MAX],
        [0xdeadbeef, 0xcafebabe],
        [1, 0],
        [0, 1],
        [0xffffffff_ffffffff, 0],
        [0, 0xffffffff_ffffffff],
        [0x123456789abcdef0, 0x0fedcba987654321],
        [42, 12345],
        [0x8000000000000000, 0x0000000000000001],
        [0xa5a5a5a5a5a5a5a5, 0x5a5a5a5a5a5a5a5a],
    ]
}

#[test]
fn next_double_matches_c() {
    unsafe {
        let c_path = c_lib_path();
        let r_path = rust_lib_path();
        assert!(c_path.exists(), "C library not built at {:?}", c_path);
        assert!(r_path.exists(), "Rust library not built at {:?}", r_path);

        let (_clib, c_next) = load(&c_path);
        let (_rlib, r_next) = load(&r_path);

        for seed in run_seeds() {
            let mut c_state = CnRnd { state: seed };
            let mut r_state = CnRnd { state: seed };

            for i in 0..1024 {
                let c_val = c_next(&mut c_state as *mut _);
                let r_val = r_next(&mut r_state as *mut _);
                assert_eq!(
                    c_val.to_bits(),
                    r_val.to_bits(),
                    "mismatch at seed {:?} iteration {}: C={:?} R={:?}",
                    seed, i, c_val, r_val
                );
                assert_eq!(
                    c_state.state, r_state.state,
                    "state mismatch at seed {:?} iteration {}",
                    seed, i
                );
            }
        }
    }
}
