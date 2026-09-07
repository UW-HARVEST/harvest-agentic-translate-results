//! Phase B — valid-path differential tests for the LOW-LEVEL entry points.
//! CONFIGS.md rows 1-13.

mod common;
use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;
const N: usize = 2000;

/// Boundary grid used by every arithmetic row.
fn grid() -> Vec<i32> {
    vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        5,
        -5,
        6,
        -6,
        127,
        128,
        -128,
        255,
        256,
        1000,
        -1000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ]
}

// ---------------------------------------------------------------------------
// Row 1 — is_valid_operation, exhaustive over all 256 char bit patterns
// ---------------------------------------------------------------------------
#[test]
fn row_01_is_valid_operation_exhaustive() {
    let p = pair();
    for byte in 0u16..=255 {
        let ch = byte as u8 as i8 as std::os::raw::c_char;
        let c = p.c.is_valid_operation_raw(ch);
        let r = p.rs.is_valid_operation_raw(ch);
        assert_eq!(
            c, r,
            "is_valid_operation(0x{byte:02x} = {ch}): C returned byte {c}, Rust returned byte {r}"
        );
        // Cross-check against the C semantics spelled out in the source.
        let expect = (ch != 0 && ch >= b'1' as i8 as std::os::raw::c_char
            && ch <= b'5' as i8 as std::os::raw::c_char) as u8;
        assert_eq!(c, expect, "C itself disagreed with the source for 0x{byte:02x}");
    }
}

// ---------------------------------------------------------------------------
// Rows 2 & 3 — get_operation_priority
// ---------------------------------------------------------------------------
#[test]
fn row_02_get_operation_priority_valid_enum() {
    let p = pair();
    for op in 1..=5 {
        assert_eq!(
            p.c.get_operation_priority(op),
            p.rs.get_operation_priority(op),
            "get_operation_priority({op})"
        );
    }
}

#[test]
fn row_03_get_operation_priority_out_of_enum_and_overflow() {
    let p = pair();
    let mut fixed = vec![0, 6, 7, -1, -5, -6, i32::MAX, i32::MIN, i32::MAX / 10, i32::MIN / 10];
    fixed.extend(grid());
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        fixed.push(rng.i32_interesting());
    }
    for op in fixed {
        assert_eq!(
            p.c.get_operation_priority(op),
            p.rs.get_operation_priority(op),
            "get_operation_priority({op}) (op*10, signed overflow path)"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 4-8 — the five arithmetic kernels
// ---------------------------------------------------------------------------

macro_rules! arith_row {
    ($name:ident, $sym:ident, $skip_div_trap:expr, $skip_zero_b:expr) => {
        #[test]
        fn $name() {
            let p = pair();
            let mut cases: Vec<(i32, i32, i32)> = Vec::new();

            // boundary grid, cross product, with the unused 3rd param varied
            let g = grid();
            for (i, &a) in g.iter().enumerate() {
                for (j, &b) in g.iter().enumerate() {
                    let unused = match (i + j) % 4 {
                        0 => 0,
                        1 => 1,
                        2 => -1,
                        _ => i32::MIN,
                    };
                    cases.push((a, b, unused));
                }
            }

            // randomized, fixed seed
            let mut rng = Rng::new(SEED ^ stringify!($sym).len() as u64);
            for _ in 0..N {
                let b = if $skip_zero_b { rng.nonzero_i32() } else { rng.i32_interesting() };
                cases.push((rng.i32_interesting(), b, rng.i32()));
            }

            let mut ran = 0usize;
            for (a, b, u) in cases {
                if $skip_div_trap && is_div_trap(a, b) {
                    // b == 0 is covered by the Phase C guard tests; INT_MIN/-1
                    // traps in C (ERRORS.md row 20).
                    if b != 0 {
                        continue;
                    }
                }
                let c = p.c.$sym(a, b, u);
                let r = p.rs.$sym(a, b, u);
                assert_eq!(c, r, "{}({a}, {b}, {u}): C={c} Rust={r}", stringify!($sym));
                ran += 1;
            }
            assert!(ran > N, "only {ran} cases ran for {}", stringify!($sym));
        }
    };
}

arith_row!(row_04_add_operation, add_operation, false, false);
arith_row!(row_05_subtract_operation, subtract_operation, false, false);
arith_row!(row_06_multiply_operation, multiply_operation, false, false);
arith_row!(row_07_divide_operation, divide_operation, true, true);
arith_row!(row_08_modulo_operation, modulo_operation, true, true);

// ---------------------------------------------------------------------------
// Rows 9 & 10 — select_operation: the returned pointer is INVOKED through FFI
// ---------------------------------------------------------------------------

fn probe_pairs() -> Vec<(i32, i32)> {
    let mut v = vec![
        (0, 0),
        (7, 3),
        (-7, 3),
        (7, -3),
        (-7, -3),
        (1, 1),
        (i32::MAX, 2),
        (i32::MIN, 2),
        (i32::MAX, i32::MAX),
        (123456, 789),
    ];
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..500 {
        v.push((rng.i32_interesting(), rng.nonzero_i32()));
    }
    v
}

#[test]
fn row_09_select_operation_valid_cases() {
    let p = pair();
    let expected_name = ["", "add_operation", "multiply_operation", "subtract_operation",
                         "divide_operation", "modulo_operation"];
    for op in 1..=5i32 {
        let fc = p.c.select_operation(op);
        let fr = p.rs.select_operation(op);

        // Pointer identity *within* each .so: select_operation must hand back
        // that .so's own kernel symbol (addresses differ between .so's).
        let name = expected_name[op as usize];
        assert_eq!(
            fc as usize,
            p.c.op_addr(name),
            "C select_operation({op}) did not return &{name}"
        );
        assert_eq!(
            fr as usize,
            p.rs.op_addr(name),
            "Rust select_operation({op}) did not return &{name}"
        );

        // Behavioural equivalence through the returned pointer.
        for (a, b) in probe_pairs() {
            if (op == 4 || op == 5) && is_div_trap(a, b) && b != 0 {
                continue;
            }
            let rc = unsafe { fc(a, b, 0) };
            let rr = unsafe { fr(a, b, 0) };
            assert_eq!(rc, rr, "select_operation({op})({a}, {b}, 0)");
        }
    }
}

#[test]
fn row_10_select_operation_default_branch() {
    let p = pair();
    let mut ops = vec![0, 6, 7, 8, -1, -2, -5, 100, i32::MAX, i32::MIN];
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..300 {
        let v = rng.i32_interesting();
        if !(1..=5).contains(&v) {
            ops.push(v);
        }
    }
    for op in ops {
        let fc = p.c.select_operation(op);
        let fr = p.rs.select_operation(op);
        assert_eq!(
            fc as usize,
            p.c.op_addr("add_operation"),
            "C select_operation({op}) default: must be &add_operation"
        );
        assert_eq!(
            fr as usize,
            p.rs.op_addr("add_operation"),
            "Rust select_operation({op}) default: must be &add_operation"
        );
        for (a, b) in probe_pairs().into_iter().take(60) {
            let rc = unsafe { fc(a, b, 0) };
            let rr = unsafe { fr(a, b, 0) };
            assert_eq!(rc, rr, "select_operation({op})({a},{b},0)");
            assert_eq!(rc, a.wrapping_add(b), "default branch must add");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — get_computation_timestamp
// ---------------------------------------------------------------------------
#[test]
fn row_11_get_computation_timestamp() {
    let p = pair();
    for i in 0..50 {
        let c = p.c.get_computation_timestamp();
        let r = p.rs.get_computation_timestamp();
        assert_eq!(
            c, r,
            "get_computation_timestamp() iteration {i}: C={c} Rust={r} \
             (time(NULL) >> 29 must agree)"
        );
        // Sanity: >> 29 of a plausible epoch second.
        assert!(c >= 3 && c <= 4, "unexpected timestamp {c}");
    }
}

// ---------------------------------------------------------------------------
// Rows 12 & 13 — allocate_results: non-NULL, fully zeroed, identical layout
// ---------------------------------------------------------------------------
#[test]
fn row_12_allocate_results_ten_is_zeroed() {
    let p = pair();
    let n = 10usize;
    let pc = p.c.allocate_results(10);
    let pr = p.rs.allocate_results(10);
    assert!(!pc.is_null(), "C allocate_results(10) returned NULL");
    assert!(!pr.is_null(), "Rust allocate_results(10) returned NULL");
    let bytes = n * std::mem::size_of::<ComputationResult>();
    let bc = unsafe { std::slice::from_raw_parts(pc as *const u8, bytes) };
    let br = unsafe { std::slice::from_raw_parts(pr as *const u8, bytes) };
    assert!(bc.iter().all(|&b| b == 0), "C calloc buffer was not zeroed");
    assert_eq!(bc, br, "allocate_results(10) buffers differ");
}

#[test]
fn row_13_allocate_results_sizes_and_layout() {
    let p = pair();
    assert_eq!(
        std::mem::size_of::<ComputationResult>(),
        24,
        "harness struct layout assumption"
    );
    for count in [1i32, 2, 3, 9, 10, 11, 100, 1000] {
        let pc = p.c.allocate_results(count);
        let pr = p.rs.allocate_results(count);
        assert!(!pc.is_null(), "C allocate_results({count}) == NULL");
        assert!(!pr.is_null(), "Rust allocate_results({count}) == NULL");
        let bytes = count as usize * std::mem::size_of::<ComputationResult>();
        let bc = unsafe { std::slice::from_raw_parts(pc as *const u8, bytes) };
        let br = unsafe { std::slice::from_raw_parts(pr as *const u8, bytes) };
        assert!(bc.iter().all(|&b| b == 0), "C buffer not zeroed for {count}");
        assert_eq!(bc, br, "allocate_results({count}) differ");
    }
}

/// Layout/stride agreement, proven by writing through the C and Rust
/// `perform_computation_with_history` and comparing the RAW BYTES. If the two
/// `.so`s disagreed about `sizeof(ComputationResult)` or field offsets, the
/// byte images would differ.
#[test]
fn row_13b_struct_stride_matches_byte_for_byte() {
    let p = pair();
    let n = 3usize;
    let mut hc = p.c.allocate_results(n as i32);
    let mut hr = p.rs.allocate_results(n as i32);
    let mut cc: std::os::raw::c_int = 0;
    let mut cr: std::os::raw::c_int = 0;
    for (i, (a, b)) in [(11, 22), (-33, 44), (i32::MAX, 1)].iter().enumerate() {
        let vc = unsafe { p.c.perform_computation_with_history(*a, *b, 1, &mut hc, &mut cc) };
        let vr = unsafe { p.rs.perform_computation_with_history(*a, *b, 1, &mut hr, &mut cr) };
        assert_eq!(vc, vr, "slot {i} return value");
        assert_eq!(cc, cr, "slot {i} history_count");
    }
    let bytes = n * std::mem::size_of::<ComputationResult>();
    let bc = unsafe { std::slice::from_raw_parts(hc as *const u8, bytes) };
    let br = unsafe { std::slice::from_raw_parts(hr as *const u8, bytes) };
    assert_eq!(bc, br, "ComputationResult array byte images differ (layout/stride mismatch)");
}
