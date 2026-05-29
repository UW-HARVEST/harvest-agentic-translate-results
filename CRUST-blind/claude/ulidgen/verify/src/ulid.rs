// Import statements
use crate::{ulidgen};
// Constants
pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn b32_index(c: char) -> Option<usize> {
    B32_ALPHABET.iter().position(|&b| b as char == c)
}

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    let mut same = true;

    ulid[26] = '\0';

    // Get current time in milliseconds since epoch (CLOCK_REALTIME equivalent)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0));
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() as u64) / 1_000_000;

    // Fill timestamp portion (positions 0..10) from right to left
    for i in (0..10).rev() {
        let new_char = B32_ALPHABET[(t % 32) as usize] as char;
        if ulid[i] != new_char {
            ulid[i] = new_char;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Increment random part in place
        let mut i: i32 = 15;
        while i >= 0 && ulid[10 + i as usize] == 'Z' {
            ulid[10 + i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            // restart 1ms + a bit later
            std::thread::sleep(std::time::Duration::new(0, 1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let idx = 10 + i as usize;
        if let Some(pos) = b32_index(ulid[idx]) {
            ulid[idx] = B32_ALPHABET[pos + 1] as char;
            return;
        }
        // else randomize again when we found invalid chars
    }

    // Generate 16 random bytes and encode as base32
    let mut rnd = [0u8; 16];
    if getrandom::getrandom(&mut rnd).is_err() {
        std::process::abort();
    }

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}
