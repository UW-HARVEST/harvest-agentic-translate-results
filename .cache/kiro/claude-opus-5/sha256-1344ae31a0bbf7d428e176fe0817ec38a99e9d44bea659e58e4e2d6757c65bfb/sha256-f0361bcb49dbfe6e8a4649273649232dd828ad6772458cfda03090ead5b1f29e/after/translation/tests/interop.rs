//! Cross-library interop: a `ProcessState` allocated by ONE implementation is
//! driven by the OTHER implementation's functions.
//!
//! This is the strongest available check that the two `.so`s agree on
//! `sizeof(ProcessState)`, on every field offset, and on the bit-field layout
//! inside `PackedFlags` — a mismatch in any of those would show up here as
//! wrong values, wrong output, or a crash, whereas a same-library test would
//! silently agree with itself.

mod common;
use common::*;

use std::ffi::c_char;

/// Every ordered pair of (allocating impl, operating impl).
fn combos(p: &Pair) -> Vec<(&'static str, &Impl, &Impl)> {
    vec![
        ("C alloc / C ops", &p.c, &p.c),
        ("C alloc / Rust ops", &p.c, &p.r),
        ("Rust alloc / C ops", &p.r, &p.c),
        ("Rust alloc / Rust ops", &p.r, &p.r),
    ]
}

#[test]
fn cross_library_pipeline_agrees() {
    let p = pair();
    let mut rng = Rng::new(0xE0001);

    for _ in 0..300 {
        let p1 = rng.next_i32();
        let p2 = rng.next_i32();
        let p3 = rng.next_i32();
        let p4 = rng.next_i32();
        let capacity = if rng.next_u32() % 4 == 0 { rng.range_i32(1, 24) } else { 128 };
        let nupd = (rng.next_u32() % 4) as usize + 1;

        let mut results: Vec<(&str, Vec<i64>, Option<Vec<u8>>, u32, u32, i32)> = Vec::new();
        let mut outs: Vec<(&str, Vec<u8>)> = Vec::new();

        for (label, alloc, ops) in combos(p) {
            let mut rets: Vec<i64> = Vec::new();
            let (st, out) = capture(|| unsafe {
                let st = (alloc.create_state)(p1, capacity);
                if st.is_null() {
                    return st;
                }
                for _ in 0..nupd {
                    (ops.update_flags)(st, p2);
                    rets.push((*st).flags as i64);
                }
                let ch = (b'0' as i32).wrapping_add(p3 % 10) as c_char;
                rets.push((ops.process_buffer)(st, ch) as i64);
                rets.push((ops.confuse_types)(st, p4 % 4) as i64);
                st
            });
            assert!(!st.is_null(), "[{label}] create_state({p1},{capacity}) returned NULL");
            let s = unsafe { *st };
            let buf = unsafe { alloc.buffer_bytes(st) };
            results.push((label, rets, buf, s.flags, s.data, s.capacity));
            outs.push((label, out));
            // Free with the SAME implementation that allocated: only that one
            // owns the allocation.
            let _ = capture(|| unsafe { (alloc.destroy_state)(st) });
        }

        // All four combinations must agree with the C/C baseline.
        let (base_label, base_rets, base_buf, base_flags, base_data, base_cap) = &results[0];
        let (_, base_out) = &outs[0];
        for (i, (label, rets, buf, flags, data, cap)) in results.iter().enumerate().skip(1) {
            assert_eq!(
                rets, base_rets,
                "[{label}] vs [{base_label}] return sequence differs for \
                 ({p1},{p2},{p3},{p4},cap={capacity})"
            );
            assert_eq!(buf, base_buf, "[{label}] vs [{base_label}] buffer differs");
            assert_eq!(
                decode_flags(*flags),
                decode_flags(*base_flags),
                "[{label}] vs [{base_label}] flags differ ({flags:#010x} vs {base_flags:#010x})"
            );
            assert_eq!(data, base_data, "[{label}] vs [{base_label}] data word differs");
            assert_eq!(cap, base_cap, "[{label}] vs [{base_label}] capacity differs");
            assert_eq!(
                &outs[i].1,
                base_out,
                "[{label}] vs [{base_label}] stdout differs:\n  {} = {}\n  {} = {}",
                label,
                show(&outs[i].1),
                base_label,
                show(base_out)
            );
        }
    }
}

#[test]
fn cross_library_struct_layout_is_identical() {
    // Write through one implementation's `create_state`, then have the other
    // implementation report each field back via the values it prints and
    // returns. Any offset or bit-field-position disagreement changes these.
    let p = pair();
    for (label, alloc, ops) in combos(p) {
        for &(initial, capacity) in &[(0i32, 128i32), (-1, 128), (i32::MIN, 128), (i32::MAX, 16)] {
            let (probe, out) = capture(|| unsafe {
                let st = (alloc.create_state)(initial, capacity);
                assert!(!st.is_null());
                // `confuse_types(_, 2)` returns `data.uint_val & 0xFF`, which
                // pins down the `data` offset and endianness.
                let low = (ops.confuse_types)(st, 2) as i64;
                // `update_flags(_, 0)` sets counter=1, mode=0, flags1..3=0,
                // pinning down the bit-field positions; `status`/`reserved`
                // must survive untouched (status=15, reserved=0).
                (ops.update_flags)(st, 0);
                let flags = (*st).flags;
                let cap = (*st).capacity;
                (alloc.destroy_state)(st);
                (low, flags, cap)
            });
            let (low, flags, cap) = probe;
            let f = decode_flags(flags);
            assert_eq!(low, (initial as u32 & 0xFF) as i64, "[{label}] data offset/endianness");
            assert_eq!(cap, capacity, "[{label}] capacity offset");
            assert_eq!(f.counter, 1, "[{label}] counter bit position");
            assert_eq!(f.mode, 0, "[{label}] mode bit position");
            assert_eq!(f.flag1, 0, "[{label}] flag1 bit position");
            assert_eq!(f.flag2, 0, "[{label}] flag2 bit position");
            assert_eq!(f.flag3, 0, "[{label}] flag3 bit position");
            assert_eq!(f.status, 15, "[{label}] status clobbered / wrong offset");
            assert_eq!(f.reserved, 0, "[{label}] reserved clobbered / wrong offset");
            assert!(!out.is_empty(), "[{label}] no output captured");
        }
    }
}
