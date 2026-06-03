use crate::tableau::beaufort_tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // Owned matrix (used when caller does not provide one)
    let owned_mat: Vec<Vec<u8>>;
    let mat_ref: Vec<&[u8]> = if mat.is_empty() {
        owned_mat = beaufort_tableau(std::str::from_utf8(BEAUFORT_ALPHA).unwrap());
        owned_mat.iter().map(|r| r.as_slice()).collect()
    } else {
        mat.to_vec()
    };

    let ksize = key.len();
    let rsize = mat_ref[0].len();

    let mut enc: Vec<u8> = Vec::with_capacity(src.len());
    let mut j: usize = 0;

    for &ch in src.iter() {
        // Find column with `ch` at top (row 0)
        let mut needed = 1;
        let mut x = 0usize;
        let y_top = 0usize;
        let mut col_x = 0usize;
        for xi in 0..rsize {
            if ch == mat_ref[y_top][xi] {
                needed = 1;
                col_x = xi;
                break;
            } else {
                needed = 0;
            }
            x = xi;
        }
        let _ = x;

        // If char not in top row, append the current char as-is
        if needed == 0 {
            enc.push(ch);
            continue;
        }

        // Determine char in `key`
        let k = key[j % ksize];
        j += 1;

        // Find row in column `col_x` that has the key char
        let mut row_y = 0usize;
        needed = 1;
        let mut found_row = false;
        for yi in 0..rsize {
            if k == mat_ref[yi][col_x] {
                needed = 1;
                row_y = yi;
                found_row = true;
                break;
            } else {
                needed = 0;
            }
        }
        let _ = found_row;

        // If key char isn't found in that column, append `ch` and step back j
        if needed == 0 {
            enc.push(ch);
            j -= 1;
            continue;
        }

        // Append left char of that row (column 0)
        enc.push(mat_ref[row_y][0]);
    }

    enc
}
