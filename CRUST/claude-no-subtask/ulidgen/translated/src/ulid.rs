// Import statements
#[allow(unused_imports)]
use crate::{ulidgen};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread::sleep;
use std::time::Duration;

// Constants
pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    // null-terminate position 26 (matching C's ulid[26] = 0)
    ulid[26] = '\0';

    // Compute current time in milliseconds since UNIX epoch
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() as u64) / 1_000_000;

    // Encode the timestamp into the first 10 characters (high to low)
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
        // Increment random part in place.
        // buf corresponds to ulid[10..26], 16 chars.
        let mut i: i32 = 15;
        while i >= 0 && ulid[10 + i as usize] == 'Z' {
            ulid[10 + i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            // Random portion wrapped completely - sleep a bit and retry
            sleep(Duration::new(0, 1_234_567));
            ulidgen_r(ulid);
            return;
        }

        // Find current char in alphabet and bump to the next
        let cur = ulid[10 + i as usize];
        if let Some(pos) = B32_ALPHABET.iter().position(|&b| b as char == cur) {
            // strchr returned non-null, so set buf[i] to next character
            ulid[10 + i as usize] = B32_ALPHABET[pos + 1] as char;
            return;
        }
        // Otherwise fall through and randomize (matches "else randomize again")
    }

    // Generate 16 random bytes and encode them
    let mut rnd = [0u8; 16];
    getrandom::getrandom(&mut rnd).expect("failed to obtain entropy");

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}
