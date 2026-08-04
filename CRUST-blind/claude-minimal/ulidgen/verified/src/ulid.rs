// Import statements
#[allow(unused_imports)]
use crate::ulidgen;
use std::fs::File;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};
// Constants
pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    // ulid[26] = 0;  (NUL terminator in the C version)
    ulid[26] = '\0';

    // uint64_t t = tv.tv_sec*1000 + tv.tv_nsec/1000000;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX EPOCH");
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() as u64) / 1_000_000;

    let mut same = true;

    // for (int i = 9; i >= 0; i--, t /= 32)
    //     if (ulid[i] != b32alphabet[t % 32])
    //         ulid[i] = b32alphabet[t % 32], same = 0;
    for i in (0..10).rev() {
        let new_char = B32_ALPHABET[(t % 32) as usize] as char;
        if ulid[i] != new_char {
            ulid[i] = new_char;
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
            std::thread::sleep(std::time::Duration::new(0, 1_234_567));
            ulidgen_r(ulid);
            return;
        }

        // char *s = strchr(b32alphabet, buf[i]);
        // if (s) { buf[i] = *(s+1); return; }
        let cur = ulid[10 + i as usize] as u8;
        if let Some(pos) = B32_ALPHABET.iter().position(|&c| c == cur) {
            if pos + 1 < B32_ALPHABET.len() {
                ulid[10 + i as usize] = B32_ALPHABET[pos + 1] as char;
                return;
            }
        }
        // else randomize again when we found invalid chars
    }

    // unsigned char rnd[16]; getentropy(rnd, sizeof rnd);
    let mut rnd = [0u8; 16];
    let mut f = File::open("/dev/urandom").expect("failed to open /dev/urandom");
    f.read_exact(&mut rnd).expect("failed to read entropy");

    // for (int i = 0; i < 16; i++) buf[i] = b32alphabet[rnd[i] % 32];
    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize] as char;
    }
}
