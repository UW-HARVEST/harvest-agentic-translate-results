//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and
//! the Rust `.so` through their exported symbols and compares return values
//! and stdout byte-for-byte.

mod common;

use common::*;
use std::ffi::{c_int, CString};

/// The five value classes `switch (numbers[i])` distinguishes.
/// `None` means "fill with a random non-case value" (the `default:` arm).
const CLASSES: [Option<i32>; 5] = [Some(10), Some(20), Some(30), Some(40), None];

/// C1 — full cross-product of the 5 case classes over all 4 argument
/// positions (625 combinations), each replayed with 16 independent random
/// fillings of its `default:` slots.
#[test]
fn c1_case_class_cross_product_all_positions() {
    let mut rng = Rng::new(SEED ^ 0xC1);
    let mut inputs: Vec<[i32; 4]> = Vec::new();
    let mut combos = 0usize;
    for a in CLASSES {
        for b in CLASSES {
            for c in CLASSES {
                for d in CLASSES {
                    combos += 1;
                    for _ in 0..16 {
                        inputs.push([
                            a.unwrap_or_else(|| rng.non_case_i32()),
                            b.unwrap_or_else(|| rng.non_case_i32()),
                            c.unwrap_or_else(|| rng.non_case_i32()),
                            d.unwrap_or_else(|| rng.non_case_i32()),
                        ]);
                    }
                }
            }
        }
    }
    assert_eq!(combos, 625, "cross-product must cover 5^4 combinations");
    assert_eq!(inputs.len(), 625 * 16);
    diff_cleanup("C1", &inputs);
}

/// C2 — all four arguments the same case label: exercises the `10 -> 20` and
/// `30 -> 40` fall-throughs four times over.
#[test]
fn c2_uniform_case_labels() {
    let inputs: Vec<[i32; 4]> = CASE_LABELS.iter().map(|&v| [v, v, v, v]).collect();
    diff_cleanup("C2", &inputs);
}

/// C3 — exactly one case label, the other three slots in `default:`, for every
/// (label, position) pair; 64 random fillings each.
#[test]
fn c3_single_label_each_position() {
    let mut rng = Rng::new(SEED ^ 0xC3);
    let mut inputs: Vec<[i32; 4]> = Vec::new();
    for &label in CASE_LABELS.iter() {
        for pos in 0..4usize {
            for _ in 0..64 {
                let mut q = [
                    rng.non_case_i32(),
                    rng.non_case_i32(),
                    rng.non_case_i32(),
                    rng.non_case_i32(),
                ];
                q[pos] = label;
                inputs.push(q);
            }
        }
    }
    assert_eq!(inputs.len(), 4 * 4 * 64);
    diff_cleanup("C3", &inputs);
}

/// C4 — all four slots in `default:`, small magnitudes.
#[test]
fn c4_default_small_magnitudes() {
    let mut rng = Rng::new(SEED ^ 0xC4);
    let inputs: Vec<[i32; 4]> = (0..4096)
        .map(|_| {
            [
                rng.range_i32(-1000, 1000),
                rng.range_i32(-1000, 1000),
                rng.range_i32(-1000, 1000),
                rng.range_i32(-1000, 1000),
            ]
        })
        .collect();
    diff_cleanup("C4", &inputs);
}

/// C5 — all four slots in `default:`, uniform over the whole `int` range.
/// This is where accumulation wraps.
#[test]
fn c5_default_full_int_range() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    let inputs: Vec<[i32; 4]> = (0..8192)
        .map(|_| {
            [
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            ]
        })
        .collect();
    diff_cleanup("C5", &inputs);
}

/// C6 — exhaustive 8^4 cross-product of the values one step either side of
/// every case label. All land in `default:`; a mis-transcribed `match` arm
/// (e.g. an inclusive range) would show up here.
#[test]
fn c6_boundary_values_adjacent_to_case_labels() {
    const ADJ: [i32; 8] = [9, 11, 19, 21, 29, 31, 39, 41];
    let mut inputs: Vec<[i32; 4]> = Vec::with_capacity(8 * 8 * 8 * 8);
    for a in ADJ {
        for b in ADJ {
            for c in ADJ {
                for d in ADJ {
                    inputs.push([a, b, c, d]);
                }
            }
        }
    }
    assert_eq!(inputs.len(), 4096);
    diff_cleanup("C6", &inputs);
}

/// C7 — exhaustive 7^4 cross-product of extreme `int` values: overflow of the
/// accumulator in both directions.
#[test]
fn c7_extreme_values_cross_product() {
    const EXT: [i32; 7] = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut inputs: Vec<[i32; 4]> = Vec::with_capacity(7 * 7 * 7 * 7);
    for a in EXT {
        for b in EXT {
            for c in EXT {
                for d in EXT {
                    inputs.push([a, b, c, d]);
                }
            }
        }
    }
    assert_eq!(inputs.len(), 2401);
    diff_cleanup("C7", &inputs);
}

/// C8 — case labels interleaved with extreme `default:` values: fall-through
/// and overflow interacting in the same call.
#[test]
fn c8_labels_mixed_with_extremes() {
    let pool: [i32; 13] = [
        10,
        20,
        30,
        40,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -1,
        0,
        1,
        i32::MAX / 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut rng = Rng::new(SEED ^ 0xC8);
    let inputs: Vec<[i32; 4]> = (0..4096)
        .map(|_| {
            [
                *rng.pick(&pool),
                *rng.pick(&pool),
                *rng.pick(&pool),
                *rng.pick(&pool),
            ]
        })
        .collect();
    diff_cleanup("C8", &inputs);
}

/// Label shapes `print_result` must render identically.
fn label_shapes(rng: &mut Rng) -> CString {
    match rng.next_u64() % 6 {
        0 => CString::new("").unwrap(),
        1 => CString::new("Total").unwrap(),
        2 => CString::new("%s %d %n %% %p %10$s").unwrap(),
        3 => CString::new("x".repeat(4096)).unwrap(),
        4 => CString::new("ünïcødé ▲ 漢字").unwrap(),
        _ => {
            // random printable ASCII, never containing an interior NUL
            let n = (rng.next_u64() % 64) as usize + 1;
            let s: String = (0..n)
                .map(|_| char::from(rng.range_i32(0x20, 0x7e) as u8))
                .collect();
            CString::new(s).unwrap()
        }
    }
}

/// C9 — `print_result` over the cross-product of label shapes and result
/// magnitudes, driven as a low-level entry point in its own right.
#[test]
fn c9_print_result_label_and_result_shapes() {
    let mut rng = Rng::new(SEED ^ 0xC9);
    let results: [c_int; 6] = [0, -1, 1, i32::MIN, i32::MAX, 42];
    let mut inputs: Vec<(CString, c_int)> = Vec::new();
    for _ in 0..2048 {
        let label = label_shapes(&mut rng);
        let r = if rng.next_u64() % 2 == 0 {
            *rng.pick(&results)
        } else {
            rng.next_i32()
        };
        inputs.push((label, r));
    }
    diff_print_result("C9", &inputs);
}

/// C10 — `cleanup_resources` with a live, host-`malloc`ed pointer. Each
/// pointer is handed to exactly one implementation so it is freed once.
#[test]
fn c10_cleanup_resources_valid_pointers() {
    let l = libs();
    for size in [1usize, 50, 4096] {
        for _ in 0..64 {
            let pc = host_malloc(size);
            assert!(!pc.is_null(), "host malloc({size}) failed");
            let (_, oc) = capture_stdout(|| unsafe { (l.c.cleanup_resources)(pc) });

            let pr = host_malloc(size);
            assert!(!pr.is_null(), "host malloc({size}) failed");
            let (_, or) = capture_stdout(|| unsafe { (l.rust.cleanup_resources)(pr) });

            assert_eq!(oc, or, "C10: cleanup_resources stdout differed (size {size})");
            assert!(
                oc.is_empty(),
                "C10: cleanup_resources must print nothing, got {:?}",
                String::from_utf8_lossy(&oc)
            );
        }
    }
}

/// C11 — end-to-end consumer sequence: `cleanup`'s return value is fed
/// straight into `print_result`, and the *combined* stdout of the pair is
/// compared. This is the composed pipeline, invisible to per-function tests.
#[test]
fn c11_cleanup_then_print_result_pipeline() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xC11);

    let pool: [i32; 9] = [10, 20, 30, 40, 0, -1, 7, i32::MAX, i32::MIN];
    let cases: Vec<([i32; 4], CString)> = (0..2048)
        .map(|_| {
            let q = if rng.next_u64() % 2 == 0 {
                [
                    *rng.pick(&pool),
                    *rng.pick(&pool),
                    *rng.pick(&pool),
                    *rng.pick(&pool),
                ]
            } else {
                [
                    rng.next_i32(),
                    rng.next_i32(),
                    rng.next_i32(),
                    rng.next_i32(),
                ]
            };
            (q, label_shapes(&mut rng))
        })
        .collect();

    let run = |im: &Impl| -> (Vec<c_int>, Vec<u8>) {
        capture_stdout(|| {
            cases
                .iter()
                .map(|(q, label)| {
                    let r = unsafe { (im.cleanup)(q[0], q[1], q[2], q[3]) };
                    unsafe { (im.print_result)(label.as_ptr(), r) };
                    r
                })
                .collect()
        })
    };

    let (rc, oc) = run(&l.c);
    let (rr, or) = run(&l.rust);

    for (i, (q, _)) in cases.iter().enumerate() {
        assert_eq!(
            rc[i], rr[i],
            "C11: cleanup({}, {}, {}, {}) C={} Rust={}",
            q[0], q[1], q[2], q[3], rc[i], rr[i]
        );
    }
    assert_eq!(
        oc.len(),
        or.len(),
        "C11: combined stdout length differed ({} vs {})",
        oc.len(),
        or.len()
    );
    assert!(oc == or, "C11: combined pipeline stdout diverged");
}

/// C12 — statefulness check: the same quadruple repeated must produce an
/// identical return value and identical stdout on every call, in both
/// implementations (no leaked accumulator, no heap-dependent formatting).
#[test]
fn c12_repeated_invocation_is_stateless() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xC12);
    let pool: [i32; 8] = [10, 20, 30, 40, 0, -5, 99, i32::MAX];

    for _ in 0..256 {
        let q = [
            *rng.pick(&pool),
            *rng.pick(&pool),
            *rng.pick(&pool),
            *rng.pick(&pool),
        ];
        let reps = 32;

        let run = |im: &Impl| -> (Vec<c_int>, Vec<u8>) {
            capture_stdout(|| {
                (0..reps)
                    .map(|_| unsafe { (im.cleanup)(q[0], q[1], q[2], q[3]) })
                    .collect()
            })
        };
        let (rc, oc) = run(&l.c);
        let (rr, or) = run(&l.rust);

        assert!(
            rc.iter().all(|&v| v == rc[0]),
            "C12: C `cleanup` is not stateless for {q:?}: {rc:?}"
        );
        assert!(
            rr.iter().all(|&v| v == rr[0]),
            "C12: Rust `cleanup` is not stateless for {q:?}: {rr:?}"
        );
        assert_eq!(rc[0], rr[0], "C12: return value differed for {q:?}");
        assert!(oc == or, "C12: stdout differed across repetition for {q:?}");
    }
}
