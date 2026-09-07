//! Phase B — pristine-state anchor (CONFIGS.md row C1).
//!
//! This is deliberately the ONLY test in its own test binary, so the two
//! libraries are freshly loaded and `the_house` is still at its initializer
//! `{.floors = 2, .bedrooms = 5, .bathrooms = 2.5}`. It pins the absolute
//! expected output so the rest of the differential suite cannot pass vacuously
//! (e.g. by both libraries printing nothing, or by both starting from the same
//! *wrong* initial state — the C bytes are ground truth here).

mod harness;

use harness::*;

#[test]
fn cfg_c1_initial_state_and_run_zero() {
    // First observation of either library in this process.
    let out = assert_same_one("C1 run(0) from pristine state", run_op(0));
    assert_eq!(
        String::from_utf8_lossy(&out),
        "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n",
        "initial state / run(0) output must match the C ground truth exactly"
    );

    // Continuing from that state: floors 3->4, bathrooms 3.5->4.5, bedrooms +7.
    let out = assert_same_one("C1 run(7) continuing", run_op(7));
    assert_eq!(
        String::from_utf8_lossy(&out),
        "The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n\
         The house has 4 floors, 5 bedrooms, and 3.5 bathrooms\n\
         The house has 4 floors, 5 bedrooms, and 4.5 bathrooms\n\
         The house has 4 floors, 12 bedrooms, and 4.5 bathrooms\n"
    );

    // A driver() call runs run() twice: 8 lines, bedrooms +3 twice.
    let out = assert_same_one("C1 driver(\"3\") continuing", drv("3"));
    assert_eq!(
        String::from_utf8_lossy(&out),
        "The house has 4 floors, 12 bedrooms, and 4.5 bathrooms\n\
         The house has 5 floors, 12 bedrooms, and 4.5 bathrooms\n\
         The house has 5 floors, 12 bedrooms, and 5.5 bathrooms\n\
         The house has 5 floors, 15 bedrooms, and 5.5 bathrooms\n\
         The house has 5 floors, 15 bedrooms, and 5.5 bathrooms\n\
         The house has 6 floors, 15 bedrooms, and 5.5 bathrooms\n\
         The house has 6 floors, 15 bedrooms, and 6.5 bathrooms\n\
         The house has 6 floors, 18 bedrooms, and 6.5 bathrooms\n"
    );

    // A rejected driver() call prints exactly the sentinel and mutates nothing.
    let out = assert_same_one("C1 driver(\"nope\") rejected", drv("nope"));
    assert_eq!(String::from_utf8_lossy(&out), "An error occurred\n");

    // ...proven by the next call resuming from the untouched state.
    let out = assert_same_one("C1 run(0) after rejection", run_op(0));
    assert_eq!(
        String::from_utf8_lossy(&out),
        "The house has 6 floors, 18 bedrooms, and 6.5 bathrooms\n\
         The house has 7 floors, 18 bedrooms, and 6.5 bathrooms\n\
         The house has 7 floors, 18 bedrooms, and 7.5 bathrooms\n\
         The house has 7 floors, 18 bedrooms, and 7.5 bathrooms\n"
    );
}
