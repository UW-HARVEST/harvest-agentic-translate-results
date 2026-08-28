//! Second level: the dynamic-array primitives `init_array`, `expand_array`,
//! `add_element` and `free_array`. Pointer values themselves are allocator
//! output and cannot match, so the tests compare the observable state: return
//! codes, whether a pointer is NULL, the `size`/`capacity` fields, and the
//! contents of the backing buffer.

mod common;

use common::{DynamicArray, both};
use std::ffi::c_int;
use std::ptr;

/// Compares everything about a returned array that is meaningfully comparable.
fn assert_headers_match(
    label: &str,
    c: &common::Api,
    rust: &common::Api,
    pc: *mut DynamicArray,
    pr: *mut DynamicArray,
) {
    assert_eq!(
        pc.is_null(),
        pr.is_null(),
        "{label}: C returned null={} but Rust returned null={}",
        pc.is_null(),
        pr.is_null()
    );
    if pc.is_null() {
        return;
    }
    let hc = c.read_header(pc);
    let hr = rust.read_header(pr);
    assert_eq!(hc.size, hr.size, "{label}: size");
    assert_eq!(hc.capacity, hr.capacity, "{label}: capacity");
    assert_eq!(
        hc.data.is_null(),
        hr.data.is_null(),
        "{label}: data null-ness"
    );
}

#[test]
fn init_array_matches_for_small_capacities() {
    let (c, rust) = both();
    for cap in 0..=64usize {
        let pc = unsafe { (c.init_array)(cap) };
        let pr = unsafe { (rust.init_array)(cap) };
        assert_headers_match(&format!("init_array({cap})"), c, rust, pc, pr);

        let h = c.read_header(pc);
        assert_eq!(h.size, 0, "init_array({cap}) must start empty");
        assert_eq!(h.capacity, cap, "init_array({cap}) capacity field");

        unsafe { (c.free_array)(pc) };
        unsafe { (rust.free_array)(pr) };
    }
}

#[test]
fn init_array_matches_for_larger_capacities() {
    let (c, rust) = both();
    for cap in [100usize, 1023, 1024, 4096, 65_536, 1_000_000] {
        let pc = unsafe { (c.init_array)(cap) };
        let pr = unsafe { (rust.init_array)(cap) };
        assert_headers_match(&format!("init_array({cap})"), c, rust, pc, pr);
        unsafe { (c.free_array)(pc) };
        unsafe { (rust.free_array)(pr) };
    }
}

/// `initial_capacity * sizeof(int)` is a `size_t` multiply that can wrap, and
/// enormous sizes must fail identically (NULL) rather than panicking or
/// aborting on the Rust side.
#[test]
fn init_array_matches_for_overflowing_and_failing_capacities() {
    let (c, rust) = both();
    let caps: &[usize] = &[
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        1 << 63,
        (1 << 63) + 1,
        1 << 62,       // *4 wraps to exactly 0 bytes
        (1 << 62) + 1, // *4 wraps to 4 bytes
        (1 << 62) + 2, // *4 wraps to 8 bytes
        1 << 61,       // 2^63 bytes: rejected by Rust's Layout, malloc fails
        (1 << 61) + 1,
        1 << 60,
        1 << 50,
        1 << 40,
    ];
    for &cap in caps {
        let pc = unsafe { (c.init_array)(cap) };
        let pr = unsafe { (rust.init_array)(cap) };
        assert_headers_match(&format!("init_array({cap:#x})"), c, rust, pc, pr);
        unsafe { (c.free_array)(pc) };
        unsafe { (rust.free_array)(pr) };
    }
}

#[test]
fn null_arguments_match() {
    let (c, rust) = both();
    let n: *mut DynamicArray = ptr::null_mut();

    assert_eq!(
        unsafe { (c.expand_array)(n) },
        unsafe { (rust.expand_array)(n) },
        "expand_array(NULL)"
    );
    assert_eq!(unsafe { (c.expand_array)(n) }, 0, "expand_array(NULL) == 0");

    for &v in &[0, 1, -1, c_int::MAX, c_int::MIN] {
        assert_eq!(
            unsafe { (c.add_element)(n, v) },
            unsafe { (rust.add_element)(n, v) },
            "add_element(NULL, {v})"
        );
        assert_eq!(unsafe { (c.add_element)(n, v) }, 0, "add_element(NULL) == 0");
    }

    // Must be a no-op rather than a crash on either side.
    unsafe { (c.free_array)(n) };
    unsafe { (rust.free_array)(n) };
}

#[test]
fn expand_array_doubles_capacity_identically() {
    let (c, rust) = both();
    for start in 1..=17usize {
        let pc = unsafe { (c.init_array)(start) };
        let pr = unsafe { (rust.init_array)(start) };
        assert_headers_match("init before expand", c, rust, pc, pr);

        // Fill the initial capacity so growth has data to preserve.
        for i in 0..start {
            let v = (i as c_int) * 3 - 5;
            assert_eq!(
                unsafe { (c.add_element)(pc, v) },
                unsafe { (rust.add_element)(pr, v) },
                "prefill add_element"
            );
        }

        for round in 0..6 {
            let rc = unsafe { (c.expand_array)(pc) };
            let rr = unsafe { (rust.expand_array)(pr) };
            assert_eq!(rc, rr, "expand_array(start={start}, round={round}) return");
            assert_headers_match(
                &format!("expand_array(start={start}, round={round})"),
                c,
                rust,
                pc,
                pr,
            );
            let h = c.read_header(pc);
            assert_eq!(
                h.capacity,
                start << (round + 1),
                "capacity after {} expansions",
                round + 1
            );
            // Growth must not disturb the live elements.
            assert_eq!(
                c.read_data(pc, h.size),
                rust.read_data(pr, h.size),
                "data preserved across expand (start={start}, round={round})"
            );
        }

        unsafe { (c.free_array)(pc) };
        unsafe { (rust.free_array)(pr) };
    }
}

/// A zero capacity makes `new_capacity` zero as well, so `realloc(ptr, 0)` is
/// reached. The arrays are intentionally leaked: on the C side that `realloc`
/// has already released `data`, so a subsequent `free_array` would double-free.
#[test]
fn expand_array_matches_on_zero_capacity() {
    let (c, rust) = both();
    let pc = unsafe { (c.init_array)(0) };
    let pr = unsafe { (rust.init_array)(0) };
    assert_headers_match("init_array(0)", c, rust, pc, pr);

    let rc = unsafe { (c.expand_array)(pc) };
    let rr = unsafe { (rust.expand_array)(pr) };
    assert_eq!(rc, rr, "expand_array on capacity 0 return value");

    let hc = c.read_header(pc);
    let hr = rust.read_header(pr);
    assert_eq!(hc.size, hr.size, "size after failed expand");
    assert_eq!(hc.capacity, hr.capacity, "capacity after failed expand");
}

/// `add_element` on a full-at-zero array must take the same failing path.
///
/// Each value gets a fresh array: the failing `expand_array` routes through
/// `realloc(data, 0)`, which glibc implements as `free(data)`, so a second
/// attempt on the same array would double-free inside the C library.
#[test]
fn add_element_matches_on_zero_capacity() {
    let (c, rust) = both();

    for &v in &[7, -7, 0, c_int::MAX, c_int::MIN] {
        let pc = unsafe { (c.init_array)(0) };
        let pr = unsafe { (rust.init_array)(0) };

        let rc = unsafe { (c.add_element)(pc, v) };
        let rr = unsafe { (rust.add_element)(pr, v) };
        assert_eq!(rc, rr, "add_element({v}) on capacity 0");
        assert_eq!(rc, 0, "add_element on capacity 0 must fail");

        let hc = c.read_header(pc);
        let hr = rust.read_header(pr);
        assert_eq!(hc.size, hr.size, "size after add_element on capacity 0");
        assert_eq!(
            hc.capacity, hr.capacity,
            "capacity after add_element on capacity 0"
        );
        // Leaked deliberately: `data` is already freed on the C side.
    }
}

#[test]
fn add_element_growth_sequence_matches() {
    let (c, rust) = both();
    for start in 1..=8usize {
        let pc = unsafe { (c.init_array)(start) };
        let pr = unsafe { (rust.init_array)(start) };

        let values: Vec<c_int> = (0..200)
            .map(|i| match i % 5 {
                0 => i as c_int,
                1 => -(i as c_int),
                2 => c_int::MAX - i as c_int,
                3 => c_int::MIN + i as c_int,
                _ => (i as c_int) * 0x1000,
            })
            .collect();

        for (idx, &v) in values.iter().enumerate() {
            let rc = unsafe { (c.add_element)(pc, v) };
            let rr = unsafe { (rust.add_element)(pr, v) };
            assert_eq!(rc, rr, "add_element #{idx} (start={start}) return");

            let hc = c.read_header(pc);
            let hr = rust.read_header(pr);
            assert_eq!(hc.size, hr.size, "size after add #{idx} (start={start})");
            assert_eq!(
                hc.capacity, hr.capacity,
                "capacity after add #{idx} (start={start})"
            );
            assert_eq!(
                c.read_data(pc, hc.size),
                rust.read_data(pr, hr.size),
                "buffer after add #{idx} (start={start})"
            );
            assert_eq!(hc.size, idx + 1, "size should track the number of adds");
        }

        // The final buffer must be exactly the values pushed, in order.
        let h = c.read_header(pc);
        assert_eq!(c.read_data(pc, h.size), values);
        assert_eq!(rust.read_data(pr, h.size), values);

        unsafe { (c.free_array)(pc) };
        unsafe { (rust.free_array)(pr) };
    }
}

/// Mixing explicit `expand_array` calls into an `add_element` stream exercises
/// the "already has room" branch of `add_element`.
#[test]
fn interleaved_expand_and_add_matches() {
    let (c, rust) = both();
    let pc = unsafe { (c.init_array)(3) };
    let pr = unsafe { (rust.init_array)(3) };

    let mut expected: Vec<c_int> = Vec::new();
    for i in 0..60i32 {
        if i % 7 == 0 {
            assert_eq!(
                unsafe { (c.expand_array)(pc) },
                unsafe { (rust.expand_array)(pr) },
                "interleaved expand_array at i={i}"
            );
        }
        let v = i * i - 1000;
        assert_eq!(
            unsafe { (c.add_element)(pc, v) },
            unsafe { (rust.add_element)(pr, v) },
            "interleaved add_element at i={i}"
        );
        expected.push(v);

        let hc = c.read_header(pc);
        let hr = rust.read_header(pr);
        assert_eq!((hc.size, hc.capacity), (hr.size, hr.capacity), "state at i={i}");
        assert_eq!(c.read_data(pc, hc.size), expected, "C buffer at i={i}");
        assert_eq!(rust.read_data(pr, hr.size), expected, "Rust buffer at i={i}");
    }

    unsafe { (c.free_array)(pc) };
    unsafe { (rust.free_array)(pr) };
}

/// Allocating and freeing many arrays in a row would surface a layout mismatch
/// between the Rust `alloc`/`realloc`/`dealloc` calls as an abort.
#[test]
fn repeated_alloc_free_cycles_are_stable() {
    let (c, rust) = both();
    for cap in 1..=32usize {
        for _ in 0..8 {
            let pc = unsafe { (c.init_array)(cap) };
            let pr = unsafe { (rust.init_array)(cap) };
            for i in 0..(cap * 3) {
                assert_eq!(
                    unsafe { (c.add_element)(pc, i as c_int) },
                    unsafe { (rust.add_element)(pr, i as c_int) },
                );
            }
            let h = c.read_header(pc);
            assert_eq!(c.read_data(pc, h.size), rust.read_data(pr, h.size));
            unsafe { (c.free_array)(pc) };
            unsafe { (rust.free_array)(pr) };
        }
    }
}
