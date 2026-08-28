// Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//
// Every call goes through `dlsym` on the two shared objects. For each scenario
// the harness compares, byte for byte:
//   * every return value,
//   * every observable field of every `ProcessState` (flags word incl. the
//     `status`/`reserved` bits, union word, capacity, buffer contents),
//   * the complete stdout produced by the library.

mod common;

use common::*;
use std::ffi::c_char;

// ===========================================================================
// C1 — create_state(capacity = 128) over random initial_val
// ===========================================================================
#[test]
fn c1_create_state_default_capacity_random_initial_val() {
    let mut rng = Rng::new(0xC1);
    for i in 0..600 {
        let init = rng.interesting_i32();
        diff(&format!("C1 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            log.push(format!("create_state -> null={}", s.is_null()));
            log_state(log, "state", s);
            if !s.is_null() {
                // create_state always produces this exact bit-field word.
                let snap = snapshot(s);
                log.push(format!("flags_is_7b05={}", snap.flags == 0x0000_7B05));
                (lib.destroy_state)(s);
            }
        });
    }
}

// ===========================================================================
// C2 — create_state boundary initial_val
// ===========================================================================
#[test]
fn c2_create_state_boundary_initial_val() {
    let mut vals: Vec<i32> = BOUNDARY_I32.to_vec();
    vals.extend_from_slice(&FLOAT_BITS);
    vals.push(1078530011);
    for init in vals {
        diff(&format!("C2 init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            log_state(log, "state", s);
            if !s.is_null() {
                (lib.destroy_state)(s);
            }
        });
    }
}

// ===========================================================================
// C3 — small capacities: snprintf truncation / exact fit / slack
// ===========================================================================
#[test]
fn c3_create_state_small_capacities() {
    let mut rng = Rng::new(0xC3);
    for cap in 1..=20i32 {
        for i in 0..30 {
            let init = rng.interesting_i32();
            diff(
                &format!("C3 cap={cap} #{i} init={init}"),
                &move |lib, log| unsafe {
                    let s = (lib.create_state)(init, cap);
                    log_state(log, "state", s);
                    if !s.is_null() {
                        log.push(format!("pb0 -> {}", (lib.process_buffer)(s, b'0' as c_char)));
                        log.push(format!("pbS -> {}", (lib.process_buffer)(s, b'S' as c_char)));
                        log.push(format!("pbC -> {}", (lib.process_buffer)(s, b':' as c_char)));
                        (lib.destroy_state)(s);
                    }
                },
            );
        }
    }
}

// ===========================================================================
// C4 — large-but-servable capacities
// ===========================================================================
#[test]
fn c4_create_state_large_capacities() {
    let caps: [i32; 8] = [21, 32, 64, 128, 256, 4096, 65536, 1 << 20];
    let mut rng = Rng::new(0xC4);
    for cap in caps {
        for i in 0..20 {
            let init = rng.interesting_i32();
            diff(
                &format!("C4 cap={cap} #{i} init={init}"),
                &move |lib, log| unsafe {
                    let s = (lib.create_state)(init, cap);
                    log_state(log, "state", s);
                    if !s.is_null() {
                        (lib.destroy_state)(s);
                    }
                },
            );
        }
    }
}

// ===========================================================================
// C5 — create/destroy round-trips
// ===========================================================================
#[test]
fn c5_create_destroy_roundtrips() {
    let mut rng = Rng::new(0xC5);
    for round in 0..40 {
        let seed = rng.next_u64();
        diff(&format!("C5 round={round}"), &move |lib, log| unsafe {
            let mut r = Rng::new(seed);
            for _ in 0..25 {
                let init = r.interesting_i32();
                let cap = 1 + (r.below(200) as i32);
                let s = (lib.create_state)(init, cap);
                log_state(log, "s", s);
                (lib.destroy_state)(s);
            }
        });
    }
}

// ===========================================================================
// C6 — process_buffer with each digit target
// ===========================================================================
#[test]
fn c6_process_buffer_digit_targets() {
    let mut rng = Rng::new(0xC6);
    for i in 0..200 {
        let init = rng.interesting_i32();
        diff(&format!("C6 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            log_state(log, "state", s);
            for d in b'0'..=b'9' {
                log.push(format!(
                    "target='{}' -> {}",
                    d as char,
                    (lib.process_buffer)(s, d as c_char)
                ));
            }
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C7 — process_buffer with each literal char of "State:Mode:"
// ===========================================================================
#[test]
fn c7_process_buffer_literal_chars() {
    let mut rng = Rng::new(0xC7);
    for i in 0..200 {
        let init = rng.interesting_i32();
        diff(&format!("C7 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            for &ch in b"State:Mode:" {
                log.push(format!(
                    "target='{}' -> {}",
                    ch as char,
                    (lib.process_buffer)(s, ch as c_char)
                ));
            }
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C8 — process_buffer with absent targets
// ===========================================================================
#[test]
fn c8_process_buffer_absent_targets() {
    let mut rng = Rng::new(0xC8);
    for i in 0..100 {
        let init = rng.interesting_i32();
        diff(&format!("C8 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            for &ch in b"z~ \t!@#QW" {
                log.push(format!(
                    "target={ch} -> {}",
                    (lib.process_buffer)(s, ch as c_char)
                ));
            }
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C9 — match at position 0 and at len-1
// ===========================================================================
#[test]
fn c9_process_buffer_first_and_last_byte() {
    let mut rng = Rng::new(0xC9);
    for i in 0..200 {
        let init = rng.interesting_i32();
        diff(&format!("C9 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            let snap = snapshot(s);
            let buf = snap.buffer.clone().unwrap_or_default();
            let first = *buf.first().unwrap_or(&b'S');
            let last = *buf.last().unwrap_or(&b'S');
            log.push(format!("first={first} last={last}"));
            log.push(format!(
                "pb(first) -> {}",
                (lib.process_buffer)(s, first as c_char)
            ));
            log.push(format!(
                "pb(last) -> {}",
                (lib.process_buffer)(s, last as c_char)
            ));
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C10 — random target across the FULL signed char range
// ===========================================================================
#[test]
fn c10_process_buffer_full_char_range() {
    let mut rng = Rng::new(0xCA);
    // exhaustive over the char domain, random initial_val each time
    for t in -128i32..=127 {
        let init = rng.interesting_i32();
        diff(
            &format!("C10 target={t} init={init}"),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(init, 128);
                log.push(format!("pb -> {}", (lib.process_buffer)(s, t as c_char)));
                (lib.destroy_state)(s);
            },
        );
    }
    // plus randomized pairs
    for i in 0..300 {
        let init = rng.interesting_i32();
        let t = (rng.next_u32() & 0xFF) as u8 as i8;
        diff(
            &format!("C10r #{i} target={t} init={init}"),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(init, 128);
                log.push(format!("pb -> {}", (lib.process_buffer)(s, t as c_char)));
                (lib.destroy_state)(s);
            },
        );
    }
}

// ===========================================================================
// C11 — truncated buffers x every digit target
// ===========================================================================
#[test]
fn c11_process_buffer_truncated_buffers() {
    let mut rng = Rng::new(0xCB);
    for cap in 1..=20i32 {
        for i in 0..10 {
            let init = rng.interesting_i32();
            diff(
                &format!("C11 cap={cap} #{i} init={init}"),
                &move |lib, log| unsafe {
                    let s = (lib.create_state)(init, cap);
                    log_state(log, "state", s);
                    for d in b'0'..=b'9' {
                        log.push(format!("t{} -> {}", d as char, (lib.process_buffer)(s, d as c_char)));
                    }
                    for &ch in b"State:Mode" {
                        log.push(format!("t{} -> {}", ch as char, (lib.process_buffer)(s, ch as c_char)));
                    }
                    (lib.destroy_state)(s);
                },
            );
        }
    }
}

// ===========================================================================
// C12 — process_buffer is idempotent (does not mutate the state)
// ===========================================================================
#[test]
fn c12_process_buffer_repeated_same_state() {
    let mut rng = Rng::new(0xCC);
    for i in 0..150 {
        let init = rng.interesting_i32();
        let t = *rng.pick(b"0123456789:SM");
        diff(
            &format!("C12 #{i} init={init} t={t}"),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(init, 128);
                for k in 0..6 {
                    log.push(format!("k={k} -> {}", (lib.process_buffer)(s, t as c_char)));
                    log_state(log, "after", s);
                }
                (lib.destroy_state)(s);
            },
        );
    }
}

// ===========================================================================
// C13 — update_flags over the full flag/mode cross-product (param 0..63)
// ===========================================================================
#[test]
fn c13_update_flags_full_low_cross_product() {
    for param in 0..64i32 {
        diff(&format!("C13 param={param}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(0x4049_0FDBu32 as i32, 128);
            log_state(log, "before", s);
            (lib.update_flags)(s, param);
            log_state(log, "after", s);
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C14 — update_flags over random / negative param
// ===========================================================================
#[test]
fn c14_update_flags_random_param() {
    let mut rng = Rng::new(0xCE);
    for i in 0..600 {
        let param = rng.interesting_i32();
        let init = rng.interesting_i32();
        diff(
            &format!("C14 #{i} param={param} init={init}"),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(init, 128);
                (lib.update_flags)(s, param);
                log_state(log, "after", s);
                (lib.destroy_state)(s);
            },
        );
    }
}

// ===========================================================================
// C15 — 5-bit counter wrap-around over 40 successive update_flags calls
// ===========================================================================
#[test]
fn c15_update_flags_counter_wraparound() {
    let mut rng = Rng::new(0xCF);
    for i in 0..40 {
        let seed = rng.next_u64();
        let init = rng.interesting_i32();
        diff(&format!("C15 #{i}"), &move |lib, log| unsafe {
            let mut r = Rng::new(seed);
            let s = (lib.create_state)(init, 128);
            for k in 0..40 {
                let param = r.interesting_i32();
                (lib.update_flags)(s, param);
                log_state(log, &format!("k={k} param={param}"), s);
            }
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C16 — the whole 32-bit flags word survives update_flags
//        (proves status/reserved are untouched)
// ===========================================================================
#[test]
fn c16_update_flags_preserves_status_and_reserved() {
    let mut rng = Rng::new(0xD0);
    for i in 0..300 {
        let param = rng.interesting_i32();
        diff(&format!("C16 #{i} param={param}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(0, 128);
            let before = snapshot(s);
            (lib.update_flags)(s, param);
            let after = snapshot(s);
            log.push(format!(
                "before=0x{:08x} after=0x{:08x} status={} reserved={}",
                before.flags,
                after.flags,
                after.status(),
                after.reserved()
            ));
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C17 — confuse_types(0) writes 1078530011, then re-read through 1/2/3
// ===========================================================================
#[test]
fn c17_confuse_types_op0_then_reread() {
    let mut rng = Rng::new(0xD1);
    for i in 0..200 {
        let init = rng.interesting_i32();
        diff(&format!("C17 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            log.push(format!("op0 -> {}", (lib.confuse_types)(s, 0)));
            log_state(log, "after0", s);
            log.push(format!("op1 -> {}", (lib.confuse_types)(s, 1)));
            log.push(format!("op2 -> {}", (lib.confuse_types)(s, 2)));
            log.push(format!("op3 -> {}", (lib.confuse_types)(s, 3)));
            log_state(log, "end", s);
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C18 — confuse_types(1): random float bit patterns (%f print + cvttss2si)
// ===========================================================================
#[test]
fn c18_confuse_types_op1_random_bits() {
    let mut rng = Rng::new(0xD2);
    for i in 0..1500 {
        let init = rng.next_i32();
        diff(
            &format!("C18 #{i} init={init} (0x{:08x})", init as u32),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(init, 128);
                log.push(format!("op1 -> {}", (lib.confuse_types)(s, 1)));
                (lib.destroy_state)(s);
            },
        );
    }
}

// ===========================================================================
// C19 — confuse_types(1): curated IEEE-754 classes + overflow boundary
// ===========================================================================
#[test]
fn c19_confuse_types_op1_curated_float_classes() {
    for &init in FLOAT_BITS.iter() {
        diff(
            &format!("C19 init=0x{:08x}", init as u32),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(init, 128);
                log.push(format!("op1 -> {}", (lib.confuse_types)(s, 1)));
                log.push(format!("op1 again -> {}", (lib.confuse_types)(s, 1)));
                log_state(log, "state", s);
                (lib.destroy_state)(s);
            },
        );
    }
    // walk the cvttss2si boundary one float ULP at a time
    for delta in -6i32..=6 {
        for base in [0x4F00_0000u32, 0xCF00_0000, 0x4CBE_BC20, 0x4BA7_D8C0] {
            let init = base.wrapping_add(delta as u32) as i32;
            diff(
                &format!("C19b base=0x{base:08x} delta={delta}"),
                &move |lib, log| unsafe {
                    let s = (lib.create_state)(init, 128);
                    log.push(format!("op1 -> {}", (lib.confuse_types)(s, 1)));
                    (lib.destroy_state)(s);
                },
            );
        }
    }
}

// ===========================================================================
// C20 — confuse_types(2): %u print + & 0xFF
// ===========================================================================
#[test]
fn c20_confuse_types_op2_random() {
    let mut rng = Rng::new(0xD4);
    for i in 0..600 {
        let init = rng.interesting_i32();
        diff(&format!("C20 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            log.push(format!("op2 -> {}", (lib.confuse_types)(s, 2)));
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C21 — confuse_types(3): four signed bytes + bytes[0]+bytes[1]
// ===========================================================================
#[test]
fn c21_confuse_types_op3_random() {
    let mut rng = Rng::new(0xD5);
    for i in 0..600 {
        let init = rng.interesting_i32();
        diff(&format!("C21 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            log.push(format!("op3 -> {}", (lib.confuse_types)(s, 3)));
            (lib.destroy_state)(s);
        });
    }
    // exhaustively cover the sign combinations of bytes[0]/bytes[1]
    for b0 in [0x00u32, 0x01, 0x7F, 0x80, 0xFF] {
        for b1 in [0x00u32, 0x01, 0x7F, 0x80, 0xFF] {
            let init = (b0 | (b1 << 8) | (0x80 << 16) | (0xFFu32 << 24)) as i32;
            diff(
                &format!("C21x b0={b0:#x} b1={b1:#x}"),
                &move |lib, log| unsafe {
                    let s = (lib.create_state)(init, 128);
                    log.push(format!("op3 -> {}", (lib.confuse_types)(s, 3)));
                    (lib.destroy_state)(s);
                },
            );
        }
    }
}

// ===========================================================================
// C22 — ops applied in sequence 0 -> 1 -> 2 -> 3 on one state
// ===========================================================================
#[test]
fn c22_confuse_types_sequence_0123() {
    let mut rng = Rng::new(0xD6);
    for i in 0..200 {
        let init = rng.interesting_i32();
        diff(&format!("C22 #{i} init={init}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, 128);
            for op in 0..4 {
                log.push(format!("op{op} -> {}", (lib.confuse_types)(s, op)));
                log_state(log, &format!("after{op}"), s);
            }
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C23 — random operation sequences of length 6 (incl. out-of-range values)
// ===========================================================================
#[test]
fn c23_confuse_types_random_op_sequences() {
    let mut rng = Rng::new(0xD7);
    for i in 0..400 {
        let init = rng.interesting_i32();
        let mut ops = [0i32; 6];
        for o in ops.iter_mut() {
            *o = match rng.below(6) {
                0 => 0,
                1 => 1,
                2 => 2,
                3 => 3,
                4 => *rng.pick(&BOUNDARY_I32),
                _ => rng.next_i32(),
            };
        }
        diff(
            &format!("C23 #{i} init={init} ops={ops:?}"),
            &move |lib, log| unsafe {
                let s = (lib.create_state)(init, 128);
                for op in ops {
                    log.push(format!("op{op} -> {}", (lib.confuse_types)(s, op)));
                    log_state(log, "st", s);
                }
                (lib.destroy_state)(s);
            },
        );
    }
}

// ===========================================================================
// C24 — full low-level pipeline, replicating `confusion` by hand
// ===========================================================================
fn manual_confusion(
    lib: &'static Lib,
    log: &mut Vec<String>,
    p1: i32,
    p2: i32,
    p3: i32,
    p4: i32,
    capacity: i32,
) {
    unsafe {
        let s = (lib.create_state)(p1, capacity);
        log_state(log, "created", s);
        if s.is_null() {
            log.push("result=-1".into());
            return;
        }
        (lib.update_flags)(s, p2);
        log_state(log, "flagged", s);

        let search = (b'0' as i32).wrapping_add(p3 % 10) as i8;
        let found = (lib.process_buffer)(s, search as c_char);
        log.push(format!("search={search} found={found}"));

        let cr = (lib.confuse_types)(s, p4 % 4);
        log.push(format!("confuse={cr}"));

        let snap = snapshot(s);
        let result = (found.wrapping_mul(10))
            .wrapping_add(cr)
            .wrapping_add((snap.counter() as i32).wrapping_mul(5))
            .wrapping_add((snap.mode() as i32).wrapping_mul(3));
        log.push(format!("result={result}"));
        log_state(log, "final", s);
        (lib.destroy_state)(s);
    }
}

#[test]
fn c24_manual_pipeline_random_params() {
    let mut rng = Rng::new(0xD8);
    for i in 0..500 {
        let (p1, p2, p3, p4) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff(
            &format!("C24 #{i} {p1},{p2},{p3},{p4}"),
            &move |lib, log| manual_confusion(lib, log, p1, p2, p3, p4, 128),
        );
    }
    // same pipeline over unusual capacities
    let mut rng = Rng::new(0xD81);
    for cap in [1i32, 2, 7, 14, 15, 16, 17, 64, 4096] {
        for i in 0..25 {
            let (p1, p2, p3, p4) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            diff(
                &format!("C24c cap={cap} #{i}"),
                &move |lib, log| manual_confusion(lib, log, p1, p2, p3, p4, cap),
            );
        }
    }
}

// ===========================================================================
// C25 — pipeline in an order the convenience wrapper never produces
// ===========================================================================
#[test]
fn c25_manual_pipeline_reordered() {
    let mut rng = Rng::new(0xD9);
    for i in 0..400 {
        let (p1, p2, p3, p4) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff(&format!("C25 #{i}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(p1, 128);
            log.push(format!("confuse -> {}", (lib.confuse_types)(s, p4 % 4)));
            log_state(log, "after_confuse", s);
            let search = (b'0' as i32).wrapping_add(p3 % 10) as i8;
            log.push(format!("pb -> {}", (lib.process_buffer)(s, search as c_char)));
            (lib.update_flags)(s, p2);
            log_state(log, "after_flags", s);
            log.push(format!("confuse2 -> {}", (lib.confuse_types)(s, p4 % 4)));
            log_state(log, "final", s);
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C26 — pipeline with k update_flags calls (counter != 1)
// ===========================================================================
#[test]
fn c26_manual_pipeline_multiple_update_flags() {
    let mut rng = Rng::new(0xDA);
    for i in 0..300 {
        let seed = rng.next_u64();
        let k = 1 + rng.below(40) as u32;
        let (p1, p3, p4) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff(&format!("C26 #{i} k={k}"), &move |lib, log| unsafe {
            let mut r = Rng::new(seed);
            let s = (lib.create_state)(p1, 128);
            for _ in 0..k {
                (lib.update_flags)(s, r.interesting_i32());
            }
            log_state(log, "flagged", s);
            let search = (b'0' as i32).wrapping_add(p3 % 10) as i8;
            let found = (lib.process_buffer)(s, search as c_char);
            let cr = (lib.confuse_types)(s, p4 % 4);
            let snap = snapshot(s);
            let result = found
                .wrapping_mul(10)
                .wrapping_add(cr)
                .wrapping_add((snap.counter() as i32).wrapping_mul(5))
                .wrapping_add((snap.mode() as i32).wrapping_mul(3));
            log.push(format!("found={found} cr={cr} result={result}"));
            (lib.destroy_state)(s);
        });
    }
}

// ===========================================================================
// C27 — confusion() with all four params random over the full int32 range
// ===========================================================================
#[test]
fn c27_confusion_random_params() {
    let mut rng = Rng::new(0xDB);
    for i in 0..1200 {
        let (p1, p2, p3, p4) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff(
            &format!("C27 #{i} {p1},{p2},{p3},{p4}"),
            &move |lib, log| unsafe {
                log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, p4)));
            },
        );
    }
    // fully uniform bit patterns as well
    let mut rng = Rng::new(0xDB2);
    for i in 0..800 {
        let (p1, p2, p3, p4) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        diff(
            &format!("C27u #{i} {p1},{p2},{p3},{p4}"),
            &move |lib, log| unsafe {
                log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, p4)));
            },
        );
    }
}

// ===========================================================================
// C28 — confusion() with param4 % 4 pinned to each of 0,1,2,3
// ===========================================================================
#[test]
fn c28_confusion_each_positive_operation() {
    let mut rng = Rng::new(0xDC);
    for op in 0..4i32 {
        for i in 0..150 {
            let p4 = op + 4 * (rng.below(1_000_000) as i32);
            let (p1, p2, p3) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            assert_eq!(p4 % 4, op);
            diff(
                &format!("C28 op={op} #{i} p1={p1} p2={p2} p3={p3} p4={p4}"),
                &move |lib, log| unsafe {
                    log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, p4)));
                },
            );
        }
    }
}

// ===========================================================================
// C29 — confusion() with param4 % 4 pinned to each of -1,-2,-3
// ===========================================================================
#[test]
fn c29_confusion_each_negative_operation() {
    let mut rng = Rng::new(0xDD);
    for op in [-1i32, -2, -3, 0] {
        for i in 0..150 {
            let p4 = op - 4 * (rng.below(1_000_000) as i32);
            let (p1, p2, p3) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            assert_eq!(p4 % 4, op);
            diff(
                &format!("C29 op={op} #{i} p4={p4}"),
                &move |lib, log| unsafe {
                    log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, p4)));
                },
            );
        }
    }
}

// ===========================================================================
// C30 — confusion() with param3 % 10 pinned to each of -9..9
// ===========================================================================
#[test]
fn c30_confusion_each_search_char() {
    let mut rng = Rng::new(0xDE);
    for rem in -9i32..=9 {
        for i in 0..60 {
            let mag = rng.below(100_000) as i32;
            let p3 = if rem >= 0 { rem + 10 * mag } else { rem - 10 * mag };
            assert_eq!(p3 % 10, rem);
            let (p1, p2, p4) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            diff(
                &format!("C30 rem={rem} #{i} p3={p3}"),
                &move |lib, log| unsafe {
                    log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, p4)));
                },
            );
        }
    }
}

// ===========================================================================
// C31 — confusion() with param2 = 0..63 (full flag/mode cross-product)
// ===========================================================================
#[test]
fn c31_confusion_all_low_param2() {
    let mut rng = Rng::new(0xDF);
    for p2 in 0..64i32 {
        for i in 0..8 {
            let (p1, p3, p4) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            diff(
                &format!("C31 p2={p2} #{i}"),
                &move |lib, log| unsafe {
                    log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, p4)));
                },
            );
        }
    }
    // negative param2 -> arithmetic shift
    for p2 in -64i32..0 {
        let (p1, p3, p4) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff(&format!("C31n p2={p2}"), &move |lib, log| unsafe {
            log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, p4)));
        });
    }
}

// ===========================================================================
// C32 — confusion() over sampled boundary 4-tuples
// ===========================================================================
#[test]
fn c32_confusion_boundary_tuples() {
    let vals: Vec<i32> = {
        let mut v = BOUNDARY_I32.to_vec();
        v.push(1078530011);
        v.push(-1078530011);
        v
    };
    let mut rng = Rng::new(0xE0);
    for i in 0..1500 {
        let p1 = *rng.pick(&vals);
        let p2 = *rng.pick(&vals);
        let p3 = *rng.pick(&vals);
        let p4 = *rng.pick(&vals);
        diff(
            &format!("C32 #{i} {p1},{p2},{p3},{p4}"),
            &move |lib, log| unsafe {
                log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, p4)));
            },
        );
    }
    // exhaustive over the small square that decides search char x operation
    for p3 in -12i32..=12 {
        for p4 in -8i32..=8 {
            diff(
                &format!("C32e p3={p3} p4={p4}"),
                &move |lib, log| unsafe {
                    log.push(format!(
                        "confusion -> {}",
                        (lib.confusion)(1078530011, 0x2A, p3, p4)
                    ));
                },
            );
        }
    }
}

// ===========================================================================
// C33 — confusion() with param1 = curated float bit patterns and operation 1
// ===========================================================================
#[test]
fn c33_confusion_float_bit_patterns_with_op1() {
    for &p1 in FLOAT_BITS.iter() {
        for p2 in [0i32, 0x3F, -1, 8, 63] {
            for p3 in [0i32, 5, -5] {
                diff(
                    &format!("C33 p1=0x{:08x} p2={p2} p3={p3}", p1 as u32),
                    &move |lib, log| unsafe {
                        log.push(format!("confusion -> {}", (lib.confusion)(p1, p2, p3, 1)));
                    },
                );
            }
        }
    }
    // all four operations for every curated pattern
    for &p1 in FLOAT_BITS.iter() {
        for p4 in [0i32, 1, 2, 3, -1, -2, -3, 4, 5] {
            diff(
                &format!("C33b p1=0x{:08x} p4={p4}", p1 as u32),
                &move |lib, log| unsafe {
                    log.push(format!("confusion -> {}", (lib.confusion)(p1, 42, 7, p4)));
                },
            );
        }
    }
}

// ===========================================================================
// C34 — repeated confusion() invocations are independent
// ===========================================================================
#[test]
fn c34_confusion_repeated_invocations() {
    let mut rng = Rng::new(0xE2);
    for i in 0..80 {
        let seed = rng.next_u64();
        diff(&format!("C34 #{i}"), &move |lib, log| unsafe {
            let mut r = Rng::new(seed);
            for k in 0..12 {
                let (p1, p2, p3, p4) = (
                    r.interesting_i32(),
                    r.interesting_i32(),
                    r.interesting_i32(),
                    r.interesting_i32(),
                );
                log.push(format!("k={k} -> {}", (lib.confusion)(p1, p2, p3, p4)));
            }
        });
    }
}

// ===========================================================================
// Soak — a much larger randomized sweep than the per-row tests, kept out of the
// default run for speed. Run with:
//     cargo test --offline --test phase_b_valid -- --ignored --nocapture
// ===========================================================================
#[test]
#[ignore = "long-running soak; run explicitly with --ignored"]
fn soak_confusion_and_pipeline() {
    let mut rng = Rng::new(0x5041_4B45);
    for i in 0..15000 {
        let (p1, p2, p3, p4) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff(&format!("soak-confusion #{i}"), &move |lib, log| unsafe {
            log.push(format!("-> {}", (lib.confusion)(p1, p2, p3, p4)));
        });
    }
    for i in 0..5000 {
        let init = rng.next_i32();
        let cap = 1 + rng.below(300) as i32;
        let param = rng.interesting_i32();
        let target = rng.next_u32() as u8 as i8;
        let op = rng.next_i32();
        diff(&format!("soak-pipeline #{i}"), &move |lib, log| unsafe {
            let s = (lib.create_state)(init, cap);
            log_state(log, "created", s);
            (lib.update_flags)(s, param);
            log.push(format!("pb -> {}", (lib.process_buffer)(s, target as c_char)));
            log.push(format!("ct -> {}", (lib.confuse_types)(s, op)));
            log_state(log, "final", s);
            (lib.destroy_state)(s);
        });
    }
}
