// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH shared objects through
// their exported C symbols and compares the return value, the byte-exact
// stdout, and the resulting `ProcessState` contents.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/// Number of randomised iterations for property-style rows.
const N: usize = 400;

// ===========================================================================
// helpers
// ===========================================================================

/// Runs `body` against both implementations and asserts identical
/// `(value, stdout, snapshot)`.
fn diff_state<R: PartialEq + std::fmt::Debug>(
    ctx: &str,
    body: impl Fn(&Impl) -> (R, *mut ProcessState),
) {
    let (c, r) = impls();

    let (cv, cout) = capture(|| body(c));
    let csnap = unsafe { snapshot(cv.1) };
    unsafe { (c.destroy_state)(cv.1) };

    let (rv, rout) = capture(|| body(r));
    let rsnap = unsafe { snapshot(rv.1) };
    unsafe { (r.destroy_state)(rv.1) };

    assert_same(ctx, (cv.0, cout), (rv.0, rout));
    assert_eq!(csnap, rsnap, "[{ctx}] ProcessState differs");
}

/// Fresh state from each implementation, guaranteed non-null.
struct Pair {
    c_state: *mut ProcessState,
    r_state: *mut ProcessState,
}

impl Pair {
    fn new(initial_val: c_int, capacity: c_int) -> Pair {
        let (c, r) = impls();
        let (c_state, _) = capture(|| unsafe { (c.create_state)(initial_val, capacity) });
        let (r_state, _) = capture(|| unsafe { (r.create_state)(initial_val, capacity) });
        assert!(!c_state.is_null() && !r_state.is_null(), "create_state failed");
        Pair { c_state, r_state }
    }

    fn assert_states_equal(&self, ctx: &str) {
        let cs = unsafe { snapshot(self.c_state) };
        let rs = unsafe { snapshot(self.r_state) };
        assert_eq!(cs, rs, "[{ctx}] ProcessState differs");
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        let (c, r) = impls();
        let _ = capture(|| unsafe { (c.destroy_state)(self.c_state) });
        let _ = capture(|| unsafe { (r.destroy_state)(self.r_state) });
    }
}

// ===========================================================================
// Row 1 — create_state, capacity 128, random initial_val
// ===========================================================================

#[test]
fn cfg_01_create_state_random_initial_val() {
    let mut rng = Rng::new();
    for i in 0..N {
        let v = rng.next_i32();
        diff_state(&format!("row1 i={i} initial_val={v}"), |im| {
            (0u8, unsafe { (im.create_state)(v, 128) })
        });
    }
}

// ===========================================================================
// Row 2 — create_state boundary initial_val renderings
// ===========================================================================

#[test]
fn cfg_02_create_state_boundary_initial_val() {
    for &v in &[
        0i32,
        1,
        -1,
        9,
        10,
        -10,
        99999,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        1078530011,
        -1078530011,
    ] {
        diff_state(&format!("row2 initial_val={v}"), |im| {
            (0u8, unsafe { (im.create_state)(v, 128) })
        });
    }
}

// ===========================================================================
// Row 3 — capacity sweep 0..=40 (malloc(0), every snprintf truncation point)
// ===========================================================================

#[test]
fn cfg_03_create_state_capacity_sweep() {
    let mut rng = Rng::with_seed(3);
    for cap in 0i32..=40 {
        for &v in &[0i32, 7, -7, i32::MIN, i32::MAX, rng.next_i32()] {
            diff_state(&format!("row3 cap={cap} initial_val={v}"), |im| {
                (0u8, unsafe { (im.create_state)(v, cap) })
            });
        }
    }
}

// ===========================================================================
// Row 4 — large capacities
// ===========================================================================

#[test]
fn cfg_04_create_state_large_capacity() {
    for &cap in &[4096i32, 65536, 1 << 20] {
        for &v in &[0i32, i32::MIN, i32::MAX] {
            diff_state(&format!("row4 cap={cap} initial_val={v}"), |im| {
                (0u8, unsafe { (im.create_state)(v, cap) })
            });
        }
    }
}

// ===========================================================================
// Row 5 — the initial PackedFlags storage word must be bit-identical
// ===========================================================================

#[test]
fn cfg_05_create_state_initial_flag_bits() {
    let p = Pair::new(42, 128);
    let c_bits = unsafe { (*p.c_state).flags };
    let r_bits = unsafe { (*p.r_state).flags };
    assert_eq!(
        c_bits, r_bits,
        "initial PackedFlags word differs: C=0x{c_bits:08X} Rust=0x{r_bits:08X}"
    );
    // flag1=1 flag2=0 flag3=1 counter=0 mode=3 status=15 reserved=0, i.e.
    //   1 | (1 << 2) | (3 << 8) | (15 << 11) == 0x0000_7B05
    // (independently confirmed with a gcc `offsetof`/bit-field probe against the
    // typedefs in c_src/src/lib.c).
    assert_eq!(
        c_bits, 0x0000_7B05,
        "C's initial PackedFlags storage word changed"
    );
    assert_eq!(c_bits & FLAG1_MASK, 1);
    assert_eq!(c_bits & FLAG2_MASK, 0);
    assert_eq!(c_bits & FLAG3_MASK, FLAG3_MASK);
    assert_eq!(counter_of(c_bits), 0);
    assert_eq!(mode_of(c_bits), 3);
    assert_eq!((c_bits & STATUS_MASK) >> 11, 15);
    assert_eq!(c_bits & RESERVED_MASK, 0);
    p.assert_states_equal("row5");
}

// ===========================================================================
// Row 6 — process_buffer over the default buffer, every target byte 0x00..=0xFF
// ===========================================================================

#[test]
fn cfg_06_process_buffer_all_targets() {
    let (c, r) = impls();
    for &initial in &[0i32, 3333, -12345, i32::MIN, i32::MAX] {
        let p = Pair::new(initial, 128);
        for t in 0u16..=255 {
            let t = t as u8 as c_char;
            let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, t) });
            let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, t) });
            assert_same(&format!("row6 initial={initial} target={}", t as u8), cv, rv);
        }
        p.assert_states_equal("row6");
    }
}

// ===========================================================================
// Rows 7 / 8 / 9 — hand-written buffers: match at first / last index,
// consecutive matches
// ===========================================================================

fn diff_process_buffer(ctx: &str, capacity: c_int, content: &[u8], target: u8) {
    let (c, r) = impls();
    let p = Pair::new(0, capacity);
    unsafe {
        set_buffer(p.c_state, content);
        set_buffer(p.r_state, content);
    }
    let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, target as c_char) });
    let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, target as c_char) });
    assert_same(ctx, cv, rv);
    p.assert_states_equal(ctx);
}

#[test]
fn cfg_07_process_buffer_match_first() {
    diff_process_buffer("row7 a-at-0", 64, b"abcdefgh", b'a');
    diff_process_buffer("row7 single-byte", 64, b"a", b'a');
    diff_process_buffer("row7 a-only-at-0", 64, b"axxxxxxx", b'a');
}

#[test]
fn cfg_08_process_buffer_match_last() {
    diff_process_buffer("row8 z-at-end", 64, b"abcdefgz", b'z');
    diff_process_buffer("row8 x-at-end-long", 64, b"0000000000000000000x", b'x');
}

#[test]
fn cfg_09_process_buffer_consecutive() {
    for n in 0usize..=32 {
        let content = vec![b'a'; n];
        diff_process_buffer(&format!("row9 aaa n={n}"), 64, &content, b'a');
    }
    diff_process_buffer("row9 mixed", 64, b"aabbaabbaa", b'a');
    diff_process_buffer("row9 mixed-b", 64, b"aabbaabbaa", b'b');
    // Every byte matches, buffer completely full.
    let content = vec![b'q'; 63];
    diff_process_buffer("row9 full", 64, &content, b'q');
}

// ===========================================================================
// Row 10 — random buffers × random targets
// ===========================================================================

#[test]
fn cfg_10_process_buffer_random() {
    let mut rng = Rng::with_seed(10);
    for i in 0..N {
        let len = rng.below(121) as usize;
        // Non-NUL bytes so `strlen` == len.
        let content: Vec<u8> = (0..len)
            .map(|_| {
                let b = (rng.below(255) + 1) as u8;
                b
            })
            .collect();
        // Bias the target towards bytes actually present.
        let target = if len > 0 && rng.below(2) == 0 {
            content[rng.below(len as u64) as usize]
        } else {
            rng.below(256) as u8
        };
        diff_process_buffer(&format!("row10 i={i} len={len} target={target}"), 128, &content, target);
    }
}

// ===========================================================================
// Row 11 — embedded NUL: strlen() < capacity, matches on both sides
// ===========================================================================

#[test]
fn cfg_11_process_buffer_embedded_nul() {
    let (c, r) = impls();
    for &target in &[b'a', b'b', b'\0', b'z'] {
        let p = Pair::new(0, 64);
        // "aab\0aab" + trailing filler: only the first "aab" is inside strlen().
        let raw: &[u8] = b"aab\0aabaab\0\0";
        unsafe {
            set_buffer_raw(p.c_state, raw);
            set_buffer_raw(p.r_state, raw);
        }
        let ctx = format!("row11 target={target}");
        let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, target as c_char) });
        let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, target as c_char) });
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);
    }
}

// ===========================================================================
// Row 12 — high-bit (negative char) buffer bytes and targets
// ===========================================================================

#[test]
fn cfg_12_process_buffer_high_bit_bytes() {
    let content: Vec<u8> = vec![0x80, 0xFF, 0x7F, 0x81, 0xFF, 0xC3, 0xA9, 0x80];
    for t in [0x80u8, 0xFF, 0x7F, 0x81, 0xC3, 0xA9, 0x01] {
        diff_process_buffer(&format!("row12 target=0x{t:02X}"), 64, &content, t);
    }
    let mut rng = Rng::with_seed(12);
    for i in 0..N {
        let len = rng.below(40) as usize;
        let content: Vec<u8> = (0..len).map(|_| 0x80u8 | (rng.below(128) as u8)).collect();
        let t = 0x80u8 | (rng.below(128) as u8);
        diff_process_buffer(&format!("row12 rnd i={i}"), 64, &content, t);
    }
}

// ===========================================================================
// Row 13 — update_flags, param 0..=63 (all flag × mode combinations)
// ===========================================================================

#[test]
fn cfg_13_update_flags_low_six_bits() {
    let (c, r) = impls();
    for param in 0i32..=63 {
        let p = Pair::new(0, 64);
        let ctx = format!("row13 param={param}");
        let cv = capture(|| unsafe { (c.update_flags)(p.c_state, param) });
        let rv = capture(|| unsafe { (r.update_flags)(p.r_state, param) });
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);
    }
}

// ===========================================================================
// Row 14 — counter wrap-around over 40 successive calls
// ===========================================================================

#[test]
fn cfg_14_update_flags_counter_wrap() {
    let (c, r) = impls();
    let p = Pair::new(11, 64);
    for i in 0..40i32 {
        let param = i * 7;
        let ctx = format!("row14 i={i} param={param}");
        let cv = capture(|| unsafe { (c.update_flags)(p.c_state, param) });
        let rv = capture(|| unsafe { (r.update_flags)(p.r_state, param) });
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);
        // Sanity: counter must be (i+1) mod 32.
        assert_eq!(
            counter_of(unsafe { (*p.c_state).flags }),
            ((i + 1) % 32) as u32,
            "{ctx}"
        );
    }
}

// ===========================================================================
// Row 15 — update_flags with random full-range params (arithmetic >> 3)
// ===========================================================================

#[test]
fn cfg_15_update_flags_random_param() {
    let (c, r) = impls();
    let mut rng = Rng::with_seed(15);
    for i in 0..N {
        let p = Pair::new(rng.next_i32(), 64);
        let param = rng.next_i32();
        let ctx = format!("row15 i={i} param={param}");
        let cv = capture(|| unsafe { (c.update_flags)(p.c_state, param) });
        let rv = capture(|| unsafe { (r.update_flags)(p.r_state, param) });
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);
    }
    // Explicit extremes.
    for &param in &[i32::MIN, i32::MAX, -1, -2, -4, -8, -9, i32::MIN + 7] {
        let p = Pair::new(0, 64);
        let ctx = format!("row15 extreme param={param}");
        let cv = capture(|| unsafe { (c.update_flags)(p.c_state, param) });
        let rv = capture(|| unsafe { (r.update_flags)(p.r_state, param) });
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);
    }
}

// ===========================================================================
// Row 16 — status / reserved must survive update_flags untouched
// ===========================================================================

#[test]
fn cfg_16_update_flags_preserves_status_reserved() {
    let (c, r) = impls();
    let mut rng = Rng::with_seed(16);
    for i in 0..64 {
        let p = Pair::new(0, 64);
        // Poke a non-trivial status/reserved pattern into both states so the
        // read-modify-write behaviour of the bit-field stores is observable.
        let poke = rng.next_u32();
        unsafe {
            (*p.c_state).flags = poke;
            (*p.r_state).flags = poke;
        }
        let param = rng.next_i32();
        let ctx = format!("row16 i={i} poke=0x{poke:08X} param={param}");
        let cv = capture(|| unsafe { (c.update_flags)(p.c_state, param) });
        let rv = capture(|| unsafe { (r.update_flags)(p.r_state, param) });
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);
        let after = unsafe { (*p.c_state).flags };
        assert_eq!(after & STATUS_MASK, poke & STATUS_MASK, "{ctx}: status changed");
        assert_eq!(
            after & RESERVED_MASK,
            poke & RESERVED_MASK,
            "{ctx}: reserved changed"
        );
    }
}

// ===========================================================================
// Rows 17..21 — confuse_types, one row per operation
// ===========================================================================

fn diff_confuse_types(ctx: &str, payload: u32, operation: c_int) {
    let (c, r) = impls();
    let p = Pair::new(0, 64);
    unsafe {
        (*p.c_state).data = payload;
        (*p.r_state).data = payload;
    }
    let cv = capture(|| unsafe { (c.confuse_types)(p.c_state, operation) });
    let rv = capture(|| unsafe { (r.confuse_types)(p.r_state, operation) });
    assert_same(ctx, cv, rv);
    p.assert_states_equal(ctx);
}

#[test]
fn cfg_17_confuse_types_op0() {
    let mut rng = Rng::with_seed(17);
    for i in 0..N {
        let payload = rng.next_u32();
        diff_confuse_types(&format!("row17 i={i} payload=0x{payload:08X}"), payload, 0);
    }
    for &payload in &[0u32, 0xFFFF_FFFF, 1078530011, 0x8000_0000] {
        diff_confuse_types(&format!("row17 fixed 0x{payload:08X}"), payload, 0);
    }
}

#[test]
fn cfg_18_confuse_types_op1_random() {
    let mut rng = Rng::with_seed(18);
    for i in 0..(N * 4) {
        let payload = rng.next_u32();
        diff_confuse_types(&format!("row18 i={i} payload=0x{payload:08X}"), payload, 1);
    }
}

/// Curated float bit patterns: zeros, denormals, normals, infinities, NaNs and
/// the exact `(int)(x * 100.0f)` overflow boundary.
fn float_boundaries() -> Vec<u32> {
    let mut v: Vec<u32> = vec![
        0x0000_0000, // +0.0
        0x8000_0000, // -0.0
        0x0000_0001, // smallest denormal
        0x0080_0000, // FLT_MIN
        0x007F_FFFF, // largest denormal
        0x7F80_0000, // +inf
        0xFF80_0000, // -inf
        0x7FC0_0000, // quiet NaN
        0xFFC0_0000, // -quiet NaN
        0x7F80_0001, // signalling NaN
        0xFF80_0001, // -signalling NaN
        0x7F7F_FFFF, // FLT_MAX
        0xFF7F_FFFF, // -FLT_MAX
        0x3F80_0000, // 1.0
        0xBF80_0000, // -1.0
        0x4049_0FDB, // pi == 1078530011
        0x3F00_0000, // 0.5
        0xBF00_0000, // -0.5
        0x4F00_0000, // 2^31 exactly
        0xCF00_0000, // -2^31 exactly
        0x4EFF_FFFF, // just under 2^31
        0xCF00_0001, // just under -2^31
    ];
    // Values whose *100 lands right at the int32 boundary.
    for f in [
        21474836.0f32,
        21474838.0,
        21474840.0,
        -21474836.0,
        -21474840.0,
        20000000.0,
        -20000000.0,
        2147483.5,
        1.0e30,
        -1.0e30,
        1.0e-30,
        -1.0e-30,
        3.4e38,
        0.005,
        -0.005,
        2.55,
        127.999,
    ] {
        v.push(f.to_bits());
    }
    v
}

#[test]
fn cfg_19_confuse_types_op1_boundaries() {
    for payload in float_boundaries() {
        diff_confuse_types(
            &format!("row19 payload=0x{payload:08X} ({})", f32::from_bits(payload)),
            payload,
            1,
        );
    }
    // Also sweep the whole exponent range with a few mantissas.
    for exp in 0u32..=255 {
        for &mant in &[0u32, 1, 0x40_0000, 0x7F_FFFF] {
            for sign in [0u32, 1] {
                let payload = (sign << 31) | (exp << 23) | mant;
                diff_confuse_types(&format!("row19 sweep 0x{payload:08X}"), payload, 1);
            }
        }
    }
}

#[test]
fn cfg_20_confuse_types_op2() {
    let mut rng = Rng::with_seed(20);
    for i in 0..N {
        let payload = rng.next_u32();
        diff_confuse_types(&format!("row20 i={i} payload=0x{payload:08X}"), payload, 2);
    }
    for &payload in &[
        0u32,
        0xFF,
        0x100,
        0x1FF,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFF,
        0xFFFF_FF00,
    ] {
        diff_confuse_types(&format!("row20 fixed 0x{payload:08X}"), payload, 2);
    }
}

#[test]
fn cfg_21_confuse_types_op3() {
    let mut rng = Rng::with_seed(21);
    for i in 0..N {
        let payload = rng.next_u32();
        diff_confuse_types(&format!("row21 i={i} payload=0x{payload:08X}"), payload, 3);
    }
    for &payload in &[
        0u32,
        0x8080_8080,
        0xFFFF_FFFF,
        0x7F7F_7F7F,
        0x8000_0000,
        0x0000_0080,
        0x0000_8080,
        0x80FF_0001,
    ] {
        diff_confuse_types(&format!("row21 fixed 0x{payload:08X}"), payload, 3);
    }
}

// ===========================================================================
// Row 22 — sequenced operations on one state (op 0's write observed by later
// operations)
// ===========================================================================

#[test]
fn cfg_22_confuse_types_sequenced() {
    let (c, r) = impls();
    let payloads = [
        0u32,
        0xFFFF_FFFF,
        0x7FC0_0000,
        0x4F00_0000,
        0x0080_0000,
        0x8080_8080,
        1078530011,
        0x3F80_0000,
    ];
    for &payload in &payloads {
        let p = Pair::new(0, 64);
        unsafe {
            (*p.c_state).data = payload;
            (*p.r_state).data = payload;
        }
        for op in [3i32, 2, 1, 0, 1, 2, 3, 0, 3, 2, 1] {
            let ctx = format!("row22 payload=0x{payload:08X} op={op}");
            let cv = capture(|| unsafe { (c.confuse_types)(p.c_state, op) });
            let rv = capture(|| unsafe { (r.confuse_types)(p.r_state, op) });
            assert_same(&ctx, cv, rv);
            p.assert_states_equal(&ctx);
        }
    }
}

// ===========================================================================
// Rows 23..29 — the composed `confusion` pipeline
// ===========================================================================

fn diff_confusion(ctx: &str, a: c_int, b: c_int, cc: c_int, d: c_int) {
    let (c, r) = impls();
    let cv = capture(|| unsafe { (c.confusion)(a, b, cc, d) });
    let rv = capture(|| unsafe { (r.confusion)(a, b, cc, d) });
    assert_same(ctx, cv, rv);
}

#[test]
fn cfg_23_confusion_random() {
    let mut rng = Rng::with_seed(23);
    for i in 0..(N * 4) {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        diff_confusion(&format!("row23 i={i} ({a},{b},{c},{d})"), a, b, c, d);
    }
}

#[test]
fn cfg_24_confusion_each_operation() {
    let mut rng = Rng::with_seed(24);
    for op in 0i32..=3 {
        for i in 0..64 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let c = rng.next_i32();
            // param4 % 4 == op  (op >= 0 so pick a non-negative param4)
            let d = (rng.next_u32() >> 3) as i32 / 4 * 4 + op;
            assert_eq!(d % 4, op);
            diff_confusion(&format!("row24 op={op} i={i} ({a},{b},{c},{d})"), a, b, c, d);
        }
    }
}

#[test]
fn cfg_25_confusion_each_search_digit() {
    let mut rng = Rng::with_seed(25);
    for digit in 0i32..=9 {
        for i in 0..32 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let d = rng.next_i32();
            let c = (rng.next_u32() >> 4) as i32 / 10 * 10 + digit;
            assert_eq!(c % 10, digit);
            diff_confusion(
                &format!("row25 digit={digit} i={i} ({a},{b},{c},{d})"),
                a,
                b,
                c,
                d,
            );
        }
    }
    // The digits that actually occur in "State:%d:Mode:%d" for a fixed value.
    for digit in 0i32..=9 {
        diff_confusion(&format!("row25 fixed digit={digit}"), 1234567890, 0, digit, 0);
    }
    // Values with many repetitions of the search digit, so the memchr loop
    // iterates many times (and LOG_OPERATION prints an increasing count).
    for (val, digit) in [
        (3333333i32, 3),
        (1111111, 1),
        (999999999, 9),
        (-3333333, 3),
        (2000000000, 0),
        (-2147483648, 4),
        (1000000000, 0),
    ] {
        diff_confusion(&format!("row25 repeat val={val} digit={digit}"), val, 0, digit, 0);
        // ... and with a non-zero mode so the trailing ":Mode:%d" digit varies.
        for b in 0i32..8 {
            diff_confusion(
                &format!("row25 repeat val={val} digit={digit} mode={b}"),
                val,
                b << 3,
                digit,
                0,
            );
        }
    }
}

#[test]
fn cfg_26_confusion_negative_param3() {
    let mut rng = Rng::with_seed(26);
    for digit in -9i32..=0 {
        for i in 0..32 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let d = rng.next_i32();
            let c = -(((rng.next_u32() >> 4) as i32 / 10 * 10) - digit);
            diff_confusion(
                &format!("row26 digit={digit} i={i} ({a},{b},{c},{d})"),
                a,
                b,
                c,
                d,
            );
        }
    }
    for c in -20i32..=0 {
        diff_confusion(&format!("row26 exact c={c}"), 505, 5, c, 0);
    }
}

#[test]
fn cfg_27_confusion_param2_sweep() {
    for b in 0i32..=63 {
        diff_confusion(&format!("row27 b={b}"), 12345, b, 3, 0);
        diff_confusion(&format!("row27 b={b} neg"), -12345, b, 7, 3);
    }
    for b in [-1i32, -2, -4, -8, -16, -64, i32::MIN, i32::MAX] {
        diff_confusion(&format!("row27 extreme b={b}"), 999, b, 9, 1);
    }
}

#[test]
fn cfg_28_confusion_int_min_wrap() {
    // param1 doubles as the union payload; pick patterns whose float reading is
    // NaN / Inf / far out of int range so `(int)(x * 100)` is the
    // "integer indefinite" INT_MIN, then check the final `result +=` wrap.
    let mut cases: Vec<i32> = float_boundaries().iter().map(|&b| b as i32).collect();
    cases.extend_from_slice(&[i32::MIN, i32::MAX, -1, 0, 1]);
    for a in cases {
        // param4 % 4 == 1 -> the float branch.
        diff_confusion(&format!("row28 a={a} op1"), a, 0, 0, 1);
        diff_confusion(&format!("row28 a={a} op1 flags"), a, 63, 5, 5);
        diff_confusion(&format!("row28 a={a} op3"), a, 21, 7, 3);
        diff_confusion(&format!("row28 a={a} op2"), a, 42, 1, 2);
    }
}

#[test]
fn cfg_29_confusion_extremes_cross() {
    let vals = [i32::MIN, i32::MAX, 0, -1];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    diff_confusion(&format!("row29 ({a},{b},{c},{d})"), a, b, c, d);
                }
            }
        }
    }
}

// ===========================================================================
// Row 30 — the same composition `confusion` performs, but hand-driven through
// the low-level exports with randomised capacity / call counts.
// ===========================================================================

#[test]
fn cfg_30_low_level_pipeline_random() {
    let (c, r) = impls();
    let mut rng = Rng::with_seed(30);
    for i in 0..N {
        let initial = rng.next_i32();
        // Capacities from "too small for the whole string" to comfortable.
        let capacity = (rng.below(60) + 1) as c_int;
        let p = Pair::new(initial, capacity);
        let ctx0 = format!("row30 i={i} initial={initial} cap={capacity}");
        p.assert_states_equal(&format!("{ctx0} after create"));

        let k = rng.below(6) as usize;
        for j in 0..k {
            let param = rng.next_i32();
            let ctx = format!("{ctx0} update[{j}] param={param}");
            let cv = capture(|| unsafe { (c.update_flags)(p.c_state, param) });
            let rv = capture(|| unsafe { (r.update_flags)(p.r_state, param) });
            assert_same(&ctx, cv, rv);
            p.assert_states_equal(&ctx);
        }

        let target = rng.below(256) as u8 as c_char;
        let ctx = format!("{ctx0} process target={}", target as u8);
        let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, target) });
        let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, target) });
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);

        // Include out-of-range operations in the sweep.
        let op = (rng.below(9) as i32) - 3;
        let ctx = format!("{ctx0} confuse op={op}");
        let cv = capture(|| unsafe { (c.confuse_types)(p.c_state, op) });
        let rv = capture(|| unsafe { (r.confuse_types)(p.r_state, op) });
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);

        // Re-read the buffer after everything, and process it again.
        let ctx = format!("{ctx0} process again");
        let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, b'0' as c_char) });
        let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, b'0' as c_char) });
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);
    }
}
