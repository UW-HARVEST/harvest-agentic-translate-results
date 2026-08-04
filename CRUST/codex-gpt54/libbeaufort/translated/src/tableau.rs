pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let bytes = alpha.as_bytes();
    let size = bytes.len();
    let mut mat = Vec::with_capacity(size);

    for y in 0..size {
        let mut row = Vec::with_capacity(size);
        for x in 0..size {
            let j = size - x;
            row.push(bytes[(j + y) % size]);
        }
        mat.push(row);
    }

    mat
}
