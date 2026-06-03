pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // If `mat' is empty, build the default tableau from BEAUFORT_ALPHA.
    let default_mat: Vec<Vec<u8>>;
    let default_refs: Vec<&[u8]>;
    let mat: &[&[u8]] = if mat.is_empty() {
        let alpha = std::str::from_utf8(BEAUFORT_ALPHA).unwrap();
        default_mat = crate::tableau::beaufort_tableau(alpha);
        default_refs = default_mat.iter().map(|r| r.as_slice()).collect();
        &default_refs
    } else {
        mat
    };

    let ksize = key.len();
    let rsize = mat[0].len();
    let mut enc: Vec<u8> = Vec::with_capacity(src.len());

    let mut j: usize = 0;

    for &ch in src.iter() {
        // find column where the top row contains `ch'
        let mut x: usize = 0;
        let mut found_col = false;
        for xi in 0..rsize {
            if ch == mat[0][xi] {
                x = xi;
                found_col = true;
                break;
            }
        }

        // if char not in top row, append the char and continue
        if !found_col {
            enc.push(ch);
            continue;
        }

        // determine char in `key'
        if ksize == 0 {
            enc.push(ch);
            continue;
        }
        let k = key[j % ksize];
        j += 1;

        // find row in column with key char
        let mut y: usize = 0;
        let mut found_row = false;
        for yi in 0..rsize {
            if k == mat[yi][x] {
                y = yi;
                found_row = true;
                break;
            }
        }

        // append char and decrement unused modulo index if not needed
        if !found_row {
            enc.push(ch);
            j -= 1;
            continue;
        }

        // append left char
        enc.push(mat[y][0]);
    }

    enc
}
