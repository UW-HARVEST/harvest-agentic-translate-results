//! Lowest-level exported function: `void run(int extra_bedrooms)`.
//!
//! Verifies the very first call (from the pristine initial state
//! `{floors=2, bedrooms=5, bathrooms=2.5}`) and then a long accumulating
//! sequence covering ordinary, zero, negative and extremal `int` arguments.

mod harness;
use harness::*;

#[test]
fn run_matches_c() {
    let (c, rust) = load_pair();

    // First call from the initial state. Also pinned against the literal text
    // the C `printf` format string produces, so a mismatch tells us which side
    // drifted.
    let c_first = c.run(0);
    let r_first = rust.run(0);
    assert_same("run(0) [first call]", &c_first, &r_first);
    assert_eq!(
        String::from_utf8_lossy(&c_first),
        "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n",
        "C reference output changed unexpectedly"
    );

    // Accumulating sequence. Both sides see the same argument in the same
    // order, so their globals stay in lockstep.
    let args: [i32; 24] = [
        0,
        1,
        2,
        7,
        -1,
        -3,
        100,
        -100,
        1_000_000,
        -1_000_000,
        i32::MAX,
        1,
        i32::MAX,
        i32::MIN,
        -1,
        i32::MIN,
        0,
        i16::MAX as i32,
        i16::MIN as i32,
        1 << 30,
        -(1 << 30),
        3,
        -7,
        42,
    ];

    for (i, &a) in args.iter().enumerate() {
        let c_out = c.run(a);
        let r_out = rust.run(a);
        assert_same(&format!("run({a}) [step {i}]"), &c_out, &r_out);
        assert!(!c_out.is_empty(), "run printed nothing at step {i}");
    }
}
