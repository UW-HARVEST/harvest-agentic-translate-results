//! High-volume randomised passes. `#[ignore]`d so the normal suite stays fast;
//! run them with:
//!
//! ```sh
//! cargo test --test heavy -- --ignored --test-threads=1
//! ```

mod common;
use common::*;

const N: usize = 200_000;

fn heavy_run<F: FnMut(&mut Rng) -> f64>(seed: u64, mut gen: F, ctx: &str) {
    let mut rng = Rng::new(seed);
    // batch in chunks to keep the captured output a sane size
    let chunk = 25_000;
    let mut done = 0;
    while done < N {
        let n = chunk.min(N - done);
        let mut cases = Vec::with_capacity(n);
        for _ in 0..n {
            let b = gen(&mut rng);
            cases.push((
                House::new(rng.next_i32(), rng.next_i32(), b),
                rng.next_i32(),
            ));
        }
        assert_run_batch(&cases, &format!("{} chunk@{}", ctx, done));
        done += n;
    }
}

#[test]
#[ignore]
fn heavy_random_bit_patterns() {
    heavy_run(0x5151_5151_ABCD_0001, |r| r.next_f64_bits(), "heavy bits");
}

#[test]
#[ignore]
fn heavy_random_decimalish() {
    heavy_run(
        0x7777_1111_2222_3333,
        |r| r.next_f64_decimalish(),
        "heavy decimalish",
    );
}

#[test]
#[ignore]
fn heavy_random_mixed() {
    heavy_run(0x1A2B_3C4D_5E6F_7080, |r| r.next_f64_mixed(), "heavy mixed");
}

/// Every `k/10` for k in [-200000, 200000] — exact one-decimal values across
/// four orders of magnitude, i.e. the fast path of the formatter.
#[test]
#[ignore]
fn heavy_all_one_decimal_values() {
    let mut cases = Vec::with_capacity(50_000);
    let mut k: i64 = -200_000;
    while k <= 200_000 {
        cases.push((House::new(1, 2, k as f64 / 10.0), 3));
        if cases.len() == 25_000 {
            assert_run_batch(&cases, "heavy k/10");
            cases.clear();
        }
        k += 1;
    }
    assert_run_batch(&cases, "heavy k/10 tail");
}

/// 50 000 randomised stdin inputs through both executables.
#[test]
#[ignore]
fn heavy_random_stdin() {
    const ALPHABET: &[u8] = b"0123456789+-  \t\n\r\x0b\x0cxXeE.,_abcfz\0\xff\x80";
    let mut rng = Rng::new(0x2468_ACE0_1357_9BDF);
    for i in 0..50_000 {
        let len = rng.below(40) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len() as u64) as usize])
            .collect();
        assert_exe_matches(&input, &format!("heavy stdin #{}", i));
    }
}
