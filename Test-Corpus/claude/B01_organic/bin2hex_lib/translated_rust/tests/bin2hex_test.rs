use libloading::{Library, Symbol};
use std::os::raw::c_char;

type Bin2HexFn = unsafe extern "C" fn(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char;

fn c_lib_path() -> String {
    // The C build artifact lives at translated_rust/c_src/build/libtranslated_rust.so
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libtranslated_rust.so", manifest)
}

fn rust_lib_path() -> String {
    // The Rust .so lives at translated_rust/target/<profile>/libbin2hex_lib.so
    // Determine which profile by checking which file actually exists.
    let manifest = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        format!("{}/target/debug/libbin2hex_lib.so", manifest),
        format!("{}/target/release/libbin2hex_lib.so", manifest),
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

fn run_pair(bin: &[u8], hex_maxlen: usize) -> (Vec<u8>, Vec<u8>) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_bin2hex: Symbol<Bin2HexFn> = c_lib.get(b"bin2hex").expect("C bin2hex");
        let r_bin2hex: Symbol<Bin2HexFn> = rust_lib.get(b"bin2hex").expect("Rust bin2hex");

        let mut c_buf = vec![0u8; hex_maxlen];
        let mut r_buf = vec![0u8; hex_maxlen];

        let c_ret = c_bin2hex(
            c_buf.as_mut_ptr() as *mut c_char,
            hex_maxlen,
            bin.as_ptr(),
            bin.len(),
        );
        assert_eq!(c_ret, c_buf.as_mut_ptr() as *mut c_char);

        let r_ret = r_bin2hex(
            r_buf.as_mut_ptr() as *mut c_char,
            hex_maxlen,
            bin.as_ptr(),
            bin.len(),
        );
        assert_eq!(r_ret, r_buf.as_mut_ptr() as *mut c_char);

        (c_buf, r_buf)
    }
}

#[test]
fn empty_input() {
    // bin_len = 0, hex_maxlen must be > 0
    let (c, r) = run_pair(&[], 1);
    assert_eq!(c, r);
    assert_eq!(c[0], 0); // null terminator
}

#[test]
fn single_byte_all_values() {
    for v in 0u16..=255 {
        let bin = [v as u8];
        let (c, r) = run_pair(&bin, 4);
        assert_eq!(c, r, "mismatch for byte {:#x}", v);
        // Hex string should be 2 chars + null + (1 trailing zero)
        assert_eq!(c[2], 0);
    }
}

#[test]
fn two_bytes_all_pairs() {
    // Sample to stay reasonable: every byte 0-255 paired with itself,
    // plus a few cross pairs.
    for v in 0u16..=255 {
        let bin = [v as u8, v as u8];
        let (c, r) = run_pair(&bin, 8);
        assert_eq!(c, r, "mismatch for [{:#x}, {:#x}]", v, v);
        assert_eq!(c[4], 0);
    }
    let pairs: [(u8, u8); 6] = [
        (0x00, 0xff),
        (0xff, 0x00),
        (0x12, 0x34),
        (0xab, 0xcd),
        (0x9a, 0xa9),
        (0x0f, 0xf0),
    ];
    for (a, b) in pairs {
        let bin = [a, b];
        let (c, r) = run_pair(&bin, 8);
        assert_eq!(c, r, "mismatch for [{:#x}, {:#x}]", a, b);
    }
}

#[test]
fn long_buffer_all_bytes() {
    let bin: Vec<u8> = (0u16..=255).map(|x| x as u8).collect();
    let hex_maxlen = bin.len() * 2 + 1;
    let (c, r) = run_pair(&bin, hex_maxlen);
    assert_eq!(c, r);
    assert_eq!(c[bin.len() * 2], 0);
}

#[test]
fn ascii_pattern() {
    let bin: Vec<u8> = b"The quick brown fox jumps over the lazy dog".to_vec();
    let hex_maxlen = bin.len() * 2 + 16;
    let (c, r) = run_pair(&bin, hex_maxlen);
    assert_eq!(c, r);
}

#[test]
fn extra_buffer_room_unchanged() {
    // Make sure any tail bytes beyond the produced string still match.
    let bin: Vec<u8> = (0..32u8).collect();
    let hex_maxlen = 256;
    let (c, r) = run_pair(&bin, hex_maxlen);
    // Only the first 2*32 + 1 bytes are guaranteed defined.
    let used = bin.len() * 2 + 1;
    assert_eq!(&c[..used], &r[..used]);
}

#[test]
fn random_pattern() {
    // Deterministic pseudo-random byte sequence.
    let mut bin = Vec::with_capacity(1024);
    let mut state: u32 = 0x1234_5678;
    for _ in 0..1024 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        bin.push((state >> 24) as u8);
    }
    let hex_maxlen = bin.len() * 2 + 1;
    let (c, r) = run_pair(&bin, hex_maxlen);
    assert_eq!(c, r);
}
