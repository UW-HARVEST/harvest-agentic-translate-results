pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let bytes = alpha.as_bytes();
    let size = bytes.len();
    let mut mat: Vec<Vec<u8>> = Vec::with_capacity(size);

    for y in 0..size {
        let mut row: Vec<u8> = Vec::with_capacity(size);
        // matches the C code: x = 0, j = size; ++x, --j
        let mut j: usize = size;
        for _x in 0..size {
            let idx = (j + y) % size;
            row.push(bytes[idx]);
            // decrement j (may underflow on the last iteration but is unused after)
            j = j.wrapping_sub(1);
        }
        mat.push(row);
    }

    mat
}
