pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let alpha_bytes = alpha.as_bytes();
    let size = alpha_bytes.len();
    let mut mat: Vec<Vec<u8>> = Vec::with_capacity(size);

    for y in 0..size {
        let mut row = vec![0u8; size];
        let mut j = size as isize;
        for x in 0..size {
            // (j + y) % size, where size > 0
            let idx = ((j + y as isize) as usize) % size;
            row[x] = alpha_bytes[idx];
            j -= 1;
        }
        mat.push(row);
    }

    mat
}
