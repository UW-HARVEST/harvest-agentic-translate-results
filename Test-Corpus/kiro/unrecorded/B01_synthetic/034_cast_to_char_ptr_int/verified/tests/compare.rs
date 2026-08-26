use libloading::{Library, Symbol};
use std::os::unix::io::FromRawFd;
use std::io::Read;

/// Capture stdout produced by calling `f`, return as bytes.
fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush all C streams

        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);

        let saved_stdout = libc::dup(1);
        assert!(saved_stdout >= 0);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        libc::fflush(std::ptr::null_mut()); // flush all C streams again
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);

        // read captured output
        libc::fcntl(pipe_fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        let mut buf = Vec::new();
        let mut read_end = std::fs::File::from_raw_fd(pipe_fds[0]);
        let _ = read_end.read_to_end(&mut buf);
        buf
    }
}

fn c_lib_path() -> String {
    format!("{}/c_src/libdriver_c.so", env!("CARGO_MANIFEST_DIR"))
}

fn rust_lib_path() -> String {
    // The cdylib is built alongside tests in target/debug/
    let manifest = env!("CARGO_MANIFEST_DIR");
    // Find it relative to the test binary
    let debug_dir = format!("{}/target/debug/libdriver.so", manifest);
    debug_dir
}

#[test]
fn test_driver_outputs_match() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust .so");

        let c_driver: Symbol<unsafe extern "C" fn(i32)> =
            c_lib.get(b"driver").expect("C driver symbol");
        let r_driver: Symbol<unsafe extern "C" fn(i32)> =
            rust_lib.get(b"driver").expect("Rust driver symbol");

        let test_values: &[i32] = &[0, 1, -1, 42, 255, 256, i32::MAX, i32::MIN, 0x12345678];

        for &val in test_values {
            let c_out = capture_stdout(|| c_driver(val));
            let r_out = capture_stdout(|| r_driver(val));
            assert_eq!(
                c_out, r_out,
                "Mismatch for driver({}): C={:?} Rust={:?}",
                val,
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
        }
    }
}

#[test]
fn test_nm_exports_match() {
    let c_out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", &c_lib_path()])
        .output()
        .expect("nm on C .so");
    let r_out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", &rust_lib_path()])
        .output()
        .expect("nm on Rust .so");

    let parse_symbols = |output: &[u8]| -> std::collections::HashSet<String> {
        String::from_utf8_lossy(output)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[1] == "T" {
                    let sym = parts[2];
                    // skip linker-generated symbols
                    if !sym.starts_with('_') {
                        return Some(sym.to_string());
                    }
                }
                None
            })
            .collect()
    };

    let c_syms = parse_symbols(&c_out.stdout);
    let r_syms = parse_symbols(&r_out.stdout);

    for sym in &c_syms {
        // skip 'main' — not needed in Rust .so
        if sym == "main" {
            continue;
        }
        assert!(
            r_syms.contains(sym),
            "C .so exports '{}' but Rust .so does not. C exports: {:?}, Rust exports: {:?}",
            sym, c_syms, r_syms
        );
    }
}
