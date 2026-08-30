// CONFIGS.md row C11 — the very first `driver` call after `dlopen`.
//
// This lives in its own test binary so that the call really is the first one
// each library sees; the C build initialises its file-scope `static int y`
// to 123 and the Rust build initialises `static Y: AtomicI32` to 123, and
// this test pins that the initialiser is equally unobservable in both
// (because `driver` overwrites `y` from its 2nd argument before reading it).

use crate::common::*;
use crate::Case;

fn c11_first_call_after_load_is_not_affected_by_the_static_initialiser() {
    // First-ever call, and it is the success path: if either build read the
    // 123 initialiser instead of `local_y`, this would report `y != 2`.
    assert_same_and_eq(1, 2, 3, "Ok!\nResult: 0\n");

    // Second call: y = 123 explicitly, which must now be *rejected*, proving
    // 123 has no special meaning after initialisation either.
    assert_same_and_eq(
        1,
        123,
        3,
        "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n",
    );

    // Third call: back to success, so the write in call 2 did not stick.
    assert_same_and_eq(1, 2, 3, "Ok!\nResult: 0\n");
}

/// Registry of this module's cases, in execution order.
pub fn cases() -> Vec<Case> {
    vec![
        ("c11_first_call_after_load_is_not_affected_by_the_static_initialiser", c11_first_call_after_load_is_not_affected_by_the_static_initialiser as fn()),
    ]
}
