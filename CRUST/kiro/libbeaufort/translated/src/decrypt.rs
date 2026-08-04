pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";
pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    use crate::tableau::beaufort_tableau;

    let owned;
    let mat: Vec<&[u8]> = if mat.is_empty() {
        owned = beaufort_tableau(std::str::from_utf8(BEAUFORT_ALPHA).unwrap());
        owned.iter().map(|r| r.as_slice()).collect()
    } else {
        mat.to_vec()
    };

    let ksize = key.len();
    let rsize = mat[0].len();
    let mut dec = Vec::with_capacity(src.len());
    let mut j = 0usize;

    for &ch in src {
        // find ch in first column
        let y_pos = (0..rsize).find(|&y| mat[y][0] == ch);
        let y = match y_pos {
            Some(y) => y,
            None => { dec.push(ch); continue; }
        };

        let k = key[j % ksize];
        j += 1;

        // find k in row y
        match mat[y].iter().position(|&c| c == k) {
            Some(x) => dec.push(mat[0][x]),
            None => { dec.push(ch); j -= 1; }
        }
    }

    dec
}