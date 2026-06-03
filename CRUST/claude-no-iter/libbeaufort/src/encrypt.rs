use crate::tableau::beaufort_tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // Build owned default tableau if `mat` is empty (mirrors NULL fallback in C).
    let owned: Vec<Vec<u8>>;
    let rows: Vec<&[u8]> = if mat.is_empty() {
        owned = beaufort_tableau(std::str::from_utf8(BEAUFORT_ALPHA).unwrap());
        owned.iter().map(|r| r.as_slice()).collect()
    } else {
        mat.to_vec()
    };

    let ksize = key.len();
    let rsize = rows[0].len();
    let mut enc: Vec<u8> = Vec::with_capacity(src.len());
    let mut j: usize = 0;

    for &ch in src.iter() {
        // Find column with `ch' at top (y = 0 in C).
        let mut x: usize = 0;
        let mut needed = 0;
        while x < rsize {
            if ch == rows[0][x] {
                needed = 1;
                break;
            } else {
                needed = 0;
            }
            x += 1;
        }

        if needed == 0 {
            enc.push(ch);
            continue;
        }

        // Determine char in `key'.
        let k = key[j % ksize];
        j += 1;

        // Find row in column `x' with `k'.
        let mut y: usize = 0;
        needed = 0;
        while y < rsize {
            if k == rows[y][x] {
                needed = 1;
                break;
            } else {
                needed = 0;
            }
            y += 1;
        }

        if needed == 0 {
            enc.push(ch);
            j -= 1;
            continue;
        }

        // Append left char.
        enc.push(rows[y][0]);
    }

    enc
}
