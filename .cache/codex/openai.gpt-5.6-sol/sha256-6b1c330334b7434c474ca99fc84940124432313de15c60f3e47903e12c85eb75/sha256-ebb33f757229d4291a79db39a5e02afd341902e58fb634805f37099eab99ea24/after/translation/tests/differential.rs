use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const MAX_NAME_LEN: usize = 50;

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
type MaxNMin = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    library: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        Self {
            library: unsafe { Library::new(path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    unsafe fn add_node(
        &self,
        id: c_int,
        parent_id: c_int,
        name: *const c_char,
        value: c_double,
    ) -> c_int {
        let function: Symbol<AddNode> = unsafe { self.library.get(b"add_node\0") }.unwrap();
        unsafe { function(id, parent_id, name, value) }
    }

    unsafe fn find_node(&self, id: c_int) -> *mut Node {
        let function: Symbol<FindNode> = unsafe { self.library.get(b"find_node_by_id\0") }.unwrap();
        unsafe { function(id) }
    }

    unsafe fn children_count(&self, parent_id: c_int) -> c_int {
        let function: Symbol<ChildrenCount> =
            unsafe { self.library.get(b"get_children_count\0") }.unwrap();
        unsafe { function(parent_id) }
    }

    unsafe fn subtree_sum(&self, node_id: c_int) -> c_double {
        let function: Symbol<SubtreeSum> =
            unsafe { self.library.get(b"calculate_subtree_sum\0") }.unwrap();
        unsafe { function(node_id) }
    }

    unsafe fn process_string(&self, string: *mut c_char) -> c_int {
        let function: Symbol<ProcessString> =
            unsafe { self.library.get(b"process_string\0") }.unwrap();
        unsafe { function(string) }
    }

    unsafe fn double_to_int(&self, value: c_double) -> c_int {
        let function: Symbol<DoubleToInt> =
            unsafe { self.library.get(b"safe_double_to_int\0") }.unwrap();
        unsafe { function(value) }
    }

    unsafe fn maxnmin(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        let function: Symbol<MaxNMin> = unsafe { self.library.get(b"maxnmin\0") }.unwrap();
        unsafe { function(a, b, c, d) }
    }
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

    fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn bounded(&mut self, upper: u32) -> u32 {
        self.next_u32() % upper
    }

    fn finite_value(&mut self) -> f64 {
        (self.i32() as f64) / 65536.0
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    libraries.sort();
    assert_eq!(libraries.len(), 1, "expected exactly one C shared library");
    libraries.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libmaxnmin_lib.so")
}

fn with_apis(test: impl FnOnce(&Api, &Api)) {
    unsafe {
        let c = Api::open(&c_library_path());
        let rust = Api::open(&rust_library_path());
        test(&c, &rust);
    }
}

fn c_string(bytes: &[u8]) -> Vec<c_char> {
    let mut result: Vec<c_char> = bytes.iter().map(|byte| *byte as c_char).collect();
    result.push(0);
    result
}

unsafe fn assert_nodes_equal(c_node: *mut Node, rust_node: *mut Node, context: &str) {
    assert_eq!(c_node.is_null(), rust_node.is_null(), "{context}: nullness");
    if c_node.is_null() {
        return;
    }
    let c = unsafe { &*c_node };
    let rust = unsafe { &*rust_node };
    assert_eq!(c.id, rust.id, "{context}: id");
    assert_eq!(c.parent_id, rust.parent_id, "{context}: parent_id");
    assert_eq!(c.name, rust.name, "{context}: name bytes");
    assert_eq!(c.value.to_bits(), rust.value.to_bits(), "{context}: value");
    assert_eq!(c.active, rust.active, "{context}: active");
}

unsafe fn add_both(
    c: &Api,
    rust: &Api,
    id: i32,
    parent: i32,
    name: &[u8],
    value: f64,
    context: &str,
) -> i32 {
    let name = c_string(name);
    let c_result = unsafe { c.add_node(id, parent, name.as_ptr(), value) };
    let rust_result = unsafe { rust.add_node(id, parent, name.as_ptr(), value) };
    assert_eq!(c_result, rust_result, "{context}: add_node result");
    c_result
}

unsafe fn compare_find(c: &Api, rust: &Api, id: i32, context: &str) -> (*mut Node, *mut Node) {
    let c_node = unsafe { c.find_node(id) };
    let rust_node = unsafe { rust.find_node(id) };
    unsafe { assert_nodes_equal(c_node, rust_node, context) };
    (c_node, rust_node)
}

fn assert_f64_bits_equal(c: f64, rust: f64, context: &str) {
    assert_eq!(c.to_bits(), rust.to_bits(), "{context}: floating output");
}

fn check_initial_empty_and_missing() {
    with_apis(|c, rust| {
        for id in [i32::MIN, -1, 0, 1, 17, i32::MAX] {
            let (c_node, rust_node) = unsafe { compare_find(c, rust, id, "CONFIG 7 / ERROR 2") };
            assert!(c_node.is_null() && rust_node.is_null());
            assert_eq!(
                unsafe { c.children_count(id) },
                unsafe { rust.children_count(id) },
                "CONFIG 10 parent {id}"
            );
            let c_sum = unsafe { c.subtree_sum(id) };
            let rust_sum = unsafe { rust.subtree_sum(id) };
            assert_f64_bits_equal(c_sum, rust_sum, "CONFIG 13 / ERROR 3");
            assert_eq!(c_sum.to_bits(), 0.0f64.to_bits());
        }
    });
}

fn check_add_and_lookup_shapes() {
    with_apis(|c, rust| {
        let mut rng = Rng::new(0xadd0_0001);
        let empty_index = unsafe {
            add_both(
                c,
                rust,
                rng.i32(),
                rng.i32(),
                b"",
                rng.finite_value(),
                "CONFIG 1",
            )
        };
        assert_eq!(empty_index, 0);

        for sequence in 0..24 {
            let length = 1 + rng.bounded(48) as usize;
            let name: Vec<u8> = (0..length).map(|_| b'a' + rng.bounded(26) as u8).collect();
            let id = 10_000 + sequence;
            let index = unsafe {
                add_both(
                    c,
                    rust,
                    id,
                    rng.i32(),
                    &name,
                    rng.finite_value(),
                    "CONFIG 2",
                )
            };
            assert_eq!(index, sequence + 1);
            let (c_node, rust_node) =
                unsafe { compare_find(c, rust, id, "CONFIG 2 present active") };
            assert!(!c_node.is_null() && !rust_node.is_null());
        }

        for sequence in 0..8 {
            let name: Vec<u8> = (0..49).map(|_| b'A' + rng.bounded(26) as u8).collect();
            let id = 20_000 + sequence;
            unsafe {
                add_both(
                    c,
                    rust,
                    id,
                    rng.i32(),
                    &name,
                    rng.finite_value(),
                    "CONFIG 3",
                );
                compare_find(c, rust, id, "CONFIG 3 exact 49-byte name");
            }
        }

        for sequence in 0..8 {
            let length = 50 + rng.bounded(80) as usize;
            let name: Vec<u8> = (0..length).map(|_| b'!' + rng.bounded(90) as u8).collect();
            let id = 30_000 + sequence;
            unsafe {
                add_both(
                    c,
                    rust,
                    id,
                    rng.i32(),
                    &name,
                    rng.finite_value(),
                    "CONFIG 4",
                );
                let (c_node, rust_node) = compare_find(c, rust, id, "CONFIG 4 truncated name");
                assert_eq!((*c_node).name[49], 0);
                assert_eq!((*rust_node).name[49], 0);
            }
        }
    });

    for sequence in 0..32 {
        with_apis(|c, rust| {
            let mut rng = Rng::new(0xd001_0000 + sequence);
            let id = rng.i32();
            unsafe {
                add_both(c, rust, id, 1, b"first", rng.finite_value(), "CONFIG 5");
                add_both(c, rust, id, 2, b"second", rng.finite_value(), "CONFIG 5");
                let (c_node, rust_node) =
                    compare_find(c, rust, id, "CONFIG 5 duplicate first match");
                assert_eq!((*c_node).parent_id, 1);
                assert_eq!((*rust_node).parent_id, 1);
            }
        });
    }

    with_apis(|c, rust| {
        let mut rng = Rng::new(0xca9a_c17a);
        for index in 0..100 {
            let name = format!("node-{index}-{:08x}", rng.next_u32());
            let result = unsafe {
                add_both(
                    c,
                    rust,
                    index,
                    rng.i32(),
                    name.as_bytes(),
                    rng.finite_value(),
                    "CONFIG 6",
                )
            };
            assert_eq!(result, index);
        }
        let extra = c_string(b"full");
        let c_result = unsafe { c.add_node(101, 0, extra.as_ptr(), 1.0) };
        let rust_result = unsafe { rust.add_node(101, 0, extra.as_ptr(), 1.0) };
        assert_eq!(c_result, -1, "ERROR 1 C capacity result");
        assert_eq!(rust_result, -1, "ERROR 1 Rust capacity result");
        unsafe { compare_find(c, rust, 99, "ERROR 1 storage remains intact") };
        let c_absent = unsafe { c.find_node(101) };
        let rust_absent = unsafe { rust.find_node(101) };
        assert!(c_absent.is_null() && rust_absent.is_null());
    });
}

fn check_active_and_children() {
    for sequence in 0..32 {
        with_apis(|c, rust| {
            let id = 1000 + sequence;
            unsafe {
                add_both(
                    c,
                    rust,
                    id,
                    sequence,
                    b"active",
                    sequence as f64,
                    "CONFIG 8",
                );
                let (c_node, rust_node) = compare_find(c, rust, id, "CONFIG 8 present active");
                (*c_node).active = 0;
                (*rust_node).active = 0;
                let (c_missing, rust_missing) =
                    compare_find(c, rust, id, "CONFIG 9 / ERROR 2 inactive");
                assert!(c_missing.is_null() && rust_missing.is_null());
                assert_f64_bits_equal(
                    c.subtree_sum(id),
                    rust.subtree_sum(id),
                    "CONFIG 13 inactive root",
                );
            }
        });
    }

    with_apis(|c, rust| {
        let mut rng = Rng::new(0xc011_d001);
        for group in 0..30 {
            let parent = 40_000 + group;
            let child = 50_000 + group;
            unsafe {
                add_both(
                    c,
                    rust,
                    child,
                    parent,
                    b"only-child",
                    rng.finite_value(),
                    "CONFIG 11",
                );
            }
            assert_eq!(
                unsafe { c.children_count(parent) },
                unsafe { rust.children_count(parent) },
                "CONFIG 11 group {group}"
            );
            assert_eq!(unsafe { c.children_count(parent) }, 1);
        }
    });

    with_apis(|c, rust| {
        let mut rng = Rng::new(0xc011_d002);
        for group in 0..30 {
            let parent = 60_000 + group;
            for child_offset in 0..3 {
                let id = 70_000 + group * 3 + child_offset;
                unsafe {
                    add_both(
                        c,
                        rust,
                        id,
                        parent,
                        b"sibling",
                        rng.finite_value(),
                        "CONFIG 12",
                    );
                }
                if child_offset == 2 {
                    let (c_node, rust_node) =
                        unsafe { compare_find(c, rust, id, "CONFIG 12 inactive sibling") };
                    unsafe {
                        (*c_node).active = 0;
                        (*rust_node).active = 0;
                    }
                }
            }
            let c_count = unsafe { c.children_count(parent) };
            let rust_count = unsafe { rust.children_count(parent) };
            assert_eq!(c_count, rust_count, "CONFIG 12 group {group}");
            assert_eq!(c_count, 2);
        }
        assert_eq!(unsafe { c.children_count(-999) }, 0, "CONFIG 10");
        assert_eq!(
            unsafe { c.children_count(-999) },
            unsafe { rust.children_count(-999) },
            "CONFIG 10 absent parent"
        );
    });
}

fn check_subtree_shapes() {
    with_apis(|c, rust| {
        let mut rng = Rng::new(0x5ab7_1eef);
        for index in 0..64 {
            let id = 100_000 + index;
            unsafe {
                add_both(
                    c,
                    rust,
                    id,
                    -1000 - index,
                    b"leaf",
                    rng.finite_value(),
                    "CONFIG 14",
                );
            }
            assert_f64_bits_equal(
                unsafe { c.subtree_sum(id) },
                unsafe { rust.subtree_sum(id) },
                "CONFIG 14 leaf",
            );
        }
    });

    with_apis(|c, rust| {
        let mut rng = Rng::new(0x5ab7_1e15);
        for group in 0..20 {
            let root = 110_000 + group * 4;
            unsafe {
                add_both(c, rust, root, -1, b"root", rng.finite_value(), "CONFIG 15");
                for offset in 1..=3 {
                    add_both(
                        c,
                        rust,
                        root + offset,
                        root,
                        b"child",
                        rng.finite_value(),
                        "CONFIG 15",
                    );
                }
            }
            assert_f64_bits_equal(
                unsafe { c.subtree_sum(root) },
                unsafe { rust.subtree_sum(root) },
                "CONFIG 15 direct children",
            );
        }
    });

    with_apis(|c, rust| {
        let mut rng = Rng::new(0x5ab7_1e16);
        for group in 0..16 {
            let root = 120_000 + group * 6;
            unsafe {
                add_both(c, rust, root, -1, b"root", rng.finite_value(), "CONFIG 16");
                add_both(
                    c,
                    rust,
                    root + 1,
                    root,
                    b"left",
                    rng.finite_value(),
                    "CONFIG 16",
                );
                add_both(
                    c,
                    rust,
                    root + 2,
                    root,
                    b"right",
                    rng.finite_value(),
                    "CONFIG 16",
                );
                add_both(
                    c,
                    rust,
                    root + 3,
                    root + 1,
                    b"grandchild",
                    rng.finite_value(),
                    "CONFIG 16",
                );
                add_both(
                    c,
                    rust,
                    root + 4,
                    root + 3,
                    b"great-grandchild",
                    rng.finite_value(),
                    "CONFIG 16",
                );
                add_both(
                    c,
                    rust,
                    root + 5,
                    root + 2,
                    b"inactive-branch",
                    rng.finite_value(),
                    "CONFIG 16",
                );
                let (c_inactive, rust_inactive) =
                    compare_find(c, rust, root + 5, "CONFIG 16 inactive branch");
                (*c_inactive).active = 0;
                (*rust_inactive).active = 0;
            }
            assert_f64_bits_equal(
                unsafe { c.subtree_sum(root) },
                unsafe { rust.subtree_sum(root) },
                "CONFIG 16 recursive tree",
            );
        }
    });
}

fn check_strings() {
    with_apis(|c, rust| {
        let mut empty = c_string(b"");
        assert_eq!(
            unsafe { c.process_string(empty.as_mut_ptr()) },
            unsafe { rust.process_string(empty.as_mut_ptr()) },
            "CONFIG 17"
        );

        let mut rng = Rng::new(0x57a1_0001);
        for _ in 0..128 {
            let mut one = c_string(&[1 + rng.bounded(127) as u8]);
            assert_eq!(
                unsafe { c.process_string(one.as_mut_ptr()) },
                unsafe { rust.process_string(one.as_mut_ptr()) },
                "CONFIG 18"
            );

            let length = 2 + rng.bounded(126) as usize;
            let bytes: Vec<u8> = (0..length).map(|_| 1 + rng.bounded(127) as u8).collect();
            let mut many = c_string(&bytes);
            assert_eq!(
                unsafe { c.process_string(many.as_mut_ptr()) },
                unsafe { rust.process_string(many.as_mut_ptr()) },
                "CONFIG 19"
            );

            let high_bytes: Vec<u8> = (0..length).map(|_| 128 + rng.bounded(128) as u8).collect();
            let mut high = c_string(&high_bytes);
            assert_eq!(
                unsafe { c.process_string(high.as_mut_ptr()) },
                unsafe { rust.process_string(high.as_mut_ptr()) },
                "CONFIG 20"
            );
        }
    });
}

fn check_double_conversion() {
    with_apis(|c, rust| {
        let mut rng = Rng::new(0xd0ab_1e00);
        for _ in 0..512 {
            let integer = (rng.i32() >> 2) as f64;
            assert_eq!(
                unsafe { c.double_to_int(integer) },
                unsafe { rust.double_to_int(integer) },
                "CONFIG 21"
            );

            let base = (rng.i32() >> 3) as f64;
            let fraction = (1 + rng.bounded(1023)) as f64 / 1024.0;
            let value = if rng.bounded(2) == 0 {
                base + fraction
            } else {
                base - fraction
            };
            assert_eq!(
                unsafe { c.double_to_int(value) },
                unsafe { rust.double_to_int(value) },
                "CONFIG 22"
            );
        }

        for value in [
            i32::MIN as f64,
            i32::MIN as f64 + 1.0,
            i32::MAX as f64 - 1.0,
            i32::MAX as f64,
        ] {
            assert_eq!(
                unsafe { c.double_to_int(value) },
                unsafe { rust.double_to_int(value) },
                "CONFIG 23 boundary {value}"
            );
        }

        for value in [i32::MAX as f64 + 1.0, i32::MAX as f64 * 2.0, f64::INFINITY] {
            assert_eq!(unsafe { c.double_to_int(value) }, i32::MAX, "ERROR 4 C");
            assert_eq!(
                unsafe { rust.double_to_int(value) },
                i32::MAX,
                "ERROR 4 Rust"
            );
        }
        for value in [
            i32::MIN as f64 - 1.0,
            i32::MIN as f64 * 2.0,
            f64::NEG_INFINITY,
        ] {
            assert_eq!(unsafe { c.double_to_int(value) }, i32::MIN, "ERROR 5 C");
            assert_eq!(
                unsafe { rust.double_to_int(value) },
                i32::MIN,
                "ERROR 5 Rust"
            );
        }
        for nan in [
            f64::NAN,
            f64::from_bits(0x7ff8_0000_0000_0001),
            f64::from_bits(0xfff8_0000_0000_1234),
        ] {
            assert_eq!(unsafe { c.double_to_int(nan) }, 0, "ERROR 6 C");
            assert_eq!(unsafe { rust.double_to_int(nan) }, 0, "ERROR 6 Rust");
        }
    });
}

unsafe fn assert_maxnmin_equal(c: &Api, rust: &Api, input: [i32; 4], row: u32) {
    let c_result = unsafe { c.maxnmin(input[0], input[1], input[2], input[3]) };
    let rust_result = unsafe { rust.maxnmin(input[0], input[1], input[2], input[3]) };
    assert_eq!(c_result, rust_result, "CONFIG {row}, input {input:?}");
}

fn check_maxnmin() {
    with_apis(|c, rust| {
        let mut rng = Rng::new(0xaa71_0000);

        for _ in 0..128 {
            let param1 = (rng.bounded(100_000) as i32) * 6;
            unsafe { assert_maxnmin_equal(c, rust, [param1, rng.i32(), rng.i32(), rng.i32()], 24) };

            let param1 = (rng.bounded(100_000) as i32) * 6 + 1 + rng.bounded(2) as i32;
            unsafe { assert_maxnmin_equal(c, rust, [param1, rng.i32(), rng.i32(), rng.i32()], 25) };

            let param1 = (rng.bounded(100_000) as i32) * 6 + 3 + rng.bounded(3) as i32;
            unsafe { assert_maxnmin_equal(c, rust, [param1, rng.i32(), rng.i32(), rng.i32()], 26) };

            let param1 = -((rng.bounded(100_000) as i32) * 6 + 1 + rng.bounded(5) as i32);
            unsafe { assert_maxnmin_equal(c, rust, [param1, rng.i32(), rng.i32(), rng.i32()], 27) };

            let param2 = (rng.bounded(100_000) as i32) * 6 + rng.bounded(6) as i32;
            let param3 = rng.bounded(201) as i32 - 100;
            unsafe {
                assert_maxnmin_equal(
                    c,
                    rust,
                    [rng.bounded(10_000) as i32, param2, param3, rng.i32()],
                    28,
                )
            };

            let param2 = -((rng.bounded(100_000) as i32) * 6 + 1 + rng.bounded(5) as i32);
            unsafe { assert_maxnmin_equal(c, rust, [rng.i32(), param2, rng.i32(), rng.i32()], 29) };

            let param2 = rng.bounded(6) as i32;
            let param3 = if rng.bounded(2) == 0 {
                i32::MAX
            } else {
                i32::MIN
            };
            unsafe { assert_maxnmin_equal(c, rust, [rng.i32(), param2, param3, rng.i32()], 30) };

            let parent_shape = [0, 1, 2, -1, -2][rng.bounded(5) as usize];
            unsafe {
                assert_maxnmin_equal(c, rust, [rng.i32(), rng.i32(), rng.i32(), parent_shape], 31)
            };

            let a = rng.i32();
            let b = if rng.bounded(2) == 0 {
                a.wrapping_neg()
            } else {
                rng.i32()
            };
            unsafe { assert_maxnmin_equal(c, rust, [a, b, -1, rng.i32()], 32) };

            let param3 = loop {
                let candidate = rng.bounded(2001) as i32 - 1000;
                if candidate != -1 {
                    break candidate;
                }
            };
            unsafe {
                assert_maxnmin_equal(
                    c,
                    rust,
                    [
                        rng.bounded(2_000_001) as i32 - 1_000_000,
                        rng.bounded(2_000_001) as i32 - 1_000_000,
                        param3,
                        rng.bounded(2_000_001) as i32 - 1_000_000,
                    ],
                    33,
                )
            };
        }

        for input in [
            [1_500_000_000, 500_000_000, 0, i32::MAX],
            [-1_500_000_000, -500_000_000, 0, i32::MAX],
            [i32::MAX, 0, 0, i32::MAX],
            [i32::MIN, 0, 0, i32::MAX],
        ] {
            unsafe { assert_maxnmin_equal(c, rust, input, 34) };
        }

        let boundary_values = [i32::MIN, i32::MIN + 1, -1, 0, i32::MAX - 1, i32::MAX];
        for index in 0..128 {
            let input = [
                boundary_values[index % boundary_values.len()],
                boundary_values[(index * 3 + 1) % boundary_values.len()],
                boundary_values[(index * 5 + 2) % boundary_values.len()],
                boundary_values[(index * 7 + 3) % boundary_values.len()],
            ];
            unsafe { assert_maxnmin_equal(c, rust, input, 35) };
        }
    });
}

#[cfg(unix)]
fn signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

fn run_null_child(library: &Path, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("null_boundary_child")
        .arg("--nocapture")
        .env("DIFFERENTIAL_NULL_LIBRARY", library)
        .env("DIFFERENTIAL_NULL_CASE", case)
        .status()
        .unwrap()
}

fn check_null_boundaries() {
    for case in ["add_node", "process_string"] {
        let c_status = run_null_child(&c_library_path(), case);
        let rust_status = run_null_child(&rust_library_path(), case);
        assert!(
            !c_status.success(),
            "C null case {case} unexpectedly succeeded"
        );
        assert!(
            !rust_status.success(),
            "Rust null case {case} unexpectedly succeeded"
        );
        #[cfg(unix)]
        {
            assert_eq!(
                signal(c_status),
                signal(rust_status),
                "ERROR boundary {case}: terminating signal differs"
            );
            assert!(
                signal(c_status).is_some(),
                "ERROR boundary {case}: expected signal termination"
            );
        }
    }
}

#[test]
fn differential_surface() {
    assert!(
        c_library_path().is_file(),
        "build the C shared library before testing"
    );
    assert!(
        rust_library_path().is_file(),
        "build the Rust release shared library before testing"
    );

    check_initial_empty_and_missing();
    check_add_and_lookup_shapes();
    check_active_and_children();
    check_subtree_shapes();
    check_strings();
    check_double_conversion();
    check_maxnmin();
    check_null_boundaries();
}

#[test]
fn null_boundary_child() {
    let Ok(path) = std::env::var("DIFFERENTIAL_NULL_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFFERENTIAL_NULL_CASE").unwrap();
    unsafe {
        let api = Api::open(Path::new(&path));
        match case.as_str() {
            "add_node" => {
                api.add_node(1, 0, std::ptr::null(), 1.0);
            }
            "process_string" => {
                api.process_string(std::ptr::null_mut());
            }
            _ => panic!("unknown null case {case}"),
        }
    }
}
