// Import statements
use crate::{ulidgen};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread::sleep;
use std::time::Duration;

// Constants
pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    // Terminator (matches C's char ulid[27] with ulid[26] = 0)
    ulid[26] = '\0';

    // Compute timestamp in milliseconds since UNIX epoch
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() / 1_000_000) as u64;

    // Encode the first 10 characters from the timestamp (Base32 Crockford)
    let mut same = true;
    for i in (0..10).rev() {
        let c = B32_ALPHABET[(t % 32) as usize] as char;
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    // Helper closure: index in alphabet (returns None if not found)
    fn b32_index(c: char) -> Option<usize> {
        B32_ALPHABET.iter().position(|&b| b as char == c)
    }

    if same {
        // The timestamp is identical to the previous call: increment the
        // random portion (positions 10..26) in place.
        let mut i: i32 = 15;
        while i >= 0 && ulid[10 + i as usize] == 'Z' {
            ulid[10 + i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            // All Z's wrapped — wait a bit and restart.
            sleep(Duration::new(0, 1_234_567));
            ulidgen_r(ulid);
            return;
        }

        if let Some(idx) = b32_index(ulid[10 + i as usize]) {
            // The C code does *(s+1), which is safe because the largest valid
            // index is 31 ('Z') — but in that case we already replaced 'Z'
            // with '0' in the loop above, so idx < 31 here.
            ulid[10 + i as usize] = B32_ALPHABET[idx + 1] as char;
            return;
        }
        // Otherwise (invalid char encountered), fall through and randomize.
    }

    // Generate 16 random bytes and map each to a Base32 char.
    let mut rnd = [0u8; 16];
    getentropy(&mut rnd);

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}

fn getentropy(buf: &mut [u8]) {
    use rand::RngCore;
    rand::thread_rng().fill_bytes(buf);
}
