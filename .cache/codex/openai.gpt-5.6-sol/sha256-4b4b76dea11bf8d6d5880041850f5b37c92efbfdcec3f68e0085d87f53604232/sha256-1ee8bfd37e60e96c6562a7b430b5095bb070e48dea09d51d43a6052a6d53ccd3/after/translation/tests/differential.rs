use libloading::Library;
use std::ffi::{CString, c_char, c_int};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type OpFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type FindFn = unsafe extern "C" fn(c_int) -> *mut TreeNode;
type AddNodeFn = unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int;
type SumFn = unsafe extern "C" fn(c_int) -> c_int;
type ParseFn = unsafe extern "C" fn(*const c_char) -> c_int;
type GetOpFn = unsafe extern "C" fn(c_int) -> OpFn;

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
    node_count: *mut c_int,
    node_table: *mut TreeNode,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        unsafe {
            Self {
                add_op: *library.get(b"add_op\0").unwrap(),
                multiply_op: *library.get(b"multiply_op\0").unwrap(),
                subtract_op: *library.get(b"subtract_op\0").unwrap(),
                divide_op: *library.get(b"divide_op\0").unwrap(),
                modulo_op: *library.get(b"modulo_op\0").unwrap(),
                find_node_by_id: *library.get(b"find_node_by_id\0").unwrap(),
                add_tree_node: *library.get(b"add_tree_node\0").unwrap(),
                calculate_tree_sum: *library.get(b"calculate_tree_sum\0").unwrap(),
                parse_operation: *library.get(b"parse_operation\0").unwrap(),
                get_operation_func: *library.get(b"get_operation_func\0").unwrap(),
                inreftree: *library.get(b"inreftree\0").unwrap(),
                node_count: *library.get(b"node_count\0").unwrap(),
                node_table: *library.get(b"node_table\0").unwrap(),
                _library: library,
            }
        }
    }

    unsafe fn reset(&self) {
        unsafe { *self.node_count = 0 };
    }

    unsafe fn count(&self) -> c_int {
        unsafe { *self.node_count }
    }

    unsafe fn snapshot(&self, count: usize) -> Vec<u8> {
        let byte_len = count * size_of::<TreeNode>();
        unsafe { std::slice::from_raw_parts(self.node_table.cast::<u8>(), byte_len).to_vec() }
    }
}

struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn range(&mut self, low: c_int, high: c_int) -> c_int {
        low + (self.next_u32() % (high - low) as u32) as c_int
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-BiOJI9.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release")
        .join(format!(
            "{}inreftree_lib{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ))
}

unsafe fn load_pair() -> (Api, Api) {
    let c = unsafe { Api::load(&c_library_path()) };
    let rust = unsafe { Api::load(&rust_library_path()) };
    (c, rust)
}

fn random_label(rng: &mut Rng, len: usize) -> CString {
    let bytes: Vec<u8> = (0..len)
        .map(|_| b'a' + (rng.next_u32() % 26) as u8)
        .collect();
    CString::new(bytes).unwrap()
}

unsafe fn add_both(
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
    assert_eq!(rust_result, c_result, "{context}: return");
    assert_eq!(
        unsafe { rust.count() },
        unsafe { c.count() },
        "{context}: count"
    );
    let initialized = usize::try_from(unsafe { c.count() }).unwrap_or(0);
    assert_eq!(
        unsafe { rust.snapshot(initialized) },
        unsafe { c.snapshot(initialized) },
        "{context}: table"
    );
    c_result
}

unsafe fn write_nodes(api: &Api, nodes: &[TreeNode]) {
    for (index, node) in nodes.iter().enumerate() {
        unsafe { api.node_table.add(index).write(*node) };
    }
    unsafe { *api.node_count = nodes.len() as c_int };
}

fn node(id: c_int, value: c_int, left: c_int, right: c_int) -> TreeNode {
    TreeNode {
        id,
        value,
        parent_id: -1,
        left_child_id: left,
        right_child_id: right,
        label: [0; 32],
    }
}

unsafe fn test_arithmetic(c: &Api, rust: &Api, rng: &mut Rng) {
    // CONFIGS 1-5; ERRORS 1-2 and arithmetic portions of 10.
    let boundaries = [
        (0, 0),
        (c_int::MAX, 0),
        (c_int::MIN, 0),
        (c_int::MAX, -1),
        (c_int::MIN, 1),
    ];
    for &(a, b) in &boundaries {
        assert_eq!(
            unsafe { (rust.add_op)(a, b, 17, -29) },
            unsafe { (c.add_op)(a, b, 17, -29) },
            "CONFIGS 1 boundary"
        );
    }

    for _ in 0..512 {
        let a = rng.range(-1_000_000, 1_000_001);
        let b = rng.range(-1_000_000, 1_000_001);
        let ignored1 = rng.next_u32() as c_int;
        let ignored2 = rng.next_u32() as c_int;
        assert_eq!(
            unsafe { (rust.add_op)(a, b, ignored1, ignored2) },
            unsafe { (c.add_op)(a, b, ignored1, ignored2) },
            "CONFIGS 1"
        );
        assert_eq!(
            unsafe { (rust.subtract_op)(a, b, ignored1, ignored2) },
            unsafe { (c.subtract_op)(a, b, ignored1, ignored2) },
            "CONFIGS 3"
        );

        let ma = rng.range(-30_000, 30_001);
        let mb = rng.range(-30_000, 30_001);
        assert_eq!(
            unsafe { (rust.multiply_op)(ma, mb, ignored1, ignored2) },
            unsafe { (c.multiply_op)(ma, mb, ignored1, ignored2) },
            "CONFIGS 2"
        );

        let divisor = loop {
            let value = rng.range(-100_000, 100_001);
            if value != 0 {
                break value;
            }
        };
        assert_eq!(
            unsafe { (rust.divide_op)(a, divisor, ignored1, ignored2) },
            unsafe { (c.divide_op)(a, divisor, ignored1, ignored2) },
            "CONFIGS 4"
        );
        assert_eq!(
            unsafe { (rust.modulo_op)(a, divisor, ignored1, ignored2) },
            unsafe { (c.modulo_op)(a, divisor, ignored1, ignored2) },
            "CONFIGS 5"
        );

        assert_eq!(
            unsafe { (rust.divide_op)(a, 0, ignored1, ignored2) },
            unsafe { (c.divide_op)(a, 0, ignored1, ignored2) },
            "ERRORS 1"
        );
        assert_eq!(
            unsafe { (rust.modulo_op)(a, 0, ignored1, ignored2) },
            unsafe { (c.modulo_op)(a, 0, ignored1, ignored2) },
            "ERRORS 2"
        );
    }
}

unsafe fn test_nodes(c: &Api, rust: &Api, rng: &mut Rng) {
    // CONFIGS 6-17 and ERRORS 3-5.
    unsafe {
        c.reset();
        rust.reset();
    }
    assert_eq!(unsafe { c.count() }, 0, "CONFIGS 6 C empty");
    assert_eq!(unsafe { rust.count() }, 0, "CONFIGS 6 Rust empty");
    for &missing in &[0, -1, c_int::MIN, c_int::MAX] {
        assert!(
            unsafe { (c.find_node_by_id)(missing) }.is_null(),
            "ERRORS 3 C"
        );
        assert!(
            unsafe { (rust.find_node_by_id)(missing) }.is_null(),
            "ERRORS 3 Rust"
        );
    }

    for &(row, len) in &[(10, 0), (11, 12), (12, 31), (13, 57)] {
        for _ in 0..64 {
            unsafe {
                c.reset();
                rust.reset();
            }
            let label = random_label(rng, len);
            unsafe {
                add_both(
                    c,
                    rust,
                    rng.next_u32() as c_int,
                    rng.next_u32() as c_int,
                    -1,
                    &label,
                    &format!("CONFIGS {row}"),
                )
            };
        }
    }

    for _ in 0..128 {
        unsafe {
            c.reset();
            rust.reset();
        }
        let label = random_label(rng, 8);
        let ids = [
            rng.range(1, 10_000),
            rng.range(10_001, 20_000),
            rng.range(20_001, 30_000),
            rng.range(30_001, 40_000),
        ];
        unsafe { add_both(c, rust, ids[0], 1, -1, &label, "CONFIGS 7") };
        let c_found = unsafe { (c.find_node_by_id)(ids[0]) };
        let r_found = unsafe { (rust.find_node_by_id)(ids[0]) };
        assert_eq!(
            unsafe { c_found.offset_from(c.node_table) },
            unsafe { r_found.offset_from(rust.node_table) },
            "CONFIGS 7"
        );

        unsafe { add_both(c, rust, ids[1], 2, -1, &label, "CONFIGS 8") };
        unsafe { add_both(c, rust, ids[2], 3, -1, &label, "CONFIGS 8") };
        for &id in &ids[..3] {
            let cp = unsafe { (c.find_node_by_id)(id) };
            let rp = unsafe { (rust.find_node_by_id)(id) };
            assert_eq!(
                unsafe { cp.offset_from(c.node_table) },
                unsafe { rp.offset_from(rust.node_table) },
                "CONFIGS 8"
            );
        }

        unsafe { add_both(c, rust, ids[0], 4, -1, &label, "CONFIGS 9,17") };
        let cp = unsafe { (c.find_node_by_id)(ids[0]) };
        let rp = unsafe { (rust.find_node_by_id)(ids[0]) };
        assert_eq!(unsafe { cp.offset_from(c.node_table) }, 0, "CONFIGS 9 C");
        assert_eq!(
            unsafe { rp.offset_from(rust.node_table) },
            0,
            "CONFIGS 9 Rust"
        );
    }

    for _ in 0..128 {
        unsafe {
            c.reset();
            rust.reset();
        }
        let label = random_label(rng, 15);
        let root = rng.range(1, 100_000);
        unsafe { add_both(c, rust, root, 1, -1, &label, "CONFIGS 14") };
        unsafe { add_both(c, rust, root + 1, 2, root, &label, "CONFIGS 14") };
        unsafe { add_both(c, rust, root + 2, 3, root, &label, "CONFIGS 15") };
        unsafe { add_both(c, rust, root + 3, 4, root, &label, "CONFIGS 16") };
    }

    unsafe {
        c.reset();
        rust.reset();
    }
    let label = CString::new("missing-parent").unwrap();
    let c_result = unsafe { (c.add_tree_node)(5, 99, 404, label.as_ptr()) };
    let r_result = unsafe { (rust.add_tree_node)(5, 99, 404, label.as_ptr()) };
    assert_eq!(r_result, c_result, "ERRORS 5 return");
    assert_eq!(c_result, -1, "ERRORS 5 sentinel");
    assert_eq!(
        unsafe { rust.count() },
        unsafe { c.count() },
        "ERRORS 5 count"
    );
    assert_eq!(
        unsafe { rust.snapshot(1) },
        unsafe { c.snapshot(1) },
        "ERRORS 5 partial slot"
    );

    unsafe {
        *c.node_count = 50;
        *rust.node_count = 50;
    }
    let c_before = unsafe { c.snapshot(50) };
    let r_before = unsafe { rust.snapshot(50) };
    assert_eq!(
        unsafe { (rust.add_tree_node)(1, 2, -1, label.as_ptr()) },
        unsafe { (c.add_tree_node)(1, 2, -1, label.as_ptr()) },
        "ERRORS 4 return"
    );
    assert_eq!(unsafe { c.snapshot(50) }, c_before, "ERRORS 4 C unchanged");
    assert_eq!(
        unsafe { rust.snapshot(50) },
        r_before,
        "ERRORS 4 Rust unchanged"
    );
}

unsafe fn test_sums(c: &Api, rust: &Api, rng: &mut Rng) {
    // CONFIGS 18-22 and ERRORS 6,10.
    for _ in 0..256 {
        let values = [
            rng.range(-100_000, 100_001),
            rng.range(-100_000, 100_001),
            rng.range(-100_000, 100_001),
            rng.range(-100_000, 100_001),
        ];
        let shapes = [
            vec![node(1, values[0], -1, -1)],
            vec![node(1, values[0], 2, -1), node(2, values[1], -1, -1)],
            vec![node(1, values[0], -1, 2), node(2, values[1], -1, -1)],
            vec![
                node(1, values[0], 2, 3),
                node(2, values[1], -1, -1),
                node(3, values[2], -1, -1),
            ],
            vec![
                node(1, values[0], 2, 3),
                node(2, values[1], 4, -1),
                node(3, values[2], -1, -1),
                node(4, values[3], -1, -1),
            ],
        ];
        for (offset, nodes) in shapes.iter().enumerate() {
            unsafe {
                write_nodes(c, nodes);
                write_nodes(rust, nodes);
            }
            assert_eq!(
                unsafe { (rust.calculate_tree_sum)(1) },
                unsafe { (c.calculate_tree_sum)(1) },
                "CONFIGS {}",
                18 + offset
            );
        }
    }

    unsafe {
        c.reset();
        rust.reset();
    }
    for &id in &[0, -1, c_int::MIN, c_int::MAX] {
        assert_eq!(
            unsafe { (rust.calculate_tree_sum)(id) },
            unsafe { (c.calculate_tree_sum)(id) },
            "ERRORS 6,10"
        );
    }
    let extreme = [node(c_int::MIN, c_int::MIN, -1, -1)];
    unsafe {
        write_nodes(c, &extreme);
        write_nodes(rust, &extreme);
    }
    assert_eq!(
        unsafe { (rust.calculate_tree_sum)(c_int::MIN) },
        unsafe { (c.calculate_tree_sum)(c_int::MIN) },
        "ERRORS 10 extreme ID/value"
    );
}

unsafe fn test_parsing_and_dispatch(c: &Api, rust: &Api, rng: &mut Rng) {
    // CONFIGS 23-33 and ERRORS 7-8.
    for _ in 0..256 {
        let token = random_label(rng, 12).into_string().unwrap();
        let cases = [
            (format!("{token}%/-*+"), 1, 23),
            (format!("{token}%-*"), 2, 24),
            (format!("{token}%/-"), 3, 25),
            (format!("{token}%/"), 4, 26),
            (format!("{token}%"), 5, 27),
            (token, 1, 28),
        ];
        for (text, expected, row) in cases {
            let string = CString::new(text).unwrap();
            let c_result = unsafe { (c.parse_operation)(string.as_ptr()) };
            let r_result = unsafe { (rust.parse_operation)(string.as_ptr()) };
            assert_eq!(c_result, expected, "CONFIGS {row} C");
            assert_eq!(r_result, c_result, "CONFIGS {row} Rust");
        }
    }
    assert_eq!(
        unsafe { (rust.parse_operation)(std::ptr::null()) },
        unsafe { (c.parse_operation)(std::ptr::null()) },
        "ERRORS 7"
    );

    for op in 1..=5 {
        for _ in 0..256 {
            let mut a = rng.range(-30_000, 30_001);
            let mut b = rng.range(-30_000, 30_001);
            if op >= 4 && b == 0 {
                b = 1;
            }
            if op == 2 {
                a = rng.range(-20_000, 20_001);
                b = rng.range(-20_000, 20_001);
            }
            let c_func = unsafe { (c.get_operation_func)(op) };
            let r_func = unsafe { (rust.get_operation_func)(op) };
            assert_eq!(
                unsafe { r_func(a, b, 7, 9) },
                unsafe { c_func(a, b, 7, 9) },
                "CONFIGS {}",
                28 + op
            );
        }
    }

    for &invalid in &[c_int::MIN, -1, 0, 6, c_int::MAX] {
        let c_func = unsafe { (c.get_operation_func)(invalid) };
        let r_func = unsafe { (rust.get_operation_func)(invalid) };
        for _ in 0..128 {
            let a = rng.range(-1_000_000, 1_000_001);
            let b = rng.range(-1_000_000, 1_000_001);
            assert_eq!(
                unsafe { r_func(a, b, 0, 0) },
                unsafe { c_func(a, b, 0, 0) },
                "ERRORS 8"
            );
        }
    }
}

unsafe fn test_inreftree(c: &Api, rust: &Api, rng: &mut Rng) {
    // CONFIGS 34-41. Inputs keep all C arithmetic and indexing defined.
    for param2_is_zero in [false, true] {
        for residue in 0..4 {
            for _ in 0..256 {
                let p1 = rng.range(0, 10_001);
                let p2 = if param2_is_zero {
                    0
                } else {
                    rng.range(1, 10_001)
                };
                let p3 = rng.range(0, 10_001);
                let p4_base = rng.range(0, 10_001);
                let sum = p1 + p2 + p3 + p4_base;
                let p4 = p4_base + (residue - sum.rem_euclid(4)).rem_euclid(4);
                let c_result = unsafe { (c.inreftree)(p1, p2, p3, p4) };
                let r_result = unsafe { (rust.inreftree)(p1, p2, p3, p4) };
                let row = if param2_is_zero {
                    38 + residue
                } else {
                    34 + residue
                };
                assert_eq!(r_result, c_result, "CONFIGS {row}");
                assert_eq!(
                    unsafe { rust.count() },
                    unsafe { c.count() },
                    "row {row} count"
                );
                assert_eq!(
                    unsafe { rust.snapshot(4) },
                    unsafe { c.snapshot(4) },
                    "row {row} table"
                );
            }
        }
    }

    // The largest nonnegative sum is a defined boundary for every integer API.
    assert_eq!(
        unsafe { (rust.inreftree)(c_int::MAX, 0, 0, 0) },
        unsafe { (c.inreftree)(c_int::MAX, 0, 0, 0) },
        "ERRORS 10 inreftree INT_MAX"
    );
}

fn test_null_label_crash() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().unwrap();
    let run = |library: &Path| {
        Command::new(&executable)
            .args(["--exact", "null_label_probe", "--nocapture"])
            .env("NULL_LABEL_PROBE_LIBRARY", library)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
    };
    let c_status = run(&c_library_path());
    let rust_status = run(&rust_library_path());
    assert!(
        c_status.signal().is_some(),
        "ERRORS 9: C unexpectedly returned {c_status}"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "ERRORS 9: null-label termination signal"
    );
}

#[test]
fn differential_surface() {
    unsafe {
        let (c, rust) = load_pair();
        let mut rng = Rng(0x8d26_3f4a_91c7_b5e1);
        test_arithmetic(&c, &rust, &mut rng);
        test_nodes(&c, &rust, &mut rng);
        test_sums(&c, &rust, &mut rng);
        test_parsing_and_dispatch(&c, &rust, &mut rng);
        test_inreftree(&c, &rust, &mut rng);
    }
    test_null_label_crash();
}

#[test]
fn null_label_probe() {
    let Some(path) = std::env::var_os("NULL_LABEL_PROBE_LIBRARY") else {
        return;
    };
    unsafe {
        let api = Api::load(Path::new(&path));
        api.reset();
        (api.add_tree_node)(1, 2, -1, std::ptr::null());
    }
    panic!("null label unexpectedly returned");
}
