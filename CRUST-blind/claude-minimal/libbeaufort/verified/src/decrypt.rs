pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
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
    let mut dec: Vec<u8> = Vec::with_capacity(src.len());

    let mut j: usize = 0;

    for &ch in src.iter() {
        // find column (row index in C since mat[y][0]) containing `ch' in column 0
        let mut y: usize = 0;
        let mut found_y = false;
        for yi in 0..rsize {
            if ch == mat[yi][0] {
                y = yi;
                found_y = true;
                break;
            }
        }

        // if not found, append char and continue
        if !found_y {
            dec.push(ch);
            continue;
        }

        // determine char in `key'
        if ksize == 0 {
            dec.push(ch);
            continue;
        }
        let k = key[j % ksize];
        j += 1;

        // find x such that mat[y][x] == k
        let mut x: usize = 0;
        let mut found_x = false;
        for xi in 0..rsize {
            if k == mat[y][xi] {
                x = xi;
                found_x = true;
                break;
            }
        }

        // if not found, append char and decrement unused modulo index
        if !found_x {
            dec.push(ch);
            j -= 1;
            continue;
        }

        dec.push(mat[0][x]);
    }

    dec
}
