pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";
pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    let ksize = key.len();
    let rsize = mat[0].len();
    let mut enc = Vec::with_capacity(src.len());
    let mut j = 0usize;

    for &ch in src {
        // find column where top row has ch
        let col = (0..rsize).find(|&x| ch == mat[0][x]);
        let x = match col {
            Some(x) => x,
            None => { enc.push(ch); continue; }
        };

        let k = key[j % ksize];
        j += 1;

        // find row in that column matching key char
        let row = (0..rsize).find(|&y| k == mat[y][x]);
        let y = match row {
            Some(y) => y,
            None => { enc.push(ch); j -= 1; continue; }
        };

        enc.push(mat[y][0]);
    }
    enc
}
