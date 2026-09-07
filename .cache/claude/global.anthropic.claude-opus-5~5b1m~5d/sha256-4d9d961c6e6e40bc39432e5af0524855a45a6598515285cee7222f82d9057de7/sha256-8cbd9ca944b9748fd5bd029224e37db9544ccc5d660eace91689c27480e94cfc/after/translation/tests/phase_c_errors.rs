//! Phase C — error-path differential tests. One test per ERRORS.md row, plus
//! the generic FFI-boundary boundaries (out-of-range "enum" values, null
//! pointers, zero/oversized lengths, one-past-range values).
//!
//! Each test asserts C and Rust return the *same specific* sentinel, not merely
//! that both "failed somehow".

mod common;

use common::*;

const E_MODE1_NOT_FOUND: i32 = 0o2 | 0o20; // 18
const E_MODE2_NOT_FOUND: i32 = 0o2 | 0o40; // 34
const E_MODE4_NOT_FOUND: i32 = 0o2 | 0o100; // 66
const E_BAD_MODE: i32 = 0o2 | 0o200; // 130
const STATUS_ERROR: i32 = 0o2; // 2

/// Guard + the shipped C/Rust pair.
///
/// The lock is mandatory for *every* test in this binary: `dlopen` of the same
/// path returns the same handle, so `node_count` / `node_storage` inside the
/// Rust `.so` are shared with the state-mutating tests below. Without it, a
/// concurrent `set_node_count` turns a happy assertion into a bogus failure.
fn shipped() -> (std::sync::MutexGuard<'static, ()>, Pair) {
    let g = common::lock_state();
    (g, Pair::shipped())
}

/// Guard + the instrumented C harness / Rust pair. `None` when the Rust `.so`
/// was built without `expose_init_test_data`.
fn instrumented() -> Option<(std::sync::MutexGuard<'static, ()>, Pair)> {
    let g = common::lock_state();
    Pair::instrumented().map(|p| (g, p))
}

macro_rules! instrumented {
    () => {
        match instrumented() {
            Some(p) => p,
            None => {
                eprintln!("skipped: Rust .so built without `expose_init_test_data`");
                return;
            }
        }
    };
}

/// Same, but for tests that need the shipped pair *and* the instrumented pair
/// while holding the lock only once.
fn both() -> (std::sync::MutexGuard<'static, ()>, Pair, Option<Pair>) {
    let g = common::lock_state();
    let s = Pair::shipped();
    let i = Pair::instrumented();
    (g, s, i)
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — mode 1, node not found -> STATUS_ERROR | 0020 == 18
// ---------------------------------------------------------------------------
#[test]
fn err_row1_mode1_node_not_found() {
    let (_g, sp, ip) = both();
    let (c, rs) = sp.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 0xC1);
    for _ in 0..5000 {
        let (id, d, f) = (r.interesting_i32(), r.interesting_i32(), r.interesting_i32());
        let gc = unsafe { c(1, id, d, f) };
        let gr = unsafe { rs(1, id, d, f) };
        assert_eq!(gc, gr, "jumpnode(1,{id},{d},{f})");
        assert_eq!(gc, E_MODE1_NOT_FOUND, "expected 18 from C for (1,{id},{d},{f})");
    }
    // and with a populated table, using ids that are definitely absent
    let Some(p) = ip else { return };
    let (ic, irs) = p.sym::<VoidFn>(b"jumpnode_initialize_test_data\0");
    unsafe {
        ic();
        irs();
    }
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for id in [0i32, -1, -2, 8, 9, 1000, i32::MAX, i32::MIN] {
        for d in [-1i32, 0, 1, 100] {
            let gc = unsafe { c(1, id, d, 0) };
            assert_eq!(gc, unsafe { rs(1, id, d, 0) });
            assert_eq!(gc, E_MODE1_NOT_FOUND, "populated, id={id}");
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — mode 2, node not found -> STATUS_ERROR | 0040 == 34
// ---------------------------------------------------------------------------
#[test]
fn err_row2_mode2_node_not_found() {
    let (_g, sp, ip) = both();
    let (c, rs) = sp.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 0xC2);
    for _ in 0..5000 {
        let (id, d, f) = (r.interesting_i32(), r.interesting_i32(), r.interesting_i32());
        let gc = unsafe { c(2, id, d, f) };
        let gr = unsafe { rs(2, id, d, f) };
        assert_eq!(gc, gr, "jumpnode(2,{id},{d},{f})");
        assert_eq!(gc, E_MODE2_NOT_FOUND);
    }
    let Some(p) = ip else { return };
    let (ic, irs) = p.sym::<VoidFn>(b"jumpnode_initialize_test_data\0");
    unsafe {
        ic();
        irs();
    }
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for id in [0i32, -1, 8, 1000, i32::MAX, i32::MIN] {
        // depth is irrelevant: the error return happens first, so even the
        // otherwise-UB negative depths are safe here.
        for d in [i32::MIN, -1, 0, 16, 17, i32::MAX] {
            let gc = unsafe { c(2, id, d, 7) };
            assert_eq!(gc, unsafe { rs(2, id, d, 7) });
            assert_eq!(gc, E_MODE2_NOT_FOUND, "populated, id={id}, d={d}");
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 — mode 4, node not found -> STATUS_ERROR | 0100 == 66
// ---------------------------------------------------------------------------
#[test]
fn err_row3_mode4_node_not_found() {
    let (_g, sp, ip) = both();
    let (c, rs) = sp.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 0xC3);
    for _ in 0..5000 {
        let (id, d, f) = (r.interesting_i32(), r.interesting_i32(), r.interesting_i32());
        let gc = unsafe { c(4, id, d, f) };
        assert_eq!(gc, unsafe { rs(4, id, d, f) }, "jumpnode(4,{id},{d},{f})");
        assert_eq!(gc, E_MODE4_NOT_FOUND);
    }
    let Some(p) = ip else { return };
    let (ic, irs) = p.sym::<VoidFn>(b"jumpnode_initialize_test_data\0");
    unsafe {
        ic();
        irs();
    }
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for id in [0i32, -1, 8, 1000, i32::MAX, i32::MIN] {
        let gc = unsafe { c(4, id, 3, 0) };
        assert_eq!(gc, unsafe { rs(4, id, 3, 0) });
        assert_eq!(gc, E_MODE4_NOT_FOUND, "populated, id={id}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — unknown operation_mode -> STATUS_ERROR | 0200 == 130
// ---------------------------------------------------------------------------
#[test]
fn err_row4_default_mode() {
    let (_g, p) = shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    let mut r = Rng::new(SEED ^ 0xC4);
    for _ in 0..20000 {
        let mode = r.next_i32();
        if (1..=4).contains(&mode) {
            continue;
        }
        let (id, d, f) = (r.interesting_i32(), r.interesting_i32(), r.interesting_i32());
        let gc = unsafe { c(mode, id, d, f) };
        assert_eq!(gc, unsafe { rs(mode, id, d, f) }, "mode={mode}");
        assert_eq!(gc, E_BAD_MODE, "mode={mode} must take the default: arm");
    }
}

/// Row 4 (cont.): exhaustive sweep of every out-of-range "enum" value near the
/// valid set. C enums/switches accept any `int`, so these are real inputs.
#[test]
fn err_out_of_range_mode_exhaustive() {
    let (_g, p) = shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for mode in -2000i32..=2000 {
        let gc = unsafe { c(mode, 1, 1, 1) };
        let gr = unsafe { rs(mode, 1, 1, 1) };
        assert_eq!(gc, gr, "mode={mode}");
        if !(1..=4).contains(&mode) {
            assert_eq!(gc, E_BAD_MODE, "mode={mode}");
        }
    }
}

/// Row 4 (cont.): the exact one-past boundaries of the documented 1..4 range,
/// and both integer extremes.
#[test]
fn err_mode_boundaries() {
    let (_g, p) = shipped();
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    // 0 is one below the range, 5 is one above; 0o5 / 0o10 catch octal mixups.
    for &mode in &[
        i32::MIN,
        i32::MIN + 1,
        -5,
        -1,
        0,
        5,
        6,
        8,
        0o10,
        0o377,
        255,
        256,
        i32::MAX - 1,
        i32::MAX,
    ] {
        let gc = unsafe { c(mode, 1, 1, 1) };
        assert_eq!(gc, unsafe { rs(mode, 1, 1, 1) }, "mode={mode}");
        assert_eq!(gc, E_BAD_MODE, "mode={mode}");
    }
    // The four valid modes must NOT hit the default arm.
    for mode in 1..=4 {
        assert_ne!(unsafe { c(mode, 1, 1, 1) }, E_BAD_MODE, "mode={mode}");
        assert_eq!(unsafe { c(mode, 1, 1, 1) }, unsafe { rs(mode, 1, 1, 1) });
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 5 — add_node past MAX_NODES -> STATUS_ERROR == 2, no mutation
// ---------------------------------------------------------------------------
#[test]
fn err_row5_add_node_capacity() {
    let (_g, p) = instrumented!();
    let (ca, ra) = p.sym::<AddNodeFn>(b"jumpnode_test_add_node\0");
    let (csc, rsc) = p.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");
    let (cg, rg) = p.sym::<GetCountFn>(b"jumpnode_test_get_node_count\0");
    let (cn, rn) = p.sym::<GetNodeFn>(b"jumpnode_test_get_node\0");

    // Exactly at capacity: at count 99 the insert succeeds and count becomes
    // 100; every insert after that is rejected.
    unsafe {
        csc(99);
        rsc(99);
    }
    let s1c = unsafe { ca(1000, -1, 2.5) };
    let s1r = unsafe { ra(1000, -1, 2.5) };
    assert_eq!(s1c, s1r);
    assert_eq!(s1c, 0, "insert at count=99 must return STATUS_OK");
    assert_eq!(unsafe { cg() }, unsafe { rg() });
    assert_eq!(unsafe { cg() }, 100);

    // Now every further insert is rejected, and node_count must not grow.
    let mut snap_c = (0i32, 0i32, 0f64, [0i32; 4]);
    unsafe {
        cn(99, &mut snap_c.0, &mut snap_c.1, &mut snap_c.2, snap_c.3.as_mut_ptr());
    }
    for k in 0..20 {
        let sc = unsafe { ca(7777 + k, 5, 9.75) };
        let sr = unsafe { ra(7777 + k, 5, 9.75) };
        assert_eq!(sc, sr, "rejected insert #{k}");
        assert_eq!(sc, STATUS_ERROR, "must be STATUS_ERROR (2), got {sc}");
        assert_eq!(unsafe { cg() }, unsafe { rg() });
        assert_eq!(unsafe { cg() }, 100, "node_count must stay pinned at 100");
    }
    // Last slot must be untouched by the rejected inserts, on both sides.
    let mut a = (0i32, 0i32, 0f64, [0i32; 4]);
    let mut b = (0i32, 0i32, 0f64, [0i32; 4]);
    unsafe {
        cn(99, &mut a.0, &mut a.1, &mut a.2, a.3.as_mut_ptr());
        rn(99, &mut b.0, &mut b.1, &mut b.2, b.3.as_mut_ptr());
    }
    assert_eq!(a.0, b.0);
    assert_eq!(a.1, b.1);
    assert_eq!(a.2.to_bits(), b.2.to_bits());
    assert_eq!(a.3, b.3);
    assert_eq!(a.0, snap_c.0, "slot 99 changed despite rejection");

    // Counts already above MAX_NODES are rejected too (>= not ==).
    for count in [100i32, 101, 150, 1000, i32::MAX] {
        unsafe {
            csc(count);
            rsc(count);
        }
        let sc = unsafe { ca(1, -1, 0.0) };
        assert_eq!(sc, unsafe { ra(1, -1, 0.0) }, "count={count}");
        assert_eq!(sc, STATUS_ERROR, "count={count} must be rejected");
        assert_eq!(unsafe { cg() }, unsafe { rg() });
    }
    // Leave the shared state at the shipped baseline for whatever runs next.
    unsafe {
        csc(0);
        rsc(0);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 6/7 — safe_double_to_int clamps
// ---------------------------------------------------------------------------
#[test]
fn err_row6_clamp_high() {
    let (_g, p) = instrumented!();
    let (cd, rd) = p.sym::<D2IFn>(b"jumpnode_test_safe_double_to_int\0");
    for &v in &[
        2147483647.0f64,
        2147483647.0000001,
        2147483647.5,
        2147483648.0,
        4e9,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ] {
        let gc = unsafe { cd(v) };
        assert_eq!(gc, unsafe { rd(v) }, "clamp-high v={v}");
        if v > 2147483647.0 {
            assert_eq!(gc, i32::MAX, "v={v} must clamp to 2147483647");
        }
    }
    // Reached end to end through mode 4 with a huge depth.
    let (ic, irs) = p.sym::<VoidFn>(b"jumpnode_initialize_test_data\0");
    unsafe {
        ic();
        irs();
    }
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for &depth in &[i32::MAX, i32::MAX - 1, 2_000_000_000, 1_000_000_000] {
        let gc = unsafe { c(4, 1, depth, 0) };
        assert_eq!(gc, unsafe { rs(4, 1, depth, 0) }, "mode4 depth={depth}");
    }
}

#[test]
fn err_row7_clamp_low() {
    let (_g, p) = instrumented!();
    let (cd, rd) = p.sym::<D2IFn>(b"jumpnode_test_safe_double_to_int\0");
    for &v in &[
        -2147483648.0f64,
        -2147483648.5,
        -2147483649.0,
        -4e9,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
    ] {
        let gc = unsafe { cd(v) };
        assert_eq!(gc, unsafe { rd(v) }, "clamp-low v={v}");
        assert_eq!(gc, i32::MIN, "v={v} must clamp to -2147483648");
    }
    let (ic, irs) = p.sym::<VoidFn>(b"jumpnode_initialize_test_data\0");
    unsafe {
        ic();
        irs();
    }
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for &depth in &[i32::MIN, i32::MIN + 1, -2_000_000_000, -1_000_000_000] {
        let gc = unsafe { c(4, 1, depth, 0) };
        assert_eq!(gc, unsafe { rs(4, 1, depth, 0) }, "mode4 depth={depth}");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 8 — process_backward with start_offset >= size -> 0 iterations
// ---------------------------------------------------------------------------
#[test]
fn err_row8_depth_ge_size() {
    let (_g, p) = instrumented!();
    let (cp, rp) = p.sym::<ProcBackFn>(b"jumpnode_test_process_backward\0");
    let mut buf: Vec<i32> = (0..64).map(|i| (i as i32) * 7 + 1).collect();
    let base = unsafe { buf.as_mut_ptr().add(16) };
    for size in [0usize, 1, 8, 16, 32] {
        for extra in 0..8i32 {
            let off = size as i32 + extra;
            let gc = unsafe { cp(base, size, off) };
            assert_eq!(gc, unsafe { rp(base, size, off) }, "size={size} off={off}");
            assert_eq!(gc, 0, "start_offset {off} >= size {size} must sum to 0");
        }
    }

    // End to end through mode 2: depth >= 16 leaves only the 16*flags term.
    let (ic, irs) = p.sym::<VoidFn>(b"jumpnode_initialize_test_data\0");
    unsafe {
        ic();
        irs();
    }
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for depth in [16i32, 17, 20, 1000, i32::MAX] {
        for flags in [0i32, 1, -1, 1000, -1000] {
            let gc = unsafe { c(2, 1, depth, flags) };
            assert_eq!(gc, unsafe { rs(2, 1, depth, flags) }, "d={depth} f={flags}");
            assert_eq!(
                gc,
                16i32.wrapping_mul(flags),
                "depth={depth} must contribute nothing"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary checks
// ---------------------------------------------------------------------------

/// Null / empty / oversized inputs to the pointer-taking helpers.
#[test]
fn err_generic_pointer_and_length_boundaries() {
    let (_g, p) = instrumented!();

    // compute_size_metric on the empty string (length 0) and on a very long one.
    let (cm, rm) = p.sym::<MetricFn>(b"jumpnode_test_compute_size_metric\0");
    let empty: [i8; 1] = [0];
    assert_eq!(unsafe { cm(empty.as_ptr()) }, unsafe {
        rm(empty.as_ptr())
    });
    assert_eq!(unsafe { cm(empty.as_ptr()) }, 8, "0*2 + 010 == 8");
    for len in [1usize, 1000, 100_000] {
        let mut s: Vec<i8> = vec![b'a' as i8; len];
        s.push(0);
        assert_eq!(
            unsafe { cm(s.as_ptr()) },
            unsafe { rm(s.as_ptr()) },
            "len={len}"
        );
    }

    // process_backward with size 0 (empty range) and with a huge size that is
    // still valid because start_offset is far past it (no dereference happens).
    let (cp, rp) = p.sym::<ProcBackFn>(b"jumpnode_test_process_backward\0");
    let mut buf: Vec<i32> = vec![3; 8];
    let base = buf.as_mut_ptr();
    assert_eq!(unsafe { cp(base, 0, 0) }, unsafe { rp(base, 0, 0) });
    assert_eq!(unsafe { cp(base, 0, 0) }, 0);
    // size == start_offset for a range of sizes: always zero iterations.
    for n in [0usize, 1, 8, 4096, usize::from(u16::MAX)] {
        let off = n as i32;
        assert_eq!(
            unsafe { cp(base, n, off) },
            unsafe { rp(base, n, off) },
            "size=off={n}"
        );
    }

    // get_node index bounds: negative, 0, MAX_NODES-1, MAX_NODES, past it.
    let (cn, rn) = p.sym::<GetNodeFn>(b"jumpnode_test_get_node\0");
    for idx in [i32::MIN, -100, -1, 0, 1, 99, 100, 101, 1000, i32::MAX] {
        let mut a = (0i32, 0i32, 0f64, [0i32; 4]);
        let mut b = (0i32, 0i32, 0f64, [0i32; 4]);
        let sc = unsafe { cn(idx, &mut a.0, &mut a.1, &mut a.2, a.3.as_mut_ptr()) };
        let sr = unsafe { rn(idx, &mut b.0, &mut b.1, &mut b.2, b.3.as_mut_ptr()) };
        assert_eq!(sc, sr, "get_node({idx}) status");
        if !(0..100).contains(&idx) {
            assert_eq!(sc, -1, "get_node({idx}) must be rejected");
        }
    }

    // find_node_by_id with a negative node_count: the `i < node_count` loop must
    // run zero times on both sides rather than walking backwards.
    let (csc, rsc) = p.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");
    let (cf, rf) = p.sym::<FindIdxFn>(b"jumpnode_test_find_node_index\0");
    for count in [-1i32, -100, i32::MIN, 0] {
        unsafe {
            csc(count);
            rsc(count);
        }
        for id in [0i32, 1, -1, i32::MAX, i32::MIN] {
            let gc = unsafe { cf(id) };
            assert_eq!(gc, unsafe { rf(id) }, "count={count} id={id}");
            assert_eq!(gc, -1, "count={count} must find nothing");
        }
    }
    // Restore a sane state for any test that runs after this one.
    unsafe {
        csc(0);
        rsc(0);
    }
}

/// A negative `node_count` must also make `jumpnode`'s three lookup arms return
/// their not-found sentinels, and must keep mode 4's `node_count > 2` block off.
#[test]
fn err_generic_negative_node_count_through_jumpnode() {
    let (_g, p) = instrumented!();
    let (csc, rsc) = p.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");
    let (ic, irs) = p.sym::<VoidFn>(b"jumpnode_initialize_test_data\0");
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    unsafe {
        ic();
        irs();
    }
    for count in [-1i32, -7, -100, i32::MIN] {
        unsafe {
            csc(count);
            rsc(count);
        }
        for id in [1i32, 2, 7, 0, -1] {
            for d in [0i32, 1, 5] {
                let g1 = unsafe { c(1, id, d, 0) };
                assert_eq!(g1, unsafe { rs(1, id, d, 0) });
                assert_eq!(g1, E_MODE1_NOT_FOUND, "count={count}");
                let g2 = unsafe { c(2, id, d, 3) };
                assert_eq!(g2, unsafe { rs(2, id, d, 3) });
                assert_eq!(g2, E_MODE2_NOT_FOUND, "count={count}");
                let g4 = unsafe { c(4, id, d, 0) };
                assert_eq!(g4, unsafe { rs(4, id, d, 0) });
                assert_eq!(g4, E_MODE4_NOT_FOUND, "count={count}");
            }
        }
        // mode 3 and default are state-independent
        assert_eq!(unsafe { c(3, 5, 5, 5) }, unsafe { rs(3, 5, 5, 5) });
        assert_eq!(unsafe { c(0, 5, 5, 5) }, unsafe { rs(0, 5, 5, 5) });
        assert_eq!(unsafe { c(0, 5, 5, 5) }, E_BAD_MODE);
    }
    unsafe {
        csc(0);
        rsc(0);
    }
}

/// A dangling `parent_id` makes mode 1's walk `break` instead of erroring — the
/// C returns the partial accumulation, not an error code.
#[test]
fn err_generic_dangling_parent_breaks_not_errors() {
    let (_g, p) = instrumented!();
    let (ca, ra) = p.sym::<AddNodeFn>(b"jumpnode_test_add_node\0");
    let (csc, rsc) = p.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    unsafe {
        csc(0);
        rsc(0);
    }
    // id 10 -> parent 999 (absent); id 11 -> parent 10 -> 999 (absent)
    for (id, parent, v) in [(10i32, 999i32, 11.5f64), (11, 10, 22.25), (12, 11, 33.125)] {
        assert_eq!(unsafe { ca(id, parent, v) }, unsafe { ra(id, parent, v) });
    }
    for id in [10i32, 11, 12] {
        for d in -2..=10 {
            let gc = unsafe { c(1, id, d, 0) };
            assert_eq!(gc, unsafe { rs(1, id, d, 0) }, "id={id} d={d}");
            assert_ne!(
                gc, E_MODE1_NOT_FOUND,
                "a dangling parent must break, not error (id={id}, d={d})"
            );
        }
    }
    // Self-parenting and a 2-cycle: the walk is bounded only by `depth`.
    unsafe {
        csc(0);
        rsc(0);
    }
    for (id, parent, v) in [(20i32, 20i32, 5.5f64), (21, 22, 1.25), (22, 21, 2.75)] {
        assert_eq!(unsafe { ca(id, parent, v) }, unsafe { ra(id, parent, v) });
    }
    for id in [20i32, 21, 22] {
        for d in [0i32, 1, 2, 3, 10, 50, 1000, 100_000] {
            let gc = unsafe { c(1, id, d, 0) };
            assert_eq!(gc, unsafe { rs(1, id, d, 0) }, "cycle id={id} d={d}");
        }
    }
    unsafe {
        csc(0);
        rsc(0);
    }
}

/// `parent_id == -1` is the documented terminator; `-2` and other negatives are
/// *not*, and must go down the ordinary lookup path.
#[test]
fn err_generic_parent_sentinel_is_exactly_minus_one() {
    let (_g, p) = instrumented!();
    let (ca, ra) = p.sym::<AddNodeFn>(b"jumpnode_test_add_node\0");
    let (csc, rsc) = p.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");
    let (c, rs) = p.sym::<JumpnodeFn>(b"jumpnode\0");
    for &sentinel in &[-1i32, -2, 0, i32::MIN, i32::MAX] {
        unsafe {
            csc(0);
            rsc(0);
        }
        for (id, parent, v) in [(1i32, sentinel, 100.5f64), (2, 1, 50.25)] {
            assert_eq!(unsafe { ca(id, parent, v) }, unsafe { ra(id, parent, v) });
        }
        for id in [1i32, 2] {
            for d in 0..=6 {
                assert_eq!(
                    unsafe { c(1, id, d, 0) },
                    unsafe { rs(1, id, d, 0) },
                    "sentinel={sentinel} id={id} d={d}"
                );
            }
        }
    }
    unsafe {
        csc(0);
        rsc(0);
    }
}

/// Regression test for the `add_node` capacity guard's *signedness*.
///
/// C: `if (node_count >= MAX_NODES)` — a **signed** `int` compare, so a negative
/// `node_count` does NOT trip the guard.
/// A translation that writes `NODE_COUNT as usize >= MAX_NODES` turns the
/// negative value into a huge unsigned one and rejects the insert instead.
#[test]
fn err_add_node_capacity_guard_is_a_signed_compare() {
    let (_g, p) = instrumented!();
    let (ca, ra) = p.sym::<AddNodeFn>(b"jumpnode_test_add_node\0");
    let (csc, rsc) = p.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");
    let (cg, rg) = p.sym::<GetCountFn>(b"jumpnode_test_get_node_count\0");

    // NEGATIVE `node_count` IS NOT DIFFERENTIALLY TESTABLE — see ERRORS.md row U4.
    //
    // C's guard is a signed compare, so a negative count does not trip it and the
    // function proceeds to store into `node_storage[negative]`. In this link
    // `node_storage[-1]` overlaps `node_count` *exactly* (`nm` shows node_count at
    // 0x4040, node_storage at 0x4060 — precisely sizeof(Node) apart), so
    // `node_storage[node_count].id = id` overwrites `node_count` with `id`. Every
    // subsequent field assignment then re-reads that clobbered index, storing
    // ~135 KB past the 3200-byte array, and the C SIGSEGVs. The C cannot survive
    // this input, so there is nothing to compare against.
    //
    // The guard's *signedness* is still fixed in the Rust to match the C
    // (`NODE_COUNT >= MAX_NODES as c_int`, not `as usize >= MAX_NODES`), because
    // the unsigned form rejects inputs the C accepts. What that test CAN pin down
    // is the whole non-negative range, below.

    // Non-negative counts across and around the boundary must agree exactly.
    for count in [0i32, 1, 50, 98, 99, 100, 101] {
        unsafe {
            csc(count);
            rsc(count);
        }
        let sc = unsafe { ca(77, -1, 2.5) };
        let cc = unsafe { cg() };
        unsafe {
            csc(count);
            rsc(count);
        }
        let sr = unsafe { ra(77, -1, 2.5) };
        let cr = unsafe { rg() };
        assert_eq!(sc, sr, "status at count={count}");
        assert_eq!(cc, cr, "count after at count={count}");
        assert_eq!(sc, if count >= 100 { STATUS_ERROR } else { 0 });
    }
    unsafe {
        csc(0);
        rsc(0);
    }
}
