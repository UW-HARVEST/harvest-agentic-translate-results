pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

fn top_row_index(mat: &[&[u8]], ch: u8) -> Option<usize> {
    mat.first()?.iter().position(|&candidate| candidate == ch)
}

fn row_index_for_column_value(mat: &[&[u8]], column: usize, target: u8) -> Option<usize> {
    mat.iter()
        .position(|row| row.get(column).copied() == Some(target))
}

pub fn beaufort_encrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
    if key.is_empty() {
        return src.to_vec();
    }

    let generated_mat;
    let generated_refs;
    let matrix: &[&[u8]] = if mat.is_empty() {
        generated_mat = crate::tableau::beaufort_tableau(
            "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        );
        generated_refs = generated_mat.iter().map(Vec::as_slice).collect::<Vec<_>>();
        &generated_refs
    } else {
        mat
    };

    let mut enc = Vec::with_capacity(src.len());
    let mut j = 0usize;

    for &ch in src {
        let Some(x) = top_row_index(matrix, ch) else {
            enc.push(ch);
            continue;
        };

        let k = key[j % key.len()];
        let Some(y) = row_index_for_column_value(matrix, x, k) else {
            enc.push(ch);
            continue;
        };

        enc.push(
            matrix
                .get(y)
                .and_then(|row| row.first())
                .copied()
                .unwrap_or(ch),
        );
        j += 1;
    }

    enc
}
