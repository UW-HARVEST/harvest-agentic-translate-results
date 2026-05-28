// Integration tests comparing the C reference shared library against the
// Rust-translated shared library through the FFI boundary.
//
// Both libraries are loaded with `libloading` and their exported symbols are
// invoked side by side. Outputs are compared byte-for-byte where the C code
// produces deterministic results.
//
// Note: the C `compare_allocations` function depends on the relative ordering
// of two `malloc` results, which is non-deterministic. For functions that
// transitively call into it, we compute the *set* of possible outputs and
// check that both libraries return values in that set.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_uchar};
use std::path::PathBuf;
use std::sync::OnceLock;

struct Libs {
    c: Library,
    r: Library,
}

fn libs() -> &'static Libs {
    static INSTANCE: OnceLock<Libs> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let mut c_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        c_path.push("c_src/build/libtranslated_rust.so");

        let mut r_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        r_path.push("target/release/libarity_lib.so");

        unsafe {
            Libs {
                c: Library::new(&c_path).expect("failed to load C library"),
                r: Library::new(&r_path).expect("failed to load Rust library"),
            }
        }
    })
}

unsafe fn sym<'lib, T>(lib: &'lib Library, name: &[u8]) -> Symbol<'lib, T> {
    lib.get(name).expect("missing symbol")
}

// --------------------------------------------------------------------------
// shift_array
// --------------------------------------------------------------------------

#[test]
fn test_shift_array_matches() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(*mut c_int, c_int, c_int);

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"shift_array");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"shift_array");

        let cases: &[(Vec<c_int>, c_int, c_int)] = &[
            (vec![1, 2, 3, 4], 4, 1),
            (vec![1, 2, 3, 4], 4, 2),
            (vec![1, 2, 3, 4], 4, 3),
            (vec![1, 2, 3, 4], 4, 0),     // no-op (positions == 0)
            (vec![1, 2, 3, 4], 4, 4),     // no-op (positions == size)
            (vec![1, 2, 3, 4], 4, -1),    // no-op (positions < 0)
            (vec![10, 20, 30, 40, 50, 60], 6, 2),
            (vec![10, 20, 30, 40, 50, 60], 6, 5),
            (vec![-7, 0, 99, -1], 4, 1),
        ];

        for (arr, size, positions) in cases {
            let mut a = arr.clone();
            let mut b = arr.clone();
            c_fn(a.as_mut_ptr(), *size, *positions);
            r_fn(b.as_mut_ptr(), *size, *positions);
            assert_eq!(a, b, "shift_array mismatch for {:?} size={} pos={}", arr, size, positions);
        }
    }
}

// --------------------------------------------------------------------------
// process_string
// --------------------------------------------------------------------------

#[test]
fn test_process_string_matches() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(*const c_char) -> c_int;

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"process_string");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"process_string");

        let cases: &[&[u8]] = &[
            b"\0",                       // empty
            b"a\0",
            b"hello\0",
            b"Hello, world!\0",
            b"\xff\xfe\xfd\0",
        ];

        for s in cases {
            let c_res = c_fn(s.as_ptr() as *const c_char);
            let r_res = r_fn(s.as_ptr() as *const c_char);
            assert_eq!(c_res, r_res, "process_string mismatch for {:?}", s);
        }
    }
}

// --------------------------------------------------------------------------
// apply_bitmask
// --------------------------------------------------------------------------

#[test]
fn test_apply_bitmask_matches() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(c_int, c_int) -> c_int;

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"apply_bitmask");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"apply_bitmask");

        for value in &[0i32, 1, 0xFF, 0xAA, 0x55, 0x12345678, -1, -42, i32::MIN, i32::MAX] {
            for op in -1..=5 {
                let c_res = c_fn(*value, op);
                let r_res = r_fn(*value, op);
                assert_eq!(c_res, r_res, "apply_bitmask({}, {}) mismatch", value, op);
            }
        }
    }
}

// --------------------------------------------------------------------------
// init_matrix
// --------------------------------------------------------------------------

#[test]
fn test_init_matrix_matches() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(*mut c_int);

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"init_matrix");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"init_matrix");

        let mut c_mat = [[0xDEADBEEFu32 as c_int; 4]; 3];
        let mut r_mat = [[0xDEADBEEFu32 as c_int; 4]; 3];

        c_fn(c_mat.as_mut_ptr() as *mut c_int);
        r_fn(r_mat.as_mut_ptr() as *mut c_int);

        assert_eq!(c_mat, r_mat, "init_matrix mismatch");
        assert_eq!(c_mat, [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]]);
    }
}

// --------------------------------------------------------------------------
// compare_allocations
// --------------------------------------------------------------------------

// Possible return values: pointer-order tag (1, 2, or 3) plus value bonus
// (0 if val1 <= 0 else 10).
fn compare_allocations_possible(val1: c_int) -> Vec<c_int> {
    let bonus = if val1 > 0 { 10 } else { 0 };
    (1..=3).map(|tag| tag + bonus).collect()
}

#[test]
fn test_compare_allocations_value_component() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(c_int, c_int) -> c_int;

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"compare_allocations");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"compare_allocations");

        for &(v1, v2) in &[(0i32, 0i32), (1, 0), (-1, 5), (100, -100), (42, 42)] {
            let c_res = c_fn(v1, v2);
            let r_res = r_fn(v1, v2);
            let possible = compare_allocations_possible(v1);
            assert!(possible.contains(&c_res),
                "C compare_allocations({}, {}) = {} not in {:?}", v1, v2, c_res, possible);
            assert!(possible.contains(&r_res),
                "Rust compare_allocations({}, {}) = {} not in {:?}", v1, v2, r_res, possible);
        }
    }
}

// --------------------------------------------------------------------------
// arity4 / arity3 / arity2 / arity
// --------------------------------------------------------------------------

// Faithfully recompute arity4 in pure Rust assuming `compare_allocations`
// returned `alloc`. Used to enumerate the possible C/Rust outputs when
// pointer ordering is unknown.
fn arity4_with_alloc(p1: c_int, p2: c_int, p3: c_int, p4: c_int, alloc: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut values = [p1, p2, p3, p4];

    // process_string("Hello") = 5; process_string("") = 0
    result += 5 + 0;

    // shift_array(values, 4, 1) -> shift right by 1, fill leading with 0
    for i in (1..4).rev() {
        values[i] = values[i - 1];
    }
    values[0] = 0;

    for v in &values {
        result += *v;
    }

    // apply_bitmask(result, p1 % 4)  (C `%` truncates toward zero, same as Rust `%`)
    let op = p1 % 4;
    result = match op {
        0 => result & 0b1111_0000,
        1 => result & 0b0000_1111,
        2 => result | 0b1010_1010,
        3 => result ^ 0b0101_0101,
        _ => result,
    };

    // matrix[0][0] + matrix[2][3] = 1 + 12
    result += 1 + 12;

    result += alloc;

    if p3 != 0 {
        result = (result * p3) / 100;
    }
    if p4 != 0 {
        result += p4;
    }
    result
}

fn arity4_possible(p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> Vec<c_int> {
    compare_allocations_possible(p1)
        .into_iter()
        .map(|alloc| arity4_with_alloc(p1, p2, p3, p4, alloc))
        .collect()
}

#[test]
fn test_arity4_consistency() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"arity4");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"arity4");

        let cases = [
            (1, 2, 0, 0),
            (5, 5, 5, 5),
            (10, 20, 30, 40),
            (-1, -2, -3, -4),
            (0, 0, 0, 0),
            (100, 200, 0, 0),
            (1, 1, 100, 1),
            (3, 7, 50, 5),
            (4, 8, 0, 0),    // p1 % 4 == 0
            (1, 8, 0, 0),    // p1 % 4 == 1
            (2, 8, 0, 0),    // p1 % 4 == 2
            (3, 8, 0, 0),    // p1 % 4 == 3
            (-4, 8, 0, 0),   // negative p1
        ];
        for (p1, p2, p3, p4) in cases {
            let c_res = c_fn(p1, p2, p3, p4);
            let r_res = r_fn(p1, p2, p3, p4);
            let possible = arity4_possible(p1, p2, p3, p4);
            assert!(possible.contains(&c_res),
                "C arity4({}, {}, {}, {}) = {} not in {:?}", p1, p2, p3, p4, c_res, possible);
            assert!(possible.contains(&r_res),
                "Rust arity4({}, {}, {}, {}) = {} not in {:?}", p1, p2, p3, p4, r_res, possible);
        }
    }
}

#[test]
fn test_arity2_consistency() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(c_int, c_int) -> c_int;

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"arity2");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"arity2");

        for (p1, p2) in [(1i32, 2i32), (5, 5), (-1, -2), (100, 200), (4, 8)] {
            let c_res = c_fn(p1, p2);
            let r_res = r_fn(p1, p2);
            let possible = arity4_possible(p1, p2, 0, 0);
            assert!(possible.contains(&c_res),
                "C arity2({}, {}) = {} not in {:?}", p1, p2, c_res, possible);
            assert!(possible.contains(&r_res),
                "Rust arity2({}, {}) = {} not in {:?}", p1, p2, r_res, possible);
        }
    }
}

#[test]
fn test_arity3_consistency() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"arity3");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"arity3");

        for (p1, p2, p3) in [(1i32, 2i32, 3i32), (5, 5, 5), (-1, -2, -3), (4, 8, 50)] {
            let c_res = c_fn(p1, p2, p3);
            let r_res = r_fn(p1, p2, p3);
            let possible = arity4_possible(p1, p2, p3, 0);
            assert!(possible.contains(&c_res),
                "C arity3({}, {}, {}) = {} not in {:?}", p1, p2, p3, c_res, possible);
            assert!(possible.contains(&r_res),
                "Rust arity3({}, {}, {}) = {} not in {:?}", p1, p2, p3, r_res, possible);
        }
    }
}

#[test]
fn test_arity_dispatch_short_len_matches_exactly() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(c_uchar, *mut c_int) -> c_int;

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"arity");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"arity");

        let mut params = [1, 2, 3, 4];
        for len in [0u8, 1u8] {
            let c_res = c_fn(len, params.as_mut_ptr());
            let r_res = r_fn(len, params.as_mut_ptr());
            assert_eq!(c_res, -1);
            assert_eq!(r_res, -1);
        }
    }
}

#[test]
fn test_arity_dispatch_consistency() {
    let libs = libs();
    type Fn_ = unsafe extern "C" fn(c_uchar, *mut c_int) -> c_int;

    unsafe {
        let c_fn: Symbol<Fn_> = sym(&libs.c, b"arity");
        let r_fn: Symbol<Fn_> = sym(&libs.r, b"arity");

        let scenarios: &[(u8, [c_int; 4])] = &[
            (2, [1, 2, 0, 0]),
            (3, [4, 5, 6, 0]),
            (4, [7, 8, 9, 10]),
            (5, [11, 12, 13, 14]),  // len > 4 still uses arity4
            (3, [-1, -2, -3, 0]),
        ];

        for (len, params) in scenarios {
            let mut p_c = *params;
            let mut p_r = *params;
            let c_res = c_fn(*len, p_c.as_mut_ptr());
            let r_res = r_fn(*len, p_r.as_mut_ptr());

            let (a, b, cc, d) = match *len {
                2 => (params[0], params[1], 0, 0),
                3 => (params[0], params[1], params[2], 0),
                _ => (params[0], params[1], params[2], params[3]),
            };
            let possible = arity4_possible(a, b, cc, d);

            assert!(possible.contains(&c_res),
                "C arity(len={}, {:?}) = {} not in {:?}", len, params, c_res, possible);
            assert!(possible.contains(&r_res),
                "Rust arity(len={}, {:?}) = {} not in {:?}", len, params, r_res, possible);
        }
    }
}

// --------------------------------------------------------------------------
// Symbol export parity: every public symbol exported by the C .so must
// also be exported by the Rust .so under the same name.
// --------------------------------------------------------------------------

#[test]
fn test_rust_so_exports_every_c_symbol() {
    let libs = libs();
    let expected = [
        "shift_array",
        "process_string",
        "apply_bitmask",
        "init_matrix",
        "compare_allocations",
        "arity4",
        "arity3",
        "arity2",
        "arity",
    ];
    unsafe {
        for name in expected {
            let mut bytes = name.as_bytes().to_vec();
            bytes.push(0);
            let c_sym: Result<Symbol<*const ()>, _> = libs.c.get(&bytes);
            let r_sym: Result<Symbol<*const ()>, _> = libs.r.get(&bytes);
            assert!(c_sym.is_ok(), "C library missing '{}'", name);
            assert!(r_sym.is_ok(), "Rust library missing '{}'", name);
        }
    }
}
