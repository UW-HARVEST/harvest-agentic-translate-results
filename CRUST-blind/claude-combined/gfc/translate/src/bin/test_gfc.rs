#[allow(unused_imports)]
use gfc::gfc::GFC;

// Ground-truth values were obtained by running the original C code
// (c_src/src/gfc.c) compiled with: gcc -I include src/gfc.c tests/gfc_dump.c -lm
// The C executable was then invoked to print encrypt/decrypt mappings for these
// (range, rounds, seed) tuples.

#[test]
fn test_range_1_rounds_1_seed_42() {
    let g = GFC::new(1, 1, 42);
    // From C output: enc[0] = 0, dec[0] = 0
    assert_eq!(g.encrypt(0), 0);
    assert_eq!(g.decrypt(0), 0);
}

#[test]
fn test_range_10_rounds_1_seed_42() {
    let g = GFC::new(10, 1, 42);
    // Ground-truth (encrypt) from C:
    let expected_enc: [u64; 10] = [2, 3, 0, 1, 5, 6, 7, 4, 8, 9];
    for (i, &expected) in expected_enc.iter().enumerate() {
        assert_eq!(
            g.encrypt(i as u64),
            expected,
            "encrypt({}) mismatch (range=10,rounds=1,seed=42)",
            i
        );
    }
    // Roundtrip: decrypt(encrypt(i)) == i
    for i in 0..10u64 {
        assert_eq!(g.decrypt(g.encrypt(i)), i);
    }
    // Direct decryption of a few values (decrypt is the inverse permutation):
    // dec[2] = 0, dec[3] = 1, dec[0] = 2, dec[1] = 3, dec[5] = 4, dec[6] = 5, dec[7] = 6, dec[4] = 7, dec[8] = 8, dec[9] = 9
    let expected_dec_inputs: [u64; 10] = [2, 3, 0, 1, 5, 6, 7, 4, 8, 9];
    let expected_dec_outputs: [u64; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    for k in 0..10 {
        assert_eq!(g.decrypt(expected_dec_inputs[k]), expected_dec_outputs[k]);
    }
}

#[test]
fn test_range_10_rounds_6_seed_42() {
    let g = GFC::new(10, 6, 42);
    let expected_enc: [u64; 10] = [1, 2, 9, 4, 7, 6, 5, 8, 0, 3];
    for (i, &e) in expected_enc.iter().enumerate() {
        assert_eq!(g.encrypt(i as u64), e, "encrypt({}) mismatch", i);
    }
    // Inverse mapping: ciphertext -> plaintext
    // From C: dec[1]=0, dec[2]=1, dec[9]=2, dec[4]=3, dec[7]=4, dec[6]=5, dec[5]=6, dec[8]=7, dec[0]=8, dec[3]=9
    assert_eq!(g.decrypt(1), 0);
    assert_eq!(g.decrypt(2), 1);
    assert_eq!(g.decrypt(9), 2);
    assert_eq!(g.decrypt(4), 3);
    assert_eq!(g.decrypt(7), 4);
    assert_eq!(g.decrypt(6), 5);
    assert_eq!(g.decrypt(5), 6);
    assert_eq!(g.decrypt(8), 7);
    assert_eq!(g.decrypt(0), 8);
    assert_eq!(g.decrypt(3), 9);
}

#[test]
fn test_range_16_rounds_4_seed_7() {
    let g = GFC::new(16, 4, 7);
    let expected_enc: [u64; 16] = [15, 12, 10, 3, 0, 9, 4, 13, 8, 14, 7, 11, 5, 6, 1, 2];
    for (i, &e) in expected_enc.iter().enumerate() {
        assert_eq!(g.encrypt(i as u64), e, "encrypt({}) mismatch", i);
    }
    for i in 0..16u64 {
        assert_eq!(g.decrypt(g.encrypt(i)), i);
    }
    // Specific direct decrypt values from C ground truth:
    assert_eq!(g.decrypt(15), 0);
    assert_eq!(g.decrypt(0), 4);
    assert_eq!(g.decrypt(8), 8);
    assert_eq!(g.decrypt(2), 15);
}

#[test]
fn test_range_100_rounds_6_seed_12345() {
    let g = GFC::new(100, 6, 12345);
    // First several encrypt values from C ground truth:
    let expected_enc: [u64; 100] = [
        73, 16, 19, 27, 10, 4, 60, 47, 14, 2, 40, 90, 26, 54, 89, 30, 43, 41, 72, 46, 36, 58, 70,
        83, 44, 51, 77, 68, 15, 29, 35, 38, 8, 11, 57, 5, 48, 61, 59, 56, 53, 85, 63, 6, 67, 20,
        52, 92, 82, 95, 76, 75, 86, 32, 87, 93, 50, 65, 97, 96, 37, 45, 0, 31, 66, 3, 99, 55, 23,
        94, 74, 64, 25, 1, 34, 84, 24, 18, 80, 21, 98, 91, 17, 62, 71, 9, 69, 28, 12, 39, 13, 22,
        49, 88, 42, 81, 33, 7, 79, 78,
    ];
    for (i, &e) in expected_enc.iter().enumerate() {
        assert_eq!(g.encrypt(i as u64), e, "encrypt({}) mismatch", i);
    }
    // Bijection: every ciphertext in [0, 100) appears exactly once.
    let mut seen = [false; 100];
    for &e in expected_enc.iter() {
        assert!(!seen[e as usize], "ciphertext {} appears twice", e);
        seen[e as usize] = true;
    }
    for (i, s) in seen.iter().enumerate() {
        assert!(*s, "ciphertext {} never appears", i);
    }
    // Roundtrip
    for i in 0..100u64 {
        assert_eq!(g.decrypt(g.encrypt(i)), i);
    }
    // Direct decrypt sanity checks (a handful from the C ground truth):
    assert_eq!(g.decrypt(73), 0);
    assert_eq!(g.decrypt(16), 1);
    assert_eq!(g.decrypt(78), 99);
    assert_eq!(g.decrypt(0), 62);
}

#[test]
fn test_bijection_range_500() {
    // Original C test exercises range=500*1000 with rounds=6. Use a smaller-but-decent
    // range here for quick CI: any (range, rounds) must yield a bijection.
    let range: u64 = 500;
    let rounds: u64 = 6;
    let g = GFC::new(range, rounds, 42);
    let mut counts = vec![0u64; range as usize];
    for i in 0..range {
        let e = g.encrypt(i);
        assert!(e < range, "encrypt produced out-of-range value");
        let d = g.decrypt(e);
        assert_eq!(d, i, "decrypt(encrypt({})) != {}", i, i);
        counts[e as usize] += 1;
    }
    for (i, c) in counts.iter().enumerate() {
        assert_eq!(*c, 1, "ciphertext {} occurs {} times", i, c);
    }
}

#[test]
fn test_range_zero_rounds_zero() {
    // From the original C test suite: range=0, rounds=0 must construct without crashing.
    let _g = GFC::new(0, 0, 42);
}

#[test]
fn test_range_zero_rounds_one() {
    // From the original C test suite: range=0, rounds=1 must construct without crashing.
    let _g = GFC::new(0, 1, 42);
}

#[test]
fn test_range_100_rounds_1_seed_42_bijection() {
    // From original C test: test_range(100, 1) must be a bijection.
    let range: u64 = 100;
    let g = GFC::new(range, 1, 42);
    let mut counts = vec![0u64; range as usize];
    for i in 0..range {
        let e = g.encrypt(i);
        assert!(e < range);
        assert_eq!(g.decrypt(e), i);
        counts[e as usize] += 1;
    }
    for c in counts.iter() {
        assert_eq!(*c, 1);
    }
}

#[test]
fn test_different_seeds_produce_different_permutations() {
    // Sanity check: distinct seeds with the same (range, rounds) should generally
    // not produce identical permutations.
    let g1 = GFC::new(100, 6, 1);
    let g2 = GFC::new(100, 6, 2);
    let mut differ = false;
    for i in 0..100u64 {
        if g1.encrypt(i) != g2.encrypt(i) {
            differ = true;
            break;
        }
    }
    assert!(differ, "different seeds produced identical permutations");
}

fn main() {}
