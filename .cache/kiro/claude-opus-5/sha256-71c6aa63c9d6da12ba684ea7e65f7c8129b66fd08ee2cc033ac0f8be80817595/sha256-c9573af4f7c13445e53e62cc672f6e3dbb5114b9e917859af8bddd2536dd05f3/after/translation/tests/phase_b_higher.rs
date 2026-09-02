// Phase B — CONFIGS.md rows 11-24: the higher-order / stateful entry points
// (`execute_operation`, `compute_checksum`, `init_state`, `apply_operation`),
// including cross-library function-pointer and cross-library state combinations.

mod common;

use common::*;
use std::ffi::{c_int, c_uint, c_void};

fn main() {
    let mut t = Runner::new();
    t.case("cfg11_execute_operation_same_library", cfg11_execute_operation_same_library);
    t.case("cfg12_execute_operation_cross_library", cfg12_execute_operation_cross_library);
    t.case("cfg13_execute_operation_op_name_shapes", cfg13_execute_operation_op_name_shapes);
    t.case("cfg14_17_checksum_counts_1_to_4", cfg14_17_checksum_counts_1_to_4);
    t.case("cfg18_checksum_count_clamped_above_four", cfg18_checksum_count_clamped_above_four);
    t.case("cfg19_checksum_byte_order_patterns", cfg19_checksum_byte_order_patterns);
    t.case("cfg19b_checksum_always_masked_to_16_bits", cfg19b_checksum_always_masked_to_16_bits);
    t.case("cfg20_init_state_full_struct_bytes", cfg20_init_state_full_struct_bytes);
    t.case("cfg21_init_state_reinitialises_used_state", cfg21_init_state_reinitialises_used_state);
    t.case("cfg22_apply_operation_single", cfg22_apply_operation_single);
    t.case("cfg23_apply_operation_chains", cfg23_apply_operation_chains);
    t.case("cfg24_apply_operation_cross_library", cfg24_apply_operation_cross_library);
    t.case("cfg19c_checksum_value_fits_state_field", cfg19c_checksum_value_fits_state_field);
    t.case("cfg09b_dispatch_pointers_distinct", cfg09b_dispatch_pointers_distinct);
    t.finish();
}

// ===========================================================================
// rows 11-13: execute_operation
// ===========================================================================

/// row 11 — each of the four operations, resolved via the *same* library's
/// `get_operation`; return values and emitted stdout bytes both compared.
fn cfg11_execute_operation_same_library() {
    let (c, r) = both();
    for op in 0..4 {
        let name = cstring(Api::leaf_name(op));
        let inputs: Vec<(c_int, c_int)> = {
            let mut rng = Rng::new(0x1100_0000 ^ op as u64);
            (0..5_000).map(|_| (rng.next_i32_biased(), rng.next_i32_biased())).collect()
        };

        let (c_res, c_out, r_res, r_out) = serial(|| {
            let (c_res, c_out) = capture_stdout(|| {
                let f = unsafe { (c.get_operation)(op) };
                inputs
                    .iter()
                    .map(|&(a, b)| unsafe { (c.execute_operation)(f, a, b, name.as_ptr()) })
                    .collect::<Vec<c_int>>()
            });
            let (r_res, r_out) = capture_stdout(|| {
                let f = unsafe { (r.get_operation)(op) };
                inputs
                    .iter()
                    .map(|&(a, b)| unsafe { (r.execute_operation)(f, a, b, name.as_ptr()) })
                    .collect::<Vec<c_int>>()
            });
            (c_res, c_out, r_res, r_out)
        });

        for (i, (&(a, b), (cv, rv))) in
            inputs.iter().zip(c_res.iter().zip(r_res.iter())).enumerate()
        {
            assert_eq!(cv, rv, "execute_operation(op {op}, {a}, {b}) [draw {i}]: C={cv} Rust={rv}");
        }
        assert_same_output(&c_out, &r_out, &format!("execute_operation(op {op})"));
    }
}

/// row 12 — cross-library dispatch: a C function pointer driven by Rust's
/// `execute_operation` and vice versa. Both libraries export both halves, so
/// this is a configuration a real consumer can reach.
fn cfg12_execute_operation_cross_library() {
    let (c, r) = both();
    for op in 0..4 {
        let name = cstring(Api::leaf_name(op));
        let c_fn = unsafe { (c.get_operation)(op) };
        let r_fn = unsafe { (r.get_operation)(op) };
        assert!(!c_fn.is_null() && !r_fn.is_null());

        let inputs: Vec<(c_int, c_int)> = {
            let mut rng = Rng::new(0x1200_0000 ^ op as u64);
            (0..3_000).map(|_| (rng.next_i32_biased(), rng.next_i32_biased())).collect()
        };

        // Four (consumer, callee) assignments; all must agree.
        let combos: [(&Api, *const c_void, &str); 4] = [
            (c, c_fn, "C exec / C fn"),
            (c, r_fn, "C exec / Rust fn"),
            (r, c_fn, "Rust exec / C fn"),
            (r, r_fn, "Rust exec / Rust fn"),
        ];

        let results: Vec<(Vec<c_int>, Vec<u8>, &str)> = serial(|| {
            combos
                .iter()
                .map(|&(api, f, label)| {
                    let (v, out) = capture_stdout(|| {
                        inputs
                            .iter()
                            .map(|&(a, b)| unsafe {
                                (api.execute_operation)(f, a, b, name.as_ptr())
                            })
                            .collect::<Vec<c_int>>()
                    });
                    (v, out, label)
                })
                .collect()
        });

        let (base_v, base_out, base_label) = &results[0];
        for (v, out, label) in &results[1..] {
            for (i, (&(a, b), (bv, xv))) in
                inputs.iter().zip(base_v.iter().zip(v.iter())).enumerate()
            {
                assert_eq!(
                    bv, xv,
                    "op {op} ({a}, {b}) [draw {i}]: {base_label}={bv} but {label}={xv}"
                );
            }
            assert_same_output(&base_out, out, &format!("op {op}: {base_label} vs {label}"));
        }
    }
}

/// row 13 — `op_name` shapes on the success path, including a payload that
/// itself contains a printf directive (it must be treated as data).
fn cfg13_execute_operation_op_name_shapes() {
    let (c, r) = both();
    let long = "N".repeat(300);
    let names = ["XOR", "", "x", "%d %s %n %%", "with\ttab\nand newline", long.as_str()];
    for name_s in names {
        let name = cstring(name_s);
        for op in 0..4 {
            let (c_res, c_out, r_res, r_out) = serial(|| {
                let (cv, co) = capture_stdout(|| {
                    let f = unsafe { (c.get_operation)(op) };
                    unsafe { (c.execute_operation)(f, -12345, 6789, name.as_ptr()) }
                });
                let (rv, ro) = capture_stdout(|| {
                    let f = unsafe { (r.get_operation)(op) };
                    unsafe { (r.execute_operation)(f, -12345, 6789, name.as_ptr()) }
                });
                (cv, co, rv, ro)
            });
            assert_eq!(c_res, r_res, "op {op}, name {name_s:?}: C={c_res} Rust={r_res}");
            assert_same_output(&c_out, &r_out, &format!("op {op}, op_name {name_s:?}"));
        }
    }
}

// ===========================================================================
// rows 14-19: compute_checksum
// ===========================================================================

fn diff_checksum(values: &[c_int], count: c_int, ctx: &str) {
    let (c, r) = both();
    let mut cv_buf: Vec<c_int> = values.to_vec();
    let mut rv_buf: Vec<c_int> = values.to_vec();
    let cs = unsafe { (c.compute_checksum)(cv_buf.as_mut_ptr(), count) };
    let rs = unsafe { (r.compute_checksum)(rv_buf.as_mut_ptr(), count) };
    assert_eq!(
        cs, rs,
        "compute_checksum({values:?}, {count}) [{ctx}]: C=0x{cs:08X} Rust=0x{rs:08X}"
    );
    assert_eq!(cv_buf, values, "C compute_checksum mutated its input");
    assert_eq!(rv_buf, values, "Rust compute_checksum mutated its input");
}

/// rows 14-17 — count = 1, 2, 3, 4 (byte loop of 4, 8, 12, 16 bytes).
fn cfg14_17_checksum_counts_1_to_4() {
    for count in 1..=4i32 {
        let mut rng = Rng::new(0x1400_0000 ^ count as u64);
        for _ in 0..5_000 {
            let vals: Vec<c_int> = (0..count).map(|_| rng.next_i32_biased()).collect();
            diff_checksum(&vals, count, &format!("count={count}"));
        }
        // Same buffer, but longer than `count`: only the first `count` ints may
        // be read, so the trailing garbage must not affect the result.
        let mut rng = Rng::new(0x1450_0000 ^ count as u64);
        for _ in 0..2_000 {
            let vals: Vec<c_int> = (0..8).map(|_| rng.next_i32_biased()).collect();
            diff_checksum(&vals, count, &format!("count={count}, oversized buffer"));
        }
    }
}

/// row 18 — the `count > 4` clamp. Result must equal the `count == 4` result.
fn cfg18_checksum_count_clamped_above_four() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1800_0018);
    for _ in 0..2_000 {
        let vals: Vec<c_int> = (0..4).map(|_| rng.next_i32_biased()).collect();
        let mut four = vals.clone();
        let base = unsafe { (c.compute_checksum)(four.as_mut_ptr(), 4) };
        for &count in &[5i32, 6, 16, 1_000, i32::MAX, i32::MAX - 1] {
            // NB: the buffer is only 4 ints long, which is exactly the point —
            // the C code clamps `copy_count` to 4 and never reads past it.
            let mut cb = vals.clone();
            let mut rb = vals.clone();
            let cs = unsafe { (c.compute_checksum)(cb.as_mut_ptr(), count) };
            let rs = unsafe { (r.compute_checksum)(rb.as_mut_ptr(), count) };
            assert_eq!(cs, rs, "compute_checksum({vals:?}, {count}): C=0x{cs:04X} Rust=0x{rs:04X}");
            assert_eq!(cs, base, "C clamp broken: count={count} != count=4");
            assert_eq!(rs, base, "Rust clamp broken: count={count} != count=4");
        }
    }
}

/// row 19 — byte-order-sensitive patterns. `compute_checksum` reinterprets the
/// `int` array as bytes, so host endianness is observable and must match.
fn cfg19_checksum_byte_order_patterns() {
    let mut patterns: Vec<Vec<c_int>> = vec![
        vec![0, 0, 0, 0],
        vec![-1, -1, -1, -1],
        vec![0x0102_0304, 0x0506_0708, 0x090A_0B0C, 0x0D0E_0F10],
        vec![0x0000_00FF, 0x0000_FF00, 0x00FF_0000, 0xFF00_0000_u32 as c_int],
        vec![i32::MIN, i32::MAX, 0, -1],
        vec![
            0xDEAD_BEEF_u32 as c_int,
            0x0000_FFFF,
            0xABCD,
            0xDEAD_BEEF_u32 as c_int,
        ],
    ];
    // 1-hot in each of the 16 bytes, and 1-cold in each of the 16 bytes.
    for byte in 0..16usize {
        for &v in &[0x01u8, 0x80, 0xFF] {
            let mut raw = [0u8; 16];
            raw[byte] = v;
            patterns.push(
                raw.chunks(4).map(|c| i32::from_ne_bytes(c.try_into().unwrap())).collect(),
            );
            let mut raw = [0xFFu8; 16];
            raw[byte] = !v;
            patterns.push(
                raw.chunks(4).map(|c| i32::from_ne_bytes(c.try_into().unwrap())).collect(),
            );
        }
    }
    for p in &patterns {
        for count in 1..=4i32 {
            diff_checksum(p, count, "byte-order pattern");
        }
        for &count in &[5i32, 9, i32::MAX] {
            diff_checksum(p, count, "byte-order pattern, clamped");
        }
    }
}

/// The `MASK_LOWER` bound holds for every input on both sides (ERRORS.md row 13
/// is the error-side mirror of this).
fn cfg19b_checksum_always_masked_to_16_bits() {
    let (c, r) = both();
    let mut rng = Rng::new(0x19B0_0019);
    for _ in 0..5_000 {
        let mut cb: Vec<c_int> = (0..4).map(|_| rng.next_i32()).collect();
        let mut rb = cb.clone();
        let count = (rng.next_u32() % 9) as c_int;
        let cs = unsafe { (c.compute_checksum)(cb.as_mut_ptr(), count) };
        let rs = unsafe { (r.compute_checksum)(rb.as_mut_ptr(), count) };
        assert_eq!(cs, rs);
        assert!(cs <= 0xFFFF, "C returned 0x{cs:08X}, above MASK_LOWER");
        assert!(rs <= 0xFFFF, "Rust returned 0x{rs:08X}, above MASK_LOWER");
    }
}

// ===========================================================================
// rows 20-21: init_state
// ===========================================================================

/// row 20 — all 12 struct bytes compared, starting from a poisoned buffer.
fn cfg20_init_state_full_struct_bytes() {
    let (c, r) = both();
    let mut vals: Vec<c_int> = INTERESTING.to_vec();
    let mut rng = Rng::new(0x2000_0020);
    vals.extend((0..3_000).map(|_| rng.next_i32_biased()));

    let (cs, c_out, rs, r_out) = serial(|| {
        let (cs, c_out) = capture_stdout(|| {
            vals.iter()
                .map(|&v| {
                    let mut s = StateBuf::poisoned();
                    unsafe { (c.init_state)(s.as_mut_ptr(), v) };
                    s
                })
                .collect::<Vec<StateBuf>>()
        });
        let (rs, r_out) = capture_stdout(|| {
            vals.iter()
                .map(|&v| {
                    let mut s = StateBuf::poisoned();
                    unsafe { (r.init_state)(s.as_mut_ptr(), v) };
                    s
                })
                .collect::<Vec<StateBuf>>()
        });
        (cs, c_out, rs, r_out)
    });

    for (i, (&v, (a, b))) in vals.iter().zip(cs.iter().zip(rs.iter())).enumerate() {
        assert_eq!(a, b, "init_state(_, {v}) [draw {i}]:\n C: {a:?}\n R: {b:?}");
        assert_eq!(a.accumulator(), v);
        assert_eq!(a.operation_count(), 0);
        assert_eq!(a.checksum(), 0);
    }
    assert_same_output(&c_out, &r_out, "init_state");
}

/// row 21 — re-initialising a state that already carries a used accumulator,
/// operation_count and checksum: all three fields must be reset.
fn cfg21_init_state_reinitialises_used_state() {
    let (c, r) = both();
    let mut rng = Rng::new(0x2100_0021);
    serial(|| {
        for _ in 0..1_000 {
            let seed = rng.next_i32_biased();
            let v2 = rng.next_i32_biased();
            let op = (rng.next_u32() % 4) as c_int;

            let mk = |api: &Api| {
                let mut s = StateBuf::poisoned();
                unsafe {
                    (api.init_state)(s.as_mut_ptr(), seed);
                    let f = (api.get_operation)(op);
                    (api.apply_operation)(s.as_mut_ptr(), v2, f);
                    (api.apply_operation)(s.as_mut_ptr(), v2, f);
                    // reinitialise on top of the used state
                    (api.init_state)(s.as_mut_ptr(), v2);
                }
                s
            };
            let (cs, _) = capture_stdout(|| mk(c));
            let (rs, _) = capture_stdout(|| mk(r));
            assert_eq!(cs, rs, "re-init with seed={seed} v2={v2} op={op}");
            assert_eq!(cs.operation_count(), 0, "operation_count not reset");
            assert_eq!(cs.checksum(), 0, "checksum not reset");
        }
    });
}

// ===========================================================================
// rows 22-24: apply_operation
// ===========================================================================

/// row 22 — one application of each op onto a fresh state; full struct bytes.
fn cfg22_apply_operation_single() {
    let (c, r) = both();
    for op in 0..4i32 {
        let mut rng = Rng::new(0x2200_0000 ^ op as u64);
        serial(|| {
            let cases: Vec<(c_int, c_int)> =
                (0..5_000).map(|_| (rng.next_i32_biased(), rng.next_i32_biased())).collect();
            let run = |api: &Api| {
                cases
                    .iter()
                    .map(|&(initial, value)| {
                        let mut s = StateBuf::poisoned();
                        unsafe {
                            (api.init_state)(s.as_mut_ptr(), initial);
                            let f = (api.get_operation)(op);
                            (api.apply_operation)(s.as_mut_ptr(), value, f);
                        }
                        s
                    })
                    .collect::<Vec<StateBuf>>()
            };
            let (cs, _) = capture_stdout(|| run(c));
            let (rs, _) = capture_stdout(|| run(r));
            for (i, (&(initial, value), (a, b))) in
                cases.iter().zip(cs.iter().zip(rs.iter())).enumerate()
            {
                assert_eq!(
                    a, b,
                    "apply_operation(op {op}, initial={initial}, value={value}) [draw {i}]:\n \
                     C: {a:?}\n R: {b:?}"
                );
                assert_eq!(a.operation_count(), 1);
            }
        });
    }
}

/// row 23 — chains of n applications with a mixed op sequence; the accumulator
/// and operation_count are compared after *every* step.
fn cfg23_apply_operation_chains() {
    let (c, r) = both();
    for &n in &[0usize, 1, 2, 3, 5, 17, 50] {
        let mut rng = Rng::new(0x2300_0000 ^ n as u64);
        serial(|| {
            for _ in 0..300 {
                let initial = rng.next_i32_biased();
                let steps: Vec<(c_int, c_int)> = (0..n)
                    .map(|_| ((rng.next_u32() % 4) as c_int, rng.next_i32_biased()))
                    .collect();
                let run = |api: &Api| {
                    let mut s = StateBuf::poisoned();
                    let mut trace = Vec::with_capacity(n + 1);
                    unsafe { (api.init_state)(s.as_mut_ptr(), initial) };
                    trace.push(s);
                    for &(op, value) in &steps {
                        let f = unsafe { (api.get_operation)(op) };
                        unsafe { (api.apply_operation)(s.as_mut_ptr(), value, f) };
                        trace.push(s);
                    }
                    trace
                };
                let (ct, _) = capture_stdout(|| run(c));
                let (rt, _) = capture_stdout(|| run(r));
                for (step, (a, b)) in ct.iter().zip(rt.iter()).enumerate() {
                    assert_eq!(
                        a, b,
                        "chain n={n} initial={initial} steps={steps:?} diverged at step {step}:\n \
                         C: {a:?}\n R: {b:?}"
                    );
                }
                assert_eq!(ct.last().unwrap().operation_count(), n as c_int);
            }
        });
    }
}

/// row 24 — cross-library: a leaf pointer from one library applied by the other,
/// and one `ComputeState` buffer driven alternately by both libraries.
fn cfg24_apply_operation_cross_library() {
    let (c, r) = both();
    let mut rng = Rng::new(0x2400_0024);
    serial(|| {
        for _ in 0..2_000 {
            let initial = rng.next_i32_biased();
            let steps: Vec<(c_int, c_int, bool, bool)> = (0..12)
                .map(|_| {
                    (
                        (rng.next_u32() % 4) as c_int,
                        rng.next_i32_biased(),
                        rng.next_u32() & 1 == 1, // which library's apply_operation
                        rng.next_u32() & 1 == 1, // which library's function pointer
                    )
                })
                .collect();

            // Reference: everything done by one library, then everything by the
            // other, then the mixed interleaving. All three must agree.
            let pure = |api: &Api| {
                let mut s = StateBuf::poisoned();
                unsafe { (api.init_state)(s.as_mut_ptr(), initial) };
                for &(op, value, _, _) in &steps {
                    let f = unsafe { (api.get_operation)(op) };
                    unsafe { (api.apply_operation)(s.as_mut_ptr(), value, f) };
                }
                s
            };
            let mixed = || {
                let mut s = StateBuf::poisoned();
                unsafe { (c.init_state)(s.as_mut_ptr(), initial) };
                for &(op, value, use_r_apply, use_r_fn) in &steps {
                    let src = if use_r_fn { r } else { c };
                    let dst = if use_r_apply { r } else { c };
                    let f = unsafe { (src.get_operation)(op) };
                    unsafe { (dst.apply_operation)(s.as_mut_ptr(), value, f) };
                }
                s
            };
            let (cs, _) = capture_stdout(|| pure(c));
            let (rs, _) = capture_stdout(|| pure(r));
            let (ms, _) = capture_stdout(mixed);
            assert_eq!(cs, rs, "pure C vs pure Rust: initial={initial} steps={steps:?}");
            assert_eq!(cs, ms, "pure C vs mixed C/Rust: initial={initial} steps={steps:?}");
        }
    });
}

/// Sanity: `compute_checksum` output feeding `ComputeState::checksum` (the field
/// is `unsigned int`, so the 16-bit-masked value round-trips unchanged).
fn cfg19c_checksum_value_fits_state_field() {
    let (c, r) = both();
    let mut rng = Rng::new(0x19C0_0019);
    for _ in 0..1_000 {
        let mut cb: Vec<c_int> = (0..4).map(|_| rng.next_i32_biased()).collect();
        let mut rb = cb.clone();
        let cs: c_uint = unsafe { (c.compute_checksum)(cb.as_mut_ptr(), 4) };
        let rs: c_uint = unsafe { (r.compute_checksum)(rb.as_mut_ptr(), 4) };
        assert_eq!(cs, rs);
        assert_eq!(cs as c_int as c_uint, cs);
    }
}

/// The function-pointer values themselves must be *distinct* per opcode in both
/// libraries (a mis-wired dispatch table would otherwise be invisible when two
/// ops happen to agree on a value).
fn cfg09b_dispatch_pointers_distinct() {
    let (c, r) = both();
    for api in [c, r] {
        let ptrs: Vec<*const c_void> = (0..4).map(|op| unsafe { (api.get_operation)(op) }).collect();
        for i in 0..4 {
            for j in (i + 1)..4 {
                assert_ne!(
                    ptrs[i], ptrs[j],
                    "{} get_operation({i}) and get_operation({j}) return the same pointer",
                    api.name
                );
            }
        }
    }
}
