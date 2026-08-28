//! Child-process differential harness (moved out of `tests/aborts.rs` so
//! several test binaries can share it).

#![allow(dead_code)]

use super::*;

// ---------------------------------------------------------------------------
// SIGABRT is expected thousands of times here; without this every one of them
// would be handed to systemd-coredump, costing ~200 ms each.
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct RLimit {
    pub cur: u64,
    pub max: u64,
}

#[repr(C)]
pub struct TimeVal {
    pub sec: i64,
    pub usec: i64,
}

#[repr(C)]
pub struct ITimerVal {
    pub interval: TimeVal,
    pub value: TimeVal,
}

unsafe extern "C" {
    fn setrlimit(resource: i32, rlim: *const RLimit) -> i32;
    fn setitimer(which: i32, new: *const ITimerVal, old: *mut ITimerVal) -> i32;
}

const RLIMIT_CORE: i32 = 4;
const ITIMER_REAL: i32 = 0;

/// Wall-clock budget for one child, in microseconds.  `cp_dynamic`'s run-length
/// loop can be made to spin forever (a corrupted run counter goes negative while
/// the loop variable is repeatedly reset), in **both** implementations — so the
/// only way to compare that outcome is to let the same alarm fire in both.
pub fn child_timeout_us() -> i64 {
    std::env::var("CP_CHILD_TIMEOUT_US")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(400_000)
}

/// Arm `SIGALRM` (default disposition: terminate with signal 14) in the child.
pub fn arm_child_timeout() {
    let us = child_timeout_us();
    let it = ITimerVal {
        interval: TimeVal { sec: 0, usec: 0 },
        value: TimeVal { sec: us / 1_000_000, usec: us % 1_000_000 },
    };
    unsafe { setitimer(ITIMER_REAL, &it, std::ptr::null_mut()) };
}

pub fn no_core_dumps() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let r = RLimit { cur: 0, max: 0 };
        unsafe { setrlimit(RLIMIT_CORE, &r) };
    });
}

// ---------------------------------------------------------------------------
// fork-based sweep — ~200x cheaper per case than re-exec, so it can cover
// thousands of inputs, including *structured* ones that reach deep code paths.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn _exit(code: i32) -> !;
}

/// Fixed-capacity, allocation-free formatter (the forked child must not touch
/// the allocator: another thread could have been holding its lock at `fork`).
pub struct Fmt {
    pub buf: [u8; 192],
    pub len: usize,
}

impl Fmt {
    pub fn new() -> Fmt {
        Fmt { buf: [0u8; 192], len: 0 }
    }
    pub fn put(&mut self, b: &[u8]) {
        for &c in b {
            if self.len < self.buf.len() {
                self.buf[self.len] = c;
                self.len += 1;
            }
        }
    }
    pub fn hex64(&mut self, mut v: u64) {
        let mut tmp = [0u8; 16];
        for i in (0..16).rev() {
            tmp[i] = b"0123456789abcdef"[(v & 0xF) as usize];
            v >>= 4;
        }
        self.put(&tmp);
    }
}

pub fn fnv_ptr(p: *const u8, n: usize) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for i in 0..n {
        h ^= unsafe { *p.add(i) } as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ForkOutcome {
    pub signal: Option<i32>,
    pub code: Option<i32>,
    pub summary: Option<String>,
    pub assertion: Option<String>,
}

/// Call `cp_inflate` in a forked child and report what happened.
///
/// Everything that allocates happens *before* `fork`; the child only performs
/// the FFI call, hashes the buffers and `write(2)`s a fixed-size line.
pub fn fork_inflate(
    l: &Lib,
    stream: &[u8],
    in_off: usize,
    in_bytes: i32,
    out_cap: usize,
    out_bytes: i32,
) -> ForkOutcome {
    no_core_dumps();
    let f = l.cp_inflate();
    let reason_slot: *mut *const std::ffi::c_char = l.data(b"cp_error_reason\0");
    let mut ibuf = AlignedBuf::new(stream, in_off);
    let mut obuf = AlignedBuf::zeroed(out_cap, 0);
    let iptr = ibuf.ptr();
    let optr = obuf.ptr();
    let i_all = ibuf.all_bytes().len();
    let o_all = obuf.all_bytes().len();
    let i_base = ibuf.all_bytes().as_ptr();
    let o_base = obuf.all_bytes().as_ptr();

    let mut fds = [0i32; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe");
    // pre-allocate the reader buffer as well
    let mut sink: Vec<u8> = Vec::with_capacity(64 * 1024);
    let mut chunk = [0u8; 8192];

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            close(fds[0]);
            dup2(fds[1], 2); // assertion diagnostics land in the pipe
            arm_child_timeout();
            *reason_slot = std::ptr::null();
            let rc = f(
                iptr as *mut std::ffi::c_void,
                in_bytes,
                optr as *mut std::ffi::c_void,
                out_bytes,
            );
            let mut m = Fmt::new();
            m.put(b"\nSUMMARY rc=");
            m.hex64(rc as u32 as u64);
            m.put(b" out=");
            m.hex64(fnv_ptr(o_base, o_all));
            m.put(b" in=");
            m.hex64(fnv_ptr(i_base, i_all));
            m.put(b" reason=");
            let rp = *reason_slot as *const u8;
            if rp.is_null() {
                m.put(b"null------------");
            } else {
                let mut n = 0usize;
                while *rp.add(n) != 0 {
                    n += 1;
                }
                m.hex64(fnv_ptr(rp, n));
            }
            m.put(b"\n");
            write(fds[1], m.buf.as_ptr(), m.len);
            close(fds[1]);
            _exit(0);
        }
    }
    unsafe { close(fds[1]) };
    loop {
        let n = unsafe { read(fds[0], chunk.as_mut_ptr(), chunk.len()) };
        if n <= 0 {
            break;
        }
        sink.extend_from_slice(&chunk[..n as usize]);
    }
    unsafe { close(fds[0]) };
    let mut status = 0i32;
    unsafe { waitpid(pid, &mut status, 0) };

    let text = String::from_utf8_lossy(&sink).into_owned();
    let summary = text
        .lines()
        .find(|l| l.starts_with("SUMMARY "))
        .map(|l| l["SUMMARY ".len()..].to_string());
    let assertion = text.lines().find(|l| l.contains("Assertion `")).map(|l| {
        match l.find(": /") {
            Some(i) => l[i + 2..].to_string(),
            None => l.to_string(),
        }
    });
    let signaled = (status & 0x7f) != 0;
    ForkOutcome {
        signal: if signaled { Some(status & 0x7f) } else { None },
        code: if signaled { None } else { Some((status >> 8) & 0xff) },
        summary,
        assertion,
    }
}

pub fn diff_fork_full(
    p: &Pair,
    stream: &[u8],
    in_off: usize,
    in_bytes: i32,
    out_cap: usize,
    out_bytes: i32,
    label: &str,
) -> ForkOutcome {
    let c = fork_inflate(&p.c, stream, in_off, in_bytes, out_cap, out_bytes);
    let r = fork_inflate(&p.rs, stream, in_off, in_bytes, out_cap, out_bytes);
    assert_eq!(
        c, r,
        "[{label}] divergence\n  stream={stream:02x?} in_off={in_off} in_bytes={in_bytes} out_bytes={out_bytes}"
    );
    c
}

pub fn diff_fork(
    p: &Pair,
    stream: &[u8],
    in_off: usize,
    in_bytes: i32,
    out_cap: usize,
    out_bytes: i32,
    label: &str,
) -> Option<String> {
    diff_fork_full(p, stream, in_off, in_bytes, out_cap, out_bytes, label)
        .assertion
        .map(|a| assertion_expr(&a))
}

pub fn assertion_expr(a: &str) -> String {
    a.split("Assertion `")
        .nth(1)
        .map(|s| s.trim_end_matches(" failed.").trim_end_matches('\'').to_string())
        .unwrap_or_else(|| a.to_string())
}
