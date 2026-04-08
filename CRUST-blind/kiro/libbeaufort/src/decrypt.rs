pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";
pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    let ksize = key.len();
    let rsize = mat[0].len();
    let mut dec = Vec::with_capacity(src.len());
    let mut j = 0usize;

    for &ch in src {
        // find row where first column matches ch
        let mut found_y = None;
        for y in 0..rsize {
            if ch == mat[y][0] {
                found_y = Some(y);
                break;
            }
        }

        let y = match found_y {
            Some(y) => y,
            None => { dec.push(ch); continue; }
        };

        // determine key char
        let k = key[j % ksize];
        j += 1;

        // find column in that row matching key char
        let mut found_x = None;
        for x in 0..rsize {
            if k == mat[y][x] {
                found_x = Some(x);
                break;
            }
        }

        match found_x {
            Some(x) => dec.push(mat[0][x]),
            None => { dec.push(ch); j -= 1; }
        }
    }

    dec
}
