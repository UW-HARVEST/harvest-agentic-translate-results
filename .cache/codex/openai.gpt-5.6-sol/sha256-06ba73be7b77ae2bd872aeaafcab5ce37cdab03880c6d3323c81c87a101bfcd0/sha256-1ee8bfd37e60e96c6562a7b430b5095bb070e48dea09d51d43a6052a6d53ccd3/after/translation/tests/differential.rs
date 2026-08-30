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

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_smallest_value: SmallestValue,
    rust_smallest_value: SmallestValue,
}

impl Apis {
    fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("../c_src/build/libSimpleList.so");
        let rust_path = manifest_dir
            .join("target")
            .join("release")
            .join("libSimpleList.so");

        assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {rust_path:?}"
        );

        // SAFETY: These paths refer to the two libraries under differential test.
        let c_library = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("failed to load {c_path:?}: {error}"));
        // SAFETY: These paths refer to the two libraries under differential test.
        let rust_library = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("failed to load {rust_path:?}: {error}"));

        // Copy function pointers while retaining both libraries for the lifetime
        // of the test API object.
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
            _c_library: c_library,
            _rust_library: rust_library,
            c_smallest_value,
            rust_smallest_value,
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn usize_in(&mut self, start: usize, end: usize) -> usize {
        assert!(start < end);
        start + self.next_u32() as usize % (end - start)
    }

    fn i32_in(&mut self, start: i32, end: i32) -> i32 {
        assert!(start < end);
        start + (self.next_u32() % (end - start) as u32) as i32
    }
}

fn make_list(values: &[i32]) -> Vec<ListNode> {
    let mut nodes: Vec<_> = values
        .iter()
        .copied()
        .map(|value| ListNode {
            value,
            next: ptr::null_mut(),
        })
        .collect();

    let base = nodes.as_mut_ptr();
    for index in 0..nodes.len().saturating_sub(1) {
        // SAFETY: The vector has its final capacity and index + 1 is in bounds.
        nodes[index].next = unsafe { base.add(index + 1) };
    }
    nodes
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        manifest_dir.join("../c_src/build/libSimpleList.so"),
        manifest_dir
            .join("target")
            .join("release")
            .join("libSimpleList.so"),
    )
}

fn assert_list_match(apis: &Apis, values: &[i32]) {
    assert!(!values.is_empty());
    let mut c_nodes = make_list(values);
    let mut rust_nodes = make_list(values);

    // SAFETY: Both lists are readable, non-empty, and null-terminated.
    let c_result = unsafe { (apis.c_smallest_value)(c_nodes.as_mut_ptr()) };
    // SAFETY: Both lists are readable, non-empty, and null-terminated.
    let rust_result = unsafe { (apis.rust_smallest_value)(rust_nodes.as_mut_ptr()) };

    assert_eq!(
        c_result.to_ne_bytes(),
        rust_result.to_ne_bytes(),
        "C/Rust result mismatch for values {values:?}"
    );
    assert_eq!(c_result, *values.iter().min().unwrap());
}

#[test]
fn shared_objects_are_loaded_from_expected_paths() {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {rust_path:?}"
    );
    let _apis = Apis::load();
}

#[test]
fn config_01_singleton_lists() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x7a65_5f01_c031_9d8b);

    for _ in 0..1_024 {
        assert_list_match(&apis, &[rng.next_i32()]);
    }
}

#[test]
fn config_02_multi_node_without_minimum_updates() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x7a65_5f02_c031_9d8b);

    for _ in 0..512 {
        let length = rng.usize_in(2, 65);
        let mut current = rng.i32_in(-1_000_000_000, 0);
        let mut values = Vec::with_capacity(length);
        values.push(current);
        for _ in 1..length {
            current += rng.i32_in(1, 10_001);
            values.push(current);
        }
        assert_list_match(&apis, &values);
    }
}

#[test]
fn config_03_multi_node_equal_values() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x7a65_5f03_c031_9d8b);

    for _ in 0..512 {
        let length = rng.usize_in(2, 65);
        let value = rng.next_i32();
        assert_list_match(&apis, &vec![value; length]);
    }
}

#[test]
fn config_04_multi_node_with_minimum_updates() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x7a65_5f04_c031_9d8b);

    for _ in 0..1_024 {
        let length = rng.usize_in(3, 65);
        let first = rng.i32_in(-500_000_000, 500_000_001);
        let forced_update_index = rng.usize_in(1, length);
        let forced_drop = rng.i32_in(1, 1_000_001);
        let mut values: Vec<_> = (0..length)
            .map(|_| rng.i32_in(-1_000_000_000, 1_000_000_001))
            .collect();
        values[0] = first;
        values[forced_update_index] = first - forced_drop;

        assert!(values.iter().skip(1).any(|value| *value < first));
        assert_list_match(&apis, &values);
    }
}

#[test]
fn config_05_c_int_boundaries() {
    let apis = Apis::load();
    let mut rng = Rng::new(0x7a65_5f05_c031_9d8b);

    for values in [
        vec![i32::MIN],
        vec![i32::MAX],
        vec![i32::MIN, i32::MAX],
        vec![i32::MAX, i32::MIN],
        vec![i32::MAX, 0, i32::MIN],
        vec![i32::MIN, 0, i32::MAX],
        vec![i32::MIN, i32::MIN],
        vec![i32::MAX, i32::MAX],
    ] {
        assert_list_match(&apis, &values);
    }

    for iteration in 0..512 {
        let length = rng.usize_in(2, 65);
        let mut values: Vec<_> = (0..length).map(|_| rng.next_i32()).collect();
        let minimum_index = rng.usize_in(0, length);
        values[minimum_index] = i32::MIN;
        if iteration % 2 == 0 {
            let candidate = rng.usize_in(0, length);
            let maximum_index = if candidate == minimum_index {
                (candidate + 1) % length
            } else {
                candidate
            };
            values[maximum_index] = i32::MAX;
        }
        assert_list_match(&apis, &values);
    }
}

#[test]
fn error_01_null_head_returns_minus_one() {
    let apis = Apis::load();

    // SAFETY: A null head is an explicitly handled input in both implementations.
    let c_result = unsafe { (apis.c_smallest_value)(ptr::null_mut()) };
    // SAFETY: A null head is an explicitly handled input in both implementations.
    let rust_result = unsafe { (apis.rust_smallest_value)(ptr::null_mut()) };

    assert_eq!(c_result.to_ne_bytes(), rust_result.to_ne_bytes());
    assert_eq!(c_result, -1);
}
