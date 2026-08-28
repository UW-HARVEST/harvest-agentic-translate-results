//! Phase C — ERRORS.md rows 22, 24 and 25: `ensure()`'s size-limit rejections.
//!
//! These are the only rejections in `cJSON.c` that need an output larger than
//! `INT_MAX / 2`, so they need a genuinely large input.  `cJSON_CreateStringReference`
//! makes that cheap: the payload is borrowed from the caller, and
//! `print_string_ptr` multiplies its length by 6 when every byte is a control
//! character (`\u00xx` costs 5 extra characters each), so a ~358 MiB buffer
//! produces an `output_length` above `INT_MAX`.
//!
//! * row 22 — `needed > INT_MAX` (cJSON.c:468), rejected before any allocation.
//! * row 24 — `needed > INT_MAX` after `needed += p->offset + 1` (cJSON.c:494).
//! * row 25 — `newsize = INT_MAX` and `hooks.reallocate` (libc `realloc`)
//!   returns NULL (cJSON.c:512).  `cJSON_InitHooks` can only ever set
//!   `reallocate` to libc `realloc` or to NULL, so the only way to observe this
//!   branch is to make a real ~2 GiB `realloc` fail — done here by temporarily
//!   lowering `RLIMIT_AS`.
//!
//! The file contains exactly ONE `#[test]` so that no other thread allocates
//! while `RLIMIT_AS` is lowered.
#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

const RLIMIT_AS: c_int = 9; // Linux
const RLIM_INFINITY: u64 = u64::MAX;

extern "C" {
    fn getrlimit(resource: c_int, rlim: *mut RLimit) -> c_int;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn sysconf(name: c_int) -> i64;
}

const SC_PAGESIZE: c_int = 30; // _SC_PAGESIZE on Linux

fn vm_size_bytes() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = statm
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let page = unsafe { sysconf(SC_PAGESIZE) }.max(4096) as u64;
    pages * page
}

/// One probe: run all four print entry points on both libraries and require the
/// results to match.  Returns whether the C side rejected the print.
unsafe fn probe_all(c: &Api, r: &Api, ic: *mut CJson, ir: *mut CJson, label: &str) -> bool {
    let a = print_and_take(c, ic);
    let b = print_and_take(r, ir);
    assert_eq!(a.is_none(), b.is_none(), "{label}: cJSON_Print nullness");
    assert_eq!(a, b, "{label}: cJSON_Print bytes");
    let rejected = a.is_none();

    let a = print_unformatted_and_take(c, ic);
    let b = print_unformatted_and_take(r, ir);
    assert_eq!(a, b, "{label}: cJSON_PrintUnformatted");

    for pb in [256i32] {
        for fmt in [1i32] {
            let a = print_buffered_and_take(c, ic, pb, fmt);
            let b = print_buffered_and_take(r, ir, pb, fmt);
            assert_eq!(
                a.is_none(),
                b.is_none(),
                "{label}: cJSON_PrintBuffered({pb}, {fmt}) nullness"
            );
            assert_eq!(a, b, "{label}: cJSON_PrintBuffered({pb}, {fmt}) bytes");
        }
    }

    let mut buf_c = vec![0x55u8; 4096];
    let mut buf_r = vec![0x55u8; 4096];
    for fmt in [1i32] {
        let x = (c.cJSON_PrintPreallocated)(ic, buf_c.as_mut_ptr() as *mut c_char, 4096, fmt);
        let y = (r.cJSON_PrintPreallocated)(ir, buf_r.as_mut_ptr() as *mut c_char, 4096, fmt);
        assert_eq!(x, y, "{label}: cJSON_PrintPreallocated({fmt}) rc");
        assert_eq!(buf_c, buf_r, "{label}: cJSON_PrintPreallocated({fmt}) buffer");
    }

    rejected
}

#[test]
fn err_ensure_size_limits() {
    let (c, r) = both();
    let _guard = lock_global_state();

    // `output_length` for a payload of `n5` control bytes costing 5 extra
    // characters plus `n1` bytes costing 1 extra character is `6*n5 + 2*n1`.
    //
    //   row 22: 6*n5 + 3 > INT_MAX          → n5 = 357_913_941  (needed = 2147483649)
    //   row 24: 6*n5 + 2*n1 + 3 == INT_MAX  → n5 = 357_913_940, n1 = 2
    //   row 25: INT_MAX/2 < needed <= INT_MAX → n5 = 200_000_000 (needed = 1200000004)
    const N_ROW22: usize = 357_913_941;
    const N_ROW24_FIVES: usize = 357_913_940;
    const N_ROW25: usize = 200_000_000;
    const CAP: usize = N_ROW24_FIVES + 2 + 1 + 8;

    // 0x01 costs 5 extra characters (``), '\n' costs 1 (`\n`).
    let mut payload: Vec<u8> = match std::panic::catch_unwind(|| vec![0x01u8; CAP]) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("SKIP err_ensure_size_limits: cannot allocate {CAP} bytes");
            return;
        }
    };

    unsafe {
        // ---------------- row 22: needed > INT_MAX (checked at line 468) -----
        payload[N_ROW22] = 0;
        let ic = (c.cJSON_CreateStringReference)(payload.as_ptr() as *const c_char);
        let ir = (r.cJSON_CreateStringReference)(payload.as_ptr() as *const c_char);
        let rejected = probe_all(&c, &r, ic, ir, "row 22 (needed > INT_MAX)");
        assert!(
            rejected,
            "row 22: the C library must reject an output length above INT_MAX"
        );
        (c.cJSON_Delete)(ic);
        (r.cJSON_Delete)(ir);
        payload[N_ROW22] = 0x01;

        // ---- row 24: needed == INT_MAX, then += offset + 1 (line 494) -------
        payload[N_ROW24_FIVES] = b'\n';
        payload[N_ROW24_FIVES + 1] = b'\n';
        payload[N_ROW24_FIVES + 2] = 0;
        let ic = (c.cJSON_CreateStringReference)(payload.as_ptr() as *const c_char);
        let ir = (r.cJSON_CreateStringReference)(payload.as_ptr() as *const c_char);
        let rejected = probe_all(&c, &r, ic, ir, "row 24 (needed + offset + 1 > INT_MAX)");
        assert!(
            rejected,
            "row 24: the C library must reject needed + offset + 1 above INT_MAX"
        );
        (c.cJSON_Delete)(ic);
        (r.cJSON_Delete)(ir);
        payload[N_ROW24_FIVES] = 0x01;
        payload[N_ROW24_FIVES + 1] = 0x01;
        payload[N_ROW24_FIVES + 2] = 0x01;

        // ---- row 25: newsize = INT_MAX and realloc fails --------------------
        payload[N_ROW25] = 0;
        let ic = (c.cJSON_CreateStringReference)(payload.as_ptr() as *const c_char);
        let ir = (r.cJSON_CreateStringReference)(payload.as_ptr() as *const c_char);

        let mut saved = RLimit::default();
        let have_rlimit = getrlimit(RLIMIT_AS, &mut saved) == 0;
        let mut lowered = false;
        if have_rlimit {
            let target = vm_size_bytes() + (128 << 20);
            let want = RLimit {
                rlim_cur: if saved.rlim_max == RLIM_INFINITY {
                    target
                } else {
                    target.min(saved.rlim_max)
                },
                rlim_max: saved.rlim_max,
            };
            lowered = setrlimit(RLIMIT_AS, &want) == 0;
        }

        // Only the two library calls happen inside the window; both results are
        // plain pointers, so nothing on the Rust side allocates here.
        let pc = (c.cJSON_Print)(ic);
        let pr = (r.cJSON_Print)(ir);
        let pc_null = pc.is_null();
        let pr_null = pr.is_null();

        if lowered {
            setrlimit(RLIMIT_AS, &saved);
        }

        assert_eq!(
            pc_null, pr_null,
            "row 25: cJSON_Print nullness differs when realloc(INT_MAX) fails"
        );
        if !pc_null {
            // The platform satisfied the ~2 GiB request after all; compare the
            // full output and report that the row could not be provoked.
            let a = cstr(pc);
            let b = cstr(pr);
            assert_eq!(a, b, "row 25: output bytes differ");
            (c.cJSON_free)(pc as *mut c_void);
            (r.cJSON_free)(pr as *mut c_void);
            eprintln!(
                "NOTE err_ensure_size_limits: realloc(INT_MAX) succeeded despite \
                 RLIMIT_AS (lowered={lowered}); row 25 could not be provoked on this \
                 platform, but C and Rust agreed on the full {} byte output.",
                a.map(|v| v.len()).unwrap_or(0)
            );
        } else {
            eprintln!("row 25 covered: realloc(newsize = INT_MAX) returned NULL on both sides");
        }

        (c.cJSON_Delete)(ic);
        (r.cJSON_Delete)(ir);
        payload[N_ROW25] = 0x01;
    }

    drop(payload);
}
