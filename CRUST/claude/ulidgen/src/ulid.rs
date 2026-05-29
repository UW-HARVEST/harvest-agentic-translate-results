// Import statements
use crate::{ulidgen};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::time::Duration;
use rand::RngCore;

// Constants
pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    // Null terminator (matches C: ulid[26] = 0)
    ulid[26] = '\0';

    // Compute current timestamp in milliseconds since the UNIX epoch
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let mut t: u64 = now.as_secs()
        .saturating_mul(1000)
        .saturating_add((now.subsec_nanos() / 1_000_000) as u64);

    // Encode the timestamp into the first 10 chars (base32, big-endian).
    // Track whether the timestamp portion changed compared to the previous call.
    let mut same = true;
    for i in (0..10).rev() {
        let new_char = B32_ALPHABET[(t % 32) as usize] as char;
        if ulid[i] != new_char {
            ulid[i] = new_char;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Increment random part in place (positions 10..26, 16 chars).
        let mut i: i32 = 15;
        while i >= 0 && ulid[10 + i as usize] == 'Z' {
            ulid[10 + i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            // restart 1ms + a bit later
            thread::sleep(Duration::new(0, 1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let pos = 10 + i as usize;
        if let Some(idx) = B32_ALPHABET
            .iter()
            .position(|&c| c as char == ulid[pos])
        {
            // idx < 31 because ulid[pos] != 'Z'
            ulid[pos] = B32_ALPHABET[idx + 1] as char;
            return;
        }
        // else: invalid char in current ULID, fall through to randomize
    }

    // Generate a fresh 16-byte random buffer and encode each byte as base32
    let mut rnd = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut rnd);

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}
