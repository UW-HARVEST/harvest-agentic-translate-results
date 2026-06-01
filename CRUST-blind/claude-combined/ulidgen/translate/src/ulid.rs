// Import statements
use crate::{ulidgen};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread::sleep;
use std::time::Duration;
use rand::RngCore;

// Constants
pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn b32_index(c: char) -> Option<usize> {
    B32_ALPHABET.iter().position(|&b| b as char == c)
}

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    // Mark index 26 as null terminator (C compatibility)
    ulid[26] = '\0';

    // Get current time in milliseconds since UNIX epoch
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch");
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() as u64) / 1_000_000;

    // Encode the timestamp into ulid[0..10] from least to most significant
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
        // Increment random part in place (positions 10..26)
        let mut idx: i32 = 15;
        while idx >= 0 && ulid[10 + idx as usize] == 'Z' {
            ulid[10 + idx as usize] = '0';
            idx -= 1;
        }

        if idx < 0 {
            // restart 1ms + a bit later
            sleep(Duration::new(0, 1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let pos = 10 + idx as usize;
        if let Some(s_idx) = b32_index(ulid[pos]) {
            ulid[pos] = B32_ALPHABET[s_idx + 1] as char;
            return;
        }
        // else fall through to randomize
    }

    // Generate fresh random portion
    let mut rnd = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut rnd);

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}
