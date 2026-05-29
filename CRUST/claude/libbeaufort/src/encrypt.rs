pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    // build the default tableau if no matrix was provided
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
    let mut enc: Vec<u8> = Vec::with_capacity(src.len());

    let mut j: usize = 0;

    for &ch in src.iter() {
        // find column with `ch' at top (y=0)
        let mut needed = 0;
        let mut x = 0usize;
        // mirror the C loop semantics: scan x with y fixed at 0
        // but the C code keeps `y = 0` and iterates x
        // The C code accidentally scans mat[0][x] -- since y starts at 0
        // and never gets reassigned in this loop. Let's match it exactly.
        let y0 = 0usize;
        for xi in 0..rsize {
            if ch == mat_slice[y0][xi] {
                needed = 1;
                x = xi;
                break;
            } else {
                needed = 0;
            }
        }

        if needed == 0 {
            enc.push(ch);
            continue;
        }

        // determine char in `key'
        let k = key[j % ksize];
        j += 1;

        // find row in column x with key char k
        let mut y = 0usize;
        needed = 0;
        for yi in 0..rsize {
            if k == mat_slice[yi][x] {
                needed = 1;
                y = yi;
                break;
            } else {
                needed = 0;
            }
        }

        if needed == 0 {
            enc.push(ch);
            j -= 1;
            continue;
        }

        // append left char
        enc.push(mat_slice[y][0]);
    }

    enc
}
