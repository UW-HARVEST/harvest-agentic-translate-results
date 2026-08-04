pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // Build a default tableau if none provided.
    let default_tableau: Vec<Vec<u8>>;
    let default_refs: Vec<&[u8]>;
    let mat_ref: &[&[u8]] = if mat.is_empty() {
        default_tableau = crate::tableau::beaufort_tableau(
            std::str::from_utf8(BEAUFORT_ALPHA).unwrap(),
        );
        default_refs = default_tableau.iter().map(|r| r.as_slice()).collect();
        &default_refs
    } else {
        mat
    };

    if mat_ref.is_empty() {
        return Vec::new();
    }

    let ksize = key.len();
    let rsize = mat_ref[0].len();
    let mut dec: Vec<u8> = Vec::with_capacity(src.len());

    if ksize == 0 || rsize == 0 {
        return src.to_vec();
    }

    let mut j: usize = 0;

    for &ch in src.iter() {
        // find row y where ch == mat[y][0]
        let mut y: usize = 0;
        let mut found_row = false;
        while y < rsize {
            if mat_ref[y][0] == ch {
                found_row = true;
                break;
            }
            y += 1;
        }

        if !found_row {
            dec.push(ch);
            continue;
        }

        // determine key char and advance j
        let k = key[j % ksize];
        j += 1;

        // find column x where k == mat[y][x]
        let mut x: usize = 0;
        let mut found_col = false;
        while x < rsize {
            if mat_ref[y][x] == k {
                found_col = true;
                break;
            }
            x += 1;
        }

        if !found_col {
            dec.push(ch);
            j -= 1;
            continue;
        }

        dec.push(mat_ref[0][x]);
    }

    dec
}
