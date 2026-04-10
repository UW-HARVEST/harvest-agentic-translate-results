// Import statements
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

// Constants
pub const ULID_LENGTH: usize = 27;

const B32: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    ulid[26] = '\0';

    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let mut t: u64 = dur.as_secs() * 1000 + dur.subsec_millis() as u64;

    let mut same = true;
    for i in (0..10).rev() {
        let c = B32[(t % 32) as usize] as char;
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        // increment random part in place (indices 10..=25)
        let mut i: i32 = 15;
        while i >= 0 && ulid[10 + i as usize] == 'Z' {
            ulid[10 + i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            std::thread::sleep(std::time::Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let idx = 10 + i as usize;
        if let Some(pos) = B32.iter().position(|&b| b as char == ulid[idx]) {
            ulid[idx] = B32[pos + 1] as char;
            return;
        }
        // else fall through to randomize
    }

    let mut rng = rand::thread_rng();
    let rnd: [u8; 16] = rng.gen();
    for i in 0..16 {
        ulid[10 + i] = B32[(rnd[i] % 32) as usize] as char;
    }
}
