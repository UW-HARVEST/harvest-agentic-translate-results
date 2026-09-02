// Phase B -- valid-path differential tests, gated on CONFIGS.md.
//
// Every entry point is driven through the `.so` exports of BOTH implementations
// and the return value *and* the bytes written to stdout are compared.
// Row ids in the test names refer to CONFIGS.md.

#[path = "support/mod.rs"]
mod support;

use std::ffi::c_int;
use support::*;

/// Full boundary cross-product plus `n` deterministic pseudo-random pairs.
fn pairs(random: usize, seed: u64) -> Vec<(c_int, c_int)> {
    let mut v = Vec::with_capacity(BOUNDARY.len() * BOUNDARY.len() + random);
    for &a in BOUNDARY.iter() {
        for &b in BOUNDARY.iter() {
            v.push((a, b));
        }
    }
    let mut rng = Rng::new(seed);
    for _ in 0..random {
        v.push((rng.next_i32(), rng.next_i32()));
    }
    v
}

/// Compare only the return value (used for the leaf ops, which never print).
fn diff_ret(sym: &str, a: c_int, b: c_int) {
    let p = pair();
    let cr = unsafe { (p.c.op(sym))(a, b) };
    let rr = unsafe { (p.rust.op(sym))(a, b) };
    assert_eq!(cr, rr, "{sym}({a}, {b}): C={cr} Rust={rr} [OP={OP} REPEAT={REPEAT}]");
}

/* ================= Group L: leaf arithmetic (op_add/op_sub/op_mul) =========
 * `int op_add(int a,int b){ return a + b; }` and friends contain no macro at
 * all, so they are identical in every OP x REPEAT configuration. Signed
 * overflow is exercised deliberately: gcc -O2 emits plain add/sub/imul, and
 * the Rust side uses wrapping_* to match.
 * ======================================================================== */

#[test]
fn l1_op_add() {
    for (a, b) in pairs(4096, 0x0A11_ADD5) {
        diff_ret("op_add", a, b);
    }
}

#[test]
fn l2_op_sub() {
    for (a, b) in pairs(4096, 0x0B22_5AB0) {
        diff_ret("op_sub", a, b);
    }
}

#[test]
fn l3_op_mul() {
    for (a, b) in pairs(4096, 0x0C33_3AA1) {
        diff_ret("op_mul", a, b);
    }
}

/// The three leaf ops must write nothing at all to stdout, in both libraries.
#[test]
fn l4_leaf_ops_are_silent() {
    for sym in ["op_add", "op_sub", "op_mul"] {
        for (a, b) in [(7, 3), (0, 0), (-5, 9), (i32::MAX, 2), (i32::MIN, -1)] {
            diff_op(sym, a, b);
            let p = pair();
            let cf = p.c.op(sym);
            let (_, out) = with_stdout_capture(|| unsafe { cf(a, b) });
            assert!(out.is_empty(), "{sym} unexpectedly printed {out:?}");
        }
    }
}

/* ================= Group G: exported data objects ======================== */

/// G1..G3 -- `int (*G_OP)(int,int) = OP_FN(OP);` must point at `op_<OP>` and
/// calling through the slot must agree with calling `op_<OP>` directly.
#[test]
fn g_call_through_g_op() {
    let p = pair();
    let direct = format!("op_{OP}");
    for (a, b) in pairs(2048, 0x6_0F_0001) {
        diff_g_op(a, b);
        // op_* never print, so no capture is needed here.
        let via_slot = unsafe { (p.c.g_op())(a, b) };
        let via_name = unsafe { (p.c.op(&direct))(a, b) };
        assert_eq!(via_slot, via_name, "C: G_OP != op_{OP} for ({a},{b})");
        let r_slot = unsafe { (p.rust.g_op())(a, b) };
        let r_name = unsafe { (p.rust.op(&direct))(a, b) };
        assert_eq!(r_slot, r_name, "Rust: G_OP != op_{OP} for ({a},{b})");
    }
}

/// G4..G6 -- `const char *G_OP_NAME = STR(OP);` stringifies the OP token.
#[test]
fn g_op_name_bytes_match() {
    let p = pair();
    let c = p.c.g_op_name();
    let r = p.rust.g_op_name();
    assert_eq!(c, r, "G_OP_NAME bytes differ: C={c:?} Rust={r:?}");
    assert_eq!(String::from_utf8_lossy(&c), OP);
    // NUL terminated, and the terminator is the only one.
    let c_full = unsafe { std::slice::from_raw_parts(*p.c.g_op_name_slot() as *const u8, c.len() + 1) };
    let r_full =
        unsafe { std::slice::from_raw_parts(*p.rust.g_op_name_slot() as *const u8, r.len() + 1) };
    assert_eq!(c_full, r_full, "G_OP_NAME storage differs including NUL");
}

/// G7..G9 -- the C globals are *mutable* objects. A consumer may store into
/// `G_OP`; the library's own helpers use `OP_FN(OP)` directly (never `G_OP`),
/// so their results must be unchanged afterwards. This also proves the Rust
/// object is in writable `.data` outside `PT_GNU_RELRO`, like the C one.
#[test]
fn g_op_slot_is_writable_and_helpers_ignore_it() {
    let p = pair();
    let c_slot = p.c.g_op_slot();
    let r_slot = p.rust.g_op_slot();
    let c_orig = unsafe { *c_slot };
    let r_orig = unsafe { *r_slot };

    // Pick a different op than the configured one so a wrong read is visible.
    let other = if OP == "mul" { "op_add" } else { "op_mul" };
    let c_other = p.c.op(other);
    let r_other = p.rust.op(other);

    unsafe {
        *c_slot = c_other;
        *r_slot = r_other;
    }
    for (a, b) in [(7, 3), (-5, 9), (0, 0), (i32::MAX, 3), (i32::MIN, -1)] {
        let (cr, _) = with_stdout_capture(|| unsafe { (*c_slot)(a, b) });
        let (rr, _) = with_stdout_capture(|| unsafe { (*r_slot)(a, b) });
        assert_eq!(cr, rr, "after storing into G_OP: {other}({a},{b})");
        // helper_call / helper_ptr expand OP_FN(OP), not G_OP.
        diff_op("helper_call", a, b);
        diff_op("helper_ptr", a, b);
    }
    unsafe {
        *c_slot = c_orig;
        *r_slot = r_orig;
    }
    diff_g_op(7, 3);
}

/* ================= Group P: helper_ptr =================================== */

/// `helper_ptr` takes the OP through a local function pointer and prints
/// `helper.ptr=%d`. Depends on OP only.
#[test]
fn p_helper_ptr() {
    for (a, b) in pairs(512, 0x9_7E_0002) {
        diff_op("helper_ptr", a, b);
    }
}

/* ================= Group H: helper_call (OP x REPEAT) ==================== */

/// `helper_call` is the composed path: `OP_FN(OP)(a,b)`, then
/// `RUN_LOOP(OP, acc, REPEAT)` from `INIT_FOR(OP)`, then
/// `printf("helper.call=%d helper.acc=%d\n", r, acc)`, returning `r + acc`.
/// This is the only entry point whose result depends on REPEAT.
#[test]
fn h_helper_call() {
    for (a, b) in pairs(512, 0x4_C0_0003) {
        diff_op("helper_call", a, b);
    }
}

/// The accumulator half of `helper_call` must be constant for a fixed
/// configuration (it does not depend on `a`/`b`), and both libraries must
/// report the same constant. Derived from the printed `helper.acc=` field.
#[test]
fn h_helper_call_accumulator_is_stable() {
    let p = pair();
    let cf = p.c.op("helper_call");
    let rf = p.rust.op("helper_call");
    let mut c_seen: Option<String> = None;
    for (a, b) in pairs(64, 0x4_C0_0004) {
        let (_, cout) = with_stdout_capture(|| unsafe { cf(a, b) });
        let (_, rout) = with_stdout_capture(|| unsafe { rf(a, b) });
        let ctext = String::from_utf8_lossy(&cout).into_owned();
        assert_eq!(ctext, String::from_utf8_lossy(&rout));
        let acc = ctext
            .rsplit("helper.acc=")
            .next()
            .unwrap()
            .trim()
            .to_string();
        match &c_seen {
            None => c_seen = Some(acc),
            Some(prev) => assert_eq!(prev, &acc, "helper.acc varied with (a,b)=({a},{b})"),
        }
    }
    assert!(c_seen.is_some());
}

/* ================= Group U: use_generated / DISPATCH_REP ================= */

/// U-<op>-0 .. U-<op>-6 -- every `case` of the `switch (n)` in `DISPATCH_REP`.
#[test]
fn u_use_generated_switch_cases() {
    for &n in DISPATCH_IN_RANGE.iter() {
        diff_unary("use_generated", n);
    }
}

/// The `n` the driver actually passes is `REPEAT`; for REPEAT == 7 that lands
/// on `default:` (there is no `case 7:`), which the translation must preserve.
#[test]
fn u_use_generated_at_repeat() {
    diff_unary("use_generated", REPEAT);
}

/// Sweep the whole small-int neighbourhood plus randomized values.
#[test]
fn u_use_generated_randomized() {
    for n in -16..=16 {
        diff_unary("use_generated", n);
    }
    let mut rng = Rng::new(0x0_5E_0005);
    for _ in 0..256 {
        diff_unary("use_generated", rng.next_i32());
    }
}

/* ================= Composed pipeline through the .so ==================== */

/// Drive the whole library surface in one interleaved sequence, the way the
/// driver does, so that any hidden state (there should be none) or ordering
/// dependency between the entry points would show up.
#[test]
fn composed_pipeline_sequence() {
    let p = pair();
    let mut rng = Rng::new(0x0_C0_0006);
    let syms = ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"];
    for _ in 0..256 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        for sym in syms {
            diff_op(sym, a, b);
        }
        diff_unary("use_generated", a);
        diff_unary("use_generated", REPEAT);
        diff_g_op(a, b);
        assert_eq!(p.c.g_op_name(), p.rust.g_op_name());
    }
}

/// Reproduce `main`'s arithmetic through the exported symbols only, and check
/// the two libraries agree on the full accumulated summary. This exercises the
/// exact composition `mdmain.c` performs, but at the library boundary.
#[test]
fn composed_summary_matches() {
    let p = pair();
    let direct = format!("op_{OP}");
    for (a, b) in pairs(256, 0x0_5F_0007) {
        let run = |imp: &Impl| -> c_int {
            let (v, _) = with_stdout_capture(|| unsafe {
                let r_call = (imp.op(&direct))(a, b);
                let x1 = (imp.op("helper_call"))(a, b);
                let x2 = (imp.op("helper_ptr"))(a, b);
                let x3 = (imp.unary("use_generated"))(REPEAT);
                let g = (imp.g_op())(a, b);
                r_call
                    .wrapping_add(x1)
                    .wrapping_add(x2)
                    .wrapping_add(x3)
                    .wrapping_add(g)
            });
            v
        };
        let c_total = run(&p.c);
        let r_total = run(&p.rust);
        assert_eq!(c_total, r_total, "composed summary for ({a},{b})");
    }
}
