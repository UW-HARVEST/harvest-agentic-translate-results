pub fn print_bin(n: u64) -> String {
    (0..64).map(|i| if n & (1u64 << (63 - i)) != 0 { '1' } else { '0' }).collect()
}
