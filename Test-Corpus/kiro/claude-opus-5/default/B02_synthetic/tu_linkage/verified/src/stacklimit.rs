// Support for reproducing the stack-overflow behavior of C's `int tmp[m];` VLA.
//
// `run_engine` case 9 declares a variable-length array sized by a bytecode
// operand. When `4 * m` exceeds the process stack limit the C program dies with
// SIGSEGV and an empty stderr (measured on this host with an 8 MiB limit:
// `m = 2093750` succeeds, `m = 2094726` dies with signal 11).
//
// Rust has no VLA, and merely overflowing the Rust stack does *not* reproduce
// that: std installs a guard-page handler that prints "has overflowed its stack"
// and aborts (SIGABRT, exit 134) instead of faulting (SIGSEGV, exit 139). So the
// budget is computed explicitly here and a genuine fault is raised instead.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Address of a local near the top of `main`'s frame, standing in for the base
/// of the process stack.
static STACK_BASE: AtomicUsize = AtomicUsize::new(0);
/// RLIMIT_STACK in bytes, or `usize::MAX` when unlimited/unknown.
static STACK_LIMIT: AtomicUsize = AtomicUsize::new(0);

/// Reads RLIMIT_STACK from procfs, avoiding a libc dependency for `getrlimit`.
fn read_stack_limit() -> usize {
    // Line format: "Max stack size            8388608              unlimited            bytes"
    if let Ok(text) = std::fs::read_to_string("/proc/self/limits") {
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("Max stack size") {
                match rest.split_whitespace().next() {
                    Some("unlimited") => return usize::MAX,
                    Some(tok) => {
                        if let Ok(v) = tok.parse::<usize>() {
                            return v;
                        }
                    }
                    None => {}
                }
            }
        }
    }
    // Fall back to the common default rather than disabling the check.
    8 * 1024 * 1024
}

/// Records the stack base. Call once, from `main`, passing the address of a
/// local variable.
pub fn init(base: usize) {
    STACK_BASE.store(base, Ordering::Relaxed);
    STACK_LIMIT.store(read_stack_limit(), Ordering::Relaxed);
}

/// Stack the C program needs beyond the VLA itself: `main`'s and `run_engine`'s
/// frames, the `alloca` alignment, the `process_stream` call chain, and the
/// argv/env block the kernel places at the stack base.
///
/// Calibrated on this host: C's largest surviving `m` is 2093750 (8375000 VLA
/// bytes against an 8388608-byte limit), leaving 13608 bytes of headroom; the
/// address-based `remaining` below already accounts for a few hundred of those.
const C_FRAME_OVERHEAD: usize = 13_180;

/// Bytes of stack still available to the caller, given the address `sp` of one
/// of its locals. Returns `usize::MAX` when the limit is unlimited or unknown.
pub fn remaining(sp: usize) -> usize {
    let base = STACK_BASE.load(Ordering::Relaxed);
    let limit = STACK_LIMIT.load(Ordering::Relaxed);
    if base == 0 || limit == usize::MAX {
        return usize::MAX;
    }
    limit
        .saturating_sub(base.saturating_sub(sp))
        .saturating_sub(C_FRAME_OVERHEAD)
}

/// Dies the way C does when the VLA overshoots the stack: SIGSEGV, no output.
///
/// std's SIGSEGV handler only claims faults near a guard page; for any other
/// address it restores the default disposition and lets the fault re-raise, so
/// this terminates with signal 11 and an empty stderr.
pub fn raise_segv() -> ! {
    unsafe {
        std::ptr::null_mut::<u8>().write_volatile(1u8);
    }
    // Not reached; present only so the function can be `-> !`.
    std::process::abort()
}
