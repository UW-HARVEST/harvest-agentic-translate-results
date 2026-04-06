pub mod decrypt;
pub mod encrypt;
pub mod tableau;

pub const BEAUFORT_ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BEAUFORT_VERSION: &str = "1";

pub fn ssize(s: &str) -> usize {
    s.len()
}

pub fn usage() {
    eprintln!("usage: beaufort [-hV] [-e|-d] [-k key] [-a alpha]");
}

pub fn help() {
    usage();
    eprintln!("\noptions:\n  -h  show help\n  -V  show version\n  -e  encrypt\n  -d  decrypt\n  -k  key\n  -a  alphabet");
}

pub fn read_stdin() -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).unwrap_or(0);
    buf
}

pub fn main() {
    // CLI entry point - not used in library tests
}
