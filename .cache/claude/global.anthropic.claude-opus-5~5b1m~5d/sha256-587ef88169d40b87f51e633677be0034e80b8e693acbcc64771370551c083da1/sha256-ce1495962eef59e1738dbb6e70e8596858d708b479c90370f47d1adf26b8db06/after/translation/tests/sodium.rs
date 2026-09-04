//! Differential tests for the `sodium` area:
//!   * `sodium/utils.c`      (memcmp/compare/is_zero/increment/add/sub/memzero/
//!                            stackzero/pad/unpad/malloc/allocarray/free/mlock/
//!                            munlock/mprotect_*/_sodium_alloc_init)
//!   * `sodium/codecs.c`     (bin2hex/hex2bin/bin2base64/base642bin/
//!                            base64_encoded_len/ip2bin/bin2ip)
//!   * `sodium/core.c`       (sodium_init/crit_enter/crit_leave/misuse/
//!                            set_misuse_handler)
//!   * `sodium/runtime.c`    (_sodium_runtime_get_cpu_features + every
//!                            sodium_runtime_has_*)
//!   * `sodium/version.c`
//!   * `randombytes/randombytes.c`, `randombytes/sysrandom/*`,
//!     `randombytes/internal/*`
//!   * `crypto_ipcrypt/*`
//!
//! Everything goes through `dlopen`/`dlsym` on the two shared objects; nothing
//! is called directly.
//!
//! `sodium_misuse()` paths call `abort()`, so they are exercised in a forked
//! child (see `assert_aborts`) rather than in-process.

#[macro_use]
mod common;

use core::ffi::{c_char, c_int, c_void};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// libc odds and ends we need in the test itself
// ---------------------------------------------------------------------------

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
}

const RLIMIT_CORE: c_int = 4;

const ENOMEM: c_int = 12;
const EINVAL: c_int = 22;
const ERANGE: c_int = 34;
const ENOSYS: c_int = 38;

const ERRNO_SENTINEL: c_int = 0x7abc;

fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}
fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}

/// Serializes the tests that mutate process-wide library state (the
/// randombytes implementation pointer, the misuse handler).
static STATE_LOCK: Mutex<()> = Mutex::new(());

/// `getsym!` requires a literal symbol name; this is the runtime-name variant.
fn sym<T: Copy>(lib: &libloading::Library, name: &str) -> T {
    let mut b = name.as_bytes().to_vec();
    b.push(0);
    unsafe {
        let s: libloading::Symbol<T> = lib
            .get(&b)
            .unwrap_or_else(|e| panic!("missing symbol {name}: {e}"));
        *s
    }
}

// ---------------------------------------------------------------------------
// abort/misuse verification via fork()
// ---------------------------------------------------------------------------

/// Runs `f` in a forked child; returns the raw `waitpid` status.
fn child_status<F: FnOnce()>(f: F) -> c_int {
    use std::io::Write;
    std::io::stdout().flush().ok();
    std::io::stderr().flush().ok();
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // no core dumps for the deliberate aborts
            let rl = RLimit { cur: 0, max: 0 };
            setrlimit(RLIMIT_CORE, &rl);
            f();
            _exit(77); // reached only if the call unexpectedly returned
        }
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid");
        status
    }
}

fn wait_signal(status: c_int) -> Option<c_int> {
    let sig = status & 0x7f;
    if sig != 0 && sig != 0x7f {
        Some(sig)
    } else {
        None
    }
}

fn wait_exit_code(status: c_int) -> Option<c_int> {
    if status & 0x7f == 0 {
        Some((status >> 8) & 0xff)
    } else {
        None
    }
}

const SIGABRT: c_int = 6;

/// Asserts that `f` kills the forked child with `SIGABRT` (i.e. C `abort()`).
fn assert_aborts<F: FnOnce()>(ctx: &str, f: F) {
    let st = child_status(f);
    assert_eq!(
        wait_signal(st),
        Some(SIGABRT),
        "{ctx}: expected SIGABRT, raw wait status {st:#x} (exit code {:?})",
        wait_exit_code(st)
    );
}

// ---------------------------------------------------------------------------
// canary helpers
// ---------------------------------------------------------------------------

const CANARY: u8 = 0x5a;
const CANARY_LEN: usize = 32;

/// Buffer of `n` usable bytes followed by `CANARY_LEN` canary bytes.
fn canary_buf(n: usize) -> Vec<u8> {
    let mut v = vec![CANARY; n + CANARY_LEN];
    for (i, b) in v[..n].iter_mut().enumerate() {
        *b = (0xa5u8).wrapping_add(i as u8);
    }
    v
}

// ===========================================================================
// utils.c
// ===========================================================================

type FnMemcmp = unsafe extern "C" fn(*const c_void, *const c_void, usize) -> c_int;

#[test]
fn utils_memcmp() {
    let (c, r) = both!("sodium_memcmp", FnMemcmp);
    let mut rng = common::Rng::new(0x5EED_0001);

    // len == 0: always "equal" (0), even for NULL pointers.
    unsafe {
        let rc = c(core::ptr::null(), core::ptr::null(), 0);
        let rr = r(core::ptr::null(), core::ptr::null(), 0);
        common::eqi("memcmp len=0 NULL", rc, rr);
        assert_eq!(rc, 0, "memcmp len=0 must be 0");
    }

    for len in [0usize, 1, 2, 7, 8, 15, 16, 17, 31, 32, 33, 64, 100] {
        for _ in 0..20 {
            let a = rng.bytes(len);
            // equal
            unsafe {
                let rc = c(a.as_ptr() as *const c_void, a.as_ptr() as *const c_void, len);
                let rr = r(a.as_ptr() as *const c_void, a.as_ptr() as *const c_void, len);
                common::eqi(&format!("memcmp equal-same-ptr len={len}"), rc, rr);
                let b = a.clone();
                let rc = c(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, len);
                let rr = r(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, len);
                common::eqi(&format!("memcmp equal len={len}"), rc, rr);
                assert_eq!(rc, 0);
            }
            // differ at every byte position, with several deltas
            for pos in 0..len {
                for delta in [1u8, 0x0f, 0x80, 0xff] {
                    let mut b = a.clone();
                    b[pos] ^= delta;
                    if b[pos] == a[pos] {
                        continue;
                    }
                    unsafe {
                        let rc = c(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, len);
                        let rr = r(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, len);
                        common::eqi(&format!("memcmp diff len={len} pos={pos} d={delta}"), rc, rr);
                        assert_eq!(rc, -1);
                    }
                }
            }
        }
    }
}

type FnCompare = unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int;

/// Reference little-endian multi-precision comparison (sanity oracle).
fn ref_compare(a: &[u8], b: &[u8]) -> c_int {
    for i in (0..a.len()).rev() {
        if a[i] > b[i] {
            return 1;
        }
        if a[i] < b[i] {
            return -1;
        }
    }
    0
}

#[test]
fn utils_compare() {
    let (c, r) = both!("sodium_compare", FnCompare);

    // len == 0 -> 0
    unsafe {
        let rc = c(core::ptr::null(), core::ptr::null(), 0);
        let rr = r(core::ptr::null(), core::ptr::null(), 0);
        common::eqi("compare len=0", rc, rr);
        assert_eq!(rc, 0);
    }

    // exhaustive for len == 1
    for x in 0u16..256 {
        for y in 0u16..256 {
            let a = [x as u8];
            let b = [y as u8];
            unsafe {
                let rc = c(a.as_ptr(), b.as_ptr(), 1);
                let rr = r(a.as_ptr(), b.as_ptr(), 1);
                common::eqi(&format!("compare len=1 {x} {y}"), rc, rr);
                assert_eq!(rc, ref_compare(&a, &b), "compare len=1 {x} vs {y}");
            }
        }
    }

    // exhaustive over a boundary-value grid for len == 2 (16^4 = 65536 pairs)
    const VALS: [u8; 16] = [
        0, 1, 2, 3, 0x0f, 0x10, 0x7e, 0x7f, 0x80, 0x81, 0xbf, 0xc0, 0xfd, 0xfe, 0xff, 0x5a,
    ];
    for &a0 in VALS.iter() {
        for &a1 in VALS.iter() {
            for &b0 in VALS.iter() {
                for &b1 in VALS.iter() {
                    let a = [a0, a1];
                    let b = [b0, b1];
                    unsafe {
                        let rc = c(a.as_ptr(), b.as_ptr(), 2);
                        let rr = r(a.as_ptr(), b.as_ptr(), 2);
                        common::eqi("compare len=2 grid", rc, rr);
                        assert_eq!(rc, ref_compare(&a, &b));
                    }
                }
            }
        }
    }

    // random pairs for len 2, and the requested lengths
    let mut rng = common::Rng::new(0x5EED_0002);
    for len in [2usize, 3, 8, 16, 32] {
        for _ in 0..4000 {
            let a = rng.bytes(len);
            let mut b = rng.bytes(len);
            // half the time make them share a long common prefix (from the top)
            if rng.below(2) == 0 {
                let keep = rng.below(len);
                b[keep..].copy_from_slice(&a[keep..]);
            }
            unsafe {
                let rc = c(a.as_ptr(), b.as_ptr(), len);
                let rr = r(a.as_ptr(), b.as_ptr(), len);
                common::eqi(&format!("compare rnd len={len}"), rc, rr);
                assert_eq!(rc, ref_compare(&a, &b));
            }
            // equal
            unsafe {
                let rc = c(a.as_ptr(), a.as_ptr(), len);
                let rr = r(a.as_ptr(), a.as_ptr(), len);
                common::eqi(&format!("compare eq len={len}"), rc, rr);
                assert_eq!(rc, 0);
            }
        }
    }
}

#[test]
fn utils_is_zero() {
    let (c, r) = both!("sodium_is_zero", unsafe extern "C" fn(*const u8, usize) -> c_int);

    unsafe {
        let rc = c(core::ptr::null(), 0);
        let rr = r(core::ptr::null(), 0);
        common::eqi("is_zero len=0", rc, rr);
        assert_eq!(rc, 1);
    }
    let mut rng = common::Rng::new(0x5EED_0003);
    for len in [0usize, 1, 2, 8, 16, 31, 32, 33, 64] {
        let zero = vec![0u8; len];
        unsafe {
            let rc = c(zero.as_ptr(), len);
            let rr = r(zero.as_ptr(), len);
            common::eqi(&format!("is_zero zeros len={len}"), rc, rr);
            assert_eq!(rc, 1);
        }
        for pos in 0..len {
            for v in [1u8, 0x80, 0xff] {
                let mut b = vec![0u8; len];
                b[pos] = v;
                unsafe {
                    let rc = c(b.as_ptr(), len);
                    let rr = r(b.as_ptr(), len);
                    common::eqi(&format!("is_zero nz len={len} pos={pos}"), rc, rr);
                    assert_eq!(rc, 0);
                }
            }
        }
        for _ in 0..20 {
            let b = rng.bytes(len);
            unsafe {
                let rc = c(b.as_ptr(), len);
                let rr = r(b.as_ptr(), len);
                common::eqi(&format!("is_zero rnd len={len}"), rc, rr);
            }
        }
    }
}

#[test]
fn utils_increment() {
    let (c, r) = both!("sodium_increment", unsafe extern "C" fn(*mut u8, usize));
    let mut rng = common::Rng::new(0x5EED_0004);

    for len in [0usize, 1, 2, 3, 7, 8, 9, 11, 12, 13, 16, 23, 24, 25, 32, 64] {
        // deterministic carry patterns + random
        let mut seeds: Vec<Vec<u8>> = vec![
            vec![0u8; len],
            vec![0xffu8; len],
            {
                let mut v = vec![0u8; len];
                if len > 0 {
                    v[0] = 0xff;
                }
                v
            },
            {
                let mut v = vec![0xffu8; len];
                if len > 0 {
                    v[len - 1] = 0;
                }
                v
            },
        ];
        // 0xff.. prefixes of every length
        for k in 0..=len {
            let mut v = vec![0u8; len];
            for i in 0..k {
                v[i] = 0xff;
            }
            seeds.push(v);
        }
        for _ in 0..20 {
            seeds.push(rng.bytes(len));
        }

        for s in seeds {
            let mut cb = canary_buf(len);
            let mut rb = cb.clone();
            cb[..len].copy_from_slice(&s);
            rb[..len].copy_from_slice(&s);
            unsafe {
                c(cb.as_mut_ptr(), len);
                r(rb.as_mut_ptr(), len);
            }
            common::eqb(&format!("increment len={len} in={}", common::hex(&s)), &cb, &rb);
            assert_eq!(&cb[len..], &vec![CANARY; CANARY_LEN][..], "increment canary");
        }
        // repeated increments (walks all carries)
        let mut cb = vec![0u8; len];
        let mut rb = vec![0u8; len];
        for _ in 0..600 {
            unsafe {
                c(cb.as_mut_ptr(), len);
                r(rb.as_mut_ptr(), len);
            }
            common::eqb(&format!("increment iter len={len}"), &cb, &rb);
        }
    }
}

#[test]
fn utils_add_sub() {
    let (ca, ra) = both!("sodium_add", unsafe extern "C" fn(*mut u8, *const u8, usize));
    let (cs, rs) = both!("sodium_sub", unsafe extern "C" fn(*mut u8, *const u8, usize));
    let mut rng = common::Rng::new(0x5EED_0005);

    for len in [0usize, 1, 2, 3, 8, 12, 16, 24, 32, 64, 65] {
        let mut cases: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        cases.push((vec![0u8; len], vec![0u8; len]));
        cases.push((vec![0xffu8; len], vec![0xffu8; len]));
        cases.push((vec![0xffu8; len], {
            let mut v = vec![0u8; len];
            if len > 0 {
                v[0] = 1;
            }
            v
        }));
        cases.push(({
            let mut v = vec![0u8; len];
            if len > 0 {
                v[0] = 1;
            }
            v
        }, vec![0xffu8; len]));
        cases.push((vec![0u8; len], {
            let mut v = vec![0u8; len];
            if len > 0 {
                v[0] = 1;
            }
            v
        }));
        // 0x00.. minus 0x01 -> full borrow chain
        for k in 0..=len.min(8) {
            let mut a = vec![0u8; len];
            let mut b = vec![0u8; len];
            for i in 0..k {
                a[i] = 0xff;
                b[i] = 0x01;
            }
            cases.push((a, b));
        }
        for _ in 0..30 {
            cases.push((rng.bytes(len), rng.bytes(len)));
        }

        for (a, b) in cases {
            for (name, cf, rf) in [("add", ca, ra), ("sub", cs, rs)] {
                let mut cb = canary_buf(len);
                let mut rb = cb.clone();
                cb[..len].copy_from_slice(&a);
                rb[..len].copy_from_slice(&a);
                unsafe {
                    cf(cb.as_mut_ptr(), b.as_ptr(), len);
                    rf(rb.as_mut_ptr(), b.as_ptr(), len);
                }
                common::eqb(
                    &format!("{name} len={len} a={} b={}", common::hex(&a), common::hex(&b)),
                    &cb,
                    &rb,
                );
                assert_eq!(&cb[len..], &vec![CANARY; CANARY_LEN][..], "{name} canary");
            }
            // aliasing: a == b
            for (name, cf, rf) in [("add-alias", ca, ra), ("sub-alias", cs, rs)] {
                let mut cb = canary_buf(len);
                let mut rb = cb.clone();
                cb[..len].copy_from_slice(&a);
                rb[..len].copy_from_slice(&a);
                unsafe {
                    let p = cb.as_mut_ptr();
                    cf(p, p, len);
                    let p = rb.as_mut_ptr();
                    rf(p, p, len);
                }
                common::eqb(&format!("{name} len={len}"), &cb, &rb);
            }
        }
    }
}

#[test]
fn utils_memzero_stackzero() {
    let (c, r) = both!("sodium_memzero", unsafe extern "C" fn(*mut c_void, usize));
    for n in [0usize, 1, 2, 7, 8, 15, 16, 31, 32, 64, 1000] {
        for off in [0usize, 1, 3] {
            let total = n + off + CANARY_LEN;
            let mut cb = vec![0xa7u8; total];
            let mut rb = cb.clone();
            unsafe {
                c(cb.as_mut_ptr().add(off) as *mut c_void, n);
                r(rb.as_mut_ptr().add(off) as *mut c_void, n);
            }
            common::eqb(&format!("memzero n={n} off={off}"), &cb, &rb);
            assert!(cb[off..off + n].iter().all(|&b| b == 0));
            assert!(cb[..off].iter().all(|&b| b == 0xa7));
            assert!(cb[off + n..].iter().all(|&b| b == 0xa7));
        }
    }
    // sodium_memzero(NULL, 0) must be a no-op
    unsafe {
        c(core::ptr::null_mut(), 0);
        r(core::ptr::null_mut(), 0);
    }

    // sodium_stackzero: empty body in this configuration (no HAVE_C_VARARRAYS
    // and no HAVE_ALLOCA) -- nothing observable, but it must exist and not
    // crash for any length.
    let (c, r) = both!("sodium_stackzero", unsafe extern "C" fn(usize));
    for n in [0usize, 1, 64, 4096, 100_000] {
        unsafe {
            c(n);
            r(n);
        }
    }
}

type FnPad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;
type FnUnpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;

#[test]
fn utils_pad_unpad() {
    let (cpad, rpad) = both!("sodium_pad", FnPad);
    let (cunpad, runpad) = both!("sodium_unpad", FnUnpad);
    let mut rng = common::Rng::new(0x5EED_0006);

    for blocksize in [1usize, 2, 3, 15, 16, 17, 32, 64] {
        for unpadded in 0..=(2 * blocksize + 1) {
            // generous buffer + canary
            let cap = unpadded + blocksize + 8;
            for max_buflen in [cap, unpadded + 1, unpadded, 0, unpadded + blocksize] {
                let base = {
                    let mut v = canary_buf(cap);
                    rng.fill(&mut v[..unpadded]);
                    v
                };
                let mut cb = base.clone();
                let mut rb = base.clone();
                let mut cl: usize = 0xdead_beef;
                let mut rl: usize = 0xdead_beef;
                let (rc, rr) = unsafe {
                    (
                        cpad(&mut cl, cb.as_mut_ptr(), unpadded, blocksize, max_buflen),
                        rpad(&mut rl, rb.as_mut_ptr(), unpadded, blocksize, max_buflen),
                    )
                };
                let ctx = format!("pad bs={blocksize} n={unpadded} max={max_buflen}");
                common::eqi(&ctx, rc, rr);
                assert_eq!(cl, rl, "{ctx}: padded_buflen");
                common::eqb(&ctx, &cb, &rb);
                assert_eq!(&cb[cap..], &vec![CANARY; CANARY_LEN][..], "{ctx}: canary");

                if rc != 0 {
                    continue;
                }
                let padded_len = cl;
                assert_eq!(padded_len % blocksize, 0, "{ctx}: padded_len not a multiple");

                // unpad the result, with the exact and with wrong block sizes
                for ubs in [blocksize, 1, blocksize + 1, blocksize.saturating_sub(1), 0] {
                    let mut cul: usize = 0xdead_beef;
                    let mut rul: usize = 0xdead_beef;
                    let (uc, ur) = unsafe {
                        (
                            cunpad(&mut cul, cb.as_ptr(), padded_len, ubs),
                            runpad(&mut rul, rb.as_ptr(), padded_len, ubs),
                        )
                    };
                    let uctx = format!("{ctx} unpad bs={ubs}");
                    common::eqi(&uctx, uc, ur);
                    assert_eq!(cul, rul, "{uctx}: unpadded_buflen");
                    if ubs == blocksize {
                        assert_eq!(uc, 0, "{uctx}: round-trip must succeed");
                        assert_eq!(cul, unpadded, "{uctx}: round-trip length");
                    }
                }

                // corrupt the padding and unpad again
                for pos in padded_len.saturating_sub(blocksize)..padded_len {
                    let mut cc = cb.clone();
                    let mut rc2 = rb.clone();
                    cc[pos] ^= 0x80;
                    rc2[pos] ^= 0x80;
                    let mut cul: usize = 0xdead_beef;
                    let mut rul: usize = 0xdead_beef;
                    let (uc, ur) = unsafe {
                        (
                            cunpad(&mut cul, cc.as_ptr(), padded_len, blocksize),
                            runpad(&mut rul, rc2.as_ptr(), padded_len, blocksize),
                        )
                    };
                    common::eqi(&format!("{ctx} unpad corrupt@{pos}"), uc, ur);
                    assert_eq!(cul, rul, "{ctx} unpad corrupt@{pos} len");
                }
            }
        }
    }

    // blocksize == 0 -> -1 (both pad and unpad), buffer untouched
    for &n in &[0usize, 1, 16] {
        let base = canary_buf(64);
        let mut cb = base.clone();
        let mut rb = base.clone();
        let mut cl = 1usize;
        let mut rl = 1usize;
        let (rc, rr) = unsafe {
            (
                cpad(&mut cl, cb.as_mut_ptr(), n, 0, 64),
                rpad(&mut rl, rb.as_mut_ptr(), n, 0, 64),
            )
        };
        common::eqi("pad blocksize=0", rc, rr);
        assert_eq!(rc, -1);
        assert_eq!(cl, rl);
        common::eqb("pad blocksize=0 buf", &cb, &rb);

        let mut cl = 1usize;
        let mut rl = 1usize;
        let (uc, ur) = unsafe {
            (
                cunpad(&mut cl, cb.as_ptr(), 64, 0),
                runpad(&mut rl, rb.as_ptr(), 64, 0),
            )
        };
        common::eqi("unpad blocksize=0", uc, ur);
        assert_eq!(uc, -1);
        assert_eq!(cl, rl);
    }

    // unpad: padded_buflen < blocksize -> -1, *unpadded_buflen_p untouched
    for (pbl, bs) in [(0usize, 1usize), (0, 16), (1, 2), (15, 16), (16, 17)] {
        let buf = vec![0u8; 64];
        let mut cl = 0xabcd_usize;
        let mut rl = 0xabcd_usize;
        let (uc, ur) = unsafe {
            (
                cunpad(&mut cl, buf.as_ptr(), pbl, bs),
                runpad(&mut rl, buf.as_ptr(), pbl, bs),
            )
        };
        common::eqi(&format!("unpad short pbl={pbl} bs={bs}"), uc, ur);
        assert_eq!(uc, -1);
        assert_eq!(cl, rl);
        assert_eq!(cl, 0xabcd, "must not write the out-param");
    }

    // unpad of an all-zero buffer (no 0x80 barrier) -> -1 but the out-param IS
    // written (C writes it unconditionally after the loop).
    for bs in [1usize, 2, 16, 17] {
        for pbl in [bs, bs * 2, bs * 3] {
            let buf = vec![0u8; pbl];
            let mut cl = 0xabcd_usize;
            let mut rl = 0xabcd_usize;
            let (uc, ur) = unsafe {
                (
                    cunpad(&mut cl, buf.as_ptr(), pbl, bs),
                    runpad(&mut rl, buf.as_ptr(), pbl, bs),
                )
            };
            common::eqi(&format!("unpad zeros pbl={pbl} bs={bs}"), uc, ur);
            assert_eq!(uc, -1);
            assert_eq!(cl, rl, "unpad zeros out-param");
        }
    }

    // unpad on fully random buffers (mostly invalid) -- exact agreement.
    for bs in [1usize, 2, 16, 17, 64] {
        for _ in 0..200 {
            let pbl = bs + rng.below(3 * bs);
            let buf = rng.bytes(pbl);
            let mut cl = 0xabcd_usize;
            let mut rl = 0xabcd_usize;
            let (uc, ur) = unsafe {
                (
                    cunpad(&mut cl, buf.as_ptr(), pbl, bs),
                    runpad(&mut rl, buf.as_ptr(), pbl, bs),
                )
            };
            common::eqi(&format!("unpad rnd pbl={pbl} bs={bs}"), uc, ur);
            assert_eq!(cl, rl, "unpad rnd out-param");
        }
    }

    // padded_buflen_p == NULL is allowed for sodium_pad.
    for bs in [1usize, 16, 17] {
        for n in [0usize, 1, 17] {
            let base = canary_buf(n + bs + 8);
            let mut cb = base.clone();
            let mut rb = base.clone();
            let (rc, rr) = unsafe {
                (
                    cpad(core::ptr::null_mut(), cb.as_mut_ptr(), n, bs, n + bs + 8),
                    rpad(core::ptr::null_mut(), rb.as_mut_ptr(), n, bs, n + bs + 8),
                )
            };
            common::eqi("pad NULL out", rc, rr);
            common::eqb("pad NULL out buf", &cb, &rb);
        }
    }

    // xpadded_len >= max_buflen with a huge unpadded_buflen: returns -1 before
    // touching `buf` (so passing a tiny buffer is safe).
    let mut dummy = [0u8; 1];
    for (n, bs) in [
        (usize::MAX - 1, 1usize),
        (usize::MAX / 2, 16),
        (usize::MAX - 16, 8),
    ] {
        let mut cl = 0usize;
        let mut rl = 0usize;
        let (rc, rr) = unsafe {
            (
                cpad(&mut cl, dummy.as_mut_ptr(), n, bs, 1),
                rpad(&mut rl, dummy.as_mut_ptr(), n, bs, 1),
            )
        };
        common::eqi(&format!("pad huge n={n} bs={bs}"), rc, rr);
        assert_eq!(rc, -1);
        assert_eq!(cl, rl);
    }
}

#[test]
fn utils_alloc_and_locks() {
    let (cmalloc, rmalloc) = both!("sodium_malloc", unsafe extern "C" fn(usize) -> *mut c_void);
    let (carr, rarr) = both!("sodium_allocarray", unsafe extern "C" fn(usize, usize) -> *mut c_void);
    let (cfree, rfree) = both!("sodium_free", unsafe extern "C" fn(*mut c_void));
    let (cml, rml) = both!("sodium_mlock", unsafe extern "C" fn(*mut c_void, usize) -> c_int);
    let (cmu, rmu) = both!("sodium_munlock", unsafe extern "C" fn(*mut c_void, usize) -> c_int);
    let (cpn, rpn) = both!("sodium_mprotect_noaccess", unsafe extern "C" fn(*mut c_void) -> c_int);
    let (cpr, rpr) = both!("sodium_mprotect_readonly", unsafe extern "C" fn(*mut c_void) -> c_int);
    let (cpw, rpw) = both!("sodium_mprotect_readwrite", unsafe extern "C" fn(*mut c_void) -> c_int);
    let (cai, rai) = both!("_sodium_alloc_init", unsafe extern "C" fn() -> c_int);

    // sodium_malloc: non-NULL, filled with GARBAGE_VALUE (0xdb)
    for size in [0usize, 1, 2, 15, 16, 17, 63, 64, 4095, 4096, 100_000] {
        unsafe {
            let cp = cmalloc(size) as *mut u8;
            let rp = rmalloc(size) as *mut u8;
            assert!(!cp.is_null(), "C sodium_malloc({size})");
            assert!(!rp.is_null(), "Rust sodium_malloc({size})");
            let cs = core::slice::from_raw_parts(cp, size);
            let rsl = core::slice::from_raw_parts(rp, size);
            common::eqb(&format!("malloc({size}) contents"), cs, rsl);
            assert!(cs.iter().all(|&b| b == 0xdb), "malloc must memset 0xdb");
            // writing/reading the returned buffer
            for i in 0..size {
                *cp.add(i) = (i as u8) ^ 0x33;
                *rp.add(i) = (i as u8) ^ 0x33;
            }
            common::eqb(
                &format!("malloc({size}) after write"),
                core::slice::from_raw_parts(cp, size),
                core::slice::from_raw_parts(rp, size),
            );
            // mlock / munlock: ENOSYS in this build; munlock also zeroes.
            if size > 0 {
                set_errno(ERRNO_SENTINEL);
                let a = cml(cp as *mut c_void, size);
                let ae = get_errno();
                set_errno(ERRNO_SENTINEL);
                let b = rml(rp as *mut c_void, size);
                let be = get_errno();
                common::eqi(&format!("mlock({size})"), a, b);
                assert_eq!(a, -1);
                assert_eq!((ae, be), (ENOSYS, ENOSYS), "mlock errno");

                set_errno(ERRNO_SENTINEL);
                let a = cmu(cp as *mut c_void, size);
                let ae = get_errno();
                set_errno(ERRNO_SENTINEL);
                let b = rmu(rp as *mut c_void, size);
                let be = get_errno();
                common::eqi(&format!("munlock({size})"), a, b);
                assert_eq!(a, -1);
                assert_eq!((ae, be), (ENOSYS, ENOSYS), "munlock errno");
                common::eqb(
                    &format!("munlock({size}) zeroed"),
                    core::slice::from_raw_parts(cp, size),
                    core::slice::from_raw_parts(rp, size),
                );
                assert!(core::slice::from_raw_parts(cp, size).iter().all(|&b| b == 0));
            }
            // mprotect_*: ENOSYS in this build (no HAVE_PAGE_PROTECTION)
            for (name, cf, rf) in [
                ("noaccess", cpn, rpn),
                ("readonly", cpr, rpr),
                ("readwrite", cpw, rpw),
            ] {
                set_errno(ERRNO_SENTINEL);
                let a = cf(cp as *mut c_void);
                let ae = get_errno();
                set_errno(ERRNO_SENTINEL);
                let b = rf(rp as *mut c_void);
                let be = get_errno();
                common::eqi(&format!("mprotect_{name}"), a, b);
                assert_eq!(a, -1);
                assert_eq!((ae, be), (ENOSYS, ENOSYS), "mprotect_{name} errno");
            }
            cfree(cp as *mut c_void);
            rfree(rp as *mut c_void);
        }
    }

    // sodium_malloc when the underlying malloc() fails -> NULL
    for size in [usize::MAX, usize::MAX / 2, usize::MAX - 4096, 1usize << 62] {
        unsafe {
            set_errno(ERRNO_SENTINEL);
            let cp = cmalloc(size);
            let ce = get_errno();
            set_errno(ERRNO_SENTINEL);
            let rp = rmalloc(size);
            let re = get_errno();
            assert!(cp.is_null(), "C sodium_malloc({size}) should fail");
            assert!(rp.is_null(), "Rust sodium_malloc({size}) should fail");
            assert_eq!(ce, re, "sodium_malloc({size}) errno");
            assert_eq!(ce, ENOMEM, "sodium_malloc({size}) errno");
        }
    }

    // mprotect_* / mlock / munlock on NULL: still ENOSYS, never dereferenced.
    unsafe {
        for (name, cf, rf) in [
            ("noaccess", cpn, rpn),
            ("readonly", cpr, rpr),
            ("readwrite", cpw, rpw),
        ] {
            set_errno(ERRNO_SENTINEL);
            let a = cf(core::ptr::null_mut());
            let ae = get_errno();
            set_errno(ERRNO_SENTINEL);
            let b = rf(core::ptr::null_mut());
            let be = get_errno();
            common::eqi(&format!("mprotect_{name}(NULL)"), a, b);
            assert_eq!((a, ae, be), (-1, ENOSYS, ENOSYS));
        }
        set_errno(ERRNO_SENTINEL);
        let a = cml(core::ptr::null_mut(), 0);
        let ae = get_errno();
        set_errno(ERRNO_SENTINEL);
        let b = rml(core::ptr::null_mut(), 0);
        let be = get_errno();
        common::eqi("mlock(NULL,0)", a, b);
        assert_eq!((a, ae, be), (-1, ENOSYS, ENOSYS));
        set_errno(ERRNO_SENTINEL);
        let a = cmu(core::ptr::null_mut(), 0);
        let ae = get_errno();
        set_errno(ERRNO_SENTINEL);
        let b = rmu(core::ptr::null_mut(), 0);
        let be = get_errno();
        common::eqi("munlock(NULL,0)", a, b);
        assert_eq!((a, ae, be), (-1, ENOSYS, ENOSYS));
    }

    // sodium_free(NULL) must be a no-op
    unsafe {
        cfree(core::ptr::null_mut());
        rfree(core::ptr::null_mut());
    }

    // sodium_allocarray: valid
    for (count, size) in [(0usize, 0usize), (0, 32), (1, 0), (1, 32), (7, 13), (100, 64)] {
        unsafe {
            let cp = carr(count, size) as *mut u8;
            let rp = rarr(count, size) as *mut u8;
            assert!(!cp.is_null() && !rp.is_null(), "allocarray({count},{size})");
            let n = count * size;
            common::eqb(
                &format!("allocarray({count},{size})"),
                core::slice::from_raw_parts(cp, n),
                core::slice::from_raw_parts(rp, n),
            );
            assert!(core::slice::from_raw_parts(cp, n).iter().all(|&b| b == 0xdb));
            cfree(cp as *mut c_void);
            rfree(rp as *mut c_void);
        }
    }

    // sodium_allocarray overflow -> NULL, errno == ENOMEM
    for (count, size) in [
        (2usize, usize::MAX / 2),
        (usize::MAX, 2),
        (3, usize::MAX / 3),
        (usize::MAX, usize::MAX),
        (1, usize::MAX),
        (0x1_0000_0000usize, 0x1_0000_0000usize),
    ] {
        unsafe {
            set_errno(ERRNO_SENTINEL);
            let cp = carr(count, size);
            let ce = get_errno();
            set_errno(ERRNO_SENTINEL);
            let rp = rarr(count, size);
            let re = get_errno();
            assert!(cp.is_null(), "C allocarray({count},{size}) should be NULL");
            assert!(rp.is_null(), "Rust allocarray({count},{size}) should be NULL");
            assert_eq!((ce, re), (ENOMEM, ENOMEM), "allocarray errno");
        }
    }

    // _sodium_alloc_init: refills the (unused in this build) canary, returns 0.
    unsafe {
        for _ in 0..3 {
            let a = cai();
            let b = rai();
            common::eqi("_sodium_alloc_init", a, b);
            assert_eq!(a, 0);
        }
    }
}

// ===========================================================================
// codecs.c -- hex
// ===========================================================================

type FnBin2Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
type FnHex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
) -> c_int;

#[test]
fn codecs_bin2hex() {
    let (c, r) = both!("sodium_bin2hex", FnBin2Hex);
    let mut rng = common::Rng::new(0x5EED_0010);

    for bin_len in 0..=64usize {
        let mut bins: Vec<Vec<u8>> = vec![vec![0u8; bin_len], vec![0xffu8; bin_len]];
        if bin_len > 0 {
            bins.push((0..bin_len).map(|i| i as u8).collect());
            // all 256 byte values individually
            for v in 0..=255u8 {
                let mut b = vec![0u8; bin_len];
                b[bin_len - 1] = v;
                bins.push(b);
            }
        }
        for _ in 0..10 {
            bins.push(rng.bytes(bin_len));
        }
        for bin in bins {
            for extra in [1usize, 2, 8] {
                let maxlen = bin_len * 2 + extra;
                let mut cb = vec![CANARY; maxlen + CANARY_LEN];
                let mut rb = cb.clone();
                let (cp, rp) = unsafe {
                    (
                        c(cb.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len),
                        r(rb.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len),
                    )
                };
                assert_eq!(cp as *const u8, cb.as_ptr(), "bin2hex must return `hex`");
                assert_eq!(rp as *const u8, rb.as_ptr(), "bin2hex must return `hex`");
                common::eqb(&format!("bin2hex len={bin_len} extra={extra}"), &cb, &rb);
                // reference check
                let want = common::hex(&bin);
                assert_eq!(&cb[..bin_len * 2], want.as_bytes());
                assert_eq!(cb[bin_len * 2], 0);
                // only 2n+1 bytes are written
                assert!(cb[bin_len * 2 + 1..].iter().all(|&b| b == CANARY));
            }
        }
    }
}

struct H2b {
    ret: c_int,
    errno: c_int,
    bin: Vec<u8>,
    bin_len: usize,
    hex_end: isize,
}

#[allow(clippy::too_many_arguments)]
fn run_hex2bin(
    f: FnHex2Bin,
    bin_maxlen: usize,
    hex: &[u8],
    hex_len: usize,
    ignore: Option<&[u8]>,
    want_bin_len: bool,
    want_hex_end: bool,
) -> H2b {
    let mut bin = vec![CANARY; bin_maxlen + CANARY_LEN];
    let mut bl: usize = 0xdead_beef;
    let mut he: *const c_char = core::ptr::null();
    let ig = ignore.map(|s| s.as_ptr() as *const c_char).unwrap_or(core::ptr::null());
    set_errno(ERRNO_SENTINEL);
    let ret = unsafe {
        f(
            bin.as_mut_ptr(),
            bin_maxlen,
            hex.as_ptr() as *const c_char,
            hex_len,
            ig,
            if want_bin_len { &mut bl } else { core::ptr::null_mut() },
            if want_hex_end { &mut he } else { core::ptr::null_mut() },
        )
    };
    let errno = get_errno();
    let hex_end = if want_hex_end {
        unsafe { he.offset_from(hex.as_ptr() as *const c_char) }
    } else {
        -1
    };
    H2b { ret, errno, bin, bin_len: bl, hex_end }
}

#[test]
fn codecs_hex2bin() {
    let (c, r) = both!("sodium_hex2bin", FnHex2Bin);
    let mut rng = common::Rng::new(0x5EED_0011);

    let ignores: [Option<&[u8]>; 4] = [None, Some(b": \n\0"), Some(b"\0"), Some(b"xyz\0")];

    // fixed hand-written cases (valid, invalid chars, odd length, separators)
    let fixed: Vec<&[u8]> = vec![
        b"",
        b"0",
        b"00",
        b"0f",
        b"0F",
        b"aB",
        b"ff",
        b"deadbeef",
        b"DEADBEEF",
        b"dEaDbEeF",
        b"0123456789abcdefABCDEF",
        b"de:ad:be:ef",
        b"de ad be ef",
        b"de\nad",
        b"de-ad",
        b"g",
        b"0g",
        b"g0",
        b"00g",
        b"000",
        b"00000",
        b"@0",
        b"/0",
        b":0",
        b"`0",
        b"G0",
        b"z0",
        b"00\x00ff",
        b"\x00\x00",
        b"00 ",
        b" 00",
        b"0 0",
        b"::::",
        b"00::11",
    ];

    for hex in fixed {
        for hex_len in [hex.len(), hex.len().saturating_sub(1), hex.len() + 0] {
            for &ig in ignores.iter() {
                for bin_maxlen in [0usize, 1, 2, 3, hex.len() / 2, hex.len() + 4] {
                    for wbl in [false, true] {
                        for whe in [false, true] {
                            let a = run_hex2bin(c, bin_maxlen, hex, hex_len, ig, wbl, whe);
                            let b = run_hex2bin(r, bin_maxlen, hex, hex_len, ig, wbl, whe);
                            let ctx = format!(
                                "hex2bin hex={:?} hex_len={hex_len} ig={:?} maxlen={bin_maxlen} wbl={wbl} whe={whe}",
                                String::from_utf8_lossy(hex),
                                ig.map(String::from_utf8_lossy)
                            );
                            common::eqi(&ctx, a.ret, b.ret);
                            assert_eq!(a.errno, b.errno, "{ctx}: errno");
                            common::eqb(&ctx, &a.bin, &b.bin);
                            assert_eq!(a.bin_len, b.bin_len, "{ctx}: bin_len");
                            assert_eq!(a.hex_end, b.hex_end, "{ctx}: hex_end");
                        }
                    }
                }
            }
        }
    }

    // randomized: random hex strings mixed with separators and junk
    let alphabet: &[u8] = b"0123456789abcdefABCDEF: \n@Gz/";
    for _ in 0..3000 {
        let n = rng.below(24);
        let hex: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let ig = ignores[rng.below(ignores.len())];
        let bin_maxlen = rng.below(16);
        let wbl = rng.below(2) == 0;
        let whe = rng.below(2) == 0;
        let a = run_hex2bin(c, bin_maxlen, &hex, n, ig, wbl, whe);
        let b = run_hex2bin(r, bin_maxlen, &hex, n, ig, wbl, whe);
        let ctx = format!("hex2bin rnd {:?}", String::from_utf8_lossy(&hex));
        common::eqi(&ctx, a.ret, b.ret);
        assert_eq!(a.errno, b.errno, "{ctx}: errno");
        common::eqb(&ctx, &a.bin, &b.bin);
        assert_eq!(a.bin_len, b.bin_len, "{ctx}: bin_len");
        assert_eq!(a.hex_end, b.hex_end, "{ctx}: hex_end");
    }

    // exact errno values on the two documented rejection sites
    for f in [c, r] {
        // bin_pos >= bin_maxlen -> ERANGE, ret -1, bin_len reset to 0
        let a = run_hex2bin(f, 1, b"deadbeef", 8, None, true, true);
        assert_eq!((a.ret, a.errno, a.bin_len), (-1, ERANGE, 0), "hex2bin ERANGE");
        // odd number of digits -> EINVAL, hex_end backs up one character
        let a = run_hex2bin(f, 8, b"abc", 3, None, true, true);
        assert_eq!((a.ret, a.errno, a.bin_len, a.hex_end), (-1, EINVAL, 0, 2), "hex2bin odd EINVAL");
        // hex_end == NULL and trailing junk -> EINVAL
        // NB: `bin_pos` is zeroed *before* this late EINVAL is raised, so
        // *bin_len still reports the bytes that were decoded.
        let a = run_hex2bin(f, 8, b"aazz", 4, None, true, false);
        assert_eq!((a.ret, a.errno, a.bin_len), (-1, EINVAL, 1), "hex2bin trailing EINVAL");
        // ...but with hex_end != NULL the very same input succeeds
        let a = run_hex2bin(f, 8, b"aazz", 4, None, true, true);
        assert_eq!((a.ret, a.bin_len, a.hex_end), (0, 1, 2), "hex2bin trailing w/ hex_end");
    }

    // round-trip against bin2hex for many sizes
    let (cb2h, _rb2h) = both!("sodium_bin2hex", FnBin2Hex);
    for n in 0..=48usize {
        let bin = rng.bytes(n);
        let mut hexbuf = vec![0u8; 2 * n + 1];
        unsafe {
            cb2h(hexbuf.as_mut_ptr() as *mut c_char, 2 * n + 1, bin.as_ptr(), n);
        }
        let a = run_hex2bin(c, n, &hexbuf, 2 * n, None, true, true);
        let b = run_hex2bin(r, n, &hexbuf, 2 * n, None, true, true);
        common::eqi("hex2bin roundtrip", a.ret, b.ret);
        assert_eq!(a.ret, 0);
        assert_eq!(a.bin_len, n);
        assert_eq!(&a.bin[..n], &bin[..]);
        common::eqb("hex2bin roundtrip", &a.bin, &b.bin);
        assert_eq!(a.hex_end, b.hex_end);
    }
}

// ===========================================================================
// codecs.c -- base64
// ===========================================================================

const VARIANTS: [c_int; 4] = [1, 3, 5, 7]; // ORIGINAL, ORIGINAL_NO_PADDING, URLSAFE, URLSAFE_NO_PADDING

type FnEncLen = unsafe extern "C" fn(usize, c_int) -> usize;
type FnBin2B64 = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
type FnB642Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
    c_int,
) -> c_int;

#[test]
fn codecs_base64_encoded_len() {
    let (c, r) = both!("sodium_base64_encoded_len", FnEncLen);
    for v in VARIANTS {
        for n in 0..=300usize {
            let (a, b) = unsafe { (c(n, v), r(n, v)) };
            assert_eq!(a, b, "encoded_len({n},{v})");
        }
        for n in [
            1000usize,
            1 << 20,
            (1usize << 40) + 1,
            usize::MAX / 8,
            (usize::MAX - 5) / 4 * 3,
        ] {
            let (a, b) = unsafe { (c(n, v), r(n, v)) };
            assert_eq!(a, b, "encoded_len({n},{v})");
        }
    }
}

#[test]
fn codecs_bin2base64() {
    let (c, r) = both!("sodium_bin2base64", FnBin2B64);
    let (cel, _) = both!("sodium_base64_encoded_len", FnEncLen);
    let mut rng = common::Rng::new(0x5EED_0020);

    for v in VARIANTS {
        for bin_len in 0..=64usize {
            let mut bins: Vec<Vec<u8>> = vec![vec![0u8; bin_len], vec![0xffu8; bin_len]];
            if bin_len > 0 {
                bins.push((0..bin_len).map(|i| (i * 7) as u8).collect());
            }
            for _ in 0..8 {
                bins.push(rng.bytes(bin_len));
            }
            let enc_len = unsafe { cel(bin_len, v) };
            for extra in [0usize, 1, 5, 32] {
                let maxlen = enc_len + extra;
                for bin in bins.iter() {
                    let mut cb = vec![CANARY; maxlen + CANARY_LEN];
                    let mut rb = cb.clone();
                    let (cp, rp) = unsafe {
                        (
                            c(cb.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len, v),
                            r(rb.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), bin_len, v),
                        )
                    };
                    assert_eq!(cp as *const u8, cb.as_ptr());
                    assert_eq!(rp as *const u8, rb.as_ptr());
                    let ctx = format!("bin2base64 v={v} n={bin_len} extra={extra}");
                    common::eqb(&ctx, &cb, &rb);
                    // the whole b64_maxlen is written (trailing NUL fill loop)
                    assert!(cb[maxlen..].iter().all(|&b| b == CANARY), "{ctx}: canary");
                    assert_eq!(cb[enc_len - 1], 0, "{ctx}: NUL terminator");
                    // charset check
                    let s = &cb[..enc_len - 1];
                    for &ch in s {
                        let ok = ch.is_ascii_alphanumeric()
                            || (v & 4 == 0 && (ch == b'+' || ch == b'/'))
                            || (v & 4 != 0 && (ch == b'-' || ch == b'_'))
                            || ch == b'=';
                        assert!(ok, "{ctx}: bad char {ch:?} in {:?}", String::from_utf8_lossy(s));
                    }
                    if v & 2 == 0 {
                        assert_eq!((enc_len - 1) % 4, 0, "{ctx}: padded length");
                    }
                }
            }
        }
    }
}

struct B2b {
    ret: c_int,
    errno: c_int,
    bin: Vec<u8>,
    bin_len: usize,
    b64_end: isize,
}

#[allow(clippy::too_many_arguments)]
fn run_b642bin(
    f: FnB642Bin,
    bin_maxlen: usize,
    b64: &[u8],
    b64_len: usize,
    ignore: Option<&[u8]>,
    want_bin_len: bool,
    want_b64_end: bool,
    variant: c_int,
) -> B2b {
    let mut bin = vec![CANARY; bin_maxlen + CANARY_LEN];
    let mut bl: usize = 0xdead_beef;
    let mut be: *const c_char = core::ptr::null();
    let ig = ignore.map(|s| s.as_ptr() as *const c_char).unwrap_or(core::ptr::null());
    set_errno(ERRNO_SENTINEL);
    let ret = unsafe {
        f(
            bin.as_mut_ptr(),
            bin_maxlen,
            b64.as_ptr() as *const c_char,
            b64_len,
            ig,
            if want_bin_len { &mut bl } else { core::ptr::null_mut() },
            if want_b64_end { &mut be } else { core::ptr::null_mut() },
            variant,
        )
    };
    let errno = get_errno();
    let b64_end = if want_b64_end {
        unsafe { be.offset_from(b64.as_ptr() as *const c_char) }
    } else {
        -1
    };
    B2b { ret, errno, bin, bin_len: bl, b64_end }
}

#[test]
fn codecs_base642bin() {
    let (c, r) = both!("sodium_base642bin", FnB642Bin);
    let (cb2b, _) = both!("sodium_bin2base64", FnBin2B64);
    let (cel, _) = both!("sodium_base64_encoded_len", FnEncLen);
    let mut rng = common::Rng::new(0x5EED_0021);

    let ignores: [Option<&[u8]>; 4] = [None, Some(b" \n\0"), Some(b"\0"), Some(b"=\0")];

    // 1) round-trip against bin2base64, for all 4 variants
    for v in VARIANTS {
        for n in 0..=48usize {
            let bin = rng.bytes(n);
            let enc_len = unsafe { cel(n, v) };
            let mut b64 = vec![0u8; enc_len];
            unsafe {
                cb2b(b64.as_mut_ptr() as *mut c_char, enc_len, bin.as_ptr(), n, v);
            }
            let b64_len = enc_len - 1;
            for bin_maxlen in [n, n + 1, n.saturating_sub(1), 0] {
                for &ig in ignores.iter() {
                    for wbl in [false, true] {
                        for wbe in [false, true] {
                            let a = run_b642bin(c, bin_maxlen, &b64, b64_len, ig, wbl, wbe, v);
                            let b = run_b642bin(r, bin_maxlen, &b64, b64_len, ig, wbl, wbe, v);
                            let ctx = format!(
                                "b642bin rt v={v} n={n} maxlen={bin_maxlen} ig={:?} wbl={wbl} wbe={wbe}",
                                ig.map(String::from_utf8_lossy)
                            );
                            common::eqi(&ctx, a.ret, b.ret);
                            assert_eq!(a.errno, b.errno, "{ctx}: errno");
                            common::eqb(&ctx, &a.bin, &b.bin);
                            assert_eq!(a.bin_len, b.bin_len, "{ctx}: bin_len");
                            assert_eq!(a.b64_end, b.b64_end, "{ctx}: b64_end");
                        }
                    }
                }
            }
            // exact-fit decode must succeed and reproduce the input
            let a = run_b642bin(c, n, &b64, b64_len, None, true, true, v);
            assert_eq!(a.ret, 0, "b642bin roundtrip v={v} n={n} must succeed");
            assert_eq!(a.bin_len, n);
            assert_eq!(&a.bin[..n], &bin[..]);
        }
    }

    // 2) hand-written valid / invalid inputs
    let fixed: Vec<&[u8]> = vec![
        b"",
        b"=",
        b"==",
        b"A",
        b"AA",
        b"AA=",
        b"AA==",
        b"AB==",
        b"AAA",
        b"AAA=",
        b"AAAA",
        b"AAAAA",
        b"AAAB",
        b"/w==",
        b"_w==",
        b"-w==",
        b"+w==",
        b"////",
        b"____",
        b"++++",
        b"----",
        b"AAAA=",
        b"AAAA==",
        b"A===",
        b"AA=A",
        b"A A",
        b"A\nA",
        b"AA\x00==",
        b"!!!!",
        b"Zg==",
        b"Zm8=",
        b"Zm9v",
        b"Zm9vYg==",
        b"Zm9vYmE=",
        b"Zm9vYmFy",
        b"Zg",
        b"Zm8",
        b"Zm9vYg",
        b"Zm9vYmE",
        b"a=b=",
        b"====",
        b"AAAA====",
        b"\x80\x81",
        b"\xff",
    ];
    for v in VARIANTS {
        for b64 in fixed.iter() {
            for b64_len in [b64.len(), b64.len().saturating_sub(1)] {
                for &ig in ignores.iter() {
                    for bin_maxlen in [0usize, 1, 2, 3, 8] {
                        for wbl in [false, true] {
                            for wbe in [false, true] {
                                let a = run_b642bin(c, bin_maxlen, b64, b64_len, ig, wbl, wbe, v);
                                let b = run_b642bin(r, bin_maxlen, b64, b64_len, ig, wbl, wbe, v);
                                let ctx = format!(
                                    "b642bin v={v} b64={:?} len={b64_len} ig={:?} maxlen={bin_maxlen} wbl={wbl} wbe={wbe}",
                                    String::from_utf8_lossy(b64),
                                    ig.map(String::from_utf8_lossy)
                                );
                                common::eqi(&ctx, a.ret, b.ret);
                                assert_eq!(a.errno, b.errno, "{ctx}: errno");
                                common::eqb(&ctx, &a.bin, &b.bin);
                                assert_eq!(a.bin_len, b.bin_len, "{ctx}: bin_len");
                                assert_eq!(a.b64_end, b.b64_end, "{ctx}: b64_end");
                            }
                        }
                    }
                }
            }
        }
    }

    // 2b) exact errno values on the documented rejection sites
    for f in [c, r] {
        // bin_pos >= bin_maxlen -> ERANGE
        let a = run_b642bin(f, 1, b"AAAAAAAA", 8, None, true, true, 1);
        assert_eq!((a.ret, a.errno, a.bin_len), (-1, ERANGE, 0), "b642bin ERANGE");
        // truncated padding -> ERANGE from _sodium_base642bin_skip_padding
        let a = run_b642bin(f, 8, b"AA", 2, None, true, true, 1);
        assert_eq!((a.ret, a.errno, a.bin_len), (-1, ERANGE, 0), "b642bin padding ERANGE");
        // non-'=' where padding is expected -> EINVAL
        let a = run_b642bin(f, 8, b"AA!!", 4, None, true, true, 1);
        assert_eq!((a.ret, a.errno, a.bin_len), (-1, EINVAL, 0), "b642bin padding EINVAL");
        // leftover bits set -> ret -1, errno untouched
        set_errno(ERRNO_SENTINEL);
        let a = run_b642bin(f, 8, b"AB==", 4, None, true, true, 1);
        assert_eq!((a.ret, a.errno, a.bin_len), (-1, ERRNO_SENTINEL, 0), "b642bin leftover bits");
        // b64_end == NULL and trailing junk -> EINVAL
        // NB: as in hex2bin, `bin_pos` is zeroed before this late EINVAL, so
        // *bin_len still reports the decoded byte count.
        let a = run_b642bin(f, 8, b"AAAA!!", 6, None, true, false, 1);
        assert_eq!((a.ret, a.errno, a.bin_len), (-1, EINVAL, 3), "b642bin trailing EINVAL");
        // ...and with b64_end != NULL it succeeds, pointing at the junk
        let a = run_b642bin(f, 8, b"AAAA!!", 6, None, true, true, 1);
        assert_eq!((a.ret, a.bin_len, a.b64_end), (0, 3, 4), "b642bin trailing w/ b64_end");
        // NO_PADDING variants accept unpadded input
        let a = run_b642bin(f, 8, b"AAA", 3, None, true, true, 3);
        assert_eq!((a.ret, a.bin_len), (0, 2), "b642bin no-padding");
        let a = run_b642bin(f, 8, b"AAA", 3, None, true, true, 1);
        assert_eq!((a.ret, a.errno), (-1, ERANGE), "b642bin padding required");
    }

    // 3) randomized inputs over a mixed alphabet
    let alphabet: &[u8] = b"ABCZaz09+/-_=! \n\x00";
    for _ in 0..6000 {
        let n = rng.below(20);
        let b64: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let v = VARIANTS[rng.below(4)];
        let ig = ignores[rng.below(ignores.len())];
        let bin_maxlen = rng.below(12);
        let wbl = rng.below(2) == 0;
        let wbe = rng.below(2) == 0;
        let a = run_b642bin(c, bin_maxlen, &b64, n, ig, wbl, wbe, v);
        let b = run_b642bin(r, bin_maxlen, &b64, n, ig, wbl, wbe, v);
        let ctx = format!("b642bin rnd v={v} {:?}", String::from_utf8_lossy(&b64));
        common::eqi(&ctx, a.ret, b.ret);
        assert_eq!(a.errno, b.errno, "{ctx}: errno");
        common::eqb(&ctx, &a.bin, &b.bin);
        assert_eq!(a.bin_len, b.bin_len, "{ctx}: bin_len");
        assert_eq!(a.b64_end, b.b64_end, "{ctx}: b64_end");
    }
}

// ===========================================================================
// codecs.c -- ip2bin / bin2ip
// ===========================================================================

type FnIp2Bin = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
type FnBin2Ip = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;

#[test]
fn codecs_ip2bin() {
    let (c, r) = both!("sodium_ip2bin", FnIp2Bin);

    let cases: Vec<&[u8]> = vec![
        b"",
        b"0.0.0.0",
        b"1.2.3.4",
        b"127.0.0.1",
        b"255.255.255.255",
        b"010.020.030.040",
        b"1.2.3.04",
        b"0000.1.1.1",
        b"256.1.1.1",
        b"1.2.3",
        b"1.2.3.4.5",
        b"1.2.3.",
        b".1.2.3",
        b"1..2.3",
        b"1.2.3.4 ",
        b" 1.2.3.4",
        b"-1.2.3.4",
        b"1.2.3.4\x00extra",
        b"::",
        b":::",
        b"::1",
        b"1::",
        b"1:2:3:4:5:6:7:8",
        b"1:2:3:4:5:6:7:8:9",
        b"1:2:3:4:5:6:7",
        b"1:2:3:4:5:6:7::",
        b"::1:2:3:4:5:6:7:8",
        b"1::8",
        b"1:2::7:8",
        b"1:::2",
        b":1::",
        b":1",
        b"1:",
        b"12345::",
        b"1234::",
        b"g::1",
        b"::ffff:1.2.3.4",
        b"0:0:0:0:0:ffff:1.2.3.4",
        b"::ffff:0102:0304",
        b"::1.2.3.4",
        b"::1.2.3",
        b"1:2:3:4:5:6:1.2.3.4",
        b"1:2:3:4:5:6:7:1.2.3.4",
        b"fe80::1%eth0",
        b"fe80::1%",
        b"fe80::1%!",
        b"fe80::1%eth0%x",
        b"fe80::1%a.b-c_d",
        b"1.2.3.4%eth0",
        b"%eth0",
        b"::%1",
        b"ABCD:ef01::",
        b"abcd:EF01::",
        b"::0",
        b"0::",
        b"0:0:0:0:0:0:0:0",
        b"0:0:0:0:0:0:0:1",
        b"1:0:0:0:0:0:0:0",
        b"1:0:0:2:0:0:0:3",
        b"::.",
        b".",
        b":",
        b"::ffff:255.255.255.255",
        b"::ffff:256.0.0.1",
    ];

    for ip in cases {
        for &ip_len in [ip.len(), ip.len() + 1, ip.len().saturating_sub(1), 0, 3].iter() {
            if ip_len > ip.len() {
                // pass a NUL-terminated copy so reading up to ip_len is safe
                let mut z = ip.to_vec();
                z.push(0);
                let mut cb = vec![CANARY; 16 + CANARY_LEN];
                let mut rb = cb.clone();
                let (a, b) = unsafe {
                    (
                        c(cb.as_mut_ptr(), z.as_ptr() as *const c_char, ip_len),
                        r(rb.as_mut_ptr(), z.as_ptr() as *const c_char, ip_len),
                    )
                };
                let ctx = format!("ip2bin {:?} len={ip_len}", String::from_utf8_lossy(ip));
                common::eqi(&ctx, a, b);
                common::eqb(&ctx, &cb, &rb);
                continue;
            }
            let mut cb = vec![CANARY; 16 + CANARY_LEN];
            let mut rb = cb.clone();
            let (a, b) = unsafe {
                (
                    c(cb.as_mut_ptr(), ip.as_ptr() as *const c_char, ip_len),
                    r(rb.as_mut_ptr(), ip.as_ptr() as *const c_char, ip_len),
                )
            };
            let ctx = format!("ip2bin {:?} len={ip_len}", String::from_utf8_lossy(ip));
            common::eqi(&ctx, a, b);
            common::eqb(&ctx, &cb, &rb);
            assert!(cb[16..].iter().all(|&x| x == CANARY), "{ctx}: canary");
        }
    }

    // randomized fuzzing over an IP-ish alphabet
    let mut rng = common::Rng::new(0x5EED_0030);
    let alphabet: &[u8] = b"0123456789abcdefABCDEF.:%_-gz ";
    for _ in 0..20000 {
        let n = rng.below(24);
        let s: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        let mut cb = vec![CANARY; 16 + CANARY_LEN];
        let mut rb = cb.clone();
        let (a, b) = unsafe {
            (
                c(cb.as_mut_ptr(), s.as_ptr() as *const c_char, n),
                r(rb.as_mut_ptr(), s.as_ptr() as *const c_char, n),
            )
        };
        let ctx = format!("ip2bin rnd {:?}", String::from_utf8_lossy(&s));
        common::eqi(&ctx, a, b);
        common::eqb(&ctx, &cb, &rb);
    }
}

#[test]
fn codecs_bin2ip() {
    let (c, r) = both!("sodium_bin2ip", FnBin2Ip);
    let (cip2bin, _) = both!("sodium_ip2bin", FnIp2Bin);
    let mut rng = common::Rng::new(0x5EED_0031);

    let mut bins: Vec<[u8; 16]> = Vec::new();
    bins.push([0u8; 16]);
    bins.push([0xffu8; 16]);
    {
        let mut b = [0u8; 16];
        b[15] = 1;
        bins.push(b);
    }
    {
        // ipv4-mapped
        for last in [[0u8, 0, 0, 0], [1, 2, 3, 4], [255, 255, 255, 255], [10, 0, 0, 1]] {
            let mut b = [0u8; 16];
            b[10] = 0xff;
            b[11] = 0xff;
            b[12..].copy_from_slice(&last);
            bins.push(b);
        }
    }
    {
        // almost-ipv4-mapped (prefix differs in one byte)
        for i in 0..12 {
            let mut b = [0u8; 16];
            b[10] = 0xff;
            b[11] = 0xff;
            b[12..].copy_from_slice(&[1, 2, 3, 4]);
            b[i] ^= 0x01;
            bins.push(b);
        }
    }
    // every single-word-set pattern, and zero runs of every length/offset
    for w in 0..8 {
        let mut b = [0u8; 16];
        b[w * 2] = 0x12;
        b[w * 2 + 1] = 0x34;
        bins.push(b);
    }
    for start in 0..8 {
        for len in 1..=(8 - start) {
            let mut b = [0x11u8; 16];
            for w in start..start + len {
                b[w * 2] = 0;
                b[w * 2 + 1] = 0;
            }
            bins.push(b);
        }
    }
    // two equal-length zero runs (tie-break must match)
    for (s1, s2) in [(0usize, 3usize), (1, 4), (2, 5), (0, 6), (3, 6)] {
        let mut b = [0x11u8; 16];
        for w in [s1, s1 + 1, s2, s2 + 1] {
            if w < 8 {
                b[w * 2] = 0;
                b[w * 2 + 1] = 0;
            }
        }
        bins.push(b);
    }
    for _ in 0..500 {
        let v = rng.bytes(16);
        let mut b = [0u8; 16];
        b.copy_from_slice(&v);
        bins.push(b);
        // sparse patterns (lots of zero words)
        let mut b2 = [0u8; 16];
        for w in 0..8 {
            if rng.below(3) == 0 {
                b2[w * 2] = rng.u8();
                b2[w * 2 + 1] = rng.u8();
            }
        }
        bins.push(b2);
    }

    for bin in bins.iter() {
        for ip_maxlen in [0usize, 1, 2, 3, 4, 5, 8, 16, 40, 46, 64] {
            let mut cb = vec![CANARY; ip_maxlen + CANARY_LEN];
            let mut rb = cb.clone();
            let (cp, rp) = unsafe {
                (
                    c(cb.as_mut_ptr() as *mut c_char, ip_maxlen, bin.as_ptr()),
                    r(rb.as_mut_ptr() as *mut c_char, ip_maxlen, bin.as_ptr()),
                )
            };
            let ctx = format!("bin2ip {} maxlen={ip_maxlen}", common::hex(bin));
            assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: NULL-ness");
            if !cp.is_null() {
                assert_eq!(cp as *const u8, cb.as_ptr(), "{ctx}: must return `ip`");
                assert_eq!(rp as *const u8, rb.as_ptr(), "{ctx}: must return `ip`");
            }
            common::eqb(&ctx, &cb, &rb);
            assert!(cb[ip_maxlen..].iter().all(|&x| x == CANARY), "{ctx}: canary");
        }
        // round-trip through ip2bin with a generous buffer
        let mut buf = vec![0u8; 64];
        let p = unsafe { c(buf.as_mut_ptr() as *mut c_char, 64, bin.as_ptr()) };
        assert!(!p.is_null());
        let s: Vec<u8> = buf.iter().copied().take_while(|&b| b != 0).collect();
        let mut back = [0u8; 16];
        let rc = unsafe { cip2bin(back.as_mut_ptr(), s.as_ptr() as *const c_char, s.len()) };
        assert_eq!(rc, 0, "bin2ip output {:?} must re-parse", String::from_utf8_lossy(&s));
        assert_eq!(&back[..], &bin[..], "bin2ip/ip2bin round-trip {:?}", String::from_utf8_lossy(&s));
    }
}

// ===========================================================================
// core.c / runtime.c / version.c
// ===========================================================================

#[test]
fn core_init_and_crit() {
    // sodium_init() has already been called once by the harness -> idempotent
    // subsequent calls must return 1 in both libraries.
    let (c, r) = both!("sodium_init", unsafe extern "C" fn() -> c_int);
    for i in 0..3 {
        let (a, b) = unsafe { (c(), r()) };
        common::eqi(&format!("sodium_init #{i}"), a, b);
        assert_eq!(a, 1, "sodium_init must return 1 once initialized");
    }

    let (c, r) = both!("sodium_crit_enter", unsafe extern "C" fn() -> c_int);
    let (c2, r2) = both!("sodium_crit_leave", unsafe extern "C" fn() -> c_int);
    for _ in 0..3 {
        let (a, b) = unsafe { (c(), r()) };
        common::eqi("sodium_crit_enter", a, b);
        assert_eq!(a, 0);
        let (a, b) = unsafe { (c2(), r2()) };
        common::eqi("sodium_crit_leave", a, b);
        assert_eq!(a, 0);
    }
    // unbalanced leave is also a no-op returning 0 in this configuration
    let (a, b) = unsafe { (c2(), r2()) };
    common::eqi("sodium_crit_leave unbalanced", a, b);
    assert_eq!(a, 0);
}

extern "C" fn dummy_misuse_handler() {}

extern "C" fn exiting_misuse_handler() {
    unsafe { _exit(42) }
}

#[test]
fn core_set_misuse_handler() {
    let _g = STATE_LOCK.lock().unwrap();
    let (c, r) = both!(
        "sodium_set_misuse_handler",
        unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int
    );
    unsafe {
        // NULL handler
        let (a, b) = (c(None), r(None));
        common::eqi("set_misuse_handler(NULL)", a, b);
        assert_eq!(a, 0);
        // real handler (never triggered in-process)
        let (a, b) = (
            c(Some(dummy_misuse_handler)),
            r(Some(dummy_misuse_handler)),
        );
        common::eqi("set_misuse_handler(fn)", a, b);
        assert_eq!(a, 0);
        // idempotent / re-settable, then reset to NULL for the abort tests
        let (a, b) = (c(None), r(None));
        common::eqi("set_misuse_handler(NULL) again", a, b);
        assert_eq!(a, 0);
    }

    // The handler really is invoked by sodium_misuse() -- verified in a forked
    // child (sodium_misuse() abort()s unconditionally afterwards).
    let l = common::libs();
    for (name, lib) in [("C", &l.c), ("Rust", &l.r)] {
        let set = getsym!(
            lib,
            "sodium_set_misuse_handler",
            unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int
        );
        let pad = getsym!(lib, "sodium_pad", FnPad);
        let st = child_status(move || unsafe {
            set(Some(exiting_misuse_handler));
            let mut out = 0usize;
            let mut b = [0u8; 1];
            // triggers the SIZE_MAX overflow -> sodium_misuse()
            pad(&mut out, b.as_mut_ptr(), usize::MAX, 16, usize::MAX);
        });
        assert_eq!(
            wait_exit_code(st),
            Some(42),
            "{name}: misuse handler was not called (status {st:#x})"
        );
    }
}

#[test]
fn runtime_features() {
    let names = [
        "sodium_runtime_has_neon",
        "sodium_runtime_has_armcrypto",
        "sodium_runtime_has_sse2",
        "sodium_runtime_has_sse3",
        "sodium_runtime_has_ssse3",
        "sodium_runtime_has_sse41",
        "sodium_runtime_has_avx",
        "sodium_runtime_has_avx2",
        "sodium_runtime_has_avx512f",
        "sodium_runtime_has_pclmul",
        "sodium_runtime_has_aesni",
        "sodium_runtime_has_rdrand",
    ];
    let l = common::libs();
    for n in names {
        let c: unsafe extern "C" fn() -> c_int = sym(&l.c, n);
        let r: unsafe extern "C" fn() -> c_int = sym(&l.r, n);
        let (a, b) = unsafe { (c(), r()) };
        common::eqi(n, a, b);
    }

    // re-running the detection must be idempotent and return the same value
    let (c, r) = both!("_sodium_runtime_get_cpu_features", unsafe extern "C" fn() -> c_int);
    for _ in 0..3 {
        let (a, b) = unsafe { (c(), r()) };
        common::eqi("_sodium_runtime_get_cpu_features", a, b);
    }
    for n in names {
        let c: unsafe extern "C" fn() -> c_int = sym(&l.c, n);
        let r: unsafe extern "C" fn() -> c_int = sym(&l.r, n);
        let (a, b) = unsafe { (c(), r()) };
        common::eqi(n, a, b);
    }
}

#[test]
fn version_functions() {
    let (c, r) = both!("sodium_version_string", unsafe extern "C" fn() -> *const c_char);
    unsafe {
        let cs = std::ffi::CStr::from_ptr(c());
        let rs = std::ffi::CStr::from_ptr(r());
        assert_eq!(cs, rs, "sodium_version_string");
    }
    for n in [
        "sodium_library_version_major",
        "sodium_library_version_minor",
        "sodium_library_minimal",
    ] {
        let l = common::libs();
        let c: unsafe extern "C" fn() -> c_int = sym(&l.c, n);
        let r: unsafe extern "C" fn() -> c_int = sym(&l.r, n);
        let (a, b) = unsafe { (c(), r()) };
        common::eqi(n, a, b);
    }
}

// ===========================================================================
// randombytes
// ===========================================================================

#[repr(C)]
#[derive(Copy, Clone)]
struct RbImpl {
    implementation_name: Option<unsafe extern "C" fn() -> *const c_char>,
    random: Option<unsafe extern "C" fn() -> u32>,
    stir: Option<unsafe extern "C" fn()>,
    uniform: Option<unsafe extern "C" fn(u32) -> u32>,
    buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    close: Option<unsafe extern "C" fn() -> c_int>,
}

struct RbApi {
    set_impl: unsafe extern "C" fn(*const RbImpl) -> c_int,
    name: unsafe extern "C" fn() -> *const c_char,
    random: unsafe extern "C" fn() -> u32,
    stir: unsafe extern "C" fn(),
    uniform: unsafe extern "C" fn(u32) -> u32,
    buf: unsafe extern "C" fn(*mut c_void, usize),
    buf_det: unsafe extern "C" fn(*mut c_void, usize, *const u8),
    seedbytes: unsafe extern "C" fn() -> usize,
    close: unsafe extern "C" fn() -> c_int,
    randombytes: unsafe extern "C" fn(*mut u8, u64),
}

fn rb_api(lib: &libloading::Library) -> RbApi {
    RbApi {
        set_impl: getsym!(lib, "randombytes_set_implementation", unsafe extern "C" fn(*const RbImpl) -> c_int),
        name: getsym!(lib, "randombytes_implementation_name", unsafe extern "C" fn() -> *const c_char),
        random: getsym!(lib, "randombytes_random", unsafe extern "C" fn() -> u32),
        stir: getsym!(lib, "randombytes_stir", unsafe extern "C" fn()),
        uniform: getsym!(lib, "randombytes_uniform", unsafe extern "C" fn(u32) -> u32),
        buf: getsym!(lib, "randombytes_buf", unsafe extern "C" fn(*mut c_void, usize)),
        buf_det: getsym!(lib, "randombytes_buf_deterministic", unsafe extern "C" fn(*mut c_void, usize, *const u8)),
        seedbytes: getsym!(lib, "randombytes_seedbytes", unsafe extern "C" fn() -> usize),
        close: getsym!(lib, "randombytes_close", unsafe extern "C" fn() -> c_int),
        randombytes: getsym!(lib, "randombytes", unsafe extern "C" fn(*mut u8, u64)),
    }
}

// ---- a fully deterministic test RNG implementation, with one independent
// ---- counter per library (slot 0 = C, slot 1 = Rust)

static mut TEST_CTR: [u64; 2] = [0, 0];
static mut TEST_STIRS: [u64; 2] = [0, 0];

unsafe fn tnext(slot: usize) -> u64 {
    TEST_CTR[slot] = TEST_CTR[slot].wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = TEST_CTR[slot];
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

unsafe extern "C" fn t_name() -> *const c_char {
    b"testrng\0".as_ptr() as *const c_char
}
unsafe extern "C" fn t_random_0() -> u32 {
    tnext(0) as u32
}
unsafe extern "C" fn t_random_1() -> u32 {
    tnext(1) as u32
}
unsafe extern "C" fn t_stir_0() {
    TEST_CTR[0] = 0x1234_5678;
    TEST_STIRS[0] += 1;
}
unsafe extern "C" fn t_stir_1() {
    TEST_CTR[1] = 0x1234_5678;
    TEST_STIRS[1] += 1;
}
unsafe fn t_fill(slot: usize, buf: *mut c_void, size: usize) {
    let p = buf as *mut u8;
    for i in 0..size {
        *p.add(i) = tnext(slot) as u8;
    }
}
unsafe extern "C" fn t_buf_0(buf: *mut c_void, size: usize) {
    t_fill(0, buf, size)
}
unsafe extern "C" fn t_buf_1(buf: *mut c_void, size: usize) {
    t_fill(1, buf, size)
}
unsafe extern "C" fn t_close_0() -> c_int {
    TEST_CTR[0] = 0xdead;
    17
}
unsafe extern "C" fn t_close_1() -> c_int {
    TEST_CTR[1] = 0xdead;
    17
}
unsafe extern "C" fn t_uniform_0(ub: u32) -> u32 {
    ub ^ (tnext(0) as u32)
}
unsafe extern "C" fn t_uniform_1(ub: u32) -> u32 {
    ub ^ (tnext(1) as u32)
}

/// Implementation with `uniform == NULL`, `stir`/`close` present: exercises the
/// rejection-sampling fallback in `randombytes_uniform`.
fn impl_no_uniform(slot: usize) -> RbImpl {
    RbImpl {
        implementation_name: Some(t_name),
        random: Some(if slot == 0 { t_random_0 } else { t_random_1 }),
        stir: Some(if slot == 0 { t_stir_0 } else { t_stir_1 }),
        uniform: None,
        buf: Some(if slot == 0 { t_buf_0 } else { t_buf_1 }),
        close: Some(if slot == 0 { t_close_0 } else { t_close_1 }),
    }
}

/// Implementation with `uniform != NULL`: exercises the delegation branch.
fn impl_with_uniform(slot: usize) -> RbImpl {
    let mut i = impl_no_uniform(slot);
    i.uniform = Some(if slot == 0 { t_uniform_0 } else { t_uniform_1 });
    i
}

/// Implementation with `stir == NULL` and `close == NULL`: exercises the
/// NULL-pointer branches of `randombytes_stir` / `randombytes_close`.
fn impl_no_stir_close(slot: usize) -> RbImpl {
    let mut i = impl_no_uniform(slot);
    i.stir = None;
    i.close = None;
    i
}

const UBS: [u32; 14] = [
    0,
    1,
    2,
    3,
    5,
    16,
    17,
    255,
    256,
    1000,
    0x8000_0000,
    0x8000_0001,
    0xffff_fffe,
    0xffff_ffff,
];

/// Runs an identical, fully deterministic sequence of `randombytes_*` calls on
/// one library and returns a transcript.
fn rb_transcript(api: &RbApi, slot: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    unsafe {
        TEST_CTR[slot] = 0xC0FF_EE00;
        TEST_STIRS[slot] = 0;

        // --- implementation with uniform == NULL
        let im = impl_no_uniform(slot);
        out.push(format!("set={}", (api.set_impl)(&im)));
        out.push(format!(
            "name={}",
            std::ffi::CStr::from_ptr((api.name)()).to_string_lossy()
        ));
        out.push(format!("seedbytes={}", (api.seedbytes)()));
        TEST_CTR[slot] = 0xC0FF_EE00;
        for _ in 0..64 {
            out.push(format!("rnd={}", (api.random)()));
        }
        TEST_CTR[slot] = 0xC0FF_EE00;
        for &ub in UBS.iter() {
            for _ in 0..8 {
                out.push(format!("uni({ub})={}", (api.uniform)(ub)));
            }
        }
        TEST_CTR[slot] = 0xC0FF_EE00;
        for n in [0usize, 1, 2, 15, 16, 17, 63, 64, 100] {
            let mut b = vec![0xEEu8; n + 8];
            (api.buf)(b.as_mut_ptr() as *mut c_void, n);
            out.push(format!("buf({n})={}", common::hex(&b)));
            let mut b = vec![0xEEu8; n + 8];
            (api.randombytes)(b.as_mut_ptr(), n as u64);
            out.push(format!("rb({n})={}", common::hex(&b)));
        }
        (api.stir)();
        out.push(format!("stirs={}", TEST_STIRS[slot]));
        out.push(format!("ctr_after_stir={}", TEST_CTR[slot]));
        out.push(format!("close={}", (api.close)()));

        // --- implementation with uniform != NULL (delegation)
        let im2 = impl_with_uniform(slot);
        out.push(format!("set2={}", (api.set_impl)(&im2)));
        TEST_CTR[slot] = 0xBEEF_0000;
        for &ub in UBS.iter() {
            for _ in 0..4 {
                out.push(format!("uni2({ub})={}", (api.uniform)(ub)));
            }
        }

        // --- implementation with stir == NULL / close == NULL
        let im3 = impl_no_stir_close(slot);
        out.push(format!("set3={}", (api.set_impl)(&im3)));
        TEST_CTR[slot] = 0x0BAD_0000;
        let before = TEST_CTR[slot];
        (api.stir)();
        out.push(format!("stir_null_noop={}", TEST_CTR[slot] == before));
        out.push(format!("close_null={}", (api.close)()));
        out.push(format!(
            "name3={}",
            std::ffi::CStr::from_ptr((api.name)()).to_string_lossy()
        ));

        // --- implementation == NULL: randombytes_close() returns 0 without
        //     dereferencing, and the next call re-installs the default
        //     (sysrandom) implementation via randombytes_init_if_needed().
        out.push(format!("set_null={}", (api.set_impl)(core::ptr::null())));
        out.push(format!("close_after_null={}", (api.close)()));
        out.push(format!(
            "name_after_null={}",
            std::ffi::CStr::from_ptr((api.name)()).to_string_lossy()
        ));
    }
    out
}

#[test]
fn randombytes_custom_implementation() {
    let _g = STATE_LOCK.lock().unwrap();
    let l = common::libs();
    let capi = rb_api(&l.c);
    let rapi = rb_api(&l.r);

    let ct = rb_transcript(&capi, 0);
    let rt = rb_transcript(&rapi, 1);
    assert_eq!(ct.len(), rt.len(), "transcript length");
    for (i, (a, b)) in ct.iter().zip(rt.iter()).enumerate() {
        assert_eq!(a, b, "randombytes transcript entry #{i}");
    }

    // restore each library's own default implementation
    let (cimpl, rimpl) = both_data!("randombytes_sysrandom_implementation", RbImpl);
    unsafe {
        (capi.set_impl)(cimpl);
        (rapi.set_impl)(rimpl);
        assert_eq!(
            std::ffi::CStr::from_ptr((capi.name)()).to_bytes(),
            b"sysrandom"
        );
        assert_eq!(
            std::ffi::CStr::from_ptr((rapi.name)()).to_bytes(),
            b"sysrandom"
        );
    }
}

#[test]
fn randombytes_buf_deterministic_bytes() {
    let (c, r) = both!(
        "randombytes_buf_deterministic",
        unsafe extern "C" fn(*mut c_void, usize, *const u8)
    );
    let (csb, rsb) = both!("randombytes_seedbytes", unsafe extern "C" fn() -> usize);
    unsafe {
        let (a, b) = (csb(), rsb());
        assert_eq!(a, b, "randombytes_seedbytes");
        assert_eq!(a, 32);
    }

    let mut rng = common::Rng::new(0x5EED_0040);
    for trial in 0..40 {
        let seed = if trial == 0 {
            vec![0u8; 32]
        } else if trial == 1 {
            vec![0xffu8; 32]
        } else {
            rng.bytes(32)
        };
        for n in [0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000] {
            let mut cb = vec![CANARY; n + CANARY_LEN];
            let mut rb = cb.clone();
            unsafe {
                c(cb.as_mut_ptr() as *mut c_void, n, seed.as_ptr());
                r(rb.as_mut_ptr() as *mut c_void, n, seed.as_ptr());
            }
            common::eqb(
                &format!("buf_deterministic n={n} seed={}", common::hex(&seed)),
                &cb,
                &rb,
            );
            assert!(cb[n..].iter().all(|&x| x == CANARY), "canary");
        }
    }
}

#[test]
fn randombytes_default_and_exported_impls() {
    let _g = STATE_LOCK.lock().unwrap();
    let l = common::libs();
    let capi = rb_api(&l.c);
    let rapi = rb_api(&l.r);

    // The default implementation is `sysrandom` in both libraries.
    unsafe {
        (capi.set_impl)(core::ptr::null());
        (rapi.set_impl)(core::ptr::null());
        let cn = std::ffi::CStr::from_ptr((capi.name)()).to_owned();
        let rn = std::ffi::CStr::from_ptr((rapi.name)()).to_owned();
        assert_eq!(cn, rn, "default implementation_name");
        assert_eq!(cn.to_bytes(), b"sysrandom");
    }

    // Exercise every function pointer of the two exported implementation
    // objects directly, in an identical order for both libraries. The RNG
    // output itself is nondeterministic, so only structure / return codes are
    // compared.
    for (label, symbol, expect_name) in [
        (
            "sysrandom",
            "randombytes_sysrandom_implementation",
            &b"sysrandom"[..],
        ),
        (
            "internal",
            "randombytes_internal_implementation",
            &b"internal"[..],
        ),
    ] {
        let cimpl: *const RbImpl = unsafe {
            let s: libloading::Symbol<*const RbImpl> =
                l.c.get(format!("{symbol}\0").as_bytes()).unwrap();
            *s
        };
        let rimpl: *const RbImpl = unsafe {
            let s: libloading::Symbol<*const RbImpl> =
                l.r.get(format!("{symbol}\0").as_bytes()).unwrap();
            *s
        };
        unsafe {
            let ci = &*cimpl;
            let ri = &*rimpl;
            // implementation_name
            let cn = std::ffi::CStr::from_ptr((ci.implementation_name.unwrap())()).to_owned();
            let rn = std::ffi::CStr::from_ptr((ri.implementation_name.unwrap())()).to_owned();
            assert_eq!(cn, rn, "{label}: implementation_name");
            assert_eq!(cn.to_bytes(), expect_name, "{label}: implementation_name");
            // uniform is NULL in both
            assert!(ci.uniform.is_none(), "{label}: C uniform must be NULL");
            assert!(ri.uniform.is_none(), "{label}: Rust uniform must be NULL");
            assert!(ci.random.is_some() && ri.random.is_some());
            assert!(ci.stir.is_some() && ri.stir.is_some());
            assert!(ci.buf.is_some() && ri.buf.is_some());
            assert!(ci.close.is_some() && ri.close.is_some());

            // close() before any stir(): sysrandom has already been stirred by
            // sodium_init(), the internal RNG has not -> deterministic codes.
            let a = (ci.close.unwrap())();
            let b = (ri.close.unwrap())();
            common::eqi(&format!("{label}: close() #1"), a, b);

            // stir(), then use it
            (ci.stir.unwrap())();
            (ri.stir.unwrap())();
            let _ = (ci.random.unwrap())();
            let _ = (ri.random.unwrap())();
            for n in [1usize, 16, 31, 32, 33, 256, 257, 1000] {
                let mut cb = vec![0u8; n];
                let mut rb = vec![0u8; n];
                (ci.buf.unwrap())(cb.as_mut_ptr() as *mut c_void, n);
                (ri.buf.unwrap())(rb.as_mut_ptr() as *mut c_void, n);
                // nondeterministic: only require that something was produced
                assert_eq!(cb.len(), rb.len());
            }
            // 512 draws must not be constant (sanity check that the RNG runs)
            let mut seen = std::collections::HashSet::new();
            for _ in 0..512 {
                seen.insert((ci.random.unwrap())());
                seen.insert((ri.random.unwrap())());
            }
            assert!(seen.len() > 500, "{label}: RNG output looks constant");

            let a = (ci.close.unwrap())();
            let b = (ri.close.unwrap())();
            common::eqi(&format!("{label}: close() #2"), a, b);
            let a = (ci.close.unwrap())();
            let b = (ri.close.unwrap())();
            common::eqi(&format!("{label}: close() #3"), a, b);

            // install via randombytes_set_implementation and go through the
            // public API
            let a = (capi.set_impl)(cimpl);
            let b = (rapi.set_impl)(rimpl);
            common::eqi(&format!("{label}: set_implementation"), a, b);
            let cn = std::ffi::CStr::from_ptr((capi.name)()).to_owned();
            let rn = std::ffi::CStr::from_ptr((rapi.name)()).to_owned();
            assert_eq!(cn, rn, "{label}: name via public API");
            assert_eq!(cn.to_bytes(), expect_name);
            (capi.stir)();
            (rapi.stir)();
            let a = (capi.close)();
            let b = (rapi.close)();
            common::eqi(&format!("{label}: randombytes_close"), a, b);
            let a = (capi.seedbytes)();
            let b = (rapi.seedbytes)();
            assert_eq!(a, b);

            // randombytes_uniform structural properties (values are random)
            for &ub in UBS.iter() {
                for _ in 0..64 {
                    let x = (capi.uniform)(ub);
                    let y = (rapi.uniform)(ub);
                    if ub < 2 {
                        assert_eq!(x, 0, "{label}: uniform({ub}) must be 0 (C)");
                        assert_eq!(y, 0, "{label}: uniform({ub}) must be 0 (Rust)");
                    } else {
                        assert!(x < ub, "{label}: C uniform({ub}) = {x} out of range");
                        assert!(y < ub, "{label}: Rust uniform({ub}) = {y} out of range");
                    }
                }
            }
            // randombytes_buf / randombytes: size 0 must not touch the buffer
            let mut cb = [0xAAu8; 8];
            let mut rb = [0xAAu8; 8];
            (capi.buf)(cb.as_mut_ptr() as *mut c_void, 0);
            (rapi.buf)(rb.as_mut_ptr() as *mut c_void, 0);
            common::eqb(&format!("{label}: buf(0)"), &cb, &rb);
            assert_eq!(cb, [0xAAu8; 8]);
            (capi.randombytes)(cb.as_mut_ptr(), 0);
            (rapi.randombytes)(rb.as_mut_ptr(), 0);
            common::eqb(&format!("{label}: randombytes(0)"), &cb, &rb);
            assert_eq!(cb, [0xAAu8; 8]);
        }
    }

    // restore the default implementation in both libraries
    let (cimpl, rimpl) = both_data!("randombytes_sysrandom_implementation", RbImpl);
    unsafe {
        (capi.set_impl)(cimpl);
        (rapi.set_impl)(rimpl);
        (capi.stir)();
        (rapi.stir)();
    }
}

/// `randombytes_sysrandom_implementation.buf(p, 0)` reaches
/// `randombytes_linux_getrandom(p, 0)`, whose `assert(chunk_size > 0U)` is live
/// (the reference C build defines no `NDEBUG`) and therefore aborts. The
/// `internal` implementation has no such assertion for size 0. This is only
/// reachable by calling the implementation struct directly -- `randombytes_buf()`
/// guards `size > 0`.
#[test]
fn randombytes_impl_buf_zero_assert() {
    let l = common::libs();
    for (lname, lib) in [("C", &l.c), ("Rust", &l.r)] {
        let sysimpl: *const RbImpl = sym(lib, "randombytes_sysrandom_implementation");
        let intimpl: *const RbImpl = sym(lib, "randombytes_internal_implementation");
        let sysimpl = sysimpl as usize;
        let intimpl = intimpl as usize;
        assert_aborts(&format!("{lname}: sysrandom.buf(p, 0)"), move || unsafe {
            let i = &*(sysimpl as *const RbImpl);
            let mut b = [0u8; 8];
            (i.buf.unwrap())(b.as_mut_ptr() as *mut c_void, 0);
        });
        let st = child_status(move || unsafe {
            let i = &*(intimpl as *const RbImpl);
            let mut b = [0u8; 8];
            (i.buf.unwrap())(b.as_mut_ptr() as *mut c_void, 0);
        });
        assert_eq!(
            wait_exit_code(st),
            Some(77),
            "{lname}: internal.buf(p, 0) must return normally (status {st:#x})"
        );
    }
}

// ===========================================================================
// crypto_ipcrypt
// ===========================================================================

#[repr(C)]
#[derive(Copy, Clone)]
struct IpcryptImpl {
    encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    nd_encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8),
    nd_decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    ndx_encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8),
    ndx_decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    pfx_encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    pfx_decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
}

#[test]
fn ipcrypt_constants() {
    for (n, want) in [
        ("crypto_ipcrypt_bytes", 16usize),
        ("crypto_ipcrypt_keybytes", 16),
        ("crypto_ipcrypt_nd_keybytes", 16),
        ("crypto_ipcrypt_nd_tweakbytes", 8),
        ("crypto_ipcrypt_nd_inputbytes", 16),
        ("crypto_ipcrypt_nd_outputbytes", 24),
        ("crypto_ipcrypt_ndx_keybytes", 32),
        ("crypto_ipcrypt_ndx_tweakbytes", 16),
        ("crypto_ipcrypt_ndx_inputbytes", 16),
        ("crypto_ipcrypt_ndx_outputbytes", 32),
        ("crypto_ipcrypt_pfx_keybytes", 32),
        ("crypto_ipcrypt_pfx_bytes", 16),
    ] {
        let l = common::libs();
        let c: unsafe extern "C" fn() -> usize = sym(&l.c, n);
        let r: unsafe extern "C" fn() -> usize = sym(&l.r, n);
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{n}");
        assert_eq!(a, want, "{n} value");
    }
}

/// Interesting 16-byte "IP" inputs (ipv4-mapped and not).
fn ipcrypt_inputs(rng: &mut common::Rng) -> Vec<[u8; 16]> {
    let mut v: Vec<[u8; 16]> = Vec::new();
    v.push([0u8; 16]);
    v.push([0xffu8; 16]);
    {
        let mut b = [0u8; 16];
        b[15] = 1;
        v.push(b);
    }
    for last in [
        [0u8, 0, 0, 0],
        [1, 2, 3, 4],
        [127, 0, 0, 1],
        [255, 255, 255, 255],
        [192, 168, 1, 1],
    ] {
        let mut b = [0u8; 16];
        b[10] = 0xff;
        b[11] = 0xff;
        b[12..].copy_from_slice(&last);
        v.push(b); // ipv4-mapped -> prefix_start = 96 in pfx
    }
    // near-miss on the mapped prefix -> prefix_start = 0
    for i in 0..12 {
        let mut b = [0u8; 16];
        b[10] = 0xff;
        b[11] = 0xff;
        b[12..].copy_from_slice(&[1, 2, 3, 4]);
        b[i] ^= 0x80;
        v.push(b);
    }
    for _ in 0..24 {
        let mut b = [0u8; 16];
        b.copy_from_slice(&rng.bytes(16));
        v.push(b);
    }
    v
}

#[test]
fn ipcrypt_kat_and_roundtrip() {
    let l = common::libs();
    let ce = getsym!(l.c, "crypto_ipcrypt_encrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));
    let re = getsym!(l.r, "crypto_ipcrypt_encrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));
    let cd = getsym!(l.c, "crypto_ipcrypt_decrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));
    let rd = getsym!(l.r, "crypto_ipcrypt_decrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));

    let mut rng = common::Rng::new(0x5EED_0050);
    let inputs = ipcrypt_inputs(&mut rng);
    let mut keys: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16]];
    for _ in 0..24 {
        keys.push(rng.bytes(16));
    }

    for k in keys.iter() {
        for inp in inputs.iter() {
            let mut cb = vec![CANARY; 16 + CANARY_LEN];
            let mut rb = cb.clone();
            unsafe {
                ce(cb.as_mut_ptr(), inp.as_ptr(), k.as_ptr());
                re(rb.as_mut_ptr(), inp.as_ptr(), k.as_ptr());
            }
            let ctx = format!("ipcrypt_encrypt k={} in={}", common::hex(k), common::hex(inp));
            common::eqb(&ctx, &cb, &rb);
            assert!(cb[16..].iter().all(|&x| x == CANARY), "{ctx}: canary");

            let mut cd2 = vec![CANARY; 16 + CANARY_LEN];
            let mut rd2 = cd2.clone();
            unsafe {
                cd(cd2.as_mut_ptr(), cb.as_ptr(), k.as_ptr());
                rd(rd2.as_mut_ptr(), rb.as_ptr(), k.as_ptr());
            }
            common::eqb(&format!("ipcrypt_decrypt {ctx}"), &cd2, &rd2);
            assert_eq!(&cd2[..16], &inp[..], "{ctx}: round-trip");

            // in-place aliasing (out == in)
            let mut cb2 = inp.to_vec();
            let mut rb2 = inp.to_vec();
            unsafe {
                let p = cb2.as_mut_ptr();
                ce(p, p, k.as_ptr());
                let p = rb2.as_mut_ptr();
                re(p, p, k.as_ptr());
            }
            common::eqb(&format!("{ctx} in-place"), &cb2, &rb2);
            assert_eq!(&cb2[..], &cb[..16]);
        }
    }
}

#[test]
fn ipcrypt_nd_ndx_pfx() {
    let l = common::libs();
    macro_rules! sym3 {
        ($n:expr) => {
            (
                getsym!(l.c, $n, unsafe extern "C" fn(*mut u8, *const u8, *const u8)),
                getsym!(l.r, $n, unsafe extern "C" fn(*mut u8, *const u8, *const u8)),
            )
        };
    }
    macro_rules! sym4 {
        ($n:expr) => {
            (
                getsym!(l.c, $n, unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8)),
                getsym!(l.r, $n, unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8)),
            )
        };
    }
    let (cnde, rnde) = sym4!("crypto_ipcrypt_nd_encrypt");
    let (cndd, rndd) = sym3!("crypto_ipcrypt_nd_decrypt");
    let (cnxe, rnxe) = sym4!("crypto_ipcrypt_ndx_encrypt");
    let (cnxd, rnxd) = sym3!("crypto_ipcrypt_ndx_decrypt");
    let (cpfe, rpfe) = sym3!("crypto_ipcrypt_pfx_encrypt");
    let (cpfd, rpfd) = sym3!("crypto_ipcrypt_pfx_decrypt");

    let mut rng = common::Rng::new(0x5EED_0051);
    let inputs = ipcrypt_inputs(&mut rng);

    // nd: 16-byte key, 8-byte tweak, 16 -> 24 bytes
    let mut nd_keys: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16]];
    for _ in 0..16 {
        nd_keys.push(rng.bytes(16));
    }
    let mut tweaks8: Vec<Vec<u8>> = vec![vec![0u8; 8], vec![0xffu8; 8]];
    for _ in 0..8 {
        tweaks8.push(rng.bytes(8));
    }
    for k in nd_keys.iter() {
        for t in tweaks8.iter() {
            for inp in inputs.iter().take(20) {
                let mut cb = vec![CANARY; 24 + CANARY_LEN];
                let mut rb = cb.clone();
                unsafe {
                    cnde(cb.as_mut_ptr(), inp.as_ptr(), t.as_ptr(), k.as_ptr());
                    rnde(rb.as_mut_ptr(), inp.as_ptr(), t.as_ptr(), k.as_ptr());
                }
                let ctx = format!("nd_encrypt k={} t={}", common::hex(k), common::hex(t));
                common::eqb(&ctx, &cb, &rb);
                assert!(cb[24..].iter().all(|&x| x == CANARY), "{ctx}: canary");
                assert_eq!(&cb[..8], &t[..], "{ctx}: tweak must be prefixed");

                let mut cd = vec![CANARY; 16 + CANARY_LEN];
                let mut rd = cd.clone();
                unsafe {
                    cndd(cd.as_mut_ptr(), cb.as_ptr(), k.as_ptr());
                    rndd(rd.as_mut_ptr(), rb.as_ptr(), k.as_ptr());
                }
                common::eqb(&format!("nd_decrypt {ctx}"), &cd, &rd);
                assert_eq!(&cd[..16], &inp[..], "{ctx}: nd round-trip");
            }
        }
    }

    // ndx / pfx: 32-byte keys (16+16). Include keys whose halves are identical,
    // which makes the two key schedules equal and triggers the `d == 0`
    // fallback branch.
    let mut k32: Vec<Vec<u8>> = Vec::new();
    k32.push(vec![0u8; 32]); // halves equal -> d == 0
    k32.push(vec![0xffu8; 32]); // halves equal -> d == 0
    {
        let h = vec![0x5au8; 16];
        let mut k = h.clone();
        k.extend_from_slice(&h);
        k32.push(k); // halves equal (and k[i]^0x5a == 0)
    }
    {
        let h: Vec<u8> = (0..16u8).collect();
        let mut k = h.clone();
        k.extend_from_slice(&h);
        k32.push(k); // halves equal
    }
    for _ in 0..16 {
        k32.push(rng.bytes(32));
    }
    let mut tweaks16: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16]];
    for _ in 0..6 {
        tweaks16.push(rng.bytes(16));
    }

    for k in k32.iter() {
        for t in tweaks16.iter() {
            for inp in inputs.iter().take(16) {
                let mut cb = vec![CANARY; 32 + CANARY_LEN];
                let mut rb = cb.clone();
                unsafe {
                    cnxe(cb.as_mut_ptr(), inp.as_ptr(), t.as_ptr(), k.as_ptr());
                    rnxe(rb.as_mut_ptr(), inp.as_ptr(), t.as_ptr(), k.as_ptr());
                }
                let ctx = format!("ndx_encrypt k={} t={}", common::hex(k), common::hex(t));
                common::eqb(&ctx, &cb, &rb);
                assert!(cb[32..].iter().all(|&x| x == CANARY), "{ctx}: canary");
                assert_eq!(&cb[..16], &t[..], "{ctx}: tweak must be prefixed");

                let mut cd = vec![CANARY; 16 + CANARY_LEN];
                let mut rd = cd.clone();
                unsafe {
                    cnxd(cd.as_mut_ptr(), cb.as_ptr(), k.as_ptr());
                    rnxd(rd.as_mut_ptr(), rb.as_ptr(), k.as_ptr());
                }
                common::eqb(&format!("ndx_decrypt {ctx}"), &cd, &rd);
                assert_eq!(&cd[..16], &inp[..], "{ctx}: ndx round-trip");
            }
        }
        // pfx
        for inp in inputs.iter() {
            let mut cb = vec![CANARY; 16 + CANARY_LEN];
            let mut rb = cb.clone();
            unsafe {
                cpfe(cb.as_mut_ptr(), inp.as_ptr(), k.as_ptr());
                rpfe(rb.as_mut_ptr(), inp.as_ptr(), k.as_ptr());
            }
            let ctx = format!("pfx_encrypt k={} in={}", common::hex(k), common::hex(inp));
            common::eqb(&ctx, &cb, &rb);
            assert!(cb[16..].iter().all(|&x| x == CANARY), "{ctx}: canary");
            // ipv4-mapped inputs keep the mapped prefix
            let mapped = inp[..10].iter().all(|&x| x == 0) && inp[10] == 0xff && inp[11] == 0xff;
            if mapped {
                assert!(
                    cb[..10].iter().all(|&x| x == 0) && cb[10] == 0xff && cb[11] == 0xff,
                    "{ctx}: mapped prefix must be preserved"
                );
            }

            let mut cd = vec![CANARY; 16 + CANARY_LEN];
            let mut rd = cd.clone();
            unsafe {
                cpfd(cd.as_mut_ptr(), cb.as_ptr(), k.as_ptr());
                rpfd(rd.as_mut_ptr(), rb.as_ptr(), k.as_ptr());
            }
            common::eqb(&format!("pfx_decrypt {ctx}"), &cd, &rd);
            assert_eq!(&cd[..16], &inp[..], "{ctx}: pfx round-trip");
        }
    }
}

#[test]
fn ipcrypt_soft_implementation_object() {
    let (cimpl, rimpl) = both_data!("ipcrypt_soft_implementation", IpcryptImpl);
    let mut rng = common::Rng::new(0x5EED_0052);
    let inputs = ipcrypt_inputs(&mut rng);

    unsafe {
        let ci = &*cimpl;
        let ri = &*rimpl;
        for _ in 0..8 {
            let k16 = rng.bytes(16);
            let k32 = rng.bytes(32);
            let t8 = rng.bytes(8);
            let t16 = rng.bytes(16);
            for inp in inputs.iter().take(10) {
                // encrypt / decrypt
                let mut a = vec![CANARY; 16 + CANARY_LEN];
                let mut b = a.clone();
                (ci.encrypt)(a.as_mut_ptr(), inp.as_ptr(), k16.as_ptr());
                (ri.encrypt)(b.as_mut_ptr(), inp.as_ptr(), k16.as_ptr());
                common::eqb("soft.encrypt", &a, &b);
                let mut a2 = vec![CANARY; 16 + CANARY_LEN];
                let mut b2 = a2.clone();
                (ci.decrypt)(a2.as_mut_ptr(), a.as_ptr(), k16.as_ptr());
                (ri.decrypt)(b2.as_mut_ptr(), b.as_ptr(), k16.as_ptr());
                common::eqb("soft.decrypt", &a2, &b2);
                assert_eq!(&a2[..16], &inp[..]);

                // nd
                let mut a = vec![CANARY; 24 + CANARY_LEN];
                let mut b = a.clone();
                (ci.nd_encrypt)(a.as_mut_ptr(), inp.as_ptr(), t8.as_ptr(), k16.as_ptr());
                (ri.nd_encrypt)(b.as_mut_ptr(), inp.as_ptr(), t8.as_ptr(), k16.as_ptr());
                common::eqb("soft.nd_encrypt", &a, &b);
                let mut a2 = vec![CANARY; 16 + CANARY_LEN];
                let mut b2 = a2.clone();
                (ci.nd_decrypt)(a2.as_mut_ptr(), a.as_ptr(), k16.as_ptr());
                (ri.nd_decrypt)(b2.as_mut_ptr(), b.as_ptr(), k16.as_ptr());
                common::eqb("soft.nd_decrypt", &a2, &b2);
                assert_eq!(&a2[..16], &inp[..]);

                // ndx
                let mut a = vec![CANARY; 32 + CANARY_LEN];
                let mut b = a.clone();
                (ci.ndx_encrypt)(a.as_mut_ptr(), inp.as_ptr(), t16.as_ptr(), k32.as_ptr());
                (ri.ndx_encrypt)(b.as_mut_ptr(), inp.as_ptr(), t16.as_ptr(), k32.as_ptr());
                common::eqb("soft.ndx_encrypt", &a, &b);
                let mut a2 = vec![CANARY; 16 + CANARY_LEN];
                let mut b2 = a2.clone();
                (ci.ndx_decrypt)(a2.as_mut_ptr(), a.as_ptr(), k32.as_ptr());
                (ri.ndx_decrypt)(b2.as_mut_ptr(), b.as_ptr(), k32.as_ptr());
                common::eqb("soft.ndx_decrypt", &a2, &b2);
                assert_eq!(&a2[..16], &inp[..]);

                // pfx
                let mut a = vec![CANARY; 16 + CANARY_LEN];
                let mut b = a.clone();
                (ci.pfx_encrypt)(a.as_mut_ptr(), inp.as_ptr(), k32.as_ptr());
                (ri.pfx_encrypt)(b.as_mut_ptr(), inp.as_ptr(), k32.as_ptr());
                common::eqb("soft.pfx_encrypt", &a, &b);
                let mut a2 = vec![CANARY; 16 + CANARY_LEN];
                let mut b2 = a2.clone();
                (ci.pfx_decrypt)(a2.as_mut_ptr(), a.as_ptr(), k32.as_ptr());
                (ri.pfx_decrypt)(b2.as_mut_ptr(), b.as_ptr(), k32.as_ptr());
                common::eqb("soft.pfx_decrypt", &a2, &b2);
                assert_eq!(&a2[..16], &inp[..]);
            }
        }
    }
}

#[test]
fn ipcrypt_pick_best_implementation() {
    let (c, r) = both!(
        "_crypto_ipcrypt_pick_best_implementation",
        unsafe extern "C" fn() -> c_int
    );
    for _ in 0..3 {
        let (a, b) = unsafe { (c(), r()) };
        common::eqi("_crypto_ipcrypt_pick_best_implementation", a, b);
        assert_eq!(a, 0);
    }
    // ... and the selected implementation still produces identical output
    let l = common::libs();
    let ce = getsym!(l.c, "crypto_ipcrypt_encrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));
    let re = getsym!(l.r, "crypto_ipcrypt_encrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));
    let mut rng = common::Rng::new(0x5EED_0053);
    for _ in 0..20 {
        let k = rng.bytes(16);
        let inp = rng.bytes(16);
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        unsafe {
            ce(a.as_mut_ptr(), inp.as_ptr(), k.as_ptr());
            re(b.as_mut_ptr(), inp.as_ptr(), k.as_ptr());
        }
        common::eqb("encrypt after pick_best", &a, &b);
    }
}

#[test]
fn ipcrypt_keygen() {
    // keygen wraps randombytes_buf; with the deterministic test RNG installed
    // its output is byte-comparable.
    let _g = STATE_LOCK.lock().unwrap();
    let l = common::libs();
    let capi = rb_api(&l.c);
    let rapi = rb_api(&l.r);

    let gens: [(&str, usize); 4] = [
        ("crypto_ipcrypt_keygen", 16),
        ("crypto_ipcrypt_nd_keygen", 16),
        ("crypto_ipcrypt_ndx_keygen", 32),
        ("crypto_ipcrypt_pfx_keygen", 32),
    ];

    unsafe {
        let cim = impl_no_uniform(0);
        let rim = impl_no_uniform(1);
        (capi.set_impl)(&cim);
        (rapi.set_impl)(&rim);
        for (name, n) in gens {
            TEST_CTR[0] = 0x1111_2222;
            TEST_CTR[1] = 0x1111_2222;
            let c: unsafe extern "C" fn(*mut u8) = sym(&l.c, name);
            let r: unsafe extern "C" fn(*mut u8) = sym(&l.r, name);
            let mut cb = vec![CANARY; n + CANARY_LEN];
            let mut rb = cb.clone();
            c(cb.as_mut_ptr());
            r(rb.as_mut_ptr());
            common::eqb(name, &cb, &rb);
            assert!(cb[n..].iter().all(|&x| x == CANARY), "{name}: canary");
            assert!(cb[..n].iter().any(|&x| x != CANARY), "{name}: nothing written");
        }
        // restore
        let (cimpl, rimpl) = both_data!("randombytes_sysrandom_implementation", RbImpl);
        (capi.set_impl)(cimpl);
        (rapi.set_impl)(rimpl);
    }

    // and with the real RNG they must at least write the right number of bytes
    for (name, n) in gens {
        let c: unsafe extern "C" fn(*mut u8) = sym(&l.c, name);
        let r: unsafe extern "C" fn(*mut u8) = sym(&l.r, name);
        let mut cb = vec![CANARY; n + CANARY_LEN];
        let mut rb = cb.clone();
        unsafe {
            c(cb.as_mut_ptr());
            r(rb.as_mut_ptr());
        }
        assert!(cb[n..].iter().all(|&x| x == CANARY), "{name}: canary (real RNG)");
        assert!(rb[n..].iter().all(|&x| x == CANARY), "{name}: canary (real RNG)");
    }
}

/// An end-to-end use of the area: parse an IP string, encrypt it with
/// ipcrypt-pfx, and format the result back to a string.
#[test]
fn ipcrypt_string_end_to_end() {
    let l = common::libs();
    let cip2bin = getsym!(l.c, "sodium_ip2bin", FnIp2Bin);
    let rip2bin = getsym!(l.r, "sodium_ip2bin", FnIp2Bin);
    let cbin2ip = getsym!(l.c, "sodium_bin2ip", FnBin2Ip);
    let rbin2ip = getsym!(l.r, "sodium_bin2ip", FnBin2Ip);
    let cpfe = getsym!(l.c, "crypto_ipcrypt_pfx_encrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));
    let rpfe = getsym!(l.r, "crypto_ipcrypt_pfx_encrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));
    let cpfd = getsym!(l.c, "crypto_ipcrypt_pfx_decrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));
    let rpfd = getsym!(l.r, "crypto_ipcrypt_pfx_decrypt", unsafe extern "C" fn(*mut u8, *const u8, *const u8));

    let ips: [&[u8]; 10] = [
        b"0.0.0.0",
        b"1.2.3.4",
        b"192.168.0.1",
        b"255.255.255.255",
        b"::",
        b"::1",
        b"2001:db8::1",
        b"fe80::1%eth0",
        b"::ffff:1.2.3.4",
        b"1:2:3:4:5:6:7:8",
    ];
    let mut rng = common::Rng::new(0x5EED_0054);
    for _ in 0..8 {
        let k = rng.bytes(32);
        for ip in ips {
            let mut cbin = [0u8; 16];
            let mut rbin = [0u8; 16];
            let (a, b) = unsafe {
                (
                    cip2bin(cbin.as_mut_ptr(), ip.as_ptr() as *const c_char, ip.len()),
                    rip2bin(rbin.as_mut_ptr(), ip.as_ptr() as *const c_char, ip.len()),
                )
            };
            common::eqi("e2e ip2bin", a, b);
            assert_eq!(a, 0);
            common::eqb("e2e ip2bin bytes", &cbin, &rbin);

            let mut cenc = [0u8; 16];
            let mut renc = [0u8; 16];
            unsafe {
                cpfe(cenc.as_mut_ptr(), cbin.as_ptr(), k.as_ptr());
                rpfe(renc.as_mut_ptr(), rbin.as_ptr(), k.as_ptr());
            }
            common::eqb("e2e pfx_encrypt", &cenc, &renc);

            let mut cs = vec![CANARY; 48 + CANARY_LEN];
            let mut rs = cs.clone();
            let (cp, rp) = unsafe {
                (
                    cbin2ip(cs.as_mut_ptr() as *mut c_char, 48, cenc.as_ptr()),
                    rbin2ip(rs.as_mut_ptr() as *mut c_char, 48, renc.as_ptr()),
                )
            };
            assert!(!cp.is_null() && !rp.is_null());
            common::eqb("e2e bin2ip", &cs, &rs);

            let mut cdec = [0u8; 16];
            let mut rdec = [0u8; 16];
            unsafe {
                cpfd(cdec.as_mut_ptr(), cenc.as_ptr(), k.as_ptr());
                rpfd(rdec.as_mut_ptr(), renc.as_ptr(), k.as_ptr());
            }
            common::eqb("e2e pfx_decrypt", &cdec, &rdec);
            assert_eq!(cdec, cbin, "e2e round-trip");
        }
    }
}

// ===========================================================================
// sodium_misuse() / abort paths (each verified in a forked child)
// ===========================================================================

#[test]
fn misuse_abort_paths() {
    let _g = STATE_LOCK.lock().unwrap();
    let l = common::libs();

    // make sure no misuse handler is installed in either library
    unsafe {
        let cs = getsym!(l.c, "sodium_set_misuse_handler", unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int);
        let rs = getsym!(l.r, "sodium_set_misuse_handler", unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int);
        cs(None);
        rs(None);
    }

    for (lname, lib) in [("C", &l.c), ("Rust", &l.r)] {
        // sodium_pad: SIZE_MAX - unpadded_buflen <= xpadlen
        let pad = getsym!(lib, "sodium_pad", FnPad);
        for (n, bs) in [(usize::MAX, 16usize), (usize::MAX, 1), (usize::MAX - 5, 8)] {
            assert_aborts(&format!("{lname}: sodium_pad(n={n},bs={bs})"), move || unsafe {
                let mut out = 0usize;
                let mut b = [0u8; 1];
                pad(&mut out, b.as_mut_ptr(), n, bs, usize::MAX);
            });
        }

        // sodium_bin2hex: hex_maxlen <= bin_len * 2
        let b2h = getsym!(lib, "sodium_bin2hex", FnBin2Hex);
        for (bin_len, hex_maxlen) in [(0usize, 0usize), (1, 2), (4, 8), (16, 5)] {
            assert_aborts(
                &format!("{lname}: sodium_bin2hex(bin_len={bin_len},hex_maxlen={hex_maxlen})"),
                move || unsafe {
                    let bin = [0u8; 32];
                    let mut hex = [0u8; 128];
                    b2h(hex.as_mut_ptr() as *mut c_char, hex_maxlen, bin.as_ptr(), bin_len);
                },
            );
        }
        // sodium_bin2hex: bin_len >= SIZE_MAX / 2
        assert_aborts(&format!("{lname}: sodium_bin2hex(bin_len=SIZE_MAX/2)"), move || unsafe {
            let bin = [0u8; 1];
            let mut hex = [0u8; 8];
            b2h(hex.as_mut_ptr() as *mut c_char, usize::MAX, bin.as_ptr(), usize::MAX / 2);
        });

        // sodium_base64_check_variant: out-of-range variant
        let enclen = getsym!(lib, "sodium_base64_encoded_len", FnEncLen);
        let b2b = getsym!(lib, "sodium_bin2base64", FnBin2B64);
        let b642 = getsym!(lib, "sodium_base642bin", FnB642Bin);
        for v in [-1i32, 0, 2, 4, 6, 8, 9, 99, 0x7fff_ffff] {
            assert_aborts(
                &format!("{lname}: sodium_base64_encoded_len(variant={v})"),
                move || unsafe {
                    enclen(10, v);
                },
            );
            assert_aborts(
                &format!("{lname}: sodium_bin2base64(variant={v})"),
                move || unsafe {
                    let bin = [0u8; 8];
                    let mut b64 = [0u8; 64];
                    b2b(b64.as_mut_ptr() as *mut c_char, 64, bin.as_ptr(), 8, v);
                },
            );
            assert_aborts(
                &format!("{lname}: sodium_base642bin(variant={v})"),
                move || unsafe {
                    let mut bin = [0u8; 8];
                    let b64 = b"AAAA";
                    b642(
                        bin.as_mut_ptr(),
                        8,
                        b64.as_ptr() as *const c_char,
                        4,
                        core::ptr::null(),
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        v,
                    );
                },
            );
        }

        // sodium_base64_encoded_len: bin_len / 3 > (SIZE_MAX - 5) / 4
        assert_aborts(&format!("{lname}: sodium_base64_encoded_len(SIZE_MAX)"), move || unsafe {
            enclen(usize::MAX, 1);
        });
        // sodium_bin2base64: nibbles > (SIZE_MAX - 5) / 4
        assert_aborts(&format!("{lname}: sodium_bin2base64(bin_len=SIZE_MAX)"), move || unsafe {
            let bin = [0u8; 1];
            let mut b64 = [0u8; 8];
            b2b(b64.as_mut_ptr() as *mut c_char, usize::MAX, bin.as_ptr(), usize::MAX, 1);
        });
        // sodium_bin2base64: b64_maxlen <= b64_len
        for (bin_len, b64_maxlen) in [(0usize, 0usize), (1, 2), (3, 4), (6, 3)] {
            assert_aborts(
                &format!("{lname}: sodium_bin2base64(bin_len={bin_len},b64_maxlen={b64_maxlen})"),
                move || unsafe {
                    let bin = [0u8; 8];
                    let mut b64 = [0u8; 64];
                    b2b(b64.as_mut_ptr() as *mut c_char, b64_maxlen, bin.as_ptr(), bin_len, 1);
                },
            );
        }

        // randombytes_buf_deterministic: size > 0x4000000000
        let bufdet = getsym!(
            lib,
            "randombytes_buf_deterministic",
            unsafe extern "C" fn(*mut c_void, usize, *const u8)
        );
        assert_aborts(
            &format!("{lname}: randombytes_buf_deterministic(size too large)"),
            move || unsafe {
                let seed = [0u8; 32];
                let mut b = [0u8; 8];
                bufdet(b.as_mut_ptr() as *mut c_void, 0x4000000000usize + 1, seed.as_ptr());
            },
        );

        // sodium_misuse() itself
        let misuse = getsym!(lib, "sodium_misuse", unsafe extern "C" fn());
        assert_aborts(&format!("{lname}: sodium_misuse()"), move || unsafe {
            misuse();
        });
    }
}
