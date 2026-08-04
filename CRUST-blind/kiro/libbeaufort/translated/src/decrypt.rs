pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";
pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    let ksize = key.len();
    let rsize = mat[0].len();
    let mut dec = Vec::with_capacity(src.len());
    let mut j = 0usize;

    for &ch in src {
        // find row where leftmost column has ch
        let row = (0..rsize).find(|&y| ch == mat[y][0]);
        let y = match row {
            Some(y) => y,
            None => { dec.push(ch); continue; }
        };

        let k = key[j % ksize];
        j += 1;

        // find column in that row matching key char
        let col = (0..rsize).find(|&x| k == mat[y][x]);
        let x = match col {
            Some(x) => x,
            None => { dec.push(ch); j -= 1; continue; }
        };

        dec.push(mat[0][x]);
    }
    dec
}
