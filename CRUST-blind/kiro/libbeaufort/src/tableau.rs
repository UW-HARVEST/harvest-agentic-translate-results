pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let alpha = alpha.as_bytes();
    let size = alpha.len();
    let mut mat = Vec::with_capacity(size);
    for y in 0..size {
        let mut row = Vec::with_capacity(size + 1);
        let mut j = size;
        for _ in 0..size {
            row.push(alpha[(j + y) % size]);
            j -= 1;
        }
        mat.push(row);
    }
    mat
}
