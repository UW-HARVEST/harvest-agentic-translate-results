//! Phase B — valid-path differential tests for the *lowest level* exported
//! entry points (CONFIGS.md rows 1–61).
//!
//! Every test drives both `.so`s through `libloading` and compares the return
//! value, stdout, stderr and the full 272-byte image of every `buffer_t`.

mod common;

use common::*;

const ITERS: usize = 120;

// ============================================================ rows 1–5 ======
// calculate_checksum

fn diff_checksum(what: &str, data: &[u8], length: usize) {
    let (c, r) = both();
    // Pass the pointer exactly as a C caller would, including the NULL/0 case.
    let p = if data.is_empty() {
        core::ptr::null()
    } else {
        data.as_ptr()
    };
    let co = observe(None, || unsafe { (c.calculate_checksum)(p, length) });
    let ro = observe(None, || unsafe { (r.calculate_checksum)(p, length) });
    same(what, &co, &ro);
}

#[test]
fn row01_checksum_length_zero() {
    // NULL data with length 0 (never dereferenced) and non-NULL data with 0.
    diff_checksum("row01/null+0", &[], 0);
    diff_checksum("row01/ptr+0", &[1, 2, 3], 0);
    let mut rng = Rng::new(0x01);
    for _ in 0..ITERS {
        let n = 1 + rng.below(64);
        let v: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
        diff_checksum("row01/rand+0", &v, 0);
    }
}

#[test]
fn row02_checksum_length_one() {
    for b in 0..=255u8 {
        diff_checksum("row02", &[b], 1);
    }
}

#[test]
fn row03_checksum_random_lengths() {
    let mut rng = Rng::new(0x03);
    for _ in 0..ITERS * 4 {
        let n = 2 + rng.below(254);
        let v: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
        diff_checksum("row03", &v, n);
        // also a length shorter than the slice
        diff_checksum("row03/short", &v, rng.below(n + 1));
    }
}

#[test]
fn row04_checksum_length_256() {
    let mut rng = Rng::new(0x04);
    for _ in 0..ITERS {
        let v: Vec<u8> = (0..256).map(|_| rng.u8()).collect();
        diff_checksum("row04", &v, 256);
    }
    // Also lengths well past 256 over a correspondingly large buffer: the C
    // function itself has no limit, it just walks `length` bytes.
    let mut rng = Rng::new(0x44);
    for n in [257usize, 512, 1000, 4096] {
        let v: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
        diff_checksum("row04/large", &v, n);
    }
}

#[test]
fn row05_checksum_extreme_byte_patterns() {
    for n in [1usize, 2, 3, 7, 8, 9, 10, 11, 31, 32, 33, 255, 256] {
        diff_checksum("row05/zeros", &vec![0x00u8; n], n);
        diff_checksum("row05/ones", &vec![0xFFu8; n], n);
        diff_checksum("row05/high", &vec![0x80u8; n], n);
        diff_checksum("row05/alt", &(0..n).map(|i| if i % 2 == 0 { 0 } else { 0xFF }).collect::<Vec<u8>>(), n);
        // 0x01 repeated exercises the `sum << 3` shift-out behaviour
        diff_checksum("row05/shift", &vec![0x01u8; n], n);
    }
}

// =========================================================== rows 6–8 ======
// validate_buffer

fn diff_validate(what: &str, b: BufferT) {
    let (c, r) = both();
    let cb = b;
    let rb = b;
    let co = observe(None, || unsafe { (c.validate_buffer)(&cb) });
    let ro = observe(None, || unsafe { (r.validate_buffer)(&rb) });
    same(what, &co, &ro);
    same_buf(what, &cb, &rb);
}

#[test]
fn row06_validate_length_zero_consistent() {
    let mut rng = Rng::new(0x06);
    for _ in 0..ITERS {
        let mut b = rng.buffer_len(0);
        b.checksum = 0; // consistent: checksum of nothing is 0
        diff_validate("row06", b);
    }
}

#[test]
fn row07_validate_consistent_checksum() {
    let mut rng = Rng::new(0x07);
    for _ in 0..ITERS * 3 {
        let b = rng.buffer(1, 256); // `Rng::buffer` sets a consistent checksum
        diff_validate("row07", b);
    }
    for len in [1usize, 2, 3, 127, 128, 254, 255, 256] {
        let b = Rng::new(len as u64).buffer_len(len);
        diff_validate("row07/fixed", b);
    }
}

#[test]
fn row08_validate_corrupted_checksum_warns() {
    let mut rng = Rng::new(0x08);
    for _ in 0..ITERS * 3 {
        let mut b = rng.buffer(0, 256);
        // Force a mismatch; also exercise the u32 formatting of large values.
        b.checksum = rng.next_u32();
        if b.checksum == checksum(&b.data[..b.length]) {
            b.checksum = b.checksum.wrapping_add(1);
        }
        diff_validate("row08", b);
    }
    for cks in [0u32, 1, 0x7FFF_FFFF, 0x8000_0000, u32::MAX] {
        let mut b = Rng::new(cks as u64).buffer_len(4);
        b.checksum = cks;
        diff_validate("row08/edge", b);
    }
}

// ============================================================== row 9 ======
// init_buffer_array / free_buffer_array

#[test]
fn row09_init_and_free_buffer_array() {
    let (c, r) = both();
    for cap in [1i32, 2, 3, 4, 7, 8, 64, 99, 100, 101, 1000, 65536] {
        let co = observe(None, || unsafe {
            let a = (c.init_buffer_array)(cap);
            let repr = if a.is_null() {
                (true, 0, 0, false)
            } else {
                (false, (*a).count, (*a).capacity, (*a).buffers.is_null())
            };
            (c.free_buffer_array)(a);
            repr
        });
        let ro = observe(None, || unsafe {
            let a = (r.init_buffer_array)(cap);
            let repr = if a.is_null() {
                (true, 0, 0, false)
            } else {
                (false, (*a).count, (*a).capacity, (*a).buffers.is_null())
            };
            (r.free_buffer_array)(a);
            repr
        });
        same(&format!("row09/cap={}", cap), &co, &ro);
        assert_eq!(co.ret, (false, 0, cap, false), "cap={}", cap);
    }
}

#[test]
fn row09_free_array_allocated_by_the_other_library() {
    // The two `init_buffer_array`s must produce interchangeable objects: both
    // use malloc/free, so an array from one can be released by the other.
    let (c, r) = both();
    let o = observe(None, || unsafe {
        let a = (c.init_buffer_array)(5);
        assert!(!a.is_null());
        let ok = (*a).count == 0 && (*a).capacity == 5;
        (r.free_buffer_array)(a);
        let b = (r.init_buffer_array)(5);
        assert!(!b.is_null());
        let ok2 = (*b).count == 0 && (*b).capacity == 5;
        (c.free_buffer_array)(b);
        ok && ok2
    });
    assert!(o.ret, "cross-library allocate/free failed");
    assert!(o.stderr.is_empty() && o.stdout.is_empty());
}

// ========================================================== rows 10–14 =====
// buffer_copy

fn copy_body(api: &Api, p: *mut BufferT) -> i64 {
    unsafe { (api.buffer_copy)(p, p.add(1)) as i64 }
}

#[test]
fn row10_copy_length_zero() {
    let mut rng = Rng::new(0x10);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        diff_bufs(
            "row10",
            || {
                let mut g = Rng::new(seed);
                let mut src = g.buffer_len(0);
                src.checksum = 0;
                let dl = g.below(200);
                let dst = g.buffer_len(dl);
                vec![src, dst]
            },
            copy_body,
            true,
        );
    }
}

#[test]
fn row11_copy_random_lengths() {
    let mut rng = Rng::new(0x11);
    for _ in 0..ITERS * 3 {
        let seed = rng.next_u64();
        diff_bufs(
            "row11",
            || {
                let mut g = Rng::new(seed);
                let src = g.buffer(1, 255);
                let mut dst = BufferT::patterned(0xA5);
                dst.length = g.below(300);
                vec![src, dst]
            },
            copy_body,
            true,
        );
    }
}

#[test]
fn row12_copy_length_256() {
    for seed in 0..40u64 {
        diff_bufs(
            "row12",
            || {
                let mut g = Rng::new(0x1200 + seed);
                let src = g.buffer_len(256);
                let dst = BufferT::patterned(0x5A);
                vec![src, dst]
            },
            copy_body,
            true,
        );
    }
}

#[test]
fn row13_copy_inconsistent_source_checksum() {
    let mut rng = Rng::new(0x13);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        diff_bufs(
            "row13",
            || {
                let mut g = Rng::new(seed);
                let mut src = g.buffer(0, 256);
                src.checksum = g.next_u32();
                let dst = BufferT::patterned(0x11);
                vec![src, dst]
            },
            copy_body,
            true,
        );
    }
}

#[test]
fn row14_copy_aliased_src_eq_dst() {
    let mut rng = Rng::new(0x14);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        diff_bufs(
            "row14",
            || vec![Rng::new(seed).buffer(0, 256)],
            |api, p| unsafe { (api.buffer_copy)(p, p) as i64 },
            true,
        );
    }
}

// ========================================================== rows 15–19 =====
// buffer_reverse

fn reverse_body(api: &Api, p: *mut BufferT) -> i64 {
    unsafe { (api.buffer_reverse)(p) as i64 }
}

#[test]
fn row15_reverse_length_zero_keeps_checksum() {
    let mut rng = Rng::new(0x15);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        diff_bufs(
            "row15",
            || {
                let mut g = Rng::new(seed);
                let mut b = g.buffer_len(0);
                b.checksum = g.next_u32(); // must be left untouched
                vec![b]
            },
            reverse_body,
            true,
        );
    }
}

#[test]
fn row16_reverse_length_one() {
    for seed in 0..40u64 {
        diff_bufs(
            "row16",
            || vec![Rng::new(0x1600 + seed).buffer_len(1)],
            reverse_body,
            true,
        );
    }
}

#[test]
fn row17_reverse_even_lengths() {
    let mut rng = Rng::new(0x17);
    for _ in 0..ITERS * 2 {
        let len = 2 * (1 + rng.below(128));
        let seed = rng.next_u64();
        diff_bufs(
            "row17",
            || vec![Rng::new(seed).buffer_len(len)],
            reverse_body,
            true,
        );
    }
}

#[test]
fn row18_reverse_odd_lengths() {
    let mut rng = Rng::new(0x18);
    for _ in 0..ITERS * 2 {
        let len = 2 * rng.below(128) + 1;
        let seed = rng.next_u64();
        diff_bufs(
            "row18",
            || vec![Rng::new(seed).buffer_len(len)],
            reverse_body,
            true,
        );
    }
}

#[test]
fn row19_reverse_twice_is_identity() {
    let mut rng = Rng::new(0x19);
    for _ in 0..ITERS {
        let len = rng.below(257);
        let seed = rng.next_u64();
        diff_bufs(
            "row19",
            || vec![Rng::new(seed).buffer_len(len)],
            |api, p| unsafe {
                let a = (api.buffer_reverse)(p) as i64;
                let b = (api.buffer_reverse)(p) as i64;
                a * 1000 + b
            },
            true,
        );
    }
}

// ========================================================== rows 20–27 =====
// buffer_merge

fn merge_body(api: &Api, p: *mut BufferT) -> i64 {
    unsafe { (api.buffer_merge)(p, p.add(1), p.add(2)) as i64 }
}

fn merge_case(what: &str, seed: u64, l1: usize, l2: usize) {
    diff_bufs(
        what,
        || {
            let mut g = Rng::new(seed);
            let a = g.buffer_len(l1);
            let b = g.buffer_len(l2);
            let mut d = BufferT::patterned(0x77);
            d.length = g.below(300);
            vec![a, b, d]
        },
        merge_body,
        true,
    );
}

#[test]
fn row20_merge_both_empty() {
    for seed in 0..30u64 {
        merge_case("row20", 0x2000 + seed, 0, 0);
    }
}

#[test]
fn row21_merge_first_empty() {
    let mut rng = Rng::new(0x21);
    for _ in 0..ITERS {
        merge_case("row21", rng.next_u64(), 0, 1 + rng.below(256));
    }
}

#[test]
fn row22_merge_second_empty() {
    let mut rng = Rng::new(0x22);
    for _ in 0..ITERS {
        merge_case("row22", rng.next_u64(), 1 + rng.below(256), 0);
    }
}

#[test]
fn row23_merge_equal_lengths() {
    let mut rng = Rng::new(0x23);
    for _ in 0..ITERS {
        let l = rng.below(129);
        merge_case("row23", rng.next_u64(), l, l);
    }
}

#[test]
fn row24_merge_unequal_lengths() {
    let mut rng = Rng::new(0x24);
    for _ in 0..ITERS * 2 {
        let l1 = rng.below(257);
        let l2 = rng.below(257);
        merge_case("row24", rng.next_u64(), l1, l2);
    }
}

#[test]
fn row25_merge_sum_exactly_256() {
    for l1 in 0..=256usize {
        merge_case("row25", 0x2500 + l1 as u64, l1, 256 - l1);
    }
}

#[test]
fn row26_merge_destination_tail_preserved() {
    let mut rng = Rng::new(0x26);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let l1 = rng.below(100);
        let l2 = rng.below(100);
        diff_bufs(
            "row26",
            || {
                let mut g = Rng::new(seed);
                let a = g.buffer_len(l1);
                let b = g.buffer_len(l2);
                // Destination fully pre-filled: any byte beyond l1+l2 that the
                // implementation touches shows up as a divergence.
                let mut d = g.buffer_len(256);
                d.checksum = 0x1234_5678;
                vec![a, b, d]
            },
            merge_body,
            true,
        );
    }
}

#[test]
fn row27_merge_aliased_sources() {
    let mut rng = Rng::new(0x27);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let l = rng.below(129);
        diff_bufs(
            "row27",
            || {
                let mut g = Rng::new(seed);
                let a = g.buffer_len(l);
                let d = BufferT::patterned(0x33);
                vec![a, d]
            },
            |api, p| unsafe { (api.buffer_merge)(p, p, p.add(1)) as i64 },
            true,
        );
    }
}

// ========================================================== rows 28–33 =====
// buffer_split

fn split_case(what: &str, seed: u64, len: usize, pos: usize) {
    diff_bufs(
        what,
        || {
            let mut g = Rng::new(seed);
            let s = g.buffer_len(len);
            let mut d1 = BufferT::patterned(0xC1);
            d1.length = g.below(300);
            let mut d2 = BufferT::patterned(0xC2);
            d2.length = g.below(300);
            vec![s, d1, d2]
        },
        move |api, p| unsafe { (api.buffer_split)(p, pos, p.add(1), p.add(2)) as i64 },
        true,
    );
}

#[test]
fn row28_split_at_zero() {
    let mut rng = Rng::new(0x28);
    for _ in 0..ITERS {
        split_case("row28", rng.next_u64(), rng.below(257), 0);
    }
}

#[test]
fn row29_split_at_length() {
    let mut rng = Rng::new(0x29);
    for _ in 0..ITERS {
        let len = rng.below(257);
        split_case("row29", rng.next_u64(), len, len);
    }
}

#[test]
fn row30_split_interior() {
    let mut rng = Rng::new(0x30);
    for _ in 0..ITERS * 3 {
        let len = 1 + rng.below(256);
        let pos = rng.below(len + 1);
        split_case("row30", rng.next_u64(), len, pos);
    }
    // exhaustive over a small length, plus the two extremes of the max length
    for len in 0..=8usize {
        for pos in 0..=len {
            split_case("row30/small", 0x3000 + (len * 16 + pos) as u64, len, pos);
        }
    }
    for pos in [0usize, 1, 128, 255, 256] {
        split_case("row30/max", 0x3100 + pos as u64, 256, pos);
    }
}

#[test]
fn row31_split_empty_source() {
    for seed in 0..30u64 {
        split_case("row31", 0x3100 + seed, 0, 0);
    }
}

#[test]
fn row32_split_destinations_prefilled() {
    let mut rng = Rng::new(0x32);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let len = rng.below(257);
        let pos = rng.below(len + 1);
        diff_bufs(
            "row32",
            || {
                let mut g = Rng::new(seed);
                let s = g.buffer_len(len);
                let d1 = g.buffer_len(256);
                let d2 = g.buffer_len(256);
                vec![s, d1, d2]
            },
            move |api, p| unsafe { (api.buffer_split)(p, pos, p.add(1), p.add(2)) as i64 },
            true,
        );
    }
}

#[test]
fn row33_split_aliased_destinations() {
    let mut rng = Rng::new(0x33);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let len = rng.below(257);
        let pos = rng.below(len + 1);
        // dst1 == dst2: the C code writes dst1 first, then dst2 over it.
        diff_bufs(
            "row33/d1eqd2",
            || {
                let mut g = Rng::new(seed);
                let s = g.buffer_len(len);
                let d = BufferT::patterned(0x99);
                vec![s, d]
            },
            move |api, p| unsafe { (api.buffer_split)(p, pos, p.add(1), p.add(1)) as i64 },
            true,
        );
        // src == dst1 (in-place truncation)
        diff_bufs(
            "row33/seqd1",
            || {
                let mut g = Rng::new(seed);
                let s = g.buffer_len(len);
                let d = BufferT::patterned(0x9A);
                vec![s, d]
            },
            move |api, p| unsafe { (api.buffer_split)(p, pos, p, p.add(1)) as i64 },
            true,
        );
    }
}

// ========================================================== rows 34–40 =====
// buffer_interleave

fn interleave_case(what: &str, seed: u64, l1: usize, l2: usize) {
    diff_bufs(
        what,
        || {
            let mut g = Rng::new(seed);
            let a = g.buffer_len(l1);
            let b = g.buffer_len(l2);
            let mut d = BufferT::patterned(0xE7);
            d.length = g.below(300);
            vec![a, b, d]
        },
        |api, p| unsafe { (api.buffer_interleave)(p, p.add(1), p.add(2)) as i64 },
        true,
    );
}

#[test]
fn row34_interleave_equal_lengths() {
    let mut rng = Rng::new(0x34);
    for _ in 0..ITERS {
        let l = rng.below(129);
        interleave_case("row34", rng.next_u64(), l, l);
    }
}

#[test]
fn row35_interleave_first_longer() {
    let mut rng = Rng::new(0x35);
    for _ in 0..ITERS {
        let l2 = rng.below(128);
        let l1 = l2 + 1 + rng.below(128 - (l2.min(127)));
        interleave_case("row35", rng.next_u64(), l1, l2);
    }
}

#[test]
fn row36_interleave_second_longer() {
    let mut rng = Rng::new(0x36);
    for _ in 0..ITERS {
        let l1 = rng.below(128);
        let l2 = l1 + 1 + rng.below(128 - (l1.min(127)));
        interleave_case("row36", rng.next_u64(), l1, l2);
    }
}

#[test]
fn row37_interleave_one_side_empty() {
    let mut rng = Rng::new(0x37);
    for _ in 0..ITERS {
        let l = 1 + rng.below(256);
        interleave_case("row37/a", rng.next_u64(), l, 0);
        interleave_case("row37/b", rng.next_u64(), 0, l);
    }
}

#[test]
fn row38_interleave_both_empty() {
    for seed in 0..30u64 {
        interleave_case("row38", 0x3800 + seed, 0, 0);
    }
}

#[test]
fn row39_interleave_sum_exactly_256() {
    for l1 in 0..=256usize {
        interleave_case("row39", 0x3900 + l1 as u64, l1, 256 - l1);
    }
}

#[test]
fn row40_interleave_aliased() {
    let mut rng = Rng::new(0x40);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let l = rng.below(129);
        diff_bufs(
            "row40/s1eqs2",
            || {
                let mut g = Rng::new(seed);
                let a = g.buffer_len(l);
                let d = BufferT::patterned(0x44);
                vec![a, d]
            },
            |api, p| unsafe { (api.buffer_interleave)(p, p, p.add(1)) as i64 },
            true,
        );
    }
}

// ========================================================== rows 41–49 =====
// buffer_rotate

fn rotate_case(what: &str, seed: u64, len: usize, positions: i32) {
    diff_bufs(
        what,
        || {
            let mut g = Rng::new(seed);
            let mut b = g.buffer_len(len);
            if len == 0 {
                b.checksum = g.next_u32(); // must survive the early return
            }
            vec![b]
        },
        move |api, p| unsafe { (api.buffer_rotate)(p, positions) as i64 },
        true,
    );
}

#[test]
fn row41_rotate_zero_positions() {
    let mut rng = Rng::new(0x41);
    for _ in 0..ITERS {
        rotate_case("row41", rng.next_u64(), rng.below(257), 0);
    }
}

#[test]
fn row42_rotate_zero_length() {
    let mut rng = Rng::new(0x42);
    for _ in 0..ITERS {
        let p = rng.i32();
        rotate_case("row42", rng.next_u64(), 0, p);
    }
}

#[test]
fn row43_rotate_interior() {
    let mut rng = Rng::new(0x43);
    for _ in 0..ITERS * 3 {
        let len = 1 + rng.below(256);
        let pos = 1 + rng.below(len);
        rotate_case("row43", rng.next_u64(), len, pos as i32);
    }
    // exhaustive for a small length
    for len in 1..=8usize {
        for pos in 0..=(len as i32 + 1) {
            rotate_case("row43/small", 0x4300 + (len * 16) as u64 + pos as u64, len, pos);
        }
    }
}

#[test]
fn row44_rotate_positions_equals_length() {
    let mut rng = Rng::new(0x44);
    for _ in 0..ITERS {
        let len = 1 + rng.below(256);
        rotate_case("row44", rng.next_u64(), len, len as i32);
    }
}

#[test]
fn row45_rotate_positions_greater_than_length() {
    let mut rng = Rng::new(0x45);
    for _ in 0..ITERS * 2 {
        let len = 1 + rng.below(256);
        let mult = 1 + rng.below(5);
        let extra = rng.below(len);
        rotate_case(
            "row45",
            rng.next_u64(),
            len,
            (len * mult + extra) as i32,
        );
    }
}

#[test]
fn row46_rotate_small_negative() {
    let mut rng = Rng::new(0x46);
    for _ in 0..ITERS * 2 {
        let len = 1 + rng.below(256);
        let pos = -((1 + rng.below(len)) as i32);
        rotate_case("row46", rng.next_u64(), len, pos);
    }
}

#[test]
fn row47_rotate_negative_at_and_past_length() {
    let mut rng = Rng::new(0x47);
    for _ in 0..ITERS {
        let len = 1 + rng.below(256);
        rotate_case("row47/eq", rng.next_u64(), len, -(len as i32));
        let mult = 1 + rng.below(5);
        rotate_case(
            "row47/past",
            rng.next_u64(),
            len,
            -((len * mult + rng.below(len)) as i32),
        );
    }
}

#[test]
fn row48_rotate_int_extremes() {
    let mut rng = Rng::new(0x48);
    for len in [1usize, 2, 3, 5, 7, 16, 100, 255, 256] {
        for pos in [i32::MIN, i32::MIN + 1, -1, 1, i32::MAX - 1, i32::MAX] {
            rotate_case("row48", 0x4800 + len as u64, len, pos);
        }
    }
    for _ in 0..ITERS {
        let len = 1 + rng.below(256);
        let pos = rng.i32();
        rotate_case("row48/rand", rng.next_u64(), len, pos);
    }
}

#[test]
fn row49_rotate_length_one() {
    for pos in [i32::MIN, -3, -2, -1, 0, 1, 2, 3, i32::MAX] {
        rotate_case("row49", 0x4900u64.wrapping_add(pos as i64 as u64), 1, pos);
    }
}

// ========================================================== rows 50–55 =====
// buffer_conditional_copy

fn cond_case(what: &str, seed: u64, len: usize, pattern: u8, copy_matching: u8) {
    diff_bufs(
        what,
        || {
            let mut g = Rng::new(seed);
            let s = g.buffer_len(len);
            let mut d = BufferT::patterned(0x6B);
            d.length = g.below(300);
            vec![s, d]
        },
        move |api, p| unsafe {
            (api.buffer_conditional_copy)(p, p.add(1), pattern, copy_matching) as i64
        },
        true,
    );
}

#[test]
fn row50_cond_copy_matching_true() {
    let mut rng = Rng::new(0x50);
    for _ in 0..ITERS * 2 {
        let seed = rng.next_u64();
        let len = 1 + rng.below(256);
        // Pick a pattern that actually occurs in the data.
        let probe = Rng::new(seed).buffer_len(len);
        let pat = probe.data[rng.below(len)];
        cond_case("row50", seed, len, pat, 1);
    }
}

#[test]
fn row51_cond_copy_matching_false() {
    let mut rng = Rng::new(0x51);
    for _ in 0..ITERS * 2 {
        let seed = rng.next_u64();
        let len = 1 + rng.below(256);
        let probe = Rng::new(seed).buffer_len(len);
        let pat = probe.data[rng.below(len)];
        cond_case("row51", seed, len, pat, 0);
    }
}

#[test]
fn row52_cond_pattern_absent() {
    let mut rng = Rng::new(0x52);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let len = 1 + rng.below(200);
        // Find a byte value that does not occur.
        let probe = Rng::new(seed).buffer_len(len);
        let mut seen = [false; 256];
        for i in 0..len {
            seen[probe.data[i] as usize] = true;
        }
        let pat = (0..256).find(|&v| !seen[v]).unwrap_or(0) as u8;
        cond_case("row52/t", seed, len, pat, 1);
        cond_case("row52/f", seed, len, pat, 0);
    }
}

#[test]
fn row53_cond_all_bytes_match() {
    for len in [1usize, 2, 3, 17, 128, 255, 256] {
        for pat in [0u8, 1, 0x7F, 0x80, 0xFF] {
            diff_bufs(
                "row53",
                || {
                    let mut s = BufferT::zeroed();
                    s.data = [pat; 256];
                    s.length = len;
                    s.checksum = checksum(&s.data[..len]);
                    let d = BufferT::patterned(0x21);
                    vec![s, d]
                },
                move |api, p| unsafe {
                    (api.buffer_conditional_copy)(p, p.add(1), pat, 1) as i64
                },
                true,
            );
            diff_bufs(
                "row53/f",
                || {
                    let mut s = BufferT::zeroed();
                    s.data = [pat; 256];
                    s.length = len;
                    s.checksum = checksum(&s.data[..len]);
                    let d = BufferT::patterned(0x22);
                    vec![s, d]
                },
                move |api, p| unsafe {
                    (api.buffer_conditional_copy)(p, p.add(1), pat, 0) as i64
                },
                true,
            );
        }
    }
}

#[test]
fn row54_cond_boundary_lengths() {
    let mut rng = Rng::new(0x54);
    for cm in [0u8, 1] {
        for len in [0usize, 1, 255, 256] {
            for _ in 0..10 {
                cond_case("row54", rng.next_u64(), len, rng.u8(), cm);
            }
        }
    }
}

#[test]
fn row55_cond_aliased_in_place() {
    let mut rng = Rng::new(0x55);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let len = rng.below(257);
        let pat = rng.u8();
        for cm in [0u8, 1] {
            diff_bufs(
                "row55",
                || vec![Rng::new(seed).buffer_len(len)],
                move |api, p| unsafe {
                    (api.buffer_conditional_copy)(p, p, pat, cm) as i64
                },
                true,
            );
        }
    }
}

// ========================================================== rows 56–61 =====
// buffer_copy_strided

fn strided_case(what: &str, seed: u64, len: usize, stride: i32) {
    diff_bufs(
        what,
        || {
            let mut g = Rng::new(seed);
            let s = g.buffer_len(len);
            let mut d = BufferT::patterned(0x8D);
            d.length = g.below(300);
            vec![s, d]
        },
        move |api, p| unsafe { (api.buffer_copy_strided)(p, p.add(1), stride) as i64 },
        true,
    );
}

#[test]
fn row56_strided_stride_one() {
    let mut rng = Rng::new(0x56);
    for _ in 0..ITERS {
        strided_case("row56", rng.next_u64(), rng.below(257), 1);
    }
}

#[test]
fn row57_strided_small_strides() {
    let mut rng = Rng::new(0x57);
    for stride in [2i32, 3, 4, 5, 7, 8, 16, 17] {
        for _ in 0..30 {
            strided_case("row57", rng.next_u64(), rng.below(257), stride);
        }
    }
    for len in 0..=16usize {
        for stride in 1..=6i32 {
            strided_case("row57/small", 0x5700 + (len * 8) as u64 + stride as u64, len, stride);
        }
    }
}

#[test]
fn row58_strided_stride_at_and_past_length() {
    let mut rng = Rng::new(0x58);
    for _ in 0..ITERS {
        let len = 1 + rng.below(256);
        strided_case("row58/eq", rng.next_u64(), len, len as i32);
        strided_case("row58/past", rng.next_u64(), len, (len + 1 + rng.below(50)) as i32);
    }
}

#[test]
fn row59_strided_stride_int_max() {
    let mut rng = Rng::new(0x59);
    for stride in [i32::MAX, i32::MAX - 1, 1 << 30, 1 << 20] {
        for _ in 0..10 {
            strided_case("row59", rng.next_u64(), rng.below(257), stride);
        }
    }
}

#[test]
fn row60_strided_empty_source() {
    let mut rng = Rng::new(0x60);
    for stride in [1i32, 2, 7, i32::MAX] {
        for _ in 0..10 {
            strided_case("row60", rng.next_u64(), 0, stride);
        }
    }
}

#[test]
fn row61_strided_aliased() {
    let mut rng = Rng::new(0x61);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let len = rng.below(257);
        let stride = 1 + rng.below(8) as i32;
        diff_bufs(
            "row61",
            || vec![Rng::new(seed).buffer_len(len)],
            move |api, p| unsafe { (api.buffer_copy_strided)(p, p, stride) as i64 },
            true,
        );
    }
}

// ================================================= extra aliasing coverage ===
// CONFIGS.md rows 27/33/40 cover the aliasing a caller is most likely to try;
// these add the fully-aliased forms that are still well defined in C (i.e. where
// the C `memcpy` calls either have identical pointers or do not overlap).

#[test]
fn alias_split_all_three_arguments_same() {
    // src == dst1 == dst2: writing dst1->length overwrites src->length, so
    // `remaining` becomes 0 and dst2 ends up empty.
    let mut rng = Rng::new(0xA1);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let len = rng.below(257);
        let pos = rng.below(len + 1);
        diff_bufs(
            "alias/split(p,p,p)",
            || vec![Rng::new(seed).buffer_len(len)],
            move |api, p| unsafe { (api.buffer_split)(p, pos, p, p) as i64 },
            true,
        );
    }
}

#[test]
fn alias_interleave_all_three_arguments_same() {
    // src1 == src2 == dst.  Every transfer in the C code is a one-byte memcpy,
    // so the read/write interleaving is fully defined even though the reads see
    // bytes an earlier iteration wrote.
    let mut rng = Rng::new(0xA2);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let len = rng.below(129); // len + len <= 256
        diff_bufs(
            "alias/interleave(p,p,p)",
            || vec![Rng::new(seed).buffer_len(len)],
            |api, p| unsafe { (api.buffer_interleave)(p, p, p) as i64 },
            true,
        );
    }
    for len in 0..=128usize {
        diff_bufs(
            "alias/interleave(p,p,p)/exhaustive",
            || vec![Rng::new(0xA200 + len as u64).buffer_len(len)],
            |api, p| unsafe { (api.buffer_interleave)(p, p, p) as i64 },
            true,
        );
    }
}

#[test]
fn alias_interleave_dst_equals_one_source() {
    let mut rng = Rng::new(0xA3);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let l1 = rng.below(129);
        let l2 = rng.below(257 - l1);
        diff_bufs(
            "alias/interleave(dst=src1)",
            || {
                let mut g = Rng::new(seed);
                let a = g.buffer_len(l1);
                let b = g.buffer_len(l2);
                vec![a, b]
            },
            |api, p| unsafe { (api.buffer_interleave)(p, p.add(1), p) as i64 },
            true,
        );
        diff_bufs(
            "alias/interleave(dst=src2)",
            || {
                let mut g = Rng::new(seed);
                let a = g.buffer_len(l1);
                let b = g.buffer_len(l2);
                vec![a, b]
            },
            |api, p| unsafe { (api.buffer_interleave)(p, p.add(1), p.add(1)) as i64 },
            true,
        );
    }
}

#[test]
fn alias_merge_dst_equals_source_non_overlapping() {
    // `buffer_merge(p, q, p)` does `memcpy(p->data, p->data, l1)` (identical
    // pointers) and then `memcpy(p->data + l1, q->data, l2)` (disjoint) — both
    // well defined.
    let mut rng = Rng::new(0xA4);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        let l1 = rng.below(129);
        let l2 = rng.below(257 - l1);
        diff_bufs(
            "alias/merge(dst=src1)",
            || {
                let mut g = Rng::new(seed);
                let a = g.buffer_len(l1);
                let b = g.buffer_len(l2);
                vec![a, b]
            },
            |api, p| unsafe { (api.buffer_merge)(p, p.add(1), p) as i64 },
            true,
        );
    }
    // src1 == src2 == dst with l1 >= l2 keeps the second memcpy disjoint
    // (dst+l1 .. dst+l1+l2 vs dst .. dst+l2).  l1 < l2 would make the C memcpy
    // overlap, which is undefined, so it is deliberately not exercised.
    for l1 in 0..=128usize {
        for l2 in 0..=l1.min(128 - l1 / 2) {
            if l1 + l2 > 256 || l2 > l1 {
                continue;
            }
            diff_bufs(
                "alias/merge(p,p,p)",
                || {
                    let mut b = Rng::new(0xA400 + (l1 * 300 + l2) as u64).buffer_len(l1);
                    b.checksum = checksum(&b.data[..l1]);
                    vec![b]
                },
                move |api, p| unsafe {
                    // Give src2 the shorter length by pointing at a copy.
                    let mut second = *p;
                    second.length = l2;
                    (api.buffer_merge)(p, &second, p) as i64
                },
                true,
            );
        }
    }
}

#[test]
fn alias_copy_dst_equals_src_with_bad_checksum() {
    // buffer_copy(p, p) where the checksum is inconsistent: the warning is
    // emitted and then the checksum is recomputed in place.
    let mut rng = Rng::new(0xA5);
    for _ in 0..ITERS {
        let seed = rng.next_u64();
        diff_bufs(
            "alias/copy(p,p)/bad-cks",
            || {
                let mut g = Rng::new(seed);
                let mut b = g.buffer(0, 256);
                b.checksum = g.next_u32();
                vec![b]
            },
            |api, p| unsafe { (api.buffer_copy)(p, p) as i64 },
            true,
        );
    }
}
