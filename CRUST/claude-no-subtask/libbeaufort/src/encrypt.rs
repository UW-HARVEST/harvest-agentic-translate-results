pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
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

    let mut enc: Vec<u8> = Vec::with_capacity(src.len());
    let mut j: usize = 0;

    for &ch in src.iter() {
        // Find column with `ch' in the top row (y=0)
        let mut x = 0usize;
        let mut found_col = false;
        while x < rsize {
            if ch == owned_mat[0][x] {
                found_col = true;
                break;
            }
            x += 1;
        }

        if !found_col {
            enc.push(ch);
            continue;
        }

        // determine char in `key'
        let k = key[j % ksize];
        j += 1;

        // find row in column with key char
        let mut y = 0usize;
        let mut found_row = false;
        while y < rsize {
            if k == owned_mat[y][x] {
                found_row = true;
                break;
            }
            y += 1;
        }

        if !found_row {
            enc.push(ch);
            j -= 1;
            continue;
        }

        // append left char
        enc.push(owned_mat[y][0]);
    }

    enc
}
