pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let bytes = alpha.as_bytes();
    let size = bytes.len();
    let mut mat: Vec<Vec<u8>> = Vec::with_capacity(size);

    for y in 0..size {
        let mut row = vec![0u8; size];
        // mirrors C: for (x = 0, j = size; x < size; ++x, --j)
        // row[x] = alpha[(j + y) % size]
        let mut j = size;
        for x in 0..size {
            row[x] = bytes[(j + y) % size];
            if j > 0 {
                j -= 1;
            }
        }
        mat.push(row);
    }

    mat
}
