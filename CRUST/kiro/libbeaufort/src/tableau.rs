pub fn beaufort_tableau(alpha: &str) -> Vec<Vec<u8>> {
    let alpha = alpha.as_bytes();
    let size = alpha.len();
    (0..size)
        .map(|y| {
            (0..size)
                .map(|x| alpha[(size - x + y) % size])
                .collect()
        })
        .collect()
}