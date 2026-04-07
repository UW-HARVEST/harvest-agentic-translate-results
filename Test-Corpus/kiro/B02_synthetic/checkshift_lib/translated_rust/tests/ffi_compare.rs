use libloading::Library;
use std::ffi::{c_char, c_int, c_uint};
use std::path::PathBuf;

type BinOp = unsafe extern "C" fn(c_int, c_int) -> c_int;

#[repr(C)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: c_uint,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libcheckshift_lib.so");
    p
}

macro_rules! load_fn {
    ($lib:expr, $name:literal, $ty:ty) => {
        unsafe { $lib.get::<$ty>($name).unwrap() }
    };
}

// ── Leaf arithmetic functions ──────────────────────────────────────

#[test]
fn test_multiply_with_static() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_fn!(c, b"multiply_with_static", BinOp);
    let r_fn = load_fn!(r, b"multiply_with_static", BinOp);

    let cases: &[(c_int, c_int)] = &[
        (0, 0), (1, 1), (-1, 1), (100, 200), (-50, 30),
        (i32::MAX, 1), (i32::MIN, 1), (12345, -6789),
    ];
    for &(a, b) in cases {
        let cv = unsafe { c_fn(a, b) };
        let rv = unsafe { r_fn(a, b) };
        assert_eq!(cv, rv, "multiply_with_static({a}, {b})");
    }
}

#[test]
fn test_add_with_static() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_fn!(c, b"add_with_static", BinOp);
    let r_fn = load_fn!(r, b"add_with_static", BinOp);

    let cases: &[(c_int, c_int)] = &[
        (0, 0), (1, -1), (i32::MAX, 0), (i32::MIN, 0), (500, 600),
    ];
    for &(a, b) in cases {
        let cv = unsafe { c_fn(a, b) };
        let rv = unsafe { r_fn(a, b) };
        assert_eq!(cv, rv, "add_with_static({a}, {b})");
    }
}

#[test]
fn test_xor_operation() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_fn!(c, b"xor_operation", BinOp);
    let r_fn = load_fn!(r, b"xor_operation", BinOp);

    let cases: &[(c_int, c_int)] = &[
        (0, 0), (0xFFFF, 0xFFFF), (-1, -1), (0xABCD, 0), (123, 456),
    ];
    for &(a, b) in cases {
        let cv = unsafe { c_fn(a, b) };
        let rv = unsafe { r_fn(a, b) };
        assert_eq!(cv, rv, "xor_operation({a}, {b})");
    }
}

#[test]
fn test_shift_with_static() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_fn!(c, b"shift_with_static", BinOp);
    let r_fn = load_fn!(r, b"shift_with_static", BinOp);

    let cases: &[(c_int, c_int)] = &[
        (0, 0), (1, 1), (0xFF, 0xFF), (1000, 2000), (-1, 4),
    ];
    for &(a, b) in cases {
        let cv = unsafe { c_fn(a, b) };
        let rv = unsafe { r_fn(a, b) };
        assert_eq!(cv, rv, "shift_with_static({a}, {b})");
    }
}

// ── get_operation ──────────────────────────────────────────────────

#[test]
fn test_get_operation() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    type GetOp = unsafe extern "C" fn(c_int) -> Option<BinOp>;
    let c_get = load_fn!(c, b"get_operation", GetOp);
    let r_get = load_fn!(r, b"get_operation", GetOp);

    // Valid opcodes: call the returned fn pointers with same args
    for opcode in 0..4 {
        let c_op = unsafe { c_get(opcode) };
        let r_op = unsafe { r_get(opcode) };
        assert!(c_op.is_some(), "C get_operation({opcode}) returned NULL");
        assert!(r_op.is_some(), "Rust get_operation({opcode}) returned NULL");
        let (a, b) = (7, 13);
        let cv = unsafe { c_op.unwrap()(a, b) };
        let rv = unsafe { r_op.unwrap()(a, b) };
        assert_eq!(cv, rv, "get_operation({opcode}) applied to ({a},{b})");
    }

    // Invalid opcodes should return NULL
    for opcode in &[-1, 4, 100] {
        let c_op = unsafe { c_get(*opcode) };
        let r_op = unsafe { r_get(*opcode) };
        assert!(c_op.is_none(), "C get_operation({opcode}) should be NULL");
        assert!(r_op.is_none(), "Rust get_operation({opcode}) should be NULL");
    }
}

// ── compute_checksum ───────────────────────────────────────────────

#[test]
fn test_compute_checksum() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    type ChecksumFn = unsafe extern "C" fn(*const c_int, c_int) -> c_uint;
    let c_fn = load_fn!(c, b"compute_checksum", ChecksumFn);
    let r_fn = load_fn!(r, b"compute_checksum", ChecksumFn);

    // NULL pointer
    let cv = unsafe { c_fn(std::ptr::null(), 0) };
    let rv = unsafe { r_fn(std::ptr::null(), 0) };
    assert_eq!(cv, rv, "compute_checksum(NULL, 0)");

    // count=0
    let vals = [1i32, 2, 3, 4];
    let cv = unsafe { c_fn(vals.as_ptr(), 0) };
    let rv = unsafe { r_fn(vals.as_ptr(), 0) };
    assert_eq!(cv, rv, "compute_checksum(_, 0)");

    // Various counts
    let test_arrays: &[&[c_int]] = &[
        &[1, 2, 3, 4],
        &[0, 0, 0, 0],
        &[-1, -2, -3, -4],
        &[i32::MAX, i32::MIN, 0, 1],
        &[42],
        &[10, 20],
        &[100, 200, 300],
        &[1, 2, 3, 4, 5, 6, 7, 8], // count > 4
    ];
    for arr in test_arrays {
        let cv = unsafe { c_fn(arr.as_ptr(), arr.len() as c_int) };
        let rv = unsafe { r_fn(arr.as_ptr(), arr.len() as c_int) };
        assert_eq!(cv, rv, "compute_checksum({arr:?}, {})", arr.len());
    }
}

// ── init_state + apply_operation ───────────────────────────────────

#[test]
fn test_init_and_apply() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    type InitFn = unsafe extern "C" fn(*mut ComputeState, c_int);
    type ApplyFn = unsafe extern "C" fn(*mut ComputeState, c_int, Option<BinOp>);
    type GetOp = unsafe extern "C" fn(c_int) -> Option<BinOp>;

    let c_init = load_fn!(c, b"init_state", InitFn);
    let r_init = load_fn!(r, b"init_state", InitFn);
    let c_apply = load_fn!(c, b"apply_operation", ApplyFn);
    let r_apply = load_fn!(r, b"apply_operation", ApplyFn);
    let c_get = load_fn!(c, b"get_operation", GetOp);
    let r_get = load_fn!(r, b"get_operation", GetOp);

    for init_val in &[0, 42, -100, i32::MAX] {
        let mut c_state = std::mem::MaybeUninit::<ComputeState>::uninit();
        let mut r_state = std::mem::MaybeUninit::<ComputeState>::uninit();

        unsafe {
            c_init(c_state.as_mut_ptr(), *init_val);
            r_init(r_state.as_mut_ptr(), *init_val);
            let cs = c_state.assume_init_ref();
            let rs = r_state.assume_init_ref();
            assert_eq!(cs.accumulator, rs.accumulator, "init acc {init_val}");
            assert_eq!(cs.operation_count, rs.operation_count, "init opcount {init_val}");
            assert_eq!(cs.checksum, rs.checksum, "init checksum {init_val}");
        }

        // Apply each operation type
        for opcode in 0..4 {
            let c_op = unsafe { c_get(opcode) };
            let r_op = unsafe { r_get(opcode) };
            unsafe {
                c_apply(c_state.as_mut_ptr(), 5, c_op);
                r_apply(r_state.as_mut_ptr(), 5, r_op);
                let cs = c_state.assume_init_ref();
                let rs = r_state.assume_init_ref();
                assert_eq!(cs.accumulator, rs.accumulator, "apply op{opcode} init={init_val}");
                assert_eq!(cs.operation_count, rs.operation_count, "apply opcount op{opcode}");
            }
        }
    }
}

// ── execute_operation ──────────────────────────────────────────────

#[test]
fn test_execute_operation() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    type ExecFn = unsafe extern "C" fn(Option<BinOp>, c_int, c_int, *const c_char) -> c_int;
    type GetOp = unsafe extern "C" fn(c_int) -> Option<BinOp>;

    let c_exec = load_fn!(c, b"execute_operation", ExecFn);
    let r_exec = load_fn!(r, b"execute_operation", ExecFn);
    let c_get = load_fn!(c, b"get_operation", GetOp);
    let r_get = load_fn!(r, b"get_operation", GetOp);

    let name = b"TEST\0".as_ptr() as *const c_char;

    // NULL function pointer
    let cv = unsafe { c_exec(None, 1, 2, name) };
    let rv = unsafe { r_exec(None, 1, 2, name) };
    assert_eq!(cv, rv, "execute_operation(NULL, 1, 2)");

    // Each valid operation
    for opcode in 0..4 {
        let c_op = unsafe { c_get(opcode) };
        let r_op = unsafe { r_get(opcode) };
        for &(a, b) in &[(3, 7), (0, 0), (-10, 20)] {
            let cv = unsafe { c_exec(c_op, a, b, name) };
            let rv = unsafe { r_exec(r_op, a, b, name) };
            assert_eq!(cv, rv, "execute_operation(op{opcode}, {a}, {b})");
        }
    }
}

// ── checkshift (top-level) ─────────────────────────────────────────

#[test]
fn test_checkshift() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();

    type CheckshiftFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
    let c_fn = load_fn!(c, b"checkshift", CheckshiftFn);
    let r_fn = load_fn!(r, b"checkshift", CheckshiftFn);

    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (1, 2, 3, 4),
        (0, 0, 0, 0),
        (10, 20, 30, 40),
        (-1, -2, -3, -4),
        (100, 200, 300, 400),
        (i32::MAX, 1, 1, 1),
        (1, i32::MAX, 1, 1),
        (42, 0, 0, 0),
        (7, 13, 19, 23),
    ];
    for &(a, b, c_val, d) in cases {
        let cv = unsafe { c_fn(a, b, c_val, d) };
        let rv = unsafe { r_fn(a, b, c_val, d) };
        assert_eq!(cv, rv, "checkshift({a}, {b}, {c_val}, {d})");
    }
}
