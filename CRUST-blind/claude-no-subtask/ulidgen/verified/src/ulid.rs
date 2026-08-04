// Import statements
use crate::{ulidgen};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::thread;
use rand::RngCore;

// Constants
pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    // Set null terminator
    ulid[26] = '\0';

    let mut same = true;

    // Get time in milliseconds since epoch
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() as u64) / 1_000_000;

    // Fill timestamp portion (indices 0..10) from right to left
    for i in (0..10).rev() {
        let new_char = B32_ALPHABET[(t % 32) as usize] as char;
        if ulid[i] != new_char {
            ulid[i] = new_char;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Increment random part in place. The "buf" starts at index 10.
        // i is the index within the random part (0..16), corresponding to ulid[10+i].
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

        let current = ulid[10 + i as usize];
        let pos = B32_ALPHABET.iter().position(|&c| c as char == current);
        if let Some(p) = pos {
            // Replace with the next character in the alphabet
            // Note: in C, strchr returns position; *(s+1) is next char (could be '\0' if at end).
            // Since 'Z' is handled above, p < 31 here, so p+1 is safe.
            if p + 1 < B32_ALPHABET.len() {
                ulid[10 + i as usize] = B32_ALPHABET[p + 1] as char;
            } else {
                // p was 31 ('Z'), shouldn't happen due to loop above, but be safe.
                ulid[10 + i as usize] = '\0';
            }
            return;
        }
        // else fallthrough: randomize again when invalid chars found
    }

    // Generate 16 random bytes and encode random part
    let mut rnd = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut rnd);

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}
