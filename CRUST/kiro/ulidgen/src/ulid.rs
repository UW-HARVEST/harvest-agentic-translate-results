// Import statements
use crate::{ulidgen};
// Constants
pub const ULID_LENGTH: usize = 27;

const B32: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    ulid[26] = '\0';

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() / 1_000_000) as u64;

    let mut same = true;
    for i in (0..10).rev() {
        let ch = B32[(t % 32) as usize] as char;
        if ulid[i] != ch {
            ulid[i] = ch;
            same = false;
        }
        t /= 32;
    }

    if same {
        // increment random part in place
        let buf = &mut ulid[10..26];
        let mut i: i32 = 15;
        while i >= 0 && buf[i as usize] == 'Z' {
            buf[i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            // restart 1ms + a bit later
            std::thread::sleep(std::time::Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let c = buf[i as usize];
        if let Some(pos) = B32.iter().position(|&b| b as char == c) {
            buf[i as usize] = B32[pos + 1] as char;
            return;
        }
        // else randomize again when we found invalid chars (fall through)
    }

    let mut rnd = [0u8; 16];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut rnd);

    for i in 0..16 {
        ulid[10 + i] = B32[(rnd[i] % 32) as usize] as char;
    }
}