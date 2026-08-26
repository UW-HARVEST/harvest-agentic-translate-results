//! Concurrency differential test.
//!
//! `my_pow` reads `errno`, which is thread-local. The C reaches it through
//! `__errno_location()`, and so must the Rust: a translation that cached the
//! pointer in a global, or used a process-wide variable, would work perfectly in
//! a single-threaded test and corrupt results under concurrency. That bug class
//! is invisible to every other test in this suite, so it gets its own binary
//! (one test per process, so the fd-2 redirection cannot race).
//!
//! Only (return bits, errno) are compared here — stderr is a single shared
//! `FILE*`, so interleaved messages from many threads are not attributable and
//! are simply discarded to /dev/null.

mod common;

use common::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn concurrent_calls_agree_and_do_not_leak_errno_across_threads() {
    let im = impls();
    // Error messages would otherwise flood the terminal from 8 threads.
    silence_stderr_forever();

    const THREADS: usize = 8;
    const ITERS: usize = 4000;

    let edom = Arc::new(AtomicUsize::new(0));
    let erange = Arc::new(AtomicUsize::new(0));
    let clean = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for t in 0..THREADS {
        let edom = Arc::clone(&edom);
        let erange = Arc::clone(&erange);
        let clean = Arc::clone(&clean);
        handles.push(std::thread::spawn(move || {
            // Each thread uses a different seed but a mix that guarantees it
            // hits all three branches, so the threads are constantly changing
            // each other's errno if the TLS slot is not respected.
            let mut rng = Rng::new(0x7000 + t as u64);
            for i in 0..ITERS {
                let (base, exp) = match i % 4 {
                    // EDOM
                    0 => (-rng.range(0.1, 100.0), rng.range(0.1, 9.0) + 0.5),
                    // ERANGE overflow
                    1 => (rng.range(2.0, 100.0), rng.range(500.0, 5000.0)),
                    // ERANGE underflow / pole
                    2 => {
                        if rng.bool() {
                            (0.0, -rng.range(1.0, 100.0))
                        } else {
                            (rng.range(2.0, 100.0), -rng.range(500.0, 5000.0))
                        }
                    }
                    // clean
                    _ => (rng.range(1.1, 10.0), rng.range(-5.0, 5.0)),
                };

                let (c_bits, c_errno) = call_raw(im.c, base, exp);
                let (r_bits, r_errno) = call_raw(im.rust, base, exp);
                note_comparison();

                assert_eq!(
                    (c_bits, c_errno),
                    (r_bits, r_errno),
                    "thread {t} iter {i}: divergence for base={base:?} (0x{:016X}) \
                     exp={exp:?} (0x{:016X}): C=(0x{c_bits:016X}, errno {c_errno}) \
                     Rust=(0x{r_bits:016X}, errno {r_errno})",
                    base.to_bits(),
                    exp.to_bits()
                );

                match c_errno {
                    e if e == EDOM => edom.fetch_add(1, Ordering::Relaxed),
                    e if e == ERANGE => erange.fetch_add(1, Ordering::Relaxed),
                    _ => clean.fetch_add(1, Ordering::Relaxed),
                };
            }
        }));
    }

    for (i, h) in handles.into_iter().enumerate() {
        h.join().unwrap_or_else(|_| panic!("thread {i} panicked"));
    }

    let (d, r, c) = (
        edom.load(Ordering::Relaxed),
        erange.load(Ordering::Relaxed),
        clean.load(Ordering::Relaxed),
    );
    assert_eq!(d + r + c, THREADS * ITERS);
    // All three branches must have been exercised concurrently, or the test
    // proves nothing about cross-thread errno interference.
    assert!(d > 0, "no EDOM results under concurrency");
    assert!(r > 0, "no ERANGE results under concurrency");
    assert!(c > 0, "no clean results under concurrency");
    // stderr is pointed at /dev/null by this test, so report on stdout.
    println!("concurrency branch coverage: EDOM={d} ERANGE={r} clean={c}");
}
