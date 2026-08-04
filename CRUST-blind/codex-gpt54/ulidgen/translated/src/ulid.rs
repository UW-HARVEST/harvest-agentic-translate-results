// Import statements
use crate::{ulidgen};
use rand::RngCore;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// Constants
pub const ULID_LENGTH: usize = 27;
// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    let _ = ulidgen::main as fn(i32, &[&str]) -> i32;

    const B32_ALPHABET: [char; 32] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G',
        'H', 'J', 'K', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'V', 'W', 'X', 'Y', 'Z',
    ];

    ulid[26] = '\0';

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let mut timestamp = duration
        .as_secs()
        .saturating_mul(1_000)
        .saturating_add(u64::from(duration.subsec_millis()));

    let mut same = true;
    for idx in (0..10).rev() {
        let ch = B32_ALPHABET[(timestamp % 32) as usize];
        if ulid[idx] != ch {
            ulid[idx] = ch;
            same = false;
        }
        timestamp /= 32;
    }

    if same {
        let mut idx = 25usize;
        while ulid[idx] == 'Z' {
            ulid[idx] = '0';
            if idx == 10 {
                break;
            }
            idx -= 1;
        }

        if idx < 10 || (idx == 10 && ulid[idx] == '0') {
            thread::sleep(Duration::new(0, 1_234_567));
            ulidgen_r(ulid);
            return;
        }

        if let Some(pos) = B32_ALPHABET.iter().position(|&candidate| candidate == ulid[idx]) {
            ulid[idx] = B32_ALPHABET[pos + 1];
            return;
        }
    }

    let mut rnd = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut rnd);
    for (slot, byte) in ulid[10..26].iter_mut().zip(rnd.iter()) {
        *slot = B32_ALPHABET[(byte % 32) as usize];
    }
}
