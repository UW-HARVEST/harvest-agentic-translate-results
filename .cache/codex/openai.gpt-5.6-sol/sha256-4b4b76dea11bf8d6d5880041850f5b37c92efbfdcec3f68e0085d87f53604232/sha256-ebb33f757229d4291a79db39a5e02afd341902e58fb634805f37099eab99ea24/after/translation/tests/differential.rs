use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::slice;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());
const MAX_NODES: usize = 50;

#[repr(C)]
#[derive(Clone, Copy)]
struct TreeNode {
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: [c_char; 32],
}

type OpFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type FindFn = unsafe extern "C" fn(c_int) -> *mut TreeNode;
type AddNodeFn = unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int;
type SumFn = unsafe extern "C" fn(c_int) -> c_int;
type ParseFn = unsafe extern "C" fn(*const c_char) -> c_int;
type GetOpFn = unsafe extern "C" fn(c_int) -> OpFn;

struct Api {
    _library: Library,
    add_op: OpFn,
    multiply_op: OpFn,
    subtract_op: OpFn,
    divide_op: OpFn,
    modulo_op: OpFn,
    find_node_by_id: FindFn,
    add_tree_node: AddNodeFn,
    calculate_tree_sum: SumFn,
    parse_operation: ParseFn,
    get_operation_func: GetOpFn,
    inreftree: OpFn,
    node_table: *mut TreeNode,
    node_count: *mut c_int,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("load {}: {error}", path.display()));
        macro_rules! function {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("load symbol {}: {error}", $name))
            };
        }
        let node_table = *unsafe { library.get::<*mut TreeNode>(b"node_table\0") }
            .expect("load node_table");
        let node_count = *unsafe { library.get::<*mut c_int>(b"node_count\0") }
            .expect("load node_count");
        Self {
            add_op: function!("add_op", OpFn),
            multiply_op: function!("multiply_op", OpFn),
            subtract_op: function!("subtract_op", OpFn),
            divide_op: function!("divide_op", OpFn),
            modulo_op: function!("modulo_op", OpFn),
            find_node_by_id: function!("find_node_by_id", FindFn),
            add_tree_node: function!("add_tree_node", AddNodeFn),
            calculate_tree_sum: function!("calculate_tree_sum", SumFn),
            parse_operation: function!("parse_operation", ParseFn),
            get_operation_func: function!("get_operation_func", GetOpFn),
            inreftree: function!("inreftree", OpFn),
            node_table,
            node_count,
            _library: library,
        }
    }
}

fn root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation has parent")
        .to_path_buf()
}

fn c_library() -> PathBuf {
    root_dir()
        .join("c_src/build")
        .join("libharvest-work-9ODZOF.so")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release")
        .join("libinreftree_lib.so")
}

unsafe fn load_pair() -> (Api, Api) {
    (
        unsafe { Api::load(&c_library()) },
        unsafe { Api::load(&rust_library()) },
    )
}

unsafe fn reset(api: &Api) {
    unsafe {
        *api.node_count = 0;
        api.node_table.write_bytes(0, MAX_NODES);
    }
}

unsafe fn table_bytes(api: &Api) -> &[u8] {
    unsafe {
        slice::from_raw_parts(
            api.node_table.cast::<u8>(),
            MAX_NODES * size_of::<TreeNode>(),
        )
    }
}

unsafe fn assert_state_eq(c: &Api, rust: &Api, context: &str) {
    assert_eq!(
        unsafe { *c.node_count },
        unsafe { *rust.node_count },
        "{context}: node_count"
    );
    assert_eq!(
        unsafe { table_bytes(c) },
        unsafe { table_bytes(rust) },
        "{context}: node_table"
    );
}

unsafe fn call_add_both(
    c: &Api,
    rust: &Api,
    id: c_int,
    value: c_int,
    parent: c_int,
    label: &CString,
    context: &str,
) -> c_int {
    let c_result = unsafe { (c.add_tree_node)(id, value, parent, label.as_ptr()) };
    let rust_result = unsafe { (rust.add_tree_node)(id, value, parent, label.as_ptr()) };
    assert_eq!(c_result, rust_result, "{context}: return value");
    unsafe { assert_state_eq(c, rust, context) };
    c_result
}

#[derive(Clone)]
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

    fn range(&mut self, low: i32, high: i32) -> i32 {
        assert!(low < high);
        low + (self.next_u32() % (high - low) as u32) as i32
    }
}

fn label_for_shape(rng: &mut Rng, shape: usize) -> CString {
    let length = match shape {
        0 => 0,
        1 => rng.range(1, 31) as usize,
        2 => 31,
        3 => rng.range(32, 81) as usize,
        _ => unreachable!(),
    };
    let bytes: Vec<u8> = (0..length)
        .map(|_| b'a' + (rng.next_u32() % 26) as u8)
        .collect();
    CString::new(bytes).unwrap()
}

unsafe fn prepare_topology(c: &Api, rust: &Api, topology: usize) {
    unsafe {
        reset(c);
        reset(rust);
    }
    let root = CString::new("parent").unwrap();
    unsafe { call_add_both(c, rust, 100, 7, -1, &root, "topology root") };
    if topology >= 2 {
        let left = CString::new("left").unwrap();
        unsafe { call_add_both(c, rust, 101, 11, 100, &left, "topology left") };
    }
    if topology >= 3 {
        let right = CString::new("right").unwrap();
        unsafe { call_add_both(c, rust, 102, 13, 100, &right, "topology right") };
    }
}

#[test]
fn valid_configuration_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { load_pair() };
    let mut rng = Rng::new(0x5eed_cafe_d15c_a11e);

    // CONFIGS 1-5: arithmetic operations.
    for case in 0..512 {
        let a = rng.range(-1_000_000, 1_000_001);
        let b = rng.range(-1_000_000, 1_000_001);
        let u1 = rng.range(-10_000, 10_001);
        let u2 = rng.range(-10_000, 10_001);
        assert_eq!(
            unsafe { (c.add_op)(a, b, u1, u2) },
            unsafe { (rust.add_op)(a, b, u1, u2) },
            "CONFIG 1 case {case}"
        );

        let ma = rng.range(-30_000, 30_001);
        let mb = rng.range(-30_000, 30_001);
        assert_eq!(
            unsafe { (c.multiply_op)(ma, mb, u1, u2) },
            unsafe { (rust.multiply_op)(ma, mb, u1, u2) },
            "CONFIG 2 case {case}"
        );
        assert_eq!(
            unsafe { (c.subtract_op)(a, b, u1, u2) },
            unsafe { (rust.subtract_op)(a, b, u1, u2) },
            "CONFIG 3 case {case}"
        );

        let divisor = loop {
            let value = rng.range(-10_000, 10_001);
            if value != 0 {
                break value;
            }
        };
        assert_eq!(
            unsafe { (c.divide_op)(a, divisor, u1, u2) },
            unsafe { (rust.divide_op)(a, divisor, u1, u2) },
            "CONFIG 4 case {case}"
        );
        assert_eq!(
            unsafe { (c.modulo_op)(a, divisor, u1, u2) },
            unsafe { (rust.modulo_op)(a, divisor, u1, u2) },
            "CONFIG 5 case {case}"
        );
    }
    let add_boundaries = [
        (i32::MIN, 0),
        (i32::MIN, 1),
        (i32::MAX, 0),
        (i32::MAX, -1),
        (0, i32::MIN),
        (0, i32::MAX),
    ];
    let multiply_boundaries = [
        (i32::MIN, 0),
        (i32::MIN, 1),
        (i32::MAX, 0),
        (i32::MAX, 1),
        (-1, i32::MAX),
        (1, i32::MIN),
    ];
    let subtract_boundaries = [
        (i32::MIN, 0),
        (i32::MIN + 1, 1),
        (i32::MAX, 0),
        (i32::MAX - 1, -1),
        (0, i32::MAX),
        (0, i32::MIN + 1),
    ];
    let division_boundaries = [
        (i32::MIN, 1),
        (i32::MIN, 2),
        (i32::MAX, 1),
        (i32::MAX, -1),
        (0, i32::MIN),
        (1, i32::MAX),
    ];
    for (a, b) in add_boundaries {
        assert_eq!(
            unsafe { (c.add_op)(a, b, i32::MIN, i32::MAX) },
            unsafe { (rust.add_op)(a, b, i32::MIN, i32::MAX) },
            "CONFIG 1 integer boundary ({a}, {b})"
        );
    }
    for (a, b) in multiply_boundaries {
        assert_eq!(
            unsafe { (c.multiply_op)(a, b, i32::MIN, i32::MAX) },
            unsafe { (rust.multiply_op)(a, b, i32::MIN, i32::MAX) },
            "CONFIG 2 integer boundary ({a}, {b})"
        );
    }
    for (a, b) in subtract_boundaries {
        assert_eq!(
            unsafe { (c.subtract_op)(a, b, i32::MIN, i32::MAX) },
            unsafe { (rust.subtract_op)(a, b, i32::MIN, i32::MAX) },
            "CONFIG 3 integer boundary ({a}, {b})"
        );
    }
    for (a, b) in division_boundaries {
        assert_eq!(
            unsafe { (c.divide_op)(a, b, i32::MIN, i32::MAX) },
            unsafe { (rust.divide_op)(a, b, i32::MIN, i32::MAX) },
            "CONFIG 4 integer boundary ({a}, {b})"
        );
        assert_eq!(
            unsafe { (c.modulo_op)(a, b, i32::MIN, i32::MAX) },
            unsafe { (rust.modulo_op)(a, b, i32::MIN, i32::MAX) },
            "CONFIG 5 integer boundary ({a}, {b})"
        );
    }

    // CONFIGS 6-8: lookup positions in one/many-node tables.
    for case in 0..128 {
        unsafe {
            reset(&c);
            reset(&rust);
        }
        for id in 0..9 {
            let label = CString::new(format!("node-{case}-{id}")).unwrap();
            unsafe {
                call_add_both(
                    &c,
                    &rust,
                    1000 + id,
                    rng.range(-1000, 1001),
                    -1,
                    &label,
                    "lookup setup",
                )
            };
        }
        for (row, id) in [(6, 1000), (7, 1004), (8, 1008)] {
            let c_node = unsafe { (c.find_node_by_id)(id) };
            let rust_node = unsafe { (rust.find_node_by_id)(id) };
            assert!(!c_node.is_null() && !rust_node.is_null(), "CONFIG {row}");
            let c_offset = unsafe { c_node.offset_from(c.node_table) };
            let rust_offset = unsafe { rust_node.offset_from(rust.node_table) };
            assert_eq!(c_offset, rust_offset, "CONFIG {row} case {case}");
            assert_eq!(
                unsafe { slice::from_raw_parts(c_node.cast::<u8>(), size_of::<TreeNode>()) },
                unsafe { slice::from_raw_parts(rust_node.cast::<u8>(), size_of::<TreeNode>()) },
                "CONFIG {row} case {case}: node bytes"
            );
        }
    }

    // CONFIGS 9-24: parent topology crossed with all strncpy label shapes.
    for topology in 0..4 {
        for shape in 0..4 {
            let row = 9 + topology * 4 + shape;
            for case in 0..96 {
                unsafe { prepare_topology(&c, &rust, topology) };
                let label = label_for_shape(&mut rng, shape);
                let parent = if topology == 0 { -1 } else { 100 };
                let result = unsafe {
                    call_add_both(
                        &c,
                        &rust,
                        200 + case,
                        rng.range(-100_000, 100_001),
                        parent,
                        &label,
                        &format!("CONFIG {row} case {case}"),
                    )
                };
                assert!(result >= 0, "CONFIG {row} case {case}");
            }
        }
    }

    // CONFIGS 25-29: leaf, left-only, right-only, two-child, and nested sums.
    for case in 0..128 {
        let values = [
            rng.range(-10_000, 10_001),
            rng.range(-10_000, 10_001),
            rng.range(-10_000, 10_001),
            rng.range(-10_000, 10_001),
        ];
        for shape in 0..5 {
            unsafe {
                reset(&c);
                reset(&rust);
            }
            let root = CString::new("root").unwrap();
            unsafe { call_add_both(&c, &rust, 1, values[0], -1, &root, "sum root") };
            if shape >= 1 {
                let child = CString::new("child").unwrap();
                unsafe { call_add_both(&c, &rust, 2, values[1], 1, &child, "sum left") };
            }
            if shape == 2 {
                unsafe {
                    (*c.node_table).left_child_id = -1;
                    (*c.node_table).right_child_id = 2;
                    (*rust.node_table).left_child_id = -1;
                    (*rust.node_table).right_child_id = 2;
                }
            } else if shape >= 3 {
                let right = CString::new("right").unwrap();
                unsafe { call_add_both(&c, &rust, 3, values[2], 1, &right, "sum right") };
            }
            if shape == 4 {
                let nested = CString::new("nested").unwrap();
                unsafe { call_add_both(&c, &rust, 4, values[3], 2, &nested, "sum nested") };
            }
            let row = 25 + shape;
            assert_eq!(
                unsafe { (c.calculate_tree_sum)(1) },
                unsafe { (rust.calculate_tree_sum)(1) },
                "CONFIG {row} case {case}"
            );
        }
    }

    // CONFIGS 30-35: operation parsing, including precedence combinations.
    let parse_cases = [
        (30, vec!["+", "abc+xyz", "%/-*+"]),
        (31, vec!["*", "abc*xyz", "%/-*"]),
        (32, vec!["-", "abc-xyz", "%/-"]),
        (33, vec!["/", "abc/xyz", "%/"]),
        (34, vec!["%", "abc%xyz", "%%%"]),
        (35, vec!["", "abc", "operation"]),
    ];
    for (row, strings) in parse_cases {
        for repeat in 0..128 {
            for text in &strings {
                let decorated = format!("{}{}{}", "x".repeat(repeat % 7), text, repeat);
                let value = CString::new(decorated).unwrap();
                assert_eq!(
                    unsafe { (c.parse_operation)(value.as_ptr()) },
                    unsafe { (rust.parse_operation)(value.as_ptr()) },
                    "CONFIG {row} repeat {repeat}, text {text:?}"
                );
            }
        }
    }

    // CONFIGS 36-40: every valid enum value and the returned callback.
    for op in 1..=5 {
        let row = 35 + op;
        let c_function = unsafe { (c.get_operation_func)(op) };
        let rust_function = unsafe { (rust.get_operation_func)(op) };
        for case in 0..256 {
            let a = rng.range(-20_000, 20_001);
            let mut b = rng.range(-20_000, 20_001);
            if op >= 4 && b == 0 {
                b = 1;
            }
            assert_eq!(
                unsafe { c_function(a, b, case, -case) },
                unsafe { rust_function(a, b, case, -case) },
                "CONFIG {row} case {case}"
            );
        }
    }

    // CONFIGS 41-48: target selection crossed with every nonnegative remainder.
    for target_fallback in [false, true] {
        for remainder in 0..4 {
            let row = 41 + usize::from(target_fallback) * 4 + remainder as usize;
            for case in 0..256 {
                let p1 = rng.range(0, 1001);
                let p2 = if target_fallback {
                    0
                } else {
                    rng.range(1, 1001)
                };
                let p3 = rng.range(0, 1001);
                let mut p4 = rng.range(0, 1001);
                let total = p1 + p2 + p3 + p4;
                p4 += (remainder - total.rem_euclid(4)).rem_euclid(4);
                assert_eq!(
                    unsafe { (c.inreftree)(p1, p2, p3, p4) },
                    unsafe { (rust.inreftree)(p1, p2, p3, p4) },
                    "CONFIG {row} case {case}"
                );
                unsafe { assert_state_eq(&c, &rust, &format!("CONFIG {row} case {case}")) };
            }
        }
    }

    // CONFIGS 49-54: both target modes crossed with C remainders -1, -2, -3.
    for target_fallback in [false, true] {
        for negative_remainder in -3..=-1 {
            let row = 49
                + usize::from(target_fallback) * 3
                + (negative_remainder + 3) as usize;
            for case in 0..256 {
                let p2 = if target_fallback {
                    0
                } else {
                    rng.range(1, 1001)
                };
                let p3 = rng.range(-1000, 1001);
                let p4 = rng.range(-1000, 1001);
                let desired_total = negative_remainder - 4 * rng.range(1, 1001);
                let p1 = desired_total - p2 - p3 - p4;
                assert!(p1 + p2 + p3 + p4 < 0);
                assert_eq!((p1 + p2 + p3 + p4) % 4, negative_remainder);
                assert_eq!(
                    unsafe { (c.inreftree)(p1, p2, p3, p4) },
                    unsafe { (rust.inreftree)(p1, p2, p3, p4) },
                    "CONFIG {row} case {case}"
                );
                unsafe { assert_state_eq(&c, &rust, &format!("CONFIG {row} case {case}")) };
            }
        }
    }

    // CONFIGS 55-56: direct exported-global observation at zero/one/many counts.
    unsafe {
        reset(&c);
        reset(&rust);
        assert_state_eq(&c, &rust, "CONFIG 55 reset");
    }
    for count in 1..=20 {
        let label = CString::new(format!("global-{count}")).unwrap();
        unsafe {
            call_add_both(
                &c,
                &rust,
                count,
                rng.range(-1000, 1001),
                -1,
                &label,
                &format!("CONFIG 56 count {count}"),
            )
        };
    }
}

#[test]
fn error_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { load_pair() };
    let mut rng = Rng::new(0xbad5_eed0_0ddc_0ffe);

    // ERRORS 1-2: divide/modulo by zero return the exact sentinel.
    for case in 0..512 {
        let a = rng.range(-1_000_000, 1_000_001);
        assert_eq!(unsafe { (c.divide_op)(a, 0, case, -case) }, 0);
        assert_eq!(
            unsafe { (c.divide_op)(a, 0, case, -case) },
            unsafe { (rust.divide_op)(a, 0, case, -case) },
            "ERROR 1 case {case}"
        );
        assert_eq!(unsafe { (c.modulo_op)(a, 0, case, -case) }, 0);
        assert_eq!(
            unsafe { (c.modulo_op)(a, 0, case, -case) },
            unsafe { (rust.modulo_op)(a, 0, case, -case) },
            "ERROR 2 case {case}"
        );
    }

    // ERROR 3: missing lookup in empty and populated tables returns NULL.
    unsafe {
        reset(&c);
        reset(&rust);
    }
    assert!(unsafe { (c.find_node_by_id)(123) }.is_null());
    assert!(unsafe { (rust.find_node_by_id)(123) }.is_null());
    for id in 0..10 {
        let label = CString::new(format!("present-{id}")).unwrap();
        unsafe { call_add_both(&c, &rust, id, id * 2, -1, &label, "missing setup") };
    }
    for missing in [-10, 10, 11, i32::MAX] {
        assert!(unsafe { (c.find_node_by_id)(missing) }.is_null());
        assert!(unsafe { (rust.find_node_by_id)(missing) }.is_null());
    }

    // ERROR 4: exact and oversized node_count capacity boundaries.
    unsafe {
        reset(&c);
        reset(&rust);
    }
    for id in 0..MAX_NODES as i32 {
        let label = CString::new(format!("full-{id}")).unwrap();
        unsafe { call_add_both(&c, &rust, id, id, -1, &label, "capacity fill") };
    }
    let c_before = unsafe { table_bytes(&c) }.to_vec();
    let rust_before = unsafe { table_bytes(&rust) }.to_vec();
    let label = CString::new("overflow").unwrap();
    for count in [50, 51, i32::MAX] {
        unsafe {
            *c.node_count = count;
            *rust.node_count = count;
        }
        assert_eq!(
            unsafe { (c.add_tree_node)(999, 999, -1, label.as_ptr()) },
            -1,
            "C ERROR 4 count {count}"
        );
        assert_eq!(
            unsafe { (rust.add_tree_node)(999, 999, -1, label.as_ptr()) },
            -1,
            "Rust ERROR 4 count {count}"
        );
        assert_eq!(unsafe { table_bytes(&c) }, c_before, "C ERROR 4 table");
        assert_eq!(
            unsafe { table_bytes(&rust) },
            rust_before,
            "Rust ERROR 4 table"
        );
    }

    // ERROR 5: missing parent leaves count unchanged but writes the staged slot.
    for case in 0..128 {
        unsafe {
            reset(&c);
            reset(&rust);
        }
        let root = CString::new("root").unwrap();
        unsafe { call_add_both(&c, &rust, 1, 5, -1, &root, "missing-parent root") };
        let candidate = label_for_shape(&mut rng, case % 4);
        assert_eq!(
            unsafe { (c.add_tree_node)(10 + case as i32, 99, 777, candidate.as_ptr()) },
            -1,
            "C ERROR 5 case {case}"
        );
        assert_eq!(
            unsafe { (rust.add_tree_node)(10 + case as i32, 99, 777, candidate.as_ptr()) },
            -1,
            "Rust ERROR 5 case {case}"
        );
        assert_eq!(unsafe { *c.node_count }, 1);
        unsafe { assert_state_eq(&c, &rust, &format!("ERROR 5 case {case}")) };
    }

    // ERROR 6: missing sum root returns the exact zero sentinel.
    unsafe {
        reset(&c);
        reset(&rust);
    }
    for id in [0, -1, 1, i32::MIN, i32::MAX] {
        assert_eq!(unsafe { (c.calculate_tree_sum)(id) }, 0);
        assert_eq!(
            unsafe { (c.calculate_tree_sum)(id) },
            unsafe { (rust.calculate_tree_sum)(id) },
            "ERROR 6 id {id}"
        );
    }

    // ERROR 7: parse_operation explicitly accepts NULL as OP_ADD.
    assert_eq!(unsafe { (c.parse_operation)(std::ptr::null()) }, 1);
    assert_eq!(
        unsafe { (c.parse_operation)(std::ptr::null()) },
        unsafe { (rust.parse_operation)(std::ptr::null()) },
        "ERROR 7"
    );

    // ERROR 8: all representative invalid enum values default to add_op.
    for op in [i32::MIN, -100, -1, 0, 6, 100, i32::MAX] {
        let c_function = unsafe { (c.get_operation_func)(op) };
        let rust_function = unsafe { (rust.get_operation_func)(op) };
        for case in 0..128 {
            let a = rng.range(-1_000_000, 1_000_001);
            let b = rng.range(-1_000_000, 1_000_001);
            let expected = unsafe { (c.add_op)(a, b, case, -case) };
            assert_eq!(unsafe { c_function(a, b, case, -case) }, expected);
            assert_eq!(
                unsafe { c_function(a, b, case, -case) },
                unsafe { rust_function(a, b, case, -case) },
                "ERROR 8 op {op}, case {case}"
            );
        }
    }
}

fn run_null_label_child(which: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("child_null_label")
        .arg("--nocapture")
        .env("DIFFERENTIAL_NULL_LABEL_LIBRARY", which)
        .status()
        .unwrap_or_else(|error| panic!("run {which} null-label child: {error}"))
}

#[test]
fn null_label_rejection_matches() {
    use std::os::unix::process::ExitStatusExt;

    let _guard = TEST_LOCK.lock().unwrap();
    let c_status = run_null_label_child("c");
    let rust_status = run_null_label_child("rust");
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "ERROR 9: C status {c_status:?}, Rust status {rust_status:?}"
    );
    assert!(
        c_status.signal().is_some(),
        "ERROR 9 should be a fatal memory-access rejection: {c_status:?}"
    );
}

#[test]
fn child_null_label() {
    let Some(which) = env::var_os("DIFFERENTIAL_NULL_LABEL_LIBRARY") else {
        return;
    };
    let path = if which == "c" {
        c_library()
    } else {
        rust_library()
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        reset(&api);
        (api.add_tree_node)(1, 1, -1, std::ptr::null());
    }
    panic!("null label unexpectedly returned");
}
