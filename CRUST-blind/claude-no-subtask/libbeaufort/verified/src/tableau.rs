pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let bytes = alpha.as_bytes();
    let size = bytes.len();
    let mut mat: Vec<Vec<u8>> = Vec::with_capacity(size);

    for y in 0..size {
        let mut row: Vec<u8> = Vec::with_capacity(size);
        let mut j: i64 = size as i64;
        for _x in 0..size {
            let idx = ((j + y as i64).rem_euclid(size as i64)) as usize;
            row.push(bytes[idx]);
            j -= 1;
        }
        mat.push(row);
    }

    mat
}
