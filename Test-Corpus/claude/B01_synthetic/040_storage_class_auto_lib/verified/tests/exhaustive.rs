// EXHAUSTIVE differential verification over the complete input domain.
//
// `driver`'s only parameter is an `int`, so the entire input domain is the 2^32
// values of `i32` — small enough to verify *completely* instead of by sampling.
// (A sampled sweep cannot rule out a translation that differs for a single
// value; this can.  Verified: a mutant that is wrong only for x == 1234567891
// survives a 4.2 M-sample sweep and is caught here.)
//
// Both libraries are driven through their `.so` exports; their stdout is piped
// to a hashing reader thread, so a full pass needs no disk space and no memory
// proportional to the ~45 GiB of text each implementation emits.
//
// POLLUTION HANDLING.  fd 1 is process-global, and after 60 s libtest's main
// thread prints "test <name> has been running for over 60 seconds" (exactly 68
// bytes) to stdout, which lands inside whichever capture is open.  Every capture
// is therefore checked against the byte length the C semantics require; a
// mismatch means the capture was polluted (or truncated), and the chunk is
// retried.  A chunk that keeps failing is BISECTED down to a single input, and
// only a per-value divergence that reproduces 5 times in a row is reported as a
// real translation bug.
//
// Marked `#[ignore]` so `cargo test` stays fast.  Run a shard with:
//
//   SHARD_INDEX=0 SHARD_COUNT=16 cargo test --release --test exhaustive \
//       -- --ignored --nocapture
//
// or every shard via ./exhaustive_sweep.sh

mod common;

use common::{capture_stdout, fnv1a, hash_stdout, impls, Impls};

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Number of bytes `printf("%d\n", y)` writes, computed without formatting.
fn printed_len(y: i32) -> u64 {
    let mut n = 2; // one digit + '\n'
    let mut m = if y < 0 {
        n += 1; // '-'
        (y as i64).unsigned_abs()
    } else {
        y as u64
    };
    while m >= 10 {
        m /= 10;
        n += 1;
    }
    n
}

fn expected_line(x: i32) -> String {
    format!("{}\n", x.wrapping_mul(2).wrapping_add(300))
}

/// Total bytes the C library must emit for `x in [lo, lo + n)`.
fn expected_bytes(lo: i32, n: u32) -> u64 {
    let mut total = 0u64;
    for k in 0..n {
        let x = lo.wrapping_add(k as i32);
        total += printed_len(x.wrapping_mul(2).wrapping_add(300));
    }
    total
}

/// One differential attempt over `x in [lo, lo + n)`.
/// `Ok(())` = both libraries emitted exactly the expected number of bytes and
/// identical content.  `Err(reason)` distinguishes pollution from divergence.
fn attempt(im: &Impls, lo: i32, n: u32) -> Result<(), String> {
    let want = expected_bytes(lo, n);

    let (h_c, n_c) = hash_stdout(|| {
        for k in 0..n {
            (im.c.driver)(lo.wrapping_add(k as i32));
        }
    });
    let (h_r, n_r) = hash_stdout(|| {
        for k in 0..n {
            (im.rust.driver)(lo.wrapping_add(k as i32));
        }
    });

    if n_c != want {
        return Err(format!(
            "C capture length {n_c} != required {want} (capture polluted/truncated)"
        ));
    }
    if n_r != want {
        return Err(format!(
            "Rust capture length {n_r} != required {want} (capture polluted/truncated)"
        ));
    }
    if h_c != h_r {
        return Err(format!("content differs: C {h_c:#018x} vs Rust {h_r:#018x}"));
    }
    Ok(())
}

/// Retries `attempt` up to `tries` times; `Ok` as soon as one attempt is clean.
fn verify(im: &Impls, lo: i32, n: u32, tries: u32, retries: &mut u64) -> Result<(), String> {
    let mut last = String::new();
    for t in 0..tries {
        match attempt(im, lo, n) {
            Ok(()) => return Ok(()),
            Err(e) => {
                if t + 1 < tries {
                    *retries += 1;
                }
                last = e;
            }
        }
    }
    Err(last)
}

/// Exact per-value check, repeated: `true` only if C and Rust never agree on a
/// clean (pollution-free) capture — i.e. a real, reproducible divergence.
fn value_diverges(im: &Impls, x: i32) -> bool {
    let want = expected_line(x);
    for _ in 0..5 {
        let out_c = capture_stdout(|| (im.c.driver)(x));
        let out_r = capture_stdout(|| (im.rust.driver)(x));
        if out_c == out_r && out_c.as_slice() == want.as_bytes() {
            return false;
        }
        // C is ground truth: if C alone matches `want` and Rust does not, that
        // is a divergence — but only report it after all repetitions failed.
    }
    true
}

/// Narrows a persistently failing range to the offending input.
fn bisect(im: &Impls, lo: i32, n: u32, retries: &mut u64) -> Option<i32> {
    if n <= 64 {
        for k in 0..n {
            let x = lo.wrapping_add(k as i32);
            if value_diverges(im, x) {
                return Some(x);
            }
        }
        return None;
    }
    let half = n / 2;
    if verify(im, lo, half, 3, retries).is_err() {
        if let Some(x) = bisect(im, lo, half, retries) {
            return Some(x);
        }
    }
    let mid = lo.wrapping_add(half as i32);
    if verify(im, mid, n - half, 3, retries).is_err() {
        if let Some(x) = bisect(im, mid, n - half, retries) {
            return Some(x);
        }
    }
    None
}

#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly (see exhaustive_sweep.sh)"]
fn exhaustive_all_i32_inputs() {
    let im = impls();

    let shard_count = env_u64("SHARD_COUNT", 1).max(1);
    let shard_index = env_u64("SHARD_INDEX", 0);
    assert!(shard_index < shard_count, "SHARD_INDEX out of range");
    let chunk_len = env_u64("CHUNK_LEN", 1 << 22).max(1) as i64;

    const DOMAIN: i64 = 1 << 32;
    let per_shard = DOMAIN / shard_count as i64;
    let start = i32::MIN as i64 + per_shard * shard_index as i64;
    let end = if shard_index as i64 == shard_count as i64 - 1 {
        i32::MAX as i64 + 1
    } else {
        start + per_shard
    };

    eprintln!(
        "shard {}/{}: x in [{}, {}) = {} values, chunk = {}",
        shard_index,
        shard_count,
        start,
        end,
        end - start,
        chunk_len
    );

    // Anti-vacuity: prove the capture path really observes the library output by
    // checking exact bytes (not just a hash) at the start of the shard.
    {
        let lo = start as i32;
        let prefix: u32 = 4096;
        let expected: String = (0..prefix)
            .map(|k| expected_line(lo.wrapping_add(k as i32)))
            .collect();
        let mut ok = false;
        for _ in 0..3 {
            let (h, len) = hash_stdout(|| {
                for k in 0..prefix {
                    (im.c.driver)(lo.wrapping_add(k as i32));
                }
            });
            if (h, len) == (fnv1a(expected.as_bytes()), expected.len() as u64) {
                ok = true;
                break;
            }
        }
        assert!(
            ok,
            "the hashing capture does not reproduce the expected C text — the \
             harness is not observing the library output"
        );
    }

    let t0 = std::time::Instant::now();
    let mut done: i64 = 0;
    let mut retries: u64 = 0;
    let mut polluted_chunks: u64 = 0;
    let mut chunk_start = start;

    while chunk_start < end {
        let chunk_end = (chunk_start + chunk_len).min(end);
        let lo = chunk_start as i32;
        let n = (chunk_end - chunk_start) as u32;

        if let Err(e) = verify(im, lo, n, 4, &mut retries) {
            polluted_chunks += 1;
            eprintln!(
                "  x in [{chunk_start}, {chunk_end}): 4 attempts failed ({e}); bisecting"
            );
            if let Some(x) = bisect(im, lo, n, &mut retries) {
                let out_c = capture_stdout(|| (im.c.driver)(x));
                let out_r = capture_stdout(|| (im.rust.driver)(x));
                panic!(
                    "REPRODUCIBLE DIVERGENCE at x = {x} (0x{:08x}):\n  C    = {:?}\n  Rust = {:?}\n  expected (from C semantics) = {:?}",
                    x as u32,
                    String::from_utf8_lossy(&out_c),
                    String::from_utf8_lossy(&out_r),
                    expected_line(x)
                );
            }
            eprintln!("  bisection found no per-value divergence -> environmental pollution; re-verifying chunk");
            verify(im, lo, n, 8, &mut retries).unwrap_or_else(|e| {
                panic!("x in [{chunk_start}, {chunk_end}) remains unstable after bisection: {e}")
            });
        }

        done += (chunk_end - chunk_start) as i64;
        chunk_start = chunk_end;
        if done % (1 << 26) == 0 {
            let secs = t0.elapsed().as_secs_f64();
            eprintln!(
                "  {:>12} / {:>12} values ({:5.1}%), {:.0}s, {:.2} M values/s, {retries} retries",
                done,
                end - start,
                100.0 * done as f64 / (end - start) as f64,
                secs,
                done as f64 / secs / 1e6
            );
        }
    }

    assert_eq!(done, end - start, "shard did not cover its whole range");
    eprintln!(
        "SHARD {}/{} OK: {} values byte-identical in {:.0}s ({} capture retries, {} polluted chunks)",
        shard_index,
        shard_count,
        done,
        t0.elapsed().as_secs_f64(),
        retries,
        polluted_chunks
    );
}
