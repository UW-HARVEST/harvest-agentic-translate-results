pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

use crate::tableau::beaufort_tableau;

pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
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

    let mut dec: Vec<u8> = Vec::with_capacity(len);

    if ksize == 0 || rsize == 0 {
        return src.to_vec();
    }

    let mut j: usize = 0;

    for &ch in src.iter() {
        // Find the row whose leftmost char equals `ch`.
        let mut y: usize = 0;
        let mut found_y = false;
        while y < rsize {
            if ch == mat[y][0] {
                found_y = true;
                break;
            }
            y += 1;
        }

        if !found_y {
            dec.push(ch);
            continue;
        }

        // Pick the next key character.
        let k = key[j % ksize];
        j += 1;

        // In row `y`, find the column where the value equals `k`.
        let mut x: usize = 0;
        let mut found_x = false;
        while x < rsize {
            if k == mat[y][x] {
                found_x = true;
                break;
            }
            x += 1;
        }

        if !found_x {
            dec.push(ch);
            j -= 1;
            continue;
        }

        // Append the top-row character of column x.
        dec.push(mat[0][x]);
    }

    dec
}
