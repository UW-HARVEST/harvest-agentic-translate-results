// Phase B — CONFIGS.md rows C21..C26
//
// `compute_checksum(int* values, int count)`:
//   copy_count = min(count, 4)
//   memcpy 4*copy_count bytes into a 16-byte unsigned char buffer
//   for each byte: checksum = (checksum << 1) ^ byte
//   checksum ^= 0xDEADBEEF
//   return checksum & 0x0000FFFF
//
// `count` changes the NUMBER of rounds, so each of 1..4 is a distinct code path,
// and the byte values matter (little-endian byte order of each int).

mod common;
use common::*;

const S: u64 = 0xC4EC_5000_ABCD_0001;

/// Call `compute_checksum` on both libraries with the same input and compare.
#[track_caller]
fn diff_checksum(ctx: &str, values: &[i32], count: i32) {
    let (c, r) = libs();
    // Separate copies, so a library that (incorrectly) writes through the
    // pointer cannot influence the other's input.
    let mut cv_in = values.to_vec();
    let mut rv_in = values.to_vec();

    let (cv, co) = capture(|| unsafe { (c.compute_checksum)(cv_in.as_mut_ptr(), count) });
    let (rv, ro) = capture(|| unsafe { (r.compute_checksum)(rv_in.as_mut_ptr(), count) });

    assert_eq!(
        cv, rv,
        "{ctx}: checksum differs (C=0x{cv:08X}, Rust=0x{rv:08X}) for count={count}, values={values:?}"
    );
    assert_stdout_eq(ctx, &co, &ro);
    assert!(co.is_empty(), "{ctx}: C printed {:?}", show(&co));
    // The input array must not be modified by either library.
    assert_eq!(cv_in, values, "{ctx}: C modified its input array");
    assert_eq!(rv_in, values, "{ctx}: Rust modified its input array");
    // Documented postcondition: masked to 16 bits.
    assert_eq!(cv & !0xFFFF, 0, "{ctx}: result not masked to 16 bits");
}

// ---------------------------------------------------------------------------
// C21..C24 — count = 1, 2, 3, 4 with random arrays
// ---------------------------------------------------------------------------

fn random_count_rows(count: i32, n: usize, seed: u64, label: &str) {
    let mut rng = Rng::new(seed);
    for i in 0..n {
        // Always give the library a full 4-element backing array; `count`
        // controls how much it is allowed to read.
        let values: Vec<i32> = (0..4).map(|_| rng.interesting_i32()).collect();
        diff_checksum(&format!("{label} iter {i}"), &values, count);
    }
}

#[test]
fn c21_compute_checksum_count_1() {
    random_count_rows(1, 500, S ^ 1, "C21 count=1");
}

#[test]
fn c22_compute_checksum_count_2() {
    random_count_rows(2, 500, S ^ 2, "C22 count=2");
}

#[test]
fn c23_compute_checksum_count_3() {
    random_count_rows(3, 500, S ^ 3, "C23 count=3");
}

#[test]
fn c24_compute_checksum_count_4() {
    random_count_rows(4, 500, S ^ 4, "C24 count=4");
}

// ---------------------------------------------------------------------------
// C25 — special byte patterns for every count 1..4 (axis A4)
// ---------------------------------------------------------------------------

#[test]
fn c25_compute_checksum_special_byte_patterns() {
    let patterns: &[[i32; 4]] = &[
        [0, 0, 0, 0],
        [-1, -1, -1, -1],                            // all 0xFF bytes
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],    // 0x80000000 sign bit
        [0x0000_00FF, 0x0000_00FF, 0x0000_00FF, 0x0000_00FF],
        [0xFF00_0000u32 as i32; 4],
        [0xAA55_AA55u32 as i32; 4],
        [0x55AA_55AAu32 as i32; 4],
        [0x0102_0304, 0x0506_0708, 0x090A_0B0C, 0x0D0E_0F10], // endianness-sensitive
        [1, 2, 3, 4],
        [i32::MAX, i32::MIN, -1, 0],
        [0xDEAD_BEEFu32 as i32, 0xCAFE_BABEu32 as i32, 0x1234_5678, 0x8765_4321u32 as i32],
        [0x0000_0001, 0x0000_0100, 0x0001_0000, 0x0100_0000], // one bit per byte lane
        [0x8000_0000u32 as i32, 0x0080_0000, 0x0000_8000, 0x0000_0080],
        [0x7F7F_7F7F, 0x8080_8080u32 as i32, 0x0F0F_0F0F, 0xF0F0_F0F0u32 as i32],
    ];

    for (p, pat) in patterns.iter().enumerate() {
        for count in 1..=4i32 {
            diff_checksum(
                &format!("C25 pattern {p} {pat:08X?} count={count}"),
                pat,
                count,
            );
        }
    }
}

#[test]
fn c25b_compute_checksum_single_byte_sweep() {
    // Sweep every possible value of the first byte with count=1: 256 distinct
    // round sequences through `checksum = (checksum << 1) ^ byte`.
    for b in 0..256i32 {
        let values = [b, 0, 0, 0];
        diff_checksum(&format!("C25b first byte = 0x{b:02X}"), &values, 1);
    }
    // And sweep each byte lane of a single int.
    for lane in 0..4 {
        for b in [0x01i32, 0x7F, 0x80, 0xFF] {
            let v = b << (8 * lane);
            let values = [v, 0, 0, 0];
            diff_checksum(&format!("C25b lane {lane} byte 0x{b:02X}"), &values, 1);
        }
    }
}

// ---------------------------------------------------------------------------
// C26 — count > 4: clamped to 4 by `(count > 4) ? 4 : count`
// ---------------------------------------------------------------------------

#[test]
fn c26_compute_checksum_count_clamped() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 0x26);

    for i in 0..200 {
        // Over-long backing array; the library must only ever read 4 ints.
        let values: Vec<i32> = (0..64).map(|_| rng.interesting_i32()).collect();

        let mut base = values.clone();
        let expect = unsafe { (c.compute_checksum)(base.as_mut_ptr(), 4) };

        for &count in &[5i32, 6, 7, 8, 16, 64, 1000, 0x10_0000, i32::MAX, i32::MAX - 1] {
            diff_checksum(&format!("C26 iter {i} count={count}"), &values, count);

            // Clamp semantics: identical to count == 4.
            let mut cb = values.clone();
            let mut rb = values.clone();
            let cv = unsafe { (c.compute_checksum)(cb.as_mut_ptr(), count) };
            let rv = unsafe { (r.compute_checksum)(rb.as_mut_ptr(), count) };
            assert_eq!(
                cv, expect,
                "C26 iter {i}: C count={count} must equal count=4"
            );
            assert_eq!(
                rv, expect,
                "C26 iter {i}: Rust count={count} must equal count=4"
            );
        }
    }
}

#[test]
fn c26b_compute_checksum_clamp_ignores_tail() {
    // Changing elements beyond index 3 must not change the result for any
    // count >= 4 -- proves the clamp truly bounds the read.
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 0x26B);

    for i in 0..100 {
        let head: Vec<i32> = (0..4).map(|_| rng.interesting_i32()).collect();

        let mut a: Vec<i32> = head.clone();
        a.extend((0..12).map(|_| 0i32));
        let mut b: Vec<i32> = head.clone();
        b.extend((0..12).map(|_| rng.i32()));

        for &count in &[4i32, 5, 16, i32::MAX] {
            let mut a1 = a.clone();
            let mut b1 = b.clone();
            let ca = unsafe { (c.compute_checksum)(a1.as_mut_ptr(), count) };
            let cb = unsafe { (c.compute_checksum)(b1.as_mut_ptr(), count) };
            assert_eq!(ca, cb, "C26b iter {i} count={count}: C read past index 3");

            let mut a2 = a.clone();
            let mut b2 = b.clone();
            let ra = unsafe { (r.compute_checksum)(a2.as_mut_ptr(), count) };
            let rb = unsafe { (r.compute_checksum)(b2.as_mut_ptr(), count) };
            assert_eq!(ra, rb, "C26b iter {i} count={count}: Rust read past index 3");
            assert_eq!(ca, ra, "C26b iter {i} count={count}: C vs Rust");
        }
    }
}
