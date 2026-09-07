// Phase B — valid-path differential tests.
// One test per row of CONFIGS.md. Every call goes through the exported C ABI of
// BOTH the C .so and the Rust .so via libloading.

mod common;

use common::*;
use std::ffi::CString;
use std::ffi::c_char;

const LABELS: [i32; 4] = [10, 20, 30, 40];

// ---------------------------------------------------------------- row 1
#[test]
fn row01_all_default_class_small_random() {
    let mut rng = Rng::new(0xC0FFEE_01);
    for _ in 0..512 {
        // Draw small values, re-drawing anything that would hit a switch label
        // so this row exercises the `default:` arm exclusively.
        let draw = |rng: &mut Rng| loop {
            let v = rng.range_i32(-1000, 1000);
            if !LABELS.contains(&v) {
                return v;
            }
        };
        let (a, b, c, d) = (
            draw(&mut rng),
            draw(&mut rng),
            draw(&mut rng),
            draw(&mut rng),
        );
        let got = diff_cleanup(a, b, c, d);
        assert_eq!(got, model_cleanup(a, b, c, d), "model disagrees on default arm");
        assert_eq!(got, a.wrapping_add(b).wrapping_add(c).wrapping_add(d));
    }
}

// ---------------------------------------------------------------- rows 2-5
#[test]
fn row02_all_tens_fallthrough() {
    // case 10 falls through into case 20 => +30 per slot => 120
    assert_eq!(diff_cleanup(10, 10, 10, 10), 120);
}

#[test]
fn row03_all_twenties() {
    assert_eq!(diff_cleanup(20, 20, 20, 20), 80);
}

#[test]
fn row04_all_thirties_fallthrough() {
    // case 30 falls through into case 40 => +70 per slot => 280
    assert_eq!(diff_cleanup(30, 30, 30, 30), 280);
}

#[test]
fn row05_all_forties() {
    assert_eq!(diff_cleanup(40, 40, 40, 40), 160);
}

// ---------------------------------------------------------------- row 6
#[test]
fn row06_exhaustive_value_class_crossproduct() {
    // One representative per distinguished value class: 10, 20, 30, 40, default.
    const CLASSES: [i32; 5] = [10, 20, 30, 40, 7];
    for &a in &CLASSES {
        for &b in &CLASSES {
            for &c in &CLASSES {
                for &d in &CLASSES {
                    let got = diff_cleanup(a, b, c, d);
                    assert_eq!(got, model_cleanup(a, b, c, d), "at ({a},{b},{c},{d})");
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn row07_each_label_in_each_position() {
    let filler = [1, 2, 3, 5];
    for &label in &LABELS {
        for pos in 0..4usize {
            let mut args = filler;
            args[pos] = label;
            let got = diff_cleanup(args[0], args[1], args[2], args[3]);
            assert_eq!(got, model_cleanup(args[0], args[1], args[2], args[3]));
        }
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn row08_label_neighbours_and_negatives_take_default() {
    const NEAR: [i32; 17] = [
        9, 11, 19, 21, 29, 31, 39, 41, 0, -10, -20, -30, -40, -9, -11, -41, 100,
    ];
    for &v in &NEAR {
        // in every position, and all four at once
        for pos in 0..4usize {
            let mut args = [1, 1, 1, 1];
            args[pos] = v;
            let got = diff_cleanup(args[0], args[1], args[2], args[3]);
            assert_eq!(got, model_cleanup(args[0], args[1], args[2], args[3]));
            // Neighbours must land in `default:` i.e. contribute exactly `v`.
            assert_eq!(got, v.wrapping_add(3), "value {v} did not take default arm");
        }
        let got = diff_cleanup(v, v, v, v);
        assert_eq!(got, model_cleanup(v, v, v, v));
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn row09_accumulator_overflow_shapes() {
    const EXT: [i32; 6] = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 1, -1];
    for &a in &EXT {
        for &b in &EXT {
            for &c in &EXT {
                for &d in &EXT {
                    let got = diff_cleanup(a, b, c, d);
                    assert_eq!(got, model_cleanup(a, b, c, d), "at ({a},{b},{c},{d})");
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn row10_unconstrained_random_ints() {
    let mut rng = Rng::new(0xC0FFEE_10);
    for _ in 0..4096 {
        let (a, b, c, d) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        let got = diff_cleanup(a, b, c, d);
        assert_eq!(got, model_cleanup(a, b, c, d), "at ({a},{b},{c},{d})");
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn row11_mixed_labels_and_random() {
    let mut rng = Rng::new(0xC0FFEE_11);
    for _ in 0..4096 {
        let slot = |rng: &mut Rng| {
            if rng.next_u64() % 2 == 0 {
                rng.pick(&LABELS)
            } else if rng.next_u64() % 3 == 0 {
                rng.next_i32()
            } else {
                rng.range_i32(-64, 64)
            }
        };
        let (a, b, c, d) = (
            slot(&mut rng),
            slot(&mut rng),
            slot(&mut rng),
            slot(&mut rng),
        );
        let got = diff_cleanup(a, b, c, d);
        assert_eq!(got, model_cleanup(a, b, c, d), "at ({a},{b},{c},{d})");
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn row12_stdout_bytes_match_and_are_nonempty() {
    // diff_cleanup already asserts byte equality; additionally pin down the
    // exact expected text so a silently-empty capture cannot pass the row.
    // TO_STRING(numbers) stringizes the macro ARGUMENT => the literal "numbers".
    let expected = b"Processed numbers: numbers\n";
    let l = libs();
    let cf = l.c_cleanup();
    let rf = l.rust_cleanup();
    for args in [
        [0, 0, 0, 0],
        [10, 20, 30, 40],
        [i32::MAX, i32::MIN, 1, -1],
        [7, 7, 7, 7],
        [-1000, 1000, 41, 9],
    ] {
        let c_out = capture_stdout(|| {
            unsafe { cf(args[0], args[1], args[2], args[3]) };
        });
        let r_out = capture_stdout(|| {
            unsafe { rf(args[0], args[1], args[2], args[3]) };
        });
        assert_eq!(c_out, r_out, "stdout mismatch for {args:?}");
        assert_eq!(
            c_out.as_slice(),
            expected,
            "C stdout is not the expected text for {args:?}"
        );
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn row13_print_result_short_labels() {
    let mut rng = Rng::new(0xC0FFEE_13);
    let labels = ["cleanup", "x", "Total", "result value", "a b c"];
    for &lab in &labels {
        let cs = CString::new(lab).unwrap();
        for &r in &[0i32, 1, -1] {
            diff_print_result(cs.as_ptr(), r, lab);
        }
        for _ in 0..64 {
            diff_print_result(cs.as_ptr(), rng.next_i32(), lab);
        }
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn row14_print_result_empty_label() {
    let cs = CString::new("").unwrap();
    let mut rng = Rng::new(0xC0FFEE_14);
    for _ in 0..64 {
        diff_print_result(cs.as_ptr(), rng.next_i32(), "empty");
    }
    diff_print_result(cs.as_ptr(), 0, "empty/0");
}

// ---------------------------------------------------------------- row 15
#[test]
fn row15_print_result_long_label_not_truncated() {
    let long: String = "L".repeat(4096);
    let cs = CString::new(long.clone()).unwrap();
    let l = libs();
    let cf = l.c_print_result();
    let rf = l.rust_print_result();
    let c_out = capture_stdout(|| unsafe { cf(cs.as_ptr(), 42) });
    let r_out = capture_stdout(|| unsafe { rf(cs.as_ptr(), 42) });
    assert_eq!(c_out, r_out, "long-label stdout mismatch");
    let expected = format!("{long}: 42\n");
    assert_eq!(
        c_out.as_slice(),
        expected.as_bytes(),
        "long label was truncated"
    );
}

// ---------------------------------------------------------------- row 16
#[test]
fn row16_print_result_percent_in_label_is_data() {
    for lab in ["%d", "%s", "%n", "100%", "%%", "%p %p %p", "a%sb%dc"] {
        let cs = CString::new(lab).unwrap();
        diff_print_result(cs.as_ptr(), 7, lab);
        // The label must appear verbatim: it is an ARGUMENT to "%s: %d\n".
        let l = libs();
        let cf = l.c_print_result();
        let out = capture_stdout(|| unsafe { cf(cs.as_ptr(), 7) });
        assert_eq!(out, format!("{lab}: 7\n").into_bytes(), "label {lab} reformatted");
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn row17_print_result_non_ascii_label() {
    let mut cases: Vec<Vec<u8>> = vec![
        "héllo wörld".as_bytes().to_vec(),
        "日本語".as_bytes().to_vec(),
        "emoji 🎉".as_bytes().to_vec(),
    ];
    // raw high bytes 0x80..=0xFF (invalid UTF-8, still a valid C string)
    cases.push((0x80u8..=0xFFu8).collect());
    for bytes in cases {
        let cs = CString::new(bytes.clone()).unwrap();
        diff_print_result(cs.as_ptr(), -5, "non-ascii");
        let l = libs();
        let cf = l.c_print_result();
        let out = capture_stdout(|| unsafe { cf(cs.as_ptr(), -5) });
        let mut expected = bytes.clone();
        expected.extend_from_slice(b": -5\n");
        assert_eq!(out, expected, "non-ascii label altered");
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn row18_print_result_extreme_results() {
    let cs = CString::new("v").unwrap();
    for &r in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        diff_print_result(cs.as_ptr(), r, "extreme");
    }
    let l = libs();
    let cf = l.c_print_result();
    let out = capture_stdout(|| unsafe { cf(cs.as_ptr(), i32::MIN) });
    assert_eq!(out, b"v: -2147483648\n".to_vec());
}

// ---------------------------------------------------------------- row 19
#[test]
fn row19_cleanup_resources_null_is_noop() {
    let l = libs();
    let cf = l.c_cleanup_resources();
    let rf = l.rust_cleanup_resources();
    let c_out = capture_stdout(|| unsafe { cf(std::ptr::null_mut()) });
    let r_out = capture_stdout(|| unsafe { rf(std::ptr::null_mut()) });
    assert!(c_out.is_empty(), "C printed on NULL: {c_out:?}");
    assert_eq!(c_out, r_out, "NULL cleanup_resources stdout mismatch");
}

// ---------------------------------------------------------------- row 20
#[test]
fn row20_cleanup_resources_frees_live_blocks() {
    let l = libs();
    let cf = l.c_cleanup_resources();
    let rf = l.rust_cleanup_resources();
    for size in [1usize, 50, 4096] {
        for _ in 0..32 {
            let p = unsafe { malloc(size) } as *mut c_char;
            assert!(!p.is_null());
            let c_out = capture_stdout(|| unsafe { cf(p) });
            let q = unsafe { malloc(size) } as *mut c_char;
            assert!(!q.is_null());
            let r_out = capture_stdout(|| unsafe { rf(q) });
            assert_eq!(c_out, r_out, "cleanup_resources stdout mismatch (size {size})");
            assert!(c_out.is_empty());
        }
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn row21_composed_pipeline_full_stream() {
    // Drive the library the way a real consumer does: run `cleanup`, report the
    // result with `print_result`, and hand a heap block to `cleanup_resources`,
    // comparing the ENTIRE combined stdout stream in one capture.
    let l = libs();
    let c_cleanup = l.c_cleanup();
    let c_print = l.c_print_result();
    let c_free = l.c_cleanup_resources();
    let r_cleanup = l.rust_cleanup();
    let r_print = l.rust_print_result();
    let r_free = l.rust_cleanup_resources();

    let label = CString::new("cleanup").unwrap();
    let mut rng = Rng::new(0xC0FFEE_21);
    let tuples: Vec<[i32; 4]> = (0..256)
        .map(|_| {
            let slot = |rng: &mut Rng| {
                if rng.next_u64() % 3 == 0 {
                    rng.pick(&LABELS)
                } else {
                    rng.next_i32()
                }
            };
            [
                slot(&mut rng),
                slot(&mut rng),
                slot(&mut rng),
                slot(&mut rng),
            ]
        })
        .collect();

    let mut c_rets = Vec::new();
    let c_out = capture_stdout(|| {
        for t in &tuples {
            let r = unsafe { c_cleanup(t[0], t[1], t[2], t[3]) };
            c_rets.push(r);
            unsafe { c_print(label.as_ptr(), r) };
            let p = unsafe { malloc(50) } as *mut c_char;
            unsafe { c_free(p) };
            unsafe { c_free(std::ptr::null_mut()) };
        }
    });

    let mut r_rets = Vec::new();
    let r_out = capture_stdout(|| {
        for t in &tuples {
            let r = unsafe { r_cleanup(t[0], t[1], t[2], t[3]) };
            r_rets.push(r);
            unsafe { r_print(label.as_ptr(), r) };
            let p = unsafe { malloc(50) } as *mut c_char;
            unsafe { r_free(p) };
            unsafe { r_free(std::ptr::null_mut()) };
        }
    });

    assert_eq!(c_rets, r_rets, "pipeline return values diverge");
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "pipeline stdout diverges"
    );
    assert_eq!(c_out, r_out, "pipeline stdout byte mismatch");
    assert!(!c_out.is_empty());
    for (t, r) in tuples.iter().zip(c_rets.iter()) {
        assert_eq!(*r, model_cleanup(t[0], t[1], t[2], t[3]));
    }
}
