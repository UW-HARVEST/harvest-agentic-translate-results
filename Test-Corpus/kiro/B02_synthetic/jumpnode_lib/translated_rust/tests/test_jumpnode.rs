use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn call_c_jumpnode(op: i32, id: i32, depth: i32, flags: i32) -> i32 {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C .so");
        let func: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
            lib.get(b"jumpnode").expect("Failed to find jumpnode");
        func(op, id, depth, flags)
    }
}

fn call_rust_jumpnode(op: i32, id: i32, depth: i32, flags: i32) -> i32 {
    unsafe { jumpnode_lib::jumpnode(op, id, depth, flags) }
}

fn assert_match(op: i32, id: i32, depth: i32, flags: i32, expected: i32) {
    let rust = call_rust_jumpnode(op, id, depth, flags);
    assert_eq!(rust, expected,
        "Rust != expected for mode={op}, id={id}, depth={depth}, flags={flags}: rust={rust}, expected={expected}");
}

/// Single test to control global state ordering
#[test]
fn test_all_modes() {
    // === Phase 1: Stateless modes (mode 3 and default) ===
    // These don't depend on global state, compare C .so vs Rust directly
    let stateless_cases = [
        (3, 5, 3, 0),
        (3, 5, 3, 127),
        (3, 10, 20, 255),
        (3, 0, 0, 0),
        (3, 1, 2, 10),
        (3, 100, 0, 0),
        (99, 0, 0, 0),
        (0, 0, 0, 0),
        (5, 1, 2, 3),
    ];
    for (op, id, depth, flags) in stateless_cases {
        let c = call_c_jumpnode(op, id, depth, flags);
        let rust = call_rust_jumpnode(op, id, depth, flags);
        assert_eq!(rust, c,
            "Rust != C for mode={op}, id={id}, depth={depth}, flags={flags}: rust={rust}, c={c}");
    }

    // === Phase 2: Uninitialized state (node_count=0) ===
    unsafe { jumpnode_lib::reset_state_for_testing() };
    let uninit_cases = [
        (1, 1, 3, 0, 18),   // STATUS_ERROR | 0o20
        (2, 1, 0, 1, 34),   // STATUS_ERROR | 0o40
        (4, 1, 2, 0, 66),   // STATUS_ERROR | 0o100
    ];
    for (op, id, depth, flags, expected) in uninit_cases {
        assert_match(op, id, depth, flags, expected);
    }

    // === Phase 3: Initialized state ===
    unsafe { jumpnode_lib::initialize_test_data_for_testing() };
    let init_cases = [
        (1, 1, 0, 0, 100),
        (1, 2, 1, 0, 201),
        (1, 4, 3, 0, 251),
        (1, 7, 5, 0, 276),
        (1, 99, 0, 0, 18),
        (2, 1, 0, 0, 1438),
        (2, 1, 0, 1, 1454),
        (2, 1, 4, 2, 830),
        (2, 3, 0, 0, 1438),
        (2, 99, 0, 0, 34),
        (4, 1, 0, 0, 215),
        (4, 1, 5, 0, 282),
        (4, 3, 1, 0, 229),
        (4, 99, 0, 0, 66),
    ];
    for (op, id, depth, flags, expected) in init_cases {
        assert_match(op, id, depth, flags, expected);
    }
}
