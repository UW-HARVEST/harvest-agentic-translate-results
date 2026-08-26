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
        let c_path = manifest_dir.join("c_src/build/libSimpleList.so");
        let rust_path = rust_library_path();

        assert!(
            c_path.is_file(),
            "C shared library does not exist at {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library does not exist at {}",
            rust_path.display()
        );

        // SAFETY: Both paths name shared libraries built from this workspace.
        let c_library = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        // SAFETY: Both paths name shared libraries built from this workspace.
        let rust_library = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

        // SAFETY: Phase A established this symbol and signature from the C header.
        let c_smallest_value =
            unsafe { *c_library.get::<SmallestValue>(b"smallestValue\0").unwrap() };
        // SAFETY: The Rust cdylib must expose the same C ABI symbol.
        let rust_smallest_value = unsafe {
            *rust_library
                .get::<SmallestValue>(b"smallestValue\0")
                .unwrap()
        };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_smallest_value,
            rust_smallest_value,
        }
    }

    fn compare(&self, values: &[c_int]) -> c_int {
        let mut list = List::new(values);
        let head = list.head();

        // SAFETY: List owns a valid, null-terminated C-layout linked list.
        let c_result = unsafe { (self.c_smallest_value)(head) };
        // SAFETY: The Rust export receives the same valid list through its C ABI.
        let rust_result = unsafe { (self.rust_smallest_value)(head) };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "result mismatch for values {values:?}: C={c_result}, Rust={rust_result}"
        );
        c_result
    }

    fn compare_null(&self) -> (c_int, c_int) {
        // SAFETY: Null is an explicitly handled input in both implementations.
        let c_result = unsafe { (self.c_smallest_value)(ptr::null_mut()) };
        // SAFETY: Null is an explicitly handled input in both implementations.
        let rust_result = unsafe { (self.rust_smallest_value)(ptr::null_mut()) };
        (c_result, rust_result)
    }
}

struct List {
    nodes: Vec<ListNode>,
}

impl List {
    fn new(values: &[c_int]) -> Self {
        assert!(!values.is_empty());
        let mut nodes: Vec<_> = values
            .iter()
            .map(|&value| ListNode {
                value,
                next: ptr::null_mut(),
            })
            .collect();

        let base = nodes.as_mut_ptr();
        for index in 0..nodes.len() - 1 {
            // SAFETY: The vector has its final capacity and index + 1 is in bounds.
            nodes[index].next = unsafe { base.add(index + 1) };
        }

        Self { nodes }
    }

    fn head(&mut self) -> *mut ListNode {
        self.nodes.as_mut_ptr()
    }
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn length(&mut self, minimum: usize, span: usize) -> usize {
        minimum + self.next_u64() as usize % span
    }
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("test executable path");
    let deps_dir = test_executable.parent().expect("test executable directory");
    let candidates = [
        deps_dir.join("libSimpleList.so"),
        deps_dir
            .parent()
            .expect("Cargo profile directory")
            .join("libSimpleList.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib not found beside test executable or in profile directory: {}",
                test_executable.display()
            )
        })
}

#[test]
fn config_1_single_node_all_int_values() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(0x243f_6a88_85a3_08d3);

    for value in [i32::MIN, -1, 0, 1, i32::MAX] {
        assert_eq!(apis.compare(&[value]), value);
    }
    for _ in 0..512 {
        let value = rng.next_i32();
        assert_eq!(apis.compare(&[value]), value);
    }
}

#[test]
fn config_2_multi_node_without_minimum_updates() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(0x1319_8a2e_0370_7344);

    for values in [
        vec![i32::MIN, i32::MIN, 0, i32::MAX],
        vec![i32::MAX, i32::MAX],
        vec![-1, -1, 0, 1],
    ] {
        assert_eq!(apis.compare(&values), values[0]);
    }

    for _ in 0..512 {
        let first = rng.next_i32();
        let length = rng.length(2, 31);
        let mut values = Vec::with_capacity(length);
        values.push(first);
        for index in 1..length {
            let candidate = rng.next_i32();
            values.push(if index % 4 == 0 || candidate < first {
                first
            } else {
                candidate
            });
        }
        assert_eq!(apis.compare(&values), first);
    }
}

#[test]
fn config_3_strictly_descending_updates_every_iteration() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(0xa409_3822_299f_31d0);

    for values in [
        vec![i32::MAX, i32::MIN],
        vec![1, 0, -1],
        vec![i32::MIN + 1, i32::MIN],
    ] {
        assert_eq!(apis.compare(&values), *values.last().unwrap());
    }

    for case in 0..512 {
        let length = rng.length(2, 31);
        let mut values: Vec<_> = (0..length).map(|_| rng.next_i32()).collect();
        if case % 16 == 0 {
            values.extend([i32::MIN, i32::MAX]);
        }
        values.sort_unstable_by(|left, right| right.cmp(left));
        values.dedup();
        if values.len() < 2 {
            values = vec![i32::MAX, i32::MIN];
        }

        assert!(
            values.windows(2).all(|pair| pair[1] < pair[0]),
            "generator must produce a strictly descending list"
        );
        assert_eq!(apis.compare(&values), *values.last().unwrap());
    }
}

#[test]
fn config_4_mixed_comparison_outcomes() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(0x082e_fa98_ec4e_6c89);

    for values in [
        vec![i32::MAX, i32::MIN, i32::MIN, i32::MAX],
        vec![i32::MAX, i32::MAX, i32::MIN],
        vec![0, 1, -1, -1, i32::MAX],
    ] {
        assert_eq!(apis.compare(&values), *values.iter().min().unwrap());
    }

    for case in 0..512 {
        let first_random = rng.next_i32();
        let second_random = rng.next_i32();
        let (low, high) = if first_random == second_random {
            (i32::MIN, i32::MAX)
        } else {
            (
                first_random.min(second_random),
                first_random.max(second_random),
            )
        };
        let mut values = if case % 2 == 0 {
            vec![high, low, low]
        } else {
            vec![high, high, low]
        };
        let tail_length = rng.length(0, 30);
        values.extend((0..tail_length).map(|_| rng.next_i32()));

        assert_eq!(apis.compare(&values), *values.iter().min().unwrap());
    }
}

#[test]
fn error_1_null_head_returns_exact_sentinel() {
    let apis = Apis::load();
    let (c_result, rust_result) = apis.compare_null();

    assert_eq!(c_result, -1);
    assert_eq!(rust_result.to_ne_bytes(), c_result.to_ne_bytes());
}
