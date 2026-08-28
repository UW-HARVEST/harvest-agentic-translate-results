// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Everything is driven through the exported C
// symbols of both shared objects; both the returned `int` and the exact stdout
// bytes are compared.

#[path = "common/mod.rs"]
mod common;

use common::*;
use core::ffi::c_char;

fn nul(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Row 1 — baseline happy path, and the stringisation of TO_STRING(numbers).
// ---------------------------------------------------------------------------
fn row01_baseline_default_args() {
    let mut cap = Capture::new("b01");
    let (rc, out) = diff_cleanup_out(&mut cap, 1, 2, 3, 4);
    assert_eq!(rc, 10, "1+2+3+4 all take the default arm");
    // TO_STRING(numbers) stringises the macro *argument*: the text is the
    // literal "numbers", not the array contents.
    assert_eq!(
        out,
        EXPECTED_CLEANUP_STDOUT,
        "unexpected stdout: \"{}\"",
        show(&out)
    );
}

// ---------------------------------------------------------------------------
// Row 2 — each case label alone in slot 0.
// ---------------------------------------------------------------------------
fn row02_each_case_label_slot0() {
    let mut cap = Capture::new("b02");
    for (v, expect) in [(10, 30), (20, 20), (30, 70), (40, 40)] {
        let rc = diff_cleanup(&mut cap, v, 0, 0, 0);
        assert_eq!(rc, expect, "cleanup({v},0,0,0)");
    }
}

// ---------------------------------------------------------------------------
// Row 3 — each case label in every slot (position independence).
// ---------------------------------------------------------------------------
fn row03_each_case_label_every_slot() {
    let mut cap = Capture::new("b03");
    for (v, expect) in [(10, 30), (20, 20), (30, 70), (40, 40)] {
        for slot in 0..4 {
            let mut args = [0i32; 4];
            args[slot] = v;
            let rc = diff_cleanup(&mut cap, args[0], args[1], args[2], args[3]);
            assert_eq!(rc, expect, "label {v} in slot {slot}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — exhaustive cross product of the 5 switch classes over 4 slots.
// ---------------------------------------------------------------------------
fn row04_exhaustive_switch_class_cross_product() {
    let mut cap = Capture::new("b04");
    // `default`-class representatives, rotated so the exhaustive sweep also
    // varies the value carried by the default arm.
    let defaults = [0i32, 7, -7, 12345, -12345, 41, i32::MAX, i32::MIN, 100000];
    let mut di = 0usize;
    let mut n = 0usize;
    for c0 in 0..5 {
        for c1 in 0..5 {
            for c2 in 0..5 {
                for c3 in 0..5 {
                    let mut args = [0i32; 4];
                    for (slot, class) in [c0, c1, c2, c3].into_iter().enumerate() {
                        args[slot] = if class < 4 {
                            CASE_LABELS[class]
                        } else {
                            let v = defaults[di % defaults.len()];
                            di += 1;
                            v
                        };
                    }
                    let rc = diff_cleanup(&mut cap, args[0], args[1], args[2], args[3]);
                    assert_eq!(rc, model_cleanup(args[0], args[1], args[2], args[3]), "{args:?}");
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 625, "5^4 class combinations");
}

// ---------------------------------------------------------------------------
// Row 5 — all four args the same case label.
// ---------------------------------------------------------------------------
fn row05_all_same_case_label() {
    let mut cap = Capture::new("b05");
    for (v, expect) in [(10, 120), (20, 80), (30, 280), (40, 160)] {
        let rc = diff_cleanup(&mut cap, v, v, v, v);
        assert_eq!(rc, expect, "cleanup({v} x4)");
    }
}

// ---------------------------------------------------------------------------
// Row 6 — all 24 permutations of the four case labels.
// ---------------------------------------------------------------------------
fn row06_all_permutations_of_case_labels() {
    let mut cap = Capture::new("b06");
    let base = [10i32, 20, 30, 40];
    let mut count = 0;
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                for l in 0..4 {
                    if [i, j, k, l].iter().collect::<std::collections::HashSet<_>>().len() != 4 {
                        continue;
                    }
                    let rc = diff_cleanup(&mut cap, base[i], base[j], base[k], base[l]);
                    assert_eq!(rc, 30 + 20 + 70 + 40, "permutation {i}{j}{k}{l}");
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 24);
}

// ---------------------------------------------------------------------------
// Row 7 — exhaustive over the near-case boundary values (one step either side
// of every case label).
// ---------------------------------------------------------------------------
fn row07_exhaustive_near_case_boundaries() {
    let mut cap = Capture::new("b07");
    let vs = NEAR_CASE;
    let mut n = 0;
    for &a in &vs {
        for &b in &vs {
            for &c0 in &vs {
                for &d in &vs {
                    let rc = diff_cleanup(&mut cap, a, b, c0, d);
                    assert_eq!(rc, a + b + c0 + d, "all near-case values take default");
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 4096, "8^4");
}

// ---------------------------------------------------------------------------
// Row 8 — negated case labels must hit `default`, not `case`.
// ---------------------------------------------------------------------------
fn row08_negated_case_labels_hit_default() {
    let mut cap = Capture::new("b08");
    let vs = NEGATED_CASE;
    let mut n = 0;
    for &a in &vs {
        for &b in &vs {
            for &c0 in &vs {
                for &d in &vs {
                    let rc = diff_cleanup(&mut cap, a, b, c0, d);
                    assert_eq!(rc, a + b + c0 + d);
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 256, "4^4");
}

// ---------------------------------------------------------------------------
// Row 9 — zeros, and zero mixed with each case label.
// ---------------------------------------------------------------------------
fn row09_zeros_and_mixed_with_labels() {
    let mut cap = Capture::new("b09");
    assert_eq!(diff_cleanup(&mut cap, 0, 0, 0, 0), 0);
    for &v in &CASE_LABELS {
        for slot in 0..4 {
            let mut args = [0i32; 4];
            args[slot] = v;
            diff_cleanup(&mut cap, args[0], args[1], args[2], args[3]);
        }
        diff_cleanup(&mut cap, v, 0, v, 0);
        diff_cleanup(&mut cap, 0, v, 0, v);
    }
}

// ---------------------------------------------------------------------------
// Row 10 — accumulator overflow shapes.
// ---------------------------------------------------------------------------
fn row10_overflow_shapes() {
    let mut cap = Capture::new("b10");
    let big = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    // 1..=4 slots filled with an extreme, rest 0.
    for &v in &big {
        for count in 1..=4usize {
            let mut args = [0i32; 4];
            for slot in 0..count {
                args[slot] = v;
            }
            let rc = diff_cleanup(&mut cap, args[0], args[1], args[2], args[3]);
            assert_eq!(rc, model_cleanup(args[0], args[1], args[2], args[3]), "{args:?}");
        }
    }
    // Extremes mixed with each other and with the case labels.
    for &v in &big {
        for &w in &big {
            diff_cleanup(&mut cap, v, w, 0, 0);
            diff_cleanup(&mut cap, v, 0, w, 0);
        }
        for &l in &CASE_LABELS {
            diff_cleanup(&mut cap, v, l, l, l);
            diff_cleanup(&mut cap, l, v, l, l);
        }
    }
    // Known wrap results.
    assert_eq!(diff_cleanup(&mut cap, i32::MAX, i32::MAX, 0, 0), -2);
    assert_eq!(diff_cleanup(&mut cap, i32::MIN, i32::MIN, 0, 0), 0);
    assert_eq!(diff_cleanup(&mut cap, i32::MAX, 1, 0, 0), i32::MIN);
    assert_eq!(diff_cleanup(&mut cap, i32::MAX, 10, 0, 0), i32::MIN + 29);
}

// ---------------------------------------------------------------------------
// Row 11 — randomised over the full i32 range.
// ---------------------------------------------------------------------------
fn row11_random_full_i32_range() {
    let mut cap = Capture::new("b11");
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..6000 {
        let (a, b, c0, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let rc = diff_cleanup(&mut cap, a, b, c0, d);
        assert_eq!(rc, model_cleanup(a, b, c0, d), "({a},{b},{c0},{d})");
    }
}

// ---------------------------------------------------------------------------
// Row 12 — randomised over a biased alphabet so the case arms are hit densely.
// ---------------------------------------------------------------------------
fn row12_random_biased_alphabet() {
    let mut cap = Capture::new("b12");
    let mut alphabet: Vec<i32> = Vec::new();
    alphabet.extend_from_slice(&CASE_LABELS);
    alphabet.extend_from_slice(&CASE_LABELS); // weight the case arms
    alphabet.extend_from_slice(&NEAR_CASE);
    alphabet.extend_from_slice(&NEGATED_CASE);
    alphabet.extend_from_slice(&EXTREMES);
    let mut rng = Rng::new(SEED ^ 12);
    let mut hit = [0usize; 5];
    for _ in 0..6000 {
        let a = rng.pick(&alphabet);
        let b = rng.pick(&alphabet);
        let c0 = rng.pick(&alphabet);
        let d = rng.pick(&alphabet);
        for v in [a, b, c0, d] {
            match v {
                10 => hit[0] += 1,
                20 => hit[1] += 1,
                30 => hit[2] += 1,
                40 => hit[3] += 1,
                _ => hit[4] += 1,
            }
        }
        let rc = diff_cleanup(&mut cap, a, b, c0, d);
        assert_eq!(rc, model_cleanup(a, b, c0, d), "({a},{b},{c0},{d})");
    }
    assert!(
        hit.iter().all(|&h| h > 100),
        "every switch arm must be exercised: {hit:?}"
    );
}

// ---------------------------------------------------------------------------
// Row 13 — randomised dense small range around zero.
// ---------------------------------------------------------------------------
fn row13_random_small_range() {
    let mut cap = Capture::new("b13");
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..6000 {
        let a = rng.range_i32(-64, 64);
        let b = rng.range_i32(-64, 64);
        let c0 = rng.range_i32(-64, 64);
        let d = rng.range_i32(-64, 64);
        let rc = diff_cleanup(&mut cap, a, b, c0, d);
        assert_eq!(rc, model_cleanup(a, b, c0, d), "({a},{b},{c0},{d})");
    }
}

// ---------------------------------------------------------------------------
// Row 14 — repeated invocation: no cross-call state, stable output.
// ---------------------------------------------------------------------------
fn row14_repeated_invocation_is_stateless() {
    let mut cap = Capture::new("b14");
    for i in 0..2048 {
        let (rc, out) = diff_cleanup_out(&mut cap, 10, 30, i % 7, 40);
        assert_eq!(rc, 30 + 70 + (i % 7) + 40);
        assert_eq!(out, EXPECTED_CLEANUP_STDOUT, "iteration {i}");
    }
}

// ---------------------------------------------------------------------------
// Rows 15-16 — print_result: normal label x every interesting result value.
// ---------------------------------------------------------------------------
fn row15_16_print_result_label_x_result() {
    let mut cap = Capture::new("b15");
    let label = nul("Result");
    let out = diff_print_result(&mut cap, label.as_ptr() as *const c_char, 0, "\"Result\"");
    assert_eq!(out, b"Result: 0\n");

    for &n in &[0i32, 1, -1, 42, -42, i32::MAX, i32::MIN, i32::MIN + 1, 1000000] {
        let out = diff_print_result(&mut cap, label.as_ptr() as *const c_char, n, "\"Result\"");
        assert_eq!(out, format!("Result: {n}\n").into_bytes());
    }
    for name in ["a", "label with spaces", "UPPER", "0", "-", ":"] {
        let l = nul(name);
        for &n in &[0i32, -1, i32::MAX, i32::MIN] {
            let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, n, name);
            assert_eq!(out, format!("{name}: {n}\n").into_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — empty label.
// ---------------------------------------------------------------------------
fn row17_print_result_empty_label() {
    let mut cap = Capture::new("b17");
    let empty = nul("");
    for &n in &[0i32, 7, -7, i32::MAX, i32::MIN] {
        let out = diff_print_result(&mut cap, empty.as_ptr() as *const c_char, n, "\"\"");
        assert_eq!(out, format!(": {n}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 18 — label containing printf conversion specifiers (it is an argument,
// never a format string).
// ---------------------------------------------------------------------------
fn row18_print_result_percent_specifiers() {
    let mut cap = Capture::new("b18");
    for s in ["%s", "%d", "%%", "%n", "%1000000d", "%s%s%s", "100%", "%p %x %lf"] {
        let l = nul(s);
        for &n in &[0i32, -5, i32::MAX] {
            let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, n, s);
            assert_eq!(out, format!("{s}: {n}\n").into_bytes(), "label {s:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — control characters and non-UTF-8 bytes in the label.
// ---------------------------------------------------------------------------
fn row19_print_result_control_and_non_utf8() {
    let mut cap = Capture::new("b19");
    for s in ["a\nb", "\n", "\t\t", "a\r\nb", "line1\nline2\nline3"] {
        let l = nul(s);
        let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, 3, s);
        assert_eq!(out, format!("{s}: 3\n").into_bytes());
    }
    // Every non-NUL byte value, as one long label and as singletons.
    let mut all: Vec<u8> = (1u8..=255).collect();
    all.push(0);
    let out = diff_print_result(&mut cap, all.as_ptr() as *const c_char, -1, "<all bytes>");
    let mut expect: Vec<u8> = (1u8..=255).collect();
    expect.extend_from_slice(b": -1\n");
    assert_eq!(out, expect);

    for b in [0x80u8, 0xC0, 0xFE, 0xFF, 0x01, 0x7F] {
        let l = vec![b, 0];
        let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, 1, "<byte>");
        assert_eq!(out, vec![b, b':', b' ', b'1', b'\n']);
    }
}

// ---------------------------------------------------------------------------
// Row 20 — oversized labels around and far beyond the stdio buffer size.
// ---------------------------------------------------------------------------
fn row20_print_result_oversized_labels() {
    let mut cap = Capture::new("b20");
    for len in [4095usize, 4096, 4097, 65536, 1 << 20] {
        let mut l = vec![b'x'; len];
        l.push(0);
        let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, 9, "<big>");
        assert_eq!(out.len(), len + 4, "len {len}");
        assert_eq!(&out[len..], b": 9\n");
    }
}

// ---------------------------------------------------------------------------
// Row 21 — randomised labels x randomised results.
// ---------------------------------------------------------------------------
fn row21_print_result_randomised() {
    let mut cap = Capture::new("b21");
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..3000 {
        let len = rng.below(513);
        let mut l: Vec<u8> = (0..len).map(|_| (rng.below(255) + 1) as u8).collect();
        l.push(0);
        let n = rng.next_i32();
        let out = diff_print_result(&mut cap, l.as_ptr() as *const c_char, n, "<random>");
        let mut expect = l[..len].to_vec();
        expect.extend_from_slice(format!(": {n}\n").as_bytes());
        assert_eq!(out, expect);
    }
}

// ---------------------------------------------------------------------------
// Row 22 — cleanup_resources(NULL).
// ---------------------------------------------------------------------------
fn row22_cleanup_resources_null() {
    let mut cap = Capture::new("b22");
    let _ = cap.take();
    unsafe {
        (c().cleanup_resources)(std::ptr::null_mut());
        let out_c = cap.take();
        (rs().cleanup_resources)(std::ptr::null_mut());
        let out_r = cap.take();
        assert!(out_c.is_empty() && out_r.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Row 23 — cleanup_resources on live blocks of every interesting size.
// ---------------------------------------------------------------------------
fn row23_cleanup_resources_sizes() {
    let mut cap = Capture::new("b23");
    for size in [0usize, 1, 8, 49, 50, 51, 4096, 1 << 20] {
        diff_cleanup_resources(&mut cap, size);
    }
}

// ---------------------------------------------------------------------------
// Row 24 — randomised block sizes, alternating which library frees.
// ---------------------------------------------------------------------------
fn row24_cleanup_resources_randomised() {
    let mut cap = Capture::new("b24");
    let mut rng = Rng::new(SEED ^ 24);
    let _ = cap.take();
    for i in 0..1024 {
        let size = rng.below(8193);
        unsafe {
            let p = malloc(size) as *mut core::ffi::c_char;
            assert!(!p.is_null(), "malloc({size})");
            if i % 2 == 0 {
                (c().cleanup_resources)(p);
            } else {
                (rs().cleanup_resources)(p);
            }
        }
        let out = cap.take();
        assert!(out.is_empty(), "cleanup_resources printed \"{}\"", show(&out));
    }
    // The pointer the caller holds is unchanged by the (dead) `= NULL` store;
    // exercise that by freeing through one library and reusing the slot.
    for _ in 0..64 {
        unsafe {
            let p = malloc(50) as *mut core::ffi::c_char;
            (c().cleanup_resources)(p);
            let q = malloc(50) as *mut core::ffi::c_char;
            (rs().cleanup_resources)(q);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 — composed pipeline: cleanup -> print_result.
// ---------------------------------------------------------------------------
fn row25_composed_cleanup_then_print_result() {
    let mut cap = Capture::new("b25");
    let mut rng = Rng::new(SEED ^ 25);
    let alphabet = {
        let mut v = Vec::new();
        v.extend_from_slice(&CASE_LABELS);
        v.extend_from_slice(&NEAR_CASE);
        v.extend_from_slice(&EXTREMES);
        v
    };
    for _ in 0..2000 {
        let a = rng.pick(&alphabet);
        let b = rng.pick(&alphabet);
        let c0 = rng.pick(&alphabet);
        let d = rng.pick(&alphabet);
        let len = rng.below(24);
        let mut label: Vec<u8> = (0..len).map(|_| b'a' + (rng.below(26)) as u8).collect();
        label.push(0);

        let _ = cap.take();
        // Full C pipeline.
        let rc_c = unsafe { (c().cleanup)(a, b, c0, d) };
        unsafe { (c().print_result)(label.as_ptr() as *const c_char, rc_c) };
        let out_c = cap.take();
        // Full Rust pipeline.
        let rc_r = unsafe { (rs().cleanup)(a, b, c0, d) };
        unsafe { (rs().print_result)(label.as_ptr() as *const c_char, rc_r) };
        let out_r = cap.take();

        assert_eq!(rc_c, rc_r, "pipeline return ({a},{b},{c0},{d})");
        assert_eq!(
            out_c,
            out_r,
            "pipeline stdout ({a},{b},{c0},{d}):\n  C   = \"{}\"\n  Rust= \"{}\"",
            show(&out_c),
            show(&out_r)
        );
        let mut expect = EXPECTED_CLEANUP_STDOUT.to_vec();
        expect.extend_from_slice(&label[..len]);
        expect.extend_from_slice(format!(": {rc_c}\n").as_bytes());
        assert_eq!(out_c, expect);
    }
}

// ---------------------------------------------------------------------------
// Row 26 — interleaved use of all three entry points, alternating order.
// ---------------------------------------------------------------------------
fn row26_interleaved_all_entry_points() {
    let mut cap = Capture::new("b26");
    let mut rng = Rng::new(SEED ^ 26);
    let label = nul("mixed");
    for i in 0..512 {
        let a = rng.pick(&[10, 20, 30, 40, 0, 5, -5, i32::MAX]);
        let b = rng.pick(&[10, 20, 30, 40, 0, 5, -5, i32::MIN]);
        let (first, second) = if i % 2 == 0 { (c(), rs()) } else { (rs(), c()) };

        let _ = cap.take();
        let rc1 = unsafe {
            let p = malloc(50) as *mut core::ffi::c_char;
            (first.cleanup_resources)(p);
            let r = (first.cleanup)(a, b, 30, 10);
            (first.print_result)(label.as_ptr() as *const c_char, r);
            r
        };
        let out1 = cap.take();
        let rc2 = unsafe {
            let p = malloc(50) as *mut core::ffi::c_char;
            (second.cleanup_resources)(p);
            let r = (second.cleanup)(a, b, 30, 10);
            (second.print_result)(label.as_ptr() as *const c_char, r);
            r
        };
        let out2 = cap.take();

        assert_eq!(rc1, rc2, "{} vs {} on ({a},{b},30,10)", first.name, second.name);
        assert_eq!(
            out1,
            out2,
            "{} vs {} stdout:\n  {} = \"{}\"\n  {} = \"{}\"",
            first.name,
            second.name,
            first.name,
            show(&out1),
            second.name,
            show(&out2)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 27 lives in phase_c_errors.rs (it depends on the forced malloc failure).
// Row 28 is driven by tests/run_all.sh across profiles/feature sets.
// ---------------------------------------------------------------------------

fn main() {
    common::run_tests(&[
        ("row01_baseline_default_args", row01_baseline_default_args),
        ("row02_each_case_label_slot0", row02_each_case_label_slot0),
        ("row03_each_case_label_every_slot", row03_each_case_label_every_slot),
        (
            "row04_exhaustive_switch_class_cross_product",
            row04_exhaustive_switch_class_cross_product,
        ),
        ("row05_all_same_case_label", row05_all_same_case_label),
        ("row06_all_permutations_of_case_labels", row06_all_permutations_of_case_labels),
        ("row07_exhaustive_near_case_boundaries", row07_exhaustive_near_case_boundaries),
        ("row08_negated_case_labels_hit_default", row08_negated_case_labels_hit_default),
        ("row09_zeros_and_mixed_with_labels", row09_zeros_and_mixed_with_labels),
        ("row10_overflow_shapes", row10_overflow_shapes),
        ("row11_random_full_i32_range", row11_random_full_i32_range),
        ("row12_random_biased_alphabet", row12_random_biased_alphabet),
        ("row13_random_small_range", row13_random_small_range),
        ("row14_repeated_invocation_is_stateless", row14_repeated_invocation_is_stateless),
        ("row15_16_print_result_label_x_result", row15_16_print_result_label_x_result),
        ("row17_print_result_empty_label", row17_print_result_empty_label),
        ("row18_print_result_percent_specifiers", row18_print_result_percent_specifiers),
        ("row19_print_result_control_and_non_utf8", row19_print_result_control_and_non_utf8),
        ("row20_print_result_oversized_labels", row20_print_result_oversized_labels),
        ("row21_print_result_randomised", row21_print_result_randomised),
        ("row22_cleanup_resources_null", row22_cleanup_resources_null),
        ("row23_cleanup_resources_sizes", row23_cleanup_resources_sizes),
        ("row24_cleanup_resources_randomised", row24_cleanup_resources_randomised),
        (
            "row25_composed_cleanup_then_print_result",
            row25_composed_cleanup_then_print_result,
        ),
        ("row26_interleaved_all_entry_points", row26_interleaved_all_entry_points),
    ]);
}
