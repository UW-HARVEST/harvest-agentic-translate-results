//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic boundary rows. Each
//! constructs the exact invalid input/condition, calls BOTH `.so`s, and asserts
//! they return the SAME sentinel (this API's only error signal is `NULL`).

mod common;

use common::{assert_same, impls, Rng, SEED};
use std::ffi::c_char;

/// E1 — `str == NULL` (`if(!str)`, lib.c:11). Both must return `NULL`.
#[test]
fn err_e1_null_input() {
    let i = impls();
    for _ in 0..1000 {
        let c = unsafe { (i.c)(std::ptr::null()) };
        let r = unsafe { (i.rust)(std::ptr::null()) };
        assert!(c.is_null(), "C returned {c:?} for NULL input");
        assert!(r.is_null(), "Rust returned {r:?} for NULL input");
        assert_eq!(c, r, "sentinel mismatch on NULL input");
    }
}

/// E2 — `malloc` returns `NULL`.
///
/// Forced in a forked child: a large NUL-terminated buffer is mapped *first*,
/// then `RLIMIT_AS` is clamped just above current usage so the library's own
/// `malloc(len)` cannot obtain more address space. Both implementations must
/// return `NULL` and must not write through it.
///
/// The child exits with:
///   0 = both returned NULL (agreement)
///   1 = C returned non-NULL, Rust NULL
///   2 = Rust returned non-NULL, C NULL
///   3 = neither returned NULL (limit did not bite; test re-tries larger)
///   4 = setup failure
#[test]
fn err_e2_malloc_failure_under_rlimit() {
    // Load both libraries in the parent so dlopen's own allocations happen
    // before the limit is applied and are inherited by the child.
    let i = impls();

    let mut outcome = None;
    // Escalate the payload size until the allocation is guaranteed to exceed
    // whatever slack remains in the address-space limit.
    for &payload_mib in &[64usize, 256, 1024] {
        let code = fork_and_probe(i, payload_mib);
        if code == 3 {
            continue; // malloc still succeeded; try a bigger request
        }
        outcome = Some((payload_mib, code));
        break;
    }

    let (mib, code) = outcome.expect(
        "could not force malloc failure at any probe size; E2 not exercised (investigate)",
    );
    assert_eq!(
        code, 0,
        "malloc-failure divergence at {mib} MiB payload: exit code {code} \
         (1 = C non-NULL/Rust NULL, 2 = Rust non-NULL/C NULL, 4 = setup failure)"
    );
}

fn fork_and_probe(i: &common::Impls, payload_mib: usize) -> i32 {
    let bytes = payload_mib * 1024 * 1024;
    let c_fn = i.c;
    let rust_fn = i.rust;

    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            // ---- child ----
            let map_len = bytes + 1;
            let region = libc::mmap(
                std::ptr::null_mut(),
                map_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            if region == libc::MAP_FAILED {
                libc::_exit(4);
            }
            let buf = region as *mut u8;
            // Fill EVERY byte: the mapping is zero-filled, so a strided fill
            // would leave a NUL at offset 1 and `strlen` would return 1,
            // making the subsequent malloc trivially small.
            libc::memset(region, b'Z' as libc::c_int, bytes);
            *buf.add(bytes) = 0;

            // Clamp address space to just above what we already occupy, so any
            // further mmap/brk (i.e. the library's malloc) fails.
            let mut usage = std::mem::zeroed::<libc::rlimit>();
            if libc::getrlimit(libc::RLIMIT_AS, &mut usage) != 0 {
                libc::_exit(4);
            }
            let current = current_vm_size().unwrap_or(0);
            if current == 0 {
                libc::_exit(4);
            }
            // Leave a small slack: enough for bookkeeping, far less than `bytes`.
            let slack = 8 * 1024 * 1024;
            let lim = libc::rlimit {
                rlim_cur: (current + slack) as libc::rlim_t,
                rlim_max: usage.rlim_max,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &lim) != 0 {
                libc::_exit(4);
            }
            if std::env::var_os("E2_DEBUG").is_some() {
                let msg = format!(
                    "[e2] payload={bytes} vmsize={current} rlim_cur={} rlim_max={}\n",
                    lim.rlim_cur, lim.rlim_max
                );
                libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
            }

            let p = buf as *const c_char;
            let c_res = c_fn(p);
            let r_res = rust_fn(p);

            let code = match (c_res.is_null(), r_res.is_null()) {
                (true, true) => 0,
                (false, true) => 1,
                (true, false) => 2,
                (false, false) => 3,
            };
            libc::_exit(code);
        }

        // ---- parent ----
        let mut status: i32 = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        if libc::WIFEXITED(status) {
            libc::WEXITSTATUS(status)
        } else {
            // Crash (e.g. write through NULL) is a hard failure, never agreement.
            -(libc::WTERMSIG(status))
        }
    }
}

/// Current virtual-memory size in bytes, from `/proc/self/statm` (field 1, in
/// pages). Read with raw syscalls only — the child must not allocate here.
fn current_vm_size() -> Option<usize> {
    let mut buf = [0u8; 128];
    unsafe {
        let fd = libc::open(c"/proc/self/statm".as_ptr(), libc::O_RDONLY);
        if fd < 0 {
            return None;
        }
        let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len() - 1);
        libc::close(fd);
        if n <= 0 {
            return None;
        }
        let s = std::str::from_utf8(&buf[..n as usize]).ok()?;
        let pages: usize = s.split_whitespace().next()?.parse().ok()?;
        let page = libc::sysconf(libc::_SC_PAGESIZE) as usize;
        Some(pages * page)
    }
}

/// G1 — NULL interleaved with valid calls: the sentinel must not depend on
/// call history, and a NULL call must not disturb a following valid call.
#[test]
fn err_g1_null_interleaved_with_valid() {
    let mut rng = Rng::new(SEED ^ 101);
    let i = impls();

    for _ in 0..2000 {
        // NULL, then valid, then NULL again — both implementations in lockstep.
        assert!(unsafe { (i.c)(std::ptr::null()) }.is_null());
        assert!(unsafe { (i.rust)(std::ptr::null()) }.is_null());

        let n = rng.below(64) as usize;
        let input = rng.cstring(n);
        assert_same(&input);

        assert!(unsafe { (i.rust)(std::ptr::null()) }.is_null());
        assert!(unsafe { (i.c)(std::ptr::null()) }.is_null());
    }
}

/// G2 — zero-length input `""`: `len == 1`, the smallest valid input (one step
/// below it is only `NULL`, covered by E1).
#[test]
fn err_g2_zero_length() {
    let i = impls();
    let input = b"\0";
    let p = input.as_ptr() as *const c_char;

    let c = unsafe { (i.c)(p) };
    let r = unsafe { (i.rust)(p) };
    assert!(!c.is_null() && !r.is_null(), "empty string must not be rejected");
    assert_eq!(unsafe { libc::strlen(c) }, 0);
    assert_eq!(unsafe { libc::strlen(r) }, 0);
    assert_eq!(unsafe { *c }, 0, "C must write the NUL terminator");
    assert_eq!(unsafe { *r }, 0, "Rust must write the NUL terminator");
    unsafe {
        libc::free(c as *mut libc::c_void);
        libc::free(r as *mut libc::c_void);
    }
    assert_same(input);
}

/// G3 — one step past the smallest input: a single payload byte (`len == 2`).
#[test]
fn err_g3_one_byte() {
    for b in 1u16..=255 {
        assert_same(&[b as u8, 0]);
    }
}

/// G4 — oversized length: crosses `malloc`'s mmap threshold but still succeeds.
#[test]
fn err_g4_oversized_length() {
    let mut rng = Rng::new(SEED ^ 104);
    let mut v: Vec<u8> = (0..(20 * 1024 * 1024)).map(|_| rng.nonzero_byte()).collect();
    v.push(0);
    assert_same(&v);
}

/// G5 — out-of-range enum across FFI: N/A for this API (`custom_strdup` takes a
/// single `const char *` and no enum/int parameter). Asserted mechanically so
/// the row is not silently dropped: the exported symbol takes exactly one
/// pointer argument, which this test documents by calling it through a
/// one-pointer signature obtained from `dlsym`.
#[test]
fn err_g5_no_enum_parameter() {
    let i = impls();
    // If the ABI ever grew an int/enum parameter this call would be wrong; the
    // header (`c_src/include/lib.h`) declares exactly `char *(const char *)`.
    let input = b"abc\0";
    let c = unsafe { (i.c)(input.as_ptr() as *const c_char) };
    let r = unsafe { (i.rust)(input.as_ptr() as *const c_char) };
    assert!(!c.is_null() && !r.is_null());
    unsafe {
        libc::free(c as *mut libc::c_void);
        libc::free(r as *mut libc::c_void);
    }
}
