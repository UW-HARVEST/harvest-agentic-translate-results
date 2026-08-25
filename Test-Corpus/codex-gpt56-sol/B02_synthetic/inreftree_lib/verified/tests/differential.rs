use libloading::Library;
use std::ffi::{CString, c_char, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

const MAX_NODES: usize = 50;
const RANDOM_CASES: usize = 256;
static LIBRARY_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct TreeNode {
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: [c_char; 32],
}

impl TreeNode {
    fn new(id: c_int, value: c_int, left: c_int, right: c_int) -> Self {
        Self {
            id,
            value,
            parent_id: -1,
            left_child_id: left,
            right_child_id: right,
            label: [0; 32],
        }
    }
}

type Operation = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type FindNode = unsafe extern "C" fn(c_int) -> *mut TreeNode;
type AddTreeNode = unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int;
type CalculateTreeSum = unsafe extern "C" fn(c_int) -> c_int;
type ParseOperation = unsafe extern "C" fn(*const c_char) -> c_int;
type GetOperation = unsafe extern "C" fn(c_int) -> Operation;
type Inreftree = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    add_op: Operation,
    multiply_op: Operation,
    subtract_op: Operation,
    divide_op: Operation,
    modulo_op: Operation,
    find_node_by_id: FindNode,
    add_tree_node: AddTreeNode,
    calculate_tree_sum: CalculateTreeSum,
    parse_operation: ParseOperation,
    get_operation_func: GetOperation,
    inreftree: Inreftree,
    node_table: *mut TreeNode,
    node_count: *mut c_int,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        unsafe {
            Self {
                add_op: *library.get(b"add_op").unwrap(),
                multiply_op: *library.get(b"multiply_op").unwrap(),
                subtract_op: *library.get(b"subtract_op").unwrap(),
                divide_op: *library.get(b"divide_op").unwrap(),
                modulo_op: *library.get(b"modulo_op").unwrap(),
                find_node_by_id: *library.get(b"find_node_by_id").unwrap(),
                add_tree_node: *library.get(b"add_tree_node").unwrap(),
                calculate_tree_sum: *library.get(b"calculate_tree_sum").unwrap(),
                parse_operation: *library.get(b"parse_operation").unwrap(),
                get_operation_func: *library.get(b"get_operation_func").unwrap(),
                inreftree: *library.get(b"inreftree").unwrap(),
                node_table: *library.get::<*mut TreeNode>(b"node_table").unwrap(),
                node_count: *library.get::<*mut c_int>(b"node_count").unwrap(),
                _library: library,
            }
        }
    }

    unsafe fn reset(&self) {
        unsafe {
            *self.node_count = 0;
            ptr::write_bytes(self.node_table, 0, MAX_NODES);
        }
    }

    unsafe fn install_nodes(&self, nodes: &[TreeNode]) {
        assert!(nodes.len() <= MAX_NODES);
        unsafe {
            self.reset();
            ptr::copy_nonoverlapping(nodes.as_ptr(), self.node_table, nodes.len());
            *self.node_count = nodes.len() as c_int;
        }
    }

    unsafe fn count(&self) -> c_int {
        unsafe { *self.node_count }
    }

    unsafe fn table_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.node_table.cast::<u8>(),
                MAX_NODES * std::mem::size_of::<TreeNode>(),
            )
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    unsafe fn load() -> Self {
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }

    unsafe fn reset(&self) {
        unsafe {
            self.c.reset();
            self.rust.reset();
        }
    }

    unsafe fn assert_state_equal(&self, context: &str) {
        unsafe {
            assert_eq!(self.c.count(), self.rust.count(), "{context}: node_count");
            assert_eq!(
                self.c.table_bytes(),
                self.rust.table_bytes(),
                "{context}: node_table"
            );
        }
    }

    unsafe fn add_node(
        &self,
        id: c_int,
        value: c_int,
        parent_id: c_int,
        label: &CString,
        context: &str,
    ) -> c_int {
        let c_result = unsafe { (self.c.add_tree_node)(id, value, parent_id, label.as_ptr()) };
        let rust_result =
            unsafe { (self.rust.add_tree_node)(id, value, parent_id, label.as_ptr()) };
        assert_eq!(c_result, rust_result, "{context}: return value");
        unsafe { self.assert_state_equal(context) };
        c_result
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

    fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn bounded(&mut self, bound: i32) -> i32 {
        (self.next_u32() % (bound as u32 * 2 + 1)) as i32 - bound
    }

    fn nonzero_bounded(&mut self, bound: i32) -> i32 {
        loop {
            let value = self.bounded(bound);
            if value != 0 {
                return value;
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/debug/libinreftree_lib.so")
}

fn assert_libraries_exist() {
    for path in [c_library_path(), rust_library_path()] {
        assert!(
            path.is_file(),
            "shared library does not exist: {}",
            path.display()
        );
    }
}

fn random_text(rng: &mut Rng, len: usize) -> String {
    (0..len)
        .map(|_| (b'a' + (rng.next_u32() % 26) as u8) as char)
        .collect()
}

unsafe fn compare_operation(
    c_operation: Operation,
    rust_operation: Operation,
    a: i32,
    b: i32,
    unused1: i32,
    unused2: i32,
    context: &str,
) {
    let c_result = unsafe { c_operation(a, b, unused1, unused2) };
    let rust_result = unsafe { rust_operation(a, b, unused1, unused2) };
    assert_eq!(c_result, rust_result, "{context}: ({a}, {b})");
}

#[test]
fn operation_and_parser_configurations_match() {
    let _guard = LIBRARY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_libraries_exist();
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0x6d5a_56e9_1357_2468);

    // C01-C05: each operation gets many fixed-seed randomized calls.
    for _ in 0..RANDOM_CASES {
        let a = rng.bounded(30_000);
        let b = rng.bounded(30_000);
        let small_a = rng.bounded(1_000);
        let small_b = rng.bounded(1_000);
        let divisor = rng.nonzero_bounded(30_000);
        let ignored1 = rng.i32();
        let ignored2 = rng.i32();
        unsafe {
            compare_operation(
                pair.c.add_op,
                pair.rust.add_op,
                a,
                b,
                ignored1,
                ignored2,
                "C01",
            );
            compare_operation(
                pair.c.multiply_op,
                pair.rust.multiply_op,
                small_a,
                small_b,
                ignored1,
                ignored2,
                "C02",
            );
            compare_operation(
                pair.c.subtract_op,
                pair.rust.subtract_op,
                a,
                b,
                ignored1,
                ignored2,
                "C03",
            );
            compare_operation(
                pair.c.divide_op,
                pair.rust.divide_op,
                a,
                divisor,
                ignored1,
                ignored2,
                "C04",
            );
            compare_operation(
                pair.c.modulo_op,
                pair.rust.modulo_op,
                a,
                divisor,
                ignored1,
                ignored2,
                "C05",
            );
        }
    }

    // Generic integer boundaries that remain defined in C.
    for (operation_c, operation_rust, cases) in [
        (
            pair.c.add_op,
            pair.rust.add_op,
            [(i32::MIN, 0), (i32::MAX, 0)],
        ),
        (
            pair.c.multiply_op,
            pair.rust.multiply_op,
            [(i32::MIN, 1), (i32::MAX, 1)],
        ),
        (
            pair.c.subtract_op,
            pair.rust.subtract_op,
            [(i32::MIN, 0), (i32::MAX, 0)],
        ),
        (
            pair.c.divide_op,
            pair.rust.divide_op,
            [(i32::MIN, 1), (i32::MAX, -1)],
        ),
        (
            pair.c.modulo_op,
            pair.rust.modulo_op,
            [(i32::MIN, 1), (i32::MAX, -1)],
        ),
    ] {
        for (a, b) in cases {
            unsafe { compare_operation(operation_c, operation_rust, a, b, 0, 0, "int boundary") };
        }
    }
    for (c_operation, rust_operation) in [
        (pair.c.add_op, pair.rust.add_op),
        (pair.c.multiply_op, pair.rust.multiply_op),
        (pair.c.subtract_op, pair.rust.subtract_op),
        (pair.c.divide_op, pair.rust.divide_op),
        (pair.c.modulo_op, pair.rust.modulo_op),
    ] {
        unsafe { compare_operation(c_operation, rust_operation, 0, 0, 0, 0, "zero boundary") };
    }
    for (c_operation, rust_operation, cases) in [
        (
            pair.c.add_op,
            pair.rust.add_op,
            [(i32::MAX, 1), (i32::MIN, -1)],
        ),
        (
            pair.c.multiply_op,
            pair.rust.multiply_op,
            [(i32::MAX, 2), (i32::MIN, -1)],
        ),
        (
            pair.c.subtract_op,
            pair.rust.subtract_op,
            [(i32::MIN, 1), (i32::MAX, -1)],
        ),
    ] {
        for (a, b) in cases {
            unsafe {
                compare_operation(c_operation, rust_operation, a, b, 0, 0, "wrapping boundary")
            };
        }
    }

    // C22-C27: parser precedence classes and fallback.
    for _ in 0..RANDOM_CASES {
        let length = (rng.next_u32() % 24) as usize;
        let text = random_text(&mut rng, length);
        let cases = [
            (format!("{text}%/-*+"), 1, "C22"),
            (format!("{text}%/-*"), 2, "C23"),
            (format!("{text}%/-"), 3, "C24"),
            (format!("{text}%/"), 4, "C25"),
            (format!("{text}%"), 5, "C26"),
            (text, 1, "C27"),
        ];
        for (text, expected, row) in cases {
            let text = CString::new(text).unwrap();
            let c_result = unsafe { (pair.c.parse_operation)(text.as_ptr()) };
            let rust_result = unsafe { (pair.rust.parse_operation)(text.as_ptr()) };
            assert_eq!(c_result, expected, "{row}: C result");
            assert_eq!(c_result, rust_result, "{row}: differential result");
        }
    }
    let empty = CString::new("").unwrap();
    assert_eq!(unsafe { (pair.c.parse_operation)(empty.as_ptr()) }, 1);
    assert_eq!(
        unsafe { (pair.c.parse_operation)(empty.as_ptr()) },
        unsafe { (pair.rust.parse_operation)(empty.as_ptr()) }
    );

    // C28 C29 C30 C31 C32: function-pointer dispatch calls the result.
    for operation in 1..=5 {
        let c_function = unsafe { (pair.c.get_operation_func)(operation) };
        let rust_function = unsafe { (pair.rust.get_operation_func)(operation) };
        for _ in 0..RANDOM_CASES {
            let a = rng.bounded(1_000);
            let mut b = rng.bounded(1_000);
            if operation >= 4 && b == 0 {
                b = 1;
            }
            unsafe {
                compare_operation(
                    c_function,
                    rust_function,
                    a,
                    b,
                    rng.i32(),
                    rng.i32(),
                    &format!("C{}", 27 + operation),
                )
            };
        }
    }
}

#[test]
fn tree_configurations_match() {
    let _guard = LIBRARY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_libraries_exist();
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0xa076_1d64_78bd_642f);

    // C06: empty table lookup.
    for _ in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let id = rng.i32();
        assert!(unsafe { (pair.c.find_node_by_id)(id) }.is_null());
        assert!(unsafe { (pair.rust.find_node_by_id)(id) }.is_null());
    }

    // C07-C09: first, later, and absent IDs in populated tables.
    for _ in 0..RANDOM_CASES {
        let first_id = rng.bounded(10_000);
        let later_id = first_id.wrapping_add(1);
        let missing_id = first_id.wrapping_add(2);
        let nodes = [
            TreeNode::new(first_id, rng.bounded(10_000), -1, -1),
            TreeNode::new(later_id, rng.bounded(10_000), -1, -1),
        ];
        unsafe {
            pair.c.install_nodes(&nodes);
            pair.rust.install_nodes(&nodes);
        }
        for (id, index, row) in [(first_id, 0, "C07"), (later_id, 1, "C08")] {
            let c_node = unsafe { (pair.c.find_node_by_id)(id) };
            let rust_node = unsafe { (pair.rust.find_node_by_id)(id) };
            assert_eq!(
                c_node,
                unsafe { pair.c.node_table.add(index) },
                "{row}: C pointer"
            );
            assert_eq!(
                rust_node,
                unsafe { pair.rust.node_table.add(index) },
                "{row}: Rust pointer"
            );
            assert_eq!(unsafe { (*c_node).value }, unsafe { (*rust_node).value });
        }
        assert!(unsafe { (pair.c.find_node_by_id)(missing_id) }.is_null());
        assert!(unsafe { (pair.rust.find_node_by_id)(missing_id) }.is_null());
    }

    // C10: roots with empty and short labels.
    for index in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let label = if index % 2 == 0 {
            CString::new("").unwrap()
        } else {
            CString::new(random_text(&mut rng, 1 + index % 30)).unwrap()
        };
        assert_eq!(
            unsafe { pair.add_node(rng.i32(), rng.i32(), -1, &label, "C10 root insertion",) },
            0
        );
    }

    // C11-C12: exact boundary and truncating label lengths.
    for _ in 0..RANDOM_CASES {
        for (length, row) in [(31, "C11"), (32 + rng.next_u32() as usize % 96, "C12")] {
            unsafe { pair.reset() };
            let label = CString::new(random_text(&mut rng, length)).unwrap();
            unsafe {
                pair.add_node(rng.i32(), rng.i32(), -1, &label, row);
                let c_label = &(*pair.c.node_table).label;
                let rust_label = &(*pair.rust.node_table).label;
                assert_eq!(c_label, rust_label, "{row}: stored label");
                assert_eq!(c_label[31], 0, "{row}: terminator");
            }
        }
    }

    // C13-C15: left, right, then full-parent attachment branches.
    for _ in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let root_id = rng.bounded(10_000);
        let left_id = root_id.wrapping_add(1);
        let right_id = root_id.wrapping_add(2);
        let extra_id = root_id.wrapping_add(3);
        let label = CString::new(random_text(&mut rng, 12)).unwrap();
        unsafe {
            pair.add_node(root_id, rng.i32(), -1, &label, "parent");
            pair.add_node(left_id, rng.i32(), root_id, &label, "C13");
            assert_eq!((*pair.c.node_table).left_child_id, left_id);
            pair.add_node(right_id, rng.i32(), root_id, &label, "C14");
            assert_eq!((*pair.c.node_table).right_child_id, right_id);
            pair.add_node(extra_id, rng.i32(), root_id, &label, "C15");
            assert_eq!((*pair.c.node_table).left_child_id, left_id);
            assert_eq!((*pair.c.node_table).right_child_id, right_id);
            assert_eq!(pair.c.count(), 4);
        }
    }

    // C16: duplicate IDs are accepted and lookup stops at the first.
    for _ in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let id = rng.i32();
        let label = CString::new("duplicate").unwrap();
        unsafe {
            pair.add_node(id, 10, -1, &label, "C16 first");
            pair.add_node(id, 20, -1, &label, "C16 duplicate");
            let c_node = (pair.c.find_node_by_id)(id);
            let rust_node = (pair.rust.find_node_by_id)(id);
            assert_eq!((*c_node).value, 10);
            assert_eq!((*rust_node).value, 10);
        }
    }

    // C17-C21: leaf, each child branch, both children, and nested recursion.
    for _ in 0..RANDOM_CASES {
        let values = [rng.i32(), rng.i32(), rng.i32(), rng.i32()];
        let shapes = [
            (vec![TreeNode::new(1, values[0], -1, -1)], "C17"),
            (
                vec![
                    TreeNode::new(1, values[0], 2, -1),
                    TreeNode::new(2, values[1], -1, -1),
                ],
                "C18",
            ),
            (
                vec![
                    TreeNode::new(1, values[0], -1, 2),
                    TreeNode::new(2, values[1], -1, -1),
                ],
                "C19",
            ),
            (
                vec![
                    TreeNode::new(1, values[0], 2, 3),
                    TreeNode::new(2, values[1], -1, -1),
                    TreeNode::new(3, values[2], -1, -1),
                ],
                "C20",
            ),
            (
                vec![
                    TreeNode::new(1, values[0], 2, 3),
                    TreeNode::new(2, values[1], 4, -1),
                    TreeNode::new(3, values[2], -1, -1),
                    TreeNode::new(4, values[3], -1, -1),
                ],
                "C21",
            ),
        ];
        for (nodes, row) in shapes {
            unsafe {
                pair.c.install_nodes(&nodes);
                pair.rust.install_nodes(&nodes);
            }
            let c_sum = unsafe { (pair.c.calculate_tree_sum)(1) };
            let rust_sum = unsafe { (pair.rust.calculate_tree_sum)(1) };
            assert_eq!(c_sum, rust_sum, "{row}");
        }
    }

    // C33: writes through both exported globals are observed by lookup.
    for _ in 0..RANDOM_CASES {
        let node = TreeNode::new(rng.i32(), rng.i32(), -1, -1);
        unsafe {
            pair.c.install_nodes(&[node]);
            pair.rust.install_nodes(&[node]);
            assert_eq!(
                (*(pair.c.find_node_by_id)(node.id)).value,
                (*(pair.rust.find_node_by_id)(node.id)).value,
                "C33"
            );
            pair.assert_state_equal("C33");
        }
    }

    // C46: count 49 accepts one final node and reaches exact capacity.
    for _ in 0..RANDOM_CASES {
        let nodes: Vec<_> = (0..MAX_NODES - 1)
            .map(|index| TreeNode::new(index as i32, rng.i32(), -1, -1))
            .collect();
        unsafe {
            pair.c.install_nodes(&nodes);
            pair.rust.install_nodes(&nodes);
        }
        let label = CString::new(random_text(&mut rng, 31)).unwrap();
        let result = unsafe { pair.add_node(10_000, rng.i32(), -1, &label, "C46") };
        assert_eq!(result, 49);
        assert_eq!(unsafe { pair.c.count() }, 50);
    }

    // Explicit zero-value boundaries for the stateful API.
    let empty = CString::new("").unwrap();
    unsafe {
        pair.reset();
        pair.add_node(0, 0, -1, &empty, "zero tree boundary");
        assert_eq!((pair.c.find_node_by_id)(0), pair.c.node_table);
        assert_eq!((pair.rust.find_node_by_id)(0), pair.rust.node_table);
        assert_eq!((pair.c.calculate_tree_sum)(0), 0);
        assert_eq!((pair.rust.calculate_tree_sum)(0), 0);
    }
}

unsafe fn compare_inreftree_case(pair: &Pair, params: [i32; 4], context: &str) {
    let c_result = unsafe { (pair.c.inreftree)(params[0], params[1], params[2], params[3]) };
    let rust_result = unsafe { (pair.rust.inreftree)(params[0], params[1], params[2], params[3]) };
    assert_eq!(c_result, rust_result, "{context}: params={params:?}");
    unsafe { pair.assert_state_equal(context) };
}

#[test]
fn composed_inreftree_configurations_match() {
    let _guard = LIBRARY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_libraries_exist();
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0xe703_7ed1_a0b4_28db);

    // C34 C35 C36 C37 C38 C39 C40 C41: target state crossed with each index.
    for target_is_zero in [false, true] {
        for remainder in 0..=3 {
            let row = 34 + target_is_zero as usize * 4 + remainder as usize;
            for _ in 0..RANDOM_CASES {
                let p1 = rng.bounded(1_000);
                let p2 = if target_is_zero {
                    0
                } else {
                    rng.nonzero_bounded(1_000)
                };
                let p3 = rng.bounded(1_000);
                let base = p1 + p2 + p3;
                let positive_total = 4_000 + remainder + 4 * (rng.next_u32() % 1_000) as i32;
                let p4 = positive_total - base;
                unsafe { compare_inreftree_case(&pair, [p1, p2, p3, p4], &format!("C{row:02}")) };
            }
        }
    }

    // C42 C43 C44: negative C remainders and the source's preceding-byte reads.
    for magnitude in 1..=3 {
        for index in 0..RANDOM_CASES {
            let p1 = rng.bounded(1_000);
            let p2 = if index % 2 == 0 {
                0
            } else {
                rng.nonzero_bounded(1_000)
            };
            let p3 = rng.bounded(1_000);
            let p4 = -magnitude - p1 - p2 - p3;
            unsafe {
                compare_inreftree_case(&pair, [p1, p2, p3, p4], &format!("C{}", 41 + magnitude))
            };
        }
    }

    // C45: inreftree discards externally populated state first.
    for _ in 0..RANDOM_CASES {
        unsafe {
            *pair.c.node_count = MAX_NODES as c_int;
            *pair.rust.node_count = MAX_NODES as c_int;
            ptr::write_bytes(pair.c.node_table, 0xa5, MAX_NODES);
            ptr::write_bytes(pair.rust.node_table, 0xa5, MAX_NODES);
            compare_inreftree_case(
                &pair,
                [
                    rng.bounded(100),
                    rng.nonzero_bounded(100),
                    rng.bounded(100),
                    rng.bounded(100),
                ],
                "C45",
            );
            assert_eq!(pair.c.count(), 4);
        }
    }

    unsafe { compare_inreftree_case(&pair, [0, 0, 0, 0], "zero boundary") };

    // Full-width values stress wrapping accumulation and result arithmetic.
    for _ in 0..RANDOM_CASES * 4 {
        unsafe {
            compare_inreftree_case(
                &pair,
                [rng.i32(), rng.i32(), rng.i32(), rng.i32()],
                "full-width randomized",
            )
        };
    }
}

#[test]
fn error_configurations_match() {
    let _guard = LIBRARY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_libraries_exist();
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0x8ebc_6af0_9c88_c6e3);

    // E01 E02: zero divisors return the exact sentinel.
    for _ in 0..RANDOM_CASES {
        let a = rng.i32();
        for (c_operation, rust_operation, row) in [
            (pair.c.divide_op, pair.rust.divide_op, "E01"),
            (pair.c.modulo_op, pair.rust.modulo_op, "E02"),
        ] {
            let ignored1 = rng.i32();
            let ignored2 = rng.i32();
            let c_result = unsafe { c_operation(a, 0, ignored1, ignored2) };
            let rust_result = unsafe { rust_operation(a, 0, ignored1, ignored2) };
            assert_eq!(c_result, 0, "{row}: C sentinel");
            assert_eq!(c_result, rust_result, "{row}: differential sentinel");
        }
    }

    // E03: absent IDs return null for empty and populated tables.
    for populated in [false, true] {
        for _ in 0..RANDOM_CASES {
            let present_id = rng.i32();
            let missing_id = present_id.wrapping_add(1);
            unsafe {
                if populated {
                    let node = TreeNode::new(present_id, rng.i32(), -1, -1);
                    pair.c.install_nodes(&[node]);
                    pair.rust.install_nodes(&[node]);
                } else {
                    pair.reset();
                }
            }
            assert!(
                unsafe { (pair.c.find_node_by_id)(missing_id) }.is_null(),
                "E03: C"
            );
            assert!(
                unsafe { (pair.rust.find_node_by_id)(missing_id) }.is_null(),
                "E03: Rust"
            );
        }
    }

    // E04: exact and oversized capacity states reject without mutation.
    let label = CString::new("capacity").unwrap();
    for count in [MAX_NODES as c_int, MAX_NODES as c_int + 1, i32::MAX] {
        for _ in 0..RANDOM_CASES {
            unsafe {
                pair.reset();
                *pair.c.node_count = count;
                *pair.rust.node_count = count;
                ptr::write_bytes(pair.c.node_table, 0x5a, MAX_NODES);
                ptr::write_bytes(pair.rust.node_table, 0x5a, MAX_NODES);
            }
            let id = rng.i32();
            let value = rng.i32();
            let c_result = unsafe { (pair.c.add_tree_node)(id, value, -1, label.as_ptr()) };
            let rust_result = unsafe { (pair.rust.add_tree_node)(id, value, -1, label.as_ptr()) };
            assert_eq!(c_result, -1, "E04: C sentinel");
            assert_eq!(c_result, rust_result, "E04: differential sentinel");
            unsafe { pair.assert_state_equal("E04") };
        }
    }

    // E05: a missing parent rejects after writing the candidate slot.
    for _ in 0..RANDOM_CASES {
        unsafe { pair.reset() };
        let id = rng.i32();
        let parent_id = id.wrapping_add(1);
        let value = rng.i32();
        let c_result = unsafe { (pair.c.add_tree_node)(id, value, parent_id, label.as_ptr()) };
        let rust_result =
            unsafe { (pair.rust.add_tree_node)(id, value, parent_id, label.as_ptr()) };
        assert_eq!(c_result, -1, "E05: C sentinel");
        assert_eq!(c_result, rust_result, "E05: differential sentinel");
        assert_eq!(unsafe { pair.c.count() }, 0);
        unsafe { pair.assert_state_equal("E05") };
    }

    // E06: a missing sum root returns zero.
    for populated in [false, true] {
        for _ in 0..RANDOM_CASES {
            let present_id = rng.i32();
            let missing_id = present_id.wrapping_add(1);
            unsafe {
                if populated {
                    let node = TreeNode::new(present_id, rng.i32(), -1, -1);
                    pair.c.install_nodes(&[node]);
                    pair.rust.install_nodes(&[node]);
                } else {
                    pair.reset();
                }
            }
            let c_result = unsafe { (pair.c.calculate_tree_sum)(missing_id) };
            let rust_result = unsafe { (pair.rust.calculate_tree_sum)(missing_id) };
            assert_eq!(c_result, 0, "E06: C sentinel");
            assert_eq!(c_result, rust_result, "E06: differential sentinel");
        }
    }

    // E07: the parser's explicit null fallback.
    for _ in 0..RANDOM_CASES {
        let c_result = unsafe { (pair.c.parse_operation)(ptr::null()) };
        let rust_result = unsafe { (pair.rust.parse_operation)(ptr::null()) };
        assert_eq!(c_result, 1, "E07: C OP_ADD");
        assert_eq!(c_result, rust_result, "E07: differential fallback");
    }

    // E08: invalid C enum integers, including both adjacent boundaries.
    for operation in [0, 6, -1, i32::MIN, i32::MAX] {
        let c_function = unsafe { (pair.c.get_operation_func)(operation) };
        let rust_function = unsafe { (pair.rust.get_operation_func)(operation) };
        assert_eq!(
            c_function as usize, pair.c.add_op as usize,
            "E08: C pointer for {operation}"
        );
        assert_eq!(
            rust_function as usize, pair.rust.add_op as usize,
            "E08: Rust pointer for {operation}"
        );
        for _ in 0..RANDOM_CASES {
            let a = rng.bounded(30_000);
            let b = rng.bounded(30_000);
            unsafe {
                compare_operation(c_function, rust_function, a, b, rng.i32(), rng.i32(), "E08")
            };
        }
    }
}

#[test]
fn abnormal_boundary_behavior_matches() {
    use std::os::unix::process::ExitStatusExt;

    let _guard = LIBRARY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let executable = std::env::current_exe().unwrap();
    let run_probe = |implementation: &str, probe: &str| {
        Command::new(&executable)
            .args(["--exact", "abnormal_boundary_probe", "--nocapture"])
            .env("INREFTREE_ABNORMAL_IMPLEMENTATION", implementation)
            .env("INREFTREE_ABNORMAL_PROBE", probe)
            .status()
            .unwrap_or_else(|error| panic!("failed to run {implementation} {probe} probe: {error}"))
    };

    for (probe, row) in [
        ("null_label", "E09"),
        ("divide_overflow", "E10"),
        ("modulo_overflow", "E11"),
    ] {
        let c_status = run_probe("c", probe);
        let rust_status = run_probe("rust", probe);
        assert!(!c_status.success(), "{row}: C unexpectedly returned");
        assert!(!rust_status.success(), "{row}: Rust unexpectedly returned");
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "{row}: process signals differ"
        );
    }
}

#[test]
fn abnormal_boundary_probe() {
    let Ok(implementation) = std::env::var("INREFTREE_ABNORMAL_IMPLEMENTATION") else {
        return;
    };
    let probe = std::env::var("INREFTREE_ABNORMAL_PROBE").unwrap();
    let path = match implementation.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown abnormal probe implementation: {other}"),
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        api.reset();
        match probe.as_str() {
            "null_label" => {
                (api.add_tree_node)(1, 1, -1, ptr::null());
            }
            "divide_overflow" => {
                (api.divide_op)(i32::MIN, -1, 0, 0);
            }
            "modulo_overflow" => {
                (api.modulo_op)(i32::MIN, -1, 0, 0);
            }
            other => panic!("unknown abnormal probe: {other}"),
        }
    }
    panic!("{implementation} {probe} unexpectedly returned");
}
