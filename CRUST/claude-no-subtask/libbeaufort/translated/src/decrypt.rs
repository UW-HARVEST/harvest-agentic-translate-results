pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // If mat is empty, generate the default tableau
    let owned_mat: Vec<Vec<u8>> = if mat.is_empty() {
        let alpha = std::str::from_utf8(BEAUFORT_ALPHA).unwrap();
        crate::tableau::beaufort_tableau(alpha)
    } else {
        mat.iter().map(|r| r.to_vec()).collect()
    };

    let ksize = key.len();
    let rsize = owned_mat[0].len();

    // Strip a trailing NUL byte if present (matches C strlen semantics)
    let effective_len = src
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(src.len());
    let src = &src[..effective_len];

    let mut dec: Vec<u8> = Vec::with_capacity(src.len());
    let mut j: usize = 0;

    for &ch in src.iter() {
        // find row whose first column equals ch
        let mut y = 0usize;
        let mut found_row = false;
        while y < rsize {
            if ch == owned_mat[y][0] {
                found_row = true;
                break;
            }
            y += 1;
        }

        if !found_row {
            dec.push(ch);
            continue;
        }

        // determine char in key
        let k = key[j % ksize];
        j += 1;

        // find column in this row with key char
        let mut x = 0usize;
        let mut found_col = false;
        while x < rsize {
            if k == owned_mat[y][x] {
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

        // append top-row char at the found column
        dec.push(owned_mat[0][x]);
    }

    dec
}
