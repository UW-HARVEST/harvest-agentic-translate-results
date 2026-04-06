use libloading::{Library, Symbol};
use std::ffi::c_char;

extern "C" { static stdout: *mut libc::FILE; }

/// Capture C stdout by redirecting fd 1 to a tmpfile.
fn capture_c_stdout(f: impl FnOnce()) -> Vec<u8> {
    unsafe {
        libc::fflush(stdout);

        let saved = libc::dup(1);
        assert!(saved >= 0);

        let tmp = libc::tmpfile();
        let tmp_fd = libc::fileno(tmp);
        libc::dup2(tmp_fd, 1);

        f();

        libc::fflush(stdout);

        libc::fseek(tmp, 0, libc::SEEK_SET);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = libc::fread(chunk.as_mut_ptr() as *mut _, 1, chunk.len(), tmp);
            if n == 0 { break; }
            buf.extend_from_slice(&chunk[..n]);
        }
        libc::fclose(tmp);

        libc::dup2(saved, 1);
        libc::close(saved);
        buf
    }
}

#[test]
fn test_driver_matches() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_lib_path = manifest.join("c_src/build/libdriver.so");

    // Find the Rust .so
    let target_dir = manifest.join("target");
    let rust_lib_path = find_rust_so(&target_dir);

    let c_lib = unsafe { Library::new(&c_lib_path).expect("load C lib") };
    let rust_lib = unsafe { Library::new(&rust_lib_path).expect("load Rust lib") };

    let c_driver: Symbol<unsafe extern "C" fn(c_char)> =
        unsafe { c_lib.get(b"driver").unwrap() };
    let rust_driver: Symbol<unsafe extern "C" fn(c_char)> =
        unsafe { rust_lib.get(b"driver").unwrap() };

    let test_chars: Vec<c_char> = vec![
        b'A' as c_char, b'z' as c_char, b'0' as c_char, b' ' as c_char,
        b'!' as c_char, b'\t' as c_char, b'\n' as c_char, 0 as c_char,
        b'F' as c_char, b'f' as c_char, 127 as c_char, 1 as c_char,
    ];

    for &ch in &test_chars {
        let c_out = capture_c_stdout(|| unsafe { c_driver(ch) });
        let r_out = capture_c_stdout(|| unsafe { rust_driver(ch) });

        assert_eq!(
            c_out, r_out,
            "Mismatch for char {} (0x{:02x})\nC:\n{}\nRust:\n{}",
            ch, ch as u8,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

fn find_rust_so(target_dir: &std::path::Path) -> std::path::PathBuf {
    // Look in debug first, then release
    for profile in &["debug", "release"] {
        let p = target_dir.join(profile).join("libdriver.so");
        if p.exists() { return p; }
    }
    panic!("Could not find Rust libdriver.so in {:?}", target_dir);
}
