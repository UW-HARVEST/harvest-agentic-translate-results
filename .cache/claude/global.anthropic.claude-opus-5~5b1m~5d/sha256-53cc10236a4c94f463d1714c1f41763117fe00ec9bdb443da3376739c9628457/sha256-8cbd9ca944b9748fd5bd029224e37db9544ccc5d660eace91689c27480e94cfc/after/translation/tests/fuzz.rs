//! Heavy randomized sweeps. These are `#[ignore]`d so the normal `cargo test`
//! run stays fast; run them with
//! `cargo test --offline --test fuzz -- --ignored --test-threads=1`.

mod common;

use std::sync::Mutex;

use common::*;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

#[test]
#[ignore = "heavy"]
fn fuzz_doubleneg_stdout() {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnDoubleneg>(SYM_DOUBLENEG) };
    let mut rng = Rng::new(0xF0F0_1234);

    for iter in 0..20_000u32 {
        // Mix full-range draws with small ones and with the extremes.
        let pick = |rng: &mut Rng| -> i32 {
            match rng.next_u64() & 7 {
                0 => i32::MIN,
                1 => i32::MAX,
                2 => 0,
                3 => (rng.next_u32() % 512) as i32 - 256,
                4 => (rng.next_u32() % 65536) as i32 - 32768,
                _ => rng.next_i32(),
            }
        };
        let (a, b, cc, d) = (pick(&mut rng), pick(&mut rng), pick(&mut rng), pick(&mut rng));
        let (cv, cout) = capture_stdout(|| unsafe { c(a, b, cc, d) });
        let (rv, rout) = capture_stdout(|| unsafe { r(a, b, cc, d) });
        assert_eq!(cv, rv, "iter {iter}: doubleneg({a},{b},{cc},{d}) return");
        assert!(
            cout == rout,
            "iter {iter}: doubleneg({a},{b},{cc},{d})\n{}",
            diff_report("fuzz", &cout, &rout)
        );
    }
}

#[test]
#[ignore = "heavy"]
fn fuzz_convert_all_bit_patterns() {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnConvertDoubleToInt>(SYM_CONVERT) };
    let mut rng = Rng::new(0xF00D_5678);
    for _ in 0..2_000_000 {
        let v = rng.next_f64_bits();
        let (cv, rv) = unsafe { (c(v), r(v)) };
        assert_eq!(cv, rv, "convert_double_to_int({})", show_f64(v));
    }
    // Dense sweep right around the two range boundaries, where cvttsd2si's
    // "integer indefinite" behaviour switches on.
    for base in [2147483648.0f64, -2147483648.0f64, 2147483647.0, -2147483649.0] {
        let mut v = base;
        for _ in 0..4096 {
            v = f64::from_bits(v.to_bits() - 1);
        }
        for _ in 0..8192 {
            let (cv, rv) = unsafe { (c(v), r(v)) };
            assert_eq!(cv, rv, "boundary convert_double_to_int({})", show_f64(v));
            v = f64::from_bits(v.to_bits() + 1);
        }
    }
}

#[test]
#[ignore = "heavy"]
fn fuzz_calc_and_buffers() {
    let p = Pair::load();
    let (ccalc, rcalc) = unsafe { p.both::<FnCalculateWithDoubles>(SYM_CALC) };
    let (ccre, rcre) = unsafe { p.both::<FnCreateNumericBuffer>(SYM_CREATE) };
    let (cfind, rfind) = unsafe { p.both::<FnFindValueInBuffer>(SYM_FIND) };
    let mut rng = Rng::new(0xFACE_9ABC);

    for _ in 0..500_000 {
        let (a, b, cc) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        let (cv, rv) = unsafe { (ccalc(a, b, cc), rcalc(a, b, cc)) };
        assert!(
            same_f64_bits(cv, rv),
            "calculate_with_doubles({a}, {b}, {cc}) => C {} vs Rust {}",
            show_f64(cv),
            show_f64(rv)
        );
    }

    for _ in 0..40_000 {
        let cap = 2048usize;
        let size = rng.below(cap as u64 + 1) as i32;
        let seed = rng.next_i32();
        let mut cb = vec![0x5Au8 as i8; cap];
        let mut rb = vec![0x5Au8 as i8; cap];
        unsafe {
            ccre(cb.as_mut_ptr(), size, seed);
            rcre(rb.as_mut_ptr(), size, seed);
        }
        assert_eq!(cb, rb, "create_numeric_buffer(size={size}, seed={seed})");
        for _ in 0..3 {
            let n = rng.below(cap as u64 + 1) as usize;
            let needle = rng.next_i32();
            let (fa, fb) = unsafe { (cfind(cb.as_ptr(), n, needle), rfind(cb.as_ptr(), n, needle)) };
            assert_eq!(fa, fb, "find_value_in_buffer(len={n}, needle={needle})");
        }
    }
}
