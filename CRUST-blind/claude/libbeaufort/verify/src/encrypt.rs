pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

use crate::tableau::beaufort_tableau;

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // Build a fallback tableau (using the default alphabet) when none is given.
    let fallback: Vec<Vec<u8>>;
    let owned_refs: Vec<&[u8]>;
    let mat: &[&[u8]] = if mat.is_empty() {
        fallback = beaufort_tableau(
            std::str::from_utf8(BEAUFORT_ALPHA).expect("BEAUFORT_ALPHA is valid UTF-8"),
        );
        owned_refs = fallback.iter().map(|row| row.as_slice()).collect();
        owned_refs.as_slice()
    } else {
        mat
    };

    let ksize = key.len();
    let len = src.len();
    let rsize = if mat.is_empty() { 0 } else { mat[0].len() };

    let mut enc: Vec<u8> = Vec::with_capacity(len);

    if ksize == 0 || rsize == 0 {
        // No key or empty tableau — return src unchanged (mirrors C behavior in
        // the trivial case where no transformation can occur).
        return src.to_vec();
    }

    let mut j: usize = 0;

    for &ch in src.iter() {
        // Locate `ch` in the top row (y == 0).
        let mut x: usize = 0;
        let mut found_x = false;
        while x < rsize {
            if ch == mat[0][x] {
                found_x = true;
                break;
            }
            x += 1;
        }

        if !found_x {
            enc.push(ch);
            continue;
        }

        // Determine character in `key`.
        let k = key[j % ksize];
        j += 1;

        // Find the row in column `x` whose value equals `k`.
        let mut y: usize = 0;
        let mut found_y = false;
        while y < rsize {
            if k == mat[y][x] {
                found_y = true;
                break;
            }
            y += 1;
        }

        if !found_y {
            enc.push(ch);
            j -= 1;
            continue;
        }

        // Append the leftmost char in the located row.
        enc.push(mat[y][0]);
    }

    enc
}
