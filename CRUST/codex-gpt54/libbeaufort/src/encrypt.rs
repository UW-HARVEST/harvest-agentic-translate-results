pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";
pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    if src.is_empty() {
        return Vec::new();
    }

    let owned_mat;
    let mat = if mat.is_empty() {
        owned_mat = crate::tableau::beaufort_tableau(std::str::from_utf8(BEAUFORT_ALPHA).unwrap());
        owned_mat.iter().map(Vec::as_slice).collect::<Vec<_>>()
    } else {
        mat.to_vec()
    };

    if mat.is_empty() || mat[0].is_empty() || key.is_empty() {
        return src.to_vec();
    }

    let top_row = mat[0];
    let mut enc = Vec::with_capacity(src.len());
    let mut j = 0usize;

    for &ch in src {
        let Some(x) = top_row.iter().position(|&value| value == ch) else {
            enc.push(ch);
            continue;
        };

        let k = key[j % key.len()];
        j += 1;

        let Some(y) = mat.iter().position(|row| row.get(x).copied() == Some(k)) else {
            enc.push(ch);
            j -= 1;
            continue;
        };

        enc.push(mat[y][0]);
    }

    enc
}
