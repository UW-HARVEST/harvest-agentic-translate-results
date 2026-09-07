//! Phase B — valid-path differential tests.
//! One test per row of CONFIGS.md. Every row is driven with many randomized
//! inputs from a fixed seed, and asserted byte-for-byte between the two `.so`s.

mod common;

use common::*;
use std::ffi::c_int;

/// Iterations per row.
const N: usize = 200;

fn seed_for(row: u64) -> Rng {
    Rng::new(SEED ^ row.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Build the backing slots for an aliasing configuration and diff one case.
fn fma_case(rng: &mut Rng, alias: Alias, vals: Vals, len: usize, ctx: &str) {
    let slots: Vec<Vec<i32>> = (0..alias.slots()).map(|_| vals.make(rng, len)).collect();
    diff_fma(alias, &slots, len as c_int, ctx);
}

// --------------------------------------------------------------------------
// Rows 1-5: fma_array, four disjoint buffers, length shapes, random values
// --------------------------------------------------------------------------

#[test]
fn row01_fma_disjoint_len1_random() {
    let mut rng = seed_for(1);
    for _ in 0..N {
        fma_case(&mut rng, Alias::Disjoint, Vals::FullRandom, 1, "row01");
    }
}

#[test]
fn row02_fma_disjoint_len2_random() {
    let mut rng = seed_for(2);
    for _ in 0..N {
        fma_case(&mut rng, Alias::Disjoint, Vals::FullRandom, 2, "row02");
    }
}

#[test]
fn row03_fma_disjoint_len3to8_random() {
    let mut rng = seed_for(3);
    for _ in 0..N {
        let len = rng.range(3, 8) as usize;
        fma_case(&mut rng, Alias::Disjoint, Vals::FullRandom, len, "row03");
    }
}

#[test]
fn row04_fma_disjoint_many_random() {
    let mut rng = seed_for(4);
    for _ in 0..N {
        let len = rng.range(64, 512) as usize;
        fma_case(&mut rng, Alias::Disjoint, Vals::FullRandom, len, "row04");
    }
}

#[test]
fn row05_fma_disjoint_len1024_random() {
    let mut rng = seed_for(5);
    for _ in 0..64 {
        fma_case(&mut rng, Alias::Disjoint, Vals::FullRandom, 1024, "row05");
    }
}

// --------------------------------------------------------------------------
// Rows 6-9: fma_array value classes
// --------------------------------------------------------------------------

#[test]
fn row06_fma_disjoint_zeros() {
    let mut rng = seed_for(6);
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        fma_case(&mut rng, Alias::Disjoint, Vals::Zeros, len, "row06");
    }
}

#[test]
fn row07_fma_disjoint_small_positive() {
    let mut rng = seed_for(7);
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        fma_case(&mut rng, Alias::Disjoint, Vals::SmallPos, len, "row07");
    }
}

#[test]
fn row08_fma_disjoint_small_negative() {
    let mut rng = seed_for(8);
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        fma_case(&mut rng, Alias::Disjoint, Vals::SmallNeg, len, "row08");
    }
}

#[test]
fn row09_fma_disjoint_extremes() {
    let mut rng = seed_for(9);
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        fma_case(&mut rng, Alias::Disjoint, Vals::Extremes, len, "row09");
    }
    // Exhaustive cross-product of the extremes over all three operands, so
    // every multiply/add wrap combination is covered, not just random draws.
    let n = EXTREMES.len();
    let mut m1 = Vec::new();
    let mut m2 = Vec::new();
    let mut ad = Vec::new();
    for a in EXTREMES {
        for b in EXTREMES {
            for c in EXTREMES {
                m1.push(a);
                m2.push(b);
                ad.push(c);
            }
        }
    }
    let len = m1.len();
    assert_eq!(len, n * n * n);
    let slots = vec![vec![0i32; len], m1, m2, ad];
    diff_fma(Alias::Disjoint, &slots, len as c_int, "row09-exhaustive");
}

// --------------------------------------------------------------------------
// Rows 10-14: fma_array aliasing configurations
// --------------------------------------------------------------------------

#[test]
fn row10_fma_mul1_eq_mul2_squaring() {
    let mut rng = seed_for(10);
    for _ in 0..N {
        let len = rng.range(1, 128) as usize;
        fma_case(&mut rng, Alias::Mul1EqMul2, Vals::FullRandom, len, "row10");
    }
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        fma_case(&mut rng, Alias::Mul1EqMul2, Vals::Extremes, len, "row10x");
    }
}

#[test]
fn row11_fma_out_eq_mul1_inplace() {
    let mut rng = seed_for(11);
    for _ in 0..N {
        let len = rng.range(1, 128) as usize;
        fma_case(&mut rng, Alias::OutEqMul1, Vals::FullRandom, len, "row11");
    }
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        fma_case(&mut rng, Alias::OutEqMul1, Vals::Extremes, len, "row11x");
    }
}

#[test]
fn row12_fma_out_eq_add() {
    let mut rng = seed_for(12);
    for _ in 0..N {
        let len = rng.range(1, 128) as usize;
        fma_case(&mut rng, Alias::OutEqAdd, Vals::FullRandom, len, "row12");
    }
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        fma_case(&mut rng, Alias::OutEqAdd, Vals::Extremes, len, "row12x");
    }
}

#[test]
fn row13_fma_all_four_aliased_random() {
    let mut rng = seed_for(13);
    for _ in 0..N {
        let len = rng.range(1, 256) as usize;
        fma_case(&mut rng, Alias::AllSame, Vals::FullRandom, len, "row13");
    }
    // Every length shape too.
    let mut rng2 = seed_for(1300);
    for len in len_shapes(&mut rng2) {
        fma_case(&mut rng2, Alias::AllSame, Vals::FullRandom, len, "row13-shapes");
    }
}

#[test]
fn row14_fma_all_four_aliased_extremes() {
    let mut rng = seed_for(14);
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        fma_case(&mut rng, Alias::AllSame, Vals::Extremes, len, "row14");
    }
    // Exhaustive: every extreme value as the single aliased element.
    for v in EXTREMES {
        diff_fma(Alias::AllSame, &[vec![v]], 1, "row14-exhaustive");
    }
    // And all of them at once in one buffer.
    diff_fma(
        Alias::AllSame,
        &[EXTREMES.to_vec()],
        EXTREMES.len() as c_int,
        "row14-all",
    );
}

// --------------------------------------------------------------------------
// Rows 15-19: driver length shapes, stdout compared byte-for-byte
// --------------------------------------------------------------------------

fn driver_case(rng: &mut Rng, vals: Vals, len: usize, ctx: &str) {
    let data = vals.make(rng, len);
    let out = diff_driver(&data, len as c_int, ctx);
    // Sanity: the C prints exactly one line per element.
    assert_eq!(
        out.iter().filter(|&&b| b == b'\n').count(),
        len,
        "[{ctx}] expected {len} lines, got {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn row15_driver_len1_random() {
    let mut rng = seed_for(15);
    for _ in 0..N {
        driver_case(&mut rng, Vals::FullRandom, 1, "row15");
    }
}

#[test]
fn row16_driver_len2_random() {
    let mut rng = seed_for(16);
    for _ in 0..N {
        driver_case(&mut rng, Vals::FullRandom, 2, "row16");
    }
}

#[test]
fn row17_driver_len3to8_random() {
    let mut rng = seed_for(17);
    for _ in 0..N {
        let len = rng.range(3, 8) as usize;
        driver_case(&mut rng, Vals::FullRandom, len, "row17");
    }
}

#[test]
fn row18_driver_many_random() {
    let mut rng = seed_for(18);
    for _ in 0..80 {
        let len = rng.range(64, 512) as usize;
        driver_case(&mut rng, Vals::FullRandom, len, "row18");
    }
}

#[test]
fn row19_driver_len1024_random() {
    let mut rng = seed_for(19);
    for _ in 0..24 {
        driver_case(&mut rng, Vals::FullRandom, 1024, "row19");
    }
}

// --------------------------------------------------------------------------
// Rows 20-23: driver value classes
// --------------------------------------------------------------------------

#[test]
fn row20_driver_zeros() {
    let mut rng = seed_for(20);
    for _ in 0..64 {
        let len = rng.range(1, 64) as usize;
        let data = vec![0i32; len];
        let out = diff_driver(&data, len as c_int, "row20");
        assert_eq!(out, "0\n".repeat(len).into_bytes(), "row20 exact stdout");
    }
}

#[test]
fn row21_driver_small_positive() {
    let mut rng = seed_for(21);
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        driver_case(&mut rng, Vals::SmallPos, len, "row21");
    }
}

#[test]
fn row22_driver_negative_formatting() {
    let mut rng = seed_for(22);
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        driver_case(&mut rng, Vals::SmallNeg, len, "row22");
    }
}

#[test]
fn row23_driver_extremes() {
    let mut rng = seed_for(23);
    for _ in 0..N {
        let len = rng.range(1, 64) as usize;
        driver_case(&mut rng, Vals::Extremes, len, "row23");
    }
    // Every extreme value on its own, and all of them together.
    for v in EXTREMES {
        diff_driver(&[v], 1, "row23-single");
    }
    diff_driver(&EXTREMES, EXTREMES.len() as c_int, "row23-all");
    // INT_MIN is the one value with no positive counterpart; `%d` must print it.
    let out = diff_driver(&[i32::MIN], 1, "row23-intmin");
    // INT_MIN*INT_MIN + INT_MIN wraps to INT_MIN (2^31*2^31 = 0 mod 2^32).
    assert_eq!(out, b"-2147483648\n".to_vec(), "row23 INT_MIN stdout");
}

// --------------------------------------------------------------------------
// Row 24: composed-pipeline cross-check
// --------------------------------------------------------------------------

/// `driver`'s printed lines must equal what `fma_array(buf,buf,buf,buf,len)`
/// leaves in `buf` — this checks the whole `driver -> inner -> fma_array ->
/// printf` pipeline, not just each wrapper in isolation, in *both* libraries.
#[test]
fn row24_driver_matches_fma_pipeline() {
    let p = pair();
    let mut rng = seed_for(24);
    for _ in 0..120 {
        let len = rng.range(1, 200) as usize;
        let vals = rng.pick(&[
            Vals::FullRandom,
            Vals::Extremes,
            Vals::SmallPos,
            Vals::SmallNeg,
            Vals::Zeros,
        ]);
        let data = vals.make(&mut rng, len);

        // Ground truth from the C fma_array with all four pointers aliased.
        let c_fma = run_fma(&p.c, Alias::AllSame, &[data.clone()], len as c_int);
        let rs_fma = run_fma(&p.rs, Alias::AllSame, &[data.clone()], len as c_int);
        assert_eq!(c_fma, rs_fma, "row24 fma stage diverged len={len}");

        let expected: Vec<u8> = c_fma[0]
            .iter()
            .map(|v| format!("{v}\n"))
            .collect::<String>()
            .into_bytes();

        let got = diff_driver(&data, len as c_int, "row24");
        assert_eq!(
            String::from_utf8_lossy(&got),
            String::from_utf8_lossy(&expected),
            "row24 driver stdout != fma_array result, len={len} vals={vals:?}"
        );
    }
}
