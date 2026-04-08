pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";
pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    let ksize = key.len();
    let rsize = mat[0].len();
    let mut enc = Vec::with_capacity(src.len());
    let mut j = 0usize;

    for &ch in src {
        // find column with ch at top row
        let mut found_x = None;
        for x in 0..rsize {
            if ch == mat[0][x] {
                found_x = Some(x);
                break;
            }
        }

        let x = match found_x {
            Some(x) => x,
            None => { enc.push(ch); continue; }
        };

        // determine key char
        let k = key[j % ksize];
        j += 1;

        // find row in that column matching key char
        let mut found_y = None;
        for y in 0..rsize {
            if k == mat[y][x] {
                found_y = Some(y);
                break;
            }
        }

        match found_y {
            Some(y) => enc.push(mat[y][0]),
            None => { enc.push(ch); j -= 1; }
        }
    }

    enc
}
