use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::ptr;

#[repr(C)]
struct ListNode {
    value: c_int,
    next: *mut ListNode,
}

type SmallestValue = unsafe extern "C" fn(*mut ListNode) -> c_int;

struct APIs {
    c_smallest_value: SmallestValue,
    rust_smallest_value: SmallestValue,
    _c_library: Library,
    _rust_library: Library,
}

impl APIs {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libSimpleList.so");
        let rust_path = manifest.join("target/release/libSimpleList.so");

        assert_library_exists(&c_path);
        assert_library_exists(&rust_path);

        // SAFETY: These paths name shared libraries built from this workspace.
        let c_library = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        // SAFETY: These paths name shared libraries built from this workspace.
        let rust_library = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

        // Copy the function pointers while retaining both Library handles for
        // at least as long as the pointers can be called.
        let c_smallest_value = unsafe {
            *c_library
                .get::<SmallestValue>(b"smallestValue\0")
                .expect("C library does not export smallestValue")
        };
        let rust_smallest_value = unsafe {
            *rust_library
                .get::<SmallestValue>(b"smallestValue\0")
                .expect("Rust library does not export smallestValue")
        };

        Self {
            c_smallest_value,
            rust_smallest_value,
            _c_library: c_library,
            _rust_library: rust_library,
        }
    }

    fn compare_values(&self, values: &[c_int]) {
        assert!(!values.is_empty());
        let mut c_list = make_list(values);
        let mut rust_list = make_list(values);

        // SAFETY: make_list creates valid, readable, null-terminated C-layout
        // linked lists that live for the duration of both calls.
        let c_result = unsafe { (self.c_smallest_value)(c_list.as_mut_ptr()) };
        // SAFETY: Same reasoning as for the C call above.
        let rust_result = unsafe { (self.rust_smallest_value)(rust_list.as_mut_ptr()) };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "differential mismatch for input {values:?}: C={c_result}, Rust={rust_result}"
        );
    }

    fn compare_null(&self) {
        // SAFETY: NULL is an explicitly supported rejection input in the C API.
        let c_result = unsafe { (self.c_smallest_value)(ptr::null_mut()) };
        // SAFETY: NULL is an explicitly supported rejection input in the Rust API.
        let rust_result = unsafe { (self.rust_smallest_value)(ptr::null_mut()) };

        assert_eq!(c_result, -1, "C NULL sentinel changed");
        assert_eq!(rust_result.to_ne_bytes(), c_result.to_ne_bytes());
    }
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "shared library does not exist: {}; build both libraries first",
        path.display()
    );
}

fn make_list(values: &[c_int]) -> Box<[ListNode]> {
    let mut nodes: Box<[ListNode]> = values
        .iter()
        .copied()
        .map(|value| ListNode {
            value,
            next: ptr::null_mut(),
        })
        .collect();

    let base = nodes.as_mut_ptr();
    for index in 0..nodes.len().saturating_sub(1) {
        // SAFETY: index + 1 is within the allocated boxed slice, and the box
        // keeps every node at a stable address.
        nodes[index].next = unsafe { base.add(index + 1) };
    }
    nodes
}

#[derive(Clone, Copy)]
struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    fn usize_in(&mut self, start: usize, end: usize) -> usize {
        assert!(start < end);
        start + (self.next_u64() as usize % (end - start))
    }

    fn i32_in(&mut self, start: i32, end: i32) -> i32 {
        assert!(start < end);
        start + (self.next_u64() % (end - start) as u64) as i32
    }
}

#[test]
fn config_01_singleton_randomized_ordinary_values() {
    let apis = APIs::load();
    let mut rng = FixedRng::new(0x15ad_5eed_0000_0001);
    for _ in 0..2_000 {
        apis.compare_values(&[rng.next_i32()]);
    }
}

#[test]
fn config_02_singleton_int_boundaries() {
    let apis = APIs::load();
    for value in [i32::MIN, i32::MAX] {
        apis.compare_values(&[value]);
    }
}

#[test]
fn config_03_two_nodes_without_minimum_update() {
    let apis = APIs::load();
    let mut rng = FixedRng::new(0x15ad_5eed_0000_0003);
    for case in 0..2_000 {
        let first = rng.next_i32();
        let second = if case % 10 == 0 {
            first
        } else {
            rng.next_i32()
        };
        let head = first.min(second);
        let tail = first.max(second);
        apis.compare_values(&[head, tail]);
    }
}

#[test]
fn config_04_two_nodes_with_minimum_update() {
    let apis = APIs::load();
    let mut rng = FixedRng::new(0x15ad_5eed_0000_0004);
    for _ in 0..2_000 {
        let first = rng.next_i32();
        let mut second = rng.next_i32();
        while second == first {
            second = rng.next_i32();
        }
        apis.compare_values(&[first.max(second), first.min(second)]);
    }
}

#[test]
fn config_05_many_nodes_all_comparisons_false() {
    let apis = APIs::load();
    let mut rng = FixedRng::new(0x15ad_5eed_0000_0005);
    for case in 0..500 {
        let length = rng.usize_in(3, 257);
        let mut values: Vec<i32> = (0..length).map(|_| rng.next_i32()).collect();
        if case % 2 == 0 {
            values[1] = values[0];
        }
        values.sort_unstable();
        apis.compare_values(&values);
    }
}

#[test]
fn config_06_many_nodes_all_comparisons_true() {
    let apis = APIs::load();
    let mut rng = FixedRng::new(0x15ad_5eed_0000_0006);
    for _ in 0..500 {
        let length = rng.usize_in(3, 257);
        let mut current = i32::MAX - rng.i32_in(0, 1_000_000);
        let mut values = Vec::with_capacity(length);
        values.push(current);
        for _ in 1..length {
            current -= rng.i32_in(1, 1_025);
            values.push(current);
        }
        apis.compare_values(&values);
    }
}

#[test]
fn config_07_many_nodes_mixed_outcomes_and_duplicate_minima() {
    let apis = APIs::load();
    let mut rng = FixedRng::new(0x15ad_5eed_0000_0007);
    for _ in 0..1_000 {
        let base = rng.i32_in(-1_000_000, 1_000_001);
        let mut values = vec![base, base - 10, base + 20, base - 10, base - 30, base - 30];
        let extra = rng.usize_in(1, 129);
        values.extend((0..extra).map(|_| base + rng.i32_in(-100, 101)));
        apis.compare_values(&values);
    }
}

#[test]
fn config_08_many_nodes_with_int_boundaries() {
    let apis = APIs::load();
    let mut rng = FixedRng::new(0x15ad_5eed_0000_0008);
    for _ in 0..500 {
        let length = rng.usize_in(3, 2_049);
        let mut values: Vec<i32> = (0..length - 2).map(|_| rng.next_i32()).collect();
        values.push(i32::MIN);
        values.push(i32::MAX);
        for index in (1..values.len()).rev() {
            let swap_with = rng.usize_in(0, index + 1);
            values.swap(index, swap_with);
        }
        apis.compare_values(&values);
    }
}

#[test]
fn error_01_null_head_returns_exact_sentinel() {
    APIs::load().compare_null();
}
