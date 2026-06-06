pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

use crate::tableau::beaufort_tableau;

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // Owned fallback if `mat` is empty.
    let owned: Vec<Vec<u8>>;
    let mat_refs: Vec<&[u8]>;
    let mat_used: &[&[u8]] = if mat.is_empty() {
        owned = beaufort_tableau(
            std::str::from_utf8(BEAUFORT_ALPHA).expect("default alpha is utf-8"),
        );
        mat_refs = owned.iter().map(|r| r.as_slice()).collect();
        &mat_refs
    } else {
        mat
    };

    let ksize = key.len();
    let rsize = if mat_used.is_empty() { 0 } else { mat_used[0].len() };
    let mut enc: Vec<u8> = Vec::with_capacity(src.len());

    let mut j: usize = 0;

    for &ch in src.iter() {
        // find column with `ch` in the top row
        let mut needed = 1;
        let mut x_found: usize = 0;
        let y0 = 0usize;
        if rsize == 0 {
            // mirror C behavior: needed starts as 1 but no rows; treat as not found
            needed = 0;
        } else {
            let mut found = false;
            for x in 0..rsize {
                if ch == mat_used[y0][x] {
                    needed = 1;
                    x_found = x;
                    found = true;
                    break;
                } else {
                    needed = 0;
                }
            }
            if !found {
                // needed already set to 0
            }
        }

        if needed == 0 {
            enc.push(ch);
            continue;
        }

        if ksize == 0 {
            // can't determine key char; just append
            enc.push(ch);
            continue;
        }

        let k = key[j % ksize];
        j = j.wrapping_add(1);

        // find row in column x_found containing k
        let mut y_found: usize = 0;
        let mut found_y = false;
        let mut needed2 = 1;
        for y in 0..rsize {
            if k == mat_used[y][x_found] {
                needed2 = 1;
                y_found = y;
                found_y = true;
                break;
            } else {
                needed2 = 0;
            }
        }

        if needed2 == 0 || !found_y {
            enc.push(ch);
            j = j.wrapping_sub(1);
            continue;
        }

        // append left char of found row
        enc.push(mat_used[y_found][0]);
    }

    enc
}
