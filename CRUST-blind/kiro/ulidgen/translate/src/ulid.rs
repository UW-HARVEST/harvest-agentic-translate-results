// Import statements
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::time::Duration;
// Constants
pub const ULID_LENGTH: usize = 27;

const B32: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    let buf_offset = 10;
    let mut same = true;

    ulid[26] = '\0';

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let mut t: u64 = now.as_secs() * 1000 + (now.subsec_nanos() / 1_000_000) as u64;

    for i in (0..10).rev() {
        let c = B32[(t % 32) as usize] as char;
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        // increment random part in place
        let mut i: i32 = 15;
        while i >= 0 && ulid[buf_offset + i as usize] == 'Z' {
            ulid[buf_offset + i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            thread::sleep(Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let ch = ulid[buf_offset + i as usize] as u8;
        if let Some(pos) = B32.iter().position(|&b| b == ch) {
            ulid[buf_offset + i as usize] = B32[pos + 1] as char;
            return;
        }
        // else fall through to randomize
    }

    let mut rnd = [0u8; 16];
    getrandom::getrandom(&mut rnd).unwrap();

    for i in 0..16 {
        ulid[buf_offset + i] = B32[(rnd[i] % 32) as usize] as char;
    }
}
