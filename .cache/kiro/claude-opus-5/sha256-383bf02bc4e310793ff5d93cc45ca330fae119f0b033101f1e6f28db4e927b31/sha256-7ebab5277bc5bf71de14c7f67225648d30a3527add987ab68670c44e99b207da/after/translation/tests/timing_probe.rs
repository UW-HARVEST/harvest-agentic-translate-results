//! Timing probe: `long_exec` calls `perform_expensive_operations` 2000 times,
//! so this reports the projected cost of a full `long_exec` run per side.
//! Ignored by default; run with `--ignored --nocapture`.

mod common;

use common::load_both;
use std::time::Instant;

#[test]
#[ignore]
fn probe_cost_of_one_pass() {
    let guard = load_both();
    let (c, rust) = &*guard;

    for imp in [c, rust] {
        // Warm the pages, then time three passes.
        imp.perform_expensive_operations();
        let start = Instant::now();
        const PASSES: u32 = 3;
        for _ in 0..PASSES {
            imp.perform_expensive_operations();
        }
        let per_pass = start.elapsed() / PASSES;
        println!(
            "{:>4}: {:?} per pass  =>  projected long_exec (2000 passes): {:?}",
            imp.name(),
            per_pass,
            per_pass * 2000
        );
    }
}
