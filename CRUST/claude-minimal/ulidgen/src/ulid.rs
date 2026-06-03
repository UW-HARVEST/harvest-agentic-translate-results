// Import statements
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// Constants
pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    let mut same = true;

    // Null-terminator slot, mirrors C's `ulid[26] = 0;`
    ulid[26] = '\0';

    // Milliseconds since the UNIX epoch (matches `tv.tv_sec*1000 + tv.tv_nsec/1000000`).
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX epoch");
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() / 1_000_000) as u64;

    // Encode the 10-character base32 timestamp, least-significant digit first.
    // Track whether every timestamp character was already correct, in which case
    // we just bump the random suffix to preserve monotonicity.
    for i in (0..10).rev() {
        let c = B32_ALPHABET[(t % 32) as usize] as char;
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Increment the 16-character random part in place.
        let mut i: i32 = 15;
        while i >= 0 && ulid[10 + i as usize] == 'Z' {
            ulid[10 + i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            // The random part overflowed; wait a bit and try again.
            thread::sleep(Duration::new(0, 1234567));
            ulidgen_r(ulid);
            return;
        }

        let current = ulid[10 + i as usize];
        if let Some(pos) = B32_ALPHABET.iter().position(|&c| c as char == current) {
            ulid[10 + i as usize] = B32_ALPHABET[pos + 1] as char;
            return;
        }
        // Fall through and re-randomize when we found an invalid char.
    }

    // Generate a fresh 16-byte random buffer (mirrors `getentropy(rnd, 16)`).
    let mut rnd = [0u8; 16];
    let mut f = File::open("/dev/urandom").expect("failed to open /dev/urandom");
    f.read_exact(&mut rnd).expect("failed to read /dev/urandom");

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}
