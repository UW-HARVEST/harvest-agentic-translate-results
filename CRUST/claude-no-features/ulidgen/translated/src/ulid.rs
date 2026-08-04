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
    let mut same = true;
    ulid[26] = '\0';

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() as u64) / 1_000_000;

    // Encode the 48-bit timestamp into the first 10 characters (Crockford base32)
    for i in (0..10).rev() {
        let c = B32_ALPHABET[(t % 32) as usize] as char;
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        // increment random part in place (positions 10..26 -> 16 chars)
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

        // find current char in alphabet and replace with next one
        let cur = ulid[10 + i as usize];
        if let Some(idx) = B32_ALPHABET.iter().position(|&c| c as char == cur) {
            if idx + 1 < B32_ALPHABET.len() {
                ulid[10 + i as usize] = B32_ALPHABET[idx + 1] as char;
                return;
            }
        }
        // else: invalid char encountered -> fall through to randomize
    }

    let mut rnd = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut rnd);
    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}
