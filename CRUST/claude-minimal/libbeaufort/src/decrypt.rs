use crate::tableau::beaufort_tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    let owned_mat: Vec<Vec<u8>>;
    let mat_ref: Vec<&[u8]> = if mat.is_empty() {
        owned_mat = beaufort_tableau(std::str::from_utf8(BEAUFORT_ALPHA).unwrap());
        owned_mat.iter().map(|r| r.as_slice()).collect()
    } else {
        mat.to_vec()
    };

    let ksize = key.len();
    let rsize = mat_ref[0].len();

    let mut dec: Vec<u8> = Vec::with_capacity(src.len());
    let mut j: usize = 0;

    for &ch in src.iter() {
        // Find row whose first column equals `ch`
        let mut needed = 1;
        let mut row_y = 0usize;
        for yi in 0..rsize {
            if ch == mat_ref[yi][0] {
                needed = 1;
                row_y = yi;
                break;
            } else {
                needed = 0;
            }
        }

        // If not found, pass through unchanged
        if needed == 0 {
            dec.push(ch);
            continue;
        }

        // Determine key char
        let k = key[j % ksize];
        j += 1;

        // Find column in that row containing the key char
        let mut col_x = 0usize;
        needed = 1;
        for xi in 0..rsize {
            if k == mat_ref[row_y][xi] {
                needed = 1;
                col_x = xi;
                break;
            } else {
                needed = 0;
            }
        }

        if needed == 0 {
            dec.push(ch);
            j -= 1;
            continue;
        }

        // Append top-row char at the matching column
        dec.push(mat_ref[0][col_x]);
    }

    dec
}
