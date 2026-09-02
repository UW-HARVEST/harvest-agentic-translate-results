// Phase B — CONFIGS.md rows 1-10: the leaf operations and `get_operation`,
// the lowest-level entry points in the library.
//
// Everything is called through the `.so` exports of BOTH libraries.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// Randomized differential check of one leaf op (`n` draws, fixed seed).
fn diff_leaf_random(opcode: c_int, seed: u64, n: usize) {
    let (c, r) = both();
    let cf = c.leaf(opcode);
    let rf = r.leaf(opcode);
    let mut rng = Rng::new(seed);
    for i in 0..n {
        let (a, b) = (rng.next_i32_biased(), rng.next_i32_biased());
        let (cv, rv) = unsafe { (cf(a, b), rf(a, b)) };
        assert_eq!(
            cv,
            rv,
            "{}({a}, {b}) [draw {i}]: C returned {cv} but Rust returned {rv}",
            Api::leaf_name(opcode)
        );
    }
}

/// Full boundary grid for one leaf op.
fn diff_leaf_grid(opcode: c_int, extra: &[c_int]) {
    let (c, r) = both();
    let cf = c.leaf(opcode);
    let rf = r.leaf(opcode);
    let mut vals: Vec<c_int> = INTERESTING.to_vec();
    vals.extend_from_slice(extra);
    for &a in &vals {
        for &b in &vals {
            let (cv, rv) = unsafe { (cf(a, b), rf(a, b)) };
            assert_eq!(
                cv,
                rv,
                "{}({a}, {b}): C returned {cv} but Rust returned {rv}",
                Api::leaf_name(opcode)
            );
        }
    }
}

// --- row 1 / 2: multiply_with_static (static_multiplier = 3, signed wrap) -----
#[test]
fn cfg01_multiply_random() {
    diff_leaf_random(0, 0x0100_0001, 20_000);
}

#[test]
fn cfg02_multiply_grid() {
    diff_leaf_grid(0, &[0x5555_5555, -0x5555_5555, 0x7FFF_FFFF / 3 + 1, i32::MIN / 3 - 1]);
}

// --- row 3 / 4: add_with_static (static_addend = 100, signed wrap) ------------
#[test]
fn cfg03_add_random() {
    diff_leaf_random(1, 0x0300_0003, 20_000);
}

#[test]
fn cfg04_add_grid() {
    diff_leaf_grid(1, &[i32::MAX - 99, i32::MAX - 100, i32::MIN + 99, i32::MIN + 101, -99, -101]);
}

// --- row 5 / 6: xor_operation (^ 0xABCD) --------------------------------------
#[test]
fn cfg05_xor_random() {
    diff_leaf_random(2, 0x0500_0005, 20_000);
}

#[test]
fn cfg06_xor_grid() {
    diff_leaf_grid(2, &[0xABCD, 0xABCC, 0xABCE, !0xABCD, 0xFFFF_ABCD_u32 as c_int]);
}

// --- row 7 / 8: shift_with_static (a << 2 | b >> 2, both signed) --------------
#[test]
fn cfg07_shift_random() {
    diff_leaf_random(3, 0x0700_0007, 20_000);
}

#[test]
fn cfg08_shift_grid_high_bits_and_negatives() {
    // `a << 2` discards the top two bits (signed overflow in C, plain shift in
    // practice); `b >> 2` is an arithmetic shift that fills with the sign bit.
    let mut extra: Vec<c_int> = Vec::new();
    for bit in 0..32u32 {
        extra.push((1u32 << bit) as c_int); // 1-hot
        extra.push(!((1u32 << bit) as c_int)); // 1-cold
    }
    extra.extend_from_slice(&[
        0x2000_0000, 0x3FFF_FFFF, 0x4000_0000, 0x6000_0000, 0x7FFF_FFFF, -1, -2, -3, -4, -5,
        i32::MIN, i32::MIN + 1, i32::MIN + 3,
    ]);
    diff_leaf_grid(3, &extra);
}

// --- row 9: get_operation dispatch -------------------------------------------
#[test]
fn cfg09_get_operation_sweep_and_dispatch() {
    let (c, r) = both();
    let mut opcodes: Vec<c_int> = (-8..=8).collect();
    opcodes.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0x01, 0x02, 0x03, 0x04]);

    for &op in &opcodes {
        let (cp, rp) = unsafe { ((c.get_operation)(op), (r.get_operation)(op)) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "get_operation({op}): C null={} but Rust null={}",
            cp.is_null(),
            rp.is_null()
        );
        if cp.is_null() {
            continue;
        }
        assert!((0..4).contains(&op), "get_operation({op}) unexpectedly non-null in C");

        // The dispatched pointers must behave identically to each other and to
        // the directly-exported leaf of the same opcode in the same library.
        let cf: BinOp = unsafe { std::mem::transmute(cp) };
        let rf: BinOp = unsafe { std::mem::transmute(rp) };
        let c_direct = c.leaf(op);
        let r_direct = r.leaf(op);

        let mut rng = Rng::new(0x0900_0000 ^ (op as u64));
        for _ in 0..2_000 {
            let (a, b) = (rng.next_i32_biased(), rng.next_i32_biased());
            let (cv, rv) = unsafe { (cf(a, b), rf(a, b)) };
            assert_eq!(cv, rv, "get_operation({op})->fn({a}, {b}): C={cv} Rust={rv}");
            let (cd, rd) = unsafe { (c_direct(a, b), r_direct(a, b)) };
            assert_eq!(cv, cd, "C get_operation({op}) dispatched to the wrong leaf");
            assert_eq!(rv, rd, "Rust get_operation({op}) dispatched to the wrong leaf");
        }
    }
}

// --- row 10: repeated calls (the lazy `static` table guard) -------------------

/// The C `get_operation` fills a function-`static` table on first use, which is a
/// benign but real race when called from several threads. The Rust builds the
/// table locally instead; this checks that the difference stays unobservable and
/// that the Rust `.so` (built with `panic = "abort"`) does not abort where the C
/// keeps going. Only the non-printing entry points are used, so no fd-1
/// redirection is involved.
#[test]
fn cfg10b_concurrent_dispatch_and_leaf_ops() {
    let (c, r) = both();
    let threads: Vec<_> = (0..8u64)
        .map(|tid| {
            std::thread::spawn(move || {
                let mut rng = Rng::new(0x10B0_0000 ^ tid);
                for _ in 0..20_000 {
                    let op = (rng.next_u32() % 7) as c_int - 2;
                    let (cp, rp) = unsafe { ((c.get_operation)(op), (r.get_operation)(op)) };
                    assert_eq!(cp.is_null(), rp.is_null(), "get_operation({op}) nullness diverged");
                    if cp.is_null() {
                        continue;
                    }
                    let cf: BinOp = unsafe { std::mem::transmute(cp) };
                    let rf: BinOp = unsafe { std::mem::transmute(rp) };
                    let (a, b) = (rng.next_i32_biased(), rng.next_i32_biased());
                    let (cv, rv) = unsafe { (cf(a, b), rf(a, b)) };
                    assert_eq!(cv, rv, "op {op} ({a}, {b}): C={cv} Rust={rv}");

                    let mut buf: [c_int; 4] = [a, b, a ^ b, a.wrapping_sub(b)];
                    let count = (rng.next_u32() % 7) as c_int - 1;
                    let (ccs, rcs) = unsafe {
                        (
                            (c.compute_checksum)(buf.as_mut_ptr(), count),
                            (r.compute_checksum)(buf.as_mut_ptr(), count),
                        )
                    };
                    assert_eq!(ccs, rcs, "compute_checksum({buf:?}, {count}) diverged");
                }
            })
        })
        .collect();
    for t in threads {
        t.join().expect("a worker thread failed");
    }
}

#[test]
fn cfg10_get_operation_repeat_calls() {
    let (c, r) = both();
    let mut first: Vec<(*const c_void, *const c_void)> = Vec::new();
    for op in -2..6 {
        first.push(unsafe { ((c.get_operation)(op), (r.get_operation)(op)) });
    }
    // Interleave 1000 more rounds; the C table is filled in on the first call
    // only, so a stale/partial table would show up as changing results here.
    for round in 0..1_000 {
        for (i, op) in (-2..6).enumerate() {
            let (cp, rp) = unsafe { ((c.get_operation)(op), (r.get_operation)(op)) };
            assert_eq!(cp, first[i].0, "C get_operation({op}) changed on round {round}");
            assert_eq!(rp, first[i].1, "Rust get_operation({op}) changed on round {round}");
            assert_eq!(cp.is_null(), rp.is_null(), "get_operation({op}) nullness diverged");
        }
    }
}
