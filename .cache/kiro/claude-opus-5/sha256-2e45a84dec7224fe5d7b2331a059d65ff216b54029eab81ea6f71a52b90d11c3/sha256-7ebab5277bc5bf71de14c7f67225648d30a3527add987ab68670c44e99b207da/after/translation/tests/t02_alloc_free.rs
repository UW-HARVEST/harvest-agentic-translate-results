//! Level 2: `allocate_block` / `free_block`.

mod common;

use common::pair;
use std::ffi::c_int;

/// For each `(count, init_value)` the returned block must have the same `size`
/// and the same payload contents in both implementations.
#[test]
fn allocate_block_contents_match() {
    let p = pair();
    let c_alloc = p.c.allocate_block();
    let c_free = p.c.free_block();
    let rs_alloc = p.rs.allocate_block();
    let rs_free = p.rs.free_block();

    let counts: [usize; 10] = [0, 1, 2, 5, 6, 7, 8, 13, 14, 1024];
    let inits: [c_int; 9] = [0, 1, -1, 42, -42, 1000, i32::MAX, i32::MIN, i32::MAX - 3];

    for &count in &counts {
        for &init in &inits {
            let cm = unsafe { c_alloc(count, init) };
            let rm = unsafe { rs_alloc(count, init) };
            assert!(!cm.is_null(), "C allocate_block({count}, {init}) returned NULL");
            assert!(!rm.is_null(), "Rust allocate_block({count}, {init}) returned NULL");

            unsafe {
                assert_eq!(
                    (*cm).size,
                    (*rm).size,
                    "size mismatch for ({count}, {init})"
                );
                assert_eq!(
                    (*cm).data.is_null(),
                    (*rm).data.is_null(),
                    "data nullness mismatch for ({count}, {init})"
                );
                if count > 0 {
                    let cs = std::slice::from_raw_parts((*cm).data, count);
                    let rs = std::slice::from_raw_parts((*rm).data, count);
                    assert_eq!(cs, rs, "payload mismatch for ({count}, {init})");
                    // Cross-check against the C semantics directly:
                    // `mb->data[i] = init_value + i` truncated to int.
                    for (i, &v) in cs.iter().enumerate() {
                        assert_eq!(
                            v,
                            init.wrapping_add(i as c_int),
                            "C payload[{i}] unexpected for ({count}, {init})"
                        );
                    }
                }
                c_free(cm);
                rs_free(rm);
            }
        }
    }
}

/// A `count` large enough to make `calloc` fail must yield NULL from both.
///
/// This is the path `betagamma` relies on for its `-1` return when
/// `(param1 % 10) + 5` is negative and sign-extends into `size_t`.
#[test]
fn allocate_block_reports_calloc_failure_identically() {
    let p = pair();
    let c_alloc = p.c.allocate_block();
    let rs_alloc = p.rs.allocate_block();

    // These are exactly the values `betagamma` can produce:
    // (param1 % 10) + 5 for param1 % 10 in -9..=-6.
    let huge: [usize; 8] = [
        (-1i64) as usize,
        (-2i64) as usize,
        (-3i64) as usize,
        (-4i64) as usize,
        usize::MAX,
        usize::MAX / 2,
        usize::MAX / 4,
        1usize << 62,
    ];

    for &count in &huge {
        let cm = unsafe { c_alloc(count, 0) };
        let rm = unsafe { rs_alloc(count, 0) };
        assert!(cm.is_null(), "C allocate_block({count:#x}) unexpectedly succeeded");
        assert!(
            rm.is_null(),
            "Rust allocate_block({count:#x}) unexpectedly succeeded (C returned NULL)"
        );
    }
}

/// `free_block(NULL)` is a no-op in both implementations.
#[test]
fn free_block_tolerates_null() {
    let p = pair();
    let c_free = p.c.free_block();
    let rs_free = p.rs.free_block();
    unsafe {
        c_free(std::ptr::null_mut());
        rs_free(std::ptr::null_mut());
    }
}

/// A block whose `data` is NULL must also be freed without touching `data`.
#[test]
fn free_block_tolerates_null_payload() {
    let p = pair();
    let c_free: common::FreeBlockFn = *p.c.free_block();
    let rs_free: common::FreeBlockFn = *p.rs.free_block();

    // Allocate the headers with the same libc `malloc` both libraries use, so
    // either `free_block` can legitimately release them.
    unsafe extern "C" {
        fn malloc(n: usize) -> *mut std::ffi::c_void;
    }
    unsafe {
        for f in [c_free, rs_free] {
            let mb =
                malloc(std::mem::size_of::<common::MemoryBlock>()) as *mut common::MemoryBlock;
            assert!(!mb.is_null());
            (*mb).data = std::ptr::null_mut();
            (*mb).size = 0;
            f(mb);
        }
    }
}
