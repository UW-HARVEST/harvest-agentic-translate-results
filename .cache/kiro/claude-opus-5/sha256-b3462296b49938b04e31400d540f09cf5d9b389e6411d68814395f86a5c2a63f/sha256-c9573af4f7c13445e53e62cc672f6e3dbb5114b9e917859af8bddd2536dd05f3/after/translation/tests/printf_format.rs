//! ERRORS.md row 15 / CONFIGS.md row 12 — `printf` field-width behaviour.
//!
//! This lives in its own test binary so that it observes the persistent global
//! state from the beginning: each integration test file is a separate process
//! with its own `dlopen`ed pair of libraries, so nothing else has advanced
//! `the_house` here. That is required to watch the `%.1f` field width grow from
//! its pristine 3-character form (`2.5`) all the way to 6 characters
//! (`1000.5`).
//!
//! Beyond the width edges, this pins that the Rust side really routes the
//! `double` through the C `printf` with a `%.1f` conversion, rather than through
//! Rust's own float formatting — a plausible translation shortcut that would
//! diverge.

mod harness;

use harness::{harness, Entry, Harness};

#[test]
fn err_15_bathrooms_width_growth() {
    let mut h = harness();

    // Confirm we really are starting from (near-)pristine state: the harness
    // constructor performed exactly one `run(0)`, so bathrooms is 3.5 and
    // floors is 3.
    let (c0, r0) = h.pristine_run0();
    assert_eq!(c0, r0, "pristine outputs already diverge");
    let (floors0, _, bathrooms0) = Harness::parse_last_state(c0);
    assert_eq!(floors0, 3, "unexpected starting floors");
    assert_eq!(bathrooms0, "3.5", "unexpected starting bathrooms");

    let mut bathroom_widths = std::collections::BTreeSet::new();
    let mut floor_widths = std::collections::BTreeSet::new();
    let mut seen_values = Vec::new();

    for i in 0..1100 {
        let (c_out, r_out) = h.call_both(Entry::Run, 0);
        assert_eq!(
            c_out,
            r_out,
            "\nerr15/iter{i} divergence\n  C:    {:?}\n  Rust: {:?}\n",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        let (floors, _, bathrooms) = Harness::parse_last_state(&c_out);
        assert!(
            bathrooms.ends_with(".5"),
            "err15: bathrooms must always be an exactly representable n.5, got {bathrooms:?}"
        );
        bathroom_widths.insert(bathrooms.len());
        floor_widths.insert(floors.to_string().len());
        if i < 12 {
            seen_values.push(bathrooms);
        }
    }

    // 3.5 -> "3.5" (3), 10.5 -> "10.5" (4), 100.5 -> "100.5" (5),
    // 1000.5 -> "1000.5" (6)
    assert!(
        bathroom_widths.contains(&3)
            && bathroom_widths.contains(&4)
            && bathroom_widths.contains(&5)
            && bathroom_widths.contains(&6),
        "err15 did not cross all the %.1f width boundaries (saw widths {bathroom_widths:?})"
    );
    // floors: 4 -> "4" (1), 10 (2), 100 (3), 1000 (4)
    assert!(
        floor_widths.contains(&1)
            && floor_widths.contains(&2)
            && floor_widths.contains(&3)
            && floor_widths.contains(&4),
        "err15 did not cross all the %d width boundaries for floors (saw {floor_widths:?})"
    );

    // Spot-check the first few values against what the C source implies.
    assert_eq!(
        seen_values,
        vec![
            "4.5", "5.5", "6.5", "7.5", "8.5", "9.5", "10.5", "11.5", "12.5", "13.5", "14.5",
            "15.5"
        ],
        "err15: bathrooms did not advance by exactly 1.0 per run"
    );
}
