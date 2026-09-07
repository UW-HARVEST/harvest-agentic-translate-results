//! CONFIGS.md C9 / C13 and the empirical half of ERRORS.md E9.
//!
//! `j += 2` signed-overflows `int` once `i` reaches 2^30. These runs execute
//! that region for real, streaming both implementations' output through a pipe
//! and comparing FNV-1a digests plus byte/line counts, because materialising
//! ~24 GiB per side is not practical.
//!
//! They are `#[ignore]`d because each takes minutes; `./run_all.sh` runs them
//! explicitly, one cargo invocation per test so no single command exceeds the
//! 600 s budget.

mod common;

use common::{Digest, Impl, Libs};

fn digest_diff(label: &str, x: i32) {
    let libs = Libs::load();

    let t = std::time::Instant::now();
    let c: Digest = libs.run_digest(Impl::C, x);
    eprintln!(
        "[{label}] C    x={x} lines={} bytes={} hash={:#018x} in {:.1}s",
        c.lines,
        c.bytes,
        c.hash,
        t.elapsed().as_secs_f64()
    );

    let t = std::time::Instant::now();
    let r: Digest = libs.run_digest(Impl::Rust, x);
    eprintln!(
        "[{label}] Rust x={x} lines={} bytes={} hash={:#018x} in {:.1}s",
        r.lines,
        r.bytes,
        r.hash,
        t.elapsed().as_secs_f64()
    );

    assert_eq!(
        c.lines, r.lines,
        "[{label}] line count differs for x={x}: C={} Rust={}",
        c.lines, r.lines
    );
    assert_eq!(
        c.bytes, r.bytes,
        "[{label}] byte count differs for x={x}: C={} Rust={}",
        c.bytes, r.bytes
    );
    assert_eq!(
        c.head,
        r.head,
        "[{label}] first bytes differ for x={x}\nC   : {}\nRust: {}",
        String::from_utf8_lossy(&c.head),
        String::from_utf8_lossy(&r.head)
    );
    assert_eq!(
        c.tail,
        r.tail,
        "[{label}] last bytes differ for x={x}\nC   : {}\nRust: {}",
        String::from_utf8_lossy(&c.tail),
        String::from_utf8_lossy(&r.tail)
    );
    assert_eq!(
        c.hash, r.hash,
        "[{label}] stream digest differs for x={x}: C={:#018x} Rust={:#018x}",
        c.hash, r.hash
    );
    assert_eq!(c.lines, x as u64, "[{label}] expected exactly x lines");
}

/// Digest reference computed independently of both libraries, so a run that
/// captured nothing on both sides cannot pass.
fn model_digest(x: i32) -> Digest {
    let mut all = Vec::new();
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        all.extend_from_slice(format!("{i} {j}\n").as_bytes());
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
    common::digest_of(&all)
}

/// The final line `driver(x)` must emit, computed with wrapping arithmetic.
fn model_last_line(x: i32) -> String {
    assert!(x > 0);
    let i = x - 1;
    let j = (i as i32).wrapping_mul(2);
    format!("{i} {j}")
}

/// C9 (approach run) — just below the overflow point, cheap enough to also
/// cross-check against an independently computed digest.
#[test]
#[ignore = "long running (~1 min); run via ./run_all.sh"]
fn c9a_just_below_overflow_model_checked() {
    // 5 million lines: verifies the digest pipeline itself against a model.
    let x = 5_000_000i32;
    let libs = Libs::load();
    let c = libs.run_digest(Impl::C, x);
    let r = libs.run_digest(Impl::Rust, x);
    let m = model_digest(x);
    assert_eq!(c, r, "[C9a] C vs Rust digest differs for x={x}");
    assert_eq!(c.hash, m.hash, "[C9a] C digest differs from the model");
    assert_eq!(c.bytes, m.bytes);
    assert_eq!(c.lines, m.lines);
}

/// C9 — `x == 2^30`: `j` reaches 2147483646, the largest even value that still
/// fits, without overflowing.
#[test]
#[ignore = "long running (~3 min); run via ./run_all.sh"]
fn c9b_exactly_at_overflow_boundary() {
    digest_diff("C9b", 1 << 30);
}

/// C9 — `x == 2^30 + 5`: `j` wraps from 2147483646 to -2147483648 and keeps
/// climbing, so the emitted lines gain a `-` sign. This is the actual overflow
/// region.
#[test]
#[ignore = "long running (~3 min); run via ./run_all.sh"]
fn c9c_past_overflow_boundary() {
    let x = (1 << 30) + 5;
    digest_diff("C9c", x);

    // Pin down what the wrapped tail actually looks like, so a future
    // regression that silently drops the sign is caught by value, not just by
    // digest equality.
    let libs = Libs::load();
    let c = libs.run_digest(Impl::C, x);
    let tail = String::from_utf8_lossy(&c.tail).to_string();
    eprintln!("[C9c] C tail:\n{tail}");
    assert!(
        tail.contains("-2147483648") || tail.contains('-'),
        "[C9c] expected a wrapped (negative) j in the tail, got:\n{tail}"
    );
    let last = tail.lines().last().unwrap_or("").to_string();
    assert_eq!(
        last,
        model_last_line(x),
        "[C9c] unexpected final line for x={x}"
    );
}

/// C13 / E9 — the full `x == i32::MAX` run: 2147483647 iterations, `j` wrapping
/// past the halfway point.
#[test]
#[ignore = "very long running (~6 min); run via ./run_all.sh"]
fn c13_int_max_full_run() {
    digest_diff("C13", i32::MAX);
}
