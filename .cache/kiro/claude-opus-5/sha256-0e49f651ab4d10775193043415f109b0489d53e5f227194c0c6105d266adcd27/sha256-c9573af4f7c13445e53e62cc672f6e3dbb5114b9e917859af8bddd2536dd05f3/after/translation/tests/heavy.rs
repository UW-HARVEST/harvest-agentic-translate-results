//! Heavy differential tests: the two configuration rows whose outputs are far
//! too large to hold in memory (`CONFIGS.md` rows C10 and C11).
//!
//! Both still go through `libloading` and the exported `driver` symbol only.
//! Instead of buffering the output, a reader thread streams it and folds it into
//! an FNV-1a hash plus a byte count, so C and Rust can be compared over tens of
//! gigabytes without storing any of it.
//!
//! Runtime: roughly 3 minutes total. Run with
//! `cargo test --release --test heavy -- --nocapture`.

mod common;

use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::raw::c_int;

use common::pair;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn fork() -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

const STDOUT_FD: c_int = 1;
const SIGKILL: c_int = 9;

/// Streaming FNV-1a, matching the probe used during investigation.
#[derive(Debug, PartialEq, Eq)]
struct Digest {
    hash: u64,
    bytes: u64,
}

fn fold(d: &mut Digest, buf: &[u8]) {
    d.bytes += buf.len() as u64;
    let mut h = d.hash;
    for &b in buf {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    d.hash = h;
}

fn new_digest() -> Digest {
    Digest {
        hash: 1469598103934665603,
        bytes: 0,
    }
}

/// Run `driver(x)` to completion with fd 1 pointing at a pipe, streaming and
/// hashing everything it writes. Nothing is buffered, so output size is bounded
/// only by time.
fn digest_full_run(f: unsafe extern "C" fn(c_int), x: i32) -> Digest {
    let (mut rx, tx) = std::io::pipe().expect("pipe");

    let reader = std::thread::spawn(move || {
        let mut d = new_digest();
        let mut buf = vec![0u8; 1 << 20];
        loop {
            match rx.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => fold(&mut d, &buf[..n]),
            }
        }
        d
    });

    let _g = common::fd_lock().lock().unwrap();
    // SAFETY: fd 1 is saved and restored; the guard serialises the redirection.
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(STDOUT_FD);
        assert!(saved >= 0);
        assert!(dup2(tx.as_raw_fd(), STDOUT_FD) >= 0);
        f(x);
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FD) >= 0);
        close(saved);
    }
    drop(tx);
    drop(_g);

    reader.join().expect("reader thread")
}

/// Start `driver(x)` in a forked child writing to a pipe, hash exactly
/// `prefix_len` bytes of its output, then SIGKILL the child. This is how an
/// effectively unbounded call (`x == INT_MAX`) is compared: both sides are
/// abandoned at the same byte offset.
fn digest_prefix_forked(f: unsafe extern "C" fn(c_int), x: i32, prefix_len: u64) -> Digest {
    let (mut rx, tx) = std::io::pipe().expect("pipe");
    let tx_fd = tx.as_raw_fd();

    let _g = common::fd_lock().lock().unwrap();
    // Flush every C stream before forking so the child cannot re-emit buffered
    // parent output.
    // SAFETY: see below; the child only calls dup2 / the pre-resolved `driver`
    // pointer / `_exit`, all async-signal-safe enough for this use, and tests
    // run single-threaded (RUST_TEST_THREADS=1 in .cargo/config.toml).
    let pid = unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child ----
            dup2(tx_fd, STDOUT_FD);
            f(x);
            // `_exit` skips libc's atexit handlers, so the final partial stdout
            // buffer would otherwise be discarded and the tail of the output
            // lost. Flush explicitly before leaving.
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        pid
    };
    drop(tx); // parent closes its write end so EOF is possible

    let mut d = new_digest();
    let mut buf = vec![0u8; 1 << 20];
    while d.bytes < prefix_len {
        let want = ((prefix_len - d.bytes) as usize).min(buf.len());
        match rx.read(&mut buf[..want]) {
            Ok(0) | Err(_) => break,
            Ok(n) => fold(&mut d, &buf[..n]),
        }
    }

    // SAFETY: reaping the child we just forked.
    unsafe {
        kill(pid, SIGKILL);
        let mut status: c_int = 0;
        waitpid(pid, &mut status, 0);
    }
    drop(rx);
    drop(_g);

    d
}

// ===========================================================================
// C11 — the `j` signed-overflow regime, i >= 2^30
// ===========================================================================

/// `j += 2` leaves `int` range once `i` reaches `2^30`: `j` becomes
/// `-2147483648`. Reaching it needs more than 1.07e9 iterations and emits about
/// 21.9 GB, so the output is streamed and hashed rather than stored.
///
/// The byte count is additionally checked against a closed-form model of
/// two's-complement wrapping, and against the model where `j` would *not* wrap.
/// That distinguishes "both agree" from "both agree and actually crossed the
/// boundary".
#[test]
fn cfg_c11_j_signed_overflow() {
    const OVERFLOW_I: i64 = 1 << 30; // first i at which j wraps
    const TAIL: i64 = 4096; // iterations past the boundary
    let x = (OVERFLOW_I + TAIL) as i32;

    let p = pair();
    let c = digest_full_run(p.c.raw_driver(), x);
    let r = digest_full_run(p.rust.raw_driver(), x);

    assert_eq!(
        c, r,
        "C11 DIVERGENCE across the j overflow boundary: C={c:?} Rust={r:?}"
    );

    // Closed-form expected byte counts.
    let expected_wrap = expected_bytes(x as i64, true);
    let expected_nowrap = expected_bytes(x as i64, false);
    assert_ne!(
        expected_wrap, expected_nowrap,
        "C11 model sanity: the two models must be distinguishable"
    );
    assert_eq!(
        c.bytes, expected_wrap,
        "C11: byte count must match the two's-complement wrapping model \
         (wrap={expected_wrap}, nowrap={expected_nowrap}, measured={})",
        c.bytes
    );
    eprintln!(
        "C11 ok: x={x}, {} bytes, hash={:#x}; {TAIL} lines had negative j \
         (first j = {} at i = {OVERFLOW_I})",
        c.bytes,
        c.hash,
        i32::MIN
    );
}

/// Total stdout bytes `driver(x)` should emit.
/// `wrap = true` models `j` as wrapping `i32` arithmetic (what the compiled C
/// does and what the Rust `wrapping_add` reproduces); `wrap = false` models `j`
/// as unbounded `2*i`, and exists only as a contrast so the assertion above is
/// meaningful.
fn expected_bytes(x: i64, wrap: bool) -> u64 {
    fn dlen(mut v: u64) -> u64 {
        if v == 0 {
            return 1;
        }
        let mut n = 0;
        while v > 0 {
            n += 1;
            v /= 10;
        }
        n
    }
    /// Sum of decimal lengths of every even value in `[0, hi)`.
    fn even_dlen_sum(hi: u64) -> u64 {
        let mut s = 0u64;
        let mut p = 1u64;
        let mut d = 1u64;
        if hi > 0 {
            s += 1; // v == 0
        }
        while p < hi {
            let a = p.max(2);
            let b = (p * 10).min(hi);
            if b > a {
                // count of even numbers in [a, b)
                let cnt = (b - 1) / 2 - (a - 1) / 2;
                s += cnt * d;
            }
            p *= 10;
            d += 1;
        }
        s
    }
    /// Sum of decimal lengths of every value in `[0, hi)`.
    fn dlen_sum(hi: u64) -> u64 {
        let mut s = 0u64;
        let mut p = 1u64;
        let mut d = 1u64;
        if hi > 0 {
            s += 1; // v == 0
        }
        while p < hi {
            let a = p.max(1);
            let b = (p * 10).min(hi);
            if b > a {
                s += (b - a) * d;
            }
            p *= 10;
            d += 1;
        }
        s
    }

    let n = x as u64;
    // `i` field: every i in [0, x)
    let ti = dlen_sum(n);

    // `j` field
    let ov = 1u64 << 30;
    let tj = if !wrap || n <= ov {
        // j = 2i for all i, no wrap needed / requested
        even_dlen_sum(2 * n)
    } else {
        // i in [0, 2^30): j = 2i, even, in [0, 2^31)
        let mut s = even_dlen_sum(2 * ov);
        // i in [2^30, x): j = 2i - 2^32 = -2^31 + 2*(i - 2^30), negative
        for k in 0..(n - ov) {
            let mag = (1u64 << 31) - 2 * k;
            s += 1 + dlen(mag); // '-' plus the magnitude
        }
        s
    };

    // Each line adds one ' ' and one '\n'.
    ti + tj + 2 * n
}

// ===========================================================================
// C10 — x == INT_MAX, the maximum valid count
// ===========================================================================

/// `INT_MAX` is a perfectly valid count, but running it to completion would emit
/// roughly 46 GB over ~3 minutes per library. Both sides are instead started in
/// a forked child, compared over an identical 64 MiB prefix, and then killed at
/// the same offset.
#[test]
fn cfg_c10_int_max_prefix() {
    const PREFIX: u64 = 64 << 20; // 64 MiB

    let p = pair();
    let c = digest_prefix_forked(p.c.raw_driver(), i32::MAX, PREFIX);
    let r = digest_prefix_forked(p.rust.raw_driver(), i32::MAX, PREFIX);

    assert_eq!(
        c.bytes, PREFIX,
        "C10: did not read the full prefix from the C child (got {})",
        c.bytes
    );
    assert_eq!(
        c, r,
        "C10 DIVERGENCE on the INT_MAX prefix: C={c:?} Rust={r:?}"
    );
    eprintln!("C10 ok: INT_MAX prefix {} bytes, hash={:#x}", c.bytes, c.hash);
}

/// The same abandon-early technique on a modest `x`, as a self-check that the
/// fork/prefix machinery agrees with a full in-memory capture. Without this,
/// a bug in `digest_prefix_forked` could make C10 vacuously pass.
#[test]
fn cfg_c10_prefix_machinery_selfcheck() {
    let p = pair();
    // A run that finishes well before the prefix limit: the prefix digest must
    // then equal the digest of the complete output.
    let full = digest_full_run(p.c.raw_driver(), 10_000);
    let forked = digest_prefix_forked(p.c.raw_driver(), 10_000, 1 << 30);
    assert_eq!(
        full, forked,
        "fork/prefix harness disagrees with the in-process capture"
    );
    assert!(full.bytes > 0, "self-check produced no output");
}
