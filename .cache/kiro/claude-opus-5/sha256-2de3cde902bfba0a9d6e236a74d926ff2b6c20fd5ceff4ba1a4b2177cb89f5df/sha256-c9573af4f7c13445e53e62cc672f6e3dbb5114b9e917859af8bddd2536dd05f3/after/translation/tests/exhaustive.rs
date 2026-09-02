//! Exhaustive differential sweep over the ENTIRE `i32` input domain.
//!
//! `get_predict_func` takes a single `int`, so the input space is finite and
//! small enough to enumerate completely. This removes all doubt that any
//! `pfcn` value exists for which C and Rust disagree.
//!
//! Marked `#[ignore]` because it takes ~1 minute; run it with:
//!
//! ```sh
//! cargo test --release --test exhaustive -- --ignored --nocapture
//! ```
//!
//! `run_all.sh` does not run it on every sweep, but it has been run and
//! passes (see VERIFICATION.md).

mod common;

use common::{c_source_oracle, Pair};

#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --ignored"]
fn exhaustive_all_i32_values() {
    let p = Pair::load();
    let c = p.c;
    let rust = p.rust;

    let mut checked: u64 = 0;
    let mut v: i32 = i32::MIN;
    loop {
        let cv = unsafe { c(v) };
        let rv = unsafe { rust(v) };
        if cv != rv {
            panic!("divergence at pfcn = {v} (0x{v:08x}): C = {cv}, Rust = {rv}");
        }
        let want = c_source_oracle(v);
        if cv != want {
            panic!("C itself disagrees with the source-derived oracle at pfcn = {v}: C = {cv}, oracle = {want}");
        }
        checked += 1;
        if v == i32::MAX {
            break;
        }
        v = v.wrapping_add(1);
    }
    assert_eq!(checked, 1u64 << 32, "must have visited every i32");
    println!("exhaustive: {checked} values checked, zero divergences");
}
