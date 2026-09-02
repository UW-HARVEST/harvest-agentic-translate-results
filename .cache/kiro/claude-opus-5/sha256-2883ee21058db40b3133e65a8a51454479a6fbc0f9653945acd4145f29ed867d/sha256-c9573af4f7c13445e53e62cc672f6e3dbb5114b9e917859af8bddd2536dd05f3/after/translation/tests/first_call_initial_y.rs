//! `CONFIGS.md` row 11, second half — the *first-ever* call in a fresh process
//! passing the static initialiser's own value (`123`) as `local_y`.
//!
//! Separate binary for the same reason as `first_call_success.rs`: there is only
//! one "first call" per process. This pins down the direction of the assignment
//! (`y = local_y`, not `local_y = y`) at the one moment the two would be
//! indistinguishable if the initialiser leaked.

mod common;

use common::{assert_same, expected};

#[test]
fn first_call_with_initialiser_value_still_takes_y_guard() {
    let out = assert_same(1, 123, 3);
    let s = String::from_utf8_lossy(&out);
    assert_eq!(
        s,
        format!(
            "{}{}{}",
            expected::ERR_Y,
            expected::FAILED,
            expected::result_line(2)
        ),
        "local_y == 123 must fail the y guard just like any other non-2 value"
    );
}
