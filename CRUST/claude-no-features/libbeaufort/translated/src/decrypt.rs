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

    if mat_ref.is_empty() {
        return Vec::new();
    }

    let ksize = key.len();
    let rsize = mat_ref[0].len();
    let mut dec: Vec<u8> = Vec::with_capacity(src.len());
    let mut j: usize = 0;

    for &ch in src {
        // find column with char (search in column 0 of every row)
        let mut y_found: Option<usize> = None;
        for y in 0..rsize {
            if ch == mat_ref[y][0] {
                y_found = Some(y);
                break;
            }
        }

        let y = match y_found {
            Some(v) => v,
            None => {
                dec.push(ch);
                continue;
            }
        };

        if ksize == 0 {
            dec.push(ch);
            continue;
        }

        let k = key[j % ksize];
        j += 1;

        let mut x_found: Option<usize> = None;
        for x in 0..rsize {
            if k == mat_ref[y][x] {
                x_found = Some(x);
                break;
            }
        }

        match x_found {
            Some(x) => {
                dec.push(mat_ref[0][x]);
            }
            None => {
                dec.push(ch);
                j -= 1;
            }
        }
    }

    dec
}
