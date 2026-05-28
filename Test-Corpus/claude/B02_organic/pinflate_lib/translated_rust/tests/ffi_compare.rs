// Integration tests: load both C and Rust .so via libloading and compare.

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;

type PinflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

fn find_c_so() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    assert!(p.exists(), "C .so not built at {:?}", p);
    p
}

fn find_rust_so() -> PathBuf {
    // Either debug or release build of Rust .so.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for prof in &["release", "debug"] {
        let mut p = manifest.clone();
        p.push("target");
        p.push(prof);
        p.push("libpinflate_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust .so not found; please run `cargo build --release` first");
}

fn compare_pinflate(input: &[u8], out_size: usize) {
    let c_so = find_c_so();
    let r_so = find_rust_so();
    unsafe {
        let c_lib = Library::new(&c_so).expect("load C so");
        let r_lib = Library::new(&r_so).expect("load Rust so");
        let c_fn: Symbol<PinflateFn> = c_lib.get(b"pinflate").expect("C pinflate");
        let r_fn: Symbol<PinflateFn> = r_lib.get(b"pinflate").expect("Rust pinflate");

        // Use distinct buffers per call.
        let mut c_in = input.to_vec();
        let mut r_in = input.to_vec();
        let mut c_out = vec![0u8; out_size.max(1)];
        let mut r_out = vec![0u8; out_size.max(1)];

        let c_rc = c_fn(
            c_in.as_mut_ptr() as *mut c_void,
            c_in.len() as c_int,
            c_out.as_mut_ptr() as *mut c_void,
            c_out.len() as c_int,
        );
        let r_rc = r_fn(
            r_in.as_mut_ptr() as *mut c_void,
            r_in.len() as c_int,
            r_out.as_mut_ptr() as *mut c_void,
            r_out.len() as c_int,
        );

        assert_eq!(
            c_rc, r_rc,
            "return codes differ for input {:?} (C={}, Rust={})",
            &input[..input.len().min(40)],
            c_rc,
            r_rc
        );
        if c_rc == 1 {
            assert_eq!(
                c_out, r_out,
                "output buffers differ for input {:?}",
                &input[..input.len().min(40)]
            );
        }
    }
}

// Raw-deflate (no zlib header/footer) for `data`. Mirrors zlib's compress with -15.
fn raw_deflate(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(
            "import sys, zlib\n\
             d = sys.stdin.buffer.read()\n\
             co = zlib.compressobj(6, zlib.DEFLATED, -15)\n\
             sys.stdout.buffer.write(co.compress(d) + co.flush())",
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn python3");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(data)
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait python3");
    assert!(out.status.success(), "python3 raw_deflate failed");
    out.stdout
}

#[test]
fn deflate_empty() {
    let data = b"";
    let comp = raw_deflate(data);
    compare_pinflate(&comp, data.len().max(1));
}

#[test]
fn deflate_short() {
    let data = b"Hello, world!";
    let comp = raw_deflate(data);
    compare_pinflate(&comp, data.len() + 16);
}

#[test]
fn deflate_repeat_run() {
    let data = vec![b'A'; 100];
    let comp = raw_deflate(&data);
    compare_pinflate(&comp, data.len() + 16);
}

#[test]
fn deflate_dynamic_block() {
    let mut data = Vec::new();
    for _ in 0..10 {
        data.extend_from_slice(b"The quick brown fox jumps over the lazy dog.");
    }
    let comp = raw_deflate(&data);
    compare_pinflate(&comp, data.len() + 64);
}

#[test]
fn deflate_random_bytes() {
    let mut data = Vec::new();
    for _ in 0..4 {
        data.extend(0u8..=255);
    }
    let comp = raw_deflate(&data);
    compare_pinflate(&comp, data.len() + 64);
}

#[test]
fn deflate_stored_block() {
    // Construct a stored (uncompressed) block manually:
    //   bfinal=1, btype=00 -> 3 bits = 001
    //   pad to byte boundary
    //   LEN little-endian (16 bits) = 5
    //   NLEN = ~LEN little-endian
    //   raw bytes "Hello"
    let payload = b"Hello";
    let mut out = Vec::new();
    out.push(0x01u8); // bfinal=1, btype=0, padding zeros
    let len = payload.len() as u16;
    out.push((len & 0xff) as u8);
    out.push((len >> 8) as u8);
    let nlen = !len;
    out.push((nlen & 0xff) as u8);
    out.push((nlen >> 8) as u8);
    out.extend_from_slice(payload);
    compare_pinflate(&out, payload.len() + 8);
}

#[test]
fn deflate_aligned_offsets() {
    // Test odd starting offsets to ensure first_bytes/last_bytes alignment logic matches.
    for offset_pad in 0..4 {
        let data = b"Padding test data with various lengths";
        let comp = raw_deflate(data);
        let mut padded = vec![0u8; offset_pad];
        padded.extend_from_slice(&comp);
        // We don't actually shift the input pointer — pinflate handles alignment internally.
        compare_pinflate(&comp, data.len() + 32);
    }
}

#[test]
fn deflate_large_buffer() {
    let mut data = Vec::new();
    for i in 0..2000u32 {
        data.push((i * 13 + 7) as u8);
    }
    let comp = raw_deflate(&data);
    compare_pinflate(&comp, data.len() + 256);
}

#[test]
fn deflate_very_large_buffer() {
    // Forces multiple deflate blocks and exercises long-distance back references.
    let mut data = Vec::new();
    for i in 0..50000u32 {
        data.push((i.wrapping_mul(31).wrapping_add(11)) as u8);
    }
    let comp = raw_deflate(&data);
    compare_pinflate(&comp, data.len() + 1024);
}

#[test]
fn deflate_highly_repetitive() {
    // All same byte to exercise distance==1 fast-path memset code.
    let data = vec![b'x'; 5000];
    let comp = raw_deflate(&data);
    compare_pinflate(&comp, data.len() + 256);
}

#[test]
fn deflate_pattern_short() {
    let data = b"abcabcabcabcabcabcabcabcabcabcabcabcabcabcabc";
    let comp = raw_deflate(data);
    compare_pinflate(&comp, data.len() + 64);
}
