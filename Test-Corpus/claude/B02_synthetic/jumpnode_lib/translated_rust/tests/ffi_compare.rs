// Integration tests comparing C and Rust shared libraries via FFI.
// Both libraries are loaded with libloading and their exported `jumpnode`
// symbol is called with identical inputs; outputs must match byte-for-byte.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type JumpnodeFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo compiles cdylib next to test binaries; resolve via CARGO_MANIFEST_DIR.
    let mut p = workspace_root();
    p.push("target");
    // Try debug then release.
    let debug = p.join("debug").join("libjumpnode_lib.so");
    if debug.exists() {
        return debug;
    }
    let release = p.join("release").join("libjumpnode_lib.so");
    if release.exists() {
        return release;
    }
    panic!(
        "Rust shared library not found. Build with `cargo build` first. Looked at: {} and {}",
        debug.display(),
        release.display()
    );
}

struct Libs {
    _c: Library,
    _r: Library,
    c_jumpnode: JumpnodeFn,
    r_jumpnode: JumpnodeFn,
}

fn load_libs() -> Libs {
    unsafe {
        let c = Library::new(c_lib_path()).expect("failed to load C .so");
        let r = Library::new(rust_lib_path()).expect("failed to load Rust .so");
        let c_sym: Symbol<JumpnodeFn> =
            c.get(b"jumpnode\0").expect("C .so missing jumpnode symbol");
        let r_sym: Symbol<JumpnodeFn> = r
            .get(b"jumpnode\0")
            .expect("Rust .so missing jumpnode symbol");
        let c_jumpnode = *c_sym;
        let r_jumpnode = *r_sym;
        Libs {
            _c: c,
            _r: r,
            c_jumpnode,
            r_jumpnode,
        }
    }
}

fn check(libs: &Libs, a: c_int, b: c_int, c: c_int, d: c_int) {
    let cv = unsafe { (libs.c_jumpnode)(a, b, c, d) };
    let rv = unsafe { (libs.r_jumpnode)(a, b, c, d) };
    assert_eq!(
        cv, rv,
        "mismatch for jumpnode({}, {}, {}, {}): C={} Rust={}",
        a, b, c, d, cv, rv
    );
}

#[test]
fn jumpnode_mode_1_node_lookup_failure() {
    // operation_mode=1, no nodes have been inserted (initialize_test_data
    // is static and never called from outside), so find_node_by_id returns
    // NULL and the function returns STATUS_ERROR | 0o0020.
    let libs = load_libs();
    for node_id in &[-100, -1, 0, 1, 7, 100, i32::MAX, i32::MIN] {
        for depth in &[-5, 0, 1, 5, 100] {
            for flags in &[-1, 0, 1, 0o177, i32::MAX] {
                check(&libs, 0o0001, *node_id, *depth, *flags);
            }
        }
    }
}

#[test]
fn jumpnode_mode_2_node_lookup_failure() {
    let libs = load_libs();
    for node_id in &[-1, 0, 1, 7, 100] {
        for depth in &[-5, 0, 1, 16, 20, 100] {
            for flags in &[-1, 0, 1, 100] {
                check(&libs, 0o0002, *node_id, *depth, *flags);
            }
        }
    }
}

#[test]
fn jumpnode_mode_3_format_and_compute() {
    // mode 3 doesn't need any node storage; it just sprintf's
    // "Node_<id>_Depth_<depth>" and computes a size metric.
    let libs = load_libs();
    for node_id in &[-1, 0, 1, 5, 12345] {
        for depth in &[-1, 0, 1, 99, 1000] {
            for flags in &[0, 1, 0o177, 0xFF, -1, 12345] {
                check(&libs, 0o0003, *node_id, *depth, *flags);
            }
        }
    }
}

#[test]
fn jumpnode_mode_4_node_lookup_failure() {
    let libs = load_libs();
    for node_id in &[-1, 0, 1, 7] {
        for depth in &[-1, 0, 1, 5] {
            for flags in &[0, 1, -1] {
                check(&libs, 0o0004, *node_id, *depth, *flags);
            }
        }
    }
}

#[test]
fn jumpnode_default_mode() {
    let libs = load_libs();
    for op in &[0, 5, 6, 7, 8, 100, -1, i32::MAX, i32::MIN] {
        check(&libs, *op, 0, 0, 0);
        check(&libs, *op, 1, 2, 3);
        check(&libs, *op, -1, -1, -1);
    }
}

#[test]
fn jumpnode_exhaustive_sample() {
    // Wider sweep covering all operation modes including invalid ones.
    let libs = load_libs();
    let modes: [c_int; 8] = [0, 0o0001, 0o0002, 0o0003, 0o0004, 5, 9, -1];
    let ids: [c_int; 5] = [-1, 0, 1, 7, 100];
    let depths: [c_int; 5] = [-1, 0, 1, 5, 32];
    let flags: [c_int; 5] = [-1, 0, 1, 0o177, 0xFF];
    for &m in &modes {
        for &i in &ids {
            for &d in &depths {
                for &f in &flags {
                    check(&libs, m, i, d, f);
                }
            }
        }
    }
}

#[test]
fn exported_symbols_match() {
    // Ensure the Rust library exports `jumpnode` like the C one.
    unsafe {
        let r = Library::new(rust_lib_path()).unwrap();
        let _: Symbol<JumpnodeFn> = r.get(b"jumpnode\0").expect("Rust missing jumpnode");
    }
}
