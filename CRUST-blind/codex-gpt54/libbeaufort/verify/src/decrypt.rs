pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

fn row_index_for_first_column(mat: &[&[u8]], ch: u8) -> Option<usize> {
    mat.iter()
        .position(|row| row.first().copied() == Some(ch))
}

fn column_index_for_row_value(row: &[u8], target: u8) -> Option<usize> {
    row.iter().position(|&candidate| candidate == target)
}

pub fn beaufort_decrypt(src: &[u8], key: &[u8], mat: &[&[u8]]) -> Vec<u8> {
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

    let mut dec = Vec::with_capacity(src.len());
    let mut j = 0usize;

    for &ch in src {
        let Some(y) = row_index_for_first_column(matrix, ch) else {
            dec.push(ch);
            continue;
        };

        let Some(row) = matrix.get(y) else {
            dec.push(ch);
            continue;
        };

        let k = key[j % key.len()];
        let Some(x) = column_index_for_row_value(row, k) else {
            dec.push(ch);
            continue;
        };

        dec.push(
            matrix
                .first()
                .and_then(|top_row| top_row.get(x))
                .copied()
                .unwrap_or(ch),
        );
        j += 1;
    }

    dec
}
