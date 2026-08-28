//! Heavy exhaustive sweeps. `#[ignore]`d so the normal suite stays fast; run with
//!
//! ```sh
//! cargo test --release --offline --test heavy_exhaustive -- --ignored --nocapture
//! ```

mod common;

use common::*;

/// All 15^5 = 759 375 strings of length 5 over the accepted alphabet.
#[test]
#[ignore = "heavy: 759375 differential calls"]
fn all_accepted_strings_of_length_five() {
    let alpha = ACCEPTED;
    let n = alpha.len();
    let total = n.pow(5);
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for i in 0..total {
        let mut bytes = [0u8; 5];
        let mut k = i;
        for slot in bytes.iter_mut() {
            *slot = alpha[k % n];
            k /= n;
        }
        let case = Case::from_bytes(&bytes);
        let c = observe_c(&case);
        let r = observe_rust(&case);
        assert_eq!(
            c,
            r,
            "divergence at {:?}",
            String::from_utf8_lossy(&bytes)
        );
        if c.ret == 0 {
            rejected += 1
        } else {
            accepted += 1
        }
    }
    assert_eq!(accepted + rejected, total);
    println!("length-5 sweep: {accepted} accepted, {rejected} rejected, 0 divergences");
    assert!(accepted > 0 && rejected > 0, "sweep must hit both outcomes");
}

/// All 7^7 = 823 543 strings of length 7 over one representative byte per switch
/// arm (digit, both signs, both exponent letters, the decimal point, and a
/// `default:` byte), each with a rotating offset.
#[test]
#[ignore = "heavy: 823543 differential calls"]
fn representative_alphabet_length_seven() {
    let alpha: &[u8] = b"7.+-eEz";
    let n = alpha.len();
    let total = n.pow(7);
    for i in 0..total {
        let mut bytes = [0u8; 7];
        let mut k = i;
        for slot in bytes.iter_mut() {
            *slot = alpha[k % n];
            k /= n;
        }
        let offset = i % 8;
        let case = Case::from_bytes(&bytes).length(7).offset(offset);
        let c = observe_c(&case);
        let r = observe_rust(&case);
        assert_eq!(
            c,
            r,
            "divergence at {:?} offset={offset}",
            String::from_utf8_lossy(&bytes)
        );
    }
    println!("length-7 representative sweep: {total} cases, 0 divergences");
}

/// Two million randomized cases across every axis.
#[test]
#[ignore = "heavy: 2000000 differential calls"]
fn two_million_random_cases() {
    let mut rng = Rng::new(0xFEED_FACE_CAFE_BEEF);
    let mut accepted = 0usize;
    for _ in 0..2_000_000 {
        let n = rng.below(24) as usize;
        let mut bytes = Vec::with_capacity(n);
        for _ in 0..n {
            match rng.below(16) {
                0 => bytes.push(rng.next_u64() as u8),
                1 => bytes.push(*rng.pick(b"xXpPnaifNAIF_,' \t\r\n")),
                _ => bytes.push(*rng.pick(ACCEPTED)),
            }
        }
        let length = if rng.below(8) == 0 {
            rng.below((n + 1) as u64) as usize
        } else {
            n
        };
        let offset = match rng.below(8) {
            0 => rng.below((n + 2) as u64) as usize,
            1 => length,
            _ => 0,
        };
        let mut case = Case::from_bytes(&bytes).length(length).offset(offset);
        case.depth = rng.next_u64() as usize;
        case.item_type = rng.next_u64() as i32;
        case.item_valueint = rng.next_u64() as i32;
        case.item_valuedouble_bits = rng.next_u64();
        let o = diff(&case);
        if o.ret != 0 {
            accepted += 1;
        }
    }
    println!("2M random cases: {accepted} accepted, 0 divergences");
    assert!(accepted > 100_000, "sweep degenerated: only {accepted} accepted");
}

/// Exhaustive over all 2^16 two-byte inputs (every possible byte pair), at every
/// offset and length.
#[test]
#[ignore = "heavy: 65536 * 12 differential calls"]
fn all_byte_pairs_all_offsets_all_lengths() {
    for hi in 0u16..256 {
        for lo in 0u16..256 {
            let bytes = [hi as u8, lo as u8];
            for length in 0..=3usize {
                for offset in 0..=2usize {
                    let case = Case::from_bytes(&bytes).length(length).offset(offset);
                    let c = observe_c(&case);
                    let r = observe_rust(&case);
                    assert_eq!(
                        c, r,
                        "divergence at [{hi:#04x},{lo:#04x}] length={length} offset={offset}"
                    );
                }
            }
        }
    }
    println!("all 65536 byte pairs x 4 lengths x 3 offsets: 0 divergences");
}
