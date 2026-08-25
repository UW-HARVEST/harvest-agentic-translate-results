use libloading::Library;
use std::ffi::{c_char, c_double, c_int};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const MAX_NODES: usize = 100;
const MAX_NAME_LEN: usize = 50;
const RANDOM_CASES: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
struct Node {
    id: c_int,
    parent_id: c_int,
    name: [c_char; MAX_NAME_LEN],
    value: c_double,
    active: c_int,
}

type AddNode = unsafe extern "C" fn(c_int, c_int, *const c_char, c_double) -> c_int;
type FindNode = unsafe extern "C" fn(c_int) -> *mut Node;
type ChildrenCount = unsafe extern "C" fn(c_int) -> c_int;
type SubtreeSum = unsafe extern "C" fn(c_int) -> c_double;
type ProcessString = unsafe extern "C" fn(*mut c_char) -> c_int;
type DoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type Maxnmin = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    add_node: AddNode,
    find_node: FindNode,
    children_count: ChildrenCount,
    subtree_sum: SubtreeSum,
    process_string: ProcessString,
    double_to_int: DoubleToInt,
    maxnmin: Maxnmin,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let add_node = *unsafe { library.get(b"add_node\0") }.unwrap();
        let find_node = *unsafe { library.get(b"find_node_by_id\0") }.unwrap();
        let children_count = *unsafe { library.get(b"get_children_count\0") }.unwrap();
        let subtree_sum = *unsafe { library.get(b"calculate_subtree_sum\0") }.unwrap();
        let process_string = *unsafe { library.get(b"process_string\0") }.unwrap();
        let double_to_int = *unsafe { library.get(b"safe_double_to_int\0") }.unwrap();
        let maxnmin = *unsafe { library.get(b"maxnmin\0") }.unwrap();
        Self {
            _library: library,
            add_node,
            find_node,
            children_count,
            subtree_sum,
            process_string,
            double_to_int,
            maxnmin,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    unsafe fn load() -> Self {
        Self {
            c: unsafe { Api::load(&c_library_path()) },
            rust: unsafe { Api::load(&rust_library_path()) },
        }
    }

    unsafe fn reset(&self) {
        let c = unsafe { (self.c.maxnmin)(0, 0, 0, 0) };
        let rust = unsafe { (self.rust.maxnmin)(0, 0, 0, 0) };
        assert_eq!(rust, c, "fixture reset diverged");
    }

    unsafe fn add(&self, id: c_int, parent_id: c_int, name: &[c_char], value: c_double) -> c_int {
        let c = unsafe { (self.c.add_node)(id, parent_id, name.as_ptr(), value) };
        let rust = unsafe { (self.rust.add_node)(id, parent_id, name.as_ptr(), value) };
        assert_eq!(rust, c, "add_node({id}, {parent_id}, .., {value:?})");
        c
    }

    unsafe fn assert_node(&self, id: c_int) -> (*mut Node, *mut Node) {
        let c = unsafe { (self.c.find_node)(id) };
        let rust = unsafe { (self.rust.find_node)(id) };
        assert_eq!(rust.is_null(), c.is_null(), "find_node_by_id({id})");
        if !c.is_null() {
            unsafe { assert_node_bytes(c, rust, id) };
        }
        (c, rust)
    }

    unsafe fn assert_children(&self, parent_id: c_int) -> c_int {
        let c = unsafe { (self.c.children_count)(parent_id) };
        let rust = unsafe { (self.rust.children_count)(parent_id) };
        assert_eq!(rust, c, "get_children_count({parent_id})");
        c
    }

    unsafe fn assert_sum(&self, id: c_int) -> c_double {
        let c = unsafe { (self.c.subtree_sum)(id) };
        let rust = unsafe { (self.rust.subtree_sum)(id) };
        assert_eq!(
            rust.to_bits(),
            c.to_bits(),
            "calculate_subtree_sum({id}): Rust {rust:?}, C {c:?}"
        );
        c
    }

    unsafe fn assert_maxnmin(&self, params: [c_int; 4]) -> c_int {
        let [a, b, c, d] = params;
        let c_result = unsafe { (self.c.maxnmin)(a, b, c, d) };
        let rust_result = unsafe { (self.rust.maxnmin)(a, b, c, d) };
        assert_eq!(rust_result, c_result, "maxnmin({a}, {b}, {c}, {d})");
        c_result
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("current test executable");
    path.pop();
    path.join("libmaxnmin_lib.so")
}

unsafe fn node_bytes<'a>(node: *const Node) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(node.cast(), size_of::<Node>()) }
}

unsafe fn assert_node_bytes(c: *const Node, rust: *const Node, id: c_int) {
    assert!(!c.is_null() && !rust.is_null());
    let c_bytes = unsafe { node_bytes(c) };
    let rust_bytes = unsafe { node_bytes(rust) };
    assert_eq!(rust_bytes, c_bytes, "Node bytes for id {id}");
}

fn c_string(bytes: &[u8]) -> Vec<c_char> {
    bytes
        .iter()
        .copied()
        .chain([0])
        .map(|byte| byte as c_char)
        .collect()
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn i32(&mut self) -> i32 {
        self.u64() as i32
    }

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        (self.u64() as usize) % upper_exclusive
    }

    fn byte_nonzero(&mut self) -> u8 {
        (self.u64() % 255 + 1) as u8
    }

    fn finite(&mut self) -> f64 {
        f64::from(self.i32()) / 1024.0
    }
}

#[test]
fn differential_surface() {
    assert_eq!(
        size_of::<Node>(),
        80,
        "test ABI must match the C Node layout"
    );
    assert!(
        c_library_path().is_file(),
        "build the C shared library first"
    );
    assert!(
        rust_library_path().is_file(),
        "Cargo must build the Rust cdylib"
    );

    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0x6d61_786e_6d69_6e21);

    unsafe {
        valid_storage_and_lookup_cases(&pair, &mut rng);
        valid_tree_cases(&pair, &mut rng);
        valid_string_cases(&pair, &mut rng);
        valid_conversion_cases(&pair, &mut rng);
        valid_maxnmin_cases(&pair, &mut rng);
        explicit_error_cases(&pair, &mut rng);
        null_boundary_cases();
    }
}

unsafe fn valid_storage_and_lookup_cases(pair: &Pair, rng: &mut Rng) {
    // CONFIGS 1: the newly loaded libraries both start empty.
    for _ in 0..RANDOM_CASES {
        unsafe { pair.assert_node(rng.i32()) };
    }

    // CONFIGS 2: empty names.
    unsafe { pair.reset() };
    for case in 0..RANDOM_CASES {
        let id = 10_000 + case as i32;
        assert_eq!(
            unsafe { pair.add(id, rng.i32(), &[0], rng.finite()) },
            6 + case as c_int
        );
        unsafe { pair.assert_node(id) };
    }

    // CONFIGS 3: names from 1 through 48 bytes.
    unsafe { pair.reset() };
    for case in 0..RANDOM_CASES {
        let length = 1 + rng.usize(48);
        let bytes: Vec<_> = (0..length).map(|_| rng.byte_nonzero()).collect();
        let id = 20_000 + case as i32;
        unsafe { pair.add(id, rng.i32(), &c_string(&bytes), rng.finite()) };
        unsafe { pair.assert_node(id) };
    }

    // CONFIGS 4: exact 49-byte names.
    unsafe { pair.reset() };
    for case in 0..RANDOM_CASES {
        let bytes: Vec<_> = (0..49).map(|_| rng.byte_nonzero()).collect();
        let id = 30_000 + case as i32;
        unsafe { pair.add(id, rng.i32(), &c_string(&bytes), rng.finite()) };
        unsafe { pair.assert_node(id) };
    }

    // CONFIGS 5: names longer than the 49-byte stored prefix.
    unsafe { pair.reset() };
    for case in 0..RANDOM_CASES {
        let length = 50 + rng.usize(151);
        let bytes: Vec<_> = (0..length).map(|_| rng.byte_nonzero()).collect();
        let id = 40_000 + case as i32;
        unsafe { pair.add(id, rng.i32(), &c_string(&bytes), rng.finite()) };
        let (c, rust) = unsafe { pair.assert_node(id) };
        assert_eq!(unsafe { (*c).name[49] }, 0);
        assert_eq!(unsafe { (*rust).name[49] }, 0);
    }

    // CONFIGS 6: arbitrary fields and representative special floating values.
    let special = [
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::from_bits(0x7ff8_1234_5678_9abc),
        f64::MIN,
        f64::MAX,
    ];
    unsafe { pair.reset() };
    for case in 0..RANDOM_CASES {
        let id = 50_000 + case as i32;
        let value = if case < special.len() {
            special[case]
        } else {
            f64::from_bits(rng.u64())
        };
        unsafe { pair.add(id, rng.i32(), &c_string(b"value"), value) };
        unsafe { pair.assert_node(id) };
    }

    // CONFIGS 7: repeatedly exercise the final valid capacity slot.
    for repetition in 0..32 {
        unsafe { pair.reset() };
        for index in 6..99 {
            let id = 60_000 + index as i32;
            unsafe { pair.add(id, rng.i32(), &c_string(b"fill"), rng.finite()) };
        }
        let id = 70_000 + repetition;
        assert_eq!(
            unsafe { pair.add(id, rng.i32(), &c_string(b"last"), rng.finite()) },
            99
        );
        unsafe { pair.assert_node(id) };
    }

    // CONFIGS 8: matches in different loop positions.
    for _ in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        for id in 1..=6 {
            unsafe { pair.assert_node(id) };
        }
        let id = rng.i32();
        if !(1..=6).contains(&id) {
            unsafe { pair.add(id, 0, &c_string(b"last"), rng.finite()) };
            unsafe { pair.assert_node(id) };
        }
    }

    // CONFIGS 9: duplicate IDs return the first active record.
    for case in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let id = 80_000 + case as i32;
        unsafe { pair.add(id, 1, &c_string(b"first"), rng.finite()) };
        let (first_c, first_rust) = unsafe { pair.assert_node(id) };
        unsafe { pair.add(id, 2, &c_string(b"second"), rng.finite()) };
        let (found_c, found_rust) = unsafe { pair.assert_node(id) };
        assert_eq!(found_c, first_c);
        assert_eq!(found_rust, first_rust);
    }

    // CONFIGS 10: inactive duplicate is skipped in favor of the later active one.
    for case in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let id = 90_000 + case as i32;
        unsafe { pair.add(id, 1, &c_string(b"inactive"), rng.finite()) };
        let (first_c, first_rust) = unsafe { pair.assert_node(id) };
        unsafe {
            (*first_c).active = 0;
            (*first_rust).active = 0;
        }
        unsafe { pair.add(id, 2, &c_string(b"active"), rng.finite()) };
        let (found_c, found_rust) = unsafe { pair.assert_node(id) };
        assert_ne!(found_c, first_c);
        assert_ne!(found_rust, first_rust);
        assert_eq!(unsafe { (*found_c).parent_id }, 2);
        assert_eq!(unsafe { (*found_rust).parent_id }, 2);
    }

    // CONFIGS 11-14: zero/one/many children and inactive exclusion.
    for case in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let parent = 100_000 + case as i32;
        assert_eq!(unsafe { pair.assert_children(parent) }, 0);
        unsafe { pair.add(110_000 + case as i32, parent, &c_string(b"one"), 1.0) };
        assert_eq!(unsafe { pair.assert_children(parent) }, 1);
        for offset in 0..(2 + rng.usize(8)) {
            unsafe {
                pair.add(
                    120_000 + case as i32 * 10 + offset as i32,
                    parent,
                    &c_string(b"many"),
                    1.0,
                )
            };
        }
        let before = unsafe { pair.assert_children(parent) };
        let (inactive_c, inactive_rust) = unsafe {
            pair.add(130_000 + case as i32, parent, &c_string(b"off"), 1.0);
            pair.assert_node(130_000 + case as i32)
        };
        unsafe {
            (*inactive_c).active = 0;
            (*inactive_rust).active = 0;
        }
        assert_eq!(unsafe { pair.assert_children(parent) }, before);
        assert_eq!(unsafe { pair.assert_children(rng.i32()) }, 0);
    }
}

unsafe fn valid_tree_cases(pair: &Pair, rng: &mut Rng) {
    // CONFIGS 15-18: leaf, direct children, recursion, sibling branches, inactive.
    for case in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let base = 200_000 + case as i32 * 10;
        let values = [
            rng.finite(),
            rng.finite(),
            rng.finite(),
            rng.finite(),
            rng.finite(),
        ];
        unsafe { pair.add(base, -1, &c_string(b"root"), values[0]) };
        assert_eq!(
            unsafe { pair.assert_sum(base) }.to_bits(),
            values[0].to_bits()
        );

        unsafe { pair.add(base + 1, base, &c_string(b"child-a"), values[1]) };
        unsafe { pair.add(base + 2, base, &c_string(b"child-b"), values[2]) };
        unsafe { pair.add(base + 3, base + 1, &c_string(b"deep"), values[3]) };
        unsafe { pair.add(base + 4, base + 2, &c_string(b"off"), values[4]) };
        let (off_c, off_rust) = unsafe { pair.assert_node(base + 4) };
        unsafe {
            (*off_c).active = 0;
            (*off_rust).active = 0;
        }

        unsafe { pair.assert_sum(base + 1) };
        unsafe { pair.assert_sum(base) };
    }
}

unsafe fn valid_string_cases(pair: &Pair, rng: &mut Rng) {
    // CONFIGS 19: empty strings.
    for _ in 0..RANDOM_CASES {
        let mut value = vec![0_i8];
        let c = unsafe { (pair.c.process_string)(value.as_mut_ptr()) };
        let rust = unsafe { (pair.rust.process_string)(value.as_mut_ptr()) };
        assert_eq!(rust, c);
        assert_eq!(c, 0);
    }

    // CONFIGS 20: one-byte and multi-byte ASCII.
    for case in 0..RANDOM_CASES {
        let length = if case == 0 { 1 } else { 1 + rng.usize(256) };
        let bytes: Vec<_> = (0..length).map(|_| (1 + rng.usize(127)) as u8).collect();
        let mut c_value = c_string(&bytes);
        let mut rust_value = c_value.clone();
        let c = unsafe { (pair.c.process_string)(c_value.as_mut_ptr()) };
        let rust = unsafe { (pair.rust.process_string)(rust_value.as_mut_ptr()) };
        assert_eq!(rust, c, "ASCII process_string case {case}");
        assert_eq!(rust_value, c_value);
    }

    // CONFIGS 21: signed high-bit bytes.
    for case in 0..RANDOM_CASES {
        let length = 1 + rng.usize(256);
        let bytes: Vec<_> = (0..length).map(|_| (128 + rng.usize(128)) as u8).collect();
        let mut c_value = c_string(&bytes);
        let mut rust_value = c_value.clone();
        let c = unsafe { (pair.c.process_string)(c_value.as_mut_ptr()) };
        let rust = unsafe { (pair.rust.process_string)(rust_value.as_mut_ptr()) };
        assert_eq!(rust, c, "high-bit process_string case {case}");
    }
}

unsafe fn valid_conversion_cases(pair: &Pair, rng: &mut Rng) {
    // CONFIGS 22: finite interior values and truncation toward zero.
    for case in 0..RANDOM_CASES * 4 {
        let value = match case {
            0 => 0.0,
            1 => -0.0,
            2 => 0.999,
            3 => -0.999,
            _ => f64::from(rng.i32()) / 2.0,
        };
        let c = unsafe { (pair.c.double_to_int)(value) };
        let rust = unsafe { (pair.rust.double_to_int)(value) };
        assert_eq!(rust, c, "safe_double_to_int({value:?})");
    }

    // CONFIGS 23: exact integer boundaries and neighboring in-range values.
    let boundaries = [
        c_int::MIN as f64,
        c_int::MIN as f64 + 1.0,
        c_int::MAX as f64 - 1.0,
        c_int::MAX as f64,
    ];
    for _ in 0..RANDOM_CASES {
        for value in boundaries {
            let c = unsafe { (pair.c.double_to_int)(value) };
            let rust = unsafe { (pair.rust.double_to_int)(value) };
            assert_eq!(rust, c, "safe_double_to_int({value:?})");
        }
    }
}

unsafe fn valid_maxnmin_cases(pair: &Pair, rng: &mut Rng) {
    // CONFIGS 24: all positive first-node selection classes.
    for _ in 0..RANDOM_CASES {
        for first in 0..6 {
            unsafe { pair.assert_maxnmin([first, rng.i32(), 2, rng.i32()]) };
        }
    }

    // CONFIGS 25: negative remainder yields ID 1 or a missing first node.
    for _ in 0..RANDOM_CASES {
        for first in -12..=0 {
            unsafe { pair.assert_maxnmin([first, 0, 2, 1]) };
        }
    }

    // CONFIGS 26: all second nodes with finite multipliers.
    for _ in 0..RANDOM_CASES {
        for second in 0..6 {
            let multiplier = (rng.i32() % 10_000).max(-10_000);
            unsafe { pair.assert_maxnmin([rng.i32(), second, multiplier, rng.i32()]) };
        }
    }

    // CONFIGS 27: negative second-node selections, including missing IDs.
    for _ in 0..RANDOM_CASES {
        for second in -12..=0 {
            unsafe { pair.assert_maxnmin([0, second, 3, 1]) };
        }
    }

    // CONFIGS 28: multiplication large enough to exceed the integer range.
    for _ in 0..RANDOM_CASES {
        for multiplier in [c_int::MIN, c_int::MAX] {
            for second in 0..6 {
                unsafe { pair.assert_maxnmin([rng.i32(), second, multiplier, 1]) };
            }
        }
    }

    // CONFIGS 29-30: positive and negative parent remainder classes.
    for _ in 0..RANDOM_CASES {
        for fourth in -12..=12 {
            unsafe { pair.assert_maxnmin([rng.i32(), rng.i32(), 2, fourth]) };
        }
    }

    // CONFIGS 31: finite positive and negative nonzero denominators.
    for _ in 0..RANDOM_CASES {
        for third in [-100, -3, -2, 0, 1, 2, 100] {
            unsafe { pair.assert_maxnmin([rng.i32(), rng.i32(), third, rng.i32()]) };
        }
    }

    // CONFIGS 32-33: division by zero gives infinity or NaN.
    for _ in 0..RANDOM_CASES {
        let nonzero = loop {
            let value = rng.i32();
            if value != 0 {
                break value;
            }
        };
        let nonzero_fourth = loop {
            let value = rng.i32();
            if value != 0 {
                break value;
            }
        };
        unsafe { pair.assert_maxnmin([nonzero, 0, -1, nonzero_fourth]) };
        unsafe { pair.assert_maxnmin([nonzero, nonzero.wrapping_neg(), -1, rng.i32()]) };
    }

    // CONFIGS 34: integer expression boundaries under the concrete C build.
    let edge = [
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for _ in 0..RANDOM_CASES {
        unsafe {
            pair.assert_maxnmin([
                edge[rng.usize(edge.len())],
                edge[rng.usize(edge.len())],
                edge[rng.usize(edge.len())],
                edge[rng.usize(edge.len())],
            ])
        };
    }

    // CONFIGS 35: fixed-seed cross-product plus full-width random tuples.
    let first = [-7, -6, -5, -1, 0, 1, 2, 5];
    let second = [-7, -6, -5, -1, 0, 2, 5];
    let third = [-2, -1, 0, 1, c_int::MIN, c_int::MAX];
    let fourth = [-4, -3, -2, -1, 0, 1, 2];
    for &a in &first {
        for &b in &second {
            for &c in &third {
                for &d in &fourth {
                    unsafe { pair.assert_maxnmin([a, b, c, d]) };
                }
            }
        }
    }
    for _ in 0..RANDOM_CASES * 32 {
        unsafe { pair.assert_maxnmin([rng.i32(), rng.i32(), rng.i32(), rng.i32()]) };
    }
}

unsafe fn explicit_error_cases(pair: &Pair, rng: &mut Rng) {
    // ERRORS 1: full storage rejects all further inserts before reading name.
    for _ in 0..32 {
        unsafe { pair.reset() };
        for index in 6..MAX_NODES {
            unsafe {
                pair.add(
                    300_000 + index as i32,
                    rng.i32(),
                    &c_string(b"fill"),
                    rng.finite(),
                )
            };
        }
        let c = unsafe { (pair.c.add_node)(rng.i32(), rng.i32(), std::ptr::null(), 0.0) };
        let rust = unsafe { (pair.rust.add_node)(rng.i32(), rng.i32(), std::ptr::null(), 0.0) };
        assert_eq!(c, -1);
        assert_eq!(rust, c);
    }

    // ERRORS 2-3: absent/inactive lookups and subtree roots.
    for case in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let absent = 400_000 + case as i32;
        unsafe { pair.assert_node(absent) };
        assert_eq!(
            unsafe { pair.assert_sum(absent) }.to_bits(),
            0.0f64.to_bits()
        );

        let id = 410_000 + case as i32;
        unsafe { pair.add(id, 0, &c_string(b"off"), rng.finite()) };
        let (c, rust) = unsafe { pair.assert_node(id) };
        unsafe {
            (*c).active = 0;
            (*rust).active = 0;
        }
        unsafe { pair.assert_node(id) };
        assert_eq!(unsafe { pair.assert_sum(id) }.to_bits(), 0.0f64.to_bits());
    }

    // ERRORS 4-5: one step outside each range, wider finite values, infinities.
    let out_of_range = [
        c_int::MAX as f64 + 1.0,
        c_int::MAX as f64 * 2.0,
        f64::MAX,
        f64::INFINITY,
        c_int::MIN as f64 - 1.0,
        c_int::MIN as f64 * 2.0,
        f64::MIN,
        f64::NEG_INFINITY,
    ];
    for _ in 0..RANDOM_CASES {
        for value in out_of_range {
            let c = unsafe { (pair.c.double_to_int)(value) };
            let rust = unsafe { (pair.rust.double_to_int)(value) };
            assert_eq!(rust, c, "safe_double_to_int({value:?})");
        }
    }

    // ERRORS 6: randomized positive and negative NaN payloads.
    for _ in 0..RANDOM_CASES * 4 {
        let payload = rng.u64() & 0x000f_ffff_ffff_ffff;
        let sign = rng.u64() & (1 << 63);
        let value = f64::from_bits(sign | 0x7ff8_0000_0000_0000 | payload);
        let c = unsafe { (pair.c.double_to_int)(value) };
        let rust = unsafe { (pair.rust.double_to_int)(value) };
        assert_eq!(rust, c);
        assert_eq!(c, 0);
    }
}

unsafe fn null_boundary_cases() {
    for operation in ["process_string", "add_node"] {
        let c = run_null_probe("c", operation);
        let rust = run_null_probe("rust", operation);
        assert_same_termination(c, rust, operation);
    }
}

fn run_null_probe(implementation: &str, operation: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "null_probe_helper",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NULL_PROBE_IMPLEMENTATION", implementation)
        .env("NULL_PROBE_OPERATION", operation)
        .status()
        .unwrap()
}

#[cfg(unix)]
fn assert_same_termination(c: ExitStatus, rust: ExitStatus, operation: &str) {
    use std::os::unix::process::ExitStatusExt;
    assert!(!c.success(), "C {operation}(NULL) unexpectedly succeeded");
    assert!(
        !rust.success(),
        "Rust {operation}(NULL) unexpectedly succeeded"
    );
    assert_eq!(
        rust.signal(),
        c.signal(),
        "{operation}(NULL) termination signal"
    );
}

#[cfg(not(unix))]
fn assert_same_termination(c: ExitStatus, rust: ExitStatus, operation: &str) {
    assert_eq!(rust.code(), c.code(), "{operation}(NULL) exit code");
}

#[test]
fn null_probe_helper() {
    let Ok(implementation) = std::env::var("NULL_PROBE_IMPLEMENTATION") else {
        return;
    };
    let operation = std::env::var("NULL_PROBE_OPERATION").unwrap();
    let path = match implementation.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown implementation {other}"),
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        match operation.as_str() {
            "process_string" => {
                (api.process_string)(std::ptr::null_mut());
            }
            "add_node" => {
                (api.add_node)(1, -1, std::ptr::null(), 1.0);
            }
            other => panic!("unknown null probe {other}"),
        }
    }
}
