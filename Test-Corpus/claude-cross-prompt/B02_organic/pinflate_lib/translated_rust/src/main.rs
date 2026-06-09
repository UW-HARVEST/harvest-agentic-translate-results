use std::io::{Read, Write};

use translated_rust::pinflate;

fn main() {
    // Read all of stdin into a Vec<u8>
    let mut input = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut input) {
        eprintln!("Error reading stdin: {}", e);
        std::process::exit(1);
    }

    // Allocate a large output buffer
    let mut output = vec![0u8; 64 * 1024 * 1024]; // 64MB

    match pinflate(&input, &mut output) {
        Ok(n) => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            if let Err(e) = handle.write_all(&output[..n]) {
                eprintln!("Error writing stdout: {}", e);
                std::process::exit(1);
            }
        }
        Err(reason) => {
            eprintln!("pinflate error: {}", reason);
            std::process::exit(1);
        }
    }
}
