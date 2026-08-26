//! Phase B — valid-path differential tests.
//!
//! One test (or one loop iteration inside a test) per row of `CONFIGS.md`.
//! Everything is driven through `dlopen`ed C symbols on BOTH `.so` files; the
//! lowest-level entry points (`create_buffer` / `append_to_buffer` /
//! `destroy_buffer` / `get_operation_name` / `perform_operation`) are exercised
//! directly, not only through the `buffapp` convenience wrapper.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ===========================================================================
// helpers
// ===========================================================================

/// Overwrite `data[0..capacity]` with a deterministic non-zero pattern so that
/// externally-forced `length` values (row group C21..C23, mirroring what
/// `buffapp` itself does at lib.c:116) compare deterministically instead of
/// reading uninitialised `malloc` bytes.
unsafe fn fill(p: *mut StringBuffer, salt: u8) {
    let b = unsafe { *p };
    for i in 0..b.capacity as isize {
        unsafe { *b.data.offset(i) = (((i as u8) % 251).wrapping_add(salt) | 1) as c_char };
    }
}

unsafe fn set_length(p: *mut StringBuffer, len: c_int) {
    unsafe { (*p).length = len };
}

/// A byte string that is guaranteed NUL-free and `n` bytes long.
fn ascii(rng: &mut Rng, n: usize) -> Vec<u8> {
    rng.bytes(n, 0x21, 0x7e)
}

// ===========================================================================
// C1 .. C7 — create_buffer over the whole capacity shape space
// ===========================================================================

fn create_destroy_rows(label: &str, caps: Vec<c_int>) {
    diff(label, |imp, t| {
        for &cap in &caps {
            let p = unsafe { (imp.create_buffer)(cap) };
            t.buf(p);
            if !p.is_null() {
                // data[0] must have been set to '\0' (even for cap == 0, which
                // is the 1-byte OOB store the C performs).
                t.content(p);
            }
            unsafe { (imp.destroy_buffer)(p) };
        }
    });
}

fn c1_create_buffer_zero_capacity() {
    create_destroy_rows("C1 cap=0", vec![0]);
}

fn c2_create_buffer_capacity_one() {
    create_destroy_rows("C2 cap=1", vec![1]);
}

fn c3_create_buffer_capacity_two() {
    create_destroy_rows("C3 cap=2", vec![2]);
}

fn c4_create_buffer_capacity_31_32_33() {
    create_destroy_rows("C4 cap=31,32,33", vec![31, 32, 33]);
}

fn c5_create_buffer_randomized_capacities() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    let caps: Vec<c_int> = (0..200).map(|_| rng.range_i32(1, 4096)).collect();
    create_destroy_rows("C5 randomized caps 1..=4096", caps);
}

fn c6_create_buffer_large_capacities() {
    create_destroy_rows("C6 cap=65536,1<<20", vec![65536, 1 << 20]);
}

fn c7_create_buffer_many_live_buffers() {
    diff("C7 64 live buffers", |imp, t| {
        let mut rng = Rng::new(SEED ^ 0xC7);
        let mut ps = Vec::new();
        for i in 0..64 {
            let cap = rng.range_i32(0, 512);
            let p = unsafe { (imp.create_buffer)(cap) };
            t.buf(p);
            // Write something through each so allocator reuse is exercised.
            let s = cstring(&ascii(&mut rng, (i % 7) as usize));
            let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
            t.buf(p);
            t.content(p);
            ps.push(p);
        }
        // Destroy in a shuffled-ish order.
        for k in 0..ps.len() {
            let idx = (k * 37) % ps.len();
            if !ps[idx].is_null() {
                unsafe { (imp.destroy_buffer)(ps[idx]) };
                ps[idx] = std::ptr::null_mut();
            }
        }
        for p in ps {
            if !p.is_null() {
                unsafe { (imp.destroy_buffer)(p) };
            }
        }
    });
}

// ===========================================================================
// C8 / C9 / C10 — cross-library ownership (A6)
// ===========================================================================

/// Run create/append/destroy with each step taken from a possibly different
/// implementation. All four assignments must produce identical traces.
fn ownership_trace(cre: &Impl, app: &Impl, des: &Impl) -> Trace {
    let mut t = Trace::new();
    let mut rng = Rng::new(SEED ^ 0xC8);
    for _ in 0..32 {
        let p = unsafe { (cre.create_buffer)(8) };
        t.buf(p);
        for _ in 0..6 {
            let n = rng.below(9) as usize;
            let s = cstring(&ascii(&mut rng, n));
            let r = unsafe { (app.append_to_buffer)(p, s.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
            t.buf(p);
            t.content(p);
        }
        unsafe { (des.destroy_buffer)(p) };
    }
    t
}

fn c8_c9_c10_cross_library_ownership() {
    let all_c = ownership_trace(c(), c(), c());
    let all_rs = ownership_trace(rs(), rs(), rs());
    assert_traces("C10 all-C vs all-Rust", &all_c, &all_rs);

    let c_create_rs_rest = ownership_trace(c(), rs(), rs());
    assert_traces("C8 C-create/Rust-append+destroy", &all_c, &c_create_rs_rest);

    let rs_create_c_rest = ownership_trace(rs(), c(), c());
    assert_traces("C9 Rust-create/C-append+destroy", &all_c, &rs_create_c_rest);

    // and the two remaining mixes, for good measure
    assert_traces("C8b C-create/Rust-append/C-destroy", &all_c, &ownership_trace(c(), rs(), c()));
    assert_traces("C9b Rust-create/C-append/Rust-destroy", &all_c, &ownership_trace(rs(), c(), rs()));
}

// ===========================================================================
// C11 .. C17 — append_to_buffer boundary shapes (A2 / A4)
// ===========================================================================

fn append_once(label: &str, cap: c_int, str_len: usize) {
    let mut seed_rng = Rng::new(SEED ^ (cap as u64) ^ ((str_len as u64) << 20));
    let payload = ascii(&mut seed_rng, str_len);
    diff(label, |imp, t| {
        let p = unsafe { (imp.create_buffer)(cap) };
        t.buf(p);
        let s = cstring(&payload);
        let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
        t.push(Obs::Ret(r));
        t.buf(p);
        t.content(p);
        unsafe { (imp.destroy_buffer)(p) };
    });
}

fn c11_append_empty_string_no_realloc() {
    append_once("C11 cap=16 len=0", 16, 0);
}

fn c12_append_one_char() {
    append_once("C12 cap=16 len=1", 16, 1);
}

fn c13_append_required_equals_capacity() {
    // required = 0 + 15 + 1 == 16 == capacity  -> NOT > capacity, no realloc
    append_once("C13 cap=16 len=15 (required==capacity)", 16, 15);
}

fn c14_append_required_one_over_capacity() {
    // required = 17 > 16 -> realloc to 34
    append_once("C14 cap=16 len=16 (required==capacity+1)", 16, 16);
}

fn c15_append_capacity_one_empty() {
    append_once("C15 cap=1 len=0", 1, 0);
}

fn c16_append_capacity_zero_empty() {
    // required = 1 > 0 -> realloc(malloc(0) ptr, 2)
    append_once("C16 cap=0 len=0", 0, 0);
}

fn c17_append_huge_single_grow() {
    append_once("C17 cap=4 len=4096", 4, 4096);
}

fn c11_c17_boundary_sweep() {
    // Full sweep of (capacity, strlen) around every boundary the `>` test can
    // straddle: this is the property-style version of C11..C17.
    for cap in 0..=40 {
        for len in 0..=42 {
            append_once(&format!("sweep cap={cap} len={len}"), cap, len);
        }
    }
}

// ===========================================================================
// C18 .. C20 — append chains (A5): realloc schedule must match exactly
// ===========================================================================

fn append_chain(label: &str, cap: c_int, count: usize, max_len: u32, seed: u64) {
    diff(label, |imp, t| {
        let mut rng = Rng::new(seed);
        let p = unsafe { (imp.create_buffer)(cap) };
        t.buf(p);
        for _ in 0..count {
            let n = rng.below(max_len + 1) as usize;
            let s = cstring(&ascii(&mut rng, n));
            let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
            t.buf(p);
            t.content(p);
        }
        unsafe { (imp.destroy_buffer)(p) };
    });
}

fn c18_append_chain_from_capacity_one() {
    append_chain("C18 cap=1 x64 appends 0..17", 1, 64, 17, SEED ^ 0x18);
}

fn c19_append_chain_never_reallocs() {
    append_chain("C19 cap=4096 x64 appends 0..17", 4096, 64, 17, SEED ^ 0x19);
}

fn c20_append_chain_mixed_schedule() {
    append_chain("C20 cap=32 x200 appends 0..40", 32, 200, 40, SEED ^ 0x20);
}

// ===========================================================================
// C21 .. C23 — externally forced `length` (A3), exactly as buffapp does
// ===========================================================================

fn c21_forced_length_random() {
    diff("C21 forced length in 0..=capacity", |imp, t| {
        let mut rng = Rng::new(SEED ^ 0x21);
        for _ in 0..300 {
            let cap = rng.range_i32(1, 96);
            let p = unsafe { (imp.create_buffer)(cap) };
            t.buf(p);
            unsafe { fill(p, 0x30) };
            let len = rng.range_i32(0, cap);
            unsafe { set_length(p, len) };
            let n = rng.below(48) as usize;
            let s = cstring(&ascii(&mut rng, n));
            let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
            t.buf(p);
            t.content(p);
            unsafe { (imp.destroy_buffer)(p) };
        }
    });
}

fn c22_forced_length_capacity_minus_one_empty_append() {
    diff("C22 forced length=cap-1, 0-byte append", |imp, t| {
        for cap in 1..=64 {
            let p = unsafe { (imp.create_buffer)(cap) };
            unsafe { fill(p, 0x41) };
            unsafe { set_length(p, cap - 1) };
            let s = cstring(b"");
            let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
            t.buf(p);
            t.content(p);
            unsafe { (imp.destroy_buffer)(p) };
        }
    });
}

fn c23_forced_length_capacity_empty_append() {
    diff("C23 forced length=cap, 0-byte append (reallocs)", |imp, t| {
        for cap in 0..=64 {
            let p = unsafe { (imp.create_buffer)(cap) };
            unsafe { fill(p, 0x61) };
            unsafe { set_length(p, cap) };
            let s = cstring(b"");
            let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
            t.buf(p);
            t.content(p);
            unsafe { (imp.destroy_buffer)(p) };
        }
    });
}

// ===========================================================================
// C24 — byte-exact payloads including high-bit bytes
// ===========================================================================

fn c24_append_high_bit_and_format_like_bytes() {
    diff("C24 non-ASCII / %-bearing payloads", |imp, t| {
        let mut rng = Rng::new(SEED ^ 0x24);
        let mut fixed: Vec<Vec<u8>> = vec![
            b"%d %s %n %%".to_vec(),
            b"\xff\xfe\x80\x81".to_vec(),
            b"\x01\x02\x03\x7f".to_vec(),
            (0x80u8..=0xffu8).collect(),
            (0x01u8..=0x7fu8).collect(),
        ];
        for _ in 0..64 {
            let n = rng.below(64) as usize;
            fixed.push(rng.bytes(n, 0x01, 0xff));
        }
        for payload in &fixed {
            let p = unsafe { (imp.create_buffer)(3) };
            t.buf(p);
            let s = cstring(payload);
            let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
            t.buf(p);
            t.content(p);
            unsafe { (imp.destroy_buffer)(p) };
        }
    });
}

// ===========================================================================
// C25 .. C29 — get_operation_name (A7)
// ===========================================================================

fn gon(label: &str, codes: Vec<c_int>) {
    diff(label, |imp, t| {
        for &code in &codes {
            let p = unsafe { (imp.get_operation_name)(code) };
            t.push(Obs::IsNull(p.is_null()));
            t.push(Obs::CStr(read_cstr(p)));
        }
    });
}

fn c25_get_operation_name_add() {
    gon("C25 op_code=0", vec![0]);
}

fn c26_get_operation_name_subtract() {
    gon("C26 op_code=1", vec![1]);
}

fn c27_get_operation_name_multiply() {
    gon("C27 op_code=2", vec![2]);
}

fn c28_get_operation_name_divide() {
    gon("C28 op_code=3", vec![3]);
}

fn c29_get_operation_name_randomized() {
    let mut rng = Rng::new(SEED ^ 0x29);
    let mut codes: Vec<c_int> = (-64..=64).collect();
    codes.extend([i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1]);
    codes.extend((0..4096).map(|_| rng.interesting_i32()));
    gon("C29 randomized op_codes", codes);
}

// ===========================================================================
// C30 .. C37 — perform_operation (A8 x A9 x A10)
// ===========================================================================

/// The operand grid every operation row uses: boundary values + randomized.
fn operand_pairs(seed: u64, n: usize) -> Vec<(c_int, c_int)> {
    const B: [i32; 10] = [
        0,
        1,
        -1,
        2,
        -2,
        7,
        -7,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
    ];
    let mut v = Vec::new();
    for &a in &B {
        for &b in &B {
            v.push((a, b));
        }
    }
    let mut rng = Rng::new(seed);
    for _ in 0..n {
        v.push((rng.interesting_i32(), rng.interesting_i32()));
    }
    v
}

fn perform_row(label: &str, op: &[u8], pairs: &[(c_int, c_int)], skip_int_min_div: bool) {
    let opz = cstring(op);
    let is_div = op == b"divide";
    diff(label, |imp, t| {
        for &(a, b) in pairs {
            // E14: `INT_MIN / -1` is a hardware trap in C (UB). Excluded.
            if skip_int_min_div && is_div && a == i32::MIN && b == -1 {
                continue;
            }
            let r = unsafe { (imp.perform_operation)(a, b, opz.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
        }
    });
}

fn c30_perform_operation_add() {
    perform_row("C30 add", b"add", &operand_pairs(SEED ^ 0x30, 2000), true);
}

fn c31_perform_operation_subtract() {
    perform_row(
        "C31 subtract",
        b"subtract",
        &operand_pairs(SEED ^ 0x31, 2000),
        true,
    );
}

fn c32_perform_operation_multiply() {
    perform_row(
        "C32 multiply",
        b"multiply",
        &operand_pairs(SEED ^ 0x32, 2000),
        true,
    );
}

fn c33_perform_operation_divide() {
    perform_row(
        "C33 divide",
        b"divide",
        &operand_pairs(SEED ^ 0x33, 2000),
        true,
    );
}

fn c34_perform_operation_divide_by_plus_minus_one() {
    let mut rng = Rng::new(SEED ^ 0x34);
    let mut pairs: Vec<(c_int, c_int)> = Vec::new();
    for _ in 0..1000 {
        let a = rng.interesting_i32();
        pairs.push((a, 1));
        if a != i32::MIN {
            pairs.push((a, -1));
        }
    }
    perform_row("C34 divide by +/-1", b"divide", &pairs, true);
}

fn c35_perform_operation_divide_truncation() {
    let pairs = vec![
        (7, 2),
        (-7, 2),
        (7, -2),
        (-7, -2),
        (1, 2),
        (-1, 2),
        (1, -2),
        (-1, -2),
        (i32::MAX, 2),
        (i32::MIN, 2),
        (i32::MIN, 3),
        (i32::MIN, -3),
        (5, 3),
        (-5, 3),
        (5, -3),
        (-5, -3),
    ];
    perform_row("C35 divide truncation-toward-zero", b"divide", &pairs, true);
}

fn c36_perform_operation_with_foreign_name_pointer() {
    // The `operation` pointer comes from the OTHER library's
    // get_operation_name — provenance must be irrelevant, only bytes matter.
    let pairs = operand_pairs(SEED ^ 0x36, 400);
    for code in 0..4 {
        let from_c = unsafe { (c().get_operation_name)(code) };
        let from_rs = unsafe { (rs().get_operation_name)(code) };
        assert_eq!(read_cstr(from_c), read_cstr(from_rs), "name for code {code}");
        for &(a, b) in &pairs {
            if code == 3 && a == i32::MIN && b == -1 {
                continue;
            }
            // C impl, name pointer from Rust .so
            let x = unsafe { (c().perform_operation)(a, b, from_rs) };
            // Rust impl, name pointer from C .so
            let y = unsafe { (rs().perform_operation)(a, b, from_c) };
            // and the straight pairings
            let z = unsafe { (c().perform_operation)(a, b, from_c) };
            let w = unsafe { (rs().perform_operation)(a, b, from_rs) };
            assert_eq!(
                (x, y, z),
                (w, w, w),
                "C36 code={code} a={a} b={b}: C(rsname)={x} RS(cname)={y} C(cname)={z} RS(rsname)={w}"
            );
        }
    }
}

fn c37_perform_operation_runtime_built_name() {
    // Same bytes, but the string is heap/stack-built rather than a literal.
    let pairs = operand_pairs(SEED ^ 0x37, 400);
    for op in OPS.iter() {
        let owned: Vec<u8> = {
            let mut v: Vec<u8> = Vec::with_capacity(op.len() + 1);
            for &b in op.iter() {
                v.push(b);
            }
            v.push(0);
            v
        };
        let label = format!("C37 runtime-built {}", String::from_utf8_lossy(op));
        let is_div = *op == &b"divide"[..];
        diff(&label, |imp, t| {
            for &(a, b) in &pairs {
                if is_div && a == i32::MIN && b == -1 {
                    continue;
                }
                let r = unsafe { (imp.perform_operation)(a, b, owned.as_ptr() as *const c_char) };
                t.push(Obs::Ret(r));
            }
        });
    }
}

// ===========================================================================
// C38 .. C86 — buffapp residue cross-product (A11 x A12 x A13)
// ===========================================================================

const RESIDUES: [i32; 7] = [0, 1, 2, 3, -1, -2, -3];

/// Produce a value `v` with `v % 4 == r` under C semantics (`%` truncates
/// toward zero, so a negative residue requires a negative `v`).
fn param_with_residue(rng: &mut Rng, r: i32, small: bool) -> i32 {
    let kmax = if small { 8 } else { i32::MAX / 4 - 2 };
    let k = rng.range_i32(0, kmax);
    match r {
        0 => {
            let v = k.wrapping_mul(4);
            if rng.below(2) == 0 { v } else { v.wrapping_neg() }
        }
        1..=3 => k.wrapping_mul(4).wrapping_add(r),
        _ => k.wrapping_mul(4).wrapping_add(-r).wrapping_neg(), // r in -3..=-1
    }
}

/// `param2` chosen so that `perform_operation(param1, param2, op(r1))` is 0,
/// which forces `intermediate3 == 0` and hence the A13 fallback branch.
fn zeroing_partner(p: i32, r: i32) -> i32 {
    match r {
        0 => p.wrapping_neg(),      // add:      p + (-p) == 0
        1 => p,                     // subtract: p - p == 0
        2 => 0,                     // multiply: p * 0 == 0
        3 => 0,                     // divide:   b == 0 -> returns 0
        _ => 0,                     // unknown:  always 0 anyway
    }
}

fn buffapp_cell(r1: i32, r3: i32) {
    let mut rng = Rng::new(SEED ^ ((r1 as u32 as u64) << 8) ^ ((r3 as u32 as u64) << 24));

    // 6 randomized samples (mixed magnitudes) ...
    for i in 0..6 {
        let small = i % 2 == 0;
        let p1 = param_with_residue(&mut rng, r1, small);
        let p3 = param_with_residue(&mut rng, r3, small);
        let p2 = if small {
            rng.range_i32(-16, 16)
        } else {
            rng.interesting_i32()
        };
        let p4 = if small {
            rng.range_i32(-16, 16)
        } else {
            rng.interesting_i32()
        };
        diff_buffapp(p1, p2, p3, p4);
    }

    // ... plus explicit A13 == 0 forcings (intermediate1 == 0, intermediate2 == 0)
    let p1 = param_with_residue(&mut rng, r1, true);
    let p3 = param_with_residue(&mut rng, r3, true);
    diff_buffapp(p1, zeroing_partner(p1, r1), p3, rng.range_i32(-16, 16));
    diff_buffapp(p1, rng.range_i32(-16, 16), p3, zeroing_partner(p3, r3));
    // and both zero at once
    diff_buffapp(p1, zeroing_partner(p1, r1), p3, zeroing_partner(p3, r3));
}

macro_rules! buffapp_cells {
    ($( $name:ident => ($r1:expr, $r3:expr) ),* $(,)?) => {
        $( fn $name() { buffapp_cell($r1, $r3); } )*
        fn register_cells(r: &mut Runner) { $( r.case(stringify!($name), $name); )* }
    };
}

buffapp_cells! {
    c38_buffapp_r1_0_r3_0    => (0, 0),
    c39_buffapp_r1_0_r3_1    => (0, 1),
    c40_buffapp_r1_0_r3_2    => (0, 2),
    c41_buffapp_r1_0_r3_3    => (0, 3),
    c42_buffapp_r1_0_r3_m1   => (0, -1),
    c43_buffapp_r1_0_r3_m2   => (0, -2),
    c44_buffapp_r1_0_r3_m3   => (0, -3),
    c45_buffapp_r1_1_r3_0    => (1, 0),
    c46_buffapp_r1_1_r3_1    => (1, 1),
    c47_buffapp_r1_1_r3_2    => (1, 2),
    c48_buffapp_r1_1_r3_3    => (1, 3),
    c49_buffapp_r1_1_r3_m1   => (1, -1),
    c50_buffapp_r1_1_r3_m2   => (1, -2),
    c51_buffapp_r1_1_r3_m3   => (1, -3),
    c52_buffapp_r1_2_r3_0    => (2, 0),
    c53_buffapp_r1_2_r3_1    => (2, 1),
    c54_buffapp_r1_2_r3_2    => (2, 2),
    c55_buffapp_r1_2_r3_3    => (2, 3),
    c56_buffapp_r1_2_r3_m1   => (2, -1),
    c57_buffapp_r1_2_r3_m2   => (2, -2),
    c58_buffapp_r1_2_r3_m3   => (2, -3),
    c59_buffapp_r1_3_r3_0    => (3, 0),
    c60_buffapp_r1_3_r3_1    => (3, 1),
    c61_buffapp_r1_3_r3_2    => (3, 2),
    c62_buffapp_r1_3_r3_3    => (3, 3),
    c63_buffapp_r1_3_r3_m1   => (3, -1),
    c64_buffapp_r1_3_r3_m2   => (3, -2),
    c65_buffapp_r1_3_r3_m3   => (3, -3),
    c66_buffapp_r1_m1_r3_0   => (-1, 0),
    c67_buffapp_r1_m1_r3_1   => (-1, 1),
    c68_buffapp_r1_m1_r3_2   => (-1, 2),
    c69_buffapp_r1_m1_r3_3   => (-1, 3),
    c70_buffapp_r1_m1_r3_m1  => (-1, -1),
    c71_buffapp_r1_m1_r3_m2  => (-1, -2),
    c72_buffapp_r1_m1_r3_m3  => (-1, -3),
    c73_buffapp_r1_m2_r3_0   => (-2, 0),
    c74_buffapp_r1_m2_r3_1   => (-2, 1),
    c75_buffapp_r1_m2_r3_2   => (-2, 2),
    c76_buffapp_r1_m2_r3_3   => (-2, 3),
    c77_buffapp_r1_m2_r3_m1  => (-2, -1),
    c78_buffapp_r1_m2_r3_m2  => (-2, -2),
    c79_buffapp_r1_m2_r3_m3  => (-2, -3),
    c80_buffapp_r1_m3_r3_0   => (-3, 0),
    c81_buffapp_r1_m3_r3_1   => (-3, 1),
    c82_buffapp_r1_m3_r3_2   => (-3, 2),
    c83_buffapp_r1_m3_r3_3   => (-3, 3),
    c84_buffapp_r1_m3_r3_m1  => (-3, -1),
    c85_buffapp_r1_m3_r3_m2  => (-3, -2),
    c86_buffapp_r1_m3_r3_m3  => (-3, -3),
}

/// Cheap belt-and-braces check that the residue helper really covers all
/// seven classes with C's truncating `%`.
fn residue_helper_is_correct() {
    let mut rng = Rng::new(1);
    for &r in RESIDUES.iter() {
        for small in [true, false] {
            for _ in 0..200 {
                let v = param_with_residue(&mut rng, r, small);
                assert_eq!(v % 4, r, "param_with_residue({r}, {small}) produced {v}");
            }
        }
    }
}

// ===========================================================================
// C87 .. C97 — buffapp special magnitudes and randomized sweeps
// ===========================================================================

fn c87_buffapp_all_zero() {
    diff_buffapp(0, 0, 0, 0);
}

fn c88_buffapp_all_one() {
    diff_buffapp(1, 1, 1, 1);
}

fn c89_buffapp_int_max_positions() {
    for pos in 0..4 {
        let mut p = [1, 2, 3, 5];
        p[pos] = i32::MAX;
        diff_buffapp(p[0], p[1], p[2], p[3]);
    }
}

fn c90_buffapp_int_min_positions() {
    for pos in 0..4 {
        let mut p = [1, 2, 3, 5];
        p[pos] = i32::MIN;
        diff_buffapp(p[0], p[1], p[2], p[3]);
    }
}

fn c91_buffapp_extreme_grid() {
    const V: [i32; 4] = [i32::MIN, i32::MAX, 0, -1];
    for &a in &V {
        for &b in &V {
            for &cc in &V {
                for &d in &V {
                    diff_buffapp(a, b, cc, d);
                }
            }
        }
    }
}

fn c92_buffapp_fallback_sum_overflows() {
    // op1 = add (p1 % 4 == 0) with p2 == -p1  => i1 == 0 => i3 == 0 => fallback
    // and the fallback sum itself overflows int.
    diff_buffapp(4, -4, i32::MAX, i32::MAX);
    diff_buffapp(0, 0, i32::MAX, i32::MAX);
    diff_buffapp(0, 0, i32::MIN, i32::MIN);
    diff_buffapp(i32::MAX - 3, 0, 0, 8); // (MAX-3) % 4 == 0 -> add
    diff_buffapp(-4, 4, i32::MIN, i32::MIN);
}

fn c93_buffapp_negative_quotient_truncation() {
    // result = i1 + i2 negative, intermediate3 = i1 * i2 positive -> negative
    // quotient, must truncate toward zero exactly like C.
    //   p1 % 4 == 0 -> add ; p3 % 4 == 0 -> add
    diff_buffapp(0, -7, 0, -3); // i1=-7, i2=-3, result=-10, i3=21 -> 0
    diff_buffapp(0, -21, 0, -1); // i1=-21, i2=-1, result=-22, i3=21 -> -1
    diff_buffapp(0, -30, 0, -1); // result=-31, i3=30 -> -1
    diff_buffapp(0, 30, 0, -1); // result=29, i3=-30 -> 0
    diff_buffapp(0, -30, 0, 1); // result=-29, i3=-30 -> 0
    diff_buffapp(0, 7, 0, -2); // result=5, i3=-14 -> 0
    diff_buffapp(0, -100, 0, -7); // result=-107, i3=700 -> 0
    diff_buffapp(0, -100, 0, 7); // result=-93, i3=-700 -> 0
}

fn c94_buffapp_intermediate3_plus_minus_one() {
    // i1 * i2 == 1  -> (1,1) or (-1,-1);  == -1 -> (1,-1) or (-1,1)
    diff_buffapp(0, 1, 0, 1);
    diff_buffapp(0, -1, 0, -1);
    diff_buffapp(0, 1, 0, -1);
    diff_buffapp(0, -1, 0, 1);
}

fn c95_buffapp_randomized_return_values() {
    let _d = discard_stdout();
    let mut rng = Rng::new(SEED ^ 0x95);
    for _ in 0..(20_000 * soak()) {
        diff_buffapp_ret(
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
}

fn c96_buffapp_randomized_with_stdout() {
    let mut rng = Rng::new(SEED ^ 0x96);
    for _ in 0..(2000 * soak()) {
        diff_buffapp(
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
}

fn c97_buffapp_small_domain_dense() {
    let mut rng = Rng::new(SEED ^ 0x97);
    for _ in 0..(1000 * soak()) {
        diff_buffapp(
            rng.range_i32(-8, 8),
            rng.range_i32(-8, 8),
            rng.range_i32(-8, 8),
            rng.range_i32(-8, 8),
        );
    }
}

fn zz_libraries_loaded() {
    assert_loaded();
}

// ===========================================================================
// Sequential runner entry point (harness = false)
// ===========================================================================

fn main() {
    let mut r = Runner::new();
    r.case("c1_create_buffer_zero_capacity", c1_create_buffer_zero_capacity);
    r.case("c2_create_buffer_capacity_one", c2_create_buffer_capacity_one);
    r.case("c3_create_buffer_capacity_two", c3_create_buffer_capacity_two);
    r.case("c4_create_buffer_capacity_31_32_33", c4_create_buffer_capacity_31_32_33);
    r.case("c5_create_buffer_randomized_capacities", c5_create_buffer_randomized_capacities);
    r.case("c6_create_buffer_large_capacities", c6_create_buffer_large_capacities);
    r.case("c7_create_buffer_many_live_buffers", c7_create_buffer_many_live_buffers);
    r.case("c8_c9_c10_cross_library_ownership", c8_c9_c10_cross_library_ownership);
    r.case("c11_append_empty_string_no_realloc", c11_append_empty_string_no_realloc);
    r.case("c12_append_one_char", c12_append_one_char);
    r.case("c13_append_required_equals_capacity", c13_append_required_equals_capacity);
    r.case("c14_append_required_one_over_capacity", c14_append_required_one_over_capacity);
    r.case("c15_append_capacity_one_empty", c15_append_capacity_one_empty);
    r.case("c16_append_capacity_zero_empty", c16_append_capacity_zero_empty);
    r.case("c17_append_huge_single_grow", c17_append_huge_single_grow);
    r.case("c11_c17_boundary_sweep", c11_c17_boundary_sweep);
    r.case("c18_append_chain_from_capacity_one", c18_append_chain_from_capacity_one);
    r.case("c19_append_chain_never_reallocs", c19_append_chain_never_reallocs);
    r.case("c20_append_chain_mixed_schedule", c20_append_chain_mixed_schedule);
    r.case("c21_forced_length_random", c21_forced_length_random);
    r.case("c22_forced_length_capacity_minus_one_empty_append", c22_forced_length_capacity_minus_one_empty_append);
    r.case("c23_forced_length_capacity_empty_append", c23_forced_length_capacity_empty_append);
    r.case("c24_append_high_bit_and_format_like_bytes", c24_append_high_bit_and_format_like_bytes);
    r.case("c25_get_operation_name_add", c25_get_operation_name_add);
    r.case("c26_get_operation_name_subtract", c26_get_operation_name_subtract);
    r.case("c27_get_operation_name_multiply", c27_get_operation_name_multiply);
    r.case("c28_get_operation_name_divide", c28_get_operation_name_divide);
    r.case("c29_get_operation_name_randomized", c29_get_operation_name_randomized);
    r.case("c30_perform_operation_add", c30_perform_operation_add);
    r.case("c31_perform_operation_subtract", c31_perform_operation_subtract);
    r.case("c32_perform_operation_multiply", c32_perform_operation_multiply);
    r.case("c33_perform_operation_divide", c33_perform_operation_divide);
    r.case("c34_perform_operation_divide_by_plus_minus_one", c34_perform_operation_divide_by_plus_minus_one);
    r.case("c35_perform_operation_divide_truncation", c35_perform_operation_divide_truncation);
    r.case("c36_perform_operation_with_foreign_name_pointer", c36_perform_operation_with_foreign_name_pointer);
    r.case("c37_perform_operation_runtime_built_name", c37_perform_operation_runtime_built_name);
    r.case("residue_helper_is_correct", residue_helper_is_correct);
    register_cells(&mut r); // C38..C86 buffapp residue cross-product
    r.case("c87_buffapp_all_zero", c87_buffapp_all_zero);
    r.case("c88_buffapp_all_one", c88_buffapp_all_one);
    r.case("c89_buffapp_int_max_positions", c89_buffapp_int_max_positions);
    r.case("c90_buffapp_int_min_positions", c90_buffapp_int_min_positions);
    r.case("c91_buffapp_extreme_grid", c91_buffapp_extreme_grid);
    r.case("c92_buffapp_fallback_sum_overflows", c92_buffapp_fallback_sum_overflows);
    r.case("c93_buffapp_negative_quotient_truncation", c93_buffapp_negative_quotient_truncation);
    r.case("c94_buffapp_intermediate3_plus_minus_one", c94_buffapp_intermediate3_plus_minus_one);
    r.case("c95_buffapp_randomized_return_values", c95_buffapp_randomized_return_values);
    r.case("c96_buffapp_randomized_with_stdout", c96_buffapp_randomized_with_stdout);
    r.case("c97_buffapp_small_domain_dense", c97_buffapp_small_domain_dense);
    r.case("zz_libraries_loaded", zz_libraries_loaded);
    r.finish();
}
