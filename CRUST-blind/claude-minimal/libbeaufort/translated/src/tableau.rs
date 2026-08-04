pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let bytes = alpha.as_bytes();
    let size = bytes.len();
    let mut mat: Vec<Vec<u8>> = Vec::with_capacity(size);

    for y in 0..size {
        let mut row: Vec<u8> = Vec::with_capacity(size);
        // In the C source, j starts at `size` and decrements as x increments.
        // mat[y][x] = alpha[(j + y) % size]
        // For x = 0..size and j = size..1, so (j + y) % size for x in 0..size
        // produces alpha[(size - x + y) % size].
        let mut j: usize = size;
        for _x in 0..size {
            row.push(bytes[(j + y) % size]);
            j -= 1;
        }
        mat.push(row);
    }

    mat
}
