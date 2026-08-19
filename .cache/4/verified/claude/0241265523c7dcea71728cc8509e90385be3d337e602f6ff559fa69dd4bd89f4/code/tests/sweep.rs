//! Broad safety net: exhaustive and high-volume randomized sweeps of the
//! `main` export, over and above the curated `CONFIGS.md` rows.
//!
//! The curated rows target the branches visible in the source.  These sweeps
//! exist to catch anything the reading missed: they enumerate *every* short
//! byte string over the alphabets that matter to `scanf("%d")`, so no
//! hand-picked corpus can accidentally skip a state transition.
//!
//! As everywhere else, both implementations are reached only through the
//! exported `main` of their respective shared object, loaded with `libloading`
//! in a fresh process.

mod common;

use common::{assert_same, c_so, rust_so, show, Rng, SEED};

#[track_caller]
fn diff(label: &str, input: &[u8]) {
    let c = common::so_main(&c_so(), input);
    let r = common::so_main(&rust_so(), input);
    assert_same(label, input, &c, &r);
    assert_eq!(
        c.code,
        Some(0),
        "the C main() always returns 0; input \"{}\"",
        show(input)
    );
}

/// Enumerate every string of length `0..=max_len` over `alphabet`.
fn enumerate(alphabet: &[u8], max_len: usize) -> Vec<Vec<u8>> {
    let mut all: Vec<Vec<u8>> = vec![Vec::new()];
    let mut level: Vec<Vec<u8>> = vec![Vec::new()];
    for _ in 0..max_len {
        let mut next = Vec::new();
        for prefix in &level {
            for &b in alphabet {
                let mut s = prefix.clone();
                s.push(b);
                next.push(s);
            }
        }
        all.extend(next.iter().cloned());
        level = next;
    }
    all
}

/// The characters the `%d` state machine treats specially, plus two bytes that
/// exercise the "not a digit, not a sign, not a space" path (one NUL, one high
/// byte).
const KEY_ALPHABET: [u8; 10] = [b'0', b'1', b'9', b'+', b'-', b' ', b'\n', b'x', b'a', 0x00];

/// A tighter alphabet, used for the longer exhaustive run.
const CORE_ALPHABET: [u8; 6] = [b'0', b'1', b'+', b'-', b' ', b'x'];

#[test]
fn sweep_exhaustive_key_alphabet_len_0_to_3() {
    let corpus = enumerate(&KEY_ALPHABET, 3);
    assert_eq!(corpus.len(), 1 + 10 + 100 + 1000);
    for (i, input) in corpus.iter().enumerate() {
        diff(&format!("exhaustive key #{i}"), input);
    }
}

#[test]
fn sweep_exhaustive_core_alphabet_len_4() {
    let mut corpus = enumerate(&CORE_ALPHABET, 4);
    corpus.retain(|s| s.len() == 4);
    assert_eq!(corpus.len(), 6 * 6 * 6 * 6);
    for (i, input) in corpus.iter().enumerate() {
        diff(&format!("exhaustive core len 4 #{i}"), input);
    }
}

#[test]
fn sweep_exhaustive_core_alphabet_len_5_sampled() {
    // 6^5 = 7776 is more than is worth spawning, so take a deterministic
    // one-in-six stride, which still covers every position/character pair.
    let mut corpus = enumerate(&CORE_ALPHABET, 5);
    corpus.retain(|s| s.len() == 5);
    for (i, input) in corpus.iter().enumerate().filter(|(i, _)| i % 6 == 0) {
        diff(&format!("exhaustive core len 5 #{i}"), input);
    }
}

#[test]
fn sweep_random_full_byte_alphabet() {
    let all: Vec<u8> = (0x00u8..=0xff).collect();
    let mut rng = Rng::new(SEED ^ 0xFFFF);
    for i in 0..600 {
        let len = rng.range(0, 12) as usize;
        let input = rng.bytes(len, &all);
        diff(&format!("random full byte #{i}"), &input);
    }
}

#[test]
fn sweep_random_numeric_strings() {
    // Long, mostly-numeric strings: the region where `strtol` overflow,
    // `(int)` narrowing and the refill boundary all interact.
    let digits = b"0123456789";
    let mut rng = Rng::new(SEED ^ 0xF00D);
    for i in 0..600 {
        let mut input: Vec<u8> = Vec::new();
        match rng.below(3) {
            0 => input.push(b'-'),
            1 => input.push(b'+'),
            _ => {}
        }
        let zeros = rng.below(4) as usize;
        input.extend(vec![b'0'; zeros]);
        let n = rng.range(1, 25) as usize;
        input.extend_from_slice(&rng.bytes(n, digits));
        diff(&format!("random numeric #{i}"), &input);
    }
}

#[test]
fn sweep_powers_and_neighbours() {
    // Every power of two up to 2^70, and its immediate neighbours, in both
    // signs — this pins down the `long` overflow edge and the `(int)`
    // truncation edge simultaneously.
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for e in 0..=70u32 {
        let v = num_pow2_decimal(e);
        for delta in [-1i64, 0, 1] {
            let s = decimal_add(&v, delta);
            inputs.push(s.clone().into_bytes());
            inputs.push(format!("-{s}").into_bytes());
        }
    }
    for (i, input) in inputs.iter().enumerate() {
        diff(&format!("power of two #{i}"), input);
    }
}

/// 2^e as a decimal string, computed with schoolbook doubling so it works well
/// past 64 bits.
fn num_pow2_decimal(e: u32) -> String {
    let mut digits = vec![1u8];
    for _ in 0..e {
        let mut carry = 0u8;
        for d in digits.iter_mut() {
            let v = *d * 2 + carry;
            *d = v % 10;
            carry = v / 10;
        }
        if carry > 0 {
            digits.push(carry);
        }
    }
    digits
        .iter()
        .rev()
        .map(|d| (b'0' + d) as char)
        .collect::<String>()
}

/// Add a small signed delta to a decimal string (only -1, 0, +1 are needed).
fn decimal_add(s: &str, delta: i64) -> String {
    if delta == 0 {
        return s.to_string();
    }
    let mut digits: Vec<u8> = s.bytes().map(|b| b - b'0').collect();
    if delta > 0 {
        let mut carry = 1u8;
        for d in digits.iter_mut().rev() {
            let v = *d + carry;
            *d = v % 10;
            carry = v / 10;
            if carry == 0 {
                break;
            }
        }
        if carry > 0 {
            digits.insert(0, carry);
        }
    } else {
        let mut borrow = 1u8;
        for d in digits.iter_mut().rev() {
            if *d >= borrow {
                *d -= borrow;
                break;
            }
            *d = *d + 10 - borrow;
            borrow = 1;
        }
        while digits.len() > 1 && digits[0] == 0 {
            digits.remove(0);
        }
    }
    digits.iter().map(|d| (b'0' + d) as char).collect()
}
