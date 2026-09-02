//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads both `.so` files through
//! `libloading` and compares the bytes each writes to `stdout`.

mod harness;

use harness::{harness, Entry, Harness, Rng, BOUNDARY_ARGS};

/// CONFIGS row 1 — pristine global state, `run(0)`, pinned against the literal
/// text derived from `c_src/src/driver.c`.
///
/// ```c
/// static house_t the_house = {.floors = 2, .bedrooms = 5, .bathrooms = 2.5};
/// void run(int extra_bedrooms) {
///     print_the_house();          // 2 floors, 5 bedrooms, 2.5 bathrooms
///     add_floor_to_the_house();   // floors: 2 -> 3
///     print_the_house();          // 3 floors, 5 bedrooms, 2.5 bathrooms
///     the_house.bathrooms += 1.0; // bathrooms: 2.5 -> 3.5
///     print_the_house();          // 3 floors, 5 bedrooms, 3.5 bathrooms
///     add_bedrooms(&the_house, 0);
///     print_the_house();          // 3 floors, 5 bedrooms, 3.5 bathrooms
/// }
/// ```
#[test]
fn cfg_01_pristine_state_exact_text() {
    let h = harness();
    let (c_out, r_out) = h.pristine_run0();

    const EXPECTED: &str = "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n\
                            The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n\
                            The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n\
                            The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n";

    assert_eq!(
        String::from_utf8_lossy(c_out),
        EXPECTED,
        "C pristine run(0) output does not match the text derived from driver.c"
    );
    assert_eq!(
        c_out,
        r_out,
        "\npristine run(0) divergence\n  C:    {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(c_out),
        String::from_utf8_lossy(r_out)
    );
}

/// CONFIGS row 2 — `run` with a zero delta, repeated. Isolates the
/// `floors`/`bathrooms` accumulation from any `bedrooms` change.
#[test]
fn cfg_02_run_zero_delta_repeated() {
    let mut h = harness();
    for i in 0..200 {
        h.assert_same(Entry::Run, 0, &format!("cfg02/iter{i}"));
    }
}

/// CONFIGS row 3 — `run`, random small positive deltas.
#[test]
fn cfg_03_run_small_positive_random() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_0003_5EED_0001);
    for i in 0..400 {
        let arg = rng.range_i32(1, 1000);
        h.assert_same(Entry::Run, arg, &format!("cfg03/iter{i}"));
    }
}

/// CONFIGS row 4 — `run`, random small negative deltas (drives `bedrooms` down
/// and through zero into negative `%d` output).
#[test]
fn cfg_04_run_small_negative_random() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_0004_5EED_0002);
    for i in 0..400 {
        let arg = rng.range_i32(-1000, -1);
        h.assert_same(Entry::Run, arg, &format!("cfg04/iter{i}"));
    }
}

/// CONFIGS row 5 — `run`, uniform over the FULL `i32` domain. Exercises
/// two's-complement wrapping of `bedrooms += extra_bedrooms` in both directions
/// from arbitrary accumulated starting values.
#[test]
fn cfg_05_run_full_i32_random() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_0005_5EED_0003);
    for i in 0..500 {
        let arg = rng.next_i32();
        h.assert_same(Entry::Run, arg, &format!("cfg05/iter{i}"));
    }
}

/// CONFIGS row 6 — `run` over the exhaustive boundary argument set.
#[test]
fn cfg_06_run_boundary_set() {
    let mut h = harness();
    // Several passes so each boundary value is applied from a different
    // accumulated state.
    for pass in 0..4 {
        for &arg in BOUNDARY_ARGS {
            h.assert_same(Entry::Run, arg, &format!("cfg06/pass{pass}"));
        }
    }
}

/// CONFIGS row 7 — `driver(0)`: the wrapper must apply `run` exactly twice
/// (8 printed lines, `floors` +2, `bathrooms` +2.0).
#[test]
fn cfg_07_driver_zero_delta() {
    let mut h = harness();

    let before = h.probe_bedrooms();
    let (c_out, r_out) = h.call_both(Entry::Driver, 0);
    assert_eq!(
        c_out,
        r_out,
        "\ndriver(0) divergence\n  C:    {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );

    let lines: Vec<&str> = std::str::from_utf8(&c_out)
        .expect("utf8")
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(lines.len(), 8, "driver must print 8 lines (2 x run)");

    let first = Harness::parse_last_state(lines[0].as_bytes());
    let last = Harness::parse_last_state(lines[7].as_bytes());
    assert_eq!(last.0 - first.0, 2, "driver(0) must add exactly 2 floors");
    assert_eq!(last.1, before, "driver(0) must not change bedrooms");

    for i in 0..50 {
        h.assert_same(Entry::Driver, 0, &format!("cfg07/iter{i}"));
    }
}

/// CONFIGS row 8 — `driver`, random small deltas of either sign.
#[test]
fn cfg_08_driver_small_random() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_0008_5EED_0004);
    for i in 0..300 {
        let arg = rng.range_i32(-5000, 5000);
        h.assert_same(Entry::Driver, arg, &format!("cfg08/iter{i}"));
    }
}

/// CONFIGS row 9 — `driver`, uniform over the FULL `i32` domain: the delta is
/// applied twice per call, so this exercises double wrapping.
#[test]
fn cfg_09_driver_full_i32_random() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_0009_5EED_0005);
    for i in 0..400 {
        let arg = rng.next_i32();
        h.assert_same(Entry::Driver, arg, &format!("cfg09/iter{i}"));
    }
}

/// CONFIGS row 10 — `driver` over the exhaustive boundary argument set.
#[test]
fn cfg_10_driver_boundary_set() {
    let mut h = harness();
    for pass in 0..4 {
        for &arg in BOUNDARY_ARGS {
            h.assert_same(Entry::Driver, arg, &format!("cfg10/pass{pass}"));
        }
    }
}

/// CONFIGS row 11 — interleaved `run` / `driver` with random full-range
/// arguments. This is the composed-pipeline row: mixing the two entry points
/// makes the total number of `run` applications land on both odd and even
/// counts, which neither per-entry-point test can reach.
#[test]
fn cfg_11_interleaved_run_and_driver_random() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_000B_5EED_0006);
    for i in 0..500 {
        let entry = if rng.bool() { Entry::Run } else { Entry::Driver };
        let arg = rng.next_i32();
        h.assert_same(entry, arg, &format!("cfg11/step{i}"));
    }
}

/// CONFIGS row 12 — long homogeneous sequence so `printf` field widths grow in
/// two columns at once: `floors` crosses 1 -> 2 -> 3 -> 4 digits and
/// `bathrooms` crosses `9.5 -> 10.5`, `99.5 -> 100.5`, `999.5 -> 1000.5`.
#[test]
fn cfg_12_long_sequence_field_width_growth() {
    let mut h = harness();
    let mut saw_4_digit_floors = false;
    let mut saw_4_digit_bathrooms = false;

    for i in 0..1200 {
        let (c_out, r_out) = h.call_both(Entry::Run, 0);
        assert_eq!(
            c_out,
            r_out,
            "\ncfg12/iter{i} divergence\n  C:    {:?}\n  Rust: {:?}\n",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        let (floors, _, bathrooms) = Harness::parse_last_state(&c_out);
        if floors >= 1000 {
            saw_4_digit_floors = true;
        }
        // "1000.5" -> 4 integer digits
        if bathrooms.split('.').next().map(|s| s.len()) >= Some(4) {
            saw_4_digit_bathrooms = true;
        }
    }

    assert!(
        saw_4_digit_floors,
        "cfg12 did not reach 4-digit floors; the field-width edge was not exercised"
    );
    assert!(
        saw_4_digit_bathrooms,
        "cfg12 did not reach 4-integer-digit bathrooms; the %.1f width edge was not exercised"
    );
}

/// CONFIGS row 13 — walk `bedrooms` to exactly `i32::MAX`, then step across the
/// overflow boundary with `+1`, `+2` and random positive deltas.
#[test]
fn cfg_13_walk_to_max_then_cross() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_000D_5EED_0007);

    for &step in &[1i32, 2, 7] {
        h.set_bedrooms(i32::MAX, "cfg13");
        h.assert_same(Entry::Run, step, &format!("cfg13/cross-by-{step}"));
        let after = h.probe_bedrooms();
        assert_eq!(
            after,
            i32::MAX.wrapping_add(step) as i64,
            "cfg13: crossing i32::MAX by {step} did not wrap as the C does"
        );
    }

    for i in 0..40 {
        h.set_bedrooms(i32::MAX, "cfg13");
        let step = rng.range_i32(1, i32::MAX);
        h.assert_same(Entry::Run, step, &format!("cfg13/rand{i}"));
    }
}

/// CONFIGS row 14 — walk `bedrooms` to exactly `i32::MIN`, then step across the
/// underflow boundary with `-1`, `-2` and random negative deltas.
#[test]
fn cfg_14_walk_to_min_then_cross() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_000E_5EED_0008);

    for &step in &[-1i32, -2, -9] {
        h.set_bedrooms(i32::MIN, "cfg14");
        h.assert_same(Entry::Run, step, &format!("cfg14/cross-by-{step}"));
        let after = h.probe_bedrooms();
        assert_eq!(
            after,
            i32::MIN.wrapping_add(step) as i64,
            "cfg14: crossing i32::MIN by {step} did not wrap as the C does"
        );
    }

    for i in 0..40 {
        h.set_bedrooms(i32::MIN, "cfg14");
        let step = rng.range_i32(i32::MIN, -1);
        h.assert_same(Entry::Run, step, &format!("cfg14/rand{i}"));
    }
}

/// CONFIGS row 15 — sign transition of the `bedrooms` `%d` column: land on
/// exactly `0`, `-1`, `1`, and step across zero in both directions.
#[test]
fn cfg_15_bedrooms_sign_transition() {
    let mut h = harness();

    for &target in &[0i32, -1, 1, 10, -10] {
        h.set_bedrooms(target, "cfg15");
        let (c_out, r_out) = h.call_both(Entry::Run, 0);
        assert_eq!(
            c_out,
            r_out,
            "\ncfg15 divergence at bedrooms={target}\n  C:    {:?}\n  Rust: {:?}\n",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        assert_eq!(Harness::parse_last_state(&c_out).1, target as i64);
    }

    // Step across zero from both sides.
    h.set_bedrooms(3, "cfg15");
    for &step in &[-1i32, -1, -1, -1, -1, -1] {
        h.assert_same(Entry::Run, step, "cfg15/down-through-zero");
    }
    h.set_bedrooms(-3, "cfg15");
    for &step in &[1i32, 1, 1, 1, 1, 1] {
        h.assert_same(Entry::Run, step, "cfg15/up-through-zero");
    }
}

/// CONFIGS row 16 — the same argument replayed many times consecutively through
/// both entry points, for several random `k`. Confirms identical *accumulation*
/// rather than only identical single-shot output.
#[test]
fn cfg_16_repeated_same_argument_accumulation() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_0010_5EED_0009);

    for round in 0..6 {
        let k = rng.next_i32();
        for i in 0..50 {
            h.assert_same(Entry::Run, k, &format!("cfg16/round{round}/run{i}"));
        }
        for i in 0..50 {
            h.assert_same(Entry::Driver, k, &format!("cfg16/round{round}/driver{i}"));
        }
    }
}

/// CONFIGS row 17 — high-volume mixed soak over the full `i32` domain with a
/// fixed seed. Every step is compared.
#[test]
fn cfg_17_soak_mixed_random() {
    let mut h = harness();
    let mut rng = Rng::new(0x0000_0011_5EED_000A);

    for i in 0..3000 {
        let entry = if rng.next_u64().is_multiple_of(3) {
            Entry::Driver
        } else {
            Entry::Run
        };
        // Mix magnitudes: mostly full-range, sometimes tiny, sometimes extreme.
        let arg = match rng.next_u64() % 8 {
            0 => 0,
            1 => rng.range_i32(-3, 3),
            2 => i32::MAX,
            3 => i32::MIN,
            _ => rng.next_i32(),
        };
        h.assert_same(entry, arg, &format!("cfg17/step{i}"));
    }
}
