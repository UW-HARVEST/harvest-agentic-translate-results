pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let bytes = alpha.as_bytes();
    let size = bytes.len();
    let mut mat: Vec<Vec<u8>> = Vec::with_capacity(size);

    for y in 0..size {
        let mut row = Vec::with_capacity(size);
        let mut j = size;
        for _x in 0..size {
            row.push(bytes[(j + y) % size]);
            j -= 1;
        }
        mat.push(row);
    }

    mat
}
