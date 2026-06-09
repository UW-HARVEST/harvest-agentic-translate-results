// Translation of c_src/src/write.c

use std::fs::File;
use std::io::Write;

pub fn write_to_file(filename: &str, content: &str) -> i32 {
    let mut file = match File::create(filename) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(
                std::io::stderr(),
                "Error opening file '{}': {}",
                filename,
                e
            );
            return e.raw_os_error().unwrap_or(1);
        }
    };

    if let Err(e) = file.write_all(content.as_bytes()) {
        let _ = writeln!(
            std::io::stderr(),
            "Error writing to file '{}': {}",
            filename,
            e
        );
        return e.raw_os_error().unwrap_or(1);
    }

    if let Err(e) = file.sync_all() {
        let _ = writeln!(
            std::io::stderr(),
            "Error closing file '{}': {}",
            filename,
            e
        );
        return e.raw_os_error().unwrap_or(1);
    }

    drop(file);
    0
}
