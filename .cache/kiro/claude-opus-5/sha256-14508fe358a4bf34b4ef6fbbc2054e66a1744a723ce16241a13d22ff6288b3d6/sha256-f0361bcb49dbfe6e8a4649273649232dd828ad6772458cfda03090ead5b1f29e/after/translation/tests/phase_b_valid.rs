//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `libloading` into either the C `.so` or the Rust
//! `.so`; nothing is called directly.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------- rows 1..=6
// The counter entry points. Both libraries own an independent `static int
// counter`, so the sequences are replayed step-for-step and every intermediate
// return value is compared.

#[test]
fn row01_reset_counter_randomized() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x0001);
    for i in 0..3000 {
        let v = rng.interesting_i32();
        let a = unsafe { (p.c.reset_counter)(v) };
        let b = unsafe { (p.rs.reset_counter)(v) };
        assert_eq!(a, b, "row1 step {i}: reset_counter({v})");
    }
}

#[test]
fn row02_increment_counter_randomized() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x0002);
    for i in 0..3000 {
        let base = rng.interesting_i32();
        assert_eq!(unsafe { (p.c.reset_counter)(base) }, unsafe {
            (p.rs.reset_counter)(base)
        });
        let v = rng.interesting_i32();
        let a = unsafe { (p.c.increment_counter)(v) };
        let b = unsafe { (p.rs.increment_counter)(v) };
        assert_eq!(a, b, "row2 step {i}: {base} += {v}");
    }
}

#[test]
fn row03_decrement_counter_randomized() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x0003);
    for i in 0..3000 {
        let base = rng.interesting_i32();
        unsafe { (p.c.reset_counter)(base) };
        unsafe { (p.rs.reset_counter)(base) };
        let v = rng.interesting_i32();
        let a = unsafe { (p.c.decrement_counter)(v) };
        let b = unsafe { (p.rs.decrement_counter)(v) };
        assert_eq!(a, b, "row3 step {i}: {base} -= {v}");
    }
}

#[test]
fn row04_multiply_counter_randomized() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x0004);
    // Explicit boundary products first.
    for &(base, v) in &[
        (0, 0),
        (1, -1),
        (-1, i32::MIN),
        (i32::MIN, -1),
        (i32::MAX, 2),
        (i32::MIN, i32::MIN),
        (65535, 65537),
        (-3, 0),
    ] {
        unsafe { (p.c.reset_counter)(base) };
        unsafe { (p.rs.reset_counter)(base) };
        let a = unsafe { (p.c.multiply_counter)(v) };
        let b = unsafe { (p.rs.multiply_counter)(v) };
        assert_eq!(a, b, "row4 boundary: {base} *= {v}");
    }
    for i in 0..3000 {
        let base = rng.interesting_i32();
        unsafe { (p.c.reset_counter)(base) };
        unsafe { (p.rs.reset_counter)(base) };
        let v = rng.interesting_i32();
        let a = unsafe { (p.c.multiply_counter)(v) };
        let b = unsafe { (p.rs.multiply_counter)(v) };
        assert_eq!(a, b, "row4 step {i}: {base} *= {v}");
    }
}

#[test]
fn row05_interleaved_counter_sequence() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x0005);
    // Start both from the same known state, then never reset again: divergence
    // in the hidden state shows up as a mismatch on a later step.
    unsafe { (p.c.reset_counter)(0) };
    unsafe { (p.rs.reset_counter)(0) };
    let cops = p.c.ops();
    let rops = p.rs.ops();
    for i in 0..4000 {
        let which = rng.below(4) as usize;
        let v = rng.interesting_i32();
        let a = unsafe { (cops[which].1)(v) };
        let b = unsafe { (rops[which].1)(v) };
        assert_eq!(a, b, "row5 step {i}: {}({v})", cops[which].0);
    }
}

#[test]
fn row06_counter_state_after_charinbuf_mode3() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x0006);
    let cops = p.c.ops();
    let rops = p.rs.ops();
    for i in 0..300 {
        let (v, o1, o2) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        // Drive mode 3 through both libraries so each leaves its own residue.
        let (rc, _) = capture(|| unsafe { (p.c.charinbuf)(3, v, o1, o2) });
        let (rr, _) = capture(|| unsafe { (p.rs.charinbuf)(3, v, o1, o2) });
        assert_eq!(rc, rr, "row6 step {i}: charinbuf(3,{v},{o1},{o2})");
        // Now read the residue back through the low-level entry points.
        for _ in 0..4 {
            let which = rng.below(4) as usize;
            let x = rng.interesting_i32();
            let a = unsafe { (cops[which].1)(x) };
            let b = unsafe { (rops[which].1)(x) };
            assert_eq!(
                a, b,
                "row6 step {i}: residue via {}({x}) after charinbuf(3,{v},{o1},{o2})",
                cops[which].0
            );
        }
    }
}

// ------------------------------------------------------------------- row 7

#[test]
fn row07_validate_uint16_range() {
    let _g = guard();
    let p = pair();
    let mut fixed: Vec<i32> = vec![i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1];
    fixed.extend(-2..=2);
    fixed.extend(65533..=65537);
    for v in fixed {
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.rs.validate_uint16_range)(v) };
        assert_eq!(a, b, "row7 boundary validate_uint16_range({v})");
    }
    let mut rng = Rng::new(0x0007);
    for _ in 0..20000 {
        let v = rng.interesting_i32();
        let a = unsafe { (p.c.validate_uint16_range)(v) };
        let b = unsafe { (p.rs.validate_uint16_range)(v) };
        assert_eq!(a, b, "row7 validate_uint16_range({v})");
    }
}

// -------------------------------------------------------------- rows 8..=11

fn diff_is_string_empty(bytes: &[u8], ctx: &str) {
    let p = pair();
    let s = cstring(bytes);
    let a = unsafe { (p.c.is_string_empty)(s.as_ptr()) };
    let b = unsafe { (p.rs.is_string_empty)(s.as_ptr()) };
    assert_eq!(a, b, "{ctx}: is_string_empty({bytes:?}) C={a} Rust={b}");
}

#[test]
fn row08_is_string_empty_ascii() {
    let _g = guard();
    for s in [
        &b"a"[..],
        b"Hello, World!",
        b"0",
        b" ",
        b"\t",
        b"\x01",
        b"\x7f",
    ] {
        diff_is_string_empty(s, "row8");
    }
}

#[test]
fn row09_is_string_empty_high_bytes() {
    let _g = guard();
    // First byte in 0x80..=0xFF: `char` is signed on x86-64, so `*str` is
    // negative here — still non-zero, so the C returns 0.
    for b in 0x80u8..=0xff {
        diff_is_string_empty(&[b], "row9");
        diff_is_string_empty(&[b, b'x'], "row9");
    }
}

#[test]
fn row10_is_string_empty_empty() {
    let _g = guard();
    diff_is_string_empty(b"", "row10");
}

#[test]
fn row11_is_string_empty_randomized() {
    let _g = guard();
    let mut rng = Rng::new(0x0011);
    for _ in 0..5000 {
        let len = rng.below(33) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            // Any non-NUL byte (a NUL would just truncate the C string).
            let mut b = rng.u8();
            if b == 0 {
                b = 1;
            }
            bytes.push(b);
        }
        diff_is_string_empty(&bytes, "row11");
    }
}

// ------------------------------------------------------------- rows 12..=14

fn diff_create_buffer(bytes: &[u8], ctx: &str) {
    let p = pair();
    let s = cstring(bytes);
    let a = unsafe { (p.c.create_buffer)(s.as_ptr()) };
    let b = unsafe { (p.rs.create_buffer)(s.as_ptr()) };
    assert!(!a.is_null(), "{ctx}: C create_buffer returned NULL");
    assert!(!b.is_null(), "{ctx}: Rust create_buffer returned NULL");
    let ca = unsafe { read_cstr(a) };
    let cb = unsafe { read_cstr(b) };
    assert_eq!(ca, bytes, "{ctx}: C copy differs from input");
    assert_eq!(cb, ca, "{ctx}: Rust copy differs from C copy");
    // The pointer must be releasable with libc free(), as the C contract says.
    unsafe { libc_free(a) };
    unsafe { libc_free(b) };
}

#[test]
fn row12_create_buffer_empty() {
    let _g = guard();
    diff_create_buffer(b"", "row12");
}

#[test]
fn row13_create_buffer_lengths() {
    let _g = guard();
    diff_create_buffer(b"x", "row13/len1");
    diff_create_buffer(b"Testing malloc and free", "row13/short");
    let long: Vec<u8> = (0..4096u32).map(|i| ((i % 94) as u8) + 33).collect();
    diff_create_buffer(&long, "row13/long4k");
}

#[test]
fn row14_create_buffer_randomized() {
    let _g = guard();
    let mut rng = Rng::new(0x0014);
    for _ in 0..2000 {
        let len = rng.below(513) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            let mut b = rng.u8();
            if b == 0 {
                b = 0xff;
            }
            bytes.push(b);
        }
        diff_create_buffer(&bytes, "row14");
    }
}

// ------------------------------------------------------------- rows 15..=22
// `find_char_in_buffer` gets the *same* caller-owned buffer from both
// libraries, so the returned pointers must be bit-identical, not merely
// equivalent offsets.

fn diff_find_char(buf: &[u8], size: usize, target: u8, ctx: &str) {
    let p = pair();
    let store = cstring(buf);
    let ptr = store.as_ptr();
    let a = unsafe { (p.c.find_char_in_buffer)(ptr, size, target as c_char) };
    let b = unsafe { (p.rs.find_char_in_buffer)(ptr, size, target as c_char) };
    let off = |q: *mut c_char| {
        if q.is_null() {
            -1i64
        } else {
            (q as usize as i64) - (ptr as usize as i64)
        }
    };
    assert_eq!(
        a,
        b,
        "{ctx}: find_char_in_buffer(len={}, size={size}, target={target:#04x}) \
         C offset={} Rust offset={}",
        buf.len(),
        off(a),
        off(b)
    );
}

#[test]
fn row15_find_char_match_at_zero() {
    let _g = guard();
    diff_find_char(b"Xabc", 4, b'X', "row15");
    diff_find_char(b"X", 1, b'X', "row15/len1");
}

#[test]
fn row16_find_char_match_middle() {
    let _g = guard();
    diff_find_char(b"Search for character X in this buffer", 37, b'X', "row16");
    diff_find_char(b"abcXdef", 7, b'X', "row16/short");
}

#[test]
fn row17_find_char_match_last_byte_in_size() {
    let _g = guard();
    diff_find_char(b"abcX", 4, b'X', "row17/exact");
    diff_find_char(b"abcXdef", 4, b'X', "row17/exact-truncated");
}

#[test]
fn row18_find_char_match_beyond_size() {
    let _g = guard();
    diff_find_char(b"abcX", 3, b'X', "row18");
    diff_find_char(b"abcdefX", 0, b'X', "row18/size0");
    diff_find_char(b"Xabc", 0, b'X', "row18/size0-at-front");
}

#[test]
fn row19_find_char_size_zero() {
    let _g = guard();
    diff_find_char(b"", 0, b'a', "row19/empty");
    diff_find_char(b"nonempty", 0, b'n', "row19/nonempty");
    diff_find_char(b"", 0, 0, "row19/nul");
}

#[test]
fn row20_find_char_target_nul() {
    let _g = guard();
    // `cstring` appends the terminator, so size == len excludes it and
    // size == len + 1 includes it.
    diff_find_char(b"abc", 3, 0, "row20/excluded");
    diff_find_char(b"abc", 4, 0, "row20/included");
    diff_find_char(b"", 1, 0, "row20/empty-included");
}

#[test]
fn row21_find_char_negative_target() {
    let _g = guard();
    for b in 0x80u8..=0xff {
        let buf = [b'a', b, b'z'];
        diff_find_char(&buf, 3, b, "row21/present");
        diff_find_char(&buf, 1, b, "row21/truncated");
        diff_find_char(b"abc", 3, b, "row21/absent");
    }
}

#[test]
fn row22_find_char_randomized() {
    let _g = guard();
    let mut rng = Rng::new(0x0022);
    for _ in 0..8000 {
        let len = rng.below(257) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            // Draw from a small alphabet often, so matches actually happen.
            let b = if rng.below(2) == 0 {
                b'a' + (rng.below(4) as u8)
            } else {
                rng.u8()
            };
            buf.push(b);
        }
        let target = if rng.below(3) == 0 {
            b'a' + (rng.below(4) as u8)
        } else {
            rng.u8()
        };
        // size may reach len + 1 so the appended terminator is in scope too.
        let size = rng.below(len as u64 + 2) as usize;
        diff_find_char(&buf, size, target, "row22");
    }
}

// ------------------------------------------------------------- rows 23..=24

#[test]
fn row23_apply_operation_with_library_ops() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x0023);
    let cops = p.c.ops();
    let rops = p.rs.ops();
    for i in 0..4000 {
        let base = rng.interesting_i32();
        unsafe { (p.c.reset_counter)(base) };
        unsafe { (p.rs.reset_counter)(base) };
        let which = rng.below(4) as usize;
        let v = rng.interesting_i32();
        let a = unsafe { (p.c.apply_operation)(Some(cops[which].1), v) };
        let b = unsafe { (p.rs.apply_operation)(Some(rops[which].1), v) };
        assert_eq!(
            a, b,
            "row23 step {i}: apply_operation({}, {v}) from base {base}",
            cops[which].0
        );
    }
}

unsafe extern "C" fn cb_identity(v: c_int) -> c_int {
    v
}
unsafe extern "C" fn cb_minus_one(_v: c_int) -> c_int {
    -1
}
unsafe extern "C" fn cb_negate(v: c_int) -> c_int {
    v.wrapping_neg()
}

#[test]
fn row24_apply_operation_with_test_callback() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x0024);
    let cbs: [(&str, CounterFn); 3] = [
        ("identity", cb_identity),
        ("minus_one", cb_minus_one),
        ("negate", cb_negate),
    ];
    for (name, cb) in cbs {
        for _ in 0..2000 {
            let v = rng.interesting_i32();
            let a = unsafe { (p.c.apply_operation)(Some(cb), v) };
            let b = unsafe { (p.rs.apply_operation)(Some(cb), v) };
            assert_eq!(a, b, "row24 {name}: apply_operation(cb, {v})");
        }
    }
}

// ------------------------------------------------------------- rows 25..=34
// `charinbuf` — return value *and* stdout bytes compared.

#[test]
fn row25_charinbuf_mode0_in_range() {
    let _g = guard();
    for v in [0, 1, 2, 255, 256, 65534, 65535] {
        diff_charinbuf(0, v, 0, 0, "row25/fixed");
    }
    let mut rng = Rng::new(0x0025);
    for _ in 0..400 {
        let v = rng.range_i32(0, 65535);
        diff_charinbuf(0, v, rng.interesting_i32(), rng.interesting_i32(), "row25");
    }
}

#[test]
fn row26_charinbuf_mode0_out_of_range() {
    let _g = guard();
    for v in [-1, -2, 65536, 65537, i32::MIN, i32::MAX] {
        diff_charinbuf(0, v, 0, 0, "row26/fixed");
    }
    let mut rng = Rng::new(0x0026);
    for _ in 0..400 {
        let v = if rng.below(2) == 0 {
            rng.range_i32(i32::MIN, -1)
        } else {
            rng.range_i32(65536, i32::MAX)
        };
        diff_charinbuf(0, v, rng.interesting_i32(), rng.interesting_i32(), "row26");
    }
}

#[test]
fn row27_charinbuf_mode1() {
    let _g = guard();
    diff_charinbuf(1, 0, 0, 0, "row27/zeros");
    let mut rng = Rng::new(0x0027);
    for _ in 0..300 {
        diff_charinbuf(
            1,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            "row27",
        );
    }
}

#[test]
fn row28_charinbuf_mode2() {
    let _g = guard();
    diff_charinbuf(2, 0, 0, 0, "row28/zeros");
    let mut rng = Rng::new(0x0028);
    for _ in 0..300 {
        diff_charinbuf(
            2,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            "row28",
        );
    }
}

#[test]
fn row29_charinbuf_mode3_small() {
    let _g = guard();
    for v in -5..=5 {
        for o1 in -3..=3 {
            for o2 in -3..=3 {
                diff_charinbuf(3, v, o1, o2, "row29");
            }
        }
    }
}

#[test]
fn row30_charinbuf_mode3_zero_operands() {
    let _g = guard();
    let mut rng = Rng::new(0x0030);
    for _ in 0..300 {
        let v = rng.interesting_i32();
        diff_charinbuf(3, v, 0, rng.interesting_i32(), "row30/opt1-zero");
        diff_charinbuf(3, v, rng.interesting_i32(), 0, "row30/opt2-zero");
        diff_charinbuf(3, v, 0, 0, "row30/both-zero");
        diff_charinbuf(3, 0, 0, 0, "row30/all-zero");
    }
}

#[test]
fn row31_charinbuf_mode3_overflow() {
    let _g = guard();
    const EDGE: [i32; 8] = [i32::MIN, i32::MIN + 1, -65536, -1, 0, 1, 65536, i32::MAX];
    for &v in &EDGE {
        for &o1 in &EDGE {
            for &o2 in &EDGE {
                diff_charinbuf(3, v, o1, o2, "row31/edges");
            }
        }
    }
    let mut rng = Rng::new(0x0031);
    for _ in 0..2000 {
        diff_charinbuf(
            3,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            "row31/random",
        );
    }
}

#[test]
fn row32_charinbuf_mode4() {
    let _g = guard();
    diff_charinbuf(4, 0, 0, 0, "row32/zeros");
    let mut rng = Rng::new(0x0032);
    for _ in 0..300 {
        diff_charinbuf(
            4,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            "row32",
        );
    }
}

#[test]
fn row33_charinbuf_invalid_modes() {
    let _g = guard();
    for m in [-1, -2, 5, 6, 7, 100, i32::MIN, i32::MIN + 1, i32::MAX] {
        diff_charinbuf(m, 0, 0, 0, "row33/fixed");
    }
    let mut rng = Rng::new(0x0033);
    for _ in 0..600 {
        let mut m = rng.interesting_i32();
        if (0..=4).contains(&m) {
            m = m.wrapping_add(5);
        }
        diff_charinbuf(
            m,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            "row33/random",
        );
    }
}

#[test]
fn row34_charinbuf_mode_sequences() {
    let _g = guard();
    let mut rng = Rng::new(0x0034);
    for _ in 0..300 {
        let len = 1 + rng.below(8);
        for _ in 0..len {
            let m = rng.range_i32(-1, 5);
            diff_charinbuf(
                m,
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                "row34/sequence",
            );
        }
    }
}

// ------------------------------------------------------------------- row 35

#[test]
fn row35_no_binary_driver() {
    // `c_src/CMakeLists.txt` has a single `add_library(... SHARED src/lib.c)`
    // target and no `add_executable`; `translation/Cargo.toml` declares only
    // `crate-type = ["cdylib"]` and has no `[[bin]]`. There is no driver
    // executable, so there is no binary stdout to compare. Asserted here so the
    // row is machine-checked rather than merely claimed.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cmake = std::fs::read_to_string(root.join("../c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "the C build grew an executable target; Phase B must compare its stdout"
    );
    let toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        !toml.contains("[[bin]]"),
        "the Rust crate grew a binary target"
    );
    assert!(!root.join("src/main.rs").exists(), "src/main.rs appeared");
}

// ------------------------------------------------------------------- row 36
// `charinbuf` zeroes the file-scope counter on entry (lib.c:101) for *every*
// mode. Modes 0/1/2/4/default never read the counter, and mode 3 overwrites it
// immediately, so the reset is invisible unless the counter is seeded first and
// then read back through a low-level entry point afterwards. Without this row a
// translation that simply omits the reset passes everything else.

#[test]
fn row36_charinbuf_zeroes_counter_on_entry_for_every_mode() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x0036);
    // Every mode, including the ones that never touch the counter, plus the
    // `default` arm.
    for mode in [0, 1, 2, 3, 4, 5, -1, 99, i32::MIN, i32::MAX] {
        for _ in 0..50 {
            let seed = rng.interesting_i32();
            // Seed both counters to the same non-zero-ish value.
            unsafe { (p.c.reset_counter)(seed) };
            unsafe { (p.rs.reset_counter)(seed) };

            let v = rng.interesting_i32();
            let (rc, oc) = capture(|| unsafe { (p.c.charinbuf)(mode, v, 3, 4) });
            let (rr, or) = capture(|| unsafe { (p.rs.charinbuf)(mode, v, 3, 4) });
            assert_eq!(rc, rr, "row36 charinbuf({mode},{v},3,4) return, seed {seed}");
            assert_eq!(oc, or, "row36 charinbuf({mode},{v},3,4) stdout, seed {seed}");

            // Read the counter back without disturbing it: `increment_counter(0)`
            // returns the current value.
            let a = unsafe { (p.c.increment_counter)(0) };
            let b = unsafe { (p.rs.increment_counter)(0) };
            assert_eq!(
                a, b,
                "row36: counter after charinbuf(mode={mode}) with seed {seed}: \
                 C={a} Rust={b}"
            );
            if mode != 3 {
                assert_eq!(
                    a, 0,
                    "row36: C must have zeroed the counter on entry to \
                     charinbuf(mode={mode}), seed {seed}"
                );
            }
        }
    }
}

// The mirror of row 36 for the other three low-level readers, so the residue is
// observed through every entry point rather than just `increment_counter`.
#[test]
fn row36b_entry_reset_observed_through_all_counter_ops() {
    let _g = guard();
    let p = pair();
    let mut rng = Rng::new(0x036B);
    let cops = p.c.ops();
    let rops = p.rs.ops();
    for mode in [0, 1, 2, 4, 7, -3] {
        for which in 1..4 {
            for _ in 0..40 {
                let seed = rng.interesting_i32();
                unsafe { (p.c.reset_counter)(seed) };
                unsafe { (p.rs.reset_counter)(seed) };
                let (rc, _) = capture(|| unsafe { (p.c.charinbuf)(mode, 11, 22, 33) });
                let (rr, _) = capture(|| unsafe { (p.rs.charinbuf)(mode, 11, 22, 33) });
                assert_eq!(rc, rr);
                let x = rng.interesting_i32();
                let a = unsafe { (cops[which].1)(x) };
                let b = unsafe { (rops[which].1)(x) };
                assert_eq!(
                    a, b,
                    "row36b: {}({x}) after charinbuf({mode}) seeded with {seed}",
                    cops[which].0
                );
            }
        }
    }
}
