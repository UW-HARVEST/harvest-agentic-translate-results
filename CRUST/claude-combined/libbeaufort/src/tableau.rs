pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let bytes = alpha.as_bytes();
    let size = bytes.len();
    let mut mat: Vec<Vec<u8>> = Vec::with_capacity(size);

    for y in 0..size {
        let mut row = vec![0u8; size];
        let mut j: usize = size;
        for x in 0..size {
            row[x] = bytes[(j + y) % size];
            j -= 1;
        }
        mat.push(row);
    }

    mat
}
