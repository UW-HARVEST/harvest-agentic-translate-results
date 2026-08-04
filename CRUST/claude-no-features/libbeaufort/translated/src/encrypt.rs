use crate::tableau::beaufort_tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // Build owned matrix from input slice or default tableau
    let owned_mat: Vec<Vec<u8>>;
    let mat_ref: Vec<&[u8]> = if mat.is_empty() {
        owned_mat = beaufort_tableau(std::str::from_utf8(BEAUFORT_ALPHA).unwrap());
        owned_mat.iter().map(|r| r.as_slice()).collect()
    } else {
        mat.to_vec()
    };

    if mat_ref.is_empty() {
        return Vec::new();
    }

    let ksize = key.len();
    let rsize = mat_ref[0].len();
    let mut enc: Vec<u8> = Vec::with_capacity(src.len());
    let mut j: usize = 0;

    for &ch in src {
        // Find column with `ch' at top (y=0)
        let mut x_found: Option<usize> = None;
        for x in 0..rsize {
            if ch == mat_ref[0][x] {
                x_found = Some(x);
                break;
            }
        }

        let x = match x_found {
            Some(v) => v,
            None => {
                enc.push(ch);
                continue;
            }
        };

        // determine char in `key'
        if ksize == 0 {
            enc.push(ch);
            continue;
        }
        let k = key[j % ksize];
        j += 1;

        // find row in column with key char
        let mut y_found: Option<usize> = None;
        for y in 0..rsize {
            if k == mat_ref[y][x] {
                y_found = Some(y);
                break;
            }
        }

        match y_found {
            Some(y) => {
                enc.push(mat_ref[y][0]);
            }
            None => {
                enc.push(ch);
                j -= 1;
            }
        }
    }

    enc
}
