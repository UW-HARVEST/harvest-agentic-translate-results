//! Throughput probe: calibrates how large an `x` the overflow test can afford.
//! Not part of the verification gate; run with `--ignored`.

mod common;

use common::{Impl, Libs};

#[test]
#[ignore]
fn bench_digest_throughput() {
    let libs = Libs::load();
    for x in [1_000_000i32, 10_000_000, 100_000_000] {
        for imp in [Impl::C, Impl::Rust] {
            let t = std::time::Instant::now();
            let d = libs.run_digest(imp, x);
            let el = t.elapsed();
            eprintln!(
                "{:>4} x={:>12} lines={:>12} bytes={:>13} {:>8.3}s -> {:.1} Mline/s",
                imp.name(),
                x,
                d.lines,
                d.bytes,
                el.as_secs_f64(),
                d.lines as f64 / el.as_secs_f64() / 1e6
            );
        }
    }
}
