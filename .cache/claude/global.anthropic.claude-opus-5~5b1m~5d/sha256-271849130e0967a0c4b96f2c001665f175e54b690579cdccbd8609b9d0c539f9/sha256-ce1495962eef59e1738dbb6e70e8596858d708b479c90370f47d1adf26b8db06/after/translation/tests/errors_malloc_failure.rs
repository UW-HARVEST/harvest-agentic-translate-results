// Phase C — ERRORS.md rows E7 and E9: the `malloc(...) == NULL -> return -1`
// branches that cannot be reached by ordinary inputs.
//
// Both branches are made reachable deterministically by *interposing* `malloc`
// from this test executable. On glibc/ELF the executable's definition of
// `malloc` preempts libc's for every caller in the process, including both
// `dlopen`ed shared objects. The C `.so` calls libc `malloc` directly, and the
// Rust `.so` deliberately imports libc `malloc` too, so a single interposer
// forces the identical failure in both implementations -- which is exactly what
// makes this a fair differential test.
//
// This file intentionally contains ONE `#[test]` so that no other test thread
// can be allocating while the interposer is armed.

mod common;

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Byte-size that `malloc` must refuse while armed. 0 == disarmed.
static FAIL_SIZE: AtomicUsize = AtomicUsize::new(0);
/// Set once the interposer has been observed to actually run.
static INTERPOSED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" {
    /// glibc's real allocator, reachable even when `malloc` is interposed.
    fn __libc_malloc(n: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn malloc(n: usize) -> *mut c_void {
    INTERPOSED.store(true, Ordering::Relaxed);
    let fail = FAIL_SIZE.load(Ordering::Relaxed);
    if fail != 0 && n == fail {
        return ptr::null_mut();
    }
    unsafe { __libc_malloc(n) }
}

fn arm(bytes: usize) {
    FAIL_SIZE.store(bytes, Ordering::Relaxed);
}

fn disarm() {
    FAIL_SIZE.store(0, Ordering::Relaxed);
}

/// `sizeof(DataPoint)` == `int` + padding + `double` == 16 on x86-64 SysV.
const SIZEOF_DATAPOINT: usize = 16;
/// `5 * sizeof(int)` -- the allocation `fallcalc` makes for `data_array`.
const FALLCALC_BYTES: usize = 5 * 4;

#[test]
fn e7_and_e9_forced_malloc_failure() {
    let (c, r) = common::both();

    // ---------------------------------------------------------------------
    // Confirm the interposer is really in the call path before drawing any
    // conclusions from it; otherwise the "failures" below would be vacuous.
    // ---------------------------------------------------------------------
    // sum(i*8 * i*1.5) for i in 0..3 = 0 + 12 + 48 = 60
    let c_probe = unsafe { (c.allocate_and_compute)(3, 1.5) };
    let r_probe = unsafe { (r.allocate_and_compute)(3, 1.5) };
    assert_eq!(c_probe, r_probe, "sanity: C and Rust must agree un-armed");
    assert_eq!(c_probe, 60, "sanity: allocate_and_compute(3, 1.5) == 60");
    assert!(
        INTERPOSED.load(Ordering::Relaxed),
        "malloc interposition is NOT active -- E7/E9 cannot be tested this way. \
         Expected the executable's `malloc` to preempt libc's for dlopen'ed .so's."
    );

    // =====================================================================
    // E7 -- allocate_and_compute: `malloc` fails for a positive `size`.
    //
    // Driven for every size in 1..=256 plus larger ones: for each, arm the
    // interposer with exactly `size * 16` bytes so the allocation inside
    // allocate_and_compute is the one that fails, and require BOTH libraries
    // to return the -1 sentinel.
    // =====================================================================
    let mut sizes: Vec<i32> = (1..=256).collect();
    sizes.extend_from_slice(&[
        257, 511, 512, 1000, 4096, 65535, 65536, 1 << 20, 1 << 24, 1 << 26, i32::MAX / 16, i32::MAX,
    ]);

    for size in sizes {
        let bytes = (size as usize).wrapping_mul(SIZEOF_DATAPOINT);

        arm(bytes);
        let cv = unsafe { (c.allocate_and_compute)(size, 1.5) };
        let rv = unsafe { (r.allocate_and_compute)(size, 1.5) };
        disarm();

        assert_eq!(
            cv, rv,
            "[E7] DIVERGENCE with size={size} (malloc({bytes}) forced to fail): \
             C returned {cv}, Rust returned {rv}"
        );
        assert_eq!(
            cv, -1,
            "[E7] size={size}: malloc({bytes}) was forced to fail, so the \
             function must return the -1 sentinel"
        );
    }

    // The same, across every multiplier class, to prove the failure path is
    // taken before `multiplier` is ever looked at.
    for m in [
        1.5f64,
        0.0,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN_POSITIVE,
    ] {
        let size = 7i32;
        arm(size as usize * SIZEOF_DATAPOINT);
        let cv = unsafe { (c.allocate_and_compute)(size, m) };
        let rv = unsafe { (r.allocate_and_compute)(size, m) };
        disarm();
        assert_eq!(cv, rv, "[E7] DIVERGENCE with size={size} mult={m:?}");
        assert_eq!(cv, -1, "[E7] size={size} mult={m:?} must return -1");
    }

    // Also verify the *un*forced huge-size request agrees. `malloc` is left
    // un-armed here; if the OS happens to satisfy a ~34 GB request we skip
    // rather than touch that much memory.
    {
        let size = i32::MAX;
        let bytes = (size as usize).wrapping_mul(SIZEOF_DATAPOINT);
        let probe = unsafe { __libc_malloc(bytes) };
        if probe.is_null() {
            let cv = unsafe { (c.allocate_and_compute)(size, 1.5) };
            let rv = unsafe { (r.allocate_and_compute)(size, 1.5) };
            assert_eq!(cv, rv, "[E7/natural] DIVERGENCE with size={size}");
            assert_eq!(cv, -1, "[E7/natural] size={size} must return -1");
        } else {
            unsafe { libc_free(probe) };
            eprintln!(
                "[E7/natural] note: the OS satisfied a {bytes}-byte request; \
                 skipping the un-forced variant (the forced variant above \
                 already covers this branch)."
            );
        }
    }

    // =====================================================================
    // E9 -- fallcalc: its own `malloc(5 * sizeof(int))` fails, so fallcalc
    // returns -1 *unmasked* (before `result &= 0777`). This is the only way
    // fallcalc can produce a value outside 0..=511.
    // =====================================================================
    let params: Vec<(i32, i32, i32, i32)> = vec![
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -1, -1, -1),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (7, -13, 129, -9),
        (12345, -67890, 1000, 77),
        (-3, 5, 128, 0),
        (0, 0, 129, -2),
    ];

    for (p1, p2, p3, p4) in params {
        arm(FALLCALC_BYTES);
        let cv = unsafe { (c.fallcalc)(p1, p2, p3, p4) };
        let rv = unsafe { (r.fallcalc)(p1, p2, p3, p4) };
        disarm();

        assert_eq!(
            cv, rv,
            "[E9] DIVERGENCE with fallcalc({p1}, {p2}, {p3}, {p4}) and \
             malloc({FALLCALC_BYTES}) forced to fail: C={cv}, Rust={rv}"
        );
        assert_eq!(
            cv, -1,
            "[E9] fallcalc({p1}, {p2}, {p3}, {p4}) must return the unmasked -1 \
             sentinel when its data_array allocation fails"
        );
    }

    // =====================================================================
    // E9 x E7 -- both allocations forced to fail is impossible to arm at once
    // (different sizes), but arming only the DataPoint size exercises the
    // nested failure: fallcalc's own malloc succeeds while the inner
    // allocate_and_compute fails, for every reachable inner size 1..=10.
    // =====================================================================
    for p4 in 0..10i32 {
        let inner_size = p4.wrapping_rem(10).wrapping_add(1);
        assert!((1..=10).contains(&inner_size));
        arm(inner_size as usize * SIZEOF_DATAPOINT);
        let cv = unsafe { (c.fallcalc)(3, -4, 5, p4) };
        let rv = unsafe { (r.fallcalc)(3, -4, 5, p4) };
        disarm();
        assert_eq!(
            cv, rv,
            "[E9/nested] DIVERGENCE with fallcalc(3, -4, 5, {p4}) and the inner \
             {inner_size}-element allocation forced to fail: C={cv}, Rust={rv}"
        );
        assert!(
            (0..=511).contains(&cv),
            "[E9/nested] the inner -1 must be folded and masked, got {cv}"
        );
    }

    // Finally: with the interposer disarmed everything is back to normal, so a
    // forced failure did not leave either library in a broken state.
    for (p1, p2, p3, p4) in [(1, 2, 3, 4), (i32::MIN, i32::MAX, -7, -9)] {
        let cv = unsafe { (c.fallcalc)(p1, p2, p3, p4) };
        let rv = unsafe { (r.fallcalc)(p1, p2, p3, p4) };
        assert_eq!(cv, rv, "[E9/recovery] DIVERGENCE after disarming");
        assert!((0..=511).contains(&cv), "[E9/recovery] unexpected {cv}");
    }
}

unsafe extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}
