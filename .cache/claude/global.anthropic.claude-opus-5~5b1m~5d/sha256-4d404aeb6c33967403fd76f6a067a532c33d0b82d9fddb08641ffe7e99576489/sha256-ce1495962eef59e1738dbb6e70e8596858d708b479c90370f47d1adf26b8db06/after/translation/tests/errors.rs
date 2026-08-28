//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Every rejection is asserted to be the *same* sentinel (`NULL` / `-1`) or the
//! *same* fatal signal on both sides — never merely "both failed somehow".

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn setrlimit(resource: i32, rlim: *const RLimit) -> i32;
}

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

const RLIMIT_AS: i32 = 9;

// ===========================================================================
// Row 1 — allocate_block: malloc(sizeof(MemoryBlock)) fails -> return NULL
//
// Triggered for real by capping the child's address space and draining the
// allocator, so `malloc(16)` genuinely returns NULL. Both libraries run in
// children forked from the identical parent state.
// ===========================================================================

/// Virtual size of this process in bytes, read *before* forking (reading
/// /proc allocates, which the child must not do).
fn vm_size_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let pages: u64 = s
        .split_whitespace()
        .next()
        .expect("statm field 0")
        .parse()
        .expect("statm field 0 is a number");
    pages * 4096
}

#[test]
fn row01_allocate_block_malloc_failure_returns_null() {
    let limit = vm_size_bytes() + 4 * 1024 * 1024;
    let _ = impls(); // dlopen both before forking

    // 1 = allocate_block returned NULL (expected), 0 = it succeeded,
    // 2 = drain loop never saw NULL, 3 = setrlimit refused.
    //
    // The drain deliberately runs through the library's OWN `allocate_block`
    // export rather than calling `malloc` from this test. LLVM recognises
    // `malloc` as an allocation function and will happily delete a drain loop
    // whose pointers are never used (and fold the null check), which silently
    // defeats the whole test. A call across the `.so` boundary is opaque.
    let probe = |im: &'static Impl| -> i32 {
        unsafe {
            let rl = RLimit {
                cur: limit,
                max: limit,
            };
            if setrlimit(RLIMIT_AS, &rl) != 0 {
                return 3;
            }
            let mut drained = false;
            'outer: for &count in &[65536usize, 4096, 256, 16, 1] {
                let mut spins = 0u64;
                loop {
                    if (im.allocate_block)(count, 0).is_null() {
                        if count == 1 {
                            drained = true;
                            break 'outer;
                        }
                        break;
                    }
                    spins += 1;
                    if spins > 2_000_000 {
                        return 2;
                    }
                }
            }
            if !drained {
                return 2;
            }
            // Now that the arena is exhausted, malloc(sizeof(MemoryBlock)) is
            // the FIRST allocation allocate_block attempts, so this exercises
            // the `if (!mb) return NULL;` branch specifically.
            let mb = (im.allocate_block)(1, 7);
            if mb.is_null() {
                1
            } else {
                0
            }
        }
    };

    let (cv, rv) = fork_both(probe);
    assert_eq!(
        cv, rv,
        "allocate_block must behave identically under malloc failure"
    );
    assert_eq!(
        cv,
        Outcome::Value(1),
        "expected both to return NULL when malloc fails (got {cv:?}); \
         2 = could not exhaust memory, 3 = setrlimit refused, 0 = allocation succeeded"
    );
}

// ===========================================================================
// Row 2 — allocate_block: calloc(count, 4) overflow -> free(mb), return NULL
// ===========================================================================

#[test]
fn row02_allocate_block_calloc_overflow_returns_null() {
    // Every one of these makes glibc's `nmemb * size` overflow check trip, so
    // calloc returns NULL *without* attempting a huge mapping.
    let counts: [usize; 8] = [
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 4 + 1,
        0x8000_0000_0000_0000,
        0x4000_0000_0000_0000,
        0xFFFF_FFFF_FFFF_FFFC,
        0x7FFF_FFFF_FFFF_FFFF,
    ];
    for &n in &counts {
        for &init in &[0, -1, i32::MAX, i32::MIN] {
            let cp = unsafe { (c().allocate_block)(n, init) };
            let rp = unsafe { (rs().allocate_block)(n, init) };
            assert!(
                cp.is_null(),
                "C: allocate_block({n:#x}, {init}) unexpectedly succeeded"
            );
            assert!(
                rp.is_null(),
                "Rust: allocate_block({n:#x}, {init}) returned non-NULL while C returned NULL"
            );
        }
    }
}

/// Pins the `int -> size_t` conversion boundary that `betagamma`'s
/// `block_size` depends on (`lib.c:126`).
///
/// C sign-extends, so `(param1 % 10) + 5 == -1` becomes `SIZE_MAX`, and glibc's
/// `nmemb * size` overflow check rejects it outright. A translation that
/// zero-extended instead would ask for `0xFFFF_FFFF` elements = 16 GiB.
/// Both counts are asserted to give the same answer from BOTH libraries, which
/// pins the whole reachable range (`(param1 % 10) + 5` in `-4..=-1`).
///
/// Caveat, recorded honestly: on this host a 16 GiB `calloc` also fails, so the
/// two conversions are observationally identical here and no black-box test can
/// separate them. On a host with >= 16 GiB of commit available the zero-extended
/// variant would instead succeed and then fill 4 billion ints, and this test
/// would diverge. The Rust matches C by construction (`as isize as usize`).
#[test]
fn row03_row04_int_to_size_t_conversion_boundary() {
    // The four counts reachable by sign-extension, and their zero-extended
    // counterparts.
    let sign_extended: [usize; 4] = [
        (-1i64) as usize,
        (-2i64) as usize,
        (-3i64) as usize,
        (-4i64) as usize,
    ];
    let zero_extended: [usize; 4] = [0xFFFF_FFFF, 0xFFFF_FFFE, 0xFFFF_FFFD, 0xFFFF_FFFC];
    for (&se, &ze) in sign_extended.iter().zip(zero_extended.iter()) {
        for &n in &[se, ze] {
            let cp = unsafe { (c().allocate_block)(n, 0) };
            let rp = unsafe { (rs().allocate_block)(n, 0) };
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "allocate_block({n:#x}): C NULL={} but Rust NULL={}",
                cp.is_null(),
                rp.is_null()
            );
            if !cp.is_null() {
                unsafe { (c().free_block)(cp) };
            }
            if !rp.is_null() {
                unsafe { (rs().free_block)(rp) };
            }
        }
    }
}

// ===========================================================================
// Rows 3 & 4 — betagamma: negative block_size -> both allocations NULL -> -1
// ===========================================================================

#[test]
fn row03_row04_betagamma_negative_block_size_returns_minus_one() {
    // (param1 % 10) + 5 < 0  <=>  param1 % 10 in {-6,-7,-8,-9}
    let mut cases: Vec<c_int> = Vec::new();
    for res in [-6, -7, -8, -9] {
        for k in 0..6 {
            cases.push(res - 10 * k);
        }
    }
    // INT_MIN % 10 == -8 in C (truncation toward zero) -> error path.
    cases.push(i32::MIN);
    cases.push(i32::MIN + 2); // % 10 == -6
    cases.push(-2_000_000_006);

    for &p1 in &cases {
        assert!(
            matches!(p1 % 10, -6 | -7 | -8 | -9),
            "test case {p1} does not actually hit the error path (residue {})",
            p1 % 10
        );
        let (cv, rv) = fork_both(|im| unsafe { (im.betagamma)(p1, 1, 2, 3) });
        assert_eq!(cv, rv, "betagamma({p1}, 1, 2, 3)");
        assert_eq!(
            cv,
            Outcome::Value(-1),
            "betagamma({p1}, ..) must return the -1 sentinel"
        );
    }

    // And with extreme param2..4, to prove the early return happens before any
    // of the arithmetic could matter.
    for &p1 in &[-6, -9, i32::MIN] {
        for &p in &[i32::MIN, -1, 0, 1, i32::MAX] {
            let (cv, rv) = fork_both(|im| unsafe { (im.betagamma)(p1, p, p, p) });
            assert_eq!(cv, rv, "betagamma({p1}, {p}, {p}, {p})");
            assert_eq!(cv, Outcome::Value(-1));
        }
    }
}

/// The mirror image of rows 3/4: residues that must *not* error.
#[test]
fn row03_row04_boundary_one_step_either_side() {
    // -5 -> block_size 0 -> OK;  -6 -> block_size -1 -> error.
    for (p1, want_error) in [
        (-5, false),
        (-6, true),
        (-15, false),
        (-16, true),
        (-4, false),
        (5, false),
        (-10, false),
        (-9, true),
        (-19, true),
        (-20, false),
    ] {
        let (cv, rv) = fork_both(|im| unsafe { (im.betagamma)(p1, 1, 1, 1) });
        assert_eq!(cv, rv, "betagamma({p1}, 1, 1, 1)");
        let is_error = cv == Outcome::Value(-1);
        assert_eq!(
            is_error, want_error,
            "betagamma({p1}, ..) error-ness wrong: got {cv:?}"
        );
    }
}

// ===========================================================================
// Row 5 — free_block(NULL) is a no-op
// ===========================================================================

#[test]
fn row05_free_block_null_is_noop() {
    let (cv, rv) = fork_both(|im| unsafe {
        for _ in 0..100 {
            (im.free_block)(std::ptr::null_mut());
        }
        1234
    });
    assert_eq!(cv, rv, "free_block(NULL)");
    assert_eq!(cv, Outcome::Value(1234), "free_block(NULL) must not crash");
}

// ===========================================================================
// Row 6 — free_block with mb->data == NULL skips the inner free
// ===========================================================================

#[test]
fn row06_free_block_null_data_field() {
    let (cv, rv) = fork_both(|im| unsafe {
        for size in [0usize, 1, usize::MAX] {
            let mb = malloc(std::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
            if mb.is_null() {
                return -2;
            }
            (*mb).data = std::ptr::null_mut();
            (*mb).size = size;
            (im.free_block)(mb);
        }
        5678
    });
    assert_eq!(cv, rv, "free_block(mb with data == NULL)");
    assert_eq!(cv, Outcome::Value(5678));
}

// ===========================================================================
// Rows 7 & 8 — compute_hash dereferences both pointers unchecked -> SIGSEGV
// ===========================================================================

#[test]
fn row07_compute_hash_null_mb1_segfaults() {
    let mut good = MemoryBlock {
        data: 0x1234_usize as *mut c_int,
        size: 1,
    };
    let g = &mut good as *mut MemoryBlock;
    let (cv, rv) = fork_both(move |im| unsafe { (im.compute_hash)(std::ptr::null_mut(), g) });
    assert_eq!(cv, rv, "compute_hash(NULL, valid)");
    assert_eq!(
        cv,
        Outcome::Signal(11),
        "compute_hash(NULL, valid) must SIGSEGV on both sides, got {cv:?}"
    );
}

#[test]
fn row08_compute_hash_null_mb2_segfaults() {
    let mut good = MemoryBlock {
        data: 0x1234_usize as *mut c_int,
        size: 1,
    };
    let g = &mut good as *mut MemoryBlock;
    let (cv, rv) = fork_both(move |im| unsafe { (im.compute_hash)(g, std::ptr::null_mut()) });
    assert_eq!(cv, rv, "compute_hash(valid, NULL)");
    assert_eq!(cv, Outcome::Signal(11), "got {cv:?}");
}

#[test]
fn row07_row08_compute_hash_both_null_segfaults() {
    let (cv, rv) =
        fork_both(|im| unsafe { (im.compute_hash)(std::ptr::null_mut(), std::ptr::null_mut()) });
    assert_eq!(cv, rv, "compute_hash(NULL, NULL)");
    assert_eq!(cv, Outcome::Signal(11), "got {cv:?}");
}

// ===========================================================================
// Row 9 — create_block(id, NULL, flags): strcpy from NULL -> SIGSEGV
// ===========================================================================

#[test]
fn row09_create_block_null_name_segfaults() {
    let (cv, rv) = fork_both(|im| unsafe {
        let b = (im.create_block)(1, std::ptr::null(), 0xAA);
        b.id // unreachable
    });
    assert_eq!(cv, rv, "create_block(1, NULL, 0xAA)");
    assert_eq!(
        cv,
        Outcome::Signal(11),
        "create_block with a NULL name must SIGSEGV on both sides, got {cv:?}"
    );
}

// ===========================================================================
// Row 10 — create_block performs NO length check on `name`
//
// Lengths 32..=35 overflow `char name[32]` but stay inside the 40-byte
// DataBlock (name is at offset 4, flags at 36, 3 tail padding bytes), so the
// result is fully observable and must match byte-for-byte including the
// clobbered `flags`. Longer names would smash the caller's frame (true UB) and
// are deliberately not compared.
// ===========================================================================

#[test]
fn row10_create_block_no_length_check_overflow_within_struct() {
    let mut r = Rng::new(0xE010);
    for len in 32..=35usize {
        for _ in 0..200 {
            let name: Vec<u8> = (0..len).map(|_| r.range(1, 255) as u8).collect();
            let buf = cstr(&name);
            let id = r.interesting_i32();
            let flags = r.next_u8();
            let cv = unsafe { (c().create_block)(id, buf.as_ptr(), flags) };
            let rv = unsafe { (rs().create_block)(id, buf.as_ptr(), flags) };
            // Full 32-byte name array plus id and the (possibly clobbered) flags.
            let cn: &[u8; 32] = unsafe { &*(cv.name.as_ptr() as *const [u8; 32]) };
            let rn: &[u8; 32] = unsafe { &*(rv.name.as_ptr() as *const [u8; 32]) };
            assert_eq!(cv.id, rv.id, "id, len={len}");
            assert_eq!(cn, rn, "name[0..32], len={len}");
            assert_eq!(
                cv.flags, rv.flags,
                "flags (clobbered by the overflow), len={len}"
            );
            // Sanity: the overflow reaches `flags` (offset 36) for len >= 32,
            // but `block.flags = flags;` runs *after* the strcpy in the C, so
            // the assignment wins and `flags` is restored. Only the 3 tail
            // padding bytes (offsets 37..39) keep the smashed values, and those
            // are indeterminate in C, so they are not compared.
            assert_eq!(
                cv.flags, flags,
                "C: block.flags = flags happens after strcpy, so it must win (len={len})"
            );
            assert_eq!(
                &cn[..],
                &name[..32],
                "C: name[0..32] must be the first 32 source bytes (len={len})"
            );
        }
    }
}

// ===========================================================================
// Generic FFI boundary cases (ERRORS.md G-rows)
// ===========================================================================

#[test]
fn g01_g02_allocate_block_zero_and_one() {
    for &(n, init) in &[(0usize, 0), (0, i32::MIN), (0, i32::MAX), (1, 0), (1, i32::MIN)] {
        let cp = unsafe { (c().allocate_block)(n, init) };
        let rp = unsafe { (rs().allocate_block)(n, init) };
        assert!(!cp.is_null(), "C: allocate_block({n}, {init}) returned NULL");
        assert!(
            !rp.is_null(),
            "Rust: allocate_block({n}, {init}) returned NULL while C did not"
        );
        unsafe {
            assert_eq!((*cp).size, n);
            assert_eq!((*rp).size, n);
            assert!(!(*cp).data.is_null() && !(*rp).data.is_null());
            for i in 0..n {
                assert_eq!(*(*cp).data.add(i), *(*rp).data.add(i), "data[{i}]");
            }
            (c().free_block)(cp);
            (rs().free_block)(rp);
        }
    }
}

#[test]
fn g03_allocate_block_init_value_wraps() {
    // init_value + i is computed in size_t, then truncated to int.
    for &init in &[i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, -1] {
        let n = 300usize;
        let cs = unsafe {
            let p = (c().allocate_block)(n, init);
            let v: Vec<c_int> = (0..n).map(|i| *(*p).data.add(i)).collect();
            (c().free_block)(p);
            v
        };
        let rsv = unsafe {
            let p = (rs().allocate_block)(n, init);
            let v: Vec<c_int> = (0..n).map(|i| *(*p).data.add(i)).collect();
            (rs().free_block)(p);
            v
        };
        assert_eq!(cs, rsv, "allocate_block({n}, {init}) wrap-around contents");
    }
}

#[test]
fn g04_compute_hash_aliased_is_zero() {
    let mut mb = MemoryBlock {
        data: 0xABCD_usize as *mut c_int,
        size: 9,
    };
    let p = &mut mb as *mut MemoryBlock;
    let cv = unsafe { (c().compute_hash)(p, p) };
    let rv = unsafe { (rs().compute_hash)(p, p) };
    assert_eq!(cv, rv);
    assert_eq!(cv, 0);
}

#[test]
fn g05_compute_hash_pointer_comparison_is_unsigned() {
    // If the Rust used signed comparison, 0xFFFF.. vs 0x1 would flip.
    let mut arr = [MemoryBlock {
        data: std::ptr::null_mut(),
        size: 0,
    }; 2];
    let base = arr.as_mut_ptr();
    let (p0, p1) = unsafe { (base.add(0), base.add(1)) };
    unsafe {
        (*p0).data = usize::MAX as *mut c_int; // "negative" if signed
        (*p1).data = 1usize as *mut c_int;
    }
    let cv = unsafe { (c().compute_hash)(p0, p1) };
    let rv = unsafe { (rs().compute_hash)(p0, p1) };
    assert_eq!(cv, rv);
    assert_eq!(cv, 210, "0xFFFF.. must compare GREATER than 0x1 (unsigned)");

    // NULL data on one side.
    unsafe {
        (*p0).data = std::ptr::null_mut();
        (*p1).data = 1usize as *mut c_int;
    }
    let cv = unsafe { (c().compute_hash)(p0, p1) };
    let rv = unsafe { (rs().compute_hash)(p0, p1) };
    assert_eq!(cv, rv);
    assert_eq!(cv, 110);
}

#[test]
fn g06_betagamma_extreme_params_do_not_trap() {
    for &p1 in &[0, 1, 5, 9, -1, -5] {
        for &p in &[i32::MIN, i32::MAX] {
            let (cv, rv) = fork_both(|im| unsafe { (im.betagamma)(p1, p, p, p) });
            assert_eq!(cv, rv, "betagamma({p1}, {p}, {p}, {p})");
            assert!(
                matches!(cv, Outcome::Value(_)),
                "must not trap on signed overflow: {cv:?}"
            );
        }
    }
    // param1 = INT_MAX: residue 7 -> block_size 12, and sum1 wraps hard.
    let (cv, rv) = fork_both(|im| unsafe { (im.betagamma)(i32::MAX, i32::MIN, i32::MAX, i32::MIN) });
    assert_eq!(cv, rv);
    assert!(matches!(cv, Outcome::Value(_)));
}

#[test]
fn g07_betagamma_block_size_zero_is_not_an_error() {
    for p1 in [-5, -15, -25, -105] {
        let (cv, rv) = fork_both(|im| unsafe { (im.betagamma)(p1, 3, 4, 5) });
        assert_eq!(cv, rv, "betagamma({p1}, 3, 4, 5)");
        assert_ne!(cv, Outcome::Value(-1), "block_size 0 must succeed");
    }
}

#[test]
fn g08_betagamma_division_truncates_toward_zero() {
    // Pick param1/param2 so (sum1 - sum2) is negative and not a multiple of 10.
    let cases = [
        (1, 2, 0, 0),
        (3, 8, 1, 1),
        (7, 13, 0, 0),
        (2, 5, 0, 0),
        (9, 100, 0, 0),
        (4, 7, 0, 0),
        (-1, 3, 0, 0),
        (-2, 9, 0, 0),
        (-3, 11, 0, 0),
        (-4, 21, 0, 0),
    ];
    for &(p1, p2, p3, p4) in &cases {
        let (cv, rv) = fork_both(|im| unsafe { (im.betagamma)(p1, p2, p3, p4) });
        assert_eq!(cv, rv, "betagamma({p1}, {p2}, {p3}, {p4})");
        assert!(matches!(cv, Outcome::Value(_)));
    }
}

// ---------------------------------------------------------------------------
// G9 — out-of-range values passed across the FFI boundary for a narrow
// parameter.
//
// This library declares no C enums, so the equivalent "a value with no valid
// variant" case is the `uint8_t flags` parameter: a caller compiled against a
// different prototype can push a full `int` into that argument slot. Under the
// x86-64 SysV ABI the upper bits of a narrow argument are unspecified, so both
// callees must ignore them identically.
// ---------------------------------------------------------------------------

type CreateBlockWideFn = unsafe extern "C" fn(c_int, *const c_char, c_int) -> DataBlock;

#[test]
fn g09_create_block_out_of_range_flags_across_ffi() {
    let buf = cstr(b"wide-flags");
    let cf: CreateBlockWideFn = unsafe { std::mem::transmute(c().create_block) };
    let rf: CreateBlockWideFn = unsafe { std::mem::transmute(rs().create_block) };
    let wide: [c_int; 12] = [
        0x100,
        0x1FF,
        0x1_0000,
        0x7FFF_FF00,
        -1,
        i32::MIN,
        i32::MAX,
        256,
        257,
        -256,
        -255,
        0xDEAD_BEEFu32 as c_int,
    ];
    for &w in &wide {
        let cv = unsafe { cf(11, buf.as_ptr(), w) };
        let rv = unsafe { rf(11, buf.as_ptr(), w) };
        assert_eq!(
            cv.observable(),
            rv.observable(),
            "create_block with out-of-range flags {w:#x} ({w})"
        );
    }
}

/// Same idea for `allocate_block`'s `int init_value` receiving an out-of-range
/// 64-bit value, and for `size_t count` receiving a "negative" value.
#[test]
fn g09_allocate_block_negative_count_across_ffi() {
    type WideFn = unsafe extern "C" fn(i64, c_int) -> *mut MemoryBlock;
    let cf: WideFn = unsafe { std::mem::transmute(c().allocate_block) };
    let rf: WideFn = unsafe { std::mem::transmute(rs().allocate_block) };
    // A caller passing a negative `int`-ish count: reinterpreted as a huge
    // size_t, calloc's overflow check trips -> NULL on both sides.
    for &n in &[-1i64, -4, -1000, i64::MIN, i64::MIN + 1] {
        let cp = unsafe { cf(n, 5) };
        let rp = unsafe { rf(n, 5) };
        assert!(cp.is_null(), "C: allocate_block({n}) should be NULL");
        assert!(
            rp.is_null(),
            "Rust: allocate_block({n}) returned non-NULL while C returned NULL"
        );
    }
}

#[test]
fn g09_free_block_dangling_low_pointers_reject_identically() {
    // Not a valid heap pointer: both must die the same way. 0x1 is unmapped.
    let (cv, rv) = fork_both(|im| unsafe {
        (im.free_block)(0x1usize as *mut MemoryBlock);
        7
    });
    assert_eq!(
        cv, rv,
        "free_block on an unmapped pointer must behave identically"
    );
}
