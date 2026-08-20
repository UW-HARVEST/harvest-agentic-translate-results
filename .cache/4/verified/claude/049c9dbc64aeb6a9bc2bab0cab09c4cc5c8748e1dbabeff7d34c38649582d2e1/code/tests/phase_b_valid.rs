//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are loaded as shared objects and driven through
//! `dlsym`; the Rust crate is never called directly.

mod common;

use std::ffi::{c_char, c_int, c_uint, CString};

use common::{c, capture, diff, iters, r, ComputeState, Lib, OperationFunc, Rng, EDGES, SEED};

const NAMES: [&str; 4] = ["MULT", "ADD", "XOR", "SHIFT"];

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

// ===========================================================================
// C1..C2 — multiply_with_static
// ===========================================================================

#[test]
fn c1_multiply_random() {
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..iters(4000) {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff(&format!("C1 #{i} multiply({a},{b})"), move |l: &Lib| {
            l.multiply_with_static(a, b)
        });
    }
}

#[test]
fn c2_multiply_edges() {
    for &a in EDGES {
        for &b in EDGES {
            diff(&format!("C2 multiply({a},{b})"), move |l: &Lib| {
                l.multiply_with_static(a, b)
            });
        }
    }
}

// ===========================================================================
// C3..C4 — add_with_static
// ===========================================================================

#[test]
fn c3_add_random() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..iters(4000) {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff(&format!("C3 #{i} add({a},{b})"), move |l: &Lib| {
            l.add_with_static(a, b)
        });
    }
}

#[test]
fn c4_add_edges() {
    // Explicit wrap triggers on top of the full edge cross product.
    let extra: &[(i32, i32)] = &[
        (i32::MAX, 1),
        (i32::MAX, 100),
        (i32::MAX - 99, 0),
        (i32::MAX - 100, 0),
        (i32::MIN, -1),
        (i32::MIN, -100),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
    ];
    for &(a, b) in extra {
        diff(&format!("C4 add-wrap({a},{b})"), move |l: &Lib| {
            l.add_with_static(a, b)
        });
    }
    for &a in EDGES {
        for &b in EDGES {
            diff(&format!("C4 add({a},{b})"), move |l: &Lib| {
                l.add_with_static(a, b)
            });
        }
    }
}

// ===========================================================================
// C5..C6 — xor_operation
// ===========================================================================

#[test]
fn c5_xor_random() {
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..iters(4000) {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff(&format!("C5 #{i} xor({a},{b})"), move |l: &Lib| {
            l.xor_operation(a, b)
        });
    }
}

#[test]
fn c6_xor_edges() {
    let mut vals: Vec<i32> = EDGES.to_vec();
    vals.push(0xABCD);
    vals.push(!0xABCD);
    for &a in &vals {
        for &b in &vals {
            diff(&format!("C6 xor({a},{b})"), move |l: &Lib| {
                l.xor_operation(a, b)
            });
        }
    }
}

// ===========================================================================
// C7..C10 — shift_with_static  ((a << 2) | (b >> 2))
// ===========================================================================

#[test]
fn c7_shift_pos_pos() {
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..iters(2000) {
        let a = (rng.next_u32() >> 1) as i32; // >= 0
        let b = (rng.next_u32() >> 1) as i32; // >= 0
        diff(&format!("C7 #{i} shift({a},{b})"), move |l: &Lib| {
            l.shift_with_static(a, b)
        });
    }
}

#[test]
fn c8_shift_neg_a() {
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..iters(2000) {
        let a = -((rng.next_u32() >> 1) as i32) - 1; // < 0
        let b = (rng.next_u32() >> 1) as i32; // >= 0
        diff(&format!("C8 #{i} shift({a},{b})"), move |l: &Lib| {
            l.shift_with_static(a, b)
        });
    }
}

#[test]
fn c9_shift_neg_b() {
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..iters(2000) {
        let a = (rng.next_u32() >> 1) as i32; // >= 0
        let b = -((rng.next_u32() >> 1) as i32) - 1; // < 0 -> arithmetic >>
        diff(&format!("C9 #{i} shift({a},{b})"), move |l: &Lib| {
            l.shift_with_static(a, b)
        });
    }
}

#[test]
fn c10_shift_random_and_edges() {
    let mut rng = Rng::new(SEED ^ 10);
    // both negative
    for i in 0..iters(1000) {
        let a = -((rng.next_u32() >> 1) as i32) - 1;
        let b = -((rng.next_u32() >> 1) as i32) - 1;
        diff(&format!("C10 negneg #{i} shift({a},{b})"), move |l: &Lib| {
            l.shift_with_static(a, b)
        });
    }
    // unconstrained full range
    for i in 0..iters(2000) {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff(&format!("C10 rand #{i} shift({a},{b})"), move |l: &Lib| {
            l.shift_with_static(a, b)
        });
    }
    // edge cross product
    for &a in EDGES {
        for &b in EDGES {
            diff(&format!("C10 edge shift({a},{b})"), move |l: &Lib| {
                l.shift_with_static(a, b)
            });
        }
    }
}

// ===========================================================================
// C11..C13 — get_operation
// ===========================================================================

#[test]
fn c11_get_operation_pointer_identity() {
    for opcode in 0..4 {
        for lib in [c(), r()] {
            let (got, out) = capture(|| lib.get_operation(opcode));
            assert!(out.is_empty(), "get_operation printed something in {}", lib.name);
            let got = got.unwrap_or_else(|| {
                panic!("{} get_operation({opcode}) returned NULL", lib.name)
            });
            let want = lib.op_symbol_addr(opcode);
            assert_eq!(
                got as usize, want,
                "{} get_operation({opcode}) must return the address of its own \
                 exported symbol {} (got {:#x}, exported {:#x})",
                lib.name, NAMES[opcode as usize], got as usize, want
            );
        }
    }
    // Non-NULL-ness and distinctness agree between the two libraries.
    for opcode in 0..4i32 {
        assert_eq!(
            c().get_operation(opcode).is_some(),
            r().get_operation(opcode).is_some(),
            "NULL-ness disagreement at opcode {opcode}"
        );
    }
    let mut caddr: Vec<usize> = (0..4).map(|i| c().get_operation(i).unwrap() as usize).collect();
    let mut raddr: Vec<usize> = (0..4).map(|i| r().get_operation(i).unwrap() as usize).collect();
    caddr.sort_unstable();
    caddr.dedup();
    raddr.sort_unstable();
    raddr.dedup();
    assert_eq!(caddr.len(), 4, "C table has duplicate entries");
    assert_eq!(raddr.len(), 4, "Rust table has duplicate entries");
}

#[test]
fn c12_get_operation_lazy_init_idempotent() {
    // First-call vs. later-call branch (`if (ops[0] == NULL)`): the table must
    // be filled exactly once and stay pointer-stable.
    let mut rng = Rng::new(SEED ^ 12);
    let first: Vec<[usize; 4]> = [c(), r()]
        .iter()
        .map(|lib| {
            let mut a = [0usize; 4];
            for i in 0..4 {
                a[i] = lib.get_operation(i as c_int).unwrap() as usize;
            }
            a
        })
        .collect();

    for i in 0..iters(1000) {
        let opcode = rng.below(4) as c_int;
        for (li, lib) in [c(), r()].iter().enumerate() {
            let (got, out) = capture(|| lib.get_operation(opcode));
            assert!(out.is_empty(), "iteration {i}: unexpected output");
            assert_eq!(
                got.unwrap() as usize, first[li][opcode as usize],
                "{} get_operation({opcode}) not pointer-stable at iteration {i}",
                lib.name
            );
        }
    }
}

#[test]
fn c13_get_operation_dispatch_matches_direct() {
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..iters(2000) {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        for opcode in 0..4i32 {
            // dispatch through the returned pointer, in each library
            let (cv, cout) = capture(|| unsafe { (c().get_operation(opcode).unwrap())(a, b) });
            let (rv, rout) = capture(|| unsafe { (r().get_operation(opcode).unwrap())(a, b) });
            assert!(cout.is_empty() && rout.is_empty());
            assert_eq!(cv, rv, "C13 #{i} dispatch opcode {opcode} ({a},{b})");

            // ... and it must equal calling the exported symbol directly
            let direct = match opcode {
                0 => c().multiply_with_static(a, b),
                1 => c().add_with_static(a, b),
                2 => c().xor_operation(a, b),
                _ => c().shift_with_static(a, b),
            };
            assert_eq!(cv, direct, "C13 #{i} dispatch != direct for opcode {opcode}");
        }
    }
}

// ===========================================================================
// C14..C16 — execute_operation
// ===========================================================================

#[test]
fn c14_execute_operation_all_ops() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..iters(600) {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        for opcode in 0..4i32 {
            let name = cstr(NAMES[opcode as usize]);
            let np = name.as_ptr();
            diff(
                &format!("C14 #{i} exec opcode {opcode} ({a},{b})"),
                move |l: &Lib| unsafe { l.execute_operation(l.get_operation(opcode), a, b, np) },
            );
        }
    }
}

#[test]
fn c15_execute_operation_op_name_shapes() {
    let long = "N".repeat(200);
    let shapes: [&str; 8] = [
        "",
        "X",
        "XOR",
        "SHIFT",
        "%d",
        "%s %s %s",
        "100%% done",
        &long,
    ];
    let mut rng = Rng::new(SEED ^ 15);
    for (si, s) in shapes.iter().enumerate() {
        let name = cstr(s);
        let np = name.as_ptr();
        for opcode in 0..4i32 {
            for i in 0..iters(8) {
                let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
                diff(
                    &format!("C15 shape {si} opcode {opcode} #{i} ({a},{b})"),
                    move |l: &Lib| unsafe {
                        l.execute_operation(l.get_operation(opcode), a, b, np)
                    },
                );
            }
        }
    }
}

/// A caller-minted `operation_func` that belongs to neither `.so`.
extern "C" fn foreign_op(a: c_int, b: c_int) -> c_int {
    a.wrapping_sub(b).wrapping_mul(7) ^ 0x1234_5678
}

#[test]
fn c16_execute_operation_foreign_func() {
    let f: OperationFunc = Some(foreign_op as unsafe extern "C" fn(c_int, c_int) -> c_int);
    let name = cstr("FOREIGN");
    let np = name.as_ptr();
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..iters(500) {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff(&format!("C16 #{i} foreign({a},{b})"), move |l: &Lib| unsafe {
            l.execute_operation(f, a, b, np)
        });
    }
}

// ===========================================================================
// C17..C18 — compute_checksum
// ===========================================================================

#[test]
fn c17_checksum_counts_1_to_4() {
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..iters(1500) {
        let vals: [c_int; 4] = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        for count in 1..=4i32 {
            diff(
                &format!("C17 #{i} count {count} {vals:?}"),
                move |l: &Lib| {
                    let mut v = vals;
                    unsafe { l.compute_checksum(v.as_mut_ptr(), count) }
                },
            );
        }
    }
}

#[test]
fn c18_checksum_byte_patterns() {
    let patterns: &[[c_int; 4]] = &[
        [0, 0, 0, 0],
        [-1, -1, -1, -1],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [0x0102_0304, 0x0506_0708, 0x090A_0B0C, 0x0D0E_0F10],
        [
            0x5555_5555u32 as i32,
            0xAAAA_AAAAu32 as i32,
            0x5555_5555u32 as i32,
            0xAAAA_AAAAu32 as i32,
        ],
        [0x0000_00FF, 0x0000_FF00, 0x00FF_0000, 0xFF00_0000u32 as i32],
        [1, 0, -1, i32::MIN],
        [0x8000_0000u32 as i32, 1, 0x7FFF_FFFF, -2],
        [0xDEAD_BEEFu32 as i32, 0xFFFFu32 as i32, 0, 0xABCD],
    ];
    for (pi, p) in patterns.iter().enumerate() {
        for count in 1..=4i32 {
            let p = *p;
            diff(&format!("C18 pattern {pi} count {count}"), move |l: &Lib| {
                let mut v = p;
                unsafe { l.compute_checksum(v.as_mut_ptr(), count) }
            });
        }
    }
    // Random byte-level fuzzing of the raw object representation.
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..iters(2000) {
        let mut bytes = [0u8; 16];
        for b in bytes.iter_mut() {
            *b = rng.next_u32() as u8;
        }
        let vals: [c_int; 4] = unsafe { std::mem::transmute(bytes) };
        for count in 1..=4i32 {
            diff(&format!("C18 fuzz #{i} count {count}"), move |l: &Lib| {
                let mut v = vals;
                unsafe { l.compute_checksum(v.as_mut_ptr(), count) }
            });
        }
    }
}

// ===========================================================================
// C19 — init_state
// ===========================================================================

#[test]
fn c19_init_state_fresh_and_dirty() {
    let mut rng = Rng::new(SEED ^ 19);
    let mut values: Vec<i32> = EDGES.to_vec();
    for _ in 0..iters(300) {
        values.push(rng.interesting_i32());
    }
    for (vi, &v) in values.iter().enumerate() {
        // fresh (zeroed) destination
        common::diff_bytes(&format!("C19 fresh #{vi} v={v}"), move |l: &Lib| {
            let mut st = ComputeState::default();
            unsafe { l.init_state(&mut st, v) };
            ((), st.bytes().to_vec())
        });
        // destination pre-filled with garbage: all three fields must be replaced
        let dirty = ComputeState {
            accumulator: 0x1BAD_F00Du32 as i32,
            operation_count: -12345,
            checksum: 0xFFFF_FFFF,
        };
        common::diff_bytes(&format!("C19 dirty #{vi} v={v}"), move |l: &Lib| {
            let mut st = dirty;
            unsafe { l.init_state(&mut st, v) };
            ((), st.bytes().to_vec())
        });
    }
}

// ===========================================================================
// C20..C21 — apply_operation
// ===========================================================================

#[test]
fn c20_apply_operation_single() {
    let mut rng = Rng::new(SEED ^ 20);
    let counts: &[c_int] = &[0, 1, 7, i32::MAX - 1, i32::MAX, -1, i32::MIN];
    for i in 0..iters(400) {
        let acc = rng.interesting_i32();
        let value = rng.interesting_i32();
        let checksum = rng.next_u32() as c_uint;
        let opcount = *rng.pick(counts);
        for opcode in 0..4i32 {
            common::diff_bytes(
                &format!("C20 #{i} opcode {opcode} acc={acc} v={value} n={opcount}"),
                move |l: &Lib| {
                    let mut st = ComputeState {
                        accumulator: acc,
                        operation_count: opcount,
                        checksum,
                    };
                    unsafe { l.apply_operation(&mut st, value, l.get_operation(opcode)) };
                    ((), st.bytes().to_vec())
                },
            );
        }
    }
}

#[test]
fn c21_apply_operation_chains() {
    let mut rng = Rng::new(SEED ^ 21);
    for &len in &[2usize, 3, 10, 64] {
        for i in 0..iters(60) {
            let initial = rng.interesting_i32();
            let steps: Vec<(c_int, c_int)> = (0..len)
                .map(|_| (rng.below(4) as c_int, rng.interesting_i32()))
                .collect();
            let steps_ref = &steps;
            common::diff_bytes(
                &format!("C21 len {len} #{i} init={initial}"),
                move |l: &Lib| {
                    let mut st = ComputeState::default();
                    unsafe { l.init_state(&mut st, initial) };
                    for &(opcode, value) in steps_ref.iter() {
                        unsafe { l.apply_operation(&mut st, value, l.get_operation(opcode)) };
                    }
                    ((), st.bytes().to_vec())
                },
            );
        }
    }
}

// ===========================================================================
// C22 — hand-composed pipeline over the low-level entry points
// ===========================================================================

#[test]
fn c22_composed_pipeline() {
    let mut rng = Rng::new(SEED ^ 22);
    let xor_name = cstr("XOR");
    let shift_name = cstr("SHIFT");
    let xn = xor_name.as_ptr();
    let sn = shift_name.as_ptr();

    for i in 0..iters(400) {
        let p: [c_int; 4] = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        // Extra mid-pipeline operations so the composition differs from the
        // fixed `checkshift` sequence.
        let extra: Vec<(c_int, c_int)> = (0..rng.below(5))
            .map(|_| (rng.below(4) as c_int, rng.interesting_i32()))
            .collect();
        let extra_ref = &extra;
        let count = 1 + (i as c_int % 4);

        common::diff_bytes(&format!("C22 #{i} p={p:?} count={count}"), move |l: &Lib| {
            let mut st = ComputeState::default();
            unsafe { l.init_state(&mut st, p[0]) };

            let mult = l.get_operation(0);
            let add = l.get_operation(1);
            let xor = l.get_operation(2);
            let shift = l.get_operation(3);

            unsafe { l.apply_operation(&mut st, p[1], mult) };
            unsafe { l.apply_operation(&mut st, p[2], add) };
            for &(opcode, value) in extra_ref.iter() {
                unsafe { l.apply_operation(&mut st, value, l.get_operation(opcode)) };
            }

            let xor_result = unsafe { l.execute_operation(xor, st.accumulator, p[3], xn) };
            let shift_result = unsafe { l.execute_operation(shift, xor_result, p[1], sn) };

            let mut params = p;
            st.checksum = unsafe { l.compute_checksum(params.as_mut_ptr(), count) };

            let final_result =
                (st.accumulator.wrapping_add(shift_result) as u32 ^ st.checksum) as c_int;

            ((xor_result, shift_result, final_result), st.bytes().to_vec())
        });
    }
}

// ===========================================================================
// C23..C24 — checkshift (the public one-shot wrapper)
// ===========================================================================

#[test]
fn c23_checkshift_random() {
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..iters(1200) {
        let (a, b, cc, d) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff(
            &format!("C23 #{i} checkshift({a},{b},{cc},{d})"),
            move |l: &Lib| l.checkshift(a, b, cc, d),
        );
    }
}

#[test]
fn c24_checkshift_edges() {
    let uniform: &[i32] = &[0, 1, -1, i32::MAX, i32::MIN, 0x7FFF, -0x8000];
    for &v in uniform {
        diff(&format!("C24 uniform {v}"), move |l: &Lib| {
            l.checkshift(v, v, v, v)
        });
    }
    // each edge value in each position, others held at a fixed non-trivial base
    let base = [0x1234, -0x4321, 7, -9];
    for pos in 0..4usize {
        for &v in EDGES {
            let mut p = base;
            p[pos] = v;
            diff(&format!("C24 pos {pos} v={v}"), move |l: &Lib| {
                l.checkshift(p[0], p[1], p[2], p[3])
            });
        }
    }
    // mixed-sign combinations
    let mix: &[i32] = &[i32::MIN, -1, 0, 1, i32::MAX];
    for &a in mix {
        for &b in mix {
            for &cc in mix {
                for &d in mix {
                    diff(&format!("C24 mix({a},{b},{cc},{d})"), move |l: &Lib| {
                        l.checkshift(a, b, cc, d)
                    });
                }
            }
        }
    }
}

// ===========================================================================
// C25 — cross-`.so` ABI: function pointers and structs handed between the two
// ===========================================================================

#[test]
fn c25_cross_module_abi() {
    let name = cstr("CROSS");
    let np = name.as_ptr();
    let mut rng = Rng::new(SEED ^ 25);

    for i in 0..iters(300) {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        for opcode in 0..4i32 {
            // --- fn pointer provenance x callee, all four combinations -------
            let mut results = Vec::new();
            let mut outs = Vec::new();
            for (pname, provider) in [("C", c()), ("Rust", r())] {
                for (cname, callee) in [("C", c()), ("Rust", r())] {
                    let f = provider.get_operation(opcode);
                    let (v, out) = capture(|| unsafe { callee.execute_operation(f, a, b, np) });
                    results.push((format!("{pname}->{cname}"), v));
                    outs.push((format!("{pname}->{cname}"), out));
                }
            }
            let (ref tag0, v0) = results[0];
            for (tag, v) in &results[1..] {
                assert_eq!(
                    v0, *v,
                    "C25 #{i} opcode {opcode}: execute_operation value differs \
                     {tag0} vs {tag} ({a},{b})"
                );
            }
            let (ref otag0, ref o0) = outs[0];
            for (tag, o) in &outs[1..] {
                assert_eq!(
                    o0, o,
                    "C25 #{i} opcode {opcode}: execute_operation stdout differs \
                     {otag0} vs {tag}\n  {}\n  {}",
                    common::show(o0),
                    common::show(o)
                );
            }

            // --- apply_operation with a foreign fn pointer -------------------
            let mut states = Vec::new();
            for (pname, provider) in [("C", c()), ("Rust", r())] {
                for (cname, callee) in [("C", c()), ("Rust", r())] {
                    let f = provider.get_operation(opcode);
                    let (st, out) = capture(|| {
                        let mut st = ComputeState {
                            accumulator: a,
                            operation_count: 3,
                            checksum: 0xBEEF,
                        };
                        unsafe { callee.apply_operation(&mut st, b, f) };
                        st
                    });
                    assert!(out.is_empty(), "apply_operation should be silent here");
                    states.push((format!("{pname}->{cname}"), st));
                }
            }
            for (tag, st) in &states[1..] {
                assert_eq!(
                    states[0].1, *st,
                    "C25 #{i} opcode {opcode}: apply_operation state differs {} vs {tag}",
                    states[0].0
                );
            }
        }

        // --- struct handed across: C writes it, Rust advances it, and vice versa
        let initial = rng.interesting_i32();
        let v1 = rng.interesting_i32();
        let v2 = rng.interesting_i32();
        let mut transcripts = Vec::new();
        for (tag, writer, advancer) in [
            ("C/C", c(), c()),
            ("C/Rust", c(), r()),
            ("Rust/C", r(), c()),
            ("Rust/Rust", r(), r()),
        ] {
            let (st, out) = capture(|| {
                let mut st = ComputeState::default();
                unsafe { writer.init_state(&mut st, initial) };
                unsafe { advancer.apply_operation(&mut st, v1, advancer.get_operation(0)) };
                unsafe { advancer.apply_operation(&mut st, v2, writer.get_operation(1)) };
                let mut params = [initial, v1, v2, 0];
                st.checksum = unsafe { advancer.compute_checksum(params.as_mut_ptr(), 4) };
                st
            });
            transcripts.push((tag, st, out));
        }
        for (tag, st, out) in &transcripts[1..] {
            assert_eq!(
                transcripts[0].1, *st,
                "C25 #{i} cross state differs {} vs {tag}",
                transcripts[0].0
            );
            assert_eq!(
                &transcripts[0].2,
                out,
                "C25 #{i} cross stdout differs {} vs {tag}",
                transcripts[0].0
            );
        }
    }
}

// ===========================================================================
// ABI sanity: struct size/alignment agreement (types have no symbols, so this
// is checked behaviourally through the two `.so`s)
// ===========================================================================

#[test]
fn c25b_compute_state_layout_agrees() {
    assert_eq!(std::mem::size_of::<ComputeState>(), 12);
    assert_eq!(std::mem::align_of::<ComputeState>(), 4);

    // A distinctive value in each field, written by each library, must land in
    // the same bytes.
    let (cb, _) = capture(|| {
        let mut st = ComputeState::default();
        unsafe { c().init_state(&mut st, 0x0A0B_0C0D) };
        unsafe { c().apply_operation(&mut st, 0, c().get_operation(2)) };
        st.bytes()
    });
    let (rb, _) = capture(|| {
        let mut st = ComputeState::default();
        unsafe { r().init_state(&mut st, 0x0A0B_0C0D) };
        unsafe { r().apply_operation(&mut st, 0, r().get_operation(2)) };
        st.bytes()
    });
    assert_eq!(cb, rb, "ComputeState object representation differs");
}

// A trivially-referenced item so the `c_char` import is always used.
const _: Option<*const c_char> = None;
