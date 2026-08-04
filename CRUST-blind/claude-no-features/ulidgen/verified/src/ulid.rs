// Import statements
use crate::{ulidgen};
// Constants
pub const ULID_LENGTH: usize = 27;
// Function Declarations
pub fn ulidgen_r(ulid: &mut [char; ULID_LENGTH]) {
    const B32: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    let mut same = true;

    // null-terminate the buffer (matches C behavior)
    ulid[26] = '\0';

    // current time in milliseconds since UNIX epoch
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0));
    let mut t: u64 = now.as_secs().wrapping_mul(1000)
        + (now.subsec_nanos() / 1_000_000) as u64;

    // encode timestamp into first 10 chars (most significant first)
    for i in (0..10).rev() {
        let c = B32[(t % 32) as usize] as char;
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Same millisecond as the previous ULID stored in this buffer:
        // increment the random part in place to preserve monotonicity.
        let mut i: i32 = 15;
        while i >= 0 && ulid[10 + i as usize] == 'Z' {
            ulid[10 + i as usize] = '0';
            i -= 1;
        }

        if i < 0 {
            // overflow: sleep slightly more than 1ms and try again
            std::thread::sleep(std::time::Duration::new(0, 1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let current = ulid[10 + i as usize] as u32;
        // find current char in the alphabet and bump to next
        let pos = B32.iter().position(|&c| c as u32 == current);
        if let Some(p) = pos {
            // p < 31 here because all 'Z' positions were rolled over above
            if p + 1 < B32.len() {
                ulid[10 + i as usize] = B32[p + 1] as char;
                return;
            }
        }
        // fall through: invalid char encountered, regenerate randomness
    }

    // generate fresh 16 bytes of randomness for the random portion
    let mut rnd = [0u8; 16];
    if getrandom::getrandom(&mut rnd).is_err() {
        // mirror C's abort() on entropy failure
        std::process::abort();
    }

    for i in 0..16 {
        ulid[10 + i] = B32[(rnd[i] % 32) as usize] as char;
    }
}
