use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const ULID_LENGTH: usize = 27;

const B32_ALPHABET: [char; 32] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
    'J', 'K', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'V', 'W', 'X', 'Y', 'Z',
];

fn current_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

fn fill_random(buf: &mut [char]) {
    let bytes = rand::random::<[u8; 16]>();
    for (slot, byte) in buf.iter_mut().zip(bytes) {
        *slot = B32_ALPHABET[(byte % 32) as usize];
    }
}

pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    loop {
        let mut t = current_millis();
        let mut same = true;

        ulid[26] = '\0';

        for i in (0..10).rev() {
            let ch = B32_ALPHABET[(t % 32) as usize];
            if ulid[i] != ch {
                ulid[i] = ch;
                same = false;
            }
            t /= 32;
        }

        let buf = &mut ulid[10..26];

        if same {
            let mut idx = 15usize;
            while buf[idx] == 'Z' {
                buf[idx] = '0';
                if idx == 0 {
                    break;
                }
                idx -= 1;
            }

            if idx == 0 && buf[0] == '0' {
                sleep(Duration::from_nanos(1_234_567));
                continue;
            }

            if let Some(pos) = B32_ALPHABET.iter().position(|&ch| ch == buf[idx]) {
                buf[idx] = B32_ALPHABET[pos + 1];
                return;
            }
        }

        fill_random(buf);
        return;
    }
}
