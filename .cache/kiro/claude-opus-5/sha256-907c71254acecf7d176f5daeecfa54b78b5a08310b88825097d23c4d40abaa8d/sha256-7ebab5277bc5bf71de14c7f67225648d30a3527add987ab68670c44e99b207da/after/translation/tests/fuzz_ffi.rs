//! Deterministic randomised differential testing of `driver`, for input shapes
//! longer than the exhaustive sweeps in `driver_ffi.rs` reach.

mod common;

use common::{check_driver, check_run};

/// xorshift64*, so the corpus is reproducible without a dependency.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Bytes that steer `strtol`: whitespace, signs, digits, and a few rejects.
const ALPHABET: &[u8] = b"0123456789+-  \t\n\r\x0b\x0cabcxXeE.,_/:";

#[test]
fn driver_matches_c_on_random_strings() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);

    for case in 0..3000 {
        let len = 1 + rng.below(12);
        let mut s = Vec::with_capacity(len);
        while s.len() < len {
            let b = ALPHABET[rng.below(ALPHABET.len())];
            s.push(b);
        }
        check_driver(
            &s,
            &format!("driver({:?}) [random #{case}]", String::from_utf8_lossy(&s)),
        );
    }
}

/// Random decimal literals across the whole `i64` range and beyond, where the
/// `int`-range check and the `ERANGE` check both matter.
#[test]
fn driver_matches_c_on_random_numbers() {
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);

    for case in 0..1500 {
        let raw = rng.next_u64();
        let s = match case % 6 {
            0 => format!("{}", raw as i64),
            1 => format!("{raw}"),
            2 => format!("{}", (raw as i32) as i64),
            3 => format!("-{raw}"),
            4 => format!("+{}", raw % 100_000),
            _ => format!("{}{}", raw, raw),
        };
        check_driver(
            s.as_bytes(),
            &format!("driver({s:?}) [random number #{case}]"),
        );
    }
}

/// Random `run` arguments, driving `bedrooms` all over the `int` range.
#[test]
fn run_matches_c_on_random_ints() {
    let mut rng = Rng(0x0123_4567_89AB_CDEF);

    for case in 0..1500 {
        let v = rng.next_u64() as i32;
        check_run(v, &format!("run({v}) [random #{case}]"));
    }
}
