use crate::tableau::beaufort_tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // Build default tableau if `mat` is empty.
    let default_mat: Vec<Vec<u8>>;
    let mat_rows: Vec<&[u8]> = if mat.is_empty() {
        default_mat = beaufort_tableau(std::str::from_utf8(BEAUFORT_ALPHA).unwrap());
        default_mat.iter().map(|r| r.as_slice()).collect()
    } else {
        mat.to_vec()
    };

    let ksize = key.len();
    let rsize = mat_rows[0].len();
    let mut enc: Vec<u8> = Vec::with_capacity(src.len());

    let mut j: usize = 0;
    for &ch in src.iter() {
        // find column where mat[0][x] == ch (the "top row")
        let mut needed = 1;
        let mut x: usize = 0;
        let mut y: usize = 0;
        let mut x_iter: usize = 0;
        while x_iter < rsize {
            if ch == mat_rows[y][x_iter] {
                needed = 1;
                x = x_iter;
                break;
            } else {
                needed = 0;
            }
            x_iter += 1;
        }

        // if char not in top row, append current char
        if needed == 0 {
            enc.push(ch);
            continue;
        }

        // determine char in `key'
        let k = key[j % ksize];
        j += 1;

        // find row in column with key char
        let mut y_iter: usize = 0;
        needed = 1;
        while y_iter < rsize {
            if k == mat_rows[y_iter][x] {
                needed = 1;
                y = y_iter;
                break;
            } else {
                needed = 0;
            }
            y_iter += 1;
        }

        if needed == 0 {
            enc.push(ch);
            j -= 1;
            continue;
        }

        enc.push(mat_rows[y][0]);
    }

    enc
}
