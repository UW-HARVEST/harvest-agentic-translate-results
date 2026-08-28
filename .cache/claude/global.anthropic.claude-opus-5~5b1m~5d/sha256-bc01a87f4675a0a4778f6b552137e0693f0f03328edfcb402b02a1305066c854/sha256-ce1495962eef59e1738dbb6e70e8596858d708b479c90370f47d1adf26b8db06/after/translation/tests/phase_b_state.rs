// Phase B — CONFIGS.md rows C15..C20
//
// `ComputeState` lifecycle through the low-level API: `init_state` and
// `apply_operation`. The state struct is embedded in a larger buffer with a
// poison guard region so that a translation writing the wrong number of bytes
// (padding / struct-layout divergence) is detected, not just a wrong field.

mod common;
use common::*;

const S: u64 = 0x57A7_E000_1234_5678;

/// Compare the raw bytes of two state buffers (state + guard region).
#[track_caller]
fn assert_buf_eq(ctx: &str, cb: &StateBuf, rb: &StateBuf) {
    assert!(
        cb.guard_intact(),
        "{ctx}: C wrote past sizeof(ComputeState); guard = {:02X?}",
        cb.guard()
    );
    assert!(
        rb.guard_intact(),
        "{ctx}: Rust wrote past sizeof(ComputeState); guard = {:02X?}",
        rb.guard()
    );
    assert_eq!(
        cb.bytes(),
        rb.bytes(),
        "{ctx}: raw state bytes differ\n  C    = {:02X?}\n  Rust = {:02X?}",
        cb.bytes(),
        rb.bytes()
    );
    assert_eq!(
        cb.state(),
        rb.state(),
        "{ctx}: ComputeState fields differ"
    );
}

// ---------------------------------------------------------------------------
// Struct layout must agree between the two ABIs.
// ---------------------------------------------------------------------------

#[test]
fn compute_state_layout() {
    // int, int, unsigned int on every target the C builds for.
    assert_eq!(STATE_SIZE, 12, "sizeof(ComputeState)");
    assert_eq!(std::mem::align_of::<ComputeState>(), 4, "alignof(ComputeState)");
}

// ---------------------------------------------------------------------------
// C15 — init_state over boundary + random initial values
// ---------------------------------------------------------------------------

#[test]
fn c15_init_state_values() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 0xC15);

    let mut values: Vec<i32> = INTERESTING.to_vec();
    for _ in 0..500 {
        values.push(rng.interesting_i32());
    }

    for (i, &v) in values.iter().enumerate() {
        let mut cb = StateBuf::new();
        let mut rb = StateBuf::new();

        let (_, co) = capture(|| unsafe { (c.init_state)(cb.as_ptr(), v) });
        let (_, ro) = capture(|| unsafe { (r.init_state)(rb.as_ptr(), v) });

        assert_stdout_eq(&format!("C15 init_state({v}) iter {i}"), &co, &ro);
        assert_buf_eq(&format!("C15 init_state({v}) iter {i}"), &cb, &rb);

        // The C template is {initial_value, 0, 0x0000}.
        let st = cb.state();
        assert_eq!(st.accumulator, v, "C15 accumulator");
        assert_eq!(st.operation_count, 0, "C15 operation_count");
        assert_eq!(st.checksum, 0, "C15 checksum");
    }
}

// ---------------------------------------------------------------------------
// C16 — re-init over a dirty state: must reset operation_count and checksum
// ---------------------------------------------------------------------------

#[test]
fn c16_init_state_over_dirty_state() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 0xC16);

    for i in 0..300 {
        let dirty = ComputeState {
            accumulator: rng.interesting_i32(),
            operation_count: rng.i32(),
            checksum: rng.next_u32(),
        };
        let v = rng.interesting_i32();

        let mut cb = StateBuf::new();
        let mut rb = StateBuf::new();
        cb.set_state(dirty);
        rb.set_state(dirty);

        let (_, co) = capture(|| unsafe { (c.init_state)(cb.as_ptr(), v) });
        let (_, ro) = capture(|| unsafe { (r.init_state)(rb.as_ptr(), v) });

        assert_stdout_eq(&format!("C16 re-init iter {i}"), &co, &ro);
        assert_buf_eq(&format!("C16 re-init iter {i} (dirty={dirty:?}, v={v})"), &cb, &rb);
        assert_eq!(cb.state().operation_count, 0, "C16 operation_count reset");
        assert_eq!(cb.state().checksum, 0, "C16 checksum reset");
    }
}

// ---------------------------------------------------------------------------
// C17 — apply_operation, each opcode, on a freshly initialised state
// ---------------------------------------------------------------------------

#[test]
fn c17_apply_operation_each_opcode() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 0xC17);

    for opcode in 0..4i32 {
        let cf = unsafe { (c.get_operation)(opcode) };
        let rf = unsafe { (r.get_operation)(opcode) };

        for i in 0..300 {
            let seed_acc = rng.interesting_i32();
            let value = rng.interesting_i32();

            let mut cb = StateBuf::new();
            let mut rb = StateBuf::new();
            // Set up state through the real API, then discard its output.
            let _ = capture(|| unsafe { (c.init_state)(cb.as_ptr(), seed_acc) });
            let _ = capture(|| unsafe { (r.init_state)(rb.as_ptr(), seed_acc) });
            assert_buf_eq("C17 setup", &cb, &rb);

            let (_, co) = capture(|| unsafe { (c.apply_operation)(cb.as_ptr(), value, cf) });
            let (_, ro) = capture(|| unsafe { (r.apply_operation)(rb.as_ptr(), value, rf) });

            let ctx = format!("C17 apply_operation(op={opcode}) iter {i} acc={seed_acc} val={value}");
            assert_stdout_eq(&ctx, &co, &ro);
            assert_buf_eq(&ctx, &cb, &rb);
            // apply_operation prints nothing on the success path.
            assert!(co.is_empty(), "{ctx}: C printed {:?}", show(&co));
            assert_eq!(cb.state().operation_count, 1, "{ctx}: operation_count");
        }
    }
}

// ---------------------------------------------------------------------------
// C18 — apply_operation with cross-library function pointers (axis A7)
// ---------------------------------------------------------------------------

#[test]
fn c18_apply_operation_cross_library_function_pointers() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 0xC18);

    for opcode in 0..4i32 {
        let cf = unsafe { (c.get_operation)(opcode) };
        let rf = unsafe { (r.get_operation)(opcode) };

        for i in 0..150 {
            let seed_acc = rng.interesting_i32();
            let value = rng.interesting_i32();

            // Four buffers: {C impl, Rust impl} x {C fnptr, Rust fnptr}
            let mut b_cc = StateBuf::new();
            let mut b_cr = StateBuf::new();
            let mut b_rc = StateBuf::new();
            let mut b_rr = StateBuf::new();
            for b in [&mut b_cc, &mut b_cr, &mut b_rc, &mut b_rr] {
                let _ = capture(|| unsafe { (c.init_state)(b.as_ptr(), seed_acc) });
            }

            let _ = capture(|| unsafe { (c.apply_operation)(b_cc.as_ptr(), value, cf) });
            let _ = capture(|| unsafe { (c.apply_operation)(b_cr.as_ptr(), value, rf) });
            let _ = capture(|| unsafe { (r.apply_operation)(b_rc.as_ptr(), value, cf) });
            let _ = capture(|| unsafe { (r.apply_operation)(b_rr.as_ptr(), value, rf) });

            let ctx = format!("C18 op={opcode} iter {i} acc={seed_acc} val={value}");
            assert_buf_eq(&format!("{ctx} [C impl: C fnptr vs Rust fnptr]"), &b_cc, &b_cr);
            assert_buf_eq(&format!("{ctx} [Rust impl: C fnptr vs Rust fnptr]"), &b_rc, &b_rr);
            assert_buf_eq(&format!("{ctx} [C impl vs Rust impl]"), &b_cc, &b_rr);
            assert_buf_eq(&format!("{ctx} [C impl/Rust ptr vs Rust impl/C ptr]"), &b_cr, &b_rc);
        }
    }
}

// ---------------------------------------------------------------------------
// C19 — long chained sequence on ONE state (axis A5)
// ---------------------------------------------------------------------------

#[test]
fn c19_apply_operation_chained_sequence() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 0xC19);

    for round in 0..200 {
        let seed_acc = rng.interesting_i32();
        let mut cb = StateBuf::new();
        let mut rb = StateBuf::new();
        let (_, co0) = capture(|| unsafe { (c.init_state)(cb.as_ptr(), seed_acc) });
        let (_, ro0) = capture(|| unsafe { (r.init_state)(rb.as_ptr(), seed_acc) });
        assert_stdout_eq(&format!("C19 round {round} init"), &co0, &ro0);
        assert_buf_eq(&format!("C19 round {round} init"), &cb, &rb);

        let steps = 25;
        for step in 0..steps {
            let opcode = (rng.below(4)) as i32;
            let value = rng.interesting_i32();
            let cf = unsafe { (c.get_operation)(opcode) };
            let rf = unsafe { (r.get_operation)(opcode) };

            let (_, co) = capture(|| unsafe { (c.apply_operation)(cb.as_ptr(), value, cf) });
            let (_, ro) = capture(|| unsafe { (r.apply_operation)(rb.as_ptr(), value, rf) });

            let ctx = format!(
                "C19 round {round} step {step} (op={opcode}, value={value}, seed_acc={seed_acc})"
            );
            assert_stdout_eq(&ctx, &co, &ro);
            assert_buf_eq(&ctx, &cb, &rb);
            assert_eq!(
                cb.state().operation_count,
                step + 1,
                "{ctx}: operation_count must accumulate"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C20 — overflow-heavy chain: INT_MIN/INT_MAX seeds, multiply-dominated
// ---------------------------------------------------------------------------

#[test]
fn c20_apply_operation_overflow_heavy_chain() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 0xC20);

    for &seed_acc in &[i32::MIN, i32::MAX, -1, 1, 0, 0x4000_0000, i32::MIN + 1, i32::MAX - 1] {
        let mut cb = StateBuf::new();
        let mut rb = StateBuf::new();
        let _ = capture(|| unsafe { (c.init_state)(cb.as_ptr(), seed_acc) });
        let _ = capture(|| unsafe { (r.init_state)(rb.as_ptr(), seed_acc) });

        for step in 0..60 {
            // Multiply (0) and shift (3) dominate to force repeated overflow.
            let opcode = match rng.below(6) {
                0 => 1i32,
                1 => 2,
                2 | 3 => 0,
                _ => 3,
            };
            let value = *INTERESTING
                .get((rng.next_u32() as usize) % INTERESTING.len())
                .unwrap();
            let cf = unsafe { (c.get_operation)(opcode) };
            let rf = unsafe { (r.get_operation)(opcode) };

            let (_, co) = capture(|| unsafe { (c.apply_operation)(cb.as_ptr(), value, cf) });
            let (_, ro) = capture(|| unsafe { (r.apply_operation)(rb.as_ptr(), value, rf) });

            let ctx = format!("C20 seed={seed_acc} step {step} op={opcode} value={value}");
            assert_stdout_eq(&ctx, &co, &ro);
            assert_buf_eq(&ctx, &cb, &rb);
        }
    }
}
