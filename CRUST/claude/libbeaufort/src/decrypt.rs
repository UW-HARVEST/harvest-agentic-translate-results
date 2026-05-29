pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    let default_owned: Vec<Vec<u8>>;
    let default_refs: Vec<&[u8]>;
    let mat_slice: &[&[u8]] = if mat.is_empty() {
        default_owned = crate::tableau::beaufort_tableau(
            std::str::from_utf8(BEAUFORT_ALPHA).unwrap(),
        );
        default_refs = default_owned.iter().map(|r| r.as_slice()).collect();
        &default_refs
    } else {
        mat
    };

    let ksize = key.len();
    let rsize = mat_slice[0].len();
    let mut dec: Vec<u8> = Vec::with_capacity(src.len());

    let mut j: usize = 0;

    for &ch in src.iter() {
        // find column with char (rows where mat[y][0] == ch)
        let mut needed = 0;
        let mut y = 0usize;
        for yi in 0..rsize {
            if ch == mat_slice[yi][0] {
                needed = 1;
                y = yi;
                break;
            } else {
                needed = 0;
            }
        }

        if needed == 0 {
            dec.push(ch);
            continue;
        }

        // determine char in `key'
        let k = key[j % ksize];
        j += 1;

        let mut x = 0usize;
        needed = 0;
        for xi in 0..rsize {
            if k == mat_slice[y][xi] {
                needed = 1;
                x = xi;
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

        dec.push(mat_slice[0][x]);
    }

    dec
}
