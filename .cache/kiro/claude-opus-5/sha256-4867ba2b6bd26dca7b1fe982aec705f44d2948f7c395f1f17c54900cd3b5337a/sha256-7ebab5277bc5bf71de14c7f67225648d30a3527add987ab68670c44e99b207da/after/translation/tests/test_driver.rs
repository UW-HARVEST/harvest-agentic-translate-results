//! Top-level exported function: `void driver(int x)`, which calls `run(x)` twice.

mod harness;
use harness::*;

#[test]
fn driver_matches_c() {
    let (c, rust) = load_pair();

    // First call from the pristine initial state: 8 lines total (two `run`s).
    let c_first = c.driver(1);
    let r_first = rust.driver(1);
    assert_same("driver(1) [first call]", &c_first, &r_first);
    assert_eq!(
        String::from_utf8_lossy(&c_first),
        "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n\
         The house has 3 floors, 6 bedrooms, and 3.5 bathrooms\n\
         The house has 3 floors, 6 bedrooms, and 3.5 bathrooms\n\
         The house has 4 floors, 6 bedrooms, and 3.5 bathrooms\n\
         The house has 4 floors, 6 bedrooms, and 4.5 bathrooms\n\
         The house has 4 floors, 7 bedrooms, and 4.5 bathrooms\n",
        "C reference output changed unexpectedly"
    );
    assert_eq!(
        c_first.iter().filter(|&&b| b == b'\n').count(),
        8,
        "driver should print 8 lines per call"
    );

    let args: [i32; 20] = [
        0,
        1,
        -1,
        5,
        -5,
        123,
        -123,
        65_535,
        -65_536,
        i32::MAX,
        i32::MIN,
        i32::MAX / 2,
        i32::MIN / 2,
        1 << 29,
        -(1 << 29),
        7,
        -7,
        999_999_999,
        -999_999_999,
        2,
    ];

    for (i, &a) in args.iter().enumerate() {
        let c_out = c.driver(a);
        let r_out = rust.driver(a);
        assert_same(&format!("driver({a}) [step {i}]"), &c_out, &r_out);
    }
}
