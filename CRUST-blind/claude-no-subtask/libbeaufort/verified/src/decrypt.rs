pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

use crate::tableau::beaufort_tableau;

pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
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
    let mut dec: Vec<u8> = Vec::with_capacity(src.len());

    let mut j: usize = 0;

    for &ch in src.iter() {
        // find row with ch in column 0
        let mut needed = 1;
        let mut y_found: usize = 0;
        if rsize == 0 {
            needed = 0;
        } else {
            let mut found = false;
            for y in 0..rsize {
                if ch == mat_used[y][0] {
                    needed = 1;
                    y_found = y;
                    found = true;
                    break;
                } else {
                    needed = 0;
                }
            }
            if !found {
                // needed already 0
            }
        }

        if needed == 0 {
            dec.push(ch);
            continue;
        }

        if ksize == 0 {
            dec.push(ch);
            continue;
        }

        let k = key[j % ksize];
        j = j.wrapping_add(1);

        // find column x in row y_found containing k
        let mut x_found: usize = 0;
        let mut found_x = false;
        let mut needed2 = 1;
        for x in 0..rsize {
            if k == mat_used[y_found][x] {
                needed2 = 1;
                x_found = x;
                found_x = true;
                break;
            } else {
                needed2 = 0;
            }
        }

        if needed2 == 0 || !found_x {
            dec.push(ch);
            j = j.wrapping_sub(1);
            continue;
        }

        dec.push(mat_used[0][x_found]);
    }

    dec
}
