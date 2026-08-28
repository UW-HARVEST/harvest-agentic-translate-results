use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_double, c_int};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

const RANDOM_CASES: usize = 24;
const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;

type AddNode = unsafe extern "C" fn(c_int, c_int, *const c_char, c_double) -> c_int;
type FindNode = unsafe extern "C" fn(c_int) -> *mut Node;
type GetChildrenCount = unsafe extern "C" fn(c_int) -> c_int;
type CalculateSubtreeSum = unsafe extern "C" fn(c_int) -> c_double;
type ProcessString = unsafe extern "C" fn(*mut c_char) -> c_int;
type SafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type Maxnmin = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Node {
    id: c_int,
    parent_id: c_int,
    name: [c_char; MAX_NAME_LEN],
    value: c_double,
    active: c_int,
}

struct Pair {
    c: Library,
    rust: Library,
}

impl Pair {
    fn new() -> Self {
        static NEXT_COPY: AtomicU64 = AtomicU64::new(0);

        fn load_copy(source: &Path, tag: &str, serial: u64) -> Library {
            let destination = env::temp_dir().join(format!(
                "maxnmin-differential-{}-{serial}-{tag}.so",
                std::process::id()
            ));
            fs::copy(source, &destination).unwrap_or_else(|error| {
                panic!(
                    "failed to copy {} to {}: {error}",
                    source.display(),
                    destination.display()
                )
            });
            let library = unsafe { Library::new(&destination) }.unwrap_or_else(|error| {
                panic!("failed to load {}: {error}", destination.display())
            });
            fs::remove_file(&destination).expect("failed to unlink temporary shared object");
            library
        }

        let serial = NEXT_COPY.fetch_add(1, Ordering::Relaxed);
        Self {
            c: load_copy(&c_library_path(), "c", serial),
            rust: load_copy(&rust_library_path(), "rust", serial),
        }
    }

    unsafe fn c_symbol<T: Copy>(&self, name: &[u8]) -> T {
        unsafe { *self.c.get::<T>(name).expect("missing C symbol") }
    }

    unsafe fn rust_symbol<T: Copy>(&self, name: &[u8]) -> T {
        unsafe { *self.rust.get::<T>(name).expect("missing Rust symbol") }
    }

    fn add(&self, id: i32, parent_id: i32, name: &CStr, value: f64) -> (i32, i32) {
        let c_add = unsafe { self.c_symbol::<AddNode>(b"add_node\0") };
        let rust_add = unsafe { self.rust_symbol::<AddNode>(b"add_node\0") };
        unsafe {
            (
                c_add(id, parent_id, name.as_ptr(), value),
                rust_add(id, parent_id, name.as_ptr(), value),
            )
        }
    }

    fn find(&self, id: i32) -> (*mut Node, *mut Node) {
        let c_find = unsafe { self.c_symbol::<FindNode>(b"find_node_by_id\0") };
        let rust_find = unsafe { self.rust_symbol::<FindNode>(b"find_node_by_id\0") };
        unsafe { (c_find(id), rust_find(id)) }
    }

    fn children(&self, parent_id: i32) -> (i32, i32) {
        let c_count = unsafe { self.c_symbol::<GetChildrenCount>(b"get_children_count\0") };
        let rust_count = unsafe { self.rust_symbol::<GetChildrenCount>(b"get_children_count\0") };
        unsafe { (c_count(parent_id), rust_count(parent_id)) }
    }

    fn subtree(&self, node_id: i32) -> (f64, f64) {
        let c_sum = unsafe { self.c_symbol::<CalculateSubtreeSum>(b"calculate_subtree_sum\0") };
        let rust_sum =
            unsafe { self.rust_symbol::<CalculateSubtreeSum>(b"calculate_subtree_sum\0") };
        unsafe { (c_sum(node_id), rust_sum(node_id)) }
    }
}

#[derive(Clone, Copy)]
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

    fn finite_f64(&mut self) -> f64 {
        (self.next_i32() % 1_000_000) as f64 / 37.0
    }

    fn ascii_string(&mut self, length: usize) -> CString {
        let bytes = (0..length)
            .map(|_| (b'!' + (self.next_u32() % 94) as u8) as u8)
            .collect::<Vec<_>>();
        CString::new(bytes).expect("generated string contains NUL")
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_root()
        .join("../c_src/build")
        .join("libharvest-work-RhN4vv.so")
}

fn rust_library_path() -> PathBuf {
    crate_root().join("target/release/libmaxnmin_lib.so")
}

fn assert_float_bits_equal(c: f64, rust: f64, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?}, Rust={rust:?}"
    );
}

unsafe fn assert_nodes_equal(c: *mut Node, rust: *mut Node, context: &str) {
    assert_eq!(c.is_null(), rust.is_null(), "{context}: pointer sentinel");
    if c.is_null() {
        return;
    }

    let c_node = unsafe { *c };
    let rust_node = unsafe { *rust };
    assert_eq!(c_node.id, rust_node.id, "{context}: id");
    assert_eq!(
        c_node.parent_id, rust_node.parent_id,
        "{context}: parent_id"
    );
    assert_eq!(c_node.name, rust_node.name, "{context}: name bytes");
    assert_eq!(
        c_node.value.to_bits(),
        rust_node.value.to_bits(),
        "{context}: value bits"
    );
    assert_eq!(c_node.active, rust_node.active, "{context}: active");
}

fn add_and_compare(pair: &Pair, id: i32, parent_id: i32, name: &CStr, value: f64) {
    let indexes = pair.add(id, parent_id, name, value);
    assert_eq!(indexes.0, indexes.1, "add_node return");
    let nodes = pair.find(id);
    unsafe { assert_nodes_equal(nodes.0, nodes.1, "stored node") };
}

#[test]
fn symbols_all_c_exports_load_from_both_libraries() {
    let pair = Pair::new();
    for name in [
        b"add_node\0".as_slice(),
        b"calculate_subtree_sum\0",
        b"find_node_by_id\0",
        b"get_children_count\0",
        b"maxnmin\0",
        b"process_string\0",
        b"safe_double_to_int\0",
    ] {
        unsafe {
            pair.c.get::<*const ()>(name).expect("missing C export");
            pair.rust
                .get::<*const ()>(name)
                .expect("missing Rust export");
        }
    }
}

#[test]
fn configs_c01_through_c05_add_node_shapes() {
    let mut rng = Rng::new(0xc01c_05ad_d00d);

    for _ in 0..RANDOM_CASES {
        let pair = Pair::new();
        add_and_compare(&pair, rng.next_i32(), rng.next_i32(), c"", rng.finite_f64());
    }

    let pair = Pair::new();
    for length in 1..=48 {
        add_and_compare(
            &pair,
            1_000 + length as i32,
            rng.next_i32(),
            &rng.ascii_string(length),
            rng.finite_f64(),
        );
    }

    let pair = Pair::new();
    for case in 0..RANDOM_CASES {
        add_and_compare(
            &pair,
            2_000 + case as i32,
            rng.next_i32(),
            &rng.ascii_string(49),
            rng.finite_f64(),
        );
    }

    let pair = Pair::new();
    for case in 0..RANDOM_CASES {
        let length = 50 + (rng.next_u32() % 151) as usize;
        add_and_compare(
            &pair,
            3_000 + case as i32,
            rng.next_i32(),
            &rng.ascii_string(length),
            rng.finite_f64(),
        );
    }

    let pair = Pair::new();
    let special_values = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        f64::MIN_POSITIVE,
        f64::MAX,
    ];
    for case in 0..RANDOM_CASES {
        let name_length = (rng.next_u32() % 70) as usize;
        add_and_compare(
            &pair,
            rng.next_i32(),
            rng.next_i32(),
            &rng.ascii_string(name_length),
            special_values[case % special_values.len()],
        );
    }
}

#[test]
fn configs_c06_through_c09_find_node_shapes() {
    let mut rng = Rng::new(0xc06c_09f1_1d00);

    let pair = Pair::new();
    for _ in 0..RANDOM_CASES {
        let nodes = pair.find(rng.next_i32());
        unsafe { assert_nodes_equal(nodes.0, nodes.1, "empty lookup") };
        assert!(nodes.0.is_null());
    }

    for _case in 0..RANDOM_CASES {
        let pair = Pair::new();
        let id = rng.next_i32();
        add_and_compare(
            &pair,
            id,
            rng.next_i32(),
            &rng.ascii_string(12),
            rng.finite_f64(),
        );
        let nodes = pair.find(id);
        unsafe { assert_nodes_equal(nodes.0, nodes.1, "one active match") };
        assert!(!nodes.0.is_null());

        let first_value = rng.finite_f64();
        let second_value = rng.finite_f64();
        let pair = Pair::new();
        assert_eq!(pair.add(id, 1, c"first", first_value), (0, 0));
        assert_eq!(pair.add(id, 2, c"second", second_value), (1, 1));
        let nodes = pair.find(id);
        unsafe {
            assert_nodes_equal(nodes.0, nodes.1, "first duplicate");
            assert_eq!((*nodes.0).value.to_bits(), first_value.to_bits());
            (*nodes.0).active = 0;
            (*nodes.1).active = 0;
        }
        let later_nodes = pair.find(id);
        unsafe {
            assert_nodes_equal(later_nodes.0, later_nodes.1, "later active duplicate");
            assert_eq!((*later_nodes.0).value.to_bits(), second_value.to_bits());
        }
    }
}

#[test]
fn configs_c10_through_c12_children_count_shapes() {
    let mut rng = Rng::new(0xc10c_12c0_017d);

    let pair = Pair::new();
    for _ in 0..RANDOM_CASES {
        let counts = pair.children(rng.next_i32());
        assert_eq!(counts, (0, 0));
    }

    for case in 0..RANDOM_CASES {
        let pair = Pair::new();
        let parent = rng.next_i32();
        let other_parent = parent.wrapping_add(1);
        assert_eq!(
            pair.add(case as i32, other_parent, c"other", rng.finite_f64()),
            (0, 0)
        );
        assert_eq!(pair.children(parent), (0, 0));
        assert_eq!(
            pair.add(10_000 + case as i32, parent, c"one", rng.finite_f64()),
            (1, 1)
        );
        assert_eq!(pair.children(parent), (1, 1));
    }

    let pair = Pair::new();
    let parent = rng.next_i32();
    let mut expected = 0;
    for case in 0..48 {
        let id = 20_000 + case;
        assert_eq!(
            pair.add(id, parent, &rng.ascii_string(8), rng.finite_f64()),
            (case, case)
        );
        let nodes = pair.find(id);
        if case % 3 == 0 {
            unsafe {
                (*nodes.0).active = 0;
                (*nodes.1).active = 0;
            }
        } else {
            expected += 1;
        }
        assert_eq!(pair.children(parent), (expected, expected));
    }
}

#[test]
fn configs_c13_through_c17_subtree_shapes() {
    let mut rng = Rng::new(0xc13c_1750_b7ee);

    for case in 0..RANDOM_CASES {
        let root = 30_000 + case as i32 * 10;
        let root_value = rng.finite_f64();

        let pair = Pair::new();
        assert_eq!(pair.add(root, -1, c"leaf", root_value), (0, 0));
        let sums = pair.subtree(root);
        assert_float_bits_equal(sums.0, sums.1, "active leaf");

        let pair = Pair::new();
        assert_eq!(pair.add(root, -1, c"root", root_value), (0, 0));
        assert_eq!(pair.add(root + 1, root, c"child", rng.finite_f64()), (1, 1));
        let sums = pair.subtree(root);
        assert_float_bits_equal(sums.0, sums.1, "one-level tree");

        let pair = Pair::new();
        assert_eq!(pair.add(root, -1, c"root", root_value), (0, 0));
        assert_eq!(pair.add(root + 1, root, c"a", rng.finite_f64()), (1, 1));
        assert_eq!(pair.add(root + 2, root, c"b", rng.finite_f64()), (2, 2));
        assert_eq!(pair.add(root + 3, root + 1, c"c", rng.finite_f64()), (3, 3));
        assert_eq!(pair.add(root + 4, root + 3, c"d", rng.finite_f64()), (4, 4));
        let sums = pair.subtree(root);
        assert_float_bits_equal(sums.0, sums.1, "multi-level tree");

        let inactive = pair.find(root + 3);
        unsafe {
            (*inactive.0).active = 0;
            (*inactive.1).active = 0;
        }
        let sums = pair.subtree(root);
        assert_float_bits_equal(sums.0, sums.1, "inactive descendant");
    }

    for special in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for case in 0..RANDOM_CASES {
            let pair = Pair::new();
            let root = 40_000 + case as i32 * 2;
            assert_eq!(pair.add(root, -1, c"root", rng.finite_f64()), (0, 0));
            assert_eq!(pair.add(root + 1, root, c"special", special), (1, 1));
            let sums = pair.subtree(root);
            assert_float_bits_equal(sums.0, sums.1, "non-finite subtree");
        }
    }
}

#[test]
fn configs_c18_through_c21_process_string_shapes() {
    let pair = Pair::new();
    let c_process = unsafe { pair.c_symbol::<ProcessString>(b"process_string\0") };
    let rust_process = unsafe { pair.rust_symbol::<ProcessString>(b"process_string\0") };
    let mut rng = Rng::new(0xc18c_21c5_7a1f);

    for _ in 0..RANDOM_CASES {
        let mut empty = vec![0_i8];
        unsafe {
            assert_eq!(
                c_process(empty.as_mut_ptr()),
                rust_process(empty.as_mut_ptr())
            );
        }
    }

    for _ in 0..RANDOM_CASES {
        let mut one = vec![(1 + rng.next_u32() % 127) as c_char, 0];
        unsafe {
            assert_eq!(c_process(one.as_mut_ptr()), rust_process(one.as_mut_ptr()));
        }
    }

    for _ in 0..RANDOM_CASES {
        let length = 2 + (rng.next_u32() % 200) as usize;
        let nul_at = 1 + (rng.next_u32() as usize % (length - 1));
        let mut bytes = (0..length)
            .map(|_| (1 + rng.next_u32() % 127) as c_char)
            .collect::<Vec<_>>();
        bytes[nul_at] = 0;
        bytes.push(0);
        unsafe {
            assert_eq!(
                c_process(bytes.as_mut_ptr()),
                rust_process(bytes.as_mut_ptr())
            );
        }
    }

    for _ in 0..RANDOM_CASES {
        let length = 1 + (rng.next_u32() % 100) as usize;
        let mut bytes = (0..length)
            .map(|_| (0x80 + rng.next_u32() % 0x80) as u8 as c_char)
            .collect::<Vec<_>>();
        bytes.push(0);
        unsafe {
            assert_eq!(
                c_process(bytes.as_mut_ptr()),
                rust_process(bytes.as_mut_ptr())
            );
        }
    }
}

#[test]
fn configs_c22_c23_and_errors_e04_through_e06_safe_conversion() {
    let pair = Pair::new();
    let c_convert = unsafe { pair.c_symbol::<SafeDoubleToInt>(b"safe_double_to_int\0") };
    let rust_convert = unsafe { pair.rust_symbol::<SafeDoubleToInt>(b"safe_double_to_int\0") };
    let mut rng = Rng::new(0xc22e_06c0_11ab);

    let exact_values = [i32::MIN as f64, -1.0, -0.0, 0.0, 1.0, i32::MAX as f64];
    for value in exact_values {
        unsafe { assert_eq!(c_convert(value), rust_convert(value), "value={value:?}") };
    }

    for _ in 0..RANDOM_CASES * 4 {
        let value = rng.next_i32() as f64;
        unsafe { assert_eq!(c_convert(value), rust_convert(value), "value={value:?}") };
    }

    for _ in 0..RANDOM_CASES * 4 {
        let integer = (rng.next_i32() % 2_000_000) as f64;
        let fraction = (1 + rng.next_u32() % 999) as f64 / 1_000.0;
        for value in [integer + fraction, integer - fraction] {
            unsafe { assert_eq!(c_convert(value), rust_convert(value), "value={value:?}") };
        }
    }

    for case in 0..RANDOM_CASES {
        let offset = 1.0 + case as f64 * 1_000_003.0;
        for value in [
            i32::MAX as f64 + offset,
            f64::INFINITY,
            i32::MIN as f64 - offset,
            f64::NEG_INFINITY,
            f64::from_bits(0x7ff8_0000_0000_0000 | case as u64),
            f64::from_bits(0xfff8_0000_0000_0000 | case as u64),
        ] {
            unsafe { assert_eq!(c_convert(value), rust_convert(value), "value={value:?}") };
        }
    }
}

#[test]
fn configs_c24_through_c31_maxnmin_cross_product() {
    let pair = Pair::new();
    let c_maxnmin = unsafe { pair.c_symbol::<Maxnmin>(b"maxnmin\0") };
    let rust_maxnmin = unsafe { pair.rust_symbol::<Maxnmin>(b"maxnmin\0") };
    let mut rng = Rng::new(0xc24c_31f0_4d1a);

    let compare = |a: i32, b: i32, c: i32, d: i32, label: &str| unsafe {
        assert_eq!(
            c_maxnmin(a, b, c, d),
            rust_maxnmin(a, b, c, d),
            "{label}: ({a}, {b}, {c}, {d})"
        );
    };

    for selector in 0..6 {
        for _ in 0..RANDOM_CASES {
            compare(
                selector + 6 * (rng.next_u32() % 10_000) as i32,
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                "param1 node selector",
            );
            compare(
                rng.next_i32(),
                selector + 6 * (rng.next_u32() % 10_000) as i32,
                rng.next_i32(),
                rng.next_i32(),
                "param2 node selector",
            );
        }
    }

    for parent_remainder in [0, 1, 2] {
        for _ in 0..RANDOM_CASES {
            compare(
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                parent_remainder + 3 * (rng.next_u32() % 10_000) as i32,
                "positive parent selector",
            );
        }
    }

    for parent_selector in [-1, -2] {
        for _ in 0..RANDOM_CASES {
            compare(
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                parent_selector - 3 * (rng.next_u32() % 10_000) as i32,
                "negative parent selector",
            );
        }
    }

    for _ in 0..RANDOM_CASES * 2 {
        compare(
            rng.next_i32(),
            rng.next_i32(),
            -1,
            rng.next_i32(),
            "zero denominator",
        );
        let denominator = (rng.next_i32() % 10_000).max(1);
        compare(
            rng.next_i32(),
            rng.next_i32(),
            denominator,
            rng.next_i32(),
            "finite denominator",
        );
    }

    for values in [
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MAX, -1, i32::MAX],
        [i32::MAX, i32::MIN, 0, i32::MIN],
        [i32::MAX, 1, i32::MAX, -1],
        [i32::MIN, -1, i32::MIN, 1],
    ] {
        for _ in 0..RANDOM_CASES {
            compare(values[0], values[1], values[2], values[3], "extreme values");
        }
    }

    for call in 0..RANDOM_CASES * 4 {
        compare(
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            &format!("repeated reset call {call}"),
        );
    }
}

#[test]
fn errors_e01_capacity_rejection() {
    let pair = Pair::new();
    let mut rng = Rng::new(0xe01c_4fac_17ee);
    for index in 0..MAX_NODES {
        let name_length = (rng.next_u32() % 80) as usize;
        assert_eq!(
            pair.add(
                index as i32,
                rng.next_i32(),
                &rng.ascii_string(name_length),
                rng.finite_f64(),
            ),
            (index as i32, index as i32)
        );
    }
    for _ in 0..RANDOM_CASES {
        let name_length = (rng.next_u32() % 100) as usize;
        assert_eq!(
            pair.add(
                rng.next_i32(),
                rng.next_i32(),
                &rng.ascii_string(name_length),
                rng.finite_f64(),
            ),
            (-1, -1)
        );
    }
}

#[test]
fn errors_e02_e03_missing_lookup_sentinels() {
    let mut rng = Rng::new(0xe02e_03ab_5e17);
    for _ in 0..RANDOM_CASES {
        let pair = Pair::new();
        let existing_id = rng.next_i32();
        let missing_id = existing_id.wrapping_add(1);
        assert_eq!(
            pair.add(existing_id, rng.next_i32(), c"present", rng.finite_f64()),
            (0, 0)
        );
        let missing = pair.find(missing_id);
        assert!(missing.0.is_null());
        assert!(missing.1.is_null());
        let sums = pair.subtree(missing_id);
        assert_eq!(sums.0.to_bits(), 0.0_f64.to_bits());
        assert_eq!(sums.1.to_bits(), 0.0_f64.to_bits());

        let existing = pair.find(existing_id);
        unsafe {
            (*existing.0).active = 0;
            (*existing.1).active = 0;
        }
        let inactive = pair.find(existing_id);
        assert!(inactive.0.is_null());
        assert!(inactive.1.is_null());
    }
}

#[test]
fn errors_e07_e08_missing_maxnmin_selectors() {
    let pair = Pair::new();
    let c_maxnmin = unsafe { pair.c_symbol::<Maxnmin>(b"maxnmin\0") };
    let rust_maxnmin = unsafe { pair.rust_symbol::<Maxnmin>(b"maxnmin\0") };
    let mut rng = Rng::new(0xe07e_08a8_5e17);

    for remainder in 1..=5 {
        for _ in 0..RANDOM_CASES {
            let negative_selector = -(remainder + 6 * (rng.next_u32() % 10_000) as i32);
            let b = rng.next_i32();
            let c = rng.next_i32();
            let d = rng.next_i32();
            unsafe {
                assert_eq!(
                    c_maxnmin(negative_selector, b, c, d),
                    rust_maxnmin(negative_selector, b, c, d),
                    "missing param1 selector"
                );
            }

            let a = rng.next_i32();
            unsafe {
                assert_eq!(
                    c_maxnmin(a, negative_selector, c, d),
                    rust_maxnmin(a, negative_selector, c, d),
                    "missing param2 selector"
                );
            }
        }
    }
}

#[test]
fn generic_null_pointer_boundaries_fault_identically() {
    use std::os::unix::process::ExitStatusExt;

    fn child_status(library: &Path, function: &str) -> std::process::ExitStatus {
        Command::new(env::current_exe().expect("missing current test executable"))
            .args(["--exact", "null_pointer_child", "--nocapture"])
            .env("MAXNMIN_NULL_LIBRARY", library)
            .env("MAXNMIN_NULL_FUNCTION", function)
            .status()
            .expect("failed to run null-pointer child")
    }

    for function in ["add_node", "process_string"] {
        let c_status = child_status(&c_library_path(), function);
        let rust_status = child_status(&rust_library_path(), function);
        assert!(
            !c_status.success(),
            "C {function}(NULL) unexpectedly returned"
        );
        assert!(
            !rust_status.success(),
            "Rust {function}(NULL) unexpectedly returned"
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "{function}(NULL) terminated differently"
        );
    }
}

#[test]
fn null_pointer_child() {
    let Ok(library_path) = env::var("MAXNMIN_NULL_LIBRARY") else {
        return;
    };
    let function = env::var("MAXNMIN_NULL_FUNCTION").expect("missing null function");
    let library = unsafe { Library::new(library_path).expect("failed to load child library") };

    unsafe {
        match function.as_str() {
            "add_node" => {
                let call = *library
                    .get::<AddNode>(b"add_node\0")
                    .expect("missing add_node");
                let _ = call(1, -1, ptr::null(), 1.0);
            }
            "process_string" => {
                let call = *library
                    .get::<ProcessString>(b"process_string\0")
                    .expect("missing process_string");
                let _ = call(ptr::null_mut());
            }
            _ => panic!("unknown null function"),
        }
    }
}
