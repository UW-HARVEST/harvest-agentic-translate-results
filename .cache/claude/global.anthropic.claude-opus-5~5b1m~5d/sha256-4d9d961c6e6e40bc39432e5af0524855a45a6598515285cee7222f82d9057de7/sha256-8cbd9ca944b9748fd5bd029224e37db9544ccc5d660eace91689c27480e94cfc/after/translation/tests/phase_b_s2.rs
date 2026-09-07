//! Phase B — valid-path differential tests, surface S2 (populated node storage).
//! CONFIGS.md rows 11-25, plus direct differential probes of every `static`
//! helper in `c_src/src/lib.c`.
//!
//! The C side is `tests/c_harness/harness.c`, which `#include`s
//! `c_src/src/lib.c` verbatim (c_src is never modified). The Rust side is the
//! `cdylib` built with `--features expose_init_test_data`. Both are loaded via
//! `libloading`; nothing is called directly.
//!
//! Every test no-ops when the Rust `.so` was built without the feature.

mod common;

use common::*;
use libloading::Symbol;

/// Ids present after `initialize_test_data`, plus ids that are absent.
const LIVE_IDS: &[i32] = &[1, 2, 3, 4, 5, 6, 7];
const DEAD_IDS: &[i32] = &[0, -1, 8, 9, 100, -100, i32::MAX, i32::MIN];

struct Ctx {
    pair: Pair,
    /// Held for the whole test: `node_storage` / `node_count` live in the
    /// `.so`s' data segments and are shared by every test in this binary.
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl Ctx {
    fn new() -> Option<Ctx> {
        let guard = common::lock_state();
        Pair::instrumented().map(|pair| Ctx {
            pair,
            _guard: guard,
        })
    }
    fn jump(&self) -> (Symbol<'_, JumpnodeFn>, Symbol<'_, JumpnodeFn>) {
        self.pair.sym::<JumpnodeFn>(b"jumpnode\0")
    }
    fn init(&self) -> (Symbol<'_, VoidFn>, Symbol<'_, VoidFn>) {
        self.pair.sym::<VoidFn>(b"jumpnode_initialize_test_data\0")
    }
}

macro_rules! ctx {
    () => {
        match Ctx::new() {
            Some(c) => c,
            None => {
                eprintln!("skipped: Rust .so built without `expose_init_test_data`");
                return;
            }
        }
    };
}

/// Calls both inits so C and Rust start from identical state.
fn init_both(ctx: &Ctx) {
    let (ic, irs) = ctx.init();
    unsafe {
        ic();
        irs();
    }
}

#[track_caller]
fn diff(
    label: &str,
    c: &Symbol<'_, JumpnodeFn>,
    rs: &Symbol<'_, JumpnodeFn>,
    m: i32,
    id: i32,
    d: i32,
    f: i32,
) {
    assert_eq_call(label, c, rs, m, id, d, f);
}

// ---------------------------------------------------------------------------
// Premise checks: the harness and the Rust hooks agree on state layout.
// ---------------------------------------------------------------------------

#[test]
fn s2_state_layout_matches() {
    let ctx = ctx!();
    let (cs, rss) = ctx.pair.sym::<IntFn>(b"jumpnode_test_sizeof_node\0");
    assert_eq!(unsafe { cs() }, unsafe { rss() }, "sizeof(Node)");
    let (cm, rsm) = ctx.pair.sym::<IntFn>(b"jumpnode_test_max_nodes\0");
    assert_eq!(unsafe { cm() }, unsafe { rsm() }, "MAX_NODES");

    init_both(&ctx);
    let (cg, rsg) = ctx.pair.sym::<GetCountFn>(b"jumpnode_test_get_node_count\0");
    assert_eq!(unsafe { cg() }, unsafe { rsg() }, "node_count after init");
    assert_eq!(unsafe { cg() }, 7);

    // Every stored node must be byte-identical.
    let (cn, rsn) = ctx.pair.sym::<GetNodeFn>(b"jumpnode_test_get_node\0");
    for i in -2..102 {
        let mut a = (0i32, 0i32, 0f64, [0i32; 4]);
        let mut b = (0i32, 0i32, 0f64, [0i32; 4]);
        let rc = unsafe { cn(i, &mut a.0, &mut a.1, &mut a.2, a.3.as_mut_ptr()) };
        let rr = unsafe { rsn(i, &mut b.0, &mut b.1, &mut b.2, b.3.as_mut_ptr()) };
        assert_eq!(rc, rr, "get_node({i}) status");
        if rc == 0 {
            assert_eq!(a.0, b.0, "node[{i}].id");
            assert_eq!(a.1, b.1, "node[{i}].parent_id");
            assert_eq!(a.2.to_bits(), b.2.to_bits(), "node[{i}].value bits");
            assert_eq!(a.3, b.3, "node[{i}].data");
        }
    }
}

// ---------------------------------------------------------------------------
// Direct differential probes of the file-static helpers.
// ---------------------------------------------------------------------------

/// `find_node_by_id` over live and dead ids, and at every `node_count` cutoff.
#[test]
fn s2_find_node_by_id() {
    let ctx = ctx!();
    init_both(&ctx);
    let (cf, rf) = ctx.pair.sym::<FindIdxFn>(b"jumpnode_test_find_node_index\0");
    let (csc, rsc) = ctx.pair.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");

    let mut r = Rng::new(SEED ^ 0x11);
    for count in 0..=7 {
        unsafe {
            csc(count);
            rsc(count);
        }
        for &id in LIVE_IDS.iter().chain(DEAD_IDS) {
            assert_eq!(
                unsafe { cf(id) },
                unsafe { rf(id) },
                "find_node_by_id({id}) at node_count={count}"
            );
        }
        for _ in 0..500 {
            let id = r.interesting_i32();
            assert_eq!(unsafe { cf(id) }, unsafe { rf(id) }, "find({id})@{count}");
        }
    }
}

/// `safe_double_to_int` across both clamps, the exact boundaries, subnormals
/// and negative zero.
#[test]
fn s2_safe_double_to_int() {
    let ctx = ctx!();
    let (cd, rd) = ctx.pair.sym::<D2IFn>(b"jumpnode_test_safe_double_to_int\0");

    let mut cases: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        2147483646.0,
        2147483647.0,
        2147483647.5,
        2147483648.0,
        1e18,
        f64::MAX,
        f64::INFINITY,
        -2147483647.0,
        -2147483648.0,
        -2147483648.5,
        -2147483649.0,
        -1e18,
        f64::MIN,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        // values relevant to the actual computations
        133.65971,
        276.3125,
        100.5,
        -100.5,
    ];
    let mut r = Rng::new(SEED ^ 0x12);
    for _ in 0..5000 {
        let bits = r.next_u64();
        // random finite doubles over a wide but finite range
        let v = (bits as i64 as f64) / (1u64 << r.range(0, 40) as u32) as f64;
        if v.is_finite() {
            cases.push(v);
        }
    }
    for _ in 0..2000 {
        cases.push(r.range(-3_000_000_000i64 as i32, i32::MAX) as f64 + 0.25);
    }
    for v in cases {
        assert_eq!(
            unsafe { cd(v) },
            unsafe { rd(v) },
            "safe_double_to_int({v:?} / bits {:#x})",
            v.to_bits()
        );
    }
}

/// `compute_size_metric` on strings of every length that can occur (and more).
#[test]
fn s2_compute_size_metric() {
    let ctx = ctx!();
    let (cm, rm) = ctx.pair.sym::<MetricFn>(b"jumpnode_test_compute_size_metric\0");
    for len in 0..300usize {
        let mut s: Vec<i8> = vec![b'x' as i8; len];
        s.push(0);
        assert_eq!(
            unsafe { cm(s.as_ptr()) },
            unsafe { rm(s.as_ptr()) },
            "compute_size_metric(len={len})"
        );
    }
    // embedded high bytes / non-ASCII must not change strlen semantics
    for pat in [0xFFu8, 0x80, 0x01, b'\n'] {
        for len in [0usize, 1, 7, 33, 49] {
            let mut s: Vec<i8> = vec![pat as i8; len];
            s.push(0);
            assert_eq!(unsafe { cm(s.as_ptr()) }, unsafe { rm(s.as_ptr()) });
        }
    }
}

/// `process_backward` — the pointer arithmetic, including *negative*
/// `start_offset`, made well-defined by handing both sides a pointer into the
/// middle of a large, fully-initialised buffer.
#[test]
fn s2_process_backward() {
    let ctx = ctx!();
    let (cp, rp) = ctx
        .pair
        .sym::<ProcBackFn>(b"jumpnode_test_process_backward\0");

    let mut r = Rng::new(SEED ^ 0x13);
    for _ in 0..3000 {
        // 64-int buffer; both sides get &buf[32] so offsets -32..=32 are valid.
        let mut buf: Vec<i32> = (0..64).map(|_| r.range(-1_000_000, 1_000_000)).collect();
        let mid = 32usize;
        let size = r.range(0, 32) as usize;
        let start_offset = r.range(-32, 32);
        let base = unsafe { buf.as_mut_ptr().add(mid) };
        let got_c = unsafe { cp(base, size, start_offset) };
        let got_rs = unsafe { rp(base, size, start_offset) };
        assert_eq!(
            got_c, got_rs,
            "process_backward(size={size}, start_offset={start_offset})"
        );
    }

    // Deterministic edge cases: empty range, single element, full range,
    // start beyond end, start exactly at end.
    let mut buf: Vec<i32> = (0..64).map(|i| i as i32 * 3 - 40).collect();
    let base = unsafe { buf.as_mut_ptr().add(32) };
    for size in [0usize, 1, 2, 15, 16, 17, 31, 32] {
        for start_offset in [-32i32, -5, -1, 0, 1, 15, 16, 17, 31, 32] {
            assert_eq!(
                unsafe { cp(base, size, start_offset) },
                unsafe { rp(base, size, start_offset) },
                "process_backward edge(size={size}, off={start_offset})"
            );
        }
    }

    // Summation that overflows `int` must wrap the same way on both sides.
    let mut big: Vec<i32> = vec![i32::MAX / 2; 64];
    big[40] = i32::MIN;
    let bbase = unsafe { big.as_mut_ptr().add(32) };
    for size in [0usize, 1, 4, 16, 32] {
        for off in [0i32, 1, 8] {
            assert_eq!(
                unsafe { cp(bbase, size, off) },
                unsafe { rp(bbase, size, off) },
                "process_backward wrap(size={size}, off={off})"
            );
        }
    }
}

/// `add_node` — normal inserts, and the `node_count >= MAX_NODES` rejection.
#[test]
fn s2_add_node_and_capacity() {
    let ctx = ctx!();
    let (ca, ra) = ctx.pair.sym::<AddNodeFn>(b"jumpnode_test_add_node\0");
    let (csc, rsc) = ctx.pair.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");
    let (cg, rg) = ctx.pair.sym::<GetCountFn>(b"jumpnode_test_get_node_count\0");
    let (cn, rn) = ctx.pair.sym::<GetNodeFn>(b"jumpnode_test_get_node\0");

    unsafe {
        csc(0);
        rsc(0);
    }
    let mut r = Rng::new(SEED ^ 0x14);
    // Fill well past MAX_NODES (100) so the capacity branch is hit repeatedly.
    for k in 0..130 {
        let id = r.interesting_i32();
        let parent = r.range(-3, 130);
        let value = (r.range(-1_000_000, 1_000_000) as f64) / 8.0;
        let sc = unsafe { ca(id, parent, value) };
        let sr = unsafe { ra(id, parent, value) };
        assert_eq!(sc, sr, "add_node #{k} status");
        assert_eq!(unsafe { cg() }, unsafe { rg() }, "node_count after add #{k}");
        if k >= 100 {
            assert_eq!(sc, 2, "add_node past MAX_NODES must return STATUS_ERROR");
            assert_eq!(unsafe { cg() }, 100, "node_count must stay at MAX_NODES");
        } else {
            assert_eq!(sc, 0, "add_node #{k} must return STATUS_OK");
        }
    }
    // Whole table must be byte-identical.
    for i in 0..100 {
        let mut a = (0i32, 0i32, 0f64, [0i32; 4]);
        let mut b = (0i32, 0i32, 0f64, [0i32; 4]);
        unsafe {
            cn(i, &mut a.0, &mut a.1, &mut a.2, a.3.as_mut_ptr());
            rn(i, &mut b.0, &mut b.1, &mut b.2, b.3.as_mut_ptr());
        }
        assert_eq!(a.0, b.0, "node[{i}].id");
        assert_eq!(a.1, b.1, "node[{i}].parent_id");
        assert_eq!(a.2.to_bits(), b.2.to_bits(), "node[{i}].value");
        assert_eq!(a.3, b.3, "node[{i}].data");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 11-25
// ---------------------------------------------------------------------------

/// Row 11: mode 1 from the root node (`parent_id == -1` stops the walk at once).
#[test]
fn cfg_row11_mode1_root() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 11);
    for depth in -5..=40 {
        diff("row11", &c, &rs, 1, 1, depth, r.interesting_i32());
    }
    for &depth in &[i32::MAX, i32::MIN, 1_000_000, -1_000_000] {
        diff("row11-extreme", &c, &rs, 1, 1, depth, r.interesting_i32());
    }
}

/// Row 12: mode 1 from a one-hop node.
#[test]
fn cfg_row12_mode1_one_hop() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 12);
    for id in [2, 3] {
        for depth in -5..=40 {
            diff("row12", &c, &rs, 1, id, depth, r.interesting_i32());
        }
        for &depth in &[i32::MAX, i32::MIN] {
            diff("row12-extreme", &c, &rs, 1, id, depth, 0);
        }
    }
}

/// Row 13: mode 1 along the deepest chain 7 -> 4 -> 2 -> 1.
#[test]
fn cfg_row13_mode1_deep_chain() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 13);
    for depth in -5..=40 {
        diff("row13", &c, &rs, 1, 7, depth, r.interesting_i32());
    }
    for &depth in &[i32::MAX, i32::MIN] {
        diff("row13-extreme", &c, &rs, 1, 7, depth, 0);
    }
}

/// Row 14: mode 1, every live id x every depth in -2..=12 x random flags
/// (flags must be ignored by this arm).
#[test]
fn cfg_row14_mode1_all_ids() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 14);
    for &id in LIVE_IDS {
        for depth in -2..=12 {
            for _ in 0..8 {
                diff("row14", &c, &rs, 1, id, depth, r.interesting_i32());
            }
        }
    }
}

/// Row 15: mode 1 with a non-existent id while storage is non-empty.
#[test]
fn cfg_row15_mode1_missing_id() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 15);
    for &id in DEAD_IDS {
        for depth in [-1, 0, 1, 5, 100] {
            diff("row15", &c, &rs, 1, id, depth, r.interesting_i32());
        }
    }
    for _ in 0..3000 {
        let id = r.interesting_i32();
        diff("row15-fuzz", &c, &rs, 1, id, r.range(-3, 30), r.interesting_i32());
    }
}

/// Row 16: mode 2, existing id, `depth` in 0..=16 (in-bounds `process_backward`).
#[test]
fn cfg_row16_mode2_inbounds() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 16);
    for &id in LIVE_IDS {
        for depth in 0..=16 {
            for _ in 0..20 {
                // keep 16*flags inside `int` so the C stays defined
                let flags = r.range(-100_000_000, 100_000_000);
                diff("row16", &c, &rs, 2, id, depth, flags);
            }
            diff("row16-f0", &c, &rs, 2, id, depth, 0);
        }
    }
}

/// Row 17: mode 2 with `depth > 16` — `process_backward` returns 0, so the
/// result is purely `16 * flags`.
#[test]
fn cfg_row17_mode2_depth_past_end() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 17);
    for &id in LIVE_IDS {
        for depth in [17i32, 18, 19, 20, 21, 50, 1000, 1_000_000] {
            let flags = r.range(-100_000_000, 100_000_000);
            diff("row17", &c, &rs, 2, id, depth, flags);
            diff("row17-f0", &c, &rs, 2, id, depth, 0);
        }
    }
}

/// Row 18: mode 2 where `16 * flags` wraps `int`. Signed overflow is UB in C but
/// both toolchains emit a wrapping multiply; this pins the behaviour down.
#[test]
fn cfg_row18_mode2_flag_wrap() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 18);
    for &flags in &[
        i32::MAX,
        i32::MIN,
        i32::MAX / 2,
        i32::MIN / 2,
        1 << 28,
        -(1 << 28),
        0x1234_5678,
        -0x1234_5678,
    ] {
        for depth in [0i32, 1, 8, 16, 17] {
            diff("row18", &c, &rs, 2, 1, depth, flags);
        }
    }
    for _ in 0..3000 {
        diff("row18-fuzz", &c, &rs, 2, 1, r.range(0, 20), r.next_i32());
    }
}

/// Row 19: mode 4 depth sweep, incl. the sign flip at depth == -10 and the
/// `node_count > 2` backward walk over the last three nodes.
#[test]
fn cfg_row19_mode4_depth_sweep() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 19);
    for &id in LIVE_IDS {
        for depth in -20..=40 {
            diff("row19", &c, &rs, 4, id, depth, r.interesting_i32());
        }
    }
}

/// Row 20: mode 4 with extreme depth, driving `safe_double_to_int` into both
/// clamps.
#[test]
fn cfg_row20_mode4_clamps() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    for &depth in &[
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1_000_000_000,
        -1_000_000_000,
        2_000_000_000,
        -2_000_000_000,
        100_000_000,
        -100_000_000,
        16_000_000,
        -16_000_000,
        // near the exact clamp thresholds: result is ~133.6597 * (1 + d/10)
        160_000_000,
        160_700_000,
        -160_700_000,
    ] {
        for &id in LIVE_IDS {
            diff("row20", &c, &rs, 4, id, depth, 0);
        }
    }
}

/// Row 21: mode 3 and the `default:` arm must be unaffected by library state.
#[test]
fn cfg_row21_mode3_and_default_with_state() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 21);
    for _ in 0..10000 {
        diff(
            "row21-m3",
            &c,
            &rs,
            3,
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
    }
    for mode in [-1i32, 0, 5, 6, 99, i32::MAX, i32::MIN] {
        for _ in 0..200 {
            diff(
                "row21-def",
                &c,
                &rs,
                mode,
                r.interesting_i32(),
                r.interesting_i32(),
                r.interesting_i32(),
            );
        }
    }
}

/// Row 22: repeated `initialize_test_data` resets `node_count` rather than
/// appending; interleaved calls must keep matching.
#[test]
fn cfg_row22_repeated_init() {
    let ctx = ctx!();
    let (cg, rg) = ctx.pair.sym::<GetCountFn>(b"jumpnode_test_get_node_count\0");
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 22);
    for round in 0..12 {
        init_both(&ctx);
        assert_eq!(unsafe { cg() }, unsafe { rg() }, "count round {round}");
        assert_eq!(unsafe { cg() }, 7, "init must reset node_count to 7");
        for _ in 0..300 {
            let mode = r.range(1, 4);
            let depth = if mode == 2 { r.range(0, 16) } else { r.range(-20, 40) };
            diff(
                "row22",
                &c,
                &rs,
                mode,
                *pick(LIVE_IDS, &mut r),
                depth,
                r.range(-1_000_000, 1_000_000),
            );
        }
    }
}

/// Row 23: whole-S2-surface fuzz (depth kept mode-2-safe).
#[test]
fn cfg_row23_s2_fuzz() {
    let ctx = ctx!();
    init_both(&ctx);
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 23);
    for _ in 0..40000 {
        let mode = r.range(-3, 8);
        let id = r.range(-3, 12);
        // mode 2 with depth < 0 reads out of bounds in the C (UB), so clamp it
        let depth = if mode == 2 { r.range(0, 40) } else { r.range(-40, 40) };
        let flags = r.range(-100_000_000, 100_000_000);
        diff("row23", &c, &rs, mode, id, depth, flags);
    }
}

/// Row 24: mode 4 with `node_count` 0/1/2 (backward-walk block disabled) vs >2.
#[test]
fn cfg_row24_mode4_small_count() {
    let ctx = ctx!();
    let (csc, rsc) = ctx.pair.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");
    let (c, rs) = ctx.jump();
    let mut r = Rng::new(SEED ^ 24);
    for count in 0..=7 {
        init_both(&ctx);
        unsafe {
            csc(count);
            rsc(count);
        }
        for id in -1..=8 {
            for depth in [-11i32, -10, -9, -1, 0, 1, 3, 10] {
                diff("row24", &c, &rs, 4, id, depth, r.interesting_i32());
                // mode 1 also depends on which ids are still visible
                diff("row24-m1", &c, &rs, 1, id, depth, 0);
                diff("row24-m2", &c, &rs, 2, id, depth.max(0), r.range(-1000, 1000));
            }
        }
    }
}

/// Row 25: a full 100-node table, then mode 4's backward walk over its tail.
#[test]
fn cfg_row25_max_nodes() {
    let ctx = ctx!();
    let (ca, ra) = ctx.pair.sym::<AddNodeFn>(b"jumpnode_test_add_node\0");
    let (csc, rsc) = ctx.pair.sym::<SetCountFn>(b"jumpnode_test_set_node_count\0");
    let (cg, rg) = ctx.pair.sym::<GetCountFn>(b"jumpnode_test_get_node_count\0");
    let (c, rs) = ctx.jump();

    unsafe {
        csc(0);
        rsc(0);
    }
    let mut r = Rng::new(SEED ^ 25);
    for i in 0..105 {
        let id = i + 1;
        let parent = if i == 0 { -1 } else { i };
        let value = (r.range(-2_000_000, 2_000_000) as f64) / 16.0;
        assert_eq!(unsafe { ca(id, parent, value) }, unsafe {
            ra(id, parent, value)
        });
    }
    assert_eq!(unsafe { cg() }, unsafe { rg() });
    assert_eq!(unsafe { cg() }, 100);

    // deep mode-1 chains (100 links), mode 4 tail walk, mode 2
    for id in [1i32, 2, 50, 99, 100, 101, 0, -1] {
        for depth in [0i32, 1, 50, 99, 100, 101, 1000] {
            diff("row25-m1", &c, &rs, 1, id, depth, 0);
            diff("row25-m4", &c, &rs, 4, id, depth, 0);
        }
        for depth in 0..=16 {
            diff("row25-m2", &c, &rs, 2, id, depth, r.range(-1000, 1000));
        }
    }
    for _ in 0..5000 {
        let mode = r.range(1, 4);
        let depth = if mode == 2 { r.range(0, 30) } else { r.range(-30, 130) };
        diff(
            "row25-fuzz",
            &c,
            &rs,
            mode,
            r.range(-3, 110),
            depth,
            r.range(-1_000_000, 1_000_000),
        );
    }
}

fn pick<'a>(xs: &'a [i32], r: &mut Rng) -> &'a i32 {
    &xs[(r.next_u64() % xs.len() as u64) as usize]
}
