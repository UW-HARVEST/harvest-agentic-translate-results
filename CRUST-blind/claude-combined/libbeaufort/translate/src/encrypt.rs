pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // Build a default tableau if mat is empty.
    let default_tableau: Vec<Vec<u8>>;
    let default_refs: Vec<&[u8]>;
    let mat: &[&[u8]] = if mat.is_empty() {
        default_tableau = crate::tableau::beaufort_tableau(
            std::str::from_utf8(BEAUFORT_ALPHA).expect("BEAUFORT_ALPHA is valid UTF-8"),
        );
        default_refs = default_tableau.iter().map(|row| row.as_slice()).collect();
        &default_refs
    } else {
        mat
    };

    let ksize = key.len();
    let rsize = mat[0].len();
    let mut enc: Vec<u8> = Vec::with_capacity(src.len());
    let mut j: usize = 0;

    if ksize == 0 {
        // Without a key, every "needed" lookup would be invalid; just return src as-is.
        // (The C code would divide by zero here.)
        return src.to_vec();
    }

    for &ch in src.iter() {
        // Find column with `ch' at top (y=0 row).
        // The C code only checks mat[0][x] (since y stays 0 in the first loop).
        let mut x_found: Option<usize> = None;
        for x in 0..rsize {
            if ch == mat[0][x] {
                x_found = Some(x);
                break;
            }
        }

        let x = match x_found {
            Some(x) => x,
            None => {
                // char not in top row; append unchanged
                enc.push(ch);
                continue;
            }
        };

        // determine char in `key'
        let k = key[j % ksize];
        j += 1;

        // find row in column with `key[k]'
        let mut y_found: Option<usize> = None;
        for y in 0..rsize {
            if k == mat[y][x] {
                y_found = Some(y);
                break;
            }
        }

        let y = match y_found {
            Some(y) => y,
            None => {
                enc.push(ch);
                j -= 1;
                continue;
            }
        };

        // append left char
        enc.push(mat[y][0]);
    }

    enc
}
