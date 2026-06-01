// Import statements
use crate::{ulidgen};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread::sleep;
use std::time::Duration;
use rand::RngCore;
// Constants
pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    let mut same = true;

    ulid[26] = '\0';

    // Get current time in milliseconds since UNIX_EPOCH
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() as u64) / 1_000_000;

    // Fill timestamp portion (first 10 chars), encoding from least significant
    // to most significant (i goes 9 down to 0).
    for i in (0..10).rev() {
        let c = B32_ALPHABET[(t % 32) as usize] as char;
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        // increment random part in place
        let mut i: i32 = 15;
        while i >= 0 && ulid[10 + i as usize] == 'Z' {
            ulid[10 + i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            // restart 1ms + a bit later
            sleep(Duration::new(0, 1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let cur = ulid[10 + i as usize];
        if let Some(pos) = B32_ALPHABET.iter().position(|&c| c as char == cur) {
            // pos < 31 here because cur != 'Z' (otherwise the while loop would
            // have already advanced past it). Increment to next character.
            ulid[10 + i as usize] = B32_ALPHABET[pos + 1] as char;
            return;
        }
        // else fall through to randomize again when we found invalid chars
    }

    // Generate 16 random bytes and encode into the random portion (chars 10..26)
    let mut rnd = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut rnd);

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}
