// Phase B — CONFIGS.md rows C1..C9
//
// The four lowest-level arithmetic kernels, called directly through both `.so`s.
// These are the leaves of the call graph; `checkshift` can only ever reach them
// with the specific values its own pipeline produces, so they must be driven
// directly to cover signed overflow / shift / sign-extension behaviour.
//
//   multiply_with_static(a,b) = (a * b) * 3
//   add_with_static(a,b)      = (a + b) + 100
//   xor_operation(a,b)        = a ^ b ^ 0xABCD
//   shift_with_static(a,b)    = (a << 2) | (b >> 2)

mod common;
use common::*;

const SEED: u64 = 0x5EED_C0FF_EE00_1234;
const N_RANDOM: usize = 2000;

/// Drive one kernel over `n` random pairs and compare C vs Rust.
fn diff_kernel_random(idx: usize, label: &str, n: usize, seed: u64) {
    let (c, r) = libs();
    let mut rng = Rng::new(seed);
    let (cf, rf) = (c.kernel(idx), r.kernel(idx));
    for i in 0..n {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let cv = unsafe { cf(a, b) };
        let rv = unsafe { rf(a, b) };
        assert_eq!(
            cv, rv,
            "{label}: iteration {i} with a={a} (0x{a:08X}), b={b} (0x{b:08X})"
        );
    }
}

/// Drive one kernel over the full boundary grid and compare C vs Rust.
fn diff_kernel_grid(idx: usize, label: &str) {
    let (c, r) = libs();
    let (cf, rf) = (c.kernel(idx), r.kernel(idx));
    for &a in INTERESTING {
        for &b in INTERESTING {
            let cv = unsafe { cf(a, b) };
            let rv = unsafe { rf(a, b) };
            assert_eq!(
                cv, rv,
                "{label}: grid a={a} (0x{a:08X}), b={b} (0x{b:08X})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C1 / C2 — multiply_with_static
// ---------------------------------------------------------------------------

#[test]
fn c1_multiply_with_static_random() {
    diff_kernel_random(0, "C1 multiply_with_static", N_RANDOM, SEED ^ 1);
}

#[test]
fn c2_multiply_with_static_boundary_grid() {
    diff_kernel_grid(0, "C2 multiply_with_static");
}

// ---------------------------------------------------------------------------
// C3 / C4 — add_with_static
// ---------------------------------------------------------------------------

#[test]
fn c3_add_with_static_random() {
    diff_kernel_random(1, "C3 add_with_static", N_RANDOM, SEED ^ 2);
}

#[test]
fn c4_add_with_static_boundary_grid() {
    // Includes INT_MAX + 100 and INT_MIN - 100, i.e. signed overflow wrap.
    diff_kernel_grid(1, "C4 add_with_static");
}

// ---------------------------------------------------------------------------
// C5 / C6 — xor_operation
// ---------------------------------------------------------------------------

#[test]
fn c5_xor_operation_random() {
    diff_kernel_random(2, "C5 xor_operation", N_RANDOM, SEED ^ 3);
}

#[test]
fn c6_xor_operation_boundary_grid() {
    diff_kernel_grid(2, "C6 xor_operation");
    // Values whose low 16 bits interact with the 0xABCD constant.
    let (c, r) = libs();
    for a in [0xABCDi32, !0xABCDi32, 0x0000_ABCD, 0xFFFF_5432u32 as i32] {
        for b in [0xABCDi32, 0, -1, 0x1234_ABCD] {
            assert_eq!(
                unsafe { (c.xor_operation)(a, b) },
                unsafe { (r.xor_operation)(a, b) },
                "C6 xor_operation: a=0x{a:08X}, b=0x{b:08X}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C7 / C8 / C9 — shift_with_static  ((a << 2) | (b >> 2))
// ---------------------------------------------------------------------------

#[test]
fn c7_shift_with_static_random() {
    diff_kernel_random(3, "C7 shift_with_static", N_RANDOM, SEED ^ 4);
}

#[test]
fn c8_shift_with_static_shift_out_and_sign_extension() {
    let (c, r) = libs();

    // `a` values whose top two bits are shifted out by `a << 2`.
    let a_shapes: &[i32] = &[
        0,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        0x4000_0000,
        0x6000_0000,
        0x2000_0000,
        0xC000_0000u32 as i32,
        0xE000_0000u32 as i32,
        0x8000_0000u32 as i32,
        0x3FFF_FFFF,
        -0x4000_0000,
        0x5555_5555,
        0xAAAA_AAAAu32 as i32,
    ];
    // `b` values exercising the ARITHMETIC right shift (sign extension) and the
    // low bits that get discarded.
    let b_shapes: &[i32] = &[
        0, 1, 2, 3, -1, -2, -3, -4, -5, -7, i32::MAX, i32::MIN, -0x7FFF_FFFF, 0x7FFF_FFFD,
        0xFFFF_FFFDu32 as i32,
    ];

    for &a in a_shapes {
        for &b in b_shapes {
            let cv = unsafe { (c.shift_with_static)(a, b) };
            let rv = unsafe { (r.shift_with_static)(a, b) };
            assert_eq!(
                cv, rv,
                "C8 shift_with_static: a={a} (0x{a:08X}), b={b} (0x{b:08X})"
            );
        }
    }
}

#[test]
fn c9_shift_with_static_boundary_grid() {
    diff_kernel_grid(3, "C9 shift_with_static");
}

// ---------------------------------------------------------------------------
// The kernels must be silent: none of them prints anything in C.
// ---------------------------------------------------------------------------

#[test]
fn kernels_produce_no_output() {
    let (c, r) = libs();
    for k in 0..4usize {
        let (cv, co) = capture(|| unsafe { (c.kernel(k))(12345, -6789) });
        let (rv, ro) = capture(|| unsafe { (r.kernel(k))(12345, -6789) });
        assert_eq!(cv, rv, "kernel {k} return value");
        assert!(co.is_empty(), "C kernel {k} unexpectedly printed {:?}", show(&co));
        assert_stdout_eq(&format!("kernel {k} silence"), &co, &ro);
    }
}
