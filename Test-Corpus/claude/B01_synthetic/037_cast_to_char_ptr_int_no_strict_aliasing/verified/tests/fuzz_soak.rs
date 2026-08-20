//! Long-running randomized soak tests (`cargo test -- --ignored`).
//!
//! Same differential assertions as the Phase B/C suites, just with a lot more
//! randomized inputs.  Kept out of the default run because they take ~1 minute.

mod common;

use common::*;

/// 200 000 random `i32` through the `driver` export.
#[test]
#[ignore]
fn soak_driver_random() {
    let mut rng = Rng::with_seed(0x9E37_79B9_7F4A_7C15);
    for round in 0..40 {
        let xs: Vec<i32> = (0..5_000).map(|_| rng.next_i32()).collect();
        assert_driver_batch_eq(&format!("soak driver round {round}"), &xs);
    }
}

/// 20 000 random byte blobs through the `main` export.
#[test]
#[ignore]
fn soak_main_random_blobs() {
    const ALPHABETS: [&[u8]; 4] = [
        b"0123456789+- \t\n\r\x0b\x0cabxXzZ.,;:/*#\0\x01\xff\x80eE",
        b"0123456789",
        b"0123456789+-  \t\n",
        b"0123456789999999999999999999999-+",
    ];
    let mut rng = Rng::with_seed(0xBF58_476D_1CE4_E5B9);
    for i in 0..20_000 {
        let alpha = ALPHABETS[i % ALPHABETS.len()];
        let n = rng.below(72) as usize;
        let input: Vec<u8> = (0..n).map(|_| *rng.pick(alpha)).collect();
        let kind = if i % 7 == 0 { Stdin::Pipe } else { Stdin::File };
        assert_main_eq(&input, kind);
    }
}

/// 4 000 random byte blobs, comparing stdout *and* the stdin bytes left behind.
#[test]
#[ignore]
fn soak_main_stdin_state() {
    let mut rng = Rng::with_seed(0x94D0_49BB_1331_11EB);
    for i in 0..4_000 {
        let n = rng.below(96) as usize;
        let input: Vec<u8> = (0..n)
            .map(|_| *rng.pick(b"0123456789+- \t\n\r\x0b\x0cabz.\0"))
            .collect();
        assert_main_drain_eq(&input, if i % 3 == 0 { Stdin::Pipe } else { Stdin::File });
        if i % 5 == 0 {
            assert_main_n_eq(&input, Stdin::File, 1 + (i % 4));
        }
    }
}

/// 4 000 random byte blobs end to end through both executables.
#[test]
#[ignore]
fn soak_exe_random_blobs() {
    let mut rng = Rng::with_seed(0x2545_F491_4F6C_DD1D);
    for i in 0..4_000 {
        let n = rng.below(64) as usize;
        let input: Vec<u8> = (0..n)
            .map(|_| *rng.pick(b"0123456789+- \t\n\r\x0b\x0cabz.\0\xff"))
            .collect();
        if i % 2 == 0 {
            assert_exe_eq(&input);
        } else {
            assert_exe_file_stdin_eq(&input);
        }
    }
}

/// Numbers of every digit length 1..=400, both signs, with a random terminator.
#[test]
#[ignore]
fn soak_digit_lengths() {
    let mut rng = Rng::with_seed(0x243F_6A88_85A3_08D3);
    for len in 1..=400usize {
        for sign in ["", "-", "+"] {
            let mut s = String::from(sign);
            s.push(char::from(b'1' + rng.below(9) as u8));
            for _ in 1..len {
                s.push(char::from(b'0' + rng.below(10) as u8));
            }
            s.push(char::from(*rng.pick(b" \n\ta.-+0")));
            assert_main_eq(s.as_bytes(), Stdin::File);
        }
    }
}
